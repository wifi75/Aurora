//! Il controllo del locatore di WSJT-X (`chkgridvar`, dal decoder di JTDX): per un nominativo e un locatore a 4 caratteri dice se il locatore cade
//! in una zona dove servono verifiche (`chkcall`: mare, Antartide, zone senza stazioni), se e' valido per il paese del nominativo (`gvalid`) e se il
//! nominativo e' impossibile (`wrongcall`). La tabella (`tabella.rs`) e' tradotta in modo meccanico da `lib/ft8var/chkgridvar.f90` di WSJT-X
//! 3.3.0-beta6 e confrontata con la funzione `chkgridvar_` di `jt9.exe` della stessa versione.
//!
//! WSJT-X scarta un messaggio dubbio quando `wrongcall || !gvalid` (vedi `ft8::filters::grid_is_false`).

mod tabella;

use std::cmp::Ordering;

/// Le tre risposte di `chkgridvar` (`lchkcall`, `lgvalid`, `lwrongcall`).
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct Flags {
    pub chkcall: bool,
    pub gvalid: bool,
    pub wrongcall: bool,
}

/// Il controllo per un nominativo e un locatore (come in Fortran: 12 caratteri, maiuscoli, spazi a destra; il resto si taglia).
pub fn check(callsign: &str, grid: &str) -> Flags {
    let (c, g) = (field(callsign), field(grid));
    let mut flags = Flags::default();
    tabella::run(&c, &g, &mut flags);
    flags
}

fn field(s: &str) -> [u8; 12] {
    let mut out = [b' '; 12];
    for (o, b) in out.iter_mut().zip(s.trim().bytes()) {
        *o = b.to_ascii_uppercase();
    }
    out
}

/// `s(a:b)` del Fortran (posizioni da 1, estremi compresi).
fn sub(s: &[u8; 12], a: usize, b: usize) -> &[u8] {
    &s[a - 1..b]
}

/// Confronto tra caratteri del Fortran: il piu' corto si allunga con spazi.
fn cmp(x: &[u8], lit: &[u8]) -> Ordering {
    let n = x.len().max(lit.len());
    (0..n).map(|i| (x.get(i).copied().unwrap_or(b' '), lit.get(i).copied().unwrap_or(b' '))).find(|(a, b)| a != b).map_or(Ordering::Equal, |(a, b)| a.cmp(&b))
}

fn eq(x: &[u8], lit: &[u8]) -> bool {
    cmp(x, lit) == Ordering::Equal
}

fn ne(x: &[u8], lit: &[u8]) -> bool {
    cmp(x, lit) != Ordering::Equal
}

fn gt(x: &[u8], lit: &[u8]) -> bool {
    cmp(x, lit) == Ordering::Greater
}

fn lt(x: &[u8], lit: &[u8]) -> bool {
    cmp(x, lit) == Ordering::Less
}

/// `len_trim` del Fortran.
fn len_trim(s: &[u8; 12]) -> usize {
    s.iter().rposition(|&b| b != b' ').map_or(0, |i| i + 1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fortran_comparisons_pad_with_spaces() {
        assert!(eq(b"A", b"A "));
        assert!(gt(b"B", b"A"));
        assert!(lt(b"0", b":"));
        assert_eq!(len_trim(&field("K1ABC")), 5);
    }

    #[test]
    fn examples_written_in_the_table() {
        // Sovrano Militare Ordine di Malta: 1A solo in JN61
        assert!(check("1A0KM", "JN61").gvalid);
        assert!(!check("1A0KM", "JN45").gvalid);
        // Singapore: 9V in OJ11 e OJ21
        assert!(check("9V1AB", "OJ11").gvalid);
        assert!(!check("9V1AB", "JN45").gvalid);
        // Burundi 9U: KI con terzo carattere 4-5 e quarto 5-7
        assert!(check("9U1AB", "KI46").gvalid);
        // Z1, Z7, Z9 non esistono
        assert!(check("Z12AB", "JN45").wrongcall);
        // AA00 e' in una zona da verificare: non valido per nessuno
        let f = check("IU3ABC", "AA00");
        assert!(f.chkcall && !f.gvalid);
    }
}
