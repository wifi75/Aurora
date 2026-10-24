//! Collega motore, log, autenticazione e API in un unico processo.

use std::{
    future::Future,
    net::SocketAddr,
    path::{Path, PathBuf},
    sync::Arc,
};

use auth::{DeviceStore, SystemClock};
use aurora_core::{Bus, Engine, EngineIo};
use axum::{http::StatusCode, routing::any};
use engine_sim::{SimConfig, SimEngine};
use logbook::Logbook;
use remote_api::{build_router, AppState, TxPolicy};
use tokio::{net::TcpListener, task::JoinHandle};
use tower_http::services::{ServeDir, ServeFile};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EngineKind {
    Sim,
}

#[derive(Debug, Clone)]
pub struct Config {
    pub bind: SocketAddr,
    pub data_dir: PathBuf,
    pub ui_dir: Option<PathBuf>,
    pub engine: EngineKind,
    /// Durata del ciclo del simulatore, in millisecondi.
    pub cycle_ms: u64,
}

#[derive(Debug)]
pub enum StartError {
    BadConfig(String),
    Io(String),
    Storage(String),
}

impl std::fmt::Display for StartError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StartError::BadConfig(e) => write!(f, "configurazione non valida: {e}"),
            StartError::Io(e) => write!(f, "errore di rete o di disco: {e}"),
            StartError::Storage(e) => write!(f, "errore di archiviazione: {e}"),
        }
    }
}

impl std::error::Error for StartError {}

impl Config {
    pub fn from_env() -> Result<Config, StartError> {
        Self::from_lookup(|k| std::env::var(k).ok())
    }

    /// Legge la configurazione da una funzione di ricerca (testabile senza toccare l'ambiente).
    pub fn from_lookup(get: impl Fn(&str) -> Option<String>) -> Result<Config, StartError> {
        let bind_text = get("AURORA_BIND").unwrap_or_else(|| "127.0.0.1:8787".to_string());
        let bind = bind_text
            .parse()
            .map_err(|_| StartError::BadConfig(format!("AURORA_BIND non e' un indirizzo valido: {bind_text}")))?;
        let engine = match get("AURORA_ENGINE").as_deref() {
            None | Some("sim") => EngineKind::Sim,
            Some(other) => return Err(StartError::BadConfig(format!("AURORA_ENGINE sconosciuto: {other}"))),
        };
        let cycle_ms = match get("AURORA_SIM_CYCLE_MS") {
            None => 15_000,
            Some(v) => v
                .parse()
                .map_err(|_| StartError::BadConfig(format!("AURORA_SIM_CYCLE_MS non e' un numero: {v}")))?,
        };
        Ok(Config {
            bind,
            data_dir: PathBuf::from(get("AURORA_DATA_DIR").unwrap_or_else(|| "data".to_string())),
            ui_dir: get("AURORA_UI_DIR").map(PathBuf::from),
            engine,
            cycle_ms,
        })
    }
}

pub struct RunningServer {
    pub addr: SocketAddr,
    pub state: AppState,
    task: JoinHandle<()>,
}

impl RunningServer {
    /// Attende la fine del server (dopo il segnale di spegnimento).
    pub async fn join(self) {
        let _ = self.task.await;
    }
}

fn open_storage(data_dir: &Path) -> Result<(Arc<DeviceStore>, Arc<Logbook>), StartError> {
    std::fs::create_dir_all(data_dir).map_err(|e| StartError::Io(format!("{}: {e}", data_dir.display())))?;
    let auth = DeviceStore::open(&data_dir.join("devices.db"), Arc::new(SystemClock))
        .map_err(|e| StartError::Storage(e.to_string()))?;
    let logbook = Logbook::open(&data_dir.join("logbook.db")).map_err(|e| StartError::Storage(e.to_string()))?;
    Ok((Arc::new(auth), Arc::new(logbook)))
}

/// Avvia il server e ritorna subito; si ferma quando `shutdown` si completa.
pub async fn run(
    config: Config,
    shutdown: impl Future<Output = ()> + Send + 'static,
) -> Result<RunningServer, StartError> {
    let (auth, logbook) = open_storage(&config.data_dir)?;
    let bus = Bus::new(4096);
    let state = AppState::new(bus.clone(), auth, logbook, TxPolicy::default_safe(), false);

    let commands = bus.take_command_rx().ok_or_else(|| StartError::Io("canale dei comandi gia' preso".into()))?;
    let io = EngineIo { events: bus.events(), spectrum: bus.spectrum(), commands };
    let engine_task = match config.engine {
        EngineKind::Sim => tokio::spawn(
            SimEngine::new(SimConfig { seed: 1, cycle_ms: config.cycle_ms }).run(io),
        ),
    };

    let mut app = build_router(state.clone()).route("/api/{*rest}", any(|| async { StatusCode::NOT_FOUND }));
    if let Some(ui) = &config.ui_dir {
        let index = ui.join("index.html");
        app = app.fallback_service(ServeDir::new(ui).fallback(ServeFile::new(index)));
    }

    let listener = TcpListener::bind(config.bind)
        .await
        .map_err(|e| StartError::Io(format!("{}: {e}", config.bind)))?;
    let addr = listener.local_addr().map_err(|e| StartError::Io(e.to_string()))?;

    let closing = state.clone();
    let task = tokio::spawn(async move {
        let served = axum::serve(listener, app.into_make_service_with_connect_info::<SocketAddr>())
            .with_graceful_shutdown(async move {
                shutdown.await;
                closing.begin_shutdown();
            })
            .await;
        if let Err(e) = served {
            tracing::error!("server terminato con errore: {e}");
        }
        engine_task.abort();
    });
    Ok(RunningServer { addr, state, task })
}
