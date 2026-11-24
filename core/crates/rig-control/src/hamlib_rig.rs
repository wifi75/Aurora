//! La radio comandata con Hamlib: un thread di supervisione tiene aperto `rigctld`, legge la radio a intervalli (polling) e riapre la
//! connessione se si guasta. Le regole (PTT, letture, tolleranza ai guasti, riconnessione) sono quelle ricavate dall'analisi di Decodium,
//! `docs/analisi-decodium-cat.md`, sezioni 1 e 6:
//! - subito dopo l'apertura il PTT si spegne, e per 5 s non si accetta nessun PTT acceso;
//! - in trasmissione non si leggono frequenza e modo (competono con il PTT sullo stesso bus) e il polling va a un quarto dell'intervallo;
//! - fra una lettura completa e l'altra si legge la sola frequenza (`f`), ogni 200 ms mentre la sintonia si muove e ogni 400 ms a riposo,
//!   cosi' la sintonia girata a mano si vede subito (mai in trasmissione, mai con un comando in corso);
//! - un guasto di lettura non spegne il CAT: 3 consecutivi (8 se sono timeout o errori di porta), poi riconnessione a gradini;
//! - il PTT si spegne sempre alla chiusura, e se il comando di spegnimento fallisce lo stato resta «acceso»: si dice la verita'.

use std::{
    sync::{
        atomic::{AtomicBool, AtomicU32, AtomicU64, AtomicU8, Ordering},
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
    /// Lettura della sola frequenza (`f`) fra una lettura completa e l'altra, perche' la sintonia girata a mano si veda subito: ogni
    /// `freq_fast` se la frequenza e' cambiata negli ultimi `freq_active_for`, altrimenti ogni `freq_idle`. `freq_idle` a zero = spenta.
    pub freq_fast: Duration,
    pub freq_idle: Duration,
    pub freq_active_for: Duration,
}

/// Lettura veloce della frequenza mentre la sintonia si muove, e a riposo (seriali veloci, rete, USB).
pub const FREQ_FAST: Duration = Duration::from_millis(100);
pub const FREQ_IDLE: Duration = Duration::from_millis(200);
/// Le stesse sulle seriali lente (9600 baud o meno): meno domande, per lasciare spazio al PTT.
pub const FREQ_FAST_SLOW_PORT: Duration = Duration::from_millis(300);
pub const FREQ_IDLE_SLOW_PORT: Duration = Duration::from_millis(700);
/// Per quanto tempo dopo un cambio di frequenza si resta alla lettura veloce.
pub const FREQ_ACTIVE_FOR: Duration = Duration::from_secs(3);

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
            freq_fast: if slow_port(config) { FREQ_FAST_SLOW_PORT } else { FREQ_FAST },
            freq_idle: if slow_port(config) { FREQ_IDLE_SLOW_PORT } else { FREQ_IDLE },
            freq_active_for: FREQ_ACTIVE_FOR,
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
    /// Ora (ms) dell'ultima volta che la radio e' andata in split senza un comando di Aurora negli ultimi secondi (la radio a mano o un
    /// programma collegato alla condivisione di `rigctld`); 0 = mai. La pagina lo mostra una volta per ogni valore nuovo.
    pub split_external_ms: u64,
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

/// L'errore di Hamlib dice «la radio non ha questa funzione» (-1 parametro non valido, -4 non implementata, -11 non disponibile)?
fn hamlib_unsupported(error: &RigError) -> bool {
    matches!(crate::hamlib::error_code(error), Some(-1 | -4 | -11))
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
    /// La radio non ha accettato il modo dati `PKTUSB` (radio senza modo dati, come la TS-480): da qui in poi si
    /// scrive `USB`, senza riprovare il modo dati a ogni cambio di banda.
    pktusb_fallback: AtomicBool,
    /// La radio non scrive il modo nel VFO di trasmissione (`X`) ne' sa copiare A=B (`G CPY`): il modo di trasmissione segue quello di
    /// ricezione. Si scopre una volta per sessione della radio e da li' non si riprova (ne' `X` ne' la copia).
    split_mode_unsupported: AtomicBool,
    /// La radio ha risposto «non implementata/non disponibile» a `S` (split): da qui in poi non lo si riprova, ne' per spegnerlo (IC-820H con
    /// Hamlib: `S 0 VFOA` a ogni cambio di frequenza, 457 avvisi in un'ora). Si azzera alla riconnessione.
    split_unsupported: AtomicBool,
    /// Quante volte la radio si e' collegata: serve ai satelliti per sapere che la memoria dei comandi rifiutati e' da azzerare.
    session: AtomicU64,
    /// La coppia di VFO che ha funzionato per la trasmissione dei satelliti: 0 ancora non nota, 1 = Sub/Main, 2 = VFOB/VFOA.
    sat_vfo_pair: AtomicU8,
    /// Quale banda sta su «Main» e quale su «Sub» (`MAIN_SUB_*`), letta una volta per connessione: sulle radio a due bande (IC-820H) i due VFO
    /// sono le bande 144 e 430 MHz, e una frequenza scritta nella banda sbagliata viene «ripiegata» (437,79 -> 137,79 MHz).
    sat_band_map: AtomicU8,
    /// IC-820H: trasmissione dei satelliti con trame CI-V dirette (prova automatica, ultima scrittura, registro dei passi). Si azzera alla
    /// riconnessione.
    civ: Mutex<CivUplink>,
    config: RigConfig,
    connector: Connector,
    timing: Timing,
    link: Mutex<Option<Arc<Link>>>,
    status: Mutex<RigStatus>,
    stop: AtomicBool,
    ptt_blocked_until: Mutex<Option<Instant>>,
    quick: Mutex<Vec<Instant>>,
    ptt_readable: AtomicBool,
    /// Il primo collegamento di questa sessione e' gia' avvenuto (per dire «all'avvio» o «dopo la riconnessione» nel registro).
    opened_once: AtomicBool,
    /// Qualcuno (lo spegnimento del PTT che non riesce) chiede di riaprire il CAT: lo fa il thread del collegamento, mai chi chiama.
    reconnect_wanted: AtomicBool,
    /// Quando e' stata chiesta l'ultima riapertura: non piu' di una ogni 10 s.
    reconnect_asked_at: Mutex<Option<Instant>>,
    /// Trasmissione comandata fuori dal CAT (PTT su RTS/DTR di un'altra porta): la dice il motore, cosi' i misuratori passano subito a
    /// potenza/ROS/ALC invece di aspettare la lettura del PTT dalla radio (ogni `poll_secs`), che non sempre vede quel PTT.
    external_tx: AtomicBool,
    /// Aurora ha acceso il PTT con il CAT e non l'ha ancora spento (serve a riconoscere un PTT acceso da altri).
    keyed_by_us: AtomicBool,
    /// Quando Aurora ha spento il PTT l'ultima volta (con il CAT).
    last_unkeyed: Mutex<Option<Instant>>,
    /// Letture di fila in cui la radio dice «in trasmissione» senza che Aurora l'abbia accesa, e quante volte si e' rimandato `T 0` per questo.
    stray_polls: AtomicU32,
    stray_resent: AtomicU32,
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
    /// La frequenza di ricezione dello Split DX: alla riconnessione lo split si rimette solo se la radio e' ancora li'.
    dx_split_rx: AtomicU64,
    /// Lo split di FT8 (XIT, «Split: Radio») e' acceso sulla radio perche' l'ha acceso Aurora prima di una trasmissione del motore. Si
    /// spegne al cambio di frequenza, prima di una trasmissione libera (modi testuali, SSTV) e alla chiusura.
    xit_split: AtomicBool,
    /// Lo Split DX e' stato spento a radio scollegata: la radio puo' averlo ancora acceso, lo si spegne appena torna.
    split_off_pending: AtomicBool,
    /// Ultimo comando di split o del VFO B mandato da Aurora: uno split acceso poco dopo e' di Aurora, piu' tardi viene da fuori.
    split_cmd_at: Mutex<Option<Instant>>,
    /// Conta le letture: lo stato dello split si legge una volta ogni [`SPLIT_CHECK_EVERY`].
    split_checks: AtomicU32,
    /// Cresce a ogni comando: una lettura iniziata prima di un comando non ne cancella l'effetto.
    epoch: AtomicU64,
    /// L'ultima volta che la frequenza letta e' cambiata: per qualche secondo la lettura della sola frequenza va piu' spesso.
    freq_moved_at: Mutex<Option<Instant>>,
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
/// `sat_band_map`: non ancora letta.
const MAIN_SUB_UNKNOWN: u8 = 0;
/// «Main» e' sulla banda bassa (VHF, sotto 300 MHz) e «Sub» su quella alta (UHF).
const MAIN_VHF_SUB_UHF: u8 = 1;
/// «Main» e' sulla banda alta e «Sub» su quella bassa.
const MAIN_UHF_SUB_VHF: u8 = 2;
/// Le due bande non si distinguono (stessa banda, o non si sono potute leggere): si usa la coppia di sempre, «Sub» in trasmissione e «Main» in ricezione.
const MAIN_SUB_SAME: u8 = 3;

/// Il modello Hamlib dell'IC-820H: a due bande (144 e 430 MHz), con «Main» sulla 144 e «Sub» sulla 430 di fabbrica.
const IC_820H_MODEL: u32 = 3032;

/// IC-820H con trame CI-V dirette: fra due scambi di banda passano almeno questi secondi (ogni scambio disturba la ricezione).
const CIV_MIN_GAP: Duration = Duration::from_secs(5);
/// Dopo quanto una prova o una scrittura CI-V fallita si riprova da sola.
const CIV_RETRY: Duration = Duration::from_secs(15);

/// Righe del registro dei passi CI-V tenute per il pacchetto di assistenza.
const CIV_LOG_MAX: usize = 60;

/// Lo stato della trasmissione con trame CI-V dirette dell'IC-820H.
#[derive(Default)]
struct CivUplink {
    /// `None` = prova non ancora fatta; `Some(Ok)` = riuscita; `Some(Err)` = fallita (non si scrive piu' fino alla riconnessione).
    probe: Option<Result<crate::ic820h_civ::Probe, String>>,
    /// Ripiego spento dopo una scrittura non confermata (motivo).
    disabled: Option<String>,
    /// Quando la prova e' fallita o il ripiego si e' spento: dopo `CIV_RETRY` si ricomincia da capo senza riconnettere (SAT spento e
    /// riacceso sulla radio: la prova falliva con le due bande uguali e poi non si riprovava piu').
    failed_at: Option<Instant>,
    last_write: Option<Instant>,
    /// Ultima scrittura confermata: trasmissione e ricezione rilette.
    last: Option<crate::ic820h_civ::Written>,
    /// Passi della prova e delle ultime scritture, con le risposte grezze della radio.
    log: Vec<String>,
    /// Il metodo di trasporto che ha avuto risposta dalla radio.
    transport: Option<crate::ic820h_civ::Transport>,
}

/// Il rapporto della trasmissione con trame CI-V dirette (stato dei satelliti e pacchetto di assistenza).
#[derive(Debug, Clone, Serialize)]
pub struct CivUplinkReport {
    /// «non provata», «riuscita», «fallita», «spenta».
    pub stato: String,
    /// Il motivo (prova fallita o scrittura non confermata) o, se riuscita, quali comandi portano sulle due bande.
    pub dettaglio: String,
    /// Frequenze lette durante la prova: ricezione e banda secondaria (Hz).
    pub ricezione_hz: Option<u64>,
    pub banda_secondaria_hz: Option<u64>,
    /// Ultima trasmissione scritta e confermata (Hz).
    pub ultima_trasmissione_hz: Option<u64>,
    /// Il metodo con cui la radio ha risposto («W eco+risposta», «W solo risposta»), vuoto se nessuno.
    pub trasporto: String,
    /// I passi con i byte inviati e ricevuti (e il metodo) di ogni trama.
    pub passi: Vec<String>,
}

/// Attesa della risposta di `rigctld` a una trama CI-V: Hamlib per l'IC-820H aspetta la radio 1 s (con i suoi tentativi), qui si lascia
/// un margine; una trama che non risponde costa al massimo questo.
const CIV_WAIT: Duration = Duration::from_secs(4);

/// Le trame CI-V mandate con `+W` (lettura di un numero esatto di byte) sulla sessione di `rigctld`. Il metodo che ha avuto risposta si
/// ricorda; se il metodo dato non riceve niente si prova l'altro (eco piu' risposta, sola risposta).
struct CivOverRigctld<'a> {
    client: &'a RigctldClient,
    transport: Option<crate::ic820h_civ::Transport>,
}

impl crate::ic820h_civ::CivPort for CivOverRigctld<'_> {
    fn exchange(&mut self, body: &[u8]) -> Result<(crate::ic820h_civ::Answer, String), String> {
        use crate::ic820h_civ::{classify, hex, parse_bytes, frame_bytes, frame_text, Answer, Transport};
        let first = self.transport.unwrap_or(Transport::EchoAndReply);
        let mut notes = Vec::new();
        let mut last_err = None;
        for method in [first, first.other()] {
            let count = method.count(body);
            match self.client.send_raw_rx_reply(&frame_text(body), count, CIV_WAIT) {
                Ok((lines, code)) => {
                    let got = parse_bytes(&lines);
                    let answer = classify(&got);
                    let received = if got.is_empty() { "niente (timeout di Hamlib)".to_string() } else { hex(&got) };
                    notes.push(format!("{} ({count} byte): inviati {} | ricevuti {received} | RPRT {code}", method.name(), hex(&frame_bytes(body))));
                    if answer != Answer::Nothing {
                        self.transport = Some(method);
                        return Ok((answer, notes.join(" ; ")));
                    }
                }
                Err(e) => {
                    notes.push(format!("{} ({count} byte): inviati {} | errore {}", method.name(), hex(&frame_bytes(body)), e.0));
                    last_err = Some(e.0);
                }
            }
        }
        // nessun metodo ha avuto risposta: «nessuna risposta» (la prova fallisce, niente scritto); un errore della sessione resta errore
        match last_err {
            Some(e) if notes.iter().all(|n| n.contains("| errore ")) => Err(format!("{e} [{}]", notes.join(" ; "))),
            _ => Ok((Answer::Nothing, notes.join(" ; "))),
        }
    }
}

