//! I file delle macro: la cartella dei file di testo che la macro `<TEXTFILE:nome>` manda, e il riepilogo che risponde a
//! «Ripristina le macro di Aurora» (l'unica importazione rimasta: le macro di fabbrica, senza leggere file).

use std::path::PathBuf;

/// Il riepilogo di un ripristino delle macro (vuoto: le macro di Aurora non vengono da file).
#[derive(Debug, Default, Clone, PartialEq, serde::Serialize)]
pub struct Imported {
    pub folder: String,
    pub default_count: usize,
    pub default_file: Option<String>,
    pub modes: std::collections::BTreeMap<String, usize>,
    pub skipped: usize,
}

/// La cartella dei file di testo delle macro (`<TEXTFILE:nome>`): `%APPDATA%\Aurora\testi`.
pub fn text_folder() -> Option<PathBuf> {
    std::env::var_os("APPDATA").map(|a| PathBuf::from(a).join("Aurora").join("testi"))
}
