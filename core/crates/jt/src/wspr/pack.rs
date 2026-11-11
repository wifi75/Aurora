//! Messaggi WSPR a 50 bit: nominativo (28 bit), locatore e potenza (22 bit). Tre tipi come in WSJT-X:
//!
//! - tipo 1: `K1ABC FN42 37` (nominativo standard, locatore a 4 caratteri, potenza in dBm 0, 3, 7, 10 ... 60);
//! - tipo 2: `PJ4/K1ABC 37`, `K1ABC/P 37` (nominativo composto, senza locatore: il prefisso o suffisso va nei 15 bit del locatore);
//! - tipo 3: `<K1ABC> FN42AB 37` (hash a 15 bit del nominativo e locatore a 6 caratteri al posto del nominativo).
//!
//! Codifica come `get_wspr_channel_symbols` di `lib/wsprd/wsprsim_utils.c:608-755` (usata da wsprd per la sottrazione) e `wqencode.f90`
//! (usata in trasmissione: la potenza si arrotonda ai valori ammessi anche nel tipo 1, `wqencode.f90:29-31`); decodifica come `unpk_` di
//! `lib/wsprd/wsprd_utils.c:299-397`, con la tabella degli hash (`nhash.c`, `lookup3.c`) che wsprd tiene in `hashtable.txt`.

// cicli con l'indice come nel C di wsprd, per confrontarli riga per riga
#![allow(clippy::needless_range_loop)]

use crate::msk40::hashlittle;
use std::collections::HashMap;

/// Righe della tabella degli hash: 15 bit (`WSPRD_HASH_COUNT`, `wsprd_utils.h:18`).
pub const HASH_COUNT: usize = 32_768;
/// `WSPRD_CALLSIGN_SIZE`, `WSPRD_MESSAGE_SIZE` (`wsprd_utils.h:14-17`): lunghezze massime con il terminatore del C.
const CALLSIGN_SIZE: usize = 13;
const MESSAGE_SIZE: usize = 23;

const ALPHABET: &[u8; 37] = b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ ";
/// Correzione della potenza ai valori ammessi (`nu` di `wqencode.f90:14` e `wsprsim_utils.c:622`).
const NU: [i32; 10] = [0, -1, 1, 0, -1, 2, 1, 0, -1, 1];

/// `nhash` (`lib/wsprd/nhash.c`): `hashlittle` con valore iniziale 146, ridotto a 15 bit.
pub fn nhash(text: &str) -> u16 {
    let h = hashlittle(text.as_bytes(), 146);
    (h & 0x7fff) as u16
}

/// La tabella degli hash dei nominativi e dei loro locatori (`hashtab` e `loctab` di `wsprd.c:799-802`).
#[derive(Debug, Clone, Default)]
pub struct HashTable {
    calls: HashMap<u16, String>,
    grids: HashMap<u16, String>,
}

impl HashTable {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn call(&self, ihash: u16) -> Option<&str> {
        self.calls.get(&ihash).map(String::as_str)
    }

    pub fn grid(&self, ihash: u16) -> Option<&str> {
        self.grids.get(&ihash).map(String::as_str)
    }

    pub fn len(&self) -> usize {
        self.calls.len()
    }

    pub fn is_empty(&self) -> bool {
        self.calls.is_empty()
    }

    /// `store_hash_call` (`wsprd_utils.c:72-79`).
    fn store_call(&mut self, ihash: u16, call: &str) -> bool {
        if call.len() >= CALLSIGN_SIZE {
            return false;
        }
        self.calls.insert(ihash, call.to_string());
        true
    }

    /// `store_hash_entry` (`wsprd_utils.c:90-96`): nominativo e locatore.
    fn store_entry(&mut self, ihash: u16, call: &str, grid: &str) -> bool {
        if !self.store_call(ihash, call) || grid.len() >= 5 {
            return false;
        }
        self.grids.insert(ihash, grid.to_string());
        true
    }

