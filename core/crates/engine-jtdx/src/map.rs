use aurora_core::{DecodeTag, DecodedMessage, QsoRecord, StationStatus};
use udp_bridge::{Decode as WireDecode, Message, Status as WireStatus};

fn non_empty(text: Option<&str>) -> Option<String> {
    text.map(str::trim).filter(|t| !t.is_empty()).map(str::to_string)
}

pub fn to_status(s: &WireStatus) -> StationStatus {
    StationStatus {
        dial_hz: s.dial_freq,
        mode: s.mode.clone().unwrap_or_default(),
        tx_enabled: s.tx_enabled,
        transmitting: s.transmitting,
        decoding: s.decoding,
        de_call: s.de_call.clone().unwrap_or_default(),
        de_grid: s.de_grid.clone().unwrap_or_default(),
        dx_call: non_empty(s.dx_call.as_deref()),
        dx_grid: non_empty(s.dx_grid.as_deref()),
        report: non_empty(s.report.as_deref()),
        tx_df: Some(s.tx_df),
        rx_df: Some(s.rx_df),
        tx_message: non_empty(s.tx_message.as_deref()),
        tr_period: s.tr_period,
        own_continent: None,
        meters: None,
    }
}

/// Versione per l'interfaccia: testo senza spazi finali. L'originale resta nello store per il Reply.
pub fn to_decoded(id: u64, d: &WireDecode) -> DecodedMessage {
    let text = d.message.as_deref().unwrap_or_default().trim_end().to_string();
    let is_cq = ["CQ ", "CQDX ", "QRZ"].iter().any(|p| text.starts_with(p));
    DecodedMessage {
        id,
        time_ms: d.time_ms,
        snr: d.snr,
        delta_time: d.delta_time as f32,
        freq_hz: d.delta_freq,
        text,
        tag: is_cq.then_some(DecodeTag::Cq),
        country: None,
        continent: None,
        hint: false,
    }
}

// [nome, da (kHz), a (kHz)]
const BANDS: &[(&str, u64, u64)] = &[
    ("160m", 1800, 2000),
    ("80m", 3500, 4000),
    ("60m", 5250, 5450),
    ("40m", 7000, 7300),
    ("30m", 10_100, 10_150),
    ("20m", 14_000, 14_350),
    ("17m", 18_068, 18_168),
    ("15m", 21_000, 21_450),
    ("12m", 24_890, 24_990),
    ("10m", 28_000, 29_700),
    ("6m", 50_000, 54_000),
    ("2m", 144_000, 148_000),
];

pub fn band_of(hz: u64) -> Option<&'static str> {
    let khz = hz / 1000;
    BANDS.iter().find(|(_, from, to)| khz >= *from && khz <= *to).map(|(name, _, _)| *name)
}

pub fn to_qso(m: &Message, time_ms: u32) -> Option<QsoRecord> {
    let Message::QsoLogged { dx_call, dx_grid, tx_freq, mode, report_sent, report_received, .. } = m else {
        return None;
    };
    Some(QsoRecord {
        call: dx_call.clone().unwrap_or_default(),
        grid: dx_grid.clone().unwrap_or_default(),
        band: band_of(*tx_freq).unwrap_or_default().to_string(),
        mode: mode.clone().unwrap_or_default(),
        freq_hz: *tx_freq,
        rst_sent: report_sent.clone().unwrap_or_default(),
        rst_rcvd: report_received.clone().unwrap_or_default(),
        time_ms,
        // il messaggio di JTDX porta anche inizio e fine, ma Aurora non lo decodifica: con JTDX il log e' il suo
        time_on_unix_ms: 0,
        time_off_unix_ms: 0,
        ..Default::default()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn wire(text: &str) -> WireDecode {
        WireDecode {
            id: Some("JTDX".into()),
            is_new: true,
            time_ms: 25_905_000,
            snr: -10,
            delta_time: 0.2,
            delta_freq: 1333,
            mode: Some("~".into()),
            message: Some(text.into()),
            low_confidence: false,
            off_air: false,
        }
    }

    #[test]
    fn to_decoded_trims_trailing_spaces_and_tags_cq() {
        let d = to_decoded(7, &wire("CQ AA1AA FN42          "));
        assert_eq!(d.id, 7);
        assert_eq!(d.text, "CQ AA1AA FN42");
        assert_eq!(d.tag, Some(DecodeTag::Cq));
        assert_eq!((d.time_ms, d.snr, d.freq_hz), (25_905_000, -10, 1333));
        assert!((d.delta_time - 0.2).abs() < 1e-6);
    }

    #[test]
    fn to_decoded_tags_cqdx_and_qrz_but_not_a_reply() {
        assert_eq!(to_decoded(1, &wire("CQDX AA1AA FN42")).tag, Some(DecodeTag::Cq));
        assert_eq!(to_decoded(1, &wire("QRZ AA1AA")).tag, Some(DecodeTag::Cq));
        assert_eq!(to_decoded(1, &wire("BB2BB AA1AA RR73")).tag, None);
    }

    #[test]
    fn to_decoded_keeps_angle_bracket_text() {
        assert_eq!(to_decoded(1, &wire("<...> AA1AA MP72")).text, "<...> AA1AA MP72");
    }

    #[test]
    fn to_decoded_handles_missing_text() {
        let mut w = wire("x");
        w.message = None;
        assert_eq!(to_decoded(1, &w).text, "");
    }

    #[test]
    fn band_of_known_bands_and_none_outside() {
        assert_eq!(band_of(14_074_000), Some("20m"));
        assert_eq!(band_of(21_074_000), Some("15m"));
        assert_eq!(band_of(7_074_000), Some("40m"));
        assert_eq!(band_of(50_313_000), Some("6m"));
        assert_eq!(band_of(0), None);
        assert_eq!(band_of(100_000_000), None);
    }

    #[test]
    fn to_qso_maps_a_logged_qso_and_ignores_other_messages() {
        let m = Message::QsoLogged {
            id: Some("JTDX".into()),
            dx_call: Some("AA1AA".into()),
            dx_grid: Some("FN42".into()),
            tx_freq: 14_074_000,
            mode: Some("FT8".into()),
            report_sent: Some("-08".into()),
            report_received: Some("-12".into()),
            extra: None,
        };
        let q = to_qso(&m, 45_015_000).unwrap();
        assert_eq!((q.call.as_str(), q.grid.as_str(), q.band.as_str(), q.mode.as_str()), ("AA1AA", "FN42", "20m", "FT8"));
        assert_eq!((q.freq_hz, q.rst_sent.as_str(), q.rst_rcvd.as_str(), q.time_ms), (14_074_000, "-08", "-12", 45_015_000));
        assert!(to_qso(&Message::Close { id: None }, 0).is_none());
    }
}
