//! Gli invii verso l'esterno della scheda Segnalazioni: spotting su PSK Reporter, spot su DXSummit, spot WSPR/FST4W su WSPRnet (modulo
//! [`wsprnet`], facoltativo e spento di fabbrica), QSO al server TCP e a eQSL, invio diretto dei QSO a Wavelog/Cloudlog, ClubLog, QRZ.com
//! Logbook e LoTW (modulo [`upload`], con coda e nuovi tentativi).
//!
//! Regole comuni (decise per tutti gli invii verso internet):
//! - ognuno si accende e si spegne dalle impostazioni (`Config`), a caldo; spento non parte niente e non si accoda niente;
//! - **non blocca mai il motore**: l'ascolto del bus passa lo spot o il QSO a un'attivita' a parte (con i suoi tempi massimi) e riprende subito;
//! - l'esito dell'ultimo invio sta in [`ReportStatus`] (e nel registro, senza mai password);
//! - le password e le chiavi (eQSL, Wavelog/Cloudlog, ClubLog, QRZ) si leggono dalla protezione dei segreti solo nel momento dell'invio.

pub mod clublog;
pub mod dxsummit;
pub mod eqsl;
pub mod hrdlog;
pub mod lotw;
pub mod psk;
pub mod qrz;
pub mod qrz_xml;
pub mod secret;
pub mod status;
pub mod tcp;
pub mod upload;
pub mod wavelog;
pub mod wsprnet;

use std::{
    collections::VecDeque,
    net::{IpAddr, SocketAddr},
    path::PathBuf,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

use aurora_core::{Bus, DecodedMessage, Event, QsoRecord};
use logbook::adif::{self, Station};
use secret::SecretStore;
use status::ReportStatus;
use tokio::{net::UdpSocket, sync::{broadcast::error::RecvError, watch}};

/// Il nome con cui si salva la password di eQSL nei segreti.
pub const EQSL_SECRET: &str = "eqsl";
/// La chiave API di Wavelog/Cloudlog.
pub const WAVELOG_SECRET: &str = "wavelog";
/// La password (o «Application Password») di ClubLog.
pub const CLUBLOG_SECRET: &str = "clublog";
/// La chiave API dell'applicazione per ClubLog.
pub const CLUBLOG_API_SECRET: &str = "clublog-api";
/// La chiave del logbook di QRZ.com.
pub const QRZ_SECRET: &str = "qrz";
/// Il codice di caricamento («upload code») di HRDLOG.NET.
pub const HRDLOG_SECRET: &str = "hrdlog";

/// HRDLOG.NET: nominativo del log (vuoto = il proprio); il codice di caricamento sta nei segreti.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct HrdlogAccount {
    pub callsign: String,
}

/// Wavelog o Cloudlog: indirizzo e numero della stazione (la chiave API sta nei segreti).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct WavelogAccount {
    pub url: String,
    pub station_id: String,
}

/// ClubLog: email dell'account e nominativo del log (vuoto = il proprio); password e chiave API stanno nei segreti.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ClublogAccount {
    pub email: String,
    pub callsign: String,
}

/// LoTW: percorso di TQSL (vuoto = cercato da solo) e posizione di stazione di TQSL.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct LotwSetup {
    pub tqsl: String,
    pub location: String,
}

/// L'account di eQSL (la password sta nei segreti).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct EqslAccount {
    pub user: String,
    pub nick: String,
}

/// Cosa e' acceso adesso e con quali dati.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Config {
    pub call: String,
    pub grid: String,
    /// Potenza scritta nei record (`TX_PWR`) quando il QSO non ne ha una.
    pub tx_power: String,
    /// Antenna scritta nel pacchetto di PSK Reporter (vuota se non si sa).
    pub antenna: String,
    pub psk: bool,
    pub dxsummit: bool,
    /// Spot WSPR e FST4W a WSPRnet («Upload spots» di WSJT-X, spento di fabbrica).
    pub wsprnet: bool,
    /// Server TCP a cui mandare ogni QSO (`EnableTCPConnection` acceso e indirizzo valido).
    pub tcp: Option<SocketAddr>,
    /// eQSL acceso con questo account.
    pub eqsl: Option<EqslAccount>,
    /// Invio diretto dei QSO: ognuno `Some` (o vero) solo se acceso.
    pub wavelog: Option<WavelogAccount>,
    pub clublog: Option<ClublogAccount>,
    pub qrz: bool,
    /// Il nominativo registrato su QRZ (vuoto = quello del QSO): QRZ scarta i QSO con `STATION_CALLSIGN` diverso dal logbook.
    pub qrz_call: String,
    pub lotw: Option<LotwSetup>,
    pub hrdlog: Option<HrdlogAccount>,
}

impl Config {
    /// C'e' almeno un destinatario dei QSO registrati.
    fn wants_qsos(&self) -> bool {
        self.tcp.is_some() || self.eqsl.is_some() || upload::Target::ALL.into_iter().any(|t| upload::enabled(self, t))
    }
}

