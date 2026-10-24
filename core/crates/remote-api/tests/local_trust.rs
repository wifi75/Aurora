mod common;

use axum::{extract::ConnectInfo, http::{Method, StatusCode}};
use common::*;
use remote_api::AppState;
use std::net::SocketAddr;

async fn devices(s: &AppState, ip: &str, proxied: bool) -> StatusCode {
    let mut r = req(Method::GET, "/api/v1/devices");
    r.peer = ConnectInfo(if ip.contains(':') { format!("[{ip}]:50000") } else { format!("{ip}:50000") }.parse::<SocketAddr>().unwrap());
    if proxied {
        r = r.header("x-forwarded-for", "1.2.3.4");
    }
    call(&router(s), r).await.status()
}

#[tokio::test]
async fn it_is_off_by_default() {
    let s = state();
    assert_eq!(devices(&s, "127.0.0.1", false).await, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn the_pc_gets_in_without_a_code_but_the_phone_does_not() {
    let s = state().with_local_trust(true);
    assert_eq!(devices(&s, "127.0.0.1", false).await, StatusCode::OK);
    assert_eq!(devices(&s, "::1", false).await, StatusCode::OK);
    for ip in ["192.168.1.50", "10.0.0.7", "8.8.8.8"] {
        assert_eq!(devices(&s, ip, false).await, StatusCode::UNAUTHORIZED, "{ip}");
    }
}

#[tokio::test]
async fn a_proxied_request_is_never_the_pc() {
    let s = state().with_local_trust(true);
    assert_eq!(devices(&s, "127.0.0.1", true).await, StatusCode::UNAUTHORIZED, "un tunnel arriva da loopback ma non e' il PC");
}
