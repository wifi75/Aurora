//! Le salvaguardie dell'automazione, una per una. Il tempo lo passa il test.

use aurora_core::{DecodeTag, DecodedMessage};
use qso_engine::{Brain, DecisionKind, Limits, Mode, Rules};

fn decode(id: u64, text: &str, tag: Option<DecodeTag>, snr: i32) -> DecodedMessage {
    DecodedMessage { id, time_ms: 0, snr, delta_time: 0.0, freq_hz: 1500, text: text.into(), tag, country: None, continent: None, hint: false }
}

fn cq(id: u64, call: &str) -> DecodedMessage {
    decode(id, &format!("CQ {call} FN42"), Some(DecodeTag::NewDxcc), -10)
}

const T0: u64 = 1_000_000;

fn live_brain() -> Brain {
    let mut b = Brain::new(Rules::default(), Limits::default(), true);
    b.set_mode(Mode::Live).unwrap();
    b.set_enabled(true, T0);
    b
}

fn kind(b: &mut Brain, d: &DecodedMessage, now: u64) -> DecisionKind {
    b.on_decode(d, now).expect("doveva decidere qualcosa").kind
}

#[test]
fn rules_validation_rejects_dangerous_or_absurd_rules() {
    assert!(Rules::default().is_valid());
    assert!(!Rules { reply_to: vec![], ..Rules::default() }.is_valid(), "senza etichette non risponderebbe mai");
    assert!(!Rules { reply_to: vec![DecodeTag::Cq], ..Rules::default() }.is_valid(), "rispondere a ogni CQ non e' ammesso");
    assert!(!Rules { min_snr: -90, ..Rules::default() }.is_valid());
    assert!(!Rules { max_attempts_per_call: 0, ..Rules::default() }.is_valid());
    assert!(!Rules { excluded_calls: vec!["NON UN NOMINATIVO".into()], ..Rules::default() }.is_valid());
}

#[test]
fn rules_survive_a_json_roundtrip() {
    let rules = Rules { min_snr: -12, excluded_calls: vec!["AT1AA".into()], ..Rules::default() };
    let back: Rules = serde_json::from_str(&serde_json::to_string(&rules).unwrap()).unwrap();
    assert_eq!(back, rules);
}

#[test]
fn starts_off_and_in_dry_run() {
    let mut b = Brain::new(Rules::default(), Limits::default(), true);
    assert!(!b.state().enabled);
    assert_eq!(b.state().mode, Mode::DryRun);
    assert!(b.on_decode(&cq(1, "AT1AA"), T0).is_none(), "da spento non decide e non registra nulla");
    assert!(b.state().log.is_empty());
}

#[test]
fn dry_run_says_what_it_would_do_without_replying() {
    let mut b = Brain::new(Rules::default(), Limits::default(), true);
    b.set_enabled(true, T0);
    let d = b.on_decode(&cq(7, "AT1AA"), T0).unwrap();
    assert_eq!(d.kind, DecisionKind::WouldReply);
    assert_eq!(d.decode_id, Some(7));
    assert!(d.reason.contains("AT1AA"), "{}", d.reason);
    assert_eq!(b.state().replies_today, 0, "la prova a vuoto non consuma il tetto giornaliero");
}

#[test]
fn live_mode_needs_permission_to_transmit() {
    let mut closed = Brain::new(Rules::default(), Limits::default(), false);
    assert!(closed.set_mode(Mode::Live).is_err());
    assert_eq!(closed.state().mode, Mode::DryRun);
}

#[test]
fn live_replies_to_a_wanted_cq() {
    let mut b = live_brain();
    let d = b.on_decode(&cq(7, "AT1AA"), T0).unwrap();
    assert_eq!(d.kind, DecisionKind::Reply);
    assert_eq!(b.state().replies_today, 1);
}

