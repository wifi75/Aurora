//! Il punteggio dei contest del gruppo 3C con i file `.contest` distribuiti (`desktop/src-tauri/contest`) e le regole di
//! `docs/contest-regole.md`. Ogni prova ha un piccolo log con nominativi finti e il punteggio calcolato a mano dal regolamento.
//! La stazione e' IU3ABC (Italia, EU, zona CQ 15).

use contest::elenco::{ContestTesto, Dominio, data_ms, leggi_dominio};
use contest::punti_testo::{InfoPaese, Punteggio, QsoTesto, valuta};
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
    const T: [(&str, &str, &str, u8, u8); 26] = [
        ("UA9", "UA9", "AS", 17, 30),
        ("UA2", "UA2", "EU", 15, 29),
        ("R2F", "UA2", "EU", 15, 29),
        ("UA", "UA", "EU", 16, 29),
        ("UR", "UR", "EU", 16, 29),
        ("SM", "SM", "EU", 14, 18),
        ("SK", "SM", "EU", 14, 18),
        ("LA", "LA", "EU", 14, 18),
        ("ON", "ON", "EU", 14, 27),
        ("OT", "ON", "EU", 14, 27),
        ("GM", "GM", "EU", 14, 27),
        ("G", "G", "EU", 14, 27),
        ("EI", "EI", "EU", 14, 27),
        ("YO", "YO", "EU", 20, 28),
        ("YV", "YV", "SA", 9, 12),
        ("YB", "YB", "OC", 28, 51),
        ("YC", "YB", "OC", 28, 51),
        ("TA", "TA", "EU", 20, 39),
        ("XE", "XE", "NA", 6, 10),
        ("JA", "JA", "AS", 25, 45),
        ("DL", "DL", "EU", 14, 28),
        ("IK", "I", "EU", 15, 28),
        ("I", "I", "EU", 15, 28),
        ("W", "K", "NA", 5, 8),
        ("K", "K", "NA", 5, 8),
        ("N", "K", "NA", 5, 8),
    ];
    T.iter().find(|t| call.starts_with(t.0)).map(|t| InfoPaese { dxcc: t.1.into(), continente: t.2.into(), cq: t.3, itu: t.4 })
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

/// Un QSO con il numero progressivo mandato.
fn n(call: &str, band: &str, mode: &str, min: i64, rcvd: &str) -> QsoTesto {
    q(call, band, mode, min, &format!("{:03}", min + 1), rcvd)
}

fn punteggio(nome: &str, log: &[QsoTesto], dom: Option<&Dominio>) -> Punteggio {
    let (_, t) = valuta(&contest(nome), log, Some(&mio()), &paese, dom);
    assert_eq!(t.errore_script, None, "{nome}");
    t
}

#[test]
fn russian_dx() {
    let dom = dominio("dom/russian.dom");
    let log = [
        n("UA3ABC", "20m", "CW", 0, "MA"),   // Russia: 10; UA e MA su 20 m
        n("DL1ABC", "20m", "CW", 1, "001"),  // stesso continente: 3; DL
        n("IK2XYZ", "20m", "CW", 2, "001"),  // stesso paese: 2; I
        n("W1AW", "40m", "CW", 3, "001"),    // altro continente: 5; K su 40 m
        n("UA3ABC", "20m", "SSB", 4, "MA"),  // altro modo: 10, niente di nuovo (moltiplicatori per banda)
        n("DL1ABC", "20m", "CW", 5, "002"),  // doppio
    ];
    let t = punteggio("RUSSIAN DX Contest", &log, Some(&dom));
    // punti 10+3+2+5+10 = 30; moltiplicatori 5; 30 x 5 = 150
    assert_eq!((t.qso, t.doppi, t.punti, t.molt, t.punteggio), (5, 1, 30, 5, 150));
}

