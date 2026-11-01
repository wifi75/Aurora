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
    let text = short(body, 120);
    let lower = text.to_lowercase();
    match status {
        200 if lower.contains("dupe") || lower.contains("duplicate") => Outcome::Duplicate(text),
        200 => Outcome::Sent(text),
        400 => Outcome::Rejected(if text.is_empty() { "dati non validi".into() } else { text }),
        401 | 403 => Outcome::Blocked(format!("Club Log ha negato l'accesso ({})", if text.is_empty() { status.to_string() } else { text })),
        408 | 429 | 500..=599 => Outcome::Retry(format!("Club Log ha risposto {status}")),
        s => Outcome::Rejected(format!("risposta {s}")),
    }
}

/// Manda un QSO (bloccante).
pub fn upload(url: &str, form: &Form<'_>, record: &str, timeout: Duration, user_agent: &str) -> Outcome {
    let fields = [("email", form.email), ("password", form.password), ("callsign", form.callsign), ("adif", record), ("api", form.api)];
    match agent(timeout, user_agent).post(url).send_form(fields) {
        Ok(mut response) => {
            let status = response.status().as_u16();
            let text = response.body_mut().read_to_string().unwrap_or_default();
            read_reply(status, &text)
        }
        Err(e) => network(&e),
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
    fn the_check_needs_every_field() {
        let ok = Form { email: "k1abc@example.com", password: "x", callsign: "K1ABC", api: "y" };
        assert!(check(&ok).unwrap().contains("K1ABC"));
        assert!(check(&Form { email: "k1abc", ..ok }).is_err());
        assert!(check(&Form { api: "", ..ok }).is_err());
        assert!(check(&Form { callsign: "", ..ok }).is_err());
    }
}
