use std::net::SocketAddr;

use auth::{AuthError, DeviceId};
use aurora_core::{
    version::{APP_AUTHOR, APP_NAME, APP_VERSION},
    Command,
};
use axum::{
    extract::{ConnectInfo, FromRequestParts, Path, State},
    http::{header, request::Parts, HeaderMap, HeaderValue, StatusCode},
    response::{IntoResponse, Response},
    routing::{delete, get, patch, post, put},
    Json, Router,
};
use qso_engine::{Mode, Rules};
use serde::Deserialize;
use serde_json::{json, Value};
use utoipa::OpenApi;

use crate::{lan, policy::transport_of, ws, AppState};

const COOKIE_NAME: &str = "aurora_session";
const COOKIE_MAX_AGE_SECS: u64 = 31_536_000;

#[derive(OpenApi)]
#[openapi(paths(crate::digi_api::digi_get, crate::digi_api::digi_put, crate::digi_api::digi_rx_start, crate::digi_api::digi_rx_stop, crate::digi_api::digi_tx, crate::digi_api::digi_tx_abort, crate::digi_api::digi_import_macros, crate::sstv_api::sstv_get, crate::sstv_api::sstv_put, crate::sstv_api::sstv_rx_start, crate::sstv_api::sstv_rx_stop, crate::sstv_api::sstv_rx_force, crate::sstv_api::sstv_rx_cancel, crate::sstv_api::sstv_rx_current, crate::sstv_api::sstv_images, crate::sstv_api::sstv_image, crate::sstv_api::sstv_image_delete, crate::sstv_api::sstv_tx, crate::sstv_api::sstv_tx_abort, crate::rotator_api::rotator_get, crate::rotator_api::rotator_put, crate::rotator_api::rotator_goto, crate::rotator_api::rotator_stop, crate::rotator_api::rotator_models, update_install, update_download, online_get, online_put, online_psk, online_solar, online_dxped, set_dx, set_report, tune, log_qso, fox_command, fox_clublog, online_refresh, history_get, history_stats, history_clear, output_level_get, output_level_put, input_level_get, input_level_put, controls_get, control_press, tx_message, tx_text, access_get, access_put, pairing_get, pairing_put, frequencies_get, frequencies_put, frequencies_defaults, schedule_get, schedule_put, stats, dxcc_matrix, log_qsos, log_add, log_delete, log_export, log_export_save, log4om_db_get, log4om_db_put, log4om_db_choose, diagnostics_get, diagnostics_crash_folder, diagnostics_support_bundle, diagnostics_support_reveal, reports_get, reports_put, reports_new_id, reports_crash_preview, reports_crash_send, reports_crash_dismiss, reports_bundle_send, log_pending, log_pending_answer, log_import, log4om, map_worked, wishlist_get, wishlist_put, wishlist_test, update_get, update_put, update_check, engine_settings_get, engine_settings_put, spot_dxsummit, audio_devices, audio_settings_put, dxcc_lookup, callsign_lookup, cluster_get, cluster_put, cluster_catalog, cluster_test, cluster_command, clock_get, clock_put, clock_check, clock_correct, ui_settings_get, ui_settings_put, band_filters_get, band_filters_put, change_band, change_frequency, change_mode, rx_frequency, free_text, call_cq, tx_frequency, station_info, health, pair_start, pair_finish, devices, revoke_device, halt_tx, reply, queue_reply, automation_get, automation_put, automation_countries, ws::stream, qso_upload_test, wavelog_stations, band_hop_next, crate::qmap_api::qmap_get, crate::qmap_api::qmap_put, crate::qmap_api::qmap_decode, crate::qmap_api::qmap_clear_average, crate::qmap_api::qmap_tune, crate::contest_api::contest_get, crate::contest_api::contest_regole, crate::contest_api::contest_attiva, crate::contest_api::contest_disattiva, crate::contest_api::contest_impostazioni, crate::contest_api::contest_valuta, crate::contest_api::contest_lavorati, crate::contest_api::contest_consiglio, crate::contest_api::contest_mappa, crate::contest_api::contest_log, crate::contest_api::contest_log_modifica, crate::contest_api::contest_log_azzera, crate::contest_api::contest_esporta, crate::contest_api::contest_calendario, crate::contest_api::contest_analisi))]
struct ApiDoc;

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/api/v1/health", get(health))
        .route("/api/v1/pair/start", post(pair_start))
        .route("/api/v1/lan/check", get(lan_check))
        .route("/api/v1/lan/allow", post(lan_allow))
        .route("/api/v1/pair/finish", post(pair_finish))
        .route("/api/v1/devices", get(devices))
        .route("/api/v1/devices/{id}", delete(revoke_device))
        .route("/api/v1/commands/halt-tx", post(halt_tx))
        .route("/api/v1/commands/reply", post(reply))
        .route("/api/v1/commands/queue-reply", post(queue_reply))
        .route("/api/v1/commands/free-text", post(free_text))
        .route("/api/v1/commands/dx", post(set_dx))
        .route("/api/v1/commands/report", post(set_report))
        .route("/api/v1/commands/tune", post(tune))
        .route("/api/v1/commands/log-qso", post(log_qso))
        .route("/api/v1/fox", post(fox_command))
        .route("/api/v1/fox/clublog", post(fox_clublog))
        .route("/api/v1/commands/cq", post(call_cq))
        .route("/api/v1/commands/tx-frequency", post(tx_frequency))
        .route("/api/v1/commands/band", post(change_band))
        .route("/api/v1/commands/band-hop-next", post(band_hop_next))
        .route("/api/v1/commands/frequency", post(change_frequency))
        .route("/api/v1/commands/mode", post(change_mode))
        .route("/api/v1/commands/rx-frequency", post(rx_frequency))
        .route("/api/v1/output-level", get(output_level_get).put(output_level_put))
        .route("/api/v1/input-level", get(input_level_get).put(input_level_put))
        .route("/api/v1/engine/native", get(native_engine_get))
        .route("/api/v1/online", get(online_get).put(online_put))
        .route("/api/v1/online/psk", get(online_psk))
        .route("/api/v1/online/solar", get(online_solar))
        .route("/api/v1/online/dxped", get(online_dxped))
        .route("/api/v1/online/refresh", post(online_refresh))
        .route("/api/v1/history", get(history_get).delete(history_clear))
        .route("/api/v1/history/stats", get(history_stats))
        .route("/api/v1/controls", get(controls_get))
        .route("/api/v1/commands/control", post(control_press))
        .route("/api/v1/commands/tx-message", post(tx_message))
        .route("/api/v1/commands/tx-text", post(tx_text))
        .route("/api/v1/frequencies", get(frequencies_get).put(frequencies_put))
        .route("/api/v1/frequencies/defaults", get(frequencies_defaults))
        .route("/api/v1/pairing-settings", get(pairing_get).put(pairing_put))
        .route("/api/v1/access", get(access_get).put(access_put))
        .route("/api/v1/schedule", get(schedule_get).put(schedule_put))
        .route("/api/v1/automation", get(automation_get).put(automation_put))
        .route("/api/v1/automation/countries", get(automation_countries))
        .route("/api/v1/station-info", get(station_info))
        .route("/api/v1/qso-upload/wavelog-stations", post(wavelog_stations))
        .route("/api/v1/stats", get(stats))
        .route("/api/v1/dxcc-matrix", get(dxcc_matrix))
        .route("/api/v1/log/qsos", get(log_qsos).post(log_add).delete(log_delete))
        .route("/api/v1/log/export.adi", get(log_export))
        .route("/api/v1/log/export-save", post(log_export_save))
        .route("/api/v1/log4om-db", get(log4om_db_get).put(log4om_db_put))
        .route("/api/v1/log4om-db/choose", post(log4om_db_choose))
        .route("/api/v1/diagnostics", get(diagnostics_get))
        .route("/api/v1/diagnostics/crash-folder", post(diagnostics_crash_folder))
        .route("/api/v1/diagnostics/support-bundle", post(diagnostics_support_bundle))
        .route("/api/v1/settings-backup", post(settings_backup))
        .route("/api/v1/settings-restore", post(settings_restore).layer(axum::extract::DefaultBodyLimit::max(SETTINGS_RESTORE_MAX_BYTES)))
        .route("/api/v1/diagnostics/support-bundle/reveal", post(diagnostics_support_reveal))
        .route("/api/v1/reports", get(reports_get).put(reports_put))
        .route("/api/v1/reports/install-id", post(reports_new_id))
        .route("/api/v1/reports/crash/preview", post(reports_crash_preview))
        .route("/api/v1/reports/crash/send", post(reports_crash_send))
        .route("/api/v1/reports/crash/dismiss", post(reports_crash_dismiss))
        .route("/api/v1/reports/bundle/send", post(reports_bundle_send))
        .route("/api/v1/log/pending", get(log_pending))
        .route("/api/v1/log/pending/{id}", post(log_pending_answer))
        .route("/api/v1/log/import", post(log_import).layer(axum::extract::DefaultBodyLimit::max(LOG_IMPORT_MAX_BYTES)))
        .route("/api/v1/log4om", get(log4om))
        .route("/api/v1/map/worked", get(map_worked))
        .route("/api/v1/wishlist", get(wishlist_get).put(wishlist_put))
        .route("/api/v1/wishlist/test", post(wishlist_test))
        .route("/api/v1/update", get(update_get).put(update_put))
        .route("/api/v1/update/check", post(update_check))
        .route("/api/v1/update/install", post(update_install))
        .route("/api/v1/update/download", post(update_download))
        .route("/api/v1/engine-settings", get(engine_settings_get).put(engine_settings_put))
        .route("/api/v1/spot/dxsummit", post(spot_dxsummit))
        .route("/api/v1/qso-upload/test", post(qso_upload_test))
        .route("/api/v1/audio/devices", get(audio_devices))
        .route("/api/v1/audio/settings", axum::routing::put(audio_settings_put))
        .route("/api/v1/dx-cluster", get(cluster_get).put(cluster_put))
        .route("/api/v1/dxcc/{call}", get(dxcc_lookup))
        .route("/api/v1/callsign/{call}", get(callsign_lookup))
        .route("/api/v1/dx-cluster/catalog", get(cluster_catalog))
        .route("/api/v1/dx-cluster/test", post(cluster_test))
        .route("/api/v1/dx-cluster/command", post(cluster_command))
        .route("/api/v1/clock", get(clock_get).put(clock_put))
        .route("/api/v1/clock/check", post(clock_check))
        .route("/api/v1/clock/correct", post(clock_correct))
        .route("/api/v1/ui-settings/{kind}", get(ui_settings_get).put(ui_settings_put))
        .route("/api/v1/band-filters", get(band_filters_get).put(band_filters_put))
        .route("/api/v1/cat", get(crate::cat::cat_get).put(crate::cat::cat_put))
        .route("/api/v1/cat/connect", post(crate::cat::cat_connect))
        .route("/api/v1/cat/disconnect", post(crate::cat::cat_disconnect))
        .route("/api/v1/cat/hamlib/restore", post(crate::cat::cat_hamlib_restore))
        .route("/api/v1/cat/profiles", post(crate::cat::cat_profiles))
        .route("/api/v1/cat/models", get(crate::cat::cat_models))
        .route("/api/v1/cat/ports", get(crate::cat::cat_ports))
        .route("/api/v1/cat/detect", post(crate::cat::cat_detect))
        .route("/api/v1/cat/extra", post(crate::cat::cat_extra))
        .route("/api/v1/sstv", get(crate::sstv_api::sstv_get).put(crate::sstv_api::sstv_put))
        .route("/api/v1/sstv/rx/start", post(crate::sstv_api::sstv_rx_start))
        .route("/api/v1/sstv/rx/stop", post(crate::sstv_api::sstv_rx_stop))
        .route("/api/v1/sstv/rx/force", post(crate::sstv_api::sstv_rx_force))
        .route("/api/v1/sstv/rx/cancel", post(crate::sstv_api::sstv_rx_cancel))
        .route("/api/v1/sstv/rx/current", get(crate::sstv_api::sstv_rx_current))
        .route("/api/v1/sstv/images", get(crate::sstv_api::sstv_images))
        .route("/api/v1/sstv/images/{id}", get(crate::sstv_api::sstv_image).delete(crate::sstv_api::sstv_image_delete))
        .route("/api/v1/sstv/tx", post(crate::sstv_api::sstv_tx).layer(axum::extract::DefaultBodyLimit::max(crate::sstv_api::SSTV_UPLOAD_MAX_BYTES)))
        .route("/api/v1/sstv/tx/abort", post(crate::sstv_api::sstv_tx_abort))
        .route("/api/v1/digi", get(crate::digi_api::digi_get).put(crate::digi_api::digi_put))
        .route("/api/v1/digi/rx/start", post(crate::digi_api::digi_rx_start))
        .route("/api/v1/digi/rx/stop", post(crate::digi_api::digi_rx_stop))
        .route("/api/v1/digi/tx", post(crate::digi_api::digi_tx))
        .route("/api/v1/digi/tx/abort", post(crate::digi_api::digi_tx_abort))
        .route("/api/v1/digi/import-macros", post(crate::digi_api::digi_import_macros))
        .route("/api/v1/sat", get(crate::sat_api::sat_get).put(crate::sat_api::sat_put))
        .route("/api/v1/sat/tle/refresh", post(crate::sat_api::sat_refresh))
        .route("/api/v1/sat/passes", get(crate::sat_api::sat_passes))
        .route("/api/v1/sat/track", get(crate::sat_api::sat_track))
        .route("/api/v1/sat/positions", get(crate::sat_api::sat_positions))
        .route("/api/v1/sat/maps", get(crate::sat_api::sat_maps).post(crate::sat_api::sat_map_upload).layer(axum::extract::DefaultBodyLimit::max(21 * 1024 * 1024)))
        .route("/api/v1/sat/maps/{name}", get(crate::sat_api::sat_map))
        .route("/api/v1/rotator", get(crate::rotator_api::rotator_get).put(crate::rotator_api::rotator_put))
        .route("/api/v1/rotator/goto", post(crate::rotator_api::rotator_goto))
        .route("/api/v1/rotator/stop", post(crate::rotator_api::rotator_stop))
        .route("/api/v1/rotator/models", get(crate::rotator_api::rotator_models))
        .route("/api/v1/qmap", get(crate::qmap_api::qmap_get).put(crate::qmap_api::qmap_put))
        .route("/api/v1/qmap/decode", post(crate::qmap_api::qmap_decode))
        .route("/api/v1/qmap/clear-average", post(crate::qmap_api::qmap_clear_average))
        .route("/api/v1/qmap/tune", post(crate::qmap_api::qmap_tune))
        .route("/api/v1/contest", get(crate::contest_api::contest_get))
        .route("/api/v1/contest/regole/{id}", get(crate::contest_api::contest_regole))
        .route("/api/v1/contest/attiva", post(crate::contest_api::contest_attiva))
        .route("/api/v1/contest/disattiva", post(crate::contest_api::contest_disattiva))
        .route("/api/v1/contest/impostazioni/{id}", put(crate::contest_api::contest_impostazioni))
        .route("/api/v1/contest/valuta", post(crate::contest_api::contest_valuta))
        .route("/api/v1/contest/lavorati", get(crate::contest_api::contest_lavorati))
        .route("/api/v1/contest/consiglio", get(crate::contest_api::contest_consiglio))
        .route("/api/v1/contest/mappa", get(crate::contest_api::contest_mappa))
        .route("/api/v1/contest/analisi", get(crate::contest_api::contest_analisi))
        .route("/api/v1/contest/log", get(crate::contest_api::contest_log))
        .route("/api/v1/contest/log/azzera", post(crate::contest_api::contest_log_azzera))
        .route("/api/v1/contest/log/{n}", patch(crate::contest_api::contest_log_modifica))
        .route("/api/v1/contest/esporta", get(crate::contest_api::contest_esporta))
        .route("/api/v1/contest/calendario", get(crate::contest_api::contest_calendario))
        .route("/api/v1/stream", get(ws::stream))
        .route("/api/v1/openapi.json", get(openapi))
        .layer(axum::middleware::from_fn_with_state(state.clone(), mark_lan))
        .with_state(state)
}

/// Ogni richiesta arrivata da un altro dispositivo dice che il telefono raggiunge Aurora (per la diagnosi dell'associazione).
async fn mark_lan(State(state): State<AppState>, req: axum::extract::Request, next: axum::middleware::Next) -> Response {
    if let Some(ConnectInfo(addr)) = req.extensions().get::<ConnectInfo<std::net::SocketAddr>>()
        && crate::pair_watch::from_lan(addr.ip())
    {
        state.pair_watch.seen_from_lan();
    }
    next.run(req).await
}

/// Il rapporto anonimo di un'associazione non riuscita, con la rete e il firewall di Windows letti adesso.
fn report_pairing(outcome: crate::pair_watch::Outcome) {
    tokio::spawn(async move {
        let check = tokio::task::spawn_blocking(lan::network_check).await.unwrap_or_default();
        tracing::warn!(target: "remote_api::associazione", "{}", outcome.message(check.category.as_deref(), check.firewall_allowed));
    });
}

pub struct ApiError {
    status: StatusCode,
    code: &'static str,
    retry_after: Option<u64>,
    /// Il perche' in chiaro (italiano), per i casi in cui il solo codice non basta (per esempio la risposta di un server esterno).
    message: Option<String>,
}

impl ApiError {
    pub(crate) fn new(status: StatusCode, code: &'static str) -> Self {
        ApiError { status, code, retry_after: None, message: None }
    }

    pub(crate) fn with_message(mut self, message: impl Into<String>) -> Self {
        self.message = Some(message.into());
        self
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let body = match &self.message {
            Some(message) => json!({ "error": self.code, "message": message }),
            None => json!({ "error": self.code }),
        };
        let mut resp = (self.status, Json(body)).into_response();
        if let Some(secs) = self.retry_after
            && let Ok(v) = HeaderValue::from_str(&secs.to_string())
        {
            resp.headers_mut().insert(header::RETRY_AFTER, v);
        }
        resp
    }
}

/// Un proxy (Vite, cloudflared, Tailscale serve) apre la connessione dal PC stesso, quindi per il core la
/// richiesta sembra locale: le sue intestazioni la tradiscono. Il codice di associazione si chiede solo
/// con una connessione diretta.
pub(crate) fn is_proxied(headers: &HeaderMap) -> bool {
    ["x-forwarded-for", "x-forwarded-host", "forwarded", "x-real-ip", "cf-connecting-ip", "cf-ray", "tailscale-user-login", "tailscale-user-name"]
        .iter()
        .any(|name| headers.contains_key(*name))
}

fn session_token(headers: &HeaderMap) -> Option<String> {
    let cookies = headers.get(header::COOKIE)?.to_str().ok()?;
    cookies
        .split(';')
        .filter_map(|c| c.trim().split_once('='))
        .find(|(name, _)| *name == COOKIE_NAME)
        .map(|(_, v)| v.to_string())
}

/// Indirizzo della rete di casa: loopback, privato o link-local. Un indirizzo pubblico non lo e' mai.
fn is_home_network(ip: std::net::IpAddr) -> bool {
    match ip {
        std::net::IpAddr::V4(v4) => v4.is_loopback() || v4.is_private() || v4.is_link_local(),
        std::net::IpAddr::V6(v6) => {
            v6.is_loopback() || (v6.segments()[0] & 0xfe00) == 0xfc00 || (v6.segments()[0] & 0xffc0) == 0xfe80
        }
    }
}

/// Dispositivo autenticato dal cookie di sessione.
pub struct Authed(pub DeviceId);

impl FromRequestParts<AppState> for Authed {
    type Rejection = ApiError;

    async fn from_request_parts(parts: &mut Parts, state: &AppState) -> Result<Self, ApiError> {
        // programma Windows: chi arriva da questo stesso PC (loopback, non da un proxy) entra senza codice
        if state.local_trust {
            let peer = parts.extensions.get::<ConnectInfo<SocketAddr>>().map(|c| c.0.ip());
            if peer.is_some_and(|ip| ip.is_loopback()) && !is_proxied(&parts.headers) {
                return Ok(Authed(DeviceId(0)));
            }
        }
        // accesso senza codice acceso: entra chi e' sulla rete di casa e non arriva da un proxy
        if state.open_access.load(std::sync::atomic::Ordering::SeqCst) {
            let peer = parts.extensions.get::<ConnectInfo<SocketAddr>>().map(|c| c.0.ip());
            if peer.is_some_and(is_home_network) && !is_proxied(&parts.headers) {
                return Ok(Authed(DeviceId(0)));
            }
        }
        session_token(&parts.headers)
            .and_then(|t| state.auth.verify(&t))
            .map(Authed)
            .ok_or_else(|| ApiError::new(StatusCode::UNAUTHORIZED, "unauthorized"))
    }
}

/// Indirizzo del client, se il server e' avviato con `ConnectInfo`.
pub(crate) struct Peer(pub(crate) Option<SocketAddr>);

impl<S: Send + Sync> FromRequestParts<S> for Peer {
    type Rejection = std::convert::Infallible;

    async fn from_request_parts(parts: &mut Parts, _: &S) -> Result<Self, Self::Rejection> {
        Ok(Peer(parts.extensions.get::<ConnectInfo<SocketAddr>>().map(|c| c.0)))
    }
}

#[utoipa::path(get, path = "/api/v1/health", responses((status = 200, description = "Stato, nome, versione e autore")))]
async fn health() -> Json<Value> {
    Json(json!({ "status": "ok", "name": APP_NAME, "version": APP_VERSION, "author": APP_AUTHOR, "os": std::env::consts::OS }))
}

#[derive(Deserialize, Default)]
struct PairStartBody {
    base_url: Option<String>,
}

/// Rete del PC per l'associazione del telefono: Wi-Fi, rete «Pubblica», firewall, indirizzi scelti (solo dal PC stesso).
#[utoipa::path(get, path = "/api/v1/lan/check", responses((status = 200, description = "{ssid, category, firewall_allowed, addresses}"), (status = 403, description = "Solo dal PC (local_only)")))]
async fn lan_check(Peer(peer): Peer, headers: HeaderMap) -> Result<Json<Value>, ApiError> {
    if !from_this_pc(peer, &headers) {
        return Err(ApiError::new(StatusCode::FORBIDDEN, "local_only"));
    }
    let check = tokio::task::spawn_blocking(lan::network_check).await.unwrap_or_default();
    Ok(Json(json!(check)))
}

/// «Consenti Aurora»: aggiunge la regola del firewall di Windows (chiede la conferma da amministratore; solo dal PC stesso).
#[utoipa::path(post, path = "/api/v1/lan/allow", responses((status = 200, description = "Regola aggiunta"), (status = 403, description = "Solo dal PC (local_only)"), (status = 500, description = "Non aggiunta (firewall_failed)")))]
async fn lan_allow(State(state): State<AppState>, Peer(peer): Peer, headers: HeaderMap) -> Result<Json<Value>, ApiError> {
    if !from_this_pc(peer, &headers) {
        return Err(ApiError::new(StatusCode::FORBIDDEN, "local_only"));
    }
    let port = match state.lan.port() {
        0 => 8787,
        p => p,
    };
    tokio::task::spawn_blocking(move || lan::allow_in_firewall(port))
        .await
        .map_err(|_| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "firewall_failed"))?
        .map_err(|e| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "firewall_failed").with_message(e))?;
    Ok(Json(json!({ "ok": true })))
}

