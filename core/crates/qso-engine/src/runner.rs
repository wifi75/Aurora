//! Il task che collega il cervello al bus: ascolta gli eventi, manda i `Reply` decisi, controlla il dead-man.

use std::{
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc, Mutex, MutexGuard,
    },
    time::Duration,
};

use aurora_core::{Bus, Command, Event};
use tokio::{sync::broadcast::error::RecvError, task::JoinHandle, time::interval};

use crate::brain::{Brain, BrainState, DecisionKind, Limits, LiveNotAllowed, Mode, Rules};

pub struct AutomationConfig {
    pub rules: Rules,
    pub limits: Limits,
    /// Vero solo se la trasmissione e' abilitata su questo PC.
    pub live_allowed: bool,
    /// Ogni quanto controllare il dead-man (in produzione: 1 secondo).
    pub tick_every: Duration,
    /// Secondi UTC.
    pub now: Arc<dyn Fn() -> u64 + Send + Sync>,
}

struct Inner {
    brain: Mutex<Brain>,
    clients: AtomicUsize,
    now: Arc<dyn Fn() -> u64 + Send + Sync>,
    task: Mutex<Option<JoinHandle<()>>>,
}

impl Drop for Inner {
    fn drop(&mut self) {
        if let Ok(mut task) = self.task.lock()
            && let Some(handle) = task.take()
        {
            handle.abort();
        }
    }
}

/// Maniglia dell'automazione: si clona a basso costo.
#[derive(Clone)]
pub struct Automation {
    inner: Arc<Inner>,
}

/// Finche' esiste, conta come un client collegato per il dead-man.
pub struct ClientGuard {
    inner: Arc<Inner>,
}

impl Drop for ClientGuard {
    fn drop(&mut self) {
        self.inner.clients.fetch_sub(1, Ordering::SeqCst);
    }
}

impl Automation {
    /// Va chiamato dentro un runtime Tokio.
    pub fn spawn(bus: &Bus, config: AutomationConfig) -> Automation {
        let inner = Arc::new(Inner {
            brain: Mutex::new(Brain::new(config.rules, config.limits, config.live_allowed)),
            clients: AtomicUsize::new(0),
            now: config.now,
            task: Mutex::new(None),
        });
        let mut events = bus.subscribe_events();
        let commands = bus.commands();
        let weak = Arc::downgrade(&inner);
        let mut ticker = interval(config.tick_every);
        let handle = tokio::spawn(async move {
            loop {
                tokio::select! {
                    received = events.recv() => {
                        let Some(inner) = weak.upgrade() else { break };
                        match received {
                            Ok(event) => handle_event(&inner, &commands, event),
                            Err(RecvError::Lagged(_)) => {}
                            Err(RecvError::Closed) => break,
                        }
                    }
                    _ = ticker.tick() => {
                        let Some(inner) = weak.upgrade() else { break };
                        let now = (inner.now)();
                        let clients = inner.clients.load(Ordering::SeqCst);
                        lock(&inner.brain).tick(now, clients);
                    }
                }
            }
        });
        if let Ok(mut slot) = inner.task.lock() {
            *slot = Some(handle);
        }
        Automation { inner }
    }

    pub fn state(&self) -> BrainState {
        lock(&self.inner.brain).state().clone()
    }

    pub fn set_enabled(&self, enabled: bool) {
        lock(&self.inner.brain).set_enabled(enabled, (self.inner.now)());
    }

    pub fn set_mode(&self, mode: Mode) -> Result<(), LiveNotAllowed> {
        lock(&self.inner.brain).set_mode(mode)
    }

    pub fn set_rules(&self, rules: Rules) {
        lock(&self.inner.brain).set_rules(rules);
    }

    /// STOP TX: spegne l'automazione.
    pub fn halt(&self) {
        lock(&self.inner.brain).on_halt((self.inner.now)());
    }

    pub fn client_guard(&self) -> ClientGuard {
        self.inner.clients.fetch_add(1, Ordering::SeqCst);
        ClientGuard { inner: Arc::clone(&self.inner) }
    }
}

/// Un lock avvelenato non deve fermare la radio: si riprende lo stato com'e'.
fn lock(brain: &Mutex<Brain>) -> MutexGuard<'_, Brain> {
    brain.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
}

fn handle_event(inner: &Inner, commands: &tokio::sync::mpsc::Sender<Command>, event: Event) {
    let now = (inner.now)();
    match event {
        Event::Decode(d) => {
            let decision = lock(&inner.brain).on_decode(&d, now);
            if let Some(decision) = decision
                && decision.kind == DecisionKind::Reply
                && let Some(decode_id) = decision.decode_id
                && commands.try_send(Command::Reply { decode_id }).is_err()
            {
                tracing::warn!("risposta automatica non consegnata al motore");
            }
        }
        Event::QsoLogged(q) => lock(&inner.brain).on_qso_logged(&q.call),
        Event::TxHalted => lock(&inner.brain).on_halt(now),
        Event::Status(_) | Event::Wanted(_) => {}
    }
}
