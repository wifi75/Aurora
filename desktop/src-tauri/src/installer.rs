//! L'installazione di un aggiornamento: il pacchetto e la sua firma si scaricano in anticipo (appena il controllo trova la versione nuova)
//! e si controllano con la chiave pubblica di Aurora; all'«Installa» si ricontrolla la firma del file pronto e si installa:
//! - Windows: si avvia l'installatore NSIS e si chiude il programma, che l'installatore riapre;
//! - macOS: si estrae Aurora.app dal pacchetto, si sostituisce quella in uso, si toglie la quarantena e si riapre Aurora dopo la chiusura.
//!
//! **Senza firma valida non si installa mai nulla.**

// su Linux resta solo il rifiuto e su macOS non si usano le parti dell'installatore di Windows
#![cfg_attr(not(windows), allow(dead_code))]

use std::path::PathBuf;
use std::sync::Arc;
#[cfg(windows)]
use std::process::Command;
#[cfg(any(windows, target_os = "macos"))]
use std::time::Duration;

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
/// Dove scaricare a mano la versione nuova se l'aggiornamento dall'app non si puo' fare.
#[cfg(any(target_os = "macos", test))]
const RELEASES_URL: &str = "https://github.com/wifi75/Aurora/releases/latest";

/// Gli argomenti dell'installatore NSIS di Tauri: `/S` lo fa girare in silenzio, senza finestre (come le app che si aggiornano da sole), `/R` fa riaprire Aurora a fine
/// installazione, `/UPDATE` dice che e' un aggiornamento di una copia gia' installata.
pub fn installer_args() -> [&'static str; 3] {
    ["/S", "/R", "/UPDATE"]
}

