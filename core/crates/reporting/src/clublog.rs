//! Invio diretto a Club Log in tempo reale (`https://clublog.org/realtime.php`, articolo «How To Upload QSOs In Real-Time» dell'assistenza di
//! Club Log e modulo d'esempio `https://clublog.org/test_realtime.html`).
//!
//! Richiesta POST con i campi `email` (quella registrata su Club Log), `password` (meglio una «Application Password» creata nelle impostazioni
//! dell'account), `callsign` (il log in cui scrivere), `adif` (**un solo** record chiuso da `<EOR>`) e `api` (la chiave dell'applicazione, che
//! Club Log rilascia agli sviluppatori su richiesta all'assistenza: in Aurora la si scrive nelle impostazioni). Risposte documentate:
//! - 200 «QSO OK», «QSO Duplicate» (da considerare riuscito), «QSO Modified» (registrato con qualche campo cambiato);
//! - 400 QSO rifiutato dal lettore ADIF;
//! - 403 accesso negato: il programma **deve fermarsi** e mostrare l'errore (chi ripete richieste con credenziali sbagliate viene bloccato dal
//!   firewall di Club Log).
//!
//! Non esiste una prova delle credenziali senza inviare un QSO: il pulsante «Prova» controlla solo che i dati ci siano.

use std::time::Duration;

use crate::upload::{Outcome, agent, network, short};

pub const URL: &str = "https://clublog.org/realtime.php";

/// I campi fissi della richiesta.
pub struct Form<'a> {
    pub email: &'a str,
    pub password: &'a str,
    pub callsign: &'a str,
    pub api: &'a str,
}

/// Come e' andata, dalla risposta.
pub fn read_reply(status: u16, body: &str) -> Outcome {
    read_reply_coded(status, body).0
}

/// Come [`read_reply`], con il codice stabile dell'esito e il valore `detail` (testo di Club Log o numero della risposta) per tradurlo:
/// - `clublog_sent`: 200, QSO registrato (anche «QSO Modified»);
/// - `clublog_duplicate`: 200 con «dupe» o «duplicate» nel testo (vale come riuscito);
/// - `clublog_rejected`: 400, QSO rifiutato dal lettore ADIF (esce dalla coda; `detail` vuoto = dati non validi);
/// - `clublog_denied`: 401 o 403, accesso negato (email, password, chiave API o nominativo non accettati); l'invio si ferma;
/// - `clublog_retry_http`: 408, 429 o 5xx, servizio occupato (si riprova);
/// - `clublog_rejected_status`: ogni altro numero, QSO rifiutato.
pub fn read_reply_coded(status: u16, body: &str) -> (Outcome, &'static str, String) {
    let text = short(body, 120);
    let lower = text.to_lowercase();
    match status {
        200 if lower.contains("dupe") || lower.contains("duplicate") => (Outcome::Duplicate(text.clone()), "clublog_duplicate", text),
        200 => (Outcome::Sent(text.clone()), "clublog_sent", text),
        400 => (Outcome::Rejected(if text.is_empty() { "dati non validi".into() } else { text.clone() }), "clublog_rejected", text),
        401 | 403 => {
            let detail = if text.is_empty() { status.to_string() } else { text };
            (Outcome::Blocked(format!("Club Log ha negato l'accesso ({detail})")), "clublog_denied", detail)
        }
        408 | 429 | 500..=599 => (Outcome::Retry(format!("Club Log ha risposto {status}")), "clublog_retry_http", status.to_string()),
        s => (Outcome::Rejected(format!("risposta {s}")), "clublog_rejected_status", s.to_string()),
    }
}

/// Manda un QSO (bloccante).
pub fn upload(url: &str, form: &Form<'_>, record: &str, timeout: Duration, user_agent: &str) -> Outcome {
    upload_coded(url, form, record, timeout, user_agent).0
}

/// Come [`upload`], con codice e `detail` dell'esito (vedi [`read_reply_coded`]); un errore di rete e' `clublog_retry_network` con la causa.
pub fn upload_coded(url: &str, form: &Form<'_>, record: &str, timeout: Duration, user_agent: &str) -> (Outcome, &'static str, String) {
    let fields = [("email", form.email), ("password", form.password), ("callsign", form.callsign), ("adif", record), ("api", form.api)];
    match agent(timeout, user_agent).post(url).send_form(fields) {
        Ok(mut response) => {
            let status = response.status().as_u16();
            let text = response.body_mut().read_to_string().unwrap_or_default();
            read_reply_coded(status, &text)
        }
        Err(e) => {
            let outcome = network(&e);
            let cause = match &outcome {
                Outcome::Retry(m) => m.clone(),
                _ => String::new(),
            };
            (outcome, "clublog_retry_network", cause)
        }
    }
}

