//! Stima del rumore del ricevitore CW, in ogni istante, per il decodificatore probabilistico: niente soglie, una varianza del rumore che
//! segue il volume, l'AGC della radio che pompa, il QRN largo e il QSB.
//!
//! Due misure, scritte per Aurora:
//! - **bande laterali**: accanto al filtro del tono, due filtri della stessa forma ai due lati (centrati a 2,5 larghezze dal tono, dentro
//!   la fascia da 1,5 a 3,5 larghezze). La larghezza segue la velocità: circa due volte la velocità di simbolo (30 Hz a 20 WPM, 60 Hz a
//!   40 WPM). Le due potenze, mediate su 120 ms e riportate alla banda di rumore del filtro del decodificatore, dicono quanto rumore c'è
//!   adesso. Se un lato è molto più forte dell'altro (medie di un secondo) c'è un'altra stazione da quella parte e si usa solo il lato
//!   pulito;
//! - **minimo scorrevole**: la potenza del filtro del tono mediata su 40 ms, il minimo degli ultimi 3 secondi diviso per il valore atteso
//!   del minimo sul solo rumore (sul solo rumore il minimo di tante medie sta sotto la media: con il filtro di 150 Hz a circa un quinto,
//!   misurato nella prova `noise_estimates_match_the_decoder_filter`). Fa da pavimento alle bande laterali.
//!
//! Le misure lavorano sulla strada larga del ricevitore (±125 Hz attorno al tono, campionamento dopo la decimazione) e sulla banda base
//! del decodificatore.

use std::collections::VecDeque;
use std::f64::consts::PI;

/// Media delle potenze del minimo scorrevole (s) e delle bande laterali (s): con 40 ms le due bande laterali strette hanno troppo pochi
/// campioni indipendenti e la stima balla (prova `cw_confronto`, media di tutte le righe: 30,4 con 40 ms, 26,7 con 120 ms, 27,3 con
/// 250 ms; con il restringimento verso la media lenta del decodificatore 26,2 con 40 ms e 25,9 con 120 ms).
const AVG_S: f64 = 0.04;
const SIDE_AVG_S: f64 = 0.12;
/// Finestra del minimo scorrevole (s) e lunghezza dei suoi blocchi (s).
const MIN_WINDOW_S: f64 = 3.0;
const MIN_BLOCK_S: f64 = 0.5;
/// Centro delle bande laterali in larghezze del filtro del tono, e distanza massima dal tono (Hz): la strada larga arriva a ±125 Hz.
const SIDE_AT: f64 = 2.5;
const SIDE_MAX_HZ: f64 = 105.0;
/// Larghezza del filtro (Hz): due volte la velocità di simbolo (1 / unità), fra questi limiti.
const WIDTH_MIN_HZ: f64 = 20.0;
const WIDTH_MAX_HZ: f64 = 60.0;
/// Un lato piu' forte dell'altro di cosi' (rapporto delle medie lunghe) ha un'altra stazione: si usa l'altro.
const SIDE_MISMATCH: f64 = 2.0;
/// Media lunga (s) per decidere se un lato e' occupato.
const SIDE_LONG_S: f64 = 1.0;
/// Quante medie indipendenti valgono, per il minimo, le medie mobili di 3 secondi (misurato: vedi `expected_min_ratio`).
const MIN_EQUIV_N: f64 = 600.0;

/// Larghezza del filtro del tono per la velocità data (Hz).
pub(crate) fn tone_width_hz(wpm: f64) -> f64 {
    (2.0 * wpm / 1.2).clamp(WIDTH_MIN_HZ, WIDTH_MAX_HZ)
}

/// Media mobile a scatola.
struct Boxcar {
    buf: VecDeque<f64>,
    sum: f64,
    len: usize,
}

impl Boxcar {
    fn new(len: usize) -> Self {
        Boxcar { buf: VecDeque::with_capacity(len + 1), sum: 0.0, len: len.max(1) }
    }

    fn push(&mut self, x: f64) -> f64 {
        self.buf.push_back(x);
        self.sum += x;
        while self.buf.len() > self.len {
            self.sum -= self.buf.pop_front().unwrap_or(0.0);
        }
        // la somma corre: ogni tanto si rifa' da capo, per non accumulare errori di arrotondamento
        if self.buf.len() == self.len && self.sum < 0.0 {
            self.sum = self.buf.iter().sum();
        }
        self.sum.max(0.0) / self.buf.len() as f64
    }
}

/// Le bande laterali: rumore stimato (potenza per campione del filtro del decodificatore).
pub(crate) struct SidebandNoise {
    fs: f64,
    /// Finestra di Hann del filtro delle bande laterali, rotazione per campione (rad) e campioni ruotati (+ e -).
    win: Vec<f64>,
    theta: f64,
    phase: f64,
    hist: [VecDeque<(f64, f64)>; 2],
    avg: [Boxcar; 2],
    /// Medie lunghe dei due lati (per scegliere quello pulito).
    long: [f64; 2],
    /// Fattore che porta la potenza di una banda laterale alla potenza di rumore del filtro del decodificatore (rumore bianco).
    scale: f64,
    /// Ritardo della misura rispetto alla banda base del decodificatore (campioni decimati).
    lag: usize,
    width_hz: f64,
    wide_taps: Vec<f32>,
    tone_taps_energy: f64,
    tone_delay_s: f64,
    decim: usize,
    /// Le due potenze dell'ultimo campione (per le prove).
    last: [f64; 2],
}

