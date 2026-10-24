//! Collega motore, log, autenticazione e API in un unico processo.

use std::{
    future::Future,
    net::SocketAddr,
    path::{Path, PathBuf},
    sync::Arc,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use audio_spectrum::run_capture;
use auth::{DeviceStore, SystemClock};
use aurora_core::{Bus, Engine, EngineIo};
use axum::{http::StatusCode, routing::any};
use dxcc::DxccDb;
use engine_jtdx::{worked::WorkedLog, JtdxConfig, JtdxEngine, SystemClock as EngineClock};
use engine_sim::{SimConfig, SimEngine};
use logbook::Logbook;
use qso_export::{Exporter, Target};
use qso_engine::{Automation, AutomationConfig, Limits, Rules};
use remote_api::{build_router, AppState, TxPolicy};
use tokio::{net::TcpListener, task::JoinHandle};
use tower_http::services::{ServeDir, ServeFile};
use udp_hub::{Hub, HubConfig};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EngineKind {
    /// Stazione simulata, senza radio.
    Sim,
    /// JTDX reale, tramite l'hub UDP.
    Jtdx,
}

#[derive(Debug, Clone)]
pub struct Config {
    pub bind: SocketAddr,
    pub data_dir: PathBuf,
    pub ui_dir: Option<PathBuf>,
    pub engine: EngineKind,
    /// Durata del ciclo del simulatore, in millisecondi.
    pub cycle_ms: u64,
    /// Cookie di sessione con attributo `Secure`: da attivare quando si arriva in HTTPS (Tailscale, Cloudflare).
    pub secure_cookie: bool,
    /// Dove l'hub ascolta il flusso di JTDX (il server UDP primario di JTDX va puntato qui).
    pub udp_listen: SocketAddr,
    /// A chi l'hub inoltra il flusso identico (per esempio GridTracker2 su 127.0.0.1:2333).
    pub udp_forward: Vec<SocketAddr>,
    /// Consente i comandi che fanno trasmettere JTDX (Reply). Predefinito: no.
    pub allow_tx: bool,
    /// Parte del nome dell'ingresso audio da cui ricavare la cascata (vuoto: nessuna cattura audio).
    pub audio_device: Option<String>,
    /// Con JTDX: chiede il Replay dei decode correnti appena lo vede. Chi riceve il flusso (GridTracker2) li
    /// riceve di nuovo, marcati come "non nuovi".
    pub replay_on_connect: bool,
    /// File `cty.dat` per riconoscere il paese di ogni stazione. Senza, niente paese ne' novita'.
    pub cty_path: Option<PathBuf>,
    /// Log ADIF dei QSO gia' fatti, per segnalare nuovi DXCC, zone e griglie.
    pub worked_adif: Option<PathBuf>,
    /// Salvaguardie dell'automazione dei QSO: si impostano solo qui, sul PC.
    pub auto_limits: Limits,
    /// Log esterni che ricevono ogni QSO come ADIF su UDP (attenzione ai doppioni con il server UDP secondario di JTDX).
    pub export_udp: Vec<SocketAddr>,
    /// File ADIF a cui aggiungere ogni QSO.
    pub export_file: Option<PathBuf>,
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
    /// Valori predefiniti (stazione simulata, trasmissione chiusa).
    pub fn defaults() -> Config {
        Config {
            bind: SocketAddr::from(([127, 0, 0, 1], 8787)),
            data_dir: PathBuf::from("data"),
            ui_dir: None,
            engine: EngineKind::Sim,
            cycle_ms: 15_000,
            secure_cookie: false,
            udp_listen: SocketAddr::from(([127, 0, 0, 1], 2334)),
            udp_forward: Vec::new(),
            allow_tx: false,
            audio_device: None,
            replay_on_connect: true,
            cty_path: None,
            worked_adif: None,
            auto_limits: Limits::default(),
            export_udp: Vec::new(),
            export_file: None,
        }
    }

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
            Some("jtdx") => EngineKind::Jtdx,
            Some(other) => return Err(StartError::BadConfig(format!("AURORA_ENGINE sconosciuto: {other}"))),
        };
        let cycle_ms = match get("AURORA_SIM_CYCLE_MS") {
            None => 15_000,
            Some(v) => v
                .parse()
                .map_err(|_| StartError::BadConfig(format!("AURORA_SIM_CYCLE_MS non e' un numero: {v}")))?,
        };
        let udp_listen = match get("AURORA_UDP_LISTEN") {
            None => Config::defaults().udp_listen,
            Some(v) => v
                .parse()
                .map_err(|_| StartError::BadConfig(format!("AURORA_UDP_LISTEN non e' un indirizzo valido: {v}")))?,
        };
        let mut udp_forward = Vec::new();
        for part in get("AURORA_UDP_FORWARD").unwrap_or_default().split(',').map(str::trim).filter(|p| !p.is_empty()) {
            udp_forward.push(
                part.parse()
                    .map_err(|_| StartError::BadConfig(format!("AURORA_UDP_FORWARD contiene un indirizzo non valido: {part}")))?,
            );
        }
        let allow_tx = match get("AURORA_ALLOW_TX").as_deref() {
            None | Some("0" | "false") => false,
            Some("1" | "true") => true,
            Some(other) => {
                return Err(StartError::BadConfig(format!("AURORA_ALLOW_TX deve essere 0 o 1, non {other}")));
            }
        };
        let replay_on_connect = match get("AURORA_REPLAY_ON_CONNECT").as_deref() {
            None | Some("1" | "true") => true,
            Some("0" | "false") => false,
            Some(other) => {
                return Err(StartError::BadConfig(format!("AURORA_REPLAY_ON_CONNECT deve essere 0 o 1, non {other}")));
            }
        };
        let mut export_udp = Vec::new();
        for part in get("AURORA_EXPORT_UDP").unwrap_or_default().split(',').map(str::trim).filter(|p| !p.is_empty()) {
            export_udp.push(
                part.parse()
                    .map_err(|_| StartError::BadConfig(format!("AURORA_EXPORT_UDP contiene un indirizzo non valido: {part}")))?,
            );
        }
        let defaults = Limits::default();
        let auto_limits = Limits {
            max_consecutive_cycles: positive(&get, "AURORA_AUTO_MAX_CYCLES", defaults.max_consecutive_cycles)?,
            max_per_day: positive(&get, "AURORA_AUTO_MAX_PER_DAY", defaults.max_per_day)?,
            dead_man_secs: u64::from(positive(&get, "AURORA_AUTO_DEAD_MAN_SECS", u32::try_from(defaults.dead_man_secs).unwrap_or(120))?),
            run_unattended: match get("AURORA_AUTO_UNATTENDED").as_deref() {
                None | Some("0") => false,
                Some("1") => true,
                Some(other) => return Err(StartError::BadConfig(format!("AURORA_AUTO_UNATTENDED deve essere 0 o 1, non {other}"))),
            },
        };
        Ok(Config {
            bind,
            data_dir: PathBuf::from(get("AURORA_DATA_DIR").unwrap_or_else(|| "data".to_string())),
            ui_dir: get("AURORA_UI_DIR").map(PathBuf::from),
            engine,
            cycle_ms,
            secure_cookie: matches!(get("AURORA_SECURE_COOKIE").as_deref(), Some("1" | "true")),
            udp_listen,
            udp_forward,
            allow_tx,
            audio_device: get("AURORA_AUDIO_DEVICE").filter(|d| !d.trim().is_empty()),
            replay_on_connect,
            cty_path: get("AURORA_CTY_PATH").filter(|p| !p.trim().is_empty()).map(PathBuf::from),
            worked_adif: get("AURORA_WORKED_ADIF").filter(|p| !p.trim().is_empty()).map(PathBuf::from),
            auto_limits,
            export_udp,
            export_file: get("AURORA_EXPORT_FILE").filter(|p| !p.trim().is_empty()).map(PathBuf::from),
        })
    }
}

