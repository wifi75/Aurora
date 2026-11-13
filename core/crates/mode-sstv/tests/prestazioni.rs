//! Prestazioni della ricezione SSTV (vedi `docs/prestazioni-motore.md`): CPU per secondo di audio del decodificatore (12 kHz) su Robot36 e
//! Martin1 con rumore, e del ricampionamento dalla scheda (48 e 44,1 kHz -> 12 kHz, lo stesso dei modi testuali). Stampa le impronte di
//! immagini e campioni (devono restare uguali a ogni ottimizzazione).
//! `cargo test -p mode-sstv --release --test prestazioni -- --ignored --nocapture`

use std::time::Instant;

use mode_sstv::resample::Resampler;
use mode_sstv::{Decoder, DecoderConfig, DecoderEvent, RATE, RgbImage, SstvMode, encode};

fn fnv(h: &mut u64, bytes: impl IntoIterator<Item = u8>) {
    for b in bytes {
        *h = (*h ^ u64::from(b)).wrapping_mul(0x0100_0000_01b3);
    }
}

fn immagine(w: u32, h: u32) -> RgbImage {
    let mut data = Vec::with_capacity((w * h * 3) as usize);
    for y in 0..h {
        for x in 0..w {
            data.extend([(x * 255 / w) as u8, (y * 255 / h) as u8, ((x ^ y) & 0xff) as u8]);
        }
    }
    RgbImage { width: w, height: h, data }
}

struct Noise(u64);
impl Noise {
    fn next(&mut self) -> f32 {
        self.0 = self.0.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        ((self.0 >> 40) as f32 / (1u64 << 24) as f32 - 0.5) * 0.2
    }
}

#[test]
#[ignore]
fn prestazioni_sstv() {
    for (mode, w, h) in [(SstvMode::Robot36, 320, 240), (SstvMode::Martin1, 320, 256)] {
        let Ok(wave) = encode(mode, &immagine(w, h), RATE) else { panic!("codifica") };
        let mut noise = Noise(99);
        let mut audio = vec![0.0f32; RATE as usize];
        audio.extend(wave);
        audio.extend(vec![0.0f32; RATE as usize * 2]);
        for s in &mut audio {
            *s = 0.5 * *s + noise.next();
        }
        let secs = audio.len() as f64 / f64::from(RATE);
        let mut dec = Decoder::new(DecoderConfig::default());
        let mut hash = 0xcbf2_9ce4_8422_2325u64;
        let mut images = 0;
        let t0 = Instant::now();
        for block in audio.chunks(480) {
            for ev in dec.push(block) {
                if let DecoderEvent::Finished(img) = ev {
                    images += 1;
                    fnv(&mut hash, img.image.data.iter().copied());
                }
            }
        }
        if let Some(DecoderEvent::Finished(img)) = dec.finish(true) {
            images += 1;
            fnv(&mut hash, img.image.data.iter().copied());
        }
        let ms = t0.elapsed().as_secs_f64() * 1000.0;
        println!("SSTV {mode:?}: {:.2} ms per secondo di audio ({:.2}% di un core), immagini {images}, impronta {hash:016x}", ms / secs, ms / secs / 10.0);
    }
    for rate in [48_000u32, 44_100] {
        let mut noise = Noise(5);
        let input: Vec<f32> = (0..rate as usize * 60).map(|n| noise.next() + 0.3 * (n as f32 * 0.13).sin()).collect();
        let mut r = Resampler::new(rate);
        let mut out = Vec::new();
        let mut hash = 0xcbf2_9ce4_8422_2325u64;
        let t0 = Instant::now();
        for chunk in input.chunks(rate as usize / 100) {
            out.clear();
            r.process(chunk, &mut out);
            fnv(&mut hash, out.iter().flat_map(|v| v.to_bits().to_le_bytes()));
        }
        let ms = t0.elapsed().as_secs_f64() * 1000.0;
        println!("ricampionamento SSTV/testuali {rate} -> 12000 Hz: {:.3} ms per secondo di audio, impronta {hash:016x}", ms / 60.0);
    }
}
