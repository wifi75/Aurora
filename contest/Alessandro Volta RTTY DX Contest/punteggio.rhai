// Alessandro Volta RTTY DX Contest (https://www.contestvolta.it/rules.pdf, aggiornato a novembre 2025).

// Ammessi: bande 80-10 m; QSO fra stazioni dello stesso paese DXCC non validi, tranne per JA, K, VE, VK e ZL dove conta l'area:
// la stessa area e' vietata ma il motore non conosce l'area della stazione propria, quindi non la controlla.
fn ammesso(qso) {
    let bande = ["80", "40", "20", "15", "10"];
    if !(qso.band in bande) { return [false, "banda"]; }
    if qso.dxcc == qso.my_dxcc && !(qso.dxcc in ["JA", "K", "VE", "VK", "ZL"]) { return [false, "stesso paese"]; }
    true
}

// Un moltiplicatore in piu' per ogni paese di un altro continente lavorato su almeno 4 bande;
// punteggio = QSO validi x punti x moltiplicatori
fn punteggio(t) {
    let extra = 0;
    for k in t.paesi.keys() {
        let p = t.paesi[k];
        if p.cont != t.my_cont && p.bande >= 4 { extra += 1; }
    }
    t.punti * (t.molt + extra) * t.qso
}
