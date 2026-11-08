//! I modi che si scelgono con i pulsanti di Aurora.

/// Identificativo del pulsante di un modo; `None` per qualunque altro testo.
pub fn button_id(mode: &str) -> Option<&'static str> {
    match mode.trim().to_uppercase().as_str() {
        "FT8" => Some("ft8Button"),
        "FT4" => Some("ft4Button"),
        // JT65 e JT9 stanno dietro al pulsante JT (il motore li distingue)
        "JT" | "JT65" | "JT9" => Some("jtButton"),
        // JT4 (sottomodo A) solo con il motore proprio
        "JT4" | "JT4A" | "JT4B" | "JT4C" | "JT4D" | "JT4E" | "JT4F" | "JT4G" => Some("jt4Button"),
        // Q65-30A, solo con il motore proprio
        "Q65" | "Q65-30A" => Some("q65Button"),
        _ => None,
    }
}