impl SidebandNoise {
    /// `wide_taps`: il filtro della strada larga a 12 kHz; `tone_taps`: quello del decodificatore; `decim`: la decimazione; `fs`: il
    /// campionamento dopo la decimazione; `wpm`: la velocità per la larghezza del filtro.
    pub(crate) fn new(fs: f64, decim: usize, wide_taps: &[f32], tone_taps: &[f32], wpm: f64) -> Self {
        let rate = fs * decim as f64;
        let mut n = SidebandNoise {
            fs,
            win: Vec::new(),
            theta: 0.0,
            phase: 0.0,
            hist: [VecDeque::new(), VecDeque::new()],
            avg: [Boxcar::new(1), Boxcar::new(1)],
            long: [0.0; 2],
            scale: 1.0,
            lag: 0,
            width_hz: 0.0,
            wide_taps: wide_taps.to_vec(),
            tone_taps_energy: tone_taps.iter().map(|&h| f64::from(h) * f64::from(h)).sum(),
            tone_delay_s: (tone_taps.len().saturating_sub(1)) as f64 / 2.0 / rate,
            decim,
            last: [0.0; 2],
        };
        n.set_wpm(wpm);
        n
    }

    /// La larghezza segue la velocità: si ricalcola solo se cambia di piu' del 15%.
    pub(crate) fn set_wpm(&mut self, wpm: f64) {
        let w = tone_width_hz(wpm);
        if self.width_hz > 0.0 && (w / self.width_hz - 1.0).abs() < 0.15 {
            return;
        }
        self.width_hz = w;
        let rate = self.fs * self.decim as f64;
        // la finestra di Hann lunga 2 / larghezza ha il lobo principale largo ±1 larghezza
        let len = ((2.0 / w) * self.fs).round().max(3.0) as usize;
        self.win = (0..len).map(|i| 0.5 - 0.5 * (2.0 * PI * (i as f64 + 0.5) / len as f64).cos()).collect();
        let off = (SIDE_AT * w).min(SIDE_MAX_HZ);
        self.theta = 2.0 * PI * off / self.fs;
        // la risposta complessiva a 12 kHz di un lato: filtro largo, decimazione, rotazione e finestra
        let d = self.decim;
        let wl = self.wide_taps.len();
        let total = wl + (len - 1) * d;
        let mut g = vec![(0.0_f64, 0.0_f64); total];
        for (j, &wj) in self.win.iter().enumerate() {
            let (s, c) = (self.theta * j as f64).sin_cos();
            for (k, &h) in self.wide_taps.iter().enumerate() {
                let e = &mut g[k + j * d];
                e.0 += wj * c * f64::from(h);
                e.1 += wj * s * f64::from(h);
            }
        }
        let energy: f64 = g.iter().map(|(a, b)| a * a + b * b).sum();
        self.scale = if energy > 0.0 { self.tone_taps_energy / energy } else { 1.0 };
        // la misura (filtro largo, finestra, media) arriva dopo la banda base del decodificatore: di quanto
        let wide_delay = (wl.saturating_sub(1)) as f64 / 2.0 / rate;
        let win_delay = (len - 1) as f64 / 2.0 / self.fs;
        let avg_len = (SIDE_AVG_S * self.fs).round().max(1.0) as usize;
        let avg_delay = (avg_len - 1) as f64 / 2.0 / self.fs;
        self.lag = ((wide_delay + win_delay + avg_delay - self.tone_delay_s) * self.fs).round().max(0.0) as usize;
        // le medie e i campioni gia' visti restano: un cambio di larghezza non lascia il decodificatore senza misura
        if self.avg[0].len != avg_len {
            self.avg = [Boxcar::new(avg_len), Boxcar::new(avg_len)];
        }
        // i campioni gia' ruotati valgono solo con la stessa rotazione
        for h in self.hist.iter_mut() {
            h.clear();
        }
    }

    /// Di quanti campioni la misura e' in ritardo sulla banda base del decodificatore.
    pub(crate) fn lag(&self) -> usize {
        self.lag
    }

