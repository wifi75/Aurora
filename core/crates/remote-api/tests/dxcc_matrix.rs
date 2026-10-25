mod common;

use std::sync::{Arc, Mutex};

use axum::http::{Method, StatusCode};
use common::*;
use remote_api::MatrixQuery;
use serde_json::json;

#[tokio::test]
async fn the_matrix_needs_a_paired_device() {
    let s = state();
    status_is(&call(&router(&s), req(Method::GET, "/api/v1/dxcc-matrix")).await, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn the_matrix_says_when_it_is_not_available() {
    let s = state();
    let cookie = cookie_for(&s);
    let resp = call(&router(&s), req(Method::GET, "/api/v1/dxcc-matrix").cookie(&cookie)).await;
    status_is(&resp, StatusCode::NOT_FOUND);
    assert_eq!(body_json(resp).await["error"], "dxcc_matrix_unavailable");
}

#[tokio::test]
async fn the_query_reaches_the_provider_trimmed_and_the_answer_comes_back() {
    let seen: Arc<Mutex<Vec<MatrixQuery>>> = Arc::default();
    let log = Arc::clone(&seen);
    let s = state().with_dxcc_matrix_if(Some(Arc::new(move |q| {
        log.lock().unwrap().push(q);
        Ok(json!({ "worked": 3, "subject": null }))
    })));
    let cookie = cookie_for(&s);
    let app = router(&s);
    let resp = call(&app, req(Method::GET, "/api/v1/dxcc-matrix?call=%20K1ABC&grid=FN42&band=20m&mode=FT8&compact=1").cookie(&cookie)).await;
    status_is(&resp, StatusCode::OK);
    assert_eq!(body_json(resp).await["worked"], 3);
    status_is(&call(&app, req(Method::GET, "/api/v1/dxcc-matrix?call=&band=%20").cookie(&cookie)).await, StatusCode::OK);
    let got = seen.lock().unwrap();
    assert_eq!(got[0], MatrixQuery { call: Some("K1ABC".into()), grid: Some("FN42".into()), band: Some("20m".into()), mode: Some("FT8".into()), compact: true });
    assert_eq!(got[1], MatrixQuery::default(), "i valori vuoti valgono come assenti");
}

#[tokio::test]
async fn an_unreadable_log_is_a_bad_gateway() {
    let s = state().with_dxcc_matrix_if(Some(Arc::new(|_| Err("file bloccato".to_string()))));
    let cookie = cookie_for(&s);
    let resp = call(&router(&s), req(Method::GET, "/api/v1/dxcc-matrix").cookie(&cookie)).await;
    status_is(&resp, StatusCode::BAD_GATEWAY);
    assert_eq!(body_json(resp).await["error"], "dxcc_matrix_read");
}
