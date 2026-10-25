//! I comandi della pagina che agiscono sulla sequenza: Salta Tx1 (anche a QSO avviato), periodo di trasmissione, Gen.Msgs, cambio modo.
//!
//! Riferimenti a JTDX 2.2.159 (`mainwindow.cpp`): `processMessage`, `on_skipTx1_clicked`, `on_TxMinuteButton_clicked`, `genStdMsgs`.

use qso_native::*;

fn seq(opts: Options) -> Sequencer {
    Sequencer::new(Mode::Ft8, Station::new("IU3ABC", "JN55rk"), opts).expect("stazione valida")
}

fn cq() -> Decoded {
    Decoded::new("CQ K1ABC FN42", -10, 1200)
}

#[test]
fn double_click_on_a_cq_starts_from_tx2_with_skip_tx1_and_from_tx1_without() {
    let mut with = seq(Options { skip_tx1: true, ..Options::default() });
    with.reply_to(Slot(10), &cq()).expect("risposta");
    assert_eq!(with.ntx(), 2);
    let tx = with.next_tx(Slot(11), 11.0 * 15.0 + 0.4).expect("trasmette");
    assert_eq!((tx.n, tx.text.as_str()), (2, "K1ABC IU3ABC -10"));

    let mut without = seq(Options::default());
    without.reply_to(Slot(10), &cq()).expect("risposta");
    let tx = without.next_tx(Slot(11), 11.0 * 15.0 + 0.4).expect("trasmette");
    assert_eq!((tx.n, tx.text.as_str()), (1, "K1ABC IU3ABC JN55"));
}

#[test]
fn a_message_to_someone_else_is_treated_like_a_cq() {
    // `processMessage`: se il proprio nominativo non e' nel messaggio si parte da Tx1, o da Tx2 con «Salta Tx1»
    let other = Decoded::new("K1ABC G3AAA -05", -12, 900);
    let mut with = seq(Options { skip_tx1: true, ..Options::default() });
    with.reply_to(Slot(10), &other).expect("risposta");
    assert_eq!(with.ntx(), 2);
    let mut without = seq(Options::default());
    without.reply_to(Slot(10), &other).expect("risposta");
    assert_eq!(without.ntx(), 1);
}

#[test]
fn a_call_to_us_starts_from_tx2_whatever_skip_tx1_says() {
    let called = Decoded::new("IU3ABC K1ABC FN42", -8, 1200);
    for skip in [false, true] {
        let mut s = seq(Options { skip_tx1: skip, ..Options::default() });
        s.reply_to(Slot(10), &called).expect("risposta");
        assert_eq!(s.ntx(), 2, "skip {skip}");
    }
}

#[test]
fn toggling_skip_tx1_moves_the_starting_message_before_the_dx_has_answered() {
    // `on_skipTx1_clicked`: acceso porta da Tx1 a Tx2, spento da Tx2 a Tx1
    let mut s = seq(Options::default());
    s.reply_to(Slot(10), &cq()).expect("risposta");
    assert_eq!(s.ntx(), 1);
    s.set_options(Options { skip_tx1: true, ..s.options().clone() });
    assert_eq!(s.ntx(), 2);
    s.set_options(Options { skip_tx1: false, ..s.options().clone() });
    assert_eq!(s.ntx(), 1);
}

#[test]
fn toggling_skip_tx1_does_not_undo_a_real_advance_of_the_qso() {
    let mut s = seq(Options::default());
    s.reply_to(Slot(10), &cq()).expect("risposta");
    s.next_tx(Slot(11), 11.0 * 15.0 + 0.4).expect("Tx1");
    // il DX risponde col suo rapporto: si passa a Tx3 e un clic su «Salta Tx1» non deve riportare indietro
    s.on_decodes(Slot(11), &[Decoded::new("IU3ABC K1ABC -07", -9, 1200)]);
    assert_eq!(s.ntx(), 3);
    s.set_options(Options { skip_tx1: true, ..s.options().clone() });
    assert_eq!(s.ntx(), 3);
}

