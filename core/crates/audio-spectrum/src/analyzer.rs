use std::{collections::VecDeque, sync::Arc};

use rustfft::{num_complex::Complex, Fft, FftPlanner};

use crate::click::{ClickDetector, ClickSpan};

/// Fino a 5000 Hz, come il Wide Graph di JTDX (`iz = nint(5000/df3)` in `symspec.f90`); la cascata mostra poi
/// solo la parte che passa dal filtro della radio.
pub const F_MAX_HZ: u32 = 5000;
/// Frequenza di campionamento di riferimento di JTDX: tutte le sue costanti sono in campioni a 12000 Hz.
const JTDX_RATE: f32 = 12_000.0;
/// `nfft3 = 16384` in `symspec.f90`: canali da 12000/16384 = 0,732 Hz.
const JTDX_NFFT: f32 = 16_384.0;
/// Canali di uscita: `nint(5000 / 0,732422)` = 6827, uno per canale della FFT di JTDX, da 0 a 5000 Hz.
pub const OUTPUT_BINS: usize = 6827;
/// Coefficienti della finestra di Nuttall di WSJT-X (`lib/nuttal_window.f90`), applicata su tutti i `nfft3` campioni quando
/// e' scelto «Low sidelobes» (`bLowSidelobes`, predefinito `true` in `Configuration.cpp`): `w(k) = a0 + a1 cos(2 pi k/n) + ...`.
const NUTTALL: [f32; 4] = [0.363_581_9, -0.489_177_5, 0.136_599_5, -0.010_641_1];
/// Larghezza della gaussiana di JTDX usata prima (0,25 * 6912 campioni a 12 kHz = 0,144 s): serve solo a tenere il rumore di
/// fondo allo stesso livello di prima, cosi' Gain e Zero gia' regolati non cambiano.
const OLD_GAUSS_WIDTH_SECS: f32 = 0.25 * 6912.0 / JTDX_RATE;
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
/// Media di `|cos x| + |sin x|` su una fase qualsiasi (4/pi): rapporto tra l'ampiezza L1 di JTDX e il modulo.
const L1_MEAN: f32 = 4.0 / std::f32::consts::PI;

