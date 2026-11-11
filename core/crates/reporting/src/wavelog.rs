//! Invio diretto a Wavelog o Cloudlog (stessa API, ereditata da Cloudlog).
//!
//! `POST {indirizzo}/index.php/api/qso` con corpo JSON `{"key", "station_profile_id", "type": "adif", "string"}` (documentazione delle due API e
//! sorgente di `application/controllers/Api.php`). Risposte lette dal sorgente:
//! - 201 `{"status": "created", ...}`: registrato (Cloudlog mette i messaggi dell'importazione, per esempio i doppioni, in `messages`);
//! - 400 `{"status": "abort", "messages": [...]}` (Wavelog): non registrato, i doppioni hanno «Duplicate» nel messaggio;
//! - 401 chiave mancante o sbagliata, o stazione di un altro utente; 403 chiave senza permesso di scrittura (Wavelog);
//! - 429 troppe richieste (limite di Wavelog).
//!
//! La prova (come «Check Connection» di WSJT-X 3.2, `Network/Cloudlog.cpp`, GPL-3.0) non scrive niente nel log:
//! 1. `GET {indirizzo}/index.php/api/auth/{chiave}` (XML `<status>`, `<rights>`): chiave valida e con permesso di scrittura (`rw`);
//!    un server che non ha questa pagina (404) passa al punto 2;
//! 2. `GET {indirizzo}/index.php/api/station_info/{chiave}`: le stazioni della chiave, lette dal server, con la scelta da fare se il
//!    numero scritto non c'e'.

use std::time::Duration;

use serde_json::{Value, json};

use crate::upload::{Outcome, agent, network, short};

/// L'indirizzo di base: senza `/` finale e senza `/index.php` (si accettano entrambe le forme). `None` se non e' un indirizzo web.
pub fn api_base(url: &str) -> Option<String> {
    let url = url.trim().trim_end_matches('/');
    let url = url.strip_suffix("/index.php").unwrap_or(url).trim_end_matches('/');
    let lower = url.to_ascii_lowercase();
    let rest = lower.strip_prefix("https://").or_else(|| lower.strip_prefix("http://"))?;
    (!rest.is_empty() && !url.chars().any(|c| c.is_whitespace() || c.is_control())).then(|| url.to_string())
}

/// `true` se l'indirizzo e' `http://` verso un server fuori dalla rete di casa: la chiave API viaggerebbe in chiaro su Internet. Per `localhost`, indirizzi
/// privati (10.x, 192.168.x, 172.16-31.x, 127.x, 169.254.x, 100.64-127.x di Tailscale e degli operatori), IPv6, nomi senza punto e nomi di rete
/// locale (`.local`, `.lan`, `.home`, `.home.arpa`, `.internal`) non c'e' nulla da dire.
pub fn insecure_remote(base: &str) -> bool {
    let lower = base.to_ascii_lowercase();
    let Some(rest) = lower.strip_prefix("http://") else { return false };
    let host = rest.split(['/', ':']).next().unwrap_or("");
    let second = |prefix: &str| host.strip_prefix(prefix).and_then(|r| r.split('.').next()).and_then(|n| n.parse::<u8>().ok());
    let private = host == "localhost"
        || host.starts_with('[')
        || !host.contains('.')
        || [".local", ".lan", ".home", ".home.arpa", ".internal"].iter().any(|s| host.ends_with(s))
        || ["127.", "10.", "192.168.", "169.254."].iter().any(|p| host.starts_with(p))
        || second("172.").is_some_and(|n| (16..=31).contains(&n))
        || second("100.").is_some_and(|n| (64..=127).contains(&n));
    !private
}

/// L'avviso sull'indirizzo `http://` si scrive una volta per avvio, non a ogni QSO.
static INSECURE_WARNED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

pub fn qso_url(base: &str) -> String {
    format!("{base}/index.php/api/qso")
}

/// Il corpo JSON della richiesta (porta la chiave: mai nel registro).
pub fn body(key: &str, station_id: &str, record: &str) -> String {
    json!({ "key": key, "station_profile_id": station_id, "type": "adif", "string": record }).to_string()
}

fn messages(v: &Value) -> String {
    v.get("messages").and_then(Value::as_array).map(|m| m.iter().filter_map(Value::as_str).filter(|s| !s.trim().is_empty()).collect::<Vec<_>>().join(" ")).unwrap_or_default()
}

