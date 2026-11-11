//! Demodulatore FM: dal segnale audio a 12 kHz alla frequenza istantanea, campione per campione.
//!
//! Come il demodulatore di QSSTV (mescolatore a 1900 Hz, filtro passa basso, differenza di fase fra campioni) scritto da capo in Rust.

use std::f32::consts::{PI, TAU};

/// Frequenza di lavoro del decodificatore.
pub const RATE: u32 = 12_000;
const CENTER: f32 = 1900.0;
const TAPS: usize = 17;
const CUTOFF: f32 = 1500.0;

pub struct FmDemod {
    taps: [f32; TAPS],
    hist_re: [f32; TAPS],
    hist_im: [f32; TAPS],
    pos: usize,
    lo_phase: f32,
    prev: (f32, f32),
    /// Potenza media (per l'indicatore del livello).
    power: f32,
}

impl Default for FmDemod {
    fn default() -> Self {
        Self::new()
    }
}

impl FmDemod {
    pub fn new() -> FmDemod {
        let mut taps = [0.0f32; TAPS];
        let fc = CUTOFF / RATE as f32;
        let mid = (TAPS - 1) as f32 / 2.0;
        let mut sum = 0.0;
        for (i, t) in taps.iter_mut().enumerate() {
            let x = i as f32 - mid;
            let sinc = if x == 0.0 { 2.0 * fc } else { (TAU * fc * x).sin() / (PI * x) };
            let window = 0.54 - 0.46 * (TAU * i as f32 / (TAPS - 1) as f32).cos();
            *t = sinc * window;
            sum += *t;
        }
        for t in &mut taps {
            *t /= sum;
        }
        FmDemod { taps, hist_re: [0.0; TAPS], hist_im: [0.0; TAPS], pos: 0, lo_phase: 0.0, prev: (1.0, 0.0), power: 0.0 }
    }

    /// Demodula `input` e aggiunge le frequenze (Hz) a `out`, una per campione.
    pub fn process(&mut self, input: &[f32], out: &mut Vec<f32>) {
        self.process_full(input, out, None);
    }

    /// Come [`FmDemod::process`] e in piu' i prodotti z(n)·z*(n-1) in `products`: sommati su un pixel danno una stima della
    /// frequenza pesata per l'ampiezza, senza gli scatti del rumore (discriminatore a prodotto mediato).
    pub fn process_full(&mut self, input: &[f32], out: &mut Vec<f32>, mut products: Option<&mut Vec<[f32; 2]>>) {
        let step = TAU * CENTER / RATE as f32;
        for &x in input {
            let (s, c) = self.lo_phase.sin_cos();
            self.lo_phase = (self.lo_phase + step) % TAU;
            self.hist_re[self.pos] = x * c;
            self.hist_im[self.pos] = -x * s;
            self.pos = (self.pos + 1) % TAPS;
            let (mut re, mut im) = (0.0f32, 0.0f32);
            let mut k = self.pos;
            for t in &self.taps {
                re += t * self.hist_re[k];
                im += t * self.hist_im[k];
                k += 1;
                if k == TAPS {
                    k = 0;
                }
            }
            let (pr, pi) = self.prev;
            // z * conj(prev)
            let dr = re * pr + im * pi;
            let di = im * pr - re * pi;
            self.prev = (re, im);
            if let Some(p) = products.as_deref_mut() {
                p.push([dr, di]);
            }
            let df = di.atan2(dr) * RATE as f32 / TAU;
            self.power = 0.999 * self.power + 0.001 * (re * re + im * im);
            out.push(CENTER + df);
        }
    }

    /// Livello del segnale nella banda SSTV, in dB rispetto al fondo scala.
    pub fn level_db(&self) -> f32 {
        10.0 * (self.power * 4.0).max(1e-10).log10()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn estimates_tone_frequencies() {
        for hz in [1100.0f32, 1200.0, 1500.0, 1900.0, 2300.0] {
            let input: Vec<f32> = (0..2400).map(|n| (TAU * hz * n as f32 / RATE as f32).sin()).collect();
            let mut out = Vec::new();
            FmDemod::new().process(&input, &mut out);
            let mean = out[200..].iter().sum::<f32>() / (out.len() - 200) as f32;
            assert!((mean - hz).abs() < 3.0, "{hz}: {mean}");
        }
    }
}


/// Frequenza (Hz) dalla somma dei prodotti z(n)·z*(n-1).
pub fn product_to_hz(re: f32, im: f32) -> f32 {
    CENTER + im.atan2(re) * RATE as f32 / TAU
}