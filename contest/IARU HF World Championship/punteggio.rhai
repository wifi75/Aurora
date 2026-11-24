// IARU HF World Championship: punti e moltiplicatori dal regolamento 1.21 (sezione 5, «Scoring»).
// Lo scambio ricevuto e' la zona ITU (numero) oppure la sigla di una stazione HQ di societa' IARU o di un ufficiale (AC, R1, R2, R3).

fn cifre(s) {
    if s == "" { return false; }
    for c in s.chars() { if c < '0' || c > '9' { return false; } }
    true
}

// L'ultima parola dello scambio (in maiuscolo).
fn ultima(testo) {
    let u = "";
    for w in testo.split(" ") { if w != "" { u = w.to_upper(); } }
    u
}

// La zona ITU mandata dal corrispondente: numero da 1 a 90; senza scambio (o con un numero fuori misura) quella del database dei paesi;
// una sigla (HQ, ufficiali) vale -1.
fn zona(qso) {
    let w = ultima(qso.exch_rcvd);
    if w == "" { return qso.itu; }
    if cifre(w) {
        let n = parse_int(w);
        if n >= 1 && n <= 90 { return n; }
        return qso.itu;
    }
    -1
}

// La propria zona: il numero scritto nello scambio mandato, altrimenti quella del database dei paesi.
fn mia_zona(qso) {
    let w = ultima(qso.exch_sent);
    if cifre(w) {
        let n = parse_int(w);
        if n >= 1 && n <= 90 { return n; }
    }
    qso.my_itu
}

// Punti: HQ e ufficiali 1; stessa zona ITU 1 (anche se in un altro continente); stesso continente ma altra zona 3; altro continente e altra zona 5.
fn punti(qso) {
    let z = zona(qso);
    if z < 0 { return 1; }
    if z == mia_zona(qso) { return 1; }
    if qso.cont != "" && qso.cont == qso.my_cont { 3 } else { 5 }
}

// Per banda (non per modo): ogni zona ITU, e ogni HQ/ufficiale diverso (le sigle non contano come zona).
fn moltiplicatori(qso) {
    let z = zona(qso);
    if z < 0 { return ["hq:" + ultima(qso.exch_rcvd) + "@" + qso.band]; }
    if z > 0 { return ["itu:" + z + "@" + qso.band]; }
    []
}
