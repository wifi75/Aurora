//! Modalita' «PC lento»: su un PC che fatica, le attivita' di contorno (DX Cluster, DXpedition, PSK Reporter, ricerca dei nominativi,
//! elenco LoTW, dati dei satelliti, aggiornamenti...) aspettano che la decodifica sia finita prima del loro lavoro pesante, cosi' non
//! rubano CPU all'audio e alla decodifica (blocchi audio persi, periodi che sforano).
//!
//! La «porta»: [`is_free`] dice se adesso e' un momento libero; [`wait_free`] (asincrona) e [`wait_free_blocking`] (thread) aspettano il
//! primo momento libero, al massimo per il tempo dato (i dati non si perdono e non invecchiano oltre quel limite). Con la modalita' spenta
//! la porta e' sempre aperta: comportamento di sempre.
//!
//! Quando si accende (`Auto`, il valore di fabbrica):
//! - almeno [`SLOW_IN_WINDOW`] degli ultimi [`WINDOW`] periodi decodificati hanno preso oltre il [`SLOW_FRACTION`] del periodo
//!   (FT8: 10,5 s su 15, la stessa soglia dell'avviso «decodifica lenta»);
//! - oppure l'audio ha perso almeno [`LOST_BLOCKS`] blocchi di campioni negli ultimi [`LOST_WINDOW`];
//! - oppure il PC ha al massimo [`FEW_CORES`] processori (sempre accesa).
//!
//! Si spegne dopo [`CALM_PERIODS`] periodi di fila sotto il [`CALM_FRACTION`] del periodo, senza blocchi persi nell'ultimo
//! [`LOST_WINDOW`]. `Always` la tiene accesa, `Never` spenta.

use std::{
    collections::VecDeque,
    sync::{
        Condvar, Mutex, OnceLock,
        atomic::{AtomicBool, AtomicU8, AtomicUsize, Ordering},
    },
    time::{Duration, Instant},
};

/// Oltre questa parte del periodo la decodifica e' lenta (come `engine_native::slow::SLOW_FRACTION`).
pub const SLOW_FRACTION: f64 = 0.7;
/// Su quanti periodi si guarda e quanti lenti accendono la modalita'.
pub const WINDOW: usize = 10;
pub const SLOW_IN_WINDOW: usize = 3;
/// Quanti periodi di fila sotto `CALM_FRACTION` la spengono.
pub const CALM_PERIODS: usize = 20;
pub const CALM_FRACTION: f64 = 0.5;
/// Blocchi audio persi che accendono la modalita', e in quanto tempo.
pub const LOST_BLOCKS: usize = 3;
pub const LOST_WINDOW: Duration = Duration::from_secs(600);
/// Con cosi' pochi processori la modalita' automatica resta sempre accesa.
pub const FEW_CORES: usize = 2;
/// L'attesa piu' lunga consigliata per un lavoro di contorno: oltre, si fa comunque (gli spot non diventano vecchi).
pub const MAX_WAIT: Duration = Duration::from_secs(30);

/// L'impostazione «PC lento» (`SlowPcMode` nelle impostazioni del motore).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum SlowPcMode {
    #[default]
    Auto,
    Always,
    Never,
}

impl SlowPcMode {
    /// Dal valore salvato; un valore sconosciuto vale `Auto`.
    pub fn parse(s: &str) -> SlowPcMode {
        match s.trim().to_ascii_lowercase().as_str() {
            "always" => SlowPcMode::Always,
            "never" => SlowPcMode::Never,
            _ => SlowPcMode::Auto,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            SlowPcMode::Auto => "auto",
            SlowPcMode::Always => "always",
            SlowPcMode::Never => "never",
        }
    }

    fn code(self) -> u8 {
        match self {
            SlowPcMode::Auto => 0,
            SlowPcMode::Always => 1,
            SlowPcMode::Never => 2,
        }
    }

    fn from_code(c: u8) -> SlowPcMode {
        match c {
            1 => SlowPcMode::Always,
            2 => SlowPcMode::Never,
            _ => SlowPcMode::Auto,
        }
    }
}

