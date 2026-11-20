//! Le rotte del WEFAX (crate `mode-wefax`): stato e impostazioni, ascolto, immagine in arrivo, archivio delle cartine. Solo ricezione.
//! Le righe in arrivo e lo stato viaggiano sul flusso WebSocket come eventi `wefax`. La cartella delle cartine si cambia solo dal PC.

use std::sync::Arc;

use axum::{
    Json,
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode, header},
    response::{IntoResponse, Response},
};
use mode_wefax::{WefaxError, WefaxService, WefaxSettings};
use serde::Deserialize;
use serde_json::{Value, json};

use crate::{
    AppState,
    routes::{ApiError, Authed, Peer, is_proxied},
};

fn service(state: &AppState) -> Result<Arc<WefaxService>, ApiError> {
    state.wefax.clone().ok_or_else(|| ApiError::new(StatusCode::NOT_FOUND, "wefax_unavailable"))
}

fn api_error(e: WefaxError) -> ApiError {
    let status = match e {
        WefaxError::NotFound => StatusCode::NOT_FOUND,
        WefaxError::Io(_) => StatusCode::INTERNAL_SERVER_ERROR,
        _ => StatusCode::BAD_REQUEST,
    };
    ApiError::new(status, e.code()).with_message(e.to_string())
}

async fn blocking<T: Send + 'static>(work: impl FnOnce() -> T + Send + 'static) -> Result<T, ApiError> {
    tokio::task::spawn_blocking(work).await.map_err(|_| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "wefax_task"))
}

fn png(bytes: Vec<u8>) -> Response {
    ([(header::CONTENT_TYPE, "image/png"), (header::CACHE_CONTROL, "no-store")], bytes).into_response()
}

/// Stato della ricezione, impostazioni, IOC e valori ammessi.
#[utoipa::path(get, path = "/api/v1/wefax", responses((status = 200, description = "`{rx, settings, folder, iocs, lpm_values, filters}`"), (status = 404, description = "Non disponibile (wefax_unavailable)")))]
pub(crate) async fn wefax_get(State(state): State<AppState>, _: Authed) -> Result<Json<Value>, ApiError> {
    let svc = service(&state)?;
    Ok(Json(json!(svc.snapshot())))
}

/// Salva le impostazioni (la cartella delle cartine solo dal PC).
#[utoipa::path(put, path = "/api/v1/wefax", responses((status = 200, description = "Salvate: come `GET`"), (status = 400, description = "Valori non validi (bad_settings)"), (status = 403, description = "Cartella cambiata da fuori dal PC (local_only)")))]
pub(crate) async fn wefax_put(State(state): State<AppState>, _: Authed, Peer(peer): Peer, headers: HeaderMap, Json(settings): Json<WefaxSettings>) -> Result<Json<Value>, ApiError> {
    let svc = service(&state)?;
    let local = peer.is_some_and(|p| p.ip().is_loopback()) && !is_proxied(&headers);
    if !local && settings.folder != svc.settings().folder {
        return Err(ApiError::new(StatusCode::FORBIDDEN, "local_only"));
    }
    let s = Arc::clone(&svc);
    blocking(move || s.apply_settings(settings)).await?.map_err(api_error)?;
    Ok(Json(json!(svc.snapshot())))
}

/// Accende l'ascolto (una copia dell'audio d'ingresso va al ricevitore WEFAX).
#[utoipa::path(post, path = "/api/v1/wefax/rx/start", responses((status = 200, description = "Ascolto acceso")))]
pub(crate) async fn wefax_rx_start(State(state): State<AppState>, _: Authed) -> Result<Json<Value>, ApiError> {
    service(&state)?.start_rx();
    Ok(Json(json!({ "ok": true })))
}

/// Spegne l'ascolto e butta l'immagine in arrivo.
#[utoipa::path(post, path = "/api/v1/wefax/rx/stop", responses((status = 200, description = "Ascolto spento")))]
pub(crate) async fn wefax_rx_stop(State(state): State<AppState>, _: Authed) -> Result<Json<Value>, ApiError> {
    let svc = service(&state)?;
    blocking(move || svc.stop_rx()).await?;
    Ok(Json(json!({ "ok": true })))
}

/// La pagina aperta (identificativo scelto dalla pagina, uno per finestra o telefono).
#[derive(Deserialize)]
pub(crate) struct PageQuery {
    id: String,
}

/// Battito della pagina `id` (all'apertura e ogni 20 s): finche' una pagina batte l'ascolto non si spegne da solo. Non accende l'ascolto.
#[utoipa::path(post, path = "/api/v1/wefax/rx/hold", params(("id" = String, Query, description = "Identificativo della pagina")), responses((status = 200, description = "Pagina contata")))]
pub(crate) async fn wefax_rx_hold(State(state): State<AppState>, _: Authed, Query(q): Query<PageQuery>) -> Result<Json<Value>, ApiError> {
    service(&state)?.hold_rx(&q.id);
    Ok(Json(json!({ "ok": true })))
}

