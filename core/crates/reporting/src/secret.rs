//! Segreti (la password di eQSL) protetti dal sistema, mai in chiaro nei file, nel registro o nelle risposte dell'API.
//!
//! Su Windows si usa la protezione dei dati dell'utente (DPAPI, `CryptProtectData`): il file contiene solo il testo cifrato e si puo' leggere soltanto
//! dallo stesso utente di Windows, sullo stesso PC. Altrove la protezione non c'e' e il segreto non si puo' salvare (la voce resta spenta).

use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
    sync::Mutex,
};

/// Dove si tengono i segreti.
pub trait SecretStore: Send + Sync {
    /// Il segreto, se c'e' ed e' leggibile da questo utente.
    fn get(&self, name: &str) -> Option<String>;
    /// Salva (o con testo vuoto cancella) il segreto.
    fn set(&self, name: &str, value: &str) -> Result<(), String>;
    /// C'e' un segreto salvato (senza leggerlo).
    fn has(&self, name: &str) -> bool;
    /// La protezione e' disponibile su questo sistema.
    fn available(&self) -> bool;
}

/// Un segreto in memoria, per le prove.
#[derive(Default)]
pub struct MemorySecrets {
    map: Mutex<BTreeMap<String, String>>,
}

impl SecretStore for MemorySecrets {
    fn get(&self, name: &str) -> Option<String> {
        self.map.lock().ok()?.get(name).cloned()
    }

    fn set(&self, name: &str, value: &str) -> Result<(), String> {
        let mut map = self.map.lock().map_err(|_| "segreti non disponibili".to_string())?;
        if value.is_empty() {
            map.remove(name);
        } else {
            map.insert(name.to_string(), value.to_string());
        }
        Ok(())
    }

    fn has(&self, name: &str) -> bool {
        self.map.lock().is_ok_and(|m| m.contains_key(name))
    }

    fn available(&self) -> bool {
        true
    }
}

/// File cifrati con la protezione dei dati dell'utente di Windows, uno per segreto, nella cartella dei dati (`<nome>.secret`).
pub struct ProtectedFiles {
    dir: PathBuf,
}

impl ProtectedFiles {
    pub fn new(dir: &Path) -> ProtectedFiles {
        ProtectedFiles { dir: dir.to_path_buf() }
    }

    fn path(&self, name: &str) -> Option<PathBuf> {
        // il nome entra in un nome di file: solo lettere, cifre e trattino
        (!name.is_empty() && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')).then(|| self.dir.join(format!("{name}.secret")))
    }
}

impl SecretStore for ProtectedFiles {
    fn get(&self, name: &str) -> Option<String> {
        let blob = std::fs::read(self.path(name)?).ok()?;
        let plain = unprotect(&blob).ok()?;
        String::from_utf8(plain).ok()
    }

    fn set(&self, name: &str, value: &str) -> Result<(), String> {
        let path = self.path(name).ok_or("nome del segreto non valido")?;
        if value.is_empty() {
            return match std::fs::remove_file(&path) {
                Ok(()) => Ok(()),
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
                Err(e) => Err(format!("segreto non cancellato: {e}")),
            };
        }
        let blob = protect(value.as_bytes())?;
        std::fs::create_dir_all(&self.dir).map_err(|e| format!("cartella dei dati: {e}"))?;
        let tmp = path.with_extension("secret.tmp");
        std::fs::write(&tmp, blob).and_then(|()| std::fs::rename(&tmp, &path)).map_err(|e| format!("segreto non salvato: {e}"))
    }

    fn has(&self, name: &str) -> bool {
        self.path(name).is_some_and(|p| p.is_file())
    }

