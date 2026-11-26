//! Reverse Beacon Network: gli spot dei ricevitori automatici (porta 7000 CW e RTTY, porta 7001 FT8 e FT4), raccolti per nominativo,
//! banda e modo (quanti ricevitori l'hanno sentito, S/N migliore), e il server telnet locale che offre ai programmi di log gli spot
//! verificati di tutta la banda di Aurora nel formato dei cluster. Tutto spento di fabbrica; si accende da Impostazioni › Log, cluster e servizi.

use std::{
    collections::{BTreeMap, BTreeSet},
    net::{IpAddr, Ipv4Addr, SocketAddr},
    path::{Path, PathBuf},
    sync::{
        Arc, Mutex,
        atomic::{AtomicUsize, Ordering},
    },
    time::Duration,
};

use serde::{Deserialize, Serialize};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
    sync::broadcast,
    task::JoinHandle,
};

use crate::{Enricher, spot, telnet, unix_ms, valid_callsign};

pub const RBN_HOST: &str = "telnet.reversebeacon.net";
/// Porta degli spot CW e RTTY.
pub const PORT_CW_RTTY: u16 = 7000;
/// Porta degli spot FT8 e FT4.
pub const PORT_DIGITAL: u16 = 7001;
/// Dopo quanto uno spot RBN si scarta (gli spot dei ricevitori si ripetono di continuo finche' la stazione e' in aria).
pub const MAX_AGE_MS: u64 = 10 * 60_000;
/// Quante voci (nominativo, banda, modo) tenere al massimo.
const MAX_ENTRIES: usize = 3_000;
/// Clienti del server locale collegati insieme, al massimo.
pub const MAX_CLIENTS: usize = 16;
const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);
const MAX_LINE: usize = 2_000;

const CONTINENTS: &[&str] = &["EU", "NA", "SA", "AS", "AF", "OC", "AN"];
const MODES: &[&str] = &["CW", "RTTY", "FT8", "FT4", "PSK31", "PSK63"];

fn default_host() -> String {
    RBN_HOST.to_string()
}

fn default_true() -> bool {
    true
}

fn default_server_port() -> u16 {
    7300
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RbnConfig {
    /// Ricezione degli spot RBN accesa. Spenta di fabbrica.
    #[serde(default)]
    pub enabled: bool,
    /// Porta 7000: CW e RTTY.
    #[serde(default = "default_true")]
    pub cw_rtty: bool,
    /// Porta 7001: FT8 e FT4.
    #[serde(default)]
    pub digital: bool,
    /// Bande da tenere (`20m`, `40m`...); vuoto = tutte.
    #[serde(default)]
    pub bands: Vec<String>,
    /// Modi da tenere (`CW`, `RTTY`, `FT8`, `FT4`); vuoto = tutti.
    #[serde(default)]
    pub modes: Vec<String>,
    /// Continenti dei ricevitori da tenere (`EU`, `NA`...); vuoto = tutti.
    #[serde(default)]
    pub continents: Vec<String>,
    /// Server telnet locale con gli spot verificati di tutta la banda di Aurora. Spento di fabbrica.
    #[serde(default)]
    pub server_enabled: bool,
    #[serde(default = "default_server_port")]
    pub server_port: u16,
    /// Il server si apre anche agli altri computer della rete di casa (altrimenti solo a questo computer).
    #[serde(default)]
    pub server_lan: bool,
    /// Indirizzo della rete RBN (si cambia solo per le prove).
    #[serde(default = "default_host", skip_serializing)]
    pub host: String,
}

impl Default for RbnConfig {
    fn default() -> RbnConfig {
        RbnConfig {
            enabled: false,
            cw_rtty: true,
            digital: false,
            bands: Vec::new(),
            modes: Vec::new(),
            continents: Vec::new(),
            server_enabled: false,
            server_port: default_server_port(),
            server_lan: false,
            host: default_host(),
        }
    }
}

impl RbnConfig {
    pub fn validate(&self) -> Result<(), String> {
        if self.bands.len() > 20 || self.bands.iter().any(|b| !spot::BAND_NAMES.contains(&b.as_str())) {
            return Err("bands: banda sconosciuta".into());
        }
        if self.modes.iter().any(|m| !MODES.contains(&m.as_str())) {
            return Err("modes: modo sconosciuto".into());
        }
        if self.continents.iter().any(|k| !CONTINENTS.contains(&k.as_str())) {
            return Err("continents: continente sconosciuto".into());
        }
        if self.server_port < 1024 {
            return Err("server_port: tra 1024 e 65535".into());
        }
        if self.host.is_empty() || !self.host.chars().all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '-') {
            return Err("host: indirizzo non valido".into());
        }
        Ok(())
    }
}

