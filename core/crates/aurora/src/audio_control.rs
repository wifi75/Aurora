//! Gestore della scheda Audio (motore proprio): apre, a caldo, l'ingresso (cascata, decodificatore e livello d'ingresso) e l'uscita della
//! trasmissione, applica i canali e il salvataggio dei file audio.
//!
//! Regole che proteggono la ricezione e la trasmissione:
//! - all'avvio vale lo store (importato da `JTDX.ini`) e, se e' vuoto, `AURORA_AUDIO_DEVICE` / `AURORA_AUDIO_OUT_DEVICE`; un nome dello store che non si
//!   apre ripiega sul valore dell'ambiente, mai sul predefinito del sistema (per non trasmettere su un'uscita sbagliata);
//! - un ingresso nuovo che non si apre lascia **riaperto quello di prima** (stesso canale), e la scelta non si salva;
//! - un'uscita nuova si apre prima e subentra a quella vecchia **tra una trasmissione e l'altra** (`SwitchableSink`): se non si apre, la vecchia non si tocca;
//! - senza catena di trasmissione (uscita o PTT non aperti all'avvio) la scelta dell'uscita si salva e vale dal prossimo avvio.

use std::{
    path::PathBuf,
    sync::{
        atomic::{AtomicU32, AtomicU8, Ordering},
        mpsc, Arc, Mutex, MutexGuard,
    },
};

use audio_spectrum::{AudioChunk, InputChannel};
use aurora_core::{Event, SpectrumRow};
use engine_native::{SaveMode, SaveSettings};
use remote_api::{
    audio_api::{is_system_default, AudioControl, AudioDevice, AudioDevices, AudioOutcome, AudioPart, PartStatus},
    engine_settings::AudioValues,
};
use tokio::sync::broadcast;
use tx_native::{AudioSink, OutputChannel, SinkSwitch, SwitchOutcome, SwitchableSink};

/// Un ingresso aperto: chiuderlo (lasciarlo cadere) ferma la cattura.
pub trait InputStream: Send {
    fn name(&self) -> String;
}

/// Cio' che serve per aprire e elencare i dispositivi: quelli veri (`cpal`) o, nelle prove, dei finti.
pub trait Backend: Send + Sync {
    fn input_devices(&self) -> Vec<AudioDevice>;
    fn output_devices(&self) -> Vec<AudioDevice>;
    /// Apre l'ingresso (`None` = predefinito del sistema) e manda l'audio a `tap`.
    fn open_input(&self, hint: Option<&str>, channel: Arc<AtomicU8>, tap: mpsc::Sender<AudioChunk>) -> Result<Box<dyn InputStream>, String>;
    /// Apre l'uscita (`None` = predefinita del sistema) e ne da' il nome vero.
    fn open_output(&self, hint: Option<&str>, gain: Arc<AtomicU32>, channel: Arc<AtomicU8>) -> Result<(Box<dyn AudioSink>, String), String>;
}

/// I dispositivi veri del sistema.
pub struct CpalBackend {
    pub rows: broadcast::Sender<SpectrumRow>,
    pub events: broadcast::Sender<Event>,
}

struct Capture(audio_spectrum::CaptureHandle);

impl InputStream for Capture {
    fn name(&self) -> String {
        self.0.device_name().to_string()
    }
}

impl Backend for CpalBackend {
    fn input_devices(&self) -> Vec<AudioDevice> {
        audio_spectrum::list_input_devices_info().into_iter().map(|d| AudioDevice { name: d.name, channels: d.channels, is_default: d.is_default }).collect()
    }

    fn output_devices(&self) -> Vec<AudioDevice> {
        tx_native::list_output_devices_info().into_iter().map(|d| AudioDevice { name: d.name, channels: d.channels, is_default: d.is_default }).collect()
    }

    fn open_input(&self, hint: Option<&str>, channel: Arc<AtomicU8>, tap: mpsc::Sender<AudioChunk>) -> Result<Box<dyn InputStream>, String> {
        audio_spectrum::run_capture_with(hint, channel, self.rows.clone(), self.events.clone(), Some(tap)).map(|h| Box::new(Capture(h)) as Box<dyn InputStream>).map_err(|e| e.to_string())
    }

    fn open_output(&self, hint: Option<&str>, gain: Arc<AtomicU32>, channel: Arc<AtomicU8>) -> Result<(Box<dyn AudioSink>, String), String> {
        let sink = tx_native::CpalSink::open_with(hint, gain, channel).map_err(|e| e.to_string())?;
        let name = sink.device_name().to_string();
        Ok((Box::new(sink), name))
    }
}

