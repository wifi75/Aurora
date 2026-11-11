//! L'elenco dei nominativi conosciuti, per il filtro contro i falsi decode (`chkflscall` di JTDX, che usa `ALLCALL7.TXT`).
//!
//! Aurora non porta con se' un elenco completo, quindi lo costruisce: i nominativi del log (le stazioni gia' collegate) e quelli sentiti in
//! messaggi decodificati bene (senza analisi a priori e con un SNR non al limite). L'elenco si salva su disco e cresce con l'uso.
//!
//! Un elenco incompleto scarta stazioni vere: per questo il filtro si accende solo quando i nominativi sono abbastanza ([`MIN_CALLS`]) e,
//! come in JTDX, agisce solo sui messaggi gia' dubbi (qualita' bassa, SNR basso, ritardo strano; vedi `ft8::filters::reject_message`).

use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

use ft8::decode::Decoded;
use ft8::filters::CallBook;

/// Sotto questo numero di nominativi l'elenco e' troppo povero e il filtro resta spento.
pub const MIN_CALLS: usize = 300;
/// Oltre questo numero non si imparano altri nominativi (file e confronto restano piccoli).
const MAX_CALLS: usize = 300_000;
/// Ogni quanto si salva il file se e' cambiato.
const SAVE_EVERY: Duration = Duration::from_secs(300);
/// Ogni quanto si ricostruisce l'elenco per il decoder se e' cambiato.
const REBUILD_EVERY: Duration = Duration::from_secs(60);
/// Un messaggio decodificato con un SNR sotto questo valore non insegna niente: e' il tipo di messaggio che puo' essere un falso.
const LEARN_MIN_SNR_DB: i32 = -18;

pub struct KnownCalls {
    calls: HashSet<String>,
    path: Option<PathBuf>,
    dirty: bool,
    last_save: Instant,
    book: Option<Arc<CallBook>>,
    book_len: usize,
    last_build: Option<Instant>,
    limit_warned: bool,
}

impl KnownCalls {
    /// Legge il file (un nominativo per riga); se non c'e' parte vuoto.
    pub fn load(path: PathBuf) -> KnownCalls {
        let (calls, path) = match std::fs::read(&path) {
            Ok(bytes) => (String::from_utf8_lossy(&bytes).lines().map(|l| l.trim().to_uppercase()).filter(|l| !l.is_empty()).collect(), Some(path)),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => (HashSet::new(), Some(path)),
            Err(e) => {
                // un file che c'e' ma non si legge non va riscritto con un elenco vuoto: si lavora in memoria e si dice perche'
                tracing::warn!("elenco dei nominativi conosciuti non leggibile ({}): {e}; resta solo in memoria e il file non si tocca", path.display());
                (HashSet::new(), None)
            }
        };
        KnownCalls { calls, path, dirty: false, last_save: Instant::now(), book: None, book_len: 0, last_build: None, limit_warned: false }
    }

    #[cfg(test)]
    pub fn in_memory() -> KnownCalls {
        KnownCalls { calls: HashSet::new(), path: None, dirty: false, last_save: Instant::now(), book: None, book_len: 0, last_build: None, limit_warned: false }
    }

    #[cfg(test)]
    fn len(&self) -> usize {
        self.calls.len()
    }

    /// Aggiunge nominativi (per esempio quelli del log).
    pub fn add_all<I: IntoIterator<Item = String>>(&mut self, calls: I) {
        for call in calls {
            self.insert(&call);
        }
    }

    fn insert(&mut self, call: &str) {
        let call = call.trim().to_uppercase();
        if call.is_empty() {
            return;
        }
        if self.calls.len() >= MAX_CALLS {
            if !self.limit_warned {
                self.limit_warned = true;
                tracing::warn!("elenco dei nominativi conosciuti pieno ({MAX_CALLS}): i nominativi nuovi non si imparano piu'");
            }
            return;
        }
        // la forma nuda (senza /P, /R...) conta come la completa: una stazione conosciuta e' conosciuta anche in portatile
        let bare = strip_suffix(&call);
        if bare != call && self.calls.insert(bare.to_string()) {
            self.dirty = true;
        }
        if self.calls.insert(call) {
            self.dirty = true;
        }
    }

    /// Impara i nominativi dei messaggi decodificati bene: non trovati con l'analisi a priori e non al limite della sensibilita'.
    pub fn learn(&mut self, decoded: &[Decoded]) {
        for d in decoded {
            if d.hint || d.snr_db < LEARN_MIN_SNR_DB {
                continue;
            }
            for call in calls_of(&d.text) {
                self.insert(&call);
            }
        }
    }

    /// L'elenco per il decoder: `None` finche' i nominativi sono pochi. Si ricostruisce al massimo una volta al minuto.
    pub fn book(&mut self) -> Option<Arc<CallBook>> {
        if self.calls.len() < MIN_CALLS {
            return None;
        }
        let stale = self.book.is_none() || (self.book_len != self.calls.len() && self.last_build.is_none_or(|t| t.elapsed() >= REBUILD_EVERY));
        if stale {
            self.book = Some(Arc::new(CallBook::new(self.calls.iter().cloned())));
            self.book_len = self.calls.len();
            self.last_build = Some(Instant::now());
        }
        self.book.clone()
    }

