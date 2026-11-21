//! Demodulatore FM della sottoportante WEFAX, ripreso da `wefax.cxx` di fldigi 4.2.13 (W1HKJ, Remi Chateauneu; a sua volta da HAMFAX di
//! Christof Schmitt): l'audio si mescola con una portante a `center_hz`, si filtra con uno dei tre FIR passa basso (Narrow, Medium,
//! Wide, coefficienti di ACfax) e la variazione di fase tra due campioni dice la frequenza istantanea: 1500 Hz = nero (0),
//! 2300 Hz = bianco (255) con la spaziatura standard di 800 Hz.

use std::f64::consts::TAU;

use serde::{Deserialize, Serialize};

/// Frequenza di campionamento del ricevitore (l'audio d'ingresso si ricampiona a questa).
pub const RATE: u32 = 12_000;

const TAPS: usize = 65;

/// Sotto questa ampiezza (dopo il filtro) due campioni di fila valgono bianco: niente segnale.
const CLIP: f64 = 0.001;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Filter {
    #[default]
    Narrow,
    Medium,
    Wide,
}

impl Filter {
    pub fn taps(self) -> &'static [f64; TAPS] {
        match self {
            Filter::Narrow => &NARROW,
            Filter::Medium => &MEDIUM,
            Filter::Wide => &WIDE,
        }
    }
}

#[rustfmt::skip]
const NARROW: [f64; TAPS] = [
    0.000495, 0.000684, 0.000885, 0.00109, 0.00128,
    0.00141, 0.00142, 0.00124, 0.000793, 2.94e-05,
    -0.00108, -0.00251, -0.0042, -0.00602, -0.00778,
    -0.00925, -0.0101, -0.0102, -0.00909, -0.00664,
    -0.00267, 0.00289, 0.01, 0.0185, 0.0281,
    0.0383, 0.0488, 0.059, 0.0682, 0.0761,
    0.0821, 0.0858, 0.0871, 0.0858, 0.0821,
    0.0761, 0.0682, 0.059, 0.0488, 0.0383,
    0.0281, 0.0185, 0.01, 0.00289, -0.00267,
    -0.00664, -0.00909, -0.0102, -0.0101, -0.00925,
    -0.00778, -0.00602, -0.0042, -0.00251, -0.00108,
    2.94e-05, 0.000793, 0.00124, 0.00142, 0.00141,
    0.00128, 0.00109, 0.000885, 0.000684, 0.000495,
];

#[rustfmt::skip]
const MEDIUM: [f64; TAPS] = [
    -0.000795, -0.000779, -0.000698, -0.000517, -0.000195,
    0.000303, 0.000982, 0.0018, 0.00267, 0.00343,
    0.00389, 0.00383, 0.00306, 0.00144, -0.00102,
    -0.0042, -0.0078, -0.0114, -0.0143, -0.0159,
    -0.0156, -0.0127, -0.00695, 0.00188, 0.0136,
    0.0277, 0.0434, 0.0596, 0.0752, 0.0889,
    0.0997, 0.106, 0.109, 0.106, 0.0997,
    0.0889, 0.0752, 0.0596, 0.0434, 0.0277,
    0.0136, 0.00188, -0.00695, -0.0127, -0.0156,
    -0.0159, -0.0143, -0.0114, -0.0078, -0.0042,
    -0.00102, 0.00144, 0.00306, 0.00383, 0.00389,
    0.00343, 0.00267, 0.0018, 0.000982, 0.000303,
    -0.000195, -0.000517, -0.000698, -0.000779, -0.000795,
];

#[rustfmt::skip]
const WIDE: [f64; TAPS] = [
    0.000716, 0.000844, 0.000845, 0.000668, 0.000259,
    -0.000402, -0.00126, -0.00216, -0.00284, -0.003,
    -0.00234, -0.00073, 0.00175, 0.0047, 0.00747,
    0.00922, 0.00911, 0.00655, 0.00143, -0.00575,
    -0.0138, -0.0209, -0.025, -0.0241, -0.0167,
    -0.00203, 0.0193, 0.0457, 0.0743, 0.102,
    0.125, 0.14, 0.145, 0.14, 0.125,
    0.102, 0.0743, 0.0457, 0.0193, -0.00203,
    -0.0167, -0.0241, -0.025, -0.0209, -0.0138,
    -0.00575, 0.00143, 0.00655, 0.00911, 0.00922,
    0.00747, 0.0047, 0.00175, -0.00073, -0.00234,
    -0.003, -0.00284, -0.00216, -0.00126, -0.000402,
    0.000259, 0.000668, 0.000845, 0.000844, 0.000716,
];

