//! TCI (Expert Electronics): comando della radio e flussi audio su WebSocket, per le SDR (SunSDR/ExpertSDR, ANAN con Thetis...).
//!
//! Il protocollo e' quello letto in WSJT-X 3 (`Transceiver/TCITransceiver.cpp`, `TCIStream.hpp`, `TCIProtocol.hpp`):
//! - testo: comandi `nome:arg,arg,...;` (piu' comandi nello stesso messaggio, separati da `;`). Il server all'avvio manda il suo stato
//!   (`protocol:ExpertSDR3,1.9;`, `device:...;`, `vfo:0,0,14074000;`, `modulation:0,usb;`, `trx:0,false;`...) e chiude con `ready;`.
//!   Aurora manda `vfo:<rx>,<vfo>,<hz>;`, `modulation:<rx>,<modo>;`, `trx:<rx>,<true|false>[,tci];`, `audio_start:<rx>;`, `audio_stop:<rx>;`.
//!   Come WSJT-X ogni comando di scrittura (frequenza, modo, PTT, split, potenza, accordo, RX2) aspetta l'eco del server: ExpertSDR e
//!   Thetis rimandano il nuovo stato a tutti i client, e senza eco il comando non si considera fatto.
//! - ricevitore: «TCI Client RX1» di WSJT-X usa il ricevitore 0, «TCI Client RX2» il ricevitore 1 (acceso con `rx_enable:1,true;` se
//!   serve e rispento alla chiusura). Si ascolta sempre sul VFO A (0) del ricevitore scelto; il VFO B (1) serve solo allo split.
//! - binario: flussi con un'intestazione di 64 byte (16 interi a 32 bit little-endian: ricevitore, frequenza di campionamento, formato,
//!   codec, crc, lunghezza in campioni, tipo, canali, 8 riservati) seguita dai campioni. Tipi: 0 IQ, 1 audio ricevuto, 2 audio da
//!   trasmettere, 3 «chrono» (il server chiede il prossimo blocco di audio da trasmettere). Formato 3 = float32, codec 0 = non compresso.
//!   Il campo canali vale dalla versione 1.9 del protocollo; prima i flussi sono sempre stereo.
//!
//! Qui ci sono il protocollo (costruzione e lettura, provate) e la radio [`TciRig`]: frequenza, modo, PTT e i ganci per l'audio
//! (`set_rx_audio`, `set_tx_audio`). Il collegamento dei flussi audio al motore e' descritto in `docs/tasks.md` (voce 16).

use std::{
    collections::VecDeque,
    io::ErrorKind,
    net::{TcpStream, ToSocketAddrs},
    sync::{
        atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering},
        Arc, Condvar, Mutex,
    },
    thread::JoinHandle,
    time::{Duration, Instant},
};

use tungstenite::{Message, WebSocket};

use crate::{
    cat_config::{split_tx_dial_hz, CatMode, RigConfig, SplitMode},
    extras::ExtraItem,
    hamlib_rig::{valid_dial_hz, LinkState, RigStatus},
    meters::{self, Level, Meters},
    Rig, RigError,
};

/// Indirizzo di base del server TCI (ExpertSDR, Thetis).
pub const DEFAULT_TCI_ADDRESS: &str = "127.0.0.1:40001";

// ---------------------------------------------------------------- testo

/// Un comando ricevuto dal server, gia' letto.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TciEvent {
    /// `vfo:<rx>,<vfo>,<hz>;` (vfo 0 = ricezione, 1 = l'altro, usato in split).
    Vfo { rx: u32, vfo: u32, hz: u64 },
    /// `modulation:<rx>,<modo>;` (ExpertSDR3 e Thetis possono mandare anche `modulation:<rx>,0,<modo>;`). Il modo e' in minuscolo.
    Modulation { rx: u32, mode: String },
    /// `trx:<rx>,<true|false>;`: la radio in trasmissione o no.
    Trx { rx: u32, on: bool },
    /// `split_enable:<rx>,<true|false>;`
    SplitEnable { rx: u32, on: bool },
    /// `drive:<rx>,<0..100>;` (ExpertSDR3, Thetis) oppure `drive:<0..100>;` (i server piu' vecchi, senza ricevitore: `rx` = `None`).
    Drive { rx: Option<u32>, percent: u32 },
    /// `tune:<rx>,<true|false>;` (o `tune:<true|false>;` senza ricevitore): la portante di accordo.
    Tune { rx: Option<u32>, on: bool },
    /// `rx_enable:<rx>,<true|false>;`: il ricevitore acceso (RX2 = 1).
    RxEnable { rx: u32, on: bool },
    /// `protocol:<programma>,<versione>;`
    Protocol { name: String, version: String },
    Device(String),
    /// `start;` / `stop;`: la SDR accesa o spenta.
    Start,
    Stop,
    AudioStart { rx: u32 },
    AudioStop { rx: u32 },
    AudioSampleRate(u32),
    /// `ready;`: lo stato iniziale e' stato mandato tutto.
    Ready,
    /// Un comando che Aurora non usa (o con argomenti non validi): nome e argomenti.
    Other { name: String, args: Vec<String> },
}

/// Legge un messaggio di testo del server: uno o piu' comandi separati da `;`. Come WSJT-X: le parti vuote si saltano, il nome e'
/// prima dei `:`, gli argomenti dopo, separati da `,`; un comando con argomenti non validi diventa `Other`.
pub fn parse_text(text: &str) -> Vec<TciEvent> {
    text.split(';')
        .map(str::trim)
        .filter(|c| !c.is_empty())
        .map(|command| {
            let (name, rest) = command.split_once(':').unwrap_or((command, ""));
            let name = name.trim().to_ascii_lowercase();
            let args: Vec<String> = rest.split(',').map(|a| a.trim().to_string()).filter(|a| !a.is_empty()).collect();
            parse_command(&name, &args).unwrap_or(TciEvent::Other { name, args })
        })
        .collect()
}

fn parse_bool(text: &str) -> Option<bool> {
    match text.to_ascii_lowercase().as_str() {
        "true" => Some(true),
        "false" => Some(false),
        _ => None,
    }
}

/// Una potenza in percento (`drive`): anche con i decimali, da 0 a 100.
fn parse_percent(text: &str) -> Option<u32> {
    let v = text.parse::<f32>().ok().filter(|v| v.is_finite() && (0.0..=100.0).contains(v))?;
    Some(v.round() as u32)
}

/// Solo cifre, come `TciProtocol::parse_frequency` (niente segni, spazi o decimali).
pub fn parse_frequency(text: &str) -> Option<u64> {
    if text.is_empty() || !text.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    text.parse().ok()
}

fn parse_command(name: &str, args: &[String]) -> Option<TciEvent> {
    let rx = || args.first().and_then(|a| a.parse::<u32>().ok());
    Some(match name {
        "vfo" => {
            let vfo = args.get(1)?.parse::<u32>().ok().filter(|v| *v <= 1)?;
            TciEvent::Vfo { rx: rx()?, vfo, hz: parse_frequency(args.get(2)?)? }
        }
        "modulation" => {
            let mode = if args.len() >= 3 && args[1] == "0" { &args[2] } else { args.get(1)? };
            TciEvent::Modulation { rx: rx()?, mode: mode.to_ascii_lowercase() }
        }
        "trx" => TciEvent::Trx { rx: rx()?, on: parse_bool(args.get(1)?)? },
        "split_enable" => TciEvent::SplitEnable { rx: rx()?, on: parse_bool(args.get(1)?)? },
        // come `Cmd_Drive` di WSJT-X: con un solo argomento e' la potenza, con due il ricevitore e la potenza
        "drive" if args.len() == 1 => TciEvent::Drive { rx: None, percent: parse_percent(&args[0])? },
        "drive" => TciEvent::Drive { rx: Some(rx()?), percent: parse_percent(args.get(1)?)? },
        "tune" if args.len() == 1 => TciEvent::Tune { rx: None, on: parse_bool(&args[0])? },
        "tune" => TciEvent::Tune { rx: Some(rx()?), on: parse_bool(args.get(1)?)? },
        "rx_enable" => TciEvent::RxEnable { rx: rx()?, on: parse_bool(args.get(1)?)? },
        "protocol" => TciEvent::Protocol { name: args.first()?.clone(), version: args.get(1).cloned().unwrap_or_default() },
        "device" => TciEvent::Device(args.first()?.clone()),
        "start" => TciEvent::Start,
        "stop" => TciEvent::Stop,
        "audio_start" => TciEvent::AudioStart { rx: rx()? },
        "audio_stop" => TciEvent::AudioStop { rx: rx()? },
        "audio_samplerate" => TciEvent::AudioSampleRate(args.first()?.parse().ok()?),
        "ready" => TciEvent::Ready,
        _ => return None,
    })
}

/// `vfo:<rx>,<vfo>,<hz>;`
pub fn cmd_vfo(rx: u32, vfo: u32, hz: u64) -> String {
    format!("vfo:{rx},{vfo},{hz};")
}

/// `modulation:<rx>,<modo>;`
pub fn cmd_modulation(rx: u32, mode: &str) -> String {
    format!("modulation:{rx},{mode};")
}

/// `trx:<rx>,<true|false>;`; con `tci_audio` aggiunge `,tci` (ExpertSDR3: l'audio da trasmettere arriva dal flusso TCI e non dal microfono).
pub fn cmd_trx(rx: u32, on: bool, tci_audio: bool) -> String {
    if tci_audio { format!("trx:{rx},{on},tci;") } else { format!("trx:{rx},{on};") }
}

pub fn cmd_audio_start(rx: u32) -> String {
    format!("audio_start:{rx};")
}

pub fn cmd_audio_stop(rx: u32) -> String {
    format!("audio_stop:{rx};")
}

/// `audio_samplerate:<hz>;` (WSJT-X non lo manda e resta sui 48000 di base del server).
pub fn cmd_audio_samplerate(hz: u32) -> String {
    format!("audio_samplerate:{hz};")
}

/// `split_enable:<rx>,<true|false>;`: la trasmissione sul VFO 1 (B).
pub fn cmd_split(rx: u32, on: bool) -> String {
    format!("split_enable:{rx},{on};")
}

/// `drive:<rx>,<0..100>;`: la potenza d'uscita in percento; senza ricevitore `drive:<0..100>;` (i server che la mandano cosi').
pub fn cmd_drive(rx: Option<u32>, percent: u32) -> String {
    match rx {
        Some(rx) => format!("drive:{rx},{};", percent.min(100)),
        None => format!("drive:{};", percent.min(100)),
    }
}

