//! Programmazione delle bande per orario (UTC), come la scheda Programmazione di JTDX.

use serde::{Deserialize, Serialize};

use crate::bands::canonical_band;

pub const MAX_ENTRIES: usize = 5;

/// Ora e minuto UTC da secondi Unix.
pub fn hm_of(unix_secs: u64) -> (u8, u8) {
    let day_secs = unix_secs % 86_400;
    ((day_secs / 3600) as u8, (day_secs / 60 % 60) as u8)
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScheduleEntry {
    pub hour: u8,
    pub minute: u8,
    pub band: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Schedule {
    enabled: bool,
    entries: Vec<ScheduleEntry>,
}

impl Schedule {
    pub fn enabled(&self) -> bool {
        self.enabled
    }

    pub fn entries(&self) -> &[ScheduleEntry] {
        &self.entries
    }

    /// Imposta acceso/spento e le righe; senza cambiare nulla se qualcosa non e' valido.
    pub fn set(&mut self, enabled: bool, entries: Vec<ScheduleEntry>) -> Result<(), &'static str> {
        if entries.len() > MAX_ENTRIES || entries.iter().any(|x| x.hour > 23 || x.minute > 59 || canonical_band(&x.band).is_none()) {
            return Err("programmazione non valida");
        }
        self.enabled = enabled;
        self.entries = entries.into_iter().map(|x| ScheduleEntry { band: canonical_band(&x.band).unwrap_or(x.band), ..x }).collect();
        Ok(())
    }

    /// La riga che scatta a questo minuto UTC, se la programmazione e' accesa.
    pub fn due(&self, hour: u8, minute: u8) -> Option<&ScheduleEntry> {
        if !self.enabled {
            return None;
        }
        self.entries.iter().find(|x| x.hour == hour && x.minute == minute)
    }
}
