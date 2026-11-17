//! Spegnimento del PTT: un solo responsabile per tutti i tentativi.
//!
//! Quando la radio non risponde (cavo USB-seriale staccato, CAT caduto) il PTT non si spegne. Prima ogni livello (CAT, blocco esterno,
//! guardia, controllo a tempo) rifaceva i suoi tentativi e ognuno scriveva una riga di errore: nei rapporti 14 righe uguali. Ora:
//!
//! - la scala dei tentativi e' una sola: subito, dopo 200 ms, 500 ms, 1 s, 2 s, poi uno ogni 5 s finche' la radio non risponde;
//! - dal terzo tentativo fallito si prova anche ogni altra via della radio ([`Ptt::recover_off`]: riapertura del CAT, linea RTS/DTR);
//! - lo stato «PTT non spento» e' unico ([`ReleaseTracker`]): finche' dura la trasmissione resta bloccata e l'interfaccia mostra un avviso;
//! - il registro ha una riga alla fine dei primi tentativi (quanti, in quanto tempo, perche') e una quando il PTT e' spento.

use std::{
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

use crate::{error::TxError, lock, ptt::Ptt};

/// I tempi della scala. Fuori dalle prove sono sempre quelli di [`ReleaseTiming::default`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReleaseTiming {
    /// L'attesa prima di ciascuno dei primi tentativi (il primo e' subito).
    pub ladder: [Duration; 5],
    /// Poi un tentativo ogni questo tempo.
    pub long_step: Duration,
}

impl Default for ReleaseTiming {
    fn default() -> Self {
        ReleaseTiming {
            ladder: [Duration::ZERO, Duration::from_millis(200), Duration::from_millis(500), Duration::from_secs(1), Duration::from_secs(2)],
            long_step: Duration::from_secs(5),
        }
    }
}

impl ReleaseTiming {
    /// L'attesa prima del tentativo numero `attempt` (da 0). Oltre la scala vale `long_step`.
    pub fn delay_before(&self, attempt: u32) -> Duration {
        self.ladder.get(attempt as usize).copied().unwrap_or(self.long_step)
    }

    /// Tempi brevi per le prove.
    pub fn fast() -> ReleaseTiming {
        let ms = Duration::from_millis;
        ReleaseTiming { ladder: [ms(0), ms(2), ms(4), ms(6), ms(8)], long_step: ms(30) }
    }
}

/// L'avviso all'interfaccia parte dal terzo fallimento o dopo questo tempo dall'inizio: un secondo tentativo che riesce subito non da' allarmi.
const WARN_AFTER_ATTEMPTS: u32 = 3;
const WARN_AFTER: Duration = Duration::from_secs(1);

/// Dal tentativo (da 0) in poi si provano anche le altre vie: i primi due sono il comando normale, che di solito riesce al secondo colpo.
const RECOVER_FROM_ATTEMPT: u32 = 2;

/// Il PTT non e' spento: da quando, quanti tentativi, perche'.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StuckInfo {
    pub attempts: u32,
    pub secs: u64,
    pub reason: String,
}

/// Chi viene avvisato quando il PTT resta acceso (`Some`, a ogni tentativo fallito) e quando e' di nuovo spento (`None`).
pub type StuckHook = Arc<dyn Fn(Option<StuckInfo>) + Send + Sync>;

struct Episode {
    since: Instant,
    attempts: u32,
    last: TxError,
    /// La riga riepilogativa del registro e' gia' uscita.
    logged: bool,
}

pub struct ReleaseTracker {
    episode: Mutex<Option<Episode>>,
    timing: Mutex<ReleaseTiming>,
    hook: Mutex<Option<StuckHook>>,
}

impl Default for ReleaseTracker {
    fn default() -> Self {
        ReleaseTracker { episode: Mutex::new(None), timing: Mutex::new(ReleaseTiming::default()), hook: Mutex::new(None) }
    }
}

/// Il testo di «PTT non spento» con la causa una volta sola: l'errore del CAT porta gia' «PTT non spento: » davanti e `TxError::Ptt`
/// aggiunge «PTT: », e sullo schermo e nel registro usciva «PTT non spento: PTT: PTT non spento: …».
pub fn not_released_text(e: &TxError) -> String {
    format!("PTT non spento: {}", cause(e))
}

fn cause(e: &TxError) -> String {
    let c = match e {
        TxError::Ptt(c) => c.clone(),
        other => other.to_string(),
    };
    c.strip_prefix("PTT non spento: ").map(str::to_string).unwrap_or(c)
}

