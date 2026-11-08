//! Locatori Maidenhead, distanze e prefissi WPX per i punti e i moltiplicatori.

/// Un locatore di 4 o 6 caratteri valido (`IO91`, `IO91wm`).
pub fn grid_valido(g: &str) -> bool {
    let b = g.as_bytes();
    let base = b.len() >= 4
        && b[..2].iter().all(|c| (b'A'..=b'R').contains(&c.to_ascii_uppercase()))
        && b[2..4].iter().all(u8::is_ascii_digit);
    match b.len() {
        4 => base && !g.eq_ignore_ascii_case("RR73"),
        6 => base && b[4..].iter().all(|c| (b'A'..=b'X').contains(&c.to_ascii_uppercase())),
        _ => false,
    }
}

/// Il centro di un locatore (latitudine, longitudine in gradi): del quadrato se ha 4 caratteri, del sottoquadrato se ne ha 6.
pub fn centro(g: &str) -> Option<(f64, f64)> {
    if !grid_valido(g) {
        return None;
    }
    let b = g.to_ascii_uppercase().into_bytes();
    let mut lon = f64::from(b[0] - b'A') * 20.0 - 180.0 + f64::from(b[2] - b'0') * 2.0;
    let mut lat = f64::from(b[1] - b'A') * 10.0 - 90.0 + f64::from(b[3] - b'0');
    if b.len() == 6 {
        lon += f64::from(b[4] - b'A') * (2.0 / 24.0) + 1.0 / 24.0;
        lat += f64::from(b[5] - b'A') * (1.0 / 24.0) + 0.5 / 24.0;
    } else {
        lon += 1.0;
        lat += 0.5;
    }
    Some((lat, lon))
}

/// Angolo al centro della Terra fra due punti, in gradi (cammino corto).
pub fn angolo_gradi(a: (f64, f64), b: (f64, f64)) -> f64 {
    let (la1, lo1, la2, lo2) = (a.0.to_radians(), a.1.to_radians(), b.0.to_radians(), b.1.to_radians());
    let h = ((la2 - la1) / 2.0).sin().powi(2) + la1.cos() * la2.cos() * ((lo2 - lo1) / 2.0).sin().powi(2);
    (2.0 * h.sqrt().min(1.0).asin()).to_degrees()
}

/// Distanza in km fra i centri di due locatori (raggio medio 6371 km). `None` se uno dei due non e' valido.
pub fn distanza_km(a: &str, b: &str) -> Option<f64> {
    Some(angolo_gradi(centro(a)?, centro(b)?).to_radians() * 6371.0)
}

/// Il campo (prime due lettere) di un locatore, maiuscolo.
pub fn campo(g: &str) -> Option<String> {
    grid_valido(g).then(|| g[..2].to_ascii_uppercase())
}

/// Il quadrato (4 caratteri) di un locatore, maiuscolo.
pub fn quadrato(g: &str) -> Option<String> {
    grid_valido(g).then(|| g[..4].to_ascii_uppercase())
}

fn suffisso(p: &str) -> bool {
    matches!(p, "P" | "M" | "MM" | "AM" | "QRP" | "R" | "A" | "LH") || (p.len() == 1 && p.chars().all(|c| c.is_ascii_digit()))
}

/// Il prefisso WPX di un nominativo: lettere e cifre fino all'ultima cifra del prefisso (`DL1ABC` -> `DL1`, `2E0ABC` -> `2E0`);
/// con un prefisso davanti si usa quello (`KH6/W1AW` -> `KH6`, `F/DL1ABC` -> `F0`).
pub fn prefisso_wpx(call: &str) -> String {
    let up = call.trim().to_ascii_uppercase();
    let parti: Vec<&str> = up.split('/').filter(|p| !p.is_empty() && !suffisso(p)).collect();
    let (core, davanti) = match parti.as_slice() {
        // la parte piu' corta e' il prefisso di dove si opera (`KH6/W1AW`, `N8BJQ/KH9`)
        [a, b] if a.len() < b.len() => (*a, true),
        [_, b] => (*b, true),
        [a] => (*a, false),
        _ => return String::new(),
    };
    if davanti {
        return if core.chars().any(|c| c.is_ascii_digit()) { core.to_string() } else { format!("{core}0") };
    }
    let b = core.as_bytes();
    // l'ultima cifra seguita solo da lettere
    let mut fine = None;
    for (i, c) in b.iter().enumerate() {
        if c.is_ascii_digit() && b[i + 1..].iter().all(u8::is_ascii_alphabetic) {
            fine = Some(i);
            break;
        }
    }
    match fine {
        Some(i) => core[..=i].to_string(),
        None => core.to_string(),
    }
}

/// La banda come la vuole il Cabrillo VHF (`50`, `70`, `144`, `222`, `432`, `902`, `1.2G`...).
pub fn banda_cabrillo(banda: &str) -> String {
    match banda.to_ascii_lowercase().as_str() {
        "6m" => "50",
        "4m" => "70",
        "2m" => "144",
        "1.25m" => "222",
        "70cm" => "432",
        "33cm" => "902",
        "23cm" => "1.2G",
        "13cm" => "2.3G",
        "160m" => "1800",
        "80m" => "3500",
        "40m" => "7000",
        "20m" => "14000",
        "15m" => "21000",
        "10m" => "28000",
        _ => "",
    }
    .to_string()
}

/// La banda come la vuole l'EDI (`PBand`).
pub fn banda_edi(banda: &str) -> String {
    match banda.to_ascii_lowercase().as_str() {
        "6m" => "50 MHz",
        "4m" => "70 MHz",
        "2m" => "145 MHz",
        "70cm" => "435 MHz",
        "23cm" => "1,3 GHz",
        "13cm" => "2,3 GHz",
        other => return other.to_string(),
    }
    .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Esempio ufficiale del WW Digi: circa 5541 km.
    #[test]
    fn distanze() {
        let d = distanza_km("FN42", "JO22").expect("km");
        assert!(d > 5000.0 && d < 6000.0, "{d}");
        assert_eq!(distanza_km("FN42", "FN42").map(|d| d.round()), Some(0.0));
        assert!(distanza_km("XX99", "FN42").is_none());
    }

    #[test]
    fn prefissi() {
        assert_eq!(prefisso_wpx("DL1ABC"), "DL1");
        assert_eq!(prefisso_wpx("IU3ABC/P"), "IU3");
        assert_eq!(prefisso_wpx("2E0ABC"), "2E0");
        assert_eq!(prefisso_wpx("KH6/W1AW"), "KH6");
        assert_eq!(prefisso_wpx("F/DL1ABC"), "F0");
        assert_eq!(prefisso_wpx("W1AW"), "W1");
    }

    #[test]
    fn campi_e_quadrati() {
        assert_eq!(campo("io91wm").as_deref(), Some("IO"));
        assert_eq!(quadrato("IO91wm").as_deref(), Some("IO91"));
        assert_eq!(campo("RR73"), None);
    }
}
