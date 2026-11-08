//! Il comando della radio di Aurora: CAT con Hamlib (rigctld incluso) o, facoltativo su Windows, OmniRig, o TCI per le SDR.

pub mod wake;
pub mod bands;
pub mod cat;
pub mod cat_config;
pub mod controls;
pub mod detect;
pub mod extras;
#[cfg(any(test, feature = "testing"))]
pub mod fake_rigctld;
pub mod hamlib;
pub mod hamlib_rig;
pub mod meters;
pub mod modes;
pub mod omnirig;
pub mod schedule;
pub mod session;
pub mod table;
pub mod tci;

pub use omnirig::WindowsRig;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RigError(pub String);

impl std::fmt::Display for RigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for RigError {}

/// Una radio comandabile. Le chiamate possono bloccare per qualche istante: chi le usa da codice
/// asincrono le sposta su un thread dedicato.
pub trait Rig: Send + Sync {
    fn set_frequency(&self, hz: u64) -> Result<(), RigError>;

    /// La frequenza di ricezione della radio (Hz), se il comando la sa leggere.
    fn frequency(&self) -> Result<u64, RigError> {
        Err(RigError("comando non supportato da questa radio".into()))
    }

    /// I misuratori della radio (S-meter in ricezione; potenza, ROS, ALC... in trasmissione), se il comando li sa leggere.
    fn meters(&self) -> Result<meters::Meters, RigError> {
        Err(RigError("comando non supportato da questa radio".into()))
    }

    /// Imposta la frequenza audio di ricezione del motore (Hz).
    fn set_rx_frequency(&self, hz: u32) -> Result<(), RigError>;

    /// Attenuazione dell'audio di uscita (cursore Pwr), in decimi di dB: 0 = massimo livello.
    fn output_attenuation(&self) -> Result<u32, RigError>;

    fn set_output_attenuation(&self, tenths: u32) -> Result<(), RigError>;

    /// Cambia il modo del motore ("FT8" o "FT4").
    fn set_mode(&self, mode: &str) -> Result<(), RigError>;

    /// Stato degli interruttori del motore (`controls::CONTROLS`): coppie nome e acceso.
    fn controls(&self) -> Result<Vec<(String, bool)>, RigError> {
        Err(RigError("comando non supportato da questa radio".into()))
    }

    /// Gli interruttori e, se la radio li sa leggere, i sei messaggi Tx con quello selezionato, in una lettura sola.
    fn controls_and_tx(&self) -> Result<controls::Reading, RigError> {
        Ok((self.controls()?, None))
    }

    /// Seleziona il messaggio di trasmissione `n` (1-6): il motore trasmette da quello e poi prosegue da solo.
    fn select_tx_message(&self, _n: u8) -> Result<(), RigError> {
        Err(RigError("comando non supportato da questa radio".into()))
    }

    /// Scrive a mano il testo del messaggio Tx `n` (1-6), come i campi dei messaggi: vale finche' i messaggi non si rigenerano.
    fn set_tx_text(&self, _n: u8, _text: &str) -> Result<(), RigError> {
        Err(RigError("comando non supportato da questa radio".into()))
    }

    /// Il clic su un indicatore del display esteso: cambia l'impostazione della radio (NB, NR, notch, ATU, preamplificatore, attenuatore,
    /// AGC, VFO, RIT/XIT) e rilegge quel dato. Solo con il CAT Hamlib.
    fn set_extra(&self, _item: extras::ExtraItem) -> Result<(), RigError> {
        Err(RigError("comando non supportato da questa radio".into()))
    }

    /// Porta un interruttore allo stato voluto (`Some`) o preme un pulsante semplice (`None`).
    fn set_control(&self, _id: &str, _on: Option<bool>) -> Result<(), RigError> {
        Err(RigError("comando non supportato da questa radio".into()))
    }
}
