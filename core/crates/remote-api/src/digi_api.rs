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

/// La pagina aperta (identificativo scelto dalla pagina, uno per finestra o telefono).
#[derive(Deserialize)]
pub(crate) struct PageQuery {
    id: String,
}

/// Battito della pagina `id` (all'apertura e ogni 20 s): finche' una pagina batte l'ascolto non si spegne da solo. Non accende l'ascolto.
#[utoipa::path(post, path = "/api/v1/digi/rx/hold", params(("id" = String, Query, description = "Identificativo della pagina")), responses((status = 200, description = "Pagina contata")))]
pub(crate) async fn digi_rx_hold(State(state): State<AppState>, _: Authed, axum::extract::Query(q): axum::extract::Query<PageQuery>) -> Result<Json<Value>, ApiError> {
    service(&state)?.hold_rx(&q.id);
    Ok(Json(json!({ "ok": true })))
}

/// La pagina `id` e' stata lasciata: se non ne resta nessuna l'ascolto si spegne, salvo «Ascolto acceso all'avvio».
#[utoipa::path(post, path = "/api/v1/digi/rx/release", params(("id" = String, Query, description = "Identificativo della pagina")), responses((status = 200, description = "Pagina tolta")))]
pub(crate) async fn digi_rx_release(State(state): State<AppState>, _: Authed, axum::extract::Query(q): axum::extract::Query<PageQuery>) -> Result<Json<Value>, ApiError> {
    let svc = service(&state)?;
    blocking(move || svc.release_rx(&q.id)).await?;
    Ok(Json(json!({ "ok": true })))
}

#[derive(Deserialize)]
pub(crate) struct TxBody {
    text: String,
    /// Una macro ripremuta: il testo non ancora partito si butta e questo ricomincia da capo (vedi `DigiService::transmit_restart`).
    #[serde(default)]
    restart: bool,
    /// Con `restart`: finito il testo si resta in trasmissione (macro senza `<RX>`).
    #[serde(default)]
    open: bool,
}

