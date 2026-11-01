//! TxInhibit (18) e InhibitStatus (17) di WSJT-X 3.3 con un controller UDP finto: prenotazione, rinnovo, scadenza, rilascio, valori fuori
//! limite ignorati e contati; il PTT vero (`NoPtt`) si spegne e si riaccende dietro `tx_native::TxInterlock`.

use std::{net::SocketAddr, sync::Arc, time::Duration};

use aurora_core::Event;
use tokio::{
    net::UdpSocket,
    sync::{broadcast, mpsc},
};
use tx_native::{NoPtt, Ptt, TxInterlock};
use udp_bridge::{decode, encode_tx_inhibit, InhibitStatus, Message};
use udp_emitter::{Emitter, EmitterConfig, EmitterHooks, InhibitControl};

/// Lo stesso collegamento che fa Aurora tra l'emettitore e la catena di trasmissione.
struct Interlock(Arc<TxInterlock>);

impl InhibitControl for Interlock {
    fn command(&self, controller: &str, ttl_ms: u32, station: &str) {
        self.0.command(controller, ttl_ms, station);
    }

    fn invalid(&self, count: u32) {
        self.0.invalid(count);
    }

    fn tick(&self) {
        self.0.tick();
    }

    fn snapshot(&self) -> InhibitStatus {
        let s = self.0.snapshot();
        InhibitStatus { inhibited: s.inhibited, source_station: s.holder, hold_rx: s.hold_rx, release_rx: s.release_rx, expiries: s.expiries, invalid: s.invalid, ..Default::default() }
    }
}

struct Bench {
    controller: UdpSocket,
    emitter: SocketAddr,
    radio: Arc<NoPtt>,
    _emitter: Emitter,
    _events: broadcast::Sender<Event>,
}

async fn bench(accept: bool) -> Bench {
    let controller = UdpSocket::bind("127.0.0.1:0").await.expect("socket del controller");
    let addr = controller.local_addr().expect("indirizzo");
    let (events, rx) = broadcast::channel(16);
    let (cmd_tx, _commands) = mpsc::channel(16);
    let mut config = EmitterConfig::jtdx(vec![addr]);
    config.accept_requests = accept;
    let radio = Arc::new(NoPtt::new());
    let interlock = Arc::new(TxInterlock::new());
    interlock.attach(radio.clone());
    // la trasmissione e' in corso: il PTT e' chiesto acceso
    interlock.set(true).expect("ptt");
    let hooks = EmitterHooks { inhibit: Some(Arc::new(Interlock(interlock))), ..Default::default() };
    let emitter = Emitter::start_with(config, rx, cmd_tx, Arc::new(|| false), hooks).await.expect("emettitore");
    let local = emitter.local_addr();
    Bench { controller, emitter: local, radio, _emitter: emitter, _events: events }
}

/// Il prossimo InhibitStatus (gli altri datagrammi si saltano), entro `within`.
async fn next_status(socket: &UdpSocket, within: Duration) -> Option<InhibitStatus> {
    let mut buf = vec![0u8; 65_535];
    let deadline = tokio::time::Instant::now() + within;
    loop {
        let left = deadline.checked_duration_since(tokio::time::Instant::now())?;
        let (n, _) = tokio::time::timeout(left, socket.recv_from(&mut buf)).await.ok()?.ok()?;
        if let Ok(packet) = decode(&buf[..n])
            && let Message::InhibitStatus(s) = packet.message
        {
            return Some(s);
        }
    }
}

impl Bench {
    async fn send(&self, datagram: &[u8]) {
        self.controller.send_to(datagram, self.emitter).await.expect("invio");
    }

    async fn status(&self) -> InhibitStatus {
        next_status(&self.controller, Duration::from_secs(3)).await.expect("nessun InhibitStatus entro 3 s")
    }
}

#[tokio::test]
async fn a_controller_holds_renews_and_lets_the_ptt_go_at_expiry() {
    let b = bench(true).await;
    let first = b.status().await;
    assert_eq!((first.id.as_str(), first.supported, first.inhibited), ("JTDX", true, false), "all'avvio: supportato, libero");
    assert!(b.radio.is_on());

    b.send(&encode_tx_inhibit("JTDX", 2, "AMP-1", 400, Some("K1ABC amp"))).await;
    let held = b.status().await;
    assert_eq!((held.inhibited, held.source_station.as_str(), held.hold_rx), (true, "K1ABC amp", 1));
    assert!(!b.radio.is_on(), "la prenotazione spegne il PTT");

    // rinnovo: lo stato non cambia (stesso testo), resta bloccata oltre i primi 400 ms
    tokio::time::sleep(Duration::from_millis(250)).await;
    b.send(&encode_tx_inhibit("JTDX", 2, "AMP-1", 400, Some("K1ABC amp"))).await;
    let renewed = b.status().await;
    assert_eq!((renewed.inhibited, renewed.hold_rx), (true, 2));
    tokio::time::sleep(Duration::from_millis(250)).await;
    assert!(!b.radio.is_on(), "rinnovata, ancora bloccata");

    let expired = b.status().await;
    assert_eq!((expired.inhibited, expired.expiries), (false, 1), "scaduta");
    assert!(b.radio.is_on(), "il PTT torna come lo chiede la trasmissione");
}

#[tokio::test]
async fn a_release_frees_at_once_and_bad_values_are_ignored_and_counted() {
    let b = bench(true).await;
    b.status().await;
    b.send(&encode_tx_inhibit("JTDX", 3, "AMP-1", 30_000, None)).await;
    assert!(b.status().await.inhibited);
    // fuori limite: si ignorano e si contano; un altro identificativo si ignora senza contarlo
    for ttl in [99, 30_001] {
        b.send(&encode_tx_inhibit("JTDX", 2, "AMP-2", ttl, None)).await;
    }
    b.send(&encode_tx_inhibit("JTDX", 2, "CON SPAZI", 500, None)).await;
    b.send(&encode_tx_inhibit("ALTRO", 2, "AMP-3", 500, None)).await;
    let mut counted = b.status().await;
    while counted.invalid < 3 {
        counted = b.status().await;
    }
    assert_eq!((counted.invalid, counted.hold_rx, counted.source_station.as_str()), (3, 1, "AMP-1"));
    // il rilascio di un altro controller non toglie la prenotazione; quello del suo controller si'
    b.send(&encode_tx_inhibit("JTDX", 2, "AMP-9", 0, None)).await;
    let other = b.status().await;
    assert_eq!((other.inhibited, other.release_rx), (true, 1));
    b.send(&encode_tx_inhibit("JTDX", 2, "AMP-1", 0, None)).await;
    let released = b.status().await;
    assert_eq!((released.inhibited, released.release_rx, released.expiries), (false, 2, 0));
    assert!(b.radio.is_on());
}

#[tokio::test]
async fn without_accept_udp_requests_nothing_is_announced_or_applied() {
    let b = bench(false).await;
    b.send(&encode_tx_inhibit("JTDX", 2, "AMP-1", 30_000, None)).await;
    assert_eq!(next_status(&b.controller, Duration::from_millis(500)).await, None, "nessun InhibitStatus");
    assert!(b.radio.is_on(), "il comando non vale");
}
