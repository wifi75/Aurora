//! L'agente di Aurora nel tray: lo stesso programma avviato con `--agente`, senza finestra e senza motore. Resta vicino all'orologio
//! anche con Aurora chiusa, apre Aurora con un clic e mostra in basso a destra la «cartolina» (con il suono) per le versioni nuove
//! e per i messaggi dell'autore. Controlla all'avvio e poi ogni ora. Con Aurora aperta non ripete gli avvisi: li mostra gia' Aurora.

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Manager, PhysicalPosition, RunEvent, WebviewUrl, WebviewWindowBuilder};

const REPO_API: &str = "https://api.github.com/repos/wifi75/Aurora/releases/latest";
const MESSAGES: &str = "https://aurora-rapporti.tizianocassone.workers.dev/v1/messaggi";
const EVERY: Duration = Duration::from_secs(60 * 60);
const VERSION: &str = env!("CARGO_PKG_VERSION");

static CHECKING: AtomicBool = AtomicBool::new(false);

/// Le scelte della pagina «Agente nel tray» (Impostazioni), salvate da Aurora in `ui-settings.json`.
#[derive(serde::Deserialize, Clone)]
#[serde(default)]
pub struct Prefs {
    pub enabled: bool,
    show: std::collections::HashMap<String, String>,
    seconds: u32,
    #[serde(rename = "quietOn")]
    quiet_on: bool,
    #[serde(rename = "quietFrom")]
    quiet_from: String,
    #[serde(rename = "quietTo")]
    quiet_to: String,
}

impl Default for Prefs {
    fn default() -> Self {
        let show = [("news", "popup"), ("prop", "icon"), ("dxped", "popup"), ("daily", "off")].into_iter().map(|(k, v)| (k.to_string(), v.to_string())).collect();
        Prefs { enabled: true, show, seconds: 10, quiet_on: false, quiet_from: "22:00".into(), quiet_to: "07:00".into() }
    }
}

impl Prefs {
    fn how(&self, kind: &str) -> &str {
        self.show.get(kind).map(String::as_str).unwrap_or("popup")
    }
    /// Adesso e' «non disturbare» (gli aggiornamenti e i messaggi dell'autore compaiono comunque).
    fn quiet(&self) -> bool {
        if !self.quiet_on {
            return false;
        }
        let now = local_minutes();
        let (Some(a), Some(b)) = (hhmm(&self.quiet_from), hhmm(&self.quiet_to)) else { return false };
        if a <= b { now >= a && now < b } else { now >= a || now < b }
    }
}

fn hhmm(t: &str) -> Option<u32> {
    let (h, m) = t.split_once(':')?;
    Some(h.trim().parse::<u32>().ok()? * 60 + m.trim().parse::<u32>().ok()?)
}

/// Minuti dalla mezzanotte, ora del PC.
fn local_minutes() -> u32 {
    #[cfg(windows)]
    {
        let mut t = unsafe { std::mem::zeroed::<windows_sys::Win32::Foundation::SYSTEMTIME>() };
        unsafe { windows_sys::Win32::System::SystemInformation::GetLocalTime(&mut t) };
        u32::from(t.wHour) * 60 + u32::from(t.wMinute)
    }
    #[cfg(not(windows))]
    {
        0
    }
}

/// Le scelte attuali (di fabbrica se la pagina non e' mai stata toccata).
pub fn prefs() -> Prefs {
    let Some(base) = std::env::var_os("APPDATA") else { return Prefs::default() };
    let file = PathBuf::from(base).join("Aurora").join("ui-settings.json");
    let Ok(text) = std::fs::read_to_string(file) else { return Prefs::default() };
    let Ok(all) = serde_json::from_str::<serde_json::Value>(&text) else { return Prefs::default() };
    all.get("desktop")
        .and_then(|d| d.get("aurora-agente"))
        .and_then(|v| v.as_str())
        .and_then(|t| serde_json::from_str::<Prefs>(t).ok())
        .unwrap_or_default()
}

