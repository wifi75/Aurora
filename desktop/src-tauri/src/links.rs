//! Link esterni: la finestra del programma non apre nuove finestre, i link a siti (note di versione, guida) vanno nel browser di Windows
//! e i `mailto:` (pacchetto per l'assistenza) nel programma di posta.

use std::process::Command;

/// Solo indirizzi web veri e messaggi di posta: niente `file:`, `javascript:` o programmi.
pub fn is_external_web(url: &str) -> bool {
    let lower = url.to_ascii_lowercase();
    (lower.starts_with("https://") || lower.starts_with("http://") || lower.starts_with("mailto:")) && !url.contains(char::is_whitespace)
}

/// Apre l'indirizzo nel browser predefinito, senza passare da una shell (nessun carattere speciale conta).
pub fn open_in_browser(url: &str) {
    if !is_external_web(url) {
        return;
    }
    // la posta dell'assistenza (Gmail) si apre in Chrome se c'e', altrimenti nel browser predefinito
    if url.to_ascii_lowercase().starts_with("https://mail.google.com/")
        && let Some(chrome) = chrome_path()
        && Command::new(chrome).arg(url).spawn().is_ok()
    {
        return;
    }
    if let Err(e) = Command::new("rundll32").args(["url.dll,FileProtocolHandler", url]).spawn() {
        tracing::warn!("link non aperto: {e}");
    }
}

/// Dove sta Chrome: nelle cartelle dei programmi o in quella dell'utente.
fn chrome_path() -> Option<std::path::PathBuf> {
    ["ProgramFiles", "ProgramFiles(x86)", "LOCALAPPDATA"]
        .iter()
        .filter_map(|v| std::env::var_os(v))
        .map(|base| std::path::PathBuf::from(base).join("Google").join("Chrome").join("Application").join("chrome.exe"))
        .find(|p| p.is_file())
}

#[cfg(test)]
mod tests {
    use super::is_external_web;

    #[test]
    fn only_real_web_addresses_are_opened() {
        assert!(is_external_web("https://github.com/wifi75/Aurora/releases/tag/v0.10.3"));
        assert!(is_external_web("HTTP://example.test/a?b=1&c=2"));
        assert!(!is_external_web("file:///C:/Windows/System32/calc.exe"));
        assert!(!is_external_web("javascript:alert(1)"));
        assert!(!is_external_web("calc.exe"));
        assert!(!is_external_web("https://example.test/a b"));
        assert!(!is_external_web(""));
        assert!(is_external_web("mailto:aiuto@example.test?subject=Aurora%20assistenza"));
    }
}
