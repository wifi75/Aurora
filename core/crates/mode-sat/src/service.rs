//! Il servizio dei satelliti: tiene i TLE, calcola ogni secondo dove sta il satellite scelto e, se acceso, **corregge il Doppler sulla radio**
//! (ricezione e trasmissione, anche con il transponder invertente) e **fa seguire il satellite al rotore** in azimut ed elevazione.
//! Impostazioni in `satelliti.json`; i satelliti dell'utente in `satelliti-utente.json`; le mappe nella cartella `maps`.
//!
//! Solo quando servono: il calcolo di ogni secondo e lo scaricamento automatico dei dati delle orbite girano mentre una pagina dei
//! satelliti e' aperta (ogni sua lettura e' un battito, [`SatService::touch`]) oppure mentre il Doppler o il rotore sono accesi (lavorano
//! sulla radio e sul rotore anche a pagina chiusa). Altrimenti, lavorando in FT8, il servizio dorme.

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
    /// Un numero che cambia quando la radio si ricollega o cambia: la memoria dei comandi rifiutati si azzera.
    fn link_id(&self) -> u64 {
        0
    }
    /// Il modo di ricezione (nomi di Hamlib: USB, LSB, FM, CW).
    fn set_rx_mode(&self, _mode: &str) -> Result<(), String> {
        Err("non supportato".into())
    }
    /// Il modo del VFO di trasmissione (con lo split vero).
    fn set_tx_mode(&self, _mode: &str) -> Result<(), String> {
        Err("non supportato".into())
    }
    /// Ripiego per le radio senza split da CAT: si passa al VFO secondario, si scrivono frequenza (e modo, se dato), si torna al VFO di
    /// ricezione. `Ok(false)` = non adesso (la radio sta trasmettendo), si riprova. Il primo errore spegne il ripiego.
    fn set_tx_via_vfo(&self, _hz: u64, _mode: Option<&str>) -> Result<bool, String> {
        Err("non supportato".into())
    }
    /// IC-820H: la trasmissione scritta sulla banda secondaria con trame CI-V dirette, dopo una prova di sicurezza automatica. `Ok(false)` =
    /// non adesso (in trasmissione o troppo presto), si riprova. Un errore spegne il ripiego fino alla riconnessione o a una nuova spunta.
    fn set_tx_via_civ(&self, _hz: u64) -> Result<bool, String> {
        Err("non supportato".into())
    }
    /// Una riga sullo stato della trasmissione con trame CI-V dirette (esito della prova), per la pagina dei satelliti.
    fn uplink_note(&self) -> Option<String> {
        None
    }
    /// Accende il modo satellite della radio (Hamlib `SATMODE`), se lo ha.
    fn enable_satmode(&self) -> Result<(), String> {
        Err("non supportato".into())
    }
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
    /// Il rotore non riceve un nuovo comando se lo spostamento rispetto all'ultimo e' minore di cosi' (gradi), in azimut ed elevazione.
    pub rotor_min_move_deg: f64,
    /// Correzione dell'azimut mandato al rotore (gradi): si somma, il risultato e' riportato in 0-360.
    pub rotor_az_offset_deg: f64,
    /// Sotto questa elevazione non si corregge il Doppler e il rotore non segue.
    pub min_el_deg: f64,
    /// Ripiego «due VFO» per le radio senza split da CAT (sperimentale, spento di fabbrica).
    pub uplink_two_vfo: bool,
    /// IC-820H: trasmissione scritta con comandi CI-V diretti dopo una prova automatica (sperimentale, spento di fabbrica). Se acceso
    /// prende il posto del ripiego a due VFO.
    pub uplink_civ_ic820h: bool,
    /// Il ripiego aggiorna la trasmissione al massimo ogni tanti secondi (2-120)...
    pub uplink_interval_s: u32,
    /// ...o appena lo scostamento supera tanti Hz (20-5000), ma mai piu' di una volta ogni 2 s.
    pub uplink_threshold_hz: u32,
    /// Modo di ricezione: «Auto» (dal transponder), «USB», «LSB», «FM» o «CW».
    pub rx_mode: String,
    /// Modo di trasmissione: «Auto» (dal transponder: invertente = laterale opposta), «USB», «LSB», «FM» o «CW».
    pub tx_mode: String,
    /// Rotore, PRIMA: va al punto di sorgere tanti secondi prima del passaggio (0-600; 0 = solo all'AOS). Serve `rotor_preposition`.
    pub rotor_lead_s: u32,
    /// Rotore, DURANTE: inizia a seguire quando l'elevazione supera tanti gradi (0-30).
    pub rotor_start_el_deg: f64,
    /// Rotore, DURANTE: un comando al massimo ogni tanti secondi (1-10).
    pub rotor_interval_s: u32,
    /// Rotore, DOPO il LOS: `stay` (resta dov'e'), `park` (posizione fissa) o `next` (punto di sorgere del prossimo passaggio).
    pub rotor_after: String,
    pub rotor_park_az_deg: f64,
    pub rotor_park_el_deg: f64,
    /// Rotore, DOPO: il comando parte tanti secondi dopo il LOS (0-300).
    pub rotor_after_delay_s: u32,
    /// La mappa del mondo (nome del file in `maps`).
    pub map: String,
    /// I server da cui si scaricano i dati delle orbite (indirizzi http/https con TLE a tre righe).
    pub tle_sources: Vec<String>,
    /// Ogni quante ore si aggiornano da soli i dati delle orbite; 0 = solo a mano.
    pub tle_update_hours: u32,
}

impl Default for SatSettings {
    fn default() -> Self {
        SatSettings { selected: Some(25544), tracked: vec![25544, 44909], transponder: 0, down_hz: None, tx_offset_hz: 0.0, doppler_on: false, rotor_on: false, rotor_preposition: true, rotor_min_move_deg: 2.0, rotor_az_offset_deg: 0.0, min_el_deg: 0.0, uplink_two_vfo: false, uplink_civ_ic820h: false, uplink_interval_s: 10, uplink_threshold_hz: 300, rx_mode: "Auto".into(), tx_mode: "Auto".into(), rotor_lead_s: 60, rotor_start_el_deg: 0.0, rotor_interval_s: 2, rotor_after: "stay".into(), rotor_park_az_deg: 0.0, rotor_park_el_deg: 0.0, rotor_after_delay_s: 10, map: String::new(), tle_sources: tle::SOURCES.iter().map(|u| (*u).to_string()).collect(), tle_update_hours: 24 }
    }
}

