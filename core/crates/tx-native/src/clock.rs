//! Orologio iniettabile: quello vero per l'uso normale, uno manuale per le prove.

use std::{
    sync::{Condvar, Mutex},
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use crate::lock;

/// L'ora UTC come durata dal 1970 e l'attesa. Chi integra puo' implementarlo con l'ora corretta dall'offset di sincronizzazione.
pub trait Clock: Send + Sync {
    fn now(&self) -> Duration;
    /// Attende `d`; puo' tornare prima se qualcuno chiama [`Clock::wake`].
    fn sleep(&self, d: Duration);
    /// Sveglia chi dorme (usato da `halt`). Con l'orologio vero non serve: le attese sono a passi brevi.
    fn wake(&self) {}
}

/// L'orologio del sistema.
#[derive(Debug, Default, Clone, Copy)]
pub struct SystemClock;

impl Clock for SystemClock {
    fn now(&self) -> Duration {
        SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or(Duration::ZERO)
    }

    fn sleep(&self, d: Duration) {
        std::thread::sleep(d);
    }
}

#[derive(Default)]
struct ClockState {
    now: Duration,
    /// Quante volte un'attesa e' entrata o si e' risvegliata: serve a `advance` per sapere che chi dormiva ha visto il nuovo tempo.
    acks: u64,
    wake: u64,
}

/// Orologio che va avanti solo quando glielo si dice: le attese restano ferme fino a `advance`.
pub struct ManualClock {
    state: Mutex<ClockState>,
    time_changed: Condvar,
    acked: Condvar,
}

/// Tetto in tempo reale per un'attesa dell'orologio manuale: evita che una prova sbagliata resti appesa.
const REAL_CAP: Duration = Duration::from_secs(10);

impl ManualClock {
    pub fn new(start: Duration) -> ManualClock {
        ManualClock { state: Mutex::new(ClockState { now: start, acks: 0, wake: 0 }), time_changed: Condvar::new(), acked: Condvar::new() }
    }

    /// Porta avanti l'ora e aspetta (al massimo 300 ms veri) che chi dorme l'abbia vista.
    pub fn advance(&self, d: Duration) {
        let mut s = lock(&self.state);
        let before = s.acks;
        s.now += d;
        self.time_changed.notify_all();
        let limit = Instant::now() + Duration::from_millis(300);
        while s.acks == before {
            let left = limit.saturating_duration_since(Instant::now());
            if left.is_zero() {
                break;
            }
            s = self.acked.wait_timeout(s, left).unwrap_or_else(|p| p.into_inner()).0;
        }
    }

    /// Porta avanti l'ora senza aspettare nessuno.
    pub fn advance_nowait(&self, d: Duration) {
        lock(&self.state).now += d;
        self.time_changed.notify_all();
    }
}

impl Clock for ManualClock {
    fn now(&self) -> Duration {
        lock(&self.state).now
    }

    fn sleep(&self, d: Duration) {
        let mut s = lock(&self.state);
        let target = s.now + d;
        let wake = s.wake;
        s.acks += 1;
        self.acked.notify_all();
        let limit = Instant::now() + REAL_CAP;
        while s.now < target && s.wake == wake {
            let left = limit.saturating_duration_since(Instant::now());
            if left.is_zero() {
                return;
            }
            s = self.time_changed.wait_timeout(s, left).unwrap_or_else(|p| p.into_inner()).0;
            s.acks += 1;
            self.acked.notify_all();
        }
    }

    fn wake(&self) {
        lock(&self.state).wake += 1;
        self.time_changed.notify_all();
    }
}