/// Gli indirizzi a cui si manda: in produzione quelli veri, nelle prove un server locale.
#[derive(Clone, Debug)]
pub struct Endpoints {
    /// `host:porta` di PSK Reporter.
    pub psk: String,
    pub psk_every: Duration,
    pub eqsl_url: String,
    pub dxsummit_url: String,
    /// Attesa massima delle richieste HTTP (eQSL 20 s, DXSummit 10 s come JTDX).
    pub eqsl_timeout: Duration,
    pub dxsummit_timeout: Duration,
    /// WSPRnet: indirizzo, attesa massima di una richiesta e ritardo massimo (a caso) dopo la decodifica del periodo.
    pub wsprnet_url: String,
    pub wsprnet_timeout: Duration,
    pub wsprnet_delay: Duration,
    /// Il programma che dichiara l'invio a PSK Reporter.
    pub software: String,
    /// Invio diretto: indirizzi fissi di ClubLog e QRZ, attese massime, il programma dichiarato (User-Agent).
    pub clublog_url: String,
    pub qrz_url: String,
    pub hrdlog_url: String,
    pub upload_timeout: Duration,
    pub lotw_timeout: Duration,
    pub user_agent: String,
    /// Il file della coda dell'invio diretto; `None` = solo in memoria.
    pub queue_file: Option<PathBuf>,
    /// Dove cercare TQSL e il suo elenco delle posizioni di stazione.
    pub tqsl_candidates: Vec<PathBuf>,
    pub tqsl_station_data: Option<PathBuf>,
}

impl Default for Endpoints {
    fn default() -> Endpoints {
        Endpoints {
            psk: format!("{}:{}", psk::HOST, psk::PORT),
            psk_every: Duration::from_secs(psk::SEND_EVERY_SECS),
            eqsl_url: eqsl::URL.to_string(),
            dxsummit_url: dxsummit::URL.to_string(),
            eqsl_timeout: eqsl::TIMEOUT,
            dxsummit_timeout: dxsummit::TIMEOUT,
            wsprnet_url: wsprnet::URL.to_string(),
            wsprnet_timeout: wsprnet::TIMEOUT,
            wsprnet_delay: wsprnet::MAX_DELAY,
            software: format!("Aurora v{}", aurora_core::version::APP_VERSION),
            clublog_url: clublog::URL.to_string(),
            qrz_url: qrz::URL.to_string(),
            hrdlog_url: hrdlog::URL.to_string(),
            upload_timeout: upload::HTTP_TIMEOUT,
            lotw_timeout: lotw::TIMEOUT,
            user_agent: format!("Aurora/{}", aurora_core::version::APP_VERSION),
            queue_file: None,
            tqsl_candidates: lotw::default_candidates(),
            tqsl_station_data: lotw::default_station_data(),
        }
    }
}

/// Il servizio in funzione.
pub struct Service {
    config: watch::Sender<Config>,
    status: Arc<ReportStatus>,
    secrets: Arc<dyn SecretStore>,
    endpoints: Endpoints,
    now: Arc<dyn Fn() -> u64 + Send + Sync>,
    last_spot: Mutex<Option<(String, Instant)>>,
    /// Quello che il motore dice adesso (frequenza, modo, rapporto): serve a comporre lo spot come lo comporrebbe JTDX.
    live: Mutex<Live>,
    /// La coda dell'invio diretto dei QSO.
    queue: Arc<upload::Queue>,
    /// Le richieste di mandare subito gli spot PSK Reporter in coda (alla chiusura): il ciclo li manda e risponde.
    flush: tokio::sync::mpsc::Sender<tokio::sync::oneshot::Sender<()>>,
    /// La coda delle richieste a WSPRnet e la sveglia del suo invio.
    wsprnet: Mutex<wsprnet::Queue>,
    wsprnet_wake: tokio::sync::Notify,
}

/// Lo stato del motore che serve allo spot.
#[derive(Clone, Debug, Default)]
struct Live {
    dial_hz: u64,
    rx_df: Option<u32>,
    mode: String,
    report: String,
}

/// Perche' uno spot non e' partito (o non e' riuscito).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SpotError {
    /// Lo spotting su DXSummit e' spento nelle impostazioni.
    Off,
    NoOwnCall,
    BadDxCall,
    /// La stessa stazione e' stata segnalata da poco.
    Already,
    /// Non si conosce ancora la frequenza della radio.
    NoFrequency,
    /// La richiesta e' partita ma e' fallita (rete, tempo scaduto, risposta d'errore).
    Failed(String),
}

impl SpotError {
    /// Il codice per l'API.
    pub fn code(&self) -> &'static str {
        match self {
            SpotError::Off => "spot_off",
            SpotError::NoOwnCall => "spot_no_call",
            SpotError::BadDxCall => "spot_bad_call",
            SpotError::Already => "spot_already",
            SpotError::NoFrequency => "spot_no_frequency",
            SpotError::Failed(_) => "spot_failed",
        }
    }
}

impl std::fmt::Display for SpotError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SpotError::Off => f.write_str("lo spotting su DXSummit e' spento"),
            SpotError::NoOwnCall => f.write_str("manca il tuo nominativo"),
            SpotError::BadDxCall => f.write_str("nominativo da segnalare non valido"),
            SpotError::Already => f.write_str("questa stazione e' gia' stata segnalata da poco"),
            SpotError::NoFrequency => f.write_str("la frequenza della radio non e' ancora nota"),
            SpotError::Failed(e) => f.write_str(e),
        }
    }
}

