//! Lettura dei messaggi FT8/FT4 e confronto dei nominativi.
//!
//! La forma dei messaggi e' quella che JTDX interpreta in `processMessage` e `DecodedText`:
//! `CQ [mod] CALL [GRID]` oppure `DESTINATARIO MITTENTE [GRIGLIA | -07 | R-07 | RRR | RR73 | 73]`.

use crate::report::{parse_r_report, parse_report};

/// Il terzo campo di un messaggio diretto.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Body {
    /// Locatore di 4 caratteri: la risposta a un CQ (Tx1).
    Grid(String),
    /// Rapporto semplice, «-07» (Tx2).
    Report(i32),
    /// Rapporto con conferma, «R-07» (Tx3).
    RReport(i32),
    Rrr,
    Rr73,
    /// «73»
    Bye,
    /// Qualunque altra cosa (testo libero, campo assente).
    Other,
}

/// Cio' che si capisce da un messaggio decodificato.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Message {
    Cq {
        modifier: Option<String>,
        call: String,
        grid: Option<String>,
    },
    Directed {
        to: String,
        from: String,
        body: Body,
    },
    /// Non interpretabile, oppure con un nominativo nascosto (`<...>`).
    Other,
}

/// Legge un messaggio. Il testo e' reso maiuscolo; i nominativi tra `<>` perdono le parentesi.
pub fn parse(text: &str) -> Message {
    let tokens: Vec<String> = text
        .split_whitespace()
        .map(str::to_ascii_uppercase)
        .collect();
    let Some(first) = tokens.first() else {
        return Message::Other;
    };
    if first == "CQ" || first == "QRZ" {
        return parse_cq(&tokens);
    }
    if tokens.len() < 2 || tokens.len() > 3 {
        return Message::Other;
    }
    let (Some(to), Some(from)) = (call_token(&tokens[0]), call_token(&tokens[1])) else {
        return Message::Other;
    };
    let body = tokens.get(2).map_or(Body::Other, |t| parse_body(t));
    Message::Directed { to, from, body }
}

fn parse_cq(tokens: &[String]) -> Message {
    let mut idx = 1;
    let mut modifier = None;
    // un modificatore (DX, NA, POTA, 000...) precede il nominativo
    if let Some(t) = tokens.get(idx)
        && is_modifier(t)
        && tokens.get(idx + 1).is_some()
    {
        modifier = Some(t.clone());
        idx += 1;
    }
    let Some(call) = tokens.get(idx).and_then(|t| call_token(t)) else {
        return Message::Other;
    };
    let grid = tokens.get(idx + 1).filter(|t| is_grid(t)).cloned();
    if tokens.len() > idx + 2 {
        return Message::Other;
    }
    Message::Cq {
        modifier,
        call,
        grid,
    }
}

fn parse_body(token: &str) -> Body {
    if is_grid(token) {
        return Body::Grid(token.to_string());
    }
    match token {
        "RRR" => Body::Rrr,
        "RR73" => Body::Rr73,
        "73" => Body::Bye,
        _ => {
            if let Some(n) = parse_report(token) {
                Body::Report(n)
            } else if let Some(n) = parse_r_report(token) {
                Body::RReport(n)
            } else {
                Body::Other
            }
        }
    }
}

/// Il nominativo di un campo, senza `<>`; `None` per `<...>` (nascosto) o se non e' un nominativo.
fn call_token(token: &str) -> Option<String> {
    let t = token.trim_start_matches('<').trim_end_matches('>');
    (is_callsign(t)).then(|| t.to_string())
}

/// Locatore di 4 caratteri (`AA00`..`RR99`); «RR73» ha la stessa forma ma e' un saluto.
pub fn is_grid(token: &str) -> bool {
    let b = token.as_bytes();
    token != "RR73"
        && b.len() == 4
        && b[..2].iter().all(|c| (b'A'..=b'R').contains(c))
        && b[2..].iter().all(u8::is_ascii_digit)
}

/// Locatore valido di 4 o 6 caratteri (`JN45` o `JN45ab`), maiuscole o minuscole.
pub fn is_valid_locator(grid: &str) -> bool {
    let g = grid.to_ascii_uppercase();
    let b = g.as_bytes();
    match b.len() {
        4 => is_grid(&g),
        6 => is_grid(&g[..4]) && b[4..].iter().all(|c| (b'A'..=b'X').contains(c)),
        _ => false,
    }
}

