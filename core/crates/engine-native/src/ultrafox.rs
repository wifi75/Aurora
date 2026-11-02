//! Il messaggio di UltraFOX (compatibile con SuperFox di WSJT-X 3.3, `sfox_pack.f90`): fino a 5 Hound con RR73 e 4 con il rapporto in una
//! sola trasmissione, oppure un CQ, con il testo libero facoltativo, il «piu' CQ» e la firma digitale (OTP a 20 bit). Si riempiono 329 bit,
//! si leggono come 47 simboli da 7 bit, si aggiunge il controllo d'integrita' a 21 bit (`nhash2`, cioe' `hashlittle` di Bob Jenkins con
//! seme 571) in tre simboli e si rovescia l'ordine: i 50 simboli vanno al codice polare ([`crate::qpc`]).

use ft8::message::{pack28_standard, pack_text71};

/// Simboli da 7 bit del messaggio (ingresso del codice polare).
pub const SYMBOLS: usize = 50;
const BITS: usize = 329;
/// Il nominativo segnaposto nelle caselle vuote («QU1RKS»): cosi' un messaggio con poche stazioni non ha lunghe file di zeri.
const NQU1RKS: u64 = 203_514_677;
const MAX_RR73: usize = 5;
const MAX_REPORTS: usize = 4;
const MAX_HOUNDS: usize = 9;

