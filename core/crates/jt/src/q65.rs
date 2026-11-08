//! Q65: 65-FSK, 85 simboli (22 di sincronismo sul tono 0), messaggio di 77 bit come FT8 con CRC-12 e codice QRA (15,65) su GF(64)
//! decodificato a passaggio di messaggi. Dai sorgenti di WSJT-X (GPL 3, codice di Nico Palermo IV3NWV): `genq65.f90`, `q65.c`,
//! `qracodes.c`, `npfwht.c`, `pdmath.c`, `qra15_65_64_irr_e23.c`, `q65sim.f90`.
//!
//! Periodi da 15, 30, 60, 120 e 300 s; sottomodi da A a E (distanza fra i toni = velocita' x 1, 2, 4, 8, 16).
//! La ricezione qui usa la metrica di Bessel per canale senza evanescenza (`qra_mfskbesselmetric`); la media fra periodi, le
//! informazioni a priori e il modello di evanescenza rapida di WSJT-X non ci sono ancora.

use ft8::message::{Bits77, CallHashes, pack, unpack};
use rustfft::{FftPlanner, num_complex::Complex};

use crate::q65tables::{ACC_INPUT_IDX, ACC_INPUT_WLOG, C2VMIDX, CDEG, GFEXP, GFLOG, MSGW, PMAT, V2CMIDX, VDEG};

const FS: f64 = 12_000.0;
/// Simboli trasmessi.
pub const NSYM: usize = 85;
/// Posizioni (da 1) dei simboli di sincronismo, trasmessi sul tono 0.
pub const ISYNC: [usize; 22] = [1, 9, 12, 13, 15, 22, 23, 26, 27, 33, 35, 38, 46, 50, 55, 60, 62, 66, 69, 74, 76, 85];

/// Simboli d'informazione (13 di messaggio + 2 di CRC-12).
const K: usize = 15;
/// Simboli di messaggio.
const NK: usize = 13;
/// Lunghezza della parola di codice prima di togliere il CRC.
const N: usize = 65;
/// Simboli trasmessi del codice (senza i due di CRC).
const NTX: usize = 63;
const M: usize = 64;
const NC: usize = N - K;
const NV: usize = 65;
const NCF: usize = 116;
const NMSG: usize = 216;
const MAXVDEG: usize = 5;
const MAXCDEG: usize = 3;
const LLH_THRESHOLD: f32 = -260.0;

/// Periodo e sottomodo.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Setup {
    /// Durata del periodo: 15, 30, 60, 120 o 300 s.
    pub period_s: u32,
    /// 0 = A ... 4 = E.
    pub submode: u8,
}

impl Setup {
    pub fn new(period_s: u32, submode: u8) -> Option<Setup> {
        (matches!(period_s, 15 | 30 | 60 | 120 | 300) && submode <= 4).then_some(Setup { period_s, submode })
    }

    /// Campioni per simbolo a 12 kHz.
    pub fn nsps(self) -> usize {
        match self.period_s {
            15 => 1_800,
            30 => 3_600,
            60 => 7_200,
            120 => 16_000,
            _ => 41_472,
        }
    }

    pub fn baud(self) -> f64 {
        FS / self.nsps() as f64
    }

    /// Distanza fra i toni in hertz.
    pub fn tone_spacing_hz(self) -> f64 {
        self.baud() * f64::from(1u32 << self.submode)
    }

    /// Inizio dell'audio dall'inizio del periodo: 0,5 s per 15 e 30 s, 1 s dai 60 s in su.
    pub fn start_s(self) -> f64 {
        if self.period_s >= 60 { 1.0 } else { 0.5 }
    }

    /// Larghezza del segnale (65 toni).
    pub fn bandwidth_hz(self) -> f64 {
        65.0 * self.tone_spacing_hz()
    }
}

impl Default for Setup {
    fn default() -> Self {
        Setup { period_s: 60, submode: 0 }
    }
}

/// `_q65_crc12`: due simboli da 6 bit (polinomio x^12 + x^11 + x^3 + x^2 + x + 1).
fn crc12(x: &[i32]) -> [i32; 2] {
    const POLY: i32 = 0xF01;
    let mut sr = 0i32;
    for &v in x {
        let mut t = v;
        for _ in 0..6 {
            sr = if (t ^ sr) & 1 != 0 { (sr >> 1) ^ POLY } else { sr >> 1 };
            t >>= 1;
        }
    }
    [sr & 0x3F, sr >> 6]
}

