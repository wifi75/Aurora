//! Prova sui file veri della stazione: attiva solo con AURORA_CTY_PATH e AURORA_WORKED_ADIF impostati.

use std::time::Instant;

use dxcc::DxccDb;
use engine_jtdx::worked::WorkedLog;

#[test]
fn real_adif_is_read_completely_and_quickly() {
    let (Ok(cty), Ok(adif)) = (std::env::var("AURORA_CTY_PATH"), std::env::var("AURORA_WORKED_ADIF")) else {
        eprintln!("AURORA_CTY_PATH e AURORA_WORKED_ADIF non impostati: prova sui file reali saltata");
        return;
    };
    let db = DxccDb::from_file(std::path::Path::new(&cty)).expect("cty.dat valido");
    let bytes = std::fs::read(&adif).expect("file ADIF leggibile");
    let text = String::from_utf8_lossy(&bytes);
    let records = text.matches("<EOR>").count() + text.matches("<eor>").count();

    let started = Instant::now();
    let log = WorkedLog::from_adif(&text, &db);
    let elapsed = started.elapsed();

    assert!(records > 0);
    // lo stesso QSO registrato due volte nello stesso minuto (JTDX lo fa) conta una volta sola: al piu' un record su cinquanta
    assert!(log.len() <= records && records - log.len() <= records / 50, "record {records}, QSO letti {}", log.len());
    assert!(elapsed.as_millis() < 2000, "lettura troppo lenta: {elapsed:?}");
    eprintln!("QSO letti: {} su {records} record in {elapsed:?}", log.len());
    let started = Instant::now();
    let matrix = log.matrix(&db);
    eprintln!(
        "scheda DXCC in {:?}: {} paesi lavorati su {}; per riga {:?}",
        started.elapsed(),
        matrix.worked,
        matrix.total,
        matrix.row_worked
    );
    assert_eq!(matrix.worked, log.stats(&db).dxcc_worked, "la scheda e le statistiche contano gli stessi paesi");
}
