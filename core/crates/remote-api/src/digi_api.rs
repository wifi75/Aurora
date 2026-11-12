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

#[derive(Deserialize, Default)]
pub(crate) struct OpenBody {
    #[serde(default)]
    text: String,
}

/// Apre la trasmissione (levetta RX/TX): si resta in trasmissione finche' non si chiude; `text` e' il testo gia' scritto (anche vuoto).
#[utoipa::path(post, path = "/api/v1/digi/tx/open", responses((status = 200, description = "Trasmissione aperta: stato `tx`"), (status = 409, description = "Occupato o senza trasmettitore (busy, no_transmitter)")))]
pub(crate) async fn digi_tx_open(State(state): State<AppState>, _: Authed, headers: HeaderMap, body: Option<Json<OpenBody>>) -> Result<Json<Value>, ApiError> {
    let svc = service(&state)?;
    check_tx_allowed(&state, &headers)?;
    let text = body.map(|b| b.0.text).unwrap_or_default();
    if text.chars().count() > TEXT_MAX {
        return Err(api_error(DigiError::BadValue(format!("testo oltre {TEXT_MAX} caratteri"))));
    }
    let status = blocking(move || svc.tx_open(&text)).await?.map_err(api_error)?;
    Ok(Json(json!({ "tx": status })))
}

/// Testo scritto durante la trasmissione: si accoda a quello che sta partendo.
#[utoipa::path(post, path = "/api/v1/digi/tx/type", responses((status = 200, description = "Accodato: stato `tx`"), (status = 400, description = "Nessuna trasmissione in corso o testo troppo lungo (bad_value)")))]
pub(crate) async fn digi_tx_type(State(state): State<AppState>, _: Authed, headers: HeaderMap, Json(body): Json<TxBody>) -> Result<Json<Value>, ApiError> {
    let svc = service(&state)?;
    check_tx_allowed(&state, &headers)?;
    if body.text.chars().count() > TEXT_MAX {
        return Err(api_error(DigiError::BadValue(format!("testo oltre {TEXT_MAX} caratteri"))));
    }
    let status = blocking(move || svc.tx_type(&body.text)).await?.map_err(api_error)?;
    Ok(Json(json!({ "tx": status })))
}

#[derive(Deserialize)]
pub(crate) struct UnsendBody {
    count: usize,
}

/// Toglie fino a `count` caratteri non ancora partiti dalla fine del testo in coda (Backspace): `{removed}`.
#[utoipa::path(post, path = "/api/v1/digi/tx/unsend", responses((status = 200, description = "`{removed}`: i caratteri tolti")))]
pub(crate) async fn digi_tx_unsend(State(state): State<AppState>, _: Authed, Json(body): Json<UnsendBody>) -> Result<Json<Value>, ApiError> {
    let svc = service(&state)?;
    let removed = blocking(move || svc.tx_unsend(body.count.min(TEXT_MAX))).await?;
    Ok(Json(json!({ "removed": removed })))
}

#[derive(Deserialize)]
pub(crate) struct ReplaceBody {
    from: usize,
    text: String,
}

/// Sostituisce il testo in coda dalla posizione `from` (contata dal primo carattere della trasmissione) con `text`, senza fermare la
/// trasmissione: `{replaced}` falso se `from` cade su caratteri gia' modulati.
#[utoipa::path(post, path = "/api/v1/digi/tx/replace", responses((status = 200, description = "`{replaced}`"), (status = 400, description = "Nessuna trasmissione in corso o testo troppo lungo (bad_value)")))]
pub(crate) async fn digi_tx_replace(State(state): State<AppState>, _: Authed, headers: HeaderMap, Json(body): Json<ReplaceBody>) -> Result<Json<Value>, ApiError> {
    let svc = service(&state)?;
    check_tx_allowed(&state, &headers)?;
    if body.text.chars().count() > TEXT_MAX {
        return Err(api_error(DigiError::BadValue(format!("testo oltre {TEXT_MAX} caratteri"))));
    }
    let replaced = blocking(move || svc.tx_replace(body.from, &body.text)).await?.map_err(api_error)?;
    Ok(Json(json!({ "replaced": replaced })))
}

/// Chiude la trasmissione aperta: finito il testo gia' scritto si torna in ricezione. Sempre consentito.
#[utoipa::path(post, path = "/api/v1/digi/tx/close", responses((status = 200, description = "Chiusa")))]
pub(crate) async fn digi_tx_close(State(state): State<AppState>, _: Authed) -> Result<Json<Value>, ApiError> {
    let svc = service(&state)?;
    blocking(move || svc.tx_close()).await?;
    Ok(Json(json!({ "ok": true })))
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
    /// La cartella dei file di macro (obbligatoria con `replace` e `add`).
    #[serde(default)]
    folder: Option<String>,
    /// `replace` (di base), `add` o `defaults`.
    #[serde(default)]
    how: mode_digi::ImportHow,
}

