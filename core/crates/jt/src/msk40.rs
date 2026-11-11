//! MSK40: i messaggi brevi di MSK144 («<IU3ABC K1ABC> R+03»), 40 bit in 20 ms: parola di sincronismo rovesciata e codice (32,16) su
//! un hash a 12 bit dei due nominativi piu' 4 bit di rapporto. Dai sorgenti di WSJT-X (GPL 3): `genmsk40.f90`, `encode_msk40.f90`,
//! `bpdecode40.f90`, `msk40decodeframe.f90`, `msk40spd.f90`, `msk40sync.f90`, `msk40_freq_search.f90`, `update_msk40_hasharray.f90`, `hash.f90`.
//! La ricezione (`spd40`) la chiama `mskrtd` ([`crate::msk144::Rtd`]) quando il blocco non ha dato un messaggio intero e «Sh» o «SWL» sono accesi.

use rustfft::{Fft, num_complex::Complex};

use crate::msk144::{BAUD, MAXRECENT, NAVPATTERNS, candidates, cshift, dithers, half_sine, platanh, sync_search};

const FS: f64 = 12_000.0;
/// Campioni del quadro breve (40 bit da 6 campioni).
pub const NSPM40: usize = 240;
/// Parola di sincronismo: quella di MSK144 al contrario.
const S8R: [i8; 8] = [1, 0, 1, 1, 0, 0, 0, 1];
/// I rapporti del messaggio breve (indice 0-15).
pub const RPT40: [&str; 16] = ["-03", "+00", "+03", "+06", "+10", "+13", "+16", "R-03", "R+00", "R+03", "R+06", "R+10", "R+13", "R+16", "RRR", "73"];
const GEN40: [u16; 16] = [0x4428, 0x5a6b, 0x1b04, 0x2c12, 0x60c4, 0x1071, 0xbe6a, 0x36dd, 0xc580, 0xad9a, 0xeca2, 0x7843, 0x332e, 0xa685, 0x5906, 0x1efe];
const COLORDER40: [u8; 32] = [4, 1, 2, 3, 0, 8, 6, 10, 13, 28, 20, 23, 17, 15, 27, 25, 16, 12, 18, 19, 7, 21, 22, 11, 24, 5, 26, 14, 9, 29, 30, 31];
const MN40: [[u8; 3]; 32] = [
    [1, 6, 13],
    [2, 3, 14],
    [4, 8, 15],
    [5, 11, 12],
    [7, 10, 16],
    [6, 9, 15],
    [1, 11, 16],
    [2, 4, 5],
    [3, 7, 9],
    [8, 10, 12],
    [8, 13, 14],
    [1, 4, 12],
    [2, 6, 10],
    [3, 11, 15],
    [5, 9, 14],
    [7, 13, 15],
    [12, 14, 16],
    [1, 2, 8],
    [3, 5, 6],
    [4, 9, 11],
    [1, 7, 14],
    [5, 10, 13],
    [3, 4, 16],
    [2, 15, 16],
    [6, 7, 12],
    [7, 8, 11],
    [1, 9, 10],
    [2, 11, 13],
    [3, 12, 13],
    [4, 6, 14],
    [1, 5, 15],
    [8, 9, 16],
];
const NM40: [[u8; 7]; 16] = [
    [1, 7, 12, 18, 21, 27, 31],
    [2, 8, 13, 18, 24, 28, 0],
    [2, 9, 14, 19, 23, 29, 0],
    [3, 8, 12, 20, 23, 30, 0],
    [4, 8, 15, 19, 22, 31, 0],
    [1, 6, 13, 19, 25, 30, 0],
    [5, 9, 16, 21, 25, 26, 0],
    [3, 10, 11, 18, 26, 32, 0],
    [6, 9, 15, 20, 27, 32, 0],
    [5, 10, 13, 22, 27, 0, 0],
    [4, 7, 14, 20, 26, 28, 0],
    [4, 10, 12, 17, 25, 29, 0],
    [1, 11, 16, 22, 28, 29, 0],
    [2, 11, 15, 17, 21, 30, 0],
    [3, 6, 14, 16, 24, 31, 0],
    [5, 7, 17, 23, 24, 32, 0],
];
const NRW40: [u8; 16] = [7, 6, 6, 6, 6, 6, 6, 6, 6, 5, 6, 6, 6, 6, 6, 6];

