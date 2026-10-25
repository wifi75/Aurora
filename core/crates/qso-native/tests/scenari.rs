//! Scenari completi della sequenza dei QSO, periodo per periodo.
//!
//! Ogni prova dichiara a quale regola di JTDX si riferisce:
//! - `processMessage`: doppio clic su una riga (`mainwindow.cpp`, scelta del Tx di partenza);
//! - `process_Auto`: avanzamento con AutoSeq a fine decodifica (`mainwindow.cpp`, `switch (m_status)`);
//! - `QsoHistory`: lo stato per stazione (`qsohistory.cpp`, `message()` e `autoseq()`);
//! - `genStdMsgs`/`genCQMsg`: i testi;
//! - `txwatchdog`, `autoStopTx`, `readFromStdout`: watchdog, arresto, DX che risponde a un altro;
//! - «docs»: `docs/logiche-jtdx.md`, sequenza osservata in ALL.TXT.
//!
//! Convenzione: il nostro nominativo e' IU3ABC (JN55). In FT8 il periodo `n` comincia a `15*n` s;
//! la trasmissione si chiede a 0,4 s dall'inizio. Con `tx_even` vero si trasmette nei periodi pari.

use qso_native::*;

const ME: &str = "IU3ABC";

struct Bench {
    seq: Sequencer,
    mode: Mode,
    events: Vec<Event>,
}

fn opts() -> Options {
    Options::default()
}

impl Bench {
    fn new(mode: Mode, opts: Options) -> Self {
        let seq = Sequencer::new(mode, Station::new(ME, "JN55rk"), opts).expect("stazione valida");
        Bench {
            seq,
            mode,
            events: Vec::new(),
        }
    }

    fn ft8(opts: Options) -> Self {
        Bench::new(Mode::Ft8, opts)
    }

    fn collect(&mut self) {
        let mut ev = self.seq.drain_events();
        self.events.append(&mut ev);
    }

    /// Messaggi decodificati a fine del periodo `slot`.
    fn rx(&mut self, slot: u64, msgs: &[(&str, i32)]) {
        let decodes: Vec<Decoded> = msgs
            .iter()
            .map(|(t, s)| Decoded::new(t, *s, 1200))
            .collect();
        self.seq.on_decodes(Slot(slot), &decodes);
        self.collect();
    }

    /// Cosa si trasmette nel periodo `slot` (richiesta a 0,4 s dall'inizio).
    fn tx(&mut self, slot: u64) -> Option<String> {
        self.tx_at(slot, 0.4).map(|t| t.text)
    }

    fn tx_at(&mut self, slot: u64, delay_s: f64) -> Option<Tx> {
        let now = Slot(slot).start_s(self.mode) + delay_s;
        let tx = self.seq.next_tx(Slot(slot), now);
        self.collect();
        tx
    }

    fn reply(&mut self, slot: u64, text: &str, snr: i32) {
        self.seq
            .reply_to(Slot(slot), &Decoded::new(text, snr, 1200))
            .expect("risposta possibile");
        self.collect();
    }

    fn records(&self) -> Vec<&QsoRecord> {
        self.events
            .iter()
            .filter_map(|e| {
                if let Event::QsoComplete(r) = e {
                    Some(r)
                } else {
                    None
                }
            })
            .collect()
    }

    fn has(&self, ev: &Event) -> bool {
        self.events.contains(ev)
    }

    fn stops(&self) -> Vec<StopReason> {
        self.events
            .iter()
            .filter_map(|e| {
                if let Event::Stopped(r) = e {
                    Some(*r)
                } else {
                    None
                }
            })
            .collect()
    }

    fn abandons(&self) -> Vec<AbandonReason> {
        self.events
            .iter()
            .filter_map(|e| {
                if let Event::Abandoned { reason, .. } = e {
                    Some(*reason)
                } else {
                    None
                }
            })
            .collect()
    }
}

// ---------------------------------------------------------------------------------------
// QSO completi
// ---------------------------------------------------------------------------------------

/// `process_Auto`: RCALL -> Tx2, RRREPORT -> Tx4, FIN; `genCQMsg` per il Tx6. QSO da chiamante di CQ.
#[test]
fn full_qso_calling_cq() {
    let mut b = Bench::ft8(opts());
    b.seq.call_cq(None).expect("cq");
    assert_eq!(b.tx(0).as_deref(), Some("CQ IU3ABC JN55"));
    assert_eq!(b.tx(1), None, "periodo dispari: si ascolta");
    b.rx(1, &[("IU3ABC LZ2KSQ KN23", -12)]);
    assert_eq!(b.seq.dx(), Some("LZ2KSQ"));
    assert_eq!(b.tx(2).as_deref(), Some("LZ2KSQ IU3ABC -12"));
    b.rx(3, &[("IU3ABC LZ2KSQ R-10", -11)]);
    assert_eq!(b.tx(4).as_deref(), Some("LZ2KSQ IU3ABC RR73"));
    b.rx(5, &[("IU3ABC LZ2KSQ 73", -11)]);
    assert_eq!(b.seq.dx(), None);
    assert_eq!(
        b.tx(6).as_deref(),
        Some("CQ IU3ABC JN55"),
        "dopo il QSO si torna al CQ"
    );

    let recs = b.records();
    assert_eq!(recs.len(), 1);
    let r = recs[0];
    assert_eq!(r.call, "LZ2KSQ");
    assert_eq!(r.grid, "KN23");
    assert_eq!(r.rst_sent, "-12");
    assert_eq!(r.rst_rcvd, "-10");
    assert_eq!(r.mode, Mode::Ft8);
    assert_eq!(
        r.time_on_ms, 15_000,
        "il primo periodo e' quello della risposta al CQ"
    );
    assert_eq!(
        r.time_off_ms, 60_000,
        "concluso quando e' partito l'RR73 (periodo 4)"
    );
    assert_eq!(r.rx_freq_hz, 1200);
    assert!(b.stops().is_empty());
}

/// docs (sequenza osservata in ALL.TXT, «Salta Tx1»): si risponde al CQ con Tx2, senza risposta si
/// ripete, poi R-rapporto -> Tx4 (RR73), poi il 73 chiude. `process_Auto` RCQ con `m_skipTx1`.
#[test]
fn full_qso_answering_cq_skip_tx1_as_in_all_txt() {
    let mut b = Bench::ft8(Options {
        skip_tx1: true,
        ..opts()
    });
    b.reply(1, "CQ LZ2KSQ KN23", -12);
    assert_eq!(b.seq.ntx(), 2);
    assert_eq!(b.tx(2).as_deref(), Some("LZ2KSQ IU3ABC -12"));
    assert_eq!(b.tx(3), None);
    b.rx(3, &[("CQ LZ2KSQ KN23", -12)]);
    assert_eq!(
        b.tx(4).as_deref(),
        Some("LZ2KSQ IU3ABC -12"),
        "nessuna risposta: JTDX ripete lo stesso messaggio"
    );
    b.rx(5, &[("IU3ABC LZ2KSQ R-10", -10)]);
    assert_eq!(b.seq.ntx(), 4);
    assert_eq!(b.tx(6).as_deref(), Some("LZ2KSQ IU3ABC RR73"));
    assert_eq!(b.records().len(), 1, "registrato quando parte l'RR73");
    b.rx(7, &[("IU3ABC LZ2KSQ 73", -10)]);
    assert_eq!(b.tx(8), None);
    let r = b.records()[0].clone();
    assert_eq!((r.rst_sent.as_str(), r.rst_rcvd.as_str()), ("-12", "-10"));
    assert_eq!(r.grid, "KN23");
    assert_eq!(b.stops(), vec![StopReason::NothingToDo]);
}

