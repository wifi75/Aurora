//! Controllo automatico dei contest a cartella del repository (`contest/<nome>/`, `docs/contest-formato.md`): ogni cartella ha i file
//! obbligatori, un `contest.toml` valido, regole accettate dal motore, elenchi e macro leggibili e, se c'e' `prove.toml`, i QSO di esempio
//! danno il punteggio atteso con il motore vero (`valuta`) e il database dei paesi vero (`cty.dat` di Aurora).

use contest::cartelle::{Origine, carica, data_compatta};
use contest::elenco::{leggi_dominio, leggi_macro};
use contest::punti_testo::{InfoPaese, QsoTesto, qso_del_contest, valuta};
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
    Some(InfoPaese { dxcc: e.prefix.clone(), continente: e.continent.clone(), cq: l.cq_zone, itu: l.itu_zone })
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
        let mio = paese_di(&db, &testo(&proprio, "nominativo_proprio"));
        // il contest come lo vede il motore, all'ora della prova (a mezzogiorno del giorno scelto)
        let ora = istante(&giorno, "1200");
        let contesto = c.in_contesto(ora);
        let mut qsos = Vec::new();
        let mut attesi = Vec::new();
        for q in prove.get("qso").and_then(toml::Value::as_array).cloned().unwrap_or_default() {
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
        let (valutati, tot) = valuta(&contesto, &validi, mio.as_ref(), &|call| paese_di(&db, call), dominio.as_ref());
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
        // totali
        let t = prove.get("atteso").cloned().unwrap_or(toml::Value::Table(Default::default()));
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
                }
            }
        }
        provate += 1;
    }
    assert!(provate > 0, "nessuna prova eseguita");
}
