//! Ham Radio Deluxe (HRD Rig Control): il programma tiene la radio e offre un server TCP (di base `127.0.0.1:7809`) a cui si mandano
//! comandi testuali, uno alla volta, con una risposta per comando.
//!
//! Due dialetti:
//! - **v5** (anche le versioni piu' nuove): ogni messaggio ha un'intestazione di 16 byte little-endian (lunghezza totale, due numeri
//!   magici `0x1234ABCD` e `0xABCD1234`, uno zero) seguita dal testo in UTF-16LE chiuso da un carattere NUL. Le risposte hanno lo
//!   stesso formato; del testo vale solo la parte prima del primo NUL.
//! - **v4**: testo semplice chiuso da CR; la risposta arriva in una lettura sola, senza intestazione.
//!
//! Si prova prima v5 con `get context`; se non risponde si chiude, si riapre e si prova v4 (una v4 non risponde ai messaggi binari).
//!
//! Comandi usati (quelli operativi preceduti da `[N] `, N = indice della radio in HRD):
//! - avvio: `get radios` (`1:IC-7300,2:FT-991`), `get radio`, `[N] get vfo-count`, `[N] get buttons`, `[N] get dropdowns`,
//!   `[N] get dropdown-list {Nome}` (solo per il modo e per `Data`);
//! - frequenza: `get frequency` (Hz), `get frequencies` (`A-B`, radio a due VFO), `set frequency-hz <Hz>`, `set frequencies-hz <A> <B>`;
//! - modo: `get dropdown-text {Mode}` (`Mode: USB`), `set dropdown <Nome> <Voce> <indice>` (spazi scritti come `~`);
//! - pulsanti (PTT `TX`, VFO A/B, split, dati): `set button-select <Nome> 1|0`, `get button-select <Nome>`.
//!
//! Ogni `set` deve rispondere `OK`. Radio a un solo VFO: 50 ms di pausa prima di ogni comando (alcune radio altrimenti li perdono).
//!
//! Lo split di FT8 e' quello simulato: prima del PTT il VFO A va sulla frequenza di trasmissione e torna a fine trasmissione (la stessa
//! regola degli altri collegamenti, `split_tx_dial_hz`). Lo split DX usa i comandi di split della radio, se HRD li offre.
//!
//! Il collegamento vive in un thread: lettura periodica di frequenza e modo e ricollegamento automatico quando HRD smette di rispondere.

use std::{
    io::{ErrorKind, Read, Write},
    net::{TcpStream, ToSocketAddrs},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering},
    },
    thread::JoinHandle,
    time::{Duration, Instant},
};

use crate::{
    Rig, RigError,
    cat_config::{CatMode, RigConfig, SplitMode, split_tx_dial_hz},
    hamlib_rig::{LinkState, RigStatus, valid_dial_hz},
    meters::Meters,
};

/// Host di base del server di HRD.
pub const DEFAULT_HRD_HOST: &str = "127.0.0.1";
/// Porta TCP di base del server di HRD.
pub const DEFAULT_HRD_PORT: u16 = 7809;

/// Primo numero magico dell'intestazione v5.
const MAGIC1: u32 = 0x1234_ABCD;
/// Secondo numero magico dell'intestazione v5.
const MAGIC2: u32 = 0xABCD_1234;
/// Dimensione dell'intestazione v5.
const HEADER: usize = 16;
/// Messaggio v5 piu' grande accettato.
const MAX_MESSAGE: usize = 1 << 20;

/// Il messaggio quando HRD non risponde (anche tradotto dall'interfaccia).
pub fn not_answering(address: &str) -> String {
    format!("Ham Radio Deluxe non risponde su {address}: apri HRD Rig Control e collega la radio")
}

/// Il messaggio mentre si ricollega dopo un guasto.
pub const LINK_LOST: &str = "Collegamento con Ham Radio Deluxe perso: nuovo tentativo";

/// Il messaggio quando HRD non offre il pulsante del PTT.
pub const NO_TX_BUTTON: &str = "HRD non ha un pulsante TX: usa un altro PTT";

// ---------------------------------------------------------------- formato dei messaggi

/// Un messaggio v5: intestazione di 16 byte e testo UTF-16LE chiuso da NUL.
pub fn encode_v5(text: &str) -> Vec<u8> {
    let units: Vec<u16> = text.encode_utf16().chain(std::iter::once(0)).collect();
    let len = HEADER + units.len() * 2;
    let mut out = Vec::with_capacity(len);
    out.extend_from_slice(&(len as u32).to_le_bytes());
    out.extend_from_slice(&MAGIC1.to_le_bytes());
    out.extend_from_slice(&MAGIC2.to_le_bytes());
    out.extend_from_slice(&0u32.to_le_bytes());
    for u in units {
        out.extend_from_slice(&u.to_le_bytes());
    }
    out
}

fn le_u32(bytes: &[u8], at: usize) -> u32 {
    let mut b = [0u8; 4];
    b.copy_from_slice(&bytes[at..at + 4]);
    u32::from_le_bytes(b)
}

/// Legge un messaggio v5 dai byte ricevuti: `Ok(None)` se ne mancano ancora, `Err` se il messaggio non e' valido. Il testo e' quello
/// prima del primo NUL.
pub fn decode_v5(buf: &[u8]) -> Result<Option<String>, String> {
    if buf.len() < HEADER {
        return Ok(None);
    }
    let size = le_u32(buf, 0) as usize;
    if le_u32(buf, 4) != MAGIC1 || le_u32(buf, 8) != MAGIC2 {
        return Err("numeri magici non validi".into());
    }
    if !(HEADER..=MAX_MESSAGE).contains(&size) {
        return Err(format!("lunghezza del messaggio non valida ({size} byte)"));
    }
    if buf.len() < size {
        return Ok(None);
    }
    let payload = &buf[HEADER..size];
    if payload.len() < 2 || !payload.len().is_multiple_of(2) || payload[payload.len() - 2..] != [0, 0] {
        return Err("testo del messaggio non valido".into());
    }
    let units: Vec<u16> = payload.as_chunks::<2>().0.iter().map(|c| u16::from_le_bytes(*c)).take_while(|u| *u != 0).collect();
    Ok(Some(String::from_utf16_lossy(&units)))
}

/// Le voci di un elenco separato da virgole, senza spazi ai lati; quelle vuote restano (contano nell'indice).
pub fn split_list(text: &str) -> Vec<String> {
    if text.trim().is_empty() {
        return Vec::new();
    }
    text.split(',').map(|s| s.trim().to_string()).collect()
}

/// `1:IC-7300,2:FT-991` -> `[(1, "IC-7300"), (2, "FT-991")]` (le voci senza indice valido si saltano).
pub fn parse_radios(text: &str) -> Vec<(u32, String)> {
    split_list(text)
        .into_iter()
        .filter_map(|entry| {
            let mut parts = entry.splitn(2, ':');
            let index = parts.next()?.trim().parse().ok()?;
            Some((index, parts.next().unwrap_or("").trim().to_string()))
        })
        .collect()
}

/// `14074000-14075000` -> le due frequenze; `14074000` da sola vale per entrambe.
pub fn parse_frequencies(text: &str) -> Option<(u64, u64)> {
    let mut parts = text.trim().split('-');
    let a = parts.next()?.trim().parse().ok()?;
    let b = match parts.next() {
        Some(p) => p.trim().parse().ok()?,
        None => a,
    };
    Some((a, b))
}

