//! I nominativi nel testo decodificato di un segnale: grammatica dei nominativi, ruoli (chi chiama CQ, chi trasmette dopo DE),
//! nominativo «verificato» quando ricorre, rapporto e parole chiave, livelli di convalida.
//!
//! Un nominativo è credibile se ha la forma giusta (prefisso, cifra, suffisso, eventuali `/P`, `/QRP`, `EA8/`...), se il prefisso porta
//! a un paese DXCC (quando Aurora ha `cty.dat`) e soprattutto se compare più di una volta nello stesso segnale: il rumore difficilmente
//! inventa due volte lo stesso nominativo. Il punteggio di una ricorrenza è la probabilità media dei suoi caratteri (dal decodificatore
//! probabilistico): due ricorrenze certe fanno «verificato», tante incerte pesano meno.

use std::collections::HashMap;

/// Quanto deve valere la somma delle probabilità delle ricorrenze perché il nominativo sia «verificato».
pub const VERIFIED_SCORE: f32 = 1.5;
/// Nominativi tenuti per segnale (i più recenti).
const MAX_CALLS: usize = 8;

/// Il livello di convalida: quanto si è severi prima di mostrare (o mandare come spot) un nominativo.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Validation {
    /// Basta la forma del nominativo.
    Minimal,
    /// Forma e paese DXCC (se `cty.dat` c'e').
    Normal,
    /// Forma, paese e almeno due ricorrenze («verificato»).
    Strict,
}

/// Il ruolo di un nominativo nel segnale.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Role {
    /// Lo si sente dopo «CQ» (o «TEST», «QRZ»): è la stazione che chiama.
    Cq,
    /// Lo si sente dopo «DE»: è la stazione che trasmette.
    De,
    /// Nominato da qualcun altro: il corrispondente.
    Other,
}

/// Un nominativo trovato in un segnale.
#[derive(Clone, Debug, PartialEq)]
pub struct HeardCall {
    pub call: String,
    pub role: Role,
    /// Quante volte e la somma delle probabilità delle ricorrenze.
    pub count: u32,
    pub score: f32,
    /// Momento (secondi del flusso) dell'ultima ricorrenza.
    pub last_s: f64,
}

impl HeardCall {
    pub fn verified(&self) -> bool {
        self.score >= VERIFIED_SCORE && self.count >= 2
    }
}

/// La forma di un nominativo: un prefisso facoltativo con `/` (`EA8/`), poi lettere e cifre con almeno una cifra preceduta da un prefisso
/// di una-tre lettere o cifre (con almeno una lettera) e seguita da un suffisso di una-quattro lettere, poi un suffisso facoltativo con
/// `/` (`/P`, `/M`, `/MM`, `/QRP`, `/1`). Le parole del traffico (CQ, 599, 5NN, TU...) non passano.
pub fn looks_like_call(word: &str) -> bool {
    let w = word.trim();
    if is_locator(w) {
        return false;
    }
    if w.len() < 3 || w.len() > 14 || !w.bytes().all(|b| b.is_ascii_uppercase() || b.is_ascii_digit() || b == b'/') {
        return false;
    }
    let parts: Vec<&str> = w.split('/').collect();
    if parts.len() > 3 || parts.iter().any(|p| p.is_empty()) {
        return false;
    }
    // la parte piu' lunga e' il nominativo di base; le altre sono prefisso o suffisso (fino a 4 caratteri)
    let (base_idx, base) = parts.iter().enumerate().max_by_key(|(_, p)| p.len()).map(|(i, p)| (i, *p)).unwrap_or((0, w));
    if parts.iter().enumerate().any(|(i, p)| i != base_idx && p.len() > 4) {
        return false;
    }
    base_shape(base)
}

