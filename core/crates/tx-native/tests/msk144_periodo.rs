//! MSK144: la durata del periodo e la finestra d'avvio seguono il periodo scelto (5, 10, 15, 30 s, `mainwindow.cpp:12081` di WSJT-X).
//! File a se': il periodo e' uno solo per programma.

use std::time::Duration;

use tx_native::{Mode, slot};

#[test]
fn msk144_period_changes_slot_length_and_window() {
    assert_eq!(Mode::Msk144.slot_len(), Duration::from_secs(15));
    assert_eq!(Mode::Msk144.tx_window(), Duration::from_secs(12));
    slot::set_msk144_period_s(30);
    assert_eq!(Mode::Msk144.slot_len(), Duration::from_secs(30));
    assert_eq!(Mode::Msk144.tx_window(), Duration::from_secs(24));
    assert_eq!(slot::slot_index(Duration::from_secs(59), Mode::Msk144), 1);
    slot::set_msk144_period_s(7);
    assert_eq!(slot::msk144_period_s(), 30, "7 s non e' fra i valori di WSJT-X");
    slot::set_msk144_period_s(5);
    assert_eq!(Mode::Msk144.slot_len(), Duration::from_secs(5));
    assert_eq!(Mode::Ft8.slot_len(), Duration::from_secs(15));
}