/// `process_Auto` RCQ -> Tx1 (senza «Salta Tx1»); RREPORT -> Tx3; RRR73 -> Tx5; `qsohistory.cpp`
/// `case RRR73`. Il QSO si registra alla ricezione dell'RR73 e il 73 parte una volta sola.
#[test]
fn full_qso_answering_cq_with_tx1() {
    let mut b = Bench::ft8(opts());
    b.reply(1, "CQ LZ2KSQ KN23", -12);
    assert_eq!(b.seq.ntx(), 1);
    assert_eq!(b.tx(2).as_deref(), Some("LZ2KSQ IU3ABC JN55"));
    b.rx(3, &[("IU3ABC LZ2KSQ -07", -9)]);
    assert_eq!(
        b.tx(4).as_deref(),
        Some("LZ2KSQ IU3ABC R-09"),
        "il rapporto e' l'SNR dell'ultimo messaggio del DX"
    );
    b.rx(5, &[("IU3ABC LZ2KSQ RR73", -9)]);
    assert_eq!(b.records().len(), 1);
    assert_eq!(b.tx(6).as_deref(), Some("LZ2KSQ IU3ABC 73"));
    assert_eq!(b.records().len(), 1, "nessuna seconda registrazione");
    assert_eq!(b.tx(8), None, "il 73 si manda una volta, poi si smette");
    let r = b.records()[0].clone();
    assert_eq!((r.rst_sent.as_str(), r.rst_rcvd.as_str()), ("-09", "-07"));
    assert_eq!(r.time_on_ms, 15_000);
    assert_eq!(
        r.time_off_ms, 75_000,
        "concluso alla ricezione dell'RR73 (periodo 5)"
    );
}

/// `genStdMsgs` + `process_Auto`: lo stesso QSO in FT4, con periodi di 7,5 s.
#[test]
fn full_qso_in_ft4() {
    let mut b = Bench::new(Mode::Ft4, opts());
    b.seq.call_cq(Some("DX")).expect("cq");
    assert_eq!(b.tx(0).as_deref(), Some("CQ DX IU3ABC JN55"));
    b.rx(1, &[("IU3ABC LZ2KSQ KN23", -5)]);
    assert_eq!(b.tx(1), None);
    let tx = b.tx_at(2, 0.3).expect("tx2");
    assert_eq!(tx.text, "LZ2KSQ IU3ABC -05");
    assert!(tx.even_slot);
    b.rx(3, &[("IU3ABC LZ2KSQ R-02", -5)]);
    assert_eq!(b.tx(4).as_deref(), Some("LZ2KSQ IU3ABC RR73"));
    let r = b.records()[0].clone();
    assert_eq!(r.mode, Mode::Ft4);
    assert_eq!(r.time_on_ms, 7_500);
    assert_eq!(r.time_off_ms, 30_000);
    // oltre il limite di avvio dell'FT4 (1 s) non si parte
    b.rx(5, &[("IU3ABC LZ2KSQ 73", -5)]);
    assert_eq!(b.tx_at(6, 1.3), None);
    assert_eq!(b.tx_at(6, 0.9).map(|t| t.n), Some(6));
}

// ---------------------------------------------------------------------------------------
// Il DX non risponde, risponde a un altro, cambia stato
// ---------------------------------------------------------------------------------------

/// `AnswerCQ` (contatore, massimo 5): si manda 5 volte Tx1, la sesta non parte; `process_Auto`
/// pulisce il DX. Senza una serie di CQ in corso ci si ferma (Aurora non chiama CQ da sola).
#[test]
fn dx_does_not_answer_repeats_then_gives_up() {
    let mut b = Bench::ft8(opts());
    b.reply(1, "CQ LZ2KSQ KN23", -12);
    for slot in [2, 4, 6, 8, 10] {
        assert_eq!(
            b.tx(slot).as_deref(),
            Some("LZ2KSQ IU3ABC JN55"),
            "periodo {slot}"
        );
    }
    assert_eq!(b.tx(12), None);
    assert_eq!(b.abandons(), vec![AbandonReason::NoReply]);
    assert_eq!(b.stops(), vec![StopReason::NothingToDo]);
    assert!(!b.seq.is_enabled());
    assert_eq!(b.seq.dx(), None);
    assert!(b.records().is_empty());
}

/// `AnswerInCall`: durante una serie di CQ, un chiamante che non risponde al Tx2 viene lasciato e
/// si torna al CQ nello stesso periodo (`process_Auto`: DX vuoto e contatori -> Tx6).
#[test]
fn caller_vanishes_and_cq_resumes() {
    let mut b = Bench::ft8(opts());
    b.seq.call_cq(None).expect("cq");
    assert_eq!(b.tx(0).as_deref(), Some("CQ IU3ABC JN55"));
    b.rx(1, &[("IU3ABC LZ2KSQ KN23", -12)]);
    for slot in [2, 4, 6, 8, 10] {
        assert_eq!(b.tx(slot).as_deref(), Some("LZ2KSQ IU3ABC -12"));
    }
    assert_eq!(b.tx(12).as_deref(), Some("CQ IU3ABC JN55"));
    assert_eq!(b.abandons(), vec![AbandonReason::NoReply]);
    assert!(b.stops().is_empty());
    assert_eq!(b.seq.phase(), Phase::Cq);
}

/// `SentRReport` (massimo 5): R+rapporto ripetuto senza chiusura.
#[test]
fn r_report_repeats_are_capped() {
    let mut b = Bench::ft8(opts());
    b.reply(1, "CQ LZ2KSQ KN23", -12);
    b.tx(2);
    b.rx(3, &[("IU3ABC LZ2KSQ -07", -9)]);
    for slot in [4, 6, 8, 10, 12] {
        assert_eq!(b.tx(slot).as_deref(), Some("LZ2KSQ IU3ABC R-09"));
    }
    assert_eq!(b.tx(14), None);
    assert_eq!(b.abandons(), vec![AbandonReason::NoReply]);
}

/// `readFromStdout` («reply other», `SeqHaltTxReplyOther`) e `process_Auto` con `m_reply_other`:
/// il DX, non ancora in contatto con noi, risponde a un altro: si lascia subito.
#[test]
fn dx_answers_someone_else() {
    let mut b = Bench::ft8(opts());
    b.reply(1, "CQ LZ2KSQ KN23", -12);
    assert!(b.tx(2).is_some());
    b.rx(3, &[("AA1AA LZ2KSQ -10", -12)]);
    assert_eq!(b.abandons(), vec![AbandonReason::DxAnsweredOther]);
    assert_eq!(b.seq.dx(), None);
    assert_eq!(b.tx(4), None);
}

