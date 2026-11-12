//! WebView2 (il motore con cui Windows disegna la finestra di Aurora) che manca o non si apre. Tauri allora si ferma con un panic
//! («Could not find the webview runtime») e Aurora ripartiva e ricadeva in pochi secondi, piu' volte di fila: lo hanno mandato i
//! rapporti del 2026-11-12 (Windows 10, sette crash in mezz'ora). L'installatore lo installa se manca, ma un aggiornamento o una
//! pulizia di Windows possono lasciarlo registrato e rotto. Qui lo si controlla prima di aprire la finestra: se manca, un messaggio
//! di Windows spiega cosa fare e porta al programma d'installazione di Microsoft, e Aurora si chiude senza crash.

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
    if ask_yes_no(title, text) {
        crate::links::open_in_browser(DOWNLOAD_URL);
    }
    false
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
            "Aurora non può aprire la sua finestra: su questo PC manca Microsoft Edge WebView2, il componente di Windows con cui la disegna (oppure non funziona).\n\nVuoi scaricare ora l'installazione gratuita di Microsoft? Dopo averla installata, riapri Aurora.",
        ),
        (
            "Aurora: WebView2 is missing",
            "Aurora cannot open its window: Microsoft Edge WebView2, the Windows component it uses to draw it, is missing on this PC (or is not working).\n\nDownload Microsoft's free installer now? After installing it, open Aurora again.",
        ),
        (
            "Aurora: WebView2 fehlt",
            "Aurora kann sein Fenster nicht öffnen: Auf diesem PC fehlt Microsoft Edge WebView2, die Windows-Komponente, mit der es gezeichnet wird (oder sie funktioniert nicht).\n\nJetzt das kostenlose Installationsprogramm von Microsoft herunterladen? Danach Aurora erneut öffnen.",
        ),
        (
            "Aurora : WebView2 est absent",
            "Aurora ne peut pas ouvrir sa fenêtre : Microsoft Edge WebView2, le composant de Windows qui la dessine, est absent de ce PC (ou ne fonctionne pas).\n\nTélécharger maintenant l'installation gratuite de Microsoft ? Une fois installée, rouvre Aurora.",
        ),
        (
            "Aurora: falta WebView2",
            "Aurora no puede abrir su ventana: en este PC falta Microsoft Edge WebView2, el componente de Windows con el que la dibuja (o no funciona).\n\n¿Descargar ahora el instalador gratuito de Microsoft? Después de instalarlo, vuelve a abrir Aurora.",
        ),
        (
            "Aurora: falta o WebView2",
            "O Aurora não consegue abrir a sua janela: neste PC falta o Microsoft Edge WebView2, o componente do Windows com que a desenha (ou não funciona).\n\nTransferir agora o instalador gratuito da Microsoft? Depois de o instalar, abre de novo o Aurora.",
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
