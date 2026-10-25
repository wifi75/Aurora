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
    /// Continente del proprio nominativo, se `cty.dat` e' caricato.
    #[serde(default)]
    pub own_continent: Option<String>,
    /// Gli strumenti della radio letti dal CAT (S-meter, potenza, ROS, ALC...), se la radio li da'.
    #[serde(default)]
    pub meters: Option<RadioMeters>,
}

/// Gli strumenti della radio, ognuno presente solo se il CAT sa leggerlo (se manca, il campo non compare nel JSON: l'interfaccia non accetta `null`). Le unita' sono quelle del pannello della radio.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct RadioMeters {
    /// S-meter in punti S (0-9) e oltre in dB sopra S9 (9 + dB/6 non si usa: sono due numeri distinti): `s_points` 0..=9, `s_over_db` 0..=60.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub s_points: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub s_over_db: Option<f32>,
    /// Potenza in uscita, in watt.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub power_w: Option<f32>,
    /// Rapporto d'onda stazionaria (1.0 e oltre).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub swr: Option<f32>,
    /// ALC, 0.0-1.0 della scala.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub alc: Option<f32>,
    /// Compressione del processore vocale, in dB.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub comp_db: Option<f32>,
    /// Corrente dello stadio finale, in ampere.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id_a: Option<f32>,
    /// Tensione di alimentazione, in volt.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub vdd_v: Option<f32>,
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
    /// Continente di chi trasmette (AF, AN, AS, EU, NA, OC, SA), se il paese e' noto.
    #[serde(default)]
    pub continent: Option<String>,
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

/// Una stazione della lista dei desiderati e' stata vista, in un messaggio decodificato o in uno spot del cluster.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct WantedAlert {
    pub call: String,
    /// Perche' e' desiderata: `call`, `prefix`, `country` o `grid`.
    pub reason: String,
    /// Dove e' stata vista: `decode` o `cluster`.
    pub source: String,
    /// Il messaggio decodificato o il commento dello spot.
    pub text: String,
    pub freq_hz: Option<u64>,
    pub band: Option<String>,
    pub snr: Option<i32>,
    pub country: Option<String>,
    pub grid: Option<String>,
    /// Quando (millisecondi Unix).
    pub time_ms: u64,
}

/// E' uscita una versione di Aurora piu' nuova di quella in uso.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct UpdateNotice {
    pub version: String,
    pub tag: String,
    pub notes: String,
    pub url: String,
}

/// Giudizio sul livello dell'audio in ingresso.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LevelState {
    /// Ingresso muto da molto: cavo, dispositivo o volume.
    Silent,
    /// Troppo debole per decodificare bene.
    Low,
    Good,
    /// Alto ma non ancora saturo.
    High,
    /// Saturo da qualche secondo: abbassare il volume della scheda o della radio.
    Overdrive,
}

/// Misura del livello dell'audio in ingresso, una ogni quarto di secondo.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct AudioLevel {
    /// Picco assoluto, 0-1.
    pub peak: f32,
    /// Valore efficace, 0-1.
    pub rms: f32,
    /// Quota di campioni a fondo scala, 0-1.
    pub clipped: f32,
    pub state: LevelState,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Event {
    Status(StationStatus),
    Decode(DecodedMessage),
    QsoLogged(QsoRecord),
    TxHalted,
    Wanted(WantedAlert),
    UpdateAvailable(UpdateNotice),
    AudioLevel(AudioLevel),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_meters_are_left_out_of_the_json_not_null() {
        let m = RadioMeters { s_points: Some(7.0), s_over_db: Some(0.0), ..RadioMeters::default() };
        let json = serde_json::to_value(&m).expect("json");
        assert_eq!(json, serde_json::json!({ "s_points": 7.0, "s_over_db": 0.0 }));
        let back: RadioMeters = serde_json::from_value(json).expect("lettura");
        assert_eq!(back, m);
    }
}
