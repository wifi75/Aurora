mod common;

use aurora_core::Command;
use axum::http::{Method, StatusCode};
use common::*;

async fn post(s: &remote_api::AppState, path: &str, body: &str, cloudflare: bool) -> axum::response::Response {
    let cookie = cookie_for(s);
    let mut r = req(Method::POST, path).cookie(&cookie).json(body);
    if cloudflare {
        r = r.header("cf-ray", "abc");
    }
    call(&router(s), r).await
}

#[tokio::test]
async fn free_text_that_transmits_is_forwarded_uppercase() {
    let s = state();
    let mut rx = s.bus.take_command_rx().unwrap();
    let resp = post(&s, "/api/v1/commands/free-text", r#"{"text":"tnx 73","send":true}"#, false).await;
    status_is(&resp, StatusCode::OK);
    assert_eq!(rx.recv().await, Some(Command::SendFreeText { text: "TNX 73".into(), send: true }));
}

#[tokio::test]
async fn free_text_is_validated() {
    let s = state();
    let mut rx = s.bus.take_command_rx().unwrap();
    for bad in [r#"{"text":"","send":false}"#, r#"{"text":"QUESTO E' TROPPO LUNGO","send":false}"#, r#"{"text":"CIAO <>","send":false}"#] {
        let resp = post(&s, "/api/v1/commands/free-text", bad, false).await;
        status_is(&resp, StatusCode::BAD_REQUEST);
        assert_eq!(body_json(resp).await["error"], "bad_text", "{bad}");
    }
    assert!(rx.try_recv().is_err());
}

#[tokio::test]
async fn transmitting_commands_need_live_tx_and_an_allowed_transport() {
    let closed = state_with_live_tx(false);
    let mut rx = closed.bus.take_command_rx().unwrap();
    for (path, body) in [("/api/v1/commands/free-text", r#"{"text":"73","send":true}"#), ("/api/v1/commands/cq", r#"{}"#)] {
        let resp = post(&closed, path, body, false).await;
        status_is(&resp, StatusCode::FORBIDDEN);
        assert_eq!(body_json(resp).await["error"], "tx_disabled");
    }
    assert!(rx.try_recv().is_err());

    let open = state();
    let mut rx = open.bus.take_command_rx().unwrap();
    for (path, body) in [("/api/v1/commands/free-text", r#"{"text":"73","send":true}"#), ("/api/v1/commands/cq", r#"{}"#)] {
        let resp = post(&open, path, body, true).await;
        status_is(&resp, StatusCode::FORBIDDEN);
        assert_eq!(body_json(resp).await["error"], "tx_not_allowed_on_transport");
    }
    assert!(rx.try_recv().is_err());
}

#[tokio::test]
async fn preparing_a_message_without_sending_does_not_need_live_tx() {
    let s = state_with_live_tx(false);
    let mut rx = s.bus.take_command_rx().unwrap();
    let resp = post(&s, "/api/v1/commands/free-text", r#"{"text":"73","send":false}"#, false).await;
    status_is(&resp, StatusCode::OK);
    assert_eq!(rx.recv().await, Some(Command::SendFreeText { text: "73".into(), send: false }));
}

#[tokio::test]
async fn cq_is_forwarded_with_an_optional_modifier() {
    let s = state();
    let mut rx = s.bus.take_command_rx().unwrap();
    status_is(&post(&s, "/api/v1/commands/cq", r#"{}"#, false).await, StatusCode::OK);
    status_is(&post(&s, "/api/v1/commands/cq", r#"{"modifier":"dx"}"#, false).await, StatusCode::OK);
    assert_eq!(rx.recv().await, Some(Command::CallCq { modifier: String::new() }));
    assert_eq!(rx.recv().await, Some(Command::CallCq { modifier: "DX".into() }));
    let bad = post(&s, "/api/v1/commands/cq", r#"{"modifier":"TROPPOLUNGO"}"#, false).await;
    status_is(&bad, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn tx_frequency_is_allowed_even_when_tx_is_closed_but_limited_to_the_audio_band() {
    let s = state_with_live_tx(false);
    let mut rx = s.bus.take_command_rx().unwrap();
    status_is(&post(&s, "/api/v1/commands/tx-frequency", r#"{"hz":1500}"#, false).await, StatusCode::OK);
    assert_eq!(rx.recv().await, Some(Command::SetTxFrequency { hz: 1500 }));
    for hz in [0, 100, 3500, 99999] {
        let resp = post(&s, "/api/v1/commands/tx-frequency", &format!(r#"{{"hz":{hz}}}"#), false).await;
        status_is(&resp, StatusCode::BAD_REQUEST);
    }
}

#[tokio::test]
async fn tx_commands_need_a_paired_device() {
    let s = state();
    for path in ["/api/v1/commands/free-text", "/api/v1/commands/cq", "/api/v1/commands/tx-frequency"] {
        let resp = call(&router(&s), req(Method::POST, path).json("{}")).await;
        status_is(&resp, StatusCode::UNAUTHORIZED);
    }
}
