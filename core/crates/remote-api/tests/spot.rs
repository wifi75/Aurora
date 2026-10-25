mod common;

use std::{
    io::{Read, Write},
    net::TcpListener,
    sync::{mpsc, Arc},
    time::Duration,
};

use aurora_core::{Bus, Event, StationStatus};
use axum::http::{Method, StatusCode};
use common::*;
use remote_api::{engine_settings::EngineSettingsStore, AppState};
use reporting::{secret::MemorySecrets, secret::SecretStore, Config, Endpoints, Service};

/// Un server HTTP locale (un thread): manda al canale la prima riga della richiesta e risponde `status_line`.
fn local_server(status_line: &'static str) -> (String, mpsc::Receiver<String>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("server");
    let addr = listener.local_addr().expect("indirizzo");
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        while let Ok((mut stream, _)) = listener.accept() {
            let mut buf = vec![0u8; 8192];
            let n = stream.read(&mut buf).unwrap_or(0);
            let text = String::from_utf8_lossy(&buf[..n]).to_string();
            let _ = tx.send(text.lines().next().unwrap_or("").to_string());
            let _ = stream.write_all(format!("HTTP/1.1 {status_line}\r\nContent-Length: 2\r\nConnection: close\r\n\r\nok").as_bytes());
        }
    });
    (format!("http://{addr}/SendSpot.aspx"), rx)
}

/// Un'applicazione con il motore proprio, il servizio degli invii (verso il server locale) e il motore che dice dove e' sintonizzato.
async fn with_service(name: &str, url: String, dxsummit: bool) -> AppState {
    let dir = std::env::temp_dir().join(format!("aurora-route-spot-{name}-{}", std::process::id()));
    std::fs::remove_dir_all(&dir).ok();
    let store = Arc::new(EngineSettingsStore::open(&dir.join("settings-motore.json"), remote_api::engine_settings::defaults));
    let bus = Bus::new(64);
    let secrets: Arc<dyn SecretStore> = Arc::new(MemorySecrets::default());
    let endpoints = Endpoints { dxsummit_url: url, dxsummit_timeout: Duration::from_secs(5), ..Endpoints::default() };
    let service = Service::spawn(&bus, store.status(), secrets, endpoints, Arc::new(|| 1_792_860_405), Config { call: "IU3ABC".into(), grid: "JN45".into(), dxsummit, ..Config::default() });
    store.set_reporting(service);
    bus.events().send(Event::Status(StationStatus { dial_hz: 14_074_000, mode: "FT8".into(), rx_df: Some(1200), tx_df: Some(800), report: Some("-07".into()), ..StationStatus::default() })).expect("evento");
    tokio::time::sleep(Duration::from_millis(150)).await;
    state().with_engine_settings(store)
}

fn spot(s: &AppState, cookie: &str, body: &str) -> Req {
    let _ = s;
    req(Method::POST, "/api/v1/spot/dxsummit").cookie(cookie).json(body)
}

