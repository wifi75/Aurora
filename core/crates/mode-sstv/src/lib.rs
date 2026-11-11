//! SSTV di Aurora, separato dai motori FT8/FT4.
//!
//! Algoritmi ripresi da QSSTV di Johan Maes ON4QZ (GPL 3, <https://github.com/ON4QZ/QSSTV>, copia in `terze-parti/qsstv`):
//! tabella dei modi (`sstvparam.cpp`), intestazione e VIS (`sstvtx.cpp`, `visfskid.cpp`), disposizione delle righe e conversioni di
//! colore (`modes/*.cpp`), inseguimento del sincronismo e correzione dell'inclinazione (`syncprocessor.cpp`). Riscritti in Rust.
//!
//! - [`modes`]: Martin 1/2, Scottie 1/2/DX, Robot 36/72, PD 50/90/120/160/180/240/290, Wraase SC2-180;
//! - [`encoder`]: immagine RGB -> audio con VIS;
//! - [`decoder`]: audio a 12 kHz -> VIS, modo, righe e immagine finita;
//! - [`service`]: ricezione dall'ingresso audio, trasmissione, archivio delle immagini in `sstv/`.

pub mod color;
pub mod decoder;
pub mod demod;
pub mod denoise;
pub mod encoder;
pub mod image_io;
pub mod modes;
pub mod resample;
pub mod service;
pub mod store;
pub mod vis;

pub use decoder::{DecodedImage, Decoder, DecoderConfig, DecoderEvent};
pub use demod::RATE;
pub use encoder::encode;
pub use modes::{MODES, ModeSpec, SstvMode};
pub use service::{SstvService, SstvSettings, TxPhoto, SstvTxPort};

/// Immagine RGB a 8 bit per canale, righe in fila.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RgbImage {
    pub width: u32,
    pub height: u32,
    pub data: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum SstvError {
    BadImage(String),
    BadRate(u32),
    UnknownMode(String),
    NotFound,
    Busy,
    NoTransmitter,
    Tx(String),
    Io(String),
}

impl SstvError {
    /// Codice per le risposte dell'API.
    pub fn code(&self) -> &'static str {
        match self {
            SstvError::BadImage(_) => "bad_image",
            SstvError::BadRate(_) => "bad_rate",
            SstvError::UnknownMode(_) => "unknown_mode",
            SstvError::NotFound => "not_found",
            SstvError::Busy => "busy",
            SstvError::NoTransmitter => "no_transmitter",
            SstvError::Tx(_) => "tx_failed",
            SstvError::Io(_) => "write_failed",
        }
    }
}

impl std::fmt::Display for SstvError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SstvError::BadImage(e) => write!(f, "immagine non valida: {e}"),
            SstvError::BadRate(r) => write!(f, "frequenza di campionamento non valida: {r}"),
            SstvError::UnknownMode(m) => write!(f, "modo SSTV sconosciuto: {m}"),
            SstvError::NotFound => write!(f, "immagine non trovata"),
            SstvError::Busy => write!(f, "trasmissione gia' in corso"),
            SstvError::NoTransmitter => write!(f, "trasmissione non disponibile"),
            SstvError::Tx(e) => write!(f, "trasmissione non riuscita: {e}"),
            SstvError::Io(e) => write!(f, "scrittura non riuscita: {e}"),
        }
    }
}

impl std::error::Error for SstvError {}

/// Descrizione di un modo per l'interfaccia.
#[derive(Debug, Clone, serde::Serialize)]
pub struct ModeInfo {
    pub id: &'static str,
    pub name: &'static str,
    pub width: u32,
    pub height: u32,
    pub vis: u8,
    pub seconds: f64,
    pub color: &'static str,
}

pub fn mode_infos() -> Vec<ModeInfo> {
    MODES
        .iter()
        .map(|m| ModeInfo {
            id: m.id,
            name: m.name,
            width: m.width,
            height: m.height,
            vis: m.vis,
            seconds: (m.total_seconds() * 10.0).round() / 10.0,
            color: match m.family {
                modes::Family::Martin | modes::Family::Scottie | modes::Family::Wraase => "rgb",
                _ => "yuv",
            },
        })
        .collect()
}
