//! Client NTP di Aurora: tiene d'occhio lo scarto dell'orologio del PC (per FT8/FT4 serve entro il mezzo secondo,
//! meglio entro 0,2 s) e, se l'operatore lo vuole, lo corregge. Tutto si imposta dalle impostazioni.

pub mod ntp;

use std::{
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use serde::{Deserialize, Serialize};
use tokio::sync::Notify;

pub const MIN_INTERVAL_SECS: u64 = 60;
pub const MAX_INTERVAL_SECS: u64 = 86_400;
pub const MIN_WARN_MS: u32 = 10;
pub const MAX_WARN_MS: u32 = 5_000;
pub const MAX_SERVERS: usize = 8;
const QUERY_TIMEOUT: Duration = Duration::from_secs(2);

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Config {
    /// Controllo periodico acceso.
    pub enabled: bool,
    /// Server NTP: `nome` o `nome:porta`.
    pub servers: Vec<String>,
    /// Ogni quanti secondi si controlla.
    pub interval_secs: u64,
    /// Oltre questo scarto (millisecondi) l'orologio e' segnalato, ed eventualmente corretto.
    pub warn_ms: u32,
    /// Correggi da solo l'orologio quando lo scarto supera la soglia (serve poter cambiare l'ora del PC).
    pub auto_correct: bool,
    /// All'avvio di Aurora: misura subito e, se l'ora e' fuori tolleranza, la corregge (serve poter cambiare l'ora del PC).
    #[serde(default = "yes")]
    pub correct_at_start: bool,
}

fn yes() -> bool {
    true
}

impl Default for Config {
    fn default() -> Config {
        Config {
            enabled: true,
            servers: ["it.pool.ntp.org", "ntp1.inrim.it", "time.cloudflare.com", "time.windows.com"].map(String::from).to_vec(),
            interval_secs: 900,
            // FT8 tollera bene qualche decimo di secondo: sotto il mezzo secondo l'avviso spaventava senza motivo (l'orologio di Windows
            // deriva di centinaia di ms tra una sincronizzazione e l'altra)
            warn_ms: 500,
            auto_correct: false,
            correct_at_start: true,
        }
    }
}

fn valid_server(server: &str) -> bool {
    let (host, port) = match server.split_once(':') {
        Some((h, p)) => (h, Some(p)),
        None => (server, None),
    };
    let host_ok = !host.is_empty() && host.len() <= 253 && host.chars().all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '-');
    let port_ok = port.is_none_or(|p| p.parse::<u16>().is_ok_and(|n| n > 0));
    host_ok && port_ok
}

impl Config {
    pub fn validate(&self) -> Result<(), String> {
        if self.servers.is_empty() || self.servers.len() > MAX_SERVERS {
            return Err(format!("servers: da 1 a {MAX_SERVERS} server"));
        }
        if let Some(bad) = self.servers.iter().find(|s| !valid_server(s)) {
            return Err(format!("servers: {bad:?} non e' un nome di server valido"));
        }
        if !(MIN_INTERVAL_SECS..=MAX_INTERVAL_SECS).contains(&self.interval_secs) {
            return Err(format!("interval_secs: tra {MIN_INTERVAL_SECS} e {MAX_INTERVAL_SECS}"));
        }
        if !(MIN_WARN_MS..=MAX_WARN_MS).contains(&self.warn_ms) {
            return Err(format!("warn_ms: tra {MIN_WARN_MS} e {MAX_WARN_MS}"));
        }
        Ok(())
    }
}

/// L'ultimo controllo.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Measurement {
    /// Quando e' stato fatto (millisecondi Unix, dall'orologio del PC).
    pub at_ms: u64,
    /// Server scelto: quello con il ritardo di rete piu' basso.
    pub server: Option<String>,
    /// Quanto l'orologio del PC e' indietro (+) o avanti (-), in millisecondi.
    pub offset_ms: Option<f64>,
    pub delay_ms: Option<f64>,
    pub servers_ok: usize,
    pub servers_tried: usize,
    /// Perche' non e' riuscito, se non e' riuscito.
    pub error: Option<String>,
    /// L'orologio e' stato corretto con questo controllo.
    pub corrected: bool,
}

