//! Le regole nuove dell'automazione: nuovo modo, paesi e continenti, limite orario, fascia oraria, CQ da solo. Il tempo lo passa il test.

use std::sync::Arc;

use aurora_core::{DecodeTag, DecodedMessage, StationStatus};
use qso_engine::{Brain, DecisionKind, Limits, Mode, Rules};

/// Mezzanotte UTC di un giorno qualunque: i minuti della fascia oraria si contano da qui.
const MIDNIGHT: u64 = 20_000 * 86_400;
const T0: u64 = MIDNIGHT + 12 * 3600;

fn decode(id: u64, text: &str, tag: Option<DecodeTag>) -> DecodedMessage {
    DecodedMessage { id, time_ms: 0, snr: -10, delta_time: 0.0, freq_hz: 1500, text: text.into(), tag, country: None, continent: None, lotw_last_upload: None, hint: false, unconfirmed: false }
}

fn cq(id: u64, call: &str) -> DecodedMessage {
    decode(id, &format!("CQ {call} FN42"), Some(DecodeTag::NewDxcc))
}

fn from(id: u64, call: &str, country: Option<&str>, continent: Option<&str>) -> DecodedMessage {
    DecodedMessage { country: country.map(str::to_string), continent: continent.map(str::to_string), ..cq(id, call) }
}

fn brain(rules: Rules, live: bool) -> Brain {
    let mut b = Brain::new(rules, Limits { max_consecutive_cycles: 99, max_per_day: 999, ..Limits::default() }, true);
    if live {
        b.set_mode(Mode::Live).unwrap();
    }
    b.set_enabled(true, T0);
    b
}

fn kind(b: &mut Brain, d: &DecodedMessage, now: u64) -> Option<DecisionKind> {
    b.on_decode(d, now).map(|d| d.kind)
}

fn status(tx_enabled: bool) -> StationStatus {
    StationStatus { dial_hz: 14_074_000, mode: "FT8".into(), tx_enabled, ..StationStatus::default() }
}

#[test]
fn the_new_rules_start_prudent() {
    let r = Rules::default();
    assert!(!r.reply_to_new_mode);
    assert_eq!(r.max_qso_per_hour, 0, "senza limite orario");
    assert!(!r.window_enabled, "fascia oraria spenta");
    assert_eq!(r.auto_cq_after_min, 0, "CQ da solo spento");
    assert!(r.excluded_countries.is_empty() && r.continents.is_empty());
}

#[test]
fn rules_saved_by_an_older_aurora_still_load_with_the_new_defaults() {
    let old = r#"{"reply_to":["new_dxcc"],"min_snr":-12,"excluded_calls":[],"max_attempts_per_call":2}"#;
    let r: Rules = serde_json::from_str(old).unwrap();
    assert!(r.is_valid());
    assert_eq!((r.max_qso_per_hour, r.window_enabled, r.auto_cq_after_min), (0, false, 0));
    let full = Rules { max_qso_per_hour: 8, window_enabled: true, window_from_min: 1320, window_to_min: 360, excluded_countries: vec!["Atlantis".into()], continents: vec!["EU".into()], auto_cq_after_min: 5, reply_to_new_mode: true, ..Rules::default() };
    let back: Rules = serde_json::from_str(&serde_json::to_string(&full).unwrap()).unwrap();
    assert_eq!(back, full);
}

#[test]
fn the_new_rules_are_validated() {
    assert!(!Rules { max_qso_per_hour: 500, ..Rules::default() }.is_valid());
    assert!(!Rules { window_from_min: 1440, ..Rules::default() }.is_valid());
    assert!(!Rules { continents: vec!["XX".into()], ..Rules::default() }.is_valid());
    assert!(Rules { continents: vec!["eu".into(), "NA".into()], ..Rules::default() }.is_valid());
    assert!(!Rules { auto_cq_after_min: 1, ..Rules::default() }.is_valid(), "CQ a raffica non ammesso");
    assert!(Rules { auto_cq_after_min: 2, ..Rules::default() }.is_valid());
    assert!(!Rules { excluded_countries: vec![String::new()], ..Rules::default() }.is_valid());
    assert!(Rules { reply_to: vec![], reply_to_new_mode: true, ..Rules::default() }.is_valid(), "basta il nuovo modo");
}

#[test]
fn an_excluded_country_is_skipped_with_its_reason() {
    let mut b = brain(Rules { excluded_countries: vec!["atlantis".into()], ..Rules::default() }, true);
    let d = b.on_decode(&from(1, "AT1AA", Some("Atlantis"), Some("EU")), T0).unwrap();
    assert_eq!(d.kind, DecisionKind::Skipped);
    assert_eq!(d.reason, "AT1AA: Atlantis e' tra i paesi esclusi");
    assert_eq!(kind(&mut b, &from(2, "LM1XY", Some("Lemuria"), Some("EU")), T0), Some(DecisionKind::Reply));
}

