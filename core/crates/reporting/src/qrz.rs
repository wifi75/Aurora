//! Invio diretto al logbook di QRZ.com (`https://logbook.qrz.com/api`, documentazione «QRZ Logbook API»).
//!
//! Richiesta POST con coppie nome=valore codificate come un modulo: `KEY` (la chiave del logbook), `ACTION=INSERT`, `ADIF` (un record). Risposta
//! nello stesso formato: `RESULT=OK|FAIL|REPLACE`, `REASON` (con `FAIL`), `LOGID`, `COUNT`. Il servizio pretende un User-Agent riconoscibile
//! (al piu' 128 caratteri) e l'abbonamento per l'inserimento. Senza `OPTION=REPLACE` un doppione non sovrascrive niente (con `REPLACE` QRZ
//! sovrascriverebbe anche un QSO gia' confermato: Aurora non lo usa).
//!
//! La prova usa `ACTION=STATUS`, che non scrive niente e risponde `RESULT=OK` se la chiave e' buona.

use std::{collections::BTreeMap, time::Duration};

use crate::upload::{Outcome, agent, network, short};

pub const URL: &str = "https://logbook.qrz.com/api";

fn hex(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'f' => Some(b - b'a' + 10),
        b'A'..=b'F' => Some(b - b'A' + 10),
        _ => None,
    }
}

/// Decodifica di un valore di modulo (`+` = spazio, `%xx`).
fn decode(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'+' => out.push(b' '),
            b'%' if i + 2 < bytes.len() => match (hex(bytes[i + 1]), hex(bytes[i + 2])) {
                (Some(a), Some(b)) => {
                    out.push(a * 16 + b);
                    i += 2;
                }
                _ => out.push(b'%'),
            },
            b => out.push(b),
        }
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// La risposta come coppie (nomi in maiuscolo).
pub fn parse_reply(body: &str) -> BTreeMap<String, String> {
    body.trim()
        .split('&')
        .filter_map(|pair| {
            let (k, v) = pair.split_once('=')?;
            Some((decode(k).trim().to_ascii_uppercase(), decode(v)))
        })
        .collect()
}

/// Come e' andato un inserimento.
pub fn read_insert(status: u16, body: &str) -> Outcome {
    if matches!(status, 408 | 429 | 500..=599) {
        return Outcome::Retry(format!("QRZ ha risposto {status}"));
    }
    if status != 200 {
        return Outcome::Rejected(format!("risposta {status}"));
    }
    let reply = parse_reply(body);
    let result = reply.get("RESULT").map(|r| r.trim().to_ascii_uppercase()).unwrap_or_default();
    let reason = short(reply.get("REASON").map_or("", String::as_str), 120);
    let lower = reason.to_lowercase();
    match result.as_str() {
        "OK" | "REPLACE" => Outcome::Sent(reply.get("LOGID").or_else(|| reply.get("LOGIDS")).map(|id| format!("LOGID {id}")).unwrap_or_default()),
        "AUTH" => Outcome::Blocked(format!("QRZ non autorizza la chiave ({reason})")),
        "FAIL" if lower.contains("duplicate") => Outcome::Duplicate(reason),
        "FAIL" if lower.contains("key") || lower.contains("auth") || lower.contains("subscri") || lower.contains("privilege") => {
            Outcome::Blocked(format!("QRZ ha rifiutato la chiave ({reason})"))
        }
        "FAIL" => Outcome::Rejected(reason),
        _ => Outcome::Retry("risposta di QRZ non leggibile".into()),
    }
}

/// Il record con `STATION_CALLSIGN` sostituito (o aggiunto) dal nominativo registrato su QRZ, se dato: i QSO fatti come
/// `IU3ABC/P` o `EA3/IU3ABC` vanno nel logbook di `IU3ABC`, che QRZ altrimenti scarterebbe.
pub fn force_station_call(record: &str, call: &str) -> String {
    let call = call.trim().to_uppercase();
    if call.is_empty() {
        return record.to_string();
    }
    let field = format!("<STATION_CALLSIGN:{}>{call} ", call.len());
    let mut out = String::with_capacity(record.len() + field.len());
    let mut rest = record;
    let mut replaced = false;
    while let Some(start) = rest.find('<') {
        out.push_str(&rest[..start]);
        let tail = &rest[start..];
        let Some(close) = tail.find('>') else { break };
        let head = &tail[1..close];
        let mut parts = head.split(':');
        let name = parts.next().unwrap_or("").to_ascii_uppercase();
        let len: usize = parts.next().and_then(|l| l.trim().parse().ok()).unwrap_or(0);
        let value_end = (close + 1 + len).min(tail.len());
        if !tail.is_char_boundary(value_end) {
            break;
        }
        if name == "STATION_CALLSIGN" {
            out.push_str(&field);
            replaced = true;
            rest = tail[value_end..].trim_start_matches(' ');
            continue;
        }
        if name == "EOR" && !replaced {
            out.push_str(&field);
            replaced = true;
        }
        out.push_str(&tail[..value_end]);
        rest = &tail[value_end..];
    }
    out.push_str(rest);
    out
}

