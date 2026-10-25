//! Motore FT4 di Aurora. Gli algoritmi vengono da JTDX 2.2.159 (`lib/ft4/*.f90`, `lib/ft4_decode.f90`, GPL 3).
//!
//! FT4 ha lo stesso messaggio a 77 bit, lo stesso CRC a 14 bit e lo stesso codice LDPC(174,91) di FT8: quelle parti sono quelle del
//! crate [`ft8`]. Cambiano il periodo (7,5 s), la modulazione (4 toni a 20,833 Hz, 576 campioni per simbolo, 103 simboli piu' due di rampa),
//! lo scrambling del messaggio con il vettore [`RVEC`] prima del CRC e tutto il sincronismo.
//!
//! Le parti, nell'ordine in cui lavora la ricezione:
//! - [`sync`]: cerca i segnali nello spettro medio del periodo, li porta a 666,67 Hz di campionamento (32 campioni per simbolo), li allinea in
//!   tempo e frequenza e ne ricava le probabilita' (LLR) dei 174 bit;
//! - [`decode`]: mette tutto insieme su un periodo di 7,5 secondi, con piu' passate e sottrazione dei segnali gia' decodificati;
//! - [`modem`]: in trasmissione, dal messaggio ai 103 toni e all'onda GFSK; serve anche alla sottrazione.
//!
//! Le convenzioni sono quelle di `ft8`: audio a 12 kHz normalizzato a +-1 (il decoder lo porta alla scala dei 16 bit), LLR positivo = bit 1,
//! `dt_s` come lo scrive JTDX (scarto rispetto a 0,5 s dall'inizio del periodo), frequenza = quella del tono 0.

pub mod decode;
pub mod modem;
pub mod sync;

#[cfg(test)]
mod testutil;

/// Campioni al secondo dell'audio.
pub const SAMPLE_RATE: usize = ft8::SAMPLE_RATE;
/// Un periodo FT4 dura 7,5 secondi.
pub const SLOT_SAMPLES: usize = 90_000;
/// Campioni che JTDX tiene del periodo (`NMAX`, 6,144 s): il segnale (5,04 s) comincia a 0,5 s, il resto non serve.
pub const NMAX: usize = 73_728;
/// Campioni per simbolo a 12 kHz (48 ms).
pub const NSPS: usize = 576;
/// Simboli di sincronismo e di dati.
pub const NN: usize = 103;
/// Simboli del segnale con il simbolo di rampa in salita e quello in discesa.
pub const NN2: usize = 105;
/// Simboli di dati (2 bit ciascuno).
pub const ND: usize = 87;
/// Bit del messaggio con il CRC.
pub const K: usize = ft8::K;
/// Bit trasmessi.
pub const N: usize = ft8::N;
/// Distanza tra i toni (Hz): 12000 / 576.
pub const TONE_SPACING_HZ: f32 = 12_000.0 / 576.0;
/// Le quattro matrici di Costas 4x4, nell'ordine in cui compaiono nel segnale (simboli 0-3, 33-36, 66-69, 99-102).
pub const COSTAS: [[u8; 4]; 4] = [[0, 1, 3, 2], [1, 0, 2, 3], [2, 3, 1, 0], [3, 2, 0, 1]];
/// Codifica Gray dei gruppi di 2 bit: indice = bit0 * 2 + bit1 (00 -> 0, 01 -> 1, 10 -> 3, 11 -> 2).
pub const GRAYMAP: [u8; 4] = [0, 1, 3, 2];
/// Vettore di scrambling del messaggio (`rvec` di `genft4`): si somma (XOR) ai 77 bit prima di CRC e codice.
pub const RVEC: [u8; 77] = [
    0, 1, 0, 0, 1, 0, 1, 0, 0, 1, 0, 1, 1, 1, 1, 0, 1, 0, 0, 0, 1, 0, 0, 1, 1, 0, 1, 1, 0, //
    1, 0, 0, 1, 0, 1, 1, 0, 0, 0, 0, 1, 0, 0, 0, 1, 0, 1, 0, 0, 1, 1, 1, 1, 0, 0, 1, 0, 1, //
    0, 1, 0, 1, 0, 1, 1, 0, 1, 1, 1, 1, 1, 0, 0, 0, 1, 0, 1,
];
