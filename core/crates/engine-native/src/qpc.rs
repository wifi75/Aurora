//! **AQP-127** (Aurora Q-ary Polar): codice polare q-ario (127, 50) su simboli da 7 bit (q = 128), compatibile con SuperFox di WSJT-X.
//! Implementazione di Tiziano Cassone IU3WJQ.
//! Le posizioni del messaggio (`INFO`) si ricalcolano con `cargo run --release --example tabelle_qpc -- 4 400000`: simulazione del canale
//! a 128 toni e decodifica col genio; l'ordine dei simboli in `XPOS` e' quello del protocollo SuperFox, da rispettare per la compatibilita'.
//!
//! Codifica: i 50 simboli del messaggio vanno nelle posizioni `XPOS[..K]`, le altre restano congelate a zero; poi la trasformata polare
//! (7 stadi a farfalla come una FFT, `y[b + passo] ^= y[b]`). Il simbolo 0 non si trasmette: restano 127 simboli.
//! Decodifica: cancellazione successiva ricorsiva sulle probabilita' dei 128 valori di ogni simbolo; le convoluzioni XOR passano per la
//! trasformata di Walsh-Hadamard.

/// Bit per simbolo e numero di valori.
pub const LOG2Q: usize = 7;
pub const Q: usize = 1 << LOG2Q;
/// Lunghezza della parola di codice prima della foratura, e simboli trasmessi.
pub const LOG2N: usize = 7;
pub const N: usize = 1 << LOG2N;
pub const NP: usize = N - 1;
/// Simboli di informazione.
pub const K: usize = 50;

/// Dove va ogni simbolo di informazione (servono i primi `K`).
const XPOS: [usize; N] = [
      1,   2,   3,   4,   5,   6,   8,   9,  10,  12,  16,  32,  17,  18,  64,  20,
     33,  34,  24,   7,  11,  36,  13,  19,  14,  65,  40,  21,  66,  22,  35,  68,
     25,  48,  37,  26,  72,  15,  38,  28,  41,  67,  23,  80,  42,  69,  49,  96,
     44,  27,  70,  50,  73,  39,  29,  52,  74,  30,  56,  81,  76,  43,  82,  84,
     97,  45,  71,  88,  98,  46, 100,  51, 104,  53,  75, 112,  54,  57,  99, 119,
     92,  77,  58, 117,  59,  83, 106,  31,  85, 108, 115, 116, 122, 125, 124,  91,
     61,  90,  89, 111,  78,  93,  94, 126,  86, 107, 110, 118, 121,  62, 120,  87,
    105,  55, 114,  60, 127,  63, 103, 101, 123,  95, 102,  47, 109,  79, 113,   0,
];

/// 1 = posizione con dati, 0 = congelata (vale sempre 0).
const INFO: [u8; N] = [
      0,   1,   1,   1,   1,   1,   1,   1,   1,   1,   1,   1,   1,   1,   1,   1,
      1,   1,   1,   1,   1,   1,   1,   1,   1,   1,   1,   1,   1,   0,   0,   0,
      1,   1,   1,   1,   1,   1,   1,   0,   1,   1,   1,   0,   1,   0,   0,   0,
      1,   1,   0,   0,   0,   0,   0,   0,   0,   0,   0,   0,   0,   0,   0,   0,
      1,   1,   1,   1,   1,   1,   0,   0,   1,   0,   0,   0,   0,   0,   0,   0,
      1,   0,   0,   0,   0,   0,   0,   0,   0,   0,   0,   0,   0,   0,   0,   0,
      1,   0,   0,   0,   0,   0,   0,   0,   0,   0,   0,   0,   0,   0,   0,   0,
      0,   0,   0,   0,   0,   0,   0,   0,   0,   0,   0,   0,   0,   0,   0,   0,
];

/// Codifica 50 simboli (valori 0..128) nei 127 simboli da trasmettere.
pub fn encode(message: &[u8; K]) -> [u8; NP] {
    let mut y = [0u8; N];
    for (k, &s) in message.iter().enumerate() {
        y[XPOS[k]] = s & (Q as u8 - 1);
    }
    polar_transform(&mut y);
    let mut out = [0u8; NP];
    out.copy_from_slice(&y[1..]);
    out
}

