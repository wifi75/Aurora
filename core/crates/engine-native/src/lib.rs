//! Il motore proprio di Aurora, parte di ricezione. Prende l'audio dell'ingresso (lo stesso della cascata), lo porta a 12 kHz, lo divide nei
//! periodi FT8 (15 secondi) o FT4 (7,5 secondi, a scelta con il pulsante del modo) e lo decodifica con il crate `ft8` o `ft4`. Per ora lavora «in ombra» accanto a JTDX: i messaggi si scrivono in
//! `native-decodes.log` (nella cartella dei dati) e si confrontano con quelli di JTDX (`GET /api/v1/engine/native`), senza toccare l'elenco
//! che vede l'utente. Quando il confronto e' abbastanza buono il motore proprio prendera' il posto di JTDX.

pub mod fox;
pub mod otp;
pub mod qpc;
pub mod ultrafox;
pub mod ultrafox_rx;
mod alltxt;
mod known;
mod mskrt;
pub use ft8::filters::GridCheck;
pub mod features;
mod parity;
mod persist;
mod resample;
mod rig;
mod slots;
mod slow;
mod smeter;
mod tx;
mod wav;

use std::{
    io::Write,
    path::PathBuf,
    sync::{atomic::{AtomicU64, Ordering}, mpsc, Arc, Mutex},
    time::{Instant, SystemTime, UNIX_EPOCH},
};

use audio_spectrum::AudioChunk;
use aurora_core::{DecodeTag, DecodedMessage, Event, StationStatus};
use ft8::{decode::Options, message::CallHashes};
use qso_native::{CallerInfo, CallerInfoFn, Mode, Options as SeqOptions};
use serde_json::{json, Value};
use tokio::sync::broadcast;

pub use alltxt::{decode_line as all_txt_decode_line, format_g as all_txt_format_g, tx_line as all_txt_tx_line, AllTxt, AllTxtScope, Utc};
pub use parity::{compare, Parity, ParitySummary, SlotSummary};
pub use rig::{NativeControls, NativeRig, SavedControls};
pub use wav::{jtdx_stem, jtdx_wav_bytes, save_jtdx_style, SaveInfo, SaveMode, SaveSettings};
pub use tx::{OptionChange, SequenceSettings, TxCommand, TxDriver, TxNote, TxPort, TxView};

/// Campioni al secondo del decoder.
pub const RATE: usize = ft8::SAMPLE_RATE;

const DAY_MS: u64 = 86_400_000;

#[derive(Default)]
struct Stats {
    last_slot_unix_ms: u64,
    last_count: usize,
    last_millis: u64,
    slots_decoded: u64,
}

/// Paese, continente ed etichetta di novita' di un messaggio sulla banda data.
pub type Annotator = Arc<dyn Fn(&str, Option<&str>) -> (Option<String>, Option<String>, Option<DecodeTag>) + Send + Sync>;

/// Il motore proprio come **unico** motore (senza JTDX): pubblica i messaggi decodificati e lo stato della stazione sul bus.
#[derive(Clone)]
pub struct LiveConfig {
    pub de_call: String,
    pub de_grid: String,
    pub own_continent: Option<String>,
    /// Frequenza della radio in Hz (0 se non si sa): la aggiorna chi legge la radio.
    pub dial_hz: Arc<AtomicU64>,
    /// Lo stato dei pulsanti (Monitora, frequenza audio...): con Monitora spento non si decodifica.
    pub controls: Arc<NativeControls>,
    /// La banda («20m») per una frequenza della radio, per le etichette di novita'.
    pub band_of: Arc<dyn Fn(u64) -> Option<String> + Send + Sync>,
    pub annotate: Annotator,
    /// La catena di trasmissione (audio in uscita e PTT); senza, il motore riceve soltanto.
    pub transmitter: Option<Arc<dyn TxPort>>,
    /// I nominativi gia' collegati (il log): insegnano al filtro contro i falsi decode chi esiste. `None`: il filtro impara solo dall'ascolto.
    pub log_calls: Option<Arc<dyn Fn() -> Vec<String> + Send + Sync>>,
    /// Controllo del locatore sul paese del nominativo (`dxcc::DxccDb::grid_plausible`): scarta i decode deboli con un locatore impossibile.
    pub grid_check: Option<ft8::filters::GridCheck>,
}

/// Il motore proprio in funzione: il confronto con JTDX e le ultime misure.
pub struct NativeEngine {
    parity: Arc<Parity>,
    stats: Arc<Mutex<Stats>>,
}

impl NativeEngine {
    /// Stato e confronto, in forma di JSON per l'API.
    pub fn status(&self) -> Value {
        let stats = self.stats.lock().map(|s| (s.last_slot_unix_ms, s.last_count, s.last_millis, s.slots_decoded)).unwrap_or_default();
        json!({
            "enabled": true,
            "mode": "shadow",
            "slots_decoded": stats.3,
            "last_slot_unix_ms": stats.0,
            "last_slot_messages": stats.1,
            "last_decode_ms": stats.2,
            "comparison": serde_json::to_value(self.parity.summary()).unwrap_or(Value::Null),
        })
    }
}

/// Segue la banda della radio: dice quando cambia (da qualunque causa: pagina, radio, spot, modo che sposta la frequenza).
#[derive(Default)]
struct BandWatch {
    band: Option<String>,
    epoch: u64,
}

impl BandWatch {
    /// Una lettura della banda. Senza banda (radio muta, frequenza fuori dalle bande dei radioamatori) la lettura non conta: come JTDX con una
    /// risposta persa della radio (`displayDialFrequency` guarda solo bande note). La prima lettura non e' un cambio. Restituisce il nuovo
    /// `epoch` quando la banda e' diversa da quella nota; spostamenti dentro la stessa banda non fanno niente.
    fn observe(&mut self, band: Option<String>) -> Option<u64> {
        let band = band.filter(|b| !b.is_empty())?;
        let changed = self.band.as_deref().is_some_and(|old| old != band);
        self.band = Some(band);
        changed.then(|| {
            self.epoch += 1;
            self.epoch
        })
    }
}

fn now_unix_ms() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |d| d.as_millis() as u64)
}

/// Il periodo cui appartiene un messaggio di JTDX: `time_ms` sono i millisecondi dalla mezzanotte UTC.
pub fn slot_of_time_ms(time_ms: u32, now_unix_ms: u64) -> u64 {
    let day_start = now_unix_ms - now_unix_ms % DAY_MS;
    let mut slot = day_start + u64::from(time_ms) / slots::SLOT_MS * slots::SLOT_MS;
    if slot > now_unix_ms + 5_000 {
        slot -= DAY_MS;
    }
    slot
}

/// Salva un periodo come fa JTDX (`aammgg_hhmmss.wav` con il blocco `INFO`). FT4: JTDX tiene i soli 21 x 3456 campioni della trama.
fn save_period(dir: &std::path::Path, mode: Mode, slot: &slots::Slot, live: Option<&LiveConfig>) {
    let (dx_call, dx_grid) = live
        .map(|l| {
            let view = tx::view_of(&l.controls);
            (view.dx.unwrap_or_default(), view.dx_grid.unwrap_or_default())
        })
        .unwrap_or_default();
    let info = SaveInfo {
        call: live.map(|l| l.de_call.clone()).unwrap_or_default(),
        grid: live.map(|l| l.de_grid.clone()).unwrap_or_default(),
        mode: mode.name().to_string(),
        dial_hz: live.map_or(0, |l| l.dial_hz.load(Ordering::Relaxed)),
        dx_call,
        dx_grid,
        program: format!("Aurora v{}", aurora_core::version::APP_VERSION),
    };
    let samples: &[f32] = if mode == Mode::Ft4 { &slot.samples[..slot.samples.len().min(21 * 3456)] } else { &slot.samples };
    if let Err(e) = wav::save_jtdx_style(dir, slot.start_unix_ms, samples, &info, now_unix_ms()) {
        tracing::warn!("file audio del periodo non salvato in {}: {e}", dir.display());
    }
}

fn format_line(slot_unix_ms: u64, d: &ft8::decode::Decoded) -> String {
    let secs = slot_unix_ms / 1000 % 86_400;
    format!("{:02}{:02}{:02} {:>4} {:>5.1} {:>4} ~  {}\n", secs / 3600, secs / 60 % 60, secs % 60, d.snr_db, d.dt_s, d.freq_hz.round() as i64, d.text)
}

/// Pubblica lo stato della stazione (modo, frequenza, trasmissione, S-meter) quattro volte al secondo. Parte una volta sola per gli stessi
/// controlli: Aurora lo avvia subito, cosi' modo e periodo si vedono e si cambiano anche senza un ingresso audio aperto (il motore che
/// decodifica parte solo con l'audio).
pub fn publish_status(events: broadcast::Sender<Event>, live: LiveConfig) {
    if live.controls.status_published.swap(true, Ordering::SeqCst) {
        return;
    }
        tokio::spawn(async move {
            // lo stato (frequenza, S-meter, trasmissione) si pubblica quattro volte al secondo: a un secondo lo S-meter sembrava fermo
            let mut tick = tokio::time::interval(std::time::Duration::from_millis(250));
            let mut last_fox = aurora_core::FoxSnapshot::default();
            let mut fox_ticks: u32 = 0;
            loop {
                tick.tick().await;
                let view = tx::view_of(&live.controls);
                let selected_text = view.texts.get(usize::from(view.selected).saturating_sub(1)).filter(|t| !t.is_empty() && view.selected > 0).cloned();
                // in trasmissione il messaggio e' quello davvero in onda: la sequenza puo' trasmettere senza un messaggio selezionato,
                // e senza testo la pagina non registrava il Tx (la risposta della stazione chiamata sembrava una chiamata a me)
                let on_air = live.controls.on_air.lock().ok().and_then(|t| t.clone());
                let selected_text = if live.controls.transmitting.load(Ordering::Relaxed) { on_air.or(selected_text) } else { selected_text };
                let status = StationStatus {
                    dial_hz: live.dial_hz.load(Ordering::Relaxed),
                    mode: rig::mode_label(live.controls.mode()),
                    tx_enabled: view.enabled,
                    transmitting: live.controls.transmitting.load(Ordering::Relaxed),
                    dx_call: view.dx.clone(),
                    dx_grid: view.dx_grid.clone(),
                    tx_message: selected_text,
                    decoding: live.controls.is_monitoring(),
                    de_call: live.de_call.clone(),
                    de_grid: live.controls.grid_or(&live.de_grid),
                    rx_df: Some(live.controls.rx_df.load(Ordering::Relaxed)),
                    tx_df: Some(live.controls.tx_df.load(Ordering::Relaxed)),
                    // FT4 ha periodi di 7,5 s: il campo e' intero, come lo manda JTDX (7)
                    tr_period: Some(match live.controls.mode() {
                        Mode::Ft8 => 15,
                        Mode::Ft4 => 7,
                        Mode::Jt65 | Mode::Jt9 | Mode::Jt4 => 60,
                        Mode::Q65 => rig::q65_setup().period_s,
                        Mode::Msk144 => 15,
                    }),
                    own_continent: live.own_continent.clone(),
                    meters: live.controls.published_meters(),
                    queue: view.queue.clone(),
                    tx_alerts: live.controls.tx_alerts(),
                    fox_verification: live.controls.fox_verification(),
                    super_hound: live.controls.switch_on("hound") && live.controls.ultrafox_recent(now_unix_ms()),
                    ..StationStatus::default()
                };
                // lo stato della Fox quando cambia e comunque ogni 2 s (una pagina appena aperta lo riceve subito);
                // l'interruttore si legge qui, cosi' la pagina lo vede anche prima del primo periodo
                let mut fox = live.controls.fox_view.lock().map(|v| v.clone()).unwrap_or_default();
                fox.enabled = live.controls.switch_on("fox");
                fox_ticks = fox_ticks.wrapping_add(1);
                if fox != last_fox || fox_ticks % 8 == 1 {
                    let _ = events.send(Event::Fox(fox.clone()));
                    last_fox = fox;
                }
                if events.send(Event::Status(status)).is_err() && events.receiver_count() == 0 {
                    // nessuno ascolta ancora: si continua, la pagina si colleghera' dopo
                }
            }
        });
    }

/// Avvia il motore proprio: `tap` porta l'audio della scheda, `events` e' il bus da cui si leggono i messaggi di JTDX per il confronto.
/// Va chiamato dentro il runtime di Tokio.
/// Con `live` il motore proprio fa da unico motore: pubblica anche i messaggi e lo stato sul bus (senza `live` lavora solo in ombra).
/// Con `record` ogni periodo si salva anche in WAV nella cartella `recordings` dei dati, per provare il decoder sulle stesse registrazioni.
pub fn start(events: broadcast::Sender<Event>, tap: mpsc::Receiver<AudioChunk>, data_dir: PathBuf, record: bool, live: Option<LiveConfig>) -> Arc<NativeEngine> {
    start_with_save(events, tap, data_dir, record, live, None)
}

