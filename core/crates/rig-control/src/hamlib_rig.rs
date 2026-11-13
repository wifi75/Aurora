//! La radio comandata con Hamlib: un thread di supervisione tiene aperto `rigctld`, legge la radio a intervalli (polling) e riapre la
//! connessione se si guasta. Le regole (PTT, letture, tolleranza ai guasti, riconnessione) sono quelle ricavate dall'analisi di Decodium,
//! `docs/analisi-decodium-cat.md`, sezioni 1 e 6:
//! - subito dopo l'apertura il PTT si spegne, e per 5 s non si accetta nessun PTT acceso;
//! - in trasmissione non si leggono frequenza e modo (competono con il PTT sullo stesso bus) e il polling va a un quarto dell'intervallo;
//! - un guasto di lettura non spegne il CAT: 3 consecutivi (8 se sono timeout o errori di porta), poi riconnessione a gradini;
//! - il PTT si spegne sempre alla chiusura, e se il comando di spegnimento fallisce lo stato resta «acceso»: si dice la verita'.

use std::{
    sync::{
        atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering},
        Arc, Mutex,
    },
    thread::JoinHandle,
    time::{Duration, Instant},
};

use serde::Serialize;

use crate::{
    // `friendly_error` di `busy_port`: lascia intatta la diagnosi di una porta occupata (dice gia' quale programma la tiene)
    busy_port::friendly_error,
    cat_config::{is_io_failure, is_serial_port, split_tx_dial_hz, PttMethod, RigConfig, SplitMode},
    hamlib::{Rigctld, RigctldClient},
    meters::{self, Level, LevelMemory, Meters, TX_LEVELS},
    Rig, RigError,
};

enum Backing {
    Daemon(Rigctld),
    Client(RigctldClient),
}

/// Una connessione aperta verso la radio: un `rigctld` lanciato da noi (che si ferma quando la connessione cade) o un client gia' collegato.
pub struct Link(Backing);

impl Link {
    pub fn from_daemon(daemon: Rigctld) -> Link {
        Link(Backing::Daemon(daemon))
    }

    pub fn from_client(client: RigctldClient) -> Link {
        Link(Backing::Client(client))
    }

    fn client(&self) -> &RigctldClient {
        match &self.0 {
            Backing::Daemon(d) => &d.client,
            Backing::Client(c) => c,
        }
    }

    fn alive(&self) -> bool {
        match &self.0 {
            Backing::Daemon(d) => d.is_running(),
            Backing::Client(_) => true,
        }
    }

    fn note(&self) -> String {
        match &self.0 {
            Backing::Daemon(d) => d.stderr_tail(),
            Backing::Client(_) => String::new(),
        }
    }
}

/// Come aprire la connessione (ogni chiamata e' un nuovo tentativo: si lancia `rigctld`, o nelle prove ci si collega a un finto).
pub type Connector = Arc<dyn Fn() -> Result<Link, RigError> + Send + Sync>;

/// Le attese del supervisore.
#[derive(Debug, Clone)]
pub struct Timing {
    /// Intervallo di lettura a riposo.
    pub poll: Duration,
    /// Intervallo minimo in trasmissione (un quarto di `poll`, mai meno di questo).
    pub tx_poll_min: Duration,
    /// Letture rapide dopo un cambio di PTT.
    pub quick_reads: [Duration; 2],
    /// Dopo il collegamento non si accetta il PTT acceso per questo tempo.
    pub startup_block: Duration,
    /// Pausa dopo ogni cambio di PTT (alcune radio non elaborano CAT mentre passano tra Tx e Rx).
    pub ptt_pause: Duration,
    /// Attesa prima di controllare che la radio abbia raggiunto la frequenza.
    pub qsy_check: Duration,
    /// Riconnessione: attesa `reconnect_step x numero del tentativo`, con tetto.
    pub reconnect_step: Duration,
    pub reconnect_cap: Duration,
    pub max_reconnects: u32,
    /// Guasti di lettura consecutivi tollerati, in generale e se sono timeout o errori di porta.
    pub tolerate: u32,
    pub tolerate_io: u32,
}

impl Timing {
    /// I tempi di Decodium: polling scelto dall'utente (le seriali non sotto 2 s), quarto in trasmissione ma non sotto 250 ms, letture a 300 e 800 ms.
    pub fn standard(config: &RigConfig) -> Timing {
        Timing {
            poll: Duration::from_secs(u64::from(config.effective_poll_secs())),
            tx_poll_min: Duration::from_millis(250),
            quick_reads: [Duration::from_millis(300), Duration::from_millis(800)],
            startup_block: Duration::from_secs(5),
            ptt_pause: Duration::from_millis(100),
            qsy_check: Duration::from_millis(500),
            reconnect_step: Duration::from_secs(5),
            reconnect_cap: Duration::from_secs(30),
            max_reconnects: 5,
            tolerate: 3,
            tolerate_io: 8,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum LinkState {
    Disconnected,
    Connecting,
    Connected,
    Reconnecting,
    Error,
}

/// Lo stato osservabile della radio.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct RigStatus {
    pub state: LinkState,
    /// Testo per l'utente (errore tradotto, avanzamento della riconnessione).
    pub message: String,
    pub frequency_hz: Option<u64>,
    pub mode: Option<String>,
    /// Larghezza del filtro della radio (Hz), come la da' Hamlib con il modo; 0 o assente se la radio non la dice.
    pub passband_hz: Option<u32>,
    /// I dati in piu' (con «Leggi anche gli altri dati della radio» acceso).
    pub extras: crate::extras::RigExtras,
    pub ptt: bool,
    pub split: Option<bool>,
    /// Guasti di lettura consecutivi.
    pub failures: u32,
    pub reconnect_attempt: u32,
    /// Il PTT acceso e' ancora bloccato dall'attesa dopo il collegamento.
    pub ptt_blocked: bool,
    pub updated_ms: u64,
}

fn now_ms() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_millis() as u64)
}

fn unsupported(message: &str) -> bool {
    message.contains("non implementata") || message.contains("non disponibile")
}

/// Quanti guasti consecutivi si tollerano per questo messaggio.
pub fn failure_limit(timing: &Timing, message: &str) -> u32 {
    if is_io_failure(message) { timing.tolerate_io } else { timing.tolerate }
}

struct Inner {
    /// «Accendi» gia' mandato in questa sessione di Aurora (si accende una volta sola, al primo collegamento).
    powered_on: AtomicBool,
    config: RigConfig,
    connector: Connector,
    timing: Timing,
    link: Mutex<Option<Arc<Link>>>,
    status: Mutex<RigStatus>,
    stop: AtomicBool,
    ptt_blocked_until: Mutex<Option<Instant>>,
    quick: Mutex<Vec<Instant>>,
    ptt_readable: AtomicBool,
    /// Trasmissione comandata fuori dal CAT (PTT su RTS/DTR di un'altra porta): la dice il motore, cosi' i misuratori passano subito a
    /// potenza/ROS/ALC invece di aspettare la lettura del PTT dalla radio (ogni `poll_secs`), che non sempre vede quel PTT.
    external_tx: AtomicBool,
    tx_audio_hz: AtomicU32,
    rx_dial: AtomicU64,
    emulate_restore: Mutex<Option<u64>>,
    /// Ruoli dei VFO in split invertiti (l'operatore riceve sul VFO B, quindi si trasmette sul VFO A). Si osserva solo fuori dalla
    /// trasmissione, come `HamlibVfoRoleState` di WSJT-X 3.2 (`Transceiver/HamlibVfoRoleState.hpp`, GPL-3.0): durante il PTT molte radio
    /// riportano il VFO di trasmissione come attivo e i ruoli si scambierebbero.
    vfo_reversed: AtomicBool,
    /// Split DX acceso (doppio clic su uno spot del cluster con «UP 5», «QSX ...»): il VFO di trasmissione l'ha scelto l'operatore e lo split
    /// di FT8 (XIT) non lo tocca finche' resta acceso. Si spegne al cambio di frequenza, da «Split DX: spegni» e alla chiusura.
    dx_split: AtomicBool,
    /// La frequenza di trasmissione dello Split DX acceso (Hz, 0 = nessuno): serve a rimetterlo dopo una riconnessione.
    dx_split_tx: AtomicU64,
    /// Lo split di FT8 (XIT, «Split: Radio») e' acceso sulla radio perche' l'ha acceso Aurora prima di una trasmissione del motore. Si
    /// spegne al cambio di frequenza, prima di una trasmissione libera (modi testuali, SSTV) e alla chiusura.
    xit_split: AtomicBool,
    /// Lo Split DX e' stato spento a radio scollegata: la radio puo' averlo ancora acceso, lo si spegne appena torna.
    split_off_pending: AtomicBool,
    /// Cresce a ogni comando: una lettura iniziata prima di un comando non ne cancella l'effetto.
    epoch: AtomicU64,
    /// Conta i cambi di frequenza chiesti: un cambio che ne vede arrivare uno piu' recente non insiste sulla sua frequenza.
    qsy_seq: AtomicU64,
    /// I misuratori: cosa la radio non da', ultima lettura, turno di lettura.
    meter: Mutex<MeterPoll>,
    extras: Mutex<crate::extras::ExtrasPoll>,
}

/// Frequenza audio massima accettata per la trasmissione (Hz): oltre e' un valore sbagliato, non un offset.
const MAX_AUDIO_HZ: u32 = 6000;

/// Frequenza accettabile per la radio: niente zero (un valore perso) e niente oltre i 300 GHz (un overflow). Dalle note di WSJT-X 3.2:
/// «Invalid frequencies [...] are rejected rather than becoming zero or overflowing».
pub fn valid_dial_hz(hz: u64) -> bool {
    hz > 0 && hz <= 300_000_000_000
}

/// Il modo letto dalla radio va gia' bene per quello richiesto? `PKTUSB` e' soddisfatto anche da `USB-D1/D2/D3` e `PKTLSB` da
/// `LSB-D1/D2/D3`: cosi' il profilo DATA2 o DATA3 scelto dall'operatore resta (radio con piu' profili dati, come la TS-990S).
/// Da `HamlibMode::satisfies_request` di WSJT-X 3.2 (`Transceiver/HamlibMode.cpp`, GPL-3.0).
pub fn mode_satisfies(requested: &str, current: &str) -> bool {
    let (r, c) = (requested.trim().to_uppercase(), current.trim().to_uppercase());
    if r == c {
        return true;
    }
    match r.as_str() {
        "PKTUSB" => matches!(c.as_str(), "USB-D1" | "USB-D2" | "USB-D3" | "USBD1" | "USBD2" | "USBD3"),
        "PKTLSB" => matches!(c.as_str(), "LSB-D1" | "LSB-D2" | "LSB-D3" | "LSBD1" | "LSBD2" | "LSBD3"),
        _ => false,
    }
}

/// Il VFO attivo e' il B (o il «Sub»)?
fn vfo_is_b(vfo: &str) -> bool {
    matches!(vfo.trim().to_uppercase().as_str(), "VFOB" | "B" | "SUB")
}

/// Ogni quanto si legge il misuratore in ricezione e in trasmissione.
const METER_RX_EVERY: Duration = Duration::from_millis(250);
const METER_TX_EVERY: Duration = Duration::from_millis(200);

/// Lo stato della lettura dei misuratori (vedi `HamlibRig::meters`).
#[derive(Default)]
struct MeterPoll {
    memory: LevelMemory,
    cache: Meters,
    last: Option<Instant>,
    /// Ultimo stato letto: al cambio tra ricezione e trasmissione si buttano le letture dell'altro stato.
    was_tx: bool,
    /// Prossimo livello da leggere sulle seriali lente (un comando per giro).
    turn: usize,
    /// Fondo scala della potenza in watt, con i MHz a cui e' stato letto, per convertire `RFPOWER_METER`.
    full_scale: Option<(u64, f32)>,
    full_scale_tries: u8,
}

