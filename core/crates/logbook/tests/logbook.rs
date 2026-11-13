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
        ..Default::default()
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
fn migration_sets_user_version_4() {
    let lb = Logbook::open_in_memory().unwrap();
    assert_eq!(lb.schema_version().unwrap(), 5);
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
fn entries_list_the_newest_first_and_the_export_keeps_only_the_chosen_ids() {
    let lb = Logbook::open_in_memory().unwrap();
    let a = lb.add(&timed("JA1XYZ", "FT8", DAY1)).unwrap();
    let b = lb.add(&timed("DL2ABC", "FT4", DAY2)).unwrap();
    let c = lb.add(&timed("K1ABC", "FT8", DAY2 + 60_000)).unwrap();
    let list = lb.entries().unwrap();
    assert_eq!(list.iter().map(|e| e.id).collect::<Vec<_>>(), vec![c, b, a]);
    assert_eq!((list[2].date.as_str(), list[2].time.as_str(), list[2].band.as_str(), list[2].mode.as_str()), ("20261024", "164645", "20m", "FT8"));
    let (text, n) = lb.export_jtdx_adif_selected(&station(), None, None, Some(&[a, c, 9999])).unwrap();
    assert_eq!(n, 2);
    assert!(text.contains("JA1XYZ") && text.contains("K1ABC") && !text.contains("DL2ABC"));
    // il filtro per data vale insieme a quello per numero
    let (_, n) = lb.export_jtdx_adif_selected(&station(), Some("20261025"), None, Some(&[a, c])).unwrap();
    assert_eq!(n, 1);
    let (_, none) = lb.export_jtdx_adif_selected(&station(), None, None, Some(&[])).unwrap();
    assert_eq!(none, 0);
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
    assert_eq!(lb.schema_version().unwrap(), 5);
    let all = lb.all_qsos().unwrap();
    assert_eq!((all.len(), all[0].time_on.as_str(), all[0].date_off.as_str(), all[0].confirmed), (1, "123015", "20261024", None));
    drop(lb);
    let _ = std::fs::remove_file(&path);
}

#[test]
fn qsos_are_deleted_by_id_and_the_log_can_be_cleared() {
    let book = Logbook::open_in_memory().expect("log");
    let q = |call: &str| QsoRecord { call: call.into(), band: "20m".into(), mode: "FT8".into(), ..Default::default() };
    let a = book.add(&q("K1ABC")).expect("a");
    let b = book.add(&q("DL1ABC")).expect("b");
    book.add(&q("JA1XYZ")).expect("c");
    let gone = book.delete(&[a, b, 999]).expect("cancellati");
    assert_eq!(gone.iter().map(|e| e.call.as_str()).collect::<Vec<_>>(), vec!["K1ABC", "DL1ABC"]);
    assert_eq!(book.count().expect("conta"), 1);
    assert_eq!(book.clear().expect("svuotato"), 1);
    assert_eq!(book.count().expect("conta"), 0);
}

#[test]
fn deleted_qsos_are_removed_from_the_adif_file_too() {
    let text = "Aurora\n<adif_ver:5>3.1.4 <eoh>\n<CALL:5>K1ABC <QSO_DATE:8>20261107 <TIME_ON:6>120015 <BAND:3>20m <MODE:3>FT8 <eor>\n<call:6>DL1ABC <qso_date:8>20261107 <time_on:6>120115 <band:3>20m <mode:3>FT8 <eor>\n";
    let gone = vec![logbook::LogEntry { id: 1, date: "20261107".into(), time: "120030".into(), call: "K1ABC".into(), band: "20m".into(), mode: "FT8".into(), freq_hz: 0, rst_sent: String::new(), rst_rcvd: String::new(), grid: String::new(), name: String::new(), qth: String::new(), comment: String::new() }];
    let (out, n) = logbook::adif_without(text, &gone);
    assert_eq!(n, 1);
    assert!(!out.contains("K1ABC") && out.contains("DL1ABC") && out.contains("<eoh>"), "{out}");
}

/// Un QSO con inizio e fine veri: 2026-10-24 16:46:45 UTC, 44 secondi.
fn logged(call: &str, mode: &str) -> QsoRecord {
    let on = 1_792_860_405_000;
    QsoRecord { time_ms: (on % 86_400_000) as u32, time_on_unix_ms: on, time_off_unix_ms: on + 44_000, comment: "FT8  Sent: -08  Rcvd: -12".into(), ..qso(call, "20m", mode) }
}

#[test]
fn a_qso_is_corrected_by_id_and_keeps_its_length() {
    let book = Logbook::open_in_memory().expect("log");
    let id = book.add(&logged("K1ABX", "FT8")).expect("aggiunto");
    let other = book.add(&logged("DL1ABC", "FT8")).expect("aggiunto");
    let before = book.entry(id).expect("letto").expect("c'e'");
    assert_eq!((before.date.as_str(), before.time.as_str(), before.comment.as_str()), ("20261024", "164645", "FT8  Sent: -08  Rcvd: -12"));
    let mut edit = logbook::LogEdit::from_entry(&before);
    edit.call = "k1abc".into();
    edit.time = "235950".into();
    edit.grid = "FN42".into();
    edit.name = "Joe".into();
    let change = book.update(id, &edit).expect("corretto").expect("c'e'");
    assert_eq!(change.old, before);
    assert_eq!((change.new.call.as_str(), change.new.time.as_str(), change.new.grid.as_str(), change.new.name.as_str()), ("K1ABC", "235950", "FN42", "Joe"));
    // la fine si sposta con l'inizio e passa al giorno dopo
    assert_eq!((change.date_off.as_str(), change.time_off.as_str()), ("20261025", "000034"));
    assert_eq!(book.entry(id).expect("letto"), Some(change.new.clone()));
    // i «gia' lavorati» leggono `all_qsos`: il nominativo vecchio non c'e' piu'
    let all = book.all_qsos().expect("tutti");
    assert!(all.iter().any(|q| q.call == "K1ABC" && q.time_on == "235950" && q.date_off == "20261025") && !all.iter().any(|q| q.call == "K1ABX"), "{all:?}");
    assert!(book.is_dupe("K1ABC", "20m", "FT8").expect("doppione") && !book.is_dupe("K1ABX", "20m", "FT8").expect("doppione"));
    // l'altro QSO non e' stato toccato
    assert_eq!(book.entry(other).expect("letto").map(|e| e.call), Some("DL1ABC".to_string()));
    assert_eq!(book.count().expect("conta"), 2);
}

#[test]
fn correcting_a_qso_that_is_not_there_changes_nothing() {
    let book = Logbook::open_in_memory().expect("log");
    let id = book.add(&logged("K1ABC", "FT8")).expect("aggiunto");
    let edit = logbook::LogEdit { call: "ZZ9ZZ".into(), ..logbook::LogEdit::from_entry(&book.entry(id).expect("letto").expect("c'e'")) };
    assert_eq!(book.update(999, &edit).expect("nessun errore"), None);
    assert_eq!(book.entry(999).expect("letto"), None);
    assert_eq!(book.entry(id).expect("letto").map(|e| e.call), Some("K1ABC".to_string()));
    // una data che non esiste e' un errore e non cambia nulla
    let bad = logbook::LogEdit { date: "20261340".into(), ..edit };
    assert!(book.update(id, &bad).is_err());
    assert_eq!(book.entry(id).expect("letto").map(|e| e.date), Some("20261024".to_string()));
    assert!(book.delete(&[999]).expect("niente").is_empty());
}

#[test]
fn the_adif_file_follows_the_correction_and_keeps_everything_else() {
    let station = logbook::adif::Station { call: "IU3ABC".into(), grid: "IO91wm".into(), tx_power: String::new() };
    let book = Logbook::open_in_memory().expect("log");
    let id = book.add(&logged("K1ABX", "FT8")).expect("aggiunto");
    book.add(&logged("DL1ABC", "FT4")).expect("aggiunto");
    // il file com'e' scritto da Aurora: intestazione e una riga per QSO, con un campo in piu' che il log non conosce
    let mut text = String::from(logbook::adif::JTDX_FILE_HEADER);
    text.push_str(&logbook::adif::jtdx_file_record(&logged("K1ABX", "FT8"), &station, "20261024").replace(" <eor>", " <app_x:3>a<b <eor>"));
    let untouched = logbook::adif::jtdx_file_record(&logged("DL1ABC", "FT4"), &station, "20261024");
    text.push_str(&untouched);

    let mut edit = logbook::LogEdit::from_entry(&book.entry(id).expect("letto").expect("c'e'"));
    edit.call = "K1ABC".into();
    edit.mode = "FT4".into();
    edit.date = "20261025".into();
    edit.time = "101500".into();
    edit.band = "40m".into();
    edit.freq_hz = 7_047_500;
    edit.rst_sent = "+03".into();
    edit.grid = String::new();
    edit.qth = "Boston".into();
    edit.comment = String::new();
    let change = book.update(id, &edit).expect("corretto").expect("c'e'");
    let (out, n) = logbook::adif_with_edit(&text, &change);
    assert_eq!(n, 1);
    assert!(out.starts_with(logbook::adif::JTDX_FILE_HEADER) && out.ends_with(&untouched), "{out}");
    let line = out.lines().nth(1).expect("riga del QSO");
    assert!(line.starts_with("<call:5>K1ABC <gridsquare:0> <mode:4>MFSK <rst_sent:3>+03 <rst_rcvd:3>-12 <qso_date:8>20261025 <time_on:6>101500 <qso_date_off:8>20261025 <time_off:6>101544 <band:3>40m <freq:8>7.047500 "), "{line}");
    assert!(line.contains(" <app_x:3>a<b ") && line.contains(" <submode:3>FT4 ") && line.contains(" <qth:6>Boston ") && !line.contains("comment") && line.ends_with(" <eor>"), "{line}");
    // riletto come lo leggono i «gia' lavorati», il file dice le stesse cose del log
    let read = aurora_core::adif::parse_qsos(&out);
    let row = book.all_qsos().expect("tutti").into_iter().find(|q| q.call == "K1ABC").expect("nel log");
    let from_file = read.iter().find(|q| q.call == "K1ABC").expect("nel file");
    assert_eq!((from_file.band.as_str(), from_file.mode.as_str(), from_file.date.as_str(), from_file.time_on.as_str(), from_file.freq_hz), (row.band.as_str(), row.mode.as_str(), row.date.as_str(), row.time_on.as_str(), row.freq_hz));
    assert_eq!(read.len(), 2);
    assert!(!read.iter().any(|q| q.call == "K1ABX"));
    // tornando da FT4 a FT8 il sottomodo sparisce; un QSO che nel file non c'e' non cambia nulla
    let back = book.update(id, &logbook::LogEdit { mode: "FT8".into(), ..logbook::LogEdit::from_entry(&change.new) }).expect("corretto").expect("c'e'");
    let (again, n) = logbook::adif_with_edit(&out, &back);
    assert!(n == 1 && again.lines().nth(1).is_some_and(|l| l.contains("<mode:3>FT8 ") && !l.contains("submode")), "{again}");
    assert_eq!(logbook::adif_with_edit(&text, &back), (text.clone(), 0));
}

#[test]
fn a_record_in_capitals_is_corrected_in_capitals() {
    let text = "Aurora\n<adif_ver:5>3.1.8 <EOH>\n<CALL:5>K1ABX <QSO_DATE:8>20261107 <TIME_ON:6>120015 <BAND:3>20m <MODE:3>FT8 <COMMENT:9><call:3>x <EOR>\n";
    let old = logbook::LogEntry { id: 1, date: "20261107".into(), time: "120015".into(), call: "K1ABX".into(), band: "20m".into(), mode: "FT8".into(), freq_hz: 0, rst_sent: String::new(), rst_rcvd: String::new(), grid: String::new(), name: String::new(), qth: String::new(), comment: String::new() };
    let new = logbook::LogEntry { call: "K1ABC".into(), name: "Joe".into(), ..old.clone() };
    let (out, n) = logbook::adif_with_edit(text, &logbook::LogChange { old, new, date_off: "20261107".into(), time_off: "120015".into() });
    assert_eq!(n, 1);
    assert_eq!(out, "Aurora\n<adif_ver:5>3.1.8 <EOH>\n<CALL:5>K1ABC <QSO_DATE:8>20261107 <TIME_ON:6>120015 <BAND:3>20m <MODE:3>FT8 <COMMENT:9><call:3>x <NAME:3>Joe <EOR>\n");
}

#[test]
fn archive_and_reset_keeps_the_old_log_and_starts_empty() {
    let dir = std::env::temp_dir().join(format!(
        "aurora-logbook-archive-{}-{}",
        std::process::id(),
        std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("logbook.db");
    let archive = dir.join("logbook-archivio-20261113-101500.db");
    let lb = Logbook::open(&path).unwrap();
    lb.add(&qso("JA1XYZ", "20m", "FT8")).unwrap();
    lb.add(&qso("IU3ABC", "40m", "FT4")).unwrap();
    assert_eq!(lb.path(), Some(path.as_path()));
    assert_eq!(lb.archive_and_reset(&archive).unwrap(), 2);
    // il log aperto e' vuoto e continua a funzionare
    assert_eq!(lb.count().unwrap(), 0);
    assert!(!lb.is_dupe("JA1XYZ", "20m", "FT8").unwrap());
    lb.add(&qso("K1ABC", "20m", "FT8")).unwrap();
    assert_eq!(lb.count().unwrap(), 1);
    // l'archivio ha tutti i QSO di prima e non si sovrascrive
    let old = Logbook::open(&archive).unwrap();
    assert_eq!(old.count().unwrap(), 2);
    assert!(old.is_dupe("JA1XYZ", "20m", "FT8").unwrap());
    assert!(old.is_dupe("IU3ABC", "40m", "FT4").unwrap());
    assert!(lb.archive_and_reset(&archive).is_err());
    assert_eq!(lb.count().unwrap(), 1);
    drop((lb, old));
    let _ = std::fs::remove_dir_all(&dir);
}
