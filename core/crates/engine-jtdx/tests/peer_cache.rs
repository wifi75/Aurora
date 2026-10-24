//! Dopo un riavvio di Aurora i comandi devono poter raggiungere JTDX subito: l'indirizzo e l'identificativo si ricordano su disco.

use std::{
    net::SocketAddr,
    path::PathBuf,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
    time::Duration,
};

use aurora_core::{Bus, Command, Engine, EngineIo};
use engine_jtdx::{decode_peer, encode_peer, Clock, JtdxConfig, JtdxEngine};
use tokio::{net::UdpSocket, time::timeout};
use udp_bridge::encode_halt_tx;
use udp_hub::{Hub, HubConfig};

struct FakeClock(AtomicU64);

impl Clock for FakeClock {
    fn now_ms(&self) -> u64 {
        self.0.load(Ordering::SeqCst)
    }
}

fn temp_file(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("aurora-peer-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir.join("jtdx-peer.txt")
}

fn fixture(name: &str) -> Vec<u8> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../udp-bridge/tests/fixtures").join(name);
    std::fs::read(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

async fn start(cache: &std::path::Path) -> (Bus, Arc<Hub>) {
    let hub = Arc::new(Hub::start(HubConfig { listen: "127.0.0.1:0".parse().unwrap(), forward: vec![] }).await.unwrap());
    let bus = Bus::new(1024);
    let io = EngineIo { events: bus.events(), spectrum: bus.spectrum(), commands: bus.take_command_rx().unwrap() };
    let config = JtdxConfig { peer_cache: Some(cache.to_path_buf()), ..JtdxConfig::default() };
    tokio::spawn(JtdxEngine::new(Arc::clone(&hub), config, Arc::new(FakeClock(AtomicU64::new(1_000_000)))).run(io));
    (bus, hub)
}

#[test]
fn the_peer_file_round_trips_and_refuses_garbage() {
    let addr: SocketAddr = "127.0.0.1:51234".parse().unwrap();
    assert_eq!(decode_peer(&encode_peer(addr, "JTDX", 2)), Some((addr, "JTDX".to_string(), 2)));
    assert_eq!(decode_peer(""), None);
    assert_eq!(decode_peer("non un indirizzo\nJTDX\n2\n"), None);
    assert_eq!(decode_peer("127.0.0.1:1\n\n2\n"), None);
    assert_eq!(decode_peer("127.0.0.1:1\nJTDX\nx\n"), None);
    assert_eq!(decode_peer("127.0.0.1:1\nJTDX\n2\nextra\n"), None);
}

#[tokio::test]
async fn stop_reaches_jtdx_right_after_a_restart_without_waiting_for_its_first_packet() {
    let jtdx = UdpSocket::bind("127.0.0.1:0").await.unwrap();
    let cache = temp_file("stop");
    std::fs::write(&cache, encode_peer(jtdx.local_addr().unwrap(), "JTDX", 2)).unwrap();
    let (bus, _hub) = start(&cache).await;
    bus.commands().send(Command::HaltTx).await.unwrap();
    let mut buf = vec![0u8; 2048];
    let (n, _) = timeout(Duration::from_secs(2), jtdx.recv_from(&mut buf)).await.expect("STOP non arrivato").unwrap();
    assert_eq!(&buf[..n], encode_halt_tx("JTDX", 2, false).as_slice());
}

#[tokio::test]
async fn the_first_packet_from_jtdx_is_remembered_for_the_next_restart() {
    let cache = temp_file("learn");
    let (_bus, hub) = start(&cache).await;
    let jtdx = UdpSocket::bind("127.0.0.1:0").await.unwrap();
    jtdx.send_to(&fixture("status-decoding.bin"), hub.local_addr()).await.unwrap();
    let mut found = None;
    for _ in 0..40 {
        if let Some(read) = std::fs::read_to_string(&cache).ok().as_deref().and_then(decode_peer) {
            found = Some(read);
            break;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    let (addr, id, _schema) = found.expect("indirizzo di JTDX non salvato");
    assert_eq!(addr, jtdx.local_addr().unwrap());
    assert_eq!(id, "JTDX");
}

#[tokio::test]
async fn without_a_cache_the_command_is_dropped_as_before() {
    let cache = temp_file("none");
    let (bus, _hub) = start(&cache).await;
    let jtdx = UdpSocket::bind("127.0.0.1:0").await.unwrap();
    bus.commands().send(Command::HaltTx).await.unwrap();
    let mut buf = vec![0u8; 2048];
    assert!(timeout(Duration::from_millis(400), jtdx.recv_from(&mut buf)).await.is_err());
}
