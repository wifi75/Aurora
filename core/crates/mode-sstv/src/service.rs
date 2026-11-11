//! Il servizio SSTV: riceve una copia dell'audio d'ingresso (solo quando l'ascolto e' acceso: l'FT8 non se ne accorge), decodifica in un
//! thread suo, salva le immagini, e trasmette un'immagine con la catena di trasmissione di Aurora (audio d'uscita e PTT).
//! Impostazioni in `sstv.json`, immagini in `sstv/` (o nella cartella scelta).

use std::{
    path::{Path, PathBuf},
    sync::{
        Arc, Mutex, MutexGuard,
        atomic::{AtomicBool, Ordering},
        mpsc::{Receiver, RecvTimeoutError, SyncSender, TrySendError, sync_channel},
    },
    time::{Duration, Instant},
};

use aurora_core::SstvEvent;
use base64::Engine as _;
use serde::{Deserialize, Serialize};

use crate::{
    Decoder, DecoderConfig, DecoderEvent, RATE, RgbImage, SstvError, SstvMode,
    encoder::encode_shifted,
    image_io::{decode_image, encode_png, fit_cover},
    resample::Resampler,
    store::{EntryExtra, ImageEntry, ImageStore},
};

/// La catena di trasmissione vista dall'SSTV (la fornisce il programma: `tx-native` con il PTT scelto).
pub trait SstvTxPort: Send + Sync {
    /// Trasmette subito l'onda (12 kHz, mono, picco 1), fuori dai periodi FT8.
    fn start(&self, wave: Vec<f32>) -> Result<(), String>;
    fn halt(&self);
    fn is_busy(&self) -> bool;
}

pub type Emitter = Arc<dyn Fn(SstvEvent) + Send + Sync>;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct SstvSettings {
    /// Modo proposto in trasmissione.
    pub default_mode: SstvMode,
    /// Salva da sola ogni immagine ricevuta.
    pub auto_save: bool,
    /// Salva anche le immagini trasmesse (cronologia).
    pub save_tx: bool,
    /// Ascolto acceso all'avvio di Aurora.
    pub auto_receive: bool,
    /// Solo questo modo in ricezione.
    pub lock_mode: Option<SstvMode>,
    /// Cartella delle immagini ricevute (vuota: la cartella `history` dentro la cartella di Aurora).
    pub folder: String,
    /// Cartella da cui si prendono le foto da trasmettere (vuota: nessuna).
    pub tx_folder: String,
    /// Correzione della sintonia in ricezione (Hz).
    pub rx_offset_hz: f32,
    /// Spostamento dei toni in trasmissione (Hz).
    pub tx_offset_hz: f32,
    /// Correzione dell'orologio di chi trasmette (ppm, l'inclinazione si corregge comunque da sola).
    pub slant_ppm: f32,
}

impl Default for SstvSettings {
    fn default() -> Self {
        SstvSettings { default_mode: SstvMode::Martin1, auto_save: true, save_tx: true, auto_receive: false, lock_mode: None, folder: String::new(), tx_folder: String::new(), rx_offset_hz: 0.0, tx_offset_hz: 0.0, slant_ppm: 0.0 }
    }
}

impl SstvSettings {
    fn validate(&self) -> Result<(), SstvError> {
        if !self.rx_offset_hz.is_finite() || self.rx_offset_hz.abs() > 200.0 || !self.tx_offset_hz.is_finite() || self.tx_offset_hz.abs() > 200.0 {
            return Err(SstvError::BadImage("correzione della sintonia oltre 200 Hz".into()));
        }
        if !self.slant_ppm.is_finite() || self.slant_ppm.abs() > 5000.0 {
            return Err(SstvError::BadImage("correzione dell'orologio oltre 5000 ppm".into()));
        }
        Ok(())
    }