/// Come si interroga un server e come si cambia l'ora del PC: due punti di aggancio, finti nei test.
pub type QueryFn = Arc<dyn Fn(&str) -> Result<ntp::Sample, String> + Send + Sync>;

pub trait ClockSetter: Send + Sync {
    /// Sposta l'orologio del PC di `offset_ms` (positivo = avanti).
    fn adjust(&self, offset_ms: f64) -> Result<(), String>;
}

/// Cambia l'ora del PC con le API di Windows (`SetSystemTime`, dopo aver acceso il privilegio `SeSystemtimePrivilege`), senza PowerShell:
/// `Set-Date` passava da `powershell.exe`, che antivirus e regole aziendali possono bloccare. Serve comunque un utente che possa cambiare
/// l'ora (amministratore): altrimenti Windows non concede il privilegio e lo si dice in chiaro.
pub struct SystemSetter;

impl ClockSetter for SystemSetter {
    fn adjust(&self, offset_ms: f64) -> Result<(), String> {
        if !offset_ms.is_finite() || offset_ms.abs() > 3_600_000.0 {
            return Err("scarto fuori misura: non si corregge da solo".into());
        }
        set_clock(offset_ms)
    }
}

#[cfg(windows)]
fn set_clock(offset_ms: f64) -> Result<(), String> {
    use windows::Win32::Foundation::{CloseHandle, ERROR_NOT_ALL_ASSIGNED, GetLastError, HANDLE, LUID};
    use windows::Win32::Security::{AdjustTokenPrivileges, LUID_AND_ATTRIBUTES, LookupPrivilegeValueW, SE_PRIVILEGE_ENABLED, SE_SYSTEMTIME_NAME, TOKEN_ADJUST_PRIVILEGES, TOKEN_PRIVILEGES, TOKEN_QUERY};
    use windows::Win32::System::SystemInformation::{GetSystemTimeAsFileTime, SetSystemTime};
    use windows::Win32::System::Threading::{GetCurrentProcess, OpenProcessToken};
    use windows::Win32::System::Time::FileTimeToSystemTime;
    use windows::Win32::Foundation::{FILETIME, SYSTEMTIME};
    const ADMIN: &str = "serve avviare Aurora come amministratore per cambiare l'ora del PC";
    // SAFETY: chiamate Win32 con puntatori a variabili locali valide per tutta la durata della chiamata; il token si chiude sempre.
    unsafe {
        let mut token = HANDLE::default();
        OpenProcessToken(GetCurrentProcess(), TOKEN_ADJUST_PRIVILEGES | TOKEN_QUERY, &mut token).map_err(|e| format!("token del processo: {e}"))?;
        let mut luid = LUID::default();
        let looked = LookupPrivilegeValueW(None, SE_SYSTEMTIME_NAME, &mut luid);
        let granted = looked.is_ok() && {
            let privileges = TOKEN_PRIVILEGES { PrivilegeCount: 1, Privileges: [LUID_AND_ATTRIBUTES { Luid: luid, Attributes: SE_PRIVILEGE_ENABLED }] };
            AdjustTokenPrivileges(token, false, Some(&privileges), 0, None, None).is_ok() && GetLastError() != ERROR_NOT_ALL_ASSIGNED
        };
        let _ = CloseHandle(token);
        if !granted {
            return Err(ADMIN.into());
        }
        let now: FILETIME = GetSystemTimeAsFileTime();
        let ticks = (u64::from(now.dwHighDateTime) << 32) | u64::from(now.dwLowDateTime);
        // FILETIME conta centinaia di nanosecondi: 1 ms = 10 000
        let shifted = i128::from(ticks) + (offset_ms * 10_000.0).round() as i128;
        let shifted = u64::try_from(shifted).map_err(|_| "ora fuori misura".to_string())?;
        let target = FILETIME { dwLowDateTime: shifted as u32, dwHighDateTime: (shifted >> 32) as u32 };
        let mut st = SYSTEMTIME::default();
        FileTimeToSystemTime(&target, &mut st).map_err(|e| format!("conversione dell'ora: {e}"))?;
        SetSystemTime(&st).map_err(|_| ADMIN.to_string())
    }
}