    /// Una riga di `hashtable.txt` (`wsprd_load_hash_line`, `wsprd_utils.c:98-117`): `indice nominativo [locatore]`.
    pub fn load_line(&mut self, line: &str) -> bool {
        let mut w = line.split_whitespace();
        let Some(ihash) = w.next().and_then(|s| s.parse::<usize>().ok()).filter(|&i| i < HASH_COUNT) else {
            return false;
        };
        let Some(call) = w.next() else {
            return false;
        };
        let ihash = ihash as u16;
        if !self.store_call(ihash, call) {
            return false;
        }
        // senza locatore resta quello che c'era (`wsprd_utils.c:113`)
        if let Some(grid) = w.next() {
            if grid.len() >= 5 {
                return false;
            }
            self.grids.insert(ihash, grid.to_string());
        }
        true
    }

    /// Le righe di `hashtable.txt` come le scrive wsprd (`wsprd.c:1637-1646`: `%5d %s %s`), in ordine di indice.
    pub fn to_lines(&self) -> Vec<String> {
        let mut keys: Vec<u16> = self.calls.keys().copied().collect();
        keys.sort_unstable();
        keys.iter()
            .map(|k| format!("{:5} {} {}", k, self.calls[k], self.grids.get(k).map(String::as_str).unwrap_or("")))
            .collect()
    }
}

/// `unpack50` (`wsprd_utils.c:119-149`): dai primi 7 byte i due campi da 28 e 22 bit.
pub fn unpack50(dat: &[u8]) -> (u32, u32) {
    let b = |i: usize| u32::from(dat.get(i).copied().unwrap_or(0));
    let n1 = (b(0) << 20) + (b(1) << 12) + (b(2) << 4) + ((b(3) >> 4) & 15);
    let n2 = ((b(3) & 15) << 18) + (b(4) << 10) + (b(5) << 2) + ((b(6) >> 6) & 3);
    (n1, n2)
}

/// `pack50` (`packjt.f90:955-979`): i due campi in 11 byte (50 bit e 31 zeri di coda per il codice convoluzionale).
pub fn pack50(n1: u32, n2: u32) -> [u8; 11] {
    let mut d = [0u8; 11];
    d[0] = ((n1 >> 20) & 0xff) as u8;
    d[1] = ((n1 >> 12) & 0xff) as u8;
    d[2] = ((n1 >> 4) & 0xff) as u8;
    d[3] = (((n1 & 0x0f) << 4) + ((n2 >> 18) & 0x0f)) as u8;
    d[4] = ((n2 >> 10) & 0xff) as u8;
    d[5] = ((n2 >> 2) & 0xff) as u8;
    d[6] = ((n2 & 0x03) << 6) as u8;
    d
}

/// `unpackcall` (`wsprd_utils.c:151-195`): il nominativo a 28 bit, senza gli spazi in testa e fino al primo spazio.
pub fn unpack_call(ncall: u32) -> Option<String> {
    if ncall >= 262_177_560 {
        return None;
    }
    let mut n = ncall;
    let mut tmp = [b' '; 6];
    for i in (3..6).rev() {
        tmp[i] = ALPHABET[(n % 27 + 10) as usize];
        n /= 27;
    }
    tmp[2] = ALPHABET[(n % 10) as usize];
    n /= 10;
    tmp[1] = ALPHABET[(n % 36) as usize];
    n /= 36;
    tmp[0] = ALPHABET[n as usize];
    let start = (0..5).find(|&i| tmp[i] != b' ').unwrap_or(5);
    let s: String = tmp[start..].iter().take_while(|&&c| c != b' ').map(|&c| c as char).collect();
    Some(s)
}

/// `unpackgrid` (`wsprd_utils.c:197-229`): il locatore a 4 caratteri dai 15 bit alti di `ngrid`.
pub fn unpack_grid(ngrid: u32) -> Option<String> {
    let ngrid = (ngrid >> 7) as i32;
    if ngrid >= 32_400 {
        return None;
    }
    let dlat = (ngrid % 180) - 90;
    let mut dlong = (ngrid / 180) * 2 - 180 + 2;
    if dlong < -180 {
        dlong += 360;
    }
    if dlong > 180 {
        dlong += 360;
    }
    let nlong = (60.0 * (180.0 - f64::from(dlong)) / 5.0) as i32;
    let n1 = nlong / 240;
    let n2 = (nlong - 240 * n1) / 24;
    let nlat = (60.0 * f64::from(dlat + 90) / 2.5) as i32;
    let m1 = nlat / 240;
    let m2 = (nlat - 240 * m1) / 24;
    let ch = |i: i32| ALPHABET.get(usize::try_from(i).ok()?).map(|&c| c as char);
    Some([ch(10 + n1)?, ch(10 + m1)?, ch(n2)?, ch(m2)?].iter().collect())
}