/// Come lanciare l'installatore **staccato** da Aurora: tramite `cmd /C start`, che termina subito, cosi' l'installatore non risulta figlio di
/// Aurora e non viene chiuso insieme a lei quando la chiude per sostituirla (ne' dalla riga di chiusura dei vecchi disinstallatori).
#[allow(dead_code)]
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
    /// Su Linux non c'e' un installatore da lanciare: chi usa Linux aggiorna ricompilando (vedi `docs/linux.md`).
    #[cfg(not(any(windows, target_os = "macos")))]
    fn install(&self, _updates: Arc<UpdateService>) -> Result<String, String> {
        let _ = &self.app;
        Err("su Linux l'aggiornamento automatico non c'e': si aggiorna ricompilando con scripts/compila-linux.sh".to_string())
    }

    /// Scarica in anticipo e controlla la firma: il file resta pronto per «Installa».
    #[cfg(any(windows, target_os = "macos"))]
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
        // l'installatore parte solo quando Aurora (e l'agente) sono davvero chiusi: alla chiusura il core si ferma con calma (porte, archivi,
        // «Spegni» la radio, fino a 6 s) e l'installatore partito subito trovava Aurora ancora aperta («Impossibile chiudere Aurora»,
        // 2026-11-08, con la cartella di Hamlib gia' tolta). Si aspetta al massimo 30 s, poi parte comunque
        let quoted = installer.replace('\'', "''");
        // se l'installatore silenzioso finisce senza riaprire Aurora (sul portatile, 2026-11-09, dopo l'aggiornamento dal tray non si
        // riapriva nulla e sembrava sparito tutto), lo script la riapre lui. Se a quel punto il programma non c'e' (installazione fallita o
        // interrotta, rapporto del 2026-11-12: «Start-Process: file non trovato») si riapre l'installatore normale, con le sue finestre,
        // cosi' chi aggiorna vede cosa non va invece di restare senza Aurora
        let exe = std::env::current_exe().ok().and_then(|p| p.to_str().map(|s| s.replace('\'', "''"))).unwrap_or_default();
        let script = format!(
            "$t=0; while ((Get-Process aurora-desktop -ErrorAction SilentlyContinue) -and $t -lt 60) {{ Start-Sleep -Milliseconds 500; $t++ }};              Get-Process aurora-desktop,aurora -ErrorAction SilentlyContinue | Stop-Process -Force; Start-Sleep -Milliseconds 500;              $p = Start-Process -FilePath '{quoted}' -ArgumentList {} -PassThru;              $p | Wait-Process -Timeout 240 -ErrorAction SilentlyContinue; Start-Sleep -Seconds 6;              if ('{exe}' -ne '' -and -not (Get-Process aurora-desktop -ErrorAction SilentlyContinue)) {{ if (Test-Path -LiteralPath '{exe}') {{ Start-Process -FilePath '{exe}' }} else {{ Start-Process -FilePath '{quoted}' }} }}",
            installer_args().iter().map(|a| format!("'{a}'")).collect::<Vec<_>>().join(",")
        );
        // percorso intero: su alcuni PC «powershell» non e' nel PATH del programma (2026-11-09: «installatore non avviato: program not found»)
        let mut launcher = Command::new(powershell_exe());
        launcher.args(["-NoProfile", "-ExecutionPolicy", "Bypass", "-WindowStyle", "Hidden", "-Command", &script]);
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
            crate::crash::exiting();
            app.exit(0);
        });
        Ok(ready.version)
    }

    /// macOS: Aurora.app pronta e firmata si estrae e prende il posto di quella in uso; Aurora si chiude e un piccolo comando staccato
    /// la riapre appena la vecchia e' uscita (prima non si potrebbe: il programma ammette una sola istanza).
    #[cfg(target_os = "macos")]
    fn install(&self, updates: Arc<UpdateService>) -> Result<String, String> {
        let ready = updates.prepare(PUBLIC_KEY, &package_dir(), MAX_PACKAGE_BYTES)?;
        let release_url = updates.available().map(|r| r.url).filter(|u| !u.is_empty()).unwrap_or_else(|| RELEASES_URL.to_string());
        updates.set_installing();
        let fail = |e: String| {
            let message = mac::with_release_link(&e, &release_url);
            updates.set_error(&message);
            message
        };
        let app_path = mac::replace_running_app(&ready.path, &package_dir()).map_err(fail)?;
        if let Err(e) = mac::relaunch_after_exit(&app_path) {
            // l'app e' gia' quella nuova: basta riaprirla
            let message = format!("Aurora {} e' installata ma non si e' riaperta da sola ({e}): chiudila e riaprila", ready.version);
            updates.set_error(&message);
            return Err(message);
        }
        tracing::info!("aggiornamento {} installato in {}", ready.version, app_path.display());

        // un attimo per rispondere alla pagina, poi si chiude: il comando staccato riapre la versione nuova
        let app = self.app.clone();
        std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(900));
            crate::crash::exiting();
            app.exit(0);
        });
        Ok(ready.version)
    }
}

/// Le parti della sostituzione di Aurora.app su macOS. Le funzioni pure si provano anche su Windows.
#[cfg(any(target_os = "macos", test))]
mod mac {
    #[cfg(target_os = "macos")]
    use std::ffi::OsStr;
    use std::path::{Path, PathBuf};

    /// Il pacchetto `.app` che contiene il programma in uso: si risale dal percorso dell'eseguibile
    /// (`.../Aurora.app/Contents/MacOS/aurora-desktop`) alla prima cartella che finisce in `.app`.
    pub fn app_bundle_of(exe: &Path) -> Option<PathBuf> {
        exe.ancestors().skip(1).find(|p| p.extension().is_some_and(|e| e.eq_ignore_ascii_case("app"))).map(Path::to_path_buf)
    }

