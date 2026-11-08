//! Il servizio QMAP: tiene la configurazione (in `qmap.json`), raccoglie un periodo di I/Q dal TCI allineato all'orologio (o legge un
//! file), lancia la ricerca in un thread e conserva l'ultimo elenco con lo spettro per l'interfaccia.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use ft8::message::CallHashes;
use rustfft::num_complex::Complex;
use serde::{Deserialize, Serialize};

use crate::iq::read_wav;
use crate::search::{MAX_SPAN_HZ, QmapDecode, SearchOptions, search};
use crate::tci_iq::{IQ_RATES, TciIqStream};

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
}

impl Default for QmapConfig {
    fn default() -> Self {
        QmapConfig { source: SourceKind::Off, tci_address: String::new(), sample_rate: 96_000, file_path: String::new(), center_hz: 0, period_s: 60, submodes: vec![0], span_hz: MAX_SPAN_HZ, fading: false }
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
}

#[derive(Default)]
struct Collector {
    period_index: u64,
    rate: u32,
    center: Option<u64>,
    samples: Vec<Complex<f32>>,
}

struct Inner {
    config: Mutex<QmapConfig>,
    snapshot: Mutex<QmapSnapshot>,
    collector: Mutex<Collector>,
    busy: AtomicBool,
    hashes: Mutex<CallHashes>,
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

    /// Lancia la ricerca in un thread (una per volta: se la precedente non ha finito il periodo si salta).
    fn decode(self: &Arc<Self>, samples: Vec<Complex<f32>>, rate: u32, center: Option<u64>, period_start_ms: u64) {
        if self.busy.swap(true, Ordering::SeqCst) {
            tracing::warn!("QMAP: ricerca precedente ancora in corso, periodo saltato");
            return;
        }
        let me = Arc::clone(self);
        let spawned = std::thread::Builder::new().name("qmap".into()).spawn(move || {
            let cfg = me.config.lock().map(|c| c.clone()).unwrap_or_default();
            me.edit(|s| s.state = "decoding".into());
            let options = SearchOptions { period_s: cfg.period_s, submodes: cfg.submodes.clone(), span_hz: cfg.span_hz, fading: cfg.fading, ..SearchOptions::default() };
            let result = {
                let mut hashes = me.hashes.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
                search(&samples, rate, center, &options, &mut hashes)
            };
            me.edit(|s| {
                s.state = if cfg.source == SourceKind::Tci { "receiving".into() } else { "off".into() };
                s.sample_rate = rate;
                s.center_hz = center;
                s.decodes = result.decodes;
                s.candidates = result.candidates;
                s.spectrum_db = result.spectrum_db;
                s.spectrum_low_hz = result.spectrum_low_hz;
                s.spectrum_high_hz = result.spectrum_high_hz;
                s.period_start_ms = period_start_ms;
                s.updated_ms = now_ms();
            });
            me.busy.store(false, Ordering::SeqCst);
        });
        if spawned.is_err() {
            self.busy.store(false, Ordering::SeqCst);
        }
    }

    /// Un blocco dal TCI: si accumula nel periodo dell'orologio; al cambio di periodo quello finito va alla ricerca.
    fn push(self: &Arc<Self>, iq: &[Complex<f32>], rate: u32, center: Option<u64>) {
        let (period_s, fallback) = self.config.lock().map(|c| (c.period_s, c.center_hz)).unwrap_or((60, 0));
        let period_ms = u64::from(period_s) * 1000;
        let index = now_ms() / period_ms;
        let center = center.or(Some(fallback).filter(|&c| c > 0));
        let finished = {
            let Ok(mut c) = self.collector.lock() else { return };
            let mut done = None;
            if c.period_index != index || c.rate != rate {
                let full = c.samples.len() as u64 >= u64::from(c.rate) * u64::from(period_s) * 8 / 10;
                if full && c.rate == rate && c.period_index + 1 == index {
                    done = Some((std::mem::take(&mut c.samples), c.rate, c.center, c.period_index * period_ms));
                }
                *c = Collector { period_index: index, rate, center, samples: Vec::with_capacity(rate as usize * period_s as usize) };
            }
            let room = (rate as usize * period_s as usize).saturating_sub(c.samples.len());
            c.samples.extend_from_slice(&iq[..iq.len().min(room)]);
            c.center = center.or(c.center);
            done
        };
        self.edit(|s| {
            if s.state != "decoding" {
                s.state = "receiving".into();
            }
            s.message.clear();
            s.sample_rate = rate;
            s.center_hz = center;
        });
        if let Some((samples, rate, center, start)) = finished {
            self.decode(samples, rate, center, start);
        }
    }
}

impl QmapService {
    /// Apre il servizio con la configurazione salvata in `path` (se c'e') e avvia la sorgente.
    pub fn open(path: Option<&Path>) -> Arc<QmapService> {
        let config = path.and_then(|p| std::fs::read_to_string(p).ok()).and_then(|t| serde_json::from_str::<QmapConfig>(&t).ok()).unwrap_or_default().sanitized();
        let snapshot = QmapSnapshot { config: config.clone(), state: "off".into(), ..QmapSnapshot::default() };
        let service = Arc::new(QmapService {
            path: path.map(Path::to_path_buf),
            inner: Arc::new(Inner { config: Mutex::new(config), snapshot: Mutex::new(snapshot), collector: Mutex::default(), busy: AtomicBool::new(false), hashes: Mutex::default() }),
            stream: Mutex::new(None),
        });
        service.apply();
        service
    }

    pub fn config(&self) -> QmapConfig {
        self.inner.config.lock().map(|c| c.clone()).unwrap_or_default()
    }

    pub fn snapshot(&self) -> QmapSnapshot {
        let mut s = self.inner.snapshot.lock().map(|s| s.clone()).unwrap_or_default();
        s.config = self.config();
        let guard = self.stream.lock();
        if let Ok(g) = &guard
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

    /// Rilegge il file scelto e lo decodifica (pulsante «Decodifica» con la sorgente file).
    pub fn decode_file(&self) -> Result<(), String> {
        let cfg = self.config();
        if cfg.file_path.trim().is_empty() {
            return Err("nessun file".into());
        }
        let block = read_wav(Path::new(cfg.file_path.trim()))?;
        let center = block.center_hz.or(Some(cfg.center_hz).filter(|&c| c > 0));
        self.inner.edit(|s| s.message.clear());
        self.inner.decode(block.samples, block.sample_rate, center, now_ms());
        Ok(())
    }

    fn apply(&self) {
        self.shutdown();
        let cfg = self.config();
        if let Ok(mut c) = self.inner.collector.lock() {
            *c = Collector::default();
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
