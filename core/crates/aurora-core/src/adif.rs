//! Lettura dei file ADIF (`<CAMPO:lunghezza>valore`) e dei QSO che contengono, per il log interno, la lista dei gia' lavorati e la scheda DXCC.
//!
//! Le lunghezze sono in byte, quindi si lavora sui byte e non sui caratteri. I nomi dei campi si confrontano in maiuscolo.

use std::collections::HashMap;

/// Un QSO letto da un log (file ADIF, log interno, log di Log4OM2), con i soli campi che servono ad Aurora.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AdifQso {
    /// Nominativo, in maiuscolo.
    pub call: String,
    /// Banda in minuscolo come nell'ADIF (`20m`); se il record ha solo la frequenza la si ricava dalla frequenza.
    pub band: String,
    /// Modo in maiuscolo; `MFSK` con sottomodo `FT4` (come lo scrive JTDX) diventa `FT4`, `JT9...`/`JT65...` diventano `JT9`/`JT65`.
    pub mode: String,
    /// Locatore come scritto nel record (puo' avere 4, 6 o 8 caratteri o essere vuoto).
    pub grid: String,
    /// `QSO_DATE` (`AAAAMMGG`) o vuoto.
    pub date: String,
    /// `TIME_ON` (`HHMMSS`, completato con zeri) o vuoto.
    pub time_on: String,
    pub date_off: String,
    pub time_off: String,
    pub freq_hz: u64,
    pub rst_sent: String,
    pub rst_rcvd: String,
    /// `None` = il record non dice nulla sulle conferme (log di Aurora, `wsjtx_log.adi`); `Some(true)` = almeno una conferma (QSL cartacea,
    /// LoTW, eQSL); `Some(false)` = il record ha i campi delle conferme e nessuno e' confermato.
    pub confirmed: Option<bool>,
}

/// Record ADIF completi (terminati da `<EOR>`), con i nomi dei campi in maiuscolo. Quello che segue l'ultimo `<EOR>` si ignora.
pub fn parse_records(text: &str) -> Vec<HashMap<String, String>> {
    let bytes = text.as_bytes();
    let mut records = Vec::new();
    let mut current: HashMap<String, String> = HashMap::new();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] != b'<' {
            i += 1;
            continue;
        }
        let Some(end) = bytes[i..].iter().position(|b| *b == b'>').map(|p| i + p) else {
            break;
        };
        let tag = String::from_utf8_lossy(&bytes[i + 1..end]).to_uppercase();
        i = end + 1;
        match tag.as_str() {
            "EOH" => current.clear(),
            "EOR" => records.push(std::mem::take(&mut current)),
            _ => {
                let mut parts = tag.split(':');
                let name = parts.next().unwrap_or_default().trim().to_string();
                let Some(len) = parts.next().and_then(|l| l.trim().parse::<usize>().ok()) else {
                    continue;
                };
                let Some(value) = bytes.get(i..i + len) else {
                    break;
                };
                current.insert(name, String::from_utf8_lossy(value).trim().to_string());
                i += len;
            }
        }
    }
    records
}

/// Il modo come lo conta JTDX (`ADIF::load`): `MFSK` vale il suo sottomodo, `JT9*` e `JT65*` il modo base.
fn normalize_mode(mode: &str, submode: &str) -> String {
    let mut m = mode.trim().to_uppercase();
    if m == "MFSK" {
        let sub = submode.trim().to_uppercase();
        if !sub.is_empty() {
            m = sub;
        }
    }
    if m.starts_with("JT9") {
        "JT9".to_string()
    } else if m.starts_with("JT65") {
        "JT65".to_string()
    } else {
        m
    }
}

fn is_yes(v: &str) -> bool {
    matches!(v.trim().to_uppercase().as_str(), "Y" | "V" | "YES" | "TRUE" | "1")
}

/// Orario ADIF completato a sei cifre (`HHMM` diventa `HHMM00`); vuoto resta vuoto.
fn pad_time(t: &str) -> String {
    let digits: String = t.chars().filter(char::is_ascii_digit).take(6).collect();
    if digits.is_empty() { digits } else { format!("{digits:0<6}") }
}