#[test]
fn russian_ww_multimode() {
    let dom = dominio("dom/russian.dom");
    let log = [
        n("UA3ABC", "20m", "CW", 0, "MA"),   // altro paese: 3; UA e MA in CW su 20 m
        n("UA3ABC", "20m", "RTTY", 1, "MA"), // 3; UA e MA in RTTY su 20 m
        n("W1AW", "40m", "CW", 2, "001"),    // altro continente 5, x2 su 40 m = 10; K
        n("IK2XYZ", "80m", "SSB", 3, "001"), // stesso paese 1, x2 = 2; I
    ];
    let t = punteggio("Russian WW MultiMode Contest", &log, Some(&dom));
    // punti 18; moltiplicatori 6; 18 x 6 = 108
    assert_eq!((t.punti, t.molt, t.punteggio), (18, 6, 108));
}

#[test]
fn scandinavian_activity() {
    let log = [
        n("SM3ABC", "20m", "CW", 0, "001"), // 1 punto; area SM3 su 20 m
        n("SK3XYZ", "20m", "CW", 1, "001"), // 1; SK3 e' la stessa area SM3
        n("LA1ABC", "40m", "CW", 2, "001"), // europeo: 1 anche su 40 m; LA1
        n("DL1ABC", "20m", "CW", 3, "001"), // non scandinavo: non ammesso
    ];
    let t = punteggio("Scandinavian Activity Contest", &log, None);
    // punti 3; moltiplicatori SM3, LA1 = 2; 3 x 2 = 6
    assert_eq!((t.esclusi, t.punti, t.molt, t.punteggio), (1, 3, 2, 6));
}

#[test]
fn stew_perry() {
    let s = |call: &str, min: i64, g: &str| q(call, "160m", "CW", min, "JN65", g);
    let log = [
        s("IK2XYZ", 0, "JN55"), // 156 km: 1 punto
        s("G4ABC", 1, "IO91"),  // 1225 km: 1 + 2 = 3
        s("W1AW", 2, "FN42"),   // 6404 km: 1 + 12 = 13
        s("UR5ABC", 3, "KN23"), // 976 km: 1 + 1 = 2
        q("DL1ABC", "80m", "CW", 4, "JN65", "JO62"), // 80 m: non ammesso
    ];
    let t = punteggio("Stew Perry Topband Distance Challenge", &log, None);
    // nessun moltiplicatore: 1+3+13+2 = 19
    assert_eq!((t.esclusi, t.punti, t.punteggio), (1, 19, 19));
}

#[test]
fn trc_dx() {
    let log = [
        n("TA1ABC", "20m", "CW", 0, "001TRC"), // stazione TRC: 10; TA e TA TRC in CW su 20 m
        n("DL1ABC", "20m", "CW", 1, "002"),    // stesso continente: 1; DL
        n("W1AW", "20m", "SSB", 2, "003"),     // altro continente: 2; K in SSB
        n("DL1ABC", "20m", "SSB", 3, "004"),   // altro modo: 1; DL in SSB
    ];
    let t = punteggio("TRC DX Contest", &log, None);
    // punti 14; moltiplicatori 5; 14 x 5 = 70
    assert_eq!((t.punti, t.molt, t.punteggio), (14, 5, 70));
    // fra due soci TRC un punto solo
    let soci = [q("TA1ABC", "20m", "CW", 0, "001TRC", "001TRC")];
    assert_eq!(punteggio("TRC DX Contest", &soci, None).punti, 1);
}

#[test]
fn trc_digi() {
    let log = [
        n("TA1ABC", "20m", "RTTY", 0, "001TRC"), // 10; TA e TA TRC su 20 m
        n("W1AW", "20m", "RTTY", 1, "001"),      // 2; K
        n("DL1ABC", "20m", "BPSK63", 2, "001"),  // non RTTY: non ammesso
        n("DL1ABC", "40m", "RTTY", 3, "001"),    // 1; DL su 40 m
    ];
    let t = punteggio("TRC DIGI Contest", &log, None);
    // punti 13; moltiplicatori 4; 13 x 4 = 52
    assert_eq!((t.esclusi, t.punti, t.molt, t.punteggio), (1, 13, 4, 52));
}

