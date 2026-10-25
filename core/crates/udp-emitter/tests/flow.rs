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
    DecodedMessage { id, time_ms: 45_015_000, snr: -12, delta_time: 0.3, freq_hz: freq, text: text.into(), tag: None, country: None, continent: None }
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