/// Come [`start`], con in piu' il salvataggio dei file audio della scheda Audio (come JTDX: Nessuno / Decodificati / Tutti, in una cartella a scelta).
/// `save` si puo' cambiare mentre il motore gira (`SaveSettings::set`): vale dal periodo dopo.
pub fn start_with_save(
    events: broadcast::Sender<Event>,
    tap: mpsc::Receiver<AudioChunk>,
    data_dir: PathBuf,
    record: bool,
    live: Option<LiveConfig>,
    save: Option<Arc<SaveSettings>>,
) -> Arc<NativeEngine> {
    let engine = Arc::new(NativeEngine { parity: Arc::new(Parity::default()), stats: Arc::new(Mutex::new(Stats::default())) });

    // i messaggi di JTDX, per il confronto
    let parity = Arc::clone(&engine.parity);
    let mut rx = events.subscribe();
    tokio::spawn(async move {
        loop {
            match rx.recv().await {
                // i messaggi del motore proprio (id alti) non sono di JTDX: non entrano nel confronto
                Ok(Event::Decode(m)) if m.id < 1_000_000_000 => parity.add_jtdx(slot_of_time_ms(m.time_ms, now_unix_ms()), &m.text, i64::from(m.freq_hz)),
                Ok(_) | Err(broadcast::error::RecvError::Lagged(_)) => {}
                Err(broadcast::error::RecvError::Closed) => break,
            }
        }
    });

    // lo stato della stazione, quando il motore proprio e' l'unico (se non lo pubblica gia' `publish_status`)
    if let Some(live) = live.clone() {
        publish_status(events.clone(), live);
    }

    // l'audio: resampling, periodi, decodifica
    let message_events = events;
    let next_id = Arc::new(AtomicU64::new(1_000_000_000));
    let (parity, stats) = (Arc::clone(&engine.parity), Arc::clone(&engine.stats));
    std::thread::spawn(move || {
        let log_path = data_dir.join("native-decodes.log");
        let comparison_path = data_dir.join("native-parity.log");
        let mut resampler: Option<(u32, resample::Resampler)> = None;
        // il modo si sceglie con il pulsante: cambia la durata dei periodi, il decoder e la trasmissione
        let mut mode = live.as_ref().map_or(Mode::Ft8, |l| l.controls.mode());
        let mut assembler = slots::SlotAssembler::new(mode);
        // Q65: periodo e sottomodo in uso (cambiano la durata dei periodi) e la media dei periodi deboli
        let mut q65_now = rig::q65_setup();
        let mut q65_avg = jt::q65::Averager::new();
        let mut msk_rt = mskrt::Realtime::default();
        let mut hashes = CallHashes::default();
        // messaggi gia' mostrati dalla decodifica anticipata FT8, per periodo: la decodifica intera pubblica solo il resto
        let mut preview_shown: std::collections::HashMap<u64, Vec<String>> = std::collections::HashMap::new();
        // i nominativi conosciuti (log e ascolto) per il filtro contro i falsi decode
        let mut known = known::KnownCalls::load(data_dir.join("known_calls.txt"));
        let mut known_seeded: Option<Instant> = None;
        let mut slow_watch = slow::SlowWatch::default();
        // i segnali CQ, le chiamate dirette a noi e il segnale del canale QSO tra un periodo FT8 e l'altro (sottopassate 3-11)
        let mut ft8_history = ft8::decode::PeriodHistory::default();
        let mut band_watch = BandWatch::default();
        // S-meter stimato dall'audio in ingresso (per le radio senza misuratori); taratura con AURORA_SMETER_S9_DB
        let mut s_meter = smeter::SMeter::new(smeter::s9_db_from_env());
        // `ALL.TXT` come JTDX, solo quando il motore proprio e' l'unico: i QSO registrati aspettano l'inizio della trasmissione che li chiude
        let mut all_txt = live.as_ref().map(|_| AllTxt::from_env(&data_dir, &format!("Aurora v{}", aurora_core::version::APP_VERSION)));
        let mut logged_due: Vec<(u64, String)> = Vec::new();
        // i periodi in cui si e' trasmesso: senza «Full duplex» non si decodificano (WSJT-X spegne il monitor in trasmissione)
        let mut tx_periods = features::TxPeriods::default();
        // profondita' 3 di JTDX («profonda»): soglia di sincronismo piu' bassa a rotazione e una seconda lettura a tempo
        // invertito, per i segnali piu' deboli (richiesta del proprietario, 2026-10-31: un segnale udibile non si decodificava)
        let ft8_options = Options::depth(3);
        let ft4_options = ft4::decode::Options::default();
        // l'ultimo periodo (per «Decodifica») e i messaggi che ne sono gia' stati pubblicati
        let mut last_slot: Option<(u64, Mode, Vec<f32>)> = None;
        let mut last_texts: Vec<String> = Vec::new();
        // la trasmissione: solo se c'e' la catena audio/PTT
        let (command_tx, command_rx) = mpsc::channel::<TxCommand>();
        // lo stato dei pulsanti e dei livelli impostati prima dello spegnimento si rimette com'era
        let controls_path = data_dir.join("controls.json");
        // memoria del livello Pwr per banda: spenta di fabbrica come in JTDX (`AURORA_PWR_BAND_MEMORY=1` la accende)
        if let Some(l) = live.as_ref() {
            l.controls.set_band_memory(std::env::var("AURORA_PWR_BAND_MEMORY").is_ok_and(|v| v == "1"));
        }
        let restored = live.as_ref().and_then(|l| {
            let saved = persist::load(&controls_path)?;
            l.controls.restore(&saved);
            tracing::info!("pulsanti e livelli ripristinati da {}", controls_path.display());
            Some(saved)
        });
        let mut last_saved = live.as_ref().map(|l| l.controls.saved());
        let mut last_save_check = std::time::Instant::now();
        let mut driver = live.as_ref().and_then(|l| {
            let port = l.transmitter.clone()?;
            l.controls.set_sender(command_tx.clone());
            // STOP TX (tasto Esc compreso) ferma audio e PTT subito, anche se il ciclo e' occupato a decodificare
            let halting = Arc::clone(&port);
            l.controls.set_halt_now(Arc::new(move || halting.halt()));
            // il livello (cursore Pwr) arriva al dispositivo subito, anche a trasmissione in corso, come in JTDX
            let leveling = Arc::clone(&port);
            l.controls.set_attenuation_hook(Arc::new(move |tenths| leveling.set_attenuation(tenths)));
            // il livello salvato (gia' ripristinato) arriva subito al dispositivo: prima restava quello di fabbrica finche' non si muoveva il cursore
            port.set_attenuation(l.controls.attenuation.load(Ordering::Relaxed));
            let base = SeqOptions { my_continent: l.own_continent.clone().unwrap_or_default(), ..SeqOptions::default() };
            match TxDriver::with_options(&l.de_call, &l.de_grid, port, Arc::clone(&l.controls), base) {
                Ok(mut d) => {
                    // continente e novita' delle stazioni per la scelta di chi chiamare (priorita' di JTDX, punteggio di Decodium)
                    d.set_caller_info(Some(caller_info_provider(l)));
                    // frequenza Tx libera (regola A): funzione nuova, spenta di base
                    if std::env::var("AURORA_FREE_TX").is_ok_and(|v| matches!(v.trim(), "1" | "true" | "on")) {
                        d.set_free_tx(true);
                    }
                    Some(d)
                }
                Err(e) => {
                    // dati di stazione ancora da compilare: e' una configurazione, non un guasto (avviso, niente rapporto)
                    tracing::warn!("trasmissione non attiva: nominativo o locatore non validi ({e})");
                    None
                }
            }
        });
        // il seme OTP della Fox salvato nella cartella dei dati torna al motore (prima si ricordava solo nel browser della pagina Fox)
        let fox_otp_path = data_dir.join(persist::FOX_OTP_FILE);
        if let (Some(d), Some(saved)) = (driver.as_mut(), persist::load_fox_otp(&fox_otp_path)) {
            if saved.seed.is_some() {
                tracing::info!("seme OTP della Fox ripristinato da {}", fox_otp_path.display());
            }
            let _ = d.handle(TxCommand::Fox(aurora_core::FoxCommand::Otp { seed: saved.seed, interval: saved.interval.clamp(1, 100), keep: false }), now_unix_ms() as f64 / 1000.0);
        }
        // il messaggio CQ (Tx6) scritto a mano resta tra un avvio e l'altro (scelta del proprietario, 2026-11-08)
        let cq_text_path = data_dir.join("tx6-cq.txt");
        if let (Some(d), Ok(text)) = (driver.as_mut(), std::fs::read_to_string(&cq_text_path)) {
            let text = text.trim().to_string();
            if !text.is_empty() {
                let _ = d.handle(TxCommand::SetTxText { n: 6, text }, now_unix_ms() as f64 / 1000.0);
            }
        }
        // gli interruttori ricordati arrivano anche alla sequenza (AutoSeq, Salta Tx1, Bypassa...), come se si fossero appena premuti
        if let (Some(saved), Some(l), Some(_)) = (restored.as_ref(), live.as_ref(), driver.as_ref()) {
            let rig = rig::NativeRig::new(Arc::clone(&l.controls), None);
            for (id, on) in &saved.switches {
                let _ = rig_control::Rig::set_control(&rig, id, Some(*on));
            }
            // Abilita Tx come era al riavvio (scelta del proprietario): la sequenza riparte armata, ma non trasmette finche' non c'e' un messaggio da mandare
            if saved.enable_tx {
                let _ = rig_control::Rig::set_control(&rig, "enable_tx", Some(true));
                tracing::info!("Abilita Tx ripristinato acceso, come prima dello spegnimento");
            }
        }
        loop {
            // ogni due secondi, se qualcosa e' cambiato, lo stato dei pulsanti e dei livelli si salva
            if last_save_check.elapsed() >= std::time::Duration::from_secs(2) {
                last_save_check = std::time::Instant::now();
                if let Some(l) = live.as_ref() {
                    let now_state = l.controls.saved();
                    if last_saved.as_ref() != Some(&now_state) {
                        match persist::save(&controls_path, &now_state) {
                            Ok(()) => last_saved = Some(now_state),
                            Err(e) => tracing::warn!("stato dei pulsanti non salvato: {e}"),
                        }
                    }
                }
            }
            // i comandi (doppio clic, CQ, STOP, opzioni) si servono subito, anche senza audio in arrivo
            while let Ok(command) = command_rx.try_recv() {
                if let Some(d) = driver.as_mut() {
                    let otp_changed = matches!(command, TxCommand::Fox(aurora_core::FoxCommand::Otp { .. }));
                    let cq_text = if let TxCommand::SetTxText { n: 6, text } = &command { Some(text.clone()) } else { None };
                    for line in d.handle(command, now_unix_ms() as f64 / 1000.0) {
                        tracing::info!("trasmissione: {line}");
                    }
                    // il seme e l'intervallo della verifica OTP si salvano subito, come li ha presi il motore
                    if otp_changed && let Err(e) = persist::save_fox_otp(&fox_otp_path, &d.fox_otp()) {
                        tracing::warn!("seme OTP della Fox non salvato: {e}");
                    }
                    // il CQ scritto a mano si ricorda (vuoto = torna quello standard e il file si toglie)
                    if let Some(text) = cq_text {
                        let text = text.trim();
                        let saved = if text.is_empty() { std::fs::remove_file(&cq_text_path).or_else(|e| if e.kind() == std::io::ErrorKind::NotFound { Ok(()) } else { Err(e) }) } else { std::fs::write(&cq_text_path, text) };
                        if let Err(e) = saved {
                            tracing::warn!("messaggio CQ non salvato: {e}");
                        }
                    }
                }
            }
            // cambio di banda (da qualunque causa): trasmissione ferma subito, Tx spento, DX e messaggi azzerati, periodo in corso scartato
            // (mescola le due bande) e un evento per l'interfaccia
            if let Some(l) = live.as_ref() {
                let dial = l.dial_hz.load(Ordering::Relaxed);
                // il livello di uscita e' ricordato per banda: con la banda nuova si ripristina quello suo (o si ricorda l'attuale)
                if dial > 0
                    && let Some(band) = (l.band_of)(dial)
                {
                    l.controls.track_band(&band);
                }
                if dial > 0
                    && let Some(epoch) = band_watch.observe((l.band_of)(dial))
                {
                    tracing::info!("motore proprio: cambio di banda ({}), epoca {epoch}", band_watch.band.as_deref().unwrap_or_default());
                    if let Some(d) = driver.as_mut() {
                        for line in d.band_changed() {
                            tracing::info!("trasmissione: {line}");
                        }
                    }
                    assembler = slots::SlotAssembler::new(mode);
                    last_slot = None;
                    last_texts.clear();
                    let _ = message_events.send(Event::BandChanged { band: band_watch.band.clone(), dial_hz: dial, epoch });
                }
            }
            // cambio di modo: periodi nuovi, trasmissione ferma, messaggi ricordati svuotati; l'audio continua ad arrivare senza interruzioni
            if let Some(l) = live.as_ref() {
                let wanted = l.controls.mode();
                if wanted != mode {
                    tracing::info!("motore proprio: modo {}", wanted.name());
                    mode = wanted;
                    assembler = slots::SlotAssembler::new(mode);
                    last_slot = None;
                    last_texts.clear();
                    q65_avg.clear();
                    if let Some(d) = driver.as_mut() {
                        for line in d.set_mode(mode) {
                            tracing::info!("trasmissione: {line}");
                        }
                    }
                }
                // Q65: «Clear Avg» svuota la media dei periodi
                if rig::take_q65_clear_avg() {
                    q65_avg.clear();
                    tracing::info!("motore proprio: media Q65 svuotata");
                }
                // Q65: un altro periodo o sottomodo rifa' i periodi come un cambio di modo
                let q65_wanted = rig::q65_setup();
                if q65_wanted != q65_now {
                    q65_now = q65_wanted;
                    q65_avg.clear();
                    if mode == Mode::Q65 {
                        tracing::info!("motore proprio: {}", rig::mode_label(mode));
                        assembler = slots::SlotAssembler::new(mode);
                        last_slot = None;
                        last_texts.clear();
                        if let Some(d) = driver.as_mut() {
                            for line in d.period_changed() {
                                tracing::info!("trasmissione: {line}");
                            }
                        }
                    }
                }
                // «Decodifica»: si rielabora l'ultimo periodo e si pubblica quello che non era ancora uscito
                if l.controls.redecode.swap(false, Ordering::Relaxed)
                    && let Some((start, slot_mode, samples)) = &last_slot
                    && *slot_mode == mode
                {
                    let decoded = decode_period(mode, samples, &period_options(&ft8_options, &l.controls, &l.de_call, *start), &ft4_period_options(&ft4_options, &l.controls, &l.de_call), &mut hashes, &mut ft8_history, &mut jt::q65::Averager::new());
                    let fresh: Vec<ft8::decode::Decoded> = decoded.into_iter().filter(|d| !last_texts.contains(&d.text)).collect();
                    tracing::info!("motore proprio: ridecodifica, {} messaggi nuovi nel periodo {}", fresh.len(), start);
                    last_texts.extend(fresh.iter().map(|d| d.text.clone()));
                    publish(l, driver.as_mut(), &message_events, &next_id, *start, &fresh);
                }
            }
            // `ALL.TXT`: frequenza cambiata, trasmissioni cominciate, QSO registrati
            if let (Some(a), Some(l)) = (all_txt.as_mut(), live.as_ref()) {
                write_traffic_log(a, driver.as_mut().map(|d| d as &mut TxDriver<dyn TxPort>), &mut logged_due, l, mode, now_unix_ms());
            }
            let chunk = match tap.recv_timeout(std::time::Duration::from_millis(100)) {
                Ok(chunk) => chunk,
                Err(mpsc::RecvTimeoutError::Timeout) => continue,
                Err(mpsc::RecvTimeoutError::Disconnected) => break,
            };
            let rs = match &mut resampler {
                Some((rate, rs)) if *rate == chunk.rate => rs,
                slot => {
                    // la scheda ha cambiato frequenza: si riparte da capo anche con i periodi
                    assembler = slots::SlotAssembler::new(mode);
                    &mut slot.insert((chunk.rate, resample::Resampler::new(chunk.rate, RATE as u32))).1
                }
            };
            let samples = rs.push(&chunk.samples);
            // MSK144: i ping si decodificano mentre il periodo scorre (come `mskrtd` di WSJT-X)
            if mode == Mode::Msk144 {
                if let Some(l) = live.as_ref()
                    && l.controls.is_monitoring()
                    && !l.controls.transmitting.load(Ordering::Relaxed)
                {
                    let his = driver.as_ref().and_then(|d| d.dx_call()).unwrap_or_default();
                    if let Some((slot_ms, fresh)) = msk_rt.push(&samples, chunk.received_unix_ms, &l.de_call, &his, &mut hashes) {
                        tracing::info!("motore proprio (MSK144): {} messaggi in tempo reale nel periodo {slot_ms}", fresh.len());
                        publish(l, driver.as_mut(), &message_events, &next_id, slot_ms, &fresh);
                    }
                }
            } else {
                msk_rt.reset();
            }
            if let Some(l) = live.as_ref() {
                if l.controls.transmitting.load(Ordering::Relaxed) {
                    tx_periods.note(chunk.received_unix_ms, mode.period_ms());
                }
                // momento di avvio della decodifica FT8 (`m_ft8DecoderStart` di WSJT-X); gli altri modi restano come sono
                let early = if mode == Mode::Ft8 { l.controls.decoder_start().early_ms() } else { slots::EARLY_MS };
                if assembler.early_ms() != early {
                    assembler.set_early_ms(early);
                }
                // decodifica anticipata (`m_earlyDecode` di WSJT-X): solo in FT8
                // decodifica anticipata SPENTA (2026-11-08): i messaggi letti a 11,8 s arrivavano alla sequenza prima dell'apertura del periodo e si
                // perdevano (R-04 di KP3B ignorato, il rapporto ripetuto invece di RR73). Da riaccendere solo dopo averli fatti passare dalla sequenza
                let _ = slots::FT8_PREVIEW_MS;
                assembler.set_preview_ms(0);
            }
            // lo S-meter si stima solo in ricezione con l'ascolto acceso; in trasmissione (o con Monitora spento) si ferma
            if let Some(l) = live.as_ref() {
                if l.controls.is_monitoring() && !l.controls.transmitting.load(Ordering::Relaxed) {
                    let estimate = s_meter.push(&samples);
                    l.controls.set_s_estimate(estimate);
                } else {
                    s_meter.pause();
                    l.controls.set_s_estimate(None);
                }
            }
            for slot in assembler.push(&samples, chunk.received_unix_ms) {
                // copia anticipata del periodo FT8 (a 11,81 s): una decodifica veloce (una passata, al piu' 1,5 s, cosi' quella intera non
                // ritarda) mostra subito le stazioni forti; la decodifica intera a fine periodo pubblica solo i messaggi nuovi
                if slot.partial {
                    if mode == Mode::Ft8
                        && let Some(l) = live.as_ref()
                        && l.controls.is_monitoring()
                        && !l.controls.transmitting.load(Ordering::Relaxed)
                    {
                        let mut quick = period_options(&ft8_options, &l.controls, &l.de_call, slot.start_unix_ms);
                        quick.passes = 1;
                        quick.a7 = false;
                        quick.a8 = None;
                        quick.deadline = Some(std::time::Instant::now() + std::time::Duration::from_millis(1500));
                        let found = ft8::decode::decode_slot(&slot.samples, &quick, &mut hashes);
                        let shown = preview_shown.entry(slot.start_unix_ms).or_default();
                        let fresh: Vec<ft8::decode::Decoded> = found.into_iter().filter(|d| !shown.contains(&d.text)).collect();
                        shown.extend(fresh.iter().map(|d| d.text.clone()));
                        if !fresh.is_empty() {
                            tracing::debug!("decodifica anticipata FT8: {} messaggi a 11,8 s", fresh.len());
                            publish(l, driver.as_mut(), &message_events, &next_id, slot.start_unix_ms, &fresh);
                        }
                    }
                    // i periodi vecchi non servono piu'
                    let keep_from = slot.start_unix_ms.saturating_sub(60_000);
                    preview_shown.retain(|start, _| *start >= keep_from);
                    continue;
                }
                // avvio anticipato (AutoSeq 3): il periodo di trasmissione comincia ora, prima della decodifica di quello appena finito; parte il messaggio
                // che la sequenza ha adesso e, se la decodifica lo cambia, ripartira' con il nuovo (ritrasmissione)
                if let (Some(d), Some(l)) = (driver.as_mut(), live.as_ref()) {
                    let dial = l.dial_hz.load(Ordering::Relaxed);
                    let band = (l.band_of)(dial).unwrap_or_default();
                    let (qsos, log) = d.begin_slot(slot.start_unix_ms, now_unix_ms() as f64 / 1000.0, dial, &band);
                    for line in log {
                        tracing::info!("trasmissione: {line}");
                    }
                    for qso in qsos {
                        note_logged(&mut logged_due, &qso, now_unix_ms());
                        let _ = message_events.send(Event::QsoCompleted(qso));
                    }
                }
                // i comandi arrivati mentre si decodificava (STOP TX, Esc, Abilita Tx) valgono PRIMA di decidere cosa trasmettere dopo questo periodo
                while let Ok(command) = command_rx.try_recv() {
                    if let Some(d) = driver.as_mut() {
                        for line in d.handle(command, now_unix_ms() as f64 / 1000.0) {
                            tracing::info!("trasmissione: {line}");
                        }
                    }
                }
                // con Monitora spento il periodo passa senza essere decodificato; cosi' anche il periodo in cui si e' trasmesso, se «Full duplex» e' spento
                let had_tx = tx_periods.had_tx(slot.start_unix_ms);
                let full_duplex = live.as_ref().is_some_and(|l| l.controls.full_duplex.load(Ordering::Relaxed));
                if live.as_ref().is_some_and(|l| !l.controls.is_monitoring() || (had_tx && !full_duplex)) {
                    if let (Some(d), Some(l)) = (driver.as_mut(), live.as_ref()) {
                        let dial = l.dial_hz.load(Ordering::Relaxed);
                        let band = (l.band_of)(dial).unwrap_or_default();
                        let (qsos, log) = d.after_slot(slot.start_unix_ms, &[], now_unix_ms() as f64 / 1000.0, dial, &band);
                        for line in log {
                            tracing::info!("trasmissione: {line}");
                        }
                        for qso in qsos {
                            note_logged(&mut logged_due, &qso, now_unix_ms());
                            let _ = message_events.send(Event::QsoCompleted(qso));
                        }
                    }
                    continue;
                }
                // i WAV portano il nome al secondo: i periodi FT4 da 7,5 s si sovrascriverebbero a vicenda, quindi si salva solo FT8
                if record
                    && mode == Mode::Ft8
                    && let Err(e) = wav::save_slot(&data_dir.join("recordings"), slot.start_unix_ms, &slot.samples)
                {
                    tracing::warn!("registrazione del periodo non salvata: {e}");
                }
                let started = Instant::now();
                // «Modo SWL», «Filtro» e Hound cambiano come si cerca (solo FT8)
                let mut options = live.as_ref().map_or_else(|| ft8_options.clone(), |l| period_options(&ft8_options, &l.controls, &l.de_call, slot.start_unix_ms));
                // un PC lento non deve restare indietro: dopo 11 s non si comincia un'altra passata (il periodo FT8 dura 15 s)
                options.deadline = Some(started + std::time::Duration::from_secs(11));
                // il log si rilegge ogni dieci minuti (i QSO nuovi); poi l'elenco per il filtro (None finche' i nominativi sono pochi)
                if let Some(log_calls) = live.as_ref().and_then(|l| l.log_calls.as_ref())
                    && known_seeded.is_none_or(|t| t.elapsed() >= std::time::Duration::from_secs(600))
                {
                    known.add_all(log_calls());
                    known_seeded = Some(Instant::now());
                }
                let call_book = known.book();
                options.call_book.clone_from(&call_book);
                options.grid_check = live.as_ref().and_then(|l| l.grid_check.clone());
                // senza un segnale vero (nessuna radio, microfono del PC) le ricerche «a priori» trovano messaggi nel rumore:
                // con l'audio debole si decodifica solo cio' che c'e' davvero, con l'audio muto il periodo non si decodifica
                let peak = slot.samples.iter().fold(0.0f32, |m, &x| m.max(x.abs()));
                let silent = peak < 0.0005;
                if peak < 0.02 {
                    options.a7 = false;
                    options.a8 = None;
                    options.ap = None;
                    options.ap_cq = false;
                }
                // FT4: analisi a priori sul proprio nominativo e sul QSO in corso (stesso interruttore «a7/a8» di FT8, acceso di base)
                let mut ft4_slot = live.as_ref().map_or_else(|| ft4_options.clone(), |l| ft4_period_options(&ft4_options, &l.controls, &l.de_call));
                ft4_slot.call_book = call_book;
                ft4_slot.grid_check = live.as_ref().and_then(|l| l.grid_check.clone());
                if peak < 0.02 {
                    ft4_slot.ap = None;
                    ft4_slot.ap_cq = false;
                }
                // FT8: i messaggi di ogni passata vanno subito nell'elenco, senza aspettare le passate seguenti; alla fine si pubblica solo il resto
                // MSK144: quelli gia' mostrati in tempo reale non si ripubblicano
                let mut early: Vec<String> = if mode == Mode::Msk144 { msk_rt.shown(slot.start_unix_ms) } else { preview_shown.remove(&slot.start_unix_ms).unwrap_or_default() };
                let mut decoded = if silent {
                    Vec::new()
                } else if mode == Mode::Ft8 && let Some(l) = live.as_ref() {
                    let mut on_pass = |batch: &[ft8::decode::DecodedInfo]| {
                        let fresh: Vec<ft8::decode::Decoded> = batch.iter().map(|i| i.decoded.clone()).filter(|d| !early.contains(&d.text)).collect();
                        early.extend(fresh.iter().map(|d| d.text.clone()));
                        publish(l, driver.as_mut(), &message_events, &next_id, slot.start_unix_ms, &fresh);
                    };
                    ft8::decode::decode_slot_streaming(&slot.samples, &options, &mut hashes, Some(&mut ft8_history), &mut on_pass).into_iter().map(|d| d.decoded).collect()
                } else {
                    decode_period(mode, &slot.samples, &options, &ft4_slot, &mut hashes, &mut ft8_history, &mut q65_avg)
                };
                // «JT65+JT9»: nello stesso minuto si decodifica anche l'altro modo JT; i suoi messaggi si distinguono in `ALL.TXT` (`@` e `#`)
                if matches!(mode, Mode::Ft8 | Mode::Ft4) {
                    known.learn(&decoded);
                    known.save_if_due();
                }
                let other_mode = match mode {
                    Mode::Jt65 => Some(Mode::Jt9),
                    Mode::Jt9 => Some(Mode::Jt65),
                    _ => None,
                };
                let mut other_texts: Vec<String> = Vec::new();
                if let Some(other) = other_mode.filter(|_| live.as_ref().is_some_and(|l| l.controls.switch_on("jt_both"))) {
                    for d in decode_period(other, &slot.samples, &options, &ft4_slot, &mut hashes, &mut ft8_history, &mut q65_avg) {
                        if !decoded.iter().any(|x| x.text == d.text) {
                            other_texts.push(d.text.clone());
                            decoded.push(d);
                        }
                    }
                }
                // Hound in FT8: si cerca anche una Fox UltraFOX/SuperFox (un solo segnale largo da 750 Hz); i suoi messaggi arrivano come quelli FT8
                if mode == Mode::Ft8 && live.as_ref().is_some_and(|l| l.controls.switch_on("hound"))
                    && let Some(fox) = ultrafox_rx::decode(&slot.samples, ultrafox::F0_HZ as f32, 200.0)
                {
                    tracing::info!("UltraFOX: {} messaggi da {} ({} dB)", fox.messages.len(), fox.fox, fox.snr);
                    if let Some(l) = live.as_ref() {
                        l.controls.note_ultrafox(now_unix_ms());
                    }
                    // in coda la riga `$VERIFY$ CALL 123456`, come `sfox_unpack.f90`: `publish` la nasconde o la fa verificare
                    for text in fox.messages.iter().chain(fox.free_text.iter()).cloned().chain(std::iter::once(fox.verify_line())) {
                        if !decoded.iter().any(|x| x.text == text) {
                            decoded.push(as_decoded(fox.freq_hz, fox.dt, fox.snr, text));
                        }
                    }
                }
                let millis = started.elapsed().as_millis() as u64;
                tracing::info!("motore proprio ({}): {} messaggi nel periodo {} in {} ms", mode.name(), decoded.len(), slot.start_unix_ms, millis);
                if let Some(r) = slow_watch.record(millis, mode.period_ms()) {
                    let swl = live.as_ref().is_some_and(|l| l.controls.switch_on("swl"));
                    let report = matches!(r.times, 1 | 10 | 100 | 1000);
                    let text = format!(
                        "decodifica lenta: {} periodi su {} oltre il {:.0}% del tempo ({} in {} ms su {} ms; {} processori, {} thread di decodifica, SWL {}, a8 {})",
                        r.slow, r.of, slow::SLOW_FRACTION * 100.0, mode.name(), millis, mode.period_ms(), slow::cores(), options.threads, if swl { "acceso" } else { "spento" }, if options.a8.is_some() { "acceso" } else { "spento" }
                    );
                    if report {
                        tracing::warn!(target: "engine_native::lento", "{text}");
                    } else {
                        tracing::info!(target: "engine_native::lento", "{text}");
                    }
                    aurora_core::notice::notice("slow_pc", format!("{:.1}", millis as f64 / 1000.0));
                }
                // file audio della scheda Audio (come JTDX): il periodo si tiene se il modo lo vuole, dopo aver saputo quanti messaggi ha dato
                if let Some(save) = &save {
                    let (save_mode, dir) = save.get();
                    if save_mode.keeps(decoded.len()) {
                        save_period(&dir, mode, &slot, live.as_ref());
                    }
                }
                if let (Some(a), Some(l)) = (all_txt.as_mut(), live.as_ref()) {
                    // le caselle «decodificati» e «e di debug» (impostazioni): senza, solo i messaggi che nominano la propria stazione
                    let scope = l.controls.all_txt_scope();
                    let rows_of = |from_other: bool| -> Vec<(i32, f32, i64, &str)> {
                        decoded
                            .iter()
                            .filter(|d| other_texts.contains(&d.text) == from_other && scope.keeps(&d.text, &l.de_call))
                            .map(|d| (d.snr_db, d.dt_s, d.freq_hz.round() as i64, d.text.as_str()))
                            .collect()
                    };
                    // la frequenza di meta' periodo: un cambio di banda arrivato mentre si decodificava non sposta i messaggi sulla banda nuova
                    let period_dial = a.dial_at(slot.start_unix_ms + mode.period_ms() / 2);
                    let period_dial = if period_dial == 0 { l.dial_hz.load(Ordering::Relaxed) } else { period_dial };
                    a.write_decodes(slot.start_unix_ms, period_dial, mode.name(), &rows_of(false));
                    if let Some(other) = other_mode.filter(|_| !other_texts.is_empty()) {
                        a.write_decodes(slot.start_unix_ms, period_dial, other.name(), &rows_of(true));
                    }
                    if scope == AllTxtScope::Debug {
                        a.write_debug(now_unix_ms(), "Decoding finished");
                    }
                }
                if let Ok(mut file) = std::fs::OpenOptions::new().create(true).append(true).open(&log_path) {
                    for d in &decoded {
                        let _ = file.write_all(format_line(slot.start_unix_ms, d).as_bytes());
                    }
                }
                if let Some(live) = &live {
                    let rest: Vec<ft8::decode::Decoded> = decoded.iter().filter(|d| !early.contains(&d.text)).cloned().collect();
                    publish(live, driver.as_mut(), &message_events, &next_id, slot.start_unix_ms, &rest);
                }
                // i comandi arrivati mentre si decodificava (STOP TX, Esc, Abilita Tx) valgono PRIMA di decidere cosa trasmettere dopo questo periodo
                while let Ok(command) = command_rx.try_recv() {
                    if let Some(d) = driver.as_mut() {
                        for line in d.handle(command, now_unix_ms() as f64 / 1000.0) {
                            tracing::info!("trasmissione: {line}");
                        }
                    }
                }
                if let (Some(d), Some(l)) = (driver.as_mut(), live.as_ref()) {
                    let dial = l.dial_hz.load(Ordering::Relaxed);
                    let band = (l.band_of)(dial).unwrap_or_default();
                    // full duplex: i decode del periodo in cui si trasmetteva si mostrano ma non fanno reagire la sequenza (`fullDuplexBlocked` di WSJT-X)
                    let list: Vec<(String, i32, u32)> =
                        if had_tx { Vec::new() } else { decoded.iter().map(|x| (x.text.clone(), x.snr_db, x.freq_hz.round().max(0.0) as u32)).collect() };
                    let (qsos, log) = d.after_slot(slot.start_unix_ms, &list, now_unix_ms() as f64 / 1000.0, dial, &band);
                    for line in log {
                        tracing::info!("trasmissione: {line}");
                    }
                    for qso in qsos {
                        note_logged(&mut logged_due, &qso, now_unix_ms());
                        let _ = message_events.send(Event::QsoCompleted(qso));
                    }
                }
                // il confronto con JTDX e' fatto sui periodi FT8
                if mode == Mode::Ft8 {
                    parity.add_native(slot.start_unix_ms, decoded.iter().map(|d| (d.text.clone(), d.freq_hz.round() as i64)).collect());
                }
                if let Ok(mut s) = stats.lock() {
                    *s = Stats { last_slot_unix_ms: slot.start_unix_ms, last_count: decoded.len(), last_millis: millis, slots_decoded: s.slots_decoded + 1 };
                }
                last_texts = decoded.iter().map(|d| d.text.clone()).collect();
                last_slot = Some((slot.start_unix_ms, mode, slot.samples));
            }
            // senza JTDX acceso il confronto non dice niente: niente riga nel registro e niente file (prima una riga ogni periodo)
            for summary in parity.finalize(now_unix_ms()).into_iter().filter(|s| s.jtdx > 0) {
                tracing::info!(
                    "confronto con JTDX, periodo {}: JTDX {}, proprio {}, in comune {}",
                    summary.slot_unix_ms,
                    summary.jtdx,
                    summary.native,
                    summary.both
                );
                if let Ok(mut file) = std::fs::OpenOptions::new().create(true).append(true).open(&comparison_path) {
                    let _ = writeln!(
                        file,
                        "{} jtdx={} proprio={} comuni={} solo_jtdx={:?} solo_proprio={:?}",
                        summary.slot_unix_ms, summary.jtdx, summary.native, summary.both, summary.only_jtdx, summary.only_native
                    );
                }
            }
        }
    });
    engine
}

