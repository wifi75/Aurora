//! PTT: il comando che mette la radio in trasmissione. Implementazioni su `rigctld` (Hamlib), su OmniRig e `NoPtt` per il VOX.
//!
//! Il PTT non va mai lasciato acceso: [`PttGuard`] lo spegne quando cade (anche per un panico) e il pianificatore ha in piu' un
//! controllo a tempo (`watchdog`).

use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant},
};

use rig_control::hamlib::RigctldClient;

use crate::{error::TxError, ptt_release::{ReleaseTracker, not_released_text}};

/// Un comando di trasmissione. `set` puo' bloccare per qualche istante (porta seriale, OmniRig): chi lo usa lo chiama da un thread dedicato.
pub trait Ptt: Send + Sync {
    fn set(&self, on: bool) -> Result<(), TxError>;
    /// Lo stato che Aurora crede di aver impostato (acceso se l'ultimo `set(true)` e' riuscito e non c'e' stato un `set(false)` riuscito).
    fn is_on(&self) -> bool;
    /// Un'altra via per spegnere il PTT quando il comando normale non riesce (riapertura del CAT, linea RTS/DTR della porta del PTT). Si chiama
    /// dal terzo tentativo fallito di [`crate::ptt_release::ReleaseTracker`]; `Ok` = la radio e' stata spenta. Di base nessuna via.
    fn recover_off(&self) -> Result<(), TxError> {
        Err(TxError::Ptt("nessuna altra via per spegnere il PTT".into()))
    }
    /// A cosa comanda questo PTT (la porta seriale, la radio di OmniRig, il CAT con il suo profilo): serve a capire se una catena
    /// ricostruita parla ancora con la stessa radio. `None` = non si sa (VOX).
    fn target(&self) -> Option<String> {
        None
    }
    /// Cosa dice la radio: `Some(true)` se risulta in trasmissione, `Some(false)` se in ricezione, `None` se non si puo' leggere (VOX, OmniRig,
    /// radio che non risponde al comando). Serve alla diagnosi della trasmissione: un PTT accettato ma ignorato dalla radio si vede da qui.
    fn read_state(&self) -> Option<bool> {
        None
    }
}

/// Nessun comando: la radio va in trasmissione col VOX, appena arriva l'audio.
#[derive(Debug, Default)]
pub struct NoPtt {
    on: AtomicBool,
}

impl NoPtt {
    pub fn new() -> NoPtt {
        NoPtt::default()
    }
}

impl Ptt for NoPtt {
    fn set(&self, on: bool) -> Result<(), TxError> {
        self.on.store(on, Ordering::SeqCst);
        Ok(())
    }

    fn is_on(&self) -> bool {
        self.on.load(Ordering::SeqCst)
    }
}

/// PTT con `T 1` / `T 0` su un `rigctld` (lo stesso client che comanda frequenza e modo).
pub struct HamlibPtt {
    client: Arc<RigctldClient>,
    on: AtomicBool,
}

impl HamlibPtt {
    pub fn new(client: Arc<RigctldClient>) -> HamlibPtt {
        HamlibPtt { client, on: AtomicBool::new(false) }
    }
}

impl Ptt for HamlibPtt {
    fn set(&self, on: bool) -> Result<(), TxError> {
        self.client.set_ptt(on).map_err(|e| TxError::Ptt(e.0))?;
        self.on.store(on, Ordering::SeqCst);
        Ok(())
    }

    fn is_on(&self) -> bool {
        self.on.load(Ordering::SeqCst)
    }

    fn read_state(&self) -> Option<bool> {
        self.client.ptt_within(Duration::from_millis(300)).ok()
    }
}

/// PTT via OmniRig (`Rig1.Tx` / `Rig2.Tx` = PM_TX / PM_RX, come `do_ptt` di `OmniRigTransceiver.cpp:685`), via COM su un thread dedicato
/// (`rig_control::omnirig::set_ptt`). Solo Windows. La radio comandata e' quella scelta in Impostazioni › Radio.
pub struct OmniRigPtt {
    on: AtomicBool,
    rig: u8,
}

impl OmniRigPtt {
    /// `rig` e' 1 o 2 (le due radio di OmniRig).
    pub fn new(rig: u8) -> Result<OmniRigPtt, TxError> {
        if !(1..=2).contains(&rig) {
            return Err(TxError::Ptt(format!("radio OmniRig non valida: {rig}")));
        }
        Ok(OmniRigPtt { on: AtomicBool::new(false), rig })
    }

    /// Apre l'oggetto OmniRig in anticipo, cosi' il primo PTT non paga la creazione.
    pub fn warm_up(&self) -> Result<(), TxError> {
        rig_control::omnirig::warm_up_ptt().map_err(|e| TxError::Ptt(e.0))
    }
}

impl Ptt for OmniRigPtt {
    fn target(&self) -> Option<String> {
        Some(format!("omnirig:{}", self.rig))
    }

    fn set(&self, on: bool) -> Result<(), TxError> {
        rig_control::omnirig::set_ptt(on).map_err(|e| TxError::Ptt(e.0))?;
        self.on.store(on, Ordering::SeqCst);
        Ok(())
    }

