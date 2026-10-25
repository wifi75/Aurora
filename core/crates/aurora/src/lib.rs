//! Collega motore, log, autenticazione e API in un unico processo.

use std::{
    future::Future,
    net::SocketAddr,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

pub mod history;
pub mod online;
pub mod updates;
pub mod wanted;

use audio_spectrum::run_capture_tapped;
use auth::{DeviceStore, SystemClock};
use aurora_core::{Bus, Engine, EngineIo, Event, StationStatus};
use axum::{http::StatusCode, routing::any};
use dxcc::DxccDb;
use engine_jtdx::{worked::WorkedLog, JtdxConfig, JtdxEngine, SystemClock as EngineClock};
use engine_sim::{SimConfig, SimEngine};
use logbook::Logbook;
use qso_export::{Exporter, Target};
use rig_control::{schedule::Schedule, table::FrequencyTable, Rig};
use qso_engine::{Automation, AutomationConfig, Limits, Rules};
use remote_api::{build_router, AppState, TxPolicy};
use tokio::{net::TcpListener, sync::broadcast, task::JoinHandle};
use tower_http::services::{ServeDir, ServeFile};
use udp_hub::{Hub, HubConfig};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EngineKind {
    /// Stazione simulata, senza radio.
    Sim,
    /// JTDX reale, tramite l'hub UDP.
    Jtdx,
    /// Il motore proprio di Aurora (crate `ft8`): ascolta l'ingresso audio e decodifica da solo, senza JTDX. Per ora in sola ricezione.
    Native,
}

/// Chi sa installare un aggiornamento (il programma Windows lo da' alla partenza).
#[derive(Clone)]
pub struct UpdateInstallerHandle(pub Arc<dyn remote_api::UpdateInstaller>);

impl std::fmt::Debug for UpdateInstallerHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "UpdateInstallerHandle")
    }
}

