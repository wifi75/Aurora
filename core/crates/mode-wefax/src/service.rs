//! Il servizio WEFAX: riceve una copia dell'audio d'ingresso (solo quando l'ascolto e' acceso: l'FT8 non se ne accorge), decodifica in un
//! thread suo, manda le righe appena arrivate all'interfaccia e salva le cartine finite. Impostazioni in `wefax.json`, cartine in `wefax/`
//! (o nella cartella scelta). Solo ricezione.

use std::{
    path::{Path, PathBuf},
    sync::{
        Arc, Mutex, MutexGuard,
        atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering},
        mpsc::{Receiver, RecvTimeoutError, SyncSender, TrySendError, sync_channel},
    },
    time::{Duration, Instant},
};

use aurora_core::WefaxEvent;
use base64::Engine as _;
use serde::{Deserialize, Serialize};

use crate::{
    Decoder, DecoderConfig, DecoderEvent, Filter, GrayImage, WefaxError,
    image_io::encode_png,
    recorder::Recorder,
    resample::Resampler,
    store::{EntryExtra, ImageEntry, ImageStore},
};

pub type Emitter = Arc<dyn Fn(WefaxEvent) + Send + Sync>;

/// Blocchi audio che la coda verso il ricevitore tiene prima di perderne (qualche secondo fino a un quarto di minuto, secondo la grandezza
/// dei blocchi della scheda): un PC carico che rallenta il ricevitore per un po' non fa perdere audio, e chi alimenta non aspetta mai.
const QUEUE_BLOCKS: usize = 512;

/// Le righe al minuto ammesse.
pub const LPM_VALUES: [u32; 4] = [240, 120, 90, 60];

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct WefaxSettings {
    /// 576 (cartine standard, 1809 colonne) o 288 (904 colonne).
    pub ioc: u32,
    /// L'IOC si riconosce dal tono d'inizio (300 Hz: 576, 675 Hz: 288) e vince su `ioc` per quella ricezione.
    pub auto_ioc: bool,
    /// Righe al minuto di partenza con IOC 576 (240, 120, 90, 60); con la fasatura si riconoscono da sole.
    pub lpm_576: u32,
    /// Righe al minuto di partenza con IOC 288.
    pub lpm_288: u32,
    /// Filtro d'ingresso: `narrow`, `medium`, `wide`.
    pub filter: Filter,
    /// Modo manuale: nessun APT ne' fasatura, l'immagine comincia subito e finisce a mano (o al massimo di righe).
    pub manual: bool,
    /// Il numero di righe al minuto e il periodo di riga si ricavano dalla fasatura (correzione automatica dell'inclinazione).
    pub auto_lpm: bool,
    /// Ricentra l'immagine da sola nelle prime 500 righe.
    pub auto_center: bool,
    /// Misura l'inclinazione dal contenuto delle prime 400 righe e la corregge da sola (anche quelle gia' ricevute).
    pub auto_slant: bool,
    /// Salva da sola ogni cartina ricevuta.
    pub auto_save: bool,
    /// Ascolto acceso all'avvio di Aurora.
    pub auto_receive: bool,
    /// Sottoportante (Hz): 1900 di norma.
    pub center_hz: f32,
    /// Escursione nero-bianco (Hz): 800 di norma (850 per il Deutscher Wetterdienst).
    pub shift_hz: f32,
    /// Correzione a mano del periodo di riga (ppm, positivo: righe piu' lunghe), sopra quella automatica.
    pub slant_ppm: f32,
    /// Errore di orologio della scheda audio ricordato tra una cartina e l'altra (ppm, media pesata delle stime dell'inclinazione automatica,
    /// al massimo 5000): si somma alla correzione a mano per le immagini senza fasatura. Lo scrive il ricevitore, non l'interfaccia: un `PUT`
    /// con un valore diverso non lo cambia (si azzera con `reset_learned`).
    pub learned_ppm: f32,
    /// Peso accumulato di `learned_ppm` e frequenza di campionamento a cui si riferisce (cambiata quella, il valore si azzera).
    pub learned_weight: f32,
    pub learned_rate: u32,
    /// Massimo di righe di un'immagine (1200 sono il normale con IOC 576).
    pub max_rows: u32,
    /// Soglia di correlazione tra righe per il segnale perso (0,05).
    pub correlation: f32,
    /// Cartella delle cartine (vuota: `wefax` nella cartella dei dati).
    pub folder: String,
    /// AFC: la sottoportante di ricezione segue il segnale se la radio e' sintonizzata un po' fuori.
    pub afc: bool,
    /// Limite globale dell'AFC: la correzione non va oltre questi Hz dall'ultima sottoportante scelta a mano (20..400, 150 di base).
    pub afc_range_hz: f32,
    /// Squelch (SQL): l'immagine parte quando il segnale supera `squelch_level` e si chiude quando sparisce per 3 secondi.
    pub squelch: bool,
    /// Soglia dello squelch (0..100; 100 = 20 dB sopra il rumore).
    pub squelch_level: f32,
    /// Registra l'audio di ogni cartina in un WAV nella sottocartella `audio` (dal tono d'inizio alla chiusura; scartata la cartina, il file
    /// si cancella).
    pub record_audio: bool,
    /// Giorni per cui si tengono le registrazioni (0: per sempre). La pulizia gira all'avvio e a ogni registrazione nuova.
    pub audio_keep_days: u32,
}

/// Giorni massimi di `audio_keep_days` (dieci anni).
pub const AUDIO_KEEP_DAYS_MAX: u32 = 3650;

impl Default for WefaxSettings {
    fn default() -> Self {
        let d = DecoderConfig::default();
        WefaxSettings {
            ioc: d.ioc,
            auto_ioc: d.auto_ioc,
            lpm_576: d.lpm_576,
            lpm_288: d.lpm_288,
            filter: d.filter,
            manual: false,
            auto_lpm: true,
            auto_center: true,
            auto_slant: d.auto_slant,
            auto_save: true,
            auto_receive: false,
            center_hz: d.center_hz,
            shift_hz: d.shift_hz,
            slant_ppm: 0.0,
            learned_ppm: 0.0,
            learned_weight: 0.0,
            learned_rate: 0,
            max_rows: d.max_rows,
            correlation: d.correlation,
            folder: String::new(),
            afc: d.afc,
            afc_range_hz: d.afc_range_hz,
            squelch: d.squelch,
            squelch_level: d.squelch_level,
            record_audio: true,
            audio_keep_days: 7,
        }
    }
}

