//! Controllo automatico dei contest a cartella del repository (`contest/<nome>/`, `docs/contest-formato.md`): ogni cartella ha i file
//! obbligatori, un `contest.toml` valido, regole accettate dal motore, elenchi e macro leggibili e, se c'e' `prove.toml`, i QSO di esempio
//! danno il punteggio atteso con il motore vero (`valuta`) e il database dei paesi vero (`cty.dat` di Aurora).

use contest::cartelle::{Origine, carica, data_compatta};
use contest::elenco::{leggi_dominio, leggi_macro};
use contest::punti_testo::{InfoPaese, QsoTesto, aggiungi_qtc, cabrillo, qso_del_contest, valuta};
use dxcc::DxccDb;
use std::path::{Path, PathBuf};

fn radice() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../contest")
}

fn cartelle() -> Vec<PathBuf> {
    let mut v: Vec<PathBuf> = std::fs::read_dir(radice()).map(|rd| rd.flatten().map(|e| e.path()).filter(|p| p.is_dir()).collect()).unwrap_or_default();
    v.sort();
    v
}

fn database() -> DxccDb {
    let cty = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../desktop/src-tauri/cty/cty.dat");
    DxccDb::from_file(&cty).unwrap_or_else(|e| panic!("cty.dat: {e}"))
}

fn paese_di(db: &DxccDb, call: &str) -> Option<InfoPaese> {
    let l = db.lookup(call)?;
    let e = db.entity(l.entity);
    Some(InfoPaese { dxcc: e.prefix.clone(), continente: e.continent.clone(), cq: l.cq_zone, itu: l.itu_zone, wae: l.wae.map(|w| db.entity(w).prefix.clone()).unwrap_or_default(), wae_cont: l.wae.map(|w| db.entity(w).continent.clone()).unwrap_or_default() })
}

#[test]
fn ci_sono_cartelle_da_controllare() {
    assert!(!cartelle().is_empty(), "nessuna cartella in {}", radice().display());
}

#[test]
fn file_obbligatori_e_contenuti() {
    for d in cartelle() {
        let nome = d.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
        for f in ["contest.toml", "regolamento.md", "punteggio.regole", "prove.toml"] {
            assert!(d.join(f).is_file(), "{nome}: manca {f}");
        }
        // il lettore controlla il contest.toml, le regole con il motore (`RegoleScore::da_testo`), i rimandi e lo script
        let c = carica(&d, Origine::Inclusa).unwrap_or_else(|e| panic!("{nome}: {e}"));
        assert_eq!(c.id, nome);
        assert!(!c.domini.is_empty() || c.scambio.ricevuto.iter().all(|x| x.elenco.is_empty()), "{nome}");
        for (file, testo) in &c.domini {
            let dom = leggi_dominio(testo);
            assert!(!dom.entries.is_empty(), "{nome}: {file} senza voci");
        }
        for (file, testo) in &c.macros {
            assert!(!leggi_macro(testo).is_empty(), "{nome}: {file} senza macro");
            let su = testo.to_uppercase();
            assert!(!su.contains("<EXEC:") && !testo.contains(":\\"), "{nome}: {file}");
        }
        // convertito come un `.contest` il contest e' valido per il motore
        let t = c.in_contesto(0);
        assert_eq!(t.controlla(), Ok(()), "{nome}");
        assert!(!c.regolamento.trim().is_empty(), "{nome}: regolamento vuoto");
        // nessun dato personale: il nominativo delle prove e' di fantasia
        let prove = c.prove.clone().unwrap_or_default();
        assert!(!prove.is_empty(), "{nome}");
    }
}

/// Il giorno `aaaa-mm-gg` e l'ora `hhmm` UTC in millisecondi.
fn istante(giorno: &str, hhmm: &str) -> i64 {
    let n = |s: &str| s.parse::<i64>().unwrap_or(0);
    let (a, m, g) = (n(&giorno[0..4]), n(&giorno[5..7]), n(&giorno[8..10]));
    let h = n(&hhmm[0..2]);
    let mi = n(&hhmm[2..4]);
    (contest::calendario::days_from_civil(a, m as u32, g as u32) * 24 + h) * 3_600_000 + mi * 60_000
}

fn testo(t: &toml::Value, k: &str) -> String {
    match t.get(k) {
        Some(toml::Value::String(s)) => s.clone(),
        Some(toml::Value::Integer(i)) => i.to_string(),
        _ => String::new(),
    }
}

