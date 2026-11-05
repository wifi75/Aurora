//! I messaggi del core che l'interfaccia traduce (`app/src/lib/coreText.ts`, elenco `EXACT`) devono esistere ancora identici nel
//! codice Rust: se un testo cambia qui, la traduzione nelle altre lingue va rifatta, altrimenti torna a comparire in italiano.

use std::path::{Path, PathBuf};

fn rust_sources(dir: &Path, out: &mut String) {
    let Ok(entries) = std::fs::read_dir(dir) else { return };
    for e in entries.flatten() {
        let path = e.path();
        let name = e.file_name().to_string_lossy().to_string();
        if path.is_dir() {
            if name != "target" && name != "tests" {
                rust_sources(&path, out);
            }
        } else if name.ends_with(".rs")
            && let Ok(text) = std::fs::read_to_string(&path)
        {
            out.push_str(&text);
        }
    }
}

#[test]
fn every_translated_core_message_still_exists_in_the_core() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../..");
    let ts = std::fs::read_to_string(root.join("app/src/lib/coreText.ts")).expect("coreText.ts");
    let start = ts.find("export const EXACT").expect("EXACT");
    let end = start + ts[start..].find("\n];").expect("fine di EXACT");
    let mut core = String::new();
    rust_sources(&root.join("core/crates"), &mut core);
    let mut checked = 0;
    for line in ts[start..end].lines() {
        let line = line.trim();
        // ogni voce comincia con la stringa italiana fra virgolette doppie o semplici
        let Some(q) = line.chars().find(|c| *c == '"' || *c == '\'') else { continue };
        let Some(rest) = line.strip_prefix('[').unwrap_or(line).trim_start().strip_prefix(q) else { continue };
        let Some(close) = rest.find(q) else { continue };
        let text = &rest[..close];
        if text.starts_with("core.") {
            continue;
        }
        assert!(core.contains(text), "il core non scrive piu' «{text}»: aggiorna app/src/lib/coreText.ts e le traduzioni");
        checked += 1;
    }
    assert!(checked >= 20, "lette solo {checked} voci da EXACT");
}