/// Cosa serve per segnalare la stazione scelta: il resto (frequenza, modo, rapporto) lo sa il servizio.
#[derive(Clone, Debug, Default)]
pub struct SpotParams {
    pub dx_call: String,
    pub dx_grid: String,
    /// Il testo dello spot (con le macro `#G`, `#D`, `#R`), come la riga di testo di JTDX.
    pub text: String,
    /// La propagazione (`Es`, `F2`...), come il campo di JTDX.
    pub prop: String,
}

/// Lo spot partito: quello che si e' mandato.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SpotDone {
    pub dx_call: String,
    pub freq_hz: u64,
    pub info: String,
}

/// La guardia contro i doppioni dei QSO (come quella dell'esportatore): lo stesso nominativo entro due periodi non e' un altro QSO.
#[derive(Default)]
struct DupeGuard {
    last: Option<(String, u64)>,
}

impl DupeGuard {
    fn accept(&mut self, call: &str, mode: &str, stamp_ms: u64) -> bool {
        let window = if mode == "FT4" { 14_000 } else { 30_000 };
        let call = call.to_uppercase();
        if let Some((last, at)) = &self.last
            && *last == call
            && stamp_ms.abs_diff(*at) <= window
        {
            return false;
        }
        self.last = Some((call, stamp_ms));
        true
    }
}

fn random_id() -> u32 {
    use std::hash::{BuildHasher, Hasher};
    std::collections::hash_map::RandomState::new().build_hasher().finish() as u32
}

/// Un numero a caso fra 0 e 1 (ritardi e margini di WSPRnet).
fn random_unit() -> f64 {
    f64::from(random_id()) / f64::from(u32::MAX)
}

/// Il nome del modo nello spot: FT4, FST4 e FST4W (anche con il periodo, «FST4-60»: WSJT-X manda il nome senza periodo), gli altri FT8.
fn spot_mode(mode: &str) -> &'static str {
    let up = mode.trim().to_ascii_uppercase();
    if up == "FT4" {
        "FT4"
    } else if up.starts_with("FST4W") {
        "FST4W"
    } else if up.starts_with("FST4") {
        "FST4"
    } else {
        "FT8"
    }
}

/// Quanto dura un periodo, in millisecondi.
fn period_ms(mode: &str) -> u64 {
    if mode.eq_ignore_ascii_case("FT4") { 7_500 } else { 15_000 }
}

impl Service {
    /// Parte (dentro un runtime Tokio). `now`: secondi Unix.
    pub fn spawn(
        bus: &Bus,
        status: Arc<ReportStatus>,
        secrets: Arc<dyn SecretStore>,
        endpoints: Endpoints,
        now: Arc<dyn Fn() -> u64 + Send + Sync>,
        initial: Config,
    ) -> Arc<Service> {
        let (config, rx) = watch::channel(initial);
        let queue = Arc::new(upload::Queue::open(endpoints.queue_file.clone()));
        let (flush, flush_rx) = tokio::sync::mpsc::channel(4);
        let service = Arc::new(Service {
            config,
            status,
            secrets,
            endpoints,
            now,
            last_spot: Mutex::new(None),
            live: Mutex::new(Live::default()),
            queue,
            flush,
            wsprnet: Mutex::new(wsprnet::Queue::default()),
            wsprnet_wake: tokio::sync::Notify::new(),
        });
        let events = bus.subscribe_events();
        tokio::spawn(run(Arc::clone(&service), events, rx, flush_rx));
        tokio::spawn(wsprnet_worker(Arc::clone(&service)));
        for target in upload::Target::ALL {
            tokio::spawn(upload::worker(Arc::clone(&service), target));
        }
        service
    }

    /// Il pulsante «Prova» dell'invio diretto: controlla le impostazioni salvate senza registrare QSO (dove il servizio lo permette). Se va
    /// bene, un servizio fermo per un errore di credenziali riparte e riprova subito quello che e' in coda.
    pub async fn test_upload(&self, target: upload::Target) -> Result<String, String> {
        let result = upload::test(self, target).await;
        if result.is_ok() {
            self.queue.set_blocked(target, false);
            self.queue.retry_now(Some(target), (self.now)());
        }
        result
    }

    /// Alla chiusura del programma: gli spot PSK Reporter ancora in coda partono subito (WSJT-X li manda alla chiusura), aspettando al massimo
    /// `within` perche' la chiusura non si allunghi. Il bus degli eventi non si chiude mai: senza questa chiamata la coda andava persa.
    pub async fn flush_psk_now(&self, within: Duration) {
        let (reply, done) = tokio::sync::oneshot::channel();
        let asked = async {
            if self.flush.send(reply).await.is_ok() {
                let _ = done.await;
            }
        };
        if tokio::time::timeout(within, asked).await.is_err() {
            tracing::warn!("PSK Reporter: spot in coda non inviati entro {} ms dalla chiusura", within.as_millis());
        }
    }

    /// Le stazioni Wavelog/Cloudlog lette dal server con la chiave salvata (per la tendina delle Impostazioni).
    pub async fn wavelog_stations(&self, url: Option<String>) -> Result<Vec<wavelog::Station>, String> {
        upload::wavelog_stations(self, url).await
    }

    /// Le credenziali sono cambiate: i servizi fermi ripartono e quello che e' in coda si riprova subito.
    pub fn credentials_changed(&self) {
        for target in upload::Target::ALL {
            self.queue.set_blocked(target, false);
        }
        self.queue.retry_now(None, (self.now)());
    }