    fn is_on(&self) -> bool {
        self.on.load(Ordering::SeqCst)
    }
}
/// Tiene acceso il PTT finche' vive: cadendo (anche per un panico) lo spegne.
pub struct PttGuard {
    ptt: Arc<dyn Ptt>,
    tracker: Arc<ReleaseTracker>,
    armed: bool,
    /// Ferma l'audio: si chiama prima di ogni spegnimento fatto cadendo, cosi' la scala dei tentativi non va con l'audio ancora in onda.
    stop_audio: Option<Box<dyn Fn() + Send + Sync>>,
}

impl PttGuard {
    /// Accende il PTT. Se il comando fallisce si prova comunque a spegnere (la risposta potrebbe essersi persa a radio gia' in trasmissione).
    pub fn engage(ptt: Arc<dyn Ptt>, tracker: Arc<ReleaseTracker>) -> Result<PttGuard, TxError> {
        let mut guard = PttGuard { ptt, tracker, armed: true, stop_audio: None };
        match guard.ptt.set(true) {
            Ok(()) => Ok(guard),
            Err(e) => {
                let _ = guard.release();
                Err(e)
            }
        }
    }

    /// Dice alla guardia come fermare l'audio quando cade con il PTT ancora acceso (errore o panico dopo l'avvio dell'audio).
    pub fn with_stop_audio(mut self, stop: Box<dyn Fn() + Send + Sync>) -> PttGuard {
        self.stop_audio = Some(stop);
        self
    }

    /// Spegne il PTT con la scala dei tentativi. Se non ci riesce resta armato: lo riprovera' `Drop`, poi il controllo a tempo.
    pub fn release(&mut self) -> Result<(), TxError> {
        self.tracker.release(&*self.ptt)?;
        self.armed = false;
        Ok(())
    }
}

impl Drop for PttGuard {
    fn drop(&mut self) {
        if !self.armed {
            return;
        }
        if let Some(stop) = &self.stop_audio {
            stop();
        }
        // un solo tentativo: se la scala e' gia' fallita, il resto lo fa il controllo a tempo, senza altre righe nel registro
        if self.tracker.is_stuck() {
            let _ = self.tracker.retry_once(&*self.ptt);
        } else if let Err(e) = self.tracker.release(&*self.ptt) {
            tracing::debug!("{}", not_released_text(&e));
        }
    }
}

/// Quanti controller si seguono uno per uno (`maximum_tracked_holds` di WSJT-X 3.3): gli altri allungano una prenotazione comune.
pub const MAX_TRACKED_HOLDS: usize = 64;

/// Lo stato del blocco della trasmissione, con i contatori dall'avvio (i campi di InhibitStatus, tipo 17 di WSJT-X).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct InhibitSnapshot {
    pub inhibited: bool,
    /// Chi blocca: le stazioni (o i controller) delle prenotazioni attive, senza doppioni, in ordine alfabetico, separate da «, ».
    pub holder: String,
    pub hold_rx: u32,
    pub release_rx: u32,
    pub expiries: u32,
    pub invalid: u32,
}

#[derive(Debug, Clone)]
struct Hold {
    expires_at: Instant,
    holder: String,
}

/// Le prenotazioni di blocco dei controller, come `TxInhibitTransceiver` di WSJT-X 3.3: una per controller (la rinnova lo stesso controller,
/// la toglie un TTL 0), al piu' 64 seguite una per una; oltre, una prenotazione comune che dura fino alla scadenza piu' lontana e che nessun
/// rilascio toglie (la sua scadenza conta una volta). La trasmissione e' bloccata finche' c'e' almeno una prenotazione (OR).
#[derive(Debug, Default)]
pub struct InhibitLeases {
    holds: Vec<(String, Hold)>,
    overflow: Option<Instant>,
    hold_rx: u32,
    release_rx: u32,
    expiries: u32,
    invalid: u32,
}

/// Il nome di chi blocca come lo mostra WSJT-X (`sanitize_holder`): al piu' 64 caratteri stampabili, senza spazi ai bordi.
fn sanitize_holder(text: &str) -> String {
    text.chars().take(64).filter(|c| !c.is_control()).collect::<String>().trim().to_string()
}

impl InhibitLeases {
    pub fn is_inhibited(&self) -> bool {
        !self.holds.is_empty() || self.overflow.is_some()
    }

    /// Un comando valido: `ttl_ms` 0 rilascia la prenotazione di `controller`, altrimenti la crea o la rinnova fino a `now + ttl_ms`.
    pub fn command(&mut self, controller: &str, ttl_ms: u32, station: &str, now: Instant) {
        if ttl_ms == 0 {
            self.release_rx = self.release_rx.saturating_add(1);
            self.holds.retain(|(c, _)| c != controller);
            return;
        }
        self.hold_rx = self.hold_rx.saturating_add(1);
        let expires_at = now + Duration::from_millis(u64::from(ttl_ms));
        let holder = sanitize_holder(if station.is_empty() { controller } else { station });
        if let Some((_, hold)) = self.holds.iter_mut().find(|(c, _)| c == controller) {
            *hold = Hold { expires_at, holder };
        } else if self.holds.len() < MAX_TRACKED_HOLDS {
            self.holds.push((controller.to_string(), Hold { expires_at, holder }));
        } else {
            self.overflow = Some(self.overflow.map_or(expires_at, |at| at.max(expires_at)));
        }
    }