#[test]
fn ignores_plain_cq_messages_between_others_and_low_snr() {
    let mut b = live_brain();
    assert!(b.on_decode(&decode(1, "CQ AT1AA FN42", Some(DecodeTag::Cq), -5), T0).is_none(), "un CQ comune non interessa");
    assert!(b.on_decode(&decode(2, "BB2BB AT1AA -10", Some(DecodeTag::NewDxcc), -5), T0).is_none(), "non e' un CQ");
    let weak = b.on_decode(&decode(3, "CQ AT1AA FN42", Some(DecodeTag::NewDxcc), -24), T0).unwrap();
    assert_eq!(weak.kind, DecisionKind::Skipped);
    assert!(weak.reason.contains("-24"), "{}", weak.reason);
}

#[test]
fn excluded_calls_are_skipped_case_insensitively() {
    let rules = Rules { excluded_calls: vec!["at1aa".into()], ..Rules::default() };
    let mut b = Brain::new(rules, Limits::default(), true);
    b.set_mode(Mode::Live).unwrap();
    b.set_enabled(true, T0);
    assert_eq!(kind(&mut b, &cq(1, "AT1AA"), T0), DecisionKind::Skipped);
}

#[test]
fn only_one_qso_at_a_time_then_free_after_logged_or_timeout() {
    let mut b = live_brain();
    assert_eq!(kind(&mut b, &cq(1, "AT1AA"), T0), DecisionKind::Reply);
    let busy = b.on_decode(&cq(2, "LM1XY"), T0 + 15).unwrap();
    assert_eq!(busy.kind, DecisionKind::Skipped);
    assert!(busy.reason.contains("AT1AA"), "{}", busy.reason);
    b.on_qso_logged("AT1AA");
    assert_eq!(kind(&mut b, &cq(3, "LM1XY"), T0 + 16), DecisionKind::Reply);
    // senza QSO registrato, dopo 60 s si puo' passare ad altro
    assert_eq!(kind(&mut b, &cq(4, "KK1ZZ"), T0 + 16 + 61), DecisionKind::Reply);
}

#[test]
fn gives_up_on_a_station_after_the_attempt_limit() {
    let rules = Rules { max_attempts_per_call: 2, ..Rules::default() };
    let mut b = Brain::new(rules, Limits { max_consecutive_cycles: 99, ..Limits::default() }, true);
    b.set_mode(Mode::Live).unwrap();
    b.set_enabled(true, T0);
    assert_eq!(kind(&mut b, &cq(1, "AT1AA"), T0), DecisionKind::Reply);
    assert_eq!(kind(&mut b, &cq(2, "AT1AA"), T0 + 100), DecisionKind::Reply);
    assert_eq!(kind(&mut b, &cq(3, "AT1AA"), T0 + 200), DecisionKind::Skipped);
}

#[test]
fn consecutive_cycle_cap_switches_the_automation_off() {
    let limits = Limits { max_consecutive_cycles: 2, ..Limits::default() };
    let mut b = Brain::new(Rules::default(), limits, true);
    b.set_mode(Mode::Live).unwrap();
    b.set_enabled(true, T0);
    assert_eq!(kind(&mut b, &cq(1, "AT1AA"), T0), DecisionKind::Reply);
    assert_eq!(kind(&mut b, &cq(2, "LM1XY"), T0 + 100), DecisionKind::Reply);
    let blocked = b.on_decode(&cq(3, "KK1ZZ"), T0 + 200).unwrap();
    assert_eq!(blocked.kind, DecisionKind::Blocked);
    assert!(!b.state().enabled, "al tetto si spegne e va riacceso a mano");
}

#[test]
fn a_logged_qso_resets_the_consecutive_counter() {
    let limits = Limits { max_consecutive_cycles: 2, ..Limits::default() };
    let mut b = Brain::new(Rules::default(), limits, true);
    b.set_mode(Mode::Live).unwrap();
    b.set_enabled(true, T0);
    assert_eq!(kind(&mut b, &cq(1, "AT1AA"), T0), DecisionKind::Reply);
    b.on_qso_logged("AT1AA");
    assert_eq!(kind(&mut b, &cq(2, "LM1XY"), T0 + 20), DecisionKind::Reply);
    b.on_qso_logged("LM1XY");
    assert_eq!(kind(&mut b, &cq(3, "KK1ZZ"), T0 + 40), DecisionKind::Reply);
}

