//! QMAP: ricezione panoramica del Q65 su una fetta di banda fino a 90 kHz, come il programma `qmap` di WSJT-X 3 (GPL 3).
//!
//! - [`iq`]: i blocchi I/Q a banda larga e il file WAV a due canali (I a sinistra, Q a destra) per le prove e le registrazioni;
//! - [`tci_iq`]: il flusso I/Q di un SDR via TCI (`iq_samplerate`, `iq_start`, blocchi binari di tipo 0);
//! - [`search`]: la ricerca dei segnali (`getcand2.f90`, `q65b.f90`): spettro medio, picchi del tono di sincronismo, conversione di
//!   ogni candidato a 12 kHz con il tono 0 a 1000 Hz e decodifica con il Q65 di `jt`;
//! - [`service`]: il servizio che raccoglie un periodo, cerca e tiene l'ultimo elenco per l'interfaccia.

pub mod iq;
pub mod search;
pub mod service;
pub mod tci_iq;

pub use search::{QmapDecode, SearchOptions, search};
pub use service::{QmapConfig, QmapService, QmapSnapshot, SourceKind};
