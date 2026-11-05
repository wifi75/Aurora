//! Aurora per Windows: il core gira dentro questo programma e la finestra mostra la sua pagina (la stessa che si apre dal telefono).
//! Il vassoio tiene Aurora a portata di mano, parte con Windows e si aggiorna da sola quando esce una versione nuova.

mod crash;
mod health;
mod installer;
mod links;
mod save_dialog;
mod settings;
mod window_state;

use std::{
    error::Error,
    path::{Path, PathBuf},
    sync::Mutex,
    time::{Duration, Instant},
};

use aurora_core::Event;
use tauri::{
    webview::NewWindowResponse, AppHandle, Manager, PhysicalPosition, PhysicalSize, RunEvent, WebviewUrl, WebviewWindow, WebviewWindowBuilder, WindowEvent,
};
use tauri_plugin_autostart::{MacosLauncher, ManagerExt as _};
use tauri_plugin_notification::NotificationExt;
use tokio::sync::{broadcast::error::RecvError, oneshot};

/// Il core in funzione: per spegnerlo e per chiedergli cose (aggiornamenti, stato).
struct Core {
    shutdown: Mutex<Option<oneshot::Sender<()>>>,
    /// Assente se il core non e' partito (la finestra dice perche').
    state: Option<remote_api::AppState>,
    data_dir: PathBuf,
}

/// L'ultimo salvataggio di posizione e misura, per ogni finestra (la principale e quelle a parte).
static LAST_WINDOW_SAVE: std::sync::LazyLock<Mutex<std::collections::HashMap<String, Instant>>> = std::sync::LazyLock::new(|| Mutex::new(std::collections::HashMap::new()));

const MAX_LOG_BYTES: u64 = 5 * 1024 * 1024;

pub fn run() {
    let built = tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| show_main(app)))
        .plugin(tauri_plugin_autostart::init(MacosLauncher::LaunchAgent, Some(vec!["--nascosto"])))
        .plugin(tauri_plugin_notification::init())
        .setup(move |app| start(app))
        .on_window_event(|window, event| match event {
            // come una normale applicazione: la X chiude la finestra e con lei tutto (il core si ferma all'uscita)
            WindowEvent::CloseRequested { .. } => {
                remember_window(window);
            }
            // dimensione e posizione si ricordano per il prossimo avvio (senza scrivere a ogni pixel)
            WindowEvent::Resized(_) | WindowEvent::Moved(_) => {
                if let Ok(mut last) = LAST_WINDOW_SAVE.lock()
                    && last.get(window.label()).is_none_or(|t| t.elapsed() > Duration::from_millis(600))
                {
                    last.insert(window.label().to_string(), Instant::now());
                    remember_window(window);
                }
            }
            _ => {}
        })
        .build(tauri::generate_context!());
    match built {
        Ok(app) => app.run(|handle, event| {
            if let RunEvent::Exit = event {
                // la principale e le finestre a parte: ognuna si ricorda dove stava
                for window in handle.webview_windows().values() {
                    remember_window(&window.as_ref().window());
                }
                stop_core(handle);
            }
        }),
        Err(e) => eprintln!("Aurora non parte: {e}"),
    }
}

fn stop_core(app: &AppHandle) {
    if let Some(core) = app.try_state::<Core>()
        && let Some(tx) = core.shutdown.lock().ok().and_then(|mut s| s.take())
    {
        let _ = tx.send(());
        // il tempo di chiudere porte e archivi
        std::thread::sleep(Duration::from_millis(400));
    }
}

