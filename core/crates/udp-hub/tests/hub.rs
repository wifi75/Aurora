use std::{net::SocketAddr, time::Duration};

use tokio::{net::UdpSocket, time::timeout};
use udp_hub::{Hub, HubConfig, HubError};

const MAGIC: [u8; 4] = [0xad, 0xbc, 0xcb, 0xda];

fn packet(payload: &[u8]) -> Vec<u8> {
    let mut p = MAGIC.to_vec();
    p.extend_from_slice(payload);
    p
}

async fn socket() -> UdpSocket {
    UdpSocket::bind("127.0.0.1:0").await.unwrap()
}

async fn recv(s: &UdpSocket) -> Option<(Vec<u8>, SocketAddr)> {
    let mut buf = vec![0u8; 65_535];
    match timeout(Duration::from_millis(500), s.recv_from(&mut buf)).await {
        Ok(Ok((n, from))) => Some((buf[..n].to_vec(), from)),
        _ => None,
    }
}

async fn start(forward: Vec<SocketAddr>) -> Hub {
    Hub::start(HubConfig { listen: "127.0.0.1:0".parse().unwrap(), forward }).await.unwrap()
}

#[tokio::test]
async fn jtdx_packet_reaches_subscriber_and_forward_target_unchanged() {
    let target = socket().await;
    let hub = start(vec![target.local_addr().unwrap()]).await;
    let mut sub = hub.subscribe();
    let jtdx = socket().await;
    let data = packet(b"Status di prova");
    jtdx.send_to(&data, hub.local_addr()).await.unwrap();

    assert_eq!(timeout(Duration::from_secs(1), sub.recv()).await.unwrap().unwrap(), data);
    let (got, _) = recv(&target).await.expect("l'inoltro deve arrivare");
    assert_eq!(got, data, "l'inoltro deve essere identico byte per byte");
    assert_eq!(hub.jtdx_addr(), Some(jtdx.local_addr().unwrap()));
}

#[tokio::test]
async fn command_from_forward_target_reaches_jtdx_via_hub() {
    let target = socket().await;
    let hub = start(vec![target.local_addr().unwrap()]).await;
    let jtdx = socket().await;
    jtdx.send_to(&packet(b"hello"), hub.local_addr()).await.unwrap();
    let (_, hub_side) = recv(&target).await.expect("inoltro");

    // il programma a valle risponde all'indirizzo da cui ha ricevuto
    let command = packet(b"Reply di prova");
    target.send_to(&command, hub_side).await.unwrap();
    let (got, _) = recv(&jtdx).await.expect("il comando deve arrivare a JTDX");
    assert_eq!(got, command);
}

#[tokio::test]
async fn send_to_jtdx_without_peer_is_no_peer_error() {
    let hub = start(vec![]).await;
    assert!(matches!(hub.send_to_jtdx(&packet(b"x")).await, Err(HubError::NoPeer)));
}

#[tokio::test]
async fn send_to_jtdx_reaches_the_known_peer() {
    let hub = start(vec![]).await;
    let jtdx = socket().await;
    jtdx.send_to(&packet(b"hello"), hub.local_addr()).await.unwrap();
    tokio::time::sleep(Duration::from_millis(100)).await;
    let cmd = packet(b"HaltTx di prova");
    hub.send_to_jtdx(&cmd).await.unwrap();
    let (got, from) = recv(&jtdx).await.unwrap();
    assert_eq!(got, cmd);
    assert_eq!(from, hub.local_addr());
}

#[tokio::test]
async fn datagram_without_magic_is_dropped_and_not_forwarded() {
    let target = socket().await;
    let hub = start(vec![target.local_addr().unwrap()]).await;
    let mut sub = hub.subscribe();
    let stranger = socket().await;
    stranger.send_to(&[1, 2, 3], hub.local_addr()).await.unwrap();
    stranger.send_to(&[7u8; 100], hub.local_addr()).await.unwrap();
    stranger.send_to(&vec![0u8; 60_000], hub.local_addr()).await.unwrap();
    assert!(recv(&target).await.is_none(), "i datagrammi estranei non vanno inoltrati");
    assert!(timeout(Duration::from_millis(200), sub.recv()).await.is_err());
    assert_eq!(hub.dropped_count(), 3);
    assert_eq!(hub.jtdx_addr(), None, "un estraneo non diventa JTDX");
}

#[tokio::test]
async fn large_valid_datagram_is_handled() {
    let target = socket().await;
    let hub = start(vec![target.local_addr().unwrap()]).await;
    let jtdx = socket().await;
    let big = packet(&vec![9u8; 60_000]);
    jtdx.send_to(&big, hub.local_addr()).await.unwrap();
    let (got, _) = recv(&target).await.expect("deve arrivare anche un datagramma grande");
    assert_eq!(got.len(), big.len());
}

#[tokio::test]
async fn jtdx_restart_with_new_source_port_redirects_commands() {
    let hub = start(vec![]).await;
    let first = socket().await;
    first.send_to(&packet(b"a"), hub.local_addr()).await.unwrap();
    tokio::time::sleep(Duration::from_millis(100)).await;
    let second = socket().await;
    second.send_to(&packet(b"b"), hub.local_addr()).await.unwrap();
    tokio::time::sleep(Duration::from_millis(100)).await;
    assert_eq!(hub.jtdx_addr(), Some(second.local_addr().unwrap()));

    let cmd = packet(b"comando");
    hub.send_to_jtdx(&cmd).await.unwrap();
    assert_eq!(recv(&second).await.unwrap().0, cmd);
    assert!(recv(&first).await.is_none(), "il vecchio indirizzo non deve ricevere piu' nulla");
}

#[tokio::test]
async fn closed_forward_target_does_not_block_others() {
    let closed = {
        let s = socket().await;
        s.local_addr().unwrap() // il socket viene chiuso subito: la porta non ascolta piu'
    };
    let alive = socket().await;
    let hub = start(vec![closed, alive.local_addr().unwrap()]).await;
    let jtdx = socket().await;
    for i in 0..10u8 {
        jtdx.send_to(&packet(&[i]), hub.local_addr()).await.unwrap();
    }
    let mut got = 0;
    while let Some((d, _)) = recv(&alive).await {
        assert_eq!(d.len(), 5);
        got += 1;
        if got == 10 {
            break;
        }
    }
    assert_eq!(got, 10, "la destinazione chiusa non deve fermare le altre");
}

#[tokio::test]
async fn stop_releases_the_port() {
    let hub = start(vec![]).await;
    let addr = hub.local_addr();
    hub.stop();
    drop(hub);
    tokio::time::sleep(Duration::from_millis(200)).await;
    UdpSocket::bind(addr).await.expect("la porta deve essere di nuovo libera");
}
