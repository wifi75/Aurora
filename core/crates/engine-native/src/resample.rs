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
    /// Sinc e finestra per i ritardi interi `t = -half+1 ..= half` (indice `t + half - 1`): con un rapporto intero (48 e 96 kHz) il centro cade
    /// sempre su un campione e i coefficienti sono sempre questi. Calcolati con le stesse operazioni del caso generale, danno gli stessi bit.
    table: Vec<(f64, f64)>,
}

impl Resampler {
    pub fn new(in_rate: u32, out_rate: u32) -> Resampler {
        let ratio = f64::from(in_rate.max(1)) / f64::from(out_rate.max(1));
        let cutoff = 0.5 * (1.0 / ratio.max(1.0)) * 0.9;
        let half = (16.0 * ratio.max(1.0)).ceil() as usize;
        let table = (0..2 * half).map(|k| taps(k as f64 - half as f64 + 1.0, half as f64, cutoff)).collect();
        Resampler { ratio, half, cutoff, base: 0, next: 0.0, buf: Vec::new(), table }
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
            if centre.fract() == 0.0 {
                // centro su un campione: i coefficienti sono quelli della tabella (`t` va da `first - centre` a `last - centre`)
                let from = (first as i64 - centre as i64 + self.half as i64 - 1) as usize;
                let samples = &self.buf[(first - self.base) as usize..=(last - self.base) as usize];
                for (&x, &(sinc, window)) in samples.iter().zip(&self.table[from..]) {
                    acc += f64::from(x) * 2.0 * self.cutoff * sinc * window;
                }
            } else {
                for i in first..=last {
                    let (sinc, window) = taps(i as f64 - centre, half, self.cutoff);
                    acc += f64::from(self.buf[(i - self.base) as usize]) * 2.0 * self.cutoff * sinc * window;
                }
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

/// Sinc e finestra di Hann del filtro per un campione a distanza `t` (in campioni d'ingresso) dal centro.
fn taps(t: f64, half: f64, cutoff: f64) -> (f64, f64) {
    let window = 0.5 * (1.0 + (PI * t / half).cos());
    let x = 2.0 * PI * cutoff * t;
    let sinc = if x.abs() < 1e-12 { 1.0 } else { x.sin() / x };
    (sinc, window)
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
        // anche le frequenze che un server TCI puo' chiedere con `audio_samplerate` (24, 16, 8 kHz)
        for in_rate in [48_000u32, 44_100, 96_000, 24_000, 16_000, 8_000] {
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

    /// Il filtro calcolato campione per campione, com'era prima della tabella: i campioni in uscita devono avere gli stessi bit.
    fn reference(in_rate: u32, input: &[f32]) -> Vec<f32> {
        let r = Resampler::new(in_rate, 12_000);
        let (half, mut next, mut out) = (r.half as f64, 0.0f64, Vec::new());
        while next + half < input.len() as f64 {
            let first = (next.floor() as i64 - r.half as i64 + 1).max(0) as usize;
            let last = (next.floor() as usize + r.half).min(input.len() - 1);
            let mut acc = 0.0f64;
            for (i, &x) in input.iter().enumerate().take(last + 1).skip(first) {
                let t = i as f64 - next;
                let window = 0.5 * (1.0 + (PI * t / half).cos());
                let x2 = 2.0 * PI * r.cutoff * t;
                let sinc = if x2.abs() < 1e-12 { 1.0 } else { x2.sin() / x2 };
                acc += f64::from(x) * 2.0 * r.cutoff * sinc * window;
            }
            out.push(acc as f32);
            next += r.ratio;
        }
        out
    }

    #[test]
    fn the_table_gives_the_same_bits_as_the_formula() {
        for in_rate in [48_000u32, 96_000, 44_100, 24_000] {
            let input: Vec<f32> = tone(in_rate, 1234.5, 0.5).iter().enumerate().map(|(n, &v)| v * 0.5 + ((n * 7919) % 101) as f32 / 1000.0).collect();
            let mut r = Resampler::new(in_rate, 12_000);
            let mut out = Vec::new();
            for chunk in input.chunks(441) {
                out.extend(r.push(chunk));
            }
            let want = reference(in_rate, &input);
            let n = out.len().min(want.len());
            assert!(n > 5000, "{in_rate}: {n}");
            assert!(out[..n].iter().zip(&want[..n]).all(|(a, b)| a.to_bits() == b.to_bits()), "{in_rate}: campioni diversi");
        }
    }

    #[test]
    fn a_tone_above_the_new_nyquist_is_removed() {
        let mut r = Resampler::new(48_000, 12_000);
        let out = r.push(&tone(48_000, 9_000.0, 1.0));
        let body = &out[500..];
        assert!(rms(body) < 0.02, "rms {}", rms(body));
    }

    /// Prestazioni (vedi `docs/prestazioni-motore.md`): 60 s di audio della scheda (rumore e due toni) portati a 12 kHz in blocchi di 10 ms.
    /// Stampa il tempo per secondo di audio e l'impronta dei campioni in uscita (deve restare uguale a ogni ottimizzazione).
    /// `cargo test -p engine-native --release --lib resample::tests::prestazioni -- --ignored --nocapture`
    #[test]
    #[ignore]
    fn prestazioni() {
        for in_rate in [48_000u32, 44_100, 96_000] {
            let mut seed = 0x1234_5678_u64;
            let input: Vec<f32> = (0..in_rate as usize * 60)
                .map(|n| {
                    seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
                    let noise = ((seed >> 40) as f32 / (1u64 << 24) as f32 - 0.5) * 0.1;
                    let t = n as f64 / f64::from(in_rate);
                    noise + (0.3 * (2.0 * PI * 1000.0 * t).sin() + 0.2 * (2.0 * PI * 2345.6 * t).sin()) as f32
                })
                .collect();
            let mut r = Resampler::new(in_rate, 12_000);
            let t0 = std::time::Instant::now();
            let mut h = 0xcbf2_9ce4_8422_2325u64;
            let mut n = 0usize;
            for chunk in input.chunks(in_rate as usize / 100) {
                for v in r.push(chunk) {
                    h = (h ^ u64::from(v.to_bits())).wrapping_mul(0x0100_0000_01b3);
                    n += 1;
                }
            }
            let ms = t0.elapsed().as_secs_f64() * 1000.0;
            println!("ricampionamento {in_rate} -> 12000 Hz: {:.2} ms per secondo di audio ({:.2}% di un core), {n} campioni, impronta {h:016x}", ms / 60.0, ms / 600.0);
        }
    }
}