    fn available(&self) -> bool {
        cfg!(windows)
    }
}

/// Un testo in piu' che lega il segreto ad Aurora (stessa utenza, ma un altro programma non lo apre per caso).
#[cfg(windows)]
const ENTROPY: &[u8] = b"Aurora eQSL";

#[cfg(windows)]
fn protect(plain: &[u8]) -> Result<Vec<u8>, String> {
    use windows::Win32::{
        Foundation::{LocalFree, HLOCAL},
        Security::Cryptography::{CryptProtectData, CRYPTPROTECT_UI_FORBIDDEN, CRYPT_INTEGER_BLOB},
    };
    let input = CRYPT_INTEGER_BLOB { cbData: u32::try_from(plain.len()).map_err(|_| "segreto troppo lungo")?, pbData: plain.as_ptr().cast_mut() };
    let entropy = CRYPT_INTEGER_BLOB { cbData: ENTROPY.len() as u32, pbData: ENTROPY.as_ptr().cast_mut() };
    let mut output = CRYPT_INTEGER_BLOB::default();
    // SAFETY: i puntatori restano validi per tutta la chiamata; il risultato e' allocato dal sistema, si copia e si libera subito.
    unsafe {
        CryptProtectData(&input, None, Some(&entropy), None, None, CRYPTPROTECT_UI_FORBIDDEN, &mut output).map_err(|e| format!("protezione dei dati di Windows: {e}"))?;
        let copy = std::slice::from_raw_parts(output.pbData, output.cbData as usize).to_vec();
        let _ = LocalFree(Some(HLOCAL(output.pbData.cast())));
        Ok(copy)
    }
}

#[cfg(windows)]
fn unprotect(blob: &[u8]) -> Result<Vec<u8>, String> {
    use windows::Win32::{
        Foundation::{LocalFree, HLOCAL},
        Security::Cryptography::{CryptUnprotectData, CRYPTPROTECT_UI_FORBIDDEN, CRYPT_INTEGER_BLOB},
    };
    let input = CRYPT_INTEGER_BLOB { cbData: u32::try_from(blob.len()).map_err(|_| "segreto troppo lungo")?, pbData: blob.as_ptr().cast_mut() };
    let entropy = CRYPT_INTEGER_BLOB { cbData: ENTROPY.len() as u32, pbData: ENTROPY.as_ptr().cast_mut() };
    let mut output = CRYPT_INTEGER_BLOB::default();
    // SAFETY: come in `protect`.
    unsafe {
        CryptUnprotectData(&input, None, Some(&entropy), None, None, CRYPTPROTECT_UI_FORBIDDEN, &mut output).map_err(|e| format!("protezione dei dati di Windows: {e}"))?;
        let copy = std::slice::from_raw_parts(output.pbData, output.cbData as usize).to_vec();
        let _ = LocalFree(Some(HLOCAL(output.pbData.cast())));
        Ok(copy)
    }
}

#[cfg(not(windows))]
fn protect(_plain: &[u8]) -> Result<Vec<u8>, String> {
    Err("la protezione dei segreti e' disponibile solo su Windows".into())
}

#[cfg(not(windows))]
fn unprotect(_blob: &[u8]) -> Result<Vec<u8>, String> {
    Err("la protezione dei segreti e' disponibile solo su Windows".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_memory_store_sets_reads_and_clears() {
        let s = MemorySecrets::default();
        assert!(!s.has("eqsl"));
        s.set("eqsl", "segreta").unwrap();
        assert_eq!((s.has("eqsl"), s.get("eqsl").as_deref()), (true, Some("segreta")));
        s.set("eqsl", "").unwrap();
        assert!(!s.has("eqsl") && s.get("eqsl").is_none());
    }

    #[cfg(windows)]
    #[test]
    fn the_file_holds_only_encrypted_text_and_is_read_back_by_the_same_user() {
        let dir = std::env::temp_dir().join(format!("aurora-secret-{}", std::process::id()));
        std::fs::remove_dir_all(&dir).ok();
        let s = ProtectedFiles::new(&dir);
        assert!(s.available() && !s.has("eqsl"));
        s.set("eqsl", "Pa55-parola-molto-segreta").unwrap();
        assert!(s.has("eqsl"));
        let raw = std::fs::read(dir.join("eqsl.secret")).unwrap();
        assert!(!raw.windows(8).any(|w| w == b"Pa55-par"), "il file non deve contenere il testo in chiaro");
        assert_eq!(s.get("eqsl").as_deref(), Some("Pa55-parola-molto-segreta"));
        // un file rovinato non si legge e non fa danni
        std::fs::write(dir.join("eqsl.secret"), b"non cifrato").unwrap();
        assert_eq!(s.get("eqsl"), None);
        s.set("eqsl", "").unwrap();
        assert!(!s.has("eqsl"));
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn secret_names_cannot_escape_the_data_folder() {
        let s = ProtectedFiles::new(Path::new("C:/dati"));
        assert!(s.path("../x").is_none() && s.path("").is_none() && s.path("a/b").is_none());
        assert!(s.path("eqsl").is_some());
    }
}
