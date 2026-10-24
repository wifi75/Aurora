mod common;

use std::sync::Arc;

use axum::http::{Method, StatusCode};
use common::*;
use dx_cluster::ClusterService;
use remote_api::AppState;

fn cluster_state(name: &str) -> AppState {
    let dir = std::env::temp_dir().join(format!("aurora-cluster-api-{name}-{}", std::process::id()));
    std::fs::remove_dir_all(&dir).ok();
    std::fs::create_dir_all(&dir).unwrap();
    state().with_cluster(ClusterService::open(&dir.join("dx-cluster.json")))
}

async fn send(s: &AppState, method: Method, uri: &str, body: Option<&str>) -> axum::response::Response {
    let cookie = cookie_for(s);
    let mut r = req(method, uri).cookie(&cookie);
    if let Some(b) = body {
        r = r.json(b);
    }
    call(&router(s), r).await
}

#[tokio::test]
async fn needs_a_paired_device() {
    let s = cluster_state("auth");
    status_is(&call(&router(&s), req(Method::GET, "/api/v1/dx-cluster")).await, StatusCode::UNAUTHORIZED);
    status_is(&call(&router(&s), req(Method::GET, "/api/v1/dx-cluster/catalog")).await, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn shows_the_defaults_switched_off_with_no_spots() {
    let s = cluster_state("defaults");
    let v = body_json(send(&s, Method::GET, "/api/v1/dx-cluster", None).await).await;
    assert_eq!(v["config"]["enabled"], false);
    assert!(v["config"]["servers"].as_array().unwrap().len() >= 1);
    assert_eq!(v["spots"].as_array().unwrap().len(), 0);
    assert_eq!(v["servers"].as_array().unwrap().len(), 0);
}

#[tokio::test]
async fn the_catalog_lists_public_servers() {
    let s = cluster_state("catalog");
    let v = body_json(send(&s, Method::GET, "/api/v1/dx-cluster/catalog", None).await).await;
    let list = v.as_array().unwrap();
    assert!(list.iter().any(|e| e["host"] == "hamqth.com" && e["port"] == 7300));
    assert!(list.iter().any(|e| e["host"] == "telnet.reversebeacon.net" && e["port"] == 7001));
}

#[tokio::test]
async fn saves_valid_settings_and_refuses_invalid_ones() {
    let s = cluster_state("put");
    let ok = r#"{"enabled":false,"callsign":"IU3ABC","servers":[{"name":"Mio","host":"cluster.example.org","port":7300,"enabled":true}],"login_commands":["set/ft8"],"max_spots":300,"max_age_min":30}"#;
    status_is(&send(&s, Method::PUT, "/api/v1/dx-cluster", Some(ok)).await, StatusCode::OK);
    let v = body_json(send(&s, Method::GET, "/api/v1/dx-cluster", None).await).await;
    assert_eq!(v["config"]["callsign"], "IU3ABC");
    assert_eq!(v["config"]["servers"][0]["host"], "cluster.example.org");
    let spot = r#"{"enabled":false,"callsign":"IU3ABC","servers":[],"login_commands":["dx 14074 K1ABC"],"max_spots":300,"max_age_min":30}"#;
    let resp = send(&s, Method::PUT, "/api/v1/dx-cluster", Some(spot)).await;
    status_is(&resp, StatusCode::BAD_REQUEST);
    assert_eq!(body_json(resp).await["error"], "bad_cluster");
}

#[tokio::test]
async fn commands_are_limited_and_need_a_connected_server() {
    let s = cluster_state("cmd");
    let publicly = send(&s, Method::POST, "/api/v1/dx-cluster/command", Some(r#"{"server":"a:1","text":"dx 14074 K1ABC"}"#)).await;
    status_is(&publicly, StatusCode::BAD_REQUEST);
    assert_eq!(body_json(publicly).await["error"], "bad_command");
    let offline = send(&s, Method::POST, "/api/v1/dx-cluster/command", Some(r#"{"server":"a:1","text":"sh/dx 5"}"#)).await;
    status_is(&offline, StatusCode::CONFLICT);
    assert_eq!(body_json(offline).await["error"], "not_connected");
}

#[tokio::test]
async fn without_the_service_everything_says_so() {
    let s = state();
    let resp = send(&s, Method::GET, "/api/v1/dx-cluster", None).await;
    status_is(&resp, StatusCode::NOT_FOUND);
    assert_eq!(body_json(resp).await["error"], "cluster_unavailable");
    let _ = Arc::new(0);
}

#[tokio::test]
async fn the_connection_test_needs_a_pairing_and_refuses_bad_addresses() {
    let s = cluster_state("probe");
    status_is(&call(&router(&s), req(Method::POST, "/api/v1/dx-cluster/test").json(r#"{"host":"a.b","port":7300}"#)).await, StatusCode::UNAUTHORIZED);
    let bad = send(&s, Method::POST, "/api/v1/dx-cluster/test", Some(r#"{"host":"; rm -rf","port":7300}"#)).await;
    status_is(&bad, StatusCode::BAD_REQUEST);
    assert_eq!(body_json(bad).await["error"], "bad_server");
    status_is(&send(&s, Method::POST, "/api/v1/dx-cluster/test", Some(r#"{"host":"a.b","port":0}"#)).await, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn the_connection_test_reports_an_unreachable_server_without_failing() {
    let s = cluster_state("probe-down");
    let resp = send(&s, Method::POST, "/api/v1/dx-cluster/test", Some(r#"{"host":"127.0.0.1","port":1}"#)).await;
    status_is(&resp, StatusCode::OK);
    let v = body_json(resp).await;
    assert_eq!(v["ok"], false);
    assert!(v["error"].is_string());
}
