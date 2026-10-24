//! La decodifica di un periodo intero: piu' passate, sottrazione dei segnali gia' decodificati, elenco senza doppioni.

use crate::message::CallHashes;

/// Un messaggio decodificato.
#[derive(Debug, Clone, PartialEq)]
pub struct Decoded {
    pub freq_hz: f32,
    pub dt_s: f32,
    pub snr_db: i32,
    pub text: String,
}

#[derive(Debug, Clone)]
pub struct Options {
    pub f_low_hz: f32,
    pub f_high_hz: f32,
}

impl Default for Options {
    fn default() -> Self {
        Options { f_low_hz: 200.0, f_high_hz: 4000.0 }
    }
}

/// Decodifica i 15 secondi di audio di un periodo (12 kHz).
pub fn decode_slot(_samples: &[f32], _options: &Options, _hashes: &mut CallHashes) -> Vec<Decoded> {
    Vec::new()
}
