//! Il punteggio dei contest del gruppo 3A con i file `.contest` distribuiti (`desktop/src-tauri/contest`) e le regole di
//! `docs/contest-regole.md`. Ogni prova ha un piccolo log con nominativi finti e il punteggio calcolato a mano dal regolamento.
//! La stazione e' IU3ABC (Italia, EU, zona CQ 15), salvo dove la prova ne sceglie un'altra.

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
    const T: [(&str, &str, &str, u8, u8); 32] = [
        ("UA9", "UA9", "AS", 17, 30),
        ("EA8", "EA8", "AF", 33, 36),
        ("KH6", "KH6", "OC", 31, 61),
        ("KL7", "KL7", "NA", 1, 1),
        ("IT9", "IT9", "EU", 15, 28),
        ("IS0", "IS", "EU", 15, 28),
        ("EA", "EA", "EU", 14, 37),
        ("XE", "XE", "NA", 6, 10),
        ("SP", "SP", "EU", 15, 28),
        ("OK", "OK", "EU", 15, 28),
        ("LY", "LY", "EU", 15, 29),
        ("YL", "YL", "EU", 15, 29),
        ("ES", "ES", "EU", 15, 29),
        ("ZS", "ZS", "AF", 38, 57),
        ("PY", "PY", "SA", 11, 15),
        ("LU", "LU", "SA", 13, 14),
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
        ("ON", "ON", "EU", 14, 27),
        ("CT", "CT", "EU", 14, 37),
        ("SM", "SM", "EU", 14, 18),
        ("YU", "YU", "EU", 15, 28),
        ("F", "F", "EU", 14, 27),
        ("G", "G", "EU", 14, 27),
    ];
    T.iter().find(|t| call.starts_with(t.0)).map(|t| InfoPaese { dxcc: t.1.into(), continente: t.2.into(), cq: t.3, itu: t.4, ..InfoPaese::default() })
}

fn q(call: &str, band: &str, mode: &str, min: i64, rcvd: &str) -> QsoTesto {
    QsoTesto {
        call: call.into(),
        band: band.into(),
        mode: mode.into(),
        freq_hz: 0,
        time_ms: data_ms("20261114120000").unwrap_or(0) + min * 60_000,
        rst_sent: "599".into(),
        rst_rcvd: "599".into(),
        exch_sent: format!("{:03}", min + 1),
        exch_rcvd: rcvd.into(),
    }
}

/// Il punteggio con la stazione `mia` (un nominativo del database finto).
fn punteggio_da(mia: &str, nome: &str, log: &[QsoTesto]) -> Punteggio {
    let mio = paese(mia).unwrap_or_default();
    let t = valuta(&contest(nome), log, Some(&mio), &paese, None).1;
    assert_eq!(t.errore_script, None, "{nome}");
    t
}

fn punteggio(nome: &str, log: &[QsoTesto]) -> Punteggio {
    punteggio_da("IU3ABC", nome, log)
}

#[test]
fn ari_international() {
    let log = [
        q("DL1ABC", "20m", "CW", 0, "001"),  // da un italiano: Europa 1; DL su 20 m
        q("W1AW", "20m", "CW", 1, "001"),    // fuori Europa 3; K
        q("IK2XYZ", "20m", "CW", 2, "MI"),   // fra italiani non vale
        q("DL1ABC", "20m", "SSB", 3, "002"), // altro modo: 1, DL gia' preso su 20 m
        q("DL1ABC", "40m", "CW", 4, "003"),  // 1; DL su 40 m
    ];
    let t = punteggio("ARI International DX Contest", &log);
    // punti 1+3+1+1 = 6; moltiplicatori 3; 6 x 3 = 18
    assert_eq!((t.esclusi, t.punti, t.molt, t.punteggio), (1, 6, 3, 18));
    let log = [
        q("IK2XYZ", "20m", "CW", 0, "MI"),  // da un tedesco: con l'Italia 10; provincia MI (I non e' un paese moltiplicatore)
        q("DL2ABC", "20m", "CW", 1, "001"), // stesso paese 0; DL
        q("EA1ABC", "20m", "CW", 2, "001"), // stesso continente 1; EA
        q("W1AW", "20m", "CW", 3, "001"),   // altro continente 3; K
    ];
    let t = punteggio_da("DL1AAA", "ARI International DX Contest", &log);
    // punti 10+0+1+3 = 14; moltiplicatori MI, DL, EA, K = 4; 14 x 4 = 56
    assert_eq!((t.punti, t.molt, t.punteggio), (14, 4, 56));
}