    fn decoder(&self) -> DecoderConfig {
        DecoderConfig { lock: self.lock_mode, offset_hz: self.rx_offset_hz, slant_ppm: self.slant_ppm }
    }
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct RxStatus {
    pub listening: bool,
    pub receiving: bool,
    pub mode: Option<SstvMode>,
    pub width: u32,
    pub height: u32,
    pub rows: u32,
    pub sync: f32,
    pub offset_hz: f32,
    pub level_db: f32,
    /// L'ultima immagine salvata.
    pub last_image: Option<String>,
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct TxStatus {
    /// `idle`, `sending`, `done`, `aborted`, `error`.
    pub state: String,
    pub mode: Option<SstvMode>,
    pub progress: f32,
    pub elapsed_s: f32,
    pub total_s: f32,
    pub message: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct Snapshot {
    pub rx: RxStatus,
    pub tx: TxStatus,
    pub settings: SstvSettings,
    pub folder: String,
    pub can_transmit: bool,
    pub modes: Vec<crate::ModeInfo>,
}

enum Msg {
    Audio(u32, Vec<f32>),
    Force(SstvMode),
    Cancel,
    Config(DecoderConfig),
}

struct Inner {
    settings_path: PathBuf,
    data_dir: PathBuf,
    settings: Mutex<SstvSettings>,
    listening: AtomicBool,
    tx_active: AtomicBool,
    tx_abort: AtomicBool,
    queue: SyncSender<Msg>,
    rx: Mutex<RxStatus>,
    current: Mutex<Option<RgbImage>>,
    tx: Mutex<TxStatus>,
    port: Mutex<Option<Arc<dyn SstvTxPort>>>,
    emit: Mutex<Option<Emitter>>,
    store: Mutex<ImageStore>,
}

fn lock<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|p| p.into_inner())
}

/// La cartella `history` dentro la cartella di Aurora (dove sta il programma); se non si puo' scrivere li' (per esempio un avvio da sviluppo) resta
/// `sstv` nella cartella dei dati. Con `AURORA_INSTALL_DIR` si sceglie un'altra cartella di Aurora.
fn default_history(data_dir: &Path) -> PathBuf {
    let install = std::env::var_os("AURORA_INSTALL_DIR").map(PathBuf::from).or_else(|| std::env::current_exe().ok().and_then(|p| p.parent().map(Path::to_path_buf)));
    if let Some(dir) = install.map(|d| d.join("history"))
        && std::fs::create_dir_all(&dir).is_ok()
    {
        // prova di scrittura: sotto Program Files una cartella creata da un altro utente puo' non essere scrivibile
        let probe = dir.join(".scrittura");
        if std::fs::write(&probe, b"ok").is_ok() {
            let _ = std::fs::remove_file(&probe);
            return dir;
        }
    }
    data_dir.join("sstv")
}

fn folder_of(data_dir: &Path, s: &SstvSettings) -> PathBuf {
    if s.folder.trim().is_empty() { default_history(data_dir) } else { PathBuf::from(s.folder.trim()) }
}

/// Le immagini gia' salvate nella vecchia cartella dei dati passano (copiate) nella cartella `history` la prima volta che questa e' vuota.
fn migrate_old_images(data_dir: &Path, target: &Path) {
    let old = data_dir.join("sstv");
    if old == target || !old.join("index.json").is_file() || target.join("index.json").is_file() {
        return;
    }
    if std::fs::create_dir_all(target).is_err() {
        return;
    }
    if let Ok(read) = std::fs::read_dir(&old) {
        for e in read.flatten() {
            let from = e.path();
            if from.is_file() && let Some(name) = from.file_name() {
                let _ = std::fs::copy(&from, target.join(name));
            }
        }
    }
}

/// Le estensioni delle foto che si possono trasmettere.
const PHOTO_EXT: [&str; 6] = ["jpg", "jpeg", "png", "webp", "gif", "bmp"];
const PHOTO_MAX_BYTES: u64 = 20 * 1024 * 1024;

#[derive(Clone, Debug, Serialize)]
pub struct TxPhoto {
    pub name: String,
    pub size: u64,
}

fn photo_ext(name: &str) -> Option<String> {
    let ext = name.rsplit_once('.')?.1.to_ascii_lowercase();
    PHOTO_EXT.contains(&ext.as_str()).then_some(ext)
}

impl Inner {
    fn emit(&self, ev: SstvEvent) {
        let emitter = lock(&self.emit).clone();
        if let Some(e) = emitter {
            e(ev);
        }
    }

