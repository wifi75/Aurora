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
    const T: [(&str, &str, &str, u8, u8); 27] = [
        ("SY2", "SV/A", "EU", 20, 28),
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
        n("LA/G3XYZ", "20m", "CW", 4, "001"), // 1; senza cifra del prefisso: area LA0
        n("SM150A", "20m", "CW", 5, "001"),   // 1; due cifre: conta la prima, SM1
    ];
    let t = punteggio("Scandinavian Activity Contest", &log, None);
    // punti 5; moltiplicatori SM3, LA1, LA0, SM1 = 4; 5 x 4 = 20
    assert_eq!((t.esclusi, t.punti, t.molt, t.punteggio), (1, 5, 4, 20));
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
    // potenza: stazione LP x2, QRP x4; la propria LP x1,5
    let s = |call: &str, min: i64, g: &str| q(call, "160m", "CW", min, "JN65 LP", g);
    let log = [
        s("IK2XYZ", 0, "JN55"),     // 1
        s("G4ABC", 1, "IO91 LP"),   // 3 x 2 = 6
        s("W1AW", 2, "FN42 QRP"),   // 13 x 4 = 52
    ];
    let t = punteggio("Stew Perry Topband Distance Challenge", &log, None);
    // 59 x 1,5 = 88,5 -> 89
    assert_eq!((t.punti, t.punteggio), (59, 89));
    let qrp = [q("W1AW", "160m", "CW", 0, "JN65 QRP", "FN42")];
    assert_eq!(punteggio("Stew Perry Topband Distance Challenge", &qrp, None).punteggio, 39);
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
        n("R2FA", "40m", "CW", 240, "KA03"),  // 16:00, secondo turno: non e' un doppio, 6; KA03 gia' preso su 40 m
        n("R2FA", "40m", "CW", 241, "KA03"),  // doppio nel secondo turno
    ];
    let t = punteggio("UA2 QSO Party", &log, None);
    // punti 6+3+2+3+6 = 20; due distretti nuovi x 15 = 30; 20 + 30 = 50
    assert_eq!((t.doppi, t.punti, t.molt, t.punteggio), (1, 20, 2, 50));
}

