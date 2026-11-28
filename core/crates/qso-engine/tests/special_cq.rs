//! Risposta automatica ai CQ speciali (POTA, SOTA, IOTA, WWFF, QRP; richiesta #115): spenta di fabbrica, mai a chi e' gia' lavorato sulla banda.

use aurora_core::{DecodeTag, DecodedMessage};
use qso_engine::{Brain, DecisionKind, Limits, Mode, Rules};

fn decode(id: u64, text: &str, tag: Option<DecodeTag>) -> DecodedMessage {
    DecodedMessage { id, time_ms: 0, snr: -10, delta_time: 0.0, freq_hz: 1500, text: text.into(), tag, country: None, continent: None, lotw_last_upload: None, hint: false, unconfirmed: false }
}

const T0: u64 = 1_000_000;

fn brain(rules: Rules) -> Brain {
    let mut b = Brain::new(rules, Limits::default(), true);
    b.set_mode(Mode::Live).unwrap();
    b.set_enabled(true, T0);
    b
}

fn pota_only() -> Rules {
    Rules { reply_to: vec![], reply_to_special: vec!["POTA".into()], ..Rules::default() }
}

#[test]
fn off_by_default() {
    assert!(Rules::default().reply_to_special.is_empty());
    let mut b = brain(Rules::default());
    assert!(b.on_decode(&decode(1, "CQ POTA IU3ABC IO91", Some(DecodeTag::Cq)), T0).is_none(), "di fabbrica un CQ POTA qualunque non interessa");
}

#[test]
fn validation_accepts_only_known_tags() {
    assert!(pota_only().is_valid(), "solo i CQ speciali basta per salvare le regole");
    assert!(!Rules { reply_to_special: vec!["TEST".into()], ..Rules::default() }.is_valid());
    assert!(!Rules { reply_to_special: vec!["DX".into()], ..Rules::default() }.is_valid());
}

#[test]
fn replies_to_the_chosen_tag_only() {
    let mut b = brain(pota_only());
    let d = b.on_decode(&decode(1, "CQ POTA IU3ABC IO91", Some(DecodeTag::Cq)), T0).expect("decisione");
    assert_eq!(d.kind, DecisionKind::Reply);
    assert!(d.reason.contains("CQ speciale POTA"), "{}", d.reason);
    let mut b = brain(pota_only());
    assert!(b.on_decode(&decode(2, "CQ SOTA IU3ABC/P", Some(DecodeTag::Cq)), T0).is_none(), "SOTA non scelto");
    assert!(b.on_decode(&decode(3, "CQ IU3ABC IO91", Some(DecodeTag::Cq)), T0).is_none(), "un CQ comune no");
}

#[test]
fn never_to_a_station_worked_on_this_band() {
    let mut b = brain(pota_only());
    assert!(b.on_decode(&decode(1, "CQ POTA IU3ABC IO91", Some(DecodeTag::WorkedCall)), T0).is_none());
}

#[test]
fn same_guards_as_the_other_replies() {
    // un QSO gia' in corso: il secondo CQ speciale aspetta
    let mut b = brain(pota_only());
    assert_eq!(b.on_decode(&decode(1, "CQ POTA IU3ABC IO91", Some(DecodeTag::Cq)), T0).unwrap().kind, DecisionKind::Reply);
    assert_eq!(b.on_decode(&decode(2, "CQ POTA K1ABC FN42", Some(DecodeTag::Cq)), T0 + 15).unwrap().kind, DecisionKind::Skipped);
    // in prova a vuoto non risponde davvero
    let mut dry = Brain::new(pota_only(), Limits::default(), true);
    dry.set_enabled(true, T0);
    assert_eq!(dry.on_decode(&decode(3, "CQ POTA IU3ABC IO91", Some(DecodeTag::Cq)), T0).unwrap().kind, DecisionKind::WouldReply);
    // automazione spenta: niente
    let mut off = Brain::new(pota_only(), Limits::default(), true);
    assert!(off.on_decode(&decode(4, "CQ POTA IU3ABC IO91", Some(DecodeTag::Cq)), T0).is_none());
}
