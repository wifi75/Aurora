//! Il CW con il WinKeyer. Con l'uscita «WinKeyer» macro e finestra di trasmissione mandano il testo al manipolatore esterno invece di
//! farne audio: il WinKeyer genera la CW e comanda PTT e tasto della radio. Il testo resta nella coda della trasmissione (`Session`) e
//! passa al WinKeyer pochi caratteri alla volta ([`IN_FLIGHT_MAX`]), cosi' Backspace e correzioni valgono fino a poco prima che escano;
//! l'evento `tx_sent` (finestra TX ed eco in ricezione) arriva con l'eco dei caratteri che il WinKeyer manda mentre li trasmette.
//! La macro che riparte e l'Esc svuotano il buffer del WinKeyer (comando 0x0A).

use std::{
    sync::{Arc, atomic::Ordering},
    time::{Duration, Instant},
};

use aurora_core::DigiEvent;
use rig_control::winkeyer::{IN_FLIGHT_MAX, WinKeyer, WinKeyerConfig};

use super::{CwOutput, DigiError, DigiMode, DigiService, DigiSettings, Inner, Msg, TxStatus, live::Session, lock};
use crate::tx_stream::TxModulator;

/// Lo stato del WinKeyer per la pagina: collegato o no, porta, versione del firmware, velocita' del potenziometro.
#[derive(Clone, Debug, Default, serde::Serialize)]
pub struct KeyerStatus {
    pub connected: bool,
    pub port: String,
    /// Versione del firmware letta all'apertura (23 = WK2.3, 31 = WK3.1); 0 se non collegato.
    pub version: u8,
    pub pot_wpm: Option<u8>,
}

/// Come si apre il WinKeyer: impostazioni e velocita' (WPM).
pub type KeyerOpener = Arc<dyn Fn(&WinKeyerConfig, u8) -> Result<WinKeyer, String> + Send + Sync>;

/// Ogni quanto il thread guarda l'eco del WinKeyer.
const STEP: Duration = Duration::from_millis(20);

pub(super) fn serial_opener() -> KeyerOpener {
    Arc::new(rig_control::winkeyer::open_serial)
}

fn wpm_of(s: &DigiSettings) -> u8 {
    s.cw_wpm.round().clamp(5.0, 99.0) as u8
}

pub(super) fn selected(s: &DigiSettings) -> bool {
    s.mode == DigiMode::Cw && s.cw_output == CwOutput::WinKeyer
}

impl DigiService {
    /// Il CW esce dal WinKeyer.
    pub(super) fn keyer_selected(&self) -> bool {
        selected(&lock(&self.inner.settings))
    }

    /// Lo stato del WinKeyer adesso.
    pub fn winkeyer_status(&self) -> KeyerStatus {
        match lock(&self.inner.keyer).as_ref() {
            Some(k) => KeyerStatus { connected: true, port: k.config().port.clone(), version: k.version(), pot_wpm: k.pot_wpm() },
            None => KeyerStatus::default(),
        }
    }

    /// Un altro modo di aprire il WinKeyer (le prove ne mettono uno finto).
    pub fn set_keyer_opener(&self, opener: KeyerOpener) {
        *lock(&self.inner.keyer_opener) = opener;
    }

    /// Il WinKeyer aperto con le impostazioni di adesso (si apre se non lo e', si riapre se e' cambiata la porta).
    fn ensure_keyer(&self) -> Result<(), DigiError> {
        let s = self.settings();
        let mut keyer = lock(&self.inner.keyer);
        if keyer.as_ref().is_some_and(|k| k.config().port == s.winkeyer.port.trim()) {
            return Ok(());
        }
        // la porta vecchia si chiude prima di aprire la nuova
        *keyer = None;
        if s.winkeyer.port.trim().is_empty() {
            return Err(DigiError::Tx("WinKeyer: scegli la porta nelle impostazioni del CW".into()));
        }
        let opener = lock(&self.inner.keyer_opener).clone();
        *keyer = Some(opener(&s.winkeyer, wpm_of(&s)).map_err(DigiError::Tx)?);
        Ok(())
    }

    /// Le impostazioni nuove arrivano al WinKeyer aperto; con un altro modo o l'uscita audio si chiude, con un'altra porta si riaprira'.
    pub(super) fn sync_keyer(&self, s: &DigiSettings) {
        let mut keyer = lock(&self.inner.keyer);
        let Some(k) = keyer.as_mut() else { return };
        if !selected(s) || k.config().port != s.winkeyer.port.trim() {
            if !self.inner.tx_active.load(Ordering::SeqCst) {
                *keyer = None;
            }
            return;
        }
        let r = if *k.config() != s.winkeyer.clamped() { k.reconfigure(&s.winkeyer, wpm_of(s)) } else { k.set_wpm(wpm_of(s)) };
        if let Err(e) = r {
            tracing::warn!("WinKeyer: impostazioni non mandate: {e}");
        }
    }

