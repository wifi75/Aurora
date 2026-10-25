//! Il servizio CAT di Aurora: tiene la configurazione e i profili (`radio.json`), collega e scollega la radio e fa da `Rig` verso il resto del
//! programma. Il backend e' Hamlib (tramite `rigctld`) oppure OmniRig. La connessione non parte mai da sola, salvo `auto_connect` o la scelta
//! di avvio (`AURORA_RIG`): caricare un profilo non collega nulla (il profilo cancellava in silenzio le scelte fatte dopo, in Decodium).

use std::{
    path::{Path, PathBuf},
    sync::{Arc, Mutex, Once},
};

use serde::Serialize;

use crate::{
    cat_config::{friendly_error, Backend, RadioFile, RadioStore, RigConfig, RigProfile, StoreError},
    hamlib::{list_models, locate_rigctld, rigctl_beside, Rigctld, RigModel},
    hamlib_rig::{HamlibRig, Link, LinkState, RigStatus, Timing},
    Rig, RigError, WindowsRig,
};

/// Apre la connessione verso la radio per una configurazione (nel sistema vero lancia `rigctld`).
pub type LinkFactory = Arc<dyn Fn(&RigConfig) -> Result<Link, RigError> + Send + Sync>;
type ModelsFn = Arc<dyn Fn() -> Result<Vec<RigModel>, RigError> + Send + Sync>;
type Locator = Arc<dyn Fn() -> Option<PathBuf> + Send + Sync>;
type TimingFn = Arc<dyn Fn(&RigConfig) -> Timing + Send + Sync>;

/// Le parti sostituibili del servizio: nel programma sono quelle di sistema, nelle prove sono finte.
#[derive(Clone)]
pub struct CatParts {
    factory: LinkFactory,
    locate: Locator,
    models: ModelsFn,
    timing: TimingFn,
}

impl CatParts {
    /// `rigctld` vero, cercato come in `hamlib::locate_rigctld`.
    pub fn system() -> CatParts {
        let factory: LinkFactory = Arc::new(|config: &RigConfig| {
            let exe = locate_rigctld().ok_or_else(|| RigError("rigctld non trovato".into()))?;
            let rigctld = config.to_rigctld(&exe)?;
            Rigctld::start(&rigctld).map(Link::from_daemon)
        });
        let models: ModelsFn = Arc::new(|| {
            let rigctld = locate_rigctld().ok_or_else(|| RigError("rigctld non trovato".into()))?;
            let rigctl = rigctl_beside(&rigctld).ok_or_else(|| RigError("rigctl non trovato accanto a rigctld".into()))?;
            list_models(&rigctl)
        });
        CatParts { factory, locate: Arc::new(locate_rigctld), models, timing: Arc::new(Timing::standard) }
    }

    /// Per le prove: il modo di aprire il collegamento, l'elenco dei modelli e le attese scelti da chi prova.
    pub fn custom(factory: LinkFactory, rigctld: Option<PathBuf>, models: Vec<RigModel>, timing: Timing) -> CatParts {
        CatParts {
            factory,
            locate: Arc::new(move || rigctld.clone()),
            models: Arc::new(move || Ok(models.clone())),
            timing: Arc::new(move |_| timing.clone()),
        }
    }
}

/// Un errore con un codice breve (per l'interfaccia) e il testo.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CatError {
    pub code: &'static str,
    pub message: String,
}

impl std::fmt::Display for CatError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.code, self.message)
    }
}

impl From<StoreError> for CatError {
    fn from(e: StoreError) -> CatError {
        let message = match &e {
            StoreError::Write(w) => w.clone(),
            other => other.code().to_string(),
        };
        CatError { code: e.code(), message }
    }
}

/// Lo stato del collegamento, per l'interfaccia.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CatStatus {
    pub backend: Backend,
    pub state: LinkState,
    pub message: String,
    /// `rigctld` trovato: senza, Hamlib non puo' collegare.
    pub rigctld_found: bool,
    /// Lettura dettagliata della radio (Hamlib): frequenza, modo, PTT, split, guasti.
    pub rig: Option<RigStatus>,
}

