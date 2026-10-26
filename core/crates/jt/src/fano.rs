//! Decodificatore sequenziale di Fano per il codice K=32 r=1/2 di JT9 (`fano232.f90` e `jt9fano.f90`, tradotti da Phil Karn KA9Q). La tabella delle
//! metriche e' identica in `jtdxjt9.exe` 2.2.159: letta con REA a `0x5b4580` (256 valori uguali, verificati uno per uno), come `ndelta` (170),
//! `nbits` (72 + 31), la rampa oltre il valore 160 (pendenza 2) e il limite dei cicli di `jt9fano_`.

use crate::jt9::{POLY1, POLY2};
use crate::pack::{Symbols, unpack_message};

/// Metrica dei simboli soft (`xx0` di `jt9fano.f90`), 256 valori.
const XX0: [f32; 256] = [
    1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0,
    1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0,
    1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0,
    1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0,
    1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0,
    1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0,
    0.988, 1.0, 0.991, 0.993, 1.0, 0.995, 1.0, 0.991,
    1.0, 0.991, 0.992, 0.991, 0.99, 0.99, 0.992, 0.996,
    0.99, 0.994, 0.993, 0.991, 0.992, 0.989, 0.991, 0.987,
    0.985, 0.989, 0.984, 0.983, 0.979, 0.977, 0.971, 0.975,
    0.974, 0.97, 0.97, 0.97, 0.967, 0.962, 0.96, 0.957,
    0.956, 0.953, 0.942, 0.946, 0.937, 0.933, 0.929, 0.92,
    0.917, 0.911, 0.903, 0.895, 0.884, 0.877, 0.869, 0.858,
    0.846, 0.834, 0.821, 0.806, 0.79, 0.775, 0.755, 0.737,
    0.713, 0.691, 0.667, 0.64, 0.612, 0.581, 0.548, 0.51,
    0.472, 0.425, 0.378, 0.328, 0.274, 0.212, 0.146, 0.075,
    0.0, -0.079, -0.163, -0.249, -0.338, -0.425, -0.514, -0.606,
    -0.706, -0.796, -0.895, -0.987, -1.084, -1.181, -1.28, -1.376,
    -1.473, -1.587, -1.678, -1.79, -1.882, -1.992, -2.096, -2.201,
    -2.301, -2.411, -2.531, -2.608, -2.69, -2.829, -2.939, -3.058,
    -3.164, -3.212, -3.377, -3.463, -3.55, -3.768, -3.677, -3.975,
    -4.062, -4.098, -4.186, -4.261, -4.472, -4.621, -4.623, -4.608,
    -4.822, -4.87, -4.652, -4.954, -5.108, -5.377, -5.544, -5.995,
    -5.632, -5.826, -6.304, -6.002, -6.559, -6.369, -6.658, -7.016,
    -6.184, -7.332, -6.534, -6.152, -6.113, -6.288, -6.426, -6.313,
    -9.966, -6.371, -9.966, -7.055, -9.966, -6.629, -6.313, -9.966,
    -5.858, -9.966, -9.966, -9.966, -9.966, -9.966, -9.966, -9.966,
    -9.966, -9.966, -9.966, -9.966, -9.966, -9.966, -9.966, -9.966,
    -9.966, -9.966, -9.966, -9.966, -9.966, -9.966, -9.966, -9.966,
    -9.966, -9.966, -9.966, -9.966, -9.966, -9.966, -9.966, -9.966,
    -9.966, -9.966, -9.966, -9.966, -9.966, -9.966, -9.966, -9.966,
    -9.966, -9.966, -9.966, -9.966, -9.966, -9.966, -9.966, -9.966,
];

const MAXBITS: usize = 103;
const NDELTA: i32 = 170;

/// `mettab(-128:127, 0:1)`: metrica di un simbolo soft per ipotesi di bit zero (indice 0) e uno (indice 1); l'indice e' il simbolo piu' 128.
fn metric_table() -> Vec<[i32; 2]> {
    let nint = |x: f32| x.round() as i32;
    let mut t = vec![[0i32; 2]; 256];
    let (bias, scale, ib, slope) = (0.5f32, 50.0f32, 160usize, 2i32);
    for i in 0..256usize {
        t[i][0] = nint(scale * (XX0[i] - bias));
        if i > ib {
            t[i][0] = t[ib][0] - slope * (i - ib) as i32;
        }
        if i >= 1 {
            t[256 - i][1] = t[i][0];
        }
    }
    t[0][1] = t[1][1];
    t
}

fn parity(n: u32) -> usize {
    (n.count_ones() & 1) as usize
}

/// Il risultato del decodificatore di Fano.
pub struct FanoResult {
    pub bytes: [u8; 13],
    pub cycles: usize,
    pub metric: i32,
}

