//! Modo e periodo di trasmissione.
//!
//! Il tempo e' sempre dato dal chiamante in secondi UTC. Un periodo (`Slot`) e' l'indice del
//! tratto di 15 s (FT8) o 7,5 s (FT4) contato dall'inizio dell'epoca UNIX; pari e dispari si
//! leggono dall'indice, come il pulsante `TxMinuteButton` di JTDX.

use std::sync::atomic::{AtomicU8, AtomicU32, Ordering};

/// Periodo di Q65 in secondi (15, 30, 60, 120, 300): uno solo per programma, lo sceglie il motore.
static Q65_PERIOD_S: AtomicU32 = AtomicU32::new(30);
/// Sottomodo di Q65 (0 = A ... 4 = E).
static Q65_SUBMODE: AtomicU8 = AtomicU8::new(0);

/// Sceglie periodo e sottomodo di Q65; valori fuori campo si ignorano.
pub fn set_q65(period_s: u32, submode: u8) {
    if matches!(period_s, 15 | 30 | 60 | 120 | 300) {
        Q65_PERIOD_S.store(period_s, Ordering::Relaxed);
    }
    if submode <= 4 {
        Q65_SUBMODE.store(submode, Ordering::Relaxed);
    }
}

/// Periodo di Q65 in uso, in secondi.
pub fn q65_period_s() -> u32 {
    Q65_PERIOD_S.load(Ordering::Relaxed)
}

/// Sottomodo di Q65 in uso (0 = A ... 4 = E).
pub fn q65_submode() -> u8 {
    Q65_SUBMODE.load(Ordering::Relaxed)
}

/// Periodo di MSK144 in secondi (5, 10, 15, 30: `ui->sbTR->values ({5, 10, 15, 30})` di WSJT-X, `mainwindow.cpp:12081`); lo sceglie il motore.
static MSK144_PERIOD_S: AtomicU32 = AtomicU32::new(15);

/// I periodi di MSK144 che WSJT-X propone (`mainwindow.cpp:12081`).
pub const MSK144_PERIODS_S: [u32; 4] = [5, 10, 15, 30];

/// Sceglie il periodo di MSK144; valori fuori campo si ignorano (`false`).
pub fn set_msk144_period_s(period_s: u32) -> bool {
    let ok = MSK144_PERIODS_S.contains(&period_s);
    if ok {
        MSK144_PERIOD_S.store(period_s, Ordering::Relaxed);
    }
    ok
}

/// Periodo di MSK144 in uso, in secondi.
pub fn msk144_period_s() -> u32 {
    MSK144_PERIOD_S.load(Ordering::Relaxed)
}

/// Campioni per simbolo di Q65 a 12 kHz (`q65params`).
fn q65_nsps(period_s: u32) -> f64 {
    match period_s {
        15 => 1_800.0,
        30 => 3_600.0,
        60 => 7_200.0,
        120 => 16_000.0,
        _ => 41_472.0,
    }
}

/// Fine del messaggio Q65 dall'inizio del periodo, in secondi: 0,5 s (1 s dai 60 s in su) piu' 85 simboli.
pub fn q65_message_end_s() -> f64 {
    let p = q65_period_s();
    (if p >= 60 { 1.0 } else { 0.5 }) + 85.0 * q65_nsps(p) / 12_000.0
}

/// Larghezza del segnale Q65 in uso (65 toni), in hertz.
pub fn q65_bandwidth_hz() -> f64 {
    65.0 * 12_000.0 / q65_nsps(q65_period_s()) * f64::from(1u32 << q65_submode())
}

/// Modo digitale: cambia solo la durata del periodo, la logica dei QSO e' la stessa.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Mode {
    Ft8,
    Ft4,
    /// JT65 (sottomodo A) e JT9: periodi di un minuto, il messaggio comincia a un secondo dall'inizio.
    Jt65,
    Jt9,
    /// JT4 (sottomodi A-G): un minuto, come JT65.
    Jt4,
    /// Q65: periodi da 15 a 300 s ([`set_q65`]), messaggi a 77 bit come FT8.
    Q65,
    /// MSK144 (meteor scatter): periodi di 5, 10, 15 o 30 s ([`set_msk144_period_s`]), quadro di 72 ms ripetuto per tutto il periodo.
    Msk144,
}

impl Mode {
    /// Durata del periodo in millisecondi.
    pub fn period_ms(self) -> u64 {
        match self {
            Mode::Ft8 => 15_000,
            Mode::Msk144 => u64::from(msk144_period_s()) * 1000,
            Mode::Ft4 => 7_500,
            Mode::Jt65 | Mode::Jt9 | Mode::Jt4 => 60_000,
            Mode::Q65 => u64::from(q65_period_s()) * 1000,
        }
    }

    pub fn period_s(self) -> f64 {
        self.period_ms() as f64 / 1000.0
    }

    /// Nome del modo come lo scrive il log ADIF.
    pub fn name(self) -> &'static str {
        match self {
            Mode::Ft8 => "FT8",
            Mode::Ft4 => "FT4",
            Mode::Jt65 => "JT65",
            Mode::Jt9 => "JT9",
            Mode::Jt4 => "JT4",
            Mode::Q65 => "Q65",
            Mode::Msk144 => "MSK144",
        }
    }
}

/// Un periodo di ricezione o trasmissione.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Slot(pub u64);

impl Slot {
    /// Il periodo che contiene l'istante `utc_s` (secondi UTC dall'epoca UNIX).
    pub fn at(mode: Mode, utc_s: f64) -> Slot {
        let ms = (utc_s.max(0.0) * 1000.0).floor() as u64;
        Slot(ms / mode.period_ms())
    }

    /// Inizio del periodo in millisecondi UTC.
    pub fn start_ms(self, mode: Mode) -> u64 {
        self.0.saturating_mul(mode.period_ms())
    }

    /// Inizio del periodo in secondi UTC.
    pub fn start_s(self, mode: Mode) -> f64 {
        self.start_ms(mode) as f64 / 1000.0
    }

    /// Periodo «pari» (primo del minuto in FT8: :00 e :30; in FT4 :00, :15, :30, :45).
    pub fn is_even(self) -> bool {
        self.0.is_multiple_of(2)
    }

    pub fn next(self) -> Slot {
        Slot(self.0 + 1)
    }

    /// Quanti periodi separano `self` da `earlier` (0 se `earlier` e' successivo).
    pub fn since(self, earlier: Slot) -> u64 {
        self.0.saturating_sub(earlier.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ft8_slots_are_15_seconds() {
        assert_eq!(Slot::at(Mode::Ft8, 0.0), Slot(0));
        assert_eq!(Slot::at(Mode::Ft8, 14.99), Slot(0));
        assert_eq!(Slot::at(Mode::Ft8, 15.0), Slot(1));
        assert!(Slot::at(Mode::Ft8, 30.2).is_even());
        assert!(!Slot::at(Mode::Ft8, 45.0).is_even());
    }

    #[test]
    fn ft4_slots_are_seven_and_a_half_seconds() {
        assert_eq!(Slot::at(Mode::Ft4, 7.49), Slot(0));
        assert_eq!(Slot::at(Mode::Ft4, 7.5), Slot(1));
        assert_eq!(Slot(3).start_ms(Mode::Ft4), 22_500);
        assert!((Slot(3).start_s(Mode::Ft4) - 22.5).abs() < 1e-9);
    }

    #[test]
    fn negative_time_is_clamped() {
        assert_eq!(Slot::at(Mode::Ft8, -5.0), Slot(0));
    }
}