#[utoipa::path(post, path = "/api/v1/pair/start", responses((status = 200, description = "Codice di associazione e QR (solo da loopback)"), (status = 403, description = "Richiesta non locale")))]
async fn pair_start(
    State(state): State<AppState>,
    Peer(peer): Peer,
    headers: HeaderMap,
    body: Option<Json<PairStartBody>>,
) -> Result<Json<Value>, ApiError> {
    if !peer.is_some_and(|p| p.ip().is_loopback()) || is_proxied(&headers) {
        return Err(ApiError::new(StatusCode::FORBIDDEN, "local_only"));
    }
    let pairing = state
        .auth
        .start_pairing()
        .map_err(|_| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "pairing_unavailable"))?;
    let base = body.and_then(|Json(b)| b.base_url);
    let addresses = phone_addresses(&state, base.as_deref());
    let mdns = state.lan.host();
    // il link porta all'IP attuale (sempre raggiungibile) e si porta dietro il nome mDNS come alternativa da ricordare
    let link = addresses.first().map(|first| {
        let mut l = format!("{first}/#/associa?codice={}", pairing.code);
        if let Some(name) = &mdns {
            l.push_str(&format!("&nome={name}"));
        }
        l
    });
    // si segue il codice: se allo scadere il telefono non si e' associato parte il rapporto anonimo con la causa probabile
    let named = lan::lan_ipv4s_named();
    let adapter = addresses
        .first()
        .and_then(|first| named.iter().find(|(ip, _)| first.contains(&format!("//{ip}:"))).map(|(_, n)| n.clone()))
        .unwrap_or_default();
    let (serial, previous) = state.pair_watch.start(link.as_deref(), &adapter, named.len());
    if let Some(previous) = previous {
        report_pairing(previous);
    }
    {
        let watch = std::sync::Arc::clone(&state.pair_watch);
        let wait = std::time::Duration::from_secs(pairing.expires_in_secs) + crate::pair_watch::GRACE;
        tokio::spawn(async move {
            tokio::time::sleep(wait).await;
            if let Some(outcome) = watch.expire(serial) {
                report_pairing(outcome);
            }
        });
    }
    let qr_text = link.clone().unwrap_or_else(|| pairing.code.clone());
    let qr_svg = qrcode::QrCode::new(qr_text.as_bytes())
        .map_err(|_| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "qr_failed"))?
        .render::<qrcode::render::svg::Color>()
        .min_dimensions(200, 200)
        .build();
    // tutti gli indirizzi, ognuno con il suo QR: se il primo non si apre sul telefono (PC su due reti, Wi-Fi diversa) si prova il successivo
    let options: Vec<Value> = addresses
        .iter()
        .map(|url| {
            let mut l = format!("{url}/#/associa?codice={}", pairing.code);
            if let Some(name) = &mdns {
                l.push_str(&format!("&nome={name}"));
            }
            let kind = if url.contains(".local") {
                "mdns"
            } else {
                named.iter().find(|(ip, _)| url.contains(&format!("//{ip}:"))).map_or("other", |(_, n)| if lan::is_wifi_adapter(n) { "wifi" } else if lan::is_virtual_adapter(n) { "virtual" } else { "cable" })
            };
            let svg = qrcode::QrCode::new(l.as_bytes()).ok().map(|q| q.render::<qrcode::render::svg::Color>().min_dimensions(120, 120).build());
            json!({ "url": url, "link": l, "kind": kind, "qr_svg": svg })
        })
        .collect();
    Ok(Json(json!({ "code": pairing.code, "expires_in": pairing.expires_in_secs, "qr_svg": qr_svg, "link": link, "addresses": addresses, "options": options, "mdns": mdns_url(&state) })))
}

/// `http://aurora.local:8787` se l'annuncio mDNS e' attivo.
fn mdns_url(state: &AppState) -> Option<String> {
    let port = state.lan.port();
    state.lan.host().filter(|_| port != 0).map(|h| format!("http://{h}:{port}"))
}

/// Gli indirizzi con cui il telefono puo' raggiungere Aurora: gli IP della rete di casa e, se c'e', il nome mDNS.
/// Se la pagina e' aperta gia' da un indirizzo di rete (non dal PC stesso) quello viene per primo.
fn phone_addresses(state: &AppState, base: Option<&str>) -> Vec<String> {
    let base = base.map(|b| b.trim_end_matches('/').to_string());
    let base_port = base
        .as_deref()
        .and_then(|b| b.rsplit(':').next())
        .and_then(|p| p.parse::<u16>().ok());
    let port = match state.lan.port() {
        0 => base_port.unwrap_or(8787),
        p => p,
    };
    let mut out = Vec::new();
    if let Some(b) = &base {
        let local = ["//localhost", "//127.", "//[::1]", "tauri:"].iter().any(|m| b.contains(m));
        if !local && b.starts_with("http") {
            out.push(b.clone());
        }
    }
    for ip in lan::lan_ipv4s() {
        let url = format!("http://{ip}:{port}");
        if !out.contains(&url) {
            out.push(url);
        }
    }
    if let Some(m) = mdns_url(state) {
        if !out.contains(&m) {
            out.push(m);
        }
    }
    out
}

#[derive(Deserialize)]
struct PairFinishBody {
    code: String,
    device_name: String,
}

#[utoipa::path(post, path = "/api/v1/pair/finish", responses((status = 204, description = "Dispositivo associato, cookie di sessione impostato"), (status = 400, description = "Codice o nome non validi"), (status = 410, description = "Codice scaduto"), (status = 429, description = "Troppi tentativi")))]
async fn pair_finish(State(state): State<AppState>, Json(body): Json<PairFinishBody>) -> Response {
    let result = state.auth.finish_pairing(&body.code, &body.device_name);
    state.pair_watch.finished(match &result {
        Ok(_) => "ok",
        Err(AuthError::BadCode) => "codice sbagliato",
        Err(AuthError::BadName) => "nome non valido",
        Err(AuthError::Expired) => "codice scaduto",
        Err(AuthError::Locked { .. }) => "troppi tentativi",
        Err(_) => "errore interno",
    });
    match result {
        Ok(token) => {
            let mut cookie = format!(
                "{COOKIE_NAME}={}; HttpOnly; SameSite=Strict; Path=/; Max-Age={COOKIE_MAX_AGE_SECS}",
                token.0
            );
            if state.secure_cookie {
                cookie.push_str("; Secure");
            }
            match HeaderValue::from_str(&cookie) {
                Ok(v) => (StatusCode::NO_CONTENT, [(header::SET_COOKIE, v)]).into_response(),
                Err(_) => ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "cookie_failed").into_response(),
            }
        }
        Err(AuthError::BadCode) => ApiError::new(StatusCode::BAD_REQUEST, "bad_code").into_response(),
        Err(AuthError::BadName) => ApiError::new(StatusCode::BAD_REQUEST, "bad_name").into_response(),
        Err(AuthError::Expired) => ApiError::new(StatusCode::GONE, "expired").into_response(),
        Err(AuthError::Locked { retry_after_secs }) => {
            let mut e = ApiError::new(StatusCode::TOO_MANY_REQUESTS, "locked");
            e.retry_after = Some(retry_after_secs);
            e.into_response()
        }
        Err(AuthError::Storage(e)) => {
            tracing::error!("associazione fallita: {e}");
            ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "storage").into_response()
        }
    }
}

#[utoipa::path(get, path = "/api/v1/devices", responses((status = 200, description = "Dispositivi associati"), (status = 401, description = "Non autenticato")))]
async fn devices(State(state): State<AppState>, _: Authed) -> Result<Json<Value>, ApiError> {
    let list = state
        .auth
        .list()
        .map_err(|_| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "storage"))?;
    Ok(Json(Value::Array(
        list.into_iter()
            .map(|d| json!({ "id": d.id.0, "name": d.name, "created_at": d.created_at }))
            .collect(),
    )))
}

#[utoipa::path(delete, path = "/api/v1/devices/{id}", responses((status = 204, description = "Dispositivo revocato"), (status = 404, description = "Dispositivo inesistente")))]
async fn revoke_device(State(state): State<AppState>, _: Authed, Path(id): Path<i64>) -> Result<StatusCode, ApiError> {
    match state.auth.revoke(DeviceId(id)) {
        Ok(true) => Ok(StatusCode::NO_CONTENT),
        Ok(false) => Err(ApiError::new(StatusCode::NOT_FOUND, "unknown_device")),
        Err(_) => Err(ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "storage")),
    }
}

fn dispatch(state: &AppState, command: Command) -> Result<Json<Value>, ApiError> {
    state
        .bus
        .commands()
        .try_send(command)
        .map(|()| Json(json!({ "ok": true })))
        .map_err(|_| ApiError::new(StatusCode::SERVICE_UNAVAILABLE, "engine_unavailable"))
}

/// Un cambio di banda comandato ferma subito la trasmissione (come lo STOP, senza spegnere l'automazione), senza aspettare che la radio
/// confermi la nuova frequenza: il motore azzera poi anche la stazione DX appena vede la banda nuova. Nessun effetto se la banda e' la stessa
/// (per esempio uno spostamento di pochi kHz dentro la banda).
pub(crate) fn halt_for_band_change(state: &AppState, target_hz: u64) {
    let current = state.current_status().filter(|s| s.dial_hz > 0).map(|s| s.dial_hz);
    let target_band = rig_control::bands::amateur_band(target_hz);
    let same = current.is_some_and(|now| target_band.is_some() && rig_control::bands::amateur_band(now) == target_band);
    if !same && state.bus.commands().try_send(Command::HaltTx).is_err() {
        tracing::warn!("cambio di banda: l'arresto della trasmissione non e' stato consegnato al motore");
    }
}

/// Fermare la trasmissione e' sempre permesso, da qualunque via.
#[utoipa::path(post, path = "/api/v1/commands/halt-tx", responses((status = 200, description = "Comando inviato (idempotente)")))]
async fn halt_tx(State(state): State<AppState>, _: Authed) -> Result<Json<Value>, ApiError> {
    // lo STOP spegne anche l'automazione, qualunque sia il motore
    if let Some(automation) = &state.automation {
        automation.halt();
    }
    dispatch(&state, Command::HaltTx)
}

#[derive(Deserialize)]
struct ReplyBody {
    decode_id: u64,
}

/// La pagina Contest manda `X-Aurora-Origine: contest` con le chiamate e il CQ: solo quei QSO entrano nel log del contest.
fn segna_origine(state: &AppState, headers: &HeaderMap) {
    if let Some(c) = &state.contest {
        c.imposta_origine(headers.get("x-aurora-origine").is_some_and(|v| v.as_bytes().eq_ignore_ascii_case(b"contest")));
    }
}

#[utoipa::path(post, path = "/api/v1/commands/reply", responses((status = 200, description = "Risposta inviata al motore"), (status = 403, description = "Trasmissione non abilitata (tx_disabled) o non consentita da questa via"), (status = 404, description = "Decode sconosciuto")))]
async fn reply(
    State(state): State<AppState>,
    _: Authed,
    headers: HeaderMap,
    Json(body): Json<ReplyBody>,
) -> Result<Json<Value>, ApiError> {
    segna_origine(&state, &headers);
    if !state.policy.live_tx() {
        return Err(ApiError::new(StatusCode::FORBIDDEN, "tx_disabled"));
    }
    if !state.policy.allows(transport_of(&headers)) {
        return Err(ApiError::new(StatusCode::FORBIDDEN, "tx_not_allowed_on_transport"));
    }
    if !state.knows_decode(body.decode_id) {
        return Err(ApiError::new(StatusCode::NOT_FOUND, "unknown_decode"));
    }
    dispatch(&state, Command::Reply { decode_id: body.decode_id })
}

#[utoipa::path(post, path = "/api/v1/commands/queue-reply", responses((status = 200, description = "Stazione messa in coda (o tolta, o subito chiamata se non c'e' un QSO in corso)"), (status = 403, description = "Trasmissione non abilitata (tx_disabled) o non consentita da questa via"), (status = 404, description = "Decode sconosciuto")))]
async fn queue_reply(
    State(state): State<AppState>,
    _: Authed,
    headers: HeaderMap,
    Json(body): Json<ReplyBody>,
) -> Result<Json<Value>, ApiError> {
    segna_origine(&state, &headers);
    check_tx_allowed(&state, &headers)?;
    if !state.knows_decode(body.decode_id) {
        return Err(ApiError::new(StatusCode::NOT_FOUND, "unknown_decode"));
    }
    dispatch(&state, Command::QueueReply { decode_id: body.decode_id })
}

/// Chi fa trasmettere deve avere la trasmissione aperta e arrivare da una via consentita.
pub(crate) fn check_tx_allowed(state: &AppState, headers: &HeaderMap) -> Result<(), ApiError> {
    if !state.policy.live_tx() {
        return Err(ApiError::new(StatusCode::FORBIDDEN, "tx_disabled"));
    }
    if !state.policy.allows(transport_of(headers)) {
        return Err(ApiError::new(StatusCode::FORBIDDEN, "tx_not_allowed_on_transport"));
    }
    Ok(())
}

#[derive(Deserialize)]
struct FreeTextBody {
    text: String,
    #[serde(default)]
    send: bool,
}

/// Testo libero di FT8: al massimo 13 caratteri tra lettere, cifre e ` +-./?`.
fn clean_free_text(text: &str) -> Option<String> {
    let t = text.trim().to_uppercase();
    let ok = (1..=13).contains(&t.chars().count()) && t.chars().all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || " +-./?".contains(c));
    ok.then_some(t)
}

#[utoipa::path(post, path = "/api/v1/commands/free-text", responses((status = 200, description = "Messaggio libero inviato al motore"), (status = 400, description = "Testo non valido (bad_text)"), (status = 403, description = "Con send=true: trasmissione non abilitata o non consentita da questa via")))]
async fn free_text(State(state): State<AppState>, _: Authed, headers: HeaderMap, Json(body): Json<FreeTextBody>) -> Result<Json<Value>, ApiError> {
    let text = clean_free_text(&body.text).ok_or_else(|| ApiError::new(StatusCode::BAD_REQUEST, "bad_text"))?;
    if body.send {
        check_tx_allowed(&state, &headers)?;
    }
    dispatch(&state, Command::SendFreeText { text, send: body.send })
}

#[derive(Deserialize)]
struct DxBody {
    #[serde(default)]
    call: String,
    #[serde(default)]
    grid: String,
}

/// Nominativo scritto a mano: lettere, cifre e `/`, da 3 a 12 caratteri (vuoto = nessun DX).
fn clean_dx_call(call: &str) -> Option<String> {
    let c = call.trim().to_uppercase();
    (c.is_empty() || ((3..=12).contains(&c.len()) && c.chars().all(|ch| ch.is_ascii_alphanumeric() || ch == '/'))).then_some(c)
}

/// Locatore Maidenhead di 4 o 6 caratteri (vuoto = nessuno).
fn clean_dx_grid(grid: &str) -> Option<String> {
    let g = grid.trim().to_uppercase();
    let b = g.as_bytes();
    let ok = g.is_empty()
        || ((g.len() == 4 || g.len() == 6)
            && (b'A'..=b'R').contains(&b[0])
            && (b'A'..=b'R').contains(&b[1])
            && b[2].is_ascii_digit()
            && b[3].is_ascii_digit()
            && (g.len() == 4 || ((b'A'..=b'X').contains(&b[4]) && (b'A'..=b'X').contains(&b[5]))));
    ok.then_some(g)
}

/// I campi «DX Call» e «Locatore» del pannello Comandi: imposta la stazione e rigenera i messaggi, senza trasmettere.
#[utoipa::path(post, path = "/api/v1/commands/dx", responses((status = 200, description = "DX impostato"), (status = 400, description = "Nominativo o locatore non validi (bad_dx)")))]
async fn set_dx(State(state): State<AppState>, _: Authed, Json(body): Json<DxBody>) -> Result<Json<Value>, ApiError> {
    let (Some(call), Some(grid)) = (clean_dx_call(&body.call), clean_dx_grid(&body.grid)) else {
        return Err(ApiError::new(StatusCode::BAD_REQUEST, "bad_dx"));
    };
    dispatch(&state, Command::SetDx { call, grid })
}

#[derive(Deserialize)]
struct ReportBody {
    db: i32,
}

/// Il rapporto da inviare scelto a mano (da -50 a +49 dB).
#[utoipa::path(post, path = "/api/v1/commands/report", responses((status = 200, description = "Rapporto impostato"), (status = 400, description = "Fuori da -50..+49 (bad_report)")))]
async fn set_report(State(state): State<AppState>, _: Authed, Json(body): Json<ReportBody>) -> Result<Json<Value>, ApiError> {
    if !(-50..=49).contains(&body.db) {
        return Err(ApiError::new(StatusCode::BAD_REQUEST, "bad_report"));
    }
    dispatch(&state, Command::SetReport { db: body.db })
}

#[derive(Deserialize)]
struct TuneBody {
    on: bool,
}

/// «Tune»: portante sulla frequenza Tx per un periodo; spegnerlo e' sempre ammesso, accenderlo vuole la trasmissione aperta.
#[utoipa::path(post, path = "/api/v1/commands/tune", responses((status = 200, description = "Tune chiesto"), (status = 403, description = "Trasmissione non abilitata o non consentita da questa via")))]
async fn tune(State(state): State<AppState>, _: Authed, headers: HeaderMap, Json(body): Json<TuneBody>) -> Result<Json<Value>, ApiError> {
    if body.on {
        check_tx_allowed(&state, &headers)?;
    }
    dispatch(&state, Command::Tune { on: body.on })
}

/// «Log QSO» a mano: registra il QSO con la stazione in corso.
#[utoipa::path(post, path = "/api/v1/commands/log-qso", responses((status = 200, description = "Registrazione chiesta")))]
async fn log_qso(State(state): State<AppState>, _: Authed) -> Result<Json<Value>, ApiError> {
    dispatch(&state, Command::LogQso)
}

#[derive(Deserialize)]
struct FoxBody {
    action: String,
    #[serde(default)]
    call: String,
    #[serde(default)]
    top: bool,
    #[serde(default)]
    slots: Option<usize>,
    #[serde(default)]
    more_cqs: bool,
    #[serde(default)]
    cq: String,
    #[serde(default)]
    free_text: Option<String>,
    #[serde(default)]
    auto_queue: bool,
    #[serde(default)]
    ultra: bool,
    /// Seme TOTP della verifica (azione `otp`), vuoto = spenta.
    #[serde(default)]
    seed: Option<String>,
    /// Ogni quanti periodi il Fox classico manda il codice (azione `otp`).
    #[serde(default)]
    interval: Option<u32>,
    /// Azione `otp`: il seme salvato nel motore resta com'e' (la pagina non lo conosce), cambia solo l'intervallo.
    #[serde(default)]
    keep_seed: bool,
}

/// I comandi della pagina Fox: `select` (in coda, `top` in testa), `unqueue`, `drop` (chiude un QSO in corso), `options`, `reset`, `otp` (seme e intervallo della verifica).
#[utoipa::path(post, path = "/api/v1/fox", responses((status = 200, description = "Comando consegnato al motore"), (status = 400, description = "Azione o nominativo non validi (bad_fox)")))]
async fn fox_command(State(state): State<AppState>, _: Authed, Json(body): Json<FoxBody>) -> Result<Json<Value>, ApiError> {
    use aurora_core::FoxCommand as F;
    let bad = || ApiError::new(StatusCode::BAD_REQUEST, "bad_fox");
    let call = || clean_dx_call(&body.call).filter(|c| !c.is_empty()).ok_or_else(bad);
    let cmd = match body.action.as_str() {
        "select" => F::Select { call: call()?, top: body.top },
        "unqueue" => F::Unqueue { call: call()? },
        "drop" => F::Drop { call: call()? },
        "options" => {
            let cq = body.cq.trim().to_uppercase();
            if !cq.starts_with("CQ") || cq.len() > 8 {
                return Err(bad());
            }
            let free_text = match body.free_text.as_deref().map(str::trim).filter(|t| !t.is_empty()) {
                Some(t) => Some(clean_free_text(t).ok_or_else(bad)?),
                None => None,
            };
            F::Options { slots: body.slots.unwrap_or(5).clamp(1, 5), more_cqs: body.more_cqs, cq, free_text, auto_queue: body.auto_queue, ultra: body.ultra }
        }
        "reset" => F::Reset,
        "otp" => {
            let seed = body.seed.as_deref().map(|s| s.trim().to_uppercase()).filter(|s| !s.is_empty());
            if seed.as_deref().is_some_and(|s| s.len() != 16 || !s.bytes().all(|b| b.is_ascii_uppercase() || (b'2'..=b'7').contains(&b))) {
                return Err(bad());
            }
            F::Otp { seed, interval: body.interval.unwrap_or(1).clamp(1, 100), keep: body.keep_seed }
        }
        _ => return Err(bad()),
    };
    dispatch(&state, Command::Fox(cmd))
}

/// Un QSO del Fox log come lo ha la pagina Fox (`FoxLogView`).
#[derive(Deserialize)]
struct FoxLogQso {
    call: String,
    #[serde(default)]
    grid: String,
    band: String,
    mode: String,
    freq_hz: u64,
    #[serde(default)]
    sent: String,
    #[serde(default)]
    rcvd: String,
    time_ms: u64,
}

#[derive(Deserialize)]
struct FoxClublogBody {
    qsos: Vec<FoxLogQso>,
}

/// Quanti QSO del Fox log si mandano al massimo in una volta (quanti ne tiene la pagina).
const FOX_CLUBLOG_MAX: usize = 500;

/// «Club Log» del Fox log: i QSO della sessione Fox entrano nella coda dell'invio diretto a Club Log (gli stessi record dell'invio in tempo reale;
/// Club Log tiene buoni i doppioni). Serve Club Log acceso nelle impostazioni.
#[utoipa::path(post, path = "/api/v1/fox/clublog", responses((status = 200, description = "`{queued}`: QSO messi in coda"), (status = 400, description = "Nessun QSO valido o troppi (bad_fox_log)"), (status = 404, description = "Solo con il motore proprio (upload_unavailable)"), (status = 409, description = "Club Log spento nelle impostazioni (clublog_off, con `message`)")))]
async fn fox_clublog(State(state): State<AppState>, _: Authed, Json(body): Json<FoxClublogBody>) -> Result<Json<Value>, ApiError> {
    let bad = || ApiError::new(StatusCode::BAD_REQUEST, "bad_fox_log");
    if body.qsos.is_empty() || body.qsos.len() > FOX_CLUBLOG_MAX {
        return Err(bad());
    }
    let mut qsos = Vec::with_capacity(body.qsos.len());
    for q in body.qsos {
        let call = clean_dx_call(&q.call).filter(|c| !c.is_empty()).ok_or_else(bad)?;
        if q.time_ms == 0 || q.freq_hz == 0 || q.band.len() > 8 || q.mode.len() > 8 || q.grid.len() > 8 || q.sent.len() > 8 || q.rcvd.len() > 8 {
            return Err(bad());
        }
        // la Fox chiude il QSO 14 s dopo l'inizio del periodo dell'RR73 (`FoxLogView::time_ms`): inizio e fine come nel registro
        qsos.push(aurora_core::QsoRecord {
            call,
            grid: q.grid.trim().to_string(),
            band: q.band.trim().to_string(),
            mode: q.mode.trim().to_uppercase(),
            freq_hz: q.freq_hz,
            rst_sent: q.sent.trim().to_string(),
            rst_rcvd: q.rcvd.trim().to_string(),
            time_ms: (q.time_ms % 86_400_000) as u32,
            time_on_unix_ms: q.time_ms.saturating_sub(14_000),
            time_off_unix_ms: q.time_ms,
            ..Default::default()
        });
    }
    let service = state.engine_settings.as_ref().and_then(|s| s.reporting()).ok_or_else(|| ApiError::new(StatusCode::NOT_FOUND, "upload_unavailable"))?;
    match service.queue_qsos(reporting::upload::Target::Clublog, &qsos) {
        Ok(queued) => Ok(Json(json!({ "queued": queued }))),
        Err(message) => Err(ApiError::new(StatusCode::CONFLICT, "clublog_off").with_message(message)),
    }
}

#[derive(Deserialize, Default)]
struct CqBody {
    #[serde(default)]
    modifier: String,
}

#[utoipa::path(post, path = "/api/v1/commands/cq", responses((status = 200, description = "CQ chiesto a JTDX"), (status = 400, description = "Modificatore non valido"), (status = 403, description = "Trasmissione non abilitata o non consentita da questa via")))]
async fn call_cq(State(state): State<AppState>, _: Authed, headers: HeaderMap, body: Option<Json<CqBody>>) -> Result<Json<Value>, ApiError> {
    segna_origine(&state, &headers);
    let modifier = body.map(|Json(b)| b.modifier).unwrap_or_default().trim().to_uppercase();
    if modifier.len() > 4 || !modifier.chars().all(|c| c.is_ascii_uppercase()) {
        return Err(ApiError::new(StatusCode::BAD_REQUEST, "bad_modifier"));
    }
    check_tx_allowed(&state, &headers)?;
    dispatch(&state, Command::CallCq { modifier })
}

#[derive(Deserialize)]
struct BandBody {
    band: String,
    /// Modo di cui si vuole la frequenza (predefinito: quello in uso).
    mode: Option<String>,
}

