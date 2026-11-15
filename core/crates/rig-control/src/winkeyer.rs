//! WinKeyer (K1EL WinKeyer 2 e 3, e i compatibili WKUSB, WKmini): un manipolatore CW esterno su porta seriale o USB. Il programma gli
//! manda il testo in ASCII, lui genera la CW con i tempi giusti, comanda il PTT e il tasto della radio e accetta il paddle.
//!
//! Protocollo (specifica d'interfaccia di K1EL, porta a 1200 baud 8N2):
//! - apertura: `00 04 nn` (prova d'eco, torna `nn`: c'e' qualcuno dall'altra parte), poi `00 02` (host open), che risponde con la
//!   versione del firmware (23 = WK2.3, 30/31 = WK3); chiusura `00 03`;
//! - comandi immediati: `01` sidetone, `02` velocita' (0 = dal potenziometro), `03` peso, `04` PTT lead/tail (unita' da 10 ms), `05`
//!   impostazione del potenziometro, `09` configurazione delle uscite (PTT, sidetone, tasto), `0A` svuota il buffer, `0B` tasto
//!   giu'/su (Tune), `0E` modo (paddle, eco dei caratteri), `15` stato;
//! - comandi nel buffer: `1B a b` unisce due lettere in un solo segno (i segni di procedura <AR> <BT> <KN> <SK>);
//! - dal WinKeyer: `C0..FF` byte di stato (bit 0 buffer quasi pieno, 1 break-in del paddle, 2 occupato a trasmettere, 3 tasto giu',
//!   4 attesa), `80..BF` posizione del potenziometro (0-63), il resto e' l'eco dei caratteri man mano che escono in CW.
//!
//! L'avanzamento del testo si conta sull'eco: un pezzo di testo risulta partito quando sono tornati tutti i suoi caratteri. Se l'eco
//! manca (alcuni compatibili non lo mandano per i segni uniti) vale lo stato: il WinKeyer non piu' occupato vuol dire tutto partito.

use std::{
    collections::VecDeque,
    time::{Duration, Instant},
};

use serde::{Deserialize, Serialize};

/// Il collegamento con il WinKeyer (porta seriale vera o finta per le prove).
pub trait KeyerLink: Send {
    fn write(&mut self, bytes: &[u8]) -> Result<(), String>;
    /// Quello che e' arrivato, aspettando al massimo `wait` se non c'e' ancora niente.
    fn read(&mut self, wait: Duration) -> Result<Vec<u8>, String>;
}

/// Come si usa il paddle collegato al WinKeyer.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PaddleMode {
    #[default]
    IambicB,
    IambicA,
    Ultimatic,
    /// Tasto semiautomatico: punti automatici, linee a mano.
    Bug,
}

impl PaddleMode {
    /// I bit 5-4 del registro di modo.
    fn bits(self) -> u8 {
        let mode: u8 = match self {
            PaddleMode::IambicB => 0b00,
            PaddleMode::IambicA => 0b01,
            PaddleMode::Ultimatic => 0b10,
            PaddleMode::Bug => 0b11,
        };
        mode << 4
    }
}

/// Le impostazioni del WinKeyer.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct WinKeyerConfig {
    /// La porta (`COM5`).
    pub port: String,
    /// Velocita' dal potenziometro del WinKeyer invece che da Aurora.
    pub use_pot: bool,
    /// Potenziometro: velocita' minima e ampiezza della scala (WPM).
    pub pot_min_wpm: u8,
    pub pot_range_wpm: u8,
    /// Peso (10-90, 50 = normale).
    pub weight: u8,
    /// Anticipo del PTT prima del primo segno e coda dopo l'ultimo (ms, passi da 10).
    pub lead_ms: u16,
    pub tail_ms: u16,
    pub paddle: PaddleMode,
    /// Paddle invertito (punti e linee scambiati).
    pub paddle_swap: bool,
    /// Il WinKeyer comanda il PTT della radio.
    pub ptt: bool,
    /// Sidetone del WinKeyer acceso e la sua nota (Hz, 400-4000).
    pub sidetone: bool,
    pub sidetone_hz: u16,
    /// Rapporto punto/linea (33-66, 50 = 1:3).
    pub ratio: u8,
    /// Attesa del PTT dopo il paddle: 0 = 1 spazio di parola, 1 = 1,33, 2 = 1,67, 3 = 2.
    pub hang_time: u8,
    /// Autospace: spazio fra le lettere automatico col paddle.
    pub autospace: bool,
    /// Contest spacing: spazio fra le parole di 6 unita' invece di 7.
    pub contest_spacing: bool,
    /// Compensazione della manipolazione (ms aggiunti a ogni segno, 0-250): radio con QSK lenta.
    pub key_comp_ms: u8,
    /// First extension (ms aggiunti al primo segno, 0-250): radio che tagliano il primo punto.
    pub first_extension_ms: u8,
    /// Farnsworth: velocita' dei caratteri (WPM, 0 = spento, 10-99).
    pub farnsworth_wpm: u8,
    /// Il paddle ha la precedenza: toccandolo durante una macro il buffer si svuota e la trasmissione si ferma.
    pub paddle_breakin: bool,
}

