//! WEFAX di Aurora (ricezione delle cartine meteo), separato dai motori FT8/FT4.
//!
//! Algoritmi ripresi da fldigi 4.2.13 di W1HKJ e Remi Chateauneu (GPL 3, copia in `terze-parti/fldigi`: `wefax.cxx`, `wefax-pic.cxx`), a
//! sua volta derivato da HAMFAX di Christof Schmitt. Riscritti in Rust; la trasmissione non c'e'.
//!
//! - [`demod`]: demodulatore FM della sottoportante (1900 Hz, nero 1500 Hz, bianco 2300 Hz) con i tre filtri Narrow, Medium, Wide;
//! - [`decoder`]: APT start e stop, fasatura, allineamento della riga, IOC 576 e 288, 60/90/120/240 righe al minuto, modo manuale e
//!   automatico, correzione dell'inclinazione, immagine che cresce riga per riga, squelch (SQL) e AFC;
//! - [`meter`]: livello del segnale per lo squelch e stima dell'errore di sintonia per l'AFC;
//! - [`service`]: ricezione dall'ingresso audio, impostazioni, archivio delle cartine in `wefax/`;
//! - [`recorder`]: registrazione WAV dell'audio di ogni cartina in `wefax/audio/`.

pub mod decoder;
pub mod demod;
pub mod image_io;
pub mod meter;
pub mod recorder;
pub mod resample;
pub mod service;
pub mod slant;
pub mod store;
pub mod wav;

pub use decoder::{Decoder, DecoderConfig, DecoderEvent, Done, Source, width_of};
pub use demod::{Filter, RATE};
pub use service::{WefaxService, WefaxSettings};

/// Immagine in scala di grigi a 8 bit, righe in fila.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GrayImage {
    pub width: u32,
    pub height: u32,
    pub data: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum WefaxError {
    BadImage(String),
    BadSettings(String),
    /// File audio che non si legge (non e' un WAV, formato o frequenza non ammessi).
    BadAudio(String),
    /// Una decodifica da file e' gia' in corso.
    Busy,
    NotFound,
    Io(String),
}

impl WefaxError {
    /// Codice per le risposte dell'API.
    pub fn code(&self) -> &'static str {
        match self {
            WefaxError::BadImage(_) => "bad_image",
            WefaxError::BadSettings(_) => "bad_settings",
            WefaxError::BadAudio(_) => "bad_audio",
            WefaxError::Busy => "busy",
            WefaxError::NotFound => "not_found",
            WefaxError::Io(_) => "write_failed",
        }
    }
}

impl std::fmt::Display for WefaxError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WefaxError::BadImage(e) => write!(f, "immagine non valida: {e}"),
            WefaxError::BadSettings(e) => write!(f, "impostazioni non valide: {e}"),
            WefaxError::BadAudio(e) => write!(f, "file audio non valido: {e}"),
            WefaxError::Busy => write!(f, "una decodifica da file e' gia' in corso"),
            WefaxError::NotFound => write!(f, "immagine non trovata"),
            WefaxError::Io(e) => write!(f, "scrittura non riuscita: {e}"),
        }
    }
}

impl std::error::Error for WefaxError {}

/// Descrizione di un IOC per l'interfaccia.
#[derive(Debug, Clone, serde::Serialize)]
pub struct IocInfo {
    pub ioc: u32,
    pub width: u32,
    /// Frequenza del tono APT start.
    pub apt_start_hz: u32,
    /// Righe al minuto di partenza.
    pub default_lpm: u32,
}

/// I due IOC della ricezione (con le righe al minuto di partenza delle impostazioni).
pub fn ioc_infos(s: &WefaxSettings) -> Vec<IocInfo> {
    vec![
        IocInfo { ioc: 576, width: width_of(576), apt_start_hz: 300, default_lpm: s.lpm_576 },
        IocInfo { ioc: 288, width: width_of(288), apt_start_hz: 675, default_lpm: s.lpm_288 },
    ]
}