/// Cosa trasmette la Fox in questo periodo.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Content {
    /// `CQ <nominativo> <locatore4>`: il nominativo puo' essere anche non standard (fino a 11 caratteri).
    Cq { call: String, grid: String },
    /// Le stazioni a cui si chiude (RR73) e quelle a cui si da' il rapporto (dB da -18 a +12).
    Hounds { fox: String, rr73: Vec<String>, reports: Vec<(String, i32)> },
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Extras {
    /// Il «piu' CQ» (`bMoreCQs`).
    pub more_cqs: bool,
    /// Testo libero di 26 caratteri al massimo (spazio, cifre, lettere e `+-./?`).
    pub free_text: Option<String>,
    /// La firma digitale (il numero a 6 cifre dato alla spedizione), 0 se non c'e'.
    pub otp: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PackError {
    BadCq,
    BadCall,
    BadReport,
    TooMany,
    BadFreeText,
    BadOtp,
}

struct Bits([u8; BITS]);

impl Bits {
    /// Scrive `value` su `width` bit a partire dalla posizione `from` (1 come in Fortran).
    fn put(&mut self, from: usize, width: usize, value: u128) {
        for i in 0..width {
            self.0[from - 1 + i] = ((value >> (width - 1 - i)) & 1) as u8;
        }
    }

    fn get(&self, from: usize, width: usize) -> u128 {
        (0..width).fold(0, |v, i| (v << 1) | u128::from(self.0[from - 1 + i]))
    }
}

fn call28(call: &str) -> Result<u128, PackError> {
    pack28_standard(call).map(u128::from).ok_or(PackError::BadCall)
}

fn grid15(grid: &str) -> Option<u128> {
    let g = grid.as_bytes();
    let ok = g.len() == 4 && (b'A'..=b'R').contains(&g[0]) && (b'A'..=b'R').contains(&g[1]) && g[2].is_ascii_digit() && g[3].is_ascii_digit();
    ok.then(|| u128::from((u32::from(g[0] - b'A') * 18 + u32::from(g[1] - b'A')) * 100 + u32::from(g[2] - b'0') * 10 + u32::from(g[3] - b'0')))
}

const CQ_CHARS: &[u8] = b" 0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ/";
const TEXT_CHARS: &[u8] = b" 0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ+-./?";

/// I 50 simboli da 7 bit da dare al codice polare.
pub fn pack(content: &Content, extras: &Extras) -> Result<[u8; SYMBOLS], PackError> {
    if extras.otp >= 1 << 20 {
        return Err(PackError::BadOtp);
    }
    let mut b = Bits([0; BITS]);
    b.put(307, 20, u128::from(extras.otp));
    let send_text = extras.free_text.is_some();
    let mut i3: u128 = 0;
    match content {
        Content::Cq { call, grid } => {
            let c = call.as_bytes();
            if !(3..=11).contains(&c.len()) || !c.iter().all(|x| CQ_CHARS[1..].contains(x)) {
                return Err(PackError::BadCq);
            }
            let mut n58: u128 = 0;
            for i in 0..11 {
                let ch = c.get(i).copied().unwrap_or(b' ');
                n58 = n58 * 38 + CQ_CHARS.iter().position(|&x| x == ch).unwrap_or(0) as u128;
            }
            b.put(1, 58, n58);
            b.put(59, 15, grid15(grid).ok_or(PackError::BadCq)?);
            i3 = 3;
        }
        Content::Hounds { fox, rr73, reports } => {
            b.put(1, 28, call28(fox)?);
            let too_many = if send_text { rr73.len() + reports.len() > 4 } else { rr73.len() > MAX_RR73 || reports.len() > MAX_REPORTS || rr73.len() + reports.len() > MAX_HOUNDS };
            if too_many || (send_text && reports.len() > MAX_REPORTS) {
                return Err(PackError::TooMany);
            }
            if send_text {
                b.put(141, 20, (1 << 20) - 1);
            }
            for (k, call) in rr73.iter().enumerate() {
                b.put(29 + 28 * k, 28, call28(call)?);
            }
            let (mut j, mut j2) = if send_text { (29 + 28 * rr73.len(), 141 + 5 * rr73.len()) } else { (169, 281) };
            if send_text {
                i3 = 2;
            }
            for (call, db) in reports {
                if !(-18..=12).contains(db) {
                    return Err(PackError::BadReport);
                }
                b.put(j, 28, call28(call)?);
                b.put(j2, 5, (db + 18) as u128);
                j += 28;
                j2 += 5;
            }
        }
    }
    if let Some(text) = &extras.free_text {
        let t = text.trim_end().as_bytes();
        if t.len() > 26 || !t.iter().all(|x| TEXT_CHARS.contains(x)) {
            return Err(PackError::BadFreeText);
        }
        let mut full = [b'.'; 26];
        full[..t.len()].copy_from_slice(t);
        let half = |h: &[u8]| pack_text71(std::str::from_utf8(h).unwrap_or("")).ok_or(PackError::BadFreeText);
        let (a, z) = (half(&full[..13])?, half(&full[13..])?);
        if i3 == 3 {
            b.put(74, 71, a);
            b.put(145, 71, z);
        } else {
            b.put(161, 71, a);
            b.put(232, 71, z);
        }
    }
    b.put(327, 3, i3);
    if extras.more_cqs {
        b.put(306, 1, 1);
    }
    if i3 == 0 {
        for i in 1..=9 {
            if b.get(i * 28 + 1, 28) == 0 {
                b.put(i * 28 + 1, 28, u128::from(NQU1RKS));
            }
        }
    } else if i3 == 3 && (0..7).all(|i| b.get(i * 32 + 74, 32) == 0) {
        for i in 0..7 {
            b.put(i * 32 + 74, 32, u128::from(NQU1RKS));
        }
    }
    let mut x = [0u8; SYMBOLS];
    for (k, s) in x.iter_mut().take(47).enumerate() {
        *s = b.get(1 + 7 * k, 7) as u8;
    }
    let crc = hashlittle(&x[..47], 571) & ((1 << 21) - 1);
    x[47] = (crc >> 14) as u8;
    x[48] = ((crc >> 7) & 127) as u8;
    x[49] = (crc & 127) as u8;
    x.reverse();
    Ok(x)
}

/// Il controllo d'integrita' di 50 simboli ricevuti (gia' rovesciati come escono da [`pack`]): `true` se i primi tre tornano.
pub fn crc_ok(symbols: &[u8; SYMBOLS]) -> bool {
    let mut x = *symbols;
    x.reverse();
    let crc = hashlittle(&x[..47], 571) & ((1 << 21) - 1);
    u32::from(x[47]) == crc >> 14 && u32::from(x[48]) == (crc >> 7) & 127 && u32::from(x[49]) == crc & 127
}

/// `hashlittle` di lookup3.c (Bob Jenkins, pubblico dominio), versione a byte: uguale a quella allineata sui processori little-endian.
pub fn hashlittle(key: &[u8], initval: u32) -> u32 {
    let mut a = 0xdead_beefu32.wrapping_add(key.len() as u32).wrapping_add(initval);
    let mut b = a;
    let mut c = a;
    let mut k = key;
    let word = |s: &[u8]| s.iter().enumerate().fold(0u32, |w, (i, &v)| w | (u32::from(v) << (8 * i)));
    while k.len() > 12 {
        a = a.wrapping_add(word(&k[0..4]));
        b = b.wrapping_add(word(&k[4..8]));
        c = c.wrapping_add(word(&k[8..12]));
        mix(&mut a, &mut b, &mut c);
        k = &k[12..];
    }
    if k.is_empty() {
        return c;
    }
    let mut tail = [0u8; 12];
    tail[..k.len()].copy_from_slice(k);
    a = a.wrapping_add(word(&tail[0..4]));
    b = b.wrapping_add(word(&tail[4..8]));
    c = c.wrapping_add(word(&tail[8..12]));
    finalize(&mut a, &mut b, &mut c);
    c
}

fn mix(a: &mut u32, b: &mut u32, c: &mut u32) {
    *a = a.wrapping_sub(*c); *a ^= c.rotate_left(4); *c = c.wrapping_add(*b);
    *b = b.wrapping_sub(*a); *b ^= a.rotate_left(6); *a = a.wrapping_add(*c);
    *c = c.wrapping_sub(*b); *c ^= b.rotate_left(8); *b = b.wrapping_add(*a);
    *a = a.wrapping_sub(*c); *a ^= c.rotate_left(16); *c = c.wrapping_add(*b);
    *b = b.wrapping_sub(*a); *b ^= a.rotate_left(19); *a = a.wrapping_add(*c);
    *c = c.wrapping_sub(*b); *c ^= b.rotate_left(4); *b = b.wrapping_add(*a);
}

fn finalize(a: &mut u32, b: &mut u32, c: &mut u32) {
    *c ^= *b; *c = c.wrapping_sub(b.rotate_left(14));
    *a ^= *c; *a = a.wrapping_sub(c.rotate_left(11));
    *b ^= *a; *b = b.wrapping_sub(a.rotate_left(25));
    *c ^= *b; *c = c.wrapping_sub(b.rotate_left(16));
    *a ^= *c; *a = a.wrapping_sub(c.rotate_left(4));
    *b ^= *a; *b = b.wrapping_sub(a.rotate_left(14));
    *c ^= *b; *c = c.wrapping_sub(b.rotate_left(24));
}

/// Simboli trasmessi: 127 di dati e 24 di sincronismo (tono 0) nelle posizioni di `ISYNC` (da 1, come in `sftx_sub.f90`).
pub const TONES: usize = 151;
const ISYNC: [usize; 24] = [1, 2, 4, 7, 11, 16, 22, 29, 37, 39, 42, 43, 45, 48, 52, 57, 63, 70, 78, 80, 83, 84, 86, 89];
/// Campioni per simbolo a 12 kHz: 11,72 baud, 128 toni di dati piu' quello del sincronismo (circa 1,5 kHz di banda).
pub const NSPS: usize = 1024;
/// La frequenza del tono 0: il segnale va da 750 a circa 2260 Hz, sempre li' (non segue la frequenza Tx scelta).
pub const F0_HZ: f64 = 750.0;
const BT: f64 = 8.0;

/// Dai 127 simboli del codice polare ai 151 toni (il dato 0 va sul tono 1, il tono 0 e' del sincronismo).
pub fn tones(code: &[u8; crate::qpc::NP]) -> [u8; TONES] {
    let mut out = [0u8; TONES];
    let mut data = code.iter();
    for (i, t) in out.iter_mut().enumerate() {
        if !ISYNC.contains(&(i + 1)) {
            *t = data.next().map_or(0, |&s| s + 1);
        }
    }
    out
}

/// Il segnale GFSK (BT 8, indice 1) a 12 kHz, con un simbolo vuoto prima e dopo per le rampe (`sfox_gen_gfsk.f90`): (151 + 2) x 1024
/// campioni, circa 13 s.
pub fn wave(tones: &[u8; TONES]) -> Vec<f32> {
    let n = (TONES + 2) * NSPS;
    let two_pi = std::f64::consts::TAU;
    let peak = two_pi / NSPS as f64;
    let pulse: Vec<f64> = (1..=3 * NSPS).map(|i| f64::from(ft8::modem::gfsk_pulse(BT as f32, ((i as f64 - 1.5 * NSPS as f64) / NSPS as f64) as f32))).collect();
    let mut dphi = vec![0f64; n];
    for (j, &t) in tones.iter().enumerate() {
        let base = j * NSPS;
        for (k, p) in pulse.iter().enumerate() {
            dphi[base + k] += peak * p * f64::from(t);
        }
    }
    let (first, last) = (f64::from(tones[0]), f64::from(tones[TONES - 1]));
    for k in 0..2 * NSPS {
        dphi[k] += peak * first * pulse[NSPS + k];
        dphi[TONES * NSPS + k] += peak * last * pulse[k];
    }
    let step = two_pi * F0_HZ / 12_000.0;
    let mut out = vec![0f32; n];
    let mut phi = 0f64;
    for k in 0..n - 1 {
        out[k] = phi.sin() as f32;
        phi += dphi[k + 1] + step;
    }
    let ramp = (NSPS as f64 / BT) as usize;
    for (k, s) in out.iter_mut().take(NSPS).enumerate() {
        *s *= if k < NSPS - ramp { 0.0 } else { ((1.0 - (two_pi * (k - (NSPS - ramp)) as f64 / (2.0 * ramp as f64)).cos()) / 2.0) as f32 };
    }
    let k1 = (TONES + 1) * NSPS;
    for (k, s) in out.iter_mut().skip(k1).enumerate() {
        *s *= if k < ramp { ((1.0 + (two_pi * k as f64 / (2.0 * ramp as f64)).cos()) / 2.0) as f32 } else { 0.0 };
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Valori di prova pubblicati in lookup3.c (`driver5`).
    #[test]
    fn hashlittle_matches_the_published_values() {
        assert_eq!(hashlittle(b"", 0), 0xdead_beef);
        assert_eq!(hashlittle(b"", 0xdead_beef), 0xbd5b_7dde);
        assert_eq!(hashlittle(b"Four score and seven years ago", 0), 0x1777_0551);
        assert_eq!(hashlittle(b"Four score and seven years ago", 1), 0xcd62_8161);
    }

    fn hounds() -> Content {
        Content::Hounds { fox: "IU3ABC".into(), rr73: vec!["K1ABC".into(), "DL1ABC".into()], reports: vec![("JA1XYZ".into(), -12), ("VK2AB".into(), 3)] }
    }

    #[test]
    fn hounds_pack_into_valid_7_bit_symbols_with_a_good_crc() {
        let x = pack(&hounds(), &Extras::default()).expect("impacchettato");
        assert!(x.iter().all(|&s| s < 128));
        assert!(crc_ok(&x));
        let mut bad = x;
        bad[20] ^= 1;
        assert!(!crc_ok(&bad));
    }

    #[test]
    fn the_symbols_go_through_the_polar_code() {
        let x = pack(&hounds(), &Extras { more_cqs: true, free_text: None, otp: 123_456 }).expect("impacchettato");
        let word = crate::qpc::encode(&x);
        let mut py = vec![0.0f32; crate::qpc::NP * crate::qpc::Q];
        for (i, &s) in word.iter().enumerate() {
            py[i * crate::qpc::Q + usize::from(s)] = 1.0;
        }
        let (back, _) = crate::qpc::decode(&py).expect("decodificato");
        assert_eq!(back, x);
        assert!(crc_ok(&back));
    }

    #[test]
    fn limits_and_bad_input_are_refused() {
        let many = |n: usize| Content::Hounds { fox: "IU3ABC".into(), rr73: (0..n).map(|i| format!("K{i}ABC")).collect(), reports: vec![] };
        assert!(pack(&many(5), &Extras::default()).is_ok());
        assert_eq!(pack(&many(6), &Extras::default()), Err(PackError::TooMany));
        assert_eq!(pack(&many(5), &Extras { free_text: Some("TNX".into()), ..Extras::default() }), Err(PackError::TooMany));
        let bad_report = Content::Hounds { fox: "IU3ABC".into(), rr73: vec![], reports: vec![("K1ABC".into(), 20)] };
        assert_eq!(pack(&bad_report, &Extras::default()), Err(PackError::BadReport));
        assert_eq!(pack(&Content::Cq { call: "IU3ABC".into(), grid: "XX99".into() }, &Extras::default()), Err(PackError::BadCq));
        assert_eq!(pack(&hounds(), &Extras { otp: 2_000_000, ..Extras::default() }), Err(PackError::BadOtp));
    }

    #[test]
    fn cq_and_free_text_pack() {
        let cq = Content::Cq { call: "IU3ABC".into(), grid: "IO91".into() };
        let x = pack(&cq, &Extras { more_cqs: false, free_text: Some("QSL VIA LOTW".into()), otp: 0 }).expect("CQ con testo");
        assert!(crc_ok(&x));
        assert_eq!(pack(&cq, &Extras { free_text: Some("minuscole".into()), ..Extras::default() }), Err(PackError::BadFreeText));
    }

    /// Energia di una frequenza in un tratto (Goertzel).
    fn power(x: &[f32], hz: f64) -> f64 {
        let w = std::f64::consts::TAU * hz / 12_000.0;
        let (mut s1, mut s2) = (0f64, 0f64);
        for &v in x {
            let s = f64::from(v) + 2.0 * w.cos() * s1 - s2;
            s2 = s1;
            s1 = s;
        }
        s1 * s1 + s2 * s2 - 2.0 * w.cos() * s1 * s2
    }

    #[test]
    fn tones_put_sync_on_zero_and_data_one_above() {
        let x = pack(&hounds(), &Extras::default()).expect("impacchettato");
        let code = crate::qpc::encode(&x);
        let t = tones(&code);
        for &i in &ISYNC {
            assert_eq!(t[i - 1], 0);
        }
        let data: Vec<u8> = t.iter().enumerate().filter(|(i, _)| !ISYNC.contains(&(i + 1))).map(|(_, &v)| v - 1).collect();
        assert_eq!(data, code.to_vec());
    }

    #[test]
    fn the_wave_lasts_153_symbols_and_each_symbol_sits_on_its_tone() {
        let x = pack(&hounds(), &Extras::default()).expect("impacchettato");
        let t = tones(&crate::qpc::encode(&x));
        let w = wave(&t);
        assert_eq!(w.len(), (TONES + 2) * NSPS);
        assert!(w.iter().all(|v| v.abs() <= 1.0));
        assert!(w[..NSPS / 2].iter().all(|&v| v == 0.0), "silenzio prima della rampa");
        let baud = 12_000.0 / NSPS as f64;
        for j in [5usize, 40, 100, 140] {
            // centro del simbolo j (il primo simbolo vero e' il secondo blocco)
            let start = (j + 1) * NSPS + NSPS / 4;
            let seg = &w[start..start + NSPS / 2];
            let right = power(seg, F0_HZ + baud * f64::from(t[j]));
            let other = power(seg, F0_HZ + baud * f64::from((t[j] + 40) % 129));
            assert!(right > 10.0 * other, "simbolo {j}");
        }
    }
}
