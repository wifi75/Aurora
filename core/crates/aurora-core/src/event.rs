use serde::{Deserialize, Serialize};

use crate::CoreError;

pub const MAX_SPECTRUM_BINS: usize = 4096;

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct StationStatus {
    pub dial_hz: u64,
    pub mode: String,
    pub tx_enabled: bool,
    pub transmitting: bool,
    pub decoding: bool,
    pub de_call: String,
    pub de_grid: String,
    /// Stazione in collegamento (nominativo DX) e suo locatore, se JTDX li riporta.
    #[serde(default)]
    pub dx_call: Option<String>,
    #[serde(default)]
    pub dx_grid: Option<String>,
    /// Rapporto di segnale impostato.
    #[serde(default)]
    pub report: Option<String>,
    /// Frequenze audio di trasmissione e ricezione (Hz).
    #[serde(default)]
    pub tx_df: Option<u32>,
    #[serde(default)]
    pub rx_df: Option<u32>,
    /// Messaggio in trasmissione o pronto.
    #[serde(default)]
    pub tx_message: Option<String>,
    /// Durata del ciclo, in secondi.
    #[serde(default)]
    pub tr_period: Option<u32>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DecodeTag {
    Cq,
    /// Paese mai collegato.
    NewDxcc,
    /// Paese gia' collegato, ma non su questa banda.
    NewDxccBand,
    NewZone,
    NewZoneBand,
    NewItu,
    NewItuBand,
    NewGrid,
    NewGridBand,
    NewPx,
    NewPxBand,
    NewCall,
    NewCallBand,
    /// Stazione gia' collegata su questa banda.
    WorkedCall,
    Auto,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DecodedMessage {
    pub id: u64,
    /// Millisecondi dalla mezzanotte UTC.
    pub time_ms: u32,
    pub snr: i32,
    pub delta_time: f32,
    pub freq_hz: u32,
    pub text: String,
    pub tag: Option<DecodeTag>,
    /// Paese di chi trasmette, se riconosciuto.
    #[serde(default)]
    pub country: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct QsoRecord {
    pub call: String,
    pub grid: String,
    pub band: String,
    pub mode: String,
    pub freq_hz: u64,
    pub rst_sent: String,
    pub rst_rcvd: String,
    pub time_ms: u32,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Event {
    Status(StationStatus),
    Decode(DecodedMessage),
    QsoLogged(QsoRecord),
    TxHalted,
}

/// Una riga della cascata: viaggia su un canale proprio, non e' un `Event`.
#[derive(Clone, Debug, PartialEq)]
pub struct SpectrumRow {
    pub seq: u64,
    pub f_min_hz: u32,
    pub f_max_hz: u32,
    pub bins: Vec<u8>,
}

impl SpectrumRow {
    pub fn validate(&self) -> Result<(), CoreError> {
        if self.bins.is_empty() {
            return Err(CoreError::InvalidSpectrum("nessun bin"));
        }
        if self.bins.len() > MAX_SPECTRUM_BINS {
            return Err(CoreError::InvalidSpectrum("troppi bin"));
        }
        if self.f_min_hz >= self.f_max_hz {
            return Err(CoreError::InvalidSpectrum("intervallo di frequenza invertito"));
        }
        Ok(())
    }
}
