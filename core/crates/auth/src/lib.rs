//! Associazione dei dispositivi e verifica dei token di sessione.
//!
//! Il token (256 bit casuali, in esadecimale) esiste in chiaro solo sul dispositivo;
//! nel database resta il suo hash SHA-256.

use std::{path::Path, sync::{Arc, Mutex}, time::{SystemTime, UNIX_EPOCH}};

use rusqlite::{params, Connection, OptionalExtension};
use sha2::{Digest, Sha256};

/// Durata predefinita del codice di associazione: 10 minuti, il tempo di arrivare al telefono.
const DEFAULT_CODE_TTL_SECS: u64 = 600;
const MIN_CODE_TTL_SECS: u64 = 60;
const MAX_CODE_TTL_SECS: u64 = 3600;
const MAX_FAILURES: u32 = 5;
const LOCK_SECS: u64 = 60;
const MAX_NAME_CHARS: usize = 64;

pub trait Clock: Send + Sync {
    fn now_secs(&self) -> u64;
}

pub struct SystemClock;

impl Clock for SystemClock {
    fn now_secs(&self) -> u64 {
        SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |d| d.as_secs())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AuthError {
    BadCode,
    Expired,
    Locked { retry_after_secs: u64 },
    BadName,
    Storage(String),
}

impl std::fmt::Display for AuthError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AuthError::BadCode => write!(f, "codice non valido"),
            AuthError::Expired => write!(f, "codice scaduto"),
            AuthError::Locked { retry_after_secs } => {
                write!(f, "troppi tentativi, riprova tra {retry_after_secs} secondi")
            }
            AuthError::BadName => write!(f, "nome del dispositivo non valido"),
            AuthError::Storage(e) => write!(f, "errore di archiviazione: {e}"),
        }
    }
}

impl std::error::Error for AuthError {}

impl From<rusqlite::Error> for AuthError {
    fn from(e: rusqlite::Error) -> Self {
        AuthError::Storage(e.to_string())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PairingCode {
    pub code: String,
    pub expires_in_secs: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Token(pub String);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DeviceId(pub i64);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeviceInfo {
    pub id: DeviceId,
    pub name: String,
    pub created_at: u64,
}

#[derive(Default)]
struct Pairing {
    code: Option<(String, u64)>,
    failures: u32,
    locked_until: u64,
}

pub struct DeviceStore {
    conn: Mutex<Connection>,
    clock: Arc<dyn Clock>,
    pairing: Mutex<Pairing>,
    code_ttl_secs: std::sync::atomic::AtomicU64,
}

fn random_bytes<const N: usize>() -> Result<[u8; N], AuthError> {
    let mut buf = [0u8; N];
    getrandom::fill(&mut buf).map_err(|e| AuthError::Storage(format!("generatore casuale: {e}")))?;
    Ok(buf)
}

fn hash_token(token: &str) -> String {
    hex::encode(Sha256::digest(token.as_bytes()))
}

/// Confronto in tempo costante per i codici di associazione.
fn ct_eq(a: &str, b: &str) -> bool {
    let (a, b) = (a.as_bytes(), b.as_bytes());
    let mut diff = a.len() ^ b.len();
    for i in 0..a.len().max(b.len()) {
        diff |= usize::from(a.get(i).copied().unwrap_or(0) ^ b.get(i).copied().unwrap_or(0));
    }
    diff == 0
}

impl DeviceStore {
    pub fn open(path: &Path, clock: Arc<dyn Clock>) -> Result<DeviceStore, AuthError> {
        Self::from_connection(Connection::open(path)?, clock)
    }

    pub fn open_in_memory(clock: Arc<dyn Clock>) -> Result<DeviceStore, AuthError> {
        Self::from_connection(Connection::open_in_memory()?, clock)
    }

    fn from_connection(conn: Connection, clock: Arc<dyn Clock>) -> Result<DeviceStore, AuthError> {
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS devices (
                id INTEGER PRIMARY KEY,
                name TEXT NOT NULL,
                token_hash TEXT NOT NULL UNIQUE,
                created_at INTEGER NOT NULL
            );",
        )?;
        Ok(DeviceStore { conn: Mutex::new(conn), clock, pairing: Mutex::new(Pairing::default()), code_ttl_secs: std::sync::atomic::AtomicU64::new(DEFAULT_CODE_TTL_SECS) })
    }

    /// Durata del codice di associazione, tenuta tra un minuto e un'ora. Il codice resta monouso.
    pub fn with_code_ttl(self, secs: u64) -> DeviceStore {
        self.set_code_ttl(secs);
        self
    }

    /// Cambia la durata dei codici da ora in poi (anche a programma acceso).
    pub fn set_code_ttl(&self, secs: u64) {
        self.code_ttl_secs.store(secs.clamp(MIN_CODE_TTL_SECS, MAX_CODE_TTL_SECS), std::sync::atomic::Ordering::SeqCst);
    }

    pub fn code_ttl_secs(&self) -> u64 {
        self.code_ttl_secs.load(std::sync::atomic::Ordering::SeqCst)
    }

    /// Limiti ammessi per la durata, in secondi.
    pub const fn code_ttl_limits() -> (u64, u64) {
        (MIN_CODE_TTL_SECS, MAX_CODE_TTL_SECS)
    }

    fn conn(&self) -> Result<std::sync::MutexGuard<'_, Connection>, AuthError> {
        self.conn.lock().map_err(|_| AuthError::Storage("lucchetto avvelenato".into()))
    }

    fn pairing(&self) -> Result<std::sync::MutexGuard<'_, Pairing>, AuthError> {
        self.pairing.lock().map_err(|_| AuthError::Storage("lucchetto avvelenato".into()))
    }

