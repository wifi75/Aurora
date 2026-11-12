//! Opzioni della cascata che chi le sceglie (il servizio dei modi testuali) e chi calcola le righe (l'analizzatore dello spettro) si
//! passano senza dipendere l'uno dall'altro.

use std::sync::atomic::{AtomicBool, Ordering};

static ANTICLICK: AtomicBool = AtomicBool::new(false);

/// Anti-click della cascata: i click di manipolazione (energia larga per pochi millisecondi) si tolgono dalle righe. Lo accende la
/// pagina PSK·RTTY·CW mentre ascolta, se l'opzione e' scelta.
pub fn set_anticlick(on: bool) {
    ANTICLICK.store(on, Ordering::Relaxed);
}

pub fn anticlick() -> bool {
    ANTICLICK.load(Ordering::Relaxed)
}
