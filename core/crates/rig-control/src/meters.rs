//! I misuratori della radio (S-meter, potenza, ROS, ALC...) letti con i livelli di Hamlib (`l STRENGTH`, `l SWR`...).
//! Qui stanno il tipo, le conversioni di unita' e la memoria di quali livelli la radio non sa dare (cosi' non si richiedono all'infinito).

use crate::hamlib::error_code;
use crate::RigError;

/// Le letture della radio, ognuna presente solo se la radio la sa dare. Stesse unita' di `aurora_core::RadioMeters`.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Meters {
    /// Punti S, 0..=9.
    pub s_points: Option<f32>,
    /// Decibel sopra S9, 0..=60.
    pub s_over_db: Option<f32>,
    pub power_w: Option<f32>,
    pub swr: Option<f32>,
    /// 0.0..=1.0 della scala.
    pub alc: Option<f32>,
    pub comp_db: Option<f32>,
    pub id_a: Option<f32>,
    pub vdd_v: Option<f32>,
}

/// I livelli di Hamlib che si leggono.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Level {
    Strength,
    PowerWatts,
    PowerFraction,
    Swr,
    Alc,
    Comp,
    Id,
    Vdd,
}

impl Level {
    /// Il nome del livello nel comando `l`.
    pub fn name(self) -> &'static str {
        match self {
            Level::Strength => "STRENGTH",
            Level::PowerWatts => "RFPOWER_METER_WATTS",
            Level::PowerFraction => "RFPOWER_METER",
            Level::Swr => "SWR",
            Level::Alc => "ALC",
            Level::Comp => "COMP_METER",
            Level::Id => "ID_METER",
            Level::Vdd => "VD_METER",
        }
    }

    fn bit(self) -> u16 {
        1 << (self as u16)
    }
}

/// I livelli di trasmissione, nell'ordine in cui si leggono (i piu' utili per primi).
pub const TX_LEVELS: [Level; 5] = [Level::Swr, Level::Alc, Level::Comp, Level::Id, Level::Vdd];

/// `STRENGTH` di Hamlib (dB rispetto a S9) in punti S e dB oltre S9: S9 = 0 dB, 6 dB per punto S sotto S9, S0 a -54 dB.
pub fn strength_to_s(db: f32) -> (f32, f32) {
    if !db.is_finite() {
        return (0.0, 0.0);
    }
    if db > 0.0 { (9.0, db.min(60.0)) } else { ((9.0 + db / 6.0).clamp(0.0, 9.0), 0.0) }
}

/// Il codice di errore di Hamlib che dice «questo livello non c'e'» (parametro non valido, non implementato, non disponibile).
pub fn is_unsupported(error: &RigError) -> bool {
    matches!(error_code(error), Some(-1 | -4 | -11))
}

/// Quali livelli la radio non da' e quanti guasti di fila ha avuto ciascuno.
#[derive(Debug, Default)]
pub struct LevelMemory {
    unsupported: u16,
    fails: [u8; 8],
}

/// Dopo tanti guasti consecutivi (timeout, rifiuti) un livello si considera non disponibile.
const MAX_FAILS: u8 = 5;

impl LevelMemory {
    pub fn supported(&self, level: Level) -> bool {
        self.unsupported & level.bit() == 0
    }

    pub fn mark_unsupported(&mut self, level: Level) {
        self.unsupported |= level.bit();
    }

    pub fn ok(&mut self, level: Level) {
        self.fails[level as usize] = 0;
    }

    /// Registra l'esito negativo di una lettura. `true` se il livello e' ora da non chiedere piu'.
    pub fn failed(&mut self, level: Level, error: &RigError) -> bool {
        if is_unsupported(error) {
            self.mark_unsupported(level);
            return true;
        }
        let n = &mut self.fails[level as usize];
        *n = n.saturating_add(1);
        if *n >= MAX_FAILS {
            self.mark_unsupported(level);
            return true;
        }
        false
    }
}

/// Scrive una lettura nel quadro dei misuratori.
pub fn store(meters: &mut Meters, level: Level, value: f32) {
    if !value.is_finite() {
        return;
    }
    match level {
        Level::Strength => {
            let (points, over) = strength_to_s(value);
            meters.s_points = Some(points);
            meters.s_over_db = Some(over);
        }
        Level::PowerWatts => meters.power_w = Some(value.max(0.0)),
        Level::PowerFraction => {}
        Level::Swr => meters.swr = Some(value.max(1.0)),
        Level::Alc => meters.alc = Some(value.max(0.0)),
        Level::Comp => meters.comp_db = Some(value.max(0.0)),
        Level::Id => meters.id_a = Some(value.max(0.0)),
        Level::Vdd => meters.vdd_v = Some(value.max(0.0)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strength_is_split_into_s_points_and_db_over_s9() {
        assert_eq!(strength_to_s(0.0), (9.0, 0.0));
        assert_eq!(strength_to_s(20.0), (9.0, 20.0));
        assert_eq!(strength_to_s(-6.0), (8.0, 0.0));
        assert_eq!(strength_to_s(-30.0), (4.0, 0.0));
        assert_eq!(strength_to_s(-54.0), (0.0, 0.0));
        assert_eq!(strength_to_s(-90.0), (0.0, 0.0), "sotto S0 resta S0");
        assert_eq!(strength_to_s(80.0), (9.0, 60.0), "oltre +60 dB si ferma al fondo scala");
        assert_eq!(strength_to_s(f32::NAN), (0.0, 0.0));
    }

    #[test]
    fn unsupported_levels_are_remembered_and_flaky_ones_give_up_after_a_while() {
        let mut memory = LevelMemory::default();
        assert!(memory.supported(Level::Swr));
        assert!(memory.failed(Level::Swr, &RigError("l: funzione non disponibile (-11)".into())));
        assert!(!memory.supported(Level::Swr) && memory.supported(Level::Alc));
        let timeout = RigError("l: timeout: la radio non risponde (-5)".into());
        for _ in 0..4 {
            assert!(!memory.failed(Level::Alc, &timeout));
        }
        memory.ok(Level::Alc);
        for _ in 0..4 {
            assert!(!memory.failed(Level::Alc, &timeout));
        }
        assert!(memory.failed(Level::Alc, &timeout));
        assert!(!memory.supported(Level::Alc));
    }

    #[test]
    fn store_applies_the_units_and_floors() {
        let mut m = Meters::default();
        store(&mut m, Level::Strength, -12.0);
        store(&mut m, Level::Swr, 0.0);
        store(&mut m, Level::PowerWatts, 24.5);
        assert_eq!((m.s_points, m.s_over_db, m.swr, m.power_w), (Some(7.0), Some(0.0), Some(1.0), Some(24.5)));
    }
}