/// La pagina `id` e' stata lasciata: se non ne resta nessuna l'ascolto si spegne, salvo «Ascolto acceso all'avvio».
#[utoipa::path(post, path = "/api/v1/wefax/rx/release", params(("id" = String, Query, description = "Identificativo della pagina")), responses((status = 200, description = "Pagina tolta")))]
pub(crate) async fn wefax_rx_release(State(state): State<AppState>, _: Authed, Query(q): Query<PageQuery>) -> Result<Json<Value>, ApiError> {
    let svc = service(&state)?;
    blocking(move || svc.release_rx(&q.id)).await?;
    Ok(Json(json!({ "ok": true })))
}

/// Comincia subito un'immagine, senza aspettare APT start e fasatura (riga non allineata), accendendo l'ascolto.
#[utoipa::path(post, path = "/api/v1/wefax/rx/force", responses((status = 200, description = "Ricezione avviata")))]
pub(crate) async fn wefax_rx_force(State(state): State<AppState>, _: Authed) -> Result<Json<Value>, ApiError> {
    let svc = service(&state)?;
    blocking(move || svc.force_rx()).await?;
    Ok(Json(json!({ "ok": true })))
}

/// Chiude e salva l'immagine in corso com'e' (in modo manuale e' l'unico modo di finirla, oltre al massimo di righe).
#[utoipa::path(post, path = "/api/v1/wefax/rx/finish", responses((status = 200, description = "Richiesta inviata: l'esito arriva con gli eventi `rx_done` o `rx_lost`")))]
pub(crate) async fn wefax_rx_finish(State(state): State<AppState>, _: Authed) -> Result<Json<Value>, ApiError> {
    let svc = service(&state)?;
    blocking(move || svc.finish_rx()).await?;
    Ok(Json(json!({ "ok": true })))
}

/// Butta l'immagine in arrivo (l'ascolto resta acceso).
#[utoipa::path(post, path = "/api/v1/wefax/rx/cancel", responses((status = 200, description = "Immagine scartata")))]
pub(crate) async fn wefax_rx_cancel(State(state): State<AppState>, _: Authed) -> Result<Json<Value>, ApiError> {
    let svc = service(&state)?;
    blocking(move || svc.cancel_rx()).await?;
    Ok(Json(json!({ "ok": true })))
}

/// L'immagine in arrivo (o l'ultima finita), in PNG in grigi.
#[utoipa::path(get, path = "/api/v1/wefax/rx/current", responses((status = 200, description = "PNG"), (status = 404, description = "Nessuna immagine (not_found)")))]
pub(crate) async fn wefax_rx_current(State(state): State<AppState>, _: Authed) -> Result<Response, ApiError> {
    let svc = service(&state)?;
    let bytes = blocking(move || svc.current_png()).await?.ok_or_else(|| api_error(WefaxError::NotFound))?;
    Ok(png(bytes))
}

/// Le cartine ricevute, dalla piu' recente.
#[utoipa::path(get, path = "/api/v1/wefax/images", responses((status = 200, description = "`{images: [{id, file, ioc, lpm, width, height, time_ms, correlation, reason, source, complete}]}`")))]
pub(crate) async fn wefax_images(State(state): State<AppState>, _: Authed) -> Result<Json<Value>, ApiError> {
    let svc = service(&state)?;
    Ok(Json(json!({ "images": svc.images() })))
}

#[derive(Deserialize)]
pub(crate) struct DeleteMany {
    #[serde(default)]
    ids: Vec<String>,
}

/// Cancella piu' cartine dell'archivio: `{ids: [..]}` → `{deleted}`.
#[utoipa::path(post, path = "/api/v1/wefax/images/delete", responses((status = 200, description = "`{deleted}`")))]
pub(crate) async fn wefax_images_delete(State(state): State<AppState>, _: Authed, Json(body): Json<DeleteMany>) -> Result<Json<Value>, ApiError> {
    let svc = service(&state)?;
    let deleted = blocking(move || svc.delete_images(&body.ids)).await?;
    Ok(Json(json!({ "deleted": deleted })))
}

/// Una cartina dell'archivio in PNG.
#[utoipa::path(get, path = "/api/v1/wefax/images/{id}", responses((status = 200, description = "PNG"), (status = 404, description = "Non trovata (not_found)")))]
pub(crate) async fn wefax_image(State(state): State<AppState>, _: Authed, Path(id): Path<String>) -> Result<Response, ApiError> {
    let svc = service(&state)?;
    let bytes = blocking(move || svc.image_png(&id)).await?.map_err(api_error)?;
    Ok(png(bytes))
}

/// Cancella una cartina dell'archivio.
#[utoipa::path(delete, path = "/api/v1/wefax/images/{id}", responses((status = 200, description = "Cancellata"), (status = 404, description = "Non trovata (not_found)")))]
pub(crate) async fn wefax_image_delete(State(state): State<AppState>, _: Authed, Path(id): Path<String>) -> Result<Json<Value>, ApiError> {
    let svc = service(&state)?;
    blocking(move || svc.delete_image(&id)).await?.map_err(api_error)?;
    Ok(Json(json!({ "ok": true })))
}