    /// Comandi malformati ricevuti (si contano soltanto).
    pub fn invalid(&mut self, count: u32) {
        self.invalid = self.invalid.saturating_add(count);
    }

    /// Toglie le prenotazioni scadute entro `now`; vero se ne e' scaduta almeno una.
    pub fn expire(&mut self, now: Instant) -> bool {
        let before = self.holds.len();
        self.holds.retain(|(_, h)| h.expires_at > now);
        let mut expired = u32::try_from(before - self.holds.len()).unwrap_or(u32::MAX);
        if self.overflow.is_some_and(|at| at <= now) {
            self.overflow = None;
            expired = expired.saturating_add(1);
        }
        self.expiries = self.expiries.saturating_add(expired);
        expired > 0
    }

    /// La prossima scadenza, se c'e' una prenotazione.
    pub fn next_expiry(&self) -> Option<Instant> {
        self.holds.iter().map(|(_, h)| h.expires_at).chain(self.overflow).min()
    }

    pub fn snapshot(&self) -> InhibitSnapshot {
        let mut holders: Vec<String> = Vec::new();
        for (_, h) in &self.holds {
            if !h.holder.is_empty() && !holders.contains(&h.holder) {
                holders.push(h.holder.clone());
            }
        }
        holders.sort_by_key(|h| h.to_lowercase());
        InhibitSnapshot {
            inhibited: self.is_inhibited(),
            holder: holders.join(", "),
            hold_rx: self.hold_rx,
            release_rx: self.release_rx,
            expiries: self.expiries,
            invalid: self.invalid,
        }
    }
}

struct InterlockState {
    leases: InhibitLeases,
    inner: Option<Arc<dyn Ptt>>,
    /// Il PTT che la trasmissione ha chiesto.
    requested: bool,
    /// Il PTT davvero acceso sulla radio.
    keyed: bool,
    /// Il PTT di una catena di trasmissione ricostruita che non si e' riusciti a spegnere: si continua a provarci, e nel frattempo non si accende altro.
    orphan: Option<Arc<dyn Ptt>>,
    /// Da quando l'orfano non si spegne.
    orphan_since: Option<Instant>,
    /// Dopo quanto un orfano che non si spegne si abbandona.
    orphan_max: Duration,
    /// Quanti orfani sono stati abbandonati (per le prove).
    abandoned: u32,
    /// Chi viene avvisato dell'abbandono (l'avviso persistente nello stato della stazione).
    abandon_hook: Option<AbandonHook>,
}

/// Chiamata quando un PTT rimasto acceso viene abbandonato; riceve il motivo.
pub type AbandonHook = Arc<dyn Fn(String) + Send + Sync>;

/// Un PTT rimasto acceso di una catena ricostruita si abbandona dopo questo tempo: la sua porta (cavo USB tolto) o il suo CAT non tornano.
const ORPHAN_MAX: Duration = Duration::from_secs(60);

/// Il PTT con il blocco esterno (TxInhibit via UDP): sta tra il pianificatore e il PTT vero. Con una prenotazione attiva il PTT non si accende
/// (e se e' acceso si spegne subito); alla scadenza o al rilascio dell'ultima, se la trasmissione lo chiede ancora, si riaccende. Come
/// `TxInhibitTransceiver` di WSJT-X l'audio non si ferma: e' il comando alla radio che resta spento.
pub struct TxInterlock {
    state: std::sync::Mutex<InterlockState>,
}

impl Default for TxInterlock {
    fn default() -> Self {
        TxInterlock::new()
    }
}

impl TxInterlock {
    pub fn new() -> TxInterlock {
        TxInterlock { state: std::sync::Mutex::new(InterlockState { leases: InhibitLeases::default(), inner: None, requested: false, keyed: false, orphan: None, orphan_since: None, orphan_max: ORPHAN_MAX, abandoned: 0, abandon_hook: None }) }
    }

    /// Il PTT vero (cambia quando si ricostruisce la catena di trasmissione). Quello di prima, se acceso, si spegne; se non si spegne resta da
    /// parte (`orphan`) e si continua a provare, ma non per sempre: vedi [`TxInterlock::judge_orphan`].
    pub fn attach(&self, inner: Arc<dyn Ptt>) {
        let mut s = crate::lock(&self.state);
        Self::retry_orphan(&mut s);
        if s.orphan.is_some() {
            // seconda ricostruzione con un orfano ancora acceso: non si accumulano
            Self::abandon(&mut s, "seconda ricostruzione della catena con il PTT ancora acceso");
        }
        if let Some(old) = s.inner.take()
            && s.keyed
            && let Err(e) = old.set(false)
        {
            // non si dimentica: si tiene da parte, `is_on` resta vero e la nuova catena parte «non spenta» e riprova
            tracing::error!("PTT precedente non spento: {e}");
            s.orphan = Some(old);
            s.orphan_since = Some(Instant::now());
        }
        s.inner = Some(inner);
        s.requested = false;
        s.keyed = false;
        Self::judge_orphan(&mut s);
    }

