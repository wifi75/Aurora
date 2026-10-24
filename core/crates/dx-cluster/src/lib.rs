//! Client DX Cluster di Aurora: si collega via telnet a uno o piu' cluster (pubblici o propri), entra con il
//! nominativo, raccoglie gli spot e li tiene in memoria per l'interfaccia. Tutto si imposta dalle impostazioni.

pub mod spot;
pub mod telnet;

use std::{
    collections::{BTreeMap, VecDeque},
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc, Mutex,
    },
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use serde::{Deserialize, Serialize};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
    sync::mpsc,
    task::JoinHandle,
};

pub const MAX_SERVERS: usize = 8;
pub const MAX_COMMANDS: usize = 10;
pub const MIN_SPOTS: usize = 50;
pub const MAX_SPOTS: usize = 2_000;
pub const MIN_AGE_MIN: u32 = 5;
pub const MAX_AGE_MIN: u32 = 1_440;
const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);
const KEEPALIVE: Duration = Duration::from_secs(240);
const MAX_LINE: usize = 2_000;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ServerCfg {
    pub name: String,
    pub host: String,
    pub port: u16,
    pub enabled: bool,
}

impl ServerCfg {
    /// Chiave che lo identifica: `host:porta`.
    pub fn key(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Config {
    /// Collegamento acceso. Spento per impostazione predefinita: si accende dalle impostazioni.
    pub enabled: bool,
    /// Nominativo con cui si entra nei cluster.
    pub callsign: String,
    pub servers: Vec<ServerCfg>,
    /// Comandi mandati al cluster dopo l'accesso (per esempio `set/ft8` o filtri).
    pub login_commands: Vec<String>,
    /// Quanti spot tenere in memoria.
    pub max_spots: usize,
    /// Dopo quanti minuti uno spot si scarta.
    pub max_age_min: u32,
}

impl Default for Config {
    fn default() -> Config {
        let server = |name: &str, host: &str, port: u16| ServerCfg { name: name.into(), host: host.into(), port, enabled: true };
        Config {
            enabled: false,
            callsign: String::new(),
            servers: vec![server("HamQTH", "hamqth.com", 7300), server("N2KI New York", "dxc.n2ki.com", 7300), server("Hamcloud", "dx.hamnet.network", 7300)],
            login_commands: Vec::new(),
            max_spots: 500,
            max_age_min: 60,
        }
    }
}

fn valid_host(host: &str) -> bool {
    !host.is_empty() && host.len() <= 253 && host.chars().all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '-')
}

/// Nominativo: lettere, cifre, barra e trattino, 3-16 caratteri, con almeno una cifra.
pub fn valid_callsign(call: &str) -> bool {
    (3..=16).contains(&call.len())
        && call.chars().all(|c| c.is_ascii_alphanumeric() || c == '/' || c == '-')
        && call.chars().any(|c| c.is_ascii_digit())
}

/// Una riga di caratteri stampabili.
fn plain_line(text: &str) -> bool {
    let t = text.trim();
    !t.is_empty() && t.len() <= 80 && t.chars().all(|c| c.is_ascii() && !c.is_ascii_control())
}

/// Comandi che Aurora manda al cluster: solo consultazione e impostazioni (`sh/dx`, `set/ft8`, filtri...).
/// Mai spot, annunci o messaggi: sarebbero pubblici e a nome dell'operatore.
const ALLOWED_PREFIXES: &[&str] = &["sh/", "show/", "sh ", "show ", "set/", "unset/", "accept/", "reject/", "clear/", "help", "bye"];

pub fn valid_command(text: &str) -> bool {
    let lower = text.trim().to_ascii_lowercase();
    plain_line(text) && ALLOWED_PREFIXES.iter().any(|p| lower.starts_with(p))
}

impl Config {
    pub fn validate(&self) -> Result<(), String> {
        if !self.callsign.is_empty() && !valid_callsign(&self.callsign) {
            return Err("callsign: non e' un nominativo valido".into());
        }
        if self.servers.len() > MAX_SERVERS {
            return Err(format!("servers: al massimo {MAX_SERVERS}"));
        }
        for s in &self.servers {
            if s.name.trim().is_empty() || s.name.len() > 40 || s.name.chars().any(char::is_control) {
                return Err("servers: nome non valido".into());
            }
            if !valid_host(&s.host) || s.port == 0 {
                return Err(format!("servers: {}:{} non e' un indirizzo valido", s.host, s.port));
            }
        }
        if self.login_commands.len() > MAX_COMMANDS || self.login_commands.iter().any(|c| !valid_command(c)) {
            return Err(format!("login_commands: al massimo {MAX_COMMANDS} comandi di testo semplice, 80 caratteri"));
        }
        if !(MIN_SPOTS..=MAX_SPOTS).contains(&self.max_spots) {
            return Err(format!("max_spots: tra {MIN_SPOTS} e {MAX_SPOTS}"));
        }
        if !(MIN_AGE_MIN..=MAX_AGE_MIN).contains(&self.max_age_min) {
            return Err(format!("max_age_min: tra {MIN_AGE_MIN} e {MAX_AGE_MIN}"));
        }
        Ok(())
    }
}

/// Un server pubblico noto, da aggiungere con un clic. Verificati raggiungibili il 2026-10-24.
#[derive(Debug, Clone, Serialize)]
pub struct CatalogEntry {
    pub name: &'static str,
    pub host: &'static str,
    pub port: u16,
    pub software: &'static str,
    pub place: &'static str,
    pub note: &'static str,
}

const fn entry(name: &'static str, host: &'static str, port: u16, software: &'static str, place: &'static str, note: &'static str) -> CatalogEntry {
    CatalogEntry { name, host, port, software, place, note }
}

pub const CATALOG: &[CatalogEntry] = &[
    entry("HamQTH", "hamqth.com", 7300, "DXSpider", "Repubblica Ceca", "Cluster molto frequentato, ottimo per l'Europa"),
    entry("N2KI New York", "dxc.n2ki.com", 7300, "DXSpider", "New York, USA", "Per inviare spot serve la registrazione"),
    entry("HamServe UK", "dxc.hamserve.uk", 7300, "DXSpider", "Regno Unito", ""),
    entry("Hamcloud", "dx.hamnet.network", 7300, "DXSpider", "Germania", "Dal 2025 raccoglie anche il vecchio DB0LJ"),
    entry("VE7CC", "ve7cc.net", 23, "CC Cluster", "Vancouver, Canada", "Molto veloce, spot filtrati dagli skimmer"),
    entry("NC7J", "dxc.nc7j.com", 7373, "AR-Cluster", "Stati Uniti", ""),
    entry("W3LPL", "dxc.w3lpl.net", 7373, "AR-Cluster", "Stati Uniti", "Molto trafficato"),
    entry("AE5E", "dxspots.com", 7300, "CC Cluster", "Minnesota, USA", ""),
    entry("DXFun", "dxfun.com", 8000, "DXSpider", "Internazionale", ""),
    entry("Reverse Beacon Network (CW, RTTY)", "telnet.reversebeacon.net", 7000, "RBN", "Rete mondiale di skimmer", "Spot automatici, non di persone"),
    entry("Reverse Beacon Network (FT8, FT4)", "telnet.reversebeacon.net", 7001, "RBN", "Rete mondiale di skimmer", "Spot automatici dei segnali FT8 e FT4"),
];

/// Uno spot con i dati del servizio.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Spot {
    /// Numero progressivo: l'interfaccia chiede "da questo in poi".
    pub id: u64,
    pub received_ms: u64,
    pub server: String,
    pub spotter: String,
    pub freq_khz: f64,
    pub dx_call: String,
    pub comment: String,
    pub time_utc: String,
    pub grid: Option<String>,
    pub band: Option<&'static str>,
    pub mode: Option<String>,
    pub country: Option<String>,
    pub continent: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ServerStatus {
    pub key: String,
    pub name: String,
    /// `connecting`, `connected` o `error`.
    pub state: &'static str,
    pub detail: Option<String>,
    pub spots: u64,
    pub since_ms: u64,
}

/// Dato un nominativo, ne dice paese e continente (dal file cty.dat).
pub type Enricher = Arc<dyn Fn(&str) -> Option<(String, String)> + Send + Sync>;

fn unix_ms() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |d| d.as_millis() as u64)
}

