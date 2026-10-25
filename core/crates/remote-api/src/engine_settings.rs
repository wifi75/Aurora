//! Impostazioni del motore proprio di Aurora: sono di Aurora, non di JTDX.
//!
//! Con `AURORA_ENGINE=native` JTDX non c'e': scrivere il suo `JTDX.ini` non cambierebbe niente. Le impostazioni stanno in un file JSON versionato
//! (`settings-motore.json`, nella cartella dei dati) con copia di sicurezza e scrittura atomica. Il `JTDX.ini` serve **una sola volta**, come
//! valori iniziali, e solo se il file di Aurora non esiste ancora. Le chiavi hanno i nomi di JTDX (cosi' si ritrovano nei suoi documenti e
//! l'interfaccia le riconosce), ma i valori sono di Aurora.
//!
//! Ogni voce ha uno stato (`support`): **viva** (ha un effetto vero, subito o al riavvio) oppure **non disponibile** con il motivo. Le voci non
//! disponibili non si possono modificare: nessun controllo finto.

use std::{
    collections::BTreeMap,
    net::{IpAddr, Ipv4Addr, SocketAddr},
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
};

use reporting::{secret::SecretStore, status::ReportStatus};

use engine_jtdx::ini::FIELDS;
use serde_json::{json, Value};
use tokio::sync::watch;

/// Versione del formato del file.
pub const FILE_VERSION: u32 = 1;
/// Nome del file nella cartella dei dati.
pub const FILE_NAME: &str = "settings-motore.json";

/// Componenti di cui una voce ha bisogno per avere un effetto vero: chi li costruisce li dichiara pronti con `set_feature`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Feature {
    /// L'emettitore dei pacchetti WSJT-X (Heartbeat, Status, Decode, QsoLogged, LoggedADIF) verso il primo server UDP.
    UdpEmitter,
    /// La scrittura di `ALL.TXT` del motore proprio.
    AllTxt,
    /// Chi apre e chiude gli ingressi e le uscite audio e salva i file audio (scheda Audio): vedi `audio_api`.
    AudioControl,
    /// La protezione dei segreti del sistema (su Windows DPAPI): senza, la password di eQSL non si puo' salvare e l'invio resta spento.
    Secrets,
    /// La finestra del programma risponde ai comandi UDP (primo piano e ripristino): solo il programma Windows con la sua finestra.
    WindowControl,
}

/// Quando ha effetto una voce viva.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Apply {
    /// Subito, senza riavviare niente.
    Now,
    /// Al prossimo avvio di Aurora (il componente legge il valore solo quando parte).
    Restart,
}

/// Cosa fa Aurora di una voce.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Support {
    Live(Apply),
    /// Viva solo quando il componente e' pronto; prima non disponibile, con il motivo dato.
    Needs(Feature, &'static str, Apply),
    /// Non c'e' nel motore proprio: il motivo spiega perche'.
    Hidden(&'static str),
}

/// Voci della scheda Segnalazioni di JTDX che non sono in `engine_jtdx::ini::FIELDS` (non si leggono ne' si scrivono nel `JTDX.ini`).
/// `EQSLPasswd`: la password di eQSL **non si legge mai dal `JTDX.ini`** e non sta nel file delle impostazioni: si salva solo nella protezione dei segreti del sistema (`SECRET_KEYS`).
/// `Region` e `Type2MsgGen` stanno nel `JTDX.ini` come valori binari (`@Variant`): non si leggono ne' si scrivono, ma la scheda Generali li mostra.
pub const EXTRA_KEYS: &[&str] = &["EQSLPasswd", "Region", "Type2MsgGen"];

/// Le voci che non finiscono mai nel file delle impostazioni ne' nelle risposte dell'API: hanno un posto protetto (`SecretStore`).
pub const SECRET_KEYS: &[&str] = &["EQSLPasswd"];

/// Le voci della scheda Audio: stanno nel file ma si cambiano solo dalla rotta `audio/settings` (apre i dispositivi, poi salva).
pub const AUDIO_KEYS: &[&str] = &["SoundInName", "SoundOutName", "AudioInputChannel", "AudioOutputChannel", "SaveDir", "SaveWav"];

/// Le voci delle evidenziazioni di «nuovo» (scheda Notifiche): una per criterio, con «per banda» e «per modo».
const NEW_PREFIXES: [&str; 6] = ["newCQZ", "newITUZ", "newDXCC", "newGrid", "newPx", "newCall"];

fn is_highlight_key(key: &str) -> bool {
    NEW_PREFIXES.iter().any(|p| key == *p || key.strip_prefix(p).is_some_and(|rest| rest == "Band" || rest == "BandMode"))
}

/// Tutte le voci di cui Aurora racconta lo stato: quelle di JTDX piu' `EXTRA_KEYS`.
pub fn all_keys() -> Vec<&'static str> {
    let mut keys: Vec<&'static str> = FIELDS.iter().map(|(k, _)| *k).collect();
    for k in EXTRA_KEYS {
        if !keys.contains(k) {
            keys.push(k);
        }
    }
    keys
}

/// Lo stato di ogni voce di JTDX nel motore proprio (vedi `docs/impostazioni-motore-proprio.md`).
pub fn support(key: &str) -> Support {
    match key {
        "MyCall" | "MyGrid" => Support::Live(Apply::Restart),
        "UDP2Server" | "UDP2ServerPort" | "EnableUDP2adifBroadcast" => Support::Live(Apply::Now),
        "UDPServer" | "UDPServerPort" | "EnableUDP1adifSending" | "AcceptUDPRequests" | "AcceptUDPReplyMessages" => Support::Needs(Feature::UdpEmitter, "emitter_pending", Apply::Now),
        "WriteDecodedALLTXT" | "WriteDecodedDebugALLTXT" => Support::Needs(Feature::AllTxt, "all_txt_pending", Apply::Now),
        // scheda Sequenza: i contatori, «ferma se risponde a un altro» e «CQ rigoroso» arrivano alla sequenza a caldo (`engine_native::SequenceSettings`)
        "SeqAnswerCQCount" | "SeqAnswerCQCounterValue" | "SeqAnswerInCallCount" | "SeqAnswerInCallCounterValue" | "SeqSentRReportCount" | "SeqSentRReportCounterValue"
        | "SeqSentRR7373Count" | "SeqSentRR7373CounterValue" | "SeqHaltTxReplyOther" | "StrictDirectionalCQ" | "CallPriorityCQ" | "MaxDistance" => Support::Live(Apply::Now),
        // scheda Generali: periodo di monitoraggio Tx (watchdog), cambi di frequenza Tx durante la trasmissione, filtro del log, non monitorare all'avvio
        "TxWatchdogTimer" | "TxQSYAllowed" | "callsignLogFiltering" | "gridLogFiltering" | "timeFromLogFiltering" => Support::Live(Apply::Now),
        "MonitorOFF" => Support::Live(Apply::Restart),
        "MonitorLastUsed" => Support::Hidden("monitor_last"),
        "Region" => Support::Hidden("iaru_region"),
        "Type2MsgGen" => Support::Hidden("type2_msgs"),
        "Decode52" => Support::Hidden("decode52"),
        "After73" | "IDint" => Support::Hidden("cw_id"),
        "TuneTimer" => Support::Hidden("tune_timer"),
        "InsertBlank" | "Scroll" | "HideTxMessages" | "Tx2QSO" | "countryPrefix" | "showAzimuth" | "AlignDistanceAzimuth" | "AlignInt" => Support::Hidden("own_screen"),
        // scheda Notifiche: evidenziazioni di «nuovo» (anche per la sequenza), «gia' lavorato», colori invertiti e marcatori
        k if is_highlight_key(k) => Support::Live(Apply::Now),
        "newPotential" | "73RR73Marker" => Support::Live(Apply::Now),
        // come si disegnano i messaggi (barrato, colori invertiti, marcatori): scelte dell'interfaccia, per dispositivo (scheda Notifiche)
        "workedColor" | "workedStriked" | "workedUnderlined" | "workedDontShow" | "txtColor" | "OtherStandardMessagesMarker" | "redMarker" | "blueMarker" | "hidehintMarker" => {
            Support::Hidden("own_notify")
        }
        // porta in primo piano / ripristina la finestra a un comando UDP: serve la finestra del programma Windows
        "udpWindowToFront" | "udpWindowRestore" => Support::Needs(Feature::WindowControl, "udp_window", Apply::Now),
        // registrazione del log: finestra di conferma, log automatico, record ADIF (modo, commento) e pulizia del DX
        "toRTTY" | "dBtoComments" | "distanceToComments" | "ClearCallGrid" => Support::Live(Apply::Now),
        // finestra di conferma e registrazione a mano: la pagina non li ha ancora, il QSO si registra sempre da solo
        "PromptToLog" => Support::Hidden("log_prompt"),
        "AutoQSOLogging" => Support::Hidden("log_auto"),
        "AllowSpotsDXSummit" => Support::Hidden("dxsummit"),
        "ClearCallGridExit" => Support::Hidden("clear_exit"),
        // un invio del record ADIF di ogni QSO al server TCP
        "TCPServer" | "TCPServerPort" | "EnableTCPConnection" => Support::Live(Apply::Now),
        // filtri di testo e falsi decode sul flusso UDP: li applica l'emettitore
        "ApplyFiltersToUDPmessages" => Support::Hidden("udp_filters"),
        "preventFalseUDPspots" => Support::Needs(Feature::UdpEmitter, "emitter_pending", Apply::Now),
        // spotting verso internet: PSK Reporter in automatico, DXSummit a richiesta (pulsante), eQSL solo con la protezione dei segreti del sistema
        "PSKReporter" => Support::Live(Apply::Now),
        "EQSLSend" | "EQSLUser" | "EQSLPasswd" | "EQSLNick" => Support::Needs(Feature::Secrets, "eqsl", Apply::Now),
        "Language" | "Miles" | "showDistance" | "countryName" | "UseDarkStyle" => Support::Hidden("own_screen"),
        "Rig" | "PTTport" | "CATSerialPort" | "CATSerialRate" | "Polling" | "CATForceDTR" | "CATForceRTS" | "TCIAudio" => Support::Hidden("radio_cat"),
        // scheda Audio: si cambiano solo dalla rotta `audio/settings`, che apre i dispositivi prima di salvare
        "SoundInName" | "SoundOutName" | "AudioInputChannel" | "AudioOutputChannel" | "SaveDir" | "SaveWav" => Support::Needs(Feature::AudioControl, "audio_start", Apply::Now),
        "AutoSequence" | "SingleShotQSO" | "SkipTx1" | "AnswerWorkedB4" => Support::Hidden("own_rules"),
        k if k.starts_with("AlertOn") || k.starts_with("BeepOn") => Support::Hidden("own_alerts"),
        _ => Support::Hidden("own_decoder"),
    }
}