#[test]
fn ua2_qso_party() {
    let log = [
        n("R2FA", "40m", "CW", 0, "KA03"),    // R2F in CW: 6; distretto KA03 su 40 m, 15 punti
        n("UA2FX", "40m", "SSB", 1, "KA03"),  // SSB: 3; distretto gia' preso su 40 m
        n("UA2FX", "20m", "CW", 2, "KA05"),   // 2; KA05 su 20 m, 15 punti
        n("DL1ABC", "80m", "SSB", 3, "005"),  // 3
    ];
    let t = punteggio("UA2 QSO Party", &log, None);
    // punti 6+3+2+3 = 14; due distretti nuovi x 15 = 30; 14 + 30 = 44
    assert_eq!((t.punti, t.molt, t.punteggio), (14, 2, 44));
}

#[test]
fn uba_dx() {
    let log = [
        n("ON4ABC", "20m", "CW", 0, "001 ANT"), // Belgio: 10; sezione ANT e prefisso ON4 su 20 m
        n("DL1ABC", "20m", "CW", 1, "001"),     // paese UE: 3; DL
        n("W1AW", "20m", "CW", 2, "001"),       // 1
        n("UA3ABC", "20m", "CW", 3, "001"),     // Russia: non ammesso
        n("OT4XYZ", "40m", "CW", 4, "005 XXX"), // Belgio: 10; XXX e OT4 su 40 m
    ];
    let t = punteggio("UBA CW contest", &log, None);
    // punti 24; moltiplicatori ANT, ON4, DL, XXX, OT4 = 5; 24 x 5 = 120
    assert_eq!((t.esclusi, t.punti, t.molt, t.punteggio), (1, 24, 5, 120));
    let ssb = [n("ON4ABC", "80m", "SSB", 0, "001 ANT"), n("DL1ABC", "80m", "SSB", 1, "001")];
    // 10 + 3 = 13; ANT, ON4, DL = 3; 39
    assert_eq!(punteggio("UBA SSB Contest", &ssb, None).punteggio, 39);
}

#[test]
fn uba_spring() {
    let log = [
        n("ON4ABC", "80m", "CW", 0, "001 ANT"), // 3; sezione ANT
        n("OT4XYZ", "80m", "CW", 1, "002 ANT"), // 3; ANT gia' presa
        n("ON5ZZ", "80m", "CW", 2, "003 XXX"),  // 3; XXX
        n("DL1ABC", "80m", "CW", 3, "004"),     // non ON: non ammesso
    ];
    let t = punteggio("UBA Spring 80M CW Contest", &log, None);
    // 9 x 2 = 18
    assert_eq!((t.esclusi, t.punti, t.molt, t.punteggio), (1, 9, 2, 18));
    let ssb = [n("ON4ABC", "80m", "SSB", 0, "001 ANT")];
    assert_eq!(punteggio("UBA Spring 80M SSB Contest", &ssb, None).punteggio, 3);
    let sei = [n("ON4ABC", "6m", "SSB", 0, "001 ANT"), n("ON4ABC", "2m", "SSB", 1, "001 ANT")];
    let t = punteggio("UBA Spring 6M SSB-CW Contest", &sei, None);
    assert_eq!((t.esclusi, t.punteggio), (1, 3));
    let due = [n("ON4ABC", "2m", "SSB", 0, "001 ANT"), n("ON4ABC", "6m", "SSB", 1, "001 ANT")];
    let t = punteggio("UBA Spring 2M SSB Contest", &due, None);
    assert_eq!((t.esclusi, t.punteggio), (1, 3));
}

#[test]
fn uk_ei_dx() {
    let log = [
        n("G4ABC", "80m", "CW", 0, "LN"),   // europeo con UK su 80 m: 4; distretto LN su 80 m
        n("DL1ABC", "80m", "CW", 1, "001"), // Europa su 80 m: 2; DL
        n("W1AW", "20m", "CW", 2, "001"),   // DX su 20 m: 2; K
        n("GM3XYZ", "20m", "CW", 3, "SA"),  // UK su 20 m: 2; distretto SA (nessun paese)
        n("EI5AB", "20m", "CW", 4, "DU"),   // 2; distretto DU
    ];
    let t = punteggio("UK_EI DX Contest", &log, None);
    // punti 12; moltiplicatori 5; 12 x 5 = 60
    assert_eq!((t.punti, t.molt, t.punteggio), (12, 5, 60));
}

