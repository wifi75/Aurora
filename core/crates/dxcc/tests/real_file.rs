//! Prova sul vero cty.dat: attiva solo se AURORA_CTY_PATH indica il file (per esempio quello di JTDX).

use dxcc::DxccDb;

fn real_db() -> Option<DxccDb> {
    let path = std::env::var("AURORA_CTY_PATH").ok()?;
    Some(DxccDb::from_file(std::path::Path::new(&path)).expect("il file indicato deve essere un cty.dat valido"))
}

fn country(db: &DxccDb, call: &str) -> Option<String> {
    db.lookup(call).map(|l| db.entity(l.entity).name.clone())
}

#[test]
fn real_cty_file_knows_common_countries() {
    let Some(db) = real_db() else {
        eprintln!("AURORA_CTY_PATH non impostato: prova sul file reale saltata");
        return;
    };
    assert!(db.len() > 300, "paesi letti: {}", db.len());
    for (call, expected) in [
        ("IU3XYZ", "Italy"),
        ("DL1ABC", "Fed. Rep. of Germany"),
        ("K1ABC", "United States"),
        ("JA1XYZ", "Japan"),
        ("VK3ZZ", "Australia"),
        ("EA8XYZ", "Canary Islands"),
        ("EA1XYZ", "Spain"),
        ("PY2XX", "Brazil"),
        ("ZS6AA", "South Africa"),
        ("DL/IU3XYZ", "Fed. Rep. of Germany"),
        ("IU3XYZ/P", "Italy"),
    ] {
        assert_eq!(country(&db, call).as_deref(), Some(expected), "{call}");
    }
    assert_eq!(country(&db, "<...>"), None);
}
