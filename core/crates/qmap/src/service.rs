//! Il servizio QMAP: tiene la configurazione (in `qmap.json`), raccoglie il periodo di I/Q dal TCI allineato all'orologio (o legge un
//! file), lancia i passaggi della ricerca e conserva l'elenco del periodo con lo spettro per l'interfaccia.
//!
//! Nel minuto di QMAP (60 s) la ricerca parte a 19,5 e 30 s (solo con i Q65-30 accesi), a 49,5 e a 58,5 s, come `bCallDecoder` di
//! `mainwindow.cpp`; i messaggi si sommano nell'elenco del minuto, che riparte dal primo passaggio. Le meta' del minuto in cui la
//! stazione ha trasmesso per piu' di 5 s non si cercano (`m_nTx30a`, `m_nTx30b`). Nominativo e locatore arrivano dalle impostazioni
//! della stazione (`set_station`) e servono per le ipotesi AP e per il Doppler dell'eco: non si salvano in `qmap.json`.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, PoisonError};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use ft8::message::CallHashes;
use rustfft::num_complex::Complex;
use serde::{Deserialize, Serialize};

use crate::astro::{MoonInfo, moon};
use crate::iq::read_wav;
use crate::search::{MAX_SPAN_HZ, Pass, QmapDecode, SearchOptions, SearchState, search_pass};
use crate::sync::{NHSYM_FULL, QMAP_STEP_S};
use crate::tci_iq::{IQ_RATES, TciIqStream};

/// Nominativo e locatore della stazione, letti dalle impostazioni a ogni uso (mai salvati qui).
pub type StationProvider = Arc<dyn Fn() -> (String, String) + Send + Sync>;
/// La stazione sta trasmettendo?
pub type TxProbe = Arc<dyn Fn() -> bool + Send + Sync>;

/// Da dove arriva l'I/Q.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum SourceKind {
    #[default]
    Off,
    /// Il flusso I/Q di un SDR via TCI.
    Tci,
    /// Un file WAV I/Q (registrazione o prova).
    File,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct QmapConfig {
    pub source: SourceKind,
    /// Indirizzo del server TCI (vuoto = 127.0.0.1:40001).
    pub tci_address: String,
    /// Frequenza di campionamento I/Q chiesta al server (48, 96, 192 o 384 kHz).
    pub sample_rate: u32,
    pub file_path: String,
    /// Centro della fetta quando la sorgente non lo dice (file senza frequenza nel nome), in Hz; 0 = sconosciuto.
    pub center_hz: u64,
    pub period_s: u32,
    pub submodes: Vec<u8>,
    pub span_hz: f64,
    pub fading: bool,
    /// Con il periodo di 60 s cerca anche i Q65-30 nelle due meta' del minuto («Also Q65-30x» di QMAP, acceso di fabbrica come la').
    pub also_30: bool,
    /// Media dei periodi per ogni segnale.
    pub average: bool,
}

impl Default for QmapConfig {
    fn default() -> Self {
        QmapConfig {
            source: SourceKind::Off,
            tci_address: String::new(),
            sample_rate: 96_000,
            file_path: String::new(),
            center_hz: 0,
            period_s: 60,
            submodes: vec![0],
            span_hz: MAX_SPAN_HZ,
            fading: false,
            also_30: true,
            average: true,
        }
    }
}

impl QmapConfig {
    /// Valori fuori misura riportati a quelli validi.
    pub fn sanitized(mut self) -> QmapConfig {
        if !IQ_RATES.contains(&self.sample_rate) {
            self.sample_rate = 96_000;
        }
        if !matches!(self.period_s, 15 | 30 | 60 | 120 | 300) {
            self.period_s = 60;
        }
        self.submodes.retain(|&s| s <= 4);
        self.submodes.sort_unstable();
        self.submodes.dedup();
        if self.submodes.is_empty() {
            self.submodes.push(0);
        }
        self.span_hz = if self.span_hz.is_finite() { self.span_hz.clamp(1_000.0, MAX_SPAN_HZ) } else { MAX_SPAN_HZ };
        self
    }

