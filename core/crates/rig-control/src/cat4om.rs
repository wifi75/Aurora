//! Cat4OM: il servizio di rete di Log4OM che tiene le radio e le divide tra piu' programmi (Log4OM, Decodium, Aurora...). Aurora si collega
//! al Control Port del gruppo come client nativo, come fa Decodium (`DecodiumCat4OmManager.cpp`): cosi' Log4OM e Aurora comandano la stessa
//! radio insieme. Con l'emulazione Hamlib di Cat4OM, invece, la sessione e' esclusiva e Log4OM non poteva piu' fare QSY dagli spot.
//!
//! Il protocollo e' quello del manuale (*CAT4OM Integration Manual*, protocollo `1.0.0`), riassunto in `docs/cat4om-protocollo.md`:
//! - WebSocket di testo, JSON in lower camel case. Primo messaggio `hello` (con `auth` `cat4om-sha256-minute-v1` se il gruppo ha la
//!   chiave), risposta `welcome` con `clientId`, `role` e lo stato completo delle radio (`radios[]`), oppure `error` e chiusura.
//! - richieste `{type:"request", id, clientId, action, radioId, params}`, risposte `{type:"response", id, success, result|error}` correlate
//!   per `id` (possono arrivare fuori ordine).
//! - modello **push**: frequenza, modo, PTT e split arrivano negli `stateUpdate` (anche quelli fatti da Log4OM); niente lettura periodica.
//!   `getAllState` solo quando il contatore `seq` salta. Il keepalive del server (`stateUpdate` vuoto ogni `keepaliveIntervalMs`) dice che il
//!   collegamento e' vivo: 3 intervalli senza nulla = collegamento perso.
//! - un solo **master** per gruppo comanda (il primo collegato, o chi chiede `getOwnership`). Aurora non lo prende al collegamento: lo chiede
//!   solo quando deve scrivere (frequenza, modo, PTT, split), cosi' Log4OM resta libero di fare QSY e riprende il comando quando gli serve.
//! - split vero con `setFrequency` sul VFO di trasmissione e `setSplit`, se la radio dichiara `SetSplit` e un secondo VFO; altrimenti lo split
//!   si simula (la frequenza si sposta solo durante la trasmissione, «Fake It»).
//! - scoperta come Decodium: la porta Control di un gruppo la assegna Cat4OM (per esempio 5001), quindi Aurora chiede alla porta di gestione
//!   (`127.0.0.1:17300`, `getServiceStatus`) i gruppi con la loro porta Control, sceglie il gruppo (per nome o il primo), lo avvia se e' fermo
//!   (`startGroup`) e si collega al suo Control. Ogni riconnessione rifa' la scoperta: al riavvio di Cat4OM la porta puo' cambiare. Una porta
//!   Control scritta a mano salta la scoperta.
//!
//! La chiave del gruppo non si scrive mai nei registri: va solo nella prova SHA-256 dell'`hello`.

use std::{
    collections::{HashMap, VecDeque},
    io::ErrorKind,
    net::{TcpStream, ToSocketAddrs},
    sync::{
        Arc, Condvar, Mutex,
        atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering},
    },
    thread::JoinHandle,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use serde::Serialize;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use tungstenite::{Message, WebSocket};

use crate::{
    Rig, RigError,
    cat_config::{CatMode, RigConfig, SplitMode, split_tx_dial_hz},
    hamlib_rig::{LinkState, RigStatus, valid_dial_hz},
    meters::{self, Level, Meters},
};

/// La porta di gestione di Cat4OM (una per servizio): da li' si leggono i gruppi e la porta Control di ciascuno.
pub const DEFAULT_CAT4OM_MANAGEMENT: &str = "127.0.0.1:17300";
/// La porta Control che Aurora proponeva fino alla 0.11.237: in una configurazione vecchia vale «non scritta» e la porta vera si scopre
/// (nelle installazioni vere Cat4OM assegna al gruppo un'altra porta, per esempio 5001).
pub const LEGACY_CAT4OM_ADDRESS: &str = "127.0.0.1:17301";
/// La versione del protocollo parlata da Aurora: conta solo il numero principale.
pub const PROTOCOL_VERSION: &str = "1.0.0";
/// Lo schema di autenticazione del manuale (§2.2).
pub const AUTH_SCHEME: &str = "cat4om-sha256-minute-v1";

// ---------------------------------------------------------------- messaggi

/// Messaggi dello stato (tradotti dall'interfaccia, `coreText`).
const NOT_LINKED: &str = "radio Cat4OM non collegata";
const NO_RADIOS: &str = "Nessuna radio nel gruppo Cat4OM: aggiungila in Cat4OM";
const AUTH_REQUIRED: &str = "Cat4OM chiede la chiave del gruppo: scrivila in Impostazioni → Radio";
const AUTH_FAILED: &str = "Cat4OM ha rifiutato la chiave del gruppo: controlla la chiave e l'ora del PC";
const VERSION_MISMATCH: &str = "Versione del protocollo Cat4OM non compatibile con Aurora (1.0.0)";
const MANAGEMENT_PORT: &str = "Nel campo «Controllo» c'e' la porta di gestione di Cat4OM: lascialo vuoto, Aurora trova da sola la porta del gruppo";
const MANAGEMENT_AUTH: &str = "La gestione di Cat4OM chiede una password: scrivi la porta Control del gruppo nel campo «Controllo»";
const NO_GROUPS: &str = "Nessun gruppo in Cat4OM: crea in Cat4OM un gruppo con la radio";

fn not_answering(address: &str, raw: &str) -> String {
    format!("Cat4OM non raggiungibile su {address}: avvia Cat4OM e controlla la porta Control del gruppo; da un altro PC abilita la rete in Settings → SERVER ({raw})")
}

fn not_control(address: &str) -> String {
    format!("Su {address} non risponde il Control di Cat4OM: controlla la porta del gruppo o lascia vuoto il campo «Controllo»")
}

fn management_unreachable(address: &str, raw: &str) -> String {
    format!("Gestione di Cat4OM non raggiungibile su {address}: avvia Log4OM e Cat4OM; da un altro PC abilita la rete in Settings → SERVER ({raw})")
}

fn not_management(address: &str) -> String {
    format!("Su {address} non risponde la gestione di Cat4OM: controlla la porta di gestione (di base 17300)")
}

fn group_missing(group: &str, list: &str) -> String {
    format!("Gruppo «{group}» non trovato in Cat4OM (presenti: {list})")
}

fn group_stopped(group: &str) -> String {
    format!("Il gruppo «{group}» di Cat4OM e' fermo: avvialo in Cat4OM")
}

fn timeout_text(action: &str, timeout: Duration) -> String {
    format!("Cat4OM non risponde a {action} (nessuna risposta in {} s)", timeout.as_secs())
}

fn refused_text(action: &str, e: &ServerError) -> String {
    format!("Cat4OM ha rifiutato {action}: {} ({})", e.message, e.code)
}

fn claimed(name: &str, by: &str) -> String {
    format!("La radio «{name}» e' presa dall'emulazione {by} di Cat4OM: spegni quel profilo in Cat4OM per comandarla da Aurora")
}

// ---------------------------------------------------------------- costruzione

/// Base64 standard (RFC 4648, con `=`), per la prova di autenticazione.
pub fn base64(data: &[u8]) -> String {
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(data.len().div_ceil(3) * 4);
    for chunk in data.chunks(3) {
        let b1 = chunk.get(1).copied().unwrap_or(0);
        let b2 = chunk.get(2).copied().unwrap_or(0);
        let n = (u32::from(chunk[0]) << 16) | (u32::from(b1) << 8) | u32::from(b2);
        out.push(char::from(TABLE[(n >> 18 & 63) as usize]));
        out.push(char::from(TABLE[(n >> 12 & 63) as usize]));
        out.push(if chunk.len() > 1 { char::from(TABLE[(n >> 6 & 63) as usize]) } else { '=' });
        out.push(if chunk.len() > 2 { char::from(TABLE[(n & 63) as usize]) } else { '=' });
    }
    out
}

/// Giorno civile (anno, mese, giorno) da giorni dal 1970-01-01 (algoritmo di H. Hinnant).
fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    let y = yoe + era * 400;
    (if m <= 2 { y + 1 } else { y }, m, d)
}

/// L'ora UTC al minuto `yyyyMMddHHmm` (`timestampUtc` dell'autenticazione).
pub fn utc_minute(now: SystemTime) -> String {
    let secs = now.duration_since(UNIX_EPOCH).map_or(0, |d| d.as_secs()) as i64;
    let (days, rem) = (secs.div_euclid(86_400), secs.rem_euclid(86_400));
    let (y, m, d) = civil_from_days(days);
    format!("{y:04}{m:02}{d:02}{:02}{:02}", rem / 3600, rem % 3600 / 60)
}

/// `proof` = Base64(SHA-256(UTF-8(chiave + "\n" + timestampUtc))).
pub fn auth_proof(key: &str, timestamp: &str) -> String {
    let digest: Vec<u8> = Sha256::digest(format!("{key}\n{timestamp}").as_bytes()).iter().copied().collect();
    base64(&digest)
}

/// Il primo messaggio: `hello` con nome e versione di Aurora e, se c'e' la chiave del gruppo, la prova di autenticazione (mai la chiave).
pub fn hello_message(app_version: &str, key: &str, now: SystemTime) -> Value {
    let mut hello = json!({ "type": "hello", "protocolVersion": PROTOCOL_VERSION, "appName": "Aurora", "appVersion": app_version });
    if !key.is_empty() {
        let ts = utc_minute(now);
        hello["auth"] = json!({ "scheme": AUTH_SCHEME, "timestampUtc": ts, "proof": auth_proof(key, &ts) });
    }
    hello
}

/// Una richiesta sul Control Port.
pub fn request_message(id: &str, client_id: &str, action: &str, radio_id: Option<&str>, params: Value) -> String {
    let mut request = json!({ "type": "request", "id": id, "clientId": client_id, "action": action, "params": params });
    if let Some(radio) = radio_id {
        request["radioId"] = json!(radio);
    }
    request.to_string()
}

static REQUEST_COUNTER: AtomicU64 = AtomicU64::new(1);

/// Un identificativo di richiesta con la forma di un UUID, mai riusato nello stesso processo (contatore, ora e processo).
pub fn new_request_id() -> String {
    let n = REQUEST_COUNTER.fetch_add(1, Ordering::Relaxed);
    let nanos = SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |d| d.as_nanos() as u64);
    let pid = u64::from(std::process::id());
    format!("{:08x}-{:04x}-4{:03x}-8{:03x}-{:012x}", nanos as u32, (nanos >> 32) as u16, pid & 0xfff, (n >> 48) & 0xfff, n & 0xffff_ffff_ffff)
}

/// Stesso numero principale del protocollo (il manuale: si confronta solo il major).
pub fn same_major(version: &str) -> bool {
    version.trim().split('.').next() == PROTOCOL_VERSION.split('.').next()
}

// ---------------------------------------------------------------- lettura

/// Un errore del server (`error` dell'handshake o di una risposta).
#[derive(Debug, Clone, PartialEq)]
pub struct ServerError {
    pub code: String,
    pub message: String,
    /// Solo sugli errori temporanei (`OWNERSHIP_COOLDOWN`).
    pub retry_after_ms: Option<u64>,
    pub details: Value,
}

impl ServerError {
    fn from_value(v: &Value) -> ServerError {
        ServerError {
            code: v.get("code").and_then(Value::as_str).unwrap_or("UNKNOWN").to_string(),
            message: v.get("message").and_then(Value::as_str).unwrap_or_default().to_string(),
            retry_after_ms: v.get("retryAfterMs").and_then(Value::as_u64),
            details: v.get("details").cloned().unwrap_or(Value::Null),
        }
    }
}

/// Il `welcome` del Control Port.
#[derive(Debug, Clone, PartialEq)]
pub struct Welcome {
    pub endpoint: String,
    pub protocol_version: String,
    pub client_id: String,
    pub group_id: String,
    pub role: String,
    pub radios: Vec<Value>,
    pub seq: Option<u64>,
    pub instance_id: String,
    pub keepalive_ms: Option<u64>,
    pub features: Vec<String>,
}

/// Un messaggio del server, gia' letto. I tipi sconosciuti si ignorano (regole di compatibilita' del manuale).
#[derive(Debug, Clone, PartialEq)]
pub enum Incoming {
    Welcome(Welcome),
    /// `managementWelcome`: la porta e' quella di gestione, non il Control di un gruppo.
    ManagementWelcome { client_id: String, protocol_version: String },
    StateUpdate { seq: Option<u64>, full: bool, master_id: Option<String>, radios: Vec<Value> },
    Response { id: String, success: bool, status: Option<String>, result: Value, error: Option<ServerError> },
    Event { event: String, radio_id: Option<String>, details: Value },
    Error(ServerError),
    Other(String),
}

fn strings(v: Option<&Value>) -> Vec<String> {
    v.and_then(Value::as_array).map(|a| a.iter().filter_map(Value::as_str).map(str::to_string).collect()).unwrap_or_default()
}

fn text(v: &Value, key: &str) -> Option<String> {
    v.get(key).and_then(Value::as_str).map(str::to_string)
}

/// Legge un messaggio di testo del server; `None` se non e' JSON o non ha `type`.
pub fn parse_message(raw: &str) -> Option<Incoming> {
    let v: Value = serde_json::from_str(raw).ok()?;
    let kind = v.get("type")?.as_str()?;
    let radios = || v.get("radios").and_then(Value::as_array).cloned().unwrap_or_default();
    Some(match kind {
        "welcome" => Incoming::Welcome(Welcome {
            endpoint: text(&v, "endpoint").unwrap_or_default(),
            protocol_version: text(&v, "protocolVersion").unwrap_or_default(),
            client_id: text(&v, "clientId").unwrap_or_default(),
            group_id: text(&v, "groupId").unwrap_or_default(),
            role: text(&v, "role").unwrap_or_default(),
            radios: radios(),
            seq: v.get("seq").and_then(Value::as_u64),
            instance_id: text(&v, "instanceId").unwrap_or_default(),
            keepalive_ms: v.get("keepaliveIntervalMs").and_then(Value::as_u64),
            features: strings(v.get("features")),
        }),
        "managementWelcome" => Incoming::ManagementWelcome {
            client_id: text(&v, "clientId").unwrap_or_default(),
            protocol_version: text(&v, "protocolVersion").unwrap_or_default(),
        },
        "stateUpdate" => Incoming::StateUpdate {
            seq: v.get("seq").and_then(Value::as_u64),
            // `full` assente = stato completo
            full: v.get("full").and_then(Value::as_bool).unwrap_or(true),
            master_id: text(&v, "masterId"),
            radios: radios(),
        },
        "response" => Incoming::Response {
            id: text(&v, "id").unwrap_or_default(),
            success: v.get("success").and_then(Value::as_bool).unwrap_or(false),
            status: text(&v, "status"),
            result: v.get("result").cloned().unwrap_or(Value::Null),
            error: v.get("error").filter(|e| e.is_object()).map(ServerError::from_value),
        },
        "event" => Incoming::Event {
            event: text(&v, "event").unwrap_or_default(),
            radio_id: text(&v, "radioId"),
            details: v.get("details").cloned().unwrap_or(Value::Null),
        },
        "error" => Incoming::Error(ServerError::from_value(&v)),
        other => Incoming::Other(other.to_string()),
    })
}