/// `unpackpfx` (`wsprd_utils.c:231-281`): aggiunge al nominativo il prefisso (fino a 3 caratteri) o il suffisso (1 carattere o 2 cifre).
pub fn unpack_pfx(nprefix: u32, call: &str) -> Option<String> {
    if call.len() >= 7 {
        return None;
    }
    let out = if nprefix < 60_000 {
        let mut n = nprefix;
        let mut pfx = [b' '; 3];
        for i in (0..3).rev() {
            let nc = n % 37;
            pfx[i] = match nc {
                0..=9 => nc as u8 + 48,
                10..=35 => nc as u8 + 55,
                _ => b' ',
            };
            n /= 37;
        }
        // `strrchr(pfx, ' ')`: dopo l'ultimo spazio
        let p = match pfx.iter().rposition(|&c| c == b' ') {
            Some(i) => &pfx[i + 1..],
            None => &pfx[..],
        };
        format!("{}/{}", String::from_utf8_lossy(p), call)
    } else {
        let nc = nprefix - 60_000;
        match nc {
            0..=9 => format!("{}/{}", call, (nc as u8 + 48) as char),
            10..=35 => format!("{}/{}", call, (nc as u8 + 55) as char),
            36..=125 => format!("{}/{}{}", call, ((nc - 26) / 10) as u8 + 48, ((nc - 26) % 10) as u8 + 48),
            _ => return None,
        }
    };
    (out.len() < CALLSIGN_SIZE).then_some(out)
}

/// Il risultato di [`unpack`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Unpacked {
    /// Il messaggio con uno spazio fra i campi (wsprd allinea la potenza a due cifre con `%2d`).
    pub message: String,
    /// Il nominativo per scartare i doppioni (`callsign` di `unpk_`): con `<>` nel tipo 3.
    pub call: String,
    /// `noprint`: messaggio decodificato ma non da mostrare (potenza non ammessa, locatore strano, `ntype` -64).
    pub noprint: bool,
}

/// `unpk_` (`wsprd_utils.c:299-397`): da 11 byte decodificati al testo, aggiornando la tabella degli hash. `None` se i campi non si
/// leggono (wsprd ritorna 1, come un `noprint`).
pub fn unpack(data: &[u8], table: &mut HashTable) -> Option<Unpacked> {
    let (n1, n2) = unpack50(data);
    let mut callsign = unpack_call(n1)?;
    let grid = unpack_grid(n2)?;
    let ntype = (n2 & 127) as i32 - 64;
    if ntype == 63 {
        return None;
    }
    let mut noprint = false;
    let message;
    if (0..=62).contains(&ntype) {
        let nu = ntype % 10;
        if nu == 0 || nu == 3 || nu == 7 {
            let ndbm = ntype;
            message = format!("{callsign} {grid} {ndbm}");
            let ih = nhash(&callsign);
            if !table.store_entry(ih, &callsign, &grid) {
                return None;
            }
        } else {
            let mut nadd = nu;
            if nu > 3 {
                nadd = nu - 3;
            }
            if nu > 7 {
                nadd = nu - 7;
            }
            let n3 = (n2 / 128) as i32 + 32_768 * (nadd - 1);
            callsign = unpack_pfx(u32::try_from(n3).ok()?, &callsign)?;
            let ndbm = ntype - nadd;
            message = format!("{callsign} {ndbm}");
            let nu = ndbm % 10;
            if nu == 0 || nu == 3 || nu == 7 || nu == 10 {
                let ih = nhash(&callsign);
                if !table.store_call(ih, &callsign) {
                    return None;
                }
            } else {
                noprint = true;
            }
        }
    } else {
        // tipo 3: il "nominativo" e' il locatore a 6 caratteri ruotato (`wsprd_utils.c:363-372`, `len` fisso a 6)
        let ndbm = -(ntype + 1);
        let mut buf = [0u8; 6];
        for (b, c) in buf.iter_mut().zip(callsign.bytes()) {
            *b = c;
        }
        let mut grid6 = String::new();
        if buf[5] != 0 {
            grid6.push(buf[5] as char);
        }
        grid6.extend(buf[..5].iter().take_while(|&&c| c != 0).map(|&c| c as char));
        let g = grid6.as_bytes();
        let nu = ndbm % 10;
        let alpha = |i: usize| g.get(i).is_some_and(u8::is_ascii_alphabetic);
        let digit = |i: usize| g.get(i).is_some_and(u8::is_ascii_digit);
        if (nu != 0 && nu != 3 && nu != 7 && nu != 10) || !alpha(0) || !alpha(1) || !digit(2) || !digit(3) {
            noprint = true;
        }
        let ihash = ((n2 as i32 - ntype - 64) / 128) as u16;
        callsign = match table.call(ihash) {
            Some(c) if !c.is_empty() => format!("<{c}>"),
            _ => "<...>".to_string(),
        };
        if callsign.len() >= CALLSIGN_SIZE {
            return None;
        }
        message = format!("{callsign} {grid6} {ndbm}");
        if ntype == -64 {
            noprint = true;
        }
    }
    // `format_checked` con `WSPRD_MESSAGE_SIZE`: con la potenza a `%2d` il testo di wsprd e' al piu' un carattere piu' lungo
    let padded = message.len() + usize::from(message.rsplit(' ').next().is_some_and(|p| p.len() == 1));
    if padded >= MESSAGE_SIZE {
        return None;
    }
    Some(Unpacked { message, call: callsign, noprint })
}

