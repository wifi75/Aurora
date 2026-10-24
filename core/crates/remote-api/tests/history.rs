mod common;

use std::sync::Arc;

use axum::http::{Method, StatusCode};
use common::*;
use decode_history::{Entry, HistoryStore};
use remote_api::AppState;
use serde_json::json;

fn entry(ts_ms: u64, text: &str, call: &str, band: &str) -> Entry {
    Entry {
        ts_ms,
        band: Some(band.into()),
        mode: "FT8".into(),
        dial_hz: 14_074_000,
        freq_hz: 1500,
        snr: -10,
        dt: 0.1,
        text: text.into(),
        call: Some(call.into()),
        country: Some("Italy".into()),
        continent: Some("EU".into()),
        tag: Some("cq".into()),
    }
}

fn with_store() -> AppState {
    let store = Arc::new(HistoryStore::open_in_memory().unwrap());
    store
        .insert_many(&[entry(1_000, "CQ K1ABC FN42", "K1ABC", "20m"), entry(2_000, "CQ JA1XYZ PM95", "JA1XYZ", "40m"), entry(3_000, "K1ABC IU3ABC -12", "IU3ABC", "20m")])
        .unwrap();
    state().with_history(store)
}

#[tokio::test]
async fn needs_a_paired_device_and_says_when_there_is_no_history() {
    let s = state();
    for (method, uri) in [(Method::GET, "/api/v1/history"), (Method::GET, "/api/v1/history/stats"), (Method::DELETE, "/api/v1/history")] {
        status_is(&call(&router(&s), req(method, uri)).await, StatusCode::UNAUTHORIZED);
    }
    let cookie = cookie_for(&s);
    let resp = call(&router(&s), req(Method::GET, "/api/v1/history").cookie(&cookie)).await;
    status_is(&resp, StatusCode::NOT_FOUND);
    assert_eq!(body_json(resp).await["error"], "history_unavailable");
}

#[tokio::test]
async fn lists_the_messages_most_recent_first_and_filters_them() {
    let s = with_store();
    let cookie = cookie_for(&s);
    let app = router(&s);
    let all = body_json(call(&app, req(Method::GET, "/api/v1/history").cookie(&cookie)).await).await;
    assert_eq!(all["total"], 3);
    assert_eq!(all["rows"][0]["text"], "K1ABC IU3ABC -12");
    assert_eq!(all["rows"][0]["band"], "20m");
    let by_call = body_json(call(&app, req(Method::GET, "/api/v1/history?q=k1abc&band=20m").cookie(&cookie)).await).await;
    assert_eq!(by_call["total"], 2);
    let paged = body_json(call(&app, req(Method::GET, "/api/v1/history?limit=1&offset=1").cookie(&cookie)).await).await;
    assert_eq!((paged["total"].as_u64(), paged["rows"].as_array().map(Vec::len)), (Some(3), Some(1)));
    assert_eq!(paged["rows"][0]["call"], "JA1XYZ");
}

#[tokio::test]
async fn reports_the_stats_and_clears_only_when_asked() {
    let s = with_store();
    let cookie = cookie_for(&s);
    let app = router(&s);
    let stats = body_json(call(&app, req(Method::GET, "/api/v1/history/stats").cookie(&cookie)).await).await;
    assert_eq!(stats, json!({ "total": 3, "oldest_ms": 1000, "calls": 3 }));
    let cleared = body_json(call(&app, req(Method::DELETE, "/api/v1/history").cookie(&cookie)).await).await;
    assert_eq!(cleared["removed"], 3);
    assert_eq!(body_json(call(&app, req(Method::GET, "/api/v1/history").cookie(&cookie)).await).await["total"], 0);
}
