/// Nome canonico della banda (`20m`), da `20`, `20M` o ` 20m `; `None` se non e' una banda nota.
pub fn canonical_band(band: &str) -> Option<String> {
    let band = band.trim().to_lowercase();
    let digits = band.strip_suffix('m').unwrap_or(&band);
    let name = format!("{digits}m");
    ft8_frequency(&name).map(|_| name)
}

/// Frequenza di dial FT8 della banda (Hz), come le frequenze predefinite di JTDX.
pub fn ft8_frequency(band: &str) -> Option<u64> {
    let band = band.trim().to_lowercase();
    let hz = match band.strip_suffix('m').unwrap_or(&band) {
        "160" => 1_840_000,
        "80" => 3_573_000,
        "60" => 5_357_000,
        "40" => 7_074_000,
        "30" => 10_136_000,
        "20" => 14_074_000,
        "17" => 18_100_000,
        "15" => 21_074_000,
        "12" => 24_915_000,
        "10" => 28_074_000,
        "6" => 50_313_000,
        "2" => 144_174_000,
        _ => return None,
    };
    Some(hz)
}

/// Se la frequenza (Hz) cade in una banda dei radioamatori, il nome della banda. Serve a non portare la radio
/// fuori banda per uno spot sbagliato.
pub fn amateur_band(hz: u64) -> Option<&'static str> {
    const BANDS: &[(&str, u64, u64)] = &[
        ("160m", 1_800_000, 2_000_000),
        ("80m", 3_500_000, 4_000_000),
        ("60m", 5_250_000, 5_450_000),
        ("40m", 7_000_000, 7_300_000),
        ("30m", 10_100_000, 10_150_000),
        ("20m", 14_000_000, 14_350_000),
        ("17m", 18_068_000, 18_168_000),
        ("15m", 21_000_000, 21_450_000),
        ("12m", 24_890_000, 24_990_000),
        ("10m", 28_000_000, 29_700_000),
        ("6m", 50_000_000, 54_000_000),
        ("2m", 144_000_000, 148_000_000),
    ];
    BANDS.iter().find(|(_, lo, hi)| hz >= *lo && hz <= *hi).map(|(name, _, _)| *name)
}

#[cfg(test)]
mod amateur_tests {
    use super::*;

    #[test]
    fn finds_the_amateur_band_or_nothing() {
        assert_eq!(amateur_band(14_074_000), Some("20m"));
        assert_eq!(amateur_band(7_143_700), Some("40m"));
        assert_eq!(amateur_band(28_451_600), Some("10m"));
        assert_eq!(amateur_band(15_000_000), None);
        assert_eq!(amateur_band(0), None);
    }
}