/// Tutto per la schermata: stato, configurazione e profili.
#[derive(Debug, Clone, Serialize)]
pub struct CatSnapshot {
    pub status: CatStatus,
    pub config: RigConfig,
    pub profiles: Vec<RigProfile>,
    pub active_profile: Option<String>,
}

pub struct CatService {
    store: RadioStore,
    parts: CatParts,
    current: Mutex<Option<Arc<HamlibRig>>>,
    omni: Mutex<(LinkState, String)>,
    models: Mutex<Option<Vec<RigModel>>>,
}

static OMNIRIG_WARM: Once = Once::new();

impl CatService {
    /// Apre il servizio con il file `radio.json` indicato (se manca si parte dai valori di base).
    pub fn open(path: &Path) -> Arc<CatService> {
        CatService::with_parts(path, CatParts::system())
    }

    pub fn with_parts(path: &Path, parts: CatParts) -> Arc<CatService> {
        Arc::new(CatService {
            store: RadioStore::open(path),
            parts,
            current: Mutex::new(None),
            omni: Mutex::new((LinkState::Disconnected, String::new())),
            models: Mutex::new(None),
        })
    }

    /// Prende il backend dalla scelta di avvio (`AURORA_RIG`) senza salvarla.
    pub fn adopt_backend(&self, backend: Backend) {
        self.store.adopt_backend(backend);
    }

    pub fn config(&self) -> RigConfig {
        self.store.config()
    }

    pub fn backend(&self) -> Backend {
        self.store.config().backend
    }

    fn current(&self) -> Option<Arc<HamlibRig>> {
        self.current.lock().ok().and_then(|c| c.clone())
    }

    pub fn status(&self) -> CatStatus {
        let config = self.store.config();
        let rigctld_found = (self.parts.locate)().is_some();
        match config.backend {
            Backend::Hamlib => match self.current() {
                Some(rig) => {
                    let rig_status = rig.status();
                    CatStatus { backend: Backend::Hamlib, state: rig_status.state, message: rig_status.message.clone(), rigctld_found, rig: Some(rig_status) }
                }
                None => CatStatus { backend: Backend::Hamlib, state: LinkState::Disconnected, message: String::new(), rigctld_found, rig: None },
            },
            Backend::Omnirig => {
                let (state, message) = self.omni.lock().map(|o| o.clone()).unwrap_or((LinkState::Disconnected, String::new()));
                CatStatus { backend: Backend::Omnirig, state, message, rigctld_found, rig: None }
            }
        }
    }

    pub fn snapshot(&self) -> CatSnapshot {
        let file: RadioFile = self.store.snapshot();
        CatSnapshot { status: self.status(), config: file.config, profiles: file.profiles, active_profile: file.active_profile }
    }

    /// `true` se c'e' qualcosa da riavviare quando cambia la configurazione (collegata, in collegamento o in errore).
    fn is_active(&self) -> bool {
        self.current().is_some() || self.omni.lock().is_ok_and(|o| o.0 != LinkState::Disconnected)
    }

    /// Collega la radio con la configurazione salvata. Per Hamlib ritorna subito (il collegamento prosegue in un thread e lo stato lo racconta);
    /// per OmniRig controlla che risponda (puo' volerci qualche secondo).
    pub fn connect(&self) -> Result<CatStatus, CatError> {
        let config = self.store.config();
        self.disconnect();
        match config.backend {
            Backend::Hamlib => {
                let Some(exe) = (self.parts.locate)() else {
                    return Err(CatError { code: "rigctld_missing", message: friendly_error("rigctld non trovato") });
                };
                // si controlla subito che i valori bastino a lanciare rigctld (modello scelto, porta valida)
                config.to_rigctld(&exe).map_err(|e| CatError { code: "bad_config", message: e.0 })?;
                let factory = Arc::clone(&self.parts.factory);
                let cfg = config.clone();
                let connector: Arc<dyn Fn() -> Result<Link, RigError> + Send + Sync> = Arc::new(move || factory(&cfg));
                let timing = (self.parts.timing)(&config);
                let rig = Arc::new(HamlibRig::start(config, connector, timing));
                if let Ok(mut slot) = self.current.lock() {
                    *slot = Some(rig);
                }
            }
            Backend::Omnirig => {
                OMNIRIG_WARM.call_once(WindowsRig::keep_warm);
                self.set_omni(LinkState::Connecting, "Collegamento a OmniRig");
                match WindowsRig.frequency() {
                    Ok(_) => self.set_omni(LinkState::Connected, ""),
                    Err(e) => self.set_omni(LinkState::Error, &friendly_error(&e.0)),
                }
            }
        }
        Ok(self.status())
    }

