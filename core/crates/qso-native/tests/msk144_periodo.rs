//! MSK144: il periodo T/R si sceglie fra 5, 10, 15 e 30 s come in WSJT-X (`ui->sbTR->values ({5, 10, 15, 30})`, `mainwindow.cpp:12081`).
//! File a se': il periodo e' uno solo per programma e le altre prove lo vogliono a 15 s.

use qso_native::{Mode, Options, Slot, slot};

#[test]
fn msk144_period_changes_slots_and_late_start() {
    assert_eq!(Mode::Msk144.period_ms(), 15_000, "di fabbrica 15 s, com'era");
    assert!(slot::set_msk144_period_s(30));
    assert_eq!(Mode::Msk144.period_ms(), 30_000);
    assert_eq!(Slot::at(Mode::Msk144, 29.9), Slot(0));
    assert_eq!(Slot::at(Mode::Msk144, 30.0), Slot(1));
    assert!(Slot::at(Mode::Msk144, 60.0).is_even(), ":00 pari");
    assert!(!Slot::at(Mode::Msk144, 30.0).is_even(), ":30 dispari");
    assert!((Options::default().late_limit_s(Mode::Msk144) - 24.0).abs() < 1e-9);
    assert!(slot::set_msk144_period_s(5));
    assert_eq!(Slot::at(Mode::Msk144, 12.0), Slot(2));
    assert!(!slot::set_msk144_period_s(20), "20 s non e' fra i valori di WSJT-X");
    assert_eq!(slot::msk144_period_s(), 5);
    // FT8 non cambia
    assert_eq!(Mode::Ft8.period_ms(), 15_000);
    assert!(slot::set_msk144_period_s(15));
    assert!((Options::default().late_limit_s(Mode::Msk144) - 12.0).abs() < 1e-9);
}