    /// Il minuto di QMAP (60 s con i passaggi a meta' minuto).
    fn minute(&self) -> bool {
        self.period_s == 60
    }

    /// I passaggi del periodo (`nhsym`): nel minuto 130 e 200 solo con i 30 s, poi 330 e 390 (`bCallDecoder` di QMAP); negli altri
    /// periodi uno solo, a periodo finito.
    fn schedule(&self) -> Vec<usize> {
        match (self.minute(), self.also_30) {
            (true, true) => vec![130, 200, 330, NHSYM_FULL],
            (true, false) => vec![330, NHSYM_FULL],
            (false, _) => vec![NHSYM_FULL],
        }
    }

    /// Campioni che servono per un passaggio: la riga `nhsym` finisce a `(nhsym+1)*0,15` s.
    fn samples_for(&self, nhsym: usize, rate: u32) -> usize {
        let secs = if self.minute() { (nhsym as f64 + 1.0) * QMAP_STEP_S } else { f64::from(self.period_s) };
        (secs * f64::from(rate)).round() as usize
    }
}

/// Tempo concesso a un passaggio in tempo reale (`tsec` di `qmapa`): 6 s a 19,5 e 49,5 s, 10 s a 30 s, 16 s a fine minuto.
fn budget_for(minute: bool, nhsym: usize) -> Option<Duration> {
    if !minute {
        return None;
    }
    Some(Duration::from_secs(match nhsym {
        200 => 10,
        n if n >= NHSYM_FULL => 16,
        _ => 6,
    }))
}

/// Quello che vede l'interfaccia.
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct QmapSnapshot {
    pub config: QmapConfig,
    /// «off», «connecting», «receiving», «decoding», «error».
    pub state: String,
    pub message: String,
    pub sample_rate: u32,
    pub center_hz: Option<u64>,
    pub decodes: Vec<QmapDecode>,
    pub candidates: usize,
    pub spectrum_db: Vec<f32>,
    pub spectrum_low_hz: f64,
    pub spectrum_high_hz: f64,
    /// Inizio (ms Unix) del periodo dell'ultimo elenco.
    pub period_start_ms: u64,
    pub updated_ms: u64,
    /// Secondi di dati dell'ultimo passaggio (QMAP: «0,15 * nhsym»).
    pub pass_s: f64,
    /// Segnali con una media in corso.
    pub averaging: usize,
    /// La Luna e il Doppler della propria eco al centro della fetta (serve il locatore nelle impostazioni).
    pub moon: Option<MoonInfo>,
}

#[derive(Default)]
struct Collector {
    period_index: u64,
    rate: u32,
    center: Option<u64>,
    samples: Vec<Complex<f32>>,
    /// Prossimo passaggio da fare nel periodo.
    next: usize,
    /// Secondi in trasmissione nella prima e nella seconda meta' del minuto (`m_nTx30a`, `m_nTx30b`).
    tx_a: u32,
    tx_b: u32,
    last_tx_sec: u64,
}

/// Un lavoro della ricerca: uno o piu' passaggi sullo stesso periodo.
struct Job {
    samples: Vec<Complex<f32>>,
    rate: u32,
    center: Option<u64>,
    period_start_ms: u64,
    /// Passaggi, ognuno con «primo del periodo» (azzera l'elenco) e il tempo concesso.
    passes: Vec<(Pass, bool, Option<Duration>)>,
}

struct Inner {
    config: Mutex<QmapConfig>,
    snapshot: Mutex<QmapSnapshot>,
    collector: Mutex<Collector>,
    /// Candidati del minuto e medie; il lucchetto mette in fila i lavori.
    search: Mutex<SearchState>,
    running: AtomicUsize,
    hashes: Mutex<CallHashes>,
    station: Mutex<Option<StationProvider>>,
    tx_probe: Mutex<Option<TxProbe>>,
}

pub struct QmapService {
    path: Option<PathBuf>,
    inner: Arc<Inner>,
    stream: Mutex<Option<TciIqStream>>,
}