impl WefaxSettings {
    fn validate(&self) -> Result<(), WefaxError> {
        let bad = |m: &str| Err(WefaxError::BadSettings(m.into()));
        if self.ioc != 576 && self.ioc != 288 {
            return bad("IOC: 576 o 288");
        }
        if !LPM_VALUES.contains(&self.lpm_576) || !LPM_VALUES.contains(&self.lpm_288) {
            return bad("righe al minuto: 60, 90, 120 o 240");
        }
        if !self.center_hz.is_finite() || !(1000.0..=3000.0).contains(&self.center_hz) {
            return bad("sottoportante tra 1000 e 3000 Hz");
        }
        if !self.shift_hz.is_finite() || !(400.0..=1200.0).contains(&self.shift_hz) {
            return bad("escursione tra 400 e 1200 Hz");
        }
        if !self.slant_ppm.is_finite() || self.slant_ppm.abs() > 20_000.0 {
            return bad("inclinazione oltre 20000 ppm");
        }
        if !(100..=10_000).contains(&self.max_rows) {
            return bad("massimo di righe tra 100 e 10000");
        }
        if !self.correlation.is_finite() || !(0.01..=0.5).contains(&self.correlation) {
            return bad("soglia di correlazione tra 0,01 e 0,5");
        }
        if !self.afc_range_hz.is_finite() || !(crate::decoder::AFC_RANGE_MIN_HZ..=crate::decoder::AFC_RANGE_MAX_HZ).contains(&self.afc_range_hz) {
            return bad("limite dell'AFC tra 20 e 400 Hz");
        }
        if !self.squelch_level.is_finite() || !(0.0..=100.0).contains(&self.squelch_level) {
            return bad("soglia dello squelch tra 0 e 100");
        }
        if self.audio_keep_days > AUDIO_KEEP_DAYS_MAX {
            return bad("giorni delle registrazioni tra 0 e 3650");
        }
        Ok(())
    }

    fn decoder(&self) -> DecoderConfig {
        DecoderConfig {
            ioc: self.ioc,
            auto_ioc: self.auto_ioc,
            lpm_576: self.lpm_576,
            lpm_288: self.lpm_288,
            filter: self.filter,
            manual: self.manual,
            auto_lpm: self.auto_lpm,
            auto_center: self.auto_center,
            auto_slant: self.auto_slant,
            center_hz: self.center_hz,
            shift_hz: self.shift_hz,
            slant_ppm: self.slant_ppm,
            learned_ppm: self.learned_ppm,
            learned_weight: self.learned_weight,
            max_rows: self.max_rows,
            correlation: self.correlation,
            afc: self.afc,
            afc_range_hz: self.afc_range_hz,
            squelch: self.squelch,
            squelch_level: self.squelch_level,
            ..DecoderConfig::default()
        }
    }
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct RxStatus {
    pub listening: bool,
    /// Un'immagine e' in arrivo.
    pub receiving: bool,
    /// `apt` (aspetta il tono di partenza), `phasing`, `image`.
    pub state: String,
    pub ioc: u32,
    pub lpm: f32,
    pub width: u32,
    /// Righe complete ricevute dell'immagine in corso.
    pub rows: u32,
    /// Correlazione media tra righe (0..1).
    pub sync: f32,
    pub level_db: f32,
    /// Livello del segnale per lo squelch (0..100).
    pub signal: f32,
    /// Correzione dell'AFC (Hz sopra la sottoportante delle impostazioni).
    pub afc_hz: f32,
    /// Blocchi audio buttati perche' il ricevitore non teneva il passo (dall'accensione dell'ascolto): ogni campione perso sposta il tempo
    /// e inclina l'immagine.
    pub dropped_blocks: u64,
    /// Una registrazione (file WAV) e' in decodifica: l'ascolto dal vivo e' sospeso finche' non finisce.
    pub decoding_file: bool,
    /// Avanzamento della decodifica del file (0..100).
    pub file_percent: f32,
    /// L'audio della cartina in arrivo si sta registrando.
    pub recording: bool,
    /// L'ultima immagine salvata.
    pub last_image: Option<String>,
    /// Come e' partita l'immagine in corso o l'ultima: `phasing`, `no_phasing`, `manual`, `forced`.
    pub source: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct Snapshot {
    pub rx: RxStatus,
    pub settings: WefaxSettings,
    pub folder: String,
    /// La cartella delle registrazioni (`audio` dentro quella delle cartine).
    pub audio_folder: String,
    pub iocs: Vec<crate::IocInfo>,
    pub lpm_values: Vec<u32>,
    pub filters: Vec<&'static str>,
}

enum Msg {
    Audio(u32, Vec<f32>),
    Force,
    Finish,
    Cancel,
    Config(DecoderConfig),
    ResetLearned,
    /// «Imposta inizio riga»: la colonna scelta diventa l'inizio della riga dell'immagine in corso e delle prossime.
    LineStart(u32),
    /// Comincia la decodifica di un file: la ricezione dal vivo si sospende e si parte con un decodificatore nuovo.
    FileStart,
    /// Un blocco del file (campioni mono alla frequenza del file); `done` e `total` sono i campioni per canale mandati e totali.
    FileAudio { rate: u32, samples: Vec<f32>, done: usize, total: usize },
    /// Fine del file (o fermato con `cancelled`): l'immagine in corso si chiude (o si butta) e la ricezione dal vivo riprende.
    FileEnd { cancelled: bool },
}

/// Il decodificatore della registrazione in decodifica: separato da quello dal vivo, cosi' l'errore di orologio ricordato della scheda
/// audio non si sporca con quello del file.
struct FileJob {
    decoder: Decoder,
    resampler: Option<Resampler>,
}

/// Blocchi del file messi in coda al ricevitore prima di aspettare: tengono la memoria bassa e il passo lo dà il ricevitore.
const FILE_MAX_PENDING: usize = 8;
/// Pausa dopo ogni blocco di un quarto di secondo di audio: il ricevitore va molto piu' veloce del tempo reale (decine o centinaia di volte)
/// ma non occupa un nucleo intero e il resto del programma respira.
const FILE_PAUSE: Duration = Duration::from_millis(2);

struct Inner {
    settings_path: PathBuf,
    data_dir: PathBuf,
    settings: Mutex<WefaxSettings>,
    listening: AtomicBool,
    /// Blocchi audio persi con la coda piena, dall'accensione dell'ascolto.
    dropped: AtomicU64,
    /// Una decodifica da file e' in corso: l'audio dal vivo non si manda al ricevitore.
    file_active: AtomicBool,
    /// Chi ha chiesto di fermare la decodifica del file.
    file_stop: AtomicBool,
    /// Blocchi del file in coda al ricevitore e non ancora decodificati.
    file_pending: AtomicUsize,
    queue: SyncSender<Msg>,
    rx: Mutex<RxStatus>,
    current: Mutex<Option<GrayImage>>,
    emit: Mutex<Option<Emitter>>,
    store: Mutex<ImageStore>,
    /// Le pagine WEFAX aperte: lasciata l'ultima, l'ascolto si spegne.
    users: aurora_core::users::PageUsers,
}

fn lock<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|p| p.into_inner())
}

