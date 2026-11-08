//! QMAP: ricezione panoramica del Q65 su una fetta di banda fino a 90 kHz, come il programma `qmap` di WSJT-X 3 (GPL 3).
//!
//! - [`iq`]: i blocchi I/Q a banda larga e il file WAV a due canali (I a sinistra, Q a destra) per le prove e le registrazioni;
//! - [`tci_iq`]: il flusso I/Q di un SDR via TCI (`iq_samplerate`, `iq_start`, blocchi binari di tipo 0);
//! - [`sync`]: gli spettri a mezzo simbolo, i candidati e il loro sincronismo (`symspec.f90`, `getcand2.f90`, `q65_sync.f90`);
//! - [`search`]: la ricerca dei segnali (`qmapa.f90`, `q65b.f90`): conversione di ogni candidato a 12 kHz con il tono 0 a 1000 Hz,
//!   decodifica con il Q65 di `jt`, passaggi a meta' minuto, Q65-30 nelle due meta' del minuto e media dei periodi;
//! - [`astro`]: la Luna e il Doppler della propria eco (`astro.f90`, `moondop.f90`, `moon2.f90`);
//! - [`service`]: il servizio che raccoglie il periodo, lancia i passaggi della ricerca e tiene l'elenco per l'interfaccia.

pub mod astro;
pub mod iq;
pub mod search;
pub mod service;
pub mod sync;
pub mod tci_iq;

pub use search::{QmapDecode, SearchOptions, search};
pub use service::{QmapConfig, QmapService, QmapSnapshot, SourceKind};
