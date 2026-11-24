// OK-OM DX Contest: moltiplicatori per banda (regolamento, sezione 8; https://okomdx.crk.cz/index.php?page=CW-rules-english).
// Punti, bande e stazioni non ammesse (Russia e Bielorussia) sono in punteggio.regole; lo script sostituisce solo i moltiplicatori.
// Due tipi, ognuno una volta per banda: i distretti OK e OM (tre lettere dello scambio ricevuto) e i paesi della lista DXCC piu' i moltiplicatori WAE
// (per i nominativi di un'entita' WAE come IT9 o GM/s vale quella). I /MM non sono moltiplicatori (6.3).
fn marittimo(qso) { qso.callsign.ends_with("/MM") }

// Il distretto: l'ultima parola di tre lettere dello scambio ricevuto (il numero progressivo e il rapporto sono cifre).
fn distretto(qso) {
    let r = "";
    for w in qso.exch_rcvd.split(" ") {
        if w.len() == 3 {
            let s = w;
            s.make_upper();
            let lettere = true;
            for c in s.chars() { if c < 'A' || c > 'Z' { lettere = false; } }
            if lettere { r = s; }
        }
    }
    r
}

fn moltiplicatori(qso) {
    if marittimo(qso) { return []; }
    let b = qso.band;
    let m = [];
    let p = if qso.wae != "" { qso.wae } else { qso.dxcc };
    if p != "" { m.push("dxcc:" + p + "@" + b); }
    if qso.dxcc == "OK" || qso.dxcc == "OM" {
        let d = distretto(qso);
        if d != "" { m.push("distretto:" + d + "@" + b); }
    }
    m
}
