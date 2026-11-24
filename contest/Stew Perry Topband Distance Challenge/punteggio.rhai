// Stew Perry Topband Distance Challenge: punti e punteggio dal regolamento (revisione 31 maggio 2025, https://www.kkn.net/stew/stew_rules.html).
// Punti per QSO (regola 5): 1 punto piu' 1 ogni 500 km interi fra i centri dei quadrati mandato e ricevuto (4 caratteri), senza percorso lungo;
// x2 se si lavora una stazione Low Power, x4 una QRP. Punteggio finale (regola 7): somma dei punti, nessun moltiplicatore; x1,5 se si e' Low Power
// (piu' di 5 W e fino a 100 W), x3 se QRP. Il programma dei log controllati applica la potenza degli altri dai loro Cabrillo: qui la potenza della
// stazione lavorata si legge dalla parola LP o QRP scritta nello scambio ricevuto, la propria da quella scritta nello scambio mandato.
fn potenza(s) {
    for p in s.to_upper().split(" ") {
        if p == "QRP" { return 2; }
        if p == "LP" { return 1; }
    }
    0
}
fn centro(g) {
    let g = g.to_upper();
    if g.len() != 4 { return (); }
    let a = "ABCDEFGHIJKLMNOPQR";
    let d = "0123456789";
    let lo = a.index_of(g[0]);
    let la = a.index_of(g[1]);
    let lo2 = d.index_of(g[2]);
    let la2 = d.index_of(g[3]);
    if lo < 0 || la < 0 || lo2 < 0 || la2 < 0 { return (); }
    [(la * 10 + la2 - 90).to_float() + 0.5, (lo * 20 + lo2 * 2 - 180).to_float() + 1.0]
}
fn quadrato(s) {
    for p in s.split(" ") {
        let c = centro(p);
        if c != () { return c; }
    }
    ()
}
fn punti(qso) {
    let a = quadrato(qso.exch_sent);
    let b = quadrato(qso.exch_rcvd);
    let f = [1, 2, 4][potenza(qso.exch_rcvd)];
    if a == () || b == () { return f; }
    let r = PI() / 180.0;
    let c = sin(a[0] * r) * sin(b[0] * r) + cos(a[0] * r) * cos(b[0] * r) * cos((a[1] - b[1]) * r);
    if c > 1.0 { c = 1.0; }
    if c < -1.0 { c = -1.0; }
    let km = acos(c) * 6371.0;
    (1 + (km / 500.0).floor().to_int()) * f
}
fn punteggio(t) {
    let m = potenza(t.exch_sent);
    if m == 2 { t.punti * 3 } else if m == 1 { (t.punti * 3 + 1) / 2 } else { t.punti }
}
