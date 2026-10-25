//! Motore FT8 di Aurora. Gli algoritmi vengono da JTDX 2.2.159 e da WSJT-X (GPL 3): vedi `docs/2026-10-24-aurora-motore-ft8.md`.
//!
//! Le parti, nell'ordine in cui lavora la ricezione:
//! - [`sync`]: cerca i segnali nello spettro, li porta a 200 Hz di campionamento e li allinea in tempo e frequenza;
//! - [`modem`]: da quel segnale ricava le probabilita' (LLR) dei 174 bit; in trasmissione fa il contrario (toni e onda GFSK);
//! - [`ldpc`] e [`crc`]: correggono gli errori e controllano il messaggio a 91 bit;
//! - [`message`]: da 77 bit al testo ("CQ DX IU3ABC IO91") e viceversa;
//! - [`decode`]: mette tutto insieme su un periodo di 15 secondi, con piu' passate e sottrazione dei segnali gia' decodificati.

pub mod ap;
pub mod crc;
pub mod decode;
pub mod filters;
pub mod ldpc;
pub mod message;
pub mod modem;
pub mod subtract;
pub mod sync;

/// Campioni al secondo dell'audio: quello che usa JTDX.
pub const SAMPLE_RATE: usize = 12_000;
/// Un periodo FT8 dura 15 secondi; JTDX lavora su 15 s esatti di campioni.
pub const SLOT_SAMPLES: usize = 15 * SAMPLE_RATE;
/// Campioni per simbolo FT8 a 12 kHz (160 ms).
pub const NSPS: usize = 1920;
/// Simboli di un messaggio: 79 (58 di dati e 3x7 di sincronismo).
pub const NN: usize = 79;
/// Simboli di dati.
pub const ND: usize = 58;
/// Bit del codice dopo la correzione: 77 di messaggio + 14 di CRC.
pub const K: usize = 91;
/// Bit trasmessi: 91 + 83 di parita'.
pub const N: usize = 174;
/// Distanza tra i toni (Hz).
pub const TONE_SPACING_HZ: f32 = 6.25;
/// Ordine dei toni di sincronismo (matrice di Costas 7x7), come in WSJT-X.
pub const COSTAS: [u8; 7] = [3, 1, 4, 0, 6, 5, 2];
