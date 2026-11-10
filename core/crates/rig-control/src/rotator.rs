//! Il rotore d'antenna con Hamlib `rotctld`, lo stesso approccio del CAT: Aurora lancia `rotctld` (accanto a `rigctld`, nella stessa cartella
//! di Hamlib) come processo nascosto, oppure si collega a un `rotctld` gia' acceso (di base `127.0.0.1:4533`), e ci parla con il suo protocollo
//! di testo su TCP.
//!
//! Comandi usati: `p` (posizione: due righe, azimut ed elevazione), `P az el` (vai a), `S` (ferma). Le risposte di errore sono `RPRT <codice>`.
//! Tutto il lavoro con il rotore sta in un thread suo: le rotte e il motore non aspettano mai il rotore. La posizione si rilegge ogni secondo;
//! se il collegamento cade si riprova da soli dopo qualche secondo.

use std::{
    io::{BufRead, BufReader, Write},
    net::{Ipv4Addr, SocketAddr, TcpListener, TcpStream, ToSocketAddrs},
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    sync::{
        Arc, Mutex,
        mpsc::{self, RecvTimeoutError},
    },
    time::{Duration, Instant},
};

use serde::{Deserialize, Serialize};

use crate::{
    RigError,
    hamlib::{RigModel, check_token, hamlib_error, parse_model_list, parse_rprt},
};

/// La porta TCP di base di `rotctld`.
pub const DEFAULT_ROTCTLD_PORT: u16 = 4533;
/// Ogni quanto si rilegge la posizione.
const POLL: Duration = Duration::from_secs(1);
/// Attesa prima di riprovare dopo un collegamento non riuscito o caduto.
const RETRY: Duration = Duration::from_secs(5);
/// Entro quanti gradi l'antenna e' arrivata sul bersaglio.
const ARRIVED_DEG: f64 = 2.0;

/// Come si raggiunge `rotctld`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum RotConnection {
    /// Aurora lancia `rotctld` con modello, porta seriale e velocita'.
    #[default]
    Launch,
    /// Aurora si collega a un `rotctld` gia' acceso (anche su un altro computer).
    Remote,
}

/// La configurazione del rotore (file `rotore.json` nella cartella dei dati).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct RotatorConfig {
    /// Rotore acceso: Aurora si collega e resta collegata (anche al prossimo avvio).
    pub enabled: bool,
    pub connection: RotConnection,
    /// Numero del modello di Hamlib (`rotctld -l`); 1 = «dummy», senza rotore, per le prove.
    pub model: u32,
    /// Il nome del modello, solo da mostrare.
    pub model_name: String,
    /// Porta seriale (`COM5`, `/dev/ttyUSB0`); vuota per i modelli che non la usano.
    pub port: String,
    /// Velocita' della seriale; 0 = quella di base del modello.
    pub baud: u32,
    /// Indirizzo del `rotctld` gia' acceso.
    pub host: String,
    /// Porta TCP del `rotctld` gia' acceso.
    pub tcp_port: u16,
}

impl Default for RotatorConfig {
    fn default() -> RotatorConfig {
        RotatorConfig { enabled: false, connection: RotConnection::Launch, model: 0, model_name: String::new(), port: String::new(), baud: 0, host: "127.0.0.1".into(), tcp_port: DEFAULT_ROTCTLD_PORT }
    }
}

/// Un errore del rotore: codice breve per l'API e testo per l'utente.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RotError {
    pub code: &'static str,
    pub message: String,
}

impl RotError {
    fn new(code: &'static str, message: impl Into<String>) -> RotError {
        RotError { code, message: message.into() }
    }
}

impl std::fmt::Display for RotError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl RotatorConfig {
    /// Controlla i valori prima di salvarli. Il modello serve solo se Aurora deve lanciare `rotctld` e il rotore e' acceso.
    pub fn validate(&self) -> Result<(), RotError> {
        if self.model_name.chars().count() > 120 {
            return Err(RotError::new("bad_model", "nome del modello troppo lungo"));
        }
        match self.connection {
            RotConnection::Launch => {
                if self.enabled && self.model == 0 {
                    return Err(RotError::new("bad_model", "scegli il modello del rotore"));
                }
                if !self.port.trim().is_empty() {
                    check_token(self.port.trim(), "porta").map_err(|e| RotError::new("bad_port", e.0))?;
                }
                if self.baud != 0 && !(300..=1_000_000).contains(&self.baud) {
                    return Err(RotError::new("bad_baud", "velocita' della seriale non valida"));
                }
            }
            RotConnection::Remote => {
                let host = self.host.trim();
                let ok = !host.is_empty() && host.len() <= 253 && host.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | ':' | '[' | ']'));
                if !ok {
                    return Err(RotError::new("bad_host", "indirizzo del rotctld non valido"));
                }
                if self.tcp_port == 0 {
                    return Err(RotError::new("bad_tcp_port", "porta del rotctld non valida"));
                }
            }
        }
        Ok(())
    }
}

