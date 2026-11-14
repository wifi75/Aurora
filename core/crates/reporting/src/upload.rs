//! Invio diretto dei QSO messi a log: Wavelog/Cloudlog, ClubLog, QRZ.com Logbook e LoTW (tramite TQSL).
//!
//! Regole:
//! - ogni QSO registrato finisce in una **coda** (una voce per servizio acceso), salvata su file accanto alle impostazioni: se la rete manca o il
//!   servizio non risponde il QSO resta li' e si riprova piu' tardi, con attese crescenti (1, 2, 5, 10, 30 minuti, poi ogni ora), anche dopo un riavvio;
//! - ogni servizio ha un'attivita' sua: un servizio lento (TQSL puo' metterci minuti) non ferma gli altri e nessuno ferma il motore;
//! - un errore di credenziali **ferma** quel servizio (i QSO restano in coda) finche' non si cambiano le impostazioni o le credenziali: ClubLog
//!   blocca l'indirizzo di chi ripete richieste con credenziali sbagliate, e riprovare a vuoto non serve a nessuno;
//! - un QSO rifiutato (dati non validi) esce dalla coda con l'errore nello stato; un doppione vale come riuscito;
//! - un QSO in coda da piu' di sette giorni si scarta (con l'avviso nel registro);
//! - le credenziali non sono mai nella coda, nel registro o nello stato: si leggono dalla protezione dei segreti al momento dell'invio.

use std::{
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
    time::Duration,
};

use serde::{Deserialize, Serialize};
use tokio::sync::watch;

use crate::{Config, Service};

/// I servizi dell'invio diretto, nell'ordine della scheda.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Target {
    /// Wavelog o Cloudlog (stessa API).
    Wavelog,
    Clublog,
    Qrz,
    Lotw,
    Hrdlog,
}

impl Target {
    pub const ALL: [Target; 5] = [Target::Wavelog, Target::Clublog, Target::Qrz, Target::Lotw, Target::Hrdlog];

    /// Il nome nello stato degli invii e nell'API.
    pub fn kind(self) -> &'static str {
        match self {
            Target::Wavelog => "wavelog",
            Target::Clublog => "clublog",
            Target::Qrz => "qrz",
            Target::Lotw => "lotw",
            Target::Hrdlog => "hrdlog",
        }
    }

    pub fn parse(name: &str) -> Option<Target> {
        Target::ALL.into_iter().find(|t| t.kind() == name)
    }

    /// Quanti QSO per invio: TQSL firma un file intero (meglio uno solo per molti QSO), gli altri uno alla volta.
    fn batch(self) -> usize {
        match self {
            Target::Lotw => 50,
            _ => 1,
        }
    }

    fn index(self) -> usize {
        match self {
            Target::Wavelog => 0,
            Target::Clublog => 1,
            Target::Qrz => 2,
            Target::Lotw => 3,
            Target::Hrdlog => 4,
        }
    }
}

/// Come e' andato un invio.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Outcome {
    /// Registrato.
    Sent(String),
    /// Il servizio lo aveva gia': vale come riuscito.
    Duplicate(String),
    /// Rete, tempo scaduto, servizio occupato: si riprova piu' tardi.
    Retry(String),
    /// Credenziali o configurazione sbagliate: il servizio si ferma (i QSO restano in coda) finche' non cambiano le impostazioni.
    Blocked(String),
    /// Il servizio ha rifiutato il QSO: esce dalla coda.
    Rejected(String),
}

/// Un QSO che aspetta di partire verso un servizio. `record` e' il record ADIF senza credenziali.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Pending {
    pub id: u64,
    pub target: Target,
    pub call: String,
    pub record: String,
    /// Secondi Unix.
    pub queued_at: u64,
    pub attempts: u32,
    pub next_at: u64,
}

