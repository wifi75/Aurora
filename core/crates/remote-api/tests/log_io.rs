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
        ..Default::default()
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

#[tokio::test]
async fn the_list_has_every_qso_newest_first_with_its_number_and_country() {
    let s = state().with_dxcc_if(Some(Arc::new(|call: &str| call.starts_with('K').then(|| serde_json::json!({ "country": "United States" })))));
    let a = s.logbook.add(&qso("K1ABC", "FT8", DAY1)).unwrap();
    let b = s.logbook.add(&qso("JA1XYZ", "FT4", DAY2)).unwrap();
    let v = body_json(get(&s, "/api/v1/log/qsos").await).await;
    assert_eq!(v["total"], 2);
    assert_eq!(v["qsos"][0]["id"], b);
    assert_eq!(v["qsos"][1]["id"], a);
    let first = &v["qsos"][1];
    assert_eq!((first["date"].as_str(), first["time"].as_str(), first["call"].as_str()), (Some("2026-10-24"), Some("16:46:45"), Some("K1ABC")));
    assert_eq!((first["band"].as_str(), first["mode"].as_str(), first["rst_sent"].as_str(), first["grid"].as_str()), (Some("20m"), Some("FT8"), Some("-08"), Some("FN42")));
    assert_eq!(first["country"], "United States");
    assert_eq!(v["qsos"][0]["country"], "");
    // dal telefono la finestra del programma non c'e'
    assert_eq!(v["save_dialog"], false);
}

#[tokio::test]
async fn the_download_keeps_only_the_chosen_ids_and_refuses_bad_ones() {
    let s = state();
    let a = s.logbook.add(&qso("K1ABC", "FT8", DAY1)).unwrap();
    let b = s.logbook.add(&qso("K1ABD", "FT4", DAY2)).unwrap();
    let c = s.logbook.add(&qso("K1ABE", "FT8", DAY2 + 60_000)).unwrap();
    let resp = get(&s, &format!("/api/v1/log/export.adi?ids={a},{c}")).await;
    assert_eq!(resp.headers()["x-aurora-qsos"], "2");
    let body = text(resp).await;
    assert!(body.contains("K1ABC") && body.contains("K1ABE") && !body.contains("K1ABD"));
    let resp = get(&s, &format!("/api/v1/log/export.adi?ids={a}-{b}")).await;
    assert_eq!(resp.headers()["x-aurora-qsos"], "2");
    for uri in ["/api/v1/log/export.adi?ids=uno", "/api/v1/log/export.adi?ids=5-2", "/api/v1/log/export.adi?ids=1-99999999"] {
        let resp = get(&s, uri).await;
        status_is(&resp, StatusCode::BAD_REQUEST);
        assert_eq!(body_json(resp).await["error"], "bad_ids", "{uri}");
    }
}

fn save_into(dir: std::path::PathBuf, seen: Arc<std::sync::Mutex<String>>) -> remote_api::SaveFileDialog {
    Arc::new(move |name: &str| {
        *seen.lock().unwrap() = name.to_string();
        // l'operatore toglie l'estensione: il file resta comunque .adi
        Some(dir.join("scelto"))
    })
}

