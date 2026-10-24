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
}

fn state_with_rig(fail: bool) -> (AppState, Arc<FakeRig>) {
    let rig = Arc::new(FakeRig { calls: Mutex::new(vec![]), fail });
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
