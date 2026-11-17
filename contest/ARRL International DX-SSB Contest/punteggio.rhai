// Chi e' ammesso (regole 2.1 e 2.3): le sei bande da 160 a 10 m, e solo W/VE con DX o DX con W/VE.
// Lo script sostituisce bands= e valid= delle regole, quindi controlla anche la banda.
// K e VE sono le entita' del database dei paesi per Stati Uniti e Canada; KH6 (Hawaii), KL (Alaska), CY9 e CY0 non sono in elenco: sono DX.
fn ammesso(qso) {
    let bande = ["160", "80", "40", "20", "15", "10"];
    if !(qso.band in bande) { return [false, "band"]; }
    let wve = ["K", "VE"];
    if (qso.my_dxcc in wve) != (qso.dxcc in wve) { true } else { [false, "valid"] }
}
