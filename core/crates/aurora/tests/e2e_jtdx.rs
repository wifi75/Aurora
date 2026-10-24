mod common;

use std::{net::SocketAddr, path::PathBuf, time::Duration};

use aurora::{run, Config, EngineKind, RunningServer};
use aurora_core::Event;
use common::*;
use tokio::{net::UdpSocket, sync::oneshot, time::timeout};
use udp_bridge::{decode, encode_halt_tx, encode_reply, Message};

const REPLY: u32 = 4;

fn fixture(name: &str) -> Vec<u8> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../udp-bridge/tests/fixtures").join(name);
    std::fs::read(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

fn kind_of(d: &[u8]) -> u32 {
    u32::from_be_bytes(d[8..12].try_into().unwrap())
}

struct Rig {
    server: RunningServer,
    jtdx: UdpSocket,
    hub_addr: SocketAddr,
    _stop: oneshot::Sender<()>,
}

async fn rig(allow_tx: bool, forward: Vec<SocketAddr>) -> Rig {
    let (tx, rx) = oneshot::channel::<()>();
    let config = Config {
        bind: "127.0.0.1:0".parse().unwrap(),
        data_dir: temp_dir("jtdx"),
        engine: EngineKind::Jtdx,
        udp_listen: "127.0.0.1:0".parse().unwrap(),
        udp_forward: forward,
        allow_tx,
        ..Config::defaults()
    };
    let server = run(config, async move {
        let _ = rx.await;
    })
    .await
    .unwrap();
    let hub_addr = server.hub.as_ref().expect("con il motore JTDX l'hub esiste").local_addr();
    let jtdx = UdpSocket::bind("127.0.0.1:0").await.unwrap();
    Rig { server, jtdx, hub_addr, _stop: tx }
}

impl Rig {
    async fn send(&self, bytes: &[u8]) {
        self.jtdx.send_to(bytes, self.hub_addr).await.unwrap();
    }

    async fn received(&self, wait_ms: u64) -> Vec<Vec<u8>> {
        let mut out = Vec::new();
        let mut buf = vec![0u8; 65_535];
        while let Ok(Ok((n, _))) = timeout(Duration::from_millis(wait_ms), self.jtdx.recv_from(&mut buf)).await {
            out.push(buf[..n].to_vec());
        }
        out
    }
}

#[test]
fn config_reads_jtdx_variables_and_rejects_bad_ones() {
    let none = |_: &str| None::<String>;
    let c = Config::from_lookup(none).unwrap();
    assert_eq!(c.udp_listen, "127.0.0.1:2334".parse::<SocketAddr>().unwrap());
    assert!(c.udp_forward.is_empty());
    assert!(!c.allow_tx, "la trasmissione e' chiusa per impostazione predefinita");
    assert!(c.audio_device.is_none());

    let env = |k: &str| match k {
        "AURORA_ENGINE" => Some("jtdx".to_string()),
        "AURORA_UDP_LISTEN" => Some("127.0.0.1:2340".to_string()),
        "AURORA_UDP_FORWARD" => Some("127.0.0.1:2333, 127.0.0.1:2335".to_string()),
        "AURORA_ALLOW_TX" => Some("1".to_string()),
        "AURORA_AUDIO_DEVICE" => Some("USB Audio".to_string()),
        _ => None,
    };
    let c = Config::from_lookup(env).unwrap();
    assert_eq!(c.engine, EngineKind::Jtdx);
    assert_eq!(c.udp_listen.port(), 2340);
    assert_eq!(c.udp_forward.len(), 2);
    assert_eq!(c.udp_forward[1].port(), 2335);
    assert!(c.allow_tx);
    assert_eq!(c.audio_device.as_deref(), Some("USB Audio"));

    let bad = |name: &'static str, value: &'static str| move |k: &str| (k == name).then(|| value.to_string());
    assert!(Config::from_lookup(bad("AURORA_UDP_LISTEN", "non-un-indirizzo")).is_err());
    assert!(Config::from_lookup(bad("AURORA_UDP_FORWARD", "127.0.0.1:2333,xyz")).is_err());
    assert!(Config::from_lookup(bad("AURORA_ALLOW_TX", "2")).is_err());
    assert!(Config::from_lookup(bad("AURORA_ALLOW_TX", "forse")).is_err());
}

#[tokio::test]
async fn e2e_jtdx_status_and_decode_arrive_over_websocket() {
    let r = rig(false, vec![]).await;
    let mut ws = open_ws(r.server.addr, &cookie_for(&r.server)).await;
    r.send(&fixture("status-decoding.bin")).await;
    r.send(&fixture("decode-cq.bin")).await;
    let (mut status, mut decode_seen) = (false, false);
    for _ in 0..10 {
        match next_event(&mut ws, 1).await {
            Some(Event::Status(s)) if s.dial_hz == 21_074_000 => status = true,
            Some(Event::Decode(d)) if d.text == "CQ AA1AA FN42" => decode_seen = true,
            _ => {}
        }
        if status && decode_seen {
            break;
        }
    }
    assert!(status && decode_seen, "status: {status}, decode: {decode_seen}");
}

#[tokio::test]
async fn e2e_forward_target_receives_identical_datagrams() {
    let target = UdpSocket::bind("127.0.0.1:0").await.unwrap();
    let r = rig(false, vec![target.local_addr().unwrap()]).await;
    let status = fixture("status-decoding.bin");
    r.send(&status).await;
    let mut buf = vec![0u8; 65_535];
    let (n, _) = timeout(Duration::from_secs(1), target.recv_from(&mut buf)).await.unwrap().unwrap();
    assert_eq!(&buf[..n], status.as_slice(), "GridTracker2 deve ricevere i byte identici");
}

#[tokio::test]
async fn e2e_reply_is_403_and_nothing_reaches_jtdx_by_default() {
    let r = rig(false, vec![]).await;
    let cookie = cookie_for(&r.server);
    let mut ws = open_ws(r.server.addr, &cookie).await;
    r.send(&fixture("decode-cq.bin")).await;
    let id = loop {
        match next_event(&mut ws, 2).await {
            Some(Event::Decode(d)) => break d.id,
            Some(_) => {}
            None => panic!("nessun decode ricevuto"),
        }
    };
    tokio::time::sleep(Duration::from_millis(200)).await;
    let body = format!(r#"{{"decode_id":{id}}}"#);
    let (status, text) = http(r.server.addr, "POST", "/api/v1/commands/reply", Some(&cookie), Some(&body)).await;
    assert_eq!(status, 403);
    assert!(text.contains("tx_disabled"));
    let got = r.received(400).await;
    assert!(got.iter().all(|d| kind_of(d) != REPLY), "nessun Reply deve arrivare a JTDX");
}

#[tokio::test]
async fn e2e_reply_reaches_jtdx_only_with_allow_tx() {
    let r = rig(true, vec![]).await;
    let cookie = cookie_for(&r.server);
    let mut ws = open_ws(r.server.addr, &cookie).await;
    let packet = fixture("decode-cq.bin");
    r.send(&packet).await;
    let id = loop {
        match next_event(&mut ws, 2).await {
            Some(Event::Decode(d)) => break d.id,
            Some(_) => {}
            None => panic!("nessun decode ricevuto"),
        }
    };
    tokio::time::sleep(Duration::from_millis(200)).await;
    let body = format!(r#"{{"decode_id":{id}}}"#);
    let (status, _) = http(r.server.addr, "POST", "/api/v1/commands/reply", Some(&cookie), Some(&body)).await;
    assert_eq!(status, 200);
    let original = match decode(&packet).unwrap().message {
        Message::Decode(d) => d,
        other => panic!("{other:?}"),
    };
    let got = r.received(500).await;
    assert!(got.contains(&encode_reply("JTDX", 2, &original, 0)), "il Reply deve riportare il decode originale");
}

#[tokio::test]
async fn e2e_halt_reaches_jtdx_by_default() {
    let r = rig(false, vec![]).await;
    let cookie = cookie_for(&r.server);
    r.send(&fixture("status-decoding.bin")).await;
    tokio::time::sleep(Duration::from_millis(200)).await;
    let (status, _) = http(r.server.addr, "POST", "/api/v1/commands/halt-tx", Some(&cookie), None).await;
    assert_eq!(status, 200);
    let got = r.received(500).await;
    assert!(got.contains(&encode_halt_tx("JTDX", 2, false)));
}

#[tokio::test]
async fn e2e_command_from_forward_target_reaches_jtdx() {
    let target = UdpSocket::bind("127.0.0.1:0").await.unwrap();
    let r = rig(false, vec![target.local_addr().unwrap()]).await;
    r.send(&fixture("status-decoding.bin")).await;
    let mut buf = vec![0u8; 65_535];
    let (_, hub_side) = timeout(Duration::from_secs(1), target.recv_from(&mut buf)).await.unwrap().unwrap();
    let command = encode_halt_tx("JTDX", 2, true);
    target.send_to(&command, hub_side).await.unwrap();
    let got = r.received(500).await;
    assert!(got.contains(&command), "il comando di GridTracker2 deve arrivare a JTDX");
}