/// Lo stato di una radio (`RadioState`) per quel che serve ad Aurora. Un campo assente resta `None` («non pubblicato»).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct RadioView {
    pub id: String,
    pub name: String,
    pub model: String,
    pub connection_status: String,
    pub active_vfo: Option<String>,
    pub tx_vfo: Option<String>,
    pub split: Option<bool>,
    pub ptt: Option<bool>,
    /// La frequenza e il modo del VFO attivo.
    pub frequency_hz: Option<u64>,
    pub mode: Option<String>,
    /// Ogni VFO con frequenza e modo.
    pub vfos: Vec<(String, Option<u64>, Option<String>)>,
    pub available_commands: Vec<String>,
    pub supported_modes: Vec<String>,
    pub available_vfos: Vec<String>,
    /// `commandVfoScopes.SetFrequency == "activeOnly"`: la frequenza si scrive solo sul VFO attivo.
    pub frequency_active_only: bool,
    /// Il profilo di emulazione (TCI, Hamlib) che tiene la radio: i comandi nativi sono rifiutati.
    pub bridge_controlled_by: Option<String>,
    pub metering: Value,
}

impl RadioView {
    pub fn from_state(v: &Value) -> Option<RadioView> {
        let id = text(v, "radioId")?;
        let vfos: Vec<(String, Option<u64>, Option<String>)> = v
            .get("vfos")
            .and_then(Value::as_object)
            .map(|m| m.iter().map(|(name, vfo)| (name.clone(), vfo.get("frequency").and_then(Value::as_u64), text(vfo, "mode"))).collect())
            .unwrap_or_default();
        let active_vfo = text(v, "activeVfo");
        // il VFO attivo; se il server non lo dice, «A» o il primo
        let active = active_vfo
            .as_deref()
            .and_then(|a| vfos.iter().find(|(n, _, _)| n == a))
            .or_else(|| vfos.iter().find(|(n, _, _)| n.eq_ignore_ascii_case("A")))
            .or_else(|| vfos.first());
        let frequency_active_only = v
            .get("commandVfoScopes")
            .and_then(Value::as_object)
            .and_then(|m| m.iter().find(|(k, _)| k.eq_ignore_ascii_case("SetFrequency")).map(|(_, s)| s.as_str() == Some("activeOnly")))
            .unwrap_or(false);
        let mut available_vfos = strings(v.get("availableVfos"));
        if available_vfos.is_empty() {
            available_vfos = vfos.iter().map(|(n, _, _)| n.clone()).collect();
        }
        Some(RadioView {
            name: text(v, "radioName").unwrap_or_else(|| id.clone()),
            model: text(v, "model").unwrap_or_default(),
            connection_status: text(v, "connectionStatus").unwrap_or_else(|| "unknown".into()),
            tx_vfo: text(v, "txVfo"),
            split: v.get("split").and_then(Value::as_bool),
            ptt: v.get("ptt").and_then(Value::as_bool),
            frequency_hz: active.and_then(|(_, f, _)| *f),
            mode: active.and_then(|(_, _, m)| m.clone()),
            available_commands: strings(v.get("availableCommands")),
            supported_modes: strings(v.get("supportedModes")),
            available_vfos,
            frequency_active_only,
            bridge_controlled_by: text(v, "bridgeControlledBy").filter(|b| !b.trim().is_empty()),
            metering: v.get("metering").cloned().unwrap_or(Value::Null),
            active_vfo,
            vfos,
            id,
        })
    }

    /// La radio dichiara il comando (`availableCommands`, confronto senza maiuscole).
    pub fn has_command(&self, name: &str) -> bool {
        self.available_commands.iter().any(|c| c.eq_ignore_ascii_case(name))
    }

    pub fn vfo_frequency(&self, vfo: &str) -> Option<u64> {
        self.vfos.iter().find(|(n, _, _)| n == vfo).and_then(|(_, f, _)| *f)
    }

    /// Il VFO per la trasmissione in split: il primo dei VFO disponibili diverso da quello attivo.
    pub fn other_vfo(&self) -> Option<String> {
        let active = self.active_vfo.clone().or_else(|| self.available_vfos.first().cloned())?;
        self.available_vfos.iter().find(|v| **v != active).cloned()
    }

    /// Lo split vero si puo' fare: `SetSplit`, un secondo VFO e la frequenza scrivibile anche sul VFO non attivo.
    pub fn real_split(&self) -> bool {
        self.has_command("SetSplit") && !self.frequency_active_only && self.other_vfo().is_some()
    }
}

/// Le letture dei misuratori (`metering` o il blocco di `signalUpdate`): S-meter (dBm → dB rispetto a S9, S9 = -73 dBm), potenza, ROS.
pub fn meter_readings(metering: &Value) -> Vec<(Level, f32)> {
    let num = |k: &str| metering.get(k).and_then(Value::as_f64).map(|v| v as f32).filter(|v| v.is_finite());
    let mut out = Vec::new();
    if let Some(dbm) = num("signalDbm") {
        out.push((Level::Strength, dbm + 73.0));
    }
    if let Some(w) = num("powerWatts") {
        out.push((Level::PowerWatts, w));
    }
    if let Some(swr) = num("swr") {
        out.push((Level::Swr, swr));
    }
    out
}

/// Il modo da scrivere per il modo CAT scelto: USB, oppure USB dati col nome che la radio dichiara (`DATA-USB` di base). `None` = non si tocca.
pub fn cat4om_mode(mode: CatMode, supported: &[String]) -> Option<String> {
    let has = |m: &str| supported.iter().any(|s| s.eq_ignore_ascii_case(m));
    match mode {
        CatMode::None => None,
        CatMode::Usb => Some("USB".into()),
        CatMode::Data => {
            if supported.is_empty() {
                return Some("DATA-USB".into());
            }
            ["DATA-USB", "PKTUSB", "USB-D", "DIGU"].iter().find(|m| has(m)).map(|m| m.to_string()).or_else(|| Some(if has("USB") { "USB" } else { "DATA-USB" }.to_string()))
        }
    }
}

/// Il modo letto va gia' bene per quello voluto: uguale, oppure un altro nome dell'USB dati.
pub fn mode_satisfied(wanted: &str, current: &str) -> bool {
    const DATA: [&str; 7] = ["DATA-USB", "PKTUSB", "USB-D", "USB-D1", "DIGU", "DATA-U", "USBD"];
    let (w, c) = (wanted.trim().to_ascii_uppercase(), current.trim().to_ascii_uppercase());
    w == c || (DATA.contains(&w.as_str()) && DATA.contains(&c.as_str()))
}

/// Sceglie la radio: quella scritta (per `radioId` o per nome), altrimenti la prima collegata o la prima. L'errore e' il testo per l'utente.
pub fn choose_radio(wanted: &str, radios: &[Value]) -> Result<String, String> {
    let views: Vec<RadioView> = radios.iter().filter_map(RadioView::from_state).collect();
    if views.is_empty() {
        return Err(NO_RADIOS.into());
    }
    let wanted = wanted.trim();
    if !wanted.is_empty() {
        return views
            .iter()
            .find(|r| r.id == wanted)
            .or_else(|| views.iter().find(|r| r.id.eq_ignore_ascii_case(wanted) || r.name.eq_ignore_ascii_case(wanted)))
            .map(|r| r.id.clone())
            .ok_or_else(|| format!("Radio «{wanted}» non trovata nel gruppo Cat4OM (presenti: {})", views.iter().map(|r| r.id.as_str()).collect::<Vec<_>>().join(", ")));
    }
    Ok(views.iter().find(|r| r.connection_status == "connected").unwrap_or(&views[0]).id.clone())
}

/// Applica le radio di uno `stateUpdate`: `full` sostituisce, altrimenti si fondono i campi per `radioId`.
pub fn merge_radios(current: &mut Vec<Value>, update: Vec<Value>, full: bool) {
    if full {
        *current = update;
        return;
    }
    for radio in update {
        let id = radio.get("radioId").and_then(Value::as_str).map(str::to_string);
        match current.iter_mut().find(|r| id.is_some() && r.get("radioId").and_then(Value::as_str) == id.as_deref()) {
            Some(old) => {
                if let (Some(old), Some(new)) = (old.as_object_mut(), radio.as_object()) {
                    for (k, v) in new {
                        old.insert(k.clone(), v.clone());
                    }
                }
            }
            None => current.push(radio),
        }
    }
}

// ---------------------------------------------------------------- scoperta (porta di gestione)

/// Un gruppo di Cat4OM come lo descrive la gestione (`getServiceStatus` → `result.groups[]`).
#[derive(Debug, Clone, PartialEq)]
pub struct GroupInfo {
    pub id: String,
    pub name: String,
    /// La porta Control del gruppo (assegnata da Cat4OM); assente se il gruppo e' fermo.
    pub control_port: Option<u16>,
    pub running: bool,
    /// Le radio del gruppo: `radioId` e `radioName`.
    pub radios: Vec<(String, String)>,
}

/// I gruppi dal risultato di `getServiceStatus`. `isRunning` assente: il gruppo gira se ha una porta Control.
pub fn parse_groups(result: &Value) -> Vec<GroupInfo> {
    let Some(groups) = result.get("groups").and_then(Value::as_array) else { return Vec::new() };
    groups
        .iter()
        .filter_map(|g| {
            let id = text(g, "id")?;
            let control_port = g.get("controlPort").and_then(Value::as_u64).and_then(|p| u16::try_from(p).ok()).filter(|p| *p != 0);
            let radios = g
                .get("radios")
                .and_then(Value::as_array)
                .map(|a| a.iter().filter_map(|r| text(r, "radioId").map(|rid| (rid.clone(), text(r, "radioName").unwrap_or(rid)))).collect())
                .unwrap_or_default();
            Some(GroupInfo {
                name: text(g, "name").unwrap_or_else(|| id.clone()),
                running: g.get("isRunning").and_then(Value::as_bool).unwrap_or(control_port.is_some()),
                control_port,
                radios,
                id,
            })
        })
        .collect()
}

/// Sceglie il gruppo: quello scritto (per `id` o per nome); vuoto = quello che contiene la radio scritta, altrimenti il primo in funzione,
/// altrimenti il primo («Scoperta automatica» di Decodium). L'errore e' il testo per l'utente.
pub fn choose_group(wanted: &str, wanted_radio: &str, groups: &[GroupInfo]) -> Result<GroupInfo, String> {
    if groups.is_empty() {
        return Err(NO_GROUPS.into());
    }
    let wanted = wanted.trim();
    if !wanted.is_empty() {
        return groups
            .iter()
            .find(|g| g.id == wanted)
            .or_else(|| groups.iter().find(|g| g.id.eq_ignore_ascii_case(wanted) || g.name.eq_ignore_ascii_case(wanted)))
            .cloned()
            .ok_or_else(|| group_missing(wanted, &groups.iter().map(|g| g.id.as_str()).collect::<Vec<_>>().join(", ")));
    }
    let radio = wanted_radio.trim();
    let has_radio = |g: &&GroupInfo| !radio.is_empty() && g.radios.iter().any(|(id, name)| id.eq_ignore_ascii_case(radio) || name.eq_ignore_ascii_case(radio));
    let chosen = groups
        .iter()
        .filter(has_radio)
        .find(|g| g.running)
        .or_else(|| groups.iter().find(has_radio))
        .or_else(|| groups.iter().find(|g| g.running))
        .unwrap_or(&groups[0]);
    Ok(chosen.clone())
}

/// L'indirizzo del Control di un gruppo: l'host della porta di gestione con la porta del gruppo.
pub fn control_address(management: &str, port: u16) -> String {
    let host = management.trim().rsplit_once(':').map_or(management.trim(), |(host, _)| host);
    let host = if host.is_empty() { "127.0.0.1" } else { host };
    format!("{host}:{port}")
}

/// Dove Aurora si collega: porta di gestione, porta Control (scoperta o scritta a mano), gruppo, radio e ruolo. Serve allo stato
/// («Collegato — group-1/kenwood/ts-990 — master», come Decodium) e al suggerimento del campo «Controllo».
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct Cat4omEndpoint {
    pub management: String,
    /// La porta Control in uso o in prova (vuota finche' la scoperta non la trova).
    pub control: String,
    /// `true` = la porta Control viene dalla scoperta, `false` = scritta a mano.
    pub discovered: bool,
    pub group: String,
    pub radio: String,
    /// `master`, `slave` o vuoto se non collegata.
    pub role: String,
}

/// Il testo dello stato a radio collegata: «gruppo/radio — ruolo».
pub fn link_summary(group: &str, radio: &str, master: bool) -> String {
    let role = if master { "master" } else { "slave" };
    if group.is_empty() { format!("{radio} — {role}") } else { format!("{group}/{radio} — {role}") }
}

// ---------------------------------------------------------------- la radio

/// I tempi del collegamento (le prove li accorciano).
#[derive(Debug, Clone, Copy)]
pub struct Cat4omTiming {
    /// Attesa del `welcome` dopo l'`hello` (il server aspetta l'`hello` circa 10 s).
    pub handshake: Duration,
    /// Attesa della risposta a una richiesta (10 s, il valore suggerito dal manuale e da Decodium).
    pub request: Duration,
    /// Attesa dello `stateUpdate` dopo una scrittura riuscita (non e' un errore se manca).
    pub echo: Duration,
    /// Attese prima di ritentare il collegamento, come Decodium: 1; 2,5; 5; 10; 20; 30 s.
    pub backoff: [Duration; 6],
    /// Massima attesa del `retryAfterMs` di `OWNERSHIP_COOLDOWN` (si riprova una volta sola).
    pub max_cooldown: Duration,
    /// Le richieste di chiusura (PTT spento, split tolto): brevi, poi si chiude comunque.
    pub release: Duration,
}

