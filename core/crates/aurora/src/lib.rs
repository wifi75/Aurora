//! Collega motore, log, autenticazione e API in un unico processo.

use std::{
    future::Future,
    net::SocketAddr,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

/// Il core ha finito di chiudersi (PTT spento, radio spenta se scelto, porte liberate): il programma aspetta questo prima di uscire.
pub static CORE_STOPPED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

pub mod audio_control;
pub mod backup;
pub mod band_hop;
pub mod history;
pub mod online;
pub mod rapporti;
pub mod support;
pub mod updates;
pub mod wanted;

use audio_spectrum::run_capture_tapped;
use auth::{DeviceStore, SystemClock};
use aurora_core::{Bus, Engine, EngineIo, Event};
use axum::{http::StatusCode, routing::any};
use dxcc::DxccDb;
use engine_jtdx::{enrich::SharedWorked, worked::WorkedLog};
use engine_sim::{SimConfig, SimEngine};
use logbook::{adif::Station, Logbook};
use qso_export::{Exporter, Target};
use rig_control::{schedule::Schedule, table::FrequencyTable, Rig};
use qso_engine::{Automation, AutomationConfig, Limits, Rules};
use remote_api::{build_router, AppState, TxPolicy};
use tokio::{net::TcpListener, sync::broadcast, task::JoinHandle};
use tower_http::services::{ServeDir, ServeFile};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EngineKind {
    /// Stazione simulata, senza radio.
    Sim,
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

/// La finestra «Salva con nome» del sistema (il programma Windows la da' alla partenza): la pagina di esportazione salva il log ADIF dove si vuole.
#[derive(Clone)]
pub struct SaveDialogHandle(pub remote_api::SaveFileDialog);

impl std::fmt::Debug for SaveDialogHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "SaveDialogHandle")
    }
}

/// Apre la cartella dei crash (dal programma Windows).
#[derive(Clone)]
pub struct CrashFolderHandle(pub remote_api::OpenCrashFolder);

impl std::fmt::Debug for CrashFolderHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "CrashFolderHandle")
    }
}

#[derive(Debug, Clone)]
pub struct Config {
    /// Se presente, la pagina propone «Installa ora» per le versioni nuove e l'API sa installarle.
    pub update_installer: Option<UpdateInstallerHandle>,
    /// Se presente, «Salva ADIF…» apre la finestra «Salva con nome» del programma (solo dal PC stesso).
    pub save_dialog: Option<SaveDialogHandle>,
    /// Se presente, Impostazioni > Aggiornamenti mostra «Apri la cartella dei crash» (solo dal PC stesso).
    pub crash_folder: Option<CrashFolderHandle>,
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
            save_dialog: None,
            crash_folder: None,
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
            Some("jtdx") => return Err(StartError::BadConfig("AURORA_ENGINE=jtdx non esiste piu': Aurora ha il suo motore (AURORA_ENGINE=native)".into())),
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
            save_dialog: None,
            crash_folder: None,
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
    task: JoinHandle<()>,
}

impl RunningServer {
    /// Attende la fine del server (dopo il segnale di spegnimento).
    pub async fn join(self) {
        let _ = self.task.await;
    }
}

/// La scelta dell'accesso senza codice fatta dal PC (Impostazioni > Smartphone/Tablet): acceso di base (scelta del proprietario, 2026-11-08:
/// sulla rete di casa niente codice); spento solo se il file dice «0». Da internet il codice resta sempre (`is_home_network`).
const OPEN_ACCESS_FILE: &str = "accesso-libero.txt";

fn read_open_access(path: &Path) -> bool {
    std::fs::read_to_string(path).map_or(true, |s| s.trim() != "0")
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
///
/// I destinatari del riquadro «Integrazioni» (`UdpDestinations`) partono, una volta sola, dagli indirizzi dell'ambiente: `AURORA_UDP_FORWARD` oltre il
/// primo (flusso WSJT-X, con il Logged ADIF se il server primario lo riceve) e `AURORA_EXPORT_UDP` (record ADIF). Poi vale il file.
fn open_engine_settings(config: &Config) -> Arc<remote_api::engine_settings::EngineSettingsStore> {
    use remote_api::engine_settings::{import_from_ini, interpret, seed_destinations, EngineSettingsStore, FILE_NAME, UDP_DESTINATIONS};
    let (listen, forward, export) = (config.udp_listen, config.udp_forward.clone(), config.export_udp.clone());
    Arc::new(EngineSettingsStore::open(&config.data_dir.join(FILE_NAME), move || {
        let mut seed = import_from_ini(&Default::default(), listen, &forward);
        let r = interpret(&seed);
        // il secondo server delle impostazioni riceve gia' l'ADIF: lo stesso indirizzo dell'ambiente non diventa un altro destinatario
        let export: Vec<SocketAddr> = export.into_iter().filter(|a| r.secondary != Some(*a)).collect();
        seed.insert(UDP_DESTINATIONS.to_string(), seed_destinations(&forward, &export, r.adif_to_primary));
        seed
    }))
}

/// I destinatari del QSO adesso: il secondo server delle impostazioni (regola anti-doppione di `engine_settings::export_udp_targets`), i
/// destinatari del riquadro «Integrazioni» che vogliono l'ADIF su UDP o l'XML di N1MM, poi i file e le altre uscite fisse di `base`. Gli
/// indirizzi di `AURORA_EXPORT_UDP` sono gia' nel riquadro (valori iniziali): non si aggiungono di nuovo.
fn live_targets(base: &[Target], reports: &remote_api::engine_settings::Reports) -> Vec<Target> {
    let mut out: Vec<Target> = remote_api::engine_settings::export_udp_targets(&[], reports).into_iter().map(Target::UdpAdif).collect();
    for d in &reports.destinations {
        let Some(addr) = d.addr() else { continue };
        if d.adif && !out.contains(&Target::UdpAdif(addr)) {
            out.push(Target::UdpAdif(addr));
        }
        if d.n1mm && !out.contains(&Target::N1mmXml(addr)) {
            out.push(Target::N1mmXml(addr));
        }
    }
    out.extend(base.iter().filter(|t| !matches!(t, Target::UdpAdif(_) | Target::N1mmXml(_))).cloned());
    out
}

/// Cosa serve all'emettitore UDP: tutto cio' che, se cambia, lo fa riavviare.
#[derive(Clone, Debug, PartialEq, Eq)]
struct UdpSettings {
    targets: Vec<SocketAddr>,
    /// I destinatari del riquadro «Integrazioni» che vogliono il flusso binario o il Logged ADIF.
    destinations: Vec<udp_emitter::Destination>,
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
        destinations: Vec::new(),
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
            // gli indirizzi di `AURORA_UDP_FORWARD` oltre il primo sono gia' nel riquadro «Integrazioni» (valori iniziali): non si aggiungono di nuovo
            UdpSettings {
                targets: remote_api::engine_settings::forward_targets(&[], &r),
                destinations: emitter_destinations(&r.destinations),
                accept_requests: r.accept_requests,
                accept_reply: r.accept_reply,
                send_adif: r.adif_to_primary,
                ..base
            }
        }
        None => base,
    }
}

/// I destinatari dell'emettitore tra quelli del riquadro «Integrazioni»: chi vuole il flusso binario o il Logged ADIF, con le opzioni multicast.
fn emitter_destinations(list: &[remote_api::engine_settings::UdpDestination]) -> Vec<udp_emitter::Destination> {
    list.iter()
        .filter(|d| d.wsjtx || d.logged_adif)
        .filter_map(|d| {
            let addr = d.addr()?;
            Some(udp_emitter::Destination {
                addr,
                stream: d.wsjtx,
                logged_adif: d.logged_adif,
                multicast: d.multicast.then(|| udp_emitter::Multicast { ttl: d.ttl, interface: d.interface() }),
            })
        })
        .collect()
}

/// Colore RGB come testo `#rrggbb` per la pagina.
fn hex_color(rgb: Option<[u8; 3]>) -> Option<String> {
    rgb.map(|[r, g, b]| format!("#{r:02x}{g:02x}{b:02x}"))
}

/// Cosa fa Aurora dei comandi in ingresso che non sono di trasmissione (arrivano solo con «accetta le richieste UDP» acceso).
fn external_hook(bus: &Bus, controls: &Arc<engine_native::NativeControls>, state: &AppState) -> Arc<dyn Fn(udp_emitter::External) + Send + Sync> {
    use udp_emitter::External;
    let (events, controls, state) = (bus.events(), Arc::clone(controls), state.clone());
    Arc::new(move |external| match external {
        External::Highlight { call, background, foreground, last_only } => {
            let _ = events.send(Event::HighlightCallsign { call, background: hex_color(background), foreground: hex_color(foreground), last_only });
        }
        External::Clear { window } => {
            // come «Cancella»: la pagina svuota le liste e il motore dimentica i messaggi ricordati per il doppio clic
            let _ = events.send(Event::ClearDecodes { window });
            let _ = controls.send(engine_native::TxCommand::Erase);
        }
        External::Location { grid } => {
            tracing::info!("Location via UDP: locatore della sessione {grid}");
            controls.set_session_grid(&grid);
            // senza trasmissione (nessuna uscita audio) la sequenza non c'e': vale comunque per lo Status
            let _ = controls.send(engine_native::TxCommand::SetMyGrid(grid));
        }
        External::Annotation { call, order } => {
            // l'ordinamento dei Hound della Fox dato da un programma esterno (registro della spedizione)
            let _ = controls.send(engine_native::TxCommand::Fox(aurora_core::FoxCommand::Annotate { call, order }));
        }
        External::SwitchConfiguration { name } => {
            tracing::info!("Switch Configuration via UDP ({name}) ignorato: Aurora non ha configurazioni con nome");
        }
        External::Configure(c) => {
            let (controls, state) = (Arc::clone(&controls), state.clone());
            tokio::spawn(async move { apply_configure(c, &controls, &state).await });
        }
    })
}

