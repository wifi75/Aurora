//! Gli encoder dei datagrammi di JTDX si rileggono con il decoder esistente (e con i comandi di ritorno).

use udp_bridge::{
    decode, decode_request, encode_clear, encode_close, encode_decode, encode_free_text, encode_halt_tx, encode_heartbeat, encode_logged_adif, encode_qso_logged, encode_replay,
    encode_reply, encode_set_tx_freq, encode_status, encode_trigger_cq, Decode, Message, QsoLoggedData, Request, Status, ADIF_HEADER, MAGIC,
};

fn sample_status() -> Status {
    Status {
        id: Some("JTDX".into()),
        dial_freq: 14_074_000,
        mode: Some("FT8".into()),
        dx_call: Some("K1ABC".into()),
        report: Some("-12".into()),
        tx_mode: Some("FT8".into()),
        tx_enabled: true,
        transmitting: false,
        decoding: true,
        rx_df: 1500,
        tx_df: 1210,
        de_call: Some("IU3ABC".into()),
        de_grid: Some("IO91wm".into()),
        dx_grid: Some("FN42".into()),
        tx_watchdog: false,
        submode: None,
        fast_mode: false,
        special_op_mode: Some(1),
        freq_tolerance: None,
        tr_period: None,
        config_name: None,
        tx_message: None,
    }
}

#[test]
fn heartbeat_is_what_jtdx_sends_and_reads_back() {
    let bytes = encode_heartbeat("JTDX", 2, 3, "2.2.159");
    assert_eq!(&bytes[..4], &MAGIC.to_be_bytes());
    let p = decode(&bytes).expect("heartbeat");
    assert_eq!(p.schema, 2);
    assert_eq!(p.message, Message::Heartbeat { id: Some("JTDX".into()), max_schema: 3, version: Some("2.2.159".into()), revision: None });
}

#[test]
fn status_roundtrips_with_the_tx_first_byte_and_nothing_after() {
    let status = sample_status();
    let bytes = encode_status(2, &status);
    let p = decode(&bytes).expect("status");
    assert_eq!(p.message, Message::Status(status));
    // un solo byte dopo `fast_mode`: niente `freq_tolerance`, `tr_period`, `config_name`, `tx_message` (JTDX non li manda)
    let mut shorter = sample_status();
    shorter.special_op_mode = None;
    assert_eq!(bytes.len(), encode_status(2, &shorter).len() + 1);
}

#[test]
fn status_with_the_wsjtx_tail_roundtrips() {
    let mut status = sample_status();
    status.special_op_mode = Some(0);
    status.freq_tolerance = Some(20);
    status.tr_period = Some(15);
    status.config_name = Some("Default".into());
    status.tx_message = Some("K1ABC IU3ABC IO91".into());
    let p = decode(&encode_status(3, &status)).expect("status");
    assert_eq!(p.schema, 3);
    assert_eq!(p.message, Message::Status(status));
}

#[test]
fn a_null_string_stays_null_and_an_empty_one_stays_empty() {
    let mut status = sample_status();
    status.dx_call = Some(String::new());
    status.dx_grid = None;
    let Message::Status(back) = decode(&encode_status(2, &status)).expect("status").message else { panic!("atteso Status") };
    assert_eq!(back.dx_call.as_deref(), Some(""));
    assert_eq!(back.dx_grid, None);
    assert_eq!(back.submode, None);
}

#[test]
fn decode_roundtrips_every_field() {
    let d = Decode {
        id: Some("JTDX".into()),
        is_new: true,
        time_ms: 45_015_000,
        snr: -12,
        delta_time: f64::from(0.3_f32),
        delta_freq: 1500,
        mode: Some("~".into()),
        message: Some("CQ K1ABC FN42".into()),
        low_confidence: true,
        off_air: false,
    };
    assert_eq!(decode(&encode_decode(2, &d)).expect("decode").message, Message::Decode(d));
}

#[test]
fn clear_and_close_carry_only_the_id() {
    assert_eq!(decode(&encode_clear("JTDX", 2)).expect("clear").message, Message::Clear { id: Some("JTDX".into()) });
    assert_eq!(decode(&encode_close("JTDX", 3)).expect("close").message, Message::Close { id: Some("JTDX".into()) });
}

