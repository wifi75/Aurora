mod common;

use std::sync::{
    atomic::{AtomicU32, Ordering},
    Arc,
};

use axum::http::{Method, StatusCode};
use common::*;
use remote_api::AppState;

fn saver_state(saved: Arc<AtomicU32>) -> AppState {
    state().with_settings_saver(Arc::new(move || {
        saved.fetch_add(1, Ordering::SeqCst);
    }))
}

async fn get(s: &AppState) -> serde_json::Value {
    let cookie = cookie_for(s);
    body_json(call(&router(s), req(Method::GET, "/api/v1/pairing-settings").cookie(&cookie)).await).await
}

async fn put(s: &AppState, body: &str, local: bool, proxied: bool) -> axum::response::Response {
    let cookie = cookie_for(s);
    let mut r = req(Method::PUT, "/api/v1/pairing-settings").cookie(&cookie).json(body);
    if local {
        r = r.via_loopback();
    }
    if proxied {
        r = r.header("x-forwarded-for", "1.2.3.4");
    }
    call(&router(s), r).await
}

#[tokio::test]
async fn needs_a_paired_device() {
    let s = state();
    status_is(&call(&router(&s), req(Method::GET, "/api/v1/pairing-settings")).await, StatusCode::UNAUTHORIZED);
    status_is(&call(&router(&s), req(Method::PUT, "/api/v1/pairing-settings").json("{}")).await, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn shows_the_current_lifetime_with_its_limits() {
    let s = state();
    let j = get(&s).await;
    assert_eq!(j["code_ttl_secs"], 600);
    assert_eq!(j["min_secs"], 60);
    assert_eq!(j["max_secs"], 3600);
}

#[tokio::test]
async fn the_operator_changes_it_from_the_pc_and_the_next_code_follows() {
    let saved = Arc::new(AtomicU32::new(0));
    let s = saver_state(saved.clone());
    status_is(&put(&s, r#"{"code_ttl_secs":900}"#, true, false).await, StatusCode::OK);
    assert_eq!(get(&s).await["code_ttl_secs"], 900);
    assert_eq!(s.auth.start_pairing().unwrap().expires_in_secs, 900);
    assert_eq!(saved.load(Ordering::SeqCst), 1, "la scelta si salva");
}

#[tokio::test]
async fn it_cannot_be_changed_from_a_phone_or_through_a_proxy() {
    let saved = Arc::new(AtomicU32::new(0));
    let s = saver_state(saved.clone());
    let phone = put(&s, r#"{"code_ttl_secs":900}"#, false, false).await;
    status_is(&phone, StatusCode::FORBIDDEN);
    assert_eq!(body_json(phone).await["error"], "local_only");
    status_is(&put(&s, r#"{"code_ttl_secs":900}"#, true, true).await, StatusCode::FORBIDDEN);
    assert_eq!(get(&s).await["code_ttl_secs"], 600);
    assert_eq!(saved.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn values_outside_the_limits_are_refused() {
    let saved = Arc::new(AtomicU32::new(0));
    let s = saver_state(saved.clone());
    for bad in [10, 59, 3601, 100_000] {
        let resp = put(&s, &format!(r#"{{"code_ttl_secs":{bad}}}"#), true, false).await;
        status_is(&resp, StatusCode::BAD_REQUEST);
        assert_eq!(body_json(resp).await["error"], "bad_ttl", "{bad}");
    }
    assert_eq!(get(&s).await["code_ttl_secs"], 600);
    assert_eq!(saved.load(Ordering::SeqCst), 0);
}