#[derive(Debug, Clone)]
pub struct Config {
    /// Se presente, la pagina propone «Installa ora» per le versioni nuove e l'API sa installarle.
    pub update_installer: Option<UpdateInstallerHandle>,
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
    /// Per quanti giorni si ricordano i messaggi ricevuti nello storico (0 = per sempre).
    pub history_days: u32,
    /// Con JTDX: chiede il Replay dei decode correnti appena lo vede. Spento per impostazione predefinita: aprendo
    /// Aurora non si rivedono le stazioni ricevute prima. Se acceso, chi riceve il flusso (GridTracker2) le riceve di
    /// nuovo, marcate come "non nuove".
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
    /// Comando della radio via OmniRig (la stessa che usa JTDX): cambio banda dall'interfaccia.
    pub rig_omnirig: bool,
    /// Durata del codice di associazione, in secondi (60-3600).
    pub pair_ttl_secs: u64,
    /// Accesso senza codice per le prove (solo rete di casa). Spento per impostazione predefinita.
    pub open_access: bool,
    /// Chi arriva da questo PC entra senza codice (programma Windows). Dall'ambiente: `AURORA_TRUST_LOCAL=1`.
    pub local_trust: bool,
    /// Il motore proprio decodifica in ombra dall'audio e si confronta con JTDX. Dall'ambiente: `AURORA_NATIVE_DECODE=1`.
    pub native_decode: bool,
    /// Salva ogni periodo in WAV (`recordings` nella cartella dei dati) per provare il decoder. Dall'ambiente: `AURORA_NATIVE_RECORD=1`.
    pub native_record: bool,
    /// Database SQLite di Log4OM2, letto solo in lettura per mostrare gli ultimi QSO.
    pub log4om_db: Option<PathBuf>,
    /// `JTDX.ini` di JTDX, letto in sola lettura per mostrare le sue impostazioni.
    pub jtdx_ini: Option<PathBuf>,
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
            update_installer: None,
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
            history_days: 30,
            replay_on_connect: false,
            cty_path: None,
            worked_adif: None,
            auto_limits: Limits::default(),
            export_udp: Vec::new(),
            export_file: None,
            rig_omnirig: false,
            pair_ttl_secs: 600,
            open_access: false,
            local_trust: false,
            native_decode: false,
            native_record: false,
            log4om_db: None,
            jtdx_ini: None,
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
            Some("native") => EngineKind::Native,
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
            None | Some("0" | "false") => false,
            Some("1" | "true") => true,
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
            update_installer: None,
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
            history_days: get("AURORA_HISTORY_DAYS").and_then(|d| d.trim().parse().ok()).unwrap_or(30),
            replay_on_connect,
            cty_path: get("AURORA_CTY_PATH").filter(|p| !p.trim().is_empty()).map(PathBuf::from),
            worked_adif: get("AURORA_WORKED_ADIF").filter(|p| !p.trim().is_empty()).map(PathBuf::from),
            auto_limits,
            jtdx_ini: get("AURORA_JTDX_INI").filter(|p| !p.trim().is_empty()).map(PathBuf::from),
            log4om_db: get("AURORA_LOG4OM_DB").filter(|p| !p.trim().is_empty()).map(PathBuf::from),
            export_udp,
            open_access: match get("AURORA_OPEN_ACCESS").as_deref() {
                None | Some("0") => false,
                Some("1") => true,
                Some(other) => return Err(StartError::BadConfig(format!("AURORA_OPEN_ACCESS deve essere 0 o 1, non {other}"))),
            },
            local_trust: match get("AURORA_TRUST_LOCAL").as_deref() {
                None | Some("0") => false,
                Some("1") => true,
                Some(other) => return Err(StartError::BadConfig(format!("AURORA_TRUST_LOCAL deve essere 0 o 1, non {other}"))),
            },
            native_decode: match get("AURORA_NATIVE_DECODE").as_deref() {
                None | Some("0") => false,
                Some("1") => true,
                Some(other) => return Err(StartError::BadConfig(format!("AURORA_NATIVE_DECODE deve essere 0 o 1, non {other}"))),
            },
            native_record: match get("AURORA_NATIVE_RECORD").as_deref() {
                None | Some("0") => false,
                Some("1") => true,
                Some(other) => return Err(StartError::BadConfig(format!("AURORA_NATIVE_RECORD deve essere 0 o 1, non {other}"))),
            },
            pair_ttl_secs: match get("AURORA_PAIR_TTL_SECS") {
                None => 600,
                Some(v) => v
                    .trim()
                    .parse::<u64>()
                    .ok()
                    .filter(|n| (60..=3600).contains(n))
                    .ok_or_else(|| StartError::BadConfig(format!("AURORA_PAIR_TTL_SECS deve essere tra 60 e 3600, non {v}")))?,
            },
            rig_omnirig: match get("AURORA_RIG").as_deref() {
                None | Some("none") | Some("") => false,
                Some("omnirig") => true,
                Some(other) => return Err(StartError::BadConfig(format!("AURORA_RIG deve essere none o omnirig, non {other}"))),
            },
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

/// Applica le impostazioni a JTDX: lo chiude (se e' acceso), scrive il suo file con una copia di sicurezza e lo riavvia
/// come stava. Se la scrittura fallisce JTDX viene comunque riavviato, cosi' non si resta senza.
fn apply_jtdx_settings(path: &Path, changes: &std::collections::BTreeMap<String, String>) -> Result<(), String> {
    let running = rig_control::jtdx_process::find().map_err(|e| e.to_string())?;
    if let Some(r) = &running {
        rig_control::jtdx_process::stop(r.id, 25).map_err(|e| e.to_string())?;
    }
    let written = engine_jtdx::ini_edit::apply_changes(path, changes);
    if let Some(r) = &running {
        rig_control::jtdx_process::start(r).map_err(|e| format!("JTDX non riavviato: {e}"))?;
    }
    written
}

fn open_storage(data_dir: &Path, pair_ttl_secs: u64) -> Result<(Arc<DeviceStore>, Arc<Logbook>), StartError> {
    std::fs::create_dir_all(data_dir).map_err(|e| StartError::Io(format!("{}: {e}", data_dir.display())))?;
    let auth = DeviceStore::open(&data_dir.join("devices.db"), Arc::new(SystemClock))
        .map_err(|e| StartError::Storage(e.to_string()))?
        .with_code_ttl(pair_ttl_secs);
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

/// Frequenze salvate; un file mancante o non valido vale come tabella predefinita.
fn load_frequencies(path: &Path) -> FrequencyTable {
    let mut table = FrequencyTable::default();
    if let Ok(text) = std::fs::read_to_string(path) {
        let loaded = serde_json::from_str::<Vec<rig_control::table::Entry>>(&text).map_err(|e| e.to_string()).and_then(|entries| table.set(entries).map_err(String::from));
        if loaded.is_err() {
            tracing::warn!("frequenze non valide in {}: uso quelle predefinite", path.display());
        }
    }
    table
}

#[derive(serde::Deserialize, serde::Serialize)]
struct ScheduleFile {
    enabled: bool,
    entries: Vec<rig_control::schedule::ScheduleEntry>,
}

fn load_schedule(path: &Path) -> Schedule {
    let mut schedule = Schedule::default();
    if let Ok(text) = std::fs::read_to_string(path) {
        let loaded = serde_json::from_str::<ScheduleFile>(&text)
            .map_err(|e| e.to_string())
            .and_then(|file| schedule.set(file.enabled, file.entries).map_err(String::from));
        if loaded.is_err() {
            tracing::warn!("programmazione non valida in {}: ignorata", path.display());
        }
    }
    schedule
}

fn load_pair_ttl(path: &Path, default: u64) -> u64 {
    let Ok(text) = std::fs::read_to_string(path) else { return default };
    match serde_json::from_str::<serde_json::Value>(&text).ok().and_then(|v| v["code_ttl_secs"].as_u64()) {
        Some(secs) if (60..=3600).contains(&secs) => secs,
        _ => {
            tracing::warn!("durata del codice non valida in {}: uso quella predefinita", path.display());
            default
        }
    }
}

fn save_settings(dir: &Path, auth: &DeviceStore, frequencies: &Mutex<FrequencyTable>, schedule: &Mutex<Schedule>) {
    if let Err(e) = std::fs::write(dir.join("pairing.json"), serde_json::json!({ "code_ttl_secs": auth.code_ttl_secs() }).to_string()) {
        tracing::warn!("durata del codice non salvata: {e}");
    }
    let table = frequencies.lock().unwrap_or_else(|p| p.into_inner());
    if let Ok(json) = serde_json::to_string(table.entries())
        && let Err(e) = std::fs::write(dir.join("frequencies.json"), json)
    {
        tracing::warn!("frequenze non salvate: {e}");
    }
    drop(table);
    let schedule = schedule.lock().unwrap_or_else(|p| p.into_inner());
    let file = ScheduleFile { enabled: schedule.enabled(), entries: schedule.entries().to_vec() };
    if let Ok(json) = serde_json::to_string(&file)
        && let Err(e) = std::fs::write(dir.join("schedule.json"), json)
    {
        tracing::warn!("programmazione non salvata: {e}");
    }
}

/// Ogni 15 secondi guarda l'ora UTC: al minuto di una riga porta la radio sulla banda, se non si trasmette.
/// Lo stato di partenza dalla finestra di JTDX. Dopo un riavvio di Aurora, JTDX manda il suo stato UDP solo a fine periodo (fino
/// a 15 secondi dopo) e nel frattempo la pagina resta vuota: lo si legge subito dalla sua finestra, una volta; il primo stato vero
/// lo sostituisce. Se JTDX non e' ancora pronto si riprova per un po'.
fn spawn_status_bootstrap(state: AppState, events: broadcast::Sender<Event>, rig: Arc<dyn Rig>, ini: Option<std::path::PathBuf>) {
    tokio::spawn(async move {
        let (call, grid) = ini
            .as_deref()
            .and_then(|path| engine_jtdx::ini::read_shown(path).ok())
            .map(|settings| (settings.get("MyCall").cloned().unwrap_or_default(), settings.get("MyGrid").cloned().unwrap_or_default()))
            .unwrap_or_default();
        for _ in 0..8 {
            if state.has_status() {
                return;
            }
            let reader = Arc::clone(&rig);
            match tokio::task::spawn_blocking(move || reader.engine_snapshot()).await {
                Ok(Ok(snapshot)) if snapshot.dial_hz.is_some() => {
                    // nel frattempo puo' essere arrivato lo stato vero: quello vale di piu'
                    if !state.has_status() {
                        tracing::info!("stato iniziale letto dalla finestra di JTDX: {} Hz", snapshot.dial_hz.unwrap_or_default());
                        let _ = events.send(Event::Status(snapshot_to_status(&snapshot, &call, &grid)));
                    }
                    return;
                }
                Ok(Ok(_)) => tracing::debug!("la finestra di JTDX non ha dato la frequenza: si riprova"),
                Ok(Err(e)) => tracing::debug!("stato iniziale di JTDX non letto: {e}"),
                Err(e) => tracing::debug!("lettura dello stato iniziale interrotta: {e}"),
            }
            tokio::time::sleep(Duration::from_secs(2)).await;
        }
    });
}

/// Lo stato per l'interfaccia a partire da quello letto dalla finestra di JTDX e dal suo file di impostazioni.
fn snapshot_to_status(snapshot: &rig_control::snapshot::Snapshot, de_call: &str, de_grid: &str) -> StationStatus {
    StationStatus {
        dial_hz: snapshot.dial_hz.unwrap_or_default(),
        mode: snapshot.mode.clone().unwrap_or_default(),
        tx_enabled: snapshot.tx_enabled,
        transmitting: snapshot.transmitting,
        decoding: false,
        de_call: de_call.to_string(),
        de_grid: de_grid.to_string(),
        dx_call: snapshot.dx_call.clone(),
        dx_grid: snapshot.dx_grid.clone(),
        report: snapshot.report.clone(),
        tx_df: snapshot.tx_df,
        rx_df: snapshot.rx_df,
        tx_message: None,
        tr_period: None,
        own_continent: None,
    }
}

fn spawn_scheduler(state: AppState, rig: Arc<dyn Rig>) {
    tokio::spawn(async move {
        let mut last: Option<(u64, u8, u8)> = None;
        let mut tick = tokio::time::interval(Duration::from_secs(15));
        loop {
            tick.tick().await;
            let now = unix_secs();
            let (hour, minute) = rig_control::schedule::hm_of(now);
            let key = (now / 86_400, hour, minute);
            if last == Some(key) {
                continue;
            }
            let band = {
                let schedule = state.schedule.lock().unwrap_or_else(|p| p.into_inner());
                schedule.due(hour, minute).map(|e| e.band.clone())
            };
            let Some(band) = band else { continue };
            last = Some(key);
            if state.is_transmitting() {
                tracing::info!("programmazione: {band} rinviata, si sta trasmettendo");
                last = None;
                continue;
            }
            let found = {
                let table = state.frequencies.lock().unwrap_or_else(|p| p.into_inner());
                table.frequency(&band, &state.current_mode())
            };
            if let Some(hz) = found {
                let rig = Arc::clone(&rig);
                match tokio::task::spawn_blocking(move || rig.set_frequency(hz)).await {
                    Ok(Ok(())) => tracing::info!("programmazione: radio portata su {band}"),
                    other => tracing::warn!("programmazione: cambio banda fallito: {other:?}"),
                }
            }
        }
    });
}

/// Regole salvate dall'ultima volta; un file mancante o non valido vale come regole predefinite.
fn load_rules(path: &Path) -> Rules {
    match std::fs::read_to_string(path) {
        Ok(text) => match serde_json::from_str::<Rules>(&text) {
            Ok(rules) if rules.is_valid() => rules,
            _ => {
                tracing::warn!("regole dell'automazione non valide in {}: uso quelle predefinite", path.display());
                Rules::default()
            }
        },
        Err(_) => Rules::default(),
    }
}

fn save_rules(path: &Path, rules: &Rules) {
    match serde_json::to_string(rules) {
        Ok(json) => {
            if let Err(e) = std::fs::write(path, json) {
                tracing::warn!("regole non salvate in {}: {e}", path.display());
            }
        }
        Err(e) => tracing::warn!("regole non serializzabili: {e}"),
    }
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
    let pair_ttl = load_pair_ttl(&config.data_dir.join("pairing.json"), config.pair_ttl_secs);
    let (auth, logbook) = open_storage(&config.data_dir, pair_ttl)?;
    let bus = Bus::new(4096);
    // con il simulatore trasmettere e' innocuo; con JTDX reale la trasmissione e' chiusa salvo AURORA_ALLOW_TX=1
    let native_controls = Arc::new(engine_native::NativeControls::default());
    let live_tx = match config.engine {
        EngineKind::Sim => true,
        EngineKind::Jtdx => config.allow_tx,
        // la trasmissione del motore proprio non c'e' ancora: i comandi di trasmissione restano chiusi
        EngineKind::Native => false,
    };
    let frequencies = Arc::new(Mutex::new(load_frequencies(&config.data_dir.join("frequencies.json"))));
    let schedule = Arc::new(Mutex::new(load_schedule(&config.data_dir.join("schedule.json"))));
    let settings_saver: Arc<dyn Fn() + Send + Sync> = {
        let (frequencies, schedule, dir, auth) = (Arc::clone(&frequencies), Arc::clone(&schedule), config.data_dir.clone(), Arc::clone(&auth));
        Arc::new(move || save_settings(&dir, &auth, &frequencies, &schedule))
    };
    let rules_path = config.data_dir.join("automation-rules.json");
    let automation = Automation::spawn(
        &bus,
        AutomationConfig {
            batch_window: Duration::from_millis(350),
            rules: load_rules(&rules_path),
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
    // il continente di chi opera (dal nominativo di JTDX): serve a rispettare i CQ direzionali («CQ NA», «CQ DX»)
    if let Some((db, _)) = &enrichment
        && let Some(call) = config.jtdx_ini.as_deref().and_then(|p| engine_jtdx::ini::read_shown(p).ok()).and_then(|m| m.get("MyCall").cloned())
        && let Some(hit) = db.lookup(&call)
    {
        automation.set_own_continent(Some(db.entity(hit.entity).continent.clone()));
    }
    let stats_source = enrichment.as_ref().zip(config.worked_adif.clone()).map(|((db, _), path)| (Arc::clone(db), path));
    let dxcc_loaded = enrichment.as_ref().map_or(0, |(db, _)| db.len());
    let worked_qsos = enrichment.as_ref().map_or(0, |(_, w)| w.len());
    let info = serde_json::json!({
        "version": aurora_core::version::APP_VERSION,
        "engine": match config.engine { EngineKind::Jtdx => "jtdx", EngineKind::Native => "native", EngineKind::Sim => "sim" },
        "tx_allowed": live_tx,
        "udp_listen": config.udp_listen.to_string(),
        "udp_forward": config.udp_forward.iter().map(ToString::to_string).collect::<Vec<_>>(),
        "export_udp": config.export_udp.iter().map(ToString::to_string).collect::<Vec<_>>(),
        "rig_control": config.rig_omnirig,
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
    let clock = clock_sync::ClockService::open(&config.data_dir.join("clock.json"), clock_sync::default_query(), Arc::new(clock_sync::PowerShellSetter));
    let _clock_task = clock.spawn();
    // DX Cluster: paese degli spot dal file cty.dat; nominativo di JTDX se non ne e' stato scelto uno; parte solo se acceso
    let wishlist = wishlist::WishlistService::open(&config.data_dir.join("wishlist.json"));
    // la regola «rispondi ai desiderati» dell'automazione guarda la stessa lista che avvisa
    {
        let list = Arc::clone(&wishlist);
        automation.set_wanted_matcher(Arc::new(move |d, dial| {
            wanted::alert_for_decode(&list.config(), d, dial, 0).map(|a| {
                match a.reason.as_str() {
                    "call" => "nominativo",
                    "prefix" => "prefisso",
                    "country" => "paese",
                    "grid" => "griglia",
                    other => other,
                }
                .to_string()
            })
        }));
    }
    let updates = updater::UpdateService::open(&config.data_dir.join("update.json"), env!("CARGO_PKG_VERSION"));
    let cluster = dx_cluster::ClusterService::open(&config.data_dir.join("dx-cluster.json"));
    // il livello dell'ingresso audio si regola sullo stesso dispositivo da cui viene la cascata
    let input_level = config.audio_device.as_deref().and_then(|hint| match audio_spectrum::InputEndpoint::find(hint) {
        Ok(endpoint) => {
            let service = input_level::InputLevelService::open(Arc::new(endpoint), &config.data_dir.join("input-level.json"));
            match service.status() {
                Ok(_) => Some(service),
                Err(e) => {
                    tracing::warn!("livello dell'ingresso audio non regolabile: {e}");
                    None
                }
            }
        }
        Err(e) => {
            tracing::warn!("livello dell'ingresso audio non regolabile: {e}");
            None
        }
    });
    if let Some(service) = &input_level {
        service.spawn_auto(bus.subscribe_events());
    }
    // PSK Reporter e propagazione: spenti finche' non si accendono
    let online = online_info::OnlineService::open(&config.data_dir.join("online.json"), Arc::new(online_info::HttpFetch::new(env!("CARGO_PKG_VERSION"))));
    online::spawn_refresher(Arc::clone(&online), bus.subscribe_events());
    // lo storico dei messaggi ricevuti: se il file non si apre Aurora parte lo stesso e lo dice
    let history = match decode_history::HistoryStore::open(&config.data_dir.join("history.sqlite")) {
        Ok(store) => {
            let store = Arc::new(store);
            history::spawn_recorder(Arc::clone(&store), bus.subscribe_events(), config.history_days);
            Some(store)
        }
        Err(e) => {
            tracing::warn!("{e}");
            None
        }
    };
    if let Some((db, _)) = enrichment.as_ref() {
        let db = Arc::clone(db);
        cluster.set_enricher(Arc::new(move |call: &str| {
            db.lookup(call).map(|l| {
                let entity = db.entity(l.entity);
                (entity.name.clone(), entity.continent.clone())
            })
        }));
    }
    if let Some(call) = config.jtdx_ini.as_deref().and_then(|p| engine_jtdx::ini::read_shown(p).ok()).and_then(|m| m.get("MyCall").cloned()) {
        cluster.default_callsign(&call);
    }
    cluster.restart();
    let state = AppState::new(bus.clone(), auth, logbook, TxPolicy::default_safe().with_live_tx(live_tx), config.secure_cookie)
        .with_station_info(info)
        .with_open_access(config.open_access)
        .with_local_trust(config.local_trust)
        .with_clock(Arc::clone(&clock))
        .with_cluster(Arc::clone(&cluster))
        .with_wishlist(Arc::clone(&wishlist))
        .with_input_level_if(input_level)
        .with_update_installer_if(config.update_installer.clone().map(|h| h.0))
        .with_online(online)
        .with_history_if(history)
        .with_updates(Arc::clone(&updates))
        .with_dxcc_if(enrichment.as_ref().map(|(db, _)| {
            let db = Arc::clone(db);
            Arc::new(move |call: &str| {
                db.lookup(call).map(|l| {
                    let e = db.entity(l.entity);
                    serde_json::json!({ "country": e.name, "continent": e.continent, "prefix": e.prefix, "cq_zone": l.cq_zone, "itu_zone": l.itu_zone })
                })
            }) as remote_api::DxccProvider
        }))
        .with_ui_settings(Arc::new(remote_api::ui_settings::UiSettings::open(&config.data_dir.join("ui-settings.json"))))
        .with_stats_if(stats_source.map(|(db, path)| {
            Arc::new(move || {
                let worked = load_worked(Some(&path), &db);
                serde_json::json!({ "stats": worked.stats(&db), "missing_dxcc": worked.missing_dxcc(&db) })
            }) as Arc<dyn Fn() -> serde_json::Value + Send + Sync>
        }))
        .with_log4om_if(config.log4om_db.clone().map(|path| {
            Arc::new(move |limit: usize| {
                logbook::log4om::read_recent(&path, limit)
                    .map(|view| serde_json::json!(view))
                    .map_err(|e| e.to_string())
            }) as remote_api::Log4omProvider
        }))
        .with_worked_map_if(config.log4om_db.clone().map(|path| {
            Arc::new(move || {
                logbook::log4om::worked_squares(&path)
                    .map(|squares| serde_json::json!({ "squares": squares }))
                    .map_err(|e| e.to_string())
            }) as remote_api::WorkedMapProvider
        }))
        .with_engine_apply_if(config.jtdx_ini.clone().map(|path| {
            Arc::new(move |changes: std::collections::BTreeMap<String, String>| apply_jtdx_settings(&path, &changes)) as remote_api::JtdxApplier
        }))
        .with_engine_config_if(config.jtdx_ini.clone().map(|path| {
            Arc::new(move || engine_jtdx::ini::read_shown(&path).map(|settings| serde_json::json!(settings))) as remote_api::JtdxConfigProvider
        }))
        .with_frequencies(Arc::clone(&frequencies))
        .with_schedule(Arc::clone(&schedule))
        .with_settings_saver(settings_saver)
        .with_rules_saver(Arc::new(move |rules: &Rules| save_rules(&rules_path, rules)))
        .with_rig_if(match config.engine {
            // motore proprio: i pulsanti agiscono sul decoder; frequenza e cambio banda vanno alla radio (OmniRig) se c'e'
            EngineKind::Native => {
                let radio = config.rig_omnirig.then(|| Arc::new(rig_control::WindowsRig) as Arc<dyn rig_control::Rig>);
                Some(Arc::new(engine_native::NativeRig::new(Arc::clone(&native_controls), radio)) as Arc<dyn rig_control::Rig>)
            }
            _ => config.rig_omnirig.then(|| {
                // le sessioni PowerShell verso JTDX si aprono subito: il primo comando (Abilita Tx...) non deve aspettarle
                rig_control::WindowsRig::keep_warm();
                Arc::new(rig_control::WindowsRig) as Arc<dyn rig_control::Rig>
            }),
        })
        .with_automation(automation);

    let commands = bus.take_command_rx().ok_or_else(|| StartError::Io("canale dei comandi gia' preso".into()))?;
    let io = EngineIo { events: bus.events(), spectrum: bus.spectrum(), commands };
    let dial_hz = Arc::new(std::sync::atomic::AtomicU64::new(0));
    let mut native_live: Option<engine_native::LiveConfig> = None;
    let (engine_task, hub) = match config.engine {
        EngineKind::Native => {
            // il motore proprio decodifica dall'audio della cascata (avviato piu' sotto); qui si prepara cio' che pubblica
            let (call, grid) = config
                .jtdx_ini
                .as_deref()
                .and_then(|p| engine_jtdx::ini::read_shown(p).ok())
                .map(|m| (m.get("MyCall").cloned().unwrap_or_default(), m.get("MyGrid").cloned().unwrap_or_default()))
                .unwrap_or_default();
            let own_continent = enrichment.as_ref().and_then(|(db, _)| db.lookup(&call).map(|hit| db.entity(hit.entity).continent.clone()));
            let enrich = enrichment.map(|(db, worked)| Arc::new(Mutex::new(engine_jtdx::enrich::Enrichment::new(db, worked))));
            native_live = Some(engine_native::LiveConfig {
                de_call: call,
                de_grid: grid,
                own_continent,
                dial_hz: Arc::clone(&dial_hz),
                controls: Arc::clone(&native_controls),
                band_of: Arc::new(|hz| rig_control::bands::amateur_band(hz).map(str::to_string)),
                annotate: Arc::new(move |text, band| match &enrich {
                    Some(e) => match e.lock() {
                        Ok(e) => {
                            let a = e.annotate(text, band);
                            (a.country, a.continent, a.tag)
                        }
                        Err(_) => (None, None, None),
                    },
                    None => (None, None, None),
                }),
            });
            // i comandi di trasmissione non sono ancora dell'engine proprio: si leggono e si dice perche' non partono
            let mut commands = io.commands;
            let task = tokio::spawn(async move {
                while let Some(command) = commands.recv().await {
                    tracing::warn!("comando {command:?} ignorato: la trasmissione del motore proprio non e' ancora pronta");
                }
            });
            (task, None)
        }
        EngineKind::Sim => (tokio::spawn(SimEngine::new(SimConfig { seed: 1, cycle_ms: config.cycle_ms }).run(io)), None),
        EngineKind::Jtdx => {
            let hub = Arc::new(
                Hub::start(HubConfig { listen: config.udp_listen, forward: config.udp_forward.clone() })
                    .await
                    .map_err(|e| StartError::Io(e.to_string()))?,
            );
            let engine_config = JtdxConfig {
                allow_tx: config.allow_tx,
                replay_on_connect: config.replay_on_connect,
                peer_cache: Some(config.data_dir.join("jtdx-peer.txt")),
                ..JtdxConfig::default()
            };
            let mut engine = JtdxEngine::new(Arc::clone(&hub), engine_config, Arc::new(EngineClock));
            if let Some((db, worked)) = enrichment {
                engine = engine.with_enrichment(db, worked);
            }
            (tokio::spawn(engine.run(io)), Some(hub))
        }
    };

    // la lista dei desiderati: guarda messaggi e spot e avvisa (anche a pagina chiusa)
    wanted::spawn_watcher(Arc::clone(&wishlist), bus.events(), Some(Arc::clone(&cluster)));

    // le nuove versioni sul repository: un controllo poco dopo l'avvio e poi ogni qualche ora
    {
        let phone = Arc::clone(&wishlist);
        updates::spawn_checker(Arc::clone(&updates), bus.events(), Arc::new(move || phone.config().notify_url), Duration::from_secs(45));
    }

    // la frequenza della radio per il motore proprio: si legge da OmniRig ogni pochi secondi
    if config.engine == EngineKind::Native
        && let Some(rig) = state.rig.clone()
    {
        let dial = Arc::clone(&dial_hz);
        std::thread::spawn(move || {
            loop {
                if let Ok(hz) = rig.frequency() {
                    dial.store(hz, std::sync::atomic::Ordering::Relaxed);
                }
                std::thread::sleep(Duration::from_secs(3));
            }
        });
    }

    // la programmazione delle bande ha senso solo se la radio e' comandabile
    if let Some(rig) = state.rig.clone() {
        spawn_scheduler(state.clone(), rig.clone());
        if config.engine == EngineKind::Jtdx {
            spawn_status_bootstrap(state.clone(), bus.events(), rig, config.jtdx_ini.clone());
        }
    }

    let mut app = build_router(state.clone()).route("/api/{*rest}", any(|| async { StatusCode::NOT_FOUND }));
    if let Some(ui) = &config.ui_dir {
        let index = ui.join("index.html");
        // il browser deve ricontrollare la pagina a ogni apertura: senza, dopo un aggiornamento resta quella vecchia
        app = app.fallback_service(ServeDir::new(ui).fallback(ServeFile::new(index))).layer(axum::middleware::map_response(
            |mut response: axum::response::Response| async move {
                response
                    .headers_mut()
                    .entry(axum::http::header::CACHE_CONTROL)
                    .or_insert(axum::http::HeaderValue::from_static("no-cache"));
                response
            },
        ));
    }

    // la cascata viene dall'audio; se l'ingresso non c'e' Aurora parte lo stesso e lo dice
    let (native_tx, native_rx) = std::sync::mpsc::channel();
    let tap = (config.native_decode || config.engine == EngineKind::Native).then_some(native_tx);
    let audio = config.audio_device.as_deref().and_then(|hint| match run_capture_tapped(Some(hint), bus.spectrum(), bus.events(), tap) {
        Ok(handle) => Some(handle),
        Err(e) => {
            tracing::warn!("cascata audio non attiva: {e}");
            None
        }
    });

    // il motore proprio decodifica in ombra l'audio della cascata e si confronta con JTDX
    if (config.native_decode || config.engine == EngineKind::Native) && audio.is_some() {
        let engine = engine_native::start(bus.events(), native_rx, config.data_dir.clone(), config.native_record, native_live.take());
        state.set_native_status(Arc::new(move || engine.status()));
    }

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_status_read_from_the_jtdx_window_fills_the_page_until_the_real_one_arrives() {
        let snapshot = rig_control::snapshot::Snapshot {
            dial_hz: Some(14_080_000),
            mode: Some("FT4".into()),
            dx_call: Some("M7SZY".into()),
            dx_grid: None,
            tx_df: Some(1084),
            rx_df: Some(1034),
            report: Some("-2".into()),
            transmitting: false,
            tx_enabled: true,
        };
        let status = snapshot_to_status(&snapshot, "IU3ABC", "IO91wm");
        assert_eq!((status.dial_hz, status.mode.as_str()), (14_080_000, "FT4"));
        assert_eq!((status.de_call.as_str(), status.de_grid.as_str()), ("IU3ABC", "IO91wm"));
        assert_eq!(status.dx_call.as_deref(), Some("M7SZY"));
        assert_eq!((status.tx_df, status.rx_df), (Some(1084), Some(1034)));
        assert!(status.tx_enabled && !status.transmitting && !status.decoding);
        assert!(status.tx_message.is_none());
    }
}