#[cfg(not(windows))]
fn set_clock(_offset_ms: f64) -> Result<(), String> {
    Err("su questo sistema Aurora non cambia l'ora del PC: usa la sincronizzazione del sistema (NTP)".into())
}

/// Tentativi della correzione all'avvio (uno ogni [`START_RETRY`]) finche' i server NTP non rispondono: circa dieci minuti.
const START_TRIES: u32 = 30;
const START_RETRY: Duration = Duration::from_secs(20);
/// Mentre l'ora e' fuori tolleranza (o non si e' potuta misurare) si ricontrolla ogni minuto: appena l'utente la sincronizza (anche da
/// Windows) l'avviso sparisce, invece di restare fino al controllo seguente.
const RECHECK_WHILE_OFF: Duration = Duration::from_secs(60);

fn next_wait(config: &Config, start_pending: bool, last: Option<&Measurement>) -> Duration {
    let normal = Duration::from_secs(config.interval_secs);
    if start_pending {
        return START_RETRY;
    }
    if !config.enabled {
        return normal;
    }
    let off = last.is_none_or(|m| m.offset_ms.is_none_or(|o| o.abs() > f64::from(config.warn_ms)));
    if off { normal.min(RECHECK_WHILE_OFF) } else { normal }
}

pub fn default_query() -> QueryFn {
    Arc::new(|server: &str| ntp::query(server, QUERY_TIMEOUT))
}

fn unix_ms() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |d| d.as_millis() as u64)
}

pub struct ClockService {
    path: PathBuf,
    config: Mutex<Config>,
    last: Mutex<Option<Measurement>>,
    /// L'ultima volta (ms Unix) che l'ora risultava giusta (entro la tolleranza o appena corretta): si ricorda tra un avvio e l'altro, cosi'
    /// l'avviso sa da quanti giorni l'orologio non si e' piu' potuto verificare.
    last_good: Mutex<Option<u64>>,
    query: QueryFn,
    setter: Arc<dyn ClockSetter>,
    wake: Notify,
}

impl ClockService {
    /// Legge la configurazione salvata; se manca o e' rovinata parte dai valori predefiniti.
    pub fn open(path: &Path, query: QueryFn, setter: Arc<dyn ClockSetter>) -> Arc<ClockService> {
        let config = std::fs::read_to_string(path)
            .ok()
            .and_then(|text| serde_json::from_str::<Config>(&text).ok())
            .filter(|c| c.validate().is_ok())
            // la soglia vecchia di fabbrica (200 ms) passa alla nuova; una scelta diversa dell'utente resta
            .map(|mut c| {
                if c.warn_ms == 200 {
                    c.warn_ms = 500;
                }
                c
            })
            .unwrap_or_default();
        let last_good = std::fs::read_to_string(path.with_extension("good")).ok().and_then(|t| t.trim().parse::<u64>().ok());
        Arc::new(ClockService { path: path.to_path_buf(), config: Mutex::new(config), last: Mutex::new(None), last_good: Mutex::new(last_good), query, setter, wake: Notify::new() })
    }

    /// Quando l'ora e' risultata giusta l'ultima volta (ms Unix), anche in un avvio precedente.
    pub fn last_good_ms(&self) -> Option<u64> {
        *self.last_good.lock().unwrap_or_else(|p| p.into_inner())
    }

