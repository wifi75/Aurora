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

async fn devices_with(s: &AppState, headers: &[(&'static str, &'static str)]) -> StatusCode {
    let mut r = req(Method::GET, "/api/v1/devices");
    r.peer = ConnectInfo("127.0.0.1:50000".parse::<SocketAddr>().unwrap());
    for (k, v) in headers {
        r = r.header(k, v);
    }
    call(&router(s), r).await.status()
}

#[tokio::test]
async fn a_page_of_another_site_on_the_same_pc_is_not_the_pc() {
    let s = state().with_local_trust(true);
    // la pagina di Aurora (servita da Aurora stessa) e un programma senza Origin entrano
    assert_eq!(devices_with(&s, &[("host", "127.0.0.1:8787"), ("origin", "http://127.0.0.1:8787")]).await, StatusCode::OK);
    assert_eq!(devices_with(&s, &[("host", "localhost:8787"), ("origin", "http://localhost:5173")]).await, StatusCode::OK);
    assert_eq!(devices_with(&s, &[("host", "[::1]:8787")]).await, StatusCode::OK);
    // un altro sito aperto nel browser dello stesso PC: no (lettura dello stato, CQ con una richiesta «semplice»)
    assert_eq!(devices_with(&s, &[("host", "127.0.0.1:8787"), ("origin", "https://example.com")]).await, StatusCode::UNAUTHORIZED);
    assert_eq!(devices_with(&s, &[("host", "127.0.0.1:8787"), ("origin", "null")]).await, StatusCode::UNAUTHORIZED);
    // DNS rebinding: il nome del sito punta a 127.0.0.1, Host e Origin sono del sito
    assert_eq!(devices_with(&s, &[("host", "evil.example:8787"), ("origin", "http://evil.example:8787")]).await, StatusCode::UNAUTHORIZED);
    assert_eq!(devices_with(&s, &[("host", "evil.example:8787")]).await, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn the_code_free_access_lets_in_the_home_network_but_not_another_site() {
    let s = state().with_open_access(true);
    // la pagina di Aurora aperta dal telefono: indirizzo IP, nome del PC, nome locale
    assert_eq!(devices_with(&s, &[("host", "192.168.1.10:8787"), ("origin", "http://192.168.1.10:8787")]).await, StatusCode::OK);
    assert_eq!(devices_with(&s, &[("host", "stazione:8787")]).await, StatusCode::OK);
    assert_eq!(devices_with(&s, &[("host", "stazione.local:8787"), ("origin", "http://stazione.local:8787")]).await, StatusCode::OK);
    // un altro sito: Origin diverso, oppure il suo nome che punta ad Aurora (DNS rebinding)
    assert_eq!(devices_with(&s, &[("host", "192.168.1.10:8787"), ("origin", "https://example.com")]).await, StatusCode::UNAUTHORIZED);
    assert_eq!(devices_with(&s, &[("host", "evil.example:8787"), ("origin", "http://evil.example:8787")]).await, StatusCode::UNAUTHORIZED);
    assert_eq!(devices_with(&s, &[("host", "evil.example:8787")]).await, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn a_proxied_request_is_never_the_pc() {
    let s = state().with_local_trust(true);
    assert_eq!(devices(&s, "127.0.0.1", true).await, StatusCode::UNAUTHORIZED, "un tunnel arriva da loopback ma non e' il PC");
}
