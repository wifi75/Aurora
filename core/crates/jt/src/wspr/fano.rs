//! Codice convoluzionale K=32 r=1/2 (Layland-Lushbaugh, lo stesso di JT9), interfogliatore e decodificatore sequenziale di Fano di WSPR:
//! `lib/wsprd/fano.c` (Phil Karn KA9Q, ritocchi di K1JT), `encode232.f90`, `inter_wspr.f90` e `deinterleave` di `wsprd_utils.c:283-301`.
//! La tabella delle metriche e' la terza di `lib/wsprd/metric_tables.c` (2-FSK con Es/No 6 dB), con lo scostamento 0,45 e la scala 10 di
//! `wsprd.c:942-946`.

use crate::jt9::{POLY1, POLY2};

/// Bit del messaggio piu' la coda di zeri: 50 + 31 (`nbits` di `wsprd.c:773`).
pub const NBITS: usize = 81;
/// Simboli del canale.
pub const NSYM: usize = 162;
/// Passo della soglia del decodificatore (`delta`, `wsprd.c:829`).
pub const DELTA: i32 = 60;
/// Scostamento della metrica (`bias`, `wsprd.c:830`).
pub const BIAS: f32 = 0.45;

/// `metric_tables[2]` di `lib/wsprd/metric_tables.c`: 2-FSK, Es/No 6 dB, rumore di valore efficace 50.
#[rustfmt::skip]
#[allow(clippy::approx_constant)]
const METRIC_TABLE_2: [f32; 256] = [
    0.9999, 0.9998, 0.9998, 0.9998, 0.9998, 0.9998, 0.9997, 0.9997,
    0.9997, 0.9997, 0.9997, 0.9996, 0.9996, 0.9996, 0.9995, 0.9995,
    0.9994, 0.9994, 0.9994, 0.9993, 0.9993, 0.9992, 0.9991, 0.9991,
    0.9990, 0.9989, 0.9988, 0.9988, 0.9988, 0.9986, 0.9985, 0.9984,
    0.9983, 0.9982, 0.9980, 0.9979, 0.9977, 0.9976, 0.9974, 0.9971,
    0.9969, 0.9968, 0.9965, 0.9962, 0.9960, 0.9957, 0.9953, 0.9950,
    0.9947, 0.9941, 0.9937, 0.9933, 0.9928, 0.9922, 0.9917, 0.9911,
    0.9904, 0.9897, 0.9890, 0.9882, 0.9874, 0.9863, 0.9855, 0.9843,
    0.9832, 0.9819, 0.9806, 0.9792, 0.9777, 0.9760, 0.9743, 0.9724,
    0.9704, 0.9683, 0.9659, 0.9634, 0.9609, 0.9581, 0.9550, 0.9516,
    0.9481, 0.9446, 0.9406, 0.9363, 0.9317, 0.9270, 0.9218, 0.9160,
    0.9103, 0.9038, 0.8972, 0.8898, 0.8822, 0.8739, 0.8647, 0.8554,
    0.8457, 0.8357, 0.8231, 0.8115, 0.7984, 0.7854, 0.7704, 0.7556,
    0.7391, 0.7210, 0.7038, 0.6840, 0.6633, 0.6408, 0.6174, 0.5939,
    0.5678, 0.5410, 0.5137, 0.4836, 0.4524, 0.4193, 0.3850, 0.3482,
    0.3132, 0.2733, 0.2315, 0.1891, 0.1435, 0.0980, 0.0493, 0.0000,
    -0.0510, -0.1052, -0.1593, -0.2177, -0.2759, -0.3374, -0.4005, -0.4599,
    -0.5266, -0.5935, -0.6626, -0.7328, -0.8051, -0.8757, -0.9498, -1.0271,
    -1.1019, -1.1816, -1.2642, -1.3459, -1.4295, -1.5077, -1.5958, -1.6818,
    -1.7647, -1.8548, -1.9387, -2.0295, -2.1152, -2.2154, -2.3011, -2.3904,
    -2.4820, -2.5786, -2.6730, -2.7652, -2.8616, -2.9546, -3.0526, -3.1445,
    -3.2445, -3.3416, -3.4357, -3.5325, -3.6324, -3.7313, -3.8225, -3.9209,
    -4.0248, -4.1278, -4.2261, -4.3193, -4.4220, -4.5262, -4.6214, -4.7242,
    -4.8234, -4.9245, -5.0298, -5.1250, -5.2232, -5.3267, -5.4332, -5.5342,
    -5.6431, -5.7270, -5.8401, -5.9350, -6.0407, -6.1418, -6.2363, -6.3384,
    -6.4536, -6.5429, -6.6582, -6.7433, -6.8438, -6.9478, -7.0789, -7.1894,
    -7.2714, -7.3815, -7.4810, -7.5575, -7.6852, -7.8071, -7.8580, -7.9724,
    -8.1000, -8.2207, -8.2867, -8.4017, -8.5287, -8.6347, -8.7082, -8.8319,
    -8.9448, -9.0355, -9.1885, -9.2095, -9.2863, -9.4186, -9.5064, -9.6386,
    -9.7207, -9.8286, -9.9453, -10.0701, -10.1735, -10.3001, -10.2858, -10.5427,
    -10.5982, -10.7361, -10.7042, -10.9212, -11.0097, -11.0469, -11.1155, -11.2812,
    -11.3472, -11.4988, -11.5327, -11.6692, -11.9376, -11.8606, -12.1372, -13.2539,
];

