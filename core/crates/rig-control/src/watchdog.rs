//! Il thread che parla con la radio (Hamlib, TCI, FT-991(A) Link, Cat4OM) se muore per un panic non ripartiva: lo stato restava l'ultima
//! istantanea e la radio non rispondeva piu' fino al riavvio di Aurora. Qui le due parti che servono al servizio CAT per accorgersene e
//! ricollegare: [`reap_panic`] (il thread e' finito per un panic?) e [`Restarts`] (pausa crescente e limite dei tentativi ravvicinati).
//! OmniRig ha gia' la sua ripartenza (`omnirig_com.rs`: un comando che trova il thread morto ne apre un altro).

use std::{
    collections::VecDeque,
    sync::Mutex,
    thread::JoinHandle,
    time::{Duration, Instant},
};

/// Tentativi ravvicinati concessi: oltre, il servizio si ferma in errore e aspetta che l'operatore ricolleghi.
pub const MAX_RESTARTS: usize = 5;
/// Finestra in cui si contano i tentativi ravvicinati.
pub const WINDOW: Duration = Duration::from_secs(600);
/// Pausa massima fra un tentativo e il successivo.
pub const MAX_PAUSE: Duration = Duration::from_secs(30);

/// Se il thread del collegamento e' finito per un panic ne restituisce il testo (e lo toglie dallo slot). Un thread ancora vivo, o finito
/// normalmente (fermato da `shutdown`), da' `None`.
pub(crate) fn reap_panic(slot: &Mutex<Option<JoinHandle<()>>>) -> Option<String> {
    let mut guard = slot.lock().unwrap_or_else(|p| p.into_inner());
    if !guard.as_ref().is_some_and(JoinHandle::is_finished) {
        return None;
    }
    let handle = guard.take()?;
    match handle.join() {
        Ok(()) => None,
        Err(payload) => Some(
            payload
                .downcast_ref::<&str>()
                .map(|s| (*s).to_string())
                .or_else(|| payload.downcast_ref::<String>().cloned())
                .unwrap_or_else(|| "panic senza testo".into()),
        ),
    }
}

/// Pausa crescente fra le ripartenze (base, 2x, 4x... fino a [`MAX_PAUSE`]) e al massimo [`MAX_RESTARTS`] in [`WINDOW`].
#[derive(Debug)]
pub struct Restarts {
    base: Duration,
    recent: VecDeque<Instant>,
}

impl Restarts {
    pub fn new(base: Duration) -> Self {
        Self { base, recent: VecDeque::new() }
    }

    /// Un altro panic adesso: la pausa prima di ricollegare (e il numero del tentativo), o `None` se i tentativi ravvicinati sono finiti.
    pub fn next(&mut self, now: Instant) -> Option<(Duration, usize)> {
        while self.recent.front().is_some_and(|t| now.saturating_duration_since(*t) >= WINDOW) {
            self.recent.pop_front();
        }
        if self.recent.len() >= MAX_RESTARTS {
            return None;
        }
        let shift = u32::try_from(self.recent.len()).unwrap_or(u32::MAX).min(16);
        let pause = self.base.saturating_mul(1u32 << shift).min(MAX_PAUSE);
        self.recent.push_back(now);
        Some((pause, self.recent.len()))
    }

    /// L'operatore ricollega a mano: si riparte da zero.
    pub fn reset(&mut self) {
        self.recent.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_pause_grows_and_the_close_attempts_are_limited() {
        let t0 = Instant::now();
        let mut r = Restarts::new(Duration::from_secs(1));
        let pauses: Vec<_> = (0..MAX_RESTARTS).map(|i| r.next(t0 + Duration::from_secs(i as u64)).map(|p| p.0.as_secs())).collect();
        assert_eq!(pauses, vec![Some(1), Some(2), Some(4), Some(8), Some(16)]);
        assert_eq!(r.next(t0 + Duration::from_secs(10)), None, "troppi tentativi ravvicinati");
        // passata la finestra si ricomincia
        assert_eq!(r.next(t0 + WINDOW + Duration::from_secs(10)).map(|p| p.0.as_secs()), Some(1));
        r.reset();
        assert_eq!(r.next(t0).map(|p| p.1), Some(1));
    }

    #[test]
    fn the_pause_never_goes_over_the_cap() {
        let mut r = Restarts::new(Duration::from_secs(20));
        let t0 = Instant::now();
        assert_eq!(r.next(t0).map(|p| p.0), Some(Duration::from_secs(20)));
        assert_eq!(r.next(t0).map(|p| p.0), Some(MAX_PAUSE));
    }

    #[test]
    fn only_a_panicked_thread_is_reaped() {
        let slot = Mutex::new(Some(std::thread::spawn(|| {})));
        while !slot.lock().unwrap().as_ref().unwrap().is_finished() {
            std::thread::sleep(Duration::from_millis(1));
        }
        assert_eq!(reap_panic(&slot), None, "finito normalmente");
        let slot = Mutex::new(Some(std::thread::spawn(|| panic!("radio finta rotta"))));
        while !slot.lock().unwrap().as_ref().unwrap().is_finished() {
            std::thread::sleep(Duration::from_millis(1));
        }
        assert_eq!(reap_panic(&slot).as_deref(), Some("radio finta rotta"));
        assert!(slot.lock().unwrap().is_none());
        let slot = Mutex::new(Some(std::thread::spawn(|| std::thread::sleep(Duration::from_millis(200)))));
        assert_eq!(reap_panic(&slot), None, "ancora vivo");
        assert!(slot.lock().unwrap().is_some());
    }
}
