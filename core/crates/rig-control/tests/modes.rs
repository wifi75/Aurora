use rig_control::modes::button_id;

#[test]
fn only_the_known_modes_have_a_button() {
    assert_eq!(button_id("FT8"), Some("ft8Button"));
    assert_eq!(button_id("ft4"), Some("ft4Button"));
    assert_eq!(button_id(" FT4 "), Some("ft4Button"));
    assert_eq!(button_id("JT"), Some("jtButton"));
    assert_eq!(button_id("jt65"), Some("jtButton"));
    assert_eq!(button_id("JT9"), Some("jtButton"));
    assert_eq!(button_id("jt4"), Some("jt4Button"));
    assert_eq!(button_id("jt4g"), Some("jt4Button"));
    for q in ["Q65", "q65-30a", "Q65-15C", "Q65-60B", "Q65-120D", "Q65-300E"] {
        assert_eq!(button_id(q), Some("q65Button"), "{q}");
    }
    for bad in ["", "JT4H", "Q65-30F", "Q65-45A", "Q65-", "Q65-A", "FT8; calc", "FT", "'FT8'", "JT; calc"] {
        assert_eq!(button_id(bad), None, "{bad}");
    }
}