/// Le prove di ogni cartella: QSO per QSO (ammesso, doppio, punti, moltiplicatori nuovi) e totali.
#[test]
fn le_prove_danno_il_punteggio_atteso() {
    let db = database();
    let mut provate = 0;
    for d in cartelle() {
        let nome = d.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
        let c = carica(&d, Origine::Inclusa).unwrap_or_else(|e| panic!("{nome}: {e}"));
        let Some(testo_prove) = c.prove.clone() else { continue };
        let prove: toml::Value = toml::from_str(&testo_prove).unwrap_or_else(|e| panic!("{nome}: prove.toml: {e}"));
        let proprio = prove.get("prova").cloned().unwrap_or(toml::Value::Table(Default::default()));
        let giorno = testo(&proprio, "data");
        assert_eq!(giorno.len(), 10, "{nome}: prova.data mancante");
        // i punti di vista: la stazione propria di `[prova]` e quelle di `[[altra_stazione]]` (stesso giorno, altri QSO e altri totali),
        // perche' in certi contest il punteggio dipende dal paese di chi lo calcola (ARRL DX: W/VE o DX)
        let mut viste: Vec<(String, String, Vec<toml::Value>, toml::Value)> = vec![(nome.clone(), testo(&proprio, "nominativo_proprio"), prove.get("qso").and_then(toml::Value::as_array).cloned().unwrap_or_default(), prove.get("atteso").cloned().unwrap_or(toml::Value::Table(Default::default())))];
        for (i, a) in prove.get("altra_stazione").and_then(toml::Value::as_array).cloned().unwrap_or_default().iter().enumerate() {
            let chiamata = testo(a, "nominativo_proprio");
            assert!(!chiamata.is_empty(), "{nome}: altra_stazione {} senza nominativo_proprio", i + 1);
            viste.push((format!("{nome} (stazione {chiamata})"), chiamata, a.get("qso").and_then(toml::Value::as_array).cloned().unwrap_or_default(), a.get("atteso").cloned().unwrap_or(toml::Value::Table(Default::default()))));
        }
        for (nome, chiamata_propria, qso_vista, atteso_vista) in viste {
        let mio = paese_di(&db, &chiamata_propria);
        // il contest come lo vede il motore, all'ora della prova (a mezzogiorno del giorno scelto)
        let ora = istante(&giorno, "1200");
        let contesto = c.in_contesto(ora);
        let mut qsos = Vec::new();
        let mut attesi = Vec::new();
        for q in qso_vista {
            let banda = testo(&q, "banda");
            let hhmm = testo(&q, "ora_utc");
            qsos.push(QsoTesto {
                call: testo(&q, "nominativo"),
                band: format!("{banda}m"),
                mode: testo(&q, "modo"),
                freq_hz: q.get("freq_khz").and_then(toml::Value::as_integer).map_or(0, |k| k as u64 * 1000),
                time_ms: istante(&giorno, &hhmm),
                rst_sent: testo(&q, "rst_inviato"),
                rst_rcvd: testo(&q, "rst_ricevuto"),
                exch_sent: testo(&q, "scambio_inviato"),
                exch_rcvd: testo(&q, "scambio_ricevuto"),
            });
            attesi.push(q.get("atteso").cloned().unwrap_or(toml::Value::Table(Default::default())));
        }
        let validi = qso_del_contest(&contesto, &qsos, ora);
        assert_eq!(validi.len(), qsos.len(), "{nome}: la finestra o i modi del contest scartano QSO della prova ({} -> {})", data_compatta(ora), contesto.start);
        let dominio = c.dominio_del_contest();
        let (valutati, mut tot) = valuta(&contesto, &validi, mio.as_ref(), &|call| paese_di(&db, call), dominio.as_ref());
        // QTC (WAE): `qtc = N` nei totali attesi li aggiunge come fa il servizio con quelli salvati, (punti + QTC) x moltiplicatori
        if let Some(n) = atteso_vista.get("qtc").and_then(toml::Value::as_integer) {
            assert!(contesto.qtc > 0, "{nome}: QTC attesi in un contest senza QTC");
            aggiungi_qtc(&mut tot, u32::try_from(n).unwrap_or(0));
        }
        assert_eq!(valutati.len(), attesi.len(), "{nome}");
        for (i, (v, a)) in valutati.iter().zip(&attesi).enumerate() {
            let n = i + 1;
            let ammesso = a.get("ammesso").and_then(toml::Value::as_bool).unwrap_or(true);
            let doppio = a.get("doppio").and_then(toml::Value::as_bool).unwrap_or(false);
            let punti = a.get("punti").and_then(toml::Value::as_integer).unwrap_or(0);
            let mut molt: Vec<String> = a.get("moltiplicatori").and_then(toml::Value::as_array).map(|m| m.iter().filter_map(|x| x.as_str().map(str::to_string)).collect()).unwrap_or_default();
            molt.sort();
            let mut trovati: Vec<String> = v.nuovi.iter().map(|k| format!("{}@{}", k.split_once(':').map_or(k.as_str(), |x| x.1), v.qso.mode.to_uppercase())).collect();
            trovati.sort();
            let caso = format!("{nome}, QSO {n} ({} {}): {}", v.qso.call, v.qso.mode, testo(a, "motivo"));
            assert_eq!(v.escluso.is_none(), ammesso, "ammesso: {caso} (motore: {:?})", v.escluso);
            assert_eq!(v.doppio, doppio, "doppio: {caso}");
            assert_eq!(v.punti, punti, "punti: {caso}");
            assert_eq!(trovati, molt, "moltiplicatori: {caso}");
        }
        // Cabrillo: il nome ufficiale dichiarato in `[contest] cabrillo_contest`, il punteggio e una riga per QSO
        let f = cabrillo(&contesto, &chiamata_propria, &valutati, &tot, "0.0.0", &[]);
        if !c.meta.cabrillo_contest.is_empty() {
            assert!(f.contains(&format!("CONTEST: {}\r\n", c.meta.cabrillo_contest)), "{nome}: Cabrillo senza CONTEST: {}\n{f}", c.meta.cabrillo_contest);
        }
        assert!(f.contains(&format!("CLAIMED-SCORE: {}\r\n", tot.punteggio)), "{nome}: Cabrillo senza il punteggio");
        assert_eq!(f.matches("\r\nQSO: ").count(), valutati.len(), "{nome}: righe QSO del Cabrillo");
        // totali
        let t = atteso_vista;
        let intero = |k: &str| t.get(k).and_then(toml::Value::as_integer);
        let controlla = |k: &str, trovato: i64| {
            if let Some(atteso) = intero(k) {
                assert_eq!(trovato, atteso, "{nome}: totale {k}");
            }
        };
        controlla("qso_ammessi", i64::from(tot.qso));
        controlla("doppi", i64::from(tot.doppi));
        controlla("non_ammessi", i64::from(tot.esclusi));
        controlla("punti", tot.punti);
        controlla("moltiplicatori", tot.molt);
        controlla("punteggio", tot.punteggio);
        // moltiplicatori per modo: `moltiplicatori_cw`, `moltiplicatori_ssb`...
        if let Some(tab) = t.as_table() {
            for (k, v) in tab {
                if let (Some(modo), Some(atteso)) = (k.strip_prefix("moltiplicatori_"), v.as_integer()) {
                    let n = valutati.iter().filter(|x| x.qso.mode.eq_ignore_ascii_case(modo)).map(|x| x.nuovi.len() as i64).sum::<i64>();
                    assert_eq!(n, atteso, "{nome}: moltiplicatori {modo}");
                    // con moltiplicatori per modo (ARRL 10) lo stesso conteggio esce dal motore in `molt_per_modo` (per la scheda del punteggio)
                    if !tot.molt_per_modo.is_empty() {
                        assert_eq!(tot.molt_per_modo.get(&modo.to_ascii_uppercase()).copied().map_or(0, i64::from), atteso, "{nome}: molt_per_modo {modo}");
                    }
                }
            }
        }
        provate += 1;
        }
    }
    assert!(provate > 0, "nessuna prova eseguita");
}

