//! PSK Reporter: chi ha ricevuto il proprio nominativo, dalla richiesta pubblica `retrieve.pskreporter.info` (solo lettura).

use serde::Serialize;

use crate::xml;

/// Quanti rapporti si tengono al massimo.
pub const MAX_REPORTS: usize = 500;

/// Una stazione che ha ricevuto il nostro segnale.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Report {
    pub receiver: String,
    /// Locatore del ricevente come lo dichiara (4-8 caratteri), maiuscolo.
    pub locator: String,
    pub snr: Option<i32>,
    pub freq_hz: u64,
    pub mode: String,
    /// Quando e' stato ricevuto (secondi Unix).
    pub time_s: u64,
    pub country: Option<String>,
}

/// Il nominativo va bene da mettere in una richiesta: lettere, cifre e «/», da 3 a 12 caratteri.
pub fn clean_callsign(call: &str) -> Option<String> {
    let call = call.trim().to_uppercase();
    (3..=12).contains(&call.len()).then_some(())?;
    call.chars().all(|c| c.is_ascii_alphanumeric() || c == '/').then_some(call)
}

/// L'indirizzo della richiesta: i rapporti sul nostro nominativo delle ultime `hours` ore, solo ricezioni (non gli elenchi dei riceventi attivi).
pub fn query_url(callsign: &str, hours: u32) -> Option<String> {
    let call = clean_callsign(callsign)?.replace('/', "%2F");
    Some(format!("https://retrieve.pskreporter.info/query?senderCallsign={call}&flowStartSeconds=-{}&rronly=1", u64::from(hours.clamp(1, 24)) * 3600))
}

/// Legge i rapporti; scarta quelli senza ricevente, frequenza o ora. Piu' recenti per primi.
pub fn parse(body: &str) -> Vec<Report> {
    let mut reports: Vec<Report> = xml::tags(body, "receptionReport")
        .into_iter()
        .filter_map(|tag| {
            let a = xml::attributes(tag);
            let receiver = a.get("receiverCallsign").filter(|c| !c.is_empty())?.to_uppercase();
            Some(Report {
                receiver,
                locator: a.get("receiverLocator").map(|l| l.to_uppercase()).unwrap_or_default(),
                snr: a.get("sNR").and_then(|v| v.trim().parse().ok()),
                freq_hz: a.get("frequency")?.trim().parse().ok()?,
                mode: a.get("mode").cloned().unwrap_or_default(),
                time_s: a.get("flowStartSeconds")?.trim().parse().ok()?,
                country: a.get("receiverDXCC").filter(|c| !c.is_empty()).cloned(),
            })
        })
        .collect();
    reports.sort_by_key(|r| std::cmp::Reverse(r.time_s));
    reports.truncate(MAX_REPORTS);
    reports
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"<?xml version="1.0"?>
<receptionReports >
  <lastSequenceNumber value="1"/>
  <receptionReport receiverCallsign="k1abc" receiverLocator="fn42" senderCallsign="IU3ABC" senderLocator="IO91WM" frequency="14075500" flowStartSeconds="1792864254" mode="FT8" isSender="1" receiverDXCC="United States" sNR="-12" />
  <receptionReport receiverCallsign="G0XYZ" receiverLocator="IO91wm12" senderCallsign="IU3ABC" frequency="7075000" flowStartSeconds="1792864900" mode="FT8" isSender="1" sNR="3" />
  <receptionReport receiverCallsign="" frequency="1" flowStartSeconds="1" />
  <receptionReport receiverCallsign="JA1AAA" frequency="abc" flowStartSeconds="5" />
  <receptionReport receiverCallsign="DL1AAA" frequency="14074000" flowStartSeconds="1792860000" mode="FT4" />
</receptionReports>"#;

    #[test]
    fn reads_the_reports_most_recent_first() {
        let r = parse(SAMPLE);
        assert_eq!(r.iter().map(|x| x.receiver.as_str()).collect::<Vec<_>>(), vec!["G0XYZ", "K1ABC", "DL1AAA"]);
        assert_eq!((r[1].locator.as_str(), r[1].snr, r[1].freq_hz, r[1].mode.as_str(), r[1].country.as_deref()), ("FN42", Some(-12), 14_075_500, "FT8", Some("United States")));
        assert_eq!((r[0].locator.as_str(), r[0].country.clone()), ("IO91WM12", None));
    }

    #[test]
    fn a_report_without_a_snr_still_counts() {
        assert_eq!(parse(SAMPLE)[2].snr, None);
    }

    #[test]
    fn an_empty_or_unrelated_answer_gives_no_reports() {
        assert!(parse("").is_empty());
        assert!(parse("<html>429 Too Many Requests</html>").is_empty());
    }

    #[test]
    fn keeps_at_most_the_cap() {
        let body: String = (0..MAX_REPORTS + 50).map(|i| format!("<receptionReport receiverCallsign=\"R{i}\" frequency=\"14074000\" flowStartSeconds=\"{i}\"/>")).collect();
        let r = parse(&body);
        assert_eq!(r.len(), MAX_REPORTS);
        assert_eq!(r[0].time_s, (MAX_REPORTS + 49) as u64);
    }

    #[test]
    fn builds_the_query_only_for_a_plausible_callsign() {
        assert_eq!(query_url(" iu3abc ", 1).as_deref(), Some("https://retrieve.pskreporter.info/query?senderCallsign=IU3ABC&flowStartSeconds=-3600&rronly=1"));
        assert!(query_url("IU3ABC/P", 2).as_deref().is_some_and(|u| u.contains("IU3ABC%2FP") && u.contains("-7200")));
        assert!(query_url("a&b=c", 1).is_none());
        assert!(query_url("", 1).is_none());
        assert!(query_url("AB", 1).is_none());
        assert!(query_url("WAYTOOLONGCALL1", 1).is_none());
    }

    #[test]
    fn the_period_is_kept_between_one_and_twenty_four_hours() {
        assert!(query_url("IU3ABC", 0).is_some_and(|u| u.contains("-3600")));
        assert!(query_url("IU3ABC", 999).is_some_and(|u| u.contains("-86400")));
    }
}