/// Configure (tipo 15) come `MainWindow::remote_configure` di WSJT-X, per cio' che Aurora ha: modo (FT8 o FT4), frequenza Rx, DX call e
/// locatore, «genera i messaggi». Tolleranza, sottomodo, modo veloce e periodo non esistono in FT8/FT4 di Aurora: si registrano e si ignorano.
async fn apply_configure(c: udp_emitter::Configure, controls: &Arc<engine_native::NativeControls>, state: &AppState) {
    if let Some(mode) = c.mode.as_deref().map(str::to_uppercase) {
        let current = controls.mode().name().to_uppercase();
        if !matches!(mode.as_str(), "FT8" | "FT4") {
            tracing::info!("Configure via UDP: modo {mode} non gestito, ignorato");
        } else if mode != current {
            if state.current_status().is_some_and(|s| s.transmitting) {
                tracing::info!("Configure via UDP: modo {mode} non cambiato durante la trasmissione");
            } else if let Err(e) = remote_api::switch_mode(state, &mode).await {
                tracing::warn!("Configure via UDP: cambio di modo a {mode} non riuscito: {e}");
            }
        }
    }
    if c.frequency_tolerance.is_some() || c.submode.is_some() || c.fast_mode {
        tracing::debug!("Configure via UDP: tolleranza, sottomodo e modo veloce non esistono in FT8/FT4, ignorati");
    }
    if let Some(period) = c.tr_period {
        let expected = match controls.mode().name() {
            "FT4" => 7,
            "FT8" => 15,
            _ => 60,
        };
        if period != expected {
            tracing::info!("Configure via UDP: periodo T/R {period} s non applicabile al modo in uso, ignorato");
        }
    }
    if let Some(hz) = c.rx_df {
        match state.active_rig() {
            Some(rig) => match tokio::task::spawn_blocking(move || rig.set_rx_frequency(hz)).await {
                Ok(Ok(())) => {}
                Ok(Err(e)) => tracing::info!("Configure via UDP: frequenza Rx {hz} Hz non impostata: {e}"),
                Err(_) => tracing::warn!("Configure via UDP: frequenza Rx non impostata"),
            },
            None => tracing::info!("Configure via UDP: nessun motore per la frequenza Rx"),
        }
    }
    let command = if c.dx_call.is_some() || c.dx_grid.is_some() {
        Some(engine_native::TxCommand::SetDx { call: c.dx_call, grid: c.dx_grid, generate: c.generate_messages })
    } else if c.generate_messages {
        Some(engine_native::TxCommand::GenMsgs)
    } else {
        None
    };
    if let Some(command) = command
        && let Err(e) = controls.send(command)
    {
        tracing::info!("Configure via UDP: DX e messaggi non impostati: {e}");
    }
}

/// Avvia l'emettitore; senza destinatari (primo server vuoto e nessun destinatario nel riquadro «Integrazioni») non parte nulla.
/// Il collegamento tra i TxInhibit dell'emettitore UDP e il blocco del PTT; tiene anche l'avviso «Tx bloccata» nello stato della stazione.
struct InhibitBridge {
    interlock: Arc<tx_native::TxInterlock>,
    controls: Arc<engine_native::NativeControls>,
}

impl InhibitBridge {
    fn publish(&self) {
        let s = self.interlock.snapshot();
        if s.inhibited {
            let alert = aurora_core::TxAlert { kind: aurora_core::TxAlertKind::Inhibited, detail: s.holder, value: None, limit: None };
            if !self.controls.tx_alerts().contains(&alert) {
                self.controls.raise_alert(alert, None);
            }
        } else {
            self.controls.clear_alert(aurora_core::TxAlertKind::Inhibited);
        }
    }
}

impl udp_emitter::InhibitControl for InhibitBridge {
    fn command(&self, controller: &str, ttl_ms: u32, station: &str) {
        self.interlock.command(controller, ttl_ms, station);
        self.publish();
    }

    fn invalid(&self, count: u32) {
        self.interlock.invalid(count);
    }

    fn tick(&self) {
        self.interlock.tick();
        self.publish();
    }

    fn snapshot(&self) -> udp_bridge::InhibitStatus {
        let s = self.interlock.snapshot();
        udp_bridge::InhibitStatus { inhibited: s.inhibited, source_station: s.holder, hold_rx: s.hold_rx, release_rx: s.release_rx, expiries: s.expiries, invalid: s.invalid, ..Default::default() }
    }
}

async fn start_udp_emitter(
    settings: &UdpSettings,
    bus: &Bus,
    controls: &Arc<engine_native::NativeControls>,
    state: &AppState,
    inhibit: &Arc<dyn udp_emitter::InhibitControl>,
) -> Option<udp_emitter::Emitter> {
    if settings.targets.is_empty() && settings.destinations.is_empty() {
        return None;
    }
    let mut emitter_config = udp_emitter::EmitterConfig::jtdx(settings.targets.clone());
    emitter_config.destinations = settings.destinations.clone();
    emitter_config.de_call = settings.call.clone();
    emitter_config.de_grid = settings.grid.clone();
    emitter_config.tx_power = settings.tx_power.clone();
    emitter_config.accept_requests = settings.accept_requests;
    emitter_config.accept_reply = settings.accept_reply;
    emitter_config.send_adif = settings.send_adif;
    let hooks = udp_emitter::EmitterHooks { on_external: Some(external_hook(bus, controls, state)), inhibit: Some(Arc::clone(inhibit)), ..Default::default() };
    let controls = Arc::clone(controls);
    match udp_emitter::Emitter::start_with(emitter_config, bus.subscribe_events(), bus.commands(), Arc::new(move || controls.tx_first()), hooks).await {
        Ok(emitter) => Some(emitter),
        Err(e) => {
            tracing::warn!("flusso UDP verso GridTracker non attivo: {e}");
            None
        }
    }
}

/// Il file ADIF di Aurora, nella cartella dei dati: stesso formato di `wsjtx_log.adi`.
const OWN_LOG_FILE: &str = "aurora_log.adi";
/// La frequenza scelta a mano quando non c'e' una radio comandabile (CAT), ricordata al riavvio.
const MANUAL_DIAL_FILE: &str = "frequenza-a-mano.txt";

/// Da dove vengono i QSO gia' fatti: i file ADIF (`wsjtx_log.adi` di JTDX, `aurora_log.adi`), il log interno di Aurora (`logbook.db`) e, se
/// c'e', il log di Log4OM2 (sola lettura). Lo stesso QSO presente in piu' sorgenti conta una volta sola.
#[derive(Clone)]
struct LogSources {
    files: Vec<PathBuf>,
    logbook: Arc<Logbook>,
    log4om: Option<PathBuf>,
    /// Il filtro del log della scheda Generali (nominativo, griglia, data di partenza): cambia a caldo con le impostazioni.
    filter: Arc<std::sync::RwLock<engine_jtdx::worked::LogFilter>>,
}

/// Le impostazioni della scheda Sequenza come le vuole la sequenza dei QSO del motore proprio.
fn sequence_settings(r: &remote_api::engine_settings::Reports) -> engine_native::SequenceSettings {
    let mut s = engine_native::SequenceSettings::new([r.seq_answer_cq, r.seq_answer_in_call, r.seq_sent_r_report, r.seq_closing], r.seq_halt_reply_other, r.strict_directional_cq, r.watchdog_min, r.tx_qsy_allowed);
    s.rr73_marker = r.rr73_marker;
    s.call_prio_cq = r.call_prio_cq;
    s.max_distance = r.max_distance;
    s.autolog = r.autolog;
    s.clear_dx = r.clear_dx;
    s
}

/// Cio' che i servizi di rete devono sapere adesso.
fn reporting_config(r: &remote_api::engine_settings::Reports, tx_power: &str) -> reporting::Config {
    reporting::Config {
        call: r.call.clone(),
        grid: r.grid.clone(),
        tx_power: tx_power.to_string(),
        antenna: String::new(),
        psk: r.psk,
        dxsummit: r.dxsummit,
        tcp: r.tcp,
        eqsl: r.eqsl.then(|| reporting::EqslAccount { user: r.eqsl_user.clone(), nick: r.eqsl_nick.clone() }),
        wavelog: r.wavelog.then(|| reporting::WavelogAccount { url: r.wavelog_url.clone(), station_id: r.wavelog_station.clone() }),
        clublog: r.clublog.then(|| reporting::ClublogAccount { email: r.clublog_email.clone(), callsign: r.clublog_call.clone() }),
        qrz: r.qrz,
        qrz_call: r.qrz_call.clone(),
        hrdlog: r.hrdlog.then(|| reporting::HrdlogAccount { callsign: r.hrdlog_call.clone() }),
        lotw: r.lotw.then(|| reporting::LotwSetup { tqsl: r.lotw_tqsl.clone(), location: r.lotw_location.clone() }),
    }
}

