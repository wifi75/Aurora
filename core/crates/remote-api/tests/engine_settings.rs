mod common;

use std::{collections::BTreeMap, sync::Arc};

use axum::http::{Method, StatusCode};
use common::*;
use remote_api::{engine_settings::EngineSettingsStore, AppState};

fn with_store(name: &str) -> (AppState, Arc<EngineSettingsStore>) {
    let dir = std::env::temp_dir().join(format!("aurora-route-engset-{name}-{}", std::process::id()));
    std::fs::remove_dir_all(&dir).ok();
    let store = Arc::new(EngineSettingsStore::open(&dir.join("settings-motore.json"), remote_api::engine_settings::defaults));
    (state().with_engine_settings(Arc::clone(&store)), store)
}

#[tokio::test]
async fn needs_a_paired_device_and_the_native_engine() {
    let s = state();
    status_is(&call(&router(&s), req(Method::GET, "/api/v1/engine-settings")).await, StatusCode::UNAUTHORIZED);
    let cookie = cookie_for(&s);
    let resp = call(&router(&s), req(Method::GET, "/api/v1/engine-settings").cookie(&cookie)).await;
    status_is(&resp, StatusCode::NOT_FOUND);
    assert_eq!(body_json(resp).await["error"], "engine_settings_unavailable", "con il motore JTDX non c'e': resta /engine-config");
}

#[tokio::test]
async fn reads_values_and_the_state_of_every_item() {
    let (s, _) = with_store("get");
    let cookie = cookie_for(&s);
    let resp = call(&router(&s), req(Method::GET, "/api/v1/engine-settings").cookie(&cookie)).await;
    status_is(&resp, StatusCode::OK);
    let doc = body_json(resp).await;
    assert_eq!(doc["version"], 1);
    assert_eq!(doc["items"]["UDP2Server"]["state"], "live");
    assert_eq!(doc["items"]["Rig"]["state"], "unavailable");
    assert_eq!(doc["items"]["AllowSpotsDXSummit"]["state"], "live", "lo spot su DXSummit e' una voce viva");
    assert_eq!(doc["values"]["AllowSpotsDXSummit"], "true", "e acceso di base");
    assert_eq!(doc["values"]["UDP2ServerPort"], "2237");
}

#[tokio::test]
async fn a_change_is_applied_at_once_and_returned() {
    let (s, store) = with_store("put");
    let cookie = cookie_for(&s);
    let body = r#"{"changes":{"UDP2Server":"192.168.1.9","UDP2ServerPort":"2240","EnableUDP2adifBroadcast":"true"}}"#;
    let resp = call(&router(&s), req(Method::PUT, "/api/v1/engine-settings").cookie(&cookie).json(body)).await;
    status_is(&resp, StatusCode::OK);
    assert_eq!(body_json(resp).await["values"]["UDP2ServerPort"], "2240");
    assert_eq!(store.reports().secondary, Some("192.168.1.9:2240".parse().unwrap()));
}

#[tokio::test]
async fn refuses_bad_values_unavailable_items_empty_bodies_and_remote_peers() {
    let (s, store) = with_store("refuse");
    let cookie = cookie_for(&s);
    let before: BTreeMap<String, String> = store.values();
    for body in [r#"{"changes":{"UDP2ServerPort":"99999"}}"#, r#"{"changes":{"Rig":"x"}}"#, r#"{"changes":{"udpWindowToFront":"true"}}"#, r#"{"changes":{}}"#] {
        let resp = call(&router(&s), req(Method::PUT, "/api/v1/engine-settings").cookie(&cookie).json(body)).await;
        status_is(&resp, StatusCode::BAD_REQUEST);
        assert_eq!(body_json(resp).await["error"], "bad_setting", "{body}");
    }
    assert_eq!(store.values(), before, "niente e' cambiato");
    let mut outside = req(Method::PUT, "/api/v1/engine-settings").cookie(&cookie).json(r#"{"changes":{"MyCall":"IU3ABC"}}"#);
    outside.peer = axum::extract::ConnectInfo("8.8.8.8:50000".parse().unwrap());
    let resp = call(&router(&s), outside).await;
    status_is(&resp, StatusCode::FORBIDDEN);
    assert_eq!(body_json(resp).await["error"], "local_only");
}