/// Anti-click: campioni da spegnere prima e dopo il click trovato (s) e fronte morbido ai due lati (s).
const CLICK_PAD_S: f32 = 0.001;
const CLICK_TAPER_S: f32 = 0.002;
/// Campioni passati al rivelatore dei click alla volta, e quanti click recenti si tengono.
const CLICK_CHUNK: usize = 64;
const RECENT_CLICKS: usize = 64;

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
/// finestra di Nuttall di WSJT-X su tutta la FFT di 1,365 s, modulo `|X| / n` (scalato di 4/pi). Il byte e' l'ampiezza in dB (`DB_MIN`, `DB_STEP`):
/// la scala mediana, i canali per pixel, il guadagno e i colori li applica il client come il Wide Graph.
///
/// Anti-click (`aurora_core::waterfall::anticlick`, o [`SpectrumAnalyzer::set_anticlick`] nelle prove): i click di manipolazione trovati
/// da [`ClickDetector`] si spengono, con fronti morbidi, prima della trasformata; gli istanti dei click si leggono con
/// [`SpectrumAnalyzer::recent_clicks`] solo con l'opzione accesa (spenta, il rivelatore non lavora).
pub struct SpectrumAnalyzer {
    fft: Arc<dyn Fft<f32>>,
    n: usize,
    hop: usize,
    window: Vec<f32>,
    samples: VecDeque<f32>,
    since_row: usize,
    buf: Vec<Complex<f32>>,
    scratch: Vec<Complex<f32>>,
    /// Per ogni campione della finestra quanto resta dopo l'anti-click (1 = tutto).
    gains: VecDeque<f32>,
    clicks: ClickDetector,
    pending: Vec<f32>,
    recent: VecDeque<ClickSpan>,
    total: u64,
    pad: u64,
    taper: u64,
    anticlick: Option<bool>,
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
        // `symspec.f90` di WSJT-X: `xc(0:n-1) = w3(1:n) * xc` con `w3` di Nuttall su tutta la FFT (1,365 s). Ogni riga raccoglie
        // circa mezzo secondo di segnale (tre simboli FT8): la traccia resta continua e i toni, larghi 1,4 Hz, restano separati.
        let raw: Vec<f32> = (0..n)
            .map(|k| {
                let x = 2.0 * std::f64::consts::PI * k as f64 / n as f64;
                let [a0, a1, a2, a3] = NUTTALL.map(f64::from);
                (a0 + a1 * x.cos() + a2 * (2.0 * x).cos() + a3 * (3.0 * x).cos()) as f32
            })
            .collect();
        // stessa energia della gaussiana di prima (somma dei quadrati sqrt(pi/2) * larghezza): il rumore resta dov'era
        let old_energy = (std::f32::consts::PI / 2.0).sqrt() * OLD_GAUSS_WIDTH_SECS * rate;
        let energy: f32 = raw.iter().map(|w| w * w).sum();
        let norm = if energy > 0.0 { (old_energy / energy).sqrt() } else { 1.0 };
        let window = raw.into_iter().map(|w| w * norm).collect();
        let fft = FftPlanner::<f32>::new().plan_fft_forward(n);
        let scratch = vec![Complex::new(0.0, 0.0); fft.get_inplace_scratch_len()];
        // come JTDX, che azzera i campioni mancanti all'inizio: le prime righe sono scure invece di aspettare la finestra piena
        let mut samples = VecDeque::with_capacity(n + 1);
        samples.extend(std::iter::repeat_n(0.0, n));
        let mut gains = VecDeque::with_capacity(n + 1);
        gains.extend(std::iter::repeat_n(1.0, n));
        Ok(SpectrumAnalyzer {
            fft,
            n,
            hop,
            window,
            samples,
            since_row: 0,
            buf: vec![Complex::new(0.0, 0.0); n],
            scratch,
            gains,
            clicks: ClickDetector::new(sample_rate),
            pending: Vec::with_capacity(CLICK_CHUNK),
            recent: VecDeque::with_capacity(RECENT_CLICKS + 1),
            total: 0,
            pad: (CLICK_PAD_S * rate).round() as u64,
            taper: (CLICK_TAPER_S * rate).round().max(1.0) as u64,
            anticlick: None,
        })
    }

    /// Accende o spegne l'anti-click per questo analizzatore, al posto dell'opzione comune (`None` torna all'opzione comune).
    pub fn set_anticlick(&mut self, on: Option<bool>) {
        self.anticlick = on;
    }

    /// Gli ultimi click trovati (al massimo 64), in campioni contati dall'inizio del flusso (vedi [`SpectrumAnalyzer::samples_seen`]).
    pub fn recent_clicks(&self) -> impl Iterator<Item = &ClickSpan> {
        self.recent.iter()
    }

    /// Campioni arrivati finora.
    pub fn samples_seen(&self) -> u64 {
        self.total
    }

    /// Passa al rivelatore i campioni in attesa e spegne nella finestra quelli dei click trovati.
    fn detect_clicks(&mut self) {
        if self.pending.is_empty() {
            return;
        }
        let spans = self.clicks.push(&self.pending);
        self.pending.clear();
        // il campione assoluto del primo della finestra (all'avvio la finestra e' piena di zeri che non contano)
        let first = self.total as i64 - self.n as i64;
        for span in spans {
            let (a, b) = (span.start.saturating_sub(self.pad), span.end + self.pad);
            for abs in a.saturating_sub(self.taper).max(first.max(0) as u64)..(b + self.taper).min(self.total) {
                // fronte a coseno rialzato prima e dopo il tratto spento
                let d = if abs < a {
                    a - abs
                } else if abs >= b {
                    abs - b + 1
                } else {
                    0
                };
                let g = if d == 0 { 0.0 } else { 0.5 - 0.5 * (std::f32::consts::PI * d as f32 / self.taper as f32).cos() };
                if let Some(slot) = self.gains.get_mut((abs as i64 - first) as usize) {
                    *slot = slot.min(g);
                }
            }
            self.recent.push_back(span);
            while self.recent.len() > RECENT_CLICKS {
                self.recent.pop_front();
            }
        }
    }

    /// Aggiunge campioni mono in [-1, 1]; restituisce le righe diventate pronte (anche zero).
    /// I campioni non finiti (NaN, infinito) valgono zero.
    pub fn push(&mut self, input: &[f32]) -> Vec<Vec<u8>> {
        let mut rows = Vec::new();
        let anticlick = self.anticlick.unwrap_or_else(aurora_core::waterfall::anticlick);
        for &s in input {
            let s = if s.is_finite() { s } else { 0.0 };
            self.samples.push_back(s);
            self.samples.pop_front();
            self.gains.push_back(1.0);
            self.gains.pop_front();
            self.total += 1;
            // con l'anti-click spento il rivelatore non lavora (erano circa 375 trasformate da 256 punti al secondo a 48 kHz buttate)
            if anticlick {
                self.pending.push(s);
                if self.pending.len() >= CLICK_CHUNK {
                    self.detect_clicks();
                }
            }
            self.since_row += 1;
            if self.since_row >= self.hop {
                self.since_row = 0;
                rows.push(self.compute_row(anticlick));
            }
        }
        rows
    }

    fn compute_row(&mut self, anticlick: bool) -> Vec<u8> {
        if anticlick {
            // i campioni piu' recenti hanno peso nullo nella finestra: il ritardo del rivelatore (qualche ms) non conta
            self.detect_clicks();
            for (i, (s, g)) in self.samples.iter().zip(&self.gains).enumerate() {
                self.buf[i] = Complex::new(s * g * FAC0 * self.window[i], 0.0);
            }
        } else {
            for (i, s) in self.samples.iter().enumerate() {
                self.buf[i] = Complex::new(s * FAC0 * self.window[i], 0.0);
            }
        }
        self.fft.process_with_scratch(&mut self.buf, &mut self.scratch);
        let fac1 = 1.0 / self.n as f32;
        (0..OUTPUT_BINS)
            .map(|j| {
                // sotto i 10 kHz di campionamento i canali oltre Nyquist restano vuoti
                if j >= self.n / 2 {
                    return 0;
                }
                // modulo vero come la potenza di `symspec.f90` di WSJT-X (`real(cx)**2 + aimag(cx)**2`): `|re| + |im|` di JTDX
                // cambia fino a 3 dB con la fase, che ruota da una riga all'altra, e rende la traccia granulosa. `4/pi` e' la media
                // di `|cos| + |sin|`: il livello medio resta quello di prima
                let amplitude = self.buf[j].norm() * fac1 * L1_MEAN;
                let db = 20.0 * amplitude.max(1e-12).log10();
                ((db - DB_MIN) / DB_STEP).clamp(0.0, 255.0).round() as u8
            })
            .collect()
    }
}

