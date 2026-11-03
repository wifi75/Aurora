//! API HTTP versionata (`/api/v1`) e flusso WebSocket di Aurora.

pub mod audio_api;
mod cat;
pub mod engine_settings;
mod frames;
pub mod lan;
mod policy;
mod routes;
pub use routes::{is_secret_name, redact_json};
pub mod ui_settings;
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
pub use routes::{switch_mode, BAND_HOP_NEXT};
pub use policy::{transport_of, Transport, TxPolicy};

/// Salva le regole dell'automazione quando cambiano.
pub type RulesSaver = Arc<dyn Fn(&Rules) + Send + Sync>;
/// Calcola le statistiche del log a ogni richiesta.
/// Legge gli ultimi N QSO del log di Log4OM2 (sola lettura).
pub type Log4omProvider = Arc<dyn Fn(usize) -> Result<serde_json::Value, String> + Send + Sync>;
/// I quadrati locatore gia' lavorati (per la mappa), dal log.
pub type WorkedMapProvider = Arc<dyn Fn() -> Result<serde_json::Value, String> + Send + Sync>;
/// Legge le impostazioni di JTDX dal suo file di configurazione (sola lettura).
pub type JtdxConfigProvider = Arc<dyn Fn() -> Result<serde_json::Value, String> + Send + Sync>;
/// Dato un nominativo, paese e zone dal file cty.dat.
pub type DxccProvider = Arc<dyn Fn(&str) -> Option<serde_json::Value> + Send + Sync>;
/// Applica modifiche alle impostazioni di JTDX (chiude JTDX, scrive il suo file, lo riavvia).
pub type JtdxApplier = Arc<dyn Fn(std::collections::BTreeMap<String, String>) -> Result<(), String> + Send + Sync>;
pub type StatsProvider = Arc<dyn Fn() -> serde_json::Value + Send + Sync>;
/// Di quale stazione la scheda DXCC deve dire le etichette (tutti i campi sono facoltativi).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MatrixQuery {
    pub call: Option<String>,
    pub grid: Option<String>,
    pub band: Option<String>,
    pub mode: Option<String>,
    /// Senza l'elenco dei paesi: per chi vuole solo i totali e le etichette di una stazione.
    pub compact: bool,
}
/// La scheda DXCC (matrice paese x banda x modo, contatori e, per una stazione, le sei etichette) dal log interno e dai log esterni.
pub type DxccMatrixProvider = Arc<dyn Fn(MatrixQuery) -> Result<serde_json::Value, String> + Send + Sync>;
/// Nominativo, locatore e potenza di chi opera, per i record ADIF esportati (possono cambiare a programma acceso).
pub type LogStationProvider = Arc<dyn Fn() -> logbook::adif::Station + Send + Sync>;
/// Chiamato dopo che il log interno e' cambiato in blocco (importazione): chi tiene in memoria i QSO gia' fatti li rilegge.
pub type LogChangedHook = Arc<dyn Fn() + Send + Sync>;
/// La finestra «Salva con nome» del sistema, aperta nel programma Windows: riceve il nome proposto e ritorna il file scelto
/// (`None` = annullato). E' bloccante: si chiama fuori dal thread asincrono.
pub type SaveFileDialog = Arc<dyn Fn(&str) -> Option<std::path::PathBuf> + Send + Sync>;

/// Apre in Esplora risorse la cartella dei crash del programma Windows (solo dal PC stesso).
pub type OpenCrashFolder = Arc<dyn Fn() -> Result<(), String> + Send + Sync>;

/// Quello che il pacchetto per l'assistenza riceve dalla rotta: lo stato del momento (gia' raccolto, senza segreti) e cosa sa la pagina.
#[derive(Debug, Clone, Default)]
pub struct SupportRequest {
    /// Stato del momento: collegamento della radio, trasmissione, audio, orologio, servizi.
    pub live: serde_json::Value,
    /// Lingua dell'interfaccia.
    pub lang: String,
    /// Scarto dell'ora locale da UTC in minuti, come lo vede la pagina (+60 = ora italiana d'inverno).
    pub local_offset_min: i32,
}

/// Il pacchetto creato: dove sta e il PTT in uso (per il messaggio di posta).
#[derive(Debug, Clone)]
pub struct SupportMade {
    pub path: std::path::PathBuf,
    pub ptt: String,
}

/// Crea il pacchetto per l'assistenza (zip sul Desktop). E' bloccante: si chiama fuori dal thread asincrono.
pub type SupportBundle = Arc<dyn Fn(SupportRequest) -> Result<SupportMade, String> + Send + Sync>;

const MAX_KNOWN_DECODES: usize = 500;