/// Come e' andata, dalla risposta.
pub fn read_reply(status: u16, body: &str) -> Outcome {
    let v: Value = serde_json::from_str(body).unwrap_or(Value::Null);
    let reason = v.get("reason").and_then(Value::as_str).unwrap_or("").to_string();
    let msgs = short(&messages(&v), 120);
    let duplicate = msgs.to_lowercase().contains("duplicate");
    match status {
        200 | 201 if v.get("status").and_then(Value::as_str) == Some("created") => {
            // Cloudlog: `imported_count` 0 con un messaggio = non importato
            let none = v.get("imported_count").and_then(Value::as_u64) == Some(0);
            if duplicate {
                Outcome::Duplicate(msgs)
            } else if none && !msgs.is_empty() {
                Outcome::Rejected(msgs)
            } else {
                Outcome::Sent(String::new())
            }
        }
        400 if duplicate => Outcome::Duplicate(msgs),
        401 | 403 => Outcome::Blocked(format!("Wavelog/Cloudlog ha rifiutato la chiave o la stazione ({})", if reason.is_empty() { status.to_string() } else { reason })),
        404 => Outcome::Blocked("indirizzo dell'API non trovato (controlla l'indirizzo di Wavelog/Cloudlog)".into()),
        408 | 429 | 500..=599 => Outcome::Retry(format!("Wavelog/Cloudlog ha risposto {status}")),
        _ => Outcome::Rejected(if !msgs.is_empty() { msgs } else if !reason.is_empty() { reason } else { format!("risposta {status}") }),
    }
}

/// Manda un QSO (bloccante).
pub fn upload(url: &str, key: &str, station_id: &str, record: &str, timeout: Duration, user_agent: &str) -> Outcome {
    let Some(base) = api_base(url) else { return Outcome::Blocked("indirizzo di Wavelog/Cloudlog non valido".into()) };
    if insecure_remote(&base) && !INSECURE_WARNED.swap(true, std::sync::atomic::Ordering::Relaxed) {
        tracing::warn!("Wavelog/Cloudlog: l'indirizzo e' http:// verso Internet, la chiave API viaggia in chiaro: meglio https://");
    }
    let request = agent(timeout, user_agent).post(qso_url(&base)).header("Content-Type", "application/json").header("Accept", "application/json");
    match request.send(body(key, station_id, record)) {
        Ok(mut response) => {
            let status = response.status().as_u16();
            let text = response.body_mut().read_to_string().unwrap_or_default();
            read_reply(status, &text)
        }
        Err(e) => network(&e),
    }
}

/// Una stazione della chiave, come la elenca `station_info`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Station {
    pub id: String,
    pub name: String,
    pub call: String,
}

/// Le stazioni lette dalla risposta di `station_info` (numero anche come `station_profile_id`, come accetta WSJT-X 3.2).
pub fn parse_stations(body: &str) -> Result<Vec<Station>, String> {
    let v: Value = serde_json::from_str(body).map_err(|_| "risposta non leggibile (e' davvero Wavelog o Cloudlog?)".to_string())?;
    let list = v.as_array().ok_or_else(|| v.get("reason").and_then(Value::as_str).unwrap_or("risposta inattesa").to_string())?;
    let field = |s: &Value, keys: &[&str]| {
        keys.iter()
            .find_map(|k| match s.get(*k) {
                Some(Value::String(t)) if !t.is_empty() => Some(t.clone()),
                Some(Value::Number(n)) => Some(n.to_string()),
                _ => None,
            })
            .unwrap_or_default()
    };
    Ok(list
        .iter()
        .map(|s| Station { id: field(s, &["station_id", "station_profile_id"]), name: field(s, &["station_profile_name", "profile_name", "name"]), call: field(s, &["station_callsign", "callsign"]) })
        .filter(|s| !s.id.is_empty())
        .collect())
}

/// Dalla risposta di `station_info`: la stazione con quel numero (nome e nominativo), se c'e'. Altrimenti l'elenco delle stazioni del
/// server tra cui scegliere.
pub fn find_station(body: &str, station_id: &str) -> Result<String, String> {
    let list = parse_stations(body)?;
    let choices = || list.iter().map(|s| format!("{} = {} ({})", s.id, s.name, s.call)).collect::<Vec<_>>().join("; ");
    if list.is_empty() {
        return Err("la chiave non ha nessuna stazione: creane una in Wavelog/Cloudlog".into());
    }
    if station_id.trim().is_empty() {
        return Err(format!("scegli il numero della stazione tra quelle del server: {}", choices()));
    }
    let station = list.iter().find(|s| s.id == station_id.trim()).ok_or_else(|| format!("la stazione {station_id} non e' tra quelle della chiave; scegli tra: {}", choices()))?;
    let others = if list.len() > 1 { format!("; altre stazioni: {}", choices()) } else { String::new() };
    Ok(format!("chiave valida con scrittura, stazione {station_id}: {} ({}){others}", station.name, station.call))
}

