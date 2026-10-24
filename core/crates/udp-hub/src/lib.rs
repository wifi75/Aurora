//! Hub UDP tra JTDX e gli altri programmi della stazione.
//!
//! JTDX manda il suo flusso a un solo indirizzo. L'hub lo riceve, lo mette a disposizione di Aurora
//! e lo **inoltra identico** a ogni destinazione configurata (per esempio GridTracker2); i comandi che
//! arrivano da quelle destinazioni vengono girati a JTDX. I datagrammi che non iniziano con il numero
//! magico del protocollo vengono scartati e mai inoltrati.

use std::{
    net::SocketAddr,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc, Mutex,
    },
};

use tokio::{net::UdpSocket, sync::broadcast, task::JoinHandle};

const MAX_DATAGRAM: usize = 65_535;
const SUBSCRIBER_CAPACITY: usize = 1024;

#[derive(Debug, Clone)]
pub struct HubConfig {
    /// Indirizzo su cui ascolta il flusso di JTDX.
    pub listen: SocketAddr,
    /// Destinazioni a cui inoltrare il flusso identico (GridTracker2, ...).
    pub forward: Vec<SocketAddr>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HubError {
    Bind(String),
    /// JTDX non ha ancora mandato nulla: non si sa a quale indirizzo scrivere.
    NoPeer,
    Send(String),
}

impl std::fmt::Display for HubError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HubError::Bind(e) => write!(f, "porta UDP non disponibile: {e}"),
            HubError::NoPeer => write!(f, "JTDX non ha ancora inviato nulla"),
            HubError::Send(e) => write!(f, "invio fallito: {e}"),
        }
    }
}

impl std::error::Error for HubError {}

struct Shared {
    listen: UdpSocket,
    peer: Mutex<Option<SocketAddr>>,
    dropped: AtomicU64,
    tx: broadcast::Sender<Vec<u8>>,
}

pub struct Hub {
    shared: Arc<Shared>,
    local: SocketAddr,
    tasks: Mutex<Vec<JoinHandle<()>>>,
}

fn has_magic(data: &[u8]) -> bool {
    data.len() >= 4 && data[..4] == udp_bridge::MAGIC.to_be_bytes()
}

impl Hub {
    pub async fn start(config: HubConfig) -> Result<Hub, HubError> {
        let listen = UdpSocket::bind(config.listen)
            .await
            .map_err(|e| HubError::Bind(format!("{}: {e}", config.listen)))?;
        let local = listen.local_addr().map_err(|e| HubError::Bind(e.to_string()))?;
        let (tx, _) = broadcast::channel(SUBSCRIBER_CAPACITY);
        let shared = Arc::new(Shared { listen, peer: Mutex::new(None), dropped: AtomicU64::new(0), tx });

        // un socket dedicato per destinazione: i comandi di ritorno arrivano sull'indirizzo da cui hanno ricevuto
        let mut forwards = Vec::new();
        for target in &config.forward {
            let bind_text = match (target.is_ipv4(), target.ip().is_loopback()) {
                (true, true) => "127.0.0.1:0",
                (true, false) => "0.0.0.0:0",
                (false, true) => "[::1]:0",
                (false, false) => "[::]:0",
            };
            let bind: SocketAddr = bind_text.parse().map_err(|e: std::net::AddrParseError| HubError::Bind(e.to_string()))?;
            let socket = UdpSocket::bind(bind).await.map_err(|e| HubError::Bind(format!("inoltro verso {target}: {e}")))?;
            forwards.push((*target, Arc::new(socket)));
        }

        let mut tasks = Vec::new();
        let sockets: Vec<(SocketAddr, Arc<UdpSocket>)> = forwards.clone();
        tasks.push(tokio::spawn(receive_from_jtdx(Arc::clone(&shared), sockets)));
        for (target, socket) in forwards {
            tasks.push(tokio::spawn(relay_commands(Arc::clone(&shared), target, socket)));
        }
        Ok(Hub { shared, local, tasks: Mutex::new(tasks) })
    }

    pub fn local_addr(&self) -> SocketAddr {
        self.local
    }

    /// Indirizzo da cui JTDX ha mandato l'ultimo datagramma valido.
    pub fn jtdx_addr(&self) -> Option<SocketAddr> {
        self.shared.peer.lock().ok().and_then(|p| *p)
    }

    /// I datagrammi validi arrivati da JTDX, nell'ordine di arrivo.
    pub fn subscribe(&self) -> broadcast::Receiver<Vec<u8>> {
        self.shared.tx.subscribe()
    }

    /// Datagrammi scartati perche' non iniziano con il numero magico.
    pub fn dropped_count(&self) -> u64 {
        self.shared.dropped.load(Ordering::Relaxed)
    }

    pub async fn send_to_jtdx(&self, bytes: &[u8]) -> Result<(), HubError> {
        let peer = self.jtdx_addr().ok_or(HubError::NoPeer)?;
        self.shared.listen.send_to(bytes, peer).await.map(|_| ()).map_err(|e| HubError::Send(e.to_string()))
    }

    pub fn stop(&self) {
        if let Ok(tasks) = self.tasks.lock() {
            for t in tasks.iter() {
                t.abort();
            }
        }
    }
}

impl Drop for Hub {
    fn drop(&mut self) {
        self.stop();
    }
}

async fn receive_from_jtdx(shared: Arc<Shared>, forwards: Vec<(SocketAddr, Arc<UdpSocket>)>) {
    let mut buf = vec![0u8; MAX_DATAGRAM];
    loop {
        let (n, from) = match shared.listen.recv_from(&mut buf).await {
            Ok(r) => r,
            // su Windows un ICMP "porta chiusa" appare come errore di ricezione: si prosegue
            Err(e) => {
                tracing::debug!("errore di ricezione sulla porta di ascolto: {e}");
                tokio::time::sleep(std::time::Duration::from_millis(10)).await;
                continue;
            }
        };
        let data = &buf[..n];
        if !has_magic(data) {
            shared.dropped.fetch_add(1, Ordering::Relaxed);
            continue;
        }
        if let Ok(mut peer) = shared.peer.lock() {
            *peer = Some(from);
        }
        let _ = shared.tx.send(data.to_vec());
        for (target, socket) in &forwards {
            // mai bloccante: una destinazione lenta o chiusa non ferma le altre
            if let Err(e) = socket.try_send_to(data, *target) {
                tracing::debug!("inoltro verso {target} non riuscito: {e}");
            }
        }
    }
}

async fn relay_commands(shared: Arc<Shared>, target: SocketAddr, socket: Arc<UdpSocket>) {
    let mut buf = vec![0u8; MAX_DATAGRAM];
    loop {
        let (n, _from) = match socket.recv_from(&mut buf).await {
            Ok(r) => r,
            Err(e) => {
                tracing::debug!("errore di ricezione dal lato {target}: {e}");
                tokio::time::sleep(std::time::Duration::from_millis(10)).await;
                continue;
            }
        };
        let data = &buf[..n];
        if !has_magic(data) {
            shared.dropped.fetch_add(1, Ordering::Relaxed);
            continue;
        }
        let Some(peer) = shared.peer.lock().ok().and_then(|p| *p) else {
            tracing::debug!("comando da {target} scartato: JTDX non ancora noto");
            continue;
        };
        if let Err(e) = shared.listen.send_to(data, peer).await {
            tracing::debug!("comando verso JTDX non riuscito: {e}");
        }
    }
}
