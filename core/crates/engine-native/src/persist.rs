//! Lo stato dei pulsanti e dei livelli si ricorda tra un avvio e l'altro: tutto cio' che si imposta resta impostato al riavvio
//! (`controls.json` nella cartella dei dati). Non si ricorda solo Monitora (parte sempre acceso); Abilita Tx si ricorda per scelta del proprietario.

use std::{io::Write, path::Path};

use crate::rig::SavedControls;

/// Legge lo stato salvato; un file assente o rovinato non e' un errore (si riparte dai valori di fabbrica).
pub fn load(path: &Path) -> Option<SavedControls> {
    let raw = std::fs::read_to_string(path).ok()?;
    match serde_json::from_str::<SavedControls>(&raw) {
        Ok(saved) if saved.version == SavedControls::VERSION => Some(saved),
        Ok(_) => None,
        Err(e) => {
            tracing::warn!("{} non leggibile, si riparte dai valori di fabbrica: {e}", path.display());
            None
        }
    }
}

/// Scrive lo stato in modo atomico (file temporaneo e rinomina): un'interruzione non lascia un file a meta'.
pub fn save(path: &Path, saved: &SavedControls) -> std::io::Result<()> {
    let text = serde_json::to_string_pretty(saved).map_err(std::io::Error::other)?;
    let tmp = path.with_extension("json.tmp");
    {
        let mut file = std::fs::File::create(&tmp)?;
        file.write_all(text.as_bytes())?;
        file.sync_all()?;
    }
    std::fs::rename(&tmp, path)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp(name: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("aurora-persist-{name}-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("cartella");
        dir.join("controls.json")
    }

    #[test]
    fn what_is_saved_is_read_back() {
        let path = temp("roundtrip");
        let mut saved = SavedControls { version: SavedControls::VERSION, rx_df: 1138, tx_df: 1749, attenuation: 100, ..SavedControls::default() };
        saved.switches.insert("skip_tx1".into(), true);
        saved.levels.insert("20mFT8".into(), 150);
        save(&path, &saved).expect("scrive");
        assert_eq!(load(&path), Some(saved));
    }

    #[test]
    fn a_missing_or_broken_file_means_factory_values() {
        let path = temp("broken");
        assert_eq!(load(&path), None);
        std::fs::write(&path, "{ non json").expect("scrive");
        assert_eq!(load(&path), None);
        std::fs::write(&path, r#"{"version":999}"#).expect("scrive");
        assert_eq!(load(&path), None, "una versione sconosciuta non si applica");
    }
}
