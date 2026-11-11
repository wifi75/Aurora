//! Il flusso del motore proprio verso GridTracker: un bus con eventi finti e un socket locale che fa da destinatario.

use std::{net::SocketAddr, sync::Arc, time::Duration};

use aurora_core::{Command, DecodedMessage, Event, QsoRecord, StationStatus};
use tokio::{
    net::UdpSocket,
    sync::{broadcast, mpsc},
};
use udp_bridge::{decode, encode_free_text, encode_halt_tx, encode_reply, Decode, Message, Packet};
use udp_emitter::{Emitter, EmitterConfig};

async fn listener() -> (UdpSocket, SocketAddr) {
    let socket = UdpSocket::bind("127.0.0.1:0").await.expect("socket di prova");
    let addr = socket.local_addr().expect("indirizzo");
    (socket, addr)
}

async fn next(socket: &UdpSocket) -> Packet {
    let mut buf = vec![0u8; 65_535];
    let (n, _) = tokio::time::timeout(Duration::from_secs(3), socket.recv_from(&mut buf)).await.expect("nessun datagramma entro 3 s").expect("ricezione");
    decode(&buf[..n]).expect("datagramma decodificabile")
}

async fn silence(socket: &UdpSocket) {
    let mut buf = vec![0u8; 65_535];
    let got = tokio::time::timeout(Duration::from_millis(300), socket.recv_from(&mut buf)).await;
    assert!(got.is_err(), "non doveva arrivare altro");
}

fn status(dx: Option<&str>) -> StationStatus {
    StationStatus {
        dial_hz: 14_074_000,
        mode: "FT8".into(),
        de_call: "IU3ABC".into(),
        de_grid: "IO91wm".into(),
        dx_call: dx.map(str::to_string),
        dx_grid: dx.map(|_| "FN42".to_string()),
        rx_df: Some(1500),
        tx_df: Some(1500),
        tr_period: Some(15),
        ..StationStatus::default()
    }
}

fn message(id: u64, text: &str, freq: u32) -> DecodedMessage {
    DecodedMessage { id, time_ms: 45_015_000, snr: -12, delta_time: 0.3, freq_hz: freq, text: text.into(), tag: None, country: None, continent: None, lotw_last_upload: None, hint: false }
}

struct Rig {
    gridtracker: UdpSocket,
    emitter: Emitter,
    events: broadcast::Sender<Event>,
    commands: mpsc::Receiver<Command>,
}

async fn rig(accept: bool, adif: bool) -> Rig {
    let (gridtracker, addr) = listener().await;
    let (events, rx) = broadcast::channel(64);
    let (cmd_tx, commands) = mpsc::channel(16);
    let mut config = EmitterConfig::jtdx(vec![addr]);
    config.de_call = "IU3ABC".into();
    config.de_grid = "IO91wm".into();
    config.accept_requests = accept;
    config.accept_reply = 3;
    config.send_adif = adif;
    let emitter = Emitter::start(config, rx, cmd_tx, Arc::new(|| true)).await.expect("emettitore");
    Rig { gridtracker, emitter, events, commands }
}

