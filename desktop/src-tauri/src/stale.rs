//! Un Aurora vecchio rimasto acceso. Windows lascia che due programmi ascoltino sulla stessa porta, uno su `0.0.0.0` e l'altro su
//! `127.0.0.1`: la finestra del programma nuovo (che apre `127.0.0.1`) finiva sul core vecchio e mostrava la versione vecchia anche
//! dopo l'aggiornamento (2026-11-09, un Aurora 0.11.155 sopravvissuto all'aggiornamento alla 0.11.204). All'avvio si chiede la versione
//! a chi risponde su `127.0.0.1:porta`; se e' un Aurora diverso da questo, si chiude il processo che tiene la porta.

use std::io::{Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::time::Duration;

/// La versione dell'Aurora che risponde su `127.0.0.1:port`, se ce n'e' uno.
fn answering_version(port: u16) -> Option<String> {
    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    let mut stream = TcpStream::connect_timeout(&addr, Duration::from_millis(400)).ok()?;
    stream.set_read_timeout(Some(Duration::from_millis(1500))).ok()?;
    stream.write_all(format!("GET /api/v1/health HTTP/1.0\r\nHost: 127.0.0.1:{port}\r\n\r\n").as_bytes()).ok()?;
    let mut text = String::new();
    let _ = stream.read_to_string(&mut text);
    let body = text.split("\r\n\r\n").nth(1)?;
    let value: serde_json::Value = serde_json::from_str(body.trim()).ok()?;
    (value.get("name")?.as_str()? == "Aurora").then(|| value.get("version")?.as_str().map(str::to_string))?
}

/// I processi che tengono `port` dal lato server (in ascolto o con connessioni aperte, da `netstat -ano`), escluso questo.
#[cfg(windows)]
fn listeners(port: u16) -> Vec<u32> {
    use std::os::windows::process::CommandExt;
    let Ok(out) = std::process::Command::new("netstat").args(["-ano", "-p", "TCP"]).creation_flags(0x0800_0000).output() else { return Vec::new() };
    let me = std::process::id();
    let suffix = format!(":{port}");
    let mut pids: Vec<u32> = String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|line| {
            let cols: Vec<&str> = line.split_whitespace().collect();
            // chi ascolta sulla porta, ma anche chi tiene ancora connessioni aperte dal lato server: il 2026-11-09 il core 0.11.155 non
            // compariva piu' in ascolto ma serviva ancora la finestra sulle connessioni gia' aperte (ESTABLISHED, indirizzo locale :8787)
            (cols.len() >= 5 && cols[1].ends_with(&suffix) && !cols[3].eq_ignore_ascii_case("TIME_WAIT")).then(|| cols[4].parse::<u32>().ok()).flatten()
        })
        .filter(|pid| *pid != me && *pid > 4)
        .collect();
    pids.sort_unstable();
    pids.dedup();
    pids
}

#[cfg(not(windows))]
fn listeners(_port: u16) -> Vec<u32> {
    Vec::new()
}

#[cfg(windows)]
fn kill(pid: u32) {
    use std::os::windows::process::CommandExt;
    let _ = std::process::Command::new("taskkill").args(["/F", "/T", "/PID", &pid.to_string()]).creation_flags(0x0800_0000).status();
}

#[cfg(not(windows))]
fn kill(pid: u32) {
    let _ = std::process::Command::new("kill").args(["-9", &pid.to_string()]).status();
}

/// Il nome del programma del processo `pid` (`aurora-desktop.exe`), da `tasklist`.
#[cfg(windows)]
fn image_name(pid: u32) -> String {
    use std::os::windows::process::CommandExt;
    std::process::Command::new("tasklist")
        .args(["/FI", &format!("PID eq {pid}"), "/FO", "CSV", "/NH"])
        .creation_flags(0x0800_0000)
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).split(',').next().unwrap_or("").trim_matches(|c: char| c == '"' || c.is_whitespace()).to_string())
        .unwrap_or_default()
}

#[cfg(not(windows))]
fn image_name(_pid: u32) -> String {
    String::new()
}

/// Prima che questo Aurora apra la porta, nessun altro Aurora deve tenerla: ne' in ascolto ne' con connessioni gia' aperte con la finestra
/// (il 2026-11-09 la finestra della 0.11.205 riusava le connessioni ancora aperte con il core 0.11.155, che non era piu' in ascolto,
/// e mostrava la versione vecchia). Si chiudono i processi `aurora` e `aurora-desktop` che tengono la porta dal lato server.
pub fn evict(port: u16) {
    evict_answering(port);
    let leftovers: Vec<u32> = listeners(port).into_iter().filter(|pid| {
        // il core da solo (`aurora.exe`, 2026-11-09: era lui il 0.11.155 rimasto acceso) e il programma Windows
        let name = image_name(*pid);
        name.eq_ignore_ascii_case("aurora.exe") || name.eq_ignore_ascii_case("aurora-desktop.exe")
    }).collect();
    if !leftovers.is_empty() {
        tracing::warn!("sulla porta {port} un Aurora rimasto acceso (processi {leftovers:?}) teneva ancora connessioni aperte: lo chiudo");
        leftovers.into_iter().for_each(kill);
        std::thread::sleep(Duration::from_millis(800));
    }
}

/// Chiude l'Aurora di un'altra versione che risponde sulla porta; al massimo tre tentativi.
fn evict_answering(port: u16) {
    let mine = env!("CARGO_PKG_VERSION");
    for _ in 0..3 {
        let Some(other) = answering_version(port) else { return };
        if other == mine {
            // la stessa versione (un'altra copia di questo programma): ci pensa il controllo dell'istanza unica
            return;
        }
        let pids = listeners(port);
        tracing::warn!("sulla porta {port} risponde ancora Aurora {other} (processi {pids:?}): lo chiudo per avviare la {mine}");
        if pids.is_empty() {
            return;
        }
        pids.into_iter().for_each(kill);
        std::thread::sleep(Duration::from_millis(800));
    }
}