/// Un QSO resta in coda al piu' sette giorni.
pub const MAX_AGE_SECS: u64 = 7 * 86_400;
/// Al piu' tanti QSO in coda per servizio (oltre si scartano i piu' vecchi).
pub const MAX_PER_TARGET: usize = 2000;
/// Attesa massima di una richiesta HTTP.
pub const HTTP_TIMEOUT: Duration = Duration::from_secs(20);

/// L'attesa prima del tentativo successivo, in secondi: 1, 2, 5, 10, 30 minuti, poi ogni ora.
pub fn retry_delay(attempts: u32) -> u64 {
    match attempts {
        0 | 1 => 60,
        2 => 120,
        3 => 300,
        4 => 600,
        5 => 1800,
        _ => 3600,
    }
}

#[derive(Serialize, Deserialize)]
struct QueueFile {
    version: u32,
    items: Vec<Pending>,
}

/// La coda dei QSO da mandare, per tutti i servizi, salvata su file a ogni cambiamento (scrittura atomica).
pub struct Queue {
    items: Mutex<Vec<Pending>>,
    file: Option<PathBuf>,
    changed: watch::Sender<u64>,
    blocked: Mutex<[bool; 5]>,
}

impl Queue {
    /// Apre la coda (dal file, se c'e'); `None` = solo in memoria (prove).
    pub fn open(file: Option<PathBuf>) -> Queue {
        let items = file
            .as_deref()
            .and_then(|p| std::fs::read_to_string(p).ok())
            .and_then(|text| serde_json::from_str::<QueueFile>(&text).ok())
            .map(|f| f.items)
            .unwrap_or_default();
        if !items.is_empty() {
            tracing::info!("invio diretto dei QSO: {} invii in coda dalla volta scorsa", items.len());
        }
        let (changed, _) = watch::channel(0);
        Queue { items: Mutex::new(items), file, changed, blocked: Mutex::new([false; 5]) }
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, Vec<Pending>> {
        self.items.lock().unwrap_or_else(|p| p.into_inner())
    }

    pub fn subscribe(&self) -> watch::Receiver<u64> {
        self.changed.subscribe()
    }

    fn bump(&self) {
        self.changed.send_modify(|n| *n = n.wrapping_add(1));
    }

    fn save(&self, items: &[Pending]) {
        let Some(path) = &self.file else { return };
        let body = QueueFile { version: 1, items: items.to_vec() };
        let Ok(text) = serde_json::to_string(&body) else { return };
        if let Some(dir) = path.parent().filter(|d| !d.as_os_str().is_empty()) {
            let _ = std::fs::create_dir_all(dir);
        }
        let tmp = path.with_extension("json.tmp");
        if let Err(e) = std::fs::write(&tmp, text).and_then(|()| std::fs::rename(&tmp, path)) {
            tracing::warn!("coda dell'invio diretto non salvata: {e}");
        }
    }

    /// Mette in coda un QSO per un servizio, da mandare subito.
    pub fn push(&self, target: Target, call: &str, record: String, now: u64) {
        {
            let mut items = self.lock();
            let id = items.iter().map(|p| p.id).max().unwrap_or(0) + 1;
            items.push(Pending { id, target, call: call.to_string(), record, queued_at: now, attempts: 0, next_at: now });
            let count = items.iter().filter(|p| p.target == target).count();
            if count > MAX_PER_TARGET
                && let Some(i) = items.iter().position(|p| p.target == target)
            {
                let dropped = items.remove(i);
                tracing::warn!("invio {}: coda piena, scartato il QSO con {}", target.kind(), dropped.call);
            }
            self.save(&items);
        }
        self.bump();
    }

    /// I QSO pronti da mandare adesso (al piu' `max`), dal piu' vecchio.
    pub fn due(&self, target: Target, now: u64, max: usize) -> Vec<Pending> {
        self.lock().iter().filter(|p| p.target == target && p.next_at <= now).take(max).cloned().collect()
    }

    /// Toglie i QSO piu' vecchi di [`MAX_AGE_SECS`]: restituisce i nominativi scartati.
    pub fn expire(&self, target: Target, now: u64) -> Vec<String> {
        let mut items = self.lock();
        let (old, keep): (Vec<Pending>, Vec<Pending>) = items.drain(..).partition(|p| p.target == target && now.saturating_sub(p.queued_at) > MAX_AGE_SECS);
        *items = keep;
        if !old.is_empty() {
            self.save(&items);
        }
        old.into_iter().map(|p| p.call).collect()
    }

    /// Toglie dalla coda i QSO partiti (o rifiutati).
    pub fn remove(&self, ids: &[u64]) {
        let mut items = self.lock();
        items.retain(|p| !ids.contains(&p.id));
        self.save(&items);
    }

    /// Rimanda i QSO a piu' tardi; restituisce quando (secondi Unix).
    pub fn later(&self, ids: &[u64], now: u64) -> u64 {
        let mut items = self.lock();
        let mut at = now;
        for p in items.iter_mut().filter(|p| ids.contains(&p.id)) {
            p.attempts = p.attempts.saturating_add(1);
            p.next_at = now + retry_delay(p.attempts);
            at = at.max(p.next_at);
        }
        self.save(&items);
        at
    }

    pub fn count(&self, target: Target) -> usize {
        self.lock().iter().filter(|p| p.target == target).count()
    }

    /// Il prossimo istante in cui un QSO del servizio sara' pronto.
    pub fn next_due(&self, target: Target) -> Option<u64> {
        self.lock().iter().filter(|p| p.target == target).map(|p| p.next_at).min()
    }

    /// Riprova subito tutto quello che aspetta (dopo un cambio di credenziali o una prova riuscita).
    pub fn retry_now(&self, target: Option<Target>, now: u64) {
        {
            let mut items = self.lock();
            for p in items.iter_mut().filter(|p| target.is_none_or(|t| p.target == t)) {
                p.next_at = p.next_at.min(now);
            }
            self.save(&items);
        }
        self.bump();
    }

    pub fn is_blocked(&self, target: Target) -> bool {
        self.blocked.lock().unwrap_or_else(|p| p.into_inner())[target.index()]
    }

    pub fn set_blocked(&self, target: Target, blocked: bool) {
        let changed = {
            let mut b = self.blocked.lock().unwrap_or_else(|p| p.into_inner());
            let changed = b[target.index()] != blocked;
            b[target.index()] = blocked;
            changed
        };
        if changed {
            self.bump();
        }
    }
}

/// Un agente HTTP con attesa massima, programma dichiarato (QRZ lo pretende) e senza errori per gli stati HTTP (si leggono le risposte).
pub(crate) fn agent(timeout: Duration, user_agent: &str) -> ureq::Agent {
    ureq::Agent::config_builder().timeout_global(Some(timeout)).http_status_as_error(false).user_agent(user_agent).build().into()
}

/// Un errore di rete come esito: si riprova.
pub(crate) fn network(e: &ureq::Error) -> Outcome {
    Outcome::Retry(crate::eqsl::describe(e))
}

/// Un testo ricevuto da un servizio, in una riga corta e senza marcatori HTML.
pub(crate) fn short(text: &str, max: usize) -> String {
    let mut out = String::new();
    let mut in_tag = false;
    for c in text.chars() {
        match c {
            '<' => in_tag = true,
            '>' => in_tag = false,
            c if !in_tag => out.push(if c.is_control() { ' ' } else { c }),
            _ => {}
        }
    }
    let joined = out.split_whitespace().collect::<Vec<_>>().join(" ");
    joined.chars().take(max).collect()
}

/// Il servizio e' acceso nelle impostazioni.
pub fn enabled(config: &Config, target: Target) -> bool {
    match target {
        Target::Wavelog => config.wavelog.is_some(),
        Target::Clublog => config.clublog.is_some(),
        Target::Qrz => config.qrz,
        Target::Lotw => config.lotw.is_some(),
        Target::Hrdlog => config.hrdlog.is_some(),
    }
}

/// La parte delle impostazioni di un servizio: se cambia, il servizio fermo riparte.
fn part(config: &Config, target: Target) -> String {
    match target {
        Target::Wavelog => format!("{:?}", config.wavelog),
        Target::Clublog => format!("{:?}{}", config.clublog, config.call),
        Target::Qrz => format!("{}{}", config.qrz, config.qrz_call),
        Target::Lotw => format!("{:?}", config.lotw),
        Target::Hrdlog => format!("{:?}", config.hrdlog),
    }
}

/// L'attivita' di un servizio: manda quello che e' in coda quando e' pronto, aspetta quando non c'e' niente da fare.
pub(crate) async fn worker(service: Arc<Service>, target: Target) {
    let queue = Arc::clone(&service.queue);
    let mut changes = queue.subscribe();
    let mut config_rx = service.config.subscribe();
    let mut last = part(&service.config.borrow(), target);
    let kind = target.kind();
    loop {
        let config = service.config.borrow().clone();
        let now = (service.now)();
        for call in queue.expire(target, now) {
            service.status.fail_coded(kind, "upload_expired", &[("calls", call.as_str())], format!("QSO con {call} scartato: in coda da piu' di sette giorni"));
        }
        service.status.set_pending(kind, queue.count(target) as u64);
        if enabled(&config, target) && !queue.is_blocked(target) {
            let items = queue.due(target, now, target.batch());
            if !items.is_empty() {
                let (outcome, code, detail) = send(&service, target, &config, &items).await;
                settle(&service, target, &items, outcome, code, &detail);
                continue;
            }
        }
        let wait = queue.next_due(target).map_or(60, |at| at.saturating_sub(now)).clamp(1, 60);
        tokio::select! {
            r = changes.changed() => if r.is_err() { break },
            r = config_rx.changed() => {
                if r.is_err() {
                    break;
                }
                let next = part(&config_rx.borrow_and_update(), target);
                if next != last {
                    last = next;
                    queue.set_blocked(target, false);
                }
            }
            () = tokio::time::sleep(Duration::from_secs(wait)) => {}
        }
    }
}

/// Lo stato e la coda dopo un invio.
/// `code` e `detail` (ClubLog e HRDLOG.NET per ora) sono il codice stabile dell'esito e il dettaglio della risposta: l'interfaccia li traduce nella
/// sua lingua, il testo italiano resta come ripiego e nel registro.
fn settle(service: &Service, target: Target, items: &[Pending], outcome: Outcome, code: &str, detail: &str) {
    let kind = target.kind();
    let ids: Vec<u64> = items.iter().map(|p| p.id).collect();
    let calls = items.iter().map(|p| p.call.as_str()).collect::<Vec<_>>().join(", ");
    let queue = &service.queue;
    match outcome {
        Outcome::Sent(msg) => {
            queue.remove(&ids);
            service.status.ok_coded(kind, code, &[("calls", &calls), ("detail", detail)], with_detail(&format!("QSO con {calls} inviato"), &msg));
        }
        Outcome::Duplicate(msg) => {
            queue.remove(&ids);
            service.status.ok_coded(kind, code, &[("calls", &calls), ("detail", detail)], with_detail(&format!("QSO con {calls} gia' presente"), &msg));
        }
        Outcome::Retry(msg) => {
            let at = queue.later(&ids, (service.now)());
            let (h, m) = (at / 3600 % 24, at / 60 % 60);
            let time = format!("{h:02}:{m:02}");
            service.status.fail_coded(kind, code, &[("detail", detail), ("time", &time)], format!("{msg}: nuovo tentativo alle {time} UTC"));
        }
        Outcome::Blocked(msg) => {
            queue.set_blocked(target, true);
            service.status.fail_coded(kind, code, &[("detail", detail)], format!("{msg}; invio fermo (i QSO restano in coda) finche' non cambi le impostazioni"));
        }
        Outcome::Rejected(msg) => {
            queue.remove(&ids);
            service.status.fail_coded(kind, code, &[("calls", &calls), ("detail", detail)], format!("QSO con {calls} rifiutato: {msg}"));
        }
    }
    service.status.set_pending(kind, queue.count(target) as u64);
}

fn with_detail(base: &str, detail: &str) -> String {
    if detail.is_empty() { base.to_string() } else { format!("{base} ({detail})") }
}

/// Manda (in un thread a parte: le richieste e TQSL sono bloccanti).
async fn send(service: &Service, target: Target, config: &Config, items: &[Pending]) -> (Outcome, &'static str, String) {
    let secrets = Arc::clone(&service.secrets);
    let endpoints = service.endpoints.clone();
    let config = config.clone();
    let records: Vec<String> = items.iter().map(|p| p.record.clone()).collect();
    let work_dir = endpoints.queue_file.as_deref().and_then(Path::parent).map_or_else(std::env::temp_dir, Path::to_path_buf);
    let id = items.first().map_or(0, |p| p.id);
    tokio::task::spawn_blocking(move || {
        let record = records.first().map_or("", String::as_str);
        let plain = |o: Outcome| (o, "", String::new());
        match target {
            Target::Wavelog => {
                let Some(account) = &config.wavelog else { return plain(Outcome::Blocked("Wavelog/Cloudlog spento".into())) };
                let Some(key) = secrets.get(crate::WAVELOG_SECRET) else { return plain(Outcome::Blocked("chiave API di Wavelog/Cloudlog non impostata".into())) };
                plain(crate::wavelog::upload(&account.url, &key, &account.station_id, record, endpoints.upload_timeout, &endpoints.user_agent))
            }
            Target::Clublog => {
                let Some(account) = &config.clublog else { return (Outcome::Blocked("ClubLog spento".into()), "clublog_off", String::new()) };
                let call = if account.callsign.is_empty() { config.call.clone() } else { account.callsign.clone() };
                if let Some(token) = secrets.get(crate::CLUBLOG_TOKEN_SECRET) {
                    return crate::clublog::upload_token(&endpoints.clublog_api_url, &token, &call, record, endpoints.upload_timeout, &endpoints.user_agent);
                }
                let (Some(password), Some(api)) = (secrets.get(crate::CLUBLOG_SECRET), secrets.get(crate::CLUBLOG_API_SECRET)) else {
                    return (Outcome::Blocked("password o chiave API di ClubLog non impostate".into()), "clublog_secrets_missing", String::new());
                };
                let form = crate::clublog::Form { email: &account.email, password: &password, callsign: &call, api: &api };
                crate::clublog::upload_coded(&endpoints.clublog_url, &form, record, endpoints.upload_timeout, &endpoints.user_agent)
            }
            Target::Qrz => {
                let Some(key) = secrets.get(crate::QRZ_SECRET) else { return plain(Outcome::Blocked("chiave del logbook QRZ non impostata".into())) };
                let record = crate::qrz::force_station_call(record, &config.qrz_call);
                plain(crate::qrz::insert(&endpoints.qrz_url, &key, &record, endpoints.upload_timeout, &endpoints.user_agent))
            }
            Target::Hrdlog => {
                let Some(account) = &config.hrdlog else { return (Outcome::Blocked("HRDLOG.NET spento".into()), "hrdlog_off", String::new()) };
                let Some(code) = secrets.get(crate::HRDLOG_SECRET) else {
                    return (Outcome::Blocked("codice di caricamento di HRDLOG.NET non impostato".into()), "hrdlog_code_missing", String::new());
                };
                // nominativo in maiuscolo e codice senza spazi: incollato da una pagina web spesso porta uno spazio o un a capo in coda
                let call = if account.callsign.trim().is_empty() { config.call.clone() } else { account.callsign.clone() };
                let call = call.trim().to_uppercase();
                let code = code.trim();
                crate::hrdlog::upload_coded(&endpoints.hrdlog_url, &call, code, record, endpoints.upload_timeout, &endpoints.user_agent)
            }
            Target::Lotw => {
                let Some(setup) = &config.lotw else { return plain(Outcome::Blocked("LoTW spento".into())) };
                let Some(tqsl) = crate::lotw::find_tqsl(&setup.tqsl, &endpoints.tqsl_candidates) else {
                    return plain(Outcome::Blocked("TQSL non trovato (indica il percorso di tqsl.exe)".into()));
                };
                plain(crate::lotw::run(&tqsl, &setup.location, &records, &work_dir, id, endpoints.lotw_timeout))
            }
        }
    })
    .await
    .unwrap_or_else(|e| (Outcome::Retry(format!("invio interrotto: {e}")), "", String::new()))
}

/// Il pulsante «Prova»: controlla le credenziali **senza** registrare QSO (dove il servizio lo permette).
/// Le stazioni Wavelog/Cloudlog della chiave salvata, dall'indirizzo dato (o da quello salvato se vuoto).
pub(crate) async fn wavelog_stations(service: &Service, url: Option<String>) -> Result<Vec<crate::wavelog::Station>, String> {
    let secrets = Arc::clone(&service.secrets);
    let endpoints = service.endpoints.clone();
    let saved = service.config.borrow().wavelog.as_ref().map(|a| a.url.clone());
    tokio::task::spawn_blocking(move || {
        let url = url.filter(|u| !u.trim().is_empty()).or(saved).ok_or("scrivi l'indirizzo di Wavelog/Cloudlog")?;
        let key = secrets.get(crate::WAVELOG_SECRET).ok_or("chiave API non impostata")?;
        crate::wavelog::stations(&url, &key, endpoints.upload_timeout, &endpoints.user_agent)
    })
    .await
    .unwrap_or_else(|e| Err(format!("lettura interrotta: {e}")))
}

pub(crate) async fn test(service: &Service, target: Target) -> Result<String, String> {
    let secrets = Arc::clone(&service.secrets);
    let endpoints = service.endpoints.clone();
    let config = service.config.borrow().clone();
    tokio::task::spawn_blocking(move || match target {
        Target::Wavelog => {
            let account = config.wavelog.as_ref().ok_or("accendi l'invio e compila indirizzo e numero della stazione")?;
            let key = secrets.get(crate::WAVELOG_SECRET).ok_or("chiave API non impostata")?;
            crate::wavelog::test(&account.url, &key, &account.station_id, endpoints.upload_timeout, &endpoints.user_agent)
        }
        Target::Clublog => {
            let account = config.clublog.as_ref().ok_or("accendi l'invio e incolla il token (o compila email e nominativo)")?;
            if let Some(token) = secrets.get(crate::CLUBLOG_TOKEN_SECRET) {
                let call = if account.callsign.is_empty() { config.call.clone() } else { account.callsign.clone() };
                return crate::clublog::check_token(&endpoints.clublog_api_url, &token, &call, endpoints.upload_timeout, &endpoints.user_agent);
            }
            let password = secrets.get(crate::CLUBLOG_SECRET).ok_or("password di ClubLog non impostata")?;
            let api = secrets.get(crate::CLUBLOG_API_SECRET).ok_or("chiave API dell'applicazione non impostata")?;
            let call = if account.callsign.is_empty() { config.call.clone() } else { account.callsign.clone() };
            crate::clublog::check(&crate::clublog::Form { email: &account.email, password: &password, callsign: &call, api: &api })
        }
        Target::Qrz => {
            if !config.qrz {
                return Err("accendi l'invio a QRZ".to_string());
            }
            let key = secrets.get(crate::QRZ_SECRET).ok_or("chiave del logbook non impostata")?;
            crate::qrz::status(&endpoints.qrz_url, &key, endpoints.upload_timeout, &endpoints.user_agent)
        }
        Target::Hrdlog => {
            let account = config.hrdlog.as_ref().ok_or("accendi l'invio a HRDLOG.NET")?;
            let call = if account.callsign.is_empty() { config.call.clone() } else { account.callsign.clone() };
            crate::hrdlog::check(&call, secrets.has(crate::HRDLOG_SECRET))
        }
        Target::Lotw => {
            let setup = config.lotw.as_ref().ok_or("accendi l'invio e scrivi la posizione di stazione di TQSL")?;
            crate::lotw::check(&setup.tqsl, &setup.location, &endpoints.tqsl_candidates, endpoints.tqsl_station_data.as_deref())
        }
    })
    .await
    .unwrap_or_else(|e| Err(format!("prova interrotta: {e}")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_delays_grow_and_stop_at_one_hour() {
        let d: Vec<u64> = (1..=8).map(retry_delay).collect();
        assert_eq!(d, vec![60, 120, 300, 600, 1800, 3600, 3600, 3600]);
    }

    #[test]
    fn the_queue_keeps_order_retries_later_and_survives_a_restart() {
        let dir = std::env::temp_dir().join(format!("aurora-upload-queue-{}", std::process::id()));
        std::fs::remove_dir_all(&dir).ok();
        let file = dir.join("invii-in-coda.json");
        let q = Queue::open(Some(file.clone()));
        q.push(Target::Qrz, "K1ABC", "<CALL:5>K1ABC <EOR>".into(), 1000);
        q.push(Target::Qrz, "K2ABC", "<CALL:5>K2ABC <EOR>".into(), 1001);
        q.push(Target::Lotw, "K1ABC", "<CALL:5>K1ABC <EOR>".into(), 1000);
        let due = q.due(Target::Qrz, 1001, 1);
        assert_eq!(due.iter().map(|p| p.call.as_str()).collect::<Vec<_>>(), vec!["K1ABC"]);
        let at = q.later(&[due[0].id], 1001);
        assert_eq!(at, 1061);
        assert_eq!(q.due(Target::Qrz, 1001, 5).len(), 1, "il primo aspetta, il secondo e' pronto");
        assert_eq!(q.next_due(Target::Qrz), Some(1001));
        // dopo un riavvio la coda c'e' ancora, senza credenziali
        let again = Queue::open(Some(file.clone()));
        assert_eq!((again.count(Target::Qrz), again.count(Target::Lotw)), (2, 1));
        let text = std::fs::read_to_string(&file).unwrap();
        assert!(text.contains("K2ABC") && !text.to_lowercase().contains("key"), "{text}");
        again.retry_now(Some(Target::Qrz), 1002);
        assert_eq!(again.due(Target::Qrz, 1002, 5).len(), 2);
        again.remove(&[due[0].id]);
        assert_eq!(again.count(Target::Qrz), 1);
        // sette giorni dopo si scarta
        assert_eq!(again.expire(Target::Qrz, 1001 + MAX_AGE_SECS + 1), vec!["K2ABC".to_string()]);
        assert_eq!((again.count(Target::Qrz), again.count(Target::Lotw)), (0, 1));
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn blocking_is_per_service() {
        let q = Queue::open(None);
        q.set_blocked(Target::Clublog, true);
        assert!(q.is_blocked(Target::Clublog) && !q.is_blocked(Target::Qrz));
        q.set_blocked(Target::Clublog, false);
        assert!(!q.is_blocked(Target::Clublog));
    }

    #[test]
    fn service_texts_are_cleaned_and_shortened() {
        assert_eq!(short("<html><body>\n QSO   OK\r\n</body></html>", 50), "QSO OK");
        assert_eq!(short("abcdef", 3), "abc");
        assert_eq!(Target::parse("lotw"), Some(Target::Lotw));
        assert_eq!(Target::parse("x"), None);
    }
}