impl AdifQso {
    /// Costruisce il QSO da un record; serve il nominativo e una banda (dal campo `BAND` o dalla frequenza).
    pub fn from_fields(r: &HashMap<String, String>) -> Option<AdifQso> {
        let get = |k: &str| r.get(k).map(String::as_str).unwrap_or("");
        let call = get("CALL").trim().to_uppercase();
        if call.is_empty() {
            return None;
        }
        let freq_hz = get("FREQ").trim().parse::<f64>().ok().filter(|f| *f > 0.0).map_or(0, |mhz| (mhz * 1_000_000.0).round() as u64);
        let mut band = get("BAND").trim().to_lowercase();
        if band.is_empty() && freq_hz > 0 {
            band = band_from_freq_hz(freq_hz).unwrap_or_default().to_string();
        }
        if band.is_empty() {
            return None;
        }
        let qsl = ["QSL_RCVD", "LOTW_QSL_RCVD", "EQSL_QSL_RCVD", "CLUBLOG_QSO_UPLOAD_STATUS_RCVD"];
        let present: Vec<&str> = qsl.iter().filter_map(|k| r.get(*k).map(String::as_str)).collect();
        let confirmed = if present.is_empty() { None } else { Some(present.iter().any(|v| is_yes(v))) };
        Some(AdifQso {
            call,
            band,
            mode: normalize_mode(get("MODE"), get("SUBMODE")),
            grid: get("GRIDSQUARE").trim().to_string(),
            date: get("QSO_DATE").trim().to_string(),
            time_on: pad_time(get("TIME_ON")),
            date_off: get("QSO_DATE_OFF").trim().to_string(),
            time_off: pad_time(get("TIME_OFF")),
            freq_hz,
            rst_sent: get("RST_SENT").trim().to_string(),
            rst_rcvd: get("RST_RCVD").trim().to_string(),
            confirmed,
        })
    }
}

/// Tutti i QSO validi di un testo ADIF (i record senza nominativo o senza banda si scartano).
pub fn parse_qsos(text: &str) -> Vec<AdifQso> {
    parse_records(text).iter().filter_map(AdifQso::from_fields).collect()
}

/// La banda (`20m`) da una frequenza in Hz, con gli stessi intervalli di JTDX (`ADIF::bandFromFrequency`); `None` fuori dalle bande.
pub fn band_from_freq_hz(hz: u64) -> Option<&'static str> {
    let mhz = hz as f64 / 1e6;
    const TABLE: &[(&str, f64, f64)] = &[
        ("2200m", 0.135, 0.139),
        ("630m", 0.45, 0.55),
        ("160m", 1.8, 2.0),
        ("80m", 3.5, 4.0),
        ("60m", 5.1, 5.45),
        ("40m", 7.0, 7.3),
        ("30m", 10.0, 10.15),
        ("20m", 14.0, 14.35),
        ("17m", 18.068, 18.168),
        ("15m", 21.0, 21.45),
        ("12m", 24.890, 24.990),
        ("10m", 28.0, 29.7),
        ("6m", 50.0, 54.0),
        ("4m", 70.0, 71.0),
        ("2m", 144.0, 148.0),
        ("1.25m", 222.0, 225.0),
        ("70cm", 420.0, 450.0),
        ("33cm", 902.0, 928.0),
        ("23cm", 1240.0, 1300.0),
        ("13cm", 2300.0, 2450.0),
        ("9cm", 3300.0, 3500.0),
        ("6cm", 5650.0, 5925.0),
        ("3cm", 10000.0, 10500.0),
        ("1.25cm", 24000.0, 24250.0),
        ("6mm", 47000.0, 47200.0),
        ("4mm", 75500.0, 81000.0),
    ];
    TABLE.iter().find(|(_, lo, hi)| mhz > *lo && mhz < *hi).map(|(name, _, _)| *name)
}