/// Che cosa dice il ricevitore della stazione.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Kind {
    Cq,
    Beacon,
    Ncdxf,
    Dx,
}

/// Una riga di spot RBN: `DX de EA5WU-#:   14025.0  K1ABC   CW    12 dB  25 WPM  CQ      1200Z`.
#[derive(Debug, Clone, PartialEq)]
pub struct RbnLine {
    /// Il ricevitore, senza `-#`.
    pub receiver: String,
    pub freq_khz: f64,
    pub call: String,
    pub mode: String,
    pub snr_db: Option<i32>,
    /// Velocita': parole al minuto in CW, baud in RTTY.
    pub speed: Option<u32>,
    pub kind: Kind,
    /// Ora UTC, `HHMM`.
    pub time_utc: String,
}

/// Legge una riga del Reverse Beacon Network; `None` se non e' uno spot.
pub fn parse_line(line: &str) -> Option<RbnLine> {
    let p = spot::parse_spot(line)?;
    let words: Vec<&str> = p.comment.split_whitespace().collect();
    let upper: Vec<String> = words.iter().map(|w| w.to_ascii_uppercase()).collect();
    let mode = upper.first().filter(|w| MODES.contains(&w.as_str()) || w.chars().all(|c| c.is_ascii_alphanumeric()))?.clone();
    let number_before = |unit: &[&str]| upper.windows(2).find(|w| unit.contains(&w[1].as_str())).and_then(|w| w[0].parse::<i64>().ok());
    let snr_db = number_before(&["DB"]).and_then(|n| i32::try_from(n).ok());
    let speed = number_before(&["WPM", "BPS", "BD"]).and_then(|n| u32::try_from(n).ok());
    let kind = if upper.iter().any(|w| w == "NCDXF") {
        Kind::Ncdxf
    } else if upper.iter().any(|w| w == "BEACON" || w == "BCN") {
        Kind::Beacon
    } else if upper.iter().any(|w| w == "CQ") {
        Kind::Cq
    } else {
        Kind::Dx
    };
    Some(RbnLine {
        receiver: p.spotter.split('-').next().unwrap_or(&p.spotter).to_uppercase(),
        freq_khz: p.freq_khz,
        call: p.dx_call,
        mode,
        snr_db,
        speed,
        kind,
        time_utc: p.time_utc,
    })
}

/// Una stazione sentita dalla rete RBN: tutti i ricevitori dello stesso nominativo, banda e modo insieme.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct RbnSpot {
    pub call: String,
    /// Frequenza dello spot con il S/N migliore.
    pub freq_khz: f64,
    pub band: &'static str,
    pub mode: String,
    /// Quanti ricevitori l'hanno sentita (negli ultimi 10 minuti).
    pub receivers: usize,
    /// Il ricevitore con il S/N migliore.
    pub best_receiver: String,
    pub best_snr_db: Option<i32>,
    pub speed: Option<u32>,
    pub kind: Kind,
    pub time_utc: String,
    pub last_ms: u64,
}