#[test]
fn daily_cap_blocks_until_the_next_utc_day() {
    let limits = Limits { max_per_day: 1, max_consecutive_cycles: 99, ..Limits::default() };
    let mut b = Brain::new(Rules::default(), limits, true);
    b.set_mode(Mode::Live).unwrap();
    let day = 86_400 * 20;
    b.set_enabled(true, day + 100);
    assert_eq!(kind(&mut b, &cq(1, "AT1AA"), day + 100), DecisionKind::Reply);
    b.on_qso_logged("AT1AA");
    assert_eq!(kind(&mut b, &cq(2, "LM1XY"), day + 200), DecisionKind::Blocked);
    assert!(b.state().enabled, "il tetto giornaliero blocca ma non spegne");
    assert_eq!(kind(&mut b, &cq(3, "LM1XY"), day + 86_400 + 5), DecisionKind::Reply);
}

#[test]
fn stop_switches_everything_off_and_logs_it() {
    let mut b = live_brain();
    b.on_halt(T0 + 5);
    assert!(!b.state().enabled);
    assert_eq!(b.state().log.last().unwrap().kind, DecisionKind::Halted);
    assert!(b.on_decode(&cq(1, "AT1AA"), T0 + 6).is_none());
}

#[test]
fn dead_man_switches_off_when_nobody_is_connected_for_too_long() {
    let limits = Limits { dead_man_secs: 120, ..Limits::default() };
    let mut b = Brain::new(Rules::default(), limits, true);
    b.set_mode(Mode::Live).unwrap();
    b.set_enabled(true, T0);
    b.tick(T0 + 60, 0);
    assert!(b.state().enabled, "ancora dentro il margine");
    b.tick(T0 + 119, 1);
    b.tick(T0 + 130, 0);
    assert!(b.state().enabled, "un client e' tornato: il conto riparte");
    b.tick(T0 + 130 + 121, 0);
    assert!(!b.state().enabled);
    assert_eq!(b.state().log.last().unwrap().kind, DecisionKind::Paused);
}

#[test]
fn unattended_mode_ignores_the_dead_man() {
    let limits = Limits { run_unattended: true, ..Limits::default() };
    let mut b = Brain::new(Rules::default(), limits, true);
    b.set_enabled(true, T0);
    b.tick(T0 + 100_000, 0);
    assert!(b.state().enabled);
}

#[test]
fn unreadable_sender_is_skipped_not_guessed() {
    let mut b = live_brain();
    let d = b.on_decode(&decode(1, "CQ <...> FN42", Some(DecodeTag::NewDxcc), -5), T0).unwrap();
    assert_eq!(d.kind, DecisionKind::Skipped);
}

#[test]
fn log_keeps_only_the_latest_decisions() {
    let mut b = Brain::new(Rules::default(), Limits::default(), true);
    b.set_enabled(true, T0);
    for i in 0..300u64 {
        b.on_decode(&decode(i, "CQ AT1AA FN42", Some(DecodeTag::NewDxcc), -30), T0 + i);
    }
    assert_eq!(b.state().log.len(), 100);
}

#[test]
fn a_ghost_decode_is_skipped_with_the_reason_and_does_not_use_up_an_attempt() {
    let mut b = live_brain();
    // «?» = decodifica incerta; un nominativo che non lo e'; un segnale molto debole di una stazione mai sentita
    let uncertain = decode(1, "CQ AT1AA FN42 ?", Some(DecodeTag::NewDxcc), -10);
    let d = b.on_decode(&uncertain, T0).unwrap();
    assert_eq!(d.kind, DecisionKind::Skipped);
    assert!(d.reason.contains("incerta"), "{}", d.reason);
    let shape = decode(2, "CQ AAAA1 FN42", Some(DecodeTag::NewDxcc), -10);
    assert!(b.on_decode(&shape, T0 + 1).unwrap().reason.contains("forma di un nominativo"));
    let weak = decode(3, "CQ AT2AA FN42", Some(DecodeTag::NewDxcc), -25);
    let d = b.on_decode(&weak, T0 + 2).unwrap();
    assert_eq!(d.kind, DecisionKind::Skipped);
    assert!(d.reason.contains("mai sentita"), "{}", d.reason);
    assert_eq!(b.state().replies_today, 0, "nessuna risposta partita");
    assert_eq!(b.on_decode(&cq(4, "AT1AA"), T0 + 3).unwrap().kind, DecisionKind::Reply, "la stessa stazione, decodificata bene, si risponde");
}

