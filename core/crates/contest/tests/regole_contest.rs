//! Il punteggio dei contest del primo e del secondo gruppo con i file `.contest` distribuiti (`desktop/src-tauri/contest`) e le regole di
//! `docs/contest-regole.md`. Ogni prova ha un piccolo log con nominativi finti e il punteggio calcolato a mano dal regolamento.
//! La stazione e' IU3ABC (Italia, EU, zona CQ 15).

use contest::elenco::{ContestTesto, Dominio, data_ms, leggi_dominio};
use contest::punti_testo::{InfoPaese, Punteggio, QsoTesto, aggiungi_qtc, valuta};
use std::path::PathBuf;

fn cartella() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../desktop/src-tauri/contest")
}

fn contest(nome: &str) -> ContestTesto {
    let testo = std::fs::read_to_string(cartella().join(format!("{nome}.contest"))).unwrap_or_default();
    let c = ContestTesto::da_testo(nome, &testo);
    assert!(!c.score.is_empty(), "{nome}: manca la sezione [Score]");
    assert_eq!(c.controlla(), Ok(()), "{nome}");
    c
}

fn dominio(file: &str) -> Dominio {
    leggi_dominio(&std::fs::read_to_string(cartella().join(file)).unwrap_or_default())
}

/// Un piccolo database DXCC finto: (prefisso del nominativo, paese, continente, zona CQ, zona ITU), il primo che combacia.
fn paese(call: &str) -> Option<InfoPaese> {
    const T: [(&str, &str, &str, u8, u8); 20] = [
        ("UA9", "UA9", "AS", 17, 30),
        ("EA8", "EA8", "AF", 33, 36),
        ("EA", "EA", "EU", 14, 37),
        ("XE", "XE", "NA", 6, 10),
        ("SP", "SP", "EU", 15, 28),
        ("OK", "OK", "EU", 15, 28),
        ("CT", "CT", "EU", 14, 37),
        ("ON", "ON", "EU", 14, 27),
        ("YU", "YU", "EU", 15, 28),
        ("SM", "SM", "EU", 14, 18),
        ("UR", "UR", "EU", 16, 29),
        ("UA", "UA", "EU", 16, 29),
        ("VE", "VE", "NA", 4, 9),
        ("VK", "VK", "OC", 30, 59),
        ("JA", "JA", "AS", 25, 45),
        ("DL", "DL", "EU", 14, 28),
        ("I", "I", "EU", 15, 28),
        ("W", "K", "NA", 5, 8),
        ("K", "K", "NA", 5, 8),
        ("N", "K", "NA", 5, 8),
    ];
    T.iter().find(|t| call.starts_with(t.0)).map(|t| InfoPaese { dxcc: t.1.into(), continente: t.2.into(), cq: t.3, itu: t.4, ..InfoPaese::default() })
}

fn mio() -> InfoPaese {
    paese("IU3ABC").unwrap_or_default()
}

fn q(call: &str, band: &str, mode: &str, min: i64, sent: &str, rcvd: &str) -> QsoTesto {
    QsoTesto {
        call: call.into(),
        band: band.into(),
        mode: mode.into(),
        freq_hz: 0,
        time_ms: data_ms("20261114120000").unwrap_or(0) + min * 60_000,
        rst_sent: "599".into(),
        rst_rcvd: "599".into(),
        exch_sent: sent.into(),
        exch_rcvd: rcvd.into(),
    }
}

fn r(call: &str, band: &str, min: i64, rcvd: &str) -> QsoTesto {
    q(call, band, "RTTY", min, &format!("{:03}", min + 1), rcvd)
}

fn punteggio(nome: &str, log: &[QsoTesto], dom: Option<&Dominio>) -> Punteggio {
    valuta(&contest(nome), log, Some(&mio()), &paese, dom).1
}

