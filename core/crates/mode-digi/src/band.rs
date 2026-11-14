//! Il thread di «tutta la banda» (CW; PSK31 e PSK63, il «browser»): riceve una copia dell'audio a 12 kHz dal ricevitore principale (se il thread resta indietro perde
//! blocchi suoi, mai quelli del ricevitore principale), lo passa al decodificatore di tutta la banda del modo ([`crate::skim`]) e manda
//! l'elenco dei segnali all'interfaccia come evento `band`, al massimo due volte al secondo e solo se e' cambiato.
//!
//! Limite di CPU: il thread misura quanto tempo gli costa ogni secondo d'audio. Sopra `BUDGET` (un quarto di un core) chiude i canali più
//! deboli fino a rientrare; sotto la metà ne permette di nuovo di più, fino a `MAX_CHANNELS`.
//!
//! Gli spot locali (nominativi verificati) vanno a chi li vuole (`set_spot_sink`: il pannello del DX Cluster di Aurora), mai in rete.

use std::{
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
        mpsc::{Receiver, RecvTimeoutError, SyncSender, TrySendError, sync_channel},
    },
    time::{Duration, Instant},
};

use aurora_core::{DigiEvent, DigiSignal, DigiSignalCall};

use crate::service::{DigiMode, DigiSettings, Emitter};
use crate::skim::calls::{CallCheck, Role, Validation};
use crate::skim::forms::FormTable;
use crate::psk::PskMode;
use crate::skim::psk::PskBandDecoder;
use crate::skim::{BandDecoder, ChannelInfo, CwBandDecoder};

/// Quanta CPU puo' usare il thread (frazione di un core).
const BUDGET: f64 = 0.25;
/// Canali al massimo e al minimo (il limite di CPU si muove fra i due).
const MAX_CHANNELS: usize = 40;
const MIN_CHANNELS: usize = 4;
/// Ogni quanto si guarda la CPU spesa.
const LOAD_EVERY: Duration = Duration::from_secs(2);
/// Al massimo due elenchi al secondo.
const EMIT_EVERY: Duration = Duration::from_millis(500);
/// Nell'evento si mandano gli ultimi caratteri di ogni segnale (lo storico intero si chiede alla rotta `GET /api/v1/digi/band`).
const TAIL_CHARS: usize = 64;
/// Sotto questa probabilità (0-255) un carattere e' incerto e si mostra in grigio.
const UNSURE_BELOW: u8 = 150;
/// Un nominativo non sentito da tanto (s) e' «vecchio»: resta nell'elenco finche' c'e' il canale, ma l'etichetta sulla cascata si toglie.
/// Lo storico di un canale e' di 60 s: poco oltre, del nominativo non resta piu' niente da leggere.
const CALL_STALE_S: f64 = 75.0;
/// La banda audio in cui si cercano i segnali (Hz).
const LO_HZ: f32 = 150.0;
const HI_HZ: f32 = 4_000.0;

/// Uno spot locale verso il pannello del DX Cluster: nominativo, tono audio (Hz, si somma alla frequenza della radio), modo, misure.
#[derive(Clone, Debug, PartialEq)]
pub struct BandSpot {
    pub call: String,
    pub audio_hz: f32,
    pub mode: &'static str,
    pub wpm: Option<f32>,
    /// S/N riferito a 500 Hz (CW: mediana degli ultimi 15 s a tasto abbassato).
    pub snr_db: f32,
    pub cq: bool,
    /// Lo si sente dopo «DE».
    pub de: bool,
}

impl BandSpot {
    /// Il commento dello spot: modo, velocita', S/N in 500 Hz, CQ o DE (`CW 22 WPM 14 dB CQ`).
    pub fn comment(&self) -> String {
        let speed = self.wpm.map(|w| format!(" {w:.0} WPM")).unwrap_or_default();
        let role = if self.cq { " CQ" } else if self.de { " DE" } else { "" };
        format!("{}{speed} {:.0} dB{role}", self.mode, self.snr_db)
    }
}

/// Le regole degli spot locali, applicate da chi conosce la banda: al piu' uno spot per nominativo e banda ogni 10 minuti, di nuovo se la
/// frequenza cambia di oltre 1 kHz o se la stazione passa a chiamare CQ.
#[derive(Default)]
pub struct SpotGate {
    sent: std::collections::HashMap<(String, String), (f64, f64, bool)>,
}