struct Entry {
    spot: RbnSpot,
    heard: BTreeMap<String, u64>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct RbnStatus {
    pub port: u16,
    /// `connecting`, `connected` o `error`.
    pub state: &'static str,
    pub detail: Option<String>,
    pub spots: u64,
}

pub struct RbnService {
    path: PathBuf,
    config: Mutex<RbnConfig>,
    entries: Mutex<BTreeMap<(String, &'static str, String), Entry>>,
    status: Mutex<BTreeMap<u16, RbnStatus>>,
    tasks: Mutex<Vec<JoinHandle<()>>>,
    enricher: Mutex<Option<Enricher>>,
    station_call: Mutex<String>,
    /// Le righe del server locale, a tutti i clienti collegati.
    local: broadcast::Sender<String>,
    clients: Arc<AtomicUsize>,
    server_state: Mutex<Option<String>>,
}

fn lock<T>(m: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|p| p.into_inner())
}

impl RbnService {
    /// Legge le impostazioni salvate (o quelle di fabbrica). Non si collega a nulla.
    pub fn open(path: &Path) -> Arc<RbnService> {
        let config = std::fs::read_to_string(path)
            .ok()
            .and_then(|text| serde_json::from_str::<RbnConfig>(&text).ok())
            .filter(|c| c.validate().is_ok())
            .unwrap_or_default();
        let (local, _) = broadcast::channel(256);
        Arc::new(RbnService {
            path: path.to_path_buf(),
            config: Mutex::new(config),
            entries: Mutex::new(BTreeMap::new()),
            status: Mutex::new(BTreeMap::new()),
            tasks: Mutex::new(Vec::new()),
            enricher: Mutex::new(None),
            station_call: Mutex::new(String::new()),
            local,
            clients: Arc::new(AtomicUsize::new(0)),
            server_state: Mutex::new(None),
        })
    }

    pub fn config(&self) -> RbnConfig {
        lock(&self.config).clone()
    }

    pub fn set_enricher(&self, enricher: Enricher) {
        *lock(&self.enricher) = Some(enricher);
    }

    /// Il nominativo di stazione: con questo si entra nella rete RBN e si firmano gli spot del server locale.
    pub fn set_station_call(self: &Arc<Self>, call: &str) {
        let call = call.trim().to_uppercase();
        let changed = {
            let mut now = lock(&self.station_call);
            let changed = *now != call;
            *now = call;
            changed
        };
        let c = self.config();
        if changed && (c.enabled || c.server_enabled) {
            self.restart();
        }
    }

    fn callsign(&self) -> String {
        lock(&self.station_call).clone()
    }

    pub fn set_config(self: &Arc<Self>, config: RbnConfig) -> Result<(), String> {
        config.validate()?;
        let text = serde_json::to_string_pretty(&config).map_err(|e| e.to_string())?;
        let tmp = self.path.with_extension("json.tmp");
        std::fs::write(&tmp, text).and_then(|()| std::fs::rename(&tmp, &self.path)).map_err(|e| format!("salvataggio: {e}"))?;
        *lock(&self.config) = config;
        self.restart();
        Ok(())
    }

    /// Chiude collegamenti e server e li riapre secondo le impostazioni.
    pub fn restart(self: &Arc<Self>) {
        for task in lock(&self.tasks).drain(..) {
            task.abort();
        }
        lock(&self.status).clear();
        *lock(&self.server_state) = None;
        let config = self.config();
        let call = self.callsign();
        let mut tasks = lock(&self.tasks);
        if config.enabled && valid_callsign(&call) {
            let ports = [(config.cw_rtty, PORT_CW_RTTY), (config.digital, PORT_DIGITAL)];
            for (_, port) in ports.into_iter().filter(|(on, _)| *on) {
                tasks.push(tokio::spawn(run_feed(Arc::clone(self), config.host.clone(), port, call.clone())));
            }
        }
        if config.server_enabled {
            tasks.push(tokio::spawn(run_local_server(Arc::clone(self), config.server_port, config.server_lan)));
        }
    }

    pub fn statuses(&self) -> Vec<RbnStatus> {
        lock(&self.status).values().cloned().collect()
    }

    /// Stato del server locale: `None` spento, altrimenti `listening` o l'errore; e quanti clienti sono collegati.
    pub fn server_status(&self) -> (Option<String>, usize) {
        (lock(&self.server_state).clone(), self.clients.load(Ordering::Relaxed))
    }

    fn set_status(&self, port: u16, state: &'static str, detail: Option<String>) {
        let mut all = lock(&self.status);
        let spots = all.get(&port).map_or(0, |s| s.spots);
        all.insert(port, RbnStatus { port, state, detail, spots });
    }