/// Gli argomenti di `rotctld` per questa configurazione, in ascolto su `tcp_port` solo in locale.
pub fn rotctld_args(config: &RotatorConfig, tcp_port: u16) -> Result<Vec<String>, RigError> {
    let mut args = vec!["-m".to_string(), config.model.to_string(), "-T".to_string(), "127.0.0.1".to_string(), "-t".to_string(), tcp_port.to_string()];
    let port = config.port.trim();
    if !port.is_empty() {
        check_token(port, "porta")?;
        args.extend(["-r".to_string(), port.to_string()]);
    }
    if config.baud != 0 {
        args.extend(["-s".to_string(), config.baud.to_string()]);
    }
    Ok(args)
}

/// L'azimut riportato in 0..360 gradi.
pub fn normalize_azimuth(az: f64) -> f64 {
    let a = az.rem_euclid(360.0);
    if a >= 360.0 { 0.0 } else { a }
}

/// La distanza angolare piu' corta tra due azimut (0..=180).
pub fn angular_distance(a: f64, b: f64) -> f64 {
    let d = (a - b).rem_euclid(360.0);
    d.min(360.0 - d)
}

/// Una connessione a un `rotctld` che ascolta.
pub struct RotctldClient {
    reader: BufReader<TcpStream>,
    writer: TcpStream,
}

impl RotctldClient {
    /// Si collega a `addr`, riprovando fino a `wait`; ogni risposta si aspetta al massimo `io_timeout`.
    pub fn connect(addr: SocketAddr, wait: Duration, io_timeout: Duration) -> Result<RotctldClient, RigError> {
        let deadline = Instant::now() + wait;
        loop {
            match TcpStream::connect_timeout(&addr, Duration::from_millis(800)) {
                Ok(stream) => {
                    stream.set_read_timeout(Some(io_timeout)).map_err(|e| RigError(e.to_string()))?;
                    stream.set_write_timeout(Some(io_timeout)).map_err(|e| RigError(e.to_string()))?;
                    let _ = stream.set_nodelay(true);
                    let writer = stream.try_clone().map_err(|e| RigError(e.to_string()))?;
                    return Ok(RotctldClient { reader: BufReader::new(stream), writer });
                }
                Err(e) if Instant::now() >= deadline => return Err(RigError(format!("rotctld non risponde su {addr}: {e}"))),
                Err(_) => std::thread::sleep(Duration::from_millis(150)),
            }
        }
    }

    /// Manda un comando e legge `lines` righe di risposta (0 per i comandi che rispondono solo `RPRT`).
    fn ask(&mut self, command: &str, lines: usize) -> Result<Vec<String>, RigError> {
        self.writer.write_all(format!("{command}\n").as_bytes()).map_err(|e| RigError(format!("rotctld: scrittura: {e}")))?;
        let mut out = Vec::new();
        loop {
            let mut line = String::new();
            let n = self.reader.read_line(&mut line).map_err(|e| RigError(format!("rotctld: lettura: {e}")))?;
            if n == 0 {
                return Err(RigError("rotctld ha chiuso la connessione".into()));
            }
            let line = line.trim().to_string();
            match parse_rprt(&line) {
                Some(Ok(())) if out.is_empty() => return if lines == 0 { Ok(out) } else { Err(RigError(format!("rotctld: risposta vuota a «{command}»"))) },
                Some(Err(code)) => return Err(RigError(format!("{command}: {} ({code})", hamlib_error(code)))),
                _ => {}
            }
            if lines == 0 {
                return Err(RigError(format!("rotctld: risposta inattesa a «{command}»: {line}")));
            }
            out.push(line);
            if out.len() >= lines {
                return Ok(out);
            }
        }
    }

