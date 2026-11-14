//! L'installazione di un aggiornamento: il pacchetto e la sua firma si scaricano in anticipo (appena il controllo trova la versione nuova)
//! e si controllano con la chiave pubblica di Aurora; all'«Installa» si ricontrolla la firma del file pronto e si installa:
//! - Windows: si chiudono Aurora e l'agente nel tray, si installa in silenzio e solo alla fine la copia di aiuto (vedi [`apply_update`])
//!   riapre Aurora, una volta;
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

/// Gli argomenti dell'installatore NSIS di Tauri: `/S` lo fa girare in silenzio, senza finestre (come le app che si aggiornano da sole),
/// `/UPDATE` dice che e' un aggiornamento di una copia gia' installata. **Niente `/R`**: Aurora la riapre soltanto la copia di aiuto
/// ([`apply_update`]), a installazione finita e riuscita. Con `/R` la riaprivano in due (l'installatore e lo script di allora) e, se l'installatore
/// ci metteva piu' dei 240 s che lo script aspettava, lo script riapriva Aurora vecchia mentre l'installazione era ancora in corso e
/// l'installatore la richiudeva (una delle cause del difetto del 2026-11-13 «si chiude, si riapre, si richiude e installa»; l'altra era
/// l'agente nel tray che apriva Aurora per farle installare l'aggiornamento, vedi `agent::install_update`).
pub fn installer_args() -> [&'static str; 2] {
    ["/S", "/UPDATE"]
}

/// Quanto la copia di aiuto aspetta la fine dell'installatore prima di smettere di guardarlo (20 minuti: un antivirus puo' rallentarlo
/// molto). Se l'installatore e' ancora al lavoro dopo questo tempo, esce senza riaprire niente: mai Aurora a meta' installazione.
const INSTALLER_WAIT_MS: u32 = 20 * 60 * 1000;
/// Quanto si aspetta che Aurora e l'agente nel tray si chiudano da soli prima di chiuderli (il core si ferma con calma, fino a 6 s).
const CLOSE_WAIT_MS: u64 = 30_000;

/// L'argomento con cui Aurora avvia la propria copia di aiuto che applica l'aggiornamento (vedi [`apply_update`]).
pub const APPLY_ARG: &str = "--applica-aggiornamento";

/// Cosa la copia di aiuto deve fare: l'installatore, il processo di Aurora da aspettare, il programma da riaprire (vuoto: non si sa).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApplyArgs {
    pub installer: PathBuf,
    pub pid: u32,
    pub exe: Option<PathBuf>,
}

/// Gli argomenti per la copia di aiuto: `--applica-aggiornamento <installatore> <pid> <programma>`. Passano come argomenti separati,
/// non dentro il testo di uno script: apici e spazi nei percorsi non rompono niente.
pub fn apply_command_args(installer: &str, pid: u32, exe: &str) -> Vec<String> {
    vec![APPLY_ARG.to_string(), installer.to_string(), pid.to_string(), exe.to_string()]
}

/// Legge gli argomenti della riga di comando (senza il nome del programma): `Some` solo se il primo e' [`APPLY_ARG`] e gli altri ci sono.
pub fn parse_apply_args(args: &[String]) -> Option<ApplyArgs> {
    match args {
        [flag, installer, pid, exe] if flag == APPLY_ARG && !installer.is_empty() => Some(ApplyArgs {
            installer: PathBuf::from(installer),
            pid: pid.parse().ok()?,
            exe: (!exe.is_empty()).then(|| PathBuf::from(exe)),
        }),
        _ => None,
    }
}

/// Cosa fare quando l'installatore ha finito.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AfterInstall {
    /// Aurora e' gia' aperta (riaperta da qualcun altro): non se ne apre un'altra.
    Nothing,
    /// Installazione riuscita: si riapre Aurora, una volta sola (all'avvio riaccende anche l'agente nel tray).
    ReopenAurora,
    /// Non riuscita, o il programma non c'e' piu': si riapre l'installatore con le sue finestre, che mostra cosa non va.
    ReopenInstaller,
}