fn init_logging(data_dir: &Path) {
    let dir = data_dir.join("logs");
    if std::fs::create_dir_all(&dir).is_err() {
        return;
    }
    let path = dir.join("aurora.log");
    if std::fs::metadata(&path).is_ok_and(|m| m.len() > MAX_LOG_BYTES) {
        let _ = std::fs::rename(&path, dir.join("aurora.vecchio.log"));
    }
    if let Ok(file) = std::fs::OpenOptions::new().create(true).append(true).open(path) {
        use tracing_subscriber::{layer::SubscriberExt as _, util::SubscriberInitExt as _};
        // accanto al file, lo strato dei rapporti degli errori (non tiene niente finche' l'operatore non li accende)
        let _ = tracing_subscriber::registry()
            .with(tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")))
            .with(tracing_subscriber::fmt::layer().with_ansi(false).with_writer(Mutex::new(file)))
            .with(aurora::rapporti::Strato)
            .try_init();
    }
}

/// La cartella dell'interfaccia: quella installata col programma; in sviluppo, quella compilata del repository.
fn ui_dir(app: &AppHandle) -> PathBuf {
    let installed = app.path().resource_dir().map(|d| d.join("dist")).unwrap_or_default();
    if installed.join("index.html").is_file() {
        return installed;
    }
    let dev = PathBuf::from("../../app/dist");
    if dev.join("index.html").is_file() { dev } else { installed }
}

fn start(app: &mut tauri::App) -> Result<(), Box<dyn Error>> {
    let handle = app.handle().clone();
    // di norma %APPDATA%\Aurora; `AURORA_DATA_DIR` nell'ambiente sceglie un'altra cartella (per le prove)
    let data_dir = match std::env::var_os("AURORA_DATA_DIR") {
        Some(dir) => PathBuf::from(dir),
        None => handle.path().data_dir()?.join("Aurora"),
    };
    std::fs::create_dir_all(&data_dir)?;
    crash::init(&data_dir);
    init_logging(&data_dir);
    health::install_panic_hook();
    crash::spawn_watchdog(handle.clone());
    health::spawn_monitor();
    let env_path = data_dir.join("aurora.env");
    if !env_path.exists() {
        let _ = std::fs::write(&env_path, settings::sample_env_file());
    }
    let places = settings::Places {
        documents: handle.path().document_dir().ok(),
        bundled_cty: handle.path().resource_dir().ok().map(|d| d.join("cty").join("cty.dat")).filter(|p| p.is_file()).or_else(|| Some(PathBuf::from("cty/cty.dat"))),
    };
    let values = settings::build(&places, &data_dir, &ui_dir(&handle), std::fs::read_to_string(&env_path).ok().as_deref(), std::env::vars());
    let installer = installer::DesktopInstaller::new(handle.clone());

    let started = settings::config_from(&values).map_err(|e| e.to_string()).and_then(|mut config| {
        config.update_installer = Some(aurora::UpdateInstallerHandle(installer.clone()));
        config.save_dialog = Some(aurora::SaveDialogHandle(save_dialog::dialog(handle.clone())));
        config.crash_folder = Some(aurora::CrashFolderHandle(crash::folder_opener()));
        let (tx, rx) = oneshot::channel::<()>();
        let server = tauri::async_runtime::block_on(aurora::run(config, async move {
            let _ = rx.await;
        }))
        .map_err(|e| e.to_string())?;
        Ok((server, tx))
    });

    match started {
        Ok((server, tx)) => {
            tracing::info!("Aurora {} in ascolto su http://{}", env!("CARGO_PKG_VERSION"), server.addr);
            let page = format!("http://127.0.0.1:{}/", server.addr.port());
            let mut events = server.state.bus.subscribe_events();
            app.manage(Core { shutdown: Mutex::new(Some(tx)), state: Some(server.state.clone()), data_dir: data_dir.clone() });
            tauri::async_runtime::spawn(async move {
                server.join().await;
            });
            // niente programma residente: l'avvio con Windows e' spento, salvo `AURORA_AUTOSTART=1`
            apply_autostart_choice(&handle, values.get("AURORA_AUTOSTART").map(String::as_str) == Some("1"));
            open_window(&handle, &page, true)?;
            // Monitora (l'ascolto) parte acceso: `AURORA_MONITOR_ON_START=0` lo lascia com'e'
            if values.get("AURORA_MONITOR_ON_START").map(String::as_str) != Some("0")
                && let Some(state) = core_state(&handle)
            {
                monitor_on_start(state);
            }
            let watcher = handle.clone();
            tauri::async_runtime::spawn(async move {
                loop {
                    match events.recv().await {
                        Ok(Event::UpdateAvailable(notice)) => announce_update(&watcher, &notice.version),
                        Ok(_) | Err(RecvError::Lagged(_)) => {}
                        Err(RecvError::Closed) => break,
                    }
                }
            });
        }
        Err(message) => {
            tracing::error!("avvio fallito: {message}");
            // la finestra dice cosa non va e il vassoio permette di aprire il file da correggere o uscire
            open_window_app(&handle, &format!("index.html?errore={}", percent_encode(&message)))?;
            app.manage(Core { shutdown: Mutex::new(None), state: None, data_dir });
        }
    }
    Ok(())
}

fn percent_encode(text: &str) -> String {
    text.bytes()
        .map(|b| if b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.' | b'~') { (b as char).to_string() } else { format!("%{b:02X}") })
        .collect()
}

fn show_main(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
    }
}

fn open_window(app: &AppHandle, url: &str, visible: bool) -> Result<(), Box<dyn Error>> {
    let parsed: tauri::Url = url.parse()?;
    // si crea nascosta, si rimette la dimensione dell'ultima volta e solo dopo si mostra (niente salti)
    let window = WebviewWindowBuilder::new(app, "main", WebviewUrl::External(parsed))
        .title("Aurora")
        .inner_size(1360.0, 860.0)
        .min_inner_size(f64::from(window_state::MIN_WIDTH), f64::from(window_state::MIN_HEIGHT))
        .visible(false)
        // gli argomenti di base di Tauri piu' il silenzio dei messaggi interni di WebView2: alla chiusura scriveva sempre in stderr.log
        // «Failed to unregister class Chrome_WidgetWin_0. Error = 1412», innocuo ma scambiato per un errore di Aurora
        .additional_browser_args("--disable-features=msWebOOUI,msPdfOOUI,msSmartScreenProtection --disable-logging --log-level=3 --autoplay-policy=no-user-gesture-required")
        // le pagine da sole (`#/finestra/...`: mappa, propagazione) si aprono in una finestra di Aurora da portare sul secondo schermo;
        // i link a siti (note di versione, guida) si aprono nel browser di Windows, non in una finestra nuova
        .on_new_window({
            let app = app.clone();
            move |url, _| {
                if let Some(page) = solo_page(url.as_str()) {
                    let app = app.clone();
                    let url = url.clone();
                    let _ = app.clone().run_on_main_thread(move || open_solo_window(&app, &page, url));
                } else {
                    links::open_in_browser(url.as_str());
                }
                NewWindowResponse::Deny
            }
        })
        .build()?;
    restore_window(app, &window);
    if visible {
        window.show()?;
    }
    Ok(())
}

/// La pagina da sola chiesta da un link `#/finestra/<pagina>` dello stesso Aurora (solo lettere minuscole: e' anche l'etichetta della finestra).
fn solo_page(url: &str) -> Option<String> {
    let page = url.split_once("#/finestra/")?.1;
    let local = url.starts_with("http://127.0.0.1:") || url.starts_with("http://localhost:");
    (local && !page.is_empty() && page.len() <= 20 && page.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit())).then(|| page.to_string())
}