#[test]
fn arrl_10m() {
    let log = [
        q("W1AW", "10m", "CW", 0, "CT"),    // CW 4; stato CT in CW
        q("W1AW", "10m", "SSB", 1, "CT"),   // fonia 2; CT di nuovo in fonia
        q("VE3ABC", "10m", "CW", 2, "ON"),  // 4; ON
        q("DL1ABC", "10m", "CW", 3, "001"), // 4; DL
        q("KH6ABC", "10m", "SSB", 4, "HI"), // 2; HI conta come stato
        q("JA1ABC", "20m", "CW", 5, "001"), // solo 10 m: non ammesso
    ];
    let t = punteggio("ARRL 10 meter Contest", &log);
    // punti 4+2+4+4+2 = 16; moltiplicatori 5; 16 x 5 = 80
    assert_eq!((t.esclusi, t.punti, t.molt, t.punteggio), (1, 16, 5, 80));
}

#[test]
fn arrl_dx() {
    let log = [
        q("W1AW", "20m", "CW", 0, "CT"),    // 3; CT su 20 m
        q("VE3ABC", "20m", "CW", 1, "ON"),  // 3; ON
        q("W1AW", "40m", "CW", 2, "CT"),    // 3; CT su 40 m
        q("DL1ABC", "20m", "CW", 3, "100"), // DX con DX: non vale
        q("KH6ABC", "20m", "CW", 4, "100"), // KH6 e' DX: non vale
    ];
    let t = punteggio("ARRL International DX-CW Contest", &log);
    // punti 9; moltiplicatori 3; 27
    assert_eq!((t.esclusi, t.punti, t.molt, t.punteggio), (2, 9, 3, 27));
    let log = [
        q("DL1ABC", "20m", "SSB", 0, "100"), // da un W: 3; DL su 20 m
        q("JA1ABC", "20m", "SSB", 1, "100"), // 3; JA
        q("KH6ABC", "20m", "SSB", 2, "100"), // 3; KH6 e' un paese
        q("VE3ABC", "20m", "SSB", 3, "ON"),  // W con VE: non vale
        q("DL1ABC", "15m", "SSB", 4, "100"), // 3; DL su 15 m
    ];
    let t = punteggio_da("W9ABC", "ARRL International DX-SSB Contest", &log);
    // punti 12; moltiplicatori 4; 48
    assert_eq!((t.esclusi, t.punti, t.molt, t.punteggio), (1, 12, 4, 48));
}

#[test]
fn arrl_field_day() {
    let log = [
        q("W1AW", "20m", "CW", 0, "1A CT"),   // CW 2
        q("W1AW", "20m", "SSB", 1, "1A CT"),  // fonia 1
        q("W1AW", "20m", "RTTY", 2, "1A CT"), // digitale 2
        q("VE3ABC", "40m", "USB", 3, "2A ON"), // 1
    ];
    let t = punteggio("ARRL-FD", &log);
    // punti 6, nessun moltiplicatore; moltiplicatore di potenza 2 (fino a 100 W): 12
    assert_eq!((t.punti, t.punteggio), (6, 12));
    // QRP o HP nello scambio mandato: 5 o 1
    let con = |sent: &str| log.iter().map(|x| QsoTesto { exch_sent: sent.into(), ..x.clone() }).collect::<Vec<_>>();
    assert_eq!(punteggio("ARRL-FD", &con("1B QRP EMA")).punteggio, 30);
    assert_eq!(punteggio("ARRL-FD", &con("3A HP EMA")).punteggio, 6);
}

