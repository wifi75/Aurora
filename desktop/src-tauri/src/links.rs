//! Link esterni: la finestra del programma non apre nuove finestre, i link a siti (note di versione, guida) vanno nel browser di Windows.

use std::process::Command;

/// Solo indirizzi web veri: niente `file:`, `javascript:` o programmi.
pub fn is_external_web(url: &str) -> bool {
    let lower = url.to_ascii_lowercase();
    (lower.starts_with("https://") || lower.starts_with("http://")) && !url.contains(char::is_whitespace)
}

/// Apre l'indirizzo nel browser predefinito, senza passare da una shell (nessun carattere speciale conta).
pub fn open_in_browser(url: &str) {
    if !is_external_web(url) {
        return;
    }
    if let Err(e) = Command::new("rundll32").args(["url.dll,FileProtocolHandler", url]).spawn() {
        tracing::warn!("link non aperto: {e}");
    }
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
    }
}
