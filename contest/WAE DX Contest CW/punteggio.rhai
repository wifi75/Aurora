// WAE DX Contest CW (DARC, «Worked All Europe DX Contest - Rules 2025», date 2026): ammissione e moltiplicatori.
// I punti (1 per QSO) e i pesi delle bande (80 m x4, 40 m x3, 20/15/10 m x2) sono in punteggio.regole (`points`, `mult_band`):
// il motore da' a ogni chiave dello script con la banda il peso di `mult_band`.

// Europeo = continente dell'entita' WAE se c'e' (TA1 in Europa, IG9 in Africa), altrimenti quello del paese DXCC.
fn cont(qso) { if qso.wae_cont != "" { qso.wae_cont } else { qso.cont } }
fn mio_cont(qso) { if qso.my_wae_cont != "" { qso.my_wae_cont } else { qso.my_cont } }

// In CW valgono solo i QSO fra una stazione europea e una non europea (punto 5). La funzione sostituisce anche `bands=`.
fn ammesso(qso) {
    if !["80", "40", "20", "15", "10"].contains(qso.band) { return [false, "band"]; }
    if cont(qso) == "" || mio_cont(qso) == "" { return [false, "continente"]; }
    if (cont(qso) == "EU") != (mio_cont(qso) == "EU") { true } else { [false, "continente"] }
}

// L'area numerica del nominativo: la cifra dopo la barra (K3ABC/1 = 1), altrimenti la prima cifra dopo la prima lettera (W6ABC = 6).
fn area(nominativo) {
    let parti = nominativo.split("/");
    for p in parti {
        if p.len() == 1 && p >= "0" && p <= "9" { return p; }
    }
    let base = parti[0];
    for p in parti { if p.len() > base.len() { base = p; } }
    let i = 0;
    for c in base.chars() {
        if i > 0 && c >= '0' && c <= '9' { return c.to_string(); }
        i += 1;
    }
    "0"
}

// Paese non europeo: le aree numeriche per W, VE, VK, ZL, ZS, JA, BY, PY e Russia asiatica (RA8, RA9, RA0), altrimenti il paese DXCC.
fn paese_dx(qso) {
    let p = qso.dxcc;
    if p == "" { return ""; }
    if p == "K" { return "W" + area(qso.callsign); }
    if p == "UA9" { return "UA" + area(qso.callsign); }
    if ["VE", "VK", "ZL", "ZS", "JA", "BY", "PY"].contains(p) { return p + area(qso.callsign); }
    p
}

// Paese europeo: l'elenco WAE (IT9, GM/s, TA1, JW/b, 4U1V... a parte), altrimenti il paese DXCC.
fn paese_eu(qso) { if qso.wae != "" { qso.wae } else { qso.dxcc } }

// Per banda: gli europei contano i paesi non europei (con le aree), i non europei i paesi dell'elenco WAE (punto 7).
fn moltiplicatori(qso) {
    let b = qso.band;
    let p = if cont(qso) == "EU" { paese_eu(qso) } else { paese_dx(qso) };
    if p == "" { [] } else { ["dxcc:" + p + "@" + b] }
}
