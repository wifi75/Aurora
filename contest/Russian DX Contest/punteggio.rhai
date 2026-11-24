// Russian DX Contest: punti e moltiplicatori dal regolamento (punti 7, 8 e 9; https://www.rdxc.org/rules_eng).
// Paesi e continenti sono quelli della lista DXCC (7.6): la Russia europea (UA), Kaliningrad (UA2) e Franz Josef Land (R1FJ) sono in Europa,
// la Russia asiatica (UA9) in Asia; le stazioni russe in Antartide (RI1AN...) sono Russia europea ai fini dei punti (7.3) e una regione a parte.

// Stazione russa: le quattro entita' russe del database dei paesi piu' le stazioni russe in Antartide (prefisso RI1AN).
fn russa(qso) {
    let e = ["UA", "UA9", "UA2", "R1FJ"];
    qso.dxcc in e || qso.callsign.starts_with("RI1AN")
}
fn mia_russa(qso) { qso.my_dxcc in ["UA", "UA9", "UA2", "R1FJ"] }
// Russia asiatica (UA9) contro Russia europea (tutto il resto, comprese Kaliningrad, Franz Josef Land e l'Antartide russa).
fn asiatica(qso) { qso.dxcc == "UA9" }
fn mia_asiatica(qso) { qso.my_dxcc == "UA9" }

// I mobili marittimi valgono 5 punti per tutti e non sono moltiplicatori (7.4).
fn marittimo(qso) { qso.callsign.ends_with("/MM") }

// Russi: Russia del proprio continente 2, Russia dell'altro continente 5, altro paese del proprio continente 3, altro continente 5.
// Gli altri: stazione russa 10, proprio paese 2, altro paese del proprio continente 3, altro continente 5.
fn punti(qso) {
    if marittimo(qso) { return 5; }
    let stesso_cont = qso.cont != "" && qso.cont == qso.my_cont;
    if mia_russa(qso) {
        if russa(qso) {
            if asiatica(qso) == mia_asiatica(qso) { return 2; }
            return 5;
        }
        if stesso_cont { return 3; }
        return 5;
    }
    if russa(qso) { return 10; }
    if qso.dxcc != "" && qso.dxcc == qso.my_dxcc { return 2; }
    if stesso_cont { 3 } else { 5 }
}

// Sigle ufficiali delle regioni (https://www.rdxc.org/info): 89 regioni, Franz Josef Land (FJ) e Antartide (AN).
fn sigle() {
    [
        "SP", "LO", "KO", "KL", "AR", "NO", "VO", "NV", "PS", "MU", "KA", "MA", "MO", "OR",
        "LP", "TV", "VR", "SM", "YR", "KS", "TL", "TB", "RA", "NN", "IV", "VL", "KU", "KG",
        "BR", "BO", "VG", "SA", "PE", "SR", "UL", "KI", "TA", "MR", "MD", "UD", "CU", "KR",
        "KC", "ST", "KM", "SO", "RK", "RO", "CN", "IN", "SE", "AO", "DA", "KB", "AD", "DO",
        "HE", "LU", "ZP", "CB", "SV", "PM", "TO", "HM", "YN", "TN", "OM", "NS", "KN", "OB",
        "KE", "BA", "AL", "GA", "KK", "HK", "EA", "SL", "MG", "AM", "CK", "PK", "BU", "YA",
        "IR", "ZK", "HA", "TU", "KT", "FJ", "AN"
    ]
}

// La regione di una stazione russa: l'ultima sigla valida dello scambio ricevuto. Kaliningrad, Franz Josef Land e l'Antartide russa
// sono regioni a parte (7.3): senza scambio valido valgono KA, FJ e AN. Un numero progressivo non e' mai una regione.
fn regione(qso) {
    if !russa(qso) { return ""; }
    let s = qso.exch_rcvd;
    s.make_upper();
    let ok = sigle();
    let r = "";
    for w in s.split(" ") { if w.len() == 2 && w in ok { r = w; } }
    if r != "" { return r; }
    if qso.dxcc == "UA2" { return "KA"; }
    if qso.dxcc == "R1FJ" { return "FJ"; }
    if qso.callsign.starts_with("RI1AN") { return "AN"; }
    ""
}

// Per banda (non per modo): ogni paese della lista DXCC + WAE (IT9, GM/s...) e ogni regione russa. Kaliningrad, Franz Josef Land e
// l'Antartide russa valgono doppio: paese e regione. I /MM non contano.
fn moltiplicatori(qso) {
    if marittimo(qso) { return []; }
    let b = qso.band;
    let m = [];
    let p = if qso.wae != "" { qso.wae } else { qso.dxcc };
    if p != "" { m.push("dxcc:" + p + "@" + b); }
    let r = regione(qso);
    if r != "" { m.push("obl:" + r + "@" + b); }
    m
}
