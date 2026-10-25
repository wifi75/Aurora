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
fn migration_sets_user_version_2() {
    let lb = Logbook::open_in_memory().unwrap();
    assert_eq!(lb.schema_version().unwrap(), 2);
}

fn station() -> logbook::adif::Station {
    logbook::adif::Station { call: "IU3ABC".into(), grid: "IO91wm".into(), tx_power: String::new() }
}

fn timed(call: &str, mode: &str, on_ms: u64) -> QsoRecord {
    QsoRecord { time_on_unix_ms: on_ms, time_off_unix_ms: on_ms + 44_000, time_ms: (on_ms % 86_400_000) as u32, ..qso(call, "20m", mode) }
}

// 2026-10-24 16:46:45 UTC e il giorno dopo
const DAY1: u64 = 1_792_860_405_000;
const DAY2: u64 = DAY1 + 86_400_000;

#[test]
fn export_uses_the_jtdx_file_layout_with_real_times_and_filters_by_date() {
    let lb = Logbook::open_in_memory().unwrap();
    lb.add(&timed("JA1XYZ", "FT8", DAY1)).unwrap();
    lb.add(&timed("DL2ABC", "FT4", DAY2)).unwrap();
    let (all, n) = lb.export_jtdx_adif(&station(), None, None).unwrap();
    assert_eq!(n, 2);
    assert!(all.starts_with("WSJT-X ADIF Export<eoh>\r\n<call:6>JA1XYZ <gridsquare:4>PM95 <mode:3>FT8 <rst_sent:3>-08 <rst_rcvd:3>-12 <qso_date:8>20261024 <time_on:6>164645 <qso_date_off:8>20261024 <time_off:6>164729 <band:3>20m <freq:9>14.074000 <station_callsign:6>IU3ABC <my_gridsquare:6>IO91wm <eor>\r\n"), "{all}");
    assert!(all.contains("<mode:4>MFSK <submode:3>FT4"));
    let (only2, n2) = lb.export_jtdx_adif(&station(), Some("20261025"), None).unwrap();
    assert_eq!(n2, 1);
    assert!(only2.contains("DL2ABC") && !only2.contains("JA1XYZ"));
    let (none, n0) = lb.export_jtdx_adif(&station(), Some("20261024"), Some("20261024")).unwrap();
    assert_eq!(n0, 1);
    assert!(none.contains("JA1XYZ"));
    let (empty, n_empty) = lb.export_jtdx_adif(&station(), Some("20270101"), None).unwrap();
    assert_eq!((n_empty, empty.as_str()), (0, "WSJT-X ADIF Export<eoh>\r\n"));
}

#[test]
fn what_was_exported_can_be_imported_into_an_empty_log_unchanged() {
    let a = Logbook::open_in_memory().unwrap();
    a.add(&timed("JA1XYZ", "FT8", DAY1)).unwrap();
    a.add(&timed("DL2ABC", "FT4", DAY2)).unwrap();
    let (text, _) = a.export_jtdx_adif(&station(), None, None).unwrap();
    let b = Logbook::open_in_memory().unwrap();
    let r = b.import_adif(&text).unwrap();
    assert_eq!((r.read, r.added, r.duplicates, r.skipped), (2, 2, 0, 0));
    let (again, _) = b.export_jtdx_adif(&station(), None, None).unwrap();
    assert_eq!(again, text, "esportare, importare ed esportare di nuovo da' lo stesso file");
}