fn polar_transform(y: &mut [u8; N]) {
    for stage in 0..LOG2N {
        let step = 1usize << stage;
        for group in (0..N).step_by(step << 1) {
            for b in group..group + step {
                y[b + step] ^= y[b];
            }
        }
    }
}

/// Decodifica: `py` ha per ognuno dei 127 simboli ricevuti le probabilita' dei 128 valori (`py[i * Q + v]`). Ritorna i 50 simboli del
/// messaggio e la parola di codice ricostruita (127 simboli), per controllare quanto somiglia a quella ricevuta.
pub fn decode(py: &[f32]) -> Option<([u8; K], [u8; NP])> {
    if py.len() != NP * Q {
        return None;
    }
    let mut p = vec![0f32; N * Q];
    // il simbolo 0 non trasmesso e' congelato: si sa che vale 0
    p[0] = 1.0;
    p[Q..].copy_from_slice(py);
    let mut x = [0u8; N];
    let mut y = [0u8; N];
    decode_rows(&mut x, &mut y, &mut p, &INFO, None);
    let mut message = [0u8; K];
    for (k, m) in message.iter_mut().enumerate() {
        *m = x[XPOS[k]];
    }
    let mut word = [0u8; NP];
    word.copy_from_slice(&y[1..]);
    Some((message, word))
}

fn decode_rows(x: &mut [u8], y: &mut [u8], py: &mut [f32], info: &[u8], errors: Option<&mut [f64]>) {
    let rows = x.len();
    if rows == 1 {
        x[0] = argmax(py);
        if let Some(e) = errors {
            // probabilita' di sbagliare questa posizione (misura morbida: con le probabilita' tutte uguali la stima varrebbe 0 per caso)
            let total: f32 = py.iter().sum();
            e[0] += if total > 0.0 { 1.0 - f64::from(py[0] / total) } else { 1.0 };
        }
        // congelato: il valore vero e' 0 qualunque sia la stima
        y[0] = if info[0] == 0 { 0 } else { x[0] };
        return;
    }
    let half = rows / 2;
    let size = half * Q;
    let (py_low, py_high) = py.split_at_mut(size);
    let received_high = py_high.to_vec();
    // meta' alta: convoluzione XOR tra le due meta'
    let mut upper = vec![0f32; size];
    for r in 0..half {
        let rows_q = r * Q..(r + 1) * Q;
        xor_convolution(&mut upper[rows_q.clone()], &py_low[rows_q.clone()], &received_high[rows_q]);
    }
    let (x_low, x_high) = x.split_at_mut(half);
    let (y_low, y_high) = y.split_at_mut(half);
    let (mut e_low, mut e_high) = match errors {
        Some(e) => {
            let (a, b) = e.split_at_mut(half);
            (Some(a), Some(b))
        }
        None => (None, None),
    };
    decode_rows(x_high, y_high, &mut upper, &info[half..], e_high.take());
    // meta' bassa: la meta' alta ricevuta spostata dalla decisione presa, per la meta' bassa ricevuta
    for r in 0..half {
        let hd = usize::from(y_high[r]);
        let shifted: Vec<f32> = (0..Q).map(|v| received_high[r * Q + (v ^ hd)]).collect();
        multiply(&mut py_low[r * Q..(r + 1) * Q], &shifted);
    }
    decode_rows(x_low, y_low, py_low, &info[..half], e_low.take());
    for r in 0..half {
        y_high[r] ^= y_low[r];
    }
}

/// Per il calcolo delle tabelle (`tabelle_qpc`): decodifica a cancellazione successiva con il «genio» (parola tutta a zero, ogni
/// decisione precedente gia' giusta) e somma, per ognuna delle 128 posizioni, la probabilita' di sbagliarla.
#[doc(hidden)]
pub fn genie_errors(py: &[f32], errors: &mut [f64; N]) {
    assert_eq!(py.len(), NP * Q);
    let mut p = vec![0f32; N * Q];
    p[0] = 1.0;
    p[Q..].copy_from_slice(py);
    let mut x = [0u8; N];
    let mut y = [0u8; N];
    decode_rows(&mut x, &mut y, &mut p, &[0u8; N], Some(&mut errors[..]));
}

