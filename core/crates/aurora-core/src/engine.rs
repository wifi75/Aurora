use std::future::Future;

use tokio::sync::{broadcast, mpsc};

use crate::{Command, Event, SpectrumRow};

/// Canali che un motore usa per pubblicare eventi e ricevere comandi.
pub struct EngineIo {
    pub events: broadcast::Sender<Event>,
    pub spectrum: broadcast::Sender<SpectrumRow>,
    pub commands: mpsc::Receiver<Command>,
}

/// Un motore produce eventi e consuma comandi finche' il canale dei comandi resta aperto.
pub trait Engine: Send + 'static {
    fn run(self, io: EngineIo) -> impl Future<Output = ()> + Send;
}