#[test]
fn tx_period_change_applies_and_clears_the_dx_with_autoseq_and_enable_tx() {
    let mut s = seq(Options::default());
    // risposta a un CQ del periodo dispari 11: si trasmette nei pari, e i messaggi sono generati per i pari
    s.reply_to(Slot(11), &cq()).expect("risposta");
    assert!(s.tx_even());
    assert!(s.dx().is_some());
    // stesso periodo: niente cambia
    s.set_tx_even(true);
    assert!(s.dx().is_some());
    // periodo diverso da quello dei messaggi generati, con Abilita Tx e AutoSeq accesi: il DX si libera e il periodo cambia davvero
    s.set_tx_even(false);
    assert!(!s.tx_even());
    assert_eq!(s.dx(), None);
    // senza DX il nuovo periodo e' quello in cui si trasmette il CQ
    s.call_cq(None).expect("CQ");
    assert!(s.next_tx(Slot(12), 12.0 * 15.0 + 0.4).is_none(), "nei pari non si trasmette piu'");
    assert!(s.next_tx(Slot(13), 13.0 * 15.0 + 0.4).is_some());
}

#[test]
fn tx_period_change_with_autoseq_off_keeps_the_dx() {
    let mut s = seq(Options { auto_seq: false, ..Options::default() });
    s.reply_to(Slot(11), &cq()).expect("risposta");
    s.set_tx_even(false);
    assert!(s.dx().is_some());
    assert!(!s.tx_even());
}

#[test]
fn gen_msgs_puts_the_starting_message_back_and_does_nothing_without_a_dx() {
    let mut s = seq(Options::default());
    s.regen_messages();
    assert_eq!(s.ntx(), 0, "senza DX niente da rigenerare");
    s.reply_to(Slot(10), &cq()).expect("risposta");
    s.select_tx(4).expect("Tx4");
    assert_eq!(s.ntx(), 4);
    s.regen_messages();
    assert_eq!(s.ntx(), 1);
    s.set_options(Options { skip_tx1: true, ..s.options().clone() });
    s.select_tx(4).expect("Tx4");
    s.regen_messages();
    assert_eq!(s.ntx(), 2, "con «Salta Tx1» si riparte da Tx2");
    assert_eq!(s.tx_texts()[1], "K1ABC IU3ABC -10");
}

#[test]
fn changing_mode_clears_the_qso_and_stops_the_transmission() {
    let mut s = seq(Options::default());
    s.reply_to(Slot(10), &cq()).expect("risposta");
    assert!(s.is_enabled());
    s.set_mode(Mode::Ft4);
    assert_eq!(s.mode(), Mode::Ft4);
    assert!(!s.is_enabled());
    assert_eq!(s.dx(), None);
    assert!(s.next_tx(Slot(100), 750.4).is_none());
    // stesso modo: non fa niente
    s.call_cq(None).expect("CQ");
    s.set_mode(Mode::Ft4);
    assert!(s.is_enabled());
    // in FT4 i periodi sono di 7,5 s: il periodo 101 (dispari, come il pulsante dopo la risposta al CQ) comincia a 757,5 s
    let tx = s.next_tx(Slot(101), 757.9).expect("CQ in FT4");
    assert_eq!(tx.text, "CQ IU3ABC JN55");
}

#[test]
fn a_double_click_with_auto_tx_off_picks_the_dx_but_does_not_enable_tx() {
    // `doubleClickOnCall`: «Abilita Tx» si accende solo se «AutoTX» e' acceso
    let mut s = seq(Options::default());
    s.reply_to_with(Slot(10), &cq(), false).expect("risposta");
    assert_eq!((s.dx(), s.ntx(), s.is_enabled()), (Some("K1ABC"), 1, false));
    assert!(s.next_tx(Slot(11), 11.0 * 15.0 + 0.4).is_none(), "spento, non si trasmette");
    s.set_enabled(true);
    let tx = s.next_tx(Slot(11), 11.0 * 15.0 + 0.4).expect("acceso, parte il messaggio di partenza");
    assert_eq!((tx.n, tx.text.as_str()), (1, "K1ABC IU3ABC JN55"));
    // con AutoTX acceso (`reply_to`) si accende da solo
    let mut on = seq(Options::default());
    on.reply_to(Slot(10), &cq()).expect("risposta");
    assert!(on.is_enabled());
}
