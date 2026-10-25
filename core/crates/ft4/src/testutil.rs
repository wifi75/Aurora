//! Strumenti comuni alle prove: generatore pseudocasuale deterministico e periodi sintetici.

use std::f64::consts::PI;

use ft8::message::CallHashes;

use crate::{NSPS, SAMPLE_RATE, SLOT_SAMPLES, modem};

/// Generatore pseudocasuale deterministico (nessuna dipendenza): uniforme in (0,1) e gaussiano (Box-Muller).
pub struct Rng(pub u64);

impl Rng {
    pub fn uniform(&mut self) -> f64 {
        self.0 = self.0.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        ((self.0 >> 11) as f64 + 0.5) / (1u64 << 53) as f64
    }

    pub fn gauss(&mut self) -> f64 {
        (-2.0 * self.uniform().ln()).sqrt() * (2.0 * PI * self.uniform()).cos()
    }
}

/// Un periodo di 7,5 s con rumore bianco (deviazione standard `sigma`, audio normalizzato) e i messaggi dati: testo, frequenza del tono 0,
/// `dt` (scarto rispetto a 0,5 s) e SNR in 2500 Hz.
pub fn slot(signals: &[(&str, f32, f32, f32)], seed: u64, sigma: f64) -> Vec<f32> {
    let mut rng = Rng(seed);
    let mut audio: Vec<f32> = (0..SLOT_SAMPLES).map(|_| (sigma * rng.gauss()) as f32).collect();
    let mut hashes = CallHashes::default();
    for &(text, freq, dt, snr) in signals {
        let wave = modem::encode_message(text, freq, &mut hashes).expect("messaggio valido");
        // potenza del rumore in 2500 Hz = sigma^2 * 2500 / 6000; l'onda ha picco a, potenza a^2 / 2
        let noise_2500 = sigma * sigma * 2500.0 / (SAMPLE_RATE as f64 / 2.0);
        let amplitude = (2.0 * noise_2500 * 10f64.powf(f64::from(snr) / 10.0)).sqrt() as f32;
        // il primo simbolo di Costas comincia a (0,5 + dt) s; l'onda parte un simbolo di rampa prima
        let start = ((f64::from(dt) + 0.5) * SAMPLE_RATE as f64).round() as i64 - NSPS as i64;
        for (i, &w) in wave.iter().enumerate() {
            let at = start + i as i64;
            if at >= 0
                && let Some(a) = audio.get_mut(at as usize)
            {
                *a += amplitude * w;
            }
        }
    }
    audio
}
