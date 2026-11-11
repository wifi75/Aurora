//! I modi che si scelgono con i pulsanti di Aurora.

/// «Q65-» seguito da un periodo di WSJT-X (15, 30, 60, 120, 300 s) e da un sottomodo da A a E.
fn is_q65_submode(m: &str) -> bool {
    m.strip_prefix("Q65-").is_some_and(|rest| {
        let (period, letter) = rest.split_at(rest.len().saturating_sub(1));
        matches!(period, "15" | "30" | "60" | "120" | "300") && matches!(letter, "A" | "B" | "C" | "D" | "E")
    })
}

/// «FST4-» seguito da un periodo di WSJT-X (15 ... 1800 s), «FST4W-» da uno di FST4W (120 ... 1800 s).
fn is_fst4_period(m: &str) -> bool {
    if let Some(p) = m.strip_prefix("FST4W-") {
        return matches!(p, "120" | "300" | "900" | "1800");
    }
    m.strip_prefix("FST4-").is_some_and(|p| matches!(p, "15" | "30" | "60" | "120" | "300" | "900" | "1800"))
}

/// Il modo come lo scrive la tabella delle frequenze: «Q65-60B» -> «Q65», «JT65B» -> «JT65» (periodo e sottomodo non cambiano la frequenza;
/// la tabella di WSJT-X ha righe JT65 senza sottomodo).
pub fn table_mode(mode: &str) -> String {
    let up = mode.trim().to_uppercase();
    if is_q65_submode(&up) {
        "Q65".to_string()
    } else if is_fst4_period(&up) {
        // «FST4-60» -> «FST4», «FST4W-120» -> «FST4W»: il periodo non cambia la frequenza
        up.split('-').next().unwrap_or("FST4").to_string()
    } else if matches!(up.as_str(), "JT65A" | "JT65B" | "JT65C") {
        "JT65".to_string()
    } else {
        up
    }
}

/// Identificativo del pulsante di un modo; `None` per qualunque altro testo.
pub fn button_id(mode: &str) -> Option<&'static str> {
    match mode.trim().to_uppercase().as_str() {
        "FT8" => Some("ft8Button"),
        "FT4" => Some("ft4Button"),
        // JT65 e JT9 stanno dietro al pulsante JT (il motore li distingue)
        "JT" | "JT65" | "JT9" => Some("jtButton"),
        // JT65A, JT65B, JT65C (sottomodo), solo con il motore proprio
        "JT65A" | "JT65B" | "JT65C" => Some("jtButton"),
        // JT4 (sottomodo A) solo con il motore proprio
        "JT4" | "JT4A" | "JT4B" | "JT4C" | "JT4D" | "JT4E" | "JT4F" | "JT4G" => Some("jt4Button"),
        // Q65 e Q65-15A ... Q65-300E (periodo e sottomodo), solo con il motore proprio
        "Q65" => Some("q65Button"),
        m if is_q65_submode(m) => Some("q65Button"),
        "MSK144" => Some("msk144Button"),
        // FST4 e FST4W, con o senza periodo («FST4-60», «FST4W-120»), solo con il motore proprio
        "FST4" => Some("fst4Button"),
        "FST4W" => Some("fst4wButton"),
        m if is_fst4_period(m) => Some(if m.starts_with("FST4W") { "fst4wButton" } else { "fst4Button" }),
        // WSPR-2 solo con il motore proprio
        "WSPR" => Some("wsprButton"),
        _ => None,
    }
}