/// `Mode: USB` -> `USB` (la parte dopo i primi due punti); senza due punti nessun valore.
pub fn dropdown_value(text: &str) -> Option<String> {
    text.split_once(':').map(|(_, v)| v.trim().to_string())
}

/// Nome con gli spazi scritti come `~`, come vogliono i comandi `set`.
fn tilde(text: &str) -> String {
    text.replace(' ', "~")
}

// ---------------------------------------------------------------- modi

/// I modi che Aurora sa chiedere alla radio attraverso il menu del modo.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HrdMode {
    Usb,
    Lsb,
    /// USB dati (quello di FT8).
    DataUsb,
}

/// Le voci del menu del modo che vanno bene per `mode`, in ordine di preferenza (per i dati solo quelle dedicate: USB con il comando
/// dei dati e' l'ultima risorsa, decisa altrove).
pub fn mode_entries(mode: HrdMode) -> &'static [&'static str] {
    match mode {
        HrdMode::Usb => &["USB", "USB(N)"],
        HrdMode::Lsb => &["LSB", "LSB(N)"],
        HrdMode::DataUsb => &["DIG", "DIGU", "DATA-U", "PKT-U", "DATA", "AFSK", "USER-U"],
    }
}

/// La voce del menu per `mode` e la sua posizione (cercando la voce dall'ultima alla prima, perche' le piu' specializzate stanno in fondo).
pub fn pick_entry(list: &[String], mode: HrdMode) -> Option<(String, usize)> {
    mode_entries(mode).iter().find_map(|want| list.iter().rposition(|e| e.eq_ignore_ascii_case(want)).map(|i| (list[i].clone(), i)))
}

/// Il nome del modo (quelli usati nel resto del programma: USB, PKTUSB, RTTY...) per una voce del menu del modo; le voci sconosciute restano come sono.
pub fn mode_name(entry: &str) -> String {
    let up = entry.trim().to_ascii_uppercase();
    let name = match up.as_str() {
        "USB" | "USB(N)" => "USB",
        "LSB" | "LSB(N)" => "LSB",
        "CW" | "CW(N)" | "CW-LSB" | "CWL" => "CW",
        "CW-R" | "CW-R(N)" | "CW-USB" | "CWU" => "CWR",
        "DIG" | "DIGU" | "DATA-U" | "PKT-U" | "DATA" | "AFSK" | "USER-U" => "PKTUSB",
        "DIGL" | "DATA-L" | "PKT-L" | "DATA-R" | "USER-L" => "PKTLSB",
        "FSK" | "RTTY" | "RTTY-LSB" => "RTTY",
        "FSK-R" | "RTTY-R" | "RTTY-USB" => "RTTYR",
        "AM" | "DSB" | "SAM" | "DRM" => "AM",
        "FM" | "FM(N)" | "FM-N" | "WFM" => "FM",
        "PKT-FM" | "PKT" | "DATA(FM)" => "PKTFM",
        _ => return entry.trim().to_string(),
    };
    name.to_string()
}

/// Con il comando dei dati acceso USB, LSB e FM diventano i modi dati.
fn with_data(name: String, data: bool) -> String {
    if !data {
        return name;
    }
    match name.as_str() {
        "USB" => "PKTUSB".into(),
        "LSB" => "PKTLSB".into(),
        "FM" => "PKTFM".into(),
        _ => name,
    }
}

/// Come si accende il modo dati, se la radio ha un comando a parte.
#[derive(Debug, Clone, PartialEq, Eq)]
enum DataControl {
    /// Pulsante `Data` (solo sulle Icom: sulle Kenwood lo stesso nome e' il DSP).
    IcomButton,
    /// Coppia di pulsanti accendi / spegni.
    Buttons { on: String, off: String },
    /// Menu `Data` con una voce accesa e una spenta (voce e posizione).
    Dropdown { on: (String, usize), off: (String, usize) },
}

// ---------------------------------------------------------------- collegamento TCP

/// Le attese del collegamento.
#[derive(Debug, Clone)]
pub struct HrdTiming {
    /// Intervallo di lettura in ricezione.
    pub poll: Duration,
    /// Intervallo di lettura in trasmissione.
    pub poll_tx: Duration,
    /// Attesa massima per aprire la connessione.
    pub connect: Duration,
    /// Attesa di una lettura dal socket.
    pub read: Duration,
    /// Letture scadute di fila prima di dare il comando per perso.
    pub attempts: u32,
    /// Letture scadute concesse alla prova del dialetto v5 (una v4 tace sempre: non serve aspettare di piu').
    pub probe_attempts: u32,
    /// Letture fallite di fila prima di ricollegarsi.
    pub tolerate: u32,
    /// Attesa tra un tentativo di collegamento e l'altro: cresce di questo passo fino al tetto.
    pub retry_step: Duration,
    pub retry_cap: Duration,
    /// Pausa prima di ogni comando con le radio a un solo VFO.
    pub single_vfo_pause: Duration,
    /// Pausa dopo `RX A` / `RX B` sulla Yaesu FTdx3000.
    pub ftdx3000_pause: Duration,
}

impl Default for HrdTiming {
    fn default() -> Self {
        HrdTiming {
            poll: Duration::from_millis(1000),
            poll_tx: Duration::from_millis(500),
            connect: Duration::from_secs(3),
            read: Duration::from_secs(1),
            attempts: 3,
            probe_attempts: 2,
            tolerate: 3,
            retry_step: Duration::from_millis(1000),
            retry_cap: Duration::from_secs(5),
            single_vfo_pause: Duration::from_millis(50),
            ftdx3000_pause: Duration::from_millis(350),
        }
    }
}

/// Il dialetto parlato da HRD.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Dialect {
    V5,
    V4,
}

/// Perche' un comando non e' andato.
#[derive(Debug, Clone, PartialEq, Eq)]
enum HrdError {
    /// Socket chiuso o guasto, nessuna risposta nel tempo, risposta illeggibile: il collegamento non e' piu' affidabile e si riapre.
    Link(String),
    /// HRD ha risposto, ma non come atteso (un `set` senza `OK`, un numero illeggibile): il collegamento resta.
    Reply(String),
}

impl HrdError {
    fn message(&self) -> String {
        match self {
            HrdError::Link(m) | HrdError::Reply(m) => format!("Ham Radio Deluxe: {m}"),
        }
    }
}

fn timed_out(e: &std::io::Error) -> bool {
    matches!(e.kind(), ErrorKind::TimedOut | ErrorKind::WouldBlock)
}

fn open_stream(address: &str, timing: &HrdTiming) -> Result<TcpStream, String> {
    let addrs: Vec<_> = address.to_socket_addrs().map_err(|e| format!("{address}: {e}"))?.collect();
    let mut last = format!("{address}: indirizzo non valido");
    for addr in addrs {
        match TcpStream::connect_timeout(&addr, timing.connect) {
            Ok(stream) => {
                let _ = stream.set_nodelay(true);
                stream.set_read_timeout(Some(timing.read.max(Duration::from_millis(1)))).map_err(|e| format!("{address}: {e}"))?;
                stream.set_write_timeout(Some(timing.connect.max(Duration::from_millis(1)))).map_err(|e| format!("{address}: {e}"))?;
                return Ok(stream);
            }
            Err(e) => last = format!("{address}: {e}"),
        }
    }
    Err(last)
}