/// Con l'opzione spenta (`SeqHaltTxReplyOther` false) si insiste fino ai tetti.
#[test]
fn dx_answers_someone_else_option_off_keeps_trying() {
    let mut b = Bench::ft8(Options {
        abandon_if_dx_answers_other: false,
        ..opts()
    });
    b.reply(1, "CQ LZ2KSQ KN23", -12);
    b.tx(2);
    b.rx(3, &[("AA1AA LZ2KSQ -10", -12)]);
    assert_eq!(b.seq.dx(), Some("LZ2KSQ"));
    assert!(b.tx(4).is_some());
}

/// Dopo che il DX ci ha scritto, un suo messaggio a un altro non fa rinunciare (contano i tetti):
/// `m_reply_other` riguarda solo la fase RCQ/SCALL/SREPORT.
#[test]
fn dx_talking_to_others_after_contact_is_ignored() {
    let mut b = Bench::ft8(opts());
    b.reply(1, "CQ LZ2KSQ KN23", -12);
    b.tx(2);
    b.rx(3, &[("IU3ABC LZ2KSQ -07", -9)]);
    b.tx(4);
    b.rx(5, &[("AA1AA LZ2KSQ -10", -9)]);
    assert_eq!(b.seq.dx(), Some("LZ2KSQ"));
    assert!(b.abandons().is_empty());
}

/// `qsohistory.cpp` `message()`: lo stato e' l'ultimo messaggio ricevuto; il DX che ripete il
/// locatore mentre siamo al Tx2 non ci sposta, e che risponde col rapporto a un Tx1 ci porta al Tx3.
#[test]
fn partner_changes_state_follows_last_message() {
    let mut b = Bench::ft8(opts());
    b.seq.call_cq(None).expect("cq");
    b.tx(0);
    b.rx(1, &[("IU3ABC LZ2KSQ KN23", -12)]);
    assert_eq!(b.seq.ntx(), 2);
    b.tx(2);
    b.rx(3, &[("IU3ABC LZ2KSQ KN23", -12)]);
    assert_eq!(b.seq.ntx(), 2, "ripete il locatore: restiamo al Tx2");
    b.rx(5, &[("IU3ABC LZ2KSQ R-10", -12)]);
    assert_eq!(b.seq.ntx(), 4);
    b.rx(7, &[("IU3ABC LZ2KSQ -10", -12)]);
    assert_eq!(
        b.seq.ntx(),
        3,
        "un rapporto semplice vuole il nostro R-rapporto"
    );
}

/// `qsohistory.cpp` `RRREPORT` richiede di aver mandato un rapporto (`t.stx_c >= SREPORT`): senza,
/// l'R-rapporto del DX non porta alla chiusura, si manda prima il nostro rapporto (Tx3).
#[test]
fn r_report_before_our_report_goes_to_tx3() {
    let mut b = Bench::ft8(opts());
    b.reply(1, "CQ LZ2KSQ KN23", -12);
    assert_eq!(b.tx(2).as_deref(), Some("LZ2KSQ IU3ABC JN55"));
    b.rx(3, &[("IU3ABC LZ2KSQ R-10", -8)]);
    assert_eq!(b.seq.ntx(), 3);
    assert_eq!(b.tx(4).as_deref(), Some("LZ2KSQ IU3ABC R-08"));
}

/// `qsohistory.cpp` `case RRR/RRR73/R73`: la chiusura vale solo se i due rapporti sono stati scambiati.
#[test]
fn closing_without_reports_is_ignored() {
    let mut b = Bench::ft8(Options {
        skip_tx1: true,
        ..opts()
    });
    b.reply(1, "CQ LZ2KSQ KN23", -12);
    b.tx(2);
    b.rx(3, &[("IU3ABC LZ2KSQ RR73", -12)]);
    assert_eq!(b.seq.ntx(), 2);
    assert!(b.records().is_empty());
    assert_eq!(b.tx(4).as_deref(), Some("LZ2KSQ IU3ABC -12"));
}

// ---------------------------------------------------------------------------------------
// RRR, 73 duplicati, chiusure
// ---------------------------------------------------------------------------------------

/// `process_Auto` `case RRR` -> Tx5: il DX chiude con RRR invece di RR73.
#[test]
fn partner_sends_rrr_instead_of_rr73() {
    let mut b = Bench::ft8(opts());
    b.reply(1, "CQ LZ2KSQ KN23", -12);
    b.tx(2);
    b.rx(3, &[("IU3ABC LZ2KSQ -07", -9)]);
    b.tx(4);
    b.rx(5, &[("IU3ABC LZ2KSQ RRR", -9)]);
    assert_eq!(b.records().len(), 1);
    assert_eq!(b.seq.ntx(), 5);
    assert_eq!(b.tx(6).as_deref(), Some("LZ2KSQ IU3ABC 73"));
    assert_eq!(b.records().len(), 1);
}

/// Casella RRR (`rrr1CheckBox`): il Tx4 e' RRR, non si registra alla partenza (non e' un 73); il
/// 73 del DX chiude e il nostro 73 parte una volta (`case R73` -> Tx5).
#[test]
fn our_rrr_closes_when_partner_sends_73() {
    let mut b = Bench::ft8(Options {
        use_rrr: true,
        ..opts()
    });
    b.seq.call_cq(None).expect("cq");
    b.tx(0);
    b.rx(1, &[("IU3ABC LZ2KSQ KN23", -12)]);
    b.tx(2);
    b.rx(3, &[("IU3ABC LZ2KSQ R-10", -12)]);
    assert_eq!(b.tx(4).as_deref(), Some("LZ2KSQ IU3ABC RRR"));
    assert!(b.records().is_empty(), "RRR non e' ancora la chiusura");
    b.rx(5, &[("IU3ABC LZ2KSQ 73", -12)]);
    assert_eq!(b.records().len(), 1);
    assert_eq!(b.tx(6).as_deref(), Some("LZ2KSQ IU3ABC 73"));
    assert_eq!(b.tx(8).as_deref(), Some("CQ IU3ABC JN55"));
    assert_eq!(b.records().len(), 1);
}

/// `qsohistory.cpp` `case R73`: dopo un nostro RR73 il 73 del DX porta a FIN, un 73 duplicato (e
/// un RR73 ripetuto) non apre un secondo QSO ne' una seconda registrazione.
#[test]
fn duplicate_73_is_ignored() {
    let mut b = Bench::ft8(opts());
    b.seq.call_cq(None).expect("cq");
    b.tx(0);
    b.rx(1, &[("IU3ABC LZ2KSQ KN23", -12)]);
    b.tx(2);
    b.rx(3, &[("IU3ABC LZ2KSQ R-10", -12)]);
    b.tx(4);
    b.rx(5, &[("IU3ABC LZ2KSQ 73", -12)]);
    b.rx(7, &[("IU3ABC LZ2KSQ 73", -12), ("IU3ABC LZ2KSQ RR73", -12)]);
    assert_eq!(b.records().len(), 1);
    assert_eq!(b.seq.dx(), None);
    assert!(b.seq.queued().is_empty());
    assert_eq!(b.tx(6).as_deref(), Some("CQ IU3ABC JN55"));
}

