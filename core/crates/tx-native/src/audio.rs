//! Uscita audio. [`AudioSink`] e' il confine verso il dispositivo: [`CpalSink`] e' quello vero (WASAPI e simili via `cpal`),
//! le prove usano una versione che registra i campioni (`testing`).
//!
//! Il flusso del dispositivo resta aperto e suona silenzio finche' non arriva un'onda: cosi' l'inizio non paga l'apertura
//! del dispositivo. L'onda arriva gia' alla frequenza del dispositivo, con livello e rampe (vedi `level::prepare_samples`).

use std::{
    f64::consts::PI,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, AtomicU8, AtomicU32, AtomicU64, Ordering},
        mpsc,
    },
    thread::JoinHandle,
    time::Duration,
};

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};

use crate::{error::TxError, lock};

/// Lo stadio finale della catena: suona campioni mono alla frequenza del dispositivo.
pub trait AudioSink: Send {
    /// Frequenza di campionamento attesa per i campioni di `start`.
    fn sample_rate(&self) -> u32;
    /// Comincia subito a suonare (senza bloccare). Errore se il dispositivo non e' piu' utilizzabile.
    fn start(&mut self, samples: Vec<f32>) -> Result<(), TxError>;
    /// Vero finche' l'onda non e' stata consegnata tutta al dispositivo.
    fn is_playing(&self) -> bool;
    /// Silenzio subito (con una discesa di pochi millisecondi, senza click).
    fn stop(&mut self);
    /// Il guadagno lineare (bit di un `f32`) che il dispositivo applica mentre suona: se c'e', il livello (cursore Pwr) segue anche durante la
    /// trasmissione, come in JTDX; altrimenti l'onda arriva gia' con il livello applicato.
    fn live_gain(&self) -> Option<Arc<AtomicU32>> {
        None
    }
}

/// Il canale d'uscita come nella scheda Audio di JTDX (`AudioOutputChannel`). In JTDX (`Modulator::readData`, `SoundOutput::setFormat`):
/// Mono apre il dispositivo con un solo canale; Sinistro scrive l'onda nel primo canale e silenzio nel secondo; Destro il contrario; Entrambi
/// la stessa onda nei due. Qui il dispositivo si apre con i suoi canali: Mono e Entrambi riempiono tutti i canali, Sinistro e Destro solo il
/// primo o il secondo, gli altri restano muti.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum OutputChannel {
    #[default]
    Mono,
    Left,
    Right,
    Both,
}

impl OutputChannel {
    pub fn parse(text: &str) -> Option<OutputChannel> {
        match text.trim().to_ascii_lowercase().as_str() {
            "mono" => Some(OutputChannel::Mono),
            "left" => Some(OutputChannel::Left),
            "right" => Some(OutputChannel::Right),
            "both" => Some(OutputChannel::Both),
            _ => None,
        }
    }

    /// Il nome nel `JTDX.ini`.
    pub fn name(self) -> &'static str {
        match self {
            OutputChannel::Mono => "Mono",
            OutputChannel::Left => "Left",
            OutputChannel::Right => "Right",
            OutputChannel::Both => "Both",
        }
    }

    pub fn code(self) -> u8 {
        match self {
            OutputChannel::Mono => 0,
            OutputChannel::Left => 1,
            OutputChannel::Right => 2,
            OutputChannel::Both => 3,
        }
    }

    pub fn from_code(code: u8) -> OutputChannel {
        match code {
            1 => OutputChannel::Left,
            2 => OutputChannel::Right,
            3 => OutputChannel::Both,
            _ => OutputChannel::Mono,
        }
    }

    pub fn load(cell: &AtomicU8) -> OutputChannel {
        OutputChannel::from_code(cell.load(Ordering::Relaxed))
    }

    /// Il canale `index` (da 0) di un dispositivo con `channels` canali suona l'onda?
    pub fn plays_on(self, index: usize, channels: usize) -> bool {
        match self {
            OutputChannel::Mono | OutputChannel::Both => true,
            OutputChannel::Left => index == 0,
            // un dispositivo mono ha un solo canale: Destro non puo' tacere l'unico che c'e'
            OutputChannel::Right => index == usize::from(channels > 1),
        }
    }
}