/// Le voci che il motore proprio tiene davvero nel file (vive o in attesa di un componente).
pub fn is_stored(key: &str) -> bool {
    matches!(support(key), Support::Live(_) | Support::Needs(..))
}

/// Le voci che stanno nel file `settings-motore.json` (tutte le tenute, tranne i segreti).
pub fn in_file(key: &str) -> bool {
    is_stored(key) && !SECRET_KEYS.contains(&key)
}

/// Una voce viva che, nel motore proprio, vale come in JTDX ma per FT8 e FT4 non cambia mai niente: la pagina lo dice sotto la voce.
pub fn note_of(key: &str) -> Option<&'static str> {
    match key {
        // JTDX converte in RTTY solo i modi che cominciano per JT9 (`log_as_RTTY() and mode.left(3)=="JT9"`)
        "toRTTY" => Some("jt9_only"),
        // il marcatore di decodifica errata (`?`) esiste solo per JT65 e JT9: FT8 e FT4 non lo producono mai
        "preventFalseUDPspots" => Some("jt_marker_only"),
        _ => None,
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SettingsError {
    /// Voce sconosciuta, non disponibile nel motore proprio o in attesa di un componente non ancora pronto.
    NotAvailable(String),
    /// Valore non ammesso.
    BadValue(String),
    Io(String),
}

impl std::fmt::Display for SettingsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SettingsError::NotAvailable(k) => write!(f, "{k}: voce non disponibile nel motore proprio"),
            SettingsError::BadValue(m) => write!(f, "{m}"),
            SettingsError::Io(e) => write!(f, "scrittura delle impostazioni: {e}"),
        }
    }
}

impl std::error::Error for SettingsError {}

/// Quello che il resto del programma deve sapere, gia' interpretato (indirizzi veri, niente testo).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Reports {
    pub call: String,
    pub grid: String,
    /// Primo server UDP (GridTracker...): destinatario del flusso WSJT-X. `None` se vuoto o non valido.
    pub primary: Option<SocketAddr>,
    /// «Invia il QSO in ADIF al server primario»: pacchetto `LoggedADIF` (tipo 12) oltre al `QsoLogged` (tipo 5).
    pub adif_to_primary: bool,
    /// `AcceptUDPRequests`: i comandi di GridTracker (Reply, HaltTx, FreeText...) arrivano ad Aurora.
    pub accept_requests: bool,
    /// `AcceptUDPReplyMessages`: 1 solo CQ, 2 CQ e 73, 3 qualsiasi messaggio.
    pub accept_reply: u8,
    /// Secondo server UDP (Log4OM2): destinatario del record ADIF; `Some` solo se l'invio e' acceso e l'indirizzo e' valido.
    pub secondary: Option<SocketAddr>,
    /// Scrivi le decodifiche in `ALL.TXT`.
    pub all_txt: bool,
    /// Le evidenziazioni di «nuovo» (scheda Notifiche): decidono anche chi e' «gia' lavorato» e se la sequenza risponde a chiunque.
    pub highlights: engine_jtdx::worked::HighlightFlags,
    /// Contatori della scheda Sequenza: massimo di trasmissioni dello stesso messaggio, `None` = contatore spento.
    pub seq_answer_cq: Option<u32>,
    pub seq_answer_in_call: Option<u32>,
    pub seq_sent_r_report: Option<u32>,
    pub seq_closing: Option<u32>,
    /// «Ferma il Tx se l'operatore che stai chiamando ha risposto ad un altro» (`SeqHaltTxReplyOther`).
    pub seq_halt_reply_other: bool,
    /// «Operazione CQ rigorosa» (`StrictDirectionalCQ`).
    pub strict_directional_cq: bool,
    /// «Periodo monitoraggio Tx» in minuti (`TxWatchdogTimer`, 0 = senza).
    pub watchdog_min: u32,
    /// «Consenti cambi di frequenza Tx durante la trasmissione» (`TxQSYAllowed`).
    pub tx_qsy_allowed: bool,
    /// Il filtro del log della scheda Generali.
    pub log_filter: engine_jtdx::worked::LogFilter,
    /// «Non monitorare all'avvio» (`MonitorOFF`).
    pub monitor_off_at_start: bool,
    /// Marcatore dei messaggi di chiusura fra altre stazioni (`73RR73Marker`): fanno entrare la stazione tra i candidati della sequenza.
    pub rr73_marker: bool,
    /// «Priorita' ai CQ» (`CallPriorityCQ`) e «Priorita' alla distanza maggiore» (`MaxDistance`).
    pub call_prio_cq: bool,
    pub max_distance: bool,
    /// «Richiedimi di loggare il QSO» (`PromptToLog`): finestra di conferma prima di registrare. Esclude `autolog`.
    pub prompt_to_log: bool,
    /// «Messa a log automatica del QSO» (`AutoQSOLogging`): il QSO si registra da solo e a fine QSO la trasmissione si ferma. Senza questa e senza la
    /// conferma il QSO si registra solo dal pulsante «Registra QSO».
    pub autolog: bool,
    /// «Converti il modo in RTTY» (`toRTTY`): vale solo per JT9, come in JTDX.
    pub to_rtty: bool,
    /// «Rapporto dB nel commento» (`dBtoComments`) e «Distanza nei commenti» (`distanceToComments`).
    pub report_in_comments: bool,
    pub distance_in_comments: bool,
    /// «Cancella i dati dopo aver loggato il QSO» (`ClearCallGrid`).
    pub clear_dx: bool,
    /// Server TCP che riceve il record ADIF di ogni QSO: `Some` solo se `EnableTCPConnection` e' acceso e l'indirizzo e' valido.
    pub tcp: Option<SocketAddr>,
    /// Spotting su PSK Reporter (`PSKReporter`) e pulsante di spot su DXSummit (`AllowSpotsDXSummit`).
    pub psk: bool,
    pub dxsummit: bool,
    /// Invio a eQSL: acceso (`EQSLSend`), utente e nickname; la password sta nella protezione dei segreti.
    pub eqsl: bool,
    pub eqsl_user: String,
    pub eqsl_nick: String,
    /// «Applica filtri di testo alla trasmissione dei messaggi UDP» e «Prevenire ... nominativi non confermati» (`ApplyFiltersToUDPmessages`, `preventFalseUDPspots`).
    pub udp_apply_filters: bool,
    pub udp_prevent_false: bool,
    /// «Notifica dell'accettazione della richiesta UDP» e «Finestra di ripristino richieste UDP accettate» (`udpWindowToFront`, `udpWindowRestore`).
    pub udp_window_to_front: bool,
    pub udp_window_restore: bool,
    /// «Messaggi decodificati e di debug» (`WriteDecodedDebugALLTXT`).
    pub all_txt_debug: bool,
}

/// Le voci della scheda Audio (nomi di dispositivo vuoti = predefinito del sistema; `save_dir` vuota = cartella `save` dei dati; `save_mode` 0, 1, 2).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct AudioValues {
    pub input: String,
    pub output: String,
    pub input_channel: String,
    pub output_channel: String,
    pub save_dir: String,
    pub save_mode: String,
}

fn truthy(value: Option<&String>) -> bool {
    value.is_some_and(|v| v == "true")
}

fn parse_host(host: &str) -> Option<IpAddr> {
    let host = host.trim();
    if host.eq_ignore_ascii_case("localhost") {
        return Some(IpAddr::V4(Ipv4Addr::LOCALHOST));
    }
    host.parse().ok()
}

/// Indirizzo e porta di un server come testo; `None` se l'indirizzo e' vuoto o non e' un IP, o la porta non e' 1-65535.
pub fn parse_server(host: &str, port: &str) -> Option<SocketAddr> {
    let port: u16 = port.trim().parse().ok().filter(|p| *p != 0)?;
    Some(SocketAddr::new(parse_host(host)?, port))
}