    fn rx_status_event(&self) {
        let st = lock(&self.rx).clone();
        self.emit(SstvEvent {
            kind: "rx_status".into(),
            listening: Some(st.listening),
            state: Some(if st.receiving { "receiving" } else { "idle" }.into()),
            mode: st.mode.map(|m| m.id().to_string()),
            sync: Some(st.sync),
            level_db: Some(st.level_db),
            offset_hz: Some(st.offset_hz),
            ..SstvEvent::default()
        });
    }

    fn tx_event(&self) {
        let st = lock(&self.tx).clone();
        self.emit(SstvEvent { kind: "tx".into(), state: Some(st.state), mode: st.mode.map(|m| m.id().to_string()), progress: Some(st.progress), message: st.message, ..SstvEvent::default() });
    }
}

/// Il servizio (uno per programma).
pub struct SstvService {
    inner: Arc<Inner>,
}

impl SstvService {
    /// Legge `sstv.json` dalla cartella dei dati e accende il thread di ricezione.
    pub fn open(data_dir: &Path) -> SstvService {
        let settings_path = data_dir.join("sstv.json");
        let settings: SstvSettings = std::fs::read(&settings_path).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or_default();
        let (queue, rx) = sync_channel::<Msg>(128);
        let history = folder_of(data_dir, &settings);
        if settings.folder.trim().is_empty() {
            migrate_old_images(data_dir, &history);
        }
        let store = ImageStore::open(&history);
        let inner = Arc::new(Inner {
            settings_path,
            data_dir: data_dir.to_path_buf(),
            listening: AtomicBool::new(settings.auto_receive),
            settings: Mutex::new(settings.clone()),
            tx_active: AtomicBool::new(false),
            tx_abort: AtomicBool::new(false),
            queue,
            rx: Mutex::new(RxStatus { listening: settings.auto_receive, ..RxStatus::default() }),
            current: Mutex::new(None),
            tx: Mutex::new(TxStatus { state: "idle".into(), ..TxStatus::default() }),
            port: Mutex::new(None),
            emit: Mutex::new(None),
            store: Mutex::new(store),
        });
        let worker = Arc::clone(&inner);
        let cfg = settings.decoder();
        if let Err(e) = std::thread::Builder::new().name("aurora-sstv".into()).spawn(move || rx_loop(&worker, rx, cfg)) {
            tracing::error!("SSTV: thread di ricezione non avviato: {e}");
        }
        SstvService { inner }
    }

    pub fn set_emitter(&self, emitter: Emitter) {
        *lock(&self.inner.emit) = Some(emitter);
    }

    pub fn set_tx_port(&self, port: Arc<dyn SstvTxPort>) {
        *lock(&self.inner.port) = Some(port);
    }

    /// Una copia dell'audio d'ingresso (frequenza della scheda). Con l'ascolto spento non costa niente.
    pub fn feed(&self, rate: u32, samples: &[f32]) {
        if !self.inner.listening.load(Ordering::Relaxed) || self.inner.tx_active.load(Ordering::Relaxed) {
            return;
        }
        // coda piena (PC molto carico): si perde un blocco piuttosto che fermare l'audio dell'FT8
        if let Err(TrySendError::Full(_)) = self.inner.queue.try_send(Msg::Audio(rate, samples.to_vec())) {
            tracing::debug!("SSTV: blocco audio perso");
        }
    }

    pub fn is_listening(&self) -> bool {
        self.inner.listening.load(Ordering::Relaxed)
    }

    pub fn snapshot(&self) -> Snapshot {
        let settings = lock(&self.inner.settings).clone();
        Snapshot {
            rx: lock(&self.inner.rx).clone(),
            tx: lock(&self.inner.tx).clone(),
            folder: lock(&self.inner.store).dir().display().to_string(),
            settings,
            can_transmit: lock(&self.inner.port).is_some(),
            modes: crate::mode_infos(),
        }
    }