/// Applica il guadagno ai campioni passando in modo lineare da `from` a `to` lungo il blocco (niente scalini, niente click) e limita a +-1.
pub fn apply_gain_ramp(buf: &mut [f32], from: f32, to: f32) {
    if buf.is_empty() {
        return;
    }
    if (from - 1.0).abs() < f32::EPSILON && (to - 1.0).abs() < f32::EPSILON {
        return;
    }
    let steps = buf.len() as f32;
    for (i, s) in buf.iter_mut().enumerate() {
        let g = from + (to - from) * ((i + 1) as f32 / steps);
        *s = (*s * g).clamp(-1.0, 1.0);
    }
}

/// Lo stato della riproduzione, indipendente dal dispositivo (e quindi provabile).
#[derive(Debug, Default)]
pub struct Playback {
    samples: Vec<f32>,
    pos: usize,
    playing: bool,
    /// Discesa in corso dopo uno `stop`: (campioni totali, fatti).
    fade: Option<(usize, usize)>,
}

impl Playback {
    pub fn new() -> Playback {
        Playback::default()
    }

    pub fn start(&mut self, samples: Vec<f32>) {
        self.playing = !samples.is_empty();
        self.samples = samples;
        self.pos = 0;
        self.fade = None;
    }

    pub fn is_playing(&self) -> bool {
        self.playing
    }

    /// Ferma con una discesa a coseno di `fade_samples` campioni dal punto attuale.
    pub fn begin_stop(&mut self, fade_samples: usize) {
        if !self.playing {
            return;
        }
        if fade_samples == 0 {
            self.playing = false;
        } else if self.fade.is_none() {
            self.fade = Some((fade_samples, 0));
        }
    }

    /// Riempie `out` (mono): campioni dell'onda, poi silenzio.
    pub fn fill(&mut self, out: &mut [f32]) {
        for o in out.iter_mut() {
            if !self.playing {
                *o = 0.0;
                continue;
            }
            let mut s = self.samples[self.pos];
            self.pos += 1;
            if let Some((total, done)) = &mut self.fade {
                s *= (0.5 * (1.0 + (PI * *done as f64 / *total as f64).cos())) as f32;
                *done += 1;
                if *done >= *total {
                    self.playing = false;
                }
            }
            if self.pos >= self.samples.len() {
                self.playing = false;
            }
            *o = s;
        }
    }
}

fn device_name(d: &cpal::Device) -> String {
    d.description().map(|x| x.name().to_string()).unwrap_or_else(|_| "(senza nome)".into())
}

/// I nomi dei dispositivi di uscita.
pub fn list_output_devices() -> Vec<String> {
    cpal::default_host().output_devices().map(|it| it.map(|d| device_name(&d)).collect()).unwrap_or_default()
}

/// Un'uscita audio del sistema, per la scelta nelle impostazioni.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OutputDeviceInfo {
    pub name: String,
    /// Il numero massimo di canali (1 = mono, 2 = stereo).
    pub channels: u16,
    /// E' l'uscita predefinita del sistema.
    pub is_default: bool,
}

/// Le uscite del sistema con i loro canali e quale e' quella predefinita.
pub fn list_output_devices_info() -> Vec<OutputDeviceInfo> {
    let host = cpal::default_host();
    let default = host.default_output_device().map(|d| device_name(&d));
    let Ok(devices) = host.output_devices() else { return Vec::new() };
    devices
        .map(|d| {
            let name = device_name(&d);
            let from_ranges = d.supported_output_configs().ok().and_then(|it| it.map(|c| c.channels()).max());
            let channels = from_ranges.or_else(|| d.default_output_config().ok().map(|c| c.channels())).unwrap_or(1);
            OutputDeviceInfo { is_default: default.as_deref() == Some(name.as_str()), name, channels }
        })
        .collect()
}

