//! JT65: trasmissione. Messaggio a 72 bit, Reed-Solomon (63,12), interleaving, codice di Gray e 126 toni (`gen65.f90` di WSJT-X).
//!
//! Un periodo dura 60 secondi: il messaggio parte a un secondo dall'inizio. Ogni simbolo dura 4096 campioni a 11025 Hz (0,3715 s); i toni di dati
//! sono 64 spaziati di `2,6917 Hz` per il sottomodo A (`B` x2, `C` x4) e il tono di sincronismo sta due intervalli sotto il tono 0.

use crate::pack::{Kind, pack_message, unpack_message};
use crate::rs::{Codec, DATA, NN};

/// Simboli trasmessi: 63 di dati e 63 di sincronismo, alternati secondo [`SYNC_PATTERN`].
pub const NSYM: usize = 126;
/// Campioni per simbolo a 11025 Hz (la velocita' di campionamento di WSJT-X per JT65).
pub const NSPS_11025: usize = 4096;
/// Distanza tra i toni del sottomodo A in hertz: 11025 / 4096.
pub const TONE_SPACING_HZ: f64 = 11025.0 / 4096.0;
/// Durata di un simbolo in secondi.
pub const SYMBOL_S: f64 = 4096.0 / 11025.0;

/// `nprc` di `gen65.f90`: dove cade il sincronismo (1) e dove i dati (0).
pub const SYNC_PATTERN: [u8; NSYM] = [
    1, 0, 0, 1, 1, 0, 0, 0, 1, 1, 1, 1, 1, 1, 0, 1, 0, 1, 0, 0, 0, 1, 0, 1, 1, 0, 0, 1, 0, 0, 0, 1, 1, 1, 0, 0, 1, 1, 1, 1, 0, 1, 1, 0, 1, 1, 1, 1, 0, 0, 0, 1, 1, 0, 1, 0,
    1, 0, 1, 1, 0, 0, 1, 1, 0, 1, 0, 1, 0, 1, 0, 0, 1, 0, 0, 0, 0, 0, 0, 1, 1, 0, 0, 0, 0, 0, 0, 0, 1, 1, 0, 1, 0, 0, 1, 0, 1, 1, 0, 1, 0, 1, 0, 1, 0, 0, 1, 1, 0, 0, 1, 0,
    0, 1, 0, 0, 0, 0, 1, 1, 1, 1, 1, 1, 1, 1,
];

/// `igray`: `dir > 0` da binario a Gray, altrimenti il contrario.
pub fn gray(n: u8, dir: i32) -> u8 {
    if dir > 0 {
        return n ^ (n >> 1);
    }
    let mut n = n;
    let mut sh = 1;
    let mut nn = n >> sh;
    while nn > 0 {
        n ^= nn;
        sh <<= 1;
        nn = if sh < 8 { n >> sh } else { 0 };
    }
    n
}

/// `interleave63`: `dir > 0` interlaccia, altrimenti disfa (matrice 7 x 9 letta per colonne).
pub fn interleave(d: &[u8; NN], dir: i32) -> [u8; NN] {
    let mut out = [0u8; NN];
    if dir >= 0 {
        for i in 0..7 {
            for j in 0..9 {
                out[j + 9 * i] = d[i + 7 * j];
            }
        }
    } else {
        for i in 0..7 {
            for j in 0..9 {
                out[i + 7 * j] = d[j + 9 * i];
            }
        }
    }
    out
}

/// Gli errori di JT65.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EncodeError {
    /// Il messaggio non e' un messaggio JT65 (le cifre del locatore, o altro).
    Message(String),
}

/// I 63 simboli di dati (da 0 a 63) che portano il messaggio, gia' con Reed-Solomon, interleaving e Gray.
pub fn data_symbols(codec: &Codec, msg: &str) -> ([u8; NN], Kind) {
    let (dgen, kind) = pack_message(msg);
    let mut sent = codec.encode(&dgen);
    sent = interleave(&sent, 1);
    for s in sent.iter_mut() {
        *s = gray(*s, 1);
    }
    (sent, kind)
}

/// I 126 toni di un messaggio (`itone` di `gen65`): 0 e' il tono di sincronismo, da 2 a 65 i toni di dati.
pub fn tones(codec: &Codec, msg: &str) -> [u8; NSYM] {
    let (sent, _) = data_symbols(codec, msg);
    let mut tones = [0u8; NSYM];
    let mut k = 0;
    for (j, t) in tones.iter_mut().enumerate() {
        if SYNC_PATTERN[j] == 0 {
            *t = sent[k] + 2;
            k += 1;
        }
    }
    tones
}

/// L'onda audio di un messaggio a 12 kHz (fase continua): `tones` da [`tones`], `f0` la frequenza del tono di sincronismo, `amplitude` il picco.
/// L'audio comincia al tempo zero della trasmissione e dura `126 * 4096 / 11025` secondi.
pub fn modulate(tones: &[u8; NSYM], f0: f64, amplitude: f32, sample_rate: f64) -> Vec<f32> {
    modulate_drifting(tones, f0, amplitude, sample_rate, 0.0)
}

