//! FT-991(A) Link: un programma di terze parti che tiene la porta COM delle Yaesu FT-991/FT-991A e offre un server UDP (di base
//! `127.0.0.1:50001`). Si manda un comando CAT Yaesu testuale per datagramma, terminato da `;`, e la risposta arriva al mittente nello
//! stesso formato.
//!
//! Comandi usati (CAT Yaesu FT-991):
//! - `PS;` -> `PS0;` spenta / `PS1;` accesa; `PS1;` / `PS0;` accende o spegne.
//! - `ID;` -> `ID0570;` FT-991, `ID0670;` FT-991A.
//! - `FA;` -> `FA014074000;` (9 cifre, Hz); la scrittura `FA014074000;` risponde con la frequenza.
//! - `MD0;` -> `MD0C;` (il codice del modo, vedi [`mode_name`]); la scrittura `MD0C;` mette DATA-USB.
//! - `TX1;` / `TX0;` PTT via CAT; `TX;` -> `TX0;` ricezione, `TX1;` trasmissione da CAT, `TX2;` trasmissione dalla radio.
//! - `SM0;` -> `SM0nnn;` (0-255): S-meter, usato solo se il programma lo sa dare.
//!
//! Solo il VFO A: lo split e' sempre quello simulato di WSJT-X («Fake It»), il VFO A si sposta sulla frequenza di trasmissione prima del
//! PTT e torna a fine trasmissione (la stessa regola del TCI e di Hamlib, `split_tx_dial_hz`).
//!
//! Il collegamento vive in un thread: un socket UDP locale effimero, risposta attesa al massimo `reply` (500 ms) con una ripetizione,
//! lettura periodica di frequenza, modo, PTT (e S-meter) e ricollegamento automatico se il programma smette di rispondere.

use std::{
    io::ErrorKind,
    net::{SocketAddr, ToSocketAddrs, UdpSocket},
    sync::{
        atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering},
        Arc, Mutex,
    },
    thread::JoinHandle,
    time::{Duration, Instant},
};

use crate::{
    cat_config::{split_tx_dial_hz, CatMode, RigConfig, SplitMode},
    hamlib_rig::{valid_dial_hz, LinkState, RigStatus},
    meters::{self, Level, Meters},
    Rig, RigError,
};

/// Indirizzo di base del server UDP di FT-991(A) Link.
pub const DEFAULT_FT991_ADDRESS: &str = "127.0.0.1:50001";

/// La frequenza piu' alta che `FA` sa scrivere (9 cifre).
const MAX_FA_HZ: u64 = 999_999_999;

/// Il messaggio quando il programma non risponde (anche tradotto dall'interfaccia, `coreText`).
pub fn not_answering(address: &str) -> String {
    format!("FT-991(A) Link non risponde su {address}: in FT991Link apri COM Port e premi Start UDP")
}

/// Il messaggio quando il programma risponde ma la radio e' spenta (`PS0;`).
pub const RADIO_OFF: &str = "La radio e' spenta (PS0): accendila oppure spunta «Accendi» nella scheda Radio";

/// Il messaggio mentre si ricollega dopo un guasto.
pub const LINK_LOST: &str = "Collegamento con FT-991(A) Link perso: nuovo tentativo";

// ---------------------------------------------------------------- testo

/// Le risposte di un datagramma: le parti separate da `;`, senza spazi, quelle vuote saltate.
pub fn replies(text: &str) -> Vec<String> {
    text.split(';').map(str::trim).filter(|p| !p.is_empty()).map(str::to_string).collect()
}

fn digits(text: &str) -> Option<u64> {
    if text.is_empty() || !text.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    text.parse().ok()
}

/// `FA014074000` -> 14074000 Hz (9 cifre esatte).
pub fn parse_frequency(reply: &str) -> Option<u64> {
    let rest = reply.strip_prefix("FA")?;
    if rest.len() != 9 {
        return None;
    }
    digits(rest)
}

/// `MD0C` -> `'C'`: il codice del modo del VFO principale (solo i codici noti).
pub fn parse_mode(reply: &str) -> Option<char> {
    let rest = reply.strip_prefix("MD0")?;
    let mut chars = rest.chars();
    let code = chars.next()?.to_ascii_uppercase();
    (chars.next().is_none() && mode_name(code).is_some()).then_some(code)
}

/// Il nome del modo per un codice `MD` della FT-991.
pub fn mode_name(code: char) -> Option<&'static str> {
    Some(match code.to_ascii_uppercase() {
        '1' => "LSB",
        '2' => "USB",
        '3' => "CW",
        '4' => "FM",
        '5' => "AM",
        '6' => "RTTY-LSB",
        '7' => "CW-R",
        '8' => "DATA-LSB",
        '9' => "RTTY-USB",
        'A' => "DATA-FM",
        'B' => "FM-N",
        'C' => "DATA-USB",
        'D' => "AM-N",
        'E' => "C4FM",
        _ => return None,
    })
}

/// Il codice `MD` per il modo CAT scelto: USB (`2`) o DATA-USB (`C`, quello per FT8); `None` = non si tocca.
pub fn mode_code(mode: CatMode) -> Option<char> {
    match mode {
        CatMode::None => None,
        CatMode::Usb => Some('2'),
        CatMode::Data => Some('C'),
    }
}

/// `TX0` ricezione; `TX1` (PTT da CAT) e `TX2` (PTT della radio) trasmissione.
pub fn parse_tx(reply: &str) -> Option<bool> {
    match reply.strip_prefix("TX")? {
        "0" => Some(false),
        "1" | "2" => Some(true),
        _ => None,
    }
}

/// `PS0` spenta, `PS1` accesa.
pub fn parse_power(reply: &str) -> Option<bool> {
    match reply.strip_prefix("PS")? {
        "0" => Some(false),
        "1" => Some(true),
        _ => None,
    }
}

