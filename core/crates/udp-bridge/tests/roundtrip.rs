use udp_bridge::wire::Writer;
use udp_bridge::{decode, Error, Message, MAGIC};

fn header(kind: u32) -> Writer {
    let mut w = Writer::new();
    w.u32(MAGIC);
    w.u32(2);
    w.u32(kind);
    w.utf8(Some("JTDX"));
    w
}

fn decode_packet(message: Option<&str>) -> Vec<u8> {
    let mut w = header(2);
    w.bool(true); // new
    w.u32(45_015_000); // 12:30:15 in ms dalla mezzanotte
    w.i32(-12);
    w.f64(0.3);
    w.u32(1500);
    w.utf8(Some("~"));
    w.utf8(message);
    w.bool(false); // low confidence
    w.bool(false); // off air
    w.into_bytes()
}

/// Pacchetto di Status; restituisce anche la lunghezza della parte obbligatoria.
fn status_packet() -> (Vec<u8>, usize) {
    let mut w = header(1);
    w.u64(14_074_000);
    w.utf8(Some("FT8"));
    w.utf8(Some("JA1XYZ"));
    w.utf8(Some("-08"));
    w.utf8(Some("FT8"));
    w.bool(true); // tx enabled
    w.bool(false); // transmitting
    w.bool(true); // decoding
    w.u32(1500);
    w.u32(1200);
    w.utf8(Some("IK1ABC"));
    w.utf8(Some("JN45"));
    w.utf8(Some("PM95"));
    w.bool(false); // tx watchdog
    w.utf8(Some(""));
    w.bool(false); // fast mode
    let mandatory = w.len();
    w.u8(0); // special op mode
    w.u32(20);
    w.u32(15);
    w.utf8(Some("Default"));
    w.utf8(Some("CQ IK1ABC JN45"));
    (w.into_bytes(), mandatory)
}

#[test]
fn decode_heartbeat() {
    let mut w = header(0);
    w.u32(3);
    w.utf8(Some("159"));
    w.utf8(Some("r1"));
    let p = decode(&w.into_bytes()).unwrap();
    assert_eq!(p.schema, 2);
    match p.message {
        Message::Heartbeat { id, max_schema, version, revision } => {
            assert_eq!(id.as_deref(), Some("JTDX"));
            assert_eq!(max_schema, 3);
            assert_eq!(version.as_deref(), Some("159"));
            assert_eq!(revision.as_deref(), Some("r1"));
        }
        other => panic!("atteso Heartbeat, trovato {other:?}"),
    }
}

#[test]
fn decode_decode_message() {
    let p = decode(&decode_packet(Some("CQ DX IK1ABC JN45"))).unwrap();
    match p.message {
        Message::Decode(d) => {
            assert!(d.is_new);
            assert_eq!(d.time_ms, 45_015_000);
            assert_eq!(d.snr, -12);
            assert_eq!(d.delta_time, 0.3);
            assert_eq!(d.delta_freq, 1500);
            assert_eq!(d.mode.as_deref(), Some("~"));
            assert_eq!(d.message.as_deref(), Some("CQ DX IK1ABC JN45"));
            assert!(!d.low_confidence && !d.off_air);
        }
        other => panic!("atteso Decode, trovato {other:?}"),
    }
}

#[test]
fn decode_status_all_fields() {
    let (bytes, _) = status_packet();
    match decode(&bytes).unwrap().message {
        Message::Status(s) => {
            assert_eq!(s.dial_freq, 14_074_000);
            assert_eq!(s.mode.as_deref(), Some("FT8"));
            assert_eq!(s.dx_call.as_deref(), Some("JA1XYZ"));
            assert!(s.tx_enabled && !s.transmitting && s.decoding);
            assert_eq!((s.rx_df, s.tx_df), (1500, 1200));
            assert_eq!(s.de_call.as_deref(), Some("IK1ABC"));
            assert_eq!(s.de_grid.as_deref(), Some("JN45"));
            assert_eq!(s.dx_grid.as_deref(), Some("PM95"));
            assert_eq!(s.submode.as_deref(), Some(""));
            assert_eq!(s.tr_period, Some(15));
            assert_eq!(s.tx_message.as_deref(), Some("CQ IK1ABC JN45"));
        }
        other => panic!("atteso Status, trovato {other:?}"),
    }
}