/// Una connessione aperta con HRD e cio' che si e' saputo della radio all'avvio.
struct Conn {
    stream: TcpStream,
    dialect: Dialect,
    attempts: u32,
    single_vfo_pause: Duration,
    ftdx3000_pause: Duration,
    /// Indice della radio in HRD (il `[N]` dei comandi).
    radio: u32,
    radio_name: String,
    vfo_count: u32,
    /// Nomi dei pulsanti, con `~` al posto degli spazi.
    buttons: Vec<String>,
    /// Nomi dei menu, con gli spazi.
    dropdowns: Vec<String>,
    /// Il menu del modo (VFO A) e le sue voci.
    mode_dropdown: Option<String>,
    mode_list: Vec<String>,
    data: Option<DataControl>,
}

impl Conn {
    /// Apre la connessione e sceglie il dialetto: v5, altrimenti (chiudendo e riaprendo) v4.
    fn open(address: &str, timing: &HrdTiming) -> Result<Conn, String> {
        let make = |stream: TcpStream, dialect: Dialect, attempts: u32| Conn {
            stream,
            dialect,
            attempts,
            single_vfo_pause: timing.single_vfo_pause,
            ftdx3000_pause: timing.ftdx3000_pause,
            radio: 0,
            radio_name: String::new(),
            vfo_count: 1,
            buttons: Vec::new(),
            dropdowns: Vec::new(),
            mode_dropdown: None,
            mode_list: Vec::new(),
            data: None,
        };
        let mut conn = make(open_stream(address, timing)?, Dialect::V5, timing.probe_attempts.max(1));
        if conn.exchange("get context").is_ok() {
            conn.attempts = timing.attempts.max(1);
            return Ok(conn);
        }
        drop(conn);
        let mut conn = make(open_stream(address, timing)?, Dialect::V4, timing.attempts.max(1));
        conn.exchange("get context").map_err(|e| e.message())?;
        Ok(conn)
    }

    /// Manda un testo e aspetta la risposta (senza contesto della radio).
    fn exchange(&mut self, text: &str) -> Result<String, HrdError> {
        let bytes = match self.dialect {
            Dialect::V5 => encode_v5(text),
            Dialect::V4 => format!("{text}\r").into_bytes(),
        };
        self.stream.write_all(&bytes).and_then(|()| self.stream.flush()).map_err(|e| HrdError::Link(format!("invio di «{text}» non riuscito ({e})")))?;
        let mut buf = Vec::new();
        let mut chunk = [0u8; 4096];
        let mut silent = 0u32;
        loop {
            if self.dialect == Dialect::V5
                && let Some(reply) = decode_v5(&buf).map_err(|e| HrdError::Link(format!("risposta a «{text}» non valida: {e}")))?
            {
                return Ok(reply);
            }
            match self.stream.read(&mut chunk) {
                Ok(0) => return Err(HrdError::Link("HRD ha chiuso la connessione".into())),
                Ok(n) => {
                    buf.extend_from_slice(&chunk[..n]);
                    if self.dialect == Dialect::V4 {
                        // v4: la risposta arriva in una lettura sola
                        return Ok(String::from_utf8_lossy(&buf).trim().to_string());
                    }
                    silent = 0;
                }
                Err(e) if timed_out(&e) => {
                    silent += 1;
                    if silent >= self.attempts {
                        return Err(HrdError::Link(format!("nessuna risposta a «{text}»")));
                    }
                }
                Err(e) if e.kind() == ErrorKind::Interrupted => {}
                Err(e) => return Err(HrdError::Link(format!("lettura della risposta a «{text}» non riuscita ({e})"))),
            }
        }
    }

    /// Un comando per la radio scelta: `[N] testo` (con la pausa delle radio a un solo VFO).
    fn command(&mut self, text: &str) -> Result<String, HrdError> {
        if self.vfo_count <= 1 && !self.single_vfo_pause.is_zero() {
            std::thread::sleep(self.single_vfo_pause);
        }
        let full = format!("[{}] {text}", self.radio);
        self.exchange(&full)
    }

    /// Un comando `set`: la risposta deve essere `OK`.
    fn set(&mut self, text: &str) -> Result<(), HrdError> {
        let reply = self.command(text)?;
        if reply.trim() == "OK" { Ok(()) } else { Err(HrdError::Reply(format!("risposta «{reply}» a «{text}» (atteso OK)"))) }
    }

    /// Le domande dell'avvio: la radio (quella scelta o la prima), i VFO, i pulsanti e i menu.
    fn handshake(&mut self, wanted: &str) -> Result<(), String> {
        let radios = parse_radios(&self.exchange("get radios").map_err(|e| e.message())?);
        if radios.is_empty() {
            return Err("Ham Radio Deluxe non ha nessuna radio collegata: collegala in HRD Rig Control".into());
        }
        let wanted = wanted.trim();
        let chosen = if wanted.is_empty() {
            radios.first().cloned()
        } else {
            radios.iter().find(|(i, name)| name.eq_ignore_ascii_case(wanted) || wanted.parse::<u32>().is_ok_and(|w| w == *i)).cloned()
        };
        let Some((radio, radio_name)) = chosen else {
            let names: Vec<&str> = radios.iter().map(|(_, n)| n.as_str()).collect();
            return Err(format!("la radio «{wanted}» non c'e' in Ham Radio Deluxe (ci sono: {})", names.join(", ")));
        };
        // solo per il registro: la radio che HRD considera attiva
        if let Ok(current) = self.exchange("get radio") {
            tracing::info!("Ham Radio Deluxe: radio attiva «{current}», comandata «{radio_name}» ([{radio}])");
        }
        self.radio = radio;
        self.radio_name = radio_name;
        self.vfo_count = self.command("get vfo-count").map_err(|e| e.message())?.trim().parse().unwrap_or(1).max(1);
        self.buttons = split_list(&self.command("get buttons").map_err(|e| e.message())?).into_iter().filter(|b| !b.is_empty()).map(|b| tilde(&b)).collect();
        self.dropdowns = split_list(&self.command("get dropdowns").map_err(|e| e.message())?).into_iter().filter(|d| !d.is_empty()).collect();
        self.mode_dropdown = ["Main Mode", "Mode", "Mode A"].iter().find_map(|want| self.dropdowns.iter().find(|d| d.as_str() == *want).cloned());
        if let Some(dd) = self.mode_dropdown.clone() {
            self.mode_list = split_list(&self.command(&format!("get dropdown-list {{{dd}}}")).map_err(|e| e.message())?);
        }
        self.data = self.find_data_control().map_err(|e| e.message())?;
        Ok(())
    }

    fn has_button(&self, name: &str) -> bool {
        self.buttons.iter().any(|b| b == name)
    }

    fn first_button(&self, names: &[&str]) -> Option<String> {
        names.iter().find(|n| self.has_button(n)).map(|n| n.to_string())
    }

    fn has_dropdown(&self, name: &str) -> bool {
        self.dropdowns.iter().any(|d| d == name)
    }

