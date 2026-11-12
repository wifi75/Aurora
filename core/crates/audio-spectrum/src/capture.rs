use std::{
    sync::{
        atomic::{AtomicBool, AtomicU8, Ordering},
        mpsc, Arc,
    },
    thread::JoinHandle,
    time::Duration,
};

use aurora_core::{AudioLevel, Event, SpectrumRow};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use tokio::sync::broadcast;

use aurora_core::audio_format::{buffer_candidates, channels_in_use, stream_candidates, StreamChoice, StreamRange};

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

/// Il dispositivo con questo identificativo del sistema. Su Linux con PulseAudio o PipeWire e' il nome della sorgente o del sink che
/// mostrano WSJT-X e JS8Call (`alsa_input.usb-…analog-stereo`): chi lo copia da li' (o lo mette in `AURORA_AUDIO_DEVICE`) apre quello giusto.
fn position_by_id(devices: &[cpal::Device], hint: &str) -> Option<usize> {
    let wanted = hint.trim();
    devices.iter().position(|d| d.id().is_ok_and(|id| id.id().eq_ignore_ascii_case(wanted)))
}

pub fn list_input_devices() -> Vec<String> {
    cpal::default_host().input_devices().map(|it| it.map(|d| device_name(&d)).collect()).unwrap_or_default()
}

/// Un ingresso audio del sistema, per la scelta nelle impostazioni.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InputDeviceInfo {
    pub name: String,
    /// I canali con cui Aurora apre l'ingresso (1 = mono, 2 = stereo).
    pub channels: u16,
    /// E' l'ingresso predefinito del sistema.
    pub is_default: bool,
}

/// Gli ingressi del sistema con i loro canali e quale e' quello predefinito.
pub fn list_input_devices_info() -> Vec<InputDeviceInfo> {
    let host = cpal::default_host();
    let default = host.default_input_device().map(|d| device_name(&d));
    let Ok(devices) = host.input_devices() else { return Vec::new() };
    let mut list: Vec<(bool, InputDeviceInfo)> = devices
        .map(|d| {
            let name = device_name(&d);
            // i canali con cui l'ingresso si apre davvero, non il massimo dichiarato (PulseAudio e PipeWire dichiarano fino a 31 canali per ogni scheda)
            let channels = channels_in_use(default_choice(&d).ok().as_ref().map(as_choice), &as_ranges(&declared_ranges(&d)), &INPUT_FORMATS);
            let monitor = d.id().is_ok_and(|id| is_monitor_id(id.id()));
            (monitor, InputDeviceInfo { is_default: default.as_deref() == Some(name.as_str()), name, channels })
        })
        .collect();
    // Linux: le sorgenti «Monitor of …» di PulseAudio e PipeWire ascoltano cio' che esce da un'uscita, non la radio. Restano scegliibili ma in
    // fondo all'elenco, dopo le schede vere: chi prova le voci una alla volta trova prima quella giusta
    list.sort_by_key(|(monitor, _)| *monitor);
    list.into_iter().map(|(_, info)| info).collect()
}

/// L'identificativo e' di una sorgente «monitor» di PulseAudio o PipeWire (`alsa_output.….monitor`): l'ascolto di un'uscita, non un ingresso vero.
fn is_monitor_id(id: &str) -> bool {
    id.trim().to_ascii_lowercase().ends_with(".monitor")
}

/// Il nome vuol dire «il dispositivo predefinito del sistema» (anche come lo scrivevano JTDX e le versioni vecchie di Aurora)?
pub fn is_default_name(hint: &str) -> bool {
    let h = hint.trim().to_lowercase();
    h.is_empty() || ["default", "default input device", "default output device", "predefinito", "predefinita", "predefinito del sistema", "predefinita del sistema", "system default"].contains(&h.as_str())
}

