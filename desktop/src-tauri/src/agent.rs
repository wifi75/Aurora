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
            // AURORA_AGENTE_PROVA=1 mostra una cartolina di prova; con il nome di un tema (natale, anno, pasqua, aggiorna, servizio) la mostra con quel tema
            let prova = std::env::var("AURORA_AGENTE_PROVA").unwrap_or_default();
            if prova == "1" || matches!(prova.as_str(), "natale" | "anno" | "pasqua" | "aggiorna" | "servizio") {
                let h = app.handle().clone();
                let theme = if prova == "1" { String::new() } else { prova };
                std::thread::spawn(move || {
                    std::thread::sleep(Duration::from_secs(3));
                    show_card(&h, Card { kind: "Prova", title: "Cartolina di prova".into(), text: "Se la vedi, la cartolina funziona.".into(), main: "OK", main_action: "close", update: theme.is_empty(), theme });
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

/// C'e' gia' un «Aggiorna ora» in corso (un secondo clic su «Riprova» non ne avvia un altro).
static UPDATING: AtomicBool = AtomicBool::new(false);

/// Manda alla cartolina lo stato dell'aggiornamento (la funzione `upd` della pagina).
fn card_state(app: &AppHandle, state: &serde_json::Value) -> bool {
    let Some(w) = app.get_webview_window("cartolina") else { return false };
    let _ = w.eval(format!("window.upd&&upd({state})"));
    true
}

/// Il motivo di un «Aggiorna ora» rifiutato da Aurora, in parole (la risposta ha `error` e a volte `message`).
fn install_refusal(status: u16, body: &serde_json::Value) -> String {
    let code = body.get("error").and_then(|v| v.as_str()).unwrap_or("");
    if let Some(m) = body.get("message").and_then(|v| v.as_str()).filter(|m| !m.is_empty()) {
        return m.to_string();
    }
    match code {
        "transmitting" => "Aurora sta trasmettendo: riprova a fine trasmissione.".into(),
        "local_only" => "L'aggiornamento si avvia solo dal PC dove gira Aurora.".into(),
        "install_unavailable" => "Questa Aurora non sa installare gli aggiornamenti da sola.".into(),
        _ => format!("Aurora ha risposto {status}{}", if code.is_empty() { String::new() } else { format!(" ({code})") }),
    }
}

/// «Aggiorna ora» dalla cartolina: Aurora installa la versione nuova (gia' scaricata in anticipo, oppure la scarica adesso). Se Aurora e'
/// chiusa la si apre e si aspetta che risponda. La cartolina resta aperta e ogni ~300 ms riceve lo stato da `GET /api/v1/update`:
/// barra, MB, velocita', tempo e i passi Scarico → Controllo → Installo → Riavvio; in errore mostra il motivo e «Riprova».
fn install_update(app: AppHandle) {
    if UPDATING.swap(true, Ordering::SeqCst) {
        return;
    }
    // il segno per Aurora: l'aggiornamento parte dal tray, niente locandina nella finestra (c'e' gia' questa cartolina). Si lascia prima
    // di aprire Aurora, cosi' vale anche all'avvio
    if let Some(base) = std::env::var_os("APPDATA") {
        let _ = std::fs::write(PathBuf::from(base).join("Aurora").join(updater::TRAY_MARK), b"1");
    }
    if !aurora_open() {
        card_state(&app, &serde_json::json!({ "phase": "starting" }));
        open_aurora();
        for _ in 0..60 {
            std::thread::sleep(Duration::from_secs(1));
            if aurora_open() {
                break;
            }
        }
    }
    // la richiesta di installazione resta aperta per tutto lo scaricamento (se il pacchetto non era gia' pronto): a parte, con molta pazienza
    let (tx, rx) = std::sync::mpsc::channel::<Result<(), String>>();
    std::thread::spawn(move || {
        let response = ureq::Agent::config_builder()
            .timeout_global(Some(Duration::from_secs(30 * 60)))
            .http_status_as_error(false)
            .build()
            .new_agent()
            .post("http://127.0.0.1:8787/api/v1/update/install")
            .send_empty();
        let outcome = match response {
            Ok(r) if r.status().as_u16() == 200 => Ok(()),
            Ok(mut r) => {
                let status = r.status().as_u16();
                let body = r.body_mut().read_json::<serde_json::Value>().unwrap_or_default();
                Err(install_refusal(status, &body))
            }
            Err(e) => Err(format!("Aurora non risponde: {e}")),
        };
        let _ = tx.send(outcome);
    });
    let poll = ureq::Agent::config_builder().timeout_global(Some(Duration::from_secs(2))).build().new_agent();
    let mut answer: Option<Result<(), String>> = None;
    let mut misses = 0u32;
    loop {
        if answer.is_none()
            && let Ok(a) = rx.try_recv()
        {
            answer = Some(a);
        }
        let stage = poll.get("http://127.0.0.1:8787/api/v1/update").call().ok().and_then(|mut r| r.body_mut().read_json::<serde_json::Value>().ok()).map(|v| v["stage"].clone());
        let shown = match (&answer, stage) {
            // Aurora ha rifiutato o lo scaricamento non e' riuscito: il motivo e «Riprova»
            (Some(Err(e)), stage) => {
                let reason = stage.as_ref().filter(|s| s["phase"] == "error").and_then(|s| s["error"].as_str().map(str::to_string)).unwrap_or_else(|| e.clone());
                agent_log(&format!("aggiornamento non riuscito: {reason}"));
                card_state(&app, &serde_json::json!({ "phase": "error", "error": reason }));
                break;
            }
            (_, Some(mut s)) => {
                misses = 0;
                // un errore di un giro precedente, finche' Aurora non ha risposto a questa richiesta: si sta ripartendo
                if answer.is_none() && s["phase"] == "error" {
                    s = serde_json::json!({ "phase": "idle", "version": s["version"] });
                }
                card_state(&app, &s)
            }
            // Aurora non risponde piu' dopo aver avviato l'installatore: si sta riavviando
            (Some(Ok(())), None) => {
                misses += 1;
                // di norma l'agente viene chiuso dall'installatore prima: se resta acceso, dopo un minuto si smette di guardare
                if misses > 200 {
                    break;
                }
                card_state(&app, &serde_json::json!({ "phase": "restart" }))
            }
            (None, None) => {
                misses += 1;
                if misses > 20 {
                    card_state(&app, &serde_json::json!({ "phase": "error", "error": "Aurora non risponde." }));
                    break;
                }
                true
            }
        };
        if !shown {
            // cartolina chiusa: l'installazione va avanti da sola in Aurora
            break;
        }
        std::thread::sleep(Duration::from_millis(300));
    }
    match &answer {
        Some(Ok(())) => agent_log("aggiornamento avviato da Aurora"),
        Some(Err(_)) => {}
        None => agent_log("cartolina dell'aggiornamento chiusa prima della fine"),
    }
    UPDATING.store(false, Ordering::SeqCst);
}

/// Apre Aurora (se e' gia' aperta, il programma porta davanti la sua finestra).
fn open_aurora() {
    allow_foreground();
    if let Ok(exe) = std::env::current_exe() {
        let _ = std::process::Command::new(exe).spawn();
    }
}

/// L'agente ha appena ricevuto il clic (tray o cartolina): solo lui puo' cedere il primo piano. Senza, Windows non lascia ad Aurora
/// portare davanti la sua finestra (la finestra dell'aggiornamento finiva sotto a tutto, 2026-11-08).
fn allow_foreground() {
    #[cfg(windows)]
    {
        // ASFW_ANY: qualunque processo, cioe' Aurora che stiamo per svegliare
        unsafe { windows_sys::Win32::UI::WindowsAndMessaging::AllowSetForegroundWindow(u32::MAX) };
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
            show_card(app, Card { kind: "Aggiornamento", title: format!("Aurora {tag} è pronta"), text: summary, main: "Aggiorna ora", main_action: "update", update: true, theme: String::new() });
            seen.versions.push(tag);
            shown = true;
        } else if asked {
            show_card(app, Card { kind: "Aggiornamenti", title: format!("Aurora è aggiornata ({VERSION})"), text: "Hai già l'ultima versione.".into(), main: "OK", main_action: "close", update: false, theme: String::new() });
            shown = true;
        }
    }
    else if asked {
        // GitHub non risponde (l'8/11/2026 dava 504 anche all'API): dal menu si chiede un controllo, quindi si risponde sempre, anche con un errore
        agent_log("controllo degli aggiornamenti non riuscito: GitHub non risponde");
        show_card(app, Card { kind: "Aggiornamenti", title: "Controllo non riuscito".into(), text: "GitHub in questo momento non risponde. Riprova tra qualche minuto: Aurora controlla comunque da sola.".into(), main: "OK", main_action: "close", update: false, theme: String::new() });
        shown = true;
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
                theme: m
                    .get("stile")
                    .and_then(|v| v.as_str())
                    .filter(|t| matches!(*t, "natale" | "anno" | "pasqua" | "aggiorna" | "servizio"))
                    .unwrap_or("")
                    .to_string(),
            });
            if !link.is_empty() {
                LINK.lock().map(|mut l| *l = link).ok();
            }
            report_seen(&id);
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
                want("prop", format!("prop-{today}-k"), Card { kind: "Propagazione", title: format!("Tempesta geomagnetica (K {k})"), text: "Le bande HF sono disturbate: meglio le bande basse e i segnali forti.".into(), main: "OK", main_action: "close", update: false, theme: String::new() }, &mut seen);
            }
            if sol.bands.iter().any(|b| b.name.contains("12m-10m") && b.time == "day" && b.condition == "Good") {
                want("prop", format!("prop-{today}-10m"), Card { kind: "Propagazione", title: "12 e 10 metri aperti".into(), text: "Le condizioni di oggi danno buone le bande alte di giorno.".into(), main: "Apri Aurora", main_action: "open", update: false, theme: String::new() }, &mut seen);
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
                want("dxped", format!("dxped-{}-{}", d.call, d.start), Card { kind: "DXpedition", title: format!("Domani parte {} · {}", d.call, d.country), text: d.info.chars().take(160).collect(), main: "Apri Aurora", main_action: "open", update: false, theme: String::new() }, &mut seen);
            }
        }
        // riepilogo di ieri (dal log di Aurora), dopo le 8 del mattino
        if local_minutes() >= 8 * 60 && let Some((n, countries)) = yesterday_summary() {
            want("daily", format!("daily-{today}"), Card { kind: "Riepilogo", title: format!("Ieri: {n} QSO"), text: format!("{countries} paesi diversi. Buoni DX anche oggi!"), main: "Apri Aurora", main_action: "open", update: false, theme: String::new() }, &mut seen);
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
    /// Tema grafico dei messaggi dell'autore: natale, anno, pasqua, aggiorna, servizio (vuoto = normale).
    theme: String,
}