/// Dove si cerca un ingresso all'avvio, nell'ordine: `(nome da cercare, viene dallo store?)`. `None` = predefinito del sistema.
/// Store vuoto: solo l'ambiente (e se manca non si apre niente, come prima). Un nome dello store che non si apre ripiega sull'ambiente.
pub fn input_candidates(store_value: &str, env: Option<&str>) -> Vec<(Option<String>, bool)> {
    let value = store_value.trim();
    let mut out = Vec::new();
    if !value.is_empty() {
        out.push((if is_system_default(value) { None } else { Some(value.to_string()) }, true));
    }
    if let Some(env) = env.map(str::trim).filter(|e| !e.is_empty()) {
        out.push((Some(env.to_string()), false));
    }
    out
}

/// Come [`input_candidates`] per l'uscita: senza niente scelto vale il predefinito del sistema (come prima della scheda Audio).
pub fn output_candidates(store_value: &str, env: Option<&str>) -> Vec<(Option<String>, bool)> {
    let value = store_value.trim();
    let env = env.map(str::trim).filter(|e| !e.is_empty());
    let mut out = Vec::new();
    if is_system_default(value) {
        out.push((None, true));
    } else if value.is_empty() {
        out.push((env.map(str::to_string), false));
    } else {
        out.push((Some(value.to_string()), true));
        if let Some(env) = env {
            out.push((Some(env.to_string()), false));
        }
    }
    out
}

/// L'ingresso (per il suo volume di registrazione) per nome, o il predefinito del sistema con `None`.
pub fn find_endpoint(hint: Option<&str>) -> Result<audio_spectrum::InputEndpoint, audio_spectrum::VolumeError> {
    match hint {
        Some(name) => audio_spectrum::InputEndpoint::find(name),
        None => audio_spectrum::InputEndpoint::find_default(),
    }
}

/// Cio' che e' in uso e da dove viene la scelta.
#[derive(Default)]
struct Current {
    /// Il valore dello store che ha dato questo dispositivo (vuoto se vale l'ambiente).
    value: String,
    /// Il dispositivo viene dall'ambiente, non dallo store.
    from_env: bool,
    name: Option<String>,
}

impl Current {
    fn is(&self, requested: &str, normalize: fn(&str) -> String) -> bool {
        let wanted = normalize(requested);
        if self.from_env {
            wanted.is_empty()
        } else {
            normalize(&self.value) == wanted
        }
    }
}

/// Uscita: «predefinito» e vuoto sono la stessa scelta.
fn norm_out(v: &str) -> String {
    let v = v.trim();
    if is_system_default(v) { String::new() } else { v.to_lowercase() }
}

/// Ingresso: «predefinito» e' una scelta (vuoto vuol dire «come l'ambiente»).
fn norm_in(v: &str) -> String {
    v.trim().to_lowercase()
}

struct State {
    input: Option<Box<dyn InputStream>>,
    in_cur: Current,
    in_channel: InputChannel,
    switch: Option<SinkSwitch>,
    out_cur: Current,
    out_channel: OutputChannel,
    /// Un'uscita aperta che aspetta la fine della trasmissione: (valore, nome).
    out_pending: Option<(String, String)>,
}

pub type InputHook = Arc<dyn Fn(&str) + Send + Sync>;
/// Cosa fare alla prima apertura di un ingresso.
pub type EngineStarter = Box<dyn FnOnce(Arc<SaveSettings>) + Send>;

pub struct AudioManager {
    backend: Arc<dyn Backend>,
    tap: mpsc::Sender<AudioChunk>,
    in_channel: Arc<AtomicU8>,
    out_channel: Arc<AtomicU8>,
    gain: Arc<AtomicU32>,
    save: Arc<SaveSettings>,
    default_save_dir: PathBuf,
    env_input: Option<String>,
    env_output: Option<String>,
    initial: AudioValues,
    /// Chiamato col nome del dispositivo ogni volta che un ingresso nuovo si apre (il livello d'ingresso lo segue).
    on_input: InputHook,
    /// Avvia il decodificatore alla prima volta che un ingresso si apre (con l'ingresso mai aperto non c'e' niente da decodificare).
    engine_starter: Mutex<Option<EngineStarter>>,
    state: Mutex<State>,
}

fn lock<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|p| p.into_inner())
}