    /// Prima di riaprire la stessa porta seriale (la COM e' esclusiva): il PTT corrente e l'orfano che comandano `target` si spengono per
    /// l'ultima volta e si lasciano cadere, cosi' la porta si libera (chiudere la maniglia abbassa le linee).
    pub fn release_target(&self, target: &str) {
        let mut s = crate::lock(&self.state);
        if let Some(o) = s.orphan.clone()
            && o.target().as_deref() == Some(target)
        {
            let _ = o.set(false);
            s.orphan = None;
            s.orphan_since = None;
        }
        if let Some(i) = s.inner.clone()
            && i.target().as_deref() == Some(target)
        {
            if let Err(e) = i.set(false) {
                tracing::warn!("PTT su {target}: ultimo spegnimento non riuscito prima di riaprire la porta: {e}");
            }
            s.inner = None;
            s.keyed = false;
            s.requested = false;
        }
    }

    /// Chi avvisare quando un orfano si abbandona (si imposta a ogni ricostruzione della catena).
    pub fn set_abandon_hook(&self, hook: Option<AbandonHook>) {
        crate::lock(&self.state).abandon_hook = hook;
    }

    /// Quanti orfani sono stati abbandonati.
    pub fn abandoned(&self) -> u32 {
        crate::lock(&self.state).abandoned
    }

    /// Cambia il tempo massimo dell'orfano (le prove lo accorciano).
    pub fn set_orphan_max(&self, max: Duration) {
        crate::lock(&self.state).orphan_max = max;
    }

    /// L'orfano non si recupera piu': si lascia andare (la chiusura del PTT abbassa comunque la linea se la maniglia e' valida), si dice nel
    /// registro e all'interfaccia, e la catena nuova e' libera.
    fn abandon(s: &mut InterlockState, why: &str) {
        let waited = s.orphan_since.map_or(0, |t| t.elapsed().as_secs());
        let Some(orphan) = s.orphan.take() else { return };
        s.orphan_since = None;
        // ultima possibilita': un'altra via (riapertura del CAT, linea della porta). Se riesce il PTT e' spento: nessun avviso
        if orphan.recover_off().is_ok() {
            tracing::info!("PTT precedente spento con un'altra via prima di abbandonarlo");
            return;
        }
        s.abandoned += 1;
        tracing::error!("PTT precedente abbandonato: radio non raggiungibile dopo {waited} s ({why})");
        if let Some(hook) = &s.abandon_hook {
            hook(why.to_string());
        }
    }

    /// Un orfano si abbandona se: (a) la catena nuova comanda altro (un'altra radio, un'altra porta, il VOX senza CAT) oppure (b) e' passato il
    /// tempo massimo. Il caso di una seconda ricostruzione sta in `attach`.
    fn judge_orphan(s: &mut InterlockState) {
        let Some(o) = s.orphan.clone() else { return };
        let new_target = s.inner.as_ref().and_then(|i| i.target());
        if o.target() != new_target {
            Self::abandon(s, "la catena nuova comanda un'altra radio");
        } else if s.orphan_since.is_some_and(|t| t.elapsed() >= s.orphan_max) {
            Self::abandon(s, "tempo massimo scaduto");
        }
    }

    /// Un tentativo sul PTT rimasto acceso della catena di prima.
    fn retry_orphan(s: &mut InterlockState) {
        let Some(o) = s.orphan.clone() else { return };
        if o.set(false).is_ok() {
            tracing::info!("PTT precedente spento");
            s.orphan = None;
            s.orphan_since = None;
            return;
        }
        Self::judge_orphan(s);
    }

    /// C'e' un PTT della catena di prima ancora da spegnere.
    pub fn has_orphan(&self) -> bool {
        crate::lock(&self.state).orphan.is_some()
    }

    /// Porta la radio allo stato voluto: acceso solo se chiesto e senza blocchi.
    fn apply(s: &mut InterlockState) -> Result<(), TxError> {
        Self::retry_orphan(s);
        // il PTT della catena di prima e' ancora acceso: nessuna accensione e nessuna conferma di «spento» finche' non lo e' davvero
        if s.orphan.is_some() {
            return Err(TxError::Ptt("PTT precedente non spento".into()));
        }
        let want = s.requested && !s.leases.is_inhibited();
        if want == s.keyed {
            return Ok(());
        }
        let Some(inner) = s.inner.clone() else { return Ok(()) };
        if want {
            inner.set(true)?;
            s.keyed = true;
        } else {
            // un solo tentativo: la scala dei tentativi e' di chi comanda l'interblocco (vedi `ptt_release`)
            inner.set(false)?;
            s.keyed = false;
        }
        Ok(())
    }

    fn after_change(s: &mut InterlockState, was_inhibited: bool) {
        let now_inhibited = s.leases.is_inhibited();
        if was_inhibited != now_inhibited {
            if now_inhibited {
                tracing::warn!("Tx bloccata da un controller esterno ({})", s.leases.snapshot().holder);
            } else {
                tracing::info!("blocco della Tx tolto");
            }
            if let Err(e) = Self::apply(s) {
                tracing::error!("PTT non portato allo stato voluto dopo il cambio del blocco: {e}");
            }
        }
    }

    /// Un TxInhibit valido (vedi [`InhibitLeases::command`]), adesso.
    pub fn command(&self, controller: &str, ttl_ms: u32, station: &str) {
        self.command_at(controller, ttl_ms, station, Instant::now());
    }

    pub fn command_at(&self, controller: &str, ttl_ms: u32, station: &str, now: Instant) {
        let mut s = crate::lock(&self.state);
        let was = s.leases.is_inhibited();
        s.leases.expire(now);
        s.leases.command(controller, ttl_ms, station, now);
        Self::after_change(&mut s, was);
    }