/// Uno spot per nominativo e banda ogni tanto (s).
pub const SPOT_EVERY_S: f64 = 600.0;

impl SpotGate {
    pub fn new() -> Self {
        Self::default()
    }

    /// Lo spot `s` sulla banda `band` a `khz`, al momento `now_s` (secondi): passa?
    pub fn allow(&mut self, s: &BandSpot, band: &str, khz: f64, now_s: f64) -> bool {
        let key = (s.call.clone(), band.to_string());
        let again = match self.sent.get(&key) {
            None => true,
            Some(&(at, f, was_cq)) => now_s - at >= SPOT_EVERY_S || (khz - f).abs() > 1.0 || (s.cq && !was_cq),
        };
        if again {
            self.sent.insert(key, (now_s, khz, s.cq));
            self.sent.retain(|_, (at, _, _)| now_s - *at < 2.0 * SPOT_EVERY_S);
        }
        again
    }
}

/// Le opzioni di tutta la banda che non cambiano il decodificatore.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Options {
    spots: bool,
    cq_only: bool,
    validation: Validation,
}

/// Chi riceve gli spot locali.
pub type SpotSink = Arc<dyn Fn(BandSpot) + Send + Sync>;
/// Dato un nominativo, il paese DXCC (da `cty.dat`); `None` se il prefisso non porta a nessun paese.
pub type CallLookup = Arc<dyn Fn(&str) -> Option<String> + Send + Sync>;

/// Che decodificatore serve.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Kind {
    Cw,
    Psk(PskMode),
    /// RTTY con spostamento e velocita' (Hz, baud) del ricevitore principale.
    Rtty(u32, u32),
}

impl Kind {
    fn of(s: &DigiSettings) -> Option<Kind> {
        match s.mode {
            DigiMode::Cw if s.band_cw => Some(Kind::Cw),
            DigiMode::Psk31 if s.band_psk => Some(Kind::Psk(PskMode::Psk31)),
            DigiMode::Psk63 if s.band_psk => Some(Kind::Psk(PskMode::Psk63)),
            // spostamento in Hz e velocita' in centesimi di baud: interi, cosi' un cambio si riconosce con `==`
            DigiMode::Rtty if s.band_rtty => Some(Kind::Rtty(s.rtty_shift_hz.round() as u32, (s.rtty_baud * 100.0).round() as u32)),
            _ => None,
        }
    }

    fn mode_name(self) -> &'static str {
        match self {
            Kind::Cw => "CW",
            Kind::Psk(m) => m.name(),
            Kind::Rtty(..) => "RTTY",
        }
    }

    fn decoder(self) -> Box<dyn BandDecoder> {
        match self {
            Kind::Cw => Box::new(CwBandDecoder::new(LO_HZ, HI_HZ, MAX_CHANNELS)),
            Kind::Psk(m) => Box::new(PskBandDecoder::new(m, LO_HZ, HI_HZ, MAX_CHANNELS)),
            Kind::Rtty(shift, baud) => Box::new(crate::skim::rtty::RttyBandDecoder::new(shift as f32, baud as f32 / 100.0, MAX_CHANNELS)),
        }
    }
}

enum Msg {
    Audio(Vec<f32>),
    Config(Option<Kind>, Options),
    Reset,
}

#[derive(Default)]
struct Hooks {
    emit: Mutex<Option<Emitter>>,
    spots: Mutex<Option<SpotSink>>,
    lookup: Mutex<Option<CallLookup>>,
    forms: Mutex<Arc<FormTable>>,
    signals: Mutex<Vec<DigiSignal>>,
}

fn lock<T>(m: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|p| p.into_inner())
}

/// Il collegamento fra il servizio e il thread di tutta la banda.
pub struct BandLink {
    queue: SyncSender<Msg>,
    active: AtomicBool,
    hooks: Arc<Hooks>,
}

impl BandLink {
    /// Accende il thread (fermo finche' le impostazioni non chiedono un decodificatore).
    pub fn spawn(settings: &DigiSettings) -> BandLink {
        let (queue, rx) = sync_channel::<Msg>(64);
        let hooks = Arc::new(Hooks::default());
        let worker = Arc::clone(&hooks);
        if let Err(e) = std::thread::Builder::new().name("aurora-digi-banda".into()).spawn(move || band_loop(&worker, rx)) {
            tracing::error!("modi testuali: thread di tutta la banda non avviato: {e}");
        }
        let link = BandLink { queue, active: AtomicBool::new(false), hooks };
        link.configure(settings);
        link
    }