/// Il riconoscimento automatico: periodi lenti e blocchi audio persi.
#[derive(Debug)]
pub struct Detector {
    recent: VecDeque<bool>,
    calm: usize,
    lost: VecDeque<Instant>,
    on: bool,
    cores: usize,
}

/// Perche' la modalita' automatica e' accesa (per i log e la diagnosi).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Reason {
    SlowDecode,
    LostAudio,
    FewCores,
}

impl Detector {
    pub fn new(cores: usize) -> Detector {
        Detector { recent: VecDeque::new(), calm: 0, lost: VecDeque::new(), on: false, cores }
    }

    /// Un periodo decodificato in `millis` su un periodo lungo `period_ms`. Restituisce lo stato nuovo se e' cambiato.
    pub fn period(&mut self, millis: u64, period_ms: u64, now: Instant) -> Option<bool> {
        if period_ms == 0 {
            return None;
        }
        let part = millis as f64 / period_ms as f64;
        self.recent.push_back(part >= SLOW_FRACTION);
        if self.recent.len() > WINDOW {
            self.recent.pop_front();
        }
        self.calm = if part < CALM_FRACTION { self.calm + 1 } else { 0 };
        self.update(now)
    }

    /// L'audio ha perso un blocco di campioni.
    pub fn audio_lost(&mut self, now: Instant) -> Option<bool> {
        self.lost.push_back(now);
        self.update(now)
    }

    fn update(&mut self, now: Instant) -> Option<bool> {
        while self.lost.front().is_some_and(|t| now.saturating_duration_since(*t) > LOST_WINDOW) {
            self.lost.pop_front();
        }
        let before = self.on;
        if self.reason().is_some() && !self.on {
            self.on = true;
        } else if self.on && self.cores > FEW_CORES && self.calm >= CALM_PERIODS && self.lost.is_empty() {
            self.on = false;
            self.recent.clear();
        }
        (self.on != before).then_some(self.on)
    }

    /// Il motivo per accendersi adesso, se c'e'.
    pub fn reason(&self) -> Option<Reason> {
        if self.cores <= FEW_CORES {
            Some(Reason::FewCores)
        } else if self.recent.iter().filter(|s| **s).count() >= SLOW_IN_WINDOW {
            Some(Reason::SlowDecode)
        } else if self.lost.len() >= LOST_BLOCKS {
            Some(Reason::LostAudio)
        } else {
            None
        }
    }

    pub fn is_on(&self) -> bool {
        self.on || self.cores <= FEW_CORES
    }
}

struct State {
    mode: AtomicU8,
    auto_on: AtomicBool,
    decoding: AtomicUsize,
    detector: Mutex<Detector>,
    gate: Mutex<()>,
    freed: Condvar,
}

fn state() -> &'static State {
    static STATE: OnceLock<State> = OnceLock::new();
    STATE.get_or_init(|| {
        let cores = std::thread::available_parallelism().map_or(1, |n| n.get());
        let detector = Detector::new(cores);
        let on = detector.is_on();
        State { mode: AtomicU8::new(0), auto_on: AtomicBool::new(on), decoding: AtomicUsize::new(0), detector: Mutex::new(detector), gate: Mutex::new(()), freed: Condvar::new() }
    })
}

fn wake_waiters() {
    let s = state();
    let _g = s.gate.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
    s.freed.notify_all();
}

/// L'impostazione «PC lento: automatico / sempre / mai».
pub fn set_mode(mode: SlowPcMode) {
    let s = state();
    if s.mode.swap(mode.code(), Ordering::SeqCst) != mode.code() {
        tracing_line(&format!("PC lento: impostazione {}", mode.as_str()));
        wake_waiters();
    }
}

pub fn mode() -> SlowPcMode {
    SlowPcMode::from_code(state().mode.load(Ordering::Relaxed))
}

