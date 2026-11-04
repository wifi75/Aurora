//! Le bande dei radioamatori per frequenza.

// [nome, da (kHz), a (kHz)]
const BANDS: &[(&str, u64, u64)] = &[
    ("160m", 1800, 2000),
    ("80m", 3500, 4000),
    ("60m", 5250, 5450),
    ("40m", 7000, 7300),
    ("30m", 10_100, 10_150),
    ("20m", 14_000, 14_350),
    ("17m", 18_068, 18_168),
    ("15m", 21_000, 21_450),
    ("12m", 24_890, 24_990),
    ("10m", 28_000, 29_700),
    ("6m", 50_000, 54_000),
    ("2m", 144_000, 148_000),
];

pub fn band_of(hz: u64) -> Option<&'static str> {
    let khz = hz / 1000;
    BANDS.iter().find(|(_, from, to)| khz >= *from && khz <= *to).map(|(name, _, _)| *name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn band_of_known_bands_and_none_outside() {
        assert_eq!(band_of(14_074_000), Some("20m"));
        assert_eq!(band_of(21_074_000), Some("15m"));
        assert_eq!(band_of(7_074_000), Some("40m"));
        assert_eq!(band_of(50_313_000), Some("6m"));
        assert_eq!(band_of(0), None);
        assert_eq!(band_of(100_000_000), None);
    }
}
