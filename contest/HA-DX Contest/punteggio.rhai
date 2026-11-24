// HA-DX Contest (MRASZ, regolamento 2026): moltiplicatori per banda, uguali in CW e in SSB.
// I punti (HA 10, stesso continente 2, altro continente 5, /MM e /AM 2) e le bande sono in punteggio.regole.
// `mult=` di punteggio.regole resta come ripiego se lo script manca o fallisce.

// Le 20 contee ungheresi (appendice del regolamento).
fn contee() { ["BN", "BA", "BE", "BO", "CS", "FE", "GY", "HB", "HE", "SZ", "KO", "NG", "PE", "SO", "SA", "TO", "VA", "VE", "ZA", "BP"] }

// Mobili marittimi e aeronautici (/MM, /AM): non sono moltiplicatori.
fn mobile(nominativo) {
    for p in nominativo.to_upper().split("/") { if p == "MM" || p == "AM" { return true; } }
    false
}

// La contea nello scambio ricevuto: la prima parola dell'elenco delle contee.
fn contea(scambio) {
    for w in scambio.split(" ") {
        let x = w.to_upper();
        if contee().contains(x) { return x; }
    }
    ""
}

// Paese: l'entita' WAE se c'e' (IT9, GM/s, TA1...), altrimenti il paese DXCC. HA non e' un moltiplicatore, ma lo sono le sue contee.
fn moltiplicatori(qso) {
    if mobile(qso.callsign) || qso.dxcc == "" { return []; }
    if qso.dxcc == "HA" {
        let c = contea(qso.exch_rcvd);
        if c == "" { return []; }
        return ["exch:" + c + "@" + qso.band];
    }
    let p = if qso.wae != "" { qso.wae } else { qso.dxcc };
    ["dxcc:" + p + "@" + qso.band]
}
