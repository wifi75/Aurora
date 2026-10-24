mod common;

use std::sync::Arc;

use axum::http::{Method, StatusCode};
use common::*;
use remote_api::AppState;

async fn get(s: &AppState) -> axum::response::Response {
    let cookie = cookie_for(s);
    call(&router(s), req(Method::GET, "/api/v1/jtdx-config").cookie(&cookie)).await
}

#[tokio::test]
async fn needs_a_paired_device() {
    let s = state();
    status_is(&call(&router(&s), req(Method::GET, "/api/v1/jtdx-config")).await, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn returns_what_the_reader_gives() {
    let s = state().with_jtdx_config_if(Some(Arc::new(|| Ok(serde_json::json!({ "UDPServerPort": "2334" })))));
    let resp = get(&s).await;
    status_is(&resp, StatusCode::OK);
    assert_eq!(body_json(resp).await["UDPServerPort"], "2334");
}

#[tokio::test]
async fn says_so_when_not_configured_or_unreadable() {
    let resp = get(&state()).await;
    status_is(&resp, StatusCode::NOT_FOUND);
    assert_eq!(body_json(resp).await["error"], "jtdx_config_unavailable");
    let s = state().with_jtdx_config_if(Some(Arc::new(|| Err("rotto".to_string()))));
    let resp = get(&s).await;
    status_is(&resp, StatusCode::BAD_GATEWAY);
    assert_eq!(body_json(resp).await["error"], "jtdx_config_read");
}
