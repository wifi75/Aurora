use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc, Arc,
    },
    thread::JoinHandle,
    time::Duration,
};

use aurora_core::SpectrumRow;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use tokio::sync::broadcast;

use crate::{AnalyzerError, SpectrumAnalyzer, F_MAX_HZ};

#[derive(Debug)]
pub enum CaptureError {
    /// Nessun ingresso corrisponde: l'elenco aiuta a scegliere il nome giusto.
    NoDevice { wanted: Option<String>, available: Vec<String> },
    Config(String),
    Stream(String),
    Analyzer(AnalyzerError),
}

impl std::fmt::Display for CaptureError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CaptureError::NoDevice { wanted, available } => write!(
                f,
                "ingresso audio non trovato ({}); disponibili: {}",
                wanted.as_deref().unwrap_or("predefinito"),
                if available.is_empty() { "nessuno".to_string() } else { available.join(", ") }
            ),
            CaptureError::Config(e) => write!(f, "configurazione audio non disponibile: {e}"),
            CaptureError::Stream(e) => write!(f, "ingresso audio non aperto (occupato da un'altra applicazione?): {e}"),
            CaptureError::Analyzer(e) => write!(f, "{e}"),
        }
    }
}

impl std::error::Error for CaptureError {}

fn device_name(d: &cpal::Device) -> String {
    d.description().map(|x| x.name().to_string()).unwrap_or_else(|_| "(senza nome)".into())
}

pub fn list_input_devices() -> Vec<String> {
    cpal::default_host().input_devices().map(|it| it.map(|d| device_name(&d)).collect()).unwrap_or_default()
}

/// Cattura in corso; si ferma con `stop` o quando viene lasciata cadere.
pub struct CaptureHandle {
    stop: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
}

impl CaptureHandle {
    pub fn stop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        if let Some(t) = self.thread.take() {
            let _ = t.join();
        }
    }
}

impl Drop for CaptureHandle {
    fn drop(&mut self) {
        self.stop();
    }
}

/// Apre l'ingresso audio (`device_hint` e' una parte del nome, senza distinzione di maiuscole) e
/// pubblica le righe di spettro. Condivide la scheda con altre applicazioni (modalita' condivisa).
pub fn run_capture(
    device_hint: Option<&str>,
    rows: broadcast::Sender<SpectrumRow>,
) -> Result<CaptureHandle, CaptureError> {
    let host = cpal::default_host();
    let device = match device_hint {
        None => host.default_input_device(),
        Some(hint) => host
            .input_devices()
            .ok()
            .and_then(|mut it| it.find(|d| device_name(d).to_lowercase().contains(&hint.to_lowercase()))),
    };
    let Some(device) = device else {
        return Err(CaptureError::NoDevice { wanted: device_hint.map(str::to_string), available: list_input_devices() });
    };
    let name = device_name(&device);
    let supported = device.default_input_config().map_err(|e| CaptureError::Config(format!("{name}: {e}")))?;
    let rate = supported.sample_rate();
    let channels = usize::from(supported.channels()).max(1);
    let format = supported.sample_format();
    let mut analyzer = SpectrumAnalyzer::new(rate).map_err(CaptureError::Analyzer)?;

    let stop = Arc::new(AtomicBool::new(false));
    let stop_thread = Arc::clone(&stop);
    let (ready_tx, ready_rx) = mpsc::channel::<Result<(), CaptureError>>();

    // lo stream non e' `Send` su tutte le piattaforme: vive e muore nel thread che lo crea
    let thread = std::thread::spawn(move || {
        let (chunks_tx, chunks_rx) = mpsc::channel::<Vec<f32>>();
        let on_error = |e: cpal::Error| tracing::warn!("errore del flusso audio: {e}");
        let built = match format {
            cpal::SampleFormat::F32 => {
                let tx = chunks_tx.clone();
                device.build_input_stream(
                    supported.into(),
                    move |data: &[f32], _| {
                        let _ = tx.send(data.iter().step_by(channels).copied().collect());
                    },
                    on_error,
                    None,
                )
            }
            cpal::SampleFormat::I16 => {
                let tx = chunks_tx.clone();
                device.build_input_stream(
                    supported.into(),
                    move |data: &[i16], _| {
                        let _ = tx.send(data.iter().step_by(channels).map(|&s| f32::from(s) / 32768.0).collect());
                    },
                    on_error,
                    None,
                )
            }
            other => {
                let _ = ready_tx.send(Err(CaptureError::Config(format!("{name}: formato campioni non gestito: {other:?}"))));
                return;
            }
        };
        let stream = match built {
            Ok(s) => s,
            Err(e) => {
                let _ = ready_tx.send(Err(CaptureError::Stream(format!("{name}: {e}"))));
                return;
            }
        };
        if let Err(e) = stream.play() {
            let _ = ready_tx.send(Err(CaptureError::Stream(format!("{name}: {e}"))));
            return;
        }
        let _ = ready_tx.send(Ok(()));
        tracing::info!("cascata dall'ingresso audio \"{name}\" a {rate} Hz");

        let mut seq = 0u64;
        while !stop_thread.load(Ordering::SeqCst) {
            match chunks_rx.recv_timeout(Duration::from_millis(100)) {
                Ok(chunk) => {
                    for bins in analyzer.push(&chunk) {
                        seq += 1;
                        let _ = rows.send(SpectrumRow { seq, f_min_hz: 0, f_max_hz: F_MAX_HZ, bins });
                    }
                }
                Err(mpsc::RecvTimeoutError::Timeout) => {}
                Err(mpsc::RecvTimeoutError::Disconnected) => break,
            }
        }
        drop(stream);
    });

    match ready_rx.recv() {
        Ok(Ok(())) => Ok(CaptureHandle { stop, thread: Some(thread) }),
        Ok(Err(e)) => {
            let _ = thread.join();
            Err(e)
        }
        Err(_) => Err(CaptureError::Stream("il thread di cattura si e' chiuso senza rispondere".into())),
    }
}