fn folder_of(data_dir: &Path, s: &WefaxSettings) -> PathBuf {
    if s.folder.trim().is_empty() { data_dir.join("wefax") } else { PathBuf::from(s.folder.trim()) }
}

impl Inner {
    fn emit(&self, ev: WefaxEvent) {
        let emitter = lock(&self.emit).clone();
        if let Some(e) = emitter {
            e(ev);
        }
    }

    fn rx_status_event(&self) {
        let st = lock(&self.rx).clone();
        self.emit(WefaxEvent {
            kind: "rx_status".into(),
            listening: Some(st.listening),
            state: Some(st.state),
            ioc: Some(st.ioc),
            lpm: Some(st.lpm),
            rows: Some(st.rows),
            sync: Some(st.sync),
            level_db: Some(st.level_db),
            ..WefaxEvent::default()
        });
    }

    fn stop_rx(&self) {
        self.listening.store(false, Ordering::Relaxed);
        lock(&self.rx).listening = false;
        let _ = self.queue.send(Msg::Cancel);
        self.rx_status_event();
    }

    /// Scrive in `wefax.json` l'errore di orologio ricordato dal ricevitore (e la frequenza di campionamento a cui si riferisce) se e' cambiato.
    fn persist_learned(&self, ppm: f32, weight: f32, rate: u32) {
        let mut s = lock(&self.settings);
        if (s.learned_ppm - ppm).abs() < 1e-3 && (s.learned_weight - weight).abs() < 1e-3 && s.learned_rate == rate {
            return;
        }
        (s.learned_ppm, s.learned_weight, s.learned_rate) = (ppm, weight, rate);
        match serde_json::to_vec_pretty(&*s) {
            Ok(json) => {
                if let Err(e) = std::fs::write(&self.settings_path, json) {
                    tracing::warn!("WEFAX: errore di orologio ricordato non salvato: {e}");
                }
            }
            Err(e) => tracing::warn!("WEFAX: errore di orologio ricordato non salvato: {e}"),
        }
    }

    /// Nessuna pagina WEFAX aperta: l'ascolto si spegne (niente piu' audio ne' CPU), salvo «Ascolto acceso all'avvio di Aurora».
    fn page_left(&self) {
        if self.listening.load(Ordering::Relaxed) && !lock(&self.settings).auto_receive {
            tracing::info!("WEFAX: nessuna pagina aperta, ascolto spento");
            self.stop_rx();
        }
    }
}

/// Il servizio (uno per programma).
pub struct WefaxService {
    inner: Arc<Inner>,
}

impl WefaxService {
    /// Legge `wefax.json` dalla cartella dei dati e accende il thread di ricezione.
    pub fn open(data_dir: &Path) -> WefaxService {
        let settings_path = data_dir.join("wefax.json");
        let mut settings: WefaxSettings = std::fs::read(&settings_path).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or_default();
        if settings.validate().is_err() {
            settings = WefaxSettings::default();
        }
        if !settings.learned_ppm.is_finite() || !settings.learned_weight.is_finite() {
            (settings.learned_ppm, settings.learned_weight) = (0.0, 0.0);
        }
        let (queue, rx) = sync_channel::<Msg>(QUEUE_BLOCKS);
        let store = ImageStore::open(&folder_of(data_dir, &settings));
        let inner = Arc::new(Inner {
            settings_path,
            data_dir: data_dir.to_path_buf(),
            listening: AtomicBool::new(settings.auto_receive),
            dropped: AtomicU64::new(0),
            file_active: AtomicBool::new(false),
            file_stop: AtomicBool::new(false),
            file_pending: AtomicUsize::new(0),
            settings: Mutex::new(settings.clone()),
            queue,
            rx: Mutex::new(RxStatus { listening: settings.auto_receive, state: "apt".into(), ioc: settings.ioc, width: crate::width_of(settings.ioc), ..RxStatus::default() }),
            current: Mutex::new(None),
            emit: Mutex::new(None),
            store: Mutex::new(store),
            users: aurora_core::users::PageUsers::new(aurora_core::users::PAGE_TTL),
        });
        let worker = Arc::clone(&inner);
        let cfg = settings.decoder();
        if let Err(e) = std::thread::Builder::new().name("aurora-wefax".into()).spawn(move || rx_loop(&worker, rx, cfg)) {
            tracing::error!("WEFAX: thread di ricezione non avviato: {e}");
        }
        WefaxService { inner }
    }

    pub fn set_emitter(&self, emitter: Emitter) {
        *lock(&self.inner.emit) = Some(emitter);
    }

    /// Una copia dell'audio d'ingresso (frequenza della scheda). Con l'ascolto spento non costa niente.
    pub fn feed(&self, rate: u32, samples: &[f32]) {
        // con una registrazione in decodifica l'audio dal vivo non si mescola
        if !self.inner.listening.load(Ordering::Relaxed) || self.inner.file_active.load(Ordering::Relaxed) {
            return;
        }
        // coda piena (PC molto carico): si perde un blocco piuttosto che fermare l'audio dell'FT8, e si conta
        if let Err(TrySendError::Full(_)) = self.inner.queue.try_send(Msg::Audio(rate, samples.to_vec())) {
            let n = self.inner.dropped.fetch_add(1, Ordering::Relaxed) + 1;
            if n == 1 || n % 500 == 0 {
                tracing::warn!("WEFAX: blocchi audio persi: {n} (il ricevitore non tiene il passo, la cartina puo' venire inclinata)");
            }
        }
    }

    pub fn is_listening(&self) -> bool {
        self.inner.listening.load(Ordering::Relaxed)
    }

    pub fn snapshot(&self) -> Snapshot {
        let settings = lock(&self.inner.settings).clone();
        let mut rx = lock(&self.inner.rx).clone();
        rx.dropped_blocks = self.inner.dropped.load(Ordering::Relaxed);
        let (folder, audio_folder) = {
            let store = lock(&self.inner.store);
            (store.dir().display().to_string(), store.audio_dir().display().to_string())
        };
        Snapshot {
            rx,
            folder,
            audio_folder,
            iocs: crate::ioc_infos(&settings),
            lpm_values: LPM_VALUES.to_vec(),
            filters: vec!["narrow", "medium", "wide"],
            settings,
        }
    }

    pub fn settings(&self) -> WefaxSettings {
        lock(&self.inner.settings).clone()
    }

