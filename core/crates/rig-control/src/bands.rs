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
