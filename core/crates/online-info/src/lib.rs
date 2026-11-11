//! Informazioni da servizi online pubblici, **solo a richiesta** (spente finche' non si accendono):
//!
//! - **PSK Reporter** (`pskreporter.info`): chi ha ricevuto il proprio nominativo. Il sito vede il nominativo e l'indirizzo IP.
//! - **hamqsl.com**: condizioni di propagazione. Il sito vede solo l'indirizzo IP.
//! - **NG3K** (`ng3k.com`): il calendario delle DXpedition annunciate. Il sito vede solo l'indirizzo IP.
//!
//! Sono richieste di sola lettura, rare (PSK Reporter chiede di non insistere: al massimo una ogni 5 minuti, qui anche meno) e
//! nulla di cio' che Aurora riceve o trasmette viene inviato: non si mandano spot ne' messaggi.

mod xml;

pub mod dxped;
pub mod foxcheck;
pub mod lotw;
pub mod psk;
pub mod psk_live;
pub mod solar;

use std::{
    path::{Path, PathBuf},
    sync::{Arc, Mutex, MutexGuard},
    time::Duration,
};

use serde::{Deserialize, Serialize};

pub use dxped::Dxpedition;
pub use psk::Report;
pub use solar::Solar;

/// Ogni quanto si chiede a PSK Reporter (anche dopo un errore).
pub const PSK_EVERY_MS: u64 = 5 * 60_000;
/// Ogni quanto si chiede la propagazione: il servizio la aggiorna ogni qualche ora.
pub const SOLAR_EVERY_MS: u64 = 30 * 60_000;
/// Ogni quanto si chiede il calendario delle DXpedition: cambia poche volte al giorno.
pub const DXPED_EVERY_MS: u64 = 6 * 60 * 60_000;
/// Con «Aggiorna ora» non si chiede piu' di una volta al minuto.
pub const MANUAL_MIN_MS: u64 = 60_000;
/// Quante ore indietro si guardano i rapporti di PSK Reporter.
pub const PSK_HOURS: u32 = 3;

/// Come si scarica una pagina: Internet davvero, nelle prove un finto.
pub trait Fetch: Send + Sync {
    fn get(&self, url: &str) -> Result<String, String>;
}

pub struct HttpFetch {
    user_agent: String,
}

impl HttpFetch {
    pub fn new(app_version: &str) -> HttpFetch {
        HttpFetch { user_agent: format!("Aurora/{app_version}") }
    }
}

impl Fetch for HttpFetch {
    fn get(&self, url: &str) -> Result<String, String> {
        let agent: ureq::Agent = ureq::Agent::config_builder().timeout_global(Some(Duration::from_secs(15))).http_status_as_error(false).build().into();
        let mut response = agent.get(url).header("User-Agent", &self.user_agent).call().map_err(|e| e.to_string())?;
        let status = response.status().as_u16();
        if status != 200 {
            return Err(format!("il servizio ha risposto {status}"));
        }
        response.body_mut().read_to_string().map_err(|e| e.to_string())
    }
}

/// Il sito di PSK Reporter (per la «Prova»: la pagina principale, senza chiedere rapporti).
pub const PSK_HOME: &str = "https://pskreporter.info/";
/// Gli ultimi spot di DXSummit in JSON (per la «Prova»: uno solo).
pub const DXSUMMIT_PROBE: &str = "https://www.dxsummit.fi/api/v1/spots?limit=1";

/// La «Prova» delle Impostazioni: una richiesta sola, al piu' `timeout`, e il corpo della risposta (200) da leggere. Bloccante.
pub fn probe(url: &str, timeout: Duration, app_version: &str) -> Result<String, String> {
    let agent: ureq::Agent = ureq::Agent::config_builder().timeout_global(Some(timeout)).http_status_as_error(false).build().into();
    let mut response = agent.get(url).header("User-Agent", &format!("Aurora/{app_version}")).call().map_err(|e| e.to_string())?;
    let status = response.status().as_u16();
    if status != 200 {
        return Err(format!("il servizio ha risposto {status}"));
    }
    response.body_mut().read_to_string().map_err(|e| e.to_string())
}

/// Quali servizi sono accesi. Tutti accesi di fabbrica (PSK Reporter, propagazione, calendario delle DXpedition); si spengono dalla pagina.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub psk: bool,
    pub solar: bool,
    pub dxped: bool,
}

