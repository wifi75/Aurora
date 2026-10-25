//! Le regole di JTDX 2.2.159 lette nel sorgente e verificate nell'eseguibile (`docs/analisi-jtdx-reverse-rea.md`, sezioni 2, 3 e 11): contatori e loro eccezioni,
//! cooldown, livelli di AutoSeq, ricerca tra i CQ degli altri, direzione del CQ, Hound, messaggio libero, watchdog e «Best caller first» di Decodium.
//!
//! Le prove partono a mezzogiorno UTC (il periodo `n` comincia a `12:00:00 + 15 n` s); il nostro nominativo e' IU3ABC.

use qso_native::*;

const ME: &str = "IU3ABC";
const BASE: u64 = 2880;

struct Bench {
    seq: Sequencer,
    events: Vec<Event>,
}

fn opts() -> Options {
    Options { autolog: false, ..Options::default() }
}

impl Bench {
    fn new(opts: Options) -> Bench {
        Bench { seq: Sequencer::new(Mode::Ft8, Station::new(ME, "JN55rk"), opts).expect("stazione"), events: Vec::new() }
    }

    fn collect(&mut self) {
        let mut e = self.seq.drain_events();
        self.events.append(&mut e);
    }

    fn rx(&mut self, n: u64, msgs: &[(&str, i32)]) {
        let list: Vec<Decoded> = msgs.iter().map(|(t, s)| Decoded::new(t, *s, 1200)).collect();
        let now = Slot(BASE + n + 1).start_s(Mode::Ft8) + 0.3;
        self.seq.on_decodes_at(Slot(BASE + n), &list, Some(now));
        self.collect();
    }

    fn tx(&mut self, n: u64) -> Option<String> {
        let s = Slot(BASE + n);
        let tx = self.seq.next_tx(s, s.start_s(Mode::Ft8) + 0.4);
        self.collect();
        tx.map(|t| t.text)
    }

    fn reply(&mut self, n: u64, text: &str, snr: i32) {
        self.seq.reply_to(Slot(BASE + n), &Decoded::new(text, snr, 1200)).expect("risposta");
        self.collect();
    }

    fn stops(&self) -> Vec<StopReason> {
        self.events.iter().filter_map(|e| if let Event::Stopped(r) = e { Some(*r) } else { None }).collect()
    }

    fn abandons(&self) -> Vec<AbandonReason> {
        self.events.iter().filter_map(|e| if let Event::Abandoned { reason, .. } = e { Some(*reason) } else { None }).collect()
    }

    fn records(&self) -> usize {
        self.events.iter().filter(|e| matches!(e, Event::QsoComplete(_))).count()
    }
}

fn info(f: impl Fn(&str) -> CallerInfo + Send + 'static) -> Option<CallerInfoFn> {
    Some(Box::new(f))
}

// ---------------------------------------------------------------------------------------
// Contatori: eccezioni, cooldown, valori di fabbrica
// ---------------------------------------------------------------------------------------

/// `process_Auto`: `AnswerCQ` non vale per le stazioni volute (priorita' 2-4 e 17-19) ne' per i nuovi DXCC (20-31): JTDX le richiama finche' non rispondono,
/// il watchdog scade o si ferma a mano. Vale per le altre (priorita' 0-1 e 5-16).
#[test]
fn answer_cq_counter_does_not_apply_to_wanted_stations_and_new_dxcc() {
    for (priority, gives_up) in [(0u8, true), (1, true), (2, false), (4, false), (5, true), (16, true), (17, false), (22, false), (31, false)] {
        let mut b = Bench::new(Options { limits: RepeatLimits { answer_cq: Some(2), ..RepeatLimits::default() }, ..opts() });
        b.seq.set_caller_info(info(move |_| CallerInfo { priority, ..CallerInfo::default() }));
        b.reply(1, "CQ LZ2KSQ KN23", -12);
        for slot in [2, 4, 6, 8] {
            b.tx(slot);
            b.rx(slot + 1, &[]);
        }
        assert_eq!(b.seq.dx().is_none(), gives_up, "priorita' {priority}");
        assert_eq!(b.abandons().contains(&AbandonReason::NoReply), gives_up, "priorita' {priority}");
    }
}

