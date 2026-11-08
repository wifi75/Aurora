//! Le rotte del QMAP (ricezione panoramica Q65): elenco dei messaggi trovati nella fetta con lo spettro, configurazione della sorgente I/Q
//! (solo dal PC: apre collegamenti e file locali) e il clic su un messaggio, che porta la radio sulla sua frequenza in Q65 con periodo e
//! sottomodo giusti.

use axum::{Json, extract::State, http::HeaderMap, http::StatusCode};
use serde::Deserialize;
use serde_json::{Value, json};

use crate::AppState;
use crate::routes::{ApiError, Authed, Peer, halt_for_band_change, is_proxied, switch_mode};

/// Dove cade il tono 0 nell'audio della radio dopo il clic (come la decodifica a 12 kHz del QMAP).
pub const QMAP_AUDIO_HZ: u64 = 1_000;

fn service(state: &AppState) -> Result<std::sync::Arc<qmap::QmapService>, ApiError> {
    state.qmap.clone().ok_or_else(|| ApiError::new(StatusCode::NOT_FOUND, "qmap_unavailable"))
}

fn require_local(peer: Option<std::net::SocketAddr>, headers: &HeaderMap) -> Result<(), ApiError> {
    if peer.is_some_and(|p| p.ip().is_loopback()) && !is_proxied(headers) { Ok(()) } else { Err(ApiError::new(StatusCode::FORBIDDEN, "local_only")) }
}

/// Stato, configurazione, ultimo elenco e spettro.
#[utoipa::path(get, path = "/api/v1/qmap", responses((status = 200, description = "Stato del QMAP, messaggi dell'ultimo periodo e spettro della fetta"), (status = 404, description = "Non disponibile (qmap_unavailable)")))]
pub(crate) async fn qmap_get(State(state): State<AppState>, _: Authed) -> Result<Json<Value>, ApiError> {
    Ok(Json(json!(service(&state)?.snapshot())))
}

/// Nuova configurazione della sorgente I/Q (solo dal PC).
#[utoipa::path(put, path = "/api/v1/qmap", responses((status = 200, description = "Configurazione salvata, sorgente riavviata"), (status = 403, description = "Solo dal PC (local_only)"), (status = 500, description = "Salvataggio fallito (qmap_save)")))]
pub(crate) async fn qmap_put(State(state): State<AppState>, _: Authed, Peer(peer): Peer, headers: HeaderMap, Json(config): Json<qmap::QmapConfig>) -> Result<Json<Value>, ApiError> {
    let svc = service(&state)?;
    require_local(peer, &headers)?;
    let result = tokio::task::spawn_blocking(move || svc.set_config(config).map(|_| svc.snapshot())).await.map_err(|_| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "qmap_task"))?;
    match result {
        Ok(snap) => Ok(Json(json!(snap))),
        Err(e) => Err(ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "qmap_save").with_message(e)),
    }
}

/// Rilegge e decodifica il file scelto (sorgente «file»).
#[utoipa::path(post, path = "/api/v1/qmap/decode", responses((status = 200, description = "Decodifica avviata"), (status = 400, description = "File non leggibile (qmap_file)")))]
pub(crate) async fn qmap_decode(State(state): State<AppState>, _: Authed, Peer(peer): Peer, headers: HeaderMap) -> Result<Json<Value>, ApiError> {
    let svc = service(&state)?;
    require_local(peer, &headers)?;
    let result = tokio::task::spawn_blocking(move || svc.decode_file()).await.map_err(|_| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "qmap_task"))?;
    result.map(|_| Json(json!({ "ok": true }))).map_err(|e| ApiError::new(StatusCode::BAD_REQUEST, "qmap_file").with_message(e))
}

#[derive(Deserialize)]
pub(crate) struct TuneBody {
    /// Frequenza del tono 0 in radiofrequenza.
    rf_hz: u64,
    period_s: u32,
    submode: u8,
}

/// La frequenza di dial (USB) che mette il tono 0 a [`QMAP_AUDIO_HZ`].
pub fn dial_for(rf_hz: u64) -> Option<u64> {
    rf_hz.checked_sub(QMAP_AUDIO_HZ).filter(|&d| d > 0)
}

/// Clic su un messaggio: Q65 con periodo e sottomodo del messaggio, radio sulla sua frequenza (tono 0 a 1000 Hz), Rx a 1000 Hz.
#[utoipa::path(post, path = "/api/v1/qmap/tune", responses((status = 200, description = "Radio e modo impostati"), (status = 400, description = "Frequenza o modo non validi (bad_frequency, bad_mode)"), (status = 409, description = "In trasmissione (transmitting)"), (status = 502, description = "La radio non ha risposto (rig_error)"), (status = 503, description = "Nessuna radio (rig_unavailable)")))]
pub(crate) async fn qmap_tune(State(state): State<AppState>, _: Authed, Json(body): Json<TuneBody>) -> Result<Json<Value>, ApiError> {
    let rig = state.active_rig().ok_or_else(|| ApiError::new(StatusCode::SERVICE_UNAVAILABLE, "rig_unavailable"))?;
    let dial = dial_for(body.rf_hz).filter(|&d| rig_control::bands::amateur_band(d).is_some()).ok_or_else(|| ApiError::new(StatusCode::BAD_REQUEST, "bad_frequency"))?;
    if jt::q65::Setup::new(body.period_s, body.submode).is_none() {
        return Err(ApiError::new(StatusCode::BAD_REQUEST, "bad_mode"));
    }
    if state.current_status().is_some_and(|s| s.transmitting) {
        return Err(ApiError::new(StatusCode::CONFLICT, "transmitting"));
    }
    let mode = qmap::search::mode_name(body.period_s, body.submode);
    // prima il modo (che puo' spostare la radio sulla frequenza della tabella), poi la frequenza del messaggio
    if let Err(e) = switch_mode(&state, &mode).await {
        tracing::warn!("QMAP: cambio modo fallito: {e}");
        return Err(ApiError::new(if e == "rig_task" { StatusCode::INTERNAL_SERVER_ERROR } else { StatusCode::BAD_GATEWAY }, if e == "rig_task" { "rig_task" } else { "rig_error" }));
    }
    halt_for_band_change(&state, dial);
    let work = move || -> Result<(), rig_control::RigError> {
        rig.set_frequency(dial)?;
        if let Err(e) = rig.set_rx_frequency(QMAP_AUDIO_HZ as u32) {
            tracing::warn!("QMAP: frequenza Rx non impostata: {e}");
        }
        Ok(())
    };
    match tokio::task::spawn_blocking(work).await {
        Ok(Ok(())) => Ok(Json(json!({ "ok": true, "dial_hz": dial, "mode": mode, "audio_hz": QMAP_AUDIO_HZ }))),
        Ok(Err(e)) => {
            tracing::warn!("QMAP: cambio frequenza fallito: {e}");
            Err(ApiError::new(StatusCode::BAD_GATEWAY, "rig_error"))
        }
        Err(_) => Err(ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "rig_task")),
    }
}

#[cfg(test)]
mod tests {
    use super::dial_for;

    #[test]
    fn the_sync_tone_lands_at_1000_hz() {
        assert_eq!(dial_for(144_120_500), Some(144_119_500));
        assert_eq!(dial_for(500), None);
    }
}