impl Default for WinKeyerConfig {
    fn default() -> Self {
        WinKeyerConfig {
            port: String::new(),
            use_pot: false,
            pot_min_wpm: 10,
            pot_range_wpm: 25,
            weight: 50,
            lead_ms: 50,
            tail_ms: 100,
            paddle: PaddleMode::IambicB,
            paddle_swap: false,
            ptt: true,
            sidetone: true,
            sidetone_hz: 800,
            ratio: 50,
            hang_time: 0,
            autospace: false,
            contest_spacing: false,
            key_comp_ms: 0,
            first_extension_ms: 0,
            farnsworth_wpm: 0,
            paddle_breakin: true,
        }
    }
}

impl WinKeyerConfig {
    /// I valori dentro i limiti del WinKeyer.
    pub fn clamped(&self) -> WinKeyerConfig {
        WinKeyerConfig {
            port: self.port.trim().to_string(),
            pot_min_wpm: self.pot_min_wpm.clamp(5, 60),
            pot_range_wpm: self.pot_range_wpm.clamp(5, 40),
            weight: self.weight.clamp(10, 90),
            lead_ms: self.lead_ms.min(2500),
            tail_ms: self.tail_ms.min(2500),
            sidetone_hz: self.sidetone_hz.clamp(400, 4000),
            ratio: self.ratio.clamp(33, 66),
            hang_time: self.hang_time.min(3),
            key_comp_ms: self.key_comp_ms.min(250),
            first_extension_ms: self.first_extension_ms.min(250),
            farnsworth_wpm: if self.farnsworth_wpm < 10 { 0 } else { self.farnsworth_wpm.min(99) },
            ..self.clone()
        }
    }
}

/// I comandi del protocollo.
pub mod cmd {
    use super::{PaddleMode, WinKeyerConfig};

    pub const HOST_OPEN: [u8; 2] = [0x00, 0x02];
    pub const HOST_CLOSE: [u8; 2] = [0x00, 0x03];
    pub const CLEAR_BUFFER: [u8; 1] = [0x0A];
    pub const REQUEST_STATUS: [u8; 1] = [0x15];
    /// Tre comandi nulli: rimettono in riga il WinKeyer se un comando era rimasto a meta'.
    pub const SYNC: [u8; 3] = [0x13, 0x13, 0x13];

    pub fn echo_test(b: u8) -> [u8; 3] {
        [0x00, 0x04, b]
    }

    /// Velocita' in WPM (5-99); 0 = dal potenziometro.
    pub fn speed(wpm: u8) -> [u8; 2] {
        [0x02, if wpm == 0 { 0 } else { wpm.clamp(5, 99) }]
    }

    pub fn weight(w: u8) -> [u8; 2] {
        [0x03, w.clamp(10, 90)]
    }

    pub fn ptt_lead_tail(lead_ms: u16, tail_ms: u16) -> [u8; 3] {
        let unit = |ms: u16| (ms / 10).min(250) as u8;
        [0x04, unit(lead_ms), unit(tail_ms)]
    }

    pub fn pot_setup(min_wpm: u8, range_wpm: u8) -> [u8; 4] {
        [0x05, min_wpm, range_wpm, 0]
    }

    /// Sidetone: la nota e' 4000/n Hz con n da 1 a 10.
    pub fn sidetone(hz: u16) -> [u8; 2] {
        let n = (4000.0 / f32::from(hz.max(1))).round().clamp(1.0, 10.0) as u8;
        [0x01, n]
    }

    /// Le uscite: bit 0 PTT, bit 1 sidetone, bit 2 e 3 le due uscite del tasto (sempre accese), bit 5-4 attesa del PTT dopo il paddle.
    pub fn pin_config(ptt: bool, sidetone: bool, hang_time: u8) -> [u8; 2] {
        [0x09, u8::from(ptt) | (u8::from(sidetone) << 1) | 0b1100 | ((hang_time.min(3)) << 4)]
    }