    /// Svuota il buffer del WinKeyer, se e' aperto (Esc).
    pub(super) fn keyer_clear(&self) {
        if let Some(k) = lock(&self.inner.keyer).as_mut()
            && let Err(e) = k.clear()
        {
            tracing::warn!("WinKeyer: buffer non svuotato: {e}");
        }
    }

    /// «Prova» delle impostazioni: apre il WinKeyer (anche con l'uscita audio) e manda «VVV». Restituisce la versione del firmware.
    pub fn winkeyer_test(&self) -> Result<u8, DigiError> {
        if self.inner.tx_active.load(Ordering::SeqCst) {
            return Err(DigiError::Busy);
        }
        self.ensure_keyer()?;
        let version = lock(&self.inner.keyer).as_ref().map_or(0, WinKeyer::version);
        self.keyer_session_with("VVV", false, true)?;
        Ok(version)
    }

    /// `<WKTEXT:testo>`: il testo va al WinKeyer (lo apre se serve), anche se il CW esce dall'audio.
    pub fn winkeyer_text(&self, text: &str) -> Result<TxStatus, DigiError> {
        if text.trim().is_empty() {
            return Err(DigiError::Empty);
        }
        self.ensure_keyer()?;
        self.keyer_session_with(text, false, false)
    }

    /// `<WKHEX:..>`: byte di comando mandati al WinKeyer cosi' come sono (lo apre se serve).
    pub fn winkeyer_raw(&self, bytes: &[u8]) -> Result<(), DigiError> {
        if bytes.is_empty() {
            return Err(DigiError::Empty);
        }
        self.ensure_keyer()?;
        match lock(&self.inner.keyer).as_mut() {
            Some(k) => k.write_raw(bytes).map_err(DigiError::Tx),
            None => Err(DigiError::Tx("WinKeyer non aperto".into())),
        }
    }

    /// Una trasmissione con il WinKeyer, o il testo accodato a quella in corso.
    pub(super) fn keyer_session(&self, text: &str, open: bool) -> Result<TxStatus, DigiError> {
        self.keyer_session_with(text, open, false)
    }