impl AudioManager {
    /// Prepara il gestore con i valori iniziali (non apre ancora niente: vedi `open_initial_input` e `build_output`).
    pub fn new(
        backend: Arc<dyn Backend>,
        tap: mpsc::Sender<AudioChunk>,
        initial: AudioValues,
        env_input: Option<String>,
        env_output: Option<String>,
        data_dir: &std::path::Path,
        on_input: InputHook,
    ) -> Arc<AudioManager> {
        let in_channel = InputChannel::parse(&initial.input_channel).unwrap_or_default();
        let out_channel = OutputChannel::parse(&initial.output_channel).unwrap_or_default();
        let default_save_dir = data_dir.join("save");
        let dir = if initial.save_dir.trim().is_empty() { default_save_dir.clone() } else { PathBuf::from(initial.save_dir.trim()) };
        Arc::new(AudioManager {
            backend,
            tap,
            in_channel: Arc::new(AtomicU8::new(in_channel.code())),
            out_channel: Arc::new(AtomicU8::new(out_channel.code())),
            gain: Arc::new(AtomicU32::new(1.0f32.to_bits())),
            save: Arc::new(SaveSettings::new(SaveMode::parse(&initial.save_mode), dir)),
            default_save_dir,
            env_input,
            env_output,
            initial,
            on_input,
            engine_starter: Mutex::new(None),
            state: Mutex::new(State { input: None, in_cur: Current::default(), in_channel, switch: None, out_cur: Current::default(), out_channel, out_pending: None }),
        })
    }

    /// Cosa fare quando un ingresso si apre per la prima volta (avviare il decodificatore).
    pub fn set_engine_starter(&self, starter: EngineStarter) {
        *lock(&self.engine_starter) = Some(starter);
    }

    pub fn save_settings(&self) -> Arc<SaveSettings> {
        Arc::clone(&self.save)
    }

    /// L'ingresso che sta ricevendo adesso.
    pub fn input_name(&self) -> Option<String> {
        lock(&self.state).in_cur.name.clone()
    }

    fn start_engine_once(&self) {
        let starter = lock(&self.engine_starter).take();
        if let Some(starter) = starter {
            starter(Arc::clone(&self.save));
        }
    }

    /// Apre l'ingresso all'avvio (store, poi ambiente). Se niente e' scelto o niente si apre la cascata resta spenta, come prima.
    pub fn open_initial_input(&self) {
        let candidates = input_candidates(&self.initial.input, self.env_input.as_deref());
        let mut st = lock(&self.state);
        for (hint, from_store) in candidates {
            match self.backend.open_input(hint.as_deref(), Arc::clone(&self.in_channel), self.tap.clone()) {
                Ok(stream) => {
                    let name = stream.name();
                    st.in_cur = Current { value: if from_store { self.initial.input.trim().to_string() } else { String::new() }, from_env: !from_store, name: Some(name.clone()) };
                    st.input = Some(stream);
                    drop(st);
                    (self.on_input)(&name);
                    self.start_engine_once();
                    return;
                }
                Err(e) => tracing::warn!("cascata audio non attiva da {}: {e}", hint.as_deref().unwrap_or("predefinito")),
            }
        }
    }

    /// Apre l'uscita all'avvio (store, poi ambiente) e la da' alla trasmissione dentro un `SwitchableSink`: cosi' si potra' cambiare a caldo.
    /// `None` se non si apre (la trasmissione resta spenta, come prima).
    pub fn build_output(&self) -> Option<Box<dyn AudioSink>> {
        for (hint, from_store) in output_candidates(&self.initial.output, self.env_output.as_deref()) {
            match self.backend.open_output(hint.as_deref(), Arc::clone(&self.gain), Arc::clone(&self.out_channel)) {
                Ok((sink, name)) => {
                    let (switchable, switch) = SwitchableSink::new(sink, Arc::clone(&self.gain));
                    let mut st = lock(&self.state);
                    st.switch = Some(switch);
                    st.out_cur = Current { value: if from_store { self.initial.output.trim().to_string() } else { String::new() }, from_env: !from_store, name: Some(name) };
                    return Some(Box::new(switchable));
                }
                Err(e) => tracing::warn!("uscita audio per la trasmissione non disponibile: {e}"),
            }
        }
        // senza catena di trasmissione si tiene almeno traccia di cio' che e' scelto
        let mut st = lock(&self.state);
        st.out_cur = Current { value: self.initial.output.trim().to_string(), from_env: false, name: None };
        None
    }

