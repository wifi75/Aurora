//! Misure sul segnale WEFAX che servono allo squelch (SQL) e all'AFC.
//!
//! - [`SignalMeter`]: livello del segnale da 0 a 100 come rapporto tra la potenza nella banda del fax (da nero a bianco) e quella delle due
//!   fasce di rumore ai lati (la piu' forte delle due, cosi' il filtro SSB della radio che taglia una fascia non alza il livello del solo
//!   rumore). 0 dB (solo rumore) vale 0, 20 dB valgono 100.
//! - [`Afc`]: istogramma della frequenza istantanea demodulata (senza i limiti di nero e bianco). Nei toni APT, nella fasatura e nelle
//!   cartine (fondo bianco e tratti neri) ci sono due picchi distanti quanto l'escursione: il loro punto di mezzo e' la sottoportante vera.

use std::f64::consts::TAU;

use crate::demod::RATE;

/// Campioni per misura del livello (circa 85 ms a 12 kHz).
pub const METER_BLOCK: usize = 1024;
/// La banda del segnale si allarga di tanto oltre il nero e il bianco: con la radio sintonizzata un po' fuori (prima che l'AFC corregga) il
/// segnale resta dentro.
const BAND_MARGIN_HZ: f64 = 100.0;
/// Fasce di rumore: da 200 a 450 Hz fuori dal nero e dal bianco.
const FLANK_NEAR_HZ: f64 = 200.0;
const FLANK_FAR_HZ: f64 = 450.0;
/// Decibel che portano il livello a 100.
const FULL_SCALE_DB: f64 = 20.0;

pub struct SignalMeter {
    window: Vec<f64>,
    buf: Vec<f64>,
    band: Vec<usize>,
    low: Vec<usize>,
    high: Vec<usize>,
    level: f32,
}

impl SignalMeter {
    pub fn new(center_hz: f32, shift_hz: f32) -> SignalMeter {
        let window = (0..METER_BLOCK)
            .map(|i| 0.5 - 0.5 * (TAU * i as f64 / METER_BLOCK as f64).cos())
            .collect();
        let mut m = SignalMeter {
            window,
            buf: Vec::with_capacity(METER_BLOCK),
            band: Vec::new(),
            low: Vec::new(),
            high: Vec::new(),
            level: 0.0,
        };
        m.set_band(center_hz, shift_hz);
        m
    }

    /// La banda del fax: da `center - shift/2` (nero) a `center + shift/2` (bianco).
    pub fn set_band(&mut self, center_hz: f32, shift_hz: f32) {
        let (c, s) = (f64::from(center_hz), f64::from(shift_hz).abs());
        let bins = |from: f64, to: f64| -> Vec<usize> {
            let k = |hz: f64| {
                (hz * METER_BLOCK as f64 / f64::from(RATE)).clamp(1.0, (METER_BLOCK / 2 - 1) as f64)
            };
            (k(from).ceil() as usize..=k(to).floor() as usize).collect()
        };
        self.band = bins(c - s / 2.0 - BAND_MARGIN_HZ, c + s / 2.0 + BAND_MARGIN_HZ);
        self.low = bins(c - s / 2.0 - FLANK_FAR_HZ, c - s / 2.0 - FLANK_NEAR_HZ);
        self.high = bins(c + s / 2.0 + FLANK_NEAR_HZ, c + s / 2.0 + FLANK_FAR_HZ);
    }

    /// Ultimo livello misurato (0..100).
    pub fn level(&self) -> f32 {
        self.level
    }

    /// Un campione a 12 kHz; ogni [`METER_BLOCK`] campioni restituisce il livello nuovo.
    pub fn push(&mut self, x: f32) -> Option<f32> {
        self.buf
            .push(if x.is_finite() { f64::from(x) } else { 0.0 });
        if self.buf.len() < METER_BLOCK {
            return None;
        }
        let energy: f64 = self.buf.iter().map(|v| v * v).sum();
        let level = if energy < 1e-9 {
            0.0
        } else {
            let mean = |ks: &[usize]| {
                if ks.is_empty() {
                    0.0
                } else {
                    ks.iter().map(|&k| self.power(k)).sum::<f64>() / ks.len() as f64
                }
            };
            let band = mean(&self.band);
            let noise = mean(&self.low).max(mean(&self.high)).max(1e-12);
            let db = 10.0 * (band / noise).max(1e-6).log10();
            (db * 100.0 / FULL_SCALE_DB).clamp(0.0, 100.0) as f32
        };
        self.buf.clear();
        // un po' di media contro i guizzi del rumore
        self.level = 0.6 * self.level + 0.4 * level;
        Some(self.level)
    }

    /// Potenza del canale `k` del blocco (Goertzel con finestra di Hann).
    fn power(&self, k: usize) -> f64 {
        let coeff = 2.0 * (TAU * k as f64 / METER_BLOCK as f64).cos();
        let (mut s1, mut s2) = (0.0, 0.0);
        for (x, w) in self.buf.iter().zip(&self.window) {
            let s0 = x * w + coeff * s1 - s2;
            s2 = s1;
            s1 = s0;
        }
        s1 * s1 + s2 * s2 - coeff * s1 * s2
    }
}

/// Correzione massima di partenza dell'AFC (Hz) rispetto alla sottoportante delle impostazioni (si cambia con `afc_range_hz`).
pub const AFC_MAX_HZ: f32 = 150.0;
/// Un tono fisso conta solo se dista dal nero o dal bianco meno di questa parte dell'escursione (4/10: a meta' tra nero e bianco, 5/10, non e'
/// piu' ne' l'uno ne' l'altro).
const TONE_MAX_SHIFT_FRACTION: f32 = 0.4;

