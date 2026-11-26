//! MT63 di Aurora contro quello di fldigi 4.2.13 (`oracle/fldigi_mt63.cpp`, che compila `mt63base.cxx` e `dsp.cxx` di fldigi cosi' come
//! sono): il nostro trasmettitore da' la stessa onda di quello di fldigi, fldigi riceve il nostro (anche in LSB, dopo il rovesciamento
//! della radio) e noi riceviamo il suo. Si compila solo con `--features fldigi-oracle` (vedi `build.rs`).
#![cfg(feature = "fldigi-oracle")]

mod common;

use mode_digi::fldigi_oracle::FldigiMt63;
use mode_digi::mt63::{Mt63Tx, Mt63Variant};

const TESTO: &str = "CQ CQ DE IU3ABC IU3ABC IO91WM pse k 599 73 <QSL?> {test} ~";

fn con_rumore(mut audio: Vec<f32>, sigma: f64, seed: u64) -> Vec<f32> {
    let mut a = vec![0.0_f32; 4000];
    a.append(&mut audio);
    a.extend(vec![0.0_f32; 56_000]);
    let mut rng = common::Rng(seed);
    common::aggiungi_rumore(&mut a, sigma, &mut rng);
    a
}

/// Le sei varianti a due frequenze: la nostra onda e' quella di fldigi campione per campione (stessi numeri a caso).
#[test]
fn la_nostra_onda_e_quella_di_fldigi() {
    for v in Mt63Variant::ALL {
        for f in [1500.0, 1000.0 + v.bandwidth_hz() / 4.0] {
            let theirs = FldigiMt63::transmit(v, f, "IU3ABC de IU3ABC 5NN k");
            let mut tx = Mt63Tx::with_fill(v, f, 1.0, false, &theirs.fill);
            tx.set_jam_bits(theirs.jam.clone());
            let ours = tx.transmit_like_fldigi(b"IU3ABC de IU3ABC 5NN k");
            assert_eq!(ours.len(), theirs.wave.len(), "{v:?} {f}: lunghezze");
            let peak = theirs.wave.iter().fold(0.0_f32, |m, x| m.max(x.abs()));
            let max = ours.iter().zip(&theirs.wave).map(|(a, b)| (a - b).abs()).fold(0.0_f32, f32::max);
            assert!(peak > 0.1 && max < 1e-5 * peak.max(1.0), "{v:?} {f}: scarto massimo {max}, picco {peak}");
        }
    }
}

/// Rovescia lo spettro come una radio in LSB vista da un ricevitore in USB: a 8 kHz la moltiplicazione per (-1)^n porta f in 4000 - f.
fn rovescia(a: &[f32]) -> Vec<f32> {
    a.iter().enumerate().map(|(i, &x)| if i % 2 == 0 { x } else { -x }).collect()
}

/// fldigi riceve il nostro in tutte le varianti, con rumore; e in LSB dopo il rovesciamento della radio.
#[test]
fn fldigi_riceve_il_nostro() {
    for v in Mt63Variant::ALL {
        for lsb in [false, true] {
            let f = 1500.0;
            let a = con_rumore(Mt63Tx::new(v, f, 0.5, lsb).transmit(TESTO.as_bytes()), 0.05, 7);
            let (a, f_rx) = if lsb { (rovescia(&a), 4000.0 - f) } else { (a, f) };
            let mut fl = FldigiMt63::new(v, f_rx, 16);
            let got: Vec<u8> = a.chunks(512).flat_map(|b| fl.process(b)).collect();
            let got = String::from_utf8_lossy(&got).into_owned();
            assert!(got.contains(TESTO), "{v:?} lsb {lsb}: fldigi ha ricevuto {got:?}");
        }
    }
}