/// Collega alle impostazioni gli invii verso l'esterno (PSK Reporter, spot DXSummit a richiesta, server TCP, eQSL, invio diretto dei QSO con la sua
/// coda `invii-in-coda.json` nella cartella dei dati), la registrazione del log (commento con rapporti e distanza,
/// modo) e la scrittura di `ALL.TXT`. Ogni invio ha il suo thread e i suoi tempi massimi: non ferma mai il motore.
fn wire_reporting(store: &Arc<remote_api::engine_settings::EngineSettingsStore>, bus: &Bus, controls: &Arc<engine_native::NativeControls>, data_dir: &Path, tx_power: String, prompt: Arc<remote_api::log_prompt::LogPrompt>) {
    use remote_api::engine_settings::Feature;
    let secrets: Arc<dyn reporting::secret::SecretStore> = Arc::new(reporting::secret::ProtectedFiles::new(data_dir));
    store.set_secrets(Arc::clone(&secrets));
    store.set_feature(Feature::AllTxt, true);
    let r = store.reports();
    controls.set_all_txt(r.all_txt, r.all_txt_debug);
    let endpoints = reporting::Endpoints { queue_file: Some(data_dir.join("invii-in-coda.json")), ..reporting::Endpoints::default() };
    let service = reporting::Service::spawn(bus, store.status(), secrets, endpoints, Arc::new(unix_secs), reporting_config(&r, &tx_power));
    store.set_reporting(Arc::clone(&service));
    {
        let (store, controls) = (Arc::clone(store), Arc::clone(controls));
        let mut changes = store.subscribe();
        tokio::spawn(async move {
            while changes.changed().await.is_ok() {
                let r = store.reports();
                controls.set_all_txt(r.all_txt, r.all_txt_debug);
                service.set_config(reporting_config(&r, &tx_power));
            }
        });
    }
    // un QSO concluso dal motore diventa un QSO registrato, con modo e commento come la finestra di registrazione di JTDX
    let (store, events, mut completed) = (Arc::clone(store), bus.events(), bus.subscribe_events());
    tokio::spawn(async move {
        loop {
            match completed.recv().await {
                Ok(Event::QsoCompleted(mut q)) => {
                    let r = store.reports();
                    let distance = match qso_native::history::grid_distance_km(&r.grid, &q.grid) {
                        0 => String::new(),
                        km => format!("{km} km"),
                    };
                    q.comment = logbook::adif::log_comment(&q.mode, &q.rst_sent, &q.rst_rcvd, &distance, r.report_in_comments, r.distance_in_comments);
                    q.mode = logbook::adif::log_mode(&q.mode, r.to_rtty);
                    // nome e citta' dalla ricerca QRZ.com/HamQTH (se accesa): al piu' 2,5 s, poi si registra senza
                    if let Some((name, qth)) = store.name_and_qth(&q.call, unix_secs(), std::time::Duration::from_millis(2500)).await {
                        if q.name.is_empty() {
                            q.name = name;
                        }
                        if q.qth.is_empty() {
                            q.qth = qth;
                        }
                    }
                    // «Richiedimi di loggare il QSO»: il QSO aspetta la conferma della finestra; con la coda piena il piu' vecchio va a log
                    if r.prompt_to_log {
                        tracing::info!("QSO con {} in attesa di conferma", q.call);
                        if let Some(oldest) = prompt.push(q) {
                            let _ = events.send(Event::QsoLogged(oldest));
                        }
                    } else {
                        // la voce spenta con QSO ancora in attesa: vanno a log, non si perdono
                        for waiting in prompt.drain() {
                            let _ = events.send(Event::QsoLogged(waiting));
                        }
                        let _ = events.send(Event::QsoLogged(q));
                    }
                }
                Ok(_) | Err(broadcast::error::RecvError::Lagged(_)) => {}
                Err(broadcast::error::RecvError::Closed) => break,
            }
        }
    });
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
    let filter = sources.filter.read().map(|f| f.clone()).unwrap_or_default();
    let mut log = WorkedLog::new();
    for q in aurora_core::adif::parse_qsos(&text).iter().filter(|q| filter.accepts(q)) {
        log.add_adif_qso(q, db);
    }
    match sources.logbook.all_qsos() {
        Ok(rows) => {
            for q in rows.iter().filter(|q| filter.accepts(q)) {
                log.add_adif_qso(q, db);
            }
        }
        Err(e) => tracing::warn!("log interno non letto: {e}"),
    }
    if let Some(path) = &sources.log4om {
        match logbook::log4om::read_all(path) {
            Ok(rows) => {
                for q in rows.iter().filter(|q| filter.accepts(q)) {
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
/// Toglie i QSO cancellati dal log anche da `aurora_log.adi`, che serve per i «gia' lavorati». Log svuotato (`None`): il file si mette da
/// parte con la data (`aurora_log-AAAAMMGG-HHMMSS.adi`) invece di cancellarlo, cosi' nulla va perso.
fn forget_in_own_file(path: &Path, gone: Option<&[logbook::LogEntry]>) {
    if !path.exists() {
        return;
    }
    match gone {
        None => {
            let stamp = logbook::adif::utc_date(unix_secs()) + &format!("-{:06}", unix_secs() % 86_400 / 3600 * 10_000 + unix_secs() % 3600 / 60 * 100 + unix_secs() % 60);
            let aside = path.with_file_name(format!("aurora_log-{stamp}.adi"));
            match std::fs::rename(path, &aside) {
                Ok(()) => tracing::info!("log svuotato: {} messo da parte come {}", path.display(), aside.display()),
                Err(e) => tracing::warn!("log svuotato ma {} non spostato: {e}", path.display()),
            }
        }
        Some(gone) if !gone.is_empty() => match std::fs::read_to_string(path) {
            Ok(text) => {
                let (kept, removed) = logbook::adif_without(&text, gone);
                if removed > 0 {
                    if let Err(e) = std::fs::write(path, kept) {
                        tracing::warn!("{} non riscritto: {e}", path.display());
                    } else {
                        tracing::info!("{removed} QSO tolti anche da {}", path.display());
                    }
                }
            }
            Err(e) => tracing::warn!("{} non letto: {e}", path.display()),
        },
        Some(_) => {}
    }
}

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
        // una tabella salvata prima delle regioni (una riga per banda e modo, quelle di allora) passa alla tabella di JTDX; una modificata dall'utente resta com'e'
        let loaded = serde_json::from_str::<Vec<rig_control::table::Entry>>(&text).map_err(|e| e.to_string()).and_then(|entries| table.set(FrequencyTable::upgraded(entries)).map_err(String::from));
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
/// Le voci delle funzioni di WSJT-X 3.x che il motore legge dai pulsanti: momento di avvio della decodifica FT8, full duplex, Wait features, verifica del Fox.
fn apply_wsjtx3_settings(controls: &engine_native::NativeControls, r: &remote_api::engine_settings::Reports) {
    use std::sync::atomic::Ordering;
    controls.set_decoder_start(engine_native::features::DecoderStart::parse(&r.ft8_decoder_start));
    controls.full_duplex.store(r.full_duplex, Ordering::Relaxed);
    controls.wait_features.store(r.wait_features, Ordering::Relaxed);
    controls.hound_auto.store(r.hound_auto, Ordering::Relaxed);
    controls.pounce_policy.store(engine_native::features::PouncePolicy::parse(&r.pounce_policy).code(), Ordering::Relaxed);
    controls.set_otp_settings(engine_native::features::OtpSettings { show: r.show_otp, verify: r.otp_verify, url: r.otp_url.clone() });
}

/// La verifica in linea del codice OTP del Fox (Hound classico, `processSFoxVerification`/`handleVerifyMsg` di WSJT-X): la richiesta parte in un thread
/// a parte; l'esito va nello stato della stazione (pagina Stazione) e, come in WSJT-X, diventa una riga tra i decode («CALL verified» o «CALL invalid»).
fn wire_fox_verifier(controls: &Arc<engine_native::NativeControls>, bus: &Bus) {
    let (weak, events) = (Arc::downgrade(controls), bus.events());
    let ids = Arc::new(std::sync::atomic::AtomicU64::new(1_900_000_000));
    controls.set_otp_hook(Arc::new(move |otp: engine_native::features::FoxOtp| {
        let Some(controls) = weak.upgrade() else { return };
        let (events, ids) = (events.clone(), Arc::clone(&ids));
        let url = controls.otp_settings().url;
        std::thread::spawn(move || {
            let fetch = online_info::foxcheck::FoxFetch { user_agent: format!("Aurora/{}", aurora_core::version::APP_VERSION) };
            match online_info::foxcheck::check(&fetch, &url, &otp.call, otp.slot_start_ms, &otp.code) {
                Ok(verified) => {
                    tracing::info!("verifica del Fox {}: {}", otp.call, if verified { "verificato" } else { "non valido" });
                    controls.set_fox_verification(Some(aurora_core::FoxVerification { call: otp.call.clone(), verified }));
                    let _ = events.send(Event::Decode(aurora_core::DecodedMessage {
                        id: ids.fetch_add(1, std::sync::atomic::Ordering::Relaxed),
                        time_ms: (otp.slot_start_ms % 86_400_000) as u32,
                        snr: 0,
                        delta_time: 0.0,
                        freq_hz: otp.freq_hz,
                        text: format!("{} {}", otp.call, if verified { "verified" } else { "invalid" }),
                        ..Default::default()
                    }));
                }
                Err(e) => tracing::warn!("verifica del Fox {} non riuscita: {e}", otp.call),
            }
        });
    }));
}

/// All'avvio la radio va sulla frequenza principale della sua banda per il modo in uso (scelta del proprietario, 2026-11-08): se il dial e' su
/// un'alternativa (14,090 in FT8) passa alla principale (14,074). Si aspetta che la radio risponda (al piu' due minuti); una volta sola.
/// L'ultima banda scelta (si ricorda tra un avvio e l'altro).
const LAST_BAND_FILE: &str = "ultima-banda.txt";

/// All'avvio, appena la radio risponde (controllo ogni 0,3 s, non piu' ogni 2): la radio va sull'ultima banda scelta e li' sulla frequenza
/// principale del modo in uso. Poi si tiene a mente la banda in uso, cosi' al prossimo avvio si riparte da quella (richiesta del proprietario
/// 2026-11-08: la banda e' l'ultima scelta, la frequenza sempre la principale della banda).
fn spawn_start_on_main_frequency(state: AppState, data_dir: std::path::PathBuf) {
    let band_file = data_dir.join(LAST_BAND_FILE);
    tokio::spawn(async move {
        let saved = std::fs::read_to_string(&band_file).ok().map(|t| t.trim().to_string()).filter(|t| !t.is_empty());
        let deadline = std::time::Instant::now() + Duration::from_secs(180);
        while std::time::Instant::now() < deadline {
            tokio::time::sleep(Duration::from_millis(300)).await;
            let Some(rig) = state.dial_rig() else { continue };
            let reader = Arc::clone(&rig);
            let Ok(Ok(hz)) = tokio::task::spawn_blocking(move || reader.frequency()).await else { continue };
            if hz == 0 {
                continue;
            }
            if state.is_transmitting() {
                break;
            }
            let mode = state.current_mode();
            let region = state.frequency_region();
            let target = {
                let table = state.frequencies.lock().unwrap_or_else(|p| p.into_inner());
                // la banda: l'ultima scelta, se nella tabella c'e' per questo modo; altrimenti quella su cui si trova la radio
                let band = saved.clone().filter(|b| table.frequency_in(region, b, &mode).is_some()).or_else(|| table.band_of(hz));
                band.and_then(|b| table.frequency_in(region, &b, &mode))
            };
            if let Some(main) = target.filter(|m| *m != hz) {
                match tokio::task::spawn_blocking(move || rig.set_frequency(main)).await {
                    Ok(Ok(())) => tracing::info!("avvio: radio portata sulla frequenza principale {main} Hz dell'ultima banda (era {hz} Hz)"),
                    other => tracing::warn!("avvio: frequenza principale non impostata: {other:?}"),
                }
            }
            break;
        }
        // da qui si ricorda la banda in uso (ogni 5 s, si scrive solo quando cambia)
        let mut last = saved;
        loop {
            tokio::time::sleep(Duration::from_secs(5)).await;
            let Some(hz) = state.current_status().map(|s| s.dial_hz).filter(|hz| *hz > 0) else { continue };
            let band = state.frequencies.lock().unwrap_or_else(|p| p.into_inner()).band_of(hz);
            if let Some(band) = band
                && last.as_deref() != Some(band.as_str())
            {
                let _ = std::fs::write(&band_file, &band);
                last = Some(band);
            }
        }
    });
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
                table.frequency_in(state.frequency_region(), &band, &state.current_mode())
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

/// Il cursore Pwr come volume di Windows dell'uscita audio che va alla radio (scelta del proprietario): un'attenuazione del segnale
/// generato, come in JTDX, poteva solo abbassare, e con il volume di Windows basso il cursore al massimo non bastava. Il segnale digitale
/// resta pieno (0 dB) e il livello lo da' il volume della scheda.
struct WindowsVolumePort {
    inner: Arc<dyn engine_native::TxPort>,
    output: Arc<dyn Fn() -> Option<String> + Send + Sync>,
    /// Il livello del cursore (quello vero, anche dopo il ripristino dei livelli salvati): si riapplica prima di ogni trasmissione, perche' il
    /// volume di Windows puo' cambiare per conto suo (all'avvio il cursore diceva 100 % e la scheda era al 50 %).
    controls: Arc<engine_native::NativeControls>,
}

impl WindowsVolumePort {
    fn apply(&self, tenths: u32) {
        let volume = 1.0 - (tenths.min(450) as f32 / 450.0);
        let Some(name) = (self.output)() else { return };
        let out = match audio_spectrum::OutputEndpoint::find(&name) {
            Ok(out) => out,
            Err(e) => {
                tracing::warn!("Pwr: uscita non trovata ({e})");
                return;
            }
        };
        // si scrive solo se e' diverso (succede prima di ogni trasmissione): cosi' nel registro resta solo quando si corregge davvero
        if out.volume().is_ok_and(|v| (v - volume).abs() < 0.005) {
            return;
        }
        match out.set_volume(volume) {
            Ok(()) => tracing::info!("Pwr: volume di Windows di «{name}» al {:.0} %", volume * 100.0),
            Err(e) => tracing::warn!("Pwr: volume dell'uscita non impostato ({e})"),
        }
    }
}

impl engine_native::TxPort for WindowsVolumePort {
    fn schedule(&self, request: tx_native::TxRequest) -> Result<(), String> {
        self.apply(self.controls.attenuation.load(std::sync::atomic::Ordering::Relaxed));
        self.inner.schedule(request)
    }

    fn halt(&self) {
        self.inner.halt();
    }

    fn replace(&self, request: tx_native::TxRequest) -> Result<bool, String> {
        self.inner.replace(request)
    }

    fn set_attenuation(&self, tenths: u32) {
        self.inner.set_attenuation(0);
        self.apply(tenths);
    }
}

/// La catena di trasmissione del motore proprio: uscita audio, PTT e pianificazione. Senza uscita audio il motore riceve soltanto.
/// PTT comandato attraverso la radio collegata con Hamlib (`T 1` / `T 0` di rigctld): rigctld usa il PTT scelto in Impostazioni > Radio
/// (CAT, oppure DTR/RTS sulla porta indicata). Se la radio non e' collegata la trasmissione non parte.
struct CatPtt {
    cat: Arc<rig_control::cat::CatService>,
    on: std::sync::atomic::AtomicBool,
}

impl tx_native::Ptt for CatPtt {
    fn set(&self, on: bool) -> Result<(), tx_native::TxError> {
        cat_ptt(&self.cat, on)?;
        self.on.store(on, std::sync::atomic::Ordering::SeqCst);
        Ok(())
    }

    fn is_on(&self) -> bool {
        self.on.load(std::sync::atomic::Ordering::SeqCst)
    }
}

/// Il PTT dalla radio collegata: `trx` con TCI, `T 1` / `T 0` di rigctld con Hamlib.
fn cat_ptt(cat: &rig_control::cat::CatService, on: bool) -> Result<(), tx_native::TxError> {
    let failed = |e: rig_control::RigError| tx_native::TxError::Ptt(e.to_string());
    if cat.backend() == rig_control::cat_config::Backend::Tci {
        return cat.tci().map_err(failed)?.set_ptt(on).map_err(failed);
    }
    cat.hamlib().map_err(failed)?.set_ptt(on).map_err(failed)
}

/// PTT senza scelta da JTDX: a ogni trasmissione guarda Impostazioni > Radio, cosi' vale anche una radio scelta dopo l'avvio
/// (senza riavviare Aurora). Con Hamlib e una radio scelta il PTT passa da rigctld; con «VOX» o senza radio nessun comando.
struct AutoPtt {
    cat: Arc<rig_control::cat::CatService>,
    on: std::sync::atomic::AtomicBool,
}

impl tx_native::Ptt for AutoPtt {
    fn set(&self, on: bool) -> Result<(), tx_native::TxError> {
        let radio = self.cat.config();
        let by_rig = radio.ptt != rig_control::cat_config::PttMethod::Vox
            && match radio.backend {
                rig_control::cat_config::Backend::Hamlib => radio.model != 0,
                rig_control::cat_config::Backend::Tci => true,
                _ => false,
            };
        if by_rig {
            cat_ptt(&self.cat, on)?;
        } else if on {
            tracing::warn!("trasmissione con il VOX: nessuna radio Hamlib con PTT scelta in Impostazioni > Radio");
            aurora_core::notice::notice("no_radio", "");
        }
        self.on.store(on, std::sync::atomic::Ordering::SeqCst);
        Ok(())
    }

    fn is_on(&self) -> bool {
        self.on.load(std::sync::atomic::Ordering::SeqCst)
    }
}

/// Il PTT da usare: quello di `AURORA_PTT_METHOD` (o di JTDX) se c'e'; altrimenti quello scelto in Impostazioni > Radio,
/// cosi' chi non viene da JTDX trasmette con il PTT della radio invece di restare sul VOX senza comando.
fn ptt_method_for(config: &Config, cat: &rig_control::cat::CatService) -> String {
    let radio = cat.config();
    // OmniRig preso da JTDX ma la radio e' collegata con Hamlib: OmniRig non comanda la radio, il PTT passa da rigctld
    // (caso reale: un vecchio JTDX.ini con PTT OmniRig e Aurora su Hamlib, la radio non andava mai in trasmissione)
    if config.ptt_method == "omnirig"
        && (radio.backend == rig_control::cat_config::Backend::Tci || radio.backend == rig_control::cat_config::Backend::Hamlib && radio.model != 0)
        && radio.ptt != rig_control::cat_config::PttMethod::Vox
    {
        return "cat".to_string();
    }
    if config.ptt_method != "vox" {
        return config.ptt_method.clone();
    }
    match (radio.backend, radio.ptt) {
        (_, rig_control::cat_config::PttMethod::Vox) => "vox".to_string(),
        (rig_control::cat_config::Backend::Omnirig, _) => "omnirig".to_string(),
        (rig_control::cat_config::Backend::Hamlib, _) if radio.model != 0 => "cat".to_string(),
        (rig_control::cat_config::Backend::Tci, _) => "cat".to_string(),
        _ => "vox".to_string(),
    }
}

fn build_transmitter(config: &Config, cat: &Arc<rig_control::cat::CatService>, controls: &Arc<engine_native::NativeControls>, interlock: &Arc<tx_native::TxInterlock>, sink: Box<dyn tx_native::AudioSink>) -> Option<Arc<dyn engine_native::TxPort>> {
    let port = config.ptt_port.clone().unwrap_or_default();
    let method = ptt_method_for(config, cat);
    let ptt: Arc<dyn tx_native::Ptt> = match method.as_str() {
        "cat" => Arc::new(CatPtt { cat: Arc::clone(cat), on: std::sync::atomic::AtomicBool::new(false) }),
        // nessun PTT da JTDX e nessuna radio ancora scelta all'avvio: si decide a ogni trasmissione
        "vox" if config.ptt_method == "vox" => Arc::new(AutoPtt { cat: Arc::clone(cat), on: std::sync::atomic::AtomicBool::new(false) }),
        "rts" | "dtr" => {
            let line = if method == "rts" { tx_native::Line::Rts } else { tx_native::Line::Dtr };
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
    // il PTT passa dal blocco esterno (TxInhibit via UDP): con una prenotazione attiva resta spento
    interlock.attach(ptt);
    let ptt: Arc<dyn tx_native::Ptt> = Arc::clone(interlock) as Arc<dyn tx_native::Ptt>;
    let tx_config = tx_native::TxConfig { allow_tx: true, attenuation_tenths: Some(controls.attenuation.load(std::sync::atomic::Ordering::Relaxed)), ..tx_native::TxConfig::default() };
    match tx_native::Transmitter::new(Arc::new(tx_native::SystemClock), ptt, sink, tx_config) {
        Ok(transmitter) => {
            let flag = Arc::clone(controls);
            transmitter.set_listener(Some(Arc::new(move |state| {
                flag.transmitting.store(state == tx_native::TxState::Transmitting, std::sync::atomic::Ordering::Relaxed);
            })));
            tracing::info!("trasmissione pronta: PTT {}{}", method, config.ptt_port.as_deref().filter(|_| method == "rts" || method == "dtr").map(|p| format!(" su {p}")).unwrap_or_default());
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
    // un ripristino fatto da Impostazioni > Sistema si applica qui, prima di aprire log e impostazioni
    backup::apply_pending(&config.data_dir);
    let (auth, logbook) = open_storage(&config.data_dir, pair_ttl)?;
    // il CAT: configurazione e profili in radio.json; la scelta salvata con «collega all'avvio» vale come AURORA_RIG
    let cat = rig_control::cat::CatService::open(&config.data_dir.join("radio.json"));
    let mut config = config;
    let stored = cat.config();
    // la scelta salvata dall'operatore nella pagina della radio vince su AURORA_RIG: il programma Windows la imposta da solo a «omnirig»
    // quando trova JTDX, e passando a Hamlib dalla pagina al riavvio si tornava su OmniRig (niente elenco dei modelli, radio «sparita»)
    if stored.auto_connect {
        config.rig_omnirig = false;
        config.rig_hamlib = false;
        match stored.backend {
            rig_control::cat_config::Backend::Omnirig => config.rig_omnirig = true,
            rig_control::cat_config::Backend::Hamlib => config.rig_hamlib = true,
            rig_control::cat_config::Backend::Tci | rig_control::cat_config::Backend::None => {}
        }
    }
    if config.rig_hamlib {
        cat.adopt_backend(rig_control::cat_config::Backend::Hamlib);
    } else if config.rig_omnirig {
        cat.adopt_backend(rig_control::cat_config::Backend::Omnirig);
    }
    let bus = Bus::new(4096);
    aurora_core::notice::install(bus.events());
    // la trasmissione e' sempre abilitata (nessuna opzione): restano STOP TX, i tetti, il dead-man e le automazioni spente di base
    let native_controls = Arc::new(engine_native::NativeControls::default());
    // il blocco della trasmissione chiesto dai controller UDP (TxInhibit): sta tra la catena di trasmissione e il PTT vero per tutta la vita del programma
    let tx_interlock = Arc::new(tx_native::TxInterlock::new());
    let live_tx = match config.engine {
        EngineKind::Sim => true,
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
        None => (String::new(), String::new()),
    };
    // finiscono in ogni record ADIF (`STATION_CALLSIGN`, `MY_GRIDSQUARE`)
    let station = Station { call: my_call.clone(), grid: my_grid.clone(), tx_power: config.tx_power.clone().unwrap_or_default() };
    // il log di Aurora si riempie sempre; i log esterni solo se configurati. Con il motore proprio il server UDP secondario delle impostazioni
    // si cambia a programma acceso: l'esportatore rilegge la lista a ogni QSO.
    let shared_targets = Arc::new(std::sync::RwLock::new(match &engine_settings {
        Some(store) => live_targets(&targets, &store.reports()),
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
        let (store, shared, base) = (Arc::clone(store), Arc::clone(&shared_targets), targets.clone());
        let mut changes = store.subscribe();
        tokio::spawn(async move {
            while changes.changed().await.is_ok() {
                let next = live_targets(&base, &store.reports());
                if let Ok(mut current) = shared.write() {
                    *current = next;
                }
            }
        });
    }
    // paesi e QSO gia' fatti si leggono una volta sola, prima di avviare il motore
    let log_filter = Arc::new(std::sync::RwLock::new(engine_settings.as_ref().map(|s| s.reports().log_filter).unwrap_or_default()));
    // le evidenziazioni di «nuovo» (scheda Notifiche): decidono le etichette delle decodifiche, anche «gia' lavorato»; cambiano a caldo
    let highlight_flags: engine_jtdx::enrich::SharedFlags = Arc::new(std::sync::RwLock::new(engine_settings.as_ref().map(|s| s.reports().highlights).unwrap_or_default()));
    let log_sources = LogSources { files: worked_paths, logbook: Arc::clone(&logbook), log4om: config.log4om_db.clone(), filter: Arc::clone(&log_filter) };
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
    // le schede Sequenza, Generali e Notifiche del motore proprio: dalle impostazioni di Aurora alla sequenza, alle etichette e al log
    if let Some(store) = &engine_settings {
        let r = store.reports();
        native_controls.monitor.store(!r.monitor_off_at_start, std::sync::atomic::Ordering::Relaxed);
        native_controls.set_sequence(sequence_settings(&r));
        native_controls.set_swr_limit(r.swr_limit_centi as f32 / 100.0);
        apply_wsjtx3_settings(&native_controls, &r);
        wire_fox_verifier(&native_controls, &bus);
        let (store, flags, filter, controls, live) = (Arc::clone(store), Arc::clone(&highlight_flags), Arc::clone(&log_filter), Arc::clone(&native_controls), live_log.clone());
        let mut changes = store.subscribe();
        tokio::spawn(async move {
            let (mut last_sequence, mut last_filter) = (sequence_settings(&r), r.log_filter);
            while changes.changed().await.is_ok() {
                let r = store.reports();
                controls.set_swr_limit(r.swr_limit_centi as f32 / 100.0);
                apply_wsjtx3_settings(&controls, &r);
                if let Ok(mut f) = flags.write() {
                    *f = r.highlights;
                }
                let sequence = sequence_settings(&r);
                if sequence != last_sequence {
                    controls.set_sequence(sequence.clone());
                    last_sequence = sequence;
                }
                if r.log_filter != last_filter {
                    if let Ok(mut f) = filter.write() {
                        *f = r.log_filter.clone();
                    }
                    last_filter = r.log_filter;
                    // il log si rilegge col filtro nuovo (fuori dal thread asincrono: legge file e basi di dati)
                    if let Some((db, worked, sources)) = live.clone() {
                        let _ = tokio::task::spawn_blocking(move || refresh_worked(&sources, &db, &worked)).await;
                    }
                }
            }
        });
    }
    let dxcc_loaded = enrichment.as_ref().map_or(0, |(db, _)| db.len());
    let worked_qsos = enrichment.as_ref().map_or(0, |(_, w)| w.read().map_or(0, |w| w.len()));
    let info = serde_json::json!({
        "version": aurora_core::version::APP_VERSION,
        "engine": match config.engine { EngineKind::Native => "native", EngineKind::Sim => "sim" },
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
    // i QSO conclusi che aspettano la conferma della finestra («Richiedimi di loggare il QSO»)
    let log_prompt = Arc::new(remote_api::log_prompt::LogPrompt::new());
    let cluster = dx_cluster::ClusterService::open(&config.data_dir.join("dx-cluster.json"));
    // il livello dell'ingresso audio si regola sullo stesso dispositivo da cui viene la cascata
    // con il motore proprio il dispositivo e' quello della scheda Audio (store, poi ambiente); poi il gestore lo fa seguire a ogni cambio di ingresso
    let level_hint: Option<Option<String>> = match &engine_settings {
        Some(store) => audio_control::input_candidates(&store.audio().input, config.audio_device.as_deref()).into_iter().next().map(|(hint, _)| hint),
        None => config.audio_device.clone().map(Some),
    };
    // l'ingresso TCI non e' un dispositivo di Windows: niente volume di registrazione da regolare (il livello si regola nella SDR)
    let level_hint = level_hint.filter(|h| !audio_control::is_tci_device(h.as_deref(), audio_spectrum::TCI_INPUT_NAME));
    let input_level = level_hint.and_then(|hint| match audio_control::find_endpoint(hint.as_deref()) {
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
    // il DX Cluster entra con il nominativo dei dati di stazione e lo segue se cambia nelle impostazioni
    cluster.set_station_call(&my_call);
    if let Some(store) = engine_settings.clone() {
        let (cluster, mut changes) = (Arc::clone(&cluster), store.subscribe());
        tokio::spawn(async move {
            while changes.changed().await.is_ok() {
                cluster.set_station_call(&store.reports().call);
            }
        });
    }
    cluster.restart();
    let own_log = Arc::clone(&logbook);
    // i rapporti degli errori: spenti di fabbrica, si accendono in Impostazioni > Assistenza
    let rapporti = rapporti::Rapporti::avvia(&config.data_dir);
    // senza CAT la frequenza la sceglie l'operatore con le bande (`with_manual_dial`) e resta al riavvio
    let manual_file = config.data_dir.join(MANUAL_DIAL_FILE);
    let dial_hz = Arc::new(std::sync::atomic::AtomicU64::new(std::fs::read_to_string(&manual_file).ok().and_then(|t| t.trim().parse().ok()).unwrap_or(0)));
    // QMAP: ricezione panoramica Q65 da un flusso I/Q (TCI o file); spento finche' non si sceglie la sorgente. Nominativo e locatore
    // (ipotesi AP e Doppler dell'eco) li legge dalle impostazioni della stazione a ogni uso.
    let qmap_service = qmap::QmapService::open(Some(&config.data_dir.join("qmap.json")));
    {
        let station = Arc::clone(&log_station);
        qmap_service.set_station(Arc::new(move || {
            let s = station();
            (s.call, s.grid)
        }));
    }
    let state = AppState::new(bus.clone(), auth, Arc::clone(&logbook), TxPolicy::default_safe().with_live_tx(live_tx), config.secure_cookie)
        .with_station_info(info)
        .with_open_access(config.open_access || read_open_access(&config.data_dir.join(OPEN_ACCESS_FILE)))
        .with_open_access_file(config.data_dir.join(OPEN_ACCESS_FILE))
        .with_local_trust(config.local_trust)
        .with_clock(Arc::clone(&clock))
        .with_cluster(Arc::clone(&cluster))
        .with_qmap(Arc::clone(&qmap_service))
        .with_log_prompt(Arc::clone(&log_prompt))
        .with_wishlist(Arc::clone(&wishlist))
        .with_input_level_if(input_level.clone())
        .with_update_installer_if(config.update_installer.clone().map(|h| h.0))
        .with_save_dialog_if(config.save_dialog.clone().map(|h| h.0))
        .with_crash_folder_if(config.crash_folder.clone().map(|h| h.0))
        .with_settings_backup(
            {
                let (dir, book) = (config.data_dir.clone(), Arc::clone(&logbook));
                Arc::new(move || {
                    let dest = support::desktop_dir().unwrap_or_else(|| dir.clone());
                    backup::make(&dir, &book, &dest, env!("CARGO_PKG_VERSION"))
                })
            },
            {
                let dir = config.data_dir.clone();
                Arc::new(move |bytes: Vec<u8>| backup::stage(&dir, &bytes))
            },
        )
        .with_support_bundle_if(Some({
            // il PTT si ricalcola a ogni pacchetto: la scelta in Impostazioni > Radio puo' essere cambiata
            let config = config.clone();
            let cat = Arc::clone(&cat);
            Arc::new(move |req: remote_api::SupportRequest| {
                let ctx = support::Context {
                    data_dir: config.data_dir.clone(),
                    engine: match config.engine { EngineKind::Native => "native", EngineKind::Sim => "sim" }.to_string(),
                    ptt_method: if matches!(config.engine, EngineKind::Native) { ptt_method_for(&config, &cat) } else { "quello di JTDX".to_string() },
                    ptt_port: config.ptt_port.clone(),
                    version: aurora_core::version::APP_VERSION.to_string(),
                };
                support::build(&ctx, &req)
            }) as remote_api::SupportBundle
        }))
        .with_error_reports_if(Some(Arc::clone(&rapporti) as remote_api::ErrorReportsHandle))
        .with_online(online)
        .with_history_if(history)
        .with_updates(Arc::clone(&updates))
        .with_dxcc_if(enrichment.as_ref().map(|(db, _)| {
            let db = Arc::clone(db);
            Arc::new(move |call: &str| {
                db.lookup(call).map(|l| {
                    let e = db.entity(l.entity);
                    serde_json::json!({ "country": e.name, "continent": e.continent, "prefix": e.prefix, "cq_zone": l.cq_zone, "itu_zone": l.itu_zone, "lat": f64::from(e.lat_e2) / 100.0, "lon": f64::from(e.lon_e2) / 100.0 })
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
        .with_manual_dial({
            let dial = Arc::clone(&dial_hz);
            let file = config.data_dir.join(MANUAL_DIAL_FILE);
            Arc::new(move |hz: u64| {
                dial.store(hz, std::sync::atomic::Ordering::Relaxed);
                if let Err(e) = std::fs::write(&file, hz.to_string()) {
                    tracing::warn!("frequenza a mano non salvata: {e}");
                }
                tracing::info!("frequenza a mano (senza CAT): {hz} Hz");
            })
        })
        .with_log_deleted({
            let own_file = config.data_dir.join(OWN_LOG_FILE);
            let live = live_log.clone();
            Arc::new(move |gone: Option<Vec<logbook::LogEntry>>| {
                forget_in_own_file(&own_file, gone.as_deref());
                if let Some((db, worked, sources)) = &live {
                    refresh_worked(sources, db, worked);
                }
            })
        })
        .with_log_changed(match live_log.clone() {
            Some((db, worked, sources)) => Arc::new(move || refresh_worked(&sources, &db, &worked)),
            None => Arc::new(|| {}),
        })
        .with_log4om_if(Some(match config.log4om_db.clone() {
            Some(path) => Arc::new(move |limit: usize| {
                logbook::log4om::read_recent(&path, limit)
                    .map(|view| {
                        let mut v = serde_json::json!(view);
                        v["source"] = serde_json::json!("log4om");
                        v
                    })
                    .map_err(|e| e.to_string())
            }) as remote_api::Log4omProvider,
            // nessun log esterno collegato: il pannello Log mostra il log interno di Aurora (con l'esportazione ADIF)
            None => {
                let own = Arc::clone(&own_log);
                let dxcc = enrichment.as_ref().map(|(db, _)| Arc::clone(db));
                Arc::new(move |limit: usize| {
                    let country = |call: &str| dxcc.as_ref().and_then(|db| db.lookup(call).map(|l| db.entity(l.entity).name.clone())).unwrap_or_default();
                    own.recent_view(limit, &country)
                        .map(|view| {
                            let mut v = serde_json::json!(view);
                            v["source"] = serde_json::json!("aurora");
                            v
                        })
                        .map_err(|e| e.to_string())
                }) as remote_api::Log4omProvider
            }
        }))
        // i quadrati lavorati: dagli stessi log della scheda DXCC (Aurora, file ADIF, Log4OM2), anche per banda, modo e conferma (Mappa DX);
        // senza paesi caricati, solo da Log4OM2 come prima
        .with_worked_map_if(match (live_log.clone(), config.log4om_db.clone()) {
            (Some((db, worked, sources)), _) => Some(Arc::new(move || {
                refresh_worked(&sources, &db, &worked);
                let w = worked.read().map_err(|_| "QSO gia' fatti non disponibili".to_string())?;
                let cells = w.grid_cells();
                let mut squares: Vec<serde_json::Value> = Vec::new();
                let mut i = 0;
                while i < cells.len() {
                    let grid = cells[i].grid.clone();
                    let (mut count, mut bands) = (0u64, Vec::<String>::new());
                    while i < cells.len() && cells[i].grid == grid {
                        count += u64::from(cells[i].qsos);
                        if !bands.contains(&cells[i].band) {
                            bands.push(cells[i].band.clone());
                        }
                        i += 1;
                    }
                    squares.push(serde_json::json!({ "grid": grid, "count": count, "bands": bands }));
                }
                Ok(serde_json::json!({ "squares": squares, "cells": cells }))
            }) as remote_api::WorkedMapProvider),
            (None, Some(path)) => Some(Arc::new(move || {
                logbook::log4om::worked_squares(&path)
                    .map(|squares| serde_json::json!({ "squares": squares }))
                    .map_err(|e| e.to_string())
            }) as remote_api::WorkedMapProvider),
            // senza file dei paesi: i quadrati dal solo log di Aurora
            (None, None) => {
                let own = Arc::clone(&own_log);
                Some(Arc::new(move || {
                    let rows = own.all_qsos().map_err(|e| e.to_string())?;
                    let mut cells: std::collections::BTreeMap<(String, String, String), (u32, u32, u32)> = std::collections::BTreeMap::new();
                    for q in &rows {
                        let g = q.grid.trim().to_uppercase();
                        let b = g.as_bytes();
                        if b.len() < 4 || !(b'A'..=b'R').contains(&b[0]) || !(b'A'..=b'R').contains(&b[1]) || !b[2].is_ascii_digit() || !b[3].is_ascii_digit() {
                            continue;
                        }
                        let c = cells.entry((g[..4].to_string(), q.band.trim().to_lowercase(), q.mode.trim().to_uppercase())).or_default();
                        c.0 += 1;
                        match q.confirmed {
                            Some(true) => c.1 += 1,
                            Some(false) => c.2 += 1,
                            None => {}
                        }
                    }
                    let mut squares: Vec<serde_json::Value> = Vec::new();
                    let mut last: Option<(String, u64, Vec<String>)> = None;
                    for ((grid, band, _), (n, _, _)) in &cells {
                        match &mut last {
                            Some((g, count, bands)) if g == grid => {
                                *count += u64::from(*n);
                                if !bands.contains(band) {
                                    bands.push(band.clone());
                                }
                            }
                            _ => {
                                if let Some((g, count, bands)) = last.take() {
                                    squares.push(serde_json::json!({ "grid": g, "count": count, "bands": bands }));
                                }
                                last = Some((grid.clone(), u64::from(*n), vec![band.clone()]));
                            }
                        }
                    }
                    if let Some((g, count, bands)) = last {
                        squares.push(serde_json::json!({ "grid": g, "count": count, "bands": bands }));
                    }
                    let cells: Vec<serde_json::Value> = cells
                        .into_iter()
                        .map(|((grid, band, mode), (qsos, confirmed, unconfirmed))| serde_json::json!({ "grid": grid, "band": band, "mode": mode, "qsos": qsos, "confirmed": confirmed, "unconfirmed": unconfirmed }))
                        .collect();
                    Ok(serde_json::json!({ "squares": squares, "cells": cells }))
                }) as remote_api::WorkedMapProvider)
            }
        })
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
            EngineKind::Sim => config.rig_hamlib.then(|| Arc::clone(&cat) as Arc<dyn rig_control::Rig>),
        })
        .with_cat(Arc::clone(&cat))
        .with_automation(automation);
    // il QMAP non cerca nelle meta' del minuto in cui la stazione trasmette
    qmap_service.set_tx_probe(state.transmitting_probe());
    // l'audio del decodificatore: con il motore proprio lo apre e lo cambia a caldo il gestore della scheda Audio (`audio_control`)
    let (native_tx, native_rx) = std::sync::mpsc::channel();
    let audio_manager: Option<Arc<audio_control::AudioManager>> = engine_settings.as_ref().map(|store| {
        let level = input_level.clone();
        let hook: audio_control::InputHook = Arc::new(move |name| {
            if let Some(level) = &level
                && !audio_control::is_tci_device(Some(name), audio_spectrum::TCI_INPUT_NAME)
            {
                match audio_control::find_endpoint(Some(name)) {
                    Ok(endpoint) => level.set_control(Arc::new(endpoint)),
                    Err(e) => tracing::warn!("livello dell'ingresso audio non regolabile su {name}: {e}"),
                }
            }
        });
        let backend = Arc::new(audio_control::CpalBackend { rows: bus.spectrum(), events: bus.events(), cat: Some(Arc::clone(&cat)) });
        audio_control::AudioManager::new(backend, native_tx.clone(), store.audio(), config.audio_device.clone(), config.audio_out_device.clone(), &config.data_dir, hook)
    });
    let state = match (&engine_settings, &audio_manager) {
        (Some(store), Some(manager)) => {
            store.set_feature(remote_api::engine_settings::Feature::AudioControl, true);
            state.with_engine_settings(Arc::clone(store)).with_audio(Arc::clone(manager) as Arc<dyn remote_api::audio_api::AudioControl>)
        }
        _ => state,
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
    let mut native_live: Option<engine_native::LiveConfig> = None;
    let engine_task = match config.engine {
        EngineKind::Native => {
            // il motore proprio decodifica dall'audio della cascata (avviato piu' sotto); qui si prepara cio' che pubblica
            let (call, grid) = (my_call.clone(), my_grid.clone());
            let own_continent = enrichment.as_ref().and_then(|(db, _)| db.lookup(&call).map(|hit| db.entity(hit.entity).continent.clone()));
            let grid_db = enrichment.as_ref().map(|(db, _)| Arc::clone(db));
            let enrich = enrichment.map(|(db, worked)| Arc::new(Mutex::new(engine_jtdx::enrich::Enrichment::with_shared(db, worked).with_flags(Arc::clone(&highlight_flags)))));
            let mode_controls = Arc::clone(&native_controls);
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
            // i nominativi gia' collegati insegnano al filtro contro i falsi decode chi esiste
            let log_calls = enrich.as_ref().and_then(|e| e.lock().ok().map(|g| g.shared_worked())).map(|worked| {
                Arc::new(move || worked.read().map(|w| w.calls()).unwrap_or_default()) as Arc<dyn Fn() -> Vec<String> + Send + Sync>
            });
            // il locatore deve essere credibile per il paese del nominativo (cty.dat)
            let grid_check = grid_db.map(|db| engine_native::GridCheck::new(move |call, grid| db.grid_plausible(call, grid)));
            native_live = Some(engine_native::LiveConfig {
                log_calls,
                grid_check,
                de_call: call,
                de_grid: grid,
                own_continent,
                dial_hz: Arc::clone(&dial_hz),
                controls: Arc::clone(&native_controls),
                transmitter: audio_manager.as_ref().and_then(|m| {
                    let transmitter = build_transmitter(&config, &cat, &native_controls, &tx_interlock, m.build_output()?);
                    if transmitter.is_none() {
                        m.output_closed();
                    }
                    let manager = Arc::clone(m);
                    // su Windows Pwr e' il volume della scheda; altrove quel comando non c'e' e il cursore attenua il segnale, come JTDX
                    // (senza, su Linux si trasmetteva sempre al massimo)
                    transmitter.map(|inner| {
                        if cfg!(windows) {
                            let port = WindowsVolumePort { inner, output: Arc::new(move || manager.output_name()), controls: Arc::clone(&native_controls) };
                            Arc::new(port) as Arc<dyn engine_native::TxPort>
                        } else {
                            drop(manager);
                            inner
                        }
                    })
                }),
                band_of: Arc::new(|hz| rig_control::bands::amateur_band(hz).map(str::to_string)),
                annotate: Arc::new(move |text, band| match &enrich {
                    Some(e) => match e.lock() {
                        Ok(e) => {
                            let a = e.annotate_in(text, band, Some(mode_controls.mode().name()));
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
                        aurora_core::Command::QueueReply { decode_id } => engine_native::TxCommand::QueueReply { decode_id },
                        aurora_core::Command::SendFreeText { text, send } => engine_native::TxCommand::FreeText { text, send },
                        aurora_core::Command::CallCq { modifier } => engine_native::TxCommand::CallCq { modifier: Some(modifier).filter(|m| !m.trim().is_empty()) },
                        aurora_core::Command::SetTxFrequency { hz } => engine_native::TxCommand::SetTxFreq(hz),
                        aurora_core::Command::SetDx { call, grid } => engine_native::TxCommand::SetDx {
                            call: Some(call).filter(|c| !c.is_empty()),
                            grid: Some(grid).filter(|g| !g.is_empty()),
                            generate: true,
                        },
                        aurora_core::Command::SetReport { db } => engine_native::TxCommand::SetReport(db),
                        aurora_core::Command::Tune { on } => engine_native::TxCommand::Tune(on),
                        aurora_core::Command::LogQso => engine_native::TxCommand::LogQso,
                        aurora_core::Command::Fox(cmd) => engine_native::TxCommand::Fox(cmd),
                    };
                    if let Err(e) = controls.send(mapped) {
                        tracing::warn!("comando di trasmissione non consegnato: {e}");
                    }
                }
            });
            task
        }
        EngineKind::Sim => tokio::spawn(SimEngine::new(SimConfig { seed: 1, cycle_ms: config.cycle_ms }).run(io)),
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
        let inhibit: Arc<dyn udp_emitter::InhibitControl> = Arc::new(InhibitBridge { interlock: Arc::clone(&tx_interlock), controls: Arc::clone(&native_controls) });
        *udp_emitter.lock().await = start_udp_emitter(&wanted, &bus, &native_controls, &state, &inhibit).await;
        if let Some(store) = engine_settings.clone() {
            let (slot, config, bus, controls, call, grid, app) = (Arc::clone(&udp_emitter), config.clone(), bus.clone(), Arc::clone(&native_controls), my_call.clone(), my_grid.clone(), state.clone());
            let inhibit = Arc::clone(&inhibit);
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
                    *slot = start_udp_emitter(&next, &bus, &controls, &app, &inhibit).await;
                    running = next;
                }
            });
        }
    }

    if let Some(store) = &engine_settings {
        wire_reporting(store, &bus, &native_controls, &config.data_dir, config.tx_power.clone().unwrap_or_default(), Arc::clone(&log_prompt));
    }

    // la lista dei desiderati: guarda messaggi e spot e avvisa (anche a pagina chiusa)
    wanted::spawn_watcher(Arc::clone(&wishlist), bus.events(), Some(Arc::clone(&cluster)));

    // le nuove versioni sul repository: un controllo poco dopo l'avvio e poi ogni qualche ora
    {
        let phone = Arc::clone(&wishlist);
        updates::spawn_checker(Arc::clone(&updates), bus.events(), Arc::new(move || phone.config().notify_url), Duration::from_secs(8));
    }

    // la frequenza della radio per il motore proprio. Il CAT la tiene gia' in memoria (la lettura costa niente) e si rilegge ogni quarto di secondo;
    // OmniRig la chiede a PowerShell (qualche centinaio di millisecondi a lettura): si aspetta quattro volte il tempo che e' costata, per non
    // tenere occupato il computer. Prima era ogni 3 s e il cambio di frequenza compariva in ritardo.
    if config.engine == EngineKind::Native
        && let Some(rig) = state.rig.clone()
    {
        let dial = Arc::clone(&dial_hz);
        std::thread::spawn(move || {
            loop {
                let started = std::time::Instant::now();
                if let Ok(hz) = rig.frequency() {
                    dial.store(hz, std::sync::atomic::Ordering::Relaxed);
                }
                let spent = started.elapsed();
                std::thread::sleep((spent * 4).clamp(Duration::from_millis(250), Duration::from_secs(3)));
            }
        });
        // gli strumenti della radio (S-meter in ricezione; potenza, ROS, ALC in trasmissione) letti dal CAT Hamlib: il CAT limita da solo il ritmo
        // delle letture (250 ms in ricezione, 200 ms in trasmissione, un comando per giro sulle seriali lente); con OmniRig o senza radio restano spenti
        let cat = Arc::clone(&cat);
        let controls = Arc::clone(&native_controls);
        std::thread::spawn(move || {
            loop {
                if let Ok(rig) = cat.hamlib() {
                    rig.set_external_tx(controls.transmitting.load(std::sync::atomic::Ordering::Relaxed));
                }
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

    spawn_start_on_main_frequency(state.clone(), config.data_dir.clone());
    // la programmazione delle bande ha senso solo se la radio e' comandabile
    if let Some(rig) = state.rig.clone() {
        spawn_scheduler(state.clone(), rig.clone());
        band_hop::spawn(state.clone(), rig.clone());
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
    // il motore proprio decodifica l'audio della cascata (e in ombra, con `AURORA_NATIVE_DECODE`, si confronta con JTDX)
    // modo, frequenza e trasmissione si pubblicano subito, anche se l'ingresso audio non c'e' (il motore che decodifica parte con l'audio)
    if let Some(live) = native_live.clone() {
        engine_native::publish_status(bus.events(), live);
    }
    let start_engine = {
        let (events, dir, record, live, engine_state) = (bus.events(), config.data_dir.clone(), config.native_record, native_live.take(), state.clone());
        move |save: Option<Arc<engine_native::SaveSettings>>| {
            let engine = engine_native::start_with_save(events, native_rx, dir, record, live, save);
            engine_state.set_native_status(Arc::new(move || engine.status()));
        }
    };
    let audio = match &audio_manager {
        // motore proprio: ingresso, canali e salvataggio vengono dalla scheda Audio e si cambiano a caldo; il decodificatore parte con il primo ingresso aperto
        Some(manager) => {
            manager.set_engine_starter(Box::new(move |save| start_engine(Some(save))));
            manager.open_initial_input();
            // la scheda audio della radio puo' comparire dopo (radio accesa dopo Aurora, cavo USB rimesso): si riprova da soli
            // e puo' sparire durante l'uso: ogni 5 s si controlla e, se manca, si riprova (l'avviso al massimo ogni 2 minuti)
            if manager.wants_input() {
                let m = Arc::clone(manager);
                // il decodificatore parte con il primo ingresso aperto e ha bisogno del runtime asincrono: senza, riaprire l'audio da
                // questo thread mandava Aurora in crash («there is no reactor running», 0.11.155-0.11.158)
                let runtime = tokio::runtime::Handle::current();
                std::thread::spawn(move || {
                    let _runtime = runtime.enter();
                    let mut missing = !m.has_input();
                    loop {
                        std::thread::sleep(std::time::Duration::from_secs(5));
                        if m.retry_input() {
                            if missing {
                                tracing::info!("ingresso audio trovato: la ricezione riparte");
                                missing = false;
                            }
                        } else {
                            missing = true;
                            aurora_core::notice::notice("audio_missing", "");
                        }
                    }
                });
            }
            None
        }
        None => {
            let tap = config.native_decode.then(|| native_tx.clone());
            let handle = config.audio_device.as_deref().and_then(|hint| match run_capture_tapped(Some(hint), bus.spectrum(), bus.events(), tap) {
                Ok(handle) => Some(handle),
                Err(e) => {
                    tracing::warn!("cascata audio non attiva: {e}");
                    None
                }
            });
            if config.native_decode && handle.is_some() {
                start_engine(None);
            }
            handle
        }
    };

    let listener = TcpListener::bind(config.bind)
        .await
        .map_err(|e| StartError::Io(port_error(config.bind, &e)))?;
    let addr = listener.local_addr().map_err(|e| StartError::Io(e.to_string()))?;
    state.lan.set_port(addr.port());
    // nome fisso in casa (aurora.local): solo se il server e' raggiungibile dalla rete; parte in disparte per non ritardare l'avvio
    let announcer: Arc<std::sync::Mutex<Option<remote_api::lan::Announcer>>> = Arc::default();
    if !addr.ip().is_loopback() {
        let slot = Arc::clone(&announcer);
        let lan = Arc::clone(&state.lan);
        tokio::task::spawn_blocking(move || {
            let started = remote_api::lan::Announcer::start(addr.port(), &lan);
            if let Ok(mut s) = slot.lock() {
                *s = started;
            }
        });
    }

    let closing = state.clone();
    let cat_for_task = Arc::clone(&cat);
    let audio_for_task = audio_manager.clone();
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
        // l'annuncio mDNS si spegne con Aurora
        if let Ok(mut s) = announcer.lock() {
            s.take();
        }
        if let Some(emitter) = udp_emitter.lock().await.take() {
            emitter.shutdown().await;
        }
        if let Some(manager) = &audio_for_task {
            manager.shutdown();
        }
        drop(audio);
        // alla chiusura il PTT si spegne e la radio si libera
        let _ = tokio::task::spawn_blocking(move || cat_for_task.shutdown()).await;
        CORE_STOPPED.store(true, std::sync::atomic::Ordering::SeqCst);
    });
    {
        // radio, PTT, audio e i valori personali da togliere si rileggono a ogni rapporto
        let state = state.clone();
        let config = config.clone();
        let cat = Arc::clone(&cat);
        rapporti.imposta_contesto(Arc::new(move || {
            let radio = cat.config();
            let audio = state.audio.as_ref().map(|a| a.devices());
            let mut noti: Vec<String> = Vec::new();
            if let Some(s) = state.current_status() {
                noti.push(s.de_call);
                noti.push(s.de_grid);
            }
            rapporti::Contesto {
                versione: aurora_core::version::APP_VERSION.to_string(),
                sistema: String::new(),
                motore: match config.engine { EngineKind::Native => "native", EngineKind::Sim => "sim" }.to_string(),
                radio: if radio.rig_name.is_empty() && radio.model != 0 { format!("hamlib {}", radio.model) } else { radio.rig_name.clone() },
                ptt: if matches!(config.engine, EngineKind::Native) { ptt_method_for(&config, &cat) } else { "jtdx".to_string() },
                cat: format!("{:?}", radio.backend).to_ascii_lowercase(),
                audio_in: audio.as_ref().and_then(|a| a.input_in_use.clone()).or_else(|| config.audio_device.clone()).unwrap_or_default(),
                audio_out: audio.as_ref().and_then(|a| a.output_in_use.clone()).or_else(|| config.audio_out_device.clone()).unwrap_or_default(),
                noti,
            }
        }));
    }
    Ok(RunningServer { addr, state, task })
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
    fn the_secondary_server_and_the_integrations_join_the_fixed_targets_without_duplicates() {
        use remote_api::engine_settings::{Reports, UdpDestination};
        let (targets, _) = export_targets(&config_for(EngineKind::Native));
        let log: SocketAddr = "127.0.0.1:2237".parse().expect("indirizzo");
        let own_file = Target::File(PathBuf::from("dati").join("aurora_log.adi"));
        let off = Reports::default();
        assert_eq!(live_targets(&targets, &off), vec![own_file.clone()], "l'ambiente e' solo il valore iniziale del riquadro: senza destinatari resta il file");
        let adif = UdpDestination { adif: true, ..UdpDestination::from_addr("Log4OM", log) };
        let with_list = Reports { destinations: vec![adif.clone()], ..Reports::default() };
        assert_eq!(live_targets(&targets, &with_list), vec![Target::UdpAdif(log), own_file.clone()]);
        let same = Reports { secondary: Some(log), destinations: vec![adif], ..Reports::default() };
        assert_eq!(live_targets(&targets, &same).len(), 2, "secondo server uguale a un destinatario: un solo invio");
        let n1mm = UdpDestination { n1mm: true, ..UdpDestination::from_addr("HRD", "127.0.0.1:12060".parse().expect("indirizzo")) };
        let got = live_targets(&targets, &Reports { secondary: Some("192.168.1.9:2240".parse().expect("indirizzo")), destinations: vec![n1mm], ..Reports::default() });
        assert_eq!(got.len(), 3);
        assert!(got.contains(&Target::N1mmXml("127.0.0.1:12060".parse().expect("indirizzo"))));
        assert!(got.contains(&own_file), "il file di Aurora c'e' sempre");
    }

    #[test]
    fn the_udp_emitter_follows_the_primary_server_and_the_integrations() {
        use remote_api::engine_settings::{EngineSettingsStore, Feature, UDP_DESTINATIONS};
        let dir = std::env::temp_dir().join(format!("aurora-udpset-{}", std::process::id()));
        std::fs::remove_dir_all(&dir).ok();
        let store = EngineSettingsStore::open(&dir.join("settings-motore.json"), remote_api::engine_settings::defaults);
        store.set_feature(Feature::UdpEmitter, true);
        let mut c = config_for(EngineKind::Native);
        c.udp_forward = vec!["127.0.0.1:2333".parse().expect("indirizzo"), "127.0.0.1:2400".parse().expect("indirizzo")];
        let first = udp_settings(&c, Some(&store), "K1ABC", "FN42");
        assert_eq!(first.targets, vec!["127.0.0.1:2237".parse().expect("indirizzo")], "il primo server delle impostazioni; il resto dell'ambiente sta nel riquadro");
        assert!(first.destinations.is_empty());
        assert!(first.send_adif && !first.accept_requests, "ADIF al primo server acceso, comandi UDP spenti di fabbrica");
        let list = r#"[{"name":"Gruppo","host":"239.255.0.1","port":2237,"wsjtx":true,"multicast":true,"ttl":3},{"name":"Log","host":"127.0.0.1","port":2333,"adif":true}]"#;
        let changes: std::collections::BTreeMap<String, String> =
            [("UDPServerPort", "2333"), ("EnableUDP1adifSending", "false"), ("AcceptUDPRequests", "true"), (UDP_DESTINATIONS, list)].iter().map(|(k, v)| ((*k).to_string(), (*v).to_string())).collect();
        store.update(&changes).expect("valido");
        let next = udp_settings(&c, Some(&store), "K1ABC", "FN42");
        assert_ne!(first, next, "una modifica fa riavviare l'emettitore");
        assert_eq!(next.targets[0], "127.0.0.1:2333".parse().expect("indirizzo"));
        assert!(!next.send_adif && next.accept_requests);
        assert_eq!(next.destinations.len(), 1, "solo chi vuole il flusso binario o il Logged ADIF");
        assert_eq!(next.destinations[0].multicast, Some(udp_emitter::Multicast { ttl: 3, interface: None }));
        assert_eq!(udp_settings(&c, Some(&store), "K1ABC", "FN42"), next, "senza modifiche non cambia: niente riavvii inutili");
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn an_explicit_log_file_is_not_duplicated_and_the_jtdx_engine_is_gone() {
        let mut c = config_for(EngineKind::Native);
        c.export_file = Some(PathBuf::from("dati").join("aurora_log.adi"));
        assert_eq!(export_targets(&c).0.len(), 2, "lo stesso file non va due volte");
        assert!(Config::from_lookup(|k| (k == "AURORA_ENGINE").then(|| "jtdx".to_string())).is_err(), "Aurora ha solo il suo motore");
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
}

/// La porta del server non si apre: se e' gia' in uso si dice da quale programma (su Windows), che di solito e' un'altra Aurora aperta.
fn port_error(bind: std::net::SocketAddr, e: &std::io::Error) -> String {
    if e.kind() != std::io::ErrorKind::AddrInUse {
        return format!("{bind}: {e}");
    }
    let who = port_owner(bind.port()).map_or_else(|| "un altro programma (forse Aurora e' gia' aperta)".to_string(), |p| format!("il programma «{p}»"));
    format!("la porta {} e' gia' usata da {who}: chiudilo o scegli un'altra porta in Impostazioni > Accesso", bind.port())
}

#[cfg(windows)]
fn port_owner(port: u16) -> Option<String> {
    use std::os::windows::process::CommandExt;
    let script = format!("(Get-Process -Id (Get-NetTCPConnection -LocalPort {port} -State Listen -ErrorAction Stop | Select-Object -First 1).OwningProcess).ProcessName");
    let out = std::process::Command::new("powershell.exe").args(["-NoProfile", "-NonInteractive", "-Command", &script]).creation_flags(0x0800_0000).output().ok()?;
    let name = String::from_utf8_lossy(&out.stdout).trim().to_string();
    (out.status.success() && !name.is_empty()).then_some(name)
}

#[cfg(not(windows))]
fn port_owner(_port: u16) -> Option<String> {
    None
}
