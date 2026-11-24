// All Asian DX Contest: chi e' ammesso e punti (regolamento, «Contacts» e «QSO Points»).
// Gli asiatici lavorano tutti tranne il proprio paese DXCC (stesso paese: ne' punti ne' moltiplicatore); gli altri solo l'Asia.
// Lo script sostituisce bands= e valid= delle regole, quindi controlla anche la banda.
// Il continente e' quello dell'entita' WAE quando c'e': la Turchia europea (TA1) e' un'entita' WAE in Europa dentro il paese DXCC Turchia (asiatica),
// e per questo contest vale come stazione non asiatica (regolamento: «Turkey, only Asian area»).
fn cont(qso) { if qso.wae_cont != "" { qso.wae_cont } else { qso.cont } }
fn mio_cont(qso) { if qso.my_wae_cont != "" { qso.my_wae_cont } else { qso.my_cont } }

fn ammesso(qso) {
    let bande = ["160", "80", "40", "20", "15", "10"];
    if !(qso.band in bande) { return [false, "band"]; }
    if mio_cont(qso) == "AS" {
        if qso.dxcc != "" && qso.dxcc == qso.my_dxcc { [false, "valid"] } else { true }
    } else if cont(qso) == "AS" { true } else { [false, "valid"] }
}

// Punti: 160 m 3, 80 e 10 m 2, altre bande 1; un asiatico con un non asiatico il triplo (9, 6, 3).
fn punti(qso) {
    let b = qso.band;
    let base = if b == "160" { 3 } else if b == "80" || b == "10" { 2 } else { 1 };
    if mio_cont(qso) == "AS" && cont(qso) != "AS" { base * 3 } else { base }
}