/// Un nome di dispositivo pronto da confrontare: le lettere scritte come `\xf3` (vecchie impostazioni copiate da JTDX) tornano lettere,
/// il numero che Windows mette davanti quando la stessa scheda si ricollega («2- USB Audio CODEC») sparisce, minuscole e spazi semplici.
fn normalized(name: &str) -> String {
    let mut decoded = String::with_capacity(name.len());
    let mut rest = name;
    while let Some(i) = rest.find("\\x") {
        decoded.push_str(&rest[..i]);
        let hex = rest.get(i + 2..i + 4).filter(|h| h.chars().all(|c| c.is_ascii_hexdigit()));
        match hex.and_then(|h| u8::from_str_radix(h, 16).ok()) {
            Some(b) => {
                decoded.push(char::from(b));
                rest = &rest[i + 4..];
            }
            None => {
                decoded.push_str("\\x");
                rest = &rest[i + 2..];
            }
        }
    }
    decoded.push_str(rest);
    let mut out = String::with_capacity(decoded.len());
    let chars: Vec<char> = decoded.to_lowercase().chars().collect();
    let mut i = 0;
    while i < chars.len() {
        out.push(chars[i]);
        if chars[i] == '(' {
            let mut j = i + 1;
            while j < chars.len() && chars[j].is_ascii_digit() {
                j += 1;
            }
            if j > i + 1 && j + 1 < chars.len() && chars[j] == '-' && chars[j + 1] == ' ' {
                i = j + 2;
                continue;
            }
        }
        i += 1;
    }
    out.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// La parte tra parentesi, cioe' la scheda («USB Audio CODEC»), uguale in ogni lingua di Windows.
fn card_of(normalized: &str) -> Option<&str> {
    let start = normalized.find('(')?;
    let end = normalized.rfind(')')?;
    (end > start + 1).then(|| normalized[start + 1..end].trim())
}

/// Sceglie un dispositivo per nome: prima il nome identico, poi quello che lo contiene, poi l'unico con la stessa scheda tra parentesi
/// (lo stesso «USB Audio CODEC» con Windows in un'altra lingua o ricollegato con un altro numero). Serve a non confondere
/// «Microfono (USB)» con «Microfono (USB) 2» quando il nome arriva da un elenco.
pub fn match_device_name(names: &[String], hint: &str) -> Option<usize> {
    // prima il nome scritto esattamente cosi' (anche con il «2- »): con due schede uguali e' l'unico modo di aprire quella giusta
    let exact = hint.trim().to_lowercase();
    if let Some(i) = names.iter().position(|n| n.trim().to_lowercase() == exact) {
        return Some(i);
    }
    let wanted = normalized(hint);
    if wanted.is_empty() {
        return None;
    }
    let all: Vec<String> = names.iter().map(|n| normalized(n)).collect();
    // poi il nome ripulito, ma solo se una scheda sola gli corrisponde: con due «USB Audio Device» non si indovina
    let same_name: Vec<usize> = all.iter().enumerate().filter(|(_, n)| **n == wanted).map(|(i, _)| i).collect();
    if same_name.len() == 1 {
        return Some(same_name[0]);
    }
    if same_name.is_empty()
        && let Some(i) = names.iter().position(|n| n.to_lowercase().contains(&exact)).or_else(|| all.iter().position(|n| n.contains(&wanted)))
    {
        return Some(i);
    }
    let card = card_of(&wanted)?;
    let same: Vec<usize> = all.iter().enumerate().filter(|(_, n)| card_of(n) == Some(card)).map(|(i, _)| i).collect();
    (same.len() == 1).then(|| same[0])
}

#[cfg(test)]
mod device_name_tests {
    use super::*;

    #[test]
    fn old_names_still_find_the_card() {
        let names = vec!["Micrófono (USB Audio Device)".to_string(), "Microphone (2- High Definition Audio Device)".to_string()];
        assert_eq!(match_device_name(&names, "Micr\\xf3fono (USB Audio Device)"), Some(0));
        let names = vec!["Microfono (USB Audio CODEC )".to_string(), "Microfono (Conexant ISST Audio)".to_string()];
        assert_eq!(match_device_name(&names, "Microfono (2- USB Audio CODEC )"), Some(0));
        let names = vec!["Microphone (USB Audio CODEC )".to_string()];
        assert_eq!(match_device_name(&names, "Micrófono (USB Audio CODEC )"), Some(0));
        let names = vec!["Microfono (USB Audio Device)".to_string(), "Altro (USB Audio Device)".to_string()];
        assert_eq!(match_device_name(&names, "Mic (USB Audio Device)"), None, "due schede uguali: non si indovina");
        assert!(is_default_name("Default Input Device"));
        assert!(!is_default_name("Microfono (USB)"));
    }
}

/// Cattura in corso; si ferma con `stop` o quando viene lasciata cadere.
pub struct CaptureHandle {
    stop: Arc<AtomicBool>,
    /// Il dispositivo si e' perso (scollegato, radio spenta): il flusso non da' piu' audio e va riaperto.
    lost: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
    device: String,
}

impl CaptureHandle {
    /// Il dispositivo si e' perso e il flusso va riaperto.
    pub fn is_lost(&self) -> bool {
        self.lost.load(Ordering::SeqCst)
    }

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
///
/// Il formato con cui si apre non e' il massimo che il dispositivo dichiara ma quello con cui lavora (uno o due canali); se non si apre si provano
/// stereo a 48 kHz, stereo a 44,1 kHz e mono (vedi `aurora_core::audio_format`), ognuno prima con un buffer piu' largo del predefinito (regge i
/// rallentamenti del PC senza perdere campioni) e poi con quello del sistema. Ogni tentativo fallito resta nel registro con il suo errore.
pub fn run_capture_with(
    device_hint: Option<&str>,
    channel: Arc<AtomicU8>,
    rows: broadcast::Sender<SpectrumRow>,
    events: broadcast::Sender<Event>,
    tap: Option<mpsc::Sender<AudioChunk>>,
) -> Result<CaptureHandle, CaptureError> {
    let host = cpal::default_host();
    let device = match device_hint.filter(|h| !is_default_name(h)) {
        None => host.default_input_device(),
        Some(hint) => host.input_devices().ok().and_then(|it| {
            let mut devices: Vec<cpal::Device> = it.collect();
            let names: Vec<String> = devices.iter().map(device_name).collect();
            position_by_id(&devices, hint).or_else(|| match_device_name(&names, hint)).map(|i| devices.swap_remove(i))
        }),
    };
    let Some(device) = device else {
        return Err(CaptureError::NoDevice { wanted: device_hint.map(str::to_string), available: list_input_devices() });
    };
    let name = device_name(&device);
    let name_for_handle = name.clone();
    let (plan, default_error) = input_plan(&device);
    if plan.is_empty() {
        let why = default_error.unwrap_or_else(|| "nessun formato utilizzabile (uno o due canali, 44,1 o 48 kHz)".into());
        return Err(CaptureError::Config(format!("{name}: {why}")));
    }

    let stop = Arc::new(AtomicBool::new(false));
    let stop_thread = Arc::clone(&stop);
    let lost = Arc::new(AtomicBool::new(false));
    let lost_thread = Arc::clone(&lost);
    let (ready_tx, ready_rx) = mpsc::channel::<Result<(), CaptureError>>();

    // lo stream non e' `Send` su tutte le piattaforme: vive e muore nel thread che lo crea
    let thread = std::thread::spawn(move || {
        // un flusso che smette di dare audio senza segnalare errori (scheda in pausa, driver bloccato) si da' per perso: il controllo ogni 5 s lo
        // riapre e avvisa, come `soundin.cpp` di WSJT-X tratta Idle/Suspended/Interrupted. Prima la cascata restava ferma in silenzio
        let stalled = Arc::clone(&lost_thread);
        // dettagli per capire il problema dal rapporto: tipo di scheda (non il nome, che puo' dire chi e'), formato e ritmo delle perdite
        let kind = device_kind(&name);
        let mut failures: Vec<String> = Vec::new();
        let mut open: Option<OpenInput> = None;
        'plan: for attempt in &plan {
            let StreamChoice { channels, rate, format } = attempt.choice;
            let (analyzer, meter) = match SpectrumAnalyzer::new(rate) {
                Ok(analyzer) => (analyzer, LevelMeter::new(rate)),
                Err(e) => {
                    failures.push(format!("{rate} Hz: {e}"));
                    continue;
                }
            };
            for frames in buffer_candidates(rate, WIDER_BUFFER_MS, attempt.buffer_limits, true) {
                let config = cpal::StreamConfig { channels, sample_rate: rate, buffer_size: frames.map_or(cpal::BufferSize::Default, cpal::BufferSize::Fixed) };
                let what = describe_format(rate, channels, format, frames);
                let detail = format!("ingresso {kind}, {rate} Hz, {channels} canali, {format:?}");
                let built = build_input(&device, format, config, &channel, input_error_handler(Arc::clone(&lost_thread), detail))
                    .and_then(|(stream, chunks, free)| stream.play().map(|()| (stream, chunks, free)).map_err(|e| e.to_string()));
                match built {
                    Ok((stream, chunks, free)) => {
                        tracing::info!("ingresso audio \"{name}\" aperto: {what}");
                        open = Some(OpenInput { stream, chunks, free, rate, analyzer, meter });
                        break 'plan;
                    }
                    Err(e) => {
                        tracing::info!("ingresso audio \"{name}\": {what} non accettato ({e}), si prova il formato seguente");
                        failures.push(format!("{what}: {e}"));
                    }
                }
            }
        }
        let Some(OpenInput { stream, chunks: chunks_rx, free: free_tx, rate, mut analyzer, mut meter }) = open else {
            let _ = ready_tx.send(Err(CaptureError::Stream(format!("{name}: {}", failures.join("; ")))));
            return;
        };
        let _ = ready_tx.send(Ok(()));

        let mut seq = 0u64;
        let mut last_chunk = std::time::Instant::now();
        while !stop_thread.load(Ordering::SeqCst) {
            match chunks_rx.recv_timeout(Duration::from_millis(100)) {
                Ok(chunk) => {
                    last_chunk = std::time::Instant::now();
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
                    let _ = free_tx.send(chunk);
                }
                Err(mpsc::RecvTimeoutError::Timeout) => {
                    if last_chunk.elapsed() > Duration::from_secs(3) && !stalled.swap(true, Ordering::SeqCst) {
                        tracing::info!("ingresso audio \"{name}\" fermo da 3 s senza errori: lo si riapre");
                    }
                }
                Err(mpsc::RecvTimeoutError::Disconnected) => break,
            }
        }
        drop(stream);
    });

    match ready_rx.recv() {
        Ok(Ok(())) => Ok(CaptureHandle { stop, lost, thread: Some(thread), device: name_for_handle }),
        Ok(Err(e)) => {
            let _ = thread.join();
            Err(e)
        }
        Err(_) => Err(CaptureError::Stream("il thread di cattura si e' chiuso senza rispondere".into())),
    }
}

/// Il buffer della cattura che si chiede alla scheda: circa 40 ms. Il predefinito della modalita' condivisa di Windows (10 ms) basta un attimo a svuotarsi
/// se il PC e' impegnato (blocchi persi, causa `Xrun`); WSJT-X lo allarga in Full Duplex per lo stesso motivo (`RxAudioBufferPolicy.hpp`, 1 s lo' dove serve).
/// Resta piccolo perche' l'ora di arrivo dei blocchi guida i periodi.
const WIDER_BUFFER_MS: u32 = 40;

/// I tipi di campioni che l'ingresso sa leggere, dal preferito.
const INPUT_FORMATS: [cpal::SampleFormat; 4] = [cpal::SampleFormat::F32, cpal::SampleFormat::I16, cpal::SampleFormat::I32, cpal::SampleFormat::I24];

type InputStream = <cpal::Device as DeviceTrait>::Stream;

/// Un formato da provare, con i limiti del buffer che il dispositivo dichiara per quel formato.
struct InputAttempt {
    choice: StreamChoice<cpal::SampleFormat>,
    buffer_limits: Option<(u32, u32)>,
}

/// L'ingresso aperto e cio' che serve per leggerlo.
struct OpenInput {
    stream: InputStream,
    /// I blocchi di audio (mono) che arrivano dalla scheda.
    chunks: mpsc::Receiver<Vec<f32>>,
    /// I buffer usati tornano al thread audio.
    free: mpsc::Sender<Vec<f32>>,
    rate: u32,
    analyzer: SpectrumAnalyzer,
    meter: LevelMeter,
}

fn describe_format(rate: u32, channels: u16, format: cpal::SampleFormat, frames: Option<u32>) -> String {
    let buffer = frames.map_or_else(|| "buffer del sistema".to_string(), |n| format!("buffer {n} campioni"));
    format!("{rate} Hz, {channels} canali, {format:?}, {buffer}")
}

fn buffer_limits(size: &cpal::SupportedBufferSize) -> Option<(u32, u32)> {
    match size {
        cpal::SupportedBufferSize::Range { min, max } => Some((*min, *max)),
        cpal::SupportedBufferSize::Unknown => None,
    }
}

fn default_choice(device: &cpal::Device) -> Result<cpal::SupportedStreamConfig, String> {
    device.default_input_config().map_err(|e| e.to_string())
}

fn declared_ranges(device: &cpal::Device) -> Vec<cpal::SupportedStreamConfigRange> {
    device.supported_input_configs().map(|it| it.collect()).unwrap_or_default()
}

fn as_ranges(ranges: &[cpal::SupportedStreamConfigRange]) -> Vec<StreamRange<cpal::SampleFormat>> {
    ranges.iter().map(|r| StreamRange { channels: r.channels(), min_rate: r.min_sample_rate(), max_rate: r.max_sample_rate(), format: r.sample_format() }).collect()
}

fn as_choice(config: &cpal::SupportedStreamConfig) -> StreamChoice<cpal::SampleFormat> {
    StreamChoice { channels: config.channels(), rate: config.sample_rate(), format: config.sample_format() }
}

/// I formati con cui provare ad aprire l'ingresso, nell'ordine (vedi `aurora_core::audio_format`), e l'errore del formato predefinito se manca.
fn input_plan(device: &cpal::Device) -> (Vec<InputAttempt>, Option<String>) {
    let default = default_choice(device);
    let ranges = declared_ranges(device);
    let candidates = stream_candidates(default.as_ref().ok().map(as_choice), &as_ranges(&ranges), &INPUT_FORMATS);
    let plan = candidates
        .into_iter()
        .map(|choice| {
            let from_default = default.as_ref().ok().filter(|d| as_choice(d) == choice).map(|d| buffer_limits(d.buffer_size()));
            let from_ranges = || {
                ranges
                    .iter()
                    .find(|r| r.channels() == choice.channels && r.sample_format() == choice.format && r.min_sample_rate() <= choice.rate && choice.rate <= r.max_sample_rate())
                    .and_then(|r| buffer_limits(r.buffer_size()))
            };
            InputAttempt { choice, buffer_limits: from_default.unwrap_or_else(from_ranges) }
        })
        .collect();
    (plan, default.err())
}

/// Cosa fare quando il flusso segnala un errore mentre riceve.
fn input_error_handler(lost: Arc<AtomicBool>, detail: String) -> impl FnMut(cpal::Error) + Send + 'static {
    // Un buffer perso ogni tanto e' normale (USB, carico del PC): lascia un buco di pochi ms nella cascata.
    // Si segnala solo la prima volta e poi a potenze di dieci, per non riempire il registro.
    let mut glitches = 0u64;
    let started = std::time::Instant::now();
    move |e: cpal::Error| {
        glitches += 1;
        let n = glitches;
        tracing::debug!("errore del flusso audio: {e}");
        let cause = format!("{:?}", e.kind());
        if matches!(e.kind(), cpal::ErrorKind::DeviceNotAvailable | cpal::ErrorKind::StreamInvalidated | cpal::ErrorKind::HostUnavailable) {
            lost.store(true, Ordering::SeqCst);
        }
        let minutes = started.elapsed().as_secs_f64() / 60.0;
        let per_min = if minutes > 0.0 { n as f64 / minutes } else { 0.0 };
        // tante perdite di fila: si dice sullo schermo cosa controllare (l'avviso non si ripete per due minuti)
        if n >= 20 && per_min >= 5.0 {
            aurora_core::notice::notice("audio_loss", format!("{per_min:.0}"));
        }
        // poche perdite sono normali (USB, PC carico): nel registro come informazione; diventano un avviso (e un rapporto) solo se frequenti
        if is_power_of_ten(n) {
            if n >= 100 {
                tracing::warn!("l'audio ha perso un blocco di campioni ({n} volte in {minutes:.0} min, {per_min:.1} al minuto, causa: {cause}; {detail}): la cascata resta attiva");
            } else {
                tracing::info!("l'audio ha perso un blocco di campioni ({n} volte finora): la cascata resta attiva");
            }
        }
    }
}