impl Default for Settings {
    fn default() -> Settings {
        Settings { psk: true, solar: true, dxped: true }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct PskSnapshot {
    /// Quando e' stata fatta l'ultima richiesta riuscita (millisecondi Unix, 0 = mai).
    pub fetched_ms: u64,
    pub callsign: String,
    pub hours: u32,
    pub reports: Vec<Report>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct SolarSnapshot {
    pub fetched_ms: u64,
    pub data: Option<Solar>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct DxpedSnapshot {
    pub fetched_ms: u64,
    pub items: Vec<Dxpedition>,
    pub error: Option<String>,
}

#[derive(Default)]
struct State {
    settings: Settings,
    psk: PskSnapshot,
    psk_attempt_ms: u64,
    solar: SolarSnapshot,
    solar_attempt_ms: u64,
    dxped: DxpedSnapshot,
    dxped_attempt_ms: u64,
}

pub struct OnlineService {
    path: PathBuf,
    fetch: Arc<dyn Fetch>,
    state: Mutex<State>,
    /// Il flusso in tempo reale dei rapporti PSK Reporter (si avvia da `ensure_live`, mai nelle prove).
    live: Mutex<Option<psk_live::Feed>>,
}

fn lock<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
}

impl OnlineService {
    pub fn open(path: &Path, fetch: Arc<dyn Fetch>) -> Arc<OnlineService> {
        let settings: Settings = std::fs::read_to_string(path).ok().and_then(|t| serde_json::from_str(&t).ok()).unwrap_or_default();
        Arc::new(OnlineService { path: path.to_path_buf(), fetch, state: Mutex::new(State { settings, ..State::default() }), live: Mutex::new(None) })
    }

    pub fn settings(&self) -> Settings {
        lock(&self.state).settings
    }

    /// Cambia gli interruttori e li salva; spegnere un servizio dimentica cio' che aveva scaricato.
    pub fn set_settings(&self, settings: Settings) -> Result<Settings, String> {
        let text = serde_json::to_string_pretty(&settings).map_err(|e| e.to_string())?;
        std::fs::write(&self.path, text).map_err(|e| format!("{}: {e}", self.path.display()))?;
        let mut state = lock(&self.state);
        if !settings.psk {
            state.psk = PskSnapshot::default();
            state.psk_attempt_ms = 0;
            *lock(&self.live) = None;
        }
        if !settings.solar {
            state.solar = SolarSnapshot::default();
            state.solar_attempt_ms = 0;
        }
        if !settings.dxped {
            state.dxped = DxpedSnapshot::default();
            state.dxped_attempt_ms = 0;
        }
        state.settings = settings;
        Ok(settings)
    }

    /// Tiene acceso (o spegne) il flusso in tempo reale di PSK Reporter in base all'interruttore e al nominativo. Non blocca; da chiamare ogni tanto.
    pub fn ensure_live(&self, callsign: &str) {
        // un nominativo con «/» (IU3ABC/P) non si puo' mettere nel topic MQTT, dove «/» separa i livelli: resta solo la richiesta ogni 5 minuti
        let wanted = if self.settings().psk { psk::clean_callsign(callsign).filter(|c| !c.contains('/')) } else { None };
        let mut live = lock(&self.live);
        match (&*live, wanted) {
            (Some(feed), Some(call)) if feed.callsign == call => {}
            (_, Some(call)) => *live = Some(psk_live::Feed::start(&call)),
            (_, None) => *live = None,
        }
    }

    /// I rapporti: quelli della richiesta (ogni 5 minuti, fino a tre ore indietro) uniti a quelli arrivati in tempo reale nelle stesse tre ore, i piu'
    /// recenti per primi.
    pub fn psk(&self) -> PskSnapshot {
        let mut snapshot = lock(&self.state).psk.clone();
        let now_s = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_secs());
        let oldest = now_s.saturating_sub(u64::from(PSK_HOURS) * 3600);
        let (live_call, live): (String, Vec<Report>) = {
            let feed = lock(&self.live);
            match &*feed {
                Some(f) if snapshot.callsign.is_empty() || f.callsign == snapshot.callsign => {
                    (f.callsign.clone(), f.reports.lock().unwrap_or_else(|p| p.into_inner()).iter().filter(|r| r.time_s >= oldest).cloned().collect())
                }
                _ => (String::new(), Vec::new()),
            }
        };
        if let Some(newest) = live.iter().map(|r| r.time_s).max() {
            let mut all = live.clone();
            all.extend(snapshot.reports.iter().filter(|r| !live.iter().any(|l| psk_live::same(l, r))).cloned());
            all.sort_by_key(|r| std::cmp::Reverse(r.time_s));
            all.truncate(psk::MAX_REPORTS);
            snapshot.reports = all;
            if snapshot.callsign.is_empty() {
                snapshot.callsign = live_call;
            }
            snapshot.hours = PSK_HOURS;
            // «aggiornato» al rapporto piu' recente arrivato dal flusso, non all'ora di adesso (il flusso puo' essere caduto da tempo)
            snapshot.fetched_ms = snapshot.fetched_ms.max(newest.saturating_mul(1000));
        }
        snapshot
    }

    pub fn solar(&self) -> SolarSnapshot {
        lock(&self.state).solar.clone()
    }

    pub fn dxped(&self) -> DxpedSnapshot {
        lock(&self.state).dxped.clone()
    }

    fn fetch_dxped(&self, now_ms: u64) {
        lock(&self.state).dxped_attempt_ms = now_ms;
        let result = self.fetch.get(dxped::URL).and_then(|body| {
            let items = dxped::parse(&body);
            if items.is_empty() { Err("risposta non riconosciuta da NG3K".to_string()) } else { Ok(items) }
        });
        let mut state = lock(&self.state);
        if !state.settings.dxped {
            return;
        }
        match result {
            Ok(items) => state.dxped = DxpedSnapshot { fetched_ms: now_ms, items, error: None },
            Err(e) => {
                tracing::warn!("NG3K: {e}");
                state.dxped.error = Some(e);
            }
        }
    }

    fn fetch_psk(&self, now_ms: u64, callsign: &str) {
        let Some(url) = psk::query_url(callsign, PSK_HOURS) else { return };
        let call = psk::clean_callsign(callsign).unwrap_or_default();
        lock(&self.state).psk_attempt_ms = now_ms;
        let result = self.fetch.get(&url).and_then(|body| {
            if body.contains("<receptionReports") { Ok(psk::parse(&body)) } else { Err("risposta non riconosciuta da PSK Reporter".to_string()) }
        });
        let mut state = lock(&self.state);
        if !state.settings.psk {
            return;
        }
        match result {
            Ok(reports) => state.psk = PskSnapshot { fetched_ms: now_ms, callsign: call, hours: PSK_HOURS, reports, error: None },
            Err(e) => {
                tracing::warn!("PSK Reporter: {e}");
                state.psk.callsign = call;
                state.psk.hours = PSK_HOURS;
                state.psk.error = Some(e);
            }
        }
    }

    fn fetch_solar(&self, now_ms: u64) {
        lock(&self.state).solar_attempt_ms = now_ms;
        let result = self.fetch.get(solar::URL).and_then(|body| solar::parse(&body).ok_or_else(|| "risposta non riconosciuta da hamqsl.com".to_string()));
        let mut state = lock(&self.state);
        if !state.settings.solar {
            return;
        }
        match result {
            Ok(data) => state.solar = SolarSnapshot { fetched_ms: now_ms, data: Some(data), error: None },
            Err(e) => {
                tracing::warn!("hamqsl.com: {e}");
                state.solar.error = Some(e);
            }
        }
    }

    /// Chiede cio' che e' acceso e scaduto (da chiamare ogni tanto, anche spesso: non fa nulla se non e' ora). E' bloccante.
    /// Restituisce `true` se ha fatto almeno una richiesta.
    pub fn refresh_due(&self, now_ms: u64, callsign: &str) -> bool {
        let (settings, psk_attempt, solar_attempt, psk_call, dxped_attempt) = {
            let s = lock(&self.state);
            (s.settings, s.psk_attempt_ms, s.solar_attempt_ms, s.psk.callsign.clone(), s.dxped_attempt_ms)
        };
        let mut done = false;
        if settings.psk && psk::clean_callsign(callsign).is_some() {
            let call = psk::clean_callsign(callsign).unwrap_or_default();
            let elapsed = now_ms.saturating_sub(psk_attempt);
            // un nominativo diverso (cambiato in JTDX) si guarda subito, ma non piu' di una volta al minuto
            let due = psk_attempt == 0 || elapsed >= PSK_EVERY_MS || (call != psk_call && elapsed >= MANUAL_MIN_MS);
            if due {
                self.fetch_psk(now_ms, callsign);
                done = true;
            }
        }
        if settings.solar && (solar_attempt == 0 || now_ms.saturating_sub(solar_attempt) >= SOLAR_EVERY_MS) {
            self.fetch_solar(now_ms);
            done = true;
        }
        if settings.dxped && (dxped_attempt == 0 || now_ms.saturating_sub(dxped_attempt) >= DXPED_EVERY_MS) {
            self.fetch_dxped(now_ms);
            done = true;
        }
        done
    }

    /// Chiede subito cio' che e' acceso, ma non piu' di una volta al minuto per servizio. Restituisce se ha chiesto qualcosa.
    pub fn refresh_now(&self, now_ms: u64, callsign: &str) -> bool {
        let (settings, psk_attempt, solar_attempt, dxped_attempt) = {
            let s = lock(&self.state);
            (s.settings, s.psk_attempt_ms, s.solar_attempt_ms, s.dxped_attempt_ms)
        };
        let mut done = false;
        if settings.psk && psk::clean_callsign(callsign).is_some() && (psk_attempt == 0 || now_ms.saturating_sub(psk_attempt) >= MANUAL_MIN_MS) {
            self.fetch_psk(now_ms, callsign);
            done = true;
        }
        if settings.solar && (solar_attempt == 0 || now_ms.saturating_sub(solar_attempt) >= MANUAL_MIN_MS) {
            self.fetch_solar(now_ms);
            done = true;
        }
        if settings.dxped && (dxped_attempt == 0 || now_ms.saturating_sub(dxped_attempt) >= MANUAL_MIN_MS) {
            self.fetch_dxped(now_ms);
            done = true;
        }
        done
    }
}

#[cfg(test)]
mod tests;
