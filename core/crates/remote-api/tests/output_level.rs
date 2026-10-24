mod common;

use std::sync::{Arc, Mutex};

use auth::{DeviceStore, SystemClock};
use aurora_core::Bus;
use axum::http::{Method, StatusCode};
use common::*;
use logbook::Logbook;
use remote_api::{AppState, TxPolicy};
use rig_control::{Rig, RigError};

struct FakeRig {
    attenuation: Mutex<u32>,
    fail: bool,
}

impl Rig for FakeRig {
    fn set_frequency(&self, _: u64) -> Result<(), RigError> {
        Ok(())
    }
    fn set_rx_frequency(&self, _: u32) -> Result<(), RigError> {
        Ok(())
    }
    fn set_mode(&self, _: &str) -> Result<(), RigError> {
        Ok(())
    }
    fn output_attenuation(&self) -> Result<u32, RigError> {
        if self.fail {
            return Err(RigError("JTDX non risponde".into()));
        }
        Ok(*self.attenuation.lock().unwrap())
    }
    fn set_output_attenuation(&self, tenths: u32) -> Result<(), RigError> {
        if self.fail {
            return Err(RigError("JTDX non risponde".into()));
        }
        *self.attenuation.lock().unwrap() = tenths;
        Ok(())
    }
}

fn state_with(rig: Option<Arc<FakeRig>>, live_tx: bool) -> AppState {
    let auth = Arc::new(DeviceStore::open_in_memory(Arc::new(SystemClock)).unwrap());
    let book = Arc::new(Logbook::open_in_memory().unwrap());
    let s = AppState::new(Bus::new(16), auth, book, TxPolicy::default_safe().with_live_tx(live_tx), false);
    match rig {
        Some(r) => s.with_rig(r),
        None => s,
    }
}

fn rig(tenths: u32, fail: bool) -> Arc<FakeRig> {
    Arc::new(FakeRig { attenuation: Mutex::new(tenths), fail })
}

async fn get(s: &AppState) -> axum::response::Response {
    let cookie = cookie_for(s);
    call(&router(s), req(Method::GET, "/api/v1/output-level").cookie(&cookie)).await
}

async fn put(s: &AppState, body: &str, cloudflare: bool) -> axum::response::Response {
    let cookie = cookie_for(s);
    let mut r = req(Method::PUT, "/api/v1/output-level").cookie(&cookie).json(body);
    if cloudflare {
        r = r.header("cf-ray", "x");
    }
    call(&router(s), r).await
}

#[tokio::test]
async fn needs_a_paired_device() {
    let s = state_with(Some(rig(0, false)), true);
    status_is(&call(&router(&s), req(Method::GET, "/api/v1/output-level")).await, StatusCode::UNAUTHORIZED);
    status_is(&call(&router(&s), req(Method::PUT, "/api/v1/output-level").json("{}")).await, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn reads_the_attenuation_in_db_with_its_limits() {
    let s = state_with(Some(rig(392, false)), false);
    let j = body_json(get(&s).await).await;
    assert_eq!(j["attenuation_db"], 39.2);
    assert_eq!(j["min_db"], 0.0);
    assert_eq!(j["max_db"], 45.0);
}

#[tokio::test]
async fn sets_the_attenuation_in_tenths_of_a_db() {
    let r = rig(392, false);
    let s = state_with(Some(r.clone()), true);
    status_is(&put(&s, r#"{"attenuation_db":12.5}"#, false).await, StatusCode::OK);
    assert_eq!(*r.attenuation.lock().unwrap(), 125);
}

#[tokio::test]
async fn the_level_can_be_changed_with_transmission_closed_but_not_from_a_refused_transport() {
    let r = rig(392, false);
    let closed = state_with(Some(r.clone()), false);
    status_is(&put(&closed, r#"{"attenuation_db":5}"#, false).await, StatusCode::OK);
    assert_eq!(*r.attenuation.lock().unwrap(), 50, "come in JTDX, anche a trasmissione chiusa");
    let resp = put(&closed, r#"{"attenuation_db":20}"#, true).await;
    status_is(&resp, StatusCode::FORBIDDEN);
    assert_eq!(body_json(resp).await["error"], "tx_not_allowed_on_transport");
    assert_eq!(*r.attenuation.lock().unwrap(), 50, "da una via non consentita nulla cambia");
}

#[tokio::test]
async fn values_outside_the_slider_are_refused() {
    let r = rig(392, false);
    let s = state_with(Some(r.clone()), true);
    for bad in ["-1", "45.1", "100", "-0.1"] {
        let resp = put(&s, &format!(r#"{{"attenuation_db":{bad}}}"#), false).await;
        status_is(&resp, StatusCode::BAD_REQUEST);
        assert_eq!(body_json(resp).await["error"], "bad_level", "{bad}");
    }
    assert_eq!(*r.attenuation.lock().unwrap(), 392);
}

#[tokio::test]
async fn failures_and_a_missing_rig_are_reported() {
    let s = state_with(Some(rig(0, true)), true);
    status_is(&get(&s).await, StatusCode::BAD_GATEWAY);
    status_is(&put(&s, r#"{"attenuation_db":5}"#, false).await, StatusCode::BAD_GATEWAY);
    let none = state_with(None, true);
    status_is(&get(&none).await, StatusCode::SERVICE_UNAVAILABLE);
    status_is(&put(&none, r#"{"attenuation_db":5}"#, false).await, StatusCode::SERVICE_UNAVAILABLE);
}
