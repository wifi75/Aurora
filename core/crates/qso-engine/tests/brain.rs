//! Le salvaguardie dell'automazione, una per una. Il tempo lo passa il test.

use aurora_core::{DecodeTag, DecodedMessage};
use qso_engine::{Brain, DecisionKind, Limits, Mode, Rules};

fn decode(id: u64, text: &str, tag: Option<DecodeTag>, snr: i32) -> DecodedMessage {
    DecodedMessage { id, time_ms: 0, snr, delta_time: 0.0, freq_hz: 1500, text: text.into(), tag, country: None }
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
