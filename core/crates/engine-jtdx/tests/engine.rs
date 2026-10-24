use std::{
    path::PathBuf,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
    time::Duration,
};

use aurora_core::{Bus, Command, DecodeTag, Engine, EngineIo, Event};
use engine_jtdx::{Clock, JtdxConfig, JtdxEngine};
use tokio::{net::UdpSocket, sync::broadcast, time::timeout};
use udp_bridge::{decode, encode_halt_tx, encode_replay, encode_reply, wire::Writer, Message, MAGIC};
use udp_hub::{Hub, HubConfig};

const REPLY: u32 = 4;

struct FakeClock(AtomicU64);

impl Clock for FakeClock {
    fn now_ms(&self) -> u64 {
        self.0.load(Ordering::SeqCst)
    }
}

struct Rig {
    bus: Bus,
    hub: Arc<Hub>,
    jtdx: UdpSocket,
    events: broadcast::Receiver<Event>,
    clock: Arc<FakeClock>,
}

async fn rig(config: JtdxConfig) -> Rig {
    let hub = Arc::new(Hub::start(HubConfig { listen: "127.0.0.1:0".parse().unwrap(), forward: vec![] }).await.unwrap());
    let bus = Bus::new(1024);
    let events = bus.subscribe_events();
    let clock = Arc::new(FakeClock(AtomicU64::new(1_000_000)));
    let io = EngineIo { events: bus.events(), spectrum: bus.spectrum(), commands: bus.take_command_rx().unwrap() };
    tokio::spawn(JtdxEngine::new(Arc::clone(&hub), config, clock.clone()).run(io));
    let jtdx = UdpSocket::bind("127.0.0.1:0").await.unwrap();
    Rig { bus, hub, jtdx, events, clock }
}

impl Rig {
    async fn send(&self, bytes: &[u8]) {
        self.jtdx.send_to(bytes, self.hub.local_addr()).await.unwrap();
    }

    async fn event(&mut self) -> Option<Event> {
        timeout(Duration::from_secs(1), self.events.recv()).await.ok()?.ok()
    }

    async fn no_event(&mut self) -> bool {
        timeout(Duration::from_millis(300), self.events.recv()).await.is_err()
    }

    /// Tutto cio' che JTDX riceve dall'hub nell'intervallo dato.
    async fn received(&self, wait_ms: u64) -> Vec<Vec<u8>> {
        let mut out = Vec::new();
        let mut buf = vec![0u8; 65_535];
        while let Ok(Ok((n, _))) = timeout(Duration::from_millis(wait_ms), self.jtdx.recv_from(&mut buf)).await {
            out.push(buf[..n].to_vec());
        }
        out
    }

    async fn command(&self, c: Command) {
        self.bus.commands().send(c).await.unwrap();
    }
}

fn kind_of(datagram: &[u8]) -> u32 {
    u32::from_be_bytes(datagram[8..12].try_into().unwrap())
}

