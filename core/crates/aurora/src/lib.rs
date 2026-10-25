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
use engine_jtdx::{enrich::SharedWorked, worked::WorkedLog, JtdxConfig, JtdxEngine, SystemClock as EngineClock};
use engine_sim::{SimConfig, SimEngine};
use logbook::{adif::Station, Logbook};
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
    /// File ADIF a cui aggiungere ogni QSO (oltre a `aurora_log.adi` nella cartella dei dati, che il motore proprio scrive sempre).
    pub export_file: Option<PathBuf>,
    /// Potenza in watt da scrivere in `TX_PWR` di ogni QSO (`AURORA_TX_POWER`); vuota come in JTDX con «Save Tx power» spento.
    pub tx_power: Option<String>,
    /// Comando della radio via OmniRig (la stessa che usa JTDX): cambio banda dall'interfaccia.
    pub rig_omnirig: bool,
    /// Comando della radio via Hamlib (`rigctld`): tutte le radio. Si configura dalla scheda Radio delle impostazioni (`radio.json`).
    pub rig_hamlib: bool,
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
    /// Uscita audio per la trasmissione (parte del nome). Dall'ambiente: `AURORA_AUDIO_OUT_DEVICE`.
    pub audio_out_device: Option<String>,
    /// Metodo del PTT: `rts`, `dtr`, `omnirig` o `vox` (di base `vox`). Dall'ambiente: `AURORA_PTT_METHOD`.
    pub ptt_method: String,
    /// Porta seriale del PTT per `rts` e `dtr` (`COM9`). Dall'ambiente: `AURORA_PTT_PORT`.
    pub ptt_port: Option<String>,
    /// Database SQLite di Log4OM2, letto solo in lettura per mostrare gli ultimi QSO.
    pub log4om_db: Option<PathBuf>,
    /// `JTDX.ini` di JTDX, letto in sola lettura per mostrare le sue impostazioni.
    pub jtdx_ini: Option<PathBuf>,
    /// Con il motore proprio: Aurora manda a `udp_forward` il flusso WSJT-X che mandava JTDX (Heartbeat, Status, Decode, QSO Logged...).
    /// Di base acceso; `AURORA_UDP_EMIT=0` lo spegne. Con il motore JTDX non serve: l'hub inoltra i pacchetti di JTDX.
    pub udp_emit: bool,
    /// `AcceptUDPRequests` di JTDX: i comandi di GridTracker (Reply, HaltTx, FreeText...) arrivano ad Aurora. `AURORA_UDP_ACCEPT=1`.
    pub udp_accept_requests: bool,
    /// `AcceptUDPReplyMessages` di JTDX: 1 solo CQ, 2 CQ e 73, 3 tutti. `AURORA_UDP_ACCEPT_REPLY`.
    pub udp_accept_reply: u8,
    /// `EnableUDP1adifSending` di JTDX: dopo il QSO Logged manda anche il Logged ADIF. `AURORA_UDP_ADIF=1`.
    pub udp_adif: bool,
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
            allow_tx: true,
            audio_device: None,
            history_days: 30,
            replay_on_connect: false,
            cty_path: None,
            worked_adif: None,
            auto_limits: Limits::default(),
            export_udp: Vec::new(),
            export_file: None,
            tx_power: None,
            rig_omnirig: false,
            rig_hamlib: false,
            pair_ttl_secs: 600,
            open_access: false,
            local_trust: false,
            native_decode: false,
            native_record: false,
            audio_out_device: None,
            ptt_method: "vox".to_string(),
            ptt_port: None,
            log4om_db: None,
            jtdx_ini: None,
            udp_emit: true,
            udp_accept_requests: false,
            udp_accept_reply: 1,
            udp_adif: false,
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
            allow_tx: true,
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
            audio_out_device: get("AURORA_AUDIO_OUT_DEVICE").filter(|d| !d.trim().is_empty()),
            ptt_method: match get("AURORA_PTT_METHOD").as_deref().map(str::to_lowercase).as_deref() {
                None | Some("") => "vox".to_string(),
                Some(m @ ("vox" | "rts" | "dtr" | "omnirig")) => m.to_string(),
                Some(other) => return Err(StartError::BadConfig(format!("AURORA_PTT_METHOD sconosciuto: {other} (vox, rts, dtr, omnirig)"))),
            },
            ptt_port: get("AURORA_PTT_PORT").filter(|p| !p.trim().is_empty()),
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
                None | Some("none") | Some("") | Some("hamlib") => false,
                Some("omnirig") => true,
                Some(other) => return Err(StartError::BadConfig(format!("AURORA_RIG deve essere none, omnirig o hamlib, non {other}"))),
            },
            rig_hamlib: get("AURORA_RIG").as_deref() == Some("hamlib"),
            export_file: get("AURORA_EXPORT_FILE").filter(|p| !p.trim().is_empty()).map(PathBuf::from),
            tx_power: get("AURORA_TX_POWER").map(|p| p.trim().to_string()).filter(|p| !p.is_empty()),
            udp_emit: flag(&get, "AURORA_UDP_EMIT", true)?,
            udp_accept_requests: flag(&get, "AURORA_UDP_ACCEPT", false)?,
            udp_accept_reply: match get("AURORA_UDP_ACCEPT_REPLY").as_deref().map(str::trim) {
                None | Some("") => 1,
                Some(v) => v
                    .parse::<u8>()
                    .ok()
                    .filter(|n| (1..=3).contains(n))
                    .ok_or_else(|| StartError::BadConfig(format!("AURORA_UDP_ACCEPT_REPLY deve essere 1, 2 o 3, non {v}")))?,
            },
            udp_adif: flag(&get, "AURORA_UDP_ADIF", false)?,
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