/// Con la direzione del CQ rigorosa (`StrictDirectionalCQ`) il contatore vale anche per i desiderati.
#[test]
fn strict_directional_cq_makes_the_counter_apply_to_wanted_stations() {
    let mut b = Bench::new(Options { strict_directional_cq: true, limits: RepeatLimits { answer_cq: Some(2), ..RepeatLimits::default() }, ..opts() });
    b.seq.set_caller_info(info(|_| CallerInfo { priority: 22, ..CallerInfo::default() }));
    b.reply(1, "CQ LZ2KSQ KN23", -12);
    for slot in [2, 4] {
        b.tx(slot);
        b.rx(slot + 1, &[]);
    }
    assert_eq!(b.seq.dx(), None);
}

/// Dopo una rinuncia di `AnswerCQ` parte un cooldown di N periodi (`m_counter`) in cui, con la ricerca tra i CQ (AutoSeq 6/7), non si risponde ad altri CQ;
/// poi si riprende. Una stazione gia' chiamata senza esito non viene richiamata prima di 5 minuti, salvo un segnale migliore (`calllist`).
#[test]
fn after_giving_up_the_search_for_cqs_pauses_and_the_same_station_is_not_called_again_for_five_minutes() {
    let mut b = Bench::new(Options {
        limits: RepeatLimits { answer_cq: Some(2), ..RepeatLimits::default() },
        call_prio_cq: true,
        ..opts()
    });
    b.reply(1, "CQ LZ2KSQ KN23", -12);
    b.tx(2);
    b.rx(3, &[("CQ LZ2KSQ KN23", -12)]);
    b.tx(4);
    // alla fine del periodo 5 il contatore (2 trasmissioni) fa lasciare LZ2KSQ; nello stesso periodo arrivano due CQ
    b.rx(5, &[("CQ LZ2KSQ KN23", -12), ("CQ K1ABC FN42", -5)]);
    assert_eq!(b.abandons(), vec![AbandonReason::NoReply]);
    assert_eq!(b.seq.dx(), None, "cooldown di 2 periodi: non si cerca tra i CQ");
    assert_eq!(b.tx(6).as_deref(), Some("CQ IU3ABC JN55"), "si chiama CQ");
    // un periodo dopo (7) il cooldown scende a 1, poi a 0: dal periodo 9 si risponde ai CQ
    b.rx(7, &[("CQ K1ABC FN42", -5)]);
    assert_eq!(b.seq.dx(), None);
    b.rx(9, &[("CQ LZ2KSQ KN23", -12), ("CQ K1ABC FN42", -5)]);
    assert_eq!(b.seq.dx(), Some("K1ABC"), "LZ2KSQ e' stato chiamato da meno di 5 minuti con lo stesso segnale: si sceglie l'altro");
}

/// I valori di fabbrica di JTDX: contatori tutti spenti, valori 2 / 3 / 5 / 4.
#[test]
fn factory_counter_values_are_those_of_jtdx() {
    assert_eq!(RepeatLimits::jtdx_factory_values(), RepeatLimits { answer_cq: Some(2), answer_in_call: Some(3), sent_r_report: Some(5), closing: Some(4) });
    assert_eq!(RepeatLimits::off(), RepeatLimits { answer_cq: None, answer_in_call: None, sent_r_report: None, closing: None });
    let o = Options::default();
    assert_eq!((o.call_mode, o.call_prio_cq, o.watchdog_min, o.rr73_marker, o.abandon_if_dx_answers_other), (3, false, 6, true, true));
}

// ---------------------------------------------------------------------------------------
// Livelli di AutoSeq e scelta di chi chiamare
// ---------------------------------------------------------------------------------------

