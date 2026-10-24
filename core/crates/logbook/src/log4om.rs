//! Lettura (solo lettura) del log di Log4OM2: database SQLite, tabella `Log`.

use std::path::Path;

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
}