    /// Il registro di modo: paddle, inversione, eco dei caratteri inviati (bit 2, serve per l'avanzamento), autospace (bit 1), contest
    /// spacing (bit 0).
    pub fn mode(paddle: PaddleMode, swap: bool, autospace: bool, contest_spacing: bool) -> [u8; 2] {
        [0x0E, paddle.bits() | (u8::from(swap) << 3) | 0b100 | (u8::from(autospace) << 1) | u8::from(contest_spacing)]
    }

    /// Rapporto punto/linea (33-66, 50 = 1:3).
    pub fn ratio(r: u8) -> [u8; 2] {
        [0x17, r.clamp(33, 66)]
    }

    pub fn key_compensation(ms: u8) -> [u8; 2] {
        [0x11, ms.min(250)]
    }

    pub fn first_extension(ms: u8) -> [u8; 2] {
        [0x10, ms.min(250)]
    }

    /// Farnsworth (0 = spento).
    pub fn farnsworth(wpm: u8) -> [u8; 2] {
        [0x0D, wpm]
    }

    pub fn key_immediate(down: bool) -> [u8; 2] {
        [0x0B, u8::from(down)]
    }

    /// Tutto quello che va mandato dopo l'apertura per queste impostazioni e questa velocita'.
    pub fn configure(cfg: &WinKeyerConfig, wpm: u8) -> Vec<u8> {
        let c = cfg.clamped();
        let mut out = Vec::new();
        out.extend_from_slice(&mode(c.paddle, c.paddle_swap, c.autospace, c.contest_spacing));
        out.extend_from_slice(&pin_config(c.ptt, c.sidetone, c.hang_time));
        out.extend_from_slice(&ratio(c.ratio));
        out.extend_from_slice(&key_compensation(c.key_comp_ms));
        out.extend_from_slice(&first_extension(c.first_extension_ms));
        out.extend_from_slice(&farnsworth(c.farnsworth_wpm));
        out.extend_from_slice(&sidetone(c.sidetone_hz));
        out.extend_from_slice(&weight(c.weight));
        out.extend_from_slice(&ptt_lead_tail(c.lead_ms, c.tail_ms));
        out.extend_from_slice(&pot_setup(c.pot_min_wpm, c.pot_range_wpm));
        out.extend_from_slice(&speed(if c.use_pot { 0 } else { wpm }));
        out.extend_from_slice(&REQUEST_STATUS);
        out
    }
}

/// I segni di procedura scritti fra `<` e `>` e le due lettere che il WinKeyer unisce.
const PROSIGNS: &[(&str, [u8; 2])] = &[("<AR>", *b"AR"), ("<BT>", *b"BT"), ("<KN>", *b"KN"), ("<SK>", *b"SK"), ("<AS>", *b"AS"), ("<BK>", *b"BK"), ("<SN>", *b"SN")];

/// Un pezzo di testo nei byte per il buffer del WinKeyer e quanti caratteri d'eco tornano. Le lettere vanno in maiuscolo, a capo e
/// tabulazioni diventano spazi, i caratteri che il WinKeyer non conosce si tolgono.
pub fn encode(text: &str) -> (Vec<u8>, usize) {
    let mut out = Vec::new();
    let mut echoes = 0;
    let upper = text.to_ascii_uppercase();
    let mut rest = upper.as_str();
    while let Some(c) = rest.chars().next() {
        if let Some((name, pair)) = PROSIGNS.iter().find(|(name, _)| rest.starts_with(name)) {
            out.push(0x1B);
            out.extend_from_slice(pair);
            echoes += 2;
            rest = &rest[name.len()..];
            continue;
        }
        rest = &rest[c.len_utf8()..];
        let b = match c {
            'A'..='Z' | '0'..='9' | ' ' => c as u8,
            '.' | ',' | '?' | '/' | '=' | '-' | '+' | '(' | ')' | '\'' | ':' | '"' | '@' | ';' | '!' | '&' | '$' => c as u8,
            '\r' | '\n' | '\t' => b' ',
            _ => continue,
        };
        out.push(b);
        echoes += 1;
    }
    (out, echoes)
}

/// Bit del byte di stato.
pub mod status {
    pub const XOFF: u8 = 0x01;
    pub const BREAKIN: u8 = 0x02;
    pub const BUSY: u8 = 0x04;
    pub const KEYDOWN: u8 = 0x08;
}

/// Quanti caratteri al massimo si tengono nel buffer del WinKeyer (ne ha 128): pochi, cosi' la ripartenza della macro e il Backspace
/// toccano il testo fino a poco prima che esca.
pub const IN_FLIGHT_MAX: usize = 12;

