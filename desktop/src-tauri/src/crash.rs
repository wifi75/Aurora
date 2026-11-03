//! Registrazione dei guasti, sempre accesa: nella cartella `crash` accanto ai dati (di norma %APPDATA%\Aurora\crash) finiscono
//! - `hang-AAAAMMGG-HHMMSS.dmp` + `.txt`: il programma e' bloccato (il runtime o la finestra non battono da oltre 20 s), un minidump con
//!   lo stack di tutti i thread, uno per blocco;
//! - `crash-….dmp` + `.txt`: un'eccezione non gestita (accesso a memoria non valida e simili), prima che Windows chiuda il processo;
//! - `panic-….txt`: un panic Rust con messaggio, thread, punto e backtrace;
//! - `stderr.log`: quello che Rust scrive sull'errore standard prima di un `abort` (allocazione fallita, panic che non puo' risalire,
//!   errori fatali del runtime). Un programma senza console lo perderebbe: e' l'unica traccia dei fast-fail (c0000409 sottocodice 7).
//!
//! Il guardiano scrive i file direttamente, senza passare dal logger (che potrebbe essere fermo con il resto); la riga nel log arriva dopo,
//! da un thread a parte. Si tengono gli ultimi 10 dump e 50 file di testo.

use std::{
    fmt::Write as _,
    path::{Path, PathBuf},
    sync::{
        Mutex, OnceLock,
        atomic::{AtomicU64, Ordering},
    },
    time::{Duration, Instant},
};

const VERSION: &str = env!("CARGO_PKG_VERSION");
/// Ogni quanto battono il runtime e la finestra, e ogni quanto guarda il guardiano.
const BEAT_EVERY: Duration = Duration::from_secs(2);
/// Oltre questo silenzio il programma si considera bloccato.
const HANG_AFTER: Duration = Duration::from_secs(20);
const KEEP_DUMPS: usize = 10;
const KEEP_TEXTS: usize = 50;
const MAX_STDERR_BYTES: u64 = 1024 * 1024;

static DIR: OnceLock<PathBuf> = OnceLock::new();
static START: OnceLock<Instant> = OnceLock::new();
/// dbghelp non e' rientrante: un dump alla volta.
static DUMP_LOCK: Mutex<()> = Mutex::new(());

/// Un battito: i millisecondi dall'avvio dell'ultima volta che la sorgente ha dato segno di vita.
pub struct Beat {
    pub name: &'static str,
    last_ms: AtomicU64,
}

impl Beat {
    const fn new(name: &'static str) -> Beat {
        Beat {
            name,
            last_ms: AtomicU64::new(0),
        }
    }
    pub fn touch(&self) {
        self.last_ms.store(now_ms(), Ordering::Relaxed);
    }
    fn last(&self) -> u64 {
        self.last_ms.load(Ordering::Relaxed)
    }
}

/// Il runtime asincrono (tokio, dove gira il core) e il thread della finestra.
pub static RUNTIME: Beat = Beat::new("runtime");
pub static WINDOW: Beat = Beat::new("finestra");

fn now_ms() -> u64 {
    START.get_or_init(Instant::now).elapsed().as_millis() as u64
}

/// La cartella dei crash (dopo `init`).
pub fn dir() -> Option<&'static Path> {
    DIR.get().map(PathBuf::as_path)
}

/// Crea la cartella, la ripulisce, dirotta l'errore standard nel suo file e installa il filtro delle eccezioni. Da chiamare per prima.
pub fn init(data_dir: &Path) {
    let dir = data_dir.join("crash");
    if std::fs::create_dir_all(&dir).is_err() {
        return;
    }
    let _ = START.get_or_init(Instant::now);
    let _ = DIR.set(dir.clone());
    prune(&dir);
    platform::redirect_stderr(&dir.join("stderr.log"));
    platform::install_exception_filter();
}

/// Apre la cartella dei crash in Esplora risorse (per «Apri la cartella dei crash» nelle impostazioni).
pub fn folder_opener() -> remote_api::OpenCrashFolder {
    std::sync::Arc::new(|| {
        let dir = dir().ok_or_else(|| "cartella dei crash non pronta".to_string())?;
        let mut command = std::process::Command::new("explorer.exe");
        command.arg(dir);
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt as _;
            // CREATE_NO_WINDOW: nessuna console di passaggio
            command.creation_flags(0x0800_0000);
        }
        command.spawn().map(|_| ()).map_err(|e| e.to_string())
    })
}

