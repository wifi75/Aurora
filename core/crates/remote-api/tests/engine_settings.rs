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

#[tokio::test]
async fn region_and_type2_messages_are_live_settings_with_their_own_values() {
    let (s, store) = with_store("region");
    let cookie = cookie_for(&s);
    let doc = body_json(call(&router(&s), req(Method::GET, "/api/v1/engine-settings").cookie(&cookie)).await).await;
    assert_eq!((doc["values"]["Region"].as_str(), doc["values"]["Type2MsgGen"].as_str()), (Some("ALL"), Some("type_2_msg_3_full")), "valori di fabbrica di JTDX");
    assert_eq!((doc["items"]["Region"]["state"].as_str(), doc["items"]["Type2MsgGen"]["state"].as_str()), (Some("live"), Some("live")));
    assert_eq!(doc["items"]["Type2MsgGen"]["note"], "jt_only", "in FT8 e FT4 JTDX non lo legge: la pagina lo dice");
    assert!(doc["items"]["Region"].get("note").is_none());
    let body = r#"{"changes":{"Region":"R1","Type2MsgGen":"type_2_msg_5_only"}}"#;
    status_is(&call(&router(&s), req(Method::PUT, "/api/v1/engine-settings").cookie(&cookie).json(body)).await, StatusCode::OK);
    let r = store.reports();
    assert_eq!((r.region, r.type2_msg_gen), (rig_control::table::Region::R1, remote_api::engine_settings::Type2MsgGen::Tx5Only));
    assert_eq!(s.frequency_region(), rig_control::table::Region::R1, "la tabella delle frequenze la legge subito");
    for bad in [r#"{"changes":{"Region":"R4"}}"#, r#"{"changes":{"Region":"Region 1"}}"#, r#"{"changes":{"Type2MsgGen":"type_2_msg_2_full"}}"#] {
        let resp = call(&router(&s), req(Method::PUT, "/api/v1/engine-settings").cookie(&cookie).json(bad)).await;
        status_is(&resp, StatusCode::BAD_REQUEST);
        assert_eq!(body_json(resp).await["error"], "bad_setting", "{bad}");
    }
    assert_eq!(store.values()["Region"], "R1", "un valore rifiutato non cambia niente");
}
