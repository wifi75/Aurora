//! Le rotte del rotore d'antenna (Hamlib `rotctld`): stato, configurazione, «vai a» e «ferma». La configurazione si cambia solo dal PC
//! (loopback, mai da un proxy); girare e fermare l'antenna basta al dispositivo associato (non trasmette nulla). Le rotte non aspettano mai
//! il rotore: i comandi passano al suo thread e lo stato si rilegge con `GET`.

use std::sync::Arc;

use axum::{
    Json,
    extract::State,
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
};
use rig_control::rotator::{RotError, RotatorConfig, RotatorService};
use serde::Deserialize;
use serde_json::{Value, json};

use crate::{
    AppState,
    routes::{Authed, Peer, is_proxied},
};

pub(crate) struct RotApiError {
    status: StatusCode,
    code: &'static str,
    message: String,
}

impl From<RotError> for RotApiError {
    fn from(e: RotError) -> RotApiError {
        let status = match e.code {
            "not_connected" => StatusCode::CONFLICT,
            "rotctld_missing" => StatusCode::CONFLICT,
            "models_failed" => StatusCode::BAD_GATEWAY,
            "write_failed" => StatusCode::INTERNAL_SERVER_ERROR,
            _ => StatusCode::BAD_REQUEST,
        };
        RotApiError { status, code: e.code, message: e.message }
    }
}

impl IntoResponse for RotApiError {
    fn into_response(self) -> Response {
        (self.status, Json(json!({ "error": self.code, "message": self.message }))).into_response()
    }
}

fn service(state: &AppState) -> Result<Arc<RotatorService>, RotApiError> {
    state.rotator.clone().ok_or(RotApiError { status: StatusCode::NOT_FOUND, code: "rotator_unavailable", message: String::new() })
}

fn require_local(peer: Option<std::net::SocketAddr>, headers: &HeaderMap) -> Result<(), RotApiError> {
    if peer.is_some_and(|p| p.ip().is_loopback()) && !is_proxied(headers) {
        Ok(())
    } else {
        Err(RotApiError { status: StatusCode::FORBIDDEN, code: "local_only", message: String::new() })
    }
}

async fn blocking<T: Send + 'static>(work: impl FnOnce() -> T + Send + 'static) -> Result<T, RotApiError> {
    tokio::task::spawn_blocking(work).await.map_err(|_| RotApiError { status: StatusCode::INTERNAL_SERVER_ERROR, code: "rotator_task", message: String::new() })
}

/// Stato e configurazione.
#[utoipa::path(get, path = "/api/v1/rotator", responses((status = 200, description = "`{status, config}`"), (status = 404, description = "Non disponibile (rotator_unavailable)")))]
pub(crate) async fn rotator_get(State(state): State<AppState>, _: Authed) -> Result<Json<Value>, RotApiError> {
    let rot = service(&state)?;
    Ok(Json(json!(blocking(move || rot.snapshot()).await?)))
}

/// Salva la configurazione e rifa' il collegamento.
#[utoipa::path(put, path = "/api/v1/rotator", responses((status = 200, description = "Salvata: `{status, config}`"), (status = 400, description = "Valori non validi (bad_model, bad_port, bad_baud, bad_host, bad_tcp_port)"), (status = 403, description = "Solo dal PC (local_only)"), (status = 500, description = "Scrittura non riuscita (write_failed)")))]
pub(crate) async fn rotator_put(State(state): State<AppState>, _: Authed, Peer(peer): Peer, headers: HeaderMap, Json(config): Json<RotatorConfig>) -> Result<Json<Value>, RotApiError> {
    let rot = service(&state)?;
    require_local(peer, &headers)?;
    let snapshot = blocking(move || rot.apply_config(config).map(|_| rot.snapshot())).await??;
    Ok(Json(json!(snapshot)))
}

#[derive(Deserialize)]
pub(crate) struct GotoBody {
    az: f64,
    #[serde(default)]
    el: Option<f64>,
}

/// Manda l'antenna a un azimut.
#[utoipa::path(post, path = "/api/v1/rotator/goto", responses((status = 200, description = "Comando consegnato al rotore: `{status}`"), (status = 400, description = "Azimut o elevazione non validi (bad_azimuth, bad_elevation)"), (status = 409, description = "Rotore non collegato (not_connected)")))]
pub(crate) async fn rotator_goto(State(state): State<AppState>, _: Authed, Json(body): Json<GotoBody>) -> Result<Json<Value>, RotApiError> {
    let rot = service(&state)?;
    let status = blocking(move || rot.goto(body.az, body.el).map(|_| rot.status())).await??;
    Ok(Json(json!({ "status": status })))
}

/// Ferma il rotore.
#[utoipa::path(post, path = "/api/v1/rotator/stop", responses((status = 200, description = "Comando consegnato al rotore: `{status}`"), (status = 409, description = "Rotore non collegato (not_connected)")))]
pub(crate) async fn rotator_stop(State(state): State<AppState>, _: Authed) -> Result<Json<Value>, RotApiError> {
    let rot = service(&state)?;
    let status = blocking(move || rot.stop().map(|_| rot.status())).await??;
    Ok(Json(json!({ "status": status })))
}

/// I modelli di rotore di Hamlib (`rotctld -l`).
#[utoipa::path(get, path = "/api/v1/rotator/models", responses((status = 200, description = "`{models: [{id, manufacturer, model, status}]}`"), (status = 409, description = "rotctld non trovato (rotctld_missing)"), (status = 502, description = "Elenco non letto (models_failed)")))]
pub(crate) async fn rotator_models(State(state): State<AppState>, _: Authed) -> Result<Json<Value>, RotApiError> {
    let rot = service(&state)?;
    let models = blocking(move || rot.models()).await??;
    Ok(Json(json!({ "models": models })))
}
