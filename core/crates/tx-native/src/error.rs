//! Errori della catena di trasmissione.

use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TxError {
    /// `allow_tx` e' falso: nessuna trasmissione.
    NotAllowed,
    /// Il livello (attenuazione) non e' stato impostato.
    LevelNotSet,
    /// C'e' gia' una trasmissione in attesa o in corso.
    Busy,
    EmptyWave,
    /// L'onda contiene valori non numerici.
    InvalidWave,
    /// L'onda dura piu' del consentito.
    TooLong { wave_ms: u64, limit_ms: u64 },
    /// Il periodo richiesto e' quello in corso ma e' troppo tardi per iniziare.
    TooLate { late_ms: u64 },
    InvalidConfig(String),
    Audio(String),
    Ptt(String),
    Thread(String),
}

impl fmt::Display for TxError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TxError::NotAllowed => write!(f, "trasmissione non consentita"),
            TxError::LevelNotSet => write!(f, "livello di trasmissione non impostato"),
            TxError::Busy => write!(f, "una trasmissione e' gia' in attesa o in corso"),
            TxError::EmptyWave => write!(f, "onda vuota"),
            TxError::InvalidWave => write!(f, "onda con valori non validi"),
            TxError::TooLong { wave_ms, limit_ms } => write!(f, "onda di {wave_ms} ms, oltre il limite di {limit_ms} ms"),
            TxError::TooLate { late_ms } => write!(f, "troppo tardi per il periodo richiesto ({late_ms} ms di ritardo)"),
            TxError::InvalidConfig(e) => write!(f, "impostazioni di trasmissione non valide: {e}"),
            TxError::Audio(e) => write!(f, "uscita audio: {e}"),
            TxError::Ptt(e) => write!(f, "PTT: {e}"),
            TxError::Thread(e) => write!(f, "thread di trasmissione non avviato: {e}"),
        }
    }
}

impl std::error::Error for TxError {}
