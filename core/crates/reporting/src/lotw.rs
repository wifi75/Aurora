//! Invio a LoTW (Logbook of The World) tramite TQSL installato, dalla riga di comando documentata dall'ARRL («TQSL command line»).
//!
//! Aurora scrive i QSO in attesa in un file ADIF temporaneo e lancia `tqsl -d -u -a compliant -x -l <posizione> <file.adi>`:
//! `-d` niente finestra delle date, `-u` firma e carica, `-a compliant` salta doppioni e QSO fuori dalle date del certificato, `-x` senza
//! finestre (esce alla fine), `-l` la posizione di stazione creata in TQSL. Il certificato e la sua eventuale password restano in TQSL: Aurora
//! non li tocca. Codici d'uscita documentati: 0 riuscito, 1 annullato, 2 rifiutato da LoTW, 3 risposta inattesa del server, 4 errore di TQSL,
//! 5 errore di TQSLlib, 6 file d'ingresso non apribile, 7 file d'uscita non apribile, 8 nessun QSO (tutti doppioni o fuori date), 9 alcuni QSO
//! saltati, 10 riga di comando sbagliata, 11 LoTW non raggiungibile.

use std::{
    ffi::OsString,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    time::{Duration, Instant},
};

use crate::upload::{Outcome, short};

/// Attesa massima di TQSL (firma e caricamento).
pub const TIMEOUT: Duration = Duration::from_secs(180);

/// Dove TQSL si installa di solito.
pub fn default_candidates() -> Vec<PathBuf> {
    let mut out = Vec::new();
    if cfg!(windows) {
        for var in ["ProgramFiles(x86)", "ProgramFiles", "ProgramW6432"] {
            if let Some(dir) = std::env::var_os(var) {
                out.push(PathBuf::from(dir).join("TrustedQSL").join("tqsl.exe"));
            }
        }
        if let Some(dir) = std::env::var_os("LOCALAPPDATA") {
            out.push(PathBuf::from(dir).join("Programs").join("TrustedQSL").join("tqsl.exe"));
        }
        out.push(PathBuf::from(r"C:\Program Files (x86)\TrustedQSL\tqsl.exe"));
        out.push(PathBuf::from(r"C:\Program Files\TrustedQSL\tqsl.exe"));
    } else {
        out.push(PathBuf::from("/usr/bin/tqsl"));
        out.push(PathBuf::from("/usr/local/bin/tqsl"));
        out.push(PathBuf::from("/Applications/TrustedQSL/tqsl.app/Contents/MacOS/tqsl"));
    }
    out.dedup();
    out
}

/// Il file delle posizioni di stazione di TQSL (`station_data`): su Windows in `%APPDATA%\TrustedQSL`, altrove in `~/.tqsl`.
pub fn default_station_data() -> Option<PathBuf> {
    if cfg!(windows) {
        std::env::var_os("APPDATA").map(|d| PathBuf::from(d).join("TrustedQSL").join("station_data"))
    } else {
        std::env::var_os("HOME").map(|d| PathBuf::from(d).join(".tqsl").join("station_data"))
    }
}

/// Il programma TQSL: quello indicato (se esiste) o il primo trovato tra le cartelle tipiche.
pub fn find_tqsl(configured: &str, candidates: &[PathBuf]) -> Option<PathBuf> {
    let configured = configured.trim().trim_matches('"');
    if !configured.is_empty() {
        let path = PathBuf::from(configured);
        return path.is_file().then_some(path);
    }
    candidates.iter().find(|p| p.is_file()).cloned()
}

fn unescape(s: &str) -> String {
    s.replace("&quot;", "\"").replace("&apos;", "'").replace("&lt;", "<").replace("&gt;", ">").replace("&amp;", "&")
}

/// I nomi delle posizioni di stazione nel file `station_data` (`<StationData name="...">`).
pub fn locations(xml: &str) -> Vec<String> {
    xml.split("<StationData").skip(1).filter_map(|chunk| {
        let rest = chunk.split_once("name=\"")?.1;
        let name = rest.split_once('"')?.0;
        Some(unescape(name))
    }).collect()
}