    pub fn invalid(&self, count: u32) {
        crate::lock(&self.state).leases.invalid(count);
    }

    /// Fa scadere le prenotazioni (da chiamare spesso: ogni 50 ms basta, la durata minima e' 100 ms); vero se lo stato e' cambiato.
    pub fn tick(&self) -> bool {
        self.tick_at(Instant::now())
    }

    pub fn tick_at(&self, now: Instant) -> bool {
        let mut s = crate::lock(&self.state);
        let was = s.leases.is_inhibited();
        let expired = s.leases.expire(now);
        Self::after_change(&mut s, was);
        expired
    }

    pub fn snapshot(&self) -> InhibitSnapshot {
        crate::lock(&self.state).leases.snapshot()
    }

    pub fn is_inhibited(&self) -> bool {
        crate::lock(&self.state).leases.is_inhibited()
    }

    /// Il PTT e' davvero acceso sulla radio (non solo chiesto).
    pub fn is_keyed(&self) -> bool {
        crate::lock(&self.state).keyed
    }
}

impl Ptt for TxInterlock {
    fn set(&self, on: bool) -> Result<(), TxError> {
        let mut s = crate::lock(&self.state);
        s.requested = on;
        if on && s.leases.is_inhibited() {
            tracing::warn!("PTT trattenuto: Tx bloccata da {}", s.leases.snapshot().holder);
        }
        Self::apply(&mut s)
    }

    fn target(&self) -> Option<String> {
        crate::lock(&self.state).inner.as_ref().and_then(|i| i.target())
    }

    /// Acceso se chiesto o se la radio risulta ancora in trasmissione: dopo uno spegnimento fallito `requested` e' gia' falso ma il PTT
    /// no, e chi controlla (controllo a tempo, guardia) deve vederlo.
    fn is_on(&self) -> bool {
        let s = crate::lock(&self.state);
        s.requested || s.keyed || s.orphan.is_some()
    }

    fn recover_off(&self) -> Result<(), TxError> {
        let (inner, orphan) = {
            let s = crate::lock(&self.state);
            (s.inner.clone(), s.orphan.clone())
        };
        if let Some(o) = orphan
            && o.recover_off().is_ok()
        {
            crate::lock(&self.state).orphan = None;
        }
        let Some(inner) = inner else { return Err(TxError::Ptt("nessun PTT collegato".into())) };
        if crate::lock(&self.state).keyed {
            inner.recover_off()?;
        }
        let mut s = crate::lock(&self.state);
        s.keyed = false;
        s.requested = false;
        if s.orphan.is_some() {
            return Err(TxError::Ptt("PTT precedente non spento".into()));
        }
        Ok(())
    }
}

#[cfg(test)]
mod interlock_tests {
    use super::*;

    fn ms(n: u64) -> Duration {
        Duration::from_millis(n)
    }

    fn rec_ptt(target: Option<&str>) -> Arc<crate::testing::RecordingPtt> {
        let rec = crate::testing::Recorder::new(Arc::new(crate::clock::ManualClock::new(Duration::ZERO)));
        let p = Arc::new(crate::testing::RecordingPtt::new(rec));
        *crate::lock(&p.target) = target.map(str::to_string);
        p
    }

    /// Un interblocco con un PTT acceso che non si spegne, poi sostituito da `next`: da' l'orfano.
    fn with_orphan(old_target: Option<&str>, next: &Arc<crate::testing::RecordingPtt>) -> (TxInterlock, Arc<crate::testing::RecordingPtt>) {
        let old = rec_ptt(old_target);
        let lock = TxInterlock::new();
        lock.attach(old.clone());
        lock.set(true).expect("acceso");
        old.fail_off.store(true, std::sync::atomic::Ordering::SeqCst);
        lock.attach(next.clone());
        (lock, old)
    }

    #[test]
    fn an_orphan_of_another_radio_is_abandoned_at_once_and_transmitting_is_possible_again() {
        let next = rec_ptt(Some("serial:COM9"));
        let (lock, _old) = with_orphan(Some("serial:COM3"), &next);
        assert!(!lock.has_orphan() && !lock.is_on());
        assert_eq!(lock.abandoned(), 1);
        lock.set(true).expect("la nuova radio trasmette");
        assert!(next.is_on());
    }

    #[test]
    fn an_orphan_of_the_same_radio_is_kept_then_abandoned_after_the_maximum_time() {
        let next = rec_ptt(Some("serial:COM3"));
        let (lock, old) = with_orphan(Some("serial:COM3"), &next);
        lock.set_orphan_max(Duration::from_millis(60));
        assert!(lock.has_orphan() && lock.is_on());
        assert!(lock.set(true).is_err(), "nessuna accensione con un orfano acceso");
        std::thread::sleep(Duration::from_millis(80));
        lock.set(false).expect("l'orfano scaduto cade e il resto va");
        assert!(!lock.has_orphan() && !lock.is_on());
        assert_eq!(lock.abandoned(), 1);
        assert!(old.is_on(), "il vecchio PTT non e' stato spento: l'avviso lo dice");
    }

