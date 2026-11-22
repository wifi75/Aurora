//! Il codificatore di Olivia e Contestia confrontato, blocco per blocco, con quello della libreria di Jalocha di fldigi 4.2.13.
//! Si compila solo con `--features fldigi-oracle` (vedi `build.rs`): senza non c'e' niente da provare.
#![cfg(feature = "fldigi-oracle")]

use mode_digi::fldigi_oracle::olivia_encode_block;
use mode_digi::olivia::{Encoder, Variant};

/// Una sequenza di caratteri di prova ripetibile (senza dipendenze): tutti i valori da 0 a 255.
fn chars(seed: u64) -> Vec<u8> {
    let mut x = seed.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1_442_695_040_888_963_407);
    (0..8)
        .map(|_| {
            x = x.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1_442_695_040_888_963_407);
            (x >> 56) as u8
        })
        .collect()
}

#[test]
fn the_olivia_blocks_are_identical_to_fldigis_for_every_number_of_tones() {
    for bits in 1..=8 {
        let ours = Encoder::new(Variant::Olivia, bits).expect("bits validi");
        for seed in 0..200u64 {
            let input = chars(seed);
            let theirs = olivia_encode_block(bits, false, &input);
            assert_eq!(ours.encode_block(&input), theirs, "Olivia, {bits} bit per simbolo, caratteri {input:?}");
        }
    }
}

#[test]
fn the_contestia_blocks_are_identical_to_fldigis_for_every_number_of_tones() {
    for bits in 1..=8 {
        let ours = Encoder::new(Variant::Contestia, bits).expect("bits validi");
        for seed in 0..200u64 {
            let input = chars(seed + 1000);
            let theirs = olivia_encode_block(bits, true, &input);
            assert_eq!(ours.encode_block(&input), theirs, "Contestia, {bits} bit per simbolo, caratteri {input:?}");
        }
    }
}

#[test]
fn readable_text_blocks_match_too() {
    for (bits, text) in [(5usize, &b"CQ CQ DE"[..]), (4, b"IU3ABC K"), (3, b"hello"), (8, b"12345678")] {
        let ours = Encoder::new(Variant::Olivia, bits).expect("bits validi");
        assert_eq!(ours.encode_block(text), olivia_encode_block(bits, false, text));
        let cont = Encoder::new(Variant::Contestia, bits).expect("bits validi");
        assert_eq!(cont.encode_block(text), olivia_encode_block(bits, true, text));
    }
}

use mode_digi::fldigi_oracle::FldigiOlivia;
use mode_digi::olivia::{Params, Transmitter};

/// Trasmette con il nostro modulatore e fa decodificare dal ricevitore di fldigi: il testo deve tornare uguale.
fn through_fldigi(variant: Variant, tones: usize, bandwidth: usize, center_hz: f64, text: &[u8], noise: f32) -> String {
    let tx = Transmitter::new(Params { variant, tones, bandwidth, center_hz }).expect("parametri validi");
    let mut audio = vec![0.0f32; 8000];
    audio.extend(tx.transmit(text));
    audio.extend(std::iter::repeat_n(0.0f32, 8000 * 8));
    if noise > 0.0 {
        let mut x: u64 = 12345;
        for s in audio.iter_mut() {
            let mut g = 0.0f32;
            for _ in 0..6 {
                x = x.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1_442_695_040_888_963_407);
                g += ((x >> 33) as f32) / (u32::MAX >> 1) as f32;
            }
            *s = *s * 0.5 + noise * (g - 3.0) * 0.8165;
        }
    }
    let mut rx = FldigiOlivia::new(tones, bandwidth, center_hz, variant == Variant::Contestia);
    for chunk in audio.chunks(512) {
        rx.process(chunk);
    }
    rx.flush();
    String::from_utf8_lossy(&rx.take()).replace('\0', "")
}

#[test]
fn our_olivia_transmission_is_decoded_by_fldigi_in_the_common_variants() {
    let text = b"CQ CQ CQ DE IU3ABC IU3ABC IO91WM K";
    for (tones, bandwidth) in [(8usize, 250usize), (16, 500), (32, 1000), (4, 250), (8, 500)] {
        let got = through_fldigi(Variant::Olivia, tones, bandwidth, 1500.0, text, 0.0);
        assert!(got.contains("CQ CQ CQ DE IU3ABC IU3ABC IO91WM K"), "Olivia {tones}/{bandwidth}: ricevuto {got:?}");
    }
}

#[test]
fn our_contestia_transmission_is_decoded_by_fldigi() {
    let got = through_fldigi(Variant::Contestia, 16, 500, 1500.0, b"CQ CQ DE IU3ABC K", 0.0);
    assert!(got.contains("CQ CQ DE IU3ABC K"), "Contestia 16/500: ricevuto {got:?}");
}

#[test]
fn our_olivia_survives_some_noise_when_decoded_by_fldigi() {
    let got = through_fldigi(Variant::Olivia, 32, 1000, 1500.0, b"CQ CQ CQ DE IU3ABC IU3ABC K", 0.15);
    assert!(got.contains("IU3ABC"), "Olivia 32/1000 con rumore: ricevuto {got:?}");
}
