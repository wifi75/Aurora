//! Le rotte del CAT: stato, configurazione e profili della radio, collegamento e scollegamento, elenco dei modelli di Hamlib, porte seriali e
//! rilevamento passivo. Le modifiche (salvare, collegare, scollegare, profili, rilevamento) si fanno solo dal PC (loopback, mai da un proxy):
//! chi comanda la radio puo' trasmettere. Le letture bastano al dispositivo associato.

use std::sync::Arc;

use axum::{
    extract::State,
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use rig_control::{
    cat::{CatError, CatService},
    cat_config::RigConfig,
    detect,
    extras::ExtraItem,
    hamlib::RigModel,
    Rig,
};
use serde::Deserialize;
use serde_json::{json, Value};

use crate::{
    routes::{is_proxied, Authed, Peer},
    AppState,
};

/// Errore delle rotte del CAT: codice breve e testo (il testo e' gia' tradotto per l'utente).
pub(crate) struct CatApiError {
    status: StatusCode,
    code: &'static str,
    message: String,
}

impl CatApiError {
    fn new(status: StatusCode, code: &'static str) -> CatApiError {
        CatApiError { status, code, message: String::new() }
    }
}

impl From<CatError> for CatApiError {
    fn from(e: CatError) -> CatApiError {
        let status = match e.code {
            "no_such_profile" => StatusCode::NOT_FOUND,
            "rigctld_missing" => StatusCode::CONFLICT,
            "write_failed" => StatusCode::INTERNAL_SERVER_ERROR,
            _ => StatusCode::BAD_REQUEST,
        };
        CatApiError { status, code: e.code, message: e.message }
    }
}

impl IntoResponse for CatApiError {
    fn into_response(self) -> Response {
        (self.status, Json(json!({ "error": self.code, "message": self.message }))).into_response()
    }
}

fn service(state: &AppState) -> Result<Arc<CatService>, CatApiError> {
    state.cat.clone().ok_or_else(|| CatApiError::new(StatusCode::NOT_FOUND, "cat_unavailable"))
}

/// Solo dal PC: connessione diretta da loopback, senza intestazioni di proxy.
fn require_local(peer: Option<std::net::SocketAddr>, headers: &HeaderMap) -> Result<(), CatApiError> {
    if peer.is_some_and(|p| p.ip().is_loopback()) && !is_proxied(headers) {
        Ok(())
    } else {
        Err(CatApiError::new(StatusCode::FORBIDDEN, "local_only"))
    }
}

async fn blocking<T: Send + 'static>(work: impl FnOnce() -> T + Send + 'static) -> Result<T, CatApiError> {
    tokio::task::spawn_blocking(work).await.map_err(|_| CatApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "cat_task"))
}

/// Stato, configurazione e profili.
pub(crate) async fn cat_get(State(state): State<AppState>, _: Authed) -> Result<Json<Value>, CatApiError> {
    let cat = service(&state)?;
    Ok(Json(json!(blocking(move || cat.snapshot()).await?)))
}

/// Salva la configurazione e la applica (se la radio era collegata si ricollega con i valori nuovi).
pub(crate) async fn cat_put(State(state): State<AppState>, _: Authed, Peer(peer): Peer, headers: HeaderMap, Json(config): Json<RigConfig>) -> Result<Json<Value>, CatApiError> {
    let cat = service(&state)?;
    require_local(peer, &headers)?;
    let snapshot = blocking(move || cat.apply_config(config).map(|_| cat.snapshot())).await??;
    Ok(Json(json!(snapshot)))
}

pub(crate) async fn cat_connect(State(state): State<AppState>, _: Authed, Peer(peer): Peer, headers: HeaderMap) -> Result<Json<Value>, CatApiError> {
    let cat = service(&state)?;
    require_local(peer, &headers)?;
    Ok(Json(json!(blocking(move || cat.connect().map(|_| cat.snapshot())).await??)))
}

pub(crate) async fn cat_disconnect(State(state): State<AppState>, _: Authed, Peer(peer): Peer, headers: HeaderMap) -> Result<Json<Value>, CatApiError> {
    let cat = service(&state)?;
    require_local(peer, &headers)?;
    Ok(Json(json!(blocking(move || {
        cat.disconnect();
        cat.snapshot()
    })
    .await?)))
}

