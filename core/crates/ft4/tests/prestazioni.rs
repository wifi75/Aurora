//! Prestazioni del decoder FT4 (vedi `docs/prestazioni-motore.md`): periodi sintetici ripetibili, tempo per periodo e impronta dei risultati.
//! `cargo test -p ft4 --release --test prestazioni -- --ignored --nocapture`

use std::f64::consts::PI;
use std::time::Instant;

use ft4::decode::{Decoded, Options, decode_slot};
use ft4::{NSPS, SAMPLE_RATE, SLOT_SAMPLES, modem};
use ft8::message::CallHashes;

struct Rng(u64);
impl Rng {
    fn uniform(&mut self) -> f64 {
        self.0 = self.0.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        ((self.0 >> 11) as f64 + 0.5) / (1u64 << 53) as f64
    }
    fn gauss(&mut self) -> f64 {
        (-2.0 * self.uniform().ln()).sqrt() * (2.0 * PI * self.uniform()).cos()
    }
}

const TESTI: [&str; 10] =
    ["CQ IU3ABC JN65", "IU3ABC K1ABC FN42", "K1ABC IU3ABC -12", "IU3ABC K1ABC R-08", "K1ABC IU3ABC RR73", "CQ DL1XYZ JO31", "DL1XYZ F5ABC IN88", "F5ABC DL1XYZ +03", "CQ JA1ABC PM95", "W1AW EA3XYZ JN11"];

/// Un periodo con `n` segnali da -16 a +4 dB sparsi fra 300 e 2700 Hz.
fn periodo(k: u64, n: usize) -> Vec<f32> {
    let mut rng = Rng(77 + k);
    let sigma = 0.05f64;
    let mut audio: Vec<f32> = (0..SLOT_SAMPLES).map(|_| (sigma * rng.gauss()) as f32).collect();
    let mut hashes = CallHashes::default();
    for i in 0..n {
        let text = TESTI[(i + k as usize) % TESTI.len()];
        let freq = 300.0 + 2400.0 * (i as f32 + 0.5) / n as f32;
        let snr = -16.0 + 20.0 * rng.uniform();
        let Ok(wave) = modem::encode_message(text, freq, &mut hashes) else { continue };
        let noise_2500 = sigma * sigma * 2500.0 / (SAMPLE_RATE as f64 / 2.0);
        let amplitude = (2.0 * noise_2500 * 10f64.powf(snr / 10.0)).sqrt() as f32;
        let start = ((0.1 * rng.uniform() + 0.5) * SAMPLE_RATE as f64).round() as usize - NSPS;
        for (j, &w) in wave.iter().enumerate() {
            if let Some(a) = audio.get_mut(start + j) {
                *a += amplitude * w;
            }
        }
    }
    audio
}

fn impronta(out: &[Decoded]) -> u64 {
    let mut righe: Vec<String> = out.iter().map(|d| format!("{}|{:08x}|{:08x}|{}|{}", d.text, d.freq_hz.to_bits(), d.dt_s.to_bits(), d.snr_db, d.hint)).collect();
    righe.sort();
    let mut h = 0xcbf2_9ce4_8422_2325u64;
    for b in righe.join("\n").bytes() {
        h = (h ^ u64::from(b)).wrapping_mul(0x0100_0000_01b3);
    }
    h
}

#[test]
#[ignore]
fn prestazioni_periodo() {
    let periods: Vec<Vec<f32>> = (0..12).map(|k| periodo(k, 8)).collect();
    let options = Options::default();
    let _ = decode_slot(&periods[0], &options, &mut CallHashes::default());
    let (mut total, mut ms, mut worst, mut h) = (0usize, 0.0f64, 0.0f64, 0u64);
    for audio in &periods {
        let t0 = Instant::now();
        let out = decode_slot(audio, &options, &mut CallHashes::default());
        let t = t0.elapsed().as_secs_f64() * 1000.0;
        worst = worst.max(t);
        ms += t;
        total += out.len();
        h = h.rotate_left(5) ^ impronta(&out);
    }
    println!("FT4 (opzioni di base): messaggi {total:>4}  impronta {h:016x}  media {:>6.0} ms/periodo  peggiore {worst:>6.0} ms", ms / periods.len() as f64);
}