#[test]
fn importing_twice_or_over_existing_qsos_adds_no_duplicates() {
    let lb = Logbook::open_in_memory().unwrap();
    lb.add(&timed("JA1XYZ", "FT8", DAY1)).unwrap();
    let file = "WSJT-X ADIF Export<eoh>\r\n\
        <call:6>JA1XYZ <gridsquare:4>PM95 <mode:3>FT8 <qso_date:8>20261024 <time_on:6>164700 <band:3>20m <freq:9>14.074000 <eor>\r\n\
        <call:6>DL2ABC <gridsquare:4>JO62 <mode:4>MFSK <submode:3>FT4 <rst_sent:3>-01 <qso_date:8>20261024 <time_on:6>170000 <band:3>20m <eor>\r\n\
        <call:6>DL2ABC <mode:4>MFSK <submode:3>FT4 <qso_date:8>20261024 <time_on:6>170030 <band:3>20m <eor>\r\n\
        <call:6>OE1AAA <mode:3>FT8 <qso_date:8>20261024 <time_on:6>171000 <eor>\r\n\
        <call:6>OE1BBB <mode:3>FT8 <band:3>40m <eor>\r\n";
    let first = lb.import_adif(file).unwrap();
    assert_eq!((first.read, first.added, first.duplicates, first.skipped), (5, 1, 2, 2), "il QSO gia' nel log e il doppio nel file sono doppioni; senza banda o data si scarta");
    let second = lb.import_adif(file).unwrap();
    assert_eq!((second.added, second.duplicates), (0, 3));
    assert_eq!(lb.count().unwrap(), 2);
    let all = lb.all_qsos().unwrap();
    let dl = all.iter().find(|q| q.call == "DL2ABC").unwrap();
    assert_eq!((dl.mode.as_str(), dl.band.as_str(), dl.date.as_str(), dl.time_on.as_str()), ("FT4", "20m", "20261024", "170000"));
}

#[test]
fn other_modes_and_confirmations_survive_the_import() {
    let lb = Logbook::open_in_memory().unwrap();
    let file = "<call:5>K1ABC<mode:3>SSB<band:3>15m<qso_date:8>20261001<time_on:4>1530<qsl_rcvd:1>Y<eor>\n<call:5>K1ABD<mode:2>CW<band:3>15m<qso_date:8>20261001<time_on:4>1540<lotw_qsl_rcvd:1>N<eor>\n";
    assert_eq!(lb.import_adif(file).unwrap().added, 2);
    let all = lb.all_qsos().unwrap();
    assert_eq!((all[0].mode.as_str(), all[0].time_on.as_str(), all[0].confirmed), ("SSB", "153000", Some(true)));
    assert_eq!(all[1].confirmed, Some(false));
    assert!(lb.add(&qso("JA1XYZ", "20m", "FT8")).is_ok());
    assert_eq!(lb.all_qsos().unwrap()[2].confirmed, None, "un QSO registrato da Aurora non ha informazioni sulle conferme");
}

#[test]
fn a_v1_database_is_migrated_keeping_its_rows() {
    let path = std::env::temp_dir().join(format!("aurora-logbook-v1-{}.db", std::process::id()));
    let _ = std::fs::remove_file(&path);
    {
        let conn = rusqlite::Connection::open(&path).unwrap();
        conn.execute_batch(
            "CREATE TABLE qso (id INTEGER PRIMARY KEY, call TEXT NOT NULL, grid TEXT NOT NULL, band TEXT NOT NULL, mode TEXT NOT NULL, freq_hz INTEGER NOT NULL,
                               rst_sent TEXT NOT NULL, rst_rcvd TEXT NOT NULL, time_ms INTEGER NOT NULL, date TEXT NOT NULL);
             CREATE INDEX qso_call_band_mode ON qso (call, band, mode);
             INSERT INTO qso VALUES (1, 'JA1XYZ', 'PM95', '20m', 'FT8', 14074000, '-08', '-12', 45015000, '20261024');
             PRAGMA user_version = 1;",
        )
        .unwrap();
    }
    let lb = Logbook::open(&path).unwrap();
    assert_eq!(lb.schema_version().unwrap(), 2);
    let all = lb.all_qsos().unwrap();
    assert_eq!((all.len(), all[0].time_on.as_str(), all[0].date_off.as_str(), all[0].confirmed), (1, "123015", "20261024", None));
    drop(lb);
    let _ = std::fs::remove_file(&path);
}
