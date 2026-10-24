//! Tipi di dominio, bus di eventi e trait `Engine` di Aurora.

pub mod bus;
pub mod command;
pub mod engine;
pub mod event;
pub mod version;

pub use bus::Bus;
pub use command::Command;
pub use engine::{Engine, EngineIo};
pub use event::{DecodeTag, DecodedMessage, Event, QsoRecord, SpectrumRow, StationStatus, WantedAlert};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CoreError {
    /// Una riga di spettro non rispetta i limiti (motivo nel messaggio).
    InvalidSpectrum(&'static str),
}

impl std::fmt::Display for CoreError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CoreError::InvalidSpectrum(why) => write!(f, "riga di spettro non valida: {why}"),
        }
    }
}

impl std::error::Error for CoreError {}