/// `tune:<rx>,<true|false>;`: portante di accordo (senza ricevitore per i server che la mandano cosi').
pub fn cmd_tune(rx: Option<u32>, on: bool) -> String {
    match rx {
        Some(rx) => format!("tune:{rx},{on};"),
        None => format!("tune:{on};"),
    }
}

/// `rx_enable:<rx>,<true|false>;`: accende o spegne un ricevitore (`rx2_enable` di WSJT-X, ricevitore 1).
pub fn cmd_rx_enable(rx: u32, on: bool) -> String {
    format!("rx_enable:{rx},{on};")
}

/// `start;` / `stop;`: accende o spegne la SDR.
pub fn cmd_power(on: bool) -> String {
    if on { "start;".into() } else { "stop;".into() }
}

/// Chiede al server i misuratori ogni `ms` millisecondi (`rx_sensors_enable` e `tx_sensors_enable` del protocollo 1.9).
pub fn cmd_sensors(ms: u32) -> String {
    let ms = ms.clamp(30, 1000);
    format!("rx_sensors_enable:true,{ms};tx_sensors_enable:true,{ms};")
}

/// Le letture dei misuratori nei comandi del server: `rx_smeter:<rx>,<canale>,<dBm>;`, `rx_sensors:<rx>,<dBm>;`, `tx_power:<W>;`,
/// `tx_swr:<ROS>;`, `tx_sensors:<rx>,<mic dBm>,<W medi>,<W di picco>,<ROS>;`. Ritorna i livelli letti (S-meter in dB rispetto a S9,
/// come Hamlib: S9 = -73 dBm); vuoto per gli altri comandi, per un altro ricevitore o per valori non validi.
pub fn meter_reading(name: &str, args: &[String], rx: u32) -> Vec<(Level, f32)> {
    let num = |i: usize| args.get(i).and_then(|a| a.parse::<f32>().ok()).filter(|v| v.is_finite());
    let same_rx = || args.first().and_then(|a| a.parse::<u32>().ok()) == Some(rx);
    let s9 = |dbm: f32| (Level::Strength, dbm + 73.0);
    let last = args.len().saturating_sub(1);
    match name {
        "rx_smeter" if same_rx() && args.get(1).map(String::as_str) == Some("0") => num(2).map(s9).into_iter().collect(),
        "rx_sensors" if same_rx() => num(1).map(s9).into_iter().collect(),
        "tx_power" => num(last).map(|w| (Level::PowerWatts, w)).into_iter().collect(),
        "tx_swr" => num(last).map(|s| (Level::Swr, s)).into_iter().collect(),
        "tx_sensors" if same_rx() => [num(2).map(|w| (Level::PowerWatts, w)), num(4).map(|s| (Level::Swr, s))].into_iter().flatten().collect(),
        _ => Vec::new(),
    }
}

/// Il modo della radio per il modo CAT scelto: USB oppure USB dati (`digu`); `None` = non si tocca.
pub fn tci_mode(mode: CatMode) -> Option<&'static str> {
    match mode {
        CatMode::None => None,
        CatMode::Usb => Some("usb"),
        CatMode::Data => Some("digu"),
    }
}

// ---------------------------------------------------------------- flussi binari

pub const HEADER_SIZE: usize = 64;
/// Massimo numero di valori float in un blocco (`TciStream::MaxFloatSamples`).
pub const MAX_FLOAT_SAMPLES: u32 = 8192;
pub const FLOAT32_FORMAT: u32 = 3;
pub const UNCOMPRESSED_CODEC: u32 = 0;
pub const DEFAULT_CHANNELS: u32 = 2;
/// La frequenza dell'audio TCI di base (`audioSampleRate` di WSJT-X).
pub const DEFAULT_AUDIO_RATE: u32 = 48_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StreamType {
    Iq,
    RxAudio,
    TxAudio,
    TxChrono,
    Unknown(u32),
}

impl StreamType {
    pub fn from_u32(v: u32) -> StreamType {
        match v {
            0 => StreamType::Iq,
            1 => StreamType::RxAudio,
            2 => StreamType::TxAudio,
            3 => StreamType::TxChrono,
            other => StreamType::Unknown(other),
        }
    }

    pub fn to_u32(self) -> u32 {
        match self {
            StreamType::Iq => 0,
            StreamType::RxAudio => 1,
            StreamType::TxAudio => 2,
            StreamType::TxChrono => 3,
            StreamType::Unknown(v) => v,
        }
    }
}

/// L'intestazione di 64 byte di un flusso (i campi riservati si scrivono a zero).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StreamHeader {
    pub receiver: u32,
    pub sample_rate: u32,
    pub format: u32,
    pub codec: u32,
    pub crc: u32,
    /// Numero di valori float del blocco (tutti i canali insieme).
    pub length: u32,
    pub kind: StreamType,
    pub channels: u32,
}

impl StreamHeader {
    /// Legge l'intestazione; `None` se il blocco e' piu' corto di 64 byte.
    pub fn parse(frame: &[u8]) -> Option<StreamHeader> {
        if frame.len() < HEADER_SIZE {
            return None;
        }
        let word = |i: usize| u32::from_le_bytes([frame[i * 4], frame[i * 4 + 1], frame[i * 4 + 2], frame[i * 4 + 3]]);
        Some(StreamHeader {
            receiver: word(0),
            sample_rate: word(1),
            format: word(2),
            codec: word(3),
            crc: word(4),
            length: word(5),
            kind: StreamType::from_u32(word(6)),
            channels: word(7),
        })
    }

    pub fn to_bytes(&self) -> [u8; HEADER_SIZE] {
        let mut out = [0u8; HEADER_SIZE];
        let words = [self.receiver, self.sample_rate, self.format, self.codec, self.crc, self.length, self.kind.to_u32(), self.channels];
        for (i, w) in words.iter().enumerate() {
            out[i * 4..i * 4 + 4].copy_from_slice(&w.to_le_bytes());
        }
        out
    }

    /// I canali del flusso: 2 se il server non ha il campo (protocollo prima della 1.9); 0 se il valore non e' 1 o 2.
    pub fn channel_count(&self, channel_field_supported: bool) -> u32 {
        if !channel_field_supported {
            return DEFAULT_CHANNELS;
        }
        if self.channels == 1 || self.channels == 2 { self.channels } else { 0 }
    }
}

/// Il campo canali esiste dalla versione 1.9 del protocollo (`TciStream::protocol_has_channels`).
pub fn protocol_has_channels(version: &str) -> bool {
    let mut parts = version.trim().split('.').map(|p| p.parse::<u32>().ok());
    match (parts.next().flatten(), parts.next()) {
        (Some(major), minor) => {
            let minor = minor.flatten().unwrap_or(0);
            (major, minor) >= (1, 9)
        }
        _ => false,
    }
}

/// I campioni float32 di un blocco: `None` se il blocco non contiene tutti i `length` valori dichiarati o se sono troppi.
pub fn float_payload(frame: &[u8], header: &StreamHeader) -> Option<Vec<f32>> {
    if header.length > MAX_FLOAT_SAMPLES || header.format != FLOAT32_FORMAT {
        return None;
    }
    let bytes = frame.get(HEADER_SIZE..HEADER_SIZE + header.length as usize * 4)?;
    Some(bytes.chunks_exact(4).map(|c| f32::from_le_bytes([c[0], c[1], c[2], c[3]])).collect())
}

/// Il «chrono» e' valido se float32 non compresso, alla frequenza attesa, non troppo lungo e a canali interi (`valid_tx_chrono`).
/// Ritorna i canali.
pub fn valid_tx_chrono(chrono: &StreamHeader, channel_field_supported: bool, expected_rate: u32) -> Option<u32> {
    let channels = chrono.channel_count(channel_field_supported);
    let ok = channels != 0
        && chrono.kind == StreamType::TxChrono
        && chrono.format == FLOAT32_FORMAT
        && chrono.codec == UNCOMPRESSED_CODEC
        && chrono.sample_rate == expected_rate
        && chrono.length <= MAX_FLOAT_SAMPLES
        && chrono.length % channels == 0;
    ok.then_some(channels)
}

/// La risposta a un «chrono»: un blocco di audio da trasmettere con la stessa lunghezza, ricevitore e frequenza. `mono` e' l'audio
/// (un campione per istante) copiato in tutti i canali; se e' piu' corto il resto e' silenzio, se e' piu' lungo si taglia.
pub fn tx_audio_frame(chrono: &StreamHeader, channels: u32, mono: &[f32]) -> Option<Vec<u8>> {
    if !(channels == 1 || channels == 2) || chrono.length % channels != 0 || chrono.length > MAX_FLOAT_SAMPLES {
        return None;
    }
    let header = StreamHeader {
        receiver: chrono.receiver,
        sample_rate: chrono.sample_rate,
        format: FLOAT32_FORMAT,
        codec: UNCOMPRESSED_CODEC,
        crc: 0,
        length: chrono.length,
        kind: StreamType::TxAudio,
        channels,
    };
    let frames = (chrono.length / channels) as usize;
    let mut out = Vec::with_capacity(HEADER_SIZE + chrono.length as usize * 4);
    out.extend_from_slice(&header.to_bytes());
    for i in 0..frames {
        let s = mono.get(i).copied().unwrap_or(0.0);
        for _ in 0..channels {
            out.extend_from_slice(&s.to_le_bytes());
        }
    }
    Some(out)
}

/// Da stereo (o mono) a mono: la media dei canali per ogni istante.
pub fn downmix(samples: &[f32], channels: u32) -> Vec<f32> {
    match channels {
        0 => Vec::new(),
        1 => samples.to_vec(),
        n => samples.chunks_exact(n as usize).map(|c| c.iter().sum::<f32>() / n as f32).collect(),
    }
}

// ---------------------------------------------------------------- la radio

/// Riceve l'audio della radio: campioni mono e la loro frequenza di campionamento.
pub type RxAudioFn = Arc<dyn Fn(&[f32], u32) + Send + Sync>;
/// Fornisce l'audio da trasmettere: riempie il buffer mono (alla frequenza indicata) e dice quanti campioni ha scritto.
pub type TxAudioFn = Arc<dyn Fn(&mut [f32], u32) -> usize + Send + Sync>;