/// Dopo quanto tempo dal WinKeyer non piu' occupato i caratteri senza eco si danno per partiti.
const IDLE_GRACE: Duration = Duration::from_millis(400);

/// Un pezzo di testo nel buffer del WinKeyer: quanti caratteri d'eco mancano e il testo originale.
struct InFlight {
    missing: usize,
    text: String,
}

/// Il WinKeyer aperto in host mode.
pub struct WinKeyer {
    link: Box<dyn KeyerLink>,
    version: u8,
    cfg: WinKeyerConfig,
    wpm: u8,
    in_flight: VecDeque<InFlight>,
    status: u8,
    pot_wpm: Option<u8>,
    idle_since: Option<Instant>,
}

impl WinKeyer {
    /// Apre il WinKeyer: prova d'eco, host open (risponde con la versione), impostazioni e velocita'.
    pub fn open(mut link: Box<dyn KeyerLink>, cfg: &WinKeyerConfig, wpm: u8) -> Result<WinKeyer, String> {
        link.write(&cmd::SYNC)?;
        // quello che era rimasto nella porta non conta
        let _ = link.read(Duration::from_millis(50))?;
        link.write(&cmd::echo_test(0x55))?;
        if !wait_for(&mut *link, Duration::from_millis(800), |b| b == 0x55)?.is_some() {
            return Err("il WinKeyer non risponde: controlla la porta e che sia acceso".into());
        }
        link.write(&cmd::HOST_OPEN)?;
        let version = wait_for(&mut *link, Duration::from_millis(1000), |b| b < 0x80)?.ok_or("il WinKeyer non ha risposto all'apertura (host open)")?;
        let mut k = WinKeyer { link, version, cfg: cfg.clamped(), wpm, in_flight: VecDeque::new(), status: 0xC0, pot_wpm: None, idle_since: None };
        let bytes = cmd::configure(&k.cfg, wpm);
        k.link.write(&bytes)?;
        tracing::info!("WinKeyer aperto sulla {} (firmware {})", k.cfg.port, version);
        Ok(k)
    }

    /// La versione del firmware (23 = WK2.3, 31 = WK3.1).
    pub fn version(&self) -> u8 {
        self.version
    }

    pub fn config(&self) -> &WinKeyerConfig {
        &self.cfg
    }

    /// Nuove impostazioni (stessa porta): si rimandano tutte.
    pub fn reconfigure(&mut self, cfg: &WinKeyerConfig, wpm: u8) -> Result<(), String> {
        self.cfg = cfg.clamped();
        self.wpm = wpm;
        let bytes = cmd::configure(&self.cfg, wpm);
        self.link.write(&bytes)
    }

    /// Cambia la velocita' (se non comanda il potenziometro).
    pub fn set_wpm(&mut self, wpm: u8) -> Result<(), String> {
        self.wpm = wpm;
        if self.cfg.use_pot { Ok(()) } else { self.link.write(&cmd::speed(wpm)) }
    }

    /// La velocita' letta dal potenziometro, se e' arrivata.
    pub fn pot_wpm(&self) -> Option<u8> {
        self.pot_wpm
    }

    /// Accoda un pezzo di testo nel buffer del WinKeyer. Un pezzo senza caratteri validi risulta subito partito al prossimo `poll`.
    pub fn send(&mut self, text: &str) -> Result<(), String> {
        let (bytes, echoes) = encode(text);
        if !bytes.is_empty() {
            self.link.write(&bytes)?;
            self.idle_since = None;
        }
        self.in_flight.push_back(InFlight { missing: echoes, text: text.to_string() });
        Ok(())
    }

    /// Quanti caratteri sono nel buffer del WinKeyer senza eco.
    pub fn in_flight(&self) -> usize {
        self.in_flight.iter().map(|p| p.missing).sum()
    }

    /// Si puo' mandare altro testo: buffer non quasi pieno e pochi caratteri in attesa d'eco.
    pub fn ready(&self) -> bool {
        self.status & status::XOFF == 0 && self.in_flight() < IN_FLIGHT_MAX
    }

    /// Il WinKeyer sta trasmettendo o ha testo da mandare.
    pub fn busy(&self) -> bool {
        !self.in_flight.is_empty() || self.status & status::BUSY != 0
    }

    pub fn status(&self) -> u8 {
        self.status
    }

