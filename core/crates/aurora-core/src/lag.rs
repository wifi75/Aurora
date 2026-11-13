//! Avviso nel registro quando un ascoltatore del bus (`broadcast`) resta indietro e perde eventi (`RecvError::Lagged`).
//!
//! Prima si perdevano in silenzio. Ogni ascoltatore tiene un suo `LagWatch`: per quelli comuni una riga `warn` al massimo ogni
//! `WARN_EVERY` (con il totale degli eventi persi nel frattempo), per non riempire il registro; per chi registra i QSO (`critical`) una
//! riga `error` a ogni perdita, perche' fra gli eventi persi puo' esserci un QSO da scrivere nel log.

use std::time::{Duration, Instant};

/// Al massimo una riga ogni tanto per ascoltatore (non critico).
pub const WARN_EVERY: Duration = Duration::from_secs(60);

#[derive(Debug)]
pub struct LagWatch {
    who: &'static str,
    critical: bool,
    pending: u64,
    last: Option<Instant>,
}

impl LagWatch {
    /// Un ascoltatore comune: `who` dice chi e' nel registro.
    pub fn new(who: &'static str) -> Self {
        Self { who, critical: false, pending: 0, last: None }
    }

    /// Un ascoltatore che scrive i QSO: ogni perdita e' un errore, senza attesa fra una riga e l'altra.
    pub fn critical(who: &'static str) -> Self {
        Self { who, critical: true, pending: 0, last: None }
    }

    /// Conta `n` eventi persi; restituisce quanti scriverne nel registro adesso (`None` se e' presto per un'altra riga).
    pub fn note(&mut self, n: u64, now: Instant) -> Option<u64> {
        self.pending = self.pending.saturating_add(n);
        let due = self.critical || self.last.is_none_or(|t| now.saturating_duration_since(t) >= WARN_EVERY);
        if !due {
            return None;
        }
        self.last = Some(now);
        Some(std::mem::take(&mut self.pending))
    }

    /// Da chiamare con il numero di `RecvError::Lagged(n)`.
    pub fn lost(&mut self, n: u64) {
        let Some(total) = self.note(n, Instant::now()) else { return };
        if self.critical {
            tracing::error!(
                "{}: rimasto indietro sul bus, {total} eventi persi: fra questi potrebbe esserci un QSO non registrato (controllare il log)",
                self.who
            );
        } else {
            tracing::warn!("{}: rimasto indietro sul bus, {total} eventi persi", self.who);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_common_listener_writes_at_most_one_line_per_minute_with_the_total() {
        let t0 = Instant::now();
        let mut w = LagWatch::new("prova");
        assert_eq!(w.note(5, t0), Some(5));
        assert_eq!(w.note(3, t0 + Duration::from_secs(1)), None);
        assert_eq!(w.note(4, t0 + Duration::from_secs(30)), None);
        // dopo un minuto una riga sola con tutto quello perso nel frattempo
        assert_eq!(w.note(1, t0 + Duration::from_secs(61)), Some(8));
        assert_eq!(w.note(2, t0 + Duration::from_secs(62)), None);
    }

    #[test]
    fn a_qso_listener_reports_every_loss() {
        let t0 = Instant::now();
        let mut w = LagWatch::critical("log");
        assert_eq!(w.note(2, t0), Some(2));
        assert_eq!(w.note(1, t0), Some(1));
        assert_eq!(w.note(7, t0 + Duration::from_millis(5)), Some(7));
    }

    #[tokio::test]
    async fn a_slow_broadcast_listener_gets_lagged_and_the_count_is_noted() {
        let (tx, mut rx) = tokio::sync::broadcast::channel::<u32>(4);
        for i in 0..10 {
            let _ = tx.send(i);
        }
        let mut w = LagWatch::new("prova");
        match rx.recv().await {
            Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => assert_eq!(w.note(n, Instant::now()), Some(6)),
            other => panic!("atteso Lagged, arrivato {other:?}"),
        }
    }
}