impl ReleaseTracker {
    pub fn new() -> Arc<ReleaseTracker> {
        Arc::new(ReleaseTracker::default())
    }

    pub fn set_timing(&self, timing: ReleaseTiming) {
        *lock(&self.timing) = timing;
    }

    pub fn timing(&self) -> ReleaseTiming {
        *lock(&self.timing)
    }

    /// Chi ascolta riceve subito lo stato di adesso, se il PTT e' gia' non spento (catena ricostruita con il PTT bloccato).
    pub fn set_hook(&self, hook: Option<StuckHook>) {
        *lock(&self.hook) = hook;
        if let Some(info) = self.info()
            && info.attempts >= WARN_AFTER_ATTEMPTS
        {
            self.notify(Some(info));
        }
    }

    /// Il PTT non e' confermato spento.
    pub fn is_stuck(&self) -> bool {
        lock(&self.episode).is_some()
    }

    pub fn info(&self) -> Option<StuckInfo> {
        lock(&self.episode).as_ref().map(|e| StuckInfo { attempts: e.attempts, secs: e.since.elapsed().as_secs(), reason: cause(&e.last) })
    }

    /// La causa dell'ultimo errore, se il PTT non e' spento.
    pub fn last_error(&self) -> Option<TxError> {
        lock(&self.episode).as_ref().map(|e| e.last.clone())
    }

    fn notify(&self, info: Option<StuckInfo>) {
        let hook = lock(&self.hook).clone();
        if let Some(h) = hook {
            h(info);
        }
    }

    /// Un tentativo, poi (dal terzo) le altre vie. `Ok` = la radio ha confermato lo spegnimento.
    fn attempt(&self, ptt: &dyn Ptt, index: u32) -> Result<(), TxError> {
        let error = match ptt.set(false) {
            Ok(()) => {
                self.cleared(true);
                return Ok(());
            }
            Err(e) => e,
        };
        tracing::debug!("PTT: spegnimento fallito (tentativo {}): {error}", index + 1);
        self.failed(&error);
        if index >= RECOVER_FROM_ATTEMPT {
            match ptt.recover_off() {
                Ok(()) => {
                    self.cleared(true);
                    return Ok(());
                }
                Err(e) => tracing::debug!("PTT: altra via non riuscita: {e}"),
            }
        }
        Err(error)
    }

    fn failed(&self, error: &TxError) {
        let (info, warn) = {
            let mut ep = lock(&self.episode);
            let e = ep.get_or_insert_with(|| Episode { since: Instant::now(), attempts: 0, last: error.clone(), logged: false });
            e.attempts += 1;
            e.last = error.clone();
            let warn = e.attempts >= WARN_AFTER_ATTEMPTS || e.since.elapsed() >= WARN_AFTER;
            (StuckInfo { attempts: e.attempts, secs: e.since.elapsed().as_secs(), reason: cause(&e.last) }, warn)
        };
        // la trasmissione e' bloccata da subito (`is_stuck`); l'avviso rosso solo quando non e' piu' un singolo intoppo
        if warn {
            self.notify(Some(info));
        }
    }

    /// Un PTT rimasto acceso da una catena di trasmissione che non c'e' piu': si parte gia' «non spento», con l'avviso, e la scala continua.
    pub fn adopt(&self, reason: &str) {
        let info = {
            let mut ep = lock(&self.episode);
            if ep.is_some() {
                return;
            }
            *ep = Some(Episode { since: Instant::now(), attempts: WARN_AFTER_ATTEMPTS, last: TxError::Ptt(reason.to_string()), logged: true });
            StuckInfo { attempts: WARN_AFTER_ATTEMPTS, secs: 0, reason: reason.to_string() }
        };
        tracing::error!("PTT: {reason}: trasmissione bloccata");
        self.notify(Some(info));
    }

    /// L'episodio e' finito. `by_command`: la radio ha confermato uno spegnimento; altrimenti il PTT risulta solo non piu' acceso (per
    /// esempio spento dal blocco esterno, o PTT di una radio abbandonata): la riga del registro non deve dire che lo abbiamo spento noi.
    fn cleared(&self, by_command: bool) {
        let Some(e) = lock(&self.episode).take() else { return };
        let secs = e.since.elapsed().as_secs_f32();
        if by_command {
            tracing::info!("PTT: spento dopo {} tentativi in {:.1} s ({})", e.attempts, secs, cause(&e.last));
        } else {
            tracing::info!("PTT: non risulta piu' acceso dopo {} tentativi in {:.1} s ({}); stato «non spento» tolto", e.attempts, secs, cause(&e.last));
        }
        self.notify(None);
    }