#[test]
fn cq_ww_rtty() {
    let log = [
        r("IK2XYZ", "20m", 0, "15"),       // stesso paese: 1 punto; zona 15, I
        r("DL1ABC", "20m", 1, "14"),       // stesso continente: 2; zona 14, DL
        r("W1AW", "20m", 2, "05 CT"),      // altro continente: 3; zona 5, K, stato CT
        r("DL1ABC", "20m", 3, "14"),       // doppio
        r("DL1ABC", "40m", 4, "14"),       // 2; zona 14 e DL su 40 m
        r("JA1ABC", "160m", 5, "25"),      // 160 m non e' del contest: non ammesso
    ];
    let t = punteggio("CQ WW RTTY DX", &log, None);
    // punti 1+2+3+2 = 8; moltiplicatori 20 m: 15, I, 14, DL, 5, K, CT = 7; 40 m: 14, DL = 2; totale 9; 8 x 9 = 72
    assert_eq!((t.qso, t.doppi, t.esclusi, t.punti, t.molt, t.punteggio), (4, 1, 1, 8, 9, 72));
}

#[test]
fn cq_wpx_rtty() {
    let log = [
        r("IK2XYZ", "20m", 0, "001"), // stesso paese 1; IK2
        r("DL1ABC", "40m", 1, "001"), // stesso continente 2, x2 su 40 m = 4; DL1
        r("W1AW", "80m", 2, "001"),   // altro continente 3, x2 su 80 m = 6; W1
        r("DL1ABC", "20m", 3, "002"), // 2; DL1 gia' preso (una volta sola)
        r("IK2ABC", "20m", 4, "001"), // 1; IK2 gia' preso
    ];
    let t = punteggio("CQ WPX RTTY Contest", &log, None);
    // punti 1+4+6+2+1 = 14; prefissi IK2, DL1, W1 = 3; 14 x 3 = 42
    assert_eq!((t.punti, t.molt, t.punteggio), (14, 3, 42));
}

#[test]
fn wae_rtty_con_qtc() {
    let log = [
        r("DL1ABC", "80m", 0, "001"), // 1 punto; DL su 80 m, peso 4
        r("W1AW", "80m", 1, "001"),   // 1; area K1 su 80 m, peso 4 (W conta per aree)
        r("W2XYZ", "80m", 2, "001"),  // 1; area K2 su 80 m, peso 4
        r("DL1ABC", "20m", 3, "002"), // 1; DL su 20 m, peso 2
        r("JA1ABC", "40m", 4, "001"), // 1; area JA1 su 40 m, peso 3
    ];
    let mut t = punteggio("WAEDC RTTY Contest", &log, None);
    aggiungi_qtc(&mut t, 3);
    // QSO 5 + QTC 3 = 8; moltiplicatori 4+4+4+2+3 = 17; 8 x 17 = 136
    assert_eq!((t.punti, t.molt, t.punteggio), (5, 17, 136));
    assert_eq!(contest("WAEDC RTTY Contest").qtc, 2);
}

#[test]
fn bartg_hf_rtty() {
    let log = [
        r("IK2XYZ", "20m", 0, "001"), // I su 20 m; continente EU
        r("W1AW", "20m", 1, "001"),   // K e area K1 su 20 m; NA
        r("W1ABC", "20m", 2, "001"),  // niente di nuovo
        r("W2ABC", "40m", 3, "001"),  // K e K2 su 40 m
        r("JA1ABC", "15m", 4, "001"), // JA e JA1 su 15 m; AS
    ];
    let t = punteggio("BARTG HF RTTY", &log, None);
    // 5 QSO x 1 punto; paesi e aree 7; continenti EU, NA, AS = 3 (una volta sola); 5 x 7 x 3 = 105
    assert_eq!((t.punti, t.molt, t.punteggio), (5, 21, 105));
}

#[test]
fn arrl_rtty_roundup() {
    let log = [
        r("W1AW", "20m", 0, "CT"),      // 1 punto; stato CT
        r("W1AW", "40m", 1, "CT"),      // altra banda: 1 punto, CT gia' preso (una volta sola)
        r("VE3ABC", "20m", 2, "ON"),    // 1; provincia ON
        r("DL1ABC", "20m", 3, "001"),   // 1; DXCC DL
        r("IK2XYZ", "40m", 4, "002"),   // 1; DXCC I
        r("W1AW", "20m", 5, "CT"),      // doppio
    ];
    let t = punteggio("ARRL RTTY ROUNDUP", &log, None);
    // 5 punti; CT, ON, DL, I = 4; 5 x 4 = 20 (W e VE non contano come DXCC)
    assert_eq!((t.doppi, t.punti, t.molt, t.punteggio), (1, 5, 4, 20));
}

