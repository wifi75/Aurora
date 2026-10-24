//! Il task che collega il cervello al bus: cosa arriva, cosa parte.

use std::{
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
    time::Duration,
};

use aurora_core::{Bus, Command, DecodeTag, DecodedMessage, Event, QsoRecord};
use qso_engine::{Automation, AutomationConfig, DecisionKind, Limits, Mode, Rules};
use tokio::{sync::mpsc, time::timeout};

struct Rig {
    bus: Bus,
    commands: mpsc::Receiver<Command>,
    auto: Automation,
    clock: Arc<AtomicU64>,
}

fn rig(live_allowed: bool, limits: Limits) -> Rig {
    let bus = Bus::new(256);
    let commands = bus.take_command_rx().unwrap();
    let clock = Arc::new(AtomicU64::new(1_000_000));
    let now = Arc::clone(&clock);
    let auto = Automation::spawn(
        &bus,
        AutomationConfig {
            rules: Rules::default(),
            limits,
            live_allowed,
            tick_every: Duration::from_millis(20),
            now: Arc::new(move || now.load(Ordering::SeqCst)),
        },
    );
    Rig { bus, commands, auto, clock }
}

fn cq(id: u64, call: &str) -> Event {
    Event::Decode(DecodedMessage {
        id,
        time_ms: 0,
        snr: -8,
        delta_time: 0.0,
        freq_hz: 1500,
        text: format!("CQ {call} FN42"),
        tag: Some(DecodeTag::NewDxcc),
        country: None,
        continent: None,
    })
}

async fn settle() {
    tokio::time::sleep(Duration::from_millis(80)).await;
}

#[tokio::test]
async fn live_automation_sends_a_reply_command_for_a_wanted_cq() {
    let mut r = rig(true, Limits::default());
    r.auto.set_mode(Mode::Live).unwrap();
    r.auto.set_enabled(true);
    r.bus.events().send(cq(5, "AT1AA")).unwrap();
    let cmd = timeout(Duration::from_secs(1), r.commands.recv()).await.unwrap().unwrap();
    assert_eq!(cmd, Command::Reply { decode_id: 5 });
    assert_eq!(r.auto.state().replies_today, 1);
}

#[tokio::test]
async fn dry_run_never_sends_a_command_but_logs_the_decision() {
    let mut r = rig(true, Limits::default());
    r.auto.set_enabled(true);
    r.bus.events().send(cq(5, "AT1AA")).unwrap();
    settle().await;
    assert!(r.commands.try_recv().is_err());
    assert_eq!(r.auto.state().log.last().unwrap().kind, DecisionKind::WouldReply);
}

#[tokio::test]
async fn disabled_automation_does_nothing() {
    let mut r = rig(true, Limits::default());
    r.bus.events().send(cq(5, "AT1AA")).unwrap();
    settle().await;
    assert!(r.commands.try_recv().is_err());
    assert!(r.auto.state().log.is_empty());
}

#[tokio::test]
async fn tx_halted_event_switches_it_off() {
    let r = rig(true, Limits::default());
    r.auto.set_enabled(true);
    r.bus.events().send(Event::TxHalted).unwrap();
    settle().await;
    assert!(!r.auto.state().enabled);
}

#[tokio::test]
async fn a_logged_qso_frees_the_next_reply() {
    let mut r = rig(true, Limits::default());
    r.auto.set_mode(Mode::Live).unwrap();
    r.auto.set_enabled(true);
    r.bus.events().send(cq(1, "AT1AA")).unwrap();
    timeout(Duration::from_secs(1), r.commands.recv()).await.unwrap().unwrap();
    r.bus
        .events()
        .send(Event::QsoLogged(QsoRecord {
            call: "AT1AA".into(),
            grid: "FN42".into(),
            band: "20m".into(),
            mode: "FT8".into(),
            freq_hz: 14_074_000,
            rst_sent: "-10".into(),
            rst_rcvd: "-12".into(),
            time_ms: 0,
        }))
        .unwrap();
    settle().await;
    r.bus.events().send(cq(2, "LM1XY")).unwrap();
    let cmd = timeout(Duration::from_secs(1), r.commands.recv()).await.unwrap().unwrap();
    assert_eq!(cmd, Command::Reply { decode_id: 2 });
}

#[tokio::test]
async fn live_mode_is_refused_when_transmission_is_closed() {
    let r = rig(false, Limits::default());
    assert!(r.auto.set_mode(Mode::Live).is_err());
    assert_eq!(r.auto.state().mode, Mode::DryRun);
}

#[tokio::test]
async fn dead_man_switches_off_without_clients_and_a_client_guard_keeps_it_on() {
    let r = rig(true, Limits { dead_man_secs: 10, ..Limits::default() });
    r.auto.set_enabled(true);
    let guard = r.auto.client_guard();
    r.clock.fetch_add(500, Ordering::SeqCst);
    settle().await;
    assert!(r.auto.state().enabled, "con un client collegato non scatta");
    drop(guard);
    settle().await; // vede lo zero e fissa l'inizio
    r.clock.fetch_add(11, Ordering::SeqCst);
    settle().await;
    assert!(!r.auto.state().enabled, "senza client oltre il limite si spegne");
}

#[tokio::test]
async fn halt_called_directly_switches_it_off() {
    let r = rig(true, Limits::default());
    r.auto.set_enabled(true);
    r.auto.halt();
    assert!(!r.auto.state().enabled);
}