    /// Le stazioni sentite di recente, della banda e del modo chiesti (se chiesti) e dentro `lo_khz..=hi_khz`; prima le piu' sentite.
    pub fn spots(&self, band: Option<&str>, mode: Option<&str>, lo_khz: f64, hi_khz: f64) -> Vec<RbnSpot> {
        let now = unix_ms();
        let mut entries = lock(&self.entries);
        entries.retain(|_, e| now.saturating_sub(e.spot.last_ms) <= MAX_AGE_MS);
        let mut out: Vec<RbnSpot> = entries
            .values_mut()
            .filter(|e| band.is_none_or(|b| e.spot.band.eq_ignore_ascii_case(b)) && mode.is_none_or(|m| e.spot.mode.eq_ignore_ascii_case(m)))
            .filter(|e| e.spot.freq_khz >= lo_khz && e.spot.freq_khz <= hi_khz)
            .map(|e| {
                e.heard.retain(|_, at| now.saturating_sub(*at) <= MAX_AGE_MS);
                e.spot.receivers = e.heard.len().max(1);
                e.spot.clone()
            })
            .collect();
        out.sort_by(|a, b| b.receivers.cmp(&a.receivers).then(b.last_ms.cmp(&a.last_ms)));
        out
    }

    /// Una riga letta dalla rete: filtri delle impostazioni, poi entra nella voce del suo nominativo, banda e modo.
    pub fn push_line(&self, line: &RbnLine) -> bool {
        let Some(band) = spot::band_of_khz(line.freq_khz) else {
            return false;
        };
        let config = self.config();
        if (!config.bands.is_empty() && !config.bands.iter().any(|b| b == band)) || (!config.modes.is_empty() && !config.modes.contains(&line.mode)) {
            return false;
        }
        if !config.continents.is_empty() {
            let continent = lock(&self.enricher).as_ref().and_then(|e| e(&line.receiver)).map(|(_, k)| k);
            if !continent.is_some_and(|k| config.continents.contains(&k)) {
                return false;
            }
        }
        let now = unix_ms();
        let mut entries = lock(&self.entries);
        let key = (line.call.clone(), band, line.mode.clone());
        let entry = entries.entry(key).or_insert_with(|| Entry {
            spot: RbnSpot {
                call: line.call.clone(),
                freq_khz: line.freq_khz,
                band,
                mode: line.mode.clone(),
                receivers: 0,
                best_receiver: line.receiver.clone(),
                best_snr_db: line.snr_db,
                speed: line.speed,
                kind: line.kind,
                time_utc: line.time_utc.clone(),
                last_ms: now,
            },
            heard: BTreeMap::new(),
        });
        entry.heard.insert(line.receiver.clone(), now);
        if entry.heard.len() > 200
            && let Some(oldest) = entry.heard.iter().min_by_key(|(_, at)| **at).map(|(k, _)| k.clone())
        {
            entry.heard.remove(&oldest);
        }
        let s = &mut entry.spot;
        let stale = now.saturating_sub(s.last_ms) > MAX_AGE_MS;
        if stale || line.snr_db > s.best_snr_db || s.best_receiver == line.receiver {
            s.freq_khz = line.freq_khz;
            s.best_snr_db = line.snr_db;
            s.best_receiver = line.receiver.clone();
        }
        s.speed = line.speed.or(s.speed);
        s.kind = line.kind;
        s.time_utc = line.time_utc.clone();
        s.last_ms = now;
        s.receivers = entry.heard.len();
        if entries.len() > MAX_ENTRIES {
            // prima le scadute (oltre il tempo massimo): di solito basta, e il giro costoso non scatta a ogni riga sopra il tetto
            entries.retain(|_, e| now.saturating_sub(e.spot.last_ms) <= MAX_AGE_MS);
        }
        if entries.len() > MAX_ENTRIES {
            let oldest: BTreeSet<_> = {
                let mut all: Vec<_> = entries.iter().map(|(k, e)| (e.spot.last_ms, k.clone())).collect();
                all.sort();
                all.into_iter().take(entries.len() - MAX_ENTRIES).map(|(_, k)| k).collect()
            };
            entries.retain(|k, _| !oldest.contains(k));
        }
        true
    }

