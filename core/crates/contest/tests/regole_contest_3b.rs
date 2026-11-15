//! Il punteggio dei contest del gruppo 3B con i file `.contest` distribuiti (`desktop/src-tauri/contest`) e le regole di
//! `docs/contest-regole.md` (sezione «Gruppo 3B»). Ogni prova ha un piccolo log con nominativi finti e il punteggio calcolato a mano
//! dal regolamento. La stazione e' IU3ABC (Italia, EU, zona CQ 15, zona ITU 28), salvo dove la prova dice altro.

use contest::elenco::{ContestTesto, data_ms};
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

/// Un piccolo database DXCC finto: (prefisso del nominativo, paese, continente, zona CQ, zona ITU), il primo che combacia.
fn paese(call: &str) -> Option<InfoPaese> {
    const T: [(&str, &str, &str, u8, u8); 26] = [
        ("UA9", "UA9", "AS", 17, 30),
        ("FM", "FM", "NA", 8, 11),
        ("HA", "HA", "EU", 15, 28),
        ("HB", "HB", "EU", 14, 28),
        ("4X", "4X", "AS", 20, 39),
        ("LU", "LU", "SA", 13, 14),
        ("EA", "EA", "EU", 14, 37),
        ("OK", "OK", "EU", 15, 28),
        ("OM", "OM", "EU", 15, 28),
        ("VE", "VE", "NA", 4, 9),
        ("VA", "VE", "NA", 4, 9),
        ("JA", "JA", "AS", 25, 45),
        ("DL", "DL", "EU", 14, 28),
        ("G", "G", "EU", 14, 27),
        ("SP", "SP", "EU", 15, 28),
        ("UA", "UA", "EU", 16, 29),
        ("RA", "UA", "EU", 16, 29),
        ("R", "UA", "EU", 16, 29),
        ("ON", "ON", "EU", 14, 27),
        ("F", "F", "EU", 14, 27),
        ("I", "I", "EU", 15, 28),
        ("W", "K", "NA", 5, 8),
        ("K", "K", "NA", 5, 8),
        ("N", "K", "NA", 5, 8),
        ("VK", "VK", "OC", 30, 59),
        ("PY", "PY", "SA", 11, 15),
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

/// Un QSO in CW con il numero progressivo mandato.
fn c(call: &str, band: &str, min: i64, rcvd: &str) -> QsoTesto {
    q(call, band, "CW", min, &format!("{:03}", min + 1), rcvd)
}

fn punteggio(nome: &str, log: &[QsoTesto]) -> Punteggio {
    valuta(&contest(nome), log, Some(&mio()), &paese, None).1
}

fn punteggio_da(nome: &str, log: &[QsoTesto], mia: &str) -> Punteggio {
    valuta(&contest(nome), log, paese(mia).as_ref(), &paese, None).1
}

#[test]
fn french_hf_championship() {
    let log = [
        c("F5ABC", "20m", 0, "75"),   // Francia, stesso continente: 1; dipartimento 75
        c("F6REF", "20m", 1, "00"),   // 1; F6REF manda 00
        c("FM5ABC", "40m", 2, "FM"),  // Martinica, altro continente: 3; FM su 40 m
        c("DL1ABC", "20m", 3, "001"), // non francese: 0 punti, nessun moltiplicatore
        c("F5ABC", "20m", 4, "75"),   // doppio
    ];
    let t = punteggio("French HF Championship", &log);
    // punti 1+1+3 = 5; moltiplicatori 75, 00, FM = 3; 5 x 3 = 15
    assert_eq!((t.doppi, t.punti, t.molt, t.punteggio), (1, 5, 3, 15));
}

#[test]
fn gacw_wwsa() {
    let log = [
        c("IK2XYZ", "20m", 0, "15"),  // stesso paese: 0 punti; zona 15 e I su 20 m
        c("LU1ABC", "20m", 1, "13"),  // Sud America da fuori: 5; zona 13, LU
        c("W1AW", "20m", 2, "05"),    // altro continente: 3; zona 5, K
        c("DL1ABC", "40m", 3, "14"),  // stesso continente: 1; zona 14 e DL su 40 m
    ];
    let t = punteggio("GACW WWSA CW DX Contest", &log);
    // punti 0+5+3+1 = 9; moltiplicatori 8; 9 x 8 = 72
    assert_eq!((t.punti, t.molt, t.punteggio), (9, 8, 72));
}

#[test]
fn ha_dx() {
    let log = [
        c("HA5ABC", "20m", 0, "BP"),  // stazione HA: 10; contea BP su 20 m
        c("HA5ABC", "20m", 1, "BP"),  // doppio
        c("HA5ABC", "40m", 2, "BP"),  // 10; BP su 40 m
        c("DL1ABC", "20m", 3, "001"), // stesso continente: 2; DL
        c("W1AW", "80m", 4, "001"),   // altro continente: 5; K su 80 m
    ];
    let t = punteggio("HA-DX Contest  SSB-CW Contest", &log);
    // punti 10+10+2+5 = 27; moltiplicatori BP 20, BP 40, DL, K = 4 (HA non e' paese); 27 x 4 = 108
    assert_eq!((t.doppi, t.punti, t.molt, t.punteggio), (1, 27, 4, 108));
}

#[test]
fn helvetia() {
    let log = [
        c("HB9ABC", "20m", 0, "ZH"),  // stazione HB: 10; cantone ZH
        c("HB9ABD", "20m", 1, "BE"),  // 10; cantone BE
        c("DL1ABC", "20m", 2, "001"), // stesso continente: 1; DL
        c("W1AW", "15m", 3, "001"),   // altro continente: 3; K su 15 m
    ];
    let t = punteggio("Helvetia Contest", &log);
    // punti 24; moltiplicatori ZH, BE, DL, K = 4; 24 x 4 = 96
    assert_eq!((t.punti, t.molt, t.punteggio), (24, 4, 96));
}

#[test]
fn holyland() {
    let log = [
        c("4X1AB", "20m", 0, "TA"),   // stazione 4X: 8; area TA e paese 4X su 20 m
        c("IK2XYZ", "20m", 1, "001"), // stesso paese: 1; I
        c("DL1ABC", "40m", 2, "001"), // stesso continente: 2; DL su 40 m
        c("W1AW", "20m", 3, "001"),   // altro continente: 4; K
    ];
    let t = punteggio("Holyland-DX Contest", &log);
    // punti 8+1+2+4 = 15; moltiplicatori TA, 4X, I, DL, K = 5; 15 x 5 = 75
    assert_eq!((t.punti, t.molt, t.punteggio), (15, 5, 75));
}

#[test]
fn iaru_region1_field_day() {
    let log = [
        c("DL1ABC/P", "40m", 0, "001"), // portatile in Europa: 4; DL su 40 m
        c("DL2ABC", "40m", 1, "002"),   // fissa in Europa: 2; DL gia' preso
        c("W1AW/P", "20m", 2, "003"),   // portatile fuori Europa: 6; K
        c("JA1ABC", "20m", 3, "004"),   // fissa fuori Europa: 3; JA
    ];
    let t = punteggio("IARU Region 1 Field Day", &log);
    // punti 15; moltiplicatori 3; 15 x 3 = 45
    assert_eq!((t.punti, t.molt, t.punteggio), (15, 3, 45));
}

#[test]
fn igry_ww_rtty() {
    let log = [
        q("DL1ABC", "20m", "RTTY", 0, "1999", "1985"), // 1; anno 1985 su 20 m
        q("W1AW", "20m", "RTTY", 1, "1999", "1970"),   // 1; 1970
        q("JA1ABC", "20m", "RTTY", 2, "1999", "1985"), // 1; 1985 gia' preso
        q("DL1ABC", "40m", "RTTY", 3, "1999", "1985"), // 1; 1985 su 40 m
        q("K1ABC", "160m", "RTTY", 4, "1999", "1990"), // 160 m non ammessa
    ];
    let t = punteggio("IGRY WW RTTY Contest", &log);
    // punti 4; moltiplicatori 3; 4 x 3 = 12
    assert_eq!((t.esclusi, t.punti, t.molt, t.punteggio), (1, 4, 3, 12));
}

#[test]
fn marconi_memorial() {
    let log = [
        c("DL1ABC", "20m", 0, "001"),
        c("DL2ABC", "20m", 1, "001"),
        c("DL1ABC", "40m", 2, "002"),
        c("W1AW", "10m", 3, "001"),
        c("IK2XYZ", "160m", 4, "001"),
    ];
    let t = punteggio("Marconi Memorial Contest", &log);
    // 5 punti; paesi per banda DL 20, DL 40, K 10, I 160 = 4; 5 x 4 = 20
    assert_eq!((t.punti, t.molt, t.punteggio), (5, 4, 20));
}
