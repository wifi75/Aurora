//! Lettura (solo lettura) del log di Log4OM2: database SQLite, tabella `Log`.

use std::path::Path;

use aurora_core::adif::AdifQso;
use rusqlite::{Connection, OpenFlags};

use crate::LogError;

/// Un QSO del log di Log4OM2, con i soli campi che Aurora mostra.
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct Log4omQso {
    pub date: String,
    pub call: String,
    pub band: String,
    pub mode: String,
    pub rst_sent: String,
    pub rst_rcvd: String,
    pub country: String,
    pub grid: String,
}

/// Totale dei QSO e gli ultimi `limit`, dal piu' recente.
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct Log4omView {
    pub total: u64,
    pub qsos: Vec<Log4omQso>,
}

/// Apre il file senza poterlo modificare: Log4OM2 puo' essere acceso e usarlo nello stesso momento.
pub fn read_recent(path: &Path, limit: usize) -> Result<Log4omView, LogError> {
    let conn = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX)?;
    conn.busy_timeout(std::time::Duration::from_secs(2))?;
    let total: i64 = conn.query_row("SELECT COUNT(*) FROM Log", [], |r| r.get(0))?;
    let mut stmt = conn.prepare(
        "SELECT COALESCE(qsodate, ''), COALESCE(callsign, ''), COALESCE(band, ''), COALESCE(mode, ''),
                COALESCE(rstsent, ''), COALESCE(rstrcvd, ''), COALESCE(country, ''), COALESCE(gridsquare, '')
         FROM Log ORDER BY qsodate DESC LIMIT ?1",
    )?;
    let qsos = stmt
        .query_map([limit as i64], |r| {
            Ok(Log4omQso {
                date: r.get(0)?,
                call: r.get(1)?,
                band: r.get(2)?,
                mode: r.get(3)?,
                rst_sent: r.get(4)?,
                rst_rcvd: r.get(5)?,
                country: r.get(6)?,
                grid: r.get(7)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(Log4omView { total: total.max(0) as u64, qsos })
}

/// Un quadrato locatore (4 caratteri) con almeno un QSO: quanti e su quali bande.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct WorkedSquare {
    pub grid: String,
    pub count: u64,
    pub bands: Vec<String>,
}

/// I quadrati locatore gia' lavorati, per la mappa: raggruppati per quadrato e per banda direttamente nel database (decine di migliaia
/// di QSO diventano qualche migliaio di righe). I QSO senza locatore valido non contano.
pub fn worked_squares(path: &Path) -> Result<Vec<WorkedSquare>, LogError> {
    let conn = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX)?;
    conn.busy_timeout(std::time::Duration::from_secs(2))?;
    let mut stmt = conn.prepare(
        "SELECT UPPER(SUBSTR(gridsquare, 1, 4)) AS g, COALESCE(band, ''), COUNT(*)
         FROM Log WHERE LENGTH(gridsquare) >= 4 GROUP BY g, band ORDER BY g",
    )?;
    let mut rows = stmt.query([])?;
    let mut squares: Vec<WorkedSquare> = Vec::new();
    while let Some(r) = rows.next()? {
        let (grid, band, count): (String, String, i64) = (r.get(0)?, r.get(1)?, r.get(2)?);
        if !is_square(&grid) {
            continue;
        }
        let count = count.max(0) as u64;
        match squares.last_mut() {
            Some(last) if last.grid == grid => {
                last.count += count;
                if !band.is_empty() && !last.bands.contains(&band) {
                    last.bands.push(band);
                }
            }
            _ => squares.push(WorkedSquare { grid, count, bands: if band.is_empty() { Vec::new() } else { vec![band] } }),
        }
    }
    Ok(squares)
}

/// Le colonne di Log4OM2 che dicono se il QSO e' confermato (QSL cartacea, LoTW, eQSL): quelle che il file ha davvero.
const CONFIRMATION_COLUMNS: [&str; 6] = ["qslrcvd", "qslreceived", "lotwqslrcvd", "lotwreceived", "eqslqslrcvd", "eqslreceived"];

/// Tutti i QSO del log di Log4OM2 (sola lettura) con banda, modo, locatore, data e, se il file ha le colonne delle conferme, lo stato delle
/// conferme. Il modo e' quello di `mode`, o di `submode` quando `mode` e' `MFSK`.
pub fn read_all(path: &Path) -> Result<Vec<AdifQso>, LogError> {
    let conn = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX)?;
    conn.busy_timeout(std::time::Duration::from_secs(2))?;
    let columns: Vec<String> = {
        let mut stmt = conn.prepare("PRAGMA table_info(Log)")?;
        let names = stmt.query_map([], |r| r.get::<_, String>(1))?.collect::<Result<Vec<_>, _>>()?;
        names.into_iter().map(|n| n.to_lowercase()).collect()
    };
    let has = |c: &str| columns.iter().any(|x| x == c);
    let submode = if has("submode") { "COALESCE(submode, '')" } else { "''" };
    let confirmations: Vec<&str> = CONFIRMATION_COLUMNS.iter().copied().filter(|c| has(c)).collect();
    let confirmed_cols: String = confirmations.iter().map(|c| format!(", COALESCE({c}, '')")).collect();
    let sql = format!(
        "SELECT COALESCE(callsign, ''), COALESCE(band, ''), COALESCE(mode, ''), {submode}, COALESCE(gridsquare, ''), COALESCE(qsodate, ''),
                COALESCE(rstsent, ''), COALESCE(rstrcvd, ''){confirmed_cols} FROM Log"
    );
    let mut stmt = conn.prepare(&sql)?;
    let mut rows = stmt.query([])?;
    let mut out = Vec::new();
    while let Some(r) = rows.next()? {
        let call: String = r.get(0)?;
        let band: String = r.get(1)?;
        if call.trim().is_empty() || band.trim().is_empty() {
            continue;
        }
        let (mode, submode): (String, String) = (r.get(2)?, r.get(3)?);
        let when: String = r.get(5)?;
        let digits: String = when.chars().filter(char::is_ascii_digit).collect();
        let mut yes = false;
        for i in 0..confirmations.len() {
            let v: String = r.get(8 + i)?;
            yes |= matches!(v.trim().to_uppercase().as_str(), "Y" | "V" | "YES" | "TRUE" | "1");
        }
        let mode = mode.trim().to_uppercase();
        let mode = if mode == "MFSK" && !submode.trim().is_empty() { submode.trim().to_uppercase() } else { mode };
        out.push(AdifQso {
            call: call.trim().to_uppercase(),
            band: band.trim().to_lowercase(),
            mode,
            grid: r.get::<_, String>(4)?.trim().to_string(),
            date: digits.chars().take(8).collect(),
            time_on: if digits.len() >= 14 { digits[8..14].to_string() } else { String::new() },
            rst_sent: r.get(6)?,
            rst_rcvd: r.get(7)?,
            confirmed: (!confirmations.is_empty()).then_some(yes),
            ..AdifQso::default()
        });
    }
    Ok(out)
}

