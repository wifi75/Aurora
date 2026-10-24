use rig_control::bands::ft8_frequency;
use rig_control::omnirig::script;

#[test]
fn every_jtdx_band_has_its_ft8_dial_frequency() {
    for (band, hz) in [("160m", 1_840_000), ("80m", 3_573_000), ("40m", 7_074_000), ("20m", 14_074_000), ("15m", 21_074_000), ("10m", 28_074_000), ("6m", 50_313_000), ("2m", 144_174_000)] {
        assert_eq!(ft8_frequency(band), Some(hz), "{band}");
    }
}

#[test]
fn bands_are_read_with_or_without_the_m_and_case() {
    assert_eq!(ft8_frequency("20"), Some(14_074_000));
    assert_eq!(ft8_frequency(" 20M "), Some(14_074_000));
}

#[test]
fn unknown_bands_are_refused() {
    for bad in ["", "m", "13m", "abc", "20mm", "0"] {
        assert_eq!(ft8_frequency(bad), None, "{bad}");
    }
}

#[test]
fn the_omnirig_script_carries_only_the_number_and_checks_the_radio_is_online() {
    let s = script(14_074_000);
    assert!(s.contains("SetSimplexMode(14074000)"), "{s}");
    assert!(s.contains("Split = 65536"), "cambiando banda lo split va spento: {s}");
    assert!(s.contains("Rig1.Status -ne 4"), "{s}");
}
