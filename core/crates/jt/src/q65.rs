//! Q65: 65-FSK, 85 simboli (22 di sincronismo sul tono 0), messaggio di 77 bit come FT8 con CRC-12 e codice QRA (15,65) su GF(64)
//! decodificato a passaggio di messaggi. Dai sorgenti di WSJT-X (GPL 3, codice di Nico Palermo IV3NWV): `genq65.f90`, `q65.c`,
//! `qracodes.c`, `npfwht.c`, `pdmath.c`, `qra15_65_64_irr_e23.c`, `q65sim.f90`.
//!
//! Periodi da 15, 30, 60, 120 e 300 s; sottomodi da A a E (distanza fra i toni = velocita' x 1, 2, 4, 8, 16).
//! La ricezione usa la metrica di Bessel (`qra_mfskbesselmetric`) e, a scelta, quella dell'evanescenza rapida per EME e scatter
//! (`q65_intrinsics_fastfading`, tabelle gaussiane e lorentziane); prova le ipotesi AP del QSO (`q65_ap.f90`) e puo' sommare i periodi
//! della stessa parita' per i segnali troppo deboli (`q65_avg`).

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
/// Soglia per le ipotesi AP con il messaggio intero (RRR, 73, RR73).
const LLH_THRESHOLD_FULL_AP: f32 = -200.0;

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

/// Es/No della metrica del decodificatore: Eb/No di 2,8 dB, rendimento 13/63 dopo la foratura, 6 bit per simbolo (`q65_init`).
fn decoder_esno() -> f32 {
    6.0 * (NK as f32 / NTX as f32) * 10f32.powf(2.8 / 10.0)
}

/// `qra_ioapprox` + `pd_norm` (`qra_mfskbesselmetric`): dalle energie (63 x 64) alle probabilita' dei simboli.
fn bessel_intrinsics(energies: &[f32]) -> Vec<f32> {
    let esno = decoder_esno();
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
fn qra_decode(intrinsics: &[f32], ap: Option<&Ap>) -> Option<[i32; NK]> {
    let mut ix = vec![0f32; N * M];
    ix[..NK * M].copy_from_slice(&intrinsics[..NK * M]);
    if let Some(ap) = ap {
        apply_ap(&mut ix, ap);
    }
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
    // con il messaggio intero imposto il CRC non dice nulla: soglia piu' severa
    let threshold = if ap.is_some_and(|a| a.kind >= 4) { LLH_THRESHOLD_FULL_AP } else { LLH_THRESHOLD };
    (llh >= threshold).then_some(msg)
}

// ---------------------------------------------------------------- evanescenza rapida

/// Modello dello spettro allargato dall'evanescenza (EME, scatter): `fadingModel` di `q65_intrinsics_fastfading`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Fading {
    Gaussian,
    /// Quello che usa WSJT-X (`nFadingModel=1`).
    Lorentzian,
}

/// Durata di simbolo di QRA64, sulla quale sono calcolate le tabelle dei pesi (`TS_QRA64`).
const TS_QRA64: f32 = 0.576;

