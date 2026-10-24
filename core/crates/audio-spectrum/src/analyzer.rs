use std::{collections::VecDeque, sync::Arc};

use rustfft::{num_complex::Complex, Fft, FftPlanner};

pub const F_MAX_HZ: u32 = 3000;
pub const OUTPUT_BINS: usize = 1024;
const ROWS_PER_SECOND: usize = 15;
const MIN_SAMPLE_RATE: u32 = 8000;
/// Scala dei valori: -110 dBFS (nero) ... -20 dBFS (massimo).
const FLOOR_DB: f32 = -110.0;
const CEIL_DB: f32 = -20.0;

#[derive(Debug, PartialEq, Eq)]
pub enum AnalyzerError {
    SampleRateTooLow(u32),
}

impl std::fmt::Display for AnalyzerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AnalyzerError::SampleRateTooLow(r) => {
                write!(f, "frequenza di campionamento troppo bassa: {r} Hz (minimo {MIN_SAMPLE_RATE} Hz)")
            }
        }
    }
}

impl std::error::Error for AnalyzerError {}

/// Trasforma l'audio in righe di 1024 valori `u8` su 0-3000 Hz, circa 15 al secondo.
pub struct SpectrumAnalyzer {
    fft: Arc<dyn Fft<f32>>,
    n: usize,
    hop: usize,
    window: Vec<f32>,
    samples: VecDeque<f32>,
    since_row: usize,
    buf: Vec<Complex<f32>>,
    scratch: Vec<Complex<f32>>,
    /// Per ogni bin di uscita: intervallo dei bin FFT da cui prendere il massimo.
    ranges: Vec<(usize, usize)>,
    /// Ampiezza di un seno a fondo scala dopo la finestra di Hann.
    full_scale: f32,
}

impl SpectrumAnalyzer {
    pub fn new(sample_rate: u32) -> Result<Self, AnalyzerError> {
        if sample_rate < MIN_SAMPLE_RATE {
            return Err(AnalyzerError::SampleRateTooLow(sample_rate));
        }
        let rate = sample_rate as usize;
        // risoluzione pari o migliore di un bin di uscita (circa 2,9 Hz)
        let n = (rate * OUTPUT_BINS).div_ceil(F_MAX_HZ as usize).next_power_of_two();
        let hop = (rate / ROWS_PER_SECOND).max(1);
        let window = (0..n)
            .map(|i| 0.5 - 0.5 * (2.0 * std::f32::consts::PI * i as f32 / (n - 1) as f32).cos())
            .collect();
        let fft = FftPlanner::<f32>::new().plan_fft_forward(n);
        let scratch = vec![Complex::new(0.0, 0.0); fft.get_inplace_scratch_len()];
        let ranges = (0..OUTPUT_BINS)
            .map(|i| {
                let bin = |edge: usize| edge * F_MAX_HZ as usize * n / (OUTPUT_BINS * rate);
                let lo = bin(i);
                let hi = bin(i + 1).max(lo + 1).min(n / 2);
                (lo.min(hi - 1), hi)
            })
            .collect();
        Ok(SpectrumAnalyzer {
            fft,
            n,
            hop,
            window,
            samples: VecDeque::with_capacity(n + 1),
            since_row: 0,
            buf: vec![Complex::new(0.0, 0.0); n],
            scratch,
            ranges,
            full_scale: n as f32 / 4.0,
        })
    }

    /// Aggiunge campioni mono in [-1, 1]; restituisce le righe diventate pronte (anche zero).
    /// I campioni non finiti (NaN, infinito) valgono zero.
    pub fn push(&mut self, input: &[f32]) -> Vec<Vec<u8>> {
        let mut rows = Vec::new();
        for &s in input {
            self.samples.push_back(if s.is_finite() { s } else { 0.0 });
            if self.samples.len() > self.n {
                self.samples.pop_front();
            }
            self.since_row += 1;
            if self.samples.len() == self.n && self.since_row >= self.hop {
                self.since_row = 0;
                rows.push(self.compute_row());
            }
        }
        rows
    }

    fn compute_row(&mut self) -> Vec<u8> {
        for (i, s) in self.samples.iter().enumerate() {
            self.buf[i] = Complex::new(s * self.window[i], 0.0);
        }
        self.fft.process_with_scratch(&mut self.buf, &mut self.scratch);
        self.ranges
            .iter()
            .map(|&(lo, hi)| {
                let magnitude = self.buf[lo..hi].iter().map(|c| c.norm()).fold(0.0f32, f32::max);
                let db = 20.0 * (magnitude.max(1e-12) / self.full_scale).log10();
                ((db - FLOOR_DB) / (CEIL_DB - FLOOR_DB) * 255.0).clamp(0.0, 255.0).round() as u8
            })
            .collect()
    }
}