    pub fn set_emitter(&self, emitter: Emitter) {
        *lock(&self.hooks.emit) = Some(emitter);
    }

    pub fn set_spot_sink(&self, sink: SpotSink) {
        *lock(&self.hooks.spots) = Some(sink);
    }

    pub fn set_call_lookup(&self, lookup: CallLookup) {
        *lock(&self.hooks.lookup) = Some(lookup);
    }

    /// Le forme dei nominativi noti per paese (probabilita' a priori per la convalida); si possono rimpiazzare quando si vuole.
    pub fn set_call_forms(&self, forms: FormTable) {
        *lock(&self.hooks.forms) = Arc::new(forms);
    }

    /// Le impostazioni chiedono un decodificatore (CW, o PSK31/PSK63 con «tutta la banda»).
    pub fn is_active(&self) -> bool {
        self.active.load(Ordering::Relaxed)
    }

    /// Le impostazioni nuove: modo, acceso o spento, spot.
    pub fn configure(&self, s: &DigiSettings) {
        let kind = Kind::of(s);
        self.active.store(kind.is_some(), Ordering::Relaxed);
        let validation = Validation::from_id(&s.band_validation).unwrap_or_default();
        let _ = self.queue.send(Msg::Config(kind, Options { spots: s.band_spots, cq_only: s.band_spots_cq_only, validation }));
    }

    /// Altro audio a 12 kHz (se il thread e' indietro il blocco si perde, solo per lui).
    pub fn feed(&self, audio: &[f32]) {
        if !self.active.load(Ordering::Relaxed) || audio.is_empty() {
            return;
        }
        if let Err(TrySendError::Full(_)) = self.queue.try_send(Msg::Audio(audio.to_vec())) {
            tracing::debug!("tutta la banda: blocco audio perso");
        }
    }

    /// Ascolto spento o «Pulisci»: si dimentica tutto (canali, storico, nominativi) e l'interfaccia riceve un elenco vuoto.
    pub fn reset(&self) {
        let _ = self.queue.send(Msg::Reset);
    }

    /// I segnali di adesso con tutto lo storico di 60 s.
    pub fn signals(&self) -> Vec<DigiSignal> {
        lock(&self.hooks.signals).clone()
    }
}

/// Un segnale per l'interfaccia; `tail`: solo gli ultimi caratteri.
fn signal_of(ch: &ChannelInfo, wpm: bool, tail: Option<usize>, lookup: Option<&CallLookup>) -> DigiSignal {
    let chars: Vec<char> = ch.text.chars().collect();
    let from = tail.map_or(0, |n| chars.len().saturating_sub(n));
    let text: String = chars[from..].iter().collect();
    let unsure = ch.conf.iter().skip(from).enumerate().filter(|(_, p)| **p < UNSURE_BELOW).map(|(i, _)| i as u32).collect();
    DigiSignal {
        id: ch.id,
        freq_hz: (ch.freq_hz * 10.0).round() / 10.0,
        wpm: wpm.then_some(ch.wpm.round()),
        snr_db: ch.snr_db.round(),
        text,
        unsure,
        calls: ch
            .calls
            .iter()
            .map(|c| DigiSignalCall {
                call: c.call.clone(),
                role: match c.role {
                    Role::Cq => "cq",
                    Role::De => "de",
                    Role::Other => "other",
                }
                .into(),
                verified: c.verified(),
                country: lookup.and_then(|f| f(&c.call)),
                stale: ch.now_s - c.last_s > CALL_STALE_S,
            })
            .collect(),
        cq: ch.cq,
        report: ch.last_report.clone(),
    }
}

/// Cosa si sa di un nominativo: il paese da `cty.dat` (se c'e') e la probabilita' a priori della sua forma in quel paese.
fn check_call(hooks: &Hooks, call: &str) -> CallCheck {
    let Some(lookup) = lock(&hooks.lookup).clone() else { return CallCheck::default() };
    let country = lookup(call);
    let forms = Arc::clone(&lock(&hooks.forms));
    CallCheck { dxcc: Some(country.is_some()), prob: country.as_deref().and_then(|c| forms.prob(call, c)) }
}