/// Apre (o porta davanti, se c'e' gia') la finestra di una pagina da sola: ridimensionabile, sul secondo schermo se c'e'.
fn open_solo_window(app: &AppHandle, page: &str, url: tauri::Url) {
    let label = format!("finestra-{page}");
    if let Some(window) = app.get_webview_window(&label) {
        let _ = window.unminimize();
        let _ = window.show();
        let _ = window.set_focus();
        return;
    }
    // il titolo vero lo mette la pagina (gli elementi che contiene)
    let title = "Aurora";
    // il display della radio da solo: finestra senza cornice (si vede solo il display); le altre finestre a parte hanno la cornice di Windows
    let display_only = url.query_pairs().any(|(k, v)| k == "items" && v == "display") && !url.query_pairs().any(|(k, v)| k == "mode" && v == "grid");
    let built = WebviewWindowBuilder::new(app, &label, WebviewUrl::External(url))
        .title(title)
        .decorations(!display_only)
        .shadow(true)
        .inner_size(if display_only { 660.0 } else { 1000.0 }, if display_only { 110.0 } else { 700.0 })
        .min_inner_size(if display_only { 160.0 } else { 360.0 }, if display_only { 40.0 } else { 260.0 })
        .resizable(true)
        .visible(false)
        .additional_browser_args("--disable-features=msWebOOUI,msPdfOOUI,msSmartScreenProtection --disable-logging --log-level=3 --autoplay-policy=no-user-gesture-required")
        .on_new_window(|url, _| {
            links::open_in_browser(url.as_str());
            NewWindowResponse::Deny
        })
        // la pagina non ha piu' niente da mostrare (tutto rimesso nella finestra principale): chiede `/__chiudi-finestra` e si chiude
        .on_navigation({
            let app = app.clone();
            let label = label.clone();
            move |url| {
                if url.path() != "/__chiudi-finestra" {
                    return true;
                }
                let app2 = app.clone();
                let label = label.clone();
                let _ = app.run_on_main_thread(move || {
                    if let Some(window) = app2.get_webview_window(&label) {
                        let _ = window.close();
                    }
                });
                false
            }
        })
        .build();
    let window = match built {
        Ok(w) => w,
        Err(e) => {
            tracing::warn!("finestra {page} non aperta: {e}");
            return;
        }
    };
    // dove stava l'ultima volta (posizione, misura, massimizzata), se quello schermo c'e' ancora
    if let Some(core) = app.try_state::<Core>()
        && let Some(state) = window_state::load_for(&core.data_dir, &label)
    {
        let monitors: Vec<window_state::Monitor> = window
            .available_monitors()
            .map(|all| all.iter().map(|m| (m.position().x, m.position().y, m.size().width, m.size().height)).collect())
            .unwrap_or_default();
        if window_state::is_usable_with(&state, &monitors, window_state::SOLO_MIN_WIDTH, window_state::SOLO_MIN_HEIGHT) {
            let _ = window.set_size(PhysicalSize::new(state.width, state.height));
            let _ = window.set_position(PhysicalPosition::new(state.x, state.y));
            if state.maximized {
                let _ = window.maximize();
            }
            let _ = window.show();
            let _ = window.set_focus();
            return;
        }
    }
    // la prima volta: sul secondo schermo, il primo diverso da quello della finestra principale, con un po' di margine dall'angolo
    let main_monitor = app.get_webview_window("main").and_then(|m| m.current_monitor().ok().flatten()).map(|m| *m.position());
    if let Ok(monitors) = window.available_monitors() {
        if let Some(other) = monitors.iter().find(|m| Some(*m.position()) != main_monitor) {
            let p = other.position();
            let _ = window.set_position(PhysicalPosition::new(p.x + 60, p.y + 60));
        }
    }
    let _ = window.show();
    let _ = window.set_focus();
}