/// `hashlittle` di lookup3.c (Bob Jenkins, pubblico dominio), versione a byte.
fn hashlittle(key: &[u8], initval: u32) -> u32 {
    fn mix(a: &mut u32, b: &mut u32, c: &mut u32) {
        *a = a.wrapping_sub(*c);
        *a ^= c.rotate_left(4);
        *c = c.wrapping_add(*b);
        *b = b.wrapping_sub(*a);
        *b ^= a.rotate_left(6);
        *a = a.wrapping_add(*c);
        *c = c.wrapping_sub(*b);
        *c ^= b.rotate_left(8);
        *b = b.wrapping_add(*a);
        *a = a.wrapping_sub(*c);
        *a ^= c.rotate_left(16);
        *c = c.wrapping_add(*b);
        *b = b.wrapping_sub(*a);
        *b ^= a.rotate_left(19);
        *a = a.wrapping_add(*c);
        *c = c.wrapping_sub(*b);
        *c ^= b.rotate_left(4);
        *b = b.wrapping_add(*a);
    }
    fn fin(a: &mut u32, b: &mut u32, c: &mut u32) {
        *c ^= *b;
        *c = c.wrapping_sub(b.rotate_left(14));
        *a ^= *c;
        *a = a.wrapping_sub(c.rotate_left(11));
        *b ^= *a;
        *b = b.wrapping_sub(a.rotate_left(25));
        *c ^= *b;
        *c = c.wrapping_sub(b.rotate_left(16));
        *a ^= *c;
        *a = a.wrapping_sub(c.rotate_left(4));
        *b ^= *a;
        *b = b.wrapping_sub(a.rotate_left(14));
        *c ^= *b;
        *c = c.wrapping_sub(b.rotate_left(24));
    }
    let mut a = 0xdead_beefu32.wrapping_add(key.len() as u32).wrapping_add(initval);
    let (mut b, mut c) = (a, a);
    let word = |s: &[u8]| s.iter().enumerate().fold(0u32, |w, (i, &v)| w | (u32::from(v) << (8 * i)));
    let mut k = key;
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
    fin(&mut a, &mut b, &mut c);
    c
}

/// L'hash a 12 bit della coppia di nominativi (`hash` di WSJT-X: testo di 37 caratteri completato con spazi, valore iniziale 146).
pub fn hash40(call1: &str, call2: &str) -> u16 {
    let mut s = format!("{} {}", call1.trim().to_uppercase(), call2.trim().to_uppercase()).into_bytes();
    s.resize(37, b' ');
    (hashlittle(&s, 146) & 4095) as u16
}

/// `encode_msk40`: 16 bit -> 32 bit (16 di controllo e i 16 di messaggio, poi riordinati).
fn encode40(message: &[u8; 16]) -> [u8; 32] {
    let mut tmp = [0u8; 32];
    for (i, g) in GEN40.iter().enumerate() {
        tmp[i] = (0..16).filter(|&j| (g >> (15 - j)) & 1 == 1).fold(0u8, |s, j| s ^ message[j]);
    }
    tmp[16..].copy_from_slice(message);
    let mut cw = [0u8; 32];
    for (k, &c) in COLORDER40.iter().enumerate() {
        cw[c as usize] = tmp[k];
    }
    cw
}

/// Un messaggio breve «<IU3ABC K1ABC> R+03»: i due nominativi e l'indice del rapporto.
pub fn parse40(text: &str) -> Option<(String, String, usize)> {
    let t = text.split_whitespace().collect::<Vec<_>>().join(" ");
    let inner = t.strip_prefix('<')?;
    let (calls, rest) = inner.split_once('>')?;
    let (c1, c2) = calls.split_once(' ')?;
    let irpt = RPT40.iter().position(|r| *r == rest.trim())?;
    // `genmsk40`: il '>' almeno alla nona posizione
    if calls.len() + 2 < 9 || c2.contains(' ') {
        return None;
    }
    Some((c1.to_uppercase(), c2.to_uppercase(), irpt))
}