    /// La catena di trasmissione non e' partita (PTT non aperto): l'uscita appena aperta si chiude e la scelta vale dal prossimo avvio.
    pub fn output_closed(&self) {
        let mut st = lock(&self.state);
        st.switch = None;
        st.out_cur.name = None;
    }

    /// Ferma l'ingresso (chiusura del programma).
    pub fn shutdown(&self) {
        let input = lock(&self.state).input.take();
        drop(input);
    }

    fn apply_input(&self, st: &mut State, w: &AudioValues) -> AudioPart {
        let channel = InputChannel::parse(&w.input_channel).unwrap_or_default();
        let requested = w.input.trim();
        let same = st.in_cur.is(requested, norm_in) && (st.input.is_some() || (requested.is_empty() && self.env_input.is_none()));
        if same {
            let changed = st.in_channel != channel;
            st.in_channel = channel;
            self.in_channel.store(channel.code(), Ordering::Relaxed);
            return AudioPart::new(if changed { PartStatus::Applied } else { PartStatus::Unchanged }, st.in_cur.name.clone());
        }
        let hint: Option<String> = if requested.is_empty() {
            match self.env_input.as_deref().map(str::trim).filter(|e| !e.is_empty()) {
                Some(env) => Some(env.to_string()),
                None => return AudioPart::failed(st.in_cur.name.clone(), false, "nessun ingresso scelto: scegli un dispositivo"),
            }
        } else if is_system_default(requested) {
            None
        } else {
            Some(requested.to_string())
        };
        let from_env = requested.is_empty();
        let old_channel = st.in_channel;
        let old_name = st.in_cur.name.clone();
        // si chiude il vecchio (alcuni dispositivi non si aprono due volte) e si apre il nuovo; se non va, si riapre il vecchio
        drop(st.input.take());
        self.in_channel.store(channel.code(), Ordering::Relaxed);
        match self.backend.open_input(hint.as_deref(), Arc::clone(&self.in_channel), self.tap.clone()) {
            Ok(stream) => {
                let name = stream.name();
                st.input = Some(stream);
                st.in_channel = channel;
                st.in_cur = Current { value: if from_env { String::new() } else { requested.to_string() }, from_env, name: Some(name.clone()) };
                (self.on_input)(&name);
                self.start_engine_once();
                AudioPart::new(PartStatus::Applied, Some(name))
            }
            Err(error) => {
                self.in_channel.store(old_channel.code(), Ordering::Relaxed);
                let Some(old) = old_name else {
                    return AudioPart::failed(None, false, error);
                };
                match self.backend.open_input(Some(&old), Arc::clone(&self.in_channel), self.tap.clone()) {
                    Ok(stream) => {
                        st.input = Some(stream);
                        tracing::warn!("ingresso audio {requested}: {error}; ripristinato {old}");
                        AudioPart::failed(Some(old), true, error)
                    }
                    Err(again) => {
                        st.in_cur.name = None;
                        tracing::error!("ingresso audio {requested}: {error}; il precedente {old} non si riapre: {again}");
                        AudioPart::failed(None, false, format!("{error}; il dispositivo di prima non si riapre: {again}"))
                    }
                }
            }
        }
    }

    fn promote_pending(&self, st: &mut State) {
        let done = match (&st.switch, &st.out_pending) {
            (Some(switch), Some(_)) => !switch.has_pending(),
            _ => false,
        };
        if done && let Some((value, name)) = st.out_pending.take() {
            st.out_cur = Current { value, from_env: false, name: Some(name) };
        }
    }

