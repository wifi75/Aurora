//! Ricevitore dei QSO dagli altri programmi di log: un ascoltatore UDP che riconosce dal contenuto il record ADIF testuale, l'XML
//! `contactinfo`/`contactreplace` di N1MM Logger+ e i messaggi QSO Logged (5) e Logged ADIF (12) del flusso binario di WSJT-X.
//!
//! Ogni QSO ricevuto va solo nel log di Aurora ([`Logbook::add_received`]): non passa dal bus (niente `Event::QsoLogged`), quindi non
//! riparte verso i destinatari UDP, il file ADIF, il TCP o i servizi in rete. Cosi' il log che lo ha mandato non lo riceve di ritorno.
//! Lo stesso QSO gia' presente (nominativo, banda, modo, inizio entro [`DUPLICATE_WINDOW_SECS`]) non si aggiunge una seconda volta.

use std::{
    collections::HashMap,
    net::{IpAddr, Ipv4Addr, SocketAddr},
    sync::{Arc, Mutex},
    time::{SystemTime, UNIX_EPOCH},
};

use aurora_core::{
    adif::{band_from_freq_hz, parse_records, unix_ms, AdifQso},
    QsoRecord,
};
use logbook::{Logbook, Received};
use tokio::{net::UdpSocket, sync::watch};

/// Porta di ascolto predefinita: libera rispetto a quelle che Aurora usa per l'invio (2237 flusso WSJT-X, 2333 ADIF su UDP, 12060 XML N1MM)
/// e per i comandi (2334).
pub const DEFAULT_PORT: u16 = 12062;
/// Lo stesso QSO se nominativo, banda e modo coincidono e l'inizio differisce di non piu' di 5 minuti.
pub const DUPLICATE_WINDOW_SECS: u64 = 300;
/// Datagrammi piu' grandi si scartano senza leggerli.
pub const MAX_DATAGRAM: usize = 64 * 1024;
/// Al piu' tanti QSO da un datagramma (gli altri si scartano).
pub const MAX_RECORDS: usize = 200;

/// Come ascoltare: spento, oppure porta e indirizzo (solo questo PC o anche la rete di casa).
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct InboundConfig {
    pub enabled: bool,
    pub port: u16,
    /// Vero: 0.0.0.0 (anche gli altri PC della rete di casa); falso: 127.0.0.1.
    pub lan: bool,
    /// Porte che Aurora usa per mandare i QSO su questo PC: ascoltare li' vorrebbe dire rileggere i propri QSO.
    pub forbidden: Vec<u16>,
}

/// Lo stato per la pagina delle impostazioni.
#[derive(Clone, Debug, Default, PartialEq, Eq, serde::Serialize)]
pub struct InboundStats {
    pub enabled: bool,
    /// L'indirizzo su cui si ascolta davvero (`None` = spento o errore).
    pub listening: Option<String>,
    pub port: u16,
    /// Perche' non si ascolta: `busy` (porta occupata), `own` (porta usata da Aurora per l'invio), altro testo = errore del sistema.
    pub error: Option<String>,
    /// QSO aggiunti al log.
    pub received: u64,
    /// QSO gia' presenti (eventualmente completati con campi che mancavano).
    pub duplicates: u64,
    /// Datagrammi o record non riconosciuti.
    pub discarded: u64,
    /// Nominativo e fonte dell'ultimo QSO aggiunto.
    pub last: Option<String>,
}

/// Cosa contiene un datagramma.
#[derive(Debug, Clone, PartialEq)]
pub enum Parsed {
    /// QSO trovati, con il nome del programma che li manda (vuoto se non si sa) e quanti record non erano QSO validi.
    Qsos { qsos: Vec<QsoRecord>, source: String, bad: usize },
    /// Messaggio noto che non porta QSO (Heartbeat, Status e Decode di WSJT-X, RadioInfo e spot di N1MM...): si ignora in silenzio.
    Ignored,
    /// Contenuto non riconosciuto.
    Unknown,
}

fn now_ms() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |d| d.as_millis() as u64)
}

/// Il modo come lo conta il log: `MFSK`+sottomodo vale il sottomodo, `USB`/`LSB` sono `SSB`.
fn normalize_mode(mode: &str) -> String {
    let m = mode.trim().to_uppercase();
    match m.as_str() {
        "USB" | "LSB" => "SSB".to_string(),
        _ => m,
    }
}

