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

/// I token dell'API v2 di Wavelog cominciano con `wl2_` (`usesApiV2` di `Cloudlog.cpp`, WSJT-X 3.3): vanno nell'intestazione `Authorization: Bearer`
/// e il QSO si manda a `/index.php/api/v2/qso`. Prima Aurora li usava come chiavi vecchie e l'invio veniva rifiutato.
pub fn uses_api_v2(key: &str) -> bool {
    key.trim().starts_with("wl2_")
}

pub fn v2_qso_url(base: &str) -> String {
    format!("{base}/index.php/api/v2/qso")
}

/// Il corpo JSON del QSO per l'API v2 (`v2QsoPayload`): `import_type`, `station_profile_id` numerico, `adif` chiuso da `<eor>`, `dryrun` per la prova.
pub fn v2_body(station_id: &str, record: &str, dry_run: bool) -> Result<String, String> {
    let id: i64 = station_id.trim().parse().map_err(|_| format!("numero di stazione non valido: «{station_id}»"))?;
    let mut adif = record.trim().to_string();
    if !adif.to_ascii_lowercase().ends_with("<eor>") {
        adif.push_str("<eor>");
    }
    let mut body = json!({ "import_type": "adif", "station_profile_id": id, "adif": adif });
    if dry_run {
        body["dryrun"] = json!(true);
    }
    Ok(body.to_string())
}

fn v2_detail(v: &Value, fallback: String) -> String {
    let error = v.get("error");
    let message = error.and_then(|e| e.get("message")).and_then(Value::as_str).map(str::trim).unwrap_or("");
    let details = error.and_then(|e| e.get("details")).map(|d| d.as_str().map_or_else(|| d.to_string(), str::to_string)).unwrap_or_default();
    match (message.is_empty(), details.trim().is_empty()) {
        (false, false) => short(&format!("{message} {details}"), 160),
        (false, true) => short(message, 160),
        (true, false) => short(&details, 160),
        (true, true) => fallback,
    }
}

/// Come e' andata con l'API v2 (`classifyV2QsoUploadReply`): 201 con `data.parsed` e almeno un QSO importato o saltato.
pub fn read_v2_reply(status: u16, body: &str) -> Outcome {
    let v: Value = serde_json::from_str(body).unwrap_or(Value::Null);
    let count = |k: &str| v.get("data").and_then(|d| d.get(k)).and_then(Value::as_u64).unwrap_or(0);
    match status {
        201 if count("parsed") > 0 && count("imported") > 0 => Outcome::Sent(String::new()),
        201 if count("parsed") > 0 && count("skipped") > 0 => Outcome::Duplicate("gia' nel log".into()),
        401 | 403 => Outcome::Rejected(v2_detail(&v, "token API v2 non valido o senza permesso di scrittura".into())),
        408 | 429 | 500..=599 => Outcome::Retry(format!("Wavelog ha risposto {status}")),
        _ => Outcome::Rejected(v2_detail(&v, format!("risposta {status} inattesa dall'API v2"))),
    }
}

fn v2_request(base: &str, key: &str, timeout: Duration, user_agent: &str) -> ureq::RequestBuilder<ureq::typestate::WithBody> {
    agent(timeout, user_agent)
        .post(v2_qso_url(base))
        .header("Content-Type", "application/json")
        .header("Accept", "application/json")
        .header("Authorization", &format!("Bearer {}", key.trim()))
}

