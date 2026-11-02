//! Invio diretto dei QSO a HRDLOG.NET.
//!
//! Richiesta POST codificata come un modulo a `https://robot.hrdlog.net/NewEntry.aspx` con `Callsign` (il nominativo del log), `Code` (il
//! codice di caricamento, «upload code», dalla pagina del profilo di HRDLOG.NET), `App` (il programma) e `ADIFData` (un record). La risposta
//! e' un breve XML: `<insert>N</insert>` con il numero di QSO inseriti, oppure `<error>testo</error>`.
//!
//! **Non verificato sulla documentazione ufficiale**: le pagine di aiuto di HRDLOG.NET non erano raggiungibili quando e' stato scritto
//! (vedi `docs/integrazioni-log.md`). Indirizzo e campi sono quelli usati da altri programmi di log; la lettura della risposta e' prudente
//! (un `<insert>0</insert>` senza errore si tratta come doppione, una risposta che non si capisce si riprova). HRDLOG.NET non ha una prova
//! documentata senza invio: «Prova» controlla solo che i dati ci siano.

use std::time::Duration;

use crate::upload::{Outcome, agent, network, short};

pub const URL: &str = "https://robot.hrdlog.net/NewEntry.aspx";

fn tag(body: &str, name: &str) -> Option<String> {
    let lower = body.to_ascii_lowercase();
    let open = format!("<{name}>");
    let close = format!("</{name}>");
    let start = lower.find(&open)? + open.len();
    let end = lower[start..].find(&close)? + start;
    Some(body[start..end].trim().to_string())
}

/// Come e' andato un invio.
pub fn read_reply(status: u16, body: &str) -> Outcome {
    if matches!(status, 408 | 429 | 500..=599) {
        return Outcome::Retry(format!("HRDLOG.NET ha risposto {status}"));
    }
    if status != 200 {
        return Outcome::Rejected(format!("risposta {status}"));
    }
    if let Some(error) = tag(body, "error") {
        let reason = short(&error, 120);
        let lower = reason.to_lowercase();
        return if lower.contains("duplicat") || lower.contains("already") {
            Outcome::Duplicate(reason)
        } else if lower.contains("code") || lower.contains("token") || lower.contains("callsign") || lower.contains("auth") || lower.contains("user") {
            Outcome::Blocked(format!("HRDLOG.NET ha rifiutato nominativo o codice ({reason})"))
        } else {
            Outcome::Rejected(reason)
        };
    }
    match tag(body, "insert").and_then(|n| n.parse::<u32>().ok()) {
        Some(0) => Outcome::Duplicate("HRDLOG.NET non ha inserito il QSO (gia' presente)".into()),
        Some(n) => Outcome::Sent(format!("{n} QSO")),
        None => Outcome::Retry("risposta di HRDLOG.NET non leggibile".into()),
    }
}

/// Invia un QSO (bloccante).
pub fn upload(url: &str, call: &str, code: &str, record: &str, timeout: Duration, user_agent: &str) -> Outcome {
    match agent(timeout, user_agent).post(url).send_form([("Callsign", call), ("Code", code), ("App", user_agent), ("ADIFData", record)]) {
        Ok(mut response) => {
            let status = response.status().as_u16();
            let text = response.body_mut().read_to_string().unwrap_or_default();
            read_reply(status, &text)
        }
        Err(e) => network(&e),
    }
}

/// La prova: HRDLOG.NET non ha una verifica documentata senza invio, si controllano i dati.
pub fn check(call: &str, has_code: bool) -> Result<String, String> {
    if call.is_empty() {
        return Err("manca il nominativo del log".into());
    }
    if !has_code {
        return Err("manca il codice di caricamento".into());
    }
    Ok(format!("dati presenti per {call} (HRDLOG.NET non ha una prova senza invio: il codice si verifica al primo QSO)"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{io::{BufRead, BufReader, Read, Write}, net::TcpListener};

    #[test]
    fn replies_are_read() {
        assert_eq!(read_reply(200, "<?xml version=\"1.0\"?><HRDLOG><insert>1</insert></HRDLOG>"), Outcome::Sent("1 QSO".into()));
        assert!(matches!(read_reply(200, "<insert>0</insert>"), Outcome::Duplicate(_)));
        assert!(matches!(read_reply(200, "<error>Invalid upload code</error>"), Outcome::Blocked(_)));
        assert!(matches!(read_reply(200, "<error>Duplicate QSO</error>"), Outcome::Duplicate(_)));
        assert!(matches!(read_reply(200, "<error>bad adif</error>"), Outcome::Rejected(_)));
        assert!(matches!(read_reply(503, ""), Outcome::Retry(_)));
        assert!(matches!(read_reply(200, "<html>"), Outcome::Retry(_)));
        assert!(check("IU3ABC", true).is_ok());
        assert!(check("IU3ABC", false).is_err());
    }

    #[test]
    fn the_form_reaches_a_fake_server() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!("http://{}/NewEntry.aspx", listener.local_addr().unwrap());
        let server = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut reader = BufReader::new(stream.try_clone().unwrap());
            let mut len = 0;
            loop {
                let mut h = String::new();
                reader.read_line(&mut h).unwrap();
                if let Some(v) = h.to_ascii_lowercase().strip_prefix("content-length:") {
                    len = v.trim().parse().unwrap();
                }
                if h == "\r\n" {
                    break;
                }
            }
            let mut body = vec![0; len];
            reader.read_exact(&mut body).unwrap();
            let reply = "<insert>1</insert>";
            write!(stream, "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{reply}", reply.len()).unwrap();
            String::from_utf8(body).unwrap()
        });
        let out = upload(&url, "IU3ABC", "abc123", "<CALL:6>IU3XYZ <EOR>", Duration::from_secs(5), "Aurora/test");
        assert_eq!(out, Outcome::Sent("1 QSO".into()));
        let form = server.join().unwrap();
        assert!(form.contains("Callsign=IU3ABC") && form.contains("Code=abc123") && form.contains("ADIFData="));
    }
}