    /// Azimut ed elevazione attuali (`p`).
    pub fn position(&mut self) -> Result<(f64, f64), RigError> {
        let lines = self.ask("p", 2)?;
        let num = |s: &str| s.trim().replace(',', ".").parse::<f64>().map_err(|_| RigError(format!("rotctld: posizione non leggibile: {s}")));
        Ok((num(&lines[0])?, num(&lines[1])?))
    }

    /// Manda l'antenna a `az` (e `el`) gradi (`P`).
    pub fn set_position(&mut self, az: f64, el: f64) -> Result<(), RigError> {
        self.ask(&format!("P {az:.1} {el:.1}"), 0).map(|_| ())
    }

    /// Ferma il rotore (`S`).
    pub fn stop(&mut self) -> Result<(), RigError> {
        self.ask("S", 0).map(|_| ())
    }

    /// Chiude la sessione lasciando acceso il demone.
    pub fn quit(&mut self) {
        let _ = self.writer.write_all(b"q\n");
    }
}

/// Un `rotctld` lanciato da Aurora: si ferma quando l'oggetto cade (cosi' la porta seriale si libera).
struct RotctldProcess {
    child: Child,
    stderr_tail: Arc<Mutex<Vec<String>>>,
}

impl RotctldProcess {
    fn start(exe: &Path, config: &RotatorConfig) -> Result<(RotctldProcess, RotctldClient), RigError> {
        let probe = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).map_err(|e| RigError(format!("nessuna porta libera: {e}")))?;
        let tcp_port = probe.local_addr().map_err(|e| RigError(e.to_string()))?.port();
        drop(probe);
        let args = rotctld_args(config, tcp_port)?;
        let mut command = Command::new(exe);
        command.args(&args).stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::piped());
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            // niente finestra nera quando Aurora gira come programma a finestre
            command.creation_flags(0x0800_0000);
        }
        let mut child = command.spawn().map_err(|e| RigError(format!("rotctld non avviato: {e}")))?;
        let stderr_tail = Arc::new(Mutex::new(Vec::<String>::new()));
        if let Some(stderr) = child.stderr.take() {
            let tail = Arc::clone(&stderr_tail);
            std::thread::spawn(move || {
                for line in BufReader::new(stderr).lines().map_while(Result::ok) {
                    if let Ok(mut lines) = tail.lock() {
                        lines.push(line);
                        if lines.len() > 8 {
                            lines.remove(0);
                        }
                    }
                }
            });
        }
        let mut process = RotctldProcess { child, stderr_tail };
        let addr = SocketAddr::from((Ipv4Addr::LOCALHOST, tcp_port));
        let deadline = Instant::now() + Duration::from_secs(6);
        loop {
            match RotctldClient::connect(addr, Duration::from_millis(250), Duration::from_secs(3)) {
                Ok(client) => return Ok((process, client)),
                Err(e) => {
                    // il demone esce da solo se il rotore non si apre (modello o porta sbagliati, porta occupata)
                    if let Ok(Some(status)) = process.child.try_wait() {
                        std::thread::sleep(Duration::from_millis(150));
                        let tail = process.stderr_tail.lock().map(|l| l.join(" | ")).unwrap_or_default();
                        return Err(RigError(format!("rotctld si e' fermato ({status}): {tail}")));
                    }
                    if Instant::now() >= deadline {
                        return Err(e);
                    }
                }
            }
        }
    }
}

impl Drop for RotctldProcess {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

/// Dove si trova `rotctld`: la variabile `AURORA_ROTCTLD` (file o cartella), poi la cartella di `rigctld` (Hamlib incluso in Aurora).
pub fn locate_rotctld() -> Option<PathBuf> {
    if let Some(path) = std::env::var("AURORA_ROTCTLD").ok().filter(|v| !v.trim().is_empty()).map(PathBuf::from) {
        if path.is_file() {
            return Some(path);
        }
        if let Some(found) = rotctld_in_dir(&path) {
            return Some(found);
        }
    }
    let rigctld = crate::hamlib::locate_rigctld()?;
    rotctld_in_dir(rigctld.parent()?)
}

/// `rotctld` (o `rotctld.exe`) in una cartella.
pub fn rotctld_in_dir(dir: &Path) -> Option<PathBuf> {
    let mut found: Vec<PathBuf> = std::fs::read_dir(dir)
        .ok()?
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| {
            let name = p.file_name().and_then(|n| n.to_str()).unwrap_or("").to_ascii_lowercase();
            name.starts_with("rotctld") && (name == "rotctld" || name.ends_with(".exe")) && p.is_file()
        })
        .collect();
    found.sort_by_key(|p| p.file_name().map_or(usize::MAX, |n| n.len()));
    found.into_iter().next()
}

