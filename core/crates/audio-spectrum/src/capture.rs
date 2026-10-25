use std::{
    sync::{
        atomic::{AtomicBool, AtomicU64, AtomicU8, Ordering},
        mpsc, Arc,
    },
    thread::JoinHandle,
    time::Duration,
};

use aurora_core::{AudioLevel, Event, SpectrumRow};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use tokio::sync::broadcast;

use crate::{channel::pick_channel, AnalyzerError, InputChannel, LevelMeter, SpectrumAnalyzer, F_MAX_HZ};

/// 1, 10, 100, 1000...
fn is_power_of_ten(mut n: u64) -> bool {
    while n >= 10 && n.is_multiple_of(10) {
        n /= 10;
    }
    n == 1
}

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

/// Un ingresso audio del sistema, per la scelta nelle impostazioni.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InputDeviceInfo {
    pub name: String,
    /// Il numero massimo di canali che il dispositivo offre (1 = mono, 2 = stereo).
    pub channels: u16,
    /// E' l'ingresso predefinito del sistema.
    pub is_default: bool,
}

/// Gli ingressi del sistema con i loro canali e quale e' quello predefinito.
pub fn list_input_devices_info() -> Vec<InputDeviceInfo> {
    let host = cpal::default_host();
    let default = host.default_input_device().map(|d| device_name(&d));
    let Ok(devices) = host.input_devices() else { return Vec::new() };
    devices
        .map(|d| {
            let name = device_name(&d);
            let from_ranges = d.supported_input_configs().ok().and_then(|it| it.map(|c| c.channels()).max());
            let channels = from_ranges.or_else(|| d.default_input_config().ok().map(|c| c.channels())).unwrap_or(1);
            InputDeviceInfo { is_default: default.as_deref() == Some(name.as_str()), name, channels }
        })
        .collect()
}

/// Sceglie un dispositivo per nome: prima il nome identico (senza distinzione di maiuscole), poi quello che lo contiene. Serve a non confondere
/// «Microfono (USB)» con «Microfono (USB) 2» quando il nome arriva da un elenco.
pub fn match_device_name(names: &[String], hint: &str) -> Option<usize> {
    let wanted = hint.trim().to_lowercase();
    names.iter().position(|n| n.to_lowercase() == wanted).or_else(|| names.iter().position(|n| n.to_lowercase().contains(&wanted)))
}

/// Cattura in corso; si ferma con `stop` o quando viene lasciata cadere.
pub struct CaptureHandle {
    stop: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
    device: String,
}