/// Codice di un carattere del nominativo (`get_callsign_character_code`, `wsprsim_utils.c:472-483`).
fn call_code(c: u8) -> Option<u32> {
    match c {
        b'0'..=b'9' => Some(u32::from(c - b'0')),
        b' ' => Some(36),
        b'A'..=b'Z' => Some(u32::from(c - b'A') + 10),
        _ => None,
    }
}

/// `packcall` (`packjt.f90:56-138`): un nominativo standard (al piu' 6 caratteri, cifra in terza o seconda posizione) a 28 bit.
pub fn pack_call(call: &str) -> Option<u32> {
    let mut c: Vec<u8> = call.trim().to_ascii_uppercase().into_bytes();
    if c.len() > 6 {
        return None;
    }
    // `packjt.f90:66-71`: Swaziland 3DA0 e Guinea 3X
    if c.starts_with(b"3DA0") {
        c = [b"3D0".as_slice(), &c[4..]].concat();
    } else if c.len() >= 3 && c.starts_with(b"3X") && c[2].is_ascii_uppercase() {
        c = [b"Q".as_slice(), &c[2..]].concat();
    }
    let at = |i: usize| c.get(i).copied().unwrap_or(b' ');
    let mut tmp = [b' '; 6];
    if at(2).is_ascii_digit() {
        for (i, t) in tmp.iter_mut().enumerate() {
            *t = at(i);
        }
    } else if at(1).is_ascii_digit() {
        if at(5) != b' ' {
            return None;
        }
        for i in 1..6 {
            tmp[i] = at(i - 1);
        }
    } else {
        return None;
    }
    let letter_or_space = |b: u8| b.is_ascii_uppercase() || b == b' ';
    let ok = (letter_or_space(tmp[0]) || tmp[0].is_ascii_digit())
        && tmp[1].is_ascii_alphanumeric()
        && tmp[2].is_ascii_digit()
        && letter_or_space(tmp[3])
        && letter_or_space(tmp[4])
        && letter_or_space(tmp[5]);
    if !ok {
        return None;
    }
    let mut n = call_code(tmp[0])?;
    n = n * 36 + call_code(tmp[1])?;
    n = n * 10 + call_code(tmp[2])?;
    n = n * 27 + call_code(tmp[3])? - 10;
    n = n * 27 + call_code(tmp[4])? - 10;
    n = n * 27 + call_code(tmp[5])? - 10;
    Some(n)
}