/// Millisecondi Unix (UTC) da `AAAAMMGG` e `HHMMSS`; `None` se la data non e' valida. L'ora vuota vale mezzanotte.
pub fn unix_ms(date: &str, time: &str) -> Option<u64> {
    let d: Vec<u32> = date.bytes().filter(u8::is_ascii_digit).map(|b| u32::from(b - b'0')).collect();
    if d.len() != 8 {
        return None;
    }
    let year = i64::from(d[0] * 1000 + d[1] * 100 + d[2] * 10 + d[3]);
    let month = i64::from(d[4] * 10 + d[5]);
    let day = i64::from(d[6] * 10 + d[7]);
    if !(1..=12).contains(&month) || !(1..=31).contains(&day) || year < 1970 {
        return None;
    }
    // giorni dal 1970-01-01 (algoritmo di Howard Hinnant)
    let y = if month <= 2 { year - 1 } else { year };
    let era = y.div_euclid(400);
    let yoe = y.rem_euclid(400);
    let mp = (month + 9) % 12;
    let doy = (153 * mp + 2) / 5 + day - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    let days = era * 146_097 + doe - 719_468;
    let t = pad_time(time);
    let secs = if t.len() == 6 {
        let n = |i: usize| t[i..i + 2].parse::<i64>().unwrap_or(0);
        n(0) * 3600 + n(2) * 60 + n(4)
    } else {
        0
    };
    u64::try_from(days * 86_400 + secs).ok().map(|s| s * 1000)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_records_with_types_lowercase_tags_and_utf8_lengths() {
        let text = "Prova\n<ADIF_VER:5>3.1.4 <EOH>\n<CALL:5>AT3AB<BAND:3>20m<RST_SENT:3:S>-10<COMMENT:6>caffè<EOR>\n<call:5>AT4CD<band:3>40m<eor>\n<CALL:3>XXX";
        let r = parse_records(text);
        assert_eq!(r.len(), 2, "il record senza <EOR> non conta");
        assert_eq!(r[0]["COMMENT"], "caffè");
        assert_eq!(r[1]["CALL"], "AT4CD");
    }

    #[test]
    fn ft4_as_mfsk_jt9_variants_and_the_band_from_the_frequency() {
        let q = parse_qsos("<call:5>K1ABC<mode:4>MFSK<submode:3>FT4<freq:9>14.080000<eor><CALL:5>K1ABD<MODE:5>JT65B<BAND:3>20M<EOR>");
        assert_eq!(q.len(), 2);
        assert_eq!((q[0].mode.as_str(), q[0].band.as_str(), q[0].freq_hz), ("FT4", "20m", 14_080_000));
        assert_eq!((q[1].mode.as_str(), q[1].band.as_str()), ("JT65", "20m"));
    }

    #[test]
    fn records_without_call_or_band_are_dropped() {
        assert!(parse_qsos("<CALL:5>AT3AB<EOR><BAND:3>20m<EOR>").is_empty());
        assert_eq!(parse_qsos("<CALL:5>AT3AB<BAND:3>20m<EOR>").len(), 1);
    }

    #[test]
    fn confirmations_are_unknown_without_the_fields_and_known_with_them() {
        let q = parse_qsos(
            "<CALL:1>A<BAND:3>20m<EOR><CALL:1>B<BAND:3>20m<QSL_RCVD:1>N<LOTW_QSL_RCVD:1>N<EOR><CALL:1>C<BAND:3>20m<QSL_RCVD:1>N<LOTW_QSL_RCVD:1>Y<EOR>",
        );
        assert_eq!(q.iter().map(|x| x.confirmed).collect::<Vec<_>>(), vec![None, Some(false), Some(true)]);
    }

    #[test]
    fn times_are_padded_and_dates_become_unix_ms() {
        let q = parse_qsos("<CALL:1>A<BAND:3>20m<QSO_DATE:8>20261024<TIME_ON:4>1646<EOR>");
        assert_eq!(q[0].time_on, "164600");
        assert_eq!(unix_ms("20261024", "164645"), Some(1_792_860_405_000));
        assert_eq!(unix_ms("19700101", ""), Some(0));
        assert_eq!(unix_ms("20261324", "000000"), None);
        assert_eq!(unix_ms("", "000000"), None);
    }

    #[test]
    fn band_from_frequency_follows_jtdx() {
        assert_eq!(band_from_freq_hz(14_074_000), Some("20m"));
        assert_eq!(band_from_freq_hz(70_154_000), Some("4m"));
        assert_eq!(band_from_freq_hz(144_174_000), Some("2m"));
        assert_eq!(band_from_freq_hz(432_100_000), Some("70cm"));
        assert_eq!(band_from_freq_hz(15_000_000), None);
    }
}