/// Promuove l'icona di Aurora accanto all'orologio (chiave `NotifyIconSettings` di Windows 11, solo per questo utente).
fn promote_icon() {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        let script = "Get-ChildItem 'HKCU:\\Control Panel\\NotifyIconSettings' -ErrorAction SilentlyContinue | ForEach-Object { $p = (Get-ItemProperty $_.PSPath -ErrorAction SilentlyContinue).ExecutablePath; if ($p -and $p -like '*aurora-desktop.exe') { Set-ItemProperty $_.PSPath -Name IsPromoted -Value 1 -Type DWord } }";
        let _ = std::process::Command::new("powershell").args(["-NoProfile", "-NonInteractive", "-WindowStyle", "Hidden", "-Command", script]).creation_flags(0x0800_0000).status();
    }
}

/// Avvia l'agente (non torna finche' non si sceglie «Esci»). Se un altro agente e' gia' acceso, esce subito.
pub fn run() {
    if !single_instance() {
        return;
    }
    let app = tauri::Builder::default()
        .setup(|app| {
            build_tray(app.handle())?;
            if std::env::var("AURORA_AGENTE_PROVA").as_deref() == Ok("1") {
                let h = app.handle().clone();
                std::thread::spawn(move || {
                    std::thread::sleep(Duration::from_secs(3));
                    show_card(&h, Card { kind: "Prova", title: "Cartolina di prova".into(), text: "Se la vedi, la cartolina funziona.".into(), main: "OK", main_action: "close", update: true });
                });
            }
            let handle = app.handle().clone();
            std::thread::spawn(move || {
                std::thread::sleep(Duration::from_secs(20));
                let mut minutes = 0u64;
                loop {
                    // spento dalla pagina «Agente nel tray»: si toglie dall'avvio con Windows ed esce
                    if !prefs().enabled {
                        crate::set_agent_autostart(false);
                        handle.exit(0);
                        return;
                    }
                    if minutes % (EVERY.as_secs() / 60) == 0 {
                        check(&handle, false);
                    }
                    minutes += 1;
                    std::thread::sleep(Duration::from_secs(60));
                }
            });
            // l'icona sempre in vista accanto all'orologio (Windows 11 la mette nella freccetta «^» finche' non la si promuove)
            std::thread::spawn(|| {
                std::thread::sleep(Duration::from_secs(8));
                promote_icon();
            });
            Ok(())
        })
        .build(tauri::generate_context!());
    let Ok(app) = app else { return };
    // chiudendo la cartolina il programma non esce: resta l'icona nel tray
    app.run(|_, event| {
        if let RunEvent::ExitRequested { api, code: None, .. } = event {
            api.prevent_exit();
        }
    });
}

/// Un solo agente per utente: un mutex con nome di Windows.
fn single_instance() -> bool {
    #[cfg(windows)]
    {
        use windows_sys::Win32::Foundation::{GetLastError, ERROR_ALREADY_EXISTS};
        use windows_sys::Win32::System::Threading::CreateMutexW;
        let name: Vec<u16> = "Local\\AuroraAgente\0".encode_utf16().collect();
        // il mutex resta aperto per tutta la vita del processo (si chiude da solo all'uscita)
        let handle = unsafe { CreateMutexW(std::ptr::null(), 0, name.as_ptr()) };
        if handle.is_null() {
            return true;
        }
        unsafe { GetLastError() != ERROR_ALREADY_EXISTS }
    }
    #[cfg(not(windows))]
    {
        true
    }
}

fn build_tray(app: &AppHandle) -> tauri::Result<()> {
    let open = MenuItem::with_id(app, "open", "Apri Aurora", true, None::<&str>)?;
    let check_now = MenuItem::with_id(app, "check", "Cerca aggiornamenti", true, None::<&str>)?;
    let sep = PredefinedMenuItem::separator(app)?;
    let quit = MenuItem::with_id(app, "quit", "Esci dall'agente", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&open, &check_now, &sep, &quit])?;
    let mut tray = TrayIconBuilder::with_id("aurora-agente").tooltip("Aurora").menu(&menu).show_menu_on_left_click(false);
    if let Some(icon) = app.default_window_icon() {
        tray = tray.icon(icon.clone());
    }
    tray.on_menu_event(|app, event| match event.id().as_ref() {
        "open" => open_aurora(),
        "check" => {
            let app = app.clone();
            std::thread::spawn(move || check(&app, true));
        }
        "quit" => app.exit(0),
        _ => {}
    })
    .on_tray_icon_event(|_, event| {
        // clic sinistro sull'icona: apre Aurora
        if let TrayIconEvent::Click { button: MouseButton::Left, button_state: MouseButtonState::Up, .. } = event {
            open_aurora();
        }
    })
    .build(app)?;
    Ok(())
}