impl SatSettings {
    fn validate(&self) -> Result<(), String> {
        if !self.min_el_deg.is_finite() || !(0.0..=60.0).contains(&self.min_el_deg) {
            return Err("elevazione minima fuori da 0-60 gradi".into());
        }
        if !self.rotor_min_move_deg.is_finite() || !(0.0..=45.0).contains(&self.rotor_min_move_deg) {
            return Err("spostamento minimo del rotore fuori da 0-45 gradi".into());
        }
        if !self.rotor_az_offset_deg.is_finite() || !(-180.0..=180.0).contains(&self.rotor_az_offset_deg) {
            return Err("correzione dell'azimut del rotore fuori da -180/+180 gradi".into());
        }
        if !self.tx_offset_hz.is_finite() || self.tx_offset_hz.abs() > 100_000.0 {
            return Err("correzione della salita oltre 100 kHz".into());
        }
        if !(2..=120).contains(&self.uplink_interval_s) {
            return Err("intervallo del ripiego a due VFO fuori da 2-120 secondi".into());
        }
        if !(20..=5000).contains(&self.uplink_threshold_hz) {
            return Err("soglia del ripiego a due VFO fuori da 20-5000 Hz".into());
        }
        for m in [&self.rx_mode, &self.tx_mode] {
            if !["Auto", "USB", "LSB", "FM", "CW"].iter().any(|ok| ok.eq_ignore_ascii_case(m)) {
                return Err("modo non valido (Auto, USB, LSB, FM, CW)".into());
            }
        }
        if self.rotor_lead_s > 600 {
            return Err("anticipo del rotore oltre 600 secondi".into());
        }
        if !self.rotor_start_el_deg.is_finite() || !(0.0..=30.0).contains(&self.rotor_start_el_deg) {
            return Err("elevazione di partenza del rotore fuori da 0-30 gradi".into());
        }
        if !(1..=10).contains(&self.rotor_interval_s) {
            return Err("aggiornamento del rotore fuori da 1-10 secondi".into());
        }
        if !["stay", "park", "next"].contains(&self.rotor_after.as_str()) {
            return Err("scelta del rotore a fine passaggio non valida".into());
        }
        if !self.rotor_park_az_deg.is_finite() || !(0.0..=360.0).contains(&self.rotor_park_az_deg) || !self.rotor_park_el_deg.is_finite() || !(0.0..=90.0).contains(&self.rotor_park_el_deg) {
            return Err("posizione di parcheggio fuori da azimut 0-360 ed elevazione 0-90".into());
        }
        if self.rotor_after_delay_s > 300 {
            return Err("ritardo del rotore dopo il LOS oltre 300 secondi".into());
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

/// Il comando da mandare al rotore per puntare `az`/`el`: l'azimut con la correzione `az_offset` sommata e riportato in 0-360.
/// Nessun comando (`None`) se rispetto all'ultimo (`last`, gia' con la correzione) lo spostamento e' minore di `min_move` gradi sia in azimut sia in elevazione.
pub fn rotor_command(last: Option<(f64, f64)>, az: f64, el: f64, min_move: f64, az_offset: f64) -> Option<(f64, f64)> {
    let az = (az + az_offset).rem_euclid(360.0);
    let az = if az >= 360.0 { 0.0 } else { az };
    let moved = last.is_none_or(|(a, e)| orbit::angle_diff_deg(a, az).abs() >= min_move || (e - el).abs() >= min_move);
    moved.then_some((az, el))
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
    /// Una nota sullo stato (esito della prova CI-V dell'IC-820H).
    pub note: Option<String>,
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
    /// L'ultima lettura di una pagina dei satelliti (`None`: mai aperta da quando Aurora e' partita).
    seen: Mutex<Option<Instant>>,
}

/// Dopo tanto tempo senza letture la pagina dei satelliti si considera chiusa (la pagina legge una volta al secondo).
const PAGE_GONE: Duration = Duration::from_secs(30);

/// Il servizio lavora: pagina aperta di recente, oppure Doppler o rotore accesi.
fn in_use(inner: &Inner) -> bool {
    let page = lock(&inner.seen).is_some_and(|t| t.elapsed() < PAGE_GONE);
    let s = lock(&inner.settings);
    page || s.doppler_on || s.rotor_on
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
            seen: Mutex::new(None),
        });
        load_sats(&inner);
        let worker = Arc::clone(&inner);
        if let Err(e) = std::thread::Builder::new().name("aurora-sat".into()).spawn(move || tracking_loop(&worker)) {
            tracing::error!("satelliti: ciclo di tracking non avviato: {e}");
        }
        // i dati vecchi o assenti non si scaricano piu' all'avvio: li scarica il ciclo appena una pagina dei satelliti li chiede
        SatService { inner }
    }

    /// Una pagina dei satelliti sta guardando (ogni sua lettura): il calcolo di ogni secondo e lo scaricamento automatico ripartono.
    pub fn touch(&self) {
        *lock(&self.inner.seen) = Some(Instant::now());
    }

