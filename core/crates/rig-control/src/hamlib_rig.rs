//! La radio comandata con Hamlib: un thread di supervisione tiene aperto `rigctld`, legge la radio a intervalli (polling) e riapre la
//! connessione se si guasta. Le regole (PTT, letture, tolleranza ai guasti, riconnessione) sono quelle ricavate dall'analisi di Decodium,
//! `docs/analisi-decodium-cat.md`, sezioni 1 e 6:
//! - subito dopo l'apertura il PTT si spegne, e per 5 s non si accetta nessun PTT acceso;
//! - in trasmissione non si leggono frequenza e modo (competono con il PTT sullo stesso bus) e il polling va a un quarto dell'intervallo;
//! - un guasto di lettura non spegne il CAT: 3 consecutivi (8 se sono timeout o errori di porta), poi riconnessione a gradini;
//! - il PTT si spegne sempre alla chiusura, e se il comando di spegnimento fallisce lo stato resta «acceso»: si dice la verita'.

use std::{
    sync::{
        atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering},
        Arc, Mutex,
    },
    thread::JoinHandle,
    time::{Duration, Instant},
};

use serde::Serialize;

use crate::{
    cat_config::{friendly_error, is_io_failure, is_serial_port, split_tx_dial_hz, PttMethod, RigConfig, SplitMode},
    hamlib::{Rigctld, RigctldClient},
    meters::{self, Level, LevelMemory, Meters, TX_LEVELS},
    Rig, RigError,
};

enum Backing {
    Daemon(Rigctld),
    Client(RigctldClient),
}

/// Una connessione aperta verso la radio: un `rigctld` lanciato da noi (che si ferma quando la connessione cade) o un client gia' collegato.
pub struct Link(Backing);

impl Link {
    pub fn from_daemon(daemon: Rigctld) -> Link {
        Link(Backing::Daemon(daemon))
    }

    pub fn from_client(client: RigctldClient) -> Link {
        Link(Backing::Client(client))
    }

    fn client(&self) -> &RigctldClient {
        match &self.0 {
            Backing::Daemon(d) => &d.client,
            Backing::Client(c) => c,
        }
    }

    fn alive(&self) -> bool {
        match &self.0 {
            Backing::Daemon(d) => d.is_running(),
            Backing::Client(_) => true,
        }
    }

    fn note(&self) -> String {
        match &self.0 {
            Backing::Daemon(d) => d.stderr_tail(),
            Backing::Client(_) => String::new(),
        }
    }
}

/// Come aprire la connessione (ogni chiamata e' un nuovo tentativo: si lancia `rigctld`, o nelle prove ci si collega a un finto).
pub type Connector = Arc<dyn Fn() -> Result<Link, RigError> + Send + Sync>;

/// Le attese del supervisore.
#[derive(Debug, Clone)]
pub struct Timing {
    /// Intervallo di lettura a riposo.
    pub poll: Duration,
    /// Intervallo minimo in trasmissione (un quarto di `poll`, mai meno di questo).
    pub tx_poll_min: Duration,
    /// Letture rapide dopo un cambio di PTT.
    pub quick_reads: [Duration; 2],
    /// Dopo il collegamento non si accetta il PTT acceso per questo tempo.
    pub startup_block: Duration,
    /// Pausa dopo ogni cambio di PTT (alcune radio non elaborano CAT mentre passano tra Tx e Rx).
    pub ptt_pause: Duration,
    /// Attesa prima di controllare che la radio abbia raggiunto la frequenza.
    pub qsy_check: Duration,
    /// Riconnessione: attesa `reconnect_step x numero del tentativo`, con tetto.
    pub reconnect_step: Duration,
    pub reconnect_cap: Duration,
    pub max_reconnects: u32,
    /// Guasti di lettura consecutivi tollerati, in generale e se sono timeout o errori di porta.
    pub tolerate: u32,
    pub tolerate_io: u32,
}

impl Timing {
    /// I tempi di Decodium: polling scelto dall'utente (le seriali non sotto 2 s), quarto in trasmissione ma non sotto 250 ms, letture a 300 e 800 ms.
    pub fn standard(config: &RigConfig) -> Timing {
        Timing {
            poll: Duration::from_secs(u64::from(config.effective_poll_secs())),
            tx_poll_min: Duration::from_millis(250),
            quick_reads: [Duration::from_millis(300), Duration::from_millis(800)],
            startup_block: Duration::from_secs(5),
            ptt_pause: Duration::from_millis(100),
            qsy_check: Duration::from_millis(500),
            reconnect_step: Duration::from_secs(5),
            reconnect_cap: Duration::from_secs(30),
            max_reconnects: 5,
            tolerate: 3,
            tolerate_io: 8,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum LinkState {
    Disconnected,
    Connecting,
    Connected,
    Reconnecting,
    Error,
}

/// Lo stato osservabile della radio.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RigStatus {
    pub state: LinkState,
    /// Testo per l'utente (errore tradotto, avanzamento della riconnessione).
    pub message: String,
    pub frequency_hz: Option<u64>,
    pub mode: Option<String>,
    pub ptt: bool,
    pub split: Option<bool>,
    /// Guasti di lettura consecutivi.
    pub failures: u32,
    pub reconnect_attempt: u32,
    /// Il PTT acceso e' ancora bloccato dall'attesa dopo il collegamento.
    pub ptt_blocked: bool,
    pub updated_ms: u64,
}

fn now_ms() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_millis() as u64)
}

fn unsupported(message: &str) -> bool {
    message.contains("non implementata") || message.contains("non disponibile")
}

/// Quanti guasti consecutivi si tollerano per questo messaggio.
pub fn failure_limit(timing: &Timing, message: &str) -> u32 {
    if is_io_failure(message) { timing.tolerate_io } else { timing.tolerate }
}

struct Inner {
    config: RigConfig,
    connector: Connector,
    timing: Timing,
    link: Mutex<Option<Arc<Link>>>,
    status: Mutex<RigStatus>,
    stop: AtomicBool,
    ptt_blocked_until: Mutex<Option<Instant>>,
    quick: Mutex<Vec<Instant>>,
    ptt_readable: AtomicBool,
    tx_audio_hz: AtomicU32,
    rx_dial: AtomicU64,
    emulate_restore: Mutex<Option<u64>>,
    /// Cresce a ogni comando: una lettura iniziata prima di un comando non ne cancella l'effetto.
    epoch: AtomicU64,
    /// I misuratori: cosa la radio non da', ultima lettura, turno di lettura.
    meter: Mutex<MeterPoll>,
}

/// Ogni quanto si legge il misuratore in ricezione e in trasmissione.
const METER_RX_EVERY: Duration = Duration::from_millis(250);
const METER_TX_EVERY: Duration = Duration::from_millis(200);