#[test]
fn qso_logged_roundtrips_with_the_times_and_the_station() {
    let q = QsoLoggedData {
        // 2026-10-24 16:47:29 e 16:46:45 UTC
        time_off_ms: Some(1_792_860_449_000),
        dx_call: "K1ABC".into(),
        dx_grid: "FN42".into(),
        dial_freq: 14_074_000,
        mode: "FT8".into(),
        report_sent: "-16".into(),
        report_received: "-09".into(),
        tx_power: "50".into(),
        comments: String::new(),
        name: String::new(),
        time_on_ms: Some(1_792_860_405_000),
        operator_call: "IU3ABC".into(),
        my_call: "IU3ABC".into(),
        my_grid: "IO91wm".into(),
    };
    let p = decode(&encode_qso_logged("JTDX", 2, &q)).expect("qso");
    let Message::QsoLogged { id, dx_call, dx_grid, tx_freq, mode, report_sent, report_received, extra } = p.message else { panic!("atteso QsoLogged") };
    assert_eq!(id.as_deref(), Some("JTDX"));
    assert_eq!((dx_call.as_deref(), dx_grid.as_deref(), tx_freq, mode.as_deref()), (Some("K1ABC"), Some("FN42"), 14_074_000, Some("FT8")));
    assert_eq!((report_sent.as_deref(), report_received.as_deref()), (Some("-16"), Some("-09")));
    let extra = extra.expect("coda del pacchetto");
    assert_eq!((extra.time_off_ms, extra.time_on_ms), (q.time_off_ms, q.time_on_ms));
    assert_eq!((extra.tx_power.as_deref(), extra.comments.as_deref(), extra.name.as_deref()), (Some("50"), Some(""), Some("")));
    assert_eq!((extra.operator_call.as_deref(), extra.my_call.as_deref(), extra.my_grid.as_deref()), (Some("IU3ABC"), Some("IU3ABC"), Some("IO91wm")));
}

#[test]
fn logged_adif_carries_the_header_and_the_record() {
    let record = "<CALL:5>K1ABC <EOR>";
    let Message::LoggedAdif { id, adif } = decode(&encode_logged_adif("JTDX", 2, record)).expect("adif").message else { panic!("atteso LoggedAdif") };
    assert_eq!(id.as_deref(), Some("JTDX"));
    assert_eq!(adif, Some(format!("{ADIF_HEADER}{record}")));
    assert!(adif.is_some_and(|a| a.starts_with("\n<adif_ver:5>3.1.0\n<programid:4>JTDX\n<EOH>\n")));
}

#[test]
fn the_return_commands_are_read_back() {
    let reply_source = Decode {
        id: Some("JTDX".into()),
        is_new: true,
        time_ms: 45_015_000,
        snr: -7,
        delta_time: 0.5,
        delta_freq: 1234,
        mode: Some("~".into()),
        message: Some("CQ K1ABC FN42".into()),
        low_confidence: false,
        off_air: false,
    };
    let a = decode_request(&encode_reply("JTDX", 3, &reply_source, 0)).expect("reply");
    assert_eq!((a.schema, a.id.as_deref()), (3, Some("JTDX")));
    assert_eq!(a.request, Some(Request::Reply { decode: reply_source, modifiers: 0 }));
    assert_eq!(decode_request(&encode_halt_tx("JTDX", 2, true)).expect("halt").request, Some(Request::HaltTx { auto_tx_only: true }));
    assert_eq!(decode_request(&encode_free_text("JTDX", 2, "TNX 73", true)).expect("free").request, Some(Request::FreeText { text: "TNX 73".into(), send: true }));
    assert_eq!(decode_request(&encode_set_tx_freq("JTDX", 2, 1800)).expect("freq").request, Some(Request::SetTxFreq { hz: 1800 }));
    assert_eq!(decode_request(&encode_trigger_cq("JTDX", 2, "DX", true)).expect("cq").request, Some(Request::TriggerCq { direction: "DX".into(), send: true }));
    assert_eq!(decode_request(&encode_replay("JTDX", 2)).expect("replay").request, Some(Request::Replay));
    // il Heartbeat di risposta del client non e' un comando, ma porta lo schema
    let hb = decode_request(&encode_heartbeat("JTDX", 3, 3, "GridTracker")).expect("heartbeat");
    assert_eq!((hb.schema, hb.request), (3, None));
}
