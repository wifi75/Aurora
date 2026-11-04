//! Il pacchetto per l'assistenza: uno zip sul Desktop con tutto quello che serve a capire un problema, da mandare a chi aiuta.
//! Dentro: `aurora.log` (le ultime parti) e i log precedenti, la cartella `crash`, i file di configurazione della cartella dati con
//! password, chiavi, token e codici sostituiti da «***», le ultime righe di `ALL.TXT` e `info.txt` con il riassunto della stazione.
//! Fuori: i database (log dei QSO, dispositivi, storico), di cui `info.txt` dice solo nome e dimensione.

use std::{
    fmt::Write as _,
    io::{Read as _, Seek as _, SeekFrom, Write as _},
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

use remote_api::{is_secret_name, redact_json, SupportRequest};

/// Limite di tutto il pacchetto (dati non compressi).
const MAX_TOTAL: u64 = 50 * 1024 * 1024;
/// Di `aurora.log` si tengono le ultime parti fino a qui.
const MAX_LOG: u64 = 10 * 1024 * 1024;
/// Un file di configurazione piu' grande di cosi' non e' una configurazione.
const MAX_CONFIG: u64 = 2 * 1024 * 1024;
/// Le righe di `ALL.TXT` che si tengono.
const ALL_TXT_LINES: usize = 2000;
/// Estensioni dei file di configurazione.
const CONFIG_EXT: [&str; 7] = ["json", "env", "ini", "txt", "toml", "cfg", "conf"];
/// Estensioni dei database e dei dati personali grandi: non si mettono, se ne dice solo la dimensione.
const DATA_EXT: [&str; 7] = ["db", "sqlite", "sqlite3", "adi", "adif", "wal", "shm"];
/// Cartelle della cartella dati che non sono configurazione.
const SKIP_DIRS: [&str; 5] = ["logs", "crash", "recordings", "save", "cache"];

/// Quello che il pacchetto sa della stazione oltre allo stato del momento.
pub struct Context {
    pub data_dir: PathBuf,
    pub engine: String,
    /// Il PTT davvero in uso (lo stesso calcolo della trasmissione).
    pub ptt_method: String,
    pub ptt_port: Option<String>,
    pub version: String,
}

/// Crea lo zip e ritorna il suo percorso.
pub fn build(ctx: &Context, req: &SupportRequest) -> Result<remote_api::SupportMade, String> {
    let now = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs() as i64).unwrap_or(0);
    let local = civil(now + i64::from(req.local_offset_min) * 60);
    let folder = desktop_dir().unwrap_or_else(|| ctx.data_dir.clone());
    let stem = format!("Aurora-assistenza-{:04}{:02}{:02}-{:02}{:02}", local.0, local.1, local.2, local.3, local.4);
    let mut path = folder.join(format!("{stem}.zip"));
    let mut n = 2;
    while path.exists() {
        path = folder.join(format!("{stem}-{n}.zip"));
        n += 1;
    }
    let file = std::fs::File::create(&path).map_err(|e| format!("{}: {e}", path.display()))?;
    let mut pack = Pack { zip: zip::ZipWriter::new(file), used: 0, notes: Vec::new(), excluded: Vec::new() };
    let result = fill(&mut pack, ctx, req, now);
    let finished = pack.zip.finish().map_err(|e| e.to_string());
    if let Err(e) = result.and(finished.map(|_| ())) {
        let _ = std::fs::remove_file(&path);
        return Err(e);
    }
    Ok(remote_api::SupportMade { path, ptt: ctx.ptt_method.clone() })
}

struct Pack {
    zip: zip::ZipWriter<std::fs::File>,
    used: u64,
    /// Cosa e' stato saltato o accorciato, per info.txt.
    notes: Vec<String>,
    /// I database lasciati fuori, con la dimensione.
    excluded: Vec<String>,
}

impl Pack {
    fn add(&mut self, name: &str, data: &[u8]) -> Result<(), String> {
        let options = zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
        self.zip.start_file(name, options).map_err(|e| e.to_string())?;
        self.zip.write_all(data).map_err(|e| e.to_string())?;
        self.used += data.len() as u64;
        Ok(())
    }

