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

/// Frequenze di lavoro (kHz, frequenza della radio) dei modi digitali, con la distanza massima: se lo spot e' li' vicino ed e' senza
/// altro indizio si dice quel modo. FT8 e FT4 come le frequenze di fabbrica di WSJT-X; JS8, PSK31 e WSPR quelle d'uso comune.
const DIGITAL_KHZ: &[(&str, f64, &[f64])] = &[
    ("FT8", 3.0, &[1840.0, 3573.0, 5357.0, 7074.0, 10136.0, 14074.0, 18100.0, 21074.0, 24915.0, 28074.0, 50313.0, 144174.0]),
    ("FT4", 2.0, &[3575.5, 7047.5, 10140.0, 14080.0, 18104.0, 21140.0, 24919.0, 28180.0, 50318.0, 144170.0]),
    ("JS8", 1.5, &[3578.0, 7078.0, 10130.0, 14078.0, 21078.0, 24922.0, 28078.0]),
    ("WSPR", 0.3, &[1836.6, 3568.6, 5364.7, 7038.6, 10138.7, 14095.6, 18104.6, 21094.6, 24924.6, 28124.6, 50293.0]),
    ("PSK31", 1.5, &[3580.0, 7040.0, 14070.0, 21070.0, 28120.0]),
];

/// Il band plan IARU Regione 1 (come il band plan italiano): i tratti dove c'e' un solo uso. Fuori da questi (digitali misti, beacon,
/// satelliti, «tutti i modi») il modo resta vuoto: meglio niente che un modo sbagliato.
const PLAN_KHZ: &[(f64, f64, &str)] = &[
    (1810.0, 1838.0, "CW"),
    (1842.0, 2000.0, "SSB"),
    (3500.0, 3570.0, "CW"),
    (3620.0, 3800.0, "SSB"),
    (7000.0, 7040.0, "CW"),
    (7050.0, 7200.0, "SSB"),
    (10100.0, 10130.0, "CW"),
    (14000.0, 14070.0, "CW"),
    (14112.0, 14350.0, "SSB"),
    (18068.0, 18095.0, "CW"),
    (18111.0, 18168.0, "SSB"),
    (21000.0, 21070.0, "CW"),
    (21151.0, 21450.0, "SSB"),
    (24890.0, 24915.0, "CW"),
    (24931.0, 24990.0, "SSB"),
    (28000.0, 28050.0, "CW"),
    (28225.0, 29200.0, "SSB"),
    (29520.0, 29700.0, "FM"),
    (50000.0, 50100.0, "CW"),
    (50100.0, 50300.0, "SSB"),
    (144000.0, 144110.0, "CW"),
    (144150.0, 144165.0, "SSB"),
    (144180.0, 144400.0, "SSB"),
];

pub fn band_of_khz(khz: f64) -> Option<&'static str> {
    BANDS.iter().find(|(_, lo, hi)| khz >= *lo && khz <= *hi).map(|(name, _, _)| *name)
}

const MODES: &[&str] = &[
    "FT8", "FT4", "MSK144", "JT65", "JT9", "JT4", "Q65", "FST4", "FST4W", "WSPR", "JS8", "JS8CALL", "RTTY", "PSK31", "PSK63", "PSK", "BPSK31", "OLIVIA",
    "MFSK", "CONTESTIA", "HELL", "VARA", "VARAC", "PACTOR", "ARDOP", "FREEDV", "CW", "SSB", "USB", "LSB", "FM", "AM", "SSTV",
];