    /// Uno spot verificato di tutta la banda di Aurora (stazione in CQ) ai clienti del server locale, nel formato dei cluster. Nessuno
    /// collegato o server spento: non fa niente.
    pub fn publish_local(&self, call: &str, freq_khz: f64, mode: &str, snr_db: f32, speed: Option<f32>, cq: bool) {
        if self.local.receiver_count() == 0 {
            return;
        }
        let me = self.callsign();
        if !valid_callsign(&me) {
            return;
        }
        let secs = unix_ms() / 1000 % 86_400;
        let line = format_local_line(&me, freq_khz, call, mode, snr_db, speed, cq, &format!("{:02}{:02}", secs / 3600, secs / 60 % 60));
        let _ = self.local.send(line);
    }
}

/// La riga del server locale, come quelle dei ricevitori della rete RBN: `DX de IU3ABC-#:  14025.0  K1ABC  CW  14 dB  22 WPM  CQ  1200Z`.
#[allow(clippy::too_many_arguments)]
pub fn format_local_line(me: &str, freq_khz: f64, call: &str, mode: &str, snr_db: f32, speed: Option<f32>, cq: bool, hhmm: &str) -> String {
    let spotter = format!("{me}-#:");
    let unit = if mode.eq_ignore_ascii_case("RTTY") { "BPS" } else { "WPM" };
    let speed = speed.map(|s| format!("{s:.0} {unit}")).unwrap_or_default();
    let kind = if cq { "CQ" } else { "DX" };
    format!("DX de {spotter:<11}{freq_khz:>9.1}  {call:<12} {mode:<5} {snr_db:>3.0} dB  {speed:<7} {kind:<7} {hhmm}Z")
}

/// Tiene collegata una porta RBN: se cade riprova con attese sempre piu' lunghe (da 5 s a 2 minuti).
async fn run_feed(svc: Arc<RbnService>, host: String, port: u16, call: String) {
    let mut wait = 5u64;
    loop {
        svc.set_status(port, "connecting", None);
        let started = std::time::Instant::now();
        let detail = match feed_session(&svc, &host, port, &call).await {
            Ok(()) => "connessione chiusa dal server".to_string(),
            Err(e) => e,
        };
        tracing::info!("rbn {port}: {detail}");
        svc.set_status(port, "error", Some(detail));
        if started.elapsed() > Duration::from_secs(60) {
            wait = 5;
        }
        tokio::time::sleep(Duration::from_secs(wait)).await;
        wait = (wait * 2).min(120);
    }
}

async fn feed_session(svc: &Arc<RbnService>, host: &str, port: u16, call: &str) -> Result<(), String> {
    let stream = tokio::time::timeout(CONNECT_TIMEOUT, TcpStream::connect((host, port)))
        .await
        .map_err(|_| "nessuna risposta dal server".to_string())?
        .map_err(|e| format!("collegamento non riuscito: {e}"))?;
    let (mut reader, mut writer) = stream.into_split();
    svc.set_status(port, "connected", None);
    let mut decoder = telnet::Decoder::default();
    let mut line: Vec<u8> = Vec::new();
    let mut buf = [0u8; 4096];
    let mut logged_in = false;
    loop {
        // il flusso RBN grezzo non sta fermo cinque minuti: senza dati (PC sospeso, NAT che chiude) si ricollega invece di restare «connected»
        let n = tokio::time::timeout(std::time::Duration::from_secs(300), reader.read(&mut buf))
            .await
            .map_err(|_| "nessun dato da 5 minuti".to_string())?
            .map_err(|e| format!("lettura: {e}"))?;
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
                    if let Some(parsed) = parse_line(&text) {
                        logged_in = true;
                        if svc.push_line(&parsed)
                            && let Some(s) = lock(&svc.status).get_mut(&port)
                        {
                            s.spots += 1;
                        }
                    } else if !logged_in && telnet::is_login_prompt(&text) {
                        logged_in = true;
                        writer.write_all(format!("{call}\r\n").as_bytes()).await.map_err(|e| format!("scrittura: {e}"))?;
                    }
                }
                control if control < 0x20 && control != b'\t' => {}
                other => line.push(other),
            }
            if line.len() > MAX_LINE {
                line.clear();
            }
        }
        // la richiesta del nominativo non finisce con un a capo
        if !logged_in && telnet::is_login_prompt(&String::from_utf8_lossy(&line)) {
            logged_in = true;
            line.clear();
            writer.write_all(format!("{call}\r\n").as_bytes()).await.map_err(|e| format!("scrittura: {e}"))?;
        }
    }
}