    fn keyer_session_with(&self, text: &str, open: bool, test: bool) -> Result<TxStatus, DigiError> {
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
        }
        if !test {
            self.ensure_keyer()?;
        }
        *lock(&self.inner.session) = Some(Session::new(text, open));
        let settings = self.settings();
        self.inner.tx_active.store(true, Ordering::SeqCst);
        self.inner.tx_abort.store(false, Ordering::SeqCst);
        let _ = self.inner.queue.send(Msg::Reset);
        tracing::info!("CW: trasmissione con il WinKeyer{}", if open { " (aperta)" } else { "" });
        *lock(&self.inner.tx) = TxStatus { state: "sending".into(), open, ..TxStatus::default() };
        self.inner.tx_event();
        let inner = Arc::clone(&self.inner);
        let tokens = TxModulator::cw(settings.tx_freq(), settings.cw_wpm);
        let spawned = std::thread::Builder::new().name("aurora-winkeyer".into()).spawn(move || run(&inner, &tokens));
        if let Err(e) = spawned {
            *lock(&self.inner.session) = None;
            self.inner.tx_active.store(false, Ordering::SeqCst);
            return Err(DigiError::Tx(e.to_string()));
        }
        Ok(lock(&self.inner.tx).clone())
    }

    /// La macro che riparte: il buffer del WinKeyer si svuota (la CW si ferma subito) e il testo nuovo parte dopo l'ultimo carattere
    /// gia' trasmesso. Restituisce lo stato e quanti caratteri della trasmissione restano (quelli usciti).
    pub(super) fn keyer_restart(&self, text: &str, open: bool) -> Result<(TxStatus, usize), DigiError> {
        {
            let mut session = lock(&self.inner.session);
            if let Some(s) = session.as_mut().filter(|s| !s.ended) {
                // l'ordine dei lucchetti e' sempre sessione, poi WinKeyer (come nel thread)
                let mut keyer = lock(&self.inner.keyer);
                if let Some(k) = keyer.as_mut() {
                    // i caratteri passati al WinKeyer ma non usciti tornano non trasmessi
                    s.taken = s.taken.saturating_sub(k.in_flight());
                    if let Err(e) = k.clear() {
                        tracing::warn!("WinKeyer: buffer non svuotato: {e}");
                    }
                }
                drop(keyer);
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
        self.keyer_session(text, open).map(|st| (st, 0))
    }
}

/// Il thread della trasmissione con il WinKeyer: passa il testo pochi caratteri alla volta, legge l'eco e chiude quando ha finito.
fn run(inner: &Arc<Inner>, tokens: &TxModulator) {
    let started = Instant::now();
    let mut last_emit = Instant::now();
    let mut sent_chars = 0usize;
    let mut closing = false;
    let mut failure: Option<String> = None;
    let state = loop {
        std::thread::sleep(STEP);
        if inner.tx_abort.load(Ordering::SeqCst) {
            break "aborted";
        }
        // l'eco: i pezzi di testo usciti in CW
        let polled = match lock(&inner.keyer).as_mut() {
            Some(k) => k.poll(Duration::ZERO).map(|done| (done, k.busy(), k.config().paddle_breakin && k.paddle_break_in())),
            None => Err("WinKeyer chiuso".to_string()),
        };
        let (done, busy, break_in) = match polled {
            Ok(v) => v,
            Err(e) => {
                failure = Some(e);
                break "error";
            }
        };
        // il paddle ha la precedenza: toccato durante la macro, il buffer si svuota e la trasmissione si ferma
        if break_in {
            tracing::info!("WinKeyer: paddle toccato, trasmissione del testo fermata");
            break "aborted";
        }
        if !done.is_empty() {
            let text: String = done.concat();
            sent_chars += text.chars().count();
            let total = lock(&inner.session).as_ref().map_or(sent_chars, Session::total).max(1);
            inner.emit(DigiEvent { kind: "tx_sent".into(), text: Some(text), mode: Some(DigiMode::Cw.id().into()), progress: Some((sent_chars as f32 / total as f32).min(1.0)), ..DigiEvent::default() });
        }
        // altro testo al WinKeyer finche' ha posto (sessione, poi WinKeyer: lo stesso ordine della macro che riparte)
        if !closing && let Err(e) = feed(inner, tokens, &mut closing) {
            failure = Some(e);
            break "error";
        }
        // occupato si rilegge dopo aver passato il testo: quello letto prima non sa dei caratteri appena mandati
        if closing && !busy && !lock(&inner.keyer).as_ref().is_some_and(WinKeyer::busy) {
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
    if state == "aborted"
        && let Some(k) = lock(&inner.keyer).as_mut()
    {
        let _ = k.clear();
    }
    *lock(&inner.session) = None;
    {
        let mut tx = lock(&inner.tx);
        tx.state = state.into();
        tx.open = false;
        if state == "done" {
            tx.progress = 1.0;
        }
        if let Some(e) = &failure {
            tx.message = Some(format!("WinKeyer: {e}"));
        }
    }
    if failure.is_some() {
        // al prossimo invio si riapre
        *lock(&inner.keyer) = None;
    }
    inner.tx_active.store(false, Ordering::SeqCst);
    inner.tx_abort.store(false, Ordering::SeqCst);
    tracing::info!("CW: trasmissione con il WinKeyer finita ({state})");
    inner.tx_event();
}

/// Passa al WinKeyer i pezzi di testo in coda finche' ha posto; `closing` diventa vero quando il testo e' finito (trasmissione non aperta).
fn feed(inner: &Inner, tokens: &TxModulator, closing: &mut bool) -> Result<(), String> {
    let mut session = lock(&inner.session);
    let Some(s) = session.as_mut() else {
        *closing = true;
        return Ok(());
    };
    let mut keyer = lock(&inner.keyer);
    let Some(k) = keyer.as_mut() else { return Err("WinKeyer chiuso".into()) };
    while k.ready() && k.in_flight() < IN_FLIGHT_MAX {
        let head: Vec<char> = s.pending.iter().take(4).copied().collect();
        if head.is_empty() {
            if !s.open {
                s.ended = true;
                *closing = true;
            }
            return Ok(());
        }
        // un segno di procedura a meta' («<S» di «<SK>») aspetta il resto
        let Some(n) = tokens.next_token(&head, s.open) else { return Ok(()) };
        let n = n.min(head.len());
        let piece: String = s.pending.drain(..n).collect();
        s.taken += n;
        k.send(&piece)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use rig_control::winkeyer::fake::{FakeLink, FakeState};

    use super::*;

    struct Dir(std::path::PathBuf);
    impl Drop for Dir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    fn service(tag: &str) -> (DigiService, Arc<Mutex<Option<Arc<Mutex<FakeState>>>>>, Arc<Mutex<Vec<DigiEvent>>>, Dir) {
        let dir = std::env::temp_dir().join(format!("aurora-wk-{tag}-{}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);
        let svc = DigiService::open(&dir);
        let fake: Arc<Mutex<Option<Arc<Mutex<FakeState>>>>> = Arc::new(Mutex::new(None));
        let slot = Arc::clone(&fake);
        svc.set_keyer_opener(Arc::new(move |cfg, wpm| {
            let (link, state) = FakeLink::new();
            *lock(&slot) = Some(state);
            WinKeyer::open(Box::new(link), cfg, wpm)
        }));
        let events = Arc::new(Mutex::new(Vec::new()));
        let sink = Arc::clone(&events);
        svc.set_emitter(Arc::new(move |e| lock(&sink).push(e)));
        let mut s = svc.settings();
        s.mode = DigiMode::Cw;
        s.cw_output = CwOutput::WinKeyer;
        s.cw_wpm = 24.0;
        s.winkeyer.port = "COM9".into();
        svc.apply_settings(s).expect("impostazioni");
        (svc, fake, events, Dir(dir))
    }

    fn state(fake: &Mutex<Option<Arc<Mutex<FakeState>>>>) -> Arc<Mutex<FakeState>> {
        lock(fake).clone().expect("WinKeyer aperto")
    }

    fn wait_state(svc: &DigiService, want: &str) -> bool {
        let t0 = Instant::now();
        while t0.elapsed() < Duration::from_secs(5) {
            if svc.snapshot().tx.state == want {
                return true;
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        false
    }

    fn sent(events: &Mutex<Vec<DigiEvent>>) -> String {
        lock(events).iter().filter(|e| e.kind == "tx_sent").filter_map(|e| e.text.clone()).collect()
    }

    #[test]
    fn la_macro_va_al_winkeyer_e_l_avanzamento_viene_dall_eco() {
        let (svc, fake, events, _d) = service("macro");
        svc.transmit("CQ DE IU3ABC <SK>").expect("trasmissione");
        assert!(wait_state(&svc, "done"));
        let st = state(&fake);
        let s = lock(&st);
        assert_eq!(s.wpm, Some(24));
        assert_eq!(s.received, "CQ DE IU3ABC <SK>");
        assert_eq!(sent(&events), "CQ DE IU3ABC <SK>");
    }

    #[test]
    fn esc_svuota_il_buffer_del_winkeyer() {
        let (svc, fake, _events, _d) = service("esc");
        svc.tx_open("CQ CQ").expect("trasmissione");
        let st = state(&fake);
        lock(&st).per_read = 0;
        svc.abort_tx();
        assert!(wait_state(&svc, "aborted"));
        assert!(lock(&st).clears >= 1);
    }

    #[test]
    fn la_macro_che_riparte_svuota_il_buffer_e_ricomincia() {
        let (svc, fake, events, _d) = service("restart");
        svc.tx_open("").expect("trasmissione aperta");
        let st = state(&fake);
        // il WinKeyer «fermo»: niente esce finche' la prova non lo fa ripartire
        lock(&st).per_read = 0;
        svc.tx_type("AAAAAAAA").expect("testo");
        std::thread::sleep(Duration::from_millis(100));
        let (_, kept) = svc.transmit_restart("TEST", false).expect("ripartenza");
        assert_eq!(kept, 0);
        lock(&st).per_read = 1;
        assert!(wait_state(&svc, "done"));
        let s = lock(&st);
        assert_eq!(s.clears, 1);
        assert_eq!(s.keyed, "TEST");
        assert_eq!(sent(&events), "TEST");
    }

    #[test]
    fn il_paddle_toccato_ferma_la_macro_e_lo_stato_dice_collegato() {
        let (svc, fake, _events, _d) = service("paddle");
        svc.tx_open("CQ CQ CQ").expect("trasmissione");
        let snap = svc.snapshot();
        assert!(snap.keyer.connected);
        assert_eq!(snap.keyer.version, 31);
        let st = state(&fake);
        {
            let mut s = lock(&st);
            s.per_read = 0;
            // stato: occupato e break-in del paddle
            s.out.push_back(0xC6);
        }
        assert!(wait_state(&svc, "aborted"));
        assert!(lock(&st).clears >= 1);
    }

    #[test]
    fn il_cambio_di_velocita_arriva_al_winkeyer_aperto() {
        let (svc, fake, _events, _d) = service("wpm");
        svc.transmit("E").expect("trasmissione");
        assert!(wait_state(&svc, "done"));
        let mut s = svc.settings();
        s.cw_wpm = 30.0;
        svc.apply_settings(s).expect("impostazioni");
        assert_eq!(lock(&state(&fake)).wpm, Some(30));
    }

    #[test]
    fn la_prova_manda_vvv_e_senza_porta_si_spiega() {
        let (svc, fake, _events, _d) = service("prova");
        assert_eq!(svc.winkeyer_test().expect("prova"), 31);
        assert!(wait_state(&svc, "done"));
        assert_eq!(lock(&state(&fake)).keyed, "VVV");
        let mut s = svc.settings();
        s.winkeyer.port = String::new();
        svc.apply_settings(s).expect("impostazioni");
        assert!(matches!(svc.transmit("E"), Err(DigiError::Tx(_))));
    }
}
