//! Trasmissione WSPR: i 162 toni del messaggio (`genwspr.f90`, `get_wspr_channel_symbols` di `wsprsim_utils.c:741-751`) e l'onda a fase
//! continua (`Modulator` con `frames_per_symbol` 8192 e la frequenza del primo tono 1,5 toni sotto quella scelta,
//! `widgets/mainwindow.cpp:13586-13593`).

use super::fano::{encode, interleave};
use super::pack::encode_message;
use super::{NSPS, NSYM, SYNC, TONE_SPACING_HZ};

/// I toni da 0 a 3 del messaggio: `2 * bit + sincronismo`. `None` se il messaggio non e' valido.
pub fn tones(message: &str) -> Option<[u8; NSYM]> {
    let data = encode_message(message)?;
    Some(tones_of(&data))
}

/// I toni dei byte gia' compressi (11 byte, 50 bit di messaggio).
pub fn tones_of(data: &[u8; 11]) -> [u8; NSYM] {
    let mut bits = encode(data);
    interleave(&mut bits);
    std::array::from_fn(|i| 2 * bits[i] + SYNC[i])
}

/// L'onda a `sample_rate` (di solito 12 kHz): `f_center` e' il centro del segnale (fra il secondo e il terzo tono), come la frequenza di
/// trasmissione di WSJT-X. `drift_hz` sposta linearmente la frequenza fra l'inizio e la fine del messaggio (per le prove del ricevitore).
pub fn modulate(tones: &[u8; NSYM], f_center: f64, amplitude: f32, sample_rate: f64, drift_hz: f64) -> Vec<f32> {
    let symbol = NSPS as f64 * sample_rate / 12_000.0;
    let total = (NSYM as f64 * symbol).round() as usize;
    let f0 = f_center - 1.5 * TONE_SPACING_HZ;
    let mut out = Vec::with_capacity(total);
    let two_pi = 2.0 * std::f64::consts::PI;
    let mut phase = 0.0f64;
    for n in 0..total {
        let k = ((n as f64 / symbol) as usize).min(NSYM - 1);
        let f = f0 + f64::from(tones[k]) * TONE_SPACING_HZ + drift_hz * (n as f64 / total as f64 - 0.5);
        out.push(amplitude * phase.sin() as f32);
        phase += two_pi * f / sample_rate;
        if phase > two_pi {
            phase -= two_pi;
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tones_carry_the_sync_vector() {
        let t = tones("IU3ABC JN55 37").unwrap_or([0; NSYM]);
        assert!(t.iter().all(|&v| v < 4));
        for (v, s) in t.iter().zip(SYNC.iter()) {
            assert_eq!(v & 1, *s);
        }
        assert!(tones("NOT A MESSAGE").is_none());
    }

    #[test]
    fn the_wave_lasts_about_110_seconds() {
        let t = tones("K1ABC FN42 37").unwrap_or([0; NSYM]);
        let w = modulate(&t, 1500.0, 1.0, 12_000.0, 0.0);
        assert_eq!(w.len(), NSYM * NSPS);
        assert!((w.len() as f64 / 12_000.0 - 110.592).abs() < 1e-6);
    }
}
