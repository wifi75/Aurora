//! Il servizio CAT di Aurora: tiene la configurazione e i profili (`radio.json`), collega e scollega la radio e fa da `Rig` verso il resto del
//! programma. Il backend e' Hamlib (tramite `rigctld`), OmniRig, TCI (le SDR, su WebSocket), FT-991(A) Link (UDP) oppure Cat4OM (WebSocket, radio divisa con Log4OM). La connessione non parte mai da sola, salvo `auto_connect` o la scelta
//! di avvio (`AURORA_RIG`): caricare un profilo non collega nulla (il profilo cancellava in silenzio le scelte fatte dopo, in Decodium).

use std::{
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

use serde::Serialize;

use crate::{
    cat4om::Cat4omRig,
    cat_config::{friendly_error, Backend, RadioFile, RadioStore, RigConfig, RigProfile, StoreError},
    hamlib::{list_models, locate_rigctld, rigctl_beside, Rigctld, RigModel},
    hamlib_rig::{HamlibRig, Link, LinkState, RigStatus, Timing},
    ft991link::Ft991Rig,
    tci::{RxAudioFn, TciRig, TxAudioFn},
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
            // «Accendi»: una volta per avvio di Aurora, prima di Hamlib, il comando di accensione direttamente sulla porta
            static WOKE: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
            if config.power_on && !WOKE.swap(true, std::sync::atomic::Ordering::SeqCst) {
                crate::wake::wake(config);
            }
            Rigctld::start(&rigctld).map(Link::from_daemon)
        });
        let models: ModelsFn = Arc::new(|| {
            let rigctld = locate_rigctld().ok_or_else(|| RigError("rigctld non trovato".into()))?;
            // senza rigctl (installazione interrotta) l'elenco lo da' anche rigctld con lo stesso `-l`: la radio non "sparisce" dalla pagina
            match rigctl_beside(&rigctld) {
                Some(rigctl) => list_models(&rigctl).or_else(|_| list_models(&rigctld)),
                None => list_models(&rigctld),
            }
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
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct CatStatus {
    pub backend: Backend,
    pub state: LinkState,
    pub message: String,
    /// `rigctld` trovato: senza, Hamlib non puo' collegare.
    pub rigctld_found: bool,
    /// OmniRig installato su questo PC (solo Windows): senza, la scelta OmniRig non puo' funzionare.
    pub omnirig_found: bool,
    /// Lettura dettagliata della radio (Hamlib): frequenza, modo, PTT, split, guasti.
    pub rig: Option<RigStatus>,
    /// Dove collegare gli altri programmi (indirizzo e porta di `rigctld`), se la condivisione e' accesa e la radio e' collegata con Hamlib.
    pub share: Option<ShareInfo>,
    /// Con Cat4OM: gestione, porta Control scoperta (o scritta), gruppo, radio e ruolo.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cat4om: Option<crate::cat4om::Cat4omEndpoint>,
}

/// L'indirizzo a cui un altro programma si collega a `rigctld` (modello «Hamlib NET rigctl»).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ShareInfo {
    pub host: String,
    pub port: u16,
}

/// Tutto per la schermata: stato, configurazione e profili.
#[derive(Debug, Clone, Serialize)]
pub struct CatSnapshot {
    pub status: CatStatus,
    pub config: RigConfig,
    pub profiles: Vec<RigProfile>,
    pub active_profile: Option<String>,
}

/// Lo Split DX acceso con un doppio clic su uno spot del cluster («UP 5», «QSX 14.030»...): la ricezione resta sulla frequenza dello spot
/// e la trasmissione va sull'altro VFO. Indipendente dallo split di FT8 (XIT) scelto nelle Impostazioni.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct DxSplit {
    pub rx_hz: u64,
    pub tx_hz: u64,
}