#[test]
fn a_weak_signal_is_fine_once_the_station_has_been_heard() {
    let mut b = Brain::new(Rules { min_snr: -30, ..Rules::default() }, Limits::default(), true);
    b.set_mode(Mode::Live).unwrap();
    b.set_enabled(true, T0);
    // la stazione e' stata sentita (anche non desiderata, anche prima di accendere l'automazione)
    b.on_decode(&decode(1, "CQ AT1AA FN42", None, -15), T0);
    let weak = decode(2, "CQ AT1AA FN42", Some(DecodeTag::NewDxcc), -25);
    assert_eq!(b.on_decode(&weak, T0 + 1).unwrap().kind, DecisionKind::Reply);
}

#[test]
fn the_heard_stations_are_remembered_even_while_the_automation_is_off() {
    let mut b = Brain::new(Rules { min_snr: -30, ..Rules::default() }, Limits::default(), true);
    assert!(b.on_decode(&decode(1, "CQ AT1AA FN42", None, -15), T0).is_none());
    b.set_mode(Mode::Live).unwrap();
    b.set_enabled(true, T0 + 5);
    let weak = decode(2, "CQ AT1AA FN42", Some(DecodeTag::NewDxcc), -25);
    assert_eq!(b.on_decode(&weak, T0 + 6).unwrap().kind, DecisionKind::Reply);
}

fn wanted_only() -> Rules {
    Rules { reply_to: vec![], reply_to_wanted: true, ..Rules::default() }
}

fn matcher() -> qso_engine::WantedMatcher {
    std::sync::Arc::new(|d: &DecodedMessage, dial: Option<u64>| {
        let wanted = d.text.contains("T1WANT") || (d.text.contains("T2BAND") && dial == Some(14_074_000));
        wanted.then(|| "nominativo".to_string())
    })
}

#[test]
fn replies_to_the_wishlist_when_the_rule_is_on_and_says_why() {
    assert!(wanted_only().is_valid(), "senza etichette ma con i desiderati e' una regola sensata");
    assert!(!Rules { reply_to: vec![], ..Rules::default() }.is_valid(), "nessuna delle due: non risponderebbe mai");
    let mut b = Brain::new(wanted_only(), Limits::default(), true);
    b.set_wanted_matcher(matcher());
    b.set_mode(Mode::Live).unwrap();
    b.set_enabled(true, T0);
    let d = b.on_decode(&decode(1, "CQ T1WANT FN42", Some(DecodeTag::Cq), -10), T0).unwrap();
    assert_eq!(d.kind, DecisionKind::Reply);
    assert!(d.reason.contains("lista dei desiderati") && d.reason.contains("nominativo"), "{}", d.reason);
    assert!(b.on_decode(&decode(2, "CQ AT9AA FN42", Some(DecodeTag::NewDxcc), -10), T0 + 100).is_none(), "gli altri CQ non interessano");
}

#[test]
fn the_wishlist_rule_does_nothing_when_off_and_is_not_triggered_by_a_reply_to_someone_else() {
    let mut b = Brain::new(Rules { reply_to_wanted: false, ..Rules::default() }, Limits::default(), true);
    b.set_wanted_matcher(matcher());
    b.set_mode(Mode::Live).unwrap();
    b.set_enabled(true, T0);
    assert!(b.on_decode(&decode(1, "CQ T1WANT FN42", Some(DecodeTag::Cq), -10), T0).is_none());
    let mut b = Brain::new(wanted_only(), Limits::default(), true);
    b.set_wanted_matcher(matcher());
    b.set_mode(Mode::Live).unwrap();
    b.set_enabled(true, T0);
    assert!(b.on_decode(&decode(2, "IU3ABC T1WANT -10", Some(DecodeTag::Cq), -10), T0).is_none(), "non e' un CQ");
}