/// Trasmette un testo con il modo e la frequenza delle impostazioni.
#[utoipa::path(post, path = "/api/v1/digi/tx", responses((status = 200, description = "Trasmissione partita: stato `tx`"), (status = 400, description = "Testo vuoto o troppo lungo (empty_text, bad_value)"), (status = 409, description = "Occupato o senza trasmettitore (busy, no_transmitter)")))]
pub(crate) async fn digi_tx(State(state): State<AppState>, _: Authed, headers: HeaderMap, Json(body): Json<TxBody>) -> Result<Json<Value>, ApiError> {
    let svc = service(&state)?;
    check_tx_allowed(&state, &headers)?;
    if body.text.chars().count() > TEXT_MAX {
        return Err(api_error(DigiError::BadValue(format!("testo oltre {TEXT_MAX} caratteri"))));
    }
    if body.restart {
        let (status, kept) = blocking(move || svc.transmit_restart(&body.text, body.open)).await?.map_err(api_error)?;
        return Ok(Json(json!({ "tx": status, "kept": kept })));
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

/// «Prova» del WinKeyer: apre la porta delle impostazioni del CW (anche con l'uscita audio) e manda «VVV». Risponde con la versione del firmware.
#[utoipa::path(post, path = "/api/v1/digi/winkeyer/test", responses((status = 200, description = "`{version}`"), (status = 502, description = "WinKeyer non aperto o muto (tx_failed)"), (status = 409, description = "Trasmissione in corso (busy)")))]
pub(crate) async fn digi_winkeyer_test(State(state): State<AppState>, _: Authed, headers: HeaderMap) -> Result<Json<Value>, ApiError> {
    check_tx_allowed(&state, &headers)?;
    let svc = service(&state)?;
    let version = blocking(move || svc.winkeyer_test()).await?.map_err(api_error)?;
    Ok(Json(json!({ "version": version })))
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
    if cmd.is_empty() || cmd.len() > 256 || !cmd.chars().all(|c| c.is_ascii_graphic() || c == ' ') {
        return Err(ApiError::new(StatusCode::BAD_REQUEST, "bad_value").with_message("comando CAT vuoto, oltre 256 caratteri o con caratteri non ammessi".to_string()));
    }
    let cat = state.cat.clone().ok_or_else(|| ApiError::new(StatusCode::NOT_FOUND, "cat_unavailable"))?;
    blocking(move || cat.send_raw_cat(&cmd)).await?.map_err(|e| ApiError::new(StatusCode::CONFLICT, "cat_failed").with_message(e.to_string()))?;
    Ok(Json(json!({ "ok": true })))
}

/// I byte scritti in esadecimale (`FE FE 94`, `0xFE,0x94`, `FEFE94`): `None` se non lo sono o se sono piu' di 64.
pub(crate) fn parse_hex(text: &str) -> Option<Vec<u8>> {
    let clean: String = text.replace("0x", "").replace("0X", "").chars().filter(|c| !matches!(c, ' ' | ',' | ';' | '$' | '-')).collect();
    if clean.is_empty() || clean.len() % 2 != 0 || clean.len() > 128 {
        return None;
    }
    (0..clean.len()).step_by(2).map(|i| u8::from_str_radix(clean.get(i..i + 2)?, 16).ok()).collect()
}

#[derive(Deserialize)]
pub(crate) struct WinKeyerSendBody {
    /// Testo da mandare in CW (`<WKTEXT:...>`).
    #[serde(default)]
    text: Option<String>,
    /// Byte di comando in esadecimale (`<WKHEX:...>`).
    #[serde(default)]
    hex: Option<String>,
}

/// Macro `<WKTEXT:testo>` e `<WKHEX:byte>`: testo o comandi al WinKeyer (si apre sulla porta delle impostazioni del CW).
#[utoipa::path(post, path = "/api/v1/digi/winkeyer/send", responses((status = 200, description = "Mandato"), (status = 400, description = "Ne' testo ne' byte validi (bad_value)"), (status = 409, description = "Trasmissione in corso (busy)"), (status = 502, description = "WinKeyer non aperto (tx_failed)")))]
pub(crate) async fn digi_winkeyer_send(State(state): State<AppState>, _: Authed, headers: HeaderMap, Json(body): Json<WinKeyerSendBody>) -> Result<Json<Value>, ApiError> {
    let text = body.text.map(|t| t.trim().to_string()).filter(|t| !t.is_empty());
    let hex = match body.hex.as_deref().map(str::trim).filter(|h| !h.is_empty()) {
        Some(h) => Some(parse_hex(h).ok_or_else(|| ApiError::new(StatusCode::BAD_REQUEST, "bad_value").with_message("byte esadecimali non validi".to_string()))?),
        None => None,
    };
    if text.as_ref().is_some_and(|t| t.chars().count() > 500) || (text.is_none() && hex.is_none()) {
        return Err(ApiError::new(StatusCode::BAD_REQUEST, "bad_value").with_message("serve un testo (al massimo 500 caratteri) o dei byte".to_string()));
    }
    check_tx_allowed(&state, &headers)?;
    let svc = service(&state)?;
    blocking(move || -> Result<(), DigiError> {
        if let Some(b) = hex {
            svc.winkeyer_raw(&b)?;
        }
        if let Some(t) = text {
            svc.winkeyer_text(&t)?;
        }
        Ok(())
    })
    .await?
    .map_err(api_error)?;
    Ok(Json(json!({ "ok": true })))
}

#[derive(Deserialize)]
pub(crate) struct PlayBody {
    /// Il file `.wav` della cartella dei testi delle macro (`<WAVE:nome>`).
    #[serde(default)]
    wave: Option<String>,
    /// Le cifre DTMF (`<DTMF:...>`).
    #[serde(default)]
    dtmf: Option<String>,
    /// Secondi di portante sulla frequenza di trasmissione (`<TUNE:s>`).
    #[serde(default)]
    tune_s: Option<f32>,
    /// Una registrazione (`<STARTPLAY:nome>`): si cerca nella cartella delle registrazioni, poi in quella dei testi delle macro.
    #[serde(default)]
    play: Option<String>,
}

/// Un nome di file senza percorsi, con `.wav` in fondo se manca.
fn wav_name(name: &str) -> Option<String> {
    let safe = !name.is_empty() && name.len() <= 64 && !name.starts_with('.') && name.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-' | ' '));
    safe.then(|| if name.to_ascii_lowercase().ends_with(".wav") { name.to_string() } else { format!("{name}.wav") })
}

/// Macro `<WAVE:nome>`, `<DTMF:cifre>` e `<TUNE:s>`: un audio in trasmissione. Il file si cerca solo per nome nella cartella dei testi delle
/// macro (al massimo 20 MB). Solo dal PC stesso.
#[utoipa::path(post, path = "/api/v1/digi/play", responses((status = 200, description = "In trasmissione: stato `tx`"), (status = 400, description = "Niente da trasmettere, nome o file non valido (bad_value)"), (status = 403, description = "Solo dal PC (local_only)"), (status = 404, description = "File non trovato (not_found)"), (status = 409, description = "Trasmissione in corso o nessun trasmettitore (busy)")))]
pub(crate) async fn digi_play(State(state): State<AppState>, _: Authed, Peer(peer): Peer, headers: HeaderMap, Json(body): Json<PlayBody>) -> Result<Json<Value>, ApiError> {
    let local = peer.is_some_and(|p| p.ip().is_loopback()) && !is_proxied(&headers);
    if !local {
        return Err(ApiError::new(StatusCode::FORBIDDEN, "local_only"));
    }
    let bad = |m: &str| ApiError::new(StatusCode::BAD_REQUEST, "bad_value").with_message(m.to_string());
    enum Ask {
        Wave(String),
        Dtmf(String),
        Tune(f32),
    }
    let recordings = state.digi.as_ref().map(|d| d.recordings_folder());
    let text_folder = mode_digi::macro_files::text_folder();
    let (wave_name, folders) = match (body.wave.as_deref().map(str::trim).filter(|n| !n.is_empty()), body.play.as_deref().map(str::trim).filter(|n| !n.is_empty())) {
        (Some(n), _) => (Some(n), vec![text_folder.clone()]),
        (None, Some(n)) => (Some(n), vec![recordings, text_folder.clone()]),
        (None, None) => (None, Vec::new()),
    };
    let ask = if let Some(name) = wave_name {
        Ask::Wave(wav_name(name).ok_or_else(|| bad("nome di file non valido"))?)
    } else if let Some(d) = body.dtmf.as_deref().map(str::trim).filter(|d| !d.is_empty()) {
        if d.len() > 64 || mode_digi::macro_audio::dtmf(d).is_empty() {
            return Err(bad("cifre DTMF non valide"));
        }
        Ask::Dtmf(d.to_string())
    } else if let Some(s) = body.tune_s.filter(|s| s.is_finite() && *s > 0.0) {
        Ask::Tune(s.min(60.0))
    } else {
        return Err(bad("niente da trasmettere"));
    };
    check_tx_allowed(&state, &headers)?;
    let svc = service(&state)?;
    let status = blocking(move || -> Result<_, ApiError> {
        let (wave, what) = match ask {
            Ask::Wave(name) => {
                let path = folders.into_iter().flatten().map(|f| f.join(&name)).find(|p| p.is_file()).ok_or_else(|| ApiError::new(StatusCode::NOT_FOUND, "not_found"))?;
                let bytes = match std::fs::metadata(&path) {
                    Ok(m) if m.is_file() && m.len() <= 20_000_000 => std::fs::read(&path).map_err(|_| ApiError::new(StatusCode::NOT_FOUND, "not_found"))?,
                    _ => return Err(ApiError::new(StatusCode::NOT_FOUND, "not_found")),
                };
                (mode_digi::macro_audio::read_wav(&bytes).map_err(|e| ApiError::new(StatusCode::BAD_REQUEST, "bad_value").with_message(e))?, "WAVE")
            }
            Ask::Dtmf(d) => (mode_digi::macro_audio::dtmf(&d), "DTMF"),
            Ask::Tune(s) => (mode_digi::macro_audio::tone(svc.tx_audio_hz(), s), "TUNE"),
        };
        svc.transmit_wave(wave, what).map_err(api_error)
    })
    .await??;
    Ok(Json(json!({ "tx": status })))
}

#[derive(Deserialize)]
pub(crate) struct RecordBody {
    /// `start`, `stop` o `replay`.
    action: String,
    /// Il nome del file (`start`; senza nome: data e ora).
    #[serde(default)]
    name: Option<String>,
    /// Secondi da rileggere (`replay`, 1-60, di base 10).
    #[serde(default)]
    seconds: Option<f32>,
}

/// Macro `<STARTRECORD:nome>`, `<STOPRECORD>`, `<REPLAY:n>`: registra l'audio ricevuto in `recordings` (WAV a 16 bit) o fa ripassare nel
/// ricevitore gli ultimi secondi. Solo dal PC stesso.
#[utoipa::path(post, path = "/api/v1/digi/record", responses((status = 200, description = "`{file}` o `{samples}`"), (status = 400, description = "Azione o nome non validi (bad_value)"), (status = 403, description = "Solo dal PC (local_only)"), (status = 404, description = "Non disponibile (digi_unavailable)")))]
pub(crate) async fn digi_record(State(state): State<AppState>, _: Authed, Peer(peer): Peer, headers: HeaderMap, Json(body): Json<RecordBody>) -> Result<Json<Value>, ApiError> {
    let local = peer.is_some_and(|p| p.ip().is_loopback()) && !is_proxied(&headers);
    if !local {
        return Err(ApiError::new(StatusCode::FORBIDDEN, "local_only"));
    }
    let bad = |m: &str| ApiError::new(StatusCode::BAD_REQUEST, "bad_value").with_message(m.to_string());
    let action = body.action.trim().to_ascii_lowercase();
    let name = match body.name.as_deref().map(str::trim).filter(|n| !n.is_empty()) {
        Some(n) => Some(wav_name(n).ok_or_else(|| bad("nome di file non valido"))?),
        None => None,
    };
    if !matches!(action.as_str(), "start" | "stop" | "replay") {
        return Err(bad("azione non valida: start, stop o replay"));
    }
    let svc = service(&state)?;
    let out = blocking(move || -> Result<Value, DigiError> {
        Ok(match action.as_str() {
            "start" => {
                let name = name.unwrap_or_else(|| format!("rx-{}.wav", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_secs())));
                json!({ "file": svc.start_record(&name)?.display().to_string() })
            }
            "stop" => json!({ "file": svc.stop_record()?.map(|p| p.display().to_string()) }),
            _ => json!({ "samples": svc.replay(body.seconds.filter(|s| s.is_finite()).unwrap_or(10.0)) }),
        })
    })
    .await?
    .map_err(api_error)?;
    Ok(Json(out))
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

/// Svuota tutta la banda: i canali, il loro storico e i nominativi sentiti ripartono da zero (il pulsante «Pulisci» del pannello Segnali).
#[utoipa::path(post, path = "/api/v1/digi/band/clear", responses((status = 200, description = "Svuotata: `{ok}`"), (status = 404, description = "Non disponibile (digi_unavailable)")))]
pub(crate) async fn digi_band_clear(State(state): State<AppState>, _: Authed) -> Result<Json<Value>, ApiError> {
    service(&state)?.clear_band();
    Ok(Json(json!({ "ok": true })))
}

/// Il corpo di `PUT /digi/notches`.
#[derive(Deserialize)]
pub(crate) struct NotchesBody {
    notches: Vec<mode_digi::notch::Notch>,
}

/// I notch della ricezione (PSK, RTTY, CW): `{notches: [{hz, width_hz}]}`. Non si salvano: all'avvio e a ogni cambio di frequenza della radio sono vuoti.
#[utoipa::path(get, path = "/api/v1/digi/notches", responses((status = 200, description = "`{notches}`"), (status = 404, description = "Non disponibile (digi_unavailable)")))]
pub(crate) async fn digi_notches_get(State(state): State<AppState>, _: Authed) -> Result<Json<Value>, ApiError> {
    Ok(Json(json!({ "notches": service(&state)?.notches() })))
}

/// Sostituisce i notch: corpo `{notches: [{hz (50-5950), width_hz (10-100, 30 se manca)}]}`; `[]` li toglie tutti. Risponde `{notches}`.
#[utoipa::path(put, path = "/api/v1/digi/notches", responses((status = 200, description = "`{notches}`"), (status = 400, description = "Valori non validi (bad_value)"), (status = 404, description = "Non disponibile (digi_unavailable)")))]
pub(crate) async fn digi_notches_put(State(state): State<AppState>, _: Authed, Json(body): Json<NotchesBody>) -> Result<Json<Value>, ApiError> {
    let svc = service(&state)?;
    svc.set_notches(body.notches).map_err(api_error)?;
    Ok(Json(json!({ "notches": svc.notches() })))
}

fn name_owned(name: &str) -> String {
    name.trim().to_string()
}

/// Il corpo di `PUT /digi/extra-rx`.
#[derive(Deserialize)]
pub(crate) struct ExtraRxBody {
    extra: Vec<mode_digi::service::ExtraRx>,
}

/// Le finestre RX in piu' (PSK, RTTY, CW): `{extra: [{id, freq_hz}]}`. Non si salvano.
#[utoipa::path(get, path = "/api/v1/digi/extra-rx", responses((status = 200, description = "`{extra}`"), (status = 404, description = "Non disponibile (digi_unavailable)")))]
pub(crate) async fn digi_extra_rx_get(State(state): State<AppState>, _: Authed) -> Result<Json<Value>, ApiError> {
    Ok(Json(json!({ "extra": service(&state)?.extra_rx() })))
}

/// Sostituisce le finestre RX in piu': corpo `{extra: [{id (> 0, diversi), freq_hz (100-5900)}]}`, al massimo 4; `[]` le chiude tutte.
/// Il testo di ognuna arriva negli eventi `rx_text` con `rx_id`. Risponde `{extra}`.
#[utoipa::path(put, path = "/api/v1/digi/extra-rx", responses((status = 200, description = "`{extra}`"), (status = 400, description = "Valori non validi (bad_value)"), (status = 404, description = "Non disponibile (digi_unavailable)")))]
pub(crate) async fn digi_extra_rx_put(State(state): State<AppState>, _: Authed, Json(body): Json<ExtraRxBody>) -> Result<Json<Value>, ApiError> {
    let svc = service(&state)?;
    svc.set_extra_rx(body.extra).map_err(api_error)?;
    Ok(Json(json!({ "extra": svc.extra_rx() })))
}