/// Distanza massima fra ricezione e trasmissione dello Split DX: oltre non e' uno split di pile-up ma un errore (o un'altra banda).
pub const DX_SPLIT_MAX_HZ: u64 = 500_000;

pub struct CatService {
    store: RadioStore,
    parts: CatParts,
    current: Mutex<Option<Arc<HamlibRig>>>,
    omni: Arc<Mutex<(LinkState, String)>>,
    /// Ricontrollo dello stato di OmniRig (come il timer «offline» di `OmniRigTransceiver.cpp`): l'ultimo e se ce n'e' uno in corso.
    omni_check: Arc<Mutex<(Option<std::time::Instant>, bool)>>,
    tci: Mutex<Option<Arc<TciRig>>>,
    /// La FT-991/FT-991A collegata attraverso FT-991(A) Link (UDP).
    ft991: Mutex<Option<Arc<Ft991Rig>>>,
    /// La radio comandata attraverso Cat4OM (Control Port del gruppo, insieme a Log4OM).
    cat4om: Mutex<Option<Arc<Cat4omRig>>>,
    /// L'audio TCI chiesto dalla scheda Audio (ricezione, trasmissione): si da' a ogni radio TCI collegata, anche dopo un cambio.
    tci_hooks: Mutex<(Option<RxAudioFn>, Option<TxAudioFn>)>,
    models: Mutex<Option<Vec<RigModel>>>,
    /// Lo Split DX acceso adesso (vedi [`DxSplit`]).
    dx_split: Mutex<Option<DxSplit>>,
}

/// Lo split che vale per OmniRig: quello scelto se il sistema di controllo e' OmniRig, altrimenti nessuno (cosi' il distacco di un altro CAT
/// non tocca OmniRig).
fn omnirig_split(config: &RigConfig) -> crate::cat_config::SplitMode {
    if config.backend == Backend::Omnirig { config.split } else { crate::cat_config::SplitMode::None }
}

impl CatService {
    /// Apre il servizio con il file `radio.json` indicato (se manca si parte dai valori di base).
    pub fn open(path: &Path) -> Arc<CatService> {
        crate::extras::set_power_file(path);
        CatService::with_parts(path, CatParts::system())
    }

