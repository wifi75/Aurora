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