/// Le cartelle ARRL DX hanno la prova da due punti di vista (stazione W e stazione DX): il punteggio dipende da chi lo calcola.
#[test]
fn arrl_dx_hanno_due_punti_di_vista() {
    let mut viste = 0;
    for d in cartelle() {
        let nome = d.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
        if !nome.starts_with("ARRL International DX-") {
            continue;
        }
        let c = carica(&d, Origine::Inclusa).unwrap_or_else(|e| panic!("{nome}: {e}"));
        let prove: toml::Value = toml::from_str(&c.prove.clone().unwrap_or_default()).unwrap_or_else(|e| panic!("{nome}: prove.toml: {e}"));
        let altre = prove.get("altra_stazione").and_then(toml::Value::as_array).map_or(0, Vec::len);
        assert!(altre >= 1, "{nome}: manca [[altra_stazione]]");
        // le due stazioni stanno in paesi diversi: una K/VE e una DX
        let db = database();
        let w = paese_di(&db, &testo(prove.get("prova").unwrap_or(&toml::Value::Boolean(false)), "nominativo_proprio")).map(|p| p.dxcc).unwrap_or_default();
        let dx = prove.get("altra_stazione").and_then(toml::Value::as_array).and_then(|a| a.first()).and_then(|a| paese_di(&db, &testo(a, "nominativo_proprio"))).map(|p| p.dxcc).unwrap_or_default();
        assert!(w == "K" || w == "VE", "{nome}: la prova principale deve essere di una stazione W/VE ({w})");
        assert!(dx != "K" && dx != "VE" && !dx.is_empty(), "{nome}: l'altra stazione deve essere DX ({dx})");
        viste += 1;
    }
    assert!(viste >= 2, "attese le cartelle ARRL DX-CW e DX-SSB");
}

