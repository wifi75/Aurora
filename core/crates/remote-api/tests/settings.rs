mod common;

use std::sync::{
    atomic::{AtomicU32, Ordering},
    Arc, Mutex,
};

use auth::{DeviceStore, SystemClock};
use aurora_core::Bus;
use axum::http::{Method, StatusCode};
use common::*;
use logbook::Logbook;
use remote_api::{AppState, TxPolicy};
use rig_control::{Rig, RigError};

struct FakeRig(Mutex<Vec<u64>>);

impl Rig for FakeRig {
    fn set_frequency(&self, hz: u64) -> Result<(), RigError> {
        self.0.lock().unwrap().push(hz);
        Ok(())
    }
    fn set_rx_frequency(&self, _: u32) -> Result<(), RigError> {
        Ok(())
    }
    fn output_attenuation(&self) -> Result<u32, RigError> {
        Ok(0)
    }
    fn set_output_attenuation(&self, _: u32) -> Result<(), RigError> {
        Ok(())
    }
    fn set_mode(&self, _: &str) -> Result<(), RigError> {
        Ok(())
    }
}

fn state_with(rig: Option<Arc<FakeRig>>, saved: Arc<AtomicU32>) -> AppState {
    let auth = Arc::new(DeviceStore::open_in_memory(Arc::new(SystemClock)).unwrap());
    let book = Arc::new(Logbook::open_in_memory().unwrap());
    let mut s = AppState::new(Bus::new(16), auth, book, TxPolicy::default_safe(), false)
        .with_settings_saver(Arc::new(move || {
            saved.fetch_add(1, Ordering::SeqCst);
        }));
    if let Some(r) = rig {
        s = s.with_rig(r);
    }
    s
}

async fn send(s: &AppState, method: Method, path: &str, body: Option<&str>) -> axum::response::Response {
    let cookie = cookie_for(s);
    let mut r = req(method, path).cookie(&cookie);
    if let Some(b) = body {
        r = r.json(b);
    }
    call(&router(s), r).await
}

#[tokio::test]
async fn needs_a_paired_device() {
    let s = state_with(None, Arc::default());
    for path in ["/api/v1/frequencies", "/api/v1/schedule"] {
        status_is(&call(&router(&s), req(Method::GET, path)).await, StatusCode::UNAUTHORIZED);
        status_is(&call(&router(&s), req(Method::PUT, path).json("{}")).await, StatusCode::UNAUTHORIZED);
    }
}

#[tokio::test]
async fn frequencies_start_with_the_defaults_and_can_be_replaced_and_saved() {
    let saved = Arc::new(AtomicU32::new(0));
    let s = state_with(None, saved.clone());
    let list = body_json(send(&s, Method::GET, "/api/v1/frequencies", None).await).await;
    assert!(list.as_array().unwrap().iter().any(|e| e["band"] == "20m" && e["mode"] == "FT8" && e["hz"] == 14_074_000));

    let put = r#"[{"band":"20m","mode":"FT8","hz":14075000}]"#;
    status_is(&send(&s, Method::PUT, "/api/v1/frequencies", Some(put)).await, StatusCode::OK);
    assert_eq!(saved.load(Ordering::SeqCst), 1);
    let list = body_json(send(&s, Method::GET, "/api/v1/frequencies", None).await).await;
    assert_eq!(list.as_array().unwrap().len(), 1);
    assert_eq!(list[0]["hz"], 14_075_000);
}

#[tokio::test]
async fn an_invalid_frequency_table_is_refused_and_nothing_is_saved() {
    let saved = Arc::new(AtomicU32::new(0));
    let s = state_with(None, saved.clone());
    let resp = send(&s, Method::PUT, "/api/v1/frequencies", Some(r#"[{"band":"20m","mode":"FT8","hz":7074000}]"#)).await;
    status_is(&resp, StatusCode::BAD_REQUEST);
    assert_eq!(body_json(resp).await["error"], "bad_frequencies");
    assert_eq!(saved.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn the_band_command_uses_the_user_table_for_the_current_mode() {
    let rig = Arc::new(FakeRig(Mutex::new(vec![])));
    let s = state_with(Some(rig.clone()), Arc::default());
    send(&s, Method::PUT, "/api/v1/frequencies", Some(r#"[{"band":"20m","mode":"FT8","hz":14075000},{"band":"20m","mode":"FT4","hz":14081000}]"#)).await;
    status_is(&send(&s, Method::POST, "/api/v1/commands/band", Some(r#"{"band":"20m"}"#)).await, StatusCode::OK);
    status_is(&send(&s, Method::POST, "/api/v1/commands/band", Some(r#"{"band":"20m","mode":"FT4"}"#)).await, StatusCode::OK);
    assert_eq!(*rig.0.lock().unwrap(), vec![14_075_000, 14_081_000]);
    let none = send(&s, Method::POST, "/api/v1/commands/band", Some(r#"{"band":"40m"}"#)).await;
    status_is(&none, StatusCode::BAD_REQUEST);
    assert_eq!(body_json(none).await["error"], "bad_band", "una banda senza frequenza nella tabella non si sceglie");
}

#[tokio::test]
async fn the_schedule_starts_off_and_empty_and_can_be_set_and_saved() {
    let saved = Arc::new(AtomicU32::new(0));
    let s = state_with(None, saved.clone());
    let j = body_json(send(&s, Method::GET, "/api/v1/schedule", None).await).await;
    assert_eq!(j["enabled"], false);
    assert_eq!(j["entries"].as_array().unwrap().len(), 0);

    let put = r#"{"enabled":true,"entries":[{"hour":6,"minute":30,"band":"20m"}]}"#;
    status_is(&send(&s, Method::PUT, "/api/v1/schedule", Some(put)).await, StatusCode::OK);
    assert_eq!(saved.load(Ordering::SeqCst), 1);
    let j = body_json(send(&s, Method::GET, "/api/v1/schedule", None).await).await;
    assert_eq!(j["enabled"], true);
    assert_eq!(j["entries"][0]["band"], "20m");
}

#[tokio::test]
async fn an_invalid_schedule_is_refused() {
    let saved = Arc::new(AtomicU32::new(0));
    let s = state_with(None, saved.clone());
    let resp = send(&s, Method::PUT, "/api/v1/schedule", Some(r#"{"enabled":true,"entries":[{"hour":25,"minute":0,"band":"20m"}]}"#)).await;
    status_is(&resp, StatusCode::BAD_REQUEST);
    assert_eq!(body_json(resp).await["error"], "bad_schedule");
    assert_eq!(saved.load(Ordering::SeqCst), 0);
}
