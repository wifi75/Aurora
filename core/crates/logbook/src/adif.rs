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
    use super::utc_date;

    #[test]
    fn utc_date_known_values() {
        assert_eq!(utc_date(0), "19700101");
        assert_eq!(utc_date(951_782_400), "20000229");
        assert_eq!(utc_date(1_793_000_000), "20261026");
    }
}
