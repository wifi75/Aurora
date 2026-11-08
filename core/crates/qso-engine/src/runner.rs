//! Il task che collega il cervello al bus: ascolta gli eventi, manda i `Reply` decisi, controlla il dead-man.

use std::{
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc, Mutex, MutexGuard,
    },
    time::Duration,
};

use aurora_core::{Bus, Command, DecodedMessage, Event, SharedUdpFilters};
use tokio::{sync::broadcast::error::RecvError, task::JoinHandle, time::interval};

use crate::brain::{Brain, BrainState, DecisionKind, Limits, LiveNotAllowed, Mode, NewModeMatcher, Rules, WantedMatcher};

/// Scarto dell'ora locale da UTC, in secondi: lo calcola chi conosce il sistema (il cervello non legge l'orologio).
pub type LocalOffset = Arc<dyn Fn() -> i64 + Send + Sync>;

/// L'interruttore «Abilita Tx» del motore, visto dalle automazioni: come e' adesso (`None` = non si sa) e come cambiarlo
/// (vero se il motore ha preso il comando).
#[derive(Clone)]
pub struct TxSwitch {
    pub get: Arc<dyn Fn() -> Option<bool> + Send + Sync>,
    pub set: Arc<dyn Fn(bool) -> bool + Send + Sync>,
}

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
    /// Per il CQ da solo e per fermare la trasmissione quando l'automazione si spegne con un suo CQ in onda.
    commands: tokio::sync::mpsc::Sender<Command>,
    /// Ora locale del PC (la fascia oraria); di base UTC.
    offset: Mutex<LocalOffset>,
    /// I paesi di cty.dat, per la scelta dei paesi esclusi nella pagina.
    countries: Mutex<Arc<Vec<String>>>,
    /// «Abilita Tx» del motore: lo accendono le automazioni in modo reale e lo rimettono com'era quando si fermano.
    tx_switch: Mutex<Option<TxSwitch>>,
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
            commands: bus.commands(),
            offset: Mutex::new(Arc::new(|| 0)),
            countries: Mutex::new(Arc::new(Vec::new())),
            tx_switch: Mutex::new(None),
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
                            Ok(event) => {
                                handle_event(&inner, event);
                                sync_tx_switch(&inner);
                            }
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
                        let offset = (lock(&inner.offset).clone())();
                        let decision = {
                            let mut brain = lock(&inner.brain);
                            brain.set_utc_offset(offset);
                            brain.tick(now, clients)
                        };
                        // il CQ da solo parte solo in modo reale (in prova a vuoto il cervello decide `WouldCallCq`)
                        let call_cq = decision.is_some_and(|d| d.kind == DecisionKind::CallCq);
                        if call_cq {
                            ensure_tx_enabled(&inner);
                        }
                        if call_cq
                            && commands.try_send(Command::CallCq { modifier: String::new() }).is_err()
                        {
                            tracing::warn!("CQ automatico non consegnato al motore");
                        }
                        send_halt_if_requested(&inner);
                        sync_tx_switch(&inner);
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
        send_halt_if_requested(&self.inner);
        sync_tx_switch(&self.inner);
    }

    /// L'interruttore «Abilita Tx» del motore: le automazioni lo accendono quando trasmettono e lo rimettono com'era quando si fermano.
    pub fn set_tx_switch(&self, switch: TxSwitch) {
        *lock(&self.inner.tx_switch) = Some(switch);
        sync_tx_switch(&self.inner);
    }

    /// I filtri di Attivita' di banda: le stazioni che nascondono non ricevono mai una risposta automatica.
    pub fn set_band_filters(&self, filters: SharedUdpFilters) {
        lock(&self.inner.brain).set_band_filters(filters);
    }

    /// Come sapere se un paese e' nuovo nel modo in uso, per la regola «nuovo modo».
    pub fn set_new_mode_matcher(&self, matcher: NewModeMatcher) {
        lock(&self.inner.brain).set_new_mode_matcher(matcher);
    }

    /// Come calcolare lo scarto dell'ora locale (per la fascia oraria); si rilegge a ogni controllo (cambio dell'ora legale).
    pub fn set_local_offset(&self, offset: LocalOffset) {
        let secs = offset();
        *lock(&self.inner.offset) = offset;
        lock(&self.inner.brain).set_utc_offset(secs);
    }

    /// I nomi dei paesi di cty.dat, in ordine: la pagina li offre per i paesi esclusi.
    pub fn set_countries(&self, mut names: Vec<String>) {
        names.sort();
        names.dedup();
        *lock(&self.inner.countries) = Arc::new(names);
    }

    pub fn countries(&self) -> Arc<Vec<String>> {
        Arc::clone(&lock(&self.inner.countries))
    }

    pub fn set_mode(&self, mode: Mode) -> Result<(), LiveNotAllowed> {
        let result = lock(&self.inner.brain).set_mode(mode);
        sync_tx_switch(&self.inner);
        result
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
        sync_tx_switch(&self.inner);
    }

    pub fn client_guard(&self) -> ClientGuard {
        self.inner.clients.fetch_add(1, Ordering::SeqCst);
        ClientGuard { inner: Arc::clone(&self.inner) }
    }
}

