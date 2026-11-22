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