    /// Mette in coda per un servizio QSO gia' registrati (il Fox log della pagina Fox verso Club Log): stesso record ADIF dell'invio in tempo
    /// reale, la coda poi li manda uno alla volta con i suoi tentativi; Club Log considera riusciti i doppioni («QSO Duplicate»). Il servizio
    /// deve essere acceso nelle impostazioni. Restituisce quanti QSO sono entrati in coda.
    pub fn queue_qsos(&self, target: upload::Target, qsos: &[QsoRecord]) -> Result<usize, String> {
        let config = self.config.borrow().clone();
        if !upload::enabled(&config, target) {
            return Err(format!("{} non e' acceso nelle impostazioni", target.kind()));
        }
        let now = (self.now)();
        let today = adif::utc_date(now);
        let station = Station { call: config.call.clone(), grid: config.grid.clone(), tx_power: config.tx_power.clone() };
        for q in qsos {
            self.queue.push(target, &q.call, adif::jtdx_record(q, &station, &today), now);
        }
        Ok(qsos.len())
    }

    /// Quanti QSO aspettano di partire verso un servizio.
    pub fn pending(&self, target: upload::Target) -> usize {
        self.queue.count(target)
    }

    pub fn status(&self) -> Arc<ReportStatus> {
        Arc::clone(&self.status)
    }

    /// Cambia cosa e' acceso, a caldo.
    pub fn set_config(&self, config: Config) {
        self.config.send_if_modified(|c| {
            let changed = *c != config;
            if changed {
                *c = config;
            }
            changed
        });
    }

    fn set_live(&self, change: impl FnOnce(&mut Live)) {
        change(&mut self.live.lock().unwrap_or_else(|p| p.into_inner()));
    }

    /// Spot di una stazione su DXSummit (pulsante «Spot DX», come `on_pbSpotDXCall_clicked` di JTDX): solo con lo spotting acceso, una volta per
    /// stazione ogni 5 minuti, con un thread a parte e attesa massima di 10 s. La frequenza e' quella della radio piu' la frequenza di ricezione
    /// (dove si sente il DX), il campo `info` e' il modo con il testo (macro espanse) come lo compone JTDX. L'esito finisce anche nello stato.
    pub async fn spot_current(&self, params: SpotParams) -> Result<SpotDone, SpotError> {
        let live = self.live.lock().unwrap_or_else(|p| p.into_inner()).clone();
        let config_grid = self.config.borrow().grid.clone();
        let freq_hz = match live.rx_df {
            Some(df) if live.dial_hz > 0 => live.dial_hz + u64::from(df),
            _ => return Err(SpotError::NoFrequency),
        };
        let dx_grid = params.dx_grid.trim().to_uppercase();
        let macros = dxsummit::Macros {
            my_grid: &config_grid,
            dx_grid: &dx_grid,
            distance_km: qso_native::history::grid_distance_km(&config_grid, &dx_grid),
            report: live.report.trim(),
        };
        let mode = if live.mode.is_empty() { "FT8" } else { live.mode.as_str() };
        let info: String = dxsummit::compose_info(mode, params.text.trim(), &params.prop, macros).chars().take(200).collect();
        let dx_call = params.dx_call.trim().to_uppercase();
        self.spot_dxsummit(&dx_call, freq_hz, &info).await?;
        Ok(SpotDone { dx_call, freq_hz, info })
    }

    /// Lo spot vero e proprio, con frequenza e `info` gia' pronti (vedi [`Service::spot_current`]).
    pub async fn spot_dxsummit(&self, dx_call: &str, freq_hz: u64, info: &str) -> Result<(), SpotError> {
        let config = self.config.borrow().clone();
        if !config.dxsummit {
            return Err(SpotError::Off);
        }
        if config.call.is_empty() {
            return Err(SpotError::NoOwnCall);
        }
        let dx_call = dx_call.trim().to_uppercase();
        if dx_call.is_empty() || !dx_call.chars().all(|c| c.is_ascii_alphanumeric() || c == '/') || dx_call.len() > 20 {
            return Err(SpotError::BadDxCall);
        }
        {
            let mut last = self.last_spot.lock().unwrap_or_else(|p| p.into_inner());
            if last.as_ref().is_some_and(|(c, at)| *c == dx_call && at.elapsed() < Duration::from_secs(300)) {
                return Err(SpotError::Already);
            }
            *last = Some((dx_call.clone(), Instant::now()));
        }
        let request = dxsummit::SpotRequest { my_call: config.call, dx_call: dx_call.clone(), freq_hz, info: info.trim().chars().take(200).collect() };
        let (url, timeout) = (self.endpoints.dxsummit_url.clone(), self.endpoints.dxsummit_timeout);
        let result = tokio::task::spawn_blocking(move || dxsummit::send(&url, &request, timeout)).await.unwrap_or_else(|e| Err(format!("invio interrotto: {e}")));
        match result {
            Ok(()) => {
                tracing::info!("spot inviato a DXSummit: {dx_call} su {} kHz", dxsummit::frequency_khz(freq_hz));
                self.status.ok("dxsummit", format!("spot inviato: {dx_call} su {} kHz", dxsummit::frequency_khz(freq_hz)));
                Ok(())
            }
            Err(e) => {
                // un invio fallito non conta: si puo' riprovare subito
                *self.last_spot.lock().unwrap_or_else(|p| p.into_inner()) = None;
                tracing::warn!("spot a DXSummit non riuscito ({dx_call}): {e}");
                self.status.fail("dxsummit", e.clone());
                Err(SpotError::Failed(e))
            }
        }
    }

}

