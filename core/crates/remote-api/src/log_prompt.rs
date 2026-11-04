//! «Richiedimi di loggare il QSO» (`PromptToLog`): con la voce accesa un QSO concluso dal motore non si registra da solo ma resta in attesa
//! qui, finche' l'operatore lo conferma (anche correggendo nominativo, locatore, rapporti, nome, citta' o commento) o lo scarta dalla
//! finestra della pagina. Confermato, diventa un `QsoLogged` come quelli registrati da soli: log interno, file ADIF e servizi.

use std::sync::{
    Mutex,
    atomic::{AtomicU64, Ordering},
};

use aurora_core::QsoRecord;
use serde::{Deserialize, Serialize};

/// Quanti QSO possono aspettare insieme: oltre, il piu' vecchio si registra da solo per non perderlo.
pub const MAX_PENDING: usize = 20;

/// Un QSO in attesa di conferma.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Pending {
    pub id: u64,
    pub qso: QsoRecord,
}

/// Le correzioni fatte nella finestra di conferma: un campo assente resta come l'ha scritto il motore.
#[derive(Clone, Debug, Default, Deserialize)]
pub struct Edits {
    pub call: Option<String>,
    pub grid: Option<String>,
    pub rst_sent: Option<String>,
    pub rst_rcvd: Option<String>,
    pub name: Option<String>,
    pub qth: Option<String>,
    pub comment: Option<String>,
}

impl Edits {
    /// Applica le correzioni; `Err` con il nome del campo se un valore non va (nominativo vuoto o con caratteri strani, testo troppo lungo).
    pub fn apply(self, qso: &mut QsoRecord) -> Result<(), &'static str> {
        fn text(value: Option<String>, max: usize, field: &'static str) -> Result<Option<String>, &'static str> {
            match value {
                None => Ok(None),
                Some(v) => {
                    let v = v.trim().to_string();
                    if v.chars().count() > max || v.chars().any(char::is_control) { Err(field) } else { Ok(Some(v)) }
                }
            }
        }
        if let Some(call) = text(self.call, 20, "call")? {
            let call = call.to_uppercase();
            if call.is_empty() || !call.chars().all(|c| c.is_ascii_alphanumeric() || c == '/') {
                return Err("call");
            }
            qso.call = call;
        }
        if let Some(grid) = text(self.grid, 8, "grid")? {
            if !grid.chars().all(|c| c.is_ascii_alphanumeric()) {
                return Err("grid");
            }
            qso.grid = grid;
        }
        if let Some(v) = text(self.rst_sent, 8, "rst_sent")? {
            qso.rst_sent = v;
        }
        if let Some(v) = text(self.rst_rcvd, 8, "rst_rcvd")? {
            qso.rst_rcvd = v;
        }
        if let Some(v) = text(self.name, 60, "name")? {
            qso.name = v;
        }
        if let Some(v) = text(self.qth, 60, "qth")? {
            qso.qth = v;
        }
        if let Some(v) = text(self.comment, 200, "comment")? {
            qso.comment = v;
        }
        Ok(())
    }
}

/// La coda dei QSO in attesa.
#[derive(Default)]
pub struct LogPrompt {
    pending: Mutex<Vec<Pending>>,
    next: AtomicU64,
}

impl LogPrompt {
    pub fn new() -> LogPrompt {
        LogPrompt { pending: Mutex::new(Vec::new()), next: AtomicU64::new(1) }
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, Vec<Pending>> {
        self.pending.lock().unwrap_or_else(|p| p.into_inner())
    }

    /// Mette in attesa un QSO concluso. Se la coda e' piena restituisce il piu' vecchio, da registrare subito.
    pub fn push(&self, qso: QsoRecord) -> Option<QsoRecord> {
        let id = self.next.fetch_add(1, Ordering::Relaxed);
        let mut pending = self.lock();
        pending.push(Pending { id, qso });
        (pending.len() > MAX_PENDING).then(|| pending.remove(0).qso)
    }

    /// I QSO in attesa, dal piu' vecchio.
    pub fn list(&self) -> Vec<Pending> {
        self.lock().clone()
    }

    /// Toglie dalla coda il QSO `id` (per registrarlo o scartarlo).
    pub fn take(&self, id: u64) -> Option<QsoRecord> {
        let mut pending = self.lock();
        let at = pending.iter().position(|p| p.id == id)?;
        Some(pending.remove(at).qso)
    }

    /// Svuota la coda (la voce e' stata spenta): restituisce i QSO, da registrare per non perderli.
    pub fn drain(&self) -> Vec<QsoRecord> {
        self.lock().drain(..).map(|p| p.qso).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn qso(call: &str) -> QsoRecord {
        QsoRecord { call: call.into(), grid: "FN42".into(), band: "20m".into(), mode: "FT8".into(), rst_sent: "-10".into(), rst_rcvd: "-12".into(), ..QsoRecord::default() }
    }

    #[test]
    fn a_qso_waits_until_it_is_taken_once() {
        let p = LogPrompt::new();
        assert!(p.push(qso("K1ABC")).is_none());
        let id = p.list()[0].id;
        assert_eq!(p.take(id).map(|q| q.call), Some("K1ABC".to_string()));
        assert!(p.take(id).is_none(), "una volta sola");
        assert!(p.list().is_empty());
    }

    #[test]
    fn a_full_queue_gives_back_the_oldest_so_nothing_is_lost() {
        let p = LogPrompt::new();
        for i in 0..MAX_PENDING {
            assert!(p.push(qso(&format!("K{i}ABC"))).is_none());
        }
        assert_eq!(p.push(qso("W1XYZ")).map(|q| q.call), Some("K0ABC".to_string()));
        assert_eq!(p.list().len(), MAX_PENDING);
        assert_eq!(p.drain().len(), MAX_PENDING);
    }

    #[test]
    fn the_edits_change_only_what_is_given_and_refuse_bad_values() {
        let mut q = qso("K1ABC");
        Edits { call: Some(" k1abc/p ".into()), rst_rcvd: Some("-05".into()), name: Some("Joe".into()), ..Edits::default() }.apply(&mut q).expect("valido");
        assert_eq!((q.call.as_str(), q.rst_rcvd.as_str(), q.rst_sent.as_str(), q.name.as_str()), ("K1ABC/P", "-05", "-10", "Joe"));
        assert_eq!(Edits { call: Some(String::new()), ..Edits::default() }.apply(&mut q), Err("call"));
        assert_eq!(Edits { call: Some("K1 ABC".into()), ..Edits::default() }.apply(&mut q), Err("call"));
        assert_eq!(Edits { grid: Some("FN-42".into()), ..Edits::default() }.apply(&mut q), Err("grid"));
        assert_eq!(Edits { comment: Some("x".repeat(201)), ..Edits::default() }.apply(&mut q), Err("comment"));
    }
}