/// Sceglie un dispositivo per nome: prima il nome identico (senza distinzione di maiuscole), poi quello che lo contiene.
pub fn match_device_name(names: &[String], hint: &str) -> Option<usize> {
    let wanted = hint.trim().to_lowercase();
    names.iter().position(|n| n.to_lowercase() == wanted).or_else(|| names.iter().position(|n| n.to_lowercase().contains(&wanted)))
}

struct SinkShared {
    playback: Mutex<Playback>,
    /// Il flusso ha dato errore (dispositivo scollegato...): non si suona piu'.
    failed: AtomicBool,
    errors: AtomicU64,
    /// Guadagno lineare applicato mentre suona (bit di un `f32`), regolabile anche a trasmissione in corso.
    gain: Arc<AtomicU32>,
    /// Il canale d'uscita (`OutputChannel::code`), cambiabile anche a flusso aperto.
    channel: Arc<AtomicU8>,
}

/// Uscita sul dispositivo vero. Il flusso vive in un thread proprio (su alcune piattaforme non e' `Send`).
pub struct CpalSink {
    name: String,
    rate: u32,
    shared: Arc<SinkShared>,
    stop_thread: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
}

/// Discesa su `stop`.
const STOP_FADE: Duration = Duration::from_millis(5);

fn render<T: Copy>(shared: &SinkShared, scratch: &mut Vec<f32>, prev_gain: &mut f32, channels: usize, data: &mut [T], conv: impl Fn(f32) -> T) {
    let frames = data.len() / channels;
    scratch.clear();
    scratch.resize(frames, 0.0);
    // se `start` tiene il blocco proprio in questo istante si suona un attimo di silenzio, mai un'attesa
    if let Ok(mut playback) = shared.playback.try_lock() {
        playback.fill(scratch);
    }
    let target = f32::from_bits(shared.gain.load(Ordering::Relaxed));
    apply_gain_ramp(scratch, *prev_gain, target);
    *prev_gain = target;
    let channel = OutputChannel::load(&shared.channel);
    let silence = conv(0.0);
    for (frame, &s) in data.chunks_mut(channels).zip(scratch.iter()) {
        for (index, o) in frame.iter_mut().enumerate() {
            *o = if channel.plays_on(index, channels) { conv(s) } else { silence };
        }
    }
}

impl CpalSink {
    /// Apre l'uscita (`device_hint` e' una parte del nome, senza distinzione di maiuscole; `None` = predefinita).
    pub fn open(device_hint: Option<&str>) -> Result<CpalSink, TxError> {
        CpalSink::open_with(device_hint, Arc::new(AtomicU32::new(1.0f32.to_bits())), Arc::new(AtomicU8::new(OutputChannel::Mono.code())))
    }