impl Default for Cat4omTiming {
    fn default() -> Self {
        Cat4omTiming {
            handshake: Duration::from_secs(10),
            request: Duration::from_secs(10),
            echo: Duration::from_millis(1000),
            backoff: [1000, 2500, 5000, 10_000, 20_000, 30_000].map(Duration::from_millis),
            max_cooldown: Duration::from_secs(6),
            release: Duration::from_millis(800),
        }
    }
}

/// L'esito di una richiesta.
#[derive(Debug, Clone, PartialEq)]
enum CallError {
    /// Collegamento assente o caduto.
    Link(String),
    /// Nessuna risposta in tempo.
    Timeout,
    Server(ServerError),
}

type Reply = Result<Value, ServerError>;

/// Lo stato di un collegamento: vale finche' il WebSocket e' aperto (dopo una riconnessione il server da' un nuovo `clientId` e un nuovo ruolo).
#[derive(Debug, Default)]
struct Session {
    client_id: String,
    instance_id: String,
    /// Il gruppo del Control (`welcome.groupId`).
    group_id: String,
    is_master: bool,
    radios: Vec<Value>,
    radio_id: Option<String>,
    seq: Option<u64>,
    keepalive: Duration,
}

struct Inner {
    /// La porta di gestione da cui si scoprono i gruppi e la loro porta Control.
    management: String,
    /// Il gruppo voluto (`id` o nome); vuoto = scoperta automatica.
    group: String,
    /// La porta Control scritta a mano; `None` = quella scoperta dalla gestione.
    control_override: Option<String>,
    /// Dove si e' collegati o si prova a collegarsi.
    endpoint: Mutex<Cat4omEndpoint>,
    /// La chiave del gruppo: mai nei registri ne' nei messaggi.
    key: String,
    wanted_radio: String,
    cat_mode: CatMode,
    split: SplitMode,
    power_off: bool,
    timing: Cat4omTiming,
    stop: AtomicBool,
    /// `welcome` ricevuto: le richieste si possono mandare.
    link_up: AtomicBool,
    /// Collegamento su e radio scelta collegata e non presa da un'emulazione: i comandi radio si possono mandare.
    ready: AtomicBool,
    session: Mutex<Session>,
    status: Mutex<RigStatus>,
    meters: Mutex<Meters>,
    outbox: Mutex<VecDeque<String>>,
    /// Le richieste in attesa di risposta: `None` finche' non arriva.
    waiting: Mutex<HashMap<String, Option<Reply>>>,
    /// Il thread del collegamento sveglia chi aspetta dopo ogni messaggio (o quando cade).
    wake_lock: Mutex<()>,
    wake: Condvar,
    /// Una richiesta di comando alla volta (ownership e azione insieme).
    command_lock: Mutex<()>,
    /// La frequenza audio di trasmissione (Hz, 0 = non ancora data): serve allo split.
    tx_audio_hz: AtomicU32,
    /// Split simulato: la frequenza di ricezione da rimettere a fine trasmissione (0 = niente).
    emulate_restore: AtomicU64,
    /// Lo split vero acceso da Aurora: si spegne alla chiusura.
    split_by_us: AtomicBool,
    /// Split DX acceso (doppio clic su uno spot del cluster con «UP 5», «QSX ...»): il VFO di trasmissione l'ha scelto l'operatore e lo
    /// split di FT8 non lo sposta finche' resta acceso.
    dx_split: AtomicBool,
    /// PTT e split gia' rimessi a posto per questo collegamento.
    released: AtomicBool,
}

impl Inner {
    fn update(&self, edit: impl FnOnce(&mut RigStatus)) {
        if let Ok(mut s) = self.status.lock() {
            edit(&mut s);
            s.updated_ms = SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |d| d.as_millis() as u64);
        }
    }

    fn set_endpoint(&self, control: &str, discovered: bool, group: &str) {
        if let Ok(mut e) = self.endpoint.lock() {
            e.control = control.to_string();
            e.discovered = discovered;
            e.group = group.to_string();
        }
    }

    fn send(&self, text: String) {
        if let Ok(mut o) = self.outbox.lock() {
            o.push_back(text);
        }
    }

    fn notify(&self) {
        let _guard = self.wake_lock.lock();
        self.wake.notify_all();
    }

    fn connected(&self) -> Result<(), RigError> {
        if self.ready.load(Ordering::SeqCst) {
            return Ok(());
        }
        let message = self.status.lock().map(|s| s.message.clone()).unwrap_or_default();
        Err(RigError(if message.is_empty() { NOT_LINKED.into() } else { message }))
    }

    fn dial(&self) -> Option<u64> {
        self.status.lock().ok().and_then(|s| s.frequency_hz)
    }

    fn ptt(&self) -> bool {
        self.status.lock().is_ok_and(|s| s.ptt)
    }

    fn is_master(&self) -> bool {
        self.session.lock().is_ok_and(|s| s.is_master)
    }

    /// La radio scelta, dallo stato ricevuto.
    fn view(&self) -> Option<RadioView> {
        let s = self.session.lock().ok()?;
        let id = s.radio_id.as_deref()?;
        s.radios.iter().filter(|r| r.get("radioId").and_then(Value::as_str) == Some(id)).find_map(RadioView::from_state)
    }

    /// Lo split in uso: «Rig» diventa simulato se la radio non sa fare lo split vero.
    fn split_mode(&self) -> SplitMode {
        match self.split {
            SplitMode::Rig if !self.view().is_some_and(|v| v.real_split()) => SplitMode::Emulate,
            other => other,
        }
    }

    /// Aspetta che `done` sia vero per al piu' `timeout`; `false` se il tempo passa o se il collegamento cade.
    fn wait_until(&self, timeout: Duration, done: impl Fn(&Inner) -> bool) -> bool {
        let deadline = Instant::now() + timeout;
        let Ok(mut guard) = self.wake_lock.lock() else { return done(self) };
        loop {
            if done(self) {
                return true;
            }
            let now = Instant::now();
            if now >= deadline || !self.link_up.load(Ordering::SeqCst) {
                return false;
            }
            guard = match self.wake.wait_timeout(guard, deadline - now) {
                Ok((g, _)) => g,
                Err(_) => return done(self),
            };
        }
    }

    /// Una richiesta con la sua risposta. Mai dal thread del collegamento (e' lui a leggere le risposte).
    fn call(&self, action: &str, on_radio: bool, params: Value, timeout: Duration) -> Result<Value, CallError> {
        let (client_id, radio_id) = self.session.lock().map(|s| (s.client_id.clone(), s.radio_id.clone())).unwrap_or_default();
        if !self.link_up.load(Ordering::SeqCst) || client_id.is_empty() {
            return Err(CallError::Link(NOT_LINKED.into()));
        }
        let radio = match (on_radio, radio_id) {
            (false, _) => None,
            (true, Some(id)) => Some(id),
            (true, None) => return Err(CallError::Link(NOT_LINKED.into())),
        };
        let id = new_request_id();
        if let Ok(mut w) = self.waiting.lock() {
            w.insert(id.clone(), None);
        }
        self.send(request_message(&id, &client_id, action, radio.as_deref(), params));
        let answered = self.wait_until(timeout, |s| s.waiting.lock().is_ok_and(|w| w.get(&id).is_some_and(Option::is_some)));
        let reply = self.waiting.lock().ok().and_then(|mut w| w.remove(&id)).flatten();
        match reply {
            Some(Ok(result)) => Ok(result),
            Some(Err(e)) => Err(CallError::Server(e)),
            None if !answered && self.link_up.load(Ordering::SeqCst) => Err(CallError::Timeout),
            None => Err(CallError::Link(NOT_LINKED.into())),
        }
    }

    fn rig_error(&self, action: &str, error: CallError) -> RigError {
        RigError(match error {
            CallError::Link(m) => m,
            CallError::Timeout => timeout_text(action, self.timing.request),
            CallError::Server(e) => match e.code.as_str() {
                "RADIO_CLAIMED" => {
                    let view = self.view();
                    let name = view.as_ref().map_or_else(String::new, |v| v.name.clone());
                    let by = view.and_then(|v| v.bridge_controlled_by).unwrap_or_else(|| "Hamlib/TCI".into());
                    claimed(&name, &by)
                }
                "TX_IN_PROGRESS" => format!("Cat4OM: un'altra radio o un altro programma sta trasmettendo ({action} rifiutato)"),
                _ => refused_text(action, &e),
            },
        })
    }

    /// Diventa master del gruppo se non lo e' gia' (`getOwnership`). Con `OWNERSHIP_COOLDOWN` aspetta `retryAfterMs` (al massimo
    /// `max_cooldown`) e riprova una volta sola, come chiede il manuale (mai in giro).
    fn ensure_master(&self) -> Result<(), RigError> {
        if self.is_master() {
            return Ok(());
        }
        let mut cooled = false;
        loop {
            match self.call("getOwnership", false, json!({}), self.timing.request) {
                Ok(result) => {
                    let role = result.get("role").and_then(Value::as_str).unwrap_or("master");
                    let master = role == "master";
                    if let Ok(mut s) = self.session.lock() {
                        s.is_master = master;
                    }
                    if !master {
                        return Err(RigError(format!("Cat4OM non ha dato il comando della radio ad Aurora (ruolo {role})")));
                    }
                    tracing::info!("Cat4OM: Aurora ora comanda il gruppo (master)");
                    // lo stato dice «master»
                    self.refresh();
                    return Ok(());
                }
                Err(CallError::Server(e)) if e.code == "OWNERSHIP_COOLDOWN" && !cooled => {
                    cooled = true;
                    let wait = Duration::from_millis(e.retry_after_ms.unwrap_or(2000)).min(self.timing.max_cooldown);
                    tracing::info!("Cat4OM: comando della radio passato da poco, nuova richiesta tra {} ms", wait.as_millis());
                    let until = Instant::now() + wait;
                    while Instant::now() < until && !self.stop.load(Ordering::SeqCst) {
                        std::thread::sleep(Duration::from_millis(10));
                    }
                }
                Err(e) => return Err(self.rig_error("getOwnership", e)),
            }
        }
    }

    /// Un comando sulla radio scelta: master prima (solo se serve), poi l'azione. Se nel frattempo un altro programma ha preso il comando
    /// (`NOT_MASTER`), lo si richiede una volta.
    fn command(&self, action: &str, params: Value) -> Result<Value, RigError> {
        let _one = self.command_lock.lock();
        self.ensure_master()?;
        match self.call(action, true, params.clone(), self.timing.request) {
            Err(CallError::Server(e)) if e.code == "NOT_MASTER" => {
                if let Ok(mut s) = self.session.lock() {
                    s.is_master = false;
                }
                self.ensure_master()?;
                self.call(action, true, params, self.timing.request).map_err(|e| self.rig_error(action, e))
            }
            other => other.map_err(|e| self.rig_error(action, e)),
        }
    }

    /// `setFrequency` su un VFO (`None` = quello attivo) e attesa dello `stateUpdate` che la riporta (senza errore se tarda).
    fn set_vfo_frequency(&self, vfo: Option<&str>, hz: u64) -> Result<(), RigError> {
        let params = match vfo {
            Some(v) => json!({ "frequency": hz, "vfo": v }),
            None => json!({ "frequency": hz }),
        };
        self.command("setFrequency", params)?;
        let seen = self.wait_until(self.timing.echo, |s| {
            let view = s.view();
            match vfo {
                Some(v) => view.and_then(|r| r.vfo_frequency(v)) == Some(hz),
                None => view.and_then(|r| r.frequency_hz) == Some(hz),
            }
        });
        if !seen {
            tracing::debug!("Cat4OM: frequenza {hz} Hz accettata ma non ancora riportata dallo stato");
        }
        Ok(())
    }

    /// Aggiorna lo stato osservabile dalla radio scelta (dopo `welcome`, `stateUpdate`, `getAllState`).
    fn refresh(&self) {
        let chosen = {
            let Ok(mut s) = self.session.lock() else { return };
            let present = s.radio_id.as_deref().is_some_and(|id| s.radios.iter().any(|r| r.get("radioId").and_then(Value::as_str) == Some(id)));
            if !present {
                let mut choice = choose_radio(&self.wanted_radio, &s.radios);
                if choice.is_err() && !s.group_id.is_empty() && self.wanted_radio.eq_ignore_ascii_case(&s.group_id) {
                    // nel campo «Radio» c'e' il nome del gruppo (un utente aveva scritto «group-1»): vale come vuoto, la prima radio disponibile
                    choice = choose_radio("", &s.radios);
                }
                match choice {
                    Ok(id) => s.radio_id = Some(id),
                    Err(message) => {
                        s.radio_id = None;
                        drop(s);
                        self.ready.store(false, Ordering::SeqCst);
                        self.update(|st| {
                            st.state = LinkState::Error;
                            st.message = message;
                        });
                        return;
                    }
                }
            }
            s.radio_id.clone()
        };
        let Some(view) = chosen.and(self.view()) else { return };
        let readings = meter_readings(&view.metering);
        if let Ok(mut m) = self.meters.lock() {
            if !view.ptt.unwrap_or(false) {
                // in ricezione potenza e ROS non valgono
                m.power_w = None;
                m.swr = None;
            }
            for (level, value) in readings {
                meters::store(&mut m, level, value);
            }
        }
        let (state, message, ready) = if let Some(by) = &view.bridge_controlled_by {
            (LinkState::Error, claimed(&view.name, by), false)
        } else if view.connection_status != "connected" {
            (LinkState::Connecting, format!("Radio Cat4OM «{}» non collegata ({})", view.name, view.connection_status), false)
        } else {
            let (group, master) = self.session.lock().map(|s| (s.group_id.clone(), s.is_master)).unwrap_or_default();
            (LinkState::Connected, link_summary(&group, &view.id, master), true)
        };
        self.ready.store(ready, Ordering::SeqCst);
        self.update(|s| {
            s.frequency_hz = view.frequency_hz;
            s.mode = view.mode.clone().map(|m| m.to_ascii_uppercase());
            s.ptt = view.ptt.unwrap_or(false);
            s.split = view.split;
            if s.state != state || ready {
                s.message = message;
            }
            s.state = state;
            if ready {
                s.reconnect_attempt = 0;
                s.failures = 0;
            }
        });
    }

    /// Un messaggio del server dopo l'handshake.
    fn handle_text(&self, raw: &str) {
        match parse_message(raw) {
            Some(Incoming::StateUpdate { seq, full, master_id, radios }) => {
                let mut reload = false;
                if let Ok(mut s) = self.session.lock() {
                    if let (Some(last), Some(seq)) = (s.seq, seq) {
                        if seq < last {
                            // contatore tornato indietro: il gruppo e' ripartito, lo stato vecchio non vale piu'
                            tracing::info!("Cat4OM: gruppo ripartito (seq {seq} dopo {last}), stato ricaricato");
                            s.radios.clear();
                            reload = true;
                        } else if seq > last + 1 {
                            reload = true;
                        }
                    }
                    if seq.is_some() {
                        s.seq = seq;
                    }
                    if let Some(master) = master_id {
                        s.is_master = !s.client_id.is_empty() && master == s.client_id;
                    }
                    // il keepalive non porta radio: non si tocca lo stato
                    if !radios.is_empty() {
                        merge_radios(&mut s.radios, radios, full);
                    }
                }
                if reload {
                    self.request_all_state();
                }
                self.refresh();
            }
            Some(Incoming::Response { id, success, status, result, error }) => {
                if id.starts_with("state-") {
                    // la risposta a `getAllState` chiesto dal collegamento stesso
                    if success && let Some(radios) = result.get("radios").and_then(Value::as_array) {
                        if let Ok(mut s) = self.session.lock() {
                            s.radios = radios.clone();
                        }
                        self.refresh();
                    }
                } else {
                    let reply = if success && status.as_deref() == Some("FAILSAFE_UNKEY") {
                        // PTT forzato spento dal server: la richiesta non e' stata onorata
                        Err(ServerError { code: "FAILSAFE_UNKEY".into(), message: "PTT spento dal server".into(), retry_after_ms: None, details: Value::Null })
                    } else if success {
                        Ok(result)
                    } else {
                        Err(error.unwrap_or(ServerError { code: "UNKNOWN".into(), message: String::new(), retry_after_ms: None, details: Value::Null }))
                    };
                    if let Ok(mut w) = self.waiting.lock()
                        && let Some(slot) = w.get_mut(&id)
                    {
                        *slot = Some(reply);
                    }
                }
            }
            Some(Incoming::Event { event, radio_id, details }) => match event.as_str() {
                "ownershipChanged" => {
                    if let (Some(master), Ok(mut s)) = (details.get("masterId").and_then(Value::as_str), self.session.lock()) {
                        s.is_master = master == s.client_id;
                    }
                    self.refresh();
                }
                "signalUpdate" => {
                    let id = self.session.lock().ok().and_then(|s| s.radio_id.clone());
                    let block = id.and_then(|id| details.get("radios").and_then(|r| r.get(&id)).cloned());
                    if let Some(block) = block
                        && let Ok(mut m) = self.meters.lock()
                    {
                        for (level, value) in meter_readings(&block) {
                            meters::store(&mut m, level, value);
                        }
                    }
                }
                _ => tracing::debug!("Cat4OM: evento {event} ({radio_id:?})"),
            },
            Some(Incoming::Error(e)) => tracing::warn!("Cat4OM: errore dal server: {} ({})", e.message, e.code),
            _ => {}
        }
        self.notify();
    }

    /// `getAllState` dal thread del collegamento (senza attesa: la risposta si riconosce dal prefisso dell'id).
    fn request_all_state(&self) {
        let client_id = self.session.lock().map(|s| s.client_id.clone()).unwrap_or_default();
        if !client_id.is_empty() {
            self.send(request_message(&format!("state-{}", new_request_id()), &client_id, "getAllState", None, json!({})));
        }
    }

    /// Alla chiusura (una volta per collegamento): PTT spento, frequenza dello split simulato rimessa, split vero tolto se l'aveva acceso
    /// Aurora. La chiusura del WebSocket poi libera il comando del gruppo (il server toglie comunque la portante al master che si scollega).
    fn release(&self) {
        if !self.link_up.load(Ordering::SeqCst) || self.released.swap(true, Ordering::SeqCst) {
            return;
        }
        let quick = self.timing.release;
        if self.ptt() && self.is_master() {
            let _ = self.call("setPtt", true, json!({ "enabled": false }), quick);
        }
        let back = self.emulate_restore.swap(0, Ordering::SeqCst);
        if back != 0 && self.is_master() {
            let _ = self.call("setFrequency", true, json!({ "frequency": back }), quick);
        }
        let dx = self.dx_split.swap(false, Ordering::SeqCst);
        if (self.split_by_us.swap(false, Ordering::SeqCst) || dx) && self.is_master() {
            let _ = self.call("setSplit", true, json!({ "enabled": false }), quick);
        }
    }
}

