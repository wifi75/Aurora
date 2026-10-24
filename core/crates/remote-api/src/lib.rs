//! API HTTP versionata (`/api/v1`) e flusso WebSocket di Aurora.

mod frames;
mod policy;
mod routes;
mod ws;

use std::{
    collections::BTreeSet,
    sync::{Arc, Mutex},
};

use auth::DeviceStore;
use aurora_core::{Bus, Event, StationStatus};
use axum::Router;
use logbook::Logbook;
use tokio::sync::{broadcast::error::RecvError, watch};

pub use frames::encode_spectrum;
pub use policy::{transport_of, Transport, TxPolicy};

const MAX_KNOWN_DECODES: usize = 500;

#[derive(Default)]
struct Shared {
    known: BTreeSet<u64>,
    status: Option<StationStatus>,
}

#[derive(Clone)]
pub struct AppState {
    pub bus: Bus,
    pub auth: Arc<DeviceStore>,
    pub logbook: Arc<Logbook>,
    pub policy: TxPolicy,
    /// Se vero il cookie di sessione porta l'attributo `Secure` (HTTPS).
    pub secure_cookie: bool,
    shared: Arc<Mutex<Shared>>,
    shutdown: Arc<watch::Sender<bool>>,
}

impl AppState {
    /// Va chiamato dentro un runtime Tokio: avvia l'indicizzatore degli eventi.
    pub fn new(
        bus: Bus,
        auth: Arc<DeviceStore>,
        logbook: Arc<Logbook>,
        policy: TxPolicy,
        secure_cookie: bool,
    ) -> AppState {
        let shared = Arc::new(Mutex::new(Shared::default()));
        let mut rx = bus.subscribe_events();
        let indexed = Arc::clone(&shared);
        tokio::spawn(async move {
            loop {
                match rx.recv().await {
                    Ok(event) => {
                        if let Ok(mut s) = indexed.lock() {
                            match event {
                                Event::Decode(d) => {
                                    s.known.insert(d.id);
                                    while s.known.len() > MAX_KNOWN_DECODES {
                                        s.known.pop_first();
                                    }
                                }
                                Event::Status(st) => s.status = Some(st),
                                _ => {}
                            }
                        }
                    }
                    Err(RecvError::Lagged(_)) => continue,
                    Err(RecvError::Closed) => break,
                }
            }
        });
        let (shutdown, _) = watch::channel(false);
        AppState { bus, auth, logbook, policy, secure_cookie, shared, shutdown: Arc::new(shutdown) }
    }

    /// Chiede a tutti i flussi aperti di chiudersi (spegnimento ordinato).
    pub fn begin_shutdown(&self) {
        let _ = self.shutdown.send(true);
    }

    pub(crate) fn subscribe_shutdown(&self) -> watch::Receiver<bool> {
        self.shutdown.subscribe()
    }

    pub fn knows_decode(&self, id: u64) -> bool {
        self.shared.lock().is_ok_and(|s| s.known.contains(&id))
    }

    pub(crate) fn current_status(&self) -> Option<StationStatus> {
        self.shared.lock().ok().and_then(|s| s.status.clone())
    }
}

pub fn build_router(state: AppState) -> Router {
    routes::router(state)
}