/// Il modello dalla risposta a `ID;`: `ID0570` FT-991, `ID0670` FT-991A, altro = una radio diversa (`Some("")`).
pub fn parse_id(reply: &str) -> Option<&'static str> {
    match reply.strip_prefix("ID")? {
        "0570" => Some("FT-991"),
        "0670" => Some("FT-991A"),
        _ => Some(""),
    }
}

/// `SM0123` -> 123 (0-255).
pub fn parse_smeter(reply: &str) -> Option<u32> {
    digits(reply.strip_prefix("SM0")?).filter(|v| *v <= 255).map(|v| v as u32)
}

/// La lettura grezza dello S-meter (0-255) in dB rispetto a S9, come lo `STRENGTH` di Hamlib (S9 = 0 dB, S0 = -54 dB, fondo scala
/// +60 dB). Tabella approssimata sulla scala della FT-991 (S9 a circa 130), interpolata tra i punti.
pub fn smeter_db(raw: u32) -> f32 {
    const TABLE: [(u32, f32); 16] = [
        (0, -54.0),
        (12, -48.0),
        (27, -42.0),
        (40, -36.0),
        (55, -30.0),
        (65, -24.0),
        (80, -18.0),
        (95, -12.0),
        (112, -6.0),
        (130, 0.0),
        (150, 10.0),
        (172, 20.0),
        (190, 30.0),
        (220, 40.0),
        (240, 50.0),
        (255, 60.0),
    ];
    let raw = raw.min(255);
    for pair in TABLE.windows(2) {
        let ((x0, y0), (x1, y1)) = (pair[0], pair[1]);
        if raw <= x1 {
            return y0 + (y1 - y0) * (raw - x0) as f32 / (x1 - x0) as f32;
        }
    }
    60.0
}

/// `FA014074000;`
pub fn cmd_frequency(hz: u64) -> String {
    format!("FA{hz:09};")
}

/// `MD0C;`
pub fn cmd_mode(code: char) -> String {
    format!("MD0{code};")
}

/// `TX1;` / `TX0;`
pub fn cmd_ptt(on: bool) -> String {
    if on { "TX1;".into() } else { "TX0;".into() }
}

/// `PS1;` / `PS0;`
pub fn cmd_power(on: bool) -> String {
    if on { "PS1;".into() } else { "PS0;".into() }
}

// ---------------------------------------------------------------- collegamento UDP

/// Le attese del collegamento.
#[derive(Debug, Clone)]
pub struct Ft991Timing {
    /// Intervallo di lettura in ricezione.
    pub poll: Duration,
    /// Intervallo di lettura in trasmissione (solo il PTT).
    pub poll_tx: Duration,
    /// Attesa massima della risposta a un comando.
    pub reply: Duration,
    /// Tentativi per una lettura (il primo e le ripetizioni).
    pub attempts: u32,
    /// Letture fallite di fila prima di ricollegarsi.
    pub tolerate: u32,
    /// Attesa tra un tentativo di collegamento e l'altro: cresce di questo passo fino al tetto.
    pub retry_step: Duration,
    pub retry_cap: Duration,
    /// Dopo «Accendi»: attesa prima di ricontrollare che la radio sia accesa.
    pub power_wait: Duration,
}

impl Default for Ft991Timing {
    fn default() -> Self {
        Ft991Timing {
            poll: Duration::from_millis(1000),
            poll_tx: Duration::from_millis(500),
            reply: Duration::from_millis(500),
            attempts: 2,
            tolerate: 3,
            retry_step: Duration::from_millis(1000),
            retry_cap: Duration::from_secs(5),
            power_wait: Duration::from_secs(3),
        }
    }
}

/// Perche' una domanda non ha avuto risposta.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AskError {
    /// Nessuna risposta valida nel tempo (anche dopo le ripetizioni).
    Silent,
    /// La radio ha rifiutato il comando (`?;`).
    Rejected,
    /// Errore del socket.
    Io(String),
}

/// Il socket verso FT-991(A) Link: locale, su una porta effimera, «collegato» al server (si ricevono solo i suoi datagrammi).
struct Link {
    socket: UdpSocket,
}

impl Link {
    fn open(address: &str) -> Result<Link, String> {
        let addr: SocketAddr = address.to_socket_addrs().map_err(|e| format!("{address}: {e}"))?.next().ok_or_else(|| format!("{address}: indirizzo non valido"))?;
        let local = if addr.is_ipv4() { "0.0.0.0:0" } else { "[::]:0" };
        let socket = UdpSocket::bind(local).map_err(|e| format!("{address}: {e}"))?;
        socket.connect(addr).map_err(|e| format!("{address}: {e}"))?;
        Ok(Link { socket })
    }

    /// Butta le risposte rimaste (a comandi di scrittura, a domande scadute): la prossima lettura vede solo la sua.
    fn drain(&self) {
        if self.socket.set_nonblocking(true).is_err() {
            return;
        }
        let mut buf = [0u8; 512];
        // un tetto, per non restare qui se il server manda di continuo
        for _ in 0..64 {
            match self.socket.recv(&mut buf) {
                Ok(_) => {}
                Err(e) if e.kind() == ErrorKind::WouldBlock => break,
                // su Windows un «porta non raggiungibile» precedente arriva qui come ConnectionReset: non conta
                Err(_) => {}
            }
        }
        let _ = self.socket.set_nonblocking(false);
    }

    fn send(&self, command: &str) -> Result<(), AskError> {
        self.socket.send(command.as_bytes()).map(|_| ()).map_err(|e| AskError::Io(e.to_string()))
    }

    /// Manda `command` e aspetta una risposta che cominci con `prefix` (le altre si saltano). Ripete fino a `attempts` volte.
    fn ask(&self, command: &str, prefix: &str, timeout: Duration, attempts: u32) -> Result<String, AskError> {
        let mut last = AskError::Silent;
        for _ in 0..attempts.max(1) {
            self.drain();
            self.send(command)?;
            match self.wait_reply(prefix, timeout) {
                Ok(reply) => return Ok(reply),
                Err(AskError::Rejected) => return Err(AskError::Rejected),
                Err(e) => last = e,
            }
        }
        Err(last)
    }

