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
    pending: VecDeque<char>,
    open: bool,
    /// Caratteri gia' passati al modulatore: non si possono piu' togliere ne' cambiare.
    taken: usize,
}

impl Session {
    /// I caratteri della trasmissione in tutto (modulati e in coda): le posizioni di [`DigiService::tx_replace`] contano da qui.
    fn total(&self) -> usize {
        self.taken + self.pending.len()
    }
}

/// Il modulatore a flusso per le impostazioni attuali; `lsb`: la radio e' in LSB (RTTY: mark e space invertiti).
fn modulator(s: &DigiSettings, lsb: bool) -> TxModulator {
    match (s.mode.psk(), s.mode) {
        (Some(mode), _) => TxModulator::psk(mode, s.tx_freq()),
        (None, DigiMode::Cw) => TxModulator::cw(s.tx_freq(), s.cw_wpm),
        (None, _) => TxModulator::rtty(s.rtty(s.tx_freq(), lsb)),
    }
}

impl DigiService {
    /// Apre la trasmissione (levetta): si resta in trasmissione finche' non si chiude con [`DigiService::tx_close`]; `text` (anche
    /// vuoto) e' il testo gia' scritto. Con una trasmissione in corso la rende aperta e ci accoda il testo.
    pub fn tx_open(&self, text: &str) -> Result<TxStatus, DigiError> {
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
            let Some(s) = session.as_mut() else { return Err(DigiError::BadValue("nessuna trasmissione in corso".into())) };
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

    /// Una trasmissione a flusso nuova, o il testo accodato a quella in corso.
    pub(super) fn start_session(&self, port: Arc<dyn DigiTxPort>, text: &str, open: bool) -> Result<TxStatus, DigiError> {
        {
            let mut session = lock(&self.inner.session);
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
            *session = Some(Session { pending: text.chars().collect(), open, taken: 0 });
        }
        let settings = self.settings();
        let lsb = self.inner.read_radio_lsb();
        let mut m = modulator(&settings, lsb);
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
        return if s.open { Next::Idle } else { Next::End };
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
                    marks.push_back((generated, t));
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
            break if closing { "done" } else { "error" };
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
        let rest: String = marks.drain(..).map(|(_, t)| t).collect();
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
    }

    impl FakePort {
        fn played(&self) -> u64 {
            let len = lock(&self.wave).len() as u64;
            lock(&self.started).map_or(0, |t| ((t.elapsed().as_secs_f64() * f64::from(RATE)) as u64).min(len))
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
        let out = String::from_utf8_lossy(&d.process(&wave).0).into_owned();
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
        let out = String::from_utf8_lossy(&d.process(&lock(&port.wave)).0).into_owned();
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
}