/// Manda al resto di Aurora un blocco arrivato dalla scheda: sceglie il canale e riusa un buffer gia' allocato (nel thread audio niente
/// allocazioni a ogni blocco: rallentano e fanno perdere campioni).
fn deliver(data: &[f32], channels: usize, channel: &AtomicU8, free: &mpsc::Receiver<Vec<f32>>, tx: &mpsc::Sender<Vec<f32>>) {
    let mut mono = free.try_recv().unwrap_or_default();
    mono.clear();
    pick_channel(data, channels, InputChannel::load(channel), &mut mono);
    let _ = tx.send(mono);
}

/// Il flusso appena costruito, il canale da cui leggere i blocchi e quello per restituire i buffer usati.
type BuiltInput = (InputStream, mpsc::Receiver<Vec<f32>>, mpsc::Sender<Vec<f32>>);

/// Costruisce il flusso con questo formato (ancora fermo: lo avvia chi chiama) e da' il canale da cui leggere i blocchi e quello per restituire i buffer.
fn build_input(
    device: &cpal::Device,
    format: cpal::SampleFormat,
    config: cpal::StreamConfig,
    channel: &Arc<AtomicU8>,
    on_error: impl FnMut(cpal::Error) + Send + 'static,
) -> Result<BuiltInput, String> {
    let channels = usize::from(config.channels).max(1);
    let (tx, chunks_rx) = mpsc::channel::<Vec<f32>>();
    let (free_tx, free) = mpsc::channel::<Vec<f32>>();
    let channel = Arc::clone(channel);
    let mut floats: Vec<f32> = Vec::new();
    let built = match format {
        cpal::SampleFormat::F32 => device.build_input_stream(config, move |data: &[f32], _| deliver(data, channels, &channel, &free, &tx), on_error, None),
        cpal::SampleFormat::I16 => device.build_input_stream(
            config,
            move |data: &[i16], _| {
                floats.clear();
                floats.extend(data.iter().map(|&s| f32::from(s) / 32768.0));
                deliver(&floats, channels, &channel, &free, &tx);
            },
            on_error,
            None,
        ),
        // 24 e 32 bit: alcune schede in modalita' condivisa di Windows espongono solo questi (WSJT-X chiede 16 bit e converte Qt); prima la
        // scheda non si apriva («formato campioni non gestito»)
        cpal::SampleFormat::I24 => device.build_input_stream(
            config,
            move |data: &[cpal::I24], _| {
                floats.clear();
                floats.extend(data.iter().map(|&s| <f32 as cpal::Sample>::from_sample(s)));
                deliver(&floats, channels, &channel, &free, &tx);
            },
            on_error,
            None,
        ),
        cpal::SampleFormat::I32 => device.build_input_stream(
            config,
            move |data: &[i32], _| {
                floats.clear();
                floats.extend(data.iter().map(|&s| <f32 as cpal::Sample>::from_sample(s)));
                deliver(&floats, channels, &channel, &free, &tx);
            },
            on_error,
            None,
        ),
        other => return Err(format!("formato campioni non gestito: {other:?}")),
    };
    built.map(|stream| (stream, chunks_rx, free_tx)).map_err(|e| e.to_string())
}

fn publish_level(events: &broadcast::Sender<Event>, level: AudioLevel) {
    // senza ascoltatori l'invio fallisce: e' normale
    let _ = events.send(Event::AudioLevel(level));
}

#[cfg(test)]
mod tests {
    use super::{is_monitor_id, is_power_of_ten, match_device_name};

    #[test]
    fn a_monitor_source_is_recognised_by_its_identifier() {
        assert!(is_monitor_id("alsa_output.usb-Burr-Brown_from_TI_USB_Audio_CODEC-00.analog-stereo.monitor"));
        assert!(!is_monitor_id("alsa_input.usb-Burr-Brown_from_TI_USB_Audio_CODEC-00.analog-stereo"));
        assert!(!is_monitor_id("Microfono (USB Audio CODEC )"));
    }

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

/// Il tipo della scheda audio dal nome, senza il nome stesso.
fn device_kind(name: &str) -> &'static str {
    let n = name.to_lowercase();
    if n.contains("usb") || n.contains("codec") {
        "USB"
    } else if n.contains("realtek") || n.contains("high definition") {
        "integrata"
    } else if n.contains("cable") || n.contains("virtual") || n.contains("vac") || n.contains("voicemeeter") {
        "virtuale"
    } else {
        "altra"
    }
}