#[tokio::test]
async fn it_starts_with_heartbeat_and_clear_then_sends_status_and_decodes_like_jtdx() {
    let rig = rig(false, false).await;
    let first = next(&rig.gridtracker).await;
    assert_eq!(first.schema, 2);
    assert_eq!(first.message, Message::Heartbeat { id: Some("JTDX".into()), max_schema: 3, version: Some("2.2.159".into()), revision: None });
    assert_eq!(next(&rig.gridtracker).await.message, Message::Clear { id: Some("JTDX".into()) });

    // Status con il DX scelto
    rig.events.send(Event::Status(status(Some("K1ABC")))).expect("evento");
    let Message::Status(s) = next(&rig.gridtracker).await.message else { panic!("atteso Status") };
    assert_eq!(s.id.as_deref(), Some("JTDX"));
    assert_eq!((s.dial_freq, s.mode.as_deref(), s.tx_mode.as_deref()), (14_074_000, Some("FT8"), Some("FT8")));
    assert_eq!((s.dx_call.as_deref(), s.dx_grid.as_deref(), s.de_call.as_deref(), s.de_grid.as_deref()), (Some("K1ABC"), Some("FN42"), Some("IU3ABC"), Some("IO91wm")));
    assert_eq!((s.rx_df, s.tx_df, s.tx_enabled, s.transmitting), (1500, 1500, false, false));
    // JTDX manda `tx_first` dopo `fast_mode`, niente periodo ne' testo
    assert_eq!((s.special_op_mode, s.tr_period, s.tx_message), (Some(1), None, None));

    // uno Status identico non si rimanda
    rig.events.send(Event::Status(status(Some("K1ABC")))).expect("evento");
    silence(&rig.gridtracker).await;

    // «Canc DX»: lo Status segue il DX scelto
    rig.events.send(Event::Status(status(None))).expect("evento");
    let Message::Status(cleared) = next(&rig.gridtracker).await.message else { panic!("atteso Status") };
    assert_eq!((cleared.dx_call.as_deref(), cleared.dx_grid.as_deref()), (Some(""), Some("")));

    // un Decode per messaggio, con i campi del bus e il carattere del modo di FT8
    rig.events.send(Event::Decode(message(7, "CQ K1ABC FN42", 1234))).expect("evento");
    let Message::Decode(d) = next(&rig.gridtracker).await.message else { panic!("atteso Decode") };
    assert_eq!(
        d,
        Decode {
            id: Some("JTDX".into()),
            is_new: true,
            time_ms: 45_015_000,
            snr: -12,
            delta_time: f64::from(0.3_f32),
            delta_freq: 1234,
            mode: Some("~".into()),
            message: Some("CQ K1ABC FN42".into()),
            low_confidence: false,
            off_air: false,
        }
    );
    rig.emitter.shutdown().await;
    assert_eq!(next(&rig.gridtracker).await.message, Message::Close { id: Some("JTDX".into()) });
}

#[tokio::test]
async fn ft4_decodes_carry_the_ft4_mode_character() {
    let rig = rig(false, false).await;
    next(&rig.gridtracker).await;
    next(&rig.gridtracker).await;
    let mut ft4 = status(None);
    ft4.mode = "FT4".into();
    rig.events.send(Event::Status(ft4)).expect("evento");
    let Message::Status(s) = next(&rig.gridtracker).await.message else { panic!("atteso Status") };
    assert_eq!(s.mode.as_deref(), Some("FT4"));
    rig.events.send(Event::Decode(message(1, "K1ABC IU3ABC -10", 900))).expect("evento");
    let Message::Decode(d) = next(&rig.gridtracker).await.message else { panic!("atteso Decode") };
    assert_eq!(d.mode.as_deref(), Some(":"));
}

#[tokio::test]
async fn a_logged_qso_goes_as_qso_logged_and_then_logged_adif() {
    let rig = rig(false, true).await;
    next(&rig.gridtracker).await;
    next(&rig.gridtracker).await;
    let qso = QsoRecord {
        call: "K1ABC".into(),
        grid: "FN42".into(),
        band: "20m".into(),
        mode: "FT8".into(),
        freq_hz: 14_075_084,
        rst_sent: "-16".into(),
        rst_rcvd: "-09".into(),
        time_ms: 45_015_000,
        time_on_unix_ms: 1_792_860_405_000,
        time_off_unix_ms: 1_792_860_449_000,
        ..Default::default()
    };
    rig.events.send(Event::QsoLogged(qso.clone())).expect("evento");
    let Message::QsoLogged { id, dx_call, dx_grid, tx_freq, mode, report_sent, report_received, extra } = next(&rig.gridtracker).await.message else {
        panic!("atteso QsoLogged")
    };
    assert_eq!(id.as_deref(), Some("JTDX"));
    assert_eq!((dx_call.as_deref(), dx_grid.as_deref(), tx_freq, mode.as_deref()), (Some("K1ABC"), Some("FN42"), 14_075_084, Some("FT8")));
    assert_eq!((report_sent.as_deref(), report_received.as_deref()), (Some("-16"), Some("-09")));
    let extra = extra.expect("coda");
    assert_eq!((extra.time_on_ms, extra.time_off_ms), (Some(1_792_860_405_000), Some(1_792_860_449_000)));
    assert_eq!((extra.my_call.as_deref(), extra.operator_call.as_deref(), extra.my_grid.as_deref()), (Some("IU3ABC"), Some("IU3ABC"), Some("IO91wm")));
    let Message::LoggedAdif { adif, .. } = next(&rig.gridtracker).await.message else { panic!("atteso LoggedAdif") };
    let adif = adif.expect("testo ADIF");
    assert!(adif.starts_with("\n<adif_ver:5>3.1.0\n<programid:4>JTDX\n<EOH>\n<BAND:3>20m"), "{adif}");
    assert!(adif.contains("<CALL:5>K1ABC") && adif.ends_with("<EOR>"), "{adif}");
    // lo stesso QSO due volte di fila e' un solo QSO
    rig.events.send(Event::QsoLogged(qso)).expect("evento");
    silence(&rig.gridtracker).await;
}

