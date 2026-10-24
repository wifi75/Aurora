//! Lettura delle righe di spot dei cluster (DXSpider, AR-Cluster, CC Cluster, RBN):
//! `DX de W3LPL:     14074.0  JA1ABC       FT8 -10 dB                     1234Z IO91`

/// Uno spot letto da una riga, senza i dati che gli da' il servizio (numero, server, paese).
#[derive(Debug, Clone, PartialEq)]
pub struct Parsed {
    pub spotter: String,
    pub freq_khz: f64,
    pub dx_call: String,
    pub comment: String,
    /// Ora UTC dello spot, `HHMM`.
    pub time_utc: String,
    pub grid: Option<String>,
    pub band: Option<&'static str>,
    pub mode: Option<String>,
}

const BANDS: &[(&str, f64, f64)] = &[
    ("160m", 1800.0, 2000.0),
    ("80m", 3500.0, 4000.0),
    ("60m", 5250.0, 5450.0),
    ("40m", 7000.0, 7300.0),
    ("30m", 10100.0, 10150.0),
    ("20m", 14000.0, 14350.0),
    ("17m", 18068.0, 18168.0),
    ("15m", 21000.0, 21450.0),
    ("12m", 24890.0, 24990.0),
    ("10m", 28000.0, 29700.0),
    ("6m", 50000.0, 54000.0),
    ("4m", 70000.0, 70500.0),
    ("2m", 144000.0, 148000.0),
    ("70cm", 430000.0, 440000.0),
];

/// Frequenze di lavoro dell'FT8 (kHz): se lo spot e' li' vicino ed e' senza altro indizio si dice FT8.
const FT8_KHZ: &[f64] = &[1840.0, 3573.0, 5357.0, 7074.0, 10136.0, 14074.0, 18100.0, 21074.0, 24915.0, 28074.0, 50313.0];

pub fn band_of_khz(khz: f64) -> Option<&'static str> {
    BANDS.iter().find(|(_, lo, hi)| khz >= *lo && khz <= *hi).map(|(name, _, _)| *name)
}

const MODES: &[&str] = &["FT8", "FT4", "MSK144", "JT65", "JT9", "WSPR", "RTTY", "PSK31", "PSK", "CW", "SSB", "USB", "LSB", "FM", "AM", "SSTV"];

/// Il modo dal commento (`FT8 -10 dB`, `CW 22 wpm`...) o, mancando, dalla frequenza dell'FT8.
pub fn guess_mode(comment: &str, khz: f64) -> Option<String> {
    let upper = comment.to_uppercase();
    let words: Vec<&str> = upper.split(|c: char| !c.is_ascii_alphanumeric()).collect();
    if let Some(found) = MODES.iter().find(|m| words.contains(m)) {
        return Some(match *found {
            "USB" | "LSB" => "SSB".to_string(),
            other => other.to_string(),
        });
    }
    FT8_KHZ.iter().any(|f| (khz - f).abs() <= 3.0).then(|| "FT8".to_string())
}

fn is_call(token: &str) -> bool {
    (3..=20).contains(&token.len()) && token.chars().all(|c| c.is_ascii_alphanumeric() || c == '/') && token.chars().any(|c| c.is_ascii_digit()) && token.chars().any(|c| c.is_ascii_alphabetic())
}

fn is_time(token: &str) -> bool {
    token.len() == 5 && token.ends_with('Z') && token[..4].chars().all(|c| c.is_ascii_digit())
}

fn is_grid(token: &str) -> bool {
    let b = token.as_bytes();
    matches!(b.len(), 4 | 6) && b[0].is_ascii_alphabetic() && b[1].is_ascii_alphabetic() && b[2].is_ascii_digit() && b[3].is_ascii_digit()
        && (b.len() == 4 || (b[4].is_ascii_alphabetic() && b[5].is_ascii_alphabetic()))
}