    fn set_omni(&self, state: LinkState, message: &str) {
        if let Ok(mut o) = self.omni.lock() {
            *o = (state, message.to_string());
        }
    }

    /// Scollega: il PTT si spegne, lo split vero si toglie e `rigctld` si ferma (la porta seriale si libera).
    pub fn disconnect(&self) -> CatStatus {
        let rig = self.current.lock().ok().and_then(|mut c| c.take());
        if let Some(rig) = rig {
            rig.shutdown();
        }
        self.set_omni(LinkState::Disconnected, "");
        self.status()
    }

    /// Salva la configurazione e la applica: se la radio era collegata (o in errore) la ricollega con i valori nuovi.
    pub fn apply_config(&self, config: RigConfig) -> Result<CatStatus, CatError> {
        self.store.set_config(config)?;
        if self.is_active() { self.connect() } else { Ok(self.status()) }
    }

    pub fn save_profile(&self, name: &str) -> Result<CatSnapshot, CatError> {
        self.store.save_profile(name)?;
        Ok(self.snapshot())
    }

    /// Carica un profilo: scollega tutto e non collega nulla (si collega a mano).
    pub fn load_profile(&self, name: &str) -> Result<CatSnapshot, CatError> {
        self.store.load_profile(name)?;
        self.disconnect();
        Ok(self.snapshot())
    }

    pub fn new_profile(&self, name: Option<&str>) -> Result<CatSnapshot, CatError> {
        self.store.new_profile(name)?;
        self.disconnect();
        Ok(self.snapshot())
    }

    pub fn delete_profile(&self, name: &str) -> Result<CatSnapshot, CatError> {
        self.store.delete_profile(name)?;
        Ok(self.snapshot())
    }

    /// I modelli di Hamlib (si leggono una volta da `rigctl -l`).
    pub fn models(&self) -> Result<Vec<RigModel>, CatError> {
        if let Some(cached) = self.models.lock().ok().and_then(|m| m.clone()) {
            return Ok(cached);
        }
        let list = (self.parts.models)().map_err(|e| CatError { code: "rigctld_missing", message: friendly_error(&e.0) })?;
        if let Ok(mut slot) = self.models.lock() {
            *slot = Some(list.clone());
        }
        Ok(list)
    }

    /// Avvio del programma: collega se la scelta salvata lo chiede (`auto_connect`) o se l'ha imposto `AURORA_RIG` (`forced`).
    pub fn start_up(&self, forced: bool) -> Option<CatStatus> {
        if forced || self.store.config().auto_connect {
            return self.connect().map_err(|e| tracing::warn!("CAT non collegato all'avvio: {e}")).ok();
        }
        None
    }

    /// La radio Hamlib collegata, per le funzioni che servono la radio vera (PTT, modo, split).
    pub fn hamlib(&self) -> Result<Arc<HamlibRig>, RigError> {
        self.current().ok_or_else(|| RigError("nessuna radio collegata: scegli il CAT in Impostazioni → Radio e premi Collega".into()))
    }

    /// `true` se c'e' una radio che risponde ai comandi (Hamlib collegata, oppure OmniRig scelto).
    pub fn is_usable(&self) -> bool {
        match self.backend() {
            Backend::Hamlib => self.current().is_some_and(|r| r.status().state == LinkState::Connected),
            Backend::Omnirig => self.omni.lock().is_ok_and(|o| o.0 == LinkState::Connected),
        }
    }

    /// Chiusura del programma: il PTT si spegne e la radio si libera.
    pub fn shutdown(&self) {
        self.disconnect();
    }
}

impl Drop for CatService {
    fn drop(&mut self) {
        self.shutdown();
    }
}

/// Le chiamate del programma alla radio passano dal backend scelto. Con OmniRig tutto va a OmniRig (anche i comandi del motore JTDX);
/// con Hamlib solo frequenza e lettura, dalla radio collegata.
impl Rig for CatService {
    fn set_frequency(&self, hz: u64) -> Result<(), RigError> {
        match self.backend() {
            Backend::Hamlib => self.hamlib()?.set_frequency(hz),
            Backend::Omnirig => WindowsRig.set_frequency(hz),
        }
    }