#[test]
fn africa_all_mode() {
    let log = [
        q("ZS1ABC", "20m", "CW", 0, "001"),  // 1; ZS in CW
        q("ZS1ABC", "40m", "CW", 1, "002"),  // 1; ZS in CW gia' preso (su qualsiasi banda)
        q("ZS1ABC", "20m", "SSB", 2, "003"), // 1; ZS in SSB
        q("DL1ABC", "20m", "CW", 3, "001"),  // 1; non africano
        q("EA8ABC", "20m", "CW", 4, "001"),  // 1; EA8 in CW
    ];
    let t = punteggio("Africa All Mode International DX Contest", &log);
    // punti 5; moltiplicatori 3; 15
    assert_eq!((t.punti, t.molt, t.punteggio), (5, 3, 15));
}

#[test]
fn all_asian() {
    let log = [
        q("JA1ABC", "20m", "CW", 0, "45"),   // da fuori Asia: 1; prefisso JA1 su 20 m
        q("JA1ABC", "80m", "CW", 1, "45"),   // 2; JA1 su 80 m
        q("UA9ABC", "160m", "CW", 2, "50"),  // 3; UA9 su 160 m
        q("DL1ABC", "20m", "CW", 3, "40"),   // non asiatico: non vale
        q("JA2ABC", "10m", "CW", 4, "30"),   // 2; JA2 su 10 m
    ];
    let t = punteggio("All Asian DX Contest", &log);
    // punti 1+2+3+2 = 8; moltiplicatori 4; 32
    assert_eq!((t.esclusi, t.punti, t.molt, t.punteggio), (1, 8, 4, 32));
    let log = [
        q("JA2ABC", "20m", "CW", 0, "30"),  // da un JA: stesso paese, non vale
        q("UA9ABC", "20m", "CW", 1, "50"),  // asiatico 1; paese UA9 su 20 m
        q("DL1ABC", "20m", "CW", 2, "40"),  // non asiatico 3; DL
        q("DL1ABC", "160m", "CW", 3, "40"), // 9 su 160 m; DL su 160 m
        q("W1AW", "80m", "CW", 4, "60"),    // 6 su 80 m; K
    ];
    let t = punteggio_da("JA1AAA", "All Asian DX Contest", &log);
    // punti 1+3+9+6 = 19; moltiplicatori 4; 76
    assert_eq!((t.esclusi, t.punti, t.molt, t.punteggio), (1, 19, 4, 76));
}

#[test]
fn baltic() {
    let log = [
        q("LY2ABC", "80m", "CW", 0, "001"),  // dall'Europa con LY: 10
        q("YL2ABC", "80m", "SSB", 1, "001"), // con YL: 10
        q("DL1ABC", "80m", "CW", 2, "001"),  // altri: 1
        q("W1AW", "80m", "CW", 3, "001"),    // 1
        q("LY2ABC", "40m", "CW", 4, "002"),  // solo 80 m: non ammesso
    ];
    let t = punteggio("Baltic contest", &log);
    // nessun moltiplicatore: 22
    assert_eq!((t.esclusi, t.punti, t.punteggio), (1, 22, 22));
}

#[test]
fn cq_160() {
    let log = [
        q("IK2XYZ", "160m", "CW", 0, "15"),  // stesso paese 2; I
        q("DL1ABC", "160m", "CW", 1, "14"),  // stesso continente 5; DL
        q("W1AW", "160m", "CW", 2, "CT"),    // altro continente 10; stato CT
        q("VE3ABC", "160m", "CW", 3, "ON"),  // 10; ON
        q("W2ABC", "160m", "CW", 4, "CT"),   // 10; CT gia' preso
        q("JA1ABC", "20m", "CW", 5, "25"),   // solo 160 m: non ammesso
    ];
    let t = punteggio("CQ WW 160 meter contest", &log);
    // punti 37; moltiplicatori 4; 148
    assert_eq!((t.esclusi, t.punti, t.molt, t.punteggio), (1, 37, 4, 148));
}