fn argmax(p: &[f32]) -> u8 {
    let mut best = 0;
    for v in 1..Q {
        if p[v] > p[best] {
            best = v;
        }
    }
    best as u8
}

/// Prodotto riga per riga, normalizzato; se si annulla diventa uniforme.
fn multiply(dst: &mut [f32], other: &[f32]) {
    let mut norm = 0.0;
    for (d, o) in dst.iter_mut().zip(other) {
        *d *= o;
        norm += *d;
    }
    if norm > 0.0 {
        dst.iter_mut().for_each(|d| *d /= norm);
    } else {
        dst.iter_mut().for_each(|d| *d = 1.0 / Q as f32);
    }
}

/// Convoluzione XOR di due distribuzioni: Walsh-Hadamard, prodotto, Walsh-Hadamard e divisione per `Q`.
fn xor_convolution(dst: &mut [f32], a: &[f32], b: &[f32]) {
    let mut fa = [0f32; Q];
    let mut fb = [0f32; Q];
    fa.copy_from_slice(a);
    fb.copy_from_slice(b);
    walsh_hadamard(&mut fa);
    walsh_hadamard(&mut fb);
    for (x, y) in fa.iter_mut().zip(fb) {
        *x *= y;
    }
    walsh_hadamard(&mut fa);
    for (d, v) in dst.iter_mut().zip(fa) {
        *d = v / Q as f32;
    }
}

fn walsh_hadamard(v: &mut [f32; Q]) {
    let mut h = 1;
    while h < Q {
        for i in (0..Q).step_by(h << 1) {
            for j in i..i + h {
                let (a, b) = (v[j], v[j + h]);
                v[j] = a + b;
                v[j + h] = a - b;
            }
        }
        h <<= 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn message(seed: u32) -> [u8; K] {
        let mut s = seed;
        std::array::from_fn(|_| {
            s = s.wrapping_mul(1_103_515_245).wrapping_add(12_345);
            ((s >> 16) & 127) as u8
        })
    }

    /// Probabilita' del simbolo ricevuto: 0.8 sul valore (giusto o sbagliato), il resto sparso su tutti.
    fn soft(word: &[u8; NP], wrong: &[usize]) -> Vec<f32> {
        let mut py = vec![0.2 / Q as f32; NP * Q];
        for (i, &s) in word.iter().enumerate() {
            let v = if wrong.contains(&i) { usize::from(s ^ 0x55) } else { usize::from(s) };
            py[i * Q + v] += 0.8;
        }
        py
    }

    #[test]
    fn tables_have_fifty_information_positions_and_xpos_covers_them() {
        assert_eq!(INFO.iter().filter(|&&f| f == 1).count(), K);
        for &p in &XPOS[..K] {
            assert_eq!(INFO[p], 1, "posizione {p}");
        }
    }

    #[test]
    fn the_transform_is_its_own_inverse_and_the_dropped_symbol_is_frozen() {
        assert_eq!(encode(&[0; K]), [0; NP]);
        let m = message(7);
        let mut y = [0u8; N];
        for (k, &s) in m.iter().enumerate() {
            y[XPOS[k]] = s;
        }
        let u = y;
        polar_transform(&mut y);
        polar_transform(&mut y);
        assert_eq!(y, u);
        assert_eq!(INFO[0], 0);
    }

    #[test]
    fn clean_word_decodes() {
        for seed in 1..20 {
            let m = message(seed);
            let w = encode(&m);
            let (d, word) = decode(&soft(&w, &[])).expect("lunghezza giusta");
            assert_eq!(d, m, "seme {seed}");
            assert_eq!(word, w);
        }
    }

    #[test]
    fn some_wrong_symbols_are_corrected() {
        let m = message(42);
        let w = encode(&m);
        let (d, _) = decode(&soft(&w, &[3, 17, 40, 77, 101, 120])).expect("lunghezza giusta");
        assert_eq!(d, m);
    }

    #[test]
    fn wrong_length_is_refused() {
        assert!(decode(&[0.0; 10]).is_none());
    }
}
