//! La trasmissione a flusso dei modi testuali.
//!
//! - **Aperta** (levetta RX/TX, tasto Pausa): si va in trasmissione e ci si resta; senza testo l'RTTY manda LTRS, il PSK le inversioni,
//!   il CW tace. Il testo scritto a mano parte mentre si scrive ([`DigiService::tx_type`]); i caratteri non ancora modulati si possono
//!   togliere ([`DigiService::tx_unsend`], il Backspace). Chiusa ([`DigiService::tx_close`]) finisce il testo gia' scritto e torna in
//!   ricezione.
//! - **A blocco** (macro, Ctrl+Invio): lo stesso flusso con il testo intero, che si chiude da solo alla fine.
//!
//! L'audio si fa un carattere alla volta (`tx_stream`) e si tiene avanti di circa 0,3 s rispetto a quello gia' uscito dalla scheda: cosi'
//! il testo resta modificabile fino all'ultimo e l'evento `tx_sent` dice quali caratteri sono partiti davvero (contati sui campioni
//! passati al dispositivo, non su quelli generati). La finestra di trasmissione li segna, quella di ricezione ne mostra l'eco.

use std::{
    collections::VecDeque,
    sync::{Arc, atomic::Ordering},
    time::{Duration, Instant},
};

use aurora_core::DigiEvent;

use super::{DigiError, DigiMode, DigiService, DigiSettings, DigiTxPort, Inner, Msg, TxStatus, lock};
use crate::{RATE, tx_stream::TxModulator};

/// Quanto audio (campioni a 12 kHz) si tiene pronto oltre quello gia' uscito: 0,3 s.
const AHEAD: u64 = 3600;
/// Ogni quanto il thread della trasmissione guarda a che punto e'.
const STEP: Duration = Duration::from_millis(20);

/// Il testo di una trasmissione a flusso ancora da modulare.
pub(super) struct Session {
    pub(super) pending: VecDeque<char>,
    pub(super) open: bool,
    /// Caratteri gia' passati al modulatore: non si possono piu' togliere ne' cambiare.
    pub(super) taken: usize,
    /// Il testo e' finito e la coda di chiusura sta partendo: niente si accoda piu' (andrebbe perso), serve una trasmissione nuova.
    pub(super) ended: bool,
}

impl Session {
    pub(super) fn new(text: &str, open: bool) -> Session {
        Session { pending: text.chars().collect(), open, taken: 0, ended: false }
    }

    /// I caratteri della trasmissione in tutto (modulati e in coda): le posizioni di [`DigiService::tx_replace`] contano da qui.
    pub(super) fn total(&self) -> usize {
        self.taken + self.pending.len()
    }
}

/// Il modulatore a flusso per le impostazioni attuali; `lsb`: la radio e' in LSB (RTTY: mark e space invertiti). Olivia e Contestia con
/// toni e banda che non stanno nella banda audio danno `BadValue`.
pub(super) fn modulator(s: &DigiSettings, lsb: bool) -> Result<TxModulator, DigiError> {
    if let Some((variant, o)) = s.olivia_settings() {
        return TxModulator::olivia(o.params(variant, s.tx_freq())).ok_or_else(|| {
            DigiError::BadValue(format!("{}: {} toni in {} Hz centrati a {:.0} Hz escono dalla banda audio", s.mode.name(), o.tones, o.bandwidth_hz, s.tx_freq()))
        });
    }
    Ok(match (s.mode.psk(), s.mode) {
        (Some(mode), _) => TxModulator::psk_reverse(mode, s.tx_freq(), lsb),
        (None, DigiMode::Cw) => TxModulator::cw(s.tx_freq(), s.cw_wpm),
        (None, _) => TxModulator::rtty(s.rtty(s.tx_freq(), lsb), s.rtty_charset).with_diddle(s.rtty_diddle),
    })
}

impl DigiService {
    /// Apre la trasmissione (levetta): si resta in trasmissione finche' non si chiude con [`DigiService::tx_close`]; `text` (anche
    /// vuoto) e' il testo gia' scritto. Con una trasmissione in corso la rende aperta e ci accoda il testo.
    pub fn tx_open(&self, text: &str) -> Result<TxStatus, DigiError> {
        self.text_mode_or_err()?;
        if self.keyer_selected() {
            return self.keyer_session(text, true);
        }
        if self.tcicw_selected() {
            return self.tcicw_session(text, true);
        }
        let port = lock(&self.inner.port).clone().ok_or(DigiError::NoTransmitter)?;
        if !port.can_stream() {
            return Err(DigiError::Tx("questa catena di trasmissione non trasmette a flusso".into()));
        }
        self.start_session(port, text, true)
    }