#[test]
fn cq_ww_dx() {
    let log = [
        q("IK2XYZ", "20m", "CW", 0, "15"), // stesso paese 0; zona 15 e I
        q("DL1ABC", "20m", "CW", 1, "14"), // stesso continente 1; 14 e DL
        q("W1AW", "20m", "CW", 2, "5"),    // altro continente 3; 5 e K
        q("DL1ABC", "40m", "CW", 3, "14"), // 1; 14 e DL su 40 m
        q("W1AW", "20m", "CW", 4, "5"),    // doppio
    ];
    let t = punteggio("CQ WW DX CONTEST", &log);
    // punti 5; moltiplicatori 8; 40
    assert_eq!((t.doppi, t.punti, t.molt, t.punteggio), (1, 5, 8, 40));
    let log = [
        q("VE3ABC", "20m", "SSB", 0, "4"),  // da un W, Nord America: 2; 4 e VE
        q("XE1ABC", "20m", "SSB", 1, "6"),  // 2; 6 e XE
        q("DL1ABC", "20m", "SSB", 2, "14"), // 3; 14 e DL
        q("W2ABC", "20m", "SSB", 3, "5"),   // stesso paese 0; 5 e K
    ];
    let t = punteggio_da("W9ABC", "CQ WW DX CONTEST", &log);
    // punti 7; moltiplicatori 8; 56
    assert_eq!((t.punti, t.molt, t.punteggio), (7, 8, 56));
}

#[test]
fn cq_wpx() {
    let log = [
        q("IK2XYZ", "40m", "CW", 0, "001"), // stesso paese 1 su ogni banda; IK2
        q("DL1ABC", "20m", "CW", 1, "001"), // stesso continente 1; DL1
        q("DL1ABC", "40m", "CW", 2, "002"), // 2 sulle bande basse
        q("W1AW", "80m", "CW", 3, "001"),   // altro continente 6 sulle basse; W1
        q("W1AW", "20m", "CW", 4, "002"),   // 3
    ];
    let t = punteggio("CQ WW WPX CW Contest", &log);
    // punti 1+1+2+6+3 = 13; prefissi IK2, DL1, W1 = 3; 39
    assert_eq!((t.punti, t.molt, t.punteggio), (13, 3, 39));
    let log = [
        q("VE3ABC", "20m", "SSB", 0, "001"), // da un W, Nord America: 2
        q("VE3ABC", "40m", "SSB", 1, "002"), // 4 sulle basse
    ];
    let t = punteggio_da("W9ABC", "CQ WW WPX SSB Contest", &log);
    // punti 6; prefisso VE3; 6
    assert_eq!((t.punti, t.molt, t.punteggio), (6, 1, 6));
}

#[test]
fn cq_m() {
    let log = [
        q("UA9ABC", "20m", "CW", 0, "001"),  // Asia ed Europa sono un solo continente: 2; UA9 su 20 m
        q("DL1ABC", "20m", "CW", 1, "001"),  // 2; DL
        q("W1AW", "20m", "CW", 2, "001"),    // 3; K
        q("JA1ABC", "40m", "CW", 3, "001"),  // 2; JA su 40 m
        q("UA9ABC", "20m", "SSB", 4, "002"), // altro modo: 2, UA9 gia' preso su 20 m
    ];
    let t = punteggio("CQ-M Contest", &log);
    // punti 11; moltiplicatori 4; 44
    assert_eq!((t.punti, t.molt, t.punteggio), (11, 4, 44));
}

