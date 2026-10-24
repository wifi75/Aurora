//! Aurora per Windows: il core gira dentro questo programma e la finestra mostra la sua pagina (la stessa che si apre dal telefono).
//! Il vassoio tiene Aurora a portata di mano, parte con Windows e si aggiorna da sola quando esce una versione nuova.

mod installer;
mod links;
mod settings;
mod window_state;

use std::{
    error::Error,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

use aurora_core::Event;
use tauri::{
    menu::{CheckMenuItem, Menu, MenuItem, PredefinedMenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    webview::NewWindowResponse,
    AppHandle, Manager, PhysicalPosition, PhysicalSize, RunEvent, WebviewUrl, WebviewWindow, WebviewWindowBuilder, WindowEvent, Wry,
};
use tauri_plugin_autostart::{MacosLauncher, ManagerExt as _};
use tauri_plugin_notification::NotificationExt;
use tokio::sync::{broadcast::error::RecvError, oneshot};

/// Il core in funzione: per spegnerlo e per chiedergli cose (aggiornamenti, stato).
struct Core {
    shutdown: Mutex<Option<oneshot::Sender<()>>>,
    /// Assente se il core non e' partito (la finestra dice perche').
    state: Option<remote_api::AppState>,
    installer: Option<Arc<installer::DesktopInstaller>>,
    data_dir: PathBuf,
}

/// La voce del vassoio che propone l'aggiornamento: spenta finche' non ce n'e' uno.
struct UpdateItem(MenuItem<Wry>);

static LAST_WINDOW_SAVE: Mutex<Option<Instant>> = Mutex::new(None);

const FIRST_RUN_MARKER: &str = "desktop-primo-avvio";
const MAX_LOG_BYTES: u64 = 5 * 1024 * 1024;

pub fn run() {
    // all'accesso di Windows Aurora parte nel vassoio, senza aprire la finestra
    let hidden = std::env::args().any(|a| a == "--nascosto");
    let built = tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| show_main(app)))
        .plugin(tauri_plugin_autostart::init(MacosLauncher::LaunchAgent, Some(vec!["--nascosto"])))
        .plugin(tauri_plugin_notification::init())
        .setup(move |app| start(app, hidden))
        .on_window_event(|window, event| match event {
            // chiudere la finestra la nasconde: Aurora resta nel vassoio, col core acceso
            WindowEvent::CloseRequested { api, .. } => {
                remember_window(window);
                api.prevent_close();
                let _ = window.hide();
            }
            // dimensione e posizione si ricordano per il prossimo avvio (senza scrivere a ogni pixel)
            WindowEvent::Resized(_) | WindowEvent::Moved(_) => {
                if let Ok(mut last) = LAST_WINDOW_SAVE.lock() {
                    if last.is_none_or(|t| t.elapsed() > Duration::from_millis(600)) {
                        *last = Some(Instant::now());
                        remember_window(window);
                    }
                }
            }
            _ => {}
        })
        .build(tauri::generate_context!());
    match built {
        Ok(app) => app.run(|handle, event| {
            if let RunEvent::Exit = event {
                if let Some(window) = handle.get_webview_window("main") {
                    remember_window(&window.as_ref().window());
                }
                stop_core(handle);
            }
        }),
        Err(e) => eprintln!("Aurora non parte: {e}"),
    }
}