    fn room(&self) -> u64 {
        MAX_TOTAL.saturating_sub(self.used)
    }

    /// Un file intero, se ci sta; altrimenti la sua parte finale (per i testi) o niente (per i dump), annotato.
    fn add_file(&mut self, name: &str, path: &Path, tail_ok: bool, limit: u64) -> Result<(), String> {
        let Ok(meta) = std::fs::metadata(path) else { return Ok(()) };
        let size = meta.len();
        let cap = limit.min(self.room());
        if size <= cap {
            if let Ok(data) = std::fs::read(path) {
                self.add(name, &data)?;
            }
        } else if tail_ok && cap > 64 * 1024 {
            if let Ok(data) = read_tail(path, cap) {
                self.notes.push(format!("{name}: {} su {}, solo la parte finale", mb(data.len() as u64), mb(size)));
                self.add(name, &data)?;
            }
        } else {
            self.notes.push(format!("{name}: saltato ({}, oltre il limite del pacchetto)", mb(size)));
        }
        Ok(())
    }
}

fn fill(pack: &mut Pack, ctx: &Context, req: &SupportRequest, now: i64) -> Result<(), String> {
    let dir = &ctx.data_dir;
    // i log: quello in uso per primo, poi i precedenti
    let logs = dir.join("logs");
    pack.add_file("logs/aurora.log", &logs.join("aurora.log"), true, MAX_LOG)?;
    for (path, name) in newest_first(&logs) {
        if name != "aurora.log" {
            pack.add_file(&format!("logs/{name}"), &path, true, MAX_LOG)?;
        }
    }
    // la configurazione, con i segreti tolti
    add_configs(pack, dir, "")?;
    // le ultime righe di ALL.TXT
    if let Some(all) = newest_all_txt(dir, now) {
        let name = all.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
        if let Ok(data) = read_tail(&all, 4 * 1024 * 1024) {
            let text = String::from_utf8_lossy(&data);
            let lines: Vec<&str> = text.lines().collect();
            let kept = &lines[lines.len().saturating_sub(ALL_TXT_LINES)..];
            pack.add(&format!("ALL.TXT/{name}"), (kept.join("\r\n") + "\r\n").as_bytes())?;
        }
    }
    // i guasti: prima i testi (piccoli e piu' utili), poi i dump dal piu' recente finche' c'e' spazio
    let crash = dir.join("crash");
    let files = newest_first(&crash);
    for (path, name) in files.iter().filter(|(_, n)| !n.ends_with(".dmp")) {
        pack.add_file(&format!("crash/{name}"), path, true, MAX_LOG)?;
    }
    // un po' di spazio resta per info.txt
    for (path, name) in files.iter().filter(|(_, n)| n.ends_with(".dmp")) {
        let size = std::fs::metadata(path).map(|m| m.len()).unwrap_or(0);
        if size + 1024 * 1024 > pack.room() {
            pack.notes.push(format!("crash/{name}: saltato ({}, oltre il limite del pacchetto)", mb(size)));
            continue;
        }
        pack.add_file(&format!("crash/{name}"), path, false, MAX_TOTAL)?;
    }
    let info = info_text(pack, ctx, req, now);
    pack.add("info.txt", info.as_bytes())
}

