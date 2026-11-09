//! Le rotte della pagina Contest: stato e punteggio, regole, attivazione con le impostazioni del contest (e ritorno a quelle normali a fine
//! gara), valore delle stazioni decodificate, moltiplicatori presi, log del contest con le correzioni a mano, esportazione e calendario.

use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
};
use serde::Deserialize;
use serde_json::{Value, json};
use std::sync::Arc;

use crate::AppState;
use crate::routes::{ApiError, Authed, switch_mode};
use aurora_core::Command;
use contest::{ContestService, Impostazioni, Ripristino};

/// Gli interruttori che il contest cambia e che a fine gara tornano come erano.
const CONTROLLI: &[&str] = &["fox", "hound", "hold_tx", "autoseq", "rrr", "skip_tx1"];

fn service(state: &AppState) -> Result<Arc<ContestService>, ApiError> {
    state.contest.clone().ok_or_else(|| ApiError::new(StatusCode::NOT_FOUND, "contest_unavailable"))
}

fn send(state: &AppState, command: Command) -> Result<(), ApiError> {
    state.bus.commands().try_send(command).map_err(|_| ApiError::new(StatusCode::SERVICE_UNAVAILABLE, "engine_unavailable"))
}

/// Banda e modo di adesso (dallo stato della stazione).
fn banda_modo(state: &AppState) -> (String, String) {
    let s = state.current_status();
    let banda = s.as_ref().and_then(|s| rig_control::bands::amateur_band(s.dial_hz)).unwrap_or_default().to_string();
    let modo = s.map(|s| s.mode.to_uppercase()).unwrap_or_default();
    (banda, modo)
}

/// Contest attivo, punteggio, rate, finestra, impostazioni e l'elenco dei contest.
#[utoipa::path(get, path = "/api/v1/contest", responses((status = 200, description = "Stato della pagina Contest"), (status = 404, description = "Non disponibile (contest_unavailable)")))]
pub(crate) async fn contest_get(State(state): State<AppState>, _: Authed) -> Result<Json<Value>, ApiError> {
    let s = service(&state)?;
    let mut v = s.stato();
    let (banda, modo) = banda_modo(&state);
    if let Some(r) = s.attivo() {
        v["banda"] = json!(banda);
        v["modo"] = json!(modo);
        v["banda_ok"] = json!(banda.is_empty() || r.banda_ammessa(&banda));
        v["modo_ok"] = json!(modo.is_empty() || r.modo_ammesso(&modo));
    }
    Ok(Json(v))
}

/// Le regole di un contest e le sue impostazioni (salvate o proposte).
#[utoipa::path(get, path = "/api/v1/contest/regole/{id}", responses((status = 200, description = "Regole e impostazioni"), (status = 404, description = "Contest sconosciuto (contest_unknown)")))]
pub(crate) async fn contest_regole(State(state): State<AppState>, _: Authed, Path(id): Path<String>) -> Result<Json<Value>, ApiError> {
    let s = service(&state)?;
    let r = s.regola(&id).ok_or_else(|| ApiError::new(StatusCode::NOT_FOUND, "contest_unknown"))?;
    let imp = s.impostazioni(&id).unwrap_or_default();
    Ok(Json(json!({ "regole": r, "impostazioni": imp })))
}

#[derive(Deserialize)]
pub(crate) struct AttivaBody {
    id: String,
}