/// Attese dell'eco del server, come i `mysleep` di WSJT-X: frequenza 2 s (`mysleep7(2000)`), VFO di trasmissione, modo, PTT e RX2 1 s
/// (`mysleep2`, `mysleep8`, `mysleep3`, `mysleep4`), split 0,5 s (`mysleep5(500)`). Potenza e accordo (che WSJT-X non comanda) 1 s.
const FREQ_ECHO: Duration = Duration::from_millis(2000);
const VFO1_ECHO: Duration = Duration::from_millis(1000);
const MODE_ECHO: Duration = Duration::from_millis(1000);
const PTT_ECHO: Duration = Duration::from_millis(1000);
const SPLIT_ECHO: Duration = Duration::from_millis(500);
const RX2_ECHO: Duration = Duration::from_millis(1000);
const LEVEL_ECHO: Duration = Duration::from_millis(1000);
/// Quante volte si manda un comando senza eco prima di dare errore (WSJT-X lo manda una volta; Aurora lo ripete una volta).
const SEND_ATTEMPTS: u32 = 2;
/// La frequenza confermata puo' scostarsi da quella chiesta (passo della SDR) ma non oltre 1 MHz (`band_change_threshold_hz`).
pub const BAND_CHANGE_HZ: u64 = 1_000_000;
/// Il passo di PWR − / + (come con Hamlib, 5%).
const DRIVE_STEP: i64 = 5;
const NO_DRIVE: u32 = u32::MAX;

/// Messaggi dello stato (tradotti dall'interfaccia, `coreText`).
const RX2_WAIT: &str = "Accensione del ricevitore RX2";
const RX2_FAILED: &str = "La SDR non ha acceso il ricevitore RX2 (rx_enable): controlla che la SDR sia accesa e che abbia RX2, poi ricollega";
const DRIVE_UNKNOWN: &str = "La SDR non ha ancora detto la potenza (drive): cambiala una volta dalla SDR";

struct Inner {
    address: String,
    cat_mode: CatMode,
    split: SplitMode,
    power_on: bool,
    power_off: bool,
    /// Il ricevitore scelto: 0 = RX1, 1 = RX2 (`rx_` di WSJT-X).
    rx: u32,
    stop: AtomicBool,
    /// La frequenza audio di trasmissione (Hz, 0 = non ancora data): serve allo split.
    tx_audio_hz: AtomicU32,
    /// Split simulato: la frequenza di ricezione da rimettere a fine trasmissione (0 = niente).
    emulate_restore: AtomicU64,
    meters: Mutex<Meters>,
    /// `ready;` ricevuto e (con RX2) il secondo ricevitore acceso: i comandi si possono mandare.
    ready: AtomicBool,
    /// `ready;` ricevuto dal server (anche se si aspetta ancora RX2).
    server_ready: AtomicBool,
    channels_field: AtomicBool,
    /// Il flusso audio ricevuto e' acceso sul server (`audio_start` confermato).
    audio_on: AtomicBool,
    status: Mutex<RigStatus>,
    outbox: Mutex<VecDeque<String>>,
    rx_audio: Mutex<Option<RxAudioFn>>,
    tx_audio: Mutex<Option<TxAudioFn>>,
    /// Le attese dell'eco: il thread del collegamento sveglia chi aspetta dopo ogni messaggio di testo.
    echo_lock: Mutex<()>,
    echo: Condvar,
    /// Quante volte il server ha mandato il VFO A (0) e il VFO B (1) del ricevitore: l'eco di un `vfo:` fa avanzare il contatore.
    vfo_seq: [AtomicU64; 2],
    /// Il VFO B (1) del ricevitore, come detto dal server.
    vfo1_hz: AtomicU64,
    split_on: AtomicBool,
    /// Lo split vero acceso da Aurora: si spegne alla chiusura, come `do_stop` di WSJT-X.
    split_by_us: AtomicBool,
    /// La potenza (`drive`) detta dal server, in percento; `NO_DRIVE` = non ancora detta.
    drive: AtomicU32,
    /// Il server scrive `drive:<rx>,<valore>` (e non `drive:<valore>`): Aurora risponde nello stesso modo.
    drive_with_rx: AtomicBool,
    tune_on: AtomicBool,
    tune_with_rx: AtomicBool,
    rx2_on: AtomicBool,
    /// RX2 acceso da Aurora (era spento al collegamento): si rispegne alla chiusura.
    rx2_by_us: AtomicBool,
    /// RX2 chiesto e non ancora confermato: scadenza dell'attesa e tentativi fatti.
    rx2_pending: Mutex<Option<(Instant, u32)>>,
    /// Il modo voluto da Aurora (`requested_mode_`): se il server ne dice un altro lo si rimanda, come WSJT-X.
    wanted_mode: Mutex<Option<&'static str>>,
    /// Un cambio di modo aspetta la sua eco: niente invii automatici nel frattempo (`busy_mode_`).
    mode_busy: AtomicBool,
    /// PTT, split, accordo e RX2 gia' rimessi a posto per questo collegamento (alla chiusura si fa una volta sola).
    released: AtomicBool,
}

