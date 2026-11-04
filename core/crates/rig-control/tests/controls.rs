use rig_control::controls::{find, Kind, CONTROLS};

#[test]
fn only_known_controls_are_reachable_and_ids_are_unique() {
    assert!(find("autoseq").is_some());
    assert!(find("enable_tx").is_some_and(|c| c.tx));
    assert!(find("clear_dx").is_some_and(|c| c.kind == Kind::Press));
    for bad in ["", "tune", "tuneButton", "AutoSeqButton", "autoseq; calc", "stop_tx"] {
        assert!(find(bad).is_none(), "{bad}");
    }
    let mut ids: Vec<_> = CONTROLS.iter().map(|c| c.id).collect();
    ids.sort_unstable();
    ids.dedup();
    assert_eq!(ids.len(), CONTROLS.len());
}

#[test]
fn the_tune_carrier_and_the_log_button_are_never_offered() {
    for c in CONTROLS {
        assert!(!c.ui_id.contains("tune"), "{}", c.ui_id);
        assert!(!c.ui_id.contains("logQSO"), "{}", c.ui_id);
    }
}

#[test]
fn only_the_controls_that_can_start_a_transmission_are_marked() {
    let tx: Vec<_> = CONTROLS.iter().filter(|c| c.tx).map(|c| c.id).collect();
    assert_eq!(tx, vec!["auto_tx", "enable_tx"]);
}
