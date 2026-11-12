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

use crate::{CwDemodulator, PskDemodulator, PskMode, RATE, RttyDemodulator, RttyParams, cw, psk, rtty};

/// La catena di trasmissione vista dai modi testuali (la fornisce il programma: `tx-native` con il PTT scelto).
pub trait DigiTxPort: Send + Sync {
    /// Trasmette subito l'onda (12 kHz, mono, picco 1), fuori dai periodi FT8.
    fn start(&self, wave: Vec<f32>) -> Result<(), String>;
    fn halt(&self);
    fn is_busy(&self) -> bool;
}

pub type Emitter = Arc<dyn Fn(DigiEvent) + Send + Sync>;

/// Il ricevitore RTTY: due filtri sui toni (come fldigi) o discriminatore di frequenza.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RttyReceiver {
    /// Due filtri, e il discriminatore sotto i 40 Hz di spostamento (dove i due filtri si sovrappongono: con 23 Hz e 3 dB di S/N i due filtri
    /// sbagliano il 24% dei caratteri, il discriminatore il 5%; con 50 Hz e oltre vincono i due filtri, `tests/rtty_confronto.rs`, prova `stretti`).
    #[default]
    Auto,
    Filters,
    Discriminator,
}

impl RttyReceiver {
    fn discriminator(self, shift_hz: f32) -> bool {
        match self {
            RttyReceiver::Auto => shift_hz < 40.0,
            RttyReceiver::Filters => false,
            RttyReceiver::Discriminator => true,
        }
    }
}

/// Il modo della radio letto dal CAT («USB», «PKTLSB», «DATA-L»...), se c'e' una radio collegata che lo dice.
pub type RadioModeSource = Arc<dyn Fn() -> Option<String> + Send + Sync>;

/// La radio e' in banda laterale inferiore (LSB, LSB dati): in RTTY mark e space si invertono da soli, come in fldigi
/// (`reverse = Reverse ^ !USB`, `src/rtty/rtty.cxx`). Nomi di Hamlib (`LSB`, `PKTLSB`), di OmniRig e TCI (`DIGL`, `DATA-L`, `LSB-D`).
pub fn is_lower_sideband(mode: &str) -> bool {
    let m = mode.trim().to_ascii_uppercase().replace(['-', '_', ' '], "");
    m.contains("LSB") || matches!(m.as_str(), "DIGL" | "DATAL" | "DIGLSB" | "PKTL")
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DigiMode {
    Psk31,
    Psk63,
    Rtty,
    Cw,
}

impl DigiMode {
    pub const ALL: [DigiMode; 4] = [DigiMode::Psk31, DigiMode::Psk63, DigiMode::Rtty, DigiMode::Cw];

    pub fn id(self) -> &'static str {
        match self {
            DigiMode::Psk31 => "psk31",
            DigiMode::Psk63 => "psk63",
            DigiMode::Rtty => "rtty",
            DigiMode::Cw => "cw",
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            DigiMode::Psk31 => "PSK31",
            DigiMode::Psk63 => "PSK63",
            DigiMode::Rtty => "RTTY",
            DigiMode::Cw => "CW",
        }
    }

    pub fn from_id(id: &str) -> Option<DigiMode> {
        DigiMode::ALL.into_iter().find(|m| m.id() == id.to_ascii_lowercase())
    }

    fn psk(self) -> Option<PskMode> {
        match self {
            DigiMode::Psk31 => Some(PskMode::Psk31),
            DigiMode::Psk63 => Some(PskMode::Psk63),
            DigiMode::Rtty | DigiMode::Cw => None,
        }
    }
}

/// Come si importano le macro: `replace` (default) sostituisce le mie, `add` tiene le mie e aggiunge solo i tasti che mancano, `defaults` rimette quelle di Aurora.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ImportHow {
    #[default]
    Replace,
    Add,
    Defaults,
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
const EVENT_NAMES: [&str; 12] = ["OnStart", "OnEnd", "OnTX", "OnCR", "OnClickAny", "OnAlarm", "OnSP0", "OnSP1", "Gray+", "Hello", "AutoCqText", "OM"];