    fn find_data_control(&mut self) -> Result<Option<DataControl>, HrdError> {
        if self.radio_name.starts_with("IC") && self.has_button("Data") {
            return Ok(Some(DataControl::IcomButton));
        }
        if self.has_button("DATA-ON(mid)") && self.has_button("DATA-OFF") {
            return Ok(Some(DataControl::Buttons { on: "DATA-ON(mid)".into(), off: "DATA-OFF".into() }));
        }
        if self.has_dropdown("Data") {
            let list = split_list(&self.command("get dropdown-list {Data}")?);
            let find = |names: &[&str]| list.iter().position(|e| names.iter().any(|n| e == n)).map(|i| (list[i].clone(), i));
            if let (Some(on), Some(off)) = (find(&["On", "Data1", "D1", "D1-FIL1"]), find(&["Off"])) {
                return Ok(Some(DataControl::Dropdown { on, off }));
            }
        }
        Ok(None)
    }

    /// Preme (`1`) o rilascia (`0`) un pulsante.
    fn press(&mut self, name: &str, on: bool) -> Result<(), HrdError> {
        self.set(&format!("set button-select {name} {}", u8::from(on)))?;
        if (name == "RX~A" || name == "RX~B") && self.radio_name.to_ascii_uppercase().contains("FTDX3000") {
            std::thread::sleep(self.ftdx3000_pause);
        }
        Ok(())
    }

    fn set_dropdown(&mut self, name: &str, value: &str, index: usize) -> Result<(), HrdError> {
        self.set(&format!("set dropdown {} {} {index}", tilde(name), tilde(value)))
    }

    fn set_data(&mut self, on: bool) -> Result<(), HrdError> {
        match self.data.clone() {
            None => Ok(()),
            Some(DataControl::IcomButton) => self.press("Data", on),
            Some(DataControl::Buttons { on: b_on, off: b_off }) => self.press(if on { &b_on } else { &b_off }, true),
            Some(DataControl::Dropdown { on: d_on, off: d_off }) => {
                let (value, index) = if on { d_on } else { d_off };
                self.set_dropdown("Data", &value, index)
            }
        }
    }

    /// Il modo dati e' acceso? (`None` se la radio non ha un comando dei dati leggibile).
    fn read_data(&mut self) -> Result<Option<bool>, HrdError> {
        match self.data.clone() {
            Some(DataControl::IcomButton) => match self.command("get button-select Data")?.trim() {
                "1" => Ok(Some(true)),
                "0" => Ok(Some(false)),
                other => Err(HrdError::Reply(format!("risposta «{other}» a «get button-select Data»"))),
            },
            Some(DataControl::Dropdown { .. }) => {
                let value = dropdown_value(&self.command("get dropdown-text {Data}")?).unwrap_or_default();
                Ok(Some(!value.is_empty() && !value.eq_ignore_ascii_case("Off")))
            }
            _ => Ok(None),
        }
    }

    /// Mette il modo chiesto: la voce del menu del modo e, se serve, il comando dei dati.
    fn apply_mode(&mut self, mode: CatMode) -> Result<Option<String>, HrdError> {
        let Some(dd) = self.mode_dropdown.clone() else {
            return Err(HrdError::Reply("la radio non ha un menu del modo in HRD".into()));
        };
        match mode {
            CatMode::None => Ok(None),
            CatMode::Usb => {
                let (entry, index) = pick_entry(&self.mode_list, HrdMode::Usb).ok_or_else(|| HrdError::Reply("la radio non ha il modo USB".into()))?;
                self.set_dropdown(&dd, &entry, index)?;
                self.set_data(false)?;
                Ok(Some("USB".into()))
            }
            CatMode::Data => {
                // una voce dati dedicata, se c'e'; altrimenti USB con il comando dei dati (Icom)
                if let Some((entry, index)) = pick_entry(&self.mode_list, HrdMode::DataUsb) {
                    self.set_dropdown(&dd, &entry, index)?;
                    return Ok(Some("PKTUSB".into()));
                }
                let usb = pick_entry(&self.mode_list, HrdMode::Usb);
                match (usb, self.data.is_some()) {
                    (Some((entry, index)), true) => {
                        self.set_dropdown(&dd, &entry, index)?;
                        self.set_data(true)?;
                        Ok(Some("PKTUSB".into()))
                    }
                    _ => Err(HrdError::Reply("la radio non ha un modo dati in HRD".into())),
                }
            }
        }
    }

    /// Il modo letto (con i nomi usati nel resto del programma).
    fn read_mode(&mut self) -> Result<Option<String>, HrdError> {
        let Some(dd) = self.mode_dropdown.clone() else { return Ok(None) };
        let reply = self.command(&format!("get dropdown-text {{{dd}}}"))?;
        let Some(entry) = dropdown_value(&reply) else {
            return Err(HrdError::Reply(format!("modo illeggibile «{reply}»")));
        };
        let data = self.read_data()?.unwrap_or(false);
        Ok(Some(with_data(mode_name(&entry), data)))
    }

    /// La frequenza di ricezione (VFO A).
    fn read_frequency(&mut self) -> Result<u64, HrdError> {
        if self.vfo_count > 1 {
            let reply = self.command("get frequencies")?;
            return parse_frequencies(&reply).map(|(a, _)| a).ok_or_else(|| HrdError::Reply(format!("frequenze illeggibili «{reply}»")));
        }
        let reply = self.command("get frequency")?;
        reply.trim().parse().map_err(|_| HrdError::Reply(format!("frequenza illeggibile «{reply}»")))
    }

    fn write_frequency(&mut self, hz: u64) -> Result<(), HrdError> {
        self.set(&format!("set frequency-hz {hz}"))
    }

    /// La radio c'e' ancora in HRD con lo stesso nome? (dopo un comando rifiutato)
    fn check_radio(&mut self) -> Result<(), HrdError> {
        let name = self.command("get radio")?;
        if name.trim().eq_ignore_ascii_case(&self.radio_name) {
            Ok(())
        } else {
            Err(HrdError::Link(format!("la radio «{}» non c'e' piu' in HRD (ora «{}»)", self.radio_name, name.trim())))
        }
    }

    /// Split DX: ricezione sul VFO A, trasmissione sul VFO B, split acceso. Solo con i comandi di split della radio.
    fn dx_split(&mut self, rx_hz: u64, tx_hz: u64) -> Result<(), HrdError> {
        let split_on = self.first_button(&["Spl~On", "Spl_On", "Split", "Split~On"]);
        let split_dd = self.has_dropdown("Split");
        if split_on.is_none() && !split_dd {
            return Err(HrdError::Reply("la radio non ha un comando di split in HRD: lo split DX non e' supportato".into()));
        }
        if self.vfo_count > 1 {
            self.set(&format!("set frequencies-hz {rx_hz} {tx_hz}"))?;
        } else {
            let to_b = self.first_button(&["VFO~B", "Sub"]).or_else(|| self.first_button(&["A~/~B", "VFO~A/B"]));
            let to_a = self.first_button(&["VFO~A", "Main"]).or_else(|| self.first_button(&["A~/~B", "VFO~A/B"]));
            let (Some(to_b), Some(to_a)) = (to_b, to_a) else {
                return Err(HrdError::Reply("la radio non ha i comandi VFO A/B in HRD: lo split DX non e' supportato".into()));
            };
            self.press(&to_b, true)?;
            self.write_frequency(tx_hz)?;
            self.press(&to_a, true)?;
            self.write_frequency(rx_hz)?;
        }
        self.split(true)
    }