#[derive(Default)]
struct Shared {
    known: BTreeSet<u64>,
    status: Option<StationStatus>,
    /// Le evidenziazioni dei nominativi chieste dai programmi collegati (Highlight Callsign): una pagina aperta dopo le riceve subito.
    highlights: std::collections::BTreeMap<String, Event>,
}

/// Quante evidenziazioni si ricordano al massimo (le piu' vecchie si scartano).
const MAX_HIGHLIGHTS: usize = 500;

/// Installa la versione nuova trovata dal controllo: la scarica, ne verifica la firma, avvia l'installazione e chiude il programma.
/// Riceve il servizio degli aggiornamenti (release trovata e token). Ritorna la versione che sta installando; e' bloccante.
/// Racconta lo stato del motore proprio in JSON.
pub type NativeStatusFn = Arc<dyn Fn() -> serde_json::Value + Send + Sync>;

pub trait UpdateInstaller: Send + Sync {
    fn install(&self, updates: Arc<updater::UpdateService>) -> Result<String, String>;
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
    /// Il CAT: configurazione, profili e collegamento della radio (si cambia dalla scheda Radio delle impostazioni).
    pub cat: Option<Arc<rig_control::cat::CatService>>,
    /// Salva le regole dell'automazione quando cambiano (per ritrovarle al riavvio).
    /// Calcola le statistiche del log (premi e paesi mancanti) a ogni richiesta.
    pub stats: Option<StatsProvider>,
    pub log4om: Option<Log4omProvider>,
    pub worked_map: Option<WorkedMapProvider>,
    pub dxcc_matrix: Option<DxccMatrixProvider>,
    pub log_station: Option<LogStationProvider>,
    pub log_changed: Option<LogChangedHook>,
    /// «Salva con nome» del programma Windows, per salvare il log ADIF dove si vuole (solo dal PC stesso).
    pub save_dialog: Option<SaveFileDialog>,
    /// «Apri la cartella dei crash» (solo nel programma Windows).
    pub crash_folder: Option<OpenCrashFolder>,
    /// «Prepara il pacchetto per l'assistenza» (solo dal PC stesso).
    pub support_bundle: Option<SupportBundle>,
    /// L'ultimo pacchetto creato, per «Apri la cartella».
    pub(crate) last_bundle: Arc<Mutex<Option<std::path::PathBuf>>>,
    /// Vero mentre la finestra «Salva con nome» e' aperta: un secondo clic non ne apre un'altra.
    pub(crate) save_open: Arc<std::sync::atomic::AtomicBool>,
    /// La lista dei desiderati, con i suoi avvisi.
    pub wishlist: Option<Arc<wishlist::WishlistService>>,
    /// Il livello dell'ingresso audio (volume di registrazione di Windows, AUTO).
    pub input_level: Option<Arc<input_level::InputLevelService>>,
    /// PSK Reporter e propagazione, a richiesta.
    pub online: Option<Arc<online_info::OnlineService>>,
    /// Lo storico dei messaggi ricevuti.
    pub history: Option<Arc<decode_history::HistoryStore>>,
    /// Il controllo delle nuove versioni di Aurora.
    pub updates: Option<Arc<updater::UpdateService>>,
    /// Chi sa installare un aggiornamento (il programma Windows); senza, l'aggiornamento si scarica a mano.
    pub update_installer: Option<Arc<dyn UpdateInstaller>>,
    pub engine_config: Option<JtdxConfigProvider>,
    pub engine_apply: Option<JtdxApplier>,
    /// Impostazioni di Aurora per il motore proprio (al posto del `JTDX.ini`); solo con `AURORA_ENGINE=native`.
    pub engine_settings: Option<Arc<engine_settings::EngineSettingsStore>>,
    /// Chi apre e chiude i dispositivi audio (scheda Audio); solo con il motore proprio.
    pub audio: Option<Arc<dyn audio_api::AudioControl>>,
    /// Client NTP: controllo dell'orologio del PC.
    pub clock: Option<Arc<clock_sync::ClockService>>,
    /// Client DX Cluster: spot dei cluster telnet.
    pub cluster: Option<Arc<dx_cluster::ClusterService>>,
    pub dxcc: Option<DxccProvider>,
    /// Preferenze dell'interfaccia salvate sul server.
    pub ui_settings: Option<Arc<ui_settings::UiSettings>>,
    pub rules_saver: Option<RulesSaver>,
    /// Frequenze di lavoro per banda e modo (si cambiano dalle impostazioni).
    pub frequencies: Arc<Mutex<FrequencyTable>>,
    /// Programmazione delle bande per orario.
    pub schedule: Arc<Mutex<Schedule>>,
    /// Chiamato dopo ogni modifica di frequenze o programmazione, per salvarle.
    pub settings_saver: Option<Arc<dyn Fn() + Send + Sync>>,
    /// Accesso senza codice dalla rete di casa (mai da internet, tunnel o proxy). Di fabbrica spento.
    pub open_access: Arc<std::sync::atomic::AtomicBool>,
    /// Dove si ricorda la scelta dell'accesso senza codice (`accesso-libero.txt` nella cartella dati), se c'e'.
    pub open_access_file: Option<std::path::PathBuf>,
    /// Il programma Windows entra senza codice: vale solo per chi arriva da questo PC (loopback, mai da un proxy). I telefoni chiedono sempre il codice.
    pub local_trust: bool,
    /// Stato del motore proprio (decodifica in ombra e confronto con JTDX), se e' acceso: lo mette chi lo avvia.
    pub native_status: Arc<std::sync::RwLock<Option<NativeStatusFn>>>,
    /// Porta e nome mDNS sulla rete di casa (per il QR e la scheda Accesso).
    pub lan: Arc<lan::LanInfo>,
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
                                Event::HighlightCallsign { ref call, ref background, ref foreground, .. } => {
                                    if background.is_none() && foreground.is_none() {
                                        s.highlights.remove(call);
                                    } else {
                                        s.highlights.insert(call.clone(), event.clone());
                                        while s.highlights.len() > MAX_HIGHLIGHTS {
                                            s.highlights.pop_first();
                                        }
                                    }
                                }
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
        AppState { bus, auth, logbook, policy, secure_cookie, automation: None, station_info: Arc::new(serde_json::json!({})), rig: None, cat: None, stats: None, log4om: None, worked_map: None, dxcc_matrix: None, log_station: None, log_changed: None, save_dialog: None, crash_folder: None, support_bundle: None, last_bundle: Arc::default(), save_open: Arc::default(), wishlist: None, input_level: None, history: None, online: None, updates: None, update_installer: None, engine_config: None, engine_apply: None, engine_settings: None, audio: None, clock: None, cluster: None, dxcc: None, ui_settings: None, rules_saver: None, frequencies: Arc::default(), schedule: Arc::default(), settings_saver: None, open_access: Arc::default(), open_access_file: None, local_trust: false, native_status: Arc::default(), lan: Arc::default(), shared, shutdown: Arc::new(shutdown) }
    }

    /// Registra la funzione che racconta lo stato del motore proprio (per `GET /api/v1/engine/native`).
    pub fn set_native_status(&self, status: NativeStatusFn) {
        if let Ok(mut slot) = self.native_status.write() {
            *slot = Some(status);
        }
    }

    /// Chi apre Aurora dallo stesso PC entra senza codice di associazione (programma Windows).
    pub fn with_local_trust(mut self, trust: bool) -> AppState {
        self.local_trust = trust;
        self
    }

    /// Parte con l'accesso senza codice acceso o spento (scelta dell'operatore all'avvio).
    pub fn with_open_access(self, open: bool) -> AppState {
        self.open_access.store(open, std::sync::atomic::Ordering::SeqCst);
        self
    }

    /// La scelta dell'accesso senza codice si salva in questo file e si rilegge all'avvio.
    pub fn with_open_access_file(mut self, path: std::path::PathBuf) -> AppState {
        self.open_access_file = Some(path);
        self
    }

    pub fn with_stats_if(self, stats: Option<StatsProvider>) -> AppState {
        match stats {
            Some(s) => self.with_stats(s),
            None => self,
        }
    }

    pub fn with_clock(mut self, clock: Arc<clock_sync::ClockService>) -> AppState {
        self.clock = Some(clock);
        self
    }

    pub fn with_dxcc_if(mut self, lookup: Option<DxccProvider>) -> AppState {
        self.dxcc = lookup;
        self
    }

    pub fn with_updates(mut self, updates: Arc<updater::UpdateService>) -> AppState {
        self.updates = Some(updates);
        self
    }

    pub fn with_input_level(mut self, service: Arc<input_level::InputLevelService>) -> AppState {
        self.input_level = Some(service);
        self
    }

    pub fn with_input_level_if(mut self, service: Option<Arc<input_level::InputLevelService>>) -> AppState {
        self.input_level = service;
        self
    }

    pub fn with_update_installer(mut self, installer: Arc<dyn UpdateInstaller>) -> AppState {
        self.update_installer = Some(installer);
        self
    }

    pub fn with_update_installer_if(mut self, installer: Option<Arc<dyn UpdateInstaller>>) -> AppState {
        self.update_installer = installer;
        self
    }

    pub fn with_online(mut self, service: Arc<online_info::OnlineService>) -> AppState {
        self.online = Some(service);
        self
    }

    pub fn with_history_if(mut self, store: Option<Arc<decode_history::HistoryStore>>) -> AppState {
        self.history = store;
        self
    }

    pub fn with_history(self, store: Arc<decode_history::HistoryStore>) -> AppState {
        self.with_history_if(Some(store))
    }

    pub fn with_wishlist(mut self, wishlist: Arc<wishlist::WishlistService>) -> AppState {
        self.wishlist = Some(wishlist);
        self
    }

    pub fn with_cluster(mut self, cluster: Arc<dx_cluster::ClusterService>) -> AppState {
        self.cluster = Some(cluster);
        self
    }

    pub fn with_ui_settings(mut self, store: Arc<ui_settings::UiSettings>) -> AppState {
        self.ui_settings = Some(store);
        self
    }

    pub fn with_engine_apply_if(mut self, applier: Option<JtdxApplier>) -> AppState {
        self.engine_apply = applier;
        self
    }

    pub fn with_engine_settings(mut self, store: Arc<engine_settings::EngineSettingsStore>) -> AppState {
        self.engine_settings = Some(store);
        self
    }

    pub fn with_audio(mut self, control: Arc<dyn audio_api::AudioControl>) -> AppState {
        self.audio = Some(control);
        self
    }

    pub fn with_engine_config_if(mut self, reader: Option<JtdxConfigProvider>) -> AppState {
        self.engine_config = reader;
        self
    }

    pub fn with_worked_map_if(mut self, reader: Option<WorkedMapProvider>) -> AppState {
        self.worked_map = reader;
        self
    }

    pub fn with_log4om_if(mut self, reader: Option<Log4omProvider>) -> AppState {
        self.log4om = reader;
        self
    }

    pub fn with_dxcc_matrix_if(mut self, provider: Option<DxccMatrixProvider>) -> AppState {
        self.dxcc_matrix = provider;
        self
    }

    pub fn with_log_station(mut self, provider: LogStationProvider) -> AppState {
        self.log_station = Some(provider);
        self
    }

    pub fn with_log_changed(mut self, hook: LogChangedHook) -> AppState {
        self.log_changed = Some(hook);
        self
    }

    pub fn with_save_dialog_if(mut self, dialog: Option<SaveFileDialog>) -> AppState {
        self.save_dialog = dialog;
        self
    }

    pub fn with_crash_folder_if(mut self, open: Option<OpenCrashFolder>) -> AppState {
        self.crash_folder = open;
        self
    }

    pub fn with_support_bundle_if(mut self, bundle: Option<SupportBundle>) -> AppState {
        self.support_bundle = bundle;
        self
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

    /// La regione IARU delle impostazioni Generali (`ALL` senza impostazioni del motore, cioe' con JTDX): decide quali righe della tabella delle frequenze valgono.
    pub fn frequency_region(&self) -> rig_control::table::Region {
        self.engine_settings.as_ref().map_or_else(Default::default, |s| s.region())
    }

    pub fn with_rig(mut self, rig: Arc<dyn Rig>) -> AppState {
        self.rig = Some(rig);
        self
    }

    /// Il servizio CAT. Se la radio e' collegata dalla scheda Radio dopo l'avvio, i comandi (cambio banda...) la usano comunque.
    pub fn with_cat(mut self, cat: Arc<rig_control::cat::CatService>) -> AppState {
        self.cat = Some(cat);
        self
    }

    /// La radio da comandare adesso: quella scelta all'avvio oppure, se nessuna, il CAT quando ha una radio collegata.
    pub fn active_rig(&self) -> Option<Arc<dyn Rig>> {
        if let Some(rig) = &self.rig {
            return Some(Arc::clone(rig));
        }
        self.cat.as_ref().filter(|c| c.is_usable()).map(|c| Arc::clone(c) as Arc<dyn Rig>)
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

    /// Se e' gia' arrivato uno stato (da JTDX o dalla lettura iniziale della sua finestra).
    pub fn has_status(&self) -> bool {
        self.current_status().is_some()
    }

    pub fn current_status(&self) -> Option<StationStatus> {
        self.shared.lock().ok().and_then(|s| s.status.clone())
    }

    /// Le evidenziazioni dei nominativi in vigore (eventi `HighlightCallsign`), per chi apre la pagina dopo.
    pub fn current_highlights(&self) -> Vec<Event> {
        self.shared.lock().map(|s| s.highlights.values().cloned().collect()).unwrap_or_default()
    }
}

pub fn build_router(state: AppState) -> Router {
    routes::router(state)
}