/// Importa le macro (`Default.mc` e i file per modo) dalla cartella dell'utente, oppure (`defaults`) rimette quelle di Aurora. Solo dal PC stesso.
#[utoipa::path(post, path = "/api/v1/digi/import-macros", responses((status = 200, description = "`{imported: {folder, default_count, default_file, modes, skipped}, ...snapshot}`"), (status = 400, description = "Cartella non trovata o senza file di macro (bad_value)"), (status = 403, description = "Solo dal PC (local_only)")))]
pub(crate) async fn digi_import_macros(State(state): State<AppState>, _: Authed, Peer(peer): Peer, headers: HeaderMap, body: Option<Json<ImportBody>>) -> Result<Json<Value>, ApiError> {
    let svc = service(&state)?;
    let local = peer.is_some_and(|p| p.ip().is_loopback()) && !is_proxied(&headers);
    if !local {
        return Err(ApiError::new(StatusCode::FORBIDDEN, "local_only"));
    }
    let how = body.as_ref().map(|b| b.0.how).unwrap_or_default();
    let folder = body.and_then(|b| b.0.folder).filter(|f| !f.trim().is_empty()).map(PathBuf::from);
    let s = Arc::clone(&svc);
    let report = blocking(move || s.import_macros(folder.as_deref(), how)).await?.map_err(api_error)?;
    Ok(Json(json!({ "imported": report, "snapshot": svc.snapshot() })))
}

#[derive(Deserialize)]
pub(crate) struct CatBody {
    cmd: String,
}

/// Manda alla radio una stringa CAT cosi' com'e' (macro `<CATCMD:...>`, per esempio `AN1;`). Solo dal PC stesso, solo con Hamlib.
#[utoipa::path(post, path = "/api/v1/digi/cat", responses((status = 200, description = "Mandata"), (status = 400, description = "Comando vuoto, troppo lungo o con caratteri non ammessi (bad_value)"), (status = 403, description = "Solo dal PC (local_only)"), (status = 409, description = "Radio non pronta o sistema di controllo senza comandi diretti (cat_failed)")))]
pub(crate) async fn digi_cat(State(state): State<AppState>, _: Authed, Peer(peer): Peer, headers: HeaderMap, Json(body): Json<CatBody>) -> Result<Json<Value>, ApiError> {
    let local = peer.is_some_and(|p| p.ip().is_loopback()) && !is_proxied(&headers);
    if !local {
        return Err(ApiError::new(StatusCode::FORBIDDEN, "local_only"));
    }
    let cmd = body.cmd.trim().to_string();
    if cmd.is_empty() || cmd.len() > 64 || !cmd.chars().all(|c| c.is_ascii_graphic() || c == ' ') {
        return Err(ApiError::new(StatusCode::BAD_REQUEST, "bad_value").with_message("comando CAT vuoto, oltre 64 caratteri o con caratteri non ammessi".to_string()));
    }
    let cat = state.cat.clone().ok_or_else(|| ApiError::new(StatusCode::NOT_FOUND, "cat_unavailable"))?;
    blocking(move || cat.send_raw_cat(&cmd)).await?.map_err(|e| ApiError::new(StatusCode::CONFLICT, "cat_failed").with_message(e.to_string()))?;
    Ok(Json(json!({ "ok": true })))
}

#[derive(Deserialize)]
pub(crate) struct TextFileQuery {
    name: String,
}

/// Il testo di un file della cartella dei testi delle macro (`<TEXTFILE:nome>`, `%APPDATA%\Aurora\testi`): solo il nome del file, niente percorsi, al massimo 64 KB. Solo dal PC stesso.
#[utoipa::path(get, path = "/api/v1/digi/textfile", responses((status = 200, description = "`{text}`"), (status = 400, description = "Nome non valido (bad_value)"), (status = 403, description = "Solo dal PC (local_only)"), (status = 404, description = "File non trovato (not_found)")))]
pub(crate) async fn digi_textfile(_: Authed, Peer(peer): Peer, headers: HeaderMap, axum::extract::Query(q): axum::extract::Query<TextFileQuery>) -> Result<Json<Value>, ApiError> {
    let local = peer.is_some_and(|p| p.ip().is_loopback()) && !is_proxied(&headers);
    if !local {
        return Err(ApiError::new(StatusCode::FORBIDDEN, "local_only"));
    }
    let name = q.name.trim();
    let safe = !name.is_empty() && name.len() <= 64 && !name.starts_with('.') && name.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-' | ' '));
    if !safe {
        return Err(ApiError::new(StatusCode::BAD_REQUEST, "bad_value").with_message("nome di file non valido".to_string()));
    }
    let folder = mode_digi::macro_files::text_folder().ok_or_else(|| ApiError::new(StatusCode::NOT_FOUND, "not_found"))?;
    let text = blocking(move || {
        let path = folder.join(name_owned(&q.name));
        match std::fs::metadata(&path) {
            Ok(m) if m.is_file() && m.len() <= 65_536 => std::fs::read(&path).ok().map(|b| b.iter().map(|&c| char::from(c)).collect::<String>()),
            _ => None,
        }
    })
    .await?;
    text.map(|t| Json(json!({ "text": t }))).ok_or_else(|| ApiError::new(StatusCode::NOT_FOUND, "not_found"))
}

/// I segnali di tutta la banda (CW, PSK31, PSK63) con lo storico di 60 s di ciascuno: frequenza, velocita', S/N, testo con i caratteri incerti, nominativi.
#[utoipa::path(get, path = "/api/v1/digi/band", responses((status = 200, description = "`{signals}`"), (status = 404, description = "Non disponibile (digi_unavailable)")))]
pub(crate) async fn digi_band(State(state): State<AppState>, _: Authed) -> Result<Json<Value>, ApiError> {
    let svc = service(&state)?;
    Ok(Json(json!({ "signals": svc.band_signals() })))
}

fn name_owned(name: &str) -> String {
    name.trim().to_string()
}
