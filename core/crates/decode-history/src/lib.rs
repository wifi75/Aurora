//! Storico dei messaggi decodificati. Ogni messaggio ricevuto si ricorda per qualche settimana e si puo' cercare per nominativo,
//! banda, modo e periodo («chi ho sentito ieri sui 20 metri?»). **Non e' il log dei QSO**: qui ci sono i messaggi sentiti, non i
//! collegamenti fatti, e non vanno nel log (quello lo tiene Log4OM2).

use std::{path::Path, sync::Mutex};

use rusqlite::{Connection, params, params_from_iter, types::Value};
use serde::Serialize;

/// Quante righe al massimo per pagina.
pub const MAX_PAGE: u32 = 500;

/// Un messaggio da ricordare.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Entry {
    /// Quando e' stato ricevuto (millisecondi Unix).
    pub ts_ms: u64,
    pub band: Option<String>,
    pub mode: String,
    pub dial_hz: u64,
    /// Frequenza audio del segnale (Hz).
    pub freq_hz: u32,
    pub snr: i32,
    pub dt: f32,
    pub text: String,
    /// Chi trasmette, se si riconosce.
    pub call: Option<String>,
    pub country: Option<String>,
    pub continent: Option<String>,
    /// L'etichetta data dal motore (`cq`, `new_dxcc`, `worked_call`...).
    pub tag: Option<String>,
}

/// Una riga dello storico.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Row {
    pub id: i64,
    #[serde(flatten)]
    pub entry: Entry,
}

/// Cosa cercare. Nominativo: cerca nel mittente e nel testo, anche solo una parte, senza distinguere le maiuscole.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Query {
    pub text: Option<String>,
    pub band: Option<String>,
    pub mode: Option<String>,
    pub from_ms: Option<u64>,
    pub to_ms: Option<u64>,
    pub limit: u32,
    pub offset: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Page {
    /// Quante righe corrispondono in tutto, non solo in questa pagina.
    pub total: u64,
    pub rows: Vec<Row>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Stats {
    pub total: u64,
    /// Il messaggio piu' vecchio ricordato (millisecondi Unix).
    pub oldest_ms: Option<u64>,
    /// Quante stazioni diverse.
    pub calls: u64,
}

#[derive(Debug)]
pub struct HistoryError(String);

impl std::fmt::Display for HistoryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "storico dei messaggi: {}", self.0)
    }
}

impl std::error::Error for HistoryError {}

impl From<rusqlite::Error> for HistoryError {
    fn from(e: rusqlite::Error) -> Self {
        HistoryError(e.to_string())
    }
}

pub struct HistoryStore {
    conn: Mutex<Connection>,
}

const SCHEMA: &str = "CREATE TABLE IF NOT EXISTS decodes (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    ts_ms INTEGER NOT NULL,
    band TEXT,
    mode TEXT NOT NULL,
    dial_hz INTEGER NOT NULL,
    freq_hz INTEGER NOT NULL,
    snr INTEGER NOT NULL,
    dt REAL NOT NULL,
    text TEXT NOT NULL,
    call TEXT,
    country TEXT,
    continent TEXT,
    tag TEXT
);
CREATE INDEX IF NOT EXISTS decodes_ts ON decodes(ts_ms);
CREATE INDEX IF NOT EXISTS decodes_call ON decodes(call);";

/// Mette al riparo `%`, `_` e `\` nel testo cercato, che vale alla lettera.
fn like_pattern(text: &str) -> String {
    let mut out = String::from("%");
    for c in text.trim().to_uppercase().chars() {
        if matches!(c, '%' | '_' | '\\') {
            out.push('\\');
        }
        out.push(c);
    }
    out.push('%');
    out
}

impl HistoryStore {
    pub fn open(path: &Path) -> Result<HistoryStore, HistoryError> {
        Self::init(Connection::open(path)?)
    }

    pub fn open_in_memory() -> Result<HistoryStore, HistoryError> {
        Self::init(Connection::open_in_memory()?)
    }

    fn init(conn: Connection) -> Result<HistoryStore, HistoryError> {
        conn.execute_batch(SCHEMA)?;
        Ok(HistoryStore { conn: Mutex::new(conn) })
    }

    fn conn(&self) -> std::sync::MutexGuard<'_, Connection> {
        self.conn.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    /// Aggiunge i messaggi in un'unica transazione.
    pub fn insert_many(&self, entries: &[Entry]) -> Result<(), HistoryError> {
        if entries.is_empty() {
            return Ok(());
        }
        let mut conn = self.conn();
        let tx = conn.transaction()?;
        {
            let mut stmt = tx.prepare_cached(
                "INSERT INTO decodes (ts_ms, band, mode, dial_hz, freq_hz, snr, dt, text, call, country, continent, tag) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12)",
            )?;
            for e in entries {
                stmt.execute(params![e.ts_ms as i64, e.band, e.mode, e.dial_hz as i64, e.freq_hz, e.snr, f64::from(e.dt), e.text, e.call, e.country, e.continent, e.tag])?;
            }
        }
        tx.commit()?;
        Ok(())
    }

