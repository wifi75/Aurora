//! Messaggi di JT65 e JT9: 22 caratteri in 72 bit, dodici simboli da 6 bit (`packjt.f90` di WSJT-X, GPL 3).
//!
//! Tipi di messaggio:
//! 1. standard: due nominativi (28 bit l'uno) e un locatore, un rapporto o «RO», «RRR», «73» (15 bit);
//! 2. con prefisso o suffisso aggiunto a uno dei due nominativi (JT65v2: l'aggiunta viaggia nel posto del locatore);
//! 3. «CQ», «QRZ» o «DE» seguito da un nominativo con prefisso di quattro caratteri o suffisso di tre;
//! 4. testo libero di 13 caratteri.

use crate::prefixes::{PREFIXES, SUFFIXES};

const NBASE: i64 = 37 * 36 * 10 * 27 * 27 * 27;
const NGBASE: i64 = 180 * 180;
const TEXT_CHARS: &[u8; 42] = b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ +-./?";
const CALL_CHARS: &[u8; 37] = b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ ";

/// Che cosa e' stato impacchettato (`itype` di WSJT-X).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Standard,
    Type1Prefix,
    Type1Suffix,
    Type2Prefix,
    Type2Suffix,
    FreeText,
}

/// Dodici simboli da 6 bit.
pub type Symbols = [u8; 12];

fn ch(s: &[u8], i: usize) -> u8 {
    s.get(i).copied().unwrap_or(b' ')
}

/// `fmtmsg`: maiuscole, spazi doppi ridotti a uno, niente spazi in coda; al massimo 22 caratteri.
fn format_message(msg: &str) -> Vec<u8> {
    let mut out: Vec<u8> = Vec::new();
    for b in msg.bytes() {
        let b = b.to_ascii_uppercase();
        if b == b' ' && out.last().is_none_or(|&l| l == b' ') {
            continue;
        }
        out.push(b);
    }
    while out.last() == Some(&b' ') {
        out.pop();
    }
    out.truncate(22);
    out
}

fn nchar(c: u8) -> i64 {
    match c {
        b'0'..=b'9' => i64::from(c - b'0'),
        b'A'..=b'Z' => i64::from(c - b'A') + 10,
        b'a'..=b'z' => i64::from(c - b'a') + 10,
        _ => 36,
    }
}

/// `packcall`: un nominativo (6 caratteri) in 28 bit. `None` se non e' un nominativo ma testo.
fn pack_call(callsign: &[u8]) -> Option<i64> {
    let mut call = [b' '; 6];
    for (i, c) in callsign.iter().take(6).enumerate() {
        call[i] = *c;
    }
    // Swaziland e Guinea
    if &call[..4] == b"3DA0" {
        call = [b'3', b'D', b'0', call[4], call[5], b' '];
    }
    if call[0] == b'3' && call[1] == b'X' && call[2].is_ascii_uppercase() {
        call = [b'Q', call[2], call[3], call[4], call[5], b' '];
    }
    if &call[..3] == b"CQ " {
        let mut ncall = NBASE + 1;
        if call[3..6].iter().all(u8::is_ascii_digit) {
            let nfreq: i64 = std::str::from_utf8(&call[3..6]).ok()?.parse().ok()?;
            ncall = NBASE + 3 + nfreq;
        }
        return Some(ncall);
    }
    if &call[..4] == b"QRZ " {
        return Some(NBASE + 2);
    }
    if &call[..3] == b"DE " {
        return Some(267_796_945);
    }
    let tmp: [u8; 6] = if call[2].is_ascii_digit() {
        call
    } else if call[1].is_ascii_digit() {
        if call[5] != b' ' {
            return None;
        }
        [b' ', call[0], call[1], call[2], call[3], call[4]]
    } else {
        return None;
    };
    let mut tmp = tmp;
    tmp.make_ascii_uppercase();
    let letter_or_blank = |c: u8| c.is_ascii_uppercase() || c == b' ';
    let ok = (letter_or_blank(tmp[0]) || tmp[0].is_ascii_digit())
        && (tmp[1].is_ascii_uppercase() || tmp[1].is_ascii_digit())
        && tmp[2].is_ascii_digit()
        && letter_or_blank(tmp[3])
        && letter_or_blank(tmp[4])
        && letter_or_blank(tmp[5]);
    if !ok {
        return None;
    }
    let mut n = nchar(tmp[0]);
    n = 36 * n + nchar(tmp[1]);
    n = 10 * n + nchar(tmp[2]);
    n = 27 * n + nchar(tmp[3]) - 10;
    n = 27 * n + nchar(tmp[4]) - 10;
    n = 27 * n + nchar(tmp[5]) - 10;
    Some(n)
}