    fn wait_reply(&self, prefix: &str, timeout: Duration) -> Result<String, AskError> {
        let until = Instant::now() + timeout;
        let mut buf = [0u8; 512];
        loop {
            let left = until.saturating_duration_since(Instant::now());
            if left.is_zero() {
                return Err(AskError::Silent);
            }
            self.socket.set_read_timeout(Some(left.max(Duration::from_millis(1)))).map_err(|e| AskError::Io(e.to_string()))?;
            match self.socket.recv(&mut buf) {
                Ok(n) => {
                    let text = String::from_utf8_lossy(&buf[..n]).to_ascii_uppercase();
                    for reply in replies(&text) {
                        if reply == "?" {
                            return Err(AskError::Rejected);
                        }
                        if reply.starts_with(prefix) {
                            return Ok(reply);
                        }
                    }
                }
                Err(e) if matches!(e.kind(), ErrorKind::WouldBlock | ErrorKind::TimedOut) => return Err(AskError::Silent),
                // il server non c'e' (porta chiusa): su Windows arriva come ConnectionReset, come un silenzio
                Err(e) if e.kind() == ErrorKind::ConnectionReset || e.kind() == ErrorKind::ConnectionRefused => {
                    std::thread::sleep(left.min(Duration::from_millis(20)));
                }
                Err(e) => return Err(AskError::Io(e.to_string())),
            }
        }
    }
}

// ---------------------------------------------------------------- la radio

/// Stato dello S-meter: si chiede finche' non si capisce che il programma non lo da'.
const SMETER_UNKNOWN: u32 = 0;
const SMETER_OK: u32 = 1;
const SMETER_NO: u32 = 2;
/// Silenzi di fila dopo cui lo S-meter si considera non disponibile.
const SMETER_MAX_SILENT: u32 = 3;

struct Inner {
    address: String,
    cat_mode: CatMode,
    /// Split simulato (anche se e' scelto quello «della radio»: c'e' solo il VFO A).
    split: bool,
    power_on: bool,
    power_off: bool,
    timing: Ft991Timing,
    stop: AtomicBool,
    ready: AtomicBool,
    link: Mutex<Option<Link>>,
    /// La frequenza audio di trasmissione (Hz, 0 = non ancora data): serve allo split simulato.
    tx_audio_hz: AtomicU32,
    /// Split simulato: la frequenza di ricezione da rimettere a fine trasmissione (0 = niente).
    emulate_restore: AtomicU64,
    smeter: AtomicU32,
    smeter_silent: AtomicU32,
    meters: Mutex<Meters>,
    status: Mutex<RigStatus>,
    /// Dopo un cambio di frequenza la radio cambia banda (anche 1-2 s) e FT-991(A) Link puo' non rispondere: fino a quest'ora le
    /// letture mancate non contano come errori (prima compariva l'avviso rosso a ogni cambio di banda, segnalazione SP4UBW).
    quiet_until: Mutex<Option<std::time::Instant>>,
}