    fn filter(q: &Query) -> (String, Vec<Value>) {
        let mut clauses: Vec<String> = Vec::new();
        let mut args: Vec<Value> = Vec::new();
        if let Some(text) = q.text.as_deref().filter(|t| !t.trim().is_empty()) {
            let pattern = like_pattern(text);
            clauses.push("(UPPER(IFNULL(call, '')) LIKE ? ESCAPE '\\' OR UPPER(text) LIKE ? ESCAPE '\\')".into());
            args.push(Value::Text(pattern.clone()));
            args.push(Value::Text(pattern));
        }
        if let Some(band) = q.band.as_deref().filter(|b| !b.is_empty()) {
            clauses.push("band = ?".into());
            args.push(Value::Text(band.to_string()));
        }
        if let Some(mode) = q.mode.as_deref().filter(|m| !m.is_empty()) {
            clauses.push("mode = ?".into());
            args.push(Value::Text(mode.to_string()));
        }
        if let Some(from) = q.from_ms {
            clauses.push("ts_ms >= ?".into());
            args.push(Value::Integer(from as i64));
        }
        if let Some(to) = q.to_ms {
            clauses.push("ts_ms <= ?".into());
            args.push(Value::Integer(to as i64));
        }
        let sql = if clauses.is_empty() { String::new() } else { format!(" WHERE {}", clauses.join(" AND ")) };
        (sql, args)
    }

