//! Frequenze di lavoro per banda e modo, come la scheda Frequenze di JTDX.

use serde::{Deserialize, Serialize};

use crate::bands::canonical_band;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Entry {
    pub band: String,
    pub mode: String,
    pub hz: u64,
}

/// Limiti (Hz) entro cui deve cadere la frequenza di ogni banda.
fn band_limits(band: &str) -> Option<(u64, u64)> {
    Some(match band {
        "160m" => (1_800_000, 2_000_000),
        "80m" => (3_500_000, 4_000_000),
        "60m" => (5_250_000, 5_450_000),
        "40m" => (7_000_000, 7_300_000),
        "30m" => (10_100_000, 10_150_000),
        "20m" => (14_000_000, 14_350_000),
        "17m" => (18_068_000, 18_168_000),
        "15m" => (21_000_000, 21_450_000),
        "12m" => (24_890_000, 24_990_000),
        "10m" => (28_000_000, 29_700_000),
        "6m" => (50_000_000, 54_000_000),
        "2m" => (144_000_000, 148_000_000),
        _ => return None,
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FrequencyTable(Vec<Entry>);

fn e(band: &str, mode: &str, hz: u64) -> Entry {
    Entry { band: band.into(), mode: mode.into(), hz }
}

impl Default for FrequencyTable {
    fn default() -> Self {
        FrequencyTable(vec![
            e("160m", "FT8", 1_840_000),
            e("160m", "FT4", 1_840_000),
            e("80m", "FT8", 3_573_000),
            e("80m", "FT4", 3_575_000),
            e("60m", "FT8", 5_357_000),
            e("40m", "FT8", 7_074_000),
            e("40m", "FT4", 7_047_500),
            e("30m", "FT8", 10_136_000),
            e("30m", "FT4", 10_140_000),
            e("20m", "FT8", 14_074_000),
            e("20m", "FT4", 14_080_000),
            e("17m", "FT8", 18_100_000),
            e("17m", "FT4", 18_104_000),
            e("15m", "FT8", 21_074_000),
            e("15m", "FT4", 21_140_000),
            e("12m", "FT8", 24_915_000),
            e("12m", "FT4", 24_919_000),
            e("10m", "FT8", 28_074_000),
            e("10m", "FT4", 28_180_000),
            e("6m", "FT8", 50_313_000),
            e("6m", "FT4", 50_318_000),
            e("2m", "FT8", 144_174_000),
            e("2m", "FT4", 144_170_000),
        ])
    }
}

fn valid_entries(entries: &[Entry]) -> bool {
    if entries.is_empty() {
        return false;
    }
    let mut seen = std::collections::HashSet::new();
    entries.iter().all(|entry| {
        let Some(band) = canonical_band(&entry.band) else { return false };
        let mode = entry.mode.trim().to_uppercase();
        let in_band = band_limits(&band).is_some_and(|(lo, hi)| (lo..=hi).contains(&entry.hz));
        in_band && matches!(mode.as_str(), "FT8" | "FT4") && seen.insert((band, mode))
    })
}

impl FrequencyTable {
    pub fn entries(&self) -> &[Entry] {
        &self.0
    }

    pub fn is_valid(&self) -> bool {
        valid_entries(&self.0)
    }

    /// La banda in cui cade una frequenza di dial, tra quelle della tabella.
    pub fn band_of(&self, hz: u64) -> Option<String> {
        self.0.iter().find(|x| band_limits(&x.band).is_some_and(|(lo, hi)| (lo..=hi).contains(&hz))).map(|x| x.band.clone())
    }

    /// Frequenza di dial per banda e modo.
    pub fn frequency(&self, band: &str, mode: &str) -> Option<u64> {
        let band = canonical_band(band)?;
        let mode = mode.trim().to_uppercase();
        self.0.iter().find(|x| x.band == band && x.mode.to_uppercase() == mode).map(|x| x.hz)
    }

    /// Sostituisce la tabella se e' valida (bande e modi noti, frequenza nella banda, senza doppioni).
    pub fn set(&mut self, entries: Vec<Entry>) -> Result<(), &'static str> {
        if !valid_entries(&entries) {
            return Err("tabella delle frequenze non valida");
        }
        self.0 = entries
            .into_iter()
            .map(|x| Entry { band: canonical_band(&x.band).unwrap_or(x.band), mode: x.mode.trim().to_uppercase(), hz: x.hz })
            .collect();
        Ok(())
    }
}
