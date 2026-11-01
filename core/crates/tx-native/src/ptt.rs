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

use rig_control::{hamlib::RigctldClient, session::SharedSession};

use crate::error::TxError;

/// Un comando di trasmissione. `set` puo' bloccare per qualche istante (porta seriale, PowerShell): chi lo usa lo chiama da un thread dedicato.
pub trait Ptt: Send + Sync {
    fn set(&self, on: bool) -> Result<(), TxError>;
    /// Lo stato che Aurora crede di aver impostato (acceso se l'ultimo `set(true)` e' riuscito e non c'e' stato un `set(false)` riuscito).
    fn is_on(&self) -> bool;
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
}

/// `PM_RX` e `PM_TX` di OmniRig (valori della sua libreria dei tipi).
const PM_RX: u32 = 0x0020_0000;
const PM_TX: u32 = 0x0040_0000;

/// Lo script che imposta `Rig<n>.Tx`. Contiene solo il numero della radio (1 o 2) e una costante: niente testo dell'utente.
/// L'oggetto COM si crea una volta sola nella sessione PowerShell; se qualcosa va storto si butta e si ricrea al comando dopo.
pub fn omnirig_ptt_script(rig: u8, on: bool) -> Result<String, TxError> {
    if !(1..=2).contains(&rig) {
        return Err(TxError::Ptt(format!("radio OmniRig non valida: {rig}")));
    }
    let value = if on { PM_TX } else { PM_RX };
    let check = if on { "if ($r.Status -ne 4) { throw 'radio non in linea' }; ".to_string() } else { String::new() };
    Ok(format!(
        "try {{ if (-not $global:AuroraOmni) {{ $global:AuroraOmni = New-Object -ComObject OmniRig.OmniRigX }}; \
         $r = $global:AuroraOmni.Rig{rig}; {check}$r.Tx = {value}; 'ok' }} catch {{ $global:AuroraOmni = $null; throw }}"
    ))
}

/// Sessione PowerShell dedicata al PTT (separata da quelle dei comandi a JTDX): aperta una volta, poi ogni comando costa pochi decimi.
static PTT_SESSION: SharedSession = SharedSession::new();

/// Tempo massimo di un comando PTT via OmniRig.
const OMNIRIG_TIMEOUT: Duration = Duration::from_secs(5);

/// PTT via OmniRig (`Rig1.Tx` / `Rig2.Tx`), con PowerShell nascosto. Solo Windows.
pub struct OmniRigPtt {
    rig: u8,
    on: AtomicBool,
}

impl OmniRigPtt {
    /// `rig` e' 1 o 2 (le due radio di OmniRig).
    pub fn new(rig: u8) -> Result<OmniRigPtt, TxError> {
        omnirig_ptt_script(rig, false)?;
        Ok(OmniRigPtt { rig, on: AtomicBool::new(false) })
    }

    /// Apre la sessione PowerShell e l'oggetto OmniRig in anticipo, cosi' il primo PTT non paga l'avvio.
    pub fn warm_up(&self) -> Result<(), TxError> {
        PTT_SESSION.run_with_timeout("'ok'", OMNIRIG_TIMEOUT).map(|_| ()).map_err(|e| TxError::Ptt(e.0))
    }
}

impl Ptt for OmniRigPtt {
    fn set(&self, on: bool) -> Result<(), TxError> {
        let script = omnirig_ptt_script(self.rig, on)?;
        PTT_SESSION.run_with_timeout(&script, OMNIRIG_TIMEOUT).map_err(|e| TxError::Ptt(e.0))?;
        self.on.store(on, Ordering::SeqCst);
        Ok(())
    }

    fn is_on(&self) -> bool {
        self.on.load(Ordering::SeqCst)
    }
}

/// Spegne il PTT con qualche tentativo (la porta seriale o OmniRig possono essere occupate per un istante).
pub fn release_ptt(ptt: &dyn Ptt) -> Result<(), TxError> {
    let mut last = TxError::Ptt("nessun tentativo".into());
    for attempt in 0..3 {
        match ptt.set(false) {
            Ok(()) => return Ok(()),
            Err(e) => {
                tracing::warn!("PTT: spegnimento fallito (tentativo {}): {e}", attempt + 1);
                last = e;
                std::thread::sleep(Duration::from_millis(20));
            }
        }
    }
    Err(last)
}

/// Tiene acceso il PTT finche' vive: cadendo (anche per un panico) lo spegne.
pub struct PttGuard {
    ptt: Arc<dyn Ptt>,
    armed: bool,
}

