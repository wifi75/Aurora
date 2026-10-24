#![allow(dead_code)]

use std::{net::SocketAddr, sync::Arc, time::Duration};

use auth::{DeviceStore, SystemClock};
use aurora_core::Bus;
use axum::{
    body::Body,
    extract::ConnectInfo,
    http::{header, Method, Request, Response, StatusCode},
    Router,
};
use logbook::Logbook;
use remote_api::{build_router, AppState, TxPolicy};
use tower::ServiceExt;

pub fn state() -> AppState {
    state_with_live_tx(true)
}

pub fn state_with_live_tx(live: bool) -> AppState {
    let bus = Bus::new(4096);
    let auth = Arc::new(DeviceStore::open_in_memory(Arc::new(SystemClock)).unwrap());
    let logbook = Arc::new(Logbook::open_in_memory().unwrap());
    AppState::new(bus, auth, logbook, TxPolicy::default_safe().with_live_tx(live), false)
}

pub fn token_for(state: &AppState) -> String {
    let code = state.auth.start_pairing().unwrap();
    state.auth.finish_pairing(&code.code, "test").unwrap().0
}

pub fn cookie_for(state: &AppState) -> String {
    format!("aurora_session={}", token_for(state))
}

pub fn loopback() -> ConnectInfo<SocketAddr> {
    ConnectInfo("127.0.0.1:50000".parse().unwrap())
}

pub fn remote_peer() -> ConnectInfo<SocketAddr> {
    ConnectInfo("192.168.1.50:50000".parse().unwrap())
}

pub struct Req {
    pub method: Method,
    pub uri: String,
    pub cookie: Option<String>,
    pub body: Option<String>,
    pub headers: Vec<(&'static str, &'static str)>,
    pub peer: ConnectInfo<SocketAddr>,
}

pub fn req(method: Method, uri: &str) -> Req {
    Req { method, uri: uri.to_string(), cookie: None, body: None, headers: vec![], peer: remote_peer() }
}

impl Req {
    pub fn cookie(mut self, c: &str) -> Self {
        self.cookie = Some(c.to_string());
        self
    }
    pub fn json(mut self, body: &str) -> Self {
        self.body = Some(body.to_string());
        self
    }
    pub fn header(mut self, k: &'static str, v: &'static str) -> Self {
        self.headers.push((k, v));
        self
    }
    pub fn via_loopback(mut self) -> Self {
        self.peer = loopback();
        self
    }
}

pub async fn call(app: &Router, r: Req) -> Response<Body> {
    let mut b = Request::builder().method(r.method).uri(r.uri);
    if let Some(c) = r.cookie {
        b = b.header(header::COOKIE, c);
    }
    for (k, v) in r.headers {
        b = b.header(k, v);
    }
    let body = match r.body {
        Some(s) => {
            b = b.header(header::CONTENT_TYPE, "application/json");
            Body::from(s)
        }
        None => Body::empty(),
    };
    let mut request = b.body(body).unwrap();
    request.extensions_mut().insert(r.peer);
    app.clone().oneshot(request).await.unwrap()
}

pub async fn body_json(resp: Response<Body>) -> serde_json::Value {
    let bytes = axum::body::to_bytes(resp.into_body(), 1 << 20).await.unwrap();
    serde_json::from_slice(&bytes).unwrap_or(serde_json::Value::Null)
}

pub fn router(state: &AppState) -> Router {
    build_router(state.clone())
}

pub async fn wait_known(state: &AppState, id: u64) {
    for _ in 0..100 {
        if state.knows_decode(id) {
            return;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    panic!("decode {id} non indicizzato");
}

pub fn status_is(resp: &Response<Body>, code: StatusCode) {
    assert_eq!(resp.status(), code);
}
