//! JT9: trasmissione. Messaggio a 72 bit, codice convoluzionale K=32 r=1/2, interleaving, codice di Gray e 85 toni (`gen9.f90` di WSJT-X, GPL 3).
//!
//! Un periodo dura 60 secondi e il messaggio parte a un secondo dall'inizio. Ogni simbolo dura 6912 campioni a 12000 Hz (0,576 s) e i toni
//! stanno a `12000 / 6912 = 1,7361 Hz` l'uno dall'altro; il tono 0 e' il sincronismo, i toni 1-8 portano tre bit.

use crate::pack::{Symbols, pack_message};

/// Simboli di un messaggio: 85 (69 di dati, 16 di sincronismo).
pub const NSYM: usize = 85;
/// Simboli di dati.
pub const NDATA: usize = 69;
/// Campioni per simbolo a 12 kHz.
pub const NSPS: usize = 6912;
/// Distanza tra i toni in hertz.
pub const TONE_SPACING_HZ: f64 = 12_000.0 / 6912.0;
/// Bit codificati: 72 di messaggio e 31 di coda, a metro 1/2, piu' uno a zero per fare 69 gruppi da tre.
pub const NBITS: usize = 207;

/// Polinomi del codice convoluzionale K=32 (Layland-Lushbaugh, `conv232.f90`).
pub const POLY1: u32 = 0xf2d0_5351;
pub const POLY2: u32 = 0xe461_3c47;

/// Dove cadono i simboli di sincronismo (`isync` di `jt9sync.f90`).
pub const SYNC: [u8; NSYM] = [
    1, 1, 0, 0, 1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 0, 0, 1, 0, 0, 0, 0, 1, 0, 0, 0,
    0, 0, 1, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 1,
];

/// `entail`: da dodici simboli da 6 bit a 13 byte, con la coda di zeri.
pub fn entail(dgen: &Symbols) -> [u8; 13] {
    let mut out = [0u8; 13];
    let mut bits = 0usize;
    for &sym in dgen {
        for j in 0..6 {
            let bit = (sym >> (5 - j)) & 1;
            out[bits / 8] |= bit << (7 - (bits % 8));
            bits += 1;
        }
    }
    out
}

/// `encode232`: 206 bit di canale (due per ogni bit di ingresso).
pub fn encode232(dat: &[u8; 13]) -> [u8; 206] {
    let mut symbols = [0u8; 206];
    let mut nstate: u32 = 0;
    let mut k = 0usize;
    'outer: for &byte in dat.iter() {
        for i in (0..8).rev() {
            nstate = (nstate << 1) | u32::from((byte >> i) & 1);
            symbols[k] = ((nstate & POLY1).count_ones() & 1) as u8;
            k += 1;
            symbols[k] = ((nstate & POLY2).count_ones() & 1) as u8;
            k += 1;
            if k >= 206 {
                break 'outer;
            }
        }
    }
    symbols
}

/// La permutazione di `interleave9`: i numeri da 0 a 255 con i bit al contrario, tenendo quelli fino a 205.
pub fn interleave_table() -> [usize; 206] {
    let mut j0 = [0usize; 206];
    let mut k = 0;
    for i in 0..256usize {
        let n = (i as u8).reverse_bits() as usize;
        if n <= 205 {
            j0[k] = n;
            k += 1;
        }
    }
    j0
}

/// `interleave9`: `dir > 0` mescola, altrimenti rimette in ordine.
pub fn interleave(ia: &[u8; 206], dir: i32) -> [u8; 206] {
    let j0 = interleave_table();
    let mut ib = [0u8; 206];
    if dir > 0 {
        for i in 0..206 {
            ib[j0[i]] = ia[i];
        }
    } else {
        for i in 0..206 {
            ib[i] = ia[j0[i]];
        }
    }
    ib
}

/// Gray a tre bit (`igray`): `dir > 0` da binario a Gray.
pub fn gray3(n: u8, dir: i32) -> u8 {
    crate::jt65::gray(n, dir) & 7
}