    pub fn with_parts(path: &Path, parts: CatParts) -> Arc<CatService> {
        let store = RadioStore::open(path);
        crate::omnirig::set_rig(store.config().omnirig_rig);
        crate::omnirig::set_split_mode(omnirig_split(&store.config()));
        Arc::new(CatService {
            store,
            parts,
            current: Mutex::new(None),
            omni: Arc::new(Mutex::new((LinkState::Disconnected, String::new()))),
            omni_check: Arc::new(Mutex::new((None, false))),
            tci: Mutex::new(None),
            ft991: Mutex::new(None),
            cat4om: Mutex::new(None),
            tci_hooks: Mutex::new((None, None)),
            models: Mutex::new(None),
            dx_split: Mutex::new(None),
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
        let omnirig_found = crate::omnirig::installed();
        match config.backend {
            Backend::Hamlib => match self.current() {
                Some(rig) => {
                    let rig_status = rig.status();
                    let share = (config.share_enabled && rig_status.state == LinkState::Connected).then(|| ShareInfo { host: "127.0.0.1".into(), port: config.share_port });
                    CatStatus { backend: Backend::Hamlib, state: rig_status.state, message: rig_status.message.clone(), rigctld_found, omnirig_found, rig: Some(rig_status), share, cat4om: None }
                }
                None => CatStatus { backend: Backend::Hamlib, state: LinkState::Disconnected, message: String::new(), rigctld_found, omnirig_found, rig: None, share: None, cat4om: None },
            },
            Backend::Omnirig => {
                self.recheck_omni();
                let (state, message) = self.omni.lock().map(|o| o.clone()).unwrap_or((LinkState::Disconnected, String::new()));
                CatStatus { backend: Backend::Omnirig, state, message, rigctld_found, omnirig_found, rig: None, share: None, cat4om: None }
            }
            Backend::Tci => match self.tci_rig() {
                Some(rig) => {
                    let rig_status = rig.status();
                    CatStatus { backend: Backend::Tci, state: rig_status.state, message: rig_status.message.clone(), rigctld_found, omnirig_found, rig: Some(rig_status), share: None, cat4om: None }
                }
                None => CatStatus { backend: Backend::Tci, state: LinkState::Disconnected, message: String::new(), rigctld_found, omnirig_found, rig: None, share: None, cat4om: None },
            },
            Backend::Ft991link => match self.ft991_rig() {
                Some(rig) => {
                    let rig_status = rig.status();
                    CatStatus { backend: Backend::Ft991link, state: rig_status.state, message: rig_status.message.clone(), rigctld_found, omnirig_found, rig: Some(rig_status), share: None, cat4om: None }
                }
                None => CatStatus { backend: Backend::Ft991link, state: LinkState::Disconnected, message: String::new(), rigctld_found, omnirig_found, rig: None, share: None, cat4om: None },
            },
            Backend::Cat4om => match self.cat4om_rig() {
                Some(rig) => {
                    let rig_status = rig.status();
                    CatStatus { backend: Backend::Cat4om, state: rig_status.state, message: rig_status.message.clone(), rigctld_found, omnirig_found, rig: Some(rig_status), share: None, cat4om: Some(rig.endpoint()) }
                }
                None => CatStatus { backend: Backend::Cat4om, state: LinkState::Disconnected, message: String::new(), rigctld_found, omnirig_found, rig: None, share: None, cat4om: None },
            },
            Backend::None => CatStatus { backend: Backend::None, state: LinkState::Disconnected, message: String::new(), rigctld_found, omnirig_found, rig: None, share: None, cat4om: None },
        }
    }

    pub fn snapshot(&self) -> CatSnapshot {
        let file: RadioFile = self.store.snapshot();
        CatSnapshot { status: self.status(), config: file.config, profiles: file.profiles, active_profile: file.active_profile }
    }

    /// `true` se c'e' qualcosa da riavviare quando cambia la configurazione (collegata, in collegamento o in errore).
    fn is_active(&self) -> bool {
        self.current().is_some() || self.tci_rig().is_some() || self.ft991_rig().is_some() || self.cat4om_rig().is_some() || self.omni.lock().is_ok_and(|o| o.0 != LinkState::Disconnected)
    }

    /// Collega la radio con la configurazione salvata. Per Hamlib ritorna subito (il collegamento prosegue in un thread e lo stato lo racconta);
    /// per OmniRig controlla che risponda (puo' volerci qualche secondo).
    pub fn connect(&self) -> Result<CatStatus, CatError> {
        let config = self.store.config();
        // ricollegando OmniRig l'oggetto aperto si tiene (si toglie solo lo split messo da Aurora): lasciarlo e ricrearlo subito chiuderebbe e
        // riaprirebbe OmniRig con la sua finestra
        self.detach(config.backend != Backend::Omnirig);
        crate::omnirig::set_split_mode(omnirig_split(&config));
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
                self.set_omni(LinkState::Connecting, "Collegamento a OmniRig");
                match WindowsRig.frequency() {
                    Ok(_) => {
                        self.set_omni(LinkState::Connected, "");
                        // l'oggetto del PTT pronto prima della prima trasmissione (il PTT non aspetta mai una creazione lunga)
                        crate::omnirig::warm_up_ptt_in_background();
                    }
                    Err(e) => self.set_omni(LinkState::Error, &friendly_error(&e.0)),
                }
            }
            Backend::Tci => {
                let rig = Arc::new(TciRig::from_config(&config));
                // i flussi audio scelti nella scheda Audio valgono anche per la radio appena collegata
                let (rx, tx) = self.tci_hooks.lock().map(|h| h.clone()).unwrap_or_default();
                rig.set_tx_audio(tx);
                rig.set_rx_audio(rx);
                if let Ok(mut slot) = self.tci.lock() {
                    *slot = Some(rig);
                }
            }
            Backend::Ft991link => {
                // il collegamento prosegue nel suo thread: lo stato dice se FT-991(A) Link risponde
                let rig = Arc::new(Ft991Rig::from_config(&config));
                if let Ok(mut slot) = self.ft991.lock() {
                    *slot = Some(rig);
                }
            }
            Backend::Cat4om => {
                // il collegamento prosegue nel suo thread (con ricollegamento): lo stato dice se Cat4OM risponde e se la radio e' collegata
                let rig = Arc::new(Cat4omRig::from_config(&config));
                if let Ok(mut slot) = self.cat4om.lock() {
                    *slot = Some(rig);
                }
            }
            Backend::None => {}
        }
        Ok(self.status())
    }