    fn apply_output(&self, st: &mut State, w: &AudioValues) -> AudioPart {
        self.promote_pending(st);
        let channel = OutputChannel::parse(&w.output_channel).unwrap_or_default();
        let requested = w.output.trim();
        let pending_same = st.out_pending.as_ref().is_some_and(|(v, _)| norm_out(v) == norm_out(requested));
        let same = st.out_cur.is(requested, norm_out) || pending_same;
        let Some(switch) = st.switch.clone() else {
            // niente catena di trasmissione: la scelta si ricorda e vale dal prossimo avvio
            let changed = !same || st.out_channel != channel;
            st.out_channel = channel;
            if !same {
                st.out_cur = Current { value: requested.to_string(), from_env: false, name: None };
            }
            return AudioPart::new(if changed { PartStatus::SavedOnly } else { PartStatus::Unchanged }, None);
        };
        let mut status = PartStatus::Unchanged;
        let mut in_use = st.out_cur.name.clone();
        if !same {
            let hint = if requested.is_empty() || is_system_default(requested) { None } else { Some(requested) };
            match self.backend.open_output(hint, Arc::clone(&self.gain), Arc::clone(&self.out_channel)) {
                Ok((sink, name)) => match switch.request(sink) {
                    SwitchOutcome::Applied => {
                        st.out_cur = Current { value: requested.to_string(), from_env: false, name: Some(name.clone()) };
                        st.out_pending = None;
                        in_use = Some(name);
                        status = PartStatus::Applied;
                    }
                    SwitchOutcome::Deferred => {
                        st.out_pending = Some((requested.to_string(), name));
                        status = PartStatus::Deferred;
                    }
                },
                Err(error) => {
                    // l'uscita vecchia non e' stata toccata: si continua su quella
                    return AudioPart { status: PartStatus::Failed, in_use, restored: st.out_cur.name.is_some(), message: Some(error) };
                }
            }
        }
        if st.out_channel != channel {
            st.out_channel = channel;
            if status == PartStatus::Unchanged {
                status = PartStatus::Applied;
            }
        }
        self.out_channel.store(channel.code(), Ordering::Relaxed);
        AudioPart::new(status, in_use)
    }

    fn apply_save(&self, w: &AudioValues) -> AudioPart {
        let mode = SaveMode::parse(&w.save_mode);
        let dir = if w.save_dir.trim().is_empty() { self.default_save_dir.clone() } else { PathBuf::from(w.save_dir.trim()) };
        if mode != SaveMode::None
            && let Err(e) = check_writable(&dir)
        {
            return AudioPart::failed(None, false, format!("{}: {e}", dir.display()));
        }
        let changed = self.save.get() != (mode, dir.clone());
        self.save.set(mode, dir);
        AudioPart::new(if changed { PartStatus::Applied } else { PartStatus::Unchanged }, None)
    }
}

/// La cartella esiste (o si crea) e ci si puo' scrivere.
fn check_writable(dir: &std::path::Path) -> std::io::Result<()> {
    std::fs::create_dir_all(dir)?;
    let probe = dir.join(".aurora-prova");
    std::fs::write(&probe, b"")?;
    let _ = std::fs::remove_file(&probe);
    Ok(())
}

impl AudioControl for AudioManager {
    fn devices(&self) -> AudioDevices {
        let (inputs, outputs) = (self.backend.input_devices(), self.backend.output_devices());
        let mut st = lock(&self.state);
        self.promote_pending(&mut st);
        AudioDevices {
            inputs,
            outputs,
            input_in_use: st.in_cur.name.clone().filter(|_| st.input.is_some()),
            output_in_use: st.out_cur.name.clone(),
            output_pending: st.out_pending.as_ref().map(|(_, n)| n.clone()),
            default_save_dir: self.default_save_dir.display().to_string(),
        }
    }

    fn apply(&self, wanted: &AudioValues) -> AudioOutcome {
        let mut st = lock(&self.state);
        let input = self.apply_input(&mut st, wanted);
        let output = self.apply_output(&mut st, wanted);
        drop(st);
        AudioOutcome { input, output, save: self.apply_save(wanted) }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        collections::BTreeSet,
        sync::atomic::{AtomicBool, AtomicUsize},
    };

    struct FakeStream {
        name: String,
        open: Arc<Mutex<Vec<String>>>,
    }

    impl InputStream for FakeStream {
        fn name(&self) -> String {
            self.name.clone()
        }
    }

    impl Drop for FakeStream {
        fn drop(&mut self) {
            lock(&self.open).retain(|n| n != &self.name);
        }
    }

    struct FakeSink {
        rate: u32,
        playing: Arc<AtomicBool>,
    }

