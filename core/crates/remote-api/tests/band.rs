mod common;

use std::{
    sync::{Arc, Mutex},
    time::Duration,
};

use auth::{DeviceStore, SystemClock};
use aurora_core::{Bus, Event, StationStatus};
use axum::http::{Method, StatusCode};
use common::*;
use logbook::Logbook;
use remote_api::{build_router, AppState, TxPolicy};
use rig_control::{Rig, RigError};

struct FakeRig {
    calls: Mutex<Vec<u64>>,
    modes: Mutex<Vec<String>>,
    rx: Mutex<Vec<u32>>,
    fail: bool,
}

impl Rig for FakeRig {
    fn set_frequency(&self, hz: u64) -> Result<(), RigError> {
        if self.fail {
            return Err(RigError("radio non in linea".into()));
        }
        self.calls.lock().unwrap().push(hz);
        Ok(())
    }

    fn set_rx_frequency(&self, hz: u32) -> Result<(), RigError> {
        if self.fail {
            return Err(RigError("JTDX non risponde".into()));
        }
        self.rx.lock().unwrap().push(hz);
        Ok(())
    }

    fn output_attenuation(&self) -> Result<u32, RigError> {
        Ok(0)
    }
    fn set_output_attenuation(&self, _: u32) -> Result<(), RigError> {
        Ok(())
    }
    fn set_mode(&self, mode: &str) -> Result<(), RigError> {
        if self.fail {
            return Err(RigError("JTDX non risponde".into()));
        }
        self.modes.lock().unwrap().push(mode.to_string());
        Ok(())
    }
}

fn state_with_rig(fail: bool) -> (AppState, Arc<FakeRig>) {
    let rig = Arc::new(FakeRig { calls: Mutex::new(vec![]), modes: Mutex::new(vec![]), rx: Mutex::new(vec![]), fail });
    let auth = Arc::new(DeviceStore::open_in_memory(Arc::new(SystemClock)).unwrap());
    let book = Arc::new(Logbook::open_in_memory().unwrap());
    let s = AppState::new(Bus::new(64), auth, book, TxPolicy::default_safe(), false).with_rig(rig.clone());
    (s, rig)
}

async fn band(s: &AppState, body: &str) -> axum::response::Response {
    let cookie = cookie_for(s);
    call(&router(s), req(Method::POST, "/api/v1/commands/band").cookie(&cookie).json(body)).await
}