/// Lo stato del guardiano: segnala un blocco quando inizia, poi tace finche' tutti i battiti non ripartono.
#[derive(Debug, Default)]
pub struct Watch {
    stalled: bool,
}

impl Watch {
    /// `beats` = (nome, ultimo battito in ms). Restituisce le sorgenti ferme solo al primo controllo di un blocco nuovo.
    pub fn check<'a>(
        &mut self,
        now_ms: u64,
        beats: &[(&'a str, u64)],
        limit_ms: u64,
    ) -> Option<Vec<(&'a str, u64)>> {
        let stale: Vec<(&str, u64)> = beats
            .iter()
            .filter(|(_, last)| now_ms.saturating_sub(*last) > limit_ms)
            .copied()
            .collect();
        if stale.is_empty() {
            self.stalled = false;
            return None;
        }
        if self.stalled {
            return None;
        }
        self.stalled = true;
        Some(stale)
    }
}

/// Fa battere runtime e finestra e avvia il guardiano in un thread suo.
pub fn spawn_watchdog(app: tauri::AppHandle) {
    // qui il log e' gia' pronto: l'esito della registrazione presso Windows Error Reporting resta scritto
    platform::register_wer();
    RUNTIME.touch();
    WINDOW.touch();
    tauri::async_runtime::spawn(async {
        loop {
            RUNTIME.touch();
            tokio::time::sleep(BEAT_EVERY).await;
        }
    });
    // prova dal vivo: `AURORA_PROVA_BLOCCO=1` ferma la finestra per 30 s un minuto dopo l'avvio (deve comparire un hang-….dmp)
    let test_block = std::env::var("AURORA_PROVA_BLOCCO").is_ok_and(|v| v == "1");
    let _ = std::thread::Builder::new()
        .name("aurora-guardiano".into())
        .spawn(move || {
            let mut watch = Watch::default();
            let mut ticks = 0u32;
            loop {
                // il battito della finestra: un compito eseguito dal suo thread; se e' fermo resta in coda e il tempo non si aggiorna
                let _ = app.run_on_main_thread(|| WINDOW.touch());
                ticks = ticks.saturating_add(1);
                if test_block && ticks == 30 {
                    let _ = app.run_on_main_thread(|| std::thread::sleep(Duration::from_secs(30)));
                }
                std::thread::sleep(BEAT_EVERY);
                let beats = [(RUNTIME.name, RUNTIME.last()), (WINDOW.name, WINDOW.last())];
                if let Some(stale) = watch.check(now_ms(), &beats, HANG_AFTER.as_millis() as u64) {
                    report_hang(&stale, &beats);
                }
            }
        });
}

fn report_hang(stale: &[(&str, u64)], beats: &[(&str, u64)]) {
    let Some(dir) = dir() else { return };
    let now = now_ms();
    let stamp = platform::timestamp();
    let base = format!("hang-{stamp}");
    let dump = dir.join(format!("{base}.dmp"));
    let dump_result = platform::write_minidump(&dump, None);
    let mut text = header("Blocco del programma", &stamp);
    for (name, last) in beats {
        let _ = writeln!(
            text,
            "battito {name}: {:.1} s fa",
            now.saturating_sub(*last) as f64 / 1000.0
        );
    }
    let names: Vec<&str> = stale.iter().map(|(n, _)| *n).collect();
    let _ = writeln!(text, "fermi: {}", names.join(", "));
    let _ = writeln!(text, "minidump: {}", describe(&dump, &dump_result));
    let _ = std::fs::write(dir.join(format!("{base}.txt")), text);
    prune(dir);
    // il log potrebbe essere fermo anche lui: la riga si scrive da un thread a parte che, al limite, resta in attesa da solo
    let line = format!(
        "programma bloccato ({}) da oltre {} s: scritto {}",
        names.join(", "),
        HANG_AFTER.as_secs(),
        dump.display()
    );
    let _ = std::thread::Builder::new()
        .name("aurora-guardiano-log".into())
        .spawn(move || tracing::error!("{line}"));
}

/// Il file di un panic: messaggio, thread, punto e backtrace. Si chiama dal gancio dei panic.
pub fn write_panic_report(info: &std::panic::PanicHookInfo<'_>) {
    let Some(dir) = dir() else { return };
    let stamp = platform::timestamp();
    let mut text = header("Panic", &stamp);
    let thread = std::thread::current();
    let _ = writeln!(text, "thread: {}", thread.name().unwrap_or("senza nome"));
    if let Some(at) = info.location() {
        let _ = writeln!(text, "punto: {}:{}:{}", at.file(), at.line(), at.column());
    }
    let message = info
        .payload()
        .downcast_ref::<&str>()
        .map(|s| (*s).to_string())
        .or_else(|| info.payload().downcast_ref::<String>().cloned());
    let _ = writeln!(
        text,
        "messaggio: {}",
        message.as_deref().unwrap_or("(non testuale)")
    );
    let _ = writeln!(
        text,
        "\nbacktrace:\n{}",
        std::backtrace::Backtrace::force_capture()
    );
    let _ = writeln!(text, "{}", platform::raw_stack());
    let _ = std::fs::write(dir.join(format!("panic-{stamp}.txt")), text);
    prune(dir);
}

fn header(what: &str, stamp: &str) -> String {
    let mut text = format!(
        "Aurora {VERSION} - {what}\nora: {stamp}\nprocesso: {}\nattivo da: {} s\n",
        std::process::id(),
        now_ms() / 1000
    );
    if let Some(u) = crate::health::sample() {
        let _ = writeln!(
            text,
            "risorse: {} MB privati, {} MB in memoria, {} maniglie, {} oggetti USER, {} oggetti GDI",
            u.private_mb, u.working_set_mb, u.handles, u.user_objects, u.gdi_objects
        );
    }
    text
}

fn describe(path: &Path, result: &Result<(), String>) -> String {
    match result {
        Ok(()) => format!(
            "{} ({} KB)",
            path.display(),
            std::fs::metadata(path).map(|m| m.len() / 1024).unwrap_or(0)
        ),
        Err(e) => format!("non scritto: {e}"),
    }
}

/// Tiene gli ultimi `KEEP_DUMPS` dump e `KEEP_TEXTS` file di testo (per data di modifica).
pub fn prune(dir: &Path) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    let mut dumps = Vec::new();
    let mut texts = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        let modified = entry.metadata().and_then(|m| m.modified()).ok();
        match path.extension().and_then(|e| e.to_str()) {
            Some("dmp") => dumps.push((modified, path)),
            Some("txt") => texts.push((modified, path)),
            _ => {}
        }
    }
    for (mut list, keep) in [(dumps, KEEP_DUMPS), (texts, KEEP_TEXTS)] {
        // i piu' recenti in testa (a pari data decide il nome, che porta l'ora)
        list.sort_by(|a, b| b.cmp(a));
        for (_, path) in list.into_iter().skip(keep) {
            let _ = std::fs::remove_file(path);
        }
    }
}

