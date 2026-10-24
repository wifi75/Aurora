mod common;

use axum::http::{Method, StatusCode};
use common::*;
use std::sync::Arc;

#[tokio::test]
async fn it_needs_a_paired_device() {
    let s = state();
    let resp = call(&router(&s), req(Method::GET, "/api/v1/engine/native")).await;
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn it_says_it_is_off_until_the_engine_registers_itself() {
    let s = state().with_local_trust(true);
    let j = body_json(call(&router(&s), req(Method::GET, "/api/v1/engine/native").via_loopback()).await).await;
    assert_eq!(j["enabled"], false);
}

#[tokio::test]
async fn it_reports_what_the_engine_tells() {
    let s = state().with_local_trust(true);
    s.set_native_status(Arc::new(|| serde_json::json!({ "enabled": true, "mode": "shadow", "comparison": { "parity": 0.9 } })));
    let j = body_json(call(&router(&s), req(Method::GET, "/api/v1/engine/native").via_loopback()).await).await;
    assert_eq!(j["enabled"], true);
    assert_eq!(j["comparison"]["parity"], 0.9);
}