/// La scelta dopo l'installatore, come nello script di prima: `code` e' il codice di uscita (`None` se non si sa, che conta come riuscita).
pub fn after_install(code: Option<u32>, aurora_running: bool, exe_exists: bool) -> AfterInstall {
    if aurora_running {
        AfterInstall::Nothing
    } else if matches!(code, None | Some(0)) && exe_exists {
        AfterInstall::ReopenAurora
    } else {
        AfterInstall::ReopenInstaller
    }
}

/// Il nome della copia di aiuto nella cartella del pacchetto.
fn helper_name(pid: u32) -> String {
    format!("aurora-aggiorna-{pid}.exe")
}

/// Avvia l'aggiornamento su Windows **senza PowerShell ne' cmd** (gli antivirus mettevano in quarantena Aurora che avviava PowerShell
/// nascosto): Aurora copia il proprio programma nella cartella del pacchetto e avvia la copia con [`APPLY_ARG`], staccata da chi la lancia
/// (Aurora o l'agente nel tray, che subito dopo si chiudono). La copia serve perche' l'installatore deve poter sostituire
/// `aurora-desktop.exe`: un programma in uso non si sostituisce. Quello che fa la copia e' in [`apply_update`].
#[cfg(windows)]
pub(crate) fn launch_update_script(installer: &std::path::Path) -> Result<(), String> {
    use std::os::windows::process::CommandExt;
    let installer = installer.to_str().ok_or("percorso del pacchetto non valido")?;
    let exe_path = std::env::current_exe().map_err(|e| format!("percorso del programma sconosciuto: {e}"))?;
    let exe = exe_path.to_str().map(str::to_string).unwrap_or_default();
    let dir = package_dir();
    std::fs::create_dir_all(&dir).map_err(|e| format!("cartella dell'aggiornamento: {e}"))?;
    let helper = dir.join(helper_name(std::process::id()));
    std::fs::copy(&exe_path, &helper).map_err(|e| format!("copia di aiuto non creata: {e}"))?;
    let args = apply_command_args(installer, std::process::id(), &exe);
    // DETACHED_PROCESS | CREATE_NEW_PROCESS_GROUP, e se si puo' fuori dal job di chi lancia (CREATE_BREAKAWAY_FROM_JOB)
    const DETACHED: u32 = 0x0000_0008 | 0x0000_0200;
    const BREAKAWAY: u32 = 0x0100_0000;
    let spawn = |flags: u32| Command::new(&helper).args(&args).current_dir(&dir).creation_flags(flags).spawn();
    spawn(DETACHED | BREAKAWAY).or_else(|_| spawn(DETACHED)).map(|_| ()).map_err(|e| format!("installatore non avviato: {e}"))
}

/// Cancella le copie di aiuto rimaste dagli aggiornamenti passati (una copia non puo' cancellare se stessa). Quelle ancora al lavoro
/// sono aperte e restano.
pub(crate) fn remove_old_helpers() {
    let Ok(entries) = std::fs::read_dir(package_dir()) else { return };
    for entry in entries.filter_map(Result::ok) {
        let name = entry.file_name().to_string_lossy().into_owned();
        if name.starts_with("aurora-aggiorna-") && name.ends_with(".exe") {
            let _ = std::fs::remove_file(entry.path());
        }
    }
}

/// Una riga nel registro dell'aggiornamento (`%TEMP%\Aurora-update\aggiornamento.log`): la copia di aiuto non ha finestre ne' console.
#[cfg(windows)]
fn log_line(text: &str) {
    use std::io::Write;
    let file = package_dir().join("aggiornamento.log");
    // il registro non cresce senza fine: oltre 256 kB si ricomincia
    if std::fs::metadata(&file).is_ok_and(|m| m.len() > 256 * 1024) {
        let _ = std::fs::remove_file(&file);
    }
    if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(&file) {
        let secs = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
        let _ = writeln!(f, "[{secs}] {text}");
    }
}