/// Attiva un contest e prepara le impostazioni giuste: sequenza di contest nel motore, Fox e Hound spenti, AutoSeq acceso, Hold Tx Freq e modo
/// come nelle impostazioni del contest. Cio' che c'era prima si ricorda per «torna alle impostazioni normali».
#[utoipa::path(post, path = "/api/v1/contest/attiva", responses((status = 200, description = "Contest attivo"), (status = 404, description = "Contest sconosciuto (contest_unknown)")))]
pub(crate) async fn contest_attiva(State(state): State<AppState>, _: Authed, Json(body): Json<AttivaBody>) -> Result<Json<Value>, ApiError> {
    let s = service(&state)?;
    let rig = state.rig.clone();
    let prima = rig.as_ref().and_then(|r| r.controls().ok()).map(|all| all.into_iter().filter(|(id, _)| CONTROLLI.contains(&id.as_str())).collect::<Vec<_>>()).unwrap_or_default();
    let modo_prima = state.current_status().map(|s| s.mode).unwrap_or_default();
    let setup = s.attiva(&body.id, Some(Ripristino { controlli: prima, modo: modo_prima.clone() })).map_err(|e| ApiError::new(StatusCode::NOT_FOUND, "contest_unknown").with_message(e))?;
    send(&state, Command::Contest(Some(setup)))?;
    let imp = s.impostazioni(&body.id).unwrap_or_default();
    let mut avvisi: Vec<String> = Vec::new();
    if let Some(rig) = &rig {
        for (id, on) in [("fox", false), ("hound", false), ("autoseq", true), ("hold_tx", imp.hold_tx), ("rrr", imp.rrr)] {
            if let Err(e) = rig.set_control(id, Some(on)) {
                avvisi.push(format!("{id}: {e}"));
            }
        }
    }
    let regole = s.regola(&body.id);
    if !imp.modo.is_empty() && !imp.modo.eq_ignore_ascii_case(&modo_prima) && regole.as_ref().is_some_and(|r| r.modo_ammesso(&imp.modo)) {
        // per il contest si passa al modo scelto (la radio resta nella banda, sulla frequenza del modo)
        if let Err(e) = switch_mode(&state, &imp.modo).await {
            avvisi.push(format!("modo: {e}"));
        }
    }
    Ok(Json(json!({ "ok": true, "avvisi": avvisi, "stato": s.stato() })))
}

/// Fine gara: il motore torna ai QSO normali e gli interruttori e il modo tornano come erano prima del contest.
#[utoipa::path(post, path = "/api/v1/contest/disattiva", responses((status = 200, description = "Contest spento, impostazioni normali ripristinate")))]
pub(crate) async fn contest_disattiva(State(state): State<AppState>, _: Authed) -> Result<Json<Value>, ApiError> {
    let s = service(&state)?;
    let ripristino = s.disattiva();
    send(&state, Command::Contest(None))?;
    let mut avvisi: Vec<String> = Vec::new();
    if let Some(r) = ripristino {
        if let Some(rig) = &state.rig {
            for (id, on) in &r.controlli {
                if let Err(e) = rig.set_control(id, Some(*on)) {
                    avvisi.push(format!("{id}: {e}"));
                }
            }
        }
        let ora = state.current_status().map(|s| s.mode).unwrap_or_default();
        if !r.modo.is_empty() && !r.modo.eq_ignore_ascii_case(&ora)
            && let Err(e) = switch_mode(&state, &r.modo).await
        {
            avvisi.push(format!("modo: {e}"));
        }
    }
    Ok(Json(json!({ "ok": true, "avvisi": avvisi, "stato": s.stato() })))
}

/// Salva le impostazioni di un contest (se e' quello attivo, la sequenza del motore cambia subito).
#[utoipa::path(put, path = "/api/v1/contest/impostazioni/{id}", responses((status = 200, description = "Impostazioni salvate"), (status = 400, description = "Valori non validi (bad_contest_settings)")))]
pub(crate) async fn contest_impostazioni(State(state): State<AppState>, _: Authed, Path(id): Path<String>, Json(body): Json<Impostazioni>) -> Result<Json<Value>, ApiError> {
    let s = service(&state)?;
    let mut imp = body;
    imp.scambio = imp.scambio.trim().to_uppercase().chars().filter(|c| c.is_ascii_alphanumeric() || *c == ' ' || *c == '#').take(12).collect();
    imp.parola_cq = imp.parola_cq.trim().to_uppercase().chars().filter(char::is_ascii_alphanumeric).take(4).collect();
    imp.grid6 = imp.grid6.trim().chars().take(6).collect();
    if let Some(setup) = s.salva_impostazioni(&id, imp).map_err(|e| ApiError::new(StatusCode::BAD_REQUEST, "bad_contest_settings").with_message(e))? {
        send(&state, Command::Contest(Some(setup)))?;
    }
    Ok(Json(json!({ "ok": true, "impostazioni": s.impostazioni(&id) })))
}