/// Due lettere da A a R e due cifre: «JN45».
fn is_square(g: &str) -> bool {
    let b = g.as_bytes();
    b.len() == 4 && b[0].is_ascii_uppercase() && (b'A'..=b'R').contains(&b[0]) && (b'A'..=b'R').contains(&b[1]) && b[2].is_ascii_digit() && b[3].is_ascii_digit()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample(dir: &Path) -> std::path::PathBuf {
        let path = dir.join("log4om-prova.sqlite");
        let conn = Connection::open(&path).unwrap();
        conn.execute_batch(
            "CREATE TABLE Log (qsoid INTEGER PRIMARY KEY, callsign TEXT, band TEXT, mode TEXT, qsodate TEXT,
                               rstsent TEXT, rstrcvd TEXT, country TEXT, gridsquare TEXT);
             INSERT INTO Log VALUES (1, 'AA1AA', '20m', 'FT8', '2026-10-24 10:00:00Z', '-10', '-12', 'Italy', 'JN45');
             INSERT INTO Log VALUES (2, 'BB2BB', '15m', 'FT4', '2026-10-24 11:00:00Z', '-05', NULL, 'Japan', NULL);
             INSERT INTO Log VALUES (3, 'CC3CC', '40m', 'FT8', '2026-10-23 09:00:00Z', '-01', '-02', 'Spain', 'IN80');",
        )
        .unwrap();
        path
    }

    #[test]
    fn returns_total_and_the_latest_first() {
        let dir = std::env::temp_dir().join(format!("aurora-log4om-{}", std::process::id()));
        std::fs::remove_dir_all(&dir).ok();
        std::fs::create_dir_all(&dir).unwrap();
        let view = read_recent(&sample(&dir), 2).unwrap();
        assert_eq!(view.total, 3);
        assert_eq!(view.qsos.iter().map(|q| q.call.as_str()).collect::<Vec<_>>(), ["BB2BB", "AA1AA"]);
        assert_eq!(view.qsos[0].rst_rcvd, "", "i campi vuoti diventano testo vuoto");
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn worked_squares_group_by_square_with_the_bands_and_skip_bad_locators() {
        let dir = std::env::temp_dir().join(format!("aurora-log4om-map-{}", std::process::id()));
        std::fs::remove_dir_all(&dir).ok();
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("map.sqlite");
        let conn = Connection::open(&path).unwrap();
        conn.execute_batch(
            "CREATE TABLE Log (qsoid INTEGER PRIMARY KEY, callsign TEXT, band TEXT, gridsquare TEXT);
             INSERT INTO Log VALUES (1, 'AA1AA', '20m', 'jn45xx');
             INSERT INTO Log VALUES (2, 'BB2BB', '15m', 'JN45');
             INSERT INTO Log VALUES (3, 'CC3CC', '15m', 'JN45AB');
             INSERT INTO Log VALUES (4, 'DD4DD', '40m', 'IN80');
             INSERT INTO Log VALUES (5, 'EE5EE', '40m', 'ZZ99');
             INSERT INTO Log VALUES (6, 'FF6FF', '40m', 'JN');
             INSERT INTO Log VALUES (7, 'GG7GG', '40m', NULL);",
        )
        .unwrap();
        drop(conn);
        let squares = worked_squares(&path).unwrap();
        assert_eq!(squares.len(), 2);
        let jn = squares.iter().find(|s| s.grid == "JN45").unwrap();
        assert_eq!(jn.count, 3);
        assert_eq!(jn.bands, ["15m", "20m"]);
        assert_eq!(squares.iter().find(|s| s.grid == "IN80").unwrap().count, 1);
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn cannot_write_to_the_file() {
        let dir = std::env::temp_dir().join(format!("aurora-log4om-w-{}", std::process::id()));
        std::fs::remove_dir_all(&dir).ok();
        std::fs::create_dir_all(&dir).unwrap();
        let path = sample(&dir);
        let ro = Connection::open_with_flags(&path, OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
        assert!(ro.execute("DELETE FROM Log", []).is_err());
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn a_missing_file_is_an_error_not_a_new_empty_log() {
        let path = std::env::temp_dir().join("aurora-non-esiste-12345.sqlite");
        assert!(read_recent(&path, 5).is_err());
        assert!(!path.exists());
    }

    #[test]
    fn read_all_returns_every_qso_with_mode_submode_and_confirmations() {
        let dir = std::env::temp_dir().join(format!("aurora-log4om-all-{}", std::process::id()));
        std::fs::remove_dir_all(&dir).ok();
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("all.sqlite");
        let conn = Connection::open(&path).unwrap();
        conn.execute_batch(
            "CREATE TABLE Log (qsoid INTEGER PRIMARY KEY, callsign TEXT, band TEXT, mode TEXT, submode TEXT, qsodate TEXT, gridsquare TEXT,
                               rstsent TEXT, rstrcvd TEXT, lotwreceived TEXT, qslrcvd TEXT);
             INSERT INTO Log VALUES (1, 'aa1aa', '20M', 'FT8', NULL, '2026-10-24 10:00:00Z', 'jn45', '-10', '-12', 'Y', 'N');
             INSERT INTO Log VALUES (2, 'BB2BB', '15m', 'MFSK', 'FT4', '2026-10-24 11:30:15', NULL, '-05', NULL, 'N', 'N');
             INSERT INTO Log VALUES (3, 'CC3CC', '', 'SSB', NULL, '2026-10-23 09:00:00Z', 'IN80', '59', '59', NULL, NULL);
             INSERT INTO Log VALUES (4, NULL, '40m', 'CW', NULL, NULL, NULL, NULL, NULL, NULL, NULL);",
        )
        .unwrap();
        drop(conn);
        let all = read_all(&path).unwrap();
        assert_eq!(all.len(), 2, "senza banda o senza nominativo non vale");
        assert_eq!((all[0].call.as_str(), all[0].band.as_str(), all[0].mode.as_str(), all[0].grid.as_str()), ("AA1AA", "20m", "FT8", "jn45"));
        assert_eq!((all[0].date.as_str(), all[0].time_on.as_str(), all[0].confirmed), ("20261024", "100000", Some(true)));
        assert_eq!((all[1].mode.as_str(), all[1].time_on.as_str(), all[1].confirmed), ("FT4", "113015", Some(false)));
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn read_all_without_confirmation_columns_says_nothing_about_them() {
        let dir = std::env::temp_dir().join(format!("aurora-log4om-allnc-{}", std::process::id()));
        std::fs::remove_dir_all(&dir).ok();
        std::fs::create_dir_all(&dir).unwrap();
        let path = sample(&dir);
        let all = read_all(&path).unwrap();
        assert_eq!(all.len(), 3);
        assert!(all.iter().all(|q| q.confirmed.is_none()));
        std::fs::remove_dir_all(&dir).ok();
    }
}