/// Il modo dal commento (`FT8 -10 dB`, `CW 22 wpm`, `VarAC VARA`...); mancando, dalla frequenza di lavoro di un modo digitale (FT8, FT4,
/// JS8, WSPR, PSK31) e poi dal band plan (tratto solo CW o solo fonia). Un commento che parla di digitale senza dire quale non prende il
/// modo dal band plan (`VarAC ... Digi` a 7105 non e' SSB).
pub fn guess_mode(comment: &str, khz: f64) -> Option<String> {
    let upper = comment.to_uppercase();
    let words: Vec<&str> = upper.split(|c: char| !c.is_ascii_alphanumeric()).collect();
    if let Some(found) = MODES.iter().find(|m| words.contains(m)) {
        return Some(match *found {
            "USB" | "LSB" => "SSB",
            "JS8CALL" => "JS8",
            "VARAC" => "VARA",
            "BPSK31" => "PSK31",
            other => other,
        }
        .to_string());
    }
    let nearest = DIGITAL_KHZ
        .iter()
        .flat_map(|(mode, near, list)| list.iter().map(move |f| (*mode, (khz - f).abs(), *near)))
        .filter(|(_, distance, near)| distance <= near)
        .min_by(|a, b| a.1.total_cmp(&b.1));
    if let Some((mode, _, _)) = nearest {
        return Some(mode.to_string());
    }
    if words.iter().any(|w| matches!(*w, "DIGI" | "DIGITAL" | "DATA" | "RTTY" | "BEACON" | "BCN" | "NCDXF")) {
        return None;
    }
    PLAN_KHZ.iter().find(|(lo, hi, _)| khz >= *lo && khz < *hi).map(|(_, _, mode)| (*mode).to_string())
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
    fn reads_rbn_spotters_with_a_hash_and_a_ssid() {
        let p = parse_spot("DX de KM3T-#:    14074.0  DL1ABC         FT8  -14 dB  2200Z").unwrap();
        assert_eq!(p.spotter, "KM3T-#");
        assert_eq!(p.mode.as_deref(), Some("FT8"));
        assert_eq!(parse_spot("DX de DK9XX-1:  21074.0  K1ABC  CQ  1200Z").unwrap().spotter, "DK9XX-1");
    }

    #[test]
    fn the_ft8_frequency_is_enough_when_the_comment_says_nothing() {
        assert_eq!(parse_spot("DX de AA1AA: 21074.2 K1ABC strong 1200Z").unwrap().mode.as_deref(), Some("FT8"));
        assert_eq!(parse_spot("DX de AA1AA: 21140.0 K1ABC strong 1200Z").unwrap().mode.as_deref(), Some("FT4"));
        assert_eq!(parse_spot("DX de AA1AA: 14095.6 K1ABC strong 1200Z").unwrap().mode.as_deref(), Some("WSPR"));
        // la frequenza di lavoro piu' vicina vince anche dove le finestre si sovrappongono
        assert_eq!(parse_spot("DX de AA1AA: 14078.5 K1ABC strong 1200Z").unwrap().mode.as_deref(), Some("JS8"));
        assert_eq!(parse_spot("DX de AA1AA: 10138.7 K1ABC strong 1200Z").unwrap().mode.as_deref(), Some("WSPR"));
        assert_eq!(parse_spot("DX de AA1AA: 18104.6 K1ABC strong 1200Z").unwrap().mode.as_deref(), Some("WSPR"));
    }

    #[test]
    fn the_band_plan_gives_cw_or_ssb_when_the_comment_says_nothing() {
        let mode = |line: &str| parse_spot(line).and_then(|p| p.mode);
        assert_eq!(mode("DX de AA1AA: 7025.0 F4HHL strong 1200Z").as_deref(), Some("CW"));
        assert_eq!(mode("DX de AA1AA: 21041.0 N5V 1200Z").as_deref(), Some("CW"));
        assert_eq!(mode("DX de AA1AA: 14282.0 3X4U QSX 14292 SC 1200Z").as_deref(), Some("SSB"));
        assert_eq!(mode("DX de AA1AA: 28465.0 PY2TBK 1200Z").as_deref(), Some("SSB"));
        assert_eq!(mode("DX de AA1AA: 7056.0 C8K 1200Z").as_deref(), Some("SSB"));
        assert_eq!(mode("DX de AA1AA: 21290.0 K1ABC strong 1200Z").as_deref(), Some("SSB"));
        // tratti misti (digitali e CW) e beacon: nessun modo
        assert_eq!(mode("DX de AA1AA: 14085.0 K1ABC 1200Z"), None);
        assert_eq!(mode("DX de AA1AA: 28200.0 K1ABC 1200Z"), None);
        // il commento vince sul band plan, e un «Digi» senza modo non diventa SSB
        assert_eq!(mode("DX de AA1AA: 7105.0 VE2EH (VarAC VARA Digi Freq) 1200Z").as_deref(), Some("VARA"));
        assert_eq!(mode("DX de AA1AA: 7105.0 VE2EH Digi mode 1200Z"), None);
        assert_eq!(mode("DX de AA1AA: 14230.0 K1ABC SSTV 1200Z").as_deref(), Some("SSTV"));
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