async fn run(
    service: Arc<Service>,
    mut events: tokio::sync::broadcast::Receiver<Event>,
    mut config_rx: watch::Receiver<Config>,
    mut flush_rx: tokio::sync::mpsc::Receiver<tokio::sync::oneshot::Sender<()>>,
) {
    let mut queue: VecDeque<psk::Spot> = VecDeque::new();
    let mut spot_cache: std::collections::HashMap<String, u64> = std::collections::HashMap::new();
    let mut packetizer = psk::Packetizer::new(random_id());
    let mut dial_hz: u64 = 0;
    let mut mode = "FT8".to_string();
    // `m_okToPost`: dopo un cambio di banda o di modo i messaggi del periodo in corso non si segnalano (ascolto misto)
    let mut blocked_until_ms: u64 = ((service.now)() * 1000) / period_ms(&mode) * period_ms(&mode);
    // banda e modo dei periodi prima dell'ultimo cambio: i messaggi di quei periodi che arrivano dopo il cambio si segnalano con la frequenza
    // del loro periodo (WSJT-X 3.2: «PSK Reporter [...] retain the band and frequency of each receive period»)
    let mut previous: Option<(u64, String)> = None;
    let mut guard = DupeGuard::default();
    let mut tick = tokio::time::interval(service.endpoints.psk_every);
    tick.tick().await;
    loop {
        tokio::select! {
            event = events.recv() => match event {
                Ok(Event::Status(s)) => {
                    let changed_mode = !s.mode.is_empty() && s.mode != mode;
                    if changed_mode {
                        previous = Some((dial_hz, mode.clone()));
                        mode = s.mode.clone();
                        blocked_until_ms = ((service.now)() * 1000) / period_ms(&mode) * period_ms(&mode);
                        flush_psk(&service, &mut packetizer, &mut queue).await;
                    }
                    dial_hz = s.dial_hz;
                    service.set_live(|l| {
                        l.dial_hz = s.dial_hz;
                        l.rx_df = s.rx_df;
                        if !s.mode.is_empty() {
                            l.mode.clone_from(&s.mode);
                        }
                        l.report = s.report.clone().unwrap_or_default();
                    });
                }
                Ok(Event::BandChanged { dial_hz: hz, .. }) => {
                    flush_psk(&service, &mut packetizer, &mut queue).await;
                    if hz != dial_hz {
                        previous = Some((dial_hz, mode.clone()));
                    }
                    service.set_live(|l| l.dial_hz = hz);
                    dial_hz = hz;
                    blocked_until_ms = ((service.now)() * 1000) / period_ms(&mode) * period_ms(&mode);
                }
                Ok(Event::Decode(d)) => {
                    let config = service.config.borrow().clone();
                    if config.psk && !config.call.is_empty() {
                        let period = Period { dial_hz, mode: &mode, blocked_until_ms, previous: previous.as_ref().map(|(h, m)| (*h, m.as_str())) };
                        queue_spot(&mut queue, &d, &period, (service.now)(), Some(&mut spot_cache));
                    }
                }
                Ok(Event::WsprCycle(cycle)) => queue_wsprnet(&service, &cycle),
                Ok(Event::QsoLogged(q)) => {
                    let config = service.config.borrow().clone();
                    let stamp = if q.time_off_unix_ms > 0 { q.time_off_unix_ms } else { (service.now)() * 1000 };
                    if config.wants_qsos() && guard.accept(&q.call, &q.mode, stamp) {
                        dispatch_qso(&service, config, q);
                    }
                }
                Ok(_) | Err(RecvError::Lagged(_)) => {}
                Err(RecvError::Closed) => {
                    // alla chiusura si manda quello che e' in coda (WSJT-X invia alla chiusura e al cambio di banda)
                    flush_psk(&service, &mut packetizer, &mut queue).await;
                    break;
                }
            },
            _ = tick.tick() => flush_psk(&service, &mut packetizer, &mut queue).await,
            Some(reply) = flush_rx.recv() => {
                flush_psk(&service, &mut packetizer, &mut queue).await;
                let _ = reply.send(());
            }
            changed = config_rx.changed() => {
                if changed.is_err() {
                    break;
                }
                if !service.config.borrow().psk {
                    queue.clear();
                }
                // WSPRnet spento: quello che aspettava non parte piu'
                if !service.config.borrow().wsprnet {
                    with_wsprnet(&service, wsprnet::Queue::clear);
                    service.wsprnet_wake.notify_one();
                }
            }
        }
    }
}

/// Banda e modo con cui segnalare un messaggio: quelli di adesso, e quelli di prima dell'ultimo cambio per i periodi precedenti.
struct Period<'a> {
    dial_hz: u64,
    mode: &'a str,
    /// Inizio del periodo del cambio: i messaggi di quel periodo (ascolto misto) non si segnalano (`m_okToPost`).
    blocked_until_ms: u64,
    /// Frequenza della radio e modo prima del cambio.
    previous: Option<(u64, &'a str)>,
}