/// La modalita' e' accesa adesso.
pub fn active() -> bool {
    let s = state();
    match mode() {
        SlowPcMode::Always => true,
        SlowPcMode::Never => false,
        SlowPcMode::Auto => s.auto_on.load(Ordering::Relaxed),
    }
}

/// La decodifica e' in corso.
pub fn decoding() -> bool {
    state().decoding.load(Ordering::SeqCst) > 0
}

/// Adesso e' un momento libero: modalita' spenta, oppure nessuna decodifica in corso.
pub fn is_free() -> bool {
    !active() || !decoding()
}

/// Segna una decodifica in corso finche' il valore vive (anche se la decodifica va in panic).
#[must_use = "la decodifica e' «in corso» finche' il valore vive"]
pub struct Busy(());

impl Drop for Busy {
    fn drop(&mut self) {
        if state().decoding.fetch_sub(1, Ordering::SeqCst) == 1 {
            wake_waiters();
        }
    }
}

/// Comincia una decodifica (il motore la chiama attorno al lavoro di ogni periodo).
pub fn busy() -> Busy {
    state().decoding.fetch_add(1, Ordering::SeqCst);
    Busy(())
}

fn set_auto(changed: Option<bool>, why: Option<Reason>) {
    if let Some(on) = changed {
        state().auto_on.store(on, Ordering::SeqCst);
        let why = match why {
            Some(Reason::SlowDecode) => "decodifiche oltre il 70% del periodo",
            Some(Reason::LostAudio) => "blocchi audio persi",
            Some(Reason::FewCores) => "pochi processori",
            None => "PC di nuovo tranquillo",
        };
        tracing_line(&format!("PC lento: modalita' automatica {} ({why})", if on { "accesa" } else { "spenta" }));
        wake_waiters();
    }
}

/// Il motore ha decodificato un periodo in `millis` su `period_ms`.
pub fn note_period(millis: u64, period_ms: u64) {
    let mut d = state().detector.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
    let changed = d.period(millis, period_ms, Instant::now());
    let why = d.reason();
    drop(d);
    set_auto(changed, why);
}

/// L'audio ha perso un blocco di campioni.
pub fn note_audio_lost() {
    let mut d = state().detector.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
    let changed = d.audio_lost(Instant::now());
    let why = d.reason();
    drop(d);
    set_auto(changed, why);
}

/// Aspetta il primo momento libero, al massimo `max` (thread). `true` se ha dovuto aspettare.
pub fn wait_free_blocking(max: Duration) -> bool {
    if is_free() {
        return false;
    }
    let s = state();
    let until = Instant::now() + max;
    let mut g = s.gate.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
    while !is_free() {
        let now = Instant::now();
        if now >= until {
            break;
        }
        // si ricontrolla almeno ogni mezzo secondo: la modalita' puo' spegnersi senza che nessuno svegli
        let step = (until - now).min(Duration::from_millis(500));
        g = match s.freed.wait_timeout(g, step) {
            Ok((g, _)) => g,
            Err(p) => p.into_inner().0,
        };
    }
    true
}