/// Livello 1 («Call First»): si prende il primo che chiama, subito, alla sua riga; il livello 3 aspetta la fine della decodifica e sceglie il migliore.
#[test]
fn level_1_takes_the_first_caller_and_level_3_the_best_one() {
    for (mode, expected) in [(1u8, "AA1AA"), (3, "BB2BB")] {
        let mut b = Bench::new(Options { call_mode: mode, ..opts() });
        b.seq.call_cq(None).expect("cq");
        b.tx(0);
        b.rx(1, &[("IU3ABC AA1AA FN42", -15), ("IU3ABC BB2BB JN45", -5)]);
        assert_eq!(b.seq.dx(), Some(expected), "livello {mode}");
    }
}

/// Livello 0: nessuna scelta di chi chiamare (si risponde a mano), ma i messaggi del DX in collegamento fanno avanzare subito.
#[test]
fn level_0_never_picks_a_caller() {
    let mut b = Bench::new(Options { call_mode: 0, ..opts() });
    b.seq.call_cq(None).expect("cq");
    b.tx(0);
    b.rx(1, &[("IU3ABC AA1AA FN42", -15)]);
    assert_eq!(b.seq.dx(), None);
    // il doppio clic sceglie e poi il QSO avanza da solo
    b.seq.reply_to(Slot(BASE + 1), &Decoded::new("IU3ABC AA1AA FN42", -15, 1200)).expect("risposta");
    assert_eq!(b.seq.ntx(), 2);
    b.tx(2);
    b.rx(3, &[("IU3ABC AA1AA R-10", -15)]);
    assert_eq!(b.seq.ntx(), 4);
}

/// Livelli 6/7 (ricerca tra i CQ): senza nessuno che ci chiama si risponde a un CQ del periodo che precede il nostro periodo di trasmissione; senza la
/// ricerca (livelli 1-3) si continua a chiamare CQ.
#[test]
fn level_7_answers_a_cq_when_nobody_calls_us_and_level_3_keeps_calling() {
    let mut with = Bench::new(Options { call_prio_cq: true, ..opts() });
    with.seq.call_cq(None).expect("cq");
    with.tx(0);
    with.rx(1, &[("CQ K1ABC FN42", -8)]);
    assert_eq!(with.seq.dx(), Some("K1ABC"));
    assert_eq!(with.tx(2).as_deref(), Some("K1ABC IU3ABC JN55"), "Tx1: risposta al CQ");
    let mut without = Bench::new(opts());
    without.seq.call_cq(None).expect("cq");
    without.tx(0);
    without.rx(1, &[("CQ K1ABC FN42", -8)]);
    assert_eq!(without.seq.dx(), None);
    assert_eq!(without.tx(2).as_deref(), Some("CQ IU3ABC JN55"));
}

/// La ricerca tra i CQ vale solo se il prossimo periodo di trasmissione e' quello che segue il periodo decodificato: un CQ del nostro periodo non si cerca.
#[test]
fn the_cq_search_only_looks_at_the_period_we_could_answer() {
    let mut b = Bench::new(Options { call_prio_cq: true, tx_even: false, ..opts() });
    b.seq.call_cq(None).expect("cq");
    // trasmettiamo nei dispari: il periodo 1 (dispari) e' il nostro, i CQ decodificati nel periodo 2 (pari) sono quelli a cui possiamo rispondere nel 3
    b.rx(1, &[("CQ K1ABC FN42", -8)]);
    assert_eq!(b.seq.dx(), None);
    b.rx(2, &[("CQ K1ABC FN42", -8)]);
    assert_eq!(b.seq.dx(), Some("K1ABC"));
}