#[test]
fn status_without_trailing_fields_is_accepted() {
    let (bytes, mandatory) = status_packet();
    match decode(&bytes[..mandatory]).unwrap().message {
        Message::Status(s) => {
            assert_eq!((s.special_op_mode, s.tr_period, s.tx_message), (None, None, None))
        }
        other => panic!("atteso Status, trovato {other:?}"),
    }
}

#[test]
fn bad_magic_is_error() {
    let mut bytes = decode_packet(Some("CQ"));
    bytes[0] = 0;
    assert!(matches!(decode(&bytes), Err(Error::BadMagic(_))));
}

#[test]
fn unknown_kind_is_unknown_not_error() {
    let mut w = header(99);
    w.u32(7);
    let bytes = w.into_bytes();
    match decode(&bytes).unwrap().message {
        Message::Unknown { kind, id, raw } => {
            assert_eq!(kind, 99);
            assert_eq!(id.as_deref(), Some("JTDX"));
            assert_eq!(raw, bytes);
        }
        other => panic!("atteso Unknown, trovato {other:?}"),
    }
}

#[test]
fn truncated_status_never_panics_and_cut_before_mandatory_end_is_error() {
    let (bytes, mandatory) = status_packet();
    for cut in 0..bytes.len() {
        let result = decode(&bytes[..cut]);
        if cut < mandatory {
            assert!(result.is_err(), "taglio a {cut} byte doveva fallire");
        }
    }
}

#[test]
fn null_and_empty_message_strings_stay_distinct() {
    let null = decode(&decode_packet(None)).unwrap();
    let empty = decode(&decode_packet(Some(""))).unwrap();
    match (null.message, empty.message) {
        (Message::Decode(a), Message::Decode(b)) => {
            assert_eq!(a.message, None);
            assert_eq!(b.message, Some(String::new()));
        }
        _ => panic!("attesi due Decode"),
    }
}

#[test]
fn invalid_utf8_in_message_is_bad_utf8() {
    let mut w = header(2);
    w.bool(true);
    w.u32(0);
    w.i32(0);
    w.f64(0.0);
    w.u32(0);
    w.utf8(Some("~"));
    w.u32(2);
    w.raw(&[0xFF, 0xFE]);
    w.bool(false);
    w.bool(false);
    assert_eq!(decode(&w.into_bytes()).unwrap_err(), Error::BadUtf8);
}

#[test]
fn huge_string_length_in_small_packet_is_too_long() {
    let mut w = header(2);
    w.bool(true);
    w.u32(0);
    w.i32(0);
    w.f64(0.0);
    w.u32(0);
    w.u32(0xFFFF_FFF0);
    assert_eq!(decode(&w.into_bytes()).unwrap_err(), Error::TooLong(0xFFFF_FFF0));
}

#[test]
fn qso_logged_reads_call_grid_freq_and_mode() {
    let mut w = header(5);
    w.i64(2_461_000); // QDateTime: giorno giuliano
    w.u32(45_015_000); // ms dalla mezzanotte
    w.u8(1); // UTC
    w.utf8(Some("JA1XYZ"));
    w.utf8(Some("PM95"));
    w.u64(14_074_000);
    w.utf8(Some("FT8"));
    w.utf8(Some("-08"));
    w.utf8(Some("-12"));
    match decode(&w.into_bytes()).unwrap().message {
        Message::QsoLogged { dx_call, dx_grid, tx_freq, mode, report_sent, report_received, .. } => {
            assert_eq!(dx_call.as_deref(), Some("JA1XYZ"));
            assert_eq!(dx_grid.as_deref(), Some("PM95"));
            assert_eq!(tx_freq, 14_074_000);
            assert_eq!(mode.as_deref(), Some("FT8"));
            assert_eq!(report_sent.as_deref(), Some("-08"));
            assert_eq!(report_received.as_deref(), Some("-12"));
        }
        other => panic!("atteso QsoLogged, trovato {other:?}"),
    }
}

/// Reverse di JTDX 159 (MessageClient::impl::heartbeat): schema massimo + UNA sola stringa (la versione).
#[test]
fn heartbeat_without_revision_is_accepted() {
    let mut w = header(0);
    w.u32(3);
    w.utf8(Some("159"));
    match decode(&w.into_bytes()).unwrap().message {
        Message::Heartbeat { max_schema, version, revision, .. } => {
            assert_eq!(max_schema, 3);
            assert_eq!(version.as_deref(), Some("159"));
            assert_eq!(revision, None);
        }
        other => panic!("atteso Heartbeat, trovato {other:?}"),
    }
}
