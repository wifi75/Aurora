//! Spettro audio per la cascata di Aurora.
//!
//! `SpectrumAnalyzer` e' puro (campioni in ingresso, righe in uscita) e si prova con segnali sintetici;
//! `run_capture` lo alimenta con l'ingresso audio scelto, in parallelo a JTDX.

mod analyzer;
mod capture;
mod endpoint_volume;
mod level;

pub use analyzer::{AnalyzerError, SpectrumAnalyzer, F_MAX_HZ, OUTPUT_BINS};
pub use endpoint_volume::{clamp_unit, InputEndpoint, VolumeError};
pub use level::LevelMeter;
pub use capture::{list_input_devices, run_capture, CaptureError, CaptureHandle};
