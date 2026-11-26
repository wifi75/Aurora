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
    let lang = crate::agent::ui_lang8();
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

/// La finestra principale non si e' aperta nemmeno dopo i tentativi (WebView2 e' registrato ma la sua creazione fallisce, per esempio con
/// 0x80070578 «identificatore della finestra non valido», rapporti del 3, 5 e 19/11 e panic della 0.11.300 a due secondi dall'avvio): prima
/// Tauri si fermava con un panic senza dire niente. Ora un messaggio spiega cosa provare (riavviare Windows, aggiornare o reinstallare WebView2)
/// e, con «Sì», Aurora reinstalla WebView2 e si riapre da sola; con «No» si chiude. Il chiamante esce subito dopo.
#[cfg(windows)]
pub fn window_failed(error: &str) {
    eprintln!("finestra principale non aperta: {error}");
    let lang = crate::agent::ui_lang8();
    let (title, text) = failed_texts(lang);
    let text = text.replace("{error}", error);
    if ask_yes_no(title, &text)
        && let Err(e) = install_and_restart()
    {
        eprintln!("{e}");
        crate::links::open_in_browser(DOWNLOAD_URL);
    }
}

#[cfg(not(windows))]
pub fn window_failed(error: &str) {
    eprintln!("finestra principale non aperta: {error}");
}

/// Titolo e testo del messaggio «la finestra non si apre» nelle otto lingue di Aurora (0 it, 1 en, 2 de, 3 fr, 4 es, 5 pt, 6 nl, 7 ru).
#[cfg(windows)]
fn failed_texts(lang: usize) -> (&'static str, &'static str) {
    const ALL: [(&str, &str); 8] = [
        (
            "Aurora: la finestra non si apre",
            "Aurora non riesce ad aprire la sua finestra. Windows ha risposto: {error}

Di solito basta riavviare Windows. Se non basta, spesso il problema è Microsoft Edge WebView2 (il componente con cui Aurora disegna la finestra): aggiornare Windows o reinstallare WebView2 lo risolve.

Vuoi che Aurora reinstalli WebView2 adesso e si riapra da sola? Con «No» Aurora si chiude.",
        ),
        (
            "Aurora: the window does not open",
            "Aurora cannot open its window. Windows answered: {error}

Restarting Windows is usually enough. If it is not, the problem is often Microsoft Edge WebView2 (the component Aurora uses to draw its window): updating Windows or reinstalling WebView2 fixes it.

Should Aurora reinstall WebView2 now and reopen by itself? With \"No\" Aurora closes.",
        ),
        (
            "Aurora: das Fenster öffnet sich nicht",
            "Aurora kann sein Fenster nicht öffnen. Windows hat geantwortet: {error}

Meist genügt ein Neustart von Windows. Wenn nicht, liegt es oft an Microsoft Edge WebView2 (der Komponente, mit der Aurora das Fenster zeichnet): Windows aktualisieren oder WebView2 neu installieren behebt es.

Soll Aurora WebView2 jetzt neu installieren und sich selbst wieder öffnen? Mit „Nein“ schließt sich Aurora.",
        ),
        (
            "Aurora : la fenêtre ne s'ouvre pas",
            "Aurora ne peut pas ouvrir sa fenêtre. Windows a répondu : {error}

Redémarrer Windows suffit en général. Sinon, le problème vient souvent de Microsoft Edge WebView2 (le composant avec lequel Aurora dessine sa fenêtre) : mettre Windows à jour ou réinstaller WebView2 le résout.

Aurora doit-elle réinstaller WebView2 maintenant et se rouvrir toute seule ? Avec « Non », Aurora se ferme.",
        ),
        (
            "Aurora: la ventana no se abre",
            "Aurora no puede abrir su ventana. Windows respondió: {error}

Normalmente basta con reiniciar Windows. Si no, el problema suele ser Microsoft Edge WebView2 (el componente con el que Aurora dibuja su ventana): actualizar Windows o reinstalar WebView2 lo resuelve.

¿Quieres que Aurora reinstale WebView2 ahora y se vuelva a abrir sola? Con «No» Aurora se cierra.",
        ),
        (
            "Aurora: a janela não abre",
            "O Aurora não consegue abrir a sua janela. O Windows respondeu: {error}

Normalmente basta reiniciar o Windows. Se não bastar, o problema costuma ser o Microsoft Edge WebView2 (o componente com que o Aurora desenha a janela): atualizar o Windows ou reinstalar o WebView2 resolve.

Queres que o Aurora reinstale o WebView2 agora e volte a abrir sozinho? Com «Não» o Aurora fecha.",
        ),
        (
            "Aurora: het venster gaat niet open",
            "Aurora kan zijn venster niet openen. Windows antwoordde: {error}

Meestal volstaat het om Windows opnieuw op te starten. Zo niet, dan is het probleem vaak Microsoft Edge WebView2 (het onderdeel waarmee Aurora het venster tekent): Windows bijwerken of WebView2 opnieuw installeren lost het op.

Moet Aurora WebView2 nu opnieuw installeren en zichzelf weer openen? Met «Nee» sluit Aurora zich af.",
        ),
        (
            "Aurora: окно не открывается",
            "Aurora не может открыть своё окно. Windows ответила: {error}

Обычно достаточно перезагрузить Windows. Если нет, проблема часто в Microsoft Edge WebView2 (компоненте, которым Aurora рисует окно): обновление Windows или переустановка WebView2 это исправляет.

Переустановить WebView2 сейчас, чтобы Aurora открылась снова сама? При выборе «Нет» Aurora закроется.",
        ),
    ];
    ALL[if lang < ALL.len() { lang } else { 1 }]
}

/// Titolo e testo del messaggio nelle otto lingue di Aurora (0 it, 1 en, 2 de, 3 fr, 4 es, 5 pt, 6 nl, 7 ru).
#[cfg(windows)]
fn texts(lang: usize) -> (&'static str, &'static str) {
    const ALL: [(&str, &str); 8] = [
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
        (
            "Aurora: WebView2 ontbreekt",
            "Aurora kan zijn venster niet openen: op deze pc ontbreekt Microsoft Edge WebView2, het Windows-onderdeel waarmee het wordt getekend (of het werkt niet).

Moet Aurora het nu installeren? Het gratis installatieprogramma van Microsoft opent en daarna opent Aurora zichzelf weer.",
        ),
        (
            "Aurora: WebView2 отсутствует",
            "Aurora не может открыть своё окно: на этом ПК отсутствует Microsoft Edge WebView2, компонент Windows, которым оно рисуется (или он не работает).

Установить его сейчас? Откроется бесплатный установщик Microsoft, и после него Aurora откроется снова сама.",
        ),
    ];
    ALL[if lang < ALL.len() { lang } else { 1 }]
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
    fn eight_languages_and_a_fallback() {
        for lang in 0..8 {
            let (title, text) = texts(lang);
            assert!(title.contains("WebView2") && text.contains("WebView2"));
        }
        assert_eq!(texts(99), texts(1), "una lingua fuori elenco vale l'inglese");
    }

    #[test]
    fn the_window_failed_message_has_eight_languages_and_the_error_placeholder() {
        for lang in 0..8 {
            let (title, text) = super::failed_texts(lang);
            assert!(!title.is_empty() && text.contains("{error}") && text.contains("WebView2"));
        }
        assert_eq!(super::failed_texts(99), super::failed_texts(1));
    }
}
