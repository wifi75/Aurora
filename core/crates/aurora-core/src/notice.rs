//! Avvisi per lo schermo da qualunque parte di Aurora (audio, PTT, OmniRig) senza passare il bus a ogni crate.
//! Lo stesso avviso non si ripete prima di `QUIET` per non riempire lo schermo.

use std::{
    collections::HashMap,
    sync::{Mutex, OnceLock},
    time::{Duration, Instant},
};

use tokio::sync::broadcast;

use crate::Event;

const QUIET: Duration = Duration::from_secs(120);

static SINK: OnceLock<broadcast::Sender<Event>> = OnceLock::new();
static LAST: OnceLock<Mutex<HashMap<String, Instant>>> = OnceLock::new();

/// Da chiamare una volta all'avvio con il bus degli eventi.
pub fn install(events: broadcast::Sender<Event>) {
    let _ = SINK.set(events);
}

/// Manda un avviso allo schermo (se nessuno ascolta, non succede niente).
pub fn notice(id: &str, detail: impl Into<String>) {
    let Some(sink) = SINK.get() else { return };
    let last = LAST.get_or_init(Mutex::default);
    let Ok(mut last) = last.lock() else { return };
    let now = Instant::now();
    if last.get(id).is_some_and(|t| now.duration_since(*t) < QUIET) {
        return;
    }
    last.insert(id.to_string(), now);
    let _ = sink.send(Event::Notice { id: id.to_string(), detail: detail.into() });
}
