//! Converte una cartella di file di contest (`Contest-Settings/*.cst`, `Contest-Macros/*.mc`, `Contest-DOM/*.dom`) nei contest predefiniti
//! di Aurora (`*.contest`, `macros/`, `dom/`), togliendo nominativi personali, percorsi assoluti e moduli esterni.
//!
//! `cargo run -p contest --example converti_contest -- <origine> <destinazione>`

use contest::elenco::{converti_cst, sanifica_macro, testo_da_byte};
use std::path::{Path, PathBuf};

fn file_di(dir: &Path, ext: &str) -> Vec<PathBuf> {
    let mut v: Vec<PathBuf> = std::fs::read_dir(dir).map(|rd| rd.flatten().map(|e| e.path()).filter(|p| p.extension().is_some_and(|x| x.eq_ignore_ascii_case(ext))).collect()).unwrap_or_default();
    v.sort();
    v
}

/// Righe CRLF in UTF-8.
fn crlf(t: &str) -> String {
    let mut s = t.replace("\r\n", "\n").replace('\r', "\n").replace('\n', "\r\n");
    if !s.ends_with("\r\n") {
        s.push_str("\r\n");
    }
    s
}

fn main() -> Result<(), String> {
    let args: Vec<String> = std::env::args().collect();
    let (Some(src), Some(dst)) = (args.get(1).map(PathBuf::from), args.get(2).map(PathBuf::from)) else {
        return Err("uso: converti_contest <origine> <destinazione>".into());
    };
    let (cst_dir, mc_dir, dom_dir) = (src.join("Contest-Settings"), src.join("Contest-Macros"), src.join("Contest-DOM"));
    for d in ["", "macros", "dom"] {
        std::fs::create_dir_all(dst.join(d)).map_err(|e| e.to_string())?;
    }
    let mut conversioni = Vec::new();
    for p in file_di(&cst_dir, "cst") {
        let t = testo_da_byte(&std::fs::read(&p).map_err(|e| e.to_string())?);
        let stem = p.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
        conversioni.push(converti_cst(&t, &stem, &|m| mc_dir.join(m).is_file()));
    }
    // tutti i nominativi personali trovati: nessuno resta nelle macro
    let mut nominativi: Vec<String> = conversioni.iter().map(|c| c.nominativo.clone()).filter(|n| !n.is_empty()).collect();
    nominativi.sort();
    nominativi.dedup();
    let nom: Vec<&str> = nominativi.iter().map(String::as_str).collect();
    let (mut n_contest, mut n_macro, mut n_dom, mut senza_macro, mut dom_mancanti) = (0, 0, 0, 0, Vec::new());
    for mut c in conversioni {
        if let Some(m) = &c.macro_origine {
            let out = dst.join("macros").join(m);
            if !out.exists() {
                let t = testo_da_byte(&std::fs::read(mc_dir.join(m)).map_err(|e| e.to_string())?);
                std::fs::write(&out, crlf(&sanifica_macro(&t, &nom))).map_err(|e| e.to_string())?;
                n_macro += 1;
            }
        } else {
            senza_macro += 1;
        }
        if let Some(d) = &c.dom_origine {
            let from = dom_dir.join(d);
            if from.is_file() {
                let out = dst.join("dom").join(d);
                if !out.exists() {
                    std::fs::write(&out, crlf(&testo_da_byte(&std::fs::read(&from).map_err(|e| e.to_string())?))).map_err(|e| e.to_string())?;
                    n_dom += 1;
                }
            } else {
                dom_mancanti.push(d.clone());
                c.contest.domain_file.clear();
            }
        }
        // convertito dalle vecchie schede: Beta finche' non e' scritto dal regolamento ufficiale, e senza lo scambio fisso di chi le ha create
        c.contest.beta = true;
        c.contest.nrs.clear();
        std::fs::write(dst.join(format!("{}.contest", c.contest.id)), c.contest.in_testo()).map_err(|e| e.to_string())?;
        n_contest += 1;
    }
    // gli elenchi dei domini non usati da nessun contest servono lo stesso (si scelgono nella finestra di modifica)
    for p in file_di(&dom_dir, "dom") {
        if let Some(nome) = p.file_name() {
            let out = dst.join("dom").join(nome);
            if !out.exists() {
                std::fs::write(&out, crlf(&testo_da_byte(&std::fs::read(&p).map_err(|e| e.to_string())?))).map_err(|e| e.to_string())?;
                n_dom += 1;
            }
        }
    }
    println!("contest: {n_contest}, file di macro: {n_macro}, domini: {n_dom}, contest senza macro: {senza_macro}, domini mancanti: {dom_mancanti:?}");
    Ok(())
}