/// Quanti dati in piu' (PRE, ATT, AGC, NB...) si leggono a ogni giro: pochi sulle seriali lente per non togliere tempo al PTT.
fn extras_per_read(config: &RigConfig) -> usize {
    if slow_port(config) {
        1
    } else if is_serial_port(&config.port) && config.baud < 19200 {
        3
    } else {
        6
    }
}

/// Seriali a 9600 baud o meno (o con la velocita' del modello, che puo' essere bassa): una raffica di comandi toglierebbe tempo al PTT.
fn slow_port(config: &RigConfig) -> bool {
    is_serial_port(&config.port) && (config.baud == 0 || config.baud <= 9600)
}

/// I passi di lettura in trasmissione: la potenza (per prima: e' la lettura principale) e poi i livelli di `TX_LEVELS`.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Step {
    Power,
    Level(Level),
}

impl Inner {
    fn update(&self, edit: impl FnOnce(&mut RigStatus)) {
        if let Ok(mut s) = self.status.lock() {
            edit(&mut s);
            s.updated_ms = now_ms();
        }
    }

    fn snapshot(&self) -> RigStatus {
        let mut s = self.status.lock().map(|s| s.clone()).unwrap_or_else(|_| RigStatus {
            state: LinkState::Error,
            message: "stato non disponibile".into(),
            frequency_hz: None,
            mode: None,
            passband_hz: None,
            extras: crate::extras::RigExtras::default(),
            ptt: false,
            split: None,
            failures: 0,
            reconnect_attempt: 0,
            ptt_blocked: false,
            updated_ms: 0,
        });
        s.ptt_blocked = self.ptt_blocked().is_some();
        s
    }

    fn ptt_blocked(&self) -> Option<Duration> {
        let until = (*self.ptt_blocked_until.lock().ok()?)?;
        until.checked_duration_since(Instant::now()).filter(|d| !d.is_zero())
    }

    fn link(&self) -> Option<Arc<Link>> {
        self.link.lock().ok().and_then(|l| l.clone())
    }

    fn take_link(&self) -> Option<Arc<Link>> {
        self.link.lock().ok().and_then(|mut l| l.take())
    }

    fn connected_link(&self) -> Result<Arc<Link>, RigError> {
        if self.snapshot().state != LinkState::Connected {
            return Err(RigError("radio non collegata".into()));
        }
        self.link().ok_or_else(|| RigError("radio non collegata".into()))
    }

    fn transmitting(&self) -> bool {
        self.external_tx.load(Ordering::SeqCst) || self.status.lock().is_ok_and(|s| s.ptt)
    }

    fn queue_quick_reads(&self) {
        if let Ok(mut q) = self.quick.lock() {
            let now = Instant::now();
            q.extend(self.timing.quick_reads.iter().map(|d| now + *d));
        }
    }

    /// Dorme, ma torna subito se si chiude. `false` = si e' chiuso.
    fn sleep(&self, total: Duration) -> bool {
        let until = Instant::now() + total;
        loop {
            if self.stop.load(Ordering::SeqCst) {
                return false;
            }
            let left = until.saturating_duration_since(Instant::now());
            if left.is_zero() {
                return true;
            }
            std::thread::sleep(left.min(Duration::from_millis(10)));
        }
    }

    /// Apre la connessione: PTT spento subito, poi la prima lettura (se la radio non risponde l'apertura non e' riuscita).
    fn open(&self) -> Result<(), RigError> {
        self.update(|s| {
            // durante la riconnessione lo stato resta «riconnessione» (con il conto dei tentativi)
            if s.state != LinkState::Reconnecting {
                s.state = LinkState::Connecting;
                s.message = "Collegamento in corso".into();
            }
        });
        let link = Arc::new((self.connector)()?);
        // «Accendi» di JTDX: la radio si accende al primo collegamento (Hamlib aspetta da solo che sia pronta); errori solo nel registro
        // (l'accensione la fa Hamlib all'apertura della porta, `auto_power_on`: qui si annota solo che e' avvenuta)
        if self.config.power_on && !self.powered_on.swap(true, Ordering::SeqCst) {
            tracing::info!("radio collegata con «Accendi» (auto_power_on di Hamlib)");
        }
        // alcune radio trasmettono all'apertura della porta: PTT spento subito, errori ignorati (un VOX non ha PTT da spegnere)
        if let Err(e) = link.client().set_ptt(false) {
            tracing::debug!("PTT spento dopo l'apertura: {e}");
        }
        if !link.alive() {
            return Err(RigError(format!("rigctld terminato: {}", link.note())));
        }
        // rigctld che si chiude subito dopo l'avvio (radio spenta, modello o velocita' sbagliati) dava solo «connessione interrotta
        // forzatamente» nei rapporti (2026-11-12, piu' installazioni): se il processo e' uscito si dice e si aggiungono le sue ultime righe
        let freq = link.client().frequency().map_err(|e| {
            std::thread::sleep(Duration::from_millis(300));
            if !link.alive() {
                RigError(format!("rigctld terminato: {} ({})", link.note(), e.0))
            } else if crate::cat_config::is_connection_reset(&e.0) {
                // rigctld vivo che chiude subito il collegamento: non riesce ad aprire la radio (spenta, porta, velocita' o modello
                // sbagliati). Rapporti del 2026-11-12, cinque PC: prima solo «connessione interrotta forzatamente dall'host remoto»
                let tail = link.note();
                let tail = if tail.is_empty() { String::new() } else { format!("; rigctld: {tail}") };
                RigError(format!("rigctld ha chiuso il collegamento: la radio non risponde a Hamlib ({}{tail})", e.0))
            } else {
                e
            }
        })?;
        let mut mode = link.client().mode().ok();
        // il modo scelto in «Modo» (dati/USB) si scrive anche al collegamento, come WSJT-X (`RadioModePolicy`): prima solo al cambio di frequenza, e
        // dopo l'avvio sulla stessa frequenza si poteva trasmettere nel modo che la radio aveva (USB analogico, FM...)
        if let Some(wanted) = self.config.cat_mode.hamlib_name()
            && !mode.as_ref().is_some_and(|m| mode_satisfies(wanted, &m.0))
        {
            match link.client().set_mode(wanted, 0) {
                Ok(()) => mode = link.client().mode().ok().or(Some((wanted.to_string(), 0))),
                Err(e) => tracing::warn!("modo {wanted} non scritto al collegamento: {e}"),
            }
        }
        self.ptt_readable.store(true, Ordering::SeqCst);
        if let Ok(mut until) = self.ptt_blocked_until.lock() {
            *until = Some(Instant::now() + self.timing.startup_block);
        }
        self.rx_dial.store(freq, Ordering::SeqCst);
        if let Ok(mut meter) = self.meter.lock() {
            // un collegamento nuovo puo' essere un'altra radio: si riprova a chiedere tutti i livelli
            *meter = MeterPoll::default();
        }
        if let Ok(mut extras) = self.extras.lock() {
            *extras = crate::extras::ExtrasPoll::default();
        }
        // la potenza scelta l'ultima volta con + e − (la radio puo' ripartire con la sua, es. 99%)
        crate::extras::restore_power(link.client());
        if let Ok(mut slot) = self.link.lock() {
            *slot = Some(link);
        }
        // un ritorno alla frequenza di prima di uno split simulato non vale piu' dopo che la radio e' stata ricollegata
        if let Ok(mut slot) = self.emulate_restore.lock() {
            *slot = None;
        }
        self.update(|s| {
            s.state = LinkState::Connected;
            s.message.clear();
            s.frequency_hz = Some(freq);
            s.passband_hz = mode.as_ref().map(|m| m.1).filter(|&w| w > 0);
            s.mode = mode.map(|m| m.0);
            s.ptt = false;
            s.failures = 0;
            s.reconnect_attempt = 0;
        });
        // lo Split DX spento mentre la radio era scollegata (ritorno a FT8, cambio di frequenza): la radio puo' averlo ancora acceso
        if self.split_off_pending.swap(false, Ordering::SeqCst)
            && !self.dx_split.load(Ordering::SeqCst)
            && let Ok(link) = self.connected_link()
        {
            let rx_vfo = if self.vfo_reversed.load(Ordering::SeqCst) { "VFOB" } else { "VFOA" };
            match link.client().set_split(false, rx_vfo) {
                Ok(()) => self.update(|s| s.split = Some(false)),
                Err(e) => tracing::warn!("split non spento dopo la riconnessione: {}", friendly_error(&e.0)),
            }
        }
        // la radio riavviata ha perso anche lo Split DX: si rimette (l'indicatore del cluster lo dice ancora acceso e l'operatore crede di
        // trasmettere sul VFO B); se non si riesce si spegne il flag, cosi' torna lo split di FT8
        let dx_tx = self.dx_split_tx.load(Ordering::SeqCst);
        if self.dx_split.load(Ordering::SeqCst) && dx_tx != 0 && let Ok(link) = self.connected_link() {
            let tx_vfo = if self.vfo_reversed.load(Ordering::SeqCst) { "VFOA" } else { "VFOB" };
            match link.client().set_split_frequency(dx_tx).and_then(|()| link.client().set_split(true, tx_vfo)) {
                Ok(()) => self.update(|s| s.split = Some(true)),
                Err(e) => {
                    tracing::warn!("split DX non rimesso dopo la riconnessione: {}", friendly_error(&e.0));
                    self.dx_split.store(false, Ordering::SeqCst);
                }
            }
        }
        // lo split di FT8 («Split: Radio») NON si rimette alla riconnessione: si accende solo un attimo prima del PTT di una trasmissione
        // del motore (`set_tx_audio_frequency`). Prima qui lo si riaccendeva con l'ultima frequenza audio nota, e a ogni caduta breve del
        // CAT la radio si trovava in split sul VFO B anche dopo un cambio di frequenza che l'aveva spento o nei modi testuali, dove nessuno
        // l'aveva chiesto (segnalazione del 2026-11-12: «la radio va in split da sola» tornando da RTTY a FT8). `xit_split` resta com'era:
        // una radio che non si e' riavviata ha ancora lo split di prima, e il prossimo cambio di frequenza o trasmissione libera lo spegne
        Ok(())
    }

    /// Una lettura periodica. In trasmissione solo il PTT.
    fn poll_once(&self) -> Result<(), RigError> {
        let link = self.connected_link()?;
        if !link.alive() {
            return Err(RigError(format!("rigctld terminato: {}", link.note())));
        }
        let epoch = self.epoch.load(Ordering::SeqCst);
        let client = link.client();
        let (mut freq, mut mode, mut passband) = (None, None, None);
        if !self.transmitting() {
            freq = Some(client.frequency()?);
            // un errore sul modo non conta come guasto (`HamlibTransceiver.cpp` di WSJT-X: alcune Yaesu FTdx rispondono male): si tiene l'ultimo
            // modo noto. Prima tre errori di fila facevano ricollegare la radio
            if let Ok((m, width)) = client.mode() {
                mode = Some(m);
                passband = Some(width).filter(|&w| w > 0);
            }
        }
        let ptt = if self.ptt_readable.load(Ordering::SeqCst) {
            match client.ptt() {
                Ok(p) => Some(p),
                Err(e) if unsupported(&e.0) => {
                    self.ptt_readable.store(false, Ordering::SeqCst);
                    None
                }
                Err(e) => return Err(e),
            }
        } else {
            None
        };
        if let Some(f) = freq {
            self.rx_dial.store(f, Ordering::SeqCst);
        }
        // i dati in piu', a turno e solo in ricezione: uno per lettura sulle seriali lente, sei sulle veloci (da 19200 baud, rete o USB senza
        // seriale), cosi' i 16 valori del display si rinfrescano in circa tre letture invece di sei
        let extras = if self.config.read_extras && freq.is_some() {
            self.extras.lock().ok().map(|mut poll| {
                poll.read(client, extras_per_read(&self.config));
                poll.cache.clone()
            })
        } else {
            None
        };
        let fresh = self.epoch.load(Ordering::SeqCst) == epoch;
        self.update(|s| {
            s.failures = 0;
            if fresh {
                if freq.is_some() {
                    s.frequency_hz = freq;
                    if mode.is_some() {
                        s.mode = mode;
                        s.passband_hz = passband;
                    }
                    if let Some(x) = extras {
                        s.extras = x;
                    }
                }
                if let Some(p) = ptt {
                    s.ptt = p;
                }
            }
        });
        Ok(())
    }

