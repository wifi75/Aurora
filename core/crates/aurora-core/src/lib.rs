//! Tipi di dominio, bus di eventi e trait `Engine` di Aurora.

pub mod adif;
pub mod audio_format;
pub mod bus;
pub mod command;
pub mod engine;
pub mod event;
pub mod filters;
pub mod idle;
pub mod notice;
pub mod users;
pub mod version;
pub mod waterfall;

pub use bus::Bus;
pub use command::{Command, ContestSetup, FoxCommand};
pub use engine::{Engine, EngineIo};
pub use filters::{BandFilters, SharedUdpFilters, UdpFilters, UI_FILTERS_KEY};
pub use event::{AudioLevel, DigiEvent, DigiSignal, DigiSignalCall, DecodeTag, DecodedMessage, Event, FoxCallerView, FoxLogView, FoxParams, FoxQsoView, FoxSnapshot, FoxVerification, LevelState, PendingQso, QsoRecord, RadioMeters, TxAlert, TxAlertKind, SpectrumRow, SstvEvent, StationStatus, UpdateNotice, WantedAlert, WsprCycle, WsprSpot};

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
