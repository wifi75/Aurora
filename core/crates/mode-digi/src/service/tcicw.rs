//! Il CW dal comando CWX della radio via TCI (Flex, ExpertSDR: `cw_macros`). Con l'uscita «TCI» macro e finestra di trasmissione mandano il
//! testo alla radio invece di farne audio: la radio genera la CW e comanda PTT e tasto. Il TCI non dice quando un carattere esce, quindi
//! Aurora tiene il testo nella sua coda (`Session`) e ne passa alla radio pochi caratteri alla volta, sempre un poco in anticipo sul
//! tempo vero calcolato dalla velocita': cosi' Backspace e correzioni valgono fino a poco prima che escano, e l'eco (`tx_sent`) e
//! l'avanzamento seguono lo stesso tempo. Esc e la macro che riparte mandano `cw_macros_stop`.

use std::{
    collections::VecDeque,
    sync::{Arc, atomic::Ordering},
    time::{Duration, Instant},
};

use aurora_core::DigiEvent;

use super::{CwOutput, DigiError, DigiMode, DigiService, DigiSettings, DigiTxPort, Inner, Msg, TxStatus, live::Session, lock};
use crate::cw::unit_seconds;
use crate::tx_stream::TxModulator;

/// Ogni quanto il thread guarda il tempo.
const STEP: Duration = Duration::from_millis(20);
/// Quanti caratteri al massimo in un comando.
const CHUNK: usize = 6;
/// Di quanto il testo in radio puo' stare davanti al tempo vero: oltre, si aspetta (e Backspace funziona ancora sul resto).
const LEAD: Duration = Duration::from_millis(1500);

pub(super) fn selected(s: &DigiSettings) -> bool {
    s.mode == DigiMode::Cw && s.cw_output == CwOutput::Tci
}

/// Quanto dura in CW un pezzo di testo (stima: in media nove unita' per lettera con lo spazio dopo, sette per la pausa fra le parole).
pub(super) fn duration_of(piece: &str, wpm: f32) -> Duration {
    let unit = unit_seconds(wpm);
    let units: f64 = piece.chars().map(|c| if c == ' ' { 7.0 } else { 9.0 }).sum();
    Duration::from_secs_f64(units * unit)
}

impl DigiService {
    /// Il CW esce dal CWX della radio via TCI.
    pub(super) fn tcicw_selected(&self) -> bool {
        selected(&lock(&self.inner.settings))
    }

    /// Una trasmissione con il CWX della radio, o il testo accodato a quella in corso.
    pub(super) fn tcicw_session(&self, text: &str, open: bool) -> Result<TxStatus, DigiError> {
        let port = lock(&self.inner.port).clone().ok_or(DigiError::NoTransmitter)?;
        {
            let mut session = lock(&self.inner.session);
            if session.as_ref().is_some_and(|s| s.ended) {
                return Err(DigiError::Busy);
            }
            if let Some(s) = session.as_mut() {
                s.pending.extend(text.chars());
                s.open |= open;
                drop(session);
                if open {
                    lock(&self.inner.tx).open = true;
                    self.inner.tx_event();
                }
                return Ok(lock(&self.inner.tx).clone());
            }
            if self.inner.tx_active.load(Ordering::SeqCst) {
                return Err(DigiError::Busy);
            }
            *session = Some(Session::new(text, open));
        }
        let settings = self.settings();
        self.inner.tx_active.store(true, Ordering::SeqCst);
        self.inner.tx_abort.store(false, Ordering::SeqCst);
        let _ = self.inner.queue.send(Msg::Reset);
        tracing::info!("CW: trasmissione con il CWX della radio (TCI){}", if open { " (aperta)" } else { "" });
        *lock(&self.inner.tx) = TxStatus { state: "sending".into(), open, ..TxStatus::default() };
        self.inner.tx_event();
        let inner = Arc::clone(&self.inner);
        let tokens = TxModulator::cw(settings.tx_freq(), settings.cw_wpm);
        let wpm = settings.cw_wpm;
        let spawned = std::thread::Builder::new().name("aurora-tci-cw".into()).spawn(move || run(&inner, &port, &tokens, wpm));
        if let Err(e) = spawned {
            *lock(&self.inner.session) = None;
            self.inner.tx_active.store(false, Ordering::SeqCst);
            return Err(DigiError::Tx(e.to_string()));
        }
        Ok(lock(&self.inner.tx).clone())
    }