/// Manda un QSO (bloccante).
pub fn upload(url: &str, key: &str, station_id: &str, record: &str, timeout: Duration, user_agent: &str) -> Outcome {
    let Some(base) = api_base(url) else { return Outcome::Blocked("indirizzo di Wavelog/Cloudlog non valido".into()) };
    if insecure_remote(&base) && !INSECURE_WARNED.swap(true, std::sync::atomic::Ordering::Relaxed) {
        tracing::warn!("Wavelog/Cloudlog: l'indirizzo e' http:// verso Internet, la chiave API viaggia in chiaro: meglio https://");
    }
    if uses_api_v2(key) {
        let data = match v2_body(station_id, record, false) {
            Ok(d) => d,
            Err(e) => return Outcome::Blocked(e),
        };
        return match v2_request(&base, key, timeout, user_agent).send(data) {
            Ok(mut response) => {
                let status = response.status().as_u16();
                let text = response.body_mut().read_to_string().unwrap_or_default();
                read_v2_reply(status, &text)
            }
            Err(e) => network(&e),
        };
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

/// Le stazioni della chiave lette dal server (bloccante): servono alla tendina della stazione nelle Impostazioni. Con un token v2 (`wl2_...`)
/// si chiede `GET /index.php/api/v2/station` con `Authorization: Bearer`, come `startV2Station` di `Cloudlog.cpp` (WSJT-X 3.3): la vecchia
/// `station_info/{chiave}` non conosce i token v2.
pub fn stations(url: &str, key: &str, timeout: Duration, user_agent: &str) -> Result<Vec<Station>, String> {
    if uses_api_v2(key) {
        return v2_stations(url, key, timeout, user_agent);
    }
    parse_stations(&station_info(url, key, timeout, user_agent)?)
}

fn v2_stations(url: &str, key: &str, timeout: Duration, user_agent: &str) -> Result<Vec<Station>, String> {
    let base = api_base(url).ok_or("indirizzo non valido: serve http:// o https://")?;
    let request = agent(timeout, user_agent).get(format!("{base}/index.php/api/v2/station")).header("Accept", "application/json").header("Authorization", &format!("Bearer {}", key.trim()));
    let mut response = request.call().map_err(|e| crate::eqsl::describe(&e))?;
    let status = response.status().as_u16();
    let text = response.body_mut().read_to_string().unwrap_or_default();
    read_v2_stations(status, &text)
}

/// Le stazioni dalla risposta di `api/v2/station` (`reply_v2_station` e `collectStationProfiles` di WSJT-X 3.3): in `data`, un elenco o un
/// oggetto con `stations`/`station_profiles`/`stationProfiles`/`profiles`; numero da `station_id`, `station_profile_id` o `id`.
pub fn read_v2_stations(status: u16, body: &str) -> Result<Vec<Station>, String> {
    let v: Value = serde_json::from_str(body).unwrap_or(Value::Null);
    match status {
        200 if v.is_object() => {}
        401 => return Err(v2_detail(&v, "token API v2 non valido".into())),
        403 => return Err(v2_detail(&v, "il token API v2 non ha il permesso di leggere le stazioni".into())),
        _ => return Err(v2_detail(&v, format!("risposta {status} inattesa dall'API v2"))),
    }
    let text = |o: &Value, keys: &[&str]| {
        keys.iter()
            .find_map(|k| match o.get(*k) {
                Some(Value::String(t)) if !t.trim().is_empty() => Some(t.trim().to_string()),
                Some(Value::Number(n)) => Some(n.to_string()),
                _ => None,
            })
            .unwrap_or_default()
    };
    fn collect(value: &Value, out: &mut Vec<Station>, text: &dyn Fn(&Value, &[&str]) -> String) {
        match value {
            Value::Array(list) => list.iter().for_each(|e| collect(e, out, text)),
            Value::Object(o) => {
                let id = text(value, &["station_id", "station_profile_id", "id"]);
                if !id.is_empty() && id != "0" {
                    out.push(Station { id, name: text(value, &["station_profile_name", "profile_name", "name"]), call: text(value, &["station_callsign", "callsign", "call"]) });
                    return;
                }
                for k in ["stations", "station_profiles", "stationProfiles", "profiles"] {
                    if let Some(child) = o.get(k) {
                        collect(child, out, text);
                    }
                }
            }
            _ => {}
        }
    }
    let mut out = Vec::new();
    if let Some(data) = v.get("data") {
        collect(data, &mut out, &text);
    }
    Ok(out)
}

/// La prova (bloccante): chiave, permesso di scrittura e stazione, senza scrivere nel log.
pub fn test(url: &str, key: &str, station_id: &str, timeout: Duration, user_agent: &str) -> Result<String, String> {
    if uses_api_v2(key) {
        return test_v2(url, key, station_id, timeout, user_agent);
    }
    match get_page(url, key, "auth", timeout, user_agent)? {
        (200, text) => check_rights(&text)?,
        (401 | 403, _) => return Err("chiave API non valida".into()),
        // server senza la pagina dei permessi: si controlla solo la stazione
        _ => {}
    }
    find_station(&station_info(url, key, timeout, user_agent)?, station_id)
}

/// La prova con un token v2: un QSO di prova in «dry run» (non scrive nel log), come `checkConnection` di WSJT-X 3.3.
fn test_v2(url: &str, key: &str, station_id: &str, timeout: Duration, user_agent: &str) -> Result<String, String> {
    let base = api_base(url).ok_or("indirizzo non valido: serve http:// o https://")?;
    let days = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_secs()) / 86_400;
    let (y, m, d) = civil_from_days(days as i64);
    let record = format!("<call:5>WSJTX<qso_date:8>{y:04}{m:02}{d:02}<time_on:6>000000<band:3>20m<mode:3>FT8");
    let data = v2_body(station_id, &record, true)?;
    let mut response = v2_request(&base, key, timeout, user_agent).send(data).map_err(|e| crate::eqsl::describe(&e))?;
    let status = response.status().as_u16();
    let text = response.body_mut().read_to_string().unwrap_or_default();
    let v: Value = serde_json::from_str(&text).unwrap_or(Value::Null);
    let data = v.get("data");
    let dry = data.and_then(|d| d.get("dryrun")).and_then(Value::as_bool).unwrap_or(false);
    let parsed = data.and_then(|d| d.get("parsed")).and_then(Value::as_u64).unwrap_or(0);
    match status {
        200 if dry && parsed > 0 => Ok(format!("token API v2 valido, stazione {} accettata (prova senza scrivere nel log)", station_id.trim())),
        401 | 403 => Err(v2_detail(&v, "token API v2 non valido o senza permesso".into())),
        _ => Err(v2_detail(&v, format!("la prova con l'API v2 non e' riuscita (risposta {status})"))),
    }
}

/// Anno, mese e giorno da giorni dal 1970 (algoritmo di Howard Hinnant, calendario gregoriano).
fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (yoe + era * 400 + i64::from(m <= 2), m, d)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn v2_tokens_use_the_v2_body_and_reply() {
        assert!(uses_api_v2("wl2_abc") && !uses_api_v2("abc123"));
        let b: Value = serde_json::from_str(&v2_body("3", "<call:5>K1ABC<eor>", false).expect("corpo")).expect("json");
        assert_eq!((b["import_type"].as_str(), b["station_profile_id"].as_i64(), b["adif"].as_str()), (Some("adif"), Some(3), Some("<call:5>K1ABC<eor>")));
        assert!(b.get("dryrun").is_none());
        let dry: Value = serde_json::from_str(&v2_body("3", "<call:5>K1ABC", true).expect("corpo")).expect("json");
        assert_eq!((dry["adif"].as_str(), dry["dryrun"].as_bool()), (Some("<call:5>K1ABC<eor>"), Some(true)));
        assert!(v2_body("abc", "x", false).is_err());
        assert!(matches!(read_v2_reply(201, r#"{"data":{"parsed":1,"imported":1,"skipped":0}}"#), Outcome::Sent(_)));
        assert!(matches!(read_v2_reply(201, r#"{"data":{"parsed":1,"imported":0,"skipped":1}}"#), Outcome::Duplicate(_)));
        assert!(matches!(read_v2_reply(401, r#"{"error":{"message":"bad token"}}"#), Outcome::Rejected(m) if m == "bad token"));
        assert!(matches!(read_v2_reply(503, ""), Outcome::Retry(_)));
        assert_eq!(civil_from_days(20_403), (2025, 11, 11));
    }

    #[test]
    fn v2_station_list_is_read_like_wsjtx() {
        let flat = r#"{"data":[{"station_id":3,"station_profile_name":"Casa","station_callsign":"IU3ABC"},{"id":"5","name":"Portatile","callsign":"IU3ABC/P"}]}"#;
        let list = read_v2_stations(200, flat).expect("elenco");
        assert_eq!(list, vec![
            Station { id: "3".into(), name: "Casa".into(), call: "IU3ABC".into() },
            Station { id: "5".into(), name: "Portatile".into(), call: "IU3ABC/P".into() },
        ]);
        let nested = r#"{"data":{"stations":[{"station_profile_id":7,"profile_name":"Club","call":"IU3ABC"}]}}"#;
        assert_eq!(read_v2_stations(200, nested).expect("annidato")[0].id, "7");
        assert!(read_v2_stations(401, r#"{"error":{"message":"bad token"}}"#).expect_err("rifiutato").contains("bad token"));
        assert!(read_v2_stations(500, "").is_err());
    }

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