fn core_state(app: &AppHandle) -> Option<remote_api::AppState> {
    app.try_state::<Core>().and_then(|core| core.state.clone())
}

/// All'avvio porta Monitora su acceso, appena il motore risponde (puo' partire dopo Aurora: si riprova per dieci minuti).
fn monitor_on_start(state: remote_api::AppState) {
    std::thread::spawn(move || {
        let Some(rig) = state.rig.clone() else { return };
        for _ in 0..120 {
            if let Ok((list, _)) = rig.controls_and_tx() {
                match list.iter().find(|(id, _)| id == "monitor") {
                    Some((_, true)) => return,
                    Some((_, false)) if rig.set_control("monitor", Some(true)).is_ok() => {
                        tracing::info!("Monitora acceso all'avvio");
                        return;
                    }
                    Some((_, false)) => {}
                    None => {}
                }
            }
            std::thread::sleep(Duration::from_secs(5));
        }
    });
}

fn restore_window(app: &AppHandle, window: &WebviewWindow) {
    let Some(core) = app.try_state::<Core>() else { return };
    let Some(state) = window_state::load(&core.data_dir) else { return };
    let monitors: Vec<window_state::Monitor> = window
        .available_monitors()
        .map(|all| all.iter().map(|m| (m.position().x, m.position().y, m.size().width, m.size().height)).collect())
        .unwrap_or_default();
    if !window_state::is_usable(&state, &monitors) {
        return;
    }
    let _ = window.set_size(PhysicalSize::new(state.width, state.height));
    let _ = window.set_position(PhysicalPosition::new(state.x, state.y));
    if state.maximized {
        let _ = window.maximize();
    }
}

/// Salva dimensione e posizione della finestra principale. Da massimizzata resta la misura di prima e si segna solo che lo era.
fn remember_window(window: &tauri::Window) {
    let label = window.label().to_string();
    if !(label == "main" || label.starts_with("finestra-")) || window.is_minimized().unwrap_or(false) {
        return;
    }
    let Some(core) = window.app_handle().try_state::<Core>() else { return };
    let previous = window_state::load_for(&core.data_dir, &label);
    let state = if window.is_maximized().unwrap_or(false) {
        match previous {
            Some(p) => window_state::WindowState { maximized: true, ..p },
            None => return,
        }
    } else {
        let (Ok(size), Ok(pos)) = (window.inner_size(), window.outer_position()) else { return };
        window_state::WindowState { width: size.width, height: size.height, x: pos.x, y: pos.y, maximized: false }
    };
    if previous != Some(state)
        && let Err(e) = window_state::save_for(&core.data_dir, &label, &state)
    {
        tracing::warn!("posizione della finestra non salvata: {e}");
    }
}

fn open_window_app(app: &AppHandle, path: &str) -> Result<(), Box<dyn Error>> {
    WebviewWindowBuilder::new(app, "main", WebviewUrl::App(path.into())).title("Aurora").inner_size(720.0, 420.0).build()?;
    Ok(())
}

/// Aurora non resta residente: l'avvio con Windows e' spento (e si toglie se una versione vecchia l'aveva acceso). Solo
/// `AURORA_AUTOSTART=1` nel file delle impostazioni o nell'ambiente lo accende.
fn apply_autostart_choice(app: &AppHandle, wanted: bool) {
    let launcher = app.autolaunch();
    let enabled = launcher.is_enabled().unwrap_or(false);
    let result = match (wanted, enabled) {
        (true, false) => launcher.enable(),
        (false, true) => launcher.disable(),
        _ => Ok(()),
    };
    if let Err(e) = result {
        tracing::warn!("avvio con Windows non aggiornato: {e}");
    }
}

fn notify(app: &AppHandle, title: &str, body: &str) {
    if let Err(e) = app.notification().builder().title(title).body(body).show() {
        tracing::warn!("notifica non mostrata: {e}");
    }
}

/// E' uscita una versione nuova: avviso di Windows; l'installazione si avvia dalla barra in alto nella pagina («Installa ora»).
fn announce_update(app: &AppHandle, version: &str) {
    notify(app, &format!("Aurora {version} disponibile"), "Apri Aurora e premi «Installa ora» nella barra in alto per aggiornare.");
}
