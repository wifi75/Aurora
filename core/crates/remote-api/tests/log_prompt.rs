mod common;

use aurora_core::{Event, QsoRecord};
use axum::http::{Method, StatusCode};
use common::*;
use serde_json::json;

fn qso(call: &str) -> QsoRecord {
    QsoRecord { call: call.into(), grid: "FN42".into(), band: "20m".into(), mode: "FT8".into(), rst_sent: "-10".into(), rst_rcvd: "-12".into(), ..QsoRecord::default() }
}

#[tokio::test]
async fn a_waiting_qso_is_listed_then_logged_with_the_corrections() {
    let s = state();
    let cookie = cookie_for(&s);
    let app = router(&s);
    let mut events = s.bus.subscribe_events();
    s.log_prompt.push(qso("K1ABC"));
    let list = body_json(call(&app, req(Method::GET, "/api/v1/log/pending").cookie(&cookie)).await).await;
    assert_eq!(list["pending"][0]["qso"]["call"], "K1ABC");
    let id = list["pending"][0]["id"].as_u64().expect("id");
    let body = json!({ "action": "log", "edits": { "rst_rcvd": "-05", "name": "Joe" } }).to_string();
    let resp = call(&app, req(Method::POST, &format!("/api/v1/log/pending/{id}")).cookie(&cookie).json(&body)).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let logged = loop {
        if let Ok(Event::QsoLogged(q)) = events.recv().await {
            break q;
        }
    };
    assert_eq!((logged.call.as_str(), logged.rst_rcvd.as_str(), logged.name.as_str()), ("K1ABC", "-05", "Joe"));
    assert!(s.log_prompt.list().is_empty(), "non aspetta piu'");
}

#[tokio::test]
async fn a_discarded_qso_is_not_logged_and_a_bad_field_keeps_it_waiting() {
    let s = state();
    let cookie = cookie_for(&s);
    let app = router(&s);
    s.log_prompt.push(qso("K1ABC"));
    let id = s.log_prompt.list()[0].id;
    let bad = json!({ "action": "log", "edits": { "call": "K1 ABC" } }).to_string();
    let resp = call(&app, req(Method::POST, &format!("/api/v1/log/pending/{id}")).cookie(&cookie).json(&bad)).await;
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    assert_eq!(s.log_prompt.list().len(), 1, "un campo sbagliato non fa sparire il QSO");
    let discard = json!({ "action": "discard" }).to_string();
    let resp = call(&app, req(Method::POST, &format!("/api/v1/log/pending/{id}")).cookie(&cookie).json(&discard)).await;
    assert_eq!(resp.status(), StatusCode::OK);
    assert!(s.log_prompt.list().is_empty());
    let again = call(&app, req(Method::POST, &format!("/api/v1/log/pending/{id}")).cookie(&cookie).json(&discard)).await;
    assert_eq!(again.status(), StatusCode::NOT_FOUND);
}
