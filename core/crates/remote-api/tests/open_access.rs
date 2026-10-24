mod common;

use axum::{extract::ConnectInfo, http::{Method, StatusCode}};
use common::*;
use remote_api::AppState;
use std::net::SocketAddr;

fn from(ip: &str, r: Req) -> Req {
    Req { peer: ConnectInfo(format!("{ip}:50000").parse::<SocketAddr>().unwrap()), ..r }
}

async fn turn(s: &AppState, open: bool, ip: &str) -> axum::response::Response {
    let body = format!(r#"{{"open":{open}}}"#);
    call(&router(s), from(ip, req(Method::PUT, "/api/v1/access").json(&body))).await
}

async fn devices(s: &AppState, ip: &str, proxied: bool) -> StatusCode {
    let mut r = from(ip, req(Method::GET, "/api/v1/devices"));
    if proxied {
        r = r.header("x-forwarded-for", "1.2.3.4");
    }
    call(&router(s), r).await.status()
}

#[tokio::test]
async fn it_is_off_by_default_and_the_state_is_public() {
    let s = state();
    assert_eq!(devices(&s, "192.168.1.50", false).await, StatusCode::UNAUTHORIZED);
    let j = body_json(call(&router(&s), req(Method::GET, "/api/v1/access")).await).await;
    assert_eq!(j["open"], false);
}

#[tokio::test]
async fn only_the_pc_can_turn_it_on_and_without_a_code() {
    let s = state();
    let phone = turn(&s, true, "192.168.1.50").await;
    status_is(&phone, StatusCode::FORBIDDEN);
    assert_eq!(body_json(phone).await["error"], "local_only");
    assert_eq!(devices(&s, "192.168.1.50", false).await, StatusCode::UNAUTHORIZED);
    status_is(&turn(&s, true, "127.0.0.1").await, StatusCode::OK);
}

#[tokio::test]
async fn through_a_proxy_even_the_pc_request_is_refused() {
    let s = state();
    let resp = call(&router(&s), from("127.0.0.1", req(Method::PUT, "/api/v1/access").json(r#"{"open":true}"#).header("cf-ray", "x"))).await;
    status_is(&resp, StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn when_on_devices_of_the_home_network_get_in_without_pairing() {
    let s = state();
    turn(&s, true, "127.0.0.1").await;
    for ip in ["192.168.1.50", "10.0.0.7", "172.16.5.5", "127.0.0.1", "169.254.1.2"] {
        assert_eq!(devices(&s, ip, false).await, StatusCode::OK, "{ip}");
    }
    let j = body_json(call(&router(&s), req(Method::GET, "/api/v1/access")).await).await;
    assert_eq!(j["open"], true);
}

#[tokio::test]
async fn public_addresses_and_proxied_requests_still_need_a_code() {
    let s = state();
    turn(&s, true, "127.0.0.1").await;
    assert_eq!(devices(&s, "8.8.8.8", false).await, StatusCode::UNAUTHORIZED);
    assert_eq!(devices(&s, "192.168.1.50", true).await, StatusCode::UNAUTHORIZED, "tunnel e proxy passano dal PC ma non sono la rete di casa");
}

#[tokio::test]
async fn turning_it_off_closes_the_door_again() {
    let s = state();
    turn(&s, true, "127.0.0.1").await;
    assert_eq!(devices(&s, "192.168.1.50", false).await, StatusCode::OK);
    status_is(&turn(&s, false, "127.0.0.1").await, StatusCode::OK);
    assert_eq!(devices(&s, "192.168.1.50", false).await, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn it_never_survives_a_restart() {
    let s = state();
    turn(&s, true, "127.0.0.1").await;
    let again = state();
    assert_eq!(devices(&again, "192.168.1.50", false).await, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn pairing_keeps_working_while_open() {
    let s = state();
    turn(&s, true, "127.0.0.1").await;
    let resp = call(&router(&s), from("127.0.0.1", req(Method::POST, "/api/v1/pair/start"))).await;
    status_is(&resp, StatusCode::OK);
}
