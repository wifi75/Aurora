//! Ripristino di Hamlib su Windows: se `rigctld.exe` sparisce (di solito lo toglie l'antivirus) si riscarica il pacchetto ufficiale di Hamlib
//! dalla pagina delle release, se ne controlla l'impronta SHA-256 (la stessa fissata in `scripts/prepara-hamlib.ps1`) e si copiano
//! `rigctl*.exe`, `rigctld*.exe` e le librerie nella cartella dei dati dell'utente (`%LOCALAPPDATA%\Aurora\hamlib`), che `locate_rigctld`
//! guarda subito dopo la cartella del programma. Non serve essere amministratori e l'installazione non viene toccata.

use std::{
    io::{Cursor, Read},
    path::{Path, PathBuf},
    time::Duration,
};

use sha2::{Digest, Sha256};

/// Hamlib ufficiale: ultima stabile al 2026-11-01. Se si cambia versione si cambia anche in `scripts/prepara-hamlib.ps1`.
pub const HAMLIB_VERSION: &str = "4.7.2";
const HAMLIB_URL: &str = "https://github.com/Hamlib/Hamlib/releases/download/4.7.2/hamlib-w64-4.7.2.zip";
const HAMLIB_SHA256: &str = "8553bc6c5c6032e8debf99c017e98f58fed7e07e7c25d04815dc3e8bbe3304c7";
/// Il pacchetto pesa una decina di MB: un limite largo contro risposte impazzite.
const MAX_BYTES: u64 = 120 * 1024 * 1024;

/// Dove si mette l'Hamlib ripristinato: `%LOCALAPPDATA%\Aurora\hamlib` su Windows; altrove niente (li si installa dal gestore dei pacchetti).
pub fn restored_dir() -> Option<PathBuf> {
    if !cfg!(windows) {
        return None;
    }
    let base = std::env::var_os("LOCALAPPDATA").filter(|v| !v.is_empty())?;
    Some(PathBuf::from(base).join("Aurora").join("hamlib"))
}

/// Il ripristino e' possibile su questo sistema?
pub fn available() -> bool {
    restored_dir().is_some()
}

/// Scarica, controlla e copia Hamlib. Ritorna la cartella dove sta ora. Bloccante.
pub fn restore() -> Result<PathBuf, String> {
    let dest = restored_dir().ok_or_else(|| "il ripristino di Hamlib e' solo per Windows: su questo sistema installa Hamlib dal gestore dei pacchetti".to_string())?;
    let bytes = download(HAMLIB_URL)?;
    install_from_zip(&bytes, HAMLIB_SHA256, &dest)?;
    Ok(dest)
}

fn download(url: &str) -> Result<Vec<u8>, String> {
    let agent: ureq::Agent = ureq::Agent::config_builder().timeout_global(Some(Duration::from_secs(300))).http_status_as_error(false).build().into();
    let mut response = agent.get(url).header("User-Agent", "Aurora-hamlib").call().map_err(|e| format!("Hamlib non scaricato: {e}"))?;
    let status = response.status().as_u16();
    if status != 200 {
        return Err(format!("Hamlib non scaricato: la pagina ha risposto {status}"));
    }
    let mut bytes = Vec::new();
    response.body_mut().as_reader().take(MAX_BYTES + 1).read_to_end(&mut bytes).map_err(|e| format!("Hamlib non scaricato: {e}"))?;
    if bytes.len() as u64 > MAX_BYTES {
        return Err("Hamlib non scaricato: il file e' troppo grande".to_string());
    }
    Ok(bytes)
}

/// Controlla l'impronta del pacchetto e ne copia i file di Hamlib in `dest`. Separata dalla rete per poterla provare.
pub fn install_from_zip(bytes: &[u8], expected_sha256: &str, dest: &Path) -> Result<(), String> {
    let digest: String = Sha256::digest(bytes).iter().map(|b| format!("{b:02x}")).collect();
    if !digest.eq_ignore_ascii_case(expected_sha256) {
        return Err("Hamlib scaricato non coincide con quello ufficiale (impronta SHA-256 diversa): non lo installo".to_string());
    }
    let mut zip = zip::ZipArchive::new(Cursor::new(bytes)).map_err(|e| format!("pacchetto di Hamlib non leggibile: {e}"))?;
    std::fs::create_dir_all(dest).map_err(|e| format!("{}: {e}", dest.display()))?;
    let mut copied_rigctld = false;
    for i in 0..zip.len() {
        let mut entry = zip.by_index(i).map_err(|e| e.to_string())?;
        if !entry.is_file() {
            continue;
        }
        // solo il nome del file, mai percorsi del pacchetto: niente uscite dalla cartella di destinazione
        let Some(name) = entry.enclosed_name().and_then(|p| p.file_name().map(|n| n.to_string_lossy().into_owned())) else {
            continue;
        };
        if !wanted_file(&entry.enclosed_name().unwrap_or_default(), &name) {
            continue;
        }
        let mut data = Vec::with_capacity(usize::try_from(entry.size()).unwrap_or(0));
        entry.read_to_end(&mut data).map_err(|e| format!("{name}: {e}"))?;
        let target = dest.join(&name);
        let tmp = dest.join(format!("{name}.tmp"));
        std::fs::write(&tmp, &data).map_err(|e| format!("{}: {e}", target.display()))?;
        std::fs::rename(&tmp, &target).map_err(|e| format!("{}: {e}", target.display()))?;
        if name.to_ascii_lowercase().starts_with("rigctld") {
            copied_rigctld = true;
        }
    }
    if copied_rigctld {
        Ok(())
    } else {
        Err("nel pacchetto di Hamlib non c'e' rigctld".to_string())
    }
}