impl Inner {
    fn update(&self, edit: impl FnOnce(&mut RigStatus)) {
        if let Ok(mut s) = self.status.lock() {
            edit(&mut s);
            s.updated_ms = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_millis() as u64);
        }
    }

    fn send(&self, text: String) {
        if let Ok(mut o) = self.outbox.lock() {
            o.push_back(text);
        }
    }

    fn connected(&self) -> Result<(), RigError> {
        if self.ready.load(Ordering::SeqCst) {
            Ok(())
        } else {
            Err(RigError("radio TCI non collegata".into()))
        }
    }

    fn dial(&self) -> Option<u64> {
        self.status.lock().ok().and_then(|s| s.frequency_hz)
    }

    fn ptt(&self) -> bool {
        self.status.lock().is_ok_and(|s| s.ptt)
    }

    fn mode_is(&self, mode: &str) -> bool {
        self.status.lock().is_ok_and(|s| s.mode.as_deref().is_some_and(|m| m.eq_ignore_ascii_case(mode)))
    }

    /// Sveglia chi aspetta un'eco (dopo un messaggio del server o quando il collegamento cade).
    fn notify(&self) {
        let _guard = self.echo_lock.lock();
        self.echo.notify_all();
    }

    /// Aspetta che `done` sia vero (lo rendono vero i messaggi del server) per al piu' `timeout`. `false` se il tempo passa o se il
    /// collegamento cade.
    fn wait_until(&self, timeout: Duration, done: impl Fn(&Inner) -> bool) -> bool {
        let deadline = Instant::now() + timeout;
        let Ok(mut guard) = self.echo_lock.lock() else { return done(self) };
        loop {
            if done(self) {
                return true;
            }
            let now = Instant::now();
            if now >= deadline || !self.ready.load(Ordering::SeqCst) {
                return false;
            }
            guard = match self.echo.wait_timeout(guard, deadline - now) {
                Ok((g, _)) => g,
                Err(_) => return done(self),
            };
        }
    }

    /// Un comando di scrittura confermato come in WSJT-X: si manda e si aspetta l'eco del server (`done`); senza eco si ripete una
    /// volta, poi errore. Mai dal thread del collegamento (e' lui a leggere l'eco).
    fn confirm(&self, command: &str, timeout: Duration, done: impl Fn(&Inner) -> bool) -> Result<(), RigError> {
        for attempt in 1..=SEND_ATTEMPTS {
            self.send(command.to_string());
            if self.wait_until(timeout, &done) {
                return Ok(());
            }
            self.connected()?;
            tracing::warn!("TCI: nessuna eco per «{command}» entro {} ms (tentativo {attempt} di {SEND_ATTEMPTS})", timeout.as_millis());
        }
        let name = command.split([':', ';']).next().unwrap_or(command);
        Err(RigError(format!("La radio TCI non ha confermato il comando {name} (nessuna eco dal server, ripetuto una volta)")))
    }

    /// Sposta un VFO del ricevitore e aspetta l'eco (`do_frequency` / `do_tx_frequency` di WSJT-X): la frequenza detta dal server non
    /// deve scostarsi di oltre 1 MHz da quella chiesta. Ritorna quella confermata.
    fn confirm_vfo(&self, vfo: u32, hz: u64) -> Result<u64, RigError> {
        let i = usize::from(vfo != 0);
        let before = self.vfo_seq[i].load(Ordering::SeqCst);
        let timeout = if vfo == 0 { FREQ_ECHO } else { VFO1_ECHO };
        self.confirm(&cmd_vfo(self.rx, vfo, hz), timeout, |s| s.vfo_seq[i].load(Ordering::SeqCst) != before)?;
        let got = if vfo == 0 { self.dial().unwrap_or(0) } else { self.vfo1_hz.load(Ordering::SeqCst) };
        if got.abs_diff(hz) > BAND_CHANGE_HZ {
            return Err(RigError(format!("La radio TCI ha rifiutato la frequenza {hz} Hz: e' rimasta su {got} Hz")));
        }
        Ok(got)
    }

    fn handle_text(&self, text: &str) {
        for event in parse_text(text) {
            match event {
                TciEvent::Vfo { rx, vfo: 0, hz } if rx == self.rx => {
                    self.update(|s| s.frequency_hz = Some(hz));
                    self.vfo_seq[0].fetch_add(1, Ordering::SeqCst);
                }
                TciEvent::Vfo { rx, vfo: _, hz } if rx == self.rx => {
                    self.vfo1_hz.store(hz, Ordering::SeqCst);
                    self.vfo_seq[1].fetch_add(1, Ordering::SeqCst);
                }
                TciEvent::Modulation { rx, mode } if rx == self.rx => {
                    self.update(|s| s.mode = Some(mode.to_ascii_uppercase()));
                    // come `Cmd_Mode` di WSJT-X: se la SDR cambia modo (cambio di banda, manopola) si rimanda quello voluto
                    let wanted = self.wanted_mode.lock().ok().and_then(|w| *w);
                    if let Some(wanted) = wanted
                        && self.ready.load(Ordering::SeqCst)
                        && !self.mode_busy.load(Ordering::SeqCst)
                        && !mode.eq_ignore_ascii_case(wanted)
                    {
                        self.send(cmd_modulation(self.rx, wanted));
                    }
                }
                TciEvent::Trx { rx, on } if rx == self.rx => {
                    self.update(|s| s.ptt = on);
                    if !on && let Ok(mut m) = self.meters.lock() {
                        // a fine trasmissione potenza e ROS non valgono piu'
                        m.power_w = None;
                        m.swr = None;
                    }
                }
                TciEvent::SplitEnable { rx, on } if rx == self.rx => {
                    self.split_on.store(on, Ordering::SeqCst);
                    self.update(|s| s.split = Some(on));
                }
                TciEvent::Drive { rx, percent } if rx.is_none_or(|r| r == self.rx) => {
                    self.drive.store(percent, Ordering::SeqCst);
                    self.drive_with_rx.store(rx.is_some(), Ordering::SeqCst);
                    // PWR − / + dei display seguono la potenza della SDR (0..1 come `power_set` di Hamlib)
                    self.update(|s| s.extras.power_set = Some(percent as f32 / 100.0));
                }
                TciEvent::Tune { rx, on } if rx.is_none_or(|r| r == self.rx) => {
                    self.tune_on.store(on, Ordering::SeqCst);
                    self.tune_with_rx.store(rx.is_some(), Ordering::SeqCst);
                    self.update(|s| s.extras.tuner = Some(on));
                }
                TciEvent::RxEnable { rx: 1, on } => {
                    self.rx2_on.store(on, Ordering::SeqCst);
                    // RX2 acceso (dopo la richiesta o dalla SDR): con il ricevitore 1 scelto il collegamento si completa
                    if on && self.rx == 1 && self.server_ready.load(Ordering::SeqCst) && !self.ready.load(Ordering::SeqCst) {
                        if let Ok(mut p) = self.rx2_pending.lock() {
                            *p = None;
                        }
                        self.finish_ready();
                    }
                }
                TciEvent::Other { name, args } => {
                    let readings = meter_reading(&name, &args, self.rx);
                    if !readings.is_empty()
                        && let Ok(mut m) = self.meters.lock()
                    {
                        for (level, value) in readings {
                            meters::store(&mut m, level, value);
                        }
                    }
                }
                TciEvent::Protocol { version, .. } => self.channels_field.store(protocol_has_channels(&version), Ordering::SeqCst),
                TciEvent::AudioStart { rx } if rx == self.rx => self.audio_on.store(true, Ordering::SeqCst),
                TciEvent::AudioStop { rx } if rx == self.rx => self.audio_on.store(false, Ordering::SeqCst),
                TciEvent::AudioSampleRate(rate) => tracing::info!("TCI: audio a {rate} Hz"),
                TciEvent::Stop => self.update(|s| s.ptt = false),
                TciEvent::Ready => self.on_ready(),
                _ => {}
            }
        }
        self.notify();
    }

    /// `ready;`: si accende la SDR se chiesto e, con RX2 scelto e spento, si accende RX2 (come `do_start` di WSJT-X); il collegamento
    /// e' completo quando RX2 risponde.
    fn on_ready(&self) {
        self.server_ready.store(true, Ordering::SeqCst);
        // «Accendi»: la SDR si accende appena il server risponde
        if self.power_on {
            self.send(cmd_power(true));
        }
        if self.rx == 1 && !self.rx2_on.load(Ordering::SeqCst) {
            self.send(cmd_rx_enable(1, true));
            self.rx2_by_us.store(true, Ordering::SeqCst);
            if let Ok(mut p) = self.rx2_pending.lock() {
                *p = Some((Instant::now() + RX2_ECHO, 1));
            }
            self.update(|s| {
                s.state = LinkState::Connecting;
                s.message = RX2_WAIT.into();
            });
            return;
        }
        self.finish_ready();
    }

    fn finish_ready(&self) {
        self.ready.store(true, Ordering::SeqCst);
        self.released.store(false, Ordering::SeqCst);
        self.update(|s| {
            s.state = LinkState::Connected;
            s.message.clear();
            s.reconnect_attempt = 0;
        });
        self.send(cmd_sensors(500));
        // split vero gia' chiesto prima del collegamento (o prima di un ricollegamento)
        self.apply_split_now();
        // con un ricevitore dell'audio gia' scelto si chiede il flusso, come `stream_audio(true)` di WSJT-X
        if self.rx_audio.lock().is_ok_and(|a| a.is_some()) {
            self.send(cmd_audio_start(self.rx));
        }
    }

    /// RX2 chiesto e senza risposta: dopo 1 s si chiede di nuovo, poi errore («TCI RX2 could not be enabled» di WSJT-X).
    fn check_rx2(&self) {
        let Ok(mut pending) = self.rx2_pending.lock() else { return };
        let Some((deadline, attempt)) = *pending else { return };
        if Instant::now() < deadline {
            return;
        }
        if attempt < SEND_ATTEMPTS {
            tracing::warn!("TCI: RX2 non ancora acceso, nuova richiesta");
            self.send(cmd_rx_enable(1, true));
            *pending = Some((Instant::now() + RX2_ECHO, attempt + 1));
            return;
        }
        *pending = None;
        drop(pending);
        tracing::warn!("TCI: {RX2_FAILED}");
        self.update(|s| {
            s.state = LinkState::Error;
            s.message = RX2_FAILED.into();
        });
    }

    /// La frequenza del VFO B per lo split vero (stessa regola di Hamlib, `split_tx_dial_hz`), se lo split vero e' scelto e la frequenza
    /// audio di trasmissione e' nota.
    fn split_target(&self) -> Option<u64> {
        if self.split != SplitMode::Rig {
            return None;
        }
        let audio = self.tx_audio_hz.load(Ordering::SeqCst);
        let dial = self.dial()?;
        (audio != 0).then(|| split_tx_dial_hz(dial, audio))
    }

    /// Split vero senza attese (dal thread del collegamento, dopo `ready;`): il VFO B sulla frequenza di trasmissione e lo split acceso
    /// per ultimo.
    fn apply_split_now(&self) {
        let Some(tx) = self.split_target() else { return };
        self.send(cmd_vfo(self.rx, 1, tx));
        self.send(cmd_split(self.rx, true));
        self.split_by_us.store(true, Ordering::SeqCst);
    }

    /// Split vero con le conferme: VFO B (1 s) e poi `split_enable` (0,5 s), ognuno solo se diverso da quanto detto dal server.
    fn apply_split_confirmed(&self) -> Result<(), RigError> {
        let Some(tx) = self.split_target() else { return Ok(()) };
        if self.vfo1_hz.load(Ordering::SeqCst) != tx {
            self.confirm_vfo(1, tx)?;
        }
        if !self.split_on.load(Ordering::SeqCst) {
            self.confirm(&cmd_split(self.rx, true), SPLIT_ECHO, |s| s.split_on.load(Ordering::SeqCst))?;
        }
        self.split_by_us.store(true, Ordering::SeqCst);
        Ok(())
    }

    /// Alla chiusura (una volta per collegamento): portante di accordo e PTT spenti, split tolto e RX2 rispento se li aveva accesi Aurora
    /// (`do_stop` di WSJT-X).
    fn release(&self) {
        if !self.ready.load(Ordering::SeqCst) || self.released.swap(true, Ordering::SeqCst) {
            return;
        }
        if self.tune_on.load(Ordering::SeqCst) {
            self.send(cmd_tune(self.tune_with_rx.load(Ordering::SeqCst).then_some(self.rx), false));
        }
        if self.ptt() {
            self.send(cmd_trx(self.rx, false, false));
        }
        if self.split_by_us.swap(false, Ordering::SeqCst) {
            self.send(cmd_split(self.rx, false));
        }
        if self.rx2_by_us.swap(false, Ordering::SeqCst) {
            self.send(cmd_rx_enable(1, false));
        }
    }

    /// Un blocco binario: audio ricevuto al ricevitore o richiesta di audio da trasmettere (la risposta, se c'e').
    fn handle_binary(&self, frame: &[u8]) -> Option<Vec<u8>> {
        let header = StreamHeader::parse(frame)?;
        if header.receiver != self.rx {
            return None;
        }
        let channels_field = self.channels_field.load(Ordering::SeqCst);
        match header.kind {
            StreamType::RxAudio => {
                let hook = self.rx_audio.lock().ok().and_then(|a| a.clone())?;
                let samples = float_payload(frame, &header)?;
                let channels = header.channel_count(channels_field);
                hook(&downmix(&samples, channels), header.sample_rate);
                None
            }
            StreamType::TxChrono => {
                let hook = self.tx_audio.lock().ok().and_then(|a| a.clone())?;
                let channels = valid_tx_chrono(&header, channels_field, header.sample_rate)?;
                let mut mono = vec![0.0f32; (header.length / channels) as usize];
                let n = hook(&mut mono, header.sample_rate).min(mono.len());
                mono.truncate(n);
                tx_audio_frame(&header, channels, &mono)
            }
            _ => None,
        }
    }
}

/// Una radio comandata via TCI. Il collegamento vive in un thread: si ricollega da solo se il server cade (ExpertSDR chiuso e
/// riaperto) e la radio risulta collegata solo dopo il `ready;` del server (e, con RX2, dopo l'accensione del secondo ricevitore).
pub struct TciRig {
    inner: Arc<Inner>,
    worker: Mutex<Option<JoinHandle<()>>>,
}

impl TciRig {
    pub fn start(address: &str, cat_mode: CatMode) -> TciRig {
        TciRig::from_config(&RigConfig { tci_address: address.to_string(), cat_mode, ..RigConfig::default() })
    }

    /// Dalla configurazione della radio: indirizzo, ricevitore, modo CAT, split, «Accendi» e «Spegni».
    pub fn from_config(config: &RigConfig) -> TciRig {
        let address = config.tci_address.trim();
        let address = if address.is_empty() { DEFAULT_TCI_ADDRESS.to_string() } else { address.to_string() };
        let inner = Arc::new(Inner {
            address,
            cat_mode: config.cat_mode,
            split: config.split,
            power_on: config.power_on,
            power_off: config.power_off,
            rx: u32::from(config.tci_receiver.min(1)),
            stop: AtomicBool::new(false),
            tx_audio_hz: AtomicU32::new(0),
            emulate_restore: AtomicU64::new(0),
            meters: Mutex::new(Meters::default()),
            ready: AtomicBool::new(false),
            server_ready: AtomicBool::new(false),
            channels_field: AtomicBool::new(false),
            audio_on: AtomicBool::new(false),
            status: Mutex::new(RigStatus {
                state: LinkState::Connecting,
                message: String::new(),
                frequency_hz: None,
                mode: None,
                passband_hz: None,
                extras: Default::default(),
                ptt: false,
                split: None,
                failures: 0,
                reconnect_attempt: 0,
                ptt_blocked: false,
                updated_ms: 0,
            }),
            outbox: Mutex::new(VecDeque::new()),
            rx_audio: Mutex::new(None),
            tx_audio: Mutex::new(None),
            echo_lock: Mutex::new(()),
            echo: Condvar::new(),
            vfo_seq: [AtomicU64::new(0), AtomicU64::new(0)],
            vfo1_hz: AtomicU64::new(0),
            split_on: AtomicBool::new(false),
            split_by_us: AtomicBool::new(false),
            drive: AtomicU32::new(NO_DRIVE),
            drive_with_rx: AtomicBool::new(true),
            tune_on: AtomicBool::new(false),
            tune_with_rx: AtomicBool::new(true),
            rx2_on: AtomicBool::new(false),
            rx2_by_us: AtomicBool::new(false),
            rx2_pending: Mutex::new(None),
            wanted_mode: Mutex::new(None),
            mode_busy: AtomicBool::new(false),
            released: AtomicBool::new(false),
        });
        let worker_inner = Arc::clone(&inner);
        let worker = std::thread::Builder::new().name("tci".into()).spawn(move || run(&worker_inner)).ok();
        TciRig { inner, worker: Mutex::new(worker) }
    }