/// Il lavoro della copia di aiuto, in quest'ordine e con un solo attore per ogni passo (come lo script PowerShell di prima):
/// 1. aspetta che Aurora **e l'agente nel tray** (sono entrambi `aurora-desktop.exe`) siano chiusi, al massimo 30 s, poi chiude chi resta
///    (anche `aurora.exe`, il server senza finestra);
/// 2. avvia l'installatore in silenzio (`/S /UPDATE`, senza `/R`) e ne aspetta la fine (al massimo [`INSTALLER_WAIT_MS`]: dopo, esce
///    senza riaprire niente);
/// 3. se e' andato bene (codice 0) riapre Aurora, **una volta sola**; se non e' andato bene, o il programma non c'e' piu', riapre
///    l'installatore con le sue finestre; se Aurora e' gia' aperta non fa niente (vedi [`after_install`]);
/// 4. se l'installatore non e' nemmeno partito, riapre Aurora com'era (i file non sono stati toccati).
///
/// L'installatore e Aurora si aprono con `ShellExecuteExW` come faceva `Start-Process`: se l'installatore chiede i diritti di
/// amministratore (installazione per tutti gli utenti) Windows mostra la sua conferma.
#[cfg(windows)]
pub fn apply_update(args: &ApplyArgs) {
    use std::time::Instant;
    log_line(&format!("aggiornamento: installatore {}, Aurora {:?}, processo {}", args.installer.display(), args.exe, args.pid));
    let started = Instant::now();
    win::wait_process(args.pid, CLOSE_WAIT_MS as u32);
    while win::running(&["aurora-desktop.exe"]) && started.elapsed() < Duration::from_millis(CLOSE_WAIT_MS) {
        std::thread::sleep(Duration::from_millis(500));
    }
    let killed = win::kill_all(&["aurora-desktop.exe", "aurora.exe"]);
    if killed > 0 {
        log_line(&format!("chiusi {killed} processi di Aurora rimasti aperti"));
    }
    std::thread::sleep(Duration::from_millis(500));

    let exe_dir = args.exe.as_deref().and_then(std::path::Path::parent).map(std::path::Path::to_path_buf);
    let reopen_aurora = || {
        if let Some(exe) = args.exe.as_deref().filter(|p| p.is_file())
            && let Err(e) = win::shell_start(exe, "", exe_dir.as_deref(), false)
        {
            log_line(&format!("Aurora non riaperta: {e}"));
        }
    };
    let process = match win::shell_start(&args.installer, &installer_args().join(" "), None, true) {
        Ok(Some(process)) => process,
        Ok(None) => {
            log_line("installatore avviato senza processo da aspettare: si esce senza riaprire niente");
            return;
        }
        Err(e) => {
            log_line(&format!("installatore non avviato: {e}; si riapre Aurora com'era"));
            reopen_aurora();
            return;
        }
    };
    let Some(code) = process.wait(INSTALLER_WAIT_MS) else {
        log_line("installatore ancora al lavoro dopo 20 minuti: non si riapre niente");
        return;
    };
    log_line(&format!("installatore finito con codice {code:?}"));
    match after_install(code, win::running(&["aurora-desktop.exe"]), args.exe.as_deref().is_some_and(std::path::Path::is_file)) {
        AfterInstall::Nothing => log_line("Aurora e' gia' aperta"),
        AfterInstall::ReopenAurora => reopen_aurora(),
        AfterInstall::ReopenInstaller => {
            log_line("installazione non riuscita: si riapre l'installatore con le sue finestre");
            if let Err(e) = win::shell_start(&args.installer, "", None, false) {
                log_line(&format!("installatore non riaperto: {e}"));
            }
        }
    }
}

/// Avvio di programmi e processi con le API di Windows (niente PowerShell ne' cmd).
#[cfg(windows)]
pub(crate) mod win {
    use std::path::Path;

