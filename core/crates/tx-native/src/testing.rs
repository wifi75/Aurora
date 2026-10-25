//! Doppioni per le prove: un dispositivo audio e un PTT finti che registrano cosa succede e a che ora (dell'orologio iniettato).
//! Non emettono audio e non toccano nessuna radio.

use std::{
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};

use crate::{audio::AudioSink, clock::Clock, error::TxError, lock, ptt::Ptt};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TxEventKind {
    PttOn,
    PttOff,
    /// Inizio audio, con il numero di campioni.
    AudioStart(usize),
    AudioStop,
}

/// Il registro degli eventi, condiviso da PTT e dispositivo finti.
pub struct Recorder {
    clock: Arc<dyn Clock>,
    events: Mutex<Vec<(Duration, TxEventKind)>>,
}

impl Recorder {
    pub fn new(clock: Arc<dyn Clock>) -> Arc<Recorder> {
        Arc::new(Recorder { clock, events: Mutex::new(Vec::new()) })
    }

    pub fn push(&self, kind: TxEventKind) {
        let now = self.clock.now();
        lock(&self.events).push((now, kind));
    }

    pub fn events(&self) -> Vec<(Duration, TxEventKind)> {
        lock(&self.events).clone()
    }

    /// L'ora del primo evento di quel tipo.
    pub fn time_of(&self, kind: &TxEventKind) -> Option<Duration> {
        lock(&self.events).iter().find(|(_, k)| k == kind).map(|(t, _)| *t)
    }
}

/// PTT finto: registra gli eventi e puo' essere fatto fallire.
pub struct RecordingPtt {
    recorder: Arc<Recorder>,
    on: AtomicBool,
    pub fail_on: AtomicBool,
    pub fail_off: AtomicBool,
}

impl RecordingPtt {
    pub fn new(recorder: Arc<Recorder>) -> RecordingPtt {
        RecordingPtt { recorder, on: AtomicBool::new(false), fail_on: AtomicBool::new(false), fail_off: AtomicBool::new(false) }
    }
}

impl Ptt for RecordingPtt {
    fn set(&self, on: bool) -> Result<(), TxError> {
        let fail = if on { &self.fail_on } else { &self.fail_off };
        if fail.load(Ordering::SeqCst) {
            return Err(TxError::Ptt("guasto simulato".into()));
        }
        if on != self.on.swap(on, Ordering::SeqCst) {
            self.recorder.push(if on { TxEventKind::PttOn } else { TxEventKind::PttOff });
        }
        Ok(())
    }

    fn is_on(&self) -> bool {
        self.on.load(Ordering::SeqCst)
    }
}

/// Lo stato osservabile del dispositivo finto.
pub struct SinkState {
    pub samples: Mutex<Vec<f32>>,
    started_at: Mutex<Option<Duration>>,
    stopped: AtomicBool,
    pub fail_start: AtomicBool,
    /// Attesa vera dentro `start`, per simulare un dispositivo che non risponde.
    pub hang_in_start: Mutex<Duration>,
}

/// Dispositivo finto: «suona» in tempo dell'orologio iniettato (l'onda finisce quando l'ora supera inizio + durata).
pub struct RecordingSink {
    recorder: Arc<Recorder>,
    clock: Arc<dyn Clock>,
    rate: u32,
    state: Arc<SinkState>,
}

impl RecordingSink {
    pub fn new(recorder: Arc<Recorder>, clock: Arc<dyn Clock>, rate: u32) -> (RecordingSink, Arc<SinkState>) {
        let state = Arc::new(SinkState {
            samples: Mutex::new(Vec::new()),
            started_at: Mutex::new(None),
            stopped: AtomicBool::new(false),
            fail_start: AtomicBool::new(false),
            hang_in_start: Mutex::new(Duration::ZERO),
        });
        (RecordingSink { recorder, clock, rate, state: Arc::clone(&state) }, state)
    }
}

impl AudioSink for RecordingSink {
    fn sample_rate(&self) -> u32 {
        self.rate
    }

    fn start(&mut self, samples: Vec<f32>) -> Result<(), TxError> {
        let hang = *lock(&self.state.hang_in_start);
        if !hang.is_zero() {
            std::thread::sleep(hang);
        }
        if self.state.fail_start.load(Ordering::SeqCst) {
            return Err(TxError::Audio("guasto simulato".into()));
        }
        self.recorder.push(TxEventKind::AudioStart(samples.len()));
        *lock(&self.state.started_at) = Some(self.clock.now());
        self.state.stopped.store(false, Ordering::SeqCst);
        *lock(&self.state.samples) = samples;
        Ok(())
    }

    fn is_playing(&self) -> bool {
        let Some(start) = *lock(&self.state.started_at) else { return false };
        if self.state.stopped.load(Ordering::SeqCst) {
            return false;
        }
        let len = lock(&self.state.samples).len();
        self.clock.now() < start + Duration::from_secs_f64(len as f64 / f64::from(self.rate))
    }

    fn stop(&mut self) {
        if self.is_playing() {
            self.state.stopped.store(true, Ordering::SeqCst);
            self.recorder.push(TxEventKind::AudioStop);
        }
    }
}