/// I file di configurazione della cartella dati (e delle sottocartelle che non sono log, crash o registrazioni), con i segreti sostituiti.
fn add_configs(pack: &mut Pack, dir: &Path, prefix: &str) -> Result<(), String> {
    let Ok(entries) = std::fs::read_dir(dir) else { return Ok(()) };
    let mut entries: Vec<_> = entries.flatten().collect();
    entries.sort_by_key(|e| e.file_name());
    for entry in entries {
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().into_owned();
        let Ok(meta) = entry.metadata() else { continue };
        if meta.is_dir() {
            if prefix.is_empty() && !SKIP_DIRS.contains(&name.to_ascii_lowercase().as_str()) {
                add_configs(pack, &path, &format!("config/{name}/"))?;
            }
            continue;
        }
        let ext = path.extension().map(|e| e.to_string_lossy().to_ascii_lowercase()).unwrap_or_default();
        let shown = if prefix.is_empty() { format!("config/{name}") } else { format!("{prefix}{name}") };
        if DATA_EXT.contains(&ext.as_str()) {
            pack.excluded.push(format!("{} ({})", shown.trim_start_matches("config/"), mb(meta.len())));
            continue;
        }
        // ALL.TXT ha la sua parte; i file grandi non sono configurazione
        if !CONFIG_EXT.contains(&ext.as_str()) || name.to_ascii_uppercase().ends_with("ALL.TXT") {
            continue;
        }
        if meta.len() > MAX_CONFIG {
            pack.notes.push(format!("{shown}: saltato ({})", mb(meta.len())));
            continue;
        }
        let Ok(text) = std::fs::read_to_string(&path) else { continue };
        let clean = if ext == "json" { redact_json_text(&text) } else { redact_lines(&text) };
        pack.add(&shown, clean.as_bytes())?;
    }
    Ok(())
}

/// Un JSON con i campi segreti a «***»; se non si legge come JSON, riga per riga.
fn redact_json_text(text: &str) -> String {
    match serde_json::from_str::<serde_json::Value>(text) {
        Ok(mut value) => {
            redact_json(&mut value);
            serde_json::to_string_pretty(&value).unwrap_or_default()
        }
        Err(_) => redact_lines(text),
    }
}

/// Righe `NOME=valore` o `nome: valore` (ambiente, ini): se il nome fa pensare a un segreto il valore diventa «***».
fn redact_lines(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for line in text.lines() {
        let at = line.find(['=', ':']);
        match at {
            Some(i) if is_secret_name(line[..i].trim()) && !line[i + 1..].trim().is_empty() && !line.trim_start().starts_with('#') => {
                out.push_str(&line[..=i]);
                out.push_str("***");
            }
            _ => out.push_str(line),
        }
        out.push('\n');
    }
    out
}

fn info_text(pack: &Pack, ctx: &Context, req: &SupportRequest, now: i64) -> String {
    let live = &req.live;
    let mut s = String::new();
    let utc = civil(now);
    let local = civil(now + i64::from(req.local_offset_min) * 60);
    let _ = writeln!(s, "Pacchetto per l'assistenza di Aurora");
    let _ = writeln!(s, "====================================\n");
    let _ = writeln!(s, "Aurora: {}", ctx.version);
    let _ = writeln!(s, "Windows: {}", windows_version());
    let _ = writeln!(s, "Lingua dell'interfaccia: {}", if req.lang.is_empty() { "—" } else { &req.lang });
    let _ = writeln!(s, "Lingua di Windows: {}", reg_value(r"HKCU\Control Panel\International", "LocaleName").unwrap_or_else(|| "—".into()));
    let _ = writeln!(s, "Ora locale: {} (UTC{:+}:{:02})", stamp(local), req.local_offset_min / 60, (req.local_offset_min % 60).abs());
    let _ = writeln!(s, "Ora UTC: {}", stamp(utc));
    let clock = &live["clock"]["last"];
    let _ = writeln!(s, "Orologio (ultimo controllo): {}", if clock.is_null() { "non controllato".to_string() } else { clock.to_string() });
    let _ = writeln!(s, "Cartella dati: {}", ctx.data_dir.display());
    let _ = writeln!(s, "\nMotore: {}", ctx.engine);
    let _ = writeln!(s, "PTT in uso: {}{}", ctx.ptt_method, ctx.ptt_port.as_deref().map(|p| format!(" su {p}")).unwrap_or_default());
    let _ = writeln!(s, "Radio (collegamento): {}", live["cat"]);
    let _ = writeln!(s, "Controlli della radio: {}", live["controls"]);
    let status = &live["status"];
    let _ = writeln!(s, "Abilita Tx: {}  In trasmissione: {}  Frequenza: {} Hz  Modo: {}", status["tx_enabled"], status["transmitting"], status["dial_hz"], status["mode"]);
    let _ = writeln!(s, "Trasmissione aperta (politica): {}", live["tx_allowed_policy"]);
    let _ = writeln!(s, "\nAudio (scelti e disponibili): {}", live["audio"]);
    match rig_control::detect::read_system() {
        Ok((ports, sound)) => {
            let _ = writeln!(s, "Porte seriali: {}", if ports.is_empty() { "nessuna".to_string() } else { ports.iter().map(|p| format!("{} ({})", p.port, p.name)).collect::<Vec<_>>().join(", ") });
            let _ = writeln!(s, "Schede audio di Windows: {}", sound.join(", "));
        }
        Err(e) => {
            let _ = writeln!(s, "Porte seriali: non lette ({e})");
        }
    }
    let station = &live["station"];
    let _ = writeln!(s, "\nUDP in ascolto: {}  inoltro: {}  ADIF via UDP: {}", station["udp_listen"], station["udp_forward"], station["export_udp"]);
    let _ = writeln!(s, "Rete di casa: porta {}  mDNS (aurora.local): {}", live["lan"]["port"], live["lan"]["mdns_host"]);
    let _ = writeln!(s, "Dispositivi associati: {}  Accesso senza codice: {}  PC fidato: {}", live["paired_devices"], live["open_access"], live["local_trust"]);
    let _ = writeln!(s, "DX Cluster: {}", live["cluster"]["servers"]);
    let _ = writeln!(s, "\nLasciati fuori (dati personali, solo la dimensione): {}", if pack.excluded.is_empty() { "nessuno".to_string() } else { pack.excluded.join(", ") });
    if !pack.notes.is_empty() {
        let _ = writeln!(s, "\nAccorciati o saltati:");
        for note in &pack.notes {
            let _ = writeln!(s, "  - {note}");
        }
    }
    let _ = writeln!(s, "\nPassword, chiavi, token e codici sono sostituiti da ***.");
    let _ = writeln!(s, "\nStato completo\n--------------\n{}", serde_json::to_string_pretty(live).unwrap_or_default());
    s
}

