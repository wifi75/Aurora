//! Modi digitali testuali di Aurora, separati dai motori FT8/FT4 (come `mode-sstv`).
//!
//! Tabelle e parametri ripresi da fldigi (Dave Freese W1HKJ, GPL 3, copia in `terze-parti/fldigi`); il resto è scritto da zero dalle
//! specifiche (PSK31 di Peter Martinez G3PLX, RTTY a 45,45 baud con spostamento di 170 Hz).
//!
//! - [`varicode`]: la tabella dei caratteri di PSK31;
//! - [`psk`]: PSK31 e PSK63 (modulatore e ricevitore);
//! - [`rtty`]: RTTY a 5 bit (modulatore e ricevitore, che segue quello di fldigi con alcune idee nostre);
//! - [`cw`]: CW, alfabeto Morse (modulatore a tono e ricevitore a soglia adattiva);
//! - [`macro_files`]: importa le macro dai file di una cartella dell'utente;
//! - [`service`]: ascolto dall'ingresso audio, trasmissione, impostazioni e macro.

pub mod cw;
#[cfg(feature = "fldigi-oracle")]
pub mod fldigi_oracle;
pub mod macro_files;
#[cfg(feature = "mmtty-oracle")]
pub mod mmtty_oracle;
pub mod psk;
pub mod rtty;
pub mod service;
pub mod varicode;

pub use cw::CwDemodulator;
pub use psk::{PskDemodulator, PskMode, RATE};
pub use rtty::{RttyDemodulator, RttyParams};
pub use service::{DigiError, DigiMacro, DigiMode, DigiService, DigiSettings, DigiTxPort, ImportHow, RadioModeSource, RttyReceiver};
