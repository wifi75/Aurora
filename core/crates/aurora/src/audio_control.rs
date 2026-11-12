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
    /// Il dispositivo si e' perso (scollegato, radio spenta) e va riaperto.
    fn is_lost(&self) -> bool {
        false
    }
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

/// L'errore dice che la scheda non c'e' (radio spenta, cavo USB staccato: «ingresso audio non trovato», «uscita audio non trovata»), non che c'e' e
/// non si apre: solo il primo caso e' un'attesa normale e non un guasto da segnalare.
fn device_missing(error: &str) -> bool {
    error.contains("non trovat")
}

/// I dispositivi veri del sistema.
pub struct CpalBackend {
    pub rows: broadcast::Sender<SpectrumRow>,
    pub events: broadcast::Sender<Event>,
    /// La radio: con il CAT su TCI o su Cat4OM ingresso e uscita possono essere l'audio che arriva in rete con il CAT.
    pub cat: Option<Arc<rig_control::cat::CatService>>,
}

/// L'audio che non passa da una scheda ma dal collegamento del CAT: il flusso della SDR (TCI) o l'audio in rete di Cat4OM.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum NetAudio {
    Tci,
    Cat4om,
}

impl NetAudio {
    const ALL: [NetAudio; 2] = [NetAudio::Tci, NetAudio::Cat4om];

    fn backend(self) -> rig_control::cat_config::Backend {
        match self {
            NetAudio::Tci => rig_control::cat_config::Backend::Tci,
            NetAudio::Cat4om => rig_control::cat_config::Backend::Cat4om,
        }
    }

    fn input_name(self) -> &'static str {
        match self {
            NetAudio::Tci => audio_spectrum::TCI_INPUT_NAME,
            NetAudio::Cat4om => rig_control::cat4om_audio::CAT4OM_AUDIO_NAME,
        }
    }

    fn output_name(self) -> &'static str {
        match self {
            NetAudio::Tci => tx_native::TCI_OUTPUT_NAME,
            NetAudio::Cat4om => rig_control::cat4om_audio::CAT4OM_AUDIO_NAME,
        }
    }

    /// Il testo se il CAT scelto non e' quello giusto.
    fn needs_cat(self, input: bool) -> &'static str {
        match (self, input) {
            (NetAudio::Tci, true) => "ingresso TCI: scegli prima TCI in Impostazioni › Stazione › Radio",
            (NetAudio::Tci, false) => "uscita TCI: scegli prima TCI in Impostazioni › Stazione › Radio",
            (NetAudio::Cat4om, true) => "ingresso Cat4OM: scegli prima Cat4OM in Impostazioni › Stazione › Radio",
            (NetAudio::Cat4om, false) => "uscita Cat4OM: scegli prima Cat4OM in Impostazioni › Stazione › Radio",
        }
    }
}

impl CpalBackend {
    /// Il CAT, se e' quello che porta questo audio.
    fn net_cat(&self, kind: NetAudio) -> Option<&Arc<rig_control::cat::CatService>> {
        self.cat.as_ref().filter(|c| c.backend() == kind.backend())
    }
}

/// La scelta e' l'ingresso o l'uscita TCI (il nome, senza distinzione di maiuscole).
pub fn is_tci_device(hint: Option<&str>, name: &str) -> bool {
    hint.is_some_and(|h| h.trim().eq_ignore_ascii_case(name))
}

/// L'ingresso non e' una scheda di Windows ma l'audio del CAT (TCI o Cat4OM): niente volume di registrazione da regolare.
pub fn is_network_input(hint: Option<&str>) -> bool {
    NetAudio::ALL.iter().any(|k| is_tci_device(hint, k.input_name()))
}

/// L'uscita non e' una scheda di Windows ma l'audio del CAT (TCI o Cat4OM): la potenza si regola nella radio o nella SDR.
pub fn is_network_output(name: &str) -> bool {
    NetAudio::ALL.iter().any(|k| is_tci_device(Some(name), k.output_name()))
}

struct Capture(audio_spectrum::CaptureHandle);