/// Dove vanno i QSO registrati e da dove si leggono quelli gia' fatti.
///
/// Destinatari: i server UDP di `AURORA_EXPORT_UDP`, il file di `AURORA_EXPORT_FILE` e, con il motore proprio, `aurora_log.adi` nella cartella
/// dei dati (senza JTDX non c'e' il suo `wsjtx_log.adi`, che Aurora non tocca mai). Lista dei lavorati: `AURORA_WORKED_ADIF` (di norma
/// `wsjtx_log.adi` di JTDX) e il file di Aurora, se c'e'.
fn export_targets(config: &Config) -> (Vec<Target>, Vec<PathBuf>) {
    let own = config.data_dir.join(OWN_LOG_FILE);
    let mut targets: Vec<Target> = config.export_udp.iter().map(|a| Target::UdpAdif(*a)).collect();
    if config.engine == EngineKind::Native {
        targets.push(Target::File(own.clone()));
    }
    if let Some(file) = &config.export_file
        && !targets.contains(&Target::File(file.clone()))
    {
        targets.push(Target::File(file.clone()));
    }
    let mut worked: Vec<PathBuf> = config.worked_adif.iter().cloned().collect();
    if (config.engine == EngineKind::Native || own.is_file()) && !worked.contains(&own) {
        worked.push(own);
    }
    (targets, worked)
}

/// Apre le impostazioni del motore proprio. Se il file di Aurora non c'e' ancora, i valori iniziali vengono dal `JTDX.ini` (una volta sola) e,
/// per il primo server UDP, da `AURORA_UDP_FORWARD` quando JTDX puntava all'ascolto di Aurora (vedi `engine_settings::import_from_ini`).
fn open_engine_settings(config: &Config) -> Arc<remote_api::engine_settings::EngineSettingsStore> {
    use remote_api::engine_settings::{import_from_ini, EngineSettingsStore, FILE_NAME};
    let (ini, listen, forward) = (config.jtdx_ini.clone(), config.udp_listen, config.udp_forward.clone());
    Arc::new(EngineSettingsStore::open(&config.data_dir.join(FILE_NAME), move || {
        let shown = ini.as_deref().and_then(|p| engine_jtdx::ini::read_shown(p).ok()).unwrap_or_default();
        import_from_ini(&shown, listen, &forward)
    }))
}

/// I destinatari del QSO adesso: i file e le altre uscite fisse di `base`, piu' i server UDP dell'ambiente e il secondo server delle
/// impostazioni, senza doppioni (regola anti-doppione di `engine_settings::export_udp_targets`).
fn live_targets(base: &[Target], env_udp: &[SocketAddr], reports: &remote_api::engine_settings::Reports) -> Vec<Target> {
    let mut out: Vec<Target> = remote_api::engine_settings::export_udp_targets(env_udp, reports).into_iter().map(Target::UdpAdif).collect();
    out.extend(base.iter().filter(|t| !matches!(t, Target::UdpAdif(_))).cloned());
    out
}

/// Cosa serve all'emettitore UDP: tutto cio' che, se cambia, lo fa riavviare.
#[derive(Clone, Debug, PartialEq, Eq)]
struct UdpSettings {
    targets: Vec<SocketAddr>,
    call: String,
    grid: String,
    tx_power: String,
    accept_requests: bool,
    accept_reply: u8,
    send_adif: bool,
}

/// Con il motore proprio vale il primo server delle impostazioni (piu' gli indirizzi di `AURORA_UDP_FORWARD` oltre il primo); senza impostazioni
/// (non succede con il motore proprio) valgono i valori dell'ambiente.
fn udp_settings(config: &Config, store: Option<&remote_api::engine_settings::EngineSettingsStore>, call: &str, grid: &str) -> UdpSettings {
    let base = UdpSettings {
        targets: config.udp_forward.clone(),
        call: call.to_string(),
        grid: grid.to_string(),
        tx_power: config.tx_power.clone().unwrap_or_default(),
        accept_requests: config.udp_accept_requests,
        accept_reply: config.udp_accept_reply,
        send_adif: config.udp_adif,
    };
    match store {
        Some(store) => {
            let r = store.reports();
            UdpSettings { targets: remote_api::engine_settings::forward_targets(&config.udp_forward, &r), accept_requests: r.accept_requests, accept_reply: r.accept_reply, send_adif: r.adif_to_primary, ..base }
        }
        None => base,
    }
}