/// Aspetta il primo momento libero, al massimo `max` (asincrona). `true` se ha dovuto aspettare.
pub async fn wait_free(max: Duration) -> bool {
    if is_free() {
        return false;
    }
    let until = tokio::time::Instant::now() + max;
    while !is_free() && tokio::time::Instant::now() < until {
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    true
}

/// Lo stato per la diagnosi e l'interfaccia: impostazione, accesa, decodifica in corso.
pub fn status() -> (SlowPcMode, bool, bool) {
    (mode(), active(), decoding())
}

fn tracing_line(text: &str) {
    tracing::info!(target: "aurora::pc_lento", "{text}");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn un_pc_veloce_non_la_accende_mai() {
        let mut d = Detector::new(16);
        let t = Instant::now();
        for _ in 0..100 {
            assert_eq!(d.period(4_000, 15_000, t), None);
        }
        assert!(!d.is_on());
    }

    #[test]
    fn tre_periodi_lenti_su_dieci_la_accendono_e_venti_tranquilli_la_spengono() {
        let mut d = Detector::new(8);
        let t = Instant::now();
        assert_eq!(d.period(11_000, 15_000, t), None);
        assert_eq!(d.period(3_000, 15_000, t), None);
        assert_eq!(d.period(11_000, 15_000, t), None);
        assert_eq!(d.period(11_000, 15_000, t), Some(true));
        assert_eq!(d.reason(), Some(Reason::SlowDecode));
        for i in 0..CALM_PERIODS {
            let changed = d.period(5_000, 15_000, t);
            assert_eq!(changed, if i + 1 == CALM_PERIODS { Some(false) } else { None }, "periodo {i}");
        }
        assert!(!d.is_on());
    }

    #[test]
    fn un_periodo_medio_ricomincia_il_conto_dei_tranquilli() {
        let mut d = Detector::new(8);
        let t = Instant::now();
        for _ in 0..3 {
            d.period(12_000, 15_000, t);
        }
        assert!(d.is_on());
        for _ in 0..CALM_PERIODS - 1 {
            d.period(5_000, 15_000, t);
        }
        // 9 s su 15: non lento, ma nemmeno tranquillo
        d.period(9_000, 15_000, t);
        assert!(d.is_on());
    }

    #[test]
    fn blocchi_audio_persi_la_accendono_e_scadono_dopo_dieci_minuti() {
        let mut d = Detector::new(8);
        let t = Instant::now();
        assert_eq!(d.audio_lost(t), None);
        assert_eq!(d.audio_lost(t), None);
        assert_eq!(d.audio_lost(t + Duration::from_secs(5)), Some(true));
        assert_eq!(d.reason(), Some(Reason::LostAudio));
        // periodi tranquilli ma blocchi persi ancora recenti: resta accesa
        for _ in 0..CALM_PERIODS {
            d.period(2_000, 15_000, t + Duration::from_secs(60));
        }
        assert!(d.is_on());
        // passati dieci minuti dall'ultimo blocco perso
        assert_eq!(d.period(2_000, 15_000, t + LOST_WINDOW + Duration::from_secs(10)), Some(false));
    }

    #[test]
    fn con_due_processori_resta_accesa() {
        let mut d = Detector::new(2);
        assert!(d.is_on());
        let t = Instant::now();
        for _ in 0..50 {
            d.period(1_000, 15_000, t);
        }
        assert!(d.is_on());
    }

    #[test]
    fn il_nome_dell_impostazione() {
        assert_eq!(SlowPcMode::parse("ALWAYS"), SlowPcMode::Always);
        assert_eq!(SlowPcMode::parse("never"), SlowPcMode::Never);
        assert_eq!(SlowPcMode::parse("boh"), SlowPcMode::Auto);
        assert_eq!(SlowPcMode::parse(SlowPcMode::Never.as_str()), SlowPcMode::Never);
    }

    #[test]
    fn la_porta_aspetta_la_fine_della_decodifica_solo_con_la_modalita_accesa() {
        // un solo test tocca lo stato globale: i test girano in parallelo
        set_mode(SlowPcMode::Never);
        let b = busy();
        assert!(is_free(), "modalita' spenta: porta sempre aperta");
        set_mode(SlowPcMode::Always);
        assert!(!is_free());
        let started = Instant::now();
        assert!(wait_free_blocking(Duration::from_millis(150)), "aspetta fino al limite");
        assert!(started.elapsed() >= Duration::from_millis(140));
        let waiter = std::thread::spawn(|| {
            let t = Instant::now();
            wait_free_blocking(Duration::from_secs(10));
            t.elapsed()
        });
        std::thread::sleep(Duration::from_millis(100));
        drop(b);
        let waited = waiter.join().unwrap_or(Duration::from_secs(99));
        assert!(waited < Duration::from_secs(2), "si sveglia a fine decodifica, non al limite: {waited:?}");
        assert!(is_free());
        set_mode(SlowPcMode::Auto);
    }
}
