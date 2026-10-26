//! Modi JT65 e JT9 di Aurora. Gli algoritmi vengono da WSJT-X e da JTDX (GPL 3): vedi `docs/2026-10-26-jt65-jt9.md`.
//!
//! - [`pack`]: da testo a 72 bit (dodici simboli da 6 bit) e viceversa;
//! - [`rs`]: codice Reed-Solomon (63,12) di JT65;
//! - [`jt65`]: simboli e toni di JT65 in trasmissione;
//! - [`fano`]: decodificatore sequenziale di Fano di JT9;
//! - [`jt9`]: simboli e toni di JT9 in trasmissione;
//! - [`jt9rx`]: ricezione di JT9 (sincronismo, simboli soft, Fano);
//! - [`rx`]: ricezione di JT65 (sincronismo, toni, decodifica a decisione soft).

pub mod fano;
pub mod jt65;
pub mod jt9;
pub mod jt9rx;
pub mod pack;
pub mod prefixes;
pub mod rs;
pub mod rx;

/// Campioni al secondo dell'audio: quello del resto di Aurora.
pub const SAMPLE_RATE: usize = 12_000;