    fn split(&mut self, on: bool) -> Result<(), HrdError> {
        let split_off = self.first_button(&["Spl~Off", "Spl_Off", "Split~Off"]);
        let split_on = self.first_button(&["Spl~On", "Spl_On", "Split", "Split~On"]);
        match (on, split_off, split_on) {
            (false, Some(off), _) => self.press(&off, true),
            (_, _, Some(b)) => self.press(&b, on),
            _ if self.has_dropdown("Split") => {
                let list = split_list(&self.command("get dropdown-list {Split}")?);
                let want = if on { "On" } else { "Off" };
                let index = list.iter().position(|e| e.eq_ignore_ascii_case(want)).ok_or_else(|| HrdError::Reply(format!("il menu Split non ha la voce {want}")))?;
                self.set_dropdown("Split", want, index)
            }
            _ => Err(HrdError::Reply("la radio non ha un comando di split in HRD".into())),
        }
    }
}

// ---------------------------------------------------------------- la radio

struct Inner {
    address: String,
    wanted_radio: String,
    cat_mode: CatMode,
    /// Split simulato di FT8 (anche con quello «della radio»).
    split: bool,
    timing: HrdTiming,
    stop: AtomicBool,
    ready: AtomicBool,
    conn: Mutex<Option<Conn>>,
    /// La frequenza audio di trasmissione (Hz, 0 = non ancora data): serve allo split simulato.
    tx_audio_hz: AtomicU32,
    /// Split simulato: la frequenza di ricezione da rimettere a fine trasmissione (0 = niente).
    emulate_restore: AtomicU64,
    /// Lo split DX acceso da Aurora.
    dx_split: AtomicBool,
    status: Mutex<RigStatus>,
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
        if self.ready.load(Ordering::SeqCst) { Ok(()) } else { Err(RigError("radio Ham Radio Deluxe non collegata".into())) }
    }

    /// Un'operazione sulla connessione aperta. Un guasto del collegamento la chiude (il thread si ricollega); un comando rifiutato fa
    /// controllare che la radio ci sia ancora.
    fn with_conn<T>(&self, op: impl FnOnce(&mut Conn) -> Result<T, HrdError>) -> Result<T, RigError> {
        let mut guard = self.conn.lock().map_err(|_| RigError("collegamento non disponibile".into()))?;
        let conn = guard.as_mut().ok_or_else(|| RigError("radio Ham Radio Deluxe non collegata".into()))?;
        match op(conn) {
            Ok(v) => Ok(v),
            Err(e) => {
                let lost = match &e {
                    HrdError::Link(_) => true,
                    HrdError::Reply(_) => conn.check_radio().is_err(),
                };
                if lost {
                    *guard = None;
                    self.ready.store(false, Ordering::SeqCst);
                }
                Err(RigError(e.message()))
            }
        }
    }

    fn set_conn(&self, conn: Option<Conn>) {
        if let Ok(mut c) = self.conn.lock() {
            *c = conn;
        }
    }

    fn has_conn(&self) -> bool {
        self.conn.lock().is_ok_and(|c| c.is_some())
    }

    fn write_frequency(&self, hz: u64) -> Result<(), RigError> {
        self.with_conn(|c| c.write_frequency(hz))?;
        self.update(|s| s.frequency_hz = Some(hz));
        Ok(())
    }

    /// Una lettura periodica (frequenza e modo; con un solo VFO, in trasmissione, niente: la radio leggerebbe male).
    fn poll(&self) -> bool {
        let ptt = self.status().ptt;
        let read = self.with_conn(|c| {
            if c.vfo_count <= 1 && ptt {
                return Ok((None, None));
            }
            let hz = c.read_frequency()?;
            let mode = c.read_mode()?;
            Ok((Some(hz), mode))
        });
        match read {
            Ok((hz, mode)) => {
                self.update(|s| {
                    if hz.is_some() {
                        s.frequency_hz = hz;
                    }
                    if mode.is_some() {
                        s.mode = mode;
                    }
                });
                true
            }
            Err(e) => {
                tracing::debug!("{}", e.0);
                false
            }
        }
    }

    fn sleep(&self, wait: Duration) {
        let until = Instant::now() + wait;
        while Instant::now() < until && !self.stop.load(Ordering::SeqCst) {
            std::thread::sleep(Duration::from_millis(10).min(wait));
        }
    }
}

/// Una radio comandata attraverso Ham Radio Deluxe (TCP).
pub struct HrdRig {
    inner: Arc<Inner>,
    worker: Mutex<Option<JoinHandle<()>>>,
}

impl HrdRig {
    /// Dalla configurazione della radio: host, porta, radio di HRD, modo CAT e split.
    pub fn from_config(config: &RigConfig) -> HrdRig {
        HrdRig::with_timing(config, HrdTiming::default())
    }