    #[test]
    fn an_orphan_that_the_other_way_turns_off_is_dropped_without_the_warning() {
        let next = rec_ptt(Some("serial:COM9"));
        let old = rec_ptt(Some("serial:COM3"));
        let lock = TxInterlock::new();
        let told = Arc::new(std::sync::Mutex::new(Vec::<String>::new()));
        let seen = Arc::clone(&told);
        lock.set_abandon_hook(Some(Arc::new(move |why| seen.lock().expect("avvisi").push(why))));
        lock.attach(old.clone());
        lock.set(true).expect("acceso");
        old.fail_off.store(true, std::sync::atomic::Ordering::SeqCst);
        old.recover_ok.store(true, std::sync::atomic::Ordering::SeqCst);
        lock.attach(next);
        assert!(!lock.has_orphan() && !old.is_on());
        assert_eq!(lock.abandoned(), 0);
        assert!(told.lock().expect("avvisi").is_empty());
    }

    #[test]
    fn a_changed_radio_profile_makes_an_orphan_and_the_abandon_tells_it() {
        // il PTT della radio A rifiuta lo spegnimento (la guardia `radio cambiata` di CatPtt), la catena si ricostruisce sulla radio B
        let a = rec_ptt(Some("cat:Hamlib:1:COM3"));
        let b = rec_ptt(Some("cat:Hamlib:2:COM4"));
        let lock = TxInterlock::new();
        let told = Arc::new(std::sync::Mutex::new(Vec::<String>::new()));
        let seen = Arc::clone(&told);
        lock.set_abandon_hook(Some(Arc::new(move |why| seen.lock().expect("avvisi").push(why))));
        lock.attach(a.clone());
        lock.set(true).expect("acceso");
        a.fail_off.store(true, std::sync::atomic::Ordering::SeqCst);
        lock.attach(b.clone());
        assert_eq!(told.lock().expect("avvisi").len(), 1, "un solo avviso");
        assert!(!lock.has_orphan());
        lock.set(true).expect("la radio B trasmette");
        assert!(b.is_on());
    }

    #[test]
    fn a_cat_orphan_with_a_vox_chain_is_abandoned() {
        let vox = rec_ptt(None);
        let (lock, _old) = with_orphan(Some("cat:Hamlib:1:COM3"), &vox);
        assert!(!lock.has_orphan());
        assert_eq!(lock.abandoned(), 1);
    }

    #[test]
    fn an_orphan_that_turns_off_by_itself_goes_away_without_the_abandon_notice() {
        let next = rec_ptt(Some("serial:COM3"));
        let (lock, old) = with_orphan(Some("serial:COM3"), &next);
        assert!(lock.has_orphan());
        old.fail_off.store(false, std::sync::atomic::Ordering::SeqCst);
        lock.set(false).expect("spento");
        assert!(!lock.has_orphan() && !old.is_on());
        assert_eq!(lock.abandoned(), 0);
    }

    #[test]
    fn two_attaches_in_a_row_do_not_stack_orphans() {
        let next = rec_ptt(Some("serial:COM3"));
        let (lock, _old) = with_orphan(Some("serial:COM3"), &next);
        assert!(lock.has_orphan());
        lock.attach(rec_ptt(Some("serial:COM3")));
        assert!(!lock.has_orphan(), "al secondo attach l'orfano ancora acceso si abbandona");
        assert_eq!(lock.abandoned(), 1);
    }

    #[test]
    fn the_same_port_is_released_before_it_is_reopened() {
        let old = rec_ptt(Some("serial:COM3"));
        let lock = TxInterlock::new();
        lock.attach(old.clone());
        lock.set(true).expect("acceso");
        lock.release_target("serial:COM3");
        assert!(!old.is_on(), "ultimo spegnimento");
        assert!(!lock.is_on());
        assert_eq!(Arc::strong_count(&old), 1, "il PTT vecchio e' lasciato cadere: la porta si libera");
    }

    #[test]
    fn a_hold_blocks_the_ptt_and_its_expiry_gives_it_back() {
        let t0 = Instant::now();
        let radio = Arc::new(NoPtt::new());
        let lock = TxInterlock::new();
        lock.attach(radio.clone());
        lock.set(true).expect("ptt");
        assert!(radio.is_on());
        lock.command_at("AMP", 600, "K1ABC amp", t0);
        assert!(!radio.is_on(), "la prenotazione spegne subito il PTT");
        assert!(lock.is_on(), "la trasmissione lo chiede ancora");
        assert_eq!(lock.snapshot(), InhibitSnapshot { inhibited: true, holder: "K1ABC amp".into(), hold_rx: 1, ..Default::default() });
        // rinnovo prima della scadenza: resta bloccata oltre i 600 ms iniziali
        lock.command_at("AMP", 600, "K1ABC amp", t0 + ms(500));
        assert!(!lock.tick_at(t0 + ms(700)));
        assert!(!radio.is_on());
        assert!(lock.tick_at(t0 + ms(1100)), "scaduta");
        assert!(radio.is_on(), "scaduta la prenotazione, il PTT torna come lo chiede la trasmissione");
        let s = lock.snapshot();
        assert_eq!((s.inhibited, s.hold_rx, s.expiries), (false, 2, 1));
    }