#[test]
fn raem() {
    let log = [
        q("UA3ABC", "20m", "CW", 0, "001 45N12E", "015 55N37E"),  // 50 + 10 lat + 25 lon = 85
        q("RAEM", "20m", "CW", 1, "002 45N12E", "100 67N33E"),    // 50 + 22 + 21 + polare 100 + RAEM 300 = 493
        q("UA9ABC", "40m", "CW", 2, "003 45N12E", "020 55N83E"),  // 50 + 10 + 71 = 131
    ];
    let t = punteggio("RAEM contest", &log, None);
    // nessun moltiplicatore, stazione non polare: 85 + 493 + 131 = 709
    assert_eq!((t.punti, t.punteggio), (709, 709));
    // da dentro il circolo polare il totale vale x1,1
    let polare: Vec<QsoTesto> = log.iter().map(|x| QsoTesto { exch_sent: x.exch_sent.replace("45N", "67N"), ..x.clone() }).collect();
    let t = punteggio("RAEM contest", &polare, None);
    // 50+12+25 = 87; 50+0+21+100+300 = 471; 50+12+71 = 133; totale 691 x 1,1 = 760,1 -> 760
    assert_eq!((t.punti, t.punteggio), (691, 760));
    assert_eq!(t.errore_script, None);
}

#[test]
fn script_sbagliato_torna_alla_sezione_score() {
    let log = [
        q("UA3ABC", "20m", "CW", 0, "001 45N12E", "015 55N37E"),
        q("RAEM", "20m", "CW", 1, "002 45N12E", "100 67N33E"),
    ];
    let mut c = contest("RAEM contest");
    // la sezione [Script] si rilegge uguale dal testo salvato
    assert!(c.script.contains("fn punti(qso)"));
    assert_eq!(ContestTesto::da_testo("RAEM contest", &c.in_testo()).script, c.script);
    // errore durante il calcolo: vale [Score], 50 punti per QSO e 300 per RAEM
    c.script = "fn punti(qso) { qso.nessuno.campo }".into();
    let t = valuta(&c, &log, Some(&mio()), &paese, None).1;
    assert_eq!((t.punti, t.punteggio), (400, 400));
    assert!(t.errore_script.as_deref().is_some_and(|e| e.contains("punti()")), "{:?}", t.errore_script);
    // ciclo infinito: fermato dal limite delle operazioni, stesso risultato
    c.script = "fn punti(qso) { loop { } }".into();
    let t = valuta(&c, &log, Some(&mio()), &paese, None).1;
    assert_eq!(t.punteggio, 400);
    assert!(t.errore_script.is_some());
    // uno script che non compila non si salva
    c.script = "fn punti(qso) { 1 + }".into();
    assert!(c.controlla().is_err());
}

#[test]
fn script_con_moltiplicatori_e_ammessi() {
    let log = [
        r("DL1ABC", "20m", 0, "001"),
        r("DL2ABC", "20m", 1, "001"),
        r("W1AW", "40m", 2, "001"),
        r("JA1ABC", "20m", 3, "001"),
    ];
    let mut c = contest("CQ WW RTTY DX");
    c.script = r#"
        fn ammesso(qso) { if qso.cont == "AS" { [false, "asia"] } else { true } }
        fn punti(qso) { 2 }
        fn moltiplicatori(qso) { ["cont:" + qso.cont] }
    "#
    .into();
    let (v, t) = valuta(&c, &log, Some(&mio()), &paese, None);
    // 3 ammessi x 2 punti; moltiplicatori EU e NA = 2; 6 x 2 = 12
    assert_eq!((t.esclusi, t.punti, t.molt, t.punteggio), (1, 6, 2, 12));
    assert_eq!(v[3].escluso.as_deref(), Some("asia"));
    assert_eq!(v[0].nuovi, vec!["cont:EU".to_string()]);
}