    /// Con OmniRig scelto e collegato (o in errore), al massimo ogni 10 s rilegge la radio in un thread a parte: se si spegne l'interfaccia lo dice, se
    /// torna si ricollega da sola. Prima lo stato si fissava al collegamento e restava «collegata» anche a radio spenta.
    fn recheck_omni(&self) {
        let state = self.omni.lock().map(|o| o.0).unwrap_or(LinkState::Disconnected);
        if state == LinkState::Disconnected || state == LinkState::Connecting {
            return;
        }
        {
            let Ok(mut c) = self.omni_check.lock() else { return };
            if c.1 || c.0.is_some_and(|t| t.elapsed() < std::time::Duration::from_secs(10)) {
                return;
            }
            *c = (Some(std::time::Instant::now()), true);
        }
        let (omni, check) = (Arc::clone(&self.omni), Arc::clone(&self.omni_check));
        let _ = std::thread::Builder::new().name("aurora-omnirig-check".into()).spawn(move || {
            let result = WindowsRig.frequency();
            if let Ok(mut o) = omni.lock()
                && o.0 != LinkState::Disconnected
            {
                *o = match result {
                    Ok(_) => (LinkState::Connected, String::new()),
                    Err(e) => (LinkState::Error, friendly_error(&e.0)),
                };
            }
            if let Ok(mut c) = check.lock() {
                c.1 = false;
            }
        });
    }

    fn set_omni(&self, state: LinkState, message: &str) {
        if let Ok(mut o) = self.omni.lock() {
            *o = (state, message.to_string());
        }
    }

    /// Scollega: il PTT si spegne, lo split vero si toglie e `rigctld` si ferma (la porta seriale si libera).
    pub fn disconnect(&self) -> CatStatus {
        self.detach(true);
        self.status()
    }

    /// Stacca tutte le radio; `release_omnirig` falso tiene aperto l'oggetto OmniRig (togliendo solo lo split messo da Aurora).
    /// Lo Split DX lo spegne ogni radio alla chiusura (lo split acceso da Aurora si toglie).
    fn detach(&self, release_omnirig: bool) {
        self.forget_dx_split();
        let rig = self.current.lock().ok().and_then(|mut c| c.take());
        if let Some(rig) = rig {
            rig.shutdown();
        }
        let tci = self.tci.lock().ok().and_then(|mut c| c.take());
        if let Some(tci) = tci {
            tci.shutdown();
        }
        let ft991 = self.ft991.lock().ok().and_then(|mut c| c.take());
        if let Some(ft991) = ft991 {
            ft991.shutdown();
        }
        let cat4om = self.cat4om.lock().ok().and_then(|mut c| c.take());
        if let Some(cat4om) = cat4om {
            cat4om.shutdown();
        }
        // l'oggetto OmniRig tenuto aperto si lascia: OmniRig puo' chiudersi e liberare la porta (passaggio a Hamlib, distacco)
        if release_omnirig {
            crate::omnirig::release();
        } else {
            crate::omnirig::reset_split();
        }
        self.set_omni(LinkState::Disconnected, "");
    }

