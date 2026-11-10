//! Le rotte dei modi testuali (crate `mode-digi`: PSK31, PSK63, RTTY): stato e impostazioni, ascolto, trasmissione e STOP. Il testo ricevuto
//! e lo stato viaggiano sul flusso WebSocket come eventi `digi`. Trasmettere segue le stesse regole delle altre trasmissioni (`check_tx_allowed`).

use std::sync::Arc;

use axum::{Json, extract::State, http::{HeaderMap, StatusCode}};
use std::path::PathBuf;
use mode_digi::{DigiError, DigiService, DigiSettings};
use serde::Deserialize;
use serde_json::{Value, json};

use crate::{
    AppState,
    routes::{ApiError, Authed, Peer, check_tx_allowed, is_proxied},
};

/// Lunghezza massima del testo di una trasmissione.
const TEXT_MAX: usize = 4000;

fn service(state: &AppState) -> Result<Arc<DigiService>, ApiError> {
    state.digi.clone().ok_or_else(|| ApiError::new(StatusCode::NOT_FOUND, "digi_unavailable"))
}

fn api_error(e: DigiError) -> ApiError {
    let status = match e {
        DigiError::Busy | DigiError::NoTransmitter => StatusCode::CONFLICT,
        DigiError::Io(_) => StatusCode::INTERNAL_SERVER_ERROR,
        DigiError::Tx(_) => StatusCode::BAD_GATEWAY,
        _ => StatusCode::BAD_REQUEST,
    };
    ApiError::new(status, e.code()).with_message(e.to_string())
}

async fn blocking<T: Send + 'static>(work: impl FnOnce() -> T + Send + 'static) -> Result<T, ApiError> {
    tokio::task::spawn_blocking(work).await.map_err(|_| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "digi_task"))
}

/// Stato della ricezione e della trasmissione, impostazioni (modo, frequenze, macro) e modi disponibili.
#[utoipa::path(get, path = "/api/v1/digi", responses((status = 200, description = "`{rx, tx, settings, can_transmit, modes}`"), (status = 404, description = "Non disponibile (digi_unavailable)")))]
pub(crate) async fn digi_get(State(state): State<AppState>, _: Authed) -> Result<Json<Value>, ApiError> {
    let svc = service(&state)?;
    Ok(Json(json!(svc.snapshot())))
}

/// Salva le impostazioni (modo, frequenze, RTTY, macro).
#[utoipa::path(put, path = "/api/v1/digi", responses((status = 200, description = "Salvate: come `GET`"), (status = 400, description = "Valori non validi (bad_value)")))]
pub(crate) async fn digi_put(State(state): State<AppState>, _: Authed, Json(settings): Json<DigiSettings>) -> Result<Json<Value>, ApiError> {
    let svc = service(&state)?;
    let s = Arc::clone(&svc);
    blocking(move || s.apply_settings(settings)).await?.map_err(api_error)?;
    Ok(Json(json!(svc.snapshot())))
}

/// Accende l'ascolto (una copia dell'audio d'ingresso va al decodificatore).
#[utoipa::path(post, path = "/api/v1/digi/rx/start", responses((status = 200, description = "Ascolto acceso")))]
pub(crate) async fn digi_rx_start(State(state): State<AppState>, _: Authed) -> Result<Json<Value>, ApiError> {
    service(&state)?.start_rx();
    Ok(Json(json!({ "ok": true })))
}

/// Spegne l'ascolto.
#[utoipa::path(post, path = "/api/v1/digi/rx/stop", responses((status = 200, description = "Ascolto spento")))]
pub(crate) async fn digi_rx_stop(State(state): State<AppState>, _: Authed) -> Result<Json<Value>, ApiError> {
    let svc = service(&state)?;
    blocking(move || svc.stop_rx()).await?;
    Ok(Json(json!({ "ok": true })))
}

#[derive(Deserialize)]
pub(crate) struct TxBody {
    text: String,
}

/// Trasmette un testo con il modo e la frequenza delle impostazioni.
#[utoipa::path(post, path = "/api/v1/digi/tx", responses((status = 200, description = "Trasmissione partita: stato `tx`"), (status = 400, description = "Testo vuoto o troppo lungo (empty_text, bad_value)"), (status = 409, description = "Occupato o senza trasmettitore (busy, no_transmitter)")))]
pub(crate) async fn digi_tx(State(state): State<AppState>, _: Authed, headers: HeaderMap, Json(body): Json<TxBody>) -> Result<Json<Value>, ApiError> {
    let svc = service(&state)?;
    check_tx_allowed(&state, &headers)?;
    if body.text.chars().count() > TEXT_MAX {
        return Err(api_error(DigiError::BadValue(format!("testo oltre {TEXT_MAX} caratteri"))));
    }
    let status = blocking(move || svc.transmit(&body.text)).await?.map_err(api_error)?;
    Ok(Json(json!({ "tx": status })))
}

/// Ferma la trasmissione (audio e PTT). Sempre consentito.
#[utoipa::path(post, path = "/api/v1/digi/tx/abort", responses((status = 200, description = "Fermata")))]
pub(crate) async fn digi_tx_abort(State(state): State<AppState>, _: Authed) -> Result<Json<Value>, ApiError> {
    let svc = service(&state)?;
    blocking(move || svc.abort_tx()).await?;
    Ok(Json(json!({ "ok": true })))
}

#[derive(Deserialize, Default)]
pub(crate) struct ImportBody {
    /// La cartella dei file; senza, `%APPDATA%\`.
    #[serde(default)]
    folder: Option<String>,
}

/// Importa le macro (`Default.mc`, `rtty.mc`, `bpsk31.mc`, `bpsk63.mc`...) dalla cartella dell'utente. Solo dal PC stesso.
#[utoipa::path(post, path = "/api/v1/digi/import-macros", responses((status = 200, description = "`{imported: {folder, default_count, default_file, modes, skipped}, ...snapshot}`"), (status = 400, description = "Cartella non trovata o senza file (bad_value)"), (status = 403, description = "Solo dal PC (local_only)")))]
pub(crate) async fn digi_import_macros(State(state): State<AppState>, _: Authed, Peer(peer): Peer, headers: HeaderMap, body: Option<Json<ImportBody>>) -> Result<Json<Value>, ApiError> {
    let svc = service(&state)?;
    let local = peer.is_some_and(|p| p.ip().is_loopback()) && !is_proxied(&headers);
    if !local {
        return Err(ApiError::new(StatusCode::FORBIDDEN, "local_only"));
    }
    let folder = body.and_then(|b| b.0.folder).filter(|f| !f.trim().is_empty()).map(PathBuf::from);
    let s = Arc::clone(&svc);
    let report = blocking(move || s.import_macros(folder.as_deref())).await?.map_err(api_error)?;
    Ok(Json(json!({ "imported": report, "snapshot": svc.snapshot() })))
}
