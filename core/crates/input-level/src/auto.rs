use std::collections::VecDeque;

use aurora_core::{AudioLevel, LevelState};

/// Picco a cui si porta il segnale.
const TARGET_PEAK: f32 = 0.70;
/// Da questo picco in su (o con campioni a fondo scala) il segnale e' al limite: si scende subito.
const CLIP_PEAK: f32 = 0.93;
/// Sotto questo picco, mantenuto per `RAISE_AFTER_MS`, si sale.
const LOW_PEAK: f32 = 0.50;
const RAISE_AFTER_MS: u64 = 3_000;
/// Quota di campioni a fondo scala oltre la quale la misura e' satura.
const CLIPPED_FRACTION: f32 = 0.001;
/// Salita al massimo del 12 % del volume per volta; discesa tra il 10 % e il 50 %.
const MAX_RISE: f32 = 1.12;
const MIN_FALL: f32 = 0.5;
const MAX_FALL: f32 = 0.9;
/// Pausa dopo un cambio: poco dopo una discesa, di piu' dopo una salita.
const PAUSE_AFTER_FALL_MS: u64 = 1_000;
const PAUSE_AFTER_RAISE_MS: u64 = 3_000;
/// Cambi piu' piccoli di cosi' (1 %) non valgono la chiamata.
const MIN_STEP: f32 = 0.01;

/// La regola AUTO: pura, riceve le misure e il volume attuale e dice se cambiarlo.
pub struct AutoGain {
    /// Picchi delle misure recenti (tre secondi) con il loro istante.
    recent: VecDeque<(u64, f32)>,
    paused_until: u64,
}

impl Default for AutoGain {
    fn default() -> Self {
        AutoGain::new()
    }
}

impl AutoGain {
    pub fn new() -> AutoGain {
        AutoGain { recent: VecDeque::new(), paused_until: 0 }
    }

    /// Una misura; `volume` e' quello di adesso (0-1). Restituisce il volume nuovo, se va cambiato.
    /// Durante la trasmissione e con l'ingresso muto non si tocca nulla.
    pub fn on_level(&mut self, level: &AudioLevel, volume: f32, now_ms: u64, transmitting: bool) -> Option<f32> {
        if transmitting || level.state == LevelState::Silent {
            self.recent.clear();
            return None;
        }
        self.recent.push_back((now_ms, level.peak));
        while self.recent.front().is_some_and(|(t, _)| now_ms.saturating_sub(*t) > RAISE_AFTER_MS) {
            self.recent.pop_front();
        }
        if now_ms < self.paused_until {
            return None;
        }

        let clipping = level.clipped >= CLIPPED_FRACTION || level.peak >= CLIP_PEAK;
        if clipping {
            let factor = (TARGET_PEAK / level.peak.max(CLIP_PEAK)).clamp(MIN_FALL, MAX_FALL);
            return self.apply(volume, volume * factor, now_ms, PAUSE_AFTER_FALL_MS);
        }

        // si sale solo dopo tre secondi interi di segnale debole
        let observed = self.recent.front().is_some_and(|(t, _)| now_ms.saturating_sub(*t) >= RAISE_AFTER_MS - 250);
        let loudest = self.recent.iter().map(|(_, p)| *p).fold(0.0f32, f32::max);
        if observed && loudest < LOW_PEAK {
            let factor = (TARGET_PEAK / loudest.max(0.001)).min(MAX_RISE);
            return self.apply(volume, (volume * factor).min(1.0), now_ms, PAUSE_AFTER_RAISE_MS);
        }
        None
    }

    fn apply(&mut self, from: f32, to: f32, now_ms: u64, pause_ms: u64) -> Option<f32> {
        if (to - from).abs() < MIN_STEP {
            return None;
        }
        self.recent.clear();
        self.paused_until = now_ms + pause_ms;
        Some(to)
    }
}