#[test]
fn uba_dx() {
    let log = [
        n("ON4ABC", "20m", "CW", 0, "001 ANT"), // Belgio: 10; sezione ANT e prefisso ON4 su 20 m
        n("DL1ABC", "20m", "CW", 1, "001"),     // paese UE: 3; DL
        n("W1AW", "20m", "CW", 2, "001"),       // 1
        n("UA3ABC", "20m", "CW", 3, "001"),     // Russia: non ammesso
        n("OT4XYZ", "40m", "CW", 4, "005 XXX"), // Belgio: 10; OT4 su 40 m (XXX non e' un moltiplicatore)
        n("SY2ABC", "20m", "CW", 5, "001"),     // Monte Athos, paese UE: 3; SV/A
    ];
    let t = punteggio("UBA CW contest", &log, None);
    // punti 27; moltiplicatori ANT, ON4, DL, OT4, SV/A = 5; bonus: 20 punti ON x 2 QSO ON / 5 QSO = 8; (27 + 8) x 5 = 175
    assert_eq!((t.esclusi, t.punti, t.molt, t.punteggio), (1, 27, 5, 175));
    let ssb = [n("ON4ABC", "80m", "SSB", 0, "001 ANT"), n("DL1ABC", "80m", "SSB", 1, "001")];
    // 10 + 3 = 13; bonus 10 x 1 / 2 = 5; ANT, ON4, DL = 3; 18 x 3 = 54
    assert_eq!(punteggio("UBA SSB Contest", &ssb, None).punteggio, 54);
    // stazione belga: ON 1, UE 2, altri 3; paesi DXCC (anche ON) e sezioni per banda, niente prefissi e niente bonus
    let on = paese("ON4XYZ").unwrap_or_default();
    let log = [n("ON4ABC", "20m", "CW", 0, "001 ANT"), n("DL1ABC", "20m", "CW", 1, "001"), n("W1AW", "20m", "CW", 2, "001")];
    let (_, t) = valuta(&contest("UBA CW contest"), &log, Some(&on), &paese, None);
    // punti 6; moltiplicatori ANT, ON, DL, K = 4; 24
    assert_eq!((t.punti, t.molt, t.punteggio, t.errore_script.clone()), (6, 4, 24, None));
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
    // stazione UK: il doppio dalle 01:00 alle 04:59 UTC
    let g = paese("G4XYZ").unwrap_or_default();
    let log = [
        n("DL1ABC", "80m", "CW", 780, "001"), // 01:00, Europa su 80 m: 4 x 2 = 8; DL su 80 m
        n("W1AW", "20m", "CW", 810, "001"),   // 01:30, DX su 20 m: 4 x 2 = 8; K
        n("DL1ABC", "20m", "CW", 1020, "002"), // 05:00, Europa su 20 m: 2; DL su 20 m
    ];
    let (_, t) = valuta(&contest("UK_EI DX Contest"), &log, Some(&g), &paese, None);
    // punti 18; moltiplicatori 3; 54
    assert_eq!((t.punti, t.molt, t.punteggio), (18, 3, 54));
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

/// Il terzo elemento dello scambio (sezione, club, stato) lo chiedono le regole `exch` e solo per le stazioni che soddisfano il loro `if:`.
fn atteso(nome: &str, call: &str, mia: &str) -> bool {
    use contest::punti_testo::scambio_atteso;
    let q = QsoTesto { call: call.into(), band: "20m".into(), mode: "CW".into(), ..Default::default() };
    scambio_atteso(&contest(nome), &q, call, paese(call).as_ref(), paese(mia).as_ref())
}

#[test]
fn scambio_atteso_uba_arrl_e_senza_exch() {
    // UBA (`exch/if:dxcc:ON`): sezione solo dalle stazioni belghe, i DX danno RST e numero
    assert!(atteso("UBA CW contest", "ON4ABC", "IU3ABC"));
    assert!(atteso("UBA CW contest", "OT4XYZ", "IU3ABC"));
    assert!(!atteso("UBA CW contest", "DL1ABC", "IU3ABC"));
    assert!(!atteso("UBA CW contest", "W1AW", "ON4XYZ"));
    // la propria stazione: un'utente belga manda la sezione, uno italiano solo il numero
    assert!(atteso("UBA CW contest", "ON4XYZ", "ON4XYZ"));
    assert!(!atteso("UBA CW contest", "IU3ABC", "IU3ABC"));
    // ARRL 10 metri (`exch/mode/if:dxcc:K,VE,...`): stato o provincia dagli USA e dal Canada, non dai DX
    assert!(atteso("ARRL 10 meter Contest", "W1AW", "IU3ABC"));
    assert!(!atteso("ARRL 10 meter Contest", "DL1ABC", "IU3ABC"));
    assert!(!atteso("ARRL 10 meter Contest", "IU3ABC", "IU3ABC"));
    // un contest senza moltiplicatore `exch` non lo chiede mai
    let senza = ContestTesto { score: "points=any=1\nmult=dxcc".into(), ..ContestTesto::default() };
    let q = QsoTesto { call: "ON4ABC".into(), ..Default::default() };
    assert!(!contest::punti_testo::scambio_atteso(&senza, &q, "ON4ABC", paese("ON4ABC").as_ref(), None));
    // le vecchie chiavi (`m_iMultipliers=5`, senza [Score]) non cambiano: lo scambio e' un campo solo
    let vecchio = ContestTesto { multipliers: 5, ..ContestTesto::default() };
    assert!(!contest::punti_testo::scambio_atteso(&vecchio, &q, "ON4ABC", paese("ON4ABC").as_ref(), None));
}

#[test]
fn cabrillo_uba_con_club_e_senza() {
    use contest::punti_testo::cabrillo;
    let c = contest("UBA CW contest");
    let ora = |hhmm: &str, freq: u64, call: &str, sent: &str, rcvd: &str| QsoTesto {
        call: call.into(),
        band: "10m".into(),
        mode: "CW".into(),
        freq_hz: freq,
        time_ms: data_ms(&format!("20230225{hhmm}00")).unwrap_or(0),
        rst_sent: "599".into(),
        rst_rcvd: "599".into(),
        exch_sent: sent.into(),
        exch_rcvd: rcvd.into(),
    };
    // lo scambio e' quello che compone l'app: numero e club separati da uno spazio (`003 ACC`); il rapporto lo mette il Cabrillo
    let log = [ora("1302", 28_010_000, "OP4K", "001 ACC", "003 ACC"), ora("1311", 28_023_000, "SV1BJW", "002 ACC", "012")];
    let on = paese("ON9TT").unwrap_or_default();
    let (v, t) = valuta(&c, &log, Some(&on), &paese, None);
    let f = cabrillo(&c, "on9tt", &v, &t, "0.11.0", &[]);
    let righe: Vec<String> = f.lines().filter(|r| r.starts_with("QSO:")).map(|r| r.split_whitespace().collect::<Vec<_>>().join(" ")).collect();
    assert_eq!(righe[0], "QSO: 28010 CW 2023-02-25 1302 ON9TT 599 001 ACC OP4K 599 003 ACC");
    assert_eq!(righe[1], "QSO: 28023 CW 2023-02-25 1311 ON9TT 599 002 ACC SV1BJW 599 012");
}