fn temp_dir(tag: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("aurora-export-{tag}-{}-{}", std::process::id(), std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

#[tokio::test]
async fn save_as_writes_the_chosen_qsos_where_the_operator_says() {
    let dir = temp_dir("save");
    let seen = Arc::new(std::sync::Mutex::new(String::new()));
    let s = state().with_save_dialog_if(Some(save_into(dir.clone(), Arc::clone(&seen))));
    let a = s.logbook.add(&qso("K1ABC", "FT8", DAY1)).unwrap();
    s.logbook.add(&qso("K1ABD", "FT4", DAY2)).unwrap();
    let cookie = cookie_for(&s);
    // dal PC la pagina sa che puo' aprire la finestra
    let v = body_json(call(&router(&s), req(Method::GET, "/api/v1/log/qsos").cookie(&cookie).via_loopback()).await).await;
    assert_eq!(v["save_dialog"], true);
    let resp = call(&router(&s), req(Method::POST, "/api/v1/log/export-save").cookie(&cookie).via_loopback().json(&format!("{{\"ids\":[{a}]}}"))).await;
    status_is(&resp, StatusCode::OK);
    let v = body_json(resp).await;
    assert_eq!((v["cancelled"].as_bool(), v["count"].as_u64()), (Some(false), Some(1)));
    let path = std::path::PathBuf::from(v["path"].as_str().unwrap());
    assert_eq!(path, dir.join("scelto.adi"));
    let written = std::fs::read_to_string(&path).unwrap();
    assert!(written.contains("K1ABC") && !written.contains("K1ABD"));
    let proposed = seen.lock().unwrap().clone();
    assert!(proposed.starts_with("aurora_log_") && proposed.ends_with(".adi") && proposed.len() == "aurora_log_20261101.adi".len(), "{proposed}");
    // tutti
    let v = body_json(call(&router(&s), req(Method::POST, "/api/v1/log/export-save").cookie(&cookie).via_loopback().json("{\"all\":true}")).await).await;
    assert_eq!(v["count"], 2);
    let _ = std::fs::remove_dir_all(&dir);
}

#[tokio::test]
async fn save_as_can_be_cancelled_and_is_only_for_this_pc() {
    let s = state().with_save_dialog_if(Some(Arc::new(|_: &str| None)));
    s.logbook.add(&qso("K1ABC", "FT8", DAY1)).unwrap();
    let cookie = cookie_for(&s);
    let v = body_json(call(&router(&s), req(Method::POST, "/api/v1/log/export-save").cookie(&cookie).via_loopback().json("{\"all\":true}")).await).await;
    assert_eq!((v["ok"].as_bool(), v["cancelled"].as_bool()), (Some(true), Some(true)));
    // dalla rete o da un proxy: no
    let resp = call(&router(&s), req(Method::POST, "/api/v1/log/export-save").cookie(&cookie).json("{\"all\":true}")).await;
    status_is(&resp, StatusCode::FORBIDDEN);
    assert_eq!(body_json(resp).await["error"], "local_only");
    let resp = call(&router(&s), req(Method::POST, "/api/v1/log/export-save").cookie(&cookie).via_loopback().header("x-forwarded-for", "1.2.3.4").json("{\"all\":true}")).await;
    status_is(&resp, StatusCode::FORBIDDEN);
    // niente scelto, niente da salvare
    for (body, code) in [("{\"ids\":[]}", "no_selection"), ("{}", "no_selection"), ("{\"ids\":[999]}", "empty_export"), ("{\"all\":true,\"from\":\"20270101\"}", "empty_export")] {
        let resp = call(&router(&s), req(Method::POST, "/api/v1/log/export-save").cookie(&cookie).via_loopback().json(body)).await;
        status_is(&resp, StatusCode::BAD_REQUEST);
        assert_eq!(body_json(resp).await["error"], code, "{body}");
    }
}

#[tokio::test]
async fn without_the_windows_program_there_is_no_save_as() {
    let s = state();
    s.logbook.add(&qso("K1ABC", "FT8", DAY1)).unwrap();
    let cookie = cookie_for(&s);
    let v = body_json(call(&router(&s), req(Method::GET, "/api/v1/log/qsos").cookie(&cookie).via_loopback()).await).await;
    assert_eq!(v["save_dialog"], false);
    let resp = call(&router(&s), req(Method::POST, "/api/v1/log/export-save").cookie(&cookie).via_loopback().json("{\"all\":true}")).await;
    status_is(&resp, StatusCode::NOT_FOUND);
    assert_eq!(body_json(resp).await["error"], "save_unavailable");
}

async fn send(s: &remote_api::AppState, method: Method, uri: &str, body: Option<&str>) -> axum::response::Response {
    let cookie = cookie_for(s);
    let r = req(method, uri).cookie(&cookie).via_loopback();
    call(&router(s), match body {
        Some(b) => r.json(b),
        None => r,
    })
    .await
}

#[tokio::test]
async fn a_logged_qso_is_corrected_and_the_program_is_told_what_changed() {
    let seen: Arc<std::sync::Mutex<Vec<logbook::LogChange>>> = Arc::default();
    let s = state().with_log_edited({
        let seen = Arc::clone(&seen);
        Arc::new(move |c| seen.lock().unwrap().push(c))
    });
    let id = s.logbook.add(&qso("K1ABX", "FT8", DAY1)).unwrap();
    s.logbook.add(&qso("DL1ABC", "FT8", DAY2)).unwrap();
    let one = body_json(get(&s, &format!("/api/v1/log/qsos/{id}")).await).await;
    assert_eq!((one["call"].as_str(), one["date"].as_str(), one["time"].as_str()), (Some("K1ABX"), Some("2026-10-24"), Some("16:46:45")));

    let resp = send(&s, Method::PATCH, &format!("/api/v1/log/qsos/{id}"), Some(r#"{"call":" k1abc ","time":"16:50","grid":"fn42ab","name":"Joe","comment":"corretto"}"#)).await;
    status_is(&resp, StatusCode::OK);
    let v = body_json(resp).await;
    assert_eq!(v["ok"], true);
    let q = &v["qso"];
    assert_eq!((q["id"].as_i64(), q["call"].as_str(), q["date"].as_str(), q["time"].as_str()), (Some(id), Some("K1ABC"), Some("2026-10-24"), Some("16:50:00")));
    assert_eq!((q["grid"].as_str(), q["name"].as_str(), q["comment"].as_str(), q["band"].as_str(), q["rst_sent"].as_str()), (Some("FN42AB"), Some("Joe"), Some("corretto"), Some("20m"), Some("-08")));
    // l'elenco mostra la riga corretta e l'altro QSO com'era
    let list = body_json(get(&s, "/api/v1/log/qsos").await).await;
    let calls: Vec<&str> = list["qsos"].as_array().unwrap().iter().filter_map(|q| q["call"].as_str()).collect();
    assert_eq!(calls, vec!["DL1ABC", "K1ABC"]);
    // il programma riceve com'era e com'e' (per il file ADIF e i lavorati), con la fine spostata come l'inizio
    let seen = seen.lock().unwrap();
    assert_eq!(seen.len(), 1);
    assert_eq!((seen[0].old.call.as_str(), seen[0].new.call.as_str(), seen[0].new.time.as_str(), seen[0].time_off.as_str()), ("K1ABX", "K1ABC", "165000", "165044"));
}

#[tokio::test]
async fn a_correction_with_nothing_new_does_not_rewrite_anything() {
    let told = Arc::new(AtomicU32::new(0));
    let t = Arc::clone(&told);
    let s = state().with_log_edited(Arc::new(move |_| {
        t.fetch_add(1, Ordering::SeqCst);
    }));
    let id = s.logbook.add(&qso("K1ABC", "FT8", DAY1)).unwrap();
    status_is(&send(&s, Method::PATCH, &format!("/api/v1/log/qsos/{id}"), Some(r#"{"call":"K1ABC","date":"2026-10-24","time":"16:46:45"}"#)).await, StatusCode::OK);
    assert_eq!(told.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn a_bad_value_or_a_missing_qso_is_refused_and_the_log_stays() {
    let s = state();
    let id = s.logbook.add(&qso("K1ABC", "FT8", DAY1)).unwrap();
    let uri = format!("/api/v1/log/qsos/{id}");
    for (body, field) in [
        (r#"{"call":"  "}"#, "call"),
        (r#"{"call":"K1 ABC"}"#, "call"),
        (r#"{"date":"2026-13-01"}"#, "time"),
        (r#"{"time":"25:00"}"#, "time"),
        (r#"{"grid":"ZZ99"}"#, "grid"),
        (r#"{"grid":"FN4"}"#, "grid"),
        (r#"{"mode":""}"#, "mode"),
        (r#"{"band":"","freq_hz":0}"#, "band"),
    ] {
        let resp = send(&s, Method::PATCH, &uri, Some(body)).await;
        status_is(&resp, StatusCode::BAD_REQUEST);
        let v = body_json(resp).await;
        assert_eq!((v["error"].as_str(), v["message"].as_str()), (Some("bad_field"), Some(field)), "{body}");
    }
    // un locatore vuoto va bene (si toglie), la banda vuota si ricava dalla frequenza
    let resp = send(&s, Method::PATCH, &uri, Some(r#"{"grid":"","band":"","freq_hz":7074000}"#)).await;
    status_is(&resp, StatusCode::OK);
    let v = body_json(resp).await;
    assert_eq!((v["qso"]["grid"].as_str(), v["qso"]["band"].as_str(), v["qso"]["call"].as_str()), (Some(""), Some("40m"), Some("K1ABC")));

    for (method, body) in [(Method::GET, None), (Method::PATCH, Some(r#"{"call":"K1ABD"}"#)), (Method::DELETE, None)] {
        let resp = send(&s, method, "/api/v1/log/qsos/999", body).await;
        status_is(&resp, StatusCode::NOT_FOUND);
        assert_eq!(body_json(resp).await["error"], "qso_not_found");
    }
    assert_eq!(s.logbook.count().unwrap(), 1);
}

#[tokio::test]
async fn one_qso_is_deleted_by_its_number_and_the_program_is_told() {
    let seen: Arc<std::sync::Mutex<Vec<String>>> = Arc::default();
    let s = state().with_log_deleted({
        let seen = Arc::clone(&seen);
        Arc::new(move |gone| seen.lock().unwrap().extend(gone.unwrap_or_default().into_iter().map(|e| e.call)))
    });
    let id = s.logbook.add(&qso("K1ABC", "FT8", DAY1)).unwrap();
    s.logbook.add(&qso("DL1ABC", "FT8", DAY2)).unwrap();
    let resp = send(&s, Method::DELETE, &format!("/api/v1/log/qsos/{id}"), None).await;
    status_is(&resp, StatusCode::OK);
    assert_eq!(body_json(resp).await["deleted"], 1);
    assert_eq!(*seen.lock().unwrap(), vec!["K1ABC".to_string()]);
    assert_eq!(s.logbook.count().unwrap(), 1);
    status_is(&send(&s, Method::DELETE, &format!("/api/v1/log/qsos/{id}"), None).await, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn correcting_and_deleting_are_only_for_the_home_network() {
    let s = state();
    let id = s.logbook.add(&qso("K1ABC", "FT8", DAY1)).unwrap();
    let cookie = cookie_for(&s);
    let uri = format!("/api/v1/log/qsos/{id}");
    status_is(&call(&router(&s), req(Method::PATCH, &uri).json(r#"{"call":"K1ABD"}"#)).await, StatusCode::UNAUTHORIZED);
    let proxied = call(&router(&s), req(Method::PATCH, &uri).cookie(&cookie).via_loopback().header("x-forwarded-for", "203.0.113.9").json(r#"{"call":"K1ABD"}"#)).await;
    status_is(&proxied, StatusCode::FORBIDDEN);
    status_is(&call(&router(&s), req(Method::DELETE, &uri).cookie(&cookie).via_loopback().header("x-forwarded-for", "203.0.113.9")).await, StatusCode::FORBIDDEN);
    assert_eq!(s.logbook.entry(id).unwrap().map(|e| e.call), Some("K1ABC".to_string()));
}
