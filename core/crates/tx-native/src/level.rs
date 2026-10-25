//! Livello e rampe: l'attenuazione si dà in decimi di dB come la «Pwr» di JTDX; all'inizio e alla fine dell'audio
//! una rampa a coseno rialzato evita i click.

use std::{f64::consts::PI, time::Duration};

use crate::{WAVE_RATE, error::TxError, resample::resample};

/// Attenuazione massima accettata (decimi di dB): 99,9 dB.
pub const MAX_ATTENUATION_TENTHS: u32 = 999;

/// Guadagno lineare per un'attenuazione in decimi di dB (0 = nessuna, 200 = 20 dB = 0,1).
pub fn attenuation_to_gain(tenths: u32) -> f32 {
    10f64.powf(-f64::from(tenths) / 200.0) as f32
}

/// Rampa di salita sui primi `n` campioni e di discesa sugli ultimi `n` (al massimo meta' dell'onda ciascuna):
/// il primo e l'ultimo campione diventano zero.
pub fn apply_fade(samples: &mut [f32], n: usize) {
    let n = n.min(samples.len() / 2);
    if n == 0 {
        return;
    }
    let last = samples.len() - 1;
    for i in 0..n {
        let g = (0.5 * (1.0 - (PI * i as f64 / n as f64).cos())) as f32;
        samples[i] *= g;
        samples[last - i] *= g;
    }
}

/// Dall'onda a 12 kHz ai campioni pronti per il dispositivo: ricampiona, applica l'attenuazione, limita a +-1 e mette le rampe.
pub fn prepare_samples(wave: &[f32], device_rate: u32, attenuation_tenths: u32, fade: Duration) -> Result<Vec<f32>, TxError> {
    if attenuation_tenths > MAX_ATTENUATION_TENTHS {
        return Err(TxError::InvalidConfig(format!("attenuazione oltre {MAX_ATTENUATION_TENTHS} decimi di dB")));
    }
    let mut out = resample(wave, WAVE_RATE, device_rate)?;
    let gain = attenuation_to_gain(attenuation_tenths);
    for s in &mut out {
        *s = (*s * gain).clamp(-1.0, 1.0);
    }
    let fade_samples = (fade.as_secs_f64() * f64::from(device_rate)).round() as usize;
    apply_fade(&mut out, fade_samples);
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn attenuation_is_in_tenths_of_a_decibel() {
        assert!((attenuation_to_gain(0) - 1.0).abs() < 1e-6);
        assert!((attenuation_to_gain(200) - 0.1).abs() < 1e-6);
        assert!((attenuation_to_gain(60) - 0.501_187).abs() < 1e-5);
        assert!((attenuation_to_gain(400) - 0.01).abs() < 1e-6);
    }

    #[test]
    fn the_fade_starts_and_ends_at_zero_and_leaves_the_middle() {
        let mut x = vec![1.0f32; 1000];
        apply_fade(&mut x, 100);
        assert_eq!(x[0], 0.0);
        assert_eq!(x[999], 0.0);
        assert!(x[..100].windows(2).all(|w| w[1] >= w[0]), "salita non monotona");
        assert!(x[900..].windows(2).all(|w| w[1] <= w[0]), "discesa non monotona");
        assert!(x[100..900].iter().all(|&v| v == 1.0));
        // nessun salto grande tra campioni vicini
        assert!(x.windows(2).all(|w| (w[1] - w[0]).abs() < 0.05));
    }

    #[test]
    fn a_short_wave_is_faded_without_overlap() {
        let mut x = vec![1.0f32; 10];
        apply_fade(&mut x, 100);
        assert_eq!((x[0], x[9]), (0.0, 0.0));
        let mut empty: Vec<f32> = Vec::new();
        apply_fade(&mut empty, 10);
    }

    #[test]
    fn prepared_samples_have_the_device_length_level_and_ramps() {
        let wave: Vec<f32> = (0..12_000).map(|n| (2.0 * PI * 1000.0 * f64::from(n) / 12_000.0).sin() as f32).collect();
        let out = prepare_samples(&wave, 48_000, 200, Duration::from_millis(5)).expect("preparazione");
        assert_eq!(out.len(), 48_000);
        assert_eq!((out[0], out[out.len() - 1]), (0.0, 0.0));
        let peak = out.iter().fold(0.0f32, |m, v| m.max(v.abs()));
        assert!((peak - 0.1).abs() < 0.01, "picco {peak}");
        assert!(prepare_samples(&wave, 48_000, 1000, Duration::ZERO).is_err());
    }
}
