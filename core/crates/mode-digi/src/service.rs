//! Il servizio dei modi testuali: riceve una copia dell'audio d'ingresso (solo con l'ascolto acceso: l'FT8 non se ne accorge), decodifica in
//! un thread suo e manda il testo ricevuto come eventi; trasmette un testo con la catena di trasmissione di Aurora (audio d'uscita e PTT).
//! Impostazioni (modo, frequenze, macro) in `digi.json`.

use std::{
    path::PathBuf,
    sync::{
        Arc, Mutex, MutexGuard,
        atomic::{AtomicBool, Ordering},
        mpsc::{Receiver, RecvTimeoutError, SyncSender, TrySendError, sync_channel},
    },
    time::{Duration, Instant},
};

use aurora_core::DigiEvent;
use mode_sstv::resample::Resampler;
use serde::{Deserialize, Serialize};

use crate::{PskDemodulator, PskMode, RATE, RttyDemodulator, RttyParams, psk, rtty};

/// La catena di trasmissione vista dai modi testuali (la fornisce il programma: `tx-native` con il PTT scelto).
pub trait DigiTxPort: Send + Sync {
    /// Trasmette subito l'onda (12 kHz, mono, picco 1), fuori dai periodi FT8.
    fn start(&self, wave: Vec<f32>) -> Result<(), String>;
    fn halt(&self);
    fn is_busy(&self) -> bool;
}

pub type Emitter = Arc<dyn Fn(DigiEvent) + Send + Sync>;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DigiMode {
    Psk31,
    Psk63,
    Rtty,
}

impl DigiMode {
    pub const ALL: [DigiMode; 3] = [DigiMode::Psk31, DigiMode::Psk63, DigiMode::Rtty];

    pub fn id(self) -> &'static str {
        match self {
            DigiMode::Psk31 => "psk31",
            DigiMode::Psk63 => "psk63",
            DigiMode::Rtty => "rtty",
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            DigiMode::Psk31 => "PSK31",
            DigiMode::Psk63 => "PSK63",
            DigiMode::Rtty => "RTTY",
        }
    }

    pub fn from_id(id: &str) -> Option<DigiMode> {
        DigiMode::ALL.into_iter().find(|m| m.id() == id.to_ascii_lowercase())
    }

    fn psk(self) -> Option<PskMode> {
        match self {
            DigiMode::Psk31 => Some(PskMode::Psk31),
            DigiMode::Psk63 => Some(PskMode::Psk63),
            DigiMode::Rtty => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum DigiError {
    UnknownMode(String),
    BadValue(String),
    Busy,
    NoTransmitter,
    Empty,
    Io(String),
    Tx(String),
}

impl std::fmt::Display for DigiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DigiError::UnknownMode(m) => write!(f, "modo sconosciuto: {m}"),
            DigiError::BadValue(v) => write!(f, "valore non valido: {v}"),
            DigiError::Busy => write!(f, "trasmissione gia' in corso"),
            DigiError::NoTransmitter => write!(f, "nessuna catena di trasmissione"),
            DigiError::Empty => write!(f, "testo vuoto"),
            DigiError::Io(e) | DigiError::Tx(e) => write!(f, "{e}"),
        }
    }
}

impl std::error::Error for DigiError {}

impl DigiError {
    pub fn code(&self) -> &'static str {
        match self {
            DigiError::UnknownMode(_) => "unknown_mode",
            DigiError::BadValue(_) => "bad_value",
            DigiError::Busy => "busy",
            DigiError::NoTransmitter => "no_transmitter",
            DigiError::Empty => "empty_text",
            DigiError::Io(_) => "io",
            DigiError::Tx(_) => "tx_failed",
        }
    }
}

/// Una macro: `name` e' il tasto («F5», «Ctrl-F5», «Shift-F5», «Ctrl-Shift-F5») oppure un evento («OnTX», «OnStart»...),
/// `label` quello che si legge sul pulsante, `text` lo script (parole fra `<` e `>`), `color` lo sfondo del pulsante (RGB, 0xRRGGBB).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct DigiMacro {
    pub name: String,
    pub label: String,
    pub text: String,
    pub color: Option<u32>,
}

