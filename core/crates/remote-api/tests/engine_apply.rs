mod common;

use std::{
    collections::BTreeMap,
    sync::{Arc, Mutex},
    time::Duration,
};

use aurora_core::{Event, StationStatus};
use axum::http::{Method, StatusCode};
use common::*;
use remote_api::AppState;

type Seen = Arc<Mutex<Vec<BTreeMap<String, String>>>>;

fn with_applier(fail: bool) -> (AppState, Seen) {
    let seen: Seen = Arc::default();
    let log = Arc::clone(&seen);
    let applier = Arc::new(move |changes: BTreeMap<String, String>| {
        log.lock().unwrap().push(changes);
        if fail { Err("rotto".to_string()) } else { Ok(()) }
    });
    (state().with_engine_apply_if(Some(applier)), seen)
}

async fn post(s: &AppState, body: &str, proxied: bool) -> axum::response::Response {
    let cookie = cookie_for(s);
    let mut r = req(Method::POST, "/api/v1/engine-config").cookie(&cookie).json(body);
    if proxied {
        r = r.header("x-forwarded-for", "1.2.3.4");
    }
    call(&router(s), r).await
}

#[tokio::test]
async fn needs_a_paired_device() {
    let (s, _) = with_applier(false);
    status_is(&call(&router(&s), req(Method::POST, "/api/v1/engine-config").json(r#"{"changes":{"MyCall":"AA1AA"}}"#)).await, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn applies_valid_changes_from_the_home_network() {
    let (s, seen) = with_applier(false);
    status_is(&post(&s, r#"{"changes":{"MyCall":"IU3ABC","NDepth":"2","SWLMode":"true"}}"#, false).await, StatusCode::OK);
    let got = seen.lock().unwrap();
    assert_eq!(got.len(), 1);
    assert_eq!(got[0]["MyCall"], "IU3ABC");
    assert_eq!(got[0]["NDepth"], "2");
}

#[tokio::test]
async fn refuses_unknown_keys_bad_values_and_empty_requests_without_touching_anything() {
    let (s, seen) = with_applier(false);
    for body in [
        r#"{"changes":{"EQSLPasswd":"x"}}"#,
        r#"{"changes":{"SWLMode":"si"}}"#,
        r#"{"changes":{"NDepth":"9"}}"#,
        r#"{"changes":{"MyCall":"A\"B"}}"#,
        r#"{"changes":{}}"#,
    ] {
        let resp = post(&s, body, false).await;
        status_is(&resp, StatusCode::BAD_REQUEST);
        assert_eq!(body_json(resp).await["error"], "bad_setting", "{body}");
    }
    assert!(seen.lock().unwrap().is_empty());
}

#[tokio::test]
async fn is_refused_from_outside_the_home_network_and_while_transmitting() {
    let (s, seen) = with_applier(false);
    let resp = post(&s, r#"{"changes":{"MyCall":"IU3ABC"}}"#, true).await;
    status_is(&resp, StatusCode::FORBIDDEN);
    assert_eq!(body_json(resp).await["error"], "local_only");

    s.bus.events().send(Event::Status(StationStatus { transmitting: true, ..StationStatus::default() })).unwrap();
    tokio::time::sleep(Duration::from_millis(150)).await;
    let resp = post(&s, r#"{"changes":{"MyCall":"IU3ABC"}}"#, false).await;
    status_is(&resp, StatusCode::CONFLICT);
    assert_eq!(body_json(resp).await["error"], "transmitting");
    assert!(seen.lock().unwrap().is_empty(), "JTDX non si tocca mentre trasmette");
}

#[tokio::test]
async fn a_failure_of_the_applier_is_reported_and_a_missing_one_says_so() {
    let (s, _) = with_applier(true);
    let resp = post(&s, r#"{"changes":{"MyCall":"IU3ABC"}}"#, false).await;
    status_is(&resp, StatusCode::BAD_GATEWAY);
    assert_eq!(body_json(resp).await["error"], "engine_apply_failed");
    let resp = post(&state(), r#"{"changes":{"MyCall":"IU3ABC"}}"#, false).await;
    status_is(&resp, StatusCode::NOT_FOUND);
}