/// Due RR73 nello stesso periodo (duplicato dal decoder) non registrano due volte.
#[test]
fn duplicate_rr73_in_one_slot_logs_once() {
    let mut b = Bench::ft8(opts());
    b.reply(1, "CQ LZ2KSQ KN23", -12);
    b.tx(2);
    b.rx(3, &[("IU3ABC LZ2KSQ -07", -9)]);
    b.tx(4);
    b.rx(
        5,
        &[("IU3ABC LZ2KSQ RR73", -9), ("IU3ABC LZ2KSQ RR73", -10)],
    );
    assert_eq!(b.records().len(), 1);
}

/// `SentRR7373` (qui 3): l'RR73 si ripete se il 73 non arriva, poi il QSO (gia' registrato alla
/// prima partenza) si chiude senza segnalare una rinuncia e si torna al CQ.
#[test]
fn rr73_repeats_without_73_then_closes() {
    let mut b = Bench::ft8(opts());
    b.seq.call_cq(None).expect("cq");
    b.tx(0);
    b.rx(1, &[("IU3ABC LZ2KSQ KN23", -12)]);
    b.tx(2);
    b.rx(3, &[("IU3ABC LZ2KSQ R-10", -12)]);
    for slot in [4, 6, 8] {
        assert_eq!(
            b.tx(slot).as_deref(),
            Some("LZ2KSQ IU3ABC RR73"),
            "periodo {slot}"
        );
    }
    assert_eq!(b.tx(10).as_deref(), Some("CQ IU3ABC JN55"));
    assert_eq!(b.records().len(), 1);
    assert!(b.abandons().is_empty());
}

/// Se il DX ripete l'R-rapporto (non ha sentito l'RR73) si rimanda l'RR73 e il contatore non riparte.
#[test]
fn partner_repeats_r_report_after_rr73() {
    let mut b = Bench::ft8(opts());
    b.seq.call_cq(None).expect("cq");
    b.tx(0);
    b.rx(1, &[("IU3ABC LZ2KSQ KN23", -12)]);
    b.tx(2);
    b.rx(3, &[("IU3ABC LZ2KSQ R-10", -12)]);
    b.tx(4);
    b.rx(5, &[("IU3ABC LZ2KSQ R-10", -12)]);
    assert_eq!(b.tx(6).as_deref(), Some("LZ2KSQ IU3ABC RR73"));
    assert_eq!(b.records().len(), 1);
}

// ---------------------------------------------------------------------------------------
// Due chiamanti, coda
// ---------------------------------------------------------------------------------------

/// `autoseq()` (priorita' pari: vince il rapporto migliore) e coda (Decodium: `m_callerQueue`):
/// due chiamanti insieme, si serve il piu' forte, l'altro aspetta e viene servito dopo.
#[test]
fn two_callers_one_served_other_queued() {
    let mut b = Bench::ft8(opts());
    b.seq.call_cq(None).expect("cq");
    b.tx(0);
    b.rx(1, &[("IU3ABC AA1AA FN42", -15), ("IU3ABC BB2BB KN23", -5)]);
    assert_eq!(b.seq.dx(), Some("BB2BB"));
    assert_eq!(b.seq.queued(), vec!["AA1AA".to_string()]);
    assert!(b.has(&Event::CallerQueued {
        call: "AA1AA".into()
    }));
    assert_eq!(b.tx(2).as_deref(), Some("BB2BB IU3ABC -05"));
    b.rx(3, &[("IU3ABC BB2BB R-10", -5), ("IU3ABC AA1AA FN42", -16)]);
    assert_eq!(b.tx(4).as_deref(), Some("BB2BB IU3ABC RR73"));
    b.rx(5, &[("IU3ABC BB2BB 73", -5), ("IU3ABC AA1AA FN42", -14)]);
    assert_eq!(b.seq.dx(), Some("AA1AA"));
    assert!(b.seq.queued().is_empty());
    assert_eq!(
        b.tx(6).as_deref(),
        Some("AA1AA IU3ABC -14"),
        "rapporto dell'ultimo messaggio sentito"
    );
    b.rx(7, &[("IU3ABC AA1AA R-12", -14)]);
    assert_eq!(b.tx(8).as_deref(), Some("AA1AA IU3ABC RR73"));
    let calls: Vec<&str> = b.records().iter().map(|r| r.call.as_str()).collect();
    assert_eq!(calls, vec!["BB2BB", "AA1AA"]);
}

/// Un terzo che ci chiama mentre siamo in QSO entra in coda e non disturba il QSO in corso.
#[test]
fn third_caller_during_qso_waits() {
    let mut b = Bench::ft8(opts());
    b.seq.call_cq(None).expect("cq");
    b.tx(0);
    b.rx(1, &[("IU3ABC AA1AA FN42", -10)]);
    b.tx(2);
    b.rx(3, &[("IU3ABC AA1AA R-10", -10), ("IU3ABC BB2BB KN23", -8)]);
    assert_eq!(b.seq.dx(), Some("AA1AA"));
    assert_eq!(b.seq.queued(), vec!["BB2BB".to_string()]);
    assert_eq!(b.seq.ntx(), 4);
}

/// Un chiamante in coda che smette di chiamare scade (nel QSO successivo non viene servito).
#[test]
fn queued_caller_expires_when_not_heard() {
    let mut b = Bench::ft8(Options {
        queue_ttl_s: 60.0,
        ..opts()
    });
    b.seq.call_cq(None).expect("cq");
    b.tx(0);
    b.rx(1, &[("IU3ABC AA1AA FN42", -10), ("IU3ABC BB2BB KN23", -20)]);
    assert_eq!(b.seq.dx(), Some("AA1AA"));
    b.tx(2);
    b.rx(3, &[("IU3ABC AA1AA R-10", -10)]);
    b.tx(4);
    b.rx(9, &[("IU3ABC AA1AA 73", -10)]);
    assert_eq!(
        b.seq.dx(),
        None,
        "BB2BB non si sente da 8 periodi (60 s): scaduto"
    );
    assert_eq!(b.tx(10).as_deref(), Some("CQ IU3ABC JN55"));
}