    fn frequency(&self) -> Result<u64, RigError> {
        match self.backend() {
            Backend::Hamlib => self.hamlib()?.frequency(),
            Backend::Omnirig => WindowsRig.frequency(),
        }
    }

    fn meters(&self) -> Result<crate::meters::Meters, RigError> {
        match self.backend() {
            Backend::Hamlib => self.hamlib()?.meters(),
            Backend::Omnirig => Err(RigError("misuratori non disponibili con OmniRig".into())),
        }
    }

    fn set_rx_frequency(&self, hz: u32) -> Result<(), RigError> {
        match self.backend() {
            Backend::Hamlib => self.hamlib()?.set_rx_frequency(hz),
            Backend::Omnirig => WindowsRig.set_rx_frequency(hz),
        }
    }

    fn output_attenuation(&self) -> Result<u32, RigError> {
        match self.backend() {
            Backend::Hamlib => self.hamlib()?.output_attenuation(),
            Backend::Omnirig => WindowsRig.output_attenuation(),
        }
    }

    fn set_output_attenuation(&self, tenths: u32) -> Result<(), RigError> {
        match self.backend() {
            Backend::Hamlib => self.hamlib()?.set_output_attenuation(tenths),
            Backend::Omnirig => WindowsRig.set_output_attenuation(tenths),
        }
    }

    fn set_mode(&self, mode: &str) -> Result<(), RigError> {
        match self.backend() {
            Backend::Hamlib => self.hamlib()?.set_mode(mode),
            Backend::Omnirig => WindowsRig.set_mode(mode),
        }
    }

    fn controls(&self) -> Result<Vec<(String, bool)>, RigError> {
        match self.backend() {
            Backend::Hamlib => Err(RigError("comando non supportato da questa radio".into())),
            Backend::Omnirig => WindowsRig.controls(),
        }
    }

    fn controls_and_tx(&self) -> Result<crate::controls::Reading, RigError> {
        match self.backend() {
            Backend::Hamlib => Err(RigError("comando non supportato da questa radio".into())),
            Backend::Omnirig => WindowsRig.controls_and_tx(),
        }
    }

    fn engine_snapshot(&self) -> Result<crate::snapshot::Snapshot, RigError> {
        match self.backend() {
            Backend::Hamlib => Err(RigError("comando non supportato da questa radio".into())),
            Backend::Omnirig => WindowsRig.engine_snapshot(),
        }
    }

    fn select_tx_message(&self, n: u8) -> Result<(), RigError> {
        match self.backend() {
            Backend::Hamlib => Err(RigError("comando non supportato da questa radio".into())),
            Backend::Omnirig => WindowsRig.select_tx_message(n),
        }
    }