/// Apre Aurora (se e' gia' aperta, il programma porta davanti la sua finestra).
fn open_aurora() {
    if let Ok(exe) = std::env::current_exe() {
        let _ = std::process::Command::new(exe).spawn();
    }
}

fn agent_file() -> Option<PathBuf> {
    let base = std::env::var_os("APPDATA")?;
    Some(PathBuf::from(base).join("Aurora").join("agente.json"))
}

#[derive(serde::Serialize, serde::Deserialize, Default)]
struct Seen {
    #[serde(default)]
    versions: Vec<String>,
    #[serde(default)]
    messages: Vec<String>,
}

fn load_seen() -> Seen {
    agent_file().and_then(|p| std::fs::read_to_string(p).ok()).and_then(|t| serde_json::from_str(&t).ok()).unwrap_or_default()
}

fn save_seen(seen: &Seen) {
    if let (Some(p), Ok(text)) = (agent_file(), serde_json::to_string(seen)) {
        let _ = std::fs::write(p, text);
    }
}

fn agent() -> ureq::Agent {
    ureq::Agent::config_builder().timeout_global(Some(Duration::from_secs(15))).build().into()
}

/// Aurora e' aperta su questo PC (risponde il suo servizio locale).
fn aurora_open() -> bool {
    ureq::Agent::config_builder()
        .timeout_global(Some(Duration::from_secs(2)))
        .build()
        .new_agent()
        .get("http://127.0.0.1:8787/api/v1/health")
        .call()
        .is_ok()
}

fn get_text(url: &str) -> Option<String> {
    let mut response = agent().get(url).header("User-Agent", "Aurora-agente").call().ok()?;
    response.body_mut().read_to_string().ok()
}

/// La data UTC fra `days` giorni, come `AAAA-MM-GG`.
fn utc_day(days: i64) -> String {
    let secs = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs() as i64).unwrap_or(0) + days * 86_400;
    let z = secs.div_euclid(86_400) + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + i64::from(m <= 2);
    format!("{y:04}-{m:02}-{d:02}")
}

/// QSO di ieri (UTC) nel log di Aurora e quanti paesi diversi.
fn yesterday_summary() -> Option<(usize, usize)> {
    let base = std::env::var_os("APPDATA")?;
    let text = std::fs::read_to_string(PathBuf::from(base).join("Aurora").join("aurora_log.adi")).ok()?;
    let day = utc_day(-1).replace('-', "");
    let lower = text.to_lowercase();
    let mut n = 0;
    let mut countries = std::collections::HashSet::new();
    for rec in lower.split("<eor>") {
        if !rec.contains(&format!("<qso_date:8>{day}")) {
            continue;
        }
        n += 1;
        if let Some(i) = rec.find("<country:") {
            let rest = &rec[i + 9..];
            if let Some((len, body)) = rest.split_once('>') {
                let len: usize = len.split(':').next().and_then(|l| l.parse().ok()).unwrap_or(0);
                countries.insert(body.chars().take(len).collect::<String>());
            }
        }
    }
    (n > 0).then_some((n, countries.len()))
}

