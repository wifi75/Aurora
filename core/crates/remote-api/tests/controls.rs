mod common;

use std::sync::{Arc, Mutex};

use auth::{DeviceStore, SystemClock};
use aurora_core::Bus;
use axum::http::{Method, StatusCode};
use common::*;
use logbook::Logbook;
use remote_api::{AppState, TxPolicy};
use rig_control::{Rig, RigError};

#[derive(Default)]
struct FakeRig {
    pressed: Mutex<Vec<(String, Option<bool>)>>,
    selected: Mutex<Vec<u8>>,
    fail: bool,
    with_tx: bool,
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
        Ok(0)
    }
    fn set_output_attenuation(&self, _: u32) -> Result<(), RigError> {
        Ok(())
    }
    fn controls(&self) -> Result<Vec<(String, bool)>, RigError> {
        if self.fail {
            return Err(RigError("JTDX non risponde".into()));
        }
        Ok(vec![("autoseq".into(), true), ("enable_tx".into(), false)])
    }
    fn controls_and_tx(&self) -> Result<rig_control::controls::Reading, RigError> {
        let switches = self.controls()?;
        let tx = self.with_tx.then(|| rig_control::controls::TxState {
            messages: vec!["M7SZY IU3ABC IO91".into(), "M7SZY IU3ABC -07".into(), String::new(), String::new(), String::new(), "CQ DX IU3ABC IO91".into()],
            selected: Some(2),
            minute_label: "TX 15/45".into(),
        });
        Ok((switches, tx))
    }
    fn select_tx_message(&self, n: u8) -> Result<(), RigError> {
        if self.fail {
            return Err(RigError("JTDX non risponde".into()));
        }
        self.selected.lock().unwrap().push(n);
        Ok(())
    }
    fn set_control(&self, id: &str, on: Option<bool>) -> Result<(), RigError> {
        if self.fail {
            return Err(RigError("JTDX non risponde".into()));
        }
        self.pressed.lock().unwrap().push((id.to_string(), on));
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

async fn get(s: &AppState) -> axum::response::Response {
    let cookie = cookie_for(s);
    call(&router(s), req(Method::GET, "/api/v1/controls").cookie(&cookie)).await
}

async fn press(s: &AppState, body: &str) -> axum::response::Response {
    let cookie = cookie_for(s);
    call(&router(s), req(Method::POST, "/api/v1/commands/control").cookie(&cookie).json(body)).await
}

#[tokio::test]
async fn needs_a_paired_device() {
    let s = state_with(Some(Arc::new(FakeRig::default())), true);
    status_is(&call(&router(&s), req(Method::GET, "/api/v1/controls")).await, StatusCode::UNAUTHORIZED);
    status_is(&call(&router(&s), req(Method::POST, "/api/v1/commands/control").json("{}")).await, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn lists_the_state_of_every_control() {
    let s = state_with(Some(Arc::new(FakeRig::default())), false);
    let r = get(&s).await;
    status_is(&r, StatusCode::OK);
    let v = body_json(r).await;
    assert_eq!(v["controls"]["autoseq"], true);
    assert_eq!(v["controls"]["enable_tx"], false);
}

#[tokio::test]
async fn returns_the_six_tx_messages_and_the_selected_one_when_the_rig_reads_them() {
    let rig = FakeRig { with_tx: true, ..FakeRig::default() };
    let s = state_with(Some(Arc::new(rig)), false);
    let v = body_json(get(&s).await).await;
    assert_eq!(v["tx"]["selected"], 2);
    assert_eq!(v["tx"]["messages"][1], "M7SZY IU3ABC -07");
    assert_eq!(v["tx"]["messages"][5], "CQ DX IU3ABC IO91");
    assert_eq!(v["tx"]["minute"], "TX 15/45");
}

#[tokio::test]
async fn tx_is_null_when_the_rig_cannot_read_the_messages() {
    let s = state_with(Some(Arc::new(FakeRig::default())), false);
    let v = body_json(get(&s).await).await;
    assert!(v["tx"].is_null());
}

#[tokio::test]
async fn without_a_rig_it_is_unavailable_and_a_failing_jtdx_is_a_bad_gateway() {
    let none = state_with(None, true);
    status_is(&get(&none).await, StatusCode::SERVICE_UNAVAILABLE);
    status_is(&press(&none, r#"{"id":"autoseq","on":true}"#).await, StatusCode::SERVICE_UNAVAILABLE);
    let bad = state_with(Some(Arc::new(FakeRig { fail: true, ..Default::default() })), true);
    status_is(&get(&bad).await, StatusCode::BAD_GATEWAY);
    status_is(&press(&bad, r#"{"id":"autoseq","on":true}"#).await, StatusCode::BAD_GATEWAY);
}

#[tokio::test]
async fn a_safe_toggle_works_even_with_transmission_closed() {
    let rig = Arc::new(FakeRig::default());
    let s = state_with(Some(rig.clone()), false);
    status_is(&press(&s, r#"{"id":"autoseq","on":false}"#).await, StatusCode::OK);
    assert_eq!(*rig.pressed.lock().unwrap(), vec![("autoseq".to_string(), Some(false))]);
}

#[tokio::test]
async fn arming_a_transmission_control_needs_transmission_open() {
    let rig = Arc::new(FakeRig::default());
    let closed = state_with(Some(rig.clone()), false);
    for id in ["enable_tx", "auto_tx"] {
        let r = press(&closed, &format!(r#"{{"id":"{id}","on":true}}"#)).await;
        status_is(&r, StatusCode::FORBIDDEN);
    }
    assert!(rig.pressed.lock().unwrap().is_empty());
    // spegnerli e' sempre permesso
    status_is(&press(&closed, r#"{"id":"enable_tx","on":false}"#).await, StatusCode::OK);
    let open = state_with(Some(rig.clone()), true);
    status_is(&press(&open, r#"{"id":"enable_tx","on":true}"#).await, StatusCode::OK);
}

#[tokio::test]
async fn arming_is_refused_from_cloudflare_even_when_open() {
    let rig = Arc::new(FakeRig::default());
    let s = state_with(Some(rig.clone()), true);
    let cookie = cookie_for(&s);
    let r = call(&router(&s), req(Method::POST, "/api/v1/commands/control").cookie(&cookie).header("cf-ray", "x").json(r#"{"id":"auto_tx","on":true}"#)).await;
    status_is(&r, StatusCode::FORBIDDEN);
    assert!(rig.pressed.lock().unwrap().is_empty());
}

#[tokio::test]
async fn rejects_unknown_controls_and_wrong_arguments() {
    let rig = Arc::new(FakeRig::default());
    let s = state_with(Some(rig.clone()), true);
    for body in [
        r#"{"id":"tune","on":true}"#,
        r#"{"id":"","on":true}"#,
        r#"{"id":"autoseq"}"#,                 // un interruttore vuole on
        r#"{"id":"clear_dx","on":true}"#,      // un pulsante semplice non ha stato
    ] {
        status_is(&press(&s, body).await, StatusCode::BAD_REQUEST);
    }
    assert!(rig.pressed.lock().unwrap().is_empty());
    status_is(&press(&s, r#"{"id":"clear_dx"}"#).await, StatusCode::OK);
    assert_eq!(*rig.pressed.lock().unwrap(), vec![("clear_dx".to_string(), None)]);
}

async fn pick(s: &AppState, body: &str) -> axum::response::Response {
    let cookie = cookie_for(s);
    call(&router(s), req(Method::POST, "/api/v1/commands/tx-message").cookie(&cookie).json(body)).await
}

#[tokio::test]
async fn selects_a_tx_message_even_with_transmission_closed() {
    let rig = Arc::new(FakeRig::default());
    let s = state_with(Some(rig.clone()), false);
    let r = pick(&s, r#"{"n":2}"#).await;
    status_is(&r, StatusCode::OK);
    assert_eq!(body_json(r).await["n"], 2);
    assert_eq!(*rig.selected.lock().unwrap(), vec![2]);
}

#[tokio::test]
async fn refuses_a_tx_message_outside_one_to_six_and_needs_a_paired_device() {
    let rig = Arc::new(FakeRig::default());
    let s = state_with(Some(rig.clone()), true);
    for bad in [r#"{"n":0}"#, r#"{"n":7}"#, r#"{"n":255}"#] {
        status_is(&pick(&s, bad).await, StatusCode::BAD_REQUEST);
    }
    status_is(&call(&router(&s), req(Method::POST, "/api/v1/commands/tx-message").json(r#"{"n":1}"#)).await, StatusCode::UNAUTHORIZED);
    assert!(rig.selected.lock().unwrap().is_empty());
}

#[tokio::test]
async fn tx_message_without_a_rig_or_with_a_failing_jtdx_is_unavailable_or_a_bad_gateway() {
    status_is(&pick(&state_with(None, true), r#"{"n":3}"#).await, StatusCode::SERVICE_UNAVAILABLE);
    let failing = FakeRig { fail: true, ..FakeRig::default() };
    status_is(&pick(&state_with(Some(Arc::new(failing)), true), r#"{"n":3}"#).await, StatusCode::BAD_GATEWAY);
}
