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
#[openapi(paths(output_level_get, output_level_put, controls_get, control_press, access_get, access_put, pairing_get, pairing_put, frequencies_get, frequencies_put, schedule_get, schedule_put, stats, log4om, change_band, change_mode, rx_frequency, free_text, call_cq, tx_frequency, station_info, health, pair_start, pair_finish, devices, revoke_device, halt_tx, reply, automation_get, automation_put, ws::stream))]
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
        .route("/api/v1/commands/mode", post(change_mode))
        .route("/api/v1/commands/rx-frequency", post(rx_frequency))
        .route("/api/v1/output-level", get(output_level_get).put(output_level_put))
        .route("/api/v1/controls", get(controls_get))
        .route("/api/v1/commands/control", post(control_press))
        .route("/api/v1/frequencies", get(frequencies_get).put(frequencies_put))
        .route("/api/v1/pairing-settings", get(pairing_get).put(pairing_put))
        .route("/api/v1/access", get(access_get).put(access_put))
        .route("/api/v1/schedule", get(schedule_get).put(schedule_put))
        .route("/api/v1/automation", get(automation_get).put(automation_put))
        .route("/api/v1/station-info", get(station_info))
        .route("/api/v1/stats", get(stats))
        .route("/api/v1/log4om", get(log4om))
        .route("/api/v1/stream", get(ws::stream))
        .route("/api/v1/openapi.json", get(openapi))
        .with_state(state)
}

pub struct ApiError {
    status: StatusCode,
    code: &'static str,
    retry_after: Option<u64>,
}

impl ApiError {
    fn new(status: StatusCode, code: &'static str) -> Self {
        ApiError { status, code, retry_after: None }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let mut resp = (self.status, Json(json!({ "error": self.code }))).into_response();
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
fn is_proxied(headers: &HeaderMap) -> bool {
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
struct Peer(Option<SocketAddr>);

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
    let rig = state.rig.clone().ok_or_else(|| ApiError::new(StatusCode::SERVICE_UNAVAILABLE, "rig_unavailable"))?;
    let mode = body.mode.clone().unwrap_or_else(|| state.current_mode());
    // il blocco chiude il lock prima di qualunque attesa
    let found = {
        let table = state.frequencies.lock().unwrap_or_else(|p| p.into_inner());
        table.frequency(&body.band, &mode)
    };
    let hz = found.ok_or_else(|| ApiError::new(StatusCode::BAD_REQUEST, "bad_band"))?;
    if state.current_status().is_some_and(|s| s.transmitting) {
        return Err(ApiError::new(StatusCode::CONFLICT, "transmitting"));
    }
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

const MAX_ATTENUATION_DB: f64 = 45.0;

/// Livello audio di uscita di JTDX (cursore Pwr) come attenuazione in dB: 0 = livello massimo.
#[utoipa::path(get, path = "/api/v1/output-level", responses((status = 200, description = "Attenuazione in dB e limiti"), (status = 502, description = "JTDX non ha risposto (rig_error)"), (status = 503, description = "Comando non disponibile (rig_unavailable)")))]
async fn output_level_get(State(state): State<AppState>, _: Authed) -> Result<Json<Value>, ApiError> {
    let rig = state.rig.clone().ok_or_else(|| ApiError::new(StatusCode::SERVICE_UNAVAILABLE, "rig_unavailable"))?;
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
    let rig = state.rig.clone().ok_or_else(|| ApiError::new(StatusCode::SERVICE_UNAVAILABLE, "rig_unavailable"))?;
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

/// Stato degli interruttori di JTDX (AutoSeq, 1 QSO, Abilita Tx...).
#[utoipa::path(get, path = "/api/v1/controls", responses((status = 200, description = "Stato di ogni interruttore"), (status = 502, description = "JTDX non ha risposto (rig_error)"), (status = 503, description = "Comando non disponibile (rig_unavailable)")))]
async fn controls_get(State(state): State<AppState>, _: Authed) -> Result<Json<Value>, ApiError> {
    let rig = state.rig.clone().ok_or_else(|| ApiError::new(StatusCode::SERVICE_UNAVAILABLE, "rig_unavailable"))?;
    match tokio::task::spawn_blocking(move || rig.controls()).await {
        Ok(Ok(list)) => {
            let map: serde_json::Map<String, Value> = list.into_iter().map(|(id, on)| (id, Value::Bool(on))).collect();
            Ok(Json(json!({ "controls": map })))
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
    let rig = state.rig.clone().ok_or_else(|| ApiError::new(StatusCode::SERVICE_UNAVAILABLE, "rig_unavailable"))?;
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
struct RxFrequencyBody {
    hz: u32,
}

/// Sposta la frequenza di ricezione (barra verde). Non fa trasmettere, quindi vale anche in trasmissione.
#[utoipa::path(post, path = "/api/v1/commands/rx-frequency", responses((status = 200, description = "Frequenza Rx impostata"), (status = 400, description = "Fuori dalla banda audio (100-3000 Hz)"), (status = 502, description = "JTDX non ha risposto (rig_error)"), (status = 503, description = "Comando non disponibile (rig_unavailable)")))]
async fn rx_frequency(State(state): State<AppState>, _: Authed, Json(body): Json<RxFrequencyBody>) -> Result<Json<Value>, ApiError> {
    let rig = state.rig.clone().ok_or_else(|| ApiError::new(StatusCode::SERVICE_UNAVAILABLE, "rig_unavailable"))?;
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
    let rig = state.rig.clone().ok_or_else(|| ApiError::new(StatusCode::SERVICE_UNAVAILABLE, "rig_unavailable"))?;
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
    let dial = {
        let table = state.frequencies.lock().unwrap_or_else(|p| p.into_inner());
        state.current_status().filter(|s| s.dial_hz > 0).and_then(|s| table.band_of(s.dial_hz)).and_then(|band| table.frequency(&band, &mode))
    };
    let work = move || -> Result<(), rig_control::RigError> {
        rig.set_mode(&mode)?;
        if let Some(hz) = dial {
            rig.set_frequency(hz)?;
        }
        Ok(())
    };
    match tokio::task::spawn_blocking(work).await {
        Ok(Ok(())) => Ok(Json(json!({ "ok": true, "mode": shown, "dial_hz": dial }))),
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
    Json((*state.station_info).clone())
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