/// I file di una cartella, dal piu' recente.
pub(crate) fn newest_first(dir: &Path) -> Vec<(PathBuf, String)> {
    let Ok(entries) = std::fs::read_dir(dir) else { return Vec::new() };
    let mut files: Vec<(SystemTime, PathBuf, String)> = entries
        .flatten()
        .filter(|e| e.file_type().is_ok_and(|t| t.is_file()))
        .map(|e| (e.metadata().and_then(|m| m.modified()).unwrap_or(UNIX_EPOCH), e.path(), e.file_name().to_string_lossy().into_owned()))
        .collect();
    files.sort_by(|a, b| b.0.cmp(&a.0));
    files.into_iter().map(|(_, p, n)| (p, n)).collect()
}

/// L'`ALL.TXT` del mese corrente se c'e', altrimenti il piu' recente (cartella dati e un livello sotto).
fn newest_all_txt(dir: &Path, now: i64) -> Option<PathBuf> {
    let (y, m, ..) = civil(now);
    let month = format!("{y:04}{m:02}_ALL.TXT");
    let mut found: Vec<(SystemTime, PathBuf)> = Vec::new();
    let mut look = |d: &Path| {
        for (path, name) in newest_first(d) {
            if name.to_ascii_uppercase().ends_with("ALL.TXT") {
                found.push((std::fs::metadata(&path).and_then(|m| m.modified()).unwrap_or(UNIX_EPOCH), path));
            }
        }
    };
    look(dir);
    if let Ok(entries) = std::fs::read_dir(dir) {
        for e in entries.flatten().filter(|e| e.file_type().is_ok_and(|t| t.is_dir())) {
            look(&e.path());
        }
    }
    if let Some((_, p)) = found.iter().find(|(_, p)| p.file_name().is_some_and(|n| n.to_string_lossy().eq_ignore_ascii_case(&month))) {
        return Some(p.clone());
    }
    found.sort_by(|a, b| b.0.cmp(&a.0));
    found.into_iter().next().map(|(_, p)| p)
}