#[test]
fn sartg_ww_rtty() {
    let log = [
        r("IK2XYZ", "20m", 0, "001"), // stesso paese 5; I su 20 m
        r("DL1ABC", "20m", 1, "001"), // stesso continente 10; DL
        r("W1AW", "20m", 2, "001"),   // altro continente 15; K e K1
        r("W1ABC", "20m", 3, "001"),  // 15; niente di nuovo
        r("JA1ABC", "40m", 4, "001"), // 15; JA e JA1 su 40 m
    ];
    let t = punteggio("SARTG WW RTTY Contest", &log, None);
    // punti 5+10+15+15+15 = 60; moltiplicatori 6; 60 x 6 = 360
    assert_eq!((t.punti, t.molt, t.punteggio), (60, 6, 360));
}

#[test]
fn russian_ww_digital() {
    let dom = dominio("dom/russian.dom");
    let log = [
        q("UA3ABC", "20m", "RTTY", 0, "001", "MA"),   // stesso continente 3; UA e MA (RTTY, 20 m)
        q("UA3ABC", "20m", "BPSK63", 1, "002", "MA"), // altro modo: 3; UA e MA di nuovo (per modo)
        q("UA3ABC", "40m", "RTTY", 2, "003", "MA"),   // 3 x2 su 40 m = 6; UA e MA su 40 m
        q("W1AW", "20m", "RTTY", 3, "004", "001"),    // altro continente 5; K
        q("IK2XYZ", "80m", "RTTY", 4, "005", "002"),  // stesso paese 1 x2 = 2; I
        q("DL1ABC", "20m", "RTTY", 5, "006", "XX"),   // 3; DL (lo scambio di un non russo non e' un oblast)
    ];
    let t = punteggio("Russian WW Digital Contest", &log, Some(&dom));
    // punti 3+3+6+5+2+3 = 22; moltiplicatori 2+2+2+1+1+1 = 9; 22 x 9 = 198
    assert_eq!((t.punti, t.molt, t.punteggio), (22, 9, 198));
}

#[test]
fn ukrainian_dx_digi() {
    let log = [
        q("UR5ABC", "20m", "RTTY", 0, "001", "KO"),  // con l'Ucraina 5; UR e KO
        q("UR5ABC", "80m", "RTTY", 1, "002", "KO"),  // 5 x2 su 80 m = 10; UR e KO su 80 m
        q("UR5ABC", "80m", "PSK31", 2, "003", "KO"), // altro modo: 10; UR e KO in PSK31
        q("DL1ABC", "20m", "RTTY", 3, "004", "001"), // stesso continente 1; DL
        q("W1AW", "40m", "PSK31", 4, "005", "001"),  // altro continente 3; K
        q("JA1ABC", "160m", "RTTY", 5, "006", "001"), // 160 m: non ammesso
    ];
    let t = punteggio("Ukrainian DX DIGI Contest", &log, None);
    // punti 5+10+10+1+3 = 29; moltiplicatori 2+2+2+1+1 = 8; 29 x 8 = 232
    assert_eq!((t.esclusi, t.punti, t.molt, t.punteggio), (1, 29, 8, 232));
}

// --- secondo gruppo ---

fn p(call: &str, band: &str, min: i64, rcvd: &str) -> QsoTesto {
    q(call, band, "BPSK63", min, &format!("{:03}", min + 1), rcvd)
}

#[test]
fn sp_dx_rtty() {
    let log = [
        r("SP5ABC", "20m", 0, "R"),   // stesso continente 5; SP e provincia R su 20 m; EU
        r("DL1ABC", "20m", 1, "001"), // 5; DL
        r("IK2XYZ", "40m", 2, "001"), // stesso paese 2; I su 40 m
        r("W1AW", "20m", 3, "001"),   // altro continente 10; K; NA
    ];
    let t = punteggio("SPDX RTTY Contest", &log, None);
    // punti 22; paesi e province 2+1+1+1 = 5; continenti EU, NA = 2; 22 x 5 x 2 = 220
    assert_eq!((t.punti, t.molt, t.punteggio), (22, 10, 220));
}