/// `pack_grid4_power` (`wsprsim_utils.c:485-491`): locatore a 4 caratteri e potenza nei 22 bit.
fn pack_grid4_power(grid: &str, power: i32) -> Option<u32> {
    let g = grid.as_bytes();
    if g.len() < 4 {
        return None;
    }
    let field = |c: u8| (b'A'..=b'R').contains(&c).then(|| i32::from(c - b'A'));
    let digit = |c: u8| c.is_ascii_digit().then(|| i32::from(c - b'0'));
    let (g0, g1, g2, g3) = (field(g[0])?, field(g[1])?, digit(g[2])?, digit(g[3])?);
    let m = (179 - 10 * g0 - g2) * 180 + 10 * g1 + g3;
    u32::try_from(m * 128 + power + 64).ok()
}

/// La potenza arrotondata ai valori ammessi 0, 3, 7, 10 ... 60 dBm (`wqencode.f90:29-31`).
pub fn normalize_power(dbm: i32) -> i32 {
    let p = dbm.clamp(0, 60);
    p + NU[(p % 10) as usize]
}

/// Prefisso o suffisso di un nominativo composto (`packpfx`, `packjt.f90:981-1039`): nominativo base, campo da 15 bit e `nadd`.
fn pack_prefix(call: &str) -> Option<(u32, i32, i32)> {
    let i1 = call.find('/')?;
    let (base, rest) = (&call[..i1], &call[i1 + 1..]);
    let code = |c: u8| match c {
        b'0'..=b'9' => i32::from(c - b'0'),
        b'A'..=b'Z' => i32::from(c - b'A') + 10,
        _ => 36,
    };
    if rest.len() == 1 {
        // suffisso di un carattere
        let n1 = pack_call(base)?;
        let c = rest.as_bytes()[0];
        let n = match c {
            b'0'..=b'9' => i32::from(c - b'0'),
            b'A'..=b'Z' => i32::from(c - b'A') + 10,
            _ => 38,
        };
        Some((n1, 60_000 - 32_768 + n, 1))
    } else if rest.len() == 2 {
        // suffisso numerico da /10 a /99 (`packjt.f90:1003-1010`; il campo supera i 15 bit e in `pack50` si perde il bit alto, come in WSJT-X)
        let n1 = pack_call(base)?;
        let b = rest.as_bytes();
        let n = 10 * (i32::from(b[0]) - 48) + i32::from(b[1]) - 48;
        Some((n1, 60_000 + 26 + n, 1))
    } else {
        if base.is_empty() || base.len() > 3 {
            return None;
        }
        let n1 = pack_call(rest)?;
        let mut pfx = [b' '; 3];
        let off = 3 - base.len();
        pfx[off..].copy_from_slice(base.as_bytes());
        let mut ng = 0i32;
        for &c in &pfx {
            ng = 37 * ng + code(c);
        }
        let mut nadd = 0;
        if ng >= 32_768 {
            ng -= 32_768;
            nadd = 1;
        }
        Some((n1, ng, nadd))
    }
}