    pub fn apply_settings(&self, mut s: WefaxSettings) -> Result<(), WefaxError> {
        s.validate()?;
        // l'errore di orologio ricordato e' del ricevitore: l'interfaccia rimanda indietro il valore che ha letto, anche vecchio
        let mut current = lock(&self.inner.settings);
        (s.learned_ppm, s.learned_weight, s.learned_rate) = (current.learned_ppm, current.learned_weight, current.learned_rate);
        let json = serde_json::to_vec_pretty(&s).map_err(|e| WefaxError::Io(e.to_string()))?;
        std::fs::write(&self.inner.settings_path, json).map_err(|e| WefaxError::Io(e.to_string()))?;
        let folder = folder_of(&self.inner.data_dir, &s);
        {
            let mut store = lock(&self.inner.store);
            if store.dir() != folder {
                *store = ImageStore::open(&folder);
            }
        }
        {
            let mut rx = lock(&self.inner.rx);
            rx.ioc = s.ioc;
            rx.width = crate::width_of(s.ioc);
        }
        let _ = self.inner.queue.send(Msg::Config(s.decoder()));
        *current = s;
        Ok(())
    }

    /// Somma `ppm` alla correzione a mano dell'inclinazione (limitata a +-20000) e la applica subito: all'immagine in corso (righe nuove e
    /// gia' scritte) e alle prossime. Restituisce la correzione a mano totale.
    pub fn adjust_slant(&self, ppm: f32) -> Result<f32, WefaxError> {
        if !ppm.is_finite() {
            return Err(WefaxError::BadSettings("correzione non valida".into()));
        }
        let mut s = self.settings();
        s.slant_ppm = (s.slant_ppm + ppm).clamp(-crate::slant::MAX_PPM, crate::slant::MAX_PPM);
        let total = s.slant_ppm;
        self.apply_settings(s)?;
        Ok(total)
    }

    /// La voce di una cartina salvata.
    pub fn image_entry(&self, id: &str) -> Option<ImageEntry> {
        lock(&self.inner.store).entry(id)
    }

    /// Larghezza dell'immagine in corso (o dell'ultima finita).
    pub fn current_width(&self) -> Option<u32> {
        lock(&self.inner.current).as_ref().map(|i| i.width).filter(|w| *w > 0).or_else(|| Some(lock(&self.inner.rx).width).filter(|w| *w > 0))
    }

    /// Raddrizza una cartina salvata di `ppm` (stesso verso di `slant_ppm`), con le righe a `ref_row` ferme. Come nuova versione (nuova voce
    /// in cima alla cronologia) o, con `replace`, al posto dell'originale. Restituisce la voce scritta.
    pub fn straighten_image(&self, id: &str, ppm: f32, ref_row: Option<f64>, replace: bool) -> Result<ImageEntry, WefaxError> {
        if !ppm.is_finite() || ppm.abs() > crate::slant::MAX_PPM {
            return Err(WefaxError::BadSettings("inclinazione oltre 20000 ppm".into()));
        }
        let mut store = lock(&self.inner.store);
        let entry = store.entry(id).ok_or(WefaxError::NotFound)?;
        let img = crate::image_io::decode_png(&store.png(id)?)?;
        let out = crate::slant::straighten(&img, ppm, ref_row.unwrap_or(f64::from(img.height) / 2.0));
        if replace {
            return store.replace(id, &out);
        }
        let extra = EntryExtra { ioc: entry.ioc, lpm: entry.lpm, correlation: entry.correlation, reason: entry.reason, source: entry.source, complete: entry.complete };
        store.add(&out, extra)
    }

    /// Ruota una cartina salvata di 90 gradi (`right`: orario, altrimenti antiorario) e la riscrive al posto dell'originale (stesso file e stesso id).
    /// Larghezza e altezza della voce si scambiano; IOC e righe al minuto restano. L'immagine in ricezione non e' nell'archivio finche' non e' finita.
    pub fn rotate_image(&self, id: &str, right: bool) -> Result<ImageEntry, WefaxError> {
        let mut store = lock(&self.inner.store);
        let img = crate::image_io::decode_png(&store.png(id)?)?;
        let out = crate::image_io::rotate_quarter(&img, right);
        store.replace(id, &out)
    }

    /// Decodifica una registrazione (file WAV) come se l'audio arrivasse dalla scheda, il piu' in fretta possibile ma a blocchi e con una pausa
    /// tra l'uno e l'altro. Le cartine vanno nella cronologia anche con «Salva ogni cartina» spento. Con il file in decodifica la ricezione
    /// dal vivo e' sospesa. Torna subito: l'avanzamento si legge in `snapshot().rx` (`decoding_file`, `file_percent`).
    pub fn decode_file<B: AsRef<[u8]> + Send + 'static>(&self, bytes: B) -> Result<(), WefaxError> {
        if self.inner.file_active.swap(true, Ordering::SeqCst) {
            return Err(WefaxError::Busy);
        }
        let wav = match crate::wav::parse(bytes.as_ref()) {
            Ok(w) => w,
            Err(e) => {
                self.inner.file_active.store(false, Ordering::SeqCst);
                return Err(e);
            }
        };
        let inner = Arc::clone(&self.inner);
        inner.file_stop.store(false, Ordering::SeqCst);
        inner.file_pending.store(0, Ordering::SeqCst);
        {
            let mut st = lock(&inner.rx);
            st.decoding_file = true;
            st.file_percent = 0.0;
        }
        let _ = inner.queue.send(Msg::FileStart);
        let worker = Arc::clone(&inner);
        let spawned = std::thread::Builder::new().name("aurora-wefax-file".into()).spawn(move || feed_file(&worker, &wav, bytes.as_ref()));
        if let Err(e) = spawned {
            let _ = inner.queue.send(Msg::FileEnd { cancelled: true });
            return Err(WefaxError::Io(e.to_string()));
        }
        inner.rx_status_event();
        Ok(())
    }

    /// Ferma la decodifica del file: l'immagine in corso si butta e la ricezione dal vivo riprende.
    pub fn stop_file(&self) {
        self.inner.file_stop.store(true, Ordering::SeqCst);
    }

    /// Porta la colonna `x` (pixel dell'immagine in corso) all'inizio di ogni riga, da adesso in poi: la cartina che arriva spostata di lato si
    /// rimette a posto senza ricominciare.
    pub fn set_line_start(&self, x: u32) {
        let _ = self.inner.queue.send(Msg::LineStart(x));
    }

    /// Sposta una cartina salvata: la colonna `x` diventa l'inizio delle righe. Come nuova versione o, con `replace`, al posto dell'originale.
    pub fn shift_saved_image(&self, id: &str, x: u32, replace: bool) -> Result<ImageEntry, WefaxError> {
        let mut store = lock(&self.inner.store);
        let entry = store.entry(id).ok_or(WefaxError::NotFound)?;
        let img = crate::image_io::decode_png(&store.png(id)?)?;
        let out = crate::slant::shift_left(&img, x);
        if replace {
            return store.replace(id, &out);
        }
        let extra = EntryExtra { ioc: entry.ioc, lpm: entry.lpm, correlation: entry.correlation, reason: entry.reason, source: entry.source, complete: entry.complete };
        store.add(&out, extra)
    }

    /// Dimentica l'errore di orologio ricordato (si cambia scheda audio o impostazioni del sistema).
    pub fn reset_learned(&self) {
        let _ = self.inner.queue.send(Msg::ResetLearned);
    }

    pub fn start_rx(&self) {
        // un ascolto nuovo riparte dal conteggio dei blocchi persi
        if !self.inner.listening.swap(true, Ordering::Relaxed) {
            self.inner.dropped.store(0, Ordering::Relaxed);
        }
        lock(&self.inner.rx).listening = true;
        self.inner.rx_status_event();
    }

    pub fn stop_rx(&self) {
        self.inner.stop_rx();
    }

    /// Un battito della pagina WEFAX `id`: finche' qualcuna batte, l'ascolto non si spegne da solo.
    pub fn hold_rx(&self, id: &str) {
        self.inner.users.hold(id);
    }

    /// La pagina `id` e' stata lasciata: se era l'ultima l'ascolto si spegne, salvo «Ascolto acceso all'avvio».
    pub fn release_rx(&self, id: &str) {
        if self.inner.users.release(id) {
            self.inner.page_left();
        }
    }

    /// Comincia subito un'immagine (APT e fasatura saltati), con l'ascolto acceso.
    pub fn force_rx(&self) {
        self.start_rx();
        let _ = self.inner.queue.send(Msg::Force);
    }

    /// Chiude e salva l'immagine in corso com'e' (serve in modo manuale, dove non c'e' un APT stop).
    pub fn finish_rx(&self) {
        let _ = self.inner.queue.send(Msg::Finish);
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

    pub fn image_png(&self, id: &str) -> Result<Vec<u8>, WefaxError> {
        lock(&self.inner.store).png(id)
    }

    /// La registrazione WAV di una cartina (`NotFound` se non c'e').
    pub fn image_audio(&self, id: &str) -> Result<Vec<u8>, WefaxError> {
        lock(&self.inner.store).audio(id)
    }

    /// La cartella delle cartine e quella delle registrazioni.
    pub fn folders(&self) -> (PathBuf, PathBuf) {
        let store = lock(&self.inner.store);
        (store.dir().to_path_buf(), store.audio_dir())
    }

    pub fn delete_image(&self, id: &str) -> Result<(), WefaxError> {
        lock(&self.inner.store).delete(id)
    }

    /// Cancella piu' immagini dell'archivio in una volta. Restituisce quante ne ha cancellate.
    pub fn delete_images(&self, ids: &[String]) -> usize {
        let mut store = lock(&self.inner.store);
        ids.iter().filter(|id| store.delete(id).is_ok()).count()
    }
}

