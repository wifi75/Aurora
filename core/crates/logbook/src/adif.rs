//! Formato ADIF: ogni campo e' `<NOME:lunghezza-in-byte>valore`.

use std::fmt::Write;

use aurora_core::{
    version::{APP_NAME, APP_VERSION},
    QsoRecord,
};

pub fn field(out: &mut String, name: &str, value: &str) {
    if !value.is_empty() {
        let _ = write!(out, "<{name}:{}>{value}", value.len());
    }
}

pub fn header() -> String {
    let mut h = String::from("Esportazione Aurora\n");
    field(&mut h, "ADIF_VER", "3.1.4");
    field(&mut h, "PROGRAMID", APP_NAME);
    field(&mut h, "PROGRAMVERSION", APP_VERSION);
    h.push_str("<EOH>\n");
    h
}

/// `date` e' `AAAAMMGG` (UTC) del momento in cui il QSO e' stato registrato.
pub fn record(q: &QsoRecord, date: &str) -> String {
    let secs = q.time_ms / 1000;
    let time_on = format!("{:02}{:02}{:02}", secs / 3600 % 24, secs / 60 % 60, secs % 60);
    let freq_mhz = format!("{}.{:06}", q.freq_hz / 1_000_000, q.freq_hz % 1_000_000);
    let mut r = String::new();
    field(&mut r, "CALL", &q.call);
    field(&mut r, "GRIDSQUARE", &q.grid);
    field(&mut r, "BAND", &q.band);
    field(&mut r, "MODE", &q.mode);
    field(&mut r, "FREQ", &freq_mhz);
    field(&mut r, "RST_SENT", &q.rst_sent);
    field(&mut r, "RST_RCVD", &q.rst_rcvd);
    field(&mut r, "QSO_DATE", date);
    field(&mut r, "TIME_ON", &time_on);
    r.push_str("<EOR>\n");
    r
}

/// Chi registra il QSO: i dati della stazione che JTDX mette in ogni record (`STATION_CALLSIGN`, `MY_GRIDSQUARE`, `TX_PWR`).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Station {
    pub call: String,
    pub grid: String,
    /// Potenza in watt come l'ha scritta l'operatore; vuota = nessun `TX_PWR` (come JTDX con «Save Tx power» spento).
    pub tx_power: String,
}

/// Data (`AAAAMMGG`) e ora (`HHMMSS`) UTC da millisecondi Unix.
pub fn utc_datetime(unix_ms: u64) -> (String, String) {
    let secs = unix_ms / 1000;
    (utc_date(secs), format!("{:02}{:02}{:02}", secs / 3600 % 24, secs / 60 % 60, secs % 60))
}

/// Inizio e fine del QSO come `(QSO_DATE, TIME_ON, QSO_DATE_OFF, TIME_OFF)`. Senza gli istanti completi si usa l'ora del giorno di `time_ms`
/// con la data `fallback_date`, per inizio e fine.
fn qso_times(q: &QsoRecord, fallback_date: &str) -> (String, String, String, String) {
    if q.time_on_unix_ms == 0 {
        let secs = q.time_ms / 1000;
        let time = format!("{:02}{:02}{:02}", secs / 3600 % 24, secs / 60 % 60, secs % 60);
        return (fallback_date.to_string(), time.clone(), fallback_date.to_string(), time);
    }
    let (date_on, time_on) = utc_datetime(q.time_on_unix_ms);
    let off = if q.time_off_unix_ms >= q.time_on_unix_ms { q.time_off_unix_ms } else { q.time_on_unix_ms };
    let (date_off, time_off) = utc_datetime(off);
    (date_on, time_on, date_off, time_off)
}

fn freq_mhz(freq_hz: u64) -> String {
    format!("{}.{:06}", freq_hz / 1_000_000, freq_hz % 1_000_000)
}

/// Un campo come lo scrive JTDX: sempre, anche vuoto (`<RST_SENT:0>`), preceduto da uno spazio.
fn jtdx_field(out: &mut String, name: &str, value: &str) {
    let _ = write!(out, " <{name}:{}>{value}", value.len());
}

/// Il modo come lo scrive JTDX: FT4 e' `MFSK` con sottomodo `FT4`, gli altri col proprio nome.
fn jtdx_mode(out: &mut String, mode: &str, upper: bool) {
    if mode == "FT4" {
        out.push_str(if upper { " <MODE:4>MFSK" } else { " <mode:4>MFSK" });
        jtdx_field(out, if upper { "SUBMODE" } else { "submode" }, mode);
    } else {
        jtdx_field(out, if upper { "MODE" } else { "mode" }, mode);
    }
}

