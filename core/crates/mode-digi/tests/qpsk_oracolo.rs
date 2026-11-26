//! Il trasmettitore QPSK31 di Aurora ricevuto dal ricevitore QPSK31 di fldigi 4.2.13 (con il suo decodificatore di Viterbi), in banda
//! laterale superiore e con l'inversione: fissa il verso di rotazione delle fasi e la tabella dei simboli.
//! Si compila solo con `--features fldigi-oracle` (vedi `build.rs`).
#![cfg(feature = "fldigi-oracle")]

mod common;

use mode_digi::fldigi_oracle::FldigiPsk;
use mode_digi::psk::{PskMode, modulate_qpsk};

const TESTO: &str = "CQ CQ DE IU3ABC IU3ABC PSE K the quick brown fox 0123456789";

fn fldigi_riceve(reverse_tx: bool, reverse_rx: bool) -> String {
    let mut audio = vec![0.0_f32; 6000];
    audio.extend(modulate_qpsk(TESTO.as_bytes(), 1000.0, PskMode::Qpsk31, 0.5, reverse_tx));
    audio.extend(vec![0.0_f32; 6000]);
    // un po' di rumore come in un ascolto vero (sul silenzio esatto fldigi, che non ha DCD in QPSK, decide simboli tutti uguali)
    let mut rng = common::Rng(0x9E37_79B9_7F4A_7C15);
    common::aggiungi_rumore(&mut audio, 0.02, &mut rng);
    let low = common::a_8k(&audio);
    let mut fl = FldigiPsk::qpsk31(1000.0, 8000.0, reverse_rx);
    let mut got = Vec::new();
    for block in low.chunks(512) {
        got.extend(fl.process(block));
    }
    String::from_utf8_lossy(&got).into_owned()
}

#[test]
fn fldigi_riceve_il_nostro_qpsk31_in_usb() {
    let r = fldigi_riceve(false, false);
    assert!(r.contains(TESTO), "fldigi ha ricevuto: {r:?}");
}

#[test]
fn fldigi_riceve_il_nostro_qpsk31_invertito() {
    let r = fldigi_riceve(true, true);
    assert!(r.contains(TESTO), "fldigi ha ricevuto: {r:?}");
    let sbagliato = fldigi_riceve(true, false);
    assert!(!sbagliato.contains("quick brown"), "senza inversione fldigi non deve leggere: {sbagliato:?}");
}