    pub fn settings(&self) -> SstvSettings {
        lock(&self.inner.settings).clone()
    }

    /// Le foto della cartella da cui si trasmette (senza cartelle dentro), per nome, al massimo 500.
    pub fn tx_photos(&self) -> Result<Vec<TxPhoto>, SstvError> {
        let folder = lock(&self.inner.settings).tx_folder.trim().to_string();
        if folder.is_empty() {
            return Err(SstvError::BadImage("nessuna cartella scelta per le foto da trasmettere".into()));
        }
        let read = std::fs::read_dir(&folder).map_err(|_| SstvError::BadImage("cartella delle foto non trovata".into()))?;
        let mut out: Vec<TxPhoto> = read
            .flatten()
            .filter_map(|e| {
                let name = e.file_name().to_string_lossy().into_owned();
                let meta = e.metadata().ok()?;
                (meta.is_file() && photo_ext(&name).is_some() && meta.len() <= PHOTO_MAX_BYTES).then_some(TxPhoto { name, size: meta.len() })
            })
            .collect();
        out.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
        out.truncate(500);
        Ok(out)
    }

    /// Una foto della cartella da cui si trasmette: solo il nome del file (niente percorsi), le estensioni delle immagini e al massimo 20 MB.
    pub fn tx_photo(&self, name: &str) -> Result<(Vec<u8>, &'static str), SstvError> {
        let folder = lock(&self.inner.settings).tx_folder.trim().to_string();
        let ok_name = !name.is_empty() && !name.contains(['/', '\\', ':']) && !name.starts_with('.') && name.len() <= 200;
        let ext = photo_ext(name).filter(|_| ok_name && !folder.is_empty()).ok_or(SstvError::NotFound)?;
        let path = Path::new(&folder).join(name);
        let meta = std::fs::metadata(&path).map_err(|_| SstvError::NotFound)?;
        if !meta.is_file() || meta.len() > PHOTO_MAX_BYTES {
            return Err(SstvError::NotFound);
        }
        let mime = match ext.as_str() {
            "jpg" | "jpeg" => "image/jpeg",
            "png" => "image/png",
            "webp" => "image/webp",
            "gif" => "image/gif",
            _ => "image/bmp",
        };
        std::fs::read(&path).map(|b| (b, mime)).map_err(|_| SstvError::NotFound)
    }

    pub fn apply_settings(&self, s: SstvSettings) -> Result<(), SstvError> {
        s.validate()?;
        let json = serde_json::to_vec_pretty(&s).map_err(|e| SstvError::Io(e.to_string()))?;
        std::fs::write(&self.inner.settings_path, json).map_err(|e| SstvError::Io(e.to_string()))?;
        let folder = folder_of(&self.inner.data_dir, &s);
        {
            let mut store = lock(&self.inner.store);
            if store.dir() != folder {
                *store = ImageStore::open(&folder);
            }
        }
        let _ = self.inner.queue.send(Msg::Config(s.decoder()));
        *lock(&self.inner.settings) = s;
        Ok(())
    }

    pub fn start_rx(&self) {
        self.inner.listening.store(true, Ordering::Relaxed);
        lock(&self.inner.rx).listening = true;
        self.inner.rx_status_event();
    }

    pub fn stop_rx(&self) {
        self.inner.listening.store(false, Ordering::Relaxed);
        lock(&self.inner.rx).listening = false;
        let _ = self.inner.queue.send(Msg::Cancel);
        self.inner.rx_status_event();
    }

    /// Ricezione subito nel modo dato, senza aspettare il VIS (immagine gia' cominciata).
    pub fn force_rx(&self, mode: SstvMode) {
        self.start_rx();
        let _ = self.inner.queue.send(Msg::Force(mode));
    }

    /// Butta l'immagine in arrivo.
    pub fn cancel_rx(&self) {
        let _ = self.inner.queue.send(Msg::Cancel);
    }

    /// L'immagine in arrivo (o l'ultima finita) in PNG.
    pub fn current_png(&self) -> Option<Vec<u8>> {
        lock(&self.inner.current).as_ref().and_then(|img| encode_png(img).ok())
    }