/// Porta la radio sulla frequenza FT8 della banda (OmniRig, condiviso con JTDX). Mai durante la trasmissione.
#[utoipa::path(post, path = "/api/v1/commands/band", responses((status = 200, description = "Radio spostata"), (status = 400, description = "Banda sconosciuta (bad_band)"), (status = 409, description = "In trasmissione (transmitting)"), (status = 502, description = "Errore della radio (rig_error)"), (status = 503, description = "Comando radio non disponibile (rig_unavailable)")))]
async fn change_band(State(state): State<AppState>, _: Authed, Json(body): Json<BandBody>) -> Result<Json<Value>, ApiError> {
    let rig = if state.manual_dial.is_some() { state.dial_rig() } else { state.active_rig() };
    if rig.is_none() && state.manual_dial.is_none() {
        return Err(ApiError::new(StatusCode::SERVICE_UNAVAILABLE, "rig_unavailable"));
    }
    let mode = body.mode.clone().unwrap_or_else(|| state.current_mode());
    // il blocco chiude il lock prima di qualunque attesa
    let region = state.frequency_region();
    let found = {
        let table = state.frequencies.lock().unwrap_or_else(|p| p.into_inner());
        // JT4 ha frequenze solo sulle microonde (elenco di WSJT-X): sulle altre bande si usa quella di JT65
        table.frequency_in(region, &body.band, &mode).or_else(|| mode.to_uppercase().starts_with("JT4").then(|| table.frequency_in(region, &body.band, "JT65")).flatten())
    };
    let hz = found.ok_or_else(|| ApiError::new(StatusCode::BAD_REQUEST, "bad_band"))?;
    if state.current_status().is_some_and(|s| s.transmitting) {
        return Err(ApiError::new(StatusCode::CONFLICT, "transmitting"));
    }
    halt_for_band_change(&state, hz);
    // senza CAT si cambia solo la frequenza di Aurora: la radio la sposta l'operatore
    let Some(rig) = rig else {
        if let Some(set) = &state.manual_dial {
            set(hz);
        }
        return Ok(Json(json!({ "ok": true, "dial_hz": hz, "manual": true })));
    };
    // il comando blocca (PowerShell e COM): fuori dal thread asincrono
    let done = tokio::task::spawn_blocking(move || rig.set_frequency(hz)).await;
    match done {
        Ok(Ok(())) => Ok(Json(json!({ "ok": true, "dial_hz": hz }))),
        Ok(Err(e)) => {
            // con una radio che risponde vale solo la sua frequenza: scrivere quella «a mano» la contraddiceva (la frequenza del motore
            // e' la stessa) e la radio rimbalzava fra due bande. Una radio spenta o scollegata passa gia' da `dial_rig` (frequenza a mano).
            tracing::warn!("cambio banda fallito: {e}");
            Err(ApiError::new(StatusCode::BAD_GATEWAY, "rig_error"))
        }
        Err(_) => Err(ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "rig_task")),
    }
}

#[derive(Deserialize)]
struct FrequencyBody {
    /// Frequenza di dial in Hz.
    hz: u64,
}

/// Porta la radio su una frequenza esatta (per esempio quella di uno spot), solo dentro le bande dei radioamatori.
/// Mai durante la trasmissione.
#[utoipa::path(post, path = "/api/v1/commands/frequency", responses((status = 200, description = "Radio spostata"), (status = 400, description = "Fuori dalle bande dei radioamatori (bad_frequency)"), (status = 409, description = "In trasmissione (transmitting)"), (status = 502, description = "Errore della radio (rig_error)"), (status = 503, description = "Comando radio non disponibile (rig_unavailable)")))]
async fn change_frequency(State(state): State<AppState>, _: Authed, Json(body): Json<FrequencyBody>) -> Result<Json<Value>, ApiError> {
    let rig = if state.manual_dial.is_some() { state.dial_rig() } else { state.active_rig() };
    if rig.is_none() && state.manual_dial.is_none() {
        return Err(ApiError::new(StatusCode::SERVICE_UNAVAILABLE, "rig_unavailable"));
    }
    if rig_control::bands::amateur_band(body.hz).is_none() {
        return Err(ApiError::new(StatusCode::BAD_REQUEST, "bad_frequency"));
    }
    if state.current_status().is_some_and(|s| s.transmitting) {
        return Err(ApiError::new(StatusCode::CONFLICT, "transmitting"));
    }
    let hz = body.hz;
    halt_for_band_change(&state, hz);
    let Some(rig) = rig else {
        if let Some(set) = &state.manual_dial {
            set(hz);
        }
        return Ok(Json(json!({ "ok": true, "dial_hz": hz, "manual": true })));
    };
    match tokio::task::spawn_blocking(move || rig.set_frequency(hz)).await {
        Ok(Ok(())) => Ok(Json(json!({ "ok": true, "dial_hz": hz }))),
        Ok(Err(e)) => {
            // con una radio che risponde vale solo la sua frequenza: scrivere quella «a mano» la contraddiceva (la frequenza del motore
            // e' la stessa) e la radio rimbalzava fra due bande. Una radio spenta o scollegata passa gia' da `dial_rig` (frequenza a mano).
            tracing::warn!("cambio frequenza fallito: {e}");
            Err(ApiError::new(StatusCode::BAD_GATEWAY, "rig_error"))
        }
        Err(_) => Err(ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "rig_task")),
    }
}

const MAX_ATTENUATION_DB: f64 = 45.0;

/// Livello audio di uscita di JTDX (cursore Pwr) come attenuazione in dB: 0 = livello massimo.
#[utoipa::path(get, path = "/api/v1/output-level", responses((status = 200, description = "Attenuazione in dB e limiti"), (status = 502, description = "JTDX non ha risposto (rig_error)"), (status = 503, description = "Comando non disponibile (rig_unavailable)")))]
async fn output_level_get(State(state): State<AppState>, _: Authed) -> Result<Json<Value>, ApiError> {
    let rig = state.active_rig().ok_or_else(|| ApiError::new(StatusCode::SERVICE_UNAVAILABLE, "rig_unavailable"))?;
    match tokio::task::spawn_blocking(move || rig.output_attenuation()).await {
        Ok(Ok(tenths)) => Ok(Json(json!({ "attenuation_db": f64::from(tenths) / 10.0, "min_db": 0.0, "max_db": MAX_ATTENUATION_DB }))),
        Ok(Err(e)) => {
            tracing::warn!("lettura del livello di uscita fallita: {e}");
            Err(ApiError::new(StatusCode::BAD_GATEWAY, "rig_error"))
        }
        Err(_) => Err(ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "rig_task")),
    }
}

#[derive(Deserialize)]
struct OutputLevelBody {
    attenuation_db: f64,
}

/// Cambia il livello di uscita, come il cursore Pwr di JTDX: si puo' anche a trasmissione chiusa (non fa trasmettere); da una via non consentita no.
#[utoipa::path(put, path = "/api/v1/output-level", responses((status = 200, description = "Livello impostato"), (status = 400, description = "Fuori dal cursore, 0-45 dB (bad_level)"), (status = 403, description = "Via non consentita"), (status = 502, description = "JTDX non ha risposto (rig_error)"), (status = 503, description = "Comando non disponibile (rig_unavailable)")))]
async fn output_level_put(State(state): State<AppState>, _: Authed, headers: HeaderMap, Json(body): Json<OutputLevelBody>) -> Result<Json<Value>, ApiError> {
    if !state.policy.allows(transport_of(&headers)) {
        return Err(ApiError::new(StatusCode::FORBIDDEN, "tx_not_allowed_on_transport"));
    }
    let rig = state.active_rig().ok_or_else(|| ApiError::new(StatusCode::SERVICE_UNAVAILABLE, "rig_unavailable"))?;
    if !body.attenuation_db.is_finite() || !(0.0..=MAX_ATTENUATION_DB).contains(&body.attenuation_db) {
        return Err(ApiError::new(StatusCode::BAD_REQUEST, "bad_level"));
    }
    let tenths = (body.attenuation_db * 10.0).round() as u32;
    match tokio::task::spawn_blocking(move || rig.set_output_attenuation(tenths)).await {
        Ok(Ok(())) => Ok(Json(json!({ "ok": true, "attenuation_db": f64::from(tenths) / 10.0 }))),
        Ok(Err(e)) => {
            tracing::warn!("cambio del livello di uscita fallito: {e}");
            Err(ApiError::new(StatusCode::BAD_GATEWAY, "rig_error"))
        }
        Err(_) => Err(ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "rig_task")),
    }
}

fn online_service(state: &AppState) -> Result<std::sync::Arc<online_info::OnlineService>, ApiError> {
    state.online.clone().ok_or_else(|| ApiError::new(StatusCode::NOT_FOUND, "online_unavailable"))
}

fn unix_ms() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_millis() as u64)
}

/// Quali servizi online sono accesi (PSK Reporter, propagazione). Spenti finche' non si accendono.
#[utoipa::path(get, path = "/api/v1/online", responses((status = 200, description = "`psk` e `solar`"), (status = 404, description = "Non disponibile (online_unavailable)")))]
async fn online_get(State(state): State<AppState>, _: Authed) -> Result<Json<Value>, ApiError> {
    Ok(Json(json!(online_service(&state)?.settings())))
}

/// Accende o spegne i servizi online. Spegnere dimentica cio' che si era scaricato.
#[utoipa::path(put, path = "/api/v1/online", responses((status = 200, description = "Impostazioni salvate"), (status = 404, description = "Non disponibile (online_unavailable)"), (status = 500, description = "Scrittura su disco fallita (online_write)")))]
async fn online_put(State(state): State<AppState>, _: Authed, Json(settings): Json<online_info::Settings>) -> Result<Json<Value>, ApiError> {
    let saved = online_service(&state)?.set_settings(settings).map_err(|e| {
        tracing::warn!("salvataggio dei servizi online fallito: {e}");
        ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "online_write")
    })?;
    Ok(Json(json!(saved)))
}

/// Chi ha ricevuto il nostro nominativo nelle ultime ore, da PSK Reporter (se acceso).
#[utoipa::path(get, path = "/api/v1/online/psk", responses((status = 200, description = "`enabled` e `psk` (`fetched_ms`, `callsign`, `reports`, `error`)"), (status = 404, description = "Non disponibile (online_unavailable)")))]
async fn online_psk(State(state): State<AppState>, _: Authed) -> Result<Json<Value>, ApiError> {
    let service = online_service(&state)?;
    Ok(Json(json!({ "enabled": service.settings().psk, "psk": service.psk() })))
}

/// Le condizioni di propagazione da hamqsl.com (se acceso).
#[utoipa::path(get, path = "/api/v1/online/solar", responses((status = 200, description = "`enabled` e `solar` (`fetched_ms`, `data`, `error`)"), (status = 404, description = "Non disponibile (online_unavailable)")))]
async fn online_solar(State(state): State<AppState>, _: Authed) -> Result<Json<Value>, ApiError> {
    let service = online_service(&state)?;
    Ok(Json(json!({ "enabled": service.settings().solar, "solar": service.solar() })))
}

/// Le DXpedition annunciate dal calendario NG3K (se acceso).
#[utoipa::path(get, path = "/api/v1/online/dxped", responses((status = 200, description = "`enabled` e `dxped` (`fetched_ms`, `items`, `error`)"), (status = 404, description = "Non disponibile (online_unavailable)")))]
async fn online_dxped(State(state): State<AppState>, _: Authed) -> Result<Json<Value>, ApiError> {
    let service = online_service(&state)?;
    Ok(Json(json!({ "enabled": service.settings().dxped, "dxped": service.dxped() })))
}

/// Chiede subito i dati ai servizi accesi, al massimo una volta al minuto per servizio; `asked` dice se ha chiesto qualcosa.
#[utoipa::path(post, path = "/api/v1/online/refresh", responses((status = 200, description = "`asked`"), (status = 404, description = "Non disponibile (online_unavailable)")))]
async fn online_refresh(State(state): State<AppState>, _: Authed) -> Result<Json<Value>, ApiError> {
    let service = online_service(&state)?;
    let callsign = state.current_status().map(|s| s.de_call).unwrap_or_default();
    match tokio::task::spawn_blocking(move || service.refresh_now(unix_ms(), &callsign)).await {
        Ok(asked) => Ok(Json(json!({ "asked": asked }))),
        Err(_) => Err(ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "online_task")),
    }
}

fn history_store(state: &AppState) -> Result<std::sync::Arc<decode_history::HistoryStore>, ApiError> {
    state.history.clone().ok_or_else(|| ApiError::new(StatusCode::NOT_FOUND, "history_unavailable"))
}

fn history_error(e: decode_history::HistoryError) -> ApiError {
    tracing::warn!("{e}");
    ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "history_error")
}

#[derive(Deserialize)]
struct HistoryParams {
    /// Parte di un nominativo o del testo del messaggio.
    q: Option<String>,
    band: Option<String>,
    mode: Option<String>,
    from_ms: Option<u64>,
    to_ms: Option<u64>,
    limit: Option<u32>,
    offset: Option<u32>,
}

/// I messaggi ricevuti, i piu' recenti per primi, filtrati e a pagine (al massimo 500 per volta).
#[utoipa::path(get, path = "/api/v1/history", responses((status = 200, description = "`total` e `rows`"), (status = 404, description = "Storico non disponibile (history_unavailable)")))]
async fn history_get(State(state): State<AppState>, _: Authed, axum::extract::Query(p): axum::extract::Query<HistoryParams>) -> Result<Json<Value>, ApiError> {
    let store = history_store(&state)?;
    let query = decode_history::Query { text: p.q, band: p.band, mode: p.mode, from_ms: p.from_ms, to_ms: p.to_ms, limit: p.limit.unwrap_or(100), offset: p.offset.unwrap_or(0) };
    match tokio::task::spawn_blocking(move || store.query(&query)).await {
        Ok(Ok(page)) => Ok(Json(json!(page))),
        Ok(Err(e)) => Err(history_error(e)),
        Err(_) => Err(ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "history_task")),
    }
}

/// Quanti messaggi e stazioni ci sono nello storico e da quando.
#[utoipa::path(get, path = "/api/v1/history/stats", responses((status = 200, description = "`total`, `oldest_ms`, `calls`"), (status = 404, description = "Storico non disponibile (history_unavailable)")))]
async fn history_stats(State(state): State<AppState>, _: Authed) -> Result<Json<Value>, ApiError> {
    let store = history_store(&state)?;
    match tokio::task::spawn_blocking(move || store.stats()).await {
        Ok(Ok(stats)) => Ok(Json(json!(stats))),
        Ok(Err(e)) => Err(history_error(e)),
        Err(_) => Err(ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "history_task")),
    }
}

/// Svuota lo storico dei messaggi (non tocca il log dei QSO).
#[utoipa::path(delete, path = "/api/v1/history", responses((status = 200, description = "Quanti messaggi sono stati tolti"), (status = 404, description = "Storico non disponibile (history_unavailable)")))]
async fn history_clear(State(state): State<AppState>, _: Authed) -> Result<Json<Value>, ApiError> {
    let store = history_store(&state)?;
    match tokio::task::spawn_blocking(move || store.clear()).await {
        Ok(Ok(removed)) => Ok(Json(json!({ "ok": true, "removed": removed }))),
        Ok(Err(e)) => Err(history_error(e)),
        Err(_) => Err(ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "history_task")),
    }
}

fn input_level_service(state: &AppState) -> Result<std::sync::Arc<input_level::InputLevelService>, ApiError> {
    state.input_level.clone().ok_or_else(|| ApiError::new(StatusCode::NOT_FOUND, "input_level_unavailable"))
}

/// Il volume di registrazione di Windows dell'ingresso audio e se la regolazione automatica (AUTO) e' accesa.
/// Il motore proprio: decodifica in ombra dall'audio e confronto con i messaggi di JTDX (quota ritrovata, messaggi solo di uno dei due).
#[utoipa::path(get, path = "/api/v1/engine/native", responses((status = 200, description = "Stato e confronto con JTDX; `enabled: false` se spento")))]
async fn native_engine_get(State(state): State<AppState>, _: Authed) -> Json<Value> {
    let status = state.native_status.read().ok().and_then(|slot| slot.as_ref().map(|f| f()));
    Json(status.unwrap_or_else(|| json!({ "enabled": false })))
}

#[utoipa::path(get, path = "/api/v1/input-level", responses((status = 200, description = "Dispositivo, volume 0-100 e AUTO"), (status = 404, description = "Nessun ingresso audio regolabile (input_level_unavailable)"), (status = 502, description = "Il dispositivo non risponde (input_level_device)")))]
async fn input_level_get(State(state): State<AppState>, _: Authed) -> Result<Json<Value>, ApiError> {
    let service = input_level_service(&state)?;
    match tokio::task::spawn_blocking(move || service.status()).await {
        Ok(Ok(status)) => Ok(Json(json!(status))),
        Ok(Err(e)) => {
            tracing::warn!("lettura del livello di ingresso fallita: {e}");
            Err(ApiError::new(StatusCode::BAD_GATEWAY, "input_level_device"))
        }
        Err(_) => Err(ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "input_level_task")),
    }
}

#[derive(Deserialize)]
struct InputLevelBody {
    volume_pct: Option<f64>,
    auto: Option<bool>,
}

/// Cambia il volume di registrazione (0-100) e/o la regolazione automatica. Con un volume scelto a mano AUTO si spegne, salvo che lo stesso corpo lo riaccenda.
#[utoipa::path(put, path = "/api/v1/input-level", responses((status = 200, description = "Impostato"), (status = 400, description = "Volume fuori scala o corpo vuoto (bad_input_level)"), (status = 404, description = "Nessun ingresso audio regolabile (input_level_unavailable)"), (status = 502, description = "Il dispositivo non risponde (input_level_device)")))]
async fn input_level_put(State(state): State<AppState>, _: Authed, Json(body): Json<InputLevelBody>) -> Result<Json<Value>, ApiError> {
    let service = input_level_service(&state)?;
    if body.volume_pct.is_none() && body.auto.is_none() {
        return Err(ApiError::new(StatusCode::BAD_REQUEST, "bad_input_level"));
    }
    let result = tokio::task::spawn_blocking(move || {
        if let Some(pct) = body.volume_pct {
            service.set_volume_pct(pct)?;
        }
        if let Some(auto) = body.auto {
            service.set_auto(auto);
        }
        service.status()
    })
    .await;
    match result {
        Ok(Ok(status)) => Ok(Json(json!({ "ok": true, "status": status }))),
        Ok(Err(input_level::InputLevelError::BadValue)) => Err(ApiError::new(StatusCode::BAD_REQUEST, "bad_input_level")),
        Ok(Err(e)) => {
            tracing::warn!("cambio del livello di ingresso fallito: {e}");
            Err(ApiError::new(StatusCode::BAD_GATEWAY, "input_level_device"))
        }
        Err(_) => Err(ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "input_level_task")),
    }
}

/// Stato degli interruttori di JTDX (AutoSeq, 1 QSO, Abilita Tx...).
#[utoipa::path(get, path = "/api/v1/controls", responses((status = 200, description = "Stato di ogni interruttore"), (status = 502, description = "JTDX non ha risposto (rig_error)"), (status = 503, description = "Comando non disponibile (rig_unavailable)")))]
async fn controls_get(State(state): State<AppState>, _: Authed) -> Result<Json<Value>, ApiError> {
    let rig = state.active_rig().ok_or_else(|| ApiError::new(StatusCode::SERVICE_UNAVAILABLE, "rig_unavailable"))?;
    match tokio::task::spawn_blocking(move || rig.controls_and_tx().map(|reading| (reading, rig.control_values().unwrap_or_default()))).await {
        Ok(Ok(((list, tx), values))) => {
            let map: serde_json::Map<String, Value> = list.into_iter().map(|(id, on)| (id, Value::Bool(on))).collect();
            // i controlli a valore numerico (la deriva di Q65), solo nel motore proprio
            let values: serde_json::Map<String, Value> = values.into_iter().map(|(id, v)| (id, Value::from(v))).collect();
            // i sei messaggi Tx di JTDX e quello selezionato (1-6), quando la radio li sa leggere
            let tx = tx.map(|t| json!({ "messages": t.messages, "selected": t.selected, "minute": t.minute_label, "autoseq": t.autoseq_label }));
            Ok(Json(json!({ "controls": map, "values": values, "tx": tx })))
        }
        Ok(Err(e)) => {
            tracing::warn!("lettura dei controlli di JTDX fallita: {e}");
            Err(ApiError::new(StatusCode::BAD_GATEWAY, "rig_error"))
        }
        Err(_) => Err(ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "rig_task")),
    }
}

#[derive(Deserialize)]
struct ControlBody {
    id: String,
    on: Option<bool>,
    /// Valore numerico per i controlli che lo hanno (la deriva massima di Q65, 0-50): al posto di `on`.
    value: Option<u32>,
}

/// Imposta un interruttore di JTDX o preme un suo pulsante. Accendere Abilita Tx o AutoTX puo' far trasmettere:
/// serve la trasmissione aperta e una via consentita; spegnerli e' sempre permesso.
#[utoipa::path(post, path = "/api/v1/commands/control", responses((status = 200, description = "Fatto"), (status = 400, description = "Controllo sconosciuto o argomenti sbagliati (bad_control)"), (status = 403, description = "Trasmissione non abilitata o via non consentita"), (status = 502, description = "JTDX non ha risposto (rig_error)"), (status = 503, description = "Comando non disponibile (rig_unavailable)")))]
async fn control_press(State(state): State<AppState>, _: Authed, headers: HeaderMap, Json(body): Json<ControlBody>) -> Result<Json<Value>, ApiError> {
    let control = rig_control::controls::find(&body.id).ok_or_else(|| ApiError::new(StatusCode::BAD_REQUEST, "bad_control"))?;
    // un valore numerico va solo ai controlli del motore proprio che ne hanno uno (lo verifica il motore)
    if let Some(value) = body.value {
        if body.on.is_some() || control.tx {
            return Err(ApiError::new(StatusCode::BAD_REQUEST, "bad_control"));
        }
        let rig = state.active_rig().ok_or_else(|| ApiError::new(StatusCode::SERVICE_UNAVAILABLE, "rig_unavailable"))?;
        let id = control.id;
        return match tokio::task::spawn_blocking(move || rig.set_control_value(id, value)).await {
            Ok(Ok(())) => Ok(Json(json!({ "ok": true, "id": id, "value": value }))),
            Ok(Err(e)) => {
                tracing::warn!("valore del controllo {id} non impostato: {e}");
                Err(ApiError::new(StatusCode::BAD_REQUEST, "bad_control"))
            }
            Err(_) => Err(ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "rig_task")),
        };
    }
    let wants_state = control.kind == rig_control::controls::Kind::Toggle;
    if wants_state != body.on.is_some() {
        return Err(ApiError::new(StatusCode::BAD_REQUEST, "bad_control"));
    }
    if control.tx && body.on == Some(true) {
        check_tx_allowed(&state, &headers)?;
    }
    let rig = state.active_rig().ok_or_else(|| ApiError::new(StatusCode::SERVICE_UNAVAILABLE, "rig_unavailable"))?;
    let (id, on) = (control.id, body.on);
    match tokio::task::spawn_blocking(move || rig.set_control(id, on)).await {
        Ok(Ok(())) => Ok(Json(json!({ "ok": true, "id": id }))),
        Ok(Err(e)) => {
            tracing::warn!("comando a JTDX fallito ({id}): {e}");
            Err(ApiError::new(StatusCode::BAD_GATEWAY, "rig_error"))
        }
        Err(_) => Err(ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "rig_task")),
    }
}

#[derive(Deserialize)]
struct TxMessageBody {
    /// Numero del messaggio Tx, da 1 a 6.
    n: u8,
}

/// Sceglie a mano il messaggio Tx da cui il motore parte (come il pallino accanto ai messaggi): trasmette quello alla
/// prossima occasione e poi prosegue da solo. Non accende la trasmissione e non ne avvia una: vale anche a trasmissione chiusa.
#[utoipa::path(post, path = "/api/v1/commands/tx-message", responses((status = 200, description = "Messaggio selezionato"), (status = 400, description = "Numero fuori da 1-6 (bad_message)"), (status = 502, description = "JTDX non ha risposto (rig_error)"), (status = 503, description = "Comando non disponibile (rig_unavailable)")))]
async fn tx_message(State(state): State<AppState>, _: Authed, Json(body): Json<TxMessageBody>) -> Result<Json<Value>, ApiError> {
    if !(1..=6).contains(&body.n) {
        return Err(ApiError::new(StatusCode::BAD_REQUEST, "bad_message"));
    }
    let rig = state.active_rig().ok_or_else(|| ApiError::new(StatusCode::SERVICE_UNAVAILABLE, "rig_unavailable"))?;
    let n = body.n;
    match tokio::task::spawn_blocking(move || rig.select_tx_message(n)).await {
        Ok(Ok(())) => Ok(Json(json!({ "ok": true, "n": n }))),
        Ok(Err(e)) => {
            tracing::warn!("selezione del messaggio Tx {n} fallita: {e}");
            Err(ApiError::new(StatusCode::BAD_GATEWAY, "rig_error"))
        }
        Err(_) => Err(ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "rig_task")),
    }
}

