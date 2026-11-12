//! Le «forme» dei nominativi per paese DXCC: quanto e' comune la forma di un nominativo (prefisso letterale, numero di cifre e di lettere
//! del suffisso) fra quelli che Aurora conosce di quel paese (il log, gli spot del DX Cluster, `MASTER.SCP` se c'e').
//!
//! La forma di `IU3ABC` e' «IU» + una cifra + tre lettere; di `K1ABC` «K1» + una cifra + tre lettere (con un prefisso di una lettera sola
//! entra anche la cifra, cosi' il prefisso letterale ha sempre 2-3 caratteri); di `A61AB` «A6» + una cifra + due lettere. Con `EA8/IU3ABC`
//! o `IU3ABC/P` conta il nominativo di base (il paese lo dice il nominativo intero). La frequenza relativa della forma fra i nominativi di
//! quel paese e' la sua probabilita' a priori: «tipica» dall'1% in su, «insolita» fra lo 0,05% e l'1%, «sospetta» sotto (anche mai vista).
//! Un paese con pochi nominativi noti (meno di `MIN_SAMPLES`) non si giudica.
//!
//! Le serie di nominativi che l'ITU non ha assegnato a nessun paese (Q, quelle che cominciano con 0 o 1, e le coppie lettera-cifra non
//! assegnate come E8 o Z9) si scartano, salvo che `cty.dat` le conosca (per esempio i prefissi speciali di alcuni paesi DXCC).

use std::collections::HashMap;

/// Sotto tanti nominativi noti di un paese la forma non si giudica.
pub const MIN_SAMPLES: u32 = 200;
/// Le soglie delle forme: «tipica» e «insolita» (frequenze relative).
pub const TYPICAL: f32 = 0.01;
pub const UNUSUAL: f32 = 0.0005;

/// Quanto e' comune la forma di un nominativo nel suo paese.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Shape {
    /// Sotto lo 0,05% dei nominativi del paese (o mai vista).
    Suspicious,
    /// Fra lo 0,05% e l'1%.
    Unusual,
    /// Dall'1% in su.
    Typical,
    /// Il paese non si conosce o ha troppi pochi nominativi per giudicare.
    Unknown,
}

impl Shape {
    pub fn of(prob: Option<f32>) -> Shape {
        match prob {
            None => Shape::Unknown,
            Some(p) if p >= TYPICAL => Shape::Typical,
            Some(p) if p >= UNUSUAL => Shape::Unusual,
            Some(_) => Shape::Suspicious,
        }
    }
}

/// La forma di un nominativo: prefisso letterale, cifre, lettere del suffisso.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Form {
    pub prefix: String,
    pub digits: u8,
    pub letters: u8,
}

/// Le lettere che l'ITU ha assegnato intere a un paese: con una cifra dopo restano lo stesso paese (K1, F5, G4...).
const WHOLE_LETTERS: &[u8] = b"BFGIKMNRW";
/// Le coppie lettera-cifra assegnate dall'ITU per le lettere divise fra piu' paesi (A2 Botswana, E7 Bosnia, Z8 Sud Sudan...).
const LETTER_DIGIT_SERIES: &[&str] = &[
    "A2", "A3", "A4", "A5", "A6", "A7", "A8", "A9", "C2", "C3", "C4", "C5", "C6", "C7", "C8", "C9", "D2", "D3", "D4", "D5", "D6", "D7", "D8", "D9",
    "E2", "E3", "E4", "E5", "E6", "E7", "H2", "H3", "H4", "H6", "H7", "H8", "H9", "J2", "J3", "J4", "J5", "J6", "J7", "J8", "L2", "L3", "L4", "L5",
    "L6", "L7", "L8", "L9", "P2", "P3", "P4", "P5", "P6", "P7", "P8", "P9", "S2", "S3", "S5", "S6", "S7", "S8", "S9", "T2", "T3", "T4", "T5", "T6",
    "T7", "T8", "V2", "V3", "V4", "V5", "V6", "V7", "V8", "Y2", "Y3", "Y4", "Y5", "Y6", "Y7", "Y8", "Y9", "Z2", "Z3", "Z8",
];

/// Il nominativo di base: con `EA8/IU3ABC` o `IU3ABC/P` la parte piu' lunga.
pub fn base_call(call: &str) -> &str {
    call.split('/').max_by_key(|p| p.len()).unwrap_or(call)
}

/// Prefisso (prima delle cifre dell'area), cifre e suffisso del nominativo di base; `None` se non ha quella forma.
fn split(base: &str) -> Option<(&str, &str, &str)> {
    let b = base.as_bytes();
    let letters = b.iter().rev().take_while(|c| c.is_ascii_uppercase()).count();
    let head = &base[..base.len() - letters];
    let digits = head.bytes().rev().take_while(|c| c.is_ascii_digit()).count();
    if letters == 0 || digits == 0 || digits == head.len() {
        return None;
    }
    let prefix = &head[..head.len() - digits];
    Some((prefix, &head[prefix.len()..], &base[base.len() - letters..]))
}

