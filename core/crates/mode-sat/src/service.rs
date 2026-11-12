//! Il servizio dei satelliti: tiene i TLE, calcola ogni secondo dove sta il satellite scelto e, se acceso, **corregge il Doppler sulla radio**
//! (ricezione e trasmissione, anche con il transponder invertente) e **fa seguire il satellite al rotore** in azimut ed elevazione.
//! Impostazioni in `satelliti.json`; i satelliti dell'utente in `satelliti-utente.json`; le mappe nella cartella `maps`.

use std::{
    path::PathBuf,
    sync::{
        Arc, Mutex, MutexGuard,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use serde::{Deserialize, Serialize};

use crate::{
    catalog::{self, SatInfo, Transponder},
    orbit::{self, Look, Observer, Pass, Sat},
    tle::{self, TleFile},
};

/// La radio vista dai satelliti (la fornisce il programma: Hamlib con split).
pub trait SatRadio: Send + Sync {
    fn ready(&self) -> bool;
    /// La frequenza di ricezione (VFO principale), in Hz.
    fn set_rx_hz(&self, hz: u64) -> Result<(), String>;
    /// La frequenza di trasmissione (VFO di split), in Hz.
    fn set_tx_hz(&self, hz: u64) -> Result<(), String>;
    fn set_split(&self, on: bool) -> Result<(), String>;
}

/// Il rotore: azimut ed elevazione.
pub trait SatRotator: Send + Sync {
    fn ready(&self) -> bool;
    fn point(&self, az_deg: f64, el_deg: f64) -> Result<(), String>;
    fn stop(&self);
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct SatSettings {
    /// Il satellite che si segue (NORAD).
    pub selected: Option<u64>,
    /// I satelliti spuntati: compaiono sulla mappa.
    pub tracked: Vec<u64>,
    /// Quale transponder del satellite scelto.
    pub transponder: usize,
    /// La frequenza di discesa scelta dentro il transponder (Hz, senza Doppler); vuoto = il centro.
    pub down_hz: Option<f64>,
    /// Correzione fine della frequenza di salita (Hz).
    pub tx_offset_hz: f64,
    pub doppler_on: bool,
    pub rotor_on: bool,
    /// Prima del passaggio il rotore va all'azimut da cui il satellite sorge.
    pub rotor_preposition: bool,
    /// Sotto questa elevazione non si corregge il Doppler e il rotore non segue.
    pub min_el_deg: f64,
    /// La mappa del mondo (nome del file in `maps`).
    pub map: String,
    /// I server da cui si scaricano i dati delle orbite (indirizzi http/https con TLE a tre righe).
    pub tle_sources: Vec<String>,
    /// Ogni quante ore si aggiornano da soli i dati delle orbite; 0 = solo a mano.
    pub tle_update_hours: u32,
}

impl Default for SatSettings {
    fn default() -> Self {
        SatSettings { selected: Some(25544), tracked: vec![25544, 44909], transponder: 0, down_hz: None, tx_offset_hz: 0.0, doppler_on: false, rotor_on: false, rotor_preposition: true, min_el_deg: 0.0, map: String::new(), tle_sources: tle::SOURCES.iter().map(|u| (*u).to_string()).collect(), tle_update_hours: 24 }
    }
}

impl SatSettings {
    fn validate(&self) -> Result<(), String> {
        if !self.min_el_deg.is_finite() || !(0.0..=60.0).contains(&self.min_el_deg) {
            return Err("elevazione minima fuori da 0-60 gradi".into());
        }
        if !self.tx_offset_hz.is_finite() || self.tx_offset_hz.abs() > 100_000.0 {
            return Err("correzione della salita oltre 100 kHz".into());
        }
        if self.tracked.len() > 200 {
            return Err("troppi satelliti spuntati".into());
        }
        if self.map.contains(['/', '\\']) || self.map.contains("..") {
            return Err("nome della mappa non valido".into());
        }
        if self.tle_sources.len() > 10 {
            return Err("troppi server per i dati delle orbite".into());
        }
        if self.tle_sources.iter().any(|u| u.len() > 300 || !(u.starts_with("https://") || u.starts_with("http://"))) {
            return Err("ogni server deve essere un indirizzo http o https".into());
        }
        if self.tle_update_hours > 720 {
            return Err("l'aggiornamento automatico non puo' superare 720 ore".into());
        }
        Ok(())
    }
}

/// Le frequenze da mandare alla radio (Hz) e di quanto sono state spostate dal Doppler.
#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
pub struct DopplerPlan {
    pub down_nominal_hz: f64,
    pub up_nominal_hz: f64,
    pub rx_hz: f64,
    pub tx_hz: f64,
    pub rx_shift_hz: f64,
    pub tx_shift_hz: f64,
}

/// Il Doppler: in ricezione la frequenza scende se il satellite si allontana; in trasmissione si sale di quanto serve perche' il satellite
/// riceva proprio la frequenza di salita voluta.
pub fn doppler_plan(look: &Look, t: &Transponder, down_nominal_hz: f64, tx_offset_hz: f64) -> DopplerPlan {
    let beta = look.range_rate_kms / orbit::C_KMS;
    let up_nominal = t.up_for_down(down_nominal_hz) + tx_offset_hz;
    let rx = down_nominal_hz * (1.0 - beta);
    let tx = up_nominal / (1.0 - beta);
    DopplerPlan { down_nominal_hz, up_nominal_hz: up_nominal, rx_hz: rx, tx_hz: tx, rx_shift_hz: rx - down_nominal_hz, tx_shift_hz: tx - up_nominal }
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct HardwareState {
    pub ready: bool,
    /// L'ultimo errore, se il comando e' fallito.
    pub error: Option<String>,
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct RotorState {
    pub ready: bool,
    pub active: bool,
    /// L'ultima posizione comandata.
    pub az_deg: f64,
    pub el_deg: f64,
    pub error: Option<String>,
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct Now {
    pub time_unix: f64,
    pub norad: Option<u64>,
    pub name: String,
    pub look: Option<Look>,
    /// Sopra l'elevazione minima.
    pub visible: bool,
    pub plan: Option<DopplerPlan>,
    pub radio: HardwareState,
    pub radio_active: bool,
    pub rotor: RotorState,
    pub next_pass: Option<Pass>,
    /// La traccia del passaggio nel cielo: coppie (azimut, elevazione) ogni 10 secondi, per il radar.
    pub next_path: Vec<(f64, f64)>,
}

struct Inner {
    dir: PathBuf,
    settings: Mutex<SatSettings>,
    catalog: Mutex<Vec<SatInfo>>,
    sats: Mutex<Vec<Sat>>,
    tle_file: TleFile,
    fetching: AtomicBool,
    now: Mutex<Now>,
    radio: Mutex<Option<Arc<dyn SatRadio>>>,
    rotor: Mutex<Option<Arc<dyn SatRotator>>>,
    observer: Arc<dyn Fn() -> Option<Observer> + Send + Sync>,
    stop: AtomicBool,
}

fn lock<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|p| p.into_inner())
}

fn now_unix() -> f64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map_or(0.0, |d| d.as_secs_f64())
}

pub struct SatService {
    inner: Arc<Inner>,
}

impl SatService {
    /// Legge le impostazioni, il catalogo e i TLE salvati e accende il ciclo di tracking. `observer` da' il punto da cui si guarda (il locatore).
    pub fn open(data_dir: &std::path::Path, observer: Arc<dyn Fn() -> Option<Observer> + Send + Sync>) -> SatService {
        let settings: SatSettings = std::fs::read(data_dir.join("satelliti.json")).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or_default();
        let user: Vec<SatInfo> = std::fs::read(data_dir.join("satelliti-utente.json")).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or_default();
        let tle_file = TleFile::new(data_dir);
        let inner = Arc::new(Inner {
            dir: data_dir.to_path_buf(),
            settings: Mutex::new(settings),
            catalog: Mutex::new(catalog::merge(catalog::default_catalog(), user)),
            sats: Mutex::new(Vec::new()),
            tle_file,
            fetching: AtomicBool::new(false),
            now: Mutex::new(Now::default()),
            radio: Mutex::new(None),
            rotor: Mutex::new(None),
            observer,
            stop: AtomicBool::new(false),
        });
        load_sats(&inner);
        let worker = Arc::clone(&inner);
        if let Err(e) = std::thread::Builder::new().name("aurora-sat".into()).spawn(move || tracking_loop(&worker)) {
            tracing::error!("satelliti: ciclo di tracking non avviato: {e}");
        }
        let svc = SatService { inner };
        // dati vecchi o assenti: si scaricano da soli, senza bloccare l'avvio
        if tle_stale(&svc.inner) {
            svc.refresh_tle_background();
        }
        svc
    }

    pub fn set_radio(&self, radio: Arc<dyn SatRadio>) {
        *lock(&self.inner.radio) = Some(radio);
    }

    pub fn set_rotor(&self, rotor: Arc<dyn SatRotator>) {
        *lock(&self.inner.rotor) = Some(rotor);
    }

    pub fn settings(&self) -> SatSettings {
        lock(&self.inner.settings).clone()
    }

    pub fn apply_settings(&self, s: SatSettings) -> Result<(), String> {
        s.validate()?;
        let json = serde_json::to_vec_pretty(&s).map_err(|e| e.to_string())?;
        std::fs::write(self.inner.dir.join("satelliti.json"), json).map_err(|e| e.to_string())?;
        let rotor_off = lock(&self.inner.settings).rotor_on && !s.rotor_on;
        *lock(&self.inner.settings) = s;
        if rotor_off && let Some(r) = lock(&self.inner.rotor).clone() {
            r.stop();
        }
        Ok(())
    }

    /// Scarica i TLE da CelesTrak (bloccante). Restituisce quanti satelliti ci sono adesso.
    pub fn refresh_tle(&self) -> Result<usize, String> {
        refresh(&self.inner)
    }

    fn refresh_tle_background(&self) {
        let inner = Arc::clone(&self.inner);
        let _ = std::thread::Builder::new().name("aurora-sat-tle".into()).spawn(move || {
            if let Err(e) = refresh(&inner) {
                tracing::warn!("satelliti: TLE non scaricati: {e}");
            }
        });
    }

    pub fn catalog(&self) -> Vec<SatInfo> {
        lock(&self.inner.catalog).clone()
    }

    pub fn observer(&self) -> Option<Observer> {
        (self.inner.observer)()
    }

    pub fn now(&self) -> Now {
        lock(&self.inner.now).clone()
    }

    /// Quanti satelliti hanno dati e da quanti secondi sono stati scaricati.
    pub fn tle_status(&self) -> (usize, Option<f64>) {
        (lock(&self.inner.sats).len(), self.inner.tle_file.fetched_unix().map(|t| now_unix() - t))
    }

    pub fn has_sat(&self, norad: u64) -> bool {
        lock(&self.inner.sats).iter().any(|s| s.norad == norad)
    }

    /// Nome e numero di tutti i satelliti con dati: quelli del catalogo con i loro transponder e gli altri senza.
    pub fn listing(&self) -> Vec<(u64, String, bool)> {
        let catalog = lock(&self.inner.catalog).clone();
        let sats = lock(&self.inner.sats);
        let mut out: Vec<(u64, String, bool)> = catalog.iter().map(|c| (c.norad, c.name.clone(), sats.iter().any(|s| s.norad == c.norad))).collect();
        for s in sats.iter() {
            if !out.iter().any(|(n, _, _)| *n == s.norad) {
                out.push((s.norad, s.name.clone(), true));
            }
        }
        out.sort_by_key(|a| a.1.to_ascii_uppercase());
        out
    }

    fn sat(&self, norad: u64) -> Option<Sat> {
        lock(&self.inner.sats).iter().find(|s| s.norad == norad).cloned()
    }

    pub fn sat_details(&self, norad: u64) -> Option<SatDetails> {
        let s = self.sat(norad)?;
        Some(SatDetails { norad, name: s.name.clone(), inclination_deg: s.inclination_deg(), eccentricity: s.eccentricity(), revs_per_day: s.revs_per_day(), period_min: s.period_min(), epoch_unix: s.epoch_unix })
    }

    /// I passaggi nelle prossime `hours` ore.
    pub fn passes(&self, norad: u64, hours: f64, min_max_el: f64) -> Vec<Pass> {
        match (self.sat(norad), self.observer()) {
            (Some(s), Some(o)) => orbit::passes(&s, &o, now_unix(), hours.clamp(1.0, 72.0), min_max_el),
            _ => Vec::new(),
        }
    }

    /// La traccia a terra di un satellite: da adesso, per `minutes` minuti.
    pub fn track(&self, norad: u64, minutes: f64) -> Vec<(f64, f64)> {
        self.sat(norad).map(|s| orbit::ground_track(&s, now_unix() - minutes * 30.0, minutes, 30.0)).unwrap_or_default()
    }

    /// Dove sono adesso i satelliti spuntati e quello scelto.
    pub fn positions(&self) -> Vec<Position> {
        let st = self.settings();
        let obs = self.observer();
        let mut ids = st.tracked.clone();
        if let Some(sel) = st.selected && !ids.contains(&sel) {
            ids.push(sel);
        }
        let t = now_unix();
        ids.iter()
            .filter_map(|&n| {
                let s = self.sat(n)?;
                let o = obs.unwrap_or(Observer { lat_deg: 0.0, lon_deg: 0.0, alt_m: 0.0 });
                let l = orbit::look(&s, &o, t)?;
                Some(Position { norad: n, name: s.name.clone(), lat_deg: l.lat_deg, lon_deg: l.lon_deg, alt_km: l.alt_km, az_deg: l.az_deg, el_deg: l.el_deg, footprint_km: orbit::footprint_radius_km(l.alt_km, st.min_el_deg), visible: obs.is_some() && l.el_deg >= st.min_el_deg })
            })
            .collect()
    }

    /// I nomi delle mappe del mondo in `maps` (`jpg`, `png`, `bmp`, `webp`).
    pub fn maps(&self) -> Vec<String> {
        let mut out: Vec<String> = std::fs::read_dir(self.inner.dir.join("maps"))
            .map(|d| d.filter_map(Result::ok).filter_map(|e| e.file_name().into_string().ok()).filter(|n| content_type(n).is_some()).collect())
            .unwrap_or_default();
        out.sort();
        out
    }

    pub fn map_bytes(&self, name: &str) -> Option<(&'static str, Vec<u8>)> {
        if name.contains(['/', '\\']) || name.contains("..") {
            return None;
        }
        let ct = content_type(name)?;
        std::fs::read(self.inner.dir.join("maps").join(name)).ok().map(|b| (ct, b))
    }

    /// Salva una mappa portata dall'utente (immagine equirettangolare, longitudine -180..180 e latitudine 90..-90).
    pub fn save_map(&self, name: &str, bytes: &[u8]) -> Result<(), String> {
        if name.contains(['/', '\\']) || name.contains("..") || content_type(name).is_none() {
            return Err("nome o tipo di file non valido (jpg, png, bmp, webp)".into());
        }
        if bytes.len() > 20 * 1024 * 1024 {
            return Err("mappa oltre 20 MB".into());
        }
        let dir = self.inner.dir.join("maps");
        std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
        std::fs::write(dir.join(name), bytes).map_err(|e| e.to_string())
    }

    pub fn sky(&self) -> Sky {
        let t = now_unix();
        let (slat, slon) = orbit::sun_subpoint(t);
        let (mlat, mlon) = orbit::moon_subpoint(t);
        Sky { sun_lat_deg: slat, sun_lon_deg: slon, moon_lat_deg: mlat, moon_lon_deg: mlon, time_unix: t }
    }

    /// Ferma il ciclo (alla chiusura del programma).
    pub fn shutdown(&self) {
        self.inner.stop.store(true, Ordering::SeqCst);
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct SatDetails {
    pub norad: u64,
    pub name: String,
    pub inclination_deg: f64,
    pub eccentricity: f64,
    pub revs_per_day: f64,
    pub period_min: f64,
    pub epoch_unix: f64,
}

#[derive(Clone, Debug, Serialize)]
pub struct Position {
    pub norad: u64,
    pub name: String,
    pub lat_deg: f64,
    pub lon_deg: f64,
    pub alt_km: f64,
    pub az_deg: f64,
    pub el_deg: f64,
    /// Il raggio dell'impronta radio sulla superficie.
    pub footprint_km: f64,
    pub visible: bool,
}

#[derive(Clone, Copy, Debug, Serialize)]
pub struct Sky {
    pub sun_lat_deg: f64,
    pub sun_lon_deg: f64,
    pub moon_lat_deg: f64,
    pub moon_lon_deg: f64,
    pub time_unix: f64,
}

fn content_type(name: &str) -> Option<&'static str> {
    let ext = name.rsplit('.').next()?.to_ascii_lowercase();
    match ext.as_str() {
        "jpg" | "jpeg" => Some("image/jpeg"),
        "png" => Some("image/png"),
        "bmp" => Some("image/bmp"),
        "webp" => Some("image/webp"),
        _ => None,
    }
}

fn load_sats(inner: &Arc<Inner>) {
    let sets = inner.tle_file.load();
    let mut sats = Vec::with_capacity(sets.len());
    for s in &sets {
        match Sat::from_tle(&s.name, &s.line1, &s.line2) {
            Ok(sat) => sats.push(sat),
            Err(e) => tracing::debug!("satelliti: TLE di {} non valido: {e}", s.name),
        }
    }
    *lock(&inner.sats) = sats;
}

fn refresh(inner: &Arc<Inner>) -> Result<usize, String> {
    if inner.fetching.swap(true, Ordering::SeqCst) {
        return Err("scarico gia' in corso".into());
    }
    let result = (|| {
        let mut text = String::new();
        let mut last_error = None;
        let sources = lock(&inner.settings).tle_sources.clone();
        if sources.is_empty() {
            return Err("nessun server scelto per i dati delle orbite".into());
        }
        for url in &sources {
            match tle::fetch(url) {
                Ok(t) => {
                    text.push_str(&t);
                    if !text.ends_with('\n') {
                        text.push('\n');
                    }
                }
                Err(e) => last_error = Some(e),
            }
        }
        if text.is_empty() {
            return Err(last_error.unwrap_or_else(|| "nessun dato scaricato".into()));
        }
        inner.tle_file.save(&text, now_unix())?;
        load_sats(inner);
        Ok(lock(&inner.sats).len())
    })();
    inner.fetching.store(false, Ordering::SeqCst);
    result
}

/// Lo stato che il ciclo si porta dietro da un secondo all'altro.
#[derive(Default)]
struct LoopState {
    last_rx: Option<u64>,
    last_tx: Option<u64>,
    split_on: bool,
    last_rotor: Option<(f64, f64)>,
    prepointed_for: Option<f64>,
    rotor_was_on: bool,
    pass_cache: Option<(u64, f64, Option<Pass>)>,
    path_cache: Vec<(f64, f64)>,
    radio_error: Option<String>,
    rotor_error: Option<String>,
}

/// I dati delle orbite sono da aggiornare: mancano, oppure l'aggiornamento automatico e' acceso ed e' passato il tempo scelto.
fn tle_stale(inner: &Inner) -> bool {
    let hours = lock(&inner.settings).tle_update_hours;
    match inner.tle_file.fetched_unix() {
        None => true,
        Some(t) => hours > 0 && now_unix() - t > f64::from(hours) * 3600.0,
    }
}

fn tracking_loop(inner: &Arc<Inner>) {
    let mut st = LoopState::default();
    let mut ticks: u64 = 0;
    let mut last_try = 0.0_f64;
    while !inner.stop.load(Ordering::SeqCst) {
        let started = Instant::now();
        tick(inner, &mut st);
        // ogni minuto si guarda se e' ora di aggiornare i dati delle orbite; dopo un tentativo non si riprova prima di mezz'ora
        ticks += 1;
        if ticks.is_multiple_of(60) && now_unix() - last_try > 1800.0 && !inner.fetching.load(Ordering::SeqCst) && tle_stale(inner) {
            last_try = now_unix();
            let worker = Arc::clone(inner);
            let _ = std::thread::Builder::new().name("aurora-sat-tle".into()).spawn(move || {
                if let Err(e) = refresh(&worker) {
                    tracing::warn!("satelliti: aggiornamento automatico dei dati delle orbite non riuscito: {e}");
                }
            });
        }
        std::thread::sleep(Duration::from_secs(1).saturating_sub(started.elapsed()).max(Duration::from_millis(100)));
    }
}

fn tick(inner: &Arc<Inner>, st: &mut LoopState) {
    let settings = lock(&inner.settings).clone();
    let t = now_unix();
    let radio = lock(&inner.radio).clone();
    let rotor = lock(&inner.rotor).clone();
    let mut now = Now { time_unix: t, radio: HardwareState { ready: radio.as_ref().is_some_and(|r| r.ready()), error: st.radio_error.clone() }, ..Now::default() };
    now.rotor = RotorState { ready: rotor.as_ref().is_some_and(|r| r.ready()), active: settings.rotor_on, error: st.rotor_error.clone(), ..RotorState::default() };
    let Some(norad) = settings.selected else {
        *lock(&inner.now) = now;
        return;
    };
    let sat = lock(&inner.sats).iter().find(|s| s.norad == norad).cloned();
    let obs = (inner.observer)();
    let (Some(sat), Some(obs)) = (sat, obs) else {
        now.norad = Some(norad);
        *lock(&inner.now) = now;
        return;
    };
    now.norad = Some(norad);
    now.name = sat.name.clone();
    let Some(look) = orbit::look(&sat, &obs, t) else {
        *lock(&inner.now) = now;
        return;
    };
    now.visible = look.el_deg >= settings.min_el_deg.max(0.0);
    now.look = Some(look);

    // il prossimo passaggio: si ricalcola ogni minuto o cambiando satellite
    let stale = !matches!(&st.pass_cache, Some((n, at, p)) if *n == norad && t - *at < 60.0 && p.is_none_or(|p| p.los_unix > t));
    if stale {
        let next = orbit::passes(&sat, &obs, t, 24.0, settings.min_el_deg.max(0.0)).into_iter().find(|p| p.los_unix > t);
        st.path_cache = next
            .map(|p| {
                let mut path = Vec::new();
                let mut x = p.aos_unix;
                while x <= p.los_unix {
                    if let Some(l) = orbit::look(&sat, &obs, x) {
                        path.push((l.az_deg, l.el_deg));
                    }
                    x += 10.0;
                }
                path
            })
            .unwrap_or_default();
        st.pass_cache = Some((norad, t, next));
    }
    now.next_path = st.path_cache.clone();
    now.next_pass = st.pass_cache.as_ref().and_then(|(_, _, p)| *p);

    // il transponder e la frequenza di discesa scelta
    let catalog = lock(&inner.catalog).clone();
    let transponder = catalog.iter().find(|c| c.norad == norad).and_then(|c| c.transponders.get(settings.transponder).cloned());
    if let Some(tp) = &transponder {
        let down = settings.down_hz.unwrap_or_else(|| tp.down_center_hz()).clamp(tp.down_low_hz.min(tp.down_high_hz), tp.down_low_hz.max(tp.down_high_hz).max(1.0));
        now.plan = Some(doppler_plan(&look, tp, down, settings.tx_offset_hz));
    }

    // ---- Doppler sulla radio ----
    if settings.doppler_on && now.visible && let (Some(radio), Some(plan), Some(tp)) = (&radio, &now.plan, &transponder) && radio.ready() {
        let mut ok = true;
        // radio con VFO separati: split acceso una volta; per le frequenze uguali (APRS) non serve
        let wants_split = (plan.up_nominal_hz - plan.down_nominal_hz).abs() > 1_000.0;
        if wants_split && !st.split_on {
            match radio.set_split(true) {
                Ok(()) => st.split_on = true,
                Err(e) => {
                    st.radio_error = Some(format!("split: {e}"));
                    ok = false;
                }
            }
        }
        let rx = plan.rx_hz.round() as u64;
        if ok && st.last_rx.is_none_or(|last| last.abs_diff(rx) >= 5) {
            match radio.set_rx_hz(rx) {
                Ok(()) => st.last_rx = Some(rx),
                Err(e) => {
                    st.radio_error = Some(format!("ricezione: {e}"));
                    ok = false;
                }
            }
        }
        let txf = plan.tx_hz.round() as u64;
        if ok && wants_split && st.last_tx.is_none_or(|last| last.abs_diff(txf) >= 5) {
            match radio.set_tx_hz(txf) {
                Ok(()) => st.last_tx = Some(txf),
                Err(e) => {
                    st.radio_error = Some(format!("trasmissione: {e}"));
                    ok = false;
                }
            }
        }
        if ok {
            st.radio_error = None;
        }
        now.radio_active = ok;
        let _ = tp;
    } else if !settings.doppler_on {
        st.last_rx = None;
        st.last_tx = None;
        st.split_on = false;
        st.radio_error = None;
    }
    now.radio.error = st.radio_error.clone();

    // ---- il rotore segue il satellite ----
    if settings.rotor_on {
        if let Some(rotor) = rotor.as_ref().filter(|r| r.ready()) {
            let target = if now.visible {
                Some((look.az_deg, look.el_deg.clamp(0.0, 90.0)))
            } else if settings.rotor_preposition && let Some(p) = now.next_pass.filter(|p| p.aos_unix - t < 150.0 && p.aos_unix > t) && st.prepointed_for != Some(p.aos_unix) {
                // poco prima del passaggio ci si mette dove il satellite sorgera'
                st.prepointed_for = Some(p.aos_unix);
                Some((p.aos_az_deg, 0.0))
            } else {
                None
            };
            if let Some((az, el)) = target {
                let moved = st.last_rotor.is_none_or(|(a, e)| orbit::angle_diff_deg(a, az).abs() >= 1.0 || (e - el).abs() >= 1.0);
                if moved {
                    match rotor.point(az, el) {
                        Ok(()) => {
                            st.last_rotor = Some((az, el));
                            st.rotor_error = None;
                        }
                        Err(e) => st.rotor_error = Some(e),
                    }
                }
            }
        }
        st.rotor_was_on = true;
    } else if st.rotor_was_on {
        st.rotor_was_on = false;
        st.last_rotor = None;
    }
    if let Some((az, el)) = st.last_rotor {
        now.rotor.az_deg = az;
        now.rotor.el_deg = el;
    }
    now.rotor.error = st.rotor_error.clone();
    *lock(&inner.now) = now;
}

#[cfg(test)]
mod tests {
    use super::*;

    fn look_with_rate(rate: f64) -> Look {
        Look { az_deg: 0.0, el_deg: 30.0, range_km: 1000.0, range_rate_kms: rate, lat_deg: 0.0, lon_deg: 0.0, alt_km: 400.0, speed_kms: 7.6, rising: rate < 0.0 }
    }

    fn rs44() -> Transponder {
        catalog::default_catalog().into_iter().find(|s| s.norad == 44909).expect("RS-44").transponders.remove(0)
    }

    #[test]
    fn il_doppler_sale_in_ricezione_quando_si_avvicina() {
        let t = rs44();
        let plan = doppler_plan(&look_with_rate(-5.0), &t, 435.625e6, 0.0);
        // si avvicina a 5 km/s: sulla ricezione +7,3 kHz circa
        assert!((plan.rx_shift_hz - 7_265.0).abs() < 30.0, "{}", plan.rx_shift_hz);
        assert!(plan.rx_hz > plan.down_nominal_hz);
        // in trasmissione si scende per compensare: la salita nominale e' 145.965
        assert!(plan.tx_shift_hz < 0.0);
        assert!((plan.up_nominal_hz - 145.965e6).abs() < 1.0);
    }

    #[test]
    fn senza_velocita_radiale_non_c_e_correzione() {
        let t = rs44();
        let plan = doppler_plan(&look_with_rate(0.0), &t, 435.625e6, 0.0);
        assert!(plan.rx_shift_hz.abs() < 1e-6 && plan.tx_shift_hz.abs() < 1e-6);
    }

    #[test]
    fn il_satellite_riceve_la_frequenza_di_salita_voluta() {
        // dopo il Doppler, cio' che il satellite riceve (tx * (1 - beta)) deve essere la salita nominale
        let t = rs44();
        for rate in [-6.5, -2.0, 0.0, 3.3, 7.0] {
            let plan = doppler_plan(&look_with_rate(rate), &t, 435.640e6, 0.0);
            let beta = rate / orbit::C_KMS;
            assert!((plan.tx_hz * (1.0 - beta) - plan.up_nominal_hz).abs() < 0.01, "rate {rate}");
            // e noi riceviamo la discesa nominale
            assert!((plan.rx_hz / (1.0 - beta) - plan.down_nominal_hz).abs() < 0.01);
        }
    }

    #[test]
    fn la_correzione_fine_della_salita_si_somma() {
        let t = rs44();
        let a = doppler_plan(&look_with_rate(0.0), &t, 435.625e6, 0.0);
        let b = doppler_plan(&look_with_rate(0.0), &t, 435.625e6, 120.0);
        assert!((b.tx_hz - a.tx_hz - 120.0).abs() < 1e-6);
    }

    #[test]
    fn le_impostazioni_si_controllano() {
        assert!(SatSettings::default().validate().is_ok());
        assert!(SatSettings { min_el_deg: 90.0, ..SatSettings::default() }.validate().is_err());
        assert!(SatSettings { map: "../x.png".into(), ..SatSettings::default() }.validate().is_err());
    }

    #[test]
    fn i_tipi_di_mappa() {
        assert_eq!(content_type("Contour HiRes.JPG"), Some("image/jpeg"));
        assert_eq!(content_type("a.bmp"), Some("image/bmp"));
        assert_eq!(content_type("a.txt"), None);
    }
}