#[test]
fn cq_mm_dx() {
    let log = [
        q("PY1ABC", "20m", "CW", 0, "SA"),  // altro continente 3; prefisso PY1 su 20 m, paese PY
        q("PY1ABC", "40m", "CW", 1, "SA"),  // 6 sulle basse; PY1 su 40 m
        q("DL1ABC", "20m", "CW", 2, "EUM"), // socio CWJF 10; paese DL
        q("IK2XYZ", "80m", "CW", 3, "EU"),  // stesso paese 1; paese I
        q("LU1ABC", "15m", "CW", 4, "SAQ"), // QRP 10; LU1 su 15 m, paese LU
        q("DL2ABC", "40m", "CW", 5, "EU"),  // stesso continente 4 sulle basse
    ];
    let t = punteggio("CQ-MM-DX", &log);
    // punti 3+6+10+1+10+4 = 34; prefissi SA 3 + paesi 4 = 7; 238
    assert_eq!((t.punti, t.molt, t.punteggio), (34, 7, 238));
}

#[test]
fn california_qso_party() {
    let log = [
        q("K6ABC", "20m", "CW", 0, "1 SCLA"),  // 3; contea SCLA
        q("K6XYZ", "20m", "SSB", 1, "5 ALAM"), // 3; ALAM
        q("K6ABC", "40m", "CW", 2, "10 SCLA"), // 3; SCLA gia' presa
        q("K6ABC", "20m", "CW", 3, "11 SCLA"), // doppio
    ];
    let t = punteggio("California QSO Party Contest", &log);
    // punti 9; moltiplicatori 2; 18
    assert_eq!((t.doppi, t.punti, t.molt, t.punteggio), (1, 9, 2, 18));
    // fra due stazioni fuori dalla California il QSO non conta
    let fuori = [q("W1AW", "20m", "CW", 0, "12 CT")];
    assert_eq!(punteggio("California QSO Party Contest", &fuori).esclusi, 1);
    // stazione californiana (manda la contea): stati e province, la California con una contea, DX niente
    let ca = |call: &str, min: i64, rcvd: &str| QsoTesto { exch_sent: format!("{} SCLA", min + 1), ..q(call, "20m", "CW", min, rcvd) };
    let log = [ca("W1AW", 0, "5 CT"), ca("VE3ABC", 1, "6 ON"), ca("DL1ABC", 2, "7 DX"), ca("K6ABC", 3, "8 ALAM")];
    let t = punteggio("California QSO Party Contest", &log);
    // punti 12; moltiplicatori CT, ON, CA = 3; 36
    assert_eq!((t.punti, t.molt, t.punteggio), (12, 3, 36));
    // al massimo 58 moltiplicatori: 60 stati finti
    let log: Vec<QsoTesto> = (0..60).map(|i| ca(&format!("W{i}ABC"), i, &format!("{i} X{i:02}"))).collect();
    let t = punteggio("California QSO Party Contest", &log);
    assert_eq!((t.punti, t.molt, t.punteggio), (180, 60, 180 * 58));
}

#[test]
fn darc_ostercontest() {
    let log = [
        q("DL1ABC", "80m", "CW", 0, "A01"),  // 1; prefisso DL1 e DOK A01 in CW su 80 m
        q("DL2ABC", "80m", "CW", 1, "NM"),   // 1; DL2 (NM non e' un DOK)
        q("DL1ABC", "80m", "SSB", 2, "A01"), // 1; DL1 e A01 in SSB
        q("DL1ABC", "40m", "CW", 3, "A01"),  // 1; DL1 e A01 su 40 m
        q("OK1ABC", "80m", "CW", 4, "005"),  // 1; OK1
        q("DL3ABC", "80m", "CW", 5, "A01"),  // 1; DL3, A01 gia' preso
        q("DL4ABC", "20m", "CW", 6, "B02"),  // solo 80 e 40 m: non ammesso
    ];
    let t = punteggio("DARC-Ostercontest", &log);
    // punti 6; moltiplicatori 2+1+2+2+1+1 = 9; 54
    assert_eq!((t.esclusi, t.punti, t.molt, t.punteggio), (1, 6, 9, 54));
}