fn now_ms() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |d| d.as_millis() as u64)
}

impl Inner {
    fn edit(&self, f: impl FnOnce(&mut QmapSnapshot)) {
        if let Ok(mut s) = self.snapshot.lock() {
            f(&mut s);
        }
    }

    fn station(&self) -> (String, String) {
        let p = self.station.lock().ok().and_then(|s| s.clone());
        p.map(|f| f()).unwrap_or_default()
    }

    fn transmitting(&self) -> bool {
        let p = self.tx_probe.lock().ok().and_then(|s| s.clone());
        p.is_some_and(|f| f())
    }

    /// Lancia un lavoro in un thread; i lavori si mettono in fila sul lucchetto dello stato della ricerca.
    fn run(self: &Arc<Self>, job: Job) {
        let me = Arc::clone(self);
        let spawned = std::thread::Builder::new().name("qmap".into()).spawn(move || {
            me.running.fetch_add(1, Ordering::SeqCst);
            me.edit(|s| s.state = "decoding".into());
            let mut state = me.search.lock().unwrap_or_else(PoisonError::into_inner);
            let cfg = me.config.lock().map(|c| c.clone()).unwrap_or_default();
            let (my_call, grid) = me.station();
            let options = SearchOptions {
                period_s: cfg.period_s,
                submodes: cfg.submodes.clone(),
                span_hz: cfg.span_hz,
                fading: cfg.fading,
                my_call,
                also_30: cfg.also_30,
                average: cfg.average,
                ..SearchOptions::default()
            };
            // Doppler della propria eco all'inizio del periodo (`m_dop00` letto al secondo 0)
            let self_dop = job.center.and_then(|c| moon(job.period_start_ms, &grid, c)).map(|m| m.self_doppler_hz);
            for (pass, first, budget) in job.passes {
                if first {
                    state.new_period();
                }
                let pass = Pass { deadline: budget.map(|b| Instant::now() + b), ..pass };
                let result = {
                    let mut hashes = me.hashes.lock().unwrap_or_else(PoisonError::into_inner);
                    search_pass(&job.samples, job.rate, job.center, &options, &pass, &mut state, &mut hashes)
                };
                let mut decodes = result.decodes;
                for d in &mut decodes {
                    // `fsked = frx - 0.001*ndop00/2.0`
                    d.sked_hz = d.rf_hz.zip(self_dop).and_then(|(rf, dop)| u64::try_from(rf as i64 - i64::from(dop) / 2).ok());
                }
                let averaging = state.averaging();
                me.edit(|s| {
                    if first || s.period_start_ms != job.period_start_ms {
                        s.decodes = decodes;
                    } else {
                        for d in decodes {
                            if !s.decodes.iter().any(|e| e.text == d.text && e.period_s == d.period_s && e.iseq == d.iseq) {
                                s.decodes.push(d);
                            }
                        }
                        s.decodes.sort_by(|a, b| a.offset_hz.total_cmp(&b.offset_hz));
                    }
                    s.sample_rate = job.rate;
                    s.center_hz = job.center;
                    s.candidates = result.candidates;
                    s.spectrum_db = result.spectrum_db;
                    s.spectrum_low_hz = result.spectrum_low_hz;
                    s.spectrum_high_hz = result.spectrum_high_hz;
                    s.period_start_ms = job.period_start_ms;
                    s.pass_s = if cfg.minute() { QMAP_STEP_S * pass.nhsym as f64 } else { f64::from(cfg.period_s) };
                    s.averaging = averaging;
                    s.updated_ms = now_ms();
                });
            }
            drop(state);
            if me.running.fetch_sub(1, Ordering::SeqCst) == 1 {
                me.edit(|s| s.state = if cfg.source == SourceKind::Tci { "receiving".into() } else { "off".into() });
            }
        });
        if spawned.is_err() {
            tracing::warn!("QMAP: thread della ricerca non avviato");
        }
    }

