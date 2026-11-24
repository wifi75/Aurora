// Japan International DX Contest: chi e' ammesso, punti e moltiplicatori (regolamento, sezioni 4, 5 e 6; https://jidx.org/jidxrule-e.html).
// Contano solo i QSO fra un JA e un DX (compresi i /MM per i JA). Le isole JD1 (Ogasawara, Minami-Torishima, Okino-Torishima) sono
// prefetture giapponesi (48, 49, 50): per il contest stanno dalla parte del Giappone. Il database dei paesi le conosce come JD/o e JD/m.
fn giapponese(d) { d == "JA" || d == "JD/m" || d == "JD/o" }
fn marittimo(qso) { qso.callsign.ends_with("/MM") }

fn solo_cifre(w) {
    if w.len() == 0 { return false; }
    for c in w.chars() { if c < '0' || c > '9' { return false; } }
    true
}

// L'ultimo numero di una o due cifre fra 1 e il massimo dello scambio ricevuto (il rapporto di tre cifre o fuori misura non conta).
// Prefettura da 1 a 50 per i JA, zona CQ da 1 a 40 per gli altri. -1 se manca.
fn numero(qso, massimo) {
    let n = -1;
    for w in qso.exch_rcvd.split(" ") {
        if w.len() <= 2 && solo_cifre(w) {
            let v = parse_int(w);
            if v >= 1 && v <= massimo { n = v; }
        }
    }
    n
}

// Ammesso: bande da 160 a 10 m (niente WARC); un JA lavora solo i DX (e i /MM), un DX solo i JA. DX-DX, DX-/MM e JA-JA non valgono.
fn ammesso(qso) {
    let bande = ["160", "80", "40", "20", "15", "10"];
    if !(qso.band in bande) { return [false, "band"]; }
    if giapponese(qso.my_dxcc) {
        if marittimo(qso) { return true; }
        if qso.dxcc != "" && !giapponese(qso.dxcc) { true } else { [false, "valid"] }
    } else {
        if giapponese(qso.dxcc) { true } else { [false, "valid"] }
    }
}

// 1,8 MHz 4 punti; 3,5 e 28 MHz 2 punti; 7, 14 e 21 MHz 1 punto.
fn punti(qso) {
    let b = qso.band;
    if b == "160" { 4 } else if b == "80" || b == "10" { 2 } else { 1 }
}

// Per banda. I JA: zone CQ (quella mandata dal corrispondente, altrimenti quella del database) e paesi DXCC; un /MM vale solo per la zona.
// I DX: la prefettura mandata dal JA (da 01 a 50, JD1 compreso).
fn moltiplicatori(qso) {
    let b = qso.band;
    let m = [];
    if giapponese(qso.my_dxcc) {
        let z = numero(qso, 40);
        if z < 1 { z = qso.cq; }
        if z >= 1 && z <= 40 { m.push("cq:" + z + "@" + b); }
        if !marittimo(qso) && qso.dxcc != "" { m.push("dxcc:" + qso.dxcc + "@" + b); }
    } else {
        let p = numero(qso, 50);
        if p >= 1 {
            let s = if p < 10 { "0" + p } else { "" + p };
            m.push("pref:" + s + "@" + b);
        }
    }
    m
}
