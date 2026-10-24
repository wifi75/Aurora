//! Messaggi di 77 bit (`packjt77`): nominativi standard, /P, non standard con hash, locatori, rapporti, testo libero.

/// 77 bit, uno per elemento (0 o 1).
pub type Bits77 = [u8; 77];

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MessageError {
    /// Tipo di messaggio non gestito.
    Unsupported,
    /// Il testo non e' un messaggio valido.
    Invalid(String),
}

/// I nominativi visti finora, per i messaggi che ne portano solo l'impronta (hash a 10, 12 e 22 bit).
#[derive(Debug, Default, Clone)]
pub struct CallHashes {
    // riempito dall'implementazione
}

/// Dal testo ai 77 bit.
pub fn pack(_text: &str, _hashes: &mut CallHashes) -> Result<Bits77, MessageError> {
    Err(MessageError::Unsupported)
}

/// Dai 77 bit al testo.
pub fn unpack(_bits: &Bits77, _hashes: &mut CallHashes) -> Result<String, MessageError> {
    Err(MessageError::Unsupported)
}
