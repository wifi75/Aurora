//! Messaggi di tipo 0.6 a 77 bit («WSPR dentro i 77 bit», 50 bit utili): quelli di FST4W, e di FST4 quando il testo ha la forma
//! di un messaggio WSPR. Porting di WSJT-X 3.3.0-beta6 (GPL 3), `lib/77bit/packjt77.f90` (`pack77_06_candidate` 2565-2609,
//! `unpack77_decode_i3_0` 2046-2111), `packjt77_grammar.f90` (`pack77_parse_wspr_source` 1082-1164, `pack77_parse_wspr_dbm`
//! 522-542, `pack77_wspr_affix_index` 1348-1399, `pack77_wspr_affix_text` 1401-1435, `to_grid4` 1465, `to_grid` 1523,
//! `pack77_grid6_wspr_index` 1615) e `packjt77_schema.f90` (campi dei tipi 1, 2 e 3: 96-128).
//!
//! Tre sottotipi:
//! - 1: `NOMINATIVO LOC4 dBm` (n28, locatore a 15 bit, potenza a 5 bit);
//! - 2: `PFX/NOMINATIVO dBm` o `NOMINATIVO/SFX dBm` (n28, affisso a 16 bit, potenza);
//! - 3: `<NOMINATIVO> LOC6` (hash a 22 bit e locatore a 25 bit), solo in FST4W.
//!
//! Va unito al pacchettamento WSPR a 50 bit del modo WSPR quando ci sara' (stessi campi, altro ordine dei bit).

use ft8::message::{Bits77, CallHashes, hash_call, is_standard_call, pack28_standard, unpack28_standard};

const BASE36: &[u8; 36] = b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ";
const NZZZ: u32 = 46_656;

fn put(bits: &mut Bits77, start1: usize, width: usize, v: u32) {
    for i in 0..width {
        bits[start1 - 1 + i] = ((v >> (width - 1 - i)) & 1) as u8;
    }
}

fn get(bits: &Bits77, start1: usize, width: usize) -> u32 {
    bits[start1 - 1..start1 - 1 + width].iter().fold(0u32, |a, &b| (a << 1) | u32::from(b & 1))
}

/// `pack77_split_source_tokens`: maiuscole, spazi multipli ridotti.
fn tokens(text: &str) -> Vec<String> {
    text.split([' ', '\0']).filter(|w| !w.is_empty()).map(str::to_ascii_uppercase).collect()
}

/// `pack77_type12_standard_call`: nominativo standard a 28 bit che non comincia per Q.
fn type12_standard_call(c: &str) -> bool {
    !c.starts_with('Q') && is_standard_call(c)
}

fn is_grid4(g: &str) -> bool {
    let b = g.as_bytes();
    b.len() == 4 && (b'A'..=b'R').contains(&b[0]) && (b'A'..=b'R').contains(&b[1]) && b[2].is_ascii_digit() && b[3].is_ascii_digit()
}

fn is_grid6(g: &str) -> bool {
    let b = g.as_bytes();
    match b.len() {
        4 => is_grid4(g),
        6 => g.get(..4).is_some_and(is_grid4) && (b'A'..=b'X').contains(&b[4]) && (b'A'..=b'X').contains(&b[5]),
        _ => false,
    }
}

fn grid4_index(g: &[u8]) -> u32 {
    u32::from(g[0] - b'A') * 1800 + u32::from(g[1] - b'A') * 100 + u32::from(g[2] - b'0') * 10 + u32::from(g[3] - b'0')
}

/// `pack77_parse_wspr_dbm`: da 0 a 60 dBm, solo i valori che tornano uguali (`nint(0.3*dBm)` e ritorno).
fn parse_dbm(t: &str) -> Option<u32> {
    if t.is_empty() || !t.bytes().all(|c| c.is_ascii_digit()) || (t.len() > 1 && t.starts_with('0')) {
        return None;
    }
    let dbm: u32 = t.parse().ok()?;
    if dbm > 60 {
        return None;
    }
    let idbm = (0.3f32 * dbm as f32).round() as u32;
    (dbm_of(idbm) == dbm).then_some(idbm)
}

fn dbm_of(idbm: u32) -> u32 {
    (idbm as f32 * 10.0 / 3.0).round() as u32
}

fn b36(c: u8) -> Option<u32> {
    BASE36.iter().position(|&x| x == c).map(|p| p as u32)
}