    /// Un campione della strada larga; restituisce il rumore stimato (potenza per campione del filtro del decodificatore), 0 finche' le
    /// medie non sono piene.
    pub(crate) fn push(&mut self, w: (f32, f32)) -> f64 {
        let (x, y) = (f64::from(w.0), f64::from(w.1));
        self.phase += self.theta;
        if self.phase > 2.0 * PI {
            self.phase -= 2.0 * PI;
        }
        let (s, c) = self.phase.sin_cos();
        // un lato sopra il tono (riportato a zero ruotando all'indietro) e uno sotto
        let rotated = [(x * c + y * s, y * c - x * s), (x * c - y * s, y * c + x * s)];
        let len = self.win.len();
        let mut est = [0.0; 2];
        for side in 0..2 {
            let h = &mut self.hist[side];
            h.push_back(rotated[side]);
            while h.len() > len {
                h.pop_front();
            }
            if h.len() < len {
                return 0.0;
            }
            let (mut re, mut im) = (0.0, 0.0);
            for (k, &(a, b)) in h.iter().enumerate() {
                re += self.win[k] * a;
                im += self.win[k] * b;
            }
            let p = self.scale * (re * re + im * im);
            est[side] = self.scale * self.avg[side].push(re * re + im * im);
            let a = (SIDE_LONG_S * self.fs).recip();
            self.long[side] = if self.long[side] > 0.0 { self.long[side] + a * (p - self.long[side]) } else { p };
        }
        self.last = est;
        // un lato occupato (medie lunghe): solo l'altro; altrimenti la media dei due
        if self.long[0] > SIDE_MISMATCH * self.long[1] {
            est[1]
        } else if self.long[1] > SIDE_MISMATCH * self.long[0] {
            est[0]
        } else {
            0.5 * (est[0] + est[1])
        }
    }
}

/// Il minimo scorrevole della potenza del filtro del decodificatore.
pub(crate) struct MinTracker {
    avg: Boxcar,
    block: usize,
    in_block: usize,
    current: f64,
    blocks: VecDeque<f64>,
    n_blocks: usize,
    /// Valore atteso del minimo diviso per la media, sul solo rumore.
    bias: f64,
}

impl MinTracker {
    /// `fs`: campionamento; `dof`: gradi di liberta' (reali) di una media di 40 ms del rumore dopo il filtro.
    pub(crate) fn new(fs: f64, dof: f64) -> Self {
        let n_blocks = (MIN_WINDOW_S / MIN_BLOCK_S).round() as usize;
        MinTracker {
            avg: Boxcar::new((AVG_S * fs).round().max(1.0) as usize),
            block: (MIN_BLOCK_S * fs).round().max(1.0) as usize,
            in_block: 0,
            current: f64::MAX,
            blocks: VecDeque::with_capacity(n_blocks + 1),
            n_blocks,
            bias: expected_min_ratio(dof, MIN_EQUIV_N),
        }
    }

    /// Una potenza; restituisce il rumore stimato (0 finche' non e' passato il primo blocco).
    pub(crate) fn push(&mut self, p: f64) -> f64 {
        let a = self.avg.push(p);
        self.current = self.current.min(a);
        self.in_block += 1;
        if self.in_block >= self.block {
            self.blocks.push_back(self.current);
            while self.blocks.len() > self.n_blocks {
                self.blocks.pop_front();
            }
            self.current = f64::MAX;
            self.in_block = 0;
        }
        let m = self.blocks.iter().copied().fold(self.current, f64::min);
        if self.blocks.is_empty() || m == f64::MAX { 0.0 } else { m / self.bias }
    }
}

/// Il minimo di `n` medie indipendenti di rumore (chi quadro con `dof` gradi di liberta', diviso per i gradi), diviso per la media:
/// quantile 1/(n+1) con l'approssimazione di Wilson e Hilferty. Le medie mobili di 40 ms prese a ogni campione per 3 secondi valgono
/// circa 600 medie indipendenti (misurato con le bande del filtro da 150 a 1000 Hz).
pub(crate) fn expected_min_ratio(dof: f64, n: f64) -> f64 {
    let dof = dof.max(2.0);
    let p = 1.0 / (n.max(1.0) + 1.0);
    let z = -normal_quantile_upper(p);
    let v = 2.0 / (9.0 * dof);
    (1.0 - v + z * v.sqrt()).max(0.05).powi(3)
}

/// Il valore z con probabilita' `p` di essere superato da una normale standard (approssimazione di Abramowitz e Stegun 26.2.23).
fn normal_quantile_upper(p: f64) -> f64 {
    let p = p.clamp(1e-12, 0.5);
    let t = (-2.0 * p.ln()).sqrt();
    t - (2.515_517 + 0.802_853 * t + 0.010_328 * t * t) / (1.0 + 1.432_788 * t + 0.189_269 * t * t + 0.001_308 * t * t * t)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn widths_follow_the_speed() {
        assert!((tone_width_hz(20.0) - 33.3).abs() < 0.5);
        assert!((tone_width_hz(40.0) - 60.0).abs() < 0.5);
        assert_eq!(tone_width_hz(5.0), WIDTH_MIN_HZ);
    }

    #[test]
    fn the_quantile_is_sane() {
        assert!((normal_quantile_upper(0.025) - 1.96).abs() < 0.01);
        assert!((normal_quantile_upper(0.5)).abs() < 0.01);
        // 75 medie di 12 gradi: il minimo sta a circa un terzo della media
        let r = expected_min_ratio(12.0, 75.0);
        assert!(r > 0.25 && r < 0.45, "{r}");
    }
}
