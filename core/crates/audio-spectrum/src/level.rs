use aurora_core::{AudioLevel, LevelState};

/// Un campione a questo valore o oltre conta come saturo.
const CLIP_LEVEL: f32 = 0.98;
/// Quanto dura ogni misura (secondi): quattro al secondo bastano a un indicatore.
const WINDOW_SECONDS: f32 = 0.25;
/// Quanto deve durare la saturazione prima di segnalare il sovraccarico.
const OVERDRIVE_SECONDS: f32 = 3.0;
/// Quanto deve durare il silenzio piatto prima di dirlo.
const SILENCE_SECONDS: f32 = 8.0;
/// Sotto questo picco l'ingresso e' muto (zero digitale o quasi).
const SILENCE_PEAK: f32 = 0.0005;
/// Sotto questo picco il segnale e' troppo debole per decodificare bene.
const LOW_PEAK: f32 = 0.02;
/// Sopra questo picco il livello e' alto ma non ancora saturo.
const HIGH_PEAK: f32 = 0.85;
/// Quota di campioni saturi in una misura oltre la quale la misura e' «satura».
const CLIPPED_FRACTION: f32 = 0.001;

/// Misura il livello dell'ingresso audio: picco, valore efficace e saturazione, a finestre di un quarto di secondo.
/// E' puro (campioni in ingresso, misure in uscita) e si prova con segnali sintetici.
pub struct LevelMeter {
    window_len: usize,
    n: usize,
    peak: f32,
    sum_sq: f64,
    clipped: usize,
    saturated_windows: u32,
    silent_windows: u32,
    overdrive_windows: u32,
    silence_windows: u32,
}

impl LevelMeter {
    pub fn new(sample_rate: u32) -> Self {
        let window_len = ((sample_rate as f32 * WINDOW_SECONDS) as usize).max(1);
        LevelMeter {
            window_len,
            n: 0,
            peak: 0.0,
            sum_sq: 0.0,
            clipped: 0,
            saturated_windows: 0,
            silent_windows: 0,
            overdrive_windows: (OVERDRIVE_SECONDS / WINDOW_SECONDS).round() as u32,
            silence_windows: (SILENCE_SECONDS / WINDOW_SECONDS).round() as u32,
        }
    }

    /// Aggiunge campioni mono in [-1, 1]; restituisce le misure completate (anche nessuna).
    /// I campioni non finiti valgono zero.
    pub fn push(&mut self, input: &[f32]) -> Vec<AudioLevel> {
        let mut out = Vec::new();
        for &raw in input {
            let s = if raw.is_finite() { raw } else { 0.0 };
            let a = s.abs();
            self.peak = self.peak.max(a);
            self.sum_sq += f64::from(s) * f64::from(s);
            if a >= CLIP_LEVEL {
                self.clipped += 1;
            }
            self.n += 1;
            if self.n >= self.window_len {
                out.push(self.finish_window());
            }
        }
        out
    }

    fn finish_window(&mut self) -> AudioLevel {
        let rms = (self.sum_sq / self.n as f64).sqrt() as f32;
        let clipped = self.clipped as f32 / self.n as f32;
        let peak = self.peak;
        self.n = 0;
        self.peak = 0.0;
        self.sum_sq = 0.0;
        self.clipped = 0;

        // «satura»: campioni a fondo scala, oppure picco pieno con molta energia (onda quadra)
        let saturated = clipped >= CLIPPED_FRACTION || (peak >= CLIP_LEVEL && rms >= 0.35);
        self.saturated_windows = if saturated { self.saturated_windows + 1 } else { 0 };
        self.silent_windows = if peak < SILENCE_PEAK { self.silent_windows + 1 } else { 0 };

        let state = if self.saturated_windows >= self.overdrive_windows {
            LevelState::Overdrive
        } else if self.silent_windows >= self.silence_windows {
            LevelState::Silent
        } else if saturated || peak >= HIGH_PEAK {
            LevelState::High
        } else if peak < LOW_PEAK {
            LevelState::Low
        } else {
            LevelState::Good
        };
        AudioLevel { peak, rms, clipped, state }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const RATE: u32 = 8000;

    fn sine(amplitude: f32, seconds: f32) -> Vec<f32> {
        (0..(RATE as f32 * seconds) as usize).map(|i| amplitude * (2.0 * std::f32::consts::PI * 1000.0 * i as f32 / RATE as f32).sin()).collect()
    }

    fn last(levels: Vec<AudioLevel>) -> AudioLevel {
        levels.into_iter().last().expect("almeno una misura")
    }

    #[test]
    fn measures_peak_and_rms_of_a_sine() {
        let mut m = LevelMeter::new(RATE);
        let l = last(m.push(&sine(0.5, 1.0)));
        assert!((l.peak - 0.5).abs() < 0.01, "{l:?}");
        assert!((l.rms - 0.5 / 2f32.sqrt()).abs() < 0.01, "{l:?}");
        assert_eq!(l.state, LevelState::Good);
        assert_eq!(l.clipped, 0.0);
    }

    #[test]
    fn emits_four_measures_per_second() {
        assert_eq!(LevelMeter::new(RATE).push(&sine(0.3, 2.0)).len(), 8);
    }

    #[test]
    fn flags_a_weak_signal_as_low() {
        let mut m = LevelMeter::new(RATE);
        assert_eq!(last(m.push(&sine(0.005, 1.0))).state, LevelState::Low);
    }

    #[test]
    fn a_loud_but_unclipped_signal_is_high() {
        let mut m = LevelMeter::new(RATE);
        assert_eq!(last(m.push(&sine(0.9, 1.0))).state, LevelState::High);
    }

    #[test]
    fn overdrive_needs_three_seconds_of_saturation() {
        let mut m = LevelMeter::new(RATE);
        let clipped = |secs: f32| -> Vec<f32> { sine(2.0, secs).into_iter().map(|s| s.clamp(-1.0, 1.0)).collect() };
        let first = m.push(&clipped(2.0));
        assert_eq!(last(first).state, LevelState::High, "due secondi non bastano");
        let later = m.push(&clipped(1.5));
        assert_eq!(last(later).state, LevelState::Overdrive);
        assert!(last(m.push(&sine(0.3, 0.5))).state == LevelState::Good, "tornando a un livello giusto l'avviso cade subito");
    }

    #[test]
    fn a_square_wave_at_full_scale_is_overdrive() {
        let mut m = LevelMeter::new(RATE);
        let square: Vec<f32> = (0..(RATE as usize * 4)).map(|i| if (i / 4) % 2 == 0 { 1.0 } else { -1.0 }).collect();
        assert_eq!(last(m.push(&square)).state, LevelState::Overdrive);
    }

    #[test]
    fn long_flat_silence_is_reported_after_eight_seconds() {
        let mut m = LevelMeter::new(RATE);
        assert_eq!(last(m.push(&vec![0.0; RATE as usize * 6])).state, LevelState::Low);
        assert_eq!(last(m.push(&vec![0.0; RATE as usize * 3])).state, LevelState::Silent);
        assert_eq!(last(m.push(&sine(0.2, 0.5))).state, LevelState::Good);
    }

    #[test]
    fn non_finite_samples_count_as_zero() {
        let mut m = LevelMeter::new(RATE);
        let l = last(m.push(&vec![f32::NAN; RATE as usize]));
        assert_eq!((l.peak, l.rms), (0.0, 0.0));
    }
}