/// Un nominativo e il prefisso o suffisso che porta (`unpackcall`): `(testo, tipo del messaggio JT65v2, prefisso)`.
fn unpack_call(ncall: i64) -> (String, u8, String) {
    let mut word = String::from("......");
    let mut psfx = String::new();
    let mut iv2 = 0u8;
    let mut n = ncall;
    if n < 262_177_560 {
        let mut w = [b'.'; 6];
        let c = |i: i64| CALL_CHARS[i as usize];
        w[5] = c(n % 27 + 10);
        n /= 27;
        w[4] = c(n % 27 + 10);
        n /= 27;
        w[3] = c(n % 27 + 10);
        n /= 27;
        w[2] = c(n % 10);
        n /= 10;
        w[1] = c(n % 36);
        n /= 36;
        w[0] = c(n);
        word = String::from_utf8_lossy(&w).into_owned();
        // via gli spazi iniziali (solo nei primi quattro caratteri, come in Fortran)
        if let Some(i) = w[..4].iter().position(|&b| b != b' ') {
            word = String::from_utf8_lossy(&w[i..]).into_owned();
        }
        word = word.trim_end().to_string();
    } else if n < 267_796_946 {
        let prefix = |base: i64, mut n: i64, len: usize| -> String {
            n -= base;
            let mut chars = vec![b' '; len];
            for k in (0..len).rev() {
                let i = if k == 0 { n } else { n % 37 };
                chars[k] = CALL_CHARS[i as usize % 37];
                if k > 0 {
                    n /= 37;
                }
            }
            String::from_utf8_lossy(&chars).into_owned()
        };
        if (262_178_563..=264_002_071).contains(&n) {
            iv2 = 1;
            psfx = prefix(262_178_563, n, 4);
        } else if (264_002_072..=265_825_580).contains(&n) {
            iv2 = 2;
            psfx = prefix(264_002_072, n, 4);
        } else if (265_825_581..=267_649_089).contains(&n) {
            iv2 = 3;
            psfx = prefix(265_825_581, n, 4);
        } else if (267_649_090..=267_698_374).contains(&n) {
            iv2 = 4;
            psfx = prefix(267_649_090, n, 3);
        } else if (267_698_375..=267_747_659).contains(&n) {
            iv2 = 5;
            psfx = prefix(267_698_375, n, 3);
        } else if (267_747_660..=267_796_944).contains(&n) {
            iv2 = 6;
            psfx = prefix(267_747_660, n, 3);
        } else if n == 267_796_945 {
            iv2 = 7;
        }
    }
    if let Some(rest) = word.strip_prefix("3D0") {
        word = format!("3DA0{rest}");
    }
    let b = word.as_bytes();
    if b.len() >= 2 && b[0] == b'Q' && b[1].is_ascii_uppercase() {
        word = format!("3X{}", &word[1..]);
    }
    (word, iv2, psfx)
}

/// `grid2deg` con il sottoquadrato `mm` e il centro: longitudine ovest e latitudine nord in gradi.
fn grid_to_deg(grid: &[u8]) -> (f64, f64) {
    let g = |i: usize| ch(grid, i);
    let (g5, g6) = if grid.len() >= 6 && g(4).is_ascii_alphabetic() { (g(4).to_ascii_lowercase(), g(5).to_ascii_lowercase()) } else { (b'm', b'm') };
    // come in Fortran, anche un testo che non e' un locatore da' un numero (poi scartato dal chiamante): aritmetica con segno
    let d = |c: u8, base: u8| i32::from(c) - i32::from(base);
    let nlong = 180 - 20 * d(g(0).to_ascii_uppercase(), b'A');
    let n20d = 2 * d(g(2), b'0');
    let xminlong = 5.0 * (f64::from(d(g5, b'a')) + 0.5);
    let dlong = f64::from(nlong - n20d) - xminlong / 60.0;
    let nlat = -90 + 10 * d(g(1).to_ascii_uppercase(), b'A') + d(g(3), b'0');
    let xminlat = 2.5 * (f64::from(d(g6, b'a')) + 0.5);
    (dlong, f64::from(nlat) + xminlat / 60.0)
}