/// Valori iniziali quando non c'e' un `JTDX.ini` da cui importare: quelli della schermata di JTDX (server 127.0.0.1, porta 2237, invii accesi, ALL.TXT dei decodificati).
pub fn defaults() -> BTreeMap<String, String> {
    [
        ("MyCall", ""),
        ("MyGrid", ""),
        ("UDPServer", "127.0.0.1"),
        ("UDPServerPort", "2237"),
        ("EnableUDP1adifSending", "true"),
        // un Reply di GridTracker fa trasmettere: di fabbrica spento (si importa da JTDX se li accettava)
        ("AcceptUDPRequests", "false"),
        ("AcceptUDPReplyMessages", "1"),
        ("UDP2Server", "127.0.0.1"),
        ("UDP2ServerPort", "2237"),
        ("EnableUDP2adifBroadcast", "true"),
        ("WriteDecodedALLTXT", "true"),
        ("WriteDecodedDebugALLTXT", "false"),
        // Registrazione del log: di fabbrica come Aurora ha sempre fatto (il QSO si registra da solo); i valori di JTDX di fabbrica sono prompt acceso e log automatico spento
        ("toRTTY", "false"),
        ("dBtoComments", "false"),
        ("distanceToComments", "false"),
        ("ClearCallGrid", "false"),
        // Server TCP (JTDX: 127.0.0.1, porta 52001, spento)
        ("TCPServer", "127.0.0.1"),
        ("TCPServerPort", "52001"),
        ("EnableTCPConnection", "false"),
        // Servizi di rete: tutto spento senza un JTDX.ini che li accendeva (JTDX: PSKReporter, AllowSpotsDXSummit e EQSLSend spenti)
        // per scelta del proprietario lo spotting su PSK Reporter e' acceso di base (si spegne dalla scheda Segnalazioni)
        ("PSKReporter", "true"),
        ("EQSLSend", "false"),
        ("EQSLUser", ""),
        ("EQSLNick", ""),
        // Server UDP primario (JTDX: `preventFalseUDPspots` acceso, il resto spento)
        ("preventFalseUDPspots", "true"),
        ("udpWindowToFront", "false"),
        ("udpWindowRestore", "false"),
        // Audio: dispositivi predefiniti del sistema (nome vuoto), canale Mono, nessun file audio, cartella `save` dei dati
        ("SoundInName", ""),
        ("SoundOutName", ""),
        ("AudioInputChannel", "Mono"),
        ("AudioOutputChannel", "Mono"),
        ("SaveDir", ""),
        ("SaveWav", "0"),
        // Sequenza: valori di fabbrica dei contatori di JTDX (2 / 3 / 5 / 4); accesi come la configurazione che Aurora ha sempre avuto (rapporto R e chiusura)
        ("SeqAnswerCQCount", "false"),
        ("SeqAnswerCQCounterValue", "2"),
        ("SeqAnswerInCallCount", "false"),
        ("SeqAnswerInCallCounterValue", "3"),
        ("SeqSentRReportCount", "true"),
        ("SeqSentRReportCounterValue", "5"),
        ("SeqSentRR7373Count", "true"),
        ("SeqSentRR7373CounterValue", "5"),
        ("SeqHaltTxReplyOther", "true"),
        ("StrictDirectionalCQ", "false"),
        ("CallPriorityCQ", "false"),
        ("MaxDistance", "false"),
        // Generali
        ("TxWatchdogTimer", "6"),
        ("TxQSYAllowed", "true"),
        ("MonitorOFF", "false"),
        ("callsignLogFiltering", "false"),
        ("gridLogFiltering", "false"),
        ("timeFromLogFiltering", ""),
        // Notifiche: valori di fabbrica di JTDX (`Configuration::read_settings`)
        ("newCQZ", "false"),
        ("newCQZBand", "false"),
        ("newCQZBandMode", "false"),
        ("newITUZ", "false"),
        ("newITUZBand", "false"),
        ("newITUZBandMode", "false"),
        ("newDXCC", "true"),
        ("newDXCCBand", "true"),
        ("newDXCCBandMode", "true"),
        ("newGrid", "false"),
        ("newGridBand", "false"),
        ("newGridBandMode", "false"),
        ("newPx", "false"),
        ("newPxBand", "false"),
        ("newPxBandMode", "false"),
        ("newCall", "true"),
        ("newCallBand", "true"),
        ("newCallBandMode", "true"),
        ("newPotential", "false"),
        ("73RR73Marker", "true"),
    ]
    .iter()
    .map(|(k, v)| ((*k).to_string(), (*v).to_string()))
    .collect()
}

/// I valori iniziali presi dal `JTDX.ini` (lettura con `engine_jtdx::ini::read_shown`), **una sola volta**.
///
/// Se il primo server di JTDX era l'ascolto di Aurora stessa (`own_listen`: l'hub UDP, di solito 127.0.0.1:2334), il flusso a GridTracker non
/// passava dal primo server ma dall'inoltro: allora il primo server diventa il primo indirizzo di `AURORA_UDP_FORWARD`. Chi non lo ha lascia vuoto.
pub fn import_from_ini(ini: &BTreeMap<String, String>, own_listen: SocketAddr, env_forward: &[SocketAddr]) -> BTreeMap<String, String> {
    let mut out = defaults();
    for key in out.clone().keys() {
        if let Some(value) = ini.get(key)
            && crate::engine_settings::validate(key, value).is_ok()
        {
            out.insert(key.clone(), value.clone());
        }
    }
    let ini_primary = parse_server(ini.get("UDPServer").map_or("", String::as_str), ini.get("UDPServerPort").map_or("", String::as_str));
    let own = |a: SocketAddr| a == own_listen || (a.port() == own_listen.port() && a.ip().is_loopback() && own_listen.ip().is_loopback());
    // il primo server vuoto: niente invio (e nemmeno l'ADIF al primo server)
    let seed = env_forward.first().copied();
    let primary = match ini_primary {
        Some(a) if own(a) => seed,
        Some(a) => Some(a),
        None if ini.contains_key("UDPServer") => seed,
        // nessun JTDX.ini: valori di fabbrica, salvo un inoltro scelto con AURORA_UDP_FORWARD
        None => seed.or(Some(addr_default())),
    };
    // la password di eQSL non si importa mai: senza, l'invio resta spento finche' non la si imposta in Aurora
    out.insert("EQSLSend".into(), "false".into());
    match primary {
        Some(a) => {
            out.insert("UDPServer".into(), a.ip().to_string());
            out.insert("UDPServerPort".into(), a.port().to_string());
        }
        None => {
            out.insert("UDPServer".into(), String::new());
            out.insert("EnableUDP1adifSending".into(), "false".into());
        }
    }
    out
}

fn addr_default() -> SocketAddr {
    SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 2237)
}

/// Controlla un valore di una voce che il motore proprio tiene (tipo, intervallo e forma dell'indirizzo).
pub fn validate(key: &str, value: &str) -> Result<(), SettingsError> {
    if !is_stored(key) {
        return Err(SettingsError::NotAvailable(key.to_string()));
    }
    // il nominativo e il locatore possono essere vuoti (nessuno impostato): gli altri testi anche, gli indirizzi si controllano sotto
    if value.is_empty() && matches!(key, "MyCall" | "MyGrid" | "UDPServer" | "UDP2Server" | "TCPServer" | "EQSLUser" | "EQSLNick" | "timeFromLogFiltering" | "SoundInName" | "SoundOutName" | "SaveDir") {
        return Ok(());
    }
    engine_jtdx::ini_edit::check_value(key, value).map_err(SettingsError::BadValue)?;
    match key {
        "SoundInName" | "SoundOutName" if value.chars().count() > 200 || value.chars().any(char::is_control) => Err(SettingsError::BadValue(format!("{key}: nome del dispositivo non valido"))),
        "AudioInputChannel" if !matches!(value, "Mono" | "Left" | "Right") => Err(SettingsError::BadValue("AudioInputChannel: Mono, Left o Right".into())),
        "AudioOutputChannel" if !matches!(value, "Mono" | "Left" | "Right" | "Both") => Err(SettingsError::BadValue("AudioOutputChannel: Mono, Left, Right o Both".into())),
        "SaveDir" if value.chars().count() > 260 || value.chars().any(|c| c.is_control() || matches!(c, '<' | '>' | '"' | '|' | '?' | '*')) => {
            Err(SettingsError::BadValue("SaveDir: percorso non valido".into()))
        }
        "UDPServer" | "UDP2Server" | "TCPServer" if parse_host(value).is_none() => Err(SettingsError::BadValue(format!("{key}: serve un indirizzo IP (o localhost)"))),
        "UDPServerPort" | "UDP2ServerPort" | "TCPServerPort" if parse_server("127.0.0.1", value).is_none() => Err(SettingsError::BadValue(format!("{key}: la porta va da 1 a 65535"))),
        "EQSLUser" | "EQSLNick" if value.chars().count() > 40 => Err(SettingsError::BadValue(format!("{key}: al piu' 40 caratteri"))),
        k if k.starts_with("Seq") && k.ends_with("CounterValue") && !value.parse::<u32>().is_ok_and(|n| (1..=5).contains(&n)) => {
            Err(SettingsError::BadValue(format!("{k}: il valore va da 1 a 5")))
        }
        "TxWatchdogTimer" if !value.parse::<u32>().is_ok_and(|n| n <= 99) => Err(SettingsError::BadValue("TxWatchdogTimer: i minuti vanno da 0 a 99".into())),
        "timeFromLogFiltering" if engine_jtdx::worked::LogFilter::parse_from(value).is_none() => {
            Err(SettingsError::BadValue("timeFromLogFiltering: serve AAAAMMGGHHMMSS (solo cifre, al piu' 14)".into()))
        }
        "MyCall" if !value.chars().all(|c| c.is_ascii_alphanumeric() || c == '/') || value.len() > 20 => Err(SettingsError::BadValue("MyCall: nominativo non valido".into())),
        "MyGrid" if !(value.len() == 4 || value.len() == 6 || value.len() == 8) || !value.chars().all(|c| c.is_ascii_alphanumeric()) => {
            Err(SettingsError::BadValue("MyGrid: il locatore ha 4, 6 o 8 caratteri".into()))
        }
        _ => Ok(()),
    }
}