    /// Aspetta il prossimo giro di lettura; `false` = si e' chiuso.
    fn wait_next(&self) -> bool {
        let delay = if self.transmitting() { (self.timing.poll / 4).max(self.timing.tx_poll_min) } else { self.timing.poll };
        let until = Instant::now() + delay;
        loop {
            if self.stop.load(Ordering::SeqCst) {
                return false;
            }
            let now = Instant::now();
            if now >= until {
                return true;
            }
            if let Ok(mut q) = self.quick.lock()
                && let Some(i) = q.iter().position(|t| *t <= now)
            {
                q.remove(i);
                return true;
            }
            std::thread::sleep((until - now).min(Duration::from_millis(10)));
        }
    }

    /// Riapre dopo un guasto, a gradini. `true` = di nuovo collegata.
    fn reconnect(&self, why: &str) -> bool {
        if let Some(link) = self.take_link()
            && self.transmitting()
        {
            // un solo tentativo: se la connessione e' morta ogni prova costa il tempo di attesa
            let _ = link.client().set_ptt(false);
        }
        let max = self.timing.max_reconnects;
        let mut last = why.to_string();
        for attempt in 1..=max {
            let wait = (self.timing.reconnect_step * attempt).min(self.timing.reconnect_cap);
            self.update(|s| {
                s.state = LinkState::Reconnecting;
                s.reconnect_attempt = attempt;
                s.message = format!("Collegamento perso: {} Nuovo tentativo {attempt} di {max} tra {} s.", friendly_error(&last), wait.as_secs_f32().ceil());
            });
            if !self.sleep(wait) {
                return false;
            }
            match self.open() {
                Ok(()) => return true,
                Err(e) => last = e.0,
            }
            if self.stop.load(Ordering::SeqCst) {
                return false;
            }
        }
        // finiti i tentativi a gradini si continua ogni `reconnect_step`, senza arrendersi
        loop {
            let wait = self.timing.reconnect_step;
            self.update(|s| {
                s.state = LinkState::Error;
                s.message = format!("{} Nuovo tentativo tra {} s.", friendly_error(&last), wait.as_secs_f32().ceil());
            });
            if !self.sleep(wait) || self.stop.load(Ordering::SeqCst) {
                return false;
            }
            match self.open() {
                Ok(()) => return true,
                Err(e) => last = e.0,
            }
        }
    }

    fn run(&self) {
        // se la radio (o il programma che la condivide, es. Log4OM) non c'e' ancora, si riprova da soli ogni `reconnect_step`
        // finche' non risponde o finche' non si preme Scollega
        while let Err(e) = self.open() {
            if self.stop.load(Ordering::SeqCst) {
                return;
            }
            let wait = self.timing.reconnect_step;
            // nel registro il motivo intero (le righe di rigctld): a schermo resta il messaggio breve
            tracing::warn!("CAT: collegamento con rigctld non riuscito: {}", e.0);
            self.update(|s| {
                s.state = LinkState::Error;
                s.message = format!("{} Nuovo tentativo tra {} s.", friendly_error(&e.0), wait.as_secs_f32().ceil());
            });
            if !self.sleep(wait) {
                return;
            }
        }
        let mut failures = 0u32;
        while self.wait_next() {
            let dead = self.link().is_some_and(|l| !l.alive());
            let outcome = if dead { Err(RigError(format!("rigctld terminato: {}", self.link().map(|l| l.note()).unwrap_or_default()))) } else { self.poll_once() };
            match outcome {
                Ok(()) => failures = 0,
                Err(e) => {
                    failures += 1;
                    self.update(|s| s.failures = failures);
                    tracing::debug!("lettura della radio fallita ({failures}): {e}");
                    if dead || failures >= failure_limit(&self.timing, &e.0) {
                        failures = 0;
                        if !self.reconnect(&e.0) {
                            return;
                        }
                    }
                }
            }
        }
    }

    /// Un livello letto e registrato; gli esiti negativi finiscono nella memoria dei livelli (senza errori per chi chiama).
    fn read_level(&self, client: &RigctldClient, poll: &mut MeterPoll, level: Level) -> Option<f32> {
        match client.level(level.name()) {
            Ok(v) => {
                poll.memory.ok(level);
                Some(v)
            }
            Err(e) => {
                if poll.memory.failed(level, &e) {
                    tracing::debug!("livello {} non letto, non si chiede piu': {e}", level.name());
                }
                None
            }
        }
    }

    /// La potenza in watt: `RFPOWER_METER_WATTS` se la radio lo da', altrimenti `RFPOWER_METER` (frazione) per il fondo scala di Hamlib.
    fn read_power(&self, client: &RigctldClient, poll: &mut MeterPoll) {
        if poll.memory.supported(Level::PowerWatts) {
            if let Some(w) = self.read_level(client, poll, Level::PowerWatts) {
                meters::store(&mut poll.cache, Level::PowerWatts, w);
                return;
            }
            if poll.memory.supported(Level::PowerWatts) {
                return;
            }
        }
        if !poll.memory.supported(Level::PowerFraction) {
            return;
        }
        let Some(fraction) = self.read_level(client, poll, Level::PowerFraction) else { return };
        let (hz, mode) = {
            let s = self.snapshot();
            (s.frequency_hz.unwrap_or_else(|| self.rx_dial.load(Ordering::SeqCst)), s.mode.unwrap_or_else(|| "USB".into()))
        };
        let mhz = hz / 1_000_000;
        let full = match poll.full_scale {
            Some((m, w)) if m == mhz => Some(w),
            _ if poll.full_scale_tries >= 3 => poll.full_scale.map(|(_, w)| w),
            _ => {
                poll.full_scale_tries += 1;
                match client.power_to_watts(1.0, hz.max(1), &mode) {
                    Ok(w) if w > 0.0 => {
                        poll.full_scale = Some((mhz, w));
                        poll.full_scale_tries = 0;
                        Some(w)
                    }
                    _ => poll.full_scale.map(|(_, w)| w),
                }
            }
        };
        match full {
            Some(w) => poll.cache.power_w = Some((fraction.clamp(0.0, 1.0) * w).max(0.0)),
            // senza il fondo scala la frazione non si puo' dire in watt: non si inventa
            None if poll.full_scale_tries >= 3 => poll.memory.mark_unsupported(Level::PowerFraction),
            None => {}
        }
    }

    /// I misuratori. In ricezione solo `STRENGTH`; in trasmissione solo i livelli di trasmissione. Ogni livello che la radio non da' si ricorda e non
    /// si chiede piu'. Sulle seriali lente (9600 baud o meno) un solo comando per chiamata, a turno.
    fn read_meters(&self) -> Result<Meters, RigError> {
        let link = self.connected_link()?;
        let tx = self.transmitting();
        let mut poll = self.meter.lock().map_err(|_| RigError("misuratori: stato avvelenato".into()))?;
        if tx != poll.was_tx {
            poll.was_tx = tx;
            poll.last = None;
            poll.turn = 0;
            if tx {
                poll.cache.s_points = None;
                poll.cache.s_over_db = None;
            } else {
                poll.cache = Meters { s_points: None, s_over_db: None, ..Meters::default() };
            }
        }
        let every = if tx { METER_TX_EVERY } else { METER_RX_EVERY };
        if poll.last.is_some_and(|t| t.elapsed() < every) {
            return Ok(poll.cache);
        }
        poll.last = Some(Instant::now());
        let client = link.client();
        let mut steps: Vec<Step> = Vec::new();
        if tx {
            if poll.memory.supported(Level::PowerWatts) || poll.memory.supported(Level::PowerFraction) {
                steps.push(Step::Power);
            }
            steps.extend(TX_LEVELS.iter().filter(|l| poll.memory.supported(**l)).map(|l| Step::Level(*l)));
        } else if poll.memory.supported(Level::Strength) {
            steps.push(Step::Level(Level::Strength));
        }
        if steps.is_empty() {
            return Ok(poll.cache);
        }
        if slow_port(&self.config) {
            let pick = steps[poll.turn % steps.len()];
            poll.turn = poll.turn.wrapping_add(1);
            steps = vec![pick];
        }
        for step in steps {
            match step {
                Step::Power => self.read_power(client, &mut poll),
                Step::Level(level) => {
                    if let Some(v) = self.read_level(client, &mut poll, level) {
                        meters::store(&mut poll.cache, level, v);
                    }
                }
            }
        }
        Ok(poll.cache)
    }

    /// Porta la radio sulla frequenza del VFO di trasmissione per lo split vero, poi accende lo split (per ultimo: alcune radio escono
    /// dallo split quando cambia il VFO di ricezione).
    fn apply_split(&self, link: &Link) -> Result<(), RigError> {
        // con lo Split DX acceso il VFO di trasmissione e' quello dello spot: lo split di FT8 non lo sposta
        if self.dx_split.load(Ordering::SeqCst) {
            return Ok(());
        }
        let (audio, dial) = (self.tx_audio_hz.load(Ordering::SeqCst), self.rx_dial.load(Ordering::SeqCst));
        if audio == 0 || dial == 0 {
            return Ok(());
        }
        if !self.transmitting()
            && let Ok(active) = link.client().vfo()
        {
            self.vfo_reversed.store(vfo_is_b(&active), Ordering::SeqCst);
        }
        let tx_vfo = if self.vfo_reversed.load(Ordering::SeqCst) { "VFOA" } else { "VFOB" };
        link.client().set_split_frequency(split_tx_dial_hz(dial, audio))?;
        link.client().set_split(true, tx_vfo)?;
        self.xit_split.store(true, Ordering::SeqCst);
        self.update(|s| s.split = Some(true));
        Ok(())
    }

    /// PTT spento con `tries` tentativi. `Ok` se la radio ha confermato.
    fn ptt_off(&self, link: &Link, tries: u32) -> Result<(), RigError> {
        let mut last = RigError("nessun tentativo".into());
        for i in 0..tries {
            match link.client().set_ptt(false) {
                Ok(()) => return Ok(()),
                Err(e) => last = e,
            }
            if i + 1 < tries {
                std::thread::sleep(self.timing.ptt_pause);
            }
        }
        Err(last)
    }
}

/// La radio comandata con Hamlib. Si collega da sola (in un thread) e resta osservabile con `status()`.
pub struct HamlibRig {
    inner: Arc<Inner>,
    thread: Mutex<Option<JoinHandle<()>>>,
}