    pub fn images(&self) -> Vec<ImageEntry> {
        lock(&self.inner.store).list()
    }

    pub fn image_png(&self, id: &str) -> Result<Vec<u8>, SstvError> {
        lock(&self.inner.store).png(id)
    }

    pub fn delete_image(&self, id: &str) -> Result<(), SstvError> {
        lock(&self.inner.store).delete(id)
    }

    /// Trasmette un'immagine (PNG o JPEG, portata alla risoluzione del modo se serve). `call`: il corrispondente, per la cronologia.
    pub fn transmit(&self, mode: SstvMode, bytes: &[u8], call: Option<String>) -> Result<TxStatus, SstvError> {
        let port = lock(&self.inner.port).clone().ok_or(SstvError::NoTransmitter)?;
        if self.inner.tx_active.load(Ordering::SeqCst) || port.is_busy() {
            return Err(SstvError::Busy);
        }
        let spec = mode.spec();
        let img = fit_cover(&decode_image(bytes)?, spec.width, spec.height);
        let settings = self.settings();
        let wave = encode_shifted(mode, &img, RATE, settings.tx_offset_hz)?;
        let total = wave.len() as f32 / RATE as f32;
        self.inner.tx_active.store(true, Ordering::SeqCst);
        self.inner.tx_abort.store(false, Ordering::SeqCst);
        // la propria trasmissione non si decodifica
        let _ = self.inner.queue.send(Msg::Cancel);
        if let Err(e) = port.start(wave) {
            self.inner.tx_active.store(false, Ordering::SeqCst);
            return Err(SstvError::Tx(e));
        }
        tracing::info!("SSTV: trasmissione {} ({total:.0} s)", spec.name);
        if settings.save_tx
            && let Err(e) = lock(&self.inner.store).add("tx", mode, &img, EntryExtra { complete: true, call, ..EntryExtra::default() })
        {
            tracing::warn!("SSTV: immagine trasmessa non salvata: {e}");
        }
        *lock(&self.inner.tx) = TxStatus { state: "sending".into(), mode: Some(mode), progress: 0.0, elapsed_s: 0.0, total_s: total, message: None };
        self.inner.tx_event();
        let inner = Arc::clone(&self.inner);
        let spawned = std::thread::Builder::new().name("aurora-sstv-tx".into()).spawn(move || tx_monitor(&inner, &port, total));
        if let Err(e) = spawned {
            tracing::error!("SSTV: controllo della trasmissione non avviato: {e}");
        }
        Ok(lock(&self.inner.tx).clone())
    }