/// Interpreta i valori: indirizzi veri e interruttori.
pub fn interpret(values: &BTreeMap<String, String>) -> Reports {
    let get = |k: &str| values.get(k).map_or("", String::as_str);
    Reports {
        call: get("MyCall").to_uppercase(),
        grid: get("MyGrid").to_string(),
        primary: parse_server(get("UDPServer"), get("UDPServerPort")),
        adif_to_primary: truthy(values.get("EnableUDP1adifSending")),
        accept_requests: truthy(values.get("AcceptUDPRequests")),
        accept_reply: get("AcceptUDPReplyMessages").parse().ok().filter(|n| (1..=3).contains(n)).unwrap_or(1),
        secondary: truthy(values.get("EnableUDP2adifBroadcast")).then(|| parse_server(get("UDP2Server"), get("UDP2ServerPort"))).flatten(),
        all_txt: truthy(values.get("WriteDecodedALLTXT")),
        highlights: engine_jtdx::worked::HighlightFlags::from_lookup(|k| values.get(k).map(|v| v == "true")),
        seq_answer_cq: counter(values, "SeqAnswerCQCount", "SeqAnswerCQCounterValue", 2),
        seq_answer_in_call: counter(values, "SeqAnswerInCallCount", "SeqAnswerInCallCounterValue", 3),
        seq_sent_r_report: counter(values, "SeqSentRReportCount", "SeqSentRReportCounterValue", 5),
        seq_closing: counter(values, "SeqSentRR7373Count", "SeqSentRR7373CounterValue", 4),
        // JTDX: `SeqHaltTxReplyOther` vale vero se manca
        seq_halt_reply_other: values.get("SeqHaltTxReplyOther").is_none_or(|v| v == "true"),
        strict_directional_cq: truthy(values.get("StrictDirectionalCQ")),
        watchdog_min: get("TxWatchdogTimer").parse().ok().filter(|n| *n <= 99).unwrap_or(6),
        tx_qsy_allowed: values.get("TxQSYAllowed").is_none_or(|v| v == "true"),
        log_filter: engine_jtdx::worked::LogFilter {
            call: truthy(values.get("callsignLogFiltering")).then(|| get("MyCall").to_uppercase()).filter(|c| !c.is_empty()),
            grid: truthy(values.get("gridLogFiltering")).then(|| get("MyGrid").to_string()).filter(|g| !g.is_empty()),
            from: engine_jtdx::worked::LogFilter::parse_from(get("timeFromLogFiltering")),
        },
        monitor_off_at_start: truthy(values.get("MonitorOFF")),
        rr73_marker: values.get("73RR73Marker").is_none_or(|v| v == "true"),
        call_prio_cq: truthy(values.get("CallPriorityCQ")),
        max_distance: truthy(values.get("MaxDistance")),
        prompt_to_log: truthy(values.get("PromptToLog")),
        // JTDX: `AutoQSOLogging` vale falso se manca; Aurora ha sempre registrato da sola, quindi senza la voce vale vero
        autolog: values.get("AutoQSOLogging").is_none_or(|v| v == "true"),
        to_rtty: truthy(values.get("toRTTY")),
        report_in_comments: truthy(values.get("dBtoComments")),
        distance_in_comments: truthy(values.get("distanceToComments")),
        clear_dx: truthy(values.get("ClearCallGrid")),
        tcp: truthy(values.get("EnableTCPConnection")).then(|| parse_server(get("TCPServer"), get("TCPServerPort"))).flatten(),
        psk: truthy(values.get("PSKReporter")),
        dxsummit: truthy(values.get("AllowSpotsDXSummit")),
        eqsl: truthy(values.get("EQSLSend")) && !get("EQSLUser").is_empty(),
        eqsl_user: get("EQSLUser").to_string(),
        eqsl_nick: get("EQSLNick").to_string(),
        udp_apply_filters: truthy(values.get("ApplyFiltersToUDPmessages")),
        // JTDX: `preventFalseUDPspots` vale vero se manca
        udp_prevent_false: values.get("preventFalseUDPspots").is_none_or(|v| v == "true"),
        udp_window_to_front: truthy(values.get("udpWindowToFront")),
        udp_window_restore: truthy(values.get("udpWindowRestore")),
        all_txt_debug: truthy(values.get("WriteDecodedDebugALLTXT")),
    }
}

/// Un contatore della scheda Sequenza: il valore se la casella e' accesa (1-5), altrimenti `None`.
fn counter(values: &BTreeMap<String, String>, on: &str, value: &str, default: u32) -> Option<u32> {
    truthy(values.get(on)).then(|| values.get(value).and_then(|v| v.parse().ok()).filter(|n| (1..=5).contains(n)).unwrap_or(default))
}

/// Destinatari del record ADIF in UDP: quelli fissi (ambiente) piu' il server secondario, **senza doppioni**.
///
/// Regola anti-doppione: lo stesso indirizzo non compare due volte (ambiente e impostazioni uguali) e, se il secondo server coincide con il primo
/// mentre il primo riceve gia' il QSO in ADIF (`LoggedADIF`), il datagramma semplice non parte: chi ascolta li registrerebbe due volte.
pub fn export_udp_targets(fixed: &[SocketAddr], reports: &Reports) -> Vec<SocketAddr> {
    let mut out: Vec<SocketAddr> = Vec::new();
    for addr in fixed.iter().copied().chain(reports.secondary) {
        if reports.secondary == Some(addr) && reports.primary == Some(addr) && reports.adif_to_primary && !fixed.contains(&addr) {
            continue;
        }
        if !out.contains(&addr) {
            out.push(addr);
        }
    }
    out
}

/// Destinatari del flusso WSJT-X: il primo server delle impostazioni piu' gli indirizzi fissi dell'ambiente (`AURORA_UDP_FORWARD`) oltre il primo,
/// che e' servito solo come valore iniziale del primo server. Senza doppioni.
pub fn forward_targets(env_forward: &[SocketAddr], reports: &Reports) -> Vec<SocketAddr> {
    let mut out: Vec<SocketAddr> = Vec::new();
    for addr in reports.primary.into_iter().chain(env_forward.iter().skip(1).copied()) {
        if !out.contains(&addr) {
            out.push(addr);
        }
    }
    out
}

#[derive(serde::Serialize, serde::Deserialize)]
struct FileBody {
    version: u32,
    values: BTreeMap<String, String>,
}

/// Il deposito delle impostazioni. Chi ne dipende (esportatore, emettitore UDP, ALL.TXT) si iscrive con `subscribe` e rilegge `reports()`.
pub struct EngineSettingsStore {
    path: PathBuf,
    values: Mutex<BTreeMap<String, String>>,
    revision: watch::Sender<u64>,
    udp_emitter: AtomicBool,
    all_txt: AtomicBool,
    audio_control: AtomicBool,
    secrets_ready: AtomicBool,
    window_control: AtomicBool,
    /// Dove sta la password di eQSL (la protezione dei segreti del sistema); `None` finche' nessuno la collega.
    secrets: Mutex<Option<Arc<dyn SecretStore>>>,
    /// Lo stato dell'ultimo invio di ogni servizio (PSK Reporter, DXSummit, eQSL, server TCP).
    status: Arc<ReportStatus>,
}

fn read_file(path: &Path) -> Option<BTreeMap<String, String>> {
    let text = std::fs::read_to_string(path).ok()?;
    let body: FileBody = serde_json::from_str(&text).ok()?;
    (body.version <= FILE_VERSION).then_some(body.values)
}

fn backup_path(path: &Path) -> PathBuf {
    let mut name = path.file_name().map(|n| n.to_os_string()).unwrap_or_default();
    name.push(".bak");
    path.with_file_name(name)
}

