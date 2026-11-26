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

use crate::notch::{AfcGuard, Notch, RxFilters};
use crate::rtty::RttyCharset;
use crate::cw_hybrid::CwHybrid;
use crate::{CwDemodulator, PskDemodulator, PskMode, RATE, RttyDemodulator, RttyParams, cw, psk, rtty};

/// La trasmissione a flusso: aperta a levetta e scritta dal vivo, con l'avanzamento carattere per carattere (vedi `service/live.rs`).
mod live;
/// Il CW con il WinKeyer: il testo va al manipolatore esterno, l'avanzamento viene dalla sua eco (vedi `service/keyer.rs`).
mod keyer;
mod tcicw;

pub use keyer::{KeyerOpener, KeyerStatus};
pub use rig_control::winkeyer::{PaddleMode, WinKeyerConfig};

/// La catena di trasmissione vista dai modi testuali (la fornisce il programma: `tx-native` con il PTT scelto).
pub trait DigiTxPort: Send + Sync {
    /// Trasmette subito l'onda (12 kHz, mono, picco 1), fuori dai periodi FT8.
    fn start(&self, wave: Vec<f32>) -> Result<(), String>;
    fn halt(&self);
    fn is_busy(&self) -> bool;
    /// La catena sa trasmettere a flusso (`start_stream`, `append`, `close_stream`, `played`).
    fn can_stream(&self) -> bool {
        false
    }
    /// Trasmissione a flusso: `first` e' il primo pezzo, gli altri arrivano con `append`; `close_stream` dice che non ne arrivano altri.
    fn start_stream(&self, _first: Vec<f32>) -> Result<(), String> {
        Err("questa catena di trasmissione non trasmette a flusso".into())
    }
    fn append(&self, _wave: Vec<f32>) -> Result<(), String> {
        Err("questa catena di trasmissione non trasmette a flusso".into())
    }
    fn close_stream(&self) {}
    /// CW dal comando CWX della radio (TCI `cw_macros`): manda `text` alla velocita' `wpm`; la radio fa CW, PTT e tasto.
    fn cw_text(&self, _text: &str, _wpm: u32) -> Result<(), String> {
        Err("questa radio non ha il comando CW del TCI: serve il collegamento TCI".into())
    }
    /// Ferma il testo CW mandato con `cw_text`.
    fn cw_text_stop(&self) {}
    /// Quanti campioni (a 12 kHz) della trasmissione a flusso sono gia' usciti dalla scheda; `None` se la catena non lo sa.
    fn played(&self) -> Option<u64> {
        None
    }
}

pub type Emitter = Arc<dyn Fn(DigiEvent) + Send + Sync>;

/// Il ricevitore CW: «preciso» il ricevitore attuale, «veloce» la catena mostra subito un testo provvisorio che il ricevitore attuale
/// sostituisce con quello definitivo ([`crate::cw_hybrid`]).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CwReceiver {
    Precise,
    /// Di base: stesso testo finale del preciso, mostrato prima (banco CW, `docs/banco-cw.md`, nono giro).
    #[default]
    Fast,
}

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

/// La frequenza della radio in Hz (0 = non nota): cambiata, i notch si tolgono.
pub type DialSource = Arc<dyn Fn() -> u64 + Send + Sync>;

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
    /// Fonia: niente decodifica e niente testo trasmesso; serve al log e ai contest (parla l'operatore al microfono).
    Ssb,
    /// Olivia (MFSK con FEC, a blocchi): toni e banda in [`DigiSettings::olivia`].
    Olivia,
    /// Contestia (come Olivia con blocchi piu' corti e 64 caratteri): toni e banda in [`DigiSettings::contestia`].
    Contestia,
    /// QPSK31: PSK a quattro fasi con codice convoluzionale (vedi `qpsk`); in LSB le fasi girano al contrario.
    Qpsk31,
    /// Throb e ThrobX (coppie di toni lentissime, vedi `throb`): la variante in [`DigiSettings::throb`]; in LSB i toni si rovesciano.
    Throb,
    /// Amtor FEC (SITOR-B, vedi `amtor`): FSK 100 baud, 170 Hz; in LSB mark e space si scambiano.
    Amtor,
}

impl DigiMode {
    pub const ALL: [DigiMode; 10] = [
        DigiMode::Psk31,
        DigiMode::Psk63,
        DigiMode::Rtty,
        DigiMode::Cw,
        DigiMode::Ssb,
        DigiMode::Olivia,
        DigiMode::Contestia,
        DigiMode::Qpsk31,
        DigiMode::Throb,
        DigiMode::Amtor,
    ];

    pub fn id(self) -> &'static str {
        match self {
            DigiMode::Psk31 => "psk31",
            DigiMode::Psk63 => "psk63",
            DigiMode::Rtty => "rtty",
            DigiMode::Cw => "cw",
            DigiMode::Ssb => "ssb",
            DigiMode::Olivia => "olivia",
            DigiMode::Contestia => "contestia",
            DigiMode::Qpsk31 => "qpsk31",
            DigiMode::Throb => "throb",
            DigiMode::Amtor => "amtor",
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            DigiMode::Psk31 => "PSK31",
            DigiMode::Psk63 => "PSK63",
            DigiMode::Rtty => "RTTY",
            DigiMode::Cw => "CW",
            DigiMode::Ssb => "SSB",
            DigiMode::Olivia => "Olivia",
            DigiMode::Contestia => "Contestia",
            DigiMode::Qpsk31 => "QPSK31",
            DigiMode::Throb => "Throb",
            DigiMode::Amtor => "Amtor",
        }
    }

    pub fn from_id(id: &str) -> Option<DigiMode> {
        DigiMode::ALL.into_iter().find(|m| m.id() == id.to_ascii_lowercase())
    }

    fn psk(self) -> Option<PskMode> {
        match self {
            DigiMode::Psk31 => Some(PskMode::Psk31),
            DigiMode::Psk63 => Some(PskMode::Psk63),
            DigiMode::Qpsk31 => Some(PskMode::Qpsk31),
            DigiMode::Rtty | DigiMode::Cw | DigiMode::Ssb | DigiMode::Olivia | DigiMode::Contestia | DigiMode::Throb | DigiMode::Amtor => None,
        }
    }

    /// Il modo ha un ricevitore e un trasmettitore di testo (in SSB parla l'operatore).
    pub fn is_text(self) -> bool {
        self != DigiMode::Ssb
    }

    /// Olivia o Contestia: quale delle due varianti.
    pub fn olivia_variant(self) -> Option<crate::olivia::Variant> {
        match self {
            DigiMode::Olivia => Some(crate::olivia::Variant::Olivia),
            DigiMode::Contestia => Some(crate::olivia::Variant::Contestia),
            _ => None,
        }
    }
}

/// I toni ammessi di Olivia e Contestia.
pub const OLIVIA_TONES: [u32; 5] = [4, 8, 16, 32, 64];
/// Le bande ammesse di Olivia e Contestia (Hz).
pub const OLIVIA_BANDWIDTHS: [u32; 4] = [250, 500, 1000, 2000];

/// Le impostazioni di Olivia o di Contestia (una copia per modo).
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct OliviaSettings {
    /// Numero di toni: 4, 8, 16, 32 o 64.
    pub tones: u32,
    /// Banda in Hz: 250, 500, 1000 o 2000 (la frequenza di ricezione e di trasmissione e' il centro).
    pub bandwidth_hz: u32,
    /// Di quanti bin si cerca il segnale attorno alla frequenza scelta (2-32; 8 come fldigi).
    pub sync_margin: u32,
    /// Su quanti blocchi si integra il sincronismo (2-32; 4 come fldigi).
    pub sync_integration: u32,
    /// Soglia dello squelch sul rapporto segnale/rumore del sincronismo (dB, 0-40): sotto non si stampa niente. Con lo squelch spento vale
    /// [`OliviaSettings::SQUELCH_DEFAULT_DB`].
    pub squelch_snr_db: f32,
}

impl OliviaSettings {
    /// La soglia di base dello squelch (dB): sopra il rapporto che il sincronismo trova nel solo rumore (fino a circa 10 dB), sotto quello
    /// di un segnale debole (17 dB e oltre; prove `lo_squelch_di_olivia_*` in `olivia_link`).
    pub const SQUELCH_DEFAULT_DB: f32 = 12.0;

    pub fn olivia() -> OliviaSettings {
        OliviaSettings { tones: 8, bandwidth_hz: 250, sync_margin: 8, sync_integration: 4, squelch_snr_db: OliviaSettings::SQUELCH_DEFAULT_DB }
    }

    pub fn contestia() -> OliviaSettings {
        OliviaSettings { tones: 16, bandwidth_hz: 500, ..OliviaSettings::olivia() }
    }

    fn is_valid(&self) -> bool {
        OLIVIA_TONES.contains(&self.tones)
            && OLIVIA_BANDWIDTHS.contains(&self.bandwidth_hz)
            && (2..=32).contains(&self.sync_margin)
            && (2..=32).contains(&self.sync_integration)
            && self.squelch_snr_db.is_finite()
            && (0.0..=40.0).contains(&self.squelch_snr_db)
    }

    /// I parametri del modulatore e del ricevitore centrati su `centre_hz`.
    pub fn params(&self, variant: crate::olivia::Variant, centre_hz: f32) -> crate::olivia::Params {
        crate::olivia::Params { variant, tones: self.tones as usize, bandwidth: self.bandwidth_hz as usize, center_hz: f64::from(centre_hz) }
    }
}

