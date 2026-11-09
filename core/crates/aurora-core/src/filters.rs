//! I filtri di testo della scheda Filtri, applicati anche ai messaggi che escono via UDP («Applica filtri di testo alla trasmissione dei messaggi UDP»,
//! `ApplyFiltersToUDPmessages` di JTDX: `readFromStdout` manda `postDecode` solo se `!filterUDP || (notified & 2)`, cioe' solo le righe che i filtri
//! lasciano vedere).
//!
//! Gli stessi filtri decidono anche a chi le Automazioni possono rispondere: una stazione nascosta in Attivita' di banda non riceve mai una
//! risposta automatica.
//!
//! La scheda Filtri vive nel browser: quando cambia, la pagina ne manda una copia ad Aurora (`PUT /api/v1/band-filters`), e all'avvio Aurora la
//! rilegge dalle preferenze dell'interfaccia salvate sul server (`aurora-filters` di `ui-settings.json`). Le regole sono le stesse di
//! `app/src/lib/filters.ts` (`applyFilters`), con gli stessi nomi di campo.

use std::sync::{Arc, RwLock};

use serde::{Deserialize, Serialize};

use crate::{DecodeTag, DecodedMessage};

const CQ_WORDS: [&str; 3] = ["CQ", "CQDX", "QRZ"];

/// I filtri come li tiene la pagina (nomi dei campi in camelCase).
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct UdpFilters {
    pub hide_continents: Vec<String>,
    pub hide_countries: Vec<String>,
    pub hide_calls: Vec<String>,
    /// `all`, `cq`, `cq_rrr` o `cq_rr73`.
    pub only: String,
    pub hide_non_standard: bool,
    pub hide_own_continent: bool,
    pub hide_worked: bool,
    /// Con `hide_worked`: nasconde anche gli scambi delle stazioni gia' collegate, non solo i loro CQ.
    pub hide_worked_all: bool,
    /// Mostra soltanto i messaggi di questi nominativi o prefissi (`JA*`); vuoto = tutti.
    pub show_only_calls: Vec<String>,
    /// Nominativi o prefissi sempre visibili, anche se un altro filtro li nasconderebbe.
    pub always_show_calls: Vec<String>,
    /// Tutti i filtri sospesi senza perderne le impostazioni.
    pub bypass: bool,
}

/// Lo stesso insieme di filtri visto da Attivita' di banda (e dalle Automazioni).
pub type BandFilters = UdpFilters;

/// La chiave delle preferenze dell'interfaccia dove la pagina tiene i filtri.
pub const UI_FILTERS_KEY: &str = "aurora-filters";

/// I filtri correnti, condivisi tra l'API che li riceve e l'emettitore UDP che li applica.
pub type SharedUdpFilters = Arc<RwLock<UdpFilters>>;

fn is_call_token(token: &str) -> bool {
    token.len() >= 3
        && token.chars().all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '/')
        && token.chars().any(|c| c.is_ascii_digit())
        && token.chars().any(|c| c.is_ascii_uppercase())
        && !is_grid_token(token)
}

fn is_grid_token(token: &str) -> bool {
    let b = token.as_bytes();
    b.len() == 4 && (b'A'..=b'R').contains(&b[0]) && (b'A'..=b'R').contains(&b[1]) && b[2].is_ascii_digit() && b[3].is_ascii_digit()
}

fn tokens(text: &str) -> Vec<String> {
    text.split_whitespace().map(str::to_uppercase).collect()
}

/// Nominativo di chi trasmette: dopo il CQ (saltando i modificatori) o il secondo del messaggio.
pub fn call_of(text: &str) -> Option<String> {
    let t = tokens(text);
    if t.first().is_some_and(|w| CQ_WORDS.contains(&w.as_str())) {
        for token in t.iter().skip(1) {
            if is_call_token(token) {
                return Some(token.clone());
            }
            if !(2..=4).contains(&token.len()) || !token.chars().all(|c| c.is_ascii_uppercase()) {
                break;
            }
        }
        return None;
    }
    t.get(1).filter(|s| is_call_token(s)).cloned()
}

pub fn is_cq(text: &str) -> bool {
    tokens(text).first().is_some_and(|w| CQ_WORDS.contains(&w.as_str()))
}

fn is_standard(text: &str) -> bool {
    if text.contains('<') {
        return false;
    }
    let t = tokens(text);
    if t.first().is_some_and(|w| CQ_WORDS.contains(&w.as_str())) {
        return call_of(text).is_some();
    }
    t.len() >= 2 && is_call_token(&t[0]) && is_call_token(&t[1])
}