/// `q65_intrinsics_fastfading`: dalle energie larghe (63 x 64·(2+bin per tono), il tono 0 dei dati a 64 bin dall'inizio di ogni riga)
/// alle probabilita' dei simboli, con l'energia di ogni tono sparsa sui bin vicini secondo la larghezza B90 (90% dell'energia).
/// `gain` moltiplica i logaritmi delle probabilita': 1 per un periodo, il numero dei periodi per le energie sommate (la loro varianza
/// relativa cala con la somma, e la verosimiglianza cresce con essa).
fn fading_intrinsics(wide: &[f32], spb: usize, b90ts: f32, model: Fading, gain: f32) -> Vec<f32> {
    let b90 = b90ts / TS_QRA64;
    let hidx = ((b90.ln() / 1.09f32.ln() - 0.499) as i32).clamp(0, 63) as usize;
    let taps = match model {
        Fading::Gaussian => crate::q65fading::GAUSS[hidx],
        Fading::Lorentzian => crate::q65fading::LORENTZ[hidx],
    };
    let hlen = taps.len();
    // soglia del 50% che peggiora di 8 dB alla larghezza massima di 240 Hz di QRA64
    let degr = 8.0 * b90.ln() / 240f32.ln();
    let esno_metric = decoder_esno() * 10f32.powf(degr / 10.0);
    let wbins = M * (2 + spb);
    let mean = wide.iter().sum::<f32>() / wide.len() as f32;
    let noise = (mean / (1.0 + esno_metric / wbins as f32)).max(f32::MIN_POSITIVE);
    let weight: Vec<f32> = taps
        .iter()
        .map(|&g| {
            let t = g * esno_metric;
            gain * t / (1.0 + t) / noise
        })
        .collect();
    let hhsz = hlen - 1;
    let hlast = 2 * hhsz;
    let mut pix = vec![0f32; NTX * M];
    for n in 0..NTX {
        let base = n * wbins + M;
        let r = row_mut(&mut pix, n);
        let mut maxlogp = 0f32;
        for (k, v) in r.iter_mut().enumerate() {
            let first = base + k * spb - hhsz;
            let mut t = weight[hhsz] * wide[first + hhsz];
            for j in 0..hhsz {
                t += weight[j] * (wide[first + j] + wide[first + hlast - j]);
            }
            maxlogp = maxlogp.max(t);
            *v = t;
        }
        for v in r.iter_mut() {
            *v = (*v - maxlogp).clamp(-85.0, 85.0).exp();
        }
        pd_norm(r);
    }
    pix
}

/// Le larghezze B90·Ts provate, come `q65_decode.f90` (B90 = 1,72^ibw Hz, da ibw 1/3/8 secondo il sottomodo, sette passi).
fn b90_sweep(setup: Setup) -> impl Iterator<Item = f32> {
    let ibwa = [1, 3, 8, 8, 8][usize::from(setup.submode.min(4))];
    let baud = setup.baud() as f32;
    (ibwa..=(ibwa + 6).min(15)).map(move |ibw| 1.72f32.powi(ibw) / baud)
}

// ---------------------------------------------------------------- informazioni a priori (AP)

/// Ipotesi sul messaggio atteso (`q65_ap.f90`): bit gia' noti dei 13 simboli di messaggio.
#[derive(Debug, Clone)]
struct Ap {
    /// 1 CQ ??? ???, 2 MioNominativo ??? ???, 3 MioNominativo DX ???, 4 ... RRR, 5 ... 73, 6 ... RR73.
    kind: u8,
    mask: [i32; NK],
    sym: [i32; NK],
}

/// L'ipotesi da un messaggio modello, tenendo i bit `known` (e sempre il tipo i3 e il 78° bit a zero).
fn ap_from(text: &str, known: std::ops::Range<usize>, kind: u8, hashes: &CallHashes) -> Option<Ap> {
    let bits = pack(text, &mut hashes.clone()).ok()?;
    // solo messaggi standard (i3 = 1), come `apsym0(1).gt.1` di WSJT-X
    if bits[74..77] != [0, 0, 1] {
        return None;
    }
    let mut maskbits = [0u8; 77];
    maskbits[known].fill(1);
    maskbits[74..77].fill(1);
    let sym = bits_to_symbols(&bits);
    let mut mask = bits_to_symbols(&maskbits);
    mask[12] |= 1;
    Some(Ap { kind, mask, sym })
}