/// Lo stato della lettura dei misuratori (vedi `HamlibRig::meters`).
#[derive(Default)]
struct MeterPoll {
    memory: LevelMemory,
    cache: Meters,
    last: Option<Instant>,
    /// Ultimo stato letto: al cambio tra ricezione e trasmissione si buttano le letture dell'altro stato.
    was_tx: bool,
    /// Prossimo livello da leggere sulle seriali lente (un comando per giro).
    turn: usize,
    /// Fondo scala della potenza in watt, con i MHz a cui e' stato letto, per convertire `RFPOWER_METER`.
    full_scale: Option<(u64, f32)>,
    full_scale_tries: u8,
}

/// Seriali a 9600 baud o meno (o con la velocita' del modello, che puo' essere bassa): una raffica di comandi toglierebbe tempo al PTT.
fn slow_port(config: &RigConfig) -> bool {
    is_serial_port(&config.port) && (config.baud == 0 || config.baud <= 9600)
}

/// I passi di lettura in trasmissione: la potenza (per prima: e' la lettura principale) e poi i livelli di `TX_LEVELS`.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Step {
    Power,
    Level(Level),
}

impl Inner {
    fn update(&self, edit: impl FnOnce(&mut RigStatus)) {
        if let Ok(mut s) = self.status.lock() {
            edit(&mut s);
            s.updated_ms = now_ms();
        }
    }

    fn snapshot(&self) -> RigStatus {
        let mut s = self.status.lock().map(|s| s.clone()).unwrap_or_else(|_| RigStatus {
            state: LinkState::Error,
            message: "stato non disponibile".into(),
            frequency_hz: None,
            mode: None,
            ptt: false,
            split: None,
            failures: 0,
            reconnect_attempt: 0,
            ptt_blocked: false,
            updated_ms: 0,
        });
        s.ptt_blocked = self.ptt_blocked().is_some();
        s
    }

    fn ptt_blocked(&self) -> Option<Duration> {
        let until = (*self.ptt_blocked_until.lock().ok()?)?;
        until.checked_duration_since(Instant::now()).filter(|d| !d.is_zero())
    }

    fn link(&self) -> Option<Arc<Link>> {
        self.link.lock().ok().and_then(|l| l.clone())
    }

    fn take_link(&self) -> Option<Arc<Link>> {
        self.link.lock().ok().and_then(|mut l| l.take())
    }

    fn connected_link(&self) -> Result<Arc<Link>, RigError> {
        if self.snapshot().state != LinkState::Connected {
            return Err(RigError("radio non collegata".into()));
        }
        self.link().ok_or_else(|| RigError("radio non collegata".into()))
    }

    fn transmitting(&self) -> bool {
        self.status.lock().is_ok_and(|s| s.ptt)
    }

    fn queue_quick_reads(&self) {
        if let Ok(mut q) = self.quick.lock() {
            let now = Instant::now();
            q.extend(self.timing.quick_reads.iter().map(|d| now + *d));
        }
    }

    /// Dorme, ma torna subito se si chiude. `false` = si e' chiuso.
    fn sleep(&self, total: Duration) -> bool {
        let until = Instant::now() + total;
        loop {
            if self.stop.load(Ordering::SeqCst) {
                return false;
            }
            let left = until.saturating_duration_since(Instant::now());
            if left.is_zero() {
                return true;
            }
            std::thread::sleep(left.min(Duration::from_millis(10)));
        }
    }

    /// Apre la connessione: PTT spento subito, poi la prima lettura (se la radio non risponde l'apertura non e' riuscita).
    fn open(&self) -> Result<(), RigError> {
        self.update(|s| {
            // durante la riconnessione lo stato resta «riconnessione» (con il conto dei tentativi)
            if s.state != LinkState::Reconnecting {
                s.state = LinkState::Connecting;
                s.message = "Collegamento in corso".into();
            }
        });
        let link = Arc::new((self.connector)()?);
        // alcune radio trasmettono all'apertura della porta: PTT spento subito, errori ignorati (un VOX non ha PTT da spegnere)
        if let Err(e) = link.client().set_ptt(false) {
            tracing::debug!("PTT spento dopo l'apertura: {e}");
        }
        if !link.alive() {
            return Err(RigError(format!("rigctld terminato: {}", link.note())));
        }
        let freq = link.client().frequency()?;
        let mode = link.client().mode().ok();
        self.ptt_readable.store(true, Ordering::SeqCst);
        if let Ok(mut until) = self.ptt_blocked_until.lock() {
            *until = Some(Instant::now() + self.timing.startup_block);
        }
        self.rx_dial.store(freq, Ordering::SeqCst);
        if let Ok(mut meter) = self.meter.lock() {
            // un collegamento nuovo puo' essere un'altra radio: si riprova a chiedere tutti i livelli
            *meter = MeterPoll::default();
        }
        if let Ok(mut slot) = self.link.lock() {
            *slot = Some(link);
        }
        self.update(|s| {
            s.state = LinkState::Connected;
            s.message.clear();
            s.frequency_hz = Some(freq);
            s.mode = mode.map(|m| m.0);
            s.ptt = false;
            s.failures = 0;
            s.reconnect_attempt = 0;
        });
        Ok(())
    }

    /// Una lettura periodica. In trasmissione solo il PTT.
    fn poll_once(&self) -> Result<(), RigError> {
        let link = self.connected_link()?;
        if !link.alive() {
            return Err(RigError(format!("rigctld terminato: {}", link.note())));
        }
        let epoch = self.epoch.load(Ordering::SeqCst);
        let client = link.client();
        let (mut freq, mut mode) = (None, None);
        if !self.transmitting() {
            freq = Some(client.frequency()?);
            mode = Some(client.mode()?.0);
        }
        let ptt = if self.ptt_readable.load(Ordering::SeqCst) {
            match client.ptt() {
                Ok(p) => Some(p),
                Err(e) if unsupported(&e.0) => {
                    self.ptt_readable.store(false, Ordering::SeqCst);
                    None
                }
                Err(e) => return Err(e),
            }
        } else {
            None
        };
        if let Some(f) = freq {
            self.rx_dial.store(f, Ordering::SeqCst);
        }
        let fresh = self.epoch.load(Ordering::SeqCst) == epoch;
        self.update(|s| {
            s.failures = 0;
            if fresh {
                if freq.is_some() {
                    s.frequency_hz = freq;
                    s.mode = mode;
                }
                if let Some(p) = ptt {
                    s.ptt = p;
                }
            }
        });
        Ok(())
    }