impl PttGuard {
    /// Accende il PTT. Se il comando fallisce si prova comunque a spegnere (la risposta potrebbe essersi persa a radio gia' in trasmissione).
    pub fn engage(ptt: Arc<dyn Ptt>) -> Result<PttGuard, TxError> {
        let mut guard = PttGuard { ptt, armed: true };
        match guard.ptt.set(true) {
            Ok(()) => Ok(guard),
            Err(e) => {
                let _ = guard.release();
                Err(e)
            }
        }
    }

    /// Spegne il PTT. Se non ci riesce resta armato: lo riprovera' `Drop`.
    pub fn release(&mut self) -> Result<(), TxError> {
        release_ptt(&*self.ptt)?;
        self.armed = false;
        Ok(())
    }
}

impl Drop for PttGuard {
    fn drop(&mut self) {
        if self.armed
            && let Err(e) = release_ptt(&*self.ptt)
        {
            tracing::error!("PTT non spento: {e}");
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
}

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
        TxInterlock { state: std::sync::Mutex::new(InterlockState { leases: InhibitLeases::default(), inner: None, requested: false, keyed: false }) }
    }

    /// Il PTT vero (cambia quando si ricostruisce la catena di trasmissione). Quello di prima, se acceso, si spegne.
    pub fn attach(&self, inner: Arc<dyn Ptt>) {
        let mut s = crate::lock(&self.state);
        if let Some(old) = s.inner.take()
            && s.keyed
            && let Err(e) = release_ptt(&*old)
        {
            tracing::error!("PTT precedente non spento: {e}");
        }
        s.inner = Some(inner);
        s.requested = false;
        s.keyed = false;
    }

    /// Porta la radio allo stato voluto: acceso solo se chiesto e senza blocchi.
    fn apply(s: &mut InterlockState) -> Result<(), TxError> {
        let want = s.requested && !s.leases.is_inhibited();
        if want == s.keyed {
            return Ok(());
        }
        let Some(inner) = s.inner.clone() else { return Ok(()) };
        if want {
            inner.set(true)?;
            s.keyed = true;
        } else {
            release_ptt(&*inner)?;
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

    fn is_on(&self) -> bool {
        crate::lock(&self.state).requested
    }
}

#[cfg(test)]
mod interlock_tests {
    use super::*;

    fn ms(n: u64) -> Duration {
        Duration::from_millis(n)
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
            let _guard = PttGuard::engage(Arc::clone(&ptt)).expect("acceso");
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
            let _guard = PttGuard::engage(inner).expect("acceso");
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
        let err = PttGuard::engage(Arc::clone(&ptt)).err().expect("doveva fallire");
        assert!(matches!(err, TxError::Ptt(_)), "{err}");
        assert!(!ptt.is_on());
        assert_eq!(seen.try_iter().collect::<Vec<_>>(), ["T 1", "T 0"]);
    }

    #[test]
    fn a_failing_off_is_retried_and_leaves_the_state_on() {
        let (addr, seen) = fake_server(|cmd| if cmd == "T 0" { "RPRT -5\n" } else { "RPRT 0\n" });
        let ptt = HamlibPtt::new(client(addr));
        ptt.set(true).expect("T 1");
        let mut guard = PttGuard { ptt: Arc::new(ptt), armed: true };
        assert!(guard.release().is_err());
        assert!(guard.ptt.is_on(), "lo stato deve restare acceso: lo spegnimento e' fallito");
        assert!(guard.armed);
        let sent: Vec<String> = seen.try_iter().collect();
        assert_eq!(sent.iter().filter(|c| *c == "T 0").count(), 3, "{sent:?}");
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
    fn the_omnirig_script_sets_tx_and_rx_on_the_chosen_rig() {
        let on = omnirig_ptt_script(1, true).expect("script");
        assert!(on.contains("Rig1") && on.contains("$r.Tx = 4194304") && on.contains("Status -ne 4"), "{on}");
        let off = omnirig_ptt_script(2, false).expect("script");
        assert!(off.contains("Rig2") && off.contains("$r.Tx = 2097152") && !off.contains("Status"), "{off}");
        assert!(omnirig_ptt_script(0, true).is_err());
        assert!(omnirig_ptt_script(3, false).is_err());
        assert!(OmniRigPtt::new(3).is_err());
        assert!(OmniRigPtt::new(1).is_ok());
    }
}
