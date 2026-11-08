//! L'installazione di un aggiornamento: il pacchetto e la sua firma si scaricano in anticipo (appena il controllo trova la versione nuova)
//! e si controllano con la chiave pubblica di Aurora; all'«Installa» si ricontrolla la firma del file pronto, si avvia l'installatore e si
//! chiude il programma, che l'installatore riapre. **Senza firma valida non si installa mai nulla.**

// su Linux resta solo il rifiuto: le parti dell'installatore di Windows non si usano
#![cfg_attr(not(windows), allow(dead_code))]

use std::path::PathBuf;
use std::sync::Arc;
#[cfg(windows)]
use std::{process::Command, time::Duration};

use remote_api::UpdateInstaller;
use tauri::AppHandle;
#[cfg(test)]
use updater::safe_file_name;
#[cfg(test)]
use updater::verify_signature;
use updater::UpdateService;

/// La chiave pubblica con cui sono firmati i pacchetti (la privata sta fuori dal repository, sul PC di chi rilascia).
const PUBLIC_KEY: &str = include_str!("../../updater.pub");
/// Un pacchetto di Aurora e' di qualche decina di MB: oltre questo limite qualcosa non va.
const MAX_PACKAGE_BYTES: u64 = 300 * 1024 * 1024;

/// Gli argomenti dell'installatore NSIS di Tauri: `/P` lo fa girare senza domande (solo l'avanzamento), `/R` fa riaprire Aurora a fine
/// installazione, `/UPDATE` dice che e' un aggiornamento di una copia gia' installata.
pub fn installer_args() -> [&'static str; 3] {
    ["/P", "/R", "/UPDATE"]
}

/// Come lanciare l'installatore **staccato** da Aurora: tramite `cmd /C start`, che termina subito, cosi' l'installatore non risulta figlio di
/// Aurora e non viene chiuso insieme a lei quando la chiude per sostituirla (ne' dalla riga di chiusura dei vecchi disinstallatori).
pub fn detached_command_args(installer: &str) -> Vec<String> {
    let mut args = vec!["/C".to_string(), "start".to_string(), String::new(), installer.to_string()];
    args.extend(installer_args().iter().map(|a| (*a).to_string()));
    args
}

/// La cartella dove si tiene il pacchetto scaricato in anticipo.
fn package_dir() -> PathBuf {
    std::env::temp_dir().join("Aurora-update")
}

pub struct DesktopInstaller {
    app: AppHandle,
}

impl DesktopInstaller {
    pub fn new(app: AppHandle) -> Arc<DesktopInstaller> {
        Arc::new(DesktopInstaller { app })
    }
}

/// Controlla la firma di un pacchetto con la chiave pubblica di Aurora (il controllo vero lo fa `UpdateService::prepare`, con la stessa
/// chiave: qui serve alle prove del pacchetto appena compilato).
#[cfg(test)]
pub fn verify_package(data: &[u8], signature: &str) -> Result<(), String> {
    verify_signature(data, signature, PUBLIC_KEY)
}

impl UpdateInstaller for DesktopInstaller {
    /// Fuori da Windows non c'e' un installatore da lanciare: chi usa Linux aggiorna ricompilando (vedi `docs/linux.md`).
    #[cfg(not(windows))]
    fn install(&self, _updates: Arc<UpdateService>) -> Result<String, String> {
        let _ = &self.app;
        Err("su Linux l'aggiornamento automatico non c'e': si aggiorna ricompilando con scripts/compila-linux.sh".to_string())
    }

    /// Scarica in anticipo e controlla la firma: il file resta pronto per «Installa».
    #[cfg(windows)]
    fn prepare(&self, updates: Arc<UpdateService>) -> Result<(), String> {
        updates.prepare(PUBLIC_KEY, &package_dir(), MAX_PACKAGE_BYTES).map(|_| ())
    }

    #[cfg(windows)]
    fn install(&self, updates: Arc<UpdateService>) -> Result<String, String> {
        // il pacchetto scaricato in anticipo si usa cosi' com'e' (con la firma ricontrollata sul file); se non c'e', si scarica adesso.
        // Se lo scaricamento in sottofondo e' ancora in corso, si aspetta che finisca invece di ricominciare.
        let ready = updates.prepare(PUBLIC_KEY, &package_dir(), MAX_PACKAGE_BYTES)?;
        let installer = ready.path.to_str().ok_or("percorso del pacchetto non valido")?.to_string();
        updates.set_installing();
        // un attimo perche' la barra e la cartolina del tray mostrino «Installo» prima che l'agente venga chiuso
        std::thread::sleep(Duration::from_millis(700));
        // l'agente nel tray e' lo stesso programma: va chiuso, altrimenti l'installatore non puo' sostituirlo
        crate::stop_agent();
        let mut launcher = Command::new("cmd");
        launcher.args(detached_command_args(&installer));
        {
            use std::os::windows::process::CommandExt;
            // `cmd` non deve aprire una finestra nera: l'installatore parte da solo
            launcher.creation_flags(0x0800_0000);
        }
        if let Err(e) = launcher.spawn() {
            let message = format!("installatore non avviato: {e}");
            updates.set_error(&message);
            return Err(message);
        }
        tracing::info!("aggiornamento {} avviato da {}", ready.version, ready.path.display());

        // un attimo per rispondere alla pagina, poi si chiude: l'installatore non puo' sostituire un programma aperto
        let app = self.app.clone();
        std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(900));
            app.exit(0);
        });
        Ok(ready.version)
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
    fn the_installer_is_started_detached_through_cmd_start() {
        let args = detached_command_args(r"C:\Temp\Aurora-update\Aurora_1.0.0_x64-setup.exe");
        assert_eq!(&args[..3], ["/C", "start", ""], "start vuole prima il titolo (vuoto)");
        assert_eq!(args[3], r"C:\Temp\Aurora-update\Aurora_1.0.0_x64-setup.exe");
        assert_eq!(&args[4..], ["/P", "/R", "/UPDATE"]);
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