#[test]
fn ok_dx_rtty() {
    let log = [
        r("OK1ABC", "20m", 0, "15"), // stesso continente su 20 m 1; OK e la stazione OK1ABC
        r("OK1XYZ", "20m", 1, "15"), // 1; stazione OK1XYZ
        r("OK1ABC", "80m", 2, "15"), // stesso continente su 80 m 3; OK e OK1ABC su 80 m
        r("W1AW", "40m", 3, "05"),   // altro continente su 40 m 6; K
        r("DL1ABC", "15m", 4, "14"), // 1; DL
    ];
    let t = punteggio("OK DX RTTY Contest", &log, None);
    // punti 1+1+3+6+1 = 12; moltiplicatori 2+1+2+1+1 = 7; 12 x 7 = 84
    assert_eq!((t.punti, t.molt, t.punteggio), (12, 7, 84));
}

#[test]
fn ea_rtty() {
    let log = [
        r("EA4ABC", "20m", 0, "M"),   // DX con EA 3; EA e provincia M
        r("EA8ABC", "20m", 1, "TF"),  // EA8 e' Spagna: 3; EA8 e TF
        r("W1AW", "20m", 2, "001"),   // 1; K e K1
        r("DL1ABC", "40m", 3, "001"), // 1; DL
        r("IK2XYZ", "40m", 4, "001"), // 1; I
    ];
    let t = punteggio("EA RTTY Contest", &log, None);
    // punti 3+3+1+1+1 = 9; moltiplicatori 2+2+2+1+1 = 8; 9 x 8 = 72
    assert_eq!((t.punti, t.molt, t.punteggio), (9, 8, 72));
}

#[test]
fn xe_rtty() {
    let dom = dominio("dom/Mexico32.dom");
    let log = [
        r("XE1ABC", "20m", 0, "AGS"), // stazione XE 4; stato AGS e paese XE
        r("XE2ABC", "20m", 1, "JAL"), // 4; JAL
        r("IK2XYZ", "20m", 2, "001"), // stesso paese 2; I
        r("W1AW", "40m", 3, "001"),   // altro paese 3; K
    ];
    let t = punteggio("Mexican RTTY Contest", &log, Some(&dom));
    // punti 13; moltiplicatori AGS, XE, JAL, I, K = 5; 13 x 5 = 65
    assert_eq!((t.punti, t.molt, t.punteggio), (13, 5, 65));
}

#[test]
fn alessandro_volta() {
    let log = [
        r("IK2XYZ", "20m", 0, "001 15"), // stesso paese: non ammesso
        r("DL1ABC", "20m", 1, "001 14"), // tabella 15 -> 14: 3; DL
        r("W1AW", "20m", 2, "001 05"),   // 15 -> 5: 20; area K1 (W conta per aree)
        r("W1AW", "10m", 3, "002 05"),   // fuori continente su 10 m: 20 x2 = 40; K1 su 10 m
        r("JA1ABC", "80m", 4, "001 25"), // 15 -> 25: 27 x2 = 54; JA1 su 80 m
        r("DL1ABC", "20m", 5, "002 14"), // doppio
    ];
    let t = punteggio("Alessandro Volta RTTY Contest", &log, None);
    // punti 3+20+40+54 = 117; moltiplicatori 4; QSO 4; 117 x 4 x 4 = 1872
    assert_eq!((t.qso, t.doppi, t.esclusi, t.punti, t.molt, t.punteggio), (4, 1, 1, 117, 4, 1872));
    // un paese di un altro continente su 4 bande: un moltiplicatore in piu'
    let log = [
        r("JA1ABC", "80m", 0, "001 25"), // 27 x2 = 54; JA1 su 80 m
        r("JA1ABC", "40m", 1, "002 25"), // 27; JA1 su 40 m
        r("JA1ABC", "20m", 2, "003 25"), // 27; JA1 su 20 m
        r("JA1ABC", "15m", 3, "004 25"), // 27; JA1 su 15 m; JA su 4 bande
    ];
    let t = punteggio("Alessandro Volta RTTY Contest", &log, None);
    // punti 135; moltiplicatori 4 + 1; QSO 4; 135 x 5 x 4 = 2700
    assert_eq!((t.punti, t.molt, t.punteggio), (135, 4, 2700));
}

