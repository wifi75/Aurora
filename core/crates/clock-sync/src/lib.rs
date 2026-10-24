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
}

impl Default for Config {
    fn default() -> Config {
        Config {
            enabled: true,
            servers: ["it.pool.ntp.org", "ntp1.inrim.it", "time.cloudflare.com", "time.windows.com"].map(String::from).to_vec(),
            interval_secs: 900,
            warn_ms: 200,
            auto_correct: false,
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

/// Cambia l'ora con PowerShell (`Set-Date`): serve avviare Aurora come amministratore.
pub struct PowerShellSetter;

impl ClockSetter for PowerShellSetter {
    fn adjust(&self, offset_ms: f64) -> Result<(), String> {
        if !offset_ms.is_finite() || offset_ms.abs() > 3_600_000.0 {
            return Err("scarto fuori misura: non si corregge da solo".into());
        }
        // il numero e' un f64 formattato qui: nessun testo dell'utente nello script
        let script = format!("Set-Date -Date ([DateTime]::Now.AddMilliseconds({offset_ms:.0})) | Out-Null");
        let out = std::process::Command::new("powershell")
            .args(["-NoProfile", "-NonInteractive", "-Command", &script])
            .output()
            .map_err(|e| format!("PowerShell non partito: {e}"))?;
        if out.status.success() {
            Ok(())
        } else {
            Err("serve avviare Aurora come amministratore per cambiare l'ora del PC".into())
        }
    }
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
            .unwrap_or_default();
        Arc::new(ClockService { path: path.to_path_buf(), config: Mutex::new(config), last: Mutex::new(None), query, setter, wake: Notify::new() })
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
        Ok(after)
    }

    /// Controlla a intervalli regolari (e subito quando le impostazioni cambiano), anche correggendo se richiesto.
    pub fn spawn(self: &Arc<Self>) -> tokio::task::JoinHandle<()> {
        let service = Arc::clone(self);
        tokio::spawn(async move {
            loop {
                let config = service.config();
                if config.enabled {
                    let worker = Arc::clone(&service);
                    let outcome = tokio::task::spawn_blocking(move || {
                        let m = worker.check_now();
                        let cfg = worker.config();
                        match m.offset_ms {
                            Some(off) if cfg.auto_correct && off.abs() > f64::from(cfg.warn_ms) => worker.correct_now().map(|_| ()).map_err(|e| format!("correzione: {e}")),
                            _ => Ok(()),
                        }
                    })
                    .await;
                    if let Ok(Err(e)) = outcome {
                        tracing::warn!("orologio: {e}");
                    }
                }
                tokio::select! {
                    () = tokio::time::sleep(Duration::from_secs(config.interval_secs)) => {}
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
        let mut ok = Config::default();
        ok.servers = vec!["192.168.1.1".into(), "time.example.org:1230".into()];
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
        let mut c = Config::default();
        c.servers = vec!["lento".into(), "rotto".into(), "veloce".into()];
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
}