/// Il nominativo di base: prefisso (1-3 caratteri con almeno una lettera, o cifra+lettera), una o due cifre, suffisso di 1-4 lettere.
fn base_shape(b: &str) -> bool {
    let bytes = b.as_bytes();
    if bytes.len() < 3 || bytes.len() > 8 {
        return false;
    }
    // il suffisso: le lettere finali
    let suffix = bytes.iter().rev().take_while(|c| c.is_ascii_uppercase()).count();
    if !(1..=4).contains(&suffix) {
        return false;
    }
    let head = &bytes[..bytes.len() - suffix];
    // la cifra (o due) dell'area prima del suffisso
    let digits = head.iter().rev().take_while(|c| c.is_ascii_digit()).count();
    if !(1..=2).contains(&digits) {
        return false;
    }
    let prefix = &head[..head.len() - digits];
    if prefix.is_empty() || prefix.len() > 3 {
        return false;
    }
    // il prefisso ha almeno una lettera (2E0, 9A, 4X: cifra e lettera vanno bene; solo cifre no)
    prefix.iter().any(|c| c.is_ascii_uppercase())
}

/// Un locatore Maidenhead di 4 o 6 caratteri (JN45, IO91WM): ha la forma di un nominativo ma non lo e'.
fn is_locator(w: &str) -> bool {
    let b = w.as_bytes();
    (b.len() == 4 || b.len() == 6)
        && (b'A'..=b'R').contains(&b[0])
        && (b'A'..=b'R').contains(&b[1])
        && b[2].is_ascii_digit()
        && b[3].is_ascii_digit()
        && (b.len() == 4 || ((b'A'..=b'X').contains(&b[4]) && (b'A'..=b'X').contains(&b[5])))
}

/// Le parole che cambiano il ruolo del nominativo che segue (o che precede).
fn cq_word(w: &str) -> bool {
    matches!(w, "CQ" | "TEST" | "QRZ" | "QRZ?" | "CQDX" | "POTA" | "SOTA")
}

/// I nominativi di un segnale, man mano che arriva il testo.
#[derive(Default)]
pub struct CallTracker {
    /// La parola in costruzione, con la somma delle probabilità dei suoi caratteri.
    word: String,
    word_conf: f32,
    /// Le ultime parole (per i ruoli).
    recent: Vec<String>,
    calls: HashMap<String, HeardCall>,
    /// Ha chiamato CQ (o TEST) di recente.
    cq: bool,
    /// L'ultimo rapporto visto (599, 5NN...).
    pub last_report: Option<String>,
    /// Ha chiesto se la frequenza e' libera («QRL?»).
    pub qrl: bool,
}

impl CallTracker {
    pub fn new() -> Self {
        Self::default()
    }

    /// Altro testo del segnale, con la probabilità (0-255) di ogni carattere (vuoto = certo), al momento `now_s`.
    pub fn push(&mut self, text: &[u8], conf: &[u8], now_s: f64) {
        for (i, &c) in text.iter().enumerate() {
            let p = conf.get(i).map_or(1.0, |&v| f32::from(v) / 255.0);
            if c == b' ' || c == b'\n' {
                self.end_word(now_s);
            } else {
                self.word.push(char::from(c.to_ascii_uppercase()));
                self.word_conf += p;
            }
        }
    }

    /// Fine del segnale o pausa lunga: chiude la parola in corso.
    pub fn flush(&mut self, now_s: f64) {
        self.end_word(now_s);
    }

    fn end_word(&mut self, now_s: f64) {
        if self.word.is_empty() {
            return;
        }
        let word = std::mem::take(&mut self.word);
        let conf = self.word_conf / word.len().max(1) as f32;
        self.word_conf = 0.0;
        if cq_word(&word) {
            self.cq = true;
        }
        if word == "QRL?" || word == "QRL" {
            self.qrl = true;
        }
        if is_report(&word) {
            self.last_report = Some(word.clone());
        }
        if looks_like_call(&word) {
            let prev = self.recent.last().map(String::as_str).unwrap_or("");
            let role = if prev == "DE" {
                Role::De
            } else if cq_word(prev) || (self.cq && self.recent.iter().rev().take(3).any(|w| cq_word(w))) {
                Role::Cq
            } else {
                Role::Other
            };
            let e = self.calls.entry(word.clone()).or_insert(HeardCall { call: word.clone(), role, count: 0, score: 0.0, last_s: now_s });
            e.count += 1;
            e.score += conf;
            e.last_s = now_s;
            // il ruolo piu' forte resta: CQ batte DE, DE batte «altro»
            e.role = match (e.role, role) {
                (Role::Cq, _) | (_, Role::Cq) => Role::Cq,
                (Role::De, _) | (_, Role::De) => Role::De,
                _ => Role::Other,
            };
            if self.calls.len() > MAX_CALLS {
                let oldest = self.calls.values().min_by(|a, b| a.last_s.total_cmp(&b.last_s)).map(|c| c.call.clone());
                if let Some(o) = oldest {
                    self.calls.remove(&o);
                }
            }
        }
        self.recent.push(word);
        if self.recent.len() > 6 {
            self.recent.remove(0);
        }
    }