/// L'elenco dei modelli di rotore che conosce Hamlib (`rotctld -l`, stesse colonne di `rigctl -l`).
pub fn list_rotator_models(rotctld: &Path) -> Result<Vec<RigModel>, RigError> {
    let mut command = Command::new(rotctld);
    command.arg("-l").stdin(Stdio::null()).stderr(Stdio::null());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x0800_0000);
    }
    let out = command.output().map_err(|e| RigError(format!("rotctld non avviato: {e}")))?;
    let models = parse_model_list(&String::from_utf8_lossy(&out.stdout));
    if models.is_empty() {
        return Err(RigError("rotctld non ha dato nessun modello".into()));
    }
    Ok(models)
}

/// Lo stato del rotore.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum RotState {
    #[default]
    Off,
    Connecting,
    Connected,
    Error,
}

/// Quello che l'interfaccia mostra: posizione, bersaglio, movimento ed eventuale errore.
#[derive(Debug, Clone, PartialEq, Serialize, Default)]
pub struct RotatorStatus {
    pub state: RotState,
    pub connected: bool,
    /// Azimut letto (gradi, 0 = nord).
    pub az: Option<f64>,
    /// Elevazione letta (0 per i rotori solo azimut).
    pub el: Option<f64>,
    /// Dove si e' mandata l'antenna; nullo dopo «Ferma».
    pub target: Option<f64>,
    pub moving: bool,
    /// L'ultimo errore, per l'utente.
    pub error: Option<String>,
    /// `rotctld` trovato sul disco (serve solo per lanciarlo).
    pub rotctld_found: bool,
}

/// Configurazione e stato insieme (risposta di `GET /rotator`).
#[derive(Debug, Clone, Serialize)]
pub struct RotatorSnapshot {
    pub status: RotatorStatus,
    pub config: RotatorConfig,
}

enum Cmd {
    Reconfigure,
    Goto(f64, f64),
    Stop,
}

struct Shared {
    config: Mutex<RotatorConfig>,
    status: Mutex<RotatorStatus>,
}

impl Shared {
    fn config(&self) -> RotatorConfig {
        self.config.lock().map(|c| c.clone()).unwrap_or_default()
    }

    fn update(&self, f: impl FnOnce(&mut RotatorStatus)) {
        if let Ok(mut s) = self.status.lock() {
            f(&mut s);
        }
    }
}

/// Il servizio del rotore: configurazione salvata, un thread che tiene il collegamento e legge la posizione, comandi senza attese.
pub struct RotatorService {
    path: PathBuf,
    shared: Arc<Shared>,
    tx: Mutex<mpsc::Sender<Cmd>>,
    models: Mutex<Option<Vec<RigModel>>>,
}

impl RotatorService {
    /// Legge `path` (un file guasto si mette da parte come `.json.bad` e si riparte da zero) e avvia il thread del rotore.
    pub fn open(path: &Path) -> RotatorService {
        let config = read_config(path);
        Self::start(path.to_path_buf(), config)
    }

    fn start(path: PathBuf, config: RotatorConfig) -> RotatorService {
        let shared = Arc::new(Shared { config: Mutex::new(config), status: Mutex::new(RotatorStatus::default()) });
        let (tx, rx) = mpsc::channel();
        let worker = Arc::clone(&shared);
        let spawned = std::thread::Builder::new().name("rotore".into()).spawn(move || run(&worker, &rx));
        if let Err(e) = spawned {
            tracing::warn!("thread del rotore non avviato: {e}");
        }
        RotatorService { path, shared, tx: Mutex::new(tx), models: Mutex::new(None) }
    }

    fn send(&self, cmd: Cmd) {
        if let Ok(tx) = self.tx.lock() {
            let _ = tx.send(cmd);
        }
    }

    pub fn config(&self) -> RotatorConfig {
        self.shared.config()
    }

