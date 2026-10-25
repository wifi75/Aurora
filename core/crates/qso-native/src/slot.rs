//! Modo e periodo di trasmissione.
//!
//! Il tempo e' sempre dato dal chiamante in secondi UTC. Un periodo (`Slot`) e' l'indice del
//! tratto di 15 s (FT8) o 7,5 s (FT4) contato dall'inizio dell'epoca UNIX; pari e dispari si
//! leggono dall'indice, come il pulsante `TxMinuteButton` di JTDX.

/// Modo digitale: cambia solo la durata del periodo, la logica dei QSO e' la stessa.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Mode {
    Ft8,
    Ft4,
}

impl Mode {
    /// Durata del periodo in millisecondi.
    pub fn period_ms(self) -> u64 {
        match self {
            Mode::Ft8 => 15_000,
            Mode::Ft4 => 7_500,
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
