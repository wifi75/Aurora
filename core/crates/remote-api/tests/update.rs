mod common;

use axum::http::{Method, StatusCode};
use common::*;
use remote_api::AppState;
use serde_json::json;

fn with_service(name: &str) -> (AppState, std::path::PathBuf) {
    let dir = std::env::temp_dir().join(format!("aurora-api-upd-{}-{name}", std::process::id()));
    std::fs::remove_dir_all(&dir).ok();
    std::fs::create_dir_all(&dir).unwrap();
    // un server che non c'e': il controllo fallisce in fretta e senza toccare internet
    let service = updater::UpdateService::open_with_api(&dir.join("update.json"), "0.9.0", "http://127.0.0.1:1");
    (state().with_updates(service), dir)
}

#[tokio::test]
async fn updates_need_a_paired_device_and_say_when_they_are_not_there() {
    let s = state();
    for (method, uri) in [(Method::GET, "/api/v1/update"), (Method::PUT, "/api/v1/update"), (Method::POST, "/api/v1/update/check")] {
        status_is(&call(&router(&s), req(method, uri)).await, StatusCode::UNAUTHORIZED);
    }
    let cookie = cookie_for(&s);
    let resp = call(&router(&s), req(Method::GET, "/api/v1/update").cookie(&cookie)).await;
    status_is(&resp, StatusCode::NOT_FOUND);
    assert_eq!(body_json(resp).await["error"], "update_unavailable");
}

#[tokio::test]
async fn shows_the_version_in_use_and_the_defaults_without_a_token() {
    let (s, dir) = with_service("get");
    let cookie = cookie_for(&s);
    let v = body_json(call(&router(&s), req(Method::GET, "/api/v1/update").cookie(&cookie)).await).await;
    assert_eq!(v["current"], "0.9.0");
    assert_eq!(v["enabled"], true);
    assert_eq!(v["repo"], "wifi75/Aurora");
    assert_eq!(v["interval_hours"], 6);
    assert_eq!(v["has_token"], false);
    assert!(v["available"].is_null());
    std::fs::remove_dir_all(&dir).ok();
}

#[tokio::test]
async fn saves_the_settings_and_the_token_never_comes_back() {
    let (s, dir) = with_service("put");
    let cookie = cookie_for(&s);
    let app = router(&s);
    let body = json!({ "enabled": true, "repo": "wifi75/Aurora", "interval_hours": 12, "prereleases": true, "token": "ghp_segretissimo" }).to_string();
    let put = call(&app, req(Method::PUT, "/api/v1/update").cookie(&cookie).json(&body)).await;
    status_is(&put, StatusCode::OK);
    let text = serde_json::to_string(&body_json(put).await).unwrap();
    assert!(!text.contains("ghp_segretissimo"), "{text}");
    let get = call(&app, req(Method::GET, "/api/v1/update").cookie(&cookie)).await;
    let raw = serde_json::to_string(&body_json(get).await).unwrap();
    assert!(raw.contains("\"has_token\":true") && raw.contains("\"interval_hours\":12") && raw.contains("\"prereleases\":true"), "{raw}");
    assert!(!raw.contains("ghp_segretissimo"));
    // senza il campo token si lascia quello che c'e'
    let keep = json!({ "enabled": false, "repo": "wifi75/Aurora", "interval_hours": 24 }).to_string();
    status_is(&call(&app, req(Method::PUT, "/api/v1/update").cookie(&cookie).json(&keep)).await, StatusCode::OK);
    let after = body_json(call(&app, req(Method::GET, "/api/v1/update").cookie(&cookie)).await).await;
    assert_eq!((after["has_token"].clone(), after["enabled"].clone()), (json!(true), json!(false)));
    // con una stringa vuota si toglie
    let clear = json!({ "enabled": true, "repo": "wifi75/Aurora", "interval_hours": 24, "token": "" }).to_string();
    status_is(&call(&app, req(Method::PUT, "/api/v1/update").cookie(&cookie).json(&clear)).await, StatusCode::OK);
    assert_eq!(body_json(call(&app, req(Method::GET, "/api/v1/update").cookie(&cookie)).await).await["has_token"], false);
    std::fs::remove_dir_all(&dir).ok();
}

#[tokio::test]
async fn refuses_values_that_cannot_work() {
    let (s, dir) = with_service("bad");
    let cookie = cookie_for(&s);
    let app = router(&s);
    for bad in [
        json!({ "enabled": true, "repo": "senza-barra", "interval_hours": 6 }),
        json!({ "enabled": true, "repo": "a/b", "interval_hours": 0 }),
        json!({ "enabled": true, "repo": "a/b", "interval_hours": 500 }),
        json!({ "enabled": true, "repo": "a/b", "interval_hours": 6, "token": "con spazio" }),
    ] {
        let resp = call(&app, req(Method::PUT, "/api/v1/update").cookie(&cookie).json(&bad.to_string())).await;
        status_is(&resp, StatusCode::BAD_REQUEST);
        assert_eq!(body_json(resp).await["error"], "bad_update");
    }
    std::fs::remove_dir_all(&dir).ok();
}

#[tokio::test]
async fn an_immediate_check_reports_the_error_instead_of_failing() {
    let (s, dir) = with_service("check");
    let cookie = cookie_for(&s);
    let resp = call(&router(&s), req(Method::POST, "/api/v1/update/check").cookie(&cookie)).await;
    status_is(&resp, StatusCode::OK);
    let v = body_json(resp).await;
    assert!(v["error"].is_string(), "{v}");
    assert!(v["checked_at_ms"].as_u64().unwrap_or(0) > 0);
    std::fs::remove_dir_all(&dir).ok();
}