    /// Salva la configurazione e la applica: se la radio era collegata (o in errore) la ricollega con i valori nuovi.
    pub fn apply_config(&self, config: RigConfig) -> Result<CatStatus, CatError> {
        let (rig, split) = (config.omnirig_rig, omnirig_split(&config));
        self.store.set_config(config)?;
        crate::omnirig::set_rig(rig);
        crate::omnirig::set_split_mode(split);
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

    fn tci_rig(&self) -> Option<Arc<TciRig>> {
        self.tci.lock().ok().and_then(|c| c.clone())
    }

    /// La radio TCI collegata (PTT, ganci dell'audio).
    pub fn tci(&self) -> Result<Arc<TciRig>, RigError> {
        self.tci_rig().ok_or_else(|| RigError("nessuna radio TCI collegata: scegli TCI in Impostazioni → Radio e premi Collega".into()))
    }

    fn ft991_rig(&self) -> Option<Arc<Ft991Rig>> {
        self.ft991.lock().ok().and_then(|c| c.clone())
    }

    /// La FT-991/FT-991A collegata con FT-991(A) Link (PTT, split simulato).
    pub fn ft991(&self) -> Result<Arc<Ft991Rig>, RigError> {
        self.ft991_rig().ok_or_else(|| RigError("nessuna radio FT-991(A) Link collegata: scegli FT-991(A) Link in Impostazioni → Radio e premi Collega".into()))
    }

    fn cat4om_rig(&self) -> Option<Arc<Cat4omRig>> {
        self.cat4om.lock().ok().and_then(|c| c.clone())
    }

    /// La radio collegata attraverso Cat4OM (PTT, split).
    pub fn cat4om(&self) -> Result<Arc<Cat4omRig>, RigError> {
        self.cat4om_rig().ok_or_else(|| RigError("nessuna radio Cat4OM collegata: scegli Cat4OM in Impostazioni → Radio e premi Collega".into()))
    }

    /// L'audio ricevuto via TCI va a `hook` (`None` = flusso fermato), anche per le radio TCI collegate dopo.
    pub fn set_tci_rx_audio(&self, hook: Option<RxAudioFn>) {
        if let Ok(mut h) = self.tci_hooks.lock() {
            h.0 = hook.clone();
        }
        if let Some(rig) = self.tci_rig() {
            rig.set_rx_audio(hook);
        }
    }

    /// L'audio da trasmettere via TCI viene da `hook` (`None` = la SDR usa la sua sorgente audio).
    pub fn set_tci_tx_audio(&self, hook: Option<TxAudioFn>) {
        if let Ok(mut h) = self.tci_hooks.lock() {
            h.1 = hook.clone();
        }
        if let Some(rig) = self.tci_rig() {
            rig.set_tx_audio(hook);
        }
    }

    /// Toglie l'audio TCI di ricezione solo se e' ancora `hook` (un ingresso nuovo puo' averlo gia' sostituito).
    pub fn release_tci_rx_audio(&self, hook: &RxAudioFn) {
        let mine = self.tci_hooks.lock().is_ok_and(|h| h.0.as_ref().is_some_and(|c| Arc::ptr_eq(c, hook)));
        if mine {
            self.set_tci_rx_audio(None);
        }
    }

    /// Come [`CatService::release_tci_rx_audio`] per l'audio da trasmettere.
    pub fn release_tci_tx_audio(&self, hook: &TxAudioFn) {
        let mine = self.tci_hooks.lock().is_ok_and(|h| h.1.as_ref().is_some_and(|c| Arc::ptr_eq(c, hook)));
        if mine {
            self.set_tci_tx_audio(None);
        }
    }

    /// La portante di accordo della SDR TCI e' accesa (TUNE): il suo spegnimento si accetta anche se la SDR dice di trasmettere.
    pub fn tci_tuning(&self) -> bool {
        self.backend() == Backend::Tci && self.tci_rig().is_some_and(|r| r.tuning())
    }

    /// La radio sta trasmettendo (Hamlib, TCI, FT-991(A) Link o Cat4OM).
    pub fn transmitting(&self) -> bool {
        self.current().is_some_and(|r| r.transmitting())
            || self.tci_rig().is_some_and(|r| r.transmitting())
            || self.ft991_rig().is_some_and(|r| r.transmitting())
            || self.cat4om_rig().is_some_and(|r| r.transmitting())
    }

    /// `true` se c'e' una radio che risponde ai comandi (Hamlib collegata, oppure OmniRig scelto).
    pub fn is_usable(&self) -> bool {
        match self.backend() {
            Backend::Hamlib => self.current().is_some_and(|r| r.status().state == LinkState::Connected),
            Backend::Omnirig => self.omni.lock().is_ok_and(|o| o.0 == LinkState::Connected),
            Backend::Tci => self.tci_rig().is_some_and(|r| r.status().state == LinkState::Connected),
            Backend::Ft991link => self.ft991_rig().is_some_and(|r| r.status().state == LinkState::Connected),
            Backend::Cat4om => self.cat4om_rig().is_some_and(|r| r.status().state == LinkState::Connected),
            Backend::None => false,
        }
    }

    /// Chiusura del programma: il PTT si spegne, con «Spegni» la radio si spegne, e la radio si libera.
    pub fn shutdown(&self) {
        if let Some(rig) = self.current() {
            rig.power_off_if_chosen();
        }
        if let Some(rig) = self.tci_rig() {
            rig.power_off_if_chosen();
        }
        if let Some(rig) = self.ft991_rig() {
            rig.power_off_if_chosen();
        }
        if let Some(rig) = self.cat4om_rig() {
            rig.power_off_if_chosen();
        }
        self.disconnect();
    }
}

impl Drop for CatService {
    fn drop(&mut self) {
        self.shutdown();
    }
}

/// Lo Split DX del cluster.
impl CatService {
    /// Lo Split DX acceso, se c'e'.
    pub fn dx_split(&self) -> Option<DxSplit> {
        self.dx_split.lock().ok().and_then(|d| *d)
    }

