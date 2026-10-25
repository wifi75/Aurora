//! Periodi UTC di FT8 (15 s) e FT4 (7,5 s) e scelta di quello in cui trasmettere.

use std::time::Duration;

use crate::error::TxError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Ft8,
    Ft4,
}

impl Mode {
    pub fn slot_len(self) -> Duration {
        match self {
            Mode::Ft8 => Duration::from_millis(15_000),
            Mode::Ft4 => Duration::from_millis(7_500),
        }
    }

    /// Ritardo dell'inizio dell'audio dall'inizio del periodo (0,5 s).
    pub fn tx_delay(self) -> Duration {
        Duration::from_millis(500)
    }

    /// La finestra in cui una trasmissione puo' ancora cominciare, contata dall'inizio del periodo: la durata del messaggio di JTDX
    /// (`txDuration` di `guiUpdate`, `REA 0x4c4350`: 13,64 s in FT8, 6,04 s in FT4). Il test `fTR < 99,0` e' sempre vero («Allow late starts»):
    /// oltre quell'istante non c'e' piu' niente da trasmettere.
    pub fn tx_window(self) -> Duration {
        match self {
            Mode::Ft8 => Duration::from_millis(13_640),
            Mode::Ft4 => Duration::from_millis(6_040),
        }
    }
}

/// Periodo pari (il primo del minuto: :00, :30 per FT8) o dispari (:15, :45).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Parity {
    Even,
    Odd,
}

fn slot_ms(mode: Mode) -> u64 {
    mode.slot_len().as_millis() as u64
}

/// Numero d'ordine del periodo che contiene `now` (durata dal 1970).
pub fn slot_index(now: Duration, mode: Mode) -> u64 {
    now.as_millis() as u64 / slot_ms(mode)
}

pub fn parity_of(index: u64) -> Parity {
    if index.is_multiple_of(2) { Parity::Even } else { Parity::Odd }
}

/// Inizio del periodo `index`.
pub fn slot_start(index: u64, mode: Mode) -> Duration {
    Duration::from_millis(index * slot_ms(mode))
}

/// L'inizio del periodo in cui trasmettere. Se il periodo in corso ha la parita' richiesta si usa quello, purche' sia ancora dentro la finestra
/// di avvio di JTDX (`Mode::tx_window`: l'avvio tardivo trasmette la coda del messaggio) e, se c'e', entro `max_late` dall'inizio nominale dell'audio:
/// altrimenti `TooLate`, perche' aspettare il periodo successivo della stessa parita' vorrebbe dire un messaggio vecchio di un intero ciclo.
/// Con l'altra parita' si usa il prossimo periodo.
pub fn choose_slot(now: Duration, mode: Mode, parity: Parity, max_late: Option<Duration>) -> Result<Duration, TxError> {
    let index = slot_index(now, mode);
    if parity_of(index) == parity {
        let start = slot_start(index, mode);
        let nominal = start + mode.tx_delay();
        let in_window = now <= start + mode.tx_window();
        let within_cap = max_late.is_none_or(|cap| now <= nominal + cap);
        if in_window && within_cap {
            Ok(start)
        } else {
            Err(TxError::TooLate { late_ms: now.saturating_sub(nominal).as_millis() as u64 })
        }
    } else {
        Ok(slot_start(index + 1, mode))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn secs(s: f64) -> Duration {
        Duration::from_secs_f64(s)
    }

    #[test]
    fn parity_follows_the_slot_index() {
        // il minuto UTC comincia con un periodo pari: :00 pari, :15 dispari, :30 pari, :45 dispari
        for (s, p) in [(0, Parity::Even), (15, Parity::Odd), (30, Parity::Even), (45, Parity::Odd), (60, Parity::Even)] {
            assert_eq!(parity_of(slot_index(Duration::from_secs(s + 1_700_000_040), Mode::Ft8)), p, ":{s}");
        }
        // FT4: periodi da 7,5 s
        assert_eq!(slot_index(secs(7.4), Mode::Ft4), 0);
        assert_eq!(slot_index(secs(7.6), Mode::Ft4), 1);
        assert_eq!(slot_start(3, Mode::Ft4), secs(22.5));
    }

    #[test]
    fn the_other_parity_waits_for_the_next_slot() {
        let base = slot_start(2_000_000, Mode::Ft8); // periodo pari
        let now = base + secs(3.0);
        assert_eq!(choose_slot(now, Mode::Ft8, Parity::Odd, None), Ok(base + secs(15.0)));
    }

    /// La finestra di avvio e' la durata del messaggio di JTDX: 13,64 s in FT8 (avvio tardivo, si emette la coda), 6,04 s in FT4.
    #[test]
    fn the_same_parity_uses_the_current_slot_for_the_whole_message_duration() {
        let base = slot_start(2_000_000, Mode::Ft8);
        assert_eq!(choose_slot(base + secs(0.1), Mode::Ft8, Parity::Even, None), Ok(base));
        assert_eq!(choose_slot(base + secs(5.0), Mode::Ft8, Parity::Even, None), Ok(base));
        assert_eq!(choose_slot(base + secs(13.6), Mode::Ft8, Parity::Even, None), Ok(base));
        let err = choose_slot(base + secs(13.7), Mode::Ft8, Parity::Even, None);
        assert_eq!(err, Err(TxError::TooLate { late_ms: 13_200 }));
        let ft4 = slot_start(4_000_000, Mode::Ft4);
        assert_eq!(choose_slot(ft4 + secs(6.0), Mode::Ft4, Parity::Even, None), Ok(ft4));
        assert!(choose_slot(ft4 + secs(6.1), Mode::Ft4, Parity::Even, None).is_err());
    }

    /// Un tetto esplicito (`max_late`) restringe la finestra.
    #[test]
    fn an_explicit_cap_narrows_the_window() {
        let base = slot_start(2_000_000, Mode::Ft8);
        assert_eq!(choose_slot(base + secs(1.4), Mode::Ft8, Parity::Even, Some(secs(1.0))), Ok(base));
        let err = choose_slot(base + secs(2.0), Mode::Ft8, Parity::Even, Some(secs(1.0)));
        assert_eq!(err, Err(TxError::TooLate { late_ms: 1500 }));
    }

    #[test]
    fn ft4_slots_are_seven_and_a_half_seconds() {
        let base = slot_start(4_000_000, Mode::Ft4); // pari
        assert_eq!(choose_slot(base + secs(1.0), Mode::Ft4, Parity::Odd, None), Ok(base + secs(7.5)));
    }
}