    /// Un blocco dal TCI: si accumula nel periodo dell'orologio; quando i dati arrivano ai tempi dei passaggi si cerca.
    fn push(self: &Arc<Self>, iq: &[Complex<f32>], rate: u32, center: Option<u64>) {
        let cfg = self.config.lock().map(|c| c.clone()).unwrap_or_default();
        let period_ms = u64::from(cfg.period_s) * 1000;
        let now = now_ms();
        let index = now / period_ms;
        let center = center.or(Some(cfg.center_hz).filter(|&c| c > 0));
        let schedule = cfg.schedule();
        let minute = cfg.minute();
        let sec = now / 1000;
        let transmitting = self.transmitting();
        let mut jobs: Vec<Job> = Vec::new();
        {
            let Ok(mut c) = self.collector.lock() else { return };
            if c.period_index != index || c.rate != rate {
                // il periodo appena finito non ha fatto l'ultimo passaggio (blocchi in ritardo): si fa adesso
                let enough = c.samples.len() as u64 >= u64::from(c.rate) * u64::from(cfg.period_s) * 8 / 10;
                if enough && c.rate == rate && c.period_index + 1 == index && c.next < schedule.len() {
                    let last = schedule[schedule.len() - 1];
                    if let Some(pass) = pass_for(&c, last, minute) {
                        let first = c.next == 0;
                        jobs.push(Job { samples: std::mem::take(&mut c.samples), rate, center: c.center, period_start_ms: c.period_index * period_ms, passes: vec![(pass, first, budget_for(minute, last))] });
                    }
                }
                // il primo periodo dopo l'avvio comincia a meta': zeri per il tempo gia' passato, passaggi gia' scaduti saltati
                let room = rate as usize * cfg.period_s as usize;
                let elapsed = (((now % period_ms) * u64::from(rate) / 1000) as usize).saturating_sub(iq.len()).min(room);
                let mut samples = Vec::with_capacity(room);
                samples.resize(elapsed, Complex::new(0.0, 0.0));
                let next = schedule.iter().take_while(|&&h| cfg.samples_for(h, rate) <= elapsed).count();
                *c = Collector { period_index: index, rate, center, samples, next, tx_a: 0, tx_b: 0, last_tx_sec: 0 };
            }
            if transmitting && c.last_tx_sec != sec {
                c.last_tx_sec = sec;
                // nel minuto la meta' in cui si trasmette; negli altri periodi conta tutto il periodo
                if !minute || sec % 60 < 30 {
                    c.tx_a += 1;
                }
                if !minute || sec % 60 >= 30 {
                    c.tx_b += 1;
                }
            }
            let room = (rate as usize * cfg.period_s as usize).saturating_sub(c.samples.len());
            c.samples.extend_from_slice(&iq[..iq.len().min(room)]);
            c.center = center.or(c.center);
            let mut passes = Vec::new();
            while c.next < schedule.len() && c.samples.len() >= cfg.samples_for(schedule[c.next], rate) {
                let nhsym = schedule[c.next];
                if let Some(pass) = pass_for(&c, nhsym, minute) {
                    passes.push((pass, c.next == 0, budget_for(minute, nhsym)));
                }
                c.next += 1;
            }
            if !passes.is_empty() {
                jobs.push(Job { samples: c.samples.clone(), rate, center: c.center, period_start_ms: index * period_ms, passes });
            }
        }
        self.edit(|s| {
            if s.state != "decoding" {
                s.state = "receiving".into();
            }
            s.message.clear();
            s.sample_rate = rate;
            s.center_hz = center;
        });
        for job in jobs {
            self.run(job);
        }
    }
}

/// Il passaggio `nhsym` del periodo raccolto, o `None` se va saltato perche' si e' trasmesso (`bSkipDecode` di QMAP).
fn pass_for(c: &Collector, nhsym: usize, minute: bool) -> Option<Pass> {
    let (skip_a, skip_b) = (c.tx_a > 5, c.tx_b > 5);
    let skip = (skip_a && skip_b) || (minute && ((nhsym <= 200 && skip_a) || (nhsym == 330 && skip_b)));
    (!skip).then_some(Pass { nhsym, period_index: c.period_index, skip_30a: skip_a, skip_30b: skip_b, deadline: None })
}

