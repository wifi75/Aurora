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
use serde::Deserialize;
use serde_json::{json, Value};
use utoipa::OpenApi;

use crate::{policy::transport_of, ws, AppState};

const COOKIE_NAME: &str = "aurora_session";
const COOKIE_MAX_AGE_SECS: u64 = 31_536_000;

#[derive(OpenApi)]
#[openapi(paths(health, pair_start, pair_finish, devices, revoke_device, halt_tx, reply, ws::stream))]
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
        if let Some(secs) = self.retry_after {
            if let Ok(v) = HeaderValue::from_str(&secs.to_string()) {
                resp.headers_mut().insert(header::RETRY_AFTER, v);
            }
        }
        resp
    }
}

fn session_token(headers: &HeaderMap) -> Option<String> {
    let cookies = headers.get(header::COOKIE)?.to_str().ok()?;
    cookies
        .split(';')
        .filter_map(|c| c.trim().split_once('='))
        .find(|(name, _)| *name == COOKIE_NAME)
        .map(|(_, v)| v.to_string())
}

/// Dispositivo autenticato dal cookie di sessione.
pub struct Authed(pub DeviceId);

impl FromRequestParts<AppState> for Authed {
    type Rejection = ApiError;

    async fn from_request_parts(parts: &mut Parts, state: &AppState) -> Result<Self, ApiError> {
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
    body: Option<Json<PairStartBody>>,
) -> Result<Json<Value>, ApiError> {
    if !peer.is_some_and(|p| p.ip().is_loopback()) {
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
    dispatch(&state, Command::HaltTx)
}

#[derive(Deserialize)]
struct ReplyBody {
    decode_id: u64,
}

#[utoipa::path(post, path = "/api/v1/commands/reply", responses((status = 200, description = "Risposta inviata al motore"), (status = 403, description = "Trasmissione non consentita da questa via"), (status = 404, description = "Decode sconosciuto")))]
async fn reply(
    State(state): State<AppState>,
    _: Authed,
    headers: HeaderMap,
    Json(body): Json<ReplyBody>,
) -> Result<Json<Value>, ApiError> {
    if !state.policy.allows(transport_of(&headers)) {
        return Err(ApiError::new(StatusCode::FORBIDDEN, "tx_not_allowed_on_transport"));
    }
    if !state.knows_decode(body.decode_id) {
        return Err(ApiError::new(StatusCode::NOT_FOUND, "unknown_decode"));
    }
    dispatch(&state, Command::Reply { decode_id: body.decode_id })
}

async fn openapi() -> Json<Value> {
    Json(serde_json::to_value(ApiDoc::openapi()).unwrap_or(Value::Null))
}
