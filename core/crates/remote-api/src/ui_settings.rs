//! Preferenze dell'interfaccia (disposizione delle finestre, filtri, macro...) tenute sul server:
//! cosi' restano anche cambiando indirizzo o browser, e al riavvio non si perde nulla.

use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
    sync::Mutex,
};

/// Preferenze di un tipo di dispositivo: chiave (`aurora-...`) e valore, come li tiene il browser.
pub type Entries = BTreeMap<String, String>;

/// Oltre questa dimensione una richiesta e' rifiutata: sono preferenze, non dati.
pub const MAX_BYTES: usize = 256 * 1024;

pub struct UiSettings {
    path: PathBuf,
    /// Per tipo di dispositivo (`desktop`, `mobile`): sul telefono la disposizione e' un'altra.
    data: Mutex<BTreeMap<String, Entries>>,
}

impl UiSettings {
    /// Legge il file se c'e'; un file assente o rovinato vale "niente salvato".
    pub fn open(path: &Path) -> UiSettings {
        let data = std::fs::read_to_string(path).ok().and_then(|text| serde_json::from_str(&text).ok()).unwrap_or_default();
        UiSettings { path: path.to_path_buf(), data: Mutex::new(data) }
    }

    pub fn get(&self, kind: &str) -> Entries {
        self.data.lock().unwrap_or_else(|p| p.into_inner()).get(kind).cloned().unwrap_or_default()
    }

    /// Sostituisce le preferenze di un tipo di dispositivo e le scrive su disco.
    pub fn put(&self, kind: &str, entries: Entries) -> std::io::Result<()> {
        let mut data = self.data.lock().unwrap_or_else(|p| p.into_inner());
        data.insert(kind.to_string(), entries);
        let text = serde_json::to_string_pretty(&*data).map_err(std::io::Error::other)?;
        // si scrive in un file accanto e lo si rinomina: un'interruzione non lascia un file a meta'
        let tmp = self.path.with_extension("json.tmp");
        std::fs::write(&tmp, text)?;
        std::fs::rename(&tmp, &self.path)
    }
}

/// Tipi di dispositivo ammessi.
pub fn valid_kind(kind: &str) -> bool {
    matches!(kind, "desktop" | "mobile")
}

/// Solo chiavi dell'interfaccia di Aurora e di dimensione ragionevole.
pub fn valid_entries(entries: &Entries) -> bool {
    entries.len() <= 64 && entries.keys().all(|k| k.starts_with("aurora-") && k.len() <= 64)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("aurora-ui-{name}-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        dir.join("ui-settings.json")
    }

    #[test]
    fn what_is_saved_is_still_there_after_a_restart() {
        let path = temp("restart");
        let store = UiSettings::open(&path);
        assert!(store.get("desktop").is_empty());
        store.put("desktop", Entries::from([("aurora-prefs".into(), "{\"a\":1}".into())])).unwrap();
        store.put("mobile", Entries::from([("aurora-layout".into(), "[]".into())])).unwrap();
        let again = UiSettings::open(&path);
        assert_eq!(again.get("desktop")["aurora-prefs"], "{\"a\":1}");
        assert_eq!(again.get("mobile")["aurora-layout"], "[]");
        std::fs::remove_dir_all(path.parent().unwrap()).ok();
    }

    #[test]
    fn a_broken_file_means_nothing_saved_not_a_crash() {
        let path = temp("broken");
        std::fs::write(&path, "non e' json").unwrap();
        assert!(UiSettings::open(&path).get("desktop").is_empty());
        std::fs::remove_dir_all(path.parent().unwrap()).ok();
    }

    #[test]
    fn only_aurora_keys_and_known_kinds_are_accepted() {
        assert!(valid_kind("desktop") && valid_kind("mobile") && !valid_kind("../x") && !valid_kind(""));
        assert!(valid_entries(&Entries::from([("aurora-prefs".into(), "x".into())])));
        assert!(!valid_entries(&Entries::from([("altro".into(), "x".into())])));
    }
}