    /// Aspetta il prossimo giro di lettura; `false` = si e' chiuso.
    fn wait_next(&self) -> bool {
        let delay = if self.transmitting() { (self.timing.poll / 4).max(self.timing.tx_poll_min) } else { self.timing.poll };
        let until = Instant::now() + delay;
        loop {
            if self.stop.load(Ordering::SeqCst) {
                return false;
            }
            let now = Instant::now();
            if now >= until {
                return true;
            }
            if let Ok(mut q) = self.quick.lock()
                && let Some(i) = q.iter().position(|t| *t <= now)
            {
                q.remove(i);
                return true;
            }
            std::thread::sleep((until - now).min(Duration::from_millis(10)));
        }
    }

    /// Riapre dopo un guasto, a gradini. `true` = di nuovo collegata.
    fn reconnect(&self, why: &str) -> bool {
        if let Some(link) = self.take_link()
            && self.transmitting()
        {
            // un solo tentativo: se la connessione e' morta ogni prova costa il tempo di attesa
            let _ = link.client().set_ptt(false);
        }
        let max = self.timing.max_reconnects;
        let mut last = why.to_string();
        for attempt in 1..=max {
            let wait = (self.timing.reconnect_step * attempt).min(self.timing.reconnect_cap);
            self.update(|s| {
                s.state = LinkState::Reconnecting;
                s.reconnect_attempt = attempt;
                s.message = format!("Collegamento perso: {} Nuovo tentativo {attempt} di {max} tra {} s.", friendly_error(&last), wait.as_secs_f32().ceil());
            });
            if !self.sleep(wait) {
                return false;
            }
            match self.open() {
                Ok(()) => return true,
                Err(e) => last = e.0,
            }
            if self.stop.load(Ordering::SeqCst) {
                return false;
            }
        }
        self.update(|s| {
            s.state = LinkState::Error;
            s.message = friendly_error(&last);
        });
        false
    }

    fn run(&self) {
        if let Err(e) = self.open() {
            if !self.stop.load(Ordering::SeqCst) {
                self.update(|s| {
                    s.state = LinkState::Error;
                    s.message = friendly_error(&e.0);
                });
            }
            return;
        }
        let mut failures = 0u32;
        while self.wait_next() {
            let dead = self.link().is_some_and(|l| !l.alive());
            let outcome = if dead { Err(RigError(format!("rigctld terminato: {}", self.link().map(|l| l.note()).unwrap_or_default()))) } else { self.poll_once() };
            match outcome {
                Ok(()) => failures = 0,
                Err(e) => {
                    failures += 1;
                    self.update(|s| s.failures = failures);
                    tracing::debug!("lettura della radio fallita ({failures}): {e}");
                    if dead || failures >= failure_limit(&self.timing, &e.0) {
                        failures = 0;
                        if !self.reconnect(&e.0) {
                            return;
                        }
                    }
                }
            }
        }
    }

    /// Un livello letto e registrato; gli esiti negativi finiscono nella memoria dei livelli (senza errori per chi chiama).
    fn read_level(&self, client: &RigctldClient, poll: &mut MeterPoll, level: Level) -> Option<f32> {
        match client.level(level.name()) {
            Ok(v) => {
                poll.memory.ok(level);
                Some(v)
            }
            Err(e) => {
                if poll.memory.failed(level, &e) {
                    tracing::debug!("livello {} non letto, non si chiede piu': {e}", level.name());
                }
                None
            }
        }
    }

    /// La potenza in watt: `RFPOWER_METER_WATTS` se la radio lo da', altrimenti `RFPOWER_METER` (frazione) per il fondo scala di Hamlib.
    fn read_power(&self, client: &RigctldClient, poll: &mut MeterPoll) {
        if poll.memory.supported(Level::PowerWatts) {
            if let Some(w) = self.read_level(client, poll, Level::PowerWatts) {
                meters::store(&mut poll.cache, Level::PowerWatts, w);
                return;
            }
            if poll.memory.supported(Level::PowerWatts) {
                return;
            }
        }
        if !poll.memory.supported(Level::PowerFraction) {
            return;
        }
        let Some(fraction) = self.read_level(client, poll, Level::PowerFraction) else { return };
        let (hz, mode) = {
            let s = self.snapshot();
            (s.frequency_hz.unwrap_or_else(|| self.rx_dial.load(Ordering::SeqCst)), s.mode.unwrap_or_else(|| "USB".into()))
        };
        let mhz = hz / 1_000_000;
        let full = match poll.full_scale {
            Some((m, w)) if m == mhz => Some(w),
            _ if poll.full_scale_tries >= 3 => poll.full_scale.map(|(_, w)| w),
            _ => {
                poll.full_scale_tries += 1;
                match client.power_to_watts(1.0, hz.max(1), &mode) {
                    Ok(w) if w > 0.0 => {
                        poll.full_scale = Some((mhz, w));
                        poll.full_scale_tries = 0;
                        Some(w)
                    }
                    _ => poll.full_scale.map(|(_, w)| w),
                }
            }
        };
        match full {
            Some(w) => poll.cache.power_w = Some((fraction.clamp(0.0, 1.0) * w).max(0.0)),
            // senza il fondo scala la frazione non si puo' dire in watt: non si inventa
            None if poll.full_scale_tries >= 3 => poll.memory.mark_unsupported(Level::PowerFraction),
            None => {}
        }
    }

    /// I misuratori. In ricezione solo `STRENGTH`; in trasmissione solo i livelli di trasmissione. Ogni livello che la radio non da' si ricorda e non
    /// si chiede piu'. Sulle seriali lente (9600 baud o meno) un solo comando per chiamata, a turno.
    fn read_meters(&self) -> Result<Meters, RigError> {
        let link = self.connected_link()?;
        let tx = self.transmitting();
        let mut poll = self.meter.lock().map_err(|_| RigError("misuratori: stato avvelenato".into()))?;
        if tx != poll.was_tx {
            poll.was_tx = tx;
            poll.last = None;
            poll.turn = 0;
            if tx {
                poll.cache.s_points = None;
                poll.cache.s_over_db = None;
            } else {
                poll.cache = Meters { s_points: None, s_over_db: None, ..Meters::default() };
            }
        }
        let every = if tx { METER_TX_EVERY } else { METER_RX_EVERY };
        if poll.last.is_some_and(|t| t.elapsed() < every) {
            return Ok(poll.cache);
        }
        poll.last = Some(Instant::now());
        let client = link.client();
        let mut steps: Vec<Step> = Vec::new();
        if tx {
            if poll.memory.supported(Level::PowerWatts) || poll.memory.supported(Level::PowerFraction) {
                steps.push(Step::Power);
            }
            steps.extend(TX_LEVELS.iter().filter(|l| poll.memory.supported(**l)).map(|l| Step::Level(*l)));
        } else if poll.memory.supported(Level::Strength) {
            steps.push(Step::Level(Level::Strength));
        }
        if steps.is_empty() {
            return Ok(poll.cache);
        }
        if slow_port(&self.config) {
            let pick = steps[poll.turn % steps.len()];
            poll.turn = poll.turn.wrapping_add(1);
            steps = vec![pick];
        }
        for step in steps {
            match step {
                Step::Power => self.read_power(client, &mut poll),
                Step::Level(level) => {
                    if let Some(v) = self.read_level(client, &mut poll, level) {
                        meters::store(&mut poll.cache, level, v);
                    }
                }
            }
        }
        Ok(poll.cache)
    }

