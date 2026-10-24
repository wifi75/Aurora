mod common;

use std::{net::SocketAddr, time::Duration};

use aurora_core::Event;
use futures_util::StreamExt;
use remote_api::{build_router, AppState};
use tokio::net::TcpListener;
use tokio_tungstenite::{connect_async, tungstenite::{client::IntoClientRequest, Message}};

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

fn request(addr: SocketAddr) -> tokio_tungstenite::tungstenite::handshake::client::Request {
    format!("ws://{addr}/api/v1/stream").into_client_request().unwrap()
}

/// Il flusso dell'utente "senza codice" non e' di un dispositivo registrato: non va chiuso dal controllo di revoca.
#[tokio::test]
async fn the_stream_of_a_code_free_visitor_stays_open_and_keeps_delivering() {
    let s = state().with_open_access(true);
    let addr = serve(&s).await;
    let (mut ws, _) = connect_async(request(addr)).await.expect("accesso senza cookie dalla rete di casa");
    // piu' del controllo di revoca (un secondo): prima il flusso veniva chiuso qui
    tokio::time::sleep(Duration::from_millis(2600)).await;
    s.bus.events().send(Event::TxHalted).unwrap();
    let next = tokio::time::timeout(Duration::from_secs(2), ws.next()).await.expect("nessun messaggio").expect("flusso chiuso").expect("errore");
    match next {
        Message::Text(t) => assert!(t.contains("tx_halted"), "{t}"),
        other => panic!("atteso un evento, trovato {other:?}"),
    }
}

#[tokio::test]
async fn turning_the_open_access_off_closes_the_visitor_stream() {
    let s = state().with_open_access(true);
    let addr = serve(&s).await;
    let (mut ws, _) = connect_async(request(addr)).await.unwrap();
    s.open_access.store(false, std::sync::atomic::Ordering::SeqCst);
    let closed = tokio::time::timeout(Duration::from_secs(3), async {
        loop {
            match ws.next().await {
                None | Some(Ok(Message::Close(_))) | Some(Err(_)) => return true,
                Some(Ok(_)) => {}
            }
        }
    })
    .await;
    assert_eq!(closed, Ok(true), "senza accesso aperto il flusso deve chiudersi");
}