impl Default for OliviaSettings {
    fn default() -> Self {
        OliviaSettings::olivia()
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
    /// In SSB non si trasmette testo.
    NoTextMode,
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
            DigiError::NoTextMode => write!(f, "in SSB non si trasmette testo: parla al microfono"),
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
            DigiError::NoTextMode => "no_text_mode",
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

/// L'uscita della trasmissione CW.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CwOutput {
    /// Audio della scheda, con il PTT di Aurora.
    #[default]
    Audio,
    /// Manipolatore esterno WinKeyer (K1EL WK2/WK3 e compatibili): CW, PTT e tasto li fa lui.
    WinKeyer,
    /// Il comando CWX della radio via TCI (`cw_macros`: Flex, ExpertSDR): la radio fa CW, PTT e tasto. Serve il collegamento TCI.
    Tci,
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
    /// RTTY: il set di caratteri (ITA2 di base, minuscole, cirillico, svedese).
    pub rtty_charset: RttyCharset,
    /// RTTY: in attesa di testo (trasmissione aperta con la casella vuota, o testo finito) si manda il «diddle» (LTRS), come MMTTY e fldigi.
    /// Spento si manda il solo tono di riposo (mark). Acceso di base.
    pub rtty_diddle: bool,
    /// RTTY: i quattro profili della finestra «Impostazioni RTTY» e quello scelto (0-3). Il profilo scelto e' gia' copiato nei campi
    /// qui sopra e in `afc_on`, `snap_on`, `squelch_on`: il motore usa quelli.
    pub rtty_profile: u8,
    pub rtty_profiles: Vec<RttyProfile>,
    /// Velocita' della CW in parole al minuto (trasmissione; in ricezione e' il valore di partenza, poi si adatta al segnale).
    pub cw_wpm: f32,
    /// CW: la velocita' di trasmissione segue quella misurata in ricezione (interruttore «TX = RX» della barra; spento di base).
    pub cw_wpm_follow_rx: bool,
    /// CW: banda del filtro di ricezione (Hz: 50, 100, 150, 250 o 500; 150 di base come fldigi).
    pub cw_bandwidth_hz: f32,
    /// CW: soglia dello squelch sul rapporto segnale/rumore (dB in 3 kHz, media nel tempo; -30..+20).
    pub cw_squelch_snr_db: f32,
    /// CW: decodifica SOM di fldigi (aiuta col tasto a mano irregolare).
    pub cw_som: bool,
    /// CW: il ricevitore (preciso o veloce con testo provvisorio).
    pub cw_receiver: CwReceiver,
    /// CW: da dove esce la trasmissione, audio della scheda o WinKeyer.
    pub cw_output: CwOutput,
    /// CW: le impostazioni del WinKeyer.
    pub winkeyer: WinKeyerConfig,
    /// Anti-click della cascata (menu Vista): i click di manipolazione si tolgono dalle righe mentre la pagina ascolta.
    pub wf_anticlick: bool,
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
    /// RTTY: si decodificano tutti i segnali della banda audio («Radar» RTTY, thread a parte con limite di CPU).
    pub band_rtty: bool,
    /// Tutta la banda: i nominativi verificati vanno come spot locali nel pannello del DX Cluster di Aurora (mai in rete).
    /// (Prima `band_spots`, acceso di base: il nome nuovo fa ripartire tutti da spento finche' non lo si accende.)
    pub local_spots: bool,
    /// Tutta la banda: spot locali solo delle stazioni che chiamano CQ.
    pub band_spots_cq_only: bool,
    /// Tutta la banda: livello di convalida dei nominativi (`minimal`, `normal`, `strict`, `maximum`).
    pub band_validation: String,
    /// Tutta la banda (Aurora BIGScan): sensibilita' della soglia che apre i canali (`high`, `medium`, `low`; `medium` = come prima).
    pub band_sensitivity: String,
    /// Passabanda «a coltello» intorno alla frequenza di ricezione, davanti al ricevitore (non a tutta la banda).
    pub passband_on: bool,
    /// Larghezza del passabanda (Hz, 20-1000); 0 = quella di base del modo (`notch::default_passband`).
    pub passband_hz: f32,
    /// Olivia: toni, banda, sincronismo e squelch.
    pub olivia: OliviaSettings,
    /// Contestia: toni, banda, sincronismo e squelch.
    pub contestia: OliviaSettings,
    /// Throb: la variante (Throb a 1, 2, 4 simboli al secondo o ThrobX).
    pub throb: crate::throb::ThrobVariant,
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
            rtty_squelch_snr_db: 0.0,
            rtty_receiver: RttyReceiver::Auto,
            rtty_charset: RttyCharset::Ita2,
            rtty_diddle: true,
            rtty_profile: 0,
            rtty_profiles: RttyProfile::factory(),
            cw_wpm: 20.0,
            cw_wpm_follow_rx: false,
            cw_bandwidth_hz: cw::BANDWIDTH_DEFAULT,
            cw_squelch_snr_db: cw::SQUELCH_DEFAULT_DB,
            cw_som: false,
            cw_receiver: CwReceiver::Fast,
            cw_output: CwOutput::Audio,
            winkeyer: WinKeyerConfig::default(),
            wf_anticlick: true,
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
            band_rtty: true,
            // spenti di base: gli spot locali partono solo se l'utente li accende (proprietario, 2026-11-14)
            local_spots: false,
            band_spots_cq_only: false,
            band_validation: "normal".into(),
            band_sensitivity: "medium".into(),
            passband_on: false,
            passband_hz: 0.0,
            olivia: OliviaSettings::olivia(),
            contestia: OliviaSettings::contestia(),
            throb: crate::throb::ThrobVariant::default(),
        }
    }
}

/// Da 10 baud (il profilo lento dei programmi classici) a 300.
fn passband_ok(hz: f32) -> bool {
    hz == 0.0 || (hz.is_finite() && (20.0..=1000.0).contains(&hz))
}

fn rtty_baud_ok(baud: f32) -> bool {
    baud.is_finite() && (10.0..=300.0).contains(&baud)
}

/// Un profilo della finestra «Impostazioni RTTY» (Set 1-4): tutto quello che serve per passare da un tipo di segnale a un altro.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct RttyProfile {
    pub shift_hz: f32,
    pub baud: f32,
    pub reverse: bool,
    pub receiver: RttyReceiver,
    pub charset: RttyCharset,
    pub afc: bool,
    pub snap: bool,
    pub squelch: bool,
    pub uos: bool,
    pub squelch_snr_db: f32,
}

impl Default for RttyProfile {
    fn default() -> Self {
        RttyProfile {
            shift_hz: 170.0,
            baud: 45.45,
            reverse: false,
            receiver: RttyReceiver::Auto,
            charset: RttyCharset::Ita2,
            afc: true,
            snap: true,
            squelch: false,
            uos: true,
            squelch_snr_db: 0.0,
        }
    }
}

impl RttyProfile {
    /// I quattro profili di fabbrica: 170 Hz / 45,45 baud; 22 Hz / 45,45 con il discriminatore; 30 Hz / 10 baud; 170 Hz / 45,45.
    pub fn factory() -> Vec<RttyProfile> {
        vec![
            RttyProfile::default(),
            RttyProfile { shift_hz: 22.0, receiver: RttyReceiver::Discriminator, ..RttyProfile::default() },
            RttyProfile { shift_hz: 30.0, baud: 10.0, ..RttyProfile::default() },
            RttyProfile::default(),
        ]
    }

    fn is_valid(&self) -> bool {
        self.shift_hz.is_finite()
            && (20.0..=1000.0).contains(&self.shift_hz)
            && rtty_baud_ok(self.baud)
            && self.squelch_snr_db.is_finite()
            && (0.0..=40.0).contains(&self.squelch_snr_db)
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
        if !rtty_baud_ok(self.rtty_baud) {
            return Err(DigiError::BadValue("velocita' RTTY fuori da 10-300 baud".into()));
        }
        if !self.rtty_squelch_snr_db.is_finite() || !(0.0..=40.0).contains(&self.rtty_squelch_snr_db) {
            return Err(DigiError::BadValue("soglia S/N dello squelch RTTY fuori da 0-40 dB".into()));
        }
        if self.rtty_profile > 3 || self.rtty_profiles.len() > 4 || !self.rtty_profiles.iter().all(RttyProfile::is_valid) {
            return Err(DigiError::BadValue("profili RTTY non validi".into()));
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
        if crate::skim::Sensitivity::from_id(&self.band_sensitivity).is_none() {
            return Err(DigiError::BadValue("sensibilita' di BIGScan non fra high, medium e low".into()));
        }
        if self.mode_macros.keys().any(|k| DigiMode::from_id(k).is_none()) {
            return Err(DigiError::BadValue("insieme di macro per un modo sconosciuto".into()));
        }
        if !passband_ok(self.passband_hz) {
            return Err(DigiError::BadValue("larghezza del passabanda fuori da 20-1000 Hz (0 = di base)".into()));
        }
        if !self.olivia.is_valid() || !self.contestia.is_valid() {
            return Err(DigiError::BadValue(
                "Olivia/Contestia: toni non fra 4, 8, 16, 32 e 64, banda non fra 250, 500, 1000 e 2000 Hz, sincronismo fuori da 2-32 o squelch fuori da 0-40 dB".into(),
            ));
        }
        Ok(())
    }

    /// Le stesse impostazioni con i valori che [`DigiSettings::validate`] rifiuta riportati a quelli di base, uno per uno (per un `digi.json`
    /// scritto a mano o rovinato). Se resta qualcosa di non valido (le macro), tutto torna ai valori di base.
    fn repaired(mut self) -> Self {
        let base = DigiSettings::default();
        let in_band = |f: f32| f.is_finite() && (200.0..=3500.0).contains(&f);
        if !in_band(self.rx_freq_hz) {
            self.rx_freq_hz = base.rx_freq_hz;
        }
        if !in_band(self.tx_freq_hz) {
            self.tx_freq_hz = base.tx_freq_hz;
        }
        if !self.rtty_shift_hz.is_finite() || !(20.0..=1000.0).contains(&self.rtty_shift_hz) {
            self.rtty_shift_hz = base.rtty_shift_hz;
        }
        if !self.psk_squelch.is_finite() || !(0.0..=100.0).contains(&self.psk_squelch) {
            self.psk_squelch = base.psk_squelch;
        }
        if !self.cw_squelch_snr_db.is_finite() || !(-30.0..=20.0).contains(&self.cw_squelch_snr_db) {
            self.cw_squelch_snr_db = base.cw_squelch_snr_db;
        }
        if !cw::BANDWIDTHS.contains(&self.cw_bandwidth_hz) {
            self.cw_bandwidth_hz = base.cw_bandwidth_hz;
        }
        if !rtty_baud_ok(self.rtty_baud) {
            self.rtty_baud = base.rtty_baud;
        }
        if !self.rtty_squelch_snr_db.is_finite() || !(0.0..=40.0).contains(&self.rtty_squelch_snr_db) {
            self.rtty_squelch_snr_db = base.rtty_squelch_snr_db;
        }
        if self.rtty_profile > 3 || self.rtty_profiles.len() > 4 || !self.rtty_profiles.iter().all(RttyProfile::is_valid) {
            self.rtty_profile = base.rtty_profile;
            self.rtty_profiles = base.rtty_profiles.clone();
        }
        if !self.cw_wpm.is_finite() || !(cw::WPM_MIN..=cw::WPM_MAX).contains(&self.cw_wpm) {
            self.cw_wpm = base.cw_wpm;
        }
        if crate::skim::calls::Validation::from_id(&self.band_validation).is_none() {
            self.band_validation = base.band_validation.clone();
        }
        if crate::skim::Sensitivity::from_id(&self.band_sensitivity).is_none() {
            self.band_sensitivity = base.band_sensitivity.clone();
        }
        if !passband_ok(self.passband_hz) {
            self.passband_hz = base.passband_hz;
        }
        if !self.olivia.is_valid() {
            self.olivia = base.olivia;
        }
        if !self.contestia.is_valid() {
            self.contestia = base.contestia;
        }
        self.mode_macros.retain(|k, _| DigiMode::from_id(k).is_some());
        if self.validate().is_ok() { self } else { base }
    }

    fn tx_freq(&self) -> f32 {
        if self.tx_follow_rx { self.rx_freq_hz } else { self.tx_freq_hz }
    }

    /// Le impostazioni di Olivia o Contestia per il modo scelto (`None` negli altri modi).
    pub fn olivia_settings(&self) -> Option<(crate::olivia::Variant, &OliviaSettings)> {
        match self.mode {
            DigiMode::Olivia => Some((crate::olivia::Variant::Olivia, &self.olivia)),
            DigiMode::Contestia => Some((crate::olivia::Variant::Contestia, &self.contestia)),
            _ => None,
        }
    }

    /// I parametri RTTY sul centro dato; `lsb`: la radio e' in LSB e mark e space si invertono (oltre a «Rev»).
    /// Il passabanda acceso (basso, alto) intorno alla frequenza di ricezione scelta.
    fn passband(&self) -> Option<(f32, f32)> {
        if !self.passband_on {
            return None;
        }
        let w = match (self.passband_hz > 0.0, self.olivia_settings()) {
            (true, _) => self.passband_hz,
            // Olivia e Contestia: la banda del modo con un po' di margine per il sincronismo
            (false, Some((_, o))) => o.bandwidth_hz as f32 * 1.2,
            // Throb: dal primo all'ultimo tono piu' il margine dell'AFC
            (false, None) if self.mode == DigiMode::Throb => (self.throb.bandwidth_hz() + 2.0 * crate::throb::afc_limit_hz(self.throb) + 20.0) as f32,
            (false, None) => crate::notch::default_passband(self.mode, self.rtty_shift_hz, self.rtty_baud),
        };
        Some((self.rx_freq_hz - w / 2.0, self.rx_freq_hz + w / 2.0))
    }

    /// Di quanto l'AFC puo' allontanarsi dalla frequenza scelta: ±15 Hz in PSK31 (in proporzione al baud), meta' spostamento in RTTY, ±50 Hz in CW.
    fn afc_limit(&self) -> f32 {
        match self.mode {
            DigiMode::Psk31 | DigiMode::Qpsk31 => 15.0,
            DigiMode::Psk63 => 30.0,
            DigiMode::Rtty => self.rtty_shift_hz / 2.0,
            DigiMode::Cw => 50.0,
            DigiMode::Ssb => 0.0,
            // il ricevitore cerca da solo il segnale entro il margine di sincronismo: l'AFC non lo sposta
            DigiMode::Olivia | DigiMode::Contestia => 0.0,
            DigiMode::Throb => crate::throb::afc_limit_hz(self.throb) as f32,
            DigiMode::Amtor => crate::amtor::AFC_LIMIT_HZ as f32,
        }
    }

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
    /// C'e' un segnale sulla frequenza di ricezione (misura di `presence` con isteresi, indipendente dallo squelch): la macro <TXE>
    /// aspetta che cada prima di trasmettere.
    pub carrier: bool,
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct TxStatus {
    /// `idle`, `sending`, `done`, `aborted`, `error`.
    pub state: String,
    pub progress: f32,
    pub elapsed_s: f32,
    pub total_s: f32,
    pub message: Option<String>,
    /// Trasmissione aperta (levetta RX/TX): resta in trasmissione finche' non si chiude, e il testo scritto parte mentre si scrive.
    pub open: bool,
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
    /// Il WinKeyer: collegato o no e versione del firmware.
    pub keyer: KeyerStatus,
}

/// Una finestra RX in piu': un ricevitore che decodifica un'altra frequenza della stessa banda audio, accanto a quello principale.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct ExtraRx {
    pub id: u32,
    pub freq_hz: f32,
    /// L'AFC di questa finestra (suo, non quello della ricezione principale): segue la sua stazione.
    #[serde(default)]
    pub afc: bool,
}

/// Ogni quanto il servizio guarda la frequenza della radio (notch, canali di tutta la banda).
const DIAL_CHECK: Duration = Duration::from_millis(200);

/// Spostamento della radio (Hz) da cui l'elenco di tutta la banda si ripulisce invece di spostare i toni: piu' della banda audio.
const BAND_CLEAR_HZ: u64 = 3_500;

/// Cosa fare dell'elenco di tutta la banda quando la radio passa da `from` a `to` (Hz): `None` = ripulire (altra banda o salto grande),
/// `Some(hz)` = spostare i toni audio di `hz` (in USB il contrario dello spostamento della radio, in LSB lo stesso).
fn band_follow(from: u64, to: u64, lsb: bool) -> Option<f32> {
    let band = aurora_core::adif::band_from_freq_hz;
    if from.abs_diff(to) >= BAND_CLEAR_HZ || band(from).is_none() || band(from) != band(to) {
        return None;
    }
    let delta = to as f64 - from as f64;
    Some((if lsb { delta } else { -delta }) as f32)
}

/// Quante finestre RX in piu' al massimo (ognuna e' un ricevitore completo).
pub const EXTRA_RX_MAX: usize = 4;

impl ExtraRx {
    pub fn is_valid(&self) -> bool {
        self.id > 0 && self.freq_hz.is_finite() && (100.0..=5_900.0).contains(&self.freq_hz)
    }
}

enum Msg {
    Audio(u32, Vec<f32>),
    Notches(Vec<Notch>),
    Extra(Vec<ExtraRx>),
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
    /// La trasmissione a flusso in corso: il testo ancora da modulare e se e' aperta.
    session: Mutex<Option<live::Session>>,
    /// Il WinKeyer aperto (CW con uscita WinKeyer): si apre alla prima trasmissione e resta aperto.
    keyer: Mutex<Option<rig_control::winkeyer::WinKeyer>>,
    /// Come si apre il WinKeyer (la porta seriale; le prove ne mettono uno finto).
    keyer_opener: Mutex<KeyerOpener>,
    /// Il thread di tutta la banda (CW, PSK).
    band: crate::band::BandLink,
    /// I notch della ricezione: non si salvano, si azzerano all'avvio e a ogni cambio di frequenza della radio.
    notches: Mutex<Vec<Notch>>,
    /// Le finestre RX in piu' (non si salvano).
    extra: Mutex<Vec<ExtraRx>>,
    /// Da dove leggere la frequenza della radio (Hz, 0 = non nota).
    dial: Mutex<Option<DialSource>>,
    /// La registrazione dell'audio ricevuto (`<STARTRECORD>`) e l'ultimo minuto per `<REPLAY>`.
    recorder: Mutex<crate::recorder::Recorder>,
    /// La cartella delle registrazioni (`recordings` nei dati).
    rec_dir: std::path::PathBuf,
    /// Le pagine PSK · RTTY · CW aperte (finestra principale, staccata, telefono): lasciata l'ultima, l'ascolto si spegne.
    users: aurora_core::users::PageUsers,
}

fn lock<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|p| p.into_inner())
}

