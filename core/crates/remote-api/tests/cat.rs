mod common;

use std::{sync::Arc, time::Duration};

use axum::http::{Method, StatusCode};
use common::*;
use remote_api::AppState;
use rig_control::{
    cat::{CatParts, CatService, LinkFactory},
    fake_rigctld::FakeRigctld,
    hamlib::RigModel,
    hamlib_rig::{Link, Timing},
};
use serde_json::{json, Value};

fn timing() -> Timing {
    Timing {
        poll: Duration::from_millis(30),
        tx_poll_min: Duration::from_millis(10),
        quick_reads: [Duration::from_millis(10), Duration::from_millis(20)],
        startup_block: Duration::from_millis(100),
        ptt_pause: Duration::from_millis(5),
        qsy_check: Duration::from_millis(20),
        reconnect_step: Duration::from_millis(40),
        reconnect_cap: Duration::from_millis(200),
        max_reconnects: 3,
        tolerate: 3,
        tolerate_io: 8,
    }
}

struct Setup {
    state: AppState,
    fake: Arc<FakeRigctld>,
}

fn setup(name: &str, rigctld: bool) -> Setup {
    let dir = std::env::temp_dir().join(format!("aurora-cat-api-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let fake = Arc::new(FakeRigctld::start(14_074_000));
    let f = Arc::clone(&fake);
    let factory: LinkFactory = Arc::new(move |_| f.client(Duration::from_millis(500)).map(Link::from_client));
    let models = vec![
        RigModel { id: 1, manufacturer: "Hamlib".into(), model: "Dummy".into(), status: "Stable".into() },
        RigModel { id: 1029, manufacturer: "Yaesu".into(), model: "FT-2000".into(), status: "Stable".into() },
    ];
    let cat = CatService::with_parts(&dir.join("radio.json"), CatParts::custom(factory, rigctld.then(|| "rigctld.exe".into()), models, timing()));
    Setup { state: state().with_cat(cat), fake }
}

enum Via {
    /// Dal telefono o da un altro PC della rete di casa.
    Network,
    /// Dal PC, connessione diretta.
    Pc,
    /// Dal PC ma attraverso un proxy (Vite, tunnel): non vale come locale.
    Proxy,
}

async fn send(s: &AppState, method: Method, uri: &str, body: Option<Value>, via: Via) -> axum::response::Response {
    let cookie = cookie_for(s);
    let mut r = req(method, uri).cookie(&cookie);
    if let Some(b) = body {
        r = r.json(&b.to_string());
    }
    r = match via {
        Via::Network => r,
        Via::Pc => r.via_loopback(),
        Via::Proxy => r.via_loopback().header("x-forwarded-for", "1.2.3.4"),
    };
    call(&router(s), r).await
}

async fn get(s: &AppState, uri: &str) -> Value {
    let resp = send(s, Method::GET, uri, None, Via::Network).await;
    status_is(&resp, StatusCode::OK);
    body_json(resp).await
}

const FT2000: &str = r#"{"model":1029,"rig_name":"Yaesu FT-2000","port":"COM3","baud":38400,"data_bits":8,"stop_bits":2,"handshake":"hardware","poll_secs":2}"#;

fn ft2000() -> Value {
    serde_json::from_str(FT2000).unwrap()
}

async fn wait_state(s: &AppState, state: &str) -> Value {
    for _ in 0..300 {
        let v = get(s, "/api/v1/cat").await;
        if v["status"]["state"] == state {
            return v;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    panic!("stato {state} non raggiunto");
}

#[tokio::test]
async fn every_route_needs_a_paired_device() {
    let s = setup("auth", true);
    for (m, uri) in [
        (Method::GET, "/api/v1/cat"),
        (Method::PUT, "/api/v1/cat"),
        (Method::POST, "/api/v1/cat/connect"),
        (Method::POST, "/api/v1/cat/disconnect"),
        (Method::POST, "/api/v1/cat/profiles"),
        (Method::GET, "/api/v1/cat/models"),
        (Method::GET, "/api/v1/cat/ports"),
        (Method::POST, "/api/v1/cat/detect"),
    ] {
        let resp = call(&router(&s.state), req(m.clone(), uri)).await;
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED, "{m} {uri}");
    }
}

#[tokio::test]
async fn without_the_service_everything_says_so() {
    let s = state();
    let resp = send(&s, Method::GET, "/api/v1/cat", None, Via::Network).await;
    status_is(&resp, StatusCode::NOT_FOUND);
    assert_eq!(body_json(resp).await["error"], "cat_unavailable");
}

#[tokio::test]
async fn it_shows_state_configuration_and_profiles() {
    let s = setup("get", true);
    let v = get(&s.state, "/api/v1/cat").await;
    assert_eq!(v["status"]["state"], "disconnected");
    assert_eq!(v["status"]["backend"], "hamlib");
    assert_eq!(v["status"]["rigctld_found"], true);
    assert_eq!(v["config"]["model"], 0);
    assert_eq!(v["config"]["poll_secs"], 2);
    assert_eq!(v["profiles"], json!([]));
    let s = setup("get-missing", false);
    assert_eq!(get(&s.state, "/api/v1/cat").await["status"]["rigctld_found"], false, "la schermata avvisa se manca rigctld");
}

#[tokio::test]
async fn saving_is_allowed_only_from_the_pc_and_validates() {
    let s = setup("put", true);
    for via in [Via::Network, Via::Proxy] {
        let resp = send(&s.state, Method::PUT, "/api/v1/cat", Some(ft2000()), via).await;
        status_is(&resp, StatusCode::FORBIDDEN);
        assert_eq!(body_json(resp).await["error"], "local_only");
    }
    assert_eq!(get(&s.state, "/api/v1/cat").await["config"]["model"], 0, "nulla e' stato salvato");

    let resp = send(&s.state, Method::PUT, "/api/v1/cat", Some(ft2000()), Via::Pc).await;
    status_is(&resp, StatusCode::OK);
    let v = body_json(resp).await;
    assert_eq!((v["config"]["model"].clone(), v["config"]["handshake"].clone()), (json!(1029), json!("hardware")));
    assert_eq!(v["status"]["state"], "disconnected", "salvare non collega");

    let mut bad = ft2000();
    bad["baud"] = json!(12345);
    let resp = send(&s.state, Method::PUT, "/api/v1/cat", Some(bad), Via::Pc).await;
    status_is(&resp, StatusCode::BAD_REQUEST);
    assert_eq!(body_json(resp).await["error"], "bad_baud");
    let mut bad = ft2000();
    bad["port"] = json!("COM3 & calc");
    let resp = send(&s.state, Method::PUT, "/api/v1/cat", Some(bad), Via::Pc).await;
    assert_eq!(body_json(resp).await["error"], "bad_port");
    assert_eq!(get(&s.state, "/api/v1/cat").await["config"]["baud"], 38400, "un valore non valido non cambia nulla");
}

#[tokio::test]
async fn connecting_and_disconnecting_are_from_the_pc_and_show_the_real_state() {
    let s = setup("connect", true);
    for via in [Via::Network, Via::Proxy] {
        let resp = send(&s.state, Method::POST, "/api/v1/cat/connect", None, via).await;
        status_is(&resp, StatusCode::FORBIDDEN);
    }
    status_is(&send(&s.state, Method::POST, "/api/v1/cat/disconnect", None, Via::Network).await, StatusCode::FORBIDDEN);

    // senza modello non si collega
    let resp = send(&s.state, Method::POST, "/api/v1/cat/connect", None, Via::Pc).await;
    status_is(&resp, StatusCode::BAD_REQUEST);
    assert_eq!(body_json(resp).await["error"], "bad_config");

    status_is(&send(&s.state, Method::PUT, "/api/v1/cat", Some(ft2000()), Via::Pc).await, StatusCode::OK);
    status_is(&send(&s.state, Method::POST, "/api/v1/cat/connect", None, Via::Pc).await, StatusCode::OK);
    let v = wait_state(&s.state, "connected").await;
    assert_eq!(v["status"]["rig"]["frequency_hz"], 14_074_000);
    assert_eq!(v["status"]["rig"]["ptt"], false);

    let v = body_json(send(&s.state, Method::POST, "/api/v1/cat/disconnect", None, Via::Pc).await).await;
    assert_eq!(v["status"]["state"], "disconnected");
    assert!(!s.fake.ptt());
}

#[tokio::test]
async fn a_missing_rigctld_is_reported_by_connect() {
    let s = setup("nodaemon", false);
    status_is(&send(&s.state, Method::PUT, "/api/v1/cat", Some(ft2000()), Via::Pc).await, StatusCode::OK);
    let resp = send(&s.state, Method::POST, "/api/v1/cat/connect", None, Via::Pc).await;
    status_is(&resp, StatusCode::CONFLICT);
    let v = body_json(resp).await;
    assert_eq!(v["error"], "rigctld_missing");
    assert!(v["message"].as_str().unwrap().contains("AURORA_RIGCTLD"));
}

#[tokio::test]
async fn the_commands_use_the_radio_connected_after_the_start() {
    let s = setup("late", true);
    let move_to = |hz: u64| send(&s.state, Method::POST, "/api/v1/commands/frequency", Some(json!({ "hz": hz })), Via::Network);
    let resp = move_to(14_074_000).await;
    status_is(&resp, StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(body_json(resp).await["error"], "rig_unavailable");

    status_is(&send(&s.state, Method::PUT, "/api/v1/cat", Some(ft2000()), Via::Pc).await, StatusCode::OK);
    status_is(&send(&s.state, Method::POST, "/api/v1/cat/connect", None, Via::Pc).await, StatusCode::OK);
    wait_state(&s.state, "connected").await;
    status_is(&move_to(7_074_000).await, StatusCode::OK);
    assert_eq!(s.fake.frequency(), 7_074_000);
    // la schermata principale vede il comando radio
    assert_eq!(get(&s.state, "/api/v1/station-info").await["rig_control"], true);
}

#[tokio::test]
async fn profiles_are_saved_loaded_created_and_deleted_from_the_pc() {
    let s = setup("profiles", true);
    status_is(&send(&s.state, Method::PUT, "/api/v1/cat", Some(ft2000()), Via::Pc).await, StatusCode::OK);
    let post = |body: Value, via: Via| send(&s.state, Method::POST, "/api/v1/cat/profiles", Some(body), via);

    status_is(&post(json!({ "action": "save", "name": "FT-2000 casa" }), Via::Network).await, StatusCode::FORBIDDEN);
    let v = body_json(post(json!({ "action": "save", "name": "  FT-2000   casa " }), Via::Pc).await).await;
    assert_eq!(v["profiles"][0]["name"], "FT-2000 casa");
    assert_eq!(v["active_profile"], "FT-2000 casa");

    // un'altra configurazione viva, poi il profilo la riporta com'era e scollega
    status_is(&send(&s.state, Method::PUT, "/api/v1/cat", Some(json!({ "model": 1 })), Via::Pc).await, StatusCode::OK);
    status_is(&send(&s.state, Method::POST, "/api/v1/cat/connect", None, Via::Pc).await, StatusCode::OK);
    wait_state(&s.state, "connected").await;
    let v = body_json(post(json!({ "action": "load", "name": "ft-2000 CASA" }), Via::Pc).await).await;
    assert_eq!(v["config"]["model"], 1029);
    assert_eq!(v["status"]["state"], "disconnected", "caricare un profilo scollega e non ricollega");

    let v = body_json(post(json!({ "action": "new" }), Via::Pc).await).await;
    assert_eq!(v["profiles"].as_array().unwrap().len(), 2);
    assert_eq!(v["config"]["model"], 0);

    let resp = post(json!({ "action": "load", "name": "non c'e'" }), Via::Pc).await;
    status_is(&resp, StatusCode::NOT_FOUND);
    assert_eq!(body_json(resp).await["error"], "no_such_profile");
    let resp = post(json!({ "action": "save", "name": "   " }), Via::Pc).await;
    assert_eq!(body_json(resp).await["error"], "bad_name");
    let resp = post(json!({ "action": "boh" }), Via::Pc).await;
    assert_eq!(body_json(resp).await["error"], "bad_action");

    let v = body_json(post(json!({ "action": "delete", "name": "FT-2000 casa" }), Via::Pc).await).await;
    assert_eq!(v["profiles"].as_array().unwrap().len(), 1);
}

#[tokio::test]
async fn the_hamlib_models_are_listed() {
    let s = setup("models", true);
    let v = get(&s.state, "/api/v1/cat/models").await;
    let models = v["models"].as_array().unwrap();
    assert_eq!(models.len(), 2);
    assert!(models.iter().any(|m| m["id"] == 1029 && m["manufacturer"] == "Yaesu" && m["model"] == "FT-2000"));
}

#[tokio::test]
async fn ports_and_detection_read_the_system_only_and_detection_is_local() {
    let s = setup("system", true);
    // le porte si leggono da chiunque sia associato; l'esito dipende dal PC, ma la forma e' sempre la stessa
    let resp = send(&s.state, Method::GET, "/api/v1/cat/ports", None, Via::Network).await;
    if resp.status() == StatusCode::OK {
        let v = body_json(resp).await;
        for p in v["ports"].as_array().unwrap() {
            assert!(p["port"].as_str().unwrap().starts_with("COM"));
        }
    } else {
        assert_eq!(resp.status(), StatusCode::BAD_GATEWAY);
    }
    status_is(&send(&s.state, Method::POST, "/api/v1/cat/detect", None, Via::Network).await, StatusCode::FORBIDDEN);
    let resp = send(&s.state, Method::POST, "/api/v1/cat/detect", None, Via::Pc).await;
    if resp.status() == StatusCode::OK {
        let v = body_json(resp).await;
        assert!(v["candidates"].is_array() && v["audio_devices"].is_array());
        // nessuna porta e' stata aperta: la radio finta non ha visto nessun comando
        assert!(s.fake.commands().is_empty());
    } else {
        assert_eq!(resp.status(), StatusCode::BAD_GATEWAY);
    }
}