/// `qra_encode` (fattore di raggruppamento 1): i 15 simboli d'informazione seguiti dai 50 di controllo.
fn qra_encode(x: &[i32; K]) -> [i32; N] {
    let mut y = [0i32; N];
    y[..K].copy_from_slice(x);
    let mut chk = 0i32;
    for k in 0..NC {
        let t = x[ACC_INPUT_IDX[k] as usize];
        if t != 0 {
            let l = (GFLOG[t as usize] + ACC_INPUT_WLOG[k]) % (M as i32 - 1);
            chk ^= GFEXP[l as usize];
        }
        y[K + k] = chk;
    }
    y
}

/// `q65_encode`: 13 simboli di messaggio -> 63 simboli trasmessi (CRC aggiunto e poi tolto).
fn encode_symbols(msg: &[i32; NK]) -> [i32; NTX] {
    let mut x = [0i32; K];
    x[..NK].copy_from_slice(msg);
    let c = crc12(msg);
    x[NK] = c[0];
    x[NK + 1] = c[1];
    let y = qra_encode(&x);
    let mut out = [0i32; NTX];
    out[..NK].copy_from_slice(&y[..NK]);
    out[NK..].copy_from_slice(&y[K..]);
    out
}

/// 77 bit -> 13 simboli (12 da 6 bit e l'ultimo da 5 bit con il 78° bit a zero), come `genq65`; RR73 ha il suo codice (`ng15`).
fn bits_to_symbols(bits: &Bits77) -> [i32; NK] {
    let mut b = *bits;
    let ng15 = b[59..74].iter().fold(0u32, |a, &x| (a << 1) | u32::from(x));
    if ng15 == 32_373 {
        for (i, c) in "111111010010011".bytes().enumerate() {
            b[59 + i] = c - b'0';
        }
    }
    let mut out = [0i32; NK];
    for (i, o) in out.iter_mut().enumerate().take(12) {
        *o = b[6 * i..6 * i + 6].iter().fold(0, |a, &x| (a << 1) | i32::from(x));
    }
    out[12] = 2 * b[72..77].iter().fold(0, |a, &x| (a << 1) | i32::from(x));
    out
}

fn symbols_to_bits(s: &[i32; NK]) -> Bits77 {
    let mut b = [0u8; 77];
    for i in 0..12 {
        for j in 0..6 {
            b[6 * i + j] = ((s[i] >> (5 - j)) & 1) as u8;
        }
    }
    let last = s[12] >> 1;
    for j in 0..5 {
        b[72 + j] = ((last >> (4 - j)) & 1) as u8;
    }
    b
}

/// Gli 85 toni (0 = sincronismo, da 1 a 64 i dati). Errore se il testo non e' un messaggio a 77 bit.
pub fn tones(text: &str, hashes: &mut CallHashes) -> Result<[u8; NSYM], String> {
    let bits = pack(text, hashes).map_err(|e| format!("{e:?}"))?;
    let sent = encode_symbols(&bits_to_symbols(&bits));
    let mut out = [0u8; NSYM];
    let mut k = 0;
    for (i, t) in out.iter_mut().enumerate() {
        if ISYNC.contains(&(i + 1)) {
            *t = 0;
        } else {
            *t = sent[k] as u8 + 1;
            k += 1;
        }
    }
    Ok(out)
}

/// L'onda a 12 kHz (fase continua): `f0` e' la frequenza del tono 0 (sincronismo).
pub fn modulate(tones: &[u8; NSYM], f0: f64, setup: Setup, amplitude: f32) -> Vec<f32> {
    let nsps = setup.nsps();
    let two_pi = 2.0 * std::f64::consts::PI;
    let mut out = Vec::with_capacity(NSYM * nsps);
    let mut phase = 0.0f64;
    for &t in tones {
        let dphi = two_pi * (f0 + f64::from(t) * setup.tone_spacing_hz()) / FS;
        for _ in 0..nsps {
            phase += dphi;
            if phase > two_pi {
                phase -= two_pi;
            }
            out.push(amplitude * phase.sin() as f32);
        }
    }
    out
}

// ---------------------------------------------------------------- decodificatore QRA

/// Trasformata di Walsh-Hadamard veloce non normalizzata (`np_fwht`), sul posto.
fn fwht(v: &mut [f32]) {
    let n = v.len();
    let mut h = 1;
    while h < n {
        for i in (0..n).step_by(2 * h) {
            for j in i..i + h {
                let (a, b) = (v[j], v[j + h]);
                v[j] = a + b;
                v[j + h] = a - b;
            }
        }
        h *= 2;
    }
}