    /// Porta la radio sulla frequenza del VFO di trasmissione per lo split vero, poi accende lo split (per ultimo: alcune radio escono
    /// dallo split quando cambia il VFO di ricezione).
    fn apply_split(&self, link: &Link) -> Result<(), RigError> {
        let (audio, dial) = (self.tx_audio_hz.load(Ordering::SeqCst), self.rx_dial.load(Ordering::SeqCst));
        if audio == 0 || dial == 0 {
            return Ok(());
        }
        link.client().set_split_frequency(split_tx_dial_hz(dial, audio))?;
        link.client().set_split(true, "VFOB")?;
        self.update(|s| s.split = Some(true));
        Ok(())
    }

    /// PTT spento con `tries` tentativi. `Ok` se la radio ha confermato.
    fn ptt_off(&self, link: &Link, tries: u32) -> Result<(), RigError> {
        let mut last = RigError("nessun tentativo".into());
        for i in 0..tries {
            match link.client().set_ptt(false) {
                Ok(()) => return Ok(()),
                Err(e) => last = e,
            }
            if i + 1 < tries {
                std::thread::sleep(self.timing.ptt_pause);
            }
        }
        Err(last)
    }
}

/// La radio comandata con Hamlib. Si collega da sola (in un thread) e resta osservabile con `status()`.
pub struct HamlibRig {
    inner: Arc<Inner>,
    thread: Mutex<Option<JoinHandle<()>>>,
}

impl HamlibRig {
    /// Avvia il supervisore: la connessione avviene in un thread, lo stato racconta come va.
    pub fn start(config: RigConfig, connector: Connector, timing: Timing) -> HamlibRig {
        let inner = Arc::new(Inner {
            config,
            connector,
            timing,
            link: Mutex::new(None),
            status: Mutex::new(RigStatus {
                state: LinkState::Connecting,
                message: "Collegamento in corso".into(),
                frequency_hz: None,
                mode: None,
                ptt: false,
                split: None,
                failures: 0,
                reconnect_attempt: 0,
                ptt_blocked: false,
                updated_ms: now_ms(),
            }),
            stop: AtomicBool::new(false),
            ptt_blocked_until: Mutex::new(None),
            quick: Mutex::new(Vec::new()),
            ptt_readable: AtomicBool::new(true),
            tx_audio_hz: AtomicU32::new(0),
            rx_dial: AtomicU64::new(0),
            emulate_restore: Mutex::new(None),
            epoch: AtomicU64::new(0),
            meter: Mutex::new(MeterPoll::default()),
        });
        let worker = Arc::clone(&inner);
        let handle = std::thread::Builder::new().name("aurora-cat".into()).spawn(move || worker.run()).ok();
        if handle.is_none() {
            inner.update(|s| {
                s.state = LinkState::Error;
                s.message = "thread del CAT non avviato".into();
            });
        }
        HamlibRig { inner, thread: Mutex::new(handle) }
    }

    pub fn config(&self) -> &RigConfig {
        &self.inner.config
    }

    pub fn status(&self) -> RigStatus {
        self.inner.snapshot()
    }

    /// Aspetta che la connessione sia riuscita o fallita (non resta in «collegamento»): per le prove e per chi parte da fermo.
    pub fn wait_settled(&self, timeout: Duration) -> RigStatus {
        let until = Instant::now() + timeout;
        loop {
            let s = self.status();
            if !matches!(s.state, LinkState::Connecting | LinkState::Reconnecting) || Instant::now() >= until {
                return s;
            }
            std::thread::sleep(Duration::from_millis(10));
        }
    }

    /// PTT acceso o spento. Acceso: solo a radio collegata e passata l'attesa dopo il collegamento. Spento: 2 tentativi, e se falliscono lo stato
    /// resta «acceso» (e l'errore si vede). Con il metodo VOX non si manda nessun comando.
    pub fn set_ptt(&self, on: bool) -> Result<(), RigError> {
        let inner = &self.inner;
        if inner.config.ptt == PttMethod::Vox {
            return Ok(());
        }
        let link = inner.connected_link()?;
        inner.epoch.fetch_add(1, Ordering::SeqCst);
        if on {
            if let Some(left) = inner.ptt_blocked() {
                return Err(RigError(format!("PTT bloccato per altri {} s dopo il collegamento", left.as_secs_f32().ceil())));
            }
            if inner.config.split == SplitMode::Emulate {
                self.emulate_begin(&link);
            }
            link.client().set_ptt(true).map_err(|e| RigError(friendly_error(&e.0)))?;
        } else {
            inner.ptt_off(&link, 2).map_err(|e| RigError(format!("PTT non spento: {}", friendly_error(&e.0))))?;
            self.emulate_end(&link);
        }
        inner.update(|s| s.ptt = on);
        std::thread::sleep(inner.timing.ptt_pause);
        inner.queue_quick_reads();
        Ok(())
    }

    /// Split simulato: la radio resta in simplex e durante la trasmissione si sposta sulla frequenza di trasmissione.
    fn emulate_begin(&self, link: &Link) {
        let (audio, dial) = (self.inner.tx_audio_hz.load(Ordering::SeqCst), self.inner.rx_dial.load(Ordering::SeqCst));
        if audio == 0 || dial == 0 {
            return;
        }
        let tx = split_tx_dial_hz(dial, audio);
        if tx != dial
            && link.client().set_frequency(tx).is_ok()
            && let Ok(mut slot) = self.inner.emulate_restore.lock()
        {
            *slot = Some(dial);
        }
    }

    fn emulate_end(&self, link: &Link) {
        let back = self.inner.emulate_restore.lock().ok().and_then(|mut s| s.take());
        if let Some(dial) = back
            && let Err(e) = link.client().set_frequency(dial)
        {
            tracing::warn!("ritorno alla frequenza di ricezione non riuscito: {e}");
        }
    }

    /// La frequenza audio con cui si trasmette (Hz): con lo split vero sposta il VFO di trasmissione, con la simulazione serve al PTT.
    pub fn set_tx_audio_frequency(&self, hz: u32) -> Result<(), RigError> {
        self.inner.tx_audio_hz.store(hz, Ordering::SeqCst);
        if self.inner.config.split == SplitMode::Rig {
            let link = self.inner.connected_link()?;
            self.inner.epoch.fetch_add(1, Ordering::SeqCst);
            self.inner.apply_split(&link).map_err(|e| RigError(friendly_error(&e.0)))?;
        }
        Ok(())
    }

