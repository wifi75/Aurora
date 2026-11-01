//! ROS alto: un finto rigctld che in trasmissione legge SWR 3,0 ferma la Tx (soglia di fabbrica 2,5 come WSJT-X) e lascia l'avviso.

use std::{
    sync::{
        Arc,
        atomic::{AtomicU32, Ordering},
    },
    time::Duration,
};

use engine_native::NativeControls;
use rig_control::{
    Rig,
    cat_config::RigConfig,
    fake_rigctld::FakeRigctld,
    hamlib::RigctldClient,
    hamlib_rig::{Connector, HamlibRig, Link, Timing},
};

fn fast() -> Timing {
    Timing {
        poll: Duration::from_millis(30),
        tx_poll_min: Duration::from_millis(10),
        quick_reads: [Duration::from_millis(10), Duration::from_millis(20)],
        startup_block: Duration::from_millis(250),
        ptt_pause: Duration::from_millis(5),
        qsy_check: Duration::from_millis(20),
        reconnect_step: Duration::from_millis(40),
        reconnect_cap: Duration::from_millis(200),
        max_reconnects: 5,
        tolerate: 3,
        tolerate_io: 8,
    }
}

#[test]
fn an_swr_of_3_read_from_the_radio_while_transmitting_stops_the_tx() {
    let fake = FakeRigctld::start(14_074_000);
    fake.set_level("RFPOWER_METER_WATTS", "50");
    fake.set_level("SWR", "3.0");
    let addr = fake.addr;
    let connector: Connector = Arc::new(move || RigctldClient::connect_with_timeout(addr, Duration::from_secs(1), Duration::from_millis(500)).map(Link::from_client));
    let rig = HamlibRig::start(RigConfig::default(), connector, fast());
    rig.wait_settled(Duration::from_secs(3));
    std::thread::sleep(Duration::from_millis(300));

    let controls = NativeControls::default();
    let halts = Arc::new(AtomicU32::new(0));
    let h = Arc::clone(&halts);
    controls.set_halt_now(Arc::new(move || {
        h.fetch_add(1, Ordering::Relaxed);
    }));
    assert_eq!(controls.swr_limit(), Some(2.5), "soglia di fabbrica");

    // il ciclo degli strumenti di Aurora: la catena dice che trasmette, il CAT legge i livelli di trasmissione
    controls.transmitting.store(true, Ordering::Relaxed);
    rig.set_external_tx(true);
    rig.set_ptt(true).expect("PTT");
    let mut seen = None;
    for _ in 0..20 {
        std::thread::sleep(Duration::from_millis(60));
        let meters = Rig::meters(&rig).expect("misuratori");
        seen = meters.swr;
        controls.set_meters(Some(meters));
        if halts.load(Ordering::Relaxed) > 0 {
            break;
        }
    }
    assert_eq!(seen, Some(3.0));
    assert!(halts.load(Ordering::Relaxed) >= 1, "la trasmissione si ferma");
    let alert = controls.tx_alerts().into_iter().find(|a| a.kind == aurora_core::TxAlertKind::HighSwr).expect("avviso del ROS");
    assert_eq!((alert.value, alert.limit), (Some(3.0), Some(2.5)));
    assert_eq!(controls.published_meters().and_then(|m| m.power_w), Some(50.0), "la potenza resta visibile");
    rig.shutdown();
}
