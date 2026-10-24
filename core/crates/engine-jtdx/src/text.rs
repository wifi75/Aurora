/// Cio' che si riesce a leggere in un messaggio FT8/FT4.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Parsed {
    /// Nominativo di chi trasmette (maiuscolo), se riconoscibile.
    pub sender: Option<String>,
    /// Locatore di 4 caratteri (maiuscolo), se presente.
    pub grid: Option<String>,
    pub is_cq: bool,
}

pub fn parse_message(text: &str) -> Parsed {
    let tokens: Vec<String> = text.split_whitespace().map(str::to_uppercase).collect();
    let Some(first) = tokens.first() else {
        return Parsed::default();
    };
    let is_cq = matches!(first.as_str(), "CQ" | "CQDX" | "QRZ");
    let sender = if is_cq {
        // dopo CQ: eventuali modificatori (DX, NA, POTA...) e poi il nominativo di chi chiama
        let mut found = None;
        for token in &tokens[1..] {
            if is_callsign(token) {
                found = Some(token.clone());
                break;
            }
            if !is_modifier(token) {
                break;
            }
        }
        found
    } else {
        // negli altri messaggi: "DESTINATARIO MITTENTE ..."
        tokens.get(1).filter(|t| is_callsign(t)).cloned()
    };
    let grid = tokens.last().filter(|t| is_grid(t)).cloned();
    Parsed { sender, grid, is_cq }
}

/// Locatore di 4 caratteri; "RR73" ha la stessa forma ma e' un saluto.
fn is_grid(token: &str) -> bool {
    let b = token.as_bytes();
    token != "RR73"
        && b.len() == 4
        && b[..2].iter().all(|c| (b'A'..=b'R').contains(c))
        && b[2..].iter().all(u8::is_ascii_digit)
}

fn is_callsign(token: &str) -> bool {
    (3..=13).contains(&token.len())
        && token.chars().all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '/')
        && token.chars().any(|c| c.is_ascii_digit())
        && token.chars().any(|c| c.is_ascii_uppercase())
        && token != "RR73"
        && !is_grid(token)
}

/// Modificatori del CQ: lettere sole, da 2 a 4 (DX, NA, EU, POTA, SOTA, TEST...).
fn is_modifier(token: &str) -> bool {
    (2..=4).contains(&token.len()) && token.chars().all(|c| c.is_ascii_uppercase())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(text: &str) -> (Option<String>, Option<String>, bool) {
        let r = parse_message(text);
        (r.sender, r.grid, r.is_cq)
    }

    fn some(s: &str) -> Option<String> {
        Some(s.to_string())
    }

    #[test]
    fn cq_plain() {
        assert_eq!(p("CQ AA1AA FN42"), (some("AA1AA"), some("FN42"), true));
        assert_eq!(p("CQ AA1AA"), (some("AA1AA"), None, true));
    }

    #[test]
    fn cq_with_modifier() {
        assert_eq!(p("CQ DX AA1AA FN42"), (some("AA1AA"), some("FN42"), true));
        assert_eq!(p("CQ NA AA1AA FN42"), (some("AA1AA"), some("FN42"), true));
        assert_eq!(p("CQ EU AA1AA"), (some("AA1AA"), None, true));
    }

    #[test]
    fn cq_pota_sota_test() {
        assert_eq!(p("CQ POTA AA1AA FN42"), (some("AA1AA"), some("FN42"), true));
        assert_eq!(p("CQ SOTA AA1AA FN42"), (some("AA1AA"), some("FN42"), true));
        assert_eq!(p("CQ TEST AA1AA FN42"), (some("AA1AA"), some("FN42"), true));
    }

    #[test]
    fn cqdx_and_qrz_forms() {
        assert_eq!(p("CQDX AA1AA FN42"), (some("AA1AA"), some("FN42"), true));
        assert_eq!(p("QRZ AA1AA"), (some("AA1AA"), None, true));
    }

    #[test]
    fn reply_with_report() {
        assert_eq!(p("BB2BB AA1AA -10"), (some("AA1AA"), None, false));
        assert_eq!(p("BB2BB AA1AA R-12"), (some("AA1AA"), None, false));
        assert_eq!(p("BB2BB AA1AA +05"), (some("AA1AA"), None, false));
    }

    #[test]
    fn reply_with_grid() {
        assert_eq!(p("BB2BB AA1AA FN42"), (some("AA1AA"), some("FN42"), false));
    }

    #[test]
    fn rr73_and_73_are_not_grids() {
        assert_eq!(p("BB2BB AA1AA RR73"), (some("AA1AA"), None, false));
        assert_eq!(p("BB2BB AA1AA 73"), (some("AA1AA"), None, false));
        assert_eq!(p("BB2BB AA1AA RRR"), (some("AA1AA"), None, false));
    }

    #[test]
    fn hashed_calls_are_not_callsigns() {
        assert_eq!(p("<...> AA1AA MP72"), (some("AA1AA"), some("MP72"), false));
        assert_eq!(p("BB2BB <...> -10"), (None, None, false));
        assert_eq!(p("CQ <...> FN42"), (None, some("FN42"), true));
    }

    #[test]
    fn compound_calls_are_kept_whole() {
        assert_eq!(p("CQ DL/AA1AA"), (some("DL/AA1AA"), None, true));
        assert_eq!(p("CQ AA1AA/P FN42"), (some("AA1AA/P"), some("FN42"), true));
    }

    #[test]
    fn lowercase_and_extra_spaces() {
        assert_eq!(p("  cq   aa1aa  fn42 "), (some("AA1AA"), some("FN42"), true));
    }

    #[test]
    fn garbage_and_free_text_give_nothing_useful() {
        assert_eq!(p(""), (None, None, false));
        assert_eq!(p("?"), (None, None, false));
        assert_eq!(p("TNX 73 GL"), (None, None, false));
    }
}
