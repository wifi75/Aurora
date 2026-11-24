// UBA Spring 80M CW Contest: quali QSO sono ammessi (regolamento UBA Spring Contest 2026, punto 7; https://www.uba.be/en/hf/contest-rules/spring-contest-dst).
// Le stazioni estere valgono solo con i belgi (ON); un belga vale con tutti. Sostituisce bands e valid di punteggio.regole.
fn ammesso(qso) {
    if qso.band != "80" { return [false, "band"]; }
    if qso.my_dxcc == "ON" || qso.dxcc == "ON" { true } else { [false, "valid"] }
}
