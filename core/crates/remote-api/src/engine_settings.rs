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
        Mutex,
    },
};

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

/// Voci della scheda Segnalazioni di JTDX che non sono in `engine_jtdx::ini::FIELDS` (non si leggono ne' si scrivono nel `JTDX.ini`): in Aurora
/// si vedono, spente e con la spiegazione. Le credenziali di eQSL non si tengono mai.
/// `Region` e `Type2MsgGen` stanno nel `JTDX.ini` come valori binari (`@Variant`): non si leggono ne' si scrivono, ma la scheda Generali li mostra.
pub const EXTRA_KEYS: &[&str] = &["toRTTY", "dBtoComments", "distanceToComments", "ClearCallGridExit", "EQSLSend", "EQSLUser", "EQSLPasswd", "EQSLNick", "Region", "Type2MsgGen"];

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
        | "SeqSentRR7373Count" | "SeqSentRR7373CounterValue" | "SeqHaltTxReplyOther" | "StrictDirectionalCQ" => Support::Live(Apply::Now),
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
        "newPotential" | "workedColor" | "workedStriked" | "workedUnderlined" | "workedDontShow" | "txtColor" | "OtherStandardMessagesMarker" | "73RR73Marker" | "redMarker"
        | "blueMarker" | "hidehintMarker" => Support::Live(Apply::Now),
        "udpWindowToFront" | "udpWindowRestore" => Support::Hidden("udp_window"),
        "TCPServer" | "TCPServerPort" | "EnableTCPConnection" => Support::Hidden("tcp"),
        "ApplyFiltersToUDPmessages" => Support::Hidden("udp_filters"),
        "preventFalseUDPspots" => Support::Hidden("false_spots"),
        "PSKReporter" => Support::Hidden("psk"),
        "AllowSpotsDXSummit" => Support::Hidden("dxsummit"),
        "EQSLSend" | "EQSLUser" | "EQSLPasswd" | "EQSLNick" => Support::Hidden("eqsl"),
        "PromptToLog" => Support::Hidden("log_prompt"),
        "AutoQSOLogging" => Support::Hidden("log_auto"),
        "toRTTY" | "dBtoComments" | "distanceToComments" => Support::Hidden("adif_options"),
        "ClearCallGrid" | "ClearCallGridExit" => Support::Hidden("clear_dx"),
        "Language" | "Miles" | "showDistance" | "countryName" | "UseDarkStyle" => Support::Hidden("own_screen"),
        "Rig" | "PTTport" | "CATSerialPort" | "CATSerialRate" | "Polling" | "CATForceDTR" | "CATForceRTS" | "TCIAudio" => Support::Hidden("radio_cat"),
        "SoundInName" | "SoundOutName" | "AudioInputChannel" | "AudioOutputChannel" => Support::Hidden("audio_start"),
        "SaveDir" | "SaveWav" => Support::Hidden("no_wav_save"),
        "AutoSequence" | "SingleShotQSO" | "SkipTx1" | "AnswerWorkedB4" | "CallPriorityCQ" | "MaxDistance" => Support::Hidden("own_rules"),
        k if k.starts_with("AlertOn") || k.starts_with("BeepOn") => Support::Hidden("own_alerts"),
        _ => Support::Hidden("own_decoder"),
    }
}

/// Le voci che il motore proprio tiene davvero nel file (vive o in attesa di un componente).
pub fn is_stored(key: &str) -> bool {
    matches!(support(key), Support::Live(_) | Support::Needs(..))
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
        ("workedColor", "false"),
        ("workedStriked", "true"),
        ("workedUnderlined", "false"),
        ("workedDontShow", "false"),
        ("txtColor", "false"),
        ("OtherStandardMessagesMarker", "true"),
        ("73RR73Marker", "true"),
        ("redMarker", "true"),
        ("blueMarker", "false"),
        ("hidehintMarker", "false"),
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
    if value.is_empty() && matches!(key, "MyCall" | "MyGrid" | "UDPServer" | "UDP2Server" | "timeFromLogFiltering") {
        return Ok(());
    }
    engine_jtdx::ini_edit::check_value(key, value).map_err(SettingsError::BadValue)?;
    match key {
        "UDPServer" | "UDP2Server" if parse_host(value).is_none() => Err(SettingsError::BadValue(format!("{key}: serve un indirizzo IP (o localhost)"))),
        "UDPServerPort" | "UDP2ServerPort" if parse_server("127.0.0.1", value).is_none() => Err(SettingsError::BadValue(format!("{key}: la porta va da 1 a 65535"))),
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
        let fresh = loaded.is_none();
        let mut values = defaults();
        for (k, v) in loaded.unwrap_or_else(seed) {
            // un valore non piu' valido o di una voce che non si tiene piu' si scarta: resta quello iniziale
            if validate(&k, &v).is_ok() {
                values.insert(k, v);
            }
        }
        let (revision, _) = watch::channel(0);
        let store = EngineSettingsStore { path: path.to_path_buf(), values: Mutex::new(values), revision, udp_emitter: AtomicBool::new(false), all_txt: AtomicBool::new(false) };
        if fresh && let Err(e) = store.write() {
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
        };
        if flag.swap(ready, Ordering::SeqCst) != ready {
            self.bump();
        }
    }

    pub fn feature_ready(&self, feature: Feature) -> bool {
        match feature {
            Feature::UdpEmitter => self.udp_emitter.load(Ordering::SeqCst),
            Feature::AllTxt => self.all_txt.load(Ordering::SeqCst),
        }
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
        for (key, value) in changes {
            if self.state_of(key).is_err() {
                return Err(SettingsError::NotAvailable(key.clone()));
            }
            validate(key, value)?;
        }
        let mut next = self.values();
        next.extend(changes.iter().map(|(k, v)| (k.clone(), v.clone())));
        // un invio acceso senza un indirizzo valido non avrebbe effetto: si rifiuta, non si finge
        let r = interpret(&next);
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
        json!({ "version": FILE_VERSION, "file": FILE_NAME, "values": values, "items": items })
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
        assert!(!got.contains_key("PSKReporter") && !got.contains_key("EQSLPasswd"), "solo le voci che il motore proprio tiene");
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
            ("PSKReporter", "true"),
            ("TCPServerPort", "4000"),
            ("NDepth", "3"),
            ("WriteDecodedALLTXT", "true"),
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
        assert_eq!(store.state_of("PSKReporter"), Err("psk"));
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
        assert_eq!(doc["items"]["TCPServer"], json!({ "state": "unavailable", "reason": "tcp" }));
        assert_eq!(doc["items"]["UDPServer"]["reason"], "emitter_pending");
        assert!(doc["values"].get("UDP2Server").is_some() && doc["values"].get("UDPServer").is_none() && doc["values"].get("PSKReporter").is_none());
    }

    #[test]
    fn every_jtdx_item_has_a_reason_or_is_live() {
        for key in all_keys() {
            if let Support::Hidden(reason) = support(key) {
                assert!(!reason.is_empty(), "{key}");
            }
        }
        assert!(all_keys().into_iter().filter(|k| is_stored(k)).count() == defaults().len(), "le voci tenute nel file sono quelle dichiarate");
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
}