    use windows_sys::Win32::Foundation::{CloseHandle, HANDLE, INVALID_HANDLE_VALUE, WAIT_OBJECT_0};
    use windows_sys::Win32::System::Diagnostics::ToolHelp::{CreateToolhelp32Snapshot, PROCESSENTRY32W, Process32FirstW, Process32NextW, TH32CS_SNAPPROCESS};
    use windows_sys::Win32::System::Threading::{GetExitCodeProcess, OpenProcess, PROCESS_SYNCHRONIZE, PROCESS_TERMINATE, TerminateProcess, WaitForSingleObject};
    use windows_sys::Win32::UI::Shell::{SEE_MASK_FLAG_NO_UI, SEE_MASK_NOASYNC, SEE_MASK_NOCLOSEPROCESS, SHELLEXECUTEINFOW, ShellExecuteExW};
    use windows_sys::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;

    fn wide(s: &std::ffi::OsStr) -> Vec<u16> {
        use std::os::windows::ffi::OsStrExt;
        s.encode_wide().chain(std::iter::once(0)).collect()
    }

    /// Un processo avviato, che si chiude da solo.
    pub struct Process(HANDLE);
    impl Process {
        /// Aspetta la fine al massimo `ms`: `None` se non e' finito, altrimenti il codice di uscita (`Some(None)` se non si legge).
        pub fn wait(&self, ms: u32) -> Option<Option<u32>> {
            // SAFETY: handle valido finche' vive `self`
            unsafe {
                if WaitForSingleObject(self.0, ms) != WAIT_OBJECT_0 {
                    return None;
                }
                let mut code = 0u32;
                Some((GetExitCodeProcess(self.0, &mut code) != 0).then_some(code))
            }
        }
    }
    impl Drop for Process {
        fn drop(&mut self) {
            // SAFETY: handle avuto da ShellExecuteExW, chiuso una volta sola
            unsafe { CloseHandle(self.0) };
        }
    }

    /// Apre `file` con i suoi argomenti come `Start-Process` (`ShellExecuteExW`, verbo predefinito: se il programma chiede i diritti
    /// di amministratore Windows mostra la conferma). Con `keep` restituisce il processo da aspettare.
    pub fn shell_start(file: &Path, params: &str, dir: Option<&Path>, keep: bool) -> Result<Option<Process>, String> {
        let file_w = wide(file.as_os_str());
        let params_w = wide(std::ffi::OsStr::new(params));
        let dir_w = dir.map(|d| wide(d.as_os_str()));
        // SAFETY: struttura azzerata e poi riempita; le stringhe vivono fino alla fine della chiamata
        unsafe {
            let mut info: SHELLEXECUTEINFOW = std::mem::zeroed();
            info.cbSize = size_of::<SHELLEXECUTEINFOW>() as u32;
            info.fMask = SEE_MASK_NOASYNC | SEE_MASK_FLAG_NO_UI | if keep { SEE_MASK_NOCLOSEPROCESS } else { 0 };
            info.lpFile = file_w.as_ptr();
            info.lpParameters = if params.is_empty() { std::ptr::null() } else { params_w.as_ptr() };
            info.lpDirectory = dir_w.as_ref().map_or(std::ptr::null(), |d| d.as_ptr());
            info.nShow = SW_SHOWNORMAL;
            if ShellExecuteExW(&mut info) == 0 {
                return Err(std::io::Error::last_os_error().to_string());
            }
            Ok((keep && !info.hProcess.is_null()).then(|| Process(info.hProcess)))
        }
    }

