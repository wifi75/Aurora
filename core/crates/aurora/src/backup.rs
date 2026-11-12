//! Copia di sicurezza delle impostazioni: uno zip sul Desktop con i file di configurazione della cartella dati e il log dei QSO,
//! e il ripristino da uno zip fatto cosi'. Le password e le chiavi dei servizi (`*.secret` e simili) restano fuori: si riscrivono.
//!
//! Il ripristino non tocca i file in uso: mette quelli dello zip nella cartella `ripristino` dei dati e Aurora li sposta al loro posto
//! al prossimo avvio, prima di aprire log e impostazioni (`apply_pending`).

use std::{
    io::{Read as _, Write as _},
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

use logbook::Logbook;
use remote_api::is_secret_name;

/// La cartella dei file da rimettere al posto al prossimo avvio.
pub const PENDING_DIR: &str = "ripristino";
/// Il nome del log dentro lo zip (una copia coerente del database, non il file mentre e' aperto).
const LOGBOOK: &str = "logbook.db";
/// Un file di impostazioni piu' grande di cosi' non e' una impostazione.
const MAX_FILE: u64 = 64 * 1024 * 1024;
/// File della cartella dati che cambiano da soli e non sono scelte di chi usa Aurora.
const SKIP: [&str; 6] = ["invii-in-coda.json", "rapporti-crash-visto.json", "update.notified", "clock.good", "desktop-primo-avvio", "jtdx-peer.txt"];
/// Il segno che lo zip e' una copia delle impostazioni di Aurora.
const MARK: &str = "aurora-impostazioni.txt";

/// Puo' stare nella copia (e tornare con il ripristino)?
fn wanted(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    let ext_ok = [".json", ".env", ".adi"].iter().any(|e| lower.ends_with(e)) || lower == LOGBOOK;
    ext_ok && !lower.starts_with("rapporti-") && !SKIP.contains(&lower.as_str()) && !is_secret_name(&lower) && !lower.contains("secret") && !name.contains(['/', '\\']) && !name.starts_with('.')
}

/// Crea lo zip in `dest` (di solito il Desktop) e ritorna il suo percorso e quanti file contiene.
pub fn make(data_dir: &Path, logbook: &Logbook, dest: &Path, version: &str) -> Result<(PathBuf, usize), String> {
    let now = SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |d| d.as_secs() as i64);
    let (y, mo, d, h, mi, _) = crate::support::civil(now);
    let path = dest.join(format!("Aurora-impostazioni-{y:04}{mo:02}{d:02}-{h:02}{mi:02}.zip"));
    let file = std::fs::File::create(&path).map_err(|e| format!("{}: {e}", path.display()))?;
    let mut zip = zip::ZipWriter::new(file);
    let options = zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
    let mut count = 0;
    let add = |zip: &mut zip::ZipWriter<std::fs::File>, name: &str, data: &[u8]| -> Result<(), String> {
        zip.start_file(name, options).map_err(|e| e.to_string())?;
        zip.write_all(data).map_err(|e| e.to_string())
    };
    add(&mut zip, MARK, format!("Copia delle impostazioni di Aurora {version}\nSi ripristina da Impostazioni › Accesso e assistenza › Assistenza.\nPassword e chiavi dei servizi non sono incluse.\n").as_bytes())?;
    let mut names: Vec<String> = std::fs::read_dir(data_dir)
        .map_err(|e| format!("{}: {e}", data_dir.display()))?
        .flatten()
        .filter(|e| e.file_type().is_ok_and(|t| t.is_file()))
        .filter_map(|e| e.file_name().to_str().map(str::to_string))
        .filter(|n| wanted(n) && n != LOGBOOK)
        .collect();
    names.sort();
    for name in names {
        let p = data_dir.join(&name);
        if std::fs::metadata(&p).is_ok_and(|m| m.len() <= MAX_FILE)
            && let Ok(data) = std::fs::read(&p)
        {
            add(&mut zip, &name, &data)?;
            count += 1;
        }
    }
    // il log si copia dal database aperto con una sua copia coerente
    let tmp = data_dir.join(format!("{LOGBOOK}.copia"));
    let _ = std::fs::remove_file(&tmp);
    logbook.copy_to(&tmp).map_err(|e| format!("log dei QSO non copiato: {e}"))?;
    let data = std::fs::read(&tmp).map_err(|e| e.to_string());
    let _ = std::fs::remove_file(&tmp);
    add(&mut zip, LOGBOOK, &data?)?;
    count += 1;
    zip.finish().map_err(|e| e.to_string())?;
    Ok((path, count))
}

/// Controlla lo zip e mette i suoi file nella cartella `ripristino`: si applicano al prossimo avvio. Ritorna quanti sono.
pub fn stage(data_dir: &Path, bytes: &[u8]) -> Result<usize, String> {
    let mut zip = zip::ZipArchive::new(std::io::Cursor::new(bytes)).map_err(|_| "not_a_backup".to_string())?;
    if zip.by_name(MARK).is_err() {
        return Err("not_a_backup".into());
    }
    let pending = data_dir.join(PENDING_DIR);
    let _ = std::fs::remove_dir_all(&pending);
    std::fs::create_dir_all(&pending).map_err(|e| e.to_string())?;
    let mut count = 0;
    for i in 0..zip.len() {
        let mut entry = zip.by_index(i).map_err(|e| e.to_string())?;
        let Ok(name) = entry.name().map(|n| n.to_string()) else { continue };
        if !entry.is_file() || !wanted(&name) || entry.size() > MAX_FILE {
            continue;
        }
        let mut data = Vec::new();
        entry.by_ref().take(MAX_FILE).read_to_end(&mut data).map_err(|e| e.to_string())?;
        std::fs::write(pending.join(&name), data).map_err(|e| e.to_string())?;
        count += 1;
    }
    if count == 0 {
        let _ = std::fs::remove_dir_all(&pending);
        return Err("not_a_backup".into());
    }
    Ok(count)
}

/// All'avvio, prima di aprire log e impostazioni: i file di un ripristino prendono il posto di quelli di prima.
pub fn apply_pending(data_dir: &Path) {
    let pending = data_dir.join(PENDING_DIR);
    let Ok(entries) = std::fs::read_dir(&pending) else { return };
    let mut moved = 0;
    for entry in entries.flatten() {
        let Some(name) = entry.file_name().to_str().map(str::to_string) else { continue };
        if !wanted(&name) {
            continue;
        }
        let target = data_dir.join(&name);
        let _ = std::fs::remove_file(&target);
        match std::fs::rename(entry.path(), &target) {
            Ok(()) => moved += 1,
            Err(e) => tracing::warn!("ripristino di {name} non riuscito: {e}"),
        }
    }
    let _ = std::fs::remove_dir_all(&pending);
    tracing::info!("impostazioni ripristinate dalla copia: {moved} file");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn secrets_and_noise_stay_out() {
        assert!(wanted("settings-motore.json"));
        assert!(wanted("logbook.db"));
        assert!(wanted("aurora.env"));
        assert!(!wanted("qrz.secret"));
        assert!(!wanted("devices.db"));
        assert!(!wanted("history.sqlite"));
        assert!(!wanted("invii-in-coda.json"));
        assert!(!wanted("../radio.json"));
        assert!(!wanted("202611_ALL.TXT"));
    }
}