/// Mette in coda lo spot di un messaggio decodificato, se ha i requisiti di `readFromStdout` (messaggio standard con nominativo e locatore validi,
/// non «ipotizzato», non del periodo del cambio di banda). Un messaggio di un periodo precedente al cambio si segnala con la banda di allora.
/// L'ora dello spot e' quella del periodo del messaggio, con la data giusta anche a cavallo della mezzanotte (`DecodedTime::spotTime` di
/// WSJT-X 3.2, `Network/DecodedTime.cpp`, GPL-3.0: un periodo che non puo' essere ancora iniziato e' di ieri).
/// Ogni quanto si puo' ridare lo stesso nominativo a PSK Reporter (`CACHE_TIMEOUT` di `PSKReporter.cpp`, WSJT-X 3.3): 5 minuti, salvo sopra 49 MHz.
const SPOT_REPEAT_SECS: u64 = 300;

/// `cache`: l'ultimo spot di ogni nominativo (`spot_cache` di WSJT-X), per non mandare lo stesso CQ a ogni periodo; `None` nelle prove che non la usano.
fn queue_spot(queue: &mut VecDeque<psk::Spot>, d: &DecodedMessage, period: &Period, now_secs: u64, cache: Option<&mut std::collections::HashMap<String, u64>>) {
    if d.hint {
        return;
    }
    let now_ms = now_secs * 1000;
    let day_start = now_ms / 86_400_000 * 86_400_000;
    let mut slot_start = day_start + u64::from(d.time_ms);
    // un minuto di tolleranza per gli orologi appena avanti; oltre, il periodo e' di ieri
    if slot_start > now_ms + 60_000 {
        slot_start = slot_start.saturating_sub(86_400_000);
    }
    let (dial_hz, mode) = if slot_start == period.blocked_until_ms {
        return;
    } else if slot_start < period.blocked_until_ms {
        match period.previous {
            Some(p) => p,
            None => return,
        }
    } else {
        (period.dial_hz, period.mode)
    };
    if dial_hz == 0 {
        return;
    }
    let Some((call, grid)) = psk::spot_call_and_grid(&d.text) else { return };
    let freq_hz = dial_hz + u64::from(d.freq_hz);
    if let Some(cache) = cache {
        if freq_hz <= 49_000_000 && cache.get(&call).is_some_and(|&t| now_secs.saturating_sub(t) <= SPOT_REPEAT_SECS) {
            return;
        }
        if cache.len() > 20_000 {
            cache.retain(|_, t| now_secs.saturating_sub(*t) <= SPOT_REPEAT_SECS);
        }
        cache.insert(call.clone(), now_secs);
    }
    if queue.len() >= psk::MAX_QUEUE {
        queue.pop_front();
    }
    queue.push_back(psk::Spot {
        call,
        grid,
        freq_hz,
        mode: spot_mode(mode).to_string(),
        snr: d.snr,
        time: u32::try_from(slot_start / 1000).unwrap_or(u32::MAX),
    });
}

/// `sendReport`: manda in pacchetti quello che e' in coda. L'indirizzo si risolve a ogni invio, con una scadenza di 5 s.
async fn flush_psk(service: &Service, packetizer: &mut psk::Packetizer, queue: &mut VecDeque<psk::Spot>) {
    if queue.is_empty() {
        return;
    }
    let config = service.config.borrow().clone();
    if !config.psk {
        queue.clear();
        return;
    }
    if config.call.is_empty() || config.grid.is_empty() {
        queue.clear();
        service.status.fail("psk", "servono il tuo nominativo e il tuo locatore (scheda Generali)");
        return;
    }
    let local = psk::Local { call: config.call.clone(), grid: config.grid.clone(), antenna: config.antenna.clone(), software: service.endpoints.software.clone() };
    let count = queue.len();
    let packets = packetizer.build(&local, queue, u32::try_from((service.now)()).unwrap_or(u32::MAX));
    match send_psk(&service.endpoints.psk, &packets).await {
        Ok(()) => service.status.ok("psk", format!("{count} spot in {} pacchetti", packets.len())),
        Err(e) => service.status.fail("psk", e),
    }
}

async fn send_psk(target: &str, packets: &[Vec<u8>]) -> Result<(), String> {
    let addr = tokio::time::timeout(Duration::from_secs(5), tokio::net::lookup_host(target))
        .await
        .map_err(|_| "indirizzo di PSK Reporter non risolto entro 5 s".to_string())?
        .map_err(|e| format!("indirizzo di PSK Reporter non risolto: {e}"))?
        .next()
        .ok_or("indirizzo di PSK Reporter non trovato")?;
    let bind = if addr.is_ipv4() { "0.0.0.0:0" } else { "[::]:0" };
    let socket = UdpSocket::bind(bind).await.map_err(|e| format!("socket UDP: {e}"))?;
    for packet in packets {
        tokio::time::timeout(Duration::from_secs(5), socket.send_to(packet, addr))
            .await
            .map_err(|_| "invio a PSK Reporter troppo lento".to_string())?
            .map_err(|e| format!("invio a PSK Reporter: {e}"))?;
    }
    Ok(())
}