    /// Una macro che riparte: la CW in corso si ferma (`cw_macros_stop`) e il testo nuovo parte da capo.
    pub(super) fn tcicw_restart(&self, text: &str, open: bool) -> Result<(TxStatus, usize), DigiError> {
        if self.inner.tx_active.load(Ordering::SeqCst) {
            self.abort_tx();
            self.wait_idle(Duration::from_secs(5));
        }
        self.tcicw_session(text, open).map(|st| (st, 0))
    }
}

/// Il thread della trasmissione con il CWX: passa il testo alla radio un poco in anticipo sul tempo vero, dice quando e' uscito e chiude.
fn run(inner: &Arc<Inner>, port: &Arc<dyn DigiTxPort>, tokens: &TxModulator, wpm: f32) {
    let started = Instant::now();
    let mut last_emit = Instant::now();
    let mut sent_chars = 0usize;
    let mut closing = false;
    let mut failure: Option<String> = None;
    // quando la radio ha finito tutto quello che le e' stato mandato, e i pezzi di testo con l'ora in cui escono
    let mut radio_free = Instant::now();
    let mut marks: VecDeque<(Instant, String)> = VecDeque::new();
    let state = loop {
        std::thread::sleep(STEP);
        if inner.tx_abort.load(Ordering::SeqCst) {
            port.cw_text_stop();
            break "aborted";
        }
        let now = Instant::now();
        // l'eco: i pezzi di testo che hanno finito di uscire
        let mut out = String::new();
        while marks.front().is_some_and(|(at, _)| *at <= now) {
            if let Some((_, t)) = marks.pop_front() {
                out.push_str(&t);
            }
        }
        if !out.is_empty() {
            sent_chars += out.chars().count();
            let total = lock(&inner.session).as_ref().map_or(sent_chars, Session::total).max(1);
            inner.emit(DigiEvent { kind: "tx_sent".into(), text: Some(out), mode: Some(DigiMode::Cw.id().into()), progress: Some((sent_chars as f32 / total as f32).min(1.0)), ..DigiEvent::default() });
        }
        // altro testo alla radio, ma non troppo avanti
        if !closing && radio_free <= now + LEAD {
            let piece = next_piece(inner, tokens, &mut closing);
            if !piece.is_empty() {
                if let Err(e) = port.cw_text(&piece, wpm.round().clamp(5.0, 99.0) as u32) {
                    failure = Some(e);
                    break "error";
                }
                radio_free = radio_free.max(now) + duration_of(&piece, wpm);
                marks.push_back((radio_free, piece));
            }
        }
        if closing && marks.is_empty() && now >= radio_free {
            break "done";
        }
        {
            let mut tx = lock(&inner.tx);
            tx.elapsed_s = started.elapsed().as_secs_f32();
            let total = lock(&inner.session).as_ref().map_or(0, Session::total);
            tx.progress = if total > 0 { (sent_chars as f32 / total as f32).min(1.0) } else { 0.0 };
        }
        if last_emit.elapsed() >= Duration::from_millis(500) {
            last_emit = Instant::now();
            inner.tx_event();
        }
    };
    *lock(&inner.session) = None;
    {
        let mut tx = lock(&inner.tx);
        tx.state = state.into();
        tx.open = false;
        if state == "done" {
            tx.progress = 1.0;
        }
        if let Some(e) = &failure {
            tx.message = Some(format!("TCI: {e}"));
        }
    }
    inner.tx_active.store(false, Ordering::SeqCst);
    inner.tx_abort.store(false, Ordering::SeqCst);
    tracing::info!("CW: trasmissione con il CWX della radio finita ({state})");
    inner.tx_event();
}