    pub fn status(&self) -> RotatorStatus {
        let mut status = self.shared.status.lock().map(|s| s.clone()).unwrap_or_default();
        status.rotctld_found = locate_rotctld().is_some();
        status
    }

    pub fn snapshot(&self) -> RotatorSnapshot {
        RotatorSnapshot { status: self.status(), config: self.config() }
    }

    /// Salva la configurazione e la applica: il collegamento si rifa' con i valori nuovi.
    pub fn apply_config(&self, mut config: RotatorConfig) -> Result<(), RotError> {
        config.port = config.port.trim().to_string();
        config.host = config.host.trim().to_string();
        config.model_name = config.model_name.trim().to_string();
        config.validate()?;
        write_config(&self.path, &config).map_err(|e| RotError::new("write_failed", e))?;
        if let Ok(mut c) = self.shared.config.lock() {
            *c = config;
        }
        self.send(Cmd::Reconfigure);
        Ok(())
    }

    /// Manda l'antenna a `az` gradi (riportati in 0..360); l'elevazione resta quella letta. Non aspetta il rotore.
    pub fn goto(&self, az: f64, el: Option<f64>) -> Result<(), RotError> {
        if !az.is_finite() || !(-360.0..=720.0).contains(&az) {
            return Err(RotError::new("bad_azimuth", "azimut non valido"));
        }
        let el = match el {
            Some(e) if !e.is_finite() || !(0.0..=180.0).contains(&e) => return Err(RotError::new("bad_elevation", "elevazione non valida")),
            Some(e) => e,
            None => self.shared.status.lock().ok().and_then(|s| s.el).unwrap_or(0.0),
        };
        self.require_connected()?;
        let az = normalize_azimuth(az);
        self.shared.update(|s| {
            s.target = Some(az);
            s.moving = true;
        });
        self.send(Cmd::Goto(az, el));
        Ok(())
    }

    /// Ferma il rotore.
    pub fn stop(&self) -> Result<(), RotError> {
        self.require_connected()?;
        self.shared.update(|s| s.target = None);
        self.send(Cmd::Stop);
        Ok(())
    }

    fn require_connected(&self) -> Result<(), RotError> {
        let connected = self.shared.status.lock().map(|s| s.connected).unwrap_or(false);
        if connected { Ok(()) } else { Err(RotError::new("not_connected", "rotore non collegato")) }
    }

    /// I modelli di rotore di Hamlib (letti una volta e poi ricordati).
    pub fn models(&self) -> Result<Vec<RigModel>, RotError> {
        if let Some(models) = self.models.lock().ok().and_then(|m| m.clone()) {
            return Ok(models);
        }
        let exe = locate_rotctld().ok_or_else(|| RotError::new("rotctld_missing", "rotctld non trovato accanto a rigctld"))?;
        let models = list_rotator_models(&exe).map_err(|e| RotError::new("models_failed", e.0))?;
        if let Ok(mut m) = self.models.lock() {
            *m = Some(models.clone());
        }
        Ok(models)
    }
}

/// Il collegamento vivo: il client e, se l'ha lanciato Aurora, il processo.
struct Link {
    client: RotctldClient,
    _process: Option<RotctldProcess>,
}

impl Drop for Link {
    fn drop(&mut self) {
        self.client.quit();
    }
}

fn connect(config: &RotatorConfig) -> Result<Link, RigError> {
    match config.connection {
        RotConnection::Remote => {
            let addr = (config.host.trim(), config.tcp_port)
                .to_socket_addrs()
                .map_err(|e| RigError(format!("indirizzo {}:{} non trovato: {e}", config.host, config.tcp_port)))?
                .next()
                .ok_or_else(|| RigError(format!("indirizzo {}:{} non trovato", config.host, config.tcp_port)))?;
            let client = RotctldClient::connect(addr, Duration::from_secs(2), Duration::from_secs(3))?;
            Ok(Link { client, _process: None })
        }
        RotConnection::Launch => {
            let exe = locate_rotctld().ok_or_else(|| RigError("rotctld non trovato accanto a rigctld (cartella di Hamlib)".into()))?;
            let (process, client) = RotctldProcess::start(&exe, config)?;
            Ok(Link { client, _process: Some(process) })
        }
    }
}