pub struct ClusterService {
    path: PathBuf,
    config: Mutex<Config>,
    spots: Mutex<VecDeque<Spot>>,
    next_id: AtomicU64,
    status: Mutex<BTreeMap<String, ServerStatus>>,
    senders: Mutex<BTreeMap<String, mpsc::UnboundedSender<String>>>,
    tasks: Mutex<Vec<JoinHandle<()>>>,
    enricher: Mutex<Option<Enricher>>,
}

fn lock<T>(m: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|p| p.into_inner())
}

impl ClusterService {
    /// Legge la configurazione salvata; se manca o e' rovinata parte dai valori predefiniti. Non si collega a nulla.
    pub fn open(path: &Path) -> Arc<ClusterService> {
        let config = std::fs::read_to_string(path)
            .ok()
            .and_then(|text| serde_json::from_str::<Config>(&text).ok())
            .filter(|c| c.validate().is_ok())
            .unwrap_or_default();
        Arc::new(ClusterService {
            path: path.to_path_buf(),
            config: Mutex::new(config),
            spots: Mutex::new(VecDeque::new()),
            next_id: AtomicU64::new(1),
            status: Mutex::new(BTreeMap::new()),
            senders: Mutex::new(BTreeMap::new()),
            tasks: Mutex::new(Vec::new()),
            enricher: Mutex::new(None),
        })
    }