impl Inner {
    fn update(&self, edit: impl FnOnce(&mut RigStatus)) {
        if let Ok(mut s) = self.status.lock() {
            edit(&mut s);
            s.updated_ms = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_millis() as u64);
        }
    }

    fn status(&self) -> RigStatus {
        self.status.lock().map(|s| s.clone()).unwrap_or_else(|e| e.into_inner().clone())
    }

    fn connected(&self) -> Result<(), RigError> {
        if self.ready.load(Ordering::SeqCst) { Ok(()) } else { Err(RigError("radio FT-991(A) Link non collegata".into())) }
    }

    /// Una domanda con la risposta attesa; senza collegamento aperto e' un silenzio.
    fn ask(&self, command: &str, prefix: &str) -> Result<String, AskError> {
        let guard = self.link.lock().map_err(|_| AskError::Io("collegamento non disponibile".into()))?;
        let link = guard.as_ref().ok_or(AskError::Silent)?;
        link.ask(command, prefix, self.timing.reply, self.timing.attempts)
    }

    /// Un comando di scrittura: si manda e basta (la risposta, se arriva, si butta alla prossima domanda).
    fn send(&self, command: &str) -> Result<(), RigError> {
        let guard = self.link.lock().map_err(|_| RigError("collegamento non disponibile".into()))?;
        let link = guard.as_ref().ok_or_else(|| RigError("radio FT-991(A) Link non collegata".into()))?;
        link.drain();
        link.send(command).map_err(|e| RigError(format!("FT-991(A) Link: invio di {command} non riuscito ({e:?})")))
    }

    /// Scrive la frequenza del VFO A. FT-991(A) Link risponde con la frequenza: si aspetta (una volta sola, senza errore se tace) cosi'
    /// il cambio e' fatto prima del comando dopo (il PTT dello split simulato).
    fn write_frequency(&self, hz: u64) -> Result<(), RigError> {
        let guard = self.link.lock().map_err(|_| RigError("collegamento non disponibile".into()))?;
        let link = guard.as_ref().ok_or_else(|| RigError("radio FT-991(A) Link non collegata".into()))?;
        match link.ask(&cmd_frequency(hz), "FA", self.timing.reply, 1) {
            Ok(reply) => {
                let read = parse_frequency(&reply).unwrap_or(hz);
                drop(guard);
                self.update(|s| s.frequency_hz = Some(read));
                Ok(())
            }
            Err(AskError::Silent) => {
                drop(guard);
                self.update(|s| s.frequency_hz = Some(hz));
                Ok(())
            }
            Err(AskError::Rejected) => Err(RigError(format!("la radio ha rifiutato la frequenza {hz} Hz"))),
            Err(AskError::Io(e)) => Err(RigError(format!("FT-991(A) Link: {e}"))),
        }
    }

    /// Una lettura periodica. `false` se il programma non ha risposto alle domande essenziali.
    fn poll(&self) -> bool {
        let ptt = match self.ask("TX;", "TX") {
            Ok(reply) => parse_tx(&reply),
            Err(_) => return false,
        };
        if let Some(on) = ptt {
            self.update(|s| s.ptt = on);
            if !on && let Ok(mut m) = self.meters.lock() {
                m.power_w = None;
                m.swr = None;
            }
        }
        if ptt == Some(true) {
            return true;
        }
        // in ricezione: frequenza, modo e (se c'e') S-meter
        match self.ask("FA;", "FA") {
            Ok(reply) => {
                if let Some(hz) = parse_frequency(&reply) {
                    self.update(|s| s.frequency_hz = Some(hz));
                }
            }
            Err(_) => return false,
        }
        if let Ok(reply) = self.ask("MD0;", "MD0")
            && let Some(code) = parse_mode(&reply)
        {
            self.update(|s| s.mode = mode_name(code).map(str::to_string));
        }
        self.poll_smeter();
        true
    }

    fn poll_smeter(&self) {
        if self.smeter.load(Ordering::SeqCst) == SMETER_NO {
            return;
        }
        match self.ask("SM0;", "SM0") {
            Ok(reply) => {
                if let Some(raw) = parse_smeter(&reply) {
                    self.smeter.store(SMETER_OK, Ordering::SeqCst);
                    self.smeter_silent.store(0, Ordering::SeqCst);
                    if let Ok(mut m) = self.meters.lock() {
                        meters::store(&mut m, Level::Strength, smeter_db(raw));
                    }
                }
            }
            Err(AskError::Rejected) => self.smeter.store(SMETER_NO, Ordering::SeqCst),
            Err(_) => {
                // un programma che non conosce `SM0;` tace: dopo qualche silenzio non si chiede piu' (prima di averlo mai letto)
                let silent = self.smeter_silent.fetch_add(1, Ordering::SeqCst) + 1;
                if self.smeter.load(Ordering::SeqCst) == SMETER_UNKNOWN && silent >= SMETER_MAX_SILENT {
                    self.smeter.store(SMETER_NO, Ordering::SeqCst);
                    tracing::info!("FT-991(A) Link: S-meter (SM0;) non disponibile");
                }
            }
        }
    }

    fn sleep(&self, wait: Duration) {
        let until = Instant::now() + wait;
        while Instant::now() < until && !self.stop.load(Ordering::SeqCst) {
            std::thread::sleep(Duration::from_millis(10).min(wait));
        }
    }

    fn set_link(&self, link: Option<Link>) {
        if let Ok(mut l) = self.link.lock() {
            *l = link;
        }
    }

    /// Controlla che il programma risponda e la radio sia accesa (con «Accendi» la accende). `Err` = il messaggio per l'utente.
    fn handshake(&self, powered: &mut bool) -> Result<(), String> {
        let power = match self.ask("PS;", "PS") {
            Ok(reply) => parse_power(&reply),
            Err(AskError::Io(e)) => return Err(format!("{} ({e})", not_answering(&self.address))),
            Err(_) => return Err(not_answering(&self.address)),
        };
        if power == Some(false) {
            if !self.power_on || *powered {
                return Err(RADIO_OFF.into());
            }
            // «Accendi», una volta per sessione: il primo comando sveglia la CPU della radio, il secondo la accende (manuale Yaesu)
            *powered = true;
            tracing::info!("FT-991(A) Link: accensione della radio («Accendi»)");
            let _ = self.send(&cmd_power(true));
            self.sleep(Duration::from_millis(300).min(self.timing.power_wait));
            let _ = self.send(&cmd_power(true));
            self.sleep(self.timing.power_wait);
            match self.ask("PS;", "PS").ok().and_then(|r| parse_power(&r)) {
                Some(true) => {}
                _ => return Err(RADIO_OFF.into()),
            }
        }
        match self.ask("ID;", "ID").ok().as_deref().and_then(parse_id) {
            Some("") => tracing::warn!("FT-991(A) Link: la radio collegata non sembra una FT-991/FT-991A"),
            Some(model) => tracing::info!("FT-991(A) Link: collegata la {model}"),
            None => {}
        }
        Ok(())
    }
}

/// Una FT-991/FT-991A comandata attraverso FT-991(A) Link (UDP).
pub struct Ft991Rig {
    inner: Arc<Inner>,
    worker: Mutex<Option<JoinHandle<()>>>,
}

impl Ft991Rig {
    /// Dalla configurazione della radio: indirizzo, modo CAT, split, «Accendi» e «Spegni».
    pub fn from_config(config: &RigConfig) -> Ft991Rig {
        Ft991Rig::with_timing(config, Ft991Timing::default())
    }

    pub fn with_timing(config: &RigConfig, timing: Ft991Timing) -> Ft991Rig {
        let address = config.ft991_address.trim();
        let address = if address.is_empty() { DEFAULT_FT991_ADDRESS.to_string() } else { address.to_string() };
        let inner = Arc::new(Inner {
            address,
            cat_mode: config.cat_mode,
            split: config.split != SplitMode::None,
            power_on: config.power_on,
            power_off: config.power_off,
            timing,
            stop: AtomicBool::new(false),
            ready: AtomicBool::new(false),
            link: Mutex::new(None),
            tx_audio_hz: AtomicU32::new(0),
            quiet_until: Mutex::new(None),
            emulate_restore: AtomicU64::new(0),
            smeter: AtomicU32::new(SMETER_UNKNOWN),
            smeter_silent: AtomicU32::new(0),
            meters: Mutex::new(Meters::default()),
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
        });
        let worker_inner = Arc::clone(&inner);
        let worker = std::thread::Builder::new().name("ft991link".into()).spawn(move || run(&worker_inner)).ok();
        Ft991Rig { inner, worker: Mutex::new(worker) }
    }

    pub fn address(&self) -> &str {
        &self.inner.address
    }

    pub fn status(&self) -> RigStatus {
        self.inner.status()
    }