/// I 40 toni del messaggio breve (`genmsk40`); `None` se il testo non e' un messaggio breve.
pub fn tones40(text: &str) -> Option<[u8; 40]> {
    let (c1, c2, irpt) = parse40(text)?;
    let ig = (u32::from(hash40(&c1, &c2)) << 4) | irpt as u32;
    let mut msg = [0u8; 16];
    for (i, m) in msg.iter_mut().enumerate() {
        *m = ((ig >> (15 - i)) & 1) as u8;
    }
    let cw = encode40(&msg);
    let mut seq = [0i32; 40];
    for i in 0..8 {
        seq[i] = 2 * i32::from(S8R[i]) - 1;
    }
    for i in 0..32 {
        seq[8 + i] = 2 * i32::from(cw[i]) - 1;
    }
    let mut out = [0u8; 40];
    for i in 1..=20 {
        let a = (seq[2 * i - 1] * seq[2 * i - 2] + 1) / 2;
        let b = -(seq[2 * i - 1] * seq[(2 * i) % 40] - 1) / 2;
        out[2 * i - 2] = (1 - a) as u8;
        out[2 * i - 1] = (1 - b) as u8;
    }
    Some(out)
}

/// L'onda di un messaggio breve: `frames` quadri da 20 ms, `f_center` la frequenza centrale.
pub fn modulate40(tones: &[u8; 40], f_center: f64, frames: usize, amplitude: f32) -> Vec<f32> {
    let two_pi = 2.0 * std::f64::consts::PI;
    let d0 = two_pi * (f_center - 0.25 * BAUD) / FS;
    let d1 = two_pi * (f_center + 0.25 * BAUD) / FS;
    let mut out = Vec::with_capacity(frames * NSPM40);
    let mut phi = 0.0f64;
    for _ in 0..frames {
        for &t in tones {
            let d = if t == 0 { d0 } else { d1 };
            for _ in 0..6 {
                phi = (phi + d) % two_pi;
                out.push(amplitude * phi.sin() as f32);
            }
        }
    }
    out
}

/// `bpdecode40`: 32 rapporti di verosimiglianza -> 16 bit di messaggio.
fn bp_decode40(llr: &[f32; 32], max_iter: usize) -> Option<[u8; 16]> {
    let mut tov = [[0f32; 3]; 32];
    let mut toc = [[0f32; 7]; 16];
    for j in 0..16 {
        for i in 0..NRW40[j] as usize {
            toc[j][i] = llr[NM40[j][i] as usize - 1];
        }
    }
    let mut zn = [0f32; 32];
    let mut cw = [0u8; 32];
    for _ in 0..=max_iter {
        for i in 0..32 {
            zn[i] = llr[i] + tov[i].iter().sum::<f32>();
            cw[i] = u8::from(zn[i] > 0.0);
        }
        if (0..16).all(|j| (0..NRW40[j] as usize).map(|i| cw[NM40[j][i] as usize - 1] as usize).sum::<usize>() % 2 == 0) {
            let mut out = [0u8; 16];
            for k in 0..16 {
                out[k] = cw[COLORDER40[16 + k] as usize];
            }
            return Some(out);
        }
        for j in 0..16 {
            for i in 0..NRW40[j] as usize {
                let ibj = NM40[j][i] as usize - 1;
                toc[j][i] = zn[ibj];
                for kk in 0..3 {
                    if MN40[ibj][kk] as usize == j + 1 {
                        toc[j][i] -= tov[ibj][kk];
                    }
                }
            }
        }
        let mut th = [[0f32; 7]; 16];
        for j in 0..16 {
            for i in 0..7 {
                th[j][i] = (-toc[j][i] / 2.0).tanh();
            }
        }
        for j in 0..32 {
            for i in 0..3 {
                let ichk = MN40[j][i] as usize - 1;
                let t: f32 = (0..NRW40[ichk] as usize).filter(|&k| NM40[ichk][k] as usize != j + 1).map(|k| th[ichk][k]).product();
                tov[j][i] = 2.0 * platanh(-t);
            }
        }
    }
    None
}

