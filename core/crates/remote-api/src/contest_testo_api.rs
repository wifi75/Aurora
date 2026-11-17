//! Le rotte dei contest dei modi testuali: elenco dei file `.contest`, lettura, salvataggio, cancellazione e scelta del contest attivo
//! (uno solo; «Nessun contest» = nessuno). Il contest attivo arriva alla pagina con le sue macro e il suo elenco dei domini.

use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
};
use serde::Deserialize;
use serde_json::{Value, json};
use std::sync::Arc;

use crate::AppState;
use crate::routes::{ApiError, Authed};
use contest::ContestService;
use contest::elenco::ContestTesto;

fn service(state: &AppState) -> Result<Arc<ContestService>, ApiError> {
    state.contest.clone().ok_or_else(|| ApiError::new(StatusCode::NOT_FOUND, "contest_unavailable"))
}

/// L'elenco dei contest (nome e note), quello attivo e i file di macro e di domini che si possono scegliere.
#[utoipa::path(get, path = "/api/v1/contest-testo", responses((status = 200, description = "Elenco dei contest dei modi testuali")))]
pub(crate) async fn elenco(State(state): State<AppState>, _: Authed) -> Result<Json<Value>, ApiError> {
    let s = service(&state)?;
    let e = s.elenco();
    let voci: Vec<Value> = e.elenco().into_iter().map(|c| json!({ "id": c.id, "name": c.name, "notes": c.notes, "start": c.start, "end": c.end })).collect();
    Ok(Json(json!({ "attivo": e.attivo_id(), "contest": voci, "macros": e.file_disponibili("macros", ".mc"), "domini": e.file_disponibili("dom", ".dom") })))
}

/// Il contest attivo con le sue macro e i domini (`null` se nessuno).
#[utoipa::path(get, path = "/api/v1/contest-testo/attivo", responses((status = 200, description = "Contest attivo o null")))]
pub(crate) async fn attivo(State(state): State<AppState>, _: Authed) -> Result<Json<Value>, ApiError> {
    let s = service(&state)?;
    Ok(Json(json!(s.elenco().attivo())))
}

/// Un contest.
#[utoipa::path(get, path = "/api/v1/contest-testo/voce/{id}", responses((status = 200, description = "Il contest"), (status = 404, description = "Sconosciuto (contest_unknown)")))]
pub(crate) async fn leggi(State(state): State<AppState>, _: Authed, Path(id): Path<String>) -> Result<Json<ContestTesto>, ApiError> {
    let s = service(&state)?;
    s.elenco().leggi(&id).map(Json).ok_or_else(|| ApiError::new(StatusCode::NOT_FOUND, "contest_unknown"))
}

fn salva_in(state: &AppState, c: ContestTesto) -> Result<Json<Value>, ApiError> {
    let s = service(state)?;
    let id = s.elenco().salva(c).map_err(|e| ApiError::new(StatusCode::BAD_REQUEST, "bad_contest").with_message(e))?;
    Ok(Json(json!({ "ok": true, "id": id })))
}

/// Salva un contest esistente.
#[utoipa::path(put, path = "/api/v1/contest-testo/voce/{id}", responses((status = 200, description = "Salvato"), (status = 400, description = "Valori non validi (bad_contest)")))]
pub(crate) async fn salva(State(state): State<AppState>, _: Authed, Path(id): Path<String>, Json(mut c): Json<ContestTesto>) -> Result<Json<Value>, ApiError> {
    c.id = id;
    salva_in(&state, c)
}

/// Aggiunge un contest nuovo (il nome del file viene dal nome del contest).
#[utoipa::path(post, path = "/api/v1/contest-testo/voce", responses((status = 200, description = "Aggiunto, con il suo id"), (status = 400, description = "Valori non validi (bad_contest)")))]
pub(crate) async fn nuovo(State(state): State<AppState>, _: Authed, Json(mut c): Json<ContestTesto>) -> Result<Json<Value>, ApiError> {
    c.id = String::new();
    salva_in(&state, c)
}