#[test]
fn ukrainian_dx() {
    let dom = dominio("dom/udxc.dom");
    let log = [
        n("UR5ABC", "20m", "CW", 0, "VI"),  // Ucraina: 10; UR e VI su 20 m
        n("DL1ABC", "20m", "CW", 1, "001"), // 2; DL
        n("IK2XYZ", "40m", "CW", 2, "001"), // 1; I su 40 m
        n("W1AW", "40m", "SSB", 3, "001"),  // 3; K su 40 m
    ];
    let t = punteggio("Ukrainian DX Contest", &log, Some(&dom));
    // punti 16; moltiplicatori 5; 16 x 5 = 80
    assert_eq!((t.punti, t.molt, t.punteggio), (16, 5, 80));
}

#[test]
fn venezuelan_independence() {
    let log = [
        n("YV5ABC", "20m", "SSB", 0, "DC"), // altro continente: 5; YV su 20 m, circuito DC
        n("YV5ABC", "20m", "CW", 1, "DC"),  // altro modo: 5, niente di nuovo
        n("DL1ABC", "40m", "SSB", 2, "001"), // 3; DL su 40 m
        n("IK2XYZ", "40m", "SSB", 3, "001"), // 1; I su 40 m
        n("YV4XX", "40m", "SSB", 4, "AR"),   // 5; YV su 40 m, circuito AR
    ];
    let t = punteggio("Venezuelan Independence Contest", &log, None);
    // punti 19; moltiplicatori 6; 19 x 6 = 114
    assert_eq!((t.punti, t.molt, t.punteggio), (19, 6, 114));
}

#[test]
fn waedc_cw_ssb() {
    let log = [
        n("W1AW", "80m", "CW", 0, "001"),   // 1; area K1 su 80 m, peso 4
        n("W2XYZ", "80m", "CW", 1, "001"),  // 1; K2 su 80 m, peso 4
        n("JA1ABC", "20m", "CW", 2, "001"), // 1; JA1 su 20 m, peso 2
        n("XE1ABC", "40m", "CW", 3, "001"), // 1; XE su 40 m, peso 3
        n("DL1ABC", "20m", "CW", 4, "001"), // Europa con Europa: non ammesso
    ];
    let t = punteggio("WAEDC CW-SSB Contest", &log, None);
    // punti 4; moltiplicatori 4+4+2+3 = 13; 4 x 13 = 52
    assert_eq!((t.esclusi, t.punti, t.molt, t.punteggio), (1, 4, 13, 52));
}

#[test]
fn yb_dx() {
    let log = |m: &str| {
        [
            n("YB1ABC", "20m", m, 0, "001"), // YB: 10; prefisso YB1 e YB su 20 m
            n("YC2XYZ", "20m", m, 1, "001"), // 10; YC2
            n("DL1ABC", "20m", m, 2, "001"), // 2; DL
            n("IK2XYZ", "40m", m, 3, "001"), // 1; I su 40 m
            n("JA1ABC", "15m", m, 4, "001"), // 3; JA su 15 m
        ]
    };
    // punti 26; moltiplicatori 6; 26 x 6 = 156
    let t = punteggio("YB DX Contest", &log("SSB"), None);
    assert_eq!((t.punti, t.molt, t.punteggio), (26, 6, 156));
    let t = punteggio("YB DX RTTY Contest", &log("RTTY"), None);
    assert_eq!((t.punti, t.molt, t.punteggio), (26, 6, 156));
}

#[test]
fn yo_dx_hf() {
    let log = [
        n("YO3ABC", "20m", "CW", 0, "BU"),  // YO: 8; YO e distretto BU su 20 m
        n("DL1ABC", "20m", "CW", 1, "001"), // 2; DL
        n("IK2XYZ", "20m", "CW", 2, "001"), // 1; I
        n("W1AW", "40m", "CW", 3, "001"),   // 4; K su 40 m
        n("YO8XX", "20m", "SSB", 4, "IS"),  // 8; IS su 20 m
    ];
    let t = punteggio("YO DX HF Contest", &log, None);
    // punti 23; moltiplicatori 6; 23 x 6 = 138
    assert_eq!((t.punti, t.molt, t.punteggio), (23, 6, 138));
}