/// Il server telnet locale: chiede il nominativo, saluta e manda gli spot di Aurora man mano che arrivano.
async fn run_local_server(svc: Arc<RbnService>, port: u16, lan: bool) {
    let ip = if lan { IpAddr::V4(Ipv4Addr::UNSPECIFIED) } else { IpAddr::V4(Ipv4Addr::LOCALHOST) };
    let listener = match TcpListener::bind(SocketAddr::new(ip, port)).await {
        Ok(l) => l,
        Err(e) => {
            tracing::warn!("server spot locale sulla porta {port}: {e}");
            *lock(&svc.server_state) = Some(format!("porta {port} non disponibile: {e}"));
            return;
        }
    };
    *lock(&svc.server_state) = Some("listening".into());
    serve(listener, svc).await;
}

async fn serve(listener: TcpListener, svc: Arc<RbnService>) {
    loop {
        let Ok((stream, _)) = listener.accept().await else {
            tokio::time::sleep(Duration::from_millis(200)).await;
            continue;
        };
        if svc.clients.load(Ordering::Relaxed) >= MAX_CLIENTS {
            drop(stream);
            continue;
        }
        let rx = svc.local.subscribe();
        let clients = Arc::clone(&svc.clients);
        clients.fetch_add(1, Ordering::Relaxed);
        tokio::spawn(async move {
            let _ = client(stream, rx).await;
            clients.fetch_sub(1, Ordering::Relaxed);
        });
    }
}

