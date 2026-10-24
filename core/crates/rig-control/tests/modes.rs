use rig_control::modes::{attenuation_read_script, attenuation_set_script, button_id, mode_script, rx_script};

#[test]
fn only_the_modes_jtdx_can_switch_to_are_known() {
    assert_eq!(button_id("FT8"), Some("ft8Button"));
    assert_eq!(button_id("ft4"), Some("ft4Button"));
    assert_eq!(button_id(" FT4 "), Some("ft4Button"));
    for bad in ["", "JT65", "FT8; calc", "FT", "'FT8'"] {
        assert_eq!(button_id(bad), None, "{bad}");
    }
}

#[test]
fn the_script_carries_only_a_known_button_id_and_toggles_it_only_when_off() {
    let s = mode_script("ft4Button");
    assert!(s.contains("*.ft4Button"), "{s}");
    assert!(s.contains("ToggleState -ne 'On'"), "{s}");
    assert!(s.contains("jtdx"), "{s}");
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