    pub fn address(&self) -> &str {
        &self.inner.address
    }

    /// Il ricevitore comandato: 0 = RX1, 1 = RX2.
    pub fn receiver(&self) -> u32 {
        self.inner.rx
    }

    pub fn status(&self) -> RigStatus {
        self.inner.status.lock().map(|s| s.clone()).unwrap_or_else(|e| e.into_inner().clone())
    }

    pub fn transmitting(&self) -> bool {
        self.status().ptt
    }

    /// Il PTT con `trx`, confermato dall'eco come `do_ptt` di WSJT-X (1 s, poi una ripetizione). Senza ricevitore dell'audio TCI per
    /// la trasmissione la SDR prende l'audio dalla sua sorgente (cavo virtuale).
    pub fn set_ptt(&self, on: bool) -> Result<(), RigError> {
        let inner = &self.inner;
        inner.connected()?;
        // split simulato: in trasmissione il VFO di ricezione va sulla frequenza di trasmissione (confermata), poi torna
        if on && inner.split == SplitMode::Emulate {
            let audio = inner.tx_audio_hz.load(Ordering::SeqCst);
            if let Some(dial) = inner.dial()
                && audio != 0
            {
                let tx = split_tx_dial_hz(dial, audio);
                if tx != dial {
                    inner.confirm_vfo(0, tx)?;
                    inner.emulate_restore.store(dial, Ordering::SeqCst);
                }
            }
        }
        let tci_audio = inner.tx_audio.lock().is_ok_and(|a| a.is_some());
        if inner.ptt() != on
            && let Err(e) = inner.confirm(&cmd_trx(inner.rx, on, tci_audio), PTT_ECHO, |s| s.ptt() == on)
        {
            if on {
                // senza conferma la trasmissione non e' partita: si chiede comunque lo spegnimento e il VFO torna dov'era
                inner.send(cmd_trx(inner.rx, false, tci_audio));
                let back = inner.emulate_restore.swap(0, Ordering::SeqCst);
                if back != 0 {
                    inner.send(cmd_vfo(inner.rx, 0, back));
                }
            }
            return Err(e);
        }
        if !on {
            let back = inner.emulate_restore.swap(0, Ordering::SeqCst);
            if back != 0
                && let Err(e) = inner.confirm_vfo(0, back)
            {
                // il PTT e' spento: l'errore resta nello stato, la trasmissione non si considera fallita
                tracing::warn!("TCI: frequenza di ricezione non rimessa dopo la trasmissione: {e}");
                inner.update(|s| s.message = e.0.clone());
            }
        }
        Ok(())
    }

    /// La frequenza audio con cui si trasmette (Hz): con lo split vero sposta il VFO B (`vfo:<rx>,1,<hz>;` e `split_enable:<rx>,true;`,
    /// confermati), con lo split simulato serve al PTT.
    pub fn set_tx_audio_frequency(&self, hz: u32) -> Result<(), RigError> {
        if hz > 5000 {
            return Err(RigError(format!("frequenza audio di trasmissione non valida: {hz} Hz")));
        }
        self.inner.tx_audio_hz.store(hz, Ordering::SeqCst);
        if self.inner.split == SplitMode::Rig {
            self.inner.connected()?;
            self.inner.apply_split_confirmed()?;
        }
        Ok(())
    }

    /// La potenza d'uscita della SDR in percento (`drive`), confermata dall'eco.
    pub fn set_drive(&self, percent: u32) -> Result<(), RigError> {
        let inner = &self.inner;
        inner.connected()?;
        let percent = percent.min(100);
        let rx = inner.drive_with_rx.load(Ordering::SeqCst).then_some(inner.rx);
        inner.confirm(&cmd_drive(rx, percent), LEVEL_ECHO, |s| s.drive.load(Ordering::SeqCst) == percent)
    }

    /// La potenza detta dalla SDR (percento), se l'ha detta.
    pub fn drive(&self) -> Option<u32> {
        Some(self.inner.drive.load(Ordering::SeqCst)).filter(|d| *d != NO_DRIVE)
    }

    /// Portante di accordo (`tune`), confermata dall'eco.
    pub fn set_tune(&self, on: bool) -> Result<(), RigError> {
        let inner = &self.inner;
        inner.connected()?;
        let rx = inner.tune_with_rx.load(Ordering::SeqCst).then_some(inner.rx);
        inner.confirm(&cmd_tune(rx, on), LEVEL_ECHO, |s| s.tune_on.load(Ordering::SeqCst) == on)
    }

    /// La portante di accordo e' accesa.
    pub fn tuning(&self) -> bool {
        self.inner.tune_on.load(Ordering::SeqCst)
    }

    /// I comandi dei display gia' esistenti: PWR − / + (`drive` a passi del 5%, come con Hamlib) e TUNE (accende o spegne la portante
    /// di accordo; ATU fa lo stesso, il TCI non comanda l'accordatore). Gli altri non esistono nel TCI.
    pub fn set_extra(&self, item: ExtraItem) -> Result<(), RigError> {
        match item {
            ExtraItem::PowerUp | ExtraItem::PowerDown => {
                let now = self.drive().ok_or_else(|| RigError(DRIVE_UNKNOWN.into()))?;
                let step = if item == ExtraItem::PowerUp { DRIVE_STEP } else { -DRIVE_STEP };
                // a passi interi del 5%: 37 sale a 40 e scende a 35
                let next = ((i64::from(now) as f64 / DRIVE_STEP as f64).round() as i64 * DRIVE_STEP + step).clamp(DRIVE_STEP, 100);
                self.set_drive(next as u32)
            }
            ExtraItem::Tune | ExtraItem::Atu => self.set_tune(!self.tuning()),
            _ => Err(RigError("comando non supportato da TCI".into())),
        }
    }

    /// «Spegni»: alla chiusura del programma la SDR si spegne con `stop;` (prima PTT, accordo, split e RX2 rimessi a posto).
    pub fn power_off_if_chosen(&self) {
        if self.inner.power_off && self.inner.ready.load(Ordering::SeqCst) {
            self.inner.release();
            self.inner.send(cmd_power(false));
            tracing::info!("SDR spenta via TCI alla chiusura («Spegni»)");
        }
    }

    /// Il ricevitore dell'audio della radio (mono, alla frequenza del server). Con `Some` si chiede il flusso (`audio_start`), con `None`
    /// lo si ferma.
    pub fn set_rx_audio(&self, hook: Option<RxAudioFn>) {
        let on = hook.is_some();
        if let Ok(mut a) = self.inner.rx_audio.lock() {
            *a = hook;
        }
        if self.inner.ready.load(Ordering::SeqCst) {
            self.inner.send(if on { cmd_audio_start(self.inner.rx) } else { cmd_audio_stop(self.inner.rx) });
        }
    }

    /// Chi fornisce l'audio da trasmettere a ogni «chrono» del server.
    pub fn set_tx_audio(&self, hook: Option<TxAudioFn>) {
        if let Ok(mut a) = self.inner.tx_audio.lock() {
            *a = hook;
        }
    }

    /// Il flusso dell'audio ricevuto e' acceso sul server.
    pub fn audio_streaming(&self) -> bool {
        self.inner.audio_on.load(Ordering::SeqCst)
    }

    /// Chiude: flusso audio fermato, PTT e accordo spenti, split e RX2 rimessi come prima, collegamento chiuso.
    pub fn shutdown(&self) {
        if self.inner.ready.load(Ordering::SeqCst) && self.audio_streaming() {
            self.inner.send(cmd_audio_stop(self.inner.rx));
        }
        self.inner.release();
        self.inner.stop.store(true, Ordering::SeqCst);
        if let Some(handle) = self.worker.lock().ok().and_then(|mut w| w.take()) {
            let _ = handle.join();
        }
        self.inner.ready.store(false, Ordering::SeqCst);
        self.inner.notify();
        self.inner.update(|s| {
            s.state = LinkState::Disconnected;
            s.ptt = false;
        });
    }
}

impl Drop for TciRig {
    fn drop(&mut self) {
        self.shutdown();
    }
}

fn open(address: &str) -> Result<WebSocket<TcpStream>, String> {
    let addr = address.to_socket_addrs().map_err(|e| format!("{address}: {e}"))?.next().ok_or_else(|| format!("{address}: indirizzo non valido"))?;
    let stream = TcpStream::connect_timeout(&addr, Duration::from_secs(3)).map_err(|e| format!("{address}: {e}"))?;
    stream.set_read_timeout(Some(Duration::from_secs(3))).map_err(|e| e.to_string())?;
    stream.set_write_timeout(Some(Duration::from_secs(3))).map_err(|e| e.to_string())?;
    let _ = stream.set_nodelay(true);
    let (ws, _) = tungstenite::client(format!("ws://{address}/"), stream).map_err(|e| format!("{address}: {e}"))?;
    // dopo l'apertura le letture aspettano poco: il thread deve anche mandare i comandi in coda
    ws.get_ref().set_read_timeout(Some(Duration::from_millis(20))).map_err(|e| e.to_string())?;
    Ok(ws)
}

/// Lo stato di un collegamento caduto non vale per il prossimo: il server rimanda tutto dopo il nuovo `ready;`.
fn reset_link(inner: &Inner) {
    inner.ready.store(false, Ordering::SeqCst);
    inner.server_ready.store(false, Ordering::SeqCst);
    inner.audio_on.store(false, Ordering::SeqCst);
    inner.split_on.store(false, Ordering::SeqCst);
    inner.tune_on.store(false, Ordering::SeqCst);
    inner.rx2_on.store(false, Ordering::SeqCst);
    if let Ok(mut p) = inner.rx2_pending.lock() {
        *p = None;
    }
    inner.notify();
}

fn run(inner: &Inner) {
    let mut attempt = 0u32;
    while !inner.stop.load(Ordering::SeqCst) {
        match open(&inner.address) {
            Ok(mut ws) => {
                attempt = 0;
                if let Ok(mut o) = inner.outbox.lock() {
                    o.clear();
                }
                let reason = serve(inner, &mut ws);
                let _ = ws.close(None);
                let _ = ws.flush();
                reset_link(inner);
                if inner.stop.load(Ordering::SeqCst) {
                    break;
                }
                tracing::warn!("TCI: collegamento perso: {reason}");
                inner.update(|s| {
                    s.state = LinkState::Reconnecting;
                    s.message = format!("Collegamento TCI perso ({reason}): nuovo tentativo");
                    s.ptt = false;
                });
            }
            Err(e) => {
                attempt += 1;
                inner.update(|s| {
                    s.state = if attempt == 1 { LinkState::Error } else { LinkState::Reconnecting };
                    s.message = format!("Server TCI non raggiungibile ({e}): avvia ExpertSDR/Thetis e attiva il TCI");
                    s.reconnect_attempt = attempt;
                    s.failures = attempt;
                });
            }
        }
        // attesa prima di riprovare, crescente fino a 5 s, interrotta subito dalla chiusura
        let wait = Duration::from_millis((500 * u64::from(attempt.clamp(1, 10))).min(5000));
        let until = Instant::now() + wait;
        while Instant::now() < until && !inner.stop.load(Ordering::SeqCst) {
            std::thread::sleep(Duration::from_millis(20));
        }
    }
}

