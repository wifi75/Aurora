//! TCI (Expert Electronics): comando della radio e flussi audio su WebSocket, per le SDR (SunSDR/ExpertSDR, ANAN con Thetis...).
//!
//! Il protocollo e' quello letto in WSJT-X 3 (`Transceiver/TCITransceiver.cpp`, `TCIStream.hpp`, `TCIProtocol.hpp`):
//! - testo: comandi `nome:arg,arg,...;` (piu' comandi nello stesso messaggio, separati da `;`). Il server all'avvio manda il suo stato
//!   (`protocol:ExpertSDR3,1.9;`, `device:...;`, `vfo:0,0,14074000;`, `modulation:0,usb;`, `trx:0,false;`...) e chiude con `ready;`.
//!   Aurora manda `vfo:<rx>,<vfo>,<hz>;`, `modulation:<rx>,<modo>;`, `trx:<rx>,<true|false>[,tci];`, `audio_start:<rx>;`, `audio_stop:<rx>;`.
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
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    thread::JoinHandle,
    time::{Duration, Instant},
};

use tungstenite::{Message, WebSocket};

use crate::{
    cat_config::CatMode,
    hamlib_rig::{valid_dial_hz, LinkState, RigStatus},
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

struct Inner {
    address: String,
    cat_mode: CatMode,
    rx: u32,
    stop: AtomicBool,
    ready: AtomicBool,
    channels_field: AtomicBool,
    /// Il flusso audio ricevuto e' acceso sul server (`audio_start` confermato).
    audio_on: AtomicBool,
    status: Mutex<RigStatus>,
    outbox: Mutex<VecDeque<String>>,
    rx_audio: Mutex<Option<RxAudioFn>>,
    tx_audio: Mutex<Option<TxAudioFn>>,
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

    fn handle_text(&self, text: &str) {
        for event in parse_text(text) {
            match event {
                TciEvent::Vfo { rx, vfo: 0, hz } if rx == self.rx => self.update(|s| s.frequency_hz = Some(hz)),
                TciEvent::Modulation { rx, mode } if rx == self.rx => self.update(|s| s.mode = Some(mode.to_ascii_uppercase())),
                TciEvent::Trx { rx, on } if rx == self.rx => self.update(|s| s.ptt = on),
                TciEvent::Protocol { version, .. } => self.channels_field.store(protocol_has_channels(&version), Ordering::SeqCst),
                TciEvent::AudioStart { rx } if rx == self.rx => self.audio_on.store(true, Ordering::SeqCst),
                TciEvent::AudioStop { rx } if rx == self.rx => self.audio_on.store(false, Ordering::SeqCst),
                TciEvent::Stop => self.update(|s| s.ptt = false),
                TciEvent::Ready => {
                    self.ready.store(true, Ordering::SeqCst);
                    self.update(|s| {
                        s.state = LinkState::Connected;
                        s.message.clear();
                        s.reconnect_attempt = 0;
                    });
                    // con un ricevitore dell'audio gia' scelto si chiede il flusso, come `stream_audio(true)` di WSJT-X
                    if self.rx_audio.lock().is_ok_and(|a| a.is_some()) {
                        self.send(cmd_audio_start(self.rx));
                    }
                }
                _ => {}
            }
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
/// riaperto) e la radio risulta collegata solo dopo il `ready;` del server.
pub struct TciRig {
    inner: Arc<Inner>,
    worker: Mutex<Option<JoinHandle<()>>>,
}

impl TciRig {
    pub fn start(address: &str, cat_mode: CatMode) -> TciRig {
        let address = if address.trim().is_empty() { DEFAULT_TCI_ADDRESS.to_string() } else { address.trim().to_string() };
        let inner = Arc::new(Inner {
            address,
            cat_mode,
            rx: 0,
            stop: AtomicBool::new(false),
            ready: AtomicBool::new(false),
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
        });
        let worker_inner = Arc::clone(&inner);
        let worker = std::thread::Builder::new().name("tci".into()).spawn(move || run(&worker_inner)).ok();
        TciRig { inner, worker: Mutex::new(worker) }
    }

    pub fn address(&self) -> &str {
        &self.inner.address
    }

    pub fn status(&self) -> RigStatus {
        self.inner.status.lock().map(|s| s.clone()).unwrap_or_else(|e| e.into_inner().clone())
    }

    pub fn transmitting(&self) -> bool {
        self.status().ptt
    }

    /// Il PTT con `trx`. Senza ricevitore dell'audio TCI per la trasmissione la SDR prende l'audio dalla sua sorgente (cavo virtuale).
    pub fn set_ptt(&self, on: bool) -> Result<(), RigError> {
        self.inner.connected()?;
        let tci_audio = self.inner.tx_audio.lock().is_ok_and(|a| a.is_some());
        self.inner.send(cmd_trx(self.inner.rx, on, tci_audio));
        self.inner.update(|s| s.ptt = on);
        Ok(())
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

    /// Chiude: PTT spento, flusso audio fermato, collegamento chiuso.
    pub fn shutdown(&self) {
        if self.inner.ready.load(Ordering::SeqCst) {
            if self.transmitting() {
                self.inner.send(cmd_trx(self.inner.rx, false, false));
            }
            if self.audio_streaming() {
                self.inner.send(cmd_audio_stop(self.inner.rx));
            }
        }
        self.inner.stop.store(true, Ordering::SeqCst);
        if let Some(handle) = self.worker.lock().ok().and_then(|mut w| w.take()) {
            let _ = handle.join();
        }
        self.inner.ready.store(false, Ordering::SeqCst);
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
                inner.ready.store(false, Ordering::SeqCst);
                inner.audio_on.store(false, Ordering::SeqCst);
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
            return "chiuso".into();
        }
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
    /// Cambio di frequenza del VFO di ricezione (non in trasmissione) e, se scelto, il modo (USB o USB dati).
    fn set_frequency(&self, hz: u64) -> Result<(), RigError> {
        if !valid_dial_hz(hz) {
            return Err(RigError(format!("frequenza non valida: {hz} Hz")));
        }
        self.inner.connected()?;
        if self.transmitting() {
            return Err(RigError("la radio sta trasmettendo: cambio di frequenza rifiutato".into()));
        }
        self.inner.send(cmd_vfo(self.inner.rx, 0, hz));
        if let Some(mode) = tci_mode(self.inner.cat_mode)
            && self.status().mode.as_deref().is_none_or(|m| !m.eq_ignore_ascii_case(mode))
        {
            self.inner.send(cmd_modulation(self.inner.rx, mode));
        }
        self.inner.update(|s| s.frequency_hz = Some(hz));
        Ok(())
    }

    fn frequency(&self) -> Result<u64, RigError> {
        let s = self.status();
        match (s.state, s.frequency_hz) {
            (LinkState::Connected, Some(hz)) => Ok(hz),
            _ => Err(RigError("radio TCI non collegata".into())),
        }
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

    /// Un server TCI minimo: manda lo stato iniziale e registra i comandi ricevuti.
    fn fake_server() -> (String, Arc<Mutex<Vec<String>>>) {
        let listener = TcpListener::bind("127.0.0.1:0").expect("porta");
        let address = listener.local_addr().expect("indirizzo").to_string();
        let seen = Arc::new(Mutex::new(Vec::new()));
        let log = Arc::clone(&seen);
        std::thread::spawn(move || {
            let (stream, _) = listener.accept().expect("accept");
            let mut ws = tungstenite::accept(stream).expect("handshake");
            ws.send(Message::text("protocol:ExpertSDR3,1.9;vfo:0,0,7074000;modulation:0,usb;trx:0,false;ready;")).expect("stato");
            while let Ok(msg) = ws.read() {
                if let Message::Text(t) = msg {
                    let t = t.as_str().to_string();
                    // la radio conferma frequenza e trasmissione come fa ExpertSDR
                    let echo = if t.starts_with("vfo:") || t.starts_with("trx:") { Some(t.clone()) } else { None };
                    log.lock().unwrap().push(t);
                    if let Some(e) = echo {
                        let _ = ws.send(Message::text(e));
                    }
                }
            }
        });
        (address, seen)
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
        assert!(wait(|| seen.lock().unwrap().len() >= 4));
        assert_eq!(*seen.lock().unwrap(), vec!["vfo:0,0,14074000;", "modulation:0,digu;", "trx:0,true;", "trx:0,false;"]);
        assert!(wait(|| rig.frequency() == Ok(14_074_000)));
        rig.shutdown();
        assert_eq!(rig.status().state, LinkState::Disconnected);
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