    /// Come `open`, con il guadagno e il canale che si tengono fuori dal dispositivo: cosi' sopravvivono al cambio di uscita.
    pub fn open_with(device_hint: Option<&str>, gain: Arc<AtomicU32>, channel: Arc<AtomicU8>) -> Result<CpalSink, TxError> {
        let host = cpal::default_host();
        let device = match device_hint {
            None => host.default_output_device(),
            Some(hint) => host.output_devices().ok().and_then(|it| {
                let mut devices: Vec<cpal::Device> = it.collect();
                let names: Vec<String> = devices.iter().map(device_name).collect();
                match_device_name(&names, hint).map(|i| devices.swap_remove(i))
            }),
        };
        let Some(device) = device else {
            let available = list_output_devices();
            return Err(TxError::Audio(format!(
                "uscita audio non trovata ({}); disponibili: {}",
                device_hint.unwrap_or("predefinita"),
                if available.is_empty() { "nessuna".to_string() } else { available.join(", ") }
            )));
        };
        let name = device_name(&device);
        let supported = device.default_output_config().map_err(|e| TxError::Audio(format!("{name}: configurazione non disponibile: {e}")))?;
        let rate = supported.sample_rate();
        let channels = usize::from(supported.channels()).max(1);
        let format = supported.sample_format();

        let shared = Arc::new(SinkShared { playback: Mutex::new(Playback::new()), failed: AtomicBool::new(false), errors: AtomicU64::new(0), gain, channel });
        let stop_thread = Arc::new(AtomicBool::new(false));
        let (ready_tx, ready_rx) = mpsc::channel::<Result<(), TxError>>();
        let thread_shared = Arc::clone(&shared);
        let thread_stop = Arc::clone(&stop_thread);
        let thread_name = name.clone();

        let thread = std::thread::Builder::new()
            .name("aurora-tx-audio".into())
            .spawn(move || {
                let err_shared = Arc::clone(&thread_shared);
                let on_error = move |e: cpal::Error| {
                    err_shared.errors.fetch_add(1, Ordering::Relaxed);
                    err_shared.failed.store(true, Ordering::SeqCst);
                    tracing::warn!("errore del flusso di uscita audio: {e}");
                };
                let mut scratch: Vec<f32> = Vec::new();
                let mut prev_gain = 1.0f32;
                let cb_shared = Arc::clone(&thread_shared);
                let config: cpal::StreamConfig = supported.into();
                let built = match format {
                    cpal::SampleFormat::F32 => {
                        device.build_output_stream(config, move |data: &mut [f32], _| render(&cb_shared, &mut scratch, &mut prev_gain, channels, data, |s| s), on_error, None)
                    }
                    cpal::SampleFormat::I16 => device.build_output_stream(
                        config,
                        move |data: &mut [i16], _| render(&cb_shared, &mut scratch, &mut prev_gain, channels, data, |s| (s.clamp(-1.0, 1.0) * 32767.0) as i16),
                        on_error,
                        None,
                    ),
                    cpal::SampleFormat::I32 => device.build_output_stream(
                        config,
                        move |data: &mut [i32], _| render(&cb_shared, &mut scratch, &mut prev_gain, channels, data, |s| (f64::from(s.clamp(-1.0, 1.0)) * 2_147_483_647.0) as i32),
                        on_error,
                        None,
                    ),
                    cpal::SampleFormat::U16 => device.build_output_stream(
                        config,
                        move |data: &mut [u16], _| render(&cb_shared, &mut scratch, &mut prev_gain, channels, data, |s| (s.clamp(-1.0, 1.0) * 32767.0 + 32768.0) as u16),
                        on_error,
                        None,
                    ),
                    other => {
                        let _ = ready_tx.send(Err(TxError::Audio(format!("{thread_name}: formato campioni non gestito: {other:?}"))));
                        return;
                    }
                };
                let stream = match built {
                    Ok(s) => s,
                    Err(e) => {
                        let _ = ready_tx.send(Err(TxError::Audio(format!("{thread_name}: uscita non aperta (occupata da un'altra applicazione?): {e}"))));
                        return;
                    }
                };
                if let Err(e) = stream.play() {
                    let _ = ready_tx.send(Err(TxError::Audio(format!("{thread_name}: {e}"))));
                    return;
                }
                let _ = ready_tx.send(Ok(()));
                tracing::info!("uscita audio \"{thread_name}\" a {rate} Hz");
                while !thread_stop.load(Ordering::SeqCst) {
                    std::thread::sleep(Duration::from_millis(50));
                }
                drop(stream);
            })
            .map_err(|e| TxError::Thread(e.to_string()))?;

        match ready_rx.recv() {
            Ok(Ok(())) => Ok(CpalSink { name, rate, shared, stop_thread, thread: Some(thread) }),
            Ok(Err(e)) => {
                let _ = thread.join();
                Err(e)
            }
            Err(_) => Err(TxError::Audio("il thread dell'uscita audio si e' chiuso senza rispondere".into())),
        }
    }

    pub fn device_name(&self) -> &str {
        &self.name
    }
}

impl AudioSink for CpalSink {
    fn sample_rate(&self) -> u32 {
        self.rate
    }

