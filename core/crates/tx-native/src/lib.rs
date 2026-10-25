//! Catena di trasmissione del motore proprio di Aurora.
//!
//! - [`audio`]: uscita audio (`AudioSink`, `CpalSink`), elenco dei dispositivi;
//! - [`resample`] e [`level`]: dai 12 kHz dell'onda alla frequenza del dispositivo, attenuazione in decimi di dB, rampe;
//! - [`ptt`]: il comando di trasmissione (`Ptt`: rigctld, OmniRig, nessuno per il VOX) e la guardia che lo spegne sempre;
//! - [`watchdog`]: controllo a tempo vero che spegne un PTT rimasto acceso;
//! - [`slot`] e [`scheduler`]: periodi UTC di FT8/FT4 e pianificatore (`Transmitter`) con `halt` e stato osservabile;
//! - [`clock`] e [`testing`]: orologio iniettabile e doppioni per le prove.
//!
//! Il crate non conosce l'encoder dei messaggi: riceve l'onda a 12 kHz (per esempio da `ft8::modem::encode_message`).

pub mod audio;
pub mod clock;
pub mod error;
pub mod level;
pub mod ptt;
pub mod ptt_serial;
pub mod resample;
pub mod scheduler;
pub mod slot;
pub mod testing;
pub mod watchdog;

pub use audio::{
    AudioSink, CpalSink, OutputChannel, OutputDeviceInfo, SinkSwitch, SwitchOutcome, SwitchableSink, list_output_devices, list_output_devices_info, match_device_name,
};
pub use clock::{Clock, ManualClock, SystemClock};
pub use error::TxError;
pub use ptt::{HamlibPtt, NoPtt, OmniRigPtt, Ptt, PttGuard};
pub use ptt_serial::{Line, SerialLinePtt};
pub use scheduler::{Transmitter, TxConfig, TxOutcome, TxPlan, TxRequest, TxState};
pub use slot::{Mode, Parity};

/// Frequenza di campionamento dell'onda in ingresso.
pub const WAVE_RATE: u32 = 12_000;

/// Blocca un mutex anche se un altro thread e' andato in panico tenendolo: lo stato resta utilizzabile (serve a spegnere il PTT).
pub(crate) fn lock<T>(m: &std::sync::Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|p| p.into_inner())
}