    /// Il paddle e' stato toccato mentre il WinKeyer trasmetteva il buffer.
    pub fn paddle_break_in(&self) -> bool {
        self.status & status::BREAKIN != 0
    }

    /// Legge quello che e' arrivato (eco, stato, potenziometro) e restituisce i pezzi di testo partiti, in ordine.
    pub fn poll(&mut self, wait: Duration) -> Result<Vec<String>, String> {
        let bytes = self.link.read(wait)?;
        let mut done = Vec::new();
        self.take_empty(&mut done);
        for b in bytes {
            match b {
                0xC0..=0xFF => {
                    self.status = b;
                    self.idle_since = if b & (status::BUSY | status::KEYDOWN) == 0 { Some(Instant::now()) } else { None };
                }
                0x80..=0xBF => self.pot_wpm = Some(self.cfg.pot_min_wpm.saturating_add(b & 0x3F)),
                _ => {
                    self.idle_since = None;
                    if let Some(front) = self.in_flight.front_mut() {
                        front.missing = front.missing.saturating_sub(1);
                    }
                    self.take_empty(&mut done);
                }
            }
        }
        // occupato finito ma eco mancante: tutto partito
        if self.idle_since.is_some_and(|t| t.elapsed() >= IDLE_GRACE) {
            done.extend(self.in_flight.drain(..).map(|p| p.text));
        }
        Ok(done)
    }

    fn take_empty(&mut self, done: &mut Vec<String>) {
        while self.in_flight.front().is_some_and(|p| p.missing == 0) {
            if let Some(p) = self.in_flight.pop_front() {
                done.push(p.text);
            }
        }
    }

    /// Svuota il buffer del WinKeyer (Esc, macro che riparte): la CW si ferma e il testo non ancora uscito si butta.
    pub fn clear(&mut self) -> Result<(), String> {
        self.in_flight.clear();
        self.link.write(&cmd::CLEAR_BUFFER)
    }

    /// Byte di comando scritti cosi' come sono (macro `<WKHEX:...>`).
    pub fn write_raw(&mut self, bytes: &[u8]) -> Result<(), String> {
        self.link.write(bytes)
    }

    /// Tasto giu' o su (Tune).
    pub fn key(&mut self, down: bool) -> Result<(), String> {
        self.link.write(&cmd::key_immediate(down))
    }

    /// Prova: manda «VVV».
    pub fn test(&mut self) -> Result<(), String> {
        self.send("VVV")
    }
}

impl Drop for WinKeyer {
    fn drop(&mut self) {
        let _ = self.link.write(&cmd::CLEAR_BUFFER);
        let _ = self.link.write(&cmd::HOST_CLOSE);
    }
}

/// Legge finche' arriva un byte che va bene (gli altri si scartano) o finisce il tempo.
fn wait_for(link: &mut dyn KeyerLink, limit: Duration, ok: impl Fn(u8) -> bool) -> Result<Option<u8>, String> {
    let t0 = Instant::now();
    while t0.elapsed() < limit {
        let left = limit.saturating_sub(t0.elapsed()).min(Duration::from_millis(100));
        if let Some(b) = link.read(left)?.into_iter().find(|&b| ok(b)) {
            return Ok(Some(b));
        }
    }
    Ok(None)
}

/// Apre il WinKeyer sulla porta delle impostazioni.
pub fn open_serial(cfg: &WinKeyerConfig, wpm: u8) -> Result<WinKeyer, String> {
    let link = serial::open(&cfg.port)?;
    WinKeyer::open(Box::new(link), cfg, wpm)
}

#[cfg(windows)]
mod serial {
    use std::{
        io::{Read, Write},
        os::windows::io::AsRawHandle,
        time::Duration,
    };

    use windows::Win32::{
        Devices::Communication::{COMMTIMEOUTS, DCB, GetCommState, NOPARITY, PURGE_RXCLEAR, PURGE_TXCLEAR, PurgeComm, SetCommState, SetCommTimeouts, TWOSTOPBITS},
        Foundation::HANDLE,
    };

    /// La porta seriale a 1200 baud 8N2, DTR alto (alcuni WinKeyer USB lo vogliono), RTS basso, senza controllo di flusso.
    pub struct SerialLink {
        file: std::fs::File,
    }

    fn handle(f: &std::fs::File) -> HANDLE {
        HANDLE(f.as_raw_handle())
    }

