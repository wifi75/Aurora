mod common;

use std::path::PathBuf;
use std::sync::Arc;

use axum::http::{Method, StatusCode};
use common::*;
use serde_json::json;

const ARRL: &str = "ARRL 10 Meter Contest";

fn dati(nome: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("aurora-api-contest-{nome}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    d
}

/// Lo stato con il servizio dei contest: i dati in una cartella provvisoria, i contest a cartella quelli del repository (sola lettura).
fn stato(nome: &str) -> (remote_api::AppState, PathBuf) {
    let d = dati(nome);
    let servizio = contest::ContestService::open(&d, Arc::new(|| ("IU3ABC".to_string(), "IO91wm".to_string())), "prova");
    servizio.elenco().imposta_cartelle_incluse(Some(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../contest")));
    (state().with_contest(servizio), d)
}

#[tokio::test]
async fn l_elenco_porta_origine_schede_e_scambio() {
    let (s, d) = stato("elenco");
    let cookie = cookie_for(&s);
    let app = router(&s);
    // un vecchio file accanto alla cartella
    std::fs::create_dir_all(&d).unwrap();
    let r = call(&app, req(Method::POST, "/api/v1/contest-testo/voce").cookie(&cookie).json(&json!({ "name": "Vecchio RTTY" }).to_string())).await;
    status_is(&r, StatusCode::OK);
    let r = call(&app, req(Method::GET, "/api/v1/contest-testo").cookie(&cookie)).await;
    status_is(&r, StatusCode::OK);
    let v = body_json(r).await;
    let voci = v["contest"].as_array().unwrap();
    let arrl = voci.iter().find(|c| c["id"] == ARRL).expect("ARRL 10 nell'elenco");
    assert_eq!(arrl["origine"], "cartella");
    assert!(arrl["schede"].as_array().unwrap().len() >= 3);
    assert!(arrl["scambio"]["ricevuto"].as_array().unwrap().len() >= 3);
    assert!(arrl["start"].as_str().unwrap().len() == 14, "date dell'edizione");
    assert!(arrl.get("regolamento").is_none(), "il regolamento solo da /voce/{{id}}");
    let vecchio = voci.iter().find(|c| c["id"] == "Vecchio RTTY").expect("il vecchio file");
    assert_eq!(vecchio["origine"], "file");
    assert_eq!(vecchio["schede"], json!([]));
    // il vecchio ARRL 10 `.contest` non e' copiato qui: l'elenco ha una sola voce con quel nome
    assert_eq!(voci.iter().filter(|c| c["name"].as_str().is_some_and(|n| n.eq_ignore_ascii_case(ARRL))).count(), 1);
    let _ = std::fs::remove_dir_all(&d);
}

#[tokio::test]
async fn la_voce_porta_il_regolamento_solo_se_e_una_cartella() {
    let (s, d) = stato("voce");
    let cookie = cookie_for(&s);
    let app = router(&s);
    let r = call(&app, req(Method::GET, &format!("/api/v1/contest-testo/voce/{}", ARRL.replace(' ', "%20"))).cookie(&cookie)).await;
    status_is(&r, StatusCode::OK);
    let v = body_json(r).await;
    assert_eq!(v["origine"], "cartella");
    assert!(v["regolamento"].as_str().unwrap().contains("regolamento"));
    assert_eq!(v["name"], ARRL);
    assert!(v["score"].as_str().unwrap().contains("bands=10"));
    assert_eq!(v["valid_cw"], true);
    // un file
    call(&app, req(Method::POST, "/api/v1/contest-testo/voce").cookie(&cookie).json(&json!({ "name": "Vecchio" }).to_string())).await;
    let v = body_json(call(&app, req(Method::GET, "/api/v1/contest-testo/voce/Vecchio").cookie(&cookie)).await).await;
    assert_eq!((v["origine"].as_str(), v["regolamento"].is_null()), (Some("file"), true));
    assert_eq!(v["schede"], json!([]));
    status_is(&call(&app, req(Method::GET, "/api/v1/contest-testo/voce/Nessuno").cookie(&cookie)).await, StatusCode::NOT_FOUND);
    let _ = std::fs::remove_dir_all(&d);
}

#[tokio::test]
async fn il_contest_attivo_viene_dalla_cartella_con_macro_e_domini() {
    let (s, d) = stato("attivo");
    let cookie = cookie_for(&s);
    let app = router(&s);
    let r = call(&app, req(Method::POST, "/api/v1/contest-testo/seleziona").cookie(&cookie).json(&json!({ "id": ARRL }).to_string())).await;
    status_is(&r, StatusCode::OK);
    let v = body_json(call(&app, req(Method::GET, "/api/v1/contest-testo/attivo").cookie(&cookie)).await).await;
    assert_eq!(v["contest"]["name"], ARRL);
    assert_eq!(v["origine"], "cartella");
    assert!(!v["schede"].as_array().unwrap().is_empty());
    assert!(!v["macros"].as_array().unwrap().is_empty());
    assert!(!v["domain"]["entries"].as_array().unwrap().is_empty());
    assert!(v["start_ms"].is_i64() && v["end_ms"].is_i64());
    let _ = std::fs::remove_dir_all(&d);
}

#[tokio::test]
async fn un_contest_incluso_non_si_modifica_ne_si_cancella() {
    let (s, d) = stato("rifiuti");
    let cookie = cookie_for(&s);
    let app = router(&s);
    let uri = format!("/api/v1/contest-testo/voce/{}", ARRL.replace(' ', "%20"));
    let corrente = body_json(call(&app, req(Method::GET, &uri).cookie(&cookie)).await).await;
    let r = call(&app, req(Method::PUT, &uri).cookie(&cookie).json(&corrente.to_string())).await;
    status_is(&r, StatusCode::CONFLICT);
    let e = body_json(r).await;
    assert_eq!(e["error"], "contest_readonly");
    assert!(e["message"].as_str().unwrap().contains("copialo nella tua cartella"), "{e}");
    let r = call(&app, req(Method::DELETE, &uri).cookie(&cookie)).await;
    status_is(&r, StatusCode::CONFLICT);
    assert_eq!(body_json(r).await["error"], "contest_readonly");
    let _ = std::fs::remove_dir_all(&d);
}

#[tokio::test]
async fn copia_nella_mia_cartella() {
    let (s, d) = stato("copia");
    let cookie = cookie_for(&s);
    let app = router(&s);
    let uri = format!("/api/v1/contest-testo/voce/{}/copia", ARRL.replace(' ', "%20"));
    let r = call(&app, req(Method::POST, &uri).cookie(&cookie)).await;
    status_is(&r, StatusCode::OK);
    assert_eq!(body_json(r).await["id"], ARRL);
    assert!(d.join("contest").join(ARRL).join("contest.toml").is_file() || d.join(ARRL).join("contest.toml").is_file());
    // una seconda copia: rifiutata; la cartella dell'utente resta sola nell'elenco
    let r = call(&app, req(Method::POST, &uri).cookie(&cookie)).await;
    status_is(&r, StatusCode::CONFLICT);
    assert_eq!(body_json(r).await["error"], "contest_copy");
    let v = body_json(call(&app, req(Method::GET, "/api/v1/contest-testo").cookie(&cookie)).await).await;
    assert_eq!(v["contest"].as_array().unwrap().iter().filter(|c| c["id"] == ARRL).count(), 1);
    // un file e uno sconosciuto
    call(&app, req(Method::POST, "/api/v1/contest-testo/voce").cookie(&cookie).json(&json!({ "name": "Vecchio" }).to_string())).await;
    status_is(&call(&app, req(Method::POST, "/api/v1/contest-testo/voce/Vecchio/copia").cookie(&cookie)).await, StatusCode::BAD_REQUEST);
    status_is(&call(&app, req(Method::POST, "/api/v1/contest-testo/voce/Nessuno/copia").cookie(&cookie)).await, StatusCode::NOT_FOUND);
    let _ = std::fs::remove_dir_all(&d);
}

#[tokio::test]
async fn le_rotte_chiedono_un_dispositivo_associato() {
    let (s, d) = stato("auth");
    let app = router(&s);
    status_is(&call(&app, req(Method::POST, "/api/v1/contest-testo/voce/x/copia")).await, StatusCode::UNAUTHORIZED);
    let _ = std::fs::remove_dir_all(&d);
}