fn call_matches(call: Option<&str>, pattern: &str) -> bool {
    let Some(call) = call else { return false };
    let p = pattern.trim().to_uppercase();
    match p.strip_suffix('*') {
        Some(prefix) => call.starts_with(prefix),
        None => call == p,
    }
}

fn any_match(call: Option<&str>, patterns: &[String]) -> bool {
    patterns.iter().any(|p| !p.trim().is_empty() && call_matches(call, p))
}

fn signoff(text: &str, strict: bool) -> bool {
    let t = tokens(text);
    let last = t.last().map_or("", String::as_str);
    last == "RR73" || last == "73" || (!strict && last == "RRR")
}

impl UdpFilters {
    /// Vero se la riga resta visibile con questi filtri (e quindi esce via UDP). `own_continent`: il continente di chi opera.
    pub fn allows(&self, d: &DecodedMessage, own_continent: Option<&str>) -> bool {
        if let Some(verdict) = self.station_verdict(d, own_continent) {
            return verdict;
        }
        let cq = is_cq(&d.text);
        match self.only.as_str() {
            "cq" if !cq => return false,
            "cq_rrr" if !cq && !signoff(&d.text, false) => return false,
            "cq_rr73" if !cq && !signoff(&d.text, true) => return false,
            _ => {}
        }
        if self.hide_non_standard && !is_standard(&d.text) {
            return false;
        }
        if self.hide_worked && d.tag == Some(DecodeTag::WorkedCall) && (self.hide_worked_all || cq) {
            return false;
        }
        true
    }

    /// Vero se la stazione che trasmette non e' nascosta (nominativi, continenti, paesi), qualunque sia il tipo di messaggio: serve a chi
    /// risponde al nostro CQ, che non e' un CQ e cadrebbe sempre nel filtro «solo CQ».
    pub fn allows_station(&self, d: &DecodedMessage, own_continent: Option<&str>) -> bool {
        self.station_verdict(d, own_continent) != Some(false)
    }