    /// Testo scritto durante la trasmissione: si accoda a quello che sta partendo.
    pub fn tx_type(&self, text: &str) -> Result<TxStatus, DigiError> {
        {
            let mut session = lock(&self.inner.session);
            let Some(s) = session.as_mut().filter(|s| !s.ended) else { return Err(DigiError::BadValue("nessuna trasmissione in corso".into())) };
            s.pending.extend(text.chars());
        }
        Ok(lock(&self.inner.tx).clone())
    }

    /// Toglie fino a `count` caratteri dalla fine del testo non ancora modulato (Backspace); restituisce quanti ne ha tolti (meno di
    /// `count` se gli altri stanno gia' partendo).
    pub fn tx_unsend(&self, count: usize) -> usize {
        let mut session = lock(&self.inner.session);
        let Some(s) = session.as_mut() else { return 0 };
        let n = count.min(s.pending.len());
        for _ in 0..n {
            s.pending.pop_back();
        }
        n
    }

    /// Sostituisce il testo in coda dalla posizione `from` (contata dal primo carattere della trasmissione) con `text`, senza fermare la
    /// trasmissione: si corregge, si cancella o si aggiunge in qualunque punto della parte non ancora modulata. `Ok(false)` se `from`
    /// cade su caratteri gia' passati al modulatore (non si cambiano piu': circa 0,3 s prima che escano); `Err` senza trasmissione.
    pub fn tx_replace(&self, from: usize, text: &str) -> Result<bool, DigiError> {
        let mut session = lock(&self.inner.session);
        let Some(s) = session.as_mut() else { return Err(DigiError::BadValue("nessuna trasmissione in corso".into())) };
        if from < s.taken {
            return Ok(false);
        }
        let keep = (from - s.taken).min(s.pending.len());
        s.pending.truncate(keep);
        s.pending.extend(text.chars());
        Ok(true)
    }

    /// Chiude la trasmissione aperta: finito il testo gia' scritto si torna in ricezione.
    pub fn tx_close(&self) {
        if let Some(s) = lock(&self.inner.session).as_mut() {
            s.open = false;
        }
        lock(&self.inner.tx).open = false;
        self.inner.tx_event();
    }

    /// Una macro premuta mentre si trasmette ricomincia da capo: il testo non ancora modulato (della macro prima e di quelle in coda)
    /// si butta, `text` parte subito dopo l'ultimo carattere gia' modulato senza togliere il PTT, e `open` dice se finito il testo si
    /// resta in trasmissione (macro senza `<RX>`). Senza trasmissione ne parte una nuova; se quella di prima stava chiudendo si aspetta
    /// che finisca (al massimo 5 s). Restituisce lo stato e quanti caratteri della trasmissione restano (quelli gia' modulati).
    pub fn transmit_restart(&self, text: &str, open: bool) -> Result<(TxStatus, usize), DigiError> {
        if text.trim().is_empty() {
            return Err(DigiError::Empty);
        }
        self.text_mode_or_err()?;
        if self.keyer_selected() {
            return self.keyer_restart(text, open);
        }
        if self.tcicw_selected() {
            return self.tcicw_restart(text, open);
        }
        let port = lock(&self.inner.port).clone().ok_or(DigiError::NoTransmitter)?;
        if port.can_stream() {
            {
                let mut session = lock(&self.inner.session);
                if let Some(s) = session.as_mut().filter(|s| !s.ended) {
                    s.pending.clear();
                    s.pending.extend(text.chars());
                    s.open = open;
                    let kept = s.taken;
                    drop(session);
                    lock(&self.inner.tx).open = open;
                    self.inner.tx_event();
                    return Ok((lock(&self.inner.tx).clone(), kept));
                }
            }
            self.wait_idle(Duration::from_secs(5));
            return self.start_session(port, text, open).map(|st| (st, 0));
        }
        // senza flusso: si ferma quella in corso e si riparte
        if self.inner.tx_active.load(Ordering::SeqCst) {
            self.abort_tx();
            self.wait_idle(Duration::from_secs(5));
        }
        self.transmit(text).map(|st| (st, 0))
    }

    /// Aspetta (al massimo `limit`) che la trasmissione in corso sia finita del tutto.
    pub(super) fn wait_idle(&self, limit: Duration) {
        let t0 = Instant::now();
        while self.inner.tx_active.load(Ordering::SeqCst) && t0.elapsed() < limit {
            std::thread::sleep(STEP);
        }
    }

