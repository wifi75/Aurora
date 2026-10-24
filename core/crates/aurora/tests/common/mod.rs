#![allow(dead_code)]

use std::{
    net::SocketAddr,
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
    time::Duration,
};

use aurora::RunningServer;
use aurora_core::Event;
use futures_util::StreamExt;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
};
use tokio_tungstenite::{
    connect_async,
    tungstenite::{client::IntoClientRequest, Message},
};

static COUNTER: AtomicU64 = AtomicU64::new(0);

pub fn temp_dir(tag: &str) -> PathBuf {
    // l'orologio di Windows e' grossolano: il contatore evita cartelle uguali tra test paralleli
    let dir = std::env::temp_dir().join(format!(
        "aurora-e2e-{tag}-{}-{}-{}",
        std::process::id(),
        COUNTER.fetch_add(1, Ordering::SeqCst),
        std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

pub fn cookie_for(server: &RunningServer) -> String {
    let code = server.state.auth.start_pairing().unwrap();
    format!("aurora_session={}", server.state.auth.finish_pairing(&code.code, "e2e").unwrap().0)
}

/// Client HTTP minimo: restituisce (stato, corpo).
pub async fn http(addr: SocketAddr, method: &str, path: &str, cookie: Option<&str>, body: Option<&str>) -> (u16, String) {
    let mut s = TcpStream::connect(addr).await.unwrap();
    let body = body.unwrap_or("");
    let mut req = format!("{method} {path} HTTP/1.1\r\nHost: {addr}\r\nConnection: close\r\n");
    if let Some(c) = cookie {
        req.push_str(&format!("Cookie: {c}\r\n"));
    }
    if !body.is_empty() {
        req.push_str(&format!("Content-Type: application/json\r\nContent-Length: {}\r\n", body.len()));
    }
    req.push_str("\r\n");
    req.push_str(body);
    s.write_all(req.as_bytes()).await.unwrap();
    let mut raw = Vec::new();
    s.read_to_end(&mut raw).await.unwrap();
    let text = String::from_utf8_lossy(&raw).to_string();
    let status = text.split_whitespace().nth(1).unwrap().parse().unwrap();
    let body = text.split("\r\n\r\n").nth(1).unwrap_or("").to_string();
    (status, body)
}

pub type Ws = tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<TcpStream>>;

pub async fn open_ws(addr: SocketAddr, cookie: &str) -> Ws {
    let mut r = format!("ws://{addr}/api/v1/stream").into_client_request().unwrap();
    r.headers_mut().insert("Cookie", cookie.parse().unwrap());
    connect_async(r).await.unwrap().0
}

pub async fn next_event(ws: &mut Ws, secs: u64) -> Option<Event> {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(secs);
    while tokio::time::Instant::now() < deadline {
        if let Ok(Some(Ok(Message::Text(t)))) = tokio::time::timeout(Duration::from_millis(500), ws.next()).await {
            return serde_json::from_str(t.as_str()).ok();
        }
    }
    None
}
