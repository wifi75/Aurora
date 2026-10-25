//! Rapporto segnale: dal valore misurato (SNR) al testo da trasmettere, e viceversa.
//!
//! JTDX scrive il rapporto con `%+2.2d` (segno e due cifre: «-07», «+05», «+00»), accetta in
//! ricezione i valori da -50 a +49 (`r.toInt()>=-50 and <=49`, `mainwindow.cpp` in
//! `processMessage`) e mette una «R» davanti nel Tx3.

pub const REPORT_MIN: i32 = -50;
pub const REPORT_MAX: i32 = 49;

/// Porta un SNR nell'intervallo che un messaggio standard puo' portare.
pub fn clamp_report(snr: i32) -> i32 {
    snr.clamp(REPORT_MIN, REPORT_MAX)
}

/// «-07», «+05», «+00».
pub fn format_report(snr: i32) -> String {
    format!("{:+03}", clamp_report(snr))
}

/// «R-07», «R+05»: il rapporto con la conferma, come nel Tx3.
pub fn format_r_report(snr: i32) -> String {
    format!("R{}", format_report(snr))
}

/// Legge «-07» o «+05» (segno e due cifre). Fuori da -50..=49 non e' un rapporto.
pub fn parse_report(token: &str) -> Option<i32> {
    let b = token.as_bytes();
    if b.len() != 3
        || !(b[0] == b'+' || b[0] == b'-')
        || !b[1].is_ascii_digit()
        || !b[2].is_ascii_digit()
    {
        return None;
    }
    let n: i32 = token.parse().ok()?;
    (REPORT_MIN..=REPORT_MAX).contains(&n).then_some(n)
}

/// Legge «R-07» o «R+05».
pub fn parse_r_report(token: &str) -> Option<i32> {
    parse_report(token.strip_prefix('R')?)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// JTDX `genStdMsgs`: `QString::asprintf("%+2.2d", n)`.
    #[test]
    fn format_matches_printf_plus_2_2d() {
        assert_eq!(format_report(-7), "-07");
        assert_eq!(format_report(5), "+05");
        assert_eq!(format_report(0), "+00");
        assert_eq!(format_report(-24), "-24");
        assert_eq!(format_report(12), "+12");
        assert_eq!(format_r_report(-7), "R-07");
        assert_eq!(format_r_report(5), "R+05");
    }

    /// JTDX `processMessage`: il rapporto valido va da -50 a 49.
    #[test]
    fn out_of_range_is_clamped_when_sending() {
        assert_eq!(format_report(-80), "-50");
        assert_eq!(format_report(120), "+49");
    }

    #[test]
    fn parse_accepts_only_signed_two_digits() {
        assert_eq!(parse_report("-07"), Some(-7));
        assert_eq!(parse_report("+05"), Some(5));
        assert_eq!(parse_report("+49"), Some(49));
        assert_eq!(parse_report("+50"), None);
        assert_eq!(parse_report("-51"), None);
        assert_eq!(parse_report("07"), None);
        assert_eq!(parse_report("-7"), None);
        assert_eq!(parse_report("RR73"), None);
        assert_eq!(parse_r_report("R-07"), Some(-7));
        assert_eq!(parse_r_report("R+05"), Some(5));
        assert_eq!(parse_r_report("-07"), None);
        assert_eq!(parse_r_report("RRR"), None);
    }
}