impl EngineSettingsStore {
    /// Apre il file. Se manca (o e' rovinato e anche la copia lo e') si parte da `seed` (il `JTDX.ini` importato, o i valori iniziali) e lo si scrive.
    pub fn open(path: &Path, seed: impl FnOnce() -> BTreeMap<String, String>) -> EngineSettingsStore {
        let loaded = read_file(path).or_else(|| {
            let backup = read_file(&backup_path(path));
            if backup.is_some() {
                tracing::warn!("impostazioni del motore: {} non leggibile, uso la copia di sicurezza", path.display());
            }
            backup
        });
        // un valore non piu' valido o di una voce che non si tiene piu' si scarta: vale come se mancasse
        let present: BTreeMap<String, String> = loaded.unwrap_or_default().into_iter().filter(|(k, v)| validate(k, v).is_ok()).collect();
        let mut values = defaults();
        // **per singola voce**: ogni voce che nel file manca (file nuovo, o scritto prima che la voce esistesse, anche in futuro) parte dal valore
        // del `JTDX.ini` se c'e' (`seed`), altrimenti da quello di fabbrica; le voci gia' nel file non si toccano mai
        let missing = values.keys().any(|k| !present.contains_key(k));
        if missing {
            for (k, v) in seed() {
                if !present.contains_key(&k) && validate(&k, &v).is_ok() {
                    values.insert(k, v);
                }
            }
        }
        values.extend(present);
        let (revision, _) = watch::channel(0);
        let store = EngineSettingsStore { path: path.to_path_buf(), values: Mutex::new(values), revision, udp_emitter: AtomicBool::new(false), all_txt: AtomicBool::new(false), audio_control: AtomicBool::new(false), secrets_ready: AtomicBool::new(false), window_control: AtomicBool::new(false), secrets: Mutex::new(None), status: Arc::new(ReportStatus::new()) };
        if missing && let Err(e) = store.write() {
            tracing::warn!("impostazioni del motore non scritte: {e}");
        }
        store
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, BTreeMap<String, String>> {
        self.values.lock().unwrap_or_else(|p| p.into_inner())
    }

    pub fn values(&self) -> BTreeMap<String, String> {
        self.lock().clone()
    }

    pub fn reports(&self) -> Reports {
        interpret(&self.lock())
    }

    /// Le voci della scheda Audio come sono adesso nel file.
    pub fn audio(&self) -> AudioValues {
        let values = self.lock();
        let get = |k: &str| values.get(k).cloned().unwrap_or_default();
        AudioValues {
            input: get("SoundInName"),
            output: get("SoundOutName"),
            input_channel: get("AudioInputChannel"),
            output_channel: get("AudioOutputChannel"),
            save_dir: get("SaveDir"),
            save_mode: get("SaveWav"),
        }
    }

    /// Cambia a ogni modifica dei valori o dei componenti pronti: chi si iscrive rilegge `reports()`.
    pub fn subscribe(&self) -> watch::Receiver<u64> {
        self.revision.subscribe()
    }

    fn bump(&self) {
        self.revision.send_modify(|r| *r += 1);
    }

    /// Chi costruisce un componente lo dichiara pronto: le voci che ne dipendono diventano vive.
    pub fn set_feature(&self, feature: Feature, ready: bool) {
        let flag = match feature {
            Feature::UdpEmitter => &self.udp_emitter,
            Feature::AllTxt => &self.all_txt,
            Feature::AudioControl => &self.audio_control,
            Feature::Secrets => &self.secrets_ready,
            Feature::WindowControl => &self.window_control,
        };
        if flag.swap(ready, Ordering::SeqCst) != ready {
            self.bump();
        }
    }

    pub fn feature_ready(&self, feature: Feature) -> bool {
        match feature {
            Feature::UdpEmitter => self.udp_emitter.load(Ordering::SeqCst),
            Feature::AllTxt => self.all_txt.load(Ordering::SeqCst),
            Feature::AudioControl => self.audio_control.load(Ordering::SeqCst),
            Feature::Secrets => self.secrets_ready.load(Ordering::SeqCst),
            Feature::WindowControl => self.window_control.load(Ordering::SeqCst),
        }
    }

    /// Collega la protezione dei segreti (la password di eQSL): le voci di eQSL diventano vive se il sistema la offre.
    pub fn set_secrets(&self, secrets: Arc<dyn SecretStore>) {
        let available = secrets.available();
        *self.secrets.lock().unwrap_or_else(|p| p.into_inner()) = Some(secrets);
        self.set_feature(Feature::Secrets, available);
    }

    fn secret_store(&self) -> Option<Arc<dyn SecretStore>> {
        self.secrets.lock().unwrap_or_else(|p| p.into_inner()).clone()
    }

    /// C'e' una password di eQSL salvata (senza leggerla).
    pub fn eqsl_password_saved(&self) -> bool {
        self.secret_store().is_some_and(|s| s.has(reporting::EQSL_SECRET))
    }

    /// Lo stato degli invii: lo scrivono i servizi, lo legge l'API.
    pub fn status(&self) -> Arc<ReportStatus> {
        Arc::clone(&self.status)
    }

    /// Stato di una voce adesso: `Ok(apply)` se e' viva, `Err(motivo)` se non e' disponibile.
    pub fn state_of(&self, key: &str) -> Result<Apply, &'static str> {
        match support(key) {
            Support::Live(apply) => Ok(apply),
            Support::Needs(feature, _, apply) if self.feature_ready(feature) => Ok(apply),
            Support::Needs(_, reason, _) | Support::Hidden(reason) => Err(reason),
        }
    }

    /// Applica le modifiche: controlla tutto prima, poi scrive il file (copia di sicurezza e scrittura atomica) e avvisa chi si e' iscritto.
    /// Se qualcosa non va non cambia niente.
    pub fn update(&self, changes: &BTreeMap<String, String>) -> Result<(), SettingsError> {
        // le voci della scheda Audio passano da `update_audio`: aprono i dispositivi prima di salvare
        if let Some(key) = changes.keys().find(|k| AUDIO_KEYS.contains(&k.as_str())) {
            return Err(SettingsError::NotAvailable(key.clone()));
        }
        self.update_checked(changes)
    }

    /// Salva le voci della scheda Audio, dopo che il chiamante ha aperto i dispositivi (o scelto di tenere i vecchi).
    pub fn update_audio(&self, changes: &BTreeMap<String, String>) -> Result<(), SettingsError> {
        if let Some(key) = changes.keys().find(|k| !AUDIO_KEYS.contains(&k.as_str())) {
            return Err(SettingsError::NotAvailable(key.clone()));
        }
        self.update_checked(changes)
    }

    fn update_checked(&self, changes: &BTreeMap<String, String>) -> Result<(), SettingsError> {
        for (key, value) in changes {
            if self.state_of(key).is_err() {
                return Err(SettingsError::NotAvailable(key.clone()));
            }
            if SECRET_KEYS.contains(&key.as_str()) {
                if value.chars().count() > 100 || value.chars().any(char::is_control) {
                    return Err(SettingsError::BadValue(format!("{key}: valore non valido")));
                }
                continue;
            }
            validate(key, value)?;
        }
        let secret_change = changes.get("EQSLPasswd");
        let mut next = self.values();
        next.extend(changes.iter().filter(|(k, _)| !SECRET_KEYS.contains(&k.as_str())).map(|(k, v)| (k.clone(), v.clone())));
        // le caselle che in JTDX si escludono a vicenda (cliccare una toglie l'altra)
        if truthy(next.get("PromptToLog")) && truthy(next.get("AutoQSOLogging")) {
            return Err(SettingsError::BadValue("PromptToLog e AutoQSOLogging si escludono: ne va accesa una sola".into()));
        }
        if truthy(next.get("WriteDecodedALLTXT")) && truthy(next.get("WriteDecodedDebugALLTXT")) {
            return Err(SettingsError::BadValue("WriteDecodedALLTXT e WriteDecodedDebugALLTXT si escludono: ne va accesa una sola".into()));
        }
        // un invio acceso senza un indirizzo valido non avrebbe effetto: si rifiuta, non si finge
        let r = interpret(&next);
        if truthy(next.get("EnableTCPConnection")) && r.tcp.is_none() {
            return Err(SettingsError::BadValue("TCPServer: serve un indirizzo e una porta validi per accendere l'invio".into()));
        }
        if truthy(next.get("EQSLSend")) {
            let has_password = match secret_change {
                Some(v) => !v.is_empty(),
                None => self.eqsl_password_saved(),
            };
            if next.get("EQSLUser").is_none_or(String::is_empty) || !has_password {
                return Err(SettingsError::BadValue("EQSLSend: servono utente e password di eQSL per accendere l'invio".into()));
            }
        }
        // la password si salva nella protezione del sistema, mai nel file
        if let Some(value) = secret_change {
            let store = self.secret_store().ok_or_else(|| SettingsError::NotAvailable("EQSLPasswd".into()))?;
            store.set(reporting::EQSL_SECRET, value).map_err(SettingsError::Io)?;
        }
        if truthy(next.get("EnableUDP2adifBroadcast")) && r.secondary.is_none() {
            return Err(SettingsError::BadValue("UDP2Server: serve un indirizzo e una porta validi per accendere l'invio".into()));
        }
        if truthy(next.get("EnableUDP1adifSending")) && r.primary.is_none() {
            return Err(SettingsError::BadValue("UDPServer: serve un indirizzo e una porta validi per accendere l'invio".into()));
        }
        let previous = std::mem::replace(&mut *self.lock(), next);
        if let Err(e) = self.write() {
            *self.lock() = previous;
            return Err(e);
        }
        self.bump();
        Ok(())
    }