/// Un CQ diretto: «CQ EU» chiamato da un americano non e' per noi se siamo in Europa; il nostro «CQ DX» esclude chi e' del nostro continente.
#[test]
fn the_direction_of_the_cq_is_respected() {
    let mut b = Bench::new(Options { call_prio_cq: true, my_continent: "EU".to_string(), ..opts() });
    b.seq.set_caller_info(info(|c| CallerInfo { continent: if c.starts_with('K') { "NA".to_string() } else { "EU".to_string() }, ..CallerInfo::default() }));
    b.seq.call_cq(None).expect("cq");
    b.tx(0);
    // «CQ AS» (per l'Asia) di un americano non e' per noi, europei
    b.rx(1, &[("CQ AS K1ABC FN42", -8)]);
    assert_eq!(b.seq.dx(), None);
    b.rx(3, &[("CQ DX K1ABC FN42", -8)]);
    assert_eq!(b.seq.dx(), Some("K1ABC"), "«CQ DX» da un altro continente vale");
    // chi chiama il nostro «CQ DX» dal nostro stesso continente e' escluso
    let mut b = Bench::new(Options { my_continent: "EU".to_string(), ..opts() });
    b.seq.set_caller_info(info(|c| CallerInfo { continent: if c.starts_with('K') { "NA".to_string() } else { "EU".to_string() }, ..CallerInfo::default() }));
    b.seq.call_cq(Some("DX")).expect("cq");
    b.tx(0);
    b.rx(1, &[("IU3ABC DL1AAA JO62", -5)]);
    assert_eq!(b.seq.dx(), None, "un europeo non risponde a «CQ DX»");
    b.rx(3, &[("IU3ABC K1ABC FN42", -15)]);
    assert_eq!(b.seq.dx(), Some("K1ABC"));
}

// ---------------------------------------------------------------------------------------
// «Best caller first» (Decodium)
// ---------------------------------------------------------------------------------------

/// Con JTDX vince il segnale piu' forte; con «Best caller first» il paese nuovo (+40) batte il segnale (-9,4 per il gia' lavorato su banda).
#[test]
fn best_caller_first_prefers_a_new_country_over_a_strong_worked_station() {
    for (selection, expected) in [(CallerSelection::Jtdx, "BB2BB"), (CallerSelection::BestCallerFirst, "AA1AA")] {
        let mut b = Bench::new(Options { caller_selection: selection, ..opts() });
        b.seq.set_caller_info(info(|c| match c {
            "AA1AA" => CallerInfo { dxcc_new: true, ..CallerInfo::default() },
            "BB2BB" => CallerInfo { worked_band: true, ..CallerInfo::default() },
            _ => CallerInfo::default(),
        }));
        b.seq.call_cq(None).expect("cq");
        b.tx(0);
        b.rx(1, &[("IU3ABC AA1AA FN42", -15), ("IU3ABC BB2BB JN45", 5)]);
        assert_eq!(b.seq.dx(), Some(expected), "{selection:?}");
    }
}

// ---------------------------------------------------------------------------------------
// Hound, messaggio libero, watchdog
// ---------------------------------------------------------------------------------------

/// Modo Hound: si parte sempre da Tx1 (nessun «Salta Tx1»), con l'RR73 della spedizione il QSO si registra e la trasmissione si ferma senza mandare il 73.
#[test]
fn hound_mode_logs_and_stops_at_the_rr73_of_the_expedition() {
    let mut b = Bench::new(Options { hound: true, skip_tx1: true, ..opts() });
    b.reply(1, "CQ LZ2KSQ KN23", -12);
    assert_eq!(b.seq.ntx(), 1, "in Hound «Salta Tx1» non vale");
    assert_eq!(b.tx(2).as_deref(), Some("LZ2KSQ IU3ABC JN55"));
    b.rx(3, &[("IU3ABC LZ2KSQ -07", -9)]);
    assert_eq!(b.tx(4).as_deref(), Some("LZ2KSQ IU3ABC R-09"));
    b.rx(5, &[("IU3ABC LZ2KSQ RR73", -9)]);
    assert_eq!(b.records(), 1, "registrato alla ricezione dell'RR73");
    assert_eq!(b.stops(), vec![StopReason::QsoFinished]);
    assert_eq!(b.tx(6), None, "nessun 73");
}