/// Il permesso della chiave dalla risposta XML di `api/auth` (`<status>Valid</status><rights>rw</rights>`).
pub fn check_rights(body: &str) -> Result<(), String> {
    let tag = |name: &str| {
        let open = format!("<{name}>");
        let start = body.find(&open)? + open.len();
        let end = body[start..].find('<')? + start;
        Some(body[start..end].trim().to_ascii_lowercase())
    };
    match (tag("status").as_deref(), tag("rights")) {
        (Some("valid"), Some(r)) if r.starts_with("rw") => Ok(()),
        (Some("valid"), Some(r)) if r.starts_with('r') => Err("la chiave API e' di sola lettura: in Wavelog/Cloudlog creane una con scrittura (rw)".into()),
        (Some("valid"), _) => Err("il server non dice i permessi della chiave".into()),
        _ => Err("chiave API non valida".into()),
    }
}

/// Una pagina dell'API con la chiave nel percorso (`{indirizzo}/index.php/api/{pagina}/{chiave}`): stato HTTP e testo.
fn get_page(url: &str, key: &str, page: &str, timeout: Duration, user_agent: &str) -> Result<(u16, String), String> {
    let base = api_base(url).ok_or("indirizzo non valido: serve http:// o https://")?;
    let path_key: String = key.chars().filter(char::is_ascii_alphanumeric).collect();
    if path_key.is_empty() || path_key.len() != key.len() {
        return Err("la chiave API contiene caratteri non previsti".into());
    }
    match agent(timeout, user_agent).get(format!("{base}/index.php/api/{page}/{path_key}")).call() {
        Ok(mut response) => Ok((response.status().as_u16(), response.body_mut().read_to_string().unwrap_or_default())),
        Err(e) => Err(crate::eqsl::describe(&e)),
    }
}

/// La risposta di `station_info` se e' andata bene, altrimenti il motivo.
fn station_info(url: &str, key: &str, timeout: Duration, user_agent: &str) -> Result<String, String> {
    match get_page(url, key, "station_info", timeout, user_agent)? {
        (200, text) => Ok(text),
        (401 | 403, _) => Err("chiave API non valida".into()),
        (404, _) => Err("indirizzo dell'API non trovato".into()),
        (s, _) => Err(format!("risposta {s}")),
    }
}

/// Le stazioni della chiave lette dal server (bloccante): servono alla tendina della stazione nelle Impostazioni.
pub fn stations(url: &str, key: &str, timeout: Duration, user_agent: &str) -> Result<Vec<Station>, String> {
    parse_stations(&station_info(url, key, timeout, user_agent)?)
}

