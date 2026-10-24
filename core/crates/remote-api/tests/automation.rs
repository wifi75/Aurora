mod common;

use std::{sync::Arc, time::Duration};

use auth::{DeviceStore, SystemClock};
use aurora_core::Bus;
use axum::http::{Method, StatusCode};
use common::*;
use logbook::Logbook;
use qso_engine::{Automation, AutomationConfig, Limits, Rules};
use remote_api::{build_router, AppState, TxPolicy};

fn state_with_automation(live_allowed: bool) -> (AppState, Automation) {
    let bus = Bus::new(256);
    let auth = Arc::new(DeviceStore::open_in_memory(Arc::new(SystemClock)).unwrap());
    let logbook = Arc::new(Logbook::open_in_memory().unwrap());
    let automation = Automation::spawn(
        &bus,
        AutomationConfig {
            rules: Rules::default(),
            limits: Limits::default(),
            live_allowed,
            batch_window: Duration::from_millis(20),
            tick_every: Duration::from_millis(50),
            now: Arc::new(|| 1_000_000),
        },
    );
    let state = AppState::new(bus, auth, logbook, TxPolicy::default_safe().with_live_tx(live_allowed), false)
        .with_automation(automation.clone());
    (state, automation)
}

#[tokio::test]
async fn needs_a_paired_device() {
    let (s, _a) = state_with_automation(true);
    let app = build_router(s);
    status_is(&call(&app, req(Method::GET, "/api/v1/automation")).await, StatusCode::UNAUTHORIZED);
    status_is(&call(&app, req(Method::PUT, "/api/v1/automation").json("{}")).await, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn without_automation_the_routes_say_so() {
    let s = state();
    let cookie = cookie_for(&s);
    let resp = call(&router(&s), req(Method::GET, "/api/v1/automation").cookie(&cookie)).await;
    status_is(&resp, StatusCode::NOT_FOUND);
    assert_eq!(body_json(resp).await["error"], "automation_unavailable");
}

#[tokio::test]
async fn starts_off_in_dry_run_with_rules_limits_and_empty_log() {
    let (s, _a) = state_with_automation(true);
    let cookie = cookie_for(&s);
    let resp = call(&build_router(s), req(Method::GET, "/api/v1/automation").cookie(&cookie)).await;
    status_is(&resp, StatusCode::OK);
    let j = body_json(resp).await;
    assert_eq!(j["enabled"], false);
    assert_eq!(j["mode"], "dry_run");
    assert_eq!(j["live_allowed"], true);
    assert_eq!(j["limits"]["max_per_day"], 40);
    assert_eq!(j["rules"]["min_snr"], -18);
    assert_eq!(j["log"].as_array().unwrap().len(), 0);
}

#[tokio::test]
async fn a_phone_can_turn_it_on_in_dry_run_and_off() {
    let (s, a) = state_with_automation(true);
    let cookie = cookie_for(&s);
    let app = build_router(s);
    let on = call(&app, req(Method::PUT, "/api/v1/automation").cookie(&cookie).json(r#"{"enabled":true}"#)).await;
    status_is(&on, StatusCode::OK);
    assert!(a.state().enabled);
    let off = call(&app, req(Method::PUT, "/api/v1/automation").cookie(&cookie).json(r#"{"enabled":false}"#)).await;
    status_is(&off, StatusCode::OK);
    assert!(!a.state().enabled);
}

#[tokio::test]
async fn live_mode_only_from_the_pc_with_the_notice_accepted() {
    let (s, a) = state_with_automation(true);
    let cookie = cookie_for(&s);
    let app = build_router(s);
    let body = r#"{"mode":"live","accept_license_notice":true}"#;

    let phone = call(&app, req(Method::PUT, "/api/v1/automation").cookie(&cookie).json(body)).await;
    status_is(&phone, StatusCode::FORBIDDEN);
    assert_eq!(body_json(phone).await["error"], "local_only");

    let proxied = call(
        &app,
        req(Method::PUT, "/api/v1/automation").cookie(&cookie).json(body).via_loopback().header("x-forwarded-for", "1.2.3.4"),
    )
    .await;
    status_is(&proxied, StatusCode::FORBIDDEN);

    let no_notice = call(
        &app,
        req(Method::PUT, "/api/v1/automation").cookie(&cookie).json(r#"{"mode":"live"}"#).via_loopback(),
    )
    .await;
    status_is(&no_notice, StatusCode::BAD_REQUEST);
    assert_eq!(body_json(no_notice).await["error"], "notice_required");
    assert_eq!(a.state().mode, qso_engine::Mode::DryRun);

    let ok = call(&app, req(Method::PUT, "/api/v1/automation").cookie(&cookie).json(body).via_loopback()).await;
    status_is(&ok, StatusCode::OK);
    assert_eq!(a.state().mode, qso_engine::Mode::Live);
}

#[tokio::test]
async fn live_mode_is_refused_when_transmission_is_closed() {
    let (s, a) = state_with_automation(false);
    let cookie = cookie_for(&s);
    let resp = call(
        &build_router(s),
        req(Method::PUT, "/api/v1/automation")
            .cookie(&cookie)
            .json(r#"{"mode":"live","accept_license_notice":true}"#)
            .via_loopback(),
    )
    .await;
    status_is(&resp, StatusCode::FORBIDDEN);
    assert_eq!(body_json(resp).await["error"], "tx_disabled");
    assert_eq!(a.state().mode, qso_engine::Mode::DryRun);
}

#[tokio::test]
async fn a_phone_cannot_turn_on_a_live_automation_but_can_turn_it_off() {
    let (s, a) = state_with_automation(true);
    let cookie = cookie_for(&s);
    a.set_mode(qso_engine::Mode::Live).unwrap();
    let app = build_router(s);
    let on = call(&app, req(Method::PUT, "/api/v1/automation").cookie(&cookie).json(r#"{"enabled":true}"#)).await;
    status_is(&on, StatusCode::FORBIDDEN);
    assert!(!a.state().enabled);
    a.set_enabled(true);
    let off = call(&app, req(Method::PUT, "/api/v1/automation").cookie(&cookie).json(r#"{"enabled":false}"#)).await;
    status_is(&off, StatusCode::OK);
    assert!(!a.state().enabled);
}

#[tokio::test]
async fn rules_can_be_changed_and_are_validated() {
    let (s, a) = state_with_automation(true);
    let cookie = cookie_for(&s);
    let app = build_router(s);
    let good = r#"{"rules":{"reply_to":["new_dxcc"],"min_snr":-12,"excluded_calls":["AT1AA"],"max_attempts_per_call":1}}"#;
    status_is(&call(&app, req(Method::PUT, "/api/v1/automation").cookie(&cookie).json(good)).await, StatusCode::OK);
    assert_eq!(a.state().rules.min_snr, -12);
    assert_eq!(a.state().rules.excluded_calls, vec!["AT1AA".to_string()]);

    for bad in [
        r#"{"rules":{"reply_to":[],"min_snr":-12,"excluded_calls":[],"max_attempts_per_call":1}}"#,
        r#"{"rules":{"reply_to":["cq"],"min_snr":-12,"excluded_calls":[],"max_attempts_per_call":1}}"#,
        r#"{"rules":{"reply_to":["new_dxcc"],"min_snr":-90,"excluded_calls":[],"max_attempts_per_call":1}}"#,
        r#"{"rules":{"reply_to":["new_dxcc"],"min_snr":-12,"excluded_calls":[],"max_attempts_per_call":0}}"#,
        r#"{"rules":{"reply_to":["new_dxcc"],"min_snr":-12,"excluded_calls":["QUESTO NON E' UN NOMINATIVO"],"max_attempts_per_call":1}}"#,
    ] {
        let resp = call(&app, req(Method::PUT, "/api/v1/automation").cookie(&cookie).json(bad)).await;
        status_is(&resp, StatusCode::BAD_REQUEST);
        assert_eq!(body_json(resp).await["error"], "bad_rules", "{bad}");
    }
    assert_eq!(a.state().rules.min_snr, -12, "le regole non valide non cambiano nulla");
}

#[tokio::test]
async fn saved_rules_are_handed_to_the_saver_only_when_valid() {
    let (s, _a) = state_with_automation(true);
    let saved = Arc::new(std::sync::Mutex::new(Vec::<qso_engine::Rules>::new()));
    let sink = Arc::clone(&saved);
    let s = s.with_rules_saver(Arc::new(move |r: &qso_engine::Rules| sink.lock().unwrap().push(r.clone())));
    let cookie = cookie_for(&s);
    let app = build_router(s);
    let good = r#"{"rules":{"reply_to":["new_dxcc"],"min_snr":-9,"excluded_calls":[],"max_attempts_per_call":2}}"#;
    status_is(&call(&app, req(Method::PUT, "/api/v1/automation").cookie(&cookie).json(good)).await, StatusCode::OK);
    let bad = r#"{"rules":{"reply_to":["cq"],"min_snr":-9,"excluded_calls":[],"max_attempts_per_call":2}}"#;
    status_is(&call(&app, req(Method::PUT, "/api/v1/automation").cookie(&cookie).json(bad)).await, StatusCode::BAD_REQUEST);
    let saved = saved.lock().unwrap();
    assert_eq!(saved.len(), 1);
    assert_eq!(saved[0].min_snr, -9);
}

#[tokio::test]
async fn a_bad_request_changes_nothing() {
    let (s, a) = state_with_automation(true);
    let cookie = cookie_for(&s);
    let app = build_router(s);
    let resp = call(
        &app,
        req(Method::PUT, "/api/v1/automation")
            .cookie(&cookie)
            .json(r#"{"enabled":true,"rules":{"reply_to":[],"min_snr":0,"excluded_calls":[],"max_attempts_per_call":1}}"#),
    )
    .await;
    status_is(&resp, StatusCode::BAD_REQUEST);
    assert!(!a.state().enabled, "se una parte non e' valida non si applica neanche il resto");
    status_is(&call(&app, req(Method::PUT, "/api/v1/automation").cookie(&cookie).json(r#"{"enabled":"si"}"#)).await, StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn halt_tx_switches_the_automation_off() {
    let (s, a) = state_with_automation(true);
    let cookie = cookie_for(&s);
    a.set_enabled(true);
    let resp = call(&build_router(s), req(Method::POST, "/api/v1/commands/halt-tx").cookie(&cookie)).await;
    status_is(&resp, StatusCode::OK);
    assert!(!a.state().enabled);
}