    #[test]
    fn the_ptt_asked_while_inhibited_stays_off_until_the_release() {
        let t0 = Instant::now();
        let radio = Arc::new(NoPtt::new());
        let lock = TxInterlock::new();
        lock.attach(radio.clone());
        lock.command_at("AMP", 30_000, "", t0);
        lock.set(true).expect("ptt");
        assert!(!radio.is_on());
        assert_eq!(lock.snapshot().holder, "AMP", "senza stazione si mostra il controller");
        lock.command_at("ALTRO", 0, "", t0 + ms(10));
        assert!(!radio.is_on(), "il rilascio di un altro controller non toglie la prenotazione");
        lock.command_at("AMP", 0, "", t0 + ms(20));
        assert!(radio.is_on());
        lock.set(false).expect("ptt");
        assert!(!radio.is_on());
        let s = lock.snapshot();
        assert_eq!((s.hold_rx, s.release_rx, s.expiries), (1, 2, 0));
    }

    #[test]
    fn holds_combine_with_or_and_the_65th_controller_joins_the_common_hold() {
        let t0 = Instant::now();
        let mut leases = InhibitLeases::default();
        for i in 0..MAX_TRACKED_HOLDS {
            leases.command(&format!("C{i}"), 1200, &format!("S{i}"), t0);
        }
        leases.command("C0", 2400, "S0", t0);
        leases.command("OLTRE", 1800, "OLTRE", t0);
        leases.command("OLTRE", 0, "OLTRE", t0);
        assert!(!leases.snapshot().holder.contains("OLTRE"), "la prenotazione comune non ha nome");
        assert!(leases.expire(t0 + ms(1300)));
        assert_eq!(leases.snapshot().expiries, 63, "63 scadute, C0 rinnovata");
        assert!(leases.is_inhibited());
        leases.command("C0", 0, "", t0 + ms(1400));
        assert!(leases.is_inhibited(), "il rilascio non toglie la prenotazione comune");
        assert!(leases.expire(t0 + ms(1800)));
        assert!(!leases.is_inhibited());
        assert_eq!(leases.snapshot().expiries, 64, "la prenotazione comune conta una volta");
        assert_eq!(leases.next_expiry(), None);
    }