/// `wqencode` (`wqencode.f90`) / `get_wspr_channel_symbols` (`wsprsim_utils.c:633-718`): il messaggio in 11 byte. `None` se non e' un
/// messaggio WSPR valido.
pub fn encode_message(message: &str) -> Option<[u8; 11]> {
    let msg = message.trim().to_ascii_uppercase();
    let words: Vec<&str> = msg.split_whitespace().collect();
    let power = |w: Option<&&str>| -> Option<i32> { Some(normalize_power(w?.parse::<i32>().ok()?)) };
    let first = *words.first()?;
    let has_slash = msg.contains('/');
    if first.starts_with('<') {
        // tipo 3: <K1ABC> FN42AB 37
        let end = first.find('>')?;
        let call = &first[1..end];
        if call.is_empty() {
            return None;
        }
        let grid = *words.get(1)?;
        let pw = power(words.get(2))?;
        let g = grid.as_bytes();
        if !(g.len() == 4 || g.len() == 6) || !g[0].is_ascii_uppercase() || !g[1].is_ascii_uppercase() {
            return None;
        }
        let ntype = -(pw + 1);
        let ihash = i32::from(nhash(call));
        let m = 128 * ihash + ntype + 64;
        // il locatore ruotato: il primo carattere in fondo (`wqencode.f90:51-53`)
        let rotated = format!("{}{}", &grid[1..], &grid[..1]);
        let n = pack_call(&rotated)?;
        Some(pack50(n, u32::try_from(m).ok()?))
    } else if has_slash {
        // tipo 2: PJ4/K1ABC 37
        if words.len() != 2 || first.starts_with('/') {
            return None;
        }
        let pw = power(words.get(1))?;
        let (n1, ng, nadd) = pack_prefix(first)?;
        let ntype = pw + 1 + nadd;
        let m = 128 * ng + ntype + 64;
        Some(pack50(n1, u32::try_from(m).ok()?))
    } else {
        // tipo 1: K1ABC FN42 37
        if words.len() != 3 || !(3..=6).contains(&first.len()) || words[1].len() != 4 {
            return None;
        }
        let n = pack_call(first)?;
        let pw = power(words.get(2))?;
        let m = pack_grid4_power(words[1], pw)?;
        Some(pack50(n, m))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn roundtrip(msg: &str, table: &mut HashTable) -> Unpacked {
        let data = encode_message(msg).unwrap_or_else(|| panic!("{msg}"));
        unpack(&data, table).unwrap_or_else(|| panic!("{msg}"))
    }

    #[test]
    fn type1_messages_round_trip() {
        let mut t = HashTable::new();
        for m in ["K1ABC FN42 37", "IU3ABC JN55 0", "G4ABC IO91 60", "K9AN EN50 33", "W1AW FN31 10", "3D2AB RH91 23"] {
            let u = roundtrip(m, &mut t);
            assert_eq!(u.message, m);
            assert!(!u.noprint);
        }
        // potenza non ammessa: arrotondata come in wqencode
        assert_eq!(roundtrip("IU3ABC JN55 31", &mut t).message, "IU3ABC JN55 30");
        assert_eq!(roundtrip("IU3ABC JN55 99", &mut t).message, "IU3ABC JN55 60");
        assert_eq!(t.call(nhash("IU3ABC")), Some("IU3ABC"));
        assert_eq!(t.grid(nhash("IU3ABC")), Some("JN55"));
    }

    #[test]
    fn type2_and_type3_round_trip_with_the_hash_table() {
        let mut t = HashTable::new();
        assert_eq!(roundtrip("PJ4/K1ABC 37", &mut t).message, "PJ4/K1ABC 37");
        assert_eq!(roundtrip("K1ABC/P 23", &mut t).message, "K1ABC/P 23");
        assert_eq!(roundtrip("F/IU3ABC 30", &mut t).message, "F/IU3ABC 30");
        assert_eq!(roundtrip("IU3ABC/7 10", &mut t).message, "IU3ABC/7 10");
        // il tipo 3 si legge per intero solo se l'hash e' noto
        let mut empty = HashTable::new();
        assert_eq!(roundtrip("<IU3ABC> IO91WM 37", &mut empty).message, "<...> IO91WM 37");
        let u = roundtrip("<PJ4/K1ABC> FK52UD 37", &mut t);
        assert_eq!(u.message, "<PJ4/K1ABC> FK52UD 37");
        assert_eq!(u.call, "<PJ4/K1ABC>");
        assert!(!u.noprint);
    }

    #[test]
    fn invalid_messages_are_rejected() {
        for m in ["", "K1ABC", "TOOLONGCALL FN42 37", "K1ABC ZZ99 37", "K1ABC FN42 X", "ABCDEF FN42 37", "<> FN42AB 37"] {
            assert!(encode_message(m).is_none(), "{m}");
        }
    }

    #[test]
    fn hash_table_lines_round_trip() {
        let mut t = HashTable::new();
        roundtrip("IU3ABC JN55 37", &mut t);
        let lines = t.to_lines();
        let mut u = HashTable::new();
        for l in &lines {
            assert!(u.load_line(l));
        }
        assert_eq!(u.to_lines(), lines);
    }

    #[test]
    fn calls_unpack_without_padding() {
        assert_eq!(unpack_call(pack_call("K1ABC").unwrap_or(0)).as_deref(), Some("K1ABC"));
        assert_eq!(unpack_call(pack_call("IU3ABC").unwrap_or(0)).as_deref(), Some("IU3ABC"));
        assert_eq!(unpack_call(pack_call("G4A").unwrap_or(0)).as_deref(), Some("G4A"));
        assert_eq!(unpack_call(262_177_560), None);
    }
}