/// La frequenza e' sulla banda alta (UHF e oltre)?
fn is_uhf(hz: u64) -> bool {
    hz >= 300_000_000
}

/// La mappa delle bande dalle frequenze lette su «Main» e su «Sub».
fn band_map_of(main_hz: u64, sub_hz: u64) -> u8 {
    match (is_uhf(main_hz), is_uhf(sub_hz)) {
        (false, true) => MAIN_VHF_SUB_UHF,
        (true, false) => MAIN_UHF_SUB_VHF,
        _ => MAIN_SUB_SAME,
    }
}

/// Su quale VFO scrivere la trasmissione e a quale tornare per la ricezione, data la mappa delle bande e la frequenza di trasmissione: la
/// trasmissione va sul VFO della sua banda, la ricezione sull'altro (segnalazione #83: IC-820H con ricezione 437 MHz e trasmissione 145 MHz, la
/// coppia fissa scriveva ogni frequenza nella banda sbagliata). Senza una mappa chiara resta la coppia di sempre.
fn uplink_vfos(map: u8, tx_hz: u64) -> (&'static str, &'static str) {
    match map {
        MAIN_VHF_SUB_UHF => if is_uhf(tx_hz) { ("Sub", "Main") } else { ("Main", "Sub") },
        MAIN_UHF_SUB_VHF => if is_uhf(tx_hz) { ("Main", "Sub") } else { ("Sub", "Main") },
        _ => ("Sub", "Main"),
    }
}

/// La trasmissione riletta dopo la scrittura e' quella scritta: stessa banda (niente ripiego di 300 MHz) e scarto entro 1 MHz (Hamlib puo'
/// restituire la lettura di poco prima, ma non una frequenza di un'altra parte della banda).
fn uplink_readback_ok(written: u64, read: u64) -> bool {
    is_uhf(written) == is_uhf(read) && written.abs_diff(read) <= 1_000_000
}

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

/// Lo stato dello split si rilegge una lettura ogni tante (un comando in piu' sulla seriale, FT-2000 compresa).
const SPLIT_CHECK_EVERY: u32 = 3;
/// Uno split acceso entro questo tempo da un comando di Aurora e' di Aurora.
const SPLIT_OWN_GRACE: Duration = Duration::from_secs(15);

impl Inner {
    /// Segna (e scrive nel registro) un comando di split o del VFO B mandato da Aurora, con il motivo: cosi' il registro dice chi ha
    /// acceso lo split, e la lettura periodica distingue uno split di Aurora da uno acceso fuori.
    fn split_command(&self, what: &str, reason: &str) {
        tracing::info!("split: {what} ({reason})");
        if let Ok(mut at) = self.split_cmd_at.lock() {
            *at = Some(Instant::now());
        }
    }

    /// Spegne lo split sul VFO di ricezione, ma solo se la radio sa farlo: dopo il primo «non disponibile» (-4, -11) non lo si riprova piu'
    /// (un solo avviso). `Ok(false)` = non mandato o rifiutato come non disponibile.
    fn try_split_off(&self, link: &Link, rx_vfo: &str, reason: &str) -> Result<bool, RigError> {
        if self.split_unsupported.load(Ordering::SeqCst) {
            return Ok(false);
        }
        self.split_command(&format!("S 0 {rx_vfo}"), reason);
        match link.client().set_split(false, rx_vfo) {
            Ok(()) => Ok(true),
            Err(e) if matches!(crate::hamlib::error_code(&e), Some(-4 | -11)) => {
                if !self.split_unsupported.swap(true, Ordering::SeqCst) {
                    tracing::warn!("la radio non ha lo split da CAT ({}): non si riprova a spegnerlo", friendly_error(&e.0));
                }
                Ok(false)
            }
            Err(e) => Err(e),
        }
    }

    /// Aurora ha mandato un comando di split da poco.
    fn split_commanded_recently(&self) -> bool {
        self.split_cmd_at.lock().ok().and_then(|at| *at).is_some_and(|t| t.elapsed() < SPLIT_OWN_GRACE)
    }

    /// Lo split letto dalla radio. Acceso senza un comando di Aurora negli ultimi secondi e con Aurora che lo sapeva spento: viene da fuori
    /// (la radio a mano, un programma collegato alla condivisione). Si scrive un avviso e lo si segna per la pagina.
    fn observe_split(&self, on: bool) {
        let known = self.status.lock().ok().and_then(|s| s.split);
        if on && known != Some(true) && !self.split_commanded_recently() {
            tracing::warn!(
                "split acceso fuori da Aurora (radio o programma collegato alla condivisione): nessun comando di split di Aurora negli ultimi {} s",
                SPLIT_OWN_GRACE.as_secs()
            );
            self.update(|s| s.split_external_ms = now_ms());
        }
        if known != Some(on) {
            self.update(|s| s.split = Some(on));
        }
    }

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
            split_external_ms: 0,
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