/// Cancella un contest.
#[utoipa::path(delete, path = "/api/v1/contest-testo/voce/{id}", responses((status = 200, description = "Cancellato"), (status = 404, description = "Sconosciuto (contest_unknown)")))]
pub(crate) async fn elimina(State(state): State<AppState>, _: Authed, Path(id): Path<String>) -> Result<Json<Value>, ApiError> {
    let s = service(&state)?;
    s.elenco().elimina(&id).map_err(|e| ApiError::new(StatusCode::NOT_FOUND, "contest_unknown").with_message(e))?;
    Ok(Json(json!({ "ok": true })))
}

#[derive(Deserialize)]
pub(crate) struct ImportaQuery {
    #[serde(default)]
    nome: String,
}

/// Importa un contest: il corpo e' il pacchetto `.zip` (`.contest` con `macros/` e `dom/`) o un `.contest` da solo, `nome` il nome
/// del file scelto. Il contest si controlla (sezioni `[Score]` e `[Script]`) e si aggiunge senza sovrascriverne un altro.
#[utoipa::path(post, path = "/api/v1/contest-testo/importa", responses((status = 200, description = "Importato, con il suo id"), (status = 400, description = "File non valido (bad_contest)")))]
pub(crate) async fn importa(State(state): State<AppState>, _: Authed, Query(q): Query<ImportaQuery>, body: axum::body::Bytes) -> Result<Json<Value>, ApiError> {
    let s = service(&state)?;
    let id = s.elenco().importa(&q.nome, &body).map_err(|e| ApiError::new(StatusCode::BAD_REQUEST, "bad_contest").with_message(e))?;
    Ok(Json(json!({ "ok": true, "id": id })))
}

/// Il pacchetto `.zip` di un contest con le sue macro e il suo elenco dei domini.
#[utoipa::path(get, path = "/api/v1/contest-testo/voce/{id}/pacchetto", responses((status = 200, description = "Pacchetto zip"), (status = 404, description = "Sconosciuto (contest_unknown)")))]
pub(crate) async fn esporta(State(state): State<AppState>, _: Authed, Path(id): Path<String>) -> Result<axum::response::Response, ApiError> {
    use axum::response::IntoResponse;
    let s = service(&state)?;
    let zip = s.elenco().esporta(&id).map_err(|e| ApiError::new(StatusCode::NOT_FOUND, "contest_unknown").with_message(e))?;
    let nome: String = contest::pacchetto::nome_pacchetto(&id).chars().filter(|c| c.is_ascii() && *c != '"' && !c.is_ascii_control()).collect();
    Ok(([(axum::http::header::CONTENT_TYPE, "application/zip".to_string()), (axum::http::header::CONTENT_DISPOSITION, format!("attachment; filename=\"{nome}\""))], zip).into_response())
}

#[derive(Deserialize)]
pub(crate) struct SelezionaBody {
    id: Option<String>,
}

/// Sceglie il contest attivo (`id` nullo = «Nessun contest»).
#[utoipa::path(post, path = "/api/v1/contest-testo/seleziona", responses((status = 200, description = "Scelto"), (status = 404, description = "Sconosciuto (contest_unknown)")))]
pub(crate) async fn seleziona(State(state): State<AppState>, _: Authed, Json(body): Json<SelezionaBody>) -> Result<Json<Value>, ApiError> {
    let s = service(&state)?;
    s.elenco().seleziona(body.id.as_deref()).map_err(|e| ApiError::new(StatusCode::NOT_FOUND, "contest_unknown").with_message(e))?;
    Ok(Json(json!({ "ok": true, "attivo": s.elenco().attivo_id() })))
}

