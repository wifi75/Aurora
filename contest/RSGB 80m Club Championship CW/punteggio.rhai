// RSGB 80m Club Championship CW: quali QSO sono ammessi (regolamento RSGB 3.5MHz Club Championship 2026, punto 6; https://www.rsgbcc.org/hf/rules/2026/r80mcc.shtml).
// Almeno una delle due stazioni deve stare nel Regno Unito o nelle dipendenze della Corona (UK&CD: G, GM, GW, GI, GD, GJ, GU). Sostituisce bands e valid.
fn ammesso(qso) {
    if qso.band != "80" { return [false, "band"]; }
    let uk = ["G", "GM", "GW", "GI", "GD", "GU", "GJ"];
    if uk.contains(qso.my_dxcc) || uk.contains(qso.dxcc) { true } else { [false, "valid"] }
}
