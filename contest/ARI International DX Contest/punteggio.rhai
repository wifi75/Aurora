// ARI International DX Contest (regolamento 2026, punti 2, 7 e 8; https://www.ari.it/images/stories/ContestHF/ariinternationaldxcontest-dxrules2026.pdf).
// Punti e doppi sono in punteggio.regole; lo script dice quali QSO sono ammessi (sostituisce bands e valid) e da' i moltiplicatori.

// Una stazione italiana: Italia, Sardegna o Sicilia (IT9 e' un'entita' WAE dentro il paese DXCC I).
fn italiana(dxcc, wae) { dxcc == "I" || dxcc == "IS" || dxcc == "IT9" || wae == "IT9" }

// Le stazioni italiane non si lavorano fra loro (punto 2); solo le bande 80-10 m senza WARC.
fn ammesso(qso) {
    if !["80", "40", "20", "15", "10"].contains(qso.band) { return [false, "band"]; }
    if italiana(qso.dxcc, qso.wae) && italiana(qso.my_dxcc, qso.my_wae) { [false, "valid"] } else { true }
}

// Le 107 province italiane (punto 7).
fn province() {
    " AL AT BI CN GE IM NO SP SV TO VB VC AO BG BS CO CR LC LO MB MI MN PV SO VA BL PD RO TV VE VI VR BZ TN GO PN TS UD BO FC FE MO PC PR RA RE RN AR FI GR LI LU MS PI PO PT SI AN AP AQ CH FM MC PE PU TE BA BR BT FG LE MT TA AV BN CB CE CS CZ IS KR NA PZ RC SA VV FR LT PG RI RM TR VT AG CL CT EN ME PA RG SR TP CA NU OR SS SU "
}

// La provincia: l'ultima parola di due lettere dello scambio ricevuto (il rapporto e il numero progressivo sono cifre).
fn provincia(qso) {
    let r = "";
    for w in qso.exch_rcvd.split(" ") {
        if w.len() == 2 {
            let s = w;
            s.make_upper();
            if province().contains(" " + s + " ") { r = s; }
        }
    }
    r
}

// Per banda: ogni provincia italiana e ogni paese (DXCC piu' elenco WAE; I, IS e IT9 non contano come paesi).
fn moltiplicatori(qso) {
    let b = qso.band;
    let m = [];
    if italiana(qso.dxcc, qso.wae) {
        let p = provincia(qso);
        if p != "" { m.push("provincia:" + p + "@" + b); }
    } else {
        let p = if qso.wae != "" { qso.wae } else { qso.dxcc };
        if p != "" { m.push("dxcc:" + p + "@" + b); }
    }
    m
}