    fn set_control(&self, id: &str, on: Option<bool>) -> Result<(), RigError> {
        match self.backend() {
            Backend::Hamlib => Err(RigError("comando non supportato da questa radio".into())),
            Backend::Omnirig => WindowsRig.set_control(id, on),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{sync::atomic::{AtomicU32, Ordering}, time::Duration};

    use super::*;
    use crate::{cat_config::Handshake, fake_rigctld::FakeRigctld};

    fn timing() -> Timing {
        Timing {
            poll: Duration::from_millis(30),
            tx_poll_min: Duration::from_millis(10),
            quick_reads: [Duration::from_millis(10), Duration::from_millis(20)],
            startup_block: Duration::from_millis(100),
            ptt_pause: Duration::from_millis(5),
            qsy_check: Duration::from_millis(20),
            reconnect_step: Duration::from_millis(40),
            reconnect_cap: Duration::from_millis(200),
            max_reconnects: 3,
            tolerate: 3,
            tolerate_io: 8,
        }
    }

    fn models() -> Vec<RigModel> {
        vec![RigModel { id: 1029, manufacturer: "Yaesu".into(), model: "FT-2000".into(), status: "Stable".into() }]
    }

    fn path(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("aurora-cat-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        dir.join("radio.json")
    }

    struct Rig4Test {
        service: Arc<CatService>,
        fake: Arc<FakeRigctld>,
        opened: Arc<AtomicU32>,
        seen_baud: Arc<Mutex<Vec<u32>>>,
    }

    fn service(name: &str, rigctld: Option<PathBuf>) -> Rig4Test {
        let fake = Arc::new(FakeRigctld::start(14_074_000));
        let opened = Arc::new(AtomicU32::new(0));
        let seen_baud = Arc::new(Mutex::new(Vec::new()));
        let (f, o, b) = (Arc::clone(&fake), Arc::clone(&opened), Arc::clone(&seen_baud));
        let factory: LinkFactory = Arc::new(move |config: &RigConfig| {
            o.fetch_add(1, Ordering::SeqCst);
            if let Ok(mut v) = b.lock() {
                v.push(config.baud);
            }
            f.client(Duration::from_millis(500)).map(Link::from_client)
        });
        let service = CatService::with_parts(&path(name), CatParts::custom(factory, rigctld, models(), timing()));
        Rig4Test { service, fake, opened, seen_baud }
    }

    fn ft2000() -> RigConfig {
        RigConfig { model: 1029, rig_name: "Yaesu FT-2000".into(), port: "COM3".into(), baud: 38400, data_bits: 8, stop_bits: 2, handshake: Handshake::Hardware, ..RigConfig::default() }
    }

    fn wait_state(service: &CatService, state: LinkState) -> CatStatus {
        let until = std::time::Instant::now() + Duration::from_secs(5);
        loop {
            let s = service.status();
            if s.state == state || std::time::Instant::now() >= until {
                return s;
            }
            std::thread::sleep(Duration::from_millis(10));
        }
    }

    #[test]
    fn it_starts_disconnected_and_never_connects_by_itself() {
        let t = service("idle", Some("rigctld.exe".into()));
        let s = t.service.status();
        assert_eq!((s.state, s.backend, s.rigctld_found), (LinkState::Disconnected, Backend::Hamlib, true));
        assert_eq!(t.opened.load(Ordering::SeqCst), 0);
        assert!(t.service.start_up(false).is_none(), "senza auto_connect all'avvio non si collega");
        assert_eq!(t.opened.load(Ordering::SeqCst), 0);
        assert!(t.service.frequency().is_err());
        assert!(!t.service.is_usable());
    }

    #[test]
    fn connect_needs_a_model_and_rigctld() {
        let t = service("needs", Some("rigctld.exe".into()));
        let e = t.service.connect().expect_err("senza modello");
        assert_eq!(e.code, "bad_config");
        let t = service("missing", None);
        t.service.apply_config(ft2000()).expect("salva");
        let e = t.service.connect().expect_err("manca rigctld");
        assert_eq!(e.code, "rigctld_missing");
        assert!(e.message.contains("AURORA_RIGCTLD"));
        assert!(!t.service.status().rigctld_found);
        assert!(t.service.models().is_ok(), "con le parti finte l'elenco c'e'");
    }

    #[test]
    fn it_connects_reads_commands_and_disconnects_with_the_ptt_off() {
        let t = service("flow", Some("rigctld.exe".into()));
        t.service.apply_config(ft2000()).expect("salva");
        assert_eq!(t.service.status().state, LinkState::Disconnected, "salvare non collega");
        let s = t.service.connect().expect("collega");
        assert!(matches!(s.state, LinkState::Connecting | LinkState::Connected));
        let s = wait_state(&t.service, LinkState::Connected);
        assert_eq!(s.rig.as_ref().and_then(|r| r.frequency_hz), Some(14_074_000));
        assert!(t.service.is_usable());
        assert_eq!(t.service.frequency().expect("frequenza"), 14_074_000);
        t.service.set_frequency(7_074_000).expect("QSY");
        assert_eq!(t.fake.frequency(), 7_074_000);
        // le funzioni del motore JTDX non esistono con Hamlib
        assert!(t.service.set_rx_frequency(1500).is_err() && t.service.controls().is_err());
        std::thread::sleep(Duration::from_millis(150));
        t.service.hamlib().expect("radio").set_ptt(true).expect("PTT");
        assert!(t.fake.ptt());
        let s = t.service.disconnect();
        assert_eq!(s.state, LinkState::Disconnected);
        assert!(!t.fake.ptt(), "scollegando il PTT si spegne");
        assert!(t.service.frequency().is_err());
    }

    #[test]
    fn changing_the_config_reconnects_only_if_it_was_active() {
        let t = service("apply", Some("rigctld.exe".into()));
        t.service.apply_config(ft2000()).expect("salva");
        assert_eq!(t.opened.load(Ordering::SeqCst), 0, "radio non collegata: salvare non la collega");
        t.service.connect().expect("collega");
        wait_state(&t.service, LinkState::Connected);
        t.service.apply_config(RigConfig { baud: 19200, ..ft2000() }).expect("applica");
        wait_state(&t.service, LinkState::Connected);
        assert_eq!(t.opened.load(Ordering::SeqCst), 2);
        assert_eq!(*t.seen_baud.lock().expect("baud"), vec![38400, 19200]);
        let e = t.service.apply_config(RigConfig { baud: 5, ..ft2000() }).expect_err("baud non valido");
        assert_eq!(e.code, "bad_baud");
        assert_eq!(t.service.config().baud, 19200, "un valore non valido non si salva");
    }

    #[test]
    fn loading_a_profile_disconnects_and_does_not_connect() {
        let t = service("profile", Some("rigctld.exe".into()));
        t.service.apply_config(ft2000()).expect("salva");
        t.service.save_profile("FT-2000 casa").expect("profilo");
        t.service.connect().expect("collega");
        wait_state(&t.service, LinkState::Connected);
        let snap = t.service.load_profile("FT-2000 casa").expect("carica");
        assert_eq!(snap.status.state, LinkState::Disconnected);
        assert_eq!(snap.active_profile.as_deref(), Some("FT-2000 casa"));
        assert_eq!(t.opened.load(Ordering::SeqCst), 1, "il profilo caricato non ricollega");
        assert!(!t.fake.ptt());
        let snap = t.service.new_profile(None).expect("nuovo");
        assert_eq!(snap.profiles.len(), 2);
        assert_eq!(snap.config.model, 0);
        let snap = t.service.delete_profile("FT-2000 casa").expect("elimina");
        assert_eq!(snap.profiles.len(), 1);
        assert_eq!(t.service.load_profile("non c'e'").expect_err("manca").code, "no_such_profile");
    }

    #[test]
    fn a_failing_radio_leaves_a_readable_error_and_connect_tries_again() {
        let t = service("error", Some("rigctld.exe".into()));
        t.service.apply_config(ft2000()).expect("salva");
        t.fake.fail_with(-5);
        t.service.connect().expect("parte");
        let s = wait_state(&t.service, LinkState::Error);
        assert!(s.message.contains("non risponde"), "{}", s.message);
        t.fake.fail_with(0);
        t.service.connect().expect("riprova");
        assert_eq!(wait_state(&t.service, LinkState::Connected).state, LinkState::Connected);
    }

    #[test]
    fn auto_connect_and_the_forced_start_connect_at_startup() {
        let t = service("auto", Some("rigctld.exe".into()));
        t.service.apply_config(RigConfig { auto_connect: true, ..ft2000() }).expect("salva");
        assert!(t.service.start_up(false).is_some());
        assert_eq!(wait_state(&t.service, LinkState::Connected).state, LinkState::Connected);
        let t = service("forced", Some("rigctld.exe".into()));
        t.service.apply_config(ft2000()).expect("salva");
        assert!(t.service.start_up(true).is_some());
        assert_eq!(wait_state(&t.service, LinkState::Connected).state, LinkState::Connected);
    }

    #[test]
    fn the_service_snapshot_carries_state_config_and_profiles() {
        let t = service("snap", Some("rigctld.exe".into()));
        t.service.apply_config(ft2000()).expect("salva");
        t.service.save_profile("uno").expect("profilo");
        let snap = t.service.snapshot();
        assert_eq!((snap.config.model, snap.profiles.len()), (1029, 1));
        let json = serde_json::to_value(&snap).expect("json");
        assert_eq!(json["status"]["state"], "disconnected");
        assert_eq!(json["config"]["backend"], "hamlib");
        assert_eq!(json["config"]["handshake"], "hardware");
    }
}
