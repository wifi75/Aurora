use rig_control::modes::{button_id, table_mode};

#[test]
fn q65_submodes_use_the_q65_rows_of_the_table() {
    assert_eq!(table_mode("Q65-60B"), "Q65");
    assert_eq!(table_mode("q65"), "Q65");
    assert_eq!(table_mode("JT4C"), "JT4C");
    assert_eq!(table_mode(" ft8 "), "FT8");
    assert_eq!(table_mode("jt65b"), "JT65");
    assert_eq!(table_mode("JT65C"), "JT65");
}

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
    assert_eq!(button_id("JT65B"), Some("jtButton"));
    assert_eq!(button_id("jt65c"), Some("jtButton"));
    for q in ["Q65", "q65-30a", "Q65-15C", "Q65-60B", "Q65-120D", "Q65-300E"] {
        assert_eq!(button_id(q), Some("q65Button"), "{q}");
    }
    for bad in ["", "JT4H", "JT65D", "Q65-30F", "Q65-45A", "Q65-", "Q65-A", "FT8; calc", "FT", "'FT8'", "JT; calc"] {
        assert_eq!(button_id(bad), None, "{bad}");
    }
}

#[test]
fn fst4_and_fst4w_with_or_without_period() {
    assert_eq!(table_mode("FST4-60"), "FST4");
    assert_eq!(table_mode("fst4w-120"), "FST4W");
    assert_eq!(table_mode("FST4W"), "FST4W");
    for m in ["FST4", "fst4-15", "FST4-1800"] {
        assert_eq!(button_id(m), Some("fst4Button"), "{m}");
    }
    for m in ["FST4W", "FST4W-120", "fst4w-1800"] {
        assert_eq!(button_id(m), Some("fst4wButton"), "{m}");
    }
    for bad in ["FST4-45", "FST4W-60", "FST4W-15", "FST4-", "FST4X"] {
        assert_eq!(button_id(bad), None, "{bad}");
    }
}
