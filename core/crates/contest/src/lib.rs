//! I contest di Aurora (pagina «Contest», `docs/piano-contest.md`).
//!
//! - [`regole`]: i file TOML di `core/crates/contest/regole` (uno per contest), letti e controllati;
//! - [`calendario`]: le date del contest dalla regola del file;
//! - [`geo`]: locatori, distanze, prefissi WPX;
//! - [`punteggio`]: doppi, punti, moltiplicatori, punteggio, valore di una stazione, rate e previsione;
//! - [`export`]: Cabrillo 3.0, EDI e ADIF, con il controllo prima dell'invio;
//! - [`servizio`]: contest attivo, impostazioni per contest, log del contest e ascolto del bus.
//!
//! La sequenza dei messaggi di contest sta nel motore (`qso-native::contest`): qui si sceglie quale usare.

pub mod calendario;
pub mod export;
pub mod geo;
pub mod punteggio;
pub mod regole;
pub mod servizio;

pub use regole::Regole;
pub use servizio::{ContestService, Impostazioni, Ripristino};
