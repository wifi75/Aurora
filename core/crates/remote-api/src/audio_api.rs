//! Scheda Audio: cio' che la rotta `audio/*` chiede a chi gestisce i dispositivi (il programma `aurora`).
//!
//! Il gestore apre e chiude ingressi e uscite a caldo; la rotta controlla i valori, chiede l'applicazione e **solo se riesce** salva nello
//! store delle impostazioni. I testi d'errore dei dispositivi arrivano grezzi (`message`): la traduzione dei motivi sta nell'interfaccia.

use serde::Serialize;

use crate::engine_settings::AudioValues;

/// Il valore che vuol dire «predefinito del sistema» (come nel `JTDX.ini`).
pub const SYSTEM_DEFAULT: &str = "Default";

/// Il valore e' la scelta «predefinito del sistema»?
pub fn is_system_default(value: &str) -> bool {
    value.trim().eq_ignore_ascii_case(SYSTEM_DEFAULT)
}

/// Un dispositivo audio del sistema.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AudioDevice {
    pub name: String,
    /// Il numero massimo di canali (1 = mono, 2 = stereo).
    pub channels: u16,
    /// E' il predefinito del sistema.
    pub is_default: bool,
}

/// L'elenco dei dispositivi e quelli in uso adesso.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct AudioDevices {
    pub inputs: Vec<AudioDevice>,
    pub outputs: Vec<AudioDevice>,
    /// L'ingresso che sta davvero ricevendo (`null` se la cascata e' spenta).
    pub input_in_use: Option<String>,
    /// L'uscita su cui si trasmette adesso (`null` se la trasmissione non e' attiva).
    pub output_in_use: Option<String>,
    /// Un'uscita nuova gia' aperta che prende il posto della vecchia alla fine della trasmissione in corso.
    pub output_pending: Option<String>,
    /// La cartella dei file audio quando `SaveDir` e' vuota.
    pub default_save_dir: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PartStatus {
    /// Gia' com'era.
    Unchanged,
    /// Applicato adesso.
    Applied,
    /// Il dispositivo nuovo e' aperto e subentra alla fine della trasmissione in corso.
    Deferred,
    /// Non si puo' applicare a caldo (la trasmissione non e' attiva): vale dal prossimo avvio.
    SavedOnly,
    /// Non riuscito: resta (o torna) quello di prima e la scelta non si salva.
    Failed,
}

/// L'esito di una parte della scheda (ingresso, uscita, file audio).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AudioPart {
    pub status: PartStatus,
    /// Il dispositivo in uso dopo l'operazione.
    pub in_use: Option<String>,
    /// Il nuovo non si e' aperto e si e' riaperto quello di prima.
    pub restored: bool,
    /// Il motivo, dal dispositivo o dal sistema.
    pub message: Option<String>,
}

impl AudioPart {
    pub fn new(status: PartStatus, in_use: Option<String>) -> AudioPart {
        AudioPart { status, in_use, restored: false, message: None }
    }

    pub fn failed(in_use: Option<String>, restored: bool, message: impl Into<String>) -> AudioPart {
        AudioPart { status: PartStatus::Failed, in_use, restored, message: Some(message.into()) }
    }

    pub fn is_failed(&self) -> bool {
        self.status == PartStatus::Failed
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AudioOutcome {
    pub input: AudioPart,
    pub output: AudioPart,
    pub save: AudioPart,
}

/// Chi apre e chiude i dispositivi audio.
pub trait AudioControl: Send + Sync {
    /// I dispositivi del sistema e quelli in uso (puo' bloccare: la rotta lo chiama fuori dal ciclo di rete).
    fn devices(&self) -> AudioDevices;
    /// Applica i valori voluti (tutti e sei). Chi fallisce lascia le cose come stavano e lo dice nella sua parte.
    fn apply(&self, wanted: &AudioValues) -> AudioOutcome;
}

/// Le chiavi dello store che appartengono a ciascuna parte, per salvare solo quelle riuscite.
pub fn keys_of_input() -> [&'static str; 2] {
    ["SoundInName", "AudioInputChannel"]
}

pub fn keys_of_output() -> [&'static str; 2] {
    ["SoundOutName", "AudioOutputChannel"]
}

pub fn keys_of_save() -> [&'static str; 2] {
    ["SaveDir", "SaveWav"]
}