/// Avvia l'emettitore; senza destinatari (primo server vuoto) non parte nulla.
async fn start_udp_emitter(settings: &UdpSettings, bus: &Bus, controls: &Arc<engine_native::NativeControls>) -> Option<udp_emitter::Emitter> {
    if settings.targets.is_empty() {
        return None;
    }
    let mut emitter_config = udp_emitter::EmitterConfig::jtdx(settings.targets.clone());
    emitter_config.de_call = settings.call.clone();
    emitter_config.de_grid = settings.grid.clone();
    emitter_config.tx_power = settings.tx_power.clone();
    emitter_config.accept_requests = settings.accept_requests;
    emitter_config.accept_reply = settings.accept_reply;
    emitter_config.send_adif = settings.send_adif;
    let controls = Arc::clone(controls);
    match udp_emitter::Emitter::start(emitter_config, bus.subscribe_events(), bus.commands(), Arc::new(move || controls.tx_first())).await {
        Ok(emitter) => Some(emitter),
        Err(e) => {
            tracing::warn!("flusso UDP verso GridTracker non attivo: {e}");
            None
        }
    }
}

/// Il file ADIF di Aurora, nella cartella dei dati: stesso formato di `wsjtx_log.adi`.
const OWN_LOG_FILE: &str = "aurora_log.adi";

/// Da dove vengono i QSO gia' fatti: i file ADIF (`wsjtx_log.adi` di JTDX, `aurora_log.adi`), il log interno di Aurora (`logbook.db`) e, se
/// c'e', il log di Log4OM2 (sola lettura). Lo stesso QSO presente in piu' sorgenti conta una volta sola.
#[derive(Clone)]
struct LogSources {
    files: Vec<PathBuf>,
    logbook: Arc<Logbook>,
    log4om: Option<PathBuf>,
}

/// Carica i QSO gia' fatti da tutte le sorgenti; una sorgente mancante o illeggibile vale come vuota (se ne parla nel registro), cosi' al
/// peggio non si segnalano novita' invece di fermare il programma.
fn load_worked(sources: &LogSources, db: &DxccDb) -> WorkedLog {
    let mut text = String::new();
    for path in &sources.files {
        match std::fs::read(path) {
            Ok(bytes) => {
                text.push_str(&String::from_utf8_lossy(&bytes));
                text.push('\n');
            }
            // il file di Aurora non c'e' finche' non si registra il primo QSO: non e' un problema
            Err(e) if e.kind() == std::io::ErrorKind::NotFound && path.file_name().is_some_and(|n| n == OWN_LOG_FILE) => {}
            Err(e) => tracing::warn!("log dei QSO non letto ({}): {e}", path.display()),
        }
    }
    let mut log = WorkedLog::from_adif(&text, db);
    match sources.logbook.all_qsos() {
        Ok(rows) => {
            for q in &rows {
                log.add_adif_qso(q, db);
            }
        }
        Err(e) => tracing::warn!("log interno non letto: {e}"),
    }
    if let Some(path) = &sources.log4om {
        match logbook::log4om::read_all(path) {
            Ok(rows) => {
                for q in &rows {
                    log.add_adif_qso(q, db);
                }
            }
            Err(e) => tracing::warn!("log di Log4OM2 non letto ({}): {e}", path.display()),
        }
    }
    tracing::info!("log letto: {} QSO", log.len());
    log
}

