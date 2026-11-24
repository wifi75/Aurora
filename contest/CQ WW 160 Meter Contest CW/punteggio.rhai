// CQ WW 160-Meter Contest (CW e SSB, regolamento 2026; https://www.cq160.com/rules.htm, sezione «Multipliers»): una banda sola, ogni moltiplicatore una volta.
// Punti, banda e doppi sono in punteggio.regole; lo script sostituisce solo i moltiplicatori.
// `mult=` di punteggio.regole resta come ripiego se lo script manca o fallisce.

// Stati USA (48 contigui + DC): i K li danno nello scambio ricevuto. Alaska e Hawaii sono paesi (KL, KH6), non stati.
fn stati() { " AL AZ AR CA CO CT DC DE FL GA ID IL IN IA KS KY LA ME MD MA MI MN MS MO MT NE NV NH NJ NM NY NC ND OH OK OR PA RI SC SD TN TX UT VT VA WA WV WI WY " }

// Le 14 province canadesi (VO1 = NF e VO2 = LB separate; PEI = PE, VE8 = NT, VY1 = YT, VY0 = NU).
fn province() { " AB BC LB MB NB NF NS NT NU ON PE QC SK YT " }

// Mobili marittimi (/MM): 5 punti e nessun moltiplicatore.
fn marittimo(nominativo) {
    for p in nominativo.to_upper().split("/") { if p == "MM" { return true; } }
    false
}

// Lo stato o la provincia: la prima parola dello scambio ricevuto dell'elenco giusto per il paese del corrispondente (NWT e' la vecchia sigla di NT).
fn area(qso) {
    let elenco = if qso.dxcc == "K" { stati() } else { province() };
    for w in qso.exch_rcvd.split(" ") {
        let x = w;
        x.make_upper();
        if x == "NWT" { x = "NT"; }
        if x.len() == 2 && elenco.contains(" " + x + " ") { return x; }
    }
    ""
}

// K e VE valgono per stato e provincia, non come paesi. Gli altri: l'entita' WAE se c'e' (IT9, GM/s, 4U1V...), altrimenti il paese DXCC.
fn moltiplicatori(qso) {
    if marittimo(qso.callsign) || qso.dxcc == "" { return []; }
    let b = qso.band;
    if qso.dxcc == "K" || qso.dxcc == "VE" {
        let a = area(qso);
        if a == "" { return []; }
        return ["exch:" + a + "@" + b];
    }
    let p = if qso.wae != "" { qso.wae } else { qso.dxcc };
    ["dxcc:" + p + "@" + b]
}
