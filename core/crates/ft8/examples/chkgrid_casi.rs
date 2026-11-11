//! Per confrontare la tabella di `ft8::chkgrid` con `chkgridvar_` di `jt9.exe`: legge righe `NOMINATIVO LOCATORE` e scrive
//! `NOMINATIVO LOCATORE chkcall gvalid wrongcall` (0/1).
//!
//! Uso: `cargo run --release -p ft8 --example chkgrid_casi < casi.txt > rust.txt`

use std::io::{BufRead, Write};

fn main() {
    let stdin = std::io::stdin();
    let mut out = std::io::BufWriter::new(std::io::stdout());
    for line in stdin.lock().lines().map_while(Result::ok) {
        let mut it = line.split_whitespace();
        let (Some(call), Some(grid)) = (it.next(), it.next()) else { continue };
        let f = ft8::chkgrid::check(call, grid);
        let _ = writeln!(out, "{call} {grid} {} {} {}", u8::from(f.chkcall), u8::from(f.gvalid), u8::from(f.wrongcall));
    }
}
