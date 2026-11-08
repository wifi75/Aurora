//! I modi che si scelgono con i pulsanti di Aurora.

/// «Q65-» seguito da un periodo di WSJT-X (15, 30, 60, 120, 300 s) e da un sottomodo da A a E.
fn is_q65_submode(m: &str) -> bool {
    m.strip_prefix("Q65-").is_some_and(|rest| {
        let (period, letter) = rest.split_at(rest.len().saturating_sub(1));
        matches!(period, "15" | "30" | "60" | "120" | "300") && matches!(letter, "A" | "B" | "C" | "D" | "E")
    })
}

/// Identificativo del pulsante di un modo; `None` per qualunque altro testo.
pub fn button_id(mode: &str) -> Option<&'static str> {
    match mode.trim().to_uppercase().as_str() {
        "FT8" => Some("ft8Button"),
        "FT4" => Some("ft4Button"),
        // JT65 e JT9 stanno dietro al pulsante JT (il motore li distingue)
        "JT" | "JT65" | "JT9" => Some("jtButton"),
        // JT4 (sottomodo A) solo con il motore proprio
        "JT4" | "JT4A" | "JT4B" | "JT4C" | "JT4D" | "JT4E" | "JT4F" | "JT4G" => Some("jt4Button"),
        // Q65 e Q65-15A ... Q65-300E (periodo e sottomodo), solo con il motore proprio
        "Q65" => Some("q65Button"),
        m if is_q65_submode(m) => Some("q65Button"),
        "MSK144" => Some("msk144Button"),
        _ => None,
    }
}
