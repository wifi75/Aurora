//! I nominativi nel testo decodificato di un segnale: grammatica dei nominativi, ruoli (chi chiama CQ, chi trasmette dopo DE),
//! nominativo «verificato» quando ricorre, rapporto e parole chiave, livelli di convalida.
//!
//! Un nominativo è credibile se ha la forma giusta (prefisso, cifra, suffisso, eventuali `/P`, `/QRP`, `EA8/`...), se il prefisso porta
//! a un paese DXCC (quando Aurora ha `cty.dat`) e soprattutto se compare più di una volta nello stesso segnale: il rumore difficilmente
//! inventa due volte lo stesso nominativo. Il punteggio di una ricorrenza è la probabilità media dei suoi caratteri (dal decodificatore
//! probabilistico): due ricorrenze certe fanno «verificato», tante incerte pesano meno.

//!
//! La forma del nominativo nel suo paese ([`super::forms`]) fa da probabilita' a priori: un nominativo dalla forma insolita deve ricorrere
//! di piu' per essere «verificato» (punteggio 2 invece di 1,5), uno dalla forma sospetta ancora di piu' (3, e almeno tre volte).

use super::forms::{Shape, itu_unassigned};
use std::collections::HashMap;

/// Quanto deve valere la somma delle probabilità delle ricorrenze perché il nominativo sia «verificato» (forma tipica o non giudicabile).
pub const VERIFIED_SCORE: f32 = 1.5;
/// Lo stesso per una forma insolita e per una sospetta.
const VERIFIED_SCORE_UNUSUAL: f32 = 2.0;
const VERIFIED_SCORE_SUSPICIOUS: f32 = 3.0;
/// Nominativi tenuti per segnale (i più recenti).
const MAX_CALLS: usize = 8;

/// Il livello di convalida: quanto si è severi prima di mostrare (o mandare come spot) un nominativo. Gli spot vogliono sempre un
/// nominativo verificato, in piu' delle condizioni del livello.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub enum Validation {
    /// Basta la forma del nominativo (e una serie ITU assegnata).
    Minimal,
    /// Forma e paese DXCC (se `cty.dat` c'e'); le forme sospette solo se verificate.
    #[default]
    Normal,
    /// Come Normale, ma le forme sospette mai e quelle insolite solo se verificate.
    Strict,
    /// Solo nominativi verificati dalla forma tipica (o di paesi con pochi nominativi noti).
    Maximum,
}

impl Validation {
    pub fn from_id(id: &str) -> Option<Validation> {
        match id {
            "minimal" => Some(Validation::Minimal),
            "normal" => Some(Validation::Normal),
            "strict" => Some(Validation::Strict),
            "maximum" => Some(Validation::Maximum),
            _ => None,
        }
    }
}

/// Quello che si sa di un nominativo fuori dal segnale: se porta a un paese DXCC (`None` = non si sa, senza `cty.dat`) e la probabilita'
/// a priori della sua forma nel paese (`None` = non giudicabile).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct CallCheck {
    pub dxcc: Option<bool>,
    pub prob: Option<f32>,
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
    /// Quanto e' comune la sua forma nel paese (la mette [`CallTracker::calls`]).
    pub shape: Shape,
}

impl HeardCall {
    pub fn verified(&self) -> bool {
        match self.shape {
            Shape::Typical | Shape::Unknown => self.score >= VERIFIED_SCORE && self.count >= 2,
            Shape::Unusual => self.score >= VERIFIED_SCORE_UNUSUAL && self.count >= 2,
            Shape::Suspicious => self.score >= VERIFIED_SCORE_SUSPICIOUS && self.count >= 3,
        }
    }