#[test]
fn radio_ww_rtty() {
    let dom = dominio("dom/russian.dom");
    let log = [
        r("UA3ABC", "20m", 0, "MA"), // stesso continente 5; UA e oblast MA
        r("UA9ABC", "20m", 1, "SV"), // altro continente 10; UA9 e SV
        r("W1AW", "20m", 2, "05"),   // 10; K
        r("DL1ABC", "40m", 3, "14"), // 5; DL
    ];
    let t = punteggio("RUSSIAN RADIO WW RTTY CONTEST", &log, Some(&dom));
    // punti 30; moltiplicatori 6; 30 x 6 = 180
    assert_eq!((t.punti, t.molt, t.punteggio), (30, 6, 180));
}

#[test]
fn open_ukraine_rtty() {
    let log = [
        r("UR5ABC", "20m", 0, "PO001"), // 2 + 10 per la regione PO nuova su 20 m
        r("UR5XYZ", "20m", 1, "PO002"), // 2
        r("UR4ABC", "80m", 2, "KV005"), // 2 + 10
        r("DL1ABC", "40m", 3, "ZZ001"), // 2 + 10
        r("UR4ABC", "80m", 600, "KV006"), // 22:00, secondo turno della parte bassa: non e' un doppio, 2 + 10 (KV di nuovo)
        r("UR4ABC", "80m", 601, "KV006"), // doppio nel secondo turno
    ];
    let t = punteggio("OPEN UKRAINE RTTY CHAMPIONSHIP", &log, None);
    // punti 2 x 5 = 10; regioni nuove PO, KV, ZZ, KV del secondo turno = 4 x 10; 10 + 40 = 50
    assert_eq!((t.doppi, t.punti, t.molt, t.punteggio), (1, 10, 4, 50));
}

#[test]
fn jarts_rtty() {
    let log = [
        r("DL1ABC", "20m", 0, "599 45"), // stesso continente 2; DL
        r("W1AW", "20m", 1, "599 50"),   // altro continente 3; area K1 (non il paese K)
        r("W2ABC", "20m", 2, "599 60"),  // 3; K2
        r("JA1ABC", "20m", 3, "599 40"), // 3; JA1
        r("IK2XYZ", "40m", 4, "599 30"), // 2; I su 40 m
    ];
    let t = punteggio("JARTS RTTY Contest", &log, None);
    // punti 13; moltiplicatori 5; 13 x 5 = 65
    assert_eq!((t.punti, t.molt, t.punteggio), (13, 5, 65));
}

#[test]
fn bartg_sprint_e_sprint75() {
    let log = [
        r("IK2XYZ", "20m", 0, "001"), // I; EU
        r("W1AW", "20m", 1, "001"),   // K e K1; NA
        r("W1AW", "40m", 2, "002"),   // un punto, niente di nuovo (una volta sola)
        r("JA1ABC", "15m", 3, "001"), // JA e JA1; AS
    ];
    for nome in ["BARTG-Sprint RTTY", "BARTG Sprint 75 RTTY"] {
        let t = punteggio(nome, &log, None);
        // punti 4; moltiplicatori 5; continenti 3; 4 x 5 x 3 = 60
        assert_eq!((t.doppi, t.punti, t.molt, t.punteggio), (0, 4, 15, 60), "{nome}");
    }
}

#[test]
fn sartg_new_year() {
    let log = [
        r("SM5ABC", "80m", 0, "001 OLE GODT NYTT AR"), // 1; area SM5 su 80 m (la Scandinavia conta per aree)
        r("SM5XYZ", "80m", 1, "001 PER GODT NYTT AR"), // 1; niente di nuovo
        r("SM3ABC", "80m", 2, "001 UNO GOTT NYTT AR"), // 1; SM3
        r("DL1ABC", "80m", 3, "001 HANS FROHES NEUES"), // 1; DL su 80 m
        r("DL1ABC", "40m", 4, "002 HANS FROHES NEUES"), // 1; DL su 40 m
        r("W1AW", "20m", 5, "001 BOB HAPPY NEW YEAR"), // 20 m non e' del contest
    ];
    let t = punteggio("SARTG New Year RTTY Contest", &log, None);
    // punti 5; moltiplicatori 4; 5 x 4 = 20
    assert_eq!((t.esclusi, t.punti, t.molt, t.punteggio), (1, 5, 4, 20));
}

