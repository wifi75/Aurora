mod common;

use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};

use axum::http::{Method, StatusCode};
use common::*;
use remote_api::{AppState, UpdateInstaller};

struct Fake {
    calls: AtomicUsize,
    result: Result<String, String>,
}

impl UpdateInstaller for Fake {
    fn install(&self) -> Result<String, String> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        self.result.clone()
    }
}

fn with_installer(result: Result<String, String>) -> (AppState, Arc<Fake>) {
    let fake = Arc::new(Fake { calls: AtomicUsize::new(0), result });
    (state().with_update_installer(fake.clone()), fake)
}

#[tokio::test]
async fn installing_needs_a_paired_device_and_a_program_that_knows_how() {
    let s = state();
    status_is(&call(&router(&s), req(Method::POST, "/api/v1/update/install")).await, StatusCode::UNAUTHORIZED);
    let cookie = cookie_for(&s);
    let resp = call(&router(&s), req(Method::POST, "/api/v1/update/install").cookie(&cookie).via_loopback()).await;
    status_is(&resp, StatusCode::NOT_FOUND);
    assert_eq!(body_json(resp).await["error"], "install_unavailable");
}

#[tokio::test]
async fn installing_is_refused_from_the_network_and_from_a_proxy() {
    let (s, fake) = with_installer(Ok("1.0.0".into()));
    let cookie = cookie_for(&s);
    let app = router(&s);
    let resp = call(&app, req(Method::POST, "/api/v1/update/install").cookie(&cookie)).await;
    status_is(&resp, StatusCode::FORBIDDEN);
    assert_eq!(body_json(resp).await["error"], "local_only");
    let proxied = call(&app, req(Method::POST, "/api/v1/update/install").cookie(&cookie).via_loopback().header("x-forwarded-for", "1.2.3.4")).await;
    status_is(&proxied, StatusCode::FORBIDDEN);
    assert_eq!(fake.calls.load(Ordering::SeqCst), 0, "nessuna installazione partita");
}

#[tokio::test]
async fn installing_from_the_pc_runs_the_installer_and_says_the_version() {
    let (s, fake) = with_installer(Ok("1.0.0".into()));
    let cookie = cookie_for(&s);
    let resp = call(&router(&s), req(Method::POST, "/api/v1/update/install").cookie(&cookie).via_loopback()).await;
    status_is(&resp, StatusCode::OK);
    assert_eq!(body_json(resp).await["version"], "1.0.0");
    assert_eq!(fake.calls.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn a_failed_download_or_signature_is_a_bad_gateway() {
    let (s, _) = with_installer(Err("la firma del pacchetto non corrisponde".into()));
    let cookie = cookie_for(&s);
    let resp = call(&router(&s), req(Method::POST, "/api/v1/update/install").cookie(&cookie).via_loopback()).await;
    status_is(&resp, StatusCode::BAD_GATEWAY);
    assert_eq!(body_json(resp).await["error"], "install_failed");
}

#[tokio::test]
async fn the_update_status_says_it_cannot_install_when_there_is_nothing_to_install() {
    let dir = std::env::temp_dir().join(format!("aurora-api-inst-{}", std::process::id()));
    std::fs::remove_dir_all(&dir).ok();
    std::fs::create_dir_all(&dir).unwrap();
    let service = updater::UpdateService::open_with_api(&dir.join("update.json"), "0.9.0", "http://127.0.0.1:1");
    let fake = Arc::new(Fake { calls: AtomicUsize::new(0), result: Ok(String::new()) });
    let s = state().with_updates(service).with_update_installer(fake);
    let cookie = cookie_for(&s);
    let v = body_json(call(&router(&s), req(Method::GET, "/api/v1/update").cookie(&cookie)).await).await;
    assert_eq!(v["can_install"], false, "nessuna versione nuova");
    std::fs::remove_dir_all(&dir).ok();
}