impl HamlibRig {
    /// Avvia il supervisore: la connessione avviene in un thread, lo stato racconta come va.
    pub fn start(config: RigConfig, connector: Connector, timing: Timing) -> HamlibRig {
        let inner = Arc::new(Inner {
            powered_on: AtomicBool::new(false),
            config,
            connector,
            timing,
            link: Mutex::new(None),
            status: Mutex::new(RigStatus {
                state: LinkState::Connecting,
                message: "Collegamento in corso".into(),
                frequency_hz: None,
                mode: None,
                passband_hz: None,
                extras: crate::extras::RigExtras::default(),
                ptt: false,
                split: None,
                failures: 0,
                reconnect_attempt: 0,
                ptt_blocked: false,
                updated_ms: now_ms(),
            }),
            stop: AtomicBool::new(false),
            ptt_blocked_until: Mutex::new(None),
            quick: Mutex::new(Vec::new()),
            ptt_readable: AtomicBool::new(true),
            external_tx: AtomicBool::new(false),
            tx_audio_hz: AtomicU32::new(0),
            rx_dial: AtomicU64::new(0),
            emulate_restore: Mutex::new(None),
            vfo_reversed: AtomicBool::new(false),
            dx_split: AtomicBool::new(false),
            dx_split_tx: AtomicU64::new(0),
            xit_split: AtomicBool::new(false),
            split_off_pending: AtomicBool::new(false),
            epoch: AtomicU64::new(0),
            qsy_seq: AtomicU64::new(0),
            meter: Mutex::new(MeterPoll::default()),
            extras: Mutex::new(crate::extras::ExtrasPoll::default()),
        });
        let worker = Arc::clone(&inner);
        let handle = std::thread::Builder::new().name("aurora-cat".into()).spawn(move || worker.run()).ok();
        if handle.is_none() {
            inner.update(|s| {
                s.state = LinkState::Error;
                s.message = "thread del CAT non avviato".into();
            });
        }
        HamlibRig { inner, thread: Mutex::new(handle) }
    }

    pub fn config(&self) -> &RigConfig {
        &self.inner.config
    }

    /// Il motore dice se sta trasmettendo con un PTT che non passa dal CAT (vedi `external_tx`).
    pub fn set_external_tx(&self, on: bool) {
        self.inner.external_tx.store(on, Ordering::SeqCst);
    }

    pub fn status(&self) -> RigStatus {
        self.inner.snapshot()
    }

    /// Aspetta che la connessione sia riuscita o fallita (non resta in «collegamento»): per le prove e per chi parte da fermo.
    pub fn wait_settled(&self, timeout: Duration) -> RigStatus {
        let until = Instant::now() + timeout;
        loop {
            let s = self.status();
            if !matches!(s.state, LinkState::Connecting | LinkState::Reconnecting) || Instant::now() >= until {
                return s;
            }
            std::thread::sleep(Duration::from_millis(10));
        }
    }

    /// PTT acceso o spento. Acceso: solo a radio collegata e passata l'attesa dopo il collegamento. Spento: 2 tentativi, e se falliscono lo stato
    /// resta «acceso» (e l'errore si vede). Con il metodo VOX non si manda nessun comando.
    pub fn set_ptt(&self, on: bool) -> Result<(), RigError> {
        let inner = &self.inner;
        if inner.config.ptt == PttMethod::Vox {
            return Ok(());
        }
        let link = inner.connected_link()?;
        inner.epoch.fetch_add(1, Ordering::SeqCst);
        if on {
            // una lettura mancata (modo, frequenza, timeout) non e' un guasto della radio: il PTT si prova lo stesso. Se il collegamento
            // e' davvero caduto lo dice `connected_link()`, e se la radio non risponde lo dice l'errore del comando, che arriva a schermo
            if let Some(left) = inner.ptt_blocked() {
                return Err(RigError(format!("PTT bloccato per altri {} s dopo il collegamento", left.as_secs_f32().ceil())));
            }
            if inner.config.split == SplitMode::Emulate {
                self.emulate_begin(&link);
            }
            link.client().set_ptt(true).map_err(|e| RigError(friendly_error(&e.0)))?;
        } else {
            inner.ptt_off(&link, 2).map_err(|e| RigError(format!("PTT non spento: {}", friendly_error(&e.0))))?;
            self.emulate_end(&link);
        }
        inner.update(|s| s.ptt = on);
        std::thread::sleep(inner.timing.ptt_pause);
        inner.queue_quick_reads();
        Ok(())
    }

    /// Split simulato: la radio resta in simplex e durante la trasmissione si sposta sulla frequenza di trasmissione.
    fn emulate_begin(&self, link: &Link) {
        if self.inner.dx_split.load(Ordering::SeqCst) {
            return;
        }
        let (audio, dial) = (self.inner.tx_audio_hz.load(Ordering::SeqCst), self.inner.rx_dial.load(Ordering::SeqCst));
        if audio == 0 || dial == 0 {
            return;
        }
        let tx = split_tx_dial_hz(dial, audio);
        if tx != dial
            && link.client().set_frequency(tx).is_ok()
            && let Ok(mut slot) = self.inner.emulate_restore.lock()
        {
            *slot = Some(dial);
        }
    }

    fn emulate_end(&self, link: &Link) {
        let back = self.inner.emulate_restore.lock().ok().and_then(|mut s| s.take());
        if let Some(dial) = back
            && let Err(e) = link.client().set_frequency(dial)
        {
            tracing::warn!("ritorno alla frequenza di ricezione non riuscito: {e}");
        }
    }

    /// La frequenza audio con cui si trasmette (Hz): con lo split vero sposta il VFO di trasmissione, con la simulazione serve al PTT.
    pub fn set_tx_audio_frequency(&self, hz: u32) -> Result<(), RigError> {
        if hz > MAX_AUDIO_HZ {
            return Err(RigError(format!("frequenza audio di trasmissione non valida: {hz} Hz")));
        }
        self.inner.tx_audio_hz.store(hz, Ordering::SeqCst);
        if self.inner.config.split == SplitMode::Rig {
            let link = self.inner.connected_link()?;
            self.inner.epoch.fetch_add(1, Ordering::SeqCst);
            self.inner.apply_split(&link).map_err(|e| RigError(friendly_error(&e.0)))?;
        } else if self.inner.config.split == SplitMode::Emulate && self.inner.status.lock().is_ok_and(|s| s.ptt) {
            let link = self.inner.connected_link()?;
            self.inner.epoch.fetch_add(1, Ordering::SeqCst);
            self.emulate_follow(&link)?;
        }
        Ok(())
    }

    /// Prima di una trasmissione libera (PSK, RTTY, CW, SSTV: l'onda non e' spostata dell'XIT di FT8): la frequenza audio di FT8 si
    /// dimentica, cosi' lo split simulato non sposta la radio, e lo split vero acceso per FT8 si spegne. Lo Split DX di uno spot, chiesto
    /// dall'operatore in quei modi, resta. La trasmissione successiva del motore rimette tutto con `set_tx_audio_frequency`.
    pub fn release_tx_split(&self) -> Result<(), RigError> {
        let inner = &self.inner;
        inner.tx_audio_hz.store(0, Ordering::SeqCst);
        if inner.dx_split.load(Ordering::SeqCst) || !inner.xit_split.load(Ordering::SeqCst) {
            return Ok(());
        }
        let link = inner.connected_link()?;
        if inner.transmitting() {
            return Ok(());
        }
        inner.epoch.fetch_add(1, Ordering::SeqCst);
        let rx_vfo = if inner.vfo_reversed.load(Ordering::SeqCst) { "VFOB" } else { "VFOA" };
        link.client().set_split(false, rx_vfo).map_err(|e| RigError(friendly_error(&e.0)))?;
        inner.xit_split.store(false, Ordering::SeqCst);
        inner.update(|s| s.split = Some(false));
        Ok(())
    }

    /// Split simulato a trasmissione in corso con la frequenza audio cambiata: la radio va subito sulla nuova frequenza di trasmissione
    /// (in WSJT-X `EmulateSplitTransceiver::set` col PTT acceso porta la radio sulla frequenza Tx a ogni cambio).
    fn emulate_follow(&self, link: &Link) -> Result<(), RigError> {
        if self.inner.dx_split.load(Ordering::SeqCst) {
            return Ok(());
        }
        let audio = self.inner.tx_audio_hz.load(Ordering::SeqCst);
        let restore = self.inner.emulate_restore.lock().ok().and_then(|s| *s);
        let dial = restore.unwrap_or_else(|| self.inner.rx_dial.load(Ordering::SeqCst));
        if audio == 0 || dial == 0 {
            return Ok(());
        }
        let tx = split_tx_dial_hz(dial, audio);
        link.client().set_frequency(tx).map_err(|e| RigError(friendly_error(&e.0)))?;
        if let Ok(mut slot) = self.inner.emulate_restore.lock() {
            *slot = (tx != dial).then_some(dial);
        }
        Ok(())
    }

    /// Il modo come lo da Hamlib (`USB`, `PKTUSB`...).
    pub fn radio_mode(&self) -> Result<(String, u32), RigError> {
        self.inner.connected_link()?.client().mode()
    }

    pub fn set_radio_mode(&self, mode: &str) -> Result<(), RigError> {
        let link = self.inner.connected_link()?;
        self.inner.epoch.fetch_add(1, Ordering::SeqCst);
        link.client().set_mode(mode, 0).map_err(|e| RigError(friendly_error(&e.0)))?;
        self.inner.update(|s| s.mode = Some(mode.to_string()));
        self.inner.queue_quick_reads();
        Ok(())
    }

    /// La radio sta trasmettendo (PTT acceso da noi o dal motore).
    pub fn transmitting(&self) -> bool {
        self.inner.transmitting()
    }

    /// Il clic su un indicatore del display esteso (NB, NR, notch, ATU, preamplificatore, attenuatore, AGC, VFO, RIT, XIT): cambia
    /// l'impostazione sulla radio, rilegge subito quel dato e lo mette nello stato. Mai in trasmissione.
    pub fn set_extra(&self, item: crate::extras::ExtraItem) -> Result<(), RigError> {
        let inner = &self.inner;
        let link = inner.connected_link()?;
        if inner.transmitting() {
            return Err(RigError("la radio sta trasmettendo: impostazione rifiutata".into()));
        }
        inner.epoch.fetch_add(1, Ordering::SeqCst);
        let cache = {
            let mut poll = inner.extras.lock().map_err(|_| RigError("letture della radio non disponibili".into()))?;
            poll.apply(link.client(), item).map_err(|e| RigError(friendly_error(&e.0)))?;
            poll.cache.clone()
        };
        inner.update(|s| s.extras = cache);
        if item == crate::extras::ExtraItem::Vfo {
            // cambiando VFO cambiano anche frequenza e modo mostrati
            inner.queue_quick_reads();
        }
        Ok(())
    }

    /// Una stringa CAT mandata alla radio cosi' com'e' (macro `<CATCMD:...>`). Non si manda durante la trasmissione.
    pub fn send_raw_cat(&self, text: &str) -> Result<(), RigError> {
        let link = self.inner.connected_link()?;
        link.client().send_raw(text).map_err(|e| RigError(friendly_error(&e.0)))
    }

    /// La frequenza del VFO di trasmissione (split), in Hz: i satelliti la correggono col Doppler.
    pub fn set_split_frequency(&self, hz: u64) -> Result<(), RigError> {
        let link = self.inner.connected_link()?;
        link.client().set_split_frequency(hz).map_err(|e| RigError(friendly_error(&e.0)))
    }

    pub fn set_split(&self, on: bool, tx_vfo: &str) -> Result<(), RigError> {
        let link = self.inner.connected_link()?;
        self.inner.epoch.fetch_add(1, Ordering::SeqCst);
        link.client().set_split(on, tx_vfo).map_err(|e| RigError(friendly_error(&e.0)))?;
        self.inner.update(|s| s.split = Some(on));
        Ok(())
    }

