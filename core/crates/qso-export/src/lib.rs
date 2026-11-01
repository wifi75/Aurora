//! Ogni QSO registrato viene salvato nel log di Aurora e inviato ai log esterni configurati, con lo stesso record ADIF che JTDX produce
//! (`LogQSO::accept`: datagramma per il secondo server UDP, riga di `wsjtx_log.adi` per il file).
//!
//! Attenzione ai doppioni: con il motore JTDX e' JTDX a mandare i QSO a Log4OM2 con il suo server UDP secondario; aggiungere qui lo stesso
//! destinatario li registrerebbe due volte. Con il motore proprio JTDX non c'e' e il destinatario e' proprio quello (un solo invio per QSO).

use std::{
    net::SocketAddr,
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex, RwLock,
    },
};

use aurora_core::{Bus, Event, QsoRecord};
use logbook::{
    adif::{self, Station},
    Logbook,
};
use tokio::{io::AsyncWriteExt, net::UdpSocket, sync::broadcast::error::RecvError};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Target {
    /// Il record ADIF di JTDX in un datagramma UDP, senza intestazione e senza andare a capo (Log4OM2, GridTracker2 in ascolto ADIF).
    UdpAdif(SocketAddr),
    /// Aggiunge una riga al file ADIF nel formato di `wsjtx_log.adi` (intestazione `WSJT-X ADIF Export<eoh>` se il file e' nuovo, CR LF).
    File(PathBuf),
    /// Il messaggio XML `contactinfo` di N1MM Logger+ in un datagramma (di norma porta 12060): HRD Logbook e i programmi che imitano N1MM.
    N1mmXml(SocketAddr),
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ExportStats {
    pub sent: u64,
    pub failed: u64,
    pub last_error: Option<String>,
}

#[derive(Clone)]
pub struct Exporter {
    stats: Arc<Mutex<ExportStats>>,
}

/// La guardia contro i doppioni di JTDX (`m_lastloggedcall`/`m_lastloggedtime`): lo stesso nominativo registrato di nuovo entro due periodi
/// (`int(periodo) * 2000` ms: 30 s in FT8, 14 s in FT4) non e' un altro QSO.
#[derive(Default)]
struct DupeGuard {
    last: Option<(String, u64)>,
}

impl DupeGuard {
    /// `stamp_ms`: l'istante del QSO. Vero se va registrato.
    fn accept(&mut self, call: &str, mode: &str, stamp_ms: u64) -> bool {
        let window = if mode == "FT4" { 14_000 } else { 30_000 };
        let call = call.to_uppercase();
        if let Some((last, at)) = &self.last
            && *last == call
            && stamp_ms.abs_diff(*at) <= window
        {
            return false;
        }
        self.last = Some((call, stamp_ms));
        true
    }
}

impl Exporter {
    /// Va chiamato dentro un runtime Tokio. `now` = secondi Unix. `station`: nominativo, locatore e potenza che JTDX scrive in ogni record.
    pub fn spawn(bus: &Bus, book: Arc<Logbook>, targets: Vec<Target>, station: Station, now: Arc<dyn Fn() -> u64 + Send + Sync>) -> Exporter {
        Exporter::spawn_shared(bus, book, Arc::new(RwLock::new(targets)), station, now)
    }

    /// Come `spawn`, ma i destinatari stanno in una lista condivisa che si puo' cambiare a programma acceso (le impostazioni di Aurora): ogni QSO
    /// usa la lista com'e' in quel momento.
    pub fn spawn_shared(bus: &Bus, book: Arc<Logbook>, targets: Arc<RwLock<Vec<Target>>>, station: Station, now: Arc<dyn Fn() -> u64 + Send + Sync>) -> Exporter {
        Exporter::spawn_shared_eqsl(bus, book, targets, station, now, Arc::new(AtomicBool::new(false)))
    }

    /// Come `spawn_shared`; con `eqsl` acceso i record portano i campi `EQSL_QSL_SENT`/`EQSL_QSLSDATE` come in JTDX con l'invio a eQSL acceso.
    pub fn spawn_shared_eqsl(
        bus: &Bus,
        book: Arc<Logbook>,
        targets: Arc<RwLock<Vec<Target>>>,
        station: Station,
        now: Arc<dyn Fn() -> u64 + Send + Sync>,
        eqsl: Arc<AtomicBool>,
    ) -> Exporter {
        let stats = Arc::new(Mutex::new(ExportStats::default()));
        let shared = Arc::clone(&stats);
        let mut events = bus.subscribe_events();
        tokio::spawn(async move {
            let socket = UdpSocket::bind("0.0.0.0:0").await.ok();
            let mut guard = DupeGuard::default();
            loop {
                match events.recv().await {
                    Ok(Event::QsoLogged(q)) => {
                        let stamp = if q.time_off_unix_ms > 0 { q.time_off_unix_ms } else { now() * 1000 };
                        if !guard.accept(&q.call, &q.mode, stamp) {
                            tracing::info!("QSO con {} gia' registrato da poco: non si registra due volte", q.call);
                            continue;
                        }
                        if q.band.is_empty() {
                            tracing::warn!("QSO con {}: banda sconosciuta (frequenza della radio non letta), il record ADIF sara' incompleto", q.call);
                        }
                        if let Err(e) = book.add(&q) {
                            tracing::warn!("QSO non salvato nel log di Aurora: {e}");
                        }
                        let date = adif::utc_date(now());
                        let current = targets.read().map(|t| t.clone()).unwrap_or_default();
                        let eqsl_on = eqsl.load(Ordering::Relaxed);
                        for target in &current {
                            let record = match target {
                                Target::UdpAdif(_) => adif::jtdx_record_with(&q, &station, &date, eqsl_on.then_some(date.as_str())),
                                Target::File(_) => adif::jtdx_file_record_with(&q, &station, &date, eqsl_on),
                                Target::N1mmXml(_) => n1mm_contact_info(&q, &station, now() * 1000),
                            };
                            let result = send(target, socket.as_ref(), &record).await;
                            record_result(&shared, target, result);
                        }
                    }
                    Ok(_) | Err(RecvError::Lagged(_)) => {}
                    Err(RecvError::Closed) => break,
                }
            }
        });
        Exporter { stats }
    }

    pub fn stats(&self) -> ExportStats {
        self.stats.lock().map(|s| s.clone()).unwrap_or_default()
    }
}

fn record_result(stats: &Mutex<ExportStats>, target: &Target, result: Result<(), String>) {
    let Ok(mut s) = stats.lock() else { return };
    match result {
        Ok(()) => s.sent += 1,
        Err(e) => {
            tracing::warn!("QSO non inviato a {target:?}: {e}");
            s.failed += 1;
            s.last_error = Some(e);
        }
    }
}

async fn send(target: &Target, socket: Option<&UdpSocket>, record: &str) -> Result<(), String> {
    match target {
        Target::UdpAdif(addr) | Target::N1mmXml(addr) => {
            let socket = socket.ok_or("socket UDP non disponibile")?;
            socket.send_to(record.as_bytes(), addr).await.map(|_| ()).map_err(|e| format!("{addr}: {e}"))
        }
        Target::File(path) => {
            let is_new = !path.exists();
            let mut file = tokio::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(path)
                .await
                .map_err(|e| format!("{}: {e}", path.display()))?;
            let text = if is_new { format!("{}{record}", adif::JTDX_FILE_HEADER) } else { record.to_string() };
            file.write_all(text.as_bytes()).await.map_err(|e| format!("{}: {e}", path.display()))
        }
    }
}


/// Il caso XML di un testo (`&`, `<`, `>`, virgolette).
fn xml_escape(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&apos;"),
            c if c.is_control() => {}
            c => out.push(c),
        }
    }
    out
}

