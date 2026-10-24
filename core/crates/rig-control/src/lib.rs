//! Comando della radio condiviso con JTDX: JTDX usa OmniRig, che accetta piu' client insieme,
//! quindi Aurora puo' cambiare frequenza senza togliere la radio a JTDX.

pub mod bands;
pub mod controls;
pub mod modes;
pub mod omnirig;
pub mod schedule;
pub mod session;
pub mod snapshot;
pub mod table;

pub mod jtdx_process;
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

    /// Imposta la frequenza audio di ricezione di JTDX (Hz).
    fn set_rx_frequency(&self, hz: u32) -> Result<(), RigError>;

    /// Attenuazione dell'audio di uscita di JTDX (cursore Pwr), in decimi di dB: 0 = massimo livello.
    fn output_attenuation(&self) -> Result<u32, RigError>;

    fn set_output_attenuation(&self, tenths: u32) -> Result<(), RigError>;

    /// Cambia il modo di JTDX ("FT8" o "FT4").
    fn set_mode(&self, mode: &str) -> Result<(), RigError>;

    /// Stato degli interruttori di JTDX (`controls::CONTROLS`): coppie nome e acceso.
    fn controls(&self) -> Result<Vec<(String, bool)>, RigError> {
        Err(RigError("comando non supportato da questa radio".into()))
    }

    /// Gli interruttori e, se la radio li sa leggere, i sei messaggi Tx con quello selezionato, in una lettura sola.
    fn controls_and_tx(&self) -> Result<controls::Reading, RigError> {
        Ok((self.controls()?, None))
    }

    /// Lo stato di JTDX letto dalla sua finestra: serve subito dopo l'avvio, prima che mandi il suo stato UDP.
    fn engine_snapshot(&self) -> Result<snapshot::Snapshot, RigError> {
        Err(RigError("comando non supportato da questa radio".into()))
    }

    /// Seleziona il messaggio di trasmissione `n` (1-6): il motore trasmette da quello e poi prosegue da solo.
    fn select_tx_message(&self, _n: u8) -> Result<(), RigError> {
        Err(RigError("comando non supportato da questa radio".into()))
    }

    /// Porta un interruttore allo stato voluto (`Some`) o preme un pulsante semplice (`None`).
    fn set_control(&self, _id: &str, _on: Option<bool>) -> Result<(), RigError> {
        Err(RigError("comando non supportato da questa radio".into()))
    }
}
