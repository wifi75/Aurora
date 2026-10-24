mod common;

use std::sync::Arc;

use auth::{DeviceStore, SystemClock};
use aurora_core::Bus;
use axum::http::{Method, StatusCode};
use common::*;
use logbook::Logbook;
use remote_api::{build_router, AppState, TxPolicy};
use serde_json::json;

#[tokio::test]
async fn station_info_needs_a_paired_device() {
    let s = state();
    let resp = call(&router(&s), req(Method::GET, "/api/v1/station-info")).await;
    status_is(&resp, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn station_info_returns_what_the_program_published() {
    let bus = Bus::new(16);
    let auth = Arc::new(DeviceStore::open_in_memory(Arc::new(SystemClock)).unwrap());
    let book = Arc::new(Logbook::open_in_memory().unwrap());
    let s = AppState::new(bus, auth, book, TxPolicy::default_safe(), false)
        .with_station_info(json!({ "tx_allowed": false, "export_udp": ["127.0.0.1:2239"] }));
    let cookie = cookie_for(&s);
    let resp = call(&build_router(s), req(Method::GET, "/api/v1/station-info").cookie(&cookie)).await;
    status_is(&resp, StatusCode::OK);
    let j = body_json(resp).await;
    assert_eq!(j["tx_allowed"], false);
    assert_eq!(j["export_udp"][0], "127.0.0.1:2239");
}

#[tokio::test]
async fn station_info_is_empty_object_when_nothing_was_published() {
    let s = state();
    let cookie = cookie_for(&s);
    let resp = call(&router(&s), req(Method::GET, "/api/v1/station-info").cookie(&cookie)).await;
    status_is(&resp, StatusCode::OK);
    assert_eq!(body_json(resp).await, json!({}));
}