/// Il giro di un collegamento aperto: manda i comandi in coda, legge i messaggi. Torna con il motivo quando il collegamento cade
/// o quando si chiude.
fn serve(inner: &Inner, ws: &mut WebSocket<TcpStream>) -> String {
    loop {
        let pending: Vec<String> = inner.outbox.lock().map(|mut o| o.drain(..).collect()).unwrap_or_default();
        for text in pending {
            if let Err(e) = ws.send(Message::text(text)) {
                return e.to_string();
            }
        }
        if inner.stop.load(Ordering::SeqCst) {
            // gli ultimi comandi (PTT spento, `stop;` di «Spegni») messi in coda insieme alla chiusura
            let last: Vec<String> = inner.outbox.lock().map(|mut o| o.drain(..).collect()).unwrap_or_default();
            for text in last {
                let _ = ws.send(Message::text(text));
            }
            let _ = ws.flush();
            return "chiuso".into();
        }
        inner.check_rx2();
        match ws.read() {
            Ok(Message::Text(text)) => inner.handle_text(text.as_str()),
            Ok(Message::Binary(data)) => {
                if let Some(reply) = inner.handle_binary(&data)
                    && let Err(e) = ws.send(Message::binary(reply))
                {
                    return e.to_string();
                }
            }
            Ok(Message::Close(_)) => return "chiuso dal server".into(),
            Ok(_) => {}
            Err(tungstenite::Error::Io(e)) if matches!(e.kind(), ErrorKind::WouldBlock | ErrorKind::TimedOut) => {
                // nessun dato: si svuota quanto resta da scrivere e si continua
                if let Err(tungstenite::Error::Io(e)) = ws.flush()
                    && !matches!(e.kind(), ErrorKind::WouldBlock | ErrorKind::TimedOut)
                {
                    return e.to_string();
                }
            }
            Err(e) => return e.to_string(),
        }
    }
}

impl Rig for TciRig {
    /// Cambio di frequenza del VFO A del ricevitore (non in trasmissione) e, se scelto, il modo (USB o USB dati), ognuno confermato
    /// dall'eco come `do_frequency` di WSJT-X: nessun invio se la radio e' gia' li'.
    fn set_frequency(&self, hz: u64) -> Result<(), RigError> {
        if !valid_dial_hz(hz) {
            return Err(RigError(format!("frequenza non valida: {hz} Hz")));
        }
        let inner = &self.inner;
        inner.connected()?;
        if self.transmitting() {
            return Err(RigError("la radio sta trasmettendo: cambio di frequenza rifiutato".into()));
        }
        if inner.dial() != Some(hz) {
            inner.confirm_vfo(0, hz)?;
        }
        if let Some(mode) = tci_mode(inner.cat_mode) {
            if let Ok(mut w) = inner.wanted_mode.lock() {
                *w = Some(mode);
            }
            if !inner.mode_is(mode) {
                inner.mode_busy.store(true, Ordering::SeqCst);
                let done = inner.confirm(&cmd_modulation(inner.rx, mode), MODE_ECHO, |s| s.mode_is(mode));
                inner.mode_busy.store(false, Ordering::SeqCst);
                done?;
            }
        }
        Ok(())
    }

    fn frequency(&self) -> Result<u64, RigError> {
        let s = self.status();
        match (s.state, s.frequency_hz) {
            (LinkState::Connected, Some(hz)) => Ok(hz),
            _ => Err(RigError("radio TCI non collegata".into())),
        }
    }

    /// I misuratori mandati dal server (`rx_sensors`, `tx_sensors`, `rx_smeter`, `tx_power`, `tx_swr`).
    fn meters(&self) -> Result<Meters, RigError> {
        self.inner.connected()?;
        Ok(self.inner.meters.lock().map(|m| *m).unwrap_or_default())
    }

    fn set_rx_frequency(&self, _hz: u32) -> Result<(), RigError> {
        Err(RigError("comando del motore non supportato da TCI".into()))
    }

    fn output_attenuation(&self) -> Result<u32, RigError> {
        Err(RigError("comando del motore non supportato da TCI".into()))
    }

    fn set_output_attenuation(&self, _tenths: u32) -> Result<(), RigError> {
        Err(RigError("comando del motore non supportato da TCI".into()))
    }

    fn set_mode(&self, _mode: &str) -> Result<(), RigError> {
        Err(RigError("comando del motore non supportato da TCI".into()))
    }

    fn set_extra(&self, item: ExtraItem) -> Result<(), RigError> {
        TciRig::set_extra(self, item)
    }
}

#[cfg(test)]
mod tests {
    use std::net::TcpListener;

    use super::*;

    #[test]
    fn it_reads_the_initial_burst_like_wsjtx() {
        let events = parse_text("protocol:ExpertSDR3,1.9;device:SunSDR2PRO;vfo:0,0,14074000;vfo:0,1,14075000;modulation:0,usb;trx:0,false;ready;");
        assert_eq!(
            events,
            vec![
                TciEvent::Protocol { name: "ExpertSDR3".into(), version: "1.9".into() },
                TciEvent::Device("SunSDR2PRO".into()),
                TciEvent::Vfo { rx: 0, vfo: 0, hz: 14_074_000 },
                TciEvent::Vfo { rx: 0, vfo: 1, hz: 14_075_000 },
                TciEvent::Modulation { rx: 0, mode: "usb".into() },
                TciEvent::Trx { rx: 0, on: false },
                TciEvent::Ready,
            ]
        );
    }

    #[test]
    fn it_reads_the_three_argument_modulation_of_expertsdr3_and_thetis() {
        assert_eq!(parse_text("modulation:0,0,DIGU;"), vec![TciEvent::Modulation { rx: 0, mode: "digu".into() }]);
    }

    #[test]
    fn it_rejects_bad_frequencies_and_vfos() {
        for bad in ["vfo:0,0,14.074;", "vfo:0,0,-1;", "vfo:0,2,14074000;", "vfo:0,0;", "trx:0,forse;"] {
            assert!(matches!(parse_text(bad).as_slice(), [TciEvent::Other { .. }]), "{bad}");
        }
        assert_eq!(parse_frequency("14074000"), Some(14_074_000));
        assert_eq!(parse_frequency(""), None);
        assert_eq!(parse_frequency("+1"), None);
    }

    #[test]
    fn it_skips_empty_parts_and_keeps_unknown_commands() {
        assert_eq!(parse_text(";;rx_smeter:0,0,-87;"), vec![TciEvent::Other { name: "rx_smeter".into(), args: vec!["0".into(), "0".into(), "-87".into()] }]);
        assert_eq!(parse_text("start;stop;audio_start:0;audio_stop:0;audio_samplerate:48000;"), vec![
            TciEvent::Start,
            TciEvent::Stop,
            TciEvent::AudioStart { rx: 0 },
            TciEvent::AudioStop { rx: 0 },
            TciEvent::AudioSampleRate(48_000),
        ]);
    }

    #[test]
    fn it_reads_split_drive_tune_and_rx_enable() {
        assert_eq!(parse_text("split_enable:0,true;rx_enable:1,false;"), vec![
            TciEvent::SplitEnable { rx: 0, on: true },
            TciEvent::RxEnable { rx: 1, on: false },
        ]);
        // `Cmd_Drive` di WSJT-X: un argomento (server vecchi) o ricevitore e potenza (ExpertSDR3, Thetis), anche con i decimali
        assert_eq!(parse_text("drive:40;drive:1,55.4;"), vec![
            TciEvent::Drive { rx: None, percent: 40 },
            TciEvent::Drive { rx: Some(1), percent: 55 },
        ]);
        assert_eq!(parse_text("tune:0,true;tune:false;"), vec![
            TciEvent::Tune { rx: Some(0), on: true },
            TciEvent::Tune { rx: None, on: false },
        ]);
        for bad in ["drive:0,140;", "drive:x;", "tune:0,forse;", "split_enable:true;"] {
            assert!(matches!(parse_text(bad).as_slice(), [TciEvent::Other { .. }]), "{bad}");
        }
        assert_eq!(cmd_rx_enable(1, true), "rx_enable:1,true;");
        assert_eq!(cmd_drive(None, 30), "drive:30;");
        assert_eq!(cmd_tune(None, true), "tune:true;");
    }

    #[test]
    fn it_builds_commands_like_wsjtx() {
        assert_eq!(cmd_vfo(0, 0, 14_074_000), "vfo:0,0,14074000;");
        assert_eq!(cmd_modulation(0, "digu"), "modulation:0,digu;");
        assert_eq!(cmd_trx(0, true, false), "trx:0,true;");
        assert_eq!(cmd_trx(0, false, true), "trx:0,false,tci;");
        assert_eq!(cmd_audio_start(0), "audio_start:0;");
        assert_eq!(cmd_audio_stop(1), "audio_stop:1;");
        assert_eq!(cmd_audio_samplerate(48_000), "audio_samplerate:48000;");
        assert_eq!(tci_mode(CatMode::Data), Some("digu"));
        assert_eq!(tci_mode(CatMode::None), None);
    }

    #[test]
    fn the_header_has_the_wsjtx_layout() {
        let h = StreamHeader { receiver: 1, sample_rate: 48_000, format: 3, codec: 0, crc: 9, length: 4, kind: StreamType::RxAudio, channels: 2 };
        let bytes = h.to_bytes();
        assert_eq!(bytes.len(), 64);
        // offset dei campi come gli static_assert di TCIStream.hpp: length 20, type 24, channels 28
        assert_eq!(u32::from_le_bytes(bytes[20..24].try_into().unwrap()), 4);
        assert_eq!(u32::from_le_bytes(bytes[24..28].try_into().unwrap()), 1);
        assert_eq!(u32::from_le_bytes(bytes[28..32].try_into().unwrap()), 2);
        assert!(bytes[32..].iter().all(|b| *b == 0));
        assert_eq!(StreamHeader::parse(&bytes), Some(h));
        assert_eq!(StreamHeader::parse(&bytes[..63]), None);
    }