#[test]
fn only_the_chosen_continents_are_answered() {
    let mut b = brain(Rules { continents: vec!["NA".into(), "SA".into()], ..Rules::default() }, true);
    let d = b.on_decode(&from(1, "AT1AA", None, Some("EU")), T0).unwrap();
    assert_eq!((d.kind, d.reason.as_str()), (DecisionKind::Skipped, "AT1AA: continente EU non tra quelli scelti"));
    let d = b.on_decode(&from(2, "AT2AA", None, None), T0).unwrap();
    assert_eq!((d.kind, d.reason.as_str()), (DecisionKind::Skipped, "AT2AA: continente sconosciuto"));
    assert_eq!(kind(&mut b, &from(3, "K1ABC", None, Some("NA")), T0), Some(DecisionKind::Reply));
}

#[test]
fn a_country_new_in_this_mode_is_answered_only_with_the_rule_on() {
    let rules = Rules { reply_to: vec![DecodeTag::NewDxcc], reply_to_new_mode: true, ..Rules::default() };
    let mut b = brain(rules, true);
    b.on_status(&status(false));
    // il log dice: AT1AA mai fatto in FT8; il messaggio non ha etichette di novita'
    b.set_new_mode_matcher(Arc::new(|call, mode| call == "AT1AA" && mode == Some("FT8")));
    let d = b.on_decode(&decode(1, "CQ AT1AA FN42", Some(DecodeTag::Cq)), T0).unwrap();
    assert_eq!(d.kind, DecisionKind::Reply);
    assert_eq!(d.reason, "Rispondo a AT1AA: paese nuovo in questo modo");
    let mut off = brain(Rules::default(), true);
    off.on_status(&status(false));
    off.set_new_mode_matcher(Arc::new(|_, _| true));
    assert_eq!(kind(&mut off, &decode(2, "CQ AT1AA FN42", Some(DecodeTag::Cq)), T0), None, "regola spenta: non interessa");
}

#[test]
fn the_hourly_limit_counts_logged_qsos_and_frees_after_an_hour() {
    let mut b = brain(Rules { max_qso_per_hour: 2, ..Rules::default() }, true);
    for (i, call) in ["AT1AA", "AT2AA"].iter().enumerate() {
        let at = T0 + i as u64 * 120;
        assert_eq!(kind(&mut b, &cq(i as u64, call), at), Some(DecisionKind::Reply));
        b.on_qso_logged_at(call, at + 60);
    }
    assert_eq!(b.state().qsos_today, 2);
    assert_eq!(b.state().qsos_last_hour, 2);
    let d = b.on_decode(&cq(10, "AT3AA"), T0 + 600).unwrap();
    assert_eq!((d.kind, d.reason.as_str()), (DecisionKind::Blocked, "Raggiunto il limite di 2 QSO all'ora"));
    let logged = b.state().log.len();
    b.on_decode(&cq(11, "AT4AA"), T0 + 630);
    assert_eq!(b.state().log.len(), logged, "lo stesso blocco non si ripete a ogni ciclo");
    // un'ora dopo il primo QSO il conto scende
    assert_eq!(kind(&mut b, &cq(12, "AT3AA"), T0 + 3600 + 61), Some(DecisionKind::Reply));
}

#[test]
fn a_qso_not_started_by_the_automation_does_not_count() {
    let mut b = brain(Rules { max_qso_per_hour: 1, ..Rules::default() }, true);
    b.on_qso_logged_at("ZZ9ZZ", T0);
    assert_eq!((b.state().qsos_today, b.state().qsos_last_hour), (0, 0));
}

#[test]
fn the_hourly_limit_also_applies_in_dry_run() {
    let mut b = brain(Rules { max_qso_per_hour: 1, ..Rules::default() }, false);
    assert_eq!(kind(&mut b, &cq(1, "AT1AA"), T0), Some(DecisionKind::WouldReply));
    assert_eq!(b.state().would_replies_today, 1);
    assert_eq!(kind(&mut b, &cq(2, "AT2AA"), T0 + 120), Some(DecisionKind::Blocked), "il registro dice che si sarebbe fermata");
}

#[test]
fn outside_the_time_window_nothing_is_answered_and_the_log_says_it_once() {
    // dalle 08:00 alle 10:00 ora locale; il PC e' a UTC+1
    let rules = Rules { window_enabled: true, window_from_min: 8 * 60, window_to_min: 10 * 60, ..Rules::default() };
    let mut b = Brain::new(rules, Limits::default(), true);
    b.set_utc_offset(3600);
    b.set_enabled(true, MIDNIGHT + 6 * 3600); // 07:00 locali
    assert!(!b.state().in_window);
    assert_eq!(b.state().log.last().unwrap().kind, DecisionKind::Paused);
    assert_eq!(b.on_decode(&cq(1, "AT1AA"), MIDNIGHT + 6 * 3600 + 30), None, "fuori fascia non si valuta nulla");
    let logged = b.state().log.len();
    b.tick(MIDNIGHT + 6 * 3600 + 60, 1);
    assert_eq!(b.state().log.len(), logged, "una volta sola");
    // 08:00 locali
    b.tick(MIDNIGHT + 7 * 3600, 1);
    assert!(b.state().in_window);
    assert_eq!(b.state().log.last().unwrap().kind, DecisionKind::Info);
    assert_eq!(kind(&mut b, &cq(2, "AT1AA"), MIDNIGHT + 7 * 3600 + 15), Some(DecisionKind::WouldReply));
}

