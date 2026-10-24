mod common;

use std::sync::{Arc, Mutex};

use axum::http::{Method, StatusCode};
use common::*;
use input_level::{InputLevelService, VolumeControl};
use remote_api::AppState;
use serde_json::json;

struct Fake(Mutex<f32>);

impl VolumeControl for Fake {
    fn name(&self) -> String {
        "Ingresso di prova".into()
    }

    fn volume(&self) -> Result<f32, String> {
        Ok(*self.0.lock().unwrap())
    }

    fn set_volume(&self, value: f32) -> Result<(), String> {
        *self.0.lock().unwrap() = value;
        Ok(())
    }
}

fn with_service(name: &str) -> (AppState, Arc<Fake>, std::path::PathBuf) {
    let path = std::env::temp_dir().join(format!("aurora-api-input-{}-{name}.json", std::process::id()));
    std::fs::remove_file(&path).ok();
    let fake = Arc::new(Fake(Mutex::new(0.8)));
    (state().with_input_level(InputLevelService::open(fake.clone(), &path)), fake, path)
}

#[tokio::test]
async fn needs_a_paired_device_and_says_when_there_is_no_input_to_adjust() {
    let s = state();
    status_is(&call(&router(&s), req(Method::GET, "/api/v1/input-level")).await, StatusCode::UNAUTHORIZED);
    status_is(&call(&router(&s), req(Method::PUT, "/api/v1/input-level").json("{}")).await, StatusCode::UNAUTHORIZED);
    let cookie = cookie_for(&s);
    let resp = call(&router(&s), req(Method::GET, "/api/v1/input-level").cookie(&cookie)).await;
    status_is(&resp, StatusCode::NOT_FOUND);
    assert_eq!(body_json(resp).await["error"], "input_level_unavailable");
}

#[tokio::test]
async fn reads_the_device_volume_and_auto() {
    let (s, _, path) = with_service("get");
    let cookie = cookie_for(&s);
    let body = body_json(call(&router(&s), req(Method::GET, "/api/v1/input-level").cookie(&cookie)).await).await;
    assert_eq!(body, json!({ "device": "Ingresso di prova", "volume_pct": 80, "auto": false }));
    std::fs::remove_file(path).ok();
}

#[tokio::test]
async fn a_manual_volume_is_applied_and_turns_auto_off_unless_asked_to_keep_it() {
    let (s, fake, path) = with_service("put");
    let cookie = cookie_for(&s);
    let app = router(&s);
    let on = call(&app, req(Method::PUT, "/api/v1/input-level").cookie(&cookie).json(&json!({ "auto": true }).to_string())).await;
    assert_eq!(body_json(on).await["status"]["auto"], true);
    let manual = call(&app, req(Method::PUT, "/api/v1/input-level").cookie(&cookie).json(&json!({ "volume_pct": 40 }).to_string())).await;
    let manual = body_json(manual).await;
    assert_eq!((manual["status"]["volume_pct"].as_i64(), manual["status"]["auto"].as_bool()), (Some(40), Some(false)));
    assert!((*fake.0.lock().unwrap() - 0.4).abs() < 1e-6);
    let both = call(&app, req(Method::PUT, "/api/v1/input-level").cookie(&cookie).json(&json!({ "volume_pct": 55, "auto": true }).to_string())).await;
    let both = body_json(both).await;
    assert_eq!((both["status"]["volume_pct"].as_i64(), both["status"]["auto"].as_bool()), (Some(55), Some(true)));
    std::fs::remove_file(path).ok();
}

#[tokio::test]
async fn refuses_a_volume_outside_the_scale_and_an_empty_request() {
    let (s, fake, path) = with_service("bad");
    let cookie = cookie_for(&s);
    let app = router(&s);
    for bad in [json!({ "volume_pct": 101 }), json!({ "volume_pct": -1 }), json!({})] {
        let resp = call(&app, req(Method::PUT, "/api/v1/input-level").cookie(&cookie).json(&bad.to_string())).await;
        status_is(&resp, StatusCode::BAD_REQUEST);
        assert_eq!(body_json(resp).await["error"], "bad_input_level");
    }
    assert_eq!(*fake.0.lock().unwrap(), 0.8);
    std::fs::remove_file(path).ok();
}