/// Il record ADIF che JTDX manda al secondo server UDP (`myadif2` di `LogQSO::accept`, quello di Log4OM2): campi in questo ordine,
/// nomi in maiuscolo, separati da uno spazio, nessuna intestazione, chiuso da `<EOR>` senza andare a capo.
/// `COMMENT` e `NAME` non ci sono: Aurora non li ha.
pub fn jtdx_record(q: &QsoRecord, station: &Station, fallback_date: &str) -> String {
    let (date_on, time_on, date_off, time_off) = qso_times(q, fallback_date);
    let mut r = String::new();
    jtdx_field(&mut r, "BAND", &q.band);
    jtdx_field(&mut r, "STATION_CALLSIGN", &station.call);
    // JTDX: 8 caratteri se il locatore ne ha almeno 8, intero se ne ha almeno 4, altrimenti niente
    if station.grid.len() > 7 {
        jtdx_field(&mut r, "MY_GRIDSQUARE", station.grid.get(..8).unwrap_or(&station.grid));
    } else if station.grid.len() > 3 {
        jtdx_field(&mut r, "MY_GRIDSQUARE", &station.grid);
    }
    jtdx_field(&mut r, "CALL", &q.call);
    jtdx_field(&mut r, "FREQ", &freq_mhz(q.freq_hz));
    jtdx_mode(&mut r, &q.mode, true);
    jtdx_field(&mut r, "QSO_DATE", &date_on);
    jtdx_field(&mut r, "TIME_ON", &time_on);
    jtdx_field(&mut r, "QSO_DATE_OFF", &date_off);
    jtdx_field(&mut r, "TIME_OFF", &time_off);
    jtdx_field(&mut r, "RST_SENT", &q.rst_sent);
    jtdx_field(&mut r, "RST_RCVD", &q.rst_rcvd);
    if !station.tx_power.is_empty() {
        jtdx_field(&mut r, "TX_PWR", &station.tx_power);
    }
    if q.grid.len() > 3 {
        jtdx_field(&mut r, "GRIDSQUARE", &q.grid);
    }
    r.push_str(" <EOR>");
    r.trim().to_string()
}

/// L'intestazione del file ADIF di JTDX (`wsjtx_log.adi`): una riga, poi i record. Va a capo come JTDX su Windows (CR LF).
pub const JTDX_FILE_HEADER: &str = "WSJT-X ADIF Export<eoh>\r\n";

/// La riga che JTDX aggiunge a `wsjtx_log.adi` (`ADIF::addQSOToFile`), con CR LF: nomi in minuscolo, locatore anche se vuoto
/// (`<gridsquare:0>`), `STATION_CALLSIGN` e `MY_GRIDSQUARE` interi, `TX_PWR` solo se c'e'. Ordine diverso da quello del datagramma UDP.
pub fn jtdx_file_record(q: &QsoRecord, station: &Station, fallback_date: &str) -> String {
    let (date_on, time_on, date_off, time_off) = qso_times(q, fallback_date);
    let mut r = String::from("<call:");
    let _ = write!(r, "{}>{}", q.call.len(), q.call);
    jtdx_field(&mut r, "gridsquare", &q.grid);
    jtdx_mode(&mut r, &q.mode, false);
    jtdx_field(&mut r, "rst_sent", &q.rst_sent);
    jtdx_field(&mut r, "rst_rcvd", &q.rst_rcvd);
    jtdx_field(&mut r, "qso_date", &date_on);
    jtdx_field(&mut r, "time_on", &time_on);
    jtdx_field(&mut r, "qso_date_off", &date_off);
    jtdx_field(&mut r, "time_off", &time_off);
    jtdx_field(&mut r, "band", &q.band);
    jtdx_field(&mut r, "freq", &freq_mhz(q.freq_hz));
    jtdx_field(&mut r, "station_callsign", &station.call);
    jtdx_field(&mut r, "my_gridsquare", &station.grid);
    if !station.tx_power.is_empty() {
        jtdx_field(&mut r, "tx_pwr", &station.tx_power);
    }
    r.push_str(" <eor>\r\n");
    r
}

/// Data civile UTC (`AAAAMMGG`) da secondi Unix (algoritmo di Howard Hinnant).
pub fn utc_date(unix_secs: u64) -> String {
    let days = (unix_secs / 86_400) as i64;
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    format!("{y:04}{m:02}{d:02}")
}

#[cfg(test)]
mod tests {
    use aurora_core::QsoRecord;

    use super::{jtdx_file_record, jtdx_record, utc_date, utc_datetime, Station};

    fn sample(mode: &str) -> QsoRecord {
        QsoRecord {
            call: "K1ABC".into(),
            grid: "FN42".into(),
            band: "20m".into(),
            mode: mode.into(),
            freq_hz: 14_075_084,
            rst_sent: "-16".into(),
            rst_rcvd: "-09".into(),
            time_ms: 0,
            // 2026-10-24 16:46:45 e 16:47:29 UTC
            time_on_unix_ms: 1_792_860_405_000,
            time_off_unix_ms: 1_792_860_449_000,
        }
    }