impl CaptureHandle {
    /// Il nome del dispositivo aperto.
    pub fn device_name(&self) -> &str {
        &self.device
    }

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

/// Un blocco di audio come arriva dalla scheda (mono, frequenza di campionamento della scheda), con l'ora di arrivo.
/// Serve al motore proprio, che lo porta a 12 kHz e lo divide nei periodi di 15 secondi.
#[derive(Debug, Clone)]
pub struct AudioChunk {
    pub rate: u32,
    pub samples: Vec<f32>,
    /// Millisecondi dal 1970 (UTC) al momento in cui il blocco e' stato ricevuto: e' la fine dei suoi campioni.
    pub received_unix_ms: u64,
}

/// Apre l'ingresso audio (`device_hint` e' una parte del nome, senza distinzione di maiuscole) e
/// pubblica le righe di spettro e, quattro volte al secondo, il livello (`Event::AudioLevel`).
/// Condivide la scheda con altre applicazioni (modalita' condivisa).
pub fn run_capture(
    device_hint: Option<&str>,
    rows: broadcast::Sender<SpectrumRow>,
    events: broadcast::Sender<Event>,
) -> Result<CaptureHandle, CaptureError> {
    run_capture_tapped(device_hint, rows, events, None)
}

/// Come [`run_capture`], e in piu' manda ogni blocco di audio a `tap` (per il decoder proprio).
pub fn run_capture_tapped(
    device_hint: Option<&str>,
    rows: broadcast::Sender<SpectrumRow>,
    events: broadcast::Sender<Event>,
    tap: Option<mpsc::Sender<AudioChunk>>,
) -> Result<CaptureHandle, CaptureError> {
    run_capture_with(device_hint, Arc::new(AtomicU8::new(InputChannel::Mono.code())), rows, events, tap)
}

/// Come [`run_capture_tapped`] con il canale scelto (`channel` si puo' cambiare mentre l'ingresso e' aperto: vale dal blocco dopo).
pub fn run_capture_with(
    device_hint: Option<&str>,
    channel: Arc<AtomicU8>,
    rows: broadcast::Sender<SpectrumRow>,
    events: broadcast::Sender<Event>,
    tap: Option<mpsc::Sender<AudioChunk>>,
) -> Result<CaptureHandle, CaptureError> {
    let host = cpal::default_host();
    let device = match device_hint {
        None => host.default_input_device(),
        Some(hint) => host.input_devices().ok().and_then(|it| {
            let mut devices: Vec<cpal::Device> = it.collect();
            let names: Vec<String> = devices.iter().map(device_name).collect();
            match_device_name(&names, hint).map(|i| devices.swap_remove(i))
        }),
    };
    let Some(device) = device else {
        return Err(CaptureError::NoDevice { wanted: device_hint.map(str::to_string), available: list_input_devices() });
    };
    let name = device_name(&device);
    let name_for_handle = name.clone();
    let supported = device.default_input_config().map_err(|e| CaptureError::Config(format!("{name}: {e}")))?;
    let rate = supported.sample_rate();
    let channels = usize::from(supported.channels()).max(1);
    let format = supported.sample_format();
    let mut analyzer = SpectrumAnalyzer::new(rate).map_err(CaptureError::Analyzer)?;
    let mut meter = LevelMeter::new(rate);

    let stop = Arc::new(AtomicBool::new(false));
    let stop_thread = Arc::clone(&stop);
    let (ready_tx, ready_rx) = mpsc::channel::<Result<(), CaptureError>>();

    // lo stream non e' `Send` su tutte le piattaforme: vive e muore nel thread che lo crea
    let thread = std::thread::spawn(move || {
        let (chunks_tx, chunks_rx) = mpsc::channel::<Vec<f32>>();
        // Un buffer perso ogni tanto e' normale (USB, carico del PC): lascia un buco di pochi ms nella cascata.
        // Si segnala solo la prima volta e poi a potenze di dieci, per non riempire il registro.
        let glitches = Arc::new(AtomicU64::new(0));
        let on_error = move |e: cpal::Error| {
            let n = glitches.fetch_add(1, Ordering::Relaxed) + 1;
            tracing::debug!("errore del flusso audio: {e}");
            if is_power_of_ten(n) {
                tracing::warn!("l'audio ha perso un blocco di campioni ({n} volte finora): la cascata resta attiva");
            }
        };
        let built = match format {
            cpal::SampleFormat::F32 => {
                let tx = chunks_tx.clone();
                let channel = Arc::clone(&channel);
                device.build_input_stream(
                    supported.into(),
                    move |data: &[f32], _| {
                        let mut mono = Vec::new();
                        pick_channel(data, channels, InputChannel::load(&channel), &mut mono);
                        let _ = tx.send(mono);
                    },
                    on_error,
                    None,
                )
            }
            cpal::SampleFormat::I16 => {
                let tx = chunks_tx.clone();
                let channel = Arc::clone(&channel);
                device.build_input_stream(
                    supported.into(),
                    move |data: &[i16], _| {
                        let floats: Vec<f32> = data.iter().map(|&s| f32::from(s) / 32768.0).collect();
                        let mut mono = Vec::new();
                        pick_channel(&floats, channels, InputChannel::load(&channel), &mut mono);
                        let _ = tx.send(mono);
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
                    if let Some(tap) = &tap {
                        let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_millis() as u64);
                        let _ = tap.send(AudioChunk { rate, samples: chunk.clone(), received_unix_ms: now });
                    }
                    for level in meter.push(&chunk) {
                        publish_level(&events, level);
                    }
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
        Ok(Ok(())) => Ok(CaptureHandle { stop, thread: Some(thread), device: name_for_handle }),
        Ok(Err(e)) => {
            let _ = thread.join();
            Err(e)
        }
        Err(_) => Err(CaptureError::Stream("il thread di cattura si e' chiuso senza rispondere".into())),
    }
}

fn publish_level(events: &broadcast::Sender<Event>, level: AudioLevel) {
    // senza ascoltatori l'invio fallisce: e' normale
    let _ = events.send(Event::AudioLevel(level));
}

#[cfg(test)]
mod tests {
    use super::{is_power_of_ten, match_device_name};

    #[test]
    fn an_exact_name_wins_over_a_name_that_merely_contains_it() {
        let names: Vec<String> = ["Microfono (2- USB Audio Device)", "Microfono (2- USB Audio Device) bis", "Line In"].iter().map(|s| s.to_string()).collect();
        assert_eq!(match_device_name(&names, "microfono (2- usb audio device) bis"), Some(1));
        assert_eq!(match_device_name(&names, "Microfono (2- USB Audio Device)"), Some(0));
        assert_eq!(match_device_name(&names, "usb audio"), Some(0), "una parte del nome basta, come con AURORA_AUDIO_DEVICE");
        assert_eq!(match_device_name(&names, "Altro"), None);
    }

    #[test]
    fn warns_only_at_powers_of_ten() {
        let hits: Vec<u64> = (1..=1000).filter(|n| is_power_of_ten(*n)).collect();
        assert_eq!(hits, vec![1, 10, 100, 1000]);
        assert!(!is_power_of_ten(0) && !is_power_of_ten(20) && !is_power_of_ten(101));
    }
}