/// Un periodo WSPR/FST4W decodificato: con l'invio a WSPRnet acceso (e nominativo e locatore noti) le sue richieste entrano in coda dopo un
/// ritardo a caso fino a 20 s, come `uploadTimer` di WSJT-X. Spento non si prepara niente.
fn queue_wsprnet(service: &Arc<Service>, cycle: &aurora_core::WsprCycle) {
    let config = service.config.borrow().clone();
    if !config.wsprnet {
        return;
    }
    if config.call.is_empty() || config.grid.is_empty() {
        service.status.fail("wsprnet", "servono il tuo nominativo e il tuo locatore (scheda Generali)");
        return;
    }
    let station = wsprnet::Station { call: config.call.clone(), grid: config.grid.clone(), version: service.endpoints.software.clone() };
    let requests = wsprnet::requests(cycle, &station);
    if requests.is_empty() {
        return;
    }
    let delay = service.endpoints.wsprnet_delay.mul_f64(random_unit());
    let service = Arc::clone(service);
    tokio::spawn(async move {
        tokio::time::sleep(delay).await;
        // spento mentre si aspettava: niente
        if !service.config.borrow().wsprnet {
            return;
        }
        let dropped = with_wsprnet(&service, |q| q.push(requests, Instant::now()));
        if dropped > 0 {
            tracing::warn!("WSPRnet: {dropped} richieste vecchie lasciate perdere");
        }
        service.wsprnet_wake.notify_one();
    });
}

/// Lavora sulla coda di WSPRnet e poi riporta nello stato quante richieste aspettano ancora: l'esito dell'ultimo invio da solo non dice che
/// altre richieste (gli spot dello stesso periodo, un nuovo tentativo dopo un errore) sono ancora da mandare.
fn with_wsprnet<R>(service: &Service, change: impl FnOnce(&mut wsprnet::Queue) -> R) -> R {
    let mut queue = service.wsprnet.lock().unwrap_or_else(|p| p.into_inner());
    let out = change(&mut queue);
    service.status.set_pending("wsprnet", queue.outstanding() as u64);
    out
}

/// Manda a WSPRnet le richieste in coda, una alla volta, con i nuovi tentativi di WSJT-X. Prima di ogni invio controlla che l'invio sia
/// ancora acceso: spento, la coda si svuota.
async fn wsprnet_worker(service: Arc<Service>) {
    loop {
        let next = service.wsprnet.lock().unwrap_or_else(|p| p.into_inner()).next_due();
        match next {
            None => service.wsprnet_wake.notified().await,
            Some(at) => {
                let wait = at.saturating_duration_since(Instant::now());
                if !wait.is_zero() {
                    tokio::select! {
                        _ = tokio::time::sleep(wait) => {}
                        _ = service.wsprnet_wake.notified() => continue,
                    }
                }
            }
        }
        if !service.config.borrow().wsprnet {
            with_wsprnet(&service, wsprnet::Queue::clear);
            continue;
        }
        // la richiesta in volo conta fra quelle da mandare finche' non se ne conosce l'esito
        let taken = with_wsprnet(&service, |q| {
            let p = q.take_due(Instant::now());
            q.sending = p.is_some();
            p
        });
        let Some(pending) = taken else { continue };
        let (url, timeout, agent) = (service.endpoints.wsprnet_url.clone(), service.endpoints.wsprnet_timeout, service.endpoints.user_agent.clone());
        let request = pending.request.clone();
        let result = tokio::task::spawn_blocking(move || wsprnet::send(&url, &request, timeout, &agent)).await.unwrap_or_else(|e| Err(format!("invio interrotto: {e}")));
        let what = if pending.request.kind == wsprnet::Kind::Spot { "spot" } else { "stato della stazione" };
        match result {
            Ok(_) => {
                service.status.ok("wsprnet", format!("{what} inviato"));
                with_wsprnet(&service, |q| q.sending = false);
            }
            Err(e) => {
                service.status.fail("wsprnet", format!("{what} non inviato: {e}"));
                let retry = with_wsprnet(&service, |q| {
                    q.sending = false;
                    q.retry(pending, Instant::now(), random_unit())
                });
                match retry {
                    Some(d) => tracing::info!("WSPRnet: nuovo tentativo fra {} s", d.as_secs()),
                    None => tracing::warn!("WSPRnet: richiesta lasciata perdere dopo i tentativi"),
                }
            }
        }
    }
}

/// Un QSO registrato: lo si manda al server TCP e a eQSL, ognuno in un'attivita' a parte, e lo si mette in coda per l'invio diretto.
fn dispatch_qso(service: &Arc<Service>, config: Config, q: QsoRecord) {
    let now = (service.now)();
    let today = adif::utc_date(now);
    let station = Station { call: config.call.clone(), grid: config.grid.clone(), tx_power: config.tx_power.clone() };
    // invio diretto: lo stesso record ADIF del secondo server UDP di JTDX, senza credenziali (si aggiungono al momento dell'invio)
    let record = adif::jtdx_record(&q, &station, &today);
    for target in upload::Target::ALL.into_iter().filter(|t| upload::enabled(&config, *t)) {
        service.queue.push(target, &q.call, record.clone(), now);
    }
    if let Some(addr) = config.tcp {
        let record = adif::jtdx_record_with(&q, &station, &today, config.eqsl.is_some().then_some(today.as_str()));
        let status = Arc::clone(&service.status);
        tokio::spawn(async move {
            match tcp::send(addr, &record).await {
                Ok(_) => status.ok("tcp", format!("QSO inviato a {addr}")),
                Err(e) => status.fail("tcp", e),
            }
        });
    }
    if let Some(account) = config.eqsl {
        let (status, secrets, url, timeout) = (Arc::clone(&service.status), Arc::clone(&service.secrets), service.endpoints.eqsl_url.clone(), service.endpoints.eqsl_timeout);
        tokio::spawn(async move {
            let Some(password) = secrets.get(EQSL_SECRET) else {
                status.fail("eqsl", "password di eQSL non impostata (o non leggibile da questo utente)");
                return;
            };
            let text = eqsl::adif(&account.user, &password, &account.nick, &q, &today);
            match tokio::task::spawn_blocking(move || eqsl::upload(&url, &text, timeout)).await {
                Ok(Ok(reply)) => status.ok("eqsl", reply),
                Ok(Err(e)) => status.fail("eqsl", e),
                Err(e) => status.fail("eqsl", format!("invio interrotto: {e}")),
            }
        });
    }
}

