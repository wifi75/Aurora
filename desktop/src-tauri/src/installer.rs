//! L'installazione di un aggiornamento: scarica il pacchetto e la sua firma dalla release, controlla la firma con la chiave pubblica di
//! Aurora, avvia l'installatore e chiude il programma, che l'installatore riapre. **Senza firma valida non si installa mai nulla.**

use std::{path::PathBuf, process::Command, sync::Arc, time::Duration};

use remote_api::UpdateInstaller;
use tauri::AppHandle;
use updater::{installer_assets, verify_signature, UpdateService};

/// La chiave pubblica con cui sono firmati i pacchetti (la privata sta fuori dal repository, sul PC di chi rilascia).
const PUBLIC_KEY: &str = include_str!("../../updater.pub");
/// Un pacchetto di Aurora e' di qualche decina di MB: oltre questo limite qualcosa non va.
const MAX_PACKAGE_BYTES: u64 = 300 * 1024 * 1024;
const MAX_SIGNATURE_BYTES: u64 = 16 * 1024;

/// Gli argomenti dell'installatore NSIS di Tauri: `/P` lo fa girare senza domande (solo l'avanzamento), `/R` fa riaprire Aurora a fine
/// installazione, `/UPDATE` dice che e' un aggiornamento di una copia gia' installata.
pub fn installer_args() -> [&'static str; 3] {
    ["/P", "/R", "/UPDATE"]
}

/// Il nome del file da salvare: solo la parte finale, senza percorsi, e solo se sembra un installatore.
pub fn safe_file_name(asset_name: &str) -> Option<String> {
    let name = asset_name.rsplit(['/', '\\']).next()?.trim();
    let ok = name.len() <= 100 && name.ends_with("-setup.exe") && name.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'));
    ok.then(|| name.to_string())
}

pub struct DesktopInstaller {
    app: AppHandle,
}

impl DesktopInstaller {
    pub fn new(app: AppHandle) -> Arc<DesktopInstaller> {
        Arc::new(DesktopInstaller { app })
    }
}

/// Controlla la firma di un pacchetto con la chiave pubblica di Aurora.
pub fn verify_package(data: &[u8], signature: &str) -> Result<(), String> {
    verify_signature(data, signature, PUBLIC_KEY)
}

impl UpdateInstaller for DesktopInstaller {
    fn install(&self, updates: Arc<UpdateService>) -> Result<String, String> {
        let release = updates.available().ok_or("nessuna versione nuova trovata")?;
        let (setup, sig) = installer_assets(&release).ok_or("la release non ha il pacchetto di installazione con la sua firma")?;
        let file_name = safe_file_name(&setup.name).ok_or("nome del pacchetto non valido")?;
        let data = updates.download(setup, MAX_PACKAGE_BYTES)?;
        let signature = String::from_utf8(updates.download(sig, MAX_SIGNATURE_BYTES)?).map_err(|_| "firma illeggibile".to_string())?;
        verify_package(&data, &signature)?;

        let dir: PathBuf = std::env::temp_dir().join("Aurora-update");
        std::fs::create_dir_all(&dir).map_err(|e| format!("cartella temporanea: {e}"))?;
        let path = dir.join(file_name);
        std::fs::write(&path, &data).map_err(|e| format!("pacchetto non salvato: {e}"))?;
        Command::new(&path).args(installer_args()).spawn().map_err(|e| format!("installatore non avviato: {e}"))?;
        tracing::info!("aggiornamento {} avviato da {}", release.version, path.display());

        // un attimo per rispondere alla pagina, poi si chiude: l'installatore non puo' sostituire un programma aperto
        let app = self.app.clone();
        std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(900));
            app.exit(0);
        });
        Ok(release.version)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_a_plain_installer_name_is_accepted() {
        assert_eq!(safe_file_name("Aurora_0.10.1_x64-setup.exe").as_deref(), Some("Aurora_0.10.1_x64-setup.exe"));
        assert_eq!(safe_file_name("..\\..\\Windows\\Aurora-setup.exe").as_deref(), Some("Aurora-setup.exe"), "solo la parte finale");
        assert_eq!(safe_file_name("a/b/Aurora-setup.exe").as_deref(), Some("Aurora-setup.exe"));
        assert!(safe_file_name("Aurora.exe").is_none());
        assert!(safe_file_name("Aurora setup-setup.exe").is_none());
        assert!(safe_file_name("x;calc-setup.exe").is_none());
        assert!(safe_file_name("").is_none());
    }

    #[test]
    fn the_installer_runs_without_questions_and_reopens_aurora() {
        let args = installer_args();
        assert!(args.contains(&"/P") && args.contains(&"/R"));
    }

    #[test]
    fn a_package_without_a_valid_signature_is_refused() {
        assert!(verify_package(b"pacchetto", "").is_err());
        assert!(verify_package(b"pacchetto", "non una firma").is_err());
    }

    #[test]
    fn the_public_key_is_a_key() {
        // la chiave incorporata si legge: una firma fasulla viene rifiutata per la firma, non perche' la chiave e' illeggibile
        let e = verify_package(b"x", "dW50cnVzdGVkIGNvbW1lbnQ6IHNpZ25hdHVyZQo=").unwrap_err();
        assert!(!e.contains("chiave pubblica non valida"), "{e}");
    }

    /// Se c'e' un pacchetto appena fatto con `scripts/rilascio-windows.ps1`, la sua firma deve tornare con la chiave pubblica incorporata.
    #[test]
    fn the_package_built_by_the_release_script_verifies_with_the_embedded_key() {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../dist-release");
        let Ok(entries) = std::fs::read_dir(&dir) else { return };
        for entry in entries.filter_map(Result::ok) {
            let path = entry.path();
            if !path.to_string_lossy().ends_with("-setup.exe") {
                continue;
            }
            let data = std::fs::read(&path).expect("pacchetto leggibile");
            let signature = std::fs::read_to_string(format!("{}.sig", path.display())).expect("firma accanto al pacchetto");
            assert_eq!(verify_package(&data, &signature), Ok(()), "{}", path.display());
            let mut tampered = data.clone();
            tampered[data.len() / 2] ^= 0xFF;
            assert!(verify_package(&tampered, &signature).is_err(), "un pacchetto alterato va rifiutato");
        }
    }
}
