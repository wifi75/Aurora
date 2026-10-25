//! Il pianificatore: dato un periodo UTC (FT8/FT4, pari o dispari) e un'onda a 12 kHz, accende il PTT poco prima, avvia l'audio
//! al momento giusto, spegne il PTT subito dopo la fine e tiene lo stato osservabile. `halt` ferma tutto subito.
//!
//! Sicurezze: nessuna trasmissione con `allow_tx` falso o senza livello; una sola trasmissione alla volta; limite di durata
//! dell'onda; il PTT si spegne sempre (guardia, panico del thread, scadenza a tempo vero del `watchdog`).

use std::{
    panic::{AssertUnwindSafe, catch_unwind},
    sync::{
        Arc, Condvar, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    thread::JoinHandle,
    time::{Duration, Instant},
};

use crate::{
    WAVE_RATE,
    audio::AudioSink,
    clock::Clock,
    error::TxError,
    level::{MAX_ATTENUATION_TENTHS, prepare_samples},
    lock,
    ptt::{Ptt, PttGuard, release_ptt},
    slot::{Mode, Parity, choose_slot},
    watchdog::Watchdog,
};

/// Passo delle attese: e' anche il tempo massimo con cui `halt` viene visto dal thread di trasmissione.
const CHUNK: Duration = Duration::from_millis(10);

#[derive(Debug, Clone, PartialEq)]
pub struct TxConfig {
    /// Interruttore generale: falso = nessuna trasmissione (si controlla a ogni passo, anche durante l'attesa).
    pub allow_tx: bool,
    /// Attenuazione in decimi di dB (come la «Pwr» di JTDX). `None` = livello non impostato: nessuna trasmissione.
    pub attenuation_tenths: Option<u32>,
    /// Quanto prima dell'audio si accende il PTT (50-300 ms di base).
    pub ptt_lead: Duration,
    /// Quanto dopo la fine dell'audio si spegne il PTT.
    pub ptt_tail: Duration,
    /// Anticipo dell'audio per la latenza del dispositivo: l'audio parte a inizio periodo + 0,5 s - anticipo.
    pub latency_advance: Duration,
    /// Ritardo massimo accettato sull'inizio nominale dell'audio, quando si chiede il periodo in corso.
    pub max_late: Duration,
    /// Durata massima di un'onda.
    pub max_duration: Duration,
    /// Margine oltre la durata prevista, dopo il quale il `watchdog` spegne il PTT.
    pub watchdog_grace: Duration,
    /// Durata delle rampe di inizio e fine.
    pub fade: Duration,
}

impl Default for TxConfig {
    fn default() -> Self {
        TxConfig {
            allow_tx: false,
            attenuation_tenths: None,
            ptt_lead: Duration::from_millis(100),
            ptt_tail: Duration::from_millis(50),
            latency_advance: Duration::from_millis(30),
            max_late: Duration::from_millis(1000),
            max_duration: Duration::from_secs(15),
            watchdog_grace: Duration::from_secs(2),
            fade: Duration::from_millis(5),
        }
    }
}

impl TxConfig {
    pub fn validate(&self) -> Result<(), TxError> {
        let bad = |what: &str| Err(TxError::InvalidConfig(what.to_string()));
        if self.attenuation_tenths.is_some_and(|t| t > MAX_ATTENUATION_TENTHS) {
            return bad("attenuazione oltre 99,9 dB");
        }
        if self.ptt_lead > Duration::from_millis(1000) {
            return bad("anticipo del PTT oltre 1000 ms");
        }
        if self.ptt_tail > Duration::from_millis(1000) {
            return bad("ritardo di spegnimento del PTT oltre 1000 ms");
        }
        if self.latency_advance > Duration::from_millis(500) {
            return bad("anticipo per la latenza oltre 500 ms");
        }
        if self.max_late > Duration::from_millis(3000) {
            return bad("ritardo massimo oltre 3000 ms");
        }
        if self.max_duration.is_zero() || self.max_duration > Duration::from_secs(30) {
            return bad("durata massima fuori da 0-30 s");
        }
        if self.fade > Duration::from_millis(50) {
            return bad("rampa oltre 50 ms");
        }
        Ok(())
    }
}

/// La richiesta: l'onda a 12 kHz (mono, picco 1,0) e il periodo.
#[derive(Debug, Clone)]
pub struct TxRequest {
    pub wave: Vec<f32>,
    pub mode: Mode,
    pub parity: Parity,
}

/// Quando succedera' cosa (ora UTC come durata dal 1970).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TxPlan {
    pub slot_start: Duration,
    pub ptt_on: Duration,
    pub audio_start: Duration,
    pub wave_len: Duration,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TxState {
    /// Nessuna trasmissione (anche dopo una conclusa normalmente).
    Idle,
    /// Richiesta accettata, in attesa dell'ora (o in preparazione).
    Waiting,
    Transmitting,
    /// L'ultima trasmissione e' stata fermata (`halt`) o e' fallita: vedi [`Transmitter::last_outcome`].
    Stopped,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TxOutcome {
    Completed,
    Halted,
    Failed(String),
}

struct Status {
    state: TxState,
    outcome: Option<TxOutcome>,
}

type Listener = Arc<dyn Fn(TxState) + Send + Sync>;
type SharedSink = Arc<Mutex<Box<dyn AudioSink>>>;

struct Inner {
    clock: Arc<dyn Clock>,
    ptt: Arc<dyn Ptt>,
    sink: SharedSink,
    config: Mutex<TxConfig>,
    status: Mutex<Status>,
    changed: Condvar,
    halt: Arc<AtomicBool>,
    watchdog: Watchdog,
    listener: Mutex<Option<Listener>>,
    worker: Mutex<Option<JoinHandle<()>>>,
}

impl Inner {
    fn set_state(&self, state: TxState) {
        lock(&self.status).state = state;
        self.changed.notify_all();
        self.notify(state);
    }

    fn finish(&self, outcome: TxOutcome) {
        let state = if outcome == TxOutcome::Completed { TxState::Idle } else { TxState::Stopped };
        {
            let mut st = lock(&self.status);
            st.state = state;
            st.outcome = Some(outcome);
        }
        self.changed.notify_all();
        self.notify(state);
    }

    fn notify(&self, state: TxState) {
        let listener = lock(&self.listener).clone();
        if let Some(l) = listener {
            l(state);
        }
    }
}

/// La catena di trasmissione.
pub struct Transmitter {
    inner: Arc<Inner>,
}

struct Job {
    wave: Vec<f32>,
    attenuation_tenths: u32,
    lead: Duration,
    tail: Duration,
    grace: Duration,
    fade: Duration,
    ptt_at: Duration,
    audio_at: Duration,
}

impl Transmitter {
    /// `clock` e' l'ora UTC, `ptt` il comando di trasmissione (`NoPtt` per il VOX), `sink` l'uscita audio.
    pub fn new(clock: Arc<dyn Clock>, ptt: Arc<dyn Ptt>, sink: Box<dyn AudioSink>, config: TxConfig) -> Result<Transmitter, TxError> {
        config.validate()?;
        let sink: SharedSink = Arc::new(Mutex::new(sink));
        let halt = Arc::new(AtomicBool::new(false));
        let trip_halt = Arc::clone(&halt);
        let trip_clock = Arc::clone(&clock);
        let trip_sink = Arc::clone(&sink);
        // quando scatta il controllo si ferma anche l'audio (senza aspettare il blocco del dispositivo, se e' lui a tenerlo)
        let on_trip = Box::new(move || {
            trip_halt.store(true, Ordering::SeqCst);
            trip_clock.wake();
            if let Ok(mut s) = trip_sink.try_lock() {
                s.stop();
            }
        });
        let watchdog = Watchdog::start(Arc::clone(&ptt), on_trip, Duration::from_millis(250))?;
        let inner = Inner {
            clock,
            ptt,
            sink,
            config: Mutex::new(config),
            status: Mutex::new(Status { state: TxState::Idle, outcome: None }),
            changed: Condvar::new(),
            halt,
            watchdog,
            listener: Mutex::new(None),
            worker: Mutex::new(None),
        };
        Ok(Transmitter { inner: Arc::new(inner) })
    }

    pub fn config(&self) -> TxConfig {
        lock(&self.inner.config).clone()
    }

    /// Cambia le impostazioni (valgono dalla prossima richiesta; `allow_tx` vale subito).
    pub fn set_config(&self, config: TxConfig) -> Result<(), TxError> {
        config.validate()?;
        *lock(&self.inner.config) = config;
        Ok(())
    }

    /// Una funzione chiamata a ogni cambio di stato (dal thread che lo cambia: deve essere breve).
    pub fn set_listener(&self, listener: Option<Arc<dyn Fn(TxState) + Send + Sync>>) {
        *lock(&self.inner.listener) = listener;
    }

    pub fn state(&self) -> TxState {
        lock(&self.inner.status).state
    }

    pub fn last_outcome(&self) -> Option<TxOutcome> {
        lock(&self.inner.status).outcome.clone()
    }

    /// Quante volte il controllo a tempo ha dovuto spegnere il PTT.
    pub fn watchdog_trips(&self) -> u64 {
        self.inner.watchdog.trips()
    }

    /// Il PTT che il pianificatore crede acceso.
    pub fn ptt_is_on(&self) -> bool {
        self.inner.ptt.is_on()
    }

    /// Aspetta (tempo vero, al massimo `timeout`) che non ci sia piu' niente in attesa o in corso.
    pub fn wait_idle(&self, timeout: Duration) -> bool {
        let limit = Instant::now() + timeout;
        let mut st = lock(&self.inner.status);
        while matches!(st.state, TxState::Waiting | TxState::Transmitting) {
            let left = limit.saturating_duration_since(Instant::now());
            if left.is_zero() {
                return false;
            }
            st = self.inner.changed.wait_timeout(st, left).unwrap_or_else(|p| p.into_inner()).0;
        }
        true
    }

    /// STOP TX: ferma subito l'audio (con una discesa di pochi ms) e fa spegnere il PTT. Non aspetta: vedi [`Transmitter::wait_idle`].
    pub fn halt(&self) {
        self.inner.halt.store(true, Ordering::SeqCst);
        self.inner.clock.wake();
        if matches!(self.state(), TxState::Waiting | TxState::Transmitting) {
            // se il thread di trasmissione tiene il dispositivo in questo istante, ci pensa lui al giro dopo
            if let Ok(mut s) = self.inner.sink.try_lock() {
                s.stop();
            }
        }
    }

    /// Accetta una trasmissione e la fa partire al momento giusto, in un thread proprio.
    pub fn schedule(&self, request: TxRequest) -> Result<TxPlan, TxError> {
        let inner = &self.inner;
        if matches!(self.state(), TxState::Waiting | TxState::Transmitting) {
            return Err(TxError::Busy);
        }
        let cfg = self.config();
        cfg.validate()?;
        if !cfg.allow_tx {
            return Err(TxError::NotAllowed);
        }
        let attenuation_tenths = cfg.attenuation_tenths.ok_or(TxError::LevelNotSet)?;
        if request.wave.is_empty() {
            return Err(TxError::EmptyWave);
        }
        if request.wave.iter().any(|v| !v.is_finite()) {
            return Err(TxError::InvalidWave);
        }
        let wave_len = Duration::from_secs_f64(request.wave.len() as f64 / f64::from(WAVE_RATE));
        let limit = cfg.max_duration.min(request.mode.slot_len().saturating_sub(request.mode.tx_delay()));
        if wave_len > limit {
            return Err(TxError::TooLong { wave_ms: wave_len.as_millis() as u64, limit_ms: limit.as_millis() as u64 });
        }
        let now = inner.clock.now();
        let slot_start = choose_slot(now, request.mode, request.parity, cfg.max_late)?;
        let audio_at = (slot_start + request.mode.tx_delay()).saturating_sub(cfg.latency_advance);
        let ptt_at = audio_at.saturating_sub(cfg.ptt_lead);

        {
            let mut st = lock(&inner.status);
            if matches!(st.state, TxState::Waiting | TxState::Transmitting) {
                return Err(TxError::Busy);
            }
            st.state = TxState::Waiting;
            st.outcome = None;
        }
        inner.halt.store(false, Ordering::SeqCst);
        // il thread della trasmissione precedente e' finito (o sta per finire): lo si raccoglie
        if let Some(old) = lock(&inner.worker).take() {
            let _ = old.join();
        }
        let job = Job {
            wave: request.wave,
            attenuation_tenths,
            lead: cfg.ptt_lead,
            tail: cfg.ptt_tail,
            grace: cfg.watchdog_grace,
            fade: cfg.fade,
            ptt_at,
            audio_at,
        };
        let worker_inner = Arc::clone(inner);
        match std::thread::Builder::new().name("aurora-tx".into()).spawn(move || run_job(&worker_inner, &job)) {
            Ok(handle) => *lock(&inner.worker) = Some(handle),
            Err(e) => {
                inner.finish(TxOutcome::Failed(e.to_string()));
                return Err(TxError::Thread(e.to_string()));
            }
        }
        inner.notify(TxState::Waiting);
        Ok(TxPlan { slot_start, ptt_on: ptt_at, audio_start: audio_at, wave_len })
    }
}

impl Drop for Transmitter {
    fn drop(&mut self) {
        self.halt();
        if let Some(handle) = lock(&self.inner.worker).take() {
            let _ = handle.join();
        }
        // ultima rete: se per qualche motivo il PTT e' ancora acceso, si spegne qui
        if self.inner.ptt.is_on() {
            let _ = release_ptt(&*self.inner.ptt);
        }
    }
}

/// Attende l'ora `target`. `Ok(false)` se e' arrivato `halt`; con `check_allowed` errore se `allow_tx` e' diventato falso.
fn wait_until(inner: &Inner, target: Duration, check_allowed: bool) -> Result<bool, TxError> {
    loop {
        if inner.halt.load(Ordering::SeqCst) {
            return Ok(false);
        }
        if check_allowed && !lock(&inner.config).allow_tx {
            return Err(TxError::NotAllowed);
        }
        let now = inner.clock.now();
        if now >= target {
            return Ok(true);
        }
        inner.clock.sleep((target - now).min(CHUNK));
    }
}

/// Il corpo del thread: esegue, poi chiude sempre in modo sicuro (audio fermo, PTT spento) qualunque sia l'esito.
fn run_job(inner: &Arc<Inner>, job: &Job) {
    let result = catch_unwind(AssertUnwindSafe(|| execute(inner, job)));
    let outcome = match result {
        Ok(Ok(outcome)) => outcome,
        Ok(Err(e)) => TxOutcome::Failed(e.to_string()),
        Err(_) => TxOutcome::Failed("panico nel thread di trasmissione".into()),
    };
    if outcome != TxOutcome::Completed {
        lock(&inner.sink).stop();
    }
    let mut outcome = outcome;
    if inner.ptt.is_on() {
        match release_ptt(&*inner.ptt) {
            Ok(()) => inner.watchdog.disarm(),
            Err(e) => {
                tracing::error!("PTT ancora acceso a fine trasmissione: {e}");
                inner.watchdog.retry_soon();
                outcome = TxOutcome::Failed(format!("PTT non spento: {e}"));
            }
        }
    } else {
        inner.watchdog.disarm();
    }
    match &outcome {
        TxOutcome::Completed => tracing::info!("trasmissione conclusa"),
        TxOutcome::Halted => tracing::info!("trasmissione fermata"),
        TxOutcome::Failed(why) => tracing::warn!("trasmissione fallita: {why}"),
    }
    inner.finish(outcome);
}

fn execute(inner: &Inner, job: &Job) -> Result<TxOutcome, TxError> {
    let rate = lock(&inner.sink).sample_rate();
    let samples = prepare_samples(&job.wave, rate, job.attenuation_tenths, job.fade)?;
    let audio_len = Duration::from_secs_f64(samples.len() as f64 / f64::from(rate.max(1)));

    if !wait_until(inner, job.ptt_at, true)? {
        return Ok(TxOutcome::Halted);
    }
    // la scadenza si arma prima di accendere: se il comando si blocca, il controllo a tempo scatta lo stesso
    inner.watchdog.arm(Instant::now() + job.lead + audio_len + job.tail + job.grace);
    let mut guard = PttGuard::engage(Arc::clone(&inner.ptt))?;

    if !wait_until(inner, job.audio_at, true)? {
        guard.release()?;
        return Ok(TxOutcome::Halted);
    }
    inner.set_state(TxState::Transmitting);
    lock(&inner.sink).start(samples)?;

    let started = inner.clock.now();
    let limit = audio_len + Duration::from_secs(2);
    let halted = loop {
        if inner.halt.load(Ordering::SeqCst) || !lock(&inner.config).allow_tx {
            break true;
        }
        if !lock(&inner.sink).is_playing() {
            break false;
        }
        if inner.clock.now().saturating_sub(started) > limit {
            return Err(TxError::Audio("riproduzione oltre la durata prevista".into()));
        }
        inner.clock.sleep(CHUNK);
    };

    if halted {
        lock(&inner.sink).stop();
        // la discesa dell'audio avviene sul dispositivo, in tempo vero: si aspetta che finisca prima di togliere il PTT
        std::thread::sleep(job.fade.max(Duration::from_millis(5)));
    } else {
        wait_until(inner, inner.clock.now() + job.tail, false)?;
    }
    guard.release()?;
    Ok(if halted { TxOutcome::Halted } else { TxOutcome::Completed })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        clock::ManualClock,
        slot::slot_start,
        testing::{Recorder, RecordingPtt, RecordingSink, SinkState, TxEventKind},
    };
    use std::sync::atomic::Ordering::SeqCst;

    const RATE: u32 = 48_000;

    fn ms(n: u64) -> Duration {
        Duration::from_millis(n)
    }

    /// Inizio di un periodo FT8 pari.
    fn base() -> Duration {
        slot_start(2_000_000, Mode::Ft8)
    }

    fn tone(seconds: f64) -> Vec<f32> {
        (0..(seconds * 12_000.0) as usize).map(|n| (0.9 * (2.0 * std::f64::consts::PI * 440.0 * n as f64 / 12_000.0).sin()) as f32).collect()
    }

    fn request(seconds: f64) -> TxRequest {
        TxRequest { wave: tone(seconds), mode: Mode::Ft8, parity: Parity::Even }
    }

    fn config() -> TxConfig {
        TxConfig { allow_tx: true, attenuation_tenths: Some(100), ..TxConfig::default() }
    }

    struct Bench {
        clock: Arc<ManualClock>,
        rec: Arc<Recorder>,
        ptt: Arc<RecordingPtt>,
        sink: Arc<SinkState>,
        tx: Transmitter,
    }

    /// Un banco di prova: orologio manuale mezzo secondo prima del periodo pari, PTT e dispositivo finti.
    fn bench_with(config: TxConfig, wrap: impl FnOnce(RecordingSink) -> Box<dyn AudioSink>) -> Bench {
        let clock = Arc::new(ManualClock::new(base() - ms(500)));
        let dyn_clock: Arc<dyn Clock> = clock.clone();
        let rec = Recorder::new(Arc::clone(&dyn_clock));
        let ptt = Arc::new(RecordingPtt::new(Arc::clone(&rec)));
        let (sink, state) = RecordingSink::new(Arc::clone(&rec), Arc::clone(&dyn_clock), RATE);
        let tx = Transmitter::new(dyn_clock, ptt.clone(), wrap(sink), config).expect("trasmettitore");
        Bench { clock, rec, ptt, sink: state, tx }
    }

    fn bench(config: TxConfig) -> Bench {
        bench_with(config, |s| Box::new(s))
    }

    impl Bench {
        /// Porta avanti l'orologio a passi di 5 ms fino a `target` (senza aspettare nessuno se il thread ha finito).
        fn run_to(&self, target: Duration) {
            while self.clock.now() < target {
                if matches!(self.tx.state(), TxState::Waiting | TxState::Transmitting) {
                    self.clock.advance(ms(5));
                } else {
                    self.clock.advance_nowait(ms(5));
                }
            }
        }
    }

    #[test]
    fn nothing_is_sent_unless_allowed_and_levelled() {
        let b = bench(TxConfig::default());
        assert_eq!(b.tx.schedule(request(1.0)), Err(TxError::NotAllowed));
        let mut c = config();
        c.attenuation_tenths = None;
        b.tx.set_config(c).expect("config");
        assert_eq!(b.tx.schedule(request(1.0)), Err(TxError::LevelNotSet));
        b.tx.set_config(config()).expect("config");
        assert_eq!(b.tx.schedule(TxRequest { wave: Vec::new(), mode: Mode::Ft8, parity: Parity::Even }), Err(TxError::EmptyWave));
        assert_eq!(b.tx.schedule(TxRequest { wave: vec![f32::NAN; 10], mode: Mode::Ft8, parity: Parity::Even }), Err(TxError::InvalidWave));
        assert!(matches!(b.tx.schedule(request(15.0)), Err(TxError::TooLong { .. })));
        // FT4: anche un'onda da FT8 non entra nei 7,5 s
        assert!(matches!(b.tx.schedule(TxRequest { wave: tone(8.0), mode: Mode::Ft4, parity: Parity::Even }), Err(TxError::TooLong { .. })));
        assert!(b.rec.events().is_empty());
        assert_eq!(b.tx.state(), TxState::Idle);
        let mut bad = config();
        bad.ptt_lead = Duration::from_secs(5);
        assert!(matches!(b.tx.set_config(bad), Err(TxError::InvalidConfig(_))));
    }

    #[test]
    fn it_starts_at_the_right_time_with_ptt_before_and_after() {
        let b = bench(config());
        let plan = b.tx.schedule(request(1.0)).expect("pianificata");
        // periodo pari: inizia fra mezzo secondo, audio a +0,5 s meno 30 ms di anticipo, PTT 100 ms prima
        assert_eq!(plan.slot_start, base());
        assert_eq!(plan.audio_start, base() + ms(470));
        assert_eq!(plan.ptt_on, base() + ms(370));
        assert_eq!(plan.wave_len, Duration::from_secs(1));
        assert_eq!(b.tx.state(), TxState::Waiting);

        b.run_to(base() + ms(300));
        assert!(!b.ptt.is_on(), "PTT acceso troppo presto");
        assert!(b.rec.events().is_empty());

        b.run_to(base() + ms(420));
        assert!(b.ptt.is_on());
        assert_eq!(b.tx.state(), TxState::Waiting, "l'audio non deve essere ancora partito");

        b.run_to(base() + ms(600));
        assert_eq!(b.tx.state(), TxState::Transmitting);
        b.run_to(base() + ms(1700));
        assert!(b.tx.wait_idle(Duration::from_secs(2)));
        assert_eq!(b.tx.state(), TxState::Idle);
        assert_eq!(b.tx.last_outcome(), Some(TxOutcome::Completed));

        let ptt_on = b.rec.time_of(&TxEventKind::PttOn).expect("PTT acceso");
        let audio = b.rec.time_of(&TxEventKind::AudioStart(RATE as usize)).expect("audio partito");
        let ptt_off = b.rec.time_of(&TxEventKind::PttOff).expect("PTT spento");
        // le attese avanzano a passi di 10 ms: il ritardo possibile e' di pochi passi
        let near = |t: Duration, want: Duration| t >= want && t <= want + ms(30);
        assert!(near(ptt_on, plan.ptt_on), "PTT acceso a {ptt_on:?}");
        assert!(near(audio, plan.audio_start), "audio a {audio:?}");
        // l'audio dura 1 s: il PTT si spegne 50 ms dopo
        assert!(near(ptt_off, audio + ms(1050)), "PTT spento a {ptt_off:?}");
        assert!(audio - ptt_on >= ms(95), "il PTT deve precedere l'audio di circa 100 ms");
        assert!(!b.ptt.is_on());
        let kinds: Vec<TxEventKind> = b.rec.events().into_iter().map(|(_, k)| k).collect();
        assert_eq!(kinds, [TxEventKind::PttOn, TxEventKind::AudioStart(RATE as usize), TxEventKind::PttOff]);

        // l'audio e' stato attenuato di 10 dB e ha le rampe
        let samples = b.sink.samples.lock().expect("campioni").clone();
        assert_eq!(samples.len(), RATE as usize);
        assert_eq!(samples[0], 0.0);
        let peak = samples.iter().fold(0.0f32, |m, v| m.max(v.abs()));
        assert!((peak - 0.9 * 0.316_23).abs() < 0.02, "picco {peak}");
    }

    #[test]
    fn halt_stops_audio_and_ptt_at_once() {
        let b = bench(config());
        b.tx.schedule(request(1.0)).expect("pianificata");
        b.run_to(base() + ms(800));
        assert_eq!(b.tx.state(), TxState::Transmitting);
        b.tx.halt();
        assert!(b.tx.wait_idle(Duration::from_secs(2)), "halt non ha fermato il thread");
        assert_eq!(b.tx.state(), TxState::Stopped);
        assert_eq!(b.tx.last_outcome(), Some(TxOutcome::Halted));
        assert!(!b.ptt.is_on());
        let kinds: Vec<TxEventKind> = b.rec.events().into_iter().map(|(_, k)| k).collect();
        assert_eq!(kinds, [TxEventKind::PttOn, TxEventKind::AudioStart(RATE as usize), TxEventKind::AudioStop, TxEventKind::PttOff]);
        // l'ora e' ferma: tutto e' avvenuto nell'istante dell'halt, ben prima della fine naturale (+1,47 s)
        let off = b.rec.time_of(&TxEventKind::PttOff).expect("PTT spento");
        assert!(off <= base() + ms(810), "{off:?}");
        // e dopo non riparte niente
        b.run_to(base() + ms(2000));
        assert_eq!(b.rec.events().len(), 4);
    }

    #[test]
    fn halt_while_waiting_never_turns_the_ptt_on() {
        let b = bench(config());
        b.tx.schedule(request(1.0)).expect("pianificata");
        b.run_to(base() + ms(200));
        b.tx.halt();
        assert!(b.tx.wait_idle(Duration::from_secs(2)));
        assert_eq!(b.tx.state(), TxState::Stopped);
        assert_eq!(b.tx.last_outcome(), Some(TxOutcome::Halted));
        assert!(b.rec.events().is_empty());
        // dopo uno stop si puo' richiedere di nuovo (periodo dispari successivo)
        let again = b.tx.schedule(TxRequest { wave: tone(1.0), mode: Mode::Ft8, parity: Parity::Odd }).expect("di nuovo");
        assert_eq!(again.slot_start, base() + Duration::from_secs(15));
    }

    #[test]
    fn two_transmissions_never_overlap() {
        let b = bench(config());
        b.tx.schedule(request(1.0)).expect("prima");
        assert_eq!(b.tx.schedule(request(1.0)), Err(TxError::Busy));
        assert_eq!(b.tx.schedule(TxRequest { wave: tone(1.0), mode: Mode::Ft8, parity: Parity::Odd }), Err(TxError::Busy));
        b.run_to(base() + ms(600));
        assert_eq!(b.tx.schedule(request(1.0)), Err(TxError::Busy), "nemmeno mentre trasmette");
    }

    #[test]
    fn a_failing_device_turns_the_ptt_off() {
        let b = bench(config());
        b.sink.fail_start.store(true, SeqCst);
        b.tx.schedule(request(1.0)).expect("pianificata");
        b.run_to(base() + ms(600));
        assert!(b.tx.wait_idle(Duration::from_secs(2)));
        assert_eq!(b.tx.state(), TxState::Stopped);
        assert!(matches!(b.tx.last_outcome(), Some(TxOutcome::Failed(_))));
        assert!(!b.ptt.is_on());
        let kinds: Vec<TxEventKind> = b.rec.events().into_iter().map(|(_, k)| k).collect();
        assert_eq!(kinds, [TxEventKind::PttOn, TxEventKind::PttOff]);
    }

    #[test]
    fn a_failing_ptt_prevents_the_audio() {
        let b = bench(config());
        b.ptt.fail_on.store(true, SeqCst);
        b.tx.schedule(request(1.0)).expect("pianificata");
        b.run_to(base() + ms(600));
        assert!(b.tx.wait_idle(Duration::from_secs(2)));
        assert_eq!(b.tx.state(), TxState::Stopped);
        assert!(matches!(b.tx.last_outcome(), Some(TxOutcome::Failed(_))));
        assert!(b.rec.events().is_empty(), "senza PTT non deve partire l'audio");
    }

    /// Un dispositivo che va in panico quando parte l'audio.
    struct PanickySink(RecordingSink);

    impl AudioSink for PanickySink {
        fn sample_rate(&self) -> u32 {
            self.0.sample_rate()
        }
        fn start(&mut self, _samples: Vec<f32>) -> Result<(), TxError> {
            panic!("panico di prova nella riproduzione");
        }
        fn is_playing(&self) -> bool {
            self.0.is_playing()
        }
        fn stop(&mut self) {
            self.0.stop();
        }
    }

    #[test]
    fn a_panic_in_the_playback_thread_turns_the_ptt_off() {
        let b = bench_with(config(), |s| Box::new(PanickySink(s)));
        b.tx.schedule(request(1.0)).expect("pianificata");
        b.run_to(base() + ms(600));
        assert!(b.tx.wait_idle(Duration::from_secs(2)));
        assert_eq!(b.tx.state(), TxState::Stopped);
        match b.tx.last_outcome() {
            Some(TxOutcome::Failed(why)) => assert!(why.contains("panico"), "{why}"),
            other => panic!("esito inatteso: {other:?}"),
        }
        assert!(!b.ptt.is_on());
        let kinds: Vec<TxEventKind> = b.rec.events().into_iter().map(|(_, k)| k).collect();
        assert_eq!(kinds, [TxEventKind::PttOn, TxEventKind::PttOff]);
        // e il trasmettitore e' ancora utilizzabile
        assert!(b.tx.schedule(TxRequest { wave: tone(1.0), mode: Mode::Ft8, parity: Parity::Odd }).is_ok());
    }

    #[test]
    fn the_watchdog_turns_the_ptt_off_when_the_thread_is_stuck() {
        let mut c = config();
        c.watchdog_grace = ms(100);
        let b = bench(c);
        *b.sink.hang_in_start.lock().expect("attesa") = ms(900);
        // 0,1 s di audio: scadenza a tempo vero = 100 (anticipo) + 100 (audio) + 50 (coda) + 100 (margine) = 350 ms
        b.tx.schedule(request(0.1)).expect("pianificata");
        b.run_to(base() + ms(475));
        let limit = Instant::now() + Duration::from_secs(3);
        while b.ptt.is_on() && Instant::now() < limit {
            std::thread::sleep(ms(10));
        }
        assert!(!b.ptt.is_on(), "il controllo non ha spento il PTT");
        assert!(b.tx.watchdog_trips() >= 1);
        // il thread, quando il dispositivo risponde, vede lo stop e chiude
        assert!(b.tx.wait_idle(Duration::from_secs(3)));
        assert_eq!(b.tx.state(), TxState::Stopped);
        assert!(!b.ptt.is_on());
    }

    #[test]
    fn a_ptt_that_will_not_turn_off_is_retried_by_the_watchdog() {
        let b = bench(config());
        b.ptt.fail_off.store(true, SeqCst);
        b.tx.schedule(request(0.2)).expect("pianificata");
        b.run_to(base() + ms(900));
        assert!(b.tx.wait_idle(Duration::from_secs(2)));
        assert!(matches!(b.tx.last_outcome(), Some(TxOutcome::Failed(_))));
        assert!(b.ptt.is_on(), "il guasto simulato impedisce lo spegnimento");
        // appena il comando torna a funzionare il controllo lo spegne da solo
        b.ptt.fail_off.store(false, SeqCst);
        let limit = Instant::now() + Duration::from_secs(3);
        while b.ptt.is_on() && Instant::now() < limit {
            std::thread::sleep(ms(10));
        }
        assert!(!b.ptt.is_on());
    }

    #[test]
    fn revoking_the_permission_while_waiting_cancels_the_transmission() {
        let b = bench(config());
        b.tx.schedule(request(1.0)).expect("pianificata");
        b.run_to(base() + ms(300));
        let mut c = b.tx.config();
        c.allow_tx = false;
        b.tx.set_config(c).expect("config");
        b.clock.advance(ms(20));
        assert!(b.tx.wait_idle(Duration::from_secs(2)));
        assert_eq!(b.tx.state(), TxState::Stopped);
        assert_eq!(b.tx.last_outcome(), Some(TxOutcome::Failed(TxError::NotAllowed.to_string())));
        assert!(b.rec.events().is_empty());
    }

    #[test]
    fn revoking_the_permission_while_transmitting_stops_it() {
        let b = bench(config());
        b.tx.schedule(request(1.0)).expect("pianificata");
        b.run_to(base() + ms(800));
        assert_eq!(b.tx.state(), TxState::Transmitting);
        let mut c = b.tx.config();
        c.allow_tx = false;
        b.tx.set_config(c).expect("config");
        b.clock.advance(ms(20));
        assert!(b.tx.wait_idle(Duration::from_secs(2)));
        assert_eq!(b.tx.state(), TxState::Stopped);
        assert!(!b.ptt.is_on());
        assert!(b.rec.time_of(&TxEventKind::AudioStop).is_some());
    }

    #[test]
    fn a_slightly_late_request_starts_at_once_and_a_very_late_one_is_refused() {
        let b = bench(config());
        // 0,7 s dopo l'inizio del periodo pari: l'audio nominale (0,47 s) e' gia' passato di poco
        b.clock.advance_nowait(ms(1200));
        let plan = b.tx.schedule(request(0.3)).expect("pianificata in ritardo");
        assert_eq!(plan.slot_start, base());
        let audio_start = TxEventKind::AudioStart((0.3 * f64::from(RATE)) as usize);
        let limit = Instant::now() + Duration::from_secs(2);
        while b.rec.time_of(&audio_start).is_none() && Instant::now() < limit {
            b.clock.advance(ms(5));
        }
        assert!(b.rec.time_of(&audio_start).is_some(), "l'audio in ritardo non e' partito");
        let on = b.rec.time_of(&TxEventKind::PttOn).expect("PTT acceso");
        assert!(on <= base() + ms(720), "{on:?}");
        b.tx.halt();
        assert!(b.tx.wait_idle(Duration::from_secs(2)));

        // 2,5 s dopo l'inizio dello stesso periodo: troppo tardi
        b.clock.advance_nowait(ms(1800));
        let err = b.tx.schedule(request(0.3)).expect_err("troppo tardi");
        assert!(matches!(err, TxError::TooLate { .. }), "{err}");
    }

    #[test]
    fn the_state_listener_sees_the_transitions() {
        let b = bench(config());
        let seen = Arc::new(Mutex::new(Vec::new()));
        let sink = Arc::clone(&seen);
        b.tx.set_listener(Some(Arc::new(move |s| sink.lock().expect("elenco").push(s))));
        b.tx.schedule(request(0.2)).expect("pianificata");
        b.run_to(base() + ms(1000));
        assert!(b.tx.wait_idle(Duration::from_secs(2)));
        assert_eq!(*seen.lock().expect("elenco"), [TxState::Waiting, TxState::Transmitting, TxState::Idle]);
    }

    #[test]
    fn dropping_the_transmitter_while_waiting_leaves_nothing_on() {
        let b = bench(config());
        b.tx.schedule(request(1.0)).expect("pianificata");
        b.run_to(base() + ms(420));
        assert!(b.ptt.is_on());
        let ptt = Arc::clone(&b.ptt);
        drop(b);
        assert!(!ptt.is_on());
    }
}
