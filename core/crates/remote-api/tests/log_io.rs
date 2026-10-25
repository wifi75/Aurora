mod common;

use std::sync::{
    atomic::{AtomicU32, Ordering},
    Arc,
};

use aurora_core::QsoRecord;
use axum::http::{header, Method, StatusCode};
use common::*;
use logbook::adif::Station;

fn qso(call: &str, mode: &str, on_ms: u64) -> QsoRecord {
    QsoRecord {
        call: call.into(),
        grid: "FN42".into(),
        band: "20m".into(),
        mode: mode.into(),
        freq_hz: 14_074_000,
        rst_sent: "-08".into(),
        rst_rcvd: "-12".into(),
        time_ms: (on_ms % 86_400_000) as u32,
        time_on_unix_ms: on_ms,
        time_off_unix_ms: on_ms + 44_000,
    }
}

// 2026-10-24 16:46:45 UTC e il giorno dopo
const DAY1: u64 = 1_792_860_405_000;
const DAY2: u64 = DAY1 + 86_400_000;

async fn get(s: &remote_api::AppState, uri: &str) -> axum::response::Response {
    let cookie = cookie_for(s);
    call(&router(s), req(Method::GET, uri).cookie(&cookie)).await
}

async fn text(resp: axum::response::Response) -> String {
    String::from_utf8(axum::body::to_bytes(resp.into_body(), 1 << 22).await.unwrap().to_vec()).unwrap()
}

#[tokio::test]
async fn the_export_needs_a_paired_device() {
    let s = state();
    status_is(&call(&router(&s), req(Method::GET, "/api/v1/log/export.adi")).await, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn the_export_is_the_jtdx_file_as_an_attachment_with_the_station_and_a_date_filter() {
    let s = state().with_log_station(Arc::new(|| Station { call: "IU3ABC".into(), grid: "IO91wm".into(), tx_power: String::new() }));
    s.logbook.add(&qso("K1ABC", "FT8", DAY1)).unwrap();
    s.logbook.add(&qso("K1ABD", "FT4", DAY2)).unwrap();
    let resp = get(&s, "/api/v1/log/export.adi").await;
    status_is(&resp, StatusCode::OK);
    assert!(resp.headers()[header::CONTENT_DISPOSITION].to_str().unwrap().contains("aurora_log.adi"));
    assert_eq!(resp.headers()["x-aurora-qsos"], "2");
    let body = text(resp).await;
    assert!(body.starts_with("WSJT-X ADIF Export<eoh>\r\n<call:5>K1ABC "), "{body}");
    assert!(body.contains("<station_callsign:6>IU3ABC <my_gridsquare:6>IO91wm <eor>"));
    let day2 = get(&s, "/api/v1/log/export.adi?from=2026-10-25&to=20261025").await;
    assert_eq!(day2.headers()["x-aurora-qsos"], "1");
    let body = text(day2).await;
    assert!(body.contains("K1ABD") && !body.contains("K1ABC"));
}

#[tokio::test]
async fn a_bad_or_reversed_range_is_refused() {
    let s = state();
    for uri in ["/api/v1/log/export.adi?from=ieri", "/api/v1/log/export.adi?to=20261301", "/api/v1/log/export.adi?from=20261025&to=20261024"] {
        let resp = get(&s, uri).await;
        status_is(&resp, StatusCode::BAD_REQUEST);
        assert_eq!(body_json(resp).await["error"], "bad_range", "{uri}");
    }
}

#[tokio::test]
async fn the_import_adds_the_qsos_once_and_tells_the_log_changed() {
    let changed = Arc::new(AtomicU32::new(0));
    let c = Arc::clone(&changed);
    let s = state().with_log_changed(Arc::new(move || {
        c.fetch_add(1, Ordering::SeqCst);
    }));
    s.logbook.add(&qso("K1ABC", "FT8", DAY1)).unwrap();
    let file = "WSJT-X ADIF Export<eoh>\r\n<call:5>K1ABC <mode:3>FT8 <qso_date:8>20261024 <time_on:6>164650 <band:3>20m <eor>\r\n\
                <call:5>K1ABD <mode:3>FT8 <qso_date:8>20261024 <time_on:6>180000 <band:3>20m <eor>\r\n";
    let cookie = cookie_for(&s);
    let mut r = req(Method::POST, "/api/v1/log/import").cookie(&cookie).json(file);
    r.headers.clear();
    let resp = call(&router(&s), r).await;
    status_is(&resp, StatusCode::OK);
    let v = body_json(resp).await;
    assert_eq!((v["read"].as_u64(), v["added"].as_u64(), v["duplicates"].as_u64(), v["skipped"].as_u64()), (Some(2), Some(1), Some(1), Some(0)));
    assert_eq!(s.logbook.count().unwrap(), 2);
    assert_eq!(changed.load(Ordering::SeqCst), 1);
    // di nuovo lo stesso file: niente di nuovo, nessun segnale di cambiamento
    let resp = call(&router(&s), req(Method::POST, "/api/v1/log/import").cookie(&cookie).json(file)).await;
    assert_eq!(body_json(resp).await["added"], 0);
    assert_eq!(changed.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn the_import_refuses_an_empty_body_and_a_proxied_request() {
    let s = state();
    let cookie = cookie_for(&s);
    let resp = call(&router(&s), req(Method::POST, "/api/v1/log/import").cookie(&cookie).json("  \n")).await;
    status_is(&resp, StatusCode::BAD_REQUEST);
    assert_eq!(body_json(resp).await["error"], "empty_file");
    let resp = call(&router(&s), req(Method::POST, "/api/v1/log/import").cookie(&cookie).header("x-forwarded-for", "1.2.3.4").json("<call:1>A<eor>")).await;
    status_is(&resp, StatusCode::FORBIDDEN);
    assert_eq!(body_json(resp).await["error"], "local_only");
    assert_eq!(s.logbook.count().unwrap(), 0);
}