    #[test]
    fn invalid_commands_are_counted_and_holders_sorted() {
        let t0 = Instant::now();
        let mut leases = InhibitLeases::default();
        leases.invalid(3);
        leases.command("b", 500, "zeta", t0);
        leases.command("a", 500, "Alfa", t0);
        leases.command("c", 500, "alfa", t0);
        leases.command("d", 500, "zeta", t0);
        let s = leases.snapshot();
        assert_eq!((s.invalid, s.holder.as_str()), (3, "Alfa, alfa, zeta"));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fast_tracker() -> Arc<ReleaseTracker> {
        let t = ReleaseTracker::new();
        t.set_timing(ReleaseTiming::fast());
        t
    }

    #[test]
    fn not_released_text_says_the_cause_once() {
        let from_cat = TxError::Ptt("PTT non spento: Errore sulla porta seriale".into());
        assert_eq!(not_released_text(&from_cat), "PTT non spento: Errore sulla porta seriale");
        assert_eq!(not_released_text(&TxError::Ptt("OmniRig non risponde".into())), "PTT non spento: OmniRig non risponde");
        assert_eq!(not_released_text(&TxError::Busy), format!("PTT non spento: {}", TxError::Busy));
    }
    use crate::ptt_release::ReleaseTiming;
    use std::{
        io::{BufRead, Write},
        net::{SocketAddr, TcpListener},
        sync::mpsc,
    };

    /// Un finto rigctld: ricorda i comandi e risponde `RPRT 0`, salvo per quelli per cui `reply` dice altro.
    fn fake_server(reply: fn(&str) -> &'static str) -> (SocketAddr, mpsc::Receiver<String>) {
        let listener = TcpListener::bind("127.0.0.1:0").expect("porta");
        let addr = listener.local_addr().expect("indirizzo");
        let (tx, rx) = mpsc::channel();
        std::thread::spawn(move || {
            let Ok((stream, _)) = listener.accept() else { return };
            let mut reader = std::io::BufReader::new(stream.try_clone().expect("clone"));
            let mut writer = stream;
            let mut line = String::new();
            while reader.read_line(&mut line).unwrap_or(0) > 0 {
                let cmd = line.trim().to_string();
                line.clear();
                // il client chiede da solo, al collegamento, se rigctld e' in modalita' VFO: qui rispondiamo di no e non lo contiamo
                if cmd == "\\chk_vfo" {
                    let _ = writer.write_all(b"0\n");
                    continue;
                }
                let _ = tx.send(cmd.clone());
                if cmd == "q" {
                    return;
                }
                let _ = writer.write_all(reply(&cmd).as_bytes());
            }
        });
        (addr, rx)
    }

    fn client(addr: SocketAddr) -> Arc<RigctldClient> {
        Arc::new(RigctldClient::connect(addr, Duration::from_secs(2)).expect("connessione"))
    }

    #[test]
    fn hamlib_ptt_sends_t1_and_t0_and_tracks_the_state() {
        let (addr, seen) = fake_server(|_| "RPRT 0\n");
        let ptt = HamlibPtt::new(client(addr));
        assert!(!ptt.is_on());
        ptt.set(true).expect("T 1");
        assert!(ptt.is_on());
        ptt.set(false).expect("T 0");
        assert!(!ptt.is_on());
        assert_eq!(seen.try_iter().collect::<Vec<_>>(), ["T 1", "T 0"]);
    }

    #[test]
    fn the_guard_turns_the_ptt_off_when_dropped() {
        let (addr, seen) = fake_server(|_| "RPRT 0\n");
        let ptt: Arc<dyn Ptt> = Arc::new(HamlibPtt::new(client(addr)));
        {
            let _guard = PttGuard::engage(Arc::clone(&ptt), fast_tracker()).expect("acceso");
            assert!(ptt.is_on());
        }
        assert!(!ptt.is_on());
        assert_eq!(seen.try_iter().collect::<Vec<_>>(), ["T 1", "T 0"]);
    }

    #[test]
    fn the_guard_turns_the_ptt_off_on_a_panic() {
        let (addr, seen) = fake_server(|_| "RPRT 0\n");
        let ptt: Arc<dyn Ptt> = Arc::new(HamlibPtt::new(client(addr)));
        let inner = Arc::clone(&ptt);
        let result = std::thread::spawn(move || {
            let _guard = PttGuard::engage(inner, fast_tracker()).expect("acceso");
            panic!("prova");
        })
        .join();
        assert!(result.is_err());
        assert!(!ptt.is_on());
        assert_eq!(seen.try_iter().collect::<Vec<_>>(), ["T 1", "T 0"]);
    }

    #[test]
    fn a_refused_t1_is_an_error_and_an_off_is_still_sent() {
        let (addr, seen) = fake_server(|cmd| if cmd == "T 1" { "RPRT -9\n" } else { "RPRT 0\n" });
        let ptt: Arc<dyn Ptt> = Arc::new(HamlibPtt::new(client(addr)));
        let err = PttGuard::engage(Arc::clone(&ptt), fast_tracker()).err().expect("doveva fallire");
        assert!(matches!(err, TxError::Ptt(_)), "{err}");
        assert!(!ptt.is_on());
        assert_eq!(seen.try_iter().collect::<Vec<_>>(), ["T 1", "T 0"]);
    }

    #[test]
    fn a_guard_that_falls_stops_the_audio_before_the_first_off() {
        use std::sync::atomic::AtomicBool;
        type Order = Arc<std::sync::Mutex<Vec<&'static str>>>;
        struct Spy(Order, AtomicBool);
        impl Ptt for Spy {
            fn set(&self, on: bool) -> Result<(), TxError> {
                self.0.lock().expect("ordine").push(if on { "on" } else { "off" });
                self.1.store(on, std::sync::atomic::Ordering::SeqCst);
                Ok(())
            }
            fn is_on(&self) -> bool {
                self.1.load(std::sync::atomic::Ordering::SeqCst)
            }
        }
        let order: Order = Arc::new(std::sync::Mutex::new(Vec::new()));
        let ptt: Arc<dyn Ptt> = Arc::new(Spy(Arc::clone(&order), AtomicBool::new(false)));
        let stop_order = Arc::clone(&order);
        {
            // l'uscita da `execute` con un errore lascia cadere la guardia con il PTT acceso
            let _guard = PttGuard::engage(ptt, fast_tracker()).expect("acceso").with_stop_audio(Box::new(move || stop_order.lock().expect("ordine").push("stop")));
        }
        assert_eq!(*order.lock().expect("ordine"), ["on", "stop", "off"]);
    }

    #[test]
    fn a_failing_off_climbs_the_ladder_and_leaves_the_state_on() {
        let (addr, seen) = fake_server(|cmd| if cmd == "T 0" { "RPRT -5\n" } else { "RPRT 0\n" });
        let ptt = HamlibPtt::new(client(addr));
        ptt.set(true).expect("T 1");
        let mut guard = PttGuard { ptt: Arc::new(ptt), tracker: fast_tracker(), armed: true, stop_audio: None };
        assert!(guard.release().is_err());
        assert!(guard.ptt.is_on(), "lo stato deve restare acceso: lo spegnimento e' fallito");
        assert!(guard.armed);
        let sent: Vec<String> = seen.try_iter().collect();
        assert_eq!(sent.iter().filter(|c| *c == "T 0").count(), 5, "un tentativo per gradino della scala: {sent:?}");
        assert!(guard.tracker.is_stuck());
        guard.armed = false; // niente tentativi in piu' alla chiusura della prova
    }

    #[test]
    fn a_dead_daemon_is_an_error_not_a_hang() {
        let (addr, _seen) = fake_server(|_| "RPRT 0\n");
        let c = client(addr);
        let ptt = HamlibPtt::new(Arc::clone(&c));
        // il finto server si ferma alla chiusura della connessione: qui la si rompe chiedendo `q`
        c.quit();
        std::thread::sleep(Duration::from_millis(100));
        assert!(ptt.set(true).is_err());
        assert!(!ptt.is_on());
    }

    #[test]
    fn no_ptt_tracks_the_state_for_vox() {
        let ptt = NoPtt::new();
        ptt.set(true).expect("ok");
        assert!(ptt.is_on());
        ptt.set(false).expect("ok");
        assert!(!ptt.is_on());
    }

    #[test]
    fn the_omnirig_ptt_accepts_only_rig_1_or_2() {
        assert!(OmniRigPtt::new(0).is_err());
        assert!(OmniRigPtt::new(3).is_err());
        assert!(OmniRigPtt::new(2).is_ok());
        assert!(OmniRigPtt::new(1).is_ok());
    }
}