/// Una radio comandata attraverso Cat4OM. Il collegamento vive in un thread, si ricollega da solo e non legge mai la radio a intervalli:
/// lo stato arriva dal server (push), cosi' la radio resta libera per Log4OM.
pub struct Cat4omRig {
    inner: Arc<Inner>,
    worker: Mutex<Option<JoinHandle<()>>>,
}

impl Cat4omRig {
    /// Dalla configurazione della radio: porta di gestione, gruppo, porta Control (se scritta), chiave del gruppo, radio, modo CAT, split e
    /// «Spegni».
    pub fn from_config(config: &RigConfig) -> Cat4omRig {
        Cat4omRig::with_timing(config, Cat4omTiming::default())
    }

    pub fn with_timing(config: &RigConfig, timing: Cat4omTiming) -> Cat4omRig {
        let management = config.cat4om_management.trim();
        let management = if management.is_empty() { DEFAULT_CAT4OM_MANAGEMENT.to_string() } else { management.to_string() };
        let control_override = config.cat4om_control_override();
        let endpoint = Cat4omEndpoint {
            management: management.clone(),
            control: control_override.clone().unwrap_or_default(),
            discovered: control_override.is_none(),
            ..Cat4omEndpoint::default()
        };
        let inner = Arc::new(Inner {
            management,
            group: config.cat4om_group.trim().to_string(),
            control_override,
            endpoint: Mutex::new(endpoint),
            key: config.cat4om_key.clone(),
            wanted_radio: config.cat4om_radio.trim().to_string(),
            cat_mode: config.cat_mode,
            split: config.split,
            power_off: config.power_off,
            timing,
            stop: AtomicBool::new(false),
            link_up: AtomicBool::new(false),
            ready: AtomicBool::new(false),
            session: Mutex::new(Session::default()),
            status: Mutex::new(RigStatus {
                state: LinkState::Connecting,
                message: String::new(),
                frequency_hz: None,
                mode: None,
                passband_hz: None,
                extras: Default::default(),
                ptt: false,
                split: None,
                failures: 0,
                reconnect_attempt: 0,
                ptt_blocked: false,
                updated_ms: 0,
            }),
            meters: Mutex::new(Meters::default()),
            outbox: Mutex::new(VecDeque::new()),
            waiting: Mutex::new(HashMap::new()),
            wake_lock: Mutex::new(()),
            wake: Condvar::new(),
            command_lock: Mutex::new(()),
            tx_audio_hz: AtomicU32::new(0),
            emulate_restore: AtomicU64::new(0),
            split_by_us: AtomicBool::new(false),
            dx_split: AtomicBool::new(false),
            released: AtomicBool::new(false),
        });
        let worker_inner = Arc::clone(&inner);
        let worker = std::thread::Builder::new().name("cat4om".into()).spawn(move || run(&worker_inner)).ok();
        Cat4omRig { inner, worker: Mutex::new(worker) }
    }

    /// La porta Control in uso (scoperta o scritta a mano); vuota finche' la scoperta non la trova.
    pub fn address(&self) -> String {
        self.inner.endpoint.lock().map(|e| e.control.clone()).unwrap_or_default()
    }

    /// Gestione, porta Control, gruppo, radio e ruolo (per lo stato e per l'interfaccia).
    pub fn endpoint(&self) -> Cat4omEndpoint {
        let mut endpoint = self.inner.endpoint.lock().map(|e| e.clone()).unwrap_or_default();
        if self.inner.link_up.load(Ordering::SeqCst)
            && let Ok(s) = self.inner.session.lock()
        {
            if !s.group_id.is_empty() {
                endpoint.group = s.group_id.clone();
            }
            endpoint.radio = s.radio_id.clone().unwrap_or_default();
            endpoint.role = if s.is_master { "master" } else { "slave" }.into();
        }
        endpoint
    }

    pub fn status(&self) -> RigStatus {
        self.inner.status.lock().map(|s| s.clone()).unwrap_or_else(|e| e.into_inner().clone())
    }

    pub fn transmitting(&self) -> bool {
        self.status().ptt
    }

    /// La radio comandata (`radioId`), se scelta.
    pub fn radio_id(&self) -> Option<String> {
        self.inner.session.lock().ok().and_then(|s| s.radio_id.clone())
    }

    /// Le radio del gruppo (per sceglierla dall'interfaccia).
    pub fn radios(&self) -> Vec<RadioView> {
        self.inner.session.lock().map(|s| s.radios.iter().filter_map(RadioView::from_state).collect()).unwrap_or_default()
    }

    /// Aurora ha il comando del gruppo (master) in questo momento.
    pub fn is_master(&self) -> bool {
        self.inner.is_master()
    }

    /// Lo split in uso: «Rig» se la radio sa fare lo split vero, altrimenti simulato.
    pub fn split_mode(&self) -> SplitMode {
        self.inner.split_mode()
    }

    /// Il PTT con `setPtt`. Con lo split simulato, prima della trasmissione il VFO attivo va sulla frequenza di trasmissione e dopo torna.
    /// Lo spegnimento a collegamento caduto riesce: il server toglie da solo la portante al master che si scollega.
    pub fn set_ptt(&self, on: bool) -> Result<(), RigError> {
        let inner = &self.inner;
        if !on && !inner.link_up.load(Ordering::SeqCst) {
            inner.update(|s| s.ptt = false);
            return Ok(());
        }
        inner.connected()?;
        if on && inner.split_mode() == SplitMode::Emulate && !inner.dx_split.load(Ordering::SeqCst) {
            let audio = inner.tx_audio_hz.load(Ordering::SeqCst);
            if let Some(dial) = inner.dial()
                && audio != 0
            {
                let tx = split_tx_dial_hz(dial, audio);
                if tx != dial {
                    inner.set_vfo_frequency(None, tx)?;
                    inner.emulate_restore.store(dial, Ordering::SeqCst);
                }
            }
        }
        if inner.ptt() != on {
            if let Err(e) = inner.command("setPtt", json!({ "enabled": on })) {
                if on {
                    // la trasmissione non e' partita: si chiede comunque lo spegnimento e il VFO torna dov'era
                    let _ = inner.call("setPtt", true, json!({ "enabled": false }), inner.timing.release);
                    let back = inner.emulate_restore.swap(0, Ordering::SeqCst);
                    if back != 0 {
                        let _ = inner.call("setFrequency", true, json!({ "frequency": back }), inner.timing.release);
                    }
                }
                return Err(e);
            }
            inner.update(|s| s.ptt = on);
        }
        if !on {
            let back = inner.emulate_restore.swap(0, Ordering::SeqCst);
            if back != 0
                && let Err(e) = inner.set_vfo_frequency(None, back)
            {
                // il PTT e' spento: l'errore resta nello stato, la trasmissione non si considera fallita
                tracing::warn!("Cat4OM: frequenza di ricezione non rimessa dopo la trasmissione: {e}");
                inner.update(|s| s.message = e.0.clone());
            }
        }
        Ok(())
    }

    /// La frequenza audio con cui si trasmette (Hz). Split vero: il VFO di trasmissione va sulla frequenza giusta e lo split si accende (ogni
    /// comando solo se serve). Split simulato: serve al PTT. Durante la trasmissione Cat4OM accetta solo lo spegnimento del PTT: niente cambi.
    pub fn set_tx_audio_frequency(&self, hz: u32) -> Result<(), RigError> {
        if hz > 5000 {
            return Err(RigError(format!("frequenza audio di trasmissione non valida: {hz} Hz")));
        }
        let inner = &self.inner;
        inner.tx_audio_hz.store(hz, Ordering::SeqCst);
        if hz == 0 || inner.split_mode() != SplitMode::Rig || inner.dx_split.load(Ordering::SeqCst) {
            return Ok(());
        }
        inner.connected()?;
        if inner.ptt() {
            return Ok(());
        }
        let Some(view) = inner.view() else { return Err(RigError(NOT_LINKED.into())) };
        let (Some(dial), Some(tx_vfo)) = (view.frequency_hz, view.other_vfo()) else { return Ok(()) };
        let target = split_tx_dial_hz(dial, hz);
        if view.vfo_frequency(&tx_vfo) != Some(target) {
            inner.set_vfo_frequency(Some(&tx_vfo), target)?;
        }
        if view.split != Some(true) || view.tx_vfo.as_deref() != Some(tx_vfo.as_str()) {
            inner.command("setSplit", json!({ "enabled": true, "txVfo": tx_vfo }))?;
        }
        inner.split_by_us.store(true, Ordering::SeqCst);
        Ok(())
    }

    /// Split DX (doppio clic su uno spot del cluster con «UP 5», «QSX 14.030»...): il VFO attivo su `rx_hz` (con il modo scelto, come un
    /// cambio di frequenza), l'altro VFO su `tx_hz` e `setSplit` per ultimo. Solo se la radio dichiara a Cat4OM lo split vero (`SetSplit` e
    /// un secondo VFO scrivibile).
    pub fn set_dx_split(&self, rx_hz: u64, tx_hz: u64) -> Result<(), RigError> {
        if !valid_dial_hz(tx_hz) {
            return Err(RigError(format!("frequenza di trasmissione non valida: {tx_hz} Hz")));
        }
        let inner = &self.inner;
        inner.connected()?;
        if !inner.view().is_some_and(|v| v.real_split()) {
            return Err(RigError("Cat4OM: la radio non dichiara lo split (SetSplit e un secondo VFO): split non supportato".into()));
        }
        self.set_frequency(rx_hz)?;
        let Some(tx_vfo) = inner.view().and_then(|v| v.other_vfo()) else {
            return Err(RigError("Cat4OM: nessun secondo VFO per la trasmissione: split non supportato".into()));
        };
        inner.dx_split.store(true, Ordering::SeqCst);
        let done = inner.set_vfo_frequency(Some(&tx_vfo), tx_hz).and_then(|()| inner.command("setSplit", json!({ "enabled": true, "txVfo": tx_vfo })));
        match done {
            Ok(_) => {
                inner.split_by_us.store(true, Ordering::SeqCst);
                Ok(())
            }
            Err(e) => {
                // la radio puo' essere ancora in split (anche quello dello spot di prima): si prova a spegnerlo; il flag resta acceso solo
                // se non si spegne, cosi' «Split DX: spegni» resta possibile e la chiusura lo spegne
                if let Err(off) = self.clear_dx_split() {
                    tracing::warn!("Cat4OM: split DX non spento dopo l'errore: {off}");
                }
                Err(e)
            }
        }
    }