/// Inserisce un QSO (bloccante).
pub fn insert(url: &str, key: &str, record: &str, timeout: Duration, user_agent: &str) -> Outcome {
    match agent(timeout, user_agent).post(url).send_form([("KEY", key), ("ACTION", "INSERT"), ("ADIF", record)]) {
        Ok(mut response) => {
            let status = response.status().as_u16();
            let text = response.body_mut().read_to_string().unwrap_or_default();
            read_insert(status, &text)
        }
        Err(e) => network(&e),
    }
}

/// La prova (bloccante): `ACTION=STATUS`, nessun QSO scritto.
pub fn status(url: &str, key: &str, timeout: Duration, user_agent: &str) -> Result<String, String> {
    let mut response = agent(timeout, user_agent).post(url).send_form([("KEY", key), ("ACTION", "STATUS")]).map_err(|e| crate::eqsl::describe(&e))?;
    let code = response.status().as_u16();
    let text = response.body_mut().read_to_string().unwrap_or_default();
    if code != 200 {
        return Err(format!("QRZ ha risposto {code}"));
    }
    read_status(&text)
}

/// La risposta di `STATUS`.
pub fn read_status(body: &str) -> Result<String, String> {
    let reply = parse_reply(body);
    match reply.get("RESULT").map(|r| r.trim().to_ascii_uppercase()).as_deref() {
        Some("OK") => {
            // `DATA` contiene altre coppie (codificate): se c'e' il nome del logbook lo si mostra
            let data = reply.get("DATA").map(|d| parse_reply(d)).unwrap_or_default();
            let book = data.get("CALLSIGN").or_else(|| data.get("BOOK_NAME")).cloned().unwrap_or_default();
            Ok(if book.is_empty() { "chiave valida".to_string() } else { format!("chiave valida, logbook {}", short(&book, 40)) })
        }
        Some(_) => Err(format!("chiave rifiutata ({})", short(reply.get("REASON").map_or("", String::as_str), 120))),
        None => Err("risposta di QRZ non leggibile".into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_reply_is_split_and_decoded() {
        let r = parse_reply("RESULT=FAIL&REASON=Unable+to+add+QSO+to+database%3A+duplicate&EXTENDED=");
        assert_eq!(r["RESULT"], "FAIL");
        assert_eq!(r["REASON"], "Unable to add QSO to database: duplicate");
        assert_eq!(decode("a%2"), "a%2");
    }

    #[test]
    fn inserts_are_read_as_documented() {
        assert_eq!(read_insert(200, "RESULT=OK&LOGID=130877825&COUNT=1"), Outcome::Sent("LOGID 130877825".into()));
        assert_eq!(read_insert(200, "RESULT=OK&LOGIDS=130877825&COUNT=1"), Outcome::Sent("LOGID 130877825".into()));
        assert!(matches!(read_insert(200, "RESULT=FAIL&REASON=Unable to add QSO to database: duplicate&EXTENDED="), Outcome::Duplicate(_)));
        assert!(matches!(read_insert(200, "RESULT=FAIL&REASON=invalid api key ABCD"), Outcome::Blocked(_)));
        assert!(matches!(read_insert(200, "RESULT=AUTH&REASON=insufficient privileges"), Outcome::Blocked(_)));
        assert!(matches!(read_insert(200, "RESULT=FAIL&REASON=wrong station_callsign"), Outcome::Rejected(m) if m == "wrong station_callsign"));
        assert!(matches!(read_insert(503, ""), Outcome::Retry(_)));
        assert!(matches!(read_insert(200, "<html>"), Outcome::Retry(_)));
    }

    #[test]
    fn the_registered_callsign_replaces_the_portable_one() {
        let r = "<CALL:6>IU3XYZ <STATION_CALLSIGN:8>IU3ABC/P <BAND:3>20m <EOR>";
        assert_eq!(force_station_call(r, "iu3abc"), "<CALL:6>IU3XYZ <STATION_CALLSIGN:6>IU3ABC <BAND:3>20m <EOR>");
        assert_eq!(force_station_call(r, ""), r);
        assert_eq!(force_station_call("<CALL:6>IU3XYZ <EOR>", "IU3ABC"), "<CALL:6>IU3XYZ <STATION_CALLSIGN:6>IU3ABC <EOR>");
    }

    #[test]
    fn the_status_says_if_the_key_is_good() {
        assert_eq!(read_status("RESULT=OK&DATA=CALLSIGN%3DK1ABC%26COUNT%3D12"), Ok("chiave valida, logbook K1ABC".into()));
        assert_eq!(read_status("RESULT=OK"), Ok("chiave valida".into()));
        assert!(read_status("RESULT=FAIL&REASON=invalid api key").unwrap_err().contains("invalid api key"));
    }
}
