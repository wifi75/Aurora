mod common;

use axum::http::{Method, StatusCode};
use common::*;
use remote_api::AppState;
use serde_json::json;

fn with_service(name: &str) -> (AppState, std::path::PathBuf) {
    let dir = std::env::temp_dir().join(format!("aurora-api-wish-{}-{name}", std::process::id()));
    std::fs::remove_dir_all(&dir).ok();
    std::fs::create_dir_all(&dir).unwrap();
    let service = wishlist::WishlistService::open(&dir.join("wishlist.json"));
    (state().with_wishlist(service), dir)
}

#[tokio::test]
async fn the_wishlist_needs_a_paired_device_and_says_when_it_is_not_there() {
    let s = state();
    for (method, uri) in [(Method::GET, "/api/v1/wishlist"), (Method::PUT, "/api/v1/wishlist"), (Method::POST, "/api/v1/wishlist/test")] {
        status_is(&call(&router(&s), req(method, uri)).await, StatusCode::UNAUTHORIZED);
    }
    let cookie = cookie_for(&s);
    let resp = call(&router(&s), req(Method::GET, "/api/v1/wishlist").cookie(&cookie)).await;
    status_is(&resp, StatusCode::NOT_FOUND);
    assert_eq!(body_json(resp).await["error"], "wishlist_unavailable");
}

#[tokio::test]
async fn starts_off_and_empty_then_saves_a_list_that_comes_back_normalized() {
    let (s, dir) = with_service("save");
    let cookie = cookie_for(&s);
    let app = router(&s);
    let first = body_json(call(&app, req(Method::GET, "/api/v1/wishlist").cookie(&cookie)).await).await;
    assert_eq!(first["config"]["enabled"], false);
    assert_eq!(first["config"]["calls"], json!([]));
    assert_eq!(first["config"]["cooldown_min"], 15);
    assert_eq!(first["recent"], json!([]));

    let body = json!({ "enabled": true, "calls": [" k1abc ", "K1ABC", "3y0j"], "grids": ["jn45"], "countries": ["Bulgaria"], "cooldown_min": 30 }).to_string();
    let put = call(&app, req(Method::PUT, "/api/v1/wishlist").cookie(&cookie).json(&body)).await;
    status_is(&put, StatusCode::OK);
    assert_eq!(body_json(put).await["config"]["calls"], json!(["K1ABC", "3Y0J"]));
    let again = body_json(call(&app, req(Method::GET, "/api/v1/wishlist").cookie(&cookie)).await).await;
    assert_eq!(again["config"]["enabled"], true);
    assert_eq!(again["config"]["grids"], json!(["JN45"]));
    assert_eq!(again["config"]["cooldown_min"], 30);
    std::fs::remove_dir_all(&dir).ok();
}

#[tokio::test]
async fn refuses_a_list_that_cannot_work_and_keeps_the_old_one() {
    let (s, dir) = with_service("bad");
    let cookie = cookie_for(&s);
    let app = router(&s);
    let good = json!({ "enabled": true, "calls": ["K1ABC"] }).to_string();
    status_is(&call(&app, req(Method::PUT, "/api/v1/wishlist").cookie(&cookie).json(&good)).await, StatusCode::OK);
    for bad in [json!({ "grids": ["JN4"] }), json!({ "calls": ["K1 ABC"] }), json!({ "cooldown_min": 0 }), json!({ "notify_url": "ftp://x" })] {
        let resp = call(&app, req(Method::PUT, "/api/v1/wishlist").cookie(&cookie).json(&bad.to_string())).await;
        status_is(&resp, StatusCode::BAD_REQUEST);
        assert_eq!(body_json(resp).await["error"], "bad_wishlist");
    }
    let kept = body_json(call(&app, req(Method::GET, "/api/v1/wishlist").cookie(&cookie)).await).await;
    assert_eq!(kept["config"]["calls"], json!(["K1ABC"]));
    std::fs::remove_dir_all(&dir).ok();
}

#[tokio::test]
async fn the_test_alert_reaches_the_page_and_the_recent_list_and_says_there_is_no_phone_address() {
    let (s, dir) = with_service("test");
    let mut events = s.bus.subscribe_events();
    let cookie = cookie_for(&s);
    let app = router(&s);
    let resp = call(&app, req(Method::POST, "/api/v1/wishlist/test").cookie(&cookie)).await;
    status_is(&resp, StatusCode::OK);
    assert_eq!(body_json(resp).await["phone"], "no_url");
    match events.recv().await.unwrap() {
        aurora_core::Event::Wanted(a) => assert_eq!((a.call.as_str(), a.source.as_str()), ("TEST", "test")),
        other => panic!("evento inatteso: {other:?}"),
    }
    let list = body_json(call(&app, req(Method::GET, "/api/v1/wishlist").cookie(&cookie)).await).await;
    assert_eq!(list["recent"][0]["call"], "TEST");
    std::fs::remove_dir_all(&dir).ok();
}