/// Gli argomenti di TQSL.
pub fn args(location: &str, file: &Path) -> Vec<OsString> {
    let mut a: Vec<OsString> = ["-d", "-u", "-a", "compliant", "-x", "-l"].iter().map(OsString::from).collect();
    a.push(OsString::from(location));
    a.push(file.as_os_str().to_os_string());
    a
}

/// L'esito dal codice d'uscita (`None` = TQSL interrotto) e dall'ultima riga dei messaggi di TQSL.
pub fn outcome(code: Option<i32>, detail: &str) -> Outcome {
    let detail = short(detail, 120);
    let with = |base: &str| if detail.is_empty() { base.to_string() } else { format!("{base}: {detail}") };
    match code {
        Some(0) => Outcome::Sent(String::new()),
        Some(9) => Outcome::Sent("alcuni QSO saltati: doppioni o fuori dalle date del certificato".into()),
        Some(8) => Outcome::Duplicate("gia' caricati o fuori dalle date del certificato".into()),
        Some(11) => Outcome::Retry(with("LoTW non raggiungibile")),
        Some(3) => Outcome::Retry(with("risposta inattesa del server di LoTW")),
        Some(7) => Outcome::Retry(with("TQSL non ha potuto scrivere il file firmato")),
        Some(2) => Outcome::Rejected(with("LoTW ha rifiutato il file")),
        Some(6) => Outcome::Rejected(with("TQSL non ha potuto aprire il file dei QSO")),
        Some(1) => Outcome::Blocked(with("TQSL ha annullato (serve la password del certificato?)")),
        Some(4) | Some(5) => Outcome::Blocked(with("errore di TQSL (certificato o posizione di stazione)")),
        Some(10) => Outcome::Blocked(with("TQSL non ha accettato la riga di comando")),
        Some(c) => Outcome::Retry(with(&format!("TQSL e' uscito con il codice {c}"))),
        None => Outcome::Retry("TQSL non ha finito in tempo ed e' stato fermato".into()),
    }
}

/// Firma e carica i record (bloccante): un file ADIF temporaneo nella cartella `dir`, cancellato alla fine.
pub fn run(tqsl: &Path, location: &str, records: &[String], dir: &Path, id: u64, timeout: Duration) -> Outcome {
    let file = dir.join(format!("lotw-{}-{id}.adi", std::process::id()));
    let log = dir.join(format!("lotw-{}-{id}.log", std::process::id()));
    let mut text = logbook::adif::header();
    for r in records {
        text.push_str(r.trim());
        text.push('\n');
    }
    if let Err(e) = std::fs::create_dir_all(dir).and_then(|()| std::fs::write(&file, text)) {
        return Outcome::Retry(format!("file per TQSL non scritto: {e}"));
    }
    let result = launch(tqsl, location, &file, &log, timeout);
    let detail = std::fs::read_to_string(&log).ok().and_then(|t| t.lines().rev().find(|l| !l.trim().is_empty()).map(str::to_string)).unwrap_or_default();
    let _ = std::fs::remove_file(&file);
    let _ = std::fs::remove_file(&log);
    match result {
        Ok(code) => outcome(code, &detail),
        Err(e) => Outcome::Blocked(format!("TQSL non avviato: {e}")),
    }
}

fn launch(tqsl: &Path, location: &str, file: &Path, log: &Path, timeout: Duration) -> std::io::Result<Option<i32>> {
    let out = std::fs::File::create(log)?;
    let err = out.try_clone()?;
    let mut command = Command::new(tqsl);
    command.args(args(location, file)).stdin(Stdio::null()).stdout(Stdio::from(out)).stderr(Stdio::from(err));
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        // CREATE_NO_WINDOW: nessuna finestra di console
        command.creation_flags(0x0800_0000);
    }
    let mut child = command.spawn()?;
    let start = Instant::now();
    loop {
        if let Some(status) = child.try_wait()? {
            return Ok(status.code());
        }
        if start.elapsed() > timeout {
            let _ = child.kill();
            let _ = child.wait();
            return Ok(None);
        }
        std::thread::sleep(Duration::from_millis(200));
    }
}