/// Il thread del rotore: collega, rilegge la posizione ogni secondo, esegue i comandi, ricollega dopo un errore. Finisce quando il servizio cade.
fn run(shared: &Shared, rx: &mpsc::Receiver<Cmd>) {
    let mut link: Option<Link> = None;
    let mut next_try = Instant::now();
    let mut last_poll = Instant::now() - POLL;
    loop {
        let config = shared.config();
        if !config.enabled {
            link = None;
            shared.update(|s| *s = RotatorStatus::default());
            match rx.recv() {
                Ok(_) => continue,
                Err(_) => return,
            }
        }
        if link.is_none() && Instant::now() >= next_try {
            shared.update(|s| {
                s.state = RotState::Connecting;
                s.connected = false;
            });
            match connect(&config) {
                Ok(l) => {
                    tracing::info!("rotore collegato");
                    link = Some(l);
                    shared.update(|s| {
                        s.state = RotState::Connected;
                        s.connected = true;
                        s.error = None;
                    });
                    last_poll = Instant::now() - POLL;
                }
                Err(e) => {
                    tracing::debug!("rotore non collegato: {e}");
                    next_try = Instant::now() + RETRY;
                    shared.update(|s| {
                        s.state = RotState::Error;
                        s.connected = false;
                        s.moving = false;
                        s.error = Some(e.0);
                    });
                }
            }
        }
        let wait = if link.is_some() { POLL.saturating_sub(last_poll.elapsed()).max(Duration::from_millis(50)) } else { next_try.saturating_duration_since(Instant::now()).clamp(Duration::from_millis(50), POLL) };
        let outcome = match rx.recv_timeout(wait) {
            Ok(Cmd::Reconfigure) => {
                link = None;
                next_try = Instant::now();
                shared.update(|s| s.target = None);
                continue;
            }
            Ok(Cmd::Goto(az, el)) => link.as_mut().map(|l| l.client.set_position(az, el)),
            Ok(Cmd::Stop) => link.as_mut().map(|l| l.client.stop()),
            Err(RecvTimeoutError::Timeout) => None,
            Err(RecvTimeoutError::Disconnected) => return,
        };
        if let Some(Err(e)) = outcome {
            // un comando rifiutato (RPRT) non vuol dire collegamento perso: lo si dice e si va avanti
            shared.update(|s| s.error = Some(e.0.clone()));
            if crate::hamlib::error_code(&e).is_none() {
                drop_link(shared, &mut link, &mut next_try, e);
                continue;
            }
        }
        if let Some(l) = link.as_mut()
            && last_poll.elapsed() >= POLL.saturating_sub(Duration::from_millis(100))
        {
            last_poll = Instant::now();
            match l.client.position() {
                Ok((az, el)) => {
                    let az = normalize_azimuth(az);
                    shared.update(|s| {
                        let turned = s.az.is_some_and(|prev| angular_distance(prev, az) > 0.5);
                        let far = s.target.is_some_and(|t| angular_distance(az, t) > ARRIVED_DEG);
                        s.moving = turned || far;
                        s.az = Some(az);
                        s.el = Some(el);
                        s.state = RotState::Connected;
                        s.connected = true;
                    });
                }
                Err(e) => drop_link(shared, &mut link, &mut next_try, e),
            }
        }
    }
}

fn drop_link(shared: &Shared, link: &mut Option<Link>, next_try: &mut Instant, e: RigError) {
    tracing::debug!("rotore scollegato: {e}");
    *link = None;
    *next_try = Instant::now() + Duration::from_secs(3);
    shared.update(|s| {
        s.state = RotState::Error;
        s.connected = false;
        s.moving = false;
        s.error = Some(e.0);
    });
}

fn read_config(path: &Path) -> RotatorConfig {
    match std::fs::read_to_string(path) {
        Ok(text) => serde_json::from_str::<RotatorConfig>(&text).unwrap_or_else(|e| {
            tracing::warn!("{}: file del rotore non leggibile ({e}), si riparte dai valori di base", path.display());
            let _ = std::fs::rename(path, path.with_extension("json.bad"));
            RotatorConfig::default()
        }),
        Err(_) => RotatorConfig::default(),
    }
}