/// `mettab[2][256]` di `wsprd.c:943-946`: metrica per bit trasmesso 0 (indice 0) e 1 (indice 1), dato il simbolo soft da 0 a 255.
pub fn metric_table(bias: f32) -> [[i32; 256]; 2] {
    let mut t = [[0i32; 256]; 2];
    for i in 0..256 {
        t[0][i] = (10.0 * (METRIC_TABLE_2[i] - bias)).round() as i32;
        t[1][i] = (10.0 * (METRIC_TABLE_2[255 - i] - bias)).round() as i32;
    }
    t
}

/// Il simbolo di due bit (POLY1 nel bit 1, POLY2 nel bit 0) dello stato del codificatore (`ENCODE` di `fano.h:267-276`).
fn encode_sym(state: u32) -> usize {
    ((((state & POLY1).count_ones() & 1) << 1) | ((state & POLY2).count_ones() & 1)) as usize
}

/// `encode` di `fano.c:62-82` (e `encode232.f90`): i bit degli 11 byte, dal piu' alto, in 162 simboli binari.
pub fn encode(data: &[u8; 11]) -> [u8; NSYM] {
    let mut out = [0u8; NSYM];
    let mut state = 0u32;
    let mut k = 0usize;
    'all: for &byte in data {
        for i in (0..8).rev() {
            state = (state << 1) | u32::from((byte >> i) & 1);
            let sym = encode_sym(state);
            for b in [sym >> 1, sym & 1] {
                if k >= NSYM {
                    break 'all;
                }
                out[k] = b as u8;
                k += 1;
            }
        }
    }
    out
}

/// La permutazione a inversione di bit di `inter_wspr.f90:12-28`: `j0[i]` e' il posto del simbolo `i`.
fn interleave_table() -> [usize; NSYM] {
    let mut j0 = [0usize; NSYM];
    let mut k = 0;
    for i in 0..=255u8 {
        let n = i.reverse_bits() as usize;
        if n < NSYM {
            j0[k] = n;
            k += 1;
        }
    }
    j0
}

/// Interfogliatura (`inter_wspr(id, 1)`, `interleave` di `wsprsim_utils.c:588-606`).
pub fn interleave<T: Copy + Default>(sym: &mut [T; NSYM]) {
    let j0 = interleave_table();
    let mut tmp = [T::default(); NSYM];
    for i in 0..NSYM {
        tmp[j0[i]] = sym[i];
    }
    *sym = tmp;
}

/// Il contrario di [`interleave`] (`deinterleave`, `wsprd_utils.c:283-301`).
pub fn deinterleave<T: Copy + Default>(sym: &mut [T; NSYM]) {
    let j0 = interleave_table();
    let mut tmp = [T::default(); NSYM];
    for i in 0..NSYM {
        tmp[i] = sym[j0[i]];
    }
    *sym = tmp;
}

/// Il risultato del decodificatore di Fano.
#[derive(Debug, Clone, Copy)]
pub struct FanoResult {
    /// I byte decodificati (10 scritti dal decodificatore; l'undicesimo resta zero).
    pub data: [u8; 11],
    /// Metrica finale del percorso.
    pub metric: i32,
    /// Cicli fatti (`*cycles = i + 1`).
    pub cycles: u32,
    /// Il decodificatore e' arrivato in fondo prima del limite.
    pub ok: bool,
}

#[derive(Clone, Copy, Default)]
struct Node {
    encstate: u32,
    gamma: i32,
    metrics: [i32; 4],
    tm: [i32; 2],
    i: usize,
}