impl QmapService {
    /// Apre il servizio con la configurazione salvata in `path` (se c'e') e avvia la sorgente.
    pub fn open(path: Option<&Path>) -> Arc<QmapService> {
        let config = path.and_then(|p| std::fs::read_to_string(p).ok()).and_then(|t| serde_json::from_str::<QmapConfig>(&t).ok()).unwrap_or_default().sanitized();
        let snapshot = QmapSnapshot { config: config.clone(), state: "off".into(), ..QmapSnapshot::default() };
        let service = Arc::new(QmapService {
            path: path.map(Path::to_path_buf),
            inner: Arc::new(Inner {
                config: Mutex::new(config),
                snapshot: Mutex::new(snapshot),
                collector: Mutex::default(),
                search: Mutex::default(),
                running: AtomicUsize::new(0),
                hashes: Mutex::default(),
                station: Mutex::new(None),
                tx_probe: Mutex::new(None),
            }),
            stream: Mutex::new(None),
        });
        service.apply();
        service
    }

    /// Nominativo (per le ipotesi AP) e locatore (per il Doppler) della stazione, letti a ogni uso.
    pub fn set_station(&self, provider: StationProvider) {
        if let Ok(mut s) = self.inner.station.lock() {
            *s = Some(provider);
        }
    }

    /// Chi dice se la stazione sta trasmettendo: le meta' del minuto in trasmissione non si decodificano.
    pub fn set_tx_probe(&self, probe: TxProbe) {
        if let Ok(mut s) = self.inner.tx_probe.lock() {
            *s = Some(probe);
        }
    }

    /// «Cancella media».
    pub fn clear_average(&self) {
        let mut state = self.inner.search.lock().unwrap_or_else(PoisonError::into_inner);
        state.clear_average();
        let n = state.averaging();
        drop(state);
        self.inner.edit(|s| s.averaging = n);
    }

    pub fn config(&self) -> QmapConfig {
        self.inner.config.lock().map(|c| c.clone()).unwrap_or_default()
    }

    pub fn snapshot(&self) -> QmapSnapshot {
        let mut s = self.inner.snapshot.lock().map(|s| s.clone()).unwrap_or_default();
        s.config = self.config();
        if let Ok(g) = self.stream.lock()
            && let Some(stream) = g.as_ref()
        {
            let st = stream.status();
            if !st.connected {
                s.state = if st.message.is_empty() { "connecting".into() } else { "error".into() };
                s.message = st.message;
            } else if !st.streaming && s.state != "decoding" {
                s.state = "connecting".into();
            }
            s.center_hz = s.center_hz.or(stream.center_hz());
        }
        let center = s.center_hz.or(Some(s.config.center_hz).filter(|&c| c > 0));
        s.moon = center.and_then(|c| moon(now_ms(), &self.inner.station().1, c));
        s
    }

    /// Nuova configurazione: si salva e si riavvia la sorgente.
    pub fn set_config(&self, config: QmapConfig) -> Result<(), String> {
        let config = config.sanitized();
        if let Some(path) = &self.path {
            let text = serde_json::to_string_pretty(&config).map_err(|e| e.to_string())?;
            std::fs::write(path, text).map_err(|e| format!("{}: {e}", path.display()))?;
        }
        if let Ok(mut c) = self.inner.config.lock() {
            *c = config;
        }
        self.apply();
        Ok(())
    }

    /// Ferma la sorgente (alla chiusura del programma).
    pub fn shutdown(&self) {
        if let Some(stream) = self.stream.lock().ok().and_then(|mut s| s.take()) {
            stream.shutdown();
        }
    }