async fn client(stream: TcpStream, mut rx: broadcast::Receiver<String>) -> Result<(), std::io::Error> {
    let (mut reader, mut writer) = stream.into_split();
    writer.write_all(b"Aurora spot server\r\nPlease enter your call: ").await?;
    let mut buf = [0u8; 512];
    let mut typed = Vec::new();
    let call = loop {
        let n = tokio::time::timeout(Duration::from_secs(60), reader.read(&mut buf)).await.map_err(|_| std::io::ErrorKind::TimedOut)??;
        if n == 0 {
            return Ok(());
        }
        typed.extend(buf[..n].iter().filter(|b| **b != 0xFF && **b != b'\r'));
        if let Some(end) = typed.iter().position(|b| *b == b'\n') {
            let text: String = String::from_utf8_lossy(&typed[..end]).chars().filter(|c| c.is_ascii_alphanumeric() || *c == '/' || *c == '-').collect();
            break text.to_uppercase();
        }
        if typed.len() > 64 {
            return Ok(());
        }
    };
    if !valid_callsign(&call) {
        writer.write_all(b"Invalid callsign\r\n").await?;
        return Ok(());
    }
    writer.write_all(format!("Hello {call}, spots from Aurora follow.\r\n{call} de AURORA >\r\n").as_bytes()).await?;
    loop {
        tokio::select! {
            line = rx.recv() => match line {
                Ok(line) => writer.write_all(format!("{line}\r\n").as_bytes()).await?,
                Err(broadcast::error::RecvError::Lagged(_)) => {}
                Err(broadcast::error::RecvError::Closed) => return Ok(()),
            },
            read = reader.read(&mut buf) => {
                let n = read?;
                let text = String::from_utf8_lossy(&buf[..n]).to_ascii_lowercase();
                if n == 0 || text.trim() == "bye" || text.trim() == "quit" {
                    return Ok(());
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::io::{AsyncBufReadExt, BufReader};

    #[test]
    fn reads_a_cw_spot() {
        let l = parse_line("DX de EA5WU-#:   14025.0  K1ABC          CW    12 dB  25 WPM  CQ      1200Z").unwrap();
        assert_eq!((l.receiver.as_str(), l.call.as_str(), l.mode.as_str()), ("EA5WU", "K1ABC", "CW"));
        assert_eq!(l.freq_khz, 14025.0);
        assert_eq!((l.snr_db, l.speed, l.kind), (Some(12), Some(25), Kind::Cq));
        assert_eq!(l.time_utc, "1200");
    }

    #[test]
    fn reads_rtty_beacons_ncdxf_and_ft8() {
        let r = parse_line("DX de W3OA-#:    14085.5  DL1ABC         RTTY  21 dB  45 BPS  CQ      0931Z").unwrap();
        assert_eq!((r.mode.as_str(), r.speed, r.snr_db), ("RTTY", Some(45), Some(21)));
        let b = parse_line("DX de DK9IP-#:    28222.0  DK0TEN         CW     7 dB  12 WPM  BEACON  1745Z").unwrap();
        assert_eq!(b.kind, Kind::Beacon);
        let n = parse_line("DX de OH6BG-#:   14100.0  4U1UN          CW    18 dB  22 WPM  NCDXF B 2316Z").unwrap();
        assert_eq!(n.kind, Kind::Ncdxf);
        let f = parse_line("DX de K9IMM-#:    7074.0  KD2ZTV         FT8   -8 dB  1234 Hz CQ      1913Z").unwrap();
        assert_eq!((f.mode.as_str(), f.snr_db, f.kind, f.speed), ("FT8", Some(-8), Kind::Cq, None));
        let d = parse_line("DX de VE2WU-#:    3530.1  W1AW           CW    30 dB  28 WPM  DX      0102Z").unwrap();
        assert_eq!(d.kind, Kind::Dx);
        assert!(parse_line("Welcome to the Reverse Beacon Network").is_none());
        assert!(parse_line("Please enter your call:").is_none());
    }

    fn svc(name: &str) -> Arc<RbnService> {
        let dir = std::env::temp_dir().join(format!("aurora-rbn-{name}-{}", std::process::id()));
        std::fs::remove_dir_all(&dir).ok();
        std::fs::create_dir_all(&dir).unwrap();
        RbnService::open(&dir.join("rbn.json"))
    }

    #[test]
    fn groups_the_receivers_of_a_station_and_keeps_the_best_snr() {
        let s = svc("groups");
        for line in [
            "DX de EA5WU-#:   14025.0  K1ABC   CW  12 dB  25 WPM  CQ  1200Z",
            "DX de DK9IP-#:   14025.1  K1ABC   CW  20 dB  25 WPM  CQ  1200Z",
            "DX de EA5WU-#:   14025.0  K1ABC   CW  11 dB  25 WPM  CQ  1201Z",
            "DX de DK9IP-#:    7012.0  K1ABC   CW  20 dB  25 WPM  CQ  1200Z",
        ] {
            assert!(s.push_line(&parse_line(line).unwrap()));
        }
        let all = s.spots(Some("20m"), Some("CW"), 14000.0, 14100.0);
        assert_eq!(all.len(), 1);
        assert_eq!((all[0].receivers, all[0].best_snr_db, all[0].freq_khz), (2, Some(20), 14025.1));
        assert!(s.spots(Some("20m"), Some("CW"), 14030.0, 14100.0).is_empty(), "fuori dalla cascata");
        assert_eq!(s.spots(None, None, 0.0, 1e9).len(), 2);
    }

    #[test]
    fn the_filters_drop_bands_modes_and_continents() {
        let s = svc("filters");
        *lock(&s.config) = RbnConfig { bands: vec!["40m".into()], modes: vec!["CW".into()], continents: vec!["EU".into()], ..RbnConfig::default() };
        s.set_enricher(Arc::new(|c: &str| Some(("x".into(), if c.starts_with('E') { "EU".into() } else { "NA".into() }))));
        assert!(!s.push_line(&parse_line("DX de EA5WU-#: 14025.0 K1ABC CW 12 dB 25 WPM CQ 1200Z").unwrap()), "banda");
        assert!(!s.push_line(&parse_line("DX de EA5WU-#: 7040.0 K1ABC RTTY 12 dB 45 BPS CQ 1200Z").unwrap()), "modo");
        assert!(!s.push_line(&parse_line("DX de W3OA-#: 7020.0 K1ABC CW 12 dB 25 WPM CQ 1200Z").unwrap()), "continente");
        assert!(s.push_line(&parse_line("DX de EA5WU-#: 7020.0 K1ABC CW 12 dB 25 WPM CQ 1200Z").unwrap()));
    }

    #[test]
    fn defaults_are_off_and_bad_values_are_refused() {
        let c = RbnConfig::default();
        assert!(c.validate().is_ok() && !c.enabled && !c.server_enabled);
        assert!(RbnConfig { bands: vec!["21m".into()], ..c.clone() }.validate().is_err());
        assert!(RbnConfig { continents: vec!["XX".into()], ..c.clone() }.validate().is_err());
        assert!(RbnConfig { server_port: 23, ..c }.validate().is_err());
    }

    #[test]
    fn the_local_line_reads_back_as_an_rbn_spot() {
        let line = format_local_line("IU3ABC", 14025.04, "K1ABC", "CW", 14.2, Some(22.0), true, "1200");
        let l = parse_line(&line).unwrap();
        assert_eq!((l.receiver.as_str(), l.call.as_str(), l.mode.as_str()), ("IU3ABC", "K1ABC", "CW"));
        assert_eq!((l.snr_db, l.speed, l.kind, l.freq_khz), (Some(14), Some(22), Kind::Cq, 14025.0));
    }

    #[tokio::test]
    async fn the_local_server_asks_the_call_and_streams_spots() {
        let s = svc("server");
        s.set_station_call("IU3ABC");
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        tokio::spawn(serve(listener, Arc::clone(&s)));
        let stream = TcpStream::connect(("127.0.0.1", port)).await.unwrap();
        let (r, mut w) = stream.into_split();
        let mut r = BufReader::new(r);
        let mut banner = String::new();
        r.read_line(&mut banner).await.unwrap();
        assert!(banner.contains("Aurora"));
        w.write_all(b"IU3ABC-2\r\n").await.unwrap();
        let mut hello = String::new();
        r.read_line(&mut hello).await.unwrap();
        assert!(hello.contains("IU3ABC-2"), "{hello:?}");
        let mut prompt = String::new();
        r.read_line(&mut prompt).await.unwrap();
        for _ in 0..50 {
            if s.local.receiver_count() > 0 {
                break;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
        s.publish_local("K1ABC", 14025.0, "CW", 14.0, Some(22.0), true);
        let mut spot = String::new();
        tokio::time::timeout(Duration::from_secs(5), r.read_line(&mut spot)).await.unwrap().unwrap();
        let l = parse_line(spot.trim_end()).unwrap();
        assert_eq!((l.receiver.as_str(), l.call.as_str(), l.kind), ("IU3ABC", "K1ABC", Kind::Cq));
    }

    #[tokio::test]
    async fn logs_in_to_the_feed_and_collects_spots() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        tokio::spawn(async move {
            let (s, _) = listener.accept().await.unwrap();
            let (r, mut w) = s.into_split();
            w.write_all(b"Welcome to the Reverse Beacon Network.\r\n\r\nPlease enter your call: ").await.unwrap();
            let mut r = BufReader::new(r);
            let mut call = String::new();
            r.read_line(&mut call).await.unwrap();
            assert_eq!(call.trim(), "IU3ABC");
            w.write_all(b"DX de EA5WU-#:   14025.0  K1ABC   CW  12 dB  25 WPM  CQ  1200Z\r\n").await.unwrap();
            tokio::time::sleep(Duration::from_secs(2)).await;
        });
        let s = svc("feed");
        let task = tokio::spawn(run_feed(Arc::clone(&s), "127.0.0.1".into(), port, "IU3ABC".into()));
        for _ in 0..100 {
            if !s.spots(None, None, 0.0, 1e9).is_empty() {
                break;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
        task.abort();
        assert_eq!(s.spots(None, None, 0.0, 1e9)[0].call, "K1ABC");
    }
}
