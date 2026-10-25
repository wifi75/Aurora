//! Gli invii verso l'esterno contro server locali: PSK Reporter (UDP), server TCP, eQSL e DXSummit (HTTP). Nessuna prova esce dal PC.

use std::{net::SocketAddr, sync::Arc, time::Duration};

use aurora_core::{Bus, DecodedMessage, Event, QsoRecord, StationStatus};
use reporting::{secret::MemorySecrets, secret::SecretStore, status::ReportStatus, Config, Endpoints, EqslAccount, Service, EQSL_SECRET};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, UdpSocket},
    sync::mpsc,
    time::timeout,
};

// 1 792 860 405 s = 2026-10-24 16:46:45 UTC
const NOW: u64 = 1_792_860_405;

fn now() -> Arc<dyn Fn() -> u64 + Send + Sync> {
    Arc::new(|| NOW)
}

fn day_ms(secs: u64) -> u32 {
    ((secs % 86_400) * 1000) as u32
}

fn config() -> Config {
    Config { call: "IU3ABC".into(), grid: "JN45".into(), tx_power: String::new(), antenna: "Dipolo".into(), ..Config::default() }
}

fn decode(text: &str, time_ms: u32, hint: bool) -> Event {
    Event::Decode(DecodedMessage { id: 1, time_ms, snr: -12, delta_time: 0.2, freq_hz: 1234, text: text.into(), hint, ..DecodedMessage::default() })
}

fn status(dial: u64) -> Event {
    Event::Status(StationStatus { dial_hz: dial, mode: "FT8".into(), ..StationStatus::default() })
}

fn qso(call: &str) -> QsoRecord {
    QsoRecord {
        call: call.into(),
        grid: "FN42".into(),
        band: "20m".into(),
        mode: "FT8".into(),
        freq_hz: 14_075_084,
        rst_sent: "-16".into(),
        rst_rcvd: "-09".into(),
        time_on_unix_ms: 1_792_860_405_000,
        time_off_unix_ms: 1_792_860_449_000,
        ..QsoRecord::default()
    }
}

struct Rig {
    bus: Bus,
    service: Arc<Service>,
    status: Arc<ReportStatus>,
    secrets: Arc<MemorySecrets>,
}

fn rig(endpoints: Endpoints, initial: Config) -> Rig {
    let bus = Bus::new(256);
    let status = Arc::new(ReportStatus::new());
    let secrets = Arc::new(MemorySecrets::default());
    let service = Service::spawn(&bus, Arc::clone(&status), Arc::clone(&secrets) as Arc<dyn SecretStore>, endpoints, now(), initial);
    Rig { bus, service, status, secrets }
}

async fn settle() {
    tokio::time::sleep(Duration::from_millis(150)).await;
}

