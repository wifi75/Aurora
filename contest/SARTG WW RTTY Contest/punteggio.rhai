// SARTG WW RTTY Contest: tre periodi (https://www.sartg.com/contest/wwrules.htm):
// sabato 00:00-08:00, sabato 16:00-24:00 e domenica 08:00-16:00 UTC. Fuori dai periodi il QSO non conta.
fn ammesso(qso) {
    let bande = ["80", "40", "20", "15", "10"];
    if !(qso.band in bande) { return [false, "banda"]; }
    let giorni = qso.time / 86400000;
    let ora = (qso.time / 3600000) % 24;
    // il giorno 0 dell'epoca (1 gennaio 1970) era un giovedi: sabato = 2, domenica = 3 (modulo 7)
    let g = giorni % 7;
    if g == 2 && (ora < 8 || ora >= 16) { return true; }
    if g == 3 && ora >= 8 && ora < 16 { return true; }
    [false, "periodo"]
}