    #[test]
    fn the_payload_must_be_complete() {
        let h = StreamHeader { receiver: 0, sample_rate: 48_000, format: 3, codec: 0, crc: 0, length: 4, kind: StreamType::RxAudio, channels: 2 };
        let mut frame = h.to_bytes().to_vec();
        for s in [0.5f32, -0.5, 0.25, 0.75] {
            frame.extend_from_slice(&s.to_le_bytes());
        }
        assert_eq!(float_payload(&frame, &h), Some(vec![0.5, -0.5, 0.25, 0.75]));
        assert_eq!(float_payload(&frame[..frame.len() - 1], &h), None);
        assert_eq!(downmix(&[0.5, -0.5, 0.25, 0.75], 2), vec![0.0, 0.5]);
        let too_long = StreamHeader { length: MAX_FLOAT_SAMPLES + 2, ..h };
        assert_eq!(float_payload(&frame, &too_long), None);
    }

    #[test]
    fn channels_follow_the_protocol_version() {
        assert!(protocol_has_channels("1.9"));
        assert!(protocol_has_channels("2.0"));
        assert!(!protocol_has_channels("1.5"));
        assert!(!protocol_has_channels(""));
        let h = StreamHeader { receiver: 0, sample_rate: 48_000, format: 3, codec: 0, crc: 0, length: 6, kind: StreamType::TxChrono, channels: 1 };
        assert_eq!(h.channel_count(false), 2);
        assert_eq!(h.channel_count(true), 1);
        assert_eq!(StreamHeader { channels: 5, ..h }.channel_count(true), 0);
    }

    #[test]
    fn it_answers_a_chrono_with_tx_audio() {
        let chrono = StreamHeader { receiver: 0, sample_rate: 48_000, format: 3, codec: 0, crc: 0, length: 6, kind: StreamType::TxChrono, channels: 2 };
        assert_eq!(valid_tx_chrono(&chrono, true, 48_000), Some(2));
        assert_eq!(valid_tx_chrono(&chrono, true, 12_000), None);
        assert_eq!(valid_tx_chrono(&StreamHeader { length: 5, ..chrono }, true, 48_000), None);
        assert_eq!(valid_tx_chrono(&StreamHeader { format: 0, ..chrono }, true, 48_000), None);
        let frame = tx_audio_frame(&chrono, 2, &[0.1, 0.2]).expect("blocco");
        let h = StreamHeader::parse(&frame).expect("intestazione");
        assert_eq!((h.kind, h.length, h.channels, h.sample_rate, h.format), (StreamType::TxAudio, 6, 2, 48_000, 3));
        // il terzo istante manca: silenzio
        assert_eq!(float_payload(&frame, &h), Some(vec![0.1, 0.1, 0.2, 0.2, 0.0, 0.0]));
        assert_eq!(tx_audio_frame(&chrono, 3, &[]), None);
    }

    /// L'eco di un server TCI: i comandi di scrittura tornano indietro come conferma (ExpertSDR manda il nuovo stato a tutti i client).
    fn echo_of(text: &str) -> Vec<String> {
        const ECHOED: [&str; 7] = ["vfo", "modulation", "trx", "split_enable", "drive", "tune", "rx_enable"];
        text.split(';')
            .map(str::trim)
            .filter_map(|c| {
                let (name, args) = c.split_once(':')?;
                if !ECHOED.contains(&name) {
                    return None;
                }
                // `trx:0,true,tci` torna senza la sorgente dell'audio
                let args = if name == "trx" { args.split(',').take(2).collect::<Vec<_>>().join(",") } else { args.to_string() };
                Some(format!("{name}:{args};"))
            })
            .collect()
    }

    /// Un server TCI minimo: manda lo stato iniziale e registra i comandi ricevuti, con l'eco come fa ExpertSDR.
    fn fake_server() -> (String, Arc<Mutex<Vec<String>>>) {
        let (address, text, _) = fake_tci("protocol:ExpertSDR3,1.9;vfo:0,0,7074000;modulation:0,usb;trx:0,false;ready;", true);
        (address, text)
    }