/// `pd_norm`: normalizza a somma 1 (uniforme se la somma non e' positiva).
fn pd_norm(p: &mut [f32]) -> f32 {
    let t: f32 = p.iter().sum();
    if t <= 0.0 {
        p.fill(1.0 / p.len() as f32);
        return t;
    }
    let inv = 1.0 / t;
    p.iter_mut().for_each(|x| *x *= inv);
    t
}

fn row(v: &[f32], i: usize) -> &[f32] {
    &v[i * M..(i + 1) * M]
}

fn row_mut(v: &mut [f32], i: usize) -> &mut [f32] {
    &mut v[i * M..(i + 1) * M]
}

/// `qra_extrinsic`: passaggio di messaggi sul grafo del codice. Ritorna l'informazione estrinseca e l'iterazione di convergenza.
fn qra_extrinsic(pix: &[f32], maxiter: usize) -> (Vec<f32>, Option<usize>) {
    let mut c2v = vec![0f32; NMSG * M];
    let mut v2c = vec![0f32; NMSG * M];
    c2v[..NV * M].copy_from_slice(pix);
    for nv in 0..NV {
        let ndeg = VDEG[nv] as usize;
        for k in 1..ndeg {
            let imsg = V2CMIDX[nv * MAXVDEG + k] as usize;
            row_mut(&mut v2c, imsg).copy_from_slice(row(pix, nv));
        }
    }
    let uniform = 1.0 / M as f32;
    let mut msgout = [0f32; M];
    let mut rc = None;
    for nit in 0..maxiter {
        for nc in NV..NCF {
            let ndeg = CDEG[nc] as usize;
            if ndeg < 2 {
                return (Vec::new(), None);
            }
            let base = nc * MAXCDEG;
            for k in 0..ndeg {
                fwht(row_mut(&mut v2c, C2VMIDX[base + k] as usize));
            }
            for k in 0..ndeg {
                msgout.fill(uniform);
                for kk in 0..ndeg {
                    if kk != k {
                        let r = row(&v2c, C2VMIDX[base + kk] as usize);
                        msgout.iter_mut().zip(r).for_each(|(a, b)| *a *= b);
                    }
                }
                msgout[0] += 1e-7;
                fwht(&mut msgout);
                let imsg = C2VMIDX[base + k] as usize;
                let w = MSGW[imsg] as usize;
                let dst = row_mut(&mut c2v, imsg);
                if w == 0 {
                    dst.copy_from_slice(&msgout);
                } else {
                    // pd_bwdperm: dst[perm[i]] = src[i]
                    let perm = &PMAT[w * M..(w + 1) * M];
                    for i in 0..M {
                        dst[perm[i] as usize] = msgout[i];
                    }
                }
            }
        }
        for nv in 0..NV {
            let ndeg = VDEG[nv] as usize;
            let base = nv * MAXVDEG;
            for k in 0..ndeg {
                msgout.fill(uniform);
                for kk in 0..ndeg {
                    if kk != k {
                        let r = row(&c2v, V2CMIDX[base + kk] as usize);
                        msgout.iter_mut().zip(r).for_each(|(a, b)| *a *= b);
                    }
                }
                pd_norm(&mut msgout);
                let imsg = V2CMIDX[base + k] as usize;
                let w = MSGW[imsg] as usize;
                let dst = row_mut(&mut v2c, imsg);
                if w == 0 {
                    dst.copy_from_slice(&msgout);
                } else {
                    // pd_fwdperm: dst[i] = src[perm[i]]
                    let perm = &PMAT[w * M..(w + 1) * M];
                    for i in 0..M {
                        dst[i] = msgout[perm[i] as usize];
                    }
                }
            }
        }
        let totex: f32 = (0..NV).map(|nv| row(&v2c, nv).iter().copied().fold(0f32, f32::max)).sum();
        if totex > NV as f32 - 0.01 {
            rc = Some(nit);
            break;
        }
    }
    (v2c[..NV * M].to_vec(), rc)
}

