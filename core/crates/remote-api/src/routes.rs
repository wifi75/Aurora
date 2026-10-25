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
    routing::{delete, get, post},
    Json, Router,
};
use qso_engine::{Mode, Rules};
use serde::Deserialize;
use serde_json::{json, Value};
use utoipa::OpenApi;

use crate::{policy::transport_of, ws, AppState};

const COOKIE_NAME: &str = "aurora_session";
const COOKIE_MAX_AGE_SECS: u64 = 31_536_000;

#[derive(OpenApi)]
#[openapi(paths(update_install, online_get, online_put, online_psk, online_solar, online_refresh, history_get, history_stats, history_clear, output_level_get, output_level_put, input_level_get, input_level_put, controls_get, control_press, tx_message, access_get, access_put, pairing_get, pairing_put, frequencies_get, frequencies_put, frequencies_defaults, schedule_get, schedule_put, stats, dxcc_matrix, log_export, log_import, log4om, map_worked, wishlist_get, wishlist_put, wishlist_test, update_get, update_put, update_check, engine_config, engine_config_apply, engine_settings_get, engine_settings_put, spot_dxsummit, audio_devices, audio_settings_put, dxcc_lookup, cluster_get, cluster_put, cluster_catalog, cluster_test, cluster_command, clock_get, clock_put, clock_check, clock_correct, ui_settings_get, ui_settings_put, change_band, change_frequency, change_mode, rx_frequency, free_text, call_cq, tx_frequency, station_info, health, pair_start, pair_finish, devices, revoke_device, halt_tx, reply, automation_get, automation_put, ws::stream))]
struct ApiDoc;

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/api/v1/health", get(health))
        .route("/api/v1/pair/start", post(pair_start))
        .route("/api/v1/pair/finish", post(pair_finish))
        .route("/api/v1/devices", get(devices))
        .route("/api/v1/devices/{id}", delete(revoke_device))
        .route("/api/v1/commands/halt-tx", post(halt_tx))
        .route("/api/v1/commands/reply", post(reply))
        .route("/api/v1/commands/free-text", post(free_text))
        .route("/api/v1/commands/cq", post(call_cq))
        .route("/api/v1/commands/tx-frequency", post(tx_frequency))
        .route("/api/v1/commands/band", post(change_band))
        .route("/api/v1/commands/frequency", post(change_frequency))
        .route("/api/v1/commands/mode", post(change_mode))
        .route("/api/v1/commands/rx-frequency", post(rx_frequency))
        .route("/api/v1/output-level", get(output_level_get).put(output_level_put))
        .route("/api/v1/input-level", get(input_level_get).put(input_level_put))
        .route("/api/v1/engine/native", get(native_engine_get))
        .route("/api/v1/online", get(online_get).put(online_put))
        .route("/api/v1/online/psk", get(online_psk))
        .route("/api/v1/online/solar", get(online_solar))
        .route("/api/v1/online/refresh", post(online_refresh))
        .route("/api/v1/history", get(history_get).delete(history_clear))
        .route("/api/v1/history/stats", get(history_stats))
        .route("/api/v1/controls", get(controls_get))
        .route("/api/v1/commands/control", post(control_press))
        .route("/api/v1/commands/tx-message", post(tx_message))
        .route("/api/v1/frequencies", get(frequencies_get).put(frequencies_put))
        .route("/api/v1/frequencies/defaults", get(frequencies_defaults))
        .route("/api/v1/pairing-settings", get(pairing_get).put(pairing_put))
        .route("/api/v1/access", get(access_get).put(access_put))
        .route("/api/v1/schedule", get(schedule_get).put(schedule_put))
        .route("/api/v1/automation", get(automation_get).put(automation_put))
        .route("/api/v1/station-info", get(station_info))
        .route("/api/v1/stats", get(stats))
        .route("/api/v1/dxcc-matrix", get(dxcc_matrix))
        .route("/api/v1/log/export.adi", get(log_export))
        .route("/api/v1/log/import", post(log_import).layer(axum::extract::DefaultBodyLimit::max(LOG_IMPORT_MAX_BYTES)))
        .route("/api/v1/log4om", get(log4om))
        .route("/api/v1/map/worked", get(map_worked))
        .route("/api/v1/wishlist", get(wishlist_get).put(wishlist_put))
        .route("/api/v1/wishlist/test", post(wishlist_test))
        .route("/api/v1/update", get(update_get).put(update_put))
        .route("/api/v1/update/check", post(update_check))
        .route("/api/v1/update/install", post(update_install))
        .route("/api/v1/engine-config", get(engine_config).post(engine_config_apply))
        .route("/api/v1/engine-settings", get(engine_settings_get).put(engine_settings_put))
        .route("/api/v1/spot/dxsummit", post(spot_dxsummit))
        .route("/api/v1/audio/devices", get(audio_devices))
        .route("/api/v1/audio/settings", axum::routing::put(audio_settings_put))
        .route("/api/v1/dx-cluster", get(cluster_get).put(cluster_put))
        .route("/api/v1/dxcc/{call}", get(dxcc_lookup))
        .route("/api/v1/dx-cluster/catalog", get(cluster_catalog))
        .route("/api/v1/dx-cluster/test", post(cluster_test))
        .route("/api/v1/dx-cluster/command", post(cluster_command))
        .route("/api/v1/clock", get(clock_get).put(clock_put))
        .route("/api/v1/clock/check", post(clock_check))
        .route("/api/v1/clock/correct", post(clock_correct))
        .route("/api/v1/ui-settings/{kind}", get(ui_settings_get).put(ui_settings_put))
        .route("/api/v1/cat", get(crate::cat::cat_get).put(crate::cat::cat_put))
        .route("/api/v1/cat/connect", post(crate::cat::cat_connect))
        .route("/api/v1/cat/disconnect", post(crate::cat::cat_disconnect))
        .route("/api/v1/cat/profiles", post(crate::cat::cat_profiles))
        .route("/api/v1/cat/models", get(crate::cat::cat_models))
        .route("/api/v1/cat/ports", get(crate::cat::cat_ports))
        .route("/api/v1/cat/detect", post(crate::cat::cat_detect))
        .route("/api/v1/stream", get(ws::stream))
        .route("/api/v1/openapi.json", get(openapi))
        .with_state(state)
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
        // prove: con l'accesso senza codice acceso entra chi e' sulla rete di casa e non arriva da un proxy
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
    Json(json!({ "status": "ok", "name": APP_NAME, "version": APP_VERSION, "author": APP_AUTHOR }))
}