/// Istogramma da -1000 a +1000 Hz attorno alla sottoportante, a passi di 10 Hz.
const HIST_BINS: usize = 200;
const HIST_STEP: f32 = 10.0;
const HIST_FROM: f32 = -1000.0;

pub struct Afc {
    hist: [u32; HIST_BINS],
    samples: u32,
}

impl Default for Afc {
    fn default() -> Self {
        Afc {
            hist: [0; HIST_BINS],
            samples: 0,
        }
    }
}

impl Afc {
    pub fn reset(&mut self) {
        *self = Afc::default();
    }

    /// Una frequenza istantanea (Hz rispetto alla sottoportante, NaN senza segnale). Ogni secondo di campioni restituisce, se i due picchi
    /// di nero e bianco sono chiari, l'errore di sintonia in Hz (positivo: il segnale sta piu' in alto della sottoportante). Con `single_tone`
    /// falso (dentro un'immagine) un solo picco non conta: puo' essere una zona uniforme della cartina e non un tono fisso.
    pub fn push(&mut self, rel_hz: f32, shift_hz: f32, single_tone: bool) -> Option<f32> {
        if rel_hz.is_finite() {
            let b = ((rel_hz - HIST_FROM) / HIST_STEP).floor();
            if (0.0..HIST_BINS as f32).contains(&b) {
                self.hist[b as usize] += 1;
            }
        }
        self.samples += 1;
        if self.samples < RATE {
            return None;
        }
        let err = self.estimate(shift_hz, single_tone);
        self.reset();
        err
    }

    fn estimate(&self, shift_hz: f32, single_tone: bool) -> Option<f32> {
        let h = &self.hist;
        let total: u64 = h.iter().map(|&v| u64::from(v)).sum();
        if total < u64::from(RATE / 4) {
            return None;
        }
        let mean = total as f32 / HIST_BINS as f32;
        let (p1, &top) = h.iter().enumerate().max_by_key(|&(_, v)| *v)?;
        if (top as f32) < 4.0 * mean {
            return None;
        }
        let f1 = self.centroid(p1);
        // l'altro picco a distanza pari all'escursione (entro il 10%), da una parte o dall'altra
        let near = (0.9 * shift_hz / HIST_STEP).floor() as usize;
        let far = (1.1 * shift_hz / HIST_STEP).ceil() as usize;
        let p2 = (0..HIST_BINS).filter(|&b| (near..=far).contains(&b.abs_diff(p1))).max_by_key(|&b| h[b]);
        if let Some(p2) = p2 {
            let second = h[p2] as f32;
            if second >= 3.0 * mean && second >= 0.02 * top as f32 {
                return Some((f1 + self.centroid(p2)) / 2.0);
            }
        }
        if !single_tone {
            return None;
        }
        // un tono fisso (le stazioni mandano il nero, a volte il bianco, prima dell'immagine): quasi tutti i campioni in un picco stretto,
        // che deve cadere sul nero o sul bianco; l'errore e' la distanza dal piu' vicino dei due
        let around: u64 = (p1.saturating_sub(3)..=(p1 + 3).min(HIST_BINS - 1)).map(|b| u64::from(h[b])).sum();
        if (around as f32) < 0.6 * total as f32 {
            return None;
        }
        let half = shift_hz / 2.0;
        let err = if (f1 + half).abs() < (f1 - half).abs() { f1 + half } else { f1 - half };
        (err.abs() <= TONE_MAX_SHIFT_FRACTION * shift_hz).then_some(err)
    }

    /// Frequenza media dei campioni entro tre caselle dal picco.
    fn centroid(&self, p: usize) -> f32 {
        let (mut w, mut s) = (0.0f32, 0.0f32);
        for b in p.saturating_sub(3)..=(p + 3).min(HIST_BINS - 1) {
            let v = self.hist[b] as f32;
            w += v;
            s += v * (HIST_FROM + (b as f32 + 0.5) * HIST_STEP);
        }
        if w > 0.0 {
            s / w
        } else {
            HIST_FROM + (p as f32 + 0.5) * HIST_STEP
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tone(hz: f64, n: usize, amp: f64) -> Vec<f32> {
        (0..n)
            .map(|i| (amp * (TAU * hz * i as f64 / f64::from(RATE)).sin()) as f32)
            .collect()
    }

    #[test]
    fn un_tono_nella_banda_alza_il_livello_il_silenzio_no() {
        let mut m = SignalMeter::new(1900.0, 800.0);
        let mut last = 0.0;
        for x in tone(2000.0, METER_BLOCK * 10, 0.3) {
            if let Some(l) = m.push(x) {
                last = l;
            }
        }
        assert!(last > 90.0, "livello {last}");
        let mut m = SignalMeter::new(1900.0, 800.0);
        for _ in 0..METER_BLOCK * 3 {
            m.push(0.0);
        }
        assert_eq!(m.level(), 0.0);
        // un tono fuori banda (nelle fasce del rumore) non e' segnale
        let mut m = SignalMeter::new(1900.0, 800.0);
        for x in tone(1150.0, METER_BLOCK * 10, 0.3) {
            m.push(x);
        }
        assert!(m.level() < 5.0, "livello {}", m.level());
    }
}
