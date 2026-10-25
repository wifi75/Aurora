use std::{sync::Arc, time::Duration};

use aurora_core::{Bus, Event, QsoRecord};
use logbook::{adif::Station, Logbook};
use qso_export::{Exporter, Target};
use tokio::{net::UdpSocket, time::timeout};

/// 2026-10-24 16:46:45 UTC
const ON_MS: u64 = 1_792_860_405_000;

fn qso(call: &str) -> QsoRecord {
    QsoRecord {
        call: call.into(),
        grid: "FN42".into(),
        band: "20m".into(),
        mode: "FT8".into(),
        freq_hz: 14_075_084,
        rst_sent: "-10".into(),
        rst_rcvd: "-12".into(),
        time_ms: 0,
        time_on_unix_ms: ON_MS,
        time_off_unix_ms: ON_MS + 44_000,
    }
}

fn station() -> Station {
    Station { call: "IU3ABC".into(), grid: "IO91wm".into(), tx_power: String::new() }
}

#[tokio::test]
async fn a_logged_qso_is_saved_and_sent_as_the_jtdx_adif_record_over_udp() {
    let bus = Bus::new(64);
    let book = Arc::new(Logbook::open_in_memory().unwrap());
    let sink = UdpSocket::bind("127.0.0.1:0").await.unwrap();
    let exporter = Exporter::spawn(&bus, Arc::clone(&book), vec![Target::UdpAdif(sink.local_addr().unwrap())], station(), Arc::new(|| 1_000_000));
    bus.events().send(Event::QsoLogged(qso("AT1AA"))).unwrap();
    let mut buf = vec![0u8; 2048];
    let (n, _) = timeout(Duration::from_secs(1), sink.recv_from(&mut buf)).await.unwrap().unwrap();
    let text = String::from_utf8_lossy(&buf[..n]).to_string();
    assert_eq!(
        text,
        "<BAND:3>20m <STATION_CALLSIGN:6>IU3ABC <MY_GRIDSQUARE:6>IO91wm <CALL:5>AT1AA <FREQ:9>14.075084 <MODE:3>FT8 \
         <QSO_DATE:8>20261024 <TIME_ON:6>164645 <QSO_DATE_OFF:8>20261024 <TIME_OFF:6>164729 \
         <RST_SENT:3>-10 <RST_RCVD:3>-12 <GRIDSQUARE:4>FN42 <EOR>"
    );
    assert_eq!(book.count().unwrap(), 1);
    assert_eq!(exporter.stats().sent, 1);
    // un solo datagramma per QSO: niente doppioni verso Log4OM2
    assert!(timeout(Duration::from_millis(300), sink.recv_from(&mut buf)).await.is_err(), "secondo datagramma inatteso");
}

#[tokio::test]
async fn the_same_call_logged_again_within_two_periods_is_not_a_new_qso() {
    let bus = Bus::new(64);
    let book = Arc::new(Logbook::open_in_memory().unwrap());
    let sink = UdpSocket::bind("127.0.0.1:0").await.unwrap();
    let exporter = Exporter::spawn(&bus, Arc::clone(&book), vec![Target::UdpAdif(sink.local_addr().unwrap())], station(), Arc::new(|| 1_000_000));
    let mut again = qso("AT1AA");
    again.time_off_unix_ms += 15_000;
    let mut later = qso("AT1AA");
    later.time_off_unix_ms += 60_000;
    for q in [qso("AT1AA"), again, qso("LM1XY"), later] {
        bus.events().send(Event::QsoLogged(q)).unwrap();
    }
    let mut buf = vec![0u8; 2048];
    for _ in 0..3 {
        timeout(Duration::from_secs(1), sink.recv_from(&mut buf)).await.unwrap().unwrap();
    }
    assert!(timeout(Duration::from_millis(300), sink.recv_from(&mut buf)).await.is_err(), "il doppione e' stato inviato");
    assert_eq!(book.count().unwrap(), 3);
    assert_eq!(exporter.stats().sent, 3);
}