fn fixture(name: &str) -> Vec<u8> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../udp-bridge/tests/fixtures").join(name);
    std::fs::read(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

fn header(kind: u32) -> Writer {
    let mut w = Writer::new();
    w.u32(MAGIC);
    w.u32(2);
    w.u32(kind);
    w.utf8(Some("JTDX"));
    w
}

fn decode_packet(text: &str, is_new: bool, time_ms: u32) -> Vec<u8> {
    let mut w = header(2);
    w.bool(is_new);
    w.u32(time_ms);
    w.i32(-10);
    w.f64(0.2);
    w.u32(1500);
    w.utf8(Some("~"));
    w.utf8(Some(text));
    w.bool(false);
    w.bool(false);
    w.into_bytes()
}

fn allow_tx() -> JtdxConfig {
    JtdxConfig { allow_tx: true, ..JtdxConfig::default() }
}

#[tokio::test]
async fn halt_command_announces_tx_halted_even_before_jtdx_is_known() {
    let mut r = rig(JtdxConfig::default()).await;
    r.command(Command::HaltTx).await;
    assert!(matches!(r.event().await, Some(Event::TxHalted)), "lo STOP deve spegnere anche l'automazione");
}

#[tokio::test]
async fn status_fixture_becomes_station_status() {
    let mut r = rig(JtdxConfig::default()).await;
    r.send(&fixture("status-decoding.bin")).await;
    match r.event().await {
        Some(Event::Status(s)) => {
            assert_eq!(s.dial_hz, 21_074_000);
            assert_eq!(s.mode, "FT8");
            assert_eq!(s.de_call, "ZZ9ZZZ");
            assert!(s.decoding && !s.tx_enabled && !s.transmitting);
        }
        other => panic!("atteso Status, trovato {other:?}"),
    }
}

#[tokio::test]
async fn decode_fixture_becomes_decoded_message_with_cq_tag() {
    let mut r = rig(JtdxConfig::default()).await;
    r.send(&fixture("decode-cq.bin")).await;
    match r.event().await {
        Some(Event::Decode(d)) => {
            assert_eq!(d.id, 1);
            assert_eq!(d.text, "CQ AA1AA FN42");
            assert_eq!(d.tag, Some(DecodeTag::Cq));
        }
        other => panic!("atteso Decode, trovato {other:?}"),
    }
}

#[tokio::test]
async fn clear_resets_reply_ability() {
    let mut r = rig(allow_tx()).await;
    r.send(&decode_packet("CQ AA1AA FN42", true, 100)).await;
    assert!(matches!(r.event().await, Some(Event::Decode(_))));
    r.send(&fixture("clear.bin")).await;
    tokio::time::sleep(Duration::from_millis(100)).await;
    r.command(Command::Reply { decode_id: 1 }).await;
    let got = r.received(400).await;
    assert!(got.iter().all(|d| kind_of(d) != REPLY), "dopo Clear nessun Reply deve partire");
}

#[tokio::test]
async fn halt_tx_always_reaches_jtdx_even_without_allow_tx() {
    let r = rig(JtdxConfig::default()).await;
    r.send(&fixture("status-decoding.bin")).await;
    tokio::time::sleep(Duration::from_millis(100)).await;
    r.command(Command::HaltTx).await;
    let got = r.received(400).await;
    assert!(got.contains(&encode_halt_tx("JTDX", 2, false)), "HaltTx deve arrivare sempre");
}

#[tokio::test]
async fn reply_not_sent_when_allow_tx_false() {
    let mut r = rig(JtdxConfig::default()).await;
    r.send(&decode_packet("CQ AA1AA FN42", true, 100)).await;
    assert!(matches!(r.event().await, Some(Event::Decode(_))));
    r.command(Command::Reply { decode_id: 1 }).await;
    let got = r.received(400).await;
    assert!(got.iter().all(|d| kind_of(d) != REPLY), "senza allow_tx nessun Reply deve raggiungere JTDX");
}

#[tokio::test]
async fn reply_sent_with_original_untrimmed_decode_when_allowed() {
    let mut r = rig(allow_tx()).await;
    let padded = "CQ AA1AA FN42          ";
    let packet = decode_packet(padded, true, 25_905_000);
    r.send(&packet).await;
    let id = match r.event().await {
        Some(Event::Decode(d)) => {
            assert_eq!(d.text, "CQ AA1AA FN42", "per l'interfaccia il testo e' ripulito");
            d.id
        }
        other => panic!("atteso Decode, trovato {other:?}"),
    };
    r.command(Command::Reply { decode_id: id }).await;
    let original = match decode(&packet).unwrap().message {
        Message::Decode(d) => d,
        other => panic!("{other:?}"),
    };
    let expected = encode_reply("JTDX", 2, &original, 0);
    let got = r.received(400).await;
    assert!(got.contains(&expected), "il Reply deve rimandare il decode originale, spazi compresi");
    assert!(expected.windows(padded.len()).any(|w| w == padded.as_bytes()));
}

#[tokio::test]
async fn reply_unknown_or_stale_decode_is_ignored() {
    let mut r = rig(allow_tx()).await;
    r.command(Command::Reply { decode_id: 999 }).await;
    r.send(&decode_packet("CQ AA1AA FN42", true, 100)).await;
    assert!(matches!(r.event().await, Some(Event::Decode(_))));
    r.clock.0.fetch_add(46_000, Ordering::SeqCst);
    r.command(Command::Reply { decode_id: 1 }).await;
    let got = r.received(400).await;
    assert!(got.iter().all(|d| kind_of(d) != REPLY), "decode sconosciuto o troppo vecchio: nessun Reply");
}

#[tokio::test]
async fn replay_is_requested_when_jtdx_first_seen_and_again_after_restart() {
    let r = rig(JtdxConfig::default()).await;
    r.send(&fixture("status-decoding.bin")).await;
    r.send(&fixture("status-decoding.bin")).await;
    let got = r.received(400).await;
    let replay = encode_replay("JTDX", 2);
    assert_eq!(got.iter().filter(|d| **d == replay).count(), 1, "un solo Replay per la stessa sessione");

    let restarted = UdpSocket::bind("127.0.0.1:0").await.unwrap();
    restarted.send_to(&fixture("status-decoding.bin"), r.hub.local_addr()).await.unwrap();
    let mut buf = vec![0u8; 65_535];
    let (n, _) = timeout(Duration::from_secs(1), restarted.recv_from(&mut buf)).await.unwrap().unwrap();
    assert_eq!(&buf[..n], replay.as_slice(), "JTDX riavviato: nuovo Replay");
}

#[tokio::test]
async fn no_replay_when_disabled() {
    let r = rig(JtdxConfig { replay_on_connect: false, ..JtdxConfig::default() }).await;
    r.send(&fixture("status-decoding.bin")).await;
    assert!(r.received(400).await.is_empty());
}

#[tokio::test]
async fn foreign_garbage_is_ignored_without_panic() {
    let mut r = rig(JtdxConfig::default()).await;
    let mut junk = MAGIC.to_be_bytes().to_vec();
    junk.extend_from_slice(&[1, 2, 3]);
    r.send(&junk).await;
    r.send(&[0xde, 0xad]).await;
    r.send(&fixture("status-decoding.bin")).await;
    assert!(matches!(r.event().await, Some(Event::Status(_))), "il motore deve continuare a funzionare");
}

#[tokio::test]
async fn replayed_duplicate_is_not_emitted_twice() {
    let mut r = rig(JtdxConfig::default()).await;
    r.send(&decode_packet("CQ AA1AA FN42", true, 500)).await;
    assert!(matches!(r.event().await, Some(Event::Decode(_))));
    r.send(&decode_packet("CQ AA1AA FN42", false, 500)).await;
    assert!(r.no_event().await, "il Replay dello stesso decode non va ripetuto");
    r.send(&decode_packet("CQ BB2BB FN42", false, 500)).await;
    assert!(matches!(r.event().await, Some(Event::Decode(d)) if d.id == 2));
}

#[tokio::test]
async fn logged_qso_becomes_event() {
    let mut r = rig(JtdxConfig::default()).await;
    let mut w = header(5);
    w.i64(2_461_000);
    w.u32(45_015_000);
    w.u8(1);
    w.utf8(Some("AA1AA"));
    w.utf8(Some("FN42"));
    w.u64(14_074_000);
    w.utf8(Some("FT8"));
    w.utf8(Some("-08"));
    w.utf8(Some("-12"));
    r.send(&w.into_bytes()).await;
    match r.event().await {
        Some(Event::QsoLogged(q)) => {
            assert_eq!((q.call.as_str(), q.band.as_str(), q.mode.as_str()), ("AA1AA", "20m", "FT8"));
            assert_eq!((q.rst_sent.as_str(), q.rst_rcvd.as_str()), ("-08", "-12"));
            assert_eq!(q.time_ms, 1_000_000);
        }
        other => panic!("atteso QsoLogged, trovato {other:?}"),
    }
}