/// La banda da una frequenza in Hz (con un margine: «14» MHz di N1MM e' l'inizio dei 20 m).
fn band_of(hz: u64) -> String {
    if hz == 0 {
        return String::new();
    }
    band_from_freq_hz(hz.saturating_add(1000)).unwrap_or_default().to_string()
}

/// Riconosce il contenuto di un datagramma. Non va mai in panico e non legge oltre i limiti, qualunque sia l'ingresso.
pub fn parse_datagram(data: &[u8], now_ms: u64) -> Parsed {
    if data.is_empty() || data.len() > MAX_DATAGRAM {
        return Parsed::Unknown;
    }
    if data.len() >= 4 && u32::from_be_bytes([data[0], data[1], data[2], data[3]]) == udp_bridge::MAGIC {
        return parse_wsjtx(data, now_ms);
    }
    let text = String::from_utf8_lossy(data);
    let lower = text.to_ascii_lowercase();
    if lower.contains("<contactinfo") || lower.contains("<contactreplace") {
        return match n1mm_qso(&text, now_ms) {
            Some((q, app)) => Parsed::Qsos { qsos: vec![q], source: if app.is_empty() { "N1MM XML".into() } else { app }, bad: 0 },
            None => Parsed::Qsos { qsos: Vec::new(), source: String::new(), bad: 1 },
        };
    }
    let trimmed = lower.trim_start();
    if trimmed.starts_with("<?xml") || ["<radioinfo", "<spot", "<dynamicresults", "<contactdelete", "<appinfo", "<lookupinfo"].iter().any(|t| trimmed.starts_with(t)) {
        return Parsed::Ignored;
    }
    if lower.contains("<call:") {
        return adif_qsos(&text, now_ms, "");
    }
    Parsed::Unknown
}

fn parse_wsjtx(data: &[u8], now_ms: u64) -> Parsed {
    use udp_bridge::Message;
    let Ok(packet) = udp_bridge::decode(data) else { return Parsed::Unknown };
    match packet.message {
        Message::QsoLogged { id, dx_call, dx_grid, tx_freq, mode, report_sent, report_received, extra } => {
            let call = dx_call.unwrap_or_default().trim().to_uppercase();
            let band = band_of(tx_freq);
            if call.is_empty() || band.is_empty() {
                return Parsed::Qsos { qsos: Vec::new(), source: String::new(), bad: 1 };
            }
            let extra = extra.unwrap_or_default();
            let ms = |v: Option<i64>| v.filter(|t| *t > 0).map(|t| t as u64);
            let off = ms(extra.time_off_ms);
            let on = ms(extra.time_on_ms).or(off).unwrap_or(now_ms);
            let q = QsoRecord {
                call,
                grid: dx_grid.unwrap_or_default().trim().to_string(),
                band,
                mode: normalize_mode(&mode.unwrap_or_default()),
                freq_hz: tx_freq,
                rst_sent: report_sent.unwrap_or_default().trim().to_string(),
                rst_rcvd: report_received.unwrap_or_default().trim().to_string(),
                time_ms: (on % 86_400_000) as u32,
                time_on_unix_ms: on,
                time_off_unix_ms: off.unwrap_or(on),
                name: extra.name.unwrap_or_default().trim().to_string(),
                tx_power: extra.tx_power.unwrap_or_default().trim().to_string(),
                comment: extra.comments.unwrap_or_default().trim().to_string(),
                exch_sent: extra.exchange_sent.unwrap_or_default().trim().to_string(),
                exch_rcvd: extra.exchange_received.unwrap_or_default().trim().to_string(),
                ..Default::default()
            };
            Parsed::Qsos { qsos: vec![q], source: id.unwrap_or_default(), bad: 0 }
        }
        Message::LoggedAdif { id, adif } => adif_qsos(&adif.unwrap_or_default(), now_ms, &id.unwrap_or_default()),
        _ => Parsed::Ignored,
    }
}