    /// Una trasmissione a flusso nuova, o il testo accodato a quella in corso.
    pub(super) fn start_session(&self, port: Arc<dyn DigiTxPort>, text: &str, open: bool) -> Result<TxStatus, DigiError> {
        {
            let mut session = lock(&self.inner.session);
            if session.as_ref().is_some_and(|s| s.ended) {
                // la trasmissione sta chiudendo: il testo andrebbe perso, si ritenta quando e' finita
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
            if self.inner.tx_active.load(Ordering::SeqCst) || port.is_busy() {
                return Err(DigiError::Busy);
            }
            *session = Some(Session { pending: text.chars().collect(), open, taken: 0, ended: false });
        }
        let settings = self.settings();
        let lsb = self.inner.read_radio_lsb();
        let mut m = match modulator(&settings, lsb) {
            Ok(m) => m,
            Err(e) => {
                *lock(&self.inner.session) = None;
                return Err(e);
            }
        };
        let first = m.preamble();
        let first_len = first.len() as u64;
        self.inner.tx_active.store(true, Ordering::SeqCst);
        self.inner.tx_abort.store(false, Ordering::SeqCst);
        // la propria trasmissione non si decodifica
        let _ = self.inner.queue.send(Msg::Reset);
        if let Err(e) = port.start_stream(first) {
            *lock(&self.inner.session) = None;
            self.inner.tx_active.store(false, Ordering::SeqCst);
            return Err(DigiError::Tx(e));
        }
        tracing::info!("{}: trasmissione {}", settings.mode.name(), if open { "aperta" } else { "a flusso" });
        *lock(&self.inner.tx) = TxStatus { state: "sending".into(), open, ..TxStatus::default() };
        self.inner.tx_event();
        let inner = Arc::clone(&self.inner);
        let mode = settings.mode;
        let worker_port = Arc::clone(&port);
        let spawned = std::thread::Builder::new().name("aurora-digi-tx".into()).spawn(move || run(&inner, &worker_port, m, first_len, mode));
        if let Err(e) = spawned {
            tracing::error!("modi testuali: trasmissione a flusso non avviata: {e}");
            port.halt();
            *lock(&self.inner.session) = None;
            self.inner.tx_active.store(false, Ordering::SeqCst);
            return Err(DigiError::Tx(e.to_string()));
        }
        Ok(lock(&self.inner.tx).clone())
    }
}

/// Il prossimo pezzo da modulare.
enum Next {
    Text(String),
    Idle,
    End,
}

fn next(inner: &Inner, m: &TxModulator) -> Next {
    let mut session = lock(&inner.session);
    let Some(s) = session.as_mut() else { return Next::End };
    let head: Vec<char> = s.pending.iter().take(4).copied().collect();
    if head.is_empty() {
        if s.open {
            return Next::Idle;
        }
        s.ended = true;
        return Next::End;
    }
    match m.next_token(&head, s.open) {
        Some(n) => {
            let n = n.min(head.len());
            s.taken += n;
            Next::Text(s.pending.drain(..n).collect())
        }
        // in CW un segno di procedura a meta' («<S» di «<SK>»): si aspetta il resto
        None => Next::Idle,
    }
}

/// Il thread della trasmissione a flusso: tiene l'audio avanti di [`AHEAD`], dice quali caratteri sono partiti e chiude alla fine.
fn run(inner: &Arc<Inner>, port: &Arc<dyn DigiTxPort>, mut m: TxModulator, mut generated: u64, mode: DigiMode) {
    let started = Instant::now();
    let mut last_emit = Instant::now();
    // dove finisce l'audio di ogni pezzo di testo (campioni a 12 kHz dall'inizio)
    let mut marks: VecDeque<(u64, String)> = VecDeque::new();
    // i pezzi di testo che non hanno dato ancora audio (Olivia raccoglie un blocco intero, un carattere che il modo non ha): partono con il
    // prossimo audio, non prima
    let mut held = String::new();
    let mut closing = false;
    let mut sent_chars = 0usize;
    let sent_event = |text: String, sent: usize| {
        let total = lock(&inner.session).as_ref().map_or(sent, Session::total).max(1);
        inner.emit(DigiEvent { kind: "tx_sent".into(), text: Some(text), mode: Some(mode.id().into()), progress: Some((sent as f32 / total as f32).min(1.0)), ..DigiEvent::default() });
    };
    let state = loop {
        std::thread::sleep(STEP);
        let elapsed = started.elapsed().as_secs_f32();
        if inner.tx_abort.load(Ordering::SeqCst) {
            break "aborted";
        }
        // senza il conto della scheda: il tempo passato, meno l'anticipo del PTT
        let played = port.played().unwrap_or_else(|| (((elapsed - 0.2).max(0.0)) * RATE) as u64);
        let mut sent = String::new();
        while marks.front().is_some_and(|(end, _)| *end <= played) {
            if let Some((_, t)) = marks.pop_front() {
                sent.push_str(&t);
            }
        }
        if !sent.is_empty() {
            sent_chars += sent.chars().count();
            sent_event(sent, sent_chars);
        }
        if !closing {
            let mut failed = None;
            while generated < played + AHEAD {
                let (wave, piece) = match next(inner, &m) {
                    Next::Text(t) => (m.token(&t), Some(t)),
                    Next::Idle => (m.idle(), None),
                    Next::End => {
                        closing = true;
                        (m.postamble(), None)
                    }
                };
                let len = wave.len() as u64;
                if !wave.is_empty()
                    && let Err(e) = port.append(wave)
                {
                    failed = Some(e);
                    break;
                }
                generated += len;
                if let Some(t) = piece {
                    held.push_str(&t);
                }
                if len > 0 && !held.is_empty() {
                    marks.push_back((generated, std::mem::take(&mut held)));
                }
                if closing {
                    port.close_stream();
                    break;
                }
            }
            if let Some(e) = failed {
                tracing::warn!("modi testuali: trasmissione a flusso interrotta: {e}");
                break "error";
            }
        }
        // la catena puo' impiegare qualche centinaio di ms per partire (PTT)
        if elapsed > 1.5 && !port.is_busy() {
            // la catena si e' fermata senza un errore di scrittura: qualcuno l'ha fermata (STOP, Esc, un'altra trasmissione), non e' un guasto
            break if closing { "done" } else { "aborted" };
        }
        {
            let mut tx = lock(&inner.tx);
            tx.elapsed_s = elapsed;
            tx.total_s = generated as f32 / RATE;
            let total = lock(&inner.session).as_ref().map_or(0, Session::total);
            tx.progress = if total > 0 { (sent_chars as f32 / total as f32).min(1.0) } else { 0.0 };
        }
        if last_emit.elapsed() >= Duration::from_millis(500) {
            last_emit = Instant::now();
            inner.tx_event();
        }
    };
    // finita bene: i caratteri ancora in coda sono partiti con l'ultimo audio
    if state == "done" {
        let mut rest: String = marks.drain(..).map(|(_, t)| t).collect();
        rest.push_str(&held);
        if !rest.is_empty() {
            sent_chars += rest.chars().count();
            sent_event(rest, sent_chars);
        }
    }
    *lock(&inner.session) = None;
    {
        let mut tx = lock(&inner.tx);
        tx.state = state.into();
        tx.open = false;
        if state == "done" {
            tx.progress = 1.0;
        }
        if state == "error" {
            tx.message = Some("trasmissione interrotta dalla catena di trasmissione (PTT, audio o un'altra trasmissione)".into());
        }
    }
    inner.tx_active.store(false, Ordering::SeqCst);
    inner.tx_abort.store(false, Ordering::SeqCst);
    tracing::info!("modi testuali: trasmissione a flusso finita ({state})");
    inner.tx_event();
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;
    use std::sync::atomic::AtomicBool;

    /// Una catena finta che «suona» in tempo vero: l'onda accodata esce a 12 kHz dall'avvio.
    #[derive(Default)]
    struct FakePort {
        wave: Mutex<Vec<f32>>,
        started: Mutex<Option<Instant>>,
        closed: AtomicBool,
        halted: AtomicBool,
        /// Quante volte piu' veloce del tempo vero «suona» (0 = tempo vero): le prove di Olivia, lento, non aspettano minuti.
        speed: std::sync::atomic::AtomicU32,
    }

    impl FakePort {
        fn played(&self) -> u64 {
            let len = lock(&self.wave).len() as u64;
            let speed = f64::from(self.speed.load(Ordering::SeqCst).max(1));
            lock(&self.started).map_or(0, |t| ((t.elapsed().as_secs_f64() * f64::from(RATE) * speed) as u64).min(len))
        }
    }

    impl DigiTxPort for Arc<FakePort> {
        fn start(&self, _wave: Vec<f32>) -> Result<(), String> {
            Err("solo a flusso".into())
        }
        fn halt(&self) {
            self.halted.store(true, Ordering::SeqCst);
        }
        fn is_busy(&self) -> bool {
            lock(&self.started).is_some() && !self.halted.load(Ordering::SeqCst) && !(self.closed.load(Ordering::SeqCst) && FakePort::played(self) >= lock(&self.wave).len() as u64)
        }
        fn can_stream(&self) -> bool {
            true
        }
        fn start_stream(&self, first: Vec<f32>) -> Result<(), String> {
            *lock(&self.wave) = first;
            *lock(&self.started) = Some(Instant::now());
            Ok(())
        }
        fn append(&self, wave: Vec<f32>) -> Result<(), String> {
            lock(&self.wave).extend(wave);
            Ok(())
        }
        fn close_stream(&self) {
            self.closed.store(true, Ordering::SeqCst);
        }
        fn played(&self) -> Option<u64> {
            Some(FakePort::played(self))
        }
    }

    fn service(mode: DigiMode) -> (DigiService, Arc<FakePort>, Arc<Mutex<Vec<DigiEvent>>>, tempdir::Dir) {
        let dir = tempdir::Dir::new();
        let svc = DigiService::open(dir.path());
        let mut s = svc.settings();
        s.mode = mode;
        if mode == DigiMode::Cw {
            s.cw_wpm = 40.0;
        }
        svc.apply_settings(s).expect("impostazioni");
        let port = Arc::new(FakePort::default());
        svc.set_tx_port(Arc::new(Arc::clone(&port)));
        let events = Arc::new(Mutex::new(Vec::new()));
        let sink = Arc::clone(&events);
        svc.set_emitter(Arc::new(move |e| lock(&sink).push(e)));
        (svc, port, events, dir)
    }

    /// Una cartella temporanea per `digi.json`, cancellata alla fine.
    mod tempdir {
        pub struct Dir(std::path::PathBuf);
        impl Dir {
            pub fn new() -> Dir {
                static N: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
                let n = N.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                let p = std::env::temp_dir().join(format!("aurora-digi-live-{}-{n}", std::process::id()));
                let _ = std::fs::create_dir_all(&p);
                Dir(p)
            }
            pub fn path(&self) -> &std::path::Path {
                &self.0
            }
        }
        impl Drop for Dir {
            fn drop(&mut self) {
                let _ = std::fs::remove_dir_all(&self.0);
            }
        }
    }

    fn sent(events: &Mutex<Vec<DigiEvent>>) -> String {
        lock(events).iter().filter(|e| e.kind == "tx_sent").filter_map(|e| e.text.clone()).collect()
    }

    fn wait_state(svc: &DigiService, state: &str, secs: u64) -> bool {
        let limit = Instant::now() + Duration::from_secs(secs);
        while Instant::now() < limit {
            if svc.snapshot().tx.state == state {
                return true;
            }
            std::thread::sleep(Duration::from_millis(20));
        }
        false
    }

    #[test]
    fn la_trasmissione_aperta_resta_accesa_e_manda_quello_che_si_scrive() {
        let (svc, port, events, _dir) = service(DigiMode::Psk63);
        let st = svc.tx_open("").expect("aperta");
        assert!(st.open && st.state == "sending");
        // senza testo resta in trasmissione (inversioni di fase)
        std::thread::sleep(Duration::from_millis(600));
        assert_eq!(svc.snapshot().tx.state, "sending");
        assert!(sent(&events).is_empty());
        svc.tx_type("CQ DE IU3ABC").expect("scritto");
        std::thread::sleep(Duration::from_millis(100));
        // il Backspace toglie solo quello che non e' ancora modulato
        svc.tx_type("XYZ").expect("scritto");
        assert_eq!(svc.tx_unsend(3), 3);
        svc.tx_close();
        assert!(wait_state(&svc, "done", 10), "stato: {:?}", svc.snapshot().tx);
        assert!(port.closed.load(Ordering::SeqCst));
        assert_eq!(sent(&events), "CQ DE IU3ABC");
        // l'audio si rilegge
        let mut d = super::super::Demod::new(&svc.settings(), false);
        let wave = lock(&port.wave).clone();
        let out = d.process(&wave).0;
        assert!(out.contains("CQ DE IU3ABC"), "ricevuto: {out:?}");
        assert!(!out.contains("XYZ"), "ricevuto: {out:?}");
    }

    #[test]
    fn i_caratteri_risultano_partiti_al_ritmo_dell_audio() {
        let (svc, _port, events, _dir) = service(DigiMode::Rtty);
        let t0 = Instant::now();
        svc.transmit("RYRY CQ").expect("partita");
        // RTTY a 45,45 baud: 165 ms per carattere, 8 bit di mark e un LTRS prima. Dopo mezzo secondo non puo' essere partito tutto
        std::thread::sleep(Duration::from_millis(500));
        let early = sent(&events);
        assert!(early.len() < 4, "gia' partiti {early:?} dopo {:?}", t0.elapsed());
        assert!(wait_state(&svc, "done", 10));
        assert_eq!(sent(&events), "RYRY CQ");
        // un'altra macro mentre si trasmette si accoda
        svc.transmit("TU").expect("partita");
        svc.transmit(" 73").expect("accodata");
        assert!(wait_state(&svc, "done", 10));
        assert_eq!(sent(&events), "RYRY CQTU 73");
    }

    #[test]
    fn la_macro_ripremuta_ricomincia_da_capo_senza_togliere_il_ptt() {
        let (svc, port, events, _dir) = service(DigiMode::Rtty);
        svc.transmit_restart("CQ TEST IU3ABC IU3ABC", false).expect("partita");
        std::thread::sleep(Duration::from_millis(900));
        // F2 di nuovo, due volte: ogni volta il resto non ancora modulato si butta e la macro riparte dall'inizio
        let (_, kept1) = svc.transmit_restart("CQ TEST IU3ABC IU3ABC", false).expect("ripartita");
        std::thread::sleep(Duration::from_millis(400));
        let (_, kept2) = svc.transmit_restart("CQ TEST IU3ABC IU3ABC", false).expect("ripartita");
        assert!(kept2 >= kept1);
        assert_eq!(svc.snapshot().tx.state, "sending");
        assert!(wait_state(&svc, "done", 15), "stato: {:?}", svc.snapshot().tx);
        let out = sent(&events);
        // l'ultima pressione esce intera e per ultima; delle prime solo i caratteri gia' modulati
        assert!(out.ends_with("CQ TEST IU3ABC IU3ABC"), "partito: {out:?}");
        assert!(out.chars().count() < 3 * 21, "partito: {out:?}");
        assert!(port.closed.load(Ordering::SeqCst));
        // una macro diversa sostituisce il resto della prima
        svc.transmit_restart("CQ TEST IU3ABC IU3ABC", false).expect("partita");
        std::thread::sleep(Duration::from_millis(700));
        svc.transmit_restart("TU 73", false).expect("ripartita");
        assert!(wait_state(&svc, "done", 15));
        let out = sent(&events);
        assert!(out.ends_with("TU 73"), "partito: {out:?}");
        assert!(!out.ends_with("IU3ABCTU 73"), "partito: {out:?}");
    }

    #[test]
    fn niente_si_accoda_alla_trasmissione_che_sta_chiudendo() {
        let (svc, _port, _events, _dir) = service(DigiMode::Rtty);
        svc.transmit("TU").expect("partita");
        // la coda e' finita e la chiusura sta partendo: prima il testo si perdeva, ora la richiesta e' rifiutata (si ritenta dopo)
        let deadline = Instant::now() + Duration::from_secs(5);
        while !lock(&svc.inner.session).as_ref().is_some_and(|s| s.ended) && Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(5));
        }
        if lock(&svc.inner.session).as_ref().is_some_and(|s| s.ended) {
            assert!(matches!(svc.transmit(" 73"), Err(DigiError::Busy)));
        }
        // la ripartenza invece aspetta la fine e trasmette di nuovo
        let (st, kept) = svc.transmit_restart("73", false).expect("ripartita");
        assert_eq!(st.state, "sending");
        assert_eq!(kept, 0);
        assert!(wait_state(&svc, "done", 10));
    }

    #[test]
    fn in_cw_il_segno_di_procedura_scritto_a_meta_aspetta_il_resto() {
        let (svc, port, events, _dir) = service(DigiMode::Cw);
        svc.tx_open("TU <S").expect("aperta");
        std::thread::sleep(Duration::from_millis(1500));
        svc.tx_type("K>").expect("scritto");
        svc.tx_close();
        assert!(wait_state(&svc, "done", 10));
        assert_eq!(sent(&events), "TU <SK>");
        assert!(lock(&port.wave).len() > 12_000);
    }

    #[test]
    fn la_coda_si_sostituisce_senza_fermare_la_trasmissione() {
        let (svc, port, events, _dir) = service(DigiMode::Rtty);
        svc.tx_open("CQ CQ DE IU3ABC IU3ABC PSE K").expect("aperta");
        std::thread::sleep(Duration::from_millis(1200));
        // l'inizio e' gia' passato al modulatore: da li' non si cambia piu'
        assert_eq!(svc.tx_replace(0, "XX").expect("in corso"), false);
        // la coda si cambia a meta': «IU3ABC PSE K» diventa «IU3ABC 599 K»
        assert!(svc.tx_replace(16, "IU3ABC 599 K").expect("in corso"));
        // e si corregge ancora piu' avanti, aggiungendo in fondo
        assert!(svc.tx_replace(28, " TU").expect("in corso"));
        assert_eq!(svc.snapshot().tx.state, "sending");
        svc.tx_close();
        assert!(wait_state(&svc, "done", 15), "stato: {:?}", svc.snapshot().tx);
        assert_eq!(sent(&events), "CQ CQ DE IU3ABC IU3ABC 599 K TU");
        let mut d = super::super::Demod::new(&svc.settings(), false);
        let mut wave = lock(&port.wave).clone();
        wave.extend(vec![0.0; 6000]);
        let out = d.process(&wave).0;
        assert!(out.contains("IU3ABC IU3ABC 599 K TU"), "ricevuto: {out:?}");
        assert!(svc.tx_replace(0, "X").is_err());
    }

    #[test]
    fn stop_ferma_la_trasmissione_aperta() {
        let (svc, port, _events, _dir) = service(DigiMode::Rtty);
        svc.tx_open("CQ CQ").expect("aperta");
        std::thread::sleep(Duration::from_millis(300));
        svc.abort_tx();
        assert!(wait_state(&svc, "aborted", 5));
        assert!(port.halted.load(Ordering::SeqCst));
        assert_eq!(svc.tx_unsend(1), 0);
        assert!(svc.tx_type("X").is_err());
    }

    #[test]
    fn in_ssb_non_parte_nessun_testo() {
        let (svc, port, _events, _dir) = service(DigiMode::Ssb);
        assert_eq!(svc.transmit("CQ TEST").unwrap_err(), DigiError::NoTextMode);
        assert_eq!(svc.tx_open("").unwrap_err(), DigiError::NoTextMode);
        assert_eq!(svc.transmit_restart("CQ", false).unwrap_err(), DigiError::NoTextMode);
        assert!(lock(&port.wave).is_empty(), "nessun audio verso la radio");
    }

    /// Olivia o Contestia con toni e banda dati, centrati a 1500 Hz, con la catena finta 8 volte piu' veloce del tempo vero.
    fn olivia_service(mode: DigiMode, tones: u32, bw: u32) -> (DigiService, Arc<FakePort>, Arc<Mutex<Vec<DigiEvent>>>, tempdir::Dir) {
        let (svc, port, events, dir) = service(mode);
        let mut s = svc.settings();
        let o = if mode == DigiMode::Olivia { &mut s.olivia } else { &mut s.contestia };
        o.tones = tones;
        o.bandwidth_hz = bw;
        s.rx_freq_hz = 1500.0;
        s.tx_freq_hz = 1500.0;
        svc.apply_settings(s).expect("impostazioni");
        port.speed.store(8, Ordering::SeqCst);
        (svc, port, events, dir)
    }

    /// Rumore gaussiano ripetibile (xorshift e Box-Muller).
    fn noise(seed: u64, n: usize, sigma: f32) -> Vec<f32> {
        let mut x = seed | 1;
        let mut uniform = move || {
            x ^= x << 13;
            x ^= x >> 7;
            x ^= x << 17;
            ((x >> 11) as f64 + 0.5) / (1u64 << 53) as f64
        };
        (0..n).map(|_| sigma * ((-2.0 * uniform().ln()).sqrt() * (2.0 * std::f64::consts::PI * uniform()).cos()) as f32).collect()
    }

    /// L'onda trasmessa con due secondi davanti, 25 dietro (il ricevitore integra piu' blocchi: gli ultimi escono dopo) e rumore moderato.
    fn with_noise(wave: &[f32], sigma: f32) -> Vec<f32> {
        let mut audio = vec![0.0; 24_000];
        audio.extend_from_slice(wave);
        audio.extend(vec![0.0; 12_000 * 25]);
        let n = noise(11, audio.len(), sigma);
        for (a, n) in audio.iter_mut().zip(n) {
            *a += n;
        }
        audio
    }

    fn received(events: &Mutex<Vec<DigiEvent>>) -> String {
        lock(events).iter().filter(|e| e.kind == "rx_text" && e.rx_id.is_none()).filter_map(|e| e.text.clone()).collect()
    }

    /// Anello chiuso nel servizio: il testo esce dal trasmettitore a flusso (8 kHz portati a 12 kHz), con il rumore entra nella ricezione
    /// del servizio (12 kHz portati a 8 kHz) e arriva intero nella finestra RX, senza caratteri spuri nel rumore prima e dopo.
    #[test]
    fn olivia_e_contestia_in_anello_chiuso_nel_servizio() {
        let text = "CQ CQ DE IU3ABC IU3ABC IO91WM K";
        for (mode, tones, bw) in [(DigiMode::Olivia, 8, 250), (DigiMode::Olivia, 32, 1000), (DigiMode::Contestia, 16, 500)] {
            let (svc, port, events, _dir) = olivia_service(mode, tones, bw);
            svc.transmit(text).expect("partita");
            assert!(wait_state(&svc, "done", 30), "{mode:?} {tones}/{bw}: {:?}", svc.snapshot().tx);
            assert_eq!(sent(&events), text, "{mode:?} {tones}/{bw}");
            let audio = with_noise(&lock(&port.wave), 0.3);
            svc.start_rx();
            for chunk in audio.chunks(1200) {
                svc.inner.queue.send(Msg::Audio(12_000, chunk.to_vec())).expect("coda");
            }
            let deadline = Instant::now() + Duration::from_secs(60);
            while !received(&events).contains(text) && Instant::now() < deadline {
                std::thread::sleep(Duration::from_millis(50));
            }
            // lascia arrivare anche la coda (dove potrebbero esserci caratteri spuri)
            std::thread::sleep(Duration::from_millis(500));
            let got = received(&events);
            assert!(got.contains(text), "{mode:?} {tones}/{bw}: ricevuto {got:?}");
            assert!(got.len() <= text.len() + 4, "{mode:?} {tones}/{bw}: caratteri spuri in {got:?}");
            assert!(lock(&events).iter().any(|e| e.kind == "rx_text" && e.mode.as_deref() == Some(mode.id())));
        }
    }

    /// Olivia trasmette a blocchi: il testo scritto mentre si trasmette (anche a meta' di un blocco, anche dopo che il blocco a meta' e' gia'
    /// partito completato con caratteri nulli) parte tutto e nell'ordine, e si rilegge intero.
    #[test]
    fn olivia_il_testo_scritto_durante_la_trasmissione_non_si_perde() {
        let (svc, port, events, _dir) = olivia_service(DigiMode::Olivia, 8, 500);
        svc.tx_open("CQ").expect("aperta");
        let pieces = ["CQ", " DE", " I", "U3ABC", " IU3ABC", " IO91WM", " K"];
        for p in &pieces[1..] {
            std::thread::sleep(Duration::from_millis(170));
            svc.tx_type(p).expect("scritto");
        }
        // una pausa lunga: il blocco a meta' parte con i nulli, la trasmissione resta aperta
        std::thread::sleep(Duration::from_millis(600));
        svc.tx_type(" TU").expect("scritto");
        svc.tx_close();
        assert!(wait_state(&svc, "done", 30), "stato: {:?}", svc.snapshot().tx);
        let want = format!("{} TU", pieces.concat());
        assert_eq!(sent(&events), want);
        let audio = with_noise(&lock(&port.wave), 0.2);
        let mut d = super::super::Demod::new(&svc.settings(), false);
        let got: String = audio.chunks(1200).map(|c| d.process(c).0).collect();
        assert!(got.contains(&want), "ricevuto: {got:?}");
    }

    /// I caratteri di Olivia risultano partiti quando parte il loro blocco, non quando entrano nel modulatore.
    #[test]
    fn olivia_i_caratteri_partono_con_il_loro_blocco() {
        let (svc, port, events, _dir) = olivia_service(DigiMode::Olivia, 8, 250);
        port.speed.store(1, Ordering::SeqCst);
        svc.transmit("CQ CQ CQ DE IU3ABC").expect("partita");
        // 8/250: un blocco di 8 caratteri dura poco piu' di 2 s; dopo 1 s nessun carattere e' ancora uscito del tutto
        std::thread::sleep(Duration::from_millis(1000));
        assert!(sent(&events).is_empty(), "gia' partiti: {:?}", sent(&events));
        svc.abort_tx();
        assert!(wait_state(&svc, "aborted", 5));
    }

    #[test]
    fn olivia_con_toni_fuori_dalla_banda_audio_non_trasmette() {
        let (svc, port, _events, _dir) = olivia_service(DigiMode::Olivia, 64, 2000);
        let mut s = svc.settings();
        s.rx_freq_hz = 3400.0;
        s.tx_freq_hz = 3400.0;
        svc.apply_settings(s).expect("impostazioni valide anche se il modo non ci sta: si cambia un campo alla volta");
        assert!(matches!(svc.transmit("CQ"), Err(DigiError::BadValue(_))));
        assert!(svc.snapshot().tx.state != "sending");
        assert!(lock(&port.wave).is_empty());
    }
}