#[derive(Deserialize)]
struct TxTextBody {
    /// Numero del messaggio Tx, da 1 a 6.
    n: u8,
    /// Il testo scritto a mano; vuoto torna al messaggio standard.
    text: String,
}

/// Scrive a mano il testo di un messaggio Tx (il campo accanto al numero): vale finche' i messaggi non si rigenerano (Gen.Msgs, nuovo rapporto,
/// stazione liberata). Maiuscolo, al piu' 37 caratteri; vuoto = torna lo standard. Non fa trasmettere.
#[utoipa::path(post, path = "/api/v1/commands/tx-text", responses((status = 200, description = "Testo impostato"), (status = 400, description = "Numero fuori da 1-6 (bad_message)"), (status = 503, description = "Comando non disponibile (rig_unavailable)")))]
async fn tx_text(State(state): State<AppState>, _: Authed, Json(body): Json<TxTextBody>) -> Result<Json<Value>, ApiError> {
    if !(1..=6).contains(&body.n) || body.text.len() > 400 {
        return Err(ApiError::new(StatusCode::BAD_REQUEST, "bad_message"));
    }
    let rig = state.active_rig().ok_or_else(|| ApiError::new(StatusCode::SERVICE_UNAVAILABLE, "rig_unavailable"))?;
    let (n, text) = (body.n, body.text);
    match tokio::task::spawn_blocking(move || rig.set_tx_text(n, &text)).await {
        Ok(Ok(())) => Ok(Json(json!({ "ok": true, "n": n }))),
        Ok(Err(e)) => {
            tracing::warn!("testo del messaggio Tx {n} non impostato: {e}");
            Err(ApiError::new(StatusCode::BAD_GATEWAY, "rig_error"))
        }
        Err(_) => Err(ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "rig_task")),
    }
}

#[derive(Deserialize)]
struct RxFrequencyBody {
    hz: u32,
}

/// Sposta la frequenza di ricezione (barra verde). Non fa trasmettere, quindi vale anche in trasmissione.
#[utoipa::path(post, path = "/api/v1/commands/rx-frequency", responses((status = 200, description = "Frequenza Rx impostata"), (status = 400, description = "Fuori dalla banda audio (100-3000 Hz)"), (status = 502, description = "JTDX non ha risposto (rig_error)"), (status = 503, description = "Comando non disponibile (rig_unavailable)")))]
async fn rx_frequency(State(state): State<AppState>, _: Authed, Json(body): Json<RxFrequencyBody>) -> Result<Json<Value>, ApiError> {
    let rig = state.active_rig().ok_or_else(|| ApiError::new(StatusCode::SERVICE_UNAVAILABLE, "rig_unavailable"))?;
    if !(100..=3000).contains(&body.hz) {
        return Err(ApiError::new(StatusCode::BAD_REQUEST, "bad_frequency"));
    }
    let hz = body.hz;
    match tokio::task::spawn_blocking(move || rig.set_rx_frequency(hz)).await {
        Ok(Ok(())) => Ok(Json(json!({ "ok": true, "hz": hz }))),
        Ok(Err(e)) => {
            tracing::warn!("cambio frequenza Rx fallito: {e}");
            Err(ApiError::new(StatusCode::BAD_GATEWAY, "rig_error"))
        }
        Err(_) => Err(ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "rig_task")),
    }
}

#[derive(Deserialize)]
struct ModeBody {
    mode: String,
}

/// Cambia tra FT8 e FT4 (con JTDX preme il suo pulsante). Con il motore proprio funziona anche senza radio collegata. Mai durante la trasmissione.
#[utoipa::path(post, path = "/api/v1/commands/mode", responses((status = 200, description = "Modo cambiato"), (status = 400, description = "Modo sconosciuto (bad_mode)"), (status = 409, description = "In trasmissione (transmitting)"), (status = 502, description = "JTDX non ha risposto (rig_error)"), (status = 503, description = "Comando non disponibile (rig_unavailable)")))]
async fn change_mode(State(state): State<AppState>, _: Authed, Json(body): Json<ModeBody>) -> Result<Json<Value>, ApiError> {
    if state.active_rig().is_none() {
        return Err(ApiError::new(StatusCode::SERVICE_UNAVAILABLE, "rig_unavailable"));
    }
    if rig_control::modes::button_id(&body.mode).is_none() {
        return Err(ApiError::new(StatusCode::BAD_REQUEST, "bad_mode"));
    }
    if state.current_status().is_some_and(|s| s.transmitting) {
        return Err(ApiError::new(StatusCode::CONFLICT, "transmitting"));
    }
    let shown = body.mode.trim().to_uppercase();
    match switch_mode(&state, &body.mode).await {
        Ok(dial) => Ok(Json(json!({ "ok": true, "mode": shown, "dial_hz": dial }))),
        Err(e) if e == "rig_task" => Err(ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "rig_task")),
        Err(e) => {
            tracing::warn!("cambio modo fallito: {e}");
            Err(ApiError::new(StatusCode::BAD_GATEWAY, "rig_error"))
        }
    }
}

/// Cambia modo come il pulsante della pagina: il motore passa al modo e la radio va sulla frequenza che la tabella indica per il nuovo modo nella
/// stessa banda. Serve anche al Configure (tipo 15) dei programmi collegati via UDP. Restituisce la frequenza di dial scelta (se cambiata);
/// l'errore e' un testo (`rig_task` se il lavoro non e' partito).
pub async fn switch_mode(state: &AppState, mode: &str) -> Result<Option<u64>, String> {
    let rig = state.active_rig().ok_or_else(|| "nessuna radio o motore attivo".to_string())?;
    let mode = mode.trim().to_uppercase();
    // JTDX al cambio di modo salta all'ultima frequenza che aveva in quel modo, anche su un'altra banda:
    // si resta nella banda di prima, sulla frequenza che la tabella indica per il nuovo modo
    // la frequenza di adesso si rilegge dalla radio (dopo lo spot di un cluster lo stato della pagina puo' essere indietro); se la radio
    // non risponde si usa l'ultima nota. La banda si cerca nella tabella e, fuori da quella, tra le bande dei radioamatori.
    let status_dial = state.current_status().filter(|s| s.dial_hz > 0).map(|s| s.dial_hz);
    let frequencies = std::sync::Arc::clone(&state.frequencies);
    let region = state.frequency_region();
    let work = move || -> Result<Option<u64>, rig_control::RigError> {
        rig.set_mode(&mode)?;
        // senza radio collegata (radioascolto senza CAT) il motore cambia modo lo stesso: la frequenza resta quella che si sceglie sulla radio
        let read = rig.frequency().ok().filter(|hz| *hz > 0);
        let radio_answers = read.is_some();
        let current = read.or(status_dial);
        let target = current.and_then(|now| {
            let table = frequencies.lock().unwrap_or_else(|p| p.into_inner());
            // come `MainWindow::switch_mode` di JTDX: resta sulla riga del nuovo modo se il dial e' gia' una riga, altrimenti la predefinita piu' vicina
            // della banda del dial; fuori dalle bande della tabella si cerca la banda tra quelle dei radioamatori
            table.best_working(region, &mode, now).or_else(|| {
                let band = rig_control::bands::amateur_band(now)?;
                table.frequency_in(region, band, &mode)
            })
        });
        if let Some(hz) = target {
            match rig.set_frequency(hz) {
                Ok(()) => {}
                Err(e) if !radio_answers => {
                    tracing::debug!("modo {mode} cambiato senza spostare la radio: {e}");
                    return Ok(None);
                }
                Err(e) => return Err(e),
            }
        }
        Ok(target)
    };
    match tokio::task::spawn_blocking(work).await {
        Ok(Ok(dial)) => Ok(dial),
        Ok(Err(e)) => Err(e.to_string()),
        Err(_) => Err("rig_task".to_string()),
    }
}

#[derive(Deserialize)]
struct TxFrequencyBody {
    hz: u32,
}

#[utoipa::path(post, path = "/api/v1/commands/tx-frequency", responses((status = 200, description = "Frequenza audio di trasmissione impostata"), (status = 400, description = "Fuori dalla banda audio (200-3000 Hz)")))]
async fn tx_frequency(State(state): State<AppState>, _: Authed, Json(body): Json<TxFrequencyBody>) -> Result<Json<Value>, ApiError> {
    if !(200..=3000).contains(&body.hz) {
        return Err(ApiError::new(StatusCode::BAD_REQUEST, "bad_frequency"));
    }
    dispatch(&state, Command::SetTxFrequency { hz: body.hz })
}

#[utoipa::path(get, path = "/api/v1/automation", responses((status = 200, description = "Stato, regole, limiti e registro delle decisioni"), (status = 404, description = "Automazione non disponibile")))]
async fn automation_get(State(state): State<AppState>, _: Authed) -> Result<Json<Value>, ApiError> {
    let automation = state.automation.as_ref().ok_or_else(|| ApiError::new(StatusCode::NOT_FOUND, "automation_unavailable"))?;
    Ok(Json(serde_json::to_value(automation.state()).unwrap_or(Value::Null)))
}

/// I paesi di cty.dat (nomi in ordine), per scegliere i paesi esclusi dall'automazione; vuoto senza cty.dat.
#[utoipa::path(get, path = "/api/v1/automation/countries", responses((status = 200, description = "Elenco dei paesi DXCC"), (status = 404, description = "Automazione non disponibile")))]
async fn automation_countries(State(state): State<AppState>, _: Authed) -> Result<Json<Value>, ApiError> {
    let automation = state.automation.as_ref().ok_or_else(|| ApiError::new(StatusCode::NOT_FOUND, "automation_unavailable"))?;
    Ok(Json(serde_json::json!({ "countries": *automation.countries() })))
}

#[derive(Deserialize)]
struct AutomationBody {
    enabled: Option<bool>,
    mode: Option<Mode>,
    rules: Option<Rules>,
    /// Conferma dell'avviso sulla responsabilita' della trasmissione automatica.
    #[serde(default)]
    accept_license_notice: bool,
}

/// Accendere e spegnere: dal telefono si accende solo in prova a vuoto. Il modo reale si sceglie dal PC.
#[utoipa::path(put, path = "/api/v1/automation", responses((status = 200, description = "Applicato; stato aggiornato"), (status = 400, description = "Regole non valide o avviso non confermato"), (status = 403, description = "Modo reale non consentito (local_only, tx_disabled)"), (status = 404, description = "Automazione non disponibile")))]
async fn automation_put(
    State(state): State<AppState>,
    _: Authed,
    Peer(peer): Peer,
    headers: HeaderMap,
    Json(body): Json<AutomationBody>,
) -> Result<Json<Value>, ApiError> {
    let automation = state.automation.as_ref().ok_or_else(|| ApiError::new(StatusCode::NOT_FOUND, "automation_unavailable"))?;
    let from_pc = peer.is_some_and(|p| p.ip().is_loopback()) && !is_proxied(&headers);
    if let Some(rules) = &body.rules
        && !rules.is_valid()
    {
        return Err(ApiError::new(StatusCode::BAD_REQUEST, "bad_rules"));
    }
    if body.mode == Some(Mode::Live) {
        if !from_pc {
            return Err(ApiError::new(StatusCode::FORBIDDEN, "local_only"));
        }
        if !body.accept_license_notice {
            return Err(ApiError::new(StatusCode::BAD_REQUEST, "notice_required"));
        }
        if !automation.state().live_allowed {
            return Err(ApiError::new(StatusCode::FORBIDDEN, "tx_disabled"));
        }
    }
    let mode_after = body.mode.unwrap_or(automation.state().mode);
    if body.enabled == Some(true) && mode_after == Mode::Live && !from_pc {
        return Err(ApiError::new(StatusCode::FORBIDDEN, "local_only"));
    }
    // tutto convalidato: ora si applica
    if let Some(rules) = body.rules {
        if let Some(save) = &state.rules_saver {
            save(&rules);
        }
        automation.set_rules(rules);
    }
    if let Some(mode) = body.mode {
        automation.set_mode(mode).map_err(|_| ApiError::new(StatusCode::FORBIDDEN, "tx_disabled"))?;
    }
    if let Some(enabled) = body.enabled {
        automation.set_enabled(enabled);
    }
    Ok(Json(serde_json::to_value(automation.state()).unwrap_or(Value::Null)))
}

#[utoipa::path(get, path = "/api/v1/station-info", responses((status = 200, description = "Come e' avviata la stazione (sola lettura)")))]
async fn station_info(State(state): State<AppState>, _: Authed) -> Json<Value> {
    let mut info = (*state.station_info).clone();
    // la radio collegata dalla scheda Radio dopo l'avvio conta: la schermata principale ne mostra i comandi
    if state.cat.as_ref().is_some_and(|c| c.is_usable())
        && let Some(map) = info.as_object_mut()
    {
        map.insert("rig_control".into(), Value::Bool(true));
    }
    // senza radio comandabile le bande si scelgono lo stesso: cambiano solo la frequenza di Aurora
    if state.manual_dial.is_some()
        && state.dial_rig().is_none()
        && let Some(map) = info.as_object_mut()
    {
        map.insert("manual_dial".into(), Value::Bool(true));
    }
    Json(info)
}

#[utoipa::path(get, path = "/api/v1/stats", responses((status = 200, description = "Avanzamento nei premi e paesi mancanti"), (status = 404, description = "Log non disponibile (stats_unavailable)")))]
async fn stats(State(state): State<AppState>, _: Authed) -> Result<Json<Value>, ApiError> {
    let compute = state.stats.clone().ok_or_else(|| ApiError::new(StatusCode::NOT_FOUND, "stats_unavailable"))?;
    // legge e analizza il log: fuori dal thread asincrono
    tokio::task::spawn_blocking(move || compute())
        .await
        .map(Json)
        .map_err(|_| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "stats_task"))
}

#[utoipa::path(get, path = "/api/v1/dxcc/{call}", responses((status = 200, description = "Paese, continente e zone del nominativo (dal file cty.dat)"), (status = 400, description = "Nominativo non valido (bad_call)"), (status = 404, description = "Sconosciuto o file non caricato (dxcc_unknown)")))]
async fn dxcc_lookup(State(state): State<AppState>, _: Authed, axum::extract::Path(call): axum::extract::Path<String>) -> Result<Json<Value>, ApiError> {
    if call.is_empty() || call.len() > 20 || !call.chars().all(|c| c.is_ascii_alphanumeric() || c == '/') {
        return Err(ApiError::new(StatusCode::BAD_REQUEST, "bad_call"));
    }
    state.dxcc.as_ref().and_then(|lookup| lookup(&call)).map(Json).ok_or_else(|| ApiError::new(StatusCode::NOT_FOUND, "dxcc_unknown"))
}

/// Paese da `cty.dat` e, se accese, nome e citta' da QRZ.com (prima) o HamQTH (se QRZ e' spento o non lo trova). Del nominativo cercato
/// si restituiscono solo nome, cognome, citta' e paese. Nessuna richiesta verso internet con le ricerche spente.
#[utoipa::path(get, path = "/api/v1/callsign/{call}", responses((status = 200, description = "`call`, `dxcc` (come `dxcc/{call}`, o null), `person` (`fname`, `name`, `city`, `country`, `lotw`, `eqsl`, `direct`, `bureau` (true/false/null; da HamQTH se QRZ non li da'), `qsl_via`, `source`) o null, `lookup` (`off`, `found`, `not_found`, `error`), `subscriber` (QRZ con abbonamento XML; null se non si sa), `message`"), (status = 400, description = "Nominativo non valido (bad_call)")))]
async fn callsign_lookup(State(state): State<AppState>, _: Authed, axum::extract::Path(call): axum::extract::Path<String>) -> Result<Json<Value>, ApiError> {
    if call.is_empty() || call.len() > 20 || !call.chars().all(|c| c.is_ascii_alphanumeric() || c == '/') {
        return Err(ApiError::new(StatusCode::BAD_REQUEST, "bad_call"));
    }
    let call = call.to_uppercase();
    let dxcc = state.dxcc.as_ref().and_then(|lookup| lookup(&call));
    let mut out = json!({ "call": call, "dxcc": dxcc, "person": null, "lookup": "off", "subscriber": null, "message": null });
    let Some(settings) = state.engine_settings.clone() else { return Ok(Json(out)) };
    let now = unix_ms() / 1000;
    // QRZ senza abbonamento non manda le QSL: se mancano si completano con HamQTH (gratis), senza cambiare nome e citta' gia' trovati
    let mut found: Option<(reporting::qrz_xml::Person, &'static str)> = None;
    for provider in [reporting::qrz_xml::Provider::Qrz, reporting::qrz_xml::Provider::HamQth] {
        if found.as_ref().is_some_and(|(p, _)| !p.qsl_unknown()) {
            break;
        }
        let Some((user, password)) = settings.lookup_credentials(provider) else { continue };
        let lookup = match provider {
            reporting::qrz_xml::Provider::Qrz => std::sync::Arc::clone(&settings.lookups().qrz),
            reporting::qrz_xml::Provider::HamQth => std::sync::Arc::clone(&settings.lookups().hamqth),
        };
        let wanted = call.clone();
        let result = tokio::task::spawn_blocking(move || (lookup.lookup(&user, &password, &wanted, now), lookup.subscriber())).await;
        let Ok((result, subscriber)) = result else { continue };
        if provider == reporting::qrz_xml::Provider::Qrz {
            out["subscriber"] = json!(subscriber);
        }
        match result {
            Ok(reporting::qrz_xml::Found::Yes(p)) => {
                match found.as_mut() {
                    Some((first, _)) => first.fill_from(&p),
                    None => found = Some((p, provider.name())),
                }
                out["lookup"] = json!("found");
                continue;
            }
            Ok(reporting::qrz_xml::Found::NotFound) if found.is_none() => out["lookup"] = json!("not_found"),
            Ok(reporting::qrz_xml::Found::NotFound) => {}
            Err(e) => {
                tracing::debug!("ricerca di {call} su {} non riuscita: {}", provider.name(), e.message());
                if out["lookup"] == "off" && found.is_none() {
                    out["lookup"] = json!("error");
                    out["message"] = json!(format!("{}: {}", provider.name(), e.message()));
                }
            }
        }
    }
    if let Some((p, source)) = found {
        out["person"] = json!({ "fname": p.fname, "name": p.name, "city": p.city, "country": p.country, "lotw": p.lotw, "eqsl": p.eqsl, "direct": p.direct, "bureau": p.bureau, "qsl_via": p.qsl_via, "source": source });
        out["lookup"] = json!("found");
    }
    Ok(Json(out))
}

fn cluster_service(state: &AppState) -> Result<std::sync::Arc<dx_cluster::ClusterService>, ApiError> {
    state.cluster.clone().ok_or_else(|| ApiError::new(StatusCode::NOT_FOUND, "cluster_unavailable"))
}

#[derive(Deserialize)]
struct ClusterQuery {
    since: Option<u64>,
    limit: Option<usize>,
}

#[utoipa::path(get, path = "/api/v1/dx-cluster", responses((status = 200, description = "Impostazioni, stato dei collegamenti e spot piu' recenti di `since` (piu' nuovi per primi)"), (status = 404, description = "Non disponibile (cluster_unavailable)")))]
async fn cluster_get(State(state): State<AppState>, _: Authed, axum::extract::Query(q): axum::extract::Query<ClusterQuery>) -> Result<Json<Value>, ApiError> {
    let cluster = cluster_service(&state)?;
    let limit = q.limit.unwrap_or(200).clamp(1, dx_cluster::MAX_SPOTS);
    Ok(Json(json!({ "config": cluster.config(), "callsign": cluster.callsign(), "servers": cluster.statuses(), "spots": cluster.spots_since(q.since.unwrap_or(0), limit) })))
}

#[utoipa::path(put, path = "/api/v1/dx-cluster", responses((status = 200, description = "Impostazioni salvate e collegamenti rifatti"), (status = 400, description = "Valori non validi (bad_cluster)"), (status = 404, description = "Non disponibile (cluster_unavailable)"), (status = 500, description = "Scrittura non riuscita (cluster_write)")))]
async fn cluster_put(State(state): State<AppState>, _: Authed, Json(config): Json<dx_cluster::Config>) -> Result<Json<Value>, ApiError> {
    let cluster = cluster_service(&state)?;
    config.validate().map_err(|_| ApiError::new(StatusCode::BAD_REQUEST, "bad_cluster"))?;
    // salva e rifa' i collegamenti: servono dentro il runtime, ma la scrittura su disco e' breve
    cluster.set_config(config).map_err(|e| {
        tracing::warn!("salvataggio delle impostazioni del DX Cluster fallito: {e}");
        ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "cluster_write")
    })?;
    Ok(Json(json!({ "ok": true })))
}

#[utoipa::path(get, path = "/api/v1/dx-cluster/catalog", responses((status = 200, description = "Server DX Cluster pubblici noti, da aggiungere con un clic")))]
async fn cluster_catalog(_: Authed) -> Json<Value> {
    Json(json!(dx_cluster::CATALOG))
}

#[derive(Deserialize)]
struct ClusterTestBody {
    host: String,
    port: u16,
}

#[utoipa::path(post, path = "/api/v1/dx-cluster/test", responses((status = 200, description = "Esito della prova: raggiungibile o no, tempo e benvenuto del server"), (status = 400, description = "Indirizzo non valido (bad_server)")))]
async fn cluster_test(_: Authed, Json(body): Json<ClusterTestBody>) -> Result<Json<Value>, ApiError> {
    if body.host.is_empty() || body.host.len() > 253 || !body.host.chars().all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '-') || body.port == 0 {
        return Err(ApiError::new(StatusCode::BAD_REQUEST, "bad_server"));
    }
    Ok(Json(json!(dx_cluster::probe(&body.host, body.port).await)))
}

#[derive(Deserialize)]
struct ClusterCommandBody {
    server: String,
    text: String,
}

#[utoipa::path(post, path = "/api/v1/dx-cluster/command", responses((status = 200, description = "Comando inviato al cluster"), (status = 400, description = "Comando non ammesso: solo consultazione e impostazioni (bad_command)"), (status = 409, description = "Server non collegato (not_connected)"), (status = 404, description = "Non disponibile (cluster_unavailable)")))]
async fn cluster_command(State(state): State<AppState>, _: Authed, Json(body): Json<ClusterCommandBody>) -> Result<Json<Value>, ApiError> {
    let cluster = cluster_service(&state)?;
    if !dx_cluster::valid_command(&body.text) {
        return Err(ApiError::new(StatusCode::BAD_REQUEST, "bad_command"));
    }
    cluster.send_command(&body.server, &body.text).map_err(|_| ApiError::new(StatusCode::CONFLICT, "not_connected"))?;
    Ok(Json(json!({ "ok": true })))
}

fn clock_service(state: &AppState) -> Result<std::sync::Arc<clock_sync::ClockService>, ApiError> {
    state.clock.clone().ok_or_else(|| ApiError::new(StatusCode::NOT_FOUND, "clock_unavailable"))
}

/// Cambiare l'ora del PC e' un'azione sul computer: solo dalla rete di casa, mai da un proxy.
fn from_home_network(peer: Option<SocketAddr>, headers: &HeaderMap) -> bool {
    peer.is_some_and(|p| is_home_network(p.ip())) && !is_proxied(headers)
}

#[utoipa::path(get, path = "/api/v1/clock", responses((status = 200, description = "Impostazioni del client NTP e ultima misura dell'orologio"), (status = 404, description = "Non disponibile (clock_unavailable)")))]
async fn clock_get(State(state): State<AppState>, _: Authed) -> Result<Json<Value>, ApiError> {
    let clock = clock_service(&state)?;
    Ok(Json(json!({ "config": clock.config(), "last": clock.last(), "last_good_ms": clock.last_good_ms() })))
}

#[utoipa::path(put, path = "/api/v1/clock", responses((status = 200, description = "Impostazioni salvate"), (status = 400, description = "Valori non validi (bad_clock)"), (status = 403, description = "La correzione automatica si accende solo dalla rete di casa (local_only)"), (status = 404, description = "Non disponibile (clock_unavailable)"), (status = 500, description = "Scrittura non riuscita (clock_write)")))]
async fn clock_put(State(state): State<AppState>, _: Authed, Peer(peer): Peer, headers: HeaderMap, Json(config): Json<clock_sync::Config>) -> Result<Json<Value>, ApiError> {
    let clock = clock_service(&state)?;
    if config.auto_correct && !clock.config().auto_correct && !from_home_network(peer, &headers) {
        return Err(ApiError::new(StatusCode::FORBIDDEN, "local_only"));
    }
    config.validate().map_err(|_| ApiError::new(StatusCode::BAD_REQUEST, "bad_clock"))?;
    tokio::task::spawn_blocking(move || clock.set_config(config))
        .await
        .map_err(|_| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "clock_task"))?
        .map_err(|e| {
            tracing::warn!("salvataggio delle impostazioni dell'orologio fallito: {e}");
            ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "clock_write")
        })?;
    Ok(Json(json!({ "ok": true })))
}

