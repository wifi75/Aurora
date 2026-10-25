//! Gli invii verso l'esterno della scheda Segnalazioni: spotting su PSK Reporter, spot su DXSummit, QSO al server TCP e a eQSL.
//!
//! Regole comuni (decise per tutti gli invii verso internet):
//! - ognuno si accende e si spegne dalle impostazioni (`Config`), a caldo; spento non parte niente e non si accoda niente;
//! - **non blocca mai il motore**: l'ascolto del bus passa lo spot o il QSO a un'attivita' a parte (con i suoi tempi massimi) e riprende subito;
//! - l'esito dell'ultimo invio sta in [`ReportStatus`] (e nel registro, senza mai password);
//! - la password di eQSL si legge dalla protezione dei segreti solo nel momento dell'invio.

pub mod dxsummit;
pub mod eqsl;
pub mod psk;
pub mod secret;
pub mod status;
pub mod tcp;

use std::{
    collections::VecDeque,
    net::{IpAddr, SocketAddr},
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
    /// Server TCP a cui mandare ogni QSO (`EnableTCPConnection` acceso e indirizzo valido).
    pub tcp: Option<SocketAddr>,
    /// eQSL acceso con questo account.
    pub eqsl: Option<EqslAccount>,
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
    /// Il programma che dichiara l'invio a PSK Reporter.
    pub software: String,
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
            software: format!("Aurora v{}", aurora_core::version::APP_VERSION),
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
        let service = Arc::new(Service { config, status, secrets, endpoints, now, last_spot: Mutex::new(None) });
        let events = bus.subscribe_events();
        tokio::spawn(run(Arc::clone(&service), events, rx));
        service
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

    /// Spot di una stazione su DXSummit (pulsante «DX Call»): solo con lo spotting acceso, una volta per stazione ogni 5 minuti, con un thread a parte e
    /// attesa massima di 10 s. Restituisce l'esito, che finisce anche nello stato.
    pub async fn spot_dxsummit(&self, dx_call: &str, freq_hz: u64, info: &str) -> Result<(), String> {
        let config = self.config.borrow().clone();
        if !config.dxsummit {
            return Err("lo spotting su DXSummit e' spento".into());
        }
        if config.call.is_empty() {
            return Err("manca il tuo nominativo".into());
        }
        let dx_call = dx_call.trim().to_uppercase();
        if dx_call.is_empty() || !dx_call.chars().all(|c| c.is_ascii_alphanumeric() || c == '/') || dx_call.len() > 20 {
            return Err("nominativo da segnalare non valido".into());
        }
        {
            let mut last = self.last_spot.lock().unwrap_or_else(|p| p.into_inner());
            if last.as_ref().is_some_and(|(c, at)| *c == dx_call && at.elapsed() < Duration::from_secs(300)) {
                return Err("questa stazione e' gia' stata segnalata da poco".into());
            }
            *last = Some((dx_call.clone(), Instant::now()));
        }
        let request = dxsummit::SpotRequest { my_call: config.call, dx_call, freq_hz, info: info.trim().chars().take(100).collect() };
        let (url, timeout) = (self.endpoints.dxsummit_url.clone(), self.endpoints.dxsummit_timeout);
        let result = tokio::task::spawn_blocking(move || dxsummit::send(&url, &request, timeout)).await.unwrap_or_else(|e| Err(format!("invio interrotto: {e}")));
        match &result {
            Ok(()) => self.status.ok("dxsummit", "spot inviato"),
            Err(e) => {
                // un invio fallito non conta: si puo' riprovare subito
                *self.last_spot.lock().unwrap_or_else(|p| p.into_inner()) = None;
                self.status.fail("dxsummit", e.clone());
            }
        }
        result
    }
}

