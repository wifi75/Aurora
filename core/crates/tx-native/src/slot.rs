//! Periodi UTC di FT8 (15 s) e FT4 (7,5 s) e scelta di quello in cui trasmettere.

use std::time::Duration;

use std::sync::atomic::{AtomicU32, Ordering};

use crate::error::TxError;

/// Periodo di Q65 in secondi (15, 30, 60, 120, 300), scelto dal motore.
static Q65_PERIOD_S: AtomicU32 = AtomicU32::new(30);

/// Sceglie il periodo di Q65; valori fuori campo si ignorano.
pub fn set_q65_period_s(period_s: u32) {
    if matches!(period_s, 15 | 30 | 60 | 120 | 300) {
        Q65_PERIOD_S.store(period_s, Ordering::Relaxed);
    }
}

/// Periodo di Q65 in uso, in secondi.
pub fn q65_period_s() -> u32 {
    Q65_PERIOD_S.load(Ordering::Relaxed)
}

/// Periodo di MSK144 in secondi (5, 10, 15, 30, `mainwindow.cpp:12081` di WSJT-X), scelto dal motore.
static MSK144_PERIOD_S: AtomicU32 = AtomicU32::new(15);

/// Sceglie il periodo di MSK144; valori fuori campo si ignorano.
pub fn set_msk144_period_s(period_s: u32) {
    if matches!(period_s, 5 | 10 | 15 | 30) {
        MSK144_PERIOD_S.store(period_s, Ordering::Relaxed);
    }
}

/// Periodo di MSK144 in uso, in secondi.
pub fn msk144_period_s() -> u32 {
    MSK144_PERIOD_S.load(Ordering::Relaxed)
}

/// Durata del messaggio Q65 in millisecondi (85 simboli, `q65params`).
fn q65_message_ms(period_s: u32) -> u64 {
    let nsps = match period_s {
        15 => 1_800,
        30 => 3_600,
        60 => 7_200,
        120 => 16_000,
        _ => 41_472,
    };
    85 * nsps / 12
}

/// Periodi di FST4 e FST4W in uso, in secondi (scelti dal motore; 60 e 120 di fabbrica come WSJT-X).
static FST4_PERIOD_S: AtomicU32 = AtomicU32::new(60);
static FST4W_PERIOD_S: AtomicU32 = AtomicU32::new(120);

/// Sceglie il periodo di FST4 (`wspr` = false) o FST4W; valori fuori campo si ignorano.
pub fn set_fst4_period_s(wspr: bool, period_s: u32) {
    if wspr && matches!(period_s, 120 | 300 | 900 | 1800) {
        FST4W_PERIOD_S.store(period_s, Ordering::Relaxed);
    } else if !wspr && matches!(period_s, 15 | 30 | 60 | 120 | 300 | 900 | 1800) {
        FST4_PERIOD_S.store(period_s, Ordering::Relaxed);
    }
}

/// Periodo in uso di FST4 (`wspr` = false) o FST4W, in secondi.
pub fn fst4_period_s(wspr: bool) -> u32 {
    if wspr { FST4W_PERIOD_S.load(Ordering::Relaxed) } else { FST4_PERIOD_S.load(Ordering::Relaxed) }
}

/// Durata del messaggio FST4 in millisecondi (160 simboli, `mainwindow.cpp:13489-13495` di WSJT-X).
fn fst4_message_ms(period_s: u32) -> u64 {
    let nsps: u64 = match period_s {
        15 => 720,
        30 => 1_680,
        60 => 3_888,
        120 => 8_200,
        300 => 21_504,
        900 => 66_560,
        _ => 134_400,
    };
    160 * nsps / 12
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Ft8,
    Ft4,
    Jt65,
    Jt9,
    Jt4,
    Q65,
    Msk144,
    Fst4,
    Fst4w,
    /// WSPR-2: periodi di due minuti, messaggio a un secondo dall'inizio.
    Wspr,
}

impl Mode {
    pub fn slot_len(self) -> Duration {
        match self {
            Mode::Ft8 => Duration::from_millis(15_000),
            Mode::Msk144 => Duration::from_millis(u64::from(msk144_period_s()) * 1000),
            Mode::Wspr => Duration::from_millis(120_000),
            Mode::Ft4 => Duration::from_millis(7_500),
            Mode::Jt65 | Mode::Jt9 | Mode::Jt4 => Duration::from_millis(60_000),
            Mode::Q65 => Duration::from_millis(u64::from(q65_period_s()) * 1000),
            Mode::Fst4 => Duration::from_millis(u64::from(fst4_period_s(false)) * 1000),
            Mode::Fst4w => Duration::from_millis(u64::from(fst4_period_s(true)) * 1000),
        }
    }

    /// Ritardo dell'inizio dell'audio dall'inizio del periodo: 0,5 s in FT8 e FT4, 1 s in JT65 e JT9.
    pub fn tx_delay(self) -> Duration {
        match self {
            Mode::Ft8 | Mode::Ft4 | Mode::Msk144 => Duration::from_millis(500),
            // Q65: 0,5 s a 15 e 30 s, 1 s dai 60 s in su
            Mode::Q65 if q65_period_s() < 60 => Duration::from_millis(500),
            // FST4: 0,5 s in FST4-15, 1 s negli altri e in FST4W (`Modulator.cpp:163-164`)
            Mode::Fst4 if fst4_period_s(false) == 15 => Duration::from_millis(500),
            Mode::Jt65 | Mode::Jt9 | Mode::Jt4 | Mode::Q65 | Mode::Fst4 | Mode::Fst4w | Mode::Wspr => Duration::from_millis(1_000),
        }
    }

    /// La finestra in cui una trasmissione puo' ancora cominciare, contata dall'inizio del periodo: la durata del messaggio di JTDX
    /// (`txDuration` di `guiUpdate`, `REA 0x4c4350`: 13,64 s in FT8, 6,04 s in FT4). Il test `fTR < 99,0` e' sempre vero («Allow late starts»):
    /// oltre quell'istante non c'e' piu' niente da trasmettere.
    pub fn tx_window(self) -> Duration {
        match self {
            Mode::Ft8 => Duration::from_millis(13_640),
            Mode::Ft4 => Duration::from_millis(6_040),
            Mode::Jt65 => Duration::from_millis(47_810),
            Mode::Jt9 => Duration::from_millis(49_960),
            // 206 simboli da 0,2286 s
            Mode::Jt4 => Duration::from_millis(47_090),
            // 85 simboli dopo il ritardo d'inizio (26 s in Q65-30)
            Mode::Q65 => Mode::Q65.tx_delay() + Duration::from_millis(q65_message_ms(q65_period_s())),
            // il quadro si ripete fino a fine periodo: si puo' partire fino ai 4/5 del periodo (12 s su 15)
            Mode::Msk144 => Duration::from_millis(u64::from(msk144_period_s()) * 800),
            // 160 simboli dopo il ritardo d'inizio
            Mode::Fst4 => Mode::Fst4.tx_delay() + Duration::from_millis(fst4_message_ms(fst4_period_s(false))),
            Mode::Fst4w => Mode::Fst4w.tx_delay() + Duration::from_millis(fst4_message_ms(fst4_period_s(true))),
            // WSPR: il radiofaro parte solo a inizio periodo (WSJT-X lo arma con `m_bTxTime` dal pianificatore, `WSPRBandHopping.cpp:256-272`);
            // un secondo di margine oltre il ritardo d'inizio
            Mode::Wspr => Duration::from_millis(2_000),
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
