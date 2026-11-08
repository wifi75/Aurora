//! La sequenza di contest (attivita' di WSJT-X 3.0.1, `genStdMsgs` e `processMessage` di `widgets/mainwindow.cpp`): messaggi in onda,
//! scambio ricevuto tradotto per la sequenza, QSO a log con gli scambi, seriale, e nessun cambiamento fuori dal contest.

use qso_native::*;

fn seq() -> Sequencer {
    Sequencer::new(Mode::Ft8, Station::new("IU3ABC", "IO91wm"), Options::default()).expect("stazione valida")
}

fn at(slot: u64) -> f64 {
    slot as f64 * 15.0 + 0.4
}

/// Chi risponde a un CQ TEST/WW parte dallo scambio (Tx2 = locatore); alla «R + locatore» del DX manda RR73 e il QSO va a log con lo scambio.
#[test]
fn ww_digi_rispondendo_a_un_cq() {
    let mut s = seq();
    s.set_contest(Some(ContestSeq::new(Activity::WwDigi)));
    s.reply_to(Slot(10), &Decoded::new("CQ WW K1ABC FN42", -10, 1200)).expect("risposta");
    let tx = s.next_tx(Slot(11), at(11)).expect("trasmette");
    assert_eq!((tx.n, tx.text.as_str()), (2, "K1ABC IU3ABC IO91"));
    s.on_decodes_at(Slot(12), &[Decoded::new("IU3ABC K1ABC R FN42", -9, 1200)], Some(at(12) + 13.0));
    let tx = s.next_tx(Slot(13), at(13)).expect("trasmette");
    assert_eq!(tx.text, "K1ABC IU3ABC RR73");
    let logged: Vec<QsoRecord> = s.drain_events().into_iter().filter_map(|e| if let Event::QsoComplete(r) = e { Some(r) } else { None }).collect();
    assert_eq!(logged.len(), 1);
    assert_eq!(logged[0].exch_rcvd, "FN42");
    assert_eq!(logged[0].exch_sent, "IO91");
    assert_eq!(logged[0].grid, "FN42");
}

/// Chi chiama CQ: alla risposta con il locatore manda subito «R + locatore» (Tx3), come WSJT-X in NA VHF/WW Digi/ARRL Digi.
#[test]
fn ww_digi_chiamando_cq() {
    let mut s = seq();
    s.set_contest(Some(ContestSeq::new(Activity::WwDigi)));
    s.call_cq(None).expect("CQ");
    let tx = s.next_tx(Slot(10), at(10)).expect("CQ in onda");
    assert_eq!(tx.text, "CQ WW IU3ABC IO91");
    s.on_decodes_at(Slot(11), &[Decoded::new("IU3ABC K1ABC FN42", -12, 1400)], Some(at(11) + 13.0));
    let tx = s.next_tx(Slot(12), at(12)).expect("risposta");
    assert_eq!((tx.n, tx.text.as_str()), (3, "K1ABC IU3ABC R IO91"));
}

/// Field Day con il seriale che non serve e Roundup con il seriale che avanza a ogni QSO a log.
#[test]
fn roundup_seriale_avanza() {
    let mut s = seq();
    let mut c = ContestSeq::new(Activity::Roundup);
    c.exchange = "#".into();
    c.serial = 7;
    s.set_contest(Some(c));
    s.reply_to(Slot(10), &Decoded::new("CQ RU K1ABC FN42", -10, 1200)).expect("risposta");
    let tx = s.next_tx(Slot(11), at(11)).expect("trasmette");
    assert_eq!(tx.text, "K1ABC IU3ABC 549 0007");
    s.on_decodes_at(Slot(12), &[Decoded::new("IU3ABC K1ABC R 579 WI", -9, 1200)], Some(at(12) + 13.0));
    s.next_tx(Slot(13), at(13)).expect("RR73");
    assert_eq!(s.contest().map(|c| c.serial), Some(8));
}

/// Fuori dal contest i messaggi restano quelli di sempre e «R + locatore» senza rapporto non va a log.
#[test]
fn fuori_dal_contest_nulla_cambia() {
    let mut s = seq();
    s.set_contest(Some(ContestSeq::new(Activity::WwDigi)));
    s.set_contest(None);
    s.reply_to(Slot(10), &Decoded::new("CQ K1ABC FN42", -10, 1200)).expect("risposta");
    let tx = s.next_tx(Slot(11), at(11)).expect("trasmette");
    assert_eq!(tx.text, "K1ABC IU3ABC IO91", "Tx1 normale");
    assert!(!s.options().skip_tx1, "«Salta Tx1» torna com'era");
}
