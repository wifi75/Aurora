//! Pacchetti reali catturati da JTDX 2.2.159 (schema 2) con `udp_capture`, il 2026-10-24.
//! Nominativi e locatori sono sostituiti da valori di fantasia della stessa lunghezza; la struttura dei byte e' quella reale.

use std::{fs, path::PathBuf};

use udp_bridge::{decode, Message};

fn fixture(name: &str) -> Vec<u8> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures").join(name);
    fs::read(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

#[test]
fn real_heartbeat_has_max_schema_3_version_and_no_revision() {
    let p = decode(&fixture("heartbeat.bin")).unwrap();
    assert_eq!(p.schema, 2, "JTDX parla lo schema 2");
    match p.message {
        Message::Heartbeat { id, max_schema, version, revision } => {
            assert_eq!(id.as_deref(), Some("JTDX"));
            assert_eq!(max_schema, 3);
            assert_eq!(version.as_deref(), Some("2.2.159"));
            assert_eq!(revision, None);
        }
        other => panic!("atteso Heartbeat, trovato {other:?}"),
    }
}

#[test]
fn real_clear_is_just_the_header() {
    let p = decode(&fixture("clear.bin")).unwrap();
    assert!(matches!(p.message, Message::Clear { id } if id.as_deref() == Some("JTDX")));
}

#[test]
fn real_status_without_cat_has_zero_frequency() {
    match decode(&fixture("status-no-cat.bin")).unwrap().message {
        Message::Status(s) => {
            assert_eq!(s.dial_freq, 0);
            assert_eq!(s.mode.as_deref(), Some("FT8"));
            assert_eq!((s.rx_df, s.tx_df), (1500, 1500));
            assert!(!s.tx_enabled && !s.transmitting && !s.decoding);
        }
        other => panic!("atteso Status, trovato {other:?}"),
    }
}

#[test]
fn real_status_while_decoding_on_15m() {
    match decode(&fixture("status-decoding.bin")).unwrap().message {
        Message::Status(s) => {
            assert_eq!(s.dial_freq, 21_074_000);
            assert_eq!(s.mode.as_deref(), Some("FT8"));
            assert_eq!(s.de_call.as_deref(), Some("ZZ9ZZZ"));
            assert_eq!(s.de_grid.as_deref(), Some("FN42aa"));
            assert!(s.decoding);
            // Byte finale dello Status di JTDX: 0 solo nel primo pacchetto all'avvio, poi sempre 1 (28 su 29).
            assert_eq!(s.special_op_mode, Some(1), "il byte finale di JTDX dopo fast_mode");
            assert_eq!((s.freq_tolerance, s.tr_period, s.tx_message), (None, None, None));
        }
        other => panic!("atteso Status, trovato {other:?}"),
    }
}

#[test]
fn real_decodes_use_mode_tilde_and_one_decimal_delta_time() {
    for (name, text) in [("decode-cq.bin", "CQ AA1AA FN42"), ("decode-rr73.bin", "BB2BB AA1AA RR73")] {
        match decode(&fixture(name)).unwrap().message {
            Message::Decode(d) => {
                assert!(d.is_new);
                assert_eq!(d.mode.as_deref(), Some("~"), "{name}: FT8 e' indicato con ~");
                assert_eq!(d.message.as_deref(), Some(text));
                assert!(!d.low_confidence && !d.off_air);
                assert!((0..86_400_000).contains(&d.time_ms));
                let tenths = f64::from(d.delta_time as f32) * 10.0;
                assert!((tenths - tenths.round()).abs() < 1e-3, "{name}: DT {} non a un decimale", d.delta_time);
            }
            other => panic!("atteso Decode, trovato {other:?}"),
        }
    }
}
