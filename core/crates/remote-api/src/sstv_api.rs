//! Le rotte dell'SSTV (crate `mode-sstv`): stato e impostazioni, ascolto, immagine in arrivo, archivio delle immagini, trasmissione
//! e STOP. Le righe in arrivo e lo stato viaggiano sul flusso WebSocket come eventi `sstv`. Trasmettere segue le stesse regole delle
//! altre trasmissioni (`check_tx_allowed`); la cartella delle immagini si cambia solo dal PC.

use std::sync::Arc;

use axum::{
    Json,
    body::Bytes,
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode, header},
    response::{IntoResponse, Response},
};
use mode_sstv::{SstvError, SstvMode, SstvService, SstvSettings};
use serde::Deserialize;
use serde_json::{Value, json};

use crate::{
    AppState,
    routes::{ApiError, Authed, Peer, check_tx_allowed, is_proxied},
};

/// Dimensione massima di un'immagine da trasmettere.
pub(crate) const SSTV_UPLOAD_MAX_BYTES: usize = 16 * 1024 * 1024;

fn service(state: &AppState) -> Result<Arc<SstvService>, ApiError> {
    state.sstv.clone().ok_or_else(|| ApiError::new(StatusCode::NOT_FOUND, "sstv_unavailable"))
}

fn api_error(e: SstvError) -> ApiError {
    let status = match e {
        SstvError::NotFound => StatusCode::NOT_FOUND,
        SstvError::Busy | SstvError::NoTransmitter => StatusCode::CONFLICT,
        SstvError::Io(_) => StatusCode::INTERNAL_SERVER_ERROR,
        SstvError::Tx(_) => StatusCode::BAD_GATEWAY,
        _ => StatusCode::BAD_REQUEST,
    };
    ApiError::new(status, e.code()).with_message(e.to_string())
}

async fn blocking<T: Send + 'static>(work: impl FnOnce() -> T + Send + 'static) -> Result<T, ApiError> {
    tokio::task::spawn_blocking(work).await.map_err(|_| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "sstv_task"))
}

fn png(bytes: Vec<u8>) -> Response {
    ([(header::CONTENT_TYPE, "image/png"), (header::CACHE_CONTROL, "no-store")], bytes).into_response()
}

/// Stato della ricezione e della trasmissione, impostazioni, modi.
#[utoipa::path(get, path = "/api/v1/sstv", responses((status = 200, description = "`{rx, tx, settings, folder, can_transmit, modes}`"), (status = 404, description = "Non disponibile (sstv_unavailable)")))]
pub(crate) async fn sstv_get(State(state): State<AppState>, _: Authed) -> Result<Json<Value>, ApiError> {
    let svc = service(&state)?;
    Ok(Json(json!(svc.snapshot())))
}

/// Salva le impostazioni (la cartella delle immagini solo dal PC).
#[utoipa::path(put, path = "/api/v1/sstv", responses((status = 200, description = "Salvate: come `GET`"), (status = 400, description = "Valori non validi (bad_image)"), (status = 403, description = "Cartella cambiata da fuori dal PC (local_only)")))]
pub(crate) async fn sstv_put(State(state): State<AppState>, _: Authed, Peer(peer): Peer, headers: HeaderMap, Json(settings): Json<SstvSettings>) -> Result<Json<Value>, ApiError> {
    let svc = service(&state)?;
    let local = peer.is_some_and(|p| p.ip().is_loopback()) && !is_proxied(&headers);
    if !local && settings.folder != svc.settings().folder {
        return Err(ApiError::new(StatusCode::FORBIDDEN, "local_only"));
    }
    let s = Arc::clone(&svc);
    blocking(move || s.apply_settings(settings)).await?.map_err(api_error)?;
    Ok(Json(json!(svc.snapshot())))
}

/// Accende l'ascolto (una copia dell'audio d'ingresso va al decodificatore SSTV).
#[utoipa::path(post, path = "/api/v1/sstv/rx/start", responses((status = 200, description = "Ascolto acceso")))]
pub(crate) async fn sstv_rx_start(State(state): State<AppState>, _: Authed) -> Result<Json<Value>, ApiError> {
    service(&state)?.start_rx();
    Ok(Json(json!({ "ok": true })))
}

