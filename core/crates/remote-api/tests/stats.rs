mod common;

use std::sync::Arc;

use axum::http::{Method, StatusCode};
use common::*;
use serde_json::json;

#[tokio::test]
async fn stats_need_a_paired_device() {
    let s = state();
    status_is(&call(&router(&s), req(Method::GET, "/api/v1/stats")).await, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn stats_say_when_the_log_is_not_available() {
    let s = state();
    let cookie = cookie_for(&s);
    let resp = call(&router(&s), req(Method::GET, "/api/v1/stats").cookie(&cookie)).await;
    status_is(&resp, StatusCode::NOT_FOUND);
    assert_eq!(body_json(resp).await["error"], "stats_unavailable");
}

#[tokio::test]
async fn stats_return_what_the_provider_computes_each_time() {
    let counter = Arc::new(std::sync::atomic::AtomicU32::new(0));
    let c = Arc::clone(&counter);
    let s = state().with_stats(Arc::new(move || json!({ "calls": c.fetch_add(1, std::sync::atomic::Ordering::SeqCst) })));
    let cookie = cookie_for(&s);
    let app = router(&s);
    let first = body_json(call(&app, req(Method::GET, "/api/v1/stats").cookie(&cookie)).await).await;
    let second = body_json(call(&app, req(Method::GET, "/api/v1/stats").cookie(&cookie)).await).await;
    assert_eq!(first["calls"], 0);
    assert_eq!(second["calls"], 1);
}