#[tokio::test]
async fn needs_a_paired_device_the_native_engine_and_the_home_network() {
    let s = state();
    status_is(&call(&router(&s), req(Method::POST, "/api/v1/spot/dxsummit").json(r#"{"dx_call":"DL1ABC"}"#)).await, StatusCode::UNAUTHORIZED);
    let cookie = cookie_for(&s);
    let resp = call(&router(&s), spot(&s, &cookie, r#"{"dx_call":"DL1ABC"}"#)).await;
    status_is(&resp, StatusCode::NOT_FOUND);
    assert_eq!(body_json(resp).await["error"], "spot_unavailable");
    let (url, requests) = local_server("200 OK");
    let s = with_service("peer", url, true).await;
    let cookie = cookie_for(&s);
    let mut outside = spot(&s, &cookie, r#"{"dx_call":"DL1ABC"}"#);
    outside.peer = axum::extract::ConnectInfo("8.8.8.8:50000".parse().unwrap());
    let resp = call(&router(&s), outside).await;
    status_is(&resp, StatusCode::FORBIDDEN);
    assert_eq!(body_json(resp).await["error"], "local_only");
    assert!(requests.try_recv().is_err(), "da fuori non parte niente");
}

#[tokio::test]
async fn the_spot_goes_to_the_server_with_the_fields_of_jtdx_and_the_same_station_is_not_spotted_twice() {
    let (url, requests) = local_server("200 OK");
    let s = with_service("ok", url, true).await;
    let cookie = cookie_for(&s);
    let resp = call(&router(&s), spot(&s, &cookie, r##"{"dx_call":"dl1abc","dx_grid":"jo62","text":"#R #D","prop":"Es"}"##)).await;
    status_is(&resp, StatusCode::OK);
    let doc = body_json(resp).await;
    assert_eq!((doc["ok"].clone(), doc["dx_call"].clone(), doc["frequency_hz"].clone()), (true.into(), "DL1ABC".into(), 14_075_200.into()), "radio piu' Rx, non Tx");
    let line = requests.recv_timeout(Duration::from_secs(5)).expect("richiesta");
    assert!(line.starts_with("GET /SendSpot.aspx?"), "{line}");
    for part in ["callSign=IU3ABC", "dxCallSign=DL1ABC", "frequency=14075.2", "info=FT8%20Es%20-07dB%20"] {
        assert!(line.contains(part), "{part} in {line}");
    }
    let doc = body_json(call(&router(&s), req(Method::GET, "/api/v1/engine-settings").cookie(&cookie)).await).await;
    assert_eq!(doc["status"]["dxsummit"]["ok"], true);
    // la stessa stazione subito dopo: 409 e niente richiesta
    let again = call(&router(&s), spot(&s, &cookie, r#"{"dx_call":"DL1ABC"}"#)).await;
    status_is(&again, StatusCode::CONFLICT);
    assert_eq!(body_json(again).await["error"], "spot_already");
    assert!(requests.recv_timeout(Duration::from_millis(300)).is_err());
}

#[tokio::test]
async fn off_bad_call_and_server_errors_are_told_in_clear() {
    let (url, requests) = local_server("500 Internal Server Error");
    let s = with_service("err", url.clone(), false).await;
    let cookie = cookie_for(&s);
    let resp = call(&router(&s), spot(&s, &cookie, r#"{"dx_call":"DL1ABC"}"#)).await;
    status_is(&resp, StatusCode::CONFLICT);
    assert_eq!(body_json(resp).await["error"], "spot_off");
    let s = with_service("err2", url, true).await;
    let cookie = cookie_for(&s);
    let resp = call(&router(&s), spot(&s, &cookie, r#"{"dx_call":"A B"}"#)).await;
    status_is(&resp, StatusCode::BAD_REQUEST);
    assert_eq!(body_json(resp).await["error"], "spot_bad_call");
    let resp = call(&router(&s), spot(&s, &cookie, &format!(r#"{{"dx_call":"DL1ABC","text":"{}"}}"#, "x".repeat(200)))).await;
    status_is(&resp, StatusCode::BAD_REQUEST);
    assert_eq!(body_json(resp).await["error"], "bad_spot");
    assert!(requests.try_recv().is_err(), "spento o non valido: niente richiesta");
    let resp = call(&router(&s), spot(&s, &cookie, r#"{"dx_call":"DL1ABC"}"#)).await;
    status_is(&resp, StatusCode::BAD_GATEWAY);
    let doc = body_json(resp).await;
    assert_eq!(doc["error"], "spot_failed");
    assert!(doc["message"].as_str().is_some_and(|m| m.contains("500")), "{doc}");
    // un invio fallito non conta: si puo' riprovare subito (un altro 502, non un 409)
    status_is(&call(&router(&s), spot(&s, &cookie, r#"{"dx_call":"DL1ABC"}"#)).await, StatusCode::BAD_GATEWAY);
}