/// La «prova» senza invio: i dati ci sono e hanno una forma credibile.
pub fn check(form: &Form<'_>) -> Result<String, String> {
    if !form.email.contains('@') {
        return Err("serve l'email registrata su Club Log".into());
    }
    if form.callsign.is_empty() {
        return Err("serve il nominativo del log (o il tuo nominativo nella scheda Generali)".into());
    }
    if form.password.is_empty() || form.api.is_empty() {
        return Err("servono la password e la chiave API dell'applicazione".into());
    }
    Ok(format!(
        "dati completi per il log {}. Club Log non offre una prova senza inviare un QSO: l'esito vero arriva col primo QSO (un errore di credenziali ferma subito l'invio)",
        form.callsign
    ))
}

// ---------------------------------------------------------------------------------------------------------------------------------------------
// API REST v1 con il token personale (articoli «REST API» dell'assistenza di Club Log). Il radioamatore crea il token in Club Log › Settings ›
// API (comincia con `cl_`, si vede una volta sola, sparisce dopo sei mesi senza uso) e lo incolla in Aurora: niente email, password ne' chiave
// dello sviluppatore. Richiesta `POST {API_URL}/qso` con `Authorization: Bearer <token>` e JSON `{"call": <log>, "adif": <un record>}`.
// Risposte: 201 `created` (con `warning` resta creato; con un token di prova `qsoid` nullo e `test_mode`), 200 `duplicate`/`updated`/`dropped`,
// errori `{"error":{"code","message"}}`: 401 token non valido (non si ripete), 403 nominativo non dell'account, 404 nominativo non registrato,
// 422 QSO rifiutato, 429 attendere `Retry-After`, 500 non ripetere subito.

/// La base dell'API REST.
pub const API_URL: &str = "https://clublog.org/api/v1";

/// Il messaggio d'errore dell'API (`error.message`), o il testo corto della risposta.
fn api_message(body: &str) -> String {
    serde_json::from_str::<serde_json::Value>(body)
        .ok()
        .and_then(|v| v.pointer("/error/message").or_else(|| v.get("message")).and_then(|m| m.as_str()).map(|m| short(m, 160)))
        .unwrap_or_else(|| short(body, 120))
}

/// Come e' andato un invio col token. Codici come [`read_reply_coded`], piu' `clublog_token_denied` (401: token non valido o revocato).
pub fn read_token_reply(status: u16, body: &str) -> (Outcome, &'static str, String) {
    let json = serde_json::from_str::<serde_json::Value>(body).ok();
    let state = json.as_ref().and_then(|v| v.get("status")).and_then(|s| s.as_str()).unwrap_or_default().to_lowercase();
    let message = api_message(body);
    match status {
        200 | 201 => match state.as_str() {
            "duplicate" => (Outcome::Duplicate(message.clone()), "clublog_duplicate", message),
            "dropped" | "rejected" => (Outcome::Rejected(if message.is_empty() { "dati non validi".into() } else { message.clone() }), "clublog_rejected", message),
            _ => (Outcome::Sent(message.clone()), "clublog_sent", message),
        },
        401 => (Outcome::Blocked(format!("Club Log non accetta il token ({message})")), "clublog_token_denied", message),
        403 | 404 => {
            let detail = if message.is_empty() { status.to_string() } else { message };
            (Outcome::Blocked(format!("Club Log ha negato l'accesso ({detail})")), "clublog_denied", detail)
        }
        408 | 429 | 500..=599 => (Outcome::Retry(format!("Club Log ha risposto {status}")), "clublog_retry_http", status.to_string()),
        400 | 422 => (Outcome::Rejected(if message.is_empty() { "dati non validi".into() } else { message.clone() }), "clublog_rejected", message),
        s => (Outcome::Rejected(format!("risposta {s}")), "clublog_rejected_status", s.to_string()),
    }
}

/// Il corpo JSON di un QSO.
pub fn token_body(callsign: &str, record: &str) -> String {
    serde_json::json!({ "call": callsign, "adif": record }).to_string()
}

/// Manda un QSO col token (bloccante).
pub fn upload_token(api_url: &str, token: &str, callsign: &str, record: &str, timeout: Duration, user_agent: &str) -> (Outcome, &'static str, String) {
    let url = format!("{}/qso", api_url.trim_end_matches('/'));
    let bearer = format!("Bearer {}", token.trim());
    let request = agent(timeout, user_agent)
        .post(&url)
        .header("Authorization", &bearer)
        .header("Content-Type", "application/json")
        .header("Accept", "application/json");
    match request.send(token_body(callsign, record)) {
        Ok(mut response) => {
            let status = response.status().as_u16();
            let text = response.body_mut().read_to_string().unwrap_or_default();
            read_token_reply(status, &text)
        }
        Err(e) => {
            let outcome = network(&e);
            let cause = match &outcome {
                Outcome::Retry(m) => m.clone(),
                _ => String::new(),
            };
            (outcome, "clublog_retry_network", cause)
        }
    }
}

/// Il token ha la forma giusta (`cl_` e solo caratteri stampabili, senza spazi).
pub fn token_looks_valid(token: &str) -> bool {
    let t = token.trim();
    t.starts_with("cl_") && t.len() > 3 && t.len() <= 200 && t.chars().all(|c| c.is_ascii_graphic())
}