/// Un lock avvelenato non deve fermare la radio: si riprende lo stato com'e'.
fn lock<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// L'automazione si e' spenta con un suo CQ in onda: si ferma la trasmissione come con STOP TX.
fn send_halt_if_requested(inner: &Inner) {
    if lock(&inner.brain).take_halt_request() && inner.commands.try_send(Command::HaltTx).is_err() {
        tracing::warn!("STOP del CQ automatico non consegnato al motore");
    }
}

/// «Abilita Tx» segue le automazioni (vedi `Brain::tx_switch_step`): si legge e si cambia solo quando cominciano o smettono di trasmettere.
fn sync_tx_switch(inner: &Inner) {
    let Some(switch) = lock(&inner.tx_switch).clone() else { return };
    let now = (inner.now)();
    let change = {
        let mut brain = lock(&inner.brain);
        if !brain.tx_switch_pending() {
            return;
        }
        brain.tx_switch_step((switch.get)(), now)
    };
    if let Some(on) = change
        && !(switch.set)(on)
    {
        tracing::warn!("Abilita Tx non cambiato dalle automazioni: motore non disponibile");
    }
}

/// Prima di una risposta o di un CQ automatico «Abilita Tx» deve essere acceso: il motore puo' averlo spento alla fine del QSO precedente.
fn ensure_tx_enabled(inner: &Inner) {
    let Some(switch) = lock(&inner.tx_switch).clone() else { return };
    if (switch.get)() != Some(true) && !(switch.set)(true) {
        tracing::warn!("Abilita Tx non acceso prima della trasmissione automatica: motore non disponibile");
    }
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
        {
            ensure_tx_enabled(inner);
            if commands.try_send(Command::Reply { decode_id }).is_err() {
                tracing::warn!("risposta automatica non consegnata al motore");
            }
        }
    }
    send_halt_if_requested(inner);
    sync_tx_switch(inner);
}

fn handle_event(inner: &Inner, event: Event) {
    let now = (inner.now)();
    match event {
        // i messaggi si raccolgono a gruppi nel ciclo principale (`flush`)
        Event::Decode(_) => {}
        Event::QsoLogged(q) => lock(&inner.brain).on_qso_logged_at(&q.call, now),
        Event::TxHalted => lock(&inner.brain).on_halt(now),
        Event::Status(s) => lock(&inner.brain).on_status(&s),
        Event::QsoCompleted(_)
        | Event::QsoPending(_)
        | Event::QsoPendingDone { .. }
        | Event::Wanted(_)
        | Event::UpdateAvailable(_)
        | Event::AudioLevel(_)
        | Event::BandChanged { .. }
        | Event::HighlightCallsign { .. }
        | Event::ClearDecodes { .. }
        | Event::Fox(_)
        | Event::Notice { .. } => {}
    }
}
