//! Un QSO chiuso dalla sequenza arriva, come datagramma UDP e come riga del file ADIF, dove JTDX lo manderebbe (Log4OM2 sul secondo server UDP).

use std::{sync::Arc, time::Duration};

use aurora_core::{Bus, Event};
use engine_native::{NativeControls, TxCommand, TxDriver, TxPort};
use logbook::{adif::Station, Logbook};
use qso_export::{Exporter, Target};
use tokio::{net::UdpSocket, time::timeout};

/// La catena di trasmissione finta: accetta tutto e non fa nulla.
struct FakePort;

impl TxPort for FakePort {
    fn schedule(&self, _request: tx_native::TxRequest) -> Result<(), String> {
        Ok(())
    }

    fn halt(&self) {}
}

// 99 999 990 s dall'epoca: un periodo FT8 pari (1973-03-03 09:46:30 UTC)
const SLOT_MS: u64 = 99_999_990_000;

#[tokio::test]
async fn a_qso_closed_by_the_sequence_reaches_the_second_udp_server_once() {
    let mut driver = TxDriver::new("IU3ABC", "IO91", Arc::new(FakePort), Arc::new(NativeControls::default())).expect("pilota");
    let base = SLOT_MS as f64 / 1000.0;
    driver.handle(TxCommand::CallCq { modifier: None }, base + 14.5);
    // K1ABC risponde al CQ, manda il rapporto e chiude con RR73
    let steps: [(u64, &[(&str, i32)]); 4] = [
        (SLOT_MS + 15_000, &[("IU3ABC K1ABC FN42", -10)]),
        (SLOT_MS + 45_000, &[("IU3ABC K1ABC -07", -10)]),
        (SLOT_MS + 75_000, &[("IU3ABC K1ABC RR73", -10)]),
        (SLOT_MS + 105_000, &[]),
    ];
    let mut qsos = Vec::new();
    for (slot, msgs) in steps {
        let list: Vec<(String, i32, u32)> = msgs.iter().map(|(t, s)| (t.to_string(), *s, 1200)).collect();
        let (q, _) = driver.after_slot(slot, &list, slot as f64 / 1000.0 + 14.3, 14_074_000, "20m");
        qsos.extend(q);
    }
    assert_eq!(qsos.len(), 1, "un solo QSO chiuso");

    // il percorso dell'applicazione: l'evento sul bus, l'esportatore, il socket di Log4OM2 (qui un socket locale)
    let bus = Bus::new(64);
    let book = Arc::new(Logbook::open_in_memory().unwrap());
    let log4om = UdpSocket::bind("127.0.0.1:0").await.unwrap();
    let file = std::env::temp_dir().join(format!("aurora-log-{}.adi", std::process::id()));
    let _ = std::fs::remove_file(&file);
    let station = Station { call: "IU3ABC".into(), grid: "IO91wm".into(), tx_power: String::new() };
    let exporter = Exporter::spawn(&bus, Arc::clone(&book), vec![Target::UdpAdif(log4om.local_addr().unwrap()), Target::File(file.clone())], station, Arc::new(|| 100_000));
    for q in qsos {
        bus.events().send(Event::QsoLogged(q)).unwrap();
    }
    let mut buf = vec![0u8; 2048];
    let (n, _) = timeout(Duration::from_secs(2), log4om.recv_from(&mut buf)).await.expect("datagramma").unwrap();
    let text = String::from_utf8_lossy(&buf[..n]).to_string();

    // 99 999 990 s = 1973-03-03 09:46:30 UTC: il QSO comincia nel periodo del primo messaggio di K1ABC (09:46:45); il suo RR73 arriva nel periodo
    // delle 09:47:45, il 73 parte in quello delle 09:48:00 e JTDX scrive la fine 14 s dopo l'inizio della trasmissione (09:48:14)
    assert_eq!(
        text,
        "<BAND:3>20m <STATION_CALLSIGN:6>IU3ABC <MY_GRIDSQUARE:6>IO91wm <CALL:5>K1ABC <FREQ:9>14.075500 <MODE:3>FT8 <QSO_DATE:8>19730303 <TIME_ON:6>094645 <QSO_DATE_OFF:8>19730303 <TIME_OFF:6>094814 <RST_SENT:3>-10 <RST_RCVD:3>-07 <GRIDSQUARE:4>FN42 <EOR>"
    );
    assert_eq!(book.count().unwrap(), 1);
    assert!(timeout(Duration::from_millis(300), log4om.recv_from(&mut buf)).await.is_err(), "un solo datagramma per QSO");
    for _ in 0..50 {
        if exporter.stats().sent >= 2 {
            break;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    let written = std::fs::read_to_string(&file).unwrap();
    let _ = std::fs::remove_file(&file);
    assert!(written.starts_with("WSJT-X ADIF Export<eoh>\r\n<call:5>K1ABC <gridsquare:4>FN42 <mode:3>FT8 "), "{written}");
}