#[test]
fn makrothen() {
    let log = [
        q("W1AW", "20m", "RTTY", 0, "JN65", "FN31"),   // 6610 km
        q("IK2XYZ", "80m", "RTTY", 1, "JN65", "JN65"), // stesso quadrato: 100 senza fattore
        q("DL1ABC", "40m", "RTTY", 2, "JN65", "JO62"), // 779 km x 1,5 = 1168
    ];
    let t = punteggio("Makrothen Contest", &log, None);
    // nessun moltiplicatore: 6610 + 100 + 1168 = 7878
    assert_eq!((t.punti, t.punteggio), (7878, 7878));
}

#[test]
fn yu_dx() {
    let log = [
        q("YU1ABC", "20m", "CW", 0, "001", "BG"),  // stazione YU 10; YU e distretto BG
        q("YU1XYZ", "20m", "CW", 1, "002", "BG"),  // 10; niente di nuovo
        q("IK2XYZ", "20m", "CW", 2, "003", "28"),  // stesso paese 1; I
        q("DL1ABC", "20m", "CW", 3, "004", "28"),  // stesso continente 2; DL
        q("W1AW", "40m", "CW", 4, "005", "08"),    // altro continente 4; K
    ];
    let t = punteggio("YU-DX-HF Contest", &log, None);
    // punti 27; moltiplicatori 5; 27 x 5 = 135
    assert_eq!((t.punti, t.molt, t.punteggio), (27, 5, 135));
}

#[test]
fn russian_ww_psk() {
    let dom = dominio("dom/russian.dom");
    let log = [
        p("UA3ABC", "20m", 0, "MA"),  // altro paese 3; UA e MA su 20 m
        p("UA3ABC", "40m", 1, "MA"),  // 3 x2 su 40 m = 6; UA e MA su 40 m
        p("W1AW", "20m", 2, "001"),   // altro continente 5; K
        p("IK2XYZ", "80m", 3, "002"), // stesso paese 1 x2 = 2; I su 80 m
    ];
    let t = punteggio("Russian WW PSK Contest", &log, Some(&dom));
    // punti 3+6+5+2 = 16; moltiplicatori 2+2+1+1 = 6; 16 x 6 = 96
    assert_eq!((t.punti, t.molt, t.punteggio), (16, 6, 96));
}

#[test]
fn ea_psk63() {
    let log = [
        p("EA4URE", "20m", 0, "HQ"), // con EA 3; EA e HQ (EA4URE vale come moltiplicatore)
        p("W1AW", "20m", 1, "001"),  // 1; K e K1
    ];
    let t = punteggio("EA PSK63 Contest", &log, None);
    // punti 4; moltiplicatori 4; 4 x 4 = 16
    assert_eq!((t.punti, t.molt, t.punteggio), (4, 4, 16));
}

#[test]
fn uba_psk63_prefix() {
    let log = [
        p("ON4ABC", "20m", 0, "ANT"), // 1; prefisso ON4 e sezione ANT
        p("ON5ABC", "20m", 1, "ANT"), // 1; ON5
        p("DL1ABC", "20m", 2, "001"), // 1; DL1
        p("UA3ABC", "20m", 3, "001"), // Russia: zero punti e nessun moltiplicatore (non ammesso)
        p("DL1ABC", "40m", 4, "002"), // 1; DL1 su 40 m
        p("ON6ABC", "20m", 5, "XXX"), // non socio: 1; ON6, la sezione XXX non e' un moltiplicatore
    ];
    let t = punteggio("UBA PSK63 Prefix Contest", &log, None);
    // punti 5; moltiplicatori 6; 5 x 6 = 30
    assert_eq!((t.esclusi, t.punti, t.molt, t.punteggio), (1, 5, 6, 30));
}