fn rows_event(width: u32, total_rows: u32, first: u32, count: u32, pixels: &[u8]) -> WefaxEvent {
    let from = (first as usize * width as usize).min(pixels.len());
    let to = ((first + count) as usize * width as usize).min(pixels.len());
    WefaxEvent {
        kind: "rx_rows".into(),
        width: Some(width),
        height: Some(total_rows),
        first_row: Some(first),
        rows: Some(count),
        gray: Some(base64::engine::general_purpose::STANDARD.encode(&pixels[from..to])),
        ..WefaxEvent::default()
    }
}

/// Il file a blocchi di un quarto di secondo verso il ricevitore, con una pausa dopo ciascuno e senza mai averne piu' di `FILE_MAX_PENDING`
/// in coda. Alla fine (o fermato) manda `FileEnd`.
fn feed_file(inner: &Arc<Inner>, wav: &crate::wav::Wav, bytes: &[u8]) {
    let total = wav.frames();
    let step = (wav.rate as usize / 4).max(1);
    let mut pos = 0usize;
    let mut cancelled = false;
    'feed: while pos < total {
        while inner.file_pending.load(Ordering::Relaxed) >= FILE_MAX_PENDING {
            if inner.file_stop.load(Ordering::Relaxed) {
                cancelled = true;
                break 'feed;
            }
            std::thread::sleep(FILE_PAUSE);
        }
        if inner.file_stop.load(Ordering::Relaxed) {
            cancelled = true;
            break;
        }
        let n = step.min(total - pos);
        let mut samples = Vec::with_capacity(n);
        wav.read(bytes, pos, n, &mut samples);
        pos += n;
        inner.file_pending.fetch_add(1, Ordering::Relaxed);
        if inner.queue.send(Msg::FileAudio { rate: wav.rate, samples, done: pos, total }).is_err() {
            // il ricevitore non c'e' piu': non resta che rimettere a posto lo stato
            inner.file_active.store(false, Ordering::SeqCst);
            lock(&inner.rx).decoding_file = false;
            return;
        }
        std::thread::sleep(FILE_PAUSE);
    }
    if inner.queue.send(Msg::FileEnd { cancelled }).is_err() {
        inner.file_active.store(false, Ordering::SeqCst);
        lock(&inner.rx).decoding_file = false;
    }
}

/// Ogni quanto lo stato va all'interfaccia mentre si ascolta.
const STATUS_EVERY: Duration = Duration::from_millis(500);

