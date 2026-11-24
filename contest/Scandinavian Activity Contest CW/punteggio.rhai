// Scandinavian Activity Contest: valgono solo i QSO fra una stazione scandinava e una che non lo e'.
fn ammesso(qso) {
    let s = ["SM", "LA", "OH", "OZ", "TF", "OY", "OX", "JW", "JX", "OH0", "OJ0"];
    if s.contains(qso.dxcc) != s.contains(qso.my_dxcc) { true } else { [false, "scandinavia"] }
}