/// I 69 simboli di dati (0..=7) di un messaggio.
pub fn data_symbols(msg: &str) -> [u8; NDATA] {
    let (dgen, _) = pack_message(msg);
    let bytes = entail(&dgen);
    let coded = encode232(&bytes);
    let scrambled = interleave(&coded, 1);
    // il bit 207 vale zero; si impacchettano a tre a tre
    let mut bits = [0u8; NBITS];
    bits[..206].copy_from_slice(&scrambled);
    let mut out = [0u8; NDATA];
    for (i, o) in out.iter_mut().enumerate() {
        let v = (bits[3 * i] << 2) | (bits[3 * i + 1] << 1) | bits[3 * i + 2];
        *o = gray3(v, 1);
    }
    out
}

/// I 85 toni (`i4tone`): 0 per il sincronismo, da 1 a 8 i dati.
pub fn tones(msg: &str) -> [u8; NSYM] {
    let data = data_symbols(msg);
    let mut out = [0u8; NSYM];
    let mut j = 0;
    for (i, t) in out.iter_mut().enumerate() {
        if SYNC[i] == 0 {
            *t = data[j] + 1;
            j += 1;
        }
    }
    out
}

/// L'onda audio a 12 kHz (fase continua): `f0` e' la frequenza del tono 0.
pub fn modulate(tones: &[u8; NSYM], f0: f64, amplitude: f32) -> Vec<f32> {
    let mut out = Vec::with_capacity(NSYM * NSPS);
    let mut phase = 0.0f64;
    for &t in tones {
        let dphi = 2.0 * std::f64::consts::PI * (f0 + f64::from(t) * TONE_SPACING_HZ) / 12_000.0;
        for _ in 0..NSPS {
            phase += dphi;
            if phase > 2.0 * std::f64::consts::PI {
                phase -= 2.0 * std::f64::consts::PI;
            }
            out.push(amplitude * phase.sin() as f32);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sync_has_sixteen_symbols_where_the_protocol_says() {
        assert_eq!(SYNC.iter().filter(|&&s| s == 1).count(), 16);
        let positions: Vec<usize> = SYNC.iter().enumerate().filter(|(_, s)| **s == 1).map(|(i, _)| i + 1).collect();
        assert_eq!(positions, vec![1, 2, 5, 10, 16, 23, 33, 35, 51, 52, 55, 60, 66, 73, 83, 85]);
    }

    #[test]
    fn interleaving_is_a_permutation_that_undoes_itself() {
        let mut seen = [false; 206];
        for &j in &interleave_table() {
            assert!(!seen[j]);
            seen[j] = true;
        }
        let mut d = [0u8; 206];
        for (i, v) in d.iter_mut().enumerate() {
            *v = (i % 2) as u8 ^ ((i / 7) % 2) as u8;
        }
        assert_eq!(interleave(&interleave(&d, 1), -1), d);
    }

    /// Il vettore della guida di WSJT-X (`jt9code "G0XYZ K1ABC FN42"`).
    #[test]
    fn the_wsjtx_manual_vector_matches() {
        let expected: [u8; 85] = [
            0, 0, 7, 3, 0, 3, 2, 5, 4, 0, 1, 7, 7, 7, 8, 0, 4, 8, 8, 2, 2, 1, 0, 1, 1, 3, 5, 4, 5, 6, 8, 7, 0, 6, 0, 1, 8, 3, 3, 7, 8, 1, 1, 2, 4, 5, 8, 1, 5, 2, 0, 0, 8, 6, 0, 5, 8, 5, 1, 0, 5,
            8, 7, 7, 2, 0, 4, 6, 6, 6, 7, 6, 0, 1, 8, 8, 5, 7, 2, 5, 1, 5, 0, 4, 0,
        ];
        assert_eq!(tones("G0XYZ K1ABC FN42"), expected);
    }
}