fn rx_loop(inner: &Arc<Inner>, queue: Receiver<Msg>, cfg: DecoderConfig) {
    let mut decoder = Decoder::new(cfg);
    let mut rate_seen = lock(&inner.settings).learned_rate;
    let mut resampler: Option<Resampler> = None;
    let mut audio = Vec::new();
    let mut file: Option<FileJob> = None;
    let mut last_status = Instant::now();
    let mut last_users = Instant::now();
    // la registrazione dell'audio delle cartine (solo dal vivo, mai dei file): all'avvio si puliscono le registrazioni vecchie
    let mut recorder = Recorder::new();
    {
        let keep = lock(&inner.settings).audio_keep_days;
        let dir = lock(&inner.store).audio_dir();
        recorder.prune_later(&dir, keep);
    }
    loop {
        let msg = match queue.recv_timeout(Duration::from_millis(500)) {
            Ok(m) => Some(m),
            Err(RecvTimeoutError::Timeout) => None,
            Err(RecvTimeoutError::Disconnected) => return,
        };
        let mut events = Vec::new();
        match msg {
            // con una registrazione in decodifica l'audio dal vivo e' sospeso (qualche blocco gia' in coda si butta)
            Some(Msg::Audio(..)) if inner.file_active.load(Ordering::Relaxed) => {}
            Some(Msg::Audio(rate, samples)) => {
                // l'errore di orologio ricordato vale per una frequenza di campionamento: cambiata quella (altra scheda), si riparte
                if rate != rate_seen {
                    if rate_seen != 0 {
                        decoder.reset_learned();
                        tracing::info!("WEFAX: frequenza di campionamento da {rate_seen} a {rate} Hz, errore di orologio ricordato azzerato");
                    }
                    rate_seen = rate;
                    inner.persist_learned(decoder.learned_ppm(), decoder.learned_weight(), rate);
                }
                let r = match &mut resampler {
                    Some(r) if r.in_rate() == rate => r,
                    slot => slot.insert(Resampler::new(rate)),
                };
                audio.clear();
                r.process(&samples, &mut audio);
                recorder.push(&audio);
                events = decoder.push(&audio);
            }
            Some(Msg::Force) => {
                decoder.force_start();
                events = decoder.take_events();
            }
            Some(Msg::Finish) => events = decoder.finish(),
            Some(Msg::Cancel) => {
                decoder.cancel();
                events = decoder.take_events();
                recorder.abort();
            }
            Some(Msg::Config(c)) => {
                if let Some(job) = file.as_mut() {
                    job.decoder.set_config(c.clone());
                    for ev in job.decoder.take_events() {
                        handle(inner, &job.decoder, ev, true);
                    }
                }
                decoder.set_config(c);
                events = decoder.take_events();
            }
            Some(Msg::FileStart) => {
                // l'immagine dal vivo in arrivo si butta; il file parte con un decodificatore nuovo, senza l'errore di orologio ricordato
                decoder.cancel();
                events = decoder.take_events();
                recorder.abort();
                let mut c = lock(&inner.settings).decoder();
                (c.learned_ppm, c.learned_weight) = (0.0, 0.0);
                file = Some(FileJob { decoder: Decoder::new(c), resampler: None });
            }
            Some(Msg::FileAudio { rate, samples, done, total }) => {
                inner.file_pending.fetch_sub(1, Ordering::Relaxed);
                if let Some(job) = file.as_mut() {
                    let r = match &mut job.resampler {
                        Some(r) if r.in_rate() == rate => r,
                        slot => slot.insert(Resampler::new(rate)),
                    };
                    audio.clear();
                    r.process(&samples, &mut audio);
                    for ev in job.decoder.push(&audio) {
                        handle(inner, &job.decoder, ev, true);
                    }
                    lock(&inner.rx).file_percent = (done as f32 / total.max(1) as f32 * 100.0).min(100.0);
                }
            }
            Some(Msg::FileEnd { cancelled }) => {
                if let Some(mut job) = file.take() {
                    let evs = if cancelled {
                        job.decoder.cancel();
                        job.decoder.take_events()
                    } else {
                        job.decoder.finish()
                    };
                    for ev in evs {
                        handle(inner, &job.decoder, ev, true);
                    }
                }
                {
                    let mut st = lock(&inner.rx);
                    st.decoding_file = false;
                    st.file_percent = 0.0;
                    st.receiving = false;
                }
                inner.file_active.store(false, Ordering::SeqCst);
                inner.rx_status_event();
            }
            Some(Msg::LineStart(x)) => {
                decoder.set_line_start(x);
            }
            Some(Msg::ResetLearned) => {
                decoder.reset_learned();
                inner.persist_learned(0.0, 0.0, rate_seen);
            }
            None => {}
        }
        for ev in events {
            let finished = matches!(ev, DecoderEvent::Finished(_));
            // la registrazione parte con il tono d'inizio (fasatura) o con l'immagine (manuale, forzata, squelch) e si butta se la cartina
            // si scarta o si torna ad aspettare senza una cartina finita (falso allarme)
            match &ev {
                DecoderEvent::State("phasing" | "image") | DecoderEvent::Started { .. } if !recorder.is_active() => start_recording(inner, &mut recorder),
                DecoderEvent::State("apt") | DecoderEvent::Discarded { .. } => recorder.abort(),
                _ => {}
            }
            let saved = handle(inner, &decoder, ev, false);
            if finished {
                end_recording(inner, &mut recorder, saved);
                inner.persist_learned(decoder.learned_ppm(), decoder.learned_weight(), rate_seen);
            }
        }
        // le pagine sparite senza salutare (finestra chiusa, rete caduta) escono da sole
        if last_users.elapsed() >= Duration::from_secs(1) {
            last_users = Instant::now();
            if inner.users.expire() {
                inner.page_left();
            }
        }
        // lo stato va all'interfaccia solo con l'ascolto acceso (o con un file in decodifica: allora e' quello del file)
        if last_status.elapsed() >= STATUS_EVERY && (inner.listening.load(Ordering::Relaxed) || file.is_some()) {
            last_status = Instant::now();
            {
                let shown = file.as_ref().map_or(&decoder, |j| &j.decoder);
                let mut st = lock(&inner.rx);
                st.level_db = shown.level_db();
                st.signal = shown.signal_level();
                st.afc_hz = shown.afc_hz();
                st.receiving = shown.is_receiving();
                st.sync = shown.correlation();
                st.state = shown.rx_state().into();
                st.lpm = shown.lpm();
                // l'IOC riconosciuto dal tono d'inizio (l'impostazione resta com'e')
                st.ioc = shown.config().ioc;
                st.width = shown.config().width();
                st.recording = file.is_none() && recorder.is_active();
            }
            inner.rx_status_event();
        }
    }
}