/// Un chiamante piu' vecchio in coda passa prima di uno nuovo piu' forte (ordine di arrivo).
#[test]
fn queue_serves_oldest_first() {
    let mut b = Bench::ft8(opts());
    b.seq.call_cq(None).expect("cq");
    b.tx(0);
    b.rx(1, &[("IU3ABC AA1AA FN42", -2)]);
    b.tx(2);
    b.rx(3, &[("IU3ABC AA1AA R-10", -2), ("IU3ABC BB2BB KN23", -20)]);
    b.tx(4);
    b.rx(
        5,
        &[
            ("IU3ABC AA1AA 73", -2),
            ("IU3ABC BB2BB KN23", -20),
            ("IU3ABC CC3CC JN45", -3),
        ],
    );
    assert_eq!(b.seq.dx(), Some("BB2BB"));
    assert_eq!(b.seq.queued(), vec!["CC3CC".to_string()]);
}

/// «Risp B4» spento (`m_answerWorkedB4`): chi si e' gia' lavorato non viene richiamato; acceso, si'.
#[test]
fn already_worked_caller_is_ignored_unless_answer_b4() {
    for (b4, expect_dx) in [(false, None), (true, Some("AA1AA"))] {
        let mut b = Bench::ft8(Options {
            answer_worked_before: b4,
            ..opts()
        });
        b.seq.call_cq(None).expect("cq");
        b.tx(0);
        b.rx(1, &[("IU3ABC AA1AA FN42", -10)]);
        b.tx(2);
        b.rx(3, &[("IU3ABC AA1AA R-10", -10)]);
        b.tx(4);
        b.rx(5, &[("IU3ABC AA1AA 73", -10)]);
        assert!(b.seq.is_worked("AA1AA/P"));
        b.rx(7, &[("IU3ABC AA1AA FN42", -10)]);
        assert_eq!(b.seq.dx(), expect_dx, "Risp B4 = {b4}");
        assert_eq!(
            b.has(&Event::CallerIgnored {
                call: "AA1AA".into()
            }),
            !b4
        );
    }
}

// ---------------------------------------------------------------------------------------
// Watchdog, halt, 1 QSO
// ---------------------------------------------------------------------------------------

/// `txwatchdog` (`TxWatchdogTimer`, qui 6 minuti): una serie di CQ senza risposta si ferma.
#[test]
fn watchdog_stops_cq_after_six_minutes() {
    let mut b = Bench::ft8(opts());
    b.seq.call_cq(None).expect("cq");
    assert!(b.tx(0).is_some());
    assert!(b.tx(22).is_some(), "330 s: ancora in tempo");
    assert_eq!(b.tx(24), None, "360 s: scaduto");
    assert_eq!(b.stops(), vec![StopReason::Watchdog]);
    assert!(!b.seq.is_enabled());
    assert_eq!(b.tx(26), None);
}

/// `txwatchdog(false)`: il watchdog riparte quando si trasmette un messaggio diverso o arriva un progresso.
#[test]
fn watchdog_restarts_on_progress() {
    let mut b = Bench::ft8(opts());
    b.seq.call_cq(None).expect("cq");
    assert!(b.tx(0).is_some());
    b.rx(21, &[("IU3ABC LZ2KSQ KN23", -12)]);
    assert!(b.tx(22).is_some());
    b.rx(23, &[("IU3ABC LZ2KSQ R-10", -12)]);
    assert!(
        b.tx(24).is_some(),
        "a 360 s dal primo CQ, ma il QSO e' progredito"
    );
    assert!(b.stops().is_empty());
}

/// Watchdog a zero: spento.
#[test]
fn watchdog_can_be_disabled() {
    let mut b = Bench::ft8(Options {
        watchdog_min: 0,
        ..opts()
    });
    b.seq.call_cq(None).expect("cq");
    assert!(b.tx(0).is_some());
    assert!(b.tx(200).is_some());
}

/// `on_stopTxButton_clicked` (Ferma Tx) a meta' QSO: niente piu' trasmissioni, il DX resta, e si
/// riprende da dove si era con Abilita Tx.
#[test]
fn halt_midway_keeps_the_qso() {
    let mut b = Bench::ft8(opts());
    b.reply(1, "CQ LZ2KSQ KN23", -12);
    assert_eq!(b.tx(2).as_deref(), Some("LZ2KSQ IU3ABC JN55"));
    b.seq.halt();
    b.collect();
    assert_eq!(b.tx(4), None);
    assert_eq!(b.stops(), vec![StopReason::Halt]);
    assert_eq!(b.seq.dx(), Some("LZ2KSQ"));
    b.rx(5, &[("IU3ABC LZ2KSQ -07", -9)]);
    assert_eq!(b.seq.ntx(), 3);
    assert_eq!(b.tx(6), None, "ancora fermo");
    b.seq.set_enabled(true);
    assert_eq!(b.tx(6).as_deref(), Some("LZ2KSQ IU3ABC R-09"));
}

/// Fermata a meta' di una serie di CQ: dopo il QSO in corso non si riprende il CQ.
#[test]
fn halt_ends_cq_run() {
    let mut b = Bench::ft8(opts());
    b.seq.call_cq(None).expect("cq");
    b.tx(0);
    b.rx(1, &[("IU3ABC LZ2KSQ KN23", -12)]);
    b.tx(2);
    b.seq.halt();
    assert_eq!(b.seq.phase(), Phase::Report);
    b.seq.set_enabled(true);
    b.rx(3, &[("IU3ABC LZ2KSQ R-10", -12)]);
    b.tx(4);
    b.rx(5, &[("IU3ABC LZ2KSQ 73", -12)]);
    b.collect();
    assert_eq!(b.tx(6), None);
    assert_eq!(b.seq.phase(), Phase::Idle);
}

/// `SingleShotQSO` («1 QSO»): finito il QSO, Abilita Tx si spegne (`autoStopTx`), anche con una
/// serie di CQ in corso.
#[test]
fn single_qso_stops_after_the_qso() {
    let mut b = Bench::ft8(Options {
        single_qso: true,
        ..opts()
    });
    b.seq.call_cq(None).expect("cq");
    b.tx(0);
    b.rx(1, &[("IU3ABC LZ2KSQ KN23", -12)]);
    b.tx(2);
    b.rx(3, &[("IU3ABC LZ2KSQ R-10", -12), ("IU3ABC AA1AA FN42", -3)]);
    b.tx(4);
    b.rx(5, &[("IU3ABC LZ2KSQ 73", -12)]);
    assert_eq!(b.stops(), vec![StopReason::SingleQso]);
    assert_eq!(b.tx(6), None);
    assert!(b.seq.queued().is_empty());
}

/// «1 QSO»: vale anche per un tentativo fallito (`counters = false` con `m_singleshot`).
#[test]
fn single_qso_stops_after_a_failed_attempt() {
    let mut b = Bench::ft8(Options {
        single_qso: true,
        ..opts()
    });
    b.seq.call_cq(None).expect("cq");
    b.tx(0);
    b.rx(1, &[("IU3ABC LZ2KSQ KN23", -12)]);
    for slot in [2, 4, 6, 8, 10] {
        assert!(b.tx(slot).is_some());
    }
    assert_eq!(b.tx(12), None);
    assert_eq!(b.stops(), vec![StopReason::SingleQso]);
}

// ---------------------------------------------------------------------------------------
// Periodo pari/dispari, avvio tardivo
// ---------------------------------------------------------------------------------------