/// I nomi validi di una macro: i 48 tasti e le macro evento.
const EVENT_NAMES: [&str; 11] = ["OnStart", "OnEnd", "OnTX", "OnCR", "OnClickAny", "OnAlarm", "OnSP0", "OnSP1", "Gray+", "Hello", "AutoCqText"];

fn valid_macro_name(name: &str) -> bool {
    if EVENT_NAMES.contains(&name) {
        return true;
    }
    let key = name.strip_prefix("Ctrl-Shift-").or_else(|| name.strip_prefix("Ctrl-")).or_else(|| name.strip_prefix("Shift-")).unwrap_or(name);
    key.strip_prefix('F').and_then(|n| n.parse::<u8>().ok()).is_some_and(|n| (1..=12).contains(&n))
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct DigiSettings {
    pub mode: DigiMode,
    /// Frequenza della portante in ricezione (PSK) o del mark (RTTY), in Hz sull'audio.
    pub rx_freq_hz: f32,
    /// In trasmissione si usa la stessa frequenza della ricezione.
    pub tx_follow_rx: bool,
    pub tx_freq_hz: f32,
    pub rtty_shift_hz: f32,
    pub rtty_baud: f32,
    /// Ascolto acceso all'avvio di Aurora.
    pub auto_receive: bool,
    /// L'insieme di macro per tutti i modi.
    pub macros: Vec<DigiMacro>,
    /// Le macro di un modo (chiave: `psk31`, `psk63`, `rtty`): sostituiscono quelle con lo stesso nome dell'insieme di base.
    pub mode_macros: std::collections::BTreeMap<String, Vec<DigiMacro>>,
}

fn mac(name: &str, label: &str, text: &str, color: u32) -> DigiMacro {
    DigiMacro { name: name.into(), label: label.into(), text: text.into(), color: Some(color) }
}

/// Le macro di base, nelle posizioni: prima riga AutoCQ · CQ · Call 3 · Call | Info · Brag · Bye · Clear | TX · RX · « · »;
/// seconda riga (Ctrl) MyCall · MyName · MyQTH | Call · Name · QTH | BM. I testi sono modelli: le parole fra < > le riempie il programma.
fn default_macros() -> Vec<DigiMacro> {
    const GIALLO: u32 = 0xFFFF80;
    const VERDE: u32 = 0x80FF80;
    const GRIGIO: u32 = 0xF0F0F0;
    const ROSSO: u32 = 0xFF8080;
    vec![
        mac("F1", "AutoCQ", "<AUTOCQ>", ROSSO),
        mac("F2", "CQ", "<TX>CQ CQ CQ DE <MYCALL> <MYCALL> <MYCALL> PSE K<RX>", GIALLO),
        mac("F3", "Call 3", "<TX><CALL> <CALL> <CALL> DE <MYCALL> K<RX>", GIALLO),
        mac("F4", "Call", "<TX><FIXSTARTTIME><CALL> DE <MYCALL> K<RX>", GIALLO),
        mac("F5", "Info", "<TX><CALL> DE <MYCALL> TNX FER CALL UR <RSTS> <RSTS> NAME <MYNAME> <MYNAME> QTH <MYQTH> <MYQTH> LOC <MYWWLOC> HW? <CALL> DE <MYCALL> K<RX>", VERDE),
        mac("F6", "Brag", "<TX><CALL> DE <MYCALL> RIG HERE IS ... ANT IS ... PWR ... W <CALL> DE <MYCALL> K<RX>", VERDE),
        mac("F7", "Bye", "<TX><CALL> DE <MYCALL> TNX FER NICE QSO <NAME> BEST 73 <CALL> DE <MYCALL> SK<SAVEQSO><RXANDCLEAR>", VERDE),
        mac("F8", "Clear", "<CLEARRXWINDOW><CLEARTXWINDOW><CLEARQSO>", GRIGIO),
        mac("F9", "TX", "<TX>", ROSSO),
        mac("F10", "RX", "<RX>", VERDE),
        mac("F11", "<<", "<SEEKLEFT>", GRIGIO),
        mac("F12", ">>", "<SEEKRIGHT>", GRIGIO),
        mac("Ctrl-F1", "MyCall", "<TX><MYCALL> <MYCALL> <MYCALL><RX>", GIALLO),
        mac("Ctrl-F2", "MyName", "<TX>MY NAME IS <MYNAME> <MYNAME> <MYNAME><RX>", GIALLO),
        mac("Ctrl-F3", "MyQTH", "<TX>MY QTH IS <MYQTH> <MYQTH> LOC <MYWWLOC><RX>", GIALLO),
        mac("Ctrl-F5", "Call", "<CALL>", GRIGIO),
        mac("Ctrl-F6", "Name", "<NAME>", GRIGIO),
        mac("Ctrl-F7", "QTH", "<QTH>", GRIGIO),
        mac("Ctrl-F12", "BM", "<BOOKMARK>", GRIGIO),
        mac("Gray+", "Save", "<FIXSTARTTIME><SAVEQSO>", GRIGIO),
    ]
}

/// Le macro per modo, come i file di macro: i tasti della velocita' per PSK e per RTTY.
fn default_mode_macros() -> std::collections::BTreeMap<String, Vec<DigiMacro>> {
    const CIANO: u32 = 0xFFFF80;
    let mut m = std::collections::BTreeMap::new();
    m.insert("psk31".to_string(), vec![mac("F10", "PSK31", "<BAUDRATE:31.25>", CIANO), mac("F11", "PSK63", "<BAUDRATE:62.5>", CIANO)]);
    m.insert("psk63".to_string(), vec![mac("F10", "PSK31", "<BAUDRATE:31.25>", CIANO), mac("F11", "PSK63", "<BAUDRATE:62.5>", CIANO)]);
    m.insert(
        "rtty".to_string(),
        vec![mac("F10", "BAUD45", "<BAUDRATE:45.45>", CIANO), mac("F11", "BAUD 50", "<BAUDRATE:50>", CIANO), mac("F12", "BAUD 75", "<BAUDRATE:75>", CIANO)],
    );
    m
}

impl Default for DigiSettings {
    fn default() -> Self {
        DigiSettings {
            mode: DigiMode::Psk31,
            rx_freq_hz: 1000.0,
            tx_follow_rx: true,
            tx_freq_hz: 1000.0,
            rtty_shift_hz: 170.0,
            rtty_baud: 45.45,
            auto_receive: false,
            macros: default_macros(),
            mode_macros: default_mode_macros(),
        }
    }
}

impl DigiSettings {
    fn validate(&self) -> Result<(), DigiError> {
        let in_band = |f: f32| f.is_finite() && (200.0..=3500.0).contains(&f);
        if !in_band(self.rx_freq_hz) || !in_band(self.tx_freq_hz) {
            return Err(DigiError::BadValue("frequenza fuori dai 200-3500 Hz dell'audio".into()));
        }
        if !self.rtty_shift_hz.is_finite() || !(50.0..=1000.0).contains(&self.rtty_shift_hz) {
            return Err(DigiError::BadValue("spostamento RTTY fuori da 50-1000 Hz".into()));
        }
        if !self.rtty_baud.is_finite() || !(30.0..=300.0).contains(&self.rtty_baud) {
            return Err(DigiError::BadValue("velocita' RTTY fuori da 30-300 baud".into()));
        }
        let sets = std::iter::once(&self.macros).chain(self.mode_macros.values());
        for set in sets {
            if set.len() > 100 || set.iter().any(|m| m.label.len() > 60 || m.text.len() > 4000) {
                return Err(DigiError::BadValue("macro troppo lunghe o troppe".into()));
            }
            if let Some(bad) = set.iter().find(|m| !valid_macro_name(&m.name)) {
                return Err(DigiError::BadValue(format!("nome di macro non valido: {}", bad.name)));
            }
        }
        if self.mode_macros.keys().any(|k| DigiMode::from_id(k).is_none()) {
            return Err(DigiError::BadValue("insieme di macro per un modo sconosciuto".into()));
        }
        Ok(())
    }

    fn tx_freq(&self) -> f32 {
        if self.tx_follow_rx { self.rx_freq_hz } else { self.tx_freq_hz }
    }

    fn rtty(&self, mark_hz: f32) -> RttyParams {
        RttyParams { mark_hz, shift_hz: self.rtty_shift_hz, baud: self.rtty_baud }
    }
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct RxStatus {
    pub listening: bool,
    pub level_db: f32,
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct TxStatus {
    /// `idle`, `sending`, `done`, `aborted`, `error`.
    pub state: String,
    pub progress: f32,
    pub elapsed_s: f32,
    pub total_s: f32,
    pub message: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct ModeInfo {
    pub id: &'static str,
    pub name: &'static str,
}

#[derive(Clone, Debug, Serialize)]
pub struct Snapshot {
    pub rx: RxStatus,
    pub tx: TxStatus,
    pub settings: DigiSettings,
    pub can_transmit: bool,
    pub modes: Vec<ModeInfo>,
}

enum Msg {
    Audio(u32, Vec<f32>),
    Config(DigiSettings),
    Reset,
}

struct Inner {
    settings_path: PathBuf,
    settings: Mutex<DigiSettings>,
    listening: AtomicBool,
    tx_active: AtomicBool,
    tx_abort: AtomicBool,
    queue: SyncSender<Msg>,
    rx: Mutex<RxStatus>,
    tx: Mutex<TxStatus>,
    port: Mutex<Option<Arc<dyn DigiTxPort>>>,
    emit: Mutex<Option<Emitter>>,
}

fn lock<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|p| p.into_inner())
}

impl Inner {
    fn emit(&self, ev: DigiEvent) {
        let emitter = lock(&self.emit).clone();
        if let Some(e) = emitter {
            e(ev);
        }
    }

    fn rx_status_event(&self) {
        let st = lock(&self.rx).clone();
        let s = lock(&self.settings).clone();
        self.emit(DigiEvent {
            kind: "rx_status".into(),
            listening: Some(st.listening),
            level_db: Some(st.level_db),
            mode: Some(s.mode.id().into()),
            freq_hz: Some(s.rx_freq_hz),
            ..DigiEvent::default()
        });
    }

    fn tx_event(&self) {
        let st = lock(&self.tx).clone();
        self.emit(DigiEvent { kind: "tx".into(), state: Some(st.state), progress: Some(st.progress), message: st.message, ..DigiEvent::default() });
    }
}

/// Il servizio (uno per programma).
pub struct DigiService {
    inner: Arc<Inner>,
}

impl DigiService {
    /// Legge `digi.json` dalla cartella dei dati e accende il thread di ricezione.
    pub fn open(data_dir: &std::path::Path) -> DigiService {
        let settings_path = data_dir.join("digi.json");
        let mut settings: DigiSettings = std::fs::read(&settings_path).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or_default();
        // le macro della prima versione non avevano il tasto: si riparte dall'insieme di base
        if settings.macros.iter().any(|m| m.name.is_empty()) {
            settings.macros = default_macros();
        }
        let (queue, rx) = sync_channel::<Msg>(128);
        let inner = Arc::new(Inner {
            settings_path,
            listening: AtomicBool::new(settings.auto_receive),
            settings: Mutex::new(settings.clone()),
            tx_active: AtomicBool::new(false),
            tx_abort: AtomicBool::new(false),
            queue,
            rx: Mutex::new(RxStatus { listening: settings.auto_receive, level_db: -90.0 }),
            tx: Mutex::new(TxStatus { state: "idle".into(), ..TxStatus::default() }),
            port: Mutex::new(None),
            emit: Mutex::new(None),
        });
        let worker = Arc::clone(&inner);
        if let Err(e) = std::thread::Builder::new().name("aurora-digi".into()).spawn(move || rx_loop(&worker, rx, settings)) {
            tracing::error!("modi testuali: thread di ricezione non avviato: {e}");
        }
        DigiService { inner }
    }

    pub fn set_emitter(&self, emitter: Emitter) {
        *lock(&self.inner.emit) = Some(emitter);
    }

    pub fn set_tx_port(&self, port: Arc<dyn DigiTxPort>) {
        *lock(&self.inner.port) = Some(port);
    }

    /// Una copia dell'audio d'ingresso (frequenza della scheda). Con l'ascolto spento non costa niente.
    pub fn feed(&self, rate: u32, samples: &[f32]) {
        if !self.inner.listening.load(Ordering::Relaxed) || self.inner.tx_active.load(Ordering::Relaxed) {
            return;
        }
        if let Err(TrySendError::Full(_)) = self.inner.queue.try_send(Msg::Audio(rate, samples.to_vec())) {
            tracing::debug!("modi testuali: blocco audio perso");
        }
    }

    pub fn is_listening(&self) -> bool {
        self.inner.listening.load(Ordering::Relaxed)
    }

    pub fn snapshot(&self) -> Snapshot {
        Snapshot {
            rx: lock(&self.inner.rx).clone(),
            tx: lock(&self.inner.tx).clone(),
            settings: lock(&self.inner.settings).clone(),
            can_transmit: lock(&self.inner.port).is_some(),
            modes: DigiMode::ALL.iter().map(|m| ModeInfo { id: m.id(), name: m.name() }).collect(),
        }
    }

    pub fn settings(&self) -> DigiSettings {
        lock(&self.inner.settings).clone()
    }

    pub fn apply_settings(&self, s: DigiSettings) -> Result<(), DigiError> {
        s.validate()?;
        let json = serde_json::to_vec_pretty(&s).map_err(|e| DigiError::Io(e.to_string()))?;
        std::fs::write(&self.inner.settings_path, json).map_err(|e| DigiError::Io(e.to_string()))?;
        let _ = self.inner.queue.send(Msg::Config(s.clone()));
        *lock(&self.inner.settings) = s;
        self.inner.rx_status_event();
        Ok(())
    }

    pub fn start_rx(&self) {
        self.inner.listening.store(true, Ordering::Relaxed);
        lock(&self.inner.rx).listening = true;
        self.inner.rx_status_event();
    }

    pub fn stop_rx(&self) {
        self.inner.listening.store(false, Ordering::Relaxed);
        lock(&self.inner.rx).listening = false;
        let _ = self.inner.queue.send(Msg::Reset);
        self.inner.rx_status_event();
    }

    /// Il testo da trasmettere diventa audio con il modo e la frequenza scelti.
    pub fn wave_of(text: &str, s: &DigiSettings) -> Vec<f32> {
        let bytes: Vec<u8> = text.chars().map(|c| if (c as u32) < 256 { c as u32 as u8 } else { b'?' }).collect();
        match s.mode.psk() {
            Some(mode) => psk::modulate(&bytes, s.tx_freq(), mode, 0.9),
            None => rtty::modulate(&bytes, s.rtty(s.tx_freq()), 0.9),
        }
    }

    /// Trasmette il testo. Il modo e la frequenza sono quelli delle impostazioni.
    pub fn transmit(&self, text: &str) -> Result<TxStatus, DigiError> {
        if text.trim().is_empty() {
            return Err(DigiError::Empty);
        }
        let port = lock(&self.inner.port).clone().ok_or(DigiError::NoTransmitter)?;
        if self.inner.tx_active.load(Ordering::SeqCst) || port.is_busy() {
            return Err(DigiError::Busy);
        }
        let settings = self.settings();
        let wave = Self::wave_of(text, &settings);
        let total = wave.len() as f32 / RATE;
        self.inner.tx_active.store(true, Ordering::SeqCst);
        self.inner.tx_abort.store(false, Ordering::SeqCst);
        // la propria trasmissione non si decodifica
        let _ = self.inner.queue.send(Msg::Reset);
        if let Err(e) = port.start(wave) {
            self.inner.tx_active.store(false, Ordering::SeqCst);
            return Err(DigiError::Tx(e));
        }
        tracing::info!("{}: trasmissione di {total:.0} s", settings.mode.name());
        *lock(&self.inner.tx) = TxStatus { state: "sending".into(), progress: 0.0, elapsed_s: 0.0, total_s: total, message: None };
        self.inner.tx_event();
        let inner = Arc::clone(&self.inner);
        let spawned = std::thread::Builder::new().name("aurora-digi-tx".into()).spawn(move || tx_monitor(&inner, &port, total));
        if let Err(e) = spawned {
            tracing::error!("modi testuali: controllo della trasmissione non avviato: {e}");
        }
        Ok(lock(&self.inner.tx).clone())
    }

    /// Ferma la trasmissione in corso (audio e PTT).
    pub fn abort_tx(&self) {
        if !self.inner.tx_active.load(Ordering::SeqCst) {
            return;
        }
        self.inner.tx_abort.store(true, Ordering::SeqCst);
        if let Some(port) = lock(&self.inner.port).clone() {
            port.halt();
        }
    }
}

fn tx_monitor(inner: &Arc<Inner>, port: &Arc<dyn DigiTxPort>, total: f32) {
    let started = Instant::now();
    let mut last_emit = Instant::now();
    let state = loop {
        std::thread::sleep(Duration::from_millis(200));
        let elapsed = started.elapsed().as_secs_f32();
        if inner.tx_abort.load(Ordering::SeqCst) {
            break "aborted";
        }
        // la catena puo' impiegare qualche centinaio di ms per partire (PTT): «finito» vale solo dopo il primo secondo
        if elapsed > 1.5 && !port.is_busy() {
            break if elapsed + 2.0 >= total { "done" } else { "error" };
        }
        {
            let mut tx = lock(&inner.tx);
            tx.elapsed_s = elapsed;
            tx.progress = (elapsed / total.max(0.1)).min(1.0);
        }
        if last_emit.elapsed() >= Duration::from_millis(500) {
            last_emit = Instant::now();
            inner.tx_event();
        }
    };
    {
        let mut tx = lock(&inner.tx);
        tx.state = state.into();
        if state == "done" {
            tx.progress = 1.0;
        }
        if state == "error" {
            tx.message = Some("trasmissione interrotta dalla catena di trasmissione (PTT, audio o un'altra trasmissione)".into());
        }
    }
    inner.tx_active.store(false, Ordering::SeqCst);
    inner.tx_abort.store(false, Ordering::SeqCst);
    tracing::info!("modi testuali: trasmissione finita ({state})");
    inner.tx_event();
}

enum Demod {
    Psk(Box<PskDemodulator>),
    Rtty(Box<RttyDemodulator>),
}

impl Demod {
    fn new(s: &DigiSettings) -> Demod {
        match s.mode.psk() {
            Some(mode) => Demod::Psk(Box::new(PskDemodulator::new(mode, s.rx_freq_hz))),
            None => Demod::Rtty(Box::new(RttyDemodulator::new(s.rtty(s.rx_freq_hz)))),
        }
    }

    fn process(&mut self, audio: &[f32]) -> Vec<u8> {
        match self {
            Demod::Psk(d) => d.process(audio),
            Demod::Rtty(d) => d.process(audio),
        }
    }
}

fn level_db(audio: &[f32]) -> f32 {
    if audio.is_empty() {
        return -90.0;
    }
    let rms = (audio.iter().map(|s| s * s).sum::<f32>() / audio.len() as f32).sqrt();
    (20.0 * rms.max(1e-5).log10()).max(-90.0)
}

fn rx_loop(inner: &Arc<Inner>, queue: Receiver<Msg>, settings: DigiSettings) {
    let mut demod = Demod::new(&settings);
    let mut mode = settings.mode;
    let mut resampler: Option<Resampler> = None;
    let mut audio = Vec::new();
    let mut last_status = Instant::now();
    let mut peak_db = -90.0_f32;
    loop {
        let msg = match queue.recv_timeout(Duration::from_millis(500)) {
            Ok(m) => Some(m),
            Err(RecvTimeoutError::Timeout) => None,
            Err(RecvTimeoutError::Disconnected) => return,
        };
        match msg {
            Some(Msg::Audio(rate, samples)) => {
                let r = match &mut resampler {
                    Some(r) if r.in_rate() == rate => r,
                    slot => slot.insert(Resampler::new(rate)),
                };
                audio.clear();
                r.process(&samples, &mut audio);
                peak_db = peak_db.max(level_db(&audio));
                let text = demod.process(&audio);
                if !text.is_empty() {
                    let s: String = text.iter().map(|&b| b as char).collect();
                    inner.emit(DigiEvent { kind: "rx_text".into(), text: Some(s), mode: Some(mode.id().into()), ..DigiEvent::default() });
                }
            }
            Some(Msg::Config(s)) => {
                demod = Demod::new(&s);
                mode = s.mode;
            }
            Some(Msg::Reset) => demod = Demod::new(&lock(&inner.settings).clone()),
            None => {}
        }
        if last_status.elapsed() >= Duration::from_millis(700) {
            last_status = Instant::now();
            lock(&inner.rx).level_db = peak_db;
            peak_db = -90.0;
            if inner.listening.load(Ordering::Relaxed) {
                inner.rx_status_event();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn i_modi_si_riconoscono_dal_nome() {
        assert_eq!(DigiMode::from_id("PSK31"), Some(DigiMode::Psk31));
        assert_eq!(DigiMode::from_id("rtty"), Some(DigiMode::Rtty));
        assert_eq!(DigiMode::from_id("cw"), None);
    }

    #[test]
    fn le_impostazioni_si_controllano() {
        let mut s = DigiSettings::default();
        assert!(s.validate().is_ok());
        s.rx_freq_hz = 50.0;
        assert!(s.validate().is_err());
        s.rx_freq_hz = 1000.0;
        s.rtty_baud = 5.0;
        assert!(s.validate().is_err());
    }

    #[test]
    fn le_macro_di_base_hanno_nomi_validi_e_le_posizioni_di_() {
        let s = DigiSettings::default();
        assert!(s.validate().is_ok());
        let nomi: Vec<&str> = s.macros.iter().map(|m| m.name.as_str()).collect();
        assert_eq!(&nomi[..4], ["F1", "F2", "F3", "F4"]);
        assert_eq!(s.macros[0].label, "AutoCQ");
        assert!(s.mode_macros.contains_key("rtty"));
    }

    #[test]
    fn i_nomi_delle_macro_si_controllano() {
        for ok in ["F1", "F12", "Ctrl-F5", "Shift-F9", "Ctrl-Shift-F12", "OnTX", "Gray+", "AutoCqText"] {
            assert!(valid_macro_name(ok), "{ok}");
        }
        for bad in ["F0", "F13", "Alt-F1", "", "ontx", "Ctrl-"] {
            assert!(!valid_macro_name(bad), "{bad}");
        }
    }

    #[test]
    fn il_testo_diventa_audio_e_si_rilegge() {
        let s = DigiSettings::default();
        let wave = DigiService::wave_of("CQ DE IU3ABC", &s);
        assert!(!wave.is_empty());
        let mut d = Demod::new(&s);
        let mut all = vec![0.0; 4000];
        all.extend(wave);
        all.extend(vec![0.0; 4000]);
        let out = String::from_utf8_lossy(&d.process(&all)).into_owned();
        assert!(out.contains("CQ DE IU3ABC"), "ricevuto: {out:?}");
    }
}