    /// Perche' l'app in quel posto non si puo' sostituire, se non si puo'.
    pub fn replace_blocker(app: &Path) -> Option<&'static str> {
        let text = app.to_string_lossy();
        if text.starts_with("/Volumes/") {
            return Some("Aurora e' aperta direttamente dal disco .dmg: trascinala in Applicazioni, aprila da li' e riprova");
        }
        // Gatekeeper fa girare le app scaricate e mai spostate da una copia di sola lettura (App Translocation)
        if text.contains("/AppTranslocation/") {
            return Some("macOS fa girare Aurora da una copia protetta (non e' mai stata spostata dopo lo scaricamento): spostala in Applicazioni, riaprila e riprova");
        }
        None
    }

    /// Il messaggio d'errore con l'indirizzo dove scaricare a mano la versione nuova.
    pub fn with_release_link(error: &str, release_url: &str) -> String {
        format!("aggiornamento dall'app non riuscito: {error}. Scarica la versione nuova da {release_url}")
    }

    /// Gli argomenti di `/bin/sh` per riaprire Aurora quando questa istanza e' uscita (al massimo dopo un minuto): il percorso e il numero
    /// del processo passano come argomenti, non dentro il testo del comando.
    pub fn relaunch_args(pid: u32, app: &Path) -> Vec<String> {
        let script = "i=0; while kill -0 \"$1\" 2>/dev/null && [ $i -lt 120 ]; do sleep 0.5; i=$((i+1)); done; \
                      /usr/bin/xattr -dr com.apple.quarantine \"$2\" 2>/dev/null; /usr/bin/open -n \"$2\"";
        vec!["-c".into(), script.into(), "aurora-riavvio".into(), pid.to_string(), app.to_string_lossy().into_owned()]
    }

    /// La sola cartella `.app` dentro la cartella estratta.
    pub fn find_extracted_app(dir: &Path) -> Result<PathBuf, String> {
        let apps: Vec<PathBuf> = std::fs::read_dir(dir)
            .map_err(|e| format!("pacchetto estratto non leggibile: {e}"))?
            .filter_map(Result::ok)
            .map(|e| e.path())
            .filter(|p| p.is_dir() && p.extension().is_some_and(|e| e.eq_ignore_ascii_case("app")))
            .collect();
        match apps.as_slice() {
            [app] if app.join("Contents").join("MacOS").is_dir() && app.join("Contents").join("Info.plist").is_file() => Ok(app.clone()),
            [_] => Err("il pacchetto non contiene un'app completa".into()),
            [] => Err("il pacchetto non contiene Aurora.app".into()),
            _ => Err("il pacchetto contiene piu' di un'app".into()),
        }
    }

    #[cfg(target_os = "macos")]
    fn run(program: &str, args: &[&OsStr]) -> Result<(), String> {
        let out = std::process::Command::new(program).args(args).output().map_err(|e| format!("{program} non avviato: {e}"))?;
        if out.status.success() { Ok(()) } else { Err(format!("{program}: {}", String::from_utf8_lossy(&out.stderr).trim())) }
    }

    #[cfg(target_os = "macos")]
    fn not_writable(dir: &Path, e: &std::io::Error) -> String {
        // 30 = EROFS (disco di sola lettura)
        if e.kind() == std::io::ErrorKind::PermissionDenied || e.raw_os_error() == Some(30) {
            format!("non ho il permesso di scrivere in {} (serve un utente amministratore, oppure tieni Aurora in una cartella tua)", dir.display())
        } else {
            format!("non riesco a scrivere in {}: {e}", dir.display())
        }
    }

    /// Estrae `package` (Aurora.app compressa, firma gia' controllata) in una cartella temporanea dentro `work_dir`, la porta accanto
    /// all'app in uso e scambia le due cartelle; la vecchia si cancella. Se lo scambio non riesce, la vecchia torna al suo posto.
    /// Ritorna il percorso dell'app aggiornata (lo stesso di prima).
    #[cfg(target_os = "macos")]
    pub fn replace_running_app(package: &Path, work_dir: &Path) -> Result<PathBuf, String> {
        let exe = std::env::current_exe().map_err(|e| format!("percorso del programma sconosciuto: {e}"))?;
        let exe = std::fs::canonicalize(&exe).unwrap_or(exe);
        let app = app_bundle_of(&exe).ok_or("Aurora non gira da un pacchetto .app")?;
        if let Some(why) = replace_blocker(&app) {
            return Err(why.to_string());
        }
        let parent = app.parent().ok_or("cartella dell'app sconosciuta")?;
        let app_name = app.file_name().ok_or("nome dell'app sconosciuto")?;
        let pid = std::process::id();
        let cleanup = |p: &Path| {
            let _ = std::fs::remove_dir_all(p);
        };

        // per prima cosa: si puo' scrivere accanto all'app? (cartella d'appoggio sullo stesso disco, per gli spostamenti)
        let staging = parent.join(format!(".Aurora-aggiornamento-{pid}"));
        cleanup(&staging);
        std::fs::create_dir(&staging).map_err(|e| not_writable(parent, &e))?;

        // estrazione nella cartella temporanea (tar di sistema: niente dipendenze in piu')
        let extracted = work_dir.join(format!("estratto-{pid}"));
        cleanup(&extracted);
        let result = (|| {
            std::fs::create_dir_all(&extracted).map_err(|e| format!("cartella temporanea: {e}"))?;
            run("/usr/bin/tar", &[OsStr::new("-xzf"), package.as_os_str(), OsStr::new("-C"), extracted.as_os_str()]).map_err(|e| format!("pacchetto non estratto: {e}"))?;
            let new_app = find_extracted_app(&extracted)?;
            // la quarantena si toglie prima di metterla al suo posto (e di nuovo alla riapertura)
            let _ = run("/usr/bin/xattr", &[OsStr::new("-dr"), OsStr::new("com.apple.quarantine"), new_app.as_os_str()]);
            let staged = staging.join(app_name);
            // stesso disco: basta spostarla; altrimenti si copia con ditto (tiene firme, collegamenti e attributi)
            if std::fs::rename(&new_app, &staged).is_err() {
                run("/usr/bin/ditto", &[new_app.as_os_str(), staged.as_os_str()]).map_err(|e| format!("app nuova non copiata accanto a quella in uso: {e}"))?;
            }
            let old = staging.join("precedente.app");
            std::fs::rename(&app, &old).map_err(|e| not_writable(parent, &e))?;
            if let Err(e) = std::fs::rename(&staged, &app) {
                // la vecchia torna al suo posto: Aurora resta com'era
                let _ = std::fs::rename(&old, &app);
                return Err(not_writable(parent, &e));
            }
            Ok::<(), String>(())
        })();
        cleanup(&extracted);
        // dentro c'e' anche la vecchia app: il programma in uso continua a girare (i file aperti restano validi fino alla chiusura)
        cleanup(&staging);
        result?;
        let _ = run("/usr/bin/xattr", &[OsStr::new("-dr"), OsStr::new("com.apple.quarantine"), app.as_os_str()]);
        Ok(app)
    }

    /// Lancia, staccato da Aurora, il comando che la riapre quando questa istanza e' uscita.
    #[cfg(target_os = "macos")]
    pub fn relaunch_after_exit(app: &Path) -> Result<(), String> {
        use std::os::unix::process::CommandExt;
        use std::process::{Command, Stdio};
        Command::new("/bin/sh")
            .args(relaunch_args(std::process::id(), app))
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            // gruppo di processi suo: la chiusura di Aurora non se lo porta dietro
            .process_group(0)
            .spawn()
            .map(|_| ())
            .map_err(|e| e.to_string())
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
        assert!(args.contains(&"/S") && args.contains(&"/R"));
    }

    #[test]
    fn the_installer_is_started_detached_through_cmd_start() {
        let args = detached_command_args(r"C:\Temp\Aurora-update\Aurora_1.0.0_x64-setup.exe");
        assert_eq!(&args[..3], ["/C", "start", ""], "start vuole prima il titolo (vuoto)");
        assert_eq!(args[3], r"C:\Temp\Aurora-update\Aurora_1.0.0_x64-setup.exe");
        assert_eq!(&args[4..], ["/S", "/R", "/UPDATE"]);
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

    #[test]
    fn the_running_app_is_found_from_the_executable() {
        use std::path::Path;
        let exe = Path::new("/Applications/Aurora.app/Contents/MacOS/aurora-desktop");
        assert_eq!(mac::app_bundle_of(exe).as_deref(), Some(Path::new("/Applications/Aurora.app")));
        let nested = Path::new("/Users/x/Programmi/Aurora.app/Contents/Resources/hamlib/rigctld");
        assert_eq!(mac::app_bundle_of(nested).as_deref(), Some(Path::new("/Users/x/Programmi/Aurora.app")));
        assert!(mac::app_bundle_of(Path::new("/usr/local/bin/aurora-desktop")).is_none(), "fuori da un .app non si sostituisce niente");
    }

    #[test]
    fn an_app_on_the_dmg_or_translocated_is_not_replaced() {
        use std::path::Path;
        assert!(mac::replace_blocker(Path::new("/Volumes/Aurora/Aurora.app")).is_some_and(|w| w.contains(".dmg")));
        assert!(mac::replace_blocker(Path::new("/private/var/folders/ab/T/AppTranslocation/1234/d/Aurora.app")).is_some());
        assert!(mac::replace_blocker(Path::new("/Applications/Aurora.app")).is_none());
        let message = mac::with_release_link("permesso negato", RELEASES_URL);
        assert!(message.contains("permesso negato") && message.contains(RELEASES_URL));
    }

    #[test]
    fn the_relaunch_waits_for_this_process_and_passes_the_path_as_an_argument() {
        let args = mac::relaunch_args(4321, std::path::Path::new("/Applications/Aurora \"prova\".app"));
        assert_eq!(args[0], "-c");
        assert!(args[1].contains("kill -0") && args[1].contains("open -n \"$2\"") && args[1].contains("com.apple.quarantine"));
        assert!(!args[1].contains("prova"), "il percorso non entra nel testo del comando");
        assert_eq!(&args[3..], ["4321", "/Applications/Aurora \"prova\".app"]);
    }

    #[test]
    fn the_extracted_package_must_hold_exactly_one_complete_app() {
        let dir = std::env::temp_dir().join(format!("aurora-mac-estratto-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        assert!(mac::find_extracted_app(&dir).is_err(), "vuota");
        let app = dir.join("Aurora.app");
        std::fs::create_dir_all(app.join("Contents").join("MacOS")).unwrap();
        assert!(mac::find_extracted_app(&dir).is_err(), "senza Info.plist");
        std::fs::write(app.join("Contents").join("Info.plist"), "<plist/>").unwrap();
        assert_eq!(mac::find_extracted_app(&dir).unwrap(), app);
        std::fs::create_dir_all(dir.join("Altra.app")).unwrap();
        assert!(mac::find_extracted_app(&dir).is_err(), "due app");
        let _ = std::fs::remove_dir_all(&dir);
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

/// PowerShell con il percorso intero (`%SystemRoot%\System32\WindowsPowerShell1.0\powershell.exe`): su alcuni PC «powershell» non e'
/// nel PATH del programma (2026-11-09: «installatore non avviato: program not found»). Se non c'e', il nome semplice.
pub fn powershell_exe() -> std::path::PathBuf {
    std::env::var_os("SystemRoot")
        .map(|r| std::path::PathBuf::from(r).join("System32").join("WindowsPowerShell").join("v1.0").join("powershell.exe"))
        .filter(|p| p.is_file())
        .unwrap_or_else(|| std::path::PathBuf::from("powershell"))
}