    /// Ferma la trasmissione SSTV in corso (audio e PTT).
    pub fn abort_tx(&self) {
        if !self.inner.tx_active.load(Ordering::SeqCst) {
            return;
        }
        self.inner.tx_abort.store(true, Ordering::SeqCst);
        if let Some(port) = lock(&self.inner.port).clone() {
            port.halt();
        }
    }
}

fn tx_monitor(inner: &Arc<Inner>, port: &Arc<dyn SstvTxPort>, total: f32) {
    let started = Instant::now();
    let mut last_emit = Instant::now();
    let state = loop {
        std::thread::sleep(Duration::from_millis(200));
        let elapsed = started.elapsed().as_secs_f32();
        if inner.tx_abort.load(Ordering::SeqCst) {
            break "aborted";
        }
        // la catena puo' impiegare qualche centinaio di ms per partire (PTT): «finito» vale solo dopo il primo secondo
        if elapsed > 1.5 && !port.is_busy() {
            break if elapsed + 2.0 >= total { "done" } else { "error" };
        }
        {
            let mut tx = lock(&inner.tx);
            tx.elapsed_s = elapsed;
            tx.progress = (elapsed / total.max(0.1)).min(1.0);
        }
        if last_emit.elapsed() >= Duration::from_millis(500) {
            last_emit = Instant::now();
            inner.tx_event();
        }
    };
    {
        let mut tx = lock(&inner.tx);
        tx.state = state.into();
        if state == "done" {
            tx.progress = 1.0;
        }
        if state == "error" {
            tx.message = Some("trasmissione interrotta dalla catena di trasmissione (PTT, audio o un'altra trasmissione)".into());
        }
    }
    inner.tx_active.store(false, Ordering::SeqCst);
    inner.tx_abort.store(false, Ordering::SeqCst);
    tracing::info!("SSTV: trasmissione finita ({state})");
    inner.tx_event();
}

fn rows_event(mode: SstvMode, img: &RgbImage, first: u32, count: u32) -> SstvEvent {
    let row_bytes = (img.width * 3) as usize;
    let from = first as usize * row_bytes;
    let to = ((first + count) as usize * row_bytes).min(img.data.len());
    SstvEvent {
        kind: "rx_rows".into(),
        mode: Some(mode.id().to_string()),
        width: Some(img.width),
        height: Some(img.height),
        first_row: Some(first),
        rows: Some(count),
        rgb: Some(base64::engine::general_purpose::STANDARD.encode(&img.data[from.min(to)..to])),
        ..SstvEvent::default()
    }
}

fn rx_loop(inner: &Arc<Inner>, queue: Receiver<Msg>, cfg: DecoderConfig) {
    let mut decoder = Decoder::new(cfg);
    let mut resampler: Option<Resampler> = None;
    let mut audio = Vec::new();
    let mut last_status = Instant::now();
    loop {
        let msg = match queue.recv_timeout(Duration::from_millis(500)) {
            Ok(m) => Some(m),
            Err(RecvTimeoutError::Timeout) => None,
            Err(RecvTimeoutError::Disconnected) => return,
        };
        let mut events = Vec::new();
        match msg {
            Some(Msg::Audio(rate, samples)) => {
                let r = match &mut resampler {
                    Some(r) if r.in_rate() == rate => r,
                    slot => slot.insert(Resampler::new(rate)),
                };
                audio.clear();
                r.process(&samples, &mut audio);
                events = decoder.push(&audio);
            }
            Some(Msg::Force(mode)) => {
                decoder.cancel();
                events.push(decoder.force_start(mode));
            }
            Some(Msg::Cancel) => {
                if decoder.is_receiving() {
                    decoder.cancel();
                    events.push(DecoderEvent::Lost);
                }
            }
            Some(Msg::Config(c)) => decoder.set_config(c),
            None => {}
        }
        for ev in events {
            handle(inner, &decoder, ev);
        }
        if last_status.elapsed() >= Duration::from_millis(700) {
            last_status = Instant::now();
            {
                let mut st = lock(&inner.rx);
                st.level_db = decoder.level_db();
                st.receiving = decoder.is_receiving();
                st.sync = decoder.sync_quality();
                if let Some(off) = decoder.offset_hz() {
                    st.offset_hz = off;
                }
            }
            inner.rx_status_event();
        }
    }
}

fn handle(inner: &Arc<Inner>, decoder: &Decoder, ev: DecoderEvent) {
    match ev {
        DecoderEvent::Started { mode, offset_hz, forced } => {
            let spec = mode.spec();
            {
                let mut st = lock(&inner.rx);
                st.receiving = true;
                st.mode = Some(mode);
                st.width = spec.width;
                st.height = spec.height;
                st.rows = 0;
                st.offset_hz = offset_hz;
                st.sync = 0.0;
            }
            *lock(&inner.current) = Some(RgbImage { width: spec.width, height: spec.height, data: vec![0; (spec.width * spec.height * 3) as usize] });
            inner.emit(SstvEvent {
                kind: "rx_start".into(),
                mode: Some(mode.id().into()),
                width: Some(spec.width),
                height: Some(spec.height),
                offset_hz: Some(offset_hz),
                message: forced.then(|| "forced".to_string()),
                ..SstvEvent::default()
            });
        }
        DecoderEvent::Rows { first, count } => {
            if let Some((mode, img, rows)) = decoder.current() {
                *lock(&inner.current) = Some(img.clone());
                {
                    let mut st = lock(&inner.rx);
                    st.rows = rows;
                    st.sync = decoder.sync_quality();
                }
                inner.emit(rows_event(mode, img, first, count));
            }
        }
        DecoderEvent::Finished(done) => {
            *lock(&inner.current) = Some(done.image.clone());
            let auto_save = lock(&inner.settings).auto_save;
            let saved = if auto_save {
                match lock(&inner.store).add("rx", done.mode, &done.image, EntryExtra { sync_quality: Some(done.sync_quality), offset_hz: Some(done.offset_hz), complete: done.complete, call: None }) {
                    Ok(e) => Some(e.id),
                    Err(e) => {
                        tracing::warn!("SSTV: immagine non salvata: {e}");
                        None
                    }
                }
            } else {
                None
            };
            {
                let mut st = lock(&inner.rx);
                st.receiving = false;
                st.rows = done.image.height;
                st.sync = done.sync_quality;
                if saved.is_some() {
                    st.last_image = saved.clone();
                }
            }
            // l'immagine finita e' ridisegnata con l'inclinazione corretta: si rimanda tutta
            inner.emit(rows_event(done.mode, &done.image, 0, done.image.height));
            inner.emit(SstvEvent { kind: "rx_done".into(), mode: Some(done.mode.id().into()), image_id: saved, sync: Some(done.sync_quality), height: Some(done.image.height), width: Some(done.image.width), ..SstvEvent::default() });
        }
        DecoderEvent::Lost => {
            lock(&inner.rx).receiving = false;
            inner.emit(SstvEvent { kind: "rx_lost".into(), ..SstvEvent::default() });
        }
    }
}

#[cfg(test)]
mod cartelle_tests {
    use super::*;