    fn station() -> Station {
        Station { call: "IU3ABC".into(), grid: "IO91wm".into(), tx_power: String::new() }
    }

    #[test]
    fn the_udp_record_is_the_one_jtdx_sends_to_its_second_server() {
        assert_eq!(
            jtdx_record(&sample("FT8"), &station(), "20000101"),
            "<BAND:3>20m <STATION_CALLSIGN:6>IU3ABC <MY_GRIDSQUARE:6>IO91wm <CALL:5>K1ABC <FREQ:9>14.075084 <MODE:3>FT8 \
             <QSO_DATE:8>20261024 <TIME_ON:6>164645 <QSO_DATE_OFF:8>20261024 <TIME_OFF:6>164729 \
             <RST_SENT:3>-16 <RST_RCVD:3>-09 <GRIDSQUARE:4>FN42 <EOR>"
        );
    }

    #[test]
    fn ft4_is_mfsk_with_the_ft4_submode_and_power_comes_after_the_reports() {
        let st = Station { tx_power: "50".into(), ..station() };
        let r = jtdx_record(&sample("FT4"), &st, "20000101");
        assert!(r.contains(" <MODE:4>MFSK <SUBMODE:3>FT4 <QSO_DATE:8>"), "{r}");
        assert!(r.contains("<RST_RCVD:3>-09 <TX_PWR:2>50 <GRIDSQUARE:4>FN42 <EOR>"), "{r}");
        assert!(!r.ends_with(' ') && !r.contains('\n'));
    }

    #[test]
    fn without_a_dx_grid_or_a_long_own_grid_the_fields_follow_jtdx() {
        let mut q = sample("FT8");
        q.grid.clear();
        q.rst_rcvd.clear();
        let st = Station { grid: "IO91wm12ab".into(), ..station() };
        let r = jtdx_record(&q, &st, "20000101");
        assert!(r.contains("<MY_GRIDSQUARE:8>IO91wm12 <CALL:5>"), "{r}");
        assert!(r.contains("<RST_RCVD:0> <EOR>"), "senza rapporto il campo c'e' lo stesso, vuoto: {r}");
        assert!(!r.contains("GRIDSQUARE:4"));
        let short = Station { grid: "IO9".into(), ..station() };
        assert!(!jtdx_record(&q, &short, "20000101").contains("MY_GRIDSQUARE"));
    }

    #[test]
    fn the_file_line_has_the_wsjtx_log_layout() {
        assert_eq!(
            jtdx_file_record(&sample("FT8"), &station(), "20000101"),
            "<call:5>K1ABC <gridsquare:4>FN42 <mode:3>FT8 <rst_sent:3>-16 <rst_rcvd:3>-09 <qso_date:8>20261024 <time_on:6>164645 \
             <qso_date_off:8>20261024 <time_off:6>164729 <band:3>20m <freq:9>14.075084 <station_callsign:6>IU3ABC \
             <my_gridsquare:6>IO91wm <eor>\r\n"
        );
        let mut q = sample("FT4");
        q.grid.clear();
        let r = jtdx_file_record(&q, &station(), "20000101");
        assert!(r.starts_with("<call:5>K1ABC <gridsquare:0> <mode:4>MFSK <submode:3>FT4 <rst_sent:3>"), "{r}");
    }

    #[test]
    fn a_qso_that_crosses_midnight_has_two_dates() {
        let mut q = sample("FT8");
        q.time_on_unix_ms = 1_792_886_390_000;
        q.time_off_unix_ms = 1_792_886_404_000;
        let r = jtdx_record(&q, &station(), "20000101");
        assert!(r.contains("<QSO_DATE:8>20261024 <TIME_ON:6>235950 <QSO_DATE_OFF:8>20261025 <TIME_OFF:6>000004"), "{r}");
    }

    #[test]
    fn without_full_times_the_fallback_date_and_the_time_of_day_are_used() {
        let mut q = sample("FT8");
        q.time_on_unix_ms = 0;
        q.time_off_unix_ms = 0;
        q.time_ms = 45_015_000;
        let r = jtdx_record(&q, &station(), "20261026");
        assert!(r.contains("<QSO_DATE:8>20261026 <TIME_ON:6>123015 <QSO_DATE_OFF:8>20261026 <TIME_OFF:6>123015"), "{r}");
        assert_eq!(utc_datetime(1_792_860_405_000), ("20261024".to_string(), "164645".to_string()));
    }

    #[test]
    fn utc_date_known_values() {
        assert_eq!(utc_date(0), "19700101");
        assert_eq!(utc_date(951_782_400), "20000229");
        assert_eq!(utc_date(1_793_000_000), "20261026");
    }
}