fn is_callsign(token: &str) -> bool {
    (3..=13).contains(&token.len())
        && token
            .chars()
            .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '/')
        && token.chars().any(|c| c.is_ascii_digit())
        && token.chars().any(|c| c.is_ascii_uppercase())
        && token != "RR73"
        && !is_grid(token)
}

/// Un nominativo accettabile per la propria stazione o per il DX.
pub fn is_valid_call(call: &str) -> bool {
    is_callsign(&call.to_ascii_uppercase())
}

/// Modificatori del CQ: da 1 a 4 lettere (DX, NA, EU, POTA, SOTA, TEST...) oppure tre cifre.
pub fn is_modifier(token: &str) -> bool {
    ((1..=4).contains(&token.len()) && token.chars().all(|c| c.is_ascii_uppercase()))
        || (token.len() == 3 && token.chars().all(|c| c.is_ascii_digit()))
}

/// Suffissi che non fanno parte del nominativo di base.
fn is_suffix(part: &str) -> bool {
    matches!(part, "P" | "R" | "M" | "A" | "QRP" | "MM" | "AM" | "LH")
        || (part.len() == 1 && part.chars().all(|c| c.is_ascii_digit()))
}

/// Nominativo senza prefisso o suffisso: `DL/AA1AA` e `AA1AA/P` danno `AA1AA` (come `Radio::base_callsign` di JTDX).
pub fn base_call(call: &str) -> String {
    let up = call.to_ascii_uppercase();
    let parts: Vec<&str> = up.split('/').filter(|p| !p.is_empty()).collect();
    let core = parts
        .iter()
        .filter(|p| !is_suffix(p))
        .max_by_key(|p| p.len());
    match (core, parts.first()) {
        (Some(c), _) => (*c).to_string(),
        (None, Some(f)) => (*f).to_string(),
        (None, None) => String::new(),
    }
}

/// Stesso operatore: i nominativi hanno lo stesso nominativo di base.
pub fn same_station(a: &str, b: &str) -> bool {
    let (a, b) = (base_call(a), base_call(b));
    !a.is_empty() && a == b
}