/// Spegne l'ascolto e butta l'immagine in arrivo.
#[utoipa::path(post, path = "/api/v1/sstv/rx/stop", responses((status = 200, description = "Ascolto spento")))]
pub(crate) async fn sstv_rx_stop(State(state): State<AppState>, _: Authed) -> Result<Json<Value>, ApiError> {
    let svc = service(&state)?;
    blocking(move || svc.stop_rx()).await?;
    Ok(Json(json!({ "ok": true })))
}

#[derive(Deserialize)]
pub(crate) struct ForceBody {
    mode: String,
}

/// Riceve subito nel modo dato, senza aspettare il VIS.
#[utoipa::path(post, path = "/api/v1/sstv/rx/force", responses((status = 200, description = "Ricezione avviata"), (status = 400, description = "Modo sconosciuto (unknown_mode)")))]
pub(crate) async fn sstv_rx_force(State(state): State<AppState>, _: Authed, Json(body): Json<ForceBody>) -> Result<Json<Value>, ApiError> {
    let svc = service(&state)?;
    let mode = SstvMode::from_id(&body.mode).ok_or_else(|| api_error(SstvError::UnknownMode(body.mode.clone())))?;
    blocking(move || svc.force_rx(mode)).await?;
    Ok(Json(json!({ "ok": true })))
}

/// Butta l'immagine in arrivo (l'ascolto resta acceso).
#[utoipa::path(post, path = "/api/v1/sstv/rx/cancel", responses((status = 200, description = "Immagine scartata")))]
pub(crate) async fn sstv_rx_cancel(State(state): State<AppState>, _: Authed) -> Result<Json<Value>, ApiError> {
    let svc = service(&state)?;
    blocking(move || svc.cancel_rx()).await?;
    Ok(Json(json!({ "ok": true })))
}

/// L'immagine in arrivo (o l'ultima finita), in PNG.
#[utoipa::path(get, path = "/api/v1/sstv/rx/current", responses((status = 200, description = "PNG"), (status = 404, description = "Nessuna immagine (not_found)")))]
pub(crate) async fn sstv_rx_current(State(state): State<AppState>, _: Authed) -> Result<Response, ApiError> {
    let svc = service(&state)?;
    let bytes = blocking(move || svc.current_png()).await?.ok_or_else(|| api_error(SstvError::NotFound))?;
    Ok(png(bytes))
}

/// Le immagini ricevute e trasmesse, dalla piu' recente.
#[utoipa::path(get, path = "/api/v1/sstv/images", responses((status = 200, description = "`{images: [{id, direction, mode, width, height, time_ms, sync_quality, call}]}`")))]
pub(crate) async fn sstv_images(State(state): State<AppState>, _: Authed) -> Result<Json<Value>, ApiError> {
    let svc = service(&state)?;
    Ok(Json(json!({ "images": svc.images() })))
}

/// Le foto della cartella scelta per la trasmissione (Impostazioni SSTV): `{folder, files: [{name, size}]}`. Solo dal PC stesso.
#[utoipa::path(get, path = "/api/v1/sstv/tx-folder", responses((status = 200, description = "`{folder, files}`"), (status = 400, description = "Nessuna cartella scelta o cartella non trovata (bad_image)"), (status = 403, description = "Solo dal PC (local_only)")))]
pub(crate) async fn sstv_tx_folder(State(state): State<AppState>, _: Authed, Peer(peer): Peer, headers: HeaderMap) -> Result<Json<Value>, ApiError> {
    if !(peer.is_some_and(|p| p.ip().is_loopback()) && !is_proxied(&headers)) {
        return Err(ApiError::new(StatusCode::FORBIDDEN, "local_only"));
    }
    let svc = service(&state)?;
    let folder = svc.settings().tx_folder;
    let files = blocking(move || svc.tx_photos()).await?.map_err(api_error)?;
    Ok(Json(json!({ "folder": folder, "files": files })))
}

#[derive(Deserialize)]
pub(crate) struct PhotoQuery {
    name: String,
}

