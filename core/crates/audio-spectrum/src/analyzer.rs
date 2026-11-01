use std::{collections::VecDeque, sync::Arc};

use rustfft::{num_complex::Complex, Fft, FftPlanner};

/// Fino a 5000 Hz, come il Wide Graph di JTDX (`iz = nint(5000/df3)` in `symspec.f90`); la cascata mostra poi
/// solo la parte che passa dal filtro della radio.
pub const F_MAX_HZ: u32 = 5000;
/// Frequenza di campionamento di riferimento di JTDX: tutte le sue costanti sono in campioni a 12000 Hz.
const JTDX_RATE: f32 = 12_000.0;
/// `nfft3 = 16384` in `symspec.f90`: canali da 12000/16384 = 0,732 Hz.
const JTDX_NFFT: f32 = 16_384.0;
/// Canali di uscita: `nint(5000 / 0,732422)` = 6827, uno per canale della FFT di JTDX, da 0 a 5000 Hz.
pub const OUTPUT_BINS: usize = 6827;
/// `nsps = 6912` per tutti i modi (`MainWindow::commonActions`, «For symspec only»); la finestra e' una gaussiana
/// `exp(-z^2)` con `z = (i - nfft3/2) / (0,25 * nsps)`: larghezza 1728 campioni a 12 kHz = 0,144 s.
const WINDOW_WIDTH_SECS: f32 = 0.25 * 6912.0 / JTDX_RATE;
/// JTDX fa una riga ogni mezzo simbolo (nsps/2 = 3456 campioni = 0,288 s, 3,47 righe al secondo). Qui 15 al secondo: la
/// cascata scorre fluida e il client media le righe come l'opzione «N Avg» (4-5 righe nostre = una riga di JTDX).
const ROWS_PER_SECOND: usize = 15;
const MIN_SAMPLE_RATE: u32 = 8000;
/// `fac0 = 0.1` per un campione a 16 bit: i campioni in [-1, 1] si portano alla scala di JTDX (interi a 16 bit) prima.
const FAC0: f32 = 0.1 * 32768.0;
/// Ampiezza di JTDX (`sx = (|re| + |im|) / nfft3`) in dB, il minimo e il passo con cui diventa un byte.
/// Il client usa gli stessi valori (`app/src/render/jtdxRow.ts`): vanno cambiati insieme.
pub const DB_MIN: f32 = -40.0;
pub const DB_STEP: f32 = 80.0 / 255.0;

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

/// Trasforma l'audio in righe di 6827 byte su 0-5000 Hz (canali da 0,732 Hz), circa 15 al secondo, con lo spettro di JTDX:
/// finestra gaussiana, FFT di 1,365 s, ampiezza `(|re| + |im|) / n`. Il byte e' l'ampiezza in dB (`DB_MIN`, `DB_STEP`):
/// la scala mediana, i canali per pixel, il guadagno e i colori li applica il client come il Wide Graph.
pub struct SpectrumAnalyzer {
    fft: Arc<dyn Fft<f32>>,
    n: usize,
    hop: usize,
    window: Vec<f32>,
    samples: VecDeque<f32>,
    since_row: usize,
    buf: Vec<Complex<f32>>,
    scratch: Vec<Complex<f32>>,
}

impl SpectrumAnalyzer {
    pub fn new(sample_rate: u32) -> Result<Self, AnalyzerError> {
        if sample_rate < MIN_SAMPLE_RATE {
            return Err(AnalyzerError::SampleRateTooLow(sample_rate));
        }
        let rate = sample_rate as f32;
        // canali da 12000/16384 Hz come JTDX, qualunque sia la frequenza di campionamento
        let n = ((rate * JTDX_NFFT / JTDX_RATE).round() as usize).max(2);
        let hop = ((sample_rate as usize) / ROWS_PER_SECOND).max(1);
        let width = WINDOW_WIDTH_SECS * rate;
        // `z = (i - nfft3/2) / width` con i da 1 (indice di Fortran): il picco cade a meta' della finestra
        let window = (0..n)
            .map(|i| {
                let z = ((i + 1) as f32 - n as f32 / 2.0) / width;
                (-z * z).exp()
            })
            .collect();
        let fft = FftPlanner::<f32>::new().plan_fft_forward(n);
        let scratch = vec![Complex::new(0.0, 0.0); fft.get_inplace_scratch_len()];
        // come JTDX, che azzera i campioni mancanti all'inizio: le prime righe sono scure invece di aspettare la finestra piena
        let mut samples = VecDeque::with_capacity(n + 1);
        samples.extend(std::iter::repeat_n(0.0, n));
        Ok(SpectrumAnalyzer { fft, n, hop, window, samples, since_row: 0, buf: vec![Complex::new(0.0, 0.0); n], scratch })
    }

    /// Aggiunge campioni mono in [-1, 1]; restituisce le righe diventate pronte (anche zero).
    /// I campioni non finiti (NaN, infinito) valgono zero.
    pub fn push(&mut self, input: &[f32]) -> Vec<Vec<u8>> {
        let mut rows = Vec::new();
        for &s in input {
            self.samples.push_back(if s.is_finite() { s } else { 0.0 });
            self.samples.pop_front();
            self.since_row += 1;
            if self.since_row >= self.hop {
                self.since_row = 0;
                rows.push(self.compute_row());
            }
        }
        rows
    }

    fn compute_row(&mut self) -> Vec<u8> {
        for (i, s) in self.samples.iter().enumerate() {
            self.buf[i] = Complex::new(s * FAC0 * self.window[i], 0.0);
        }
        self.fft.process_with_scratch(&mut self.buf, &mut self.scratch);
        let fac1 = 1.0 / self.n as f32;
        (0..OUTPUT_BINS)
            .map(|j| {
                // sotto i 10 kHz di campionamento i canali oltre Nyquist restano vuoti
                if j >= self.n / 2 {
                    return 0;
                }
                let c = self.buf[j];
                let amplitude = (c.re.abs() + c.im.abs()) * fac1;
                let db = 20.0 * amplitude.max(1e-12).log10();
                ((db - DB_MIN) / DB_STEP).clamp(0.0, 255.0).round() as u8
            })
            .collect()
    }
}