#[cfg(windows)]
mod platform {
    use std::{os::windows::io::AsRawHandle, path::Path};

    use windows_sys::Win32::{
        Foundation::{HANDLE, SYSTEMTIME},
        System::{
            Console::{STD_ERROR_HANDLE, SetStdHandle},
            Diagnostics::Debug::{
                EXCEPTION_POINTERS, MINIDUMP_EXCEPTION_INFORMATION, MiniDumpWithHandleData,
                MiniDumpWithIndirectlyReferencedMemory, MiniDumpWithThreadInfo,
                MiniDumpWithUnloadedModules, MiniDumpWriteDump, RtlCaptureStackBackTrace,
                SetUnhandledExceptionFilter,
            },
            ErrorReporting::WerRegisterAppLocalDump,
            LibraryLoader::GetModuleHandleW,
            SystemInformation::GetLocalTime,
            Threading::{GetCurrentProcess, GetCurrentProcessId, GetCurrentThreadId},
        },
    };

    use super::{DUMP_LOCK, MAX_STDERR_BYTES};

    /// Ora locale come `AAAAMMGG-HHMMSS`.
    pub fn timestamp() -> String {
        // SAFETY: `GetLocalTime` riempie la struttura passata, inizializzata a zero.
        let t = unsafe {
            let mut t: SYSTEMTIME = std::mem::zeroed();
            GetLocalTime(&mut t);
            t
        };
        format!(
            "{:04}{:02}{:02}-{:02}{:02}{:02}",
            t.wYear, t.wMonth, t.wDay, t.wHour, t.wMinute, t.wSecond
        )
    }