/// Rilegge le sorgenti e sostituisce i QSO gia' fatti condivisi (etichette delle decodifiche, scheda DXCC e statistiche guardano gli stessi).
fn refresh_worked(sources: &LogSources, db: &DxccDb, shared: &SharedWorked) {
    let fresh = load_worked(sources, db);
    if let Ok(mut w) = shared.write() {
        *w = fresh;
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
        meters: None,
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
/// Un interruttore 0/1 dell'ambiente (`default` se manca o e' vuoto).
fn flag(get: &impl Fn(&str) -> Option<String>, name: &str, default: bool) -> Result<bool, StartError> {
    match get(name).as_deref().map(str::trim) {
        None | Some("") => Ok(default),
        Some("0" | "false") => Ok(false),
        Some("1" | "true") => Ok(true),
        Some(other) => Err(StartError::BadConfig(format!("{name} deve essere 0 o 1, non {other}"))),
    }
}

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

/// La catena di trasmissione del motore proprio: uscita audio, PTT e pianificazione. Senza uscita audio il motore riceve soltanto.
fn build_transmitter(config: &Config, controls: &Arc<engine_native::NativeControls>) -> Option<Arc<dyn engine_native::TxPort>> {
    let sink = match tx_native::CpalSink::open(config.audio_out_device.as_deref()) {
        Ok(sink) => sink,
        Err(e) => {
            tracing::warn!("uscita audio per la trasmissione non disponibile: {e}");
            return None;
        }
    };
    let port = config.ptt_port.clone().unwrap_or_default();
    let ptt: Arc<dyn tx_native::Ptt> = match config.ptt_method.as_str() {
        "rts" | "dtr" => {
            let line = if config.ptt_method == "rts" { tx_native::Line::Rts } else { tx_native::Line::Dtr };
            match tx_native::SerialLinePtt::open(&port, line) {
                Ok(ptt) => Arc::new(ptt),
                Err(e) => {
                    tracing::error!("PTT sulla seriale non attivo ({e}): niente trasmissione, per non rischiare una radio senza comando");
                    return None;
                }
            }
        }
        "omnirig" => match tx_native::OmniRigPtt::new(1) {
            Ok(ptt) => Arc::new(ptt),
            Err(e) => {
                tracing::error!("PTT con OmniRig non attivo ({e}): niente trasmissione");
                return None;
            }
        },
        _ => Arc::new(tx_native::NoPtt::new()),
    };
    let tx_config = tx_native::TxConfig { allow_tx: true, attenuation_tenths: Some(controls.attenuation.load(std::sync::atomic::Ordering::Relaxed)), ..tx_native::TxConfig::default() };
    match tx_native::Transmitter::new(Arc::new(tx_native::SystemClock), ptt, Box::new(sink), tx_config) {
        Ok(transmitter) => {
            let flag = Arc::clone(controls);
            transmitter.set_listener(Some(Arc::new(move |state| {
                flag.transmitting.store(state == tx_native::TxState::Transmitting, std::sync::atomic::Ordering::Relaxed);
            })));
            tracing::info!("trasmissione pronta: PTT {}{}", config.ptt_method, config.ptt_port.as_deref().map(|p| format!(" su {p}")).unwrap_or_default());
            Some(Arc::new(transmitter))
        }
        Err(e) => {
            tracing::error!("trasmissione non attiva: {e}");
            None
        }
    }
}

/// Avvia il server e ritorna subito; si ferma quando `shutdown` si completa.
pub async fn run(
    config: Config,
    shutdown: impl Future<Output = ()> + Send + 'static,
) -> Result<RunningServer, StartError> {
    let pair_ttl = load_pair_ttl(&config.data_dir.join("pairing.json"), config.pair_ttl_secs);
    let (auth, logbook) = open_storage(&config.data_dir, pair_ttl)?;
    // il CAT: configurazione e profili in radio.json; la scelta salvata con «collega all'avvio» vale come AURORA_RIG
    let cat = rig_control::cat::CatService::open(&config.data_dir.join("radio.json"));
    let mut config = config;
    let stored = cat.config();
    if !config.rig_omnirig && !config.rig_hamlib && stored.auto_connect {
        match stored.backend {
            rig_control::cat_config::Backend::Omnirig => config.rig_omnirig = true,
            rig_control::cat_config::Backend::Hamlib => config.rig_hamlib = true,
        }
    }
    if config.rig_hamlib {
        cat.adopt_backend(rig_control::cat_config::Backend::Hamlib);
    } else if config.rig_omnirig {
        cat.adopt_backend(rig_control::cat_config::Backend::Omnirig);
    }
    let bus = Bus::new(4096);
    // la trasmissione e' sempre abilitata (nessuna opzione): restano STOP TX, i tetti, il dead-man e le automazioni spente di base
    let native_controls = Arc::new(engine_native::NativeControls::default());
    let live_tx = match config.engine {
        EngineKind::Sim => true,
        EngineKind::Jtdx => config.allow_tx,
        EngineKind::Native => true,
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
    let (targets, worked_paths) = export_targets(&config);
    // motore proprio: le impostazioni sono di Aurora (`settings-motore.json`); il `JTDX.ini` serve solo come valori iniziali, una volta
    let engine_settings = (config.engine == EngineKind::Native).then(|| open_engine_settings(&config));
    // nominativo e locatore: con il motore proprio quelli delle impostazioni di Aurora, con JTDX quelli del suo file
    let (my_call, my_grid) = match &engine_settings {
        Some(store) => {
            let r = store.reports();
            (r.call, r.grid)
        }
        None => {
            let shown = config.jtdx_ini.as_deref().and_then(|p| engine_jtdx::ini::read_shown(p).ok()).unwrap_or_default();
            (shown.get("MyCall").cloned().unwrap_or_default(), shown.get("MyGrid").cloned().unwrap_or_default())
        }
    };
    // finiscono in ogni record ADIF (`STATION_CALLSIGN`, `MY_GRIDSQUARE`)
    let station = Station { call: my_call.clone(), grid: my_grid.clone(), tx_power: config.tx_power.clone().unwrap_or_default() };
    // il log di Aurora si riempie sempre; i log esterni solo se configurati. Con il motore proprio il server UDP secondario delle impostazioni
    // si cambia a programma acceso: l'esportatore rilegge la lista a ogni QSO.
    let shared_targets = Arc::new(std::sync::RwLock::new(match &engine_settings {
        Some(store) => live_targets(&targets, &config.export_udp, &store.reports()),
        None => targets.clone(),
    }));
    // il record dei QSO esportati dalla pagina porta nominativo e locatore correnti (con il motore proprio possono cambiare dalle impostazioni)
    let log_station: remote_api::LogStationProvider = {
        let (store, call, grid, power) = (engine_settings.clone(), my_call.clone(), my_grid.clone(), config.tx_power.clone().unwrap_or_default());
        Arc::new(move || match &store {
            Some(s) => {
                let r = s.reports();
                Station { call: r.call, grid: r.grid, tx_power: power.clone() }
            }
            None => Station { call: call.clone(), grid: grid.clone(), tx_power: power.clone() },
        })
    };
    let _exporter = Exporter::spawn_shared(&bus, Arc::clone(&logbook), Arc::clone(&shared_targets), station, Arc::new(unix_secs));
    if let Some(store) = &engine_settings {
        let (store, shared, base, env_udp) = (Arc::clone(store), Arc::clone(&shared_targets), targets.clone(), config.export_udp.clone());
        let mut changes = store.subscribe();
        tokio::spawn(async move {
            while changes.changed().await.is_ok() {
                let next = live_targets(&base, &env_udp, &store.reports());
                if let Ok(mut current) = shared.write() {
                    *current = next;
                }
            }
        });
    }
    // paesi e QSO gia' fatti si leggono una volta sola, prima di avviare il motore
    let log_sources = LogSources { files: worked_paths, logbook: Arc::clone(&logbook), log4om: config.log4om_db.clone() };
    let enrichment: Option<(Arc<DxccDb>, SharedWorked)> = load_dxcc(config.cty_path.as_deref()).map(|db| {
        let worked = Arc::new(std::sync::RwLock::new(load_worked(&log_sources, &db)));
        (db, worked)
    });
    // il continente di chi opera (dal nominativo di JTDX): serve a rispettare i CQ direzionali («CQ NA», «CQ DX»)
    if let Some((db, _)) = &enrichment
        && !my_call.is_empty()
        && let Some(hit) = db.lookup(&my_call)
    {
        automation.set_own_continent(Some(db.entity(hit.entity).continent.clone()));
    }
    let live_log = enrichment.as_ref().map(|(db, worked)| (Arc::clone(db), Arc::clone(worked), log_sources.clone()));
    let dxcc_loaded = enrichment.as_ref().map_or(0, |(db, _)| db.len());
    let worked_qsos = enrichment.as_ref().map_or(0, |(_, w)| w.read().map_or(0, |w| w.len()));
    let info = serde_json::json!({
        "version": aurora_core::version::APP_VERSION,
        "engine": match config.engine { EngineKind::Jtdx => "jtdx", EngineKind::Native => "native", EngineKind::Sim => "sim" },
        "tx_allowed": live_tx,
        "udp_listen": config.udp_listen.to_string(),
        "udp_forward": config.udp_forward.iter().map(ToString::to_string).collect::<Vec<_>>(),
        "export_udp": config.export_udp.iter().map(ToString::to_string).collect::<Vec<_>>(),
        "rig_control": config.rig_omnirig || config.rig_hamlib,
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
    if !my_call.is_empty() {
        cluster.default_callsign(&my_call);
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
        .with_stats_if(live_log.clone().map(|(db, worked, sources)| {
            Arc::new(move || {
                refresh_worked(&sources, &db, &worked);
                match worked.read() {
                    Ok(w) => serde_json::json!({ "stats": w.stats(&db), "missing_dxcc": w.missing_dxcc(&db) }),
                    Err(_) => serde_json::json!({ "stats": serde_json::Value::Null, "missing_dxcc": [] }),
                }
            }) as Arc<dyn Fn() -> serde_json::Value + Send + Sync>
        }))
        .with_dxcc_matrix_if(live_log.clone().map(|(db, worked, sources)| {
            // la scheda rilegge le sorgenti a ogni richiesta (un QSO fatto in Log4OM2 compare anche senza riavviare Aurora)
            Arc::new(move |q: remote_api::MatrixQuery| {
                refresh_worked(&sources, &db, &worked);
                let w = worked.read().map_err(|_| "QSO gia' fatti non disponibili".to_string())?;
                let mut out = serde_json::json!(w.matrix(&db));
                out["subject"] = match &q.call {
                    Some(call) => serde_json::json!(w.subject(call, q.grid.as_deref(), q.band.as_deref(), q.mode.as_deref(), &db)),
                    None => serde_json::Value::Null,
                };
                if q.compact
                    && let Some(map) = out.as_object_mut()
                {
                    map.remove("countries");
                }
                Ok(out)
            }) as remote_api::DxccMatrixProvider
        }))
        .with_log_station(log_station)
        .with_log_changed(match live_log.clone() {
            Some((db, worked, sources)) => Arc::new(move || refresh_worked(&sources, &db, &worked)),
            None => Arc::new(|| {}),
        })
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
                // la radio e' il servizio CAT (Hamlib o OmniRig, scelto e collegato dalla scheda Radio, anche dopo l'avvio)
                let radio = Some(Arc::clone(&cat) as Arc<dyn rig_control::Rig>);
                Some(Arc::new(engine_native::NativeRig::new(Arc::clone(&native_controls), radio)) as Arc<dyn rig_control::Rig>)
            }
            _ => config
                .rig_omnirig
                .then(|| {
                    // le sessioni PowerShell verso JTDX si aprono subito: il primo comando (Abilita Tx...) non deve aspettarle
                    rig_control::WindowsRig::keep_warm();
                    Arc::new(rig_control::WindowsRig) as Arc<dyn rig_control::Rig>
                })
                .or_else(|| config.rig_hamlib.then(|| Arc::clone(&cat) as Arc<dyn rig_control::Rig>)),
        })
        .with_cat(Arc::clone(&cat))
        .with_automation(automation);
    let state = match &engine_settings {
        Some(store) => state.with_engine_settings(Arc::clone(store)),
        None => state,
    };

    // collegamento della radio all'avvio (AURORA_RIG o «collega all'avvio»): in un thread, per non fermare l'avvio se la radio tarda
    {
        let forced = config.rig_hamlib || (config.rig_omnirig && config.engine == EngineKind::Native);
        let cat = Arc::clone(&cat);
        std::thread::spawn(move || {
            cat.start_up(forced);
        });
    }

    let commands = bus.take_command_rx().ok_or_else(|| StartError::Io("canale dei comandi gia' preso".into()))?;
    let io = EngineIo { events: bus.events(), spectrum: bus.spectrum(), commands };
    let dial_hz = Arc::new(std::sync::atomic::AtomicU64::new(0));
    let mut native_live: Option<engine_native::LiveConfig> = None;
    let (engine_task, hub) = match config.engine {
        EngineKind::Native => {
            // il motore proprio decodifica dall'audio della cascata (avviato piu' sotto); qui si prepara cio' che pubblica
            let (call, grid) = (my_call.clone(), my_grid.clone());
            let own_continent = enrichment.as_ref().and_then(|(db, _)| db.lookup(&call).map(|hit| db.entity(hit.entity).continent.clone()));
            let enrich = enrichment.map(|(db, worked)| Arc::new(Mutex::new(engine_jtdx::enrich::Enrichment::with_shared(db, worked))));
            // un QSO appena registrato non e' piu' una novita' (con JTDX lo fa il suo motore: `to_qso` + `record_qso`)
            if let Some(enrich) = &enrich {
                let enrich = Arc::clone(enrich);
                let mut events = bus.subscribe_events();
                tokio::spawn(async move {
                    loop {
                        match events.recv().await {
                            Ok(Event::QsoLogged(q)) => {
                                if let Ok(e) = enrich.lock() {
                                    e.record_qso(&q.call, &q.grid, &q.band, &q.mode);
                                }
                            }
                            Ok(_) | Err(broadcast::error::RecvError::Lagged(_)) => {}
                            Err(broadcast::error::RecvError::Closed) => break,
                        }
                    }
                });
            }
            native_live = Some(engine_native::LiveConfig {
                de_call: call,
                de_grid: grid,
                own_continent,
                dial_hz: Arc::clone(&dial_hz),
                controls: Arc::clone(&native_controls),
                transmitter: build_transmitter(&config, &native_controls),
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
            // i comandi della pagina e dell'automazione (doppio clic, CQ, STOP, frequenza Tx) arrivano al motore proprio
            let mut commands = io.commands;
            let controls = Arc::clone(&native_controls);
            let task = tokio::spawn(async move {
                while let Some(command) = commands.recv().await {
                    let mapped = match command {
                        aurora_core::Command::HaltTx => engine_native::TxCommand::Halt,
                        aurora_core::Command::Reply { decode_id } => engine_native::TxCommand::Reply { decode_id },
                        aurora_core::Command::SendFreeText { text, send } => engine_native::TxCommand::FreeText { text, send },
                        aurora_core::Command::CallCq { modifier } => engine_native::TxCommand::CallCq { modifier: Some(modifier).filter(|m| !m.trim().is_empty()) },
                        aurora_core::Command::SetTxFrequency { hz } => engine_native::TxCommand::SetTxFreq(hz),
                    };
                    if let Err(e) = controls.send(mapped) {
                        tracing::warn!("comando di trasmissione non consegnato: {e}");
                    }
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
                engine = engine.with_shared_enrichment(db, worked);
            }
            (tokio::spawn(engine.run(io)), Some(hub))
        }
    };

    // con il motore proprio non c'e' JTDX a parlare il protocollo UDP di WSJT-X con GridTracker (e, tramite lui, Log4OM2): lo fa Aurora.
    // Destinatari, invio dell'ADIF e comandi accettati vengono dalle impostazioni di Aurora (il primo server UDP); a ogni modifica si riavvia
    // solo l'emettitore (qualche istante, senza toccare il resto).
    let udp_emitter: Arc<tokio::sync::Mutex<Option<udp_emitter::Emitter>>> = Arc::new(tokio::sync::Mutex::new(None));
    if config.engine == EngineKind::Native && config.udp_emit {
        let wanted = udp_settings(&config, engine_settings.as_deref(), &my_call, &my_grid);
        if let Some(store) = &engine_settings {
            store.set_feature(remote_api::engine_settings::Feature::UdpEmitter, true);
        }
        *udp_emitter.lock().await = start_udp_emitter(&wanted, &bus, &native_controls).await;
        if let Some(store) = engine_settings.clone() {
            let (slot, config, bus, controls, call, grid) = (Arc::clone(&udp_emitter), config.clone(), bus.clone(), Arc::clone(&native_controls), my_call.clone(), my_grid.clone());
            let mut changes = store.subscribe();
            let mut running = wanted;
            tokio::spawn(async move {
                while changes.changed().await.is_ok() {
                    let next = udp_settings(&config, Some(&store), &call, &grid);
                    if next == running {
                        continue;
                    }
                    let mut slot = slot.lock().await;
                    if let Some(old) = slot.take() {
                        old.shutdown().await;
                    }
                    *slot = start_udp_emitter(&next, &bus, &controls).await;
                    running = next;
                }
            });
        }
    }

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
        // gli strumenti della radio (S-meter in ricezione; potenza, ROS, ALC in trasmissione) letti dal CAT Hamlib: il CAT limita da solo il ritmo
        // delle letture (250 ms in ricezione, 200 ms in trasmissione, un comando per giro sulle seriali lente); con OmniRig o senza radio restano spenti
        let cat = Arc::clone(&cat);
        let controls = Arc::clone(&native_controls);
        std::thread::spawn(move || {
            loop {
                match rig_control::Rig::meters(cat.as_ref()) {
                    Ok(meters) => {
                        controls.set_meters(Some(meters));
                        std::thread::sleep(Duration::from_millis(100));
                    }
                    Err(_) => {
                        controls.set_meters(None);
                        std::thread::sleep(Duration::from_secs(1));
                    }
                }
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
    let cat_for_task = Arc::clone(&cat);
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
        if let Some(emitter) = udp_emitter.lock().await.take() {
            emitter.shutdown().await;
        }
        drop(audio);
        // alla chiusura il PTT si spegne e la radio si libera
        let _ = tokio::task::spawn_blocking(move || cat_for_task.shutdown()).await;
    });
    Ok(RunningServer { addr, state, hub, task })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config_for(engine: EngineKind) -> Config {
        let mut c = Config::defaults();
        c.engine = engine;
        c.data_dir = PathBuf::from("dati");
        c.worked_adif = Some(PathBuf::from("jtdx/wsjtx_log.adi"));
        c.export_udp = vec!["127.0.0.1:2237".parse().expect("indirizzo")];
        c
    }

    #[test]
    fn with_the_native_engine_the_qso_goes_once_to_the_udp_target_and_to_the_own_adif_file() {
        let (targets, worked) = export_targets(&config_for(EngineKind::Native));
        assert_eq!(targets, vec![Target::UdpAdif("127.0.0.1:2237".parse().expect("indirizzo")), Target::File(PathBuf::from("dati").join("aurora_log.adi"))]);
        assert_eq!(worked, vec![PathBuf::from("jtdx/wsjtx_log.adi"), PathBuf::from("dati").join("aurora_log.adi")], "i lavorati leggono anche il file di Aurora");
    }

    #[test]
    fn the_secondary_server_of_the_settings_joins_the_fixed_targets_without_duplicates() {
        use remote_api::engine_settings::Reports;
        let (targets, _) = export_targets(&config_for(EngineKind::Native));
        let env: Vec<SocketAddr> = vec!["127.0.0.1:2237".parse().expect("indirizzo")];
        let own_file = Target::File(PathBuf::from("dati").join("aurora_log.adi"));
        let off = Reports::default();
        assert_eq!(live_targets(&targets, &env, &off), vec![Target::UdpAdif(env[0]), own_file.clone()], "secondo server spento: resta l'ambiente");
        let same = Reports { secondary: Some(env[0]), ..Reports::default() };
        assert_eq!(live_targets(&targets, &env, &same).len(), 2, "uguale all'ambiente: un solo invio");
        let other = Reports { secondary: Some("192.168.1.9:2240".parse().expect("indirizzo")), ..Reports::default() };
        let got = live_targets(&targets, &env, &other);
        assert_eq!(got.len(), 3);
        assert!(got.contains(&own_file), "il file di Aurora c'e' sempre");
        assert_eq!(live_targets(&targets, &[], &off), vec![own_file], "senza ambiente e senza secondo server: solo il file");
    }

    #[test]
    fn the_udp_emitter_follows_the_primary_server_of_the_settings() {
        use remote_api::engine_settings::{EngineSettingsStore, Feature};
        let dir = std::env::temp_dir().join(format!("aurora-udpset-{}", std::process::id()));
        std::fs::remove_dir_all(&dir).ok();
        let store = EngineSettingsStore::open(&dir.join("settings-motore.json"), remote_api::engine_settings::defaults);
        store.set_feature(Feature::UdpEmitter, true);
        let mut c = config_for(EngineKind::Native);
        c.udp_forward = vec!["127.0.0.1:2333".parse().expect("indirizzo"), "127.0.0.1:2400".parse().expect("indirizzo")];
        let first = udp_settings(&c, Some(&store), "IU3ABC", "IO91wm");
        assert_eq!(first.targets, vec!["127.0.0.1:2237".parse().expect("indirizzo"), "127.0.0.1:2400".parse().expect("indirizzo")], "primo server delle impostazioni + gli altri dell'ambiente");
        assert!(first.send_adif && !first.accept_requests, "ADIF al primo server acceso, comandi UDP spenti di fabbrica");
        let changes: std::collections::BTreeMap<String, String> =
            [("UDPServerPort", "2333"), ("EnableUDP1adifSending", "false"), ("AcceptUDPRequests", "true")].iter().map(|(k, v)| ((*k).to_string(), (*v).to_string())).collect();
        store.update(&changes).expect("valido");
        let next = udp_settings(&c, Some(&store), "IU3ABC", "IO91wm");
        assert_ne!(first, next, "una modifica fa riavviare l'emettitore");
        assert_eq!(next.targets[0], "127.0.0.1:2333".parse().expect("indirizzo"));
        assert!(!next.send_adif && next.accept_requests);
        assert_eq!(udp_settings(&c, Some(&store), "IU3ABC", "IO91wm"), next, "senza modifiche non cambia: niente riavvii inutili");
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn with_jtdx_aurora_writes_no_file_of_its_own_and_an_explicit_file_is_not_duplicated() {
        let (targets, worked) = export_targets(&config_for(EngineKind::Jtdx));
        assert_eq!(targets.len(), 1, "il file di JTDX non si tocca e non se ne crea un altro");
        assert_eq!(worked, vec![PathBuf::from("jtdx/wsjtx_log.adi")]);
        let mut c = config_for(EngineKind::Native);
        c.export_file = Some(PathBuf::from("dati").join("aurora_log.adi"));
        assert_eq!(export_targets(&c).0.len(), 2, "lo stesso file non va due volte");
    }

    #[test]
    fn the_udp_flow_options_come_from_the_environment_with_jtdx_defaults() {
        let none = Config::from_lookup(|_| None).expect("configurazione di fabbrica");
        // acceso di base (serve solo con il motore proprio e un destinatario), comandi e ADIF come in JTDX: spenti
        assert_eq!((none.udp_emit, none.udp_accept_requests, none.udp_accept_reply, none.udp_adif), (true, false, 1, false));
        let env = |key: &str| match key {
            "AURORA_UDP_EMIT" => Some("0".to_string()),
            "AURORA_UDP_ACCEPT" => Some("1".to_string()),
            "AURORA_UDP_ACCEPT_REPLY" => Some("3".to_string()),
            "AURORA_UDP_ADIF" => Some("1".to_string()),
            _ => None,
        };
        let set = Config::from_lookup(env).expect("configurazione");
        assert_eq!((set.udp_emit, set.udp_accept_requests, set.udp_accept_reply, set.udp_adif), (false, true, 3, true));
        assert!(Config::from_lookup(|k| (k == "AURORA_UDP_ACCEPT_REPLY").then(|| "4".to_string())).is_err());
        assert!(Config::from_lookup(|k| (k == "AURORA_UDP_ADIF").then(|| "si".to_string())).is_err());
    }

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
