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

/// In contest la trasmissione resta sempre accesa: finito un QSO si libera il DX e si torna subito al CQ, senza Stopped, e se nessuno risponde
/// il CQ continua (richiesta del proprietario, 2026-11-10).
#[test]
fn in_contest_dopo_il_qso_si_torna_al_cq_con_la_trasmissione_accesa() {
    let mut s = seq();
    s.set_contest(Some(ContestSeq::new(Activity::WwDigi)));
    s.call_cq(None).expect("CQ");
    assert_eq!(s.next_tx(Slot(10), at(10)).expect("CQ in onda").text, "CQ WW IU3ABC IO91");
    s.on_decodes_at(Slot(11), &[Decoded::new("IU3ABC K1ABC FN42", -12, 1400)], Some(at(11) + 13.0));
    assert_eq!(s.next_tx(Slot(12), at(12)).expect("risposta").text, "K1ABC IU3ABC R IO91");
    // il DX chiude con RR73: da qui in poi il QSO e' finito
    s.on_decodes_at(Slot(13), &[Decoded::new("IU3ABC K1ABC RR73", -12, 1400)], Some(at(13) + 13.0));
    let mut events = s.drain_events();
    let mut cq_again = None;
    for slot in [14u64, 16, 18, 20] {
        let t = s.next_tx(Slot(slot), at(slot));
        // il periodo dopo arrivano le decodifiche (anche vuote) e la sequenza decide
        s.on_decodes_at(Slot(slot + 1), &[], Some(at(slot + 1) + 13.0));
        if let Some(tx) = t {
            if tx.text.starts_with("CQ ") && cq_again.is_none() {
                cq_again = Some(slot);
            }
        }
        events.extend(s.drain_events());
    }
    assert!(events.iter().any(|e| matches!(e, Event::QsoComplete(_))), "il QSO e' a log: {events:?}");
    assert!(!events.iter().any(|e| matches!(e, Event::Stopped(_))), "niente arresto in contest: {events:?}");
    assert!(s.is_enabled(), "Abilita Tx resta acceso");
    assert!(cq_again.is_some(), "dopo il QSO si torna a chiamare CQ");
    // e il CQ continua finche' nessuno risponde
    for slot in [22u64, 24, 26] {
        let tx = s.next_tx(Slot(slot), at(slot)).unwrap_or_else(|| panic!("CQ al periodo {slot}"));
        assert_eq!(tx.text, "CQ WW IU3ABC IO91", "periodo {slot}");
        s.on_decodes_at(Slot(slot + 1), &[], Some(at(slot + 1) + 13.0));
    }
    assert!(s.is_enabled());
}

/// In contest il watchdog non ferma mai il CQ, anche dopo ore di chiamate senza risposta.
#[test]
fn in_contest_il_watchdog_non_ferma_il_cq() {
    let mut s = seq();
    s.set_contest(Some(ContestSeq::new(Activity::WwDigi)));
    s.call_cq(None).expect("CQ");
    for slot in (10..120u64).step_by(2) {
        assert!(s.next_tx(Slot(slot), at(slot)).is_some(), "periodo {slot}");
    }
    assert!(s.is_enabled());
    assert!(!s.drain_events().iter().any(|e| matches!(e, Event::Stopped(_))));
}

/// L'operatore invece puo' sempre fermare, anche in contest.
#[test]
fn in_contest_l_operatore_ferma_ancora() {
    let mut s = seq();
    s.set_contest(Some(ContestSeq::new(Activity::WwDigi)));
    s.call_cq(None).expect("CQ");
    assert!(s.next_tx(Slot(10), at(10)).is_some());
    s.halt();
    assert!(!s.is_enabled());
    assert!(s.drain_events().iter().any(|e| matches!(e, Event::Stopped(StopReason::Halt))));
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

/// Il seriale lo decide il log del contest: se un QSO non entra nel log del contest (fuori orario) la pagina rimanda il seriale giusto,
/// anche piu' basso di quello che la sequenza aveva gia' fatto avanzare.
#[test]
fn il_seriale_si_riallinea_al_log_del_contest() {
    let mut s = seq();
    let mut c = ContestSeq::new(Activity::Roundup);
    c.serial = 8;
    s.set_contest(Some(c.clone()));
    c.serial = 7;
    s.set_contest(Some(c));
    assert_eq!(s.contest().map(|c| c.serial), Some(7));
}

/// In contest «gia' lavorato» vale per banda: lo stesso nominativo su un'altra banda e' un QSO nuovo.
#[test]
fn in_contest_gia_lavorato_vale_per_banda() {
    let mut s = seq();
    s.set_contest(Some(ContestSeq::new(Activity::WwDigi)));
    s.set_worked_scope("20m");
    s.mark_worked("K1ABC");
    assert!(s.is_worked("K1ABC"));
    s.set_worked_scope("40m");
    assert!(!s.is_worked("K1ABC"), "su 40 m il QSO conta");
    s.set_worked_scope("20M");
    assert!(s.is_worked("K1ABC/P"), "su 20 m e' doppio");
}