#[utoipa::path(post, path = "/api/v1/clock/check", responses((status = 200, description = "Misura fatta ora: scarto e ritardo di rete"), (status = 404, description = "Non disponibile (clock_unavailable)")))]
async fn clock_check(State(state): State<AppState>, _: Authed) -> Result<Json<Value>, ApiError> {
    let clock = clock_service(&state)?;
    tokio::task::spawn_blocking(move || json!(clock.check_now()))
        .await
        .map(Json)
        .map_err(|_| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "clock_task"))
}

#[utoipa::path(post, path = "/api/v1/clock/correct", responses((status = 200, description = "Orologio corretto; nuova misura"), (status = 403, description = "Solo dalla rete di casa (local_only) o serve l'amministratore (clock_needs_admin)"), (status = 404, description = "Non disponibile (clock_unavailable)"), (status = 502, description = "Nessun server ha risposto o correzione fallita (clock_correct_failed)")))]
async fn clock_correct(State(state): State<AppState>, _: Authed, Peer(peer): Peer, headers: HeaderMap) -> Result<Json<Value>, ApiError> {
    let clock = clock_service(&state)?;
    if !from_home_network(peer, &headers) {
        return Err(ApiError::new(StatusCode::FORBIDDEN, "local_only"));
    }
    match tokio::task::spawn_blocking(move || clock.correct_now()).await {
        Ok(Ok(m)) => Ok(Json(json!(m))),
        Ok(Err(e)) if e.contains("amministratore") => Err(ApiError::new(StatusCode::FORBIDDEN, "clock_needs_admin")),
        Ok(Err(e)) => {
            tracing::warn!("correzione dell'orologio fallita: {e}");
            Err(ApiError::new(StatusCode::BAD_GATEWAY, "clock_correct_failed"))
        }
        Err(_) => Err(ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "clock_task")),
    }
}


#[utoipa::path(get, path = "/api/v1/engine-settings", responses((status = 200, description = "Impostazioni di Aurora per il motore proprio: valori e, per ogni voce, se e' viva o non disponibile (con il motivo)"), (status = 404, description = "Solo con il motore proprio (engine_settings_unavailable)")))]
async fn engine_settings_get(State(state): State<AppState>, _: Authed) -> Result<Json<Value>, ApiError> {
    let store = state.engine_settings.clone().ok_or_else(|| ApiError::new(StatusCode::NOT_FOUND, "engine_settings_unavailable"))?;
    Ok(Json(store.document()))
}

#[derive(Deserialize)]
struct EngineSettingsBody {
    changes: std::collections::BTreeMap<String, String>,
}

/// Cambia le impostazioni del motore proprio: Aurora le salva nel suo file e le applica subito (nessun riavvio). Solo dalla rete di casa:
/// cambiano a chi Aurora manda i QSO.
#[utoipa::path(put, path = "/api/v1/engine-settings", responses((status = 200, description = "Salvate e applicate; restituisce il documento aggiornato"), (status = 400, description = "Voce o valore non ammessi (bad_setting)"), (status = 403, description = "Solo dalla rete di casa (local_only)"), (status = 404, description = "Solo con il motore proprio (engine_settings_unavailable)"), (status = 500, description = "Scrittura non riuscita (engine_settings_write)")))]
async fn engine_settings_put(State(state): State<AppState>, _: Authed, Peer(peer): Peer, headers: HeaderMap, Json(body): Json<EngineSettingsBody>) -> Result<Json<Value>, ApiError> {
    let store = state.engine_settings.clone().ok_or_else(|| ApiError::new(StatusCode::NOT_FOUND, "engine_settings_unavailable"))?;
    if !from_home_network(peer, &headers) {
        return Err(ApiError::new(StatusCode::FORBIDDEN, "local_only"));
    }
    if body.changes.is_empty() || body.changes.len() > 50 {
        return Err(ApiError::new(StatusCode::BAD_REQUEST, "bad_setting"));
    }
    let doc = tokio::task::spawn_blocking(move || store.update(&body.changes).map(|()| store.document()))
        .await
        .map_err(|_| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "engine_settings_task"))?;
    match doc {
        Ok(doc) => Ok(Json(doc)),
        Err(crate::engine_settings::SettingsError::Io(e)) => {
            tracing::warn!("impostazioni del motore non salvate: {e}");
            Err(ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "engine_settings_write"))
        }
        // il perche' vero («UDP2Server: serve un indirizzo…»): l'interfaccia lo mostra accanto alla voce
        Err(e) => Err(ApiError::new(StatusCode::BAD_REQUEST, "bad_setting").with_message(e.to_string())),
    }
}

#[derive(Deserialize)]
struct SpotDxsummitBody {
    dx_call: String,
    /// Il locatore del DX, se si conosce (serve alle macro `#G` e `#D` del testo).
    #[serde(default)]
    dx_grid: String,
    /// Il testo dello spot, con le macro `#G`, `#D`, `#R` come nella riga di testo di JTDX.
    #[serde(default)]
    text: String,
    /// La propagazione (`Es`, `F2`...), come il campo di JTDX.
    #[serde(default)]
    prop: String,
    /// Frequenza dello spot in kHz, scelta a mano (finestra «Invia spot» dei modi testuali): senza, si usa radio piu' frequenza di ricezione del motore.
    #[serde(default)]
    freq_khz: Option<f64>,
    /// Il campo `info` completo (modo e testo), insieme a `freq_khz`.
    #[serde(default)]
    info: Option<String>,
}

/// Segnala la stazione scelta su DXSummit, come il pulsante «DX Call» di JTDX: frequenza = radio piu' frequenza di ricezione, `info` = modo e testo.
/// Non blocca il motore (richiesta HTTP a parte, attesa massima 10 s) e non segnala due volte la stessa stazione in breve tempo. Solo dalla rete di casa.
#[utoipa::path(post, path = "/api/v1/spot/dxsummit", responses((status = 200, description = "Spot inviato: `{ok, dx_call, frequency_hz, info}`"), (status = 400, description = "Nominativo o testo non validi (bad_spot)"), (status = 403, description = "Solo dalla rete di casa (local_only)"), (status = 404, description = "Solo con il motore proprio (spot_unavailable)"), (status = 409, description = "Spotting spento (spot_off), manca il nominativo (spot_no_call), stazione gia' segnalata (spot_already), frequenza non nota (spot_no_frequency)"), (status = 502, description = "DXSummit non ha risposto o ha dato errore (spot_failed, con `message`)")))]
async fn spot_dxsummit(State(state): State<AppState>, _: Authed, Peer(peer): Peer, headers: HeaderMap, Json(body): Json<SpotDxsummitBody>) -> Result<Json<Value>, ApiError> {
    if !from_home_network(peer, &headers) {
        return Err(ApiError::new(StatusCode::FORBIDDEN, "local_only"));
    }
    let service = state.engine_settings.as_ref().and_then(|s| s.reporting()).ok_or_else(|| ApiError::new(StatusCode::NOT_FOUND, "spot_unavailable"))?;
    if body.text.chars().count() > 120 || body.prop.chars().count() > 10 || body.dx_grid.chars().count() > 8 {
        return Err(ApiError::new(StatusCode::BAD_REQUEST, "bad_spot"));
    }
    // frequenza e testo scelti a mano (PSK, RTTY...): lo spot parte com'e', senza i dati dell'FT8 del motore
    let manual = match body.freq_khz {
        Some(khz) if khz.is_finite() && (100.0..=1_000_000.0).contains(&khz) => {
            let info = body.info.clone().unwrap_or_default();
            if info.chars().count() > 120 {
                return Err(ApiError::new(StatusCode::BAD_REQUEST, "bad_spot"));
            }
            Some(((khz * 1000.0).round() as u64, info))
        }
        Some(_) => return Err(ApiError::new(StatusCode::BAD_REQUEST, "bad_spot")),
        None => None,
    };
    let sent = if let Some((freq_hz, info)) = manual {
        let call = body.dx_call.trim().to_uppercase();
        service.spot_dxsummit(&call, freq_hz, &info).await.map(|()| reporting::SpotDone { dx_call: call, freq_hz, info })
    } else {
        let params = reporting::SpotParams { dx_call: body.dx_call, dx_grid: body.dx_grid, text: body.text, prop: body.prop };
        service.spot_current(params).await
    };
    match sent {
        Ok(done) => Ok(Json(json!({ "ok": true, "dx_call": done.dx_call, "frequency_hz": done.freq_hz, "info": done.info }))),
        Err(e) => {
            use reporting::SpotError;
            let status = match e {
                SpotError::BadDxCall => StatusCode::BAD_REQUEST,
                SpotError::Failed(_) => StatusCode::BAD_GATEWAY,
                SpotError::Off | SpotError::NoOwnCall | SpotError::Already | SpotError::NoFrequency => StatusCode::CONFLICT,
            };
            Err(ApiError::new(status, e.code()).with_message(e.to_string()))
        }
    }
}

#[derive(Deserialize)]
struct QsoUploadTestBody {
    /// `wavelog`, `clublog`, `qrz`, `lotw`, `hrdlog`, `qrzlookup`, `hamqth`, `eqsl`, `pskreporter`, `dxsummit`, `ng3k`, `solar`, `ntp`, `udp`, `udp2` o `tcp`.
    service: String,
}

/// Il pulsante «Prova» dell'invio diretto dei QSO: controlla le impostazioni salvate **senza registrare QSO** (Wavelog/Cloudlog: elenco delle
/// stazioni della chiave; QRZ: `ACTION=STATUS`; LoTW: TQSL e posizione di stazione presenti; ClubLog: solo i dati, perche' non ha una prova
/// senza invio). Se va bene, un servizio fermo per un errore di credenziali riparte. Gli altri servizi esterni hanno una prova leggera
/// ([`service_test`]). Solo dalla rete di casa.
#[utoipa::path(post, path = "/api/v1/qso-upload/test", responses((status = 200, description = "Prova riuscita: `{ok, message}`"), (status = 400, description = "Servizio sconosciuto (bad_service)"), (status = 403, description = "Solo dalla rete di casa (local_only)"), (status = 404, description = "Solo con il motore proprio (upload_unavailable)"), (status = 502, description = "Prova non riuscita (upload_test_failed, con `message`)")))]
async fn qso_upload_test(State(state): State<AppState>, _: Authed, Peer(peer): Peer, headers: HeaderMap, Json(body): Json<QsoUploadTestBody>) -> Result<Json<Value>, ApiError> {
    if !from_home_network(peer, &headers) {
        return Err(ApiError::new(StatusCode::FORBIDDEN, "local_only"));
    }
    // gli altri servizi esterni (eQSL, spot, dati online, orologio, programmi collegati): una prova leggera, senza mandare QSO
    if SERVICE_TESTS.contains(&body.service.as_str()) {
        return match service_test(&state, body.service).await {
            Ok(message) => Ok(Json(json!({ "ok": true, "message": message }))),
            Err(message) => Err(ApiError::new(StatusCode::BAD_GATEWAY, "upload_test_failed").with_message(message)),
        };
    }
    // le ricerche dei nominativi hanno la loro prova: un accesso nuovo con utente e password salvati
    if let Some(provider) = match body.service.as_str() {
        "qrzlookup" => Some(reporting::qrz_xml::Provider::Qrz),
        "hamqth" => Some(reporting::qrz_xml::Provider::HamQth),
        _ => None,
    } {
        let settings = state.engine_settings.clone().ok_or_else(|| ApiError::new(StatusCode::NOT_FOUND, "upload_unavailable"))?;
        let Some((user, password)) = settings.lookup_saved_credentials(provider) else {
            return Err(ApiError::new(StatusCode::BAD_GATEWAY, "upload_test_failed").with_message("servono utente e password salvati"));
        };
        let lookup = match provider {
            reporting::qrz_xml::Provider::Qrz => std::sync::Arc::clone(&settings.lookups().qrz),
            reporting::qrz_xml::Provider::HamQth => std::sync::Arc::clone(&settings.lookups().hamqth),
        };
        return match tokio::task::spawn_blocking(move || lookup.test(&user, &password)).await {
            Ok(Ok(message)) => Ok(Json(json!({ "ok": true, "message": message }))),
            Ok(Err(message)) => Err(ApiError::new(StatusCode::BAD_GATEWAY, "upload_test_failed").with_message(message)),
            Err(_) => Err(ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "upload_task")),
        };
    }
    let service = state.engine_settings.as_ref().and_then(|s| s.reporting()).ok_or_else(|| ApiError::new(StatusCode::NOT_FOUND, "upload_unavailable"))?;
    let target = reporting::upload::Target::parse(&body.service).ok_or_else(|| ApiError::new(StatusCode::BAD_REQUEST, "bad_service"))?;
    match service.test_upload(target).await {
        Ok(message) => Ok(Json(json!({ "ok": true, "message": message }))),
        Err(message) => Err(ApiError::new(StatusCode::BAD_GATEWAY, "upload_test_failed").with_message(message)),
    }
}

/// I servizi con una «Prova» fuori dall'invio dei QSO.
const SERVICE_TESTS: [&str; 9] = ["eqsl", "pskreporter", "dxsummit", "ng3k", "solar", "ntp", "udp", "udp2", "tcp"];

/// Attesa massima di una prova.
const SERVICE_TEST_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(8);

/// La prova di un servizio esterno. Le impostazioni si leggono **prima** (nessun lucchetto durante la rete) e la richiesta, bloccante,
/// gira in un thread a parte: eQSL = accesso con le credenziali salvate (pagina dell'ultimo invio, nessun QSO); PSK Reporter, DXSummit,
/// NG3K e hamqsl.com = una pagina letta e riconosciuta; NTP = una misura dello scarto; UDP = un Heartbeat al server; TCP = la connessione.
async fn service_test(state: &AppState, service: String) -> Result<String, String> {
    use online_info as oi;
    let timeout = SERVICE_TEST_TIMEOUT;
    if service == "ntp" {
        let clock = state.clock.clone().ok_or_else(|| "orologio non disponibile".to_string())?;
        let m = tokio::task::spawn_blocking(move || clock.check_now()).await.map_err(|_| "prova interrotta".to_string())?;
        return match (m.offset_ms, m.server) {
            (Some(offset), Some(server)) => Ok(format!("scarto {offset:+.0} ms da {server} (ritardo {:.0} ms, {}/{} server)", m.delay_ms.unwrap_or_default(), m.servers_ok, m.servers_tried)),
            _ => Err(m.error.filter(|e| !e.is_empty()).unwrap_or_else(|| "nessun server ha risposto".to_string())),
        };
    }
    // i valori salvati, letti adesso e copiati: il lucchetto delle impostazioni non resta preso durante la prova
    let settings = state.engine_settings.clone();
    let values = settings.as_ref().map(|s| s.values()).unwrap_or_default();
    let eqsl = if service == "eqsl" { settings.as_ref().and_then(|s| s.eqsl_saved_credentials()) } else { None };
    let get = move |k: &str| values.get(k).cloned().unwrap_or_default();
    let server = |host: &str, port: &str| crate::engine_settings::parse_server(&get(host), &get(port)).ok_or_else(|| "indirizzo o porta non validi".to_string());
    let target = match service.as_str() {
        "udp" => Some(server("UDPServer", "UDPServerPort")?),
        "udp2" => Some(server("UDP2Server", "UDP2ServerPort")?),
        "tcp" => Some(server("TCPServer", "TCPServerPort")?),
        _ => None,
    };
    if service == "eqsl" && eqsl.is_none() {
        return Err("servono utente e password salvati".to_string());
    }
    let job = move || -> Result<String, String> {
        match service.as_str() {
            "eqsl" => {
                let (user, password, nick) = eqsl.unwrap_or_default();
                reporting::eqsl::test_login(reporting::eqsl::LAST_UPLOAD_URL, &user, &password, &nick, timeout)
            }
            "pskreporter" => {
                // gli spot partono in UDP verso questo nome: deve risolversi; poi il sito deve rispondere
                use std::net::ToSocketAddrs;
                let addr = (reporting::psk::HOST, reporting::psk::PORT).to_socket_addrs().map_err(|e| format!("{}: {e}", reporting::psk::HOST))?.next().ok_or_else(|| format!("{}: indirizzo non trovato", reporting::psk::HOST))?;
                let body = oi::probe(oi::PSK_HOME, timeout, APP_VERSION)?;
                if body.to_lowercase().contains("pskreporter") { Ok(format!("raggiungibile ({} = {})", reporting::psk::HOST, addr.ip())) } else { Err("risposta non riconosciuta da PSK Reporter".to_string()) }
            }
            "dxsummit" => {
                let body = oi::probe(oi::DXSUMMIT_PROBE, timeout, APP_VERSION)?;
                match serde_json::from_str::<Vec<Value>>(&body) {
                    Ok(list) => Ok(match list.first().and_then(|s| s.get("dx_call")).and_then(Value::as_str) {
                        Some(call) => format!("raggiungibile (ultimo spot: {call})"),
                        None => "raggiungibile".to_string(),
                    }),
                    Err(_) => Err("risposta non riconosciuta da DXSummit".to_string()),
                }
            }
            "ng3k" => {
                let items = oi::dxped::parse(&oi::probe(oi::dxped::URL, timeout, APP_VERSION)?);
                if items.is_empty() { Err("risposta non riconosciuta da NG3K".to_string()) } else { Ok(format!("{} DXpedition nel calendario", items.len())) }
            }
            "solar" => {
                let data = oi::solar::parse(&oi::probe(oi::solar::URL, timeout, APP_VERSION)?).ok_or_else(|| "risposta non riconosciuta da hamqsl.com".to_string())?;
                Ok(match data.sfi { Some(sfi) => format!("dati letti (SFI {sfi}, aggiornati {})", data.updated), None => format!("dati letti (aggiornati {})", data.updated) })
            }
            "udp" | "udp2" => {
                let addr = target.ok_or_else(|| "indirizzo o porta non validi".to_string())?;
                let bind: std::net::SocketAddr = if addr.is_ipv4() { ([0, 0, 0, 0], 0).into() } else { (std::net::Ipv6Addr::UNSPECIFIED, 0).into() };
                let socket = std::net::UdpSocket::bind(bind).map_err(|e| format!("socket non aperto: {e}"))?;
                if addr.ip().is_multicast() || addr.ip() == std::net::IpAddr::V4(std::net::Ipv4Addr::BROADCAST) {
                    let _ = socket.set_broadcast(true);
                }
                let packet = udp_bridge::emit::encode_heartbeat("JTDX", udp_bridge::emit::INITIAL_SCHEMA, udp_bridge::emit::MAX_SCHEMA, "2.2.159");
                let sent = socket.send_to(&packet, addr).map_err(|e| format!("pacchetto non partito verso {addr}: {e}"))?;
                Ok(format!("Heartbeat di {sent} byte partito verso {addr}"))
            }
            "tcp" => {
                let addr = target.ok_or_else(|| "indirizzo o porta non validi".to_string())?;
                let stream = std::net::TcpStream::connect_timeout(&addr, timeout).map_err(|e| format!("connessione a {addr} non riuscita: {e}"))?;
                let _ = stream.shutdown(std::net::Shutdown::Both);
                Ok(format!("connessione a {addr} aperta"))
            }
            _ => Err("servizio sconosciuto".to_string()),
        }
    };
    tokio::task::spawn_blocking(job).await.map_err(|_| "prova interrotta".to_string())?
}

#[derive(Deserialize, Default)]
struct WavelogStationsBody {
    /// L'indirizzo scritto nelle Impostazioni (anche non ancora salvato); vuoto = quello salvato.
    #[serde(default)]
    url: Option<String>,
}

/// Le stazioni Wavelog/Cloudlog della chiave salvata, lette dal server: riempiono la tendina della stazione. Solo dalla rete di casa.
#[utoipa::path(post, path = "/api/v1/qso-upload/wavelog-stations", responses((status = 200, description = "`{stations: [{id, name, call}]}`"), (status = 403, description = "Solo dalla rete di casa (local_only)"), (status = 404, description = "Solo con il motore proprio (upload_unavailable)"), (status = 502, description = "Lettura non riuscita (wavelog_stations_failed, con `message`)")))]
async fn wavelog_stations(State(state): State<AppState>, _: Authed, Peer(peer): Peer, headers: HeaderMap, Json(body): Json<WavelogStationsBody>) -> Result<Json<Value>, ApiError> {
    if !from_home_network(peer, &headers) {
        return Err(ApiError::new(StatusCode::FORBIDDEN, "local_only"));
    }
    let service = state.engine_settings.as_ref().and_then(|s| s.reporting()).ok_or_else(|| ApiError::new(StatusCode::NOT_FOUND, "upload_unavailable"))?;
    match service.wavelog_stations(body.url).await {
        Ok(list) => Ok(Json(json!({ "stations": list.iter().map(|s| json!({ "id": s.id, "name": s.name, "call": s.call })).collect::<Vec<_>>() }))),
        Err(message) => Err(ApiError::new(StatusCode::BAD_GATEWAY, "wavelog_stations_failed").with_message(message)),
    }
}

/// Il clic destro sulla barra delle bande: chi fa girare il Band Hopping passa subito alla voce successiva dell'elenco.
pub static BAND_HOP_NEXT: tokio::sync::Notify = tokio::sync::Notify::const_new();

/// Passa subito alla voce successiva dell'elenco del Band Hopping (anche con il giro automatico spento). 409 se l'elenco e' vuoto o si trasmette.
#[utoipa::path(post, path = "/api/v1/commands/band-hop-next", responses((status = 200, description = "Salto chiesto"), (status = 409, description = "Elenco vuoto (band_hop_empty) o in trasmissione (transmitting)"), (status = 404, description = "Solo con il motore proprio (engine_settings_unavailable)")))]
async fn band_hop_next(State(state): State<AppState>, _: Authed) -> Result<Json<Value>, ApiError> {
    let settings = state.engine_settings.as_ref().ok_or_else(|| ApiError::new(StatusCode::NOT_FOUND, "engine_settings_unavailable"))?;
    if settings.reports().band_hop_list.split(',').all(|s| s.trim().is_empty()) {
        return Err(ApiError::new(StatusCode::CONFLICT, "band_hop_empty"));
    }
    if state.current_status().is_some_and(|s| s.transmitting) {
        return Err(ApiError::new(StatusCode::CONFLICT, "transmitting"));
    }
    BAND_HOP_NEXT.notify_one();
    Ok(Json(json!({ "ok": true })))
}

fn audio_control(state: &AppState) -> Result<(std::sync::Arc<dyn crate::audio_api::AudioControl>, std::sync::Arc<crate::engine_settings::EngineSettingsStore>), ApiError> {
    match (state.audio.clone(), state.engine_settings.clone()) {
        (Some(control), Some(store)) => Ok((control, store)),
        _ => Err(ApiError::new(StatusCode::NOT_FOUND, "audio_unavailable")),
    }
}

fn audio_values_json(v: &crate::engine_settings::AudioValues) -> Value {
    json!({ "input": v.input, "output": v.output, "input_channel": v.input_channel, "output_channel": v.output_channel, "save_dir": v.save_dir, "save_mode": v.save_mode })
}

/// Gli ingressi e le uscite del sistema (con i canali e il predefinito), quelli in uso adesso e i valori scelti. Le voci con `name` vuoto o
/// `Default` valgono «predefinito del sistema».
#[utoipa::path(get, path = "/api/v1/audio/devices", responses((status = 200, description = "Dispositivi audio, in uso e valori scelti"), (status = 404, description = "Solo con il motore proprio (audio_unavailable)")))]
async fn audio_devices(State(state): State<AppState>, _: Authed) -> Result<Json<Value>, ApiError> {
    let (control, store) = audio_control(&state)?;
    let devices = tokio::task::spawn_blocking(move || control.devices()).await.map_err(|_| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "audio_task"))?;
    Ok(Json(json!({
        "inputs": devices.inputs,
        "outputs": devices.outputs,
        "input_in_use": devices.input_in_use,
        "output_in_use": devices.output_in_use,
        "output_pending": devices.output_pending,
        "default_save_dir": devices.default_save_dir,
        "system_default": crate::audio_api::SYSTEM_DEFAULT,
        "values": audio_values_json(&store.audio()),
    })))
}

#[derive(Deserialize)]
struct AudioSettingsBody {
    input: Option<String>,
    output: Option<String>,
    input_channel: Option<String>,
    output_channel: Option<String>,
    save_dir: Option<String>,
    save_mode: Option<u8>,
}

