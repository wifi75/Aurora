//! Filtro dei falsi decode: prima di rispondere a un CQ si scarta cio' che ha l'aria di un errore di decodifica.
//! Un falso decode inventa nominativi che non esistono e far trasmettere la stazione verso un fantasma e' tempo e aria persi.
//!
//! Regole (idee del confronto con Decodium, scritte da zero): il motore segna con «?» le decodifiche incerte; il nominativo deve
//! avere la forma dei nominativi (prefisso, cifra, suffisso di lettere); un segnale molto debole di una stazione mai sentita prima
//! e' sospetto. Vale solo per decidere se rispondere: l'elenco dei messaggi non cambia.

/// Sotto questo rapporto (dB), una stazione mai sentita prima non e' credibile.
pub const WEAK_UNSEEN_SNR: i32 = -23;

fn alnum(s: &str) -> bool {
    !s.is_empty() && s.chars().all(|c| c.is_ascii_uppercase() || c.is_ascii_digit())
}

/// Un nominativo senza barre: da 1 a 3 caratteri di prefisso, una cifra, da 1 a 4 lettere («AA1AA», «3Y0J», «VP8DIF», «9A1A»).
fn plain_call(call: &str) -> bool {
    let b = call.as_bytes();
    if !alnum(call) || !(3..=8).contains(&b.len()) {
        return false;
    }
    b.iter().enumerate().any(|(i, c)| {
        c.is_ascii_digit() && (1..=3).contains(&i) && (1..=4).contains(&(b.len() - i - 1)) && b[i + 1..].iter().all(u8::is_ascii_uppercase)
    })
}

/// Il nominativo ha una forma credibile? Con le barre («F/AA1AA», «AA1AA/P», «AA1AA/QRP») basta una parte che lo sia e le altre brevi.
pub fn plausible_call(call: &str) -> bool {
    let call = call.trim().to_uppercase();
    if !call.contains('/') {
        return plain_call(&call);
    }
    let parts: Vec<&str> = call.split('/').collect();
    parts.len() <= 3 && parts.iter().all(|p| alnum(p) && p.len() <= 8) && parts.iter().any(|p| plain_call(p)) && parts.iter().filter(|p| !plain_call(p)).all(|p| p.len() <= 4)
}

/// Perche' una decodifica e' sospetta, se lo e'. `heard_before` = la stazione e' gia' stata sentita in questa sessione.
pub fn suspicion(text: &str, call: &str, snr: i32, heard_before: bool) -> Option<&'static str> {
    if text.trim_end().ends_with('?') {
        return Some("decodifica incerta (segnata con «?» dal motore)");
    }
    if !plausible_call(call) {
        return Some("il nominativo non ha la forma di un nominativo");
    }
    if snr < WEAK_UNSEEN_SNR && !heard_before {
        return Some("segnale molto debole di una stazione mai sentita prima: possibile falso decode");
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn real_world_callsigns_are_plausible() {
        for call in ["AA1AA", "IU3ABC", "JA1XYZ", "K1A", "W1AW", "3Y0J", "VP8DIF", "9A1A", "4X4AA", "E51ABC", "VK9XX", "3DA0XY", "R2AAA", "HB9ABC", "F/AA1AA", "AA1AA/P", "AA1AA/QRP", "IU3ABC/MM", "PA/DL1AB/P"] {
            assert!(plausible_call(call), "{call}");
        }
    }

    #[test]
    fn things_that_are_not_callsigns_are_not() {
        for call in ["", "CQ", "ABCDE", "12345", "A1", "AAAA1", "AA1AAAAA", "AA1A1A", "1A1AA1", "AA1AA/", "/AA1AA", "A/B/C/D", "AA1AA/LONGSUFFIX", "AA 1AA", "aa1aa$"] {
            assert!(!plausible_call(call), "{call}");
        }
    }

    #[test]
    fn a_question_mark_marks_an_uncertain_decode() {
        assert!(suspicion("CQ AA1AA FN42 ?", "AA1AA", -10, true).is_some());
        assert!(suspicion("CQ AA1AA FN42", "AA1AA", -10, true).is_none());
    }

    #[test]
    fn a_weak_signal_is_suspect_only_from_a_station_never_heard() {
        assert!(suspicion("CQ AA1AA FN42", "AA1AA", -24, false).is_some());
        assert!(suspicion("CQ AA1AA FN42", "AA1AA", -24, true).is_none());
        assert!(suspicion("CQ AA1AA FN42", "AA1AA", -23, false).is_none(), "il limite e' sotto -23");
    }

    #[test]
    fn an_implausible_call_is_refused_whatever_the_signal() {
        assert!(suspicion("CQ ABCDE FN42", "ABCDE", 5, true).is_some());
    }
}
