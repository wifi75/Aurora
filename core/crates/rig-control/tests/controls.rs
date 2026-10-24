use rig_control::controls::{find, read_script, set_script, Kind, CONTROLS};

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

#[test]
fn the_read_script_names_every_toggle_and_no_button() {
    let s = read_script();
    for c in CONTROLS {
        assert_eq!(s.contains(c.ui_id), c.kind == Kind::Toggle, "{}", c.id);
    }
    assert!(s.contains("ToggleState"), "{s}");
    assert!(s.contains("jtdx"), "{s}");
}

#[test]
fn a_toggle_is_pressed_only_when_it_differs_from_the_wanted_state() {
    let s = set_script(find("autoseq").unwrap(), Some(true));
    assert!(s.contains("*.AutoSeqButton"), "{s}");
    assert!(s.contains("InvokePattern"), "{s}");
    assert!(s.contains("-ne $true") || s.contains("-ne 1"), "{s}");
    let off = set_script(find("autoseq").unwrap(), Some(false));
    assert!(off.contains("-ne $false") || off.contains("-ne 0"), "{off}");
}

#[test]
fn a_plain_button_is_always_pressed() {
    let s = set_script(find("clear_dx").unwrap(), None);
    assert!(s.contains("*.ClearDxButton"), "{s}");
    assert!(s.contains("Invoke()"), "{s}");
    assert!(!s.contains("ToggleState"), "{s}");
}