    /// Lo Split DX e' acceso (o non si e' riusciti a spegnerlo).
    pub fn dx_split_active(&self) -> bool {
        self.inner.dx_split.load(Ordering::SeqCst)
    }

    /// Spegne lo Split DX, se acceso (`setSplit` con `enabled: false`).
    pub fn clear_dx_split(&self) -> Result<(), RigError> {
        let inner = &self.inner;
        if !inner.dx_split.load(Ordering::SeqCst) {
            return Ok(());
        }
        inner.connected()?;
        inner.command("setSplit", json!({ "enabled": false }))?;
        inner.dx_split.store(false, Ordering::SeqCst);
        Ok(())
    }

    /// «Spegni»: alla chiusura del programma `powerOff`, se la radio lo dichiara (prima PTT e split rimessi a posto).
    pub fn power_off_if_chosen(&self) {
        let inner = &self.inner;
        if !inner.power_off || !inner.ready.load(Ordering::SeqCst) || !inner.view().is_some_and(|v| v.has_command("PowerOff")) {
            return;
        }
        inner.release();
        match inner.command("powerOff", json!({})) {
            Ok(_) => tracing::info!("radio spenta via Cat4OM alla chiusura («Spegni»)"),
            Err(e) => tracing::warn!("Cat4OM: radio non spenta: {e}"),
        }
    }

    /// Chiude: PTT spento, split rimesso, collegamento chiuso (il comando del gruppo torna libero).
    pub fn shutdown(&self) {
        self.inner.release();
        self.inner.stop.store(true, Ordering::SeqCst);
        if let Some(handle) = self.worker.lock().ok().and_then(|mut w| w.take()) {
            let _ = handle.join();
        }
        self.inner.ready.store(false, Ordering::SeqCst);
        self.inner.link_up.store(false, Ordering::SeqCst);
        self.inner.notify();
        self.inner.update(|s| {
            s.state = LinkState::Disconnected;
            s.ptt = false;
        });
    }
}

impl Drop for Cat4omRig {
    fn drop(&mut self) {
        self.shutdown();
    }
}

fn open(address: &str) -> Result<WebSocket<TcpStream>, String> {
    let addr = address.to_socket_addrs().map_err(|e| e.to_string())?.next().ok_or_else(|| "indirizzo non valido".to_string())?;
    let stream = TcpStream::connect_timeout(&addr, Duration::from_secs(3)).map_err(|e| e.to_string())?;
    stream.set_read_timeout(Some(Duration::from_secs(3))).map_err(|e| e.to_string())?;
    stream.set_write_timeout(Some(Duration::from_secs(3))).map_err(|e| e.to_string())?;
    let _ = stream.set_nodelay(true);
    let (ws, _) = tungstenite::client(format!("ws://{address}/"), stream).map_err(|e| e.to_string())?;
    // dopo l'apertura le letture aspettano poco: il thread deve anche mandare le richieste in coda
    ws.get_ref().set_read_timeout(Some(Duration::from_millis(20))).map_err(|e| e.to_string())?;
    Ok(ws)
}

fn would_block(e: &tungstenite::Error) -> bool {
    matches!(e, tungstenite::Error::Io(io) if matches!(io.kind(), ErrorKind::WouldBlock | ErrorKind::TimedOut))
}

/// `hello` e attesa del `welcome` (il primo messaggio del server). L'errore e' il testo per l'utente.
fn handshake(inner: &Inner, ws: &mut WebSocket<TcpStream>, address: &str) -> Result<Welcome, String> {
    let hello = hello_message(env!("CARGO_PKG_VERSION"), &inner.key, SystemTime::now());
    ws.send(Message::text(hello.to_string())).map_err(|e| not_answering(address, &e.to_string()))?;
    let deadline = Instant::now() + inner.timing.handshake;
    while Instant::now() < deadline && !inner.stop.load(Ordering::SeqCst) {
        match ws.read() {
            Ok(Message::Text(text)) => {
                return match parse_message(text.as_str()) {
                    Some(Incoming::Welcome(w)) if w.endpoint == "control" || w.endpoint.is_empty() => {
                        if !same_major(&w.protocol_version) {
                            Err(VERSION_MISMATCH.into())
                        } else if w.client_id.is_empty() {
                            Err(not_control(address))
                        } else {
                            Ok(w)
                        }
                    }
                    Some(Incoming::ManagementWelcome { .. }) => Err(MANAGEMENT_PORT.into()),
                    Some(Incoming::Error(e)) => Err(match e.code.as_str() {
                        "AUTH_REQUIRED" => AUTH_REQUIRED.into(),
                        "AUTH_FAILED" => AUTH_FAILED.into(),
                        "PROTOCOL_VERSION_MISMATCH" => VERSION_MISMATCH.into(),
                        _ => format!("Cat4OM ha rifiutato il collegamento: {} ({})", e.message, e.code),
                    }),
                    _ => Err(not_control(address)),
                };
            }
            Ok(Message::Close(_)) => return Err(not_control(address)),
            Ok(_) => {}
            Err(e) if would_block(&e) => {}
            Err(e) => return Err(not_answering(address, &e.to_string())),
        }
    }
    Err(not_control(address))
}

/// Il `welcome` ricevuto: nuova sessione (un `instanceId` diverso vuol dire un altro servizio, ma lo stato vecchio si scarta comunque).
fn adopt_welcome(inner: &Inner, welcome: Welcome, address: &str) {
    if let Ok(mut s) = inner.session.lock() {
        if !s.instance_id.is_empty() && s.instance_id != welcome.instance_id {
            tracing::info!("Cat4OM: servizio diverso dal precedente, stato azzerato");
        }
        *s = Session {
            client_id: welcome.client_id,
            instance_id: welcome.instance_id,
            group_id: welcome.group_id.clone(),
            is_master: welcome.role == "master",
            radios: welcome.radios,
            radio_id: None,
            seq: welcome.seq,
            keepalive: Duration::from_millis(welcome.keepalive_ms.filter(|ms| *ms > 0).unwrap_or(10_000)),
        };
    }
    tracing::info!("Cat4OM: collegato al gruppo «{}» su {address} (ruolo {})", welcome.group_id, welcome.role);
    inner.released.store(false, Ordering::SeqCst);
    inner.link_up.store(true, Ordering::SeqCst);
    inner.refresh();
    inner.notify();
}

/// Lo stato di un collegamento caduto non vale per il prossimo.
fn reset_link(inner: &Inner) {
    inner.link_up.store(false, Ordering::SeqCst);
    inner.ready.store(false, Ordering::SeqCst);
    inner.split_by_us.store(false, Ordering::SeqCst);
    if let Ok(mut w) = inner.waiting.lock() {
        // chi aspetta una risposta la perde: si sveglia con «non collegata»
        for slot in w.values_mut() {
            if slot.is_none() {
                *slot = Some(Err(ServerError { code: "DISCONNECTED".into(), message: NOT_LINKED.into(), retry_after_ms: None, details: Value::Null }));
            }
        }
    }
    if let Ok(mut s) = inner.session.lock() {
        s.is_master = false;
        s.client_id.clear();
    }
    if let Ok(mut o) = inner.outbox.lock() {
        o.clear();
    }
    inner.notify();
}

/// Dove la scoperta ha trovato il gruppo: porta Control e `id` del gruppo.
#[derive(Debug, Clone, PartialEq)]
struct Discovered {
    control: String,
    group: String,
}

/// Una richiesta sulla porta di gestione con la sua risposta. Il collegamento serve solo alla scoperta: le risposte si leggono qui.
fn management_call(inner: &Inner, ws: &mut WebSocket<TcpStream>, management: &str, client_id: &str, action: &str, params: Value) -> Result<Value, String> {
    let id = new_request_id();
    ws.send(Message::text(request_message(&id, client_id, action, None, params))).map_err(|e| management_unreachable(management, &e.to_string()))?;
    let deadline = Instant::now() + inner.timing.request;
    while Instant::now() < deadline && !inner.stop.load(Ordering::SeqCst) {
        match ws.read() {
            Ok(Message::Text(text)) => {
                // i push (`serviceStatus`) e le risposte d'altro si ignorano
                if let Some(Incoming::Response { id: answered, success, result, error, .. }) = parse_message(text.as_str())
                    && answered == id
                {
                    return if success {
                        Ok(result)
                    } else {
                        let e = error.unwrap_or(ServerError { code: "UNKNOWN".into(), message: String::new(), retry_after_ms: None, details: Value::Null });
                        Err(refused_text(action, &e))
                    };
                }
            }
            Ok(Message::Close(_)) => return Err(management_unreachable(management, "chiuso dal server")),
            Ok(_) => {}
            Err(e) if would_block(&e) => {}
            Err(e) => return Err(management_unreachable(management, &e.to_string())),
        }
    }
    Err(timeout_text(action, inner.timing.request))
}

/// `hello` sulla porta di gestione (senza chiave: la password di gestione e' un'altra) e attesa del `managementWelcome`; torna il `clientId`.
fn management_handshake(inner: &Inner, ws: &mut WebSocket<TcpStream>, management: &str) -> Result<String, String> {
    let hello = hello_message(env!("CARGO_PKG_VERSION"), "", SystemTime::now());
    ws.send(Message::text(hello.to_string())).map_err(|e| management_unreachable(management, &e.to_string()))?;
    let deadline = Instant::now() + inner.timing.handshake;
    while Instant::now() < deadline && !inner.stop.load(Ordering::SeqCst) {
        match ws.read() {
            Ok(Message::Text(text)) => {
                return match parse_message(text.as_str()) {
                    Some(Incoming::ManagementWelcome { client_id, protocol_version }) => {
                        if !protocol_version.is_empty() && !same_major(&protocol_version) {
                            Err(VERSION_MISMATCH.into())
                        } else {
                            Ok(client_id)
                        }
                    }
                    Some(Incoming::Error(e)) => Err(match e.code.as_str() {
                        "AUTH_REQUIRED" | "AUTH_FAILED" => MANAGEMENT_AUTH.into(),
                        "PROTOCOL_VERSION_MISMATCH" => VERSION_MISMATCH.into(),
                        _ => format!("Cat4OM ha rifiutato il collegamento: {} ({})", e.message, e.code),
                    }),
                    _ => Err(not_management(management)),
                };
            }
            Ok(Message::Close(_)) => return Err(not_management(management)),
            Ok(_) => {}
            Err(e) if would_block(&e) => {}
            Err(e) => return Err(management_unreachable(management, &e.to_string())),
        }
    }
    Err(not_management(management))
}

/// La scoperta di Decodium: gestione → `getServiceStatus` → gruppo scelto (avviato con `startGroup` se fermo) → la sua porta Control.
fn discover(inner: &Inner, management: &str) -> Result<Discovered, String> {
    let mut ws = open(management).map_err(|e| management_unreachable(management, &e))?;
    let found = discover_on(inner, &mut ws, management);
    let _ = ws.close(None);
    let _ = ws.flush();
    found
}

fn discover_on(inner: &Inner, ws: &mut WebSocket<TcpStream>, management: &str) -> Result<Discovered, String> {
    let client_id = management_handshake(inner, ws, management)?;
    let groups = parse_groups(&management_call(inner, ws, management, &client_id, "getServiceStatus", json!({}))?);
    let mut group = choose_group(&inner.group, &inner.wanted_radio, &groups)?;
    if !group.running || group.control_port.is_none() {
        // un gruppo fermo non ha la porta Control aperta: lo si avvia, come Decodium, e si rilegge lo stato finche' la porta compare
        tracing::info!("Cat4OM: il gruppo «{}» e' fermo, lo avvio", group.id);
        management_call(inner, ws, management, &client_id, "startGroup", json!({ "groupId": group.id }))?;
        let until = Instant::now() + inner.timing.request;
        loop {
            let groups = parse_groups(&management_call(inner, ws, management, &client_id, "getServiceStatus", json!({}))?);
            if let Some(again) = groups.into_iter().find(|g| g.id == group.id) {
                group = again;
            }
            if (group.running && group.control_port.is_some()) || Instant::now() >= until || inner.stop.load(Ordering::SeqCst) {
                break;
            }
            let pause = Instant::now() + Duration::from_millis(250);
            while Instant::now() < pause && !inner.stop.load(Ordering::SeqCst) {
                std::thread::sleep(Duration::from_millis(10));
            }
        }
    }
    match (group.running, group.control_port) {
        (true, Some(port)) => {
            let control = control_address(management, port);
            tracing::info!("Cat4OM: gruppo «{}» trovato sulla porta Control {control} (gestione {management})", group.id);
            Ok(Discovered { control, group: group.id })
        }
        _ => Err(group_stopped(&group.id)),
    }
}

/// Il motivo di un errore: alla prima volta errore, poi riconnessione in corso.
fn report_failure(inner: &Inner, attempt: u32, message: String, keep_error: bool) {
    tracing::info!("Cat4OM: {message}");
    inner.update(|s| {
        s.state = if keep_error || attempt == 1 { LinkState::Error } else { LinkState::Reconnecting };
        s.message = message;
        s.reconnect_attempt = attempt;
        s.failures = attempt;
    });
}