/// La priorita' di una stazione come la calcola `displayDecodedText` di JTDX (`REA 0x4eb170`) dalle evidenziazioni di «nuovo»: nuova zona CQ 30, su banda 28,
/// zona ITU 26 e 24, DXCC 22 e su banda 20, locatore 15 e 13, prefisso 11 e 9, nominativo 7 e 5 (con LoTW 31, 29, 27, 25, 23, 21, 16, 14, 12, 10, 8, 6: Aurora non
/// conosce LoTW). I desiderati (2-4 e 17-19) li aggiunge chi ha la lista. Nessuna etichetta: 0.
fn priority_of(tag: Option<DecodeTag>) -> u8 {
    match tag {
        Some(DecodeTag::NewZone) => 30,
        Some(DecodeTag::NewZoneBand) => 28,
        Some(DecodeTag::NewItu) => 26,
        Some(DecodeTag::NewItuBand) => 24,
        Some(DecodeTag::NewDxcc) => 22,
        Some(DecodeTag::NewDxccBand) => 20,
        Some(DecodeTag::NewGrid) => 15,
        Some(DecodeTag::NewGridBand) => 13,
        Some(DecodeTag::NewPx) => 11,
        Some(DecodeTag::NewPxBand) => 9,
        Some(DecodeTag::NewCall) => 7,
        Some(DecodeTag::NewCallBand) => 5,
        _ => 0,
    }
}