    /// Una riga sola nel registro per tutta la serie di tentativi falliti.
    fn log_summary(&self) {
        let mut ep = lock(&self.episode);
        if let Some(e) = ep.as_mut()
            && !e.logged
        {
            e.logged = true;
            tracing::error!("PTT: spegnimento fallito, {} tentativi in {} s: {}", e.attempts, e.since.elapsed().as_secs(), cause(&e.last));
        }
    }

    /// La serie di tentativi della scala (0, 200 ms, 500 ms, 1 s, 2 s). Chi chiama resta fermo fino alla fine: e' il thread della
    /// trasmissione, che non ha altro da fare. Se fallisce, il controllo a tempo continua con [`ReleaseTracker::retry_once`].
    pub fn release(&self, ptt: &dyn Ptt) -> Result<(), TxError> {
        let timing = self.timing();
        let mut last = TxError::Ptt("nessun tentativo".into());
        for i in 0..timing.ladder.len() as u32 {
            let wait = timing.delay_before(i);
            if !wait.is_zero() {
                std::thread::sleep(wait);
            }
            match self.attempt(ptt, i) {
                Ok(()) => return Ok(()),
                Err(e) => last = e,
            }
        }
        self.log_summary();
        Err(last)
    }

    /// Un solo tentativo (con le altre vie): per il controllo a tempo, che ripete ogni `long_step`, e per la guardia che cade.
    pub fn retry_once(&self, ptt: &dyn Ptt) -> Result<(), TxError> {
        // in una serie gia' lunga le altre vie si provano a ogni giro
        let index = self.info().map_or(0, |i| i.attempts).max(RECOVER_FROM_ATTEMPT);
        let result = self.attempt(ptt, index);
        if result.is_err() {
            self.log_summary();
        }
        result
    }

    /// Il PTT risulta spento senza un nostro comando riuscito (per esempio la radio si e' riconnessa e lo ha spento da sola):
    /// lo stato «non spento» si toglie.
    pub fn confirm_off(&self) {
        self.cleared(false);
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};

    use super::*;

    /// PTT che fallisce finche' `fail` e' vero, con il conto dei tentativi e un'altra via che riesce se `recover_ok`.
    struct Flaky {
        fail: AtomicBool,
        on: AtomicBool,
        tries: AtomicU32,
        recovers: AtomicU32,
        recover_ok: AtomicBool,
    }

    impl Flaky {
        fn new(fail: bool) -> Flaky {
            Flaky { fail: AtomicBool::new(fail), on: AtomicBool::new(true), tries: AtomicU32::new(0), recovers: AtomicU32::new(0), recover_ok: AtomicBool::new(false) }
        }
    }

    impl Ptt for Flaky {
        fn set(&self, on: bool) -> Result<(), TxError> {
            if !on {
                self.tries.fetch_add(1, Ordering::SeqCst);
            }
            if self.fail.load(Ordering::SeqCst) {
                return Err(TxError::Ptt("Errore sulla porta seriale".into()));
            }
            self.on.store(on, Ordering::SeqCst);
            Ok(())
        }

        fn is_on(&self) -> bool {
            self.on.load(Ordering::SeqCst)
        }

        fn recover_off(&self) -> Result<(), TxError> {
            self.recovers.fetch_add(1, Ordering::SeqCst);
            if self.recover_ok.load(Ordering::SeqCst) {
                self.on.store(false, Ordering::SeqCst);
                Ok(())
            } else {
                Err(TxError::Ptt("nessuna via".into()))
            }
        }
    }

    fn tracker() -> Arc<ReleaseTracker> {
        let t = ReleaseTracker::new();
        t.set_timing(ReleaseTiming::fast());
        t
    }

    #[test]
    fn the_ladder_is_progressive_and_then_every_five_seconds() {
        let t = ReleaseTiming::default();
        let ms: Vec<u128> = (0..8).map(|i| t.delay_before(i).as_millis()).collect();
        assert_eq!(ms, [0, 200, 500, 1000, 2000, 5000, 5000, 5000]);
    }