/// Paese, continente e zone di un nominativo dal database DXCC (`None` se manca).
fn info_paese(state: &AppState, call: &str) -> Option<contest::punti_testo::InfoPaese> {
    let v = state.dxcc.as_ref()?(call)?;
    let s = |k: &str| v.get(k).and_then(Value::as_str).unwrap_or("").to_string();
    let n = |k: &str| v.get(k).and_then(Value::as_u64).and_then(|x| u8::try_from(x).ok()).unwrap_or(0);
    Some(contest::punti_testo::InfoPaese { dxcc: s("prefix"), continente: s("continent"), cq: n("cq_zone"), itu: n("itu_zone") })
}

/// Il contest attivo valutato sui QSO del log: contest, QSO valutati, totali (con i QTC), nominativo della stazione e QTC dell'edizione.
type Valutazione = (ContestTesto, Vec<contest::punti_testo::Valutato>, contest::punti_testo::Punteggio, String, Vec<contest::qtc::Qtc>);

async fn valuta_attivo(state: &AppState, prova: Option<contest::punti_testo::QsoTesto>) -> Result<Valutazione, ApiError> {
    use contest::punti_testo::{QsoTesto, qso_del_contest, scambi_da_commento, valuta};
    let s = service(state)?;
    let attivo = s.elenco().attivo().ok_or_else(|| ApiError::new(StatusCode::CONFLICT, "contest_off"))?;
    let book = state.logbook.clone();
    let entries = tokio::task::spawn_blocking(move || book.entries())
        .await
        .map_err(|_| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "log_task"))?
        .map_err(|e| {
            tracing::warn!("log non letto: {e}");
            ApiError::new(StatusCode::BAD_GATEWAY, "log_read")
        })?;
    let qsos: Vec<QsoTesto> = entries
        .iter()
        .filter_map(|e| {
            let ms = contest::elenco::data_ms(&format!("{}{:0<6}", e.date, e.time))?;
            let (exch_sent, exch_rcvd) = scambi_da_commento(&e.comment);
            Some(QsoTesto { call: e.call.clone(), band: e.band.clone(), mode: e.mode.clone(), freq_hz: e.freq_hz, time_ms: ms, rst_sent: e.rst_sent.clone(), rst_rcvd: e.rst_rcvd.clone(), exch_sent, exch_rcvd })
        })
        .collect();
    let ora = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_millis() as i64);
    let mut del_contest = qso_del_contest(&attivo.contest, &qsos, ora);
    if let Some(p) = prova {
        del_contest.push(QsoTesto { time_ms: ora, ..p });
    }
    let (call, _) = s.stazione();
    let mio = info_paese(state, &call);
    let paese = |c: &str| info_paese(state, c);
    let (valutati, mut tot) = valuta(&attivo.contest, &del_contest, mio.as_ref(), &paese, attivo.domain.as_ref());
    let qtc = if attivo.contest.qtc > 0 { contest::qtc::nella_finestra(&s.qtc(&attivo.contest.id), contest::punti_testo::finestra(&attivo.contest, ora)) } else { Vec::new() };
    if attivo.contest.qtc > 0 {
        contest::punti_testo::aggiungi_qtc(&mut tot, qtc.len() as u32);
    }
    contest::punti_testo::ritmo(&mut tot, &valutati, ora);
    Ok((attivo.contest, valutati, tot, call, qtc))
}

#[derive(Deserialize)]
pub(crate) struct AmmessoQuery {
    call: String,
    #[serde(default)]
    band: String,
    #[serde(default)]
    mode: String,
    #[serde(default)]
    exch: String,
}