#[test]
fn the_wishlist_rule_keeps_every_safeguard_and_the_ghost_filter() {
    let mut b = Brain::new(wanted_only(), Limits { max_consecutive_cycles: 1, ..Limits::default() }, true);
    b.set_wanted_matcher(matcher());
    b.set_mode(Mode::Live).unwrap();
    b.set_enabled(true, T0);
    let weak = decode(1, "CQ T1WANT FN42", Some(DecodeTag::Cq), -26);
    assert_eq!(b.on_decode(&weak, T0).unwrap().kind, DecisionKind::Skipped, "un desiderato debole e mai sentito resta sospetto");
    assert_eq!(b.on_decode(&decode(2, "CQ T1WANT FN42", Some(DecodeTag::Cq), -10), T0 + 1).unwrap().kind, DecisionKind::Reply);
    // il tetto delle risposte di fila vale anche per i desiderati
    b.on_qso_logged("T1WANT");
    assert_eq!(b.on_decode(&decode(3, "CQ T1WANT FN42", Some(DecodeTag::Cq), -10), T0 + 200).unwrap().kind, DecisionKind::Reply);
    assert_eq!(b.on_decode(&decode(4, "CQ T1WANT FN42", Some(DecodeTag::Cq), -10), T0 + 400).unwrap().kind, DecisionKind::Blocked);
}

#[test]
fn the_band_filter_of_the_wishlist_sees_the_frequency_of_the_radio() {
    let mut b = Brain::new(wanted_only(), Limits::default(), true);
    b.set_wanted_matcher(matcher());
    b.set_mode(Mode::Live).unwrap();
    b.set_enabled(true, T0);
    let d = decode(1, "CQ T2BAND FN42", Some(DecodeTag::Cq), -10);
    assert!(b.on_decode(&d, T0).is_none(), "frequenza della radio ancora ignota");
    b.on_dial(14_074_000);
    assert_eq!(b.on_decode(&d, T0 + 1).unwrap().kind, DecisionKind::Reply);
}

fn tagged(id: u64, call: &str, tag: DecodeTag, snr: i32) -> DecodedMessage {
    decode(id, &format!("CQ {call} FN42"), Some(tag), snr)
}

fn replies(decisions: &[qso_engine::Decision]) -> Vec<String> {
    decisions.iter().filter(|d| matches!(d.kind, DecisionKind::Reply | DecisionKind::WouldReply)).filter_map(|d| d.call.clone()).collect()
}

#[test]
fn among_several_cqs_in_one_cycle_it_answers_only_the_rarest() {
    let mut b = live_brain();
    let batch = [
        tagged(1, "AT1AA", DecodeTag::NewGrid, -5),
        tagged(2, "AT2AA", DecodeTag::NewDxcc, -15),
        tagged(3, "AT3AA", DecodeTag::NewDxccBand, -2),
        tagged(4, "AT4AA", DecodeTag::NewZone, -3),
    ];
    let decisions = b.on_batch(&batch, T0);
    assert_eq!(replies(&decisions), vec!["AT2AA"], "il paese mai collegato batte tutto, anche con meno segnale");
    let info = decisions.iter().find(|d| d.kind == DecisionKind::Info).expect("dice perche' ha scelto");
    assert!(info.reason.contains("scelto AT2AA") && info.reason.contains("AT1AA, AT3AA, AT4AA"), "{}", info.reason);
    assert_eq!(b.state().replies_today, 1, "una sola risposta partita");
}

