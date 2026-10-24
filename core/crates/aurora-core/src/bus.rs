use std::sync::{Arc, Mutex};

use tokio::sync::{broadcast, mpsc};

use crate::{Command, Event, SpectrumRow};

struct Inner {
    events: broadcast::Sender<Event>,
    spectrum: broadcast::Sender<SpectrumRow>,
    cmd_tx: mpsc::Sender<Command>,
    cmd_rx: Mutex<Option<mpsc::Receiver<Command>>>,
}

/// Punto di incontro tra motore e consumatori; si clona a basso costo.
#[derive(Clone)]
pub struct Bus {
    inner: Arc<Inner>,
}

impl Bus {
    pub fn new(capacity: usize) -> Bus {
        let (events, _) = broadcast::channel(capacity);
        let (spectrum, _) = broadcast::channel(capacity);
        let (cmd_tx, cmd_rx) = mpsc::channel(capacity);
        Bus { inner: Arc::new(Inner { events, spectrum, cmd_tx, cmd_rx: Mutex::new(Some(cmd_rx)) }) }
    }

    pub fn events(&self) -> broadcast::Sender<Event> {
        self.inner.events.clone()
    }

    pub fn spectrum(&self) -> broadcast::Sender<SpectrumRow> {
        self.inner.spectrum.clone()
    }

    pub fn subscribe_events(&self) -> broadcast::Receiver<Event> {
        self.inner.events.subscribe()
    }

    pub fn subscribe_spectrum(&self) -> broadcast::Receiver<SpectrumRow> {
        self.inner.spectrum.subscribe()
    }

    pub fn commands(&self) -> mpsc::Sender<Command> {
        self.inner.cmd_tx.clone()
    }

    /// Il ricevitore dei comandi e' uno solo: lo prende il motore, una volta.
    pub fn take_command_rx(&self) -> Option<mpsc::Receiver<Command>> {
        self.inner.cmd_rx.lock().ok().and_then(|mut g| g.take())
    }
}