/// Nominativo «standard» per l'FT8: l'espressione di `MainWindow::stdCall` di JTDX, cioe'
/// prefisso (`[A-Z]{0,2}`, `[A-Z][0-9]` o `[0-9][A-Z]`), cifra e fino a tre lettere, con `/R` o `/P` finali.
pub fn is_std_call(call: &str) -> bool {
    let up = call.trim().to_ascii_uppercase();
    let core = up
        .strip_suffix("/R")
        .or_else(|| up.strip_suffix("/P"))
        .unwrap_or(&up);
    let b = core.as_bytes();
    let letter = |c: u8| c.is_ascii_uppercase();
    let digit = |c: u8| c.is_ascii_digit();
    (0..=2usize).any(|n| {
        if b.len() < n {
            return false;
        }
        let (pre, rest) = b.split_at(n);
        let pre_ok = match pre {
            [] => true,
            [a] => letter(*a),
            [a, c] => {
                (letter(*a) && letter(*c)) || (letter(*a) && digit(*c)) || (digit(*a) && letter(*c))
            }
            _ => false,
        };
        let rest_ok = match rest.split_first() {
            Some((d, tail)) => digit(*d) && tail.len() <= 3 && tail.iter().all(|c| letter(*c)),
            None => false,
        };
        pre_ok && rest_ok
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn s(x: &str) -> String {
        x.to_string()
    }

    /// `DecodedText`/`processMessage`: la forma `CQ [mod] CALL [GRID]`.
    #[test]
    fn cq_forms() {
        assert_eq!(
            parse("CQ LZ2KSQ KN23"),
            Message::Cq {
                modifier: None,
                call: s("LZ2KSQ"),
                grid: Some(s("KN23"))
            }
        );
        assert_eq!(
            parse("CQ DX AA1AA FN42"),
            Message::Cq {
                modifier: Some(s("DX")),
                call: s("AA1AA"),
                grid: Some(s("FN42"))
            }
        );
        assert_eq!(
            parse("CQ POTA AA1AA/P"),
            Message::Cq {
                modifier: Some(s("POTA")),
                call: s("AA1AA/P"),
                grid: None
            }
        );
        assert_eq!(
            parse("cq 000 aa1aa fn42"),
            Message::Cq {
                modifier: Some(s("000")),
                call: s("AA1AA"),
                grid: Some(s("FN42"))
            }
        );
        assert_eq!(
            parse("QRZ AA1AA"),
            Message::Cq {
                modifier: None,
                call: s("AA1AA"),
                grid: None
            }
        );
    }

    /// `processMessage`: `r.left(3)=="RRR"`, `RR73`, `73`, `R...` e il rapporto da -50 a 49.
    #[test]
    fn directed_bodies() {
        let d = |t: &str| match parse(t) {
            Message::Directed { to, from, body } => (to, from, body),
            other => panic!("atteso un messaggio diretto, trovato {other:?}"),
        };
        assert_eq!(
            d("IU3ABC LZ2KSQ KN23"),
            (s("IU3ABC"), s("LZ2KSQ"), Body::Grid(s("KN23")))
        );
        assert_eq!(d("IU3ABC LZ2KSQ -12").2, Body::Report(-12));
        assert_eq!(d("IU3ABC LZ2KSQ +05").2, Body::Report(5));
        assert_eq!(d("IU3ABC LZ2KSQ R-10").2, Body::RReport(-10));
        assert_eq!(d("IU3ABC LZ2KSQ RRR").2, Body::Rrr);
        assert_eq!(d("IU3ABC LZ2KSQ RR73").2, Body::Rr73);
        assert_eq!(d("IU3ABC LZ2KSQ 73").2, Body::Bye);
        assert_eq!(d("IU3ABC LZ2KSQ").2, Body::Other);
        assert_eq!(d("IU3ABC LZ2KSQ HELLO").2, Body::Other);
    }

    #[test]
    fn hashed_and_garbage_are_other() {
        assert_eq!(parse("<...> LZ2KSQ -12"), Message::Other);
        assert_eq!(parse("IU3ABC <...> -12"), Message::Other);
        assert_eq!(parse(""), Message::Other);
        assert_eq!(parse("TNX 73 GL"), Message::Other);
        assert_eq!(parse("A B C D"), Message::Other);
    }

    #[test]
    fn hashed_call_in_brackets_is_resolved() {
        assert_eq!(
            parse("<DL/AA1AA> IU3ABC RR73"),
            Message::Directed {
                to: s("DL/AA1AA"),
                from: s("IU3ABC"),
                body: Body::Rr73
            }
        );
    }

    /// `Radio::base_callsign`: prefisso e suffisso non contano.
    #[test]
    fn base_call_strips_prefix_and_suffix() {
        assert_eq!(base_call("AA1AA"), "AA1AA");
        assert_eq!(base_call("AA1AA/P"), "AA1AA");
        assert_eq!(base_call("aa1aa/qrp"), "AA1AA");
        assert_eq!(base_call("DL/AA1AA"), "AA1AA");
        assert_eq!(base_call("VP9/W1AW"), "W1AW");
        assert_eq!(base_call("AA1AA/7"), "AA1AA");
        assert!(same_station("IU3ABC/P", "IU3ABC"));
        assert!(same_station("DL/IU3ABC", "IU3ABC/R"));
        assert!(!same_station("IU3ABC", "IU3ABD"));
        assert!(!same_station("", ""));
    }

    /// `MainWindow::stdCall`.
    #[test]
    fn std_call_matches_jtdx_regex() {
        for c in [
            "IU3ABC", "AA1AA", "K1ABC", "W1AW", "9A1A", "IU3ABC/P", "IU3ABC/R", "PA3XYZ", "3D2AB",
        ] {
            assert!(is_std_call(c), "{c} dovrebbe essere standard");
        }
        for c in [
            "DL/AA1AA",
            "AA1AA/QRP",
            "IU3ABCDE",
            "ABC",
            "123",
            "",
            "AA1AA/7",
        ] {
            assert!(!is_std_call(c), "{c} non dovrebbe essere standard");
        }
    }

    #[test]
    fn locators() {
        assert!(is_valid_locator("JN45"));
        assert!(is_valid_locator("jn45rk"));
        assert!(!is_valid_locator("JN4"));
        assert!(!is_valid_locator("ZZ99"));
        assert!(!is_valid_locator("RR73"));
        assert!(!is_valid_locator("JN45YY"));
    }
}
