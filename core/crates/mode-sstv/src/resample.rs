//! Dalla frequenza della scheda audio ai 12 kHz del decodificatore: filtro passa basso e interpolazione lineare, a flusso.

use std::f64::consts::{PI, TAU};

use crate::demod::RATE;

const TAPS: usize = 63;

pub struct Resampler {
    in_rate: u32,
    taps: Vec<f32>,
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
        Resampler { in_rate, taps: taps.into_iter().map(|t| t as f32).collect(), hist: vec![0.0; TAPS], pos: 0, filtered: Vec::new(), next: 0.0 }
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
            self.hist[self.pos] = x;
            self.pos = (self.pos + 1) % TAPS;
            let mut acc = 0.0;
            let mut k = self.pos;
            for t in &self.taps {
                acc += t * self.hist[k];
                k = (k + 1) % TAPS;
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