/// Un indirizzo di server da testo: un IP (o `localhost`) e una porta.
pub fn server_addr(host: &str, port: u16) -> Option<SocketAddr> {
    let host = host.trim();
    let ip: IpAddr = if host.eq_ignore_ascii_case("localhost") { IpAddr::from([127, 0, 0, 1]) } else { host.parse().ok()? };
    (port != 0).then_some(SocketAddr::new(ip, port))
}

#[cfg(test)]
mod tests {

    #[test]
    fn spot_mode_names_fst4_without_the_period() {
        assert_eq!(spot_mode("FST4-60"), "FST4");
        assert_eq!(spot_mode("fst4w-120"), "FST4W");
        assert_eq!(spot_mode("FT4"), "FT4");
        assert_eq!(spot_mode("FT8"), "FT8");
    }

    use super::*;

    fn decode(time_ms: u32) -> DecodedMessage {
        DecodedMessage { id: 1, time_ms, snr: -7, delta_time: 0.1, freq_hz: 1000, text: "CQ K1ABC FN42".into(), tag: None, country: None, continent: None, lotw_last_upload: None, hint: false, unconfirmed: false }
    }

    /// 2026-11-01 00:00:00 UTC.
    const MIDNIGHT: u64 = 1_793_491_200;

    #[test]
    fn the_same_call_is_spotted_once_every_five_minutes_below_49_mhz() {
        let mut q = VecDeque::new();
        let mut cache = std::collections::HashMap::new();
        let hf = Period { dial_hz: 14_074_000, mode: "FT8", blocked_until_ms: 0, previous: None };
        queue_spot(&mut q, &decode(0), &hf, MIDNIGHT + 13, Some(&mut cache));
        queue_spot(&mut q, &decode(15_000), &hf, MIDNIGHT + 28, Some(&mut cache));
        assert_eq!(q.len(), 1, "stesso CQ dopo 15 s: non si ridà");
        queue_spot(&mut q, &decode(315_000), &hf, MIDNIGHT + 328, Some(&mut cache));
        assert_eq!(q.len(), 2, "dopo 5 minuti si ridà");
        let vhf = Period { dial_hz: 50_313_000, ..hf };
        queue_spot(&mut q, &decode(330_000), &vhf, MIDNIGHT + 343, Some(&mut cache));
        queue_spot(&mut q, &decode(345_000), &vhf, MIDNIGHT + 358, Some(&mut cache));
        assert_eq!(q.len(), 4, "sopra 49 MHz ogni spot passa");
    }

    #[test]
    fn a_spot_received_just_before_midnight_keeps_yesterdays_date() {
        let mut q = VecDeque::new();
        let period = Period { dial_hz: 14_074_000, mode: "FT8", blocked_until_ms: 0, previous: None };
        // periodo delle 23:59:45, elaborato alle 00:00:02
        queue_spot(&mut q, &decode(86_385_000), &period, MIDNIGHT + 2, None);
        assert_eq!(q.front().map(|s| u64::from(s.time)), Some(MIDNIGHT - 15));
        // un periodo di oggi resta di oggi, con l'ora del periodo
        queue_spot(&mut q, &decode(0), &period, MIDNIGHT + 13, None);
        assert_eq!(q.back().map(|s| u64::from(s.time)), Some(MIDNIGHT));
    }

    #[test]
    fn after_a_band_change_older_periods_keep_their_band_and_the_mixed_one_is_skipped() {
        let mut q = VecDeque::new();
        let changed_at = (MIDNIGHT + 3600) * 1000;
        let period = Period { dial_hz: 7_074_000, mode: "FT8", blocked_until_ms: changed_at, previous: Some((14_080_000, "FT4")) };
        let now = MIDNIGHT + 3600 + 3;
        // periodo prima del cambio: banda e modo di allora
        queue_spot(&mut q, &decode(3_600_000 - 15_000), &period, now, None);
        assert_eq!(q.back().map(|s| (s.freq_hz, s.mode.as_str())), Some((14_081_000, "FT4")));
        // periodo del cambio: niente spot
        queue_spot(&mut q, &decode(3_600_000), &period, now, None);
        assert_eq!(q.len(), 1);
        // periodo dopo il cambio: banda nuova
        queue_spot(&mut q, &decode(3_615_000), &period, now + 15, None);
        assert_eq!(q.back().map(|s| (s.freq_hz, s.mode.as_str())), Some((7_075_000, "FT8")));
        // senza la banda di prima i periodi vecchi non si segnalano
        let none = Period { previous: None, ..period };
        queue_spot(&mut q, &decode(3_600_000 - 15_000), &none, now, None);
        assert_eq!(q.len(), 2);
    }
}