async fn run(service: Arc<Service>, mut events: tokio::sync::broadcast::Receiver<Event>, mut config_rx: watch::Receiver<Config>) {
    let mut queue: VecDeque<psk::Spot> = VecDeque::new();
    let mut packetizer = psk::Packetizer::new(random_id());
    let mut dial_hz: u64 = 0;
    let mut mode = "FT8".to_string();
    // `m_okToPost`: dopo un cambio di banda o di modo i messaggi del periodo in corso non si segnalano (ascolto misto)
    let mut blocked_until_ms: u64 = ((service.now)() * 1000) / period_ms(&mode) * period_ms(&mode);
    let mut guard = DupeGuard::default();
    let mut tick = tokio::time::interval(service.endpoints.psk_every);
    tick.tick().await;
    loop {
        tokio::select! {
            event = events.recv() => match event {
                Ok(Event::Status(s)) => {
                    let changed_mode = !s.mode.is_empty() && s.mode != mode;
                    if changed_mode {
                        mode = s.mode.clone();
                        blocked_until_ms = ((service.now)() * 1000) / period_ms(&mode) * period_ms(&mode);
                        flush_psk(&service, &mut packetizer, &mut queue).await;
                    }
                    dial_hz = s.dial_hz;
                }
                Ok(Event::BandChanged { dial_hz: hz, .. }) => {
                    flush_psk(&service, &mut packetizer, &mut queue).await;
                    dial_hz = hz;
                    blocked_until_ms = ((service.now)() * 1000) / period_ms(&mode) * period_ms(&mode);
                }
                Ok(Event::Decode(d)) => {
                    let config = service.config.borrow().clone();
                    if config.psk && !config.call.is_empty() {
                        queue_spot(&mut queue, &d, dial_hz, &mode, blocked_until_ms, (service.now)());
                    }
                }
                Ok(Event::QsoLogged(q)) => {
                    let config = service.config.borrow().clone();
                    let stamp = if q.time_off_unix_ms > 0 { q.time_off_unix_ms } else { (service.now)() * 1000 };
                    if (config.tcp.is_some() || config.eqsl.is_some()) && guard.accept(&q.call, &q.mode, stamp) {
                        dispatch_qso(&service, config, q);
                    }
                }
                Ok(_) | Err(RecvError::Lagged(_)) => {}
                Err(RecvError::Closed) => break,
            },
            _ = tick.tick() => flush_psk(&service, &mut packetizer, &mut queue).await,
            changed = config_rx.changed() => {
                if changed.is_err() {
                    break;
                }
                if !service.config.borrow().psk {
                    queue.clear();
                }
            }
        }
    }
}

/// Mette in coda lo spot di un messaggio decodificato, se ha i requisiti di `readFromStdout` (messaggio standard con nominativo e locatore validi,
/// non «ipotizzato», non del periodo del cambio di banda).
fn queue_spot(queue: &mut VecDeque<psk::Spot>, d: &DecodedMessage, dial_hz: u64, mode: &str, blocked_until_ms: u64, now_secs: u64) {
    if d.hint || dial_hz == 0 {
        return;
    }
    // l'inizio del periodo del messaggio: i millisecondi del giorno, sul giorno di adesso
    let day_start = now_secs * 1000 / 86_400_000 * 86_400_000;
    let mut slot_start = day_start + u64::from(d.time_ms);
    if slot_start > now_secs * 1000 + 3_600_000 {
        slot_start = slot_start.saturating_sub(86_400_000);
    }
    if slot_start <= blocked_until_ms {
        return;
    }
    let Some((call, grid)) = psk::spot_call_and_grid(&d.text) else { return };
    if queue.len() >= psk::MAX_QUEUE {
        queue.pop_front();
    }
    queue.push_back(psk::Spot {
        call,
        grid,
        freq_hz: dial_hz + u64::from(d.freq_hz),
        mode: if mode.eq_ignore_ascii_case("FT4") { "FT4" } else { "FT8" }.to_string(),
        snr: d.snr,
        time: u32::try_from(now_secs).unwrap_or(u32::MAX),
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

/// Un QSO registrato: lo si manda al server TCP e a eQSL, ognuno in un'attivita' a parte.
fn dispatch_qso(service: &Arc<Service>, config: Config, q: QsoRecord) {
    let now = (service.now)();
    let today = adif::utc_date(now);
    let station = Station { call: config.call.clone(), grid: config.grid.clone(), tx_power: config.tx_power.clone() };
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