/// `qra_ioapprox` + `pd_norm` (`qra_mfskbesselmetric`): dalle energie (63 x 64) alle probabilita' dei simboli.
fn bessel_intrinsics(energies: &[f32]) -> Vec<f32> {
    // Eb/No di 2,8 dB, rendimento 13/63 dopo la foratura, 6 bit per simbolo (`q65_init`)
    let ebno = 10f32.powf(2.8 / 10.0);
    let esno = 6.0 * (NK as f32 / NTX as f32) * ebno;
    let mean = energies.iter().sum::<f32>() / energies.len() as f32;
    let sigma = (mean / (1.0 + esno / M as f32) / 2.0).sqrt();
    let c = (2.0 * esno).sqrt() / sigma.max(f32::MIN_POSITIVE);
    let mut pix: Vec<f32> = energies.iter().map(|e| e.max(0.0).sqrt()).collect();
    for k in 0..NTX {
        let r = row_mut(&mut pix, k);
        for v in r.iter_mut() {
            let x = *v * c;
            let sq = x * x;
            let l = (sq * (x + 0.039) / (sq * 0.9931 + x * 2.6936 + 0.5185)).min(80.0);
            *v = l.exp();
        }
        pd_norm(r);
    }
    pix
}

/// `q65_decode`: dalle probabilita' dei 63 simboli ai 13 di messaggio, con CRC e controllo di verosimiglianza.
fn qra_decode(intrinsics: &[f32]) -> Option<[i32; NK]> {
    let mut ix = vec![0f32; N * M];
    ix[..NK * M].copy_from_slice(&intrinsics[..NK * M]);
    ix[NK * M..K * M].fill(1.0 / M as f32);
    ix[K * M..].copy_from_slice(&intrinsics[NK * M..]);
    let (mut ex, rc) = qra_extrinsic(&ix, 100);
    rc?;
    let mut x = [0i32; K];
    for (k, xv) in x.iter_mut().enumerate() {
        let e = row_mut(&mut ex, k);
        e.iter_mut().zip(row(&ix, k)).for_each(|(a, b)| *a *= b);
        // pd_argmax: a parita' vince l'indice piu' basso
        let mut best = (0usize, -1f32);
        for (i, &v) in e.iter().enumerate().rev() {
            if v >= best.1 {
                best = (i, v);
            }
        }
        *xv = best.0 as i32;
    }
    let mut msg = [0i32; NK];
    msg.copy_from_slice(&x[..NK]);
    if crc12(&msg) != [x[NK], x[NK + 1]] {
        return None;
    }
    let cw = encode_symbols(&msg);
    let llh: f32 = cw.iter().enumerate().map(|(k, &y)| row(intrinsics, k)[y as usize].max(1e-36).ln()).sum();
    (llh >= LLH_THRESHOLD).then_some(msg)
}

// ---------------------------------------------------------------- ricezione

/// Un messaggio decodificato.
#[derive(Debug, Clone, PartialEq)]
pub struct Decoded {
    pub text: String,
    /// Frequenza del tono di sincronismo (tono 0), in hertz.
    pub freq_hz: f32,
    /// Ritardo rispetto all'inizio nominale, in secondi.
    pub dt_s: f32,
    /// Rapporto segnale/rumore in 2500 Hz, in dB.
    pub snr_db: i32,
    pub sync: f32,
}

#[derive(Debug, Clone)]
pub struct Options {
    pub setup: Setup,
    pub f_low_hz: f32,
    pub f_high_hz: f32,
    pub max_candidates: usize,
    /// Soglia del sincronismo (rapporto dell'energia sui simboli di sincronismo rispetto al rumore).
    pub min_sync: f32,
}

impl Default for Options {
    fn default() -> Self {
        Options { setup: Setup::default(), f_low_hz: 200.0, f_high_hz: 3000.0, max_candidates: 20, min_sync: 2.5 }
    }
}

/// Spettri di potenza di un simbolo (un bin = la velocita' di simbolo) a passi di mezzo simbolo.
fn spectra(audio: &[f32], nsps: usize, nbins: usize, step: usize) -> Vec<Vec<f32>> {
    let fft = FftPlanner::<f32>::new().plan_fft_forward(nsps);
    let mut buf = vec![Complex::new(0f32, 0.0); nsps];
    let mut out = Vec::new();
    let mut k = 0usize;
    while k + nsps <= audio.len() {
        for (b, &x) in buf.iter_mut().zip(&audio[k..k + nsps]) {
            *b = Complex::new(x, 0.0);
        }
        fft.process(&mut buf);
        out.push(buf[..nbins].iter().map(|c| c.norm_sqr()).collect());
        k += step;
    }
    out
}