fn sync_wave40() -> [Complex<f32>; 42] {
    let p = half_sine();
    let s: Vec<f32> = S8R.iter().map(|&v| f32::from(2 * v - 1)).collect();
    let mut out = [Complex::new(0f32, 0.0); 42];
    for k in 0..6 {
        out[k].im = p[6 + k] * s[0];
        out[36 + k].re = p[k] * s[7];
    }
    for k in 0..12 {
        out[6 + k].im = p[k] * s[2];
        out[18 + k].im = p[k] * s[4];
        out[30 + k].im = p[k] * s[6];
        out[k].re = p[k] * s[1];
        out[12 + k].re = p[k] * s[3];
        out[24 + k].re = p[k] * s[5];
    }
    out
}

/// Cio' che serve a riconoscere un messaggio breve: la coppia del QSO, l'ascolto e gli hash delle coppie di nominativi recenti
/// (`mycall`, `hiscall`, `bswl`, `nhasharray` e `recent_calls` di `msk40decodeframe.f90`).
pub(crate) struct ShortContext<'a> {
    pub mycall: &'a str,
    pub hiscall: &'a str,
    pub swl: bool,
    pub nhasharray: &'a [[i32; MAXRECENT]; MAXRECENT],
    pub recent_calls: &'a [String],
}

/// `msk40decodeframe`: un quadro breve allineato -> messaggio ed esito. 1 = «<MYCALL HISCALL> RPT» del QSO (l'hash della coppia nell'ordine
/// proprio nominativo, corrispondente: e' il messaggio che il corrispondente manda a noi); in ascolto 2 = coppia trovata fra i nominativi
/// recenti, 3 = solo l'hash («<1234> R+03»). Solo i rapporti da «R-03» in su (`nrxrpt.ge.7`).
fn decode_frame40(c: &mut [Complex<f32>], ctx: &ShortContext, xsnr: f32) -> Option<(String, u8)> {
    let cb = sync_wave40();
    let p = half_sine();
    // l'hash della coppia del QSO; senza corrispondente non eguaglia nessun hash ricevuto (`ihash=9999`)
    let ihash = (!ctx.hiscall.trim().is_empty()).then(|| hash40(ctx.mycall, ctx.hiscall));
    let cca: Complex<f32> = (0..42).map(|k| c[k] * cb[k].conj()).sum();
    let ph = cca.im.atan2(cca.re);
    let rot = Complex::new(ph.cos(), -ph.sin());
    for v in c.iter_mut() {
        *v *= rot;
    }
    let mut soft = [0f32; 40];
    soft[0] = (0..6).map(|k| c[k].im * p[6 + k]).sum::<f32>() + (0..6).map(|k| c[NSPM40 - 6 + k].im * p[k]).sum::<f32>();
    soft[1] = (0..12).map(|k| c[k].re * p[k]).sum();
    for i in 2..=20usize {
        let a = (i - 1) * 12 - 6;
        soft[2 * i - 2] = (0..12).map(|k| c[a + k].im * p[k]).sum();
        let b = (i - 1) * 12;
        soft[2 * i - 1] = (0..12).map(|k| c[(b + k) % NSPM40].re * p[k]).sum();
    }
    let hard: Vec<u8> = soft.iter().map(|&v| u8::from(v >= 0.0)).collect();
    let bad = (8 - (0..8).map(|k| (2 * i32::from(hard[k]) - 1) * (2 * i32::from(S8R[k]) - 1)).sum::<i32>()) / 2;
    if bad > 3 {
        return None;
    }
    let av = soft.iter().sum::<f32>() / 40.0;
    let s2 = soft.iter().map(|v| v * v).sum::<f32>() / 40.0;
    let sig = (s2 - av * av).sqrt();
    if sig <= 0.0 || !sig.is_finite() {
        return None;
    }
    // con il ping debole il decodificatore si fida meno dei bit (`msk40decodeframe.f90:110-112`)
    let sigma = if xsnr < 0.0 { 0.75 - 0.11 * xsnr } else { 0.75 };
    let mut llr = [0f32; 32];
    for i in 0..32 {
        llr[i] = 2.0 * soft[8 + i] / sig / (sigma * sigma);
    }
    let decoded = bp_decode40(&llr, 5)?;
    let cw = encode40(&decoded);
    let (mut nhamm, mut cord) = (0, 0f32);
    for i in 0..32 {
        if cw[i] != hard[8 + i] {
            nhamm += 1;
            cord += (soft[8 + i] / sig).abs();
        }
    }
    let imsg = decoded.iter().fold(0u32, |a, &b| (a << 1) | u32::from(b));
    let rpt = (imsg & 15) as usize;
    let h = (imsg >> 4) as i32;
    if nhamm > 4 || cord >= 0.65 || rpt < 7 {
        return None;
    }
    if ihash.is_some_and(|x| i32::from(x) == h) {
        return Some((format!("<{} {}> {}", ctx.mycall.trim(), ctx.hiscall.trim(), RPT40[rpt]), 1));
    }
    if !ctx.swl {
        return None;
    }
    // ascolto: la coppia fra i nominativi recenti (vince l'ultima trovata, come il ciclo di WSJT-X), altrimenti il solo hash
    let mut found = None;
    for i in 0..MAXRECENT {
        for j in i + 1..MAXRECENT {
            if h == ctx.nhasharray[i][j] {
                found = Some((i, j));
            } else if h == ctx.nhasharray[j][i] {
                found = Some((j, i));
            }
        }
    }
    Some(match found {
        Some((a, b)) => (format!("<{} {}> {}", ctx.recent_calls[a].trim(), ctx.recent_calls[b].trim(), RPT40[rpt]), 2),
        None => (format!("<{h:04}> {}", RPT40[rpt]), 3),
    })
}