    /// Minidump del processo: stack e stato di tutti i thread, la memoria a cui puntano, maniglie e moduli (qualche MB).
    pub fn write_minidump(
        path: &Path,
        exception: Option<*const EXCEPTION_POINTERS>,
    ) -> Result<(), String> {
        // si aspetta al massimo 5 s un dump gia' in corso (mai all'infinito: puo' chiamarci il filtro delle eccezioni)
        let mut guard = None;
        for _ in 0..50 {
            if let Ok(g) = DUMP_LOCK.try_lock() {
                guard = Some(g);
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(100));
        }
        let _guard = guard.ok_or_else(|| "un altro dump e' in corso".to_string())?;
        let file = std::fs::File::create(path).map_err(|e| e.to_string())?;
        let kind = MiniDumpWithThreadInfo
            | MiniDumpWithIndirectlyReferencedMemory
            | MiniDumpWithHandleData
            | MiniDumpWithUnloadedModules;
        // SAFETY: lo pseudo-handle del processo e' sempre valido; quello del file resta aperto finche' `file` vive (fino a fine
        // funzione); le informazioni sull'eccezione, se ci sono, vengono dal filtro di Windows e sono valide durante la chiamata.
        let ok = unsafe {
            let info = exception.map(|pointers| MINIDUMP_EXCEPTION_INFORMATION {
                ThreadId: GetCurrentThreadId(),
                ExceptionPointers: pointers.cast_mut(),
                ClientPointers: 0,
            });
            let info_ptr = info.as_ref().map_or(std::ptr::null(), |i| {
                i as *const MINIDUMP_EXCEPTION_INFORMATION
            });
            MiniDumpWriteDump(
                GetCurrentProcess(),
                GetCurrentProcessId(),
                file.as_raw_handle() as HANDLE,
                kind,
                info_ptr,
                std::ptr::null(),
                std::ptr::null(),
            )
        };
        if ok == 0 {
            drop(file);
            let _ = std::fs::remove_file(path);
            return Err(format!(
                "MiniDumpWriteDump: {}",
                std::io::Error::last_os_error()
            ));
        }
        Ok(())
    }

    /// Gli indirizzi dello stack corrente come scarto dall'inizio di aurora-desktop.exe (leggibili con i simboli della stessa build).
    pub fn raw_stack() -> String {
        let mut frames = [std::ptr::null_mut(); 64];
        // SAFETY: il vettore ha spazio per 64 indirizzi, quanti se ne chiedono; `GetModuleHandleW(null)` da' la base dell'eseguibile.
        let (count, base) = unsafe {
            (
                RtlCaptureStackBackTrace(
                    0,
                    frames.len() as u32,
                    frames.as_mut_ptr(),
                    std::ptr::null_mut(),
                ),
                GetModuleHandleW(std::ptr::null()) as usize,
            )
        };
        let mut text = format!("indirizzi (base dell'eseguibile {base:#x}):");
        for frame in frames.iter().take(usize::from(count)) {
            let ip = *frame as usize;
            text.push_str(&format!("\n  {ip:#x} (+{:#x})", ip.wrapping_sub(base)));
        }
        text
    }