/// Ripristina Hamlib (Windows): riscarica il pacchetto ufficiale, ne controlla l'impronta e lo copia nella cartella dei dati dell'utente.
/// Solo dal PC. Risponde con lo stato del CAT riletto, cosi' l'interfaccia vede subito `rigctld_found`.
pub(crate) async fn cat_hamlib_restore(State(state): State<AppState>, _: Authed, Peer(peer): Peer, headers: HeaderMap) -> Result<Json<Value>, CatApiError> {
    let cat = service(&state)?;
    require_local(peer, &headers)?;
    if !rig_control::hamlib_restore::available() {
        return Err(CatApiError { status: StatusCode::CONFLICT, code: "restore_unavailable", message: "su questo sistema Hamlib si installa dal gestore dei pacchetti".to_string() });
    }
    let done = blocking(move || rig_control::hamlib_restore::restore().map(|_| cat.snapshot())).await?;
    match done {
        Ok(snapshot) => Ok(Json(json!(snapshot))),
        Err(message) => Err(CatApiError { status: StatusCode::BAD_GATEWAY, code: "restore_failed", message }),
    }
}

#[derive(Deserialize)]
pub(crate) struct ProfileBody {
    /// `save`, `load`, `new` o `delete`.
    action: String,
    #[serde(default)]
    name: Option<String>,
}

/// Profili con nome: salva la configurazione viva, carica (scollega e non collega), nuovo (con le impostazioni di base), elimina.
pub(crate) async fn cat_profiles(State(state): State<AppState>, _: Authed, Peer(peer): Peer, headers: HeaderMap, Json(body): Json<ProfileBody>) -> Result<Json<Value>, CatApiError> {
    let cat = service(&state)?;
    require_local(peer, &headers)?;
    let name = body.name.unwrap_or_default();
    let action = body.action;
    let snapshot = blocking(move || match action.as_str() {
        "save" => cat.save_profile(&name),
        "load" => cat.load_profile(&name),
        "new" => cat.new_profile(Some(&name).filter(|n| !n.trim().is_empty()).map(String::as_str)),
        "delete" => cat.delete_profile(&name),
        _ => Err(CatError { code: "bad_action", message: "azione non valida".into() }),
    })
    .await??;
    Ok(Json(json!(snapshot)))
}

#[derive(Deserialize)]
pub(crate) struct ExtraBody {
    /// `nb`, `nr`, `notch`, `atu`, `preamp`, `att`, `agc`, `vfo`, `rit` o `xit`.
    item: String,
}

/// Il clic su un indicatore del display esteso: cambia quella impostazione sulla radio (NB, NR, notch, ATU: inverte; preamplificatore,
/// attenuatore, AGC: passa al valore successivo; VFO: passa all'altro; RIT/XIT: azzera) e risponde con i dati riletti. Solo dal PC e mai
/// mentre la radio trasmette.
pub(crate) async fn cat_extra(State(state): State<AppState>, _: Authed, Peer(peer): Peer, headers: HeaderMap, Json(body): Json<ExtraBody>) -> Result<Json<Value>, CatApiError> {
    let cat = service(&state)?;
    require_local(peer, &headers)?;
    let item = ExtraItem::parse(body.item.trim()).ok_or_else(|| CatApiError { status: StatusCode::BAD_REQUEST, code: "bad_item", message: "impostazione sconosciuta".into() })?;
    let result = blocking(move || {
        // con TCI la portante di accordo accesa puo' far dire alla SDR che trasmette: TUNE (o ATU) per spegnerla resta permesso
        let stops_tune = matches!(item, ExtraItem::Tune | ExtraItem::Atu) && cat.tci_tuning();
        if cat.transmitting() && !stops_tune {
            return Err(CatApiError { status: StatusCode::CONFLICT, code: "transmitting", message: "La radio sta trasmettendo: aspetta la fine della trasmissione.".into() });
        }
        cat.set_extra(item).map_err(|e| {
            let lower = e.0.to_lowercase();
            let (status, code) = if lower.contains("non collegata") || lower.contains("nessuna radio collegata") {
                (StatusCode::CONFLICT, "not_connected")
            } else if lower.contains("trasmettendo") {
                (StatusCode::CONFLICT, "transmitting")
            } else if lower.contains("non supporta") || lower.contains("non supportato") || lower.contains("non sa dare") || lower.contains("non si cambiano") {
                (StatusCode::NOT_IMPLEMENTED, "unsupported")
            } else {
                (StatusCode::BAD_GATEWAY, "rig_error")
            };
            CatApiError { status, code, message: e.0 }
        })?;
        Ok(cat.status().rig.map(|r| r.extras))
    })
    .await??;
    Ok(Json(json!({ "ok": true, "item": body.item.trim(), "extras": result })))
}

