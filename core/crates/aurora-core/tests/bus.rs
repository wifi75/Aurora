use aurora_core::{
    Bus, Command, DecodeTag, DecodedMessage, Event, SpectrumRow, StationStatus,
};

fn decode_event() -> Event {
    Event::Decode(DecodedMessage {
        id: 7,
        time_ms: 45_015_000,
        snr: -12,
        delta_time: 0.3,
        freq_hz: 1500,
        text: "CQ DX JA1XYZ PM95".to_string(),
        tag: Some(DecodeTag::NewDxcc),
        country: None,
        continent: None,
    })
}

#[tokio::test]
async fn two_subscribers_both_receive_event() {
    let bus = Bus::new(16);
    let mut a = bus.subscribe_events();
    let mut b = bus.subscribe_events();
    bus.events().send(Event::TxHalted).unwrap();
    assert_eq!(a.recv().await.unwrap(), Event::TxHalted);
    assert_eq!(b.recv().await.unwrap(), Event::TxHalted);
}

#[tokio::test]
async fn command_channel_delivers_in_order() {
    let bus = Bus::new(16);
    let mut rx = bus.take_command_rx().unwrap();
    bus.commands().send(Command::HaltTx).await.unwrap();
    bus.commands().send(Command::Reply { decode_id: 3 }).await.unwrap();
    assert_eq!(rx.recv().await, Some(Command::HaltTx));
    assert_eq!(rx.recv().await, Some(Command::Reply { decode_id: 3 }));
}

#[test]
fn take_command_rx_works_only_once() {
    let bus = Bus::new(4);
    assert!(bus.take_command_rx().is_some());
    assert!(bus.take_command_rx().is_none());
}

#[test]
fn spectrum_validate_rejects_empty_and_oversized_and_inverted_range() {
    let ok = SpectrumRow { seq: 1, f_min_hz: 0, f_max_hz: 3000, bins: vec![0; 1024] };
    assert!(ok.validate().is_ok());
    assert!(SpectrumRow { bins: vec![], ..ok.clone() }.validate().is_err());
    assert!(SpectrumRow { bins: vec![0; 4097], ..ok.clone() }.validate().is_err());
    assert!(SpectrumRow { f_min_hz: 3000, f_max_hz: 3000, ..ok.clone() }.validate().is_err());
    assert!(SpectrumRow { bins: vec![0; 4096], ..ok }.validate().is_ok());
}

#[test]
fn event_serializes_with_type_tag() {
    assert_eq!(serde_json::to_string(&Event::TxHalted).unwrap(), r#"{"type":"tx_halted"}"#);
    let v: serde_json::Value = serde_json::to_value(decode_event()).unwrap();
    assert_eq!(v["type"], "decode");
    assert_eq!(v["id"], 7);
    assert_eq!(v["tag"], "new_dxcc");
    let status = Event::Status(StationStatus {
        dial_hz: 14_074_000,
        mode: "FT8".into(),
        tx_enabled: true,
        transmitting: false,
        decoding: true,
        de_call: "AA1AA".into(),
        de_grid: "FN42".into(),
        ..StationStatus::default()
    });
    let back: Event = serde_json::from_str(&serde_json::to_string(&status).unwrap()).unwrap();
    assert_eq!(back, status);
}

#[test]
fn status_carries_the_dx_and_transmit_details_and_old_json_still_parses() {
    let status = Event::Status(StationStatus {
        dial_hz: 21_074_000,
        mode: "FT8".into(),
        de_call: "AA1AA".into(),
        de_grid: "FN42".into(),
        dx_call: Some("BB2BB".into()),
        dx_grid: Some("KK00".into()),
        report: Some("-10".into()),
        tx_df: Some(1500),
        rx_df: Some(1200),
        tx_message: Some("BB2BB AA1AA FN42".into()),
        tr_period: Some(15),
        ..StationStatus::default()
    });
    let back: Event = serde_json::from_str(&serde_json::to_string(&status).unwrap()).unwrap();
    assert_eq!(back, status);
    let old = r#"{"type":"status","dial_hz":1,"mode":"FT8","tx_enabled":false,"transmitting":false,"decoding":false,"de_call":"A","de_grid":"B"}"#;
    let parsed: Event = serde_json::from_str(old).unwrap();
    assert!(matches!(parsed, Event::Status(s) if s.dx_call.is_none() && s.tx_df.is_none()));
}

#[test]
fn decode_event_carries_country_and_the_new_tags() {
    let event = Event::Decode(DecodedMessage {
        id: 3,
        time_ms: 1,
        snr: -5,
        delta_time: 0.1,
        freq_hz: 900,
        text: "CQ AT3AB FN42".into(),
        tag: Some(DecodeTag::NewDxccBand),
        country: Some("Atlantis".into()),
        continent: Some("EU".into()),
    });
    let v: serde_json::Value = serde_json::to_value(&event).unwrap();
    assert_eq!(v["country"], "Atlantis");
    assert_eq!(v["continent"], "EU");
    assert_eq!(v["tag"], "new_dxcc_band");
    let grid = Event::Decode(DecodedMessage { tag: Some(DecodeTag::NewGrid), country: None, ..match event.clone() {
        Event::Decode(d) => d,
        _ => unreachable!(),
    } });
    let v: serde_json::Value = serde_json::to_value(&grid).unwrap();
    assert_eq!(v["tag"], "new_grid");
    assert!(v["country"].is_null());
}

#[test]
fn decode_json_without_country_still_parses() {
    let old = r#"{"type":"decode","id":1,"time_ms":0,"snr":-1,"delta_time":0.0,"freq_hz":100,"text":"X","tag":null}"#;
    match serde_json::from_str::<Event>(old).unwrap() {
        Event::Decode(d) => assert_eq!(d.country, None),
        other => panic!("{other:?}"),
    }
}