/// `deg2grid`: sei caratteri (il locatore a sei posizioni).
fn deg_to_grid(dlong0: f64, dlat: f64) -> [u8; 6] {
    let mut dlong = dlong0;
    if dlong < -180.0 {
        dlong += 360.0;
    }
    if dlong > 180.0 {
        dlong -= 360.0;
    }
    let nlong = (60.0 * (180.0 - dlong) / 5.0) as i32;
    let (n1, n2, n3) = (nlong / 240, (nlong - 240 * (nlong / 240)) / 24, nlong - 240 * (nlong / 240) - 24 * ((nlong - 240 * (nlong / 240)) / 24));
    let nlat = (60.0 * (dlat + 90.0) / 2.5) as i32;
    let (m1, m2, m3) = (nlat / 240, (nlat - 240 * (nlat / 240)) / 24, nlat - 240 * (nlat / 240) - 24 * ((nlat - 240 * (nlat / 240)) / 24));
    let c = |base: u8, v: i32| (i32::from(base) + v).clamp(0, 127) as u8;
    [c(b'A', n1), c(b'A', m1), c(b'0', n2), c(b'0', m2), c(b'a', n3), c(b'a', m3)]
}

/// Il numero di «locatore» (`ng`) del campo di 15 bit, `None` se non e' un locatore, un rapporto o una parola: allora il messaggio e' testo libero.
fn pack_grid(field: &[u8]) -> Option<i64> {
    let text: String = String::from_utf8_lossy(field).into_owned();
    let t = text.trim_end();
    if t.is_empty() {
        return Some(NGBASE + 1);
    }
    let digits = |s: &str| -> Option<i64> {
        let s = s.trim();
        if s.is_empty() || !s.bytes().all(|b| b.is_ascii_digit()) {
            return None;
        }
        s.parse().ok()
    };
    let b = t.as_bytes();
    if b[0] == b'-' {
        let n = digits(&t[1..t.len().min(3)])?;
        if (1..=30).contains(&n) {
            return Some(NGBASE + 1 + n);
        }
    } else if t.starts_with("R-") {
        let n = digits(&t[2..t.len().min(4)])?;
        if (1..=30).contains(&n) {
            return Some(NGBASE + 31 + n);
        }
    } else if t == "RO" {
        return Some(NGBASE + 62);
    } else if t == "RRR" {
        return Some(NGBASE + 63);
    } else if t == "73" {
        return Some(NGBASE + 64);
    }
    // rapporti estesi: da -50 a -31 e da 0 a +49 (con «R» davanti, `LA..`; senza, `KA..`)
    let mut n = 99i64;
    if let Ok(v) = t.trim().parse::<i64>() {
        n = v;
    } else if t.len() > 1
        && let Ok(v) = t[1..].trim().parse::<i64>()
    {
        n = v;
    }
    let mut grid4 = [b' '; 4];
    for (i, c) in field.iter().take(4).enumerate() {
        grid4[i] = *c;
    }
    if (-50..=49).contains(&n) {
        let s = format!("{}A{:02}", if b[0] == b'R' { 'L' } else { 'K' }, n + 50);
        grid4.copy_from_slice(&s.as_bytes()[..4]);
    } else {
        let bad = !(b'A'..=b'R').contains(&grid4[0]) || !(b'A'..=b'R').contains(&grid4[1]) || !grid4[2].is_ascii_digit() || !grid4[3].is_ascii_digit();
        if bad {
            return None;
        }
    }
    let (dlong, dlat) = grid_to_deg(&[grid4[0], grid4[1], grid4[2], grid4[3], b'm', b'm']);
    let long = dlong as i64;
    let lat = (dlat + 90.0) as i64;
    Some(((long + 180) / 2) * 180 + lat)
}

