mod common;

use std::{net::SocketAddr, path::PathBuf, time::Duration};

use aurora::{run, Config, EngineKind, RunningServer};
use aurora_core::{DecodeTag, Event};
use common::*;
use futures_util::StreamExt;
use tokio::sync::oneshot;
use tokio_tungstenite::tungstenite::Message;

fn config(ui_dir: Option<PathBuf>) -> Config {
    Config {
        bind: "127.0.0.1:0".parse().unwrap(),
        data_dir: temp_dir("data"),
        ui_dir,
        engine: EngineKind::Sim,
        cycle_ms: 300,
        ..Config::defaults()
    }
}

async fn start(ui_dir: Option<PathBuf>) -> (RunningServer, oneshot::Sender<()>) {
    let (tx, rx) = oneshot::channel::<()>();
    let server = run(config(ui_dir), async move {
        let _ = rx.await;
    })
    .await
    .unwrap();
    (server, tx)
}

#[tokio::test]
async fn e2e_pair_connect_receive_decode_and_spectrum_within_3s() {
    let (server, _stop) = start(None).await;
    let (status, body) = http(server.addr, "GET", "/api/v1/health", None, None).await;
    assert_eq!(status, 200);
    assert!(body.contains("Tiziano Cassone"));
    let mut ws = open_ws(server.addr, &cookie_for(&server)).await;
    let (mut decode, mut spectrum) = (false, false);
    let deadline = tokio::time::Instant::now() + Duration::from_secs(3);
    while tokio::time::Instant::now() < deadline && !(decode && spectrum) {
        match tokio::time::timeout(Duration::from_millis(500), ws.next()).await {
            Ok(Some(Ok(Message::Text(t)))) => decode |= t.as_str().contains(r#""type":"decode""#),
            Ok(Some(Ok(Message::Binary(b)))) => spectrum |= b.len() == 17 + 1024 && b[0] == 1,
            _ => {}
        }
    }
    assert!(decode && spectrum, "decode: {decode}, spettro: {spectrum}");
}

#[tokio::test]
async fn e2e_halt_tx_emits_tx_halted_over_stream() {
    let (server, _stop) = start(None).await;
    let cookie = cookie_for(&server);
    let mut ws = open_ws(server.addr, &cookie).await;
    let (status, _) = http(server.addr, "POST", "/api/v1/commands/halt-tx", Some(&cookie), None).await;
    assert_eq!(status, 200);
    let mut seen = false;
    for _ in 0..40 {
        match next_event(&mut ws, 1).await {
            Some(Event::TxHalted) => {
                seen = true;
                break;
            }
            Some(_) => {}
            None => break,
        }
    }
    assert!(seen, "TxHalted non ricevuto");
}

#[tokio::test]
async fn e2e_reply_then_status_shows_transmitting() {
    let (server, _stop) = start(None).await;
    let cookie = cookie_for(&server);
    let mut ws = open_ws(server.addr, &cookie).await;
    let mut id = None;
    for _ in 0..40 {
        if let Some(Event::Decode(d)) = next_event(&mut ws, 1).await {
            id = Some(d.id);
            break;
        }
    }
    let id = id.expect("nessun decode ricevuto");
    // l'indicizzatore del server ha bisogno di un istante per registrare l'id
    tokio::time::sleep(Duration::from_millis(200)).await;
    let body = format!(r#"{{"decode_id":{id}}}"#);
    let (status, _) = http(server.addr, "POST", "/api/v1/commands/reply", Some(&cookie), Some(&body)).await;
    assert_eq!(status, 200);
    let mut transmitting = false;
    for _ in 0..40 {
        match next_event(&mut ws, 1).await {
            Some(Event::Status(s)) if s.transmitting => {
                transmitting = true;
                break;
            }
            Some(_) => {}
            None => break,
        }
    }
    assert!(transmitting, "nessuno Status con transmitting=true");
}

#[tokio::test]
async fn e2e_decode_with_tag_roundtrips_to_json() {
    let (server, _stop) = start(None).await;
    let mut ws = open_ws(server.addr, &cookie_for(&server)).await;
    let mut tagged = false;
    for _ in 0..60 {
        if let Some(Event::Decode(d)) = next_event(&mut ws, 1).await
            && matches!(d.tag, Some(DecodeTag::Cq | DecodeTag::NewDxcc | DecodeTag::NewZone))
        {
            tagged = true;
            break;
        }
    }
    assert!(tagged, "nessun decode con etichetta");
}

#[tokio::test]
async fn e2e_graceful_shutdown_closes_streams() {
    let (server, stop) = start(None).await;
    let mut ws = open_ws(server.addr, &cookie_for(&server)).await;
    tokio::time::sleep(Duration::from_millis(200)).await;
    stop.send(()).unwrap();
    let closed = tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            match ws.next().await {
                None | Some(Err(_)) | Some(Ok(Message::Close(_))) => return true,
                Some(Ok(_)) => {}
            }
        }
    })
    .await;
    assert_eq!(closed, Ok(true));
    tokio::time::timeout(Duration::from_secs(5), server.join()).await.expect("il server deve terminare");
}

#[tokio::test]
async fn serves_ui_index_with_spa_fallback_and_keeps_api_404() {
    let ui = temp_dir("ui");
    std::fs::write(ui.join("index.html"), "<h1>Interfaccia Aurora</h1>").unwrap();
    let (server, _stop) = start(Some(ui)).await;
    for path in ["/", "/pair", "/live/qualcosa"] {
        let (status, body) = http(server.addr, "GET", path, None, None).await;
        assert_eq!(status, 200, "{path}");
        assert!(body.contains("Interfaccia Aurora"), "{path}");
    }
    let (status, body) = http(server.addr, "GET", "/api/v1/health", None, None).await;
    assert_eq!(status, 200);
    assert!(body.contains(r#""status":"ok""#));
    let (status, body) = http(server.addr, "GET", "/api/v1/non-esiste", None, None).await;
    assert_eq!(status, 404);
    assert!(!body.contains("Interfaccia Aurora"));
}

#[test]
fn config_defaults_and_overrides_and_errors() {
    let none = |_: &str| None::<String>;
    let c = Config::from_lookup(none).unwrap();
    assert_eq!(c.bind, "127.0.0.1:8787".parse::<SocketAddr>().unwrap());
    assert_eq!(c.cycle_ms, 15_000);
    assert!(c.ui_dir.is_none());
    assert!(!c.secure_cookie);
    assert!(matches!(c.engine, EngineKind::Sim));

    let env = |k: &str| match k {
        "AURORA_BIND" => Some("0.0.0.0:9000".to_string()),
        "AURORA_UI_DIR" => Some("ui".to_string()),
        "AURORA_SIM_CYCLE_MS" => Some("500".to_string()),
        "AURORA_SECURE_COOKIE" => Some("1".to_string()),
        _ => None,
    };
    let c = Config::from_lookup(env).unwrap();
    assert_eq!(c.bind.port(), 9000);
    assert_eq!(c.ui_dir, Some(PathBuf::from("ui")));
    assert_eq!(c.cycle_ms, 500);
    assert!(c.secure_cookie);

    assert!(Config::from_lookup(|k| (k == "AURORA_BIND").then(|| "non-un-indirizzo".to_string())).is_err());
    assert!(Config::from_lookup(|k| (k == "AURORA_ENGINE").then(|| "marte".to_string())).is_err());
}
