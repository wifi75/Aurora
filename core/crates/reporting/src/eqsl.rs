//! Invio a eQSL (`EQSLSend`, `EQSLUser`, `EQSLPasswd`, `EQSLNick`): un QSO per volta, come `EQSL::upload` e `EQSL::work` di JTDX.
//!
//! Il programma manda una richiesta GET a `importADIF.cfm` con un solo parametro, `ADIFdata`, che contiene il record ADIF con utente, password e
//! nickname della stazione. JTDX usa `http://`; Aurora usa **`https://`** (la password non viaggia in chiaro). La password non e' mai nel registro, nello stato ne' in un
//! messaggio d'errore: viene letta dalla protezione dei segreti solo al momento dell'invio.

use std::time::Duration;

use aurora_core::QsoRecord;
use logbook::adif::utc_datetime;

/// Indirizzo di eQSL (JTDX: `http://www.eqsl.cc/qslcard/importADIF.cfm`).
pub const URL: &str = "https://www.eqsl.cc/qslcard/importADIF.cfm";
/// Attesa massima: il motore non aspetta mai eQSL (l'invio ha un thread proprio).
pub const TIMEOUT: Duration = Duration::from_secs(20);

fn field(out: &mut String, name: &str, value: &str) {
    out.push_str(&format!("<{name}:{}>{value}", value.encode_utf16().count()));
}

/// Il testo ADIF di `EQSL::upload`: intestazione con versione, utente, password e programma, poi (dopo `<EOH>`) nickname, nominativo, modo, data, ora
/// (solo `hhmm`), rapporto inviato, banda e l'eventuale messaggio.
pub fn adif(user: &str, password: &str, nick: &str, q: &QsoRecord, fallback_date: &str) -> String {
    let mut s = String::from("<ADIF_VER:5>2.1.9");
    field(&mut s, "EQSL_USER", user);
    field(&mut s, "EQSL_PSWD", password);
    s.push_str("<PROGRAMID:6>Aurora<EOH>");
    field(&mut s, "APP_EQSL_QTH_NICKNAME", nick);
    field(&mut s, "CALL", &q.call);
    field(&mut s, "MODE", &q.mode);
    let (date, time) = if q.time_on_unix_ms > 0 {
        utc_datetime(q.time_on_unix_ms)
    } else {
        let secs = u64::from(q.time_ms / 1000);
        (fallback_date.to_string(), format!("{:02}{:02}{:02}", secs / 3600 % 24, secs / 60 % 60, secs % 60))
    };
    s.push_str("<QSO_DATE:8>");
    s.push_str(&date);
    s.push_str("<TIME_ON:4>");
    s.push_str(time.get(..4).unwrap_or(&time));
    field(&mut s, "RST_SENT", &q.rst_sent);
    field(&mut s, "BAND", &q.band);
    if !q.eqsl_comment.is_empty() {
        field(&mut s, "QSLMSG", &q.eqsl_comment);
    }
    s.push_str("<EOR>");
    s
}

/// Cosa ha risposto eQSL, in una riga: `Ok` con il resoconto, `Err` con l'errore (la risposta e' una pagina HTML con `Result:` oppure `Error:`).
pub fn read_reply(body: &str) -> Result<String, String> {
    let text: String = body.replace(['\r', '\n'], " ");
    let lower = text.to_lowercase();
    if let Some(i) = lower.find("error:") {
        let rest: String = text[i..].chars().take(90).collect();
        return Err(format!("eQSL ha rifiutato l'invio ({})", rest.split('<').next().unwrap_or("").trim()));
    }
    if let Some(i) = lower.find("result:") {
        let rest: String = text[i..].chars().take(60).collect();
        return Ok(rest.split('<').next().unwrap_or("").trim().to_string());
    }
    Ok("inviato".to_string())
}

/// Manda il QSO (bloccante: si chiama da un thread a parte). `url` e' di solito [`URL`]; le prove usano un server locale.
pub fn upload(url: &str, adif: &str, timeout: Duration) -> Result<String, String> {
    let agent: ureq::Agent = ureq::Agent::config_builder().timeout_global(Some(timeout)).http_status_as_error(false).build().into();
    let mut response = agent.get(url).query("ADIFdata", adif).call().map_err(|e| describe(&e))?;
    let status = response.status().as_u16();
    if status != 200 {
        return Err(format!("eQSL ha risposto {status}"));
    }
    let body = response.body_mut().read_to_string().map_err(|e| describe(&e))?;
    read_reply(&body)
}

/// Un errore di rete senza indirizzo (la richiesta porta la password): solo il tipo di problema.
pub(crate) fn describe(e: &ureq::Error) -> String {
    match e {
        ureq::Error::Timeout(_) => "tempo scaduto".to_string(),
        ureq::Error::Io(_) | ureq::Error::HostNotFound | ureq::Error::ConnectionFailed => "server non raggiungibile".to_string(),
        ureq::Error::Tls(_) | ureq::Error::Rustls(_) => "errore di sicurezza della connessione".to_string(),
        _ => "errore di rete".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn qso() -> QsoRecord {
        QsoRecord {
            call: "K1ABC".into(),
            band: "20m".into(),
            mode: "FT8".into(),
            rst_sent: "-16".into(),
            time_on_unix_ms: 1_792_860_405_000, // 2026-10-24 16:46:45 UTC
            ..Default::default()
        }
    }

    #[test]
    fn the_adif_is_the_one_of_jtdx_with_the_hour_and_minute_only() {
        let s = adif("IU3ABC", "segreta", "Casa", &qso(), "20000101");
        assert_eq!(
            s,
            "<ADIF_VER:5>2.1.9<EQSL_USER:6>IU3ABC<EQSL_PSWD:7>segreta<PROGRAMID:6>Aurora<EOH><APP_EQSL_QTH_NICKNAME:4>Casa<CALL:5>K1ABC<MODE:3>FT8\
             <QSO_DATE:8>20261024<TIME_ON:4>1646<RST_SENT:3>-16<BAND:3>20m<EOR>"
        );
        let with_msg = adif("A", "b", "", &QsoRecord { eqsl_comment: "TNX 73".into(), ..qso() }, "20000101");
        assert!(with_msg.ends_with("<BAND:3>20m<QSLMSG:6>TNX 73<EOR>") && with_msg.contains("<APP_EQSL_QTH_NICKNAME:0><CALL"), "{with_msg}");
    }

    #[test]
    fn without_the_start_time_the_day_time_and_the_fallback_date_are_used() {
        let q = QsoRecord { time_on_unix_ms: 0, time_ms: 45_015_000, ..qso() };
        assert!(adif("A", "b", "c", &q, "20261026").contains("<QSO_DATE:8>20261026<TIME_ON:4>1230"));
    }

    #[test]
    fn the_reply_is_read_for_errors_and_results() {
        assert_eq!(read_reply("<html><body>Result: 1 out of 1 records added</body></html>"), Ok("Result: 1 out of 1 records added".to_string()));
        let e = read_reply("<HTML>\nError: No match on eQSL_User/eQSL_Pswd\n</HTML>").unwrap_err();
        assert!(e.contains("No match on eQSL_User"), "{e}");
        assert_eq!(read_reply("boh"), Ok("inviato".to_string()));
    }
}
