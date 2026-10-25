use aurora_core::QsoRecord;
use logbook::Logbook;

fn qso(call: &str, band: &str, mode: &str) -> QsoRecord {
    QsoRecord {
        call: call.to_string(),
        grid: "PM95".to_string(),
        band: band.to_string(),
        mode: mode.to_string(),
        freq_hz: 14_074_000,
        rst_sent: "-08".to_string(),
        rst_rcvd: "-12".to_string(),
        time_ms: 45_015_000,
        time_on_unix_ms: 0,
        time_off_unix_ms: 0,
    }
}

#[test]
fn add_then_count() {
    let lb = Logbook::open_in_memory().unwrap();
    assert_eq!(lb.count().unwrap(), 0);
    lb.add(&qso("JA1XYZ", "20m", "FT8")).unwrap();
    lb.add(&qso("DL2ABC", "20m", "FT8")).unwrap();
    assert_eq!(lb.count().unwrap(), 2);
}

#[test]
fn dupe_same_call_band_mode() {
    let lb = Logbook::open_in_memory().unwrap();
    lb.add(&qso("JA1XYZ", "20m", "FT8")).unwrap();
    assert!(lb.is_dupe("JA1XYZ", "20m", "FT8").unwrap());
}

#[test]
fn not_dupe_on_other_band() {
    let lb = Logbook::open_in_memory().unwrap();
    lb.add(&qso("JA1XYZ", "20m", "FT8")).unwrap();
    assert!(!lb.is_dupe("JA1XYZ", "40m", "FT8").unwrap());
    assert!(!lb.is_dupe("JA1XYZ", "20m", "FT4").unwrap());
    assert!(!lb.is_dupe("DL2ABC", "20m", "FT8").unwrap());
}

#[test]
fn call_comparison_is_case_insensitive() {
    let lb = Logbook::open_in_memory().unwrap();
    lb.add(&qso("ja1xyz", "20m", "FT8")).unwrap();
    assert!(lb.is_dupe("JA1XYZ", "20m", "FT8").unwrap());
    assert!(lb.is_dupe("Ja1Xyz", "20m", "FT8").unwrap());
}

#[test]
fn adif_contains_call_band_mode_and_eoh_eor() {
    let lb = Logbook::open_in_memory().unwrap();
    lb.add(&qso("JA1XYZ", "20m", "FT8")).unwrap();
    let adif = lb.export_adif().unwrap();
    assert!(adif.contains("<EOH>"));
    assert!(adif.contains("<CALL:6>JA1XYZ"));
    assert!(adif.contains("<BAND:3>20m"));
    assert!(adif.contains("<MODE:3>FT8"));
    assert!(adif.contains("<FREQ:9>14.074000"));
    assert!(adif.contains("<TIME_ON:6>123015"));
    assert!(adif.contains("<EOR>"));
    let date_field = adif.split("<QSO_DATE:8>").nth(1).expect("QSO_DATE presente");
    assert!(date_field[..8].chars().all(|c| c.is_ascii_digit()));
}

#[test]
fn adif_length_is_in_bytes_for_utf8() {
    let lb = Logbook::open_in_memory().unwrap();
    let mut q = qso("JA1XYZ", "20m", "FT8");
    q.grid = "JN45é".to_string();
    lb.add(&q).unwrap();
    assert!(lb.export_adif().unwrap().contains("<GRIDSQUARE:6>JN45é"));
}

#[test]
fn reopening_keeps_data() {
    let path = std::env::temp_dir().join(format!(
        "aurora-logbook-test-{}-{}.db",
        std::process::id(),
        std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()
    ));
    {
        let lb = Logbook::open(&path).unwrap();
        lb.add(&qso("JA1XYZ", "20m", "FT8")).unwrap();
    }
    let lb = Logbook::open(&path).unwrap();
    assert_eq!(lb.count().unwrap(), 1);
    assert!(lb.is_dupe("JA1XYZ", "20m", "FT8").unwrap());
    drop(lb);
    let _ = std::fs::remove_file(&path);
}

#[test]
fn migration_sets_user_version_1() {
    let lb = Logbook::open_in_memory().unwrap();
    assert_eq!(lb.schema_version().unwrap(), 1);
}