    /// I processi con uno di questi nomi di file (minuscoli), tranne questo.
    fn find(names: &[&str]) -> Vec<u32> {
        let me = std::process::id();
        let mut found = Vec::new();
        // SAFETY: istantanea chiusa in fondo; dwSize impostato come vuole Process32FirstW
        unsafe {
            let snap = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0);
            if snap == INVALID_HANDLE_VALUE {
                return found;
            }
            let mut entry = PROCESSENTRY32W { dwSize: size_of::<PROCESSENTRY32W>() as u32, ..Default::default() };
            let mut ok = Process32FirstW(snap, &mut entry) != 0;
            while ok {
                let end = entry.szExeFile.iter().position(|&c| c == 0).unwrap_or(entry.szExeFile.len());
                let name = String::from_utf16_lossy(&entry.szExeFile[..end]).to_lowercase();
                if entry.th32ProcessID != me && names.contains(&name.as_str()) {
                    found.push(entry.th32ProcessID);
                }
                ok = Process32NextW(snap, &mut entry) != 0;
            }
            CloseHandle(snap);
        }
        found
    }

    /// `true` se c'e' un processo con uno di questi nomi.
    pub fn running(names: &[&str]) -> bool {
        !find(names).is_empty()
    }

    /// Chiude d'autorita' i processi con questi nomi (come `Stop-Process -Force`); restituisce quanti.
    pub fn kill_all(names: &[&str]) -> usize {
        find(names)
            .into_iter()
            .filter(|&pid| {
                // SAFETY: handle chiuso subito dopo
                unsafe {
                    let h = OpenProcess(PROCESS_TERMINATE, 0, pid);
                    if h.is_null() {
                        return false;
                    }
                    let ok = TerminateProcess(h, 1) != 0;
                    CloseHandle(h);
                    ok
                }
            })
            .count()
    }

    /// Aspetta che il processo `pid` finisca, al massimo `ms` (se non c'e' piu' torna subito).
    pub fn wait_process(pid: u32, ms: u32) {
        // SAFETY: handle chiuso subito dopo l'attesa
        unsafe {
            let h = OpenProcess(PROCESS_SYNCHRONIZE, 0, pid);
            if !h.is_null() {
                WaitForSingleObject(h, ms);
                CloseHandle(h);
            }
        }
    }
}

/// Un solo avvio dell'installatore per vita del programma: due richieste ravvicinate (la locandina che parte da sola e la barretta, o
/// la cartolina del tray e la locandina) lanciavano due script e due installatori, e il secondo richiudeva l'Aurora appena riaperta dal primo.
static INSTALL_LAUNCHED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// Prende il diritto di avviare l'installatore: `false` se e' gia' stato avviato.
pub(crate) fn claim_install() -> bool {
    !INSTALL_LAUNCHED.swap(true, std::sync::atomic::Ordering::SeqCst)
}

/// L'avvio non e' riuscito: si potra' riprovare.
pub(crate) fn release_install() {
    INSTALL_LAUNCHED.store(false, std::sync::atomic::Ordering::SeqCst);
}