    fn wait(cond: impl Fn() -> bool) -> bool {
        let until = Instant::now() + Duration::from_secs(5);
        while Instant::now() < until {
            if cond() {
                return true;
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        false
    }

    #[test]
    fn it_drives_a_tci_server() {
        let (address, seen) = fake_server();
        let rig = TciRig::start(&address, CatMode::Data);
        assert!(wait(|| rig.status().state == LinkState::Connected), "{:?}", rig.status());
        assert_eq!(rig.frequency(), Ok(7_074_000));
        assert_eq!(rig.status().mode.as_deref(), Some("USB"));
        rig.set_frequency(14_074_000).expect("frequenza");
        rig.set_ptt(true).expect("PTT");
        assert!(rig.set_frequency(7_074_000).is_err(), "niente cambio di frequenza in trasmissione");
        rig.set_ptt(false).expect("PTT");
        let commands = || seen.lock().unwrap().iter().filter(|c| !c.contains("sensors_enable")).cloned().collect::<Vec<_>>();
        assert!(wait(|| commands().len() >= 4));
        assert_eq!(commands(), vec!["vfo:0,0,14074000;", "modulation:0,digu;", "trx:0,true;", "trx:0,false;"]);
        assert!(seen.lock().unwrap().iter().any(|c| c == "rx_sensors_enable:true,500;tx_sensors_enable:true,500;"), "misuratori chiesti dopo ready");
        assert!(wait(|| rig.frequency() == Ok(14_074_000)));
        rig.shutdown();
        assert_eq!(rig.status().state, LinkState::Disconnected);
    }

    #[test]
    fn it_reads_the_meters() {
        let args = |s: &str| s.split(',').map(str::to_string).collect::<Vec<_>>();
        assert_eq!(meter_reading("rx_smeter", &args("0,0,-73"), 0), vec![(Level::Strength, 0.0)]);
        assert_eq!(meter_reading("rx_smeter", &args("0,1,-73"), 0), vec![], "solo il canale 0");
        assert_eq!(meter_reading("rx_sensors", &args("1,-90"), 0), vec![], "altro ricevitore");
        assert_eq!(meter_reading("tx_power", &args("50.5"), 0), vec![(Level::PowerWatts, 50.5)]);
        assert_eq!(meter_reading("tx_swr", &args("1.3"), 0), vec![(Level::Swr, 1.3)]);
        assert_eq!(meter_reading("tx_sensors", &args("0,-20,40,55,1.2"), 0), vec![(Level::PowerWatts, 40.0), (Level::Swr, 1.2)]);
        assert_eq!(meter_reading("tx_swr", &args("x"), 0), vec![]);
        assert_eq!(cmd_split(0, true), "split_enable:0,true;");
        assert_eq!(cmd_drive(Some(0), 140), "drive:0,100;");
        assert_eq!(cmd_tune(Some(0), false), "tune:0,false;");
        assert_eq!((cmd_power(true), cmd_power(false)), ("start;".to_string(), "stop;".to_string()));
    }

    /// Server TCI con audio: dopo `ready;` manda un blocco di audio ricevuto stereo e tre «chrono» di lunghezze diverse; registra
    /// i comandi di testo e i blocchi binari che riceve.
    #[allow(clippy::type_complexity)]
    fn fake_audio_server(greeting: &'static str) -> (String, Arc<Mutex<Vec<String>>>, Arc<Mutex<Vec<Vec<u8>>>>) {
        fake_tci(greeting, true)
    }

    /// Il server di prova: con `echo` conferma i comandi di scrittura, senza resta muto come un server che non li ha eseguiti.
    #[allow(clippy::type_complexity)]
    fn fake_tci(greeting: &'static str, echo: bool) -> (String, Arc<Mutex<Vec<String>>>, Arc<Mutex<Vec<Vec<u8>>>>) {
        let listener = TcpListener::bind("127.0.0.1:0").expect("porta");
        let address = listener.local_addr().expect("indirizzo").to_string();
        let (text, binary) = (Arc::new(Mutex::new(Vec::new())), Arc::new(Mutex::new(Vec::new())));
        let (log_text, log_binary) = (Arc::clone(&text), Arc::clone(&binary));
        std::thread::spawn(move || {
            let (stream, _) = listener.accept().expect("accept");
            let mut ws = tungstenite::accept(stream).expect("handshake");
            ws.send(Message::text(greeting)).expect("stato");
            let mut sent_audio = false;
            while let Ok(msg) = ws.read() {
                match msg {
                    Message::Text(t) => {
                        let t = t.as_str().to_string();
                        let start = t.contains("audio_start") || t.contains("trx:0,true");
                        let echoes = if echo { echo_of(&t) } else { Vec::new() };
                        log_text.lock().unwrap().push(t);
                        if start && !sent_audio {
                            sent_audio = true;
                            let rx = StreamHeader { receiver: 0, sample_rate: 48_000, format: 3, codec: 0, crc: 0, length: 4, kind: StreamType::RxAudio, channels: 2 };
                            let mut frame = rx.to_bytes().to_vec();
                            for s in [0.2f32, 0.4, -0.2, -0.4] {
                                frame.extend_from_slice(&s.to_le_bytes());
                            }
                            let _ = ws.send(Message::binary(frame));
                            for length in [8u32, 4, 6] {
                                let chrono = StreamHeader { length, kind: StreamType::TxChrono, ..rx };
                                let _ = ws.send(Message::binary(chrono.to_bytes().to_vec()));
                            }
                        }
                        for e in echoes {
                            let _ = ws.send(Message::text(e));
                        }
                    }
                    Message::Binary(b) => log_binary.lock().unwrap().push(b.to_vec()),
                    _ => {}
                }
            }
        });
        (address, text, binary)
    }

    #[test]
    fn it_streams_audio_both_ways() {
        let (address, text, binary) = fake_audio_server("protocol:ExpertSDR3,1.9;vfo:0,0,14074000;trx:0,false;ready;");
        let rig = TciRig::start(&address, CatMode::None);
        let heard: Arc<Mutex<Vec<(Vec<f32>, u32)>>> = Arc::new(Mutex::new(Vec::new()));
        let h = Arc::clone(&heard);
        rig.set_rx_audio(Some(Arc::new(move |s: &[f32], rate| h.lock().unwrap().push((s.to_vec(), rate)))));
        // l'onda da trasmettere: 1, 2, 3... consegnata un chrono alla volta
        let next = Arc::new(Mutex::new(1.0f32));
        let n = Arc::clone(&next);
        rig.set_tx_audio(Some(Arc::new(move |buf: &mut [f32], rate| {
            assert_eq!(rate, 48_000);
            let mut v = n.lock().unwrap();
            for s in buf.iter_mut() {
                *s = *v;
                *v += 1.0;
            }
            buf.len()
        })));
        assert!(wait(|| rig.status().state == LinkState::Connected));
        assert!(wait(|| text.lock().unwrap().iter().any(|t| t == "audio_start:0;")), "{:?}", text.lock().unwrap());
        rig.set_ptt(true).expect("PTT");
        assert!(wait(|| text.lock().unwrap().iter().any(|t| t == "trx:0,true,tci;")), "PTT con l'audio dal flusso TCI");
        assert!(wait(|| !heard.lock().unwrap().is_empty()));
        assert_eq!(heard.lock().unwrap()[0], (vec![0.3, -0.3], 48_000), "stereo portato a mono");
        assert!(wait(|| binary.lock().unwrap().len() == 3));
        let frames = binary.lock().unwrap().clone();
        let mut all = Vec::new();
        for (frame, length) in frames.iter().zip([8u32, 4, 6]) {
            let h = StreamHeader::parse(frame).expect("intestazione");
            assert_eq!((h.kind, h.length, h.channels, h.sample_rate), (StreamType::TxAudio, length, 2, 48_000), "stessa lunghezza del chrono");
            let samples = float_payload(frame, &h).expect("campioni");
            all.extend(downmix(&samples, 2));
        }
        assert_eq!(all, (1..=9).map(|v| v as f32).collect::<Vec<_>>(), "l'onda continua da un chrono all'altro");
        rig.shutdown();
    }

    #[test]
    fn split_and_power_follow_the_config() {
        let (address, text, _) = fake_audio_server("protocol:ExpertSDR3,1.9;vfo:0,0,14074000;ready;");
        let config = RigConfig { tci_address: address, split: SplitMode::Rig, power_on: true, power_off: true, ..RigConfig::default() };
        let rig = TciRig::from_config(&config);
        assert!(wait(|| rig.status().state == LinkState::Connected));
        rig.set_tx_audio_frequency(2200).expect("split");
        assert_eq!(rig.status().split, Some(true), "split confermato dall'eco");
        rig.power_off_if_chosen();
        rig.shutdown();
        assert!(wait(|| text.lock().unwrap().iter().any(|c| c == "stop;")), "{:?}", text.lock().unwrap());
        let seen = text.lock().unwrap().clone();
        let pos = |c: &str| seen.iter().position(|t| t == c).unwrap_or_else(|| panic!("{c} manca: {seen:?}"));
        assert!(pos("start;") < pos("vfo:0,1,14074500;"));
        assert!(pos("vfo:0,1,14074500;") < pos("split_enable:0,true;"));
        // come `do_stop` di WSJT-X: lo split acceso da Aurora si toglie prima di spegnere
        assert!(pos("split_enable:0,false;") < pos("stop;"));
        assert_eq!(seen.last().map(String::as_str), Some("stop;"));
    }

    #[test]
    fn emulated_split_moves_the_vfo_only_while_transmitting() {
        let (address, text, _) = fake_audio_server("vfo:0,0,14074000;ready;");
        let rig = TciRig::from_config(&RigConfig { tci_address: address, split: SplitMode::Emulate, ..RigConfig::default() });
        assert!(wait(|| rig.status().state == LinkState::Connected));
        rig.set_tx_audio_frequency(300).expect("frequenza audio");
        rig.set_ptt(true).expect("PTT");
        rig.set_ptt(false).expect("PTT");
        let expected = ["vfo:0,0,14072500;", "trx:0,true;", "trx:0,false;", "vfo:0,0,14074000;"];
        let filtered = || text.lock().unwrap().iter().filter(|c| !c.contains("sensors")).cloned().collect::<Vec<_>>();
        assert!(wait(|| filtered().len() >= 4));
        assert_eq!(filtered(), expected);
    }

    #[test]
    fn without_an_echo_a_command_is_sent_twice_and_then_fails() {
        let (address, text, _) = fake_tci("protocol:ExpertSDR3,1.9;vfo:0,0,14074000;trx:0,false;ready;", false);
        let rig = TciRig::start(&address, CatMode::None);
        assert!(wait(|| rig.status().state == LinkState::Connected));
        let err = rig.set_frequency(7_074_000).expect_err("nessuna eco della frequenza");
        assert!(err.0.contains("vfo") && err.0.contains("nessuna eco"), "{err:?}");
        assert_eq!(rig.frequency(), Ok(14_074_000), "la frequenza resta quella detta dal server");
        let err = rig.set_ptt(true).expect_err("nessuna eco del PTT");
        assert!(err.0.contains("trx"), "{err:?}");
        assert!(!rig.transmitting());
        let seen = text.lock().unwrap().clone();
        assert_eq!(seen.iter().filter(|c| *c == "vfo:0,0,7074000;").count(), 2, "ripetuto una volta: {seen:?}");
        assert_eq!(seen.iter().filter(|c| *c == "trx:0,true;").count(), 2, "{seen:?}");
        assert!(seen.iter().any(|c| c == "trx:0,false;"), "senza conferma si chiede comunque lo spegnimento");
        rig.shutdown();
    }

    #[test]
    fn the_same_frequency_is_not_sent_again() {
        let (address, seen) = fake_server();
        let rig = TciRig::start(&address, CatMode::Usb);
        assert!(wait(|| rig.status().state == LinkState::Connected));
        // frequenza e modo gia' quelli della radio: come WSJT-X non si manda niente
        rig.set_frequency(7_074_000).expect("frequenza");
        std::thread::sleep(Duration::from_millis(100));
        assert!(seen.lock().unwrap().iter().all(|c| c.contains("sensors_enable")), "{:?}", seen.lock().unwrap());
        rig.shutdown();
    }

    #[test]
    fn rx2_is_switched_on_and_off_like_wsjtx() {
        let (address, text, _) = fake_tci("protocol:ExpertSDR3,1.9;rx_enable:1,false;vfo:0,0,14074000;vfo:1,0,7074000;modulation:1,usb;ready;", true);
        let rig = TciRig::from_config(&RigConfig { tci_address: address, tci_receiver: 1, cat_mode: CatMode::Data, ..RigConfig::default() });
        assert_eq!(rig.receiver(), 1);
        assert!(wait(|| rig.status().state == LinkState::Connected), "{:?}", rig.status());
        assert_eq!(rig.frequency(), Ok(7_074_000), "la frequenza e' quella del ricevitore 1");
        rig.set_frequency(10_136_000).expect("frequenza");
        rig.set_ptt(true).expect("PTT");
        rig.set_ptt(false).expect("PTT");
        rig.shutdown();
        // l'ultimo comando parte con la chiusura: si aspetta che il server l'abbia letto
        assert!(wait(|| text.lock().unwrap().iter().any(|c| c == "rx_enable:1,false;")), "{:?}", text.lock().unwrap());
        let seen: Vec<String> = text.lock().unwrap().iter().filter(|c| !c.contains("sensors")).cloned().collect();
        assert_eq!(seen, vec![
            "rx_enable:1,true;",
            "vfo:1,0,10136000;",
            "modulation:1,digu;",
            "trx:1,true;",
            "trx:1,false;",
            "rx_enable:1,false;",
        ]);
    }

    #[test]
    fn rx2_already_on_is_left_on() {
        let (address, text, _) = fake_tci("protocol:ExpertSDR3,1.9;rx_enable:1,true;vfo:1,0,7074000;ready;", true);
        let rig = TciRig::from_config(&RigConfig { tci_address: address, tci_receiver: 1, ..RigConfig::default() });
        assert!(wait(|| rig.status().state == LinkState::Connected));
        rig.shutdown();
        assert!(text.lock().unwrap().iter().all(|c| !c.starts_with("rx_enable")), "{:?}", text.lock().unwrap());
    }

    #[test]
    fn without_rx2_the_link_is_not_ready() {
        let (address, text, _) = fake_tci("protocol:ExpertSDR3,1.9;rx_enable:1,false;vfo:1,0,7074000;ready;", false);
        let rig = TciRig::from_config(&RigConfig { tci_address: address, tci_receiver: 1, ..RigConfig::default() });
        assert!(wait(|| rig.status().state == LinkState::Error), "{:?}", rig.status());
        assert_eq!(rig.status().message, RX2_FAILED);
        assert!(rig.set_frequency(7_074_000).is_err());
        assert_eq!(text.lock().unwrap().iter().filter(|c| *c == "rx_enable:1,true;").count(), 2);
        rig.shutdown();
    }

    #[test]
    fn drive_and_tune_follow_the_display_buttons() {
        let (address, text, _) = fake_tci("protocol:ExpertSDR3,1.9;vfo:0,0,14074000;drive:0,37;tune:0,false;ready;", true);
        let rig = TciRig::start(&address, CatMode::None);
        assert!(wait(|| rig.status().state == LinkState::Connected));
        assert_eq!(rig.drive(), Some(37));
        assert_eq!(rig.status().extras.power_set, Some(0.37), "PWR del display dalla SDR");
        assert_eq!(rig.status().extras.tuner, Some(false));
        rig.set_extra(ExtraItem::PowerUp).expect("PWR +");
        assert_eq!(rig.drive(), Some(40), "a passi interi del 5%");
        rig.set_extra(ExtraItem::PowerDown).expect("PWR -");
        assert_eq!(rig.drive(), Some(35));
        rig.set_extra(ExtraItem::Tune).expect("TUNE");
        assert!(rig.tuning());
        rig.set_extra(ExtraItem::Tune).expect("TUNE di nuovo");
        assert!(!rig.tuning());
        assert!(rig.set_extra(ExtraItem::Nb).is_err(), "NB non esiste nel TCI");
        rig.shutdown();
        let seen: Vec<String> = text.lock().unwrap().iter().filter(|c| !c.contains("sensors")).cloned().collect();
        assert_eq!(seen, vec!["drive:0,40;", "drive:0,35;", "tune:0,true;", "tune:0,false;"]);
    }

    #[test]
    fn without_a_known_drive_pwr_is_refused() {
        let (address, _, _) = fake_tci("protocol:ExpertSDR3,1.9;vfo:0,0,14074000;ready;", true);
        let rig = TciRig::start(&address, CatMode::None);
        assert!(wait(|| rig.status().state == LinkState::Connected));
        assert_eq!(rig.set_extra(ExtraItem::PowerUp).expect_err("potenza non nota").0, DRIVE_UNKNOWN);
        rig.shutdown();
    }

    #[test]
    fn it_reports_an_unreachable_server() {
        let listener = TcpListener::bind("127.0.0.1:0").expect("porta");
        let address = listener.local_addr().expect("indirizzo").to_string();
        drop(listener);
        let rig = TciRig::start(&address, CatMode::None);
        assert!(wait(|| matches!(rig.status().state, LinkState::Error | LinkState::Reconnecting)));
        assert!(rig.set_frequency(14_074_000).is_err());
        assert!(rig.set_ptt(true).is_err());
    }
}
