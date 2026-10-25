//! Il task che collega il cervello al bus: ascolta gli eventi, manda i `Reply` decisi, controlla il dead-man.

use std::{
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc, Mutex, MutexGuard,
    },
    time::Duration,
};

use aurora_core::{Bus, Command, DecodedMessage, Event};
use tokio::{sync::broadcast::error::RecvError, task::JoinHandle, time::interval};

use crate::brain::{Brain, BrainState, DecisionKind, Limits, LiveNotAllowed, Mode, Rules, WantedMatcher};

pub struct AutomationConfig {
    pub rules: Rules,
    pub limits: Limits,
    /// Vero solo se la trasmissione e' abilitata su questo PC.
    pub live_allowed: bool,
    /// Ogni quanto controllare il dead-man (in produzione: 1 secondo).
    pub tick_every: Duration,
    /// Per quanto tempo si raccolgono i messaggi di un ciclo prima di scegliere a quale CQ rispondere (in produzione: 350 ms:
    /// arrivano tutti insieme a fine ciclo, e la risposta deve partire subito).
    pub batch_window: Duration,
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
        let window = config.batch_window;
        let handle = tokio::spawn(async move {
            // i messaggi dello stesso ciclo si valutano insieme: si risponde a uno solo, il migliore
            let mut pending: Vec<DecodedMessage> = Vec::new();
            let mut flush_at: Option<tokio::time::Instant> = None;
            loop {
                tokio::select! {
                    received = events.recv() => {
                        let Some(inner) = weak.upgrade() else { break };
                        match received {
                            Ok(Event::Decode(d)) => {
                                pending.push(d);
                                flush_at.get_or_insert_with(|| tokio::time::Instant::now() + window);
                            }
                            Ok(event) => handle_event(&inner, event),
                            Err(RecvError::Lagged(_)) => {}
                            Err(RecvError::Closed) => break,
                        }
                    }
                    () = async {
                        match flush_at {
                            Some(at) => tokio::time::sleep_until(at).await,
                            None => std::future::pending::<()>().await,
                        }
                    } => {
                        flush_at = None;
                        if let Some(inner) = weak.upgrade() {
                            flush(&inner, &commands, std::mem::take(&mut pending));
                        } else {
                            break;
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

    /// Come riconoscere le stazioni della lista dei desiderati, per la regola «rispondi ai desiderati».
    pub fn set_wanted_matcher(&self, matcher: WantedMatcher) {
        lock(&self.inner.brain).set_wanted_matcher(matcher);
    }

    /// Il continente di chi opera, per rispettare i CQ direzionali.
    pub fn set_own_continent(&self, continent: Option<String>) {
        lock(&self.inner.brain).set_own_continent(continent);
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

/// Valuta i messaggi di un ciclo e manda la risposta al CQ scelto, se c'e'.
fn flush(inner: &Inner, commands: &tokio::sync::mpsc::Sender<Command>, decodes: Vec<DecodedMessage>) {
    if decodes.is_empty() {
        return;
    }
    let now = (inner.now)();
    let decisions = lock(&inner.brain).on_batch(&decodes, now);
    for decision in decisions {
        if decision.kind == DecisionKind::Reply
            && let Some(decode_id) = decision.decode_id
            && commands.try_send(Command::Reply { decode_id }).is_err()
        {
            tracing::warn!("risposta automatica non consegnata al motore");
        }
    }
}

fn handle_event(inner: &Inner, event: Event) {
    let now = (inner.now)();
    match event {
        // i messaggi si raccolgono a gruppi nel ciclo principale (`flush`)
        Event::Decode(_) => {}
        Event::QsoLogged(q) => lock(&inner.brain).on_qso_logged(&q.call),
        Event::TxHalted => lock(&inner.brain).on_halt(now),
        Event::Status(s) => lock(&inner.brain).on_dial(s.dial_hz),
        Event::Wanted(_) | Event::UpdateAvailable(_) | Event::AudioLevel(_) | Event::BandChanged { .. } => {}
    }
}