    /// Passa il livello di convalida, sapendo cosa dice `check` del nominativo.
    pub fn passes(&self, level: Validation, check: CallCheck) -> bool {
        // una serie che l'ITU non ha assegnato si scarta, salvo che cty.dat la conosca (prefissi speciali)
        if check.dxcc != Some(true) && itu_unassigned(&self.call) {
            return false;
        }
        let country = check.dxcc != Some(false);
        let verified = self.verified();
        match level {
            Validation::Minimal => true,
            Validation::Normal => country && (self.shape != Shape::Suspicious || verified),
            Validation::Strict => country && self.shape != Shape::Suspicious && (self.shape != Shape::Unusual || verified),
            Validation::Maximum => country && verified && matches!(self.shape, Shape::Typical | Shape::Unknown),
        }
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
            let e = self.calls.entry(word.clone()).or_insert(HeardCall { call: word.clone(), role, count: 0, score: 0.0, last_s: now_s, shape: Shape::Unknown });
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

    /// I nominativi che passano la convalida, dal più probabile; `check`: cosa si sa del nominativo (paese DXCC, forma).
    pub fn calls(&self, level: Validation, check: &dyn Fn(&str) -> CallCheck) -> Vec<HeardCall> {
        let mut v: Vec<HeardCall> = self
            .calls
            .values()
            .filter_map(|c| {
                let info = check(&c.call);
                let mut c = c.clone();
                c.shape = Shape::of(info.prob);
                c.passes(level, info).then_some(c)
            })
            .collect();
        // un pezzo di un nominativo piu' lungo dello stesso segnale, sentito almeno altrettanto, e' un frammento (uno spazio inventato
        // dentro il nominativo, frequente con il tasto a mano: «OH2AB C», «O H2ABC»): non si mostra
        let all: Vec<(String, f32)> = self.calls.values().map(|c| (c.call.clone(), c.score)).collect();
        v.retain(|c| !all.iter().any(|(other, score)| other.len() > c.call.len() && other.contains(c.call.as_str()) && *score >= c.score));
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
        let all = t.calls(Validation::Minimal, &|_| CallCheck::default());
        let me = all.iter().find(|c| c.call == "IU3ABC").cloned();
        let you = all.iter().find(|c| c.call == "K1ABC").cloned();
        assert!(me.as_ref().is_some_and(|c| c.role == Role::Cq && c.verified()), "{me:?}");
        assert!(you.as_ref().is_some_and(|c| c.role == Role::De && c.verified()), "{you:?}");
        assert_eq!(t.last_report.as_deref(), Some("599"));
    }

    /// Uno spazio inventato dentro il nominativo (tasto a mano) fa leggere un pezzo: «OH2AB C», «O H2ABC». Il pezzo non si mostra
    /// accanto al nominativo intero sentito piu' volte; un nominativo diverso che non e' un pezzo resta.
    #[test]
    fn a_fragment_of_a_longer_call_is_not_shown() {
        let mut t = CallTracker::new();
        t.push(b"CQ DE OH2ABC OH2ABC K CQ DE OH2AB C OH2ABC K CQ DE O H2ABC OH2AB C K DE K1XYZ K1XYZ ", &[], 1.0);
        let all: Vec<String> = t.calls(Validation::Minimal, &|_| CallCheck::default()).into_iter().map(|c| c.call).collect();
        assert!(all.contains(&"OH2ABC".to_string()) && all.contains(&"K1XYZ".to_string()), "{all:?}");
        assert!(!all.contains(&"OH2AB".to_string()) && !all.contains(&"H2ABC".to_string()), "{all:?}");
    }

    #[test]
    fn one_uncertain_sighting_is_not_verified() {
        let mut t = CallTracker::new();
        t.push(b"DE IU3XQZ ", &[0, 0, 0, 90, 90, 90, 90, 90, 90, 90, 0], 1.0);
        t.flush(2.0);
        assert!(t.calls(Validation::Maximum, &|_| CallCheck::default()).is_empty());
        assert_eq!(t.calls(Validation::Minimal, &|_| CallCheck::default()).len(), 1);
        // un prefisso che non porta a nessun paese si scarta dal livello normale
        assert!(t.calls(Validation::Normal, &|_| CallCheck { dxcc: Some(false), prob: None }).is_empty());
    }

    #[test]
    fn the_levels_follow_the_shape_of_the_call() {
        let mut t = CallTracker::new();
        // due volte, certo: verificato con una forma tipica o insolita (punteggio 2), non con una sospetta (vuole 3 e tre volte)
        t.push(b"DE IU3ABC IU3ABC ", &[], 1.0);
        t.flush(2.0);
        let with = |p: f32| move |_: &str| CallCheck { dxcc: Some(true), prob: Some(p) };
        let typical = t.calls(Validation::Maximum, &with(0.2));
        assert!(typical.len() == 1 && typical[0].shape == Shape::Typical && typical[0].verified());
        // insolita: verificata, passa fino a Severo ma non Massimo
        assert_eq!(t.calls(Validation::Strict, &with(0.002)).len(), 1);
        assert!(t.calls(Validation::Maximum, &with(0.002)).is_empty());
        // sospetta e non verificata: solo Minimo
        assert!(t.calls(Validation::Normal, &with(0.0001)).is_empty());
        assert_eq!(t.calls(Validation::Minimal, &with(0.0001)).len(), 1);
        // una terza volta la verifica anche sospetta: Normale la mostra, Severo no
        t.push(b"IU3ABC ", &[], 3.0);
        t.flush(4.0);
        assert_eq!(t.calls(Validation::Normal, &with(0.0001)).len(), 1);
        assert!(t.calls(Validation::Strict, &with(0.0001)).is_empty());
        // una serie ITU non assegnata si scarta sempre, salvo che cty.dat la conosca
        let mut q = CallTracker::new();
        q.push(b"DE QA1ABC QA1ABC ", &[], 1.0);
        q.flush(2.0);
        assert!(q.calls(Validation::Minimal, &|_| CallCheck::default()).is_empty());
        assert_eq!(q.calls(Validation::Minimal, &|_| CallCheck { dxcc: Some(true), prob: None }).len(), 1);
    }
}
