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
    // la dichiarazione per le impostazioni: bande, modi, doppi, categorie, edizione e Cabrillo
    let dic = &v["dichiarazione"];
    assert_eq!(dic["bande"], json!(["28 MHz"]));
    assert_eq!(dic["modi"]["ammessi"], json!(["cw", "ssb"]));
    assert_eq!(dic["modi"]["limite"][0]["frequenza_massima_khz"], 28300);
    assert_eq!(dic["doppi"]["per"], "modo");
    assert!(dic["categorie"]["potenza"].as_array().unwrap().len() == 3);
    assert_eq!(dic["edizione"]["durata_ore"], 48);
    assert_eq!(dic["edizione"]["date"]["inizio"].as_str().unwrap().len(), 14);
    assert!(dic["cabrillo"]["obbligatori"].as_array().unwrap().iter().any(|x| x == "CALLSIGN"));
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
async fn un_contest_incluso_non_si_cancella_e_si_modificano_solo_i_dati_personali() {
    let (s, d) = stato("rifiuti");
    let cookie = cookie_for(&s);
    let app = router(&s);
    let uri = format!("/api/v1/contest-testo/voce/{}", ARRL.replace(' ', "%20"));
    let corrente = body_json(call(&app, req(Method::GET, &uri).cookie(&cookie)).await).await;
    // la cancellazione dall'elenco resta rifiutata
    let r = call(&app, req(Method::DELETE, &uri).cookie(&cookie)).await;
    status_is(&r, StatusCode::CONFLICT);
    let e = body_json(r).await;
    assert_eq!(e["error"], "contest_readonly");
    assert!(e["message"].as_str().unwrap().contains("copialo nella tua cartella"), "{e}");
    // il salvataggio accetta solo date, ore, numero e macro: nome e regole del corpo si ignorano senza errore
    let mut corpo = corrente.clone();
    corpo["name"] = json!("Altro nome");
    corpo["score"] = json!("mult=pippo");
    corpo["start"] = json!("20301201000000");
    corpo["end"] = json!("20301202235959");
    corpo["work_time"] = json!(12);
    let r = call(&app, req(Method::PUT, &uri).cookie(&cookie).json(&corpo.to_string())).await;
    status_is(&r, StatusCode::OK);
    let v = body_json(r).await;
    assert_eq!((v["start"].as_str(), v["end"].as_str(), v["work_time"].as_i64()), (Some("20301201000000"), Some("20301202235959"), Some(12)));
    assert_eq!(v["name"], ARRL);
    assert_eq!(v["score"], corrente["score"]);
    assert_eq!(v["sovrascritta"]["work_time"], 12);
    assert!(d.join("contest").join("sovrascritture").join(format!("{ARRL}.json")).is_file());
    // la rilettura, l'elenco e il contest attivo vedono le nuove date
    let v = body_json(call(&app, req(Method::GET, &uri).cookie(&cookie)).await).await;
    assert_eq!(v["start"], "20301201000000");
    let lista = body_json(call(&app, req(Method::GET, "/api/v1/contest-testo").cookie(&cookie)).await).await;
    assert_eq!(lista["contest"].as_array().unwrap().iter().find(|c| c["id"] == ARRL).unwrap()["end"], "20301202235959");
    call(&app, req(Method::POST, "/api/v1/contest-testo/seleziona").cookie(&cookie).json(&json!({ "id": ARRL }).to_string())).await;
    let a = body_json(call(&app, req(Method::GET, "/api/v1/contest-testo/attivo").cookie(&cookie)).await).await;
    assert_eq!(a["contest"]["start"], "20301201000000");
    // valori non validi: 400 e il salvato resta
    for brutto in [json!({ "work_time": 99 }), json!({ "start": "x", "end": "y" }), json!({ "macros": "macros/nessuna.mc" })] {
        let r = call(&app, req(Method::PUT, &uri).cookie(&cookie).json(&brutto.to_string())).await;
        status_is(&r, StatusCode::BAD_REQUEST);
        assert_eq!(body_json(r).await["error"], "bad_contest");
    }
    assert_eq!(body_json(call(&app, req(Method::GET, &uri).cookie(&cookie)).await).await["work_time"], 12);
    // «Ripristina i valori del contest»
    let ripristina = format!("{uri}/sovrascrittura");
    let r = call(&app, req(Method::DELETE, &ripristina).cookie(&cookie)).await;
    status_is(&r, StatusCode::OK);
    let v = body_json(r).await;
    assert_eq!(v["start"], corrente["start"]);
    assert_eq!(v["work_time"], corrente["work_time"]);
    assert!(v["sovrascritta"].is_null());
    assert!(!d.join("contest").join("sovrascritture").join(format!("{ARRL}.json")).exists());
    // un vecchio file non ha modifiche personali da ripristinare, uno sconosciuto neppure
    call(&app, req(Method::POST, "/api/v1/contest-testo/voce").cookie(&cookie).json(&json!({ "name": "Vecchio" }).to_string())).await;
    status_is(&call(&app, req(Method::DELETE, "/api/v1/contest-testo/voce/Vecchio/sovrascrittura").cookie(&cookie)).await, StatusCode::BAD_REQUEST);
    status_is(&call(&app, req(Method::DELETE, "/api/v1/contest-testo/voce/Nessuno/sovrascrittura").cookie(&cookie)).await, StatusCode::NOT_FOUND);
    // un vecchio file si salva come prima
    let mut vecchio = body_json(call(&app, req(Method::GET, "/api/v1/contest-testo/voce/Vecchio").cookie(&cookie)).await).await;
    vecchio["notes"] = json!("RTTY");
    status_is(&call(&app, req(Method::PUT, "/api/v1/contest-testo/voce/Vecchio").cookie(&cookie).json(&vecchio.to_string())).await, StatusCode::OK);
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