    pub fn transmitting(&self) -> bool {
        self.status().ptt
    }

    /// Il PTT via CAT (`TX1;` / `TX0;`). Con lo split simulato il VFO A va prima sulla frequenza di trasmissione e torna dopo.
    pub fn set_ptt(&self, on: bool) -> Result<(), RigError> {
        let inner = &self.inner;
        inner.connected()?;
        if on && inner.split {
            let audio = inner.tx_audio_hz.load(Ordering::SeqCst);
            if let Some(dial) = self.status().frequency_hz
                && audio != 0
                && inner.emulate_restore.load(Ordering::SeqCst) == 0
            {
                let tx = split_tx_dial_hz(dial, audio);
                if tx != dial && tx <= MAX_FA_HZ {
                    inner.write_frequency(tx)?;
                    inner.emulate_restore.store(dial, Ordering::SeqCst);
                }
            }
        }
        inner.send(&cmd_ptt(on))?;
        inner.update(|s| s.ptt = on);
        if !on {
            let back = inner.emulate_restore.swap(0, Ordering::SeqCst);
            if back != 0 {
                inner.write_frequency(back)?;
            }
            if let Ok(mut m) = inner.meters.lock() {
                m.power_w = None;
                m.swr = None;
            }
        }
        Ok(())
    }

    /// La frequenza audio con cui si trasmette (Hz): serve allo split simulato al prossimo PTT.
    pub fn set_tx_audio_frequency(&self, hz: u32) -> Result<(), RigError> {
        if hz > 5000 {
            return Err(RigError(format!("frequenza audio di trasmissione non valida: {hz} Hz")));
        }
        self.inner.tx_audio_hz.store(hz, Ordering::SeqCst);
        // a trasmissione in corso il VFO A va subito sulla nuova frequenza di trasmissione (`EmulateSplitTransceiver` di WSJT-X)
        if self.inner.split && hz != 0 && self.transmitting() {
            let restore = self.inner.emulate_restore.load(Ordering::SeqCst);
            let dial = if restore != 0 { Some(restore) } else { self.status().frequency_hz };
            if let Some(dial) = dial {
                let tx = split_tx_dial_hz(dial, hz);
                if tx <= MAX_FA_HZ {
                    self.inner.write_frequency(tx)?;
                    self.inner.emulate_restore.store(if tx != dial { dial } else { 0 }, Ordering::SeqCst);
                }
            }
        }
        Ok(())
    }

    /// «Spegni»: alla chiusura del programma la radio si spegne con `PS0;`.
    pub fn power_off_if_chosen(&self) {
        if self.inner.power_off && self.inner.ready.load(Ordering::SeqCst) {
            if self.transmitting() {
                let _ = self.set_ptt(false);
            }
            if self.inner.send(&cmd_power(false)).is_ok() {
                tracing::info!("FT-991 spenta via FT-991(A) Link alla chiusura («Spegni»)");
            }
        }
    }

    /// Chiude: PTT spento (e frequenza rimessa se lo split simulato l'aveva spostata), collegamento chiuso.
    pub fn shutdown(&self) {
        if self.inner.ready.load(Ordering::SeqCst) && (self.transmitting() || self.inner.emulate_restore.load(Ordering::SeqCst) != 0) {
            let _ = self.set_ptt(false);
        }
        self.inner.stop.store(true, Ordering::SeqCst);
        if let Some(handle) = self.worker.lock().ok().and_then(|mut w| w.take()) {
            let _ = handle.join();
        }
        self.inner.ready.store(false, Ordering::SeqCst);
        self.inner.set_link(None);
        self.inner.update(|s| {
            s.state = LinkState::Disconnected;
            s.ptt = false;
        });
    }
}

impl Drop for Ft991Rig {
    fn drop(&mut self) {
        self.shutdown();
    }
}

fn run(inner: &Inner) {
    let mut attempt = 0u32;
    // «Accendi» si prova una volta sola (al primo collegamento dopo la scelta della radio)
    let mut powered = false;
    while !inner.stop.load(Ordering::SeqCst) {
        let opened = Link::open(&inner.address).map_err(|e| format!("{} ({e})", not_answering(&inner.address)));
        let checked = opened.and_then(|link| {
            inner.set_link(Some(link));
            inner.handshake(&mut powered)
        });
        match checked {
            Ok(()) => {
                attempt = 0;
                inner.ready.store(true, Ordering::SeqCst);
                inner.update(|s| {
                    s.state = LinkState::Connected;
                    s.message.clear();
                    s.reconnect_attempt = 0;
                    s.failures = 0;
                    s.split = None;
                });
                serve(inner);
                inner.ready.store(false, Ordering::SeqCst);
                inner.set_link(None);
                if inner.stop.load(Ordering::SeqCst) {
                    break;
                }
                tracing::warn!("FT-991(A) Link: collegamento perso, nuovo tentativo");
                inner.update(|s| {
                    s.state = LinkState::Reconnecting;
                    s.message = LINK_LOST.into();
                    s.ptt = false;
                });
            }
            Err(message) => {
                inner.set_link(None);
                if inner.stop.load(Ordering::SeqCst) {
                    break;
                }
                attempt += 1;
                inner.update(|s| {
                    s.state = if attempt == 1 { LinkState::Error } else { LinkState::Reconnecting };
                    s.message = message;
                    s.reconnect_attempt = attempt;
                    s.failures = attempt;
                });
            }
        }
        // attesa prima di riprovare, crescente fino al tetto, interrotta subito dalla chiusura
        let wait = (inner.timing.retry_step * attempt.clamp(1, 10)).min(inner.timing.retry_cap);
        inner.sleep(wait);
    }
}

