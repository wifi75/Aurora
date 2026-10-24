use udp_bridge::message::{encode_free_text, encode_halt_tx, encode_reply, FREE_TEXT, HALT_TX, REPLY};
use udp_bridge::wire::Reader;
use udp_bridge::{Decode, MAGIC};

fn sample_decode() -> Decode {
    Decode {
        id: Some("JTDX".to_string()),
        is_new: true,
        time_ms: 45_015_000,
        snr: -12,
        delta_time: 0.3,
        delta_freq: 1500,
        mode: Some("~".to_string()),
        message: Some("CQ DX JA1XYZ PM95".to_string()),
        low_confidence: false,
        off_air: false,
    }
}

#[test]
fn encode_halt_tx_layout() {
    let bytes = encode_halt_tx("JTDX", 2, true);
    let mut r = Reader::new(&bytes);
    assert_eq!(r.u32(), Ok(MAGIC));
    assert_eq!(r.u32(), Ok(2));
    assert_eq!(r.u32(), Ok(HALT_TX));
    assert_eq!(r.utf8(), Ok(Some("JTDX".to_string())));
    assert_eq!(r.bool(), Ok(true));
    assert!(r.is_empty());
}

#[test]
fn encode_reply_echoes_decode_fields() {
    let d = sample_decode();
    let bytes = encode_reply("JTDX", 2, &d, 0);
    let mut r = Reader::new(&bytes);
    assert_eq!(r.u32(), Ok(MAGIC));
    assert_eq!(r.u32(), Ok(2));
    assert_eq!(r.u32(), Ok(REPLY));
    assert_eq!(r.utf8(), Ok(Some("JTDX".to_string())));
    assert_eq!(r.u32(), Ok(d.time_ms));
    assert_eq!(r.i32(), Ok(d.snr));
    assert_eq!(r.f64(), Ok(d.delta_time));
    assert_eq!(r.u32(), Ok(d.delta_freq));
    assert_eq!(r.utf8(), Ok(d.mode.clone()));
    assert_eq!(r.utf8(), Ok(d.message.clone()));
    assert_eq!(r.bool(), Ok(d.low_confidence));
    assert_eq!(r.u8(), Ok(0));
    assert!(r.is_empty());
}

#[test]
fn encode_free_text_utf8() {
    let bytes = encode_free_text("JTDX", 2, "73 da IU3CYV è qui", false);
    let mut r = Reader::new(&bytes);
    assert_eq!(r.u32(), Ok(MAGIC));
    assert_eq!(r.u32(), Ok(2));
    assert_eq!(r.u32(), Ok(FREE_TEXT));
    assert_eq!(r.utf8(), Ok(Some("JTDX".to_string())));
    assert_eq!(r.utf8(), Ok(Some("73 da IU3CYV è qui".to_string())));
    assert_eq!(r.bool(), Ok(false));
    assert!(r.is_empty());
}
