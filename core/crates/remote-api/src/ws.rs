use std::{sync::atomic::Ordering, time::Duration};

use auth::DeviceId;
use aurora_core::Event;
use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        State,
    },
    response::Response,
};
use tokio::{sync::broadcast::error::RecvError, time::interval};

use crate::{frames::encode_spectrum, routes::Authed, AppState};

const SEND_TIMEOUT: Duration = Duration::from_secs(5);
const REVOCATION_CHECK: Duration = Duration::from_secs(1);

#[utoipa::path(get, path = "/api/v1/stream", responses((status = 101, description = "WebSocket: eventi in JSON, spettro in binario"), (status = 401, description = "Non autenticato")))]
pub async fn stream(State(state): State<AppState>, Authed(device): Authed, ws: WebSocketUpgrade) -> Response {
    ws.on_upgrade(move |socket| run(socket, state, device))
}

/// Invia un messaggio; un client troppo lento (o caduto) fa chiudere solo la sua connessione.
async fn send(socket: &mut WebSocket, message: Message) -> bool {
    matches!(tokio::time::timeout(SEND_TIMEOUT, socket.send(message)).await, Ok(Ok(())))
}

async fn send_event(socket: &mut WebSocket, event: &Event) -> bool {
    match serde_json::to_string(event) {
        Ok(json) => send(socket, Message::Text(json.into())).await,
        Err(_) => true,
    }
}

async fn run(mut socket: WebSocket, state: AppState, device: DeviceId) {
    let mut events = state.bus.subscribe_events();
    let mut spectrum = state.bus.subscribe_spectrum();
    let mut check = interval(REVOCATION_CHECK);
    let mut stop = state.subscribe_shutdown();
    // finche' il flusso e' aperto l'automazione sa che c'e' qualcuno (dead-man)
    let _present = state.automation.as_ref().map(|a| a.client_guard());
    if *stop.borrow() {
        return;
    }

    if let Some(status) = state.current_status()
        && !send_event(&mut socket, &Event::Status(status)).await
    {
        return;
    }

    loop {
        tokio::select! {
            received = events.recv() => match received {
                Ok(event) => if !send_event(&mut socket, &event).await { return; },
                // Un client in ritardo riparte dallo stato corrente, non dagli arretrati.
                Err(RecvError::Lagged(_)) => {
                    if let Some(status) = state.current_status()
                        && !send_event(&mut socket, &Event::Status(status)).await
                    {
                        return;
                    }
                }
                Err(RecvError::Closed) => return,
            },
            received = spectrum.recv() => match received {
                Ok(row) => {
                    if let Err(e) = row.validate() {
                        tracing::warn!("riga di spettro scartata: {e}");
                    } else if !send(&mut socket, Message::Binary(encode_spectrum(&row).into())).await {
                        return;
                    }
                }
                Err(RecvError::Lagged(_)) => {}
                Err(RecvError::Closed) => return,
            },
            incoming = socket.recv() => match incoming {
                None | Some(Err(_)) | Some(Ok(Message::Close(_))) => return,
                Some(Ok(_)) => {}
            },
            _ = stop.changed() => {
                let _ = socket.send(Message::Close(None)).await;
                return;
            }
            _ = check.tick() => {
                // L'accesso senza codice usa un dispositivo virtuale che non sta nell'archivio:
                // vale finche' resta acceso, non finche' esiste una registrazione.
                let still_valid = if device == DeviceId(0) && (state.local_trust || state.open_access.load(Ordering::SeqCst)) {
                    true
                } else {
                    state.auth.exists(device)
                };
                if !still_valid {
                    let _ = socket.send(Message::Close(None)).await;
                    return;
                }
            }
        }
    }
}
