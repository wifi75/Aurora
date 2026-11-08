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
            batch_window: Duration::from_millis(20),
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
        hint: false,
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
            time_on_unix_ms: 0,
            time_off_unix_ms: 0,
            ..Default::default()
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

#[tokio::test]
async fn the_cqs_of_one_cycle_make_a_single_reply_to_the_best_one() {
    let r = rig(true, Limits::default());
    r.auto.set_mode(Mode::Live).unwrap();
    r.auto.set_enabled(true);
    let mut rx = r.commands;
    // arrivano quasi insieme, come a fine ciclo: un paese nuovo debole e una griglia nuova forte
    let _ = r.bus.events().send(Event::Decode(DecodedMessage { id: 1, time_ms: 0, snr: -15, delta_time: 0.0, freq_hz: 1500, text: "CQ AT1AA FN42".into(), tag: Some(DecodeTag::NewDxcc), country: None, continent: None, hint: false }));
    let _ = r.bus.events().send(Event::Decode(DecodedMessage { id: 2, time_ms: 0, snr: -3, delta_time: 0.0, freq_hz: 1500, text: "CQ AT2AA FN42".into(), tag: Some(DecodeTag::NewGrid), country: None, continent: None, hint: false }));
    let first = tokio::time::timeout(Duration::from_secs(2), rx.recv()).await.expect("una risposta").expect("comando");
    assert!(matches!(first, Command::Reply { decode_id: 1 }), "{first:?}");
    assert!(tokio::time::timeout(Duration::from_millis(300), rx.recv()).await.is_err(), "una sola risposta per ciclo");
}

/// Un «Abilita Tx» finto: ricorda com'e' e quante volte e' stato cambiato.
fn fake_switch(start: bool) -> (qso_engine::TxSwitch, Arc<std::sync::atomic::AtomicBool>) {
    let on = Arc::new(std::sync::atomic::AtomicBool::new(start));
    let (get_on, set_on) = (Arc::clone(&on), Arc::clone(&on));
    let switch = qso_engine::TxSwitch {
        get: Arc::new(move || Some(get_on.load(Ordering::SeqCst))),
        set: Arc::new(move |v| {
            set_on.store(v, Ordering::SeqCst);
            true
        }),
    };
    (switch, on)
}

#[tokio::test]
async fn live_automation_switches_enable_tx_on_and_back_off() {
    let r = rig(true, Limits::default());
    let (switch, on) = fake_switch(false);
    r.auto.set_tx_switch(switch);
    r.auto.set_enabled(true);
    assert!(!on.load(Ordering::SeqCst), "in prova a vuoto Abilita Tx non si tocca");
    r.auto.set_mode(Mode::Live).unwrap();
    assert!(on.load(Ordering::SeqCst), "in modo reale si accende");
    r.auto.set_enabled(false);
    assert!(!on.load(Ordering::SeqCst), "spente le automazioni torna spento");
}

#[tokio::test]
async fn enable_tx_already_on_stays_on_and_stop_brings_it_back() {
    let r = rig(true, Limits::default());
    let (switch, on) = fake_switch(true);
    r.auto.set_tx_switch(switch);
    r.auto.set_mode(Mode::Live).unwrap();
    r.auto.set_enabled(true);
    r.auto.set_enabled(false);
    assert!(on.load(Ordering::SeqCst), "era acceso: resta acceso");

    let r = rig(true, Limits::default());
    let (switch, on) = fake_switch(false);
    r.auto.set_tx_switch(switch);
    r.auto.set_mode(Mode::Live).unwrap();
    r.auto.set_enabled(true);
    assert!(on.load(Ordering::SeqCst));
    r.bus.events().send(Event::TxHalted).unwrap();
    settle().await;
    assert!(!r.auto.state().enabled);
    assert!(!on.load(Ordering::SeqCst), "STOP: torna com'era");
}

#[tokio::test]
async fn an_automatic_reply_turns_enable_tx_back_on_if_the_engine_switched_it_off() {
    let mut r = rig(true, Limits::default());
    let (switch, on) = fake_switch(false);
    r.auto.set_tx_switch(switch);
    r.auto.set_mode(Mode::Live).unwrap();
    r.auto.set_enabled(true);
    // il motore lo ha spento alla fine di un QSO
    on.store(false, Ordering::SeqCst);
    r.bus.events().send(cq(5, "AT1AA")).unwrap();
    let cmd = timeout(Duration::from_secs(1), r.commands.recv()).await.unwrap().unwrap();
    assert_eq!(cmd, Command::Reply { decode_id: 5 });
    assert!(on.load(Ordering::SeqCst));
}

#[tokio::test]
async fn hidden_by_band_activity_filters_means_no_reply_command() {
    let mut r = rig(true, Limits::default());
    let filters = Arc::new(std::sync::RwLock::new(aurora_core::UdpFilters { hide_calls: vec!["AT1AA".into()], ..Default::default() }));
    r.auto.set_band_filters(filters);
    r.auto.set_mode(Mode::Live).unwrap();
    r.auto.set_enabled(true);
    r.bus.events().send(cq(5, "AT1AA")).unwrap();
    settle().await;
    assert!(r.commands.try_recv().is_err());
    let last = r.auto.state().log.last().cloned().unwrap();
    assert_eq!(last.kind, DecisionKind::Skipped);
    assert!(last.reason.contains("Attivita' di banda"), "{}", last.reason);
}
