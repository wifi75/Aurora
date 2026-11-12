//! Noise blanker per i ricevitori a banda stretta (CW, e in futuro PSK): toglie le scariche brevi e forti (QRN, linee elettriche,
//! accensioni) prima del filtro stretto, che altrimenti le allungherebbe fino a farle sembrare toni.
//!
//! Lavora sull'audio a 12 kHz con un ritardo di 2 ms: un campione molto oltre il livello tipico della banda (il quantile 95% di |x|,
//! che le scariche, rare, non spostano) apre un buco di ±2 ms attorno a sé; i campioni del buco valgono zero. Un «impulso» che dura più
//! di 6 ms non è un impulso ma un segnale nuovo e forte: il livello tipico lo raggiunge subito e il buco si chiude (si perdono al massimo
//! 6 ms dell'inizio).

use std::collections::VecDeque;

/// Ritardo e mezza larghezza del buco (campioni a 12 kHz).
const GUARD: usize = 24;
/// Un impulso piu' lungo di cosi' (campioni) e' un segnale.
const MAX_IMPULSE: usize = 72;
/// Soglia: tante volte il quantile 95% di |x|.
const FACTOR_DB: f32 = 12.0;
/// Passo del quantile (dB per campione).
const ETA_DB: f32 = 0.02;

pub struct NoiseBlanker {
    q95_db: f32,
    warm: usize,
    line: VecDeque<f32>,
    /// Quanti campioni d'uscita restano da azzerare.
    blank_left: usize,
    /// Da quanti campioni dura l'evento in corso (0 = nessuno) e da quanti non si vede un campione oltre la soglia.
    event: usize,
    quiet: usize,
    /// Il livello piu' alto dell'evento in corso (dB).
    event_peak: f32,
    /// Campioni azzerati in tutto (per le prove e la diagnosi).
    blanked: u64,
    /// Il ritardo (campioni): `GUARD` di base, 0 senza ritardo (il buco copre solo da dove la scarica supera la soglia in poi).
    delay: usize,
}

impl Default for NoiseBlanker {
    fn default() -> Self {
        Self::new()
    }
}

impl NoiseBlanker {
    pub fn new() -> Self {
        NoiseBlanker { q95_db: -200.0, warm: 0, line: VecDeque::with_capacity(GUARD + 1), blank_left: 0, event: 0, quiet: 0, event_peak: -200.0, blanked: 0, delay: GUARD }
    }

    /// Senza ritardo: il buco comincia dove la scarica supera la soglia (se ne perde il fronte, 1-2 campioni) e senza scariche l'uscita e'
    /// l'ingresso, campione per campione. Serve al PSK, dove anche un ritardo di 2 ms sposta il sincronismo.
    pub fn without_delay() -> Self {
        NoiseBlanker { delay: 0, ..Self::new() }
    }

    /// Campioni azzerati finora.
    pub fn blanked(&self) -> u64 {
        self.blanked
    }

    /// Un campione d'ingresso; restituisce quello di 2 ms prima, azzerato se cade in un buco.
    pub fn process(&mut self, x: f32) -> f32 {
        let l = 20.0 * x.abs().max(1e-9).log10();
        self.warm += 1;
        if self.warm < 240 {
            // i primi 20 ms: il livello parte dal massimo visto
            self.q95_db = self.q95_db.max(l);
        }
        let over = self.warm >= 240 && l > self.q95_db + FACTOR_DB;
        if over {
            if self.event == 0 {
                self.event = 1;
                self.event_peak = l;
            }
            self.event_peak = self.event_peak.max(l);
            self.quiet = 0;
        } else if self.event > 0 {
            self.quiet += 1;
            if self.quiet > GUARD {
                self.event = 0;
            }
        }
        if self.event > 0 {
            self.event += 1;
            if self.event > MAX_IMPULSE {
                // troppo lungo per un impulso: e' un segnale, il livello tipico va al suo picco (non al campione di adesso, che puo' cadere
                // vicino a uno zero della sinusoide) e il buco si chiude
                self.q95_db = self.event_peak.max(self.q95_db);
                self.event = 0;
                self.blank_left = 0;
            } else if over {
                // il buco copre i 2 ms in coda (gia' nel ritardo) e i 2 ms che vengono
                self.blank_left = self.blank_left.max(2 * GUARD + 1);
            }
        }
        // quantile 95% (le scariche, rare, non lo spostano)
        self.q95_db += ETA_DB * (0.95 - if l < self.q95_db { 1.0 } else { 0.0 });
        self.line.push_back(x);
        if self.line.len() <= self.delay {
            return 0.0;
        }
        let y = self.line.pop_front().unwrap_or(0.0);
        if self.blank_left > 0 {
            self.blank_left -= 1;
            self.blanked += 1;
            return 0.0;
        }
        y
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn noise(n: usize, amp: f32, seed: u64) -> Vec<f32> {
        let mut x = seed | 1;
        (0..n)
            .map(|_| {
                x ^= x >> 12;
                x ^= x << 25;
                x ^= x >> 27;
                ((x.wrapping_mul(0x2545_F491_4F6C_DD1D) >> 40) as f32 / (1u64 << 23) as f32 - 1.0) * amp
            })
            .collect()
    }

    #[test]
    fn without_delay_the_output_is_the_input_until_an_impulse() {
        let mut input = noise(12_000, 0.1, 5);
        for (i, s) in input[6_000..6_024].iter_mut().enumerate() {
            *s += if i % 2 == 0 { 3.0 } else { -3.0 };
        }
        let mut nb = NoiseBlanker::without_delay();
        let out: Vec<f32> = input.iter().map(|&x| nb.process(x)).collect();
        assert_eq!(&out[..6_000], &input[..6_000]);
        assert!(out[6_002..6_024].iter().all(|&y| y == 0.0));
        assert_eq!(&out[6_200..], &input[6_200..]);
    }

    #[test]
    fn impulses_go_and_the_rest_stays() {
        let mut input = noise(24_000, 0.1, 7);
        for k in [6_000usize, 12_000, 18_000] {
            for (i, s) in input[k..k + 24].iter_mut().enumerate() {
                *s += if i % 2 == 0 { 3.0 } else { -3.0 };
            }
        }
        let mut nb = NoiseBlanker::new();
        let out: Vec<f32> = input.iter().map(|&x| nb.process(x)).collect();
        for k in [6_000usize, 12_000, 18_000] {
            assert!(out[k + GUARD..k + 2 * GUARD].iter().all(|&y| y == 0.0), "impulso a {k} non tolto");
        }
        // solo i buchi: tre impulsi, circa 4 ms ciascuno
        assert!(nb.blanked() < 3 * (2 * GUARD + 1 + 24) as u64 + 10, "azzerati {}", nb.blanked());
    }

    #[test]
    fn a_strong_tone_that_starts_is_not_blanked_for_long() {
        let mut input = noise(12_000, 0.01, 3);
        for (i, s) in input.iter_mut().enumerate().skip(6_000) {
            *s += (2.0 * std::f32::consts::PI * 700.0 * i as f32 / 12_000.0).sin();
        }
        let mut nb = NoiseBlanker::new();
        for &x in &input {
            nb.process(x);
        }
        assert!(nb.blanked() <= (MAX_IMPULSE + 2 * GUARD) as u64, "azzerati {}", nb.blanked());
    }

    #[test]
    fn gaussian_like_noise_is_untouched() {
        let mut nb = NoiseBlanker::new();
        for x in noise(120_000, 0.3, 11) {
            nb.process(x);
        }
        assert_eq!(nb.blanked(), 0);
    }
}