/// Prepara il pacchetto (scaricato e con la firma controllata, o gia' pronto e ricontrollato) con la chiave e la cartella di Aurora:
/// lo usa anche l'agente nel tray quando aggiorna con Aurora chiusa.
pub(crate) fn prepare_package(updates: &UpdateService) -> Result<updater::Prepared, String> {
    updates.prepare(PUBLIC_KEY, &package_dir(), MAX_PACKAGE_BYTES)
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
        let ready = prepare_package(&updates)?;
        // una seconda richiesta (altro pulsante, la locandina che parte da sola, la cartolina del tray) non lancia un secondo installatore
        if !claim_install() {
            tracing::info!("aggiornamento {} gia' avviato: richiesta doppia ignorata", ready.version);
            return Ok(ready.version);
        }
        updates.set_installing();
        // un attimo perche' la barra e la cartolina del tray mostrino «Aurora si chiude per installare» prima della chiusura
        std::thread::sleep(Duration::from_millis(700));
        // la copia di aiuto aspetta che Aurora e l'agente siano chiusi (il core si ferma con calma, fino a 6 s: un installatore partito
        // subito trovava Aurora ancora aperta, 2026-11-08), installa e solo alla fine riapre Aurora, una volta; se l'installazione non
        // riesce riapre l'installatore con le sue finestre (vedi `apply_update`)
        if let Err(message) = launch_update_script(&ready.path) {
            release_install();
            updates.set_error(&message);
            return Err(message);
        }
        // l'agente nel tray e' lo stesso programma: va chiuso, altrimenti l'installatore non puo' sostituirlo (riparte con Aurora nuova)
        crate::stop_agent();
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
    fn the_installer_runs_without_questions_and_does_not_reopen_aurora_itself() {
        let args = installer_args();
        assert!(args.contains(&"/S") && args.contains(&"/UPDATE"));
        assert!(!args.contains(&"/R"), "Aurora la riapre solo lo script: con /R la riaprivano in due");
    }

    const SETUP: &str = r"C:\Users\IU3ABC\AppData\Local\Temp\Aurora-update\Aurora_1.0.0_x64-setup.exe";
    const EXE: &str = r"C:\Users\IU3ABC\AppData\Local\Aurora\aurora-desktop.exe";

    fn strings(args: &[&str]) -> Vec<String> {
        args.iter().map(|a| (*a).to_string()).collect()
    }

    #[test]
    fn the_helper_arguments_go_and_come_back_unchanged() {
        let odd = r"C:\Temp\O'Brien \Aurora-setup.exe";
        let args = apply_command_args(odd, 4321, EXE);
        assert_eq!(args, strings(&[APPLY_ARG, odd, "4321", EXE]));
        assert_eq!(parse_apply_args(&args), Some(ApplyArgs { installer: odd.into(), pid: 4321, exe: Some(EXE.into()) }));
        // programma sconosciuto: si installa lo stesso, ma non si riapre niente
        assert_eq!(parse_apply_args(&apply_command_args(SETUP, 7, "")).and_then(|a| a.exe), None);
    }

    #[test]
    fn other_command_lines_are_not_an_update() {
        assert_eq!(parse_apply_args(&[]), None);
        assert_eq!(parse_apply_args(&strings(&["--agente"])), None);
        assert_eq!(parse_apply_args(&strings(&[APPLY_ARG, SETUP, "4321"])), None, "manca il programma");
        assert_eq!(parse_apply_args(&strings(&[APPLY_ARG, SETUP, "non un numero", EXE])), None);
        assert_eq!(parse_apply_args(&strings(&[APPLY_ARG, "", "1", EXE])), None, "senza installatore");
        assert_eq!(parse_apply_args(&strings(&["--altro", SETUP, "1", EXE])), None);
    }

    #[test]
    fn after_the_installer_aurora_reopens_once_or_the_installer_shows_the_error() {
        assert_eq!(after_install(Some(0), false, true), AfterInstall::ReopenAurora);
        assert_eq!(after_install(None, false, true), AfterInstall::ReopenAurora, "codice sconosciuto come nello script");
        assert_eq!(after_install(Some(2), false, true), AfterInstall::ReopenInstaller, "in errore l'installatore con le sue finestre, non Aurora vecchia");
        assert_eq!(after_install(Some(0), false, false), AfterInstall::ReopenInstaller, "programma sparito");
        assert_eq!(after_install(Some(0), true, true), AfterInstall::Nothing, "Aurora gia' aperta: non se ne apre un'altra");
        assert_eq!(after_install(Some(1), true, true), AfterInstall::Nothing);
    }

    #[test]
    fn the_helper_copy_has_its_own_name() {
        assert_eq!(helper_name(42), "aurora-aggiorna-42.exe");
        assert!(!helper_name(42).eq_ignore_ascii_case("aurora-desktop.exe"), "non deve contare come Aurora aperta");
    }

    #[test]
    fn the_installer_is_claimed_once() {
        release_install();
        assert!(claim_install());
        assert!(!claim_install(), "una seconda richiesta non lancia un secondo installatore");
        release_install();
        assert!(claim_install(), "dopo un avvio fallito si puo' riprovare");
        release_install();
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