/// Le DXpedition di paesi non ancora lavorati, salvate da Aurora (pannello DXpedition) nelle impostazioni. `None` se l'elenco manca
/// o e' vecchio di oltre 7 giorni: allora si avvisa per tutte.
fn missing_dxpeds() -> Option<Vec<String>> {
    let base = std::env::var_os("APPDATA")?;
    let text = std::fs::read_to_string(PathBuf::from(base).join("Aurora").join("ui-settings.json")).ok()?;
    let all: serde_json::Value = serde_json::from_str(&text).ok()?;
    let raw = all.get("desktop")?.get("aurora-dxped-mancanti")?.as_str()?;
    let v: serde_json::Value = serde_json::from_str(raw).ok()?;
    let at = v.get("at")?.as_u64()?;
    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).ok()?.as_millis() as u64;
    if now.saturating_sub(at) > 7 * 86_400_000 {
        return None;
    }
    Some(v.get("calls")?.as_array()?.iter().filter_map(|c| c.as_str().map(str::to_string)).collect())
}

fn get_json(url: &str) -> Option<serde_json::Value> {
    let mut response = agent().get(url).header("User-Agent", "Aurora-agente").header("Accept", "application/json").call().ok()?;
    response.body_mut().read_json::<serde_json::Value>().ok()
}