    pub fn open(port: &str) -> Result<SerialLink, String> {
        let name = port.trim().trim_start_matches(r"\\.\").to_ascii_uppercase();
        let digits = name.get(3..).unwrap_or("");
        if !(name.starts_with("COM") && !digits.is_empty() && digits.chars().all(|c| c.is_ascii_digit())) {
            return Err(format!("porta del WinKeyer non valida: {port:?}"));
        }
        let path = format!(r"\\.\{name}");
        let file = crate::busy_port::open_with_diagnosis(&path, || std::fs::OpenOptions::new().read(true).write(true).open(&path), |e| e.to_string()).map_err(|(e, diagnosis)| match diagnosis {
            Some(d) => d.message,
            None => format!("porta {name} non apribile: {e}"),
        })?;
        let h = handle(&file);
        let mut dcb = DCB { DCBlength: std::mem::size_of::<DCB>() as u32, ..DCB::default() };
        // SAFETY: la maniglia e' valida finche' vive `file`; `dcb` e' una struttura nostra.
        unsafe { GetCommState(h, &mut dcb) }.map_err(|e| format!("porta {name}: impostazioni non leggibili: {e}"))?;
        dcb.BaudRate = 1200;
        dcb.ByteSize = 8;
        dcb.Parity = NOPARITY;
        dcb.StopBits = TWOSTOPBITS;
        // fBinary (bit 0), DTR acceso (bit 4-5 = 1), RTS spento (bit 12-13 = 0), niente controllo di flusso ne' XON/XOFF
        dcb._bitfield = 0x0001 | (1 << 4);
        // SAFETY: come sopra.
        unsafe { SetCommState(h, &dcb) }.map_err(|e| format!("porta {name}: 1200 baud 8N2 non impostabile: {e}"))?;
        // SAFETY: come sopra.
        let _ = unsafe { PurgeComm(h, PURGE_RXCLEAR | PURGE_TXCLEAR) };
        Ok(SerialLink { file })
    }

    impl super::KeyerLink for SerialLink {
        fn write(&mut self, bytes: &[u8]) -> Result<(), String> {
            self.file.write_all(bytes).map_err(|e| format!("WinKeyer: scrittura non riuscita: {e}"))
        }

        fn read(&mut self, wait: Duration) -> Result<Vec<u8>, String> {
            // torna subito con quello che c'e', o aspetta al massimo `wait` il primo byte
            let timeouts = COMMTIMEOUTS { ReadIntervalTimeout: u32::MAX, ReadTotalTimeoutMultiplier: u32::MAX, ReadTotalTimeoutConstant: (wait.as_millis() as u32).max(1), WriteTotalTimeoutMultiplier: 0, WriteTotalTimeoutConstant: 2000 };
            // SAFETY: la maniglia e' valida finche' vive `file`.
            unsafe { SetCommTimeouts(handle(&self.file), &timeouts) }.map_err(|e| format!("WinKeyer: attese della porta non impostabili: {e}"))?;
            let mut buf = [0u8; 256];
            let n = self.file.read(&mut buf).map_err(|e| format!("WinKeyer: lettura non riuscita: {e}"))?;
            Ok(buf[..n].to_vec())
        }
    }
}

#[cfg(not(windows))]
mod serial {
    pub struct SerialLink;

    impl super::KeyerLink for SerialLink {
        fn write(&mut self, _bytes: &[u8]) -> Result<(), String> {
            Err("il WinKeyer e' disponibile solo su Windows".into())
        }
        fn read(&mut self, _wait: std::time::Duration) -> Result<Vec<u8>, String> {
            Err("il WinKeyer e' disponibile solo su Windows".into())
        }
    }

    pub fn open(_port: &str) -> Result<SerialLink, String> {
        Err("il WinKeyer e' disponibile solo su Windows".into())
    }
}

/// Un WinKeyer finto per le prove: risponde alla prova d'eco e all'apertura, ricorda i comandi e rimanda l'eco di un carattere del buffer
/// a ogni lettura, con i byte di stato «occupato» e «libero» come quello vero.
#[cfg(any(test, feature = "testing"))]
pub mod fake {
    use std::{
        collections::VecDeque,
        sync::{Arc, Mutex},
        time::Duration,
    };

    #[derive(Default, Debug)]
    pub struct FakeState {
        pub opened: bool,
        pub closed: bool,
        pub wpm: Option<u8>,
        pub weight: Option<u8>,
        pub lead_tail: Option<(u8, u8)>,
        pub mode: Option<u8>,
        pub pin_config: Option<u8>,
        pub clears: usize,
        /// Tutto il testo arrivato nel buffer (i segni uniti come `<AR>`).
        pub received: String,
        /// Il testo uscito in CW (con l'eco).
        pub keyed: String,
        /// Lettere nel buffer non ancora uscite.
        pub buffer: VecDeque<u8>,
        /// Byte da mandare al programma.
        pub out: VecDeque<u8>,
        /// Non risponde (WinKeyer spento).
        pub dead: bool,
        /// Quanti caratteri escono a ogni lettura.
        pub per_read: usize,
        busy: bool,
        pending: Vec<u8>,
    }

