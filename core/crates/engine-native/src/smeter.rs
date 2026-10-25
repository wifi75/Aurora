//! S-meter stimato dal livello dell'audio in ingresso, per le radio che non danno i misuratori via CAT (OmniRig). Non e' una misura: il
//! livello della scheda dipende dal volume di registrazione e dall'AGC, non dalla radio. Per questo il riferimento e' **relativo**:
//! il pavimento di rumore e' il basso percentile dei livelli recenti, e i punti S si contano da li'.
//!
//! - Finestre di 150 ms a 12 kHz, livello RMS in dB.
//! - Pavimento: 10° percentile degli ultimi ~30 s (200 finestre), inseguito lentamente (costante di tempo ~7 s) cosi' un segnale lungo
//!   non lo trascina in alto e un cambio di guadagno lo recupera in pochi secondi.
//! - Scala: 6 dB per punto S, S9 a `s9_db` sopra il pavimento (40 dB di base: il rumore di fondo cade a circa S2, come su una banda HF
//!   tranquilla), oltre S9 si contano i dB (fino a +60).
//! - Strumento: attacco rapido (~60 ms), rilascio lento (~400 ms) sul livello relativo in dB.

use std::collections::VecDeque;

/// Durata di una finestra di misura, in campioni a 12 kHz (150 ms).
pub const WINDOW: usize = 1800;
/// Finestre ricordate per il pavimento: 200 x 150 ms = 30 s.
const HISTORY: usize = 200;
/// Finestre minime per fidarsi del pavimento (~3 s): prima non si stima.
const WARMUP: usize = 20;
const FLOOR_PERCENTILE: f32 = 0.10;
/// Inseguimento del pavimento per finestra (150 ms / ~7 s).
const FLOOR_ALPHA: f32 = 0.02;
/// Attacco e rilascio dello strumento per finestra: costanti di tempo ~60 ms e ~400 ms con finestre da 150 ms.
const ATTACK_ALPHA: f32 = 0.92;
const RELEASE_ALPHA: f32 = 0.31;
/// Distanza S9 dal pavimento: valore di base e limiti della taratura.
pub const DEFAULT_S9_DB: f32 = 40.0;
pub const MIN_S9_DB: f32 = 20.0;
pub const MAX_S9_DB: f32 = 70.0;
const DB_PER_S: f32 = 6.0;
const MAX_OVER_DB: f32 = 60.0;
/// Livello minimo (dBFS) per non fare logaritmi di zero con l'audio muto.
const SILENCE_DB: f32 = -120.0;

/// Una stima: punti S (0..=9) e dB oltre S9 (0..=60).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SEstimate {
    pub s_points: f32,
    pub s_over_db: f32,
}

/// Distanza S9-pavimento dalla variabile `AURORA_SMETER_S9_DB` (dB); valori assenti o illeggibili danno il valore di base.
pub fn s9_db_from_env() -> f32 {
    parse_s9_db(std::env::var("AURORA_SMETER_S9_DB").ok().as_deref())
}

fn parse_s9_db(raw: Option<&str>) -> f32 {
    raw.and_then(|s| s.trim().replace(',', ".").parse::<f32>().ok()).filter(|v| v.is_finite()).map_or(DEFAULT_S9_DB, |v| v.clamp(MIN_S9_DB, MAX_S9_DB))
}

/// Dal livello sopra il pavimento (dB) ai punti S e ai dB oltre S9, per un S9 a `s9_db` sopra il pavimento.
pub fn level_to_s(above_floor_db: f32, s9_db: f32) -> SEstimate {
    let s = 9.0 + (above_floor_db - s9_db) / DB_PER_S;
    if s >= 9.0 {
        SEstimate { s_points: 9.0, s_over_db: ((s - 9.0) * DB_PER_S).min(MAX_OVER_DB) }
    } else {
        SEstimate { s_points: s.max(0.0), s_over_db: 0.0 }
    }
}

