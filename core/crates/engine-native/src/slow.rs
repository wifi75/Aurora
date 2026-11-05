//! Quanto ci mette la decodifica di ogni periodo. Un PC che arriva spesso al limite perde i segnali deboli (le ultime passate
//! saltano) senza che nessuno se ne accorga: qui si contano i periodi lenti e, se sono tanti, si avvisa sullo schermo e nei rapporti.

use std::collections::VecDeque;

/// Oltre questa parte del periodo la decodifica e' «lenta» (FT8: 11 s su 15, il limite oltre il quale non si fanno altre passate).
pub const SLOW_FRACTION: f64 = 0.7;
/// Su quanti periodi si guarda e quanti lenti bastano per dirlo.
pub const WINDOW: usize = 10;
pub const SLOW_IN_WINDOW: usize = 4;

#[derive(Debug, Default)]
pub struct SlowWatch {
    recent: VecDeque<bool>,
    /// Quante volte si e' gia' avvisato (il rapporto parte la prima volta e poi a potenze di dieci).
    warned: u64,
}

/// Cosa dire di un periodo lento, se c'e' da dirlo.
#[derive(Debug, PartialEq)]
pub struct SlowReport {
    pub slow: usize,
    pub of: usize,
    pub times: u64,
}

impl SlowWatch {
    /// Registra un periodo decodificato in `millis` su un periodo lungo `period_ms`.
    pub fn record(&mut self, millis: u64, period_ms: u64) -> Option<SlowReport> {
        let slow = period_ms > 0 && millis as f64 >= period_ms as f64 * SLOW_FRACTION;
        self.recent.push_back(slow);
        if self.recent.len() > WINDOW {
            self.recent.pop_front();
        }
        let count = self.recent.iter().filter(|s| **s).count();
        if !slow || count < SLOW_IN_WINDOW {
            return None;
        }
        self.warned += 1;
        // dopo un avviso si ricomincia a contare: lo stesso problema non si ridice a ogni periodo
        self.recent.clear();
        Some(SlowReport { slow: count, of: WINDOW, times: self.warned })
    }
}

/// I processori del PC (per il rapporto).
pub fn cores() -> usize {
    std::thread::available_parallelism().map_or(1, |n| n.get())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_fast_pc_is_never_reported_and_a_slow_one_is_after_a_few_periods() {
        let mut w = SlowWatch::default();
        for _ in 0..30 {
            assert!(w.record(3_000, 15_000).is_none());
        }
        let mut reports = Vec::new();
        for _ in 0..8 {
            reports.extend(w.record(12_000, 15_000));
        }
        assert_eq!(reports.len(), 2, "uno ogni {SLOW_IN_WINDOW} periodi lenti, non a ogni periodo");
        assert_eq!(reports[0], SlowReport { slow: 4, of: WINDOW, times: 1 });
    }

    #[test]
    fn ft4_has_its_own_limit() {
        let mut w = SlowWatch::default();
        for _ in 0..4 {
            assert!(w.record(4_000, 7_500).is_none(), "4 s su 7,5 non e' lento");
        }
    }
}