    /// Il collegamento finto; `state` resta in mano alla prova.
    pub struct FakeLink {
        pub state: Arc<Mutex<FakeState>>,
    }

    impl FakeLink {
        pub fn new() -> (FakeLink, Arc<Mutex<FakeState>>) {
            let state = Arc::new(Mutex::new(FakeState { per_read: 1, ..FakeState::default() }));
            (FakeLink { state: Arc::clone(&state) }, state)
        }
    }

    /// Quanti byte di parametri segue ogni comando.
    fn params(cmd: u8, admin: Option<u8>) -> usize {
        match (cmd, admin) {
            (0x00, None) => 1,
            (0x00, Some(0x04)) => 1,
            (0x00, Some(_)) => 0,
            (0x04 | 0x1B, _) => 2,
            (0x05, _) => 3,
            (0x0F, _) => 15,
            (0x07 | 0x08 | 0x0A | 0x13 | 0x15 | 0x1E | 0x1F, _) => 0,
            (0x01..=0x1F, _) => 1,
            _ => 0,
        }
    }

    impl FakeState {
        fn feed(&mut self, b: u8) {
            self.pending.push(b);
            loop {
                let Some(&c) = self.pending.first() else { return };
                if c >= 0x20 {
                    self.pending.remove(0);
                    self.received.push(c as char);
                    self.buffer.push_back(c);
                    continue;
                }
                let admin = if c == 0x00 { self.pending.get(1).copied() } else { None };
                if c == 0x00 && admin.is_none() {
                    return;
                }
                let need = 1 + usize::from(c == 0x00) + params(c, admin);
                if self.pending.len() < need {
                    return;
                }
                let p: Vec<u8> = self.pending.drain(..need).collect();
                self.command(&p);
            }
        }

        fn command(&mut self, p: &[u8]) {
            match p {
                [0x00, 0x04, b] => self.out.push_back(*b),
                [0x00, 0x02] => {
                    self.opened = true;
                    self.out.push_back(31);
                }
                [0x00, 0x03] => self.closed = true,
                [0x02, w] => self.wpm = Some(*w),
                [0x03, w] => self.weight = Some(*w),
                [0x04, l, t] => self.lead_tail = Some((*l, *t)),
                [0x0E, m] => self.mode = Some(*m),
                [0x09, m] => self.pin_config = Some(*m),
                [0x0A] => {
                    self.clears += 1;
                    self.buffer.clear();
                }
                [0x15] => self.out.push_back(0xC0 | if self.busy { 0x04 } else { 0 }),
                [0x1B, a, b] => {
                    self.received.push_str(&format!("<{}{}>", *a as char, *b as char));
                    self.buffer.push_back(*a);
                    self.buffer.push_back(*b);
                }
                _ => {}
            }
        }

        /// Un passo del tempo: escono `per_read` caratteri dal buffer con la loro eco.
        fn step(&mut self) {
            if !self.opened {
                return;
            }
            for _ in 0..self.per_read {
                match self.buffer.pop_front() {
                    Some(c) => {
                        if !self.busy {
                            self.busy = true;
                            self.out.push_back(0xC4);
                        }
                        self.keyed.push(c as char);
                        self.out.push_back(c);
                    }
                    None => {
                        if self.busy {
                            self.busy = false;
                            self.out.push_back(0xC0);
                        }
                        break;
                    }
                }
            }
        }
    }

    impl super::KeyerLink for FakeLink {
        fn write(&mut self, bytes: &[u8]) -> Result<(), String> {
            let mut s = self.state.lock().map_err(|_| "stato finto")?;
            if s.dead {
                return Ok(());
            }
            for &b in bytes {
                s.feed(b);
            }
            Ok(())
        }