/// Il livello RMS di un blocco, in dB rispetto al fondo scala (1.0).
pub fn rms_db(block: &[f32]) -> f32 {
    if block.is_empty() {
        return SILENCE_DB;
    }
    let mean_sq = block.iter().map(|&v| f64::from(v) * f64::from(v)).sum::<f64>() / block.len() as f64;
    if mean_sq <= 0.0 { SILENCE_DB } else { (10.0 * mean_sq.log10() as f32).max(SILENCE_DB) }
}

pub struct SMeter {
    s9_db: f32,
    pending: Vec<f32>,
    history: VecDeque<f32>,
    floor: Option<f32>,
    /// Livello relativo smorzato (dB sopra il pavimento).
    needle: f32,
    estimate: Option<SEstimate>,
}

impl SMeter {
    pub fn new(s9_db: f32) -> SMeter {
        SMeter { s9_db: s9_db.clamp(MIN_S9_DB, MAX_S9_DB), pending: Vec::with_capacity(WINDOW), history: VecDeque::with_capacity(HISTORY), floor: None, needle: 0.0, estimate: None }
    }

    /// L'ultima stima, se c'e' (non prima del riscaldamento).
    #[cfg(test)]
    fn estimate(&self) -> Option<SEstimate> {
        self.estimate
    }

    /// Dimentica le finestre in corso e la stima (non il pavimento): in trasmissione o con l'ascolto spento non si stima.
    pub fn pause(&mut self) {
        self.pending.clear();
        self.estimate = None;
        self.needle = 0.0;
    }

    /// Alimenta con audio a 12 kHz; ogni finestra completa aggiorna pavimento e stima. Restituisce l'ultima stima.
    pub fn push(&mut self, samples: &[f32]) -> Option<SEstimate> {
        let mut rest = samples;
        while !rest.is_empty() {
            let take = (WINDOW - self.pending.len()).min(rest.len());
            self.pending.extend_from_slice(&rest[..take]);
            rest = &rest[take..];
            if self.pending.len() >= WINDOW {
                let level = rms_db(&self.pending);
                self.pending.clear();
                self.window(level);
            }
        }
        self.estimate
    }

    fn window(&mut self, level_db: f32) {
        if self.history.len() == HISTORY {
            self.history.pop_front();
        }
        self.history.push_back(level_db);
        let target = percentile(&self.history, FLOOR_PERCENTILE);
        let floor = match self.floor {
            // un pavimento che scende si segue subito (il rumore e' calato davvero), uno che sale lentamente
            Some(f) if target < f => target,
            Some(f) => f + (target - f) * FLOOR_ALPHA,
            None => target,
        };
        self.floor = Some(floor);
        if self.history.len() < WARMUP {
            return;
        }
        let rel = (level_db - floor).max(0.0);
        let alpha = if rel > self.needle { ATTACK_ALPHA } else { RELEASE_ALPHA };
        self.needle += (rel - self.needle) * alpha;
        self.estimate = Some(level_to_s(self.needle, self.s9_db));
    }

    /// Il pavimento attuale, in dBFS (per i test).
    #[cfg(test)]
    fn floor_db(&self) -> Option<f32> {
        self.floor
    }
}

