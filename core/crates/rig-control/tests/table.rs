use rig_control::schedule::{hm_of, Schedule, ScheduleEntry};
use rig_control::table::{Entry, FrequencyTable};

#[test]
fn the_default_table_has_ft8_and_ft4_on_the_usual_bands() {
    let t = FrequencyTable::default();
    assert_eq!(t.frequency("20m", "FT8"), Some(14_074_000));
    assert_eq!(t.frequency("20m", "FT4"), Some(14_080_000));
    assert_eq!(t.frequency("40m", "FT4"), Some(7_047_500));
    assert_eq!(t.frequency("15m", "ft8"), Some(21_074_000), "il modo non distingue le maiuscole");
    assert_eq!(t.frequency("15", "FT8"), Some(21_074_000), "la banda si scrive anche senza la m");
    assert_eq!(t.frequency("13m", "FT8"), None);
    assert!(t.is_valid());
}

#[test]
fn a_user_table_replaces_the_default_and_is_validated() {
    let mut t = FrequencyTable::default();
    t.set(vec![Entry::new("20m", "FT8", 14_075_000)]).unwrap();
    assert_eq!(t.frequency("20m", "FT8"), Some(14_075_000));
    assert_eq!(t.frequency("40m", "FT8"), None);
}

#[test]
fn invalid_tables_are_refused() {
    let mut t = FrequencyTable::default();
    let before = t.entries().to_vec();
    for bad in [
        vec![],
        vec![Entry::new("13m", "FT8", 14_074_000)],
        vec![Entry::new("20m", "WSPR", 14_095_600)],
        vec![Entry::new("20m", "FT8", 5)],
        vec![Entry::new("20m", "FT8", 9_000_000_000)],
        vec![Entry::new("20m", "FT8", 14_074_000), Entry::new("20M", "ft8", 14_074_000)],
    ] {
        assert!(t.set(bad).is_err());
    }
    assert_eq!(t.entries(), &before[..], "una tabella non valida non cambia nulla");
}

#[test]
fn a_frequency_must_belong_to_its_band() {
    let mut t = FrequencyTable::default();
    assert!(t.set(vec![Entry::new("20m", "FT8", 7_074_000)]).is_err(), "7 MHz non e' 20 m");
}

#[test]
fn the_schedule_fires_at_the_minute_and_only_when_enabled() {
    let mut s = Schedule::default();
    s.set(true, vec![ScheduleEntry { hour: 6, minute: 30, band: "20m".into() }, ScheduleEntry { hour: 18, minute: 0, band: "40m".into() }]).unwrap();
    assert_eq!(s.due(6, 30).map(|e| e.band.as_str()), Some("20m"));
    assert_eq!(s.due(18, 0).map(|e| e.band.as_str()), Some("40m"));
    assert!(s.due(6, 31).is_none());
    s.set(false, s.entries().to_vec()).unwrap();
    assert!(s.due(6, 30).is_none(), "spenta non scatta");
}

#[test]
fn invalid_schedules_are_refused() {
    let mut s = Schedule::default();
    for bad in [
        vec![ScheduleEntry { hour: 24, minute: 0, band: "20m".into() }],
        vec![ScheduleEntry { hour: 1, minute: 60, band: "20m".into() }],
        vec![ScheduleEntry { hour: 1, minute: 0, band: "13m".into() }],
        (0..6).map(|i| ScheduleEntry { hour: i, minute: 0, band: "20m".into() }).collect(),
    ] {
        assert!(s.set(true, bad).is_err());
    }
    assert!(s.entries().is_empty());
}

#[test]
fn hour_and_minute_come_from_unix_seconds_in_utc() {
    assert_eq!(hm_of(0), (0, 0));
    assert_eq!(hm_of(86_399), (23, 59));
    assert_eq!(hm_of(6 * 3600 + 30 * 60 + 59), (6, 30));
    assert_eq!(hm_of(86_400 * 20_000 + 12 * 3600 + 5 * 60), (12, 5));
}

#[test]
fn band_of_finds_the_band_a_dial_frequency_belongs_to() {
    let t = rig_control::table::FrequencyTable::default();
    assert_eq!(t.band_of(21_074_000).as_deref(), Some("15m"));
    assert_eq!(t.band_of(14_074_000).as_deref(), Some("20m"));
    assert_eq!(t.band_of(50_313_000).as_deref(), Some("6m"));
    assert_eq!(t.band_of(12_345_678), None);
    assert_eq!(t.band_of(0), None);
}