/// Legge una riga del cluster; `None` se non e' uno spot (benvenuti, comandi, annunci...).
pub fn parse_spot(line: &str) -> Option<Parsed> {
    let line = line.trim();
    let rest = line.get(..6).filter(|p| p.eq_ignore_ascii_case("DX de ")).map(|_| &line[6..])?;
    let (spotter, rest) = rest.split_once(':')?;
    let spotter = spotter.trim();
    if spotter.is_empty() || spotter.len() > 20 || spotter.contains(char::is_whitespace) {
        return None;
    }
    let tokens: Vec<&str> = rest.split_whitespace().collect();
    let freq_khz: f64 = tokens.first()?.parse().ok()?;
    if !(100.0..=30_000_000.0).contains(&freq_khz) {
        return None;
    }
    let dx_call = tokens.get(1)?.to_uppercase();
    if !is_call(&dx_call) {
        return None;
    }
    let tail = &tokens[2..];
    // ora e (se c'e') locatore, in fondo alla riga
    let (comment_tokens, time, grid) = match tail {
        [head @ .., t, g] if is_time(t) && is_grid(g) => (head, *t, Some(g.to_string())),
        [head @ .., t] if is_time(t) => (head, *t, None),
        _ => return None,
    };
    let comment = comment_tokens.join(" ");
    Some(Parsed {
        spotter: spotter.to_string(),
        freq_khz,
        dx_call,
        mode: guess_mode(&comment, freq_khz),
        comment,
        time_utc: time[..4].to_string(),
        grid,
        band: band_of_khz(freq_khz),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_a_dxspider_ft8_spot() {
        let p = parse_spot("DX de W3LPL:     14074.0  JA1ABC       FT8 -10 dB                     1234Z").unwrap();
        assert_eq!((p.spotter.as_str(), p.dx_call.as_str()), ("W3LPL", "JA1ABC"));
        assert_eq!(p.freq_khz, 14074.0);
        assert_eq!(p.band, Some("20m"));
        assert_eq!(p.mode.as_deref(), Some("FT8"));
        assert_eq!(p.time_utc, "1234");
        assert_eq!(p.comment, "FT8 -10 dB");
        assert_eq!(p.grid, None);
    }

    #[test]
    fn reads_the_locator_after_the_time() {
        let p = parse_spot("DX de G4ABC:     14025.0  K1ABC        cw 22 wpm       0856Z IO91").unwrap();
        assert_eq!(p.grid.as_deref(), Some("IO91"));
        assert_eq!(p.mode.as_deref(), Some("CW"));
        assert_eq!(p.time_utc, "0856");
        let six = parse_spot("DX de G4ABC:  7010.5  K1ABC  tnx  0856Z IO91wm").unwrap();
        assert_eq!(six.grid.as_deref(), Some("IO91wm"));
    }

    #[test]
    fn reads_skimmer_spotters_with_a_hash_and_a_ssid() {
        let p = parse_spot("DX de KM3T-#:    14074.0  DL1ABC         FT8  -14 dB  2200Z").unwrap();
        assert_eq!(p.spotter, "KM3T-#");
        assert_eq!(p.mode.as_deref(), Some("FT8"));
        assert_eq!(parse_spot("DX de DK9XX-1:  21074.0  K1ABC  CQ  1200Z").unwrap().spotter, "DK9XX-1");
    }

    #[test]
    fn the_ft8_frequency_is_enough_when_the_comment_says_nothing() {
        assert_eq!(parse_spot("DX de AA1AA: 21074.2 K1ABC strong 1200Z").unwrap().mode.as_deref(), Some("FT8"));
        assert_eq!(parse_spot("DX de AA1AA: 21290.0 K1ABC strong 1200Z").unwrap().mode, None);
    }

    #[test]
    fn ssb_sidebands_are_just_ssb() {
        assert_eq!(parse_spot("DX de AA1AA: 14200.0 K1ABC usb 59 1200Z").unwrap().mode.as_deref(), Some("SSB"));
    }

    #[test]
    fn ignores_everything_that_is_not_a_spot() {
        for line in [
            "",
            "Welcome to the cluster",
            "To ALL de W3LPL: hello 1200Z",
            "DX de W3LPL: abc K1ABC cw 1200Z",
            "DX de W3LPL: 14074.0 ??? cw 1200Z",
            "DX de W3LPL: 14074.0 K1ABC cw no time",
            "DX de : 14074.0 K1ABC cw 1200Z",
            "DX de W3LPL: 99.0 K1ABC cw 1200Z",
        ] {
            assert!(parse_spot(line).is_none(), "{line:?}");
        }
    }

    #[test]
    fn bands_follow_the_amateur_allocations() {
        assert_eq!(band_of_khz(7074.0), Some("40m"));
        assert_eq!(band_of_khz(28074.0), Some("10m"));
        assert_eq!(band_of_khz(50313.0), Some("6m"));
        assert_eq!(band_of_khz(15000.0), None);
    }
}