    /// Il filtro delle eccezioni non gestite: dump + testo, poi lascia proseguire Windows (che chiude il processo e fa il suo rapporto).
    unsafe extern "system" fn on_unhandled(pointers: *const EXCEPTION_POINTERS) -> i32 {
        let _ = std::panic::catch_unwind(|| {
            let Some(dir) = super::dir() else { return };
            let stamp = timestamp();
            // SAFETY: Windows passa puntatori validi per la durata del filtro; si controlla comunque che non siano nulli.
            let (code, address) = unsafe {
                match pointers.as_ref().and_then(|p| p.ExceptionRecord.as_ref()) {
                    Some(r) => (r.ExceptionCode as u32, r.ExceptionAddress as usize),
                    None => (0, 0),
                }
            };
            let dump = dir.join(format!("crash-{stamp}.dmp"));
            let result = write_minidump(&dump, Some(pointers));
            // SAFETY: base dell'eseguibile, sempre presente.
            let base = unsafe { GetModuleHandleW(std::ptr::null()) } as usize;
            let mut text = super::header("Eccezione non gestita", &stamp);
            text.push_str(&format!(
                "codice: {code:#010x}\nindirizzo: {address:#x} (eseguibile +{:#x})\nthread: {}\nminidump: {}\n",
                address.wrapping_sub(base),
                std::thread::current().name().unwrap_or("senza nome"),
                super::describe(&dump, &result)
            ));
            let _ = std::fs::write(dir.join(format!("crash-{stamp}.txt")), text);
        });
        0 // EXCEPTION_CONTINUE_SEARCH
    }

    pub fn install_exception_filter() {
        // SAFETY: si registra una funzione `extern "system"` statica, valida per tutta la vita del processo.
        unsafe {
            SetUnhandledExceptionFilter(Some(on_unhandled));
        }
    }