#[tokio::test]
async fn gridtracker_commands_reach_aurora_only_when_requests_are_accepted() {
    let mut rig = rig(true, false).await;
    next(&rig.gridtracker).await;
    next(&rig.gridtracker).await;
    rig.events.send(Event::Decode(message(42, "CQ K1ABC FN42", 1234))).expect("evento");
    let Message::Decode(d) = next(&rig.gridtracker).await.message else { panic!("atteso Decode") };
    let to = rig.emitter.local_addr();

    // Reply: il doppio clic di GridTracker su quella riga
    rig.gridtracker.send_to(&encode_reply("JTDX", 3, &d, 0), to).await.expect("invio");
    assert_eq!(tokio::time::timeout(Duration::from_secs(3), rig.commands.recv()).await.expect("comando").expect("canale"), Command::Reply { decode_id: 42 });
    // un Reply a un messaggio mai visto si ignora
    let unknown = Decode { message: Some("CQ ZZ9ZZ AA00".into()), ..d.clone() };
    rig.gridtracker.send_to(&encode_reply("JTDX", 3, &unknown, 0), to).await.expect("invio");
    // un comando per un'altra istanza si ignora
    rig.gridtracker.send_to(&encode_halt_tx("WSJT-X", 3, false), to).await.expect("invio");
    rig.gridtracker.send_to(&encode_halt_tx("JTDX", 3, false), to).await.expect("invio");
    assert_eq!(tokio::time::timeout(Duration::from_secs(3), rig.commands.recv()).await.expect("comando").expect("canale"), Command::HaltTx);
    rig.gridtracker.send_to(&encode_free_text("JTDX", 3, "TNX 73", false), to).await.expect("invio");
    assert_eq!(
        tokio::time::timeout(Duration::from_secs(3), rig.commands.recv()).await.expect("comando").expect("canale"),
        Command::SendFreeText { text: "TNX 73".into(), send: false }
    );
}

#[tokio::test]
async fn commands_are_ignored_when_the_requests_are_not_accepted_but_replay_still_works() {
    let mut rig = rig(false, false).await;
    next(&rig.gridtracker).await;
    next(&rig.gridtracker).await;
    rig.events.send(Event::Status(status(Some("K1ABC")))).expect("evento");
    next(&rig.gridtracker).await;
    rig.events.send(Event::Decode(message(5, "CQ K1ABC FN42", 1234))).expect("evento");
    let Message::Decode(d) = next(&rig.gridtracker).await.message else { panic!("atteso Decode") };
    let to = rig.emitter.local_addr();
    rig.gridtracker.send_to(&encode_reply("JTDX", 3, &d, 0), to).await.expect("invio");
    rig.gridtracker.send_to(&encode_halt_tx("JTDX", 3, false), to).await.expect("invio");
    assert!(tokio::time::timeout(Duration::from_millis(400), rig.commands.recv()).await.is_err(), "nessun comando senza AcceptUDPRequests");

    // il Replay si accetta sempre: rimanda i decode (non nuovi) e lo stato
    rig.gridtracker.send_to(&udp_bridge::encode_replay("JTDX", 3), to).await.expect("invio");
    let Message::Decode(again) = next(&rig.gridtracker).await.message else { panic!("atteso Decode") };
    assert_eq!((again.is_new, again.message.as_deref()), (false, Some("CQ K1ABC FN42")));
    let Message::Status(s) = next(&rig.gridtracker).await.message else { panic!("atteso Status") };
    assert_eq!(s.dx_call.as_deref(), Some("K1ABC"));
}