/// ARRL 10: i moltiplicatori contano per modo; il motore li restituisce per modo (`molt_per_modo`) e nelle celle (`modi`) senza cambiare
/// il punteggio. Un contest senza moltiplicatori per modo non ha i campi nuovi nel JSON.
#[test]
fn arrl_10_moltiplicatori_per_modo() {
    let db = database();
    let c = carica(&radice().join("ARRL 10 Meter Contest"), Origine::Inclusa).unwrap_or_else(|e| panic!("{e}"));
    let prove: toml::Value = toml::from_str(&c.prove.clone().unwrap_or_default()).unwrap_or_else(|e| panic!("{e}"));
    let giorno = prove.get("prova").map(|p| testo(p, "data")).unwrap_or_default();
    let ora = istante(&giorno, "1200");
    let contesto = c.in_contesto(ora);
    let q = |call: &str, modo: &str, rcvd: &str, hhmm: &str| QsoTesto { call: call.into(), band: "10m".into(), mode: modo.into(), freq_hz: if modo == "CW" { 28_020_000 } else { 28_400_000 }, time_ms: istante(&giorno, hhmm), rst_sent: if modo == "CW" { "599".into() } else { "59".into() }, rst_rcvd: if modo == "CW" { "599".into() } else { "59".into() }, exch_sent: "001".into(), exch_rcvd: rcvd.into() };
    let qsos = vec![q("W1AW", "CW", "CT", "0010"), q("K1ABC", "SSB", "CT", "0020"), q("DL1ABC", "CW", "001", "0030"), q("W1AW", "SSB", "CT", "0040")];
    let mio = paese_di(&db, "IU3ABC");
    let validi = qso_del_contest(&contesto, &qsos, ora);
    assert_eq!(validi.len(), qsos.len());
    let dominio = c.dominio_del_contest();
    let (_, tot) = valuta(&contesto, &validi, mio.as_ref(), &|call| paese_di(&db, call), dominio.as_ref());
    // CT in CW e in SSB, DL in CW: 3 moltiplicatori (2 CW, 1 SSB); il QSO 4 e' un altro modo ma CT in SSB c'e' gia'
    assert_eq!(tot.molt, 3);
    assert_eq!(tot.molt_per_modo.get("CW"), Some(&2));
    assert_eq!(tot.molt_per_modo.get("SSB"), Some(&1));
    assert_eq!(tot.molt_per_modo.values().map(|n| i64::from(*n)).sum::<i64>(), tot.molt);
    let ct = tot.griglia.iter().find(|g| g.valore == "CT").unwrap_or_else(|| panic!("CT non in griglia: {:?}", tot.griglia));
    assert_eq!(ct.modi, vec!["CW".to_string(), "SSB".to_string()]);
    let dl = tot.griglia.iter().find(|g| g.valore == "DL").unwrap_or_else(|| panic!("DL non in griglia: {:?}", tot.griglia));
    assert_eq!(dl.modi, vec!["CW".to_string()]);
    let j = serde_json::to_value(&tot).unwrap_or_default();
    assert!(j.get("molt_per_modo").is_some());

    // CQ WW DX CW: nessun moltiplicatore per modo, nessun campo nuovo nel JSON
    let c = carica(&radice().join("CQ WW DX CW Contest"), Origine::Inclusa).unwrap_or_else(|e| panic!("{e}"));
    let giorno = toml::from_str::<toml::Value>(&c.prove.clone().unwrap_or_default()).ok().and_then(|p| p.get("prova").map(|p| testo(p, "data"))).unwrap_or_default();
    let ora = istante(&giorno, "1200");
    let contesto = c.in_contesto(ora);
    let qsos = vec![QsoTesto { call: "DL1ABC".into(), band: "20m".into(), mode: "CW".into(), freq_hz: 14_020_000, time_ms: istante(&giorno, "0010"), rst_sent: "599".into(), rst_rcvd: "599".into(), exch_sent: "15".into(), exch_rcvd: "14".into() }];
    let validi = qso_del_contest(&contesto, &qsos, ora);
    let (_, tot) = valuta(&contesto, &validi, mio.as_ref(), &|call| paese_di(&db, call), c.dominio_del_contest().as_ref());
    assert!(tot.molt_per_modo.is_empty());
    assert!(!tot.griglia.is_empty());
    let j = serde_json::to_value(&tot).unwrap_or_default();
    assert!(j.get("molt_per_modo").is_none());
    assert!(j["griglia"].as_array().is_some_and(|g| g.iter().all(|c| c.get("modi").is_none() && c.get("bande").is_some())));
}
