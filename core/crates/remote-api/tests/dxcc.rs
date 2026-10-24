mod common;

use std::sync::Arc;

use axum::http::{Method, StatusCode};
use common::*;
use remote_api::AppState;

async fn get(s: &AppState, call: &str) -> axum::response::Response {
    let cookie = cookie_for(s);
    call_get(s, call, &cookie).await
}

async fn call_get(s: &AppState, call: &str, cookie: &str) -> axum::response::Response {
    call_router(s, req(Method::GET, &format!("/api/v1/dxcc/{call}")).cookie(cookie)).await
}

async fn call_router(s: &AppState, r: Req) -> axum::response::Response {
    call(&router(s), r).await
}

fn with_lookup() -> AppState {
    state().with_dxcc_if(Some(Arc::new(|call: &str| (call == "DO1ID").then(|| serde_json::json!({ "country": "Fed. Rep. of Germany", "continent": "EU" })))))
}

#[tokio::test]
async fn needs_a_paired_device() {
    let s = with_lookup();
    status_is(&call(&router(&s), req(Method::GET, "/api/v1/dxcc/DO1ID")).await, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn gives_the_country_of_a_callsign() {
    let s = with_lookup();
    let resp = get(&s, "DO1ID").await;
    status_is(&resp, StatusCode::OK);
    assert_eq!(body_json(resp).await["country"], "Fed. Rep. of Germany");
}

#[tokio::test]
async fn unknown_callsigns_bad_text_and_a_missing_file_are_told_apart() {
    let s = with_lookup();
    let unknown = get(&s, "ZZ9ZZ").await;
    status_is(&unknown, StatusCode::NOT_FOUND);
    assert_eq!(body_json(unknown).await["error"], "dxcc_unknown");
    let bad = get(&s, "A%20B").await;
    status_is(&bad, StatusCode::BAD_REQUEST);
    assert_eq!(body_json(bad).await["error"], "bad_call");
    status_is(&get(&state(), "DO1ID").await, StatusCode::NOT_FOUND);
}
