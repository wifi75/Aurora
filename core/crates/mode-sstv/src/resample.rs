//! Dalla frequenza della scheda audio ai 12 kHz del decodificatore: filtro passa basso e interpolazione lineare, a flusso.

use std::f64::consts::{PI, TAU};

use crate::demod::RATE;

const TAPS: usize = 63;

pub struct Resampler {
    in_rate: u32,
    taps: Vec<f32>,
    /// I campioni in ingresso ancora utili: `TAPS - 1` di storico (all'inizio zeri) e poi i nuovi. Il campione filtrato `j` e' la somma dei
    /// coefficienti per `buf[j..j + TAPS]` (dal piu' vecchio), calcolata solo per i due campioni vicini a ogni uscita.
    buf: Vec<f32>,
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
        Resampler { in_rate, taps: taps.into_iter().map(|t| t as f32).collect(), buf: vec![0.0; TAPS - 1], next: 0.0 }
    }

    pub fn in_rate(&self) -> u32 {
        self.in_rate
    }

    pub fn process(&mut self, input: &[f32], out: &mut Vec<f32>) {
        if self.in_rate == RATE {
            out.extend_from_slice(input);
            return;
        }
        self.buf.extend_from_slice(input);
        // quanti campioni filtrati sono calcolabili: uno per ogni campione arrivato dopo lo storico
        let flen = self.buf.len() - (TAPS - 1);
        let step = f64::from(self.in_rate) / f64::from(RATE);
        while self.next + 1.0 < flen as f64 {
            let i = self.next.floor() as usize;
            let frac = (self.next - i as f64) as f32;
            // il FIR solo dove serve (due punti per uscita, non uno per ogni campione in ingresso): stessi valori e stessi bit di prima
            out.push(self.fir(i) * (1.0 - frac) + self.fir(i + 1) * frac);
            self.next += step;
        }
        let drop = (self.next.floor() as usize).min(flen);
        self.buf.drain(..drop);
        self.next -= drop as f64;
    }

    /// Il campione filtrato `j`: somme nello stesso ordine di sempre (dal campione piu' vecchio).
    fn fir(&self, j: usize) -> f32 {
        let mut acc = 0.0;
        for (t, h) in self.taps.iter().zip(&self.buf[j..j + TAPS]) {
            acc += t * h;
        }
        acc
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Il filtro com'era prima: un FIR per ogni campione in ingresso, poi l'interpolazione. Deve dare gli stessi bit.
    fn reference(in_rate: u32, input: &[f32], chunk: usize) -> Vec<f32> {
        let r = Resampler::new(in_rate);
        let mut hist = vec![0.0f32; TAPS];
        let (mut filtered, mut next, mut out) = (Vec::<f32>::new(), 0.0f64, Vec::new());
        let step = f64::from(in_rate) / f64::from(RATE);
        for block in input.chunks(chunk) {
            for &x in block {
                hist.remove(0);
                hist.push(x);
                let mut acc = 0.0;
                for (t, h) in r.taps.iter().zip(&hist) {
                    acc += t * h;
                }
                filtered.push(acc);
            }
            while next + 1.0 < filtered.len() as f64 {
                let i = next.floor() as usize;
                let frac = (next - i as f64) as f32;
                out.push(filtered[i] * (1.0 - frac) + filtered[i + 1] * frac);
                next += step;
            }
            let drop = (next.floor() as usize).min(filtered.len());
            filtered.drain(..drop);
            next -= drop as f64;
        }
        out
    }

    #[test]
    fn the_decimated_filter_gives_the_same_bits_as_the_full_one() {
        for in_rate in [48_000u32, 44_100, 96_000, 24_000] {
            let input: Vec<f32> = (0..in_rate as usize / 2).map(|n| ((n as f32 * 0.37).sin() * 0.4) + ((n * 7919) % 101) as f32 / 1000.0).collect();
            let mut r = Resampler::new(in_rate);
            let mut out = Vec::new();
            for chunk in input.chunks(441) {
                r.process(chunk, &mut out);
            }
            let want = reference(in_rate, &input, 441);
            let n = out.len().min(want.len());
            assert!(n > 3000, "{in_rate}: {n}");
            assert!(out[..n].iter().zip(&want[..n]).all(|(a, b)| a.to_bits() == b.to_bits()), "{in_rate}: campioni diversi");
        }
    }
}