#[test]
fn with_the_same_rarity_the_stronger_signal_wins_and_then_the_first_arrived() {
    let mut b = live_brain();
    let batch = [tagged(1, "AT1AA", DecodeTag::NewDxcc, -12), tagged(2, "AT2AA", DecodeTag::NewDxcc, -4), tagged(3, "AT3AA", DecodeTag::NewDxcc, -4)];
    assert_eq!(replies(&b.on_batch(&batch, T0)), vec!["AT2AA"]);
}

#[test]
fn the_rule_can_prefer_the_strongest_signal() {
    let rules = Rules { prefer: qso_engine::Prefer::Snr, ..Rules::default() };
    let mut b = Brain::new(rules, Limits::default(), true);
    b.set_mode(Mode::Live).unwrap();
    b.set_enabled(true, T0);
    // il paese nuovo e' debole (-17), la griglia nuova e' forte: con «segnale» vince la griglia
    let batch = [tagged(1, "AT1AA", DecodeTag::NewDxcc, -17), tagged(2, "AT2AA", DecodeTag::NewGrid, -3)];
    assert_eq!(replies(&b.on_batch(&batch, T0)), vec!["AT2AA"]);
}

#[test]
fn a_wishlist_station_beats_any_novelty() {
    let mut b = Brain::new(Rules { reply_to: vec![DecodeTag::NewDxcc], reply_to_wanted: true, ..Rules::default() }, Limits::default(), true);
    b.set_wanted_matcher(std::sync::Arc::new(|d: &DecodedMessage, _| d.text.contains("T1WANT").then(|| "nominativo".to_string())));
    b.set_mode(Mode::Live).unwrap();
    b.set_enabled(true, T0);
    let batch = [tagged(1, "AT1AA", DecodeTag::NewDxcc, -3), tagged(2, "T1WANT", DecodeTag::Cq, -14)];
    assert_eq!(replies(&b.on_batch(&batch, T0)), vec!["T1WANT"]);
}

#[test]
fn skipped_cqs_in_the_batch_do_not_compete_and_a_lone_candidate_needs_no_explanation() {
    let mut b = live_brain();
    let batch = [tagged(1, "AT1AA", DecodeTag::NewDxcc, -30), tagged(2, "AT2AA", DecodeTag::NewGrid, -5)];
    let decisions = b.on_batch(&batch, T0);
    assert_eq!(replies(&decisions), vec!["AT2AA"], "il primo e' sotto il segnale minimo");
    assert!(decisions.iter().all(|d| d.kind != DecisionKind::Info), "un solo candidato: niente spiegazione della scelta");
}

#[test]
fn the_caps_apply_to_the_chosen_reply_only() {
    let mut b = Brain::new(Rules::default(), Limits { max_per_day: 1, ..Limits::default() }, true);
    b.set_mode(Mode::Live).unwrap();
    b.set_enabled(true, T0);
    let batch = [tagged(1, "AT1AA", DecodeTag::NewDxcc, -5), tagged(2, "AT2AA", DecodeTag::NewDxcc, -6)];
    assert_eq!(replies(&b.on_batch(&batch, T0)).len(), 1);
    let again = [tagged(3, "AT3AA", DecodeTag::NewDxcc, -5)];
    let later = b.on_batch(&again, T0 + 100);
    assert_eq!(later.last().map(|d| d.kind), Some(DecisionKind::Blocked), "tetto giornaliero raggiunto");
}

#[test]
fn rules_without_the_preference_in_an_old_file_default_to_rarity() {
    let old = r#"{"reply_to":["new_dxcc"],"min_snr":-18,"excluded_calls":[],"max_attempts_per_call":2}"#;
    let rules: Rules = serde_json::from_str(old).unwrap();
    assert_eq!(rules.prefer, qso_engine::Prefer::Rarity);
    assert!(!rules.reply_to_wanted);
}

