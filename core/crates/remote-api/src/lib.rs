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
use qso_engine::{Automation, Rules};
use rig_control::{schedule::Schedule, table::FrequencyTable, Rig};
use tokio::sync::{broadcast::error::RecvError, watch};

pub use frames::encode_spectrum;
pub use policy::{transport_of, Transport, TxPolicy};

/// Salva le regole dell'automazione quando cambiano.
pub type RulesSaver = Arc<dyn Fn(&Rules) + Send + Sync>;
/// Calcola le statistiche del log a ogni richiesta.
pub type StatsProvider = Arc<dyn Fn() -> serde_json::Value + Send + Sync>;

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
    /// L'automazione dei QSO, se il programma la offre.
    pub automation: Option<Automation>,
    /// Come e' avviata la stazione (da mostrare nelle impostazioni, sola lettura).
    pub station_info: Arc<serde_json::Value>,
    /// La radio comandabile (cambio banda), se disponibile.
    pub rig: Option<Arc<dyn Rig>>,
    /// Salva le regole dell'automazione quando cambiano (per ritrovarle al riavvio).
    /// Calcola le statistiche del log (premi e paesi mancanti) a ogni richiesta.
    pub stats: Option<StatsProvider>,
    pub rules_saver: Option<RulesSaver>,
    /// Frequenze di lavoro per banda e modo (si cambiano dalle impostazioni).
    pub frequencies: Arc<Mutex<FrequencyTable>>,
    /// Programmazione delle bande per orario.
    pub schedule: Arc<Mutex<Schedule>>,
    /// Chiamato dopo ogni modifica di frequenze o programmazione, per salvarle.
    pub settings_saver: Option<Arc<dyn Fn() + Send + Sync>>,
    /// Accesso senza codice per le prove, solo dalla rete di casa. Mai salvato: al riavvio e' sempre spento.
    pub open_access: Arc<std::sync::atomic::AtomicBool>,
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
        AppState { bus, auth, logbook, policy, secure_cookie, automation: None, station_info: Arc::new(serde_json::json!({})), rig: None, stats: None, rules_saver: None, frequencies: Arc::default(), schedule: Arc::default(), settings_saver: None, open_access: Arc::default(), shared, shutdown: Arc::new(shutdown) }
    }

    /// Parte con l'accesso senza codice acceso o spento (scelta dell'operatore all'avvio).
    pub fn with_open_access(self, open: bool) -> AppState {
        self.open_access.store(open, std::sync::atomic::Ordering::SeqCst);
        self
    }

    pub fn with_stats_if(self, stats: Option<StatsProvider>) -> AppState {
        match stats {
            Some(s) => self.with_stats(s),
            None => self,
        }
    }

    pub fn with_stats(mut self, stats: StatsProvider) -> AppState {
        self.stats = Some(stats);
        self
    }

    pub fn with_rules_saver(mut self, saver: RulesSaver) -> AppState {
        self.rules_saver = Some(saver);
        self
    }

    pub fn with_frequencies(mut self, frequencies: Arc<Mutex<FrequencyTable>>) -> AppState {
        self.frequencies = frequencies;
        self
    }

    pub fn with_schedule(mut self, schedule: Arc<Mutex<Schedule>>) -> AppState {
        self.schedule = schedule;
        self
    }

    pub fn with_settings_saver(mut self, saver: Arc<dyn Fn() + Send + Sync>) -> AppState {
        self.settings_saver = Some(saver);
        self
    }

    /// Vero se JTDX sta trasmettendo adesso.
    pub fn is_transmitting(&self) -> bool {
        self.current_status().is_some_and(|s| s.transmitting)
    }

    /// Modo in uso ("FT8" o "FT4"); FT8 finche' JTDX non lo ha detto.
    pub fn current_mode(&self) -> String {
        self.current_status().map(|s| s.mode).filter(|m| !m.is_empty()).unwrap_or_else(|| "FT8".into())
    }

    pub fn with_rig(mut self, rig: Arc<dyn Rig>) -> AppState {
        self.rig = Some(rig);
        self
    }

    pub fn with_rig_if(self, rig: Option<Arc<dyn Rig>>) -> AppState {
        match rig {
            Some(r) => self.with_rig(r),
            None => self,
        }
    }

    pub fn with_station_info(mut self, info: serde_json::Value) -> AppState {
        self.station_info = Arc::new(info);
        self
    }

    pub fn with_automation(mut self, automation: Automation) -> AppState {
        self.automation = Some(automation);
        self
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