#[derive(Deserialize)]
pub(crate) struct SplitDxBody {
    /// Ricezione (VFO A): la frequenza dello spot, in Hz.
    rx_hz: u64,
    /// Trasmissione (VFO B), in Hz: ricezione piu' lo spostamento del commento («UP 5») o la frequenza di «QSX».
    tx_hz: u64,
}

/// Lo Split DX come lo vede l'interfaccia.
fn split_dx_json(split: Option<rig_control::cat::DxSplit>) -> Value {
    match split {
        Some(s) => json!({ "active": true, "rx_hz": s.rx_hz, "tx_hz": s.tx_hz }),
        None => json!({ "active": false }),
    }
}

/// Un errore della radio sullo Split DX con un codice che l'interfaccia sa tradurre.
fn split_dx_error(e: rig_control::RigError) -> CatApiError {
    let lower = e.0.to_lowercase();
    let (status, code) = if lower.contains("non collegata") || lower.contains("nessuna radio") {
        (StatusCode::CONFLICT, "not_connected")
    } else if lower.contains("trasmettendo") {
        (StatusCode::CONFLICT, "transmitting")
    } else if lower.contains("non supportat") || lower.contains("esiste solo su windows") {
        (StatusCode::NOT_IMPLEMENTED, "unsupported")
    } else if lower.contains("non valid") || lower.contains("fuori dalla banda") {
        (StatusCode::BAD_REQUEST, "bad_split")
    } else {
        (StatusCode::BAD_GATEWAY, "rig_error")
    };
    CatApiError { status, code, message: e.0 }
}

/// Lo Split DX acceso adesso (per l'indicatore del cluster).
pub(crate) async fn cat_split_dx_get(State(state): State<AppState>, _: Authed) -> Result<Json<Value>, CatApiError> {
    let cat = service(&state)?;
    Ok(Json(split_dx_json(cat.dx_split())))
}

/// Doppio clic su uno spot del cluster con «UP 5», «QSX 14.030»...: la radio riceve sulla frequenza dello spot (VFO A) e trasmette su `tx_hz`
/// (VFO B) con lo split acceso. Vale come un cambio di frequenza (lo puo' chiedere anche il telefono associato); mai in trasmissione.
pub(crate) async fn cat_split_dx_set(State(state): State<AppState>, _: Authed, Json(body): Json<SplitDxBody>) -> Result<Json<Value>, CatApiError> {
    let cat = service(&state)?;
    if state.current_status().is_some_and(|s| s.transmitting) {
        return Err(CatApiError { status: StatusCode::CONFLICT, code: "transmitting", message: "La radio sta trasmettendo: aspetta la fine della trasmissione.".into() });
    }
    crate::routes::halt_for_band_change(&state, body.rx_hz);
    let (rx, tx) = (body.rx_hz, body.tx_hz);
    let split = blocking(move || cat.set_dx_split(rx, tx)).await?.map_err(split_dx_error)?;
    Ok(Json(json!({ "ok": true, "split": split_dx_json(Some(split)) })))
}

/// «Split DX: spegni»: la radio torna in simplex sulla frequenza di ricezione.
pub(crate) async fn cat_split_dx_clear(State(state): State<AppState>, _: Authed) -> Result<Json<Value>, CatApiError> {
    let cat = service(&state)?;
    let was_on = blocking(move || cat.clear_dx_split()).await?.map_err(split_dx_error)?;
    Ok(Json(json!({ "ok": true, "was_on": was_on, "split": split_dx_json(None) })))
}