pub(crate) fn valid_macro_name(name: &str) -> bool {
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
    /// Frequenza della portante in ricezione (PSK, CW) o del centro fra mark e space (RTTY, come in fldigi e come la disegna la cascata),
    /// in Hz sull'audio.
    pub rx_freq_hz: f32,
    /// In trasmissione si usa la stessa frequenza della ricezione.
    pub tx_follow_rx: bool,
    pub tx_freq_hz: f32,
    pub rtty_shift_hz: f32,
    pub rtty_baud: f32,
    /// RTTY: mark e space scambiati a mano (tasto «Rev» di fldigi). Con la radio in LSB si invertono gia' da soli.
    pub rtty_reverse: bool,
    /// RTTY: UOS, dopo uno spazio si torna alle lettere (acceso di base, come `UOSrx` di fldigi).
    pub rtty_uos: bool,
    /// RTTY: soglia dello squelch sul rapporto segnale/rumore (dB; 0 = solo rumore).
    pub rtty_squelch_snr_db: f32,
    /// RTTY: il ricevitore (due filtri, discriminatore o scelta secondo lo spostamento).
    pub rtty_receiver: RttyReceiver,
    /// Velocita' della CW in parole al minuto (trasmissione; in ricezione e' il valore di partenza, poi si adatta al segnale).
    pub cw_wpm: f32,
    /// CW: banda del filtro di ricezione (Hz: 50, 100, 150, 250 o 500; 150 di base come fldigi).
    pub cw_bandwidth_hz: f32,
    /// CW: soglia dello squelch sul rapporto segnale/rumore (dB in 3 kHz, media nel tempo; -30..+20).
    pub cw_squelch_snr_db: f32,
    /// CW: decodifica SOM di fldigi (aiuta col tasto a mano irregolare).
    pub cw_som: bool,
    /// PSK: soglia dello squelch sulla metrica di qualita' della fase (0-100, come fldigi).
    pub psk_squelch: f32,
    /// AFC: la ricezione segue la deriva del segnale (Options › AFC).
    pub afc_on: bool,
    /// Squelch: si stampa solo sopra la soglia del modo (`psk_squelch`, `rtty_squelch_snr_db`, `cw_squelch_snr_db`). Le soglie sono
    /// relative (qualita' della fase o rapporto segnale/rumore), non dipendono dal volume: la vecchia `squelch_db` in dB assoluti delle
    /// impostazioni salvate si ignora e valgono i valori di base.
    pub squelch_on: bool,
    /// Snap: un clic sulla cascata porta sul segnale piu' vicino (lo fa l'interfaccia).
    pub snap_on: bool,
    /// Ascolto acceso all'avvio di Aurora.
    pub auto_receive: bool,
    /// L'insieme di macro per tutti i modi.
    pub macros: Vec<DigiMacro>,
    /// Le macro di un modo (chiave: `psk31`, `psk63`, `rtty`): sostituiscono quelle con lo stesso nome dell'insieme di base.
    pub mode_macros: std::collections::BTreeMap<String, Vec<DigiMacro>>,
    /// Per modo: le macro del modo si uniscono a quelle di base (`true`, opzione «Merge») oppure le sostituiscono del tutto (`false`). Un modo senza voce unisce.
    pub merge_macros: std::collections::BTreeMap<String, bool>,
    /// CW: si decodificano tutti i segnali della banda audio (pannello «Segnali», thread a parte con limite di CPU).
    pub band_cw: bool,
    /// PSK31 e PSK63: si decodificano tutti i segnali della banda audio («browser», pannello «Segnali»).
    pub band_psk: bool,
    /// Tutta la banda: i nominativi verificati vanno come spot locali nel pannello del DX Cluster di Aurora (mai in rete).
    pub band_spots: bool,
    /// Tutta la banda: spot locali solo delle stazioni che chiamano CQ.
    pub band_spots_cq_only: bool,
    /// Tutta la banda: livello di convalida dei nominativi (`minimal`, `normal`, `strict`, `maximum`).
    pub band_validation: String,
}