    /// Le righe che corrispondono, le piu' recenti per prime, una pagina per volta.
    pub fn query(&self, q: &Query) -> Result<Page, HistoryError> {
        let (where_sql, args) = Self::filter(q);
        let limit = if q.limit == 0 { 100 } else { q.limit.min(MAX_PAGE) };
        let conn = self.conn();
        let total: i64 = conn.query_row(&format!("SELECT COUNT(*) FROM decodes{where_sql}"), params_from_iter(args.iter()), |r| r.get(0))?;
        let mut page_args = args;
        page_args.push(Value::Integer(i64::from(limit)));
        page_args.push(Value::Integer(i64::from(q.offset)));
        let mut stmt = conn.prepare(&format!(
            "SELECT id, ts_ms, band, mode, dial_hz, freq_hz, snr, dt, text, call, country, continent, tag FROM decodes{where_sql} ORDER BY ts_ms DESC, id DESC LIMIT ? OFFSET ?"
        ))?;
        let rows = stmt
            .query_map(params_from_iter(page_args.iter()), |r| {
                Ok(Row {
                    id: r.get(0)?,
                    entry: Entry {
                        ts_ms: r.get::<_, i64>(1)? as u64,
                        band: r.get(2)?,
                        mode: r.get(3)?,
                        dial_hz: r.get::<_, i64>(4)? as u64,
                        freq_hz: r.get(5)?,
                        snr: r.get(6)?,
                        dt: r.get::<_, f64>(7)? as f32,
                        text: r.get(8)?,
                        call: r.get(9)?,
                        country: r.get(10)?,
                        continent: r.get(11)?,
                        tag: r.get(12)?,
                    },
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Page { total: total as u64, rows })
    }

    pub fn stats(&self) -> Result<Stats, HistoryError> {
        let conn = self.conn();
        let (total, oldest, calls): (i64, Option<i64>, i64) =
            conn.query_row("SELECT COUNT(*), MIN(ts_ms), COUNT(DISTINCT call) FROM decodes", [], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))?;
        Ok(Stats { total: total as u64, oldest_ms: oldest.map(|v| v as u64), calls: calls as u64 })
    }

    /// Toglie i messaggi ricevuti prima di `ts_ms`; restituisce quanti.
    pub fn prune_before(&self, ts_ms: u64) -> Result<u64, HistoryError> {
        Ok(self.conn().execute("DELETE FROM decodes WHERE ts_ms < ?1", params![ts_ms as i64])? as u64)
    }

    /// Svuota lo storico; restituisce quanti messaggi c'erano.
    pub fn clear(&self) -> Result<u64, HistoryError> {
        Ok(self.conn().execute("DELETE FROM decodes", [])? as u64)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(ts_ms: u64, text: &str, call: Option<&str>, band: &str, mode: &str) -> Entry {
        Entry {
            ts_ms,
            band: Some(band.into()),
            mode: mode.into(),
            dial_hz: 14_074_000,
            freq_hz: 1500,
            snr: -10,
            dt: 0.2,
            text: text.into(),
            call: call.map(Into::into),
            country: Some("Italy".into()),
            continent: Some("EU".into()),
            tag: Some("cq".into()),
        }
    }

    fn store() -> HistoryStore {
        let s = HistoryStore::open_in_memory().unwrap();
        s.insert_many(&[
            entry(1_000, "CQ K1ABC FN42", Some("K1ABC"), "20m", "FT8"),
            entry(2_000, "K1ABC IU3ABC -12", Some("IU3ABC"), "20m", "FT8"),
            entry(3_000, "CQ DX JA1XYZ PM95", Some("JA1XYZ"), "40m", "FT8"),
            entry(4_000, "CQ W1AW FN31", Some("W1AW"), "20m", "FT4"),
        ])
        .unwrap();
        s
    }

    fn ids(page: &Page) -> Vec<u64> {
        page.rows.iter().map(|r| r.entry.ts_ms).collect()
    }

    #[test]
    fn returns_the_most_recent_first_with_the_total() {
        let page = store().query(&Query::default()).unwrap();
        assert_eq!(page.total, 4);
        assert_eq!(ids(&page), vec![4_000, 3_000, 2_000, 1_000]);
    }

    #[test]
    fn searches_the_sender_and_the_text_by_a_part_without_case() {
        let s = store();
        assert_eq!(ids(&s.query(&Query { text: Some("k1".into()), ..Query::default() }).unwrap()), vec![2_000, 1_000], "il mittente e il testo");
        assert_eq!(ids(&s.query(&Query { text: Some("ja1".into()), ..Query::default() }).unwrap()), vec![3_000]);
        assert_eq!(s.query(&Query { text: Some("   ".into()), ..Query::default() }).unwrap().total, 4, "un testo vuoto non filtra");
    }

    #[test]
    fn the_search_text_is_taken_literally() {
        let s = store();
        for wild in ["%", "_", "K%", "\\"] {
            assert_eq!(s.query(&Query { text: Some(wild.into()), ..Query::default() }).unwrap().total, 0, "{wild}");
        }
    }

    #[test]
    fn filters_by_band_mode_and_period_together() {
        let s = store();
        assert_eq!(ids(&s.query(&Query { band: Some("20m".into()), ..Query::default() }).unwrap()), vec![4_000, 2_000, 1_000]);
        assert_eq!(ids(&s.query(&Query { band: Some("20m".into()), mode: Some("FT8".into()), ..Query::default() }).unwrap()), vec![2_000, 1_000]);
        assert_eq!(ids(&s.query(&Query { from_ms: Some(2_000), to_ms: Some(3_000), ..Query::default() }).unwrap()), vec![3_000, 2_000]);
    }

    #[test]
    fn pages_through_the_results_and_caps_the_page_size() {
        let s = store();
        let first = s.query(&Query { limit: 3, ..Query::default() }).unwrap();
        assert_eq!((first.total, first.rows.len()), (4, 3));
        let second = s.query(&Query { limit: 3, offset: 3, ..Query::default() }).unwrap();
        assert_eq!(ids(&second), vec![1_000]);
        let many: Vec<Entry> = (0..600).map(|i| entry(10_000 + i, "CQ X1X", None, "20m", "FT8")).collect();
        s.insert_many(&many).unwrap();
        assert_eq!(s.query(&Query { limit: 100_000, ..Query::default() }).unwrap().rows.len(), MAX_PAGE as usize);
    }

    #[test]
    fn counts_distinct_stations_and_remembers_the_oldest() {
        let stats = store().stats().unwrap();
        assert_eq!((stats.total, stats.oldest_ms, stats.calls), (4, Some(1_000), 4));
        assert_eq!(HistoryStore::open_in_memory().unwrap().stats().unwrap(), Stats { total: 0, oldest_ms: None, calls: 0 });
    }

    #[test]
    fn prunes_old_messages_and_clears_everything() {
        let s = store();
        assert_eq!(s.prune_before(3_000).unwrap(), 2);
        assert_eq!(s.query(&Query::default()).unwrap().total, 2);
        assert_eq!(s.clear().unwrap(), 2);
        assert_eq!(s.query(&Query::default()).unwrap().total, 0);
    }

    #[test]
    fn survives_a_restart_on_a_file() {
        let path = std::env::temp_dir().join(format!("aurora-history-{}.sqlite", std::process::id()));
        std::fs::remove_file(&path).ok();
        {
            let s = HistoryStore::open(&path).unwrap();
            s.insert_many(&[entry(1, "CQ K1ABC", Some("K1ABC"), "20m", "FT8")]).unwrap();
        }
        assert_eq!(HistoryStore::open(&path).unwrap().stats().unwrap().total, 1);
        std::fs::remove_file(&path).ok();
    }
}