/// I QSO di un testo ADIF: uno o piu' record, con o senza intestazione, `<eor>` in qualunque forma; l'ultimo record senza `<eor>` vale se
/// ha il nominativo. La fonte e' `PROGRAMID` dell'intestazione se c'e', altrimenti `fallback`.
fn adif_qsos(text: &str, now_ms: u64, fallback: &str) -> Parsed {
    let lower = text.to_ascii_lowercase();
    let mut body = text.to_string();
    let after_eor = lower.rfind("<eor>").map_or(0, |p| p + 5);
    if lower[after_eor..].contains("<call:") {
        body.push_str("<eor>");
    }
    let program = lower.find("<eoh>").and_then(|h| header_field(text.get(..h).unwrap_or(""), "programid")).unwrap_or_default();
    let records = parse_records(&body);
    let mut qsos = Vec::new();
    let mut bad = 0;
    for r in records.iter() {
        if qsos.len() >= MAX_RECORDS {
            bad += 1;
            continue;
        }
        match adif_record(r, now_ms) {
            Some(q) => qsos.push(q),
            None => bad += 1,
        }
    }
    if qsos.is_empty() && bad == 0 {
        return Parsed::Unknown;
    }
    let source = if program.is_empty() { fallback.to_string() } else { program };
    Parsed::Qsos { qsos, source: if source.is_empty() { "ADIF".into() } else { source }, bad }
}

/// Un campo `<NOME:n>valore` dell'intestazione ADIF.
fn header_field(header: &str, name: &str) -> Option<String> {
    parse_records(&format!("{header}<eor>")).into_iter().next()?.get(&name.to_uppercase()).cloned().filter(|v| !v.is_empty())
}

fn adif_record(r: &HashMap<String, String>, now_ms: u64) -> Option<QsoRecord> {
    let a = AdifQso::from_fields(r)?;
    let get = |k: &str| r.get(k).map(|v| v.trim().to_string()).unwrap_or_default();
    let on = if a.date.is_empty() { now_ms } else { unix_ms(&a.date, if a.time_on.is_empty() { "000000" } else { &a.time_on })? };
    let off = if a.time_off.is_empty() { on } else { unix_ms(if a.date_off.is_empty() { &a.date } else { &a.date_off }, &a.time_off).unwrap_or(on) };
    let pick = |a: &str, b: &str| {
        let first = get(a);
        if first.is_empty() { get(b) } else { first }
    };
    Some(QsoRecord {
        call: a.call,
        grid: a.grid,
        band: a.band,
        mode: normalize_mode(&a.mode),
        freq_hz: a.freq_hz,
        rst_sent: a.rst_sent,
        rst_rcvd: a.rst_rcvd,
        time_ms: (on % 86_400_000) as u32,
        time_on_unix_ms: on,
        time_off_unix_ms: off,
        name: get("NAME"),
        qth: get("QTH"),
        tx_power: get("TX_PWR"),
        comment: get("COMMENT"),
        exch_sent: pick("STX_STRING", "STX"),
        exch_rcvd: pick("SRX_STRING", "SRX"),
        state: get("STATE"),
        ..Default::default()
    })
}

/// Il testo di un elemento XML semplice (`<nome>valore</nome>`), con le entita' sciolte; nomi senza badare alle maiuscole.
fn xml_field(xml: &str, lower: &str, name: &str) -> String {
    let open = format!("<{}>", name.to_ascii_lowercase());
    let close = format!("</{}>", name.to_ascii_lowercase());
    let Some(start) = lower.find(&open).map(|p| p + open.len()) else { return String::new() };
    let Some(len) = lower.get(start..).and_then(|rest| rest.find(&close)) else { return String::new() };
    let raw = xml.get(start..start + len).unwrap_or("");
    raw.replace("&lt;", "<").replace("&gt;", ">").replace("&quot;", "\"").replace("&apos;", "'").replace("&amp;", "&").trim().to_string()
}

