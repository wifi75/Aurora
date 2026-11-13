//! Le rotte dei satelliti (crate `mode-sat`): stato e impostazioni, passaggi, traccia a terra, posizioni, dati delle orbite e mappe del mondo.
//! L'interfaccia le legge una volta al secondo: il tracking vero (Doppler sulla radio, rotore) lo fa il servizio, non la pagina.

use std::sync::Arc;

use axum::{
    Json,
    body::Bytes,
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode, header},
    response::{IntoResponse, Response},
};
use mode_sat::{SatService, SatSettings};
use serde::Deserialize;
use serde_json::{Value, json};

use crate::{
    AppState,
    routes::{ApiError, Authed, Peer, is_proxied},
};

fn service(state: &AppState) -> Result<Arc<SatService>, ApiError> {
    state.sat.clone().ok_or_else(|| ApiError::new(StatusCode::NOT_FOUND, "sat_unavailable"))
}

async fn blocking<T: Send + 'static>(work: impl FnOnce() -> T + Send + 'static) -> Result<T, ApiError> {
    tokio::task::spawn_blocking(work).await.map_err(|_| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "sat_task"))
}

/// Tutto quello che serve alla pagina: osservatore, dati delle orbite, elenco e catalogo, impostazioni, stato di adesso, Sole e Luna, mappe.
#[utoipa::path(get, path = "/api/v1/sat", responses((status = 200, description = "`{observer, tle, listing, catalog, settings, now, sky, details, maps}`"), (status = 404, description = "Non disponibile (sat_unavailable)")))]
pub(crate) async fn sat_get(State(state): State<AppState>, _: Authed) -> Result<Json<Value>, ApiError> {
    let svc = service(&state)?;
    // la pagina legge una volta al secondo: ogni lettura tiene acceso il calcolo dei satelliti (senza letture, a pagina chiusa, dorme)
    svc.touch();
    let s = svc.settings();
    let (count, age) = svc.tle_status();
    let listing: Vec<Value> = svc.listing().into_iter().map(|(norad, name, has_tle)| json!({ "norad": norad, "name": name, "has_tle": has_tle })).collect();
    Ok(Json(json!({
        "observer": svc.observer().map(|o| json!({ "lat_deg": o.lat_deg, "lon_deg": o.lon_deg })),
        "tle": { "count": count, "age_s": age, "presets": mode_sat::tle::PRESETS.iter().map(|(name, url)| json!({ "name": name, "url": url })).collect::<Vec<_>>() },
        "listing": listing,
        "catalog": svc.catalog(),
        "settings": s,
        "now": svc.now(),
        "sky": svc.sky(),
        "details": s.selected.and_then(|n| svc.sat_details(n)),
        "maps": svc.maps(),
    })))
}

/// Salva le impostazioni (satellite scelto, spuntati, transponder, frequenza, Doppler, rotore...).
#[utoipa::path(put, path = "/api/v1/sat", responses((status = 200, description = "Salvate: come `GET`"), (status = 400, description = "Valori non validi (bad_value)")))]
pub(crate) async fn sat_put(State(state): State<AppState>, a: Authed, Json(settings): Json<SatSettings>) -> Result<Json<Value>, ApiError> {
    let svc = service(&state)?;
    let s = Arc::clone(&svc);
    blocking(move || s.apply_settings(settings)).await?.map_err(|e| ApiError::new(StatusCode::BAD_REQUEST, "bad_value").with_message(e))?;
    sat_get(State(state), a).await
}

/// Scarica di nuovo i dati delle orbite da CelesTrak.
#[utoipa::path(post, path = "/api/v1/sat/tle/refresh", responses((status = 200, description = "`{count}`"), (status = 502, description = "Scarico non riuscito (tle_failed)")))]
pub(crate) async fn sat_refresh(State(state): State<AppState>, _: Authed) -> Result<Json<Value>, ApiError> {
    let svc = service(&state)?;
    let count = blocking(move || svc.refresh_tle()).await?.map_err(|e| ApiError::new(StatusCode::BAD_GATEWAY, "tle_failed").with_message(e))?;
    Ok(Json(json!({ "count": count })))
}

#[derive(Deserialize)]
pub(crate) struct PassQuery {
    norad: u64,
    #[serde(default)]
    hours: Option<f64>,
    #[serde(default)]
    min_el: Option<f64>,
}

