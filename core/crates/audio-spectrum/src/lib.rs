//! Spettro audio per la cascata di Aurora.
//!
//! `SpectrumAnalyzer` e' puro (campioni in ingresso, righe in uscita) e si prova con segnali sintetici;
//! `run_capture` lo alimenta con l'ingresso audio scelto, in parallelo a JTDX. `ClickDetector` trova i click di manipolazione (per
//! l'anti-click della cascata e, piu' avanti, per i canali CW di tutta la banda).

mod analyzer;
mod capture;
mod click;
mod channel;
mod endpoint_volume;
mod level;
mod tci_source;

pub use analyzer::{AnalyzerError, SpectrumAnalyzer, F_MAX_HZ, OUTPUT_BINS};
pub use click::{ClickDetector, ClickSpan};
pub use endpoint_volume::{clamp_unit, InputEndpoint, OutputEndpoint, VolumeError};
pub use level::LevelMeter;
pub use capture::{list_input_devices, list_input_devices_info, is_default_name, match_device_name, run_capture, run_capture_tapped, run_capture_with, AudioChunk, CaptureError, CaptureHandle, InputDeviceInfo};
pub use channel::{pick_channel, InputChannel};
pub use tci_source::{run_tci_feed, TciFeedHandle, TciRxSink, TCI_INPUT_NAME};
