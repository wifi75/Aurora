//! Porta l'audio della scheda (44,1, 48, 96 kHz...) ai 12 kHz del decoder, con un filtro sinc a finestra (passa-basso a ~5,4 kHz).

use std::f64::consts::PI;

pub struct Resampler {
    /// Campioni in ingresso per ogni campione in uscita.
    ratio: f64,
    /// Meta' larghezza del filtro, in campioni di ingresso.
    half: usize,
    /// Frequenza di taglio in cicli per campione di ingresso.
    cutoff: f64,
    /// Indice assoluto (nel flusso di ingresso) del primo campione di `buf`.
    base: u64,
    /// Posizione assoluta del prossimo campione da produrre.
    next: f64,
    buf: Vec<f32>,
}

impl Resampler {
    pub fn new(in_rate: u32, out_rate: u32) -> Resampler {
        let ratio = f64::from(in_rate.max(1)) / f64::from(out_rate.max(1));
        let cutoff = 0.5 * (1.0 / ratio.max(1.0)) * 0.9;
        let half = (16.0 * ratio.max(1.0)).ceil() as usize;
        Resampler { ratio, half, cutoff, base: 0, next: 0.0, buf: Vec::new() }
    }

    /// Aggiunge campioni in ingresso e restituisce quelli in uscita ormai calcolabili.
    pub fn push(&mut self, input: &[f32]) -> Vec<f32> {
        if (self.ratio - 1.0).abs() < f64::EPSILON {
            return input.to_vec();
        }
        self.buf.extend_from_slice(input);
        let mut out = Vec::with_capacity((input.len() as f64 / self.ratio) as usize + 2);
        let end = self.base + self.buf.len() as u64;
        let half = self.half as f64;
        while self.next + half < end as f64 {
            let centre = self.next;
            let first = (centre.floor() as i64 - self.half as i64 + 1).max(self.base as i64) as u64;
            let last = (centre.floor() as u64 + self.half as u64).min(end - 1);
            let mut acc = 0.0f64;
            for i in first..=last {
                let t = i as f64 - centre;
                let window = 0.5 * (1.0 + (PI * t / half).cos());
                let x = 2.0 * PI * self.cutoff * t;
                let sinc = if x.abs() < 1e-12 { 1.0 } else { x.sin() / x };
                acc += f64::from(self.buf[(i - self.base) as usize]) * 2.0 * self.cutoff * sinc * window;
            }
            out.push(acc as f32);
            self.next += self.ratio;
        }
        let keep_from = (self.next.floor() as i64 - self.half as i64).max(self.base as i64) as u64;
        let drop = (keep_from - self.base) as usize;
        if drop > 0 {
            self.buf.drain(..drop);
            self.base = keep_from;
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tone(rate: u32, freq: f64, seconds: f64) -> Vec<f32> {
        (0..(f64::from(rate) * seconds) as usize).map(|n| (2.0 * PI * freq * n as f64 / f64::from(rate)).sin() as f32).collect()
    }

    fn rms(x: &[f32]) -> f64 {
        (x.iter().map(|&v| f64::from(v) * f64::from(v)).sum::<f64>() / x.len() as f64).sqrt()
    }

    #[test]
    fn same_rate_passes_through() {
        let mut r = Resampler::new(12_000, 12_000);
        assert_eq!(r.push(&[0.5, -0.5, 0.25]), vec![0.5, -0.5, 0.25]);
    }

    #[test]
    fn a_tone_keeps_its_level_and_the_rate_is_right() {
        for in_rate in [48_000u32, 44_100, 96_000] {
            let mut r = Resampler::new(in_rate, 12_000);
            let input = tone(in_rate, 1500.0, 2.0);
            // a blocchi irregolari, come arrivano dalla scheda
            let mut out = Vec::new();
            for chunk in input.chunks(441) {
                out.extend(r.push(chunk));
            }
            let expected = 2 * 12_000;
            assert!(out.len().abs_diff(expected) < 40, "{in_rate}: {} campioni", out.len());
            let body = &out[2000..out.len() - 2000];
            assert!((rms(body) - std::f64::consts::FRAC_1_SQRT_2).abs() < 0.02, "{in_rate}: rms {}", rms(body));
        }
    }

    #[test]
    fn a_tone_above_the_new_nyquist_is_removed() {
        let mut r = Resampler::new(48_000, 12_000);
        let out = r.push(&tone(48_000, 9_000.0, 1.0));
        let body = &out[500..];
        assert!(rms(body) < 0.02, "rms {}", rms(body));
    }
}