/// Cambia la scheda Audio: controlla i valori, **apre i dispositivi nuovi e li applica** (l'ingresso a caldo, l'uscita dalla prossima trasmissione)
/// e solo per le parti riuscite salva nello store. Un dispositivo che non si apre lascia (o rimette) quello di prima. Solo dalla rete di casa.
#[utoipa::path(put, path = "/api/v1/audio/settings", responses((status = 200, description = "Esito per parte (input, output, save) e valori salvati; `ok` e' falso se una parte non e' riuscita"), (status = 400, description = "Valore non ammesso o corpo vuoto (bad_audio_setting)"), (status = 403, description = "Solo dalla rete di casa (local_only)"), (status = 404, description = "Solo con il motore proprio (audio_unavailable)"), (status = 500, description = "Scrittura non riuscita (engine_settings_write)")))]
async fn audio_settings_put(State(state): State<AppState>, _: Authed, Peer(peer): Peer, headers: HeaderMap, Json(body): Json<AudioSettingsBody>) -> Result<Json<Value>, ApiError> {
    use crate::{audio_api as a, engine_settings as es};
    let (control, store) = audio_control(&state)?;
    if !from_home_network(peer, &headers) {
        return Err(ApiError::new(StatusCode::FORBIDDEN, "local_only"));
    }
    let mut asked: std::collections::BTreeMap<String, String> = std::collections::BTreeMap::new();
    for (key, value) in [
        ("SoundInName", body.input),
        ("SoundOutName", body.output),
        ("AudioInputChannel", body.input_channel),
        ("AudioOutputChannel", body.output_channel),
        ("SaveDir", body.save_dir),
        ("SaveWav", body.save_mode.map(|m| m.to_string())),
    ] {
        if let Some(value) = value {
            asked.insert(key.to_string(), value.trim().to_string());
        }
    }
    if asked.is_empty() || asked.iter().any(|(k, v)| es::validate(k, v).is_err()) {
        return Err(ApiError::new(StatusCode::BAD_REQUEST, "bad_audio_setting"));
    }
    let result = tokio::task::spawn_blocking(move || {
        let before = store.audio();
        let mut wanted = store.values();
        wanted.extend(asked);
        let get = |k: &str| wanted.get(k).cloned().unwrap_or_default();
        let desired = es::AudioValues { input: get("SoundInName"), output: get("SoundOutName"), input_channel: get("AudioInputChannel"), output_channel: get("AudioOutputChannel"), save_dir: get("SaveDir"), save_mode: get("SaveWav") };
        let outcome = control.apply(&desired);
        // si salvano solo le parti riuscite; chi e' fallito resta come nel file
        let mut changes = std::collections::BTreeMap::new();
        for (part, keys) in [(&outcome.input, a::keys_of_input()), (&outcome.output, a::keys_of_output()), (&outcome.save, a::keys_of_save())] {
            if part.is_failed() {
                continue;
            }
            for key in keys {
                let value = get(key);
                if store.values().get(key).map(String::as_str).unwrap_or("") != value {
                    changes.insert(key.to_string(), value);
                }
            }
        }
        let saved = if changes.is_empty() { Ok(()) } else { store.update_audio(&changes) };
        if saved.is_err() {
            // il file non si e' scritto: i dispositivi tornano com'erano, cosi' il programma e il file dicono la stessa cosa
            let _ = control.apply(&before);
        }
        (outcome, saved.map(|()| store.audio()))
    })
    .await
    .map_err(|_| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "audio_task"))?;
    match result {
        (outcome, Ok(values)) => Ok(Json(json!({
            "ok": !(outcome.input.is_failed() || outcome.output.is_failed() || outcome.save.is_failed()),
            "input": outcome.input,
            "output": outcome.output,
            "save": outcome.save,
            "values": audio_values_json(&values),
        }))),
        (_, Err(crate::engine_settings::SettingsError::Io(e))) => {
            tracing::warn!("impostazioni audio non salvate: {e}");
            Err(ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "engine_settings_write"))
        }
        (_, Err(_)) => Err(ApiError::new(StatusCode::BAD_REQUEST, "bad_audio_setting")),
    }
}

#[utoipa::path(get, path = "/api/v1/ui-settings/{kind}", responses((status = 200, description = "Preferenze dell'interfaccia per tipo di dispositivo (desktop o mobile)"), (status = 400, description = "Tipo non valido (bad_kind)")))]
async fn ui_settings_get(State(state): State<AppState>, _: Authed, axum::extract::Path(kind): axum::extract::Path<String>) -> Result<Json<Value>, ApiError> {
    if !crate::ui_settings::valid_kind(&kind) {
        return Err(ApiError::new(StatusCode::BAD_REQUEST, "bad_kind"));
    }
    let entries = state.ui_settings.as_ref().map(|s| s.get(&kind)).unwrap_or_default();
    Ok(Json(json!(entries)))
}

#[utoipa::path(put, path = "/api/v1/ui-settings/{kind}", responses((status = 200, description = "Preferenze salvate"), (status = 400, description = "Tipo o contenuto non valido (bad_kind, bad_settings)"), (status = 404, description = "Salvataggio non disponibile (ui_settings_unavailable)"), (status = 500, description = "Scrittura non riuscita (ui_settings_write)")))]
async fn ui_settings_put(
    State(state): State<AppState>,
    _: Authed,
    axum::extract::Path(kind): axum::extract::Path<String>,
    Json(entries): Json<crate::ui_settings::Entries>,
) -> Result<Json<Value>, ApiError> {
    if !crate::ui_settings::valid_kind(&kind) {
        return Err(ApiError::new(StatusCode::BAD_REQUEST, "bad_kind"));
    }
    let size: usize = entries.iter().map(|(k, v)| k.len() + v.len()).sum();
    if !crate::ui_settings::valid_entries(&entries) || size > crate::ui_settings::MAX_BYTES {
        return Err(ApiError::new(StatusCode::BAD_REQUEST, "bad_settings"));
    }
    let store = state.ui_settings.clone().ok_or_else(|| ApiError::new(StatusCode::NOT_FOUND, "ui_settings_unavailable"))?;
    // i filtri di Attivita' di banda del PC valgono anche per le Automazioni
    if kind == "desktop"
        && let Some(filters) = entries.get(aurora_core::UI_FILTERS_KEY).and_then(|text| serde_json::from_str::<aurora_core::UdpFilters>(text).ok())
    {
        *state.band_filters.write().unwrap_or_else(|p| p.into_inner()) = filters;
    }
    tokio::task::spawn_blocking(move || store.put(&kind, entries))
        .await
        .map_err(|_| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "ui_settings_task"))?
        .map_err(|e| {
            tracing::warn!("salvataggio delle preferenze fallito: {e}");
            ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "ui_settings_write")
        })?;
    Ok(Json(json!({ "ok": true })))
}

/// I filtri di Attivita' di banda che Aurora conosce adesso (quelli che seguono le Automazioni).
#[utoipa::path(get, path = "/api/v1/band-filters", responses((status = 200, description = "Filtri di Attivita' di banda (nomi dei campi come nella pagina)")))]
async fn band_filters_get(State(state): State<AppState>, _: Authed) -> Json<Value> {
    let filters = state.band_filters.read().unwrap_or_else(|p| p.into_inner()).clone();
    Json(serde_json::to_value(filters).unwrap_or(Value::Null))
}

/// La pagina manda i filtri di Attivita' di banda quando cambiano: le Automazioni non rispondono mai alle stazioni che nascondono.
/// Restano in memoria; al riavvio Aurora li rilegge dalle preferenze dell'interfaccia salvate.
#[utoipa::path(put, path = "/api/v1/band-filters", responses((status = 200, description = "Filtri applicati"), (status = 400, description = "Elenchi troppo lunghi (bad_filters)")))]
async fn band_filters_put(State(state): State<AppState>, _: Authed, Json(filters): Json<aurora_core::UdpFilters>) -> Result<Json<Value>, ApiError> {
    let lists = [&filters.hide_continents, &filters.hide_countries, &filters.hide_calls, &filters.show_only_calls, &filters.always_show_calls];
    if lists.iter().any(|l| l.len() > 1000) {
        return Err(ApiError::new(StatusCode::BAD_REQUEST, "bad_filters"));
    }
    *state.band_filters.write().unwrap_or_else(|p| p.into_inner()) = filters;
    Ok(Json(json!({ "ok": true })))
}

fn update_service(state: &AppState) -> Result<std::sync::Arc<updater::UpdateService>, ApiError> {
    state.updates.clone().ok_or_else(|| ApiError::new(StatusCode::NOT_FOUND, "update_unavailable"))
}

/// Cosa offre la diagnostica da qui: `crash_folder` vero se si puo' aprire la cartella dei crash (programma Windows, richiesta dal PC stesso).
#[utoipa::path(get, path = "/api/v1/diagnostics", responses((status = 200, description = "{crash_folder}")))]
async fn diagnostics_get(State(state): State<AppState>, _: Authed, Peer(peer): Peer, headers: HeaderMap) -> Json<Value> {
    let here = from_this_pc(peer, &headers);
    Json(json!({ "crash_folder": state.crash_folder.is_some() && here, "support_bundle": state.support_bundle.is_some() && here }))
}

#[derive(Deserialize, Default)]
struct SupportBody {
    #[serde(default)]
    lang: String,
    #[serde(default)]
    local_offset_min: i32,
}

/// Parti di nome di campo che possono nascondere un segreto: il loro valore diventa «***».
const SECRET_WORDS: [&str; 9] = ["password", "passwd", "key", "token", "secret", "code", "pin", "credential", "seed"];

/// Vero se il nome del campo (o della variabile d'ambiente) fa pensare a un segreto.
pub fn is_secret_name(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    SECRET_WORDS.iter().any(|w| lower.contains(w))
}

/// Sostituisce con «***» i valori dei campi segreti, a ogni livello.
pub fn redact_json(value: &mut Value) {
    match value {
        Value::Object(map) => {
            for (k, v) in map.iter_mut() {
                if is_secret_name(k) && !v.is_null() && v.as_str() != Some("") {
                    *v = json!("***");
                } else {
                    redact_json(v);
                }
            }
        }
        Value::Array(items) => items.iter_mut().for_each(redact_json),
        _ => {}
    }
}

/// Lo stato del momento per il pacchetto. Radio, audio e controlli possono rispondere lenti: si chiama fuori dal ciclo di rete.
fn support_live(state: &AppState) -> Value {
    let status = state.current_status();
    let mut live = json!({
        "station": *state.station_info,
        "tx_allowed_policy": state.policy.live_tx(),
        "status": status.map(|s| json!({ "dial_hz": s.dial_hz, "mode": s.mode, "tx_enabled": s.tx_enabled, "transmitting": s.transmitting, "decoding": s.decoding })),
        "cat": state.cat.as_ref().map(|c| json!(c.status())),
        "controls": state.rig.as_ref().map(|r| match r.controls_and_tx() {
            Ok(reading) => json!(format!("{reading:?}")),
            Err(e) => json!(format!("errore: {e}")),
        }),
        "audio": state.audio.as_ref().map(|a| json!(a.devices())),
        "clock": state.clock.as_ref().map(|c| json!({ "last": c.last(), "last_good_ms": c.last_good_ms() })),
        "cluster": state.cluster.as_ref().map(|c| json!({ "config": c.config(), "servers": c.statuses() })),
        "lan": { "port": state.lan.port(), "mdns_host": state.lan.host() },
        "paired_devices": state.auth.list().map(|l| l.len()).ok(),
        "open_access": state.open_access.load(std::sync::atomic::Ordering::Relaxed),
        "local_trust": state.local_trust,
        "native_engine": state.native_status.read().ok().and_then(|f| f.as_ref().map(|f| f())),
    });
    redact_json(&mut live);
    live
}

/// Prepara sul Desktop lo zip per l'assistenza (log, crash, impostazioni senza segreti, info.txt) e lo mostra in Esplora file.
/// Lo zip di una copia delle impostazioni non supera questo (log di molti QSO compreso).
const SETTINGS_RESTORE_MAX_BYTES: usize = 200 * 1024 * 1024;

/// Crea la copia delle impostazioni sul Desktop e la mostra (solo dal PC stesso).
#[utoipa::path(post, path = "/api/v1/settings-backup", responses((status = 200, description = "{ok, path, name, files}"), (status = 403, description = "Solo dal PC (local_only)"), (status = 404, description = "Non disponibile (backup_unavailable)"), (status = 500, description = "Copia non riuscita (backup_failed)")))]
async fn settings_backup(State(state): State<AppState>, _: Authed, Peer(peer): Peer, headers: HeaderMap) -> Result<Json<Value>, ApiError> {
    if !from_this_pc(peer, &headers) {
        return Err(ApiError::new(StatusCode::FORBIDDEN, "local_only"));
    }
    let make = state.settings_backup.clone().ok_or_else(|| ApiError::new(StatusCode::NOT_FOUND, "backup_unavailable"))?;
    let (path, files) = tokio::task::spawn_blocking(move || {
        let made = make()?;
        if let Err(e) = reveal_in_explorer(&made.0) {
            tracing::warn!("copia delle impostazioni non mostrata: {e}");
        }
        Ok::<_, String>(made)
    })
    .await
    .map_err(|_| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "backup_failed"))?
    .map_err(|e| {
        tracing::warn!("copia delle impostazioni non riuscita: {e}");
        ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "backup_failed").with_message(e)
    })?;
    tracing::info!("copia delle impostazioni: {} ({files} file)", path.display());
    let name = path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
    Ok(Json(json!({ "ok": true, "path": path.display().to_string(), "name": name, "files": files })))
}

/// Ripristina da una copia delle impostazioni: i file si applicano al prossimo avvio di Aurora (solo dal PC stesso).
#[utoipa::path(post, path = "/api/v1/settings-restore", responses((status = 200, description = "{ok, files}: si applicano riavviando Aurora"), (status = 400, description = "Non e' una copia di Aurora (not_a_backup)"), (status = 403, description = "Solo dal PC (local_only)"), (status = 404, description = "Non disponibile (backup_unavailable)")))]
async fn settings_restore(State(state): State<AppState>, _: Authed, Peer(peer): Peer, headers: HeaderMap, body: axum::body::Bytes) -> Result<Json<Value>, ApiError> {
    if !from_this_pc(peer, &headers) {
        return Err(ApiError::new(StatusCode::FORBIDDEN, "local_only"));
    }
    let restore = state.settings_restore.clone().ok_or_else(|| ApiError::new(StatusCode::NOT_FOUND, "backup_unavailable"))?;
    let files = tokio::task::spawn_blocking(move || restore(body.to_vec()))
        .await
        .map_err(|_| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "restore_failed"))?
        .map_err(|e| if e == "not_a_backup" { ApiError::new(StatusCode::BAD_REQUEST, "not_a_backup") } else { ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "restore_failed").with_message(e) })?;
    tracing::info!("ripristino pronto: {files} file al prossimo avvio");
    Ok(Json(json!({ "ok": true, "files": files })))
}

#[utoipa::path(post, path = "/api/v1/diagnostics/support-bundle", responses((status = 200, description = "{ok, name, path, version, call, radio, ptt}"), (status = 403, description = "Solo dal PC (local_only)"), (status = 404, description = "Solo nel programma Windows (support_unavailable)"), (status = 500, description = "Pacchetto non creato (bundle_failed)")))]
async fn diagnostics_support_bundle(State(state): State<AppState>, _: Authed, Peer(peer): Peer, headers: HeaderMap, body: Option<Json<SupportBody>>) -> Result<Json<Value>, ApiError> {
    if !from_this_pc(peer, &headers) {
        return Err(ApiError::new(StatusCode::FORBIDDEN, "local_only"));
    }
    let build = state.support_bundle.clone().ok_or_else(|| ApiError::new(StatusCode::NOT_FOUND, "support_unavailable"))?;
    let body = body.map(|Json(b)| b).unwrap_or_default();
    let worker = state.clone();
    let made = tokio::task::spawn_blocking(move || {
        let request = crate::SupportRequest { live: support_live(&worker), lang: body.lang, local_offset_min: body.local_offset_min.clamp(-14 * 60, 14 * 60) };
        let made = build(request)?;
        if let Err(e) = reveal_in_explorer(&made.path) {
            tracing::warn!("pacchetto per l'assistenza non mostrato: {e}");
        }
        Ok::<_, String>(made)
    })
    .await
    .map_err(|_| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "bundle_failed"))?
    .map_err(|e| {
        tracing::warn!("pacchetto per l'assistenza non creato: {e}");
        ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "bundle_failed")
    })?;
    tracing::info!("pacchetto per l'assistenza creato: {}", made.path.display());
    let name = made.path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
    let path = made.path.display().to_string();
    // quanto basta per il messaggio di posta: versione, nominativo, radio e PTT
    let call = state.current_status().map(|s| s.de_call).unwrap_or_default();
    let radio = state.cat.as_ref().map(|c| c.config().rig_name).unwrap_or_default();
    let version = state.station_info["version"].as_str().unwrap_or_default().to_string();
    if let Ok(mut last) = state.last_bundle.lock() {
        *last = Some(made.path);
    }
    Ok(Json(json!({ "ok": true, "name": name, "path": path, "version": version, "call": call, "radio": radio, "ptt": made.ptt })))
}

/// Riapre Esplora file sull'ultimo pacchetto creato («Apri la cartella»).
#[utoipa::path(post, path = "/api/v1/diagnostics/support-bundle/reveal", responses((status = 200, description = "{ok}"), (status = 403, description = "Solo dal PC (local_only)"), (status = 404, description = "Nessun pacchetto (no_bundle)"), (status = 500, description = "Cartella non aperta (open_failed)")))]
async fn diagnostics_support_reveal(State(state): State<AppState>, _: Authed, Peer(peer): Peer, headers: HeaderMap) -> Result<Json<Value>, ApiError> {
    if !from_this_pc(peer, &headers) {
        return Err(ApiError::new(StatusCode::FORBIDDEN, "local_only"));
    }
    let path = state.last_bundle.lock().ok().and_then(|l| l.clone()).ok_or_else(|| ApiError::new(StatusCode::NOT_FOUND, "no_bundle"))?;
    match tokio::task::spawn_blocking(move || reveal_in_explorer(&path)).await {
        Ok(Ok(())) => Ok(Json(json!({ "ok": true }))),
        _ => Err(ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "open_failed")),
    }
}

/// I rapporti degli errori: solo dal PC stesso e solo dove il programma li sa fare.
fn error_reports(state: &AppState, peer: Option<SocketAddr>, headers: &HeaderMap) -> Result<crate::ErrorReportsHandle, ApiError> {
    if !from_this_pc(peer, headers) {
        return Err(ApiError::new(StatusCode::FORBIDDEN, "local_only"));
    }
    state.error_reports.clone().ok_or_else(|| ApiError::new(StatusCode::NOT_FOUND, "reports_unavailable"))
}

/// Una chiamata bloccante dei rapporti fuori dal ciclo asincrono.
async fn reports_blocking(work: impl FnOnce() -> Result<Value, String> + Send + 'static) -> Result<Json<Value>, ApiError> {
    tokio::task::spawn_blocking(work)
        .await
        .map_err(|_| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "reports_failed"))?
        .map(Json)
        .map_err(|e| {
            tracing::warn!("rapporti degli errori: {e}");
            ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "reports_failed")
        })
}

/// Impostazioni dei rapporti degli errori, identificativo dell'installazione, coda e crash da segnalare.
#[utoipa::path(get, path = "/api/v1/reports", responses((status = 200, description = "{ask_after_crash, auto_errors, install_id, configured, crash_pending, queue, sample}"), (status = 403, description = "Solo dal PC (local_only)")))]
async fn reports_get(State(state): State<AppState>, _: Authed, Peer(peer): Peer, headers: HeaderMap) -> Result<Json<Value>, ApiError> {
    let reports = error_reports(&state, peer, &headers)?;
    reports_blocking(move || Ok(reports.state())).await
}

/// Accende o spegne la domanda dopo un crash (`ask_after_crash`) e gli errori automatici (`auto_errors`).
#[utoipa::path(put, path = "/api/v1/reports", responses((status = 200, description = "Lo stato aggiornato"), (status = 403, description = "Solo dal PC (local_only)")))]
async fn reports_put(State(state): State<AppState>, _: Authed, Peer(peer): Peer, headers: HeaderMap, Json(body): Json<Value>) -> Result<Json<Value>, ApiError> {
    let reports = error_reports(&state, peer, &headers)?;
    reports_blocking(move || reports.update(&body)).await
}

/// Un identificativo dell'installazione nuovo.
#[utoipa::path(post, path = "/api/v1/reports/install-id", responses((status = 200, description = "Lo stato con il nuovo identificativo"), (status = 403, description = "Solo dal PC (local_only)")))]
async fn reports_new_id(State(state): State<AppState>, _: Authed, Peer(peer): Peer, headers: HeaderMap) -> Result<Json<Value>, ApiError> {
    let reports = error_reports(&state, peer, &headers)?;
    reports_blocking(move || Ok(reports.new_install_id())).await
}

/// «Vedi cosa mando»: i file del rapporto del crash, gia' ripuliti, con il loro testo.
#[utoipa::path(post, path = "/api/v1/reports/crash/preview", responses((status = 200, description = "{files: [{name, bytes, text}], bytes}"), (status = 403, description = "Solo dal PC (local_only)")))]
async fn reports_crash_preview(State(state): State<AppState>, _: Authed, Peer(peer): Peer, headers: HeaderMap) -> Result<Json<Value>, ApiError> {
    let reports = error_reports(&state, peer, &headers)?;
    reports_blocking(move || reports.crash_preview()).await
}

/// «Manda il rapporto»: lo zip senza dati personali va a chi sviluppa Aurora.
#[utoipa::path(post, path = "/api/v1/reports/crash/send", responses((status = 200, description = "{ok}"), (status = 403, description = "Solo dal PC (local_only)"), (status = 500, description = "Invio non riuscito (reports_failed)")))]
async fn reports_crash_send(State(state): State<AppState>, _: Authed, Peer(peer): Peer, headers: HeaderMap) -> Result<Json<Value>, ApiError> {
    let reports = error_reports(&state, peer, &headers)?;
    reports_blocking(move || reports.crash_send()).await
}

/// «Manda il pacchetto a chi sviluppa Aurora»: lo zip senza dati personali e un codice breve da comunicare a chi aiuta.
#[utoipa::path(post, path = "/api/v1/reports/bundle/send", responses((status = 200, description = "{ok, code, bytes}"), (status = 403, description = "Solo dal PC (local_only)"), (status = 500, description = "Invio non riuscito (reports_failed)")))]
async fn reports_bundle_send(State(state): State<AppState>, _: Authed, Peer(peer): Peer, headers: HeaderMap) -> Result<Json<Value>, ApiError> {
    let reports = error_reports(&state, peer, &headers)?;
    reports_blocking(move || reports.bundle_send()).await
}

/// «No»: il crash resta sul PC e il riquadro non torna.
#[utoipa::path(post, path = "/api/v1/reports/crash/dismiss", responses((status = 200, description = "{ok}"), (status = 403, description = "Solo dal PC (local_only)")))]
async fn reports_crash_dismiss(State(state): State<AppState>, _: Authed, Peer(peer): Peer, headers: HeaderMap) -> Result<Json<Value>, ApiError> {
    let reports = error_reports(&state, peer, &headers)?;
    reports_blocking(move || {
        reports.crash_dismiss();
        Ok(json!({ "ok": true }))
    })
    .await
}

/// Apre Esplora file con il file selezionato (`explorer.exe /select,<percorso>`).
fn reveal_in_explorer(path: &std::path::Path) -> Result<(), String> {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt as _;
        let mut command = std::process::Command::new("explorer.exe");
        // il percorso tra virgolette dentro l'argomento, come lo vuole Explorer: gli spazi del nome utente non lo spezzano
        command.raw_arg(format!("/select,\"{}\"", path.display()));
        // CREATE_NO_WINDOW: nessuna console di passaggio
        command.creation_flags(0x0800_0000);
        command.spawn().map(|_| ()).map_err(|e| e.to_string())
    }
    #[cfg(not(windows))]
    {
        let _ = path;
        Err("solo su Windows".to_string())
    }
}

