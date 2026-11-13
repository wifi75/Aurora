//! Prestazioni della catena della cascata (vedi `docs/prestazioni-motore.md`): CPU per secondo di audio a 48 kHz, con e senza anti-click,
//! e impronta delle righe (deve restare uguale a ogni ottimizzazione).
//! `cargo test -p audio-spectrum --release --test prestazioni -- --ignored --nocapture`

use std::f64::consts::PI;
use std::time::Instant;

use audio_spectrum::{LevelMeter, SpectrumAnalyzer};

fn audio(rate: u32, secs: usize) -> Vec<f32> {
    let mut seed = 0x9e37_79b9_u64;
    (0..rate as usize * secs)
        .map(|n| {
            seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
            let noise = ((seed >> 40) as f32 / (1u64 << 24) as f32 - 0.5) * 0.05;
            let t = n as f64 / f64::from(rate);
            // un tono fisso e un CW a 25 WPM (punto 48 ms) che fa click
            let key = ((t / 0.048) as u64).is_multiple_of(2);
            noise + (0.2 * (2.0 * PI * 1500.0 * t).sin() + if key { 0.3 * (2.0 * PI * 700.0 * t).sin() } else { 0.0 }) as f32
        })
        .collect()
}

#[test]
#[ignore]
fn prestazioni_cascata() {
    const SECS: usize = 30;
    for rate in [48_000u32, 44_100, 96_000] {
        let input = audio(rate, SECS);
        for anticlick in [false, true] {
            let mut an = match SpectrumAnalyzer::new(rate) {
                Ok(a) => a,
                Err(e) => panic!("{e}"),
            };
            an.set_anticlick(Some(anticlick));
            let t0 = Instant::now();
            let (mut rows, mut h) = (0usize, 0xcbf2_9ce4_8422_2325u64);
            for chunk in input.chunks(rate as usize / 100) {
                for row in an.push(chunk) {
                    rows += 1;
                    for b in row {
                        h = (h ^ u64::from(b)).wrapping_mul(0x0100_0000_01b3);
                    }
                }
            }
            let ms = t0.elapsed().as_secs_f64() * 1000.0;
            println!(
                "cascata {rate} Hz anti-click {}: {:.2} ms per secondo di audio ({:.2}% di un core), {rows} righe, impronta {h:016x}",
                if anticlick { "si'" } else { "no" },
                ms / SECS as f64,
                ms / SECS as f64 / 10.0
            );
        }
        let mut meter = LevelMeter::new(rate);
        let t0 = Instant::now();
        let mut levels = 0usize;
        for chunk in input.chunks(rate as usize / 100) {
            levels += meter.push(chunk).len();
        }
        let ms = t0.elapsed().as_secs_f64() * 1000.0;
        println!("livello {rate} Hz: {:.3} ms per secondo di audio, {levels} eventi in {SECS} s", ms / SECS as f64);
    }
}