/// `msk40spd.f90`: il decodificatore dei ping brevi dei messaggi brevi (quadri da 20 ms, passo di 5 ms). Restituisce messaggio, esito,
/// frequenza e istante (s dall'inizio del blocco).
pub(crate) fn spd40(cbig: &[Complex<f32>], ntol: u32, fc: f32, ctx: &ShortContext, fft: &dyn Fft<f32>) -> Option<(String, u8, f32, f32)> {
    let n = cbig.len();
    let cb = sync_wave40();
    for cand in candidates(cbig, NSPM40, 60, 150, 3.5, ntol, fc, fft) {
        let mut ib = cand.nstart.saturating_sub(NSPM40).max(1);
        let mut ie = ib - 1 + 3 * NSPM40;
        if ie > n {
            ie = n;
            ib = ie + 1 - 3 * NSPM40;
        }
        let cdat = &cbig[ib - 1..ie];
        let fo = fc + cand.ferr;
        for mask in &NAVPATTERNS {
            let s = sync_search(cdat, NSPM40, mask, 29.0, 7.2, fo, &cb, None, 24.0);
            if !s.ok {
                continue;
            }
            for &pk in &s.peaks {
                for ic0 in dithers(pk, NSPM40) {
                    let mut ct = cshift(&s.c, ic0);
                    if let Some((text, nsuccess)) = decode_frame40(&mut ct, ctx, cand.snr) {
                        return Some((text, nsuccess, s.fest, (cand.nstart + NSPM40 / 2) as f32 / 12_000.0));
                    }
                }
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn short_messages_parse_only_with_the_wsjt_reports() {
        assert_eq!(parse40("<IU3ABC K1ABC> R+03"), Some(("IU3ABC".into(), "K1ABC".into(), 9)));
        assert!(parse40("<IU3ABC K1ABC> +05").is_none());
        assert!(parse40("IU3ABC K1ABC R+03").is_none());
        let t = tones40("<IU3ABC K1ABC> RRR").expect("breve");
        assert!(t.iter().all(|&v| v <= 1));
    }

    #[test]
    fn the_short_code_decodes_its_own_codeword() {
        let msg = [1, 0, 1, 1, 0, 0, 1, 0, 1, 1, 1, 0, 0, 0, 1, 1];
        let cw = encode40(&msg);
        let mut llr = [0f32; 32];
        for i in 0..32 {
            llr[i] = if cw[i] == 1 { 4.0 } else { -4.0 };
        }
        assert_eq!(bp_decode40(&llr, 5), Some(msg));
    }
}