/// `processMessage`: `m_txFirst != (nmod!=0)`: si risponde nel periodo opposto a quello del DX.
#[test]
fn reply_picks_the_period_opposite_to_the_dx() {
    let mut b = Bench::ft8(opts());
    b.reply(3, "CQ LZ2KSQ KN23", -12);
    assert!(b.seq.tx_even());
    assert_eq!(b.tx(3), None);
    let tx = b.tx_at(4, 0.4).expect("tx");
    assert!(tx.even_slot);

    let mut b = Bench::ft8(opts());
    b.reply(4, "CQ LZ2KSQ KN23", -12);
    assert!(!b.seq.tx_even());
    assert_eq!(b.tx(4), None);
    assert!(b.tx(5).is_some());
}

/// `TxMinuteButton`: il CQ parte nel periodo scelto dall'operatore.
#[test]
fn cq_uses_the_configured_period() {
    let mut b = Bench::ft8(Options {
        tx_even: false,
        ..opts()
    });
    b.seq.call_cq(None).expect("cq");
    assert_eq!(b.tx(0), None);
    assert!(b.tx(1).is_some());
    b.seq.set_tx_even(true);
    assert!(b.tx(2).is_some());
}

/// Un chiamante che ci risponde nel periodo «sbagliato» (quello nostro) ci fa cambiare periodo
/// (`processMessage` segue il periodo del messaggio).
#[test]
fn caller_in_unexpected_period_moves_our_period() {
    let mut b = Bench::ft8(opts());
    b.seq.call_cq(None).expect("cq");
    b.tx(0);
    b.rx(2, &[("IU3ABC LZ2KSQ KN23", -12)]);
    assert!(!b.seq.tx_even());
    assert_eq!(b.tx(4), None);
    assert_eq!(b.tx(3).as_deref(), Some("LZ2KSQ IU3ABC -12"));
}

/// docs («Da verificare»): il limite di avvio tardivo non e' documentato in JTDX; Aurora usa 2 s
/// in FT8 e 1 s in FT4, modificabili. Una richiesta tardiva non parte e non conta come invio.
#[test]
fn late_request_does_not_transmit_nor_count() {
    let mut b = Bench::ft8(Options {
        limits: RepeatLimits {
            answer_cq: Some(2),
            ..RepeatLimits::default()
        },
        ..opts()
    });
    b.reply(1, "CQ LZ2KSQ KN23", -12);
    assert!(b.tx_at(2, 2.5).is_none());
    assert!(b.tx_at(2, 1.9).is_some());
    assert!(b.tx_at(4, 0.2).is_some());
    assert_eq!(b.tx_at(6, 0.2), None, "due invii fatti, tetto 2");
    assert_eq!(b.abandons(), vec![AbandonReason::NoReply]);
}

/// «Bypassa Tx1» (Decodium, TX immediato al click): la prima trasmissione puo' partire fino ai tre
/// quarti del periodo, le successive no.
#[test]
fn bypass_tx1_allows_a_late_first_transmission_only() {
    let mut b = Bench::ft8(Options {
        bypass_tx1: true,
        ..opts()
    });
    b.reply(1, "CQ LZ2KSQ KN23", -12);
    assert!(b.tx_at(2, 9.0).is_some());
    assert!(
        b.tx_at(2, 9.0).is_some(),
        "stesso periodo: stesso messaggio"
    );
    assert!(b.tx_at(4, 9.0).is_none(), "la seconda non e' piu' la prima");
}

/// Una sola chiamata per periodo: la stessa richiesta due volte da' lo stesso messaggio, contato una volta.
#[test]
fn next_tx_is_idempotent_within_a_slot() {
    let mut b = Bench::ft8(Options {
        limits: RepeatLimits {
            answer_cq: Some(2),
            ..RepeatLimits::default()
        },
        ..opts()
    });
    b.reply(1, "CQ LZ2KSQ KN23", -12);
    let a = b.tx_at(2, 0.4).expect("tx");
    let c = b.tx_at(2, 0.9).expect("tx");
    assert_eq!(a, c);
    assert!(b.tx(4).is_some(), "il secondo invio e' ancora permesso");
    assert_eq!(b.tx(6), None);
}

// ---------------------------------------------------------------------------------------
// Doppio clic: da dove si parte (processMessage)
// ---------------------------------------------------------------------------------------

/// `processMessage`: il Tx di partenza dipende da cio' che contiene il messaggio cliccato.
#[test]
fn double_click_start_message_table() {
    let cases: [(&str, bool, u8); 9] = [
        ("CQ LZ2KSQ KN23", false, 1),
        ("CQ LZ2KSQ KN23", true, 2),
        ("AA1AA LZ2KSQ -10", false, 1),
        ("IU3ABC LZ2KSQ KN23", false, 2),
        ("IU3ABC LZ2KSQ -07", false, 3),
        ("IU3ABC LZ2KSQ R-07", false, 4),
        ("IU3ABC LZ2KSQ RR73", false, 5),
        ("IU3ABC LZ2KSQ RRR", false, 5),
        ("IU3ABC LZ2KSQ 73", true, 5),
    ];
    for (text, skip, expected) in cases {
        let mut b = Bench::ft8(Options {
            skip_tx1: skip,
            ..opts()
        });
        b.reply(1, text, -10);
        assert_eq!(b.seq.ntx(), expected, "{text} (salta Tx1 = {skip})");
    }
}

/// `genStdMsgs`: i sei messaggi dopo il doppio clic (Tx6 e' il CQ).
#[test]
fn six_messages_after_double_click() {
    let mut b = Bench::ft8(opts());
    b.reply(1, "CQ LZ2KSQ KN23", -7);
    let t = b.seq.tx_texts();
    assert_eq!(t[0], "LZ2KSQ IU3ABC JN55");
    assert_eq!(t[1], "LZ2KSQ IU3ABC -07");
    assert_eq!(t[2], "LZ2KSQ IU3ABC R-07");
    assert_eq!(t[3], "LZ2KSQ IU3ABC RR73");
    assert_eq!(t[4], "LZ2KSQ IU3ABC 73");
    assert_eq!(t[5], "CQ IU3ABC JN55");
    assert_eq!(b.seq.dx_grid(), Some("KN23"));
}

/// `processMessage` («treat like a CQ»): rispondere a un messaggio diretto ad altri fa partire dal Tx1.
#[test]
fn reply_to_message_between_third_parties() {
    let mut b = Bench::ft8(opts());
    b.reply(1, "AA1AA LZ2KSQ -10", -12);
    assert_eq!(b.seq.dx(), Some("LZ2KSQ"));
    assert_eq!(b.seq.ntx(), 1);
}

/// `processMessage`: scegliere un altro DX sostituisce il precedente.
#[test]
fn replying_to_another_station_replaces_the_dx() {
    let mut b = Bench::ft8(opts());
    b.reply(1, "CQ LZ2KSQ KN23", -12);
    b.reply(1, "CQ AA1AA FN42", -8);
    assert_eq!(b.seq.dx(), Some("AA1AA"));
    assert_eq!(b.abandons(), vec![AbandonReason::Replaced]);
}

