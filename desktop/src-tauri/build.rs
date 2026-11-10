use std::path::Path;

/// Con `AURORA_REQUIRE_HAMLIB=1` (la impostano gli script e i flussi di rilascio) la compilazione si ferma se in `hamlib-bundle` non c'e'
/// `rigctld`: un installatore senza Hamlib non deve uscire. Nelle compilazioni di sviluppo la cartella puo' essere vuota.
fn check_hamlib_bundle() {
    println!("cargo:rerun-if-env-changed=AURORA_REQUIRE_HAMLIB");
    println!("cargo:rerun-if-changed=hamlib-bundle");
    if std::env::var("AURORA_REQUIRE_HAMLIB").map_or(true, |v| v.trim().is_empty() || v == "0") {
        return;
    }
    let has_rigctld = std::fs::read_dir(Path::new("hamlib-bundle"))
        .map(|d| d.filter_map(Result::ok).any(|e| e.file_name().to_string_lossy().to_ascii_lowercase().starts_with("rigctld")))
        .unwrap_or(false);
    assert!(has_rigctld, "desktop/src-tauri/hamlib-bundle non contiene rigctld: prepara Hamlib prima di fare l'installatore (scripts/prepara-hamlib.ps1 su Windows, i passi Hamlib dei flussi macOS e Linux)");
}

fn main() {
    check_hamlib_bundle();
    tauri_build::build()
}
