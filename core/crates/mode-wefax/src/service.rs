//! Il servizio WEFAX: riceve una copia dell'audio d'ingresso (solo quando l'ascolto e' acceso: l'FT8 non se ne accorge), decodifica in un
//! thread suo, manda le righe appena arrivate all'interfaccia e salva le cartine finite. Impostazioni in `wefax.json`, cartine in `wefax/`
//! (o nella cartella scelta). Solo ricezione.

use std::{
    path::{Path, PathBuf},
    sync::{
        Arc, Mutex, MutexGuard,
        atomic::{AtomicBool, Ordering},
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
    resample::Resampler,
    store::{EntryExtra, ImageEntry, ImageStore},
};

pub type Emitter = Arc<dyn Fn(WefaxEvent) + Send + Sync>;

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
    /// Squelch (SQL): l'immagine parte quando il segnale supera `squelch_level` e si chiude quando sparisce per 3 secondi.
    pub squelch: bool,
    /// Soglia dello squelch (0..100; 100 = 20 dB sopra il rumore).
    pub squelch_level: f32,
}

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
            squelch: d.squelch,
            squelch_level: d.squelch_level,
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
        if !self.squelch_level.is_finite() || !(0.0..=100.0).contains(&self.squelch_level) {
            return bad("soglia dello squelch tra 0 e 100");
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
}

struct Inner {
    settings_path: PathBuf,
    data_dir: PathBuf,
    settings: Mutex<WefaxSettings>,
    listening: AtomicBool,
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
        let (queue, rx) = sync_channel::<Msg>(128);
        let store = ImageStore::open(&folder_of(data_dir, &settings));
        let inner = Arc::new(Inner {
            settings_path,
            data_dir: data_dir.to_path_buf(),
            listening: AtomicBool::new(settings.auto_receive),
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
        if !self.inner.listening.load(Ordering::Relaxed) {
            return;
        }
        // coda piena (PC molto carico): si perde un blocco piuttosto che fermare l'audio dell'FT8
        if let Err(TrySendError::Full(_)) = self.inner.queue.try_send(Msg::Audio(rate, samples.to_vec())) {
            tracing::debug!("WEFAX: blocco audio perso");
        }
    }

    pub fn is_listening(&self) -> bool {
        self.inner.listening.load(Ordering::Relaxed)
    }

    pub fn snapshot(&self) -> Snapshot {
        let settings = lock(&self.inner.settings).clone();
        Snapshot {
            rx: lock(&self.inner.rx).clone(),
            folder: lock(&self.inner.store).dir().display().to_string(),
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

    /// Dimentica l'errore di orologio ricordato (si cambia scheda audio o impostazioni del sistema).
    pub fn reset_learned(&self) {
        let _ = self.inner.queue.send(Msg::ResetLearned);
    }

    pub fn start_rx(&self) {
        self.inner.listening.store(true, Ordering::Relaxed);
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

/// Ogni quanto lo stato va all'interfaccia mentre si ascolta.
const STATUS_EVERY: Duration = Duration::from_millis(500);

fn rx_loop(inner: &Arc<Inner>, queue: Receiver<Msg>, cfg: DecoderConfig) {
    let mut decoder = Decoder::new(cfg);
    let mut rate_seen = lock(&inner.settings).learned_rate;
    let mut resampler: Option<Resampler> = None;
    let mut audio = Vec::new();
    let mut last_status = Instant::now();
    let mut last_users = Instant::now();
    loop {
        let msg = match queue.recv_timeout(Duration::from_millis(500)) {
            Ok(m) => Some(m),
            Err(RecvTimeoutError::Timeout) => None,
            Err(RecvTimeoutError::Disconnected) => return,
        };
        let mut events = Vec::new();
        match msg {
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
            }
            Some(Msg::Config(c)) => {
                decoder.set_config(c);
                events = decoder.take_events();
            }
            Some(Msg::ResetLearned) => {
                decoder.reset_learned();
                inner.persist_learned(0.0, 0.0, rate_seen);
            }
            None => {}
        }
        for ev in events {
            let finished = matches!(ev, DecoderEvent::Finished(_));
            handle(inner, &decoder, ev);
            if finished {
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
        // lo stato va all'interfaccia solo con l'ascolto acceso
        if last_status.elapsed() >= STATUS_EVERY && inner.listening.load(Ordering::Relaxed) {
            last_status = Instant::now();
            {
                let mut st = lock(&inner.rx);
                st.level_db = decoder.level_db();
                st.signal = decoder.signal_level();
                st.afc_hz = decoder.afc_hz();
                st.receiving = decoder.is_receiving();
                st.sync = decoder.correlation();
                st.state = decoder.rx_state().into();
                st.lpm = decoder.lpm();
                // l'IOC riconosciuto dal tono d'inizio (l'impostazione resta com'e')
                st.ioc = decoder.config().ioc;
                st.width = decoder.config().width();
            }
            inner.rx_status_event();
        }
    }
}

fn handle(inner: &Arc<Inner>, decoder: &Decoder, ev: DecoderEvent) {
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
            let auto_save = lock(&inner.settings).auto_save;
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
    fn le_impostazioni_vecchie_senza_afc_ne_squelch_si_leggono() {
        let old = r#"{"ioc":576,"lpm_576":120,"lpm_288":60,"filter":"narrow","manual":false,"center_hz":1900.0,"shift_hz":800.0}"#;
        let s: WefaxSettings = serde_json::from_str(old).expect("impostazioni vecchie");
        assert!(s.afc);
        assert!(s.auto_ioc);
        assert!(!s.squelch);
        assert_eq!(s.squelch_level, 30.0);
        assert!(s.validate().is_ok());
        let mut bad = s.clone();
        bad.squelch_level = 120.0;
        assert!(bad.validate().is_err());
    }
}
