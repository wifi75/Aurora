// IPA Contest (IPARC, regolamento 2025-2026): punti e moltiplicatori.
// Una stazione IPARC e' quella che manda IPA nello scambio (dopo il numero; i soci degli USA aggiungono lo stato: IPA CT).
// Punti: 1 per QSO, 5 per ogni stazione IPARC.
// Moltiplicatori per banda: ogni paese DXCC con una stazione IPARC e ogni stato USA con una stazione IPARC (per gli USA vale lo stato,
// non il paese). Il punteggio e' la somma, banda per banda, dei punti per i moltiplicatori della banda (`band_score=1` in punteggio.regole).
fn parole(s) { let v = []; for w in s.split(" ") { if w != "" { v.push(w.to_upper()); } } v }
fn cifre(s) { if s == "" { return false; } for ch in s { if ch < '0' || ch > '9' { return false; } } true }
fn punti(qso) { if parole(qso.exch_rcvd).contains("IPA") { 5 } else { 1 } }
fn moltiplicatori(qso) {
    let p = parole(qso.exch_rcvd);
    if !p.contains("IPA") { return []; }
    if qso.dxcc == "" { return []; }
    if qso.dxcc != "K" { return ["dxcc:" + qso.dxcc + "@" + qso.band]; }
    let k = [];
    for w in p { if w != "IPA" && !cifre(w) { k.push("state:" + w + "@" + qso.band); } }
    k
}