    /// Genera un nuovo codice di 8 cifre; sostituisce quello eventualmente attivo.
    pub fn start_pairing(&self) -> Result<PairingCode, AuthError> {
        let n = u64::from_be_bytes(random_bytes::<8>()?) % 100_000_000;
        let code = format!("{n:08}");
        let ttl = self.code_ttl_secs();
        let expires = self.clock.now_secs() + ttl;
        self.pairing()?.code = Some((code.clone(), expires));
        Ok(PairingCode { code, expires_in_secs: ttl })
    }

    pub fn finish_pairing(&self, code: &str, device_name: &str) -> Result<Token, AuthError> {
        let now = self.clock.now_secs();
        let name = device_name.trim();
        if name.is_empty() || name.chars().count() > MAX_NAME_CHARS {
            return Err(AuthError::BadName);
        }
        let mut p = self.pairing()?;
        if p.locked_until > now {
            return Err(AuthError::Locked { retry_after_secs: p.locked_until - now });
        }
        if p.locked_until != 0 && p.locked_until <= now {
            p.locked_until = 0;
            p.failures = 0;
        }
        let Some((expected, expires)) = p.code.clone() else {
            return Self::register_failure(&mut p, now);
        };
        if now > expires {
            p.code = None;
            return Err(AuthError::Expired);
        }
        if !ct_eq(&expected, code) {
            return Self::register_failure(&mut p, now);
        }
        p.code = None;
        p.failures = 0;
        drop(p);

        let token = hex::encode(random_bytes::<32>()?);
        self.conn()?.execute(
            "INSERT INTO devices (name, token_hash, created_at) VALUES (?1, ?2, ?3)",
            params![name, hash_token(&token), now as i64],
        )?;
        Ok(Token(token))
    }

    fn register_failure(p: &mut Pairing, now: u64) -> Result<Token, AuthError> {
        p.failures += 1;
        if p.failures >= MAX_FAILURES {
            p.locked_until = now + LOCK_SECS;
        }
        Err(AuthError::BadCode)
    }

    pub fn verify(&self, token: &str) -> Option<DeviceId> {
        if token.len() != 64 || !token.chars().all(|c| c.is_ascii_hexdigit()) {
            return None;
        }
        let conn = self.conn().ok()?;
        conn.query_row(
            "SELECT id FROM devices WHERE token_hash = ?1",
            params![hash_token(token)],
            |r| r.get::<_, i64>(0),
        )
        .optional()
        .ok()
        .flatten()
        .map(DeviceId)
    }

    /// Il dispositivo e' ancora associato (non revocato)?
    pub fn exists(&self, id: DeviceId) -> bool {
        self.conn()
            .and_then(|c| {
                c.query_row("SELECT COUNT(*) FROM devices WHERE id = ?1", params![id.0], |r| r.get::<_, i64>(0))
                    .map_err(AuthError::from)
            })
            .is_ok_and(|n| n > 0)
    }

    pub fn list(&self) -> Result<Vec<DeviceInfo>, AuthError> {
        let conn = self.conn()?;
        let mut stmt = conn.prepare("SELECT id, name, created_at FROM devices ORDER BY id")?;
        let rows = stmt.query_map([], |r| {
            Ok(DeviceInfo { id: DeviceId(r.get(0)?), name: r.get(1)?, created_at: r.get::<_, i64>(2)? as u64 })
        })?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    pub fn revoke(&self, id: DeviceId) -> Result<bool, AuthError> {
        Ok(self.conn()?.execute("DELETE FROM devices WHERE id = ?1", params![id.0])? > 0)
    }
}
