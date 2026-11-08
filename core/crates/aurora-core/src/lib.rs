//! Tipi di dominio, bus di eventi e trait `Engine` di Aurora.

pub mod adif;
pub mod bus;
pub mod command;
pub mod engine;
pub mod event;
pub mod filters;
pub mod notice;
pub mod version;

pub use bus::Bus;
pub use command::{Command, FoxCommand};
pub use engine::{Engine, EngineIo};
pub use filters::{BandFilters, SharedUdpFilters, UdpFilters, UI_FILTERS_KEY};
pub use event::{AudioLevel, DecodeTag, DecodedMessage, Event, FoxCallerView, FoxLogView, FoxParams, FoxQsoView, FoxSnapshot, FoxVerification, LevelState, PendingQso, QsoRecord, RadioMeters, TxAlert, TxAlertKind, SpectrumRow, StationStatus, UpdateNotice, WantedAlert};

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