/// `pack77_wspr_affix_index`: `slash` e' la posizione (da 1) della barra, `m1` la lunghezza.
fn affix_index(t: &[u8], slash: usize, m1: usize) -> Option<u32> {
    if slash <= 4 && m1 - slash < 3 {
        return None;
    }
    if slash > 4 && m1 - slash > 3 {
        return None;
    }
    for (i, &c) in t.iter().enumerate().take(m1) {
        if i + 1 != slash {
            b36(c)?;
        }
    }
    let at = |i1: usize| b36(t[i1 - 1]).unwrap_or(0);
    let mut npfx;
    if slash <= 4 {
        npfx = at(1);
        if slash >= 3 {
            npfx = 36 * npfx + at(2);
        }
        if slash == 4 {
            npfx = 36 * npfx + at(3);
        }
    } else {
        npfx = 0;
        match m1 - slash {
            1 => npfx = at(slash + 1),
            2 => npfx = 36 * at(slash + 1) + at(slash + 2),
            3 => {
                if !t[slash + 2].is_ascii_digit() {
                    return None;
                }
                npfx = 360 * at(slash + 1) + 10 * at(slash + 2) + at(slash + 3);
            }
            _ => {}
        }
        npfx += NZZZ;
    }
    Some(npfx)
}

/// `pack77_wspr_affix_text`: testo dell'affisso e se e' un prefisso.
fn affix_text(npfx: u32) -> Option<(String, bool)> {
    let mut c = [b' '; 3];
    if npfx < NZZZ {
        let mut n = npfx;
        for i in (0..3).rev() {
            c[i] = BASE36[(n % 36) as usize];
            n /= 36;
            if n == 0 {
                break;
            }
        }
        Some((String::from_utf8_lossy(&c).trim().to_string(), true))
    } else {
        let n = npfx - NZZZ;
        if n <= 35 {
            c[0] = BASE36[n as usize];
        } else if n <= 1295 {
            c[0] = BASE36[(n / 36) as usize];
            c[1] = BASE36[(n % 36) as usize];
        } else if n <= 12959 {
            c[0] = BASE36[(n / 360) as usize];
            c[1] = BASE36[((n / 10) % 36) as usize];
            c[2] = BASE36[(n % 10) as usize];
        } else {
            return None;
        }
        Some((String::from_utf8_lossy(&c).trim().to_string(), false))
    }
}

/// Impacchetta un messaggio di tipo 0.6 se il testo ne ha la forma (`pack77_06_candidate`). Il sottotipo 3 (`<CALL> LOC6`) solo
/// con `prefer_wspr` (FST4W). Memorizza il nominativo negli hash come WSJT-X (`commit_hash_facts`).
pub fn pack(text: &str, hashes: &mut CallHashes, prefer_wspr: bool) -> Option<Bits77> {
    let w = tokens(text);
    if w.len() < 2 || w.len() > 3 {
        return None;
    }
    let mut bits = [0u8; 77];
    if w.len() == 3 {
        if !type12_standard_call(&w[0]) || !is_grid4(&w[1]) {
            return None;
        }
        let idbm = parse_dbm(&w[2])?;
        let n28 = pack28_standard(&w[0])?;
        put(&mut bits, 1, 28, n28);
        put(&mut bits, 29, 15, grid4_index(w[1].as_bytes()));
        put(&mut bits, 44, 5, idbm);
        put(&mut bits, 72, 3, 6);
        hashes.save_call(&w[0]);
        return Some(bits);
    }
    let (w1, w2) = (w[0].as_bytes(), &w[1]);
    let (m1, m2) = (w1.len(), w2.len());
    if (5..=10).contains(&m1)
        && m2 <= 2
        && let Some(s0) = w[0].find('/')
    {
        {
            let slash = s0 + 1;
            if slash >= 2 && slash < m1 && !w[0][slash..].contains('/') {
                let base = if slash <= 4 { &w[0][slash..] } else { &w[0][..slash - 1] };
                if type12_standard_call(base) {
                    let idbm = parse_dbm(w2)?;
                    let npfx = affix_index(w1, slash, m1)?;
                    let (txt, is_prefix) = affix_text(npfx)?;
                    if slash <= 4 {
                        if !is_prefix || txt != w[0][..slash - 1] {
                            return None;
                        }
                    } else if is_prefix || txt != w[0][slash..] {
                        return None;
                    }
                    let n28 = pack28_standard(base)?;
                    put(&mut bits, 1, 28, n28);
                    put(&mut bits, 29, 16, npfx);
                    put(&mut bits, 45, 5, idbm);
                    put(&mut bits, 50, 1, 1);
                    put(&mut bits, 72, 3, 6);
                    hashes.save_call(&w[0]);
                    return Some(bits);
                }
            }
        }
    }
    if prefer_wspr && (5..=12).contains(&m1) && m2 <= 6 {
        let inner = w[0].strip_prefix('<').and_then(|s| s.strip_suffix('>'))?;
        if m1 > 13 || inner.contains('<') || inner.contains('>') || !is_grid6(w2) {
            return None;
        }
        let g = w2.as_bytes();
        let mut igrid6 = grid4_index(&g[..4]) * 625;
        igrid6 += if g.len() == 6 { u32::from(g[4] - b'A') * 25 + u32::from(g[5] - b'A') } else { 24 * 25 + 24 };
        put(&mut bits, 1, 22, hash_call(inner, 22));
        put(&mut bits, 23, 25, igrid6);
        put(&mut bits, 48, 3, 2);
        put(&mut bits, 72, 3, 6);
        hashes.save_call(inner);
        return Some(bits);
    }
    None
}

