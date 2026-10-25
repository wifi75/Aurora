//! Lo stato dell'ultimo invio di ogni servizio (PSK Reporter, DXSummit, eQSL, server TCP): serve alla scheda Segnalazioni per dire se funziona.

use std::{collections::BTreeMap, sync::Mutex, time::{SystemTime, UNIX_EPOCH}};

use serde_json::{json, Value};

/// I servizi di cui si tiene lo stato.
pub const KINDS: [&str; 4] = ["psk", "dxsummit", "eqsl", "tcp"];

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SendState {
    /// `None` = non e' ancora partito niente.
    pub ok: Option<bool>,
    /// Quando (millisecondi Unix) e' finito l'ultimo invio.
    pub at_ms: u64,
    /// Cosa e' successo, in una riga (mai password ne' indirizzi con credenziali).
    pub message: String,
    pub sent: u64,
    pub failed: u64,
}

#[derive(Default)]
pub struct ReportStatus {
    states: Mutex<BTreeMap<&'static str, SendState>>,
}

fn now_ms() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |d| u64::try_from(d.as_millis()).unwrap_or(u64::MAX))
}

impl ReportStatus {
    pub fn new() -> ReportStatus {
        ReportStatus::default()
    }

    fn with(&self, kind: &'static str, change: impl FnOnce(&mut SendState)) {
        let mut states = self.states.lock().unwrap_or_else(|p| p.into_inner());
        change(states.entry(kind).or_default());
    }

    /// Un invio riuscito.
    pub fn ok(&self, kind: &'static str, message: impl Into<String>) {
        let message = message.into();
        self.with(kind, |s| {
            s.ok = Some(true);
            s.at_ms = now_ms();
            s.message = message;
            s.sent += 1;
        });
    }

    /// Un invio fallito: l'errore va anche nel registro.
    pub fn fail(&self, kind: &'static str, message: impl Into<String>) {
        let message = message.into();
        tracing::warn!("invio {kind} non riuscito: {message}");
        self.with(kind, |s| {
            s.ok = Some(false);
            s.at_ms = now_ms();
            s.message = message;
            s.failed += 1;
        });
    }

    pub fn get(&self, kind: &str) -> SendState {
        self.states.lock().unwrap_or_else(|p| p.into_inner()).get(kind).cloned().unwrap_or_default()
    }

    /// Per l'API: una voce per servizio.
    pub fn snapshot(&self) -> Value {
        let mut out = serde_json::Map::new();
        for kind in KINDS {
            let s = self.get(kind);
            out.insert(
                kind.to_string(),
                json!({ "ok": s.ok, "at_ms": s.at_ms, "message": s.message, "sent": s.sent, "failed": s.failed }),
            );
        }
        Value::Object(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_state_starts_empty_and_follows_the_last_send() {
        let s = ReportStatus::new();
        assert_eq!(s.get("psk"), SendState::default());
        s.ok("psk", "12 spot");
        s.fail("psk", "rete non raggiungibile");
        let st = s.get("psk");
        assert_eq!((st.ok, st.message.as_str(), st.sent, st.failed), (Some(false), "rete non raggiungibile", 1, 1));
        s.ok("psk", "3 spot");
        assert_eq!(s.get("psk").ok, Some(true));
        let snap = s.snapshot();
        assert_eq!(snap["psk"]["sent"], 2);
        assert_eq!(snap["eqsl"]["ok"], Value::Null, "un servizio mai usato ha ok nullo");
        assert_eq!(snap.as_object().map(serde_json::Map::len), Some(4));
    }
}