/// L'ingresso dal CAT: chiuderlo ferma il flusso audio (TCI `audio_stop`, Cat4OM `stopAudioStream`).
struct NetInput {
    _feed: audio_spectrum::TciFeedHandle,
    hook: audio_spectrum::TciRxSink,
    cat: Arc<rig_control::cat::CatService>,
    kind: NetAudio,
}

impl InputStream for NetInput {
    fn name(&self) -> String {
        self.kind.input_name().to_string()
    }
}

impl Drop for NetInput {
    fn drop(&mut self) {
        match self.kind {
            NetAudio::Tci => self.cat.release_tci_rx_audio(&self.hook),
            NetAudio::Cat4om => self.cat.release_cat4om_rx_audio(&self.hook),
        }
    }
}

/// L'uscita verso il CAT: chiuderla rida' alla radio la sua sorgente audio (con Cat4OM chiude lo stream di trasmissione).
struct NetOutput {
    sink: tx_native::TciSink,
    source: tx_native::TciTxSource,
    cat: Arc<rig_control::cat::CatService>,
    kind: NetAudio,
}

impl AudioSink for NetOutput {
    fn sample_rate(&self) -> u32 {
        self.sink.sample_rate()
    }
    fn start(&mut self, samples: Vec<f32>) -> Result<(), tx_native::TxError> {
        self.sink.start(samples)
    }
    fn is_playing(&self) -> bool {
        self.sink.is_playing()
    }
    fn stop(&mut self) {
        self.sink.stop();
    }
    fn live_gain(&self) -> Option<Arc<AtomicU32>> {
        self.sink.live_gain()
    }
    fn has_failed(&self) -> bool {
        self.sink.has_failed()
    }
}

impl Drop for NetOutput {
    fn drop(&mut self) {
        match self.kind {
            NetAudio::Tci => self.cat.release_tci_tx_audio(&self.source),
            NetAudio::Cat4om => self.cat.release_cat4om_tx_audio(&self.source),
        }
    }
}

impl InputStream for Capture {
    fn name(&self) -> String {
        self.0.device_name().to_string()
    }
    fn is_lost(&self) -> bool {
        self.0.is_lost()
    }
}

impl Backend for CpalBackend {
    fn input_devices(&self) -> Vec<AudioDevice> {
        let mut list: Vec<AudioDevice> = audio_spectrum::list_input_devices_info().into_iter().map(|d| AudioDevice { name: d.name, channels: d.channels, is_default: d.is_default }).collect();
        for kind in NetAudio::ALL {
            if self.net_cat(kind).is_some() {
                list.insert(0, AudioDevice { name: kind.input_name().to_string(), channels: 1, is_default: false });
            }
        }
        list
    }

    fn output_devices(&self) -> Vec<AudioDevice> {
        let mut list: Vec<AudioDevice> = tx_native::list_output_devices_info().into_iter().map(|d| AudioDevice { name: d.name, channels: d.channels, is_default: d.is_default }).collect();
        for kind in NetAudio::ALL {
            if self.net_cat(kind).is_some() {
                list.insert(0, AudioDevice { name: kind.output_name().to_string(), channels: 1, is_default: false });
            }
        }
        list
    }

    fn open_input(&self, hint: Option<&str>, channel: Arc<AtomicU8>, tap: mpsc::Sender<AudioChunk>) -> Result<Box<dyn InputStream>, String> {
        if let Some(kind) = NetAudio::ALL.into_iter().find(|k| is_tci_device(hint, k.input_name())) {
            let cat = self.net_cat(kind).ok_or(kind.needs_cat(true))?;
            // lo stesso ingresso del TCI: blocchi mono alla frequenza del server, portati a 12 kHz dal decodificatore
            let (feed, sink) = audio_spectrum::run_tci_feed(self.rows.clone(), self.events.clone(), Some(tap));
            match kind {
                NetAudio::Tci => cat.set_tci_rx_audio(Some(Arc::clone(&sink))),
                NetAudio::Cat4om => cat.set_cat4om_rx_audio(Some(Arc::clone(&sink))),
            }
            return Ok(Box::new(NetInput { _feed: feed, hook: sink, cat: Arc::clone(cat), kind }));
        }
        audio_spectrum::run_capture_with(hint, channel, self.rows.clone(), self.events.clone(), Some(tap)).map(|h| Box::new(Capture(h)) as Box<dyn InputStream>).map_err(|e| e.to_string())
    }

