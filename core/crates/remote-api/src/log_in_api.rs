//! Stato del ricevitore dei QSO dagli altri log (`qso_export::inbound`): per la riga della scheda «Programmi collegati».

use axum::{Json, extract::State};
use serde_json::{Value, json};

use crate::{AppState, routes::Authed};

/// `{enabled, listening, port, error, received, duplicates, discarded, last}`; `{available: false}` se il ricevitore non c'e' (motore JTDX).
pub(crate) async fn log_in_status(State(state): State<AppState>, _: Authed) -> Json<Value> {
    match &state.log_in {
        Some(status) => {
            let mut v = status();
            v["available"] = json!(true);
            Json(v)
        }
        None => Json(json!({ "available": false })),
    }
}
