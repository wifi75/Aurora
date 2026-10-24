//! Tiene aggiornati i servizi online che si sono accesi (PSK Reporter, propagazione): ogni mezzo minuto guarda se e' ora di chiedere.

use std::{sync::Arc, time::Duration};

use aurora_core::Event;
use online_info::OnlineService;
use tokio::sync::broadcast;

const TICK: Duration = Duration::from_secs(30);

fn now_ms() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_millis() as u64)
}

/// Il nominativo viene dallo stato della stazione (quello di JTDX); senza, PSK Reporter non si chiede.
pub fn spawn_refresher(service: Arc<OnlineService>, mut events: broadcast::Receiver<Event>) {
    tokio::spawn(async move {
        let mut callsign = String::new();
        let mut tick = tokio::time::interval(TICK);
        loop {
            tokio::select! {
                received = events.recv() => match received {
                    Ok(Event::Status(s)) => callsign = s.de_call,
                    Ok(_) | Err(broadcast::error::RecvError::Lagged(_)) => {}
                    Err(broadcast::error::RecvError::Closed) => return,
                },
                _ = tick.tick() => {
                    let service = Arc::clone(&service);
                    let callsign = callsign.clone();
                    // la richiesta di rete e' bloccante
                    let _ = tokio::task::spawn_blocking(move || service.refresh_due(now_ms(), &callsign)).await;
                }
            }
        }
    });
}