fn run(inner: &Inner) {
    let mut attempt = 0u32;
    // la porta scritta in «Controllo» ha risposto come gestione (per esempio 17300): da li' si scopre il gruppo
    let mut override_is_management = false;
    while !inner.stop.load(Ordering::SeqCst) {
        // la porta Control: scritta a mano, oppure scoperta ogni volta (al riavvio di Cat4OM puo' cambiare)
        let target = match (&inner.control_override, override_is_management) {
            (Some(address), false) => Ok(address.clone()),
            (written, _) => {
                let management = written.clone().filter(|_| override_is_management).unwrap_or_else(|| inner.management.clone());
                discover(inner, &management).map(|found| {
                    inner.set_endpoint(&found.control, true, &found.group);
                    found.control
                })
            }
        };
        match target {
            Err(message) => {
                attempt += 1;
                report_failure(inner, attempt, message, true);
            }
            Ok(address) => match open(&address) {
                Ok(mut ws) => match handshake(inner, &mut ws, &address) {
                    Ok(welcome) => {
                        attempt = 0;
                        adopt_welcome(inner, welcome, &address);
                        let reason = serve(inner, &mut ws);
                        let _ = ws.close(None);
                        let _ = ws.flush();
                        // chiusura ordinata: si legge (e si scarta) quel che arriva ancora finche' il server risponde, al massimo 800 ms (Decodium)
                        let until = Instant::now() + Duration::from_millis(800);
                        while Instant::now() < until {
                            match ws.read() {
                                Ok(Message::Close(_)) => break,
                                Err(e) if would_block(&e) => {}
                                Err(_) => break,
                                Ok(_) => {}
                            }
                        }
                        reset_link(inner);
                        if inner.stop.load(Ordering::SeqCst) {
                            break;
                        }
                        tracing::warn!("Cat4OM: collegamento perso: {reason}");
                        inner.update(|s| {
                            s.state = LinkState::Reconnecting;
                            s.message = format!("Collegamento con Cat4OM perso ({reason}): nuovo tentativo");
                            s.ptt = false;
                        });
                    }
                    Err(message) => {
                        let _ = ws.close(None);
                        let _ = ws.flush();
                        if message == MANAGEMENT_PORT && inner.control_override.is_some() && !override_is_management {
                            // la porta scritta e' quella di gestione: si usa per la scoperta, subito
                            tracing::info!("Cat4OM: {address} e' la porta di gestione, scopro da li' il gruppo e la sua porta Control");
                            override_is_management = true;
                            continue;
                        }
                        attempt += 1;
                        report_failure(inner, attempt, message, true);
                    }
                },
                Err(e) => {
                    attempt += 1;
                    report_failure(inner, attempt, not_answering(&address, &e), false);
                }
            },
        }
        // attesa prima di riprovare (1; 2,5; 5; 10; 20; 30 s), interrotta subito dalla chiusura
        let wait = inner.timing.backoff[(attempt.max(1) as usize - 1).min(5)];
        let until = Instant::now() + wait;
        while Instant::now() < until && !inner.stop.load(Ordering::SeqCst) {
            std::thread::sleep(Duration::from_millis(20));
        }
    }
}

/// Il giro di un collegamento aperto: manda le richieste in coda e legge i messaggi. Torna con il motivo quando il collegamento cade, quando
/// il server tace per 3 intervalli di keepalive o quando si chiude.
fn serve(inner: &Inner, ws: &mut WebSocket<TcpStream>) -> String {
    let mut last_rx = Instant::now();
    loop {
        let pending: Vec<String> = inner.outbox.lock().map(|mut o| o.drain(..).collect()).unwrap_or_default();
        for text in pending {
            if let Err(e) = ws.send(Message::text(text)) {
                return e.to_string();
            }
        }
        if inner.stop.load(Ordering::SeqCst) {
            let _ = ws.flush();
            return "chiuso".into();
        }
        let silence = inner.session.lock().map(|s| s.keepalive).unwrap_or(Duration::from_secs(10)) * 3;
        if last_rx.elapsed() > silence {
            return format!("nessun messaggio da {} s", silence.as_secs());
        }
        match ws.read() {
            Ok(Message::Text(text)) => {
                last_rx = Instant::now();
                inner.handle_text(text.as_str());
            }
            // nessun audio richiesto: i blocchi binari si ignorano
            Ok(Message::Close(_)) => return "chiuso dal server".into(),
            Ok(_) => last_rx = Instant::now(),
            Err(e) if would_block(&e) => {
                if let Err(e) = ws.flush()
                    && !would_block(&e)
                {
                    return e.to_string();
                }
            }
            Err(e) => return e.to_string(),
        }
    }
}

impl Rig for Cat4omRig {
    /// Cambio di frequenza del VFO attivo (non in trasmissione) e, se scelto, il modo (USB o USB dati): nessun invio se la radio e' gia' li'.
    fn set_frequency(&self, hz: u64) -> Result<(), RigError> {
        if !valid_dial_hz(hz) {
            return Err(RigError(format!("frequenza non valida: {hz} Hz")));
        }
        let inner = &self.inner;
        inner.connected()?;
        if self.transmitting() {
            return Err(RigError("la radio sta trasmettendo: cambio di frequenza rifiutato".into()));
        }
        if inner.dial() != Some(hz) {
            inner.set_vfo_frequency(None, hz)?;
        }
        let supported = inner.view().map(|v| v.supported_modes).unwrap_or_default();
        if let Some(mode) = cat4om_mode(inner.cat_mode, &supported) {
            let current = inner.status.lock().ok().and_then(|s| s.mode.clone()).unwrap_or_default();
            if !mode_satisfied(&mode, &current) {
                inner.command("setMode", json!({ "mode": mode }))?;
                inner.wait_until(inner.timing.echo, |s| s.status.lock().is_ok_and(|st| st.mode.as_deref().is_some_and(|m| mode_satisfied(&mode, m))));
            }
        }
        Ok(())
    }

    fn frequency(&self) -> Result<u64, RigError> {
        let s = self.status();
        match (s.state, s.frequency_hz) {
            (LinkState::Connected, Some(hz)) => Ok(hz),
            _ => Err(RigError(NOT_LINKED.into())),
        }
    }

    /// I misuratori pubblicati dal server (`metering`: dBm, watt, ROS).
    fn meters(&self) -> Result<Meters, RigError> {
        self.inner.connected()?;
        Ok(self.inner.meters.lock().map(|m| *m).unwrap_or_default())
    }

    fn set_rx_frequency(&self, _hz: u32) -> Result<(), RigError> {
        Err(RigError("comando del motore non supportato da Cat4OM".into()))
    }

    fn output_attenuation(&self) -> Result<u32, RigError> {
        Err(RigError("comando del motore non supportato da Cat4OM".into()))
    }

    fn set_output_attenuation(&self, _tenths: u32) -> Result<(), RigError> {
        Err(RigError("comando del motore non supportato da Cat4OM".into()))
    }

    fn set_mode(&self, _mode: &str) -> Result<(), RigError> {
        Err(RigError("comando del motore non supportato da Cat4OM".into()))
    }
}

#[cfg(test)]
mod tests {
    use std::net::TcpListener;

    use super::*;

    // ------------------------------------------------ costruzione e lettura

    #[test]
    fn base64_follows_rfc_4648() {
        for (raw, enc) in [("", ""), ("f", "Zg=="), ("fo", "Zm8="), ("foo", "Zm9v"), ("foob", "Zm9vYg=="), ("fooba", "Zm9vYmE="), ("foobar", "Zm9vYmFy")] {
            assert_eq!(base64(raw.as_bytes()), enc, "{raw}");
        }
    }

    #[test]
    fn the_timestamp_is_utc_to_the_minute() {
        // 2026-08-11 14:23:59 UTC, l'esempio del manuale
        let t = UNIX_EPOCH + Duration::from_secs(1_786_458_180 + 59);
        assert_eq!(utc_minute(t), "202608111423");
        assert_eq!(utc_minute(UNIX_EPOCH), "197001010000");
        assert_eq!(utc_minute(UNIX_EPOCH + Duration::from_secs(951_782_400)), "200002290000", "29 febbraio 2000");
    }

    #[test]
    fn the_hello_carries_the_proof_never_the_key() {
        let t = UNIX_EPOCH + Duration::from_secs(1_786_458_180);
        let plain = hello_message("1.2.3", "", t);
        assert_eq!(plain, json!({ "type": "hello", "protocolVersion": "1.0.0", "appName": "Aurora", "appVersion": "1.2.3" }));
        let hello = hello_message("1.2.3", "chiave-di-prova", t);
        assert_eq!(hello["auth"]["scheme"], "cat4om-sha256-minute-v1");
        assert_eq!(hello["auth"]["timestampUtc"], "202608111423");
        let proof = hello["auth"]["proof"].as_str().expect("prova");
        let expected: Vec<u8> = Sha256::digest(b"chiave-di-prova\n202608111423").iter().copied().collect();
        assert_eq!(proof, base64(&expected));
        assert_eq!(proof.len(), 44);
        assert!(!hello.to_string().contains("chiave-di-prova"));
        assert_ne!(auth_proof("chiave-di-prova", "202608111424"), proof, "cambia ogni minuto");
    }

    #[test]
    fn requests_have_the_documented_shape() {
        let r: Value = serde_json::from_str(&request_message("freq-1", "c1", "setFrequency", Some("run"), json!({ "frequency": 14_074_000, "vfo": "A" }))).expect("json");
        assert_eq!(r, json!({ "type": "request", "id": "freq-1", "clientId": "c1", "action": "setFrequency", "radioId": "run", "params": { "frequency": 14_074_000, "vfo": "A" } }));
        let own: Value = serde_json::from_str(&request_message("own-1", "c1", "getOwnership", None, json!({}))).expect("json");
        assert!(own.get("radioId").is_none());
        let (a, b) = (new_request_id(), new_request_id());
        assert_ne!(a, b);
        assert_eq!(a.len(), 36);
        assert!(same_major("1.4.0") && !same_major("2.0.0"));
    }

    fn radio(id: &str, commands: &[&str]) -> Value {
        json!({
            "radioId": id, "radioName": format!("Radio {id}"), "model": "Yaesu FT-991A", "connectionStatus": "connected",
            "activeVfo": "A", "txVfo": "A", "split": false, "ptt": false, "availableVfos": ["A", "B"],
            "availableCommands": commands, "commandVfoScopes": {}, "supportedModes": ["LSB", "USB", "CW", "DATA-USB", "FT8"],
            "vfos": { "A": { "frequency": 7_074_000, "mode": "USB" }, "B": { "frequency": 7_074_000, "mode": "USB" } },
            "metering": { "signalDbm": -85, "swr": null, "powerWatts": null }, "bridgeControlledBy": null
        })
    }

    const ALL: [&str; 7] = ["SetFrequency", "SetMode", "SetPtt", "SetSplit", "SetVfo", "PowerOff", "Tune"];

