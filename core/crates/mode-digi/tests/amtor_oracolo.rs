//! Amtor FEC (SITOR-B) di Aurora contro il modo SITORB di fldigi 4.2.13 (`oracle/fldigi_sitorb.cpp`): fldigi riceve il nostro
//! trasmettitore e noi riceviamo il suo. Si compila solo con `--features fldigi-oracle` (vedi `build.rs`).
#![cfg(feature = "fldigi-oracle")]

mod common;

use mode_digi::amtor::{AmtorRx, AmtorTx};
use mode_digi::fldigi_oracle::FldigiSitorb;

const TESTO: &str = "CQ CQ DE IU3ABC IU3ABC IO91WM 599 73 (K)? 1234567890 -/.,:";
const RATE: f64 = 12_000.0;

fn con_rumore(mut audio: Vec<f32>, seed: u64) -> Vec<f32> {
    let mut a = vec![0.0_f32; 3000];
    a.append(&mut audio);
    a.extend(vec![0.0_f32; 6000]);
    let mut rng = common::Rng(seed);
    common::aggiungi_rumore(&mut a, 0.02, &mut rng);
    a
}

/// fldigi (a 8 kHz, come lavora di solito) riceve il nostro Amtor FEC tutto, cifre e segni compresi.
#[test]
fn fldigi_riceve_il_nostro() {
    let a = con_rumore(AmtorTx::new(RATE, 1000.0, 0.5, false).transmit(TESTO.as_bytes()), 11);
    let low = common::a_8k(&a);
    let mut fl = FldigiSitorb::new(8000.0, 1000.0);
    let got: Vec<u8> = low.chunks(512).flat_map(|b| fl.process(b)).collect();
    let got = String::from_utf8_lossy(&got).into_owned();
    assert!(got.contains(TESTO), "fldigi ha ricevuto {got:?}");
}

/// Noi riceviamo il trasmettitore di fldigi (che manda solo due coppie di fasatura prima del testo).
#[test]
fn riceviamo_fldigi() {
    let a = con_rumore(FldigiSitorb::transmit(RATE, 1000.0, TESTO), 12);
    let mut rx = AmtorRx::new(RATE, 1000.0, false, false);
    for b in a.chunks(512) {
        rx.process(b);
    }
    let got = String::from_utf8_lossy(&rx.take()).into_owned();
    assert!(got.contains("DE IU3ABC IU3ABC IO91WM 599 73 (K)? 1234567890 -/.,:"), "ricevuto {got:?}");
}