#[test]
fn a_station_that_never_answered_can_be_tried_again_after_the_cooldown() {
    let rules = Rules { max_attempts_per_call: 1, cooldown_secs: 900, ..Rules::default() };
    let mut b = Brain::new(rules, Limits { max_consecutive_cycles: 99, ..Limits::default() }, true);
    b.set_mode(Mode::Live).unwrap();
    b.set_enabled(true, T0);
    assert_eq!(kind(&mut b, &cq(1, "AT1AA"), T0), DecisionKind::Reply);
    assert_eq!(kind(&mut b, &cq(2, "AT1AA"), T0 + 100), DecisionKind::Skipped, "ancora nella pausa");
    assert_eq!(kind(&mut b, &cq(3, "AT1AA"), T0 + 900 + 61), DecisionKind::Reply, "la pausa e' scaduta");
}

#[test]
fn the_cooldown_must_be_a_sensible_time() {
    assert!(!Rules { cooldown_secs: 10, ..Rules::default() }.is_valid());
    assert!(!Rules { cooldown_secs: 100_000, ..Rules::default() }.is_valid());
    assert!(Rules { cooldown_secs: 600, ..Rules::default() }.is_valid());
}

fn directional(id: u64, text: &str, continent: Option<&str>) -> DecodedMessage {
    DecodedMessage { continent: continent.map(str::to_string), ..decode(id, text, Some(DecodeTag::NewDxcc), -10) }
}

#[test]
fn a_directional_cq_is_answered_only_from_the_right_continent() {
    let mut b = live_brain();
    b.set_own_continent(Some("EU".into()));
    assert_eq!(kind(&mut b, &directional(1, "CQ NA K1ABC FN42", Some("NA")), T0), DecisionKind::Skipped, "e' per il Nord America");
    assert_eq!(kind(&mut b, &directional(2, "CQ EU K1ABC FN42", Some("NA")), T0 + 100), DecisionKind::Reply);
}

#[test]
fn cq_dx_is_skipped_from_the_own_continent_but_not_from_far_away() {
    let mut b = live_brain();
    b.set_own_continent(Some("EU".into()));
    assert_eq!(kind(&mut b, &directional(1, "CQ DX DL1ABC JO31", Some("EU")), T0), DecisionKind::Skipped);
    assert_eq!(kind(&mut b, &directional(2, "CQ DX K1ABC FN42", Some("NA")), T0 + 100), DecisionKind::Reply);
    assert_eq!(kind(&mut b, &directional(3, "CQDX W9XYZ EN37", Some("NA")), T0 + 200), DecisionKind::Reply, "attivo: un solo QSO per volta, ma non esclusa per la zona");
}

#[test]
fn directions_are_ignored_when_the_rule_is_off_or_the_continent_is_unknown() {
    let mut b = live_brain();
    assert_eq!(kind(&mut b, &directional(1, "CQ NA K1ABC FN42", Some("NA")), T0), DecisionKind::Reply, "continente proprio sconosciuto");
    let rules = Rules { respect_cq_direction: false, ..Rules::default() };
    let mut b = Brain::new(rules, Limits::default(), true);
    b.set_mode(Mode::Live).unwrap();
    b.set_enabled(true, T0);
    b.set_own_continent(Some("EU".into()));
    assert_eq!(kind(&mut b, &directional(2, "CQ NA K1ABC FN42", Some("NA")), T0), DecisionKind::Reply);
}

#[test]
fn the_score_weighs_rarity_and_signal_together() {
    let rules = Rules { prefer: qso_engine::Prefer::Score, ..Rules::default() };
    let mut b = Brain::new(rules, Limits::default(), true);
    b.set_mode(Mode::Live).unwrap();
    b.set_enabled(true, T0);
    // stessa novita': vince il segnale; una novita' piu' rara batte un segnale solo un po' piu' forte
    let weak_rare = decode(1, "CQ AA1AA FN42", Some(DecodeTag::NewDxcc), -16);
    let strong_common = decode(2, "CQ BB1BB FN42", Some(DecodeTag::NewGrid), -2);
    let decisions = b.on_batch(&[weak_rare, strong_common], T0);
    let reply = decisions.iter().find(|d| d.kind == DecisionKind::Reply).expect("una risposta");
    assert_eq!(reply.call.as_deref(), Some("AA1AA"), "paese nuovo (4) +4 punti di segnale batte griglia (1) +11");
}
