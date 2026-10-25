//! Log dei QSO su SQLite.

pub mod adif;
pub mod log4om;

use std::{
    path::Path,
    sync::Mutex,
    time::{SystemTime, UNIX_EPOCH},
};

use aurora_core::{
    adif::{parse_records, unix_ms, AdifQso},
    QsoRecord,
};
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

/// Un QSO e' lo stesso se la stazione, la banda, il modo e il giorno coincidono e l'inizio differisce di meno di due minuti.
const DUPLICATE_WINDOW_SECS: u64 = 120;

/// Secondi dalla mezzanotte da un orario `HHMMSS` (valori mancanti o non validi valgono 0).
fn seconds_of_day(t: &str) -> u64 {
    let n = |i: usize| t.get(i..i + 2).and_then(|v| v.parse::<u64>().ok()).unwrap_or(0);
    n(0) * 3600 + n(2) * 60 + n(4)
}

/// Esito di un'importazione ADIF.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, serde::Serialize)]
pub struct ImportReport {
    /// Record trovati nel file.
    pub read: usize,
    /// QSO aggiunti al log.
    pub added: usize,
    /// QSO gia' presenti (o ripetuti nel file): non aggiunti.
    pub duplicates: usize,
    /// Record scartati: senza nominativo, banda o data valida.
    pub skipped: usize,
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
        if version < 2 {
            // seconda versione: ora di inizio e di fine come nell'ADIF e la conferma (vuota = il log non lo dice), per l'esportazione
            // completa e per non registrare due volte lo stesso QSO quando si importa un file
            conn.execute_batch(
                "ALTER TABLE qso ADD COLUMN time_on TEXT NOT NULL DEFAULT '';
                 ALTER TABLE qso ADD COLUMN date_off TEXT NOT NULL DEFAULT '';
                 ALTER TABLE qso ADD COLUMN time_off TEXT NOT NULL DEFAULT '';
                 ALTER TABLE qso ADD COLUMN confirmed INTEGER;
                 UPDATE qso SET time_on = printf('%02d%02d%02d', (time_ms / 1000) / 3600 % 24, (time_ms / 1000) / 60 % 60, (time_ms / 1000) % 60),
                                date_off = date;
                 UPDATE qso SET time_off = time_on;
                 CREATE INDEX qso_date ON qso (date);
                 PRAGMA user_version = 2;",
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
        // la data e' quella dell'inizio del QSO (JTDX: `QSO_DATE`), se nota
        let date = if q.time_on_unix_ms > 0 { adif::utc_date(q.time_on_unix_ms / 1000) } else { adif::utc_date(now) };
        let (_, time_on, date_off, time_off) = adif::qso_times(q, &date);
        let conn = self.conn()?;
        conn.execute(
            "INSERT INTO qso (call, grid, band, mode, freq_hz, rst_sent, rst_rcvd, time_ms, date, time_on, date_off, time_off)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
            params![
                q.call.to_uppercase(),
                q.grid,
                q.band,
                q.mode,
                q.freq_hz as i64,
                q.rst_sent,
                q.rst_rcvd,
                q.time_ms as i64,
                date,
                time_on,
                date_off,
                time_off
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

    /// Tutti i QSO del log, dal piu' vecchio inserito (per la scheda DXCC e per i gia' lavorati).
    pub fn all_qsos(&self) -> Result<Vec<AdifQso>, LogError> {
        let conn = self.conn()?;
        let mut stmt = conn.prepare(
            "SELECT call, grid, band, mode, freq_hz, rst_sent, rst_rcvd, date, time_on, date_off, time_off, confirmed FROM qso ORDER BY id",
        )?;
        let rows = stmt.query_map([], |r| {
            Ok(AdifQso {
                call: r.get(0)?,
                grid: r.get(1)?,
                band: r.get::<_, String>(2)?.to_lowercase(),
                mode: r.get::<_, String>(3)?.to_uppercase(),
                freq_hz: r.get::<_, i64>(4)?.max(0) as u64,
                rst_sent: r.get(5)?,
                rst_rcvd: r.get(6)?,
                date: r.get(7)?,
                time_on: r.get(8)?,
                date_off: r.get(9)?,
                time_off: r.get(10)?,
                confirmed: r.get::<_, Option<i64>>(11)?.map(|v| v != 0),
                // il log interno e' sempre del proprio nominativo: non conserva con quale
                station_call: String::new(),
                my_grid: String::new(),
            })
        })?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    /// Il log come file ADIF di JTDX (`wsjtx_log.adi`: stesse righe di `jtdx_file_record`, CR LF), con i QSO tra due date (`AAAAMMGG`, estremi
    /// compresi; `None` = senza limite). Ritorna il testo e quanti QSO contiene.
    pub fn export_jtdx_adif(&self, station: &adif::Station, from: Option<&str>, to: Option<&str>) -> Result<(String, usize), LogError> {
        let conn = self.conn()?;
        let mut stmt = conn.prepare(
            "SELECT call, grid, band, mode, freq_hz, rst_sent, rst_rcvd, date, time_on, date_off, time_off FROM qso
             WHERE (?1 = '' OR date >= ?1) AND (?2 = '' OR date <= ?2) ORDER BY date, time_on, id",
        )?;
        let rows = stmt.query_map(params![from.unwrap_or(""), to.unwrap_or("")], |r| {
            let (date, time_on, date_off, time_off): (String, String, String, String) = (r.get(7)?, r.get(8)?, r.get(9)?, r.get(10)?);
            let on = unix_ms(&date, &time_on).unwrap_or(0);
            let off = unix_ms(if date_off.is_empty() { &date } else { &date_off }, if time_off.is_empty() { &time_on } else { &time_off }).unwrap_or(on);
            Ok((
                QsoRecord {
                    call: r.get(0)?,
                    grid: r.get(1)?,
                    band: r.get(2)?,
                    mode: r.get(3)?,
                    freq_hz: r.get::<_, i64>(4)?.max(0) as u64,
                    rst_sent: r.get(5)?,
                    rst_rcvd: r.get(6)?,
                    time_ms: (on / 1000 % 86_400 * 1000) as u32,
                    time_on_unix_ms: on,
                    time_off_unix_ms: off,
                    ..Default::default()
                },
                date,
            ))
        })?;
        let mut out = String::from(adif::JTDX_FILE_HEADER);
        let mut count = 0;
        for row in rows {
            let (q, date) = row?;
            out.push_str(&adif::jtdx_file_record(&q, station, &date));
            count += 1;
        }
        Ok((out, count))
    }

    /// Aggiunge al log i QSO di un testo ADIF (per esempio il `wsjtx_log.adi` di JTDX). Un QSO gia' presente (stessa stazione, banda e modo,
    /// stesso giorno, inizio entro due minuti) non si aggiunge, neppure se e' ripetuto nel file; un record senza nominativo, banda o data valida
    /// si scarta. Tutto o niente: se la scrittura fallisce il log resta com'era.
    pub fn import_adif(&self, text: &str) -> Result<ImportReport, LogError> {
        let records = parse_records(text);
        let mut report = ImportReport { read: records.len(), ..ImportReport::default() };
        let mut conn = self.conn()?;
        let tx = conn.transaction()?;
        {
            let mut find = tx.prepare("SELECT time_on FROM qso WHERE call = ?1 AND band = ?2 AND mode = ?3 AND date = ?4")?;
            let mut insert = tx.prepare(
                "INSERT INTO qso (call, grid, band, mode, freq_hz, rst_sent, rst_rcvd, time_ms, date, time_on, date_off, time_off, confirmed)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
            )?;
            for record in &records {
                let Some(q) = AdifQso::from_fields(record) else {
                    report.skipped += 1;
                    continue;
                };
                if unix_ms(&q.date, &q.time_on).is_none() {
                    report.skipped += 1;
                    continue;
                }
                let time_on = if q.time_on.is_empty() { "000000".to_string() } else { q.time_on.clone() };
                let at = seconds_of_day(&time_on);
                let known: Vec<String> = find.query_map(params![q.call, q.band, q.mode, q.date], |r| r.get(0))?.collect::<Result<_, _>>()?;
                if known.iter().any(|t| seconds_of_day(t).abs_diff(at) <= DUPLICATE_WINDOW_SECS) {
                    report.duplicates += 1;
                    continue;
                }
                let (date_off, time_off) = if q.time_off.is_empty() {
                    (q.date.clone(), time_on.clone())
                } else {
                    (if q.date_off.is_empty() { q.date.clone() } else { q.date_off.clone() }, q.time_off.clone())
                };
                insert.execute(params![
                    q.call,
                    q.grid,
                    q.band,
                    q.mode,
                    q.freq_hz as i64,
                    q.rst_sent,
                    q.rst_rcvd,
                    (at * 1000) as i64,
                    q.date,
                    time_on,
                    date_off,
                    time_off,
                    q.confirmed.map(i64::from)
                ])?;
                report.added += 1;
            }
        }
        tx.commit()?;
        Ok(report)
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
                    time_on_unix_ms: 0,
                    time_off_unix_ms: 0,
                    ..Default::default()
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