/// «Ho mostrato la cartolina del messaggio `id`»: con il codice anonimo dell'installazione del motore (`rapporti-installazione.txt`), solo se
/// il conteggio delle installazioni attive e' acceso (`count_active` in `rapporti.json`, acceso se il file non c'e'). Serve solo a contare.
fn report_seen(id: &str) {
    let Some(base) = std::env::var_os("APPDATA").map(|b| PathBuf::from(b).join("Aurora")) else { return };
    let acceso = std::fs::read_to_string(base.join("rapporti.json"))
        .ok()
        .and_then(|t| serde_json::from_str::<serde_json::Value>(&t).ok())
        .and_then(|v| v.get("count_active").and_then(|b| b.as_bool()))
        .unwrap_or(true);
    let inst = std::fs::read_to_string(base.join("rapporti-installazione.txt")).unwrap_or_default().trim().to_string();
    if !acceso || inst.len() != 36 || id.is_empty() {
        return;
    }
    let url = format!("{MESSAGES}/{id}/visto");
    std::thread::spawn(move || {
        let r = agent()
            .post(&url)
            .header("X-Aurora-Key", "aurora-rapporti-v1")
            .header("User-Agent", "Aurora-agente")
            .send_json(serde_json::json!({ "installazione": inst }));
        agent_log(&format!("visto inviato: {}", r.map(|x| x.status().as_u16()).unwrap_or(0)));
    });
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
        .replace("{THEME}", if card.theme.is_empty() { "" } else { &card.theme })
        .replace("{SECONDS}", &(prefs().seconds.clamp(3, 60) * 1000).to_string());
    // la cartolina come file nella cartella temporanea: un indirizzo `data:` non sempre si apre come pagina principale
    let file = std::env::temp_dir().join("aurora-cartolina.html");
    if std::fs::write(&file, &html).is_err() {
        return;
    }
    let Ok(url) = tauri::Url::from_file_path(&file) else { return };
    agent_log("cartolina richiesta");
    let hat = card.theme == "natale";
    let tall = card.update;
    let app2 = app.clone();
    let r = app.run_on_main_thread(move || {
        agent_log("cartolina: thread principale");
        if let Some(old) = app2.get_webview_window("cartolina") {
            let _ = old.close();
        }
        // la cartolina dell'aggiornamento un po' piu' alta: dopo «Aggiorna ora» ci stanno barra, dati e passi
        let (w, h) = if hat { (430.0, 272.0) } else if tall { (390.0, 250.0) } else { (390.0, 230.0) };
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
                            "aurora-update" => {
                                // la cartolina resta aperta e mostra l'avanzamento (la pagina e' gia' passata alla barra)
                                allow_foreground();
                                let app = app.clone();
                                std::thread::spawn(move || install_update(app));
                                return;
                            }
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
.b{position:relative;isolation:isolate}.b>*:not(.dc):not(.x){position:relative;z-index:1}.x{z-index:2}.dc{position:absolute;inset:0;z-index:0;pointer-events:none;overflow:hidden}.dc:before{content:'';position:absolute;inset:0}.dc:after{position:absolute;right:46px;top:50%;transform:translateY(-50%);font-size:32px;line-height:1;filter:drop-shadow(0 2px 6px rgb(0 0 0/.45))}.natale{border-color:#e8c063;overflow:visible;top:46px;left:44px}.natale .b{background:radial-gradient(circle at 18% 50%,#8a1626,#3a0b14 72%);border-radius:13px 13px 0 0}.natale .dc:before{background-image:radial-gradient(2px 2px at 12% 20%,#fff 50%,transparent 51%),radial-gradient(1.5px 1.5px at 30% 70%,#fff 50%,transparent 51%),radial-gradient(2px 2px at 55% 35%,#fff 50%,transparent 51%),radial-gradient(1.5px 1.5px at 72% 80%,#fff 50%,transparent 51%),radial-gradient(2px 2px at 88% 25%,#fff 50%,transparent 51%);animation:snow 6s linear infinite;opacity:.85}.natale .dc:after{content:'🎄'}.natale .k b{background:linear-gradient(100deg,#ffe08a,#fff4d6 50%,#e8c063);-webkit-background-clip:text;background-clip:text}.natale button.m{background:#e8c063;border-color:#e8c063;color:#3a0b14}.natale .t{background:linear-gradient(90deg,#d6303f,#e8c063);border-radius:0 0 0 13px}.natale:before{content:'';position:absolute;z-index:3;left:-35px;top:-45px;width:70px;height:58px;transform:rotate(-45deg);pointer-events:none;filter:drop-shadow(0 4px 6px rgb(0 0 0/.45));background:no-repeat center/contain url("data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 120 100'%3E%3Cdefs%3E%3ClinearGradient id='r' x1='0' y1='0' x2='1' y2='1'%3E%3Cstop offset='0' stop-color='%23ff4d5e'/%3E%3Cstop offset='1' stop-color='%23b3121f'/%3E%3C/linearGradient%3E%3C/defs%3E%3Cpath d='M14 78 C26 40 52 14 82 10 C98 8 108 18 104 30 C100 22 90 20 80 26 C66 36 64 60 66 78 Z' fill='url(%23r)'/%3E%3Cpath d='M18 60 C30 44 46 34 60 32' stroke='%23ffffff' stroke-opacity='.18' stroke-width='5' fill='none' stroke-linecap='round'/%3E%3Crect x='6' y='70' width='70' height='20' rx='10' fill='%23f7f7f2'/%3E%3Cpath d='M12 76 h58' stroke='%23d9d9d0' stroke-width='3' stroke-linecap='round' stroke-dasharray='2 6'/%3E%3Ccircle cx='106' cy='32' r='11' fill='%23f7f7f2'/%3E%3Ccircle cx='103' cy='29' r='4' fill='%23ffffff'/%3E%3C/svg%3E")}.anno{border-color:#b48cff}.anno .b{background:radial-gradient(circle at 18% 50%,#2b1a5e,#0a0820 72%)}.anno .dc:before{background-image:radial-gradient(2px 2px at 15% 30%,#ffd76a 50%,transparent 51%),radial-gradient(2.5px 2.5px at 38% 65%,#fff 50%,transparent 51%),radial-gradient(2px 2px at 60% 22%,#c9a8ff 50%,transparent 51%),radial-gradient(2px 2px at 78% 70%,#ffd76a 50%,transparent 51%);animation:tw 1.8s ease-in-out infinite alternate}.anno .dc:after{content:'🥂'}.anno .k b{background:linear-gradient(100deg,#ffd76a,#c9a8ff 55%,#8e7bff);-webkit-background-clip:text;background-clip:text}.anno button.m{background:#ffd76a;border-color:#ffd76a;color:#2a1a00}.pasqua{border-color:#f7a8c8}.pasqua .b{background:radial-gradient(circle at 18% 50%,#2f5a3a,#13261b 72%)}.pasqua .dc:before{background-image:radial-gradient(3px 3px at 14% 28%,#f7a8c8 50%,transparent 51%),radial-gradient(3px 3px at 34% 72%,#ffe58a 50%,transparent 51%),radial-gradient(3px 3px at 56% 30%,#c9b6ff 50%,transparent 51%),radial-gradient(3px 3px at 74% 74%,#9ee6b0 50%,transparent 51%);animation:tw 2.6s ease-in-out infinite alternate}.pasqua .dc:after{content:'🐣'}.pasqua .k b{background:linear-gradient(100deg,#f7a8c8,#ffe58a 50%,#9ee6b0);-webkit-background-clip:text;background-clip:text}.pasqua button.m{background:#9ee6b0;border-color:#9ee6b0;color:#0f2e19}.aggiorna .b{background:radial-gradient(circle at 18% 50%,#0f4a44,#071a20 72%)}.aggiorna .dc:before{background:repeating-linear-gradient(0deg,transparent 0 14px,rgb(44 245 164/.07) 14px 15px);animation:up 2.4s linear infinite}.aggiorna .dc:after{content:'⬆';color:#2cf5a4;font-weight:700}.servizio{border-color:#f5a524}.servizio .b{background:repeating-linear-gradient(135deg,#3a2806 0 14px,#2a1d04 14px 28px)}.servizio .dc:after{content:'⚠️'}.servizio .k b{background:linear-gradient(100deg,#ffc65c,#f5a524 60%,#ff8a3d);-webkit-background-clip:text;background-clip:text}.servizio button.m{background:#f5a524;border-color:#f5a524;color:#2a1800}@keyframes snow{from{background-position:0 -60px}to{background-position:0 60px}}@keyframes tw{from{opacity:.35}to{opacity:1}}@keyframes up{from{background-position:0 0}to{background-position:0 -30px}}
.pg{display:none}.on .pg{display:block}.on #tx,.on #ac{display:none}.br{height:7px;border-radius:99px;background:#1e2c44;overflow:hidden;margin:6px 0 7px}#bf{display:block;height:100%;width:0;background:linear-gradient(90deg,#2cf5a4,#27a0f5);border-radius:inherit;transition:width .3s}#bf.w{width:35%;animation:wt 1.2s ease-in-out infinite}#bf.ko{background:#e5484d}@keyframes wt{from{transform:translateX(-100%)}to{transform:translateX(290%)}}
.mt{font-size:12px;color:#aab6c4;white-space:nowrap;overflow:hidden;text-overflow:ellipsis;font-variant-numeric:tabular-nums}.mt.ko{color:#ffb4a8;white-space:normal;max-height:32px}.ps{display:flex;align-items:center;gap:3px;white-space:nowrap;margin-top:9px;font-size:11.5px}.ps span{padding:3px 7px;border-radius:99px;border:1px solid #2a3850;color:#8fa0b5}.ps span.cur{border-color:#2cf5a4;color:#2cf5a4;background:rgb(44 245 164/.12);font-weight:600}.ps span.ok{color:#dbe6f2;border-color:#24533f}.ps span.ko{border-color:#e5484d;color:#ffb4a8}.ps b{color:#4a5a72;font-weight:400}#ra{display:none;margin-top:8px}
</style></head><body><div class="c {THEME}" id="c"><div class="b"><i class="dc"></i><div class="l"></div><div class="k"><b>AURORA</b><span>{KIND}</span></div><button class="x" onclick="go('close')">×</button></div>
<div class="d"><h3>{TITLE}</h3><p id="tx">{TEXT}</p><div class="a" id="ac"><button class="n" onclick="go('close')">Chiudi</button><button class="n m" onclick="go('{ACTION}')">{MAIN}</button></div>
<div class="pg" id="pg"><div class="br"><i id="bf"></i></div><div class="mt" id="mt">Avvio…</div><div class="ps" id="ps"></div><div class="a" id="ra"><button class="n" onclick="go('close')">Chiudi</button><button class="n m" onclick="go('update')">Riprova</button></div></div></div><i class="t" id="t"></i></div>
<script>
var prog=false,last=0,STEPS=['Scarico','Controllo','Installo','Riavvio'];
function $(i){return document.getElementById(i)}
function go(a){if(a==='update')startProg();location.href='https://aurora-'+a+'/'}
function mb(b){return (b/1048576).toFixed(1).replace('.',',')}
function startProg(){prog=true;last=0;$('c').classList.add('on');$('t').style.display='none';upd({phase:'idle'})}
function upd(s){if(!prog){prog=true;$('c').classList.add('on');$('t').style.display='none'}
var ph=s.phase||'idle',i={starting:0,idle:0,downloading:0,verifying:1,ready:2,installing:2,restart:3}[ph],err=ph==='error';if(err)i=last;else last=i;
var h='';STEPS.forEach(function(n,k){var c=k<i?'ok':k===i?(err?'ko':'cur'):'';h+=(k?'<b>›</b>':'')+'<span class="'+c+'">'+(c==='ok'?'✓ ':'')+n+'</span>'});$('ps').innerHTML=h;
var bf=$('bf'),pc=s.percent==null?null:s.percent,m='';bf.className='';
if(ph==='downloading'&&pc!=null){bf.style.width=pc+'%';m=pc+'%';if(s.total)m+=' · '+mb(s.done)+' / '+mb(s.total)+' MB';if(s.speed_bps)m+=' · '+mb(s.speed_bps)+' MB/s';if(s.eta_s)m+=' · mancano '+s.eta_s+' s'}
else if(ph==='starting'||ph==='idle'||ph==='downloading'){bf.className='w';m=ph==='starting'?'Apro Aurora…':ph==='idle'?'Preparo lo scaricamento…':'Scarico la versione nuova…'}
else if(ph==='verifying'){bf.className='w';m='Controllo la firma del pacchetto…'}
else if(ph==='ready'){bf.style.width='100%';m='Pacchetto pronto e firmato: avvio l’installazione…'}
else if(ph==='installing'){bf.style.width='100%';m='Avvio l’installatore: Aurora si chiude e si riapre da sola.'}
else if(ph==='restart'){bf.style.width='100%';m='Aurora si sta riavviando…'}
else{bf.style.width='100%';bf.className='ko';m=s.error||'Aggiornamento non riuscito.'}
$('mt').textContent=m;$('mt').title=m;$('mt').className=err?'mt ko':'mt';$('ra').style.display=err?'flex':'none'}
var total={SECONDS},left=total,hover=false,c=$('c'),t=$('t');
c.onmouseenter=function(){hover=true};c.onmouseleave=function(){hover=false};
setInterval(function(){if(hover||prog)return;left-=100;t.style.width=Math.max(0,left/total*100)+'%';if(left<=0)go('close')},100);
try{var x=new AudioContext(),o=x.createGain();o.gain.value=.9;o.connect(x.destination);var n={UPDATE}?[523.25,659.25,783.99,1046.5,1318.5]:[523.25,659.25,783.99,1046.5],s=x.currentTime+.05;
n.forEach(function(f,i){var at=s+i*.085,g=x.createGain(),v=x.createOscillator();v.type='triangle';v.frequency.value=f;g.gain.setValueAtTime(.0001,at);g.gain.exponentialRampToValueAtTime(.16,at+.015);g.gain.exponentialRampToValueAtTime(.0001,at+.9);v.connect(g);g.connect(o);v.start(at);v.stop(at+.95)})}catch(e){}
</script></body></html>"#;