/// Hound: il CQ non e' ammesso, e dopo tre trasmissioni dell'R+rapporto senza risposta si lascia la stazione.
#[test]
fn hound_mode_has_no_cq_and_gives_up_after_three_r_reports() {
    let mut b = Bench::new(Options { hound: true, ..opts() });
    b.reply(1, "CQ LZ2KSQ KN23", -12);
    b.tx(2);
    b.rx(3, &[("IU3ABC LZ2KSQ -07", -9)]);
    for slot in [4, 6, 8, 10] {
        b.tx(slot);
        b.rx(slot + 1, &[]);
    }
    assert!(b.stops().contains(&StopReason::Halt), "{:?}", b.stops());
    let mut c = Bench::new(Options { hound: true, ..opts() });
    c.seq.call_cq(None).expect("cq");
    assert_eq!(c.tx(0), None, "il CQ non e' ammesso in Hound");
}

/// Un messaggio libero che non e' un 73 spegne AutoSeq e lo riaccende alla generazione dei messaggi successiva (`m_FTsetAutoSeqOff`, `m_wasAutoSeq`).
#[test]
fn a_free_text_switches_autoseq_off_until_the_messages_are_generated_again() {
    let mut b = Bench::new(opts());
    assert!(b.seq.auto_seq_effective());
    b.seq.note_free_text("GRAZIE PER IL QSO");
    assert!(!b.seq.auto_seq_effective());
    // con AutoSeq spento un caller non viene preso
    b.seq.call_cq(None).expect("cq (txb6 la riaccende)");
    assert!(b.seq.auto_seq_effective(), "Tx6 riaccende AutoSeq");
    b.seq.note_free_text("TNX 73");
    assert!(b.seq.auto_seq_effective(), "un 73 non lo spegne");
    assert!(is_autoseq_73("TU 73 GL"));
    assert!(is_autoseq_73("K1ABC TNX"));
    assert!(!is_autoseq_73("GRAZIE PER IL QSO"));
    assert!(!is_autoseq_73("A B C D 73"), "conta solo la prima meta' delle parole (le prime quattro)");
    b.seq.note_free_text("CIAO");
    b.seq.regen_messages();
    assert!(b.seq.auto_seq_effective(), "Gen.Msgs lo riaccende");
}

/// Il watchdog si azzera con un intervento dell'operatore (`eventFilter`: tasto, clic, finestra attiva).
#[test]
fn an_operator_action_resets_the_watchdog() {
    let mut b = Bench::new(opts());
    b.seq.call_cq(None).expect("cq");
    for slot in (0..20).step_by(2) {
        assert!(b.tx(slot).is_some());
        b.rx(slot + 1, &[]);
    }
    b.seq.user_activity();
    for slot in (20..30).step_by(2) {
        assert!(b.tx(slot).is_some(), "periodo {slot}: il watchdog e' ripartito");
        b.rx(slot + 1, &[]);
    }
    assert!(b.stops().is_empty());
}

/// Il modo e la banda: al cambio di modo o di banda la sequenza riparte da zero (memoria dei QSO e coda svuotate, trasmissione ferma).
#[test]
fn a_band_change_forgets_the_callers_and_the_history() {
    let mut b = Bench::new(opts());
    b.seq.call_cq(None).expect("cq");
    b.tx(0);
    b.rx(1, &[("IU3ABC AA1AA FN42", -15), ("IU3ABC BB2BB JN45", -5)]);
    assert!(b.seq.dx().is_some());
    b.seq.band_changed();
    b.collect();
    assert_eq!(b.seq.dx(), None);
    assert!(b.seq.queued().is_empty());
    assert!(!b.seq.is_enabled());
    assert!(b.seq.history().entry("AA1AA").is_none(), "`QsoHistory::init`");
    assert!(b.stops().contains(&StopReason::Halt));
}