/// La prova vera del token: `GET /me/usage?days=1` non scrive niente e risponde 200 se il token vale.
pub fn check_token(api_url: &str, token: &str, callsign: &str, timeout: Duration, user_agent: &str) -> Result<String, String> {
    if !token_looks_valid(token) {
        return Err("il token di Club Log comincia con cl_: copialo intero da Club Log › Settings › API".into());
    }
    if callsign.is_empty() {
        return Err("serve il nominativo del log (o il tuo nominativo nella scheda Generali)".into());
    }
    let url = format!("{}/me/usage?days=1", api_url.trim_end_matches('/'));
    let bearer = format!("Bearer {}", token.trim());
    match agent(timeout, user_agent).get(&url).header("Authorization", &bearer).header("Accept", "application/json").call() {
        Ok(mut response) => {
            let status = response.status().as_u16();
            let text = response.body_mut().read_to_string().unwrap_or_default();
            match status {
                200 => Ok(format!("token accettato da Club Log: i QSO andranno nel log {callsign}")),
                401 => Err(format!("Club Log non accetta il token ({}): creane uno nuovo in Settings › API", api_message(&text))),
                s => Err(format!("Club Log ha risposto {s}: {}", api_message(&text))),
            }
        }
        Err(e) => Err(crate::eqsl::describe(&e)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_documented_replies_are_understood() {
        assert_eq!(read_reply(200, "QSO OK"), Outcome::Sent("QSO OK".into()));
        assert_eq!(read_reply(200, "QSO Duplicate"), Outcome::Duplicate("QSO Duplicate".into()));
        assert!(matches!(read_reply(200, "QSO Modified: band"), Outcome::Sent(_)));
        assert!(matches!(read_reply(400, "Rejected: invalid call"), Outcome::Rejected(m) if m.contains("invalid call")));
        assert!(matches!(read_reply(403, "Access denied"), Outcome::Blocked(_)));
        assert!(matches!(read_reply(500, ""), Outcome::Retry(_)));
    }

    #[test]
    fn every_reply_has_a_stable_code_and_its_detail() {
        let code = |status, body| {
            let (_, c, d) = read_reply_coded(status, body);
            (c, d)
        };
        assert_eq!(code(200, "QSO OK"), ("clublog_sent", "QSO OK".to_string()));
        assert_eq!(code(200, "QSO Duplicate"), ("clublog_duplicate", "QSO Duplicate".to_string()));
        assert_eq!(code(400, "Rejected: invalid call"), ("clublog_rejected", "Rejected: invalid call".to_string()));
        assert_eq!(code(403, "Login failed"), ("clublog_denied", "Login failed".to_string()));
        assert_eq!(code(403, ""), ("clublog_denied", "403".to_string()), "senza testo il dettaglio e' il numero");
        assert_eq!(code(503, ""), ("clublog_retry_http", "503".to_string()));
        assert_eq!(code(404, ""), ("clublog_rejected_status", "404".to_string()));
    }

    #[test]
    fn the_check_needs_every_field() {
        let ok = Form { email: "k1abc@example.com", password: "x", callsign: "K1ABC", api: "y" };
        assert!(check(&ok).unwrap().contains("K1ABC"));
        assert!(check(&Form { email: "k1abc", ..ok }).is_err());
        assert!(check(&Form { api: "", ..ok }).is_err());
        assert!(check(&Form { callsign: "", ..ok }).is_err());
    }
    #[test]
    fn the_token_replies_are_understood() {
        let code = |status, body| read_token_reply(status, body).1;
        assert_eq!(code(201, r#"{"status":"created","qsoid":12,"message":"QSO added"}"#), "clublog_sent");
        assert_eq!(code(201, r#"{"status":"created","qsoid":null,"test_mode":true}"#), "clublog_sent");
        assert_eq!(code(200, r#"{"status":"duplicate","message":"dupe"}"#), "clublog_duplicate");
        assert_eq!(code(200, r#"{"status":"dropped","message":"dropped"}"#), "clublog_rejected");
        assert_eq!(code(401, r#"{"error":{"code":"unauthorized","message":"bad token","details":null}}"#), "clublog_token_denied");
        assert_eq!(read_token_reply(422, r#"{"error":{"code":"unprocessable","message":"invalid band"}}"#).2, "invalid band");
        assert!(matches!(read_token_reply(429, "").0, Outcome::Retry(_)));
        assert!(matches!(read_token_reply(403, "").0, Outcome::Blocked(_)));
    }

    #[test]
    fn the_token_body_and_its_shape() {
        let v: serde_json::Value = serde_json::from_str(&token_body("IU3ABC", "<call:4>K1AB<eor>")).unwrap();
        assert_eq!((v["call"].as_str(), v["adif"].as_str()), (Some("IU3ABC"), Some("<call:4>K1AB<eor>")));
        assert!(token_looks_valid("cl_abc123") && !token_looks_valid("abc") && !token_looks_valid("cl_ a"));
    }
}