/// Le letture periodiche di un collegamento aperto. Torna quando il programma smette di rispondere o quando si chiude.
fn serve(inner: &Inner) {
    let mut failures = 0u32;
    while !inner.stop.load(Ordering::SeqCst) {
        if inner.poll() {
            if failures != 0 {
                inner.update(|s| s.failures = 0);
            }
            failures = 0;
        } else if inner.quiet_until.lock().ok().and_then(|q| *q).is_some_and(|t| std::time::Instant::now() < t) {
            // cambio di banda in corso: la lettura mancata non conta
        } else {
            failures += 1;
            inner.update(|s| s.failures = failures);
            if failures >= inner.timing.tolerate {
                return;
            }
        }
        let wait = if inner.status().ptt { inner.timing.poll_tx } else { inner.timing.poll };
        inner.sleep(wait);
    }
}

impl Rig for Ft991Rig {
    /// Cambio di frequenza del VFO A (non in trasmissione) e, se scelto, il modo (USB o DATA-USB).
    fn set_frequency(&self, hz: u64) -> Result<(), RigError> {
        if !valid_dial_hz(hz) || hz > MAX_FA_HZ {
            return Err(RigError(format!("frequenza non valida: {hz} Hz")));
        }
        self.inner.connected()?;
        if self.transmitting() {
            return Err(RigError("la radio sta trasmettendo: cambio di frequenza rifiutato".into()));
        }
        if let Ok(mut q) = self.inner.quiet_until.lock() {
            *q = Some(std::time::Instant::now() + Duration::from_secs(3));
        }
        let band_before = self.status().frequency_hz.map(|f| f / 1_000_000);
        self.inner.write_frequency(hz)?;
        if let Some(code) = mode_code(self.inner.cat_mode) {
            let wanted = mode_name(code);
            // cambiando banda l'FT-991 riprende il modo salvato per quella banda (spesso FM): il modo si rimanda sempre, dopo
            // una breve attesa perche' la radio finisca la commutazione (prima, con il modo letto ancora della banda vecchia, non partiva)
            let band_changed = band_before != Some(hz / 1_000_000);
            if band_changed {
                std::thread::sleep(Duration::from_millis(300));
            }
            if band_changed || self.status().mode.as_deref() != wanted {
                self.inner.send(&cmd_mode(code))?;
                self.inner.update(|s| s.mode = wanted.map(str::to_string));
            }
        }
        Ok(())
    }

    fn frequency(&self) -> Result<u64, RigError> {
        let s = self.status();
        match (s.state, s.frequency_hz) {
            (LinkState::Connected, Some(hz)) => Ok(hz),
            _ => Err(RigError("radio FT-991(A) Link non collegata".into())),
        }
    }

    /// Lo S-meter (`SM0;`), se FT-991(A) Link lo da'.
    fn meters(&self) -> Result<Meters, RigError> {
        self.inner.connected()?;
        if self.inner.smeter.load(Ordering::SeqCst) != SMETER_OK {
            return Err(RigError("misuratori non disponibili con FT-991(A) Link".into()));
        }
        Ok(self.inner.meters.lock().map(|m| *m).unwrap_or_default())
    }

    fn set_rx_frequency(&self, _hz: u32) -> Result<(), RigError> {
        Err(RigError("comando del motore non supportato da FT-991(A) Link".into()))
    }

    fn output_attenuation(&self) -> Result<u32, RigError> {
        Err(RigError("comando del motore non supportato da FT-991(A) Link".into()))
    }

    fn set_output_attenuation(&self, _tenths: u32) -> Result<(), RigError> {
        Err(RigError("comando del motore non supportato da FT-991(A) Link".into()))
    }