#[test]
fn a_window_across_midnight_works() {
    // dalle 22:00 alle 06:00, PC a UTC
    let rules = Rules { window_enabled: true, window_from_min: 22 * 60, window_to_min: 6 * 60, ..Rules::default() };
    let mut b = Brain::new(rules, Limits::default(), true);
    b.set_enabled(true, MIDNIGHT + 23 * 3600);
    assert!(b.state().in_window, "23:00 e' dentro");
    b.tick(MIDNIGHT + 86_400 + 5 * 3600, 1);
    assert!(b.state().in_window, "05:00 e' dentro");
    b.tick(MIDNIGHT + 86_400 + 12 * 3600, 1);
    assert!(!b.state().in_window, "12:00 e' fuori");
}

#[test]
fn the_auto_cq_is_off_by_default() {
    let mut b = brain(Rules::default(), true);
    b.on_status(&status(false));
    assert!(b.tick(T0 + 3 * 3600, 1).is_none());
}

#[test]
fn the_auto_cq_calls_after_the_quiet_minutes_only_with_the_radio_idle() {
    let mut b = brain(Rules { auto_cq_after_min: 5, ..Rules::default() }, true);
    b.on_status(&status(false));
    assert!(b.tick(T0 + 299, 1).is_none(), "non ancora");
    let d = b.tick(T0 + 300, 1).expect("chiama CQ");
    assert_eq!(d.kind, DecisionKind::CallCq);
    assert_eq!(d.reason, "Nessun CQ a cui rispondere da 5 min: chiamo CQ");
    assert_eq!(b.state().consecutive, 1, "conta come una risposta di fila senza QSO");
    // la radio trasmette (il CQ e' in onda): niente altri CQ
    b.on_status(&status(true));
    assert!(b.tick(T0 + 600, 1).is_none());
}

#[test]
fn a_cq_worth_answering_resets_the_quiet_time() {
    let mut b = brain(Rules { auto_cq_after_min: 5, ..Rules::default() }, false);
    b.on_status(&status(false));
    b.on_decode(&cq(1, "AT1AA"), T0 + 200);
    assert!(b.tick(T0 + 300, 1).is_none(), "c'era qualcuno a cui rispondere 100 s fa");
    let d = b.tick(T0 + 500, 1).expect("in prova a vuoto lo dice");
    assert_eq!(d.kind, DecisionKind::WouldCallCq);
    assert!(!b.take_halt_request(), "in prova a vuoto non c'e' nulla da fermare");
}

#[test]
fn the_auto_cq_respects_the_safety_caps_and_stops_its_cq_when_switched_off() {
    let mut b = Brain::new(Rules { auto_cq_after_min: 2, ..Rules::default() }, Limits { max_consecutive_cycles: 1, ..Limits::default() }, true);
    b.set_mode(Mode::Live).unwrap();
    b.set_enabled(true, T0);
    b.on_status(&status(false));
    assert_eq!(b.tick(T0 + 120, 1).map(|d| d.kind), Some(DecisionKind::CallCq));
    // il CQ non ha portato QSO e la radio e' di nuovo ferma: al tetto si spegne e chiede di fermare la trasmissione
    let d = b.tick(T0 + 240, 1).expect("tetto");
    assert_eq!(d.kind, DecisionKind::Blocked);
    assert!(!b.state().enabled);
    assert!(b.take_halt_request());
    assert!(!b.take_halt_request(), "una volta sola");
}

#[test]
fn switching_off_by_hand_stops_the_automatic_cq() {
    let mut b = brain(Rules { auto_cq_after_min: 2, ..Rules::default() }, true);
    b.on_status(&status(false));
    assert_eq!(b.tick(T0 + 120, 1).map(|d| d.kind), Some(DecisionKind::CallCq));
    b.set_enabled(false, T0 + 130);
    assert!(b.take_halt_request());
}

#[test]
fn a_qso_from_our_cq_counts_for_the_day_and_the_hour() {
    let mut b = brain(Rules { auto_cq_after_min: 2, max_qso_per_hour: 5, ..Rules::default() }, true);
    b.on_status(&status(false));
    assert_eq!(b.tick(T0 + 120, 1).map(|d| d.kind), Some(DecisionKind::CallCq));
    b.on_status(&status(true));
    b.on_qso_logged_at("K1ABC", T0 + 240);
    assert_eq!((b.state().qsos_today, b.state().qsos_last_hour, b.state().consecutive), (1, 1, 0));
}

#[test]
fn the_auto_cq_waits_for_the_time_window() {
    let rules = Rules { auto_cq_after_min: 2, window_enabled: true, window_from_min: 8 * 60, window_to_min: 9 * 60, ..Rules::default() };
    let mut b = brain(rules, true); // T0 = 12:00 UTC, fuori fascia
    b.on_status(&status(false));
    assert!(b.tick(T0 + 600, 1).is_none());
}