    fn forget_dx_split(&self) {
        if let Ok(mut d) = self.dx_split.lock() {
            *d = None;
        }
    }

    /// Accende lo Split DX: ricezione (VFO A) su `rx_hz`, trasmissione (VFO B) su `tx_hz`, split della radio acceso. Le due frequenze stanno
    /// nella stessa banda dei radioamatori e a non piu' di [`DX_SPLIT_MAX_HZ`]. Mai in trasmissione. FT-991(A) Link comanda solo il VFO A e
    /// non lo sa fare; con Cat4OM serve una radio che dichiari lo split.
    pub fn set_dx_split(&self, rx_hz: u64, tx_hz: u64) -> Result<DxSplit, RigError> {
        let band = crate::bands::amateur_band(rx_hz);
        if band.is_none() || band != crate::bands::amateur_band(tx_hz) {
            return Err(RigError(format!("split fuori dalla banda: ricezione {rx_hz} Hz, trasmissione {tx_hz} Hz")));
        }
        if rx_hz == tx_hz || rx_hz.abs_diff(tx_hz) > DX_SPLIT_MAX_HZ {
            return Err(RigError(format!("split non valido: ricezione {rx_hz} Hz, trasmissione {tx_hz} Hz")));
        }
        if self.transmitting() {
            return Err(RigError("la radio sta trasmettendo: split rifiutato".into()));
        }
        let done = match self.backend() {
            Backend::Hamlib => self.hamlib()?.set_dx_split(rx_hz, tx_hz),
            Backend::Tci => self.tci()?.set_dx_split(rx_hz, tx_hz),
            Backend::Cat4om => self.cat4om()?.set_dx_split(rx_hz, tx_hz),
            Backend::Omnirig => crate::omnirig::set_dx_split(rx_hz, tx_hz),
            Backend::Ft991link => Err(RigError("FT-991(A) Link comanda solo il VFO A: lo split DX non e' supportato".into())),
            Backend::None => Err(RigError("nessuna radio: sistema di controllo «Nessuno»".into())),
        };
        // la ricezione puo' essere gia' cambiata anche se lo split non si e' acceso: lo stato dice solo cio' che e' vero
        let state = done.is_ok().then_some(DxSplit { rx_hz, tx_hz });
        if let Ok(mut d) = self.dx_split.lock() {
            *d = state;
        }
        done.map(|()| DxSplit { rx_hz, tx_hz })
    }