    fn set_mode(&self, _mode: &str) -> Result<(), RigError> {
        Err(RigError("comando del motore non supportato da FT-991(A) Link".into()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_reads_the_yaesu_replies() {
        assert_eq!(replies("FA014074000;"), vec!["FA014074000"]);
        assert_eq!(replies(" FA014074000; MD0C;;"), vec!["FA014074000", "MD0C"]);
        assert_eq!(parse_frequency("FA145525000"), Some(145_525_000));
        assert_eq!(parse_frequency("FA014074000"), Some(14_074_000));
        assert_eq!(parse_frequency("FA14074000"), None, "servono 9 cifre");
        assert_eq!(parse_frequency("FA01407400X"), None);
        assert_eq!(parse_mode("MD0C"), Some('C'));
        assert_eq!(parse_mode("MD02"), Some('2'));
        assert_eq!(parse_mode("MD0Z"), None);
        assert_eq!(parse_mode("MD1C"), None);
        assert_eq!(mode_name('C'), Some("DATA-USB"));
        assert_eq!(mode_name('e'), Some("C4FM"));
        assert_eq!(parse_tx("TX0"), Some(false));
        assert_eq!(parse_tx("TX1"), Some(true));
        assert_eq!(parse_tx("TX2"), Some(true), "PTT premuto sulla radio");
        assert_eq!(parse_tx("TX"), None);
        assert_eq!(parse_power("PS1"), Some(true));
        assert_eq!(parse_power("PS0"), Some(false));
        assert_eq!(parse_id("ID0570"), Some("FT-991"));
        assert_eq!(parse_id("ID0670"), Some("FT-991A"));
        assert_eq!(parse_id("ID1"), Some(""));
        assert_eq!(parse_smeter("SM0130"), Some(130));
        assert_eq!(parse_smeter("SM0999"), None);
    }

    #[test]
    fn it_builds_the_commands() {
        assert_eq!(cmd_frequency(14_074_000), "FA014074000;");
        assert_eq!(cmd_frequency(145_525_000), "FA145525000;");
        assert_eq!(cmd_mode('C'), "MD0C;");
        assert_eq!((cmd_ptt(true), cmd_ptt(false)), ("TX1;".to_string(), "TX0;".to_string()));
        assert_eq!((cmd_power(true), cmd_power(false)), ("PS1;".to_string(), "PS0;".to_string()));
        assert_eq!(mode_code(CatMode::Data), Some('C'));
        assert_eq!(mode_code(CatMode::Usb), Some('2'));
        assert_eq!(mode_code(CatMode::None), None);
    }

    #[test]
    fn the_smeter_follows_the_s9_scale() {
        assert_eq!(smeter_db(0), -54.0);
        assert_eq!(smeter_db(130), 0.0);
        assert_eq!(smeter_db(255), 60.0);
        assert_eq!(smeter_db(400), 60.0);
        let mid = smeter_db(121);
        assert!(mid > -6.0 && mid < 0.0, "{mid}");
    }

    /// Un finto FT-991(A) Link: risponde come il programma vero e registra i comandi ricevuti.
    struct FakeLink {
        address: String,
        seen: Arc<Mutex<Vec<String>>>,
        state: Arc<Mutex<FakeRadio>>,
        stop: Arc<AtomicBool>,
    }

    #[derive(Clone)]
    struct FakeRadio {
        on: bool,
        hz: u64,
        mode: char,
        tx: bool,
        /// `None` = il programma non conosce `SM0;` (tace).
        smeter: Option<u32>,
        /// Il programma non risponde a niente (COM Port chiusa, UDP fermo).
        mute: bool,
    }

    impl FakeLink {
        fn start(radio: FakeRadio) -> FakeLink {
            let socket = UdpSocket::bind("127.0.0.1:0").expect("porta");
            socket.set_read_timeout(Some(Duration::from_millis(20))).expect("timeout");
            let address = socket.local_addr().expect("indirizzo").to_string();
            let seen = Arc::new(Mutex::new(Vec::new()));
            let state = Arc::new(Mutex::new(radio));
            let stop = Arc::new(AtomicBool::new(false));
            let (log, radio, halt) = (Arc::clone(&seen), Arc::clone(&state), Arc::clone(&stop));
            std::thread::spawn(move || {
                let mut buf = [0u8; 512];
                while !halt.load(Ordering::SeqCst) {
                    let Ok((n, from)) = socket.recv_from(&mut buf) else { continue };
                    let command = String::from_utf8_lossy(&buf[..n]).to_string();
                    log.lock().unwrap().push(command.clone());
                    let mut r = radio.lock().unwrap();
                    if r.mute {
                        continue;
                    }
                    let body = command.trim_end_matches(';');
                    let reply = match body {
                        "PS" => Some(format!("PS{};", u8::from(r.on))),
                        "PS1" => {
                            r.on = true;
                            None
                        }
                        "PS0" => {
                            r.on = false;
                            None
                        }
                        _ if !r.on => None,
                        "ID" => Some("ID0670;".to_string()),
                        "FA" => Some(format!("FA{:09};", r.hz)),
                        "MD0" => Some(format!("MD0{};", r.mode)),
                        "TX" => Some(format!("TX{};", u8::from(r.tx))),
                        "TX1" => {
                            r.tx = true;
                            None
                        }
                        "TX0" => {
                            r.tx = false;
                            None
                        }
                        "SM0" => r.smeter.map(|v| format!("SM0{v:03};")),
                        b if b.starts_with("FA") => {
                            r.hz = b[2..].parse().unwrap_or(r.hz);
                            Some(format!("FA{:09};", r.hz))
                        }
                        b if b.starts_with("MD0") => {
                            r.mode = b.chars().nth(3).unwrap_or(r.mode);
                            None
                        }
                        _ => Some("?;".to_string()),
                    };
                    if let Some(reply) = reply {
                        let _ = socket.send_to(reply.as_bytes(), from);
                    }
                }
            });
            FakeLink { address, seen, state, stop }
        }

        fn radio(&self) -> FakeRadio {
            self.state.lock().unwrap().clone()
        }

        fn edit(&self, f: impl FnOnce(&mut FakeRadio)) {
            f(&mut self.state.lock().unwrap());
        }

        /// I comandi ricevuti, senza le letture periodiche.
        fn writes(&self) -> Vec<String> {
            let reads = ["PS;", "ID;", "FA;", "MD0;", "TX;", "SM0;"];
            self.seen.lock().unwrap().iter().filter(|c| !reads.contains(&c.as_str())).cloned().collect()
        }
    }

    impl Drop for FakeLink {
        fn drop(&mut self) {
            self.stop.store(true, Ordering::SeqCst);
        }
    }

    fn radio() -> FakeRadio {
        FakeRadio { on: true, hz: 7_074_000, mode: '2', tx: false, smeter: Some(130), mute: false }
    }

    fn fast() -> Ft991Timing {
        Ft991Timing {
            poll: Duration::from_millis(30),
            poll_tx: Duration::from_millis(20),
            reply: Duration::from_millis(100),
            attempts: 2,
            tolerate: 3,
            retry_step: Duration::from_millis(30),
            retry_cap: Duration::from_millis(100),
            power_wait: Duration::from_millis(50),
        }
    }

    fn config(address: &str) -> RigConfig {
        RigConfig { ft991_address: address.to_string(), ..RigConfig::default() }
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
    fn it_connects_reads_and_drives_the_radio() {
        let fake = FakeLink::start(radio());
        let rig = Ft991Rig::with_timing(&RigConfig { cat_mode: CatMode::Data, ..config(&fake.address) }, fast());
        assert!(wait(|| rig.status().state == LinkState::Connected), "{:?}", rig.status());
        assert!(wait(|| rig.frequency() == Ok(7_074_000)));
        assert!(wait(|| rig.status().mode.as_deref() == Some("USB")));
        rig.set_frequency(14_074_000).expect("frequenza");
        assert_eq!(fake.radio().hz, 14_074_000);
        assert!(wait(|| fake.radio().mode == 'C'), "DATA-USB per FT8");
        rig.set_ptt(true).expect("PTT");
        assert!(wait(|| fake.radio().tx));
        assert!(rig.transmitting());
        assert!(rig.set_frequency(7_074_000).is_err(), "niente cambio di frequenza in trasmissione");
        rig.set_ptt(false).expect("PTT");
        assert!(wait(|| !fake.radio().tx));
        assert_eq!(fake.writes(), vec!["FA014074000;", "MD0C;", "TX1;", "TX0;"]);
        rig.shutdown();
        assert_eq!(rig.status().state, LinkState::Disconnected);
    }

    #[test]
    fn it_follows_the_ptt_pressed_on_the_radio() {
        let fake = FakeLink::start(radio());
        let rig = Ft991Rig::with_timing(&config(&fake.address), fast());
        assert!(wait(|| rig.status().state == LinkState::Connected));
        fake.edit(|r| r.tx = true);
        assert!(wait(|| rig.transmitting()));
        fake.edit(|r| r.tx = false);
        assert!(wait(|| !rig.transmitting()));
    }

    #[test]
    fn emulated_split_moves_vfo_a_only_while_transmitting() {
        let fake = FakeLink::start(FakeRadio { hz: 14_074_000, ..radio() });
        let rig = Ft991Rig::with_timing(&RigConfig { split: SplitMode::Emulate, ..config(&fake.address) }, fast());
        assert!(wait(|| rig.frequency() == Ok(14_074_000)));
        rig.set_tx_audio_frequency(300).expect("frequenza audio");
        rig.set_ptt(true).expect("PTT");
        assert_eq!(fake.radio().hz, 14_072_500, "VFO A sulla frequenza di trasmissione prima del PTT");
        rig.set_ptt(false).expect("PTT");
        assert_eq!(fake.radio().hz, 14_074_000, "a fine trasmissione torna");
        assert_eq!(fake.writes(), vec!["FA014072500;", "TX1;", "TX0;", "FA014074000;"]);
        assert!(rig.set_tx_audio_frequency(9000).is_err());
    }

    #[test]
    fn the_rig_split_is_simulated_too_with_only_vfo_a() {
        let fake = FakeLink::start(FakeRadio { hz: 14_074_000, ..radio() });
        let rig = Ft991Rig::with_timing(&RigConfig { split: SplitMode::Rig, ..config(&fake.address) }, fast());
        assert!(wait(|| rig.frequency() == Ok(14_074_000)));
        rig.set_tx_audio_frequency(2200).expect("frequenza audio");
        rig.set_ptt(true).expect("PTT");
        rig.set_ptt(false).expect("PTT");
        assert_eq!(fake.writes(), vec!["FA014074500;", "TX1;", "TX0;", "FA014074000;"]);
    }

    #[test]
    fn the_smeter_is_read_when_available_and_ignored_when_not() {
        let fake = FakeLink::start(radio());
        let rig = Ft991Rig::with_timing(&config(&fake.address), fast());
        assert!(wait(|| rig.meters().is_ok()));
        let m = rig.meters().expect("misuratori");
        assert_eq!((m.s_points, m.s_over_db), (Some(9.0), Some(0.0)));

        let silent = FakeLink::start(FakeRadio { smeter: None, ..radio() });
        let rig = Ft991Rig::with_timing(&config(&silent.address), fast());
        assert!(wait(|| rig.status().state == LinkState::Connected));
        let asked = || silent.seen.lock().unwrap().iter().filter(|c| *c == "SM0;").count();
        assert!(wait(|| asked() >= SMETER_MAX_SILENT as usize * 2));
        std::thread::sleep(Duration::from_millis(300));
        let after = asked();
        std::thread::sleep(Duration::from_millis(300));
        assert_eq!(asked(), after, "dopo qualche silenzio SM0; non si chiede piu'");
        assert!(rig.meters().is_err());
        assert_eq!(rig.status().state, LinkState::Connected, "lo S-meter mancante non fa cadere il collegamento");
    }

    #[test]
    fn it_reports_a_silent_program_and_reconnects_when_it_answers() {
        let fake = FakeLink::start(FakeRadio { mute: true, ..radio() });
        let rig = Ft991Rig::with_timing(&config(&fake.address), fast());
        assert!(wait(|| matches!(rig.status().state, LinkState::Error | LinkState::Reconnecting)));
        let message = rig.status().message;
        assert_eq!(message, not_answering(&fake.address));
        assert!(message.contains("Start UDP"));
        assert!(rig.set_frequency(14_074_000).is_err());
        assert!(rig.set_ptt(true).is_err());
        fake.edit(|r| r.mute = false);
        assert!(wait(|| rig.status().state == LinkState::Connected), "si ricollega da solo");
        // il programma smette di rispondere: si ricollega
        fake.edit(|r| r.mute = true);
        assert!(wait(|| rig.status().state != LinkState::Connected));
        fake.edit(|r| r.mute = false);
        assert!(wait(|| rig.status().state == LinkState::Connected));
    }

    #[test]
    fn a_closed_port_is_reported_too() {
        let socket = UdpSocket::bind("127.0.0.1:0").expect("porta");
        let address = socket.local_addr().expect("indirizzo").to_string();
        drop(socket);
        let rig = Ft991Rig::with_timing(&config(&address), fast());
        assert!(wait(|| matches!(rig.status().state, LinkState::Error | LinkState::Reconnecting)));
        assert!(rig.status().message.starts_with(&not_answering(&address)), "{}", rig.status().message);
    }

    #[test]
    fn a_radio_that_is_off_is_reported_or_switched_on() {
        let fake = FakeLink::start(FakeRadio { on: false, ..radio() });
        let rig = Ft991Rig::with_timing(&config(&fake.address), fast());
        assert!(wait(|| rig.status().message == RADIO_OFF));
        assert_ne!(rig.status().state, LinkState::Connected);
        rig.shutdown();

        let fake = FakeLink::start(FakeRadio { on: false, ..radio() });
        let rig = Ft991Rig::with_timing(&RigConfig { power_on: true, power_off: true, ..config(&fake.address) }, fast());
        assert!(wait(|| rig.status().state == LinkState::Connected), "{:?}", rig.status());
        assert!(fake.radio().on);
        rig.power_off_if_chosen();
        assert!(wait(|| !fake.radio().on), "«Spegni» alla chiusura");
    }
}