#[derive(Deserialize)]
pub(crate) struct Stazione {
    call: String,
    #[serde(default)]
    grid: String,
}

#[derive(Deserialize)]
pub(crate) struct ValutaBody {
    stazioni: Vec<Stazione>,
    #[serde(default)]
    banda: Option<String>,
    #[serde(default)]
    modo: Option<String>,
}

/// Quanto vale chiamare adesso ogni stazione: doppio, punti, moltiplicatori nuovi, bande su cui serve ancora.
#[utoipa::path(post, path = "/api/v1/contest/valuta", responses((status = 200, description = "Valore di ogni stazione")))]
pub(crate) async fn contest_valuta(State(state): State<AppState>, _: Authed, Json(body): Json<ValutaBody>) -> Result<Json<Value>, ApiError> {
    let s = service(&state)?;
    let (banda, modo) = banda_modo(&state);
    let banda = body.banda.filter(|b| !b.is_empty()).unwrap_or(banda);
    let modo = body.modo.filter(|m| !m.is_empty()).unwrap_or(modo);
    let lista: Vec<(String, String, String, String)> = body.stazioni.into_iter().take(200).map(|x| (x.call.trim().to_uppercase(), x.grid.trim().to_string(), banda.clone(), modo.clone())).collect();
    Ok(Json(json!({ "banda": banda, "modo": modo, "valori": s.valuta(&lista) })))
}

#[derive(Deserialize)]
pub(crate) struct BandaQuery {
    #[serde(default)]
    banda: Option<String>,
}

/// I moltiplicatori gia' presi su una banda (per la mappa dei mancanti).
#[utoipa::path(get, path = "/api/v1/contest/lavorati", responses((status = 200, description = "Moltiplicatori presi per tipo")))]
pub(crate) async fn contest_lavorati(State(state): State<AppState>, _: Authed, Query(q): Query<BandaQuery>) -> Result<Json<Value>, ApiError> {
    let s = service(&state)?;
    let banda = q.banda.filter(|b| !b.is_empty()).unwrap_or_else(|| banda_modo(&state).0);
    let tipi: Vec<Value> = s.lavorati(&banda).into_iter().map(|(tipo, valori)| json!({ "tipo": tipo, "valori": valori })).collect();
    Ok(Json(json!({ "banda": banda, "moltiplicatori": tipi })))
}

/// Consiglio di banda e modo: per ogni banda ammessa stazioni in ascolto, moltiplicatori nuovi, rate stimato, la banda e il modo migliori.
#[utoipa::path(get, path = "/api/v1/contest/consiglio", responses((status = 200, description = "Consiglio di banda e modo"), (status = 409, description = "Nessun contest attivo (contest_off)")))]
pub(crate) async fn contest_consiglio(State(state): State<AppState>, _: Authed) -> Result<Json<Value>, ApiError> {
    let s = service(&state)?;
    s.consiglio().map(Json).ok_or_else(|| ApiError::new(StatusCode::CONFLICT, "contest_off"))
}

/// La mappa dei moltiplicatori di una banda: presi, e stazioni in ascolto con i moltiplicatori nuovi che darebbero.
#[utoipa::path(get, path = "/api/v1/contest/mappa", responses((status = 200, description = "Moltiplicatori presi e stazioni in ascolto"), (status = 409, description = "Nessun contest attivo (contest_off)")))]
pub(crate) async fn contest_mappa(State(state): State<AppState>, _: Authed, Query(q): Query<BandaQuery>) -> Result<Json<Value>, ApiError> {
    let s = service(&state)?;
    let banda = q.banda.filter(|b| !b.is_empty()).unwrap_or_else(|| banda_modo(&state).0);
    s.mappa(&banda).map(Json).ok_or_else(|| ApiError::new(StatusCode::CONFLICT, "contest_off"))
}