// ---------------------------------------------------------------------------------------
// Messaggi che non ci riguardano, nominativi composti, rapporti
// ---------------------------------------------------------------------------------------

/// `processMessage`: «not interested if not from QSO partner»; i messaggi tra terzi e le chiusure
/// di QSO che non esistono non cambiano nulla.
#[test]
fn messages_of_others_are_ignored() {
    let mut b = Bench::ft8(opts());
    b.seq.call_cq(None).expect("cq");
    b.tx(0);
    b.events.clear();
    b.rx(
        1,
        &[
            ("CQ AA1AA FN42", -5),
            ("BB2BB AA1AA -10", -8),
            ("BB2BB AA1AA RR73", -8),
            ("AA1AA CC3CC 73", -9),
            ("IU3ABC DD4DD RR73", -9),
            ("IU3ABC EE5EE 73", -9),
            ("IU3ABC FF6FF RRR", -9),
            ("IU3ABD GG7GG KN23", -9),
            ("TNX FOR QSO", -9),
            ("<...> HH8HH -10", -9),
        ],
    );
    assert_eq!(b.seq.dx(), None);
    assert!(b.seq.queued().is_empty());
    assert!(b.events.is_empty(), "nessun evento: {:?}", b.events);
    assert_eq!(b.tx(2).as_deref(), Some("CQ IU3ABC JN55"));
}

/// Durante un QSO i messaggi di altri non spostano il DX ne' il messaggio.
#[test]
fn messages_of_others_do_not_disturb_a_qso() {
    let mut b = Bench::ft8(opts());
    b.reply(1, "CQ LZ2KSQ KN23", -12);
    b.tx(2);
    b.rx(
        3,
        &[
            ("BB2BB AA1AA RR73", -8),
            ("CQ DD4DD JN45", -3),
            ("IU3ABC LZ2KSQ -07", -9),
        ],
    );
    assert_eq!(b.seq.dx(), Some("LZ2KSQ"));
    assert_eq!(b.seq.ntx(), 3);
}

/// `base_callsign` e `genStdMsgs`: il DX con /P si risponde con il nominativo intero e lo si
/// riconosce anche quando risponde col nominativo di base.
#[test]
fn portable_partner() {
    let mut b = Bench::ft8(opts());
    b.reply(1, "CQ AA1AA/P FN42", -12);
    assert_eq!(b.tx(2).as_deref(), Some("AA1AA/P IU3ABC JN55"));
    b.rx(3, &[("IU3ABC AA1AA -08", -9)]);
    assert_eq!(b.seq.ntx(), 3);
    assert_eq!(
        b.seq.dx(),
        Some("AA1AA/P"),
        "il nominativo di base non toglie il /P"
    );
    assert_eq!(b.tx(4).as_deref(), Some("AA1AA/P IU3ABC R-09"));
    b.rx(5, &[("IU3ABC AA1AA/P RR73", -9)]);
    assert_eq!(b.records().len(), 1);
    assert_eq!(b.records()[0].call, "AA1AA/P");
}

/// `genStdMsgs` con DX non standard (prefisso): il DX va tra `<>` e la chiusura e' nella forma
/// `DX <MIA>`.
#[test]
fn compound_partner_uses_hashed_forms() {
    let mut b = Bench::ft8(Options {
        skip_tx1: true,
        ..opts()
    });
    b.reply(1, "CQ DL/AA1AA", -12);
    assert_eq!(b.tx(2).as_deref(), Some("<DL/AA1AA> IU3ABC -12"));
    b.rx(3, &[("IU3ABC DL/AA1AA R-10", -12)]);
    assert_eq!(b.tx(4).as_deref(), Some("DL/AA1AA <IU3ABC> RR73"));
}

/// Il nostro nominativo con /P: le chiamate a «IU3ABC/P» e «IU3ABC» sono per noi (`dtext.contains("/" + m_baseCall)`).
#[test]
fn messages_to_our_base_call_with_suffix_are_ours() {
    let seq = Sequencer::new(Mode::Ft8, Station::new("IU3ABC/P", "JN55"), opts()).expect("ok");
    let mut b = Bench {
        seq,
        mode: Mode::Ft8,
        events: Vec::new(),
    };
    b.seq.call_cq(None).expect("cq");
    assert_eq!(b.tx(0).as_deref(), Some("CQ IU3ABC/P JN55"));
    b.rx(1, &[("IU3ABC LZ2KSQ KN23", -12)]);
    assert_eq!(b.seq.dx(), Some("LZ2KSQ"));
    assert_eq!(b.tx(2).as_deref(), Some("LZ2KSQ IU3ABC/P -12"));
}

/// `genStdMsgs`: il rapporto fisso al posto dell'SNR misurato; il positivo ha il segno.
#[test]
fn report_policy_fixed_and_measured() {
    let mut b = Bench::ft8(Options {
        report: ReportPolicy::Fixed(-5),
        ..opts()
    });
    b.reply(1, "CQ LZ2KSQ KN23", -20);
    assert_eq!(b.seq.tx_texts()[1], "LZ2KSQ IU3ABC -05");

    let mut b = Bench::ft8(Options {
        skip_tx1: true,
        ..opts()
    });
    b.reply(1, "CQ LZ2KSQ KN23", 5);
    assert_eq!(b.tx(2).as_deref(), Some("LZ2KSQ IU3ABC +05"));
    b.rx(3, &[("IU3ABC LZ2KSQ R+00", 0)]);
    assert_eq!(b.tx(4).as_deref(), Some("LZ2KSQ IU3ABC RR73"));
    let r = b.records()[0].clone();
    assert_eq!((r.rst_sent.as_str(), r.rst_rcvd.as_str()), ("+05", "+00"));
}

/// Tx = Rx (`LockTxFreq`): si trasmette sulla frequenza del DX.
#[test]
fn follow_dx_frequency_option() {
    let mut b = Bench::ft8(Options {
        follow_dx_freq: true,
        ..opts()
    });
    b.seq
        .reply_to(Slot(1), &Decoded::new("CQ LZ2KSQ KN23", -12, 2100))
        .expect("ok");
    assert_eq!(b.tx_at(2, 0.4).map(|t| t.freq_hz), Some(2100));
    let mut b = Bench::ft8(opts());
    b.seq
        .reply_to(Slot(1), &Decoded::new("CQ LZ2KSQ KN23", -12, 2100))
        .expect("ok");
    assert_eq!(b.tx_at(2, 0.4).map(|t| t.freq_hz), Some(1500));
}

// ---------------------------------------------------------------------------------------
// Comandi, AutoSeq spento, eventi, errori
// ---------------------------------------------------------------------------------------