pub struct RunningServer {
    pub addr: SocketAddr,
    pub state: AppState,
    /// Presente solo con il motore JTDX.
    pub hub: Option<Arc<Hub>>,
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

/// Carica `cty.dat`; se manca o non e' leggibile Aurora parte lo stesso, senza paese ne' novita'.
fn load_dxcc(path: Option<&Path>) -> Option<Arc<DxccDb>> {
    let path = path?;
    match DxccDb::from_file(path) {
        Ok(db) => {
            tracing::info!("cty.dat letto: {} paesi", db.len());
            Some(Arc::new(db))
        }
        Err(e) => {
            tracing::warn!("paesi DXCC non attivi: {e}");
            None
        }
    }
}

/// Carica i QSO gia' fatti; un file mancante o illeggibile vale come log vuoto (nessuna novita' segnalata).
fn load_worked(path: Option<&Path>, db: &DxccDb) -> WorkedLog {
    let Some(path) = path else { return WorkedLog::new() };
    match std::fs::read(path) {
        Ok(bytes) => {
            let log = WorkedLog::from_adif(&String::from_utf8_lossy(&bytes), db);
            tracing::info!("log letto: {} QSO", log.len());
            log
        }
        Err(e) => {
            tracing::warn!("log dei QSO non letto ({}): {e}", path.display());
            WorkedLog::new()
        }
    }
}

fn unix_secs() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |d| d.as_secs())
}

/// Numero intero maggiore di zero da una variabile; un tetto a zero bloccherebbe tutto e si rifiuta.
fn positive(get: &impl Fn(&str) -> Option<String>, name: &str, default: u32) -> Result<u32, StartError> {
    match get(name) {
        None => Ok(default),
        Some(v) => v
            .trim()
            .parse::<u32>()
            .ok()
            .filter(|n| *n > 0)
            .ok_or_else(|| StartError::BadConfig(format!("{name} deve essere un numero maggiore di zero, non {v}"))),
    }
}

