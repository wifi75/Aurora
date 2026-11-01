//! Verifica del codice OTP del Fox (Hound classico), come `Network/FoxVerifier.cpp` di WSJT-X 3.3 (GPL): richiesta
//! `GET {url}/check/{CALL}/{ora ISO}/{codice}.text`, al massimo 5 s, risposta che finisce con « VERIFIED» o « INVALID». Il servizio vede solo il
//! nominativo del Fox, l'ora e il codice (e l'indirizzo IP).

use std::time::Duration;

/// Tempo massimo della richiesta (`FOXVERIFIER_DEFAULT_TIMEOUT_MSEC`).
pub const TIMEOUT: Duration = Duration::from_secs(5);

/// L'indirizzo della richiesta: il nominativo codificato (per `E51D/MM`), l'ora in ISO 8601 senza millisecondi (`Qt::ISODate`, UTC con «Z»).
pub fn check_url(base: &str, call: &str, unix_ms: u64, code: &str) -> String {
    let encoded: String = call
        .bytes()
        .map(|b| if b.is_ascii_alphanumeric() || matches!(b, b'-' | b'.' | b'_' | b'~') { (b as char).to_string() } else { format!("%{b:02X}") })
        .collect();
    format!("{}/check/{encoded}/{}/{code}.text", base.trim_end_matches('/'), iso_utc(unix_ms / 1000))
}

/// `AAAA-MM-GGThh:mm:ssZ` dai secondi dal 1970.
fn iso_utc(secs: u64) -> String {
    let days = (secs / 86_400) as i64;
    let rem = secs % 86_400;
    // algoritmo dei giorni civili (H. Hinnant)
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + i64::from(m <= 2);
    format!("{y:04}-{m:02}-{d:02}T{:02}:{:02}:{:02}Z", rem / 3600, rem % 3600 / 60, rem % 60)
}

/// Chiede al servizio: `Ok(true)` verificato, `Ok(false)` non valido, `Err` errore di rete o risposta che non dice nessuna delle due.
pub fn check(fetch: &dyn crate::Fetch, base: &str, call: &str, unix_ms: u64, code: &str) -> Result<bool, String> {
    let body = fetch.get(&check_url(base, call, unix_ms, code))?;
    // si leggono al massimo 1024 caratteri, come `reply_->read(1024)`
    let body: String = body.chars().take(1024).collect();
    let body = body.trim();
    if body.ends_with(" VERIFIED") {
        Ok(true)
    } else if body.ends_with(" INVALID") {
        Ok(false)
    } else {
        Err(format!("risposta non riconosciuta: {body}"))
    }
}

/// Il recupero HTTP con il tempo massimo della verifica (5 s invece dei 15 degli altri servizi).
pub struct FoxFetch {
    pub user_agent: String,
}

impl crate::Fetch for FoxFetch {
    fn get(&self, url: &str) -> Result<String, String> {
        let agent: ureq::Agent = ureq::Agent::config_builder().timeout_global(Some(TIMEOUT)).http_status_as_error(false).build().into();
        let mut response = agent.get(url).header("User-Agent", &self.user_agent).header("Accept", "*/*").call().map_err(|e| e.to_string())?;
        let status = response.status().as_u16();
        if !(200..=299).contains(&status) {
            return Err(format!("il servizio ha risposto {status}"));
        }
        response.body_mut().read_to_string().map_err(|e| e.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    struct Fake {
        answer: Result<String, String>,
        asked: Mutex<Vec<String>>,
    }

    impl crate::Fetch for Fake {
        fn get(&self, url: &str) -> Result<String, String> {
            if let Ok(mut a) = self.asked.lock() {
                a.push(url.to_string());
            }
            self.answer.clone()
        }
    }

    fn fake(answer: Result<&str, &str>) -> Fake {
        Fake { answer: answer.map(str::to_string).map_err(str::to_string), asked: Mutex::new(Vec::new()) }
    }

    #[test]
    fn the_url_is_the_one_of_wsjtx() {
        // 2026-11-01 17:21:00 UTC
        let ms = 1_793_553_660_000;
        assert_eq!(check_url("https://www.9dx.cc", "K8R", ms, "920749"), "https://www.9dx.cc/check/K8R/2026-11-01T17:21:00Z/920749.text");
        assert_eq!(check_url("https://x.y/", "E51D/MM", 0, "000001"), "https://x.y/check/E51D%2FMM/1970-01-01T00:00:00Z/000001.text");
    }

    #[test]
    fn verified_invalid_and_errors() {
        let ok = fake(Ok("172100 -00  0.0  750 ~  K8R VERIFIED\n"));
        assert_eq!(check(&ok, "https://www.9dx.cc", "K8R", 0, "920749"), Ok(true));
        assert_eq!(ok.asked.lock().map(|a| a.len()).unwrap_or(0), 1);
        assert_eq!(check(&fake(Ok("K8R INVALID")), "u", "K8R", 0, "1"), Ok(false));
        assert!(check(&fake(Ok("boh")), "u", "K8R", 0, "1").is_err());
        assert!(check(&fake(Err("timeout")), "u", "K8R", 0, "1").is_err());
    }
}
