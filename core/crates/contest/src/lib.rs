//! I contest di Aurora (pagina «Contest», `docs/piano-contest.md`).
//!
//! - [`regole`]: i file TOML di `core/crates/contest/regole` (uno per contest), letti e controllati;
//! - [`cartelle`]: i contest a cartella (`contest/<nome>/`: `contest.toml`, regole, dati, macro, prove), date delle edizioni, due radici;
//! - [`calendario`]: le date del contest dalla regola del file;
//! - [`geo`]: locatori, distanze, prefissi WPX;
//! - [`liste`]: sezioni ARRL/RAC, stati USA, province canadesi e olandesi per controllare lo scambio ricevuto;
//! - [`punteggio`]: doppi, punti, moltiplicatori, punteggio, valore di una stazione, rate e previsione;
//! - [`export`]: Cabrillo 3.0, EDI e ADIF, con il controllo prima dell'invio;
//! - [`elenco`]: i contest dei modi testuali (file `.contest`, macro e domini), uno attivo alla volta;
//! - [`punti_testo`]: punteggio, statistica per banda e Cabrillo del contest attivo dei modi testuali;
//! - [`regole_punti`]: le regole del punteggio dei contest dei modi testuali (sezione `[Score]`), a pezzi riusabili;
//! - [`script`]: gli script Rhai dei casi speciali (sezione `[Script]`), con limiti e senza accesso a file, rete o processi;
//! - [`pacchetto`]: importa ed esporta un contest (`.zip` con il `.contest`, le macro e i domini);
//! - [`qtc`]: i QTC del WAE (chi li scambia, cosa mandare, righe del Cabrillo);
//! - [`servizio`]: contest attivo, impostazioni per contest, log del contest e ascolto del bus.
//!
//! La sequenza dei messaggi di contest sta nel motore (`qso-native::contest`): qui si sceglie quale usare.

pub mod calendario;
pub mod cartelle;
pub mod elenco;
pub mod export;
pub mod geo;
pub mod liste;
pub mod punteggio;
pub mod punti_testo;
pub mod pacchetto;
pub mod qtc;
pub mod regole;
pub mod regole_punti;
pub mod script;
pub mod servizio;

pub use regole::Regole;
pub use servizio::{ContestService, Impostazioni, Ripristino};
