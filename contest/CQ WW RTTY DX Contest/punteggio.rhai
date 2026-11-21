// CQ WW RTTY DX: punti e moltiplicatori (https://www.cqwwrtty.com/rules.htm, «QSO Points» e «Multipliers»).
// Il paese e' quello della lista DXCC + WAE (+ IG9/IH9): per i nominativi di un'entita' WAE (IT9, GM/s, TA1...) vale quella.

fn paese(qso) { if qso.wae != "" { qso.wae } else { qso.dxcc } }
fn mio_paese(qso) { if qso.my_wae != "" { qso.my_wae } else { qso.my_dxcc } }

// Stesso paese 1, stesso continente 2, altro continente 3 (nessuna eccezione per il Nord America).
fn punti(qso) {
    let p = paese(qso);
    if p != "" && p == mio_paese(qso) { return 1; }
    if qso.cont != "" && qso.cont == qso.my_cont { return 2; }
    3
}

// La zona e' quella mandata dal corrispondente (primo numero da 1 a 40 dello scambio); se manca, quella del database dei paesi.
fn zona(qso) {
    for w in qso.exch_rcvd.split(" ") {
        if w.len() == 0 || w.len() > 2 { continue; }
        let cifre = true;
        for c in w.chars() { if c < '0' || c > '9' { cifre = false; } }
        if cifre {
            let n = parse_int(w);
            if n >= 1 && n <= 40 { return n; }
        }
    }
    qso.cq
}

// Lo stato o la provincia dei W/VE (48 stati + DC, 14 aree canadesi); "" se lo scambio non ne ha uno dell'elenco.
fn qth(qso) {
    let elenco = ["AL", "AZ", "AR", "CA", "CO", "CT", "DE", "DC", "FL", "GA", "ID", "IL", "IN", "IA", "KS", "KY", "LA", "ME", "MD", "MA", "MI", "MN", "MS", "MO", "MT", "NE", "NV", "NH", "NJ", "NM", "NY", "NC", "ND", "OH", "OK", "OR", "PA", "RI", "SC", "SD", "TN", "TX", "UT", "VT", "VA", "WA", "WV", "WI", "WY", "NB", "NS", "QC", "ON", "MB", "SK", "AB", "BC", "NWT", "NF", "LB", "NU", "YT", "PEI"];
    for w in qso.exch_rcvd.to_upper().split(" ") {
        let v = w;
        if v == "NT" { v = "NWT"; }
        if v == "PE" { v = "PEI"; }
        if v == "NL" { v = "NF"; }
        if v in elenco { return v; }
    }
    ""
}

// Per banda: zona CQ, paese, e per K e VE lo stato o la provincia. I /MM contano solo per la zona.
fn moltiplicatori(qso) {
    let b = qso.band;
    let m = [];
    let z = zona(qso);
    if z >= 1 && z <= 40 { m.push("cq:" + z + "@" + b); }
    let p = paese(qso);
    if !qso.callsign.ends_with("/MM") && p != "" { m.push("dxcc:" + p + "@" + b); }
    if qso.dxcc == "K" || qso.dxcc == "VE" {
        let s = qth(qso);
        if s != "" { m.push("dom:" + s + "@" + b); }
    }
    m
}
