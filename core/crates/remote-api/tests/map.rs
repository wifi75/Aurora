mod common;

use std::sync::Arc;

use axum::http::{Method, StatusCode};
use common::*;
use serde_json::json;

#[tokio::test]
async fn the_map_needs_a_paired_device() {
    let s = state();
    status_is(&call(&router(&s), req(Method::GET, "/api/v1/map/worked")).await, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn the_map_says_when_the_log_is_not_configured() {
    let s = state();
    let cookie = cookie_for(&s);
    let resp = call(&router(&s), req(Method::GET, "/api/v1/map/worked").cookie(&cookie)).await;
    status_is(&resp, StatusCode::NOT_FOUND);
    assert_eq!(body_json(resp).await["error"], "map_unavailable");
}

#[tokio::test]
async fn the_map_returns_the_worked_squares_the_provider_reads() {
    let s = state().with_worked_map_if(Some(Arc::new(|| Ok(json!({ "squares": [{ "grid": "JN45", "count": 3, "bands": ["20m", "15m"] }] })))));
    let cookie = cookie_for(&s);
    let resp = call(&router(&s), req(Method::GET, "/api/v1/map/worked").cookie(&cookie)).await;
    status_is(&resp, StatusCode::OK);
    let v = body_json(resp).await;
    assert_eq!(v["squares"][0]["grid"], "JN45");
    assert_eq!(v["squares"][0]["count"], 3);
}

#[tokio::test]
async fn an_unreadable_log_is_a_bad_gateway() {
    let s = state().with_worked_map_if(Some(Arc::new(|| Err("file bloccato".to_string()))));
    let cookie = cookie_for(&s);
    status_is(&call(&router(&s), req(Method::GET, "/api/v1/map/worked").cookie(&cookie)).await, StatusCode::BAD_GATEWAY);
}