/// Le ipotesi per i nominativi dati (vuoti = sconosciuti): sempre il CQ, poi quelle con il mio nominativo e con il DX.
fn ap_list(my_call: &str, dx_call: &str, hashes: &CallHashes) -> Vec<Ap> {
    let (my, dx) = (my_call.trim().to_uppercase(), dx_call.trim().to_uppercase());
    let mut out = Vec::new();
    out.extend(ap_from("CQ K1ABC FN42", 0..29, 1, hashes));
    if !my.is_empty() {
        out.extend(ap_from(&format!("{my} K1ABC FN42"), 0..29, 2, hashes));
        if !dx.is_empty() {
            out.extend(ap_from(&format!("{my} {dx} FN42"), 0..58, 3, hashes));
            for (kind, end) in [(4, "RRR"), (5, "73"), (6, "RR73")] {
                out.extend(ap_from(&format!("{my} {dx} {end}"), 0..77, kind, hashes));
            }
        }
    }
    out
}

/// `_q65_mask`: azzera le probabilita' dei valori che contraddicono i bit noti.
fn apply_ap(ix: &mut [f32], ap: &Ap) {
    for k in 0..NK {
        let mask = ap.mask[k];
        if mask == 0 {
            continue;
        }
        let r = row_mut(ix, k);
        for (v, p) in r.iter_mut().enumerate() {
            if ((v as i32 ^ ap.sym[k]) & mask) != 0 {
                *p = 0.0;
            }
        }
        pd_norm(r);
    }
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
    /// Ipotesi AP usata (0 = nessuna, poi i tipi di `q65_ap.f90`: 1 CQ, 2 mio nominativo, 3 con il DX, 4 RRR, 5 73, 6 RR73).
    pub ap: u8,
    /// Periodi sommati per decodificarlo (1 = un periodo solo).
    pub averaged: u32,
    /// Decodificato con la metrica dell'evanescenza rapida (larghezza B90 in Hz), non con quella di Bessel.
    pub b90_hz: Option<f32>,
}

#[derive(Debug, Clone)]
pub struct Options {
    pub setup: Setup,
    pub f_low_hz: f32,
    pub f_high_hz: f32,
    pub max_candidates: usize,
    /// Soglia del sincronismo (rapporto dell'energia sui simboli di sincronismo rispetto al rumore).
    pub min_sync: f32,
    /// Il mio nominativo e quello del DX per le ipotesi AP (vuoti = sconosciuti).
    pub my_call: String,
    pub dx_call: String,
    /// Decodifica con le ipotesi AP (CQ sempre, le altre se ci sono i nominativi).
    pub ap: bool,
    /// Metrica dell'evanescenza rapida, provata dopo quella di Bessel su piu' larghezze B90.
    pub fading: Option<Fading>,
}