fn stop_core(app: &AppHandle) {
    if let Some(core) = app.try_state::<Core>() {
        if let Some(tx) = core.shutdown.lock().ok().and_then(|mut s| s.take()) {
            let _ = tx.send(());
            // il tempo di chiudere porte e archivi
            std::thread::sleep(Duration::from_millis(400));
        }
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
        let _ = tracing_subscriber::fmt()
            .with_env_filter(tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")))
            .with_ansi(false)
            .with_writer(Mutex::new(file))
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

fn start(app: &mut tauri::App, hidden: bool) -> Result<(), Box<dyn Error>> {
    let handle = app.handle().clone();
    // di norma %APPDATA%\Aurora; `AURORA_DATA_DIR` nell'ambiente sceglie un'altra cartella (per le prove)
    let data_dir = match std::env::var_os("AURORA_DATA_DIR") {
        Some(dir) => PathBuf::from(dir),
        None => handle.path().data_dir()?.join("Aurora"),
    };
    std::fs::create_dir_all(&data_dir)?;
    init_logging(&data_dir);
    let env_path = data_dir.join("aurora.env");
    if !env_path.exists() {
        let _ = std::fs::write(&env_path, settings::sample_env_file());
    }
    let places = settings::Places {
        local_app_data: std::env::var_os("LOCALAPPDATA").map(PathBuf::from),
        documents: handle.path().document_dir().ok(),
        jtdx_roots: settings::Places::standard_jtdx_roots(),
    };
    let values = settings::build(&places, &data_dir, &ui_dir(&handle), std::fs::read_to_string(&env_path).ok().as_deref(), std::env::vars());
    let installer = installer::DesktopInstaller::new(handle.clone());

    let started = settings::config_from(&values).map_err(|e| e.to_string()).and_then(|mut config| {
        config.update_installer = Some(aurora::UpdateInstallerHandle(installer.clone()));
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
            app.manage(Core { shutdown: Mutex::new(Some(tx)), state: Some(server.state.clone()), installer: Some(installer), data_dir: data_dir.clone() });
            tauri::async_runtime::spawn(async move {
                server.join().await;
            });
            // `AURORA_AUTOSTART=0` (nel file o nell'ambiente) tiene Aurora fuori dai programmi che partono con Windows
            if values.get("AURORA_AUTOSTART").map(String::as_str) != Some("0") {
                first_run_autostart(&handle, &data_dir);
            }
            let update_item = build_tray(&handle)?;
            app.manage(UpdateItem(update_item));
            open_window(&handle, &page, !hidden)?;
            // Monitora (l'ascolto) parte acceso: `AURORA_MONITOR_ON_START=0` lo lascia com'e'
            if values.get("AURORA_MONITOR_ON_START").map(String::as_str) != Some("0") {
                if let Some(state) = core_state(&handle) {
                    monitor_on_start(state);
                }
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
            app.manage(Core { shutdown: Mutex::new(None), state: None, installer: None, data_dir });
            let _ = build_tray(&handle);
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
        // i link a siti (note di versione, guida) si aprono nel browser di Windows, non in una finestra nuova
        .on_new_window(|url, _| {
            links::open_in_browser(url.as_str());
            NewWindowResponse::Deny
        })
        .build()?;
    restore_window(app, &window);
    if visible {
        window.show()?;
    }
    Ok(())
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
                    Some((_, false)) => {
                        if rig.set_control("monitor", Some(true)).is_ok() {
                            tracing::info!("Monitora acceso all'avvio");
                            return;
                        }
                    }
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
    if window.label() != "main" || window.is_minimized().unwrap_or(false) {
        return;
    }
    let Some(core) = window.app_handle().try_state::<Core>() else { return };
    let previous = window_state::load(&core.data_dir);
    let state = if window.is_maximized().unwrap_or(false) {
        match previous {
            Some(p) => window_state::WindowState { maximized: true, ..p },
            None => return,
        }
    } else {
        let (Ok(size), Ok(pos)) = (window.inner_size(), window.outer_position()) else { return };
        window_state::WindowState { width: size.width, height: size.height, x: pos.x, y: pos.y, maximized: false }
    };
    if previous != Some(state) {
        if let Err(e) = window_state::save(&core.data_dir, &state) {
            tracing::warn!("posizione della finestra non salvata: {e}");
        }
    }
}

fn open_window_app(app: &AppHandle, path: &str) -> Result<(), Box<dyn Error>> {
    WebviewWindowBuilder::new(app, "main", WebviewUrl::App(path.into())).title("Aurora").inner_size(720.0, 420.0).build()?;
    Ok(())
}

/// Al primo avvio Aurora si mette tra i programmi che partono con Windows; poi decide l'utente dal vassoio.
fn first_run_autostart(app: &AppHandle, data_dir: &Path) {
    let marker = data_dir.join(FIRST_RUN_MARKER);
    if marker.exists() {
        return;
    }
    if let Err(e) = app.autolaunch().enable() {
        tracing::warn!("avvio con Windows non attivato: {e}");
    }
    let _ = std::fs::write(marker, "fatto\n");
}

fn notify(app: &AppHandle, title: &str, body: &str) {
    if let Err(e) = app.notification().builder().title(title).body(body).show() {
        tracing::warn!("notifica non mostrata: {e}");
    }
}

/// E' uscita una versione nuova: avviso di Windows e la voce del vassoio diventa «Installa Aurora X».
fn announce_update(app: &AppHandle, version: &str) {
    if let Some(item) = app.try_state::<UpdateItem>() {
        let _ = item.0.set_text(format!("Installa Aurora {version}"));
        let _ = item.0.set_enabled(true);
    }
    notify(app, &format!("Aurora {version} disponibile"), "Dal vassoio (icona di Aurora) scegli «Installa» per aggiornare.");
}

fn build_tray(app: &AppHandle) -> Result<MenuItem<Wry>, Box<dyn Error>> {
    let open = MenuItem::with_id(app, "open", "Apri Aurora", true, None::<&str>)?;
    let update = MenuItem::with_id(app, "install", "Nessun aggiornamento", false, None::<&str>)?;
    let check = MenuItem::with_id(app, "check", "Cerca aggiornamenti", true, None::<&str>)?;
    let autostart = CheckMenuItem::with_id(app, "autostart", "Avvia con Windows", true, app.autolaunch().is_enabled().unwrap_or(false), None::<&str>)?;
    let data = MenuItem::with_id(app, "data", "Apri la cartella dei dati", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Esci da Aurora", true, None::<&str>)?;
    let menu = Menu::with_items(
        app,
        &[&open, &PredefinedMenuItem::separator(app)?, &update, &check, &autostart, &data, &PredefinedMenuItem::separator(app)?, &quit],
    )?;
    let icon = app.default_window_icon().cloned().ok_or("icona del programma mancante")?;
    TrayIconBuilder::with_id("aurora")
        .icon(icon)
        .tooltip("Aurora")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id().as_ref() {
            "open" => show_main(app),
            "install" => install_from_tray(app),
            "check" => check_from_tray(app),
            "autostart" => toggle_autostart(app),
            "data" => open_data_folder(app),
            "quit" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click { button: MouseButton::Left, button_state: MouseButtonState::Up, .. } = event {
                show_main(tray.app_handle());
            }
        })
        .build(app)?;
    Ok(update)
}

fn toggle_autostart(app: &AppHandle) {
    let launcher = app.autolaunch();
    let result = if launcher.is_enabled().unwrap_or(false) { launcher.disable() } else { launcher.enable() };
    if let Err(e) = result {
        notify(app, "Avvio con Windows", &format!("Non e' stato possibile cambiarlo: {e}"));
    }
}

fn open_data_folder(app: &AppHandle) {
    if let Some(core) = app.try_state::<Core>() {
        let _ = std::process::Command::new("explorer").arg(&core.data_dir).spawn();
    }
}

fn check_from_tray(app: &AppHandle) {
    let Some(core) = app.try_state::<Core>() else { return };
    let Some(updates) = core.state.as_ref().and_then(|s| s.updates.clone()) else { return };
    let app = app.clone();
    std::thread::spawn(move || {
        let status = updates.check();
        match (status.available, status.error) {
            (Some(release), _) => announce_update(&app, &release.version),
            (None, Some(error)) => notify(&app, "Aggiornamenti", &format!("Controllo non riuscito: {error}")),
            (None, None) => notify(&app, "Aggiornamenti", "Aurora e' gia' all'ultima versione."),
        }
    });
}

fn install_from_tray(app: &AppHandle) {
    let Some(core) = app.try_state::<Core>() else { return };
    let Some(state) = core.state.as_ref() else { return };
    let Some(updates) = state.updates.clone() else { return };
    if state.current_status().is_some_and(|s| s.transmitting) {
        notify(app, "Aggiornamento", "Si sta trasmettendo: riprova a fine trasmissione.");
        return;
    }
    let Some(installer) = core.installer.clone() else { return };
    let app = app.clone();
    notify(&app, "Aggiornamento", "Scarico e controllo la versione nuova…");
    std::thread::spawn(move || {
        use remote_api::UpdateInstaller as _;
        if let Err(e) = installer.install(updates) {
            notify(&app, "Aggiornamento non riuscito", &e);
        }
    });
}