/// `fano` di `fano.c:87-239`: `symbols` sono 162 simboli soft deinterfogliati (0-255, 128 = nessuna informazione), `maxcycles` i cicli per bit.
pub fn fano(symbols: &[u8; NSYM], mettab: &[[i32; 256]; 2], delta: i32, maxcycles: u32) -> FanoResult {
    let nbits = NBITS;
    let mut nodes = [Node::default(); NBITS + 1];
    let tail = nbits - 31;
    for (n, node) in nodes.iter_mut().enumerate().take(nbits) {
        let (s0, s1) = (symbols[2 * n] as usize, symbols[2 * n + 1] as usize);
        node.metrics = [
            mettab[0][s0] + mettab[0][s1],
            mettab[0][s0] + mettab[1][s1],
            mettab[1][s0] + mettab[0][s1],
            mettab[1][s0] + mettab[1][s1],
        ];
    }
    let mut np = 0usize;
    nodes[np].encstate = 0;
    let lsym = encode_sym(nodes[np].encstate);
    let (m0, m1) = (nodes[np].metrics[lsym], nodes[np].metrics[3 ^ lsym]);
    if m0 > m1 {
        nodes[np].tm = [m0, m1];
    } else {
        nodes[np].tm = [m1, m0];
        nodes[np].encstate += 1;
    }
    nodes[np].i = 0;
    let maxcycles = maxcycles.saturating_mul(nbits as u32);
    nodes[np].gamma = 0;
    let mut t = 0i32;
    let mut i = 1u32;
    while i <= maxcycles {
        let ngamma = nodes[np].gamma + nodes[np].tm[nodes[np].i];
        if ngamma >= t {
            if nodes[np].gamma < t + delta {
                while ngamma >= t + delta {
                    t += delta;
                }
            }
            nodes[np + 1].gamma = ngamma;
            nodes[np + 1].encstate = nodes[np].encstate << 1;
            np += 1;
            if np == nbits {
                break;
            }
            let lsym = encode_sym(nodes[np].encstate);
            if np >= tail {
                nodes[np].tm[0] = nodes[np].metrics[lsym];
            } else {
                let (m0, m1) = (nodes[np].metrics[lsym], nodes[np].metrics[3 ^ lsym]);
                if m0 > m1 {
                    nodes[np].tm = [m0, m1];
                } else {
                    nodes[np].tm = [m1, m0];
                    nodes[np].encstate += 1;
                }
            }
            nodes[np].i = 0;
            i += 1;
            continue;
        }
        loop {
            if np == 0 || nodes[np - 1].gamma < t {
                t -= delta;
                if nodes[np].i != 0 {
                    nodes[np].i = 0;
                    nodes[np].encstate ^= 1;
                }
                break;
            }
            np -= 1;
            if np < tail && nodes[np].i != 1 {
                nodes[np].i += 1;
                nodes[np].encstate ^= 1;
                break;
            }
        }
        i += 1;
    }
    let metric = nodes[np].gamma;
    let mut data = [0u8; 11];
    for (k, d) in data.iter_mut().take(nbits >> 3).enumerate() {
        *d = (nodes[7 + 8 * k].encstate & 0xff) as u8;
    }
    FanoResult { data, metric, cycles: i + 1, ok: i < maxcycles }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn interleaving_is_a_permutation() {
        let mut s = [0u8; NSYM];
        for (i, v) in s.iter_mut().enumerate() {
            *v = i as u8;
        }
        let orig = s;
        interleave(&mut s);
        assert_ne!(s, orig);
        deinterleave(&mut s);
        assert_eq!(s, orig);
    }

    #[test]
    fn the_metric_table_matches_wsprd() {
        let t = metric_table(BIAS);
        // 10 * (0,9999 - 0,45) = 5,499 -> 5
        assert_eq!(t[0][0], 5);
        assert_eq!(t[1][255], 5);
        assert!(t[0][255] < -100);
    }

    #[test]
    fn clean_symbols_decode_and_noise_does_not() {
        let data = [0x12, 0x34, 0x56, 0x78, 0x9a, 0xbc, 0xc0, 0, 0, 0, 0];
        let bits = encode(&data);
        let soft: [u8; NSYM] = std::array::from_fn(|i| if bits[i] == 1 { 220 } else { 36 });
        let r = fano(&soft, &metric_table(BIAS), DELTA, 10_000);
        assert!(r.ok);
        assert_eq!(&r.data[..7], &data[..7]);
        let mut seed = 7u32;
        let junk: [u8; NSYM] = std::array::from_fn(|_| {
            seed = seed.wrapping_mul(1_103_515_245).wrapping_add(12_345);
            (seed >> 16) as u8
        });
        assert!(!fano(&junk, &metric_table(BIAS), DELTA, 500).ok);
    }
}
