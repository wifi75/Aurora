//! I dati delle orbite (TLE): lettura del formato a due o tre righe, archivio nella cartella dei dati e scarico da CelesTrak.

use std::path::{Path, PathBuf};
use std::time::Duration;

/// Un TLE: nome e le due righe.
#[derive(Clone, Debug, PartialEq)]
pub struct TleSet {
    pub name: String,
    pub line1: String,
    pub line2: String,
}

/// I gruppi di CelesTrak da scaricare: i satelliti radioamatoriali e le stazioni (ISS).
pub const SOURCES: [&str; 2] = [
    "https://celestrak.org/NORAD/elements/gp.php?GROUP=amateur&FORMAT=tle",
    "https://celestrak.org/NORAD/elements/gp.php?GROUP=stations&FORMAT=tle",
];

/// Il numero NORAD dalla riga 1 (colonne 3-7).
pub fn norad_of(line1: &str) -> Option<u64> {
    line1.get(2..7)?.trim().parse().ok()
}

/// Legge un testo con TLE a tre righe (nome, riga 1, riga 2) o a due righe (senza nome). Le righe che non sono TLE si saltano.
pub fn parse(text: &str) -> Vec<TleSet> {
    let lines: Vec<&str> = text.lines().map(str::trim_end).filter(|l| !l.trim().is_empty()).collect();
    let mut out = Vec::new();
    let mut i = 0;
    while i < lines.len() {
        let l = lines[i];
        if l.starts_with("1 ") && l.len() >= 68 && i + 1 < lines.len() && lines[i + 1].starts_with("2 ") {
            // due righe senza nome: il nome e' il numero NORAD
            let name = norad_of(l).map_or_else(|| "?".to_string(), |n| n.to_string());
            out.push(TleSet { name, line1: l.to_string(), line2: lines[i + 1].to_string() });
            i += 2;
        } else if i + 2 < lines.len() && lines[i + 1].starts_with("1 ") && lines[i + 2].starts_with("2 ") {
            out.push(TleSet { name: l.trim().trim_start_matches("0 ").to_string(), line1: lines[i + 1].to_string(), line2: lines[i + 2].to_string() });
            i += 3;
        } else {
            i += 1;
        }
    }
    out
}

/// Scarica un gruppo da CelesTrak. Bloccante.
pub fn fetch(url: &str) -> Result<String, String> {
    let agent: ureq::Agent = ureq::Agent::config_builder().timeout_global(Some(Duration::from_secs(20))).http_status_as_error(false).build().into();
    let mut response = agent.get(url).header("User-Agent", "Aurora (satelliti)").call().map_err(|e| e.to_string())?;
    let status = response.status().as_u16();
    if status != 200 {
        return Err(format!("CelesTrak ha risposto {status}"));
    }
    let body = response.body_mut().read_to_string().map_err(|e| e.to_string())?;
    if parse(&body).is_empty() {
        return Err("la risposta di CelesTrak non contiene TLE".into());
    }
    Ok(body)
}

/// L'archivio dei TLE: `tle.txt` e il momento dello scarico in `tle.stamp`.
pub struct TleFile {
    text: PathBuf,
    stamp: PathBuf,
}

impl TleFile {
    pub fn new(data_dir: &Path) -> TleFile {
        TleFile { text: data_dir.join("tle.txt"), stamp: data_dir.join("tle.stamp") }
    }

    pub fn load(&self) -> Vec<TleSet> {
        std::fs::read_to_string(&self.text).map(|t| parse(&t)).unwrap_or_default()
    }

    /// Quando sono stati scaricati (secondi Unix), se si sa.
    pub fn fetched_unix(&self) -> Option<f64> {
        std::fs::read_to_string(&self.stamp).ok().and_then(|s| s.trim().parse().ok())
    }

    pub fn save(&self, text: &str, now_unix: f64) -> Result<(), String> {
        std::fs::write(&self.text, text).map_err(|e| e.to_string())?;
        std::fs::write(&self.stamp, format!("{now_unix}")).map_err(|e| e.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const THREE: &str = "ISS (ZARYA)\r\n1 25544U 98067A   08264.51782528 -.00002182  00000-0 -11606-4 0  2927\r\n2 25544  51.6416 247.4627 0006703 130.5360 325.0288 15.72125391563537\r\nRS-44 (DOSAAF-85)\r\n1 44909U 19096E   24300.50000000  .00000100  00000-0  10000-4 0  9990\r\n2 44909  82.5200 100.0000 0200000 100.0000 260.0000 12.79000000100000\r\n";

    #[test]
    fn legge_tle_a_tre_righe() {
        let sets = parse(THREE);
        assert_eq!(sets.len(), 2);
        assert_eq!(sets[0].name, "ISS (ZARYA)");
        assert_eq!(norad_of(&sets[0].line1), Some(25544));
        assert_eq!(sets[1].name, "RS-44 (DOSAAF-85)");
        assert_eq!(norad_of(&sets[1].line1), Some(44909));
    }

    #[test]
    fn legge_tle_a_due_righe_e_salta_le_righe_estranee() {
        let two = "1 25544U 98067A   08264.51782528 -.00002182  00000-0 -11606-4 0  2927\n2 25544  51.6416 247.4627 0006703 130.5360 325.0288 15.72125391563537\nfine\n";
        let sets = parse(two);
        assert_eq!(sets.len(), 1);
        assert_eq!(sets[0].name, "25544");
    }

    #[test]
    fn un_testo_senza_tle_da_un_elenco_vuoto() {
        assert!(parse("").is_empty());
        assert!(parse("<html>errore</html>").is_empty());
    }

    #[test]
    fn l_archivio_si_scrive_e_si_rilegge() {
        let dir = std::env::temp_dir().join("aurora-prova-tle");
        std::fs::create_dir_all(&dir).expect("cartella");
        let f = TleFile::new(&dir);
        assert!(f.load().is_empty());
        f.save(THREE, 1_700_000_000.0).expect("scrittura");
        assert_eq!(f.load().len(), 2);
        assert_eq!(f.fetched_unix(), Some(1_700_000_000.0));
        std::fs::remove_dir_all(&dir).ok();
    }
}
