//! Log dei QSO su SQLite.

pub mod adif;

use std::{
    path::Path,
    sync::Mutex,
    time::{SystemTime, UNIX_EPOCH},
};

use aurora_core::QsoRecord;
use rusqlite::{params, Connection};

#[derive(Debug)]
pub enum LogError {
    Storage(String),
}

impl std::fmt::Display for LogError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LogError::Storage(e) => write!(f, "errore del log: {e}"),
        }
    }
}

impl std::error::Error for LogError {}

impl From<rusqlite::Error> for LogError {
    fn from(e: rusqlite::Error) -> Self {
        LogError::Storage(e.to_string())
    }
}

pub struct Logbook {
    conn: Mutex<Connection>,
}

impl Logbook {
    pub fn open(path: &Path) -> Result<Logbook, LogError> {
        Self::from_connection(Connection::open(path)?)
    }

    pub fn open_in_memory() -> Result<Logbook, LogError> {
        Self::from_connection(Connection::open_in_memory()?)
    }

    fn from_connection(conn: Connection) -> Result<Logbook, LogError> {
        let version: i64 = conn.query_row("PRAGMA user_version", [], |r| r.get(0))?;
        if version < 1 {
            conn.execute_batch(
                "CREATE TABLE qso (
                    id INTEGER PRIMARY KEY,
                    call TEXT NOT NULL,
                    grid TEXT NOT NULL,
                    band TEXT NOT NULL,
                    mode TEXT NOT NULL,
                    freq_hz INTEGER NOT NULL,
                    rst_sent TEXT NOT NULL,
                    rst_rcvd TEXT NOT NULL,
                    time_ms INTEGER NOT NULL,
                    date TEXT NOT NULL
                );
                CREATE INDEX qso_call_band_mode ON qso (call, band, mode);
                PRAGMA user_version = 1;",
            )?;
        }
        Ok(Logbook { conn: Mutex::new(conn) })
    }

    fn conn(&self) -> Result<std::sync::MutexGuard<'_, Connection>, LogError> {
        self.conn.lock().map_err(|_| LogError::Storage("lucchetto avvelenato".into()))
    }

    pub fn schema_version(&self) -> Result<i64, LogError> {
        Ok(self.conn()?.query_row("PRAGMA user_version", [], |r| r.get(0))?)
    }

    pub fn add(&self, q: &QsoRecord) -> Result<i64, LogError> {
        let now = SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |d| d.as_secs());
        let conn = self.conn()?;
        conn.execute(
            "INSERT INTO qso (call, grid, band, mode, freq_hz, rst_sent, rst_rcvd, time_ms, date)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                q.call.to_uppercase(),
                q.grid,
                q.band,
                q.mode,
                q.freq_hz as i64,
                q.rst_sent,
                q.rst_rcvd,
                q.time_ms as i64,
                adif::utc_date(now)
            ],
        )?;
        Ok(conn.last_insert_rowid())
    }

    pub fn is_dupe(&self, call: &str, band: &str, mode: &str) -> Result<bool, LogError> {
        let found: i64 = self.conn()?.query_row(
            "SELECT COUNT(*) FROM qso WHERE call = ?1 AND band = ?2 AND mode = ?3",
            params![call.to_uppercase(), band, mode],
            |r| r.get(0),
        )?;
        Ok(found > 0)
    }

    pub fn count(&self) -> Result<u64, LogError> {
        let n: i64 = self.conn()?.query_row("SELECT COUNT(*) FROM qso", [], |r| r.get(0))?;
        Ok(n as u64)
    }

    pub fn export_adif(&self) -> Result<String, LogError> {
        let conn = self.conn()?;
        let mut stmt = conn.prepare(
            "SELECT call, grid, band, mode, freq_hz, rst_sent, rst_rcvd, time_ms, date FROM qso ORDER BY id",
        )?;
        let rows = stmt.query_map([], |r| {
            Ok((
                QsoRecord {
                    call: r.get(0)?,
                    grid: r.get(1)?,
                    band: r.get(2)?,
                    mode: r.get(3)?,
                    freq_hz: r.get::<_, i64>(4)? as u64,
                    rst_sent: r.get(5)?,
                    rst_rcvd: r.get(6)?,
                    time_ms: r.get::<_, i64>(7)? as u32,
                },
                r.get::<_, String>(8)?,
            ))
        })?;
        let mut out = adif::header();
        for row in rows {
            let (q, date) = row?;
            out.push_str(&adif::record(&q, &date));
        }
        Ok(out)
    }
}
