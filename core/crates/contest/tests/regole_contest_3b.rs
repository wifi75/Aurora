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
    const T: [(&str, &str, &str, u8, u8); 27] = [
        ("EF", "EA", "EU", 14, 37),
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

#[test]
fn japan_international_dx() {
    for nome in ["Japan International DX CW Contest", "Japan International DX SSB Contest"] {
        let log = [
            c("JA1ABC", "20m", 0, "13"),  // JA: 1 punto; prefettura 13 su 20 m
            c("JA2ABC", "80m", 1, "20"),  // 1 x 2 su 80 m = 2; prefettura 20
            c("JA3ABC", "160m", 2, "25"), // 1 x 4 su 160 m = 4; prefettura 25
            c("DL1ABC", "20m", 3, "14"),  // DX con DX: niente
            c("JA1ABC", "10m", 4, "13"),  // 1 x 2 su 10 m = 2; prefettura 13 su 10 m
        ];
        let t = punteggio(nome, &log);
        // punti 1+2+4+2 = 9; moltiplicatori 4; 9 x 4 = 36
        assert_eq!((t.punti, t.molt, t.punteggio), (9, 4, 36), "{nome}");
    }
    // dal Giappone: paesi e zone CQ per banda, solo i QSO con i DX
    let log = [
        c("W1AW", "20m", 0, "05"),   // 1; K e zona 5
        c("JA2ABC", "20m", 1, "20"), // JA con JA: niente
        c("DL1ABC", "40m", 2, "14"), // 1; DL e zona 14 su 40 m
    ];
    let t = punteggio_da("Japan International DX CW Contest", &log, "JA1XYZ");
    assert_eq!((t.punti, t.molt, t.punteggio), (2, 4, 8));
}

#[test]
fn kcj() {
    let log = [
        c("JA1ABC", "20m", 0, "TK"), // DX con JA: 2; TK su 20 m
        c("JA2ABC", "20m", 1, "AC"), // 2; AC
        c("DL1ABC", "20m", 2, "14"), // DX con DX: 1, nessun moltiplicatore
        c("JA1ABC", "6m", 3, "TK"),  // 2; TK su 6 m
    ];
    let t = punteggio("KCJ Contest", &log);
    // punti 7; moltiplicatori 3; 7 x 3 = 21
    assert_eq!((t.punti, t.molt, t.punteggio), (7, 3, 21));
}

#[test]
fn king_of_spain_cw() {
    let log = [
        c("EA1ABC", "20m", 0, "M"),   // DX con EA: 3; EA e provincia M
        c("EA2ABC", "20m", 1, "BI"),  // 3; BI
        c("DL1ABC", "20m", 2, "001"), // DX con DX: 1; DL
        c("EF0F", "40m", 3, "SMR"),   // 3; EA su 40 m e SMR
    ];
    let t = punteggio("King of Spain CW Contest", &log);
    // punti 10; moltiplicatori EA 20, M, BI, DL, EA 40, SMR = 6; 10 x 6 = 60
    assert_eq!((t.punti, t.molt, t.punteggio), (10, 6, 60));
}

#[test]
fn ok_om_dx_cw() {
    let log = [
        c("OK1ABC", "20m", 0, "PRC"), // stazione OK: 10; distretto PRC e OK
        c("OM3ABC", "20m", 1, "BAA"), // 10; BAA e OM
        c("IK2XYZ", "20m", 2, "001"), // stesso paese: 1; I
        c("DL1ABC", "40m", 3, "001"), // stesso continente: 3; DL su 40 m
        c("W1AW", "15m", 4, "001"),   // altro continente: 5; K su 15 m
    ];
    let t = punteggio("OK-OM-DX-CW Contest", &log);
    // punti 29; moltiplicatori 7; 29 x 7 = 203
    assert_eq!((t.punti, t.molt, t.punteggio), (29, 7, 203));
}

#[test]
fn ok_om_dx_ssb() {
    let log = [q("OK1ABC", "80m", "SSB", 0, "001", "PRC"), q("JA1ABC", "20m", "SSB", 1, "002", "001")];
    let t = punteggio("OK-OM-DX-SSB Contest", &log);
    // 10 + 5 = 15; PRC, OK, JA = 3; 45
    assert_eq!((t.punti, t.molt, t.punteggio), (15, 3, 45));
    // da OK: stesso paese 2, OM 3, altro continente 5
    let log = [q("OK2ABC", "20m", "SSB", 0, "PRC", "BRN"), q("OM3ABC", "20m", "SSB", 1, "PRC", "BAA"), q("W1AW", "20m", "SSB", 2, "PRC", "001")];
    let t = punteggio_da("OK-OM-DX-SSB Contest", &log, "OK1XYZ");
    // 2 + 3 + 5 = 10; BRN, OK, BAA, OM, K = 5; 50
    assert_eq!((t.punti, t.molt, t.punteggio), (10, 5, 50));
}

#[test]
fn pdc() {
    let log = [
        q("IK2XYZ", "20m", "BPSK63", 0, "001", "001"), // stesso paese: 1; prefisso del proprio paese escluso
        q("DL1ABC", "20m", "BPSK63", 1, "002", "001"), // 2; DL1 su 20 m
        q("DL1ABC", "20m", "RTTY", 2, "003", "002"),   // altro modo, non doppio: 2
        q("DL1ABC", "20m", "BPSK63", 3, "004", "003"), // doppio
        q("W1AW", "40m", "FT4", 4, "005", "001"),      // 2; W1 su 40 m
    ];
    let t = punteggio("PDC Contest", &log);
    // punti 7; prefissi 2; 7 x 2 = 14
    assert_eq!((t.doppi, t.punti, t.molt, t.punteggio), (1, 7, 2, 14));
}

#[test]
fn rac() {
    let log = [
        q("VA3RAC", "20m", "CW", 0, "001", "ON"),  // stazione ufficiale RAC: 20; ON su 20 m in CW
        q("VE3ABC", "20m", "CW", 1, "002", "ON"),  // Canada: 10
        q("VE3ABC", "20m", "SSB", 2, "003", "ON"), // altro modo: 10; ON su 20 m in fonia
        q("DL1ABC", "40m", "CW", 3, "004", "001"), // estero: 2
    ];
    let t = punteggio("RAC Contest", &log);
    // punti 42; moltiplicatori 2; 84
    assert_eq!((t.punti, t.molt, t.punteggio), (42, 2, 84));
    // senza QSO con il Canada i moltiplicatori valgono 1
    let t = punteggio("RAC Contest", &[q("DL1ABC", "40m", "CW", 0, "001", "001"), q("W1AW", "20m", "CW", 1, "002", "001")]);
    assert_eq!((t.punti, t.punteggio, t.errore_script.clone()), (4, 4, None));
}

#[test]
fn hsc_cw() {
    let log = [
        c("DL1ABC", "20m", 0, "1234"), // socio HSC: 5; DL su 20 m
        c("W1AW", "20m", 1, "NM"),     // non socio: 2; K
        c("DL2ABC", "40m", 2, "NM"),   // 2; DL su 40 m
    ];
    let t = punteggio("HSC-CW Contest", &log);
    // punti 9; moltiplicatori 3; 27
    assert_eq!((t.punti, t.molt, t.punteggio, t.errore_script.clone()), (9, 3, 27, None));
}

#[test]
fn iaru_hf() {
    let log = [
        c("DL1ABC", "20m", 0, "28"),  // stessa zona ITU: 1; zona 28
        c("EA1ABC", "20m", 1, "37"),  // stesso continente, altra zona: 3; zona 37
        c("W1AW", "20m", 2, "08"),    // altro continente e zona: 5; zona 8
        c("DA0HQ", "20m", 3, "DARC"), // stazione HQ: 1; DARC
        c("W1AW", "40m", 4, "08"),    // 5; zona 8 su 40 m
    ];
    let t = punteggio("IARU HF World-Championship-CW-SSB Contest", &log);
    // punti 15; moltiplicatori 28, 37, 8, DARC su 20 m e 8 su 40 m = 5; 75
    assert_eq!((t.punti, t.molt, t.punteggio, t.errore_script.clone()), (15, 5, 75, None));
}

#[test]
fn iota() {
    let log = [
        q("G3ABC", "20m", "CW", 0, "001", "001 EU005"),     // World con Island: 15; EU005 in CW su 20 m
        q("DL1ABC", "20m", "CW", 1, "002", "002"),          // World con World: 2
        q("G3ABC", "20m", "SSB", 2, "003", "003 EU005"),    // altro modo: 15; EU005 in SSB
        q("GM3ABC", "40m", "CW", 3, "004", "004 EU-010"),   // 15; EU010 su 40 m
    ];
    let t = punteggio("IOTA_CW-SSB", &log);
    // punti 47; moltiplicatori 3; 141
    assert_eq!((t.punti, t.molt, t.punteggio, t.errore_script.clone()), (47, 3, 141, None));
    // da un'isola (EU130): World 5, stessa isola 5, altra isola 15
    let log = [
        q("G3ABC", "20m", "CW", 0, "001 EU130", "001 EU005"),
        q("DL1ABC", "20m", "CW", 1, "002 EU130", "002"),
        q("IK1ABC", "20m", "CW", 2, "003 EU130", "003 EU130"),
    ];
    let t = punteggio("IOTA_CW-SSB", &log);
    assert_eq!((t.punti, t.molt, t.punteggio), (25, 2, 50));
}

#[test]
fn ipa() {
    let log = [
        c("DL1ABC", "20m", 0, "001 IPA"),    // socio IPARC: 5; DL su 20 m
        c("DL2ABC", "20m", 1, "002"),        // 1
        c("W1AW", "20m", 2, "003 IPA NY"),   // 5; stato NY
        c("W2ABC", "20m", 3, "004 IPA CA"),  // 5; stato CA
        c("DL3ABC", "40m", 4, "005 IPA"),    // 5; DL su 40 m
    ];
    let t = punteggio("IPA CW-SSB Contest", &log);
    // banda per banda: 20 m punti 16 x 3 moltiplicatori = 48, 40 m 5 x 1 = 5; 53
    assert_eq!((t.punti, t.molt, t.punteggio, t.errore_script.clone()), (21, 4, 53, None));
}

#[test]
fn r3a_cup_digi() {
    let log = [
        q("RA3ABC", "80m", "RTTY", 0, "001", "LF"),   // Mosca: 5; LF su 80 m
        q("RA1ABC", "80m", "RTTY", 1, "002", "MO54"), // Russia: 1; MO54
        q("UA9ABC", "40m", "RTTY", 2, "001", "NS01"), // 1; NS01 su 40 m
        q("DL1ABC", "40m", "RTTY", 3, "002", "005"),  // 1; nessun moltiplicatore
        q("RA3ABD", "20m", "RTTY", 4, "003", "LF"),   // 20 m non ammessa
        q("RA3ABC", "80m", "RTTY", 60, "004", "LF"),  // 13:00, altro turno: non e' un doppio, 5
        q("RA3ABC", "80m", "RTTY", 61, "005", "LF"),  // doppio nello stesso turno
    ];
    let t = punteggio("R3A-CUP-DIGI", &log);
    // punti 13; moltiplicatori 3; 39
    assert_eq!((t.esclusi, t.doppi, t.punti, t.molt, t.punteggio, t.errore_script.clone()), (1, 1, 13, 3, 39, None));
}

#[test]
fn rda() {
    let log = [
        c("RA3ABC", "20m", 0, "TB02"), // stazione russa: 10; distretto TB02
        c("UA9ABC", "40m", 1, "NS01"), // 10; NS01
        c("RA3ABC", "40m", 2, "TB02"), // 10; TB02 gia' preso (una volta sola)
        c("DL1ABC", "20m", 3, "001"),  // non russa: 0
    ];
    let t = punteggio("RDA Contest", &log);
    // punti 30; moltiplicatori 2; 60
    assert_eq!((t.punti, t.molt, t.punteggio, t.errore_script.clone()), (30, 2, 60, None));
    // dalla Russia europea: Russia stesso continente 1, altro 2, altro paese stesso continente 3, altro continente 5
    let log = [c("RA1ABC", "20m", 0, "SP01"), c("UA9ABC", "20m", 1, "NS01"), c("DL1ABC", "20m", 2, "001"), c("W1AW", "40m", 3, "001")];
    let t = punteggio_da("RDA Contest", &log, "RA3XYZ");
    // punti 11; SP01, NS01 e i paesi UA, UA9, DL su 20 m, K su 40 m = 6; 66
    assert_eq!((t.punti, t.molt, t.punteggio), (11, 6, 66));
}

#[test]
fn rsgb_160() {
    let log = [
        c("G3ABC", "160m", 0, "001 LN"),               // 2 + 5 per il distretto LN
        c("G4ABC", "160m", 1, "002 LN"),               // 2
        c("DL1ABC", "160m", 2, "003"),                 // 2 + 5 per il paese DL
        c("DL2ABC", "160m", 3, "004"),                 // 2
        q("G3ABC", "160m", "SSB", 4, "005", "005 LN"), // altro modo: 2
        c("G5ABC", "80m", 5, "006 LN"),                // 80 m non ammessa
    ];
    let t = punteggio("RSGB 160 M CONTEST", &log);
    // punti 20 con i bonus, nessun moltiplicatore
    assert_eq!((t.esclusi, t.punti, t.punteggio), (1, 20, 20));
}

#[test]
fn rsgb_80() {
    let log = [
        c("G3ABC", "80m", 0, "001"),                 // 1
        c("G6XX", "80m", 1, "002"),                  // stazione RSGB: 5
        c("DL1ABC", "80m", 2, "003"),                // nessuna delle due nel Regno Unito: non ammesso
        q("GM6XX", "80m", "SSB", 3, "004", "004"),   // 5
    ];
    let t = punteggio("RSGB 80 M CONTEST", &log);
    assert_eq!((t.esclusi, t.punti, t.punteggio, t.errore_script.clone()), (1, 11, 11, None));
}