    fn start(&mut self, samples: Vec<f32>) -> Result<(), TxError> {
        if self.shared.failed.load(Ordering::SeqCst) {
            return Err(TxError::Audio(format!("{}: il dispositivo non risponde piu'", self.name)));
        }
        lock(&self.shared.playback).start(samples);
        Ok(())
    }

    fn is_playing(&self) -> bool {
        !self.shared.failed.load(Ordering::SeqCst) && lock(&self.shared.playback).is_playing()
    }

    fn stop(&mut self) {
        let n = (STOP_FADE.as_secs_f64() * f64::from(self.rate)).round() as usize;
        lock(&self.shared.playback).begin_stop(n);
    }

    fn live_gain(&self) -> Option<Arc<AtomicU32>> {
        Some(Arc::clone(&self.shared.gain))
    }
}

impl Drop for CpalSink {
    fn drop(&mut self) {
        self.stop_thread.store(true, Ordering::SeqCst);
        if let Some(t) = self.thread.take() {
            let _ = t.join();
        }
    }
}

/// Com'e' andato un cambio di uscita.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SwitchOutcome {
    /// La nuova uscita e' gia' quella in uso.
    Applied,
    /// C'e' una trasmissione in attesa o in corso: la nuova uscita vale dalla prossima, quella in corso non si tocca.
    Deferred,
}

struct SwitchState {
    current: Box<dyn AudioSink>,
    pending: Option<Box<dyn AudioSink>>,
    /// Qualcuno ha chiesto la frequenza dell'uscita e non ha ancora cominciato a suonare: la trasmissione sta preparando i campioni per
    /// quella frequenza, quindi l'uscita non si cambia (i campioni arriverebbero a una frequenza sbagliata).
    armed: bool,
}

impl SwitchState {
    fn swap_if_possible(&mut self) -> bool {
        if self.pending.is_some()
            && !self.armed
            && !self.current.is_playing()
            && let Some(next) = self.pending.take()
        {
            self.current = next;
            return true;
        }
        false
    }
}

/// Un'uscita che si puo' cambiare mentre Aurora e' accesa (scheda Audio): `SinkSwitch` ne porta una nuova gia' aperta e lei la mette al posto
/// della vecchia **tra una trasmissione e l'altra**. Il guadagno resta quello condiviso (`live_gain`).
pub struct SwitchableSink {
    shared: Arc<Mutex<SwitchState>>,
    gain: Arc<AtomicU32>,
}

/// Il comando per cambiare l'uscita di un [`SwitchableSink`].
#[derive(Clone)]
pub struct SinkSwitch {
    shared: Arc<Mutex<SwitchState>>,
}

impl SwitchableSink {
    pub fn new(first: Box<dyn AudioSink>, gain: Arc<AtomicU32>) -> (SwitchableSink, SinkSwitch) {
        let shared = Arc::new(Mutex::new(SwitchState { current: first, pending: None, armed: false }));
        (SwitchableSink { shared: Arc::clone(&shared), gain }, SinkSwitch { shared })
    }
}

impl SinkSwitch {
    /// Mette `next` al posto dell'uscita attuale: subito se non si sta trasmettendo, altrimenti dalla prossima trasmissione. Una richiesta
    /// ancora in attesa viene sostituita (la vecchia si chiude).
    pub fn request(&self, next: Box<dyn AudioSink>) -> SwitchOutcome {
        let mut state = lock(&self.shared);
        state.pending = Some(next);
        if state.swap_if_possible() { SwitchOutcome::Applied } else { SwitchOutcome::Deferred }
    }

    /// C'e' una nuova uscita che aspetta la fine della trasmissione?
    pub fn has_pending(&self) -> bool {
        lock(&self.shared).pending.is_some()
    }
}

impl AudioSink for SwitchableSink {
    fn sample_rate(&self) -> u32 {
        let mut state = lock(&self.shared);
        state.swap_if_possible();
        state.armed = true;
        state.current.sample_rate()
    }

