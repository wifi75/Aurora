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
use rusqlite::{OptionalExtension, params, Connection};

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

/// Un QSO del log interno com'e' salvato, con il suo numero: per l'elenco della pagina di esportazione.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct LogEntry {
    pub id: i64,
    /// `AAAAMMGG` (UTC, inizio del QSO).
    pub date: String,
    /// `HHMMSS` (UTC, inizio del QSO).
    pub time: String,
    pub call: String,
    /// In minuscolo, come nell'ADIF (`20m`).
    pub band: String,
    /// In maiuscolo (`FT8`).
    pub mode: String,
    pub freq_hz: u64,
    pub rst_sent: String,
    pub rst_rcvd: String,
    pub grid: String,
}

pub struct Logbook {
    conn: Mutex<Connection>,
}

impl Logbook {
    pub fn open(path: &Path) -> Result<Logbook, LogError> {
        Self::from_connection(Connection::open(path)?)
    }

    /// Una copia coerente del log in `path` (che non deve esistere), anche mentre e' aperto: per la copia delle impostazioni.
    pub fn copy_to(&self, path: &Path) -> Result<(), LogError> {
        let conn = self.conn.lock().unwrap_or_else(|p| p.into_inner());
        conn.execute("VACUUM INTO ?1", [path.to_string_lossy().as_ref()])?;
        Ok(())
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

    /// Gli ultimi `limit` QSO del log interno, dal piu' recente, nella stessa forma del log di Log4OM2 (il pannello Log li mostra uguali).
    /// `country_of` da' il paese di un nominativo (vuoto se non lo conosce).
    pub fn recent_view(&self, limit: usize, country_of: &dyn Fn(&str) -> String) -> Result<log4om::Log4omView, LogError> {
        let total = self.count()?;
        let conn = self.conn()?;
        let mut stmt = conn.prepare(
            "SELECT call, grid, band, mode, rst_sent, rst_rcvd, date, time_on FROM qso ORDER BY date DESC, time_on DESC, id DESC LIMIT ?1",
        )?;
        let rows = stmt.query_map(params![limit as i64], |r| {
            let (call, grid, band, mode, sent, rcvd, date, time): (String, String, String, String, String, String, String, String) =
                (r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?, r.get(5)?, r.get(6)?, r.get(7)?);
            Ok((call, grid, band, mode, sent, rcvd, date, time))
        })?;
        let mut qsos = Vec::new();
        for row in rows {
            let (call, grid, band, mode, rst_sent, rst_rcvd, date, time) = row?;
            // «2026-11-01 08:15:00Z» come nel log di Log4OM2
            let d = if date.len() >= 8 { format!("{}-{}-{}", &date[0..4], &date[4..6], &date[6..8]) } else { date.clone() };
            let t = if time.len() >= 4 { format!("{}:{}:{}", &time[0..2], &time[2..4], if time.len() >= 6 { &time[4..6] } else { "00" }) } else { "00:00:00".to_string() };
            let country = country_of(&call);
            qsos.push(log4om::Log4omQso { date: format!("{d} {t}Z"), call, band: band.to_lowercase(), mode: mode.to_uppercase(), rst_sent, rst_rcvd, country, grid });
        }
        Ok(log4om::Log4omView { total, qsos })
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

    /// Tutti i QSO del log interno, dal piu' recente, con il loro numero (`id`): la pagina di esportazione li elenca e sceglie quali salvare.
    pub fn entries(&self) -> Result<Vec<LogEntry>, LogError> {
        let conn = self.conn()?;
        let mut stmt = conn.prepare(
            "SELECT id, date, time_on, call, band, mode, freq_hz, rst_sent, rst_rcvd, grid FROM qso ORDER BY date DESC, time_on DESC, id DESC",
        )?;
        let rows = stmt.query_map([], |r| {
            Ok(LogEntry {
                id: r.get(0)?,
                date: r.get(1)?,
                time: r.get(2)?,
                call: r.get(3)?,
                band: r.get::<_, String>(4)?.to_lowercase(),
                mode: r.get::<_, String>(5)?.to_uppercase(),
                freq_hz: r.get::<_, i64>(6)?.max(0) as u64,
                rst_sent: r.get(7)?,
                rst_rcvd: r.get(8)?,
                grid: r.get(9)?,
            })
        })?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    /// Cancella i QSO con questi id e ritorna quelli cancellati (per toglierli anche dal file ADIF di Aurora). Gli id che non ci sono si saltano.
    pub fn delete(&self, ids: &[i64]) -> Result<Vec<LogEntry>, LogError> {
        let mut conn = self.conn()?;
        let tx = conn.transaction()?;
        let mut gone = Vec::new();
        for &id in ids {
            let row = tx
                .query_row(
                    "SELECT id, date, time_on, call, band, mode, freq_hz, rst_sent, rst_rcvd, grid FROM qso WHERE id = ?1",
                    params![id],
                    |r| {
                        Ok(LogEntry {
                            id: r.get(0)?,
                            date: r.get(1)?,
                            time: r.get(2)?,
                            call: r.get(3)?,
                            band: r.get::<_, String>(4)?.to_lowercase(),
                            mode: r.get::<_, String>(5)?.to_uppercase(),
                            freq_hz: r.get::<_, i64>(6)?.max(0) as u64,
                            rst_sent: r.get(7)?,
                            rst_rcvd: r.get(8)?,
                            grid: r.get(9)?,
                        })
                    },
                )
                .optional()?;
            if let Some(e) = row {
                tx.execute("DELETE FROM qso WHERE id = ?1", params![id])?;
                gone.push(e);
            }
        }
        tx.commit()?;
        Ok(gone)
    }

    /// Svuota il log; ritorna quanti QSO c'erano.
    pub fn clear(&self) -> Result<u64, LogError> {
        let n = self.conn()?.execute("DELETE FROM qso", [])?;
        Ok(n as u64)
    }

    /// Il log come file ADIF di JTDX (`wsjtx_log.adi`: stesse righe di `jtdx_file_record`, CR LF), con i QSO tra due date (`AAAAMMGG`, estremi
    /// compresi; `None` = senza limite). Ritorna il testo e quanti QSO contiene.
    pub fn export_jtdx_adif(&self, station: &adif::Station, from: Option<&str>, to: Option<&str>) -> Result<(String, usize), LogError> {
        self.export_jtdx_adif_selected(station, from, to, None)
    }

    /// Come `export_jtdx_adif`, ma con i soli QSO il cui numero (`id`, quello di `entries`) e' in `ids` (`None` = tutti). Un numero che
    /// non c'e' nel log si ignora.
    /// Il log come Cabrillo 3.0 generico (per i contest senza regole in Aurora o per chi lo chiede in un formato solo): una riga `QSO:` per
    /// collegamento con frequenza in kHz, modo `DG` (digitale), data, ora, nominativo, rapporto e locatore inviati e ricevuti.
    /// Stessi filtri di `export_jtdx_adif_selected` (date `AAAAMMGG` comprese, `ids` scelti). Restituisce il testo e il numero di QSO.
    pub fn export_cabrillo_selected(&self, station: &adif::Station, from: Option<&str>, to: Option<&str>, ids: Option<&[i64]>) -> Result<(String, usize), LogError> {
        let wanted: Option<std::collections::HashSet<i64>> = ids.map(|ids| ids.iter().copied().collect());
        let (from, to) = (from.unwrap_or(""), to.unwrap_or(""));
        let mut rows: Vec<LogEntry> = self
            .entries()?
            .into_iter()
            .filter(|e| (from.is_empty() || e.date.as_str() >= from) && (to.is_empty() || e.date.as_str() <= to))
            .filter(|e| wanted.as_ref().is_none_or(|w| w.contains(&e.id)))
            .collect();
        rows.sort_by(|a, b| (a.date.as_str(), a.time.as_str(), a.id).cmp(&(b.date.as_str(), b.time.as_str(), b.id)));
        let me = station.call.trim().to_uppercase();
        let my_grid: String = station.grid.trim().chars().take(4).collect::<String>().to_uppercase();
        let mut out = String::new();
        out.push_str("START-OF-LOG: 3.0\r\n");
        out.push_str(&format!("CALLSIGN: {me}\r\n"));
        out.push_str("CATEGORY-MODE: DIGI\r\n");
        if !my_grid.is_empty() {
            out.push_str(&format!("GRID-LOCATOR: {my_grid}\r\n"));
        }
        out.push_str(&format!("CREATED-BY: Aurora {}\r\n", env!("CARGO_PKG_VERSION")));
        let field = |s: &str, dflt: &str| -> String {
            let t = s.trim().to_uppercase();
            if t.is_empty() { dflt.to_string() } else { t }
        };
        for e in &rows {
            let date = match (e.date.get(0..4), e.date.get(4..6), e.date.get(6..8)) {
                (Some(y), Some(m), Some(d)) => format!("{y}-{m}-{d}"),
                _ => e.date.clone(),
            };
            let hhmm: String = e.time.chars().take(4).collect();
            out.push_str(&format!(
                "QSO: {:>5} DG {date} {hhmm} {:<13} {:>3} {:<6} {:<13} {:>3} {:<6}\r\n",
                e.freq_hz / 1000,
                me,
                field(&e.rst_sent, "-"),
                field(&my_grid, "-"),
                field(&e.call, "-"),
                field(&e.rst_rcvd, "-"),
                field(&e.grid.chars().take(4).collect::<String>(), "-"),
            ));
        }
        out.push_str("END-OF-LOG:\r\n");
        Ok((out, rows.len()))
    }

    pub fn export_jtdx_adif_selected(&self, station: &adif::Station, from: Option<&str>, to: Option<&str>, ids: Option<&[i64]>) -> Result<(String, usize), LogError> {
        let wanted: Option<std::collections::HashSet<i64>> = ids.map(|ids| ids.iter().copied().collect());
        let conn = self.conn()?;
        let mut stmt = conn.prepare(
            "SELECT call, grid, band, mode, freq_hz, rst_sent, rst_rcvd, date, time_on, date_off, time_off, id FROM qso
             WHERE (?1 = '' OR date >= ?1) AND (?2 = '' OR date <= ?2) ORDER BY date, time_on, id",
        )?;
        let rows = stmt.query_map(params![from.unwrap_or(""), to.unwrap_or("")], |r| {
            let id: i64 = r.get(11)?;
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
                id,
            ))
        })?;
        let mut out = String::from(adif::JTDX_FILE_HEADER);
        let mut count = 0;
        for row in rows {
            let (q, date, id) = row?;
            if wanted.as_ref().is_some_and(|w| !w.contains(&id)) {
                continue;
            }
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

/// Il testo di un file ADIF senza i QSO cancellati dal log (stesso nominativo, data, banda e ora di inizio al minuto); ritorna anche quanti
/// ne ha tolti. Tutto il resto (intestazione, campi in piu', QSO non toccati) resta com'era.
pub fn adif_without(text: &str, gone: &[LogEntry]) -> (String, usize) {
    let lower = text.to_ascii_lowercase();
    let mut out = String::with_capacity(text.len());
    let mut start = 0;
    let mut removed = 0;
    // l'intestazione arriva fino a <eoh>
    if let Some(h) = lower.find("<eoh>") {
        start = h + 5;
        out.push_str(&text[..start]);
    }
    while let Some(rel) = lower[start..].find("<eor>") {
        let end = start + rel + 5;
        let record = &text[start..end];
        let field = |name: &str| adif_field(record, name).unwrap_or_default();
        let matches = gone.iter().any(|e| {
            field("call").eq_ignore_ascii_case(&e.call)
                && field("qso_date") == e.date
                && field("band").eq_ignore_ascii_case(&e.band)
                && field("time_on").get(0..4) == e.time.get(0..4)
        });
        if matches {
            removed += 1;
        } else {
            out.push_str(record);
        }
        start = end;
    }
    out.push_str(&text[start..]);
    (out, removed)
}

/// Il valore di un campo ADIF (`<CALL:5>K1ABC`), senza badare a maiuscole e minuscole nel nome.
fn adif_field(record: &str, name: &str) -> Option<String> {
    let lower = record.to_ascii_lowercase();
    let tag = format!("<{name}:");
    let at = lower.find(&tag)? + tag.len();
    let close = at + lower[at..].find('>')?;
    let len: usize = lower[at..close].split(':').next()?.trim().parse().ok()?;
    record.get(close + 1..close + 1 + len).map(|v| v.trim().to_string())
}

