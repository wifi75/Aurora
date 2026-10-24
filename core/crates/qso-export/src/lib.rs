//! Ogni QSO registrato viene salvato nel log di Aurora e inviato ai log esterni configurati.
//!
//! Attenzione ai doppioni: JTDX manda gia' i QSO a Log4OM2 con il suo server UDP secondario. Aggiungere
//! qui lo stesso destinatario li registrerebbe due volte.

use std::{
    net::SocketAddr,
    path::PathBuf,
    sync::{Arc, Mutex},
};

use aurora_core::{Bus, Event};
use logbook::{adif, Logbook};
use tokio::{io::AsyncWriteExt, net::UdpSocket, sync::broadcast::error::RecvError};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Target {
    /// Il record ADIF in un datagramma UDP (Log4OM2, GridTracker2 in ascolto ADIF).
    UdpAdif(SocketAddr),
    /// Aggiunge al file ADIF, con l'intestazione se il file e' nuovo.
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

impl Exporter {
    /// Va chiamato dentro un runtime Tokio. `now` = secondi Unix.
    pub fn spawn(bus: &Bus, book: Arc<Logbook>, targets: Vec<Target>, now: Arc<dyn Fn() -> u64 + Send + Sync>) -> Exporter {
        let stats = Arc::new(Mutex::new(ExportStats::default()));
        let shared = Arc::clone(&stats);
        let mut events = bus.subscribe_events();
        tokio::spawn(async move {
            let socket = UdpSocket::bind("0.0.0.0:0").await.ok();
            loop {
                match events.recv().await {
                    Ok(Event::QsoLogged(q)) => {
                        if let Err(e) = book.add(&q) {
                            tracing::warn!("QSO non salvato nel log di Aurora: {e}");
                        }
                        let record = adif::record(&q, &adif::utc_date(now()));
                        for target in &targets {
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
            let text = if is_new { format!("{}{record}", adif::header()) } else { record.to_string() };
            file.write_all(text.as_bytes()).await.map_err(|e| format!("{}: {e}", path.display()))
        }
    }
}

