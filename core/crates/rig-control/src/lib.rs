//! Comando della radio condiviso con JTDX: JTDX usa OmniRig, che accetta piu' client insieme,
//! quindi Aurora puo' cambiare frequenza senza togliere la radio a JTDX.

pub mod bands;
pub mod modes;
pub mod omnirig;

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

    /// Cambia il modo di JTDX ("FT8" o "FT4").
    fn set_mode(&self, mode: &str) -> Result<(), RigError>;
}