/// Il QSO di un messaggio `contactinfo` (o `contactreplace`) di N1MM Logger+, e il programma che lo manda (`app`).
fn n1mm_qso(xml: &str, now_ms: u64) -> Option<(QsoRecord, String)> {
    // le entita' e i tag sono ASCII: le posizioni nel testo in minuscolo valgono anche nell'originale
    let lower = xml.to_ascii_lowercase();
    let f = |name: &str| xml_field(xml, &lower, name);
    let call = f("call").to_uppercase();
    if call.is_empty() {
        return None;
    }
    let tens = |name: &str| f(name).parse::<f64>().ok().filter(|v| v.is_finite() && *v > 0.0 && *v < 1e12).map_or(0, |v| (v * 10.0).round() as u64);
    let freq_hz = match tens("rxfreq") {
        0 => tens("txfreq"),
        hz => hz,
    };
    let mut band = band_of(freq_hz);
    if band.is_empty() {
        let mhz = f("band").parse::<f64>().ok().filter(|v| v.is_finite() && *v > 0.0 && *v < 1e6).unwrap_or(0.0);
        band = band_of((mhz * 1e6) as u64);
    }
    if band.is_empty() {
        return None;
    }
    // «2026-10-24 16:47:29»
    let stamp = f("timestamp");
    let digits: String = stamp.chars().filter(char::is_ascii_digit).collect();
    let at = if digits.len() >= 12 { unix_ms(&digits[0..8], &format!("{:0<6}", &digits[8..digits.len().min(14)])) } else { None }.unwrap_or(now_ms);
    let exch_rcvd = {
        let e = f("exchange1");
        if e.is_empty() { f("rcvnr").trim_start_matches('0').to_string() } else { e }
    };
    let sntnr = f("sntnr");
    let q = QsoRecord {
        call,
        grid: f("gridsquare"),
        band,
        mode: normalize_mode(&f("mode")),
        freq_hz,
        rst_sent: f("snt"),
        rst_rcvd: f("rcv"),
        time_ms: (at % 86_400_000) as u32,
        time_on_unix_ms: at,
        time_off_unix_ms: at,
        name: f("name"),
        qth: f("qth"),
        tx_power: f("power"),
        comment: f("comment"),
        exch_sent: if sntnr.trim_start_matches('0').is_empty() { String::new() } else { sntnr },
        exch_rcvd,
        state: f("state"),
        ..Default::default()
    };
    Some((q, f("app")))
}

/// Il ricevitore in funzione: lo stato si legge con [`Inbound::stats`].
#[derive(Clone)]
pub struct Inbound {
    stats: Arc<Mutex<InboundStats>>,
}

impl Inbound {
    /// Va chiamato dentro un runtime Tokio. `config` si puo' cambiare a programma acceso: l'ascoltatore si riapre. `changed` si chiama dopo
    /// ogni QSO aggiunto o completato (per aggiornare i gia' lavorati); mai per i QSO gia' presenti.
    pub fn spawn(book: Arc<Logbook>, mut config: watch::Receiver<InboundConfig>, changed: Arc<dyn Fn() + Send + Sync>) -> Inbound {
        let stats = Arc::new(Mutex::new(InboundStats::default()));
        let shared = Arc::clone(&stats);
        tokio::spawn(async move {
            loop {
                let wanted = config.borrow_and_update().clone();
                let socket = open(&wanted, &shared).await;
                match socket {
                    Some(socket) => {
                        let mut buf = vec![0u8; MAX_DATAGRAM + 1];
                        loop {
                            tokio::select! {
                                c = config.changed() => {
                                    if c.is_err() { return; }
                                    if *config.borrow() != wanted { break; }
                                }
                                r = socket.recv_from(&mut buf) => {
                                    match r {
                                        Ok((n, from)) => handle(&book, &shared, &changed, buf.get(..n).unwrap_or(&[]), from),
                                        // su Windows un datagramma troppo grande o una porta remota chiusa danno un errore: si continua
                                        Err(e) => tracing::debug!("ricevitore dei QSO: {e}"),
                                    }
                                }
                            }
                        }
                    }
                    None => {
                        if config.changed().await.is_err() {
                            return;
                        }
                    }
                }
            }
        });
        Inbound { stats }
    }

    pub fn stats(&self) -> InboundStats {
        self.stats.lock().map(|s| s.clone()).unwrap_or_default()
    }
}

/// Apre l'ascoltatore come chiesto e scrive lo stato; `None` se spento o se non si puo'.
async fn open(c: &InboundConfig, stats: &Mutex<InboundStats>) -> Option<UdpSocket> {
    let set = |listening: Option<String>, error: Option<String>| {
        if let Ok(mut s) = stats.lock() {
            s.enabled = c.enabled;
            s.port = c.port;
            s.listening = listening;
            s.error = error;
        }
    };
    if !c.enabled || c.port == 0 {
        set(None, None);
        return None;
    }
    if c.forbidden.contains(&c.port) {
        tracing::warn!("ricevitore dei QSO: la porta {} e' quella su cui Aurora manda i QSO, non si ascolta", c.port);
        set(None, Some("own".into()));
        return None;
    }
    let ip = if c.lan { IpAddr::V4(Ipv4Addr::UNSPECIFIED) } else { IpAddr::V4(Ipv4Addr::LOCALHOST) };
    let addr = SocketAddr::new(ip, c.port);
    // niente SO_REUSEADDR: se un altro programma ha gia' la porta l'apertura fallisce e si vede
    match UdpSocket::bind(addr).await {
        Ok(socket) => {
            tracing::info!("ricevitore dei QSO in ascolto su {addr}");
            set(Some(addr.to_string()), None);
            Some(socket)
        }
        Err(e) => {
            tracing::warn!("ricevitore dei QSO: porta {addr} non disponibile: {e}");
            let busy = e.kind() == std::io::ErrorKind::AddrInUse || e.raw_os_error() == Some(10048) || e.raw_os_error() == Some(10013);
            set(None, Some(if busy { "busy".into() } else { e.to_string() }));
            None
        }
    }
}

