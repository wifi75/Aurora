use std::{sync::Arc, time::Duration};

use aurora_core::{Bus, Event, QsoRecord};
use logbook::Logbook;
use qso_export::{Exporter, Target};
use tokio::{net::UdpSocket, time::timeout};

fn qso(call: &str) -> QsoRecord {
    QsoRecord {
        call: call.into(),
        grid: "FN42".into(),
        band: "20m".into(),
        mode: "FT8".into(),
        freq_hz: 14_074_000,
        rst_sent: "-10".into(),
        rst_rcvd: "-12".into(),
        time_ms: 45_015_000,
    }
}

#[tokio::test]
async fn a_logged_qso_is_saved_and_sent_as_adif_over_udp() {
    let bus = Bus::new(64);
    let book = Arc::new(Logbook::open_in_memory().unwrap());
    let sink = UdpSocket::bind("127.0.0.1:0").await.unwrap();
    let exporter = Exporter::spawn(&bus, Arc::clone(&book), vec![Target::UdpAdif(sink.local_addr().unwrap())], Arc::new(|| 1_000_000));
    bus.events().send(Event::QsoLogged(qso("AT1AA"))).unwrap();
    let mut buf = vec![0u8; 2048];
    let (n, _) = timeout(Duration::from_secs(1), sink.recv_from(&mut buf)).await.unwrap().unwrap();
    let text = String::from_utf8_lossy(&buf[..n]).to_string();
    assert!(text.contains("<CALL:5>AT1AA") && text.contains("<EOR>"), "{text}");
    assert_eq!(book.count().unwrap(), 1);
    assert_eq!(exporter.stats().sent, 1);
}

#[tokio::test]
async fn the_file_target_appends_records_after_a_single_header() {
    let bus = Bus::new(64);
    let book = Arc::new(Logbook::open_in_memory().unwrap());
    let path = std::env::temp_dir().join(format!("aurora-export-{}.adi", std::process::id()));
    let _ = std::fs::remove_file(&path);
    let exporter = Exporter::spawn(&bus, book, vec![Target::File(path.clone())], Arc::new(|| 1_000_000));
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
    assert_eq!(text.matches("<EOH>").count(), 1);
    assert_eq!(text.matches("<EOR>").count(), 2);
}

#[tokio::test]
async fn a_broken_target_is_counted_and_does_not_stop_the_others() {
    let bus = Bus::new(64);
    let book = Arc::new(Logbook::open_in_memory().unwrap());
    let sink = UdpSocket::bind("127.0.0.1:0").await.unwrap();
    let bad = Target::File("/non/esiste/cartella/log.adi".into());
    let exporter = Exporter::spawn(&bus, book, vec![bad, Target::UdpAdif(sink.local_addr().unwrap())], Arc::new(|| 1_000_000));
    bus.events().send(Event::QsoLogged(qso("AT1AA"))).unwrap();
    let mut buf = vec![0u8; 2048];
    timeout(Duration::from_secs(1), sink.recv_from(&mut buf)).await.unwrap().unwrap();
    let stats = exporter.stats();
    assert_eq!((stats.sent, stats.failed), (1, 1));
    assert!(stats.last_error.is_some());
}
