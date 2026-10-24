//! Modulazione FT8: dai 174 bit ai 79 toni e all'onda GFSK; in ricezione, dal segnale allineato alle probabilita' dei bit.

use crate::message::{CallHashes, MessageError};

/// Dai 174 bit ai 79 simboli (0-7): sincronismo di Costas all'inizio, a meta' e in fondo, codifica Gray dei dati.
pub fn tones_from_codeword(_codeword: &[u8; 174]) -> [u8; 79] {
    [0; 79]
}

/// Onda GFSK (BT = 2.0) di un messaggio, con frequenza di base `f0_hz`, a 12 kHz.
pub fn gfsk_wave(_tones: &[u8; 79], _f0_hz: f32) -> Vec<f32> {
    Vec::new()
}

/// Dal testo all'onda pronta da trasmettere.
pub fn encode_message(_text: &str, _f0_hz: f32, _hashes: &mut CallHashes) -> Result<Vec<f32>, MessageError> {
    Err(MessageError::Unsupported)
}