async fn rig_with(accept: bool, apply_filters: bool, filters: aurora_core::UdpFilters, accepted: Arc<std::sync::atomic::AtomicUsize>) -> Rig {
    let (gridtracker, addr) = listener().await;
    let (events, rx) = broadcast::channel(64);
    let (cmd_tx, commands) = mpsc::channel(16);
    let mut config = EmitterConfig::jtdx(vec![addr]);
    config.de_call = "IU3ABC".into();
    config.accept_requests = accept;
    config.accept_reply = 3;
    config.apply_filters = apply_filters;
    let hooks = udp_emitter::EmitterHooks {
        filters: Some(Arc::new(std::sync::RwLock::new(filters))),
        own_continent: Some("EU".into()),
        on_reply_accepted: Some(Arc::new(move || {
            accepted.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        })),
        ..Default::default()
    };
    let emitter = Emitter::start_with(config, rx, cmd_tx, Arc::new(|| true), hooks).await.expect("emettitore");
    Rig { gridtracker, emitter, events, commands }
}

#[tokio::test]
async fn with_the_text_filters_applied_hidden_rows_do_not_leave_and_cannot_be_replied_to() {
    let filters = aurora_core::UdpFilters { hide_continents: vec!["NA".into()], hide_own_continent: true, ..Default::default() };
    let mut rig = rig_with(true, true, filters, Arc::new(std::sync::atomic::AtomicUsize::new(0))).await;
    next(&rig.gridtracker).await;
    next(&rig.gridtracker).await;
    let with = |id: u64, text: &str, continent: &str| DecodedMessage { continent: Some(continent.into()), ..message(id, text, 1200 + id as u32) };
    rig.events.send(Event::Decode(with(1, "CQ K1ABC FN42", "NA"))).expect("evento");
    rig.events.send(Event::Decode(with(2, "CQ DL1ABC JO62", "EU"))).expect("evento");
    rig.events.send(Event::Decode(with(3, "CQ JA1ABC PM95", "AS"))).expect("evento");
    let Message::Decode(shown) = next(&rig.gridtracker).await.message else { panic!("atteso Decode") };
    assert_eq!(shown.message.as_deref(), Some("CQ JA1ABC PM95"), "il Nord America e il proprio continente sono nascosti");
    silence(&rig.gridtracker).await;
    // un Reply a una riga che non e' uscita non fa partire niente
    let hidden = Decode { message: Some("CQ K1ABC FN42".into()), delta_freq: 1201, ..shown.clone() };
    rig.gridtracker.send_to(&encode_reply("JTDX", 3, &hidden, 0), rig.emitter.local_addr()).await.expect("invio");
    assert!(tokio::time::timeout(Duration::from_millis(400), rig.commands.recv()).await.is_err());
}

#[tokio::test]
async fn without_the_option_the_filters_do_not_touch_the_udp_flow() {
    let filters = aurora_core::UdpFilters { hide_continents: vec!["NA".into()], ..Default::default() };
    let rig = rig_with(false, false, filters, Arc::new(std::sync::atomic::AtomicUsize::new(0))).await;
    next(&rig.gridtracker).await;
    next(&rig.gridtracker).await;
    let na = DecodedMessage { continent: Some("NA".into()), ..message(1, "CQ K1ABC FN42", 1200) };
    rig.events.send(Event::Decode(na)).expect("evento");
    let Message::Decode(d) = next(&rig.gridtracker).await.message else { panic!("atteso Decode") };
    assert_eq!(d.message.as_deref(), Some("CQ K1ABC FN42"));
}

#[tokio::test]
async fn an_accepted_reply_notifies_the_window_hook_and_other_requests_do_not() {
    let counter = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let mut rig = rig_with(true, false, aurora_core::UdpFilters::default(), Arc::clone(&counter)).await;
    next(&rig.gridtracker).await;
    next(&rig.gridtracker).await;
    rig.events.send(Event::Decode(message(9, "CQ K1ABC FN42", 1234))).expect("evento");
    let Message::Decode(d) = next(&rig.gridtracker).await.message else { panic!("atteso Decode") };
    let to = rig.emitter.local_addr();
    rig.gridtracker.send_to(&encode_halt_tx("JTDX", 3, false), to).await.expect("invio");
    assert_eq!(tokio::time::timeout(Duration::from_secs(3), rig.commands.recv()).await.expect("comando").expect("canale"), Command::HaltTx);
    assert_eq!(counter.load(std::sync::atomic::Ordering::SeqCst), 0, "HaltTx non porta la finestra in primo piano");
    let unknown = Decode { message: Some("CQ ZZ9ZZ AA00".into()), ..d.clone() };
    rig.gridtracker.send_to(&encode_reply("JTDX", 3, &unknown, 0), to).await.expect("invio");
    rig.gridtracker.send_to(&encode_reply("JTDX", 3, &d, 0), to).await.expect("invio");
    assert_eq!(tokio::time::timeout(Duration::from_secs(3), rig.commands.recv()).await.expect("comando").expect("canale"), Command::Reply { decode_id: 9 });
    assert_eq!(counter.load(std::sync::atomic::Ordering::SeqCst), 1, "solo il Reply trovato e accettato");
}

