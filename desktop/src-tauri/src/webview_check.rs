//! WebView2 (il motore con cui Windows disegna la finestra di Aurora) che manca o non si apre. Tauri allora si ferma con un panic
//! («Could not find the webview runtime») e Aurora ripartiva e ricadeva in pochi secondi, piu' volte di fila: lo hanno mandato i
//! rapporti del 2026-11-12 (Windows 10, sette crash in mezz'ora). L'installatore lo installa se manca, ma un aggiornamento o una
//! pulizia di Windows possono lasciarlo registrato e rotto. Qui lo si controlla prima di aprire la finestra: se manca, un messaggio
//! di Windows lo spiega e, con «Sì», Aurora scarica l'installazione di Microsoft, la apre (con la sua finestra di avanzamento, per
//! l'utente di Windows, senza diritti di amministratore), ne aspetta la fine e si riapre da sola; se lo scaricamento non riesce si apre
//! la pagina di Microsoft nel browser. Aurora si chiude senza crash.

/// Il programma d'installazione di WebView2 di Microsoft (lo stesso che usa l'installatore di Aurora).
#[cfg(windows)]
const DOWNLOAD_URL: &str = "https://go.microsoft.com/fwlink/p/?LinkId=2124703";

/// `true` se la finestra si puo' aprire. Senza WebView2 mostra il messaggio e restituisce `false`.
#[cfg(windows)]
pub fn ensure() -> bool {
    let Err(e) = tauri::webview_version() else { return true };
    eprintln!("WebView2 non disponibile: {e}");
    let lang = crate::agent::ui_lang();
    let (title, text) = texts(lang);
    if ask_yes_no(title, text)
        && let Err(e) = install_and_restart()
    {
        eprintln!("{e}");
        crate::links::open_in_browser(DOWNLOAD_URL);
    }
    false
}

/// Scarica il programma d'installazione di WebView2 di Microsoft nella cartella temporanea (qui, in Rust, prima che Aurora si chiuda),
/// lo apre (`/install`, con la sua finestra di avanzamento) e ne aspetta la fine; poi riapre Aurora. Niente PowerShell: prima lo faceva
/// uno script nascosto, che gli antivirus segnalano. Se lo scaricamento o l'avvio non riescono, l'errore fa aprire la pagina di Microsoft.
#[cfg(windows)]
fn install_and_restart() -> Result<(), String> {
    let file = std::env::temp_dir().join("MicrosoftEdgeWebview2Setup.exe");
    download(&file)?;
    let process = crate::installer::win::shell_start(&file, "/install", None, true)?;
    if let Some(process) = process {
        // l'installazione di Microsoft scarica il componente: puo' metterci qualche minuto
        process.wait(30 * 60 * 1000);
    }
    let _ = std::fs::remove_file(&file);
    if let Ok(exe) = std::env::current_exe() {
        let dir = exe.parent().map(std::path::Path::to_path_buf);
        crate::installer::win::shell_start(&exe, "", dir.as_deref(), false)?;
    }
    Ok(())
}

/// Lo scaricamento del programma d'installazione (un paio di MB) da Microsoft, con lo stesso cliente HTTP del resto di Aurora.
#[cfg(windows)]
fn download(file: &std::path::Path) -> Result<(), String> {
    use std::io::Read;
    let agent: ureq::Agent = ureq::Agent::config_builder().timeout_global(Some(std::time::Duration::from_secs(120))).build().into();
    let mut response = agent.get(DOWNLOAD_URL).call().map_err(|e| format!("WebView2 non scaricato: {e}"))?;
    let mut data = Vec::new();
    response.body_mut().as_reader().take(50 * 1024 * 1024).read_to_end(&mut data).map_err(|e| format!("WebView2 non scaricato: {e}"))?;
    // un eseguibile di Windows comincia con «MZ»: una pagina d'errore non si apre
    if !data.starts_with(b"MZ") {
        return Err("lo scaricamento di WebView2 non e' un programma".into());
    }
    std::fs::write(file, data).map_err(|e| format!("WebView2 non salvato: {e}"))
}

