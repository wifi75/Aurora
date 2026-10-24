mod common;

use axum::http::{Method, StatusCode};
use common::*;
use remote_api::{ui_settings::UiSettings, AppState};
use std::sync::Arc;

fn state_with_store(name: &str) -> (AppState, std::path::PathBuf) {
    let dir = std::env::temp_dir().join(format!("aurora-ui-api-{name}-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("ui-settings.json");
    (state().with_ui_settings(Arc::new(UiSettings::open(&path))), path)
}

async fn put(s: &AppState, kind: &str, body: &str) -> axum::response::Response {
    let cookie = cookie_for(s);
    call(&router(s), req(Method::PUT, &format!("/api/v1/ui-settings/{kind}")).cookie(&cookie).json(body)).await
}

async fn get(s: &AppState, kind: &str) -> axum::response::Response {
    let cookie = cookie_for(s);
    call(&router(s), req(Method::GET, &format!("/api/v1/ui-settings/{kind}")).cookie(&cookie)).await
}

#[tokio::test]
async fn needs_a_paired_device() {
    let (s, _) = state_with_store("auth");
    status_is(&call(&router(&s), req(Method::GET, "/api/v1/ui-settings/desktop")).await, StatusCode::UNAUTHORIZED);
    status_is(&call(&router(&s), req(Method::PUT, "/api/v1/ui-settings/desktop").json("{}")).await, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn saved_preferences_come_back_and_survive_a_restart() {
    let (s, path) = state_with_store("roundtrip");
    assert_eq!(body_json(get(&s, "desktop").await).await, serde_json::json!({}));
    status_is(&put(&s, "desktop", r#"{"aurora-prefs":"{\"log4omCount\":35}"}"#).await, StatusCode::OK);
    assert_eq!(body_json(get(&s, "desktop").await).await["aurora-prefs"], "{\"log4omCount\":35}");
    assert_eq!(body_json(get(&s, "mobile").await).await, serde_json::json!({}), "il telefono ha le sue");

    let again = state().with_ui_settings(Arc::new(UiSettings::open(&path)));
    assert_eq!(body_json(get(&again, "desktop").await).await["aurora-prefs"], "{\"log4omCount\":35}");
    std::fs::remove_dir_all(path.parent().unwrap()).ok();
}

#[tokio::test]
async fn unknown_kinds_foreign_keys_and_huge_bodies_are_refused() {
    let (s, path) = state_with_store("refuse");
    status_is(&get(&s, "tablet").await, StatusCode::BAD_REQUEST);
    status_is(&put(&s, "tablet", "{}").await, StatusCode::BAD_REQUEST);
    let resp = put(&s, "desktop", r#"{"altro":"x"}"#).await;
    status_is(&resp, StatusCode::BAD_REQUEST);
    assert_eq!(body_json(resp).await["error"], "bad_settings");
    let huge = format!(r#"{{"aurora-prefs":"{}"}}"#, "x".repeat(300 * 1024));
    status_is(&put(&s, "desktop", &huge).await, StatusCode::BAD_REQUEST);
    std::fs::remove_dir_all(path.parent().unwrap()).ok();
}

#[tokio::test]
async fn without_a_store_reading_is_empty_and_writing_says_so() {
    let s = state();
    assert_eq!(body_json(get(&s, "desktop").await).await, serde_json::json!({}));
    status_is(&put(&s, "desktop", r#"{"aurora-prefs":"x"}"#).await, StatusCode::NOT_FOUND);
}
