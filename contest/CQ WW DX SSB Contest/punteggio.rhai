// CQ WW DX: punti e moltiplicatori (regolamento, «QSO Points» e «Multipliers»).
// Il paese e' quello della lista DXCC + WAE (+ IG9/IH9): per i nominativi di un'entita' WAE (IT9, GM/s, TA1, 4U1V, JW/b, IG9) vale quella.

fn paese(qso) { if qso.wae != "" { qso.wae } else { qso.dxcc } }
fn mio_paese(qso) { if qso.my_wae != "" { qso.my_wae } else { qso.my_dxcc } }

// Stesso paese 0 (vale per i moltiplicatori), paesi diversi del Nord America 2, stesso continente 1, altro continente 3.
fn punti(qso) {
    let p = paese(qso);
    if p != "" && p == mio_paese(qso) { return 0; }
    if qso.cont == "" || qso.cont != qso.my_cont { return 3; }
    if qso.cont == "NA" { return 2; }
    1
}

// La zona e' quella mandata dal corrispondente (primo numero da 1 a 40 dello scambio ricevuto); se manca, quella del database dei paesi.
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

// Una zona CQ e un paese per banda; i mobili marittimi (/MM, senza paese nel database) contano solo per la zona.
fn moltiplicatori(qso) {
    let b = qso.band;
    let m = [];
    let z = zona(qso);
    if z >= 1 && z <= 40 { m.push("cq:" + z + "@" + b); }
    let p = paese(qso);
    if !qso.callsign.ends_with("/MM") && p != "" { m.push("dxcc:" + p + "@" + b); }
    m
}