    /// L'errore standard va in `stderr.log` (accodato; ricominciato oltre 1 MB): li' Rust scrive il motivo di ogni `abort`.
    pub fn redirect_stderr(path: &Path) {
        if std::fs::metadata(path).is_ok_and(|m| m.len() > MAX_STDERR_BYTES) {
            let _ = std::fs::remove_file(path);
        }
        // il file deve contenere solo errori veri: via le righe di avvio delle versioni vecchie e l'avviso innocuo di WebView2
        // alla chiusura della finestra («Failed to unregister class Chrome_WidgetWin_0»), che non indica nessun guasto
        if let Ok(old) = std::fs::read_to_string(path) {
            let kept: Vec<&str> = old.lines().filter(|l| !harmless(l)).collect();
            let _ = std::fs::write(path, if kept.is_empty() { String::new() } else { kept.join("
") + "
" });
        }
        let Ok(file) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
        else {
            return;
        };
        let handle = file.as_raw_handle();
        // il file resta aperto per tutta la vita del processo: Windows usa la maniglia a ogni scrittura su stderr
        std::mem::forget(file);
        // SAFETY: la maniglia e' valida e non verra' mai chiusa (il `File` e' stato dimenticato).
        unsafe {
            SetStdHandle(STD_ERROR_HANDLE, handle as HANDLE);
        }
    }

    /// Righe che non sono errori: l'intestazione di avvio scritta dalle versioni fino alla 0.11.92 e l'avviso di WebView2 alla chiusura.
    pub(crate) fn harmless(line: &str) -> bool {
        let line = line.trim();
        line.is_empty() || (line.starts_with("--- Aurora ") && line.ends_with(" ---")) || line.contains("Failed to unregister class Chrome_WidgetWin")
    }

    /// Chiede a Windows Error Reporting di tenere un dump locale. Funziona solo per le app impacchettate (MSIX): per un programma
    /// installato normalmente Windows risponde con un errore, che si annota nel log. Non richiede diritti di amministratore.
    pub fn register_wer() {
        let relative: Vec<u16> = "crash".encode_utf16().chain(std::iter::once(0)).collect();
        // SAFETY: stringa UTF-16 terminata da zero, viva durante la chiamata.
        let hr = unsafe { WerRegisterAppLocalDump(relative.as_ptr()) };
        if hr == 0 {
            tracing::info!("Windows Error Reporting terra' un dump locale dei crash");
        } else {
            tracing::debug!(
                "dump locale di Windows Error Reporting non disponibile (HRESULT {:#010x}): resta il guardiano interno",
                hr as u32
            );
        }
    }
}

#[cfg(not(windows))]
mod platform {
    use std::path::Path;

    pub fn timestamp() -> String {
        format!(
            "{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0)
        )
    }
    pub fn write_minidump(_path: &Path, _exception: Option<()>) -> Result<(), String> {
        Err("minidump solo su Windows".into())
    }
    pub fn raw_stack() -> String {
        String::new()
    }
    pub fn install_exception_filter() {}
    pub fn redirect_stderr(_path: &Path) {}
    pub fn register_wer() {}
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_block_is_reported_once_and_again_after_recovery() {
        let mut w = Watch::default();
        assert!(
            w.check(10_000, &[("runtime", 9_000), ("finestra", 9_500)], 20_000)
                .is_none()
        );
        let stale = w.check(40_000, &[("runtime", 39_000), ("finestra", 10_000)], 20_000);
        assert_eq!(stale, Some(vec![("finestra", 10_000)]));
        // lo stesso blocco non produce un secondo dump
        assert!(
            w.check(60_000, &[("runtime", 59_000), ("finestra", 10_000)], 20_000)
                .is_none()
        );
        // ripartito e di nuovo fermo: un blocco nuovo
        assert!(
            w.check(62_000, &[("runtime", 61_000), ("finestra", 61_500)], 20_000)
                .is_none()
        );
        assert!(
            w.check(90_000, &[("runtime", 65_000), ("finestra", 89_000)], 20_000)
                .is_some()
        );
    }

    fn scratch(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("aurora-crash-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let _ = std::fs::create_dir_all(&dir);
        dir
    }

    #[test]
    fn pruning_keeps_the_newest_dumps_and_texts() {
        let dir = scratch("prune");
        for i in 0..14 {
            std::fs::write(
                dir.join(format!("hang-2026110{}-0000{i:02}.dmp", i % 2)),
                b"x",
            )
            .unwrap();
        }
        for i in 0..55 {
            std::fs::write(dir.join(format!("panic-20261101-00{i:04}.txt")), b"x").unwrap();
        }
        std::fs::write(dir.join("stderr.log"), b"x").unwrap();
        prune(&dir);
        let count = |ext: &str| {
            std::fs::read_dir(&dir)
                .unwrap()
                .flatten()
                .filter(|e| e.path().extension().is_some_and(|x| x == ext))
                .count()
        };
        assert_eq!(count("dmp"), KEEP_DUMPS);
        assert_eq!(count("txt"), KEEP_TEXTS);
        assert!(dir.join("stderr.log").exists());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[cfg(windows)]
    #[test]
    fn a_minidump_of_the_running_process_is_written() {
        let dir = scratch("dump");
        let path = dir.join("hang-prova.dmp");
        // un thread fermo in attesa, come in un blocco vero: il dump deve comunque riuscire
        let (tx, rx) = std::sync::mpsc::channel::<()>();
        let waiter = std::thread::spawn(move || {
            let _ = rx.recv();
        });
        platform::write_minidump(&path, None).unwrap();
        drop(tx);
        waiter.join().unwrap();
        let bytes = std::fs::read(&path).unwrap();
        assert_eq!(&bytes[..4], b"MDMP");
        assert!(bytes.len() > 10_000);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[cfg(windows)]
    #[test]
    fn a_hang_report_writes_dump_and_text() {
        let dir = scratch("hang");
        let _ = DIR.set(dir.clone());
        let Some(dir) = super::dir().map(Path::to_path_buf) else {
            return;
        };
        report_hang(
            &[("finestra", 0)],
            &[("runtime", now_ms()), ("finestra", 0)],
        );
        let names: Vec<String> = std::fs::read_dir(&dir)
            .unwrap()
            .flatten()
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .collect();
        assert!(
            names
                .iter()
                .any(|n| n.starts_with("hang-") && n.ends_with(".dmp")),
            "{names:?}"
        );
        let text = names
            .iter()
            .find(|n| n.ends_with(".txt"))
            .map(|n| std::fs::read_to_string(dir.join(n)).unwrap())
            .unwrap_or_default();
        assert!(
            text.contains("fermi: finestra") && text.contains(VERSION),
            "{text}"
        );
    }

    #[test]
    fn timestamps_are_sortable() {
        let t = platform::timestamp();
        if cfg!(windows) {
            assert_eq!(t.len(), 15);
            assert_eq!(&t[8..9], "-");
        }
    }
}