    /// I nominativi che passano la convalida, dal più probabile; `dxcc`: il nominativo porta a un paese (se si sa).
    pub fn calls(&self, level: Validation, dxcc: &dyn Fn(&str) -> Option<bool>) -> Vec<HeardCall> {
        let mut v: Vec<HeardCall> = self
            .calls
            .values()
            .filter(|c| match level {
                Validation::Minimal => true,
                Validation::Normal => dxcc(&c.call) != Some(false),
                Validation::Strict => dxcc(&c.call) != Some(false) && c.verified(),
            })
            .cloned()
            .collect();
        v.sort_by(|a, b| b.score.total_cmp(&a.score));
        v
    }

    /// Dimentica i nominativi non sentiti da `max_age_s`.
    pub fn expire(&mut self, now_s: f64, max_age_s: f64) {
        self.calls.retain(|_, c| now_s - c.last_s <= max_age_s);
        if self.calls.is_empty() {
            self.cq = false;
            self.qrl = false;
        }
    }
}

/// Un rapporto RST (599, 579, 5NN, 4NN...).
fn is_report(w: &str) -> bool {
    let b = w.as_bytes();
    b.len() == 3 && (b[0] == b'5' || b[0] == b'4' || b[0] == b'3') && b[1..].iter().all(|c| c.is_ascii_digit() || *c == b'N')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_shape_of_a_callsign() {
        for ok in ["IU3ABC", "K1ABC", "W1AW", "2E0ABC", "9A1A", "EA8/IU3ABC", "IU3ABC/P", "IU3ABC/QRP", "VK2ABC/MM", "JA1ABC", "4X1AB", "IU3ABC/1"] {
            assert!(looks_like_call(ok), "{ok}");
        }
        for no in ["CQ", "DE", "599", "5NN", "TEST", "73", "IO91WM", "14080", "100W", "UR", "ABCDEF", "123", "IU3ABCDEF", "TU"] {
            assert!(!looks_like_call(no), "{no}");
        }
    }

    #[test]
    fn roles_and_verification() {
        let mut t = CallTracker::new();
        t.push(b"CQ CQ DE IU3ABC IU3ABC K ", &[], 1.0);
        t.push(b"IU3ABC DE K1ABC K1ABC KN ", &[], 5.0);
        t.push(b"K1ABC DE IU3ABC UR RST 599 599 ", &[], 9.0);
        let all = t.calls(Validation::Minimal, &|_| None);
        let me = all.iter().find(|c| c.call == "IU3ABC").cloned();
        let you = all.iter().find(|c| c.call == "K1ABC").cloned();
        assert!(me.as_ref().is_some_and(|c| c.role == Role::Cq && c.verified()), "{me:?}");
        assert!(you.as_ref().is_some_and(|c| c.role == Role::De && c.verified()), "{you:?}");
        assert_eq!(t.last_report.as_deref(), Some("599"));
    }

    #[test]
    fn one_uncertain_sighting_is_not_verified() {
        let mut t = CallTracker::new();
        t.push(b"DE IU3XQZ ", &[0, 0, 0, 90, 90, 90, 90, 90, 90, 90, 0], 1.0);
        t.flush(2.0);
        assert!(t.calls(Validation::Strict, &|_| None).is_empty());
        assert_eq!(t.calls(Validation::Minimal, &|_| None).len(), 1);
        // un prefisso che non porta a nessun paese si scarta dal livello normale
        assert!(t.calls(Validation::Normal, &|_| Some(false)).is_empty());
    }
}
