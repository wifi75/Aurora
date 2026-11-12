//! Modi digitali testuali di Aurora, separati dai motori FT8/FT4 (come `mode-sstv`).
//!
//! Tabelle e parametri ripresi da fldigi (Dave Freese W1HKJ, GPL 3, copia in `terze-parti/fldigi`); il resto è scritto da zero dalle
//! specifiche (PSK31 di Peter Martinez G3PLX, RTTY a 45,45 baud con spostamento di 170 Hz).
//!
//! - [`varicode`]: la tabella dei caratteri di PSK31;
//! - [`psk`]: PSK31 e PSK63 (modulatore e ricevitore);
//! - [`psk_viterbi`]: la varicode letta con le probabilità dei bit, con lo squelch sulla probabilità dei caratteri;
//! - [`rtty`]: RTTY a 5 bit (modulatore e ricevitore, che segue quello di fldigi con alcune idee nostre);
//! - [`rtty_multi`]: RTTY multicanale (trova i segnali nella banda e li decodifica tutti);
//! - [`cw`]: CW, alfabeto Morse (modulatore a tono e ricevitore a soglia adattiva);
//! - [`macro_files`]: importa le macro dai file di una cartella dell'utente;
//! - [`tx_stream`]: i modulatori a flusso (trasmissione scritta dal vivo, avanzamento carattere per carattere);
//! - [`cw_bayes`]: il decodificatore CW probabilistico (traliccio con velocità, pugno e modello del testo), di base nel ricevitore CW;
//! - [`blanker`]: noise blanker contro le scariche, davanti ai filtri stretti;
//! - [`skim`]: decodifica di tutti i segnali CW della banda audio, con storico, nominativi e spot locali;
//! - [`service`]: ascolto dall'ingresso audio, trasmissione, impostazioni e macro.

pub mod blanker;
pub mod cw;
pub mod cw_bayes;
#[cfg(feature = "fldigi-oracle")]
pub mod fldigi_oracle;
pub mod macro_files;
#[cfg(feature = "mmtty-oracle")]
pub mod mmtty_oracle;
pub mod psk;
pub mod psk_viterbi;
pub mod rtty;
pub mod rtty_multi;
pub mod service;
pub mod skim;
pub mod tx_stream;
pub mod varicode;

pub use cw::CwDemodulator;
pub use psk::{PskDemodulator, PskMode, RATE};
pub use rtty::{RttyChar, RttyDemodulator, RttyDiversity, RttyParams};
pub use service::{DigiError, DigiMacro, DigiMode, DigiService, DigiSettings, DigiTxPort, ImportHow, RadioModeSource, RttyReceiver};
