//! Invio diretto a Wavelog o Cloudlog (stessa API, ereditata da Cloudlog).
//!
//! `POST {indirizzo}/index.php/api/qso` con corpo JSON `{"key", "station_profile_id", "type": "adif", "string"}` (documentazione delle due API e
//! sorgente di `application/controllers/Api.php`). Risposte lette dal sorgente:
//! - 201 `{"status": "created", ...}`: registrato (Cloudlog mette i messaggi dell'importazione, per esempio i doppioni, in `messages`);
//! - 400 `{"status": "abort", "messages": [...]}` (Wavelog): non registrato, i doppioni hanno «Duplicate» nel messaggio;
//! - 401 chiave mancante o sbagliata, o stazione di un altro utente; 403 chiave senza permesso di scrittura (Wavelog);
//! - 429 troppe richieste (limite di Wavelog).
//!
//! La prova usa `GET {indirizzo}/index.php/api/station_info/{chiave}` (sola lettura): elenca le stazioni della chiave, cosi' si controllano
//! chiave e numero della stazione senza scrivere niente nel log.

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

/// Dalla risposta di `station_info`: la stazione con quel numero (nome e nominativo), se c'e'.
pub fn find_station(body: &str, station_id: &str) -> Result<String, String> {
    let v: Value = serde_json::from_str(body).map_err(|_| "risposta non leggibile (e' davvero Wavelog o Cloudlog?)".to_string())?;
    let list = v.as_array().ok_or_else(|| v.get("reason").and_then(Value::as_str).unwrap_or("risposta inattesa").to_string())?;
    let id_of = |s: &Value| match s.get("station_id") {
        Some(Value::String(t)) => t.clone(),
        Some(Value::Number(n)) => n.to_string(),
        _ => String::new(),
    };
    let station = list.iter().find(|s| id_of(s) == station_id.trim()).ok_or_else(|| {
        let ids: Vec<String> = list.iter().map(id_of).collect();
        format!("la stazione {station_id} non e' tra quelle della chiave ({})", ids.join(", "))
    })?;
    let text = |k: &str| station.get(k).and_then(Value::as_str).unwrap_or("").to_string();
    Ok(format!("chiave valida, stazione {station_id}: {} ({})", text("station_profile_name"), text("station_callsign")))
}

/// La prova (bloccante): chiave e stazione, senza scrivere nel log. Non controlla il permesso di scrittura (si vede al primo QSO).
pub fn test(url: &str, key: &str, station_id: &str, timeout: Duration, user_agent: &str) -> Result<String, String> {
    let base = api_base(url).ok_or("indirizzo non valido: serve http:// o https://")?;
    let path_key: String = key.chars().filter(char::is_ascii_alphanumeric).collect();
    if path_key.is_empty() || path_key.len() != key.len() {
        return Err("la chiave API contiene caratteri non previsti".into());
    }
    match agent(timeout, user_agent).get(format!("{base}/index.php/api/station_info/{path_key}")).header("Accept", "application/json").call() {
        Ok(mut response) => {
            let status = response.status().as_u16();
            let text = response.body_mut().read_to_string().unwrap_or_default();
            match status {
                200 => find_station(&text, station_id),
                401 | 403 => Err("chiave API non valida".into()),
                404 => Err("indirizzo dell'API non trovato".into()),
                s => Err(format!("risposta {s}")),
            }
        }
        Err(e) => Err(crate::eqsl::describe(&e)),
    }
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
        assert_eq!(find_station(body, "2").unwrap(), "chiave valida, stazione 2: Portatile (K1ABC/P)");
        assert!(find_station(body, "1").unwrap().contains("Casa"));
        assert!(find_station(body, "7").unwrap_err().contains("1, 2"));
        assert!(find_station("<html>", "1").is_err());
    }
}