    pub fn with_timing(config: &RigConfig, timing: HrdTiming) -> HrdRig {
        let host = config.hrd_host.trim();
        let host = if host.is_empty() { DEFAULT_HRD_HOST } else { host };
        let port = if config.hrd_port == 0 { DEFAULT_HRD_PORT } else { config.hrd_port };
        let address = if host.contains(':') && !host.starts_with('[') { format!("[{host}]:{port}") } else { format!("{host}:{port}") };
        let inner = Arc::new(Inner {
            address,
            wanted_radio: config.hrd_radio.clone(),
            cat_mode: config.cat_mode,
            split: config.split != SplitMode::None,
            timing,
            stop: AtomicBool::new(false),
            ready: AtomicBool::new(false),
            conn: Mutex::new(None),
            tx_audio_hz: AtomicU32::new(0),
            emulate_restore: AtomicU64::new(0),
            dx_split: AtomicBool::new(false),
            status: Mutex::new(RigStatus {
                state: LinkState::Connecting,
                message: String::new(),
                frequency_hz: None,
                mode: None,
                passband_hz: None,
                extras: Default::default(),
                ptt: false,
                split: None,
                split_external_ms: 0,
                failures: 0,
                reconnect_attempt: 0,
                ptt_blocked: false,
                updated_ms: 0,
            }),
        });
        let worker_inner = Arc::clone(&inner);
        let worker = std::thread::Builder::new().name("hrd".into()).spawn(move || run(&worker_inner)).ok();
        HrdRig { inner, worker: Mutex::new(worker) }
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

    /// Il PTT con il pulsante `TX` di HRD. Con lo split simulato il VFO A va prima sulla frequenza di trasmissione e torna dopo.
    pub fn set_ptt(&self, on: bool) -> Result<(), RigError> {
        let inner = &self.inner;
        inner.connected()?;
        if on && !inner.with_conn(|c| Ok(c.has_button("TX")))? {
            return Err(RigError(NO_TX_BUTTON.into()));
        }
        if on && inner.split && !inner.dx_split.load(Ordering::SeqCst) {
            let audio = inner.tx_audio_hz.load(Ordering::SeqCst);
            if let Some(dial) = self.status().frequency_hz
                && audio != 0
                && inner.emulate_restore.load(Ordering::SeqCst) == 0
            {
                let tx = split_tx_dial_hz(dial, audio);
                if tx != dial {
                    inner.write_frequency(tx)?;
                    inner.emulate_restore.store(dial, Ordering::SeqCst);
                }
            }
        }
        let pressed = inner.with_conn(|c| if c.has_button("TX") { c.press("TX", on) } else { Ok(()) });
        if pressed.is_ok() || !on {
            inner.update(|s| s.ptt = on);
        }
        if !on {
            let back = inner.emulate_restore.swap(0, Ordering::SeqCst);
            if back != 0 {
                inner.write_frequency(back)?;
            }
        }
        pressed
    }

    /// La frequenza audio con cui si trasmette (Hz): serve allo split simulato al prossimo PTT.
    pub fn set_tx_audio_frequency(&self, hz: u32) -> Result<(), RigError> {
        if hz > 5000 {
            return Err(RigError(format!("frequenza audio di trasmissione non valida: {hz} Hz")));
        }
        self.inner.tx_audio_hz.store(hz, Ordering::SeqCst);
        // a trasmissione in corso il VFO A va subito sulla nuova frequenza di trasmissione
        if self.inner.split && hz != 0 && self.transmitting() && !self.inner.dx_split.load(Ordering::SeqCst) {
            let restore = self.inner.emulate_restore.load(Ordering::SeqCst);
            let dial = if restore != 0 { Some(restore) } else { self.status().frequency_hz };
            if let Some(dial) = dial {
                let tx = split_tx_dial_hz(dial, hz);
                self.inner.write_frequency(tx)?;
                self.inner.emulate_restore.store(if tx != dial { dial } else { 0 }, Ordering::SeqCst);
            }
        }
        Ok(())
    }

    /// Prima di una trasmissione libera (PSK, RTTY, CW, SSTV): la frequenza audio di FT8 si dimentica, cosi' lo split simulato non sposta
    /// il VFO A.
    pub fn release_tx_split(&self) {
        self.inner.tx_audio_hz.store(0, Ordering::SeqCst);
    }

    /// Split DX con i comandi di split della radio in HRD (errore chiaro se HRD non li offre).
    pub fn set_dx_split(&self, rx_hz: u64, tx_hz: u64) -> Result<(), RigError> {
        self.inner.connected()?;
        self.inner.with_conn(|c| c.dx_split(rx_hz, tx_hz))?;
        self.inner.dx_split.store(true, Ordering::SeqCst);
        self.inner.update(|s| {
            s.frequency_hz = Some(rx_hz);
            s.split = Some(true);
        });
        Ok(())
    }

    pub fn dx_split_active(&self) -> bool {
        self.inner.dx_split.load(Ordering::SeqCst)
    }

    pub fn clear_dx_split(&self) -> Result<(), RigError> {
        if !self.dx_split_active() {
            return Ok(());
        }
        self.inner.connected()?;
        self.inner.with_conn(|c| c.split(false))?;
        self.inner.dx_split.store(false, Ordering::SeqCst);
        self.inner.update(|s| s.split = Some(false));
        Ok(())
    }

    /// Il thread del collegamento e' morto per un panic? Ne restituisce il testo, una volta sola.
    pub fn worker_panic(&self) -> Option<String> {
        crate::watchdog::reap_panic(&self.worker)
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
        self.inner.set_conn(None);
        self.inner.update(|s| {
            s.state = LinkState::Disconnected;
            s.ptt = false;
        });
    }
}

impl Drop for HrdRig {
    fn drop(&mut self) {
        self.shutdown();
    }
}

fn run(inner: &Inner) {
    let mut attempt = 0u32;
    while !inner.stop.load(Ordering::SeqCst) {
        let opened = Conn::open(&inner.address, &inner.timing).map_err(|e| format!("{} ({e})", not_answering(&inner.address)));
        let checked = opened.and_then(|mut conn| {
            conn.handshake(&inner.wanted_radio)?;
            tracing::info!(
                "Ham Radio Deluxe: collegata «{}» ({:?}, {} VFO, PTT {})",
                conn.radio_name,
                conn.dialect,
                conn.vfo_count,
                if conn.has_button("TX") { "con il pulsante TX" } else { "assente" }
            );
            inner.set_conn(Some(conn));
            Ok(())
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
                inner.set_conn(None);
                // lo split DX resta segnato: la radio e' ancora in split e lo spegnimento deve poter arrivare dopo il ricollegamento
                if inner.stop.load(Ordering::SeqCst) {
                    break;
                }
                tracing::warn!("Ham Radio Deluxe: collegamento perso, nuovo tentativo");
                inner.update(|s| {
                    s.state = LinkState::Reconnecting;
                    s.message = LINK_LOST.into();
                    s.ptt = false;
                });
            }
            Err(message) => {
                inner.set_conn(None);
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
        let wait = (inner.timing.retry_step * attempt.clamp(1, 10)).min(inner.timing.retry_cap);
        inner.sleep(wait);
    }
}

/// Le letture periodiche di una connessione aperta. Torna quando HRD smette di rispondere, quando la connessione si chiude o alla chiusura.
fn serve(inner: &Inner) {
    let mut failures = 0u32;
    while !inner.stop.load(Ordering::SeqCst) {
        if inner.poll() {
            if failures != 0 {
                inner.update(|s| s.failures = 0);
            }
            failures = 0;
        } else {
            if !inner.has_conn() {
                return;
            }
            failures += 1;
            inner.update(|s| s.failures = failures);
            if failures >= inner.timing.tolerate {
                return;
            }
        }
        if !inner.has_conn() {
            return;
        }
        let wait = if inner.status().ptt { inner.timing.poll_tx } else { inner.timing.poll };
        inner.sleep(wait);
    }
}

impl Rig for HrdRig {
    /// Cambio di frequenza del VFO A (non in trasmissione) e, se scelto, il modo (USB o USB dati), rimandato dopo un cambio di banda.
    fn set_frequency(&self, hz: u64) -> Result<(), RigError> {
        if !valid_dial_hz(hz) {
            return Err(RigError(format!("frequenza non valida: {hz} Hz")));
        }
        self.inner.connected()?;
        if self.transmitting() {
            return Err(RigError("la radio sta trasmettendo: cambio di frequenza rifiutato".into()));
        }
        let band_before = self.status().frequency_hz.map(|f| f / 1_000_000);
        self.inner.write_frequency(hz)?;
        let cat_mode = self.inner.cat_mode;
        if cat_mode != CatMode::None {
            let wanted = match cat_mode {
                CatMode::Usb => Some("USB"),
                CatMode::Data => Some("PKTUSB"),
                CatMode::None => None,
            };
            // cambiando banda la radio puo' riprendere il modo salvato per quella banda: si rimanda
            if band_before != Some(hz / 1_000_000) || self.status().mode.as_deref() != wanted {
                // la frequenza e' gia' cambiata: un modo che la radio non offre non fa fallire il cambio
                match self.inner.with_conn(|c| c.apply_mode(cat_mode)) {
                    Ok(Some(mode)) => self.inner.update(|s| s.mode = Some(mode)),
                    Ok(None) => {}
                    Err(e) => tracing::warn!("modo non impostato: {}", e.0),
                }
            }
        }
        Ok(())
    }

    fn frequency(&self) -> Result<u64, RigError> {
        let s = self.status();
        match (s.state, s.frequency_hz) {
            (LinkState::Connected, Some(hz)) => Ok(hz),
            _ => Err(RigError("radio Ham Radio Deluxe non collegata".into())),
        }
    }

    fn meters(&self) -> Result<Meters, RigError> {
        Err(RigError("misuratori non disponibili con Ham Radio Deluxe".into()))
    }

    fn set_rx_frequency(&self, _hz: u32) -> Result<(), RigError> {
        Err(RigError("comando del motore non supportato da Ham Radio Deluxe".into()))
    }

    fn output_attenuation(&self) -> Result<u32, RigError> {
        Err(RigError("comando del motore non supportato da Ham Radio Deluxe".into()))
    }

    fn set_output_attenuation(&self, _tenths: u32) -> Result<(), RigError> {
        Err(RigError("comando del motore non supportato da Ham Radio Deluxe".into()))
    }

    fn set_mode(&self, _mode: &str) -> Result<(), RigError> {
        Err(RigError("comando del motore non supportato da Ham Radio Deluxe".into()))
    }
}

#[cfg(test)]
mod tests {
    use std::net::TcpListener;

    use super::*;

    // ------------------------------------------------ finto server HRD

    #[derive(Clone)]
    struct FakeState {
        v4: bool,
        radio: String,
        vfo_count: u32,
        buttons: String,
        modes: String,
        frequency: u64,
        mode: String,
        tx: bool,
        data: bool,
        /// Risposta dei `set` diversa da OK.
        bad_set: bool,
        /// Chiude la connessione al prossimo comando.
        kill: bool,
        log: Vec<String>,
        connections: u32,
    }

    impl Default for FakeState {
        fn default() -> Self {
            FakeState {
                v4: false,
                radio: "IC-7300".into(),
                vfo_count: 1,
                buttons: "TX,VFO A,VFO B,Split,Data".into(),
                modes: "LSB,USB,CW,RTTY,AM,FM".into(),
                frequency: 14_074_000,
                mode: "USB".into(),
                tx: false,
                data: false,
                bad_set: false,
                kill: false,
                log: Vec::new(),
                connections: 0,
            }
        }
    }

    type Shared = Arc<Mutex<FakeState>>;

    fn answer(state: &Shared, text: &str) -> Option<String> {
        let mut s = state.lock().ok()?;
        if s.kill {
            s.kill = false;
            return None;
        }
        s.log.push(text.to_string());
        let body = match text.strip_prefix('[') {
            Some(rest) => rest.split_once("] ").map(|(_, b)| b.to_string()).unwrap_or_default(),
            None => text.to_string(),
        };
        let ok = if s.bad_set { "ERROR".to_string() } else { "OK".to_string() };
        let reply = match body.as_str() {
            "get context" => "1".into(),
            "get radios" => format!("1:{}", s.radio),
            "get radio" => s.radio.clone(),
            "get vfo-count" => s.vfo_count.to_string(),
            "get buttons" => s.buttons.clone(),
            "get dropdowns" => "Mode,AGC".into(),
            "get dropdown-list {Mode}" => s.modes.clone(),
            "get frequency" => s.frequency.to_string(),
            "get frequencies" => format!("{}-{}", s.frequency, s.frequency + 1000),
            "get dropdown-text {Mode}" => format!("Mode: {}", s.mode),
            "get button-select Data" => if s.data { "1".into() } else { "0".into() },
            b if b.starts_with("set frequency-hz ") => {
                if !s.bad_set {
                    s.frequency = b["set frequency-hz ".len()..].trim().parse().unwrap_or(0);
                }
                ok
            }
            b if b.starts_with("set dropdown Mode ") => {
                if !s.bad_set {
                    s.mode = b.split(' ').nth(3).unwrap_or("").replace('~', " ");
                }
                ok
            }
            b if b.starts_with("set button-select TX ") => {
                if !s.bad_set {
                    s.tx = b.ends_with(" 1");
                }
                ok
            }
            b if b.starts_with("set button-select Data ") => {
                if !s.bad_set {
                    s.data = b.ends_with(" 1");
                }
                ok
            }
            b if b.starts_with("set ") => ok,
            _ => "".into(),
        };
        Some(reply)
    }

    fn serve_v5(mut stream: TcpStream, state: Shared) {
        let mut buf = Vec::new();
        let mut chunk = [0u8; 1024];
        loop {
            match decode_v5(&buf) {
                Ok(Some(text)) => {
                    let size = le_u32(&buf, 0) as usize;
                    buf.drain(..size);
                    let Some(reply) = answer(&state, &text) else { return };
                    if stream.write_all(&encode_v5(&reply)).is_err() {
                        return;
                    }
                    continue;
                }
                Ok(None) => {}
                Err(_) => return,
            }
            match stream.read(&mut chunk) {
                Ok(0) | Err(_) => return,
                Ok(n) => buf.extend_from_slice(&chunk[..n]),
            }
        }
    }

    fn serve_v4(mut stream: TcpStream, state: Shared) {
        // i messaggi binari (senza CR) restano senza risposta, come fa una v4
        let mut buf = Vec::new();
        let mut chunk = [0u8; 1024];
        loop {
            while let Some(pos) = buf.iter().position(|b| *b == b'\r') {
                let line: Vec<u8> = buf.drain(..=pos).collect();
                let text = String::from_utf8_lossy(&line[..line.len() - 1]).to_string();
                let Some(reply) = answer(&state, &text) else { return };
                if stream.write_all(format!("{reply}\r\n").as_bytes()).is_err() {
                    return;
                }
            }
            match stream.read(&mut chunk) {
                Ok(0) | Err(_) => return,
                Ok(n) => buf.extend_from_slice(&chunk[..n]),
            }
        }
    }

    fn fake(state: FakeState) -> (u16, Shared) {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
        let port = listener.local_addr().expect("addr").port();
        let shared = Arc::new(Mutex::new(state));
        let s = Arc::clone(&shared);
        std::thread::spawn(move || {
            for stream in listener.incoming() {
                let Ok(stream) = stream else { continue };
                let s = Arc::clone(&s);
                let v4 = {
                    let mut st = s.lock().expect("stato");
                    st.connections += 1;
                    st.v4
                };
                std::thread::spawn(move || if v4 { serve_v4(stream, s) } else { serve_v5(stream, s) });
            }
        });
        (port, shared)
    }

    fn timing() -> HrdTiming {
        HrdTiming {
            poll: Duration::from_millis(30),
            poll_tx: Duration::from_millis(30),
            connect: Duration::from_millis(500),
            read: Duration::from_millis(100),
            attempts: 3,
            probe_attempts: 1,
            tolerate: 2,
            retry_step: Duration::from_millis(50),
            retry_cap: Duration::from_millis(100),
            single_vfo_pause: Duration::from_millis(1),
            ftdx3000_pause: Duration::from_millis(1),
        }
    }

    fn rig(port: u16, edit: impl FnOnce(&mut RigConfig)) -> HrdRig {
        let mut config = RigConfig { backend: crate::cat_config::Backend::Hrd, hrd_host: "127.0.0.1".into(), hrd_port: port, ..RigConfig::default() };
        edit(&mut config);
        HrdRig::with_timing(&config, timing())
    }

    fn wait_for(what: &str, cond: impl Fn() -> bool) {
        let until = Instant::now() + Duration::from_secs(5);
        while Instant::now() < until {
            if cond() {
                return;
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        panic!("tempo scaduto: {what}");
    }

    fn connected(r: &HrdRig) {
        wait_for("collegata", || r.status().state == LinkState::Connected && r.status().frequency_hz.is_some());
    }

    // ------------------------------------------------ prove

    #[test]
    fn it_frames_v5_messages() {
        let msg = encode_v5("get frequency");
        assert_eq!(msg.len(), 16 + 2 * 14);
        assert_eq!(le_u32(&msg, 0) as usize, msg.len());
        assert_eq!(&msg[4..8], &[0xCD, 0xAB, 0x34, 0x12]);
        assert_eq!(&msg[8..12], &[0x34, 0x12, 0xCD, 0xAB]);
        assert_eq!(decode_v5(&msg), Ok(Some("get frequency".into())));
        assert_eq!(decode_v5(&msg[..20]), Ok(None), "messaggio a meta'");
        assert_eq!(decode_v5(&msg[..10]), Ok(None));
        let mut bad = msg.clone();
        bad[4] = 0;
        assert!(decode_v5(&bad).is_err(), "numeri magici");
        // dopo il primo NUL il resto si butta
        let mut two = encode_v5("USB\u{0}LSB");
        assert_eq!(decode_v5(&two), Ok(Some("USB".into())));
        two[0] = 8;
        assert!(decode_v5(&two).is_err(), "lunghezza sotto l'intestazione");
    }

    #[test]
    fn it_parses_lists_and_modes() {
        assert_eq!(parse_radios("1:IC-7300,2:FT-991"), vec![(1, "IC-7300".into()), (2, "FT-991".into())]);
        assert_eq!(split_list("On,,Off"), vec!["On", "", "Off"]);
        assert_eq!(parse_frequencies("14074000-14075000"), Some((14_074_000, 14_075_000)));
        assert_eq!(dropdown_value("Mode: DATA-U"), Some("DATA-U".into()));
        assert_eq!(dropdown_value("niente"), None);
        let list: Vec<String> = "LSB,USB,CW,DATA-L,DATA-U".split(',').map(String::from).collect();
        assert_eq!(pick_entry(&list, HrdMode::DataUsb), Some(("DATA-U".into(), 4)));
        assert_eq!(pick_entry(&list, HrdMode::Usb), Some(("USB".into(), 1)));
        assert_eq!(mode_name("data-u"), "PKTUSB");
        assert_eq!(mode_name("RTTY-USB"), "RTTYR");
        assert_eq!(mode_name("C4FM"), "C4FM");
        assert_eq!(with_data("USB".into(), true), "PKTUSB");
    }

    #[test]
    fn hrd_v5_handshake_frequency_mode_and_ptt() {
        let (port, state) = fake(FakeState::default());
        let r = rig(port, |c| c.cat_mode = CatMode::Data);
        connected(&r);
        assert_eq!(r.frequency().ok(), Some(14_074_000));
        assert_eq!(r.status().mode.as_deref(), Some("USB"));
        {
            let s = state.lock().expect("stato");
            assert!(s.log.iter().any(|l| l == "get radios"), "handshake senza contesto");
            assert!(s.log.iter().any(|l| l == "[1] get vfo-count"), "comandi con il contesto");
            assert!(!s.log.iter().any(|l| l.starts_with("[1] get context")));
        }
        // IC-7300 senza voce dati: USB e pulsante Data
        r.set_frequency(7_074_000).expect("frequenza");
        {
            let s = state.lock().expect("stato");
            assert_eq!(s.frequency, 7_074_000);
            assert!(s.log.iter().any(|l| l == "[1] set dropdown Mode USB 1"), "{:?}", s.log);
            assert!(s.log.iter().any(|l| l == "[1] set button-select Data 1"));
        }
        assert_eq!(r.status().mode.as_deref(), Some("PKTUSB"));
        r.set_ptt(true).expect("ptt");
        assert!(state.lock().expect("stato").tx);
        assert!(r.transmitting());
        r.set_ptt(false).expect("ptt off");
        assert!(!state.lock().expect("stato").tx);
        assert!(r.meters().is_err());
        r.shutdown();
        assert_eq!(r.status().state, LinkState::Disconnected);
    }

    #[test]
    fn hrd_v4_after_reopening() {
        let (port, state) = fake(FakeState { v4: true, vfo_count: 2, radio: "FT-991".into(), modes: "LSB,USB,DATA-L,DATA-U".into(), ..FakeState::default() });
        let r = rig(port, |c| c.cat_mode = CatMode::Data);
        connected(&r);
        assert!(state.lock().expect("stato").connections >= 2, "la v5 non risponde: si riapre");
        r.set_frequency(21_074_000).expect("frequenza");
        let s = state.lock().expect("stato").clone();
        assert_eq!(s.frequency, 21_074_000);
        assert_eq!(s.mode, "DATA-U");
        assert!(s.log.iter().any(|l| l == "[1] get frequencies"), "due VFO: get frequencies");
        assert!(s.log.iter().any(|l| l == "[1] set dropdown Mode DATA-U 3"));
    }

    #[test]
    fn hrd_set_without_ok_is_an_error() {
        let (port, state) = fake(FakeState::default());
        let r = rig(port, |_| {});
        connected(&r);
        state.lock().expect("stato").bad_set = true;
        let err = r.set_frequency(7_074_000).expect_err("rifiutato");
        assert!(err.0.contains("atteso OK"), "{}", err.0);
        assert_eq!(state.lock().expect("stato").frequency, 14_074_000);
        assert_eq!(r.status().state, LinkState::Connected, "la radio c'e' ancora: il collegamento resta");
    }

    #[test]
    fn hrd_without_tx_button() {
        let (port, _state) = fake(FakeState { buttons: "VFO A,VFO B".into(), ..FakeState::default() });
        let r = rig(port, |_| {});
        connected(&r);
        let err = r.set_ptt(true).expect_err("senza TX");
        assert_eq!(err.0, NO_TX_BUTTON);
        assert!(!r.transmitting());
    }

    #[test]
    fn hrd_reconnects_after_drop() {
        let (port, state) = fake(FakeState::default());
        let r = rig(port, |_| {});
        connected(&r);
        state.lock().expect("stato").kill = true;
        wait_for("caduta vista", || r.status().state != LinkState::Connected);
        assert!(matches!(r.status().state, LinkState::Reconnecting | LinkState::Error | LinkState::Connecting), "{:?}", r.status().state);
        wait_for("ricollegata", || r.status().state == LinkState::Connected);
        assert!(state.lock().expect("stato").connections >= 2);
    }

    #[test]
    fn hrd_unreachable_is_reported() {
        let port = TcpListener::bind("127.0.0.1:0").expect("bind").local_addr().expect("addr").port();
        let r = rig(port, |_| {});
        wait_for("errore", || matches!(r.status().state, LinkState::Error | LinkState::Reconnecting));
        assert!(r.status().message.contains("Ham Radio Deluxe non risponde"), "{}", r.status().message);
        assert!(r.set_ptt(true).is_err());
    }
}