/// Chi dice alla sequenza continente e novita' di una stazione: l'annotatore del registro, interrogato con un CQ della stessa stazione sulla banda attuale.
fn caller_info_provider(live: &LiveConfig) -> CallerInfoFn {
    let annotate = Arc::clone(&live.annotate);
    let band_of = Arc::clone(&live.band_of);
    let dial = Arc::clone(&live.dial_hz);
    Box::new(move |call| {
        let band = band_of(dial.load(Ordering::Relaxed));
        let (_country, continent, tag) = annotate(&format!("CQ {call}"), band.as_deref());
        CallerInfo {
            continent: continent.unwrap_or_default(),
            prefix: String::new(),
            priority: priority_of(tag),
            dxcc_new: tag == Some(DecodeTag::NewDxcc),
            dxcc_new_band: tag == Some(DecodeTag::NewDxccBand),
            worked_band: tag == Some(DecodeTag::WorkedCall),
            worked_elsewhere: tag == Some(DecodeTag::NewCallBand),
        }
    })
}

/// Un QSO registrato: JTDX scrive «QSO logged» quando comincia a trasmettere RR73 o 73 (pochi millisecondi dopo la riga «Transmitting»). Qui la
/// sequenza lo chiude un periodo prima: la riga aspetta l'inizio di quel periodo di trasmissione (fine QSO meno la coda di 14 s in FT8, 7 in FT4).
fn note_logged(due: &mut Vec<(u64, String)>, qso: &aurora_core::QsoRecord, now_ms: u64) {
    let tail = if qso.mode == "FT4" { 7_000 } else { 14_000 };
    let when = if qso.time_off_unix_ms > 0 { qso.time_off_unix_ms.saturating_sub(tail) } else { now_ms };
    due.push((when, qso.call.clone()));
}