/// I file che servono: `rigctld*.exe`, `rigctl*.exe` e le librerie (`.dll`) della cartella `bin` del pacchetto.
fn wanted_file(path: &Path, name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    let in_bin = path.parent().and_then(|p| p.file_name()).is_some_and(|d| d.eq_ignore_ascii_case("bin"));
    in_bin && (lower.ends_with(".dll") || (lower.ends_with(".exe") && lower.starts_with("rigctl")))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn pacchetto(files: &[(&str, &[u8])]) -> Vec<u8> {
        let mut zip = zip::ZipWriter::new(Cursor::new(Vec::new()));
        let options = zip::write::SimpleFileOptions::default();
        for (name, data) in files {
            zip.start_file(*name, options).expect("file");
            zip.write_all(data).expect("dati");
        }
        zip.finish().expect("fine").into_inner()
    }

    fn sha(bytes: &[u8]) -> String {
        Sha256::digest(bytes).iter().map(|b| format!("{b:02x}")).collect()
    }

    fn temp(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("aurora-hamlib-restore-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        d
    }

    #[test]
    fn it_copies_rigctld_and_libraries_only() {
        let zip = pacchetto(&[
            ("hamlib-w64-4.7.2/bin/rigctld.exe", b"d"),
            ("hamlib-w64-4.7.2/bin/rigctl.exe", b"c"),
            ("hamlib-w64-4.7.2/bin/libhamlib-4.dll", b"l"),
            ("hamlib-w64-4.7.2/bin/rigctlcom.exe", b"x"),
            ("hamlib-w64-4.7.2/bin/altro.txt", b"t"),
            ("hamlib-w64-4.7.2/doc/readme.dll", b"no"),
        ]);
        let dest = temp("ok");
        install_from_zip(&zip, &sha(&zip), &dest).expect("installato");
        assert_eq!(std::fs::read(dest.join("rigctld.exe")).expect("rigctld"), b"d");
        assert!(dest.join("libhamlib-4.dll").is_file() && dest.join("rigctl.exe").is_file());
        assert!(!dest.join("altro.txt").exists() && !dest.join("readme.dll").exists());
        let _ = std::fs::remove_dir_all(&dest);
    }

    #[test]
    fn it_refuses_a_package_with_the_wrong_hash() {
        let zip = pacchetto(&[("hamlib/bin/rigctld.exe", b"d")]);
        let dest = temp("hash");
        let e = install_from_zip(&zip, &"0".repeat(64), &dest).expect_err("impronta diversa");
        assert!(e.contains("SHA-256"));
        assert!(!dest.exists(), "niente copiato se l'impronta non torna");
    }

    #[test]
    fn it_refuses_a_package_without_rigctld() {
        let zip = pacchetto(&[("hamlib/bin/libhamlib-4.dll", b"l")]);
        let dest = temp("senza");
        let e = install_from_zip(&zip, &sha(&zip), &dest).expect_err("manca rigctld");
        assert!(e.contains("rigctld"));
        let _ = std::fs::remove_dir_all(&dest);
    }

    #[test]
    fn it_ignores_path_tricks_in_the_package() {
        let zip = pacchetto(&[("../../fuori/bin/rigctld.exe", b"d"), ("hamlib/bin/rigctld.exe", b"ok")]);
        let dest = temp("traversal");
        install_from_zip(&zip, &sha(&zip), &dest).expect("installato");
        assert_eq!(std::fs::read(dest.join("rigctld.exe")).expect("rigctld"), b"ok");
        let _ = std::fs::remove_dir_all(&dest);
    }
}