/// Un datagramma arrivato: QSO nel log, contatori e una riga nel registro.
fn handle(book: &Logbook, stats: &Mutex<InboundStats>, changed: &Arc<dyn Fn() + Send + Sync>, data: &[u8], from: SocketAddr) {
    let bump = |f: &dyn Fn(&mut InboundStats)| {
        if let Ok(mut s) = stats.lock() {
            f(&mut s);
        }
    };
    let (qsos, source, bad) = match parse_datagram(data, now_ms()) {
        Parsed::Qsos { qsos, source, bad } => (qsos, source, bad),
        Parsed::Ignored => return,
        Parsed::Unknown => {
            tracing::info!("ricevitore dei QSO: datagramma di {} byte da {from} non riconosciuto, scartato", data.len());
            bump(&|s| s.discarded += 1);
            return;
        }
    };
    if bad > 0 {
        tracing::info!("ricevitore dei QSO: {bad} record da {from} senza nominativo, banda o data valida, scartati");
        bump(&|s| s.discarded += bad as u64);
    }
    let source: String = source.chars().filter(|c| !c.is_control()).take(40).collect();
    let mut touched = false;
    for q in qsos {
        match book.add_received(&q, DUPLICATE_WINDOW_SECS, &source) {
            Ok(Received::Added(_)) => {
                tracing::info!("QSO con {} {} {} ricevuto da {source} ({from}): aggiunto al log", q.call, q.band, q.mode);
                let last = format!("{} ({source})", q.call);
                bump(&|s| {
                    s.received += 1;
                    s.last = Some(last.clone());
                });
                touched = true;
            }
            Ok(Received::Completed(_)) => {
                tracing::info!("QSO con {} ricevuto da {source}: gia' presente, completato con i campi che mancavano", q.call);
                bump(&|s| s.duplicates += 1);
                touched = true;
            }
            Ok(Received::Present(_)) => {
                tracing::info!("QSO con {} ricevuto da {source}: gia' presente nel log", q.call);
                bump(&|s| s.duplicates += 1);
            }
            Err(e) => {
                tracing::warn!("QSO con {} ricevuto da {source} non salvato: {e}", q.call);
                bump(&|s| s.discarded += 1);
            }
        }
    }
    if touched {
        changed();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{n1mm_contact_info, Exporter, Target};
    use logbook::adif::Station;

    const NOW: u64 = 1_794_000_000_000;

    fn qsos(p: Parsed) -> Vec<QsoRecord> {
        match p {
            Parsed::Qsos { qsos, .. } => qsos,
            other => panic!("niente QSO: {other:?}"),
        }
    }

    #[test]
    fn one_adif_record_without_header() {
        let text = "<call:5>K1ABC <gridsquare:4>FN42 <mode:3>FT8 <rst_sent:3>-10 <rst_rcvd:3>-12 <qso_date:8>20261107 <time_on:6>120015 <time_off:6>120115 <band:3>20m <freq:9>14.075084 <name:3>Joe <comment:4>ciao <eor>";
        let q = qsos(parse_datagram(text.as_bytes(), NOW));
        assert_eq!(q.len(), 1);
        let q = &q[0];
        assert_eq!((q.call.as_str(), q.band.as_str(), q.mode.as_str(), q.grid.as_str()), ("K1ABC", "20m", "FT8", "FN42"));
        assert_eq!((q.rst_sent.as_str(), q.rst_rcvd.as_str(), q.name.as_str(), q.comment.as_str()), ("-10", "-12", "Joe", "ciao"));
        assert_eq!(q.freq_hz, 14_075_084);
        assert_eq!(q.time_on_unix_ms, unix_ms("20261107", "120015").unwrap_or(0));
        assert_eq!(q.time_off_unix_ms, unix_ms("20261107", "120115").unwrap_or(0));
    }

    #[test]
    fn several_records_with_header_and_any_eor_case() {
        let text = "Log4OM export\n<adif_ver:5>3.1.4<programid:6>Log4OM<EOH>\n<CALL:5>K1ABC<BAND:3>40M<MODE:4>MFSK<SUBMODE:3>FT4<QSO_DATE:8>20261107<TIME_ON:4>1200<EOR>\n<call:6>DL1ABC<band:3>20m<mode:3>USB<qso_date:8>20261107<time_on:6>130000<srx_string:2>15<stx:3>001<Eor>";
        let p = parse_datagram(text.as_bytes(), NOW);
        let Parsed::Qsos { qsos, source, bad } = p else { panic!("{p:?}") };
        assert_eq!((qsos.len(), source.as_str(), bad), (2, "Log4OM", 0));
        assert_eq!((qsos[0].band.as_str(), qsos[0].mode.as_str()), ("40m", "FT4"));
        assert_eq!((qsos[1].mode.as_str(), qsos[1].exch_rcvd.as_str(), qsos[1].exch_sent.as_str()), ("SSB", "15", "001"));
    }

    #[test]
    fn missing_fields_and_non_ascii() {
        // senza data: vale l'ora di ricezione; senza banda ma con la frequenza: dalla frequenza
        let name = "José Müller";
        let text = format!("<call:5>EA1AB<freq:5>7.074<mode:3>FT8<name:{}>{name}<eor>", name.len());
        let q = qsos(parse_datagram(text.as_bytes(), NOW));
        assert_eq!((q[0].band.as_str(), q[0].name.as_str(), q[0].time_on_unix_ms), ("40m", name, NOW));
        // senza nominativo o senza banda e frequenza: scartato
        let p = parse_datagram(b"<call:5>EA1AB<mode:3>FT8<eor><band:3>20m<eor>", NOW);
        assert_eq!(p, Parsed::Qsos { qsos: Vec::new(), source: "ADIF".into(), bad: 2 });
        // l'ultimo record senza <eor> vale
        assert_eq!(qsos(parse_datagram(b"<call:5>K1ABC<band:3>20m<mode:3>FT8", NOW)).len(), 1);
    }

    #[test]
    fn n1mm_contact_info_round_trip() {
        let q = QsoRecord {
            call: "K1ABC".into(),
            grid: "FN42".into(),
            band: "20m".into(),
            mode: "FT8".into(),
            freq_hz: 14_075_080,
            rst_sent: "-16".into(),
            rst_rcvd: "-09".into(),
            time_on_unix_ms: 1_792_860_405_000,
            time_off_unix_ms: 1_792_860_449_000,
            name: "Al & Bo <x>".into(),
            comment: "très bien".into(),
            ..Default::default()
        };
        let station = Station { call: "IU3ABC".into(), grid: "IO91WM".into(), tx_power: "50".into() };
        let xml = n1mm_contact_info(&q, &station, 0);
        let p = parse_datagram(xml.as_bytes(), NOW);
        let Parsed::Qsos { qsos, source, .. } = p else { panic!("{p:?}") };
        assert_eq!(source, "Aurora");
        let r = &qsos[0];
        assert_eq!((r.call.as_str(), r.band.as_str(), r.mode.as_str(), r.freq_hz), ("K1ABC", "20m", "FT8", 14_075_080));
        assert_eq!((r.rst_sent.as_str(), r.rst_rcvd.as_str(), r.grid.as_str()), ("-16", "-09", "FN42"));
        assert_eq!((r.name.as_str(), r.comment.as_str(), r.tx_power.as_str()), ("Al & Bo <x>", "très bien", "50"));
        // N1MM ha un solo orario: e' la fine del QSO di Aurora
        assert_eq!((r.time_on_unix_ms, r.time_off_unix_ms), (q.time_off_unix_ms, q.time_off_unix_ms));
        // rimesso nel log che lo ha prodotto, e' lo stesso QSO
        let book = Logbook::open_in_memory().expect("log");
        book.add(&q).expect("aggiunto");
        assert!(matches!(book.add_received(r, DUPLICATE_WINDOW_SECS, &source), Ok(Received::Present(_))));
    }

    #[test]
    fn n1mm_other_messages_are_ignored_and_replace_is_read() {
        assert_eq!(parse_datagram(b"<?xml version=\"1.0\"?><RadioInfo><Freq>1407400</Freq></RadioInfo>", NOW), Parsed::Ignored);
        let replace = "<?xml version=\"1.0\"?><contactreplace><call>K1ABC</call><band>3.5</band><mode>CW</mode><timestamp>2026-11-07 12:00:00</timestamp></contactreplace>";
        let q = qsos(parse_datagram(replace.as_bytes(), NOW));
        assert_eq!((q[0].band.as_str(), q[0].mode.as_str()), ("80m", "CW"));
    }

    fn qso_logged_datagram() -> Vec<u8> {
        let data = udp_bridge::QsoLoggedData {
            time_off_ms: Some(1_792_860_449_000),
            time_on_ms: Some(1_792_860_405_000),
            dx_call: "K1ABC".into(),
            dx_grid: "FN42".into(),
            dial_freq: 14_074_000,
            mode: "FT8".into(),
            report_sent: "-10".into(),
            report_received: "-05".into(),
            name: "Joe".into(),
            ..Default::default()
        };
        udp_bridge::encode_qso_logged("Log4OM", 3, &data)
    }

    #[test]
    fn wsjtx_qso_logged_and_logged_adif() {
        let p = parse_datagram(&qso_logged_datagram(), NOW);
        let Parsed::Qsos { qsos, source, .. } = p else { panic!("{p:?}") };
        assert_eq!(source, "Log4OM");
        assert_eq!((qsos[0].call.as_str(), qsos[0].band.as_str(), qsos[0].time_on_unix_ms), ("K1ABC", "20m", 1_792_860_405_000));
        let adif = udp_bridge::encode_logged_adif("JTDX", 3, "<call:5>K1ABC <band:3>20m <mode:3>FT8 <qso_date:8>20261107 <time_on:6>120000 <eor>");
        assert_eq!(qsos_of(parse_datagram(&adif, NOW)), 1);
        let hb = udp_bridge::encode_heartbeat("Log4OM", 3, 3, "2.0");
        assert_eq!(parse_datagram(&hb, NOW), Parsed::Ignored);
    }

    fn qsos_of(p: Parsed) -> usize {
        match p {
            Parsed::Qsos { qsos, .. } => qsos.len(),
            _ => 0,
        }
    }

    #[test]
    fn random_and_truncated_input_never_panics() {
        let samples: Vec<Vec<u8>> = vec![
            b"<call:5>K1ABC <band:3>20m <mode:3>FT8 <qso_date:8>20261107 <time_on:6>120000 <eor>".to_vec(),
            n1mm_contact_info(&QsoRecord { call: "K1ABC".into(), band: "20m".into(), freq_hz: 14_074_000, ..Default::default() }, &Station::default(), NOW).into_bytes(),
            qso_logged_datagram(),
            b"<call:18446744073709551615>K1ABC<eor>".to_vec(),
            b"<call:99999999999999999999999>K<eor>".to_vec(),
        ];
        for s in &samples {
            for n in 0..=s.len() {
                let _ = parse_datagram(&s[..n], NOW);
            }
        }
        // generatore pseudo-casuale deterministico (xorshift)
        let mut x: u64 = 0x9e37_79b9_7f4a_7c15;
        let mut next = || {
            x ^= x << 13;
            x ^= x >> 7;
            x ^= x << 17;
            x
        };
        for _ in 0..3000 {
            let len = (next() % 600) as usize;
            let mut buf: Vec<u8> = (0..len).map(|_| next() as u8).collect();
            // ogni tanto con l'inizio giusto, per entrare nei vari lettori
            match next() % 4 {
                0 if buf.len() >= 4 => buf[..4].copy_from_slice(&udp_bridge::MAGIC.to_be_bytes()),
                1 => buf.splice(0..0, b"<contactinfo><call>".iter().copied()).for_each(drop),
                2 => buf.splice(0..0, b"<call:3>".iter().copied()).for_each(drop),
                _ => {}
            }
            let _ = parse_datagram(&buf, NOW);
        }
        assert_eq!(parse_datagram(&vec![b'a'; MAX_DATAGRAM + 1], NOW), Parsed::Unknown);
        assert_eq!(parse_datagram(b"hello", NOW), Parsed::Unknown);
    }

    fn free_port() -> u16 {
        std::net::UdpSocket::bind("127.0.0.1:0").and_then(|s| s.local_addr()).map(|a| a.port()).unwrap_or(0)
    }

    async fn wait_for(inbound: &Inbound, ok: impl Fn(&InboundStats) -> bool) -> InboundStats {
        for _ in 0..100 {
            let s = inbound.stats();
            if ok(&s) {
                return s;
            }
            tokio::time::sleep(std::time::Duration::from_millis(20)).await;
        }
        inbound.stats()
    }

    #[tokio::test]
    async fn received_qsos_go_to_the_log_once_and_are_not_sent_on() {
        let bus = aurora_core::Bus::new(64);
        let book = Arc::new(Logbook::open_in_memory().expect("log"));
        // un destinatario finto in ascolto, come un log esterno a cui Aurora manda i propri QSO
        let fake = UdpSocket::bind("127.0.0.1:0").await.expect("finto");
        let fake_addr = fake.local_addr().expect("indirizzo");
        let _exporter = Exporter::spawn(&bus, Arc::clone(&book), vec![Target::UdpAdif(fake_addr), Target::N1mmXml(fake_addr)], Station::default(), Arc::new(|| NOW / 1000));
        let port = free_port();
        let (tx, rx) = watch::channel(InboundConfig { enabled: true, port, lan: false, forbidden: vec![fake_addr.port()] });
        let calls = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let counted = Arc::clone(&calls);
        let inbound = Inbound::spawn(Arc::clone(&book), rx, Arc::new(move || {
            counted.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        }));
        let s = wait_for(&inbound, |s| s.listening.is_some()).await;
        assert_eq!(s.listening.as_deref(), Some(format!("127.0.0.1:{port}").as_str()));
        let sender = UdpSocket::bind("127.0.0.1:0").await.expect("mittente");
        let record = b"<call:5>K1ABC <band:3>20m <mode:3>FT8 <qso_date:8>20261107 <time_on:6>120000 <eor>";
        sender.send_to(record, ("127.0.0.1", port)).await.expect("inviato");
        // lo stesso QSO da un secondo programma, due minuti dopo
        sender.send_to(b"<call:5>K1ABC <band:3>20M <mode:3>FT8 <qso_date:8>20261107 <time_on:6>120200 <eor>", ("127.0.0.1", port)).await.expect("inviato");
        sender.send_to(b"garbage", ("127.0.0.1", port)).await.expect("inviato");
        let s = wait_for(&inbound, |s| s.received + s.duplicates + s.discarded >= 3).await;
        assert_eq!((s.received, s.duplicates, s.discarded), (1, 1, 1));
        assert_eq!(book.count().expect("conto"), 1);
        assert_eq!(calls.load(std::sync::atomic::Ordering::Relaxed), 1);
        assert_eq!(book.entries().expect("elenco")[0].source, "ADIF");
        // il destinatario finto non riceve niente
        let mut buf = [0u8; 2048];
        let got = tokio::time::timeout(std::time::Duration::from_millis(400), fake.recv_from(&mut buf)).await;
        assert!(got.is_err(), "il QSO ricevuto non deve ripartire verso i destinatari");
        // spento: non ascolta piu'
        tx.send(InboundConfig { enabled: false, ..InboundConfig::default() }).expect("cambio");
        let s = wait_for(&inbound, |s| s.listening.is_none()).await;
        assert!(!s.enabled && s.error.is_none());
    }

    #[tokio::test]
    async fn a_busy_port_is_reported_without_stopping() {
        let held = std::net::UdpSocket::bind("127.0.0.1:0").expect("occupata");
        let port = held.local_addr().expect("porta").port();
        let book = Arc::new(Logbook::open_in_memory().expect("log"));
        let (tx, rx) = watch::channel(InboundConfig { enabled: true, port, lan: false, forbidden: Vec::new() });
        let inbound = Inbound::spawn(book, rx, Arc::new(|| {}));
        let s = wait_for(&inbound, |s| s.error.is_some()).await;
        assert_eq!(s.error.as_deref(), Some("busy"));
        assert!(s.listening.is_none());
        // la porta usata da Aurora per l'invio non si apre
        tx.send(InboundConfig { enabled: true, port: 2237, lan: false, forbidden: vec![2237] }).expect("cambio");
        let s = wait_for(&inbound, |s| s.error.as_deref() == Some("own")).await;
        assert_eq!(s.error.as_deref(), Some("own"));
        // un'altra porta libera: si riparte
        let free = free_port();
        tx.send(InboundConfig { enabled: true, port: free, lan: false, forbidden: Vec::new() }).expect("cambio");
        let s = wait_for(&inbound, |s| s.listening.is_some()).await;
        assert!(s.error.is_none());
    }
}