    pub fn config(&self) -> Config {
        lock(&self.config).clone()
    }

    pub fn set_enricher(&self, enricher: Enricher) {
        *lock(&self.enricher) = Some(enricher);
    }

    /// Se il nominativo non e' ancora impostato usa questo (quello di JTDX).
    pub fn default_callsign(&self, call: &str) {
        let mut config = lock(&self.config);
        if config.callsign.is_empty() && valid_callsign(call) {
            config.callsign = call.to_uppercase();
        }
    }

    /// Valida, salva su disco e rifa' i collegamenti con le impostazioni nuove.
    pub fn set_config(self: &Arc<Self>, config: Config) -> Result<(), String> {
        config.validate()?;
        let text = serde_json::to_string_pretty(&config).map_err(|e| e.to_string())?;
        let tmp = self.path.with_extension("json.tmp");
        std::fs::write(&tmp, text).and_then(|()| std::fs::rename(&tmp, &self.path)).map_err(|e| format!("salvataggio: {e}"))?;
        *lock(&self.config) = config;
        self.restart();
        Ok(())
    }

    /// Chiude i collegamenti e li riapre secondo le impostazioni (nessuno se spento o senza nominativo).
    pub fn restart(self: &Arc<Self>) {
        for task in lock(&self.tasks).drain(..) {
            task.abort();
        }
        lock(&self.senders).clear();
        lock(&self.status).clear();
        let config = self.config();
        if !config.enabled || !valid_callsign(&config.callsign) {
            return;
        }
        let mut tasks = lock(&self.tasks);
        for server in config.servers.iter().filter(|s| s.enabled) {
            let (svc, server, call, commands) = (Arc::clone(self), server.clone(), config.callsign.clone(), config.login_commands.clone());
            tasks.push(tokio::spawn(run_server(svc, server, call, commands)));
        }
    }

    /// Gli spot piu' recenti di `since` (piu' nuovi per primi), al massimo `limit`.
    pub fn spots_since(&self, since: u64, limit: usize) -> Vec<Spot> {
        lock(&self.spots).iter().filter(|s| s.id > since).take(limit).cloned().collect()
    }

    pub fn statuses(&self) -> Vec<ServerStatus> {
        lock(&self.status).values().cloned().collect()
    }

    /// Manda un comando a un cluster collegato (per esempio `sh/dx 20`).
    pub fn send_command(&self, server_key: &str, text: &str) -> Result<(), String> {
        if !valid_command(text) {
            return Err("comando non valido".into());
        }
        let senders = lock(&self.senders);
        let sender = senders.get(server_key).ok_or_else(|| "server non collegato".to_string())?;
        sender.send(text.trim().to_string()).map_err(|_| "server non collegato".to_string())
    }

    fn set_status(&self, server: &ServerCfg, state: &'static str, detail: Option<String>) {
        let mut all = lock(&self.status);
        let key = server.key();
        let spots = all.get(&key).map_or(0, |s| s.spots);
        all.insert(key.clone(), ServerStatus { key, name: server.name.clone(), state, detail, spots, since_ms: unix_ms() });
    }