impl Default for Options {
    fn default() -> Self {
        Options {
            setup: Setup::default(),
            f_low_hz: 200.0,
            f_high_hz: 3000.0,
            max_candidates: 20,
            min_sync: 2.5,
            my_call: String::new(),
            dx_call: String::new(),
            ap: true,
            fading: None,
        }
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

fn bins_for(options: &Options) -> usize {
    let setup = options.setup;
    let top = (f64::from(options.f_high_hz) + setup.bandwidth_hz()) / setup.baud();
    // 64 bin oltre l'ultimo tono per la metrica dell'evanescenza
    ((top.ceil() as usize) + M + 4).min(setup.nsps() / 2)
}

/// Candidati (bin del tono 0, ritardo in passi, sincronismo) dagli spettri fatti a passi di 1/`per_sym` di simbolo.
fn find_candidates(sp: &[Vec<f32>], options: &Options, per_sym: usize) -> Vec<(usize, i64, f32)> {
    let setup = options.setup;
    let nbins = sp.first().map_or(0, Vec::len);
    if sp.len() < per_sym * NSYM || nbins == 0 {
        return Vec::new();
    }
    let step = setup.nsps() / per_sym;
    let ps = per_sym as i64;
    let baud = setup.baud();
    let spb = 1usize << setup.submode;
    // rumore di ogni bin: mediana nel tempo (i segnali occupano pochi bin per volta)
    let noise: Vec<f32> = (0..nbins)
        .map(|i| {
            let mut col: Vec<f32> = sp.iter().map(|r| r[i]).collect();
            col.sort_by(f32::total_cmp);
            col[col.len() / 2].max(f32::MIN_POSITIVE)
        })
        .collect();
    let start0 = setup.start_s() * FS / step as f64;
    let lag_lo = (start0 - 2.0 * FS / step as f64).floor().max(0.0) as i64;
    let lag_hi = ((start0 + 2.0 * FS / step as f64).ceil() as i64).min(sp.len() as i64 - ps * NSYM as i64);
    let ia = ((f64::from(options.f_low_hz) / baud).floor() as usize).max(1);
    let ib = ((f64::from(options.f_high_hz) / baud).ceil() as usize).min(nbins.saturating_sub(65 * spb + M + 1));
    let mut found: Vec<(usize, i64, f32)> = Vec::new();
    for i in ia..=ib {
        let mut best = (0i64, f32::MIN);
        for lag in lag_lo..=lag_hi {
            let s: f32 = ISYNC.iter().map(|&p| sp[(lag + ps * (p as i64 - 1)) as usize][i] / noise[i]).sum::<f32>() / ISYNC.len() as f32;
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
    candidates
}

/// Energie larghe dei 63 simboli di dati: per ognuno 64·(2+spb) bin a partire da 64 bin sotto il primo tono dei dati.
/// `symbol(s, bin)` da' l'energia del bin nel simbolo `s` (da 0 a 84).
fn wide_energies(i: usize, spb: usize, mut symbol: impl FnMut(usize, usize) -> f32) -> Vec<f32> {
    let wbins = M * (2 + spb);
    let lo = (i + spb) as i64 - M as i64;
    let mut out = vec![0f32; NTX * wbins];
    let mut k = 0;
    for s in 0..NSYM {
        if ISYNC.contains(&(s + 1)) {
            continue;
        }
        for (b, o) in out[k * wbins..(k + 1) * wbins].iter_mut().enumerate() {
            let bin = lo + b as i64;
            if bin >= 0 {
                *o = symbol(s, bin as usize);
            }
        }
        k += 1;
    }
    out
}

/// Le energie dei 64 toni (63 x 64) dalle energie larghe.
fn narrow(wide: &[f32], spb: usize) -> Vec<f32> {
    let wbins = M * (2 + spb);
    let mut out = vec![0f32; NTX * M];
    for k in 0..NTX {
        for t in 0..M {
            out[k * M + t] = wide[k * wbins + M + t * spb];
        }
    }
    out
}

/// Esito della decodifica di un candidato.
struct Hit {
    msg: [i32; NK],
    ap: u8,
    b90_hz: Option<f32>,
}

/// Prova un candidato: Bessel, poi l'evanescenza rapida, poi le stesse con ogni ipotesi AP (`q65_dec_q012`).
fn decode_wide(wide: &[f32], options: &Options, aps: &[Ap], gain: f32) -> Option<Hit> {
    let setup = options.setup;
    let spb = 1usize << setup.submode;
    let bessel = bessel_intrinsics(&narrow(wide, spb));
    let fading: Vec<(f32, Vec<f32>)> = match options.fading {
        Some(model) => b90_sweep(setup).map(|b| (b * setup.baud() as f32, fading_intrinsics(wide, spb, b, model, gain))).collect(),
        None => Vec::new(),
    };
    for ap in std::iter::once(None).chain(aps.iter().map(Some)) {
        let kind = ap.map_or(0, |a| a.kind);
        if let Some(msg) = qra_decode(&bessel, ap) {
            return Some(Hit { msg, ap: kind, b90_hz: None });
        }
        for (b90, pix) in &fading {
            if let Some(msg) = qra_decode(pix, ap) {
                return Some(Hit { msg, ap: kind, b90_hz: Some(*b90) });
            }
        }
    }
    None
}

/// S/N in 2500 Hz dalla parola decodificata (`q65_esnodb`).
fn snr_of(wide: &[f32], msg: &[i32; NK], setup: Setup) -> f64 {
    let energies = narrow(wide, 1usize << setup.submode);
    let cw = encode_symbols(msg);
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
    esno - 10.0 * (2500.0 / setup.baud()).log10()
}

/// Media dei periodi (`q65_avg` di WSJT-X): somma gli spettri dei periodi con la stessa parita' finche' un segnale debole si decodifica.
#[derive(Debug, Default, Clone)]
pub struct Averager {
    setup: Option<Setup>,
    /// Per parita' (periodo pari, dispari): spettri sommati e numero di periodi.
    sums: [Option<(Vec<Vec<f32>>, u32)>; 2],
}

impl Averager {
    pub fn new() -> Averager {
        Averager::default()
    }

    /// Svuota la media («Clear Avg» di WSJT-X).
    pub fn clear(&mut self) {
        self.sums = [None, None];
    }

    /// Periodi sommati per la parita' di `period_index`.
    pub fn count(&self, period_index: u64) -> u32 {
        self.sums[(period_index % 2) as usize].as_ref().map_or(0, |s| s.1)
    }

    fn add(&mut self, setup: Setup, period_index: u64, sp: &[Vec<f32>]) -> u32 {
        if self.setup != Some(setup) {
            self.clear();
            self.setup = Some(setup);
        }
        let slot = &mut self.sums[(period_index % 2) as usize];
        match slot {
            Some((sum, n)) if sum.len() == sp.len() && sum.first().map(Vec::len) == sp.first().map(Vec::len) => {
                for (a, b) in sum.iter_mut().zip(sp) {
                    a.iter_mut().zip(b).for_each(|(x, y)| *x += y);
                }
                *n += 1;
                *n
            }
            _ => {
                *slot = Some((sp.to_vec(), 1));
                1
            }
        }
    }
}

/// Decodifica un periodo (audio a 12 kHz dall'inizio del periodo).
pub fn decode(audio: &[f32], options: &Options, hashes: &mut CallHashes) -> Vec<Decoded> {
    decode_inner(audio, options, hashes, None)
}

/// Come [`decode`], e in piu' somma il periodo nella media della sua parita' (`period_index` = numero d'ordine del periodo) e prova
/// a decodificare la somma. Dopo una decodifica dalla media, la media di quella parita' si svuota.
pub fn decode_averaged(audio: &[f32], options: &Options, hashes: &mut CallHashes, averager: &mut Averager, period_index: u64) -> Vec<Decoded> {
    decode_inner(audio, options, hashes, Some((averager, period_index)))
}

#[allow(clippy::too_many_arguments)]
fn push_hit(out: &mut Vec<Decoded>, hit: &Hit, wide: &[f32], setup: Setup, i: usize, start: i64, sync: f32, averaged: u32, hashes: &mut CallHashes) -> bool {
    let Ok(text) = unpack(&symbols_to_bits(&hit.msg), hashes) else { return false };
    if out.iter().any(|d| d.text == text) {
        return false;
    }
    out.push(Decoded {
        text,
        freq_hz: (i as f64 * setup.baud()) as f32,
        dt_s: (start as f64 / FS - setup.start_s()) as f32,
        snr_db: (snr_of(wide, &hit.msg, setup).round() as i32).clamp(-35, 25),
        sync,
        ap: hit.ap,
        averaged,
        b90_hz: hit.b90_hz,
    });
    true
}

fn decode_inner(audio: &[f32], options: &Options, hashes: &mut CallHashes, averager: Option<(&mut Averager, u64)>) -> Vec<Decoded> {
    let setup = options.setup;
    let nsps = setup.nsps();
    let spb = 1usize << setup.submode;
    let half = nsps / 2;
    let sp = spectra(audio, nsps, bins_for(options), half);
    let aps = if options.ap { ap_list(&options.my_call, &options.dx_call, hashes) } else { Vec::new() };
    let mut out: Vec<Decoded> = Vec::new();

    let fft = FftPlanner::<f32>::new().plan_fft_forward(nsps);
    let mut buf = vec![Complex::new(0f32, 0.0); nsps];
    let mut spec = vec![0f32; half];
    for (i, lag, sync) in find_candidates(&sp, options, 2) {
        // si prova il ritardo trovato e i quarti di simbolo vicini
        for off in [0i64, -1, 1, -2, 2] {
            let start = lag * half as i64 + off * (nsps as i64 / 4);
            let mut last = usize::MAX;
            let wide = wide_energies(i, spb, |s, bin| {
                if s != last {
                    last = s;
                    let a = start + (s * nsps) as i64;
                    for (j, b) in buf.iter_mut().enumerate() {
                        let idx = a + j as i64;
                        let v = if idx >= 0 && (idx as usize) < audio.len() { audio[idx as usize] } else { 0.0 };
                        *b = Complex::new(v, 0.0);
                    }
                    fft.process(&mut buf);
                    spec.iter_mut().zip(&buf).for_each(|(o, c)| *o = c.norm_sqr());
                }
                spec.get(bin).copied().unwrap_or(0.0)
            });
            if let Some(hit) = decode_wide(&wide, options, &aps, 1.0) {
                push_hit(&mut out, &hit, &wide, setup, i, start, sync, 1, hashes);
                break;
            }
        }
    }

    if let Some((averager, index)) = averager {
        // per la media spettri a quarti di simbolo: si somma solo il ritardo dei passi, senza ritocchi dopo
        let quarter = nsps / 4;
        let n = averager.add(setup, index, &spectra(audio, nsps, bins_for(options), quarter));
        if n >= 2
            && let Some((sum, _)) = averager.sums[(index % 2) as usize].as_ref()
        {
            // le energie sommate non seguono piu' la distribuzione di un periodo solo: si usa la metrica lineare nelle energie
            // dell'evanescenza (la lorentziana di WSJT-X se non ne e' scelta un'altra), pesata con il numero dei periodi
            // il rumore delle energie sommate oscilla meno: la soglia del sincronismo scende con la radice dei periodi
            let min_sync = 1.0 + (options.min_sync - 1.0) / (n as f32).sqrt();
            let avg_options = Options { fading: Some(options.fading.unwrap_or(Fading::Lorentzian)), min_sync, ..options.clone() };
            let options = &avg_options;
            let mut any = false;
            for (i, lag, sync) in find_candidates(sum, options, 4) {
                let wide = wide_energies(i, spb, |s, bin| sum[(lag + 4 * s as i64) as usize].get(bin).copied().unwrap_or(0.0));
                if let Some(hit) = decode_wide(&wide, options, &aps, n as f32) {
                    any |= push_hit(&mut out, &hit, &wide, setup, i, lag * quarter as i64, sync, n, hashes);
                }
            }
            if any {
                averager.sums[(index % 2) as usize] = None;
            }
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
        assert_eq!(qra_decode(&bessel_intrinsics(&energies), None), Some(msg));
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

    /// Ogni periodo con ogni sottomodo, con rumore: l'onda generata torna indietro.
    #[test]
    fn every_period_and_submode_goes_round_trip_with_noise() {
        for p in [15, 30, 60, 120, 300] {
            for s in 0..=4u8 {
                let setup = Setup::new(p, s).expect("modo");
                // con i sottomodi larghi il segnale deve stare nella banda audio
                let f0 = if setup.bandwidth_hz() > 1500.0 { 300.0 } else { 1000.0 };
                // Q65-15E e' largo 6,9 kHz: non sta nell'audio a 12 kHz
                if f0 + setup.bandwidth_hz() > 5_500.0 {
                    continue;
                }
                let audio = period("IU3ABC K1ABC FN42", f0, 0.2, setup, 0.12, 1.0);
                let o = Options { setup, f_low_hz: f0 as f32 - 50.0, f_high_hz: f0 as f32 + 50.0, ap: false, ..Options::default() };
                let d = decode(&audio, &o, &mut CallHashes::default());
                let hit = d.iter().find(|d| d.text == "IU3ABC K1ABC FN42").unwrap_or_else(|| panic!("Q65-{p}{}: {d:?}", (b'A' + s) as char));
                assert!((hit.freq_hz - f0 as f32).abs() <= setup.baud() as f32 * 1.5, "{p} {s}: {hit:?}");
                assert_eq!((hit.ap, hit.averaged), (0, 1));
            }
        }
    }

    /// Un segnale troppo debole per un periodo solo si decodifica sommando i periodi della stessa parita'.
    #[test]
    fn averaging_decodes_a_signal_too_weak_for_one_period() {
        let setup = Setup::new(15, 0).expect("modo");
        let o = Options { setup, f_low_hz: 950.0, f_high_hz: 1050.0, ap: false, ..Options::default() };
        let msg = "IU3ABC K1ABC FN42";
        let wave = modulate(&tones(msg, &mut CallHashes::default()).expect("messaggio"), 1000.0, setup, 0.05);
        let make = |seed: u64| {
            let mut audio = noise(15 * 12_000, 1.0, seed);
            let at = (setup.start_s() * FS) as usize;
            audio[at..at + wave.len()].iter_mut().zip(&wave).for_each(|(a, w)| *a += w);
            audio
        };
        let mut avg = Averager::new();
        let mut decoded_at = None;
        for k in 0..8u64 {
            let audio = make(100 + k);
            let single = decode(&audio, &o, &mut CallHashes::default());
            assert!(!single.iter().any(|d| d.text == msg), "un periodo solo basta gia' (periodo {k}): alzare il rumore");
            // solo i periodi pari: la media e' per parita'
            let d = decode_averaged(&audio, &o, &mut CallHashes::default(), &mut avg, 2 * k);
            if let Some(hit) = d.iter().find(|d| d.text == msg) {
                assert!(hit.averaged >= 2, "{hit:?}");
                decoded_at = Some(k);
                break;
            }
        }
        assert!(decoded_at.is_some(), "la media non ha decodificato");
        assert_eq!(avg.count(0), 0, "dopo la decodifica la media si svuota");
    }

    /// Con il mio nominativo e il DX noti, un segnale sotto la soglia normale si decodifica con le ipotesi AP.
    #[test]
    fn ap_decodes_below_the_normal_threshold() {
        let setup = Setup::new(15, 0).expect("modo");
        let msg = "IU3ABC K1ABC FN42";
        let mut gained = 0;
        for seed in 0..6u64 {
            let wave = modulate(&tones(msg, &mut CallHashes::default()).expect("messaggio"), 1000.0, setup, 0.06);
            let mut audio = noise(15 * 12_000, 1.0, 40 + seed);
            let at = (setup.start_s() * FS) as usize;
            audio[at..at + wave.len()].iter_mut().zip(&wave).for_each(|(a, w)| *a += w);
            let base = Options { setup, f_low_hz: 950.0, f_high_hz: 1050.0, min_sync: 1.5, ..Options::default() };
            let plain = decode(&audio, &Options { ap: false, ..base.clone() }, &mut CallHashes::default());
            let with_ap = decode(&audio, &Options { my_call: "IU3ABC".into(), dx_call: "K1ABC".into(), ..base }, &mut CallHashes::default());
            let p = plain.iter().any(|d| d.text == msg);
            let a = with_ap.iter().find(|d| d.text == msg);
            assert!(!p || a.is_some(), "l'AP non deve perdere quello che si decodifica senza");
            if !p && let Some(a) = a {
                assert!(a.ap >= 2, "{a:?}");
                gained += 1;
            }
        }
        assert!(gained >= 2, "l'AP ha recuperato solo {gained} periodi su 6");
    }

    /// Con le ipotesi AP accese (anche il messaggio intero) il rumore non diventa un messaggio.
    #[test]
    fn ap_and_fading_do_not_invent_messages_in_noise() {
        let setup = Setup::new(15, 0).expect("modo");
        let o = Options { setup, my_call: "IU3ABC".into(), dx_call: "K1ABC".into(), fading: Some(Fading::Lorentzian), max_candidates: 8, ..Options::default() };
        for seed in 0..3 {
            let audio = noise(15 * 12_000, 1.0, 70 + seed);
            assert!(decode(&audio, &o, &mut CallHashes::default()).is_empty(), "seme {seed}");
        }
    }

    /// Un segnale allargato dall'evanescenza (EME): la metrica di Bessel non basta, quella lorentziana si'.
    #[test]
    fn the_fading_metric_decodes_a_spread_signal() {
        let setup = Setup::new(15, 2).expect("modo"); // Q65-15C: 26,7 Hz fra i toni
        let msg = "IU3ABC K1ABC FN42";
        let tn = tones(msg, &mut CallHashes::default()).expect("messaggio");
        // il segnale e' la somma di molte copie con frequenza e fase casuali (allargamento di circa 30 Hz)
        const SPREAD: f64 = 30.0;
        const AMP: f32 = 0.15;
        let mut audio = noise(15 * 12_000, 1.0, 3);
        let mut x = 99u64;
        let mut rnd = || {
            x ^= x << 13;
            x ^= x >> 7;
            x ^= x << 17;
            (x >> 11) as f64 / (1u64 << 53) as f64
        };
        let at = (setup.start_s() * FS) as usize;
        for _ in 0..24 {
            let df = (rnd() - 0.5) * SPREAD;
            let wave = modulate(&tn, 800.0 + df, setup, AMP);
            let shift = (rnd() * 1000.0) as usize;
            for (k, w) in wave.iter().enumerate() {
                // la fase cambia nel tempo: un'altra copia per ogni simbolo
                let flip = if (k / setup.nsps() + shift).is_multiple_of(2) { 1.0 } else { -1.0 };
                audio[at + k] += w * flip / 24f32.sqrt();
            }
        }
        let o = Options { setup, f_low_hz: 750.0, f_high_hz: 850.0, ap: false, min_sync: 1.5, ..Options::default() };
        let fading = decode(&audio, &Options { fading: Some(Fading::Lorentzian), ..o.clone() }, &mut CallHashes::default());
        let plain = decode(&audio, &o, &mut CallHashes::default());
        let hit = fading.iter().find(|d| d.text == msg).expect("decodificato con l'evanescenza");
        assert!(hit.b90_hz.is_some(), "{hit:?}");
        assert!(!plain.iter().any(|d| d.text == msg), "con Bessel non dovrebbe bastare: allargare di piu'");
    }

    /// Sommando solo rumore non esce nessun messaggio.
    #[test]
    fn averaging_noise_invents_nothing() {
        let setup = Setup::new(15, 0).expect("modo");
        let o = Options { setup, my_call: "IU3ABC".into(), dx_call: "K1ABC".into(), ..Options::default() };
        let mut avg = Averager::new();
        for k in 0..6u64 {
            let audio = noise(15 * 12_000, 1.0, 200 + k);
            assert!(decode_averaged(&audio, &o, &mut CallHashes::default(), &mut avg, 2 * k).is_empty(), "periodo {k}");
        }
        assert_eq!(avg.count(0), 6);
    }

    #[test]
    fn the_fading_tables_have_the_lengths_of_wsjtx() {
        assert_eq!(crate::q65fading::GAUSS[0].len(), 2);
        assert_eq!(crate::q65fading::GAUSS[63].len(), 65);
        assert!(crate::q65fading::LORENTZ.iter().all(|t| !t.is_empty() && t.len() <= 65));
    }

    #[test]
    fn pure_noise_decodes_nothing() {
        let setup = Setup::new(30, 0).expect("modo");
        let audio = noise(30 * 12_000, 1.0, 9);
        assert!(decode(&audio, &Options { setup, ..Options::default() }, &mut CallHashes::default()).is_empty());
    }
}
