mod common;

use aurora_core::{Command, DecodedMessage, Event};
use axum::http::{header, Method, StatusCode};
use common::*;

fn decode(id: u64) -> Event {
    Event::Decode(DecodedMessage {
        id,
        time_ms: 0,
        snr: -10,
        delta_time: 0.1,
        freq_hz: 1500,
        text: "CQ DX JA1XYZ PM95".into(),
        tag: None,
    })
}

#[tokio::test]
async fn health_is_public_and_has_author() {
    let s = state();
    let resp = call(&router(&s), req(Method::GET, "/api/v1/health")).await;
    status_is(&resp, StatusCode::OK);
    let j = body_json(resp).await;
    assert_eq!(j["status"], "ok");
    assert_eq!(j["name"], "Aurora");
    assert_eq!(j["author"], "Tiziano Cassone");
    assert!(j["version"].as_str().is_some_and(|v| !v.is_empty()));
}

#[tokio::test]
async fn protected_route_without_cookie_is_401() {
    let s = state();
    let resp = call(&router(&s), req(Method::GET, "/api/v1/devices")).await;
    status_is(&resp, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn unknown_cookie_is_401() {
    let s = state();
    let bad = format!("aurora_session={}", "a".repeat(64));
    let resp = call(&router(&s), req(Method::GET, "/api/v1/devices").cookie(&bad)).await;
    status_is(&resp, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn pair_flow_sets_httponly_samesite_cookie() {
    let s = state();
    let app = router(&s);
    let start = call(&app, req(Method::POST, "/api/v1/pair/start").from_loopback()).await;
    status_is(&start, StatusCode::OK);
    let j = body_json(start).await;
    let code = j["code"].as_str().unwrap().to_string();
    assert_eq!(code.len(), 8);
    assert_eq!(j["expires_in"], 120);
    assert!(j["qr_svg"].as_str().unwrap().contains("<svg"));

    let body = format!(r#"{{"code":"{code}","device_name":"iPhone"}}"#);
    let fin = call(&app, req(Method::POST, "/api/v1/pair/finish").json(&body)).await;
    status_is(&fin, StatusCode::NO_CONTENT);
    let set_cookie = fin.headers().get(header::SET_COOKIE).unwrap().to_str().unwrap().to_string();
    assert!(set_cookie.starts_with("aurora_session="));
    assert!(set_cookie.contains("HttpOnly"));
    assert!(set_cookie.contains("SameSite=Strict"));
    assert!(!set_cookie.contains("Secure"), "Secure solo se richiesto dalla configurazione");

    let cookie = set_cookie.split(';').next().unwrap().to_string();
    let list = call(&app, req(Method::GET, "/api/v1/devices").cookie(&cookie)).await;
    status_is(&list, StatusCode::OK);
    let devices = body_json(list).await;
    assert_eq!(devices[0]["name"], "iPhone");
    assert!(!devices.to_string().contains(&cookie[15..]));
}

#[tokio::test]
async fn pair_start_refuses_non_loopback() {
    let s = state();
    let resp = call(&router(&s), req(Method::POST, "/api/v1/pair/start")).await;
    status_is(&resp, StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn locked_pairing_returns_429_with_retry_after() {
    let s = state();
    let app = router(&s);
    call(&app, req(Method::POST, "/api/v1/pair/start").from_loopback()).await;
    let wrong = r#"{"code":"00000000","device_name":"x"}"#;
    let mut last = None;
    for _ in 0..6 {
        last = Some(call(&app, req(Method::POST, "/api/v1/pair/finish").json(wrong)).await);
    }
    let resp = last.unwrap();
    status_is(&resp, StatusCode::TOO_MANY_REQUESTS);
    let retry: u64 = resp.headers().get(header::RETRY_AFTER).unwrap().to_str().unwrap().parse().unwrap();
    assert!((1..=60).contains(&retry));
}

#[tokio::test]
async fn bad_code_and_bad_name_errors_are_clear() {
    let s = state();
    let app = router(&s);
    let resp = call(&app, req(Method::POST, "/api/v1/pair/finish").json(r#"{"code":"12345678","device_name":"x"}"#)).await;
    status_is(&resp, StatusCode::BAD_REQUEST);
    assert_eq!(body_json(resp).await["error"], "bad_code");
    let resp = call(&app, req(Method::POST, "/api/v1/pair/finish").json(r#"{"code":"12345678","device_name":"  "}"#)).await;
    status_is(&resp, StatusCode::BAD_REQUEST);
    assert_eq!(body_json(resp).await["error"], "bad_name");
}

#[tokio::test]
async fn halt_tx_is_idempotent_200_and_forwards_command() {
    let s = state();
    let mut rx = s.bus.take_command_rx().unwrap();
    let app = router(&s);
    let cookie = cookie_for(&s);
    for _ in 0..2 {
        let resp = call(&app, req(Method::POST, "/api/v1/commands/halt-tx").cookie(&cookie)).await;
        status_is(&resp, StatusCode::OK);
    }
    assert_eq!(rx.recv().await, Some(Command::HaltTx));
    assert_eq!(rx.recv().await, Some(Command::HaltTx));
}

#[tokio::test]
async fn halt_tx_without_cookie_is_401() {
    let s = state();
    let resp = call(&router(&s), req(Method::POST, "/api/v1/commands/halt-tx")).await;
    status_is(&resp, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn halt_tx_is_never_blocked_by_transport_policy() {
    let s = state();
    let cookie = cookie_for(&s);
    let resp = call(&router(&s), req(Method::POST, "/api/v1/commands/halt-tx").cookie(&cookie).header("cf-ray", "abc")).await;
    status_is(&resp, StatusCode::OK);
}

#[tokio::test]
async fn reply_unknown_decode_id_is_404() {
    let s = state();
    let cookie = cookie_for(&s);
    let resp = call(&router(&s), req(Method::POST, "/api/v1/commands/reply").cookie(&cookie).json(r#"{"decode_id":424242}"#)).await;
    status_is(&resp, StatusCode::NOT_FOUND);
    assert_eq!(body_json(resp).await["error"], "unknown_decode");
}

#[tokio::test]
async fn reply_known_decode_id_is_forwarded() {
    let s = state();
    let mut rx = s.bus.take_command_rx().unwrap();
    s.bus.events().send(decode(77)).unwrap();
    wait_known(&s, 77).await;
    let cookie = cookie_for(&s);
    let resp = call(&router(&s), req(Method::POST, "/api/v1/commands/reply").cookie(&cookie).json(r#"{"decode_id":77}"#)).await;
    status_is(&resp, StatusCode::OK);
    assert_eq!(rx.recv().await, Some(Command::Reply { decode_id: 77 }));
}

#[tokio::test]
async fn tx_command_denied_by_policy_on_cloudflare_is_403() {
    let s = state();
    s.bus.events().send(decode(5)).unwrap();
    wait_known(&s, 5).await;
    let cookie = cookie_for(&s);
    let resp = call(&router(&s), req(Method::POST, "/api/v1/commands/reply").cookie(&cookie).json(r#"{"decode_id":5}"#).header("cf-ray", "abc")).await;
    status_is(&resp, StatusCode::FORBIDDEN);
    assert_eq!(body_json(resp).await["error"], "tx_not_allowed_on_transport");
}

#[tokio::test]
async fn revoked_device_gets_401() {
    let s = state();
    let token = token_for(&s);
    let cookie = format!("aurora_session={token}");
    let id = s.auth.verify(&token).unwrap();
    let app = router(&s);
    status_is(&call(&app, req(Method::GET, "/api/v1/devices").cookie(&cookie)).await, StatusCode::OK);
    let resp = call(&app, req(Method::DELETE, &format!("/api/v1/devices/{}", id.0)).cookie(&cookie)).await;
    status_is(&resp, StatusCode::NO_CONTENT);
    status_is(&call(&app, req(Method::GET, "/api/v1/devices").cookie(&cookie)).await, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn openapi_lists_health_and_stream() {
    let s = state();
    let resp = call(&router(&s), req(Method::GET, "/api/v1/openapi.json")).await;
    status_is(&resp, StatusCode::OK);
    let j = body_json(resp).await;
    assert!(j["paths"]["/api/v1/health"].is_object());
    assert!(j["paths"]["/api/v1/stream"].is_object());
}