impl Inner {
    fn stop_rx(&self) {
        self.listening.store(false, Ordering::Relaxed);
        self.sync_anticlick();
        lock(&self.rx).listening = false;
        let _ = self.queue.send(Msg::Reset);
        self.band.reset();
        self.rx_status_event();
    }

    /// Nessuna pagina dei modi testuali aperta: l'ascolto (ricevitore e tutta la banda) si spegne, salvo «Ascolto acceso all'avvio di
    /// Aurora». Una trasmissione in corso continua: spegnere l'ascolto non la tocca.
    fn page_left(&self) {
        if self.listening.load(Ordering::Relaxed) && !lock(&self.settings).auto_receive {
            tracing::info!("modi testuali: nessuna pagina aperta, ascolto spento");
            self.stop_rx();
        }
    }

    /// L'anti-click della cascata vale mentre la pagina ascolta, se e' scelto (l'analizzatore dello spettro legge l'opzione comune).
    fn sync_anticlick(&self) {
        let on = self.listening.load(Ordering::Relaxed) && lock(&self.settings).wf_anticlick;
        aurora_core::waterfall::set_anticlick(on);
    }

    /// Legge adesso il modo della radio (se c'e' da dove) e ricorda se e' LSB; senza radio resta l'ultima lettura (di base USB).
    fn read_radio_lsb(&self) -> bool {
        let source = lock(&self.radio_mode).clone();
        if let Some(mode) = source.and_then(|f| f()) {
            self.radio_lsb.store(is_lower_sideband(&mode), Ordering::Relaxed);
        }
        self.radio_lsb.load(Ordering::Relaxed)
    }

    fn set_notches(&self, notches: Vec<Notch>) {
        *lock(&self.notches) = notches.clone();
        let _ = self.queue.send(Msg::Notches(notches));
        self.emit(DigiEvent { kind: "notches".into(), ..DigiEvent::default() });
    }