fn write_config(path: &Path, config: &RotatorConfig) -> Result<(), String> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    }
    let text = serde_json::to_string_pretty(config).map_err(|e| e.to_string())?;
    let tmp = path.with_extension("json.tmp");
    std::fs::write(&tmp, text).map_err(|e| format!("{}: {e}", tmp.display()))?;
    std::fs::rename(&tmp, path).map_err(|e| format!("{}: {e}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicBool, Ordering};

    /// Un finto `rotctld`: risponde a `p`, `P` e `S` come quello vero e muove l'antenna di colpo sul bersaglio.
    struct FakeRotctld {
        addr: SocketAddr,
        pos: Arc<Mutex<(f64, f64)>>,
        log: Arc<Mutex<Vec<String>>>,
        refuse: Arc<AtomicBool>,
    }

    impl FakeRotctld {
        fn start(az: f64) -> FakeRotctld {
            let listener = TcpListener::bind("127.0.0.1:0").expect("porta locale");
            let addr = listener.local_addr().expect("indirizzo");
            let pos = Arc::new(Mutex::new((az, 0.0)));
            let log = Arc::new(Mutex::new(Vec::new()));
            let refuse = Arc::new(AtomicBool::new(false));
            let (p, l, r) = (Arc::clone(&pos), Arc::clone(&log), Arc::clone(&refuse));
            std::thread::spawn(move || {
                for stream in listener.incoming() {
                    let Ok(stream) = stream else { continue };
                    let (p, l, r) = (Arc::clone(&p), Arc::clone(&l), Arc::clone(&r));
                    std::thread::spawn(move || serve(stream, &p, &l, &r));
                }
            });
            FakeRotctld { addr, pos, log, refuse }
        }
    }

    fn serve(stream: TcpStream, pos: &Mutex<(f64, f64)>, log: &Mutex<Vec<String>>, refuse: &AtomicBool) {
        let mut writer = stream.try_clone().expect("copia");
        for line in BufReader::new(stream).lines().map_while(Result::ok) {
            let line = line.trim().to_string();
            log.lock().expect("log").push(line.clone());
            let mut parts = line.split_whitespace();
            let reply = match parts.next() {
                Some("p") => {
                    let (az, el) = *pos.lock().expect("pos");
                    format!("{az:.6}\n{el:.6}\n")
                }
                Some("P") if refuse.load(Ordering::SeqCst) => "RPRT -9\n".into(),
                Some("P") => {
                    let az: f64 = parts.next().and_then(|v| v.parse().ok()).unwrap_or(0.0);
                    let el: f64 = parts.next().and_then(|v| v.parse().ok()).unwrap_or(0.0);
                    *pos.lock().expect("pos") = (az, el);
                    "RPRT 0\n".into()
                }
                Some("S") => "RPRT 0\n".into(),
                Some("q") => return,
                _ => "RPRT -4\n".into(),
            };
            if writer.write_all(reply.as_bytes()).is_err() {
                return;
            }
        }
    }

    fn wait_for(service: &RotatorService, what: impl Fn(&RotatorStatus) -> bool) -> RotatorStatus {
        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            let s = service.status();
            if what(&s) || Instant::now() > deadline {
                return s;
            }
            std::thread::sleep(Duration::from_millis(50));
        }
    }

    fn temp_file(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("aurora-rotore-{}-{name}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);
        dir.join("rotore.json")
    }

    #[test]
    fn azimuths_wrap_around() {
        assert_eq!(normalize_azimuth(370.0), 10.0);
        assert_eq!(normalize_azimuth(-2.0), 358.0);
        assert_eq!(normalize_azimuth(360.0), 0.0);
        assert_eq!(angular_distance(359.0, 1.0), 2.0);
        assert_eq!(angular_distance(10.0, 190.0), 180.0);
    }

    #[test]
    fn launch_arguments_carry_model_port_and_speed() {
        let config = RotatorConfig { model: 603, port: "COM5".into(), baud: 9600, ..RotatorConfig::default() };
        let args = rotctld_args(&config, 4600).expect("argomenti");
        assert_eq!(args, ["-m", "603", "-T", "127.0.0.1", "-t", "4600", "-r", "COM5", "-s", "9600"]);
        let bad = RotatorConfig { port: "COM5 & x".into(), ..config };
        assert!(rotctld_args(&bad, 4600).is_err());
    }

    #[test]
    fn config_is_validated() {
        assert!(RotatorConfig::default().validate().is_ok(), "spento senza modello va bene");
        let on = RotatorConfig { enabled: true, ..RotatorConfig::default() };
        assert_eq!(on.validate().map_err(|e| e.code), Err("bad_model"));
        let remote = RotatorConfig { enabled: true, connection: RotConnection::Remote, host: "".into(), ..RotatorConfig::default() };
        assert_eq!(remote.validate().map_err(|e| e.code), Err("bad_host"));
        let remote = RotatorConfig { enabled: true, connection: RotConnection::Remote, tcp_port: 0, ..RotatorConfig::default() };
        assert_eq!(remote.validate().map_err(|e| e.code), Err("bad_tcp_port"));
    }

    #[test]
    fn the_client_reads_moves_and_stops() {
        let fake = FakeRotctld::start(123.0);
        let mut client = RotctldClient::connect(fake.addr, Duration::from_secs(1), Duration::from_secs(1)).expect("collegato");
        assert_eq!(client.position().expect("posizione"), (123.0, 0.0));
        client.set_position(270.0, 0.0).expect("vai");
        assert_eq!(client.position().expect("posizione").0, 270.0);
        client.stop().expect("ferma");
        fake.refuse.store(true, Ordering::SeqCst);
        let err = client.set_position(10.0, 0.0).expect_err("rifiutato");
        assert!(err.0.contains("(-9)"), "{err}");
        assert!(fake.log.lock().expect("log").contains(&"P 270.0 0.0".to_string()));
    }

    #[test]
    fn the_service_connects_to_a_remote_rotctld_and_turns_the_antenna() {
        let fake = FakeRotctld::start(45.0);
        let path = temp_file("remoto");
        let service = RotatorService::open(&path);
        assert_eq!(service.goto(90.0, None).map_err(|e| e.code), Err("not_connected"));
        let config = RotatorConfig { enabled: true, connection: RotConnection::Remote, host: "127.0.0.1".into(), tcp_port: fake.addr.port(), ..RotatorConfig::default() };
        service.apply_config(config.clone()).expect("salvata");
        let s = wait_for(&service, |s| s.connected && s.az == Some(45.0));
        assert_eq!((s.state, s.az), (RotState::Connected, Some(45.0)));
        service.goto(-10.0, None).expect("vai");
        let s = wait_for(&service, |s| s.az == Some(350.0) && !s.moving);
        assert_eq!((s.az, s.target, s.moving), (Some(350.0), Some(350.0), false));
        assert_eq!(fake.pos.lock().expect("pos").0, 350.0);
        service.stop().expect("ferma");
        assert_eq!(service.status().target, None);
        assert_eq!(service.goto(f64::NAN, None).map_err(|e| e.code), Err("bad_azimuth"));
        // la configurazione resta per il prossimo avvio
        assert_eq!(RotatorService::open(&path).config(), config);
    }

    #[test]
    fn an_unreachable_rotctld_is_an_error_not_a_hang() {
        let port = TcpListener::bind("127.0.0.1:0").expect("porta").local_addr().expect("indirizzo").port();
        let service = RotatorService::open(&temp_file("assente"));
        let config = RotatorConfig { enabled: true, connection: RotConnection::Remote, host: "127.0.0.1".into(), tcp_port: port, ..RotatorConfig::default() };
        service.apply_config(config).expect("salvata");
        let s = wait_for(&service, |s| s.state == RotState::Error);
        assert_eq!(s.state, RotState::Error);
        assert!(s.error.is_some());
        service.apply_config(RotatorConfig::default()).expect("spento");
        let s = wait_for(&service, |s| s.state == RotState::Off);
        assert_eq!(s.state, RotState::Off);
    }

    #[test]
    fn a_broken_file_starts_from_defaults() {
        let path = temp_file("guasto");
        std::fs::write(&path, "{ non json").expect("scritto");
        assert_eq!(read_config(&path), RotatorConfig::default());
        assert!(path.with_extension("json.bad").exists());
    }

    #[test]
    fn the_model_list_of_rotctld_is_read() {
        let text = " Rot #  Mfg                    Model                   Version         Status      Macro\n     1  Hamlib                 Dummy                   20221128.0      Stable      ROT_MODEL_DUMMY\n   603  Yaesu                  GS-232B                 20220109.0      Stable      ROT_MODEL_GS232B\n";
        let models = parse_model_list(text);
        assert_eq!(models.len(), 2);
        assert_eq!(models[1].id, 603);
        assert_eq!(models[1].model, "GS-232B");
    }
}