/// Decodifica un periodo (audio a 12 kHz dall'inizio del periodo).
pub fn decode(audio: &[f32], options: &Options, hashes: &mut CallHashes) -> Vec<Decoded> {
    let setup = options.setup;
    let nsps = setup.nsps();
    let baud = setup.baud();
    let spb = 1usize << setup.submode; // bin per tono
    let top = (f64::from(options.f_high_hz) + setup.bandwidth_hz()) / baud;
    let nbins = ((top.ceil() as usize) + 4).min(nsps / 2);
    let half = nsps / 2;
    let sp = spectra(audio, nsps, nbins, half);
    if sp.len() < 2 * NSYM {
        return Vec::new();
    }
    // rumore di ogni bin: mediana nel tempo (i segnali occupano pochi bin per volta)
    let noise: Vec<f32> = (0..nbins)
        .map(|i| {
            let mut col: Vec<f32> = sp.iter().map(|r| r[i]).collect();
            col.sort_by(f32::total_cmp);
            col[col.len() / 2].max(f32::MIN_POSITIVE)
        })
        .collect();
    let start0 = setup.start_s() * FS / half as f64;
    let lag_lo = (start0 - 2.0 * FS / half as f64).floor().max(0.0) as i64;
    let lag_hi = ((start0 + 2.0 * FS / half as f64).ceil() as i64).min(sp.len() as i64 - 2 * NSYM as i64);
    let ia = ((f64::from(options.f_low_hz) / baud).floor() as usize).max(1);
    let ib = ((f64::from(options.f_high_hz) / baud).ceil() as usize).min(nbins.saturating_sub(65 * spb + 1));
    let mut found: Vec<(usize, i64, f32)> = Vec::new();
    for i in ia..=ib {
        let mut best = (0i64, f32::MIN);
        for lag in lag_lo..=lag_hi {
            let s: f32 = ISYNC.iter().map(|&p| sp[(lag + 2 * (p as i64 - 1)) as usize][i] / noise[i]).sum::<f32>() / ISYNC.len() as f32;
            if s > best.1 {
                best = (lag, s);
            }
        }
        found.push((i, best.0, best.1));
    }
    let width = 2 * spb;
    let mut candidates: Vec<(usize, i64, f32)> = found
        .iter()
        .copied()
        .filter(|c| c.2 >= options.min_sync && found.iter().all(|o| o.0.abs_diff(c.0) > width || o.2 <= c.2))
        .collect();
    candidates.sort_by(|a, b| b.2.total_cmp(&a.2));
    candidates.truncate(options.max_candidates);

    let fft = FftPlanner::<f32>::new().plan_fft_forward(nsps);
    let mut buf = vec![Complex::new(0f32, 0.0); nsps];
    let mut out: Vec<Decoded> = Vec::new();
    for (i, lag, sync) in candidates {
        let mut done = None;
        // si prova il ritardo trovato e i quarti di simbolo vicini
        'try_dt: for off in [0i64, -1, 1, -2, 2] {
            let start = lag * half as i64 + off * (nsps as i64 / 4);
            let mut energies = vec![0f32; NTX * M];
            let mut k = 0;
            for s in 0..NSYM {
                if ISYNC.contains(&(s + 1)) {
                    continue;
                }
                let a = start + (s * nsps) as i64;
                for (j, b) in buf.iter_mut().enumerate() {
                    let idx = a + j as i64;
                    let v = if idx >= 0 && (idx as usize) < audio.len() { audio[idx as usize] } else { 0.0 };
                    *b = Complex::new(v, 0.0);
                }
                fft.process(&mut buf);
                for t in 0..M {
                    let bin = i + (t + 1) * spb;
                    energies[k * M + t] = buf.get(bin).map_or(0.0, |c| c.norm_sqr());
                }
                k += 1;
            }
            let pix = bessel_intrinsics(&energies);
            if let Some(msg) = qra_decode(&pix) {
                // S/N dalla parola decodificata (`q65_esnodb`)
                let cw = encode_symbols(&msg);
                let (mut sn, mut nn) = (0f64, 0f64);
                for (r, &y) in cw.iter().enumerate() {
                    for t in 0..M {
                        let e = f64::from(energies[r * M + t]);
                        if t == y as usize { sn += e } else { nn += e }
                    }
                }
                sn /= NTX as f64;
                nn /= (NTX * (M - 1)) as f64;
                let esno = if nn > 0.0 { ((sn - nn).max(0.316 * nn) / nn).log10() * 10.0 } else { 50.0 };
                let snr = esno - 10.0 * (2500.0 / baud).log10();
                done = Some((msg, start, snr));
                break 'try_dt;
            }
        }
        if let Some((msg, start, snr)) = done
            && let Ok(text) = unpack(&symbols_to_bits(&msg), hashes)
            && !out.iter().any(|d| d.text == text)
        {
            out.push(Decoded {
                text,
                freq_hz: (i as f64 * baud) as f32,
                dt_s: (start as f64 / FS - setup.start_s()) as f32,
                snr_db: (snr.round() as i32).clamp(-35, 25),
                sync,
            });
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn noise(n: usize, sigma: f32, seed: u64) -> Vec<f32> {
        let mut x = seed.max(1);
        let mut next = || {
            x ^= x << 13;
            x ^= x >> 7;
            x ^= x << 17;
            (x >> 11) as f64 / (1u64 << 53) as f64
        };
        (0..n)
            .map(|_| {
                let (u1, u2) = (next().max(1e-12), next());
                sigma * ((-2.0 * u1.ln()).sqrt() * (2.0 * std::f64::consts::PI * u2).cos()) as f32
            })
            .collect()
    }

    fn period(msg: &str, f0: f64, dt: f64, setup: Setup, amp: f32, sigma: f32) -> Vec<f32> {
        let mut audio = noise(setup.period_s as usize * 12_000, sigma, 5);
        let wave = modulate(&tones(msg, &mut CallHashes::default()).expect("messaggio"), f0, setup, amp);
        let at = ((setup.start_s() + dt) * FS) as usize;
        for (k, v) in wave.iter().enumerate() {
            if at + k < audio.len() {
                audio[at + k] += v;
            }
        }
        audio
    }

    #[test]
    fn the_code_is_systematic_and_the_crc_checks() {
        let msg = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 14];
        let cw = encode_symbols(&msg);
        assert_eq!(&cw[..13], &msg);
        assert!(cw.iter().all(|&v| (0..64).contains(&v)));
    }

    #[test]
    fn a_clean_codeword_decodes_through_the_message_passing() {
        let msg = [33, 2, 63, 0, 5, 17, 40, 8, 9, 10, 11, 12, 20];
        let cw = encode_symbols(&msg);
        let mut energies = vec![1.0f32; NTX * M];
        for (k, &y) in cw.iter().enumerate() {
            energies[k * M + y as usize] = 30.0;
        }
        assert_eq!(qra_decode(&bessel_intrinsics(&energies)), Some(msg));
    }

    #[test]
    fn tones_have_22_sync_symbols_on_tone_zero() {
        let t = tones("CQ IU3ABC IO91", &mut CallHashes::default()).expect("messaggio");
        assert_eq!(t.iter().filter(|&&v| v == 0).count() >= 22, true);
        for p in ISYNC {
            assert_eq!(t[p - 1], 0);
        }
        assert!(t.iter().all(|&v| v <= 64));
    }

    #[test]
    fn bits_go_back_and_forth() {
        let bits = pack("IU3ABC K1ABC -15", &mut CallHashes::default()).expect("messaggio");
        assert_eq!(symbols_to_bits(&bits_to_symbols(&bits)), bits);
    }

    #[test]
    fn a_noisy_q65_60a_signal_decodes() {
        let setup = Setup::new(60, 0).expect("modo");
        let audio = period("CQ IU3ABC IO91", 1500.0, 0.3, setup, 0.05, 1.0);
        let d = decode(&audio, &Options { setup, ..Options::default() }, &mut CallHashes::default());
        let hit = d.iter().find(|d| d.text == "CQ IU3ABC IO91").expect("decodificato");
        assert!((hit.freq_hz - 1500.0).abs() < 2.0, "{hit:?}");
    }

    #[test]
    fn q65_15a_and_30b_decode() {
        for (p, s) in [(15, 0), (30, 1)] {
            let setup = Setup::new(p, s).expect("modo");
            let audio = period("IU3ABC K1ABC RR73", 1000.0, 0.1, setup, 0.1, 1.0);
            let d = decode(&audio, &Options { setup, ..Options::default() }, &mut CallHashes::default());
            assert!(d.iter().any(|d| d.text == "IU3ABC K1ABC RR73"), "{p} {s}: {d:?}");
        }
    }

    #[test]
    fn pure_noise_decodes_nothing() {
        let setup = Setup::new(30, 0).expect("modo");
        let audio = noise(30 * 12_000, 1.0, 9);
        assert!(decode(&audio, &Options { setup, ..Options::default() }, &mut CallHashes::default()).is_empty());
    }
}