    fn read_dial(&self) -> u64 {
        let source = lock(&self.dial).clone();
        source.map_or(0, |f| f())
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

/// I modi che hanno un file di macro loro (`psk31.macro`...).
fn macro_modes() -> Vec<&'static str> {
    DigiMode::ALL.iter().map(|m| m.id()).collect()
}

/// Il nome della copia di sicurezza di `digi.json` fatta prima di spostare le macro nei file.
pub const DIGI_JSON_BACKUP: &str = "digi.json.prima-dei-file-macro.bak";

/// `digi.json` senza le macro (stanno nei file della cartella `macro/`); `merge_macros` resta.
fn settings_json(s: &DigiSettings) -> Result<Vec<u8>, serde_json::Error> {
    let mut v = serde_json::to_value(s)?;
    if let Some(o) = v.as_object_mut() {
        o.remove("macros");
        o.remove("mode_macros");
    }
    serde_json::to_vec_pretty(&v)
}

fn save_macro_files(data_dir: &std::path::Path, s: &DigiSettings) -> std::io::Result<()> {
    crate::macro_files::save_dir(&data_dir.join(crate::macro_files::MACRO_DIR), &s.macros, &s.mode_macros, &macro_modes())
}

/// Le macro vengono dai file di `macro/`. La prima volta (cartella senza `comuni.macro`) quelle di `digi.json`, o quelle di base, si scrivono
/// nei file; se `digi.json` le conteneva se ne fa prima una copia di sicurezza e poi lo si riscrive senza.
fn load_or_migrate_macros(data_dir: &std::path::Path, raw_json: Option<&[u8]>, settings: &mut DigiSettings) {
    let dir = data_dir.join(crate::macro_files::MACRO_DIR);
    if let Some((common, per_mode)) = crate::macro_files::load_dir(&dir, &macro_modes()) {
        // un file scritto a mano con un tasto sconosciuto o troppe voci: quelle voci si saltano, il resto vale
        let keep = |mut l: Vec<DigiMacro>| {
            let n = l.len();
            for m in l.iter_mut().filter(|m| m.name.starts_with("OnEnd") && !valid_macro_name(&m.name)) {
                m.name = "OnEnd".to_string();
            }
            l.retain(|m| valid_macro_name(&m.name) && m.label.len() <= 60 && m.text.len() <= 4000);
            l.truncate(100);
            if l.len() != n {
                tracing::warn!("macro: {} voci non valide saltate in {}", n - l.len(), dir.display());
            }
            l
        };
        settings.macros = keep(common);
        settings.mode_macros = per_mode.into_iter().map(|(k, v)| (k, keep(v))).collect();
        // i vecchi `<EXEC:Backup.bat>` (copia di sicurezza fatta da un programma esterno) si tolgono dai file: Aurora non li esegue e fa
        // da sola la copia all'avvio. Prima di riscrivere si tiene una copia dei file com'erano
        let mut changed = crate::macro_files::togli_exec(&mut settings.macros);
        for l in settings.mode_macros.values_mut() {
            changed |= crate::macro_files::togli_exec(l);
        }
        if changed {
            let old = dir.join("prima-di-togliere-exec");
            if !old.exists()
                && std::fs::create_dir_all(&old).is_ok()
                && let Ok(rd) = std::fs::read_dir(&dir)
            {
                for e in rd.flatten().filter(|e| e.path().extension().is_some_and(|x| x == crate::macro_files::EXTENSION)) {
                    let _ = std::fs::copy(e.path(), old.join(e.file_name()));
                }
            }
            match save_macro_files(data_dir, settings) {
                Ok(()) => tracing::info!("macro: tolti i comandi EXEC dai file di {}", dir.display()),
                Err(e) => tracing::warn!("macro: file senza EXEC non riscritti: {e}"),
            }
        }
        return;
    }
    let json_has_macros = raw_json
        .and_then(|b| serde_json::from_slice::<serde_json::Value>(b).ok())
        .and_then(|v| v.as_object().map(|o| o.contains_key("macros") || o.contains_key("mode_macros")))
        .unwrap_or(false);
    let json_path = data_dir.join("digi.json");
    crate::macro_files::togli_exec(&mut settings.macros);
    for l in settings.mode_macros.values_mut() {
        crate::macro_files::togli_exec(l);
    }
    if json_has_macros {
        let backup = data_dir.join(DIGI_JSON_BACKUP);
        if !backup.exists() {
            if let Err(e) = std::fs::copy(&json_path, &backup) {
                tracing::warn!("macro: copia di sicurezza di digi.json non riuscita ({e}): le macro restano in digi.json");
                return;
            }
        }
    }
    if let Err(e) = save_macro_files(data_dir, settings) {
        tracing::warn!("macro: file in {} non scritti: {e}", dir.display());
        return;
    }
    if json_has_macros {
        match settings_json(settings) {
            Ok(json) => {
                if let Err(e) = std::fs::write(&json_path, json) {
                    tracing::warn!("macro: digi.json non riscritto senza le macro: {e}");
                }
            }
            Err(e) => tracing::warn!("macro: digi.json non riscritto: {e}"),
        }
        tracing::info!("macro: spostate da digi.json in {} (copia di sicurezza {DIGI_JSON_BACKUP})", dir.display());
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
        let raw = std::fs::read(&settings_path).ok();
        let mut settings: DigiSettings = raw.as_deref().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or_default();
        load_or_migrate_macros(data_dir, raw.as_deref(), &mut settings);
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
        // un file scritto a mano o rovinato (velocita' RTTY 0, frequenza fuori banda...) faceva cadere il thread di ricezione al primo blocco
        // audio: i valori fuori dai limiti tornano quelli di base, il resto resta com'era
        if let Err(e) = settings.validate() {
            tracing::warn!("modi testuali: {} con valori non validi ({e}): quelli tornano ai valori di base", settings_path.display());
            settings = settings.repaired();
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
            session: Mutex::new(None),
            keyer: Mutex::new(None),
            keyer_opener: Mutex::new(keyer::serial_opener()),
            band: crate::band::BandLink::spawn(&settings),
            notches: Mutex::new(Vec::new()),
            extra: Mutex::new(Vec::new()),
            dial: Mutex::new(None),
            recorder: Mutex::new(crate::recorder::Recorder::default()),
            rec_dir: data_dir.join("recordings"),
            users: aurora_core::users::PageUsers::new(aurora_core::users::PAGE_TTL),
        });
        let worker = Arc::clone(&inner);
        inner.sync_anticlick();
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

    /// «Pulisci» del pannello Segnali: i canali di tutta la banda ripartono da zero (elenco, storico e nominativi).
    pub fn clear_band(&self) {
        self.inner.band.reset();
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
        // un campione non finito (TCI, driver) bloccherebbe per sempre i filtri dei ricevitori: vale zero, come nella cascata
        let copy: Vec<f32> = samples.iter().map(|&x| if x.is_finite() { x } else { 0.0 }).collect();
        lock(&self.inner.recorder).push(rate, &copy);
        if let Err(TrySendError::Full(_)) = self.inner.queue.try_send(Msg::Audio(rate, copy)) {
            tracing::debug!("modi testuali: blocco audio perso");
        }
    }

    /// `<STARTRECORD:nome>`: comincia a registrare l'audio ricevuto in `recordings/nome.wav` (senza nome: data e ora).
    pub fn start_record(&self, name: &str) -> Result<std::path::PathBuf, DigiError> {
        let path = self.inner.rec_dir.join(name);
        lock(&self.inner.recorder).start(path.clone()).map_err(DigiError::Tx)?;
        tracing::info!("macro: registrazione in {}", path.display());
        Ok(path)
    }

    /// `<STOPRECORD>`: chiude la registrazione e la salva.
    pub fn stop_record(&self) -> Result<Option<std::path::PathBuf>, DigiError> {
        lock(&self.inner.recorder).stop().map_err(DigiError::Tx)
    }

    /// `<REPLAY:n>`: gli ultimi n secondi ricevuti ripassano nel ricevitore (si rilegge un segnale perso).
    pub fn replay(&self, seconds: f32) -> usize {
        let (rate, samples) = lock(&self.inner.recorder).last(seconds);
        if rate == 0 || samples.is_empty() {
            return 0;
        }
        let n = samples.len();
        for chunk in samples.chunks(rate as usize / 10 + 1) {
            if self.inner.queue.send(Msg::Audio(rate, chunk.to_vec())).is_err() {
                break;
            }
        }
        n
    }

    /// La cartella delle registrazioni.
    pub fn recordings_folder(&self) -> std::path::PathBuf {
        self.inner.rec_dir.clone()
    }

    /// Da dove leggere la frequenza della radio: quando cambia (banda o sintonia) i notch si tolgono da soli.
    pub fn set_dial_source(&self, source: DialSource) {
        *lock(&self.inner.dial) = Some(source);
    }

    /// I notch della ricezione.
    pub fn notches(&self) -> Vec<Notch> {
        lock(&self.inner.notches).clone()
    }

    /// Sostituisce i notch (frequenze 50-5950 Hz, larghezze 10-100 Hz). Non si salvano.
    pub fn set_notches(&self, notches: Vec<Notch>) -> Result<(), DigiError> {
        if notches.len() > crate::notch::NOTCH_MAX || !notches.iter().all(Notch::is_valid) {
            return Err(DigiError::BadValue("notch non validi (frequenza 50-5950 Hz, larghezza 10-100 Hz)".into()));
        }
        self.inner.set_notches(notches);
        Ok(())
    }

    /// Le finestre RX in piu'.
    pub fn extra_rx(&self) -> Vec<ExtraRx> {
        lock(&self.inner.extra).clone()
    }

    /// Sostituisce le finestre RX in piu' (al massimo `EXTRA_RX_MAX`, frequenze 100-5900 Hz, numeri diversi e non zero). Non si salvano.
    pub fn set_extra_rx(&self, list: Vec<ExtraRx>) -> Result<(), DigiError> {
        let mut ids: Vec<u32> = list.iter().map(|x| x.id).collect();
        ids.sort_unstable();
        ids.dedup();
        if list.len() > EXTRA_RX_MAX || ids.len() != list.len() || !list.iter().all(ExtraRx::is_valid) {
            return Err(DigiError::BadValue(format!("finestre RX non valide (al massimo {EXTRA_RX_MAX}, frequenza 100-5900 Hz, numeri diversi)")));
        }
        *lock(&self.inner.extra) = list.clone();
        let _ = self.inner.queue.send(Msg::Extra(list));
        self.inner.emit(DigiEvent { kind: "extra_rx".into(), ..DigiEvent::default() });
        Ok(())
    }

    /// Toglie tutti i notch.
    pub fn clear_notches(&self) {
        self.inner.set_notches(Vec::new());
    }

    pub fn is_listening(&self) -> bool {
        self.inner.listening.load(Ordering::Relaxed)
    }

    pub fn snapshot(&self) -> Snapshot {
        // un lucchetto alla volta: i temporanei di un'espressione restano chiusi fino alla sua fine
        let settings = self.settings();
        let keyer = self.winkeyer_status();
        let can_transmit = lock(&self.inner.port).is_some() || keyer::selected(&settings);
        Snapshot { rx: lock(&self.inner.rx).clone(), tx: lock(&self.inner.tx).clone(), settings, can_transmit, modes: DigiMode::ALL.iter().map(|m| ModeInfo { id: m.id(), name: m.name() }).collect(), keyer }
    }

    pub fn settings(&self) -> DigiSettings {
        lock(&self.inner.settings).clone()
    }

    pub fn apply_settings(&self, s: DigiSettings) -> Result<(), DigiError> {
        s.validate()?;
        if let Some(dir) = self.inner.settings_path.parent() {
            save_macro_files(dir, &s).map_err(|e| DigiError::Io(format!("file delle macro: {e}")))?;
        }
        let json = settings_json(&s).map_err(|e| DigiError::Io(e.to_string()))?;
        std::fs::write(&self.inner.settings_path, json).map_err(|e| DigiError::Io(e.to_string()))?;
        let _ = self.inner.queue.send(Msg::Config(s.clone()));
        self.inner.band.configure(&s);
        self.sync_keyer(&s);
        *lock(&self.inner.settings) = s;
        self.inner.sync_anticlick();
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
        self.inner.sync_anticlick();
        lock(&self.inner.rx).listening = true;
        self.inner.rx_status_event();
    }

    pub fn stop_rx(&self) {
        self.inner.stop_rx();
    }

    /// Un battito della pagina dei modi testuali `id`: finche' qualcuna batte, l'ascolto non si spegne da solo.
    pub fn hold_rx(&self, id: &str) {
        self.inner.users.hold(id);
    }

    /// La pagina `id` e' stata lasciata: se era l'ultima l'ascolto si spegne, salvo «Ascolto acceso all'avvio» (la trasmissione no).
    pub fn release_rx(&self, id: &str) {
        if self.inner.users.release(id) {
            self.inner.page_left();
        }
    }

    /// Il decodificatore di tutta la banda sta lavorando (ascolto acceso e CW o PSK con «tutta la banda»): solo allora servono le forme
    /// dei nominativi.
    pub fn band_in_use(&self) -> bool {
        self.inner.listening.load(Ordering::Relaxed) && self.inner.band.is_active()
    }

    /// Il testo da trasmettere diventa audio con il modo e la frequenza scelti; `lsb`: la radio e' in LSB (RTTY: mark e space invertiti).
    /// Olivia e Contestia con toni e banda fuori dalla banda audio danno un'onda vuota.
    pub fn wave_of(text: &str, s: &DigiSettings, lsb: bool) -> Vec<f32> {
        if s.olivia_settings().is_some() || matches!(s.mode, DigiMode::Throb | DigiMode::Amtor) {
            return live::modulator(s, lsb).map(|mut m| crate::tx_stream::whole(&mut m, text)).unwrap_or_default();
        }
        let bytes: Vec<u8> = text.chars().map(|c| if (c as u32) < 256 { c as u32 as u8 } else { b'?' }).collect();
        match (s.mode.psk(), s.mode) {
            // QPSK31: in LSB le fasi girano al contrario
            (Some(mode), _) if mode.is_qpsk() => psk::modulate_qpsk(&bytes, s.tx_freq(), mode, 0.9, lsb),
            (Some(mode), _) => psk::modulate(&bytes, s.tx_freq(), mode, 0.9),
            (None, DigiMode::Cw) => cw::modulate(&bytes, s.tx_freq(), s.cw_wpm, 0.9),
            (None, _) => rtty::modulate_charset(text, s.rtty(s.tx_freq(), lsb), 0.9, s.rtty_charset),
        }
    }

    /// Trasmette il testo. Il modo e la frequenza sono quelli delle impostazioni. Con una trasmissione a flusso in corso (anche aperta)
    /// il testo si accoda a quello che sta partendo.
    pub fn transmit(&self, text: &str) -> Result<TxStatus, DigiError> {
        if text.trim().is_empty() {
            return Err(DigiError::Empty);
        }
        self.text_mode_or_err()?;
        if self.keyer_selected() {
            return self.keyer_session(text, false);
        }
        if self.tcicw_selected() {
            return self.tcicw_session(text, false);
        }
        let port = lock(&self.inner.port).clone().ok_or(DigiError::NoTransmitter)?;
        if port.can_stream() {
            return self.start_session(port, text, false);
        }
        if self.inner.tx_active.load(Ordering::SeqCst) || port.is_busy() {
            return Err(DigiError::Busy);
        }
        let settings = self.settings();
        let lsb = self.inner.read_radio_lsb();
        let wave = Self::wave_of(text, &settings, lsb);
        if wave.is_empty() {
            return Err(live::modulator(&settings, lsb).err().unwrap_or(DigiError::Empty));
        }
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
        *lock(&self.inner.tx) = TxStatus { state: "sending".into(), progress: 0.0, elapsed_s: 0.0, total_s: total, message: None, open: false };
        self.inner.tx_event();
        // l'eco nella finestra di ricezione: senza flusso tutto subito
        self.inner.emit(DigiEvent { kind: "tx_sent".into(), text: Some(text.to_string()), mode: Some(settings.mode.id().into()), ..DigiEvent::default() });
        let inner = Arc::clone(&self.inner);
        let spawned = std::thread::Builder::new().name("aurora-digi-tx".into()).spawn(move || tx_monitor(&inner, &port, total));
        if let Err(e) = spawned {
            tracing::error!("modi testuali: controllo della trasmissione non avviato: {e}");
        }
        Ok(lock(&self.inner.tx).clone())
    }

    /// Trasmette un audio gia' pronto (12 kHz, mono): il file `.wav` di `<WAVE:nome>`, i toni di `<DTMF:...>`, la portante di `<TUNE>`.
    /// Non si accoda: con una trasmissione in corso risponde `Busy`.
    pub fn transmit_wave(&self, wave: Vec<f32>, what: &str) -> Result<TxStatus, DigiError> {
        if wave.is_empty() {
            return Err(DigiError::Empty);
        }
        let port = lock(&self.inner.port).clone().ok_or(DigiError::NoTransmitter)?;
        if self.inner.tx_active.load(Ordering::SeqCst) || port.is_busy() {
            return Err(DigiError::Busy);
        }
        let total = wave.len() as f32 / RATE;
        self.inner.tx_active.store(true, Ordering::SeqCst);
        self.inner.tx_abort.store(false, Ordering::SeqCst);
        let _ = self.inner.queue.send(Msg::Reset);
        if let Err(e) = port.start(wave) {
            self.inner.tx_active.store(false, Ordering::SeqCst);
            return Err(DigiError::Tx(e));
        }
        tracing::info!("macro: trasmissione audio ({what}) di {total:.1} s");
        *lock(&self.inner.tx) = TxStatus { state: "sending".into(), progress: 0.0, elapsed_s: 0.0, total_s: total, message: None, open: false };
        self.inner.tx_event();
        let inner = Arc::clone(&self.inner);
        let spawned = std::thread::Builder::new().name("aurora-digi-tx".into()).spawn(move || tx_monitor(&inner, &port, total));
        if let Err(e) = spawned {
            tracing::error!("modi testuali: controllo della trasmissione non avviato: {e}");
        }
        Ok(lock(&self.inner.tx).clone())
    }

    /// `NoTextMode` se il modo scelto non trasmette testo (SSB).
    pub(crate) fn text_mode_or_err(&self) -> Result<(), DigiError> {
        if self.settings().mode.is_text() { Ok(()) } else { Err(DigiError::NoTextMode) }
    }

    /// La frequenza audio di trasmissione adesso (per la portante di `<TUNE>`).
    pub fn tx_audio_hz(&self) -> f32 {
        self.settings().tx_freq()
    }

    /// Ferma la trasmissione in corso (audio e PTT).
    pub fn abort_tx(&self) {
        if !self.inner.tx_active.load(Ordering::SeqCst) {
            return;
        }
        self.inner.tx_abort.store(true, Ordering::SeqCst);
        self.keyer_clear();
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

/// Segnale presente con isteresi: si accende sopra la soglia alta, si spegne sotto quella bassa.
fn carrier_present((value, (on, off)): (Option<f32>, (f32, f32)), was: bool) -> bool {
    match value {
        Some(v) if v.is_finite() => v >= if was { off } else { on },
        _ => false,
    }
}

enum Demod {
    Psk(Box<PskDemodulator>),
    Rtty(Box<RttyDemodulator>),
    Cw(Box<CwDemodulator>),
    /// Il ricevitore veloce e il testo provvisorio cambiato nell'ultimo blocco, da mandare alla finestra.
    CwFast(Box<CwHybrid>, Option<String>),
    /// Olivia e Contestia (ricevitore a 8 kHz dietro il decimatore).
    Olivia(Box<crate::olivia_link::OliviaRx>),
    /// Throb e ThrobX (ricevitore a 8 kHz dietro il decimatore).
    Throb(Box<crate::throb::ThrobLink>),
    /// Amtor FEC (12 kHz, nessun ricampionamento).
    Amtor(Box<crate::amtor::AmtorLink>),
    /// SSB: nessuna decodifica, solo la frequenza scelta.
    Off(f32),
}

impl Demod {
    fn new(s: &DigiSettings, lsb: bool) -> Demod {
        if !s.mode.is_text() {
            return Demod::Off(s.rx_freq_hz);
        }
        if let Some((variant, o)) = s.olivia_settings() {
            // squelch spento: la soglia di base, che toglie i caratteri senza senso del solo rumore (come la soglia di base del PSK)
            let squelch = if s.squelch_on { o.squelch_snr_db } else { OliviaSettings::SQUELCH_DEFAULT_DB };
            return match crate::olivia_link::OliviaRx::new(variant, o, s.rx_freq_hz, squelch) {
                Some(d) => Demod::Olivia(Box::new(d)),
                None => {
                    tracing::warn!("{}: {} toni in {} Hz centrati a {:.0} Hz escono dalla banda audio, nessuna ricezione", s.mode.name(), o.tones, o.bandwidth_hz, s.rx_freq_hz);
                    Demod::Off(s.rx_freq_hz)
                }
            };
        }
        if s.mode == DigiMode::Throb {
            // in LSB i toni arrivano rovesciati; lo squelch e' sempre acceso (nel rumore il ricevitore sceglierebbe comunque due toni)
            return Demod::Throb(Box::new(crate::throb::ThrobLink::new(s.throb, s.rx_freq_hz, lsb, s.afc_on)));
        }
        if s.mode == DigiMode::Amtor {
            // in LSB mark e space si scambiano; esce testo solo agganciati ai caratteri (nel rumore niente)
            return Demod::Amtor(Box::new(crate::amtor::AmtorLink::new(s.rx_freq_hz, lsb, s.afc_on)));
        }
        if s.mode == DigiMode::Cw {
            let mut d = CwDemodulator::new(s.rx_freq_hz, s.cw_wpm);
            d.set_bandwidth(s.cw_bandwidth_hz);
            d.set_afc(s.afc_on);
            d.set_som(s.cw_som);
            // lo squelch del CW guarda il rapporto segnale/rumore misurato sullo spettro, non il livello
            d.set_squelch_snr_db(s.squelch_on.then_some(s.cw_squelch_snr_db));
            if s.cw_receiver == CwReceiver::Fast {
                return Demod::CwFast(Box::new(CwHybrid::new(d, s.cw_wpm)), None);
            }
            return Demod::Cw(Box::new(d));
        }
        match s.mode.psk() {
            Some(mode) => {
                let mut d = PskDemodulator::new(mode, s.rx_freq_hz);
                // QPSK31: in LSB le fasi girano al contrario (negli altri modi non cambia niente)
                d.set_reverse(lsb);
                d.set_afc(s.afc_on);
                // con l'AFC acceso la ricezione si aggancia da sola alla portante entro il limite dell'AFC (oltre un quarto di baud l'AFC
                // della fase non la ritrova, prova `psk_finestra_e_lista`)
                d.set_acquire(Some(s.afc_limit()));
                // lo squelch del PSK guarda la metrica di qualita' della fase di fldigi (0-100), non il livello; spento vale la soglia di
                // base come la lista «Segnali» (senza, nelle pause e nel rumore si scrive spazzatura); 0 lo apre del tutto
                d.set_squelch(Some(if s.squelch_on { s.psk_squelch } else { psk::SQUELCH_DEFAULT }));
                Demod::Psk(Box::new(d))
            }
            None => {
                let mut d = RttyDemodulator::new(s.rtty(s.rx_freq_hz, lsb));
                d.set_afc(s.afc_on);
                d.set_uos(s.rtty_uos);
                d.set_charset(s.rtty_charset);
                d.set_discriminator(s.rtty_receiver.discriminator(s.rtty_shift_hz));
                // lo squelch dell'RTTY guarda la probabilita' dei caratteri (passo 9 di `docs/progetto-rtty-oltre.md`); la soglia sul
                // rapporto segnale/rumore (come la «metric» di fldigi) si aggiunge solo se e' sopra 0 dB
                d.set_squelch_snr_db(s.squelch_on.then_some(s.rtty_squelch_snr_db).filter(|&db| db > 0.0));
                d.set_squelch_prob(s.squelch_on.then_some((rtty::SQUELCH_OPEN, rtty::SQUELCH_CLOSE)));
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
            Demod::CwFast(h, _) => h.fine().frequency(),
            Demod::Olivia(d) => d.frequency(),
            Demod::Throb(d) => d.frequency(),
            Demod::Amtor(d) => d.frequency(),
            Demod::Off(hz) => *hz,
        }
    }

    /// Rapporto segnale/rumore (dB) nella misura di ogni modo.
    fn snr_db(&self) -> Option<f32> {
        match self {
            Demod::Rtty(d) => Some(d.snr_db()),
            Demod::Psk(d) => d.snr_db(),
            Demod::Cw(d) => Some(d.snr_db()),
            Demod::CwFast(h, _) => Some(h.fine().snr_db()),
            Demod::Olivia(d) => Some(d.snr_db()),
            Demod::Throb(d) => Some(d.snr_db()),
            Demod::Amtor(d) => Some(d.snr_db()),
            Demod::Off(_) => None,
        }
    }

    /// Le misure in piu' di ogni modo: IMD e qualita' (PSK), velocita' (CW).
    fn extras(&self) -> (Option<f32>, Option<f32>, Option<f32>) {
        match self {
            Demod::Psk(d) => (d.imd_db(), Some(d.metric()), None),
            Demod::Cw(d) => (None, None, Some(d.wpm())),
            Demod::CwFast(h, _) => (None, None, Some(h.fine().wpm())),
            Demod::Rtty(_) | Demod::Olivia(_) | Demod::Throb(_) | Demod::Amtor(_) | Demod::Off(_) => (None, None, None),
        }
    }

    /// PSK: la portante a cui il ricevitore si e' agganciato da solo (una volta), da dire ai limiti dell'AFC.
    fn take_acquired(&mut self) -> Option<f32> {
        match self {
            Demod::Psk(d) => d.take_acquired(),
            _ => None,
        }
    }

    /// Porta l'ascolto su `hz` (limiti dell'AFC) senza cambiare la frequenza scelta.
    fn steer(&mut self, hz: f32) {
        match self {
            Demod::Psk(d) => d.steer(hz),
            Demod::Rtty(d) => d.steer(hz),
            Demod::Cw(d) => d.steer(hz),
            Demod::CwFast(h, _) => h.steer(hz),
            // il sincronismo cerca da solo il segnale entro il suo margine
            Demod::Olivia(_) => {}
            Demod::Throb(d) => d.steer(hz),
            Demod::Amtor(d) => d.steer(hz),
            Demod::Off(f) => *f = hz,
        }
    }

    /// La misura del segnale per i limiti dell'AFC, con le soglie d'isteresi (accendi, spegni): PSK la qualita' della fase (0-100), RTTY il
    /// rapporto segnale/rumore dei toni, CW quello in 3 kHz.
    fn presence(&self) -> (Option<f32>, (f32, f32)) {
        match self {
            Demod::Psk(d) => (Some(d.metric()), (25.0, 12.0)),
            Demod::Rtty(d) => (Some(d.snr_db()), (11.0, 6.0)),
            Demod::Cw(d) => (Some(d.snr_db()), (cw::SQUELCH_DEFAULT_DB, cw::SQUELCH_DEFAULT_DB - 5.0)),
            Demod::CwFast(h, _) => (Some(h.fine().snr_db()), (cw::SQUELCH_DEFAULT_DB, cw::SQUELCH_DEFAULT_DB - 5.0)),
            Demod::Olivia(d) => (Some(d.snr_db()), (d.threshold_db(), d.threshold_db() - 3.0)),
            Demod::Throb(d) => (Some(d.snr_db()), (crate::throb::ThrobRx::threshold_db(), crate::throb::ThrobRx::threshold_db() - 3.0)),
            // Amtor: la quota di coppie buone in dB (0 = tutte buone)
            Demod::Amtor(d) => (Some(d.snr_db()), (-3.0, -9.0)),
            Demod::Off(_) => (None, (0.0, 0.0)),
        }
    }

    /// Livello (dB) e squelch aperto.
    fn signal(&self) -> (f32, bool) {
        match self {
            Demod::Psk(d) => d.signal(),
            Demod::Rtty(d) => d.signal(),
            Demod::Cw(d) => d.signal(),
            Demod::CwFast(h, _) => h.fine().signal(),
            Demod::Olivia(d) => d.signal(),
            Demod::Throb(d) => d.signal(),
            Demod::Amtor(d) => d.signal(),
            Demod::Off(_) => (-90.0, false),
        }
    }

    /// Il testo provvisorio intero (CW veloce), se e' cambiato dall'ultima volta: sostituisce quello mostrato.
    fn take_provisional(&mut self) -> Option<String> {
        match self {
            Demod::CwFast(_, prov) => prov.take(),
            _ => None,
        }
    }

    /// I caratteri nuovi e le posizioni di quelli incerti (RTTY: probabilita' sotto `rtty::WEAK_PROB`).
    fn process(&mut self, audio: &[f32]) -> (String, Vec<u32>) {
        let text = |b: Vec<u8>| b.into_iter().map(char::from).collect::<String>();
        match self {
            Demod::Psk(d) => (text(d.process(audio)), Vec::new()),
            Demod::Rtty(d) => {
                let chars = d.process_chars(audio);
                let weak = chars.iter().enumerate().filter(|(_, c)| c.prob < rtty::WEAK_PROB).map(|(i, _)| i as u32).collect();
                (chars.iter().map(|c| c.ch).collect(), weak)
            }
            Demod::Cw(d) => (text(d.process(audio)), Vec::new()),
            Demod::CwFast(h, prov) => {
                let out = h.process(audio);
                if out.provisional.is_some() {
                    *prov = out.provisional;
                }
                (out.fixed.iter().map(|s| char::from(s.ch)).collect(), Vec::new())
            }
            Demod::Olivia(d) => (d.process(audio), Vec::new()),
            Demod::Throb(d) => (d.process(audio), Vec::new()),
            Demod::Amtor(d) => (d.process(audio), Vec::new()),
            Demod::Off(_) => (String::new(), Vec::new()),
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

/// I limiti dell'AFC per le impostazioni e i notch dati.
fn afc_guard(s: &DigiSettings, notches: &[Notch]) -> AfcGuard {
    let mut g = AfcGuard::new(s.rx_freq_hz, s.afc_limit());
    g.set_filters(notches, s.passband());
    g
}

fn rx_loop(inner: &Arc<Inner>, queue: Receiver<Msg>, settings: DigiSettings) {
    let mut lsb = inner.read_radio_lsb();
    let mut last_radio = Instant::now();
    inner.band.set_lsb(lsb, &settings);
    let mut demod = Demod::new(&settings, lsb);
    let mut mode = settings.mode;
    // le finestre RX in piu': ognuna un ricevitore sulla sua frequenza, con le impostazioni del principale
    let mut extras: Vec<(ExtraRx, Demod, AfcGuard)> = Vec::new();
    let extra_demod = |s: &DigiSettings, x: &ExtraRx, lsb: bool| {
        let mut s = s.clone();
        s.rx_freq_hz = x.freq_hz;
        (Demod::new(&s, lsb), afc_guard(&s, &[]))
    };
    // notch e passabanda davanti ai ricevitori; senza nessuno dei due l'audio passa com'e'
    let mut cur = settings;
    let mut notches: Vec<Notch> = Vec::new();
    let mut filters = RxFilters::new();
    let mut guard = afc_guard(&cur, &notches);
    let mut dial = inner.read_dial();
    let mut last_dial = Instant::now();
    let mut resampler: Option<Resampler> = None;
    let mut audio = Vec::new();
    let mut last_status = Instant::now();
    let mut peak_db = -90.0_f32;
    let mut last_users = Instant::now();
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
                let (band_audio, rx_audio) = filters.process(&audio);
                inner.band.feed(band_audio);
                let (text, weak) = demod.process(rx_audio);
                if let Some(hz) = demod.take_acquired() {
                    guard.retune(hz);
                }
                if cur.afc_on {
                    let (level, th) = demod.presence();
                    if let Some(hz) = guard.step(demod.frequency(), level, th, demod.signal().1) {
                        demod.steer(hz);
                    }
                }
                if !text.is_empty() {
                    let weak = (!weak.is_empty()).then_some(weak);
                    inner.emit(DigiEvent { kind: "rx_text".into(), text: Some(text), weak, mode: Some(mode.id().into()), ..DigiEvent::default() });
                }
                // il provvisorio dopo il definitivo: la finestra toglie quello vecchio e mostra il nuovo in coda
                if let Some(prov) = demod.take_provisional() {
                    inner.emit(DigiEvent { kind: "rx_prov".into(), text: Some(prov), mode: Some(mode.id().into()), ..DigiEvent::default() });
                }
                // le finestre in piu' ascoltano la banda intera (i notch e il passabanda sono del ricevitore principale)
                for (x, d, g) in extras.iter_mut() {
                    let (text, weak) = d.process(band_audio);
                    // l'AFC di ogni finestra e' suo: si muove solo se acceso su quella finestra
                    if let Some(hz) = d.take_acquired() {
                        g.retune(hz);
                    }
                    if x.afc {
                        let (level, th) = d.presence();
                        if let Some(hz) = g.step(d.frequency(), level, th, d.signal().1) {
                            d.steer(hz);
                        }
                    }
                    if !text.is_empty() {
                        let weak = (!weak.is_empty()).then_some(weak);
                        inner.emit(DigiEvent { kind: "rx_text".into(), text: Some(text), weak, mode: Some(mode.id().into()), rx_id: Some(x.id), freq_hz: Some(d.frequency()), ..DigiEvent::default() });
                    }
                    if let Some(prov) = d.take_provisional() {
                        inner.emit(DigiEvent { kind: "rx_prov".into(), text: Some(prov), mode: Some(mode.id().into()), rx_id: Some(x.id), ..DigiEvent::default() });
                    }
                }
            }
            Some(Msg::Extra(list)) => {
                // un ricevitore gia' aperto sulla stessa frequenza resta (non perde quello che sta leggendo)
                let mut old = std::mem::take(&mut extras);
                for x in list {
                    let keep = old.iter().position(|(o, _, _)| o.id == x.id && (o.freq_hz - x.freq_hz).abs() < 1.0);
                    let (d, g) = match keep {
                        Some(i) => {
                            let (_, d, g) = old.swap_remove(i);
                            (d, g)
                        }
                        None => extra_demod(&cur, &x, lsb),
                    };
                    extras.push((x, d, g));
                }
            }
            Some(Msg::Config(s)) => {
                if matches!(demod, Demod::CwFast(..)) {
                    // il provvisorio del ricevitore che se ne va non avra' mai il suo definitivo
                    inner.emit(DigiEvent { kind: "rx_prov".into(), text: Some(String::new()), mode: Some(mode.id().into()), ..DigiEvent::default() });
                }
                demod = Demod::new(&s, lsb);
                mode = s.mode;
                cur = s;
                for (x, d, g) in extras.iter_mut() {
                    (*d, *g) = extra_demod(&cur, x, lsb);
                }
                filters.configure(&notches, cur.passband());
                guard = afc_guard(&cur, &notches);
            }
            Some(Msg::Notches(n)) => {
                notches = n;
                filters.configure(&notches, cur.passband());
                guard.set_filters(&notches, cur.passband());
            }
            Some(Msg::Reset) => {
                cur = lock(&inner.settings).clone();
                demod = Demod::new(&cur, lsb);
                guard = afc_guard(&cur, &notches);
                for (x, d, g) in extras.iter_mut() {
                    (*d, *g) = extra_demod(&cur, x, lsb);
                }
            }
            None => {}
        }
        // le pagine sparite senza salutare (finestra chiusa, rete caduta) escono da sole
        if last_users.elapsed() >= Duration::from_secs(1) {
            last_users = Instant::now();
            if inner.users.expire() {
                inner.page_left();
            }
        }
        // i notch valgono per la frequenza della radio su cui si sono messi: cambiata la banda o la sintonia si tolgono.
        // La frequenza arriva fino a cinque volte al secondo mentre si gira la sintonia: si guarda ogni `DIAL_CHECK`, cosi' gli
        // spostamenti piccoli si sommano e i canali si spostano una volta per giro (`dial` e' l'ultima frequenza gia' applicata)
        if last_dial.elapsed() >= DIAL_CHECK {
            last_dial = Instant::now();
            let now = inner.read_dial();
            if now != 0 && now != dial {
                if dial != 0 && !lock(&inner.notches).is_empty() {
                    tracing::info!("modi testuali: radio da {dial} a {now} Hz, notch tolti");
                    inner.set_notches(Vec::new());
                }
                // tutta la banda (Aurora BIGScan): spostata di poco la radio ogni stazione cambia tono audio e i canali la seguono;
                // cambiata banda (o un salto oltre la banda audio) l'elenco vecchio non vale piu': si ripulisce
                if dial != 0 {
                    match band_follow(dial, now, inner.read_radio_lsb()) {
                        Some(hz) => inner.band.shift(hz),
                        None => {
                            tracing::info!("modi testuali: radio da {dial} a {now} Hz, tutta la banda ripulita");
                            inner.band.reset();
                        }
                    }
                }
                dial = now;
            }
        }
        // RTTY: con la radio passata da USB a LSB (o viceversa) mark e space si scambiano da soli (solo in ascolto: spento, il CAT non si legge)
        if mode == DigiMode::Rtty && inner.listening.load(Ordering::Relaxed) && last_radio.elapsed() >= Duration::from_secs(1) {
            last_radio = Instant::now();
            let now = inner.read_radio_lsb();
            if now != lsb {
                lsb = now;
                tracing::info!("RTTY: radio in {}, mark e space {}", if lsb { "LSB" } else { "USB" }, if lsb { "invertiti" } else { "normali" });
                let now_settings = lock(&inner.settings).clone();
                inner.band.set_lsb(lsb, &now_settings);
                demod = Demod::new(&now_settings, lsb);
                for (x, d, g) in extras.iter_mut() {
                    (*d, *g) = extra_demod(&cur, x, lsb);
                }
            }
        }
        if last_status.elapsed() >= Duration::from_millis(400) {
            last_status = Instant::now();
            {
                let mut rx = lock(&inner.rx);
                rx.level_db = peak_db;
                rx.freq_hz = demod.frequency();
                rx.squelch_open = demod.signal().1;
                rx.carrier = carrier_present(demod.presence(), rx.carrier);
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

    /// La portante per <TXE>: soglia alta per accendersi, bassa per spegnersi, niente misura = niente segnale.
    #[test]
    fn carrier_has_hysteresis() {
        assert!(!carrier_present((Some(10.0), (11.0, 6.0)), false));
        assert!(carrier_present((Some(12.0), (11.0, 6.0)), false));
        assert!(carrier_present((Some(8.0), (11.0, 6.0)), true));
        assert!(!carrier_present((Some(5.0), (11.0, 6.0)), true));
        assert!(!carrier_present((None, (11.0, 6.0)), true));
    }

    /// Tutta la banda e la radio che si sposta: di poco nella stessa banda i toni si spostano (al contrario in USB, uguale in LSB);
    /// un salto oltre la banda audio, un cambio di banda o una frequenza fuori dalle bande ripuliscono l'elenco.
    #[test]
    fn the_band_list_follows_small_dial_moves() {
        assert_eq!(band_follow(14_080_000, 14_080_300, false), Some(-300.0));
        assert_eq!(band_follow(14_080_300, 14_080_000, false), Some(300.0));
        assert_eq!(band_follow(3_580_000, 3_580_200, true), Some(200.0));
        assert_eq!(band_follow(14_080_000, 14_084_000, false), None);
        assert_eq!(band_follow(7_040_000, 14_080_000, false), None);
        assert_eq!(band_follow(13_999_900, 14_000_100, false), None);
        assert_eq!(band_follow(12_000_000, 12_000_100, false), None);
    }

    #[test]
    fn i_modi_si_riconoscono_dal_nome() {
        assert_eq!(DigiMode::from_id("PSK31"), Some(DigiMode::Psk31));
        assert_eq!(DigiMode::from_id("rtty"), Some(DigiMode::Rtty));
        assert_eq!(DigiMode::from_id("cw"), Some(DigiMode::Cw));
        assert_eq!(DigiMode::from_id("olivia"), Some(DigiMode::Olivia));
        assert_eq!(DigiMode::from_id("Contestia"), Some(DigiMode::Contestia));
        assert_eq!(DigiMode::from_id("mfsk16"), None);
        assert_eq!(DigiMode::from_id("SSB"), Some(DigiMode::Ssb));
        assert!(!DigiMode::Ssb.is_text() && DigiMode::Cw.is_text());
        let s = DigiSettings { mode: DigiMode::Ssb, rx_freq_hz: 1200.0, ..DigiSettings::default() };
        let mut d = Demod::new(&s, false);
        assert!(matches!(d, Demod::Off(_)));
        assert_eq!(d.process(&[0.1; 1200]).0, "", "in SSB non si decodifica niente");
        assert!((d.frequency() - 1200.0).abs() < f32::EPSILON);
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
        all.extend(vec![0.0; 12000]);
        let out = d.process(&all).0;
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
            let mut wave = DigiService::wave_of("CQ DE IU3ABC 599", &s, lsb);
            wave.extend(vec![0.0; 6000]);
            let out = d.process(&wave).0;
            assert!(out.contains("CQ DE IU3ABC 599"), "lsb {lsb}, ricevuto: {out:?}");
        }
    }

    #[test]
    fn qpsk31_trasmesso_e_ricevuto_dal_servizio_in_usb_e_in_lsb() {
        let s = DigiSettings { mode: DigiMode::Qpsk31, rx_freq_hz: 1200.0, tx_freq_hz: 1200.0, ..DigiSettings::default() };
        assert_eq!(DigiMode::from_id("qpsk31"), Some(DigiMode::Qpsk31));
        for lsb in [false, true] {
            let mut d = Demod::new(&s, lsb);
            assert!(matches!(d, Demod::Psk(..)), "QPSK31 deve andare al ricevitore PSK, non all'RTTY");
            let mut wave = vec![0.0; 6000];
            wave.extend(DigiService::wave_of("CQ DE IU3ABC 599 K", &s, lsb));
            wave.extend(vec![0.0; 24_000]);
            let mut out = String::new();
            for b in wave.chunks(1200) {
                out.push_str(&d.process(b).0);
            }
            assert!(out.contains("CQ DE IU3ABC 599 K"), "lsb {lsb}, ricevuto: {out:?}");
            // con la banda laterale sbagliata non si legge
            let mut wrong = Demod::new(&s, !lsb);
            let bad = wrong.process(&wave).0;
            assert!(!bad.contains("IU3ABC"), "lsb {lsb}, banda sbagliata: {bad:?}");
        }
    }

    /// Throb (ThrobX2 e Throb4) dal servizio in USB e in LSB; le impostazioni di prima senza il campo valgono Throb2.
    #[test]
    fn throb_trasmesso_e_ricevuto_dal_servizio_in_usb_e_in_lsb() {
        let old: DigiSettings = serde_json::from_str(r#"{"mode":"psk31"}"#).expect("json");
        assert_eq!((old.mode, old.throb), (DigiMode::Psk31, crate::throb::ThrobVariant::Throb2));
        let t: DigiSettings = serde_json::from_str(r#"{"mode":"throb","throb":"throbx2"}"#).expect("json");
        assert_eq!((t.mode, t.throb), (DigiMode::Throb, crate::throb::ThrobVariant::ThrobX2));
        for v in [crate::throb::ThrobVariant::ThrobX2, crate::throb::ThrobVariant::Throb4] {
            let s = DigiSettings { mode: DigiMode::Throb, throb: v, rx_freq_hz: 1200.0, tx_freq_hz: 1200.0, ..DigiSettings::default() };
            for lsb in [false, true] {
                let mut d = Demod::new(&s, lsb);
                assert!(matches!(d, Demod::Throb(..)));
                let mut wave = vec![0.0; 6000];
                wave.extend(DigiService::wave_of("CQ DE IU3ABC K", &s, lsb));
                wave.extend(vec![0.0; 24_000]);
                let out: String = wave.chunks(1200).map(|b| d.process(b).0).collect();
                assert_eq!(out.trim(), "CQ DE IU3ABC K", "{v:?} lsb {lsb}");
                let bad = Demod::new(&s, !lsb).process(&wave).0;
                assert!(!bad.contains("IU3ABC"), "{v:?} lsb {lsb}, banda sbagliata: {bad:?}");
            }
        }
    }

    /// Amtor FEC dal servizio in USB e in LSB.
    #[test]
    fn amtor_trasmesso_e_ricevuto_dal_servizio_in_usb_e_in_lsb() {
        assert_eq!(DigiMode::from_id("amtor"), Some(DigiMode::Amtor));
        let s = DigiSettings { mode: DigiMode::Amtor, rx_freq_hz: 1500.0, tx_freq_hz: 1500.0, ..DigiSettings::default() };
        for lsb in [false, true] {
            let mut d = Demod::new(&s, lsb);
            assert!(matches!(d, Demod::Amtor(..)));
            let mut wave = vec![0.0; 6000];
            wave.extend(DigiService::wave_of("CQ DE IU3ABC 599 K", &s, lsb));
            wave.extend(vec![0.0; 12_000]);
            let out: String = wave.chunks(1200).map(|b| d.process(b).0).collect();
            assert_eq!(out, "CQ DE IU3ABC 599 K", "lsb {lsb}");
            let bad = Demod::new(&s, !lsb).process(&wave).0;
            assert!(!bad.contains("IU3ABC"), "lsb {lsb}, banda sbagliata: {bad:?}");
        }
    }

    #[test]
    fn cw_veloce_mostra_il_provvisorio_e_poi_il_definitivo() {
        // senza il campo (impostazioni salvate prima dell'opzione) vale il veloce, il predefinito
        let old: DigiSettings = serde_json::from_str("{}").unwrap_or_default();
        assert_eq!(old.cw_receiver, CwReceiver::Fast);
        let s = DigiSettings { mode: DigiMode::Cw, rx_freq_hz: 700.0, tx_freq_hz: 700.0, cw_receiver: CwReceiver::Fast, ..DigiSettings::default() };
        let mut d = Demod::new(&s, false);
        assert!(matches!(d, Demod::CwFast(..)));
        let mut wave = DigiService::wave_of("CQ CQ DE IU3ABC IU3ABC K", &s, false);
        wave.extend(vec![0.0; 12_000 * 4]);
        let (mut fixed, mut prov_seen) = (String::new(), false);
        for b in wave.chunks(1200) {
            fixed.push_str(&d.process(b).0);
            prov_seen |= d.take_provisional().is_some_and(|p| !p.trim().is_empty());
        }
        assert!(prov_seen, "nessun testo provvisorio");
        assert!(fixed.contains("IU3ABC"), "definitivo: {fixed:?}");
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
    fn la_sensibilita_di_bigscan_manca_nei_file_vecchi_e_vale_media() {
        let s: DigiSettings = serde_json::from_str(r#"{"mode":"cw"}"#).expect("json");
        assert_eq!(s.band_sensitivity, "medium");
        assert!(s.validate().is_ok());
        let bad = DigiSettings { band_sensitivity: "max".into(), ..DigiSettings::default() };
        assert!(bad.validate().is_err());
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
        assert!(!s.cw_wpm_follow_rx);
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
        assert!(s.rtty_diddle, "il diddle e' acceso di base");
        assert_eq!(s.rtty_receiver, RttyReceiver::Auto);
        assert!(!s.rtty_receiver.discriminator(170.0));
        assert!(s.rtty_receiver.discriminator(23.0));
        let d: DigiSettings = serde_json::from_str(r#"{"rtty_receiver":"discriminator"}"#).expect("json");
        assert_eq!(d.rtty_receiver, RttyReceiver::Discriminator);
        assert!(s.validate().is_ok());
    }

    /// Manda `wave` al servizio (gia' in ascolto) e aspetta fino a 10 s che nel testo ricevuto compaia `want`.
    fn ricevi(svc: &DigiService, wave: &[f32], want: &str) -> String {
        let (tx, rx) = std::sync::mpsc::channel::<String>();
        let tx = Mutex::new(tx);
        svc.set_emitter(Arc::new(move |ev: DigiEvent| {
            if ev.kind == "rx_text"
                && let Some(t) = ev.text
            {
                let _ = lock(&tx).send(t);
            }
        }));
        for chunk in wave.chunks(1200) {
            svc.feed(RATE as u32, chunk);
            std::thread::sleep(Duration::from_millis(2));
        }
        let mut out = String::new();
        let deadline = Instant::now() + Duration::from_secs(10);
        while !out.contains(want) && Instant::now() < deadline {
            if let Ok(t) = rx.recv_timeout(Duration::from_millis(200)) {
                out.push_str(&t);
            }
        }
        out
    }

    /// Al primo avvio le macro di `digi.json` passano nei file di `macro/`: copia di sicurezza, `digi.json` senza macro, `merge_macros`
    /// resta. Una modifica successiva si salva nei file e un nuovo avvio la rilegge da li'.
    #[test]
    fn le_macro_di_digi_json_passano_nei_file_e_si_salvano_li() {
        let dir = std::env::temp_dir().join(format!("aurora-digi-macro-mig-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("cartella di prova");
        let mut s = DigiSettings::default();
        s.macros = vec![mac("F2", "Mio CQ", "<TX>CQ IU3ABC<RX>", 0x112233)];
        s.mode_macros.insert("cw".into(), vec![mac("F2", "CW CQ", "<TX>CQ CW<RX>", 0x445566)]);
        s.merge_macros.insert("cw".into(), false);
        std::fs::write(dir.join("digi.json"), serde_json::to_vec(&s).expect("json")).expect("digi.json");
        let svc = DigiService::open(&dir);
        assert_eq!(svc.settings().macros[0].label, "Mio CQ");
        assert!(dir.join(DIGI_JSON_BACKUP).is_file(), "copia di sicurezza");
        assert!(dir.join("macro").join("comuni.macro").is_file());
        assert!(dir.join("macro").join("cw.macro").is_file());
        let json: serde_json::Value = serde_json::from_slice(&std::fs::read(dir.join("digi.json")).expect("digi.json")).expect("json");
        assert!(json.get("macros").is_none() && json.get("mode_macros").is_none(), "digi.json senza macro");
        assert_eq!(json["merge_macros"]["cw"], serde_json::Value::Bool(false), "l'unione resta un'impostazione");
        let mut next = svc.settings();
        next.mode_macros.get_mut("cw").expect("cw")[0].label = "Nuovo".into();
        svc.apply_settings(next).expect("salva");
        drop(svc);
        let again = DigiService::open(&dir).settings();
        assert_eq!(again.mode_macros["cw"][0].label, "Nuovo");
        assert_eq!(again.mode_macros["cw"][0].color, Some(0x445566));
        assert_eq!(again.merge_macros.get("cw"), Some(&false));
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// Un `digi.json` con la velocita' RTTY a 0 (scritto a mano o rovinato) faceva cadere il thread di ricezione appena acceso: la velocita'
    /// torna quella di base, il resto resta, e l'RTTY si riceve.
    #[test]
    fn un_digi_json_con_valori_impossibili_non_ferma_la_ricezione() {
        let dir = std::env::temp_dir().join(format!("aurora-digi-bad-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("cartella di prova");
        std::fs::write(dir.join("digi.json"), r#"{"mode":"rtty","rtty_baud":0,"rx_freq_hz":1500,"tx_freq_hz":1500,"auto_receive":true}"#).expect("digi.json");
        let svc = DigiService::open(&dir);
        let s = svc.settings();
        assert!(rtty_baud_ok(s.rtty_baud), "velocita' {}", s.rtty_baud);
        assert_eq!(s.mode, DigiMode::Rtty);
        assert!((s.rx_freq_hz - 1500.0).abs() < 0.1, "la frequenza scelta resta");
        svc.start_rx();
        let mut wave = vec![0.0; 6000];
        wave.extend(DigiService::wave_of("RYRYRY CQ IU3ABC IU3ABC K", &s, false));
        wave.extend(vec![0.0; 12000]);
        let out = ricevi(&svc, &wave, "IU3ABC");
        let _ = std::fs::remove_dir_all(&dir);
        assert!(out.contains("IU3ABC"), "ricevuto: {out:?}");
    }

    /// Una finestra RX in piu' su un'altra frequenza legge la sua stazione e manda il testo con il suo numero; la ricezione principale
    /// legge la sua. Valori non validi si rifiutano.
    #[test]
    fn una_finestra_rx_in_piu_legge_la_sua_stazione() {
        let dir = std::env::temp_dir().join(format!("aurora-digi-extra-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("cartella di prova");
        let s = DigiSettings { mode: DigiMode::Cw, cw_wpm: 20.0, rx_freq_hz: 700.0, auto_receive: true, ..DigiSettings::default() };
        std::fs::write(dir.join("digi.json"), serde_json::to_vec(&s).expect("json")).expect("digi.json");
        let svc = DigiService::open(&dir);
        assert!(svc.set_extra_rx(vec![ExtraRx { id: 1, freq_hz: 50.0, afc: false }]).is_err(), "frequenza fuori banda");
        assert!(svc.set_extra_rx(vec![ExtraRx { id: 2, freq_hz: 900.0, afc: false }, ExtraRx { id: 2, freq_hz: 1100.0, afc: false }]).is_err(), "numeri uguali");
        svc.set_extra_rx(vec![ExtraRx { id: 3, freq_hz: 1500.0, afc: false }]).expect("finestra in piu'");
        assert_eq!(svc.extra_rx().len(), 1);
        svc.start_rx();
        let (tx, rx) = std::sync::mpsc::channel::<(Option<u32>, String)>();
        let tx = Mutex::new(tx);
        svc.set_emitter(Arc::new(move |ev: DigiEvent| {
            if ev.kind == "rx_text"
                && let Some(t) = ev.text
            {
                let _ = lock(&tx).send((ev.rx_id, t));
            }
        }));
        let main = crate::cw::modulate(b"CQ IU3ABC IU3ABC K", 700.0, 20.0, 0.3);
        let other = crate::cw::modulate(b"CQ K1XYZ K1XYZ K", 1500.0, 20.0, 0.3);
        let mut wave = vec![0.0_f32; 6000 + main.len().max(other.len()) + 12_000];
        for (w, m) in wave.iter_mut().skip(6000).zip(main) {
            *w += m;
        }
        for (w, o) in wave.iter_mut().skip(6000).zip(other) {
            *w += o;
        }
        for chunk in wave.chunks(1200) {
            svc.feed(RATE as u32, chunk);
            std::thread::sleep(Duration::from_millis(2));
        }
        let (mut main_text, mut extra_text) = (String::new(), String::new());
        let deadline = Instant::now() + Duration::from_secs(10);
        while !(main_text.contains("IU3ABC") && extra_text.contains("K1XYZ")) && Instant::now() < deadline {
            if let Ok((id, t)) = rx.recv_timeout(Duration::from_millis(200)) {
                if id == Some(3) {
                    extra_text.push_str(&t);
                } else {
                    main_text.push_str(&t);
                }
            }
        }
        let _ = std::fs::remove_dir_all(&dir);
        assert!(main_text.contains("IU3ABC") && !main_text.contains("K1XYZ"), "principale: {main_text:?}");
        assert!(extra_text.contains("K1XYZ") && !extra_text.contains("IU3ABC"), "in piu': {extra_text:?}");
    }

    /// Un campione non finito (NaN) nell'audio (TCI, driver) bloccava per sempre i filtri del ricevitore: dopo, niente piu' testo.
    #[test]
    fn un_campione_nan_non_blocca_il_ricevitore() {
        let dir = std::env::temp_dir().join(format!("aurora-digi-nan-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("cartella di prova");
        let s = DigiSettings { mode: DigiMode::Cw, cw_wpm: 20.0, ..DigiSettings::default() };
        std::fs::write(dir.join("digi.json"), serde_json::to_vec(&s).expect("json")).expect("digi.json");
        let svc = DigiService::open(&dir);
        svc.start_rx();
        let mut wave = vec![0.0; 1200];
        wave[100] = f32::NAN;
        wave[200] = f32::INFINITY;
        wave.extend(vec![0.0; 6000]);
        wave.extend(DigiService::wave_of("CQ IU3ABC", &s, false));
        wave.extend(vec![0.0; 12000]);
        let out = ricevi(&svc, &wave, "CQ IU3ABC");
        let _ = std::fs::remove_dir_all(&dir);
        assert!(out.contains("CQ IU3ABC"), "ricevuto: {out:?}");
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

    /// Olivia e Contestia: valori di base, un `digi.json` di prima senza i campi, i valori fuori dai limiti rifiutati e riparati uno per uno.
    #[test]
    fn le_impostazioni_di_olivia_e_contestia() {
        let s: DigiSettings = serde_json::from_str(r#"{"mode":"olivia","rx_freq_hz":1500}"#).expect("json");
        assert_eq!(s.mode, DigiMode::Olivia);
        assert_eq!((s.olivia.tones, s.olivia.bandwidth_hz, s.olivia.sync_margin, s.olivia.sync_integration), (8, 250, 8, 4));
        assert_eq!((s.contestia.tones, s.contestia.bandwidth_hz), (16, 500));
        assert!(s.validate().is_ok());
        let json = serde_json::to_value(&s).expect("json");
        assert_eq!(json["olivia"]["tones"], 8);
        assert_eq!(json["contestia"]["bandwidth_hz"], 500);
        for bad in [
            OliviaSettings { tones: 12, ..OliviaSettings::olivia() },
            OliviaSettings { bandwidth_hz: 125, ..OliviaSettings::olivia() },
            OliviaSettings { sync_margin: 1, ..OliviaSettings::olivia() },
            OliviaSettings { sync_integration: 99, ..OliviaSettings::olivia() },
            OliviaSettings { squelch_snr_db: f32::NAN, ..OliviaSettings::olivia() },
        ] {
            let s = DigiSettings { contestia: bad, cw_wpm: 25.0, ..DigiSettings::default() };
            assert!(s.validate().is_err(), "{bad:?}");
            let r = s.repaired();
            assert_eq!(r.contestia, OliviaSettings::contestia());
            assert_eq!(r.cw_wpm, 25.0, "il resto resta");
        }
        let c = DigiSettings { mode: DigiMode::Contestia, ..DigiSettings::default() };
        assert_eq!(c.olivia_settings().map(|(v, o)| (v, o.tones)), Some((crate::olivia::Variant::Contestia, 16)));
        assert!(DigiSettings::default().olivia_settings().is_none());
        assert!(DigiService::open(&std::env::temp_dir().join("aurora-digi-olivia-modi")).snapshot().modes.iter().any(|m| m.id == "contestia" && m.name == "Contestia"));
        let _ = std::fs::remove_dir_all(std::env::temp_dir().join("aurora-digi-olivia-modi"));
    }

    /// Nel solo rumore (ascolto acceso, Olivia scelto, squelch spento: vale la soglia di base) la finestra RX non riceve niente.
    #[test]
    fn olivia_nel_rumore_non_scrive_niente() {
        let dir = std::env::temp_dir().join(format!("aurora-digi-olivia-rumore-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("cartella di prova");
        let s = DigiSettings { mode: DigiMode::Olivia, rx_freq_hz: 1500.0, ..DigiSettings::default() };
        std::fs::write(dir.join("digi.json"), serde_json::to_vec(&s).expect("json")).expect("digi.json");
        let svc = DigiService::open(&dir);
        let texts = Arc::new(Mutex::new(String::new()));
        let sink = Arc::clone(&texts);
        svc.set_emitter(Arc::new(move |ev: DigiEvent| {
            if ev.kind == "rx_text"
                && let Some(t) = ev.text
            {
                lock(&sink).push_str(&t);
            }
        }));
        svc.start_rx();
        let mut x: u64 = 0x2545_F491_4F6C_DD1D;
        for _ in 0..400 {
            let chunk: Vec<f32> = (0..1200)
                .map(|_| {
                    x ^= x << 13;
                    x ^= x >> 7;
                    x ^= x << 17;
                    ((x >> 40) as f32 / (1u64 << 24) as f32 - 0.5) * 0.6
                })
                .collect();
            svc.inner.queue.send(Msg::Audio(12_000, chunk)).expect("coda");
        }
        // aspetta che la coda si svuoti
        let deadline = Instant::now() + Duration::from_secs(30);
        while svc.inner.queue.try_send(Msg::Notches(Vec::new())).is_err() && Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(50));
        }
        std::thread::sleep(Duration::from_millis(600));
        let snr = svc.snapshot().rx.snr_db;
        let _ = std::fs::remove_dir_all(&dir);
        assert!(lock(&texts).is_empty(), "nel rumore: {:?}", lock(&texts));
        assert!(snr.is_some(), "l'S/N di Olivia arriva allo stato");
    }

    /// Due pagine aperte (finestra principale e finestra staccata): chiudendone una l'ascolto resta, chiudendo l'ultima si spegne; con
    /// «Ascolto acceso all'avvio» resta acceso anche senza pagine.
    #[test]
    fn l_ascolto_si_spegne_solo_quando_esce_l_ultima_pagina() {
        let dir = std::env::temp_dir().join(format!("aurora-digi-pagine-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("cartella di prova");
        let svc = DigiService::open(&dir);
        svc.hold_rx("principale");
        svc.hold_rx("staccata");
        svc.start_rx();
        svc.release_rx("staccata");
        assert!(svc.is_listening(), "la finestra principale e' ancora aperta");
        svc.release_rx("principale");
        assert!(!svc.is_listening(), "nessuna pagina: ascolto spento");
        assert!(!svc.band_in_use());
        let s = DigiSettings { auto_receive: true, ..svc.settings() };
        svc.apply_settings(s).expect("impostazioni");
        svc.hold_rx("principale");
        svc.start_rx();
        svc.release_rx("principale");
        let _ = std::fs::remove_dir_all(&dir);
        assert!(svc.is_listening(), "«Ascolto acceso all'avvio»: resta acceso");
    }
}