        fn read(&mut self, wait: Duration) -> Result<Vec<u8>, String> {
            let out: Vec<u8> = {
                let mut s = self.state.lock().map_err(|_| "stato finto")?;
                if !s.dead {
                    s.step();
                }
                s.out.drain(..).collect()
            };
            if out.is_empty() {
                std::thread::sleep(wait.min(Duration::from_millis(5)));
            }
            Ok(out)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::fake::FakeLink;
    use super::*;

    fn open() -> (WinKeyer, std::sync::Arc<std::sync::Mutex<fake::FakeState>>) {
        let (link, state) = FakeLink::new();
        let cfg = WinKeyerConfig { port: "COM9".into(), ..WinKeyerConfig::default() };
        let k = WinKeyer::open(Box::new(link), &cfg, 22).expect("apertura");
        (k, state)
    }

    fn drain(k: &mut WinKeyer) -> String {
        let mut out = String::new();
        for _ in 0..200 {
            out.extend(k.poll(Duration::from_millis(1)).expect("poll"));
            if !k.busy() {
                break;
            }
        }
        out
    }

    #[test]
    fn l_apertura_fa_la_prova_d_eco_e_host_open_e_manda_le_impostazioni() {
        let (k, state) = open();
        assert_eq!(k.version(), 31);
        let s = state.lock().expect("stato");
        assert!(s.opened);
        assert_eq!(s.wpm, Some(22));
        assert_eq!(s.weight, Some(50));
        assert_eq!(s.lead_tail, Some((5, 10)));
        // Iambic B, eco dei caratteri acceso
        assert_eq!(s.mode, Some(0b0000_0100));
        // PTT, sidetone, tasto
        assert_eq!(s.pin_config, Some(0b1111));
    }

    #[test]
    fn un_winkeyer_che_non_risponde_da_errore() {
        let (link, state) = FakeLink::new();
        state.lock().expect("stato").dead = true;
        let r = WinKeyer::open(Box::new(link), &WinKeyerConfig::default(), 20);
        assert!(r.is_err());
    }

    #[test]
    fn il_testo_risulta_partito_con_l_eco() {
        let (mut k, state) = open();
        for t in ["C", "Q", " ", "<SK>", "é"] {
            k.send(t).expect("invio");
        }
        assert_eq!(drain(&mut k), "CQ <SK>é");
        let s = state.lock().expect("stato");
        assert_eq!(s.received, "CQ <SK>");
        assert_eq!(s.keyed, "CQ SK");
    }

    #[test]
    fn svuotare_ferma_la_cw_e_butta_il_testo() {
        let (mut k, state) = open();
        for c in "CQ CQ CQ DE IU3ABC".chars() {
            k.send(&c.to_string()).expect("invio");
        }
        let first = k.poll(Duration::from_millis(1)).expect("poll");
        assert_eq!(first.concat(), "C");
        k.clear().expect("svuota");
        assert_eq!(k.in_flight(), 0);
        let s = state.lock().expect("stato");
        assert_eq!(s.clears, 1);
        assert!(s.buffer.is_empty());
    }

    #[test]
    fn la_velocita_cambia_e_col_potenziometro_vale_zero() {
        let (mut k, state) = open();
        k.set_wpm(30).expect("velocita'");
        assert_eq!(state.lock().expect("stato").wpm, Some(30));
        let cfg = WinKeyerConfig { use_pot: true, ..k.config().clone() };
        k.reconfigure(&cfg, 30).expect("impostazioni");
        assert_eq!(state.lock().expect("stato").wpm, Some(0));
        k.set_wpm(35).expect("velocita'");
        assert_eq!(state.lock().expect("stato").wpm, Some(0));
    }

    #[test]
    fn i_byte_del_potenziometro_e_dello_stato_non_sono_eco() {
        let (mut k, state) = open();
        state.lock().expect("stato").out.extend([0x80 | 12, 0xC1]);
        k.send("E").expect("invio");
        let _ = k.poll(Duration::from_millis(1)).expect("poll");
        assert_eq!(k.pot_wpm(), Some(10 + 12));
        assert_ne!(k.status() & status::BUSY, 0);
    }

    #[test]
    fn la_codifica_toglie_i_caratteri_sconosciuti_e_unisce_i_prosign() {
        assert_eq!(encode("cq <ar>\n"), (vec![b'C', b'Q', b' ', 0x1B, b'A', b'R', b' '], 6));
        assert_eq!(encode("~#"), (vec![], 0));
        assert_eq!(cmd::sidetone(800), [0x01, 5]);
        assert_eq!(cmd::ptt_lead_tail(3000, 15), [0x04, 250, 1]);
        assert_eq!(cmd::mode(PaddleMode::Ultimatic, true, true, false), [0x0E, 0b0010_1110]);
    }

    #[test]
    fn chiudendo_si_svuota_e_si_esce_dall_host_mode() {
        let (k, state) = open();
        drop(k);
        let s = state.lock().expect("stato");
        assert!(s.closed);
        assert!(s.clears >= 1);
    }
}