/// Avvia il server e ritorna subito; si ferma quando `shutdown` si completa.
pub async fn run(
    config: Config,
    shutdown: impl Future<Output = ()> + Send + 'static,
) -> Result<RunningServer, StartError> {
    let (auth, logbook) = open_storage(&config.data_dir)?;
    let bus = Bus::new(4096);
    // con il simulatore trasmettere e' innocuo; con JTDX reale la trasmissione e' chiusa salvo AURORA_ALLOW_TX=1
    let live_tx = match config.engine {
        EngineKind::Sim => true,
        EngineKind::Jtdx => config.allow_tx,
    };
    let automation = Automation::spawn(
        &bus,
        AutomationConfig {
            rules: Rules::default(),
            limits: config.auto_limits.clone(),
            live_allowed: live_tx,
            tick_every: Duration::from_secs(1),
            now: Arc::new(unix_secs),
        },
    );
    let mut targets: Vec<Target> = config.export_udp.iter().map(|a| Target::UdpAdif(*a)).collect();
    targets.extend(config.export_file.clone().map(Target::File));
    // il log di Aurora si riempie sempre; i log esterni solo se configurati
    let _exporter = Exporter::spawn(&bus, Arc::clone(&logbook), targets, Arc::new(unix_secs));
    // paesi e QSO gia' fatti si leggono una volta sola, prima di avviare il motore
    let enrichment = load_dxcc(config.cty_path.as_deref()).map(|db| {
        let worked = load_worked(config.worked_adif.as_deref(), &db);
        (db, worked)
    });
    let dxcc_loaded = enrichment.as_ref().map_or(0, |(db, _)| db.len());
    let worked_qsos = enrichment.as_ref().map_or(0, |(_, w)| w.len());
    let info = serde_json::json!({
        "version": aurora_core::version::APP_VERSION,
        "engine": if config.engine == EngineKind::Jtdx { "jtdx" } else { "sim" },
        "tx_allowed": live_tx,
        "udp_listen": config.udp_listen.to_string(),
        "udp_forward": config.udp_forward.iter().map(ToString::to_string).collect::<Vec<_>>(),
        "export_udp": config.export_udp.iter().map(ToString::to_string).collect::<Vec<_>>(),
        "export_file": config.export_file.as_ref().map(|p| p.display().to_string()),
        "audio_device": config.audio_device,
        "cty_loaded": dxcc_loaded,
        "worked_qsos": worked_qsos,
        "limits": {
            "max_consecutive_cycles": config.auto_limits.max_consecutive_cycles,
            "max_per_day": config.auto_limits.max_per_day,
            "dead_man_secs": config.auto_limits.dead_man_secs,
            "run_unattended": config.auto_limits.run_unattended,
        },
    });
    let state = AppState::new(bus.clone(), auth, logbook, TxPolicy::default_safe().with_live_tx(live_tx), config.secure_cookie)
        .with_station_info(info)
        .with_automation(automation);

    let commands = bus.take_command_rx().ok_or_else(|| StartError::Io("canale dei comandi gia' preso".into()))?;
    let io = EngineIo { events: bus.events(), spectrum: bus.spectrum(), commands };
    let (engine_task, hub) = match config.engine {
        EngineKind::Sim => (tokio::spawn(SimEngine::new(SimConfig { seed: 1, cycle_ms: config.cycle_ms }).run(io)), None),
        EngineKind::Jtdx => {
            let hub = Arc::new(
                Hub::start(HubConfig { listen: config.udp_listen, forward: config.udp_forward.clone() })
                    .await
                    .map_err(|e| StartError::Io(e.to_string()))?,
            );
            let jtdx_config = JtdxConfig {
                allow_tx: config.allow_tx,
                replay_on_connect: config.replay_on_connect,
                ..JtdxConfig::default()
            };
            let mut engine = JtdxEngine::new(Arc::clone(&hub), jtdx_config, Arc::new(EngineClock));
            if let Some((db, worked)) = enrichment {
                engine = engine.with_enrichment(db, worked);
            }
            (tokio::spawn(engine.run(io)), Some(hub))
        }
    };

    let mut app = build_router(state.clone()).route("/api/{*rest}", any(|| async { StatusCode::NOT_FOUND }));
    if let Some(ui) = &config.ui_dir {
        let index = ui.join("index.html");
        app = app.fallback_service(ServeDir::new(ui).fallback(ServeFile::new(index)));
    }

    // la cascata viene dall'audio; se l'ingresso non c'e' Aurora parte lo stesso e lo dice
    let audio = config.audio_device.as_deref().and_then(|hint| match run_capture(Some(hint), bus.spectrum()) {
        Ok(handle) => Some(handle),
        Err(e) => {
            tracing::warn!("cascata audio non attiva: {e}");
            None
        }
    });

    let listener = TcpListener::bind(config.bind)
        .await
        .map_err(|e| StartError::Io(format!("{}: {e}", config.bind)))?;
    let addr = listener.local_addr().map_err(|e| StartError::Io(e.to_string()))?;

    let closing = state.clone();
    let hub_for_task = hub.clone();
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
        if let Some(hub) = hub_for_task {
            hub.stop();
        }
        drop(audio);
    });
    Ok(RunningServer { addr, state, hub, task })
}