/// Il log del contest valutato (punti, doppi, moltiplicatori aperti, stato).
#[utoipa::path(get, path = "/api/v1/contest/log", responses((status = 200, description = "QSO del log del contest e totali")))]
pub(crate) async fn contest_log(State(state): State<AppState>, _: Authed) -> Result<Json<Value>, ApiError> {
    let s = service(&state)?;
    match s.log() {
        Some((qso, totali)) => Ok(Json(json!({ "qso": qso, "totali": totali }))),
        None => Ok(Json(json!({ "qso": [], "totali": null }))),
    }
}

#[derive(Deserialize)]
pub(crate) struct ModificaBody {
    #[serde(default)]
    exch_rcvd: Option<String>,
    #[serde(default)]
    grid: Option<String>,
    #[serde(default)]
    stato: Option<contest::punteggio::Stato>,
}

/// Corregge a mano un QSO del log del contest (scambio copiato male, stato valido/incerto/errore).
#[utoipa::path(patch, path = "/api/v1/contest/log/{n}", responses((status = 200, description = "QSO corretto"), (status = 404, description = "QSO non trovato (contest_qso)")))]
pub(crate) async fn contest_log_modifica(State(state): State<AppState>, _: Authed, Path(n): Path<u32>, Json(body): Json<ModificaBody>) -> Result<Json<Value>, ApiError> {
    let s = service(&state)?;
    let exch = body.exch_rcvd.map(|x| x.chars().filter(|c| c.is_ascii_alphanumeric() || *c == ' ').take(20).collect::<String>());
    let grid = body.grid.map(|g| g.chars().filter(char::is_ascii_alphanumeric).take(6).collect::<String>());
    s.modifica(n, exch, grid, body.stato).map_err(|e| ApiError::new(StatusCode::NOT_FOUND, "contest_qso").with_message(e))?;
    Ok(Json(json!({ "ok": true })))
}

/// Ricomincia il log del contest (il vecchio resta come copia).
#[utoipa::path(post, path = "/api/v1/contest/log/azzera", responses((status = 200, description = "Log del contest vuoto")))]
pub(crate) async fn contest_log_azzera(State(state): State<AppState>, _: Authed) -> Result<Json<Value>, ApiError> {
    service(&state)?.azzera_log();
    Ok(Json(json!({ "ok": true })))
}

#[derive(Deserialize)]
pub(crate) struct EsportaQuery {
    #[serde(default)]
    formato: Option<String>,
    #[serde(default)]
    banda: Option<String>,
}

/// Il file da mandare allo sponsor (Cabrillo, EDI o ADIF), il nome proposto e il controllo prima dell'invio. Niente invio automatico.
#[utoipa::path(get, path = "/api/v1/contest/esporta", responses((status = 200, description = "`{testo, nome, problemi, invio_url}`"), (status = 409, description = "Nessun contest attivo (contest_off)")))]
pub(crate) async fn contest_esporta(State(state): State<AppState>, _: Authed, Query(q): Query<EsportaQuery>) -> Result<Json<Value>, ApiError> {
    let s = service(&state)?;
    let formato = q.formato.filter(|f| matches!(f.as_str(), "cabrillo" | "edi" | "adif"));
    let (testo, nome, problemi) = s.esporta(formato.as_deref(), q.banda.as_deref()).ok_or_else(|| ApiError::new(StatusCode::CONFLICT, "contest_off"))?;
    let invio = s.attivo().map(|r| r.export.invio_url).unwrap_or_default();
    Ok(Json(json!({ "testo": testo, "nome": nome, "problemi": problemi, "invio_url": invio })))
}

/// I contest dei prossimi 12 mesi.
#[utoipa::path(get, path = "/api/v1/contest/calendario", responses((status = 200, description = "Contest in corso e in arrivo")))]
pub(crate) async fn contest_calendario(State(state): State<AppState>, _: Authed) -> Result<Json<Value>, ApiError> {
    Ok(Json(json!({ "contest": service(&state)?.calendario(366) })))
}
