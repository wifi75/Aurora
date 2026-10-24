mod common;

use std::{net::SocketAddr, time::Duration};

use aurora_core::{DecodedMessage, Event, SpectrumRow};
use futures_util::StreamExt;
use remote_api::{build_router, AppState};
use tokio::net::TcpListener;
use tokio_tungstenite::{
    connect_async,
    tungstenite::{client::IntoClientRequest, Message},
};

use common::*;

async fn serve(state: &AppState) -> SocketAddr {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let app = build_router(state.clone());
    tokio::spawn(async move {
        axum::serve(listener, app.into_make_service_with_connect_info::<SocketAddr>()).await.unwrap();
    });
    addr
}

fn ws_request(addr: SocketAddr, cookie: Option<&str>) -> tokio_tungstenite::tungstenite::handshake::client::Request {
    let mut r = format!("ws://{addr}/api/v1/stream").into_client_request().unwrap();
    if let Some(c) = cookie {
        r.headers_mut().insert("Cookie", c.parse().unwrap());
    }
    r
}

fn row(seq: u64) -> SpectrumRow {
    SpectrumRow { seq, f_min_hz: 0, f_max_hz: 3000, bins: (0..1024).map(|i| (i % 251) as u8).collect() }
}

#[tokio::test]
async fn ws_without_cookie_is_rejected() {
    let s = state();
    let addr = serve(&s).await;
    let err = connect_async(ws_request(addr, None)).await.unwrap_err();
    assert!(err.to_string().contains("401"), "errore: {err}");
}

#[tokio::test]
async fn ws_receives_json_event_and_binary_spectrum_frame_with_expected_layout() {
    let s = state();
    let addr = serve(&s).await;
    let cookie = cookie_for(&s);
    let (mut ws, _) = connect_async(ws_request(addr, Some(&cookie))).await.unwrap();
    tokio::time::sleep(Duration::from_millis(100)).await;
    s.bus
        .events()
        .send(Event::Decode(DecodedMessage {
            id: 9,
            time_ms: 1,
            snr: -3,
            delta_time: 0.2,
            freq_hz: 1200,
            text: "CQ TEST I2ABC JN45".into(),
            tag: None,
            country: None,
        }))
        .unwrap();
    s.bus.spectrum().send(row(0x0102_0304_0506_0708)).unwrap();

    let mut got_text = false;
    let mut got_bin = false;
    for _ in 0..10 {
        let msg = tokio::time::timeout(Duration::from_secs(2), ws.next()).await.unwrap().unwrap().unwrap();
        match msg {
            Message::Text(t) => {
                let v: serde_json::Value = serde_json::from_str(t.as_str()).unwrap();
                if v["type"] == "decode" {
                    assert_eq!(v["id"], 9);
                    got_text = true;
                }
            }
            Message::Binary(b) => {
                assert_eq!(b[0], 0x01);
                assert_eq!(u64::from_be_bytes(b[1..9].try_into().unwrap()), 0x0102_0304_0506_0708);
                assert_eq!(u32::from_be_bytes(b[9..13].try_into().unwrap()), 0);
                assert_eq!(u32::from_be_bytes(b[13..17].try_into().unwrap()), 3000);
                assert_eq!(b.len(), 17 + 1024);
                assert_eq!(b[17 + 5], 5);
                got_bin = true;
            }
            _ => {}
        }
        if got_text && got_bin {
            break;
        }
    }
    assert!(got_text && got_bin, "testo: {got_text}, binario: {got_bin}");
}

#[tokio::test]
async fn slow_client_does_not_block_fast_client() {
    let s = state();
    let addr = serve(&s).await;
    let cookie = cookie_for(&s);
    let (_slow, _) = connect_async(ws_request(addr, Some(&cookie))).await.unwrap();
    let (mut fast, _) = connect_async(ws_request(addr, Some(&cookie))).await.unwrap();
    tokio::time::sleep(Duration::from_millis(100)).await;
    let spectrum = s.bus.spectrum();
    tokio::spawn(async move {
        for seq in 0..1500u64 {
            let _ = spectrum.send(row(seq));
            tokio::time::sleep(Duration::from_millis(1)).await;
        }
    });
    let mut frames = 0;
    let deadline = tokio::time::Instant::now() + Duration::from_secs(8);
    while tokio::time::Instant::now() < deadline && frames < 200 {
        if let Ok(Some(Ok(Message::Binary(_)))) = tokio::time::timeout(Duration::from_secs(1), fast.next()).await {
            frames += 1;
        }
    }
    assert!(frames >= 200, "il client veloce ha ricevuto solo {frames} righe");
}

#[tokio::test]
async fn ws_closed_after_device_revoked() {
    let s = state();
    let addr = serve(&s).await;
    let token = token_for(&s);
    let cookie = format!("aurora_session={token}");
    let (mut ws, _) = connect_async(ws_request(addr, Some(&cookie))).await.unwrap();
    let id = s.auth.verify(&token).unwrap();
    s.auth.revoke(id).unwrap();
    let closed = tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            match ws.next().await {
                None | Some(Err(_)) | Some(Ok(Message::Close(_))) => return true,
                Some(Ok(_)) => {}
            }
        }
    })
    .await;
    assert_eq!(closed, Ok(true), "il flusso doveva chiudersi dopo la revoca");
}

#[tokio::test]
async fn ws_closed_on_shutdown() {
    let s = state();
    let addr = serve(&s).await;
    let cookie = cookie_for(&s);
    let (mut ws, _) = connect_async(ws_request(addr, Some(&cookie))).await.unwrap();
    tokio::time::sleep(Duration::from_millis(100)).await;
    s.begin_shutdown();
    let closed = tokio::time::timeout(Duration::from_secs(3), async {
        loop {
            match ws.next().await {
                None | Some(Err(_)) | Some(Ok(Message::Close(_))) => return true,
                Some(Ok(_)) => {}
            }
        }
    })
    .await;
    assert_eq!(closed, Ok(true), "il flusso doveva chiudersi allo spegnimento");
}

#[tokio::test]
async fn invalid_spectrum_row_is_not_forwarded_to_clients() {
    let s = state();
    let addr = serve(&s).await;
    let cookie = cookie_for(&s);
    let (mut ws, _) = connect_async(ws_request(addr, Some(&cookie))).await.unwrap();
    tokio::time::sleep(Duration::from_millis(100)).await;
    let spectrum = s.bus.spectrum();
    spectrum.send(SpectrumRow { seq: 1, f_min_hz: 0, f_max_hz: 3000, bins: vec![] }).unwrap();
    spectrum.send(SpectrumRow { seq: 2, f_min_hz: 0, f_max_hz: 3000, bins: vec![0; 5000] }).unwrap();
    spectrum.send(SpectrumRow { seq: 3, f_min_hz: 3000, f_max_hz: 0, bins: vec![1; 8] }).unwrap();
    spectrum.send(row(4)).unwrap();
    let msg = tokio::time::timeout(Duration::from_secs(2), async {
        loop {
            if let Some(Ok(Message::Binary(b))) = ws.next().await {
                return b;
            }
        }
    })
    .await
    .expect("deve arrivare almeno la riga valida");
    assert_eq!(u64::from_be_bytes(msg[1..9].try_into().unwrap()), 4, "le righe non valide non vanno inoltrate");
}