/// Il controllo: versione nuova (sempre, scelta del proprietario) e messaggi dell'autore. `asked`: dal menu, mostra anche «aggiornata».
fn check(app: &AppHandle, asked: bool) {
    if CHECKING.swap(true, Ordering::SeqCst) {
        return;
    }
    let mut seen = load_seen();
    let open = aurora_open();
    let mut shown = false;
    if let Some(release) = get_json(REPO_API) {
        let tag = release.get("tag_name").and_then(|v| v.as_str()).unwrap_or("").trim_start_matches('v').to_string();
        if !tag.is_empty() && updater::is_newer(&tag, VERSION) && (asked || (!open && !seen.versions.contains(&tag))) {
            let notes = release.get("body").and_then(|v| v.as_str()).unwrap_or("");
            let summary: String = notes
                .lines()
                .map(|l| l.trim().trim_start_matches(['-', '*', ' ']).replace("**", ""))
                .filter(|l| l.len() > 3)
                .take(3)
                .collect::<Vec<_>>()
                .join(" · ");
            show_card(app, Card { kind: "Aggiornamento", title: format!("Aurora {tag} è pronta"), text: summary, main: "Aggiorna ora", main_action: "open", update: true });
            seen.versions.push(tag);
            shown = true;
        } else if asked {
            show_card(app, Card { kind: "Aggiornamenti", title: format!("Aurora è aggiornata ({VERSION})"), text: "Hai già l'ultima versione.".into(), main: "OK", main_action: "close", update: false });
            shown = true;
        }
    }
    if !shown && !open && let Some(list) = get_json(&format!("{MESSAGES}?v={VERSION}")) {
        let next = list.get("messaggi").and_then(|m| m.as_array()).and_then(|a| {
            a.iter().find(|m| m.get("id").and_then(|i| i.as_str()).is_some_and(|id| !seen.messages.iter().any(|s| s == id)))
        });
        if let Some(m) = next {
            let id = m.get("id").and_then(|i| i.as_str()).unwrap_or("").to_string();
            let text = |key: &str| {
                let t = m.get(key);
                t.and_then(|t| t.get("it")).or_else(|| t.and_then(|t| t.get("en"))).and_then(|v| v.as_str()).unwrap_or("").to_string()
            };
            let link = m.get("link").and_then(|v| v.as_str()).unwrap_or("").to_string();
            let festive = m.get("tema").and_then(|v| v.as_str()) == Some("festa");
            show_card(app, Card {
                kind: if festive { "✦ IU3WJQ" } else { "IU3WJQ" },
                title: text("titolo"),
                text: text("testo"),
                main: if link.is_empty() { "OK" } else { "Apri" },
                main_action: if link.is_empty() { "close" } else { "link" },
                update: false,
            });
            if !link.is_empty() {
                LINK.lock().map(|mut l| *l = link).ok();
            }
            seen.messages.push(id);
        }
    }
    // gli avvisi che si possono scegliere: popup, solo icona (nel suggerimento dell'icona) o spenti; mai durante «non disturbare»
    let p = prefs();
    if !open && !p.quiet() {
        let mut icon_notes: Vec<String> = Vec::new();
        let mut cards: Vec<Card> = Vec::new();
        let today = utc_day(0);
        let mut want = |kind: &str, key: String, card: Card, seen: &mut Seen| {
            if seen.messages.iter().any(|s| *s == key) {
                return;
            }
            match p.how(kind) {
                "popup" => cards.push(card),
                "icon" => icon_notes.push(card.title.clone()),
                _ => return,
            }
            seen.messages.push(key);
        };
        // propagazione (hamqsl.com): tempesta geomagnetica e 12-10 m aperti di giorno, una volta al giorno
        if let Some(sol) = get_text(online_info::solar::URL).and_then(|b| online_info::solar::parse(&b)) {
            if let Some(k) = sol.k_index.filter(|k| *k >= 5) {
                want("prop", format!("prop-{today}-k"), Card { kind: "Propagazione", title: format!("Tempesta geomagnetica (K {k})"), text: "Le bande HF sono disturbate: meglio le bande basse e i segnali forti.".into(), main: "OK", main_action: "close", update: false }, &mut seen);
            }
            if sol.bands.iter().any(|b| b.name.contains("12m-10m") && b.time == "day" && b.condition == "Good") {
                want("prop", format!("prop-{today}-10m"), Card { kind: "Propagazione", title: "12 e 10 metri aperti".into(), text: "Le condizioni di oggi danno buone le bande alte di giorno.".into(), main: "Apri Aurora", main_action: "open", update: false }, &mut seen);
            }
        }
        // DXpedition che partono domani (NG3K)
        let tomorrow = utc_day(1);
        if let Some(html) = get_text(online_info::dxped::URL) {
            let missing = missing_dxpeds();
            for d in online_info::dxped::parse(&html)
                .into_iter()
                .filter(|d| d.start == tomorrow)
                .filter(|d| missing.as_ref().is_none_or(|m| m.iter().any(|c| c.eq_ignore_ascii_case(&d.call))))
                .take(3)
            {
                want("dxped", format!("dxped-{}-{}", d.call, d.start), Card { kind: "DXpedition", title: format!("Domani parte {} · {}", d.call, d.country), text: d.info.chars().take(160).collect(), main: "Apri Aurora", main_action: "open", update: false }, &mut seen);
            }
        }
        // riepilogo di ieri (dal log di Aurora), dopo le 8 del mattino
        if local_minutes() >= 8 * 60 && let Some((n, countries)) = yesterday_summary() {
            want("daily", format!("daily-{today}"), Card { kind: "Riepilogo", title: format!("Ieri: {n} QSO"), text: format!("{countries} paesi diversi. Buoni DX anche oggi!"), main: "Apri Aurora", main_action: "open", update: false }, &mut seen);
        }
        if !icon_notes.is_empty() {
            let note = format!("Aurora · {}", icon_notes.join(" · "));
            let app2 = app.clone();
            let _ = app.run_on_main_thread(move || {
                if let Some(tray) = app2.tray_by_id("aurora-agente") {
                    let _ = tray.set_tooltip(Some(note.chars().take(120).collect::<String>()));
                }
            });
        }
        // una cartolina alla volta, distanziate
        if !cards.is_empty() {
            let app2 = app.clone();
            let gap = Duration::from_millis(u64::from(p.seconds.clamp(3, 60)) * 1000 + 3000);
            std::thread::spawn(move || {
                if shown {
                    std::thread::sleep(gap);
                }
                for (i, c) in cards.into_iter().enumerate() {
                    if i > 0 {
                        std::thread::sleep(gap);
                    }
                    show_card(&app2, c);
                }
            });
        }
    }
    seen.versions.sort();
    seen.versions.dedup();
    let keep = seen.messages.len().saturating_sub(200);
    seen.messages.drain(..keep);
    save_seen(&seen);
    CHECKING.store(false, Ordering::SeqCst);
}

static LINK: std::sync::Mutex<String> = std::sync::Mutex::new(String::new());

struct Card {
    kind: &'static str,
    title: String,
    text: String,
    main: &'static str,
    /// `open` apre Aurora, `link` apre il collegamento del messaggio, `close` chiude e basta.
    main_action: &'static str,
    update: bool,
}

