// RSGB IOTA Contest: stazioni Island (riferimento IOTA nello scambio, EU115 o EU-115) e World.
// Il riferimento e' una parola dello scambio fatta da due lettere del continente (EU, AF, AN, AS, NA, OC, SA) e tre cifre.
fn cifre(s) { if s == "" { return false; } for ch in s { if ch < '0' || ch > '9' { return false; } } true }
fn iota(s) {
    for w in s.split(" ") {
        let x = w.to_upper();
        x.replace("-", "");
        if x.len() == 5 && ["EU", "AF", "AN", "AS", "NA", "OC", "SA"].contains(x.sub_string(0, 2)) && cifre(x.sub_string(2)) { return x; }
    }
    ""
}
// Punti: da un'isola World 5, isola con lo stesso riferimento 5, altra isola 15; da World: World 2, isola 15.
fn punti(qso) {
    let mia = iota(qso.exch_sent);
    let sua = iota(qso.exch_rcvd);
    if mia != "" { if sua == "" || sua == mia { 5 } else { 15 } } else if sua == "" { 2 } else { 15 }
}
// Moltiplicatori: i riferimenti IOTA lavorati su ogni banda, in CW e in SSB a parte.
fn moltiplicatori(qso) { let r = iota(qso.exch_rcvd); if r == "" { [] } else { ["iota:" + r + "/" + qso.mode + "@" + qso.band] } }