    /// Le regole sulla stazione: `Some` se decidono da sole, `None` se la riga dipende dal tipo di messaggio.
    fn station_verdict(&self, d: &DecodedMessage, own_continent: Option<&str>) -> Option<bool> {
        if self.bypass {
            return Some(true);
        }
        if !self.always_show_calls.is_empty() && any_match(call_of(&d.text).as_deref(), &self.always_show_calls) {
            return Some(true);
        }
        if !self.show_only_calls.is_empty() && !any_match(call_of(&d.text).as_deref(), &self.show_only_calls) {
            return Some(false);
        }
        if d.continent.as_deref().is_some_and(|c| self.hide_continents.iter().any(|h| h == c)) {
            return Some(false);
        }
        if self.hide_own_continent && own_continent.is_some() && d.continent.as_deref() == own_continent {
            return Some(false);
        }
        if let Some(country) = d.country.as_deref()
            && self.hide_countries.iter().any(|c| c.trim().eq_ignore_ascii_case(country))
        {
            return Some(false);
        }
        if !self.hide_calls.is_empty() {
            let call = call_of(&d.text);
            if self.hide_calls.iter().any(|p| call_matches(call.as_deref(), p)) {
                return Some(false);
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn decode(text: &str, country: Option<&str>, continent: Option<&str>, tag: Option<DecodeTag>) -> DecodedMessage {
        DecodedMessage {
            id: 1,
            time_ms: 0,
            snr: -10,
            delta_time: 0.1,
            freq_hz: 1500,
            text: text.into(),
            tag,
            country: country.map(Into::into),
            continent: continent.map(Into::into),
            hint: false,
        }
    }

    #[test]
    fn no_filter_lets_everything_through() {
        let f = UdpFilters::default();
        assert!(f.allows(&decode("CQ K1ABC FN42", Some("United States"), Some("NA"), None), None));
        assert!(f.allows(&decode("<...> K1ABC RR73", None, None, None), None));
    }

    #[test]
    fn continents_countries_calls_and_own_continent_hide_rows() {
        let f = UdpFilters { hide_continents: vec!["NA".into()], ..UdpFilters::default() };
        assert!(!f.allows(&decode("CQ K1ABC FN42", Some("United States"), Some("NA"), None), None));
        assert!(f.allows(&decode("CQ DL1ABC JO62", Some("Germany"), Some("EU"), None), None));
        let f = UdpFilters { hide_countries: vec![" germany ".into()], ..UdpFilters::default() };
        assert!(!f.allows(&decode("CQ DL1ABC JO62", Some("Germany"), Some("EU"), None), None));
        let f = UdpFilters { hide_calls: vec!["DL1*".into(), "K1ABC".into()], ..UdpFilters::default() };
        assert!(!f.allows(&decode("CQ DL1ABC JO62", None, None, None), None));
        assert!(!f.allows(&decode("IU3ABC K1ABC -10", None, None, None), None), "il secondo nominativo e' chi trasmette");
        assert!(f.allows(&decode("CQ DL2XYZ JO62", None, None, None), None));
        let f = UdpFilters { hide_own_continent: true, ..UdpFilters::default() };
        assert!(!f.allows(&decode("CQ DL1ABC JO62", None, Some("EU"), None), Some("EU")));
        assert!(f.allows(&decode("CQ DL1ABC JO62", None, Some("EU"), None), None), "senza continente proprio non si nasconde niente");
    }

    #[test]
    fn only_cq_and_signoffs_non_standard_and_worked() {
        let cq = decode("CQ DX K1ABC FN42", None, None, None);
        let rr73 = decode("IU3ABC K1ABC RR73", None, None, None);
        let rrr = decode("IU3ABC K1ABC RRR", None, None, None);
        let report = decode("IU3ABC K1ABC -10", None, None, None);
        let only = |o: &str| UdpFilters { only: o.into(), ..UdpFilters::default() };
        assert!(only("all").allows(&report, None) && only("").allows(&report, None));
        assert!(only("cq").allows(&cq, None) && !only("cq").allows(&rr73, None));
        assert!(only("cq_rrr").allows(&rrr, None) && only("cq_rrr").allows(&rr73, None) && !only("cq_rrr").allows(&report, None));
        assert!(only("cq_rr73").allows(&rr73, None) && !only("cq_rr73").allows(&rrr, None));
        let ns = UdpFilters { hide_non_standard: true, ..UdpFilters::default() };
        assert!(!ns.allows(&decode("<K1ABC> IU3ABC -10", None, None, None), None));
        assert!(ns.allows(&report, None));
        let worked = UdpFilters { hide_worked: true, ..UdpFilters::default() };
        assert!(!worked.allows(&decode("CQ K1ABC FN42", None, None, Some(DecodeTag::WorkedCall)), None));
        assert!(worked.allows(&decode("IU3ABC K1ABC -10", None, None, Some(DecodeTag::WorkedCall)), None), "gli altri messaggi restano");
    }

    #[test]
    fn bypass_always_show_show_only_and_worked_all_follow_the_page() {
        let cq = decode("CQ K1ABC FN42", Some("United States"), Some("NA"), None);
        let hide_na = UdpFilters { hide_continents: vec!["NA".into()], ..UdpFilters::default() };
        assert!(UdpFilters { bypass: true, ..hide_na.clone() }.allows(&cq, None), "il bypass sospende tutto");
        assert!(UdpFilters { always_show_calls: vec!["K1*".into()], ..hide_na.clone() }.allows(&cq, None), "sempre visibile vince");
        let only_ja = UdpFilters { show_only_calls: vec!["JA*".into()], ..UdpFilters::default() };
        assert!(!only_ja.allows(&cq, None));
        assert!(only_ja.allows(&decode("CQ JA1ABC PM95", None, None, None), None));
        let all = UdpFilters { hide_worked: true, hide_worked_all: true, ..UdpFilters::default() };
        assert!(!all.allows(&decode("IU3ABC K1ABC -10", None, None, Some(DecodeTag::WorkedCall)), None), "anche gli scambi");
    }

    #[test]
    fn the_page_json_is_read_with_its_own_names_and_missing_fields_default() {
        let f: UdpFilters = serde_json::from_str(r#"{"hideContinents":["AS"],"only":"cq","hideWorked":true}"#).expect("json");
        assert_eq!((f.hide_continents, f.only.as_str(), f.hide_worked, f.hide_calls.len()), (vec!["AS".to_string()], "cq", true, 0));
        // la pagina manda anche le evidenziazioni: campi in piu' non sono un errore
        let f: UdpFilters = serde_json::from_str(r#"{"bypass":true,"highlightCalls":["K1*"],"showOnlyCalls":["JA*"]}"#).expect("json");
        assert!(f.bypass && f.show_only_calls == vec!["JA*".to_string()]);
    }
}
