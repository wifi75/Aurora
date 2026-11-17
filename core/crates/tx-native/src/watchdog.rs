//! Controllo di sicurezza del PTT: un thread proprio, a tempo vero (non quello dell'orologio iniettato), che spegne il PTT se
//! resta acceso oltre la scadenza. Copre il caso in cui il thread di riproduzione si blocchi (dispositivo che non risponde,
//! comando che non torna) e non arrivi mai a spegnere.

use std::{
    sync::{
        Arc, Condvar, Mutex,
        atomic::{AtomicU64, Ordering},
    },
    thread::JoinHandle,
    time::Instant,
};

use crate::{error::TxError, lock, ptt::Ptt, ptt_release::ReleaseTracker};

struct State {
    deadline: Option<Instant>,
    quit: bool,
}

struct Shared {
    state: Mutex<State>,
    changed: Condvar,
    trips: AtomicU64,
}

pub struct Watchdog {
    shared: Arc<Shared>,
    tracker: Arc<ReleaseTracker>,
    thread: Option<JoinHandle<()>>,
}

impl Watchdog {
    /// Avvia il controllo. `on_trip` si chiama quando scatta (il pianificatore ci ferma anche l'audio); se lo spegnimento
    /// fallisce si riprova a ogni `long_step` del `tracker` (5 s), finche' la radio non risponde.
    pub fn start(ptt: Arc<dyn Ptt>, tracker: Arc<ReleaseTracker>, on_trip: Box<dyn Fn() + Send + Sync>) -> Result<Watchdog, TxError> {
        let shared = Arc::new(Shared { state: Mutex::new(State { deadline: None, quit: false }), changed: Condvar::new(), trips: AtomicU64::new(0) });
        let thread_shared = Arc::clone(&shared);
        let thread_tracker = Arc::clone(&tracker);
        let thread = std::thread::Builder::new()
            .name("aurora-tx-watchdog".into())
            .spawn(move || {
                let mut st = lock(&thread_shared.state);
                loop {
                    if st.quit {
                        return;
                    }
                    let Some(deadline) = st.deadline else {
                        st = thread_shared.changed.wait(st).unwrap_or_else(|p| p.into_inner());
                        continue;
                    };
                    let now = Instant::now();
                    if now < deadline {
                        st = thread_shared.changed.wait_timeout(st, deadline - now).unwrap_or_else(|p| p.into_inner()).0;
                        continue;
                    }
                    drop(st);
                    let mut ok = true;
                    if ptt.is_on() {
                        // la prima volta e' la scadenza (si ferma anche l'audio); i giri dopo sono solo tentativi di spegnimento
                        if !thread_tracker.is_stuck() {
                            tracing::error!("PTT acceso oltre la scadenza: lo spengo");
                            on_trip();
                            thread_shared.trips.fetch_add(1, Ordering::SeqCst);
                        }
                        ok = thread_tracker.retry_once(&*ptt).is_ok();
                    } else {
                        // spento (anche da solo, a radio riconnessa): niente piu' stato «non spento»
                        thread_tracker.confirm_off();
                    }
                    st = lock(&thread_shared.state);
                    if st.deadline == Some(deadline) {
                        st.deadline = if ok { None } else { Some(Instant::now() + thread_tracker.timing().long_step) };
                    }
                }
            })
            .map_err(|e| TxError::Thread(e.to_string()))?;
        Ok(Watchdog { shared, tracker, thread: Some(thread) })
    }

    /// Il PTT deve essere spento entro `deadline`.
    pub fn arm(&self, deadline: Instant) {
        lock(&self.shared.state).deadline = Some(deadline);
        self.shared.changed.notify_all();
    }

    /// Il PTT e' spento: nessuna scadenza.
    pub fn disarm(&self) {
        lock(&self.shared.state).deadline = None;
        self.shared.changed.notify_all();
    }

    /// Lo spegnimento e' fallito: il controllo ci riprova fra poco.
    pub fn retry_soon(&self) {
        lock(&self.shared.state).deadline = Some(Instant::now() + self.tracker.timing().long_step);
        self.shared.changed.notify_all();
    }

    /// Un tentativo adesso (il pulsante «Riprova a spegnere» e STOP TX), se un controllo e' armato.
    pub fn kick(&self) {
        let mut st = lock(&self.shared.state);
        if st.deadline.is_some() {
            st.deadline = Some(Instant::now());
            drop(st);
            self.shared.changed.notify_all();
        }
    }

    /// Quante volte e' scattato.
    pub fn trips(&self) -> u64 {
        self.shared.trips.load(Ordering::SeqCst)
    }
}

impl Drop for Watchdog {
    fn drop(&mut self) {
        lock(&self.shared.state).quit = true;
        self.shared.changed.notify_all();
        if let Some(t) = self.thread.take() {
            let _ = t.join();
        }
    }
}