/// `fano232`: `symbols` sono 206 simboli soft con segno, `nbits` 103 (72 + 31 di coda). Ritorna i byte decodificati e i cicli fatti.
pub fn fano232(symbols: &[i8], nbits: usize, maxcycles: usize) -> Option<FanoResult> {
    let mettab = metric_table();
    let at = |s: i8, b: usize| mettab[(i32::from(s) + 128) as usize][b];
    let ntail = nbits - 31;
    let mut metrics = [[0i32; 4]; MAXBITS + 1];
    for np in 0..nbits {
        let a = symbols[2 * np];
        let b = symbols[2 * np + 1];
        metrics[np][0] = at(a, 0) + at(b, 0);
        metrics[np][1] = at(a, 0) + at(b, 1);
        metrics[np][2] = at(a, 1) + at(b, 0);
        metrics[np][3] = at(a, 1) + at(b, 1);
    }
    let mut nstate = [0u32; MAXBITS + 1];
    let mut gamma = [0i32; MAXBITS + 1];
    let mut tm = [[0i32; 2]; MAXBITS + 1];
    let mut ii = [0usize; MAXBITS + 1];

    let lsym_of = |state: u32| -> usize { 2 * parity(state & POLY1) + parity(state & POLY2) };

    let mut np = 0usize;
    nstate[np] = 0;
    let lsym = lsym_of(nstate[np]);
    let (m0, m1) = (metrics[np][lsym], metrics[np][3 ^ lsym]);
    if m0 > m1 {
        tm[np] = [m0, m1];
    } else {
        tm[np] = [m1, m0];
        nstate[np] += 1;
    }
    ii[np] = 0;
    gamma[np] = 0;
    let mut nt = 0i32;
    let total = nbits * maxcycles;
    let mut i = 1usize;
    let mut done = false;
    while i <= total {
        let ngamma = gamma[np] + tm[np][ii[np]];
        if ngamma >= nt {
            if gamma[np] < nt + NDELTA {
                nt += NDELTA * ((ngamma - nt) / NDELTA);
            }
            gamma[np + 1] = ngamma;
            nstate[np + 1] = nstate[np] << 1;
            np += 1;
            if np == nbits {
                done = true;
                break;
            }
            let lsym = lsym_of(nstate[np]);
            if np >= ntail {
                tm[np][0] = metrics[np][lsym];
            } else {
                let (m0, m1) = (metrics[np][lsym], metrics[np][3 ^ lsym]);
                if m0 > m1 {
                    tm[np] = [m0, m1];
                } else {
                    tm[np] = [m1, m0];
                    nstate[np] += 1;
                }
            }
            ii[np] = 0;
        } else {
            loop {
                let noback = np == 0 || gamma[np - 1] < nt;
                if noback {
                    nt -= NDELTA;
                    if ii[np] != 0 {
                        ii[np] = 0;
                        nstate[np] ^= 1;
                    }
                    break;
                }
                np -= 1;
                if np < ntail && ii[np] != 1 {
                    ii[np] += 1;
                    nstate[np] ^= 1;
                    break;
                }
            }
        }
        i += 1;
    }
    if !done {
        return None;
    }
    let nbytes = nbits.div_ceil(8);
    let mut bytes = [0u8; 13];
    let mut p = 7usize;
    for b in bytes.iter_mut().take(nbytes - 1) {
        *b = (nstate[p] & 0xff) as u8;
        p += 8;
    }
    Some(FanoResult { bytes, cycles: i, metric: gamma[np] })
}

/// `jt9fano`: da 207 simboli soft al messaggio, se il decodificatore di Fano arriva in fondo entro `limit` cicli per bit.
pub fn decode_message(symbols: &[i8; 207], limit: usize) -> Option<String> {
    let r = fano232(&symbols[..206], 72 + 31, limit)?;
    if r.cycles >= (72 + 31) * limit {
        return None;
    }
    // 72 bit in dodici simboli da 6 bit
    let mut words: Symbols = [0; 12];
    for (w, word) in words.iter_mut().enumerate() {
        let mut v = 0u8;
        for j in 0..6 {
            let bit_index = w * 6 + j;
            let bit = (r.bytes[bit_index / 8] >> (7 - bit_index % 8)) & 1;
            v = (v << 1) | bit;
        }
        *word = v;
    }
    let msg = unpack_message(&words);
    if msg.contains("000AAA ") || msg.is_empty() {
        return None;
    }
    Some(msg)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::jt9::{encode232, entail};
    use crate::pack::pack_message;

    /// Simboli soft di un messaggio senza rumore: +-100.
    fn clean_soft(msg: &str) -> [i8; 207] {
        let (dgen, _) = pack_message(msg);
        let coded = encode232(&entail(&dgen));
        let mut soft = [0i8; 207];
        for (i, &c) in coded.iter().enumerate() {
            soft[i] = if c == 1 { 100 } else { -100 };
        }
        soft
    }

    #[test]
    fn the_metric_table_has_the_expected_shape() {
        let t = metric_table();
        assert_eq!(t[128][0], -25, "il simbolo 0: (0 - 0,5) * 50");
        assert!(t[255][0] < t[200][0]);
        assert_eq!(t[0][1], t[1][1]);
    }

    #[test]
    fn a_clean_message_decodes_through_fano() {
        let soft = clean_soft("CQ IU3ABC JN55");
        assert_eq!(decode_message(&soft, 5000).as_deref(), Some("CQ IU3ABC JN55"));
    }

    #[test]
    fn a_noisy_message_still_decodes_and_garbage_does_not() {
        let mut soft = clean_soft("K1ABC IU3ABC -15");
        let mut seed = 12345u32;
        for s in soft.iter_mut().take(206) {
            seed = seed.wrapping_mul(1_103_515_245).wrapping_add(12_345);
            let noise = ((seed >> 16) % 160) as i32 - 80;
            *s = (i32::from(*s) + noise).clamp(-127, 127) as i8;
        }
        assert_eq!(decode_message(&soft, 30_000).as_deref(), Some("K1ABC IU3ABC -15"));
        let mut junk = [0i8; 207];
        for s in junk.iter_mut() {
            seed = seed.wrapping_mul(1_103_515_245).wrapping_add(12_345);
            *s = ((seed >> 16) % 255) as i8;
        }
        assert_eq!(decode_message(&junk, 5000), None);
    }
}