pub struct FmDemod {
    taps: &'static [f64; TAPS],
    /// Gli ultimi `TAPS` campioni mescolati (parte reale e immaginaria), scritti due volte come nel ricampionatore.
    i_hist: [f64; 2 * TAPS],
    q_hist: [f64; 2 * TAPS],
    pos: usize,
    phase: f64,
    phase_step: f64,
    prev: (f64, f64),
    deviation_ratio: f64,
}

impl FmDemod {
    /// `center_hz`: sottoportante (1900 Hz di norma); `shift_hz`: escursione nero-bianco (800 Hz di norma).
    pub fn new(filter: Filter, center_hz: f32, shift_hz: f32) -> FmDemod {
        let shift = f64::from(shift_hz).max(1.0);
        FmDemod {
            taps: filter.taps(),
            i_hist: [0.0; 2 * TAPS],
            q_hist: [0.0; 2 * TAPS],
            pos: 0,
            phase: 0.0,
            phase_step: TAU * f64::from(center_hz) / f64::from(RATE),
            prev: (0.0, 0.0),
            deviation_ratio: (f64::from(RATE) / shift) / TAU,
        }
    }

    /// Sposta la sottoportante senza toccare il filtro (serve all'AFC: nessun buco nel segnale demodulato).
    pub fn set_center(&mut self, center_hz: f32) {
        self.phase_step = TAU * f64::from(center_hz) / f64::from(RATE);
    }

    /// Un valore 0..255 (0 nero, 255 bianco) per ogni campione d'ingresso.
    pub fn process(&mut self, audio: &[f32], out: &mut Vec<u8>) {
        self.process_freq(audio, out, None);
    }

    /// Come [`FmDemod::process`] e in piu', se richiesto, la frequenza istantanea in Hz rispetto alla sottoportante, senza i limiti del
    /// nero e del bianco (NaN senza segnale): serve all'AFC.
    pub fn process_freq(&mut self, audio: &[f32], out: &mut Vec<u8>, mut freq: Option<&mut Vec<f32>>) {
        let hz_per_rad = f64::from(RATE) / TAU;
        for &a in audio {
            let a = if a.is_finite() { f64::from(a) } else { 0.0 };
            let (sin, cos) = self.phase.sin_cos();
            self.phase += self.phase_step;
            if self.phase >= TAU {
                self.phase -= TAU;
            }
            self.i_hist[self.pos] = a * cos;
            self.i_hist[self.pos + TAPS] = a * cos;
            self.q_hist[self.pos] = a * sin;
            self.q_hist[self.pos + TAPS] = a * sin;
            self.pos = (self.pos + 1) % TAPS;
            let (mut zi, mut zq) = (0.0, 0.0);
            for (k, t) in self.taps.iter().enumerate() {
                zi += t * self.i_hist[self.pos + k];
                zq += t * self.q_hist[self.pos + k];
            }
            let (pi, pq) = self.prev;
            let value = if zi.hypot(zq) <= CLIP && pi.hypot(pq) <= CLIP {
                if let Some(f) = freq.as_deref_mut() {
                    f.push(f32::NAN);
                }
                255
            } else {
                // arg(conj(prev) * curr)
                let re = pi * zi + pq * zq;
                let im = pi * zq - pq * zi;
                let angle = im.atan2(re);
                if let Some(f) = freq.as_deref_mut() {
                    f.push((-angle * hz_per_rad) as f32);
                }
                let v = (255.0 * (0.5 - self.deviation_ratio * angle)).round();
                v.clamp(0.0, 255.0) as u8
            };
            self.prev = (zi, zq);
            out.push(value);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tone(hz: f64, n: usize) -> Vec<f32> {
        (0..n).map(|i| (0.5 * (TAU * hz * i as f64 / f64::from(RATE)).sin()) as f32).collect()
    }

    #[test]
    fn nero_a_1500_bianco_a_2300_grigio_al_centro() {
        for (hz, want) in [(1500.0, 0i32), (1900.0, 128), (2300.0, 255)] {
            let mut d = FmDemod::new(Filter::Narrow, 1900.0, 800.0);
            let mut out = Vec::new();
            d.process(&tone(hz, 3000), &mut out);
            let v = i32::from(out[2500]);
            assert!((v - want).abs() <= 3, "{hz} Hz: {v} invece di {want}");
        }
    }

    #[test]
    fn il_silenzio_e_bianco_e_i_valori_non_finiti_non_rompono() {
        let mut d = FmDemod::new(Filter::Wide, 1900.0, 800.0);
        let mut out = Vec::new();
        let mut a = vec![0.0f32; 200];
        a[10] = f32::NAN;
        a[20] = f32::INFINITY;
        d.process(&a, &mut out);
        assert!(out.iter().all(|&v| v == 255));
    }
}