    /// Split DX (doppio clic su uno spot del cluster con «UP 5», «QSX 14.030»...): la ricezione va su `rx_hz` come un cambio di frequenza
    /// (controllo dell'arrivo, modo riscritto, split di prima spento), il VFO di trasmissione su `tx_hz` e lo split si accende per ultimo
    /// (alcune radio escono dallo split quando cambia il VFO di ricezione). Indipendente dallo split di FT8: vedi `dx_split`.
    pub fn set_dx_split(&self, rx_hz: u64, tx_hz: u64) -> Result<(), RigError> {
        if !valid_dial_hz(tx_hz) {
            return Err(RigError(format!("frequenza di trasmissione non valida: {tx_hz} Hz")));
        }
        self.set_frequency(rx_hz)?;
        let inner = &self.inner;
        let link = inner.connected_link()?;
        if inner.transmitting() {
            return Err(RigError("la radio sta trasmettendo: split rifiutato".into()));
        }
        inner.epoch.fetch_add(1, Ordering::SeqCst);
        // l'operatore puo' ricevere sul VFO B: allora si trasmette sul VFO A
        if let Ok(active) = link.client().vfo() {
            inner.vfo_reversed.store(vfo_is_b(&active), Ordering::SeqCst);
        }
        let tx_vfo = if inner.vfo_reversed.load(Ordering::SeqCst) { "VFOA" } else { "VFOB" };
        let fr = |e: RigError| RigError(friendly_error(&e.0));
        link.client().set_split_frequency(tx_hz).map_err(fr)?;
        link.client().set_split(true, tx_vfo).map_err(fr)?;
        inner.dx_split_tx.store(tx_hz, Ordering::SeqCst);
        inner.dx_split.store(true, Ordering::SeqCst);
        inner.xit_split.store(false, Ordering::SeqCst);
        inner.split_off_pending.store(false, Ordering::SeqCst);
        inner.update(|s| s.split = Some(true));
        inner.queue_quick_reads();
        Ok(())
    }

    /// Lo Split DX e' acceso (o non si e' riusciti a spegnerlo).
    pub fn dx_split_active(&self) -> bool {
        self.inner.dx_split.load(Ordering::SeqCst)
    }

    /// Spegne lo Split DX, se acceso: la radio torna in simplex sul VFO di ricezione.
    pub fn clear_dx_split(&self) -> Result<(), RigError> {
        let inner = &self.inner;
        if !inner.dx_split.load(Ordering::SeqCst) {
            return Ok(());
        }
        let Ok(link) = inner.connected_link() else {
            // radio scollegata (CAT caduto, riconnessione in corso): lo Split DX non e' piu' voluto e non si rimette quando la radio
            // torna; se lei lo ha ancora acceso lo si spegne li'. Prima restava segnato e la riconnessione lo riaccendeva sul VFO B
            inner.dx_split.store(false, Ordering::SeqCst);
            inner.dx_split_tx.store(0, Ordering::SeqCst);
            inner.split_off_pending.store(true, Ordering::SeqCst);
            return Ok(());
        };
        if inner.transmitting() {
            return Err(RigError("la radio sta trasmettendo: split non spento".into()));
        }
        inner.epoch.fetch_add(1, Ordering::SeqCst);
        let rx_vfo = if inner.vfo_reversed.load(Ordering::SeqCst) { "VFOB" } else { "VFOA" };
        link.client().set_split(false, rx_vfo).map_err(|e| RigError(friendly_error(&e.0)))?;
        inner.dx_split.store(false, Ordering::SeqCst);
        inner.update(|s| s.split = Some(false));
        Ok(())
    }

    /// Lo Split DX e' acceso.
    pub fn dx_split_on(&self) -> bool {
        self.inner.dx_split.load(Ordering::SeqCst)
    }

    /// Ferma tutto: chiude il supervisore, spegne il PTT (3 tentativi), toglie lo split vero e libera la radio (il demone si ferma).
    /// «Spegni» di JTDX: alla chiusura di Aurora, se scelto, la radio si spegne via CAT. Va chiamata prima di [`HamlibRig::shutdown`].
    pub fn power_off_if_chosen(&self) {
        if !self.inner.config.power_off {
            return;
        }
        let Some(link) = self.inner.link() else { return };
        let _ = self.inner.ptt_off(&link, 3);
        match link.client().set_powerstat(false) {
            Ok(()) => tracing::info!("radio spenta via CAT alla chiusura («Spegni»)"),
            Err(e) => tracing::warn!("spegnimento della radio via CAT non riuscito: {e}"),
        }
    }

    /// Il thread del collegamento e' morto per un panic? Ne restituisce il testo, una volta sola (lo guarda il servizio CAT per ricollegare).
    pub fn worker_panic(&self) -> Option<String> {
        crate::watchdog::reap_panic(&self.thread)
    }

    pub fn shutdown(&self) {
        self.inner.stop.store(true, Ordering::SeqCst);
        if let Some(handle) = self.thread.lock().ok().and_then(|mut h| h.take()) {
            let _ = handle.join();
        }
        if let Some(link) = self.inner.take_link() {
            if let Err(e) = self.inner.ptt_off(&link, 3) {
                tracing::warn!("PTT non confermato spento alla chiusura: {e}");
                self.inner.update(|s| {
                    s.state = LinkState::Disconnected;
                    s.message = "Chiusa, ma il PTT potrebbe essere rimasto acceso: controlla la radio.".into();
                });
                return;
            }
            self.emulate_end(&link);
            let dx_split = self.inner.dx_split.swap(false, Ordering::SeqCst);
            if self.inner.config.split == SplitMode::Rig || dx_split {
                let _ = link.client().set_split(false, "VFOA");
            }
        }
        self.inner.update(|s| {
            s.state = LinkState::Disconnected;
            s.message.clear();
            s.ptt = false;
            s.failures = 0;
            s.reconnect_attempt = 0;
        });
    }
}

impl Drop for HamlibRig {
    fn drop(&mut self) {
        self.shutdown();
    }
}

impl Rig for HamlibRig {
    /// Cambio di frequenza (non in trasmissione): scrive, controlla che la radio ci sia arrivata (fino a due ripetizioni), riscrive il modo scelto
    /// se la radio ha richiamato quello memorizzato per la banda e, con lo split vero, riposiziona il VFO di trasmissione.
    fn set_frequency(&self, hz: u64) -> Result<(), RigError> {
        let inner = &self.inner;
        if !valid_dial_hz(hz) {
            return Err(RigError(format!("frequenza non valida: {hz} Hz")));
        }
        let link = inner.connected_link()?;
        if inner.transmitting() {
            return Err(RigError("la radio sta trasmettendo: cambio di frequenza rifiutato".into()));
        }
        inner.epoch.fetch_add(1, Ordering::SeqCst);
        let mine = inner.qsy_seq.fetch_add(1, Ordering::SeqCst) + 1;
        let fr = |e: RigError| RigError(friendly_error(&e.0));
        link.client().set_frequency(hz).map_err(fr)?;
        for attempt in 0..3 {
            std::thread::sleep(inner.timing.qsy_check);
            // un cambio piu' recente (due modi scelti uno dopo l'altro) ha gia' portato la radio altrove: questo non insiste e non e' un
            // errore. Prima i due cambi si contendevano la radio e il primo finiva con «la radio non ha raggiunto la frequenza»
            if inner.qsy_seq.load(Ordering::SeqCst) != mine {
                return Ok(());
            }
            match link.client().frequency() {
                Ok(f) if f.abs_diff(hz) <= 100 => break,
                Ok(_) if attempt < 2 => link.client().set_frequency(hz).map_err(fr)?,
                Ok(f) => return Err(RigError(format!("la radio non ha raggiunto la frequenza ({f} Hz invece di {hz} Hz)"))),
                Err(_) => break,
            }
        }
        inner.rx_dial.store(hz, Ordering::SeqCst);
        inner.update(|s| s.frequency_hz = Some(hz));
        if let Some(wanted) = inner.config.cat_mode.hamlib_name() {
            let current = link.client().mode().map(|m| m.0).ok();
            if !current.as_deref().is_some_and(|c| mode_satisfies(wanted, c)) {
                match link.client().set_mode(wanted, 0) {
                    Ok(()) => inner.update(|s| s.mode = Some(wanted.to_string())),
                    Err(e) => tracing::warn!("modo {wanted} non scritto dopo il cambio di frequenza: {e}"),
                }
            }
        }
        // split vero: al cambio di frequenza (banda) la radio esce dallo split. Il VFO di trasmissione si rimette sulla banda nuova solo un
        // attimo prima del PTT (`set_tx_audio_frequency`, chiamato da Aurora prima di ogni trasmissione): cosi' la radio non resta mai in split
        // con il VFO B sulla banda vecchia o su una frequenza che il proprietario non si aspetta (segnalazione del 2026-11-11)
        // lo Split DX di uno spot del cluster finisce al cambio di frequenza (spot senza «UP», cambio di banda), anche senza lo split di FT8
        let dx_split = inner.dx_split.swap(false, Ordering::SeqCst);
        if inner.config.split == SplitMode::Rig || dx_split {
            let rx_vfo = if inner.vfo_reversed.load(Ordering::SeqCst) { "VFOB" } else { "VFOA" };
            match link.client().set_split(false, rx_vfo) {
                Ok(()) => {
                    inner.xit_split.store(false, Ordering::SeqCst);
                    inner.update(|s| s.split = Some(false));
                }
                Err(e) => tracing::warn!("split non spento dopo il cambio di frequenza: {e}"),
            }
        }
        inner.queue_quick_reads();
        Ok(())
    }

    /// L'ultima frequenza letta dal polling (nessuna lettura in piu' sul bus della radio).
    fn frequency(&self) -> Result<u64, RigError> {
        let s = self.status();
        match (s.state, s.frequency_hz) {
            (LinkState::Connected, Some(hz)) => Ok(hz),
            _ => Err(RigError("radio non collegata".into())),
        }
    }

    /// I misuratori letti con i livelli di Hamlib (`STRENGTH` in ricezione; potenza, SWR, ALC... in trasmissione).
    fn meters(&self) -> Result<Meters, RigError> {
        self.inner.read_meters()
    }

    fn set_rx_frequency(&self, _hz: u32) -> Result<(), RigError> {
        Err(RigError("comando del motore non supportato da Hamlib".into()))
    }

    fn output_attenuation(&self) -> Result<u32, RigError> {
        Err(RigError("comando del motore non supportato da Hamlib".into()))
    }

    fn set_output_attenuation(&self, _tenths: u32) -> Result<(), RigError> {
        Err(RigError("comando del motore non supportato da Hamlib".into()))
    }