/// La banda come la scrive N1MM (MHz del limite inferiore: `14`, `3.5`, `1.8`, `144`...); dalla frequenza se la banda non e' nota.
fn n1mm_band(band: &str, freq_hz: u64) -> String {
    let known = match band.to_ascii_lowercase().as_str() {
        "2200m" => "0.136",
        "630m" => "0.472",
        "160m" => "1.8",
        "80m" => "3.5",
        "60m" => "5",
        "40m" => "7",
        "30m" => "10",
        "20m" => "14",
        "17m" => "18",
        "15m" => "21",
        "12m" => "24",
        "10m" => "28",
        "6m" => "50",
        "4m" => "70",
        "2m" => "144",
        "1.25m" => "222",
        "70cm" => "420",
        "23cm" => "1240",
        _ => "",
    };
    if !known.is_empty() {
        return known.to_string();
    }
    (freq_hz / 1_000_000).to_string()
}

/// Un identificativo di 32 cifre esadecimali (come il GUID senza trattini di N1MM), stabile per lo stesso QSO: chi riceve due volte lo stesso
/// messaggio lo riconosce.
fn n1mm_id(q: &QsoRecord, station: &Station) -> String {
    // FNV-1a a 64 bit su due semi diversi
    let hash = |seed: u64| {
        let mut h = seed;
        for b in format!("{}|{}|{}|{}|{}", station.call, q.call, q.time_on_unix_ms, q.time_off_unix_ms, q.freq_hz).bytes() {
            h ^= u64::from(b);
            h = h.wrapping_mul(0x0100_0000_01b3);
        }
        h
    };
    format!("{:016x}{:016x}", hash(0xcbf2_9ce4_8422_2325), hash(0x84222325_cbf29ce4))
}