/// Registro dell'agente (non ha la console): `%TEMP%\aurora-agente.log`.
fn agent_log(line: &str) {
    use std::io::Write;
    if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(std::env::temp_dir().join("aurora-agente.log")) {
        let _ = writeln!(f, "{line}");
    }
}

fn agent_webview_dir() -> PathBuf {
    std::env::var_os("LOCALAPPDATA").map(PathBuf::from).unwrap_or_else(std::env::temp_dir).join("Aurora").join("agente-webview")
}

fn esc(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;")
}

#[allow(dead_code)]
fn percent(s: &str) -> String {
    let mut out = String::with_capacity(s.len() * 2);
    for b in s.bytes() {
        if b.is_ascii_alphanumeric() || b"-_.~".contains(&b) {
            out.push(b as char);
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    out
}

/// La cartolina sopra l'orologio: finestra senza cornice, sempre davanti, si chiude da sola dopo 20 s (non col mouse sopra).
fn show_card(app: &AppHandle, card: Card) {
    let html = CARD_HTML
        .replace("{KIND}", &esc(card.kind))
        .replace("{TITLE}", &esc(&card.title))
        .replace("{TEXT}", &esc(&card.text))
        .replace("{MAIN}", &esc(card.main))
        .replace("{ACTION}", card.main_action)
        .replace("{UPDATE}", if card.update { "1" } else { "0" })
        .replace("{SECONDS}", &(prefs().seconds.clamp(3, 60) * 1000).to_string());
    // la cartolina come file nella cartella temporanea: un indirizzo `data:` non sempre si apre come pagina principale
    let file = std::env::temp_dir().join("aurora-cartolina.html");
    if std::fs::write(&file, &html).is_err() {
        return;
    }
    let Ok(url) = tauri::Url::from_file_path(&file) else { return };
    agent_log("cartolina richiesta");
    let app2 = app.clone();
    let r = app.run_on_main_thread(move || {
        agent_log("cartolina: thread principale");
        if let Some(old) = app2.get_webview_window("cartolina") {
            let _ = old.close();
        }
        let (w, h) = (390.0, 230.0);
        let built = WebviewWindowBuilder::new(&app2, "cartolina", WebviewUrl::External(url))
            .title("Aurora")
            .decorations(false)
            .transparent(true)
            .always_on_top(true)
            .skip_taskbar(true)
            .resizable(false)
            .focused(false)
            .shadow(false)
            .inner_size(w, h)
            .visible(false)
            .additional_browser_args("--autoplay-policy=no-user-gesture-required --disable-logging --log-level=3")
            // una cartella del motore web tutta sua: con Aurora aperta (stessa cartella, impostazioni diverse) WebView2 rifiutava la finestra
            .data_directory(agent_webview_dir())
            .on_navigation({
                let app = app2.clone();
                move |u| {
                    let what = u.host_str().unwrap_or("");
                    if !what.starts_with("aurora-") {
                        return true;
                    }
                    let app = app.clone();
                    let what = what.to_string();
                    let _ = app.clone().run_on_main_thread(move || {
                        match what.as_str() {
                            "aurora-open" => open_aurora(),
                            "aurora-link" => {
                                if let Ok(l) = LINK.lock()
                                    && !l.is_empty()
                                {
                                    crate::links::open_in_browser(&l);
                                }
                            }
                            _ => {}
                        }
                        if let Some(w) = app.get_webview_window("cartolina") {
                            let _ = w.close();
                        }
                    });
                    false
                }
            })
            .build();
        let window = match built {
            Ok(w) => w,
            Err(e) => {
                agent_log(&format!("cartolina non aperta: {e}"));
                return;
            }
        };
        // in basso a destra dello schermo principale, sopra la barra delle applicazioni
        if let Ok(Some(m)) = window.primary_monitor() {
            let scale = m.scale_factor();
            let size = m.size();
            let pos = m.position();
            let x = pos.x + size.width as i32 - ((w + 16.0) * scale) as i32;
            let y = pos.y + size.height as i32 - ((h + 56.0) * scale) as i32;
            let _ = window.set_position(PhysicalPosition::new(x, y));
        }
        let shown = window.show();
        agent_log(&format!("cartolina mostrata: {shown:?}"));
    });
    if let Err(e) = r {
        agent_log(&format!("thread principale non raggiunto: {e}"));
    }
}

const CARD_HTML: &str = r#"<!doctype html><html><head><meta charset="utf-8"><style>
html,body{margin:0;background:transparent;font:13px system-ui,sans-serif;color:#e6edf3;overflow:hidden}
.c{position:absolute;inset:6px;border-radius:14px;overflow:hidden;background:#0b1220;border:1px solid #2cf5a4;box-shadow:0 10px 30px rgb(0 0 0/.5);animation:in .34s cubic-bezier(.2,.9,.3,1.15) both}
.b{display:flex;align-items:center;gap:12px;padding:12px 14px;background:radial-gradient(circle at 18% 50%,#14304f,#0b1626 70%);border-bottom:1px solid #1e2c44}
.l{width:40px;height:40px;border-radius:50%;border:3px solid #2cf5a4;box-shadow:0 0 14px rgb(44 245 164/.5);animation:spin 3s linear infinite;border-right-color:#8e7bff;border-bottom-color:#27a0f5}
.k b{display:block;font:700 17px ui-monospace,monospace;letter-spacing:.32em;background:linear-gradient(100deg,#2cf5a4,#27a0f5 55%,#8e7bff);-webkit-background-clip:text;background-clip:text;color:transparent}
.k span{font-size:11px;letter-spacing:.12em;text-transform:uppercase;color:#8fa0b5}
.x{position:absolute;top:8px;right:10px;border:0;background:none;color:#8fa0b5;font-size:18px;cursor:pointer}
.d{padding:10px 16px 12px}h3{margin:0 0 4px;font-size:15px}p{margin:0;font-size:12.5px;line-height:1.45;color:#aab6c4;max-height:54px;overflow:hidden}
.a{display:flex;justify-content:flex-end;gap:8px;margin-top:10px}button.n{height:28px;padding:0 14px;border-radius:8px;border:1px solid #2a3850;background:none;color:#dbe6f2;font-weight:600;cursor:pointer}
button.m{background:#2cf5a4;border-color:#2cf5a4;color:#04342c}.t{position:absolute;left:0;bottom:0;height:3px;background:linear-gradient(90deg,#2cf5a4,#27a0f5);width:100%}
@keyframes in{from{opacity:0;transform:translateY(24px)}to{opacity:1;transform:none}}@keyframes spin{to{transform:rotate(360deg)}}
</style></head><body><div class="c" id="c"><div class="b"><div class="l"></div><div class="k"><b>AURORA</b><span>{KIND}</span></div><button class="x" onclick="go('close')">×</button></div>
<div class="d"><h3>{TITLE}</h3><p>{TEXT}</p><div class="a"><button class="n" onclick="go('close')">Chiudi</button><button class="n m" onclick="go('{ACTION}')">{MAIN}</button></div></div><i class="t" id="t"></i></div>
<script>
function go(a){location.href='https://aurora-'+a+'/'}
var total={SECONDS},left=total,hover=false,c=document.getElementById('c'),t=document.getElementById('t');
c.onmouseenter=function(){hover=true};c.onmouseleave=function(){hover=false};
setInterval(function(){if(hover)return;left-=100;t.style.width=Math.max(0,left/total*100)+'%';if(left<=0)go('close')},100);
try{var x=new AudioContext(),o=x.createGain();o.gain.value=.9;o.connect(x.destination);var n={UPDATE}?[523.25,659.25,783.99,1046.5,1318.5]:[523.25,659.25,783.99,1046.5],s=x.currentTime+.05;
n.forEach(function(f,i){var at=s+i*.085,g=x.createGain(),v=x.createOscillator();v.type='triangle';v.frequency.value=f;g.gain.setValueAtTime(.0001,at);g.gain.exponentialRampToValueAtTime(.16,at+.015);g.gain.exponentialRampToValueAtTime(.0001,at+.9);v.connect(g);g.connect(o);v.start(at);v.stop(at+.95)})}catch(e){}
</script></body></html>"#;