    /// Il servizio sta lavorando (pagina aperta, Doppler o rotore accesi).
    pub fn in_use(&self) -> bool {
        in_use(&self.inner)
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

    /// I prossimi passaggi di tutti i satelliti spuntati, in ordine di inizio, con elevazione massima almeno `min_max_el`.
    pub fn passes_all(&self, hours: f64, min_max_el: f64) -> Vec<SatPass> {
        let Some(o) = self.observer() else { return Vec::new() };
        let t = now_unix();
        let hours = hours.clamp(1.0, 72.0);
        let per_sat = self
            .settings()
            .tracked
            .iter()
            .filter_map(|&n| {
                let s = self.sat(n)?;
                let list = orbit::passes(&s, &o, t, hours, min_max_el);
                Some((n, s.name.clone(), list))
            })
            .collect();
        merge_passes(per_sat, min_max_el)
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

/// Un passaggio con il satellite a cui appartiene (per l'elenco unico).
#[derive(Clone, Debug, Serialize)]
pub struct SatPass {
    pub norad: u64,
    pub name: String,
    #[serde(flatten)]
    pub pass: Pass,
}

/// Unisce i passaggi dei singoli satelliti in un elenco solo: tiene quelli con elevazione massima almeno `min_max_el`, in ordine di inizio.
pub fn merge_passes(per_sat: Vec<(u64, String, Vec<Pass>)>, min_max_el: f64) -> Vec<SatPass> {
    let mut out: Vec<SatPass> = per_sat
        .into_iter()
        .flat_map(|(norad, name, list)| list.into_iter().filter(move |p| p.max_el_deg >= min_max_el).map(move |pass| SatPass { norad, name: name.clone(), pass }))
        .collect();
    out.sort_by(|a, b| a.pass.aos_unix.total_cmp(&b.pass.aos_unix));
    out
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
    /// La radio non accetta il comando di split (Hamlib: «funzione non disponibile»): non lo si riprova ogni secondo e il Doppler della ricezione va avanti lo stesso.
    split_failed: bool,
    /// La radio non accetta la frequenza di trasmissione: non la si riprova ogni secondo.
    tx_failed: bool,
    last_rotor: Option<(f64, f64)>,
    prepointed_for: Option<f64>,
    rotor_was_on: bool,
    /// Il momento dell'ultimo comando di inseguimento al rotore (secondi unix).
    last_rotor_cmd_t: f64,
    /// Il satellite visibile e seguito nel secondo precedente: quando smette e' il LOS.
    rotor_tracking: bool,
    rotor_norad: Option<u64>,
    /// Il LOS e' avvenuto a questo momento e il comando di fine passaggio non e' ancora partito.
    los_at: Option<f64>,
    /// Memoria della radio.
    link_id: Option<u64>,
    last_rx_mode: Option<String>,
    last_tx_mode: Option<String>,
    rx_mode_failed: bool,
    tx_mode_failed: bool,
    uplink_last: Option<(u64, f64)>,
    /// Il ripiego a due VFO e' stato rifiutato: fermo fino a `vfo_retry_at` (secondi del passaggio), poi si riprova da solo.
    vfo_failed: bool,
    vfo_retry_at: f64,
    satmode_tried: bool,
    two_vfo_prev: bool,
    /// La trasmissione con trame CI-V dirette e' fallita: ferma fino a `civ_retry_at` (secondi del passaggio), poi si riprova da sola (come
    /// il ripiego a due VFO), o prima con una nuova spunta o alla riconnessione.
    civ_failed: bool,
    civ_retry_at: f64,
    civ_prev: bool,
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
    let mut was_in_use = false;
    while !inner.stop.load(Ordering::SeqCst) {
        let started = Instant::now();
        // pagina chiusa e Doppler e rotore spenti: niente calcoli e niente scaricamenti (si riguarda una volta al secondo)
        let using = in_use(inner);
        if using {
            tick(inner, &mut st);
            ticks += 1;
        }
        // appena il servizio riparte e poi ogni minuto si guarda se e' ora di aggiornare i dati delle orbite; dopo un tentativo non si
        // riprova prima di mezz'ora
        let check = using && (!was_in_use || ticks.is_multiple_of(60));
        was_in_use = using;
        if check && now_unix() - last_try > 1800.0 && !inner.fetching.load(Ordering::SeqCst) && tle_stale(inner) {
            last_try = now_unix();
            let worker = Arc::clone(inner);
            let _ = std::thread::Builder::new().name("aurora-sat-tle".into()).spawn(move || {
                // PC lento: lo scaricamento e la lettura dei dati aspettano la fine della decodifica (al massimo 30 s)
                aurora_core::idle::wait_free_blocking(aurora_core::idle::MAX_WAIT);
                if let Err(e) = refresh(&worker) {
                    tracing::warn!("satelliti: aggiornamento automatico dei dati delle orbite non riuscito: {e}");
                }
            });
        }
        std::thread::sleep(Duration::from_secs(1).saturating_sub(started.elapsed()).max(Duration::from_millis(100)));
    }
}

impl LoopState {
    /// Ricomincia un passaggio: frequenze e modi si riscrivono una volta.
    fn reset_pass(&mut self) {
        self.last_rx = None;
        self.last_tx = None;
        self.split_on = false;
        self.last_rx_mode = None;
        self.last_tx_mode = None;
        self.uplink_last = None;
    }

    /// La radio e' un'altra o si e' ricollegata: i comandi rifiutati si riprovano (una volta).
    fn reset_failures(&mut self) {
        self.split_failed = false;
        self.tx_failed = false;
        self.rx_mode_failed = false;
        self.tx_mode_failed = false;
        self.vfo_failed = false;
        self.civ_failed = false;
        self.satmode_tried = false;
    }
}

/// I modi da usare: la scelta dell'operatore o, con «Auto», quelli del transponder.
pub fn sat_modes(tp: &Transponder, rx_choice: &str, tx_choice: &str) -> (String, String) {
    let (arx, atx) = tp.auto_modes();
    let pick = |choice: &str, auto: String| if choice.trim().is_empty() || choice.eq_ignore_ascii_case("auto") { auto } else { choice.trim().to_ascii_uppercase() };
    (pick(rx_choice, arx), pick(tx_choice, atx))
}

/// Il ripiego a due VFO scrive la trasmissione la prima volta, poi al massimo ogni `interval_s` secondi (se e' cambiata di almeno 5 Hz) o
/// appena e' cambiata di `threshold_hz` (ma non piu' di una volta ogni 2 s): ogni cambio di VFO puo' far gracchiare la radio.
pub fn uplink_due(last: Option<(u64, f64)>, hz: u64, t: f64, interval_s: u32, threshold_hz: u32) -> bool {
    let Some((last_hz, last_t)) = last else { return true };
    let (diff, dt) = (last_hz.abs_diff(hz), t - last_t);
    (dt >= f64::from(interval_s) && diff >= 5) || (dt >= 2.0 && diff >= u64::from(threshold_hz))
}

/// Dopo un rifiuto il ripiego a due VFO si riprova dopo questi secondi.
const VFO_RETRY_S: f64 = 60.0;

/// La parte radio di un secondo di inseguimento: split o ripiego a due VFO, frequenze, modi. `true` se la ricezione ha funzionato.
fn drive_radio(radio: &dyn SatRadio, st: &mut LoopState, s: &SatSettings, plan: &DopplerPlan, tp: &Transponder, t: f64) -> bool {
    let id = radio.link_id();
    if st.link_id != Some(id) {
        if st.link_id.is_some() {
            st.reset_failures();
            st.reset_pass();
        }
        st.link_id = Some(id);
    }
    if s.uplink_two_vfo && !st.two_vfo_prev {
        // riacceso dall'operatore: si riprova
        st.vfo_failed = false;
        st.satmode_tried = false;
    }
    st.two_vfo_prev = s.uplink_two_vfo;
    // un rifiuto (Sub spento, un disturbo sul cavo) non vale per sempre: dopo `VFO_RETRY_S` si riprova, con la spunta delle impostazioni intatta
    if st.vfo_failed && t >= st.vfo_retry_at {
        st.vfo_failed = false;
    }
    if s.uplink_civ_ic820h && !st.civ_prev {
        st.civ_failed = false;
    }
    // SAT spento e riacceso sulla radio (la prova CI-V falliva con le due bande uguali): ora che le bande sono diverse si riprova da soli
    if st.civ_failed && t >= st.civ_retry_at {
        st.civ_failed = false;
    }
    st.civ_prev = s.uplink_civ_ic820h;
    // la trasmissione CI-V dell'IC-820H prende il posto del ripiego a due VFO; fallita, il Doppler segue solo la ricezione
    let civ = s.uplink_civ_ic820h;
    let two_vfo = !civ && s.uplink_two_vfo && !st.vfo_failed;
    let mut ok = true;
    // radio con VFO separati: split acceso una volta; per le frequenze uguali (APRS) non serve
    let wants_split = (plan.up_nominal_hz - plan.down_nominal_hz).abs() > 1_000.0;
    if wants_split && !two_vfo && !civ && !st.split_on && !st.split_failed {
        match radio.set_split(true) {
            Ok(()) => st.split_on = true,
            Err(e) => {
                // si segna una volta (IC-820H: 5000 tentativi in un'ora, segnalazione #72): la ricezione col Doppler va avanti
                tracing::warn!("satelliti: la radio non accetta lo split ({e}): il Doppler segue solo la ricezione");
                st.split_failed = true;
                st.radio_error = Some(format!("split: {e}. Per trasmettere prova «Trasmissione con due VFO» nelle impostazioni dei satelliti"));
            }
        }
    }
    let (rx_mode, tx_mode) = sat_modes(tp, &s.rx_mode, &s.tx_mode);
    if st.last_rx_mode.as_deref() != Some(rx_mode.as_str()) && !st.rx_mode_failed {
        match radio.set_rx_mode(&rx_mode) {
            Ok(()) => st.last_rx_mode = Some(rx_mode.clone()),
            Err(e) => {
                tracing::warn!("satelliti: la radio non accetta il modo di ricezione {rx_mode} ({e}): non si riprova");
                st.rx_mode_failed = true;
                st.radio_error = Some(format!("modo {rx_mode}: {e}"));
            }
        }
    }
    let rx = plan.rx_hz.round() as u64;
    if st.last_rx.is_none_or(|last| last.abs_diff(rx) >= 5) {
        match radio.set_rx_hz(rx) {
            Ok(()) => st.last_rx = Some(rx),
            Err(e) => {
                st.radio_error = Some(format!("ricezione: {e}"));
                ok = false;
            }
        }
    }
    let txf = plan.tx_hz.round() as u64;
    if civ {
        if ok && wants_split && !st.civ_failed && uplink_due(st.uplink_last, txf, t, s.uplink_interval_s, s.uplink_threshold_hz) {
            match radio.set_tx_via_civ(txf) {
                Ok(true) => st.uplink_last = Some((txf, t)),
                Ok(false) => {}
                Err(e) => {
                    tracing::warn!("satelliti: trasmissione CI-V dell'IC-820H non disponibile ({e}): il Doppler segue solo la ricezione, riprovo tra {VFO_RETRY_S:.0} s");
                    st.civ_failed = true;
                    st.civ_retry_at = t + VFO_RETRY_S;
                    st.radio_error = Some(format!("CI-V IC-820H: {e}. La trasmissione resta da impostare a mano"));
                }
            }
        }
    } else if ok && wants_split && two_vfo {
        if !st.satmode_tried {
            st.satmode_tried = true;
            if let Err(e) = radio.enable_satmode() {
                tracing::info!("satelliti: modo satellite della radio non acceso da CAT ({e}): se serve si accende a mano");
            }
        }
        if uplink_due(st.uplink_last, txf, t, s.uplink_interval_s, s.uplink_threshold_hz) {
            let mode = (st.last_tx_mode.as_deref() != Some(tx_mode.as_str()) && !st.tx_mode_failed).then_some(tx_mode.as_str());
            match radio.set_tx_via_vfo(txf, mode) {
                Ok(true) => {
                    st.uplink_last = Some((txf, t));
                    if mode.is_some() {
                        st.last_tx_mode = Some(tx_mode.clone());
                    }
                }
                Ok(false) => {}
                Err(e) => {
                    tracing::warn!("satelliti: la radio non accetta il ripiego a due VFO ({e}): riprovo tra {VFO_RETRY_S:.0} s");
                    st.vfo_failed = true;
                    st.vfo_retry_at = t + VFO_RETRY_S;
                    st.radio_error = Some(format!("due VFO: {e}. La trasmissione resta da impostare a mano, riprovo tra {VFO_RETRY_S:.0} s"));
                }
            }
        }
    } else if ok && wants_split && !st.tx_failed && st.last_tx.is_none_or(|last| last.abs_diff(txf) >= 5) {
        match radio.set_tx_hz(txf) {
            Ok(()) => {
                st.last_tx = Some(txf);
                if st.last_tx_mode.as_deref() != Some(tx_mode.as_str()) && !st.tx_mode_failed {
                    match radio.set_tx_mode(&tx_mode) {
                        Ok(()) => st.last_tx_mode = Some(tx_mode.clone()),
                        Err(e) => {
                            tracing::warn!("satelliti: la radio non accetta il modo di trasmissione {tx_mode} ({e}): non si riprova");
                            st.tx_mode_failed = true;
                            st.radio_error = Some(format!("modo di trasmissione {tx_mode}: {e}"));
                        }
                    }
                }
            }
            Err(e) => {
                tracing::warn!("satelliti: la radio non accetta la frequenza di trasmissione ({e}): non si riprova");
                st.tx_failed = true;
                st.radio_error = Some(format!("trasmissione: {e}"));
            }
        }
    }
    if ok && !(st.split_failed || st.tx_failed || st.vfo_failed || (civ && st.civ_failed) || st.rx_mode_failed || st.tx_mode_failed) {
        st.radio_error = None;
    }
    ok
}

/// Cio' che serve al rotore per decidere.
struct RotorView {
    norad: u64,
    visible: bool,
    az: f64,
    el: f64,
    next_pass: Option<Pass>,
}

/// L'azimut di sorgere del primo passaggio nelle prossime 12 ore fra i satelliti seguiti.
fn next_rise_az(inner: &Inner, s: &SatSettings, obs: &Observer, t: f64) -> Option<f64> {
    let mut ids = s.tracked.clone();
    if let Some(sel) = s.selected && !ids.contains(&sel) {
        ids.push(sel);
    }
    let sats = lock(&inner.sats).clone();
    let mut best: Option<Pass> = None;
    for n in ids {
        let Some(sat) = sats.iter().find(|x| x.norad == n) else { continue };
        if let Some(p) = orbit::passes(sat, obs, t, 12.0, s.min_el_deg.max(0.0)).into_iter().find(|p| p.aos_unix > t)
            && best.is_none_or(|b| p.aos_unix < b.aos_unix)
        {
            best = Some(p);
        }
    }
    best.map(|p| p.aos_az_deg)
}

/// Il rotore in un secondo: PRIMA (va al punto di sorgere `rotor_lead_s` secondi prima), DURANTE (segue ogni `rotor_interval_s` secondi se
/// si sposta oltre la tolleranza) e DOPO il LOS (resta, parcheggia o va al prossimo punto di sorgere; un solo comando, annullato se riparte
/// un passaggio).
fn drive_rotor(rotor: &dyn SatRotator, st: &mut LoopState, s: &SatSettings, t: f64, v: &RotorView, next_rise: &dyn Fn() -> Option<f64>) {
    fn send(rotor: &dyn SatRotator, st: &mut LoopState, t: f64, az: f64, el: f64) {
        match rotor.point(az, el) {
            Ok(()) => {
                st.last_rotor = Some((az, el));
                st.last_rotor_cmd_t = t;
                st.rotor_error = None;
            }
            Err(e) => st.rotor_error = Some(e),
        }
    }
    if st.rotor_norad != Some(v.norad) {
        // un altro satellite: non e' un LOS
        st.rotor_norad = Some(v.norad);
        st.rotor_tracking = false;
        st.los_at = None;
    }
    if v.visible {
        // un passaggio e' in corso: il comando di fine passaggio atteso si annulla
        st.los_at = None;
        if v.el >= s.rotor_start_el_deg {
            st.rotor_tracking = true;
            if t - st.last_rotor_cmd_t >= f64::from(s.rotor_interval_s)
                && let Some((az, el)) = rotor_command(st.last_rotor, v.az, v.el.clamp(0.0, 90.0), s.rotor_min_move_deg, s.rotor_az_offset_deg)
            {
                send(rotor, st, t, az, el);
            }
        }
        return;
    }
    if st.rotor_tracking {
        st.rotor_tracking = false;
        st.los_at = Some(t);
    }
    let lead = f64::from(s.rotor_lead_s);
    if s.rotor_preposition && s.rotor_lead_s > 0 && let Some(p) = v.next_pass.filter(|p| p.aos_unix > t && p.aos_unix - t <= lead) && st.prepointed_for != Some(p.aos_unix) {
        // poco prima del passaggio ci si mette dove il satellite sorgera': parte un nuovo passaggio, il comando di fine passaggio si annulla
        st.prepointed_for = Some(p.aos_unix);
        st.los_at = None;
        if let Some((az, el)) = rotor_command(st.last_rotor, p.aos_az_deg, 0.0, s.rotor_min_move_deg, s.rotor_az_offset_deg) {
            send(rotor, st, t, az, el);
        }
        return;
    }
    if let Some(at) = st.los_at && t - at >= f64::from(s.rotor_after_delay_s) {
        st.los_at = None;
        match s.rotor_after.as_str() {
            "park" => send(rotor, st, t, s.rotor_park_az_deg.rem_euclid(360.0), s.rotor_park_el_deg.clamp(0.0, 90.0)),
            "next" => {
                if let Some(az) = next_rise() && let Some((az, el)) = rotor_command(st.last_rotor, az, 0.0, s.rotor_min_move_deg, s.rotor_az_offset_deg) {
                    send(rotor, st, t, az, el);
                }
            }
            _ => {}
        }
    }
}

fn tick(inner: &Arc<Inner>, st: &mut LoopState) {
    let settings = lock(&inner.settings).clone();
    let t = now_unix();
    let radio = lock(&inner.radio).clone();
    let rotor = lock(&inner.rotor).clone();
    let mut now = Now { time_unix: t, radio: HardwareState { ready: radio.as_ref().is_some_and(|r| r.ready()), error: st.radio_error.clone(), note: radio.as_ref().filter(|_| settings.uplink_civ_ic820h).and_then(|r| r.uplink_note()) }, ..Now::default() };
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
    // il Doppler muove la radio solo con la pagina dei satelliti aperta: prima, acceso una volta, a ogni passaggio di un satellite seguito
    // metteva la radio in split sul VFO B anche lavorando in FT8 o nei modi testuali, e lo split restava acceso dopo il passaggio
    // (proprietario, 2026-11-14)
    let page_open = lock(&inner.seen).is_some_and(|t| t.elapsed() < PAGE_GONE);
    if settings.doppler_on && page_open && now.visible && let (Some(radio), Some(plan), Some(tp)) = (&radio, &now.plan, &transponder) && radio.ready() {
        now.radio_active = drive_radio(radio.as_ref(), st, &settings, plan, tp, t);
    } else {
        // lo split acceso per il Doppler si toglie quando il Doppler si spegne, il satellite tramonta o si lascia la pagina
        if st.split_on
            && let Some(radio) = &radio
            && radio.ready()
            && let Err(e) = radio.set_split(false)
        {
            tracing::warn!("satelliti: split non spento alla fine del Doppler: {e}");
        }
        st.reset_pass();
        // i comandi che la radio ha rifiutato si dimenticano solo se la radio non c'e' piu': tra un passaggio e l'altro la memoria resta
        if radio.as_ref().is_none_or(|r| !r.ready()) {
            st.reset_failures();
            st.radio_error = None;
        }
    }
    now.radio.error = st.radio_error.clone();

    // ---- il rotore segue il satellite ----
    if settings.rotor_on {
        if let Some(rotor) = rotor.as_ref().filter(|r| r.ready()) {
            let view = RotorView { norad, visible: now.visible, az: look.az_deg, el: look.el_deg, next_pass: now.next_pass };
            let next_rise = || next_rise_az(inner, &settings, &obs, t);
            drive_rotor(rotor.as_ref(), st, &settings, t, &view, &next_rise);
        }
        st.rotor_was_on = true;
    } else if st.rotor_was_on {
        st.rotor_was_on = false;
        st.last_rotor = None;
        st.rotor_tracking = false;
        st.los_at = None;
    }
    if let Some((az, el)) = st.last_rotor {
        // si mostra dove punta il satellite, senza la correzione dell'azimut
        now.rotor.az_deg = (az - settings.rotor_az_offset_deg).rem_euclid(360.0);
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

    fn pass_at(aos: f64, max_el: f64) -> Pass {
        Pass { aos_unix: aos, los_unix: aos + 600.0, max_el_deg: max_el, max_el_unix: aos + 300.0, aos_az_deg: 10.0, los_az_deg: 200.0, max_az_deg: 100.0, duration_s: 600.0 }
    }

    #[test]
    fn i_passaggi_di_tutti_si_ordinano_per_inizio() {
        let v = merge_passes(vec![(1, "A".into(), vec![pass_at(300.0, 20.0), pass_at(900.0, 30.0)]), (2, "B".into(), vec![pass_at(100.0, 40.0), pass_at(500.0, 50.0)])], 0.0);
        let order: Vec<(u64, f64)> = v.iter().map(|p| (p.norad, p.pass.aos_unix)).collect();
        assert_eq!(order, vec![(2, 100.0), (1, 300.0), (2, 500.0), (1, 900.0)]);
        assert_eq!(v[0].name, "B");
    }

    #[test]
    fn i_passaggi_di_tutti_si_filtrano_per_elevazione() {
        let v = merge_passes(vec![(1, "A".into(), vec![pass_at(300.0, 5.0), pass_at(900.0, 30.0)]), (2, "B".into(), vec![pass_at(100.0, 10.0)])], 10.0);
        let order: Vec<u64> = v.iter().map(|p| p.norad).collect();
        assert_eq!(order, vec![2, 1]);
        assert!(merge_passes(Vec::new(), 0.0).is_empty());
    }

    #[test]
    fn il_rotore_non_si_muove_sotto_la_soglia() {
        // primo comando: sempre
        assert_eq!(rotor_command(None, 100.0, 20.0, 2.0, 0.0), Some((100.0, 20.0)));
        // sotto soglia sia in azimut sia in elevazione: niente
        assert_eq!(rotor_command(Some((100.0, 20.0)), 101.5, 21.5, 2.0, 0.0), None);
        // un solo asse oltre la soglia basta
        assert_eq!(rotor_command(Some((100.0, 20.0)), 102.0, 20.5, 2.0, 0.0), Some((102.0, 20.5)));
        assert_eq!(rotor_command(Some((100.0, 20.0)), 100.5, 22.5, 2.0, 0.0), Some((100.5, 22.5)));
        // il confronto passa dal nord: 359 -> 1 sono 2 gradi
        assert_eq!(rotor_command(Some((359.0, 0.0)), 0.5, 0.0, 2.0, 0.0), None);
        assert!(rotor_command(Some((359.0, 0.0)), 1.5, 0.0, 2.0, 0.0).is_some());
        // soglia zero: ogni cambiamento (e anche lo stesso valore) parte
        assert!(rotor_command(Some((100.0, 20.0)), 100.0, 20.0, 0.0, 0.0).is_some());
    }

    #[test]
    fn la_correzione_dell_azimut_si_somma_e_si_normalizza() {
        assert_eq!(rotor_command(None, 350.0, 10.0, 2.0, 15.0), Some((5.0, 10.0)));
        assert_eq!(rotor_command(None, 10.0, 10.0, 2.0, -15.0), Some((355.0, 10.0)));
        assert_eq!(rotor_command(None, 90.0, 10.0, 2.0, 0.0), Some((90.0, 10.0)));
        assert_eq!(rotor_command(None, 180.0, 0.0, 2.0, 180.0), Some((0.0, 0.0)));
        // la soglia confronta i comandi gia' corretti
        assert_eq!(rotor_command(Some((5.0, 10.0)), 350.5, 10.0, 2.0, 15.0), None);
    }

    #[test]
    fn le_impostazioni_del_rotore_si_controllano_e_hanno_i_valori_di_base() {
        let d = SatSettings::default();
        assert_eq!((d.rotor_min_move_deg, d.rotor_az_offset_deg), (2.0, 0.0));
        let old: SatSettings = serde_json::from_str("{\"rotor_on\": true}").expect("vecchio file");
        assert_eq!((old.rotor_min_move_deg, old.rotor_az_offset_deg), (2.0, 0.0));
        assert!(SatSettings { rotor_min_move_deg: 50.0, ..SatSettings::default() }.validate().is_err());
        assert!(SatSettings { rotor_min_move_deg: -1.0, ..SatSettings::default() }.validate().is_err());
        assert!(SatSettings { rotor_az_offset_deg: 200.0, ..SatSettings::default() }.validate().is_err());
        assert!(SatSettings { rotor_az_offset_deg: f64::NAN, ..SatSettings::default() }.validate().is_err());
        assert!(SatSettings { rotor_az_offset_deg: -45.0, rotor_min_move_deg: 0.0, ..SatSettings::default() }.validate().is_ok());
    }

    #[test]
    fn i_tipi_di_mappa() {
        assert_eq!(content_type("Contour HiRes.JPG"), Some("image/jpeg"));
        assert_eq!(content_type("a.bmp"), Some("image/bmp"));
        assert_eq!(content_type("a.txt"), None);
    }

    // ---- radio e rotore finti ----

    #[derive(Default)]
    struct FakeRadio {
        log: Mutex<Vec<String>>,
        split_err: bool,
        vfo_err: bool,
        civ_err: bool,
    }

    impl FakeRadio {
        fn count(&self, head: &str) -> usize {
            lock(&self.log).iter().filter(|c| c.starts_with(head)).count()
        }
    }

    impl SatRadio for FakeRadio {
        fn ready(&self) -> bool {
            true
        }
        fn set_rx_hz(&self, hz: u64) -> Result<(), String> {
            lock(&self.log).push(format!("rx {hz}"));
            Ok(())
        }
        fn set_tx_hz(&self, hz: u64) -> Result<(), String> {
            lock(&self.log).push(format!("tx {hz}"));
            Ok(())
        }
        fn set_split(&self, on: bool) -> Result<(), String> {
            lock(&self.log).push(format!("split {on}"));
            if self.split_err { Err("S: funzione non disponibile (-11)".into()) } else { Ok(()) }
        }
        fn set_rx_mode(&self, mode: &str) -> Result<(), String> {
            lock(&self.log).push(format!("rxmode {mode}"));
            Ok(())
        }
        fn set_tx_mode(&self, mode: &str) -> Result<(), String> {
            lock(&self.log).push(format!("txmode {mode}"));
            Ok(())
        }
        fn set_tx_via_vfo(&self, hz: u64, mode: Option<&str>) -> Result<bool, String> {
            lock(&self.log).push(format!("vfo {hz} {}", mode.unwrap_or("-")));
            if self.vfo_err { Err("V: funzione non disponibile (-11)".into()) } else { Ok(true) }
        }
        fn set_tx_via_civ(&self, hz: u64) -> Result<bool, String> {
            lock(&self.log).push(format!("civ {hz}"));
            if self.civ_err { Err("prova CI-V fallita: 07 D1 e 07 D0 non cambiano banda".into()) } else { Ok(true) }
        }
        fn enable_satmode(&self) -> Result<(), String> {
            lock(&self.log).push("satmode".into());
            Err("U: funzione non disponibile (-11)".into())
        }
    }

    fn plan_at(rate: f64) -> DopplerPlan {
        doppler_plan(&look_with_rate(rate), &rs44(), 435.625e6, 0.0)
    }

    #[test]
    fn lo_split_rifiutato_si_prova_una_volta_sola_e_il_doppler_va_avanti() {
        let radio = FakeRadio { split_err: true, ..FakeRadio::default() };
        let mut st = LoopState::default();
        let s = SatSettings::default();
        for i in 0..6 {
            assert!(drive_radio(&radio, &mut st, &s, &plan_at(-5.0 + f64::from(i)), &rs44(), f64::from(i)));
        }
        assert_eq!(radio.count("split"), 1);
        assert_eq!(radio.count("rx "), 6);
        assert!(st.radio_error.as_deref().is_some_and(|e| e.contains("due VFO")));
    }

    #[test]
    fn il_modo_non_si_ripete_ogni_secondo() {
        let radio = FakeRadio::default();
        let mut st = LoopState::default();
        let s = SatSettings::default();
        for i in 0..6 {
            drive_radio(&radio, &mut st, &s, &plan_at(-5.0 + f64::from(i)), &rs44(), f64::from(i));
        }
        assert_eq!(radio.count("rxmode USB"), 1);
        assert_eq!(radio.count("txmode LSB"), 1);
        // un'altra scelta dell'operatore: un solo comando nuovo
        let s = SatSettings { rx_mode: "LSB".into(), ..SatSettings::default() };
        drive_radio(&radio, &mut st, &s, &plan_at(1.0), &rs44(), 10.0);
        drive_radio(&radio, &mut st, &s, &plan_at(2.0), &rs44(), 11.0);
        assert_eq!(radio.count("rxmode LSB"), 1);
    }

    #[test]
    fn il_ripiego_a_due_vfo_non_tocca_lo_split_e_scrive_la_trasmissione_a_intervalli() {
        let radio = FakeRadio::default();
        let mut st = LoopState::default();
        let s = SatSettings { uplink_two_vfo: true, ..SatSettings::default() };
        // a t=0 primo giro: frequenza e modo (LSB, transponder invertente); satmode provato una volta
        drive_radio(&radio, &mut st, &s, &plan_at(-5.0), &rs44(), 0.0);
        assert_eq!(radio.count("split"), 0);
        assert_eq!(radio.count("satmode"), 1);
        assert_eq!(radio.count("vfo "), 1);
        assert!(lock(&radio.log).iter().any(|c| c.ends_with(" LSB") && c.starts_with("vfo ")));
        // i secondi dopo, con poca differenza, niente
        for i in 1..9 {
            drive_radio(&radio, &mut st, &s, &plan_at(-5.0 + 0.01 * f64::from(i)), &rs44(), f64::from(i));
        }
        assert_eq!(radio.count("vfo "), 1);
        // passati 10 s e spostata di almeno 5 Hz: una scrittura, senza modo
        drive_radio(&radio, &mut st, &s, &plan_at(-4.8), &rs44(), 10.5);
        assert_eq!(radio.count("vfo "), 2);
        assert!(lock(&radio.log).last().is_some_and(|c| c.ends_with(" -")));
        // un salto grande prima dell'intervallo: dopo 2 s si scrive
        drive_radio(&radio, &mut st, &s, &plan_at(2.0), &rs44(), 11.0);
        assert_eq!(radio.count("vfo "), 2);
        drive_radio(&radio, &mut st, &s, &plan_at(2.0), &rs44(), 12.6);
        assert_eq!(radio.count("vfo "), 3);
        assert_eq!(radio.count("satmode"), 1);
    }

    #[test]
    fn il_ripiego_si_spegne_da_solo_al_primo_rifiuto() {
        let radio = FakeRadio { vfo_err: true, ..FakeRadio::default() };
        let mut st = LoopState::default();
        let s = SatSettings { uplink_two_vfo: true, ..SatSettings::default() };
        for i in 0..8 {
            assert!(drive_radio(&radio, &mut st, &s, &plan_at(-5.0 + f64::from(i)), &rs44(), f64::from(i) * 5.0));
        }
        assert_eq!(radio.count("vfo "), 1);
        assert!(st.vfo_failed);
        assert!(st.radio_error.as_deref().is_some_and(|e| e.contains("due VFO") && e.contains("riprovo")));
        // la ricezione col Doppler e' continuata
        assert_eq!(radio.count("rx "), 8);
    }

    /// IC-820H con il Sub spento a meta' passaggio (pacchetto di Vladimir, #83, 2026-11-21): il ripiego si fermava per sempre, e la spunta
    /// «Trasmissione con due VFO» si toglieva da sola dalle impostazioni salvate, anche dopo aver riacceso il Sub. Ora riprova da solo dopo
    /// un minuto e, se la radio risponde, la trasmissione torna a seguire il Doppler.
    #[test]
    fn il_ripiego_riprova_da_solo_dopo_un_minuto_e_non_toglie_la_spunta() {
        let spenta = FakeRadio { vfo_err: true, ..FakeRadio::default() };
        let mut st = LoopState::default();
        let s = SatSettings { uplink_two_vfo: true, ..SatSettings::default() };
        drive_radio(&spenta, &mut st, &s, &plan_at(-5.0), &rs44(), 0.0);
        assert!(st.vfo_failed);
        // il Sub viene riacceso: prima del minuto non si riprova, dopo si'
        let accesa = FakeRadio::default();
        drive_radio(&accesa, &mut st, &s, &plan_at(-4.0), &rs44(), 30.0);
        assert_eq!(accesa.count("vfo "), 0, "prima del minuto non si tocca la radio");
        drive_radio(&accesa, &mut st, &s, &plan_at(-3.0), &rs44(), 61.0);
        assert_eq!(accesa.count("vfo "), 1, "dopo il minuto si riprova");
        assert!(!st.vfo_failed);
        assert!(st.radio_error.is_none());
        assert!(s.uplink_two_vfo);
    }

    #[test]
    fn la_trasmissione_ci_v_prende_il_posto_dello_split_e_dei_due_vfo() {
        let radio = FakeRadio::default();
        let mut st = LoopState::default();
        let s = SatSettings { uplink_civ_ic820h: true, uplink_two_vfo: true, ..SatSettings::default() };
        drive_radio(&radio, &mut st, &s, &plan_at(-5.0), &rs44(), 0.0);
        assert_eq!(radio.count("civ "), 1);
        assert_eq!(radio.count("vfo "), 0);
        assert_eq!(radio.count("split"), 0);
        assert_eq!(radio.count("tx "), 0);
        assert!(st.radio_error.is_none());
    }

    #[test]
    fn la_trasmissione_ci_v_fallita_si_riprova_dopo_un_minuto_e_la_ricezione_va_avanti() {
        let radio = FakeRadio { civ_err: true, ..FakeRadio::default() };
        let mut st = LoopState::default();
        let mut s = SatSettings { uplink_civ_ic820h: true, ..SatSettings::default() };
        for i in 0..59 {
            assert!(drive_radio(&radio, &mut st, &s, &plan_at(-5.0 + f64::from(i) * 0.01), &rs44(), f64::from(i)));
        }
        assert_eq!(radio.count("civ "), 1, "un solo tentativo nel primo minuto");
        assert_eq!(radio.count("rx "), 59);
        drive_radio(&radio, &mut st, &s, &plan_at(-4.0), &rs44(), 61.0);
        assert_eq!(radio.count("civ "), 2, "dopo un minuto si riprova da sola (SAT riacceso sulla radio)");
        assert!(st.radio_error.as_deref().is_some_and(|e| e.contains("CI-V")));
        // spenta e riaccesa dall'operatore: si riprova una volta
        s.uplink_civ_ic820h = false;
        drive_radio(&radio, &mut st, &s, &plan_at(-3.0), &rs44(), 300.0);
        s.uplink_civ_ic820h = true;
        drive_radio(&radio, &mut st, &s, &plan_at(-2.0), &rs44(), 301.0);
        assert_eq!(radio.count("civ "), 3);
    }

    #[test]
    fn i_modi_automatici_e_la_scelta_dell_operatore() {
        assert_eq!(sat_modes(&rs44(), "Auto", "Auto"), ("USB".to_string(), "LSB".to_string()));
        assert_eq!(sat_modes(&rs44(), "usb", "CW"), ("USB".to_string(), "CW".to_string()));
        assert!(SatSettings { rx_mode: "AM".into(), ..SatSettings::default() }.validate().is_err());
    }

    #[test]
    fn un_vecchio_file_delle_impostazioni_ha_i_valori_di_fabbrica_nuovi() {
        let old: SatSettings = serde_json::from_str("{\"doppler_on\": true, \"rotor_on\": true}").expect("vecchio file");
        assert!(!old.uplink_two_vfo);
        assert_eq!((old.uplink_interval_s, old.uplink_threshold_hz), (10, 300));
        assert_eq!((old.rotor_lead_s, old.rotor_interval_s, old.rotor_after_delay_s), (60, 2, 10));
        assert_eq!((old.rotor_after.as_str(), old.rotor_start_el_deg), ("stay", 0.0));
        assert!(old.validate().is_ok());
        assert!(SatSettings { rotor_lead_s: 601, ..SatSettings::default() }.validate().is_err());
        assert!(SatSettings { rotor_interval_s: 0, ..SatSettings::default() }.validate().is_err());
        assert!(SatSettings { rotor_after: "boh".into(), ..SatSettings::default() }.validate().is_err());
        assert!(SatSettings { rotor_park_el_deg: 91.0, ..SatSettings::default() }.validate().is_err());
        assert!(SatSettings { rotor_after_delay_s: 301, ..SatSettings::default() }.validate().is_err());
        assert!(SatSettings { rotor_start_el_deg: 31.0, ..SatSettings::default() }.validate().is_err());
    }

    #[derive(Default)]
    struct FakeRotor {
        points: Mutex<Vec<(f64, f64)>>,
    }

    impl SatRotator for FakeRotor {
        fn ready(&self) -> bool {
            true
        }
        fn point(&self, az: f64, el: f64) -> Result<(), String> {
            lock(&self.points).push((az, el));
            Ok(())
        }
        fn stop(&self) {}
    }

    fn view(visible: bool, az: f64, el: f64, aos: Option<f64>) -> RotorView {
        RotorView { norad: 25544, visible, az, el, next_pass: aos.map(|a| pass_at(a, 40.0)) }
    }

    fn no_next() -> Option<f64> {
        None
    }

    #[test]
    fn il_rotore_parte_solo_nei_secondi_di_anticipo_scelti() {
        let rotor = FakeRotor::default();
        let mut st = LoopState::default();
        let s = SatSettings { rotor_lead_s: 60, ..SatSettings::default() };
        // a 100 s dal sorgere: fermo
        drive_rotor(&rotor, &mut st, &s, 1000.0, &view(false, 0.0, -10.0, Some(1100.0)), &no_next);
        assert!(lock(&rotor.points).is_empty());
        // a 50 s: un comando verso l'azimut di sorgere (10 gradi), elevazione 0; poi non si ripete
        drive_rotor(&rotor, &mut st, &s, 1050.0, &view(false, 0.0, -10.0, Some(1100.0)), &no_next);
        drive_rotor(&rotor, &mut st, &s, 1051.0, &view(false, 0.0, -10.0, Some(1100.0)), &no_next);
        assert_eq!(*lock(&rotor.points), vec![(10.0, 0.0)]);
        // anticipo 0: solo all'AOS
        let rotor = FakeRotor::default();
        let mut st = LoopState::default();
        let s = SatSettings { rotor_lead_s: 0, ..SatSettings::default() };
        drive_rotor(&rotor, &mut st, &s, 1099.0, &view(false, 0.0, -10.0, Some(1100.0)), &no_next);
        assert!(lock(&rotor.points).is_empty());
    }

    #[test]
    fn durante_il_passaggio_un_comando_ogni_intervallo_e_solo_oltre_la_tolleranza() {
        let rotor = FakeRotor::default();
        let mut st = LoopState::default();
        let s = SatSettings { rotor_interval_s: 2, rotor_min_move_deg: 2.0, rotor_start_el_deg: 5.0, ..SatSettings::default() };
        // sotto l'elevazione di partenza non segue
        drive_rotor(&rotor, &mut st, &s, 0.0, &view(true, 100.0, 3.0, None), &no_next);
        assert!(lock(&rotor.points).is_empty());
        // ogni secondo il satellite si sposta di 1,5 gradi: con intervallo 2 s e tolleranza 2 gradi un comando ogni 2 s
        for i in 1..=6 {
            drive_rotor(&rotor, &mut st, &s, f64::from(i), &view(true, 100.0 + 1.5 * f64::from(i), 20.0, None), &no_next);
        }
        let n = lock(&rotor.points).len();
        assert_eq!(n, 3, "{:?}", lock(&rotor.points));
        // fermo: niente di nuovo
        drive_rotor(&rotor, &mut st, &s, 20.0, &view(true, 109.0, 20.0, None), &no_next);
        drive_rotor(&rotor, &mut st, &s, 21.0, &view(true, 109.0, 20.0, None), &no_next);
        assert_eq!(lock(&rotor.points).len(), n);
    }

    fn pass_then_los(rotor: &FakeRotor, st: &mut LoopState, s: &SatSettings) {
        drive_rotor(rotor, st, s, 100.0, &view(true, 100.0, 30.0, None), &no_next);
        drive_rotor(rotor, st, s, 101.0, &view(true, 110.0, 25.0, None), &no_next);
        // LOS a t=102
        drive_rotor(rotor, st, s, 102.0, &view(false, 120.0, -1.0, None), &no_next);
    }

    #[test]
    fn dopo_il_los_resta_fermo_se_non_si_sceglie_il_parcheggio() {
        let rotor = FakeRotor::default();
        let mut st = LoopState::default();
        let s = SatSettings { rotor_after: "stay".into(), rotor_after_delay_s: 10, ..SatSettings::default() };
        pass_then_los(&rotor, &mut st, &s);
        let n = lock(&rotor.points).len();
        for t in [105.0, 112.0, 130.0] {
            drive_rotor(&rotor, &mut st, &s, t, &view(false, 120.0, -5.0, None), &no_next);
        }
        assert_eq!(lock(&rotor.points).len(), n);
    }

    #[test]
    fn il_parcheggio_parte_una_volta_dopo_il_ritardo() {
        let rotor = FakeRotor::default();
        let mut st = LoopState::default();
        let s = SatSettings { rotor_after: "park".into(), rotor_after_delay_s: 10, rotor_park_az_deg: 350.0, rotor_park_el_deg: 15.0, rotor_az_offset_deg: 20.0, ..SatSettings::default() };
        pass_then_los(&rotor, &mut st, &s);
        let n = lock(&rotor.points).len();
        drive_rotor(&rotor, &mut st, &s, 105.0, &view(false, 120.0, -5.0, None), &no_next);
        assert_eq!(lock(&rotor.points).len(), n, "prima del ritardo niente");
        for t in [112.0, 113.0, 140.0] {
            drive_rotor(&rotor, &mut st, &s, t, &view(false, 120.0, -5.0, None), &no_next);
        }
        // un solo comando, nella posizione scelta e senza la correzione dell'azimut
        assert_eq!(lock(&rotor.points).len(), n + 1);
        assert_eq!(lock(&rotor.points).last().copied(), Some((350.0, 15.0)));
    }

    #[test]
    fn il_parcheggio_si_annulla_se_parte_un_nuovo_passaggio() {
        let rotor = FakeRotor::default();
        let mut st = LoopState::default();
        let s = SatSettings { rotor_after: "park".into(), rotor_after_delay_s: 10, rotor_park_az_deg: 0.0, rotor_park_el_deg: 0.0, rotor_lead_s: 60, ..SatSettings::default() };
        pass_then_los(&rotor, &mut st, &s);
        // prima del ritardo parte la preparazione del passaggio seguente (a 50 s dal sorgere): niente parcheggio
        drive_rotor(&rotor, &mut st, &s, 106.0, &view(false, 120.0, -5.0, Some(156.0)), &no_next);
        let pts = lock(&rotor.points).clone();
        assert_eq!(pts.last().copied(), Some((10.0, 0.0)));
        for t in [115.0, 130.0] {
            drive_rotor(&rotor, &mut st, &s, t, &view(false, 120.0, -5.0, Some(156.0)), &no_next);
        }
        assert_eq!(*lock(&rotor.points), pts, "nessun parcheggio dopo");
    }

    #[test]
    fn dopo_il_los_va_al_prossimo_punto_di_sorgere_se_c_e() {
        let rotor = FakeRotor::default();
        let mut st = LoopState::default();
        let s = SatSettings { rotor_after: "next".into(), rotor_after_delay_s: 5, ..SatSettings::default() };
        pass_then_los(&rotor, &mut st, &s);
        let n = lock(&rotor.points).len();
        let next = || Some(215.0);
        drive_rotor(&rotor, &mut st, &s, 107.0, &view(false, 120.0, -5.0, None), &next);
        drive_rotor(&rotor, &mut st, &s, 108.0, &view(false, 120.0, -5.0, None), &next);
        assert_eq!(lock(&rotor.points).len(), n + 1);
        assert_eq!(lock(&rotor.points).last().copied(), Some((215.0, 0.0)));
    }
}