    /// Il modo come lo da Hamlib (`USB`, `PKTUSB`...).
    pub fn radio_mode(&self) -> Result<(String, u32), RigError> {
        self.inner.connected_link()?.client().mode()
    }

    pub fn set_radio_mode(&self, mode: &str) -> Result<(), RigError> {
        let link = self.inner.connected_link()?;
        self.inner.epoch.fetch_add(1, Ordering::SeqCst);
        link.client().set_mode(mode, 0).map_err(|e| RigError(friendly_error(&e.0)))?;
        self.inner.update(|s| s.mode = Some(mode.to_string()));
        self.inner.queue_quick_reads();
        Ok(())
    }

    pub fn set_split(&self, on: bool, tx_vfo: &str) -> Result<(), RigError> {
        let link = self.inner.connected_link()?;
        self.inner.epoch.fetch_add(1, Ordering::SeqCst);
        link.client().set_split(on, tx_vfo).map_err(|e| RigError(friendly_error(&e.0)))?;
        self.inner.update(|s| s.split = Some(on));
        Ok(())
    }

    /// Ferma tutto: chiude il supervisore, spegne il PTT (3 tentativi), toglie lo split vero e libera la radio (il demone si ferma).
    pub fn shutdown(&self) {
        self.inner.stop.store(true, Ordering::SeqCst);
        if let Some(handle) = self.thread.lock().ok().and_then(|mut h| h.take()) {
            let _ = handle.join();
        }
        if let Some(link) = self.inner.take_link() {
            if let Err(e) = self.inner.ptt_off(&link, 3) {
                tracing::warn!("PTT non confermato spento alla chiusura: {e}");
                self.inner.update(|s| {
                    s.state = LinkState::Disconnected;
                    s.message = "Chiusa, ma il PTT potrebbe essere rimasto acceso: controlla la radio.".into();
                });
                return;
            }
            self.emulate_end(&link);
            if self.inner.config.split == SplitMode::Rig {
                let _ = link.client().set_split(false, "VFOA");
            }
        }
        self.inner.update(|s| {
            s.state = LinkState::Disconnected;
            s.message.clear();
            s.ptt = false;
            s.failures = 0;
            s.reconnect_attempt = 0;
        });
    }
}

impl Drop for HamlibRig {
    fn drop(&mut self) {
        self.shutdown();
    }
}

impl Rig for HamlibRig {
    /// Cambio di frequenza (non in trasmissione): scrive, controlla che la radio ci sia arrivata (fino a due ripetizioni), riscrive il modo scelto
    /// se la radio ha richiamato quello memorizzato per la banda e, con lo split vero, riposiziona il VFO di trasmissione.
    fn set_frequency(&self, hz: u64) -> Result<(), RigError> {
        let inner = &self.inner;
        let link = inner.connected_link()?;
        if inner.transmitting() {
            return Err(RigError("la radio sta trasmettendo: cambio di frequenza rifiutato".into()));
        }
        inner.epoch.fetch_add(1, Ordering::SeqCst);
        let fr = |e: RigError| RigError(friendly_error(&e.0));
        link.client().set_frequency(hz).map_err(fr)?;
        for attempt in 0..3 {
            std::thread::sleep(inner.timing.qsy_check);
            match link.client().frequency() {
                Ok(f) if f.abs_diff(hz) <= 100 => break,
                Ok(_) if attempt < 2 => link.client().set_frequency(hz).map_err(fr)?,
                Ok(f) => return Err(RigError(format!("la radio non ha raggiunto la frequenza ({f} Hz invece di {hz} Hz)"))),
                Err(_) => break,
            }
        }
        inner.rx_dial.store(hz, Ordering::SeqCst);
        inner.update(|s| s.frequency_hz = Some(hz));
        if let Some(wanted) = inner.config.cat_mode.hamlib_name() {
            let current = link.client().mode().map(|m| m.0).ok();
            if current.as_deref() != Some(wanted) {
                match link.client().set_mode(wanted, 0) {
                    Ok(()) => inner.update(|s| s.mode = Some(wanted.to_string())),
                    Err(e) => tracing::warn!("modo {wanted} non scritto dopo il cambio di frequenza: {e}"),
                }
            }
        }
        if inner.config.split == SplitMode::Rig
            && let Err(e) = inner.apply_split(&link)
        {
            tracing::warn!("split non riposizionato dopo il cambio di frequenza: {e}");
        }
        inner.queue_quick_reads();
        Ok(())
    }

    /// L'ultima frequenza letta dal polling (nessuna lettura in piu' sul bus della radio).
    fn frequency(&self) -> Result<u64, RigError> {
        let s = self.status();
        match (s.state, s.frequency_hz) {
            (LinkState::Connected, Some(hz)) => Ok(hz),
            _ => Err(RigError("radio non collegata".into())),
        }
    }

    /// I misuratori letti con i livelli di Hamlib (`STRENGTH` in ricezione; potenza, SWR, ALC... in trasmissione).
    fn meters(&self) -> Result<Meters, RigError> {
        self.inner.read_meters()
    }

    fn set_rx_frequency(&self, _hz: u32) -> Result<(), RigError> {
        Err(RigError("comando del motore non supportato da Hamlib".into()))
    }

    fn output_attenuation(&self) -> Result<u32, RigError> {
        Err(RigError("comando del motore non supportato da Hamlib".into()))
    }

    fn set_output_attenuation(&self, _tenths: u32) -> Result<(), RigError> {
        Err(RigError("comando del motore non supportato da Hamlib".into()))
    }