fn mac(name: &str, label: &str, text: &str, color: u32) -> DigiMacro {
    DigiMacro { name: name.into(), label: label.into(), text: text.into(), color: Some(color) }
}

/// Le macro di base: prima riga AutoCQ · CQ · Call 3 · Call | Info · Brag · Bye · Clear | TX · RX · « · »;
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

/// Le macro per modo: i tasti della velocita' per PSK e per RTTY.
fn default_mode_macros() -> std::collections::BTreeMap<String, Vec<DigiMacro>> {
    const CIANO: u32 = 0xFFFF80;
    let mut m = std::collections::BTreeMap::new();
    m.insert("psk31".to_string(), vec![mac("F10", "PSK31", "<BAUDRATE:31.25>", CIANO), mac("F11", "PSK63", "<BAUDRATE:62.5>", CIANO)]);
    m.insert("psk63".to_string(), vec![mac("F10", "PSK31", "<BAUDRATE:31.25>", CIANO), mac("F11", "PSK63", "<BAUDRATE:62.5>", CIANO)]);
    m.insert("cw".to_string(), vec![mac("F10", "WPM 15", "<WPM:15>", CIANO), mac("F11", "WPM 20", "<WPM:20>", CIANO), mac("F12", "WPM 25", "<WPM:25>", CIANO)]);
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
            rtty_reverse: false,
            rtty_uos: true,
            rtty_squelch_snr_db: 6.0,
            rtty_receiver: RttyReceiver::Auto,
            cw_wpm: 20.0,
            cw_bandwidth_hz: cw::BANDWIDTH_DEFAULT,
            cw_squelch_snr_db: cw::SQUELCH_DEFAULT_DB,
            cw_som: false,
            psk_squelch: psk::SQUELCH_DEFAULT,
            afc_on: true,
            squelch_on: false,
            snap_on: true,
            auto_receive: false,
            macros: default_macros(),
            mode_macros: default_mode_macros(),
            merge_macros: std::collections::BTreeMap::new(),
            band_cw: true,
            band_psk: true,
            band_spots: true,
            band_spots_cq_only: false,
            band_validation: "normal".into(),
        }
    }
}

