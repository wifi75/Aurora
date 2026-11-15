//! Le rotte delle macro dei modi testuali che parlano col core: WinKeyer (`<WKTEXT>`, `<WKHEX>`) e audio in trasmissione (`<WAVE>`,
//! `<DTMF>`, `<TUNE>`).
mod common;

use std::sync::Arc;

use axum::http::{Method, StatusCode};
use common::*;
use remote_api::AppState;

fn with_digi() -> (AppState, std::path::PathBuf) {
    let dir = std::env::temp_dir().join(format!("aurora-digi-macro-{}-{}", std::process::id(), rand_suffix()));
    std::fs::create_dir_all(&dir).unwrap();
    let svc = Arc::new(mode_digi::DigiService::open(&dir));
    (state_with_live_tx(true).with_digi(svc), dir)
}

fn rand_suffix() -> u128 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()
}

async fn post(s: &AppState, uri: &str, body: &str) -> axum::response::Response {
    let cookie = cookie_for(s);
    call(&router(s), req(Method::POST, uri).cookie(&cookie).json(body).via_loopback()).await
}

#[tokio::test]
async fn play_needs_something_to_send() {
    let (s, dir) = with_digi();
    let r = post(&s, "/api/v1/digi/play", "{}").await;
    status_is(&r, StatusCode::BAD_REQUEST);
    let r = post(&s, "/api/v1/digi/play", r#"{"dtmf":"xyz"}"#).await;
    status_is(&r, StatusCode::BAD_REQUEST);
    let r = post(&s, "/api/v1/digi/play", r#"{"wave":"..\\secret.wav"}"#).await;
    status_is(&r, StatusCode::BAD_REQUEST);
    let _ = std::fs::remove_dir_all(dir);
}

#[tokio::test]
async fn play_without_transmitter_is_a_conflict() {
    let (s, dir) = with_digi();
    let r = post(&s, "/api/v1/digi/play", r#"{"dtmf":"123#"}"#).await;
    status_is(&r, StatusCode::CONFLICT);
    let r = post(&s, "/api/v1/digi/play", r#"{"tune_s":2}"#).await;
    status_is(&r, StatusCode::CONFLICT);
    let _ = std::fs::remove_dir_all(dir);
}

#[tokio::test]
async fn play_is_only_from_the_pc() {
    let (s, dir) = with_digi();
    let cookie = cookie_for(&s);
    let r = call(&router(&s), req(Method::POST, "/api/v1/digi/play").cookie(&cookie).json(r#"{"dtmf":"1"}"#)).await;
    status_is(&r, StatusCode::FORBIDDEN);
    let _ = std::fs::remove_dir_all(dir);
}

#[tokio::test]
async fn winkeyer_send_checks_the_body_and_the_port() {
    let (s, dir) = with_digi();
    let r = post(&s, "/api/v1/digi/winkeyer/send", "{}").await;
    status_is(&r, StatusCode::BAD_REQUEST);
    let r = post(&s, "/api/v1/digi/winkeyer/send", r#"{"hex":"ZZ"}"#).await;
    status_is(&r, StatusCode::BAD_REQUEST);
    // senza porta del WinKeyer nelle impostazioni: errore della trasmissione
    let r = post(&s, "/api/v1/digi/winkeyer/send", r#"{"text":"TEST"}"#).await;
    status_is(&r, StatusCode::BAD_GATEWAY);
    let _ = std::fs::remove_dir_all(dir);
}

#[tokio::test]
async fn record_starts_stops_and_replays() {
    let (s, dir) = with_digi();
    let r = post(&s, "/api/v1/digi/record", r#"{"action":"dance"}"#).await;
    status_is(&r, StatusCode::BAD_REQUEST);
    let r = post(&s, "/api/v1/digi/record", r#"{"action":"start","name":"../x"}"#).await;
    status_is(&r, StatusCode::BAD_REQUEST);
    let r = post(&s, "/api/v1/digi/record", r#"{"action":"start","name":"prova"}"#).await;
    status_is(&r, StatusCode::OK);
    assert!(body_json(r).await["file"].as_str().unwrap().ends_with("prova.wav"));
    let r = post(&s, "/api/v1/digi/record", r#"{"action":"stop"}"#).await;
    status_is(&r, StatusCode::OK);
    assert!(dir.join("recordings").join("prova.wav").is_file());
    let r = post(&s, "/api/v1/digi/record", r#"{"action":"replay","seconds":5}"#).await;
    assert_eq!(body_json(r).await["samples"], 0);
    let _ = std::fs::remove_dir_all(dir);
}

#[tokio::test]
async fn without_digi_service_the_routes_are_not_found() {
    let s = state_with_live_tx(true);
    let r = post(&s, "/api/v1/digi/winkeyer/send", r#"{"hex":"0x0A"}"#).await;
    status_is(&r, StatusCode::NOT_FOUND);
}