/// La prova (bloccante): chiave, permesso di scrittura e stazione, senza scrivere nel log.
pub fn test(url: &str, key: &str, station_id: &str, timeout: Duration, user_agent: &str) -> Result<String, String> {
    match get_page(url, key, "auth", timeout, user_agent)? {
        (200, text) => check_rights(&text)?,
        (401 | 403, _) => return Err("chiave API non valida".into()),
        // server senza la pagina dei permessi: si controlla solo la stazione
        _ => {}
    }
    find_station(&station_info(url, key, timeout, user_agent)?, station_id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_address_is_accepted_with_or_without_index_php() {
        assert_eq!(api_base("https://log.example.com/").as_deref(), Some("https://log.example.com"));
        assert_eq!(api_base(" https://log.example.com/wavelog/index.php ").as_deref(), Some("https://log.example.com/wavelog"));
        assert_eq!(api_base("http://192.168.1.5/index.php/").as_deref(), Some("http://192.168.1.5"));
        assert_eq!(api_base("log.example.com"), None);
        assert!(insecure_remote("http://log.example.com"));
        assert!(!insecure_remote("https://log.example.com"));
        assert!(!insecure_remote("http://192.168.1.5"));
        assert!(!insecure_remote("http://172.20.0.3:8080/wavelog"));
        assert!(insecure_remote("http://172.40.0.3"));
        assert!(!insecure_remote("http://wavelog"));
        assert!(!insecure_remote("http://wavelog.lan/index.php"));
        assert!(!insecure_remote("http://nas.home.arpa"));
        assert!(!insecure_remote("http://100.101.1.2"));
        assert!(!insecure_remote("http://[fd00::5]:8080"));
        assert!(insecure_remote("http://100.200.1.2"));
        assert_eq!(api_base("https://"), None);
        assert_eq!(qso_url("https://log.example.com"), "https://log.example.com/index.php/api/qso");
    }

    #[test]
    fn the_body_is_the_documented_json() {
        let v: Value = serde_json::from_str(&body("abc", "3", "<CALL:5>K1ABC <EOR>")).unwrap();
        assert_eq!(v, json!({ "key": "abc", "station_profile_id": "3", "type": "adif", "string": "<CALL:5>K1ABC <EOR>" }));
    }

    #[test]
    fn replies_of_wavelog_and_cloudlog_are_understood() {
        assert_eq!(read_reply(201, r#"{"status":"created","type":"adif","string":"","adif_count":1,"adif_errors":0,"messages":[""]}"#), Outcome::Sent(String::new()));
        assert_eq!(read_reply(201, r#"{"status":"created","imported_count":1,"messages":[]}"#), Outcome::Sent(String::new()));
        assert!(matches!(read_reply(201, r#"{"status":"created","imported_count":0,"messages":["Date/Time: 2026-10-24 16:46:45 Callsign: K1ABC Band: 20m  Duplicate<br>"]}"#), Outcome::Duplicate(_)));
        assert!(matches!(read_reply(400, r#"{"status":"abort","messages":["","K1ABC Duplicate"]}"#), Outcome::Duplicate(_)));
        assert!(matches!(read_reply(400, r#"{"status":"abort","messages":["","Wrong band"]}"#), Outcome::Rejected(m) if m.contains("Wrong band")));
        assert!(matches!(read_reply(401, r#"{"status":"failed","reason":"missing or wrong api key"}"#), Outcome::Blocked(m) if m.contains("wrong api key")));
        assert!(matches!(read_reply(403, r#"{"status":"failed","reason":"API key does not have write permissions"}"#), Outcome::Blocked(_)));
        assert!(matches!(read_reply(429, ""), Outcome::Retry(_)));
        assert!(matches!(read_reply(503, "<html>"), Outcome::Retry(_)));
        assert!(matches!(read_reply(404, "not found"), Outcome::Blocked(_)));
    }

    #[test]
    fn the_test_finds_the_station_among_those_of_the_key() {
        let body = r#"[{"station_id":"1","station_profile_name":"Casa","station_callsign":"K1ABC"},{"station_id":2,"station_profile_name":"Portatile","station_callsign":"K1ABC/P"}]"#;
        assert!(find_station(body, "2").unwrap().starts_with("chiave valida con scrittura, stazione 2: Portatile (K1ABC/P); altre stazioni:"));
        assert!(find_station(body, "1").unwrap().contains("Casa"));
        let wrong = find_station(body, "7").unwrap_err();
        assert!(wrong.contains("1 = Casa (K1ABC)") && wrong.contains("2 = Portatile (K1ABC/P)"), "{wrong}");
        assert!(find_station(body, "").unwrap_err().starts_with("scegli il numero"));
        assert!(find_station("[]", "1").unwrap_err().contains("nessuna stazione"));
        assert!(find_station("<html>", "1").is_err());
        let alt = r#"[{"station_profile_id":5,"profile_name":"Club","callsign":"K1XYZ"}]"#;
        assert_eq!(parse_stations(alt).unwrap(), vec![Station { id: "5".into(), name: "Club".into(), call: "K1XYZ".into() }]);
    }

    #[test]
    fn the_rights_of_the_key_must_include_writing() {
        assert!(check_rights("<?xml version=\"1.0\"?><auth><status>Valid</status><rights>rw</rights></auth>").is_ok());
        assert!(check_rights("<auth><status>Valid</status><rights>r</rights></auth>").unwrap_err().contains("sola lettura"));
        assert!(check_rights("<auth><message>Key Invalid - either not found or disabled</message></auth>").unwrap_err().contains("non valida"));
        assert!(check_rights("<auth><status>Valid</status></auth>").is_err());
    }
}