/// `m_autoseq` spento: i messaggi non fanno avanzare nulla, si trasmette il messaggio scelto a mano.
#[test]
fn autoseq_off_does_not_advance_and_manual_selection_works() {
    let mut b = Bench::ft8(Options {
        auto_seq: false,
        ..opts()
    });
    b.reply(1, "CQ LZ2KSQ KN23", -12);
    assert_eq!(b.tx(2).as_deref(), Some("LZ2KSQ IU3ABC JN55"));
    b.rx(3, &[("IU3ABC LZ2KSQ -07", -9)]);
    assert_eq!(b.seq.ntx(), 1);
    b.seq.select_tx(3).expect("select");
    assert_eq!(b.tx(4).as_deref(), Some("LZ2KSQ IU3ABC R-12"));
}

/// Il pallino «Next»: la scelta a mano riparte dal messaggio scelto (anche con AutoSeq acceso).
#[test]
fn manual_selection_restarts_repeat_count() {
    let mut b = Bench::ft8(opts());
    b.reply(1, "CQ LZ2KSQ KN23", -12);
    b.tx(2);
    b.seq.select_tx(2).expect("select");
    assert_eq!(b.tx(4).as_deref(), Some("LZ2KSQ IU3ABC -12"));
}

/// `clearDX`: Canc DX libera la stazione e, in una serie di CQ, si torna al CQ.
#[test]
fn clear_dx_returns_to_cq() {
    let mut b = Bench::ft8(opts());
    b.seq.call_cq(Some("NA")).expect("cq");
    b.tx(0);
    b.rx(1, &[("IU3ABC LZ2KSQ KN23", -12)]);
    assert_eq!(b.seq.dx(), Some("LZ2KSQ"));
    b.seq.clear_dx();
    b.collect();
    assert_eq!(b.abandons(), vec![AbandonReason::Cleared]);
    assert_eq!(b.tx(2).as_deref(), Some("CQ NA IU3ABC JN55"));
}

/// Gli eventi di stato seguono le fasi (`m_QSOProgress`).
#[test]
fn state_events_follow_the_phases() {
    let mut b = Bench::ft8(opts());
    b.seq.call_cq(None).expect("cq");
    b.tx(0);
    b.rx(1, &[("IU3ABC LZ2KSQ KN23", -12)]);
    b.tx(2);
    b.rx(3, &[("IU3ABC LZ2KSQ R-10", -12)]);
    b.tx(4);
    b.rx(5, &[("IU3ABC LZ2KSQ 73", -12)]);
    let phases: Vec<Phase> = b
        .events
        .iter()
        .filter_map(|e| {
            if let Event::StateChanged { phase, .. } = e {
                Some(*phase)
            } else {
                None
            }
        })
        .collect();
    assert_eq!(
        phases,
        vec![Phase::Cq, Phase::Report, Phase::Closing, Phase::Cq]
    );
}

/// `tx_texts` senza DX: solo il CQ.
#[test]
fn texts_without_dx_only_cq() {
    let b = Bench::ft8(opts());
    let t = b.seq.tx_texts();
    assert!(t[..5].iter().all(String::is_empty));
    assert_eq!(t[5], "CQ IU3ABC JN55");
}

/// Con la trasmissione chiusa non si da' mai nulla, e un chiamante non apre un QSO.
#[test]
fn disabled_sequencer_never_transmits_nor_takes_callers() {
    let mut b = Bench::ft8(opts());
    assert_eq!(b.tx(0), None);
    b.rx(1, &[("IU3ABC LZ2KSQ KN23", -12)]);
    assert_eq!(b.seq.dx(), None);
    assert_eq!(b.tx(2), None);
}

#[test]
fn errors_are_reported_not_panicked() {
    assert!(matches!(
        Sequencer::new(Mode::Ft8, Station::new("X", "JN55"), opts()),
        Err(SeqError::InvalidCall(_))
    ));
    assert!(matches!(
        Sequencer::new(Mode::Ft8, Station::new(ME, "ZZ99"), opts()),
        Err(SeqError::InvalidGrid(_))
    ));
    let mut b = Bench::ft8(opts());
    assert!(matches!(
        b.seq.call_cq(Some("TOOLONG")),
        Err(SeqError::InvalidModifier(_))
    ));
    assert!(matches!(
        b.seq.call_cq(Some("d-x")),
        Err(SeqError::InvalidModifier(_))
    ));
    assert!(b.seq.call_cq(Some("")).is_ok());
    assert!(matches!(
        b.seq.reply_to(Slot(1), &Decoded::new("HELLO", 0, 1000)),
        Err(SeqError::NotReplyable(_))
    ));
    assert_eq!(
        b.seq
            .reply_to(Slot(1), &Decoded::new("CQ IU3ABC/P JN55", 0, 1000)),
        Err(SeqError::OwnCall)
    );
    assert_eq!(b.seq.select_tx(7), Err(SeqError::InvalidTxNumber(7)));
    assert_eq!(b.seq.select_tx(0), Err(SeqError::InvalidTxNumber(0)));
    assert_eq!(b.seq.select_tx(2), Err(SeqError::NoDx));
    assert!(b.seq.select_tx(6).is_ok());
}

/// Scenario lungo: CQ, due QSO di fila e ritorno al CQ, con una sola registrazione per QSO.
#[test]
fn two_qsos_in_a_row_each_logged_once() {
    let mut b = Bench::ft8(opts());
    b.seq.call_cq(None).expect("cq");
    b.tx(0);
    b.rx(1, &[("IU3ABC AA1AA FN42", -10)]);
    b.tx(2);
    b.rx(3, &[("IU3ABC AA1AA R-11", -10)]);
    b.tx(4);
    b.rx(5, &[("IU3ABC AA1AA 73", -10)]);
    assert_eq!(b.tx(6).as_deref(), Some("CQ IU3ABC JN55"));
    b.rx(7, &[("IU3ABC BB2BB KN23", -4)]);
    assert_eq!(b.tx(8).as_deref(), Some("BB2BB IU3ABC -04"));
    b.rx(9, &[("IU3ABC BB2BB R-02", -4)]);
    assert_eq!(b.tx(10).as_deref(), Some("BB2BB IU3ABC RR73"));
    b.rx(11, &[("IU3ABC BB2BB 73", -4)]);
    assert_eq!(b.tx(12).as_deref(), Some("CQ IU3ABC JN55"));
    let calls: Vec<&str> = b.records().iter().map(|r| r.call.as_str()).collect();
    assert_eq!(calls, vec!["AA1AA", "BB2BB"]);
}

/// `guiUpdate` (REA 0x4c4350): con `m_ntx == 5` e il messaggio che contiene «73» JTDX registra il QSO anche senza aver ricevuto un rapporto
/// (il campo `RST_RCVD` resta vuoto); una sola volta.
#[test]
fn sending_73_logs_the_qso_even_without_a_received_report_like_jtdx() {
    let mut b = Bench::ft8(opts());
    b.reply(1, "CQ LZ2KSQ KN23", -12);
    b.seq.select_tx(5).expect("Tx5");
    assert_eq!(b.tx(2).as_deref(), Some("LZ2KSQ IU3ABC 73"));
    let records = b.records();
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].call, "LZ2KSQ");
    assert_eq!(records[0].rst_rcvd, "");
}
