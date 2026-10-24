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
    assert_eq!(log.len(), records, "ogni record completo deve essere letto");
    assert!(elapsed.as_millis() < 2000, "lettura troppo lenta: {elapsed:?}");
    eprintln!("QSO letti: {} in {elapsed:?}", log.len());
}
