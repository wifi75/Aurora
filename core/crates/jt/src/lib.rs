//! Modi JT65 e JT9 di Aurora. Gli algoritmi vengono da WSJT-X e da JTDX (GPL 3): vedi `docs/2026-10-26-jt65-jt9.md`.
//!
//! - [`pack`]: da testo a 72 bit (dodici simboli da 6 bit) e viceversa;
//! - [`rs`]: codice Reed-Solomon (63,12) di JT65;
//! - [`jt65`]: simboli e toni di JT65 in trasmissione;
//! - [`msk144`]: MSK144 (meteor scatter: codice LDPC 128,90, onda MSK, ricezione dei ping);
//! - [`q65`]: Q65 (toni, onda e ricezione con il codice QRA);
//! - [`jt4`]: JT4 (toni, onda e ricezione);
//! - [`fst4`]: FST4 e FST4W (toni, onda GFSK e ricezione con i codici LDPC 240,101 e 240,74 di [`fst4ldpc`]);
//! - [`fano`]: decodificatore sequenziale di Fano di JT9;
//! - [`jt9`]: simboli e toni di JT9 in trasmissione;
//! - [`jt9rx`]: ricezione di JT9 (sincronismo, simboli soft, Fano);
//! - [`rx`]: ricezione di JT65 (sincronismo, toni, decodifica a decisione soft);
//! - [`wspr`]: WSPR-2 (messaggi a 50 bit, Fano, OSD, onda e ricezione di wsprd).

pub mod fano;

/// La FFT diretta di `n` punti: un pianificatore per thread le ricorda, invece di rifare piano e tabelle a ogni candidato (JT9 e JT65). Lo
/// stesso pianificatore sceglie lo stesso algoritmo, quindi i risultati sono gli stessi.
pub(crate) fn fft_forward(n: usize) -> std::sync::Arc<dyn rustfft::Fft<f32>> {
    thread_local! {
        static PLANNER: std::cell::RefCell<rustfft::FftPlanner<f32>> = std::cell::RefCell::new(rustfft::FftPlanner::new());
    }
    PLANNER.with(|planner| planner.borrow_mut().plan_fft_forward(n))
}

pub mod fst4;
pub mod fst4ldpc;
pub mod fst4msg;
mod fst4tables;
pub mod jt4;
pub mod jt65;
pub mod jt9;
pub mod jt9rx;
pub mod pack;
pub mod prefixes;
pub mod msk144;
pub mod msk40;
mod msk144tables;
pub mod q65;
mod q65fading;
mod q65tables;
pub mod rs;
pub mod rx;
pub mod wspr;

/// Campioni al secondo dell'audio: quello del resto di Aurora.
pub const SAMPLE_RATE: usize = 12_000;