    /// Rilegge il file scelto e lo decodifica (pulsante «Decodifica» con la sorgente file): tutti i passaggi completi del periodo.
    pub fn decode_file(&self) -> Result<(), String> {
        let cfg = self.config();
        if cfg.file_path.trim().is_empty() {
            return Err("nessun file".into());
        }
        let block = read_wav(Path::new(cfg.file_path.trim()))?;
        let center = block.center_hz.or(Some(cfg.center_hz).filter(|&c| c > 0));
        self.inner.edit(|s| s.message.clear());
        let start = now_ms();
        let full = Pass::full(start / (u64::from(cfg.period_s) * 1000));
        let passes = if cfg.minute() && cfg.also_30 { vec![(Pass { nhsym: 200, ..full }, true, None), (full, false, None)] } else { vec![(full, true, None)] };
        self.inner.run(Job { samples: block.samples, rate: block.sample_rate, center, period_start_ms: start, passes });
        Ok(())
    }

    fn apply(&self) {
        self.shutdown();
        let cfg = self.config();
        if let Ok(mut c) = self.inner.collector.lock() {
            *c = Collector::default();
        }
        // periodo, sottomodi o sorgente nuovi: le medie non valgono piu' (se una ricerca e' in corso restano fino a «Cancella media»)
        if let Ok(mut st) = self.inner.search.try_lock() {
            st.clear_average();
            st.new_period();
        }
        self.inner.edit(|s| {
            s.state = "off".into();
            s.message.clear();
        });
        match cfg.source {
            SourceKind::Off => {}
            SourceKind::Tci => {
                let inner = Arc::clone(&self.inner);
                let stream = TciIqStream::start(&cfg.tci_address, 0, cfg.sample_rate, Arc::new(move |iq, rate, center| inner.push(iq, rate, center)));
                if let Ok(mut s) = self.stream.lock() {
                    *s = Some(stream);
                }
            }
            SourceKind::File => {
                if let Err(e) = self.decode_file() {
                    self.inner.edit(|s| {
                        s.state = "error".into();
                        s.message = e;
                    });
                }
            }
        }
    }
}

impl Drop for QmapService {
    fn drop(&mut self) {
        self.shutdown();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::iq::{IqBlock, write_wav};
    use crate::search::tests::{add_q65, noise};
    use jt::q65::Setup;

    #[test]
    fn config_is_sanitized() {
        let c = QmapConfig { sample_rate: 12_345, period_s: 7, submodes: vec![9, 2, 2], span_hz: 1e9, ..QmapConfig::default() }.sanitized();
        assert_eq!((c.sample_rate, c.period_s, c.submodes.clone(), c.span_hz), (96_000, 60, vec![2], MAX_SPAN_HZ));
    }

    #[test]
    fn a_file_source_decodes_and_saves_the_config() {
        let dir = std::env::temp_dir().join(format!("aurora-qmap-svc-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("cartella");
        let wav = dir.join("eme_144100kHz.wav");
        let fs = 48_000;
        let setup = Setup::new(15, 0).expect("15A");
        let mut iq = noise(15 * fs as usize, 1.0, 11);
        add_q65(&mut iq, fs, "CQ IU3ABC IO91", 7_000.0, setup, (0.1f32 * 2.0 * 2500.0 / fs as f32).sqrt());
        write_wav(&wav, &IqBlock { sample_rate: fs, center_hz: None, samples: iq }).expect("wav");
        let json = dir.join("qmap.json");
        let svc = QmapService::open(Some(&json));
        svc.set_config(QmapConfig { source: SourceKind::File, file_path: wav.display().to_string(), period_s: 15, ..QmapConfig::default() }).expect("config");
        let until = std::time::Instant::now() + std::time::Duration::from_secs(120);
        while svc.snapshot().updated_ms == 0 && std::time::Instant::now() < until {
            std::thread::sleep(std::time::Duration::from_millis(50));
        }
        let snap = svc.snapshot();
        assert_eq!(snap.decodes.len(), 1, "{snap:?}");
        assert_eq!(snap.decodes[0].rf_hz, Some((144_100_000.0 + snap.decodes[0].offset_hz).round() as u64));
        assert!(std::fs::read_to_string(&json).expect("salvata").contains("\"file\""));
        drop(svc);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