/// Un piccolo server HTTP: manda al canale la prima riga della richiesta e risponde con `body`.
async fn http_server(status_line: &'static str, body: &'static str) -> (String, mpsc::UnboundedReceiver<String>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("server");
    let addr = listener.local_addr().expect("indirizzo");
    let (tx, rx) = mpsc::unbounded_channel();
    tokio::spawn(async move {
        loop {
            let Ok((mut stream, _)) = listener.accept().await else { break };
            let tx = tx.clone();
            tokio::spawn(async move {
                let mut buf = vec![0u8; 16_384];
                let n = stream.read(&mut buf).await.unwrap_or(0);
                let text = String::from_utf8_lossy(&buf[..n]).to_string();
                let _ = tx.send(text.lines().next().unwrap_or("").to_string());
                let response = format!("HTTP/1.1 {status_line}\r\nContent-Type: text/html\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len());
                let _ = stream.write_all(response.as_bytes()).await;
                let _ = stream.shutdown().await;
            });
        }
    });
    (format!("http://{addr}"), rx)
}

fn psk_endpoints(addr: SocketAddr) -> Endpoints {
    Endpoints { psk: addr.to_string(), psk_every: Duration::from_millis(300), ..Endpoints::default() }
}

fn contains(haystack: &[u8], needle: &[u8]) -> bool {
    haystack.windows(needle.len()).any(|w| w == needle)
}

#[tokio::test]
async fn psk_reporter_receives_the_spots_in_the_ipfix_format_every_few_minutes() {
    let psk = UdpSocket::bind("127.0.0.1:0").await.expect("socket");
    let r = rig(psk_endpoints(psk.local_addr().unwrap()), Config { psk: true, ..config() });
    r.bus.events().send(status(14_074_000)).unwrap();
    settle().await;
    let next_slot = day_ms(NOW) / 15_000 * 15_000 + 15_000;
    // il periodo del cambio e' bloccato; poi uno spot buono, uno «ipotizzato», un messaggio senza locatore, uno non standard
    r.bus.events().send(decode("CQ K1ABC FN42", day_ms(NOW) / 15_000 * 15_000, false)).unwrap();
    r.bus.events().send(decode("CQ DL1ABC JO62", next_slot, false)).unwrap();
    r.bus.events().send(decode("CQ JA1ABC PM95", next_slot, true)).unwrap();
    r.bus.events().send(decode("IU3ABC EA1ABC -10", next_slot, false)).unwrap();
    r.bus.events().send(decode("<...> F5ABC JN18", next_slot, false)).unwrap();
    let mut buf = vec![0u8; 4096];
    let (n, _) = timeout(Duration::from_secs(3), psk.recv_from(&mut buf)).await.expect("pacchetto").unwrap();
    let packet = &buf[..n];
    assert_eq!(&packet[..2], &[0x00, 0x0A], "versione IPFIX");
    assert_eq!(u16::from_be_bytes([packet[2], packet[3]]) as usize, n, "lunghezza");
    assert!(contains(packet, b"\x06IU3ABC\x04JN45"), "il ricevitore: nominativo e locatore");
    assert!(contains(packet, b"\x06Dipolo"), "l'antenna");
    assert!(contains(packet, b"Aurora v"), "il programma");
    // il trasmettitore: DL1ABC a 14 074 000 + 1234 Hz
    let spot = [&[6u8][..], b"DL1ABC", &(14_075_234u32).to_be_bytes(), &[(-12i8) as u8, 3], b"FT8", &[4], b"JO62", &[1], &(NOW as u32).to_be_bytes()].concat();
    assert!(contains(packet, &spot), "il record dello spot");
    assert!(!contains(packet, b"K1ABC"), "il periodo del cambio di banda non si segnala");
    assert!(!contains(packet, b"JA1ABC"), "i decode ipotizzati non si segnalano");
    assert!(!contains(packet, b"EA1ABC") && !contains(packet, b"F5ABC"));
    settle().await;
    let st = r.status.get("psk");
    assert_eq!((st.ok, st.sent), (Some(true), 1), "{st:?}");
    // un secondo periodo senza spot non manda niente
    assert!(timeout(Duration::from_millis(700), psk.recv_from(&mut buf)).await.is_err());
}

#[tokio::test]
async fn psk_reporter_off_sends_and_queues_nothing_and_it_can_be_turned_on_while_running() {
    let psk = UdpSocket::bind("127.0.0.1:0").await.expect("socket");
    let r = rig(psk_endpoints(psk.local_addr().unwrap()), config());
    r.bus.events().send(status(14_074_000)).unwrap();
    settle().await;
    let slot = day_ms(NOW) / 15_000 * 15_000 + 15_000;
    r.bus.events().send(decode("CQ DL1ABC JO62", slot, false)).unwrap();
    let mut buf = vec![0u8; 4096];
    assert!(timeout(Duration::from_millis(800), psk.recv_from(&mut buf)).await.is_err(), "spento: niente invio");
    r.service.set_config(Config { psk: true, ..config() });
    settle().await;
    r.bus.events().send(decode("CQ DL2ABC JO62", slot, false)).unwrap();
    let (n, _) = timeout(Duration::from_secs(3), psk.recv_from(&mut buf)).await.expect("pacchetto").unwrap();
    assert!(contains(&buf[..n], b"DL2ABC") && !contains(&buf[..n], b"DL1ABC"), "quello che e' arrivato da spento non si accoda");
    // spegnendo, quello che e' in coda si butta
    r.service.set_config(config());
    settle().await;
    r.bus.events().send(decode("CQ DL3ABC JO62", slot, false)).unwrap();
    assert!(timeout(Duration::from_millis(800), psk.recv_from(&mut buf)).await.is_err());
}

#[tokio::test]
async fn psk_without_call_or_grid_reports_the_reason_and_sends_nothing() {
    let psk = UdpSocket::bind("127.0.0.1:0").await.expect("socket");
    let r = rig(psk_endpoints(psk.local_addr().unwrap()), Config { psk: true, grid: String::new(), ..config() });
    r.bus.events().send(status(14_074_000)).unwrap();
    settle().await;
    r.bus.events().send(decode("CQ DL1ABC JO62", day_ms(NOW) / 15_000 * 15_000 + 15_000, false)).unwrap();
    tokio::time::sleep(Duration::from_millis(900)).await;
    let st = r.status.get("psk");
    assert_eq!(st.ok, Some(false));
    assert!(st.message.contains("locatore"), "{}", st.message);
    let mut buf = vec![0u8; 4096];
    assert!(timeout(Duration::from_millis(300), psk.recv_from(&mut buf)).await.is_err());
}

#[tokio::test]
async fn the_tcp_server_receives_the_log_command_for_every_qso_and_a_nak_is_an_error() {
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("server");
    let addr = listener.local_addr().unwrap();
    let (tx, mut rx) = mpsc::unbounded_channel();
    tokio::spawn(async move {
        let mut count = 0;
        loop {
            let Ok((mut stream, _)) = listener.accept().await else { break };
            let mut buf = vec![0u8; 4096];
            let n = stream.read(&mut buf).await.unwrap_or(0);
            let _ = tx.send(String::from_utf8_lossy(&buf[..n]).to_string());
            count += 1;
            let _ = stream.write_all(if count == 1 { b"ACK" } else { b"NAK bad" }).await;
        }
    });
    let r = rig(Endpoints::default(), Config { tcp: Some(addr), ..config() });
    r.bus.events().send(Event::QsoLogged(qso("K1ABC"))).unwrap();
    let text = timeout(Duration::from_secs(3), rx.recv()).await.expect("messaggio").unwrap();
    assert!(text.starts_with("<command:3>Log <parameters:"), "{text}");
    assert!(text.contains("<BAND:3>20m <STATION_CALLSIGN:6>IU3ABC <MY_GRIDSQUARE:4>JN45 <CALL:5>K1ABC"), "{text}");
    assert!(text.ends_with("<EOR> "), "il record porta lo spazio finale: {text:?}");
    let declared: usize = text.split("<parameters:").nth(1).unwrap().split('>').next().unwrap().parse().unwrap();
    assert_eq!(declared, text.split("> ").skip(1).collect::<Vec<_>>().join("> ").len(), "la lunghezza dichiarata e' quella del record");
    settle().await;
    assert_eq!(r.status.get("tcp").ok, Some(true));
    // un secondo QSO: il programma risponde NAK
    r.bus.events().send(Event::QsoLogged(QsoRecord { time_off_unix_ms: 1_792_860_999_000, ..qso("DL1ABC") })).unwrap();
    timeout(Duration::from_secs(3), rx.recv()).await.expect("secondo messaggio");
    settle().await;
    let st = r.status.get("tcp");
    assert_eq!(st.ok, Some(false));
    assert!(st.message.contains("rifiutato"), "{}", st.message);
    // lo stesso QSO subito dopo non si manda due volte
    r.bus.events().send(Event::QsoLogged(QsoRecord { time_off_unix_ms: 1_792_861_000_000, ..qso("DL1ABC") })).unwrap();
    assert!(timeout(Duration::from_millis(500), rx.recv()).await.is_err());
}

#[tokio::test]
async fn an_unreachable_tcp_server_is_an_error_in_the_state_and_never_blocks_the_engine() {
    // una porta chiusa: si prende una porta libera e si chiude
    let closed = {
        let l = TcpListener::bind("127.0.0.1:0").await.unwrap();
        l.local_addr().unwrap()
    };
    let psk = UdpSocket::bind("127.0.0.1:0").await.expect("socket");
    let r = rig(psk_endpoints(psk.local_addr().unwrap()), Config { tcp: Some(closed), psk: true, ..config() });
    r.bus.events().send(status(14_074_000)).unwrap();
    settle().await;
    r.bus.events().send(Event::QsoLogged(qso("K1ABC"))).unwrap();
    r.bus.events().send(decode("CQ DL1ABC JO62", day_ms(NOW) / 15_000 * 15_000 + 15_000, false)).unwrap();
    let mut buf = vec![0u8; 4096];
    timeout(Duration::from_secs(3), psk.recv_from(&mut buf)).await.expect("PSK Reporter parte lo stesso").unwrap();
    tokio::time::sleep(Duration::from_millis(2500)).await;
    let st = r.status.get("tcp");
    assert_eq!(st.ok, Some(false), "{st:?}");
}

#[tokio::test]
async fn eqsl_gets_the_adif_with_the_protected_password_and_nothing_secret_reaches_the_state() {
    let (url, mut requests) = http_server("200 OK", "<html>Result: 1 out of 1 records added</html>").await;
    let endpoints = Endpoints { eqsl_url: format!("{url}/qslcard/importADIF.cfm"), eqsl_timeout: Duration::from_secs(5), ..Endpoints::default() };
    let r = rig(endpoints, Config { eqsl: Some(EqslAccount { user: "IU3ABC".into(), nick: "Casa".into() }), ..config() });
    r.secrets.set(EQSL_SECRET, "Pa55 segreta&1").unwrap();
    r.bus.events().send(Event::QsoLogged(QsoRecord { eqsl_comment: "TNX".into(), ..qso("K1ABC") })).unwrap();
    let line = timeout(Duration::from_secs(5), requests.recv()).await.expect("richiesta").unwrap();
    assert!(line.starts_with("GET /qslcard/importADIF.cfm?ADIFdata="), "{line}");
    let encoded = line.split("ADIFdata=").nth(1).unwrap().split(' ').next().unwrap();
    let decoded = percent_decode(encoded);
    assert!(decoded.starts_with("<ADIF_VER:5>2.1.9<EQSL_USER:6>IU3ABC<EQSL_PSWD:14>Pa55 segreta&1<PROGRAMID:6>Aurora<EOH><APP_EQSL_QTH_NICKNAME:4>Casa<CALL:5>K1ABC<MODE:3>FT8"), "{decoded}");
    assert!(decoded.contains("<QSLMSG:3>TNX<EOR>"), "{decoded}");
    tokio::time::sleep(Duration::from_millis(300)).await;
    let st = r.status.get("eqsl");
    assert_eq!(st.ok, Some(true), "{st:?}");
    assert!(!format!("{st:?}").contains("segreta") && !r.status.snapshot().to_string().contains("segreta"), "la password non e' mai nello stato");
}

#[tokio::test]
async fn eqsl_errors_are_reported_without_the_password() {
    // credenziali rifiutate
    let (url, _requests) = http_server("200 OK", "<html>Error: No match on eQSL_User/eQSL_Pswd</html>").await;
    let endpoints = Endpoints { eqsl_url: format!("{url}/x"), eqsl_timeout: Duration::from_secs(5), ..Endpoints::default() };
    let r = rig(endpoints, Config { eqsl: Some(EqslAccount { user: "IU3ABC".into(), nick: String::new() }), ..config() });
    r.secrets.set(EQSL_SECRET, "PasswordUnica9").unwrap();
    r.bus.events().send(Event::QsoLogged(qso("K1ABC"))).unwrap();
    tokio::time::sleep(Duration::from_millis(700)).await;
    let st = r.status.get("eqsl");
    assert_eq!(st.ok, Some(false));
    assert!(st.message.contains("rifiutato") && !st.message.contains("PasswordUnica9"), "{}", st.message);
    // server che non c'e': errore generico, senza indirizzo ne' password
    let dead = {
        let l = TcpListener::bind("127.0.0.1:0").await.unwrap();
        l.local_addr().unwrap()
    };
    r.service.set_config(Config { eqsl: Some(EqslAccount { user: "IU3ABC".into(), nick: String::new() }), ..config() });
    let r2 = rig(Endpoints { eqsl_url: format!("http://{dead}/x"), eqsl_timeout: Duration::from_secs(3), ..Endpoints::default() }, Config { eqsl: Some(EqslAccount::default()), ..config() });
    r2.secrets.set(EQSL_SECRET, "PasswordUnica9").unwrap();
    r2.bus.events().send(Event::QsoLogged(qso("DL1ABC"))).unwrap();
    tokio::time::sleep(Duration::from_millis(3500)).await;
    let st2 = r2.status.get("eqsl");
    assert_eq!(st2.ok, Some(false));
    assert!(!st2.message.contains("PasswordUnica9") && !st2.message.contains(&dead.to_string()), "{}", st2.message);
    // senza password salvata
    let r3 = rig(Endpoints::default(), Config { eqsl: Some(EqslAccount::default()), ..config() });
    r3.bus.events().send(Event::QsoLogged(qso("EA1ABC"))).unwrap();
    tokio::time::sleep(Duration::from_millis(400)).await;
    assert!(r3.status.get("eqsl").message.contains("password"), "{}", r3.status.get("eqsl").message);
}

#[tokio::test]
async fn a_hanging_eqsl_server_does_not_stop_the_other_sends() {
    // un server che accetta e non risponde mai
    let hang = TcpListener::bind("127.0.0.1:0").await.expect("server");
    let hang_addr = hang.local_addr().unwrap();
    tokio::spawn(async move {
        let mut keep = Vec::new();
        while let Ok((s, _)) = hang.accept().await {
            keep.push(s);
        }
    });
    let psk = UdpSocket::bind("127.0.0.1:0").await.expect("socket");
    let endpoints = Endpoints { eqsl_url: format!("http://{hang_addr}/x"), eqsl_timeout: Duration::from_secs(2), ..psk_endpoints(psk.local_addr().unwrap()) };
    let r = rig(endpoints, Config { eqsl: Some(EqslAccount::default()), psk: true, ..config() });
    r.secrets.set(EQSL_SECRET, "x").unwrap();
    r.bus.events().send(status(14_074_000)).unwrap();
    settle().await;
    r.bus.events().send(Event::QsoLogged(qso("K1ABC"))).unwrap();
    r.bus.events().send(decode("CQ DL1ABC JO62", day_ms(NOW) / 15_000 * 15_000 + 15_000, false)).unwrap();
    let mut buf = vec![0u8; 4096];
    timeout(Duration::from_millis(1500), psk.recv_from(&mut buf)).await.expect("lo spot parte mentre eQSL e' ancora in attesa").unwrap();
    // poi la richiesta scade, con un errore nello stato
    tokio::time::sleep(Duration::from_millis(2500)).await;
    assert_eq!(r.status.get("eqsl").ok, Some(false));
}

#[tokio::test]
async fn dxsummit_spots_a_station_only_when_asked_with_the_parameters_of_jtdx() {
    let (url, mut requests) = http_server("200 OK", "ok").await;
    let endpoints = Endpoints { dxsummit_url: format!("{url}/SendSpot.aspx"), dxsummit_timeout: Duration::from_secs(5), ..Endpoints::default() };
    let r = rig(endpoints, Config { dxsummit: true, ..config() });
    r.bus.events().send(decode("CQ DL1ABC JO62", 0, false)).unwrap();
    settle().await;
    assert!(requests.try_recv().is_err(), "una decodifica non fa partire nessuno spot");
    assert!(r.service.spot_dxsummit("dl1abc", 14_074_500, "FT8").await.is_ok());
    let line = requests.recv().await.unwrap();
    assert!(line.starts_with("GET /SendSpot.aspx?"), "{line}");
    for part in ["callSign=IU3ABC", "dxCallSign=DL1ABC", "frequency=14074.5", "info=FT8"] {
        assert!(line.contains(part), "{part} in {line}");
    }
    assert_eq!(r.status.get("dxsummit").ok, Some(true));
    // la stessa stazione subito dopo: no (una volta per stazione)
    assert!(r.service.spot_dxsummit("DL1ABC", 14_074_500, "FT8").await.unwrap_err().contains("gia'"));
    // un'altra si
    assert!(r.service.spot_dxsummit("K1ABC", 14_074_500, "FT8 test").await.is_ok());
    // spento: rifiutato
    r.service.set_config(config());
    assert!(r.service.spot_dxsummit("EA1ABC", 14_074_500, "FT8").await.unwrap_err().contains("spento"));
    // nominativo non valido
    r.service.set_config(Config { dxsummit: true, ..config() });
    assert!(r.service.spot_dxsummit("A B", 1, "FT8").await.is_err());
}

#[tokio::test]
async fn dxsummit_errors_are_reported_and_a_failed_spot_can_be_retried() {
    let (url, _r) = http_server("500 Internal Server Error", "boom").await;
    let endpoints = Endpoints { dxsummit_url: format!("{url}/SendSpot.aspx"), dxsummit_timeout: Duration::from_secs(5), ..Endpoints::default() };
    let r = rig(endpoints, Config { dxsummit: true, ..config() });
    let e = r.service.spot_dxsummit("DL1ABC", 14_074_000, "FT8").await.unwrap_err();
    assert!(e.contains("500"), "{e}");
    assert_eq!(r.status.get("dxsummit").ok, Some(false));
    let again = r.service.spot_dxsummit("DL1ABC", 14_074_000, "FT8").await.unwrap_err();
    assert!(!again.contains("gia'"), "un invio fallito non conta come segnalato: {again}");
}

fn percent_decode(s: &str) -> String {
    let b = s.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'%' && i + 2 < b.len() + 0 {
            out.push(u8::from_str_radix(&s[i + 1..i + 3], 16).unwrap_or(b'?'));
            i += 3;
        } else if b[i] == b'+' {
            out.push(b' ');
            i += 1;
        } else {
            out.push(b[i]);
            i += 1;
        }
    }
    String::from_utf8_lossy(&out).to_string()
}