impl DigiSettings {
    fn validate(&self) -> Result<(), DigiError> {
        let in_band = |f: f32| f.is_finite() && (200.0..=3500.0).contains(&f);
        if !in_band(self.rx_freq_hz) || !in_band(self.tx_freq_hz) {
            return Err(DigiError::BadValue("frequenza fuori dai 200-3500 Hz dell'audio".into()));
        }
        // da 23 Hz (lo spostamento piu' stretto dell'elenco di fldigi) a 1000 Hz
        if !self.rtty_shift_hz.is_finite() || !(20.0..=1000.0).contains(&self.rtty_shift_hz) {
            return Err(DigiError::BadValue("spostamento RTTY fuori da 20-1000 Hz".into()));
        }
        if !self.psk_squelch.is_finite() || !(0.0..=100.0).contains(&self.psk_squelch) {
            return Err(DigiError::BadValue("soglia dello squelch PSK fuori da 0-100".into()));
        }
        if !self.cw_squelch_snr_db.is_finite() || !(-30.0..=20.0).contains(&self.cw_squelch_snr_db) {
            return Err(DigiError::BadValue("soglia S/N dello squelch CW fuori da -30..+20 dB".into()));
        }
        if !cw::BANDWIDTHS.contains(&self.cw_bandwidth_hz) {
            return Err(DigiError::BadValue("banda CW non fra 50, 100, 150, 250 e 500 Hz".into()));
        }
        if !self.rtty_baud.is_finite() || !(30.0..=300.0).contains(&self.rtty_baud) {
            return Err(DigiError::BadValue("velocita' RTTY fuori da 30-300 baud".into()));
        }
        if !self.rtty_squelch_snr_db.is_finite() || !(0.0..=40.0).contains(&self.rtty_squelch_snr_db) {
            return Err(DigiError::BadValue("soglia S/N dello squelch RTTY fuori da 0-40 dB".into()));
        }
        if !self.cw_wpm.is_finite() || !(cw::WPM_MIN..=cw::WPM_MAX).contains(&self.cw_wpm) {
            return Err(DigiError::BadValue("velocita' CW fuori da 5-60 WPM".into()));
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
        if crate::skim::calls::Validation::from_id(&self.band_validation).is_none() {
            return Err(DigiError::BadValue("convalida dei nominativi non fra minimal, normal, strict e maximum".into()));
        }
        if self.mode_macros.keys().any(|k| DigiMode::from_id(k).is_none()) {
            return Err(DigiError::BadValue("insieme di macro per un modo sconosciuto".into()));
        }
        Ok(())
    }

    fn tx_freq(&self) -> f32 {
        if self.tx_follow_rx { self.rx_freq_hz } else { self.tx_freq_hz }
    }

    /// I parametri RTTY sul centro dato; `lsb`: la radio e' in LSB e mark e space si invertono (oltre a «Rev»).
    fn rtty(&self, centre_hz: f32, lsb: bool) -> RttyParams {
        RttyParams { centre_hz, shift_hz: self.rtty_shift_hz, baud: self.rtty_baud, reverse: self.rtty_reverse ^ lsb }
    }
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct RxStatus {
    pub listening: bool,
    pub level_db: f32,
    /// La frequenza su cui si ascolta adesso (con l'AFC puo' essere diversa da quella scelta).
    pub freq_hz: f32,
    /// Lo squelch lascia passare il segnale.
    pub squelch_open: bool,
    /// Rapporto segnale/rumore (dB): RTTY quello dello squelch, PSK quello di fldigi (toni contro rumore), CW in 3 kHz (media nel tempo).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub snr_db: Option<f32>,
    /// PSK: IMD (dB), misurata durante le inversioni continue (preambolo o pausa).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub imd_db: Option<f32>,
    /// PSK: metrica di qualita' della fase (0-100) su cui lavora lo squelch.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub quality: Option<f32>,
    /// CW: la velocita' capita (WPM).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub wpm: Option<f32>,
    /// La radio e' in LSB: in RTTY mark e space sono gia' invertiti da soli.
    pub radio_lsb: bool,
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
    radio_mode: Mutex<Option<RadioModeSource>>,
    /// L'ultima lettura: la radio e' in LSB.
    radio_lsb: AtomicBool,
    /// Il thread di tutta la banda (CW, PSK).
    band: crate::band::BandLink,
}

fn lock<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|p| p.into_inner())
}