/// Il messaggio `contactinfo` di N1MM Logger+ (messaggi UDP esterni, porta 12060) per un QSO: nominativi, banda, frequenze in decine di Hz,
/// modo, rapporti, locatore, nome, potenza e commento; i campi di gara restano vuoti o a zero. `now_ms` serve se il QSO non ha l'ora di fine.
pub fn n1mm_contact_info(q: &QsoRecord, station: &Station, now_ms: u64) -> String {
    let stamp = if q.time_off_unix_ms > 0 { q.time_off_unix_ms } else if q.time_on_unix_ms > 0 { q.time_on_unix_ms } else { now_ms };
    let (date, time) = adif::utc_datetime(stamp);
    let part = |text: &str, a: usize, b: usize| text.get(a..b).unwrap_or("00").to_string();
    let timestamp = format!("{}-{}-{} {}:{}:{}", part(&date, 0, 4), part(&date, 4, 6), part(&date, 6, 8), part(&time, 0, 2), part(&time, 2, 4), part(&time, 4, 6));
    let tens = (q.freq_hz / 10).to_string();
    let power = if q.tx_power.is_empty() { station.tx_power.clone() } else { q.tx_power.clone() };
    let fields: [(&str, String); 26] = [
        ("app", "Aurora".to_string()),
        ("contestname", "DX".to_string()),
        ("contestnr", "0".to_string()),
        ("timestamp", timestamp),
        ("mycall", station.call.clone()),
        ("band", n1mm_band(&q.band, q.freq_hz)),
        ("rxfreq", tens.clone()),
        ("txfreq", tens),
        ("operator", station.call.clone()),
        ("mode", q.mode.clone()),
        ("call", q.call.clone()),
        ("snt", q.rst_sent.clone()),
        ("rcv", q.rst_rcvd.clone()),
        ("gridsquare", q.grid.clone()),
        ("exchange1", String::new()),
        ("comment", q.comment.clone()),
        ("name", q.name.clone()),
        ("power", power),
        ("misctext", String::new()),
        ("points", "1".to_string()),
        ("radionr", "1".to_string()),
        ("RadioInterfaced", "1".to_string()),
        ("NetworkedCompNr", "0".to_string()),
        ("IsOriginal", "True".to_string()),
        ("ID", n1mm_id(q, station)),
        ("IsClaimedQso", "True".to_string()),
    ];
    let mut out = String::from("<?xml version=\"1.0\" encoding=\"utf-8\"?>\n<contactinfo>\n");
    for (name, value) in fields {
        out.push_str(&format!("\t<{name}>{}</{name}>\n", xml_escape(&value)));
    }
    out.push_str("</contactinfo>\n");
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_n1mm_contact_info_carries_the_qso() {
        let q = QsoRecord {
            call: "K1ABC".into(),
            grid: "FN42".into(),
            band: "20m".into(),
            mode: "FT8".into(),
            freq_hz: 14_075_084,
            rst_sent: "-16".into(),
            rst_rcvd: "-09".into(),
            time_on_unix_ms: 1_792_860_405_000,
            time_off_unix_ms: 1_792_860_449_000,
            name: "Al & Bo".into(),
            ..Default::default()
        };
        let station = Station { call: "IU3ABC".into(), grid: "FN42".into(), tx_power: "50".into() };
        let xml = n1mm_contact_info(&q, &station, 0);
        assert!(xml.starts_with("<?xml version=\"1.0\" encoding=\"utf-8\"?>\n<contactinfo>"));
        for part in ["<mycall>IU3ABC</mycall>", "<call>K1ABC</call>", "<band>14</band>", "<rxfreq>1407508</rxfreq>", "<mode>FT8</mode>", "<snt>-16</snt>", "<rcv>-09</rcv>", "<gridsquare>FN42</gridsquare>", "<power>50</power>", "<name>Al &amp; Bo</name>", "<timestamp>2026-10-24 16:47:29</timestamp>"] {
            assert!(xml.contains(part), "manca {part} in {xml}");
        }
        let id = xml.split("<ID>").nth(1).and_then(|r| r.split("</ID>").next()).unwrap_or_default().to_string();
        assert_eq!(id.len(), 32);
        assert_eq!(n1mm_contact_info(&q, &station, 0), xml, "lo stesso QSO ha lo stesso identificativo");
        assert_eq!(n1mm_band("", 7_074_000), "7");
    }
}