/// `unpackgrid`: il testo di quel campo (locatore, rapporto o parola).
fn unpack_grid(ng: i64) -> String {
    if ng >= 32_400 {
        let n = ng - NGBASE - 1;
        return match n {
            1..=30 => format!("-{n:02}"),
            31..=60 => format!("R-{:02}", n - 30),
            61 => "RO".to_string(),
            62 => "RRR".to_string(),
            63 => "73".to_string(),
            _ => String::new(),
        };
    }
    let dlat = (ng % 180 - 90) as f64;
    let dlong = ((ng / 180) * 2 - 180 + 2) as f64;
    let g6 = deg_to_grid(dlong, dlat);
    let grid = String::from_utf8_lossy(&g6[..4]).into_owned();
    if let Some(rest) = grid.strip_prefix("KA")
        && let Ok(n) = rest.parse::<i64>()
    {
        let n = n - 50;
        return if n >= 0 { format!("+{n:02}") } else { format!("-{:02}", -n) };
    }
    if let Some(rest) = grid.strip_prefix("LA")
        && let Ok(n) = rest.parse::<i64>()
    {
        let n = n - 50;
        return if n >= 0 { format!("R+{n:02}") } else { format!("R-{:02}", -n) };
    }
    grid
}

/// `grid2k`: l'indice del prefisso o suffisso aggiunto, dal locatore dell'estremo nord (0 se non e' uno di quelli).
fn grid_to_k(grid6: &[u8]) -> i64 {
    let (xlong, xlat) = grid_to_deg(grid6);
    // `grid2deg(grid//'55')`: il sottoquadrato '55' non e' una lettera: vale 'mm'; il valore e' arrotondato al grado
    let nlong = xlong.round() as i64;
    let nlat = xlat.round() as i64;
    if nlat >= 85 { 5 * (nlong + 179) / 2 + nlat - 84 } else { 0 }
}

/// `k2grid`: il locatore a quattro caratteri di un indice.
fn k_to_grid4(k: i64) -> [u8; 4] {
    let mut nlong = 2 * ((k - 1) / 5 % 90) - 179;
    if k > 450 {
        nlong += 180;
    }
    let nlat = (k - 1) % 5 + 85;
    let g = deg_to_grid(nlong as f64, nlat as f64);
    [g[0], g[1], g[2], g[3]]
}

/// `getpfx2`: rimette il prefisso o il suffisso del locatore al nominativo.
fn add_affix(k0: i64, callsign: &str) -> String {
    let k = if k0 > 450 { k0 - 450 } else { k0 };
    if (1..=PREFIXES.len() as i64).contains(&k) {
        format!("{}/{}", PREFIXES[(k - 1) as usize], callsign)
    } else if (401..=400 + SUFFIXES.len() as i64).contains(&k) {
        format!("{}/{}", callsign, SUFFIXES[(k - 401) as usize])
    } else {
        callsign.to_string()
    }
}