/// Scrive in `ALL.TXT` quello che e' successo dall'ultima volta: la frequenza della radio cambiata (a trasmissione ferma), le trasmissioni il
/// cui periodo e' cominciato, i QSO registrati (dopo la riga di trasmissione che li chiude).
fn write_traffic_log(all_txt: &mut AllTxt, driver: Option<&mut TxDriver<dyn TxPort>>, logged_due: &mut Vec<(u64, String)>, live: &LiveConfig, mode: Mode, now_ms: u64) {
    if !all_txt.is_enabled() {
        return;
    }
    let dial = live.dial_hz.load(Ordering::Relaxed);
    all_txt.note_dial(now_ms, dial);
    if !live.controls.transmitting.load(Ordering::Relaxed) {
        all_txt.write_frequency_change(now_ms, dial, mode.name());
    }
    if let Some(driver) = driver {
        for note in driver.due_notes(now_ms) {
            if note.retransmit {
                all_txt.write_retransmit(now_ms, 0.0, dial, note.tx_df, note.mode.name(), &note.text);
            } else {
                all_txt.write_tx(now_ms, 0.0, dial, note.tx_df, note.mode.name(), &note.text);
            }
        }
    }
    let (due, later): (Vec<_>, Vec<_>) = std::mem::take(logged_due).into_iter().partition(|(when, _)| *when <= now_ms);
    *logged_due = later;
    for (_, call) in due {
        all_txt.write_qso_logged(now_ms, 0.0, &call);
    }
}

/// Le opzioni di decodifica FT8 di questo periodo: «Modo SWL» (finestra e soglie di JTDX), «Filtro» (solo +-60 Hz attorno alla frequenza di
/// ricezione, +-290 Hz in Hound: `nfilter` di `ft8_decode`, letto con REA in `jtdxjt9.exe`), e la parita' del periodo (sottopassate 3-11:
/// FT8 alterna periodi che cominciano a un multiplo pari o dispari di 15 s dall'inizio dei tempi, `docs/analisi-jtdx-reverse-rea.md` 7.2.3).
/// «a7/a8» (`ap_list`, acceso di base come l'analisi a priori di WSJT-X) accende le decodifiche a liste di WSJT-X: "a7" sui seguiti
/// dei QSO del periodo precedente della stessa parita', "a8" sul DX alla frequenza di ricezione quando DX Call e DX Grid sono
/// noti (non in Hound, come `ncontest` 6/7 di WSJT-X). «Multithread» (`mtd`, acceso di base) usa tutti i processori: i decode
/// sono gli stessi del singolo thread.
fn period_options(base: &Options, controls: &rig::NativeControls, de_call: &str, slot_start_ms: u64) -> Options {
    let threads = if controls.switch_on("mtd") { std::thread::available_parallelism().map_or(1, std::num::NonZero::get) } else { 1 };
    let list = controls.switch_on("ap_list");
    let view = tx::view_of(controls);
    let a8 = (list && !controls.switch_on("hound")).then(|| ft8::a8::A8Query {
        mycall: de_call.to_string(),
        dxcall: view.dx.clone().unwrap_or_default(),
        dxgrid: view.dx_grid.clone().unwrap_or_default(),
        rx_freq_hz: controls.rx_df.load(Ordering::Relaxed) as f32,
        threads,
    });
    let mut options = Options {
        swl: controls.switch_on("swl"),
        agc: controls.switch_on("agc"),
        even_period: (slot_start_ms / 15_000).is_multiple_of(2),
        a7: list,
        a8: a8.filter(ft8::a8::A8Query::usable),
        period_key: Some(slot_start_ms / 15_000),
        threads,
        // nominativo, DX e punto del QSO per i tipi a priori (in Hound si cerca solo la Fox: niente QSO normale)
        ap: if controls.switch_on("hound") { None } else { ap_options(controls, de_call) },
        ..base.clone()
    };
    if controls.switch_on("filter") {
        let half = if controls.switch_on("hound") { 290.0 } else { 60.0 };
        let rx = controls.rx_df.load(Ordering::Relaxed) as f32;
        options.f_low_hz = (rx - half).max(base.f_low_hz);
        options.f_high_hz = (rx + half).min(base.f_high_hz);
    }
    options
}

/// Le opzioni di decodifica FT4 di questo periodo: l'analisi a priori (interruttore «a7/a8», `ap_list`, acceso di base) sul proprio nominativo e sul
/// QSO in corso. Il punto del QSO viene dal messaggio selezionato (1-5; CQ o nessuno = 0); il DX e il suo locatore dalla pagina; la frequenza
/// di ricezione e di trasmissione dalle barre. I tipi di QSO valgono solo mentre si trasmette con un DX scelto (`idle` spento).
fn ft4_period_options(base: &ft4::decode::Options, controls: &rig::NativeControls, de_call: &str) -> ft4::decode::Options {
    let mut options = base.clone();
    if let Some(ap) = ap_options(controls, de_call) {
        options.ap = Some(ap);
        options.ap_cq = true;
    }
    options
}

/// L'analisi a priori di questo periodo (FT8 e FT4): il proprio nominativo, il DX e il suo locatore dalla pagina, il punto del QSO dal messaggio
/// selezionato (1-5; CQ o nessuno = 0), le frequenze dalle barre. `None` con l'interruttore «a7/a8» (`ap_list`) spento.
fn ap_options(controls: &rig::NativeControls, de_call: &str) -> Option<ft8::ap::ApOptions> {
    if !controls.switch_on("ap_list") {
        return None;
    }
    let view = tx::view_of(controls);
    let dx = view.dx.clone().unwrap_or_default();
    let mut ap = ft8::ap::ApOptions::new(de_call);
    ap.qso_progress = if dx.is_empty() || !(1..=5).contains(&view.selected) { 0 } else { view.selected };
    ap.hisgrid = view.dx_grid.clone().unwrap_or_default();
    ap.idle = !(view.enabled && !dx.is_empty());
    // i tipi con il proprio nominativo solo con Abilita Tx: in ascolto a vuoto un messaggio "per me" trovato nel rumore e' quasi sempre un falso
    ap.my_call_active = view.enabled;
    ap.dx_search = !dx.is_empty();
    ap.rx_freq_hz = Some(controls.rx_df.load(Ordering::Relaxed) as f32);
    ap.tx_freq_hz = Some(controls.tx_df.load(Ordering::Relaxed) as f32);
    ap.hiscall = dx;
    Some(ap)
}

fn as_decoded(freq_hz: f32, dt_s: f32, snr_db: i32, text: String) -> ft8::decode::Decoded {
    ft8::decode::Decoded { freq_hz, dt_s, snr_db, text, hint: false }
}

/// Decodifica un periodo nel modo dato (audio a 12 kHz: 15 s in FT8, 7,5 s in FT4, un minuto in JT65 e JT9).
/// `ft8_history` tiene i segnali CQ, le chiamate dirette a noi e il segnale del canale QSO tra un periodo FT8 e l'altro
/// (sottopassate 3-11 di `ft8b`); gli altri modi non la usano.
#[allow(clippy::too_many_arguments)]
fn decode_period(
    mode: Mode,
    samples: &[f32],
    ft8_options: &Options,
    ft4_options: &ft4::decode::Options,
    hashes: &mut CallHashes,
    ft8_history: &mut ft8::decode::PeriodHistory,
    q65_avg: &mut jt::q65::Averager,
) -> Vec<ft8::decode::Decoded> {
    match mode {
        Mode::Ft8 => ft8::decode::decode_slot_with_history(samples, ft8_options, hashes, ft8_history),
        Mode::Ft4 => {
            // AGCc anche in FT4 (stesso interruttore di FT8)
            let mut o = ft4_options.clone();
            o.agc = ft8_options.agc;
            ft4::decode::decode_slot(samples, &o, hashes)
        }
        Mode::Jt65 => jt::rx::decode(samples, &jt::rx::Options::default()).into_iter().map(|d| as_decoded(d.freq_hz, d.dt_s, d.snr_db, d.text)).collect(),
        Mode::Jt9 => jt::jt9rx::decode(samples, &jt::jt9rx::Options::default()).into_iter().map(|d| as_decoded(d.freq_hz, d.dt_s, d.snr_db, d.text)).collect(),
        Mode::Jt4 => jt::jt4::decode(samples, &jt::jt4::Options { submode: rig::jt4_submode(), ..jt::jt4::Options::default() }).into_iter().map(|d| as_decoded(d.freq_hz, d.dt_s, d.snr_db, d.text)).collect(),
        Mode::Msk144 => jt::msk144::decode(
            samples,
            &jt::msk144::Options {
                my_call: ft8_options.ap.as_ref().map(|a| a.mycall.clone()).unwrap_or_default(),
                his_call: ft8_options.ap.as_ref().map(|a| a.hiscall.clone()).unwrap_or_default(),
                ..jt::msk144::Options::default()
            },
            hashes,
        ).into_iter().map(|d| as_decoded(d.freq_hz, d.t_s, d.snr_db, d.text)).collect(),
        // Q65: periodo e sottomodo scelti, ipotesi AP con il mio nominativo e il DX, metrica dell'evanescenza (lorentziana come WSJT-X, o gaussiana)
        // e media dei periodi della stessa parita'; i messaggi da AP o dalla media hanno il segno di quelli a priori di FT8
        Mode::Q65 => {
            let setup = rig::q65_setup();
            let ap = ft8_options.ap.as_ref();
            // la frequenza di ricezione (`nfqso` di WSJT-X): il picco migliore entro 50 Hz si prova sempre, e con il DX noto vi si
            // prova la lista dei messaggi del QSO
            let rx_hz = ap.and_then(|a| a.rx_freq_hz).or_else(|| ft8_options.a8.as_ref().map(|q| q.rx_freq_hz));
            // contest (`ncontest = 1`): i chiamanti di oggi fanno la lista dei messaggi; l'ora e' quella del periodo
            let contest = rig::q65_contest();
            let now = ft8_options.period_key.map_or(0, |k| (k * 15) as i64);
            let callers = if contest { rig::with_q65_callers(|c| { c.expire(now); c.list().to_vec() }) } else { Vec::new() };
            let o = jt::q65::Options {
                setup,
                my_call: ap.map(|a| a.mycall.clone()).unwrap_or_default(),
                dx_call: ap.map(|a| a.hiscall.clone()).unwrap_or_default(),
                dx_grid: ap.map(|a| a.hisgrid.clone()).unwrap_or_default(),
                rx_hz,
                threads: ft8_options.threads,
                fading: Some(rig::q65_fading()),
                eme_delay: rig::q65_eme(),
                max_drift: rig::q65_max_drift(),
                contest,
                callers,
                ..jt::q65::Options::default()
            };
            // il numero del periodo dall'inizio del periodo (`period_key` conta i quarti di minuto)
            let index = ft8_options.period_key.map_or(0, |k| k * 15 / u64::from(setup.period_s));
            let decoded = jt::q65::decode_averaged(samples, &o, hashes, q65_avg, index);
            // `record_caller` di `q65_decode.f90`: ogni messaggio del contest aggiorna la lista dei chiamanti
            if contest {
                rig::with_q65_callers(|c| decoded.iter().for_each(|d| c.record(d.freq_hz.round() as i32, &d.text, now)));
            }
            decoded
                .into_iter()
                .map(|d| ft8::decode::Decoded { hint: d.ap > 0 || d.averaged > 1, ..as_decoded(d.freq_hz, d.dt_s, d.snr_db, d.text) })
                .collect()
        }
    }
}