/// Il prossimo pezzo di testo in coda (al massimo [`CHUNK`] caratteri, un segno di procedura intero); vuoto se non c'e' niente da mandare.
/// `closing` diventa vero quando il testo e' finito e la trasmissione non e' aperta.
fn next_piece(inner: &Inner, tokens: &TxModulator, closing: &mut bool) -> String {
    let mut session = lock(&inner.session);
    let Some(s) = session.as_mut() else {
        *closing = true;
        return String::new();
    };
    let mut piece = String::new();
    while piece.chars().count() < CHUNK {
        let head: Vec<char> = s.pending.iter().take(4).copied().collect();
        if head.is_empty() {
            if !s.open {
                s.ended = true;
                *closing = true;
            }
            break;
        }
        // un segno di procedura a meta' («<S» di «<SK>») aspetta il resto
        let Some(n) = tokens.next_token(&head, s.open) else { break };
        let n = n.min(head.len());
        piece.extend(s.pending.drain(..n));
        s.taken += n;
    }
    piece
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use super::*;

    /// Una radio finta con il comando CW del TCI: registra i testi e gli stop.
    #[derive(Default)]
    struct FakeRadio {
        texts: Mutex<Vec<(String, u32)>>,
        stops: Mutex<u32>,
    }

    impl DigiTxPort for Arc<FakeRadio> {
        fn start(&self, _wave: Vec<f32>) -> Result<(), String> {
            Err("solo CW dal TCI".into())
        }
        fn halt(&self) {}
        fn is_busy(&self) -> bool {
            false
        }
        fn cw_text(&self, text: &str, wpm: u32) -> Result<(), String> {
            lock(&self.texts).push((text.to_string(), wpm));
            Ok(())
        }
        fn cw_text_stop(&self) {
            *lock(&self.stops) += 1;
        }
    }

    fn service(tag: &str) -> (DigiService, Arc<FakeRadio>, Arc<Mutex<Vec<DigiEvent>>>, std::path::PathBuf) {
        let dir = std::env::temp_dir().join(format!("aurora-tcicw-{tag}-{}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);
        let svc = DigiService::open(&dir);
        let radio = Arc::new(FakeRadio::default());
        svc.set_tx_port(Arc::new(Arc::clone(&radio)));
        let events = Arc::new(Mutex::new(Vec::new()));
        let sink = Arc::clone(&events);
        svc.set_emitter(Arc::new(move |e| lock(&sink).push(e)));
        let mut st = svc.settings();
        st.mode = DigiMode::Cw;
        st.cw_output = CwOutput::Tci;
        st.cw_wpm = 40.0;
        svc.apply_settings(st).expect("impostazioni");
        (svc, radio, events, dir)
    }

    fn wait_state(svc: &DigiService, want: &str) -> bool {
        let t0 = Instant::now();
        while t0.elapsed() < Duration::from_secs(10) {
            if svc.snapshot().tx.state == want {
                return true;
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        false
    }

    #[test]
    fn la_macro_va_al_cwx_della_radio_e_l_eco_segue_il_tempo() {
        let (svc, radio, events, dir) = service("macro");
        svc.transmit("CQ DE IU3ABC <SK>").expect("trasmissione");
        assert!(wait_state(&svc, "done"), "stato: {:?}", svc.snapshot().tx);
        let texts = lock(&radio.texts).clone();
        let all: String = texts.iter().map(|(t, _)| t.as_str()).collect();
        assert_eq!(all, "CQ DE IU3ABC <SK>");
        assert!(texts.iter().all(|(t, w)| *w == 40 && t.chars().count() <= CHUNK + 3), "{texts:?}");
        let echo: String = lock(&events).iter().filter(|e| e.kind == "tx_sent").filter_map(|e| e.text.clone()).collect();
        assert_eq!(echo, "CQ DE IU3ABC <SK>");
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn esc_ferma_la_cw_della_radio() {
        let (svc, radio, _events, dir) = service("esc");
        svc.tx_open("CQ CQ CQ CQ CQ CQ").expect("trasmissione");
        std::thread::sleep(Duration::from_millis(200));
        svc.abort_tx();
        assert!(wait_state(&svc, "aborted"), "stato: {:?}", svc.snapshot().tx);
        assert_eq!(*lock(&radio.stops), 1);
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn la_durata_stimata_segue_la_velocita() {
        let lento = duration_of("CQ CQ", 10.0);
        let veloce = duration_of("CQ CQ", 20.0);
        assert!(lento > veloce);
        assert!((lento.as_secs_f64() / veloce.as_secs_f64() - 2.0).abs() < 0.01);
        // cinque caratteri a 20 wpm: quattro lettere da 9 unita' e una pausa da 7, unita' di 60 ms
        assert!((duration_of("CQ CQ", 20.0).as_secs_f64() - (4.0 * 9.0 + 7.0) * 0.06).abs() < 1e-9);
    }
}