/// Una foto della cartella scelta per la trasmissione, con il suo tipo. Solo il nome del file, solo dal PC stesso.
#[utoipa::path(get, path = "/api/v1/sstv/tx-folder/file", responses((status = 200, description = "L'immagine"), (status = 403, description = "Solo dal PC (local_only)"), (status = 404, description = "Non trovata (not_found)")))]
pub(crate) async fn sstv_tx_photo(State(state): State<AppState>, _: Authed, Peer(peer): Peer, headers: HeaderMap, Query(q): Query<PhotoQuery>) -> Result<Response, ApiError> {
    if !(peer.is_some_and(|p| p.ip().is_loopback()) && !is_proxied(&headers)) {
        return Err(ApiError::new(StatusCode::FORBIDDEN, "local_only"));
    }
    let svc = service(&state)?;
    let (bytes, mime) = blocking(move || svc.tx_photo(&q.name)).await?.map_err(api_error)?;
    Ok(([(axum::http::header::CONTENT_TYPE, mime)], bytes).into_response())
}

/// Un'immagine dell'archivio in PNG.
#[utoipa::path(get, path = "/api/v1/sstv/images/{id}", responses((status = 200, description = "PNG"), (status = 404, description = "Non trovata (not_found)")))]
pub(crate) async fn sstv_image(State(state): State<AppState>, _: Authed, Path(id): Path<String>) -> Result<Response, ApiError> {
    let svc = service(&state)?;
    let bytes = blocking(move || svc.image_png(&id)).await?.map_err(api_error)?;
    Ok(png(bytes))
}

/// Cancella un'immagine dell'archivio.
#[utoipa::path(delete, path = "/api/v1/sstv/images/{id}", responses((status = 200, description = "Cancellata"), (status = 404, description = "Non trovata (not_found)")))]
pub(crate) async fn sstv_image_delete(State(state): State<AppState>, _: Authed, Path(id): Path<String>) -> Result<Json<Value>, ApiError> {
    let svc = service(&state)?;
    blocking(move || svc.delete_image(&id)).await?.map_err(api_error)?;
    Ok(Json(json!({ "ok": true })))
}

#[derive(Deserialize)]
pub(crate) struct TxQuery {
    mode: String,
    /// Il corrispondente, per la cronologia.
    #[serde(default)]
    call: Option<String>,
}

/// Trasmette un'immagine (corpo: PNG o JPEG; `?mode=PD120&call=IU3ABC`). Se non e' alla risoluzione del modo la si adatta.
#[utoipa::path(post, path = "/api/v1/sstv/tx", responses((status = 200, description = "Trasmissione partita: stato `tx`"), (status = 400, description = "Immagine o modo non validi (bad_image, unknown_mode)"), (status = 403, description = "Trasmissione non consentita (tx_disabled)"), (status = 409, description = "Gia' in trasmissione o catena non pronta (busy, no_transmitter)"), (status = 502, description = "La catena di trasmissione ha rifiutato (tx_failed)")))]
pub(crate) async fn sstv_tx(State(state): State<AppState>, _: Authed, headers: HeaderMap, Query(q): Query<TxQuery>, body: Bytes) -> Result<Json<Value>, ApiError> {
    let svc = service(&state)?;
    check_tx_allowed(&state, &headers)?;
    let mode = SstvMode::from_id(&q.mode).ok_or_else(|| api_error(SstvError::UnknownMode(q.mode.clone())))?;
    let call = q.call.filter(|c| c.len() <= 16 && c.chars().all(|ch| ch.is_ascii_alphanumeric() || ch == '/'));
    let status = blocking(move || svc.transmit(mode, &body, call)).await?.map_err(api_error)?;
    Ok(Json(json!({ "tx": status })))
}

/// Ferma la trasmissione SSTV (audio e PTT). Sempre consentito.
#[utoipa::path(post, path = "/api/v1/sstv/tx/abort", responses((status = 200, description = "Fermata")))]
pub(crate) async fn sstv_tx_abort(State(state): State<AppState>, _: Authed) -> Result<Json<Value>, ApiError> {
    let svc = service(&state)?;
    blocking(move || svc.abort_tx()).await?;
    Ok(Json(json!({ "ok": true })))
}
