use rig_control::modes::{attenuation_read_script, attenuation_set_script, button_id, mode_script, rx_script};

#[test]
fn only_the_modes_jtdx_can_switch_to_are_known() {
    assert_eq!(button_id("FT8"), Some("ft8Button"));
    assert_eq!(button_id("ft4"), Some("ft4Button"));
    assert_eq!(button_id(" FT4 "), Some("ft4Button"));
    assert_eq!(button_id("JT"), Some("jtButton"));
    for bad in ["", "JT65", "JT9", "FT8; calc", "FT", "'FT8'", "JT; calc"] {
        assert_eq!(button_id(bad), None, "{bad}");
    }
}

#[test]
fn the_script_carries_only_a_known_button_id_and_presses_it_like_a_click() {
    let s = mode_script("ft4Button");
    assert!(s.contains("*.ft4Button"), "{s}");
    assert!(s.contains("jtdx"), "{s}");
    // Toggle() accende solo la casella senza cambiare modo: serve il clic (Invoke), che fa scattare il segnale di JTDX
    assert!(s.contains("InvokePattern"), "{s}");
    assert!(!s.contains(".Toggle()"), "{s}");
}

#[test]
fn the_script_presses_the_other_button_too_when_both_look_on_and_never_twice() {
    let s = mode_script("ft4Button");
    // si preme solo se il pulsante e' spento o se anche l'altro risulta acceso (stato incoerente)
    assert!(s.contains("(-not $on) -or $otherOn"), "{s}");
    assert!(s.contains("*.ft8Button"), "{s}");
    assert!(s.contains("*.jtButton"), "il pulsante JT e' uno degli altri: {s}");
    assert_eq!(s.matches(".Invoke()").count(), 1, "{s}");
    let jt = mode_script("jtButton");
    assert!(jt.contains("*.ft8Button") && jt.contains("*.ft4Button"), "{jt}");
    assert!(!jt.contains("-like '*.jtButton') -or ($_.Current.AutomationId -like '*.jtButton'"), "{jt}");
}

#[test]
fn the_rx_script_sets_a_number_on_the_rx_spin_box() {
    let s = rx_script(1234);
    assert!(s.contains("*.RxFreqSpinBox"), "{s}");
    assert!(s.contains("SetValue(1234)"), "{s}");
}

#[test]
fn the_output_level_scripts_use_the_slider_of_jtdx() {
    let read = attenuation_read_script();
    assert!(read.contains("*.outAttenuation"), "{read}");
    assert!(read.contains("Current.Value"), "{read}");
    let set = attenuation_set_script(125);
    assert!(set.contains("*.outAttenuation"), "{set}");
    assert!(set.contains("SetValue(125)"), "{set}");
}