/// Il valore di un nominativo nel contest attivo, mentre lo si scrive: ammesso o no (motivo `band`/`valid`), doppio, punti e
/// moltiplicatori nuovi che porterebbe.
#[utoipa::path(get, path = "/api/v1/contest-testo/ammesso", responses((status = 200, description = "Valore del nominativo"), (status = 409, description = "Nessun contest attivo (contest_off)")))]
pub(crate) async fn ammesso(State(state): State<AppState>, _: Authed, Query(q): Query<AmmessoQuery>) -> Result<Json<Value>, ApiError> {
    let prova = contest::punti_testo::QsoTesto { call: q.call.trim().to_ascii_uppercase(), band: q.band, mode: q.mode, exch_rcvd: q.exch, ..Default::default() };
    let (contest, valutati, _, mia, _) = valuta_attivo(&state, Some(prova)).await?;
    let v = valutati.last().ok_or_else(|| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "contest_eval"))?;
    // l'ultimo QSO del contest con lo stesso nominativo (per le macro `<PREVNRS>`, `<QSOBEFORETIME>`...): ora, banda, modo e scambi
    let precedente = valutati[..valutati.len() - 1]
        .iter()
        .rev()
        .find(|p| p.qso.call.eq_ignore_ascii_case(&v.qso.call))
        .map(|p| json!({ "time_ms": p.qso.time_ms, "band": p.qso.band, "mode": p.qso.mode, "exch_rcvd": p.qso.exch_rcvd, "exch_sent": p.qso.exch_sent }));
    // lo scambio mandato nell'ultimo QSO del log (`<PREVNRS>`: si ripete a chi chiede di nuovo il numero dopo che il QSO e' a log)
    let ultimo_qso = valutati[..valutati.len() - 1].last();
    let ultimo_mandato = ultimo_qso.map(|p| p.qso.exch_sent.clone());
    // nominativo e rapporto mandato dell'ultimo QSO del log (`<PREVCALL>`, `<PREVRSTS>`)
    let ultimo = ultimo_qso.map(|p| json!({ "call": p.qso.call, "rst_sent": p.qso.rst_sent }));
    // lo scambio ricevuto da questo nominativo ha un terzo elemento (sezione, club, stato)? Con le regole del punteggio, qui sta l'unica verita'
    let info = info_paese(&state, &v.qso.call);
    let exch_atteso = contest::punti_testo::scambio_atteso(&contest, &v.qso, &v.qso.call, info.as_ref(), info_paese(&state, &mia).as_ref());
    Ok(Json(json!({ "ammesso": v.escluso.is_none(), "exch_atteso": exch_atteso, "motivo": v.escluso, "doppio": v.doppio, "punti": v.punti, "nuovi": v.nuovi, "precedente": precedente, "ultimo_mandato": ultimo_mandato, "ultimo": ultimo })))
}

/// Il punteggio del contest attivo dei modi testuali: QSO, doppi, punti, moltiplicatori, punteggio e righe per banda.
#[utoipa::path(get, path = "/api/v1/contest-testo/punteggio", responses((status = 200, description = "Punteggio del contest attivo"), (status = 409, description = "Nessun contest attivo (contest_off)")))]
pub(crate) async fn punteggio(State(state): State<AppState>, _: Authed) -> Result<Json<Value>, ApiError> {
    let (_, _, tot, _, _) = valuta_attivo(&state, None).await?;
    Ok(Json(json!(tot)))
}

/// Il file Cabrillo 3.0 del contest attivo dei modi testuali, con il nome proposto.
#[utoipa::path(get, path = "/api/v1/contest-testo/cabrillo", responses((status = 200, description = "`{testo, nome, qso}`"), (status = 409, description = "Nessun contest attivo (contest_off)")))]
pub(crate) async fn cabrillo(State(state): State<AppState>, _: Authed) -> Result<Json<Value>, ApiError> {
    let (c, valutati, tot, call, qtc) = valuta_attivo(&state, None).await?;
    let s = service(&state)?;
    let testo = contest::punti_testo::cabrillo(&c, &call, &valutati, &tot, s.versione(), &qtc);
    let nome = format!("{}-{}.log", call.trim().to_ascii_uppercase().replace('/', "_"), c.id.replace(' ', "_"));
    Ok(Json(json!({ "testo": testo, "nome": nome, "qso": valutati.len() })))
}

#[derive(Deserialize)]
pub(crate) struct QtcQuery {
    #[serde(default)]
    call: String,
}