    fn note(&self, m: &Measurement) {
        let good = m.corrected || m.offset_ms.is_some_and(|off| off.abs() <= f64::from(self.config().warn_ms));
        if !good {
            return;
        }
        *self.last_good.lock().unwrap_or_else(|p| p.into_inner()) = Some(m.at_ms);
        if let Err(e) = std::fs::write(self.path.with_extension("good"), m.at_ms.to_string()) {
            tracing::warn!("orologio: ultima verifica non salvata ({e})");
        }
    }

    pub fn config(&self) -> Config {
        self.config.lock().unwrap_or_else(|p| p.into_inner()).clone()
    }

    pub fn last(&self) -> Option<Measurement> {
        self.last.lock().unwrap_or_else(|p| p.into_inner()).clone()
    }

    /// Valida, salva su disco e fa ripartire il controllo con i valori nuovi.
    pub fn set_config(&self, config: Config) -> Result<(), String> {
        config.validate()?;
        let text = serde_json::to_string_pretty(&config).map_err(|e| e.to_string())?;
        let tmp = self.path.with_extension("json.tmp");
        std::fs::write(&tmp, text).and_then(|()| std::fs::rename(&tmp, &self.path)).map_err(|e| format!("salvataggio: {e}"))?;
        *self.config.lock().unwrap_or_else(|p| p.into_inner()) = config;
        self.wake.notify_one();
        Ok(())
    }

    /// Interroga tutti i server (in modo bloccante) e tiene la misura con il ritardo di rete piu' basso.
    pub fn check_now(&self) -> Measurement {
        let servers = self.config().servers;
        let mut best: Option<(String, ntp::Sample)> = None;
        let mut ok = 0;
        let mut errors = Vec::new();
        for server in &servers {
            match (self.query)(server) {
                Ok(sample) => {
                    ok += 1;
                    if best.as_ref().is_none_or(|(_, b)| sample.delay_ms < b.delay_ms) {
                        best = Some((server.clone(), sample));
                    }
                }
                Err(e) => errors.push(e),
            }
        }
        let measurement = Measurement {
            at_ms: unix_ms(),
            server: best.as_ref().map(|(s, _)| s.clone()),
            offset_ms: best.as_ref().map(|(_, s)| s.offset_ms),
            delay_ms: best.as_ref().map(|(_, s)| s.delay_ms),
            servers_ok: ok,
            servers_tried: servers.len(),
            error: if best.is_none() { Some(errors.join("; ")) } else { None },
            corrected: false,
        };
        *self.last.lock().unwrap_or_else(|p| p.into_inner()) = Some(measurement.clone());
        self.note(&measurement);
        measurement
    }

    /// Misura, sposta l'orologio del PC dello scarto trovato e misura di nuovo.
    pub fn correct_now(&self) -> Result<Measurement, String> {
        let before = self.check_now();
        let offset = before.offset_ms.ok_or_else(|| before.error.clone().unwrap_or_else(|| "nessuna misura".into()))?;
        self.setter.adjust(offset)?;
        let mut after = self.check_now();
        after.corrected = true;
        *self.last.lock().unwrap_or_else(|p| p.into_inner()) = Some(after.clone());
        self.note(&after);
        Ok(after)
    }

