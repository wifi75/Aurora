//! Dalla frequenza della scheda audio ai 12 kHz del ricevitore: filtro passa basso e interpolazione lineare, a flusso
//! (stessa tecnica del ricampionatore dell'SSTV di Aurora).

use std::f64::consts::{PI, TAU};

use crate::demod::RATE;

const TAPS: usize = 63;

pub struct Resampler {
    in_rate: u32,
    taps: Vec<f32>,
    /// Gli ultimi `TAPS` campioni, due volte di fila (vedi `process`).
    hist: Vec<f32>,
    pos: usize,
    filtered: Vec<f32>,
    /// Posizione (in campioni filtrati) del prossimo campione da produrre.
    next: f64,
}

impl Resampler {
    pub fn new(in_rate: u32) -> Resampler {
        let in_rate = in_rate.max(1);
        let fc = (0.42 * f64::from(RATE) / f64::from(in_rate)).min(0.5);
        let mid = (TAPS - 1) as f64 / 2.0;
        let mut taps: Vec<f64> = (0..TAPS)
            .map(|i| {
                let x = i as f64 - mid;
                let sinc = if x == 0.0 { 2.0 * fc } else { (TAU * fc * x).sin() / (PI * x) };
                sinc * (0.42 - 0.5 * (TAU * i as f64 / (TAPS - 1) as f64).cos() + 0.08 * (2.0 * TAU * i as f64 / (TAPS - 1) as f64).cos())
            })
            .collect();
        let sum: f64 = taps.iter().sum();
        for t in &mut taps {
            *t /= sum;
        }
        Resampler { in_rate, taps: taps.into_iter().map(|t| t as f32).collect(), hist: vec![0.0; 2 * TAPS], pos: 0, filtered: Vec::new(), next: 0.0 }
    }

    pub fn in_rate(&self) -> u32 {
        self.in_rate
    }

    pub fn process(&mut self, input: &[f32], out: &mut Vec<f32>) {
        if self.in_rate == RATE {
            out.extend_from_slice(input);
            return;
        }
        for &x in input {
            // lo storico e' scritto due volte (a `pos` e a `pos + TAPS`): gli ultimi TAPS campioni sono sempre `hist[pos..pos + TAPS]`
            self.hist[self.pos] = x;
            self.hist[self.pos + TAPS] = x;
            self.pos = (self.pos + 1) % TAPS;
            let mut acc = 0.0;
            for (t, h) in self.taps.iter().zip(&self.hist[self.pos..self.pos + TAPS]) {
                acc += t * h;
            }
            self.filtered.push(acc);
        }
        let step = f64::from(self.in_rate) / f64::from(RATE);
        while self.next + 1.0 < self.filtered.len() as f64 {
            let i = self.next.floor() as usize;
            let frac = (self.next - i as f64) as f32;
            out.push(self.filtered[i] * (1.0 - frac) + self.filtered[i + 1] * frac);
            self.next += step;
        }
        let drop = (self.next.floor() as usize).min(self.filtered.len());
        self.filtered.drain(..drop);
        self.next -= drop as f64;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Per ogni frequenza di scheda audio comune i campioni prodotti sono `RATE / in_rate` di quelli entrati e un tono di 1900 Hz resta a 1900 Hz:
    /// un errore di rapporto sposterebbe l'orologio della riga e inclinerebbe la cartina.
    #[test]
    fn the_resampler_keeps_the_time_base_and_the_tone_for_common_sound_card_rates() {
        for in_rate in [8_000u32, 11_025, 16_000, 22_050, 24_000, 44_100, 48_000, 96_000] {
            let secs = 30.0;
            let n = (f64::from(in_rate) * secs) as usize;
            let input: Vec<f32> = (0..n).map(|i| (TAU * 1900.0 * i as f64 / f64::from(in_rate)).sin() as f32).collect();
            let mut r = Resampler::new(in_rate);
            let mut out = Vec::new();
            for chunk in input.chunks(1024) {
                r.process(chunk, &mut out);
            }
            let expected = secs * f64::from(RATE);
            let ratio = out.len() as f64 / expected;
            assert!((ratio - 1.0).abs() < 2e-4, "{in_rate} Hz: {} campioni invece di {expected} (rapporto {ratio})", out.len());
            // frequenza dal numero di passaggi per lo zero (saltando il transitorio iniziale)
            let tail = &out[2_000..];
            let crossings = tail.windows(2).filter(|w| w[0] < 0.0 && w[1] >= 0.0).count() as f64;
            let hz = crossings / (tail.len() as f64 / f64::from(RATE));
            assert!((hz - 1900.0).abs() < 1.0, "{in_rate} Hz: tono a {hz} Hz");
        }
    }
}
