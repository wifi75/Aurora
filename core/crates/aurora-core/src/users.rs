//! Chi sta usando una pagina che tiene acceso un ricevitore (SSTV, PSK · RTTY · CW, satelliti).
//!
//! Ogni pagina aperta (finestra principale, finestra staccata, telefono) ha il suo identificativo e manda un «battito» ogni tanto
//! (`hold`); lasciando la pagina lo toglie (`release`). Il ricevitore si spegne solo quando non resta nessuno: una finestra staccata che si
//! chiude non spegne l'ascolto della finestra principale. Una pagina sparita senza salutare (finestra chiusa, programma chiuso, rete
//! caduta) esce da sola dopo `ttl` senza battiti.

use std::{
    collections::HashMap,
    sync::Mutex,
    time::{Duration, Instant},
};

/// Quanto resta valido un battito: le pagine ne mandano uno ogni 20 s; WebView2 e i browser rallentano i timer delle finestre nascoste
/// fino a uno al minuto, quindi il margine e' largo. Una pagina lasciata normalmente esce subito (`release`), non dopo `PAGE_TTL`.
pub const PAGE_TTL: Duration = Duration::from_secs(90);

/// Al massimo tante pagine insieme: oltre, si dimentica la piu' vecchia (un client che inventa identificativi non fa crescere la memoria).
const MAX_USERS: usize = 64;
/// Lunghezza massima di un identificativo.
const MAX_ID: usize = 64;

#[derive(Debug)]
pub struct PageUsers {
    seen: Mutex<HashMap<String, Instant>>,
    ttl: Duration,
}

impl PageUsers {
    pub fn new(ttl: Duration) -> PageUsers {
        PageUsers { seen: Mutex::new(HashMap::new()), ttl }
    }

    fn seen(&self) -> std::sync::MutexGuard<'_, HashMap<String, Instant>> {
        self.seen.lock().unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    /// Un battito della pagina `id` (entrando e poi ogni tanto). Un identificativo vuoto o troppo lungo non conta.
    pub fn hold(&self, id: &str) {
        self.hold_at(id, Instant::now());
    }

    fn hold_at(&self, id: &str, now: Instant) {
        let id = id.trim();
        if id.is_empty() || id.len() > MAX_ID {
            return;
        }
        let mut seen = self.seen();
        if !seen.contains_key(id)
            && seen.len() >= MAX_USERS
            && let Some(oldest) = seen.iter().min_by_key(|(_, t)| **t).map(|(k, _)| k.clone())
        {
            seen.remove(&oldest);
        }
        seen.insert(id.to_string(), now);
    }

    /// La pagina `id` e' stata lasciata. `true` se era l'ultima: adesso non la usa piu' nessuno.
    pub fn release(&self, id: &str) -> bool {
        let mut seen = self.seen();
        seen.remove(id.trim()).is_some() && seen.is_empty()
    }

    /// Toglie le pagine senza battito da oltre `ttl`. `true` se cosi' non resta nessuno (e prima qualcuno c'era).
    pub fn expire(&self) -> bool {
        self.expire_at(Instant::now())
    }

    fn expire_at(&self, now: Instant) -> bool {
        let mut seen = self.seen();
        if seen.is_empty() {
            return false;
        }
        seen.retain(|_, t| now.saturating_duration_since(*t) < self.ttl);
        seen.is_empty()
    }

    /// Qualche pagina la sta usando (con un battito recente).
    pub fn any(&self) -> bool {
        let now = Instant::now();
        self.seen().values().any(|t| now.saturating_duration_since(*t) < self.ttl)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn si_spegne_solo_quando_esce_l_ultima_pagina() {
        let u = PageUsers::new(Duration::from_secs(90));
        u.hold("principale");
        u.hold("staccata");
        assert!(!u.release("staccata"), "resta la finestra principale");
        assert!(u.any());
        assert!(u.release("principale"));
        assert!(!u.any());
        assert!(!u.release("principale"), "gia' uscita: non si spegne due volte");
    }

    #[test]
    fn una_pagina_senza_battito_esce_da_sola() {
        let u = PageUsers::new(Duration::from_secs(90));
        let t0 = Instant::now();
        u.hold_at("telefono", t0);
        assert!(!u.expire_at(t0 + Duration::from_secs(60)));
        assert!(u.expire_at(t0 + Duration::from_secs(91)));
        assert!(!u.expire_at(t0 + Duration::from_secs(200)), "nessuno da togliere: niente da spegnere");
    }

    #[test]
    fn identificativi_vuoti_o_troppi_non_fanno_crescere_la_memoria() {
        let u = PageUsers::new(Duration::from_secs(90));
        u.hold("");
        u.hold(&"x".repeat(200));
        assert!(!u.any());
        for i in 0..500 {
            u.hold(&format!("p{i}"));
        }
        assert!(u.seen().len() <= MAX_USERS);
    }
}
