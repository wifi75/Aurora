// UBA DX Contest: bonus delle stazioni fuori dal Belgio (regolamento, «Bonus»).
// Bonus = punti dei QSO con il Belgio x la loro percentuale sui QSO validi (per difetto), sommato ai punti prima dei moltiplicatori.
// Esempio del regolamento: 50 QSO ON (500 punti) su 320 QSO = 15,6% -> 78 punti in piu'.
fn punteggio(t) {
    if t.my_dxcc == "ON" || t.qso == 0 || !("ON" in t.paesi) { return t.punti * t.molt; }
    let on = t.paesi["ON"];
    (t.punti + on.punti * on.qso / t.qso) * t.molt
}