/// `true` se i bit sono di tipo 0.6.
pub fn is_type06(bits: &Bits77) -> bool {
    get(bits, 75, 3) == 0 && get(bits, 72, 3) == 6
}

fn to_grid4(n: u32) -> Option<String> {
    let (j1, r) = (n / 1800, n % 1800);
    let (j2, r) = (r / 100, r % 100);
    if j1 > 17 || j2 > 17 {
        return None;
    }
    Some(format!("{}{}{}{}", (b'A' + j1 as u8) as char, (b'A' + j2 as u8) as char, r / 10, r % 10))
}

fn to_grid(n: u32) -> Option<String> {
    let j1 = n / (18 * 100 * 625);
    let r = n % (18 * 100 * 625);
    let j2 = r / (100 * 625);
    let r = r % (100 * 625);
    let j3 = r / (10 * 625);
    let r = r % (10 * 625);
    let j4 = r / 625;
    let r = r % 625;
    let (j5, j6) = (r / 25, r % 25);
    if j1 > 17 || j2 > 17 {
        return None;
    }
    let mut g = format!("{}{}{}{}", (b'A' + j1 as u8) as char, (b'A' + j2 as u8) as char, j3, j4);
    if j5 != 24 || j6 != 24 {
        g.push((b'A' + j5 as u8) as char);
        g.push((b'A' + j6 as u8) as char);
    }
    Some(g)
}

/// Il testo di un messaggio di tipo 0.6 (`unpack77_decode_i3_0`, ramo `n3 = 6`), o `None` se i bit non sono validi.
pub fn unpack(bits: &Bits77, hashes: &mut CallHashes) -> Option<String> {
    if !is_type06(bits) || get(bits, 51, 21) != 0 {
        return None;
    }
    if get(bits, 50, 1) == 1 {
        // tipo 2
        let idbm = get(bits, 45, 5);
        if idbm > 18 {
            return None;
        }
        let call = unpack28_standard(get(bits, 1, 28))?;
        let (pfx, is_prefix) = affix_text(get(bits, 29, 16))?;
        let full = if is_prefix { format!("{pfx}/{call}") } else { format!("{call}/{pfx}") };
        hashes.save_call(&full);
        return Some(format!("{full} {}", dbm_of(idbm)));
    }
    if get(bits, 49, 1) == 0 {
        // tipo 1
        let idbm = get(bits, 44, 5);
        if idbm > 18 {
            return None;
        }
        let call = unpack28_standard(get(bits, 1, 28))?;
        let grid = to_grid4(get(bits, 29, 15))?;
        hashes.save_call(&call);
        return Some(format!("{call} {grid} {}", dbm_of(idbm)));
    }
    // tipo 3: il selettore deve valere 2 (`010`)
    if get(bits, 48, 3) != 2 {
        return None;
    }
    let n22 = get(bits, 1, 22);
    let call = hashes.call22(n22).map_or_else(|| "<...>".to_string(), |c| format!("<{c}>"));
    let grid = to_grid(get(bits, 23, 25))?;
    Some(format!("{call} {grid}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn round(text: &str, prefer: bool) -> Option<String> {
        let mut h = CallHashes::new();
        let b = pack(text, &mut h, prefer)?;
        unpack(&b, &mut h)
    }

    #[test]
    fn i_tre_sottotipi_tornano_uguali() {
        assert_eq!(round("IU3ABC IO91 37", true).as_deref(), Some("IU3ABC IO91 37"));
        assert_eq!(round("iu3abc  io91 30", false).as_deref(), Some("IU3ABC IO91 30"));
        assert_eq!(round("PJ4/K1ABC 27", true).as_deref(), Some("PJ4/K1ABC 27"));
        assert_eq!(round("K1ABC/P 10", true).as_deref(), Some("K1ABC/P 10"));
        assert_eq!(round("K1ABC/12 10", true).as_deref(), Some("K1ABC/12 10"));
        assert_eq!(round("<IU3ABC> IO91WM", true).as_deref(), Some("<IU3ABC> IO91WM"));
        assert_eq!(round("<IU3ABC> IO91", true).as_deref(), Some("<IU3ABC> IO91"));
    }

    #[test]
    fn potenze_e_forme_non_valide() {
        assert!(round("IU3ABC IO91 5", true).is_none());
        assert!(round("IU3ABC IO91 61", true).is_none());
        assert!(round("IU3ABC IO91 07", true).is_none());
        assert!(round("<IU3ABC> IO91WM", false).is_none());
        assert!(round("CQ IU3ABC IO91", true).is_none());
        assert!(round("QA1ABC IO91 30", true).is_none());
    }

    #[test]
    fn hash_sconosciuto() {
        let mut h = CallHashes::new();
        let b = pack("<IU3ABC> IO91WM", &mut h, true).unwrap_or([0; 77]);
        let mut vuoto = CallHashes::new();
        assert_eq!(unpack(&b, &mut vuoto).as_deref(), Some("<...> IO91WM"));
    }
}