#[tokio::test]
async fn the_comment_name_and_power_of_the_qso_travel_in_the_udp_qso_logged() {
    let rig = rig(false, true).await;
    next(&rig.gridtracker).await;
    next(&rig.gridtracker).await;
    let qso = QsoRecord {
        call: "K1ABC".into(),
        grid: "FN42".into(),
        band: "20m".into(),
        mode: "FT8".into(),
        freq_hz: 14_075_084,
        rst_sent: "-16".into(),
        rst_rcvd: "-09".into(),
        time_off_unix_ms: 1_792_860_449_000,
        time_on_unix_ms: 1_792_860_405_000,
        name: "Mario".into(),
        comment: "Distance: 6000 km".into(),
        tx_power: "50".into(),
        ..Default::default()
    };
    rig.events.send(Event::QsoLogged(qso)).expect("evento");
    let Message::QsoLogged { extra, .. } = next(&rig.gridtracker).await.message else { panic!("atteso QsoLogged") };
    let extra = extra.expect("coda");
    assert_eq!((extra.name.as_deref(), extra.comments.as_deref(), extra.tx_power.as_deref()), (Some("Mario"), Some("Distance: 6000 km"), Some("50")));
    let Message::LoggedAdif { adif, .. } = next(&rig.gridtracker).await.message else { panic!("atteso LoggedAdif") };
    assert!(adif.expect("adif").contains("<NAME:5>Mario <COMMENT:17>Distance: 6000 km"));
}

async fn rig_external(accept: bool, seen: Arc<std::sync::Mutex<Vec<udp_emitter::External>>>) -> Rig {
    let (gridtracker, addr) = listener().await;
    let (events, rx) = broadcast::channel(64);
    let (cmd_tx, commands) = mpsc::channel(16);
    let mut config = EmitterConfig::jtdx(vec![addr]);
    config.accept_requests = accept;
    let hooks = udp_emitter::EmitterHooks {
        on_external: Some(Arc::new(move |e| {
            if let Ok(mut v) = seen.lock() {
                v.push(e);
            }
        })),
        ..Default::default()
    };
    let emitter = Emitter::start_with(config, rx, cmd_tx, Arc::new(|| true), hooks).await.expect("emettitore");
    Rig { gridtracker, emitter, events, commands }
}