fn band_loop(hooks: &Arc<Hooks>, queue: Receiver<Msg>) {
    let mut kind: Option<Kind> = None;
    let mut opts = Options { spots: true, cq_only: false, validation: Validation::Normal };
    let mut decoder: Option<Box<dyn BandDecoder>> = None;
    let mut max_channels = MAX_CHANNELS;
    let (mut spent, mut heard) = (Duration::ZERO, 0.0_f64);
    let mut last_load = Instant::now();
    let mut last_emit = Instant::now();
    let mut last_sent: Option<Vec<DigiSignal>> = None;
    let emit = |ev: DigiEvent| {
        let e = lock(&hooks.emit).clone();
        if let Some(e) = e {
            e(ev);
        }
    };
    let empty = |kind: Option<Kind>| DigiEvent { kind: "band".into(), mode: kind.map(|k| k.mode_name().to_ascii_lowercase()), signals: Some(Vec::new()), ..DigiEvent::default() };
    loop {
        let msg = match queue.recv_timeout(EMIT_EVERY) {
            Ok(m) => Some(m),
            Err(RecvTimeoutError::Timeout) => None,
            Err(RecvTimeoutError::Disconnected) => return,
        };
        match msg {
            Some(Msg::Audio(audio)) => {
                let Some(k) = kind else { continue };
                let d = decoder.get_or_insert_with(|| {
                    let mut d = k.decoder();
                    d.set_max_channels(max_channels);
                    d.set_validation(opts.validation);
                    let slot = Arc::clone(hooks);
                    d.set_call_check(Box::new(move |c: &str| check_call(&slot, c)));
                    d
                });
                let started = Instant::now();
                d.process(&audio);
                spent += started.elapsed();
                heard += audio.len() as f64 / f64::from(crate::RATE);
                let spots = d.take_spots();
                if opts.spots {
                    let sink = lock(&hooks.spots).clone();
                    // con «solo CQ» passano solo le stazioni che chiamano
                    for s in spots.into_iter().filter(|s| s.cq || !opts.cq_only) {
                        if let Some(sink) = &sink {
                            sink(BandSpot { call: s.call, audio_hz: s.freq_hz, mode: k.mode_name(), wpm: (k == Kind::Cw).then_some(s.wpm.round()), snr_db: s.snr_db.round(), cq: s.cq, de: s.de });
                        }
                    }
                }
            }
            Some(Msg::Config(k, o)) => {
                if o.validation != opts.validation
                    && let Some(d) = decoder.as_mut()
                {
                    d.set_validation(o.validation);
                }
                opts = o;
                if k != kind {
                    kind = k;
                    decoder = None;
                    max_channels = MAX_CHANNELS;
                    lock(&hooks.signals).clear();
                    last_sent = None;
                    emit(empty(kind));
                }
            }
            Some(Msg::Reset) => {
                if let Some(d) = decoder.as_mut() {
                    d.clear();
                }
                lock(&hooks.signals).clear();
                last_sent = None;
                emit(empty(kind));
            }
            None => {}
        }
        // il limite di CPU: canali in meno se si spende troppo, di nuovo in piu' quando avanza
        if last_load.elapsed() >= LOAD_EVERY {
            last_load = Instant::now();
            if let Some(d) = decoder.as_mut()
                && heard > 0.5
            {
                let load = spent.as_secs_f64() / heard;
                let open = d.channel_count();
                if load > BUDGET && open > MIN_CHANNELS {
                    max_channels = ((open as f64 * BUDGET / load).floor() as usize).max(MIN_CHANNELS);
                    d.set_max_channels(max_channels);
                    tracing::info!("tutta la banda: CPU al {:.0}% di un core, canali al massimo {max_channels}", load * 100.0);
                } else if load < 0.5 * BUDGET && max_channels < MAX_CHANNELS && open + 2 >= max_channels {
                    max_channels = (max_channels + 4).min(MAX_CHANNELS);
                    d.set_max_channels(max_channels);
                }
            }
            (spent, heard) = (Duration::ZERO, 0.0);
        }
        if last_emit.elapsed() >= EMIT_EVERY {
            last_emit = Instant::now();
            let (Some(k), Some(d)) = (kind, decoder.as_ref()) else { continue };
            let lookup = lock(&hooks.lookup).clone();
            let chans = d.channels();
            let wpm = k == Kind::Cw;
            *lock(&hooks.signals) = chans.iter().map(|c| signal_of(c, wpm, None, lookup.as_ref())).collect();
            let list: Vec<DigiSignal> = chans.iter().map(|c| signal_of(c, wpm, Some(TAIL_CHARS), lookup.as_ref())).collect();
            if last_sent.as_ref() != Some(&list) {
                emit(DigiEvent { kind: "band".into(), mode: Some(k.mode_name().to_ascii_lowercase()), signals: Some(list.clone()), ..DigiEvent::default() });
                last_sent = Some(list);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::skim::calls::HeardCall;

    #[test]
    fn the_rules_of_the_local_spots() {
        let spot = |cq: bool, de: bool| BandSpot { call: "IU3ABC".into(), audio_hz: 700.0, mode: "CW", wpm: Some(22.0), snr_db: 14.0, cq, de };
        assert_eq!(spot(true, false).comment(), "CW 22 WPM 14 dB CQ");
        assert_eq!(spot(false, true).comment(), "CW 22 WPM 14 dB DE");
        let mut g = SpotGate::new();
        assert!(g.allow(&spot(false, true), "20m", 14_025.0, 0.0));
        // ancora lui, stessa banda: niente per 10 minuti
        assert!(!g.allow(&spot(false, true), "20m", 14_025.3, 300.0));
        // passa a chiamare CQ: si ripete subito
        assert!(g.allow(&spot(true, false), "20m", 14_025.3, 310.0));
        assert!(!g.allow(&spot(true, false), "20m", 14_025.3, 320.0));
        // oltre 1 kHz piu' in la', o un'altra banda: subito
        assert!(g.allow(&spot(true, false), "20m", 14_027.0, 330.0));
        assert!(g.allow(&spot(true, false), "40m", 7_025.0, 340.0));
        // dopo 10 minuti di nuovo
        assert!(g.allow(&spot(true, false), "20m", 14_027.0, 330.0 + SPOT_EVERY_S));
    }

    #[test]
    fn the_tail_keeps_the_unsure_positions_in_step() {
        let ch = ChannelInfo {
            id: 3,
            freq_hz: 712.34,
            wpm: 21.6,
            snr_db: 9.4,
            text: "CQ DE IU3ABC K".into(),
            conf: vec![255, 255, 255, 255, 255, 255, 100, 255, 255, 255, 255, 255, 255, 40],
            calls: vec![HeardCall { call: "IU3ABC".into(), role: Role::Cq, count: 2, score: 1.9, last_s: 3.0, shape: crate::skim::forms::Shape::Typical }],
            cq: true,
            last_report: None,
            now_s: 10.0,
        };
        let lookup: CallLookup = Arc::new(|c: &str| c.starts_with("IU").then(|| "Italy".to_string()));
        let s = signal_of(&ch, true, Some(8), Some(&lookup));
        assert_eq!(s.text, "IU3ABC K");
        assert_eq!(s.unsure, vec![0, 7]);
        assert_eq!(s.wpm, Some(22.0));
        assert_eq!(s.freq_hz, 712.3);
        assert_eq!(s.calls[0].role, "cq");
        assert!(s.calls[0].verified);
        assert_eq!(s.calls[0].country.as_deref(), Some("Italy"));
        // sentito 7 s fa: l'etichetta sulla cascata resta; dopo 75 s senza sentirlo si toglie (il nominativo resta nell'elenco)
        assert!(!s.calls[0].stale);
        let later = ChannelInfo { now_s: 3.0 + CALL_STALE_S + 1.0, ..ch.clone() };
        let old = signal_of(&later, true, Some(8), Some(&lookup));
        assert!(old.calls[0].stale);
        assert_eq!(old.calls[0].call, "IU3ABC");
        let full = signal_of(&ch, false, None, None);
        assert_eq!(full.text.len(), 14);
        assert_eq!(full.wpm, None);
    }

    /// Dal servizio: l'audio arriva anche al thread di tutta la banda, che manda l'elenco dei segnali e lo spot locale. Restituisce il
    /// primo segnale che chiama CQ con il nominativo verificato, lo storico completo e gli spot.
    fn through_the_service(s: DigiSettings, wave: Vec<f32>, tag: &str) -> (Option<DigiSignal>, Vec<DigiSignal>, Vec<BandSpot>) {
        use crate::service::DigiService;
        let dir = std::env::temp_dir().join(format!("aurora-digi-banda-{tag}-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("cartella di prova");
        std::fs::write(dir.join("digi.json"), serde_json::to_vec(&s).expect("json")).expect("digi.json");
        let svc = DigiService::open(&dir);
        let (tx, rx) = std::sync::mpsc::channel::<DigiEvent>();
        let tx = Mutex::new(tx);
        svc.set_emitter(Arc::new(move |ev: DigiEvent| {
            if ev.kind == "band" {
                let _ = lock(&tx).send(ev);
            }
        }));
        let spots = Arc::new(Mutex::new(Vec::<BandSpot>::new()));
        let got = Arc::clone(&spots);
        svc.set_spot_sink(Arc::new(move |s| lock(&got).push(s)));
        svc.set_call_lookup(Arc::new(|c: &str| c.starts_with("IU").then(|| "Italy".to_string())));
        svc.start_rx();
        for chunk in wave.chunks(1200) {
            svc.feed(crate::RATE as u32, chunk);
            // il thread di tutta la banda non deve restare indietro (senza ottimizzazioni e' lento)
            std::thread::sleep(Duration::from_millis(25));
        }
        let deadline = Instant::now() + Duration::from_secs(20);
        let mut seen = None;
        while seen.is_none() && Instant::now() < deadline {
            if let Ok(ev) = rx.recv_timeout(Duration::from_millis(200)) {
                // il primo «CQ» si perde mentre il canale si apre: il ruolo CQ arriva con la seconda chiamata
                seen = ev.signals.unwrap_or_default().into_iter().find(|s| s.cq && s.calls.iter().any(|c| c.call == "IU3ABC" && c.verified));
            }
        }
        let full = svc.band_signals();
        svc.stop_rx();
        let _ = std::fs::remove_dir_all(&dir);
        let spots = lock(&spots).clone();
        (seen, full, spots)
    }

    #[test]
    fn the_service_sends_the_cw_band_list_and_a_local_spot() {
        let s = DigiSettings { mode: DigiMode::Cw, ..DigiSettings::default() };
        let mut wave = vec![0.0; 6000];
        wave.extend(crate::cw::modulate(b"CQ CQ DE IU3ABC IU3ABC K   CQ CQ DE IU3ABC IU3ABC K", 800.0, 22.0, 0.3));
        wave.extend(vec![0.0; 12000]);
        let (seen, full, spots) = through_the_service(s, wave, "cw");
        let sig = seen.expect("nessun segnale con IU3ABC");
        assert!((sig.freq_hz - 800.0).abs() < 5.0, "{sig:?}");
        assert!(sig.cq && sig.wpm.is_some(), "{sig:?}");
        assert_eq!(sig.calls[0].country.as_deref(), Some("Italy"));
        assert!(full.iter().any(|s| s.text.contains("DE IU3ABC")), "{full:?}");
        assert!(spots.iter().any(|s| s.call == "IU3ABC" && s.mode == "CW" && (s.audio_hz - 800.0).abs() < 5.0), "{spots:?}");
    }

    #[test]
    fn the_service_sends_the_psk_band_list_and_a_local_spot() {
        let s = DigiSettings { mode: DigiMode::Psk31, ..DigiSettings::default() };
        let mut wave = vec![0.0; 6000];
        wave.extend(crate::psk::modulate(b"cq cq cq de IU3ABC IU3ABC IU3ABC pse k  cq cq de IU3ABC IU3ABC k", 1530.0, PskMode::Psk31, 0.2));
        wave.extend(vec![0.0; 12000]);
        let (seen, full, spots) = through_the_service(s, wave, "psk");
        let sig = seen.expect("nessun segnale con IU3ABC");
        assert!((sig.freq_hz - 1530.0).abs() < 3.0, "{sig:?}");
        assert!(sig.cq && sig.wpm.is_none(), "{sig:?}");
        assert!(full.iter().any(|s| s.text.contains("de IU3ABC")), "{full:?}");
        assert!(spots.iter().any(|s| s.call == "IU3ABC" && s.mode == "PSK31"), "{spots:?}");
    }
}