    fn open_output(&self, hint: Option<&str>, gain: Arc<AtomicU32>, channel: Arc<AtomicU8>) -> Result<(Box<dyn AudioSink>, String), String> {
        if let Some(kind) = NetAudio::ALL.into_iter().find(|k| is_tci_device(hint, k.output_name())) {
            let cat = self.net_cat(kind).ok_or(kind.needs_cat(false))?;
            // la stessa uscita del TCI: l'onda a 48 kHz, portata alla frequenza chiesta da chi la tira (SDR o collegamento Cat4OM)
            let (sink, source) = tx_native::TciSink::new(gain);
            match kind {
                NetAudio::Tci => cat.set_tci_tx_audio(Some(Arc::clone(&source))),
                NetAudio::Cat4om => cat.set_cat4om_tx_audio(Some(Arc::clone(&source))),
            }
            return Ok((Box::new(NetOutput { sink, source, cat: Arc::clone(cat), kind }), kind.output_name().to_string()));
        }
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
    /// L'ultimo errore di apertura dell'ingresso e dell'uscita (vuoto quando si aprono): la scheda Audio lo mostra, cosi' si legge il motivo vero.
    in_error: Arc<Mutex<Option<String>>>,
    out_error: Arc<Mutex<Option<String>>>,
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
        // i file audio stanno sempre nella cartella di Aurora: una cartella copiata da JTDX (o comunque fuori da Aurora) non vale
        let dir = if initial.save_dir.trim().is_empty() || !PathBuf::from(initial.save_dir.trim()).starts_with(data_dir) {
            default_save_dir.clone()
        } else {
            PathBuf::from(initial.save_dir.trim())
        };
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
            in_error: Arc::new(Mutex::new(None)),
            out_error: Arc::new(Mutex::new(None)),
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

    /// L'uscita usata per trasmettere adesso (il suo volume di Windows e' il cursore Pwr).
    pub fn output_name(&self) -> Option<String> {
        lock(&self.state).out_cur.name.clone()
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
        self.try_open_input(true);
    }

    /// C'e' un ingresso aperto?
    pub fn has_input(&self) -> bool {
        lock(&self.state).input.is_some()
    }

    /// C'e' un ingresso scelto da aprire (nelle impostazioni o nell'ambiente)?
    pub fn wants_input(&self) -> bool {
        !input_candidates(&self.initial.input, self.env_input.as_deref()).is_empty()
    }

    /// Riprova ad aprire l'ingresso scelto (la scheda della radio accesa dopo Aurora); `log` scrive gli errori nel registro.
    pub fn retry_input(&self) -> bool {
        // un ingresso che si e' perso durante l'uso (radio spenta, cavo mosso) si chiude e si riapre
        let lost = {
            let mut st = lock(&self.state);
            if st.input.as_ref().is_some_and(|i| i.is_lost()) { st.input.take() } else { None }
        };
        if let Some(stream) = lost {
            // non e' un guasto (radio spenta, cavo mosso): si riprende da solo, quindi resta nel registro senza diventare un rapporto
            tracing::info!("ingresso audio \"{}\" perso: lo riapro appena torna", stream.name());
            drop(stream);
        }
        if self.has_input() {
            return true;
        }
        self.try_open_input(false);
        self.has_input()
    }

    fn try_open_input(&self, log: bool) {
        let candidates = input_candidates(&self.initial.input, self.env_input.as_deref());
        let mut st = lock(&self.state);
        for (hint, from_store) in candidates {
            match self.backend.open_input(hint.as_deref(), Arc::clone(&self.in_channel), self.tap.clone()) {
                Ok(stream) => {
                    let name = stream.name();
                    tracing::info!("ingresso audio in uso: \"{name}\" (scelta: {}, canale {})", hint.as_deref().unwrap_or("predefinito del sistema"), st.in_channel.name());
                    *lock(&self.in_error) = None;
                    st.in_cur = Current { value: if from_store { self.initial.input.trim().to_string() } else { String::new() }, from_env: !from_store, name: Some(name.clone()) };
                    st.input = Some(stream);
                    drop(st);
                    (self.on_input)(&name);
                    self.start_engine_once();
                    return;
                }
                // una scheda scelta per nome che non c'e' (radio spenta, USB staccato) viene riprovata ogni 5 secondi e l'utente ne vede l'avviso sullo schermo:
                // non e' un guasto da segnalare. Una scheda che c'e' ma non si apre (occupata, formato rifiutato) resta un avviso vero.
                Err(e) => {
                    // il motivo resta a disposizione della scheda Audio; nei tentativi ripetuti (ogni 5 secondi) si scrive solo quando cambia
                    let changed = lock(&self.in_error).replace(e.clone()).as_deref() != Some(e.as_str());
                    let wanted = hint.as_deref().unwrap_or("predefinito");
                    if log && hint.is_some() && device_missing(&e) {
                        tracing::info!("cascata audio non ancora attiva da {wanted}: {e} (si riprova da sola)");
                    } else if log {
                        tracing::warn!("cascata audio non attiva da {wanted}: {e}");
                    } else if changed {
                        tracing::info!("ingresso audio {wanted} ancora non aperto: {e}");
                    }
                }
            }
        }
    }

    /// Apre l'uscita all'avvio (store, poi ambiente) e la da' alla trasmissione dentro un `SwitchableSink`: cosi' si potra' cambiare a caldo.
    /// `None` se non si apre (la trasmissione resta spenta, come prima).
    pub fn build_output(&self) -> Option<Box<dyn AudioSink>> {
        let stored = self.initial.output.trim().to_string();
        let pending_possible = !stored.is_empty() && !is_system_default(&stored);
        for (hint, from_store) in output_candidates(&self.initial.output, self.env_output.as_deref()) {
            match self.backend.open_output(hint.as_deref(), Arc::clone(&self.gain), Arc::clone(&self.out_channel)) {
                Ok((sink, name)) => {
                    let (switchable, switch) = SwitchableSink::new(sink, Arc::clone(&self.gain));
                    let mut st = lock(&self.state);
                    tracing::info!("uscita audio in uso: \"{name}\" (scelta: {}, canale {})", hint.as_deref().unwrap_or("predefinita del sistema"), st.out_channel.name());
                    *lock(&self.out_error) = None;
                    st.switch = Some(switch);
                    st.out_cur = Current { value: if from_store { self.initial.output.trim().to_string() } else { String::new() }, from_env: !from_store, name: Some(name) };
                    return Some(Box::new(switchable));
                }
                // con una scheda scelta che non c'e' ancora (radio spenta, USB staccato) sotto si prepara l'uscita «in attesa»: come WSJT-X («Not found»
                // e il nome resta salvato) non e' un guasto e non va nei rapporti degli errori; senza quella rete di sicurezza si': e' un avviso vero
                Err(e) => {
                    if pending_possible && device_missing(&e) {
                        tracing::info!("uscita audio per la trasmissione non ancora disponibile, resta in attesa della scheda: {e}");
                    } else {
                        tracing::warn!("uscita audio per la trasmissione non disponibile: {e}");
                    }
                    *lock(&self.out_error) = Some(e);
                }
            }
        }
        // l'uscita scelta non c'e' ancora (radio spenta, cavo USB staccato): la trasmissione si prepara lo stesso con un'uscita «in attesa»
        // che si riapre da sola appena la scheda compare. Prima la trasmissione restava spenta fino al riavvio di Aurora.
        if pending_possible {
            let backend = Arc::clone(&self.backend);
            let (gain, channel, hint) = (Arc::clone(&self.gain), Arc::clone(&self.out_channel), stored.clone());
            let last_error = Arc::clone(&self.out_error);
            let reopen: tx_native::ReopenFn = Arc::new(move || match backend.open_output(Some(&hint), Arc::clone(&gain), Arc::clone(&channel)) {
                Ok((sink, name)) => {
                    tracing::info!("uscita audio \"{name}\" comparsa: la trasmissione la usa da adesso");
                    *lock(&last_error) = None;
                    Ok(sink)
                }
                Err(e) => {
                    // riprovata a ogni trasmissione: nel registro solo quando il motivo cambia
                    if lock(&last_error).replace(e.clone()).as_deref() != Some(e.as_str()) {
                        tracing::info!("uscita audio {hint} ancora non aperta: {e}");
                    }
                    Err(tx_native::TxError::Audio(e))
                }
            });
            let (switchable, switch) = SwitchableSink::new(Box::new(tx_native::PendingSink::new(stored.clone(), reopen)), Arc::clone(&self.gain));
            let mut st = lock(&self.state);
            st.switch = Some(switch);
            st.out_cur = Current { value: stored, from_env: false, name: None };
            return Some(Box::new(switchable));
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
            if changed {
                tracing::info!("canale d'ingresso audio: da {} a {} ({})", st.in_channel.name(), channel.name(), st.in_cur.name.as_deref().unwrap_or("nessun ingresso aperto"));
            }
            st.in_channel = channel;
            self.in_channel.store(channel.code(), Ordering::Relaxed);
            return AudioPart::new(if changed { PartStatus::Applied } else { PartStatus::Unchanged }, st.in_cur.name.clone());
        }
        let hint: Option<String> = if requested.is_empty() {
            match self.env_input.as_deref().map(str::trim).filter(|e| !e.is_empty()) {
                Some(env) => Some(env.to_string()),
                None => {
                    tracing::warn!("ingresso audio: nessun dispositivo scelto, resta {}", st.in_cur.name.as_deref().unwrap_or("spento"));
                    return AudioPart::failed(st.in_cur.name.clone(), false, "nessun ingresso scelto: scegli un dispositivo");
                }
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
                tracing::info!("ingresso audio cambiato: da \"{}\" a \"{name}\" (scelta: {}, canale {})", old_name.as_deref().unwrap_or("nessuno"), hint.as_deref().unwrap_or("predefinito del sistema"), channel.name());
                *lock(&self.in_error) = None;
                (self.on_input)(&name);
                self.start_engine_once();
                AudioPart::new(PartStatus::Applied, Some(name))
            }
            Err(error) => {
                self.in_channel.store(old_channel.code(), Ordering::Relaxed);
                *lock(&self.in_error) = Some(error.clone());
                let Some(old) = old_name else {
                    tracing::warn!("ingresso audio {requested}: {error}; nessun ingresso aperto");
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
            if changed {
                tracing::info!("uscita audio: scelta salvata ({}, canale {}), vale dal prossimo avvio perche' la trasmissione non e' attiva", if requested.is_empty() { "predefinita del sistema" } else { requested }, channel.name());
            }
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
                        tracing::info!("uscita audio cambiata: da \"{}\" a \"{name}\" (scelta: {}, canale {})", in_use.as_deref().unwrap_or("nessuna"), hint.unwrap_or("predefinita del sistema"), channel.name());
                        *lock(&self.out_error) = None;
                        st.out_cur = Current { value: requested.to_string(), from_env: false, name: Some(name.clone()) };
                        st.out_pending = None;
                        in_use = Some(name);
                        status = PartStatus::Applied;
                    }
                    SwitchOutcome::Deferred => {
                        tracing::info!("uscita audio \"{name}\" aperta: prende il posto di \"{}\" alla fine della trasmissione in corso", in_use.as_deref().unwrap_or("nessuna"));
                        *lock(&self.out_error) = None;
                        st.out_pending = Some((requested.to_string(), name));
                        status = PartStatus::Deferred;
                    }
                },
                Err(error) => {
                    // l'uscita vecchia non e' stata toccata: si continua su quella
                    tracing::warn!("uscita audio {}: {error}; resta {}", hint.unwrap_or("predefinita del sistema"), in_use.as_deref().unwrap_or("nessuna uscita aperta"));
                    *lock(&self.out_error) = Some(error.clone());
                    return AudioPart { status: PartStatus::Failed, in_use, restored: st.out_cur.name.is_some(), message: Some(error) };
                }
            }
        }
        if st.out_channel != channel {
            tracing::info!("canale d'uscita audio: da {} a {} ({})", st.out_channel.name(), channel.name(), in_use.as_deref().unwrap_or("nessuna uscita aperta"));
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
        // solo dentro la cartella di Aurora (quella che contiene `save`): altrimenti si torna a quella di Aurora
        let inside = self.default_save_dir.parent().is_some_and(|root| PathBuf::from(w.save_dir.trim()).starts_with(root));
        let dir = if w.save_dir.trim().is_empty() || !inside { self.default_save_dir.clone() } else { PathBuf::from(w.save_dir.trim()) };
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
            input_error: lock(&self.in_error).clone(),
            output_error: lock(&self.out_error).clone(),
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

    #[test]
    fn the_audio_of_the_cat_is_not_a_windows_device() {
        for name in ["TCI (dalla radio)", "cat4om (AUDIO IN RETE)", " Cat4OM (audio in rete) "] {
            assert!(is_network_input(Some(name)), "{name}");
        }
        assert!(is_network_output("TCI (alla radio)") && is_network_output("Cat4OM (audio in rete)"));
        assert!(!is_network_input(Some("Microfono (USB Audio CODEC)")) && !is_network_input(None));
        assert!(!is_network_output("Altoparlanti (USB Audio CODEC)"));
        // senza CAT nessun audio di rete nell'elenco e la scelta dice cosa manca
        let (rows, _) = broadcast::channel(4);
        let (events, _) = broadcast::channel(4);
        let backend = CpalBackend { rows, events, cat: None };
        let (tap, _) = mpsc::channel();
        let refused = backend.open_input(Some("Cat4OM (audio in rete)"), Arc::new(AtomicU8::new(0)), tap).err();
        assert_eq!(refused.as_deref(), Some("ingresso Cat4OM: scegli prima Cat4OM in Impostazioni › Stazione › Radio"));
        let refused = backend.open_output(Some("Cat4OM (audio in rete)"), Arc::new(AtomicU32::new(0)), Arc::new(AtomicU8::new(0))).err();
        assert_eq!(refused.as_deref(), Some("uscita Cat4OM: scegli prima Cat4OM in Impostazioni › Stazione › Radio"));
    }

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
        // uscita scelta assente (radio spenta), nessun ambiente: la trasmissione resta in attesa di quella scheda, mai un'altra uscita
        let mut sink = r.manager.build_output().expect("uscita in attesa");
        assert!(sink.start(vec![0.0; 10]).is_err(), "finche' la scheda manca non si suona");
        assert!(!lock(&r.backend.opened).contains(&"out:default".to_string()));
        // la radio si accende: alla trasmissione seguente si riapre proprio quella scheda
        lock(&r.backend.broken).retain(|b| b != "Rotta");
        let _ = sink.sample_rate();
        assert!(sink.start(vec![0.0; 10]).is_ok(), "scheda ritrovata: si trasmette");
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
    fn the_real_opening_error_stays_readable_until_the_device_opens() {
        let r = rig("errori", values("Rotto", "Rotta"), None, None, &["Rotto", "Rotta"]);
        r.manager.open_initial_input();
        r.manager.build_output();
        let devices = r.manager.devices();
        assert_eq!(devices.input_error.as_deref(), Some("ingresso audio non aperto: Rotto"), "il motivo vero, non solo «non disponibile»");
        assert_eq!(devices.output_error.as_deref(), Some("uscita audio non aperta: Rotta"));
        // un ingresso che si apre cancella l'errore; un'uscita che non si apre lo aggiorna
        let out = r.manager.apply(&values("Line In", "Rotta"));
        assert_eq!(out.input.status, PartStatus::Applied);
        let devices = r.manager.devices();
        assert_eq!(devices.input_error, None);
        assert!(devices.output_error.is_some());
        // un'uscita che si apre cancella l'errore
        lock(&r.backend.broken).clear();
        let _ = r.manager.apply(&values("Line In", "Cuffie"));
        assert_eq!(r.manager.devices().output_error, None);
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

        // senza nessuna uscita scelta e nessuna aperta, la trasmissione non c'e' e la scelta si salva soltanto
        let r = rig("no-tx", values("", ""), None, None, &[]);
        r.manager.output_closed();
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