/// Come [`modulate`] con una deriva lineare della frequenza: `drift_hz` e' di quanto cambia tra l'inizio e la fine del messaggio (per le prove del
/// ricevitore, che segue la deriva).
pub fn modulate_drifting(tones: &[u8; NSYM], f0: f64, amplitude: f32, sample_rate: f64, drift_hz: f64) -> Vec<f32> {
    let symbol = 4096.0 * sample_rate / 11_025.0;
    let total = (NSYM as f64 * symbol).round() as usize;
    let mut out = Vec::with_capacity(total);
    let mut phase = 0.0f64;
    for n in 0..total {
        let k = ((n as f64 / symbol) as usize).min(NSYM - 1);
        let f = f0 + f64::from(tones[k]) * TONE_SPACING_HZ + drift_hz * (n as f64 / total as f64 - 0.5);
        phase += 2.0 * std::f64::consts::PI * f / sample_rate;
        if phase > 2.0 * std::f64::consts::PI {
            phase -= 2.0 * std::f64::consts::PI;
        }
        out.push(amplitude * phase.sin() as f32);
    }
    out
}

/// Il messaggio che si riceverebbe: pacchetto e riletto, come lo trasmette davvero JT65 (maiuscole, spazi, parole abbreviate).
pub fn message_as_sent(msg: &str) -> String {
    unpack_message(&pack_message(msg).0)
}

/// Dai 63 simboli di dati ricevuti (dopo Gray e interleaving, `0..=63`) al testo, se il Reed-Solomon li accetta.
pub fn decode_symbols(codec: &Codec, symbols: &[u8; NN], erasures: &[usize]) -> Option<(String, usize)> {
    let mut deinterleaved = [0u8; NN];
    for (i, s) in symbols.iter().enumerate() {
        deinterleaved[i] = gray(*s, -1);
    }
    let deinterleaved = interleave(&deinterleaved, -1);
    let (dat, nerr) = codec.decode(&deinterleaved, erasures)?;
    let dat: [u8; DATA] = dat;
    Some((unpack_message(&dat), nerr))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gray_code_round_trips() {
        for n in 0..64u8 {
            assert_eq!(gray(gray(n, 1), -1), n);
        }
        assert_eq!(gray(2, 1), 3);
    }

    #[test]
    fn interleaving_round_trips() {
        let mut d = [0u8; NN];
        for (i, v) in d.iter_mut().enumerate() {
            *v = i as u8;
        }
        assert_eq!(interleave(&interleave(&d, 1), -1), d);
        assert_ne!(interleave(&d, 1), d);
    }

    /// Il vettore della guida di WSJT-X (`jt65code "G0XYZ K1ABC FN42"`).
    #[test]
    fn the_wsjtx_manual_vector_matches() {
        let codec = Codec::new();
        let (symbols, _) = data_symbols(&codec, "G0XYZ K1ABC FN42");
        let expected: [u8; 63] = [
            56, 40, 8, 40, 51, 47, 50, 34, 44, 53, 22, 53, 28, 31, 13, 60, 46, 2, 14, 58, 43, 41, 58, 35, 8, 35, 3, 24, 1, 21, 41, 43, 0, 25, 54, 9, 41, 54, 7, 25, 21, 9, 62, 59, 7,
            43, 31, 21, 57, 13, 59, 41, 17, 49, 19, 54, 21, 39, 33, 42, 18, 2, 60,
        ];
        assert_eq!(symbols, expected);
        let (dgen, _) = pack_message("G0XYZ K1ABC FN42");
        assert_eq!(dgen, [61, 36, 45, 30, 3, 55, 3, 2, 14, 5, 33, 40]);
        let (dgen3, _) = pack_message("G0XYZ K1ABC FN43");
        assert_eq!(dgen3, [61, 36, 45, 30, 3, 55, 3, 2, 14, 5, 33, 41]);
    }

    #[test]
    fn a_clean_message_decodes_and_errors_up_to_the_limit_are_corrected() {
        let codec = Codec::new();
        let (mut symbols, _) = data_symbols(&codec, "CQ IU3ABC JN55");
        let (text, nerr) = decode_symbols(&codec, &symbols, &[]).expect("senza errori");
        assert_eq!((text.as_str(), nerr), ("CQ IU3ABC JN55", 0));
        for i in 0..20 {
            symbols[i * 3] = (symbols[i * 3] + 7) % 64;
        }
        let (text, nerr) = decode_symbols(&codec, &symbols, &[]).expect("20 errori si correggono");
        assert_eq!((text.as_str(), nerr), ("CQ IU3ABC JN55", 20));
    }

    #[test]
    fn the_tone_array_has_sync_and_data_in_the_right_places() {
        let codec = Codec::new();
        let t = tones(&codec, "CQ IU3ABC JN55");
        assert_eq!(t.len(), 126);
        assert_eq!(SYNC_PATTERN.iter().filter(|&&p| p == 1).count(), 63);
        for (j, &tone) in t.iter().enumerate() {
            if SYNC_PATTERN[j] == 1 {
                assert_eq!(tone, 0);
            } else {
                assert!((2..=65).contains(&tone));
            }
        }
    }
}