    /// Spegne lo Split DX (pulsante «Split DX: spegni», ritorno a un modo del motore, cambio di frequenza). `Ok(false)` se non era acceso.
    pub fn clear_dx_split(&self) -> Result<bool, RigError> {
        if self.dx_split().is_none() {
            return Ok(false);
        }
        let done = match self.backend() {
            Backend::Hamlib => self.hamlib().and_then(|r| r.clear_dx_split()),
            Backend::Tci => self.tci().and_then(|r| r.clear_dx_split()),
            Backend::Cat4om => self.cat4om().and_then(|r| r.clear_dx_split()),
            Backend::Omnirig => crate::omnirig::clear_dx_split(),
            Backend::Ft991link | Backend::None => Ok(()),
        };
        done?;
        self.forget_dx_split();
        Ok(true)
    }
}

/// Le chiamate del programma alla radio passano dal backend scelto. Con OmniRig tutto va a OmniRig (anche i comandi del motore JTDX);
/// con Hamlib solo frequenza e lettura, dalla radio collegata.
impl CatService {
    /// Manda alla radio una stringa CAT cosi' com'e', se il sistema di controllo e' Hamlib. Con gli altri («TCI», FT-991 diretto, OmniRig) non c'e' un modo
    /// uguale per tutti e si risponde che non e' supportato.
    pub fn send_raw_cat(&self, text: &str) -> Result<(), RigError> {
        match self.backend() {
            Backend::Hamlib => self.hamlib()?.send_raw_cat(text),
            Backend::None => Err(RigError("nessuna radio: sistema di controllo «Nessuno»".into())),
            _ => Err(RigError("i comandi CAT diretti funzionano solo con Hamlib".into())),
        }
    }
}

impl Rig for CatService {
    /// Cambio di frequenza; lo Split DX acceso finisce qui (spot senza «UP», cambio di banda, frequenza scritta a mano).
    fn set_frequency(&self, hz: u64) -> Result<(), RigError> {
        if let Err(e) = self.clear_dx_split() {
            // la radio resta in split solo se non riesce a spegnerlo: lo stato di Aurora non lo dice piu' acceso (Hamlib e OmniRig lo spengono
            // comunque con il cambio di frequenza)
            tracing::warn!("split DX non spento prima del cambio di frequenza: {e}");
            self.forget_dx_split();
        }
        match self.backend() {
            Backend::Hamlib => self.hamlib()?.set_frequency(hz),
            Backend::Tci => self.tci()?.set_frequency(hz),
            Backend::Ft991link => self.ft991()?.set_frequency(hz),
            Backend::Cat4om => self.cat4om()?.set_frequency(hz),
            Backend::Omnirig => WindowsRig.set_frequency(hz),
            Backend::None => Err(RigError("nessuna radio: sistema di controllo «Nessuno»".into())),
        }
    }