/// I passaggi di un satellite nelle prossime ore, con orari, azimut ed elevazione.
#[utoipa::path(get, path = "/api/v1/sat/passes", responses((status = 200, description = "`{passes: [{aos_unix, los_unix, max_el_deg, max_el_unix, aos_az_deg, los_az_deg, max_az_deg, duration_s}]}`")))]
pub(crate) async fn sat_passes(State(state): State<AppState>, _: Authed, Query(q): Query<PassQuery>) -> Result<Json<Value>, ApiError> {
    let svc = service(&state)?;
    let passes = blocking(move || svc.passes(q.norad, q.hours.unwrap_or(24.0), q.min_el.unwrap_or(0.0))).await?;
    Ok(Json(json!({ "passes": passes })))
}

#[derive(Deserialize)]
pub(crate) struct TrackQuery {
    norad: u64,
    #[serde(default)]
    minutes: Option<f64>,
}

/// La traccia a terra di un satellite (latitudine e longitudine ogni 30 secondi) attorno ad adesso.
#[utoipa::path(get, path = "/api/v1/sat/track", responses((status = 200, description = "`{points: [[lat, lon], ...]}`")))]
pub(crate) async fn sat_track(State(state): State<AppState>, _: Authed, Query(q): Query<TrackQuery>) -> Result<Json<Value>, ApiError> {
    let svc = service(&state)?;
    let points = blocking(move || svc.track(q.norad, q.minutes.unwrap_or(100.0).clamp(10.0, 300.0))).await?;
    Ok(Json(json!({ "points": points })))
}

/// Dove sono adesso i satelliti spuntati e quello scelto, con l'impronta radio.
#[utoipa::path(get, path = "/api/v1/sat/positions", responses((status = 200, description = "`{positions: [{norad, name, lat_deg, lon_deg, alt_km, az_deg, el_deg, footprint_km, visible}]}`")))]
pub(crate) async fn sat_positions(State(state): State<AppState>, _: Authed) -> Result<Json<Value>, ApiError> {
    let svc = service(&state)?;
    Ok(Json(json!({ "positions": svc.positions() })))
}

/// I nomi delle mappe del mondo disponibili.
#[utoipa::path(get, path = "/api/v1/sat/maps", responses((status = 200, description = "`{maps: [nome]}`")))]
pub(crate) async fn sat_maps(State(state): State<AppState>, _: Authed) -> Result<Json<Value>, ApiError> {
    let svc = service(&state)?;
    Ok(Json(json!({ "maps": svc.maps() })))
}

/// Una mappa del mondo (immagine equirettangolare).
#[utoipa::path(get, path = "/api/v1/sat/maps/{name}", responses((status = 200, description = "L'immagine"), (status = 404, description = "Non trovata (not_found)")))]
pub(crate) async fn sat_map(State(state): State<AppState>, _: Authed, Path(name): Path<String>) -> Result<Response, ApiError> {
    let svc = service(&state)?;
    let (ct, bytes) = blocking(move || svc.map_bytes(&name)).await?.ok_or_else(|| ApiError::new(StatusCode::NOT_FOUND, "not_found"))?;
    Ok(([(header::CONTENT_TYPE, ct), (header::CACHE_CONTROL, "public, max-age=3600")], bytes).into_response())
}

#[derive(Deserialize)]
pub(crate) struct MapQuery {
    name: String,
}

/// Aggiunge una mappa (`?name=Mia.jpg`, corpo: l'immagine, al massimo 20 MB). Solo dal PC stesso.
#[utoipa::path(post, path = "/api/v1/sat/maps", responses((status = 200, description = "Salvata"), (status = 400, description = "Nome o tipo non validi (bad_value)"), (status = 403, description = "Solo dal PC (local_only)")))]
pub(crate) async fn sat_map_upload(State(state): State<AppState>, _: Authed, Peer(peer): Peer, headers: HeaderMap, Query(q): Query<MapQuery>, body: Bytes) -> Result<Json<Value>, ApiError> {
    let svc = service(&state)?;
    let local = peer.is_some_and(|p| p.ip().is_loopback()) && !is_proxied(&headers);
    if !local {
        return Err(ApiError::new(StatusCode::FORBIDDEN, "local_only"));
    }
    let s = Arc::clone(&svc);
    blocking(move || s.save_map(&q.name, &body)).await?.map_err(|e| ApiError::new(StatusCode::BAD_REQUEST, "bad_value").with_message(e))?;
    Ok(Json(json!({ "maps": svc.maps() })))
}