    fn temp(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("aurora-sstv-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("cartella di prova");
        dir
    }

    #[test]
    fn le_estensioni_delle_foto() {
        assert_eq!(photo_ext("a.JPG").as_deref(), Some("jpg"));
        assert_eq!(photo_ext("b.png").as_deref(), Some("png"));
        assert!(photo_ext("c.txt").is_none());
        assert!(photo_ext("senza_estensione").is_none());
    }

    #[test]
    fn le_vecchie_immagini_passano_nella_cartella_history_una_volta() {
        let data = temp("dati");
        let old = data.join("sstv");
        std::fs::create_dir_all(&old).expect("vecchia");
        std::fs::write(old.join("index.json"), "[]").expect("indice");
        std::fs::write(old.join("uno.png"), b"png").expect("immagine");
        let target = temp("history");
        migrate_old_images(&data, &target);
        assert!(target.join("index.json").is_file() && target.join("uno.png").is_file());
        // la seconda volta non copia di nuovo sopra quello che c'e' gia'
        std::fs::write(target.join("uno.png"), b"nuovo").expect("modifica");
        migrate_old_images(&data, &target);
        assert_eq!(std::fs::read(target.join("uno.png")).expect("lettura"), b"nuovo");
        let _ = std::fs::remove_dir_all(&data);
        let _ = std::fs::remove_dir_all(&target);
    }

    #[test]
    fn le_foto_da_trasmettere_si_leggono_solo_dalla_cartella_scelta() {
        let data = temp("dati2");
        let photos = temp("foto");
        std::fs::write(photos.join("mare.jpg"), b"jpg").expect("foto");
        std::fs::write(photos.join("note.txt"), b"testo").expect("testo");
        let svc = SstvService::open(&data);
        let mut s = svc.settings();
        s.tx_folder = photos.display().to_string();
        svc.apply_settings(s).expect("impostazioni");
        let list = svc.tx_photos().expect("elenco");
        assert_eq!(list.iter().map(|p| p.name.as_str()).collect::<Vec<_>>(), ["mare.jpg"]);
        assert_eq!(svc.tx_photo("mare.jpg").expect("foto").1, "image/jpeg");
        // niente percorsi, niente file che non sono immagini
        assert!(svc.tx_photo(r"..\mare.jpg").is_err());
        assert!(svc.tx_photo("../mare.jpg").is_err());
        assert!(svc.tx_photo("note.txt").is_err());
        let _ = std::fs::remove_dir_all(&data);
        let _ = std::fs::remove_dir_all(&photos);
    }
}