    impl AudioSink for FakeSink {
        fn sample_rate(&self) -> u32 {
            self.rate
        }
        fn start(&mut self, _: Vec<f32>) -> Result<(), tx_native::TxError> {
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

    /// Dispositivi finti: «Rotto» e «Occupato» non si aprono; il predefinito del sistema e' «Microfono USB» / «Altoparlanti USB».
    #[derive(Default)]
    struct FakeBackend {
        open: Arc<Mutex<Vec<String>>>,
        broken: Mutex<BTreeSet<String>>,
        opened: Mutex<Vec<String>>,
        playing: Arc<AtomicBool>,
    }

    impl FakeBackend {
        fn real_name(hint: Option<&str>) -> String {
            match hint {
                None => "Microfono USB".to_string(),
                Some(h) if h.ends_with('!') => h.to_string(),
                Some(h) => format!("{h}!"),
            }
        }
    }

    impl Backend for FakeBackend {
        fn input_devices(&self) -> Vec<AudioDevice> {
            vec![AudioDevice { name: "Microfono USB".into(), channels: 2, is_default: true }]
        }
        fn output_devices(&self) -> Vec<AudioDevice> {
            vec![AudioDevice { name: "Altoparlanti USB".into(), channels: 2, is_default: true }]
        }
        fn open_input(&self, hint: Option<&str>, _: Arc<AtomicU8>, _: mpsc::Sender<AudioChunk>) -> Result<Box<dyn InputStream>, String> {
            lock(&self.opened).push(format!("in:{}", hint.unwrap_or("default")));
            if hint.is_some_and(|h| lock(&self.broken).contains(h.trim_end_matches('!'))) {
                return Err(format!("ingresso audio non aperto: {}", hint.unwrap_or("")));
            }
            let name = Self::real_name(hint);
            lock(&self.open).push(name.clone());
            Ok(Box::new(FakeStream { name, open: Arc::clone(&self.open) }))
        }
        fn open_output(&self, hint: Option<&str>, _: Arc<AtomicU32>, _: Arc<AtomicU8>) -> Result<(Box<dyn AudioSink>, String), String> {
            lock(&self.opened).push(format!("out:{}", hint.unwrap_or("default")));
            if hint.is_some_and(|h| lock(&self.broken).contains(h)) {
                return Err(format!("uscita audio non aperta: {}", hint.unwrap_or("")));
            }
            Ok((Box::new(FakeSink { rate: 48_000, playing: Arc::clone(&self.playing) }), hint.map(|h| format!("{h}!")).unwrap_or_else(|| "Altoparlanti USB".into())))
        }
    }

    struct Rig {
        manager: Arc<AudioManager>,
        backend: Arc<FakeBackend>,
        hooked: Arc<Mutex<Vec<String>>>,
        engine_starts: Arc<AtomicUsize>,
        dir: PathBuf,
    }

    fn values(input: &str, output: &str) -> AudioValues {
        AudioValues { input: input.into(), output: output.into(), input_channel: "Mono".into(), output_channel: "Mono".into(), save_dir: String::new(), save_mode: "0".into() }
    }

    fn rig(name: &str, initial: AudioValues, env_in: Option<&str>, env_out: Option<&str>, broken: &[&str]) -> Rig {
        let dir = std::env::temp_dir().join(format!("aurora-audio-manager-{name}-{}", std::process::id()));
        std::fs::remove_dir_all(&dir).ok();
        let backend = Arc::new(FakeBackend::default());
        for b in broken {
            lock(&backend.broken).insert((*b).to_string());
        }
        let hooked = Arc::new(Mutex::new(Vec::new()));
        let (tx, _rx) = mpsc::channel();
        let hook_log = Arc::clone(&hooked);
        let manager = AudioManager::new(backend.clone(), tx, initial, env_in.map(str::to_string), env_out.map(str::to_string), &dir, Arc::new(move |n| lock(&hook_log).push(n.to_string())));
        let engine_starts = Arc::new(AtomicUsize::new(0));
        let counter = Arc::clone(&engine_starts);
        manager.set_engine_starter(Box::new(move |_| {
            counter.fetch_add(1, Ordering::SeqCst);
        }));
        Rig { manager, backend, hooked, engine_starts, dir }
    }

    #[test]
    fn with_nothing_chosen_nothing_changes_from_before() {
        let r = rig("nothing", values("", ""), None, None, &[]);
        r.manager.open_initial_input();
        assert!(r.manager.input_name().is_none(), "senza ingresso scelto la cascata resta spenta, come prima");
        assert_eq!(r.engine_starts.load(Ordering::SeqCst), 0);
        assert!(r.manager.build_output().is_some());
        assert_eq!(*lock(&r.backend.opened), vec!["out:default".to_string()], "l'uscita e' la predefinita, come prima");
        // «niente di scelto» e' gia' cosi': applicare gli stessi valori non apre niente
        let out = r.manager.apply(&values("", ""));
        assert_eq!((out.input.status, out.output.status, out.save.status), (PartStatus::Unchanged, PartStatus::Unchanged, PartStatus::Unchanged));
        assert_eq!(lock(&r.backend.opened).len(), 1);
    }

    #[test]
    fn the_environment_is_used_when_the_store_is_empty_and_the_store_wins_otherwise() {
        let r = rig("env", values("", ""), Some("USB Audio"), Some("Cuffie"), &[]);
        r.manager.open_initial_input();
        assert_eq!(r.manager.input_name().as_deref(), Some("USB Audio!"));
        assert_eq!(r.engine_starts.load(Ordering::SeqCst), 1, "il decodificatore parte quando c'e' un ingresso");
        r.manager.build_output();
        assert!(lock(&r.backend.opened).contains(&"out:Cuffie".to_string()));
        // applicare gli stessi valori vuoti non riapre niente
        let before = lock(&r.backend.opened).len();
        let out = r.manager.apply(&values("", ""));
        assert_eq!((out.input.status, out.output.status), (PartStatus::Unchanged, PartStatus::Unchanged));
        assert_eq!(lock(&r.backend.opened).len(), before);

        let r = rig("store", values("Line In", "Altoparlanti"), Some("USB Audio"), Some("Cuffie"), &[]);
        r.manager.open_initial_input();
        assert_eq!(r.manager.input_name().as_deref(), Some("Line In!"));
    }

    #[test]
    fn a_stored_name_that_does_not_open_falls_back_to_the_environment_never_to_the_system_default() {
        let r = rig("fallback", values("Rotto", "Rotta"), Some("USB Audio"), None, &["Rotto", "Rotta"]);
        r.manager.open_initial_input();
        assert_eq!(r.manager.input_name().as_deref(), Some("USB Audio!"));
        assert!(r.manager.build_output().is_none(), "uscita scelta e rotta, nessun ambiente: niente trasmissione, non si trasmette su un'altra uscita");
        assert!(!lock(&r.backend.opened).contains(&"out:default".to_string()));
        let r = rig("nothing-left", values("Rotto", ""), None, None, &["Rotto"]);
        r.manager.open_initial_input();
        assert!(r.manager.input_name().is_none());
    }

    #[test]
    fn a_new_input_replaces_the_old_one_and_the_level_follows() {
        let r = rig("switch", values("Line In", ""), None, None, &[]);
        r.manager.open_initial_input();
        let mut next = values("Altro", "");
        next.input_channel = "Right".into();
        let out = r.manager.apply(&next);
        assert_eq!(out.input.status, PartStatus::Applied);
        assert_eq!(out.input.in_use.as_deref(), Some("Altro!"));
        assert_eq!(*lock(&r.backend.open), vec!["Altro!".to_string()], "il vecchio e' chiuso");
        assert_eq!(*lock(&r.hooked), vec!["Line In!".to_string(), "Altro!".to_string()]);
        assert_eq!(r.engine_starts.load(Ordering::SeqCst), 1, "il decodificatore parte una volta sola");
        assert_eq!(InputChannel::load(&r.manager.in_channel), InputChannel::Right);
    }

    #[test]
    fn an_input_that_does_not_open_restores_the_previous_one_with_the_old_channel() {
        let r = rig("restore", values("Line In", ""), None, None, &["Rotto"]);
        r.manager.open_initial_input();
        let mut next = values("Rotto", "");
        next.input_channel = "Left".into();
        let out = r.manager.apply(&next);
        assert_eq!(out.input.status, PartStatus::Failed);
        assert!(out.input.restored);
        assert_eq!(out.input.in_use.as_deref(), Some("Line In!"));
        assert!(out.input.message.as_deref().unwrap_or("").contains("non aperto"));
        assert_eq!(*lock(&r.backend.open), vec!["Line In!".to_string()], "il precedente ha ripreso a ricevere");
        assert_eq!(InputChannel::load(&r.manager.in_channel), InputChannel::Mono, "il canale non cambia se il dispositivo non si apre");
        assert_eq!(r.manager.input_name().as_deref(), Some("Line In!"));
        // anche se il vecchio non si riapre, l'esito lo dice e la ricezione risulta ferma
        lock(&r.backend.broken).insert("Line In".into());
        let out = r.manager.apply(&values("Rotto", ""));
        assert!(out.input.is_failed() && !out.input.restored && out.input.in_use.is_none());
        assert!(r.manager.devices().input_in_use.is_none());
    }

    #[test]
    fn an_empty_input_cannot_replace_a_running_one() {
        let r = rig("empty", values("Line In", ""), None, None, &[]);
        r.manager.open_initial_input();
        let out = r.manager.apply(&values("", ""));
        assert!(out.input.is_failed(), "svuotare la scelta non deve spegnere la ricezione");
        assert_eq!(r.manager.input_name().as_deref(), Some("Line In!"));
    }

    #[test]
    fn the_system_default_input_can_be_chosen_explicitly() {
        let r = rig("default", values("", ""), Some("USB Audio"), None, &[]);
        r.manager.open_initial_input();
        let out = r.manager.apply(&values("Default", ""));
        assert_eq!(out.input.status, PartStatus::Applied);
        assert_eq!(out.input.in_use.as_deref(), Some("Microfono USB"));
    }

    #[test]
    fn the_output_changes_between_transmissions_and_a_failure_keeps_the_old_one() {
        let r = rig("output", values("", "Altoparlanti"), None, None, &["Rotta"]);
        let mut sink = r.manager.build_output().expect("uscita");
        // idle: il cambio e' immediato
        let out = r.manager.apply(&values("", "Cuffie"));
        assert_eq!((out.output.status, out.output.in_use.as_deref()), (PartStatus::Applied, Some("Cuffie!")));
        // un'uscita che non si apre lascia la vecchia, e lo dice
        let out = r.manager.apply(&values("", "Rotta"));
        assert_eq!(out.output.status, PartStatus::Failed);
        assert!(out.output.restored);
        assert_eq!(out.output.in_use.as_deref(), Some("Cuffie!"));
        // in trasmissione il cambio aspetta la fine
        sink.sample_rate();
        sink.start(vec![0.0; 8]).expect("avvio");
        let out = r.manager.apply(&values("", "Speaker"));
        assert_eq!(out.output.status, PartStatus::Deferred);
        let devices = r.manager.devices();
        assert_eq!((devices.output_in_use.as_deref(), devices.output_pending.as_deref()), (Some("Cuffie!"), Some("Speaker!")));
        // la stessa richiesta ripetuta non apre un'altra uscita
        let opened = lock(&r.backend.opened).len();
        assert_eq!(r.manager.apply(&values("", "Speaker")).output.status, PartStatus::Unchanged);
        assert_eq!(lock(&r.backend.opened).len(), opened);
        sink.stop();
        assert_eq!(sink.sample_rate(), 48_000, "alla prossima trasmissione subentra la nuova");
        let devices = r.manager.devices();
        assert_eq!((devices.output_in_use.as_deref(), devices.output_pending), (Some("Speaker!"), None));
    }

    #[test]
    fn the_output_channel_applies_at_once_and_without_a_transmitter_the_choice_is_only_saved() {
        let r = rig("channel", values("", ""), None, None, &[]);
        r.manager.build_output();
        let mut next = values("", "");
        next.output_channel = "Both".into();
        assert_eq!(r.manager.apply(&next).output.status, PartStatus::Applied);
        assert_eq!(OutputChannel::load(&r.manager.out_channel), OutputChannel::Both);

        let r = rig("no-tx", values("", "Rotta"), None, None, &["Rotta"]);
        assert!(r.manager.build_output().is_none());
        let out = r.manager.apply(&values("", "Altra"));
        assert_eq!((out.output.status, out.output.in_use), (PartStatus::SavedOnly, None));
        assert_eq!(r.manager.apply(&values("", "Altra")).output.status, PartStatus::Unchanged);
    }

    #[test]
    fn the_audio_files_follow_the_settings_and_an_unusable_folder_is_refused() {
        let r = rig("save", values("", ""), None, None, &[]);
        assert_eq!(r.manager.save_settings().get().0, SaveMode::None);
        let mut next = values("", "");
        next.save_mode = "2".into();
        next.save_dir = r.dir.join("audio").display().to_string();
        let out = r.manager.apply(&next);
        assert_eq!(out.save.status, PartStatus::Applied);
        assert_eq!(r.manager.save_settings().get(), (SaveMode::All, r.dir.join("audio")));
        assert!(r.dir.join("audio").is_dir(), "la cartella si crea subito, cosi' un errore si vede ora");
        assert!(!r.dir.join("audio").join(".aurora-prova").exists());
        assert_eq!(r.manager.apply(&next).save.status, PartStatus::Unchanged);
        // una cartella sotto un file non si crea: resta quella di prima
        let file = r.dir.join("file.txt");
        std::fs::write(&file, b"x").expect("file");
        next.save_dir = file.join("sotto").display().to_string();
        let out = r.manager.apply(&next);
        assert!(out.save.is_failed());
        assert_eq!(r.manager.save_settings().get(), (SaveMode::All, r.dir.join("audio")));
        // con «Nessuno» la cartella non si controlla
        next.save_mode = "0".into();
        assert_eq!(r.manager.apply(&next).save.status, PartStatus::Applied);
    }
}