/// Un avvenimento del decodificatore verso lo stato e l'interfaccia. `force_save`: la cartina si salva anche con «Salva ogni cartina» spento
/// (le registrazioni decodificate da file vanno sempre nella cronologia).
/// Restituisce l'id della cartina salvata (solo per `Finished`).
fn handle(inner: &Arc<Inner>, decoder: &Decoder, ev: DecoderEvent, force_save: bool) -> Option<String> {
    let mut saved_id = None;
    match ev {
        DecoderEvent::Started { ioc, lpm, width, source } => {
            {
                let mut st = lock(&inner.rx);
                st.receiving = true;
                st.ioc = ioc;
                st.lpm = lpm;
                st.width = width;
                st.rows = 0;
                st.source = Some(source.id().into());
            }
            *lock(&inner.current) = Some(GrayImage { width, height: 0, data: Vec::new() });
            inner.emit(WefaxEvent { kind: "rx_start".into(), ioc: Some(ioc), lpm: Some(lpm), width: Some(width), message: Some(source.id().into()), ..WefaxEvent::default() });
        }
        DecoderEvent::Rows { first, count } => {
            if let Some((width, rows, pixels)) = decoder.current() {
                *lock(&inner.current) = Some(GrayImage { width, height: rows, data: pixels.to_vec() });
                lock(&inner.rx).rows = rows;
                inner.emit(rows_event(width, rows, first, count, pixels));
            }
        }
        DecoderEvent::Finished(done) => {
            let auto_save = force_save || lock(&inner.settings).auto_save;
            let saved = if auto_save {
                let extra = EntryExtra { ioc: done.ioc, lpm: done.lpm, correlation: Some(done.correlation), reason: Some(done.reason.clone()), source: Some(done.source.id().into()), complete: done.complete };
                match lock(&inner.store).add(&done.image, extra) {
                    Ok(e) => Some(e.id),
                    Err(e) => {
                        tracing::warn!("WEFAX: immagine non salvata: {e}");
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
                if saved.is_some() {
                    st.last_image = saved.clone();
                }
            }
            // l'immagine finale (senza le righe del tono di stop) si rimanda tutta
            let (w, h) = (done.image.width, done.image.height);
            inner.emit(rows_event(w, h, 0, h, &done.image.data));
            *lock(&inner.current) = Some(done.image);
            saved_id.clone_from(&saved);
            inner.emit(WefaxEvent { kind: "rx_done".into(), image_id: saved, width: Some(w), height: Some(h), ioc: Some(done.ioc), lpm: Some(done.lpm), sync: Some(done.correlation), complete: Some(done.complete), message: Some(done.reason), ..WefaxEvent::default() });
        }
        DecoderEvent::Discarded { reason } => {
            lock(&inner.rx).receiving = false;
            inner.emit(WefaxEvent { kind: "rx_lost".into(), message: Some(reason), ..WefaxEvent::default() });
        }
        DecoderEvent::IocDetected { from, ioc, lpm, width } => {
            {
                let mut st = lock(&inner.rx);
                st.ioc = ioc;
                st.lpm = lpm;
                st.width = width;
            }
            tracing::info!("WEFAX: tono d'inizio dell'IOC {ioc}, si passa da {from} a {ioc} ({width} colonne)");
            inner.emit(WefaxEvent { kind: "ioc_auto".into(), ioc: Some(ioc), lpm: Some(lpm), width: Some(width), message: Some(format!("{from}")), ..WefaxEvent::default() });
        }
        DecoderEvent::State(state) => {
            lock(&inner.rx).state = state.into();
            inner.rx_status_event();
        }
    }
    saved_id
}

/// Comincia a registrare l'audio della cartina che sta arrivando (se la registrazione e' accesa).
fn start_recording(inner: &Arc<Inner>, recorder: &mut Recorder) {
    let (on, keep) = {
        let s = lock(&inner.settings);
        (s.record_audio, s.audio_keep_days)
    };
    if !on {
        return;
    }
    let dir = lock(&inner.store).audio_dir();
    recorder.begin(&dir, &crate::store::stamp(crate::store::now_ms()), keep);
}

/// Chiude la registrazione: il WAV prende il nome della cartina salvata e le si lega; senza cartina salvata resta con la data di chiusura.
fn end_recording(inner: &Arc<Inner>, recorder: &mut Recorder, saved: Option<String>) {
    if !recorder.is_active() {
        return;
    }
    match saved {
        Some(id) => {
            let weak = Arc::downgrade(inner);
            let name = crate::recorder::audio_name_for(&id);
            recorder.end(
                name,
                Box::new(move |written| {
                    let (Some(name), Some(inner)) = (written, weak.upgrade()) else { return };
                    if let Err(e) = lock(&inner.store).set_audio(&id, &name) {
                        tracing::warn!("WEFAX: registrazione {name} non legata alla cartina: {e}");
                    }
                }),
            );
        }
        None => recorder.end(format!("wefax-{}.wav", crate::store::stamp(crate::store::now_ms())), Box::new(|_| {})),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("aurora-wefax-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("cartella di prova");
        dir
    }

    #[test]
    fn l_ascolto_si_spegne_solo_quando_esce_l_ultima_pagina() {
        let data = temp("pagine");
        let svc = WefaxService::open(&data);
        svc.hold_rx("principale");
        svc.hold_rx("telefono");
        svc.start_rx();
        svc.release_rx("telefono");
        assert!(svc.is_listening());
        svc.release_rx("principale");
        assert!(!svc.is_listening());
        let _ = std::fs::remove_dir_all(&data);
    }

    #[test]
    fn le_impostazioni_si_validano_e_si_salvano() {
        let data = temp("impost");
        let svc = WefaxService::open(&data);
        let mut s = svc.settings();
        s.ioc = 300;
        assert!(svc.apply_settings(s).is_err());
        let mut s = svc.settings();
        s.ioc = 288;
        s.lpm_288 = 90;
        s.filter = Filter::Wide;
        svc.apply_settings(s.clone()).expect("impostazioni");
        let again = WefaxService::open(&data);
        assert_eq!(again.settings(), s);
        assert_eq!(again.snapshot().rx.width, 904);
        let _ = std::fs::remove_dir_all(&data);
    }

    /// Aspetta fino a due secondi che il ricevitore (un altro thread) faccia quello che e' stato chiesto.
    fn attendi(mut fatto: impl FnMut() -> bool) -> bool {
        for _ in 0..200 {
            if fatto() {
                return true;
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        false
    }

    #[test]
    fn l_errore_di_orologio_ricordato_si_salva_non_si_cambia_da_fuori_e_si_azzera() {
        let data = temp("ricordato");
        let svc = WefaxService::open(&data);
        svc.inner.persist_learned(1500.0, 3.0, 48_000);
        assert_eq!(svc.settings().learned_ppm, 1500.0);
        // l'interfaccia rimanda indietro i valori che ha letto (anche vecchi) o ne scrive altri: il valore resta quello del ricevitore
        let mut s = svc.settings();
        s.learned_ppm = 0.0;
        s.learned_weight = 0.0;
        s.slant_ppm = 12.0;
        svc.apply_settings(s).expect("impostazioni");
        assert_eq!(svc.settings().learned_ppm, 1500.0);
        assert_eq!(svc.settings().slant_ppm, 12.0);
        // e' nel file: al prossimo avvio c'e' ancora
        let again = WefaxService::open(&data);
        assert_eq!((again.settings().learned_ppm, again.settings().learned_rate), (1500.0, 48_000));
        drop(again);
        // l'azzeramento lo fa il ricevitore e lo scrive nel file
        svc.reset_learned();
        assert!(attendi(|| svc.settings().learned_ppm == 0.0), "azzeramento non arrivato");
        assert_eq!(WefaxService::open(&data).settings().learned_ppm, 0.0);
        let _ = std::fs::remove_dir_all(&data);
    }

    #[test]
    fn cambiata_la_frequenza_di_campionamento_l_errore_ricordato_si_azzera_ma_non_all_avvio() {
        let data = temp("rate");
        let svc = WefaxService::open(&data);
        svc.inner.persist_learned(800.0, 2.0, 48_000);
        svc.start_rx();
        // stessa frequenza di prima (dopo un riavvio di Aurora): resta
        svc.feed(48_000, &vec![0.0; 4800]);
        std::thread::sleep(Duration::from_millis(100));
        assert_eq!(svc.settings().learned_ppm, 800.0);
        // un'altra scheda a 44100 Hz: si riparte
        svc.feed(44_100, &vec![0.0; 4410]);
        assert!(attendi(|| svc.settings().learned_ppm == 0.0 && svc.settings().learned_rate == 44_100), "reset non arrivato");
        let _ = std::fs::remove_dir_all(&data);
    }

    #[test]
    fn i_blocchi_audio_persi_si_contano_senza_mai_bloccare_chi_alimenta() {
        let data = temp("persi");
        let svc = WefaxService::open(&data);
        svc.start_rx();
        // alimentazione normale: un blocco da 10 ms ogni 10 ms per due secondi, niente di perso
        for _ in 0..200 {
            svc.feed(48_000, &vec![0.1; 480]);
            std::thread::sleep(Duration::from_millis(10));
        }
        assert_eq!(svc.snapshot().rx.dropped_blocks, 0, "con l'alimentazione normale non si perde niente");
        // molto piu' veloce del ricevitore: blocchi da 100 ms a raffica, la coda si riempie, il contatore sale e feed non aspetta
        let t = Instant::now();
        for _ in 0..3000 {
            svc.feed(48_000, &vec![0.1; 4800]);
        }
        let tempo = t.elapsed();
        let persi = svc.snapshot().rx.dropped_blocks;
        eprintln!("3000 blocchi da 100 ms in {tempo:?}: persi {persi}");
        assert!(persi > 0, "il contatore doveva salire");
        assert!(tempo < Duration::from_secs(10), "feed ha aspettato: {tempo:?}");
        // un ascolto nuovo riparte da zero
        svc.stop_rx();
        svc.start_rx();
        assert_eq!(svc.snapshot().rx.dropped_blocks, 0);
        let _ = std::fs::remove_dir_all(&data);
    }

    #[test]
    fn le_impostazioni_vecchie_senza_afc_ne_squelch_si_leggono() {
        let old = r#"{"ioc":576,"lpm_576":120,"lpm_288":60,"filter":"narrow","manual":false,"center_hz":1900.0,"shift_hz":800.0}"#;
        let s: WefaxSettings = serde_json::from_str(old).expect("impostazioni vecchie");
        assert!(s.afc);
        assert!(s.auto_ioc);
        assert_eq!(s.afc_range_hz, 150.0);
        assert!(!s.squelch);
        assert_eq!(s.squelch_level, 30.0);
        assert!(s.validate().is_ok());
        let mut bad = s.clone();
        bad.squelch_level = 120.0;
        assert!(bad.validate().is_err());
        for fuori in [10.0, 450.0, f32::NAN] {
            let mut bad = s.clone();
            bad.afc_range_hz = fuori;
            assert!(bad.validate().is_err(), "limite {fuori}");
        }
        let mut ok = s.clone();
        ok.afc_range_hz = 400.0;
        assert!(ok.validate().is_ok());
    }

    #[test]
    fn la_correzione_a_mano_si_somma_si_limita_e_raddrizza_le_cartine_salvate() {
        let data = temp("inclinazione");
        let svc = WefaxService::open(&data);
        assert_eq!(svc.adjust_slant(120.0).expect("somma"), 120.0);
        assert_eq!(svc.adjust_slant(-20.0).expect("somma"), 100.0);
        assert_eq!(svc.settings().slant_ppm, 100.0);
        assert_eq!(svc.adjust_slant(50_000.0).expect("somma"), 20_000.0);
        assert!(svc.adjust_slant(f32::NAN).is_err());
        // cartina salvata con una linea inclinata di 0,5 px per riga: come nuova versione l'originale resta, con `replace` si sostituisce
        let (w, h) = (400u32, 300u32);
        let mut img = GrayImage { width: w, height: h, data: vec![230; (w * h) as usize] };
        for r in 0..h as usize {
            let x = 50 + (0.5 * r as f64) as usize;
            img.data[r * w as usize + x] = 20;
        }
        let entry = lock(&svc.inner.store).add(&img, EntryExtra { ioc: 576, lpm: 120.0, complete: true, ..EntryExtra::default() }).expect("salva");
        let ppm = 0.5 / f64::from(w) * 1e6;
        let nuova = svc.straighten_image(&entry.id, ppm as f32, Some(0.0), false).expect("nuova versione");
        assert_ne!(nuova.id, entry.id);
        assert_eq!(svc.images().len(), 2);
        let png = crate::image_io::decode_png(&svc.image_png(&nuova.id).expect("png")).expect("lettura");
        let scura = |r: usize| (0..w as usize).min_by_key(|&c| png.data[r * w as usize + c]).unwrap_or(0) as i64;
        assert!((scura(10) - scura(280)).abs() <= 2, "{} contro {}", scura(10), scura(280));
        // l'originale e' rimasto storto
        let orig = crate::image_io::decode_png(&svc.image_png(&entry.id).expect("png")).expect("lettura");
        assert_eq!(orig, img);
        let sost = svc.straighten_image(&entry.id, ppm as f32, Some(0.0), true).expect("sostituita");
        assert_eq!(sost.id, entry.id);
        assert_eq!(svc.images().len(), 2);
        assert!(svc.straighten_image("non-esiste", 10.0, None, false).is_err());
        let _ = std::fs::remove_dir_all(&data);
    }

    #[test]
    fn la_rotazione_di_una_cartina_salvata_scambia_le_misure_e_torna_indietro() {
        let data = temp("rotazione");
        let svc = WefaxService::open(&data);
        // 3 x 2 con un riferimento riconoscibile: ogni pixel diverso
        let img = GrayImage { width: 3, height: 2, data: vec![1, 2, 3, 4, 5, 6] };
        let entry = lock(&svc.inner.store).add(&img, EntryExtra { ioc: 576, lpm: 120.0, complete: true, ..EntryExtra::default() }).expect("salva");
        let leggi = |id: &str| crate::image_io::decode_png(&svc.image_png(id).expect("png")).expect("lettura");
        let r = svc.rotate_image(&entry.id, true).expect("destra");
        assert_eq!((r.id.as_str(), r.width, r.height, r.ioc, r.lpm), (entry.id.as_str(), 2, 3, 576, 120.0));
        assert_eq!(leggi(&entry.id).data, vec![4, 1, 5, 2, 6, 3]);
        assert_eq!(svc.image_entry(&entry.id).map(|e| (e.width, e.height)), Some((2, 3)));
        let l = svc.rotate_image(&entry.id, false).expect("sinistra");
        assert_eq!((l.width, l.height), (3, 2));
        assert_eq!(leggi(&entry.id), img);
        svc.rotate_image(&entry.id, false).expect("sinistra");
        assert_eq!(leggi(&entry.id).data, vec![3, 6, 2, 5, 1, 4]);
        for _ in 0..3 {
            svc.rotate_image(&entry.id, false).expect("sinistra");
        }
        assert_eq!(leggi(&entry.id), img);
        for _ in 0..4 {
            svc.rotate_image(&entry.id, true).expect("destra");
        }
        assert_eq!(leggi(&entry.id), img);
        assert_eq!(svc.images().len(), 1);
        assert_eq!(svc.rotate_image("non-esiste", true), Err(WefaxError::NotFound));
        assert_eq!(svc.rotate_image("../index", true), Err(WefaxError::NotFound));
        let _ = std::fs::remove_dir_all(&data);
    }
}