/// I modelli di Hamlib (da `rigctl -l`).
pub(crate) async fn cat_models(State(state): State<AppState>, _: Authed) -> Result<Json<Value>, CatApiError> {
    let cat = service(&state)?;
    let models = blocking(move || cat.models()).await??;
    Ok(Json(json!({ "models": models })))
}

/// Le porte seriali con il loro nome (PowerShell nascosto, sola lettura).
pub(crate) async fn cat_ports(_: Authed) -> Result<Json<Value>, CatApiError> {
    let (ports, _) = blocking(detect::read_system)
        .await?
        .map_err(|e| CatApiError { status: StatusCode::BAD_GATEWAY, code: "ports_failed", message: e.0 })?;
    let list: Vec<Value> = ports.iter().map(|p| json!({ "port": p.port, "name": p.name, "virtual": p.virtual_port, "virtual_kind": p.virtual_kind })).collect();
    Ok(Json(json!({ "ports": list })))
}

/// Il numero del modello di Hamlib che corrisponde al testo trovato dal rilevamento («FT-2000»): il nome esatto prima, poi il piu' corto che lo contiene.
pub(crate) fn resolve_model(token: &str, models: &[RigModel]) -> Option<u32> {
    let norm = |s: &str| s.chars().filter(|c| c.is_ascii_alphanumeric()).map(|c| c.to_ascii_lowercase()).collect::<String>();
    let wanted = norm(token);
    if wanted.is_empty() {
        return None;
    }
    // l'elenco ha anche i modelli di rete e quelli «dummy»: si preferiscono marche vere e stato stabile
    let candidates: Vec<&RigModel> = models.iter().filter(|m| norm(&m.model).contains(&wanted)).collect();
    candidates
        .iter()
        .find(|m| norm(&m.model) == wanted && m.status.eq_ignore_ascii_case("stable"))
        .or_else(|| candidates.iter().find(|m| norm(&m.model) == wanted))
        .or_else(|| candidates.iter().min_by_key(|m| (!m.status.eq_ignore_ascii_case("stable"), m.model.len())))
        .map(|m| m.id)
}

/// Rilevamento passivo della radio: identita' USB e nomi delle schede audio che Windows gia' conosce. Non apre nessuna porta e non manda
/// nessun comando alla radio.
pub(crate) async fn cat_detect(State(state): State<AppState>, _: Authed, Peer(peer): Peer, headers: HeaderMap) -> Result<Json<Value>, CatApiError> {
    let cat = service(&state)?;
    require_local(peer, &headers)?;
    let result = blocking(move || {
        let (ports, audio) = detect::read_system()?;
        Ok::<_, rig_control::RigError>((detect::detect(&ports, &audio), cat.models().ok()))
    })
    .await?
    .map_err(|e| CatApiError { status: StatusCode::BAD_GATEWAY, code: "detect_failed", message: e.0 })?;
    let (result, models) = result;
    let candidates: Vec<Value> = result
        .candidates
        .iter()
        .map(|c| {
            let mut v = json!(c);
            let model = models.as_deref().and_then(|m| resolve_model(&c.rig_token, m));
            if let Some(obj) = v.as_object_mut() {
                obj.insert("rig_model".into(), json!(model));
            }
            v
        })
        .collect();
    Ok(Json(json!({ "candidates": candidates, "audio_devices": result.audio_devices, "ports_seen": result.ports_seen })))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn model(id: u32, name: &str, status: &str) -> RigModel {
        RigModel { id, manufacturer: "X".into(), model: name.into(), status: status.into() }
    }

    #[test]
    fn a_detected_name_resolves_to_the_hamlib_model() {
        let models = [model(1029, "FT-2000", "Stable"), model(1040, "FT-2000D", "Stable"), model(1035, "FT-991", "Stable"), model(1036, "FT-991A", "Stable"), model(3073, "IC-7300", "Stable")];
        assert_eq!(resolve_model("FT-2000", &models), Some(1029));
        assert_eq!(resolve_model("FT-991", &models), Some(1035), "il nome esatto prima del piu' lungo");
        assert_eq!(resolve_model("ft991a", &models), Some(1036));
        assert_eq!(resolve_model("IC-7300", &models), Some(3073));
        assert_eq!(resolve_model("TS-9999", &models), None);
        assert_eq!(resolve_model("", &models), None);
    }
}