/// Gli ultimi `max` byte di un file.
pub(crate) fn read_tail(path: &Path, max: u64) -> std::io::Result<Vec<u8>> {
    let mut file = std::fs::File::open(path)?;
    let size = file.metadata()?.len();
    if size > max {
        file.seek(SeekFrom::Start(size - max))?;
    }
    let mut data = Vec::with_capacity(size.min(max) as usize);
    file.read_to_end(&mut data)?;
    Ok(data)
}

fn mb(bytes: u64) -> String {
    format!("{:.1} MB", bytes as f64 / 1_048_576.0)
}

/// Anno, mese, giorno, ora, minuto, secondo dai secondi Unix (calendario gregoriano).
pub(crate) fn civil(secs: i64) -> (i64, u32, u32, u32, u32, u32) {
    let days = secs.div_euclid(86_400);
    let rest = secs.rem_euclid(86_400);
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let month = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    let year = yoe + era * 400 + i64::from(month <= 2);
    (year, month, day, (rest / 3600) as u32, (rest / 60 % 60) as u32, (rest % 60) as u32)
}

fn stamp(t: (i64, u32, u32, u32, u32, u32)) -> String {
    format!("{:04}-{:02}-{:02} {:02}:{:02}:{:02}", t.0, t.1, t.2, t.3, t.4, t.5)
}

/// Un valore del registro di Windows letto con `reg query` (solo lettura, senza finestra).
fn reg_value(key: &str, name: &str) -> Option<String> {
    let mut command = std::process::Command::new("reg");
    command.args(["query", key, "/v", name]);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt as _;
        command.creation_flags(0x0800_0000);
    }
    let out = command.output().ok()?;
    let text = String::from_utf8_lossy(&out.stdout);
    text.lines().find_map(|line| {
        let mut parts = line.trim().splitn(3, "    ");
        let (n, _kind, value) = (parts.next()?, parts.next()?, parts.next()?);
        n.eq_ignore_ascii_case(name).then(|| value.trim().to_string())
    })
}

/// «Windows 11 Pro 24H2 (build 26100.1234)».
pub(crate) fn windows_version() -> String {
    const KEY: &str = r"HKLM\SOFTWARE\Microsoft\Windows NT\CurrentVersion";
    let product = reg_value(KEY, "ProductName").unwrap_or_else(|| "Windows".into());
    let release = reg_value(KEY, "DisplayVersion").unwrap_or_default();
    let build = reg_value(KEY, "CurrentBuild").unwrap_or_default();
    // UBR e' un REG_DWORD: «0x4d2»
    let ubr = reg_value(KEY, "UBR").and_then(|v| u64::from_str_radix(v.trim_start_matches("0x"), 16).ok());
    // dalla build 22000 e' Windows 11, anche se ProductName dice ancora 10
    let product = if build.parse::<u32>().is_ok_and(|b| b >= 22_000) { product.replace("Windows 10", "Windows 11") } else { product };
    format!("{product} {release} (build {build}{})", ubr.map(|u| format!(".{u}")).unwrap_or_default())
}

/// Il Desktop vero dell'utente (anche se spostato in OneDrive); `None` se non c'e'.
pub(crate) fn desktop_dir() -> Option<PathBuf> {
    let from_shell = reg_value(r"HKCU\Software\Microsoft\Windows\CurrentVersion\Explorer\Shell Folders", "Desktop").map(PathBuf::from);
    let fallback = std::env::var_os("USERPROFILE").map(|home| PathBuf::from(home).join("Desktop"));
    from_shell.into_iter().chain(fallback).find(|p| p.is_dir())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn civil_date() {
        assert_eq!(civil(0), (1970, 1, 1, 0, 0, 0));
        assert_eq!(civil(1_793_664_000), (2026, 11, 3, 0, 0, 0));
    }

    #[test]
    fn secret_lines_are_hidden() {
        let text = "AURORA_ENGINE=native\nAURORA_QRZ_PASSWORD=segreta\n# AURORA_API_KEY=\nLOTW_TOKEN: abc\n";
        let out = redact_lines(text);
        assert!(out.contains("AURORA_ENGINE=native"));
        assert!(out.contains("AURORA_QRZ_PASSWORD=***"));
        assert!(out.contains("LOTW_TOKEN:***"));
        assert!(!out.contains("segreta"));
    }
}