    /// Salva il file se e' cambiato e l'ultimo salvataggio e' di piu' di cinque minuti fa.
    pub fn save_if_due(&mut self) {
        if !self.dirty || self.last_save.elapsed() < SAVE_EVERY {
            return;
        }
        let Some(path) = &self.path else { return };
        let mut sorted: Vec<&String> = self.calls.iter().collect();
        sorted.sort();
        let text = sorted.iter().map(|c| c.as_str()).collect::<Vec<_>>().join("\n");
        match std::fs::write(path, text) {
            Ok(()) => {
                self.dirty = false;
                self.last_save = Instant::now();
            }
            Err(e) => {
                tracing::warn!("elenco dei nominativi non salvato: {e}");
                self.last_save = Instant::now();
            }
        }
    }
}

/// Toglie un suffisso di servizio (`/P`, `/R`, `/M`, `/QRP`...) dal nominativo.
fn strip_suffix(call: &str) -> &str {
    match call.rsplit_once('/') {
        Some((head, tail)) if matches!(tail, "P" | "R" | "M" | "QRP" | "MM" | "AM") || tail.len() == 1 && tail.chars().all(|c| c.is_ascii_digit()) => head,
        _ => call,
    }
}

/// Un locatore a 4 o 6 caratteri (`FN42`, `IO91wm`) o il finto locatore `RR73`: non e' un nominativo.
fn is_grid_like(word: &str) -> bool {
    let b = word.as_bytes();
    let field = |c: u8| (b'A'..=b'R').contains(&c.to_ascii_uppercase());
    (b.len() == 4 || b.len() == 6) && field(b[0]) && field(b[1]) && b[2].is_ascii_digit() && b[3].is_ascii_digit() && (b.len() == 4 || (b'A'..=b'X').contains(&b[4].to_ascii_uppercase()) && (b'A'..=b'X').contains(&b[5].to_ascii_uppercase()))
}

/// Un nominativo plausibile: 3-11 caratteri tra lettere, cifre e `/`, con almeno una cifra e una lettera.
fn is_call_like(word: &str) -> bool {
    (3..=11).contains(&word.len())
        && word.bytes().all(|c| c.is_ascii_alphanumeric() || c == b'/')
        && word.bytes().any(|c| c.is_ascii_digit())
        && word.bytes().any(|c| c.is_ascii_alphabetic())
        && !is_grid_like(word)
}

/// I nominativi di un messaggio: tra i primi tre campi, quelli che hanno la forma di un nominativo (niente `CQ`, `DX`, locatori, rapporti).
/// Gli hash `<...>` e i nominativi tra parentesi angolari si leggono comunque: le parentesi si tolgono.
pub fn calls_of(text: &str) -> Vec<String> {
    text.split_whitespace()
        .take(3)
        .map(|w| w.trim_matches(|c| c == '<' || c == '>'))
        .filter(|w| !matches!(*w, "CQ" | "DE" | "QRZ" | "TU;") && is_call_like(&w.to_uppercase()))
        .map(str::to_uppercase)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dec(text: &str, snr: i32, hint: bool) -> Decoded {
        Decoded { freq_hz: 1000.0, dt_s: 0.0, snr_db: snr, text: text.to_string(), hint }
    }

    #[test]
    fn i_nominativi_di_un_messaggio() {
        assert_eq!(calls_of("CQ K1ABC FN42"), ["K1ABC"]);
        assert_eq!(calls_of("CQ DX K1ABC FN42"), ["K1ABC"]);
        assert_eq!(calls_of("IU3ABC K1ABC -15"), ["IU3ABC", "K1ABC"]);
        assert_eq!(calls_of("IU3ABC K1ABC R-15"), ["IU3ABC", "K1ABC"]);
        assert_eq!(calls_of("IU3ABC K1ABC RR73"), ["IU3ABC", "K1ABC"]);
        assert_eq!(calls_of("<IU3ABC> K1ABC 73"), ["IU3ABC", "K1ABC"]);
        assert_eq!(calls_of("<...> K1ABC -10"), ["K1ABC"]);
        assert_eq!(calls_of("CQ PJ4/K1ABC"), ["PJ4/K1ABC"]);
        assert!(calls_of("CQ TEST 73").is_empty());
    }

    #[test]
    fn impara_solo_da_messaggi_puliti() {
        let mut k = KnownCalls::in_memory();
        k.learn(&[dec("CQ K1ABC FN42", -10, false), dec("CQ W9XYZ EN37", -22, false), dec("CQ N5QQQ EM10", -5, true)]);
        assert_eq!(k.len(), 1);
        assert!(k.calls.contains("K1ABC"));
    }

    #[test]
    fn il_portatile_insegna_anche_la_forma_nuda() {
        let mut k = KnownCalls::in_memory();
        k.add_all(["K1ABC/P".to_string()]);
        assert!(k.calls.contains("K1ABC") && k.calls.contains("K1ABC/P"));
    }

    #[test]
    fn il_filtro_si_accende_solo_con_abbastanza_nominativi() {
        let mut k = KnownCalls::in_memory();
        k.add_all((0..MIN_CALLS - 1).map(|i| format!("K{i}ABC")));
        assert!(k.book().is_none());
        k.add_all(["W1AW".to_string(), "W2AW".to_string()]);
        let book = k.book().expect("elenco pronto");
        assert!(book.contains("W1AW"));
    }
}