/// La prova senza caricare niente: TQSL c'e' e la posizione di stazione esiste.
pub fn check(configured: &str, location: &str, candidates: &[PathBuf], station_data: Option<&Path>) -> Result<String, String> {
    let tqsl = find_tqsl(configured, candidates).ok_or_else(|| {
        if configured.trim().is_empty() { "TQSL non trovato nelle cartelle tipiche: indica il percorso di tqsl.exe".to_string() } else { "il percorso di TQSL indicato non esiste".to_string() }
    })?;
    let found = format!("TQSL trovato ({})", tqsl.display());
    let Some(text) = station_data.and_then(|p| std::fs::read_to_string(p).ok()) else {
        return Ok(format!("{found}; elenco delle posizioni di stazione non leggibile: controlla in TQSL che «{location}» esista"));
    };
    let names = locations(&text);
    if names.iter().any(|n| n == location.trim()) {
        Ok(format!("{found}; posizione di stazione «{}» presente", location.trim()))
    } else {
        Err(format!("{found}, ma la posizione di stazione «{}» non c'e' (in TQSL: {})", location.trim(), names.join(", ")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_command_line_is_the_documented_one() {
        let a = args("Casa", Path::new("x.adi"));
        assert_eq!(a, ["-d", "-u", "-a", "compliant", "-x", "-l", "Casa", "x.adi"].map(OsString::from).to_vec());
    }

    #[test]
    fn the_exit_codes_become_outcomes() {
        assert_eq!(outcome(Some(0), ""), Outcome::Sent(String::new()));
        assert!(matches!(outcome(Some(8), ""), Outcome::Duplicate(_)));
        assert!(matches!(outcome(Some(9), ""), Outcome::Sent(_)));
        assert!(matches!(outcome(Some(11), "no network"), Outcome::Retry(m) if m.contains("no network")));
        assert!(matches!(outcome(Some(2), ""), Outcome::Rejected(_)));
        assert!(matches!(outcome(Some(4), ""), Outcome::Blocked(_)));
        assert!(matches!(outcome(None, ""), Outcome::Retry(_)));
    }

    #[test]
    fn locations_are_read_from_station_data() {
        let xml = "<StationDataFile>\n  <StationData name=\"Casa\">\n    <CALL>K1ABC</CALL>\n  </StationData>\n  <StationData name=\"Portatile &amp; bici\">\n  </StationData>\n</StationDataFile>";
        assert_eq!(locations(xml), vec!["Casa".to_string(), "Portatile & bici".to_string()]);
    }

    #[test]
    fn tqsl_is_the_configured_file_or_the_first_candidate_found() {
        let dir = std::env::temp_dir().join(format!("aurora-tqsl-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let fake = dir.join("tqsl.exe");
        std::fs::write(&fake, b"x").unwrap();
        let missing = dir.join("manca.exe");
        assert_eq!(find_tqsl("", &[missing.clone(), fake.clone()]), Some(fake.clone()));
        assert_eq!(find_tqsl(&format!("\"{}\"", fake.display()), &[]), Some(fake.clone()));
        assert_eq!(find_tqsl(&missing.display().to_string(), std::slice::from_ref(&fake)), None, "un percorso indicato e sbagliato non si sostituisce");
        let data = dir.join("station_data");
        std::fs::write(&data, "<StationDataFile><StationData name=\"Casa\"></StationData></StationDataFile>").unwrap();
        assert!(check("", "Casa", std::slice::from_ref(&fake), Some(&data)).unwrap().contains("presente"));
        assert!(check("", "Mare", std::slice::from_ref(&fake), Some(&data)).unwrap_err().contains("Casa"));
        assert!(check("", "Casa", std::slice::from_ref(&fake), None).unwrap().contains("non leggibile"));
        assert!(check("", "Casa", &[missing], Some(&data)).is_err());
        std::fs::remove_dir_all(&dir).ok();
    }
}