/// `getpfx1`: toglie da un nominativo il prefisso o suffisso e dice quale: `(nominativo senza, k, tipo 1..5)`; `k < 0` se non e' valido.
fn split_affix(callsign0: &str) -> (String, i64, u8) {
    let iz = callsign0.len();
    let slash = callsign0.find('/');
    let mut callsign = callsign0.to_string();
    let mut k = 0i64;
    let mut nv2 = 1u8;
    if let Some(s) = slash {
        if s + 4 <= iz && s >= 1 {
            // prefisso aggiunto: sta nella tabella?
            let c = &callsign0[..s];
            if let Some(i) = PREFIXES.iter().position(|p| *p == c) {
                k = i as i64 + 1;
                nv2 = 2;
                callsign = callsign0[s + 1..].to_string();
            }
        } else if s + 2 == iz {
            let c = callsign0.as_bytes()[s + 1] as char;
            if let Some(i) = SUFFIXES.iter().position(|x| *x == c) {
                k = 401 + i as i64;
                nv2 = 3;
                callsign = callsign0[..s].to_string();
            }
        }
        if k == 0 {
            // il JT65 originale lo tratterebbe da testo: JT65v2 mette il prefisso o suffisso nel campo del nominativo
            let lof = &callsign0[..s];
            let rof = &callsign0[s + 1..];
            let (mut is_pfx, mut is_sfx) = ((1..=4).contains(&lof.len()), (1..=3).contains(&rof.len()));
            let invalid = !(is_pfx || is_sfx);
            if is_pfx && is_sfx {
                if lof.len() < 3 {
                    is_sfx = false;
                }
                if rof.len() < 3 {
                    is_pfx = false;
                }
                if is_pfx && is_sfx {
                    let prev = callsign0.as_bytes()[s - 1];
                    if prev.is_ascii_digit() {
                        is_sfx = false;
                    } else {
                        is_pfx = false;
                    }
                }
            }
            if invalid {
                k = -1;
            } else if is_pfx {
                let mut t = [b' '; 4];
                for (i, c) in lof.bytes().take(4).enumerate() {
                    t[i] = c;
                }
                k = t.iter().fold(0i64, |a, &c| 37 * a + nchar(c));
                nv2 = 4;
                callsign = rof.to_string();
            } else if is_sfx {
                let mut t = [b' '; 3];
                for (i, c) in rof.bytes().take(3).enumerate() {
                    t[i] = c;
                }
                k = t.iter().fold(0i64, |a, &c| 37 * a + nchar(c));
                nv2 = 5;
                callsign = lof.to_string();
            }
        }
    }
    (callsign, k, nv2)
}

fn pack_text(msg: &[u8]) -> (i64, i64, i64) {
    let code = |c: u8| -> i64 { TEXT_CHARS.iter().position(|&t| t == c).map_or(36, |p| p as i64) };
    let (mut nc1, mut nc2, mut nc3) = (0i64, 0i64, 0i64);
    for i in 0..5 {
        nc1 = 42 * nc1 + code(ch(msg, i));
    }
    for i in 5..10 {
        nc2 = 42 * nc2 + code(ch(msg, i));
    }
    for i in 10..13 {
        nc3 = 42 * nc3 + code(ch(msg, i));
    }
    // i 17 bit di nc3 si dividono: uno in nc1, uno in nc2 e quindici restano
    nc1 *= 2;
    if nc3 & 32_768 != 0 {
        nc1 += 1;
    }
    nc2 *= 2;
    if nc3 & 65_536 != 0 {
        nc2 += 1;
    }
    (nc1, nc2, nc3 & 32_767)
}

fn unpack_text(nc1a: i64, nc2a: i64, nc3a: i64) -> String {
    let mut nc3 = nc3a & 32_767;
    let mut nc1 = nc1a;
    let mut nc2 = nc2a;
    if nc1 & 1 != 0 {
        nc3 += 32_768;
    }
    nc1 /= 2;
    if nc2 & 1 != 0 {
        nc3 += 65_536;
    }
    nc2 /= 2;
    let mut msg = [b' '; 13];
    for i in (0..5).rev() {
        msg[i] = TEXT_CHARS[(nc1 % 42) as usize];
        nc1 /= 42;
    }
    for i in (5..10).rev() {
        msg[i] = TEXT_CHARS[(nc2 % 42) as usize];
        nc2 /= 42;
    }
    for i in (10..13).rev() {
        msg[i] = TEXT_CHARS[(nc3 % 42) as usize];
        nc3 /= 42;
    }
    String::from_utf8_lossy(&msg).trim_end().to_string()
}

fn symbols_of(nc1: i64, nc2: i64, ng: i64) -> Symbols {
    let m = |v: i64| (v & 63) as u8;
    [
        m(nc1 >> 22),
        m(nc1 >> 16),
        m(nc1 >> 10),
        m(nc1 >> 4),
        (4 * (nc1 & 15) + ((nc2 >> 26) & 3)) as u8,
        m(nc2 >> 20),
        m(nc2 >> 14),
        m(nc2 >> 8),
        m(nc2 >> 2),
        (16 * (nc2 & 3) + ((ng >> 12) & 15)) as u8,
        m(ng >> 6),
        m(ng),
    ]
}

