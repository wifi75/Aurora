//! Gli spot pubblici degli attivatori POTA (`api.pota.app/spot/activator`): per ogni nominativo il parco (riferimento e nome), la
//! frequenza e il modo. Serve all'etichetta dei CQ speciali («CQ POTA ...», richiesta #115). Si legge soltanto: Aurora non manda niente
//! (ne' spot ne' nominativo), e la richiesta parte solo se l'utente accende «Cerca il parco negli spot POTA».

use std::collections::BTreeMap;

use serde::Serialize;
use serde_json::Value;

pub const URL: &str = "https://api.pota.app/spot/activator";
/// Quanti spot si tengono al massimo (la lista pubblica ne ha di solito qualche centinaio).
pub const MAX_SPOTS: usize = 2000;

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct PotaSpot {
    pub activator: String,
    pub reference: String,
    pub name: String,
    /// In kHz, come negli spot POTA (0 = sconosciuta).
    pub freq_khz: f64,
    pub mode: String,
}

fn text(item: &Value, keys: &[&str]) -> String {
    keys.iter().filter_map(|k| item.get(*k)).find_map(|v| v.as_str().map(|s| s.trim().to_string()).filter(|s| !s.is_empty())).unwrap_or_default()
}

/// La frequenza arriva come testo («14074.0») o come numero.
fn khz(item: &Value) -> f64 {
    match item.get("frequency") {
        Some(Value::Number(n)) => n.as_f64().unwrap_or(0.0),
        Some(Value::String(s)) => s.trim().parse::<f64>().unwrap_or(0.0),
        _ => 0.0,
    }
}

/// Gli spot della risposta, per nominativo (maiuscolo); se un nominativo ha piu' spot vince il primo, cioe' il piu' recente della lista.
/// Una risposta che non e' una lista JSON e' un errore; le voci senza nominativo o senza riferimento si saltano.
pub fn parse(body: &str) -> Result<BTreeMap<String, PotaSpot>, String> {
    let value: Value = serde_json::from_str(body).map_err(|e| format!("risposta non riconosciuta da POTA: {e}"))?;
    let items = value.as_array().ok_or_else(|| "risposta non riconosciuta da POTA: manca la lista degli spot".to_string())?;
    let mut out = BTreeMap::new();
    for item in items.iter().take(MAX_SPOTS) {
        let activator = text(item, &["activator"]).to_ascii_uppercase();
        let reference = text(item, &["reference"]).to_ascii_uppercase();
        if activator.is_empty() || reference.is_empty() || activator.len() > 20 || reference.len() > 20 {
            continue;
        }
        let spot = PotaSpot { activator: activator.clone(), reference, name: text(item, &["name", "parkName"]), freq_khz: khz(item), mode: text(item, &["mode"]).to_ascii_uppercase() };
        out.entry(activator).or_insert(spot);
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"[
      {"spotId": 1, "activator": "iu3abc/p", "frequency": "14074.0", "mode": "FT8", "reference": "IT-0001", "name": "Parco di prova", "spotTime": "2026-11-28T10:00:00"},
      {"spotId": 2, "activator": "IU3ABC/P", "frequency": "7074", "mode": "FT8", "reference": "IT-0002", "name": "Altro parco"},
      {"spotId": 3, "activator": "IU3ABD", "frequency": 10136.5, "mode": "ft4", "reference": "IT-0003", "parkName": "Parco col nome vecchio"},
      {"spotId": 4, "activator": "", "frequency": "14074", "reference": "IT-0004"},
      {"spotId": 5, "activator": "IU3ABE", "frequency": "x"}
    ]"#;

    #[test]
    fn reads_the_spots_by_callsign() {
        let spots = parse(SAMPLE).expect("lista valida");
        assert_eq!(spots.len(), 2);
        let a = &spots["IU3ABC/P"];
        assert_eq!((a.reference.as_str(), a.name.as_str(), a.mode.as_str()), ("IT-0001", "Parco di prova", "FT8"));
        assert!((a.freq_khz - 14074.0).abs() < 1e-9);
        let b = &spots["IU3ABD"];
        assert_eq!((b.name.as_str(), b.mode.as_str()), ("Parco col nome vecchio", "FT4"));
        assert!((b.freq_khz - 10136.5).abs() < 1e-9);
    }

    #[test]
    fn rejects_what_is_not_a_list() {
        assert!(parse("<html>errore</html>").is_err());
        assert!(parse(r#"{"error": "busy"}"#).is_err());
        assert_eq!(parse("[]").map(|m| m.len()), Ok(0));
    }
}