fn divieto(d: Result<(), contest::qtc::Divieto>) -> Value {
    match d {
        Ok(()) => Value::Null,
        Err(e) => json!(e),
    }
}

/// I QTC del contest attivo e, per il nominativo chiesto, cosa si puo' fare: QTC gia' scambiati, quanti ne restano, prossima serie,
/// se si possono mandare o ricevere (o perche' no) e i QSO del log proposti da mandare.
#[utoipa::path(get, path = "/api/v1/contest-testo/qtc", responses((status = 200, description = "`{regola, qtc, stazione}`"), (status = 409, description = "Nessun contest attivo (contest_off)")))]
pub(crate) async fn qtc(State(state): State<AppState>, _: Authed, Query(q): Query<QtcQuery>) -> Result<Json<Value>, ApiError> {
    use contest::qtc::{Divieto, RegolaQtc, ammesso, prossima_serie, proposte, residuo, scambiati};
    let (c, valutati, _, call, tutti) = valuta_attivo(&state, None).await?;
    let Some(regola) = RegolaQtc::da_codice(c.qtc) else {
        return Ok(Json(json!({ "regola": Value::Null, "qtc": [], "stazione": Value::Null })));
    };
    let dest = q.call.trim().to_ascii_uppercase();
    let stazione = if dest.is_empty() {
        Value::Null
    } else {
        let validi: Vec<contest::punti_testo::QsoTesto> = valutati.iter().filter(|v| !v.doppio).map(|v| v.qso.clone()).collect();
        let mio = info_paese(&state, &call).map(|i| i.continente).unwrap_or_default();
        let suo = info_paese(&state, &dest).map(|i| i.continente).unwrap_or_default();
        let resta = residuo(&tutti, &dest);
        let esauriti = |r: Result<(), Divieto>| if r.is_ok() && resta == 0 { Err(Divieto::Esauriti) } else { r };
        json!({
            "call": dest,
            "continente": suo,
            "mio_continente": mio,
            "scambiati": scambiati(&tutti, &dest),
            "residuo": resta,
            "serie": prossima_serie(&tutti),
            "invio": divieto(esauriti(ammesso(regola, true, &mio, &suo))),
            "ricezione": divieto(esauriti(ammesso(regola, false, &mio, &suo))),
            "proposte": proposte(&validi, &tutti, &dest),
        })
    };
    Ok(Json(json!({ "regola": regola, "qtc": tutti, "stazione": stazione })))
}

#[derive(Deserialize)]
pub(crate) struct QtcBody {
    qtc: Vec<contest::qtc::Qtc>,
}

/// Salva una serie di QTC mandati o ricevuti nel contest attivo (al massimo 10 con ogni stazione).
#[utoipa::path(post, path = "/api/v1/contest-testo/qtc", responses((status = 200, description = "Salvati, con il totale"), (status = 400, description = "Serie non valida (bad_qtc)"), (status = 409, description = "Nessun contest attivo o senza QTC (contest_off)")))]
pub(crate) async fn qtc_salva(State(state): State<AppState>, _: Authed, Json(body): Json<QtcBody>) -> Result<Json<Value>, ApiError> {
    let s = service(&state)?;
    let attivo = s.elenco().attivo().ok_or_else(|| ApiError::new(StatusCode::CONFLICT, "contest_off"))?;
    if attivo.contest.qtc <= 0 {
        return Err(ApiError::new(StatusCode::CONFLICT, "contest_off"));
    }
    let ora = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_millis() as i64);
    let finestra = contest::punti_testo::finestra(&attivo.contest, ora);
    let tutti = s.aggiungi_qtc(&attivo.contest.id, &body.qtc, finestra).map_err(|e| ApiError::new(StatusCode::BAD_REQUEST, "bad_qtc").with_message(e))?;
    Ok(Json(json!({ "ok": true, "totale": contest::qtc::nella_finestra(&tutti, finestra).len() })))
}