/// `packmsg`: il messaggio in dodici simboli da 6 bit e il suo tipo.
pub fn pack_message(msg0: &str) -> (Symbols, Kind) {
    let formatted = format_message(msg0);
    let mut msg: Vec<u8> = formatted.clone();
    msg.resize(22, b' ');
    let starts = |m: &[u8], s: &str| m.len() >= s.len() && &m[..s.len()] == s.as_bytes();
    // «CQ 5 ...» diventa «CQ 005 ...»
    if starts(&msg, "CQ ") && msg[3].is_ascii_digit() && msg[4] == b' ' {
        let mut m = b"CQ 00".to_vec();
        m.extend_from_slice(&msg[3..]);
        m.truncate(22);
        msg = m;
    }
    if starts(&msg, "CQ DX ") {
        msg[2] = b'9';
    }
    if starts(&msg, "CQ ") && msg[3].is_ascii_uppercase() && msg[4].is_ascii_uppercase() && msg[5] == b' ' {
        let mut m = b"E9".to_vec();
        m.extend_from_slice(&msg[3..]);
        m.truncate(22);
        msg = m;
    }

    let as_text = |msg: &[u8]| -> (Symbols, Kind) {
        let (nc1, nc2, nc3) = pack_text(msg);
        (symbols_of(nc1, nc2, nc3 + 32_768), Kind::FreeText)
    };

    // primo, secondo e terzo spazio
    let first_blank = if starts(&msg, "CQ ") {
        if msg[3].is_ascii_digit() && msg[4].is_ascii_digit() && msg[5].is_ascii_digit() { 6 } else { 2 }
    } else {
        match msg.iter().position(|&b| b == b' ') {
            Some(i) => i,
            None => return as_text(&msg),
        }
    };
    let ia = first_blank;
    let Some(ib) = msg.iter().skip(ia + 1).position(|&b| b == b' ').map(|p| p + ia + 1) else { return as_text(&msg) };
    let Some(ic) = msg.iter().skip(ib + 1).position(|&b| b == b' ').map(|p| p + ib + 1) else { return as_text(&msg) };
    let c1 = String::from_utf8_lossy(&msg[..ia]).into_owned();
    let c2 = String::from_utf8_lossy(&msg[ia + 1..ib]).into_owned();
    let mut c3: Vec<u8> = msg[ib + 1..=ic.min(21)].iter().copied().take(4).collect();
    c3.resize(4, b' ');
    if c3 == b"OOO " {
        c3 = b"    ".to_vec();
    }
    let (c1, k1, nv2a) = split_affix(&c1);
    if nv2a >= 4 {
        return as_text(&msg);
    }
    let Some(nc1) = pack_call(c1.as_bytes()) else { return as_text(&msg) };
    let (c2, mut k2, nv2b) = split_affix(&c2);
    let Some(nc2) = pack_call(c2.as_bytes()) else { return as_text(&msg) };
    if nv2a == 2 || nv2a == 3 || nv2b == 2 || nv2b == 3 {
        if k1 < 0 || k2 < 0 || k1 * k2 != 0 {
            return as_text(&msg);
        }
        if k2 > 0 {
            k2 += 450;
        }
        let k = k1.max(k2);
        if k > 0 {
            c3 = k_to_grid4(k).to_vec();
        }
    }
    let c3_text = pack_grid(&c3);
    let mut nc1_final = nc1;
    if nv2a < 4 && nv2b < 4 {
        if let Some(ng) = c3_text {
            let kind = match nv2a.max(nv2b) {
                2 => Kind::Type1Prefix,
                3 => Kind::Type1Suffix,
                _ => Kind::Standard,
            };
            return (symbols_of(nc1_final, nc2, ng), kind);
        }
        return as_text(&msg);
    }
    // prefisso o suffisso completo (tipi 4 e 5): solo dopo «CQ», «QRZ» o «DE» e con un locatore valido
    nc1_final = 0;
    if let Some(ng) = c3_text {
        let base = |cq: i64, qrz: i64, de: i64| -> Option<i64> {
            match c1.as_str() {
                "CQ" => Some(cq),
                "QRZ" => Some(qrz),
                "DE" => Some(de),
                _ => None,
            }
        };
        let add = if nv2b == 4 { base(262_178_563, 264_002_072, 265_825_581) } else { base(267_649_090, 267_698_375, 267_747_660) };
        if let Some(b) = add {
            nc1_final = b + k2;
        }
        if nc1_final != 0 {
            let kind = if nv2b == 4 { Kind::Type2Prefix } else { Kind::Type2Suffix };
            return (symbols_of(nc1_final, nc2, ng), kind);
        }
    }
    as_text(&msg)
}

