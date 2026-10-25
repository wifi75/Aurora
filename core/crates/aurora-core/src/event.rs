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
    /// `true` se `s_points`/`s_over_db` non vengono dalla radio ma sono stimati dal livello dell'audio in ingresso (le altre letture, se ci sono, sono vere).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub estimated: bool,
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

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
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
    /// Decodifica «ipotizzata» (con informazioni a priori, il marcatore `*` di JTDX): meno sicura delle altre. Non va ai servizi pubblici di spotting e
    /// nel flusso UDP porta il bit di bassa fiducia (`low_confidence`).
    #[serde(default)]
    pub hint: bool,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct QsoRecord {
    pub call: String,
    pub grid: String,
    pub band: String,
    pub mode: String,
    pub freq_hz: u64,
    pub rst_sent: String,
    pub rst_rcvd: String,
    pub time_ms: u32,
    /// Inizio del QSO (millisecondi UTC dall'epoca UNIX): `QSO_DATE` e `TIME_ON` del log. 0 = non noto (si usa `time_ms` con la data di oggi).
    #[serde(default)]
    pub time_on_unix_ms: u64,
    /// Fine del QSO come la scrive JTDX (istante dell'invio di RR73/73 + 14 s in FT8, + 7 s in FT4): `QSO_DATE_OFF` e `TIME_OFF`. 0 = non noto.
    #[serde(default)]
    pub time_off_unix_ms: u64,
    /// Nome dell'operatore (campo `NAME` del log; si scrive nella finestra di conferma del log).
    #[serde(default)]
    pub name: String,
    /// Potenza in watt scritta nel QSO (`TX_PWR`); vuota = quella della stazione.
    #[serde(default)]
    pub tx_power: String,
    /// Commento (campo `COMMENT`): rapporti e distanza se `dBtoComments` e `distanceToComments` sono accesi, piu' quello scritto a mano.
    #[serde(default)]
    pub comment: String,
    /// Messaggio per eQSL (`QSLMSG`): serve solo all'invio a eQSL, non entra nel log.
    #[serde(default)]
    pub eqsl_comment: String,
}

/// Un QSO concluso che aspetta di essere registrato.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PendingQso {
    pub id: u64,
    /// `true`: la finestra di conferma si apre da sola (`PromptToLog`); `false`: il QSO resta in attesa del pulsante «Registra QSO».
    pub prompt: bool,
    /// I campi come li proporrebbe JTDX; chi conferma puo' cambiarli.
    pub record: QsoRecord,
    /// Con eQSL acceso la finestra mostra anche il messaggio per eQSL.
    #[serde(default)]
    pub eqsl: bool,
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
    /// Il motore ha concluso un QSO: non e' ancora nel log. Chi decide se e come registrarlo (`PromptToLog`, `AutoQSOLogging`) lo trasforma in `QsoLogged`.
    QsoCompleted(QsoRecord),
    /// Un QSO aspetta la conferma o la registrazione a mano (finestra di registrazione del log).
    QsoPending(PendingQso),
    /// Il QSO in attesa e' stato registrato o scartato: la finestra si chiude.
    QsoPendingDone { id: u64 },
    TxHalted,
    Wanted(WantedAlert),
    UpdateAvailable(UpdateNotice),
    AudioLevel(AudioLevel),
    /// La banda della radio e' cambiata (da qualunque causa): il motore ha gia' fermato la trasmissione e azzerato la stazione DX;
    /// l'interfaccia svuota attivita' di banda, CQ e novita' e i campi del chiamante. `epoch` cresce a ogni cambio.
    BandChanged { band: Option<String>, dial_hz: u64, epoch: u64 },
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

    #[test]
    fn estimated_flag_is_only_serialised_when_true() {
        let m = RadioMeters { s_points: Some(4.0), estimated: true, ..RadioMeters::default() };
        let json = serde_json::to_value(&m).expect("json");
        assert_eq!(json, serde_json::json!({ "s_points": 4.0, "estimated": true }));
        assert_eq!(serde_json::from_value::<RadioMeters>(json).expect("lettura"), m);
    }
}