/// Apre in Esplora risorse la cartella dei crash (minidump e rapporti dei blocchi, delle eccezioni e dei panic).
#[utoipa::path(post, path = "/api/v1/diagnostics/crash-folder", responses((status = 200, description = "{ok}"), (status = 403, description = "Solo dal PC (local_only)"), (status = 404, description = "Solo nel programma Windows (crash_folder_unavailable)"), (status = 500, description = "Cartella non aperta (open_failed)")))]
async fn diagnostics_crash_folder(State(state): State<AppState>, _: Authed, Peer(peer): Peer, headers: HeaderMap) -> Result<Json<Value>, ApiError> {
    if !from_this_pc(peer, &headers) {
        return Err(ApiError::new(StatusCode::FORBIDDEN, "local_only"));
    }
    let open = state.crash_folder.clone().ok_or_else(|| ApiError::new(StatusCode::NOT_FOUND, "crash_folder_unavailable"))?;
    match tokio::task::spawn_blocking(move || open()).await {
        Ok(Ok(())) => Ok(Json(json!({ "ok": true }))),
        Ok(Err(e)) => {
            tracing::warn!("cartella dei crash non aperta: {e}");
            Err(ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "open_failed"))
        }
        Err(_) => Err(ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "open_failed")),
    }
}

/// Versione in uso, impostazioni del controllo e ultima release trovata (il token non si restituisce mai).
#[utoipa::path(get, path = "/api/v1/update", responses((status = 200, description = "Stato degli aggiornamenti"), (status = 404, description = "Non disponibile (update_unavailable)")))]
async fn update_get(State(state): State<AppState>, _: Authed) -> Result<Json<Value>, ApiError> {
    let status = update_service(&state)?.status();
    Ok(Json(update_json(&state, &status)))
}

/// Lo stato degli aggiornamenti con `can_install`: vero se questo programma sa installare e la release nuova ha pacchetto e firma.
fn update_json(state: &AppState, status: &updater::Status) -> Value {
    let can_install = state.update_installer.is_some() && status.available.as_ref().is_some_and(|r| updater::installer_assets(r).is_some());
    let mut value = json!(status);
    value["can_install"] = json!(can_install);
    value
}

#[derive(Deserialize)]
struct UpdateBody {
    enabled: bool,
    repo: String,
    interval_hours: u32,
    #[serde(default)]
    prereleases: bool,
    /// Token di sola lettura per un repository privato: assente = si lascia quello che c'e', vuoto = si toglie.
    token: Option<String>,
}

/// Cambia le impostazioni del controllo degli aggiornamenti.
#[utoipa::path(put, path = "/api/v1/update", responses((status = 200, description = "Salvato"), (status = 400, description = "Valore non valido (bad_update)"), (status = 500, description = "Scrittura fallita (update_write)")))]
async fn update_put(State(state): State<AppState>, _: Authed, Json(body): Json<UpdateBody>) -> Result<Json<Value>, ApiError> {
    let service = update_service(&state)?;
    let config = updater::UpdateConfig { enabled: body.enabled, repo: body.repo, interval_hours: body.interval_hours, prereleases: body.prereleases, token: String::new() };
    let mut check = config.clone();
    check.token = body.token.clone().unwrap_or_default();
    check.validate().map_err(|_| ApiError::new(StatusCode::BAD_REQUEST, "bad_update"))?;
    service.set_config(config, body.token).map_err(|e| {
        tracing::warn!("salvataggio delle impostazioni degli aggiornamenti fallito: {e}");
        ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "update_write")
    })?;
    Ok(Json(json!(service.status())))
}

/// Controlla subito se c'e' una versione nuova (serve internet).
#[utoipa::path(post, path = "/api/v1/update/check", responses((status = 200, description = "Esito del controllo"), (status = 404, description = "Non disponibile (update_unavailable)")))]
async fn update_check(State(state): State<AppState>, _: Authed) -> Result<Json<Value>, ApiError> {
    let service = update_service(&state)?;
    let status = tokio::task::spawn_blocking(move || service.check()).await.map_err(|_| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "update_task"))?;
    Ok(Json(update_json(&state, &status)))
}

/// Installa la versione nuova: la scarica, ne controlla la firma, avvia l'installazione e chiude Aurora, che riparte da sola.
/// Solo dal PC stesso e mai mentre si trasmette.
#[utoipa::path(post, path = "/api/v1/update/install", responses((status = 200, description = "Installazione avviata (`version`)"), (status = 403, description = "Solo dal PC (local_only)"), (status = 404, description = "Questo programma non sa installare (install_unavailable)"), (status = 409, description = "Si sta trasmettendo (transmitting)"), (status = 502, description = "Scaricamento o firma non riusciti (install_failed)")))]
async fn update_install(State(state): State<AppState>, _: Authed, Peer(peer): Peer, headers: HeaderMap) -> Result<Json<Value>, ApiError> {
    let installer = state.update_installer.clone().ok_or_else(|| ApiError::new(StatusCode::NOT_FOUND, "install_unavailable"))?;
    if !peer.is_some_and(|p| p.ip().is_loopback()) || is_proxied(&headers) {
        return Err(ApiError::new(StatusCode::FORBIDDEN, "local_only"));
    }
    if state.current_status().is_some_and(|s| s.transmitting) {
        return Err(ApiError::new(StatusCode::CONFLICT, "transmitting"));
    }
    let updates = update_service(&state)?;
    match tokio::task::spawn_blocking(move || installer.install(updates)).await {
        Ok(Ok(version)) => Ok(Json(json!({ "ok": true, "version": version }))),
        Ok(Err(e)) => {
            // errore (non avviso): cosi' arriva anche nei rapporti, e la pagina mostra il motivo invece del solo codice
            tracing::error!("installazione dell'aggiornamento non riuscita: {e}");
            Err(ApiError::new(StatusCode::BAD_GATEWAY, "install_failed").with_message(e))
        }
        Err(_) => Err(ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "update_task")),
    }
}

/// Scarica in anticipo la versione nuova e ne controlla la firma, senza installarla (il «Riprova» dopo uno scaricamento non riuscito).
/// Risponde subito: l'avanzamento si legge in `stage` di `GET /api/v1/update`.
#[utoipa::path(post, path = "/api/v1/update/download", responses((status = 200, description = "Scaricamento avviato: stato degli aggiornamenti"), (status = 404, description = "Questo programma non sa installare (install_unavailable)")))]
async fn update_download(State(state): State<AppState>, _: Authed) -> Result<Json<Value>, ApiError> {
    let installer = state.update_installer.clone().ok_or_else(|| ApiError::new(StatusCode::NOT_FOUND, "install_unavailable"))?;
    let updates = update_service(&state)?;
    // la pagina che rilegge lo stato vede subito «downloading», anche prima che parta il thread
    if let Some(release) = updates.available() {
        updates.begin(&release.version);
    }
    let service = std::sync::Arc::clone(&updates);
    tokio::task::spawn_blocking(move || {
        if let Err(e) = installer.prepare(service) {
            tracing::warn!("scaricamento dell'aggiornamento non riuscito: {e}");
        }
    });
    Ok(Json(update_json(&state, &updates.status())))
}

fn wishlist_service(state: &AppState) -> Result<std::sync::Arc<wishlist::WishlistService>, ApiError> {
    state.wishlist.clone().ok_or_else(|| ApiError::new(StatusCode::NOT_FOUND, "wishlist_unavailable"))
}

/// La lista dei desiderati e gli ultimi avvisi.
#[utoipa::path(get, path = "/api/v1/wishlist", responses((status = 200, description = "Lista dei desiderati e avvisi recenti"), (status = 404, description = "Non disponibile (wishlist_unavailable)")))]
async fn wishlist_get(State(state): State<AppState>, _: Authed) -> Result<Json<Value>, ApiError> {
    let service = wishlist_service(&state)?;
    Ok(Json(json!({ "config": service.config(), "recent": service.recent() })))
}

/// Cambia la lista dei desiderati: si salva e vale subito.
#[utoipa::path(put, path = "/api/v1/wishlist", responses((status = 200, description = "Lista salvata"), (status = 400, description = "Voce non valida (bad_wishlist)"), (status = 500, description = "Scrittura su disco fallita (wishlist_write)")))]
async fn wishlist_put(State(state): State<AppState>, _: Authed, Json(config): Json<wishlist::Wishlist>) -> Result<Json<Value>, ApiError> {
    let service = wishlist_service(&state)?;
    let config = config.normalized();
    config.validate().map_err(|e| {
        tracing::debug!("lista dei desiderati rifiutata: {e}");
        ApiError::new(StatusCode::BAD_REQUEST, "bad_wishlist")
    })?;
    let saved = service.set_config(config).map_err(|e| {
        tracing::warn!("salvataggio della lista dei desiderati fallito: {e}");
        ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "wishlist_write")
    })?;
    Ok(Json(json!({ "ok": true, "config": saved })))
}

/// Manda un avviso di prova: compare nella pagina e, se c'e' un indirizzo, arriva al telefono. Dice se il telefono l'ha ricevuto.
#[utoipa::path(post, path = "/api/v1/wishlist/test", responses((status = 200, description = "Avviso di prova inviato; `phone` dice l'esito verso il telefono"), (status = 404, description = "Non disponibile (wishlist_unavailable)")))]
async fn wishlist_test(State(state): State<AppState>, _: Authed) -> Result<Json<Value>, ApiError> {
    let service = wishlist_service(&state)?;
    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_millis() as u64);
    let alert = aurora_core::WantedAlert {
        call: "TEST".into(),
        reason: "call".into(),
        source: "test".into(),
        text: "Avviso di prova di Aurora".into(),
        freq_hz: None,
        band: None,
        snr: None,
        country: None,
        grid: None,
        time_ms: now,
    };
    service.remember(alert.clone());
    let _ = state.bus.events().send(aurora_core::Event::Wanted(alert.clone()));
    let url = service.config().notify_url;
    if url.is_empty() {
        return Ok(Json(json!({ "ok": true, "phone": "no_url" })));
    }
    let result = tokio::task::spawn_blocking(move || wishlist::send_notification(&url, &wishlist::alert_title(&alert), &wishlist::alert_body(&alert)))
        .await
        .map_err(|_| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "wishlist_task"))?;
    match result {
        Ok(()) => Ok(Json(json!({ "ok": true, "phone": "sent" }))),
        Err(e) => {
            tracing::warn!("avviso di prova al telefono non inviato: {e}");
            Ok(Json(json!({ "ok": true, "phone": "failed" })))
        }
    }
}

/// I quadrati locatore gia' lavorati, con il numero di QSO e le bande, per la mappa.
#[utoipa::path(get, path = "/api/v1/map/worked", responses((status = 200, description = "Quadrati lavorati: totale e [{grid, count, bands}]"), (status = 404, description = "Log non configurato (map_unavailable)"), (status = 502, description = "File non leggibile (map_read)")))]
async fn map_worked(State(state): State<AppState>, _: Authed) -> Result<Json<Value>, ApiError> {
    let read = state.worked_map.clone().ok_or_else(|| ApiError::new(StatusCode::NOT_FOUND, "map_unavailable"))?;
    tokio::task::spawn_blocking(move || read())
        .await
        .map_err(|_| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "map_task"))?
        .map(Json)
        .map_err(|_| ApiError::new(StatusCode::BAD_GATEWAY, "map_read"))
}

/// Il file ADIF di un log lungo anni sta in qualche megabyte: il limite ferma solo gli errori.
// i log di una vita (70-100 mila QSO esportati da altri programmi con tutti i campi) superano i 64 MB
const LOG_IMPORT_MAX_BYTES: usize = 256 * 1024 * 1024;

#[derive(Deserialize)]
struct MatrixParams {
    call: Option<String>,
    grid: Option<String>,
    band: Option<String>,
    mode: Option<String>,
    compact: Option<String>,
}

/// La scheda DXCC: paesi lavorati per banda e modo con i contatori e, se si indica `call`, le etichette NEW ONE / NEW BAND / NEW MODE /
/// NEW GRID / GRID BAND / GRID MODE per quella stazione sulla banda e nel modo dati (`band`, `mode`, `grid` facoltativi); con `compact=1` senza
/// l'elenco dei paesi.
#[utoipa::path(get, path = "/api/v1/dxcc-matrix", responses((status = 200, description = "Matrice DXCC e, con `call`, `subject` con le etichette"), (status = 404, description = "Paesi o log non disponibili (dxcc_matrix_unavailable)"), (status = 502, description = "Log non leggibile (dxcc_matrix_read)")))]
async fn dxcc_matrix(State(state): State<AppState>, _: Authed, axum::extract::Query(p): axum::extract::Query<MatrixParams>) -> Result<Json<Value>, ApiError> {
    let compute = state.dxcc_matrix.clone().ok_or_else(|| ApiError::new(StatusCode::NOT_FOUND, "dxcc_matrix_unavailable"))?;
    let clean = |v: Option<String>| v.map(|s| s.trim().to_string()).filter(|s| !s.is_empty());
    let query = crate::MatrixQuery { call: clean(p.call), grid: clean(p.grid), band: clean(p.band), mode: clean(p.mode), compact: matches!(p.compact.as_deref(), Some("1" | "true")) };
    // legge e analizza il log: fuori dal thread asincrono
    tokio::task::spawn_blocking(move || compute(query))
        .await
        .map_err(|_| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "dxcc_matrix_task"))?
        .map(Json)
        .map_err(|e| {
            tracing::warn!("scheda DXCC non calcolata: {e}");
            ApiError::new(StatusCode::BAD_GATEWAY, "dxcc_matrix_read")
        })
}

#[derive(Deserialize)]
struct ExportParams {
    from: Option<String>,
    to: Option<String>,
    ids: Option<String>,
}

/// `AAAAMMGG` o `AAAA-MM-GG` -> `AAAAMMGG`; vuoto = nessun limite; altro = `None` (data non valida).
fn adif_date(v: Option<&str>) -> Option<Option<String>> {
    let v = v.map(str::trim).filter(|v| !v.is_empty());
    let Some(v) = v else { return Some(None) };
    let digits: String = v.chars().filter(|c| *c != '-').collect();
    (digits.len() == 8 && aurora_core::adif::unix_ms(&digits, "").is_some()).then_some(Some(digits))
}

/// Le date `from` e `to` dell'esportazione, controllate: valide e non rovesciate (`bad_range`).
fn export_range(from: Option<&str>, to: Option<&str>) -> Result<(Option<String>, Option<String>), ApiError> {
    let (Some(from), Some(to)) = (adif_date(from), adif_date(to)) else {
        return Err(ApiError::new(StatusCode::BAD_REQUEST, "bad_range"));
    };
    if let (Some(f), Some(t)) = (&from, &to)
        && f > t
    {
        return Err(ApiError::new(StatusCode::BAD_REQUEST, "bad_range"));
    }
    Ok((from, to))
}

/// Quanti numeri di QSO puo' indicare al massimo una richiesta (un log lungo anni ne ha decine di migliaia).
const MAX_EXPORT_IDS: usize = 1_000_000;

/// `ids=3,7,10-25`: i numeri dei QSO da esportare, anche a intervalli; assente o vuoto = tutti (`None`). Altro: `bad_ids`.
fn export_ids(v: Option<&str>) -> Result<Option<Vec<i64>>, ApiError> {
    let bad = || ApiError::new(StatusCode::BAD_REQUEST, "bad_ids");
    let Some(v) = v.map(str::trim).filter(|v| !v.is_empty()) else { return Ok(None) };
    let mut ids: Vec<i64> = Vec::new();
    for part in v.split(',').map(str::trim).filter(|p| !p.is_empty()) {
        let (a, b) = match part.split_once('-') {
            Some((a, b)) => (a.trim().parse::<i64>().map_err(|_| bad())?, b.trim().parse::<i64>().map_err(|_| bad())?),
            None => {
                let n = part.parse::<i64>().map_err(|_| bad())?;
                (n, n)
            }
        };
        // un intervallo rovesciato o troppo lungo non e' una scelta fatta dalla pagina
        let room = MAX_EXPORT_IDS.saturating_sub(ids.len()) as u64;
        if a > b || b.abs_diff(a) >= room {
            return Err(bad());
        }
        ids.extend(a..=b);
    }
    Ok(Some(ids))
}

/// Il testo ADIF (formato di JTDX) dei QSO scelti, letto fuori dal thread asincrono.
async fn export_text(state: &AppState, from: Option<String>, to: Option<String>, ids: Option<Vec<i64>>) -> Result<(String, usize), ApiError> {
    let station = state.log_station.as_ref().map(|s| s()).unwrap_or_default();
    let book = state.logbook.clone();
    tokio::task::spawn_blocking(move || book.export_jtdx_adif_selected(&station, from.as_deref(), to.as_deref(), ids.as_deref()))
        .await
        .map_err(|_| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "log_task"))?
        .map_err(|e| {
            tracing::warn!("log non esportato: {e}");
            ApiError::new(StatusCode::BAD_GATEWAY, "log_read")
        })
}

/// Come `export_text`, ma in Cabrillo 3.0 generico (`logbook::Logbook::export_cabrillo_selected`).
async fn cabrillo_text(state: &AppState, from: Option<String>, to: Option<String>, ids: Option<Vec<i64>>) -> Result<(String, usize), ApiError> {
    let station = state.log_station.as_ref().map(|s| s()).unwrap_or_default();
    let book = state.logbook.clone();
    tokio::task::spawn_blocking(move || book.export_cabrillo_selected(&station, from.as_deref(), to.as_deref(), ids.as_deref()))
        .await
        .map_err(|_| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "log_task"))?
        .map_err(|e| {
            tracing::warn!("log Cabrillo non esportato: {e}");
            ApiError::new(StatusCode::BAD_GATEWAY, "log_read")
        })
}

/// `AAAAMMGG` -> `AAAA-MM-GG` (cio' che non ha la forma attesa resta com'e').
fn shown_date(d: &str) -> String {
    match (d.len(), d.get(0..4), d.get(4..6), d.get(6..8)) {
        (8, Some(y), Some(m), Some(g)) => format!("{y}-{m}-{g}"),
        _ => d.to_string(),
    }
}

/// `HHMMSS` -> `HH:MM:SS` (senza secondi: `:00`).
fn shown_time(t: &str) -> String {
    match (t.get(0..2), t.get(2..4)) {
        (Some(h), Some(m)) => format!("{h}:{m}:{}", t.get(4..6).unwrap_or("00")),
        _ => t.to_string(),
    }
}

/// La richiesta arriva da questo stesso PC (loopback, non da un proxy): solo qui si apre una finestra del programma.
fn from_this_pc(peer: Option<SocketAddr>, headers: &HeaderMap) -> bool {
    peer.is_some_and(|p| p.ip().is_loopback()) && !is_proxied(headers)
}

/// Tutti i QSO del log interno, dal piu' recente: `{total, qsos: [{id, date, time, call, band, mode, freq_hz, rst_sent, rst_rcvd, grid,
/// country}], save_dialog}`. `country` viene dal database DXCC (vuoto se manca); `save_dialog` dice se da qui si puo' usare
/// `POST /api/v1/log/export-save` (programma Windows, richiesta dal PC stesso).
#[utoipa::path(get, path = "/api/v1/log/qsos", responses((status = 200, description = "{total, qsos, save_dialog}"), (status = 502, description = "Log non leggibile (log_read)")))]
async fn log_qsos(State(state): State<AppState>, _: Authed, Peer(peer): Peer, headers: HeaderMap) -> Result<Json<Value>, ApiError> {
    let book = state.logbook.clone();
    let dxcc = state.dxcc.clone();
    let rows = tokio::task::spawn_blocking(move || -> Result<Vec<Value>, logbook::LogError> {
        // un paese per nominativo: chi e' a log molte volte si cerca una volta sola
        let mut countries: std::collections::HashMap<String, String> = std::collections::HashMap::new();
        let mut country_of = |call: &str| -> String {
            let Some(lookup) = &dxcc else { return String::new() };
            countries
                .entry(call.to_string())
                .or_insert_with(|| lookup(call).and_then(|v| v.get("country").and_then(Value::as_str).map(str::to_string)).unwrap_or_default())
                .clone()
        };
        Ok(book
            .entries()?
            .into_iter()
            .map(|e| {
                let country = country_of(&e.call);
                json!({ "id": e.id, "date": shown_date(&e.date), "time": shown_time(&e.time), "call": e.call, "band": e.band, "mode": e.mode,
                        "freq_hz": e.freq_hz, "rst_sent": e.rst_sent, "rst_rcvd": e.rst_rcvd, "grid": e.grid, "country": country })
            })
            .collect())
    })
    .await
    .map_err(|_| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "log_task"))?
    .map_err(|e| {
        tracing::warn!("log non letto: {e}");
        ApiError::new(StatusCode::BAD_GATEWAY, "log_read")
    })?;
    let save_dialog = state.save_dialog.is_some() && from_this_pc(peer, &headers);
    Ok(Json(json!({ "total": rows.len(), "qsos": rows, "save_dialog": save_dialog })))
}

/// Il log interno come file ADIF nel formato di JTDX (`wsjtx_log.adi`), con i QSO tra le date `from` e `to` (estremi compresi, `AAAAMMGG`
/// o `AAAA-MM-GG`, facoltative) e, con `ids` (`3,7,10-25`), solo quei QSO. Risponde con un allegato `aurora_log.adi` e `x-aurora-qsos`
/// con il numero di QSO.
#[utoipa::path(get, path = "/api/v1/log/export.adi", responses((status = 200, description = "File ADIF come allegato"), (status = 400, description = "Data non valida o intervallo rovesciato (bad_range), numeri dei QSO non validi (bad_ids)"), (status = 502, description = "Log non leggibile (log_read)")))]
async fn log_export(State(state): State<AppState>, _: Authed, axum::extract::Query(p): axum::extract::Query<ExportParams>) -> Result<Response, ApiError> {
    let (from, to) = export_range(p.from.as_deref(), p.to.as_deref())?;
    let ids = export_ids(p.ids.as_deref())?;
    let (text, count) = export_text(&state, from, to, ids).await?;
    let mut resp = text.into_response();
    let h = resp.headers_mut();
    h.insert(header::CONTENT_TYPE, HeaderValue::from_static("text/plain; charset=utf-8"));
    h.insert(header::CONTENT_DISPOSITION, HeaderValue::from_static("attachment; filename=\"aurora_log.adi\""));
    if let Ok(v) = HeaderValue::from_str(&count.to_string()) {
        h.insert("x-aurora-qsos", v);
    }
    Ok(resp)
}

#[derive(Deserialize, Default)]
struct ExportSaveBody {
    /// I numeri dei QSO da salvare (quelli di `GET /api/v1/log/qsos`).
    #[serde(default)]
    ids: Option<Vec<i64>>,
    /// Tutti i QSO (tra le date, se ci sono): `ids` non conta.
    #[serde(default)]
    all: bool,
    from: Option<String>,
    to: Option<String>,
    /// `cabrillo` per il Cabrillo 3.0 generico; altrimenti ADIF.
    #[serde(default)]
    format: Option<String>,
}

/// Toglie il segno «finestra aperta» anche se la richiesta si interrompe a meta'.
struct SaveOpen(std::sync::Arc<std::sync::atomic::AtomicBool>);

impl Drop for SaveOpen {
    fn drop(&mut self) {
        self.0.store(false, std::sync::atomic::Ordering::SeqCst);
    }
}

/// Salva i QSO scelti come file ADIF dove vuole l'operatore: apre la finestra «Salva con nome» del programma Windows (nome proposto
/// `aurora_log_AAAAMMGG.adi`, filtro `*.adi`) e scrive il file. Solo dal PC stesso. Risponde `{ok, cancelled, path, count}`.
#[utoipa::path(post, path = "/api/v1/log/export-save", responses((status = 200, description = "{ok, cancelled, path, count}: salvato o annullato"), (status = 400, description = "Nessun QSO scelto (no_selection), nessun QSO da salvare (empty_export), date non valide (bad_range)"), (status = 403, description = "Solo dal PC (local_only)"), (status = 404, description = "Finestra non disponibile fuori dal programma Windows (save_unavailable)"), (status = 409, description = "Finestra gia' aperta (save_busy)"), (status = 500, description = "File non scritto (write_failed)")))]
async fn log_export_save(State(state): State<AppState>, _: Authed, Peer(peer): Peer, headers: HeaderMap, Json(body): Json<ExportSaveBody>) -> Result<Json<Value>, ApiError> {
    if !from_this_pc(peer, &headers) {
        return Err(ApiError::new(StatusCode::FORBIDDEN, "local_only"));
    }
    let dialog = state.save_dialog.clone().ok_or_else(|| ApiError::new(StatusCode::NOT_FOUND, "save_unavailable"))?;
    let (from, to) = export_range(body.from.as_deref(), body.to.as_deref())?;
    let ids = if body.all {
        None
    } else {
        match body.ids {
            Some(ids) if !ids.is_empty() => Some(ids),
            _ => return Err(ApiError::new(StatusCode::BAD_REQUEST, "no_selection")),
        }
    };
    let cabrillo = body.format.as_deref() == Some("cabrillo");
    let (text, count) = if cabrillo { cabrillo_text(&state, from, to, ids).await? } else { export_text(&state, from, to, ids).await? };
    let ext = if cabrillo { "log" } else { "adi" };
    if count == 0 {
        return Err(ApiError::new(StatusCode::BAD_REQUEST, "empty_export"));
    }
    if state.save_open.swap(true, std::sync::atomic::Ordering::SeqCst) {
        return Err(ApiError::new(StatusCode::CONFLICT, "save_busy"));
    }
    let open = SaveOpen(std::sync::Arc::clone(&state.save_open));
    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_secs());
    let name = format!("aurora_log_{}.{ext}", logbook::adif::utc_date(now));
    let written = tokio::task::spawn_blocking(move || -> Result<Option<std::path::PathBuf>, String> {
        let _open = open;
        let Some(mut path) = dialog(&name) else { return Ok(None) };
        // senza estensione il file diventa `.adi`, come dice il filtro della finestra
        if path.extension().is_none() {
            path.set_extension(ext);
        }
        std::fs::write(&path, text.as_bytes()).map_err(|e| format!("{}: {e}", path.display()))?;
        Ok(Some(path))
    })
    .await
    .map_err(|_| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "log_task"))?
    .map_err(|e| {
        tracing::warn!("log ADIF non salvato: {e}");
        ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "write_failed").with_message(e)
    })?;
    Ok(Json(match written {
        Some(path) => {
            tracing::info!("log ADIF salvato: {count} QSO in {}", path.display());
            json!({ "ok": true, "cancelled": false, "path": path.display().to_string(), "count": count })
        }
        None => json!({ "ok": true, "cancelled": true }),
    }))
}

