mod common;

use std::sync::{Arc, Mutex};

use axum::http::{Method, StatusCode};
use clock_sync::{ntp::Sample, ClockService, ClockSetter, QueryFn};
use common::*;
use remote_api::AppState;

struct Setter {
    calls: Mutex<Vec<f64>>,
    admin: bool,
}

impl ClockSetter for Setter {
    fn adjust(&self, offset_ms: f64) -> Result<(), String> {
        self.calls.lock().unwrap().push(offset_ms);
        if self.admin { Ok(()) } else { Err("serve avviare Aurora come amministratore".into()) }
    }
}

fn clock_state(name: &str, admin: bool) -> (AppState, Arc<Setter>) {
    let dir = std::env::temp_dir().join(format!("aurora-clock-api-{name}-{}", std::process::id()));
    std::fs::remove_dir_all(&dir).ok();
    std::fs::create_dir_all(&dir).unwrap();
    let query: QueryFn = Arc::new(|_: &str| Ok(Sample { offset_ms: 250.0, delay_ms: 12.0, stratum: 2 }));
    let setter = Arc::new(Setter { calls: Mutex::default(), admin });
    let service = ClockService::open(&dir.join("clock.json"), query, setter.clone());
    (state().with_clock(service), setter)
}

async fn send(s: &AppState, method: Method, uri: &str, body: Option<&str>, proxied: bool) -> axum::response::Response {
    let cookie = cookie_for(s);
    let mut r = req(method, uri).cookie(&cookie);
    if let Some(b) = body {
        r = r.json(b);
    }
    if proxied {
        r = r.header("x-forwarded-for", "1.2.3.4");
    }
    call(&router(s), r).await
}

#[tokio::test]
async fn needs_a_paired_device() {
    let (s, _) = clock_state("auth", true);
    status_is(&call(&router(&s), req(Method::GET, "/api/v1/clock")).await, StatusCode::UNAUTHORIZED);
    status_is(&call(&router(&s), req(Method::POST, "/api/v1/clock/check")).await, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn shows_the_configuration_and_measures_on_demand() {
    let (s, _) = clock_state("check", true);
    let v = body_json(send(&s, Method::GET, "/api/v1/clock", None, false).await).await;
    assert_eq!(v["config"]["warn_ms"], 200);
    assert!(v["last"].is_null());
    let m = body_json(send(&s, Method::POST, "/api/v1/clock/check", None, false).await).await;
    assert_eq!(m["offset_ms"], 250.0);
    assert_eq!(m["server"], "it.pool.ntp.org");
    let v = body_json(send(&s, Method::GET, "/api/v1/clock", None, false).await).await;
    assert_eq!(v["last"]["offset_ms"], 250.0);
}

#[tokio::test]
async fn saves_valid_settings_and_refuses_invalid_ones() {
    let (s, _) = clock_state("put", true);
    let ok = r#"{"enabled":true,"servers":["ntp.example.org"],"interval_secs":600,"warn_ms":100,"auto_correct":false}"#;
    status_is(&send(&s, Method::PUT, "/api/v1/clock", Some(ok), false).await, StatusCode::OK);
    let v = body_json(send(&s, Method::GET, "/api/v1/clock", None, false).await).await;
    assert_eq!(v["config"]["servers"][0], "ntp.example.org");
    assert_eq!(v["config"]["interval_secs"], 600);
    let bad = r#"{"enabled":true,"servers":["; calc"],"interval_secs":600,"warn_ms":100,"auto_correct":false}"#;
    let resp = send(&s, Method::PUT, "/api/v1/clock", Some(bad), false).await;
    status_is(&resp, StatusCode::BAD_REQUEST);
    assert_eq!(body_json(resp).await["error"], "bad_clock");
}

#[tokio::test]
async fn automatic_correction_is_switched_on_only_from_the_home_network() {
    let (s, _) = clock_state("auto", true);
    let on = r#"{"enabled":true,"servers":["ntp.example.org"],"interval_secs":600,"warn_ms":100,"auto_correct":true}"#;
    let resp = send(&s, Method::PUT, "/api/v1/clock", Some(on), true).await;
    status_is(&resp, StatusCode::FORBIDDEN);
    assert_eq!(body_json(resp).await["error"], "local_only");
    status_is(&send(&s, Method::PUT, "/api/v1/clock", Some(on), false).await, StatusCode::OK);
}

#[tokio::test]
async fn correcting_the_clock_needs_the_home_network_and_reports_a_missing_admin() {
    let (s, setter) = clock_state("fix", true);
    let resp = send(&s, Method::POST, "/api/v1/clock/correct", None, true).await;
    status_is(&resp, StatusCode::FORBIDDEN);
    assert_eq!(body_json(resp).await["error"], "local_only");
    assert!(setter.calls.lock().unwrap().is_empty(), "nulla e' stato toccato");
    let resp = send(&s, Method::POST, "/api/v1/clock/correct", None, false).await;
    status_is(&resp, StatusCode::OK);
    assert_eq!(*setter.calls.lock().unwrap(), vec![250.0]);

    let (s, _) = clock_state("noadmin", false);
    let resp = send(&s, Method::POST, "/api/v1/clock/correct", None, false).await;
    status_is(&resp, StatusCode::FORBIDDEN);
    assert_eq!(body_json(resp).await["error"], "clock_needs_admin");
}

#[tokio::test]
async fn without_the_service_everything_says_so() {
    let s = state();
    let resp = send(&s, Method::GET, "/api/v1/clock", None, false).await;
    status_is(&resp, StatusCode::NOT_FOUND);
    assert_eq!(body_json(resp).await["error"], "clock_unavailable");
}