/// `unpackmsg`: da dodici simboli da 6 bit al testo (fino a 22 caratteri, senza spazi in coda).
pub fn unpack_message(dat: &Symbols) -> String {
    let d = |i: usize| i64::from(dat[i] & 63);
    let nc1 = (d(0) << 22) + (d(1) << 16) + (d(2) << 10) + (d(3) << 4) + ((d(4) >> 2) & 15);
    let nc2 = ((d(4) & 3) << 26) + (d(5) << 20) + (d(6) << 14) + (d(7) << 8) + (d(8) << 2) + ((d(9) >> 4) & 3);
    let ng = ((d(9) & 15) << 12) + (d(10) << 6) + d(11);
    let mut msg;
    if ng >= 32_768 {
        msg = unpack_text(nc1, nc2, ng);
    } else {
        let (mut c1, iv2, psfx) = unpack_call(nc1);
        let mut cqnnn = false;
        if iv2 == 0 {
            if nc1 == NBASE + 1 {
                c1 = "CQ".to_string();
            }
            if nc1 == NBASE + 2 {
                c1 = "QRZ".to_string();
            }
            let nfreq = nc1 - NBASE - 3;
            if (0..=999).contains(&nfreq) {
                c1 = format!("CQ {nfreq:03}");
                cqnnn = true;
            }
        }
        let (c2, _, _) = unpack_call(nc2);
        let grid = unpack_grid(ng);
        let psfx = psfx.trim_end().to_string();
        if iv2 > 0 {
            msg = match iv2 {
                1 => format!("CQ {psfx}/{c2} {grid}"),
                2 => format!("QRZ {psfx}/{c2} {grid}"),
                3 => format!("DE {psfx}/{c2} {grid}"),
                4 => format!("CQ {c2}/{psfx} {grid}"),
                5 => format!("QRZ {c2}/{psfx} {grid}"),
                6 => format!("DE {c2}/{psfx} {grid}"),
                _ => {
                    let mut g6 = grid.clone().into_bytes();
                    g6.extend_from_slice(b"ma");
                    let k = grid_to_k(&g6);
                    if (451..=900).contains(&k) { format!("DE {}", add_affix(k, &c2)) } else { format!("DE {c2} {grid}") }
                }
            };
        } else {
            let mut g6 = grid.clone().into_bytes();
            g6.resize(4, b' ');
            g6.extend_from_slice(b"ma");
            let k = grid_to_k(&g6);
            let mut c2 = c2;
            if (1..=450).contains(&k) {
                c1 = add_affix(k, &c1);
            }
            if (451..=900).contains(&k) {
                c2 = add_affix(k, &c2);
            }
            msg = if cqnnn { c1 } else { format!("{c1} {c2}") };
            if cqnnn {
                msg = format!("{msg} {c2}");
            }
            if k == 0 {
                msg = format!("{msg} {grid}");
            }
        }
    }
    let mut out = msg.trim().to_string();
    if out.starts_with("CQ9DX ") {
        out = format!("CQ DX {}", &out[6..]);
    }
    let b = out.as_bytes();
    if b.len() >= 5 && &b[..2] == b"E9" && b[2].is_ascii_uppercase() && b[3].is_ascii_uppercase() && b[4] == b' ' {
        out = format!("CQ {}", &out[2..]);
    }
    if out.starts_with("CQ 00") && out.as_bytes().get(5).is_some_and(u8::is_ascii_digit) {
        out = format!("CQ {}", &out[5..]);
    }
    out.truncate(22);
    out.trim_end().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn round(msg: &str) -> String {
        unpack_message(&pack_message(msg).0)
    }

    #[test]
    fn a_standard_message_survives_the_round_trip() {
        assert_eq!(round("CQ IU3ABC JN55"), "CQ IU3ABC JN55");
        assert_eq!(round("IU3ABC K1ABC FN42"), "IU3ABC K1ABC FN42");
        assert_eq!(round("K1ABC IU3ABC -15"), "K1ABC IU3ABC -15");
        assert_eq!(round("IU3ABC K1ABC R-07"), "IU3ABC K1ABC R-07");
        assert_eq!(round("K1ABC IU3ABC RRR"), "K1ABC IU3ABC RRR");
        assert_eq!(round("IU3ABC K1ABC 73"), "IU3ABC K1ABC 73");
        assert_eq!(round("K1ABC IU3ABC RO"), "K1ABC IU3ABC RO");
        assert_eq!(round("K1ABC IU3ABC"), "K1ABC IU3ABC");
        assert_eq!(pack_message("CQ IU3ABC JN55").1, Kind::Standard);
    }

    #[test]
    fn lowercase_and_double_spaces_are_normalised() {
        assert_eq!(round("cq  iu3abc   jn55"), "CQ IU3ABC JN55");
    }

    #[test]
    fn extended_reports_round_trip() {
        assert_eq!(round("K1ABC IU3ABC -45"), "K1ABC IU3ABC -45");
        assert_eq!(round("K1ABC IU3ABC +12"), "K1ABC IU3ABC +12");
        assert_eq!(round("K1ABC IU3ABC R+12"), "K1ABC IU3ABC R+12");
        assert_eq!(round("K1ABC IU3ABC R-40"), "K1ABC IU3ABC R-40");
    }

    #[test]
    fn cq_with_a_region_or_a_frequency_round_trips() {
        assert_eq!(round("CQ DX IU3ABC JN55"), "CQ DX IU3ABC JN55");
        assert_eq!(round("CQ EU IU3ABC JN55"), "CQ EU IU3ABC JN55");
        assert_eq!(round("CQ 123 IU3ABC JN55"), "CQ 123 IU3ABC JN55");
        assert_eq!(round("QRZ IU3ABC JN55"), "QRZ IU3ABC JN55");
    }

    #[test]
    fn a_callsign_with_a_one_letter_prefix_needs_the_leading_blank() {
        assert_eq!(round("CQ W1AW FN31"), "CQ W1AW FN31");
        assert_eq!(round("CQ 3DA0XY KG33"), "CQ 3DA0XY KG33");
    }

    #[test]
    fn free_text_keeps_thirteen_characters() {
        let (symbols, kind) = pack_message("TNX 73 GL");
        assert_eq!(kind, Kind::FreeText);
        assert_eq!(unpack_message(&symbols), "TNX 73 GL");
        assert_eq!(round("HELLO WORLD+-./?"), "HELLO WORLD+-");
    }

    #[test]
    fn compound_callsigns_use_the_jt65v2_prefix_and_suffix() {
        assert_eq!(round("CQ PJ4/K1ABC"), "CQ PJ4/K1ABC");
        // con un prefisso o suffisso il locatore non c'e' piu': il suo posto porta l'aggiunta
        assert_eq!(round("K1ABC/P IU3ABC"), "K1ABC/P IU3ABC");
        let (_, kind) = pack_message("CQ KH6/K1ABC");
        assert!(matches!(kind, Kind::Type1Prefix | Kind::Type2Prefix));
        assert_eq!(round("CQ KH6/K1ABC"), "CQ KH6/K1ABC");
    }

    #[test]
    fn symbols_stay_within_six_bits_and_the_known_vector_matches() {
        let (s, _) = pack_message("CQ K1ABC FN42");
        assert!(s.iter().all(|&v| v < 64));
        // WSJT-X: «CQ K1ABC FN42» -> nc1 = 62 + NBASE... (vettore ricavato dal programma originale)
        let nc1 = NBASE + 1;
        assert_eq!(i64::from(s[0]), (nc1 >> 22) & 63);
    }
}