async fn wait_for(seen: &std::sync::Mutex<Vec<udp_emitter::External>>, n: usize) -> Vec<udp_emitter::External> {
    for _ in 0..100 {
        let now = seen.lock().map(|v| v.clone()).unwrap_or_default();
        if now.len() >= n {
            return now;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    seen.lock().map(|v| v.clone()).unwrap_or_default()
}

#[tokio::test]
async fn highlight_clear_location_configure_and_switch_reach_the_hook_only_when_accepted() {
    use udp_bridge::{encode_clear_window, encode_configure, encode_highlight_callsign, encode_location, encode_switch_configuration, ConfigureFields};
    use udp_emitter::External;
    let seen = Arc::new(std::sync::Mutex::new(Vec::new()));
    let rig = rig_external(true, Arc::clone(&seen)).await;
    let to = rig.emitter.local_addr();
    let datagrams = [
        encode_highlight_callsign("JTDX", 3, "k1abc", Some([255, 0, 0]), None, true),
        encode_clear_window("JTDX", 3, 1),
        encode_location("JTDX", 3, "fn42ab"),
        encode_location("JTDX", 3, "non un locatore"),
        encode_configure("JTDX", 3, &ConfigureFields { mode: "FT4".into(), rx_df: 1200, ..ConfigureFields::default() }),
        encode_switch_configuration("JTDX", 3, "Gara"),
    ];
    for d in &datagrams {
        rig.gridtracker.send_to(d, to).await.expect("invio");
    }
    let got = wait_for(&seen, 5).await;
    assert_eq!(got.len(), 5, "il Location non valido si scarta: {got:?}");
    assert_eq!(got[0], External::Highlight { call: "K1ABC".into(), background: Some([255, 0, 0]), foreground: None, last_only: true });
    assert_eq!(got[1], External::Clear { window: 1 });
    assert_eq!(got[2], External::Location { grid: "FN42ab".into() });
    let External::Configure(c) = &got[3] else { panic!("atteso Configure") };
    assert_eq!((c.mode.as_deref(), c.rx_df, c.dx_call.as_deref()), (Some("FT4"), Some(1200), None));
    assert_eq!(got[4], External::SwitchConfiguration { name: "Gara".into() });

    // con «accetta le richieste» spento non passa niente
    let closed = Arc::new(std::sync::Mutex::new(Vec::new()));
    let rig = rig_external(false, Arc::clone(&closed)).await;
    for d in &datagrams {
        rig.gridtracker.send_to(d, rig.emitter.local_addr()).await.expect("invio");
    }
    tokio::time::sleep(Duration::from_millis(300)).await;
    assert!(closed.lock().map(|v| v.is_empty()).unwrap_or(false));
}

#[tokio::test]
async fn each_destination_receives_only_its_own_choice() {
    let (stream_socket, stream_addr) = listener().await;
    let (adif_socket, adif_addr) = listener().await;
    let (events, rx) = broadcast::channel(64);
    let (cmd_tx, _commands) = mpsc::channel(16);
    let mut config = EmitterConfig::jtdx(Vec::new());
    config.de_call = "IU3ABC".into();
    config.destinations = vec![
        udp_emitter::Destination { addr: stream_addr, stream: true, logged_adif: false, multicast: None },
        udp_emitter::Destination { addr: adif_addr, stream: false, logged_adif: true, multicast: None },
    ];
    let _emitter = Emitter::start(config, rx, cmd_tx, Arc::new(|| true)).await.expect("emettitore");
    assert!(matches!(next(&stream_socket).await.message, Message::Heartbeat { .. }));
    assert!(matches!(next(&stream_socket).await.message, Message::Clear { .. }));
    events.send(Event::QsoLogged(QsoRecord { call: "K1ABC".into(), grid: "FN42".into(), mode: "FT8".into(), time_off_unix_ms: 1_792_860_449_000, ..Default::default() })).expect("evento");
    assert!(matches!(next(&stream_socket).await.message, Message::QsoLogged { .. }));
    silence(&stream_socket).await;
    let Message::LoggedAdif { adif, .. } = next(&adif_socket).await.message else { panic!("solo il Logged ADIF") };
    assert!(adif.expect("adif").contains("<CALL:5>K1ABC"));
    silence(&adif_socket).await;
}

#[tokio::test]
async fn a_multicast_group_on_the_loopback_receives_the_flow() {
    use std::net::Ipv4Addr;
    let group = Ipv4Addr::new(239, 255, 77, 31);
    let std_socket = std::net::UdpSocket::bind("0.0.0.0:0").expect("socket");
    let port = std_socket.local_addr().expect("indirizzo").port();
    if std_socket.join_multicast_v4(&group, &Ipv4Addr::LOCALHOST).is_err() {
        // un sistema senza multicast sul loopback: la prova non ha senso
        return;
    }
    std_socket.set_nonblocking(true).expect("non bloccante");
    let listener = UdpSocket::from_std(std_socket).expect("tokio");
    let (_events, rx) = broadcast::channel(64);
    let (cmd_tx, _commands) = mpsc::channel(16);
    let mut config = EmitterConfig::jtdx(Vec::new());
    config.destinations = vec![udp_emitter::Destination {
        addr: SocketAddr::from((group, port)),
        stream: true,
        logged_adif: false,
        multicast: Some(udp_emitter::Multicast { ttl: 1, interface: Some(Ipv4Addr::LOCALHOST.into()) }),
    }];
    let emitter = Emitter::start(config, rx, cmd_tx, Arc::new(|| true)).await.expect("emettitore");
    let mut buf = vec![0u8; 65_535];
    let Ok(Ok((n, from))) = tokio::time::timeout(Duration::from_secs(2), listener.recv_from(&mut buf)).await else {
        // alcuni sistemi non riportano il multicast sul loopback: niente da verificare
        return;
    };
    assert!(matches!(decode(&buf[..n]).expect("datagramma").message, Message::Heartbeat { .. }));
    // il mittente e' il socket multicast dell'emettitore: i comandi tornano li'
    assert!(emitter.local_addrs().iter().any(|a| a.port() == from.port()));
}