    #[test]
    fn a_working_ptt_is_off_at_the_first_attempt_and_nothing_is_reported() {
        let t = tracker();
        let ptt = Flaky::new(false);
        t.release(&ptt).expect("spento");
        assert_eq!(ptt.tries.load(Ordering::SeqCst), 1);
        assert!(!t.is_stuck());
    }

    #[test]
    fn a_failing_ptt_gets_the_whole_ladder_and_the_other_ways_from_the_third_attempt() {
        let t = tracker();
        let calls = Arc::new(Mutex::new(Vec::<Option<StuckInfo>>::new()));
        let seen = Arc::clone(&calls);
        t.set_hook(Some(Arc::new(move |i| lock(&seen).push(i))));
        let ptt = Flaky::new(true);
        let err = t.release(&ptt).expect_err("non spento");
        assert!(not_released_text(&err).starts_with("PTT non spento: Errore sulla porta"));
        assert_eq!(ptt.tries.load(Ordering::SeqCst), 5, "un tentativo per gradino della scala");
        assert_eq!(ptt.recovers.load(Ordering::SeqCst), 3, "le altre vie dal terzo tentativo");
        assert!(t.is_stuck());
        let info = t.info().expect("stato");
        assert_eq!(info.attempts, 5);
        assert_eq!(info.reason, "Errore sulla porta seriale");
        // l'avviso parte dal terzo fallimento e si aggiorna a ogni tentativo; il blocco c'e' dal primo
        let calls = lock(&calls).clone();
        assert_eq!(calls.len(), 3);
        assert_eq!(calls[0].as_ref().map(|i| i.attempts), Some(3));
    }

    #[test]
    fn the_exit_after_success_clears_the_state_and_tells_it() {
        let t = tracker();
        let calls = Arc::new(Mutex::new(Vec::<Option<StuckInfo>>::new()));
        let seen = Arc::clone(&calls);
        t.set_hook(Some(Arc::new(move |i| lock(&seen).push(i))));
        let ptt = Flaky::new(true);
        assert!(t.release(&ptt).is_err());
        ptt.fail.store(false, Ordering::SeqCst);
        t.retry_once(&ptt).expect("spento al giro dopo");
        assert!(!t.is_stuck());
        assert!(!ptt.is_on());
        assert_eq!(lock(&calls).last(), Some(&None), "l'avviso si toglie");
    }

    #[test]
    fn another_way_that_works_ends_the_episode_at_once() {
        let t = tracker();
        let ptt = Flaky::new(true);
        ptt.recover_ok.store(true, Ordering::SeqCst);
        t.release(&ptt).expect("spento con l'altra via");
        assert!(!t.is_stuck());
        assert_eq!(ptt.tries.load(Ordering::SeqCst), 3, "i primi tentativi normali e al terzo l'altra via");
    }

    #[test]
    fn one_failure_followed_by_success_gives_no_warning() {
        let t = tracker();
        let calls = Arc::new(Mutex::new(Vec::<Option<StuckInfo>>::new()));
        let seen = Arc::clone(&calls);
        t.set_hook(Some(Arc::new(move |i| lock(&seen).push(i))));
        let ptt = Flaky::new(true);
        t.attempt(&ptt, 0).expect_err("primo tentativo fallito");
        assert!(t.is_stuck(), "il blocco della trasmissione c'e' subito");
        assert!(lock(&calls).iter().all(Option::is_none), "ma nessun avviso rosso");
        ptt.fail.store(false, Ordering::SeqCst);
        t.attempt(&ptt, 1).expect("spento");
        assert!(!t.is_stuck());
        assert!(lock(&calls).iter().all(Option::is_none), "nessun avviso rosso a spegnimento riuscito");
    }

    #[test]
    fn an_adopted_episode_warns_at_once_and_a_late_hook_hears_it() {
        let t = tracker();
        t.adopt("PTT precedente non spento");
        assert!(t.is_stuck());
        let calls = Arc::new(Mutex::new(Vec::<Option<StuckInfo>>::new()));
        let seen = Arc::clone(&calls);
        t.set_hook(Some(Arc::new(move |i| lock(&seen).push(i))));
        assert_eq!(lock(&calls).len(), 1);
        assert!(lock(&calls)[0].is_some());
    }

    #[test]
    fn a_ptt_found_off_by_itself_clears_the_state() {
        let t = tracker();
        let ptt = Flaky::new(true);
        assert!(t.release(&ptt).is_err());
        ptt.on.store(false, Ordering::SeqCst);
        t.confirm_off();
        assert!(!t.is_stuck());
    }
}