    fn set_mode(&self, _mode: &str) -> Result<(), RigError> {
        Err(RigError("comando del motore non supportato da Hamlib".into()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{cat_config::CatMode, fake_rigctld::FakeRigctld, hamlib::RigctldConfig};

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

    fn connector(fake: &FakeRigctld) -> Connector {
        let addr = fake.addr;
        Arc::new(move || RigctldClient::connect_with_timeout(addr, Duration::from_secs(1), Duration::from_millis(500)).map(Link::from_client))
    }

    fn start(fake: &FakeRigctld, config: RigConfig) -> HamlibRig {
        start_with(fake, config, fast())
    }

    fn start_with(fake: &FakeRigctld, config: RigConfig, timing: Timing) -> HamlibRig {
        let rig = HamlibRig::start(config, connector(fake), timing);
        let s = rig.wait_settled(Duration::from_secs(3));
        assert_eq!(s.state, LinkState::Connected, "{s:?}");
        rig
    }

    /// Lettura lenta: il guasto dura abbastanza perche' la prova lo veda senza gare con il thread del polling.
    fn slow() -> Timing {
        Timing { poll: Duration::from_millis(120), ..fast() }
    }

    fn wait_until(what: &str, cond: impl Fn() -> bool) {
        let until = Instant::now() + Duration::from_secs(5);
        while !cond() {
            assert!(Instant::now() < until, "tempo scaduto: {what}");
            std::thread::sleep(Duration::from_millis(5));
        }
    }

    #[test]
    fn it_turns_the_ptt_off_first_and_reads_the_radio() {
        let fake = FakeRigctld::start(14_074_000);
        let rig = start(&fake, RigConfig::default());
        let commands = fake.commands();
        assert_eq!(commands.first().map(String::as_str), Some("T 0"), "il PTT si spegne subito dopo l'apertura: {commands:?}");
        let s = rig.status();
        assert_eq!((s.frequency_hz, s.mode.as_deref(), s.ptt), (Some(14_074_000), Some("PKTUSB"), false));
        assert_eq!(rig.frequency().expect("frequenza"), 14_074_000);
    }

    #[test]
    fn polling_follows_the_radio() {
        let fake = FakeRigctld::start(7_074_000);
        let rig = start(&fake, RigConfig::default());
        fake.set_frequency(14_074_000);
        fake.set_mode("USB");
        wait_until("la frequenza segue la radio", || {
            let s = rig.status();
            s.frequency_hz == Some(14_074_000) && s.mode.as_deref() == Some("USB")
        });
    }

    #[test]
    fn the_ptt_is_refused_for_a_while_after_connecting_then_works() {
        let fake = FakeRigctld::start(14_074_000);
        let rig = start(&fake, RigConfig::default());
        assert!(rig.status().ptt_blocked);
        let err = rig.set_ptt(true).expect_err("bloccato");
        assert!(err.0.contains("bloccato"), "{err}");
        assert!(!fake.ptt());
        std::thread::sleep(Duration::from_millis(300));
        assert!(!rig.status().ptt_blocked);
        rig.set_ptt(true).expect("PTT acceso");
        assert!(fake.ptt() && rig.status().ptt);
        rig.set_ptt(false).expect("PTT spento");
        assert!(!fake.ptt() && !rig.status().ptt);
    }

    #[test]
    fn nothing_is_sent_without_a_connection() {
        let fake = FakeRigctld::start(14_074_000);
        let rig = start(&fake, RigConfig::default());
        rig.shutdown();
        assert!(rig.set_ptt(true).is_err());
        assert!(rig.set_frequency(14_074_000).is_err());
        assert_eq!(rig.status().state, LinkState::Disconnected);
    }

    #[test]
    fn while_transmitting_only_the_ptt_is_read_and_the_frequency_cannot_change() {
        let fake = FakeRigctld::start(14_074_000);
        let rig = start(&fake, RigConfig::default());
        std::thread::sleep(Duration::from_millis(300));
        rig.set_ptt(true).expect("PTT");
        assert!(rig.set_frequency(7_074_000).is_err(), "niente cambio di frequenza in trasmissione");
        std::thread::sleep(Duration::from_millis(150));
        fake.clear_commands();
        wait_until("letture del PTT in trasmissione", || fake.commands().iter().filter(|c| *c == "t").count() >= 4);
        let during = fake.commands();
        assert!(during.iter().all(|c| c == "t"), "in trasmissione si legge solo il PTT: {during:?}");
        rig.set_ptt(false).expect("PTT spento");
        fake.clear_commands();
        wait_until("le letture di frequenza riprendono", || fake.commands().iter().any(|c| c == "f"));
    }

    #[test]
    fn a_failed_ptt_off_keeps_the_truth_and_the_close_tries_again() {
        let fake = FakeRigctld::start(14_074_000);
        let rig = start(&fake, RigConfig::default());
        std::thread::sleep(Duration::from_millis(300));
        rig.set_ptt(true).expect("PTT");
        fake.fail_ptt_off(true);
        let err = rig.set_ptt(false).expect_err("spegnimento fallito");
        assert!(err.0.contains("PTT non spento"), "{err}");
        assert!(rig.status().ptt, "lo stato dice la verita': ancora acceso");
        assert!(fake.ptt());
        fake.fail_ptt_off(false);
        rig.shutdown();
        assert!(!fake.ptt(), "alla chiusura il PTT si spegne");
    }

    #[test]
    fn the_ptt_is_always_off_when_the_rig_is_dropped() {
        let fake = FakeRigctld::start(14_074_000);
        {
            let rig = start(&fake, RigConfig::default());
            std::thread::sleep(Duration::from_millis(300));
            rig.set_ptt(true).expect("PTT");
            assert!(fake.ptt());
        }
        assert!(!fake.ptt(), "il PTT e' spento anche se l'oggetto cade senza chiusura esplicita");
        assert_eq!(fake.commands().iter().rev().find(|c| c.starts_with('T')).map(String::as_str), Some("T 0"));
    }

    #[test]
    fn a_few_failed_reads_do_not_switch_the_cat_off() {
        let fake = FakeRigctld::start(14_074_000);
        let rig = start_with(&fake, RigConfig::default(), slow());
        fake.fail_with(-9);
        wait_until("un guasto", || rig.status().failures >= 1);
        assert_eq!(rig.status().state, LinkState::Connected);
        fake.fail_with(0);
        wait_until("i guasti si azzerano", || rig.status().failures == 0);
        assert_eq!(rig.status().state, LinkState::Connected);
    }

    #[test]
    fn timeouts_are_tolerated_longer_than_other_failures() {
        let t = fast();
        assert_eq!(failure_limit(&t, "f: timeout: la radio non risponde (-5)"), 8);
        assert_eq!(failure_limit(&t, "M: comando rifiutato dalla radio (-9)"), 3);
        let fake = FakeRigctld::start(14_074_000);
        let rig = start_with(&fake, RigConfig::default(), slow());
        fake.fail_with(-5);
        wait_until("quattro timeout", || rig.status().failures >= 4);
        assert_eq!(rig.status().state, LinkState::Connected, "con i timeout se ne tollerano fino a 8");
    }

    #[test]
    fn it_reconnects_by_steps_after_the_connection_drops() {
        let fake = FakeRigctld::start(14_074_000);
        let rig = start(&fake, RigConfig::default());
        fake.hang_up(true);
        wait_until("riconnessione", || rig.status().state == LinkState::Reconnecting);
        let s = rig.status();
        assert!(s.message.contains("Nuovo tentativo"), "{}", s.message);
        fake.hang_up(false);
        wait_until("di nuovo collegata", || rig.status().state == LinkState::Connected);
        let offs = fake.commands().iter().filter(|c| *c == "T 0").count();
        assert!(offs >= 2, "dopo ogni riapertura il PTT si spegne: {offs}");
        assert_eq!(rig.status().reconnect_attempt, 0);
    }

    #[test]
    fn it_gives_up_after_the_last_attempt_and_says_why() {
        let fake = FakeRigctld::start(14_074_000);
        let rig = start(&fake, RigConfig::default());
        fake.fail_with(-5);
        wait_until("errore finale", || rig.status().state == LinkState::Error);
        let s = rig.status();
        assert!(s.message.contains("non risponde"), "messaggio tradotto: {}", s.message);
        assert!(rig.set_ptt(true).is_err());
    }

    #[test]
    fn a_failed_first_open_is_an_error_with_a_readable_message() {
        let connector: Connector = Arc::new(|| Err(RigError("rigctld si e' fermato (exit code: 1): rig_open: Access denied".into())));
        let rig = HamlibRig::start(RigConfig::default(), connector, fast());
        let s = rig.wait_settled(Duration::from_secs(2));
        assert_eq!(s.state, LinkState::Error);
        assert!(s.message.contains("occupata"), "{}", s.message);
    }

    #[test]
    fn a_radio_that_stays_silent_at_the_start_is_an_open_error() {
        let fake = FakeRigctld::start(14_074_000);
        fake.fail_with(-5);
        let rig = HamlibRig::start(RigConfig::default(), connector(&fake), fast());
        let s = rig.wait_settled(Duration::from_secs(3));
        assert_eq!(s.state, LinkState::Error, "{s:?}");
    }

    #[test]
    fn a_qsy_rewrites_the_chosen_mode_when_the_radio_recalls_another() {
        let fake = FakeRigctld::start(7_074_000);
        fake.recall_mode_on_qsy(true);
        let rig = start(&fake, RigConfig { cat_mode: CatMode::Data, ..RigConfig::default() });
        rig.set_frequency(14_074_000).expect("QSY");
        assert_eq!(fake.frequency(), 14_074_000);
        assert_eq!(fake.mode(), "PKTUSB");
        assert_eq!(rig.status().frequency_hz, Some(14_074_000));
        // con «non toccare il modo» la radio resta com'e'
        let fake = FakeRigctld::start(7_074_000);
        fake.recall_mode_on_qsy(true);
        let rig = start(&fake, RigConfig::default());
        rig.set_frequency(14_074_000).expect("QSY");
        assert_eq!(fake.mode(), "USB");
        assert!(!fake.commands().iter().any(|c| c.starts_with("M ")));
    }

    #[test]
    fn real_split_moves_the_tx_vfo_and_switches_the_split_on_last() {
        let fake = FakeRigctld::start(14_074_000);
        let rig = start(&fake, RigConfig { split: SplitMode::Rig, ..RigConfig::default() });
        fake.clear_commands();
        rig.set_tx_audio_frequency(2200).expect("split");
        let split_commands = |fake: &FakeRigctld| fake.commands().into_iter().filter(|c| c.starts_with("I ") || c.starts_with("S ")).collect::<Vec<_>>();
        assert_eq!(split_commands(&fake), ["I 14074500", "S 1 VFOB"]);
        assert!(fake.split() && rig.status().split == Some(true));
        // dopo un cambio di frequenza il VFO di trasmissione segue
        rig.set_frequency(7_074_000).expect("QSY");
        let c = fake.commands();
        assert!(c.iter().any(|x| x == "I 7074500"), "{c:?}");
        // alla chiusura lo split si toglie
        rig.shutdown();
        assert!(!fake.split());
    }

    #[test]
    fn emulated_split_shifts_the_frequency_only_while_transmitting() {
        let fake = FakeRigctld::start(14_074_000);
        let rig = start(&fake, RigConfig { split: SplitMode::Emulate, ..RigConfig::default() });
        rig.set_tx_audio_frequency(2200).expect("frequenza audio");
        std::thread::sleep(Duration::from_millis(300));
        rig.set_ptt(true).expect("PTT");
        assert_eq!(fake.frequency(), 14_074_500);
        rig.set_ptt(false).expect("PTT spento");
        assert_eq!(fake.frequency(), 14_074_000, "a fine trasmissione si torna alla ricezione");
        assert!(!fake.commands().iter().any(|c| c.starts_with("S ") || c.starts_with("I ")), "nessuno split vero");
    }

    #[test]
    fn vox_sends_no_ptt_command() {
        let fake = FakeRigctld::start(14_074_000);
        let rig = start(&fake, RigConfig { ptt: PttMethod::Vox, ..RigConfig::default() });
        fake.clear_commands();
        rig.set_ptt(true).expect("VOX");
        assert!(!fake.commands().iter().any(|c| c.starts_with("T ")));
    }

    fn level_commands(fake: &FakeRigctld) -> Vec<String> {
        fake.commands().into_iter().filter(|c| c.starts_with("l ")).collect()
    }

    fn tx_rig(fake: &FakeRigctld, config: RigConfig) -> HamlibRig {
        let rig = start(fake, config);
        std::thread::sleep(Duration::from_millis(300));
        rig.set_ptt(true).expect("PTT");
        std::thread::sleep(Duration::from_millis(210));
        rig
    }

    #[test]
    fn in_receive_only_the_strength_is_read_and_converted_to_s_points() {
        let fake = FakeRigctld::start(14_074_000);
        fake.set_level("STRENGTH", "-12");
        let rig = start(&fake, RigConfig::default());
        let m = rig.meters().expect("misuratori");
        assert_eq!((m.s_points, m.s_over_db, m.power_w, m.swr), (Some(7.0), Some(0.0), None, None));
        std::thread::sleep(Duration::from_millis(260));
        fake.set_level("STRENGTH", "27");
        let m = rig.meters().expect("misuratori");
        assert_eq!((m.s_points, m.s_over_db), (Some(9.0), Some(27.0)));
        assert!(level_commands(&fake).iter().all(|c| c == "l STRENGTH"), "in ricezione solo STRENGTH: {:?}", level_commands(&fake));
    }

    #[test]
    fn a_call_inside_the_interval_does_not_ask_the_radio_again() {
        let fake = FakeRigctld::start(14_074_000);
        fake.set_level("STRENGTH", "-6");
        let rig = start(&fake, RigConfig::default());
        let _ = rig.meters().expect("misuratori");
        let _ = rig.meters().expect("misuratori");
        assert_eq!(fake.count("l STRENGTH"), 1);
    }

    #[test]
    fn an_unsupported_level_stays_none_and_is_not_asked_again() {
        let fake = FakeRigctld::start(14_074_000);
        let rig = start(&fake, RigConfig::default());
        for _ in 0..4 {
            let m = rig.meters().expect("nessun errore per un livello non supportato");
            assert_eq!((m.s_points, m.s_over_db), (None, None));
            std::thread::sleep(Duration::from_millis(260));
        }
        assert_eq!(fake.count("l STRENGTH"), 1, "la richiesta non si ripete");
    }

    #[test]
    fn in_transmit_only_the_tx_levels_are_read_and_missing_ones_are_asked_once() {
        let fake = FakeRigctld::start(14_074_000);
        fake.set_level("STRENGTH", "-12");
        fake.set_level("RFPOWER_METER_WATTS", "20.5");
        fake.set_level("SWR", "1.5");
        fake.set_level("ALC", "0.3");
        fake.set_level("COMP_METER", "6");
        fake.set_level("VD_METER", "13.8");
        let rig = tx_rig(&fake, RigConfig::default());
        fake.clear_commands();
        let mut last = Meters::default();
        for _ in 0..3 {
            last = rig.meters().expect("misuratori");
            std::thread::sleep(Duration::from_millis(210));
        }
        assert_eq!((last.power_w, last.swr, last.alc, last.comp_db, last.vdd_v), (Some(20.5), Some(1.5), Some(0.3), Some(6.0), Some(13.8)));
        assert_eq!((last.id_a, last.s_points), (None, None));
        let asked = level_commands(&fake);
        assert!(!asked.iter().any(|c| c == "l STRENGTH"), "in trasmissione niente STRENGTH: {asked:?}");
        assert_eq!(fake.count("l ID_METER"), 1, "ID non supportato: una sola richiesta");
        // di nuovo in ricezione: i dati di trasmissione spariscono e torna lo S-meter
        rig.set_ptt(false).expect("PTT spento");
        std::thread::sleep(Duration::from_millis(260));
        let m = rig.meters().expect("misuratori");
        assert_eq!((m.power_w, m.swr, m.s_points), (None, None, Some(7.0)));
    }

    #[test]
    fn power_falls_back_to_the_fraction_times_the_full_scale() {
        let fake = FakeRigctld::start(14_074_000);
        fake.set_level("RFPOWER_METER", "0.25");
        fake.set_full_power_mw(100_000);
        let rig = tx_rig(&fake, RigConfig::default());
        let m = rig.meters().expect("misuratori");
        assert_eq!(m.power_w, Some(25.0));
        assert_eq!(fake.count("l RFPOWER_METER_WATTS"), 1, "i watt diretti non ci sono: una sola richiesta");
    }

    #[test]
    fn a_fraction_without_a_known_full_scale_gives_no_power() {
        let fake = FakeRigctld::start(14_074_000);
        fake.set_level("RFPOWER_METER", "0.25");
        let rig = tx_rig(&fake, RigConfig::default());
        for _ in 0..5 {
            let m = rig.meters().expect("misuratori");
            assert_eq!(m.power_w, None, "senza fondo scala non si inventano i watt");
            std::thread::sleep(Duration::from_millis(210));
        }
    }

    #[test]
    fn a_slow_serial_port_gets_one_meter_command_per_call() {
        let fake = FakeRigctld::start(14_074_000);
        fake.set_level("RFPOWER_METER_WATTS", "10");
        fake.set_level("SWR", "1.2");
        let config = RigConfig { port: "COM3".into(), baud: 9600, ..RigConfig::default() };
        let rig = tx_rig(&fake, config);
        fake.clear_commands();
        let m = rig.meters().expect("misuratori");
        assert_eq!(level_commands(&fake).len(), 1, "un comando per giro: {:?}", level_commands(&fake));
        assert_eq!(m.power_w, Some(10.0));
        std::thread::sleep(Duration::from_millis(210));
        let m = rig.meters().expect("misuratori");
        assert_eq!((m.power_w, m.swr), (Some(10.0), Some(1.2)));
    }

    #[test]
    fn the_default_rig_has_no_meters() {
        struct Plain;
        impl Rig for Plain {
            fn set_frequency(&self, _: u64) -> Result<(), RigError> {
                Ok(())
            }
            fn set_rx_frequency(&self, _: u32) -> Result<(), RigError> {
                Ok(())
            }
            fn output_attenuation(&self) -> Result<u32, RigError> {
                Ok(0)
            }
            fn set_output_attenuation(&self, _: u32) -> Result<(), RigError> {
                Ok(())
            }
            fn set_mode(&self, _: &str) -> Result<(), RigError> {
                Ok(())
            }
        }
        assert!(Plain.meters().is_err());
    }

    fn real_rigctld() -> Option<std::path::PathBuf> {
        crate::hamlib::locate_rigctld()
    }

    /// Con un rigctld vero e il modello «dummy»: tutto il percorso (avvio del processo, polling, comandi, chiusura).
    #[test]
    fn a_real_dummy_rig_goes_through_the_whole_path() {
        let Some(exe) = real_rigctld() else { return };
        let config = RigConfig { model: 1, cat_mode: CatMode::Usb, ..RigConfig::default() };
        let rigctld: RigctldConfig = config.to_rigctld(&exe).expect("argomenti");
        let connector: Connector = Arc::new(move || Rigctld::start(&rigctld).map(Link::from_daemon));
        let rig = HamlibRig::start(config, connector, Timing { startup_block: Duration::from_millis(100), ..fast() });
        let s = rig.wait_settled(Duration::from_secs(15));
        assert_eq!(s.state, LinkState::Connected, "{s:?}");
        rig.set_frequency(14_074_000).expect("QSY");
        wait_until("la lettura segue", || rig.status().frequency_hz == Some(14_074_000));
        assert_eq!(rig.radio_mode().expect("modo").0, "USB");
        std::thread::sleep(Duration::from_millis(150));
        rig.set_ptt(true).expect("PTT");
        assert!(rig.status().ptt);
        rig.set_ptt(false).expect("PTT spento");
        rig.shutdown();
        assert_eq!(rig.status().state, LinkState::Disconnected);
    }

    #[test]
    fn killing_the_real_daemon_triggers_the_reconnection() {
        let Some(exe) = real_rigctld() else { return };
        let config = RigConfig { model: 1, ..RigConfig::default() };
        let rigctld: RigctldConfig = config.to_rigctld(&exe).expect("argomenti");
        let connector: Connector = Arc::new(move || Rigctld::start(&rigctld).map(Link::from_daemon));
        let rig = HamlibRig::start(config, connector, fast());
        assert_eq!(rig.wait_settled(Duration::from_secs(15)).state, LinkState::Connected);
        // si uccide il processo del demone dall'esterno: il supervisore se ne accorge e lo rilancia
        let link = rig.inner.link().expect("collegamento");
        if let Backing::Daemon(d) = &link.0 {
            let _ = std::process::Command::new("taskkill").args(["/PID", &d.pid().to_string(), "/F"]).output();
        }
        drop(link);
        wait_until("riconnessione dopo la morte del demone", || rig.status().state == LinkState::Reconnecting);
        assert_eq!(rig.wait_settled(Duration::from_secs(20)).state, LinkState::Connected);
        assert!(rig.frequency().is_ok());
    }
}