impl Inner {
    /// Legge adesso il modo della radio (se c'e' da dove) e ricorda se e' LSB; senza radio resta l'ultima lettura (di base USB).
    fn read_radio_lsb(&self) -> bool {
        let source = lock(&self.radio_mode).clone();
        if let Some(mode) = source.and_then(|f| f()) {
            self.radio_lsb.store(is_lower_sideband(&mode), Ordering::Relaxed);
        }
        self.radio_lsb.load(Ordering::Relaxed)
    }

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
            freq_hz: Some(st.freq_hz),
            squelch_open: Some(st.squelch_open),
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
        // la macro evento di chiusura ha preso il nome «OnEnd»: un nome vecchio che comincia cosi' e non e' piu' valido si riporta a quello
        for m in settings.macros.iter_mut().chain(settings.mode_macros.values_mut().flatten()) {
            if m.name.starts_with("OnEnd") && !valid_macro_name(&m.name) {
                m.name = "OnEnd".to_string();
            }
        }
        let (queue, rx) = sync_channel::<Msg>(128);
        let inner = Arc::new(Inner {
            settings_path,
            listening: AtomicBool::new(settings.auto_receive),
            settings: Mutex::new(settings.clone()),
            tx_active: AtomicBool::new(false),
            tx_abort: AtomicBool::new(false),
            queue,
            rx: Mutex::new(RxStatus { listening: settings.auto_receive, level_db: -90.0, freq_hz: settings.rx_freq_hz, squelch_open: true, ..RxStatus::default() }),
            tx: Mutex::new(TxStatus { state: "idle".into(), ..TxStatus::default() }),
            port: Mutex::new(None),
            emit: Mutex::new(None),
            radio_mode: Mutex::new(None),
            radio_lsb: AtomicBool::new(false),
            band: crate::band::BandLink::spawn(&settings),
        });
        let worker = Arc::clone(&inner);
        if let Err(e) = std::thread::Builder::new().name("aurora-digi".into()).spawn(move || rx_loop(&worker, rx, settings)) {
            tracing::error!("modi testuali: thread di ricezione non avviato: {e}");
        }
        DigiService { inner }
    }

    pub fn set_emitter(&self, emitter: Emitter) {
        self.inner.band.set_emitter(Arc::clone(&emitter));
        *lock(&self.inner.emit) = Some(emitter);
    }

    /// Chi riceve gli spot locali di tutta la banda (il pannello del DX Cluster di Aurora).
    pub fn set_spot_sink(&self, sink: crate::band::SpotSink) {
        self.inner.band.set_spot_sink(sink);
    }

    /// Il paese DXCC di un nominativo (bandiera e convalida dei nominativi di tutta la banda).
    pub fn set_call_lookup(&self, lookup: crate::band::CallLookup) {
        self.inner.band.set_call_lookup(lookup);
    }

    /// Le forme dei nominativi noti per paese (convalida dei nominativi di tutta la banda).
    pub fn set_call_forms(&self, forms: crate::skim::forms::FormTable) {
        self.inner.band.set_call_forms(forms);
    }

    /// I segnali di tutta la banda di adesso, con lo storico di 60 s.
    pub fn band_signals(&self) -> Vec<aurora_core::DigiSignal> {
        self.inner.band.signals()
    }

    pub fn set_tx_port(&self, port: Arc<dyn DigiTxPort>) {
        *lock(&self.inner.port) = Some(port);
    }

    /// Da dove leggere il modo della radio (CAT): in RTTY con la radio in LSB mark e space si invertono da soli. Si rilegge ogni secondo.
    pub fn set_radio_mode_source(&self, source: RadioModeSource) {
        *lock(&self.inner.radio_mode) = Some(source);
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
        self.inner.band.configure(&s);
        *lock(&self.inner.settings) = s;
        self.inner.rx_status_event();
        Ok(())
    }

    /// Rimette le macro di Aurora (`Defaults`). L'importazione da file non c'e' piu': `Replace` e `Add` rispondono con un errore.
    pub fn import_macros(&self, _folder: Option<&std::path::Path>, how: ImportHow) -> Result<crate::macro_files::Imported, DigiError> {
        if how != ImportHow::Defaults {
            return Err(DigiError::BadValue("l'importazione delle macro da file non c'e' piu': usa «Ripristina le macro di Aurora»".into()));
        }
        let mut next = self.settings();
        next.macros = default_macros();
        next.mode_macros = default_mode_macros();
        next.merge_macros.clear();
        self.apply_settings(next)?;
        Ok(crate::macro_files::Imported::default())
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
        self.inner.band.reset();
        self.inner.rx_status_event();
    }

    /// Il testo da trasmettere diventa audio con il modo e la frequenza scelti; `lsb`: la radio e' in LSB (RTTY: mark e space invertiti).
    pub fn wave_of(text: &str, s: &DigiSettings, lsb: bool) -> Vec<f32> {
        let bytes: Vec<u8> = text.chars().map(|c| if (c as u32) < 256 { c as u32 as u8 } else { b'?' }).collect();
        match (s.mode.psk(), s.mode) {
            (Some(mode), _) => psk::modulate(&bytes, s.tx_freq(), mode, 0.9),
            (None, DigiMode::Cw) => cw::modulate(&bytes, s.tx_freq(), s.cw_wpm, 0.9),
            (None, _) => rtty::modulate(&bytes, s.rtty(s.tx_freq(), lsb), 0.9),
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
        let lsb = self.inner.read_radio_lsb();
        let wave = Self::wave_of(text, &settings, lsb);
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
    Cw(Box<CwDemodulator>),
}

impl Demod {
    fn new(s: &DigiSettings, lsb: bool) -> Demod {
        if s.mode == DigiMode::Cw {
            let mut d = CwDemodulator::new(s.rx_freq_hz, s.cw_wpm);
            d.set_bandwidth(s.cw_bandwidth_hz);
            d.set_afc(s.afc_on);
            d.set_som(s.cw_som);
            // lo squelch del CW guarda il rapporto segnale/rumore misurato sullo spettro, non il livello
            d.set_squelch_snr_db(s.squelch_on.then_some(s.cw_squelch_snr_db));
            return Demod::Cw(Box::new(d));
        }
        match s.mode.psk() {
            Some(mode) => {
                let mut d = PskDemodulator::new(mode, s.rx_freq_hz);
                d.set_afc(s.afc_on);
                // lo squelch del PSK guarda la metrica di qualita' della fase di fldigi (0-100), non il livello
                d.set_squelch(s.squelch_on.then_some(s.psk_squelch));
                Demod::Psk(Box::new(d))
            }
            None => {
                let mut d = RttyDemodulator::new(s.rtty(s.rx_freq_hz, lsb));
                d.set_afc(s.afc_on);
                d.set_uos(s.rtty_uos);
                d.set_discriminator(s.rtty_receiver.discriminator(s.rtty_shift_hz));
                // lo squelch dell'RTTY guarda il rapporto segnale/rumore, come la «metric» di fldigi, non il livello
                d.set_squelch_snr_db(s.squelch_on.then_some(s.rtty_squelch_snr_db));
                Demod::Rtty(Box::new(d))
            }
        }
    }

    /// La frequenza su cui si ascolta adesso.
    fn frequency(&self) -> f32 {
        match self {
            Demod::Psk(d) => d.frequency(),
            Demod::Rtty(d) => d.frequency(),
            Demod::Cw(d) => d.frequency(),
        }
    }

    /// Rapporto segnale/rumore (dB) nella misura di ogni modo.
    fn snr_db(&self) -> Option<f32> {
        match self {
            Demod::Rtty(d) => Some(d.snr_db()),
            Demod::Psk(d) => d.snr_db(),
            Demod::Cw(d) => Some(d.snr_db()),
        }
    }

    /// Le misure in piu' di ogni modo: IMD e qualita' (PSK), velocita' (CW).
    fn extras(&self) -> (Option<f32>, Option<f32>, Option<f32>) {
        match self {
            Demod::Psk(d) => (d.imd_db(), Some(d.metric()), None),
            Demod::Cw(d) => (None, None, Some(d.wpm())),
            Demod::Rtty(_) => (None, None, None),
        }
    }

    /// Livello (dB) e squelch aperto.
    fn signal(&self) -> (f32, bool) {
        match self {
            Demod::Psk(d) => d.signal(),
            Demod::Rtty(d) => d.signal(),
            Demod::Cw(d) => d.signal(),
        }
    }

    fn process(&mut self, audio: &[f32]) -> Vec<u8> {
        match self {
            Demod::Psk(d) => d.process(audio),
            Demod::Rtty(d) => d.process(audio),
            Demod::Cw(d) => d.process(audio),
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
    let mut lsb = inner.read_radio_lsb();
    let mut last_radio = Instant::now();
    let mut demod = Demod::new(&settings, lsb);
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
                inner.band.feed(&audio);
                let text = demod.process(&audio);
                if !text.is_empty() {
                    let s: String = text.iter().map(|&b| b as char).collect();
                    inner.emit(DigiEvent { kind: "rx_text".into(), text: Some(s), mode: Some(mode.id().into()), ..DigiEvent::default() });
                }
            }
            Some(Msg::Config(s)) => {
                demod = Demod::new(&s, lsb);
                mode = s.mode;
            }
            Some(Msg::Reset) => demod = Demod::new(&lock(&inner.settings).clone(), lsb),
            None => {}
        }
        // RTTY: con la radio passata da USB a LSB (o viceversa) mark e space si scambiano da soli
        if mode == DigiMode::Rtty && last_radio.elapsed() >= Duration::from_secs(1) {
            last_radio = Instant::now();
            let now = inner.read_radio_lsb();
            if now != lsb {
                lsb = now;
                tracing::info!("RTTY: radio in {}, mark e space {}", if lsb { "LSB" } else { "USB" }, if lsb { "invertiti" } else { "normali" });
                demod = Demod::new(&lock(&inner.settings).clone(), lsb);
            }
        }
        if last_status.elapsed() >= Duration::from_millis(400) {
            last_status = Instant::now();
            {
                let mut rx = lock(&inner.rx);
                rx.level_db = peak_db;
                rx.freq_hz = demod.frequency();
                rx.squelch_open = demod.signal().1;
                rx.snr_db = demod.snr_db();
                (rx.imd_db, rx.quality, rx.wpm) = demod.extras();
                rx.radio_lsb = lsb;
            }
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
        assert_eq!(DigiMode::from_id("cw"), Some(DigiMode::Cw));
        assert_eq!(DigiMode::from_id("olivia"), None);
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
        s.rtty_baud = 45.45;
        s.cw_wpm = 2.0;
        assert!(s.validate().is_err());
    }

    #[test]
    fn le_macro_di_base_hanno_nomi_validi_e_le_posizioni_giuste() {
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
        let wave = DigiService::wave_of("CQ DE IU3ABC", &s, false);
        assert!(!wave.is_empty());
        let mut d = Demod::new(&s, false);
        let mut all = vec![0.0; 4000];
        all.extend(wave);
        all.extend(vec![0.0; 4000]);
        let out = String::from_utf8_lossy(&d.process(&all)).into_owned();
        assert!(out.contains("CQ DE IU3ABC"), "ricevuto: {out:?}");
    }

    #[test]
    fn rtty_in_usb_il_mark_e_il_tono_alto_e_in_lsb_si_inverte() {
        let s = DigiSettings { mode: DigiMode::Rtty, rx_freq_hz: 1500.0, tx_freq_hz: 1500.0, ..DigiSettings::default() };
        let usb = s.rtty(1500.0, false);
        assert_eq!((usb.mark_hz(), usb.space_hz()), (1585.0, 1415.0));
        let lsb = s.rtty(1500.0, true);
        assert_eq!((lsb.mark_hz(), lsb.space_hz()), (1415.0, 1585.0));
        // «Rev» in LSB rimette il mark in alto
        let rev = DigiSettings { rtty_reverse: true, ..s.clone() };
        assert_eq!(rev.rtty(1500.0, true).mark_hz(), 1585.0);
        assert_eq!(rev.rtty(1500.0, false).mark_hz(), 1415.0);
        // trasmissione e ricezione con le stesse impostazioni si capiscono, in USB e in LSB
        for lsb in [false, true] {
            let mut d = Demod::new(&s, lsb);
            let out = String::from_utf8_lossy(&d.process(&DigiService::wave_of("CQ DE IU3ABC 599", &s, lsb))).into_owned();
            assert!(out.contains("CQ DE IU3ABC 599"), "lsb {lsb}, ricevuto: {out:?}");
        }
    }

    #[test]
    fn i_modi_lsb_della_radio_si_riconoscono() {
        for m in ["LSB", "PKTLSB", "lsb", "DATA-L", "DIGL", "LSB-D", "ECSSLSB"] {
            assert!(is_lower_sideband(m), "{m}");
        }
        for m in ["USB", "PKTUSB", "DATA-U", "DIGU", "RTTY", "CW", "FM", ""] {
            assert!(!is_lower_sideband(m), "{m}");
        }
    }

    #[test]
    fn lo_squelch_vecchio_in_db_assoluti_lascia_il_posto_alle_soglie_relative() {
        // un digi.json di prima: squelch acceso a -45 dB sul livello; ora le soglie sono relative e valgono quelle di base
        let s: DigiSettings = serde_json::from_str(r#"{"mode":"psk31","squelch_on":true,"squelch_db":-45,"cw_wpm":22}"#).expect("json");
        assert!(s.squelch_on);
        assert_eq!(s.psk_squelch, psk::SQUELCH_DEFAULT);
        assert_eq!(s.cw_squelch_snr_db, cw::SQUELCH_DEFAULT_DB);
        assert_eq!(s.cw_bandwidth_hz, cw::BANDWIDTH_DEFAULT);
        assert!(!s.cw_som);
        assert!(s.validate().is_ok());
        let json = serde_json::to_string(&s).expect("json");
        assert!(!json.contains("\"squelch_db\""), "la soglia vecchia non si riscrive: {json}");
        let bad = DigiSettings { cw_bandwidth_hz: 120.0, ..s.clone() };
        assert!(bad.validate().is_err());
        let bad = DigiSettings { psk_squelch: 120.0, ..s };
        assert!(bad.validate().is_err());
    }

    #[test]
    fn le_impostazioni_rtty_vecchie_si_leggono_con_i_valori_di_base() {
        let s: DigiSettings = serde_json::from_str(r#"{"mode":"rtty","rx_freq_hz":1500,"rtty_shift_hz":170,"rtty_baud":45.45}"#).expect("json");
        assert!(!s.rtty_reverse);
        assert!(s.rtty_uos);
        assert_eq!(s.rtty_receiver, RttyReceiver::Auto);
        assert!(!s.rtty_receiver.discriminator(170.0));
        assert!(s.rtty_receiver.discriminator(23.0));
        let d: DigiSettings = serde_json::from_str(r#"{"rtty_receiver":"discriminator"}"#).expect("json");
        assert_eq!(d.rtty_receiver, RttyReceiver::Discriminator);
        assert!(s.validate().is_ok());
    }

    /// Issue #45: dall'FT8 alla pagina dei modi testuali con il CW scelto. La pagina spegne l'ascolto uscendo e lo riaccende rientrando:
    /// dopo lo spegnimento (che azzera il demodulatore) e la riaccensione l'audio arriva al decodificatore CW e il testo si legge.
    #[test]
    fn il_cw_decodifica_dopo_spegnimento_e_riaccensione_dell_ascolto() {
        let dir = std::env::temp_dir().join(format!("aurora-digi-cw45-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("cartella di prova");
        let s = DigiSettings { mode: DigiMode::Cw, cw_wpm: 20.0, ..DigiSettings::default() };
        std::fs::write(dir.join("digi.json"), serde_json::to_vec(&s).expect("json")).expect("digi.json");
        let svc = DigiService::open(&dir);
        let (tx, rx) = std::sync::mpsc::channel::<String>();
        let tx = Mutex::new(tx);
        svc.set_emitter(Arc::new(move |ev: DigiEvent| {
            if ev.kind == "rx_text"
                && let Some(t) = ev.text
            {
                let _ = lock(&tx).send(t);
            }
        }));
        // ascolto acceso (prima visita), spento (si torna all'FT8), riacceso (si rientra)
        svc.start_rx();
        svc.stop_rx();
        assert!(!svc.is_listening());
        svc.start_rx();
        assert!(svc.is_listening());
        let mut wave = vec![0.0; 6000];
        wave.extend(DigiService::wave_of("CQ IU3ABC", &s, false));
        wave.extend(vec![0.0; 12000]);
        for chunk in wave.chunks(1200) {
            svc.feed(RATE as u32, chunk);
            // la coda e' corta: si lascia al thread il tempo di consumare
            std::thread::sleep(Duration::from_millis(2));
        }
        let mut out = String::new();
        let deadline = Instant::now() + Duration::from_secs(10);
        while !out.contains("CQ IU3ABC") && Instant::now() < deadline {
            if let Ok(t) = rx.recv_timeout(Duration::from_millis(200)) {
                out.push_str(&t);
            }
        }
        let _ = std::fs::remove_dir_all(&dir);
        assert!(out.contains("CQ IU3ABC"), "ricevuto: {out:?}");
    }
}
