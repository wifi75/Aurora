//! Ogni QSO registrato viene salvato nel log di Aurora e inviato ai log esterni configurati, con lo stesso record ADIF che JTDX produce
//! (`LogQSO::accept`: datagramma per il secondo server UDP, riga di `wsjtx_log.adi` per il file).
//!
//! Attenzione ai doppioni: con il motore JTDX e' JTDX a mandare i QSO a Log4OM2 con il suo server UDP secondario; aggiungere qui lo stesso
//! destinatario li registrerebbe due volte. Con il motore proprio JTDX non c'e' e il destinatario e' proprio quello (un solo invio per QSO).

use std::{
    net::SocketAddr,
    path::PathBuf,
    sync::{Arc, Mutex},
};

use aurora_core::{Bus, Event};
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
                        for target in &targets {
                            let record = match target {
                                Target::UdpAdif(_) => adif::jtdx_record(&q, &station, &date),
                                Target::File(_) => adif::jtdx_file_record(&q, &station, &date),
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
        Target::UdpAdif(addr) => {
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

