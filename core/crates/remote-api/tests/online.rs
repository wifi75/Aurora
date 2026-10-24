mod common;

use std::sync::Arc;

use axum::http::{Method, StatusCode};
use common::*;
use online_info::{Fetch, OnlineService};
use remote_api::AppState;
use serde_json::json;

struct Net;

impl Fetch for Net {
    fn get(&self, url: &str) -> Result<String, String> {
        if url.contains("pskreporter") {
            Ok(r#"<receptionReports><receptionReport receiverCallsign="K1ABC" receiverLocator="FN42" frequency="14075500" flowStartSeconds="100" mode="FT8" sNR="-9"/></receptionReports>"#.into())
        } else {
            Ok("<solar><solardata><solarflux>120</solarflux></solardata></solar>".into())
        }
    }
}

fn with_service(name: &str) -> (AppState, std::path::PathBuf) {
    let path = std::env::temp_dir().join(format!("aurora-api-online-{}-{name}.json", std::process::id()));
    std::fs::remove_file(&path).ok();
    (state().with_online(OnlineService::open(&path, Arc::new(Net))), path)
}

#[tokio::test]
async fn needs_a_paired_device_and_says_when_the_service_is_not_there() {
    let s = state();
    for (method, uri) in [(Method::GET, "/api/v1/online"), (Method::PUT, "/api/v1/online"), (Method::GET, "/api/v1/online/psk"), (Method::GET, "/api/v1/online/solar"), (Method::POST, "/api/v1/online/refresh")] {
        status_is(&call(&router(&s), req(method, uri)).await, StatusCode::UNAUTHORIZED);
    }
    let cookie = cookie_for(&s);
    let resp = call(&router(&s), req(Method::GET, "/api/v1/online").cookie(&cookie)).await;
    status_is(&resp, StatusCode::NOT_FOUND);
    assert_eq!(body_json(resp).await["error"], "online_unavailable");
}

#[tokio::test]
async fn starts_off_and_asks_nothing_until_turned_on() {
    let (s, path) = with_service("off");
    let cookie = cookie_for(&s);
    let app = router(&s);
    assert_eq!(body_json(call(&app, req(Method::GET, "/api/v1/online").cookie(&cookie)).await).await, json!({ "psk": false, "solar": false }));
    let psk = body_json(call(&app, req(Method::GET, "/api/v1/online/psk").cookie(&cookie)).await).await;
    assert_eq!((psk["enabled"].as_bool(), psk["psk"]["reports"].as_array().map(Vec::len)), (Some(false), Some(0)));
    let refreshed = body_json(call(&app, req(Method::POST, "/api/v1/online/refresh").cookie(&cookie)).await).await;
    assert_eq!(refreshed["asked"], false);
    std::fs::remove_file(path).ok();
}

#[tokio::test]
async fn turning_propagation_on_and_refreshing_brings_the_data() {
    let (s, path) = with_service("solar");
    let cookie = cookie_for(&s);
    let app = router(&s);
    let put = call(&app, req(Method::PUT, "/api/v1/online").cookie(&cookie).json(&json!({ "solar": true }).to_string())).await;
    assert_eq!(body_json(put).await, json!({ "psk": false, "solar": true }));
    let refreshed = body_json(call(&app, req(Method::POST, "/api/v1/online/refresh").cookie(&cookie)).await).await;
    assert_eq!(refreshed["asked"], true);
    let solar = body_json(call(&app, req(Method::GET, "/api/v1/online/solar").cookie(&cookie)).await).await;
    assert_eq!((solar["enabled"].as_bool(), solar["solar"]["data"]["sfi"].as_u64()), (Some(true), Some(120)));
    std::fs::remove_file(path).ok();
}
