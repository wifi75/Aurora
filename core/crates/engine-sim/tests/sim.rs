use std::time::Duration;

use aurora_core::{Bus, Command, Engine, EngineIo, Event};
use engine_sim::{SimConfig, SimEngine};
use tokio::task::JoinHandle;

fn start(bus: &Bus, seed: u64) -> JoinHandle<()> {
    let io = EngineIo {
        events: bus.events(),
        spectrum: bus.spectrum(),
        commands: bus.take_command_rx().expect("receiver disponibile"),
    };
    tokio::spawn(SimEngine::new(SimConfig { seed, cycle_ms: 15_000 }).run(io))
}

fn drain(rx: &mut tokio::sync::broadcast::Receiver<Event>) -> Vec<Event> {
    let mut out = Vec::new();
    while let Ok(e) = rx.try_recv() {
        out.push(e);
    }
    out
}

fn decode_texts(events: &[Event]) -> Vec<String> {
    events
        .iter()
        .filter_map(|e| match e {
            Event::Decode(d) => Some(d.text.clone()),
            _ => None,
        })
        .collect()
}

#[tokio::test(start_paused = true)]
async fn emits_status_on_start() {
    let bus = Bus::new(4096);
    let mut rx = bus.subscribe_events();
    let h = start(&bus, 1);
    let first = tokio::time::timeout(Duration::from_secs(1), rx.recv()).await.unwrap().unwrap();
    match first {
        Event::Status(s) => {
            assert_eq!(s.dial_hz, 14_074_000);
            assert_eq!(s.mode, "FT8");
            assert_eq!(s.de_call, "IU3CYV");
            assert_eq!(s.de_grid, "JN55");
            assert!(!s.transmitting);
        }
        other => panic!("atteso Status, trovato {other:?}"),
    }
    h.abort();
}

#[tokio::test(start_paused = true)]
async fn emits_spectrum_rows_with_1024_bins_in_range() {
    let bus = Bus::new(4096);
    let mut rx = bus.subscribe_spectrum();
    let h = start(&bus, 1);
    tokio::time::sleep(Duration::from_millis(500)).await;
    let mut rows = 0;
    let mut last_seq = None;
    while let Ok(row) = rx.try_recv() {
        assert_eq!(row.bins.len(), 1024);
        assert_eq!((row.f_min_hz, row.f_max_hz), (0, 3000));
        assert!(row.validate().is_ok());
        if let Some(prev) = last_seq {
            assert_eq!(row.seq, prev + 1, "seq deve crescere di uno");
        }
        last_seq = Some(row.seq);
        rows += 1;
    }
    assert!(rows >= 5, "righe ricevute: {rows}");
    h.abort();
}

#[tokio::test(start_paused = true)]
async fn emits_decodes_each_cycle() {
    let bus = Bus::new(8192);
    let mut rx = bus.subscribe_events();
    let h = start(&bus, 1);
    tokio::time::sleep(Duration::from_millis(31_000)).await;
    let events = drain(&mut rx);
    let n = decode_texts(&events).len();
    assert!((15..=36).contains(&n), "decode in tre cicli: {n}");
    let statuses = events.iter().filter(|e| matches!(e, Event::Status(_))).count();
    assert!(statuses >= 3, "status: {statuses}");
    h.abort();
}

#[tokio::test(start_paused = true)]
async fn same_seed_same_decode_texts() {
    let run = |seed: u64| async move {
        let bus = Bus::new(8192);
        let mut rx = bus.subscribe_events();
        let h = start(&bus, seed);
        tokio::time::sleep(Duration::from_millis(16_000)).await;
        h.abort();
        decode_texts(&drain(&mut rx))
    };
    let a = run(42).await;
    let b = run(42).await;
    let c = run(43).await;
    assert!(!a.is_empty());
    assert_eq!(a, b);
    assert_ne!(a, c);
}

#[tokio::test(start_paused = true)]
async fn halt_tx_emits_tx_halted_and_clears_transmitting() {
    let bus = Bus::new(8192);
    let mut rx = bus.subscribe_events();
    let h = start(&bus, 1);
    tokio::time::sleep(Duration::from_millis(100)).await;
    let id = drain(&mut rx)
        .into_iter()
        .find_map(|e| if let Event::Decode(d) = e { Some(d.id) } else { None })
        .expect("almeno un decode");
    bus.commands().send(Command::Reply { decode_id: id }).await.unwrap();
    tokio::time::sleep(Duration::from_millis(100)).await;
    let after_reply = drain(&mut rx);
    assert!(after_reply.iter().any(|e| matches!(e, Event::Status(s) if s.transmitting)));
    bus.commands().send(Command::HaltTx).await.unwrap();
    tokio::time::sleep(Duration::from_millis(100)).await;
    let after_halt = drain(&mut rx);
    assert!(after_halt.contains(&Event::TxHalted));
    assert!(after_halt.iter().any(|e| matches!(e, Event::Status(s) if !s.transmitting)));
    h.abort();
}

#[tokio::test(start_paused = true)]
async fn reply_unknown_id_is_ignored_without_panic() {
    let bus = Bus::new(8192);
    let mut rx = bus.subscribe_events();
    let h = start(&bus, 1);
    tokio::time::sleep(Duration::from_millis(100)).await;
    drain(&mut rx);
    bus.commands().send(Command::Reply { decode_id: 999_999 }).await.unwrap();
    bus.commands().send(Command::HaltTx).await.unwrap();
    tokio::time::sleep(Duration::from_millis(100)).await;
    let events = drain(&mut rx);
    assert!(events.contains(&Event::TxHalted), "il motore deve essere ancora vivo");
    assert!(!events.iter().any(|e| matches!(e, Event::Status(s) if s.transmitting)));
    assert!(!h.is_finished());
    h.abort();
}
