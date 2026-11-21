// REF Contest CW (Championnat de France HF): regolamento del 13/5/2026, punti 4, 6 e 7.
// Francesi = Francia, Corsica e dipartimenti o territori d'oltremare: tutti i paesi del database con il prefisso che comincia per F
// (FG, FM, FR, FO, FK, FT/x...) piu' TK. I /MM sono stazioni a parte (numero progressivo, 3 punti), qualunque sia il nominativo.

fn francese(p) { p == "TK" || (p.len() > 0 && p.sub_string(0, 1) == "F") }
fn metropoli(p) { p == "F" || p == "TK" }
fn mm(qso) { qso.callsign.ends_with("/MM") }

// Gli stranieri lavorano solo i francesi (punto 4b); i /MM valgono 3 punti anche per loro (punto 6b, da verificare).
// La funzione sostituisce anche `bands=`: le bande si ricontrollano qui.
fn ammesso(qso) {
    if !["80", "40", "20", "15", "10"].contains(qso.band) { return [false, "band"]; }
    if francese(qso.my_dxcc) || mm(qso) || francese(qso.dxcc) { true } else { [false, "solo QSO con stazioni francesi"] }
}

// Francese: francese stesso continente 6, altro continente 15, straniero stesso continente 1, altro 2, /MM 3.
// Straniero: francese stesso continente 1, altro continente 3, /MM 3.
fn punti(qso) {
    if mm(qso) { return 3; }
    let stesso = qso.cont != "" && qso.cont == qso.my_cont;
    if francese(qso.my_dxcc) {
        if francese(qso.dxcc) { if stesso { 6 } else { 15 } } else { if stesso { 1 } else { 2 } }
    } else {
        if stesso { 1 } else { 3 }
    }
}

// Il dipartimento della metropoli: prima parola dello scambio fra 01-95 (non 20: la Corsica e' 2A e 2B), 2A, 2B e 00 (F6REF).
fn dipartimento(qso) {
    let s = qso.exch_rcvd;
    s.make_upper();
    for w in s.split(" ") {
        if w == "2A" || w == "2B" || w == "00" { return w; }
        if w.len() == 2 {
            let cifre = true;
            for c in w.chars() { if c < '0' || c > '9' { cifre = false; } }
            if cifre {
                let n = parse_int(w);
                if n >= 1 && n <= 95 && n != 20 { return w; }
            }
        }
    }
    ""
}

// Per banda: dipartimenti, 00 e prefissi d'oltremare per tutti; i francesi anche le entita' DXCC (non F, TK e oltremare).
fn moltiplicatori(qso) {
    let b = qso.band;
    if mm(qso) { return []; }
    if francese(qso.dxcc) {
        if metropoli(qso.dxcc) {
            let d = dipartimento(qso);
            if d == "" { return []; }
            return ["dip:" + d + "@" + b];
        }
        return ["dip:" + qso.dxcc + "@" + b];
    }
    if francese(qso.my_dxcc) && qso.dxcc != "" { return ["dxcc:" + qso.dxcc + "@" + b]; }
    []
}