    /// Controlla a intervalli regolari (e subito quando le impostazioni cambiano), anche correggendo se richiesto.
    pub fn spawn(self: &Arc<Self>) -> tokio::task::JoinHandle<()> {
        let service = Arc::clone(self);
        tokio::spawn(async move {
            // la correzione dell'avvio resta in attesa finche' non arriva una misura: all'avvio di Windows la rete spesso non c'e' ancora, e
            // prima il primo giro falliva e si riprovava solo dopo `interval_secs` (15 minuti), con l'avviso dell'orologio sullo schermo
            let mut start_pending = service.config().correct_at_start;
            let mut start_tries = 0u32;
            loop {
                let config = service.config();
                let at_start = start_pending;
                if config.enabled || at_start {
                    let worker = Arc::clone(&service);
                    let outcome = tokio::task::spawn_blocking(move || {
                        let m = worker.check_now();
                        let cfg = worker.config();
                        let measured = m.offset_ms.is_some();
                        let result = match m.offset_ms {
                            Some(off) if (cfg.auto_correct || at_start) && off.abs() > f64::from(cfg.warn_ms) => worker.correct_now().map(|_| ()).map_err(|e| format!("correzione: {e}")),
                            _ => Ok(()),
                        };
                        (measured, result)
                    })
                    .await;
                    let measured = matches!(outcome, Ok((true, _)));
                    if let Ok((_, Err(e))) = outcome {
                        tracing::warn!("orologio: {e}");
                    }
                    if at_start {
                        start_tries += 1;
                        if measured || start_tries >= START_TRIES {
                            start_pending = false;
                        }
                    }
                }
                let wait = next_wait(&config, start_pending, service.last().as_ref());
                tokio::select! {
                    () = tokio::time::sleep(wait) => {}
                    () = service.wake.notified() => {}
                }
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    struct FakeSetter {
        calls: Mutex<Vec<f64>>,
        fail: bool,
    }

    impl ClockSetter for FakeSetter {
        fn adjust(&self, offset_ms: f64) -> Result<(), String> {
            self.calls.lock().unwrap().push(offset_ms);
            if self.fail { Err("serve avviare Aurora come amministratore".into()) } else { Ok(()) }
        }
    }

    fn temp(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("aurora-clock-{name}-{}", std::process::id()));
        std::fs::remove_dir_all(&dir).ok();
        std::fs::create_dir_all(&dir).unwrap();
        dir.join("clock.json")
    }

    fn sample(offset_ms: f64, delay_ms: f64) -> ntp::Sample {
        ntp::Sample { offset_ms, delay_ms, stratum: 2 }
    }

    #[test]
    fn the_defaults_are_valid_and_the_limits_are_enforced() {
        assert!(Config::default().validate().is_ok());
        let bad = |f: &dyn Fn(&mut Config)| {
            let mut c = Config::default();
            f(&mut c);
            c.validate().is_err()
        };
        assert!(bad(&|c| c.servers.clear()));
        assert!(bad(&|c| c.servers = vec!["a b".into()]));
        assert!(bad(&|c| c.servers = vec!["host:0".into()]));
        assert!(bad(&|c| c.servers = vec!["; calc".into()]));
        assert!(bad(&|c| c.servers = (0..9).map(|i| format!("s{i}.example.org")).collect()));
        assert!(bad(&|c| c.interval_secs = 5));
        assert!(bad(&|c| c.warn_ms = 1));
        let ok = Config { servers: vec!["192.168.1.1".into(), "time.example.org:1230".into()], ..Config::default() };
        assert!(ok.validate().is_ok());
    }

    #[test]
    fn keeps_the_sample_with_the_lowest_network_delay() {
        let query: QueryFn = Arc::new(|server: &str| match server {
            "lento" => Ok(sample(300.0, 90.0)),
            "veloce" => Ok(sample(120.0, 12.0)),
            _ => Err(format!("{server}: nessuna risposta")),
        });
        let service = ClockService::open(&temp("best"), query, Arc::new(FakeSetter { calls: Mutex::default(), fail: false }));
        let c = Config { servers: vec!["lento".into(), "rotto".into(), "veloce".into()], ..Config::default() };
        service.set_config(c).unwrap();
        let m = service.check_now();
        assert_eq!(m.server.as_deref(), Some("veloce"));
        assert_eq!(m.offset_ms, Some(120.0));
        assert_eq!((m.servers_ok, m.servers_tried), (2, 3));
        assert!(m.error.is_none());
        assert_eq!(service.last(), Some(m));
    }

    #[test]
    fn when_no_server_answers_it_says_why() {
        let query: QueryFn = Arc::new(|s: &str| Err(format!("{s}: nessuna risposta")));
        let service = ClockService::open(&temp("none"), query, Arc::new(FakeSetter { calls: Mutex::default(), fail: false }));
        let m = service.check_now();
        assert!(m.server.is_none() && m.offset_ms.is_none());
        assert!(m.error.unwrap().contains("nessuna risposta"));
    }

    #[test]
    fn correcting_moves_the_clock_by_the_measured_offset_and_measures_again() {
        let calls = Arc::new(AtomicUsize::new(0));
        let seen = Arc::clone(&calls);
        let query: QueryFn = Arc::new(move |_: &str| {
            // prima misura: 400 ms indietro; dopo la correzione: 3 ms
            Ok(if seen.fetch_add(1, Ordering::SeqCst) < 4 { sample(400.0, 10.0) } else { sample(3.0, 10.0) })
        });
        let setter = Arc::new(FakeSetter { calls: Mutex::default(), fail: false });
        let service = ClockService::open(&temp("fix"), query, setter.clone());
        let after = service.correct_now().unwrap();
        assert_eq!(*setter.calls.lock().unwrap(), vec![400.0]);
        assert!(after.corrected);
        assert_eq!(after.offset_ms, Some(3.0));
    }

    #[test]
    fn a_refusal_of_the_system_is_reported_and_nothing_is_marked_corrected() {
        let query: QueryFn = Arc::new(|_: &str| Ok(sample(400.0, 10.0)));
        let setter = Arc::new(FakeSetter { calls: Mutex::default(), fail: true });
        let service = ClockService::open(&temp("deny"), query, setter);
        let err = service.correct_now().unwrap_err();
        assert!(err.contains("amministratore"));
        assert!(!service.last().unwrap().corrected);
    }

    #[test]
    fn the_configuration_is_saved_and_survives_a_restart() {
        let path = temp("persist");
        let query: QueryFn = Arc::new(|_: &str| Err("x".into()));
        let setter = Arc::new(FakeSetter { calls: Mutex::default(), fail: false });
        let service = ClockService::open(&path, query.clone(), setter.clone());
        let mut c = service.config();
        c.servers = vec!["ntp.example.org".into()];
        c.interval_secs = 300;
        c.warn_ms = 150;
        c.auto_correct = true;
        c.enabled = false;
        service.set_config(c.clone()).unwrap();
        assert_eq!(ClockService::open(&path, query, setter).config(), c);
        assert!(service.set_config(Config { warn_ms: 0, ..c }).is_err(), "valori fuori limite: non si salva");
    }

    #[test]
    fn a_broken_file_gives_the_defaults() {
        let path = temp("broken");
        std::fs::write(&path, "non e' json").unwrap();
        let query: QueryFn = Arc::new(|_: &str| Err("x".into()));
        let service = ClockService::open(&path, query, Arc::new(FakeSetter { calls: Mutex::default(), fail: false }));
        assert_eq!(service.config(), Config::default());
    }

    #[test]
    fn the_check_comes_back_soon_while_the_clock_is_off_or_the_start_correction_waits() {
        let config = Config::default();
        let m = |off: Option<f64>| Measurement { at_ms: 0, server: None, offset_ms: off, delay_ms: None, servers_ok: 0, servers_tried: 1, error: None, corrected: false };
        assert_eq!(next_wait(&config, true, None), START_RETRY, "correzione dell'avvio in attesa della rete");
        assert_eq!(next_wait(&config, false, Some(&m(Some(729.0)))), RECHECK_WHILE_OFF, "ora fuori tolleranza: si ricontrolla ogni minuto");
        assert_eq!(next_wait(&config, false, Some(&m(None))), RECHECK_WHILE_OFF, "misura non riuscita");
        assert_eq!(next_wait(&config, false, Some(&m(Some(20.0)))), Duration::from_secs(config.interval_secs), "ora buona: intervallo normale");
    }
}