    #[test]
    fn it_reads_welcome_state_update_response_event_and_error() {
        let welcome = json!({ "type": "welcome", "endpoint": "control", "protocolVersion": "1.0.0", "groupId": "main", "clientId": "c-1",
            "role": "slave", "sessionToken": null, "radios": [radio("run", &ALL)], "seq": 0, "authRequired": false, "features": ["stateUpdateSeq"],
            "instanceId": "i-1", "serviceName": "SHACK-PC", "keepaliveIntervalMs": 10000, "managementPort": 17300, "x-futuro": 1 });
        let Some(Incoming::Welcome(w)) = parse_message(&welcome.to_string()) else { panic!("welcome") };
        assert_eq!((w.client_id.as_str(), w.role.as_str(), w.seq, w.keepalive_ms, w.radios.len()), ("c-1", "slave", Some(0), Some(10_000), 1));
        let update = json!({ "type": "stateUpdate", "masterId": "c-2", "seq": 4, "changes": {}, "groupChanges": [], "radios": [] });
        assert_eq!(parse_message(&update.to_string()), Some(Incoming::StateUpdate { seq: Some(4), full: true, master_id: Some("c-2".into()), radios: vec![] }));
        let bad = json!({ "type": "response", "id": "x", "success": false, "result": null, "error": { "code": "OWNERSHIP_COOLDOWN", "message": "wait", "retryAfterMs": 2380 } });
        let Some(Incoming::Response { success: false, error: Some(e), .. }) = parse_message(&bad.to_string()) else { panic!("risposta") };
        assert_eq!((e.code.as_str(), e.retry_after_ms), ("OWNERSHIP_COOLDOWN", Some(2380)));
        let ev = json!({ "type": "event", "event": "ownershipChanged", "radioId": null, "details": { "masterId": "c-1" } });
        assert!(matches!(parse_message(&ev.to_string()), Some(Incoming::Event { event, .. }) if event == "ownershipChanged"));
        assert!(matches!(parse_message(r#"{"type":"error","code":"AUTH_FAILED","message":"no"}"#), Some(Incoming::Error(e)) if e.code == "AUTH_FAILED"));
        assert_eq!(
            parse_message(r#"{"type":"managementWelcome","endpoint":"management","clientId":"m-1","protocolVersion":"1.0.0"}"#),
            Some(Incoming::ManagementWelcome { client_id: "m-1".into(), protocol_version: "1.0.0".into() })
        );
        assert_eq!(parse_message(r#"{"type":"x-nuovo"}"#), Some(Incoming::Other("x-nuovo".into())), "tipi sconosciuti ignorati");
        assert_eq!(parse_message("non json"), None);
    }

    #[test]
    fn the_radio_view_follows_the_active_vfo_and_capabilities() {
        let mut state = radio("run", &ALL);
        state["activeVfo"] = json!("B");
        state["vfos"]["B"] = json!({ "frequency": 14_074_000, "mode": "DATA-USB" });
        let v = RadioView::from_state(&state).expect("radio");
        assert_eq!((v.frequency_hz, v.mode.as_deref(), v.other_vfo().as_deref()), (Some(14_074_000), Some("DATA-USB"), Some("A")));
        assert!(v.real_split() && v.has_command("setsplit"));
        state["commandVfoScopes"] = json!({ "SetFrequency": "activeOnly" });
        assert!(!RadioView::from_state(&state).expect("radio").real_split(), "frequenza solo sul VFO attivo: split simulato");
        let mut single = radio("x", &["SetFrequency", "SetPtt"]);
        single["availableVfos"] = json!(["A"]);
        assert!(!RadioView::from_state(&single).expect("radio").real_split());
        state["bridgeControlledBy"] = json!("hamlib");
        assert_eq!(RadioView::from_state(&state).expect("radio").bridge_controlled_by.as_deref(), Some("hamlib"));
        assert_eq!(meter_readings(&json!({ "signalDbm": -73, "swr": 1.4, "powerWatts": 50 })), vec![(Level::Strength, 0.0), (Level::PowerWatts, 50.0), (Level::Swr, 1.4)]);
        assert_eq!(meter_readings(&json!({ "swr": null })), vec![]);
    }

    #[test]
    fn modes_radios_and_partial_updates() {
        let modes = |m: &[&str]| m.iter().map(|s| s.to_string()).collect::<Vec<_>>();
        assert_eq!(cat4om_mode(CatMode::Data, &modes(&["USB", "DATA-USB"])).as_deref(), Some("DATA-USB"));
        assert_eq!(cat4om_mode(CatMode::Data, &modes(&["USB", "PKTUSB"])).as_deref(), Some("PKTUSB"));
        assert_eq!(cat4om_mode(CatMode::Data, &[]).as_deref(), Some("DATA-USB"));
        assert_eq!(cat4om_mode(CatMode::Usb, &[]).as_deref(), Some("USB"));
        assert_eq!(cat4om_mode(CatMode::None, &[]), None);
        assert!(mode_satisfied("DATA-USB", "pktusb") && !mode_satisfied("DATA-USB", "USB"));
        let radios = vec![{ let mut r = radio("a", &ALL); r["connectionStatus"] = json!("disconnected"); r }, radio("b", &ALL)];
        assert_eq!(choose_radio("", &radios), Ok("b".into()), "la prima collegata");
        assert_eq!(choose_radio("a", &radios), Ok("a".into()));
        assert_eq!(choose_radio("radio B", &radios), Ok("b".into()), "anche per nome");
        assert!(choose_radio("zz", &radios).expect_err("manca").contains("presenti: a, b"));
        assert_eq!(choose_radio("", &[]), Err(NO_RADIOS.into()));
        let mut current = radios.clone();
        merge_radios(&mut current, vec![json!({ "radioId": "b", "ptt": true })], false);
        assert_eq!((current[1]["ptt"].as_bool(), current[1]["radioName"].as_str()), (Some(true), Some("Radio b")));
        merge_radios(&mut current, vec![radio("c", &ALL)], true);
        assert_eq!(current.len(), 1);
    }

    // ------------------------------------------------ server finto

    /// Lo stato del Cat4OM finto (un gruppo, Log4OM gia' collegato e master).
    struct FakeState {
        radios: Vec<Value>,
        group: String,
        master: String,
        seq: u64,
        requests: Vec<Value>,
        hellos: Vec<Value>,
        key: Option<String>,
        push: bool,
        drop_now: bool,
        cooldown_once: bool,
        connections: u32,
    }

    struct Fake {
        address: String,
        state: Arc<Mutex<FakeState>>,
    }

    impl Fake {
        fn requests(&self) -> Vec<Value> {
            self.state.lock().unwrap().requests.clone()
        }

        fn actions(&self) -> Vec<String> {
            self.requests().iter().map(|r| r["action"].as_str().unwrap_or_default().to_string()).collect()
        }

        /// Log4OM fa QSY da uno spot: prende il comando e sposta il VFO A.
        fn log4om_qsy(&self, hz: u64) {
            let mut s = self.state.lock().unwrap();
            s.master = "log4om".into();
            s.radios[0]["vfos"]["A"]["frequency"] = json!(hz);
            s.push = true;
        }
    }

    fn fake(radios: Vec<Value>, key: Option<&str>) -> Fake {
        let listener = TcpListener::bind("127.0.0.1:0").expect("porta");
        let address = listener.local_addr().expect("indirizzo").to_string();
        let state = Arc::new(Mutex::new(FakeState {
            radios,
            group: "main".into(),
            master: "log4om".into(),
            seq: 0,
            requests: Vec::new(),
            hellos: Vec::new(),
            key: key.map(str::to_string),
            push: false,
            drop_now: false,
            cooldown_once: false,
            connections: 0,
        }));
        let shared = Arc::clone(&state);
        std::thread::spawn(move || {
            for stream in listener.incoming() {
                let Ok(stream) = stream else { continue };
                serve_fake(stream, &shared);
            }
        });
        Fake { address, state }
    }

    fn state_update(s: &mut FakeState) -> String {
        s.seq += 1;
        let changes: serde_json::Map<String, Value> = s.radios.iter().filter_map(|r| r["radioId"].as_str()).map(|id| (id.to_string(), json!(["data"]))).collect();
        json!({ "type": "stateUpdate", "timestamp": "2026-11-11T10:00:00Z", "masterId": s.master, "seq": s.seq, "full": true,
            "groupChanges": [], "changes": changes, "radios": s.radios }).to_string()
    }

    fn serve_fake(stream: TcpStream, shared: &Arc<Mutex<FakeState>>) {
        let Ok(mut ws) = tungstenite::accept(stream) else { return };
        let Ok(Message::Text(hello)) = ws.read() else { return };
        let hello: Value = serde_json::from_str(hello.as_str()).unwrap_or_default();
        let client = {
            let mut s = shared.lock().unwrap();
            s.hellos.push(hello.clone());
            s.connections += 1;
            if let Some(key) = s.key.clone() {
                let ts = hello["auth"]["timestampUtc"].as_str().unwrap_or_default().to_string();
                let now = SystemTime::now();
                let fresh = [now - Duration::from_secs(60), now, now + Duration::from_secs(60)].iter().any(|t| utc_minute(*t) == ts);
                let error = match hello["auth"]["proof"].as_str() {
                    None => Some("AUTH_REQUIRED"),
                    Some(p) if !fresh || p != auth_proof(&key, &ts) => Some("AUTH_FAILED"),
                    Some(_) => None,
                };
                if let Some(code) = error {
                    let _ = ws.send(Message::text(json!({ "type": "error", "code": code, "message": "auth" }).to_string()));
                    let _ = ws.close(None);
                    return;
                }
            }
            let client = format!("client-{}", s.connections);
            let welcome = json!({ "type": "welcome", "endpoint": "control", "protocolVersion": "1.0.0", "groupId": s.group, "clientId": client,
                "role": if s.master == client { "master" } else { "slave" }, "sessionToken": null, "radios": s.radios, "seq": s.seq,
                "authRequired": s.key.is_some(), "features": ["stateUpdateSeq"], "instanceId": "fake", "keepaliveIntervalMs": 10000 });
            if ws.send(Message::text(welcome.to_string())).is_err() {
                return;
            }
            client
        };
        let _ = ws.get_ref().set_read_timeout(Some(Duration::from_millis(20)));
        loop {
            {
                let mut s = shared.lock().unwrap();
                if s.drop_now {
                    s.drop_now = false;
                    return;
                }
                if s.push {
                    s.push = false;
                    let update = state_update(&mut s);
                    if ws.send(Message::text(update)).is_err() {
                        return;
                    }
                }
            }
            match ws.read() {
                Ok(Message::Text(t)) => {
                    let request: Value = serde_json::from_str(t.as_str()).unwrap_or_default();
                    let mut s = shared.lock().unwrap();
                    s.requests.push(request.clone());
                    let (reply, changed) = answer(&mut s, &client, &request);
                    let _ = ws.send(Message::text(reply.to_string()));
                    if changed {
                        let update = state_update(&mut s);
                        let _ = ws.send(Message::text(update));
                    }
                }
                Ok(Message::Close(_)) => return,
                Ok(_) => {}
                Err(e) if would_block(&e) => {}
                Err(_) => return,
            }
        }
    }

    /// La risposta del finto Cat4OM a una richiesta e se lo stato e' cambiato.
    fn answer(s: &mut FakeState, client: &str, r: &Value) -> (Value, bool) {
        let id = r["id"].clone();
        let action = r["action"].as_str().unwrap_or_default();
        let p = &r["params"];
        let ok = |result: Value| json!({ "type": "response", "id": id, "success": true, "radioId": r["radioId"], "result": result });
        let fail = |code: &str, retry: Option<u64>| json!({ "type": "response", "id": id, "success": false, "result": null,
            "error": { "code": code, "message": code, "retryAfterMs": retry } });
        match action {
            "getOwnership" => {
                if s.cooldown_once {
                    s.cooldown_once = false;
                    return (fail("OWNERSHIP_COOLDOWN", Some(50)), false);
                }
                s.master = client.to_string();
                (ok(json!({ "role": "master" })), true)
            }
            "getAllState" => (ok(json!({ "radios": s.radios })), false),
            "setFrequency" | "setMode" | "setPtt" | "setSplit" | "powerOff" => {
                if s.master != client {
                    return (fail("NOT_MASTER", None), false);
                }
                let Some(radio) = s.radios.iter_mut().find(|x| x["radioId"] == r["radioId"]) else { return (fail("RADIO_NOT_FOUND", None), false) };
                let active = radio["activeVfo"].as_str().unwrap_or("A").to_string();
                match action {
                    "setFrequency" => {
                        let vfo = p["vfo"].as_str().unwrap_or(&active).to_string();
                        radio["vfos"][vfo]["frequency"] = p["frequency"].clone();
                    }
                    "setMode" => radio["vfos"][active]["mode"] = p["mode"].clone(),
                    "setPtt" => radio["ptt"] = p["enabled"].clone(),
                    "setSplit" => {
                        let on = p["enabled"].as_bool().unwrap_or(false);
                        radio["split"] = json!(on);
                        radio["txVfo"] = if on { p["txVfo"].clone() } else { json!(active) };
                    }
                    _ => radio["connectionStatus"] = json!("stopped"),
                }
                (ok(Value::Null), true)
            }
            _ => (fail("UNKNOWN_ACTION", None), false),
        }
    }

    fn timing() -> Cat4omTiming {
        Cat4omTiming {
            handshake: Duration::from_secs(2),
            request: Duration::from_secs(2),
            echo: Duration::from_millis(300),
            backoff: [Duration::from_millis(50); 6],
            max_cooldown: Duration::from_secs(1),
            release: Duration::from_millis(300),
        }
    }

    fn wait(cond: impl Fn() -> bool) -> bool {
        let until = Instant::now() + Duration::from_secs(5);
        while Instant::now() < until {
            if cond() {
                return true;
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        false
    }

    fn rig(fake: &Fake, edit: impl FnOnce(&mut RigConfig)) -> Cat4omRig {
        let mut config = RigConfig { cat4om_address: fake.address.clone(), ..RigConfig::default() };
        edit(&mut config);
        Cat4omRig::with_timing(&config, timing())
    }

    #[test]
    fn it_shares_the_radio_with_log4om_without_polling() {
        let fake = fake(vec![radio("run", &ALL)], None);
        let rig = rig(&fake, |c| c.cat_mode = CatMode::Data);
        assert!(wait(|| rig.status().state == LinkState::Connected), "{:?}", rig.status());
        assert_eq!(rig.frequency(), Ok(7_074_000));
        assert!(!rig.is_master(), "al collegamento il comando resta a Log4OM");
        // nessuna lettura a intervalli: a riposo Aurora non chiede nulla
        std::thread::sleep(Duration::from_millis(300));
        assert!(fake.requests().is_empty(), "{:?}", fake.requests());
        // una scrittura: prima il comando del gruppo, poi frequenza e modo dati
        rig.set_frequency(14_074_000).expect("frequenza");
        assert_eq!(fake.actions(), ["getOwnership", "setFrequency", "setMode"]);
        assert_eq!(fake.requests()[1]["params"], json!({ "frequency": 14_074_000 }));
        assert_eq!(fake.requests()[2]["params"]["mode"], "DATA-USB");
        assert!(rig.is_master());
        assert!(wait(|| rig.frequency() == Ok(14_074_000)));
        // Log4OM fa QSY da uno spot: Aurora lo segue senza chiedere nulla
        fake.log4om_qsy(7_074_000);
        assert!(wait(|| rig.frequency() == Ok(7_074_000)));
        assert!(!rig.is_master(), "il comando e' passato a Log4OM");
        assert_eq!(fake.requests().len(), 3);
        // PTT: Aurora riprende il comando solo ora
        rig.set_ptt(true).expect("PTT");
        assert!(rig.transmitting());
        assert!(rig.set_frequency(3_573_000).is_err(), "niente cambio di frequenza in trasmissione");
        rig.set_ptt(false).expect("PTT");
        assert_eq!(fake.actions()[3..], ["getOwnership", "setPtt", "setPtt"]);
        assert_eq!(rig.meters().expect("misuratori").s_points, Some(9.0 - 12.0 / 6.0));
        rig.shutdown();
        assert_eq!(rig.status().state, LinkState::Disconnected);
        assert_eq!(fake.requests().len(), 6, "alla chiusura nulla da rimettere");
    }

    #[test]
    fn real_split_uses_vfo_b_and_is_undone_on_close() {
        let fake = fake(vec![radio("run", &ALL)], None);
        let rig = rig(&fake, |c| c.split = SplitMode::Rig);
        assert!(wait(|| rig.status().state == LinkState::Connected));
        assert_eq!(rig.split_mode(), SplitMode::Rig);
        rig.set_tx_audio_frequency(2200).expect("split");
        assert_eq!(fake.actions(), ["getOwnership", "setFrequency", "setSplit"]);
        assert_eq!(fake.requests()[1]["params"], json!({ "frequency": 7_074_500, "vfo": "B" }));
        assert_eq!(fake.requests()[2]["params"], json!({ "enabled": true, "txVfo": "B" }));
        assert!(wait(|| rig.status().split == Some(true)));
        // gia' a posto: nessun comando in piu'
        rig.set_tx_audio_frequency(2300).expect("split");
        assert_eq!(fake.requests().len(), 3);
        rig.shutdown();
        let last = fake.requests().last().cloned().expect("richiesta");
        assert_eq!((last["action"].as_str(), &last["params"]), (Some("setSplit"), &json!({ "enabled": false })));
    }

    #[test]
    fn without_set_split_the_split_is_emulated_during_tx() {
        let fake = fake(vec![radio("run", &["SetFrequency", "SetMode", "SetPtt"])], None);
        let rig = rig(&fake, |c| c.split = SplitMode::Rig);
        assert!(wait(|| rig.status().state == LinkState::Connected));
        assert_eq!(rig.split_mode(), SplitMode::Emulate, "la radio non dichiara SetSplit");
        rig.set_tx_audio_frequency(300).expect("frequenza audio");
        assert!(fake.requests().is_empty());
        rig.set_ptt(true).expect("PTT");
        rig.set_ptt(false).expect("PTT");
        let freq = |r: &Value| r["params"]["frequency"].as_u64();
        let seen = fake.requests();
        assert_eq!(fake.actions(), ["getOwnership", "setFrequency", "setPtt", "setPtt", "setFrequency"]);
        assert_eq!((freq(&seen[1]), freq(&seen[4])), (Some(7_072_500), Some(7_074_000)));
        assert!(wait(|| rig.frequency() == Ok(7_074_000)));
    }

    #[test]
    fn ownership_cooldown_is_waited_once_and_not_master_is_retried() {
        let fake = fake(vec![radio("run", &ALL)], None);
        fake.state.lock().unwrap().cooldown_once = true;
        let rig = rig(&fake, |_| {});
        assert!(wait(|| rig.status().state == LinkState::Connected));
        rig.set_frequency(14_074_000).expect("frequenza dopo l'attesa");
        assert_eq!(fake.actions(), ["getOwnership", "getOwnership", "setFrequency"]);
        // Log4OM riprende il comando senza che Aurora lo sappia ancora (nessun stateUpdate): NOT_MASTER e nuova richiesta
        fake.state.lock().unwrap().master = "log4om".into();
        rig.set_frequency(14_075_000).expect("frequenza");
        assert_eq!(fake.actions()[3..], ["setFrequency", "getOwnership", "setFrequency"]);
    }

    #[test]
    fn the_group_key_and_the_radio_choice() {
        let fake = fake(vec![radio("run", &ALL), radio("mult", &ALL)], Some("chiave-di-prova"));
        let wrong = rig(&fake, |c| c.cat4om_key = "sbagliata".into());
        assert!(wait(|| wrong.status().message == AUTH_FAILED), "{:?}", wrong.status());
        assert_eq!(wrong.status().state, LinkState::Error);
        wrong.shutdown();
        let missing = rig(&fake, |_| {});
        assert!(wait(|| missing.status().message == AUTH_REQUIRED), "{:?}", missing.status());
        missing.shutdown();
        let good = rig(&fake, |c| {
            c.cat4om_key = "chiave-di-prova".into();
            c.cat4om_radio = "mult".into();
        });
        assert!(wait(|| good.status().state == LinkState::Connected), "{:?}", good.status());
        assert_eq!(good.radio_id().as_deref(), Some("mult"));
        assert_eq!(good.radios().len(), 2);
        good.shutdown();
        assert!(fake.state.lock().unwrap().hellos.iter().all(|h| !h.to_string().contains("chiave-di-prova")), "la chiave non viaggia");
        let other = rig(&fake, |c| {
            c.cat4om_key = "chiave-di-prova".into();
            c.cat4om_radio = "nessuna".into();
        });
        assert!(wait(|| other.status().message.contains("non trovata")), "{:?}", other.status());
        assert!(other.set_frequency(14_074_000).is_err());
    }

    #[test]
    fn a_claimed_radio_and_a_reconnection() {
        let mut state = radio("run", &ALL);
        state["bridgeControlledBy"] = json!("hamlib");
        let fake = fake(vec![state], None);
        let rig = rig(&fake, |_| {});
        assert!(wait(|| rig.status().message.contains("emulazione hamlib")), "{:?}", rig.status());
        assert!(rig.set_ptt(true).is_err(), "radio presa dall'emulazione Hamlib di Cat4OM");
        // il profilo Hamlib si spegne: la radio torna comandabile
        {
            let mut s = fake.state.lock().unwrap();
            s.radios[0]["bridgeControlledBy"] = Value::Null;
            s.push = true;
        }
        assert!(wait(|| rig.status().state == LinkState::Connected));
        // il servizio cade e torna: nuovo collegamento, nuovo clientId
        fake.state.lock().unwrap().drop_now = true;
        assert!(wait(|| fake.state.lock().unwrap().connections == 2));
        assert!(wait(|| rig.status().state == LinkState::Connected), "{:?}", rig.status());
        rig.set_frequency(14_074_000).expect("frequenza dopo la riconnessione");
        assert_eq!(fake.requests()[0]["clientId"], "client-2");
    }

    // ------------------------------------------------ scoperta dalla porta di gestione

    #[test]
    fn groups_are_read_and_chosen_like_decodium() {
        // il caso di un utente: un gruppo, porta Control 5001 assegnata da Cat4OM, una Kenwood TS-990
        let status = json!({ "groups": [
            { "id": "group-1", "name": "Group 1", "controlPort": 5001, "autoStart": true, "isRunning": true, "radioCount": 1,
              "radios": [ { "radioId": "kenwood/ts-990", "radioName": "kenwood/ts-990", "connectionState": "connected" } ], "x-futuro": 1 },
            { "id": "spare", "name": "Riserva", "controlPort": 0, "isRunning": false, "radios": [] },
            { "name": "senza id" }
        ] });
        let groups = parse_groups(&status);
        assert_eq!(groups.len(), 2, "un gruppo senza id si scarta");
        assert_eq!((groups[0].control_port, groups[0].running), (Some(5001), true));
        assert_eq!((groups[1].control_port, groups[1].running, groups[1].name.as_str()), (None, false, "Riserva"));
        let found = choose_group("", "", &groups).expect("scoperta automatica");
        assert_eq!(found.id, "group-1");
        assert_eq!(control_address("127.0.0.1:17300", 5001), "127.0.0.1:5001", "la porta del gruppo, non la 17301 di una volta");
        assert_eq!(control_address("192.168.1.20:17300", 5002), "192.168.1.20:5002", "Cat4OM su un altro PC");
        assert_eq!(control_address("[::1]:17300", 5001), "[::1]:5001");
        assert_eq!(choose_group("GROUP 1", "", &groups).expect("per nome").id, "group-1");
        assert_eq!(choose_group("spare", "", &groups).expect("per id").id, "spare");
        assert_eq!(choose_group("", "KENWOOD/TS-990", &groups).expect("per radio").id, "group-1");
        assert_eq!(choose_group("zz", "", &groups), Err("Gruppo «zz» non trovato in Cat4OM (presenti: group-1, spare)".into()));
        assert_eq!(choose_group("", "", &[]), Err(NO_GROUPS.into()));
        assert_eq!(parse_groups(&json!({})), vec![]);
        assert_eq!(link_summary("group-1", "kenwood/ts-990", true), "group-1/kenwood/ts-990 — master");
        assert_eq!(link_summary("", "run", false), "run — slave");
    }

    /// Il finto servizio di gestione: `getServiceStatus` con i gruppi dati; `startGroup` avvia il gruppo sulla porta in `x-port`.
    struct FakeManagement {
        address: String,
        state: Arc<Mutex<ManagementState>>,
    }

    struct ManagementState {
        groups: Vec<Value>,
        actions: Vec<String>,
        connections: u32,
    }

    impl FakeManagement {
        fn actions(&self) -> Vec<String> {
            self.state.lock().unwrap().actions.clone()
        }

        fn connections(&self) -> u32 {
            self.state.lock().unwrap().connections
        }
    }

    fn fake_management(groups: Vec<Value>) -> FakeManagement {
        let listener = TcpListener::bind("127.0.0.1:0").expect("porta");
        let address = listener.local_addr().expect("indirizzo").to_string();
        let state = Arc::new(Mutex::new(ManagementState { groups, actions: Vec::new(), connections: 0 }));
        let shared = Arc::clone(&state);
        std::thread::spawn(move || {
            for stream in listener.incoming() {
                let Ok(stream) = stream else { continue };
                let Ok(mut ws) = tungstenite::accept(stream) else { continue };
                let Ok(Message::Text(hello)) = ws.read() else { continue };
                let hello: Value = serde_json::from_str(hello.as_str()).unwrap_or_default();
                assert!(hello.get("auth").is_none(), "alla gestione non va la chiave del gruppo");
                shared.lock().unwrap().connections += 1;
                let welcome = json!({ "type": "managementWelcome", "endpoint": "management", "protocolVersion": "1.0.0", "clientId": "m-1",
                    "serviceVersion": "0.12.6.0", "authRequired": false, "keepaliveIntervalMs": 15000 });
                if ws.send(Message::text(welcome.to_string())).is_err() {
                    continue;
                }
                while let Ok(Message::Text(t)) = ws.read() {
                    let r: Value = serde_json::from_str(t.as_str()).unwrap_or_default();
                    let mut s = shared.lock().unwrap();
                    let action = r["action"].as_str().unwrap_or_default().to_string();
                    s.actions.push(action.clone());
                    let result = match action.as_str() {
                        "getServiceStatus" => json!({ "groups": s.groups }),
                        "startGroup" => {
                            if let Some(g) = s.groups.iter_mut().find(|g| g["id"] == r["params"]["groupId"]) {
                                g["isRunning"] = json!(true);
                                g["controlPort"] = g["x-port"].clone();
                            }
                            Value::Null
                        }
                        _ => Value::Null,
                    };
                    // un push prima della risposta: va ignorato
                    let _ = ws.send(Message::text(json!({ "type": "serviceStatus", "isKeepAlive": true }).to_string()));
                    let _ = ws.send(Message::text(json!({ "type": "response", "id": r["id"], "success": true, "result": result }).to_string()));
                }
            }
        });
        FakeManagement { address, state }
    }

    fn port_of(address: &str) -> u16 {
        address.rsplit_once(':').and_then(|(_, p)| p.parse().ok()).expect("porta")
    }

    fn group(id: &str, port: Option<u16>, radios: &[&str]) -> Value {
        json!({ "id": id, "name": id, "controlPort": port, "isRunning": port.is_some(), "x-port": port,
            "radios": radios.iter().map(|r| json!({ "radioId": r, "radioName": r, "connectionState": "connected" })).collect::<Vec<_>>() })
    }

    fn discovering(management: &str, edit: impl FnOnce(&mut RigConfig)) -> Cat4omRig {
        let mut config = RigConfig { cat4om_management: management.to_string(), ..RigConfig::default() };
        edit(&mut config);
        Cat4omRig::with_timing(&config, timing())
    }

    #[test]
    fn discovery_finds_the_group_control_port_like_decodium() {
        let control = fake(vec![radio("kenwood/ts-990", &ALL)], None);
        control.state.lock().unwrap().group = "group-1".into();
        let management = fake_management(vec![group("group-1", Some(port_of(&control.address)), &["kenwood/ts-990"])]);
        let rig = discovering(&management.address, |_| {});
        assert!(wait(|| rig.status().state == LinkState::Connected), "{:?}", rig.status());
        assert_eq!(rig.status().message, "group-1/kenwood/ts-990 — slave", "lo stato come Decodium");
        assert_eq!(management.actions(), ["getServiceStatus"]);
        let endpoint = rig.endpoint();
        assert_eq!((endpoint.control.as_str(), endpoint.discovered, endpoint.group.as_str()), (control.address.as_str(), true, "group-1"));
        assert_eq!((endpoint.radio.as_str(), endpoint.role.as_str()), ("kenwood/ts-990", "slave"));
        rig.set_frequency(5_357_000).expect("frequenza");
        assert!(wait(|| rig.status().message == "group-1/kenwood/ts-990 — master"), "{:?}", rig.status());
        // Cat4OM riparte: la riconnessione rifa' la scoperta (la porta puo' essere cambiata)
        control.state.lock().unwrap().drop_now = true;
        assert!(wait(|| control.state.lock().unwrap().connections == 2));
        assert!(wait(|| rig.status().state == LinkState::Connected), "{:?}", rig.status());
        assert_eq!(management.connections(), 2, "nuova scoperta a ogni collegamento");
    }

    #[test]
    fn a_written_port_is_honoured_and_the_management_port_written_there_discovers() {
        let control = fake(vec![radio("run", &ALL)], None);
        let management = fake_management(vec![group("main", Some(port_of(&control.address)), &["run"])]);
        // la porta scritta a mano si usa senza chiedere alla gestione (il vecchio 17301 lo toglie solo l'apertura dei file vecchi)
        let written = discovering(&management.address, |c| c.cat4om_address = control.address.clone());
        assert!(wait(|| written.status().state == LinkState::Connected), "{:?}", written.status());
        assert_eq!((written.address(), written.endpoint().discovered), (control.address.clone(), false));
        assert_eq!(management.connections(), 0, "la gestione non si tocca");
        written.shutdown();
        // nel campo «Controllo» la porta di gestione (un utente aveva provato 17300): Aurora scopre da li'
        let closed = TcpListener::bind("127.0.0.1:0").expect("porta");
        let nobody = closed.local_addr().expect("indirizzo").to_string();
        drop(closed);
        let misplaced = discovering(&nobody, |c| c.cat4om_address = management.address.clone());
        assert!(wait(|| misplaced.status().state == LinkState::Connected), "{:?}", misplaced.status());
        assert_eq!(misplaced.address(), control.address);
    }

    #[test]
    fn a_stopped_group_is_started_and_a_wrong_group_says_so() {
        let control = fake(vec![radio("run", &ALL)], None);
        let mut stopped = group("main", None, &["run"]);
        stopped["x-port"] = json!(port_of(&control.address));
        let management = fake_management(vec![group("other", None, &[]), stopped]);
        let rig = discovering(&management.address, |c| c.cat4om_radio = "run".into());
        assert!(wait(|| rig.status().state == LinkState::Connected), "{:?}", rig.status());
        assert_eq!(management.actions()[..3], ["getServiceStatus", "startGroup", "getServiceStatus"], "il gruppo con la radio, avviato");
        rig.shutdown();
        let wrong = discovering(&management.address, |c| c.cat4om_group = "group-9".into());
        assert!(wait(|| wrong.status().message == "Gruppo «group-9» non trovato in Cat4OM (presenti: other, main)"), "{:?}", wrong.status());
        assert_eq!(wrong.status().state, LinkState::Error);
        assert!(wrong.set_frequency(14_074_000).is_err());
    }

    #[test]
    fn an_unreachable_management_port_says_cat4om_is_not_running() {
        let listener = TcpListener::bind("127.0.0.1:0").expect("porta");
        let address = listener.local_addr().expect("indirizzo").to_string();
        drop(listener);
        let rig = discovering(&address, |_| {});
        assert!(wait(|| rig.status().message.starts_with(&format!("Gestione di Cat4OM non raggiungibile su {address}: avvia Log4OM e Cat4OM"))), "{:?}", rig.status());
        assert_eq!(rig.status().state, LinkState::Error);
        assert_eq!(rig.address(), "", "nessuna porta Control scoperta");
        // un Control al posto della gestione
        let control = fake(vec![radio("run", &ALL)], None);
        let swapped = discovering(&control.address, |_| {});
        assert!(wait(|| swapped.status().message.starts_with(&format!("Su {} non risponde la gestione di Cat4OM", control.address))), "{:?}", swapped.status());
    }

    #[test]
    fn an_unreachable_service_says_so() {
        let listener = TcpListener::bind("127.0.0.1:0").expect("porta");
        let address = listener.local_addr().expect("indirizzo").to_string();
        drop(listener);
        let rig = Cat4omRig::with_timing(&RigConfig { cat4om_address: address.clone(), ..RigConfig::default() }, timing());
        assert!(wait(|| rig.status().message.starts_with(&format!("Cat4OM non raggiungibile su {address}"))), "{:?}", rig.status());
        assert!(rig.frequency().is_err());
        assert!(rig.set_ptt(false).is_ok(), "lo spegnimento del PTT non fallisce a collegamento assente");
    }
}
