// EUDX Contest (European Union DX Contest): punti e moltiplicatori dal regolamento della 6a edizione (https://www.eudx-contest.com/rules/).

// Lo stato dell'Unione Europea di un paese DXCC (sigle delle regioni del regolamento, sezione 7), "" fuori dall'UE.
// Oltremare: FR14-FR20 (Guyana, Guadalupa, Martinica, Mayotte, Riunione, Saint-Martin, Polinesia e Saint-Pierre) e NL13 (Aruba,
// Bonaire, Curacao, Sint Maarten, Saba e Sint Eustatius) sono regioni; Monte Athos (SV/a) e Market Reef (OJ0) non sono nominati (da verificare).
fn stato(p) {
    switch p {
        "OE" => "AT", "ON" => "BE", "LZ" => "BG", "9A" => "HR", "5B" => "CY", "OK" => "CZ", "OZ" => "DK", "ES" => "EE",
        "OH" | "OH0" | "OJ0" => "FI",
        "F" | "TK" | "FY" | "FG" | "FM" | "FH" | "FR" | "FS" | "FO" | "FO/a" | "FO/m" | "FP" => "FR",
        "DL" => "DE", "SV" | "SV5" | "SV9" | "SV/a" => "GR", "HA" => "HU", "EI" => "IE", "I" | "IS" => "IT",
        "YL" => "LV", "LY" => "LT", "LX" => "LX", "9H" => "MT",
        "PA" | "P4" | "PJ2" | "PJ4" | "PJ5" | "PJ7" => "NL",
        "SP" => "PL", "CT" | "CU" | "CT3" => "PT", "YO" => "RO", "OM" => "SK", "S5" => "SI",
        "EA" | "EA6" | "EA8" | "EA9" => "ES", "SM" => "SE",
        _ => ""
    }
}

// Quante regioni ha ogni stato (sezione 7): 275 in tutto.
fn regioni(s) {
    switch s {
        "AT" => 9, "BE" => 11, "BG" => 6, "HR" => 5, "CY" => 5, "CZ" => 14, "DK" => 5, "EE" => 5, "FI" => 19, "FR" => 20,
        "DE" => 16, "GR" => 13, "HU" => 7, "IE" => 4, "IT" => 21, "LV" => 6, "LT" => 5, "LX" => 1, "MT" => 5, "NL" => 13,
        "PL" => 16, "PT" => 7, "RO" => 8, "SK" => 8, "SI" => 6, "ES" => 19, "SE" => 21,
        _ => 0
    }
}

fn cont(qso) { if qso.wae_cont != "" { qso.wae_cont } else { qso.cont } }
fn mio_cont(qso) { if qso.my_wae_cont != "" { qso.my_wae_cont } else { qso.my_cont } }

// Stazione UE: proprio stato 2, altro stato UE 10, fuori UE stesso continente 3, altro continente 5.
// Stazione fuori UE: con l'UE 10, proprio paese 2, altro paese stesso continente 3, altro continente 5.
fn punti(qso) {
    let mio = stato(qso.my_dxcc);
    let suo = stato(qso.dxcc);
    let stesso_cont = cont(qso) != "" && cont(qso) == mio_cont(qso);
    if mio != "" {
        if suo == mio { return 2; }
        if suo != "" { return 10; }
    } else {
        if suo != "" { return 10; }
        if qso.dxcc != "" && qso.dxcc == qso.my_dxcc { return 2; }
    }
    if stesso_cont { 3 } else { 5 }
}

// La regione mandata: due lettere di uno stato UE e due cifre fra 01 e il numero delle sue regioni (IT03, DE16).
fn regione(qso) {
    let s = qso.exch_rcvd;
    s.make_upper();
    for w in s.split(" ") {
        if w.len() != 4 { continue; }
        let n = w.sub_string(2, 2);
        let cifre = true;
        for c in n.chars() { if c < '0' || c > '9' { cifre = false; } }
        if !cifre { continue; }
        let k = parse_int(n);
        if k >= 1 && k <= regioni(w.sub_string(0, 2)) { return w; }
    }
    ""
}

// Per banda (non per modo): ogni regione UE e ogni paese della lista DXCC + WAE (IT9, GM/s...) + IG9/IH9.
fn moltiplicatori(qso) {
    let b = qso.band;
    let m = [];
    let r = regione(qso);
    if r != "" { m.push("reg:" + r + "@" + b); }
    let p = if qso.wae != "" { qso.wae } else { qso.dxcc };
    if p != "" { m.push("dxcc:" + p + "@" + b); }
    m
}