/// Pubblica i messaggi di un periodo sul bus (identici in FT8 e FT4) e li ricorda per il doppio clic.
fn publish(live: &LiveConfig, mut driver: Option<&mut TxDriver<dyn TxPort>>, events: &broadcast::Sender<Event>, next_id: &AtomicU64, slot_start_ms: u64, decoded: &[ft8::decode::Decoded]) {
    let band = (live.band_of)(live.dial_hz.load(Ordering::Relaxed));
    let time_ms = (slot_start_ms % DAY_MS) as u32;
    let hound = live.controls.switch_on("hound");
    for d in decoded {
        // Hound: il codice OTP del Fox (`CALL.123456` del Fox classico alla sua frequenza, `$VERIFY$ CALL 123456` del SuperFox a 750 Hz;
        // `processSFoxVerification` di WSJT-X) si nasconde e/o si fa verificare
        let otp = if hound {
            features::parse_fox_otp(&d.text)
                .map(|(call, code)| (call, code, d.freq_hz.round().max(0.0) as u32))
                .or_else(|| features::parse_superfox_verify(&d.text).map(|(call, code)| (call, code, features::SUPERFOX_VERIFY_HZ)))
        } else {
            None
        };
        if let Some((call, code, freq_hz)) = otp {
            let (hide, check) = features::otp_action(&code, &live.controls.otp_settings());
            if check {
                live.controls.request_otp_check(features::FoxOtp { call, code, freq_hz, slot_start_ms });
            }
            if hide {
                continue;
            }
        }
        let (country, continent, tag) = (live.annotate)(&d.text, band.as_deref());
        let id = next_id.fetch_add(1, Ordering::Relaxed);
        if let Some(driver) = driver.as_deref_mut() {
            driver.remember(id, &d.text, d.snr_db, d.freq_hz.round().max(0.0) as u32, slot_start_ms);
        }
        let _ = events.send(Event::Decode(DecodedMessage {
            id,
            time_ms,
            snr: d.snr_db,
            delta_time: d.dt_s,
            freq_hz: d.freq_hz.round().max(0.0) as u32,
            text: d.text.clone(),
            tag,
            country,
            continent,
            hint: d.hint,
        }));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ft8::{modem, SAMPLE_RATE};

    /// «Filtro»: solo +-60 Hz attorno alla frequenza di ricezione, +-290 Hz in Hound; spento, tutta la banda; «Modo SWL» passa al decodificatore.
    #[test]
    fn the_filter_button_narrows_the_search_around_the_receive_frequency() {
        let controls = NativeControls::default();
        let base = Options::default();
        controls.rx_df.store(1500, Ordering::Relaxed);
        let o = period_options(&base, &controls, "", 0);
        assert_eq!((o.f_low_hz, o.f_high_hz, o.swl), (base.f_low_hz, base.f_high_hz, false), "filtro spento: tutta la banda");
        controls.set_switch("filter", true);
        let o = period_options(&base, &controls, "", 0);
        assert_eq!((o.f_low_hz, o.f_high_hz), (1440.0, 1560.0));
        controls.set_switch("hound", true);
        let o = period_options(&base, &controls, "", 0);
        assert_eq!((o.f_low_hz, o.f_high_hz), (1210.0, 1790.0));
        // vicino al bordo la banda resta dentro quella del decodificatore
        controls.rx_df.store(300, Ordering::Relaxed);
        assert_eq!(period_options(&base, &controls, "", 0).f_low_hz, base.f_low_hz);
        controls.set_switch("swl", true);
        assert!(period_options(&base, &controls, "", 0).swl);
    }

    /// La parita' del periodo segue il tempo assoluto: alterna ogni 15 s, indipendente dalle altre opzioni.
    #[test]
    fn period_options_alternates_the_parity_every_15_seconds() {
        let controls = NativeControls::default();
        let base = Options::default();
        assert!(period_options(&base, &controls, "", 0).even_period);
        assert!(!period_options(&base, &controls, "", 15_000).even_period);
        assert!(period_options(&base, &controls, "", 30_000).even_period);
        assert!(!period_options(&base, &controls, "", 45_000).even_period);
    }

    /// «a7/a8» e «Multithread» sono accesi di base; "a8" parte solo con DX Call e DX Grid noti e non in Hound.
    #[test]
    fn period_options_list_decoding_and_threads() {
        let controls = NativeControls::default();
        let base = Options::default();
        let o = period_options(&base, &controls, "IU3ABC", 30_000);
        assert!(o.a7 && o.a8.is_none() && o.period_key == Some(2) && o.threads >= 1);
        if let Ok(mut v) = controls.tx_view.lock() {
            v.dx = Some("K1ABC".into());
            v.dx_grid = Some("FN42".into());
        }
        let o = period_options(&base, &controls, "IU3ABC", 0);
        let q = o.a8.expect("a8 con DX noto");
        assert_eq!((q.dxcall.as_str(), q.dxgrid.as_str(), q.rx_freq_hz), ("K1ABC", "FN42", 1500.0));
        controls.set_switch("hound", true);
        assert!(period_options(&base, &controls, "IU3ABC", 0).a8.is_none());
        controls.set_switch("hound", false);
        controls.set_switch("ap_list", false);
        controls.set_switch("mtd", false);
        let o = period_options(&base, &controls, "IU3ABC", 0);
        assert!(!o.a7 && o.a8.is_none() && o.threads == 1);
    }

    /// Il motore proprio come unico motore: i messaggi e lo stato arrivano sul bus, con paese ed etichetta.
    #[test]
    fn live_mode_publishes_decodes_and_status_on_the_bus() {
        let (tx, rx) = mpsc::channel();
        let (events, mut bus) = broadcast::channel(256);
        let dir = std::env::temp_dir().join(format!("aurora-live-{}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);
        let runtime = tokio::runtime::Builder::new_current_thread().enable_all().build().expect("runtime");
        let live = LiveConfig {
            de_call: "IU3ABC".into(),
            de_grid: "IO91".into(),
            own_continent: Some("EU".into()),
            dial_hz: Arc::new(AtomicU64::new(14_074_000)),
            controls: Arc::new(NativeControls::default()),
            transmitter: None,
            log_calls: None,
            grid_check: None,
            band_of: Arc::new(|hz| (hz == 14_074_000).then(|| "20m".to_string())),
            annotate: Arc::new(|text, band| {
                assert_eq!(band, Some("20m"));
                (Some("United States".into()), Some("NA".into()), text.starts_with("CQ").then_some(DecodeTag::Cq))
            }),
        };
        let engine = runtime.block_on(async { start(events.clone(), rx, dir.clone(), false, Some(live)) });

        let t0 = 1_000_005_000u64 - 15_000;
        let wave = modem::encode_message("CQ K1ABC FN42", 1500.0, &mut CallHashes::default()).expect("onda");
        let mut audio = vec![0.0f32; 3 * SAMPLE_RATE * 15];
        let mut seed = 9u64;
        for a in audio.iter_mut() {
            seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
            *a = ((seed >> 40) as f32 / (1u64 << 24) as f32 - 0.5) * 0.1;
        }
        let begin = (15_000 + 500 + 300) * SAMPLE_RATE / 1000;
        for (i, w) in wave.iter().enumerate() {
            audio[begin + i] += 0.02 * w;
        }
        for (n, chunk) in audio.chunks(480).enumerate() {
            let end_ms = t0 + ((n + 1) * 480 * 1000 / SAMPLE_RATE) as u64;
            tx.send(AudioChunk { rate: 12_000, samples: chunk.to_vec(), received_unix_ms: end_ms }).expect("invio");
        }
        drop(tx);
        for _ in 0..1200 {
            if engine.status()["slots_decoded"].as_u64().unwrap_or(0) >= 2 {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(50));
        }
        let mut decode = None;
        let mut status = None;
        runtime.block_on(async {
            tokio::time::sleep(std::time::Duration::from_millis(1200)).await;
            while let Ok(event) = bus.try_recv() {
                match event {
                    Event::Decode(m) => decode = Some(m),
                    Event::Status(s) => status = Some(s),
                    _ => {}
                }
            }
        });
        let m = decode.expect("un messaggio sul bus");
        assert_eq!(m.text, "CQ K1ABC FN42");
        assert!((m.freq_hz as i64 - 1500).abs() <= 1, "{}", m.freq_hz);
        assert_eq!((m.country.as_deref(), m.continent.as_deref(), m.tag), (Some("United States"), Some("NA"), Some(DecodeTag::Cq)));
        assert!(m.id >= 1_000_000_000, "gli id del motore proprio sono alti");
        let s = status.expect("lo stato sul bus");
        assert_eq!((s.dial_hz, s.mode.as_str(), s.de_call.as_str(), s.tr_period), (14_074_000, "FT8", "IU3ABC", Some(15)));
        // `ALL.TXT` come JTDX: un file al mese nella cartella dei dati, CR LF, riga di frequenza e riga di decodifica
        let all_txt = std::fs::read_to_string(dir.join("197001_ALL.TXT")).expect("ALL.TXT del mese");
        assert!(all_txt.ends_with("\r\n") && !all_txt.replace("\r\n", "").contains('\n'), "{all_txt:?}");
        let lines: Vec<&str> = all_txt.lines().collect();
        assert!(lines.iter().any(|l| l.ends_with(" 14.074 MHz  FT8") || l.contains(" MHz  FT8 Aurora v")), "{lines:?}");
        assert!(lines.iter().any(|l| l.starts_with("19700112_") && l.contains(" ~ CQ K1ABC FN42")), "{lines:?}");
        let _ = std::fs::remove_dir_all(dir);
    }

    /// JT65 come motore vivo: tre minuti di audio con un CQ nel secondo, il minuto intero esce decodificato sul bus con il suo `ALL.TXT` (`#`).
    #[test]
    fn live_mode_decodes_a_jt65_minute_and_a_jt9_minute() {
        for (mode, symbol, tr) in [(Mode::Jt65, " # ", 60u32), (Mode::Jt9, " @ ", 60u32)] {
            let (tx, rx) = mpsc::channel();
            let (events, mut bus) = broadcast::channel(256);
            let dir = std::env::temp_dir().join(format!("aurora-live-jt-{}-{}", mode.name(), std::process::id()));
            let _ = std::fs::remove_dir_all(&dir);
            let _ = std::fs::create_dir_all(&dir);
            let runtime = tokio::runtime::Builder::new_current_thread().enable_all().build().expect("runtime");
            let controls = Arc::new(NativeControls::default());
            controls.set_mode(mode);
            let live = LiveConfig {
                de_call: "IU3ABC".into(),
                de_grid: "IO91".into(),
                own_continent: Some("EU".into()),
                dial_hz: Arc::new(AtomicU64::new(14_076_000)),
                controls,
                transmitter: None,
            log_calls: None,
            grid_check: None,
                band_of: Arc::new(|hz| (hz == 14_076_000).then(|| "20m".to_string())),
                annotate: Arc::new(|_, _| (None, None, None)),
            };
            let engine = runtime.block_on(async { start(events.clone(), rx, dir.clone(), false, Some(live)) });
            let t0 = 1_000_020_000u64 - 60_000;
            let wave = match mode {
                Mode::Jt65 => jt::jt65::modulate(&jt::jt65::tones(&jt::rs::Codec::new(), "CQ K1ABC FN42"), 1500.0, 1.0, 12_000.0),
                _ => jt::jt9::modulate(&jt::jt9::tones("CQ K1ABC FN42"), 1500.0, 1.0),
            };
            let mut audio = vec![0.0f32; 3 * 60 * SAMPLE_RATE];
            let mut seed = 21u64;
            for a in audio.iter_mut() {
                seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
                *a = ((seed >> 40) as f32 / (1u64 << 24) as f32 - 0.5) * 0.1;
            }
            // il messaggio comincia a un secondo dall'inizio del secondo minuto, con 0,3 s di ritardo
            let begin = (60_000 + 1_000 + 300) * SAMPLE_RATE / 1000;
            for (i, w) in wave.iter().enumerate() {
                audio[begin + i] += 0.02 * w;
            }
            for (n, chunk) in audio.chunks(480).enumerate() {
                let end_ms = t0 + ((n + 1) * 480 * 1000 / SAMPLE_RATE) as u64;
                tx.send(AudioChunk { rate: 12_000, samples: chunk.to_vec(), received_unix_ms: end_ms }).expect("invio");
            }
            drop(tx);
            for _ in 0..2400 {
                if engine.status()["slots_decoded"].as_u64().unwrap_or(0) >= 2 {
                    break;
                }
                std::thread::sleep(std::time::Duration::from_millis(50));
            }
            let mut texts = Vec::new();
            let mut status = None;
            runtime.block_on(async {
                tokio::time::sleep(std::time::Duration::from_millis(1500)).await;
                while let Ok(event) = bus.try_recv() {
                    match event {
                        Event::Decode(m) => texts.push((m.text, m.freq_hz)),
                        Event::Status(s) => status = Some(s),
                        _ => {}
                    }
                }
            });
            assert_eq!(texts.len(), 1, "{}: {texts:?}", mode.name());
            assert_eq!(texts[0].0, "CQ K1ABC FN42");
            assert!((texts[0].1 as i64 - 1500).abs() <= 2, "{}: {}", mode.name(), texts[0].1);
            let s = status.expect("lo stato sul bus");
            assert_eq!((s.mode.as_str(), s.tr_period), (mode.name(), Some(tr)));
            let all_txt = std::fs::read_to_string(dir.join("197001_ALL.TXT")).expect("ALL.TXT del mese");
            assert!(all_txt.lines().any(|l| l.contains(symbol) && l.contains("CQ K1ABC FN42")), "{}: {all_txt:?}", mode.name());
            let _ = std::fs::remove_dir_all(dir);
        }
    }

    /// «JT65+JT9»: in JT65 un messaggio JT9 si decodifica solo con l'interruttore acceso, e in `ALL.TXT` ha il segno di JT9.
    #[test]
    fn jt_both_decodes_the_other_jt_mode_in_the_same_minute() {
        for both in [false, true] {
            let (tx, rx) = mpsc::channel();
            let (events, mut bus) = broadcast::channel(256);
            let dir = std::env::temp_dir().join(format!("aurora-live-jtboth-{both}-{}", std::process::id()));
            let _ = std::fs::remove_dir_all(&dir);
            let _ = std::fs::create_dir_all(&dir);
            let runtime = tokio::runtime::Builder::new_current_thread().enable_all().build().expect("runtime");
            let controls = Arc::new(NativeControls::default());
            controls.set_mode(Mode::Jt65);
            controls.set_switch("jt_both", both);
            let live = LiveConfig {
                de_call: "IU3ABC".into(),
                de_grid: "IO91".into(),
                own_continent: None,
                dial_hz: Arc::new(AtomicU64::new(14_076_000)),
                controls,
                transmitter: None,
            log_calls: None,
            grid_check: None,
                band_of: Arc::new(|hz| (hz == 14_076_000).then(|| "20m".to_string())),
                annotate: Arc::new(|_, _| (None, None, None)),
            };
            let engine = runtime.block_on(async { start(events.clone(), rx, dir.clone(), false, Some(live)) });
            let t0 = 1_000_020_000u64 - 60_000;
            let wave = jt::jt9::modulate(&jt::jt9::tones("CQ K1ABC FN42"), 1500.0, 1.0);
            let mut audio = vec![0.0f32; 3 * 60 * SAMPLE_RATE];
            let mut seed = 31u64;
            for a in audio.iter_mut() {
                seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
                *a = ((seed >> 40) as f32 / (1u64 << 24) as f32 - 0.5) * 0.1;
            }
            let begin = (60_000 + 1_000 + 300) * SAMPLE_RATE / 1000;
            for (i, w) in wave.iter().enumerate() {
                audio[begin + i] += 0.02 * w;
            }
            for (n, chunk) in audio.chunks(480).enumerate() {
                let end_ms = t0 + ((n + 1) * 480 * 1000 / SAMPLE_RATE) as u64;
                tx.send(AudioChunk { rate: 12_000, samples: chunk.to_vec(), received_unix_ms: end_ms }).expect("invio");
            }
            drop(tx);
            for _ in 0..2400 {
                if engine.status()["slots_decoded"].as_u64().unwrap_or(0) >= 2 {
                    break;
                }
                std::thread::sleep(std::time::Duration::from_millis(50));
            }
            let mut texts = Vec::new();
            runtime.block_on(async {
                tokio::time::sleep(std::time::Duration::from_millis(1500)).await;
                while let Ok(event) = bus.try_recv() {
                    if let Event::Decode(m) = event {
                        texts.push(m.text);
                    }
                }
            });
            let all_txt = std::fs::read_to_string(dir.join("197001_ALL.TXT")).unwrap_or_default();
            if both {
                assert_eq!(texts, vec!["CQ K1ABC FN42".to_string()], "con l'interruttore si decodifica anche JT9");
                assert!(all_txt.lines().any(|l| l.contains(" @ ") && l.contains("CQ K1ABC FN42")), "{all_txt:?}");
            } else {
                assert!(texts.is_empty(), "in JT65 un messaggio JT9 non si decodifica: {texts:?}");
            }
            let _ = std::fs::remove_dir_all(dir);
        }
    }

    /// «Decodificati»: il file del periodo con il messaggio resta, quelli di solo rumore no; il nome e' quello di JTDX.
    #[test]
    fn the_audio_save_mode_keeps_only_the_decoded_periods() {
        let (tx, rx) = mpsc::channel();
        let (events, _keep) = broadcast::channel(16);
        let dir = std::env::temp_dir().join(format!("aurora-savewav-{}", std::process::id()));
        let save_dir = dir.join("audio");
        let _ = std::fs::remove_dir_all(&dir);
        let _ = std::fs::create_dir_all(&dir);
        let runtime = tokio::runtime::Builder::new_current_thread().enable_all().build().expect("runtime");
        let save = Arc::new(SaveSettings::new(SaveMode::Decoded, save_dir.clone()));
        let engine = runtime.block_on(async { start_with_save(events, rx, dir.clone(), false, None, Some(Arc::clone(&save))) });
        let t0 = 1_000_005_000u64 - 15_000;
        let wave = modem::encode_message("CQ K1ABC FN42", 1500.0, &mut CallHashes::default()).expect("onda");
        let mut audio = vec![0.0f32; 3 * SAMPLE_RATE * 15];
        let mut seed = 11u64;
        for a in audio.iter_mut() {
            seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
            *a = ((seed >> 40) as f32 / (1u64 << 24) as f32 - 0.5) * 0.1;
        }
        let begin = (15_000 + 500 + 300) * SAMPLE_RATE / 1000;
        for (i, w) in wave.iter().enumerate() {
            audio[begin + i] += 0.02 * w;
        }
        for (n, chunk) in audio.chunks(480).enumerate() {
            let end_ms = t0 + ((n + 1) * 480 * 1000 / SAMPLE_RATE) as u64;
            tx.send(AudioChunk { rate: 12_000, samples: chunk.to_vec(), received_unix_ms: end_ms }).expect("invio");
        }
        drop(tx);
        for _ in 0..1200 {
            if engine.status()["slots_decoded"].as_u64().unwrap_or(0) >= 2 {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(50));
        }
        std::thread::sleep(std::time::Duration::from_millis(300));
        let names: Vec<String> = std::fs::read_dir(&save_dir).map(|d| d.filter_map(|e| e.ok()).map(|e| e.file_name().to_string_lossy().into_owned()).collect()).unwrap_or_default();
        let wanted = format!("{}.wav", jtdx_stem(1_000_005_000));
        assert!(names.contains(&wanted), "il periodo con il messaggio va salvato: {names:?}");
        assert_eq!(names.len(), 1, "i periodi di solo rumore no: {names:?}");
        let bytes = std::fs::read(save_dir.join(&wanted)).expect("file");
        assert_eq!(&bytes[..4], b"RIFF");
        assert!(String::from_utf8_lossy(&bytes).contains("Mode=FT8"), "il blocco INFO descrive il periodo");
        assert!(!dir.join("recordings").exists(), "il registratore delle misure e' un'altra cosa");
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn jtdx_time_maps_to_the_slot_of_today() {
        // 12:00:30 UTC del giorno 20000 dal 1970
        let now = 20_000 * DAY_MS + 12 * 3_600_000 + 30_000 + 4_000;
        let slot = slot_of_time_ms(12 * 3_600_000 + 30_000, now);
        assert_eq!(slot, 20_000 * DAY_MS + 12 * 3_600_000 + 30_000);
        // appena dopo mezzanotte, un messaggio delle 23:59:45 e' di ieri
        let after_midnight = 20_001 * DAY_MS + 10_000;
        assert_eq!(slot_of_time_ms(23 * 3_600_000 + 59 * 60_000 + 45_000, after_midnight), 20_001 * DAY_MS - 15_000);
    }

    /// Un periodo FT8 vero (messaggio sintetico dentro rumore) passato dalla scheda finta fino ai risultati.
    #[test]
    fn audio_in_decoded_messages_out() {
        let (tx, rx) = mpsc::channel();
        let (events, _keep) = broadcast::channel(16);
        let dir = std::env::temp_dir().join(format!("aurora-native-{}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);
        let runtime = tokio::runtime::Builder::new_current_thread().enable_all().build().expect("runtime");
        let engine = runtime.block_on(async { start(events, rx, dir.clone(), true, None) });

        // 12 kHz, due periodi interi a partire da un multiplo di 15 s: il messaggio sta nel secondo
        let t0 = 1_000_005_000u64 - 15_000;
        let mut wave = modem::encode_message("CQ K1ABC FN42", 1500.0, &mut CallHashes::default()).expect("onda");
        let mut audio = vec![0.0f32; 3 * SAMPLE_RATE * 15];
        let mut seed = 7u64;
        for a in audio.iter_mut() {
            seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
            *a = ((seed >> 40) as f32 / (1u64 << 24) as f32 - 0.5) * 0.1;
        }
        let start = (15_000 + 500 + 300) * SAMPLE_RATE / 1000;
        for (i, w) in wave.drain(..).enumerate() {
            audio[start + i] += 0.02 * w;
        }
        for (n, chunk) in audio.chunks(480).enumerate() {
            let end_ms = t0 + ((n + 1) * 480 * 1000 / SAMPLE_RATE) as u64;
            tx.send(AudioChunk { rate: 12_000, samples: chunk.to_vec(), received_unix_ms: end_ms }).expect("invio");
        }
        drop(tx);
        // il thread finisce quando il canale si chiude
        for _ in 0..1200 {
            if engine.status()["slots_decoded"].as_u64().unwrap_or(0) >= 2 {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(50));
        }
        let status = engine.status();
        assert!(status["slots_decoded"].as_u64().unwrap_or(0) >= 2, "{status}");
        assert!(std::fs::read_dir(dir.join("recordings")).map(|d| d.count()).unwrap_or(0) >= 2, "i periodi si registrano in WAV");
        let log = std::fs::read_to_string(dir.join("native-decodes.log")).unwrap_or_default();
        assert!(log.contains("CQ K1ABC FN42"), "{log}");
        let _ = std::fs::remove_dir_all(dir);
    }

    // ---- FT4 e cambio di modo -------------------------------------------------------------------------------------------------------

    /// Rumore deterministico (uniforme, ampiezza +-0,05).
    fn noise_audio(len: usize, seed: u64) -> Vec<f32> {
        let mut s = seed;
        (0..len)
            .map(|_| {
                s = s.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
                ((s >> 40) as f32 / (1u64 << 24) as f32 - 0.5) * 0.1
            })
            .collect()
    }

    /// Audio a 12 kHz da `start_ms` per `duration_ms`, con rumore e le onde date: `(istante in cui comincia il periodo in ms, onda, campioni di anticipo
    /// rispetto a 0,5 s)`: ogni onda sta a 0,5 s dall'inizio del suo periodo.
    fn synth(start_ms: u64, duration_ms: u64, seed: u64, waves: &[(u64, &[f32], usize)]) -> Vec<f32> {
        let mut audio = noise_audio((duration_ms as usize) * SAMPLE_RATE / 1000, seed);
        for (slot_start, wave, lead) in waves {
            let begin = ((slot_start - start_ms) as usize) * SAMPLE_RATE / 1000 + 6_000 - lead;
            for (i, w) in wave.iter().enumerate() {
                if let Some(a) = audio.get_mut(begin + i) {
                    *a += 0.05 * w;
                }
            }
        }
        audio
    }

    /// Manda l'audio al motore a blocchi da 40 ms con l'ora di arrivo giusta.
    fn feed(tx: &mpsc::Sender<AudioChunk>, start_ms: u64, audio: &[f32]) {
        for (n, chunk) in audio.chunks(480).enumerate() {
            let end_ms = start_ms + ((n + 1) * 480 * 1000 / SAMPLE_RATE) as u64;
            tx.send(AudioChunk { rate: 12_000, samples: chunk.to_vec(), received_unix_ms: end_ms }).expect("invio");
        }
    }

    fn live_config(controls: Arc<NativeControls>) -> LiveConfig {
        LiveConfig {
            de_call: "IU3ABC".into(),
            de_grid: "IO91".into(),
            own_continent: Some("EU".into()),
            dial_hz: Arc::new(AtomicU64::new(14_074_000)),
            controls,
            transmitter: None,
            log_calls: None,
            grid_check: None,
            band_of: Arc::new(|_| Some("20m".to_string())),
            annotate: Arc::new(|_, _| (None, None, None)),
        }
    }

    /// Hound: la riga OTP del Fox classico si nasconde (di fabbrica) e, se il codice non e' 000000, si chiede la verifica; senza Hound passa com'e'.
    #[test]
    fn the_fox_code_is_hidden_and_checked_only_in_hound() {
        let controls = Arc::new(NativeControls::default());
        let asked = Arc::new(Mutex::new(Vec::new()));
        let sink = Arc::clone(&asked);
        controls.set_otp_hook(Arc::new(move |otp| {
            if let Ok(mut a) = sink.lock() {
                a.push(otp);
            }
        }));
        let live = live_config(Arc::clone(&controls));
        let (events, mut rx) = broadcast::channel(16);
        let next = AtomicU64::new(1_000_000_000);
        let line = |t: &str| as_decoded(750.0, 0.1, -5, t.to_string());
        let lines = [line("K8R.920749"), line("K8R.000000"), line("CQ K8R FN42")];
        publish(&live, None, &events, &next, 1_005_000, &lines);
        let mut texts = Vec::new();
        while let Ok(Event::Decode(m)) = rx.try_recv() {
            texts.push(m.text);
        }
        assert_eq!(texts.len(), 3, "senza Hound nessun filtro: {texts:?}");
        controls.set_switch("hound", true);
        publish(&live, None, &events, &next, 1_020_000, &lines);
        let mut texts = Vec::new();
        while let Ok(Event::Decode(m)) = rx.try_recv() {
            texts.push(m.text);
        }
        assert_eq!(texts, vec!["CQ K8R FN42".to_string()]);
        let asked = asked.lock().map(|a| a.clone()).unwrap_or_default();
        assert_eq!(asked.len(), 1, "000000 non si verifica");
        assert_eq!((asked[0].call.as_str(), asked[0].code.as_str(), asked[0].slot_start_ms), ("K8R", "920749", 1_020_000));
    }

    /// Aspetta (al massimo `secs` secondi) che sul bus siano comparsi tutti i testi dati; restituisce i messaggi e gli stati letti.
    fn wait_for_texts(
        runtime: &tokio::runtime::Runtime,
        bus: &mut broadcast::Receiver<Event>,
        wanted: &[&str],
        secs: u64,
    ) -> (Vec<DecodedMessage>, Vec<StationStatus>) {
        let (mut decodes, mut statuses) = (Vec::new(), Vec::new());
        let deadline = Instant::now() + std::time::Duration::from_secs(secs);
        while Instant::now() < deadline {
            runtime.block_on(async {
                tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                while let Ok(event) = bus.try_recv() {
                    match event {
                        Event::Decode(m) => decodes.push(m),
                        Event::Status(s) => statuses.push(s),
                        _ => {}
                    }
                }
            });
            if wanted.iter().all(|w| decodes.iter().any(|d| d.text == *w)) {
                break;
            }
        }
        (decodes, statuses)
    }

    /// Un periodo FT4 vero (messaggio sintetico del modulatore ft4 dentro rumore) passato dalla scheda finta fino al bus, con lo stato in FT4.
    #[test]
    fn ft4_audio_is_decoded_into_the_same_events_and_the_status_says_ft4() {
        let (tx, rx) = mpsc::channel();
        let (events, mut bus) = broadcast::channel(512);
        let dir = std::env::temp_dir().join(format!("aurora-ft4-{}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);
        let runtime = tokio::runtime::Builder::new_current_thread().enable_all().build().expect("runtime");
        let controls = Arc::new(NativeControls::default());
        let rig = NativeRig::new(Arc::clone(&controls), None);
        use rig_control::Rig;
        rig.set_mode("FT4").expect("modo");
        let engine = runtime.block_on(async { start(events.clone(), rx, dir.clone(), true, Some(live_config(Arc::clone(&controls)))) });

        // periodi FT4 allineati a 7,5 s: il messaggio sta nel secondo
        let t0 = 1_000_005_000u64;
        let wave = ft4::modem::encode_message("CQ K1ABC FN42", 1500.0, &mut CallHashes::default()).expect("onda");
        let audio = synth(t0, 3 * 7_500 + 6_800, 21, &[(t0 + 7_500, &wave, ft4::NSPS)]);
        feed(&tx, t0, &audio);
        let (decodes, statuses) = wait_for_texts(&runtime, &mut bus, &["CQ K1ABC FN42"], 90);
        let m = decodes.iter().find(|d| d.text == "CQ K1ABC FN42").expect("il messaggio FT4 sul bus");
        assert!((m.freq_hz as i64 - 1500).abs() <= 2, "{}", m.freq_hz);
        assert!(m.id >= 1_000_000_000);
        assert_eq!(m.time_ms as u64, (t0 + 7_500) % DAY_MS, "il messaggio e' del periodo da 7,5 s in cui e' stato ricevuto");
        let s = statuses.last().expect("lo stato");
        assert_eq!((s.mode.as_str(), s.tr_period), ("FT4", Some(7)));
        assert!(engine.status()["slots_decoded"].as_u64().unwrap_or(0) >= 2);
        // i periodi FT4 non finiscono nei WAV (il nome al secondo li sovrascriverebbe)
        assert_eq!(std::fs::read_dir(dir.join("recordings")).map(|d| d.count()).unwrap_or(0), 0);
        drop(tx);
        let _ = std::fs::remove_dir_all(dir);
    }

    /// FT8 -> FT4 -> FT8 senza fermare l'audio: dopo ogni cambio il motore decodifica nel modo nuovo, lo stato lo dice e i periodi hanno la durata giusta.
    #[test]
    fn switching_between_ft8_and_ft4_keeps_decoding_without_losing_the_audio() {
        let (tx, rx) = mpsc::channel();
        let (events, mut bus) = broadcast::channel(512);
        let dir = std::env::temp_dir().join(format!("aurora-switch-{}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);
        let runtime = tokio::runtime::Builder::new_current_thread().enable_all().build().expect("runtime");
        let controls = Arc::new(NativeControls::default());
        let rig = NativeRig::new(Arc::clone(&controls), None);
        use rig_control::Rig;
        let _engine = runtime.block_on(async { start(events.clone(), rx, dir.clone(), false, Some(live_config(Arc::clone(&controls)))) });
        let mut all: Vec<DecodedMessage> = Vec::new();
        let mut modes: Vec<String> = Vec::new();

        // 1) FT8: un periodo da 15 s con il messaggio, poi altri 14,5 s perche' venga consegnato
        let t0 = 1_000_005_000u64;
        let w8 = modem::encode_message("CQ K1ABC FN42", 1500.0, &mut CallHashes::default()).expect("onda");
        feed(&tx, t0, &synth(t0, 30_000, 31, &[(t0, &w8, 0)]));
        let (d, s) = wait_for_texts(&runtime, &mut bus, &["CQ K1ABC FN42"], 120);
        assert!(d.iter().any(|m| m.text == "CQ K1ABC FN42"), "FT8 prima del cambio");
        all.extend(d);
        modes.extend(s.into_iter().map(|s| s.mode));

        // 2) FT4: il pulsante cambia il modo; l'audio continua dall'istante in cui eravamo arrivati
        rig.set_mode("FT4").expect("modo");
        std::thread::sleep(std::time::Duration::from_millis(500));
        let t1 = t0 + 30_000;
        let first_ft4_slot = t1.div_ceil(7_500) * 7_500 + 7_500;
        let w4 = ft4::modem::encode_message("CQ K2DEF FN30", 1100.0, &mut CallHashes::default()).expect("onda");
        feed(&tx, t1, &synth(t1, 30_000, 32, &[(first_ft4_slot, &w4, ft4::NSPS)]));
        let (d, s) = wait_for_texts(&runtime, &mut bus, &["CQ K2DEF FN30"], 120);
        assert!(d.iter().any(|m| m.text == "CQ K2DEF FN30" && m.time_ms as u64 == first_ft4_slot % DAY_MS), "FT4 dopo il cambio: {:?}", d.iter().map(|m| (&m.text, m.time_ms)).collect::<Vec<_>>());
        all.extend(d);
        modes.extend(s.into_iter().map(|s| s.mode));

        // 3) di nuovo FT8
        rig.set_mode("FT8").expect("modo");
        std::thread::sleep(std::time::Duration::from_millis(500));
        let t2 = t1 + 30_000;
        let first_ft8_slot = t2.div_ceil(15_000) * 15_000;
        let w8b = modem::encode_message("CQ K3GHI FN20", 1900.0, &mut CallHashes::default()).expect("onda");
        feed(&tx, t2, &synth(t2, 45_000, 33, &[(first_ft8_slot, &w8b, 0)]));
        let (d, s) = wait_for_texts(&runtime, &mut bus, &["CQ K3GHI FN20"], 120);
        assert!(d.iter().any(|m| m.text == "CQ K3GHI FN20"), "FT8 dopo il ritorno");
        all.extend(d);
        modes.extend(s.into_iter().map(|s| s.mode));
        assert!(modes.iter().any(|m| m == "FT4"), "lo stato ha detto FT4: {modes:?}");
        assert_eq!(modes.last().map(String::as_str), Some("FT8"));
        drop(tx);
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn the_band_watch_ignores_unknown_readings_and_moves_inside_a_band() {
        let mut w = BandWatch::default();
        assert_eq!(w.observe(None), None, "senza banda la lettura non conta");
        assert_eq!(w.observe(Some("20m".into())), None, "la prima lettura non e' un cambio");
        assert_eq!(w.observe(Some("20m".into())), None, "stessa banda");
        assert_eq!(w.observe(None), None, "una risposta persa della radio non e' un cambio");
        assert_eq!(w.observe(Some("20m".into())), None);
        assert_eq!(w.observe(Some("40m".into())), Some(1));
        assert_eq!(w.observe(Some("40m".into())), None);
        assert_eq!(w.observe(Some("20m".into())), Some(2));
    }

    /// Un trasmettitore finto: conta gli arresti.
    #[derive(Default)]
    struct CountingPort {
        halts: std::sync::atomic::AtomicU32,
    }

    impl TxPort for CountingPort {
        fn schedule(&self, _request: tx_native::TxRequest) -> Result<(), String> {
            Ok(())
        }

        fn halt(&self) {
            self.halts.fetch_add(1, Ordering::Relaxed);
        }
    }

    /// Cambio di banda con Tx abilitato: arresto subito, Tx spento, evento sul bus; uno spostamento dentro la banda non fa niente.
    #[test]
    fn a_band_change_halts_tx_switches_it_off_and_publishes_the_event() {
        let (_audio, rx) = mpsc::channel();
        let (events, mut bus) = broadcast::channel(512);
        let dir = std::env::temp_dir().join(format!("aurora-bandchange-{}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);
        let runtime = tokio::runtime::Builder::new_current_thread().enable_all().build().expect("runtime");
        let controls = Arc::new(NativeControls::default());
        let port = Arc::new(CountingPort::default());
        let mut live = live_config(Arc::clone(&controls));
        live.band_of = Arc::new(|hz| Some(if hz < 10_000_000 { "40m" } else { "20m" }.to_string()));
        live.transmitter = Some(Arc::clone(&port) as Arc<dyn TxPort>);
        let dial = Arc::clone(&live.dial_hz);
        let _engine = runtime.block_on(async { start(events.clone(), rx, dir.clone(), false, Some(live)) });
        let wait = |what: &str, cond: &dyn Fn() -> bool| {
            for _ in 0..200 {
                if cond() {
                    return;
                }
                std::thread::sleep(std::time::Duration::from_millis(50));
            }
            panic!("{what}");
        };
        // il motore legge la banda iniziale (20m) e il pilota prende i comandi
        std::thread::sleep(std::time::Duration::from_millis(400));
        wait("CQ non consegnato", &|| controls.send(TxCommand::CallCq { modifier: None }).is_ok());
        wait("Tx non abilitato", &|| tx::view_of(&controls).enabled);
        let halts = port.halts.load(Ordering::Relaxed);

        // dentro la banda: niente
        dial.store(14_080_000, Ordering::Relaxed);
        std::thread::sleep(std::time::Duration::from_millis(600));
        assert!(tx::view_of(&controls).enabled, "uno spostamento dentro la banda non spegne il Tx");
        assert_eq!(port.halts.load(Ordering::Relaxed), halts);
        let mut seen = Vec::new();
        runtime.block_on(async {
            while let Ok(event) = bus.try_recv() {
                seen.push(event);
            }
        });
        assert!(!seen.iter().any(|e| matches!(e, Event::BandChanged { .. })), "nessun evento: {seen:?}");

        // altra banda
        dial.store(7_074_000, Ordering::Relaxed);
        wait("Tx ancora abilitato dopo il cambio di banda", &|| !tx::view_of(&controls).enabled);
        assert!(port.halts.load(Ordering::Relaxed) > halts, "la trasmissione va fermata");
        let view = tx::view_of(&controls);
        assert_eq!((view.dx, view.dx_grid), (None, None));
        let mut band_event = None;
        for _ in 0..40 {
            runtime.block_on(async {
                while let Ok(event) = bus.try_recv() {
                    if let Event::BandChanged { band, dial_hz, epoch } = event {
                        band_event = Some((band, dial_hz, epoch));
                    }
                }
            });
            if band_event.is_some() {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(50));
        }
        assert_eq!(band_event, Some((Some("40m".to_string()), 7_074_000, 1)));
        let _ = std::fs::remove_dir_all(dir);
    }

    /// «Decodifica»: rielabora l'ultimo periodo senza ripubblicare i messaggi gia' usciti e senza errori; la richiesta si consuma.
    #[test]
    fn the_decode_button_reprocesses_the_last_period_without_duplicates() {
        let (tx, rx) = mpsc::channel();
        let (events, mut bus) = broadcast::channel(512);
        let dir = std::env::temp_dir().join(format!("aurora-redecode-{}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);
        let runtime = tokio::runtime::Builder::new_current_thread().enable_all().build().expect("runtime");
        let controls = Arc::new(NativeControls::default());
        let rig = NativeRig::new(Arc::clone(&controls), None);
        use rig_control::Rig;
        // senza nessun periodo ancora decodificato la richiesta e' accettata senza errore
        rig.set_control("decode", None).expect("accettato");
        let _engine = runtime.block_on(async { start(events.clone(), rx, dir.clone(), false, Some(live_config(Arc::clone(&controls)))) });
        let t0 = 1_000_005_000u64;
        let wave = modem::encode_message("CQ K1ABC FN42", 1500.0, &mut CallHashes::default()).expect("onda");
        feed(&tx, t0, &synth(t0, 30_000, 41, &[(t0, &wave, 0)]));
        let (first, _) = wait_for_texts(&runtime, &mut bus, &["CQ K1ABC FN42"], 120);
        assert_eq!(first.iter().filter(|m| m.text == "CQ K1ABC FN42").count(), 1);
        // il motore e' fermo in attesa di audio: la richiesta viene raccolta al giro dopo
        rig.set_control("decode", None).expect("accettato");
        // il motore puo' essere ancora occupato a decodificare il secondo periodo: si aspetta che raccolga la richiesta
        for _ in 0..1200 {
            if !controls.redecode.load(Ordering::Relaxed) {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(50));
        }
        assert!(!controls.redecode.load(Ordering::Relaxed), "la richiesta e' stata raccolta");
        let (more, _) = wait_for_texts(&runtime, &mut bus, &["mai"], 3);
        assert!(more.iter().all(|m| m.text != "CQ K1ABC FN42"), "nessun doppione: {:?}", more.iter().map(|m| &m.text).collect::<Vec<_>>());
        drop(tx);
        let _ = std::fs::remove_dir_all(dir);
    }
}
