//! I file di Aurora che l'antivirus ha tolto (`GET /api/v1/health/files`): Aurora, all'avvio, controlla i file principali accanto al
//! programma e dice qui quali mancano e quale antivirus c'e'. La pagina lo mostra come avviso fisso, finche' non si rimettono a posto.
//! Il controllo vero sta nel programma (`antivirus.rs`): qui c'e' solo lo stato e, per «Ripristina Hamlib», il modo di rifare il conto.

use std::sync::{Mutex, OnceLock};

use serde_json::{Value, json};

#[derive(Default)]
struct Stato {
    missing: Vec<String>,
    antivirus: String,
}

static STATO: Mutex<Stato> = Mutex::new(Stato { missing: Vec::new(), antivirus: String::new() });
static RICALCOLO: OnceLock<fn() -> Vec<String>> = OnceLock::new();

/// Scrive i file che mancano e il nome dell'antivirus (vuoto se non si conosce).
pub fn set(missing: Vec<String>, antivirus: &str) {
    if let Ok(mut s) = STATO.lock() {
        s.missing = missing;
        s.antivirus = antivirus.to_string();
    }
}

/// Indica come rifare il conto dei file mancanti (lo chiama il programma all'avvio).
pub fn set_recompute(f: fn() -> Vec<String>) {
    let _ = RICALCOLO.set(f);
}

/// Rifa il conto dei file mancanti (dopo «Ripristina Hamlib»), tenendo il nome dell'antivirus. Senza ricalcolo registrato non fa nulla.
pub fn refresh() {
    let Some(f) = RICALCOLO.get() else { return };
    let missing = f();
    if let Ok(mut s) = STATO.lock() {
        s.missing = missing;
    }
}

/// Il corpo della risposta: `{ "missing": [..], "antivirus": ".." }`.
pub fn snapshot() -> Value {
    match STATO.lock() {
        Ok(s) => json!({ "missing": s.missing, "antivirus": s.antivirus }),
        Err(_) => json!({ "missing": [], "antivirus": "" }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_state_is_serialised() {
        set(vec![r"hamlib\rigctld.exe".into(), r"cty\cty.dat".into()], "Avast Free Antivirus");
        let v = snapshot();
        assert_eq!(v["missing"], json!([r"hamlib\rigctld.exe", r"cty\cty.dat"]));
        assert_eq!(v["antivirus"], "Avast Free Antivirus");
        set(Vec::new(), "");
        assert_eq!(snapshot(), json!({ "missing": [], "antivirus": "" }));
    }
}