/// La forma di un nominativo (del suo nominativo di base).
pub fn form_of(call: &str) -> Option<Form> {
    let (prefix, digits, suffix) = split(base_call(call))?;
    let first = prefix.as_bytes().first().copied()?;
    let (prefix, digits) = if prefix.len() == 1 {
        // una lettera sola: la cifra fa parte del prefisso letterale (K1, A6); con due cifre (A61AB) la seconda e' quella dell'area
        let lit = format!("{prefix}{}", &digits[..1]);
        if digits.len() == 2 && !WHOLE_LETTERS.contains(&first) { (lit, digits.len() - 1) } else { (lit, digits.len()) }
    } else {
        (prefix.to_string(), digits.len())
    };
    Some(Form { prefix, digits: digits as u8, letters: suffix.len() as u8 })
}

/// La serie del nominativo non e' assegnata dall'ITU a nessun paese.
pub fn itu_unassigned(call: &str) -> bool {
    let base = base_call(call).as_bytes();
    let (Some(&a), Some(&b)) = (base.first(), base.get(1)) else { return true };
    if a == b'Q' || a == b'0' || a == b'1' {
        return true;
    }
    // lettera seguita da una cifra: se la lettera non e' intera di un paese, la coppia deve essere una serie assegnata
    if a.is_ascii_uppercase() && b.is_ascii_digit() && !WHOLE_LETTERS.contains(&a) {
        let pair = [a, b];
        return !LETTER_DIGIT_SERIES.iter().any(|s| s.as_bytes() == pair);
    }
    false
}

/// Le forme dei nominativi noti, contate per paese.
#[derive(Clone, Debug, Default)]
pub struct FormTable {
    countries: HashMap<String, (u32, HashMap<Form, u32>)>,
    calls: u32,
}

impl FormTable {
    pub fn new() -> Self {
        Self::default()
    }

    /// Un nominativo noto del paese `country`.
    pub fn add(&mut self, call: &str, country: &str) {
        let Some(form) = form_of(call) else { return };
        let entry = self.countries.entry(country.to_string()).or_default();
        entry.0 += 1;
        *entry.1.entry(form).or_insert(0) += 1;
        self.calls += 1;
    }

    /// Quanti nominativi sono stati contati.
    pub fn len(&self) -> u32 {
        self.calls
    }

    pub fn is_empty(&self) -> bool {
        self.calls == 0
    }

    /// La probabilita' a priori della forma del nominativo nel suo paese; `None` se il paese ha meno di `MIN_SAMPLES` nominativi noti.
    pub fn prob(&self, call: &str, country: &str) -> Option<f32> {
        let (total, forms) = self.countries.get(country)?;
        if *total < MIN_SAMPLES {
            return None;
        }
        let form = form_of(call)?;
        Some(forms.get(&form).copied().unwrap_or(0) as f32 / *total as f32)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_form_of_a_callsign() {
        let f = |c: &str| form_of(c).map(|f| (f.prefix, f.digits, f.letters));
        assert_eq!(f("IU3ABC"), Some(("IU".into(), 1, 3)));
        assert_eq!(f("K1ABC"), Some(("K1".into(), 1, 3)));
        assert_eq!(f("A61AB"), Some(("A6".into(), 1, 2)));
        assert_eq!(f("9A1A"), Some(("9A".into(), 1, 1)));
        assert_eq!(f("EA8/IU3ABC"), Some(("IU".into(), 1, 3)));
        assert_eq!(f("IU3ABC/P"), Some(("IU".into(), 1, 3)));
        assert_eq!(f("CQ"), None);
    }

    #[test]
    fn unassigned_itu_series() {
        for no in ["QA1ABC", "0A1A", "1B2C", "E81AB", "Z91A", "J91ABC"] {
            assert!(itu_unassigned(no), "{no}");
        }
        for ok in ["IU3ABC", "K1ABC", "E71A", "Z81A", "A61AB", "9A1A", "EA8/IU3ABC", "IU3ABC/P", "F5ABC"] {
            assert!(!itu_unassigned(ok), "{ok}");
        }
    }

    #[test]
    fn typical_unusual_and_suspicious_forms() {
        let mut t = FormTable::new();
        for i in 0..900 {
            t.add(&format!("IU{}A{}{}", i % 10, (b'A' + (i / 10 % 26) as u8) as char, (b'A' + (i / 260) as u8) as char), "Italy");
        }
        for i in 0..95 {
            t.add(&format!("IK{}A{}", i % 10, (b'A' + (i % 26) as u8) as char), "Italy");
        }
        t.add("IZ3ABCD", "Italy");
        assert_eq!(Shape::of(t.prob("IU3ABC", "Italy")), Shape::Typical);
        assert_eq!(Shape::of(t.prob("IZ3ABCD", "Italy")), Shape::Unusual);
        assert_eq!(Shape::of(t.prob("IW3ABCDE", "Italy")), Shape::Suspicious);
        // pochi nominativi: non si giudica
        t.add("K1ABC", "United States");
        assert_eq!(Shape::of(t.prob("K1ABC", "United States")), Shape::Unknown);
    }
}