    /// Scrive il modo della radio secondo la scelta «Modo» (dati/USB). Se la radio non accetta `PKTUSB` (modo dati)
    /// il comando di Hamlib fallisce e lei resta nel modo precedente: una Kenwood TS-480, per esempio, restava in
    /// LSB e sui modi digitali delle bande basse non si vedeva niente. Si ripiega allora su `USB` (per FT8/FT4/JT65
    /// la banda laterale giusta e' sempre la superiore) e il ripiego resta per la sessione, senza riprovarlo a ogni banda.
    fn set_cat_mode(&self, link: &Link) -> Option<String> {
        let wanted = if self.pktusb_fallback.load(Ordering::SeqCst) {
            "USB"
        } else {
            self.config.cat_mode.hamlib_name()?
        };
        let current = link.client().mode().map(|m| m.0).ok();
        if current.as_deref().is_some_and(|c| mode_satisfies(wanted, c)) {
            return Some(wanted.to_string());
        }
        match link.client().set_mode(wanted, 0) {
            Ok(()) => {
                self.update(|s| s.mode = Some(wanted.to_string()));
                Some(wanted.to_string())
            }
            Err(e) if wanted == "PKTUSB" => {
                tracing::warn!("modo dati {wanted} non supportato dalla radio: si usa USB ({e})");
                self.pktusb_fallback.store(true, Ordering::SeqCst);
                // se la radio e' gia' in USB non si riscrive: una Icom senza modo dati in Hamlib (IC-756PRO) puo' essere su USB-D, che Hamlib
                // legge come «USB»: scrivere USB la toglierebbe dal modo dati scelto sulla radio
                if current.as_deref().is_some_and(|c| mode_satisfies("USB", c)) {
                    tracing::info!("la radio e' gia' in USB: il modo non si riscrive (se e' su USB-D resta cosi')");
                    return Some("USB".to_string());
                }
                match link.client().set_mode("USB", 0) {
                    Ok(()) => {
                        self.update(|s| s.mode = Some("USB".to_string()));
                        Some("USB".to_string())
                    }
                    Err(e2) => {
                        tracing::warn!("nemmeno USB e' stato scritto: {e2}");
                        None
                    }
                }
            }
            Err(e) => {
                tracing::warn!("modo {wanted} non scritto: {e}");
                None
            }
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
        // se la radio e' in trasmissione (per un guasto, o per un PTT rimasto acceso) lo si dice nel registro; la lettura non c'e' con tutte le radio
        let found_on = self.ptt_readable.load(Ordering::SeqCst) && link.client().ptt_within(Duration::from_millis(500)).unwrap_or(false);
        match link.client().set_ptt(false) {
            Ok(()) => {
                if found_on {
                    if self.opened_once.load(Ordering::SeqCst) {
                        tracing::info!("PTT trovato acceso dopo la riconnessione: spento");
                    } else {
                        tracing::info!("PTT trovato acceso all'avvio: spento");
                    }
                }
                // il PTT e' spento davvero: lo stato non resta «acceso» dopo un guasto (il controllo della trasmissione se ne accorge)
                self.update(|s| s.ptt = false);
            }
            Err(e) => tracing::debug!("PTT spento dopo l'apertura: {e}"),
        }
        self.opened_once.store(true, Ordering::SeqCst);
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
        if let Some(name) = self.set_cat_mode(&link) {
            mode = link.client().mode().ok().or(Some((name, 0)));
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
        // radio ricollegata (o riavviata): i comandi rifiutati si riprovano una volta
        self.split_unsupported.store(false, Ordering::SeqCst);
        self.sat_vfo_pair.store(0, Ordering::SeqCst);
        self.sat_band_map.store(MAIN_SUB_UNKNOWN, Ordering::SeqCst);
        if let Ok(mut civ) = self.civ.lock() {
            *civ = CivUplink::default();
        }
        self.session.fetch_add(1, Ordering::SeqCst);
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
            match self.try_split_off(&link, rx_vfo, "split DX spento mentre la radio era scollegata") {
                Ok(true) => self.update(|s| s.split = Some(false)),
                Ok(false) => {}
                Err(e) => tracing::warn!("split non spento dopo la riconnessione: {}", friendly_error(&e.0)),
            }
        }
        // la radio riavviata ha perso anche lo Split DX: si rimette (l'indicatore del cluster lo dice ancora acceso e l'operatore crede di
        // trasmettere sul VFO B); se non si riesce si spegne il flag, cosi' torna lo split di FT8
        let dx_tx = self.dx_split_tx.load(Ordering::SeqCst);
        if self.dx_split.load(Ordering::SeqCst) && dx_tx != 0 && let Ok(link) = self.connected_link() {
            // solo se la radio e' ancora sulla frequenza dello spot: se nel frattempo l'operatore si e' spostato (manopola, banda, altra
            // pagina) lo Split DX non e' piu' voluto e si dimentica; prima tornava da solo alla riconnessione, anche ore dopo, mettendo la
            // radio in split su una frequenza vecchia mentre l'operatore non toccava niente (proprietario, 2026-11-14)
            let rx = self.dx_split_rx.load(Ordering::SeqCst);
            let here = link.client().frequency().ok();
            if rx == 0 || here.is_none_or(|f| f.abs_diff(rx) > 2_000) {
                self.dx_split.store(false, Ordering::SeqCst);
                self.dx_split_tx.store(0, Ordering::SeqCst);
                self.dx_split_rx.store(0, Ordering::SeqCst);
                tracing::info!("split DX dimenticato: la radio non e' piu' sulla frequenza dello spot");
            } else {
                let tx_vfo = if self.vfo_reversed.load(Ordering::SeqCst) { "VFOA" } else { "VFOB" };
                self.split_command(&format!("I {dx_tx}, S 1 {tx_vfo}"), "split DX rimesso dopo la riconnessione");
                self.write_split_mode(&link);
                let set = link.client().set_split_frequency(dx_tx).and_then(|()| link.client().set_split(true, tx_vfo));
                match set {
                    Ok(()) => self.update(|s| s.split = Some(true)),
                    Err(e) => {
                        tracing::warn!("split DX non rimesso dopo la riconnessione: {}", friendly_error(&e.0));
                        self.dx_split.store(false, Ordering::SeqCst);
                    }
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
    /// Quanto dopo che Aurora ha spento il PTT una radio che risulta in trasmissione si considera «rimasta» o «tornata» in trasmissione da sola.
    const STRAY_WINDOW: Duration = Duration::from_secs(30);

    /// La radio dice «in trasmissione» ma Aurora non l'ha accesa e l'ha spenta da poco: con alcune radio (TS-850 con la seriale USB, pacchetto
    /// 2948CDCC) la radio resta o torna in trasmissione dopo il nostro `T 0` e nessuno la spegneva piu', mentre Aurora, vedendo il PTT acceso,
    /// non decodificava piu'. Dopo due letture di fila si rimanda `T 0`, al massimo tre volte per episodio (il PTT acceso a mano dall'operatore
    /// dopo la finestra non si tocca, e un PTT tenuto da un filo RTS/DTR non lo spegne comunque il CAT).
    fn watch_stray_ptt(&self, link: &Link, radio_ptt: bool) {
        if !radio_ptt || self.keyed_by_us.load(Ordering::SeqCst) || self.external_tx.load(Ordering::SeqCst) {
            self.stray_polls.store(0, Ordering::SeqCst);
            if !radio_ptt {
                self.stray_resent.store(0, Ordering::SeqCst);
            }
            return;
        }
        let recent = self.last_unkeyed.lock().ok().and_then(|t| *t).is_some_and(|t| t.elapsed() <= Self::STRAY_WINDOW);
        if !recent || self.stray_polls.fetch_add(1, Ordering::SeqCst) + 1 < 2 {
            return;
        }
        self.stray_polls.store(0, Ordering::SeqCst);
        let sent = self.stray_resent.fetch_add(1, Ordering::SeqCst);
        if sent >= 3 {
            return;
        }
        tracing::warn!("CAT: la radio risulta in trasmissione ma Aurora aveva spento il PTT: rimando T 0 ({} di 3)", sent + 1);
        if let Err(e) = self.ptt_off(link, 1) {
            tracing::warn!("CAT: T 0 non passato: {}", e.0);
        }
    }

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
        if let Some(p) = ptt {
            self.watch_stray_ptt(&link, p);
        }
        if let Some(f) = freq {
            self.rx_dial.store(f, Ordering::SeqCst);
            if self.status.lock().is_ok_and(|s| s.frequency_hz.is_some_and(|old| old != f)) {
                self.note_freq_moved();
            }
        }
        // lo split della radio, ogni tante letture e solo in ricezione: chi lo accende fuori da Aurora (radio, programma sulla condivisione).
        // Solo con «Leggi anche gli altri dati della radio»: su alcune radio `get_split_vfo` di Hamlib commuta davvero il VFO per leggerlo
        // e la radio saltava fra VFO A e VFO B a ogni lettura (#67, dalla 0.11.258 in cui la lettura era sempre accesa)
        let split = if self.config.read_extras
            && freq.is_some()
            && self.split_checks.fetch_add(1, Ordering::SeqCst) % SPLIT_CHECK_EVERY == 0
        {
            client.split().ok().map(|(on, _)| on)
        } else {
            None
        };
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
        if fresh && let Some(on) = split {
            self.observe_split(on);
        }
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

    fn note_freq_moved(&self) {
        if let Ok(mut at) = self.freq_moved_at.lock() {
            *at = Some(Instant::now());
        }
    }

    /// Fra quanto rileggere la sola frequenza: presto se la sintonia si e' mossa da poco, piu' piano a riposo; `None` = lettura spenta.
    fn freq_interval(&self) -> Option<Duration> {
        if self.timing.freq_idle.is_zero() {
            return None;
        }
        let moving = self.freq_moved_at.lock().ok().and_then(|at| *at).is_some_and(|t| t.elapsed() < self.timing.freq_active_for);
        Some(if moving { self.timing.freq_fast } else { self.timing.freq_idle })
    }

    /// Legge solo la frequenza (`f`), per seguire la sintonia girata a mano senza aspettare la lettura completa. Non si fa in trasmissione
    /// (frequenza e modo competono con il PTT sullo stesso bus, `docs/analisi-decodium-cat.md`), con una lettura completa in coda o con la
    /// sessione di `rigctld` occupata da un comando: in quei casi si salta il giro. Un errore non conta come guasto (lo conta la lettura completa).
    fn read_frequency(&self) {
        if self.transmitting() || self.quick.lock().is_ok_and(|q| !q.is_empty()) {
            return;
        }
        let Some(link) = self.link() else { return };
        if !link.alive() || self.status.lock().map_or(true, |s| s.state != LinkState::Connected) {
            return;
        }
        let client = link.client();
        if client.busy() {
            return;
        }
        let epoch = self.epoch.load(Ordering::SeqCst);
        match client.frequency() {
            Ok(f) if valid_dial_hz(f) => {
                // un comando arrivato nel frattempo (cambio di frequenza, PTT) vince sulla lettura
                if self.epoch.load(Ordering::SeqCst) != epoch || self.transmitting() {
                    return;
                }
                let old = self.status.lock().ok().and_then(|s| s.frequency_hz);
                if old != Some(f) {
                    self.rx_dial.store(f, Ordering::SeqCst);
                    self.update(|s| s.frequency_hz = Some(f));
                    if old.is_some() {
                        self.note_freq_moved();
                    }
                }
            }
            Ok(_) => {}
            Err(e) => tracing::debug!("lettura veloce della frequenza fallita: {e}"),
        }
    }

    /// Aspetta il prossimo giro di lettura completa, leggendo intanto la sola frequenza; `false` = si e' chiuso.
    fn wait_next(&self) -> bool {
        let delay = if self.transmitting() { (self.timing.poll / 4).max(self.timing.tx_poll_min) } else { self.timing.poll };
        let until = Instant::now() + delay;
        let mut freq_at = self.freq_interval().map(|d| Instant::now() + d);
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
            if let Some(at) = freq_at
                && now >= at
            {
                // a meno di un intervallo dalla lettura completa non serve: la frequenza la legge lei
                if until.saturating_duration_since(now) > self.timing.freq_fast {
                    self.read_frequency();
                }
                freq_at = self.freq_interval().map(|d| Instant::now() + d);
                continue;
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
        let mut tries = 0u32;
        while let Err(e) = self.open() {
            if self.stop.load(Ordering::SeqCst) {
                return;
            }
            let wait = self.timing.reconnect_step;
            tries += 1;
            // nel registro il motivo intero (le righe di rigctld): a schermo resta il messaggio breve. Con la radio spenta si riprova per ore
            // ogni 10 s: l'avviso si scrive al primo tentativo e poi ogni 30 (circa 5 minuti), gli altri solo a livello di debug; prima erano
            // quasi 5.000 al giorno per PC e riempivano i rapporti
            if tries == 1 || tries % 30 == 0 {
                tracing::warn!("CAT: collegamento con rigctld non riuscito (tentativo {tries}): {}", e.0);
            } else {
                tracing::debug!("CAT: collegamento con rigctld non riuscito (tentativo {tries}): {}", e.0);
            }
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
            // riapertura chiesta dallo spegnimento del PTT che non riesce: la fa questo thread, con T 0 appena la radio risponde (`open`)
            if self.reconnect_wanted.swap(false, Ordering::SeqCst) {
                failures = 0;
                if !self.reconnect("PTT non spento: riapertura del collegamento") {
                    return;
                }
                continue;
            }
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
            if self.config.read_tx_power && (poll.memory.supported(Level::PowerWatts) || poll.memory.supported(Level::PowerFraction)) {
                steps.push(Step::Power);
            }
            if self.config.read_tx_swr {
                steps.extend(TX_LEVELS.iter().filter(|l| poll.memory.supported(**l)).map(|l| Step::Level(*l)));
            }
            if self.config.read_tx_alc && poll.memory.supported(Level::Alc) {
                steps.push(Step::Level(Level::Alc));
            }
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
        self.split_command(&format!("I {}, S 1 {tx_vfo}", split_tx_dial_hz(dial, audio)), "split di FT8 «Split: Radio» prima del PTT");
        self.write_split_mode(link);
        link.client().set_split_frequency(split_tx_dial_hz(dial, audio))?;
        link.client().set_split(true, tx_vfo)?;
        self.xit_split.store(true, Ordering::SeqCst);
        self.update(|s| s.split = Some(true));
        Ok(())
    }

    /// Il VFO di trasmissione prende lo stesso modo della ricezione: quello di «Modo scritto nella radio» (USB, PKTUSB) o, senza scelta,
    /// quello letto dalla radio. Prima si spostava solo la frequenza e il VFO B teneva il suo modo: dopo un cambio di banda la radio
    /// trasmetteva in LSB (segnalazione del 2026-11-14, Hamlib con «Split: Radio»). Un errore non ferma lo split: si scrive nel registro.
    fn write_split_mode(&self, link: &Link) {
        let (current, passband) = self.status.lock().map(|s| (s.mode.clone(), s.passband_hz.unwrap_or(0))).unwrap_or((None, 0));
        // modo scelto: banda normale del modo (0); modo copiato dalla ricezione: anche la sua banda passante
        // la radio senza modo dati (ripiego su USB, vedi `set_cat_mode`): anche il VFO di trasmissione va in USB
        let chosen = if self.pktusb_fallback.load(Ordering::SeqCst) { Some("USB") } else { self.config.cat_mode.hamlib_name() };
        let Some((mode, passband)) = chosen.map(|m| (m.to_string(), 0)).or(current.map(|m| (m, passband))) else {
            return;
        };
        // radio senza `X` e senza copia A=B (scoperto prima): il modo di trasmissione segue quello di ricezione, niente da riprovare
        if self.split_mode_unsupported.load(Ordering::SeqCst) {
            return;
        }
        self.split_command(&format!("X {mode} {passband}"), "modo del VFO di trasmissione");
        if let Err(e) = link.client().set_split_mode(&mode, passband) {
            // `X` non supportato da radio o modello: non e' un guasto, solo un ripiego; un errore vero (porta, timeout) resta un avviso
            let x_unsupported = hamlib_unsupported(&e);
            if !x_unsupported {
                // radio o modello di Hamlib senza `X`: si copia il VFO di ricezione in quello di trasmissione (A=B, modo compreso); la
                // frequenza di trasmissione si riscrive subito dopo da chi chiama
                tracing::warn!("modo {mode} non scritto nel VFO di trasmissione ({}): copia A=B", friendly_error(&e.0));
            }
            self.split_command("G CPY", "copia A=B, ripiego del modo del VFO di trasmissione");
            if let Err(c) = link.client().vfo_op("CPY") {
                if x_unsupported && hamlib_unsupported(&c) {
                    // ne' `X` ne' la copia: una sola riga di informazione, poi la radio non si interroga piu' su questo
                    if !self.split_mode_unsupported.swap(true, Ordering::SeqCst) {
                        tracing::info!("la radio non supporta la copia A=B: il modo di trasmissione segue quello di ricezione");
                    }
                } else {
                    if x_unsupported {
                        tracing::warn!("modo {mode} non scritto nel VFO di trasmissione ({}): copia A=B", friendly_error(&e.0));
                    }
                    tracing::warn!("copia A=B non riuscita: {}", friendly_error(&c.0));
                }
            }
        }
    }

    /// PTT spento con `tries` tentativi. `Ok` se la radio ha confermato.
    fn ptt_off(&self, link: &Link, tries: u32) -> Result<(), RigError> {
        let mut last = RigError("nessun tentativo".into());
        for i in 0..tries {
            match link.client().set_ptt(false) {
                Ok(()) => {
                    self.keyed_by_us.store(false, Ordering::SeqCst);
                    return Ok(());
                }
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
            pktusb_fallback: AtomicBool::new(false),
            split_mode_unsupported: AtomicBool::new(false),
            split_unsupported: AtomicBool::new(false),
            session: AtomicU64::new(0),
            sat_vfo_pair: AtomicU8::new(0),
            sat_band_map: AtomicU8::new(MAIN_SUB_UNKNOWN),
            civ: Mutex::new(CivUplink::default()),
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
                split_external_ms: 0,
                failures: 0,
                reconnect_attempt: 0,
                ptt_blocked: false,
                updated_ms: now_ms(),
            }),
            stop: AtomicBool::new(false),
            ptt_blocked_until: Mutex::new(None),
            quick: Mutex::new(Vec::new()),
            ptt_readable: AtomicBool::new(true),
            opened_once: AtomicBool::new(false),
            reconnect_wanted: AtomicBool::new(false),
            reconnect_asked_at: Mutex::new(None),
            external_tx: AtomicBool::new(false),
            keyed_by_us: AtomicBool::new(false),
            last_unkeyed: Mutex::new(None),
            stray_polls: AtomicU32::new(0),
            stray_resent: AtomicU32::new(0),
            tx_audio_hz: AtomicU32::new(0),
            rx_dial: AtomicU64::new(0),
            emulate_restore: Mutex::new(None),
            vfo_reversed: AtomicBool::new(false),
            dx_split: AtomicBool::new(false),
            dx_split_tx: AtomicU64::new(0),
            dx_split_rx: AtomicU64::new(0),
            xit_split: AtomicBool::new(false),
            split_off_pending: AtomicBool::new(false),
            split_cmd_at: Mutex::new(None),
            split_checks: AtomicU32::new(0),
            epoch: AtomicU64::new(0),
            freq_moved_at: Mutex::new(None),
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

    /// PTT acceso o spento. Acceso: solo a radio collegata e passata l'attesa dopo il collegamento. Spento: un tentativo (i successivi li fa chi spegne), e se fallisce lo stato
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
            inner.keyed_by_us.store(true, Ordering::SeqCst);
            inner.stray_resent.store(0, Ordering::SeqCst);
            if let Err(e) = link.client().set_ptt_source(true, inner.config.ptt_data) {
                inner.keyed_by_us.store(false, Ordering::SeqCst);
                return Err(RigError(friendly_error(&e.0)));
            }
        } else {
            // un solo tentativo: la scala dei tentativi e' di chi spegne (`tx_native::ptt_release`), che non deve moltiplicarla
            inner.ptt_off(&link, 1).map_err(|e| RigError(format!("PTT non spento: {}", friendly_error(&e.0))))?;
            // da ora la radio che risulta in trasmissione senza che l'abbiamo accesa e' una radio «rimasta» o «tornata» in TX (`watch_stray_ptt`)
            if let Ok(mut t) = inner.last_unkeyed.lock() {
                *t = Some(Instant::now());
            }
            self.emulate_end(&link);
        }
        inner.update(|s| s.ptt = on);
        std::thread::sleep(inner.timing.ptt_pause);
        inner.queue_quick_reads();
        Ok(())
    }

    /// Un'altra via per spegnere il PTT quando `T 0` non passa: riprova sul collegamento in uso e, se fallisce, chiede al thread del collegamento di
    /// riaprirlo (la riapertura manda `T 0` per prima cosa). Non apre mai un secondo collegamento da chi chiama. Una riapertura ogni 10 s al massimo.
    pub fn force_ptt_off(&self) -> Result<(), RigError> {
        let inner = &self.inner;
        if inner.config.ptt == PttMethod::Vox {
            return Ok(());
        }
        let direct = match inner.link() {
            Some(link) => inner.ptt_off(&link, 1),
            None => Err(RigError("radio non collegata".into())),
        };
        let Err(e) = direct else {
            inner.update(|s| s.ptt = false);
            return Ok(());
        };
        // senza collegamento il thread sta gia' riaprendo da solo
        if inner.link().is_some() {
            let mut asked = inner.reconnect_asked_at.lock().map_err(|_| RigError("stato interno non disponibile".into()))?;
            if asked.is_none_or(|t| t.elapsed() >= Duration::from_secs(10)) {
                *asked = Some(Instant::now());
                inner.reconnect_wanted.store(true, Ordering::SeqCst);
            }
        }
        Err(RigError(friendly_error(&e.0)))
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
        inner.try_split_off(&link, rx_vfo, "split di FT8 tolto prima di una trasmissione libera").map_err(|e| RigError(friendly_error(&e.0)))?;
        inner.xit_split.store(false, Ordering::SeqCst);
        inner.update(|s| s.split = Some(false));
        Ok(())
    }

    /// «Spegni split»: la radio torna in simplex sul VFO di ricezione anche se Aurora non sapeva che fosse in split (acceso a mano sulla
    /// radio, da un altro programma o rimasto da prima). Non in trasmissione.
    pub fn force_split_off(&self) -> Result<(), RigError> {
        let inner = &self.inner;
        let link = inner.connected_link()?;
        if inner.transmitting() {
            return Err(RigError("la radio sta trasmettendo: split non spento".into()));
        }
        inner.epoch.fetch_add(1, Ordering::SeqCst);
        inner.tx_audio_hz.store(0, Ordering::SeqCst);
        let rx_vfo = if inner.vfo_reversed.load(Ordering::SeqCst) { "VFOB" } else { "VFOA" };
        inner.split_command(&format!("S 0 {rx_vfo}"), "«Spegni split»");
        // subito alla radio, prima delle letture periodiche in coda
        link.client().urgent(|c| c.set_split(false, rx_vfo)).map_err(|e| RigError(friendly_error(&e.0)))?;
        // una lettura partita mentre il comando aspettava ha letto lo split ancora acceso: la si scarta, se no il display lo rimostrava
        // acceso fino al controllo seguente
        inner.epoch.fetch_add(1, Ordering::SeqCst);
        inner.xit_split.store(false, Ordering::SeqCst);
        inner.dx_split.store(false, Ordering::SeqCst);
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

    /// Il VFO in uso secondo Hamlib («VFOA», «VFOB»...), se la radio lo dice (le Icom piu' vecchie no: Hamlib se lo ricorda da solo).
    pub fn current_vfo(&self) -> Option<String> {
        let link = self.inner.connected_link().ok()?;
        link.client().urgent(|c| c.vfo()).ok().filter(|v| !v.is_empty())
    }

    /// Rimette la radio sul VFO dato (`V VFOA`).
    pub fn select_vfo(&self, vfo: &str) -> Result<(), RigError> {
        let link = self.inner.connected_link()?;
        self.inner.epoch.fetch_add(1, Ordering::SeqCst);
        link.client().urgent(|c| c.set_vfo(vfo)).map_err(|e| RigError(friendly_error(&e.0)))?;
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
        if item == crate::extras::ExtraItem::Vfo {
            inner.split_command("V (cambio di VFO)", "clic sull'indicatore del VFO");
        }
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
        self.inner.split_command(&format!("w {text}"), "comando CAT di una macro (<CATCMD>)");
        link.client().send_raw(text).map_err(|e| RigError(friendly_error(&e.0)))
    }

    /// La frequenza del VFO di trasmissione (split), in Hz: i satelliti la correggono col Doppler.
    pub fn set_split_frequency(&self, hz: u64) -> Result<(), RigError> {
        let link = self.inner.connected_link()?;
        // Hamlib 4.6 `rig_set_split_freq`: con lo split spento lo ACCENDE da se' prima di scrivere il VFO B
        self.inner.split_command(&format!("I {hz}"), "satelliti");
        link.client().set_split_frequency(hz).map_err(|e| RigError(friendly_error(&e.0)))
    }

    pub fn set_split(&self, on: bool, tx_vfo: &str) -> Result<(), RigError> {
        let link = self.inner.connected_link()?;
        self.inner.epoch.fetch_add(1, Ordering::SeqCst);
        self.inner.split_command(&format!("S {} {tx_vfo}", u8::from(on)), "satelliti");
        link.client().set_split(on, tx_vfo).map_err(|e| RigError(friendly_error(&e.0)))?;
        self.inner.update(|s| s.split = Some(on));
        Ok(())
    }

    /// Un numero che cambia a ogni collegamento della radio (la memoria dei comandi rifiutati dei satelliti si azzera).
    pub fn session_id(&self) -> u64 {
        (Arc::as_ptr(&self.inner) as usize as u64).wrapping_mul(31).wrapping_add(self.inner.session.load(Ordering::SeqCst))
    }

    /// La ricezione dei satelliti (Doppler): solo `F`, una volta al secondo. Niente controllo dell'arrivo, niente modo riscritto, niente
    /// spegnimento dello split, a differenza di `set_frequency` che e' pensata per un cambio di banda.
    pub fn set_tracking_frequency(&self, hz: u64) -> Result<(), RigError> {
        let inner = &self.inner;
        if !valid_dial_hz(hz) {
            return Err(RigError(format!("frequenza non valida: {hz} Hz")));
        }
        let link = inner.connected_link()?;
        if inner.transmitting() {
            return Err(RigError("la radio sta trasmettendo: cambio di frequenza rifiutato".into()));
        }
        inner.epoch.fetch_add(1, Ordering::SeqCst);
        link.client().set_frequency(hz).map_err(|e| RigError(friendly_error(&e.0)))?;
        inner.rx_dial.store(hz, Ordering::SeqCst);
        inner.update(|s| s.frequency_hz = Some(hz));
        Ok(())
    }

    /// Il modo di ricezione dei satelliti («USB», «LSB», «FM», «CW»), scritto una volta quando cambia.
    pub fn set_tracking_mode(&self, mode: &str) -> Result<(), RigError> {
        let link = self.inner.connected_link()?;
        if self.inner.transmitting() {
            return Err(RigError("la radio sta trasmettendo: modo non cambiato".into()));
        }
        self.inner.epoch.fetch_add(1, Ordering::SeqCst);
        link.client().set_mode(mode, 0).map_err(|e| RigError(friendly_error(&e.0)))?;
        self.inner.update(|s| s.mode = Some(mode.to_string()));
        Ok(())
    }

    /// Il modo del VFO di trasmissione con lo split vero (`X`).
    pub fn set_tracking_tx_mode(&self, mode: &str) -> Result<(), RigError> {
        let link = self.inner.connected_link()?;
        self.inner.split_command(&format!("X {mode} 0"), "satelliti");
        link.client().set_split_mode(mode, 0).map_err(|e| RigError(friendly_error(&e.0)))
    }

    /// Accende o spegne il modo satellite della radio (`U SATMODE`), se Hamlib lo ha per quel modello.
    pub fn set_satmode(&self, on: bool) -> Result<(), RigError> {
        let link = self.inner.connected_link()?;
        link.client().set_func("SATMODE", on).map_err(|e| RigError(friendly_error(&e.0)))
    }

    /// Ripiego per le radio senza split da CAT (sperimentale): la trasmissione dei satelliti sul VFO secondario. Si passa al VFO di
    /// trasmissione, si scrivono frequenza (e modo), si torna al VFO di ricezione, tutto in un blocco che le letture periodiche non
    /// interrompono. Prima `V Sub`/`V Main`, se la radio le rifiuta `V VFOB`/`V VFOA`; la coppia che funziona si ricorda fino alla
    /// riconnessione. `Ok(false)` = la radio sta trasmettendo, non si tocca niente. Qualunque rifiuto e' un errore (il chiamante spegne il
    /// ripiego) e il ritorno al VFO di ricezione si tenta sempre.
    pub fn set_uplink_via_vfo(&self, hz: u64, mode: Option<&str>) -> Result<bool, RigError> {
        let inner = &self.inner;
        if !valid_dial_hz(hz) {
            return Err(RigError(format!("frequenza di trasmissione non valida: {hz} Hz")));
        }
        let link = inner.connected_link()?;
        if inner.transmitting() {
            return Ok(false);
        }
        let reversed = inner.vfo_reversed.load(Ordering::SeqCst);
        let ab: (&str, &str) = if reversed { ("VFOA", "VFOB") } else { ("VFOB", "VFOA") };
        let pairs: [(u8, &str, &str); 2] = [(1, "Sub", "Main"), (2, ab.0, ab.1)];
        let known = inner.sat_vfo_pair.load(Ordering::SeqCst);
        inner.epoch.fetch_add(1, Ordering::SeqCst);
        inner.split_command(&format!("V, F {hz}"), "satelliti: trasmissione sul VFO secondario");
        let out = link.client().urgent(|c| {
            let mut last = RigError("nessun VFO provato".into());
            for (id, tx_vfo, rx_vfo) in pairs {
                if known != 0 && known != id {
                    continue;
                }
                // Main/Sub: se le due bande sono diverse la trasmissione va sul VFO della sua banda (vedi `uplink_vfos`)
                let (tx_vfo, rx_vfo) = if id == 1 {
                    let mut map = inner.sat_band_map.load(Ordering::SeqCst);
                    if map == MAIN_SUB_UNKNOWN {
                        // dopo ogni cambio di VFO si aspetta un attimo: Hamlib tiene in memoria l'ultima frequenza letta e, a lettura troppo vicina,
                        // restituiva la stessa su Main e su Sub (pacchetto di Vladimir, 0.11.318 e 0.11.319: «uguali o non leggibili»)
                        let read = |vfo: &str| -> Result<u64, RigError> {
                            c.set_vfo(vfo)?;
                            std::thread::sleep(Duration::from_millis(700));
                            c.frequency()
                        };
                        let (main, sub) = (read("Main"), read("Sub"));
                        // `V Sub` rifiutato dalla radio (-9): il Sub e' spento, c'e' una banda sola e il «Main» e' quella di ricezione. Scrivere
                        // adesso la trasmissione la metterebbe sopra la ricezione (pacchetto di Vladimir, 2026-11-21): non si scrive niente
                        if let Err(e) = &sub
                            && crate::hamlib::error_code(e) == Some(-9)
                        {
                            tracing::warn!("satelliti: il Sub della radio non risponde ({}): trasmissione non scritta, la ricezione resta dov'e'", friendly_error(&e.0));
                            return Err(RigError("il Sub della radio e' spento: accendilo per trasmettere sul secondo VFO".into()));
                        }
                        tracing::info!("satelliti: lettura delle bande dei VFO: Main = {:?} Hz, Sub = {:?} Hz", main.as_ref().ok(), sub.as_ref().ok());
                        let main_hz = main.as_ref().ok().copied();
                        map = match (main, sub) {
                            (Ok(main), Ok(sub)) => band_map_of(main, sub),
                            _ => MAIN_SUB_SAME,
                        };
                        // lettura senza risposta chiara sull'IC-820H (modello 3032): vale la banda letta su Main (pacchetto 3EFC9CF8: Main = 435,63,
                        // Sub = 445,20, prima si supponeva Main = VHF e la trasmissione 145,96 finiva sopra la ricezione, riletta 445,96); senza
                        // nessuna lettura, la disposizione di fabbrica (Main = 144, Sub = 430). Il controllo prima della scrittura fa il resto.
                        if map == MAIN_SUB_SAME && inner.config.model == IC_820H_MODEL {
                            map = match main_hz {
                                Some(hz) if is_uhf(hz) => MAIN_UHF_SUB_VHF,
                                _ => MAIN_VHF_SUB_UHF,
                            };
                        }
                        inner.sat_band_map.store(map, Ordering::SeqCst);
                        tracing::info!("satelliti: bande dei VFO della radio: {}", match map {
                            MAIN_VHF_SUB_UHF => "Main = VHF, Sub = UHF",
                            MAIN_UHF_SUB_VHF => "Main = UHF, Sub = VHF",
                            _ => "uguali o non leggibili (coppia di sempre)",
                        });
                    }
                    uplink_vfos(map, hz)
                } else {
                    (tx_vfo, rx_vfo)
                };
                match c.set_vfo(tx_vfo) {
                    Ok(()) => {
                        inner.sat_vfo_pair.store(id, Ordering::SeqCst);
                        // IC-820H: prima di scrivere si legge il VFO di trasmissione; se non e' nella banda della trasmissione (e' quello della
                        // ricezione, o Hamlib non ha cambiato banda) non si scrive niente
                        if id == 1 && inner.config.model == IC_820H_MODEL {
                            std::thread::sleep(Duration::from_millis(700));
                            match c.frequency() {
                                Ok(here) if is_uhf(here) == is_uhf(hz) => {}
                                here => {
                                    let _ = c.set_vfo(rx_vfo);
                                    let read = here.map_or_else(|e| format!("errore: {}", e.0), |f| format!("{f} Hz"));
                                    tracing::warn!("satelliti: il VFO {tx_vfo} legge {read}, non la banda della trasmissione {hz} Hz: niente scritto");
                                    return Err(RigError(format!("il VFO {tx_vfo} della radio legge {read}, non la banda della trasmissione: niente scritto")));
                                }
                            }
                        }
                        let wrote = c.set_frequency(hz).and_then(|()| mode.map_or(Ok(()), |m| c.set_mode(m, 0)));
                        // la frequenza si rilegge dal VFO di trasmissione prima di tornare a quello di ricezione: se la radio l'ha ripiegata in
                        // un'altra banda o non l'ha presa (IC-820H, #83: scritta 145,96 sulla banda di ricezione, riletta 445,96) non e' riuscita
                        let checked = match (&wrote, c.frequency()) {
                            (Ok(()), Ok(read)) => {
                                tracing::info!("satelliti: trasmissione scritta {hz} Hz sul VFO {tx_vfo}, riletta {read} Hz (scarto {} Hz)", read.abs_diff(hz));
                                if uplink_readback_ok(hz, read) {
                                    Ok(())
                                } else {
                                    tracing::warn!("satelliti: la trasmissione non e' arrivata sul VFO {tx_vfo} (riletta {read} Hz invece di {hz} Hz)");
                                    Err(RigError(format!("la trasmissione {hz} Hz non e' arrivata sul VFO {tx_vfo}: la radio rilegge {read} Hz")))
                                }
                            }
                            (Ok(()), Err(e)) => {
                                tracing::info!("satelliti: trasmissione scritta {hz} Hz sul VFO {tx_vfo}, rilettura non riuscita: {}", e.0);
                                Ok(())
                            }
                            (Err(_), _) => Ok(()),
                        };
                        let back = c.set_vfo(rx_vfo);
                        return wrote.and(checked).and(back).map(|()| true);
                    }
                    Err(e) if crate::meters::is_unsupported(&e) => last = e,
                    Err(e) => return Err(e),
                }
            }
            Err(RigError(format!("la radio non accetta il cambio di VFO ({})", last.0)))
        });
        inner.epoch.fetch_add(1, Ordering::SeqCst);
        out.map_err(|e| RigError(friendly_error(&e.0)))
    }

    /// IC-820H (modello 3032, solo quello): la frequenza di trasmissione dei satelliti scritta sulla banda secondaria con trame CI-V dirette
    /// (vedi [`crate::ic820h_civ`]). La prima volta fa la prova di sicurezza (selezione e letture, nessuna scrittura); se fallisce non scrive
    /// mai fino alla riconnessione. `Ok(false)` = non adesso (in trasmissione, o troppo presto dopo l'ultimo scambio). Un errore spegne il ripiego.
    pub fn set_uplink_via_civ(&self, hz: u64) -> Result<bool, RigError> {
        let inner = &self.inner;
        if inner.config.model != IC_820H_MODEL {
            return Err(RigError(format!("comandi CI-V diretti solo per l'IC-820H (modello Hamlib {IC_820H_MODEL}), non per il modello {}", inner.config.model)));
        }
        if !valid_dial_hz(hz) {
            return Err(RigError(format!("frequenza di trasmissione non valida: {hz} Hz")));
        }
        let link = inner.connected_link()?;
        let mut civ = inner.civ.lock().map_err(|_| RigError("stato CI-V non disponibile".into()))?;
        if civ.failed_at.is_some_and(|t| t.elapsed() >= CIV_RETRY) {
            civ.probe = None;
            civ.disabled = None;
            civ.failed_at = None;
        }
        if let Some(why) = &civ.disabled {
            return Err(RigError(format!("trasmissione CI-V spenta: {why}")));
        }
        if let Some(Err(why)) = &civ.probe {
            return Err(RigError(format!("prova CI-V fallita: {why}")));
        }
        if civ.last_write.is_some_and(|t| t.elapsed() < CIV_MIN_GAP) {
            return Ok(false);
        }
        // mai mentre si trasmette: lo si guarda subito prima di cambiare banda
        if inner.transmitting() {
            return Ok(false);
        }
        inner.epoch.fetch_add(1, Ordering::SeqCst);
        inner.split_command("w 07 D1/05/07 D0", "satelliti: trasmissione CI-V diretta (IC-820H)");
        let mut log = Vec::new();
        let known = civ.probe.clone();
        let mut transport = civ.transport;
        let mut probed = None;
        let mut wrong_band = false;
        let out = link.client().urgent(|c| {
            let mut port = CivOverRigctld { client: c, transport };
            let probe = match known {
                Some(Ok(p)) => p,
                _ => {
                    let result = crate::ic820h_civ::probe(&mut port, &mut log);
                    match &result {
                        Ok(p) => tracing::info!("satelliti: prova CI-V dell'IC-820H riuscita: ricezione {} Hz, banda secondaria {} Hz, andata 07 {:02X}, ritorno 07 {:02X}", p.rx_hz, p.tx_band_hz, p.to_tx, p.to_rx),
                        Err(e) => tracing::warn!("satelliti: prova CI-V dell'IC-820H fallita: {e}; la trasmissione non si scrive, il Doppler segue solo la ricezione"),
                    }
                    probed = Some(result.clone());
                    transport = port.transport;
                    result?
                }
            };
            if inner.transmitting() {
                return Ok(None);
            }
            // trasmissione nella stessa banda della ricezione (un altro transponder): non si scrive, ma il ripiego resta acceso
            if crate::ic820h_civ::is_uhf(hz) != crate::ic820h_civ::is_uhf(probe.tx_band_hz) {
                wrong_band = true;
                return Err(format!("la trasmissione {hz} Hz non e' nella banda secondaria della radio ({} Hz): niente scritto", probe.tx_band_hz));
            }
            let out = crate::ic820h_civ::write_tx(&mut port, &mut log, &probe, hz).map(Some);
            transport = port.transport;
            out
        });
        inner.epoch.fetch_add(1, Ordering::SeqCst);
        if let Some(p) = probed {
            if p.is_err() {
                civ.failed_at = Some(Instant::now());
            }
            civ.probe = Some(p);
        }
        for line in &log {
            tracing::info!("satelliti: CI-V {line}");
        }
        civ.transport = transport;
        civ.log.extend(log);
        let excess = civ.log.len().saturating_sub(CIV_LOG_MAX);
        civ.log.drain(..excess);
        match out {
            Ok(Some(w)) => {
                civ.last_write = Some(Instant::now());
                civ.last = Some(w);
                tracing::info!("satelliti: trasmissione CI-V scritta {hz} Hz, riletta {} Hz; ricezione riletta {} Hz", w.tx_hz, w.rx_hz);
                Ok(true)
            }
            Ok(None) => Ok(false),
            Err(e) => {
                if matches!(civ.probe, Some(Ok(_))) && !wrong_band {
                    tracing::warn!("satelliti: trasmissione CI-V non confermata ({e}): ripiego spento fino alla riconnessione");
                    civ.disabled = Some(e.clone());
                    civ.failed_at = Some(Instant::now());
                }
                Err(RigError(e))
            }
        }
    }

    /// Lo stato della trasmissione con trame CI-V dirette dell'IC-820H; `None` per gli altri modelli.
    pub fn civ_uplink_report(&self) -> Option<CivUplinkReport> {
        if self.inner.config.model != IC_820H_MODEL {
            return None;
        }
        let civ = self.inner.civ.lock().ok()?;
        let (stato, dettaglio) = match (&civ.probe, &civ.disabled) {
            (_, Some(why)) => ("spenta", why.clone()),
            (None, None) => ("non provata", String::new()),
            (Some(Err(why)), None) => ("fallita", why.clone()),
            (Some(Ok(p)), None) => ("riuscita", format!("andata 07 {:02X}, ritorno 07 {:02X}", p.to_tx, p.to_rx)),
        };
        let probe = civ.probe.as_ref().and_then(|p| p.as_ref().ok());
        Some(CivUplinkReport {
            stato: stato.into(),
            dettaglio,
            ricezione_hz: probe.map(|p| p.rx_hz),
            banda_secondaria_hz: probe.map(|p| p.tx_band_hz),
            ultima_trasmissione_hz: civ.last.map(|w| w.tx_hz),
            trasporto: civ.transport.map(|t| t.name().to_string()).unwrap_or_default(),
            passi: civ.log.clone(),
        })
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
        inner.split_command(&format!("I {tx_hz}, S 1 {tx_vfo}"), "split DX da uno spot del cluster");
        inner.write_split_mode(&link);
        link.client().set_split_frequency(tx_hz).map_err(fr)?;
        link.client().set_split(true, tx_vfo).map_err(fr)?;
        inner.dx_split_tx.store(tx_hz, Ordering::SeqCst);
        inner.dx_split_rx.store(rx_hz, Ordering::SeqCst);
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
        inner.split_command(&format!("S 0 {rx_vfo}"), "split DX spento");
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
        crate::trace::note("chiusura del collegamento con la radio (spegnimento di Aurora o scollegamento)", false);
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
                self.inner.split_command("S 0 VFOA", "chiusura");
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
        inner.set_cat_mode(&link);
        // split vero: al cambio di frequenza (banda) la radio esce dallo split. Il VFO di trasmissione si rimette sulla banda nuova solo un
        // attimo prima del PTT (`set_tx_audio_frequency`, chiamato da Aurora prima di ogni trasmissione): cosi' la radio non resta mai in split
        // con il VFO B sulla banda vecchia o su una frequenza che il proprietario non si aspetta (segnalazione del 2026-11-11)
        // lo Split DX di uno spot del cluster finisce al cambio di frequenza (spot senza «UP», cambio di banda), anche senza lo split di FT8
        let dx_split = inner.dx_split.swap(false, Ordering::SeqCst);
        if inner.config.split == SplitMode::Rig || dx_split {
            let rx_vfo = if inner.vfo_reversed.load(Ordering::SeqCst) { "VFOB" } else { "VFOA" };
            match inner.try_split_off(&link, rx_vfo, "cambio di frequenza") {
                Ok(true) => {
                    inner.xit_split.store(false, Ordering::SeqCst);
                    inner.update(|s| s.split = Some(false));
                }
                Ok(false) => {}
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
            freq_fast: Duration::ZERO,
            freq_idle: Duration::ZERO,
            freq_active_for: Duration::ZERO,
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

    /// Lettura completa lenta come su una seriale (2 s) e lettura veloce della sola frequenza con i tempi veri.
    fn tracking() -> Timing {
        Timing { poll: Duration::from_secs(2), freq_fast: FREQ_FAST, freq_idle: FREQ_IDLE, freq_active_for: FREQ_ACTIVE_FOR, ..fast() }
    }

    fn freq_reads(fake: &FakeRigctld) -> usize {
        fake.commands().iter().filter(|c| *c == "f" || c.starts_with("f ")).count()
    }

    #[test]
    fn it_follows_the_dial_turned_by_hand_within_half_a_second() {
        let fake = FakeRigctld::start(14_080_000);
        let rig = start_with(&fake, RigConfig::default(), tracking());
        std::thread::sleep(Duration::from_millis(100));
        for step in 1..=5u64 {
            let hz = 14_080_000 + step * 50;
            fake.set_frequency(hz);
            let t = Instant::now();
            wait_until("frequenza nuova", || rig.status().frequency_hz == Some(hz));
            assert!(t.elapsed() < Duration::from_millis(500), "passo {step}: vista dopo {:?}", t.elapsed());
        }
    }

    #[test]
    fn at_rest_the_frequency_reads_stay_few() {
        let fake = FakeRigctld::start(14_080_000);
        let _rig = start_with(&fake, RigConfig::default(), tracking());
        std::thread::sleep(Duration::from_millis(500));
        fake.clear_commands();
        std::thread::sleep(Duration::from_secs(2));
        let n = freq_reads(&fake);
        // a riposo: una ogni 200 ms (10 in 2 s) piu' la lettura completa
        assert!((2..=13).contains(&n), "letture della frequenza a riposo in 2 s: {n}");
    }

    #[test]
    fn it_does_not_read_the_frequency_while_transmitting() {
        let fake = FakeRigctld::start(14_080_000);
        let rig = start_with(&fake, RigConfig::default(), Timing { startup_block: Duration::from_millis(10), ..tracking() });
        std::thread::sleep(Duration::from_millis(100));
        rig.set_ptt(true).expect("PTT");
        std::thread::sleep(Duration::from_millis(100));
        fake.clear_commands();
        std::thread::sleep(Duration::from_millis(1000));
        assert_eq!(freq_reads(&fake), 0, "{:?}", fake.commands());
        rig.set_ptt(false).expect("PTT spento");
    }

    #[test]
    fn it_turns_the_ptt_off_first_and_reads_the_radio() {
        let fake = FakeRigctld::start(14_074_000);
        let rig = start(&fake, RigConfig::default());
        // la prima domanda e' solo `\chk_vfo` (modalita' VFO del demone); il primo comando alla radio e' lo spegnimento del PTT (la lettura `t` che lo precede dice solo se era rimasto acceso)
        let commands: Vec<String> = fake.commands().into_iter().filter(|c| c != "\\chk_vfo" && c != "t").collect();
        assert_eq!(commands.first().map(String::as_str), Some("T 0"), "il PTT si spegne subito dopo l'apertura: {commands:?}");
        let s = rig.status();
        assert_eq!((s.frequency_hz, s.mode.as_deref(), s.ptt), (Some(14_074_000), Some("PKTUSB"), false));
        assert_eq!(rig.frequency().expect("frequenza"), 14_074_000);
    }

    /// TS-850 (pacchetto 2948CDCC): dopo il nostro `T 0` la radio torna in trasmissione da sola e nessuno la spegneva piu'.
    #[test]
    fn a_radio_that_goes_back_to_transmit_after_our_ptt_off_gets_another_t0() {
        let fake = FakeRigctld::start(14_074_000);
        let rig = start(&fake, RigConfig::default());
        std::thread::sleep(Duration::from_millis(300));
        rig.set_ptt(true).expect("acceso");
        rig.set_ptt(false).expect("spento");
        fake.set_ptt(true); // la radio torna in trasmissione da sola
        wait_until("la radio viene spenta di nuovo", || !fake.ptt());
        wait_until("lo stato segue la radio", || !rig.status().ptt);
    }

    /// Un PTT acceso a mano, senza una nostra trasmissione appena finita, non si tocca.
    #[test]
    fn a_ptt_that_nobody_here_turned_on_is_left_alone() {
        let fake = FakeRigctld::start(14_074_000);
        let rig = start(&fake, RigConfig::default());
        std::thread::sleep(Duration::from_millis(300));
        fake.set_ptt(true); // l'operatore preme il PTT sul microfono: Aurora non ha trasmesso
        std::thread::sleep(Duration::from_millis(900));
        assert!(fake.ptt(), "nessuno lo spegne");
        drop(rig);
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
    fn a_ptt_left_on_is_turned_off_at_the_start() {
        let fake = FakeRigctld::start(14_074_000);
        fake.set_ptt(true);
        let rig = start(&fake, RigConfig::default());
        assert!(!fake.ptt(), "il PTT trovato acceso all'avvio si spegne");
        assert!(!rig.status().ptt);
    }

    #[test]
    fn force_ptt_off_sends_one_more_t0_and_clears_the_state() {
        let fake = FakeRigctld::start(14_074_000);
        let rig = start(&fake, RigConfig::default());
        std::thread::sleep(Duration::from_millis(300));
        rig.set_ptt(true).expect("PTT");
        fake.fail_ptt_off(true);
        fake.clear_commands();
        assert!(rig.set_ptt(false).is_err());
        let sent = fake.commands().iter().filter(|c| *c == "T 0").count();
        assert_eq!(sent, 1, "set_ptt(false) e' un solo tentativo: i successivi sono di chi spegne");
        assert!(rig.status().ptt);
        fake.fail_ptt_off(false);
        rig.force_ptt_off().expect("T 0");
        assert!(!fake.ptt() && !rig.status().ptt);
    }

    #[test]
    fn force_ptt_off_reopens_the_cat_and_the_reopening_turns_the_ptt_off() {
        let fake = FakeRigctld::start(14_074_000);
        let rig = start(&fake, RigConfig::default());
        std::thread::sleep(Duration::from_millis(300));
        rig.set_ptt(true).expect("PTT");
        fake.fail_ptt_off(true);
        assert!(rig.force_ptt_off().is_err(), "T 0 non passa");
        // la radio torna a rispondere mentre il collegamento si riapre: all'apertura il primo comando e' `T 0`
        fake.fail_ptt_off(false);
        wait_until("PTT spento dalla riapertura", || !fake.ptt() && !rig.status().ptt);
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
        let split_commands = |fake: &FakeRigctld| fake.commands().into_iter().filter(|c| c.starts_with("I ") || c.starts_with("S ") || c.starts_with("X ")).collect::<Vec<_>>();
        assert_eq!(split_commands(&fake), ["X PKTUSB 2400", "I 14074500", "S 1 VFOB"]);
        assert!(fake.split() && rig.status().split == Some(true));
        // al cambio di banda la radio esce dallo split (niente VFO B lasciato sulla banda vecchia)...
        rig.set_frequency(7_074_000).expect("QSY");
        assert!(!fake.split() && rig.status().split == Some(false), "{:?}", fake.commands());
        // ...e prima della trasmissione il VFO di trasmissione va sulla banda nuova e lo split si riaccende
        fake.clear_commands();
        rig.set_tx_audio_frequency(2200).expect("split prima del PTT");
        assert_eq!(split_commands(&fake), ["X PKTUSB 2400", "I 7074500", "S 1 VFOB"]);
        assert!(fake.split());
        // alla chiusura lo split si toglie
        rig.shutdown();
        assert!(!fake.split());
    }

    #[test]
    fn the_tx_vfo_gets_the_configured_mode_before_every_split_even_after_a_qsy_in_the_same_band() {
        let fake = FakeRigctld::start(21_074_000);
        let rig = start(&fake, RigConfig { split: SplitMode::Rig, cat_mode: CatMode::Usb, ..RigConfig::default() });
        let seq = |fake: &FakeRigctld| fake.commands().into_iter().filter(|c| ["X ", "I ", "S ", "G "].iter().any(|p| c.starts_with(p))).collect::<Vec<_>>();
        for dial in [21_074_000, 21_076_000] {
            rig.set_frequency(dial).expect("QSY");
            fake.clear_commands();
            rig.set_tx_audio_frequency(1000).expect("split");
            assert_eq!(seq(&fake), ["X USB 0".to_string(), format!("I {}", split_tx_dial_hz(dial, 1000)), "S 1 VFOB".to_string()]);
            assert!(!seq(&fake).iter().any(|c| c.contains("LSB")));
        }
    }

    fn split_mode_commands(fake: &FakeRigctld) -> Vec<String> {
        fake.commands().into_iter().filter(|c| c.starts_with("X ") || c == "G CPY").collect()
    }

    #[test]
    fn a_radio_without_x_and_without_the_a_equals_b_copy_is_asked_only_once() {
        let fake = FakeRigctld::start(14_074_000);
        fake.fail_split_mode_with(-1);
        let rig = start(&fake, RigConfig { split: SplitMode::Rig, ..RigConfig::default() });
        fake.clear_commands();
        rig.set_tx_audio_frequency(2200).expect("split anche senza modo di trasmissione");
        assert_eq!(split_mode_commands(&fake), ["X PKTUSB 2400", "G CPY"]);
        assert!(rig.inner.split_mode_unsupported.load(Ordering::SeqCst));
        // al cambio banda successivo ne' `X` ne' la copia si ritentano; lo split funziona lo stesso
        rig.set_frequency(7_074_000).expect("QSY");
        fake.clear_commands();
        rig.set_tx_audio_frequency(2200).expect("split prima del PTT");
        assert!(split_mode_commands(&fake).is_empty(), "{:?}", fake.commands());
        assert!(fake.split());
    }

    #[test]
    fn a_radio_that_copies_a_to_b_keeps_the_same_commands_and_the_same_order() {
        let fake = FakeRigctld::start(14_074_000);
        fake.fail_split_mode_with(-4);
        fake.accept_vfo_copy(true);
        let rig = start(&fake, RigConfig { split: SplitMode::Rig, ..RigConfig::default() });
        for dial in [14_074_000, 7_074_000] {
            rig.set_frequency(dial).expect("QSY");
            fake.clear_commands();
            rig.set_tx_audio_frequency(2200).expect("split");
            let seq: Vec<String> = fake.commands().into_iter().filter(|c| c.starts_with("X ") || c == "G CPY" || c.starts_with("I ") || c.starts_with("S 1")).collect();
            assert_eq!(seq, ["X PKTUSB 2400".to_string(), "G CPY".to_string(), format!("I {}", split_tx_dial_hz(dial, 2200)), "S 1 VFOB".to_string()]);
        }
        assert!(!rig.inner.split_mode_unsupported.load(Ordering::SeqCst));
    }

    #[test]
    fn a_real_link_error_on_the_tx_mode_is_not_remembered_as_unsupported() {
        // -6: errore di ingresso/uscita della porta seriale: resta un avviso e al prossimo cambio si riprova
        let fake = FakeRigctld::start(14_074_000);
        fake.fail_split_mode_with(-6);
        let rig = start(&fake, RigConfig { split: SplitMode::Rig, ..RigConfig::default() });
        fake.clear_commands();
        rig.set_tx_audio_frequency(2200).expect("split");
        assert!(!rig.inner.split_mode_unsupported.load(Ordering::SeqCst));
        rig.set_frequency(7_074_000).expect("QSY");
        fake.clear_commands();
        rig.set_tx_audio_frequency(2200).expect("split prima del PTT");
        assert_eq!(split_mode_commands(&fake), ["X PKTUSB 2400", "G CPY"]);
    }

    fn split_commands(fake: &FakeRigctld) -> Vec<String> {
        fake.commands().into_iter().filter(|c| c.starts_with("F ") || c.starts_with("I ") || c.starts_with("S ")).collect()
    }

    /// I comandi di rigctld che leggono o cambiano il VFO o lo split, e che su alcune radio commutano davvero il VFO.
    fn vfo_touching(c: &str) -> bool {
        let head = c.split_whitespace().next().unwrap_or("");
        matches!(head, "V" | "v" | "s" | "S" | "i" | "I" | "x" | "X" | r"\get_vfo_info" | r"\set_vfo" | r"\get_split_vfo")
            || c.contains("VFOB")
    }

    #[test]
    fn polling_without_extras_never_touches_the_vfo_or_the_split() {
        // #67: con «Leggi anche gli altri dati» spento la lettura periodica chiede solo frequenza, modo e PTT
        let fake = FakeRigctld::start(14_074_000);
        let rig = start_with(&fake, RigConfig::default(), Timing { poll: Duration::from_millis(1), ..fast() });
        wait_until("letture", || fake.commands().iter().filter(|c| c.as_str() == "f").count() >= 30);
        let bad: Vec<String> = fake.commands().into_iter().filter(|c| vfo_touching(c)).collect();
        assert!(bad.is_empty(), "comandi del VFO nella lettura periodica: {bad:?}");
        assert_eq!(fake.vfo(), "VFOA");
        assert_eq!(rig.status().split, None);
    }

    #[test]
    fn polling_with_extras_never_switches_the_vfo() {
        // con i dati in piu' si leggono VFO B e split, ma senza mai cambiare il VFO della radio
        let fake = FakeRigctld::start(14_074_000);
        let config = RigConfig { read_extras: true, ..RigConfig::default() };
        let _rig = start_with(&fake, config, Timing { poll: Duration::from_millis(1), ..fast() });
        wait_until("letture", || fake.commands().iter().filter(|c| c.as_str() == "f").count() >= 30);
        let bad: Vec<String> =
            fake.commands().into_iter().filter(|c| c.starts_with("V ") || c.starts_with(r"\set_vfo") || c.starts_with("S ")).collect();
        assert!(bad.is_empty(), "cambi di VFO o split nella lettura periodica: {bad:?}");
        assert_eq!(fake.vfo(), "VFOA");
    }

    #[test]
    fn a_split_switched_on_outside_aurora_is_detected_and_flagged() {
        let fake = FakeRigctld::start(28_074_000);
        let rig = start(&fake, RigConfig { read_extras: true, ..RigConfig::default() });
        wait_until("split letto spento", || rig.status().split == Some(false));
        assert_eq!(rig.status().split_external_ms, 0);
        // la radio (o un programma sulla porta condivisa) accende lo split: Aurora non ha mandato niente
        fake.set_split(true);
        wait_until("split acceso da fuori segnalato", || {
            let s = rig.status();
            s.split == Some(true) && s.split_external_ms > 0
        });
        // spento e riacceso da fuori: un avviso nuovo
        let first = rig.status().split_external_ms;
        fake.set_split(false);
        wait_until("split letto spento", || rig.status().split == Some(false));
        std::thread::sleep(Duration::from_millis(5));
        fake.set_split(true);
        wait_until("secondo avviso", || rig.status().split_external_ms > first);
    }

    #[test]
    fn split_off_is_immediate_and_a_poll_already_running_does_not_switch_it_back_on() {
        // radio su seriale lenta (ogni risposta 40 ms) con lettura continua: «Spegni split» arriva in mezzo a una lettura completa. Prima
        // il comando aspettava le domande della lettura (frequenza, modo, PTT, split: fino a 4 x 40 ms qui, secondi su una radio vera)
        let fake = FakeRigctld::start(28_074_000);
        fake.set_delay_ms(40);
        let rig = start_with(&fake, RigConfig { read_extras: true, ..RigConfig::default() }, Timing { poll: Duration::from_millis(1), ..fast() });
        for round in 0..12 {
            fake.set_split(true);
            wait_until("split letto acceso", || rig.status().split == Some(true));
            std::thread::sleep(Duration::from_millis(3 * round));
            let t0 = Instant::now();
            rig.force_split_off().expect("spegni split");
            let took = t0.elapsed();
            assert!(!fake.split(), "la radio deve essere in simplex appena il comando torna");
            assert_eq!(rig.status().split, Some(false));
            // il comando aspetta al massimo la domanda gia' in corso (40 ms) piu' la sua (40 ms), non una lettura completa
            assert!(took < Duration::from_millis(120), "spegnimento lento: {took:?}");
            // le letture che seguono (anche quella partita prima del comando) non lo riaccendono sul display
            let checks = rig.inner.split_checks.load(Ordering::SeqCst);
            let until = Instant::now() + Duration::from_secs(5);
            while rig.inner.split_checks.load(Ordering::SeqCst) < checks + 2 * SPLIT_CHECK_EVERY && Instant::now() < until {
                assert_eq!(rig.status().split, Some(false), "giro {round}: split tornato acceso da una lettura vecchia");
                std::thread::sleep(Duration::from_millis(1));
            }
        }
    }

    #[test]
    fn a_split_switched_on_by_aurora_is_not_flagged() {
        let fake = FakeRigctld::start(14_074_000);
        let rig = start(&fake, RigConfig { read_extras: true, ..RigConfig::default() });
        wait_until("split letto spento", || rig.status().split == Some(false));
        rig.set_dx_split(14_025_000, 14_030_000).expect("split DX");
        // diverse letture dello split dopo il comando di Aurora
        let checks = rig.inner.split_checks.load(Ordering::SeqCst);
        wait_until("altre letture", || rig.inner.split_checks.load(Ordering::SeqCst) >= checks + 2 * SPLIT_CHECK_EVERY);
        let s = rig.status();
        assert_eq!((s.split, s.split_external_ms), (Some(true), 0));
    }

    #[test]
    fn a_radio_without_data_mode_is_not_taken_out_of_usb_when_it_is_already_in_usb() {
        // Icom senza modo dati in Hamlib (IC-756PRO): `M PKTUSB` e' rifiutato; la radio puo' essere su USB-D, che Hamlib legge «USB»
        let fake = FakeRigctld::start(14_074_000);
        fake.refuse_mode("PKTUSB");
        fake.set_mode_by_hand("USB");
        fake.clear_commands();
        let rig = start(&fake, RigConfig { cat_mode: CatMode::Data, ..RigConfig::default() });
        wait_until("collegata", || rig.status().state == LinkState::Connected);
        let writes: Vec<String> = fake.commands().into_iter().filter(|c| c.starts_with("M ")).collect();
        assert_eq!(writes, ["M PKTUSB 0"], "dopo il rifiuto non si scrive USB: {writes:?}");
        assert_eq!(fake.mode(), "USB");
    }

    #[test]
    fn a_radio_without_data_mode_that_is_not_in_usb_is_still_put_in_usb() {
        // la TS-480 restava in LSB sulle bande basse: li' il ripiego su USB serve ancora
        let fake = FakeRigctld::start(7_074_000);
        fake.refuse_mode("PKTUSB");
        fake.set_mode_by_hand("LSB");
        fake.clear_commands();
        let rig = start(&fake, RigConfig { cat_mode: CatMode::Data, ..RigConfig::default() });
        wait_until("collegata", || rig.status().state == LinkState::Connected);
        let writes: Vec<String> = fake.commands().into_iter().filter(|c| c.starts_with("M ")).collect();
        assert_eq!(writes, ["M PKTUSB 0", "M USB 0"], "{writes:?}");
        assert_eq!(fake.mode(), "USB");
    }

    #[test]
    fn dx_split_tunes_the_rx_then_the_tx_vfo_then_switches_split_on() {
        let fake = FakeRigctld::start(14_074_000);
        // con lo split di FT8 («Rig») acceso: il cambio di ricezione lo spegne, poi lo Split DX mette il suo VFO di trasmissione
        let rig = start(&fake, RigConfig { split: SplitMode::Rig, ..RigConfig::default() });
        fake.clear_commands();
        rig.set_dx_split(14_025_000, 14_030_000).expect("split DX");
        assert_eq!(split_commands(&fake), ["F 14025000", "S 0 VFOA", "I 14030000", "S 1 VFOB"]);
        assert!(fake.commands().iter().any(|c| c.starts_with("X ")), "{:?}", fake.commands());
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
    fn with_ptt_data_the_ptt_is_asked_as_data() {
        let fake = FakeRigctld::start(14_074_000);
        let rig = start_with(&fake, RigConfig { ptt_data: true, ..RigConfig::default() }, Timing { startup_block: Duration::from_millis(10), ..tracking() });
        std::thread::sleep(Duration::from_millis(100));
        fake.clear_commands();
        rig.set_ptt(true).expect("PTT");
        assert!(fake.commands().iter().any(|c| c == "T 3"), "PTT dati: {:?}", fake.commands());
        assert!(!fake.commands().iter().any(|c| c == "T 1"), "{:?}", fake.commands());
        rig.set_ptt(false).expect("PTT spento");
        assert_eq!(fake.commands().iter().rev().find(|c| c.starts_with('T')).map(String::as_str), Some("T 0"), "lo spegnimento resta T 0");
    }

    #[test]
    fn by_default_the_ptt_is_the_plain_one() {
        let fake = FakeRigctld::start(14_074_000);
        let rig = start_with(&fake, RigConfig::default(), Timing { startup_block: Duration::from_millis(10), ..tracking() });
        std::thread::sleep(Duration::from_millis(100));
        fake.clear_commands();
        rig.set_ptt(true).expect("PTT");
        assert!(fake.commands().iter().any(|c| c == "T 1"), "{:?}", fake.commands());
        rig.set_ptt(false).expect("PTT spento");
    }

    #[test]
    fn with_read_tx_alc_the_alc_is_read_in_transmit_too() {
        let fake = FakeRigctld::start(14_074_000);
        fake.set_level("RFPOWER_METER_WATTS", "20.5");
        fake.set_level("SWR", "1.5");
        fake.set_level("ALC", "0.3");
        let rig = tx_rig(&fake, RigConfig { read_tx_alc: true, ..RigConfig::default() });
        let mut last = Meters::default();
        for _ in 0..3 {
            last = rig.meters().expect("misuratori");
            std::thread::sleep(Duration::from_millis(210));
        }
        assert_eq!((last.power_w, last.swr, last.alc), (Some(20.5), Some(1.5), Some(0.3)));
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
        // in trasmissione solo potenza e ROS: ALC, compressione, corrente e tensione non si chiedono (cambierebbero il selettore del misuratore)
        assert_eq!((last.power_w, last.swr, last.alc, last.comp_db, last.vdd_v), (Some(20.5), Some(1.5), None, None, None));
        assert_eq!((last.id_a, last.s_points), (None, None));
        let asked = level_commands(&fake);
        assert!(!asked.iter().any(|c| c == "l STRENGTH"), "in trasmissione niente STRENGTH: {asked:?}");
        for unwanted in ["l ALC", "l COMP_METER", "l ID_METER", "l VD_METER"] {
            assert_eq!(fake.count(unwanted), 0, "{unwanted} non si chiede piu' in trasmissione: {asked:?}");
        }
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

    /// I comandi che cambiano frequenza, modo, VFO o split (senza le letture periodiche).
    fn writes(fake: &FakeRigctld) -> Vec<String> {
        fake.commands().into_iter().filter(|c| ["V ", "F ", "M ", "S ", "X ", "I "].iter().any(|h| c.starts_with(h))).collect()
    }

    #[test]
    fn a_refused_split_off_is_not_sent_again() {
        // IC-820H: `S 0 VFOA` rifiutato con -11 a ogni cambio di frequenza (457 avvisi in un'ora)
        let fake = FakeRigctld::start(14_074_000);
        fake.fail_split_with(-11);
        let rig = start(&fake, RigConfig { split: SplitMode::Rig, ..RigConfig::default() });
        for hz in [7_074_000, 7_075_000, 7_076_000] {
            rig.set_frequency(hz).expect("QSY");
        }
        let s_cmds = writes(&fake).into_iter().filter(|c| c.starts_with("S ")).count();
        assert_eq!(s_cmds, 1, "{:?}", writes(&fake));
    }

    #[test]
    fn tracking_frequency_sends_only_the_frequency() {
        let fake = FakeRigctld::start(435_600_000);
        let rig = start(&fake, RigConfig { split: SplitMode::Rig, ..RigConfig::default() });
        fake.clear_commands();
        for hz in [435_626_135, 435_626_092, 435_626_049] {
            rig.set_tracking_frequency(hz).expect("Doppler");
        }
        assert_eq!(writes(&fake), ["F 435626135", "F 435626092", "F 435626049"]);
    }

    /// Segnalazione #83 (IC-820H, ISS: ricezione 437 MHz, trasmissione 145 MHz): la trasmissione va sul VFO della sua banda.
    #[test]
    fn the_uplink_goes_to_the_vfo_of_its_own_band() {
        // Main = 144, Sub = 430: trasmissione VHF su Main, ricezione UHF su Sub
        assert_eq!(uplink_vfos(band_map_of(137_790_000, 445_990_000), 145_990_000), ("Main", "Sub"));
        // trasmissione UHF con le stesse bande: va su Sub
        assert_eq!(uplink_vfos(band_map_of(137_790_000, 445_990_000), 435_000_000), ("Sub", "Main"));
        // Main = 430, Sub = 144 (bande scambiate dalla radio): il contrario
        assert_eq!(uplink_vfos(band_map_of(445_990_000, 137_790_000), 145_990_000), ("Sub", "Main"));
        assert_eq!(uplink_vfos(band_map_of(445_990_000, 137_790_000), 435_000_000), ("Main", "Sub"));
        // stessa banda o mappa sconosciuta: la coppia di sempre
        assert_eq!(uplink_vfos(band_map_of(435_600_000, 435_600_000), 145_990_000), ("Sub", "Main"));
        assert_eq!(uplink_vfos(MAIN_SUB_UNKNOWN, 145_990_000), ("Sub", "Main"));
    }

    #[test]
    fn two_vfo_uplink_writes_the_second_vfo_and_goes_back() {
        let fake = FakeRigctld::start(435_600_000);
        let rig = start(&fake, RigConfig::default());
        fake.clear_commands();
        assert_eq!(rig.set_uplink_via_vfo(145_900_000, Some("LSB")).expect("uplink"), true);
        // la prima volta si leggono le bande dei due VFO (V Main, V Sub) per scegliere dove scrivere
        assert_eq!(writes(&fake), ["V Main", "V Sub", "V Sub", "F 145900000", "M LSB 0", "V Main"]);
        // senza modo nuovo: niente M
        fake.clear_commands();
        rig.set_uplink_via_vfo(145_900_300, None).expect("uplink");
        assert_eq!(writes(&fake), ["V Sub", "F 145900300", "V Main"]);
    }

    /// Radio a due bande come l'IC-820H (pacchetti di Vladimir, #83): ricezione UHF 437,79 e trasmissione VHF 145,99 con Main = 144 e Sub = 430. Con la
    /// coppia fissa ogni frequenza finiva nella banda sbagliata e la radio la ripiegava di 300 MHz (437,79 -> 137,79 su Main); ora, dopo il primo giro,
    /// Main tiene la trasmissione, Sub tiene la ricezione e le scritture della ricezione successive vanno su Sub.
    #[test]
    fn a_dual_band_radio_ends_up_with_rx_on_its_uhf_vfo_and_tx_on_its_vhf_vfo() {
        let fake = FakeRigctld::start(435_600_000);
        fake.enable_dual_bands(145_000_000, 435_000_000);
        let rig = start(&fake, RigConfig { model: 3032, ..RigConfig::default() });
        // la ricezione scritta sul VFO in uso (Main, banda 144): la radio la ripiega, come nel pacchetto
        let _ = rig.set_frequency(437_790_000);
        assert_eq!(fake.band_frequencies()[0], 137_790_000, "la radio ripiega 437,79 -> 137,79 su Main");
        assert_eq!(rig.set_uplink_via_vfo(145_990_000, None).expect("uplink"), true);
        assert_eq!(fake.band_frequencies(), [145_990_000, 435_000_000], "la trasmissione sta su Main (144), Sub (430) e' intatto");
        // dopo il giro il VFO in uso e' Sub: la ricezione successiva (Doppler) finisce nella banda giusta
        rig.set_frequency(437_790_000).expect("ricezione");
        assert_eq!(fake.band_frequencies(), [145_990_000, 437_790_000], "ricezione 437,79 su Sub, trasmissione 145,99 su Main");
        // il giro dopo (10 s) aggiorna solo la trasmissione e rimette il VFO di ricezione
        rig.set_uplink_via_vfo(145_990_300, None).expect("uplink");
        rig.set_frequency(437_790_100).expect("ricezione");
        assert_eq!(fake.band_frequencies(), [145_990_300, 437_790_100]);
    }

    /// IC-820H (modello 3032), pacchetto 3EFC9CF8 (#83): Main e Sub leggono tutte e due la banda alta (Main = 435,63 = ricezione). Prima si
    /// supponeva la disposizione di fabbrica (Main = VHF) e la trasmissione 145,96 veniva scritta sopra la ricezione e data per riuscita
    /// (riletta 445,96). Ora vale la banda letta su Main (UHF, quindi la trasmissione VHF andrebbe su Sub) e prima di scrivere si rilegge il
    /// VFO di trasmissione: legge la banda alta, quindi niente scritto.
    #[test]
    fn the_ic_820h_never_writes_the_uplink_over_the_receive_band_when_both_vfos_read_uhf() {
        let fake = FakeRigctld::start(435_633_438);
        let rig = start(&fake, RigConfig { model: 3032, ..RigConfig::default() });
        fake.clear_commands();
        let err = rig.set_uplink_via_vfo(145_962_174, None).expect_err("nessuna banda VHF sulla radio");
        assert!(err.0.contains("niente scritto"), "{}", err.0);
        assert!(!writes(&fake).iter().any(|c| c.starts_with("F ")), "{:?}", writes(&fake));
        assert_eq!(fake.frequency(), 435_633_438, "la ricezione e' intatta");
        assert_eq!(rig.inner.sat_band_map.load(Ordering::SeqCst), MAIN_UHF_SUB_VHF, "etichette dalle frequenze lette: Main = UHF");
    }

    /// La trasmissione ripiegata in un'altra banda (scritta 145,96, riletta 445,96: e' finita sul VFO della banda alta) non e' riuscita.
    #[test]
    fn an_uplink_read_back_in_another_band_is_an_error() {
        let fake = FakeRigctld::start(435_600_000);
        fake.enable_dual_bands(145_000_000, 435_000_000);
        let rig = start(&fake, RigConfig::default());
        // mappa sbagliata: Main creduto UHF, la trasmissione VHF va su Sub (banda alta) e la radio la ripiega
        rig.inner.sat_band_map.store(MAIN_UHF_SUB_VHF, Ordering::SeqCst);
        let err = rig.set_uplink_via_vfo(145_962_174, None).expect_err("riletta 445,96");
        assert!(err.0.contains("non e' arrivata"), "{}", err.0);
        assert_eq!(fake.band_frequencies()[1], 445_962_174);
        assert!(uplink_readback_ok(145_962_174, 145_962_180));
        assert!(!uplink_readback_ok(145_962_174, 445_962_174));
        assert!(!uplink_readback_ok(145_962_174, 147_000_000));
    }

    #[test]
    fn two_vfo_uplink_falls_back_to_vfob_and_remembers_it() {
        let fake = FakeRigctld::start(435_600_000);
        fake.refuse_vfo("Sub");
        let rig = start(&fake, RigConfig::default());
        fake.clear_commands();
        rig.set_uplink_via_vfo(145_900_000, None).expect("uplink");
        assert_eq!(writes(&fake), ["V Main", "V Sub", "V Sub", "V VFOB", "F 145900000", "V VFOA"]);
        fake.clear_commands();
        rig.set_uplink_via_vfo(145_900_100, None).expect("uplink");
        assert_eq!(writes(&fake), ["V VFOB", "F 145900100", "V VFOA"]);
    }

    /// IC-820H con il Sub spento (pacchetto di Vladimir del 2026-11-21, #83): `V Sub` risponde `RPRT -9` (comando rifiutato dalla radio) e sul
    /// «Main» c'e' la banda di ricezione (435,63). Prima, con la lettura delle bande fallita, valeva la disposizione di fabbrica (Main = 144) e
    /// la trasmissione 145,96 veniva scritta sul Main, cioe' sopra la ricezione: la radio restava su 145,96 e Aurora perdeva il segnale.
    #[test]
    fn with_the_sub_off_the_uplink_is_not_written_over_the_receive_frequency() {
        let fake = FakeRigctld::start(435_633_000);
        fake.reject_vfo("Sub");
        let rig = start(&fake, RigConfig { model: 3032, ..RigConfig::default() });
        fake.clear_commands();
        let err = rig.set_uplink_via_vfo(145_962_153, Some("LSB")).expect_err("senza il Sub non si trasmette sul secondo VFO");
        assert!(err.0.contains("Sub"), "{}", err.0);
        assert!(!writes(&fake).iter().any(|c| c.starts_with("F ") || c.starts_with("M ")), "{:?}", writes(&fake));
        assert_eq!(fake.frequency(), 435_633_000, "la frequenza di ricezione e' intatta");
        // il rifiuto non si ripete a ogni giro di 10 s sulla radio: la seconda volta non si tocca niente
        fake.clear_commands();
        assert!(rig.set_uplink_via_vfo(145_962_200, None).is_err());
        assert!(!writes(&fake).iter().any(|c| c.starts_with("F ")), "{:?}", writes(&fake));
    }

    #[test]
    fn two_vfo_uplink_fails_cleanly_when_no_vfo_is_accepted() {
        let fake = FakeRigctld::start(435_600_000);
        fake.refuse_vfo("Sub");
        fake.refuse_vfo("VFOB");
        let rig = start(&fake, RigConfig::default());
        fake.clear_commands();
        assert!(rig.set_uplink_via_vfo(145_900_000, Some("LSB")).is_err());
        assert!(writes(&fake).iter().all(|c| c.starts_with("V ")), "{:?}", writes(&fake));
    }

    #[test]
    fn two_vfo_uplink_never_switches_while_transmitting() {
        let fake = FakeRigctld::start(435_600_000);
        let rig = start(&fake, RigConfig::default());
        rig.inner.external_tx.store(true, Ordering::SeqCst);
        fake.clear_commands();
        assert_eq!(rig.set_uplink_via_vfo(145_900_000, None).expect("in trasmissione"), false);
        assert!(writes(&fake).is_empty());
    }

    fn ic820h(fake: &FakeRigctld, rx_hz: u64, sub_hz: u64, civ: crate::fake_rigctld::CivBehaviour) -> HamlibRig {
        fake.enable_dual_bands(rx_hz, sub_hz);
        fake.set_civ(civ);
        let rig = start(&fake, RigConfig { model: 3032, ..RigConfig::default() });
        fake.clear_commands();
        rig
    }

    #[test]
    fn ic_820h_civ_probe_then_writes_the_uplink_on_the_sub_band_and_comes_back() {
        let fake = FakeRigctld::start(145_950_000);
        let rig = ic820h(&fake, 145_950_000, 435_000_000, Default::default());
        assert!(rig.set_uplink_via_civ(435_123_456).expect("scrittura CI-V"));
        assert_eq!(fake.band_frequencies(), [145_950_000, 435_123_456]);
        assert_eq!(fake.current_band(), 0, "la radio torna sulla banda di ricezione");
        assert_eq!(fake.civ_writes(), 1);
        let report = rig.civ_uplink_report().expect("rapporto");
        assert_eq!(report.stato, "riuscita");
        assert_eq!(report.ricezione_hz, Some(145_950_000));
        assert_eq!(report.banda_secondaria_hz, Some(435_000_000));
        assert_eq!(report.ultima_trasmissione_hz, Some(435_123_456));
        assert_eq!(report.trasporto, "W eco+risposta");
        assert!(report.passi.iter().any(|p| p.contains("inviati FE FE 42 E0 07 D1 FD | ricevuti FE FE 42 E0 07 D1 FD FE FE E0 42 FB FD")), "{:?}", report.passi);
        // subito dopo: troppo presto, niente scambio di banda
        fake.clear_commands();
        assert!(!rig.set_uplink_via_civ(435_123_900).expect("troppo presto"));
        assert!(fake.commands().iter().all(|c| !c.starts_with("+W")), "{:?}", fake.commands());
    }

    /// La radio vera (pacchetto 3EFC9CF8): `+w` con la trama in maiuscolo non ha terminatore, Hamlib legge fino al timeout e non restituisce
    /// niente; con `xfd` minuscolo si ferma all'eco. Il trasporto vecchio non vede mai la risposta della radio.
    #[test]
    fn ic_820h_civ_the_plain_w_command_never_returns_the_radio_answer() {
        let fake = FakeRigctld::start(145_950_000);
        fake.enable_dual_bands(145_950_000, 435_000_000);
        let c = fake.client(Duration::from_millis(500)).expect("client");
        let (lines, _) = c.send_raw_reply(&crate::ic820h_civ::frame_text(&[0x03])).expect("risposta");
        assert_eq!(crate::ic820h_civ::classify(&crate::ic820h_civ::parse_bytes(&lines)), crate::ic820h_civ::Answer::Nothing, "{lines:?}");
        let (lines, _) = c.send_raw_reply(&crate::ic820h_civ::frame_text(&[0x03]).to_lowercase()).expect("risposta");
        assert_eq!(crate::ic820h_civ::classify(&crate::ic820h_civ::parse_bytes(&lines)), crate::ic820h_civ::Answer::Nothing, "solo l'eco: {lines:?}");
        // con il numero di byte (eco + risposta) la frequenza arriva
        let (lines, _) = c.send_raw_rx_reply(&crate::ic820h_civ::frame_text(&[0x03]), 17, Duration::from_secs(1)).expect("risposta");
        assert_eq!(crate::ic820h_civ::classify(&crate::ic820h_civ::parse_bytes(&lines)), crate::ic820h_civ::Answer::Freq(145_950_000), "{lines:?} {:?}", fake.commands());
    }

    /// Interfaccia senza eco: la prima lettura (eco + risposta) va in timeout, la seconda (sola risposta) riesce e il metodo si ricorda.
    #[test]
    fn ic_820h_civ_falls_back_to_the_reply_only_count_without_echo() {
        let fake = FakeRigctld::start(145_950_000);
        let rig = ic820h(&fake, 145_950_000, 435_000_000, crate::fake_rigctld::CivBehaviour { no_echo: true, ..Default::default() });
        assert!(rig.set_uplink_via_civ(435_123_456).expect("scrittura CI-V"));
        assert_eq!(fake.band_frequencies(), [145_950_000, 435_123_456]);
        assert_eq!(fake.current_band(), 0);
        let report = rig.civ_uplink_report().expect("rapporto");
        assert_eq!(report.trasporto, "W solo risposta");
        assert!(report.passi.iter().any(|p| p.contains("inviati FE FE 42 E0 03 FD") && p.contains("ricevuti FE FE E0 42 03")), "{:?}", report.passi);
        // dopo la prima trama le altre usano subito il metodo giusto: un solo `+W ... 17` (la prima lettura con eco)
        assert!(fake.commands().iter().filter(|c| c.starts_with("+W ") && c.ends_with(" 17")).count() <= 1, "{:?}", fake.commands());
    }

    #[test]
    fn ic_820h_civ_handles_the_inverted_selection_of_the_satellite_mode() {
        // ricezione sulla banda alta (secondaria), trasmissione sulla 145; con il modo satellite D1 porta sulla principale (al contrario)
        let fake = FakeRigctld::start(435_800_000);
        fake.enable_dual_bands(145_900_000, 435_800_000);
        fake.set_civ(crate::fake_rigctld::CivBehaviour { inverted: true, ..Default::default() });
        let c = fake.client(Duration::from_millis(500)).expect("client");
        c.send_raw_reply(&crate::ic820h_civ::frame_text(&[0x07, 0xD0])).expect("selezione");
        assert_eq!(fake.current_band(), 1);
        drop(c);
        let rig = start(&fake, RigConfig { model: 3032, ..RigConfig::default() });
        fake.clear_commands();
        assert!(rig.set_uplink_via_civ(145_912_345).expect("scrittura CI-V"));
        assert_eq!(fake.band_frequencies(), [145_912_345, 435_800_000]);
        assert_eq!(fake.current_band(), 1, "di nuovo sulla banda di ricezione");
    }

    fn assert_never_writes(civ: crate::fake_rigctld::CivBehaviour, why: &str) {
        let fake = FakeRigctld::start(145_950_000);
        let rig = ic820h(&fake, 145_950_000, 435_000_000, civ);
        let err = rig.set_uplink_via_civ(435_123_456).expect_err(why);
        assert_eq!(fake.civ_writes(), 0, "{why}: nessuna 05 ({:?})", fake.commands());
        assert_eq!(fake.band_frequencies()[0], if civ.main_moves_on_return { fake.band_frequencies()[0] } else { 145_950_000 }, "{why}");
        assert_eq!(fake.band_frequencies()[1], 435_000_000, "{why}");
        assert_eq!(fake.current_band(), 0, "{why}: la radio resta sulla banda di ricezione");
        let report = rig.civ_uplink_report().expect("rapporto");
        assert_eq!(report.stato, "fallita", "{why}: {report:?}");
        assert!(!report.passi.is_empty());
        // la prova fallita non si ripete e non scrive mai
        fake.clear_commands();
        assert!(rig.set_uplink_via_civ(435_123_900).is_err());
        assert!(fake.commands().iter().all(|c| !c.starts_with("+W")), "{why}: {:?}", fake.commands());
        drop(err);
    }

    #[test]
    fn ic_820h_civ_a_failed_probe_is_tried_again_after_a_minute_without_reconnecting() {
        // SAT spento sulla radio: la prova fallisce; riacceso, dopo un minuto si riprova da sola e scrive (prima serviva ricollegare)
        let fake = FakeRigctld::start(145_950_000);
        let rig = ic820h(&fake, 145_950_000, 435_000_000, crate::fake_rigctld::CivBehaviour { select_ignored: true, ..Default::default() });
        assert!(rig.set_uplink_via_civ(435_123_456).is_err());
        fake.set_civ(crate::fake_rigctld::CivBehaviour::default());
        assert!(rig.set_uplink_via_civ(435_123_456).is_err(), "nei primi secondi non si riprova");
        assert_eq!(fake.civ_writes(), 0);
        if let Ok(mut civ) = rig.inner.civ.lock() {
            civ.failed_at = Some(Instant::now() - CIV_RETRY - Duration::from_secs(1));
        }
        assert!(rig.set_uplink_via_civ(435_123_456).expect("scrittura CI-V dopo il nuovo tentativo"));
        assert_eq!(fake.civ_writes(), 1);
    }

    #[test]
    fn ic_820h_civ_probe_fails_without_writing_when_d1_does_not_change_band() {
        assert_never_writes(crate::fake_rigctld::CivBehaviour { select_ignored: true, ..Default::default() }, "D1 non cambia banda");
    }

    #[test]
    fn ic_820h_civ_probe_fails_without_writing_when_the_radio_answers_fa() {
        assert_never_writes(crate::fake_rigctld::CivBehaviour { select_refused: true, ..Default::default() }, "FA");
    }

    #[test]
    fn ic_820h_civ_probe_fails_without_writing_when_only_the_echo_comes_back() {
        assert_never_writes(crate::fake_rigctld::CivBehaviour { echo_only: true, ..Default::default() }, "solo eco");
    }

    #[test]
    fn ic_820h_civ_probe_fails_without_writing_when_the_main_band_moves_after_the_return() {
        assert_never_writes(crate::fake_rigctld::CivBehaviour { main_moves_on_return: true, ..Default::default() }, "principale cambiata");
    }

    #[test]
    fn ic_820h_civ_switches_off_after_an_unconfirmed_write_and_comes_back_to_the_receive_band() {
        let fake = FakeRigctld::start(145_950_000);
        let rig = ic820h(&fake, 145_950_000, 435_000_000, crate::fake_rigctld::CivBehaviour { write_off: true, ..Default::default() });
        assert!(rig.set_uplink_via_civ(435_123_456).is_err());
        assert_eq!(fake.band_frequencies()[0], 145_950_000, "la ricezione e' intatta");
        assert_eq!(fake.current_band(), 0);
        assert_eq!(rig.civ_uplink_report().expect("rapporto").stato, "spenta");
        fake.clear_commands();
        assert!(rig.set_uplink_via_civ(435_123_900).is_err());
        assert!(fake.commands().iter().all(|c| !c.starts_with("+W")));
    }

    #[test]
    fn ic_820h_civ_refuses_an_uplink_on_the_receive_band_and_other_models() {
        let fake = FakeRigctld::start(145_950_000);
        let rig = ic820h(&fake, 145_950_000, 435_000_000, Default::default());
        assert!(rig.set_uplink_via_civ(145_990_000).is_err());
        assert_eq!(fake.civ_writes(), 0);
        assert_eq!(fake.band_frequencies(), [145_950_000, 435_000_000]);
        assert_eq!(rig.civ_uplink_report().expect("rapporto").stato, "riuscita", "la prova resta valida");
        assert!(rig.set_uplink_via_civ(435_100_000).expect("dopo il transponder giusto"));
        let other = FakeRigctld::start(145_950_000);
        let rig = start(&other, RigConfig { model: 3070, ..RigConfig::default() });
        other.clear_commands();
        assert!(rig.set_uplink_via_civ(435_000_000).is_err());
        assert!(rig.civ_uplink_report().is_none());
        assert!(other.commands().iter().all(|c| !c.starts_with("+W")));
    }

    #[test]
    fn ic_820h_civ_never_switches_band_while_transmitting() {
        let fake = FakeRigctld::start(145_950_000);
        let rig = ic820h(&fake, 145_950_000, 435_000_000, Default::default());
        rig.inner.external_tx.store(true, Ordering::SeqCst);
        assert!(!rig.set_uplink_via_civ(435_123_456).expect("in trasmissione"));
        assert!(fake.commands().iter().all(|c| !c.starts_with("+W")));
    }
}