#[test]
fn epc_ww_dx() {
    let log = [
        q("IK2XYZ", "40m", "BPSK125", 0, "001"), // stesso paese 1; I su 40 m
        q("DL1ABC", "15m", "BPSK125", 1, "001"), // stesso continente su 15 m 3; DL
        q("DL1ABC", "20m", "BPSK125", 2, "002"), // 2; DL su 20 m
        q("W1AW", "80m", "BPSK125", 3, "001"),   // altro continente su 80 m 6; K
        q("W1AW", "10m", "BPSK125", 4, "002"),   // su 10 m 5; K
        q("JA1ABC", "20m", "BPSK125", 5, "001"), // su 20 m 4; JA
    ];
    let t = punteggio("EPC WW DX Contest", &log);
    // punti 21; moltiplicatori 6; 126
    assert_eq!((t.punti, t.molt, t.punteggio), (21, 6, 126));
}

#[test]
fn eu_psk_dx() {
    let log = [
        q("DL1ABC", "20m", "BPSK63", 0, "DEBYMU"), // stesso continente 2; area DEBYMU e DL su 20 m
        q("IK2XYZ", "20m", "BPSK63", 1, "ITLOMI"), // stesso paese 1; ITLOMI e I
        q("W1AW", "20m", "BPSK63", 2, "001"),      // altro continente 3; K
        q("DL2ABC", "40m", "BPSK63", 3, "DEBYMU"), // 2; DEBYMU e DL su 40 m
    ];
    let t = punteggio("EU PSK DX CONTEST", &log);
    // punti 8; moltiplicatori 7; 56
    assert_eq!((t.punti, t.molt, t.punteggio), (8, 7, 56));
    let log = [
        q("DL1ABC", "20m", "BPSK63", 0, "DEBYMU"), // da un W con l'Europa: 5; DEBYMU e DL
        q("VE3ABC", "20m", "BPSK63", 1, "001"),    // stesso continente 2; VE
    ];
    let t = punteggio_da("W9ABC", "EU PSK DX CONTEST", &log);
    assert_eq!((t.punti, t.molt, t.punteggio), (7, 3, 21));
}

#[test]
fn eu_hf() {
    let log = [
        q("DL1ABC", "20m", "CW", 0, "85"),  // 1; anno 85 su 20 m
        q("DL1ABC", "20m", "SSB", 1, "85"), // altro modo: 1, 85 gia' preso su 20 m
        q("IK2XYZ", "40m", "CW", 2, "91"),  // 1; 91 su 40 m
        q("W1AW", "20m", "CW", 3, "70"),    // solo Europa: non vale
        q("OK1ABC", "20m", "CW", 4, "85"),  // 1
    ];
    let t = punteggio("EU-HF Contest", &log);
    // punti 4; moltiplicatori 2; 8
    assert_eq!((t.esclusi, t.punti, t.molt, t.punteggio), (1, 4, 2, 8));
}

#[test]
fn field_day_hf() {
    let log = [
        q("DL1ABC", "20m", "CW", 0, "001"),    // fissa della 1a Regione 2; DL su 20 m
        q("DL1ABC/P", "40m", "CW", 1, "001"),  // portatile 5; DL su 40 m
        q("W1AW", "20m", "CW", 2, "001"),      // fissa fuori dalla 1a Regione 3; K
        q("UA9ABC", "20m", "CW", 3, "001"),    // UA9 e' nella 1a Regione: 2; UA9
        q("JA1ABC", "20m", "CW", 4, "001"),    // 3; JA
        q("DL2ABC/MM", "15m", "CW", 5, "001"), // mobile marittima 5, nessun moltiplicatore
    ];
    let t = punteggio("Fieldday Contest", &log);
    // punti 2+5+3+2+3+5 = 20; moltiplicatori 5; 100
    assert_eq!((t.punti, t.molt, t.punteggio), (20, 5, 100));
}