/// `GET /api/v1/log4om-db`: il database di Log4OM2 in uso `{path, source, locked, dialog}`. `source` e' `auto` (trovato da solo),
/// `chosen` (scelto dalle Impostazioni), `env` (scritto in aurora.env: `locked`, si cambia solo li') o `none`; `dialog` dice se da qui
/// si puo' aprire la finestra «Apri file».
#[utoipa::path(get, path = "/api/v1/log4om-db", responses((status = 200, description = "{path, source, locked, dialog}"), (status = 404, description = "Non disponibile (log4om_db_unavailable)")))]
async fn log4om_db_get(State(state): State<AppState>, _: Authed, Peer(peer): Peer, headers: HeaderMap) -> Result<Json<Value>, ApiError> {
    let control = state.log4om_db.clone().ok_or_else(|| ApiError::new(StatusCode::NOT_FOUND, "log4om_db_unavailable"))?;
    let mut v = tokio::task::spawn_blocking(move || (control.get)()).await.map_err(|_| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "log_task"))?;
    v["dialog"] = json!(state.open_dialog.is_some() && from_this_pc(peer, &headers));
    Ok(Json(v))
}

#[derive(Deserialize, utoipa::ToSchema)]
struct Log4omDbBody {
    /// Il percorso del file; vuoto o assente = non usare piu' Log4OM2.
    path: Option<String>,
}

/// Cambia (o toglie, con `path` vuoto) il database di Log4OM2: lo controlla, lo ricorda e ricarica subito i QSO gia' fatti. Solo
/// dalla rete di casa. Risponde come il `GET`.
#[utoipa::path(put, path = "/api/v1/log4om-db", request_body = Log4omDbBody, responses((status = 200, description = "{path, source, locked}"), (status = 400, description = "File mancante o non di Log4OM2 (log4om_db_invalid)"), (status = 403, description = "Solo dalla rete di casa (local_only)"), (status = 409, description = "Scritto in aurora.env (log4om_db_locked)")))]
async fn log4om_db_put(State(state): State<AppState>, _: Authed, Peer(peer): Peer, headers: HeaderMap, Json(body): Json<Log4omDbBody>) -> Result<Json<Value>, ApiError> {
    if !from_home_network(peer, &headers) {
        return Err(ApiError::new(StatusCode::FORBIDDEN, "local_only"));
    }
    let path = body.path.map(|p| p.trim().trim_matches('"').to_string()).filter(|p| !p.is_empty()).map(std::path::PathBuf::from);
    apply_log4om_db(&state, path).await
}

/// Apre la finestra «Apri file» del programma Windows per scegliere il database di Log4OM2, poi fa come il `PUT`. Solo dal PC stesso.
/// Annullata: `{cancelled: true}`.
#[utoipa::path(post, path = "/api/v1/log4om-db/choose", responses((status = 200, description = "{path, source, locked} o {cancelled}"), (status = 400, description = "File non di Log4OM2 (log4om_db_invalid)"), (status = 403, description = "Solo dal PC (local_only)"), (status = 404, description = "Finestra non disponibile (open_unavailable)"), (status = 409, description = "Finestra gia' aperta (save_busy) o aurora.env (log4om_db_locked)")))]
async fn log4om_db_choose(State(state): State<AppState>, _: Authed, Peer(peer): Peer, headers: HeaderMap) -> Result<Json<Value>, ApiError> {
    if !from_this_pc(peer, &headers) {
        return Err(ApiError::new(StatusCode::FORBIDDEN, "local_only"));
    }
    let dialog = state.open_dialog.clone().ok_or_else(|| ApiError::new(StatusCode::NOT_FOUND, "open_unavailable"))?;
    if state.save_open.swap(true, std::sync::atomic::Ordering::SeqCst) {
        return Err(ApiError::new(StatusCode::CONFLICT, "save_busy"));
    }
    let open = SaveOpen(std::sync::Arc::clone(&state.save_open));
    let chosen = tokio::task::spawn_blocking(move || {
        let _open = open;
        dialog("Database di Log4OM")
    })
    .await
    .map_err(|_| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "log_task"))?;
    match chosen {
        Some(path) => apply_log4om_db(&state, Some(path)).await,
        None => Ok(Json(json!({ "cancelled": true }))),
    }
}

async fn apply_log4om_db(state: &AppState, path: Option<std::path::PathBuf>) -> Result<Json<Value>, ApiError> {
    let control = state.log4om_db.clone().ok_or_else(|| ApiError::new(StatusCode::NOT_FOUND, "log4om_db_unavailable"))?;
    if (control.get)()["locked"] == json!(true) {
        return Err(ApiError::new(StatusCode::CONFLICT, "log4om_db_locked"));
    }
    tokio::task::spawn_blocking(move || (control.set)(path))
        .await
        .map_err(|_| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "log_task"))?
        .map(Json)
        .map_err(|e| ApiError::new(StatusCode::BAD_REQUEST, "log4om_db_invalid").with_message(e))
}

/// Aggiunge al log interno i QSO di un file ADIF (corpo della richiesta, per esempio il `wsjtx_log.adi` di JTDX), senza doppioni.
/// Risponde `{read, added, duplicates, skipped}`. Solo dalla rete di casa.
/// I QSO conclusi che aspettano la conferma («Richiedimi di loggare il QSO»), dal piu' vecchio.
#[utoipa::path(get, path = "/api/v1/log/pending", responses((status = 200, description = "{pending: [{id, qso}]}")))]
async fn log_pending(State(state): State<AppState>, _: Authed) -> Json<Value> {
    Json(json!({ "pending": state.log_prompt.list() }))
}

#[derive(Deserialize)]
struct PendingAnswer {
    /// `log` (registra, con le correzioni) o `discard` (scarta).
    action: String,
    #[serde(default)]
    edits: crate::log_prompt::Edits,
}

/// La risposta della finestra di conferma: `log` registra il QSO (con le correzioni), `discard` lo scarta.
#[utoipa::path(post, path = "/api/v1/log/pending/{id}", responses((status = 200, description = "Registrato o scartato"), (status = 400, description = "Azione o campo non validi (bad_action, bad_field)"), (status = 404, description = "QSO non in attesa (not_pending)")))]
async fn log_pending_answer(State(state): State<AppState>, _: Authed, axum::extract::Path(id): axum::extract::Path<u64>, Json(body): Json<PendingAnswer>) -> Result<Json<Value>, ApiError> {
    if body.action != "log" && body.action != "discard" {
        return Err(ApiError::new(StatusCode::BAD_REQUEST, "bad_action"));
    }
    // si controllano le correzioni prima di togliere il QSO dalla coda: un campo sbagliato non lo fa sparire
    let mut qso = state.log_prompt.list().into_iter().find(|p| p.id == id).map(|p| p.qso).ok_or_else(|| ApiError::new(StatusCode::NOT_FOUND, "not_pending"))?;
    if body.action == "log" {
        body.edits.apply(&mut qso).map_err(|field| ApiError::new(StatusCode::BAD_REQUEST, "bad_field").with_message(field))?;
    }
    if state.log_prompt.take(id).is_none() {
        return Err(ApiError::new(StatusCode::NOT_FOUND, "not_pending"));
    }
    if body.action == "log" {
        let _ = state.bus.events().send(aurora_core::Event::QsoLogged(qso.clone()));
        return Ok(Json(json!({ "ok": true, "logged": qso.call })));
    }
    Ok(Json(json!({ "ok": true, "discarded": qso.call })))
}

#[utoipa::path(post, path = "/api/v1/log/import", responses((status = 200, description = "{read, added, duplicates, skipped}"), (status = 400, description = "Corpo vuoto (empty_file)"), (status = 403, description = "Solo dalla rete di casa (local_only)"), (status = 413, description = "File troppo grande"), (status = 502, description = "Scrittura non riuscita (log_write)")))]
async fn log_import(State(state): State<AppState>, _: Authed, Peer(peer): Peer, headers: HeaderMap, body: axum::body::Bytes) -> Result<Json<Value>, ApiError> {
    if !from_home_network(peer, &headers) {
        return Err(ApiError::new(StatusCode::FORBIDDEN, "local_only"));
    }
    if body.iter().all(u8::is_ascii_whitespace) {
        return Err(ApiError::new(StatusCode::BAD_REQUEST, "empty_file"));
    }
    let book = state.logbook.clone();
    let report = tokio::task::spawn_blocking(move || book.import_adif(&String::from_utf8_lossy(&body)))
        .await
        .map_err(|_| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "log_task"))?
        .map_err(|e| {
            tracing::warn!("log non importato: {e}");
            ApiError::new(StatusCode::BAD_GATEWAY, "log_write")
        })?;
    if report.added > 0
        && let Some(changed) = state.log_changed.clone()
    {
        let _ = tokio::task::spawn_blocking(move || changed()).await;
    }
    Ok(Json(json!(report)))
}

#[derive(Deserialize)]
struct LogDeleteBody {
    #[serde(default)]
    ids: Vec<i64>,
    /// Tutto il log (vale solo con `all: true`, per non svuotarlo per sbaglio).
    #[serde(default)]
    all: bool,
}

/// Cancella i QSO scelti (`ids`) o tutto il log (`all: true`); toglie gli stessi QSO anche dal file ADIF di Aurora. Solo dalla rete di casa.
#[utoipa::path(delete, path = "/api/v1/log/qsos", responses((status = 200, description = "{deleted}"), (status = 400, description = "Niente da cancellare (bad_delete)"), (status = 403, description = "Solo dalla rete di casa (local_only)")))]
async fn log_delete(State(state): State<AppState>, _: Authed, Peer(peer): Peer, headers: HeaderMap, Json(body): Json<LogDeleteBody>) -> Result<Json<Value>, ApiError> {
    if !from_home_network(peer, &headers) {
        return Err(ApiError::new(StatusCode::FORBIDDEN, "local_only"));
    }
    if !body.all && body.ids.is_empty() {
        return Err(ApiError::new(StatusCode::BAD_REQUEST, "bad_delete"));
    }
    let book = state.logbook.clone();
    let all = body.all;
    let result = tokio::task::spawn_blocking(move || if all { book.clear().map(|n| (n, None)) } else { book.delete(&body.ids).map(|g| (g.len() as u64, Some(g))) })
        .await
        .map_err(|_| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "log_task"))?
        .map_err(|e| {
            tracing::warn!("QSO non cancellati: {e}");
            ApiError::new(StatusCode::BAD_GATEWAY, "log_write")
        })?;
    let (deleted, gone) = result;
    tracing::info!("log: {deleted} QSO cancellati{}", if all { " (log svuotato)" } else { "" });
    if let Some(hook) = state.log_deleted.clone() {
        let _ = tokio::task::spawn_blocking(move || hook(gone)).await;
    }
    Ok(Json(json!({ "deleted": deleted })))
}

#[derive(Deserialize)]
struct LogAddBody {
    call: String,
    #[serde(default)]
    grid: String,
    /// `20m`; vuota = dalla frequenza.
    #[serde(default)]
    band: String,
    mode: String,
    #[serde(default)]
    freq_hz: u64,
    /// `AAAA-MM-GG` UTC.
    date: String,
    /// `HH:MM` o `HH:MM:SS` UTC.
    time: String,
    #[serde(default)]
    rst_sent: String,
    #[serde(default)]
    rst_rcvd: String,
    #[serde(default)]
    name: String,
    #[serde(default)]
    comment: String,
}

/// Un QSO scritto a mano: va a log come quelli fatti da Aurora (log interno, file ADIF, programmi collegati e servizi di log accesi).
#[utoipa::path(post, path = "/api/v1/log/qsos", responses((status = 200, description = "QSO registrato"), (status = 400, description = "Campo non valido (bad_field con field)"), (status = 403, description = "Solo dalla rete di casa (local_only)")))]
async fn log_add(State(state): State<AppState>, _: Authed, Peer(peer): Peer, headers: HeaderMap, Json(b): Json<LogAddBody>) -> Result<Json<Value>, ApiError> {
    if !from_home_network(peer, &headers) {
        return Err(ApiError::new(StatusCode::FORBIDDEN, "local_only"));
    }
    let bad = |field: &str| {
        let mut e = ApiError::new(StatusCode::BAD_REQUEST, "bad_field");
        e.message = Some(field.to_string());
        e
    };
    let call = b.call.trim().to_uppercase();
    if call.len() < 3 || call.len() > 13 || !call.chars().all(|c| c.is_ascii_alphanumeric() || c == '/') {
        return Err(bad("call"));
    }
    let mode = b.mode.trim().to_uppercase();
    if mode.is_empty() || mode.len() > 12 || !mode.chars().all(|c| c.is_ascii_alphanumeric()) {
        return Err(bad("mode"));
    }
    let start = manual_time(&b.date, &b.time).ok_or_else(|| bad("time"))?;
    let band = if b.band.trim().is_empty() { band_of_hz(b.freq_hz).unwrap_or_default() } else { b.band.trim().to_lowercase() };
    if band.is_empty() {
        return Err(bad("band"));
    }
    let clip = |s: &str, n: usize| s.trim().chars().take(n).collect::<String>();
    let q = aurora_core::QsoRecord {
        call,
        grid: clip(&b.grid, 8).to_uppercase(),
        band,
        mode,
        freq_hz: b.freq_hz,
        rst_sent: clip(&b.rst_sent, 8),
        rst_rcvd: clip(&b.rst_rcvd, 8),
        time_ms: ((start % 86_400_000) as u32),
        time_on_unix_ms: start,
        time_off_unix_ms: start,
        name: clip(&b.name, 60),
        comment: clip(&b.comment, 200),
        ..Default::default()
    };
    tracing::info!("QSO scritto a mano: {} {} {}", q.call, q.band, q.mode);
    let _ = state.bus.events().send(aurora_core::Event::QsoLogged(q));
    Ok(Json(json!({ "ok": true })))
}

/// `AAAA-MM-GG` e `HH:MM[:SS]` UTC in millisecondi dall'epoca UNIX.
fn manual_time(date: &str, time: &str) -> Option<u64> {
    let d: Vec<u64> = date.trim().split('-').map(|p| p.parse().ok()).collect::<Option<_>>()?;
    let t: Vec<u64> = time.trim().split(':').map(|p| p.parse().ok()).collect::<Option<_>>()?;
    let (&[y, m, g], [h, mi, rest @ ..]) = (d.as_slice(), t.as_slice()) else { return None };
    let s = rest.first().copied().unwrap_or(0);
    if !(1970..=2200).contains(&y) || !(1..=12).contains(&m) || !(1..=31).contains(&g) || *h > 23 || *mi > 59 || s > 59 {
        return None;
    }
    // giorni dall'epoca (algoritmo di Howard Hinnant, calendario gregoriano)
    let (y2, m2) = if m <= 2 { (y - 1, m + 9) } else { (y, m - 3) };
    let era = y2 / 400;
    let yoe = y2 - era * 400;
    let doy = (153 * m2 + 2) / 5 + g - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    let days = era * 146_097 + doe - 719_468;
    Some(((days * 86_400) + h * 3600 + mi * 60 + s) * 1000)
}

/// La banda di una frequenza (per i QSO scritti a mano senza banda).
fn band_of_hz(hz: u64) -> Option<String> {
    let khz = hz / 1000;
    let bands: [(u64, u64, &str); 13] = [(1800, 2000, "160m"), (3500, 4000, "80m"), (5250, 5450, "60m"), (7000, 7300, "40m"), (10100, 10150, "30m"), (14000, 14350, "20m"), (18068, 18168, "17m"), (21000, 21450, "15m"), (24890, 24990, "12m"), (28000, 29700, "10m"), (50000, 54000, "6m"), (144000, 148000, "2m"), (430000, 440000, "70cm")];
    bands.iter().find(|(a, b, _)| (*a..=*b).contains(&khz)).map(|(_, _, n)| n.to_string())
}

#[derive(Deserialize)]
struct Log4omQuery {
    limit: Option<usize>,
}

#[utoipa::path(get, path = "/api/v1/log4om", responses((status = 200, description = "Ultimi QSO del log di Log4OM2 (sola lettura)"), (status = 404, description = "Log non configurato (log4om_unavailable)"), (status = 502, description = "File non leggibile (log4om_read)")))]
async fn log4om(State(state): State<AppState>, _: Authed, axum::extract::Query(q): axum::extract::Query<Log4omQuery>) -> Result<Json<Value>, ApiError> {
    let read = state.log4om.clone().ok_or_else(|| ApiError::new(StatusCode::NOT_FOUND, "log4om_unavailable"))?;
    let limit = q.limit.unwrap_or(20).clamp(1, 500);
    tokio::task::spawn_blocking(move || read(limit))
        .await
        .map_err(|_| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "log4om_task"))?
        .map(Json)
        .map_err(|_| ApiError::new(StatusCode::BAD_GATEWAY, "log4om_read"))
}

#[utoipa::path(get, path = "/api/v1/pairing-settings", responses((status = 200, description = "Durata del codice di associazione e limiti")))]
async fn pairing_get(State(state): State<AppState>, _: Authed) -> Json<Value> {
    let (min, max) = auth::DeviceStore::code_ttl_limits();
    Json(json!({ "code_ttl_secs": state.auth.code_ttl_secs(), "min_secs": min, "max_secs": max }))
}

#[derive(Deserialize)]
struct PairingBody {
    code_ttl_secs: u64,
}

/// Quanto vale un codice di associazione: lo decide l'operatore, solo dal PC (e' una scelta di sicurezza).
#[utoipa::path(put, path = "/api/v1/pairing-settings", responses((status = 200, description = "Durata impostata"), (status = 400, description = "Fuori dai limiti (bad_ttl)"), (status = 403, description = "Solo dal PC (local_only)")))]
async fn pairing_put(State(state): State<AppState>, _: Authed, Peer(peer): Peer, headers: HeaderMap, Json(body): Json<PairingBody>) -> Result<Json<Value>, ApiError> {
    if !peer.is_some_and(|p| p.ip().is_loopback()) || is_proxied(&headers) {
        return Err(ApiError::new(StatusCode::FORBIDDEN, "local_only"));
    }
    let (min, max) = auth::DeviceStore::code_ttl_limits();
    if !(min..=max).contains(&body.code_ttl_secs) {
        return Err(ApiError::new(StatusCode::BAD_REQUEST, "bad_ttl"));
    }
    state.auth.set_code_ttl(body.code_ttl_secs);
    saved(&state);
    Ok(Json(json!({ "ok": true, "code_ttl_secs": state.auth.code_ttl_secs() })))
}

#[utoipa::path(get, path = "/api/v1/access", responses((status = 200, description = "Se l'accesso senza codice e' acceso")))]
async fn access_get(State(state): State<AppState>) -> Json<Value> {
    Json(json!({ "open": state.open_access.load(std::sync::atomic::Ordering::SeqCst), "mdns": mdns_url(&state) }))
}

#[derive(Deserialize)]
struct AccessBody {
    open: bool,
}

/// Accesso senza codice dalla rete di casa: si accende e si spegne solo dal PC e resta al riavvio.
#[utoipa::path(put, path = "/api/v1/access", responses((status = 200, description = "Impostato"), (status = 403, description = "Solo dal PC (local_only)")))]
async fn access_put(State(state): State<AppState>, Peer(peer): Peer, headers: HeaderMap, Json(body): Json<AccessBody>) -> Result<Json<Value>, ApiError> {
    if !peer.is_some_and(|p| p.ip().is_loopback()) || is_proxied(&headers) {
        return Err(ApiError::new(StatusCode::FORBIDDEN, "local_only"));
    }
    if let Some(path) = &state.open_access_file
        && let Err(e) = std::fs::write(path, if body.open { "1
" } else { "0
" })
    {
        tracing::error!("accesso senza codice non salvato in {}: {e}", path.display());
        return Err(ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "storage"));
    }
    state.open_access.store(body.open, std::sync::atomic::Ordering::SeqCst);
    tracing::warn!("accesso senza codice {}", if body.open { "ACCESO (rete di casa)" } else { "spento" });
    Ok(Json(json!({ "ok": true, "open": body.open })))
}

fn saved(state: &AppState) {
    if let Some(save) = &state.settings_saver {
        save();
    }
}

#[utoipa::path(get, path = "/api/v1/frequencies", responses((status = 200, description = "Frequenze di lavoro per banda e modo")))]
async fn frequencies_get(State(state): State<AppState>, _: Authed) -> Json<Value> {
    let table = state.frequencies.lock().unwrap_or_else(|p| p.into_inner());
    Json(serde_json::to_value(table.entries()).unwrap_or(Value::Null))
}

/// La tabella iniziale di JTDX (FT8 e FT4), con le righe di ogni regione IARU: serve al tasto «Frequenze iniziali» della scheda Frequenze.
#[utoipa::path(get, path = "/api/v1/frequencies/defaults", responses((status = 200, description = "Tabella iniziale delle frequenze, con regione e riga predefinita")))]
async fn frequencies_defaults(_: Authed) -> Json<Value> {
    Json(json!(rig_control::table::FrequencyTable::default().entries()))
}

#[utoipa::path(put, path = "/api/v1/frequencies", responses((status = 200, description = "Tabella sostituita"), (status = 400, description = "Tabella non valida (bad_frequencies)")))]
async fn frequencies_put(State(state): State<AppState>, _: Authed, Json(entries): Json<Vec<rig_control::table::Entry>>) -> Result<Json<Value>, ApiError> {
    {
        let mut table = state.frequencies.lock().unwrap_or_else(|p| p.into_inner());
        table.set(entries).map_err(|_| ApiError::new(StatusCode::BAD_REQUEST, "bad_frequencies"))?;
    }
    saved(&state);
    Ok(Json(json!({ "ok": true })))
}

#[utoipa::path(get, path = "/api/v1/schedule", responses((status = 200, description = "Programmazione delle bande")))]
async fn schedule_get(State(state): State<AppState>, _: Authed) -> Json<Value> {
    let schedule = state.schedule.lock().unwrap_or_else(|p| p.into_inner());
    Json(json!({ "enabled": schedule.enabled(), "entries": schedule.entries() }))
}

#[derive(Deserialize)]
struct ScheduleBody {
    enabled: bool,
    entries: Vec<rig_control::schedule::ScheduleEntry>,
}

#[utoipa::path(put, path = "/api/v1/schedule", responses((status = 200, description = "Programmazione impostata"), (status = 400, description = "Programmazione non valida (bad_schedule)")))]
async fn schedule_put(State(state): State<AppState>, _: Authed, Json(body): Json<ScheduleBody>) -> Result<Json<Value>, ApiError> {
    {
        let mut schedule = state.schedule.lock().unwrap_or_else(|p| p.into_inner());
        schedule.set(body.enabled, body.entries).map_err(|_| ApiError::new(StatusCode::BAD_REQUEST, "bad_schedule"))?;
    }
    saved(&state);
    Ok(Json(json!({ "ok": true })))
}

async fn openapi() -> Json<Value> {
    Json(serde_json::to_value(ApiDoc::openapi()).unwrap_or(Value::Null))
}
