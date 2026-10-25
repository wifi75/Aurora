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
    hamlib::RigModel,
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