#[tokio::test]
async fn a_known_band_moves_the_radio_to_its_ft8_frequency() {
    let (s, rig) = state_with_rig(false);
    let resp = band(&s, r#"{"band":"20m"}"#).await;
    status_is(&resp, StatusCode::OK);
    assert_eq!(*rig.calls.lock().unwrap(), vec![14_074_000]);
}

#[tokio::test]
async fn an_unknown_band_is_refused_and_nothing_moves() {
    let (s, rig) = state_with_rig(false);
    let resp = band(&s, r#"{"band":"13m"}"#).await;
    status_is(&resp, StatusCode::BAD_REQUEST);
    assert_eq!(body_json(resp).await["error"], "bad_band");
    assert!(rig.calls.lock().unwrap().is_empty());
}

#[tokio::test]
async fn it_never_changes_band_while_transmitting() {
    let (s, rig) = state_with_rig(false);
    s.bus
        .events()
        .send(Event::Status(StationStatus { transmitting: true, mode: "FT8".into(), ..StationStatus::default() }))
        .unwrap();
    tokio::time::sleep(Duration::from_millis(150)).await;
    let resp = band(&s, r#"{"band":"40m"}"#).await;
    status_is(&resp, StatusCode::CONFLICT);
    assert_eq!(body_json(resp).await["error"], "transmitting");
    assert!(rig.calls.lock().unwrap().is_empty());
}

#[tokio::test]
async fn a_rig_failure_is_reported_as_bad_gateway() {
    let (s, _) = state_with_rig(true);
    let resp = band(&s, r#"{"band":"20m"}"#).await;
    status_is(&resp, StatusCode::BAD_GATEWAY);
    assert_eq!(body_json(resp).await["error"], "rig_error");
}

#[tokio::test]
async fn without_a_rig_the_command_says_so() {
    let s = state();
    let cookie = cookie_for(&s);
    let resp = call(&router(&s), req(Method::POST, "/api/v1/commands/band").cookie(&cookie).json(r#"{"band":"20m"}"#)).await;
    status_is(&resp, StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(body_json(resp).await["error"], "rig_unavailable");
}

#[tokio::test]
async fn it_needs_a_paired_device() {
    let (s, _) = state_with_rig(false);
    let resp = call(&build_router(s), req(Method::POST, "/api/v1/commands/band").json(r#"{"band":"20m"}"#)).await;
    status_is(&resp, StatusCode::UNAUTHORIZED);
}

async fn mode(s: &AppState, body: &str) -> axum::response::Response {
    let cookie = cookie_for(s);
    call(&router(s), req(Method::POST, "/api/v1/commands/mode").cookie(&cookie).json(body)).await
}

#[tokio::test]
async fn a_known_mode_is_handed_to_the_rig_uppercase() {
    let (s, rig) = state_with_rig(false);
    status_is(&mode(&s, r#"{"mode":"ft4"}"#).await, StatusCode::OK);
    assert_eq!(*rig.modes.lock().unwrap(), vec!["FT4".to_string()]);
}

#[tokio::test]
async fn an_unknown_mode_is_refused() {
    let (s, rig) = state_with_rig(false);
    let resp = mode(&s, r#"{"mode":"JT65"}"#).await;
    status_is(&resp, StatusCode::BAD_REQUEST);
    assert_eq!(body_json(resp).await["error"], "bad_mode");
    assert!(rig.modes.lock().unwrap().is_empty());
}

#[tokio::test]
async fn the_mode_is_never_changed_while_transmitting() {
    let (s, rig) = state_with_rig(false);
    s.bus.events().send(Event::Status(StationStatus { transmitting: true, ..StationStatus::default() })).unwrap();
    tokio::time::sleep(Duration::from_millis(150)).await;
    let resp = mode(&s, r#"{"mode":"FT4"}"#).await;
    status_is(&resp, StatusCode::CONFLICT);
    assert!(rig.modes.lock().unwrap().is_empty());
}

#[tokio::test]
async fn mode_failures_and_missing_rig_are_reported() {
    let (s, _) = state_with_rig(true);
    status_is(&mode(&s, r#"{"mode":"FT8"}"#).await, StatusCode::BAD_GATEWAY);
    let none = state();
    status_is(&mode(&none, r#"{"mode":"FT8"}"#).await, StatusCode::SERVICE_UNAVAILABLE);
}

async fn rx(s: &AppState, body: &str) -> axum::response::Response {
    let cookie = cookie_for(s);
    call(&router(s), req(Method::POST, "/api/v1/commands/rx-frequency").cookie(&cookie).json(body)).await
}

#[tokio::test]
async fn the_rx_frequency_is_handed_to_the_rig_within_the_audio_band() {
    let (s, rig) = state_with_rig(false);
    status_is(&rx(&s, r#"{"hz":1000}"#).await, StatusCode::OK);
    assert_eq!(*rig.rx.lock().unwrap(), vec![1000]);
    for hz in [0, 50, 3500, 99999] {
        let resp = rx(&s, &format!(r#"{{"hz":{hz}}}"#)).await;
        status_is(&resp, StatusCode::BAD_REQUEST);
    }
    assert_eq!(rig.rx.lock().unwrap().len(), 1);
}

#[tokio::test]
async fn the_rx_frequency_may_change_while_transmitting() {
    let (s, rig) = state_with_rig(false);
    s.bus.events().send(Event::Status(StationStatus { transmitting: true, ..StationStatus::default() })).unwrap();
    tokio::time::sleep(Duration::from_millis(150)).await;
    status_is(&rx(&s, r#"{"hz":1200}"#).await, StatusCode::OK);
    assert_eq!(*rig.rx.lock().unwrap(), vec![1200]);
}

#[tokio::test]
async fn rx_failures_and_missing_rig_are_reported() {
    let (s, _) = state_with_rig(true);
    status_is(&rx(&s, r#"{"hz":1000}"#).await, StatusCode::BAD_GATEWAY);
    status_is(&rx(&state(), r#"{"hz":1000}"#).await, StatusCode::SERVICE_UNAVAILABLE);
}

async fn publish_status(s: &AppState, dial_hz: u64, mode: &str) {
    s.bus
        .events()
        .send(Event::Status(StationStatus { dial_hz, mode: mode.into(), ..StationStatus::default() }))
        .unwrap();
    tokio::time::sleep(Duration::from_millis(150)).await;
}

#[tokio::test]
async fn changing_mode_keeps_the_band_and_moves_the_dial_to_the_frequency_of_that_mode() {
    let (s, rig) = state_with_rig(false);
    publish_status(&s, 21_074_000, "FT8").await;
    let resp = mode(&s, r#"{"mode":"FT4"}"#).await;
    status_is(&resp, StatusCode::OK);
    assert_eq!(*rig.modes.lock().unwrap(), vec!["FT4".to_string()]);
    assert_eq!(*rig.calls.lock().unwrap(), vec![21_140_000]);
    assert_eq!(body_json(resp).await["dial_hz"], 21_140_000);
}

#[tokio::test]
async fn changing_mode_without_a_known_band_leaves_the_dial_to_jtdx() {
    let (s, rig) = state_with_rig(false);
    status_is(&mode(&s, r#"{"mode":"FT4"}"#).await, StatusCode::OK);
    publish_status(&s, 12_345_678, "FT8").await;
    status_is(&mode(&s, r#"{"mode":"FT8"}"#).await, StatusCode::OK);
    assert!(rig.calls.lock().unwrap().is_empty());
}

async fn frequency(s: &AppState, body: &str) -> axum::response::Response {
    let cookie = cookie_for(s);
    call(&router(s), req(Method::POST, "/api/v1/commands/frequency").cookie(&cookie).json(body)).await
}

#[tokio::test]
async fn an_exact_frequency_inside_an_amateur_band_moves_the_radio() {
    let (s, rig) = state_with_rig(false);
    let resp = frequency(&s, r#"{"hz":7143700}"#).await;
    status_is(&resp, StatusCode::OK);
    assert_eq!(body_json(resp).await["dial_hz"], 7_143_700);
    assert_eq!(*rig.calls.lock().unwrap(), vec![7_143_700]);
}

#[tokio::test]
async fn a_frequency_outside_the_amateur_bands_is_refused() {
    let (s, rig) = state_with_rig(false);
    for hz in [0u64, 100_000, 15_000_000, 99_000_000, 500_000_000] {
        let resp = frequency(&s, &format!(r#"{{"hz":{hz}}}"#)).await;
        status_is(&resp, StatusCode::BAD_REQUEST);
        assert_eq!(body_json(resp).await["error"], "bad_frequency", "{hz}");
    }
    assert!(rig.calls.lock().unwrap().is_empty());
}

#[tokio::test]
async fn the_frequency_is_never_changed_while_transmitting_and_failures_are_reported() {
    let (s, rig) = state_with_rig(false);
    s.bus.events().send(Event::Status(StationStatus { transmitting: true, ..StationStatus::default() })).unwrap();
    tokio::time::sleep(Duration::from_millis(150)).await;
    let resp = frequency(&s, r#"{"hz":14074000}"#).await;
    status_is(&resp, StatusCode::CONFLICT);
    assert!(rig.calls.lock().unwrap().is_empty());
    let (s, _) = state_with_rig(true);
    status_is(&frequency(&s, r#"{"hz":14074000}"#).await, StatusCode::BAD_GATEWAY);
}