    fn frequency(&self) -> Result<u64, RigError> {
        match self.backend() {
            Backend::Hamlib => self.hamlib()?.frequency(),
            Backend::Tci => self.tci()?.frequency(),
            Backend::Ft991link => self.ft991()?.frequency(),
            Backend::Cat4om => self.cat4om()?.frequency(),
            Backend::Omnirig => WindowsRig.frequency(),
            Backend::None => Err(RigError("nessuna radio: sistema di controllo «Nessuno»".into())),
        }
    }

    fn meters(&self) -> Result<crate::meters::Meters, RigError> {
        match self.backend() {
            Backend::Hamlib => self.hamlib()?.meters(),
            Backend::Tci => self.tci()?.meters(),
            Backend::Ft991link => self.ft991()?.meters(),
            Backend::Cat4om => self.cat4om()?.meters(),
            Backend::Omnirig => Err(RigError("misuratori non disponibili con OmniRig".into())),
            Backend::None => Err(RigError("nessuna radio: sistema di controllo «Nessuno»".into())),
        }
    }

    fn set_rx_frequency(&self, hz: u32) -> Result<(), RigError> {
        match self.backend() {
            Backend::Hamlib => self.hamlib()?.set_rx_frequency(hz),
            Backend::Tci => self.tci()?.set_rx_frequency(hz),
            Backend::Ft991link => self.ft991()?.set_rx_frequency(hz),
            Backend::Cat4om => self.cat4om()?.set_rx_frequency(hz),
            Backend::Omnirig => WindowsRig.set_rx_frequency(hz),
            Backend::None => Err(RigError("nessuna radio: sistema di controllo «Nessuno»".into())),
        }
    }

    fn output_attenuation(&self) -> Result<u32, RigError> {
        match self.backend() {
            Backend::Hamlib => self.hamlib()?.output_attenuation(),
            Backend::Tci => self.tci()?.output_attenuation(),
            Backend::Ft991link => self.ft991()?.output_attenuation(),
            Backend::Cat4om => self.cat4om()?.output_attenuation(),
            Backend::Omnirig => WindowsRig.output_attenuation(),
            Backend::None => Err(RigError("nessuna radio: sistema di controllo «Nessuno»".into())),
        }
    }

    fn set_output_attenuation(&self, tenths: u32) -> Result<(), RigError> {
        match self.backend() {
            Backend::Hamlib => self.hamlib()?.set_output_attenuation(tenths),
            Backend::Tci => self.tci()?.set_output_attenuation(tenths),
            Backend::Ft991link => self.ft991()?.set_output_attenuation(tenths),
            Backend::Cat4om => self.cat4om()?.set_output_attenuation(tenths),
            Backend::Omnirig => WindowsRig.set_output_attenuation(tenths),
            Backend::None => Err(RigError("nessuna radio: sistema di controllo «Nessuno»".into())),
        }
    }

    fn set_mode(&self, mode: &str) -> Result<(), RigError> {
        match self.backend() {
            Backend::Hamlib => self.hamlib()?.set_mode(mode),
            Backend::Tci => self.tci()?.set_mode(mode),
            Backend::Ft991link => self.ft991()?.set_mode(mode),
            Backend::Cat4om => self.cat4om()?.set_mode(mode),
            Backend::Omnirig => WindowsRig.set_mode(mode),
            Backend::None => Err(RigError("nessuna radio: sistema di controllo «Nessuno»".into())),
        }
    }

    fn set_extra(&self, item: crate::extras::ExtraItem) -> Result<(), RigError> {
        match self.backend() {
            Backend::Hamlib => self.hamlib()?.set_extra(item),
            // PWR − / + e TUNE dei display: `drive` e `tune` della SDR
            Backend::Tci => self.tci()?.set_extra(item),
            Backend::Ft991link => Err(RigError("comando non supportato da FT-991(A) Link".into())),
            Backend::Cat4om => Err(RigError("comando non supportato da Cat4OM".into())),
            Backend::Omnirig => WindowsRig.set_extra(item),
            Backend::None => Err(RigError("nessuna radio: sistema di controllo «Nessuno»".into())),
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