#[derive(Deserialize, Default)]
struct PairStartBody {
    base_url: Option<String>,
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
    let qr_text = match body.and_then(|Json(b)| b.base_url) {
        Some(base) => format!("{}/#/pair/{}", base.trim_end_matches('/'), pairing.code),
        None => pairing.code.clone(),
    };
    let qr_svg = qrcode::QrCode::new(qr_text.as_bytes())
        .map_err(|_| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "qr_failed"))?
        .render::<qrcode::render::svg::Color>()
        .min_dimensions(200, 200)
        .build();
    Ok(Json(json!({ "code": pairing.code, "expires_in": pairing.expires_in_secs, "qr_svg": qr_svg })))
}

#[derive(Deserialize)]
struct PairFinishBody {
    code: String,
    device_name: String,
}

#[utoipa::path(post, path = "/api/v1/pair/finish", responses((status = 204, description = "Dispositivo associato, cookie di sessione impostato"), (status = 400, description = "Codice o nome non validi"), (status = 410, description = "Codice scaduto"), (status = 429, description = "Troppi tentativi")))]
async fn pair_finish(State(state): State<AppState>, Json(body): Json<PairFinishBody>) -> Response {
    match state.auth.finish_pairing(&body.code, &body.device_name) {
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
fn halt_for_band_change(state: &AppState, target_hz: u64) {
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

#[utoipa::path(post, path = "/api/v1/commands/reply", responses((status = 200, description = "Risposta inviata al motore"), (status = 403, description = "Trasmissione non abilitata (tx_disabled) o non consentita da questa via"), (status = 404, description = "Decode sconosciuto")))]
async fn reply(
    State(state): State<AppState>,
    _: Authed,
    headers: HeaderMap,
    Json(body): Json<ReplyBody>,
) -> Result<Json<Value>, ApiError> {
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

/// Chi fa trasmettere deve avere la trasmissione aperta e arrivare da una via consentita.
fn check_tx_allowed(state: &AppState, headers: &HeaderMap) -> Result<(), ApiError> {
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

#[derive(Deserialize, Default)]
struct CqBody {
    #[serde(default)]
    modifier: String,
}

#[utoipa::path(post, path = "/api/v1/commands/cq", responses((status = 200, description = "CQ chiesto a JTDX"), (status = 400, description = "Modificatore non valido"), (status = 403, description = "Trasmissione non abilitata o non consentita da questa via")))]
async fn call_cq(State(state): State<AppState>, _: Authed, headers: HeaderMap, body: Option<Json<CqBody>>) -> Result<Json<Value>, ApiError> {
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
    let rig = state.active_rig().ok_or_else(|| ApiError::new(StatusCode::SERVICE_UNAVAILABLE, "rig_unavailable"))?;
    let mode = body.mode.clone().unwrap_or_else(|| state.current_mode());
    // il blocco chiude il lock prima di qualunque attesa
    let region = state.frequency_region();
    let found = {
        let table = state.frequencies.lock().unwrap_or_else(|p| p.into_inner());
        table.frequency_in(region, &body.band, &mode)
    };
    let hz = found.ok_or_else(|| ApiError::new(StatusCode::BAD_REQUEST, "bad_band"))?;
    if state.current_status().is_some_and(|s| s.transmitting) {
        return Err(ApiError::new(StatusCode::CONFLICT, "transmitting"));
    }
    halt_for_band_change(&state, hz);
    // il comando blocca (PowerShell e COM): fuori dal thread asincrono
    let done = tokio::task::spawn_blocking(move || rig.set_frequency(hz)).await;
    match done {
        Ok(Ok(())) => Ok(Json(json!({ "ok": true, "dial_hz": hz }))),
        Ok(Err(e)) => {
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
    let rig = state.active_rig().ok_or_else(|| ApiError::new(StatusCode::SERVICE_UNAVAILABLE, "rig_unavailable"))?;
    if rig_control::bands::amateur_band(body.hz).is_none() {
        return Err(ApiError::new(StatusCode::BAD_REQUEST, "bad_frequency"));
    }
    if state.current_status().is_some_and(|s| s.transmitting) {
        return Err(ApiError::new(StatusCode::CONFLICT, "transmitting"));
    }
    let hz = body.hz;
    halt_for_band_change(&state, hz);
    match tokio::task::spawn_blocking(move || rig.set_frequency(hz)).await {
        Ok(Ok(())) => Ok(Json(json!({ "ok": true, "dial_hz": hz }))),
        Ok(Err(e)) => {
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
    match tokio::task::spawn_blocking(move || rig.controls_and_tx()).await {
        Ok(Ok((list, tx))) => {
            let map: serde_json::Map<String, Value> = list.into_iter().map(|(id, on)| (id, Value::Bool(on))).collect();
            // i sei messaggi Tx di JTDX e quello selezionato (1-6), quando la radio li sa leggere
            let tx = tx.map(|t| json!({ "messages": t.messages, "selected": t.selected, "minute": t.minute_label, "autoseq": t.autoseq_label }));
            Ok(Json(json!({ "controls": map, "tx": tx })))
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
}

/// Imposta un interruttore di JTDX o preme un suo pulsante. Accendere Abilita Tx o AutoTX puo' far trasmettere:
/// serve la trasmissione aperta e una via consentita; spegnerli e' sempre permesso.
#[utoipa::path(post, path = "/api/v1/commands/control", responses((status = 200, description = "Fatto"), (status = 400, description = "Controllo sconosciuto o argomenti sbagliati (bad_control)"), (status = 403, description = "Trasmissione non abilitata o via non consentita"), (status = 502, description = "JTDX non ha risposto (rig_error)"), (status = 503, description = "Comando non disponibile (rig_unavailable)")))]
async fn control_press(State(state): State<AppState>, _: Authed, headers: HeaderMap, Json(body): Json<ControlBody>) -> Result<Json<Value>, ApiError> {
    let control = rig_control::controls::find(&body.id).ok_or_else(|| ApiError::new(StatusCode::BAD_REQUEST, "bad_control"))?;
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

/// Porta JTDX in FT8 o FT4 (preme il suo pulsante). Mai durante la trasmissione.
#[utoipa::path(post, path = "/api/v1/commands/mode", responses((status = 200, description = "Modo cambiato"), (status = 400, description = "Modo sconosciuto (bad_mode)"), (status = 409, description = "In trasmissione (transmitting)"), (status = 502, description = "JTDX non ha risposto (rig_error)"), (status = 503, description = "Comando non disponibile (rig_unavailable)")))]
async fn change_mode(State(state): State<AppState>, _: Authed, Json(body): Json<ModeBody>) -> Result<Json<Value>, ApiError> {
    let rig = state.active_rig().ok_or_else(|| ApiError::new(StatusCode::SERVICE_UNAVAILABLE, "rig_unavailable"))?;
    if rig_control::modes::button_id(&body.mode).is_none() {
        return Err(ApiError::new(StatusCode::BAD_REQUEST, "bad_mode"));
    }
    if state.current_status().is_some_and(|s| s.transmitting) {
        return Err(ApiError::new(StatusCode::CONFLICT, "transmitting"));
    }
    let mode = body.mode.trim().to_uppercase();
    let shown = mode.clone();
    // JTDX al cambio di modo salta all'ultima frequenza che aveva in quel modo, anche su un'altra banda:
    // si resta nella banda di prima, sulla frequenza che la tabella indica per il nuovo modo
    // la frequenza di adesso si rilegge dalla radio (dopo lo spot di un cluster lo stato della pagina puo' essere indietro); se la radio
    // non risponde si usa l'ultima nota. La banda si cerca nella tabella e, fuori da quella, tra le bande dei radioamatori.
    let status_dial = state.current_status().filter(|s| s.dial_hz > 0).map(|s| s.dial_hz);
    let frequencies = std::sync::Arc::clone(&state.frequencies);
    let region = state.frequency_region();
    let work = move || -> Result<Option<u64>, rig_control::RigError> {
        rig.set_mode(&mode)?;
        let current = rig.frequency().ok().filter(|hz| *hz > 0).or(status_dial);
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
            rig.set_frequency(hz)?;
        }
        Ok(target)
    };
    match tokio::task::spawn_blocking(work).await {
        Ok(Ok(dial)) => Ok(Json(json!({ "ok": true, "mode": shown, "dial_hz": dial }))),
        Ok(Err(e)) => {
            tracing::warn!("cambio modo fallito: {e}");
            Err(ApiError::new(StatusCode::BAD_GATEWAY, "rig_error"))
        }
        Err(_) => Err(ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "rig_task")),
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

#[derive(Deserialize)]
struct JtdxApplyBody {
    changes: std::collections::BTreeMap<String, String>,
}

/// Cambia impostazioni di JTDX: lo chiude, scrive il suo file (con copia di sicurezza) e lo riavvia. Solo dalla rete di casa
/// e mai mentre si trasmette: JTDX resta spento qualche secondo.
#[utoipa::path(post, path = "/api/v1/engine-config", responses((status = 200, description = "Impostazioni applicate e JTDX riavviato"), (status = 400, description = "Voce o valore non ammessi (bad_setting)"), (status = 403, description = "Solo dalla rete di casa (local_only)"), (status = 404, description = "Non disponibile (engine_config_unavailable)"), (status = 409, description = "Si sta trasmettendo (transmitting)"), (status = 502, description = "Applicazione non riuscita (engine_apply_failed)")))]
async fn engine_config_apply(State(state): State<AppState>, _: Authed, Peer(peer): Peer, headers: HeaderMap, Json(body): Json<JtdxApplyBody>) -> Result<Json<Value>, ApiError> {
    let apply = state.engine_apply.clone().ok_or_else(|| ApiError::new(StatusCode::NOT_FOUND, "engine_config_unavailable"))?;
    if !from_home_network(peer, &headers) {
        return Err(ApiError::new(StatusCode::FORBIDDEN, "local_only"));
    }
    if body.changes.is_empty() || body.changes.len() > 100 || body.changes.iter().any(|(k, v)| engine_jtdx::ini_edit::check_value(k, v).is_err()) {
        return Err(ApiError::new(StatusCode::BAD_REQUEST, "bad_setting"));
    }
    if state.current_status().is_some_and(|s| s.transmitting) {
        return Err(ApiError::new(StatusCode::CONFLICT, "transmitting"));
    }
    match tokio::task::spawn_blocking(move || apply(body.changes)).await {
        Ok(Ok(())) => Ok(Json(json!({ "ok": true }))),
        Ok(Err(e)) => {
            tracing::warn!("applicazione delle impostazioni di JTDX fallita: {e}");
            Err(ApiError::new(StatusCode::BAD_GATEWAY, "engine_apply_failed"))
        }
        Err(_) => Err(ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "jtdx_task")),
    }
}

#[utoipa::path(get, path = "/api/v1/dxcc/{call}", responses((status = 200, description = "Paese, continente e zone del nominativo (dal file cty.dat)"), (status = 400, description = "Nominativo non valido (bad_call)"), (status = 404, description = "Sconosciuto o file non caricato (dxcc_unknown)")))]
async fn dxcc_lookup(State(state): State<AppState>, _: Authed, axum::extract::Path(call): axum::extract::Path<String>) -> Result<Json<Value>, ApiError> {
    if call.is_empty() || call.len() > 20 || !call.chars().all(|c| c.is_ascii_alphanumeric() || c == '/') {
        return Err(ApiError::new(StatusCode::BAD_REQUEST, "bad_call"));
    }
    state.dxcc.as_ref().and_then(|lookup| lookup(&call)).map(Json).ok_or_else(|| ApiError::new(StatusCode::NOT_FOUND, "dxcc_unknown"))
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
    Ok(Json(json!({ "config": cluster.config(), "servers": cluster.statuses(), "spots": cluster.spots_since(q.since.unwrap_or(0), limit) })))
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
    Ok(Json(json!({ "config": clock.config(), "last": clock.last() })))
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

#[utoipa::path(get, path = "/api/v1/engine-config", responses((status = 200, description = "Impostazioni di JTDX lette dal suo file (sola lettura, senza segreti)"), (status = 404, description = "File non configurato (engine_config_unavailable)"), (status = 502, description = "File non leggibile (engine_config_read)")))]
async fn engine_config(State(state): State<AppState>, _: Authed) -> Result<Json<Value>, ApiError> {
    let read = state.engine_config.clone().ok_or_else(|| ApiError::new(StatusCode::NOT_FOUND, "engine_config_unavailable"))?;
    tokio::task::spawn_blocking(move || read())
        .await
        .map_err(|_| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "engine_config_task"))?
        .map(Json)
        .map_err(|_| ApiError::new(StatusCode::BAD_GATEWAY, "engine_config_read"))
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
        Err(_) => Err(ApiError::new(StatusCode::BAD_REQUEST, "bad_setting")),
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
    let params = reporting::SpotParams { dx_call: body.dx_call, dx_grid: body.dx_grid, text: body.text, prop: body.prop };
    match service.spot_current(params).await {
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
    tokio::task::spawn_blocking(move || store.put(&kind, entries))
        .await
        .map_err(|_| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "ui_settings_task"))?
        .map_err(|e| {
            tracing::warn!("salvataggio delle preferenze fallito: {e}");
            ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "ui_settings_write")
        })?;
    Ok(Json(json!({ "ok": true })))
}

fn update_service(state: &AppState) -> Result<std::sync::Arc<updater::UpdateService>, ApiError> {
    state.updates.clone().ok_or_else(|| ApiError::new(StatusCode::NOT_FOUND, "update_unavailable"))
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
            tracing::warn!("installazione dell'aggiornamento non riuscita: {e}");
            Err(ApiError::new(StatusCode::BAD_GATEWAY, "install_failed"))
        }
        Err(_) => Err(ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "update_task")),
    }
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
const LOG_IMPORT_MAX_BYTES: usize = 64 * 1024 * 1024;

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
}

/// `AAAAMMGG` o `AAAA-MM-GG` -> `AAAAMMGG`; vuoto = nessun limite; altro = `None` (data non valida).
fn adif_date(v: Option<&str>) -> Option<Option<String>> {
    let v = v.map(str::trim).filter(|v| !v.is_empty());
    let Some(v) = v else { return Some(None) };
    let digits: String = v.chars().filter(|c| *c != '-').collect();
    (digits.len() == 8 && aurora_core::adif::unix_ms(&digits, "").is_some()).then_some(Some(digits))
}

/// Il log interno come file ADIF nel formato di JTDX (`wsjtx_log.adi`), con i QSO tra le date `from` e `to` (estremi compresi, `AAAAMMGG`
/// o `AAAA-MM-GG`, facoltative). Risponde con un allegato `aurora_log.adi` e `x-aurora-qsos` con il numero di QSO.
#[utoipa::path(get, path = "/api/v1/log/export.adi", responses((status = 200, description = "File ADIF come allegato"), (status = 400, description = "Data non valida o intervallo rovesciato (bad_range)"), (status = 502, description = "Log non leggibile (log_read)")))]
async fn log_export(State(state): State<AppState>, _: Authed, axum::extract::Query(p): axum::extract::Query<ExportParams>) -> Result<Response, ApiError> {
    let (Some(from), Some(to)) = (adif_date(p.from.as_deref()), adif_date(p.to.as_deref())) else {
        return Err(ApiError::new(StatusCode::BAD_REQUEST, "bad_range"));
    };
    if let (Some(f), Some(t)) = (&from, &to)
        && f > t
    {
        return Err(ApiError::new(StatusCode::BAD_REQUEST, "bad_range"));
    }
    let station = state.log_station.as_ref().map(|s| s()).unwrap_or_default();
    let book = state.logbook.clone();
    let (text, count) = tokio::task::spawn_blocking(move || book.export_jtdx_adif(&station, from.as_deref(), to.as_deref()))
        .await
        .map_err(|_| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "log_task"))?
        .map_err(|e| {
            tracing::warn!("log non esportato: {e}");
            ApiError::new(StatusCode::BAD_GATEWAY, "log_read")
        })?;
    let mut resp = text.into_response();
    let h = resp.headers_mut();
    h.insert(header::CONTENT_TYPE, HeaderValue::from_static("text/plain; charset=utf-8"));
    h.insert(header::CONTENT_DISPOSITION, HeaderValue::from_static("attachment; filename=\"aurora_log.adi\""));
    if let Ok(v) = HeaderValue::from_str(&count.to_string()) {
        h.insert("x-aurora-qsos", v);
    }
    Ok(resp)
}

/// Aggiunge al log interno i QSO di un file ADIF (corpo della richiesta, per esempio il `wsjtx_log.adi` di JTDX), senza doppioni.
/// Risponde `{read, added, duplicates, skipped}`. Solo dalla rete di casa.
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
    Json(json!({ "open": state.open_access.load(std::sync::atomic::Ordering::SeqCst) }))
}

#[derive(Deserialize)]
struct AccessBody {
    open: bool,
}

/// Accesso senza codice, solo per le prove: si accende e si spegne dal PC, non resta al riavvio.
#[utoipa::path(put, path = "/api/v1/access", responses((status = 200, description = "Impostato"), (status = 403, description = "Solo dal PC (local_only)")))]
async fn access_put(State(state): State<AppState>, Peer(peer): Peer, headers: HeaderMap, Json(body): Json<AccessBody>) -> Result<Json<Value>, ApiError> {
    if !peer.is_some_and(|p| p.ip().is_loopback()) || is_proxied(&headers) {
        return Err(ApiError::new(StatusCode::FORBIDDEN, "local_only"));
    }
    state.open_access.store(body.open, std::sync::atomic::Ordering::SeqCst);
    tracing::warn!("accesso senza codice {}", if body.open { "ACCESO (solo prove, rete di casa)" } else { "spento" });
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