#[tokio::test]
async fn the_file_target_appends_wsjtx_log_lines_after_a_single_header() {
    let bus = Bus::new(64);
    let book = Arc::new(Logbook::open_in_memory().unwrap());
    let path = std::env::temp_dir().join(format!("aurora-export-{}.adi", std::process::id()));
    let _ = std::fs::remove_file(&path);
    let exporter = Exporter::spawn(&bus, book, vec![Target::File(path.clone())], station(), Arc::new(|| 1_000_000));
    bus.events().send(Event::QsoLogged(qso("AT1AA"))).unwrap();
    bus.events().send(Event::QsoLogged(qso("LM1XY"))).unwrap();
    for _ in 0..50 {
        if exporter.stats().sent >= 2 {
            break;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    let text = std::fs::read_to_string(&path).unwrap();
    let _ = std::fs::remove_file(&path);
    assert!(text.starts_with("WSJT-X ADIF Export<eoh>\r\n<call:5>AT1AA <gridsquare:4>FN42 <mode:3>FT8 "), "{text}");
    assert_eq!(text.matches("<eoh>").count(), 1);
    assert_eq!(text.matches("<eor>\r\n").count(), 2);
}

#[tokio::test]
async fn a_broken_target_is_counted_and_does_not_stop_the_others() {
    let bus = Bus::new(64);
    let book = Arc::new(Logbook::open_in_memory().unwrap());
    let sink = UdpSocket::bind("127.0.0.1:0").await.unwrap();
    let bad = Target::File("/non/esiste/cartella/log.adi".into());
    let exporter = Exporter::spawn(&bus, book, vec![bad, Target::UdpAdif(sink.local_addr().unwrap())], station(), Arc::new(|| 1_000_000));
    bus.events().send(Event::QsoLogged(qso("AT1AA"))).unwrap();
    let mut buf = vec![0u8; 2048];
    timeout(Duration::from_secs(1), sink.recv_from(&mut buf)).await.unwrap().unwrap();
    let stats = exporter.stats();
    assert_eq!((stats.sent, stats.failed), (1, 1));
    assert!(stats.last_error.is_some());
}

#[tokio::test]
async fn the_targets_can_be_changed_while_running_and_each_qso_uses_the_list_of_that_moment() {
    let bus = Bus::new(64);
    let book = Arc::new(Logbook::open_in_memory().unwrap());
    let (first, second) = (UdpSocket::bind("127.0.0.1:0").await.unwrap(), UdpSocket::bind("127.0.0.1:0").await.unwrap());
    let targets = Arc::new(std::sync::RwLock::new(vec![Target::UdpAdif(first.local_addr().unwrap())]));
    let _exporter = Exporter::spawn_shared(&bus, book, Arc::clone(&targets), station(), Arc::new(|| 1_000_000));
    let mut buf = vec![0u8; 2048];
    bus.events().send(Event::QsoLogged(qso("AT1AA"))).unwrap();
    timeout(Duration::from_secs(1), first.recv_from(&mut buf)).await.unwrap().unwrap();
    // il destinatario cambia senza riavviare niente
    *targets.write().unwrap() = vec![Target::UdpAdif(second.local_addr().unwrap())];
    bus.events().send(Event::QsoLogged(qso("LM1XY"))).unwrap();
    timeout(Duration::from_secs(1), second.recv_from(&mut buf)).await.unwrap().unwrap();
    assert!(timeout(Duration::from_millis(300), first.recv_from(&mut buf)).await.is_err(), "il vecchio destinatario non riceve piu' niente");
    // lista vuota (invio spento): il QSO si salva ma non parte nulla
    targets.write().unwrap().clear();
    bus.events().send(Event::QsoLogged(qso("DL1ZZ"))).unwrap();
    assert!(timeout(Duration::from_millis(300), second.recv_from(&mut buf)).await.is_err());
}