    fn start(&mut self, samples: Vec<f32>) -> Result<(), TxError> {
        let mut state = lock(&self.shared);
        state.armed = false;
        state.current.start(samples)
    }

    fn is_playing(&self) -> bool {
        lock(&self.shared).current.is_playing()
    }

    fn stop(&mut self) {
        let mut state = lock(&self.shared);
        state.armed = false;
        state.current.stop();
    }

    fn live_gain(&self) -> Option<Arc<AtomicU32>> {
        Some(Arc::clone(&self.gain))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Un'uscita finta con una frequenza e un nome, per provare il cambio.
    struct FakeSink {
        rate: u32,
        playing: Arc<AtomicBool>,
        started: Arc<Mutex<Vec<usize>>>,
    }

    impl AudioSink for FakeSink {
        fn sample_rate(&self) -> u32 {
            self.rate
        }
        fn start(&mut self, samples: Vec<f32>) -> Result<(), TxError> {
            lock(&self.started).push(samples.len());
            self.playing.store(true, Ordering::SeqCst);
            Ok(())
        }
        fn is_playing(&self) -> bool {
            self.playing.load(Ordering::SeqCst)
        }
        fn stop(&mut self) {
            self.playing.store(false, Ordering::SeqCst);
        }
    }

    fn fake(rate: u32) -> (Box<dyn AudioSink>, Arc<AtomicBool>, Arc<Mutex<Vec<usize>>>) {
        let (playing, started) = (Arc::new(AtomicBool::new(false)), Arc::new(Mutex::new(Vec::new())));
        (Box::new(FakeSink { rate, playing: Arc::clone(&playing), started: Arc::clone(&started) }), playing, started)
    }

    #[test]
    fn an_idle_sink_is_replaced_at_once_and_the_new_rate_is_used() {
        let (first, _, first_started) = fake(48_000);
        let (mut sink, switch) = SwitchableSink::new(first, Arc::new(AtomicU32::new(1.0f32.to_bits())));
        let (second, _, second_started) = fake(44_100);
        assert_eq!(switch.request(second), SwitchOutcome::Applied);
        assert!(!switch.has_pending());
        assert_eq!(sink.sample_rate(), 44_100);
        sink.start(vec![0.0; 10]).expect("avvio");
        assert!(lock(&first_started).is_empty());
        assert_eq!(*lock(&second_started), vec![10]);
    }

    #[test]
    fn a_change_during_a_transmission_waits_for_the_next_one_and_never_cuts_the_audio() {
        let (first, first_playing, first_started) = fake(48_000);
        let (mut sink, switch) = SwitchableSink::new(first, Arc::new(AtomicU32::new(1.0f32.to_bits())));
        // la trasmissione ha chiesto la frequenza e sta per suonare: l'uscita non si cambia (i campioni sono a 48 kHz)
        assert_eq!(sink.sample_rate(), 48_000);
        let (second, _, second_started) = fake(44_100);
        assert_eq!(switch.request(second), SwitchOutcome::Deferred);
        assert!(switch.has_pending());
        sink.start(vec![0.0; 20]).expect("avvio");
        assert_eq!(*lock(&first_started), vec![20], "la trasmissione finisce sul dispositivo di prima");
        assert!(sink.is_playing());
        // ancora in corso: la nuova uscita aspetta
        assert!(switch.has_pending());
        first_playing.store(false, Ordering::SeqCst);
        // finita: la prossima trasmissione trova la nuova uscita, con la sua frequenza
        assert_eq!(sink.sample_rate(), 44_100);
        assert!(!switch.has_pending());
        sink.start(vec![0.0; 5]).expect("avvio");
        assert_eq!(*lock(&second_started), vec![5]);
    }

    #[test]
    fn a_second_request_replaces_the_one_still_waiting() {
        let (first, _, _) = fake(48_000);
        let (sink, switch) = SwitchableSink::new(first, Arc::new(AtomicU32::new(1.0f32.to_bits())));
        assert_eq!(sink.sample_rate(), 48_000);
        let (a, _, _) = fake(8_000);
        let (b, _, _) = fake(16_000);
        switch.request(a);
        switch.request(b);
        let mut sink = sink;
        sink.stop();
        assert_eq!(sink.sample_rate(), 16_000, "vale l'ultima richiesta");
    }

    fn drain(pb: &mut Playback, total: usize, block: usize) -> Vec<f32> {
        let mut all = Vec::new();
        while all.len() < total {
            let mut buf = vec![9.0f32; block.min(total - all.len())];
            pb.fill(&mut buf);
            all.extend(buf);
        }
        all
    }

    #[test]
    fn the_gain_ramps_without_steps_and_never_clips() {
        let mut buf = vec![1.0f32; 4];
        apply_gain_ramp(&mut buf, 1.0, 0.5);
        assert!((buf[0] - 0.875).abs() < 1e-6 && (buf[3] - 0.5).abs() < 1e-6, "{buf:?}");
        let mut loud = vec![0.9f32; 3];
        apply_gain_ramp(&mut loud, 2.0, 2.0);
        assert!(loud.iter().all(|s| (*s - 1.0).abs() < 1e-6), "limitato a 1: {loud:?}");
        let mut same = vec![0.3f32; 3];
        apply_gain_ramp(&mut same, 1.0, 1.0);
        assert_eq!(same, vec![0.3f32; 3]);
    }

    #[test]
    fn the_output_channel_decides_where_the_wave_goes() {
        // (canale, indice, canali) -> suona?
        for c in [OutputChannel::Mono, OutputChannel::Both] {
            assert!(c.plays_on(0, 2) && c.plays_on(1, 2), "{c:?}: tutti i canali");
        }
        assert!(OutputChannel::Left.plays_on(0, 2) && !OutputChannel::Left.plays_on(1, 2));
        assert!(!OutputChannel::Right.plays_on(0, 2) && OutputChannel::Right.plays_on(1, 2));
        assert!(OutputChannel::Right.plays_on(0, 1), "su un dispositivo mono l'unico canale suona");
        assert!(!OutputChannel::Left.plays_on(2, 4) && !OutputChannel::Right.plays_on(3, 4), "gli altri canali restano muti");
    }

    fn rendered(channel: OutputChannel, channels: usize) -> Vec<f32> {
        let shared = SinkShared {
            playback: Mutex::new(Playback::new()),
            failed: AtomicBool::new(false),
            errors: AtomicU64::new(0),
            gain: Arc::new(AtomicU32::new(1.0f32.to_bits())),
            channel: Arc::new(AtomicU8::new(channel.code())),
        };
        lock(&shared.playback).start(vec![0.5, -0.5, 0.25]);
        let mut data = vec![9.0f32; 3 * channels];
        render(&shared, &mut Vec::new(), &mut 1.0, channels, &mut data, |s| s);
        data
    }

    #[test]
    fn the_sink_writes_the_wave_on_the_chosen_channel_and_silence_on_the_other() {
        assert_eq!(rendered(OutputChannel::Left, 2), vec![0.5, 0.0, -0.5, 0.0, 0.25, 0.0]);
        assert_eq!(rendered(OutputChannel::Right, 2), vec![0.0, 0.5, 0.0, -0.5, 0.0, 0.25]);
        assert_eq!(rendered(OutputChannel::Both, 2), vec![0.5, 0.5, -0.5, -0.5, 0.25, 0.25]);
        assert_eq!(rendered(OutputChannel::Mono, 2), vec![0.5, 0.5, -0.5, -0.5, 0.25, 0.25]);
        assert_eq!(rendered(OutputChannel::Left, 1), vec![0.5, -0.5, 0.25], "un dispositivo mono suona sempre");
    }

    #[test]
    fn the_channel_names_are_the_ones_of_the_jtdx_ini() {
        for c in [OutputChannel::Mono, OutputChannel::Left, OutputChannel::Right, OutputChannel::Both] {
            assert_eq!(OutputChannel::parse(c.name()), Some(c));
            assert_eq!(OutputChannel::from_code(c.code()), c);
        }
        assert_eq!(OutputChannel::parse("right"), Some(OutputChannel::Right));
        assert_eq!(OutputChannel::parse("Centro"), None);
    }

    #[test]
    fn an_exact_device_name_wins_over_one_that_contains_it() {
        let names: Vec<String> = ["Altoparlanti (2- USB Audio Device)", "Altoparlanti (2- USB Audio Device) bis"].iter().map(|s| s.to_string()).collect();
        assert_eq!(match_device_name(&names, "altoparlanti (2- usb audio device) bis"), Some(1));
        assert_eq!(match_device_name(&names, "Altoparlanti (2- USB Audio Device)"), Some(0));
        assert_eq!(match_device_name(&names, "usb audio"), Some(0));
        assert_eq!(match_device_name(&names, "Cuffie"), None);
    }

    #[test]
    fn playback_delivers_the_wave_then_silence() {
        let mut pb = Playback::new();
        assert!(!pb.is_playing());
        let wave: Vec<f32> = (0..1000).map(|n| n as f32 / 1000.0).collect();
        pb.start(wave.clone());
        assert!(pb.is_playing());
        // blocchi di dimensione irregolare, come li chiede il dispositivo
        let mut got = Vec::new();
        for block in [128, 300, 7, 500, 100] {
            let mut buf = vec![9.0f32; block];
            pb.fill(&mut buf);
            got.extend(buf);
        }
        assert_eq!(&got[..1000], &wave[..]);
        assert!(got[1000..].iter().all(|&v| v == 0.0));
        assert!(!pb.is_playing());
    }

    #[test]
    fn an_empty_wave_does_not_play() {
        let mut pb = Playback::new();
        pb.start(Vec::new());
        assert!(!pb.is_playing());
    }

    #[test]
    fn stop_fades_out_without_a_click_and_goes_silent() {
        let mut pb = Playback::new();
        pb.start(vec![1.0; 10_000]);
        let mut got = drain(&mut pb, 100, 64);
        pb.begin_stop(240);
        got.extend(drain(&mut pb, 600, 100));
        assert!(!pb.is_playing());
        assert!(got[..100].iter().all(|&v| v == 1.0));
        assert!(got[100..340].windows(2).all(|w| w[1] <= w[0]), "discesa non monotona");
        assert!(got[340..].iter().all(|&v| v == 0.0));
        assert!(got.windows(2).all(|w| (w[1] - w[0]).abs() < 0.05), "salto udibile");
    }

    #[test]
    fn stop_without_fade_is_immediate_and_stop_when_idle_is_harmless() {
        let mut pb = Playback::new();
        pb.begin_stop(100);
        pb.start(vec![0.5; 100]);
        pb.begin_stop(0);
        assert!(!pb.is_playing());
        assert!(drain(&mut pb, 10, 10).iter().all(|&v| v == 0.0));
    }

    /// Per il proprietario: un la a 440 Hz di mezzo secondo sull'uscita predefinita, con -20 dB. Emette suono vero.
    #[test]
    #[ignore = "emette suono sul dispositivo predefinito"]
    fn plays_a_440_hz_tone_on_the_default_device() {
        let mut sink = CpalSink::open(None).expect("uscita predefinita");
        let wave: Vec<f32> = (0..6000).map(|n| (2.0 * PI * 440.0 * f64::from(n) / 12_000.0).sin() as f32).collect();
        let samples = crate::level::prepare_samples(&wave, sink.sample_rate(), 200, Duration::from_millis(10)).expect("preparazione");
        sink.start(samples).expect("avvio");
        let started = std::time::Instant::now();
        while sink.is_playing() && started.elapsed() < Duration::from_secs(3) {
            std::thread::sleep(Duration::from_millis(10));
        }
        assert!(!sink.is_playing(), "il suono non e' finito in tempo");
        std::thread::sleep(Duration::from_millis(200));
    }
}
