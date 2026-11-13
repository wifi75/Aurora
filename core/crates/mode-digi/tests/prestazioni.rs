//! Prestazioni dei ricevitori testuali (vedi `docs/prestazioni-motore.md`): CPU per secondo di audio a 12 kHz di PSK31, PSK63, RTTY e CW con
//! un segnale a 0 dB nel rumore, e impronta del testo uscito (deve restare uguale a ogni ottimizzazione). Il ricampionamento dalla scheda
//! (48 kHz) lo misura `mode-sstv` (stesso `Resampler`). Tutta la banda: `psk_confronto::tutti_i_segnali` e `cw_confronto::cpu_tutta_la_banda`.
//! `cargo test -p mode-digi --release --test prestazioni -- --ignored --nocapture`

mod common;

use std::time::Instant;

use common::{Rng, aggiungi_rumore, sigma_per_snr, testo};
use mode_digi::{CwDemodulator, PskDemodulator, PskMode, RttyDemodulator, RttyParams, cw, psk, rtty};

const AMP: f64 = 0.3;

fn impronta(text: &[u8]) -> u64 {
    let mut h = 0xcbf2_9ce4_8422_2325u64;
    for &b in text {
        h = (h ^ u64::from(b)).wrapping_mul(0x0100_0000_01b3);
    }
    h
}

fn misura(nome: &str, mut audio: Vec<f32>, mut rx: impl FnMut(&[f32]) -> Vec<u8>) {
    let mut rng = Rng(4242);
    aggiungi_rumore(&mut audio, sigma_per_snr(AMP, 0.0), &mut rng);
    let secs = audio.len() as f64 / 12_000.0;
    let mut out = Vec::new();
    let t0 = Instant::now();
    for block in audio.chunks(480) {
        out.extend(rx(block));
    }
    let ms = t0.elapsed().as_secs_f64() * 1000.0;
    println!("{nome:<8}: {:.3} ms per secondo di audio ({:.2}% di un core), {} caratteri, impronta {:016x}", ms / secs, ms / secs / 10.0, out.len(), impronta(&out));
}

#[test]
#[ignore]
fn prestazioni_ricevitori() {
    let mut rng = Rng(7);
    let text = testo(&mut rng, 400);
    {
        let wave = psk::modulate(text.as_bytes(), 1000.0, PskMode::Psk31, AMP as f32);
        let mut d = PskDemodulator::new(PskMode::Psk31, 1000.0);
        misura("PSK31", wave, |b| d.process(b));
    }
    {
        let wave = psk::modulate(text.as_bytes(), 1000.0, PskMode::Psk63, AMP as f32);
        let mut d = PskDemodulator::new(PskMode::Psk63, 1000.0);
        misura("PSK63", wave, |b| d.process(b));
    }
    {
        let params = RttyParams::default();
        let wave = rtty::modulate(text.to_uppercase().as_bytes(), params, AMP as f32);
        let mut d = RttyDemodulator::new(params);
        misura("RTTY", wave, |b| d.process(b));
    }
    {
        let wave = cw::modulate(text.to_uppercase().as_bytes(), 700.0, 22.0, AMP as f32);
        let mut d = CwDemodulator::new(700.0, 22.0);
        misura("CW", wave, |b| d.process(b));
    }
}