    fn write(&self) -> Result<(), SettingsError> {
        let body = FileBody { version: FILE_VERSION, values: self.values() };
        let text = serde_json::to_string_pretty(&body).map_err(|e| SettingsError::Io(e.to_string()))?;
        if let Some(dir) = self.path.parent().filter(|d| !d.as_os_str().is_empty()) {
            std::fs::create_dir_all(dir).map_err(|e| SettingsError::Io(e.to_string()))?;
        }
        // copia del file buono di prima, poi scrittura in un file accanto e rinomina: un'interruzione non lascia un file a meta'
        if self.path.is_file() {
            std::fs::copy(&self.path, backup_path(&self.path)).map_err(|e| SettingsError::Io(format!("copia di sicurezza: {e}")))?;
        }
        let tmp = self.path.with_extension("json.tmp");
        std::fs::write(&tmp, text).and_then(|()| std::fs::rename(&tmp, &self.path)).map_err(|e| SettingsError::Io(e.to_string()))
    }

    /// Il documento che l'API restituisce: valori, e per **ogni** voce di JTDX lo stato nel motore proprio.
    pub fn document(&self) -> Value {
        let mut items = serde_json::Map::new();
        for key in all_keys() {
            let item = match self.state_of(key) {
                Ok(apply) => json!({ "state": "live", "apply": if apply == Apply::Now { "now" } else { "restart" } }),
                Err(reason) => json!({ "state": "unavailable", "reason": reason }),
            };
            items.insert(key.to_string(), item);
        }
        let values: BTreeMap<String, String> = self.values().into_iter().filter(|(k, _)| self.state_of(k).is_ok()).collect();
        // una nota per le voci vive che per FT8 e FT4 non cambiano niente (come in JTDX)
        for (key, item) in &mut items {
            if let (Some(note), Some(obj)) = (note_of(key), item.as_object_mut()) {
                obj.insert("note".into(), json!(note));
            }
        }
        // i segreti non si restituiscono mai: si dice solo se ci sono
        let secrets: BTreeMap<&str, bool> = SECRET_KEYS.iter().map(|k| (*k, self.eqsl_password_saved())).collect();
        json!({ "version": FILE_VERSION, "file": FILE_NAME, "values": values, "items": items, "secrets": secrets, "status": self.status.snapshot() })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("aurora-engset-{name}-{}", std::process::id()));
        std::fs::remove_dir_all(&dir).ok();
        std::fs::create_dir_all(&dir).unwrap();
        dir.join(FILE_NAME)
    }

    fn map(pairs: &[(&str, &str)]) -> BTreeMap<String, String> {
        pairs.iter().map(|(k, v)| ((*k).to_string(), (*v).to_string())).collect()
    }

    fn addr(s: &str) -> SocketAddr {
        s.parse().unwrap()
    }

    /// Un `JTDX.ini` campione (solo valori inventati): primo server = hub di Aurora, secondo = log esterno, ALL.TXT acceso.
    const INI: &str = "[Configuration]\nMyCall=IU3ABC\nMyGrid=IO91wm\nUDPServer=127.0.0.1\nUDPServerPort=2334\nEnableUDP1adifSending=true\nUDP2Server=127.0.0.1\nUDP2ServerPort=2237\nEnableUDP2adifBroadcast=true\nWriteDecodedALLTXT=true\nPSKReporter=true\nEQSLPasswd=segreta\n";

    #[test]
    fn the_first_import_comes_from_the_ini_and_the_hub_becomes_the_forward_address() {
        let ini = engine_jtdx::ini::parse_shown(INI);
        let got = import_from_ini(&ini, addr("127.0.0.1:2334"), &[addr("127.0.0.1:2333")]);
        assert_eq!((got["MyCall"].as_str(), got["MyGrid"].as_str()), ("IU3ABC", "IO91wm"));
        assert_eq!((got["UDPServer"].as_str(), got["UDPServerPort"].as_str()), ("127.0.0.1", "2333"), "2334 e' Aurora stessa: il primo server diventa l'inoltro");
        assert_eq!((got["UDP2Server"].as_str(), got["UDP2ServerPort"].as_str(), got["EnableUDP2adifBroadcast"].as_str()), ("127.0.0.1", "2237", "true"));
        assert_eq!((got["EnableUDP1adifSending"].as_str(), got["WriteDecodedALLTXT"].as_str()), ("true", "true"));
        assert_eq!((got["PSKReporter"].as_str(), got["EQSLSend"].as_str()), ("true", "false"), "PSK Reporter acceso di base (e dal JTDX.ini); eQSL resta spento senza password");
        assert!(!got.contains_key("EQSLPasswd"), "la password non si importa mai");
    }

    #[test]
    fn an_external_primary_server_is_kept_and_no_ini_means_defaults() {
        let ini = engine_jtdx::ini::parse_shown("[Configuration]\nUDPServer=192.168.1.20\nUDPServerPort=2333\n");
        let got = import_from_ini(&ini, addr("127.0.0.1:2334"), &[]);
        assert_eq!((got["UDPServer"].as_str(), got["UDPServerPort"].as_str()), ("192.168.1.20", "2333"));
        let none = import_from_ini(&BTreeMap::new(), addr("127.0.0.1:2334"), &[]);
        assert_eq!(none, defaults());
        let r = interpret(&none);
        assert_eq!((r.primary, r.secondary, r.all_txt), (Some(addr("127.0.0.1:2237")), Some(addr("127.0.0.1:2237")), true), "come la schermata di JTDX");
        // il primo server di JTDX era Aurora stessa e non c'e' un inoltro: primo server vuoto e invio spento
        let own = import_from_ini(&engine_jtdx::ini::parse_shown(INI), addr("127.0.0.1:2334"), &[]);
        assert_eq!((own["UDPServer"].as_str(), own["EnableUDP1adifSending"].as_str()), ("", "false"));
    }

    #[test]
    fn a_missing_file_is_created_from_the_seed_once_and_then_the_ini_is_not_read_again() {
        let path = temp("seed");
        let first = EngineSettingsStore::open(&path, || import_from_ini(&engine_jtdx::ini::parse_shown(INI), addr("127.0.0.1:2334"), &[]));
        assert_eq!(first.values()["MyCall"], "IU3ABC");
        assert!(path.is_file(), "il file di Aurora e' scritto subito");
        first.update(&map(&[("MyCall", "IU3XYZ")])).unwrap();
        let second = EngineSettingsStore::open(&path, || panic!("il JTDX.ini si legge una volta sola"));
        assert_eq!(second.values()["MyCall"], "IU3XYZ", "vale quello salvato, non il JTDX.ini");
    }

    #[test]
    fn changes_are_validated_applied_saved_with_a_backup_and_announced() {
        let path = temp("update");
        let store = EngineSettingsStore::open(&path, defaults);
        let watcher = store.subscribe();
        store.update(&map(&[("UDP2Server", "192.168.1.9"), ("UDP2ServerPort", "2240"), ("EnableUDP2adifBroadcast", "true")])).unwrap();
        assert!(watcher.has_changed().unwrap(), "chi si e' iscritto viene avvisato");
        assert_eq!(store.reports().secondary, Some(addr("192.168.1.9:2240")));
        store.update(&map(&[("MyCall", "IU3ABC")])).unwrap();
        let backup: FileBody = serde_json::from_str(&std::fs::read_to_string(backup_path(&path)).unwrap()).unwrap();
        assert_eq!(backup.values["MyCall"], "", "la copia e' il file di prima");
        let saved: FileBody = serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        assert_eq!((saved.version, saved.values["MyCall"].as_str()), (FILE_VERSION, "IU3ABC"));
    }

    #[test]
    fn bad_values_and_unavailable_items_are_refused_and_nothing_changes() {
        let store = EngineSettingsStore::open(&temp("bad"), defaults);
        for (k, v) in [
            ("UDP2ServerPort", "0"),
            ("UDP2ServerPort", "70000"),
            ("UDP2Server", "non un indirizzo"),
            ("MyGrid", "XX"),
            ("MyCall", "A B"),
            ("EnableUDP2adifBroadcast", "si"),
            ("PSKReporter", "si"),
            ("TCPServerPort", "0"),
            ("NDepth", "3"),
            ("PromptToLog", "true"),
            ("EQSLSend", "true"),
        ] {
            assert!(store.update(&map(&[(k, v)])).is_err(), "{k}={v}");
        }
        // acceso senza un indirizzo valido: rifiutato anche se il componente e' pronto
        store.set_feature(Feature::UdpEmitter, true);
        store.update(&map(&[("UDPServer", ""), ("EnableUDP1adifSending", "false")])).unwrap();
        assert!(store.update(&map(&[("EnableUDP1adifSending", "true")])).is_err());
        let before = store.values();
        // un gruppo con una voce sbagliata non cambia nemmeno le altre
        assert!(store.update(&map(&[("MyCall", "IU3ABC"), ("UDP2ServerPort", "x")])).is_err());
        assert_eq!(store.values(), before);
    }

    #[test]
    fn items_wait_for_their_component_and_become_live_when_it_is_ready() {
        let store = EngineSettingsStore::open(&temp("feature"), defaults);
        assert_eq!(store.state_of("UDPServer"), Err("emitter_pending"));
        assert_eq!(store.state_of("WriteDecodedALLTXT"), Err("all_txt_pending"));
        assert_eq!(store.state_of("UDP2Server"), Ok(Apply::Now));
        assert_eq!(store.state_of("MyCall"), Ok(Apply::Restart));
        assert_eq!(store.state_of("PSKReporter"), Ok(Apply::Now));
        assert_eq!(store.state_of("EQSLUser"), Err("eqsl"));
        assert_eq!(store.state_of("udpWindowToFront"), Err("udp_window"));
        store.set_feature(Feature::UdpEmitter, true);
        store.update(&map(&[("UDPServer", "127.0.0.1"), ("UDPServerPort", "2333"), ("EnableUDP1adifSending", "true")])).unwrap();
        assert_eq!(store.reports().primary, Some(addr("127.0.0.1:2333")));
        assert!(store.reports().adif_to_primary);
    }

    #[test]
    fn a_corrupt_file_falls_back_to_the_backup_and_then_to_the_seed() {
        let path = temp("corrupt");
        let store = EngineSettingsStore::open(&path, defaults);
        store.update(&map(&[("MyCall", "IU3ABC")])).unwrap();
        store.update(&map(&[("MyGrid", "IO91wm")])).unwrap();
        std::fs::write(&path, "{ rotto").unwrap();
        let back = EngineSettingsStore::open(&path, defaults);
        assert_eq!(back.values()["MyCall"], "IU3ABC", "la copia di sicurezza e' l'ultimo file buono");
        std::fs::write(&path, "{ rotto").unwrap();
        std::fs::write(backup_path(&path), "anche questo").unwrap();
        let lost = EngineSettingsStore::open(&path, || map(&[("MyCall", "IU3SEME")]));
        assert_eq!(lost.values()["MyCall"], "IU3SEME");
    }

    #[test]
    fn the_document_lists_every_jtdx_item_with_its_state_and_only_live_values() {
        let store = EngineSettingsStore::open(&temp("doc"), defaults);
        let doc = store.document();
        assert_eq!(doc["items"].as_object().unwrap().len(), all_keys().len(), "ogni voce ha uno stato");
        assert_eq!(doc["items"]["EQSLPasswd"]["reason"], "eqsl");
        assert_eq!(doc["items"]["UDP2Server"]["state"], "live");
        assert_eq!(doc["items"]["MyCall"]["apply"], "restart");
        assert_eq!(doc["items"]["TCPServer"], json!({ "state": "live", "apply": "now" }));
        assert_eq!(doc["items"]["toRTTY"]["note"], "jt9_only");
        assert_eq!(doc["items"]["UDPServer"]["reason"], "emitter_pending");
        assert!(doc["values"].get("UDP2Server").is_some() && doc["values"].get("UDPServer").is_none() && doc["values"].get("PSKReporter").is_some());
    }

    #[test]
    fn every_jtdx_item_has_a_reason_or_is_live() {
        for key in all_keys() {
            if let Support::Hidden(reason) = support(key) {
                assert!(!reason.is_empty(), "{key}");
            }
        }
        assert!(all_keys().into_iter().filter(|k| is_stored(k)).count() == defaults().len() + SECRET_KEYS.len(), "le voci tenute nel file sono quelle dichiarate");
        for key in defaults().keys() {
            assert!(is_stored(key), "{key}: il valore iniziale esiste ma la voce non e' tenuta");
        }
    }

    #[test]
    fn anti_duplicate_rule_for_the_adif_datagram() {
        let r = |primary: Option<&str>, adif1: bool, secondary: Option<&str>| Reports { primary: primary.map(addr), adif_to_primary: adif1, secondary: secondary.map(addr), ..Reports::default() };
        let log = addr("127.0.0.1:2237");
        // ambiente e impostazioni uguali: una volta sola
        assert_eq!(export_udp_targets(&[log], &r(None, false, Some("127.0.0.1:2237"))), vec![log]);
        // due destinatari diversi: due invii
        assert_eq!(export_udp_targets(&[addr("127.0.0.1:2240")], &r(None, false, Some("127.0.0.1:2237"))), vec![addr("127.0.0.1:2240"), log]);
        // il secondo server e' il primo che riceve gia' l'ADIF (tipo 12): il datagramma semplice non parte
        assert_eq!(export_udp_targets(&[], &r(Some("127.0.0.1:2237"), true, Some("127.0.0.1:2237"))), Vec::<SocketAddr>::new());
        // stesso indirizzo ma il primo non manda l'ADIF: il datagramma parte
        assert_eq!(export_udp_targets(&[], &r(Some("127.0.0.1:2237"), false, Some("127.0.0.1:2237"))), vec![log]);
        // nessun secondo server: solo quelli fissi
        assert_eq!(export_udp_targets(&[log], &r(None, false, None)), vec![log]);
    }

    #[test]
    fn forward_targets_come_from_the_primary_server_and_keep_extra_env_addresses() {
        let reports = Reports { primary: Some(addr("127.0.0.1:2333")), ..Reports::default() };
        assert_eq!(forward_targets(&[addr("127.0.0.1:2333")], &reports), vec![addr("127.0.0.1:2333")], "il primo dell'ambiente e' solo il valore iniziale");
        assert_eq!(forward_targets(&[addr("127.0.0.1:2333"), addr("127.0.0.1:2400")], &reports), vec![addr("127.0.0.1:2333"), addr("127.0.0.1:2400")]);
        assert_eq!(forward_targets(&[addr("127.0.0.1:2333")], &Reports::default()), Vec::<SocketAddr>::new(), "primo server vuoto: nessun invio");
    }

    #[test]
    fn hosts_and_ports_are_parsed_strictly() {
        assert_eq!(parse_server("localhost", "2237"), Some(addr("127.0.0.1:2237")));
        assert_eq!(parse_server("::1", "2237"), Some(addr("[::1]:2237")));
        assert_eq!(parse_server("", "2237"), None);
        assert_eq!(parse_server("127.0.0.1", "0"), None);
        assert_eq!(parse_server("127.0.0.1", "65536"), None);
    }

    #[test]
    fn the_sequence_tab_has_live_counters_with_the_owner_values_as_default() {
        let r = interpret(&defaults());
        // di fabbrica: rapporto R e chiusura accesi a 5 (la configurazione che Aurora ha sempre avuto), gli altri spenti
        assert_eq!((r.seq_answer_cq, r.seq_answer_in_call, r.seq_sent_r_report, r.seq_closing), (None, None, Some(5), Some(5)));
        assert!(r.seq_halt_reply_other && !r.strict_directional_cq && !r.call_prio_cq && !r.max_distance);
        let prio = interpret(&map(&[("CallPriorityCQ", "true"), ("MaxDistance", "true")]));
        assert!(prio.call_prio_cq && prio.max_distance);
        let on = interpret(&map(&[
            ("SeqAnswerCQCount", "true"),
            ("SeqAnswerCQCounterValue", "2"),
            ("SeqAnswerInCallCount", "true"),
            ("SeqAnswerInCallCounterValue", "3"),
            ("SeqSentRReportCount", "false"),
            ("SeqSentRR7373Count", "true"),
            ("SeqSentRR7373CounterValue", "4"),
            ("SeqHaltTxReplyOther", "false"),
            ("StrictDirectionalCQ", "true"),
        ]));
        assert_eq!((on.seq_answer_cq, on.seq_answer_in_call, on.seq_sent_r_report, on.seq_closing), (Some(2), Some(3), None, Some(4)));
        assert!(!on.seq_halt_reply_other && on.strict_directional_cq);
        let store = EngineSettingsStore::open(&temp("seq"), defaults);
        for key in ["SeqAnswerCQCount", "SeqAnswerInCallCounterValue", "SeqHaltTxReplyOther", "StrictDirectionalCQ", "CallPriorityCQ", "MaxDistance", "TxWatchdogTimer", "TxQSYAllowed", "newDXCC", "newCallBandMode", "newPotential", "73RR73Marker", "callsignLogFiltering", "timeFromLogFiltering"] {
            assert_eq!(store.state_of(key), Ok(Apply::Now), "{key} e' viva");
        }
        assert_eq!(store.state_of("MonitorOFF"), Ok(Apply::Restart));
        for (key, why) in [("Region", "iaru_region"), ("Type2MsgGen", "type2_msgs"), ("Decode52", "decode52"), ("After73", "cw_id"), ("TuneTimer", "tune_timer"), ("MonitorLastUsed", "monitor_last"), ("workedStriked", "own_notify"), ("InsertBlank", "own_screen")] {
            assert_eq!(store.state_of(key), Err(why), "{key}");
        }
    }

    #[test]
    fn the_counters_the_watchdog_and_the_log_date_are_validated() {
        assert!(validate("SeqAnswerCQCounterValue", "5").is_ok());
        assert!(validate("SeqSentRReportCounterValue", "0").is_err());
        assert!(validate("SeqSentRR7373CounterValue", "6").is_err());
        assert!(validate("SeqAnswerInCallCounterValue", "x").is_err());
        assert!(validate("TxWatchdogTimer", "0").is_ok() && validate("TxWatchdogTimer", "99").is_ok());
        assert!(validate("TxWatchdogTimer", "100").is_err());
        assert!(validate("timeFromLogFiltering", "").is_ok());
        assert!(validate("timeFromLogFiltering", "20240315101500").is_ok());
        assert!(validate("timeFromLogFiltering", "2024-03-15").is_err());
        assert!(validate("timeFromLogFiltering", "202403151015001").is_err());
        // le voci che Aurora non tiene non si accettano
        assert!(matches!(validate("Decode52", "true"), Err(SettingsError::NotAvailable(_))));
    }

    #[test]
    fn the_notification_flags_become_jtdx_highlights_and_a_bad_value_changes_nothing() {
        let r = interpret(&defaults());
        assert_eq!(r.highlights, engine_jtdx::worked::HighlightFlags::jtdx_defaults(), "come i valori di fabbrica di JTDX");
        let store = EngineSettingsStore::open(&temp("flags"), defaults);
        let before = store.reports();
        store.update(&map(&[("newCQZ", "true"), ("newCQZBand", "true"), ("newDXCCBandMode", "false"), ("newPotential", "true")])).unwrap();
        let h = store.reports().highlights;
        assert!(h.cqz.on && h.cqz.band && !h.cqz.band_mode && h.potential);
        assert!(h.dxcc.on && h.dxcc.band && !h.dxcc.band_mode);
        // un valore non valido rifiuta tutto, anche le voci buone della stessa richiesta
        assert!(store.update(&map(&[("newGrid", "true"), ("SeqSentRReportCounterValue", "9")])).is_err());
        assert!(!store.reports().highlights.grid.on);
        assert_ne!(before.highlights, store.reports().highlights);
    }

    #[test]
    fn the_log_filter_and_the_general_options_reach_the_reports() {
        let store = EngineSettingsStore::open(&temp("filter"), defaults);
        assert!(store.reports().log_filter.is_off());
        store.update(&map(&[("MyCall", "IU3ABC"), ("MyGrid", "IO91wm"), ("callsignLogFiltering", "true"), ("gridLogFiltering", "true"), ("timeFromLogFiltering", "2024")])).unwrap();
        let f = store.reports().log_filter;
        assert_eq!((f.call.as_deref(), f.grid.as_deref(), f.from), (Some("IU3ABC"), Some("IO91wm"), Some(20_240_000_000_000)));
        // senza nominativo impostato il filtro per nominativo non puo' valere
        store.update(&map(&[("MyCall", "")])).unwrap();
        assert_eq!(store.reports().log_filter.call, None);
        store.update(&map(&[("TxWatchdogTimer", "9"), ("TxQSYAllowed", "false"), ("MonitorOFF", "true"), ("73RR73Marker", "false")])).unwrap();
        let r = store.reports();
        assert_eq!((r.watchdog_min, r.tx_qsy_allowed, r.monitor_off_at_start, r.rr73_marker), (9, false, true, false));
    }

    #[test]
    fn the_first_import_takes_the_sequence_and_notification_choices_from_the_ini() {
        let ini = engine_jtdx::ini::parse_shown("[Configuration]\nSeqAnswerCQCount=true\nSeqAnswerCQCounterValue=4\nnewCQZ=true\nnewCQZBand=false\nnewDXCCBandMode=false\nSeqHaltTxReplyOther=false\nTxQSYAllowed=true\nworkedStriked=true\nRegion=@Variant(\\0\\0)\n");
        let got = import_from_ini(&ini, addr("127.0.0.1:2334"), &[]);
        assert_eq!((got["SeqAnswerCQCount"].as_str(), got["SeqAnswerCQCounterValue"].as_str()), ("true", "4"));
        assert_eq!((got["newCQZ"].as_str(), got["newCQZBand"].as_str(), got["newDXCCBandMode"].as_str()), ("true", "false", "false"));
        assert_eq!(got["SeqHaltTxReplyOther"], "false");
        assert!(!got.contains_key("workedStriked") && !got.contains_key("Region"), "scelte di schermo e valori binari non si importano");
    }

    #[test]
    fn an_old_file_gets_each_missing_key_from_the_ini_and_keeps_the_keys_it_has() {
        let path = temp("oldfile");
        // un file scritto prima che esistessero le voci di Sequenza, Generali e Notifiche: solo alcune voci, una con un valore scelto dal proprietario
        let old = r#"{"version":1,"values":{"MyCall":"IU3ABC","MyGrid":"IO91wm","UDPServer":"192.168.1.20","UDPServerPort":"2333","EnableUDP1adifSending":"true","UDP2Server":"127.0.0.1","UDP2ServerPort":"2237","EnableUDP2adifBroadcast":"true","AcceptUDPRequests":"false","AcceptUDPReplyMessages":"1","WriteDecodedALLTXT":"true","WriteDecodedDebugALLTXT":"false"}}"#;
        std::fs::write(&path, old).unwrap();
        let ini = "[Configuration]
MyCall=IU3ZZZ
SeqHaltTxReplyOther=false
SeqAnswerCQCount=false
StrictDirectionalCQ=false
SeqSentRReportCounterValue=3
newCQZ=true
newDXCCBandMode=false
TxWatchdogTimer=9
";
        let store = EngineSettingsStore::open(&path, || import_from_ini(&engine_jtdx::ini::parse_shown(ini), addr("127.0.0.1:2334"), &[]));
        let v = store.values();
        // le voci nuove vengono dal JTDX.ini...
        assert_eq!(v["SeqHaltTxReplyOther"], "false", "il valore del JTDX.ini, non quello di fabbrica (acceso)");
        assert_eq!((v["SeqAnswerCQCount"].as_str(), v["StrictDirectionalCQ"].as_str(), v["SeqSentRReportCounterValue"].as_str()), ("false", "false", "3"));
        assert_eq!((v["newCQZ"].as_str(), v["newDXCCBandMode"].as_str(), v["TxWatchdogTimer"].as_str()), ("true", "false", "9"));
        // ...quelle che mancano anche dal JTDX.ini, dai valori di fabbrica
        assert_eq!(v["TxQSYAllowed"], "true");
        assert_eq!(v["newCall"], "true");
        // ...e le voci gia' nel file non si sovrascrivono mai
        assert_eq!((v["MyCall"].as_str(), v["UDPServer"].as_str()), ("IU3ABC", "192.168.1.20"));
        assert!(!store.reports().seq_halt_reply_other);
        // il file e' riscritto completo: al prossimo avvio il JTDX.ini non serve piu' e una modifica a mano non si perde
        store.update(&map(&[("SeqHaltTxReplyOther", "true")])).unwrap();
        let again = EngineSettingsStore::open(&path, || panic!("il file e' completo: il JTDX.ini non si legge"));
        assert_eq!(again.values()["SeqHaltTxReplyOther"], "true");
    }
    #[test]
    fn the_audio_entries_are_kept_validated_and_changed_only_by_the_audio_route() {
        let path = temp("audio");
        let store = EngineSettingsStore::open(&path, defaults);
        assert_eq!(store.audio(), AudioValues { input: "".into(), output: "".into(), input_channel: "Mono".into(), output_channel: "Mono".into(), save_dir: "".into(), save_mode: "0".into() });
        // finche' il componente audio non e' pronto le voci non sono vive
        assert!(store.state_of("SoundInName").is_err());
        store.set_feature(Feature::AudioControl, true);
        assert_eq!(store.state_of("AudioOutputChannel"), Ok(Apply::Now));
        // la rotta generica non le tocca: aprirebbe il file senza aprire i dispositivi
        assert!(matches!(store.update(&map(&[("SoundInName", "Altro")])), Err(SettingsError::NotAvailable(_))));
        store.update_audio(&map(&[("SoundInName", "Microfono (2- USB Audio Device)"), ("AudioInputChannel", "Right"), ("SaveWav", "1"), ("SaveDir", "D:/audio/ft8")])).expect("salvataggio");
        assert_eq!(store.audio().input, "Microfono (2- USB Audio Device)");
        assert_eq!((store.audio().input_channel.as_str(), store.audio().save_mode.as_str(), store.audio().save_dir.as_str()), ("Right", "1", "D:/audio/ft8"));
        // e la rotta audio non tocca le altre voci
        assert!(matches!(store.update_audio(&map(&[("MyCall", "IU3ABC")])), Err(SettingsError::NotAvailable(_))));
        // valori non ammessi: Entrambi non esiste in ingresso, il modo di salvataggio e' 0, 1 o 2, la cartella non ha caratteri di controllo
        for (k, v) in [("AudioInputChannel", "Both"), ("AudioOutputChannel", "Centro"), ("SaveWav", "3"), ("SaveDir", "D:/a|b"), ("SoundOutName", "uno
due")] {
            assert!(matches!(store.update_audio(&map(&[(k, v)])), Err(SettingsError::BadValue(_))), "{k}={v}");
        }
        assert_eq!(store.audio().input_channel, "Right", "un rifiuto non cambia niente");
        // restano al riavvio
        let again = EngineSettingsStore::open(&path, defaults);
        assert_eq!(again.audio(), store.audio());
        let doc = again.document();
        assert_eq!(doc["items"]["SoundInName"]["state"], "unavailable", "senza componente audio pronto");
    }

    #[test]
    fn the_audio_entries_come_from_the_ini_once() {
        let ini = engine_jtdx::ini::parse_shown("[Configuration]
SoundInName=Microfono (2- USB Audio Device)
SoundOutName=Altoparlanti (2- USB Audio Device)
AudioInputChannel=Left
AudioOutputChannel=Both
SaveDir=C:/Users/x/save
SaveWav=2
");
        let got = import_from_ini(&ini, addr("127.0.0.1:2334"), &[]);
        assert_eq!((got["SoundInName"].as_str(), got["SoundOutName"].as_str()), ("Microfono (2- USB Audio Device)", "Altoparlanti (2- USB Audio Device)"));
        assert_eq!((got["AudioInputChannel"].as_str(), got["AudioOutputChannel"].as_str(), got["SaveWav"].as_str(), got["SaveDir"].as_str()), ("Left", "Both", "2", "C:/Users/x/save"));
    }
}