    fn push_spot(&self, server: &ServerCfg, parsed: spot::Parsed) {
        let (max_spots, max_age_ms) = {
            let c = lock(&self.config);
            (c.max_spots, u64::from(c.max_age_min) * 60_000)
        };
        let enriched = lock(&self.enricher).as_ref().and_then(|e| e(&parsed.dx_call));
        let now = unix_ms();
        let mut spots = lock(&self.spots);
        // lo stesso spot arriva da piu' cluster: si tiene il primo
        let duplicate = spots
            .iter()
            .take(300)
            .any(|s| s.dx_call == parsed.dx_call && s.spotter == parsed.spotter && s.time_utc == parsed.time_utc && (s.freq_khz - parsed.freq_khz).abs() < 0.15);
        if duplicate {
            return;
        }
        let (country, continent) = enriched.map_or((None, None), |(a, b)| (Some(a), Some(b)));
        spots.push_front(Spot {
            id: self.next_id.fetch_add(1, Ordering::SeqCst),
            received_ms: now,
            server: server.name.clone(),
            spotter: parsed.spotter,
            freq_khz: parsed.freq_khz,
            dx_call: parsed.dx_call,
            comment: parsed.comment,
            time_utc: parsed.time_utc,
            grid: parsed.grid,
            band: parsed.band,
            mode: parsed.mode,
            country,
            continent,
        });
        spots.truncate(max_spots);
        while spots.back().is_some_and(|s| now.saturating_sub(s.received_ms) > max_age_ms) {
            spots.pop_back();
        }
        drop(spots);
        if let Some(s) = lock(&self.status).get_mut(&server.key()) {
            s.spots += 1;
        }
    }
}

/// Tiene un server collegato: se cade riprova con attese sempre piu' lunghe (da 5 s a 2 minuti).
async fn run_server(svc: Arc<ClusterService>, server: ServerCfg, callsign: String, commands: Vec<String>) {
    let key = server.key();
    let mut wait = 5u64;
    loop {
        svc.set_status(&server, "connecting", None);
        let (tx, rx) = mpsc::unbounded_channel();
        lock(&svc.senders).insert(key.clone(), tx);
        let started = std::time::Instant::now();
        let outcome = session(&svc, &server, &callsign, &commands, rx).await;
        lock(&svc.senders).remove(&key);
        let detail = match outcome {
            Ok(()) => "connessione chiusa dal server".to_string(),
            Err(e) => e,
        };
        tracing::info!("dx cluster {}: {detail}", server.name);
        svc.set_status(&server, "error", Some(detail));
        if started.elapsed() > Duration::from_secs(60) {
            wait = 5;
        }
        tokio::time::sleep(Duration::from_secs(wait)).await;
        wait = (wait * 2).min(120);
    }
}

async fn session(svc: &Arc<ClusterService>, server: &ServerCfg, callsign: &str, commands: &[String], mut outgoing: mpsc::UnboundedReceiver<String>) -> Result<(), String> {
    let stream = tokio::time::timeout(CONNECT_TIMEOUT, TcpStream::connect((server.host.as_str(), server.port)))
        .await
        .map_err(|_| "nessuna risposta dal server".to_string())?
        .map_err(|e| format!("collegamento non riuscito: {e}"))?;
    let (mut reader, mut writer) = stream.into_split();
    svc.set_status(server, "connected", None);

    let mut decoder = telnet::Decoder::default();
    let mut line: Vec<u8> = Vec::new();
    let mut buf = [0u8; 4096];
    let mut logged_in = false;
    let mut keepalive = tokio::time::interval(KEEPALIVE);
    keepalive.tick().await;

    loop {
        tokio::select! {
            read = reader.read(&mut buf) => {
                let n = read.map_err(|e| format!("lettura: {e}"))?;
                if n == 0 {
                    return Ok(());
                }
                let (text, replies) = decoder.feed(&buf[..n]);
                if !replies.is_empty() {
                    writer.write_all(&replies).await.map_err(|e| format!("scrittura: {e}"))?;
                }
                for byte in text {
                    match byte {
                        b'\n' => {
                            let text = String::from_utf8_lossy(&line).into_owned();
                            line.clear();
                            if let Some(parsed) = spot::parse_spot(&text) {
                                logged_in = true;
                                svc.push_spot(server, parsed);
                            } else if !logged_in && telnet::is_login_prompt(&text) {
                                logged_in = true;
                                log_in(&mut writer, callsign, commands).await?;
                            }
                        }
                        b'\r' | 0 => {}
                        other => line.push(other),
                    }
                    if line.len() > MAX_LINE {
                        line.clear();
                    }
                }
                // la richiesta del nominativo di solito non finisce con un a capo
                if !logged_in && telnet::is_login_prompt(&String::from_utf8_lossy(&line)) {
                    logged_in = true;
                    line.clear();
                    log_in(&mut writer, callsign, commands).await?;
                }
            }
            Some(command) = outgoing.recv() => {
                writer.write_all(format!("{command}\r\n").as_bytes()).await.map_err(|e| format!("scrittura: {e}"))?;
            }
            _ = keepalive.tick() => {
                writer.write_all(b"\r\n").await.map_err(|e| format!("scrittura: {e}"))?;
            }
        }
    }
}