    fn set_mode(&self, _mode: &str) -> Result<(), RigError> {
        Err(RigError("comando del motore non supportato da Hamlib".into()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{cat_config::CatMode, fake_rigctld::FakeRigctld, hamlib::RigctldConfig};

    fn fast() -> Timing {
        Timing {
            poll: Duration::from_millis(30),
            tx_poll_min: Duration::from_millis(10),
            quick_reads: [Duration::from_millis(10), Duration::from_millis(20)],
            startup_block: Duration::from_millis(250),
            ptt_pause: Duration::from_millis(5),
            qsy_check: Duration::from_millis(20),
            reconnect_step: Duration::from_millis(40),
            reconnect_cap: Duration::from_millis(200),
            max_reconnects: 5,
            tolerate: 3,
            tolerate_io: 8,
        }
    }

    fn connector(fake: &FakeRigctld) -> Connector {
        let addr = fake.addr;
        Arc::new(move || RigctldClient::connect_with_timeout(addr, Duration::from_secs(1), Duration::from_millis(500)).map(Link::from_client))
    }

    fn start(fake: &FakeRigctld, config: RigConfig) -> HamlibRig {
        start_with(fake, config, fast())
    }

    fn start_with(fake: &FakeRigctld, config: RigConfig, timing: Timing) -> HamlibRig {
        let rig = HamlibRig::start(config, connector(fake), timing);
        let s = rig.wait_settled(Duration::from_secs(3));
        assert_eq!(s.state, LinkState::Connected, "{s:?}");
        rig
    }

    /// Lettura lenta: il guasto dura abbastanza perche' la prova lo veda senza gare con il thread del polling.
    fn slow() -> Timing {
        Timing { poll: Duration::from_millis(120), ..fast() }
    }

    fn wait_until(what: &str, cond: impl Fn() -> bool) {
        let until = Instant::now() + Duration::from_secs(5);
        while !cond() {
            assert!(Instant::now() < until, "tempo scaduto: {what}");
            std::thread::sleep(Duration::from_millis(5));
        }
    }

    #[test]
    fn it_turns_the_ptt_off_first_and_reads_the_radio() {
        let fake = FakeRigctld::start(14_074_000);
        let rig = start(&fake, RigConfig::default());
        // la prima domanda e' solo `\chk_vfo` (modalita' VFO del demone); il primo comando alla radio e' lo spegnimento del PTT
        let commands: Vec<String> = fake.commands().into_iter().filter(|c| c != "\\chk_vfo").collect();
        assert_eq!(commands.first().map(String::as_str), Some("T 0"), "il PTT si spegne subito dopo l'apertura: {commands:?}");
        let s = rig.status();
        assert_eq!((s.frequency_hz, s.mode.as_deref(), s.ptt), (Some(14_074_000), Some("PKTUSB"), false));
        assert_eq!(rig.frequency().expect("frequenza"), 14_074_000);
    }

    #[test]
    fn polling_follows_the_radio() {
        let fake = FakeRigctld::start(7_074_000);
        let rig = start(&fake, RigConfig::default());
        fake.set_frequency(14_074_000);
        fake.set_mode("USB");
        wait_until("la frequenza segue la radio", || {
            let s = rig.status();
            s.frequency_hz == Some(14_074_000) && s.mode.as_deref() == Some("USB")
        });
    }

    #[test]
    fn the_ptt_is_refused_for_a_while_after_connecting_then_works() {
        let fake = FakeRigctld::start(14_074_000);
        let rig = start(&fake, RigConfig::default());
        assert!(rig.status().ptt_blocked);
        let err = rig.set_ptt(true).expect_err("bloccato");
        assert!(err.0.contains("bloccato"), "{err}");
        assert!(!fake.ptt());
        std::thread::sleep(Duration::from_millis(300));
        assert!(!rig.status().ptt_blocked);
        rig.set_ptt(true).expect("PTT acceso");
        assert!(fake.ptt() && rig.status().ptt);
        rig.set_ptt(false).expect("PTT spento");
        assert!(!fake.ptt() && !rig.status().ptt);
    }

    #[test]
    fn nothing_is_sent_without_a_connection() {
        let fake = FakeRigctld::start(14_074_000);
        let rig = start(&fake, RigConfig::default());
        rig.shutdown();
        assert!(rig.set_ptt(true).is_err());
        assert!(rig.set_frequency(14_074_000).is_err());
        assert_eq!(rig.status().state, LinkState::Disconnected);
    }

    #[test]
    fn while_transmitting_only_the_ptt_is_read_and_the_frequency_cannot_change() {
        let fake = FakeRigctld::start(14_074_000);
        let rig = start(&fake, RigConfig::default());
        std::thread::sleep(Duration::from_millis(300));
        rig.set_ptt(true).expect("PTT");
        assert!(rig.set_frequency(7_074_000).is_err(), "niente cambio di frequenza in trasmissione");
        std::thread::sleep(Duration::from_millis(150));
        fake.clear_commands();
        wait_until("letture del PTT in trasmissione", || fake.commands().iter().filter(|c| *c == "t").count() >= 4);
        let during = fake.commands();
        assert!(during.iter().all(|c| c == "t"), "in trasmissione si legge solo il PTT: {during:?}");
        rig.set_ptt(false).expect("PTT spento");
        fake.clear_commands();
        wait_until("le letture di frequenza riprendono", || fake.commands().iter().any(|c| c == "f"));
    }

    #[test]
    fn a_failed_ptt_off_keeps_the_truth_and_the_close_tries_again() {
        let fake = FakeRigctld::start(14_074_000);
        let rig = start(&fake, RigConfig::default());
        std::thread::sleep(Duration::from_millis(300));
        rig.set_ptt(true).expect("PTT");
        fake.fail_ptt_off(true);
        let err = rig.set_ptt(false).expect_err("spegnimento fallito");
        assert!(err.0.contains("PTT non spento"), "{err}");
        assert!(rig.status().ptt, "lo stato dice la verita': ancora acceso");
        assert!(fake.ptt());
        fake.fail_ptt_off(false);
        rig.shutdown();
        assert!(!fake.ptt(), "alla chiusura il PTT si spegne");
    }

    #[test]
    fn the_ptt_is_always_off_when_the_rig_is_dropped() {
        let fake = FakeRigctld::start(14_074_000);
        {
            let rig = start(&fake, RigConfig::default());
            std::thread::sleep(Duration::from_millis(300));
            rig.set_ptt(true).expect("PTT");
            assert!(fake.ptt());
        }
        assert!(!fake.ptt(), "il PTT e' spento anche se l'oggetto cade senza chiusura esplicita");
        assert_eq!(fake.commands().iter().rev().find(|c| c.starts_with('T')).map(String::as_str), Some("T 0"));
    }

    #[test]
    fn a_few_failed_reads_do_not_switch_the_cat_off() {
        let fake = FakeRigctld::start(14_074_000);
        let rig = start_with(&fake, RigConfig::default(), slow());
        fake.fail_with(-9);
        wait_until("un guasto", || rig.status().failures >= 1);
        assert_eq!(rig.status().state, LinkState::Connected);
        fake.fail_with(0);
        wait_until("i guasti si azzerano", || rig.status().failures == 0);
        assert_eq!(rig.status().state, LinkState::Connected);
    }

    #[test]
    fn timeouts_are_tolerated_longer_than_other_failures() {
        let t = fast();
        assert_eq!(failure_limit(&t, "f: timeout: la radio non risponde (-5)"), 8);
        assert_eq!(failure_limit(&t, "M: comando rifiutato dalla radio (-9)"), 3);
        let fake = FakeRigctld::start(14_074_000);
        let rig = start_with(&fake, RigConfig::default(), slow());
        fake.fail_with(-5);
        wait_until("quattro timeout", || rig.status().failures >= 4);
        assert_eq!(rig.status().state, LinkState::Connected, "con i timeout se ne tollerano fino a 8");
    }

    #[test]
    fn it_reconnects_by_steps_after_the_connection_drops() {
        let fake = FakeRigctld::start(14_074_000);
        let rig = start(&fake, RigConfig::default());
        fake.hang_up(true);
        wait_until("riconnessione", || rig.status().state == LinkState::Reconnecting);
        let s = rig.status();
        assert!(s.message.contains("Nuovo tentativo"), "{}", s.message);
        fake.hang_up(false);
        wait_until("di nuovo collegata", || rig.status().state == LinkState::Connected);
        let offs = fake.commands().iter().filter(|c| *c == "T 0").count();
        assert!(offs >= 2, "dopo ogni riapertura il PTT si spegne: {offs}");
        assert_eq!(rig.status().reconnect_attempt, 0);
    }

    #[test]
    fn it_gives_up_after_the_last_attempt_and_says_why() {
        let fake = FakeRigctld::start(14_074_000);
        let rig = start(&fake, RigConfig::default());
        fake.fail_with(-5);
        wait_until("errore finale", || rig.status().state == LinkState::Error);
        let s = rig.status();
        assert!(s.message.contains("non risponde"), "messaggio tradotto: {}", s.message);
        assert!(rig.set_ptt(true).is_err());
    }

    #[test]
    fn a_failed_first_open_is_an_error_with_a_readable_message() {
        let connector: Connector = Arc::new(|| Err(RigError("rigctld si e' fermato (exit code: 1): rig_open: Access denied".into())));
        let rig = HamlibRig::start(RigConfig::default(), connector, fast());
        let s = rig.wait_settled(Duration::from_secs(2));
        assert_eq!(s.state, LinkState::Error);
        assert!(s.message.contains("occupata"), "{}", s.message);
    }

    #[test]
    fn a_busy_port_diagnosis_reaches_the_status_unchanged() {
        let why = "La porta COM8 e' occupata da OmniRig: chiudi OmniRig oppure scegli OmniRig come sistema di controllo in Impostazioni › Stazione › Radio";
        let connector: Connector = Arc::new(move || Err(RigError(why.into())));
        let rig = HamlibRig::start(RigConfig::default(), connector, fast());
        let s = rig.wait_settled(Duration::from_secs(2));
        assert_eq!(s.state, LinkState::Error);
        assert!(s.message.starts_with(why), "la diagnosi non si riscrive con il testo generico: {}", s.message);
    }

    #[test]
    fn a_radio_that_stays_silent_at_the_start_is_an_open_error() {
        let fake = FakeRigctld::start(14_074_000);
        fake.fail_with(-5);
        let rig = HamlibRig::start(RigConfig::default(), connector(&fake), fast());
        let s = rig.wait_settled(Duration::from_secs(3));
        assert_eq!(s.state, LinkState::Error, "{s:?}");
    }

    #[test]
    fn a_qsy_rewrites_the_chosen_mode_when_the_radio_recalls_another() {
        let fake = FakeRigctld::start(7_074_000);
        fake.recall_mode_on_qsy(true);
        let rig = start(&fake, RigConfig { cat_mode: CatMode::Data, ..RigConfig::default() });
        rig.set_frequency(14_074_000).expect("QSY");
        assert_eq!(fake.frequency(), 14_074_000);
        assert_eq!(fake.mode(), "PKTUSB");
        assert_eq!(rig.status().frequency_hz, Some(14_074_000));
        // con «non toccare il modo» la radio resta com'e'
        let fake = FakeRigctld::start(7_074_000);
        fake.recall_mode_on_qsy(true);
        let rig = start(&fake, RigConfig::default());
        rig.set_frequency(14_074_000).expect("QSY");
        assert_eq!(fake.mode(), "USB");
        assert!(!fake.commands().iter().any(|c| c.starts_with("M ")));
    }

    #[test]
    fn a_data2_or_data3_profile_chosen_on_the_radio_is_kept_after_a_qsy() {
        let fake = FakeRigctld::start(7_074_000);
        fake.set_mode("USB-D2");
        let rig = start(&fake, RigConfig { cat_mode: CatMode::Data, ..RigConfig::default() });
        rig.set_frequency(14_074_000).expect("QSY");
        assert_eq!(fake.mode(), "USB-D2");
        assert!(!fake.commands().iter().any(|c| c.starts_with("M ")), "nessun cambio di modo");
        assert!(mode_satisfies("PKTUSB", "usb-d3") && mode_satisfies("PKTLSB", "LSB-D1") && mode_satisfies("USB", "USB"));
        assert!(!mode_satisfies("PKTUSB", "USB") && !mode_satisfies("USB", "USB-D1") && !mode_satisfies("PKTUSB", "LSB-D2"));
    }

    #[test]
    fn invalid_frequencies_are_refused_before_reaching_the_radio() {
        let fake = FakeRigctld::start(14_074_000);
        let rig = start(&fake, RigConfig { split: SplitMode::Rig, ..RigConfig::default() });
        fake.clear_commands();
        assert!(rig.set_frequency(0).is_err());
        assert!(rig.set_frequency(u64::MAX).is_err());
        assert!(rig.set_tx_audio_frequency(70_000).is_err());
        assert!(!fake.commands().iter().any(|c| c.starts_with("F ") || c.starts_with("I ")), "{:?}", fake.commands());
        assert_eq!(fake.frequency(), 14_074_000);
    }

    #[test]
    fn split_keeps_the_vfo_roles_while_transmitting_and_follows_the_tx_offset() {
        let fake = FakeRigctld::start(14_074_000);
        let rig = start(&fake, RigConfig { split: SplitMode::Rig, ..RigConfig::default() });
        std::thread::sleep(Duration::from_millis(300));
        // l'operatore riceve sul VFO B: si trasmette sul VFO A
        fake.client(Duration::from_secs(1)).expect("client").set_vfo("VFOB").expect("VFO B");
        fake.clear_commands();
        rig.set_tx_audio_frequency(1500).expect("split");
        assert!(fake.commands().iter().any(|c| c == "S 1 VFOA"), "{:?}", fake.commands());
        rig.set_ptt(true).expect("PTT");
        // in trasmissione la radio dice VFO A (quello di trasmissione): i ruoli non si scambiano e l'offset nuovo segue
        fake.client(Duration::from_secs(1)).expect("client").set_vfo("VFOA").expect("VFO A");
        fake.clear_commands();
        rig.set_tx_audio_frequency(2000).expect("offset nuovo");
        let c = fake.commands();
        assert!(c.iter().any(|x| x.starts_with("I ")) && c.iter().any(|x| x == "S 1 VFOA"), "{c:?}");
        assert!(!c.iter().any(|x| x == "v"), "il VFO attivo non si rilegge in trasmissione: {c:?}");
        // e la banda non cambia durante la trasmissione
        assert!(rig.set_frequency(7_074_000).is_err());
        rig.set_ptt(false).expect("PTT spento");
    }

    #[test]
    fn after_a_failed_read_the_ptt_is_still_tried() {
        let fake = FakeRigctld::start(14_074_000);
        let rig = start(&fake, RigConfig::default());
        std::thread::sleep(Duration::from_millis(300));
        rig.inner.update(|s| s.failures = 1);
        rig.set_ptt(true).expect("PTT acceso anche dopo una lettura mancata");
        assert!(fake.ptt());
        rig.set_ptt(false).expect("PTT spento");
    }

    #[test]
    fn real_split_moves_the_tx_vfo_and_switches_the_split_on_last() {
        let fake = FakeRigctld::start(14_074_000);
        let rig = start(&fake, RigConfig { split: SplitMode::Rig, ..RigConfig::default() });
        fake.clear_commands();
        rig.set_tx_audio_frequency(2200).expect("split");
        let split_commands = |fake: &FakeRigctld| fake.commands().into_iter().filter(|c| c.starts_with("I ") || c.starts_with("S ")).collect::<Vec<_>>();
        assert_eq!(split_commands(&fake), ["I 14074500", "S 1 VFOB"]);
        assert!(fake.split() && rig.status().split == Some(true));
        // al cambio di banda la radio esce dallo split (niente VFO B lasciato sulla banda vecchia)...
        rig.set_frequency(7_074_000).expect("QSY");
        assert!(!fake.split() && rig.status().split == Some(false), "{:?}", fake.commands());
        // ...e prima della trasmissione il VFO di trasmissione va sulla banda nuova e lo split si riaccende
        fake.clear_commands();
        rig.set_tx_audio_frequency(2200).expect("split prima del PTT");
        assert_eq!(split_commands(&fake), ["I 7074500", "S 1 VFOB"]);
        assert!(fake.split());
        // alla chiusura lo split si toglie
        rig.shutdown();
        assert!(!fake.split());
    }

    fn split_commands(fake: &FakeRigctld) -> Vec<String> {
        fake.commands().into_iter().filter(|c| c.starts_with("F ") || c.starts_with("I ") || c.starts_with("S ")).collect()
    }

    #[test]
    fn dx_split_tunes_the_rx_then_the_tx_vfo_then_switches_split_on() {
        let fake = FakeRigctld::start(14_074_000);
        // con lo split di FT8 («Rig») acceso: il cambio di ricezione lo spegne, poi lo Split DX mette il suo VFO di trasmissione
        let rig = start(&fake, RigConfig { split: SplitMode::Rig, ..RigConfig::default() });
        fake.clear_commands();
        rig.set_dx_split(14_025_000, 14_030_000).expect("split DX");
        assert_eq!(split_commands(&fake), ["F 14025000", "S 0 VFOA", "I 14030000", "S 1 VFOB"]);
        assert!(fake.split() && rig.dx_split_on() && rig.status().split == Some(true));
        assert_eq!(fake.frequency(), 14_025_000);
        // lo split di FT8 non sposta il VFO di trasmissione scelto dallo spot
        fake.clear_commands();
        rig.set_tx_audio_frequency(1500).expect("frequenza audio");
        assert!(split_commands(&fake).is_empty(), "{:?}", fake.commands());
        // «Split DX: spegni»
        rig.clear_dx_split().expect("spento");
        assert!(!fake.split() && !rig.dx_split_on());
        assert_eq!(split_commands(&fake), ["S 0 VFOA"]);
        // gia' spento: nessun comando
        fake.clear_commands();
        rig.clear_dx_split().expect("niente da fare");
        assert!(split_commands(&fake).is_empty());
    }

    #[test]
    fn dx_split_ends_at_the_next_frequency_change_and_at_close_even_without_ft8_split() {
        let fake = FakeRigctld::start(14_074_000);
        let rig = start(&fake, RigConfig::default());
        rig.set_dx_split(7_005_000, 7_007_000).expect("split DX");
        assert!(fake.split());
        fake.clear_commands();
        rig.set_frequency(7_074_000).expect("spot senza UP");
        assert!(!fake.split() && !rig.dx_split_on(), "{:?}", fake.commands());
        assert_eq!(split_commands(&fake), ["F 7074000", "S 0 VFOA"]);
        // un'altra frequenza senza Split DX non manda comandi di split
        fake.clear_commands();
        rig.set_frequency(7_075_000).expect("QSY");
        assert_eq!(split_commands(&fake), ["F 7075000"]);
        // alla chiusura lo Split DX si toglie
        rig.set_dx_split(7_005_000, 7_006_000).expect("split DX");
        rig.shutdown();
        assert!(!fake.split());
    }

    #[test]
    fn dx_split_refuses_a_wrong_tx_frequency_and_while_transmitting() {
        let fake = FakeRigctld::start(14_074_000);
        let rig = start(&fake, RigConfig::default());
        fake.clear_commands();
        assert!(rig.set_dx_split(14_025_000, 0).is_err());
        assert!(fake.commands().is_empty(), "nulla alla radio: {:?}", fake.commands());
        std::thread::sleep(Duration::from_millis(300));
        rig.set_ptt(true).expect("PTT");
        assert!(rig.set_dx_split(14_025_000, 14_026_000).expect_err("in trasmissione").0.contains("trasmettendo"));
        assert!(!fake.split());
    }

    #[test]
    fn emulated_split_shifts_the_frequency_only_while_transmitting() {
        let fake = FakeRigctld::start(14_074_000);
        let rig = start(&fake, RigConfig { split: SplitMode::Emulate, ..RigConfig::default() });
        rig.set_tx_audio_frequency(2200).expect("frequenza audio");
        std::thread::sleep(Duration::from_millis(300));
        rig.set_ptt(true).expect("PTT");
        assert_eq!(fake.frequency(), 14_074_500);
        rig.set_ptt(false).expect("PTT spento");
        assert_eq!(fake.frequency(), 14_074_000, "a fine trasmissione si torna alla ricezione");
        assert!(!fake.commands().iter().any(|c| c.starts_with("S ") || c.starts_with("I ")), "nessuno split vero");
    }

    /// Il CAT cade e si riprende da solo (la frequenza sparisce dal selettore e poi torna).
    fn drop_and_reconnect(fake: &FakeRigctld, rig: &HamlibRig) {
        fake.hang_up(true);
        wait_until("riconnessione", || rig.status().state == LinkState::Reconnecting);
        fake.hang_up(false);
        wait_until("di nuovo collegata", || rig.status().state == LinkState::Connected);
        // un giro di letture dopo il collegamento: i comandi dell'apertura sono partiti tutti
        std::thread::sleep(Duration::from_millis(150));
    }

    #[test]
    fn from_rtty_back_to_ft8_the_ft8_split_is_not_switched_on_again_by_a_reconnection() {
        let fake = FakeRigctld::start(14_074_000);
        let rig = start(&fake, RigConfig { split: SplitMode::Rig, ..RigConfig::default() });
        // FT8 con «Split: Radio»: prima della trasmissione lo split si accende
        rig.set_tx_audio_frequency(2200).expect("split di FT8");
        assert!(fake.split());
        // pagina PSK·RTTY·CW senza Lock ne' «UP N», poi di nuovo FT8: due cambi di frequenza, la radio resta in simplex
        rig.set_frequency(14_085_000).expect("RTTY");
        rig.set_frequency(14_074_000).expect("FT8");
        assert!(!fake.split());
        // il CAT cade e torna: lo split non si riaccende da solo (prima: `I 14074500` e `S 1 VFOB` con l'ultima frequenza audio nota)
        fake.clear_commands();
        drop_and_reconnect(&fake, &rig);
        assert!(!fake.split(), "{:?}", fake.commands());
        assert!(!fake.commands().iter().any(|c| c.starts_with("I ") || c.starts_with("S 1")), "{:?}", fake.commands());
        assert_eq!(rig.status().split, Some(false));
        // la trasmissione FT8 successiva lo accende, come sempre
        fake.clear_commands();
        rig.set_tx_audio_frequency(2200).expect("split prima del PTT");
        assert_eq!(split_commands(&fake), ["I 14074500", "S 1 VFOB"]);
    }

    #[test]
    fn a_text_mode_transmission_goes_out_in_simplex_whatever_the_ft8_split() {
        // split vero di FT8 ancora acceso (la pagina dei modi testuali aperta senza cambiare frequenza): la trasmissione RTTY lo spegne
        let fake = FakeRigctld::start(14_074_000);
        let rig = start(&fake, RigConfig { split: SplitMode::Rig, ..RigConfig::default() });
        rig.set_tx_audio_frequency(2200).expect("split di FT8");
        fake.clear_commands();
        rig.release_tx_split().expect("simplex");
        assert_eq!(split_commands(&fake), ["S 0 VFOA"]);
        assert!(!fake.split() && rig.status().split == Some(false));
        // gia' in simplex (Lock dei modi testuali: la trasmissione e' spostata solo nell'audio): nessun comando di split, nemmeno dopo una caduta
        fake.clear_commands();
        rig.release_tx_split().expect("niente da fare");
        drop_and_reconnect(&fake, &rig);
        assert!(!fake.commands().iter().any(|c| c.starts_with("I ") || c.starts_with("S ")), "{:?}", fake.commands());
        assert!(!fake.split());

        // split simulato di FT8: la trasmissione RTTY non sposta la radio dell'XIT di FT8
        let fake = FakeRigctld::start(14_074_000);
        let rig = start(&fake, RigConfig { split: SplitMode::Emulate, ..RigConfig::default() });
        rig.set_tx_audio_frequency(2200).expect("frequenza audio di FT8");
        rig.set_frequency(14_085_000).expect("RTTY");
        rig.release_tx_split().expect("simplex");
        std::thread::sleep(Duration::from_millis(300));
        rig.set_ptt(true).expect("PTT");
        assert_eq!(fake.frequency(), 14_085_000, "la radio trasmette dove riceve");
        rig.set_ptt(false).expect("PTT spento");
        assert_eq!(fake.frequency(), 14_085_000);
    }

    #[test]
    fn the_dx_split_of_a_spot_stays_for_text_transmissions_and_ends_when_the_page_is_left() {
        let fake = FakeRigctld::start(14_074_000);
        let rig = start(&fake, RigConfig { split: SplitMode::Rig, ..RigConfig::default() });
        rig.set_tx_audio_frequency(2200).expect("split di FT8");
        // RTTY, spot con «UP 2»: lo ha chiesto l'operatore, la trasmissione RTTY lo lascia acceso
        rig.set_dx_split(14_085_000, 14_087_000).expect("split DX");
        fake.clear_commands();
        rig.release_tx_split().expect("lo Split DX resta");
        assert!(split_commands(&fake).is_empty(), "{:?}", fake.commands());
        assert!(fake.split() && rig.dx_split_on());
        // ritorno a FT8 a radio collegata: lo Split DX si spegne e non torna dopo una caduta del CAT
        rig.clear_dx_split().expect("spento");
        rig.set_frequency(14_074_000).expect("FT8");
        fake.clear_commands();
        drop_and_reconnect(&fake, &rig);
        assert!(!fake.split(), "{:?}", fake.commands());
        assert!(!fake.commands().iter().any(|c| c.starts_with("I ") || c.starts_with("S 1")), "{:?}", fake.commands());
    }

    #[test]
    fn a_dx_split_cleared_while_the_cat_is_down_is_not_restored_and_is_switched_off_at_the_reconnection() {
        let fake = FakeRigctld::start(14_074_000);
        let rig = start(&fake, RigConfig::default());
        rig.set_dx_split(14_085_000, 14_087_000).expect("split DX");
        assert!(fake.split());
        // il CAT cade mentre si torna a FT8: lo spegnimento non arriva alla radio, ma lo Split DX non e' piu' voluto
        fake.hang_up(true);
        wait_until("riconnessione", || rig.status().state == LinkState::Reconnecting);
        rig.clear_dx_split().expect("dimenticato anche a radio scollegata");
        assert!(!rig.dx_split_on());
        fake.clear_commands();
        fake.hang_up(false);
        wait_until("di nuovo collegata", || rig.status().state == LinkState::Connected);
        // la radio l'aveva ancora acceso: si spegne appena torna, e non si riaccende
        wait_until("split spento", || !fake.split());
        std::thread::sleep(Duration::from_millis(150));
        assert!(!fake.split());
        assert!(!fake.commands().iter().any(|c| c.starts_with("I ") || c.starts_with("S 1")), "{:?}", fake.commands());
        assert_eq!(rig.status().split, Some(false));
        // uno Split DX ancora voluto, invece, dopo la caduta si rimette
        rig.set_dx_split(14_085_000, 14_087_000).expect("split DX");
        fake.clear_commands();
        drop_and_reconnect(&fake, &rig);
        assert!(fake.split() && rig.dx_split_on());
        assert!(fake.commands().iter().any(|c| c == "I 14087000"), "{:?}", fake.commands());
    }

    #[test]
    fn vox_sends_no_ptt_command() {
        let fake = FakeRigctld::start(14_074_000);
        let rig = start(&fake, RigConfig { ptt: PttMethod::Vox, ..RigConfig::default() });
        fake.clear_commands();
        rig.set_ptt(true).expect("VOX");
        assert!(!fake.commands().iter().any(|c| c.starts_with("T ")));
    }

    fn level_commands(fake: &FakeRigctld) -> Vec<String> {
        fake.commands().into_iter().filter(|c| c.starts_with("l ")).collect()
    }

    fn tx_rig(fake: &FakeRigctld, config: RigConfig) -> HamlibRig {
        let rig = start(fake, config);
        std::thread::sleep(Duration::from_millis(300));
        rig.set_ptt(true).expect("PTT");
        std::thread::sleep(Duration::from_millis(210));
        rig
    }

    #[test]
    fn in_receive_only_the_strength_is_read_and_converted_to_s_points() {
        let fake = FakeRigctld::start(14_074_000);
        fake.set_level("STRENGTH", "-12");
        let rig = start(&fake, RigConfig::default());
        let m = rig.meters().expect("misuratori");
        assert_eq!((m.s_points, m.s_over_db, m.power_w, m.swr), (Some(7.0), Some(0.0), None, None));
        std::thread::sleep(Duration::from_millis(260));
        fake.set_level("STRENGTH", "27");
        let m = rig.meters().expect("misuratori");
        assert_eq!((m.s_points, m.s_over_db), (Some(9.0), Some(27.0)));
        assert!(level_commands(&fake).iter().all(|c| c == "l STRENGTH"), "in ricezione solo STRENGTH: {:?}", level_commands(&fake));
    }

    #[test]
    fn a_call_inside_the_interval_does_not_ask_the_radio_again() {
        let fake = FakeRigctld::start(14_074_000);
        fake.set_level("STRENGTH", "-6");
        let rig = start(&fake, RigConfig::default());
        let _ = rig.meters().expect("misuratori");
        let _ = rig.meters().expect("misuratori");
        assert_eq!(fake.count("l STRENGTH"), 1);
    }

    #[test]
    fn an_unsupported_level_stays_none_and_is_not_asked_again() {
        let fake = FakeRigctld::start(14_074_000);
        let rig = start(&fake, RigConfig::default());
        for _ in 0..4 {
            let m = rig.meters().expect("nessun errore per un livello non supportato");
            assert_eq!((m.s_points, m.s_over_db), (None, None));
            std::thread::sleep(Duration::from_millis(260));
        }
        assert_eq!(fake.count("l STRENGTH"), 1, "la richiesta non si ripete");
    }

    #[test]
    fn in_transmit_only_the_tx_levels_are_read_and_missing_ones_are_asked_once() {
        let fake = FakeRigctld::start(14_074_000);
        fake.set_level("STRENGTH", "-12");
        fake.set_level("RFPOWER_METER_WATTS", "20.5");
        fake.set_level("SWR", "1.5");
        fake.set_level("ALC", "0.3");
        fake.set_level("COMP_METER", "6");
        fake.set_level("VD_METER", "13.8");
        let rig = tx_rig(&fake, RigConfig::default());
        fake.clear_commands();
        let mut last = Meters::default();
        for _ in 0..3 {
            last = rig.meters().expect("misuratori");
            std::thread::sleep(Duration::from_millis(210));
        }
        assert_eq!((last.power_w, last.swr, last.alc, last.comp_db, last.vdd_v), (Some(20.5), Some(1.5), Some(0.3), Some(6.0), Some(13.8)));
        assert_eq!((last.id_a, last.s_points), (None, None));
        let asked = level_commands(&fake);
        assert!(!asked.iter().any(|c| c == "l STRENGTH"), "in trasmissione niente STRENGTH: {asked:?}");
        assert_eq!(fake.count("l ID_METER"), 1, "ID non supportato: una sola richiesta");
        // di nuovo in ricezione: i dati di trasmissione spariscono e torna lo S-meter
        rig.set_ptt(false).expect("PTT spento");
        std::thread::sleep(Duration::from_millis(260));
        let m = rig.meters().expect("misuratori");
        assert_eq!((m.power_w, m.swr, m.s_points), (None, None, Some(7.0)));
    }

    #[test]
    fn power_falls_back_to_the_fraction_times_the_full_scale() {
        let fake = FakeRigctld::start(14_074_000);
        fake.set_level("RFPOWER_METER", "0.25");
        fake.set_full_power_mw(100_000);
        let rig = tx_rig(&fake, RigConfig::default());
        let m = rig.meters().expect("misuratori");
        assert_eq!(m.power_w, Some(25.0));
        assert_eq!(fake.count("l RFPOWER_METER_WATTS"), 1, "i watt diretti non ci sono: una sola richiesta");
    }

    #[test]
    fn a_fraction_without_a_known_full_scale_gives_no_power() {
        let fake = FakeRigctld::start(14_074_000);
        fake.set_level("RFPOWER_METER", "0.25");
        let rig = tx_rig(&fake, RigConfig::default());
        for _ in 0..5 {
            let m = rig.meters().expect("misuratori");
            assert_eq!(m.power_w, None, "senza fondo scala non si inventano i watt");
            std::thread::sleep(Duration::from_millis(210));
        }
    }

    #[test]
    fn a_slow_serial_port_gets_one_meter_command_per_call() {
        let fake = FakeRigctld::start(14_074_000);
        fake.set_level("RFPOWER_METER_WATTS", "10");
        fake.set_level("SWR", "1.2");
        let config = RigConfig { port: "COM3".into(), baud: 9600, ..RigConfig::default() };
        let rig = tx_rig(&fake, config);
        fake.clear_commands();
        let m = rig.meters().expect("misuratori");
        assert_eq!(level_commands(&fake).len(), 1, "un comando per giro: {:?}", level_commands(&fake));
        assert_eq!(m.power_w, Some(10.0));
        std::thread::sleep(Duration::from_millis(210));
        let m = rig.meters().expect("misuratori");
        assert_eq!((m.power_w, m.swr), (Some(10.0), Some(1.2)));
    }

    #[test]
    fn the_default_rig_has_no_meters() {
        struct Plain;
        impl Rig for Plain {
            fn set_frequency(&self, _: u64) -> Result<(), RigError> {
                Ok(())
            }
            fn set_rx_frequency(&self, _: u32) -> Result<(), RigError> {
                Ok(())
            }
            fn output_attenuation(&self) -> Result<u32, RigError> {
                Ok(0)
            }
            fn set_output_attenuation(&self, _: u32) -> Result<(), RigError> {
                Ok(())
            }
            fn set_mode(&self, _: &str) -> Result<(), RigError> {
                Ok(())
            }
        }
        assert!(Plain.meters().is_err());
    }

    fn real_rigctld() -> Option<std::path::PathBuf> {
        crate::hamlib::locate_rigctld()
    }

    /// Con un rigctld vero e il modello «dummy»: tutto il percorso (avvio del processo, polling, comandi, chiusura).
    #[test]
    fn a_real_dummy_rig_goes_through_the_whole_path() {
        let Some(exe) = real_rigctld() else { return };
        let config = RigConfig { model: 1, cat_mode: CatMode::Usb, ..RigConfig::default() };
        let rigctld: RigctldConfig = config.to_rigctld(&exe).expect("argomenti");
        let connector: Connector = Arc::new(move || Rigctld::start(&rigctld).map(Link::from_daemon));
        let rig = HamlibRig::start(config, connector, Timing { startup_block: Duration::from_millis(100), ..fast() });
        let s = rig.wait_settled(Duration::from_secs(15));
        assert_eq!(s.state, LinkState::Connected, "{s:?}");
        rig.set_frequency(14_074_000).expect("QSY");
        wait_until("la lettura segue", || rig.status().frequency_hz == Some(14_074_000));
        assert_eq!(rig.radio_mode().expect("modo").0, "USB");
        std::thread::sleep(Duration::from_millis(150));
        rig.set_ptt(true).expect("PTT");
        assert!(rig.status().ptt);
        rig.set_ptt(false).expect("PTT spento");
        rig.shutdown();
        assert_eq!(rig.status().state, LinkState::Disconnected);
    }

    #[test]
    fn killing_the_real_daemon_triggers_the_reconnection() {
        let Some(exe) = real_rigctld() else { return };
        let config = RigConfig { model: 1, ..RigConfig::default() };
        let rigctld: RigctldConfig = config.to_rigctld(&exe).expect("argomenti");
        let connector: Connector = Arc::new(move || Rigctld::start(&rigctld).map(Link::from_daemon));
        let rig = HamlibRig::start(config, connector, fast());
        assert_eq!(rig.wait_settled(Duration::from_secs(15)).state, LinkState::Connected);
        // si uccide il processo del demone dall'esterno: il supervisore se ne accorge e lo rilancia
        let link = rig.inner.link().expect("collegamento");
        if let Backing::Daemon(d) = &link.0 {
            let _ = std::process::Command::new("taskkill").args(["/PID", &d.pid().to_string(), "/F"]).output();
        }
        drop(link);
        wait_until("riconnessione dopo la morte del demone", || rig.status().state == LinkState::Reconnecting);
        assert_eq!(rig.wait_settled(Duration::from_secs(20)).state, LinkState::Connected);
        assert!(rig.frequency().is_ok());
    }
}