fn percentile(values: &VecDeque<f32>, p: f32) -> f32 {
    let mut sorted: Vec<f32> = values.iter().copied().collect();
    sorted.sort_by(|a, b| a.total_cmp(b));
    let idx = ((sorted.len() - 1) as f32 * p).round() as usize;
    sorted[idx.min(sorted.len() - 1)]
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Un tono di ampiezza `amp` lungo `windows` finestre.
    fn tone(amp: f32, windows: usize) -> Vec<f32> {
        (0..WINDOW * windows).map(|i| amp * (i as f32 * 0.5).sin()).collect()
    }

    #[test]
    fn known_levels_map_to_classic_s_points() {
        let s9 = 40.0;
        let at = |d| level_to_s(d, s9);
        assert_eq!(at(40.0), SEstimate { s_points: 9.0, s_over_db: 0.0 });
        let s5 = at(40.0 - 24.0);
        assert!((s5.s_points - 5.0).abs() < 1e-4 && s5.s_over_db == 0.0);
        let plus20 = at(60.0);
        assert_eq!(plus20.s_points, 9.0);
        assert!((plus20.s_over_db - 20.0).abs() < 1e-4);
        // il pavimento (0 dB) cade a S2,33 con S9 a 40 dB; sotto non si scende oltre S0 e sopra non oltre +60
        assert!((at(0.0).s_points - (9.0 - 40.0 / 6.0)).abs() < 1e-4);
        assert_eq!(level_to_s(-50.0, s9).s_points, 0.0);
        assert_eq!(at(200.0).s_over_db, 60.0);
    }

    #[test]
    fn rms_of_a_full_scale_sine_is_minus_three_db() {
        let block: Vec<f32> = (0..WINDOW).map(|i| (i as f32 * 0.37).sin()).collect();
        assert!((rms_db(&block) + 3.0).abs() < 0.2);
        assert_eq!(rms_db(&[0.0; 16]), SILENCE_DB);
    }

    #[test]
    fn no_estimate_during_warmup_then_noise_sits_around_s2() {
        let mut m = SMeter::new(40.0);
        assert_eq!(m.push(&tone(0.01, WARMUP - 2)), None);
        let e = m.push(&tone(0.01, 30)).expect("stima dopo il riscaldamento");
        assert!(e.s_points > 1.5 && e.s_points < 3.0, "{e:?}");
        assert_eq!(e.s_over_db, 0.0);
    }

    #[test]
    fn a_strong_signal_raises_the_needle_fast_and_releases_slowly() {
        let mut m = SMeter::new(40.0);
        m.push(&tone(0.001, 60));
        // +40 dB sul rumore: S9 dopo un paio di finestre
        let up = m.push(&tone(0.1, 3)).expect("stima");
        assert!(up.s_points >= 8.5, "{up:?}");
        // tornato il rumore, dopo una finestra (150 ms) lo strumento e' ancora alto: rilascio lento
        let down1 = m.push(&tone(0.001, 1)).expect("stima");
        assert!(down1.s_points > 6.0, "{down1:?}");
        let down2 = m.push(&tone(0.001, 12)).expect("stima");
        assert!(down2.s_points < 3.0, "{down2:?}");
    }

    #[test]
    fn the_floor_adapts_to_a_new_noise_level_and_ignores_short_signals() {
        let mut m = SMeter::new(40.0);
        m.push(&tone(0.001, 60));
        let low = m.floor_db().expect("pavimento");
        // un segnale breve (10 finestre su 200) non sposta il pavimento
        m.push(&tone(0.1, 10));
        assert!((m.floor_db().expect("pavimento") - low).abs() < 1.0);
        // il rumore sale di 20 dB per un tempo lungo: il pavimento lo raggiunge, lo strumento torna al rumore (~S2)
        m.push(&tone(0.01, 600));
        let high = m.floor_db().expect("pavimento");
        assert!((high - low - 20.0).abs() < 2.0, "{low} -> {high}");
        let e = m.push(&tone(0.01, 5)).expect("stima");
        assert!(e.s_points < 3.0, "{e:?}");
    }

    #[test]
    fn pause_drops_the_estimate_and_pending_audio() {
        let mut m = SMeter::new(40.0);
        m.push(&tone(0.01, 40));
        assert!(m.estimate().is_some());
        m.pause();
        assert_eq!(m.estimate(), None);
        // mezza finestra forte rimasta in sospeso non finisce nella finestra successiva
        m.push(&tone(0.5, 1)[..WINDOW / 2]);
        m.pause();
        let e = m.push(&tone(0.01, 1)).expect("stima");
        assert!(e.s_points < 3.5, "{e:?}");
    }

    #[test]
    fn s9_distance_from_environment_is_clamped_with_a_fallback() {
        assert_eq!(parse_s9_db(None), DEFAULT_S9_DB);
        assert_eq!(parse_s9_db(Some("45")), 45.0);
        assert_eq!(parse_s9_db(Some("42,5")), 42.5);
        assert_eq!(parse_s9_db(Some("5")), MIN_S9_DB);
        assert_eq!(parse_s9_db(Some("500")), MAX_S9_DB);
        assert_eq!(parse_s9_db(Some("abc")), DEFAULT_S9_DB);
        assert_eq!(parse_s9_db(Some("NaN")), DEFAULT_S9_DB);
    }
}