async fn log_in(writer: &mut tokio::net::tcp::OwnedWriteHalf, callsign: &str, commands: &[String]) -> Result<(), String> {
    writer.write_all(format!("{callsign}\r\n").as_bytes()).await.map_err(|e| format!("scrittura: {e}"))?;
    for command in commands {
        tokio::time::sleep(Duration::from_millis(400)).await;
        writer.write_all(format!("{}\r\n", command.trim()).as_bytes()).await.map_err(|e| format!("scrittura: {e}"))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::{
        io::{AsyncBufReadExt, BufReader},
        net::TcpListener,
    };

    fn temp(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("aurora-cluster-{name}-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        dir.join("dx-cluster.json")
    }

    fn local(port: u16) -> ServerCfg {
        ServerCfg { name: "Prova".into(), host: "127.0.0.1".into(), port, enabled: true }
    }

    #[test]
    fn the_defaults_are_valid_and_off() {
        let c = Config::default();
        assert!(c.validate().is_ok());
        assert!(!c.enabled, "non si collega a nessuno finche' non lo si accende");
        assert!(c.servers.len() <= MAX_SERVERS);
    }

    #[test]
    fn the_catalog_entries_are_all_valid_servers() {
        for e in CATALOG {
            let s = ServerCfg { name: e.name.into(), host: e.host.into(), port: e.port, enabled: true };
            assert!(Config { servers: vec![s], ..Config::default() }.validate().is_ok(), "{}", e.name);
        }
    }

    #[test]
    fn refuses_invalid_values() {
        let bad = |f: &dyn Fn(&mut Config)| {
            let mut c = Config::default();
            f(&mut c);
            c.validate().is_err()
        };
        assert!(bad(&|c| c.callsign = "AB".into()));
        assert!(bad(&|c| c.callsign = "NOCIFRA".into()));
        assert!(bad(&|c| c.callsign = "A1 B2".into()));
        assert!(bad(&|c| c.servers[0].host = "; rm".into()));
        assert!(bad(&|c| c.servers[0].port = 0));
        assert!(bad(&|c| c.servers[0].name = String::new()));
        assert!(bad(&|c| c.servers = (0..9).map(|i| local(7000 + i)).collect()));
        assert!(bad(&|c| c.login_commands = vec!["a\r\nb".into()]));
        assert!(bad(&|c| c.login_commands = vec![String::new()]));
        assert!(bad(&|c| c.login_commands = vec!["dx 14074 K1ABC cq".into()]), "niente spot pubblici");
        assert!(bad(&|c| c.login_commands = vec!["ann/full ciao a tutti".into()]));
        assert!(bad(&|c| c.login_commands = vec!["talk W1AW ciao".into()]));
        assert!(bad(&|c| c.login_commands = vec!["x".repeat(81)]));
        assert!(bad(&|c| c.max_spots = 10));
        assert!(bad(&|c| c.max_age_min = 1));
        let mut ok = Config::default();
        ok.callsign = "IU3ABC".into();
        ok.login_commands = vec!["set/ft8".into(), "SH/DX 10".into(), "accept/spots on hf/ft8".into()];
        assert!(ok.validate().is_ok());
    }

    #[tokio::test]
    async fn nothing_connects_while_switched_off_or_without_a_callsign() {
        let svc = ClusterService::open(&temp("off"));
        let mut c = Config::default();
        c.servers = vec![local(9)];
        svc.set_config(c.clone()).unwrap();
        assert!(svc.statuses().is_empty(), "spento");
        c.enabled = true;
        svc.set_config(c).unwrap();
        assert!(svc.statuses().is_empty(), "senza nominativo");
    }

    #[tokio::test]
    async fn logs_in_collects_spots_and_forwards_commands() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        let server = tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            let (read, mut write) = stream.into_split();
            let mut reader = BufReader::new(read);
            write.write_all(&[255, 253, 24]).await.unwrap(); // IAC DO TERMINAL-TYPE
            write.write_all(b"Benvenuti\r\nlogin: ").await.unwrap();
            // prima del nominativo il client rifiuta l'opzione proposta (byte di telnet): si legge in byte
            let mut call = Vec::new();
            reader.read_until(10, &mut call).await.unwrap();
            let refusal_or_call = String::from_utf8_lossy(&call).into_owned();
            write.write_all(b"Hello IU3ABC, there are 3 users\r\nDX de W3LPL:     14074.0  JA1ABC       FT8 -10 dB                     1234Z\r\n").await.unwrap();
            write.write_all(b"DX de W3LPL:     14074.0  JA1ABC       FT8 -10 dB                     1234Z\r\n").await.unwrap();
            write.write_all(b"DX de G4ABC:      7010.5  K1ABC        cw 22 wpm       0856Z IO91\r\n").await.unwrap();
            let mut command = Vec::new();
            reader.read_until(10, &mut command).await.unwrap();
            (refusal_or_call, String::from_utf8_lossy(&command).into_owned())
        });

        let svc = ClusterService::open(&temp("flow"));
        svc.set_enricher(Arc::new(|call: &str| (call == "JA1ABC").then(|| ("Japan".to_string(), "AS".to_string()))));
        let mut c = Config::default();
        c.enabled = true;
        c.callsign = "IU3ABC".into();
        c.servers = vec![local(port)];
        svc.set_config(c).unwrap();

        let mut waited = 0;
        while svc.spots_since(0, 10).len() < 2 && waited < 60 {
            tokio::time::sleep(Duration::from_millis(100)).await;
            waited += 1;
        }
        let spots = svc.spots_since(0, 10);
        assert_eq!(spots.len(), 2, "lo spot doppio si tiene una volta sola: {spots:?}");
        assert_eq!(spots[0].dx_call, "K1ABC", "i piu' nuovi per primi");
        assert_eq!(spots[0].grid.as_deref(), Some("IO91"));
        assert_eq!(spots[1].country.as_deref(), Some("Japan"));
        assert_eq!(svc.spots_since(spots[1].id, 10).len(), 1, "dall'id in poi");
        let status = svc.statuses();
        assert_eq!((status[0].state, status[0].spots), ("connected", 2));

        svc.send_command(&format!("127.0.0.1:{port}"), "sh/dx 5").unwrap();
        assert!(svc.send_command("altro:1", "sh/dx").is_err());
        assert!(svc.send_command(&format!("127.0.0.1:{port}"), "dx 14074 K1ABC").is_err(), "uno spot e' pubblico");
        assert!(svc.send_command(&format!("127.0.0.1:{port}"), "a\nb").is_err());
        let (call, command) = server.await.unwrap();
        assert!(call.contains("IU3ABC"), "{call:?}");
        assert_eq!(command.trim(), "sh/dx 5");
    }

    #[tokio::test]
    async fn an_unreachable_server_is_reported_as_an_error() {
        let svc = ClusterService::open(&temp("down"));
        let mut c = Config::default();
        c.enabled = true;
        c.callsign = "IU3ABC".into();
        c.servers = vec![local(1)];
        svc.set_config(c).unwrap();
        let mut waited = 0;
        while svc.statuses().first().is_none_or(|s| s.state != "error") && waited < 60 {
            tokio::time::sleep(Duration::from_millis(100)).await;
            waited += 1;
        }
        let status = svc.statuses();
        assert_eq!(status[0].state, "error");
        assert!(status[0].detail.is_some());
    }

    #[test]
    fn the_configuration_survives_a_restart_and_a_broken_file_gives_the_defaults() {
        let path = temp("persist");
        let rt = tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap();
        rt.block_on(async {
            let svc = ClusterService::open(&path);
            let mut c = svc.config();
            c.callsign = "IU3ABC".into();
            c.max_spots = 123;
            svc.set_config(c.clone()).unwrap();
            assert_eq!(ClusterService::open(&path).config(), c);
        });
        std::fs::write(&path, "non e' json").unwrap();
        assert_eq!(ClusterService::open(&path).config(), Config::default());
    }

    #[test]
    fn the_callsign_of_jtdx_fills_an_empty_one_only() {
        let svc = ClusterService::open(&temp("default"));
        svc.default_callsign("iu3abc");
        assert_eq!(svc.config().callsign, "IU3ABC");
        svc.default_callsign("K1XYZ");
        assert_eq!(svc.config().callsign, "IU3ABC");
    }
}