#[cfg(not(windows))]
pub fn ensure() -> bool {
    true
}

/// Titolo e testo del messaggio nelle sei lingue di Aurora (0 it, 1 en, 2 de, 3 fr, 4 es, 5 pt).
#[cfg(windows)]
fn texts(lang: usize) -> (&'static str, &'static str) {
    const ALL: [(&str, &str); 6] = [
        (
            "Aurora: manca WebView2",
            "Aurora non può aprire la sua finestra: su questo PC manca Microsoft Edge WebView2, il componente di Windows con cui la disegna (oppure non funziona).\n\nVuoi che Aurora lo installi ora? Si apre l'installazione gratuita di Microsoft e alla fine Aurora si riapre da sola.",
        ),
        (
            "Aurora: WebView2 is missing",
            "Aurora cannot open its window: Microsoft Edge WebView2, the Windows component it uses to draw it, is missing on this PC (or is not working).\n\nShould Aurora install it now? Microsoft's free installer opens and, when it finishes, Aurora opens again by itself.",
        ),
        (
            "Aurora: WebView2 fehlt",
            "Aurora kann sein Fenster nicht öffnen: Auf diesem PC fehlt Microsoft Edge WebView2, die Windows-Komponente, mit der es gezeichnet wird (oder sie funktioniert nicht).\n\nSoll Aurora es jetzt installieren? Das kostenlose Installationsprogramm von Microsoft öffnet sich, danach startet Aurora von selbst neu.",
        ),
        (
            "Aurora : WebView2 est absent",
            "Aurora ne peut pas ouvrir sa fenêtre : Microsoft Edge WebView2, le composant de Windows qui la dessine, est absent de ce PC (ou ne fonctionne pas).\n\nAurora doit-elle l'installer maintenant ? L'installation gratuite de Microsoft s'ouvre et, à la fin, Aurora se rouvre toute seule.",
        ),
        (
            "Aurora: falta WebView2",
            "Aurora no puede abrir su ventana: en este PC falta Microsoft Edge WebView2, el componente de Windows con el que la dibuja (o no funciona).\n\n¿Quieres que Aurora lo instale ahora? Se abre el instalador gratuito de Microsoft y, al terminar, Aurora se vuelve a abrir sola.",
        ),
        (
            "Aurora: falta o WebView2",
            "O Aurora não consegue abrir a sua janela: neste PC falta o Microsoft Edge WebView2, o componente do Windows com que a desenha (ou não funciona).\n\nQueres que o Aurora o instale agora? Abre-se o instalador gratuito da Microsoft e, no fim, o Aurora volta a abrir sozinho.",
        ),
    ];
    ALL[lang.min(ALL.len() - 1)]
}

/// Messaggio di Windows con «Sì» e «No»; `true` se si sceglie «Sì».
#[cfg(windows)]
fn ask_yes_no(title: &str, text: &str) -> bool {
    use windows_sys::Win32::UI::WindowsAndMessaging::{IDYES, MB_ICONWARNING, MB_SETFOREGROUND, MB_YESNO, MessageBoxW};
    let wide = |s: &str| s.encode_utf16().chain(std::iter::once(0)).collect::<Vec<u16>>();
    let (title, text) = (wide(title), wide(text));
    // SAFETY: le due stringhe sono UTF-16 terminate da zero e vivono fino alla fine della chiamata; nessuna finestra proprietaria.
    let answer = unsafe { MessageBoxW(std::ptr::null_mut(), text.as_ptr(), title.as_ptr(), MB_YESNO | MB_ICONWARNING | MB_SETFOREGROUND) };
    answer == IDYES
}

#[cfg(all(test, windows))]
mod tests {
    use super::texts;

    #[test]
    fn six_languages_and_a_fallback() {
        for lang in 0..6 {
            let (title, text) = texts(lang);
            assert!(title.contains("WebView2") && text.contains("WebView2"));
        }
        assert_eq!(texts(99), texts(5));
    }
}
