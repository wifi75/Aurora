//! Q65: 65-FSK, 85 simboli (22 di sincronismo sul tono 0), messaggio di 77 bit come FT8 con CRC-12 e codice QRA (15,65) su GF(64)
//! decodificato a passaggio di messaggi. Dai sorgenti di WSJT-X (GPL 3, codice di Nico Palermo IV3NWV): `genq65.f90`, `q65.c`,
//! `qracodes.c`, `npfwht.c`, `pdmath.c`, `qra15_65_64_irr_e23.c`, `q65sim.f90`.
//!
//! Periodi da 15, 30, 60, 120 e 300 s; sottomodi da A a E (distanza fra i toni = velocita' x 1, 2, 4, 8, 16).
//! La ricezione usa la metrica di Bessel (`qra_mfskbesselmetric`) e, a scelta, quella dell'evanescenza rapida per EME e scatter
//! (`q65_intrinsics_fastfading`, tabelle gaussiane e lorentziane); prova le ipotesi AP del QSO (`q65_ap.f90`) e puo' sommare i periodi
//! della stessa parita' per i segnali troppo deboli (`q65_avg`).
//!
//! Un periodo si decodifica come `q65_decode.f90` e `q65.f90`: correlazione sui 22 simboli di sincronismo negli spettri a 1/8 di
//! simbolo (`q65_ccf_22`), lista dei messaggi del QSO sulla frequenza di ricezione se il DX e' noto (`q65_dec_q3`), poi per ogni
//! candidato la ricerca fine di `q65_loops` (mezzi bin, sedicesimi di simbolo, larghezze B90) sulle energie normalizzate di `spec64`.

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
        // blocchi di 2h: la prima meta' e la seconda si combinano a coppie (il compilatore le vettorizza)
        for block in v.chunks_exact_mut(2 * h) {
            let (lo, hi) = block.split_at_mut(h);
            for (a, b) in lo.iter_mut().zip(hi.iter_mut()) {
                let (x, y) = (*a, *b);
                *a = x + y;
                *b = x - y;
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

/// Soglia della verosimiglianza della decodifica da lista (`PLOG_MIN` di `q65.f90`).
const PLOG_MIN: f32 = -242.0;

/// Un messaggio della lista del QSO: i 13 simboli di messaggio e la parola di codice trasmessa.
struct ListWord {
    msg: [i32; NK],
    cw: [i32; NTX],
}

/// `stdcall`: nominativo standard (prefisso con almeno una lettera, cifra dell'area al secondo o terzo posto, al piu' tre lettere dopo).
fn std_call(call: &str) -> bool {
    let c = call.as_bytes();
    let n = c.len();
    let Some(iarea) = (1..n).rev().find(|&i| c[i].is_ascii_digit()) else { return false };
    let npdig = c[..iarea].iter().filter(|b| b.is_ascii_digit()).count();
    let nplet = c[..iarea].iter().filter(|b| b.is_ascii_uppercase()).count();
    let nslet = c[iarea + 1..].iter().filter(|b| b.is_ascii_uppercase()).count();
    // `iarea` qui parte da 0: in WSJT-X l'area sta fra la seconda e la terza posizione
    (1..=2).contains(&iarea) && nplet > 0 && npdig < iarea && nslet <= 3
}

/// `q65_stdcall` (`q65_set_list.f90:97-114`): come `stdcall`, ma un «/R» o «/P» finale (nominativo di piu' di 3 caratteri) non
/// rende il nominativo non standard: si manda ancora come messaggio di tipo 1 con il bit ipa/ipb acceso.
fn q65_std_call(call: &str) -> bool {
    let n = call.len();
    if n > 3 && (call.ends_with("/R") || call.ends_with("/P")) { std_call(&call[..n - 2]) } else { std_call(call) }
}

/// Le righe da 1 a 206 di `q65_set_list`: i due nominativi, RRR, RR73, 73, il CQ del DX, il locatore e i rapporti da -50 a +49 con e
/// senza R.
fn qso_list_text(i: i32, my: &str, dx: &str, grid: &str, my_std: bool, dx_std: bool) -> String {
    let pair = if !my_std {
        if i == 1 || i >= 6 { format!("<{my}> {dx}") } else if (2..=4).contains(&i) { format!("{my} <{dx}>") } else { format!("{my} {dx}") }
    } else if !dx_std {
        if i <= 4 || i == 6 { format!("<{my}> {dx}") } else if i >= 7 { format!("{my} <{dx}>") } else { format!("{my} {dx}") }
    } else {
        format!("{my} {dx}")
    };
    match i {
        2 => format!("{pair} RRR"),
        3 => format!("{pair} RR73"),
        4 => format!("{pair} 73"),
        5 if dx_std => format!("CQ {dx} {grid}").trim_end().to_string(),
        5 => format!("CQ {dx}"),
        6 if dx_std && !grid.is_empty() => format!("{pair} {grid}"),
        7.. => {
            let snr = -50 + (i - 7) / 2;
            if i % 2 == 1 { format!("{pair} {snr:+03}") } else { format!("{pair} R{snr:+03}") }
        }
        _ => pair,
    }
}

/// `q65_set_list`: i messaggi possibili del QSO con il DX, per la decodifica da lista (`q65_dec_q3`). Sono 206 (`qso_list_text`);
/// se il DX e' un nominativo standard con «/R» o «/P» (`q65_set_list.f90:19-29`) si tengono anche quelli che dava la vecchia strada
/// del nominativo non standard: il CQ senza locatore (207) e, se il mio nominativo e' standard, «<MIO> DX» con niente, RRR, RR73 o
/// 73 (208-211) e «MIO <DX>» con i rapporti con e senza R (212-411, come 7-206): 411 in tutto.
fn qso_list(my_call: &str, dx_call: &str, dx_grid: &str, hashes: &CallHashes) -> Vec<ListWord> {
    let (my, dx) = (my_call.trim().to_uppercase(), dx_call.trim().to_uppercase());
    let grid: String = dx_grid.trim().to_uppercase().chars().take(4).collect();
    if my.is_empty() || dx.is_empty() {
        return Vec::new();
    }
    let (my_std, dx_std) = (q65_std_call(&my), q65_std_call(&dx));
    let n = dx.len();
    let mut ncw = 206;
    if dx_std && n > 3 && (dx.ends_with("/R") || dx.ends_with("/P")) {
        ncw = if my_std { 411 } else { 207 };
    }
    let mut out = Vec::new();
    for i in 1..=ncw {
        let text = match i {
            ..=206 => qso_list_text(i, &my, &dx, &grid, my_std, dx_std),
            207 => format!("CQ {dx}"),
            208 => format!("<{my}> {dx}"),
            209 => format!("<{my}> {dx} RRR"),
            210 => format!("<{my}> {dx} RR73"),
            211 => format!("<{my}> {dx} 73"),
            _ => {
                // `isnr = -50 + (i-205-7)/2`, rapporto semplice con `i-205` dispari
                let k = i - 205;
                let snr = -50 + (k - 7) / 2;
                if k % 2 == 1 { format!("{my} <{dx}> {snr:+03}") } else { format!("{my} <{dx}> R{snr:+03}") }
            }
        };
        let Ok(bits) = pack(&text, &mut hashes.clone()) else { continue };
        let msg = bits_to_symbols(&bits);
        out.push(ListWord { msg, cw: encode_symbols(&msg) });
    }
    out
}

/// Capienza della lista dei chiamanti dei contest (`Q65_CALLER_CAPACITY` di `q65_limits.h`).
pub const MAX_CALLERS: usize = 50;

/// Una stazione sentita in contest (`type q3list` di `types.f90`): nominativo (6 caratteri al piu', come in WSJT-X), locatore a 4
/// caratteri, ora dell'ultimo ascolto in secondi e frequenza.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Caller {
    pub call: String,
    pub grid: String,
    pub nsec: i64,
    pub nfreq: i32,
}

/// La lista dei chiamanti dei contest di `q65_callers.F90` (NA VHF, WW Digi, ARRL Digi: `ncontest = 1`), tenuta in memoria come fa
/// il motore di WSJT-X (`engine_active`); ne nasce la lista dei messaggi del contest di `q65_set_list2`.
#[derive(Debug, Clone, Default)]
pub struct Callers {
    list: Vec<Caller>,
}

/// `valid_grid` di `q65_callers.F90` (e `isgrid` di `q65_set_list2.f90`): locatore a 4 caratteri, non «RR73».
fn valid_grid(g: &str) -> bool {
    let b = g.as_bytes();
    b.len() == 4 && (b'A'..=b'R').contains(&b[0]) && (b'A'..=b'R').contains(&b[1]) && b[2].is_ascii_digit() && b[3].is_ascii_digit() && g != "RR73"
}

impl Callers {
    pub fn list(&self) -> &[Caller] {
        &self.list
    }

    /// `q65_expire_callers`: toglie chi non si sente da piu' di 24 ore.
    pub fn expire(&mut self, now: i64) {
        self.list.retain(|c| now - c.nsec <= 86_400);
    }

    /// `q65_record_caller`: dal messaggio decodificato «X CALL GRID» (o «X CALL R GRID») ricorda CALL e GRID; se CALL c'e' gia' ne
    /// aggiorna ora e frequenza. Messaggi con «/» si ignorano; piena la lista, esce il piu' vecchio.
    pub fn record(&mut self, freq: i32, msg: &str, now: i64) {
        self.expire(now);
        if msg.contains('/') || !msg.is_ascii() {
            return;
        }
        // come in Fortran: il testo a 37 caratteri, posizioni da 1
        let mut text: String = format!("{msg:<37}").chars().take(37).collect();
        if let Some(i0) = text.find(" R ").map(|i| i + 1)
            && i0 >= 7
        {
            text = format!("{}{}", &text[..i0], &text[i0 + 2..]);
            text = format!("{text:<37}");
        }
        let Some(i1) = text.find(' ').map(|i| i + 1) else { return };
        if !(4..=13).contains(&i1) {
            return;
        }
        let Some(i2) = text[i1..].find(' ').map(|i| i + 1 + i1) else { return };
        if i2 <= i1 + 1 || i2 + 4 > text.len() {
            return;
        }
        let call: String = text[i1..i2 - 1].chars().take(6).collect();
        let grid = text[i2..i2 + 4].to_string();
        if let Some(c) = self.list.iter_mut().find(|c| c.call == call) {
            c.nsec = now;
            c.nfreq = freq;
            return;
        }
        if !valid_grid(&grid) {
            return;
        }
        if self.list.len() == MAX_CALLERS
            && let Some(oldest) = self.list.iter().enumerate().min_by_key(|(_, c)| c.nsec).map(|(i, _)| i)
        {
            self.list.remove(oldest);
        }
        self.list.push(Caller { call, grid, nsec: now, nfreq: freq });
    }

    /// `q65_remove_caller`.
    pub fn remove(&mut self, call: &str) {
        let call: String = call.trim().to_uppercase().chars().take(6).collect();
        if let Some(i) = self.list.iter().position(|c| c.call == call) {
            self.list.remove(i);
        }
    }
}

/// `q65_set_list2`: la lista dei messaggi dei contest. Per ogni chiamante (e per il DX, se ha nominativo standard e locatore e non e'
/// gia' fra i chiamanti) «MIO CALL GRID», «MIO CALL R GRID», RRR, RR73 e 73, ognuno con il 78° bit a 0 e a 1 (il chiamante puo' aver
/// copiato o no il bit dell'ultima trasmissione: `iflag` di `genq65`).
fn contest_list(my_call: &str, dx_call: &str, dx_grid: &str, callers: &[Caller], hashes: &CallHashes) -> Vec<ListWord> {
    let my = my_call.trim().to_uppercase();
    if my.is_empty() || callers.len() > MAX_CALLERS {
        return Vec::new();
    }
    let dx: String = dx_call.trim().to_uppercase().chars().take(6).collect();
    let grid: String = dx_grid.trim().to_uppercase().chars().take(4).collect();
    let mut who: Vec<(String, String)> = callers.iter().map(|c| (c.call.clone(), c.grid.clone())).collect();
    if std_call(dx_call.trim()) && valid_grid(&grid) && !callers.iter().any(|c| c.call == dx) {
        who.push((dx, grid));
    }
    let mut out = Vec::new();
    for (c6, g4) in &who {
        for k in 1..=5 {
            let pair = format!("{my} {c6}");
            let text = match k {
                1 => format!("{pair} {g4}"),
                2 => format!("{pair} R {g4}"),
                3 => format!("{pair} RRR"),
                4 => format!("{pair} RR73"),
                _ => format!("{pair} 73"),
            };
            let Ok(bits) = pack(text.trim_end(), &mut hashes.clone()) else { continue };
            let msg = bits_to_symbols(&bits);
            for iflag in 0..=1 {
                let mut m = msg;
                m[12] |= iflag;
                out.push(ListWord { msg: m, cw: encode_symbols(&m) });
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
    /// Ipotesi AP usata (0 = nessuna, poi i tipi di `q65_ap.f90`: 1 CQ, 2 mio nominativo, 3 con il DX o dalla lista dei messaggi del
    /// QSO, il «q3» di WSJT-X, 4 RRR, 5 73, 6 RR73).
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
    /// «Auto Clear Avg after decode» di WSJT-X (bit 128 di `ndepth`, `q65_decode.f90:605-606,717-718`): un messaggio nuovo entro
    /// `rx_tol_hz` dalla frequenza di ricezione svuota la media della sua parita'. In WSJT-X e' spento di fabbrica (`NDepth` vale 3,
    /// `mainwindow_settings.cpp:529`, letto con lo stesso 3 nel `wsjtx.exe` compilato): spento, la media resta finche' non si preme
    /// «Clear Avg» o cambiano periodo e sottomodo.
    pub auto_clear_avg: bool,
    /// «Enable averaging» del menu Decode di WSJT-X (bit 16 di `ndepth`, `iand(effort,16)` di `q65_decode.f90:525,742`): spento, la media
    /// dei periodi si accumula lo stesso (`q65_symspec` la aggiorna sempre, `q65.f90:630-635`) ma non si prova a decodificarla. In WSJT-X
    /// e' spento di fabbrica (`NDepth` = 3); qui di base acceso, com'era prima che il motore lo leggesse dalle impostazioni.
    pub average_decode: bool,
    /// «Single decode» di WSJT-X (`nexp_decode & 32`): si prova solo la frequenza di ricezione (e la sua media), niente giro dei
    /// candidati (`if(single_decode .or. lagain) go to 900`, `q65_decode.f90:628`). Spento di fabbrica.
    pub single_decode: bool,
    /// Soglia dei candidati di un periodo, come `q65_ccf_22`: altezza del picco della correlazione sui 22 simboli di sincronismo
    /// sopra la mediana, in unita' della dispersione (84° meno 50° percentile). WSJT-X usa 6.
    pub sync_snr: f32,
    /// Frequenza di ricezione (`nfqso`): il picco di sincronismo piu' alto entro `rx_tol_hz` si prova sempre, anche sotto soglia.
    pub rx_hz: Option<f32>,
    /// Tolleranza attorno alla frequenza di ricezione (`ntol`).
    pub rx_tol_hz: f32,
    /// Thread per provare i candidati (1 = solo quello che chiama).
    pub threads: usize,
    /// Il mio nominativo e quello del DX per le ipotesi AP (vuoti = sconosciuti).
    pub my_call: String,
    pub dx_call: String,
    /// Locatore del DX (4 caratteri, vuoto = sconosciuto) per la lista dei messaggi del QSO (`q65_set_list`).
    pub dx_grid: String,
    /// Decodifica con le ipotesi AP (CQ sempre, le altre se ci sono i nominativi).
    pub ap: bool,
    /// Metrica dell'evanescenza rapida, provata dopo quella di Bessel su piu' larghezze B90.
    pub fading: Option<Fading>,
    /// Ritardo dell'eco lunare (`emedelay` di WSJT-X): il ritardo si cerca fino a 5,5 s dopo l'inizio nominale (4 s nel Q65-15)
    /// invece di 2 s, come `lag2` di `q65.f90` con `emedelay > 0`.
    pub eme_delay: bool,
    /// Deriva massima di frequenza (`max_drift` di WSJT-X, contributo di W3SZ), in bin (= velocita' di simbolo) per trasmissione,
    /// da 0 a 50 a passi di 5 come lo «Max Drift» di WSJT-X. Diverso da 0, `q65_ccf_22` prova le derive da -max a +max e cerca solo
    /// entro `rx_tol_hz` dalla frequenza di ricezione; a 50 c'e' in piu' l'ultimo tentativo di lista sugli spettri raddrizzati.
    pub max_drift: i32,
    /// Lista dei messaggi dei contest (`ncontest = 1`, `q65_set_list2`) al posto di quella del QSO, fatta con `callers`.
    pub contest: bool,
    /// Le stazioni sentite in contest (da [`Callers::list`]).
    pub callers: Vec<Caller>,
}

impl Default for Options {
    fn default() -> Self {
        Options {
            setup: Setup::default(),
            f_low_hz: 200.0,
            f_high_hz: 3000.0,
            max_candidates: 20,
            auto_clear_avg: false,
            average_decode: true,
            single_decode: false,
            sync_snr: 6.0,
            rx_hz: None,
            rx_tol_hz: 50.0,
            threads: 1,
            my_call: String::new(),
            dx_call: String::new(),
            dx_grid: String::new(),
            ap: true,
            fading: None,
            eme_delay: false,
            max_drift: 0,
            contest: false,
            callers: Vec::new(),
        }
    }
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

// ---------------------------------------------------------------- un periodo: sincronismo e ricerca come WSJT-X

/// Passi di tempo per simbolo negli spettri del sincronismo (`NSTEP` di `q65.f90`).
const NSTEP: usize = 8;
/// Tetto delle energie normalizzate dei simboli (`s3lim` di `q65_loops.f90`).
const S3LIM: f32 = 20.0;
/// Passi di frequenza (mezzo bin) e di tempo (1/16 di simbolo) provati attorno al candidato, e distanza massima della ricerca
/// (`idfmax`, `idtmax`, `maxdist` di `q65_loops` alla profondita' 3).
const IDFMAX: i32 = 5;
const IDTMAX: i32 = 5;
const MAXDIST: i32 = 5;

/// `pctile`: il valore al percentile `pct` (posto `nint(n·pct/100)`, da 1, dei valori ordinati). Riordina `x`.
fn pctile(x: &mut [f32], pct: f32) -> f32 {
    if x.is_empty() {
        return 1.0;
    }
    let n = x.len();
    let j = ((n as f32 * 0.01 * pct).round() as usize).clamp(1, n) - 1;
    *x.select_nth_unstable_by(j, f32::total_cmp).1
}

/// `smo121`: media mobile 1-2-1 sul posto.
fn smo121(x: &mut [f32]) {
    if x.len() < 3 {
        return;
    }
    let mut x0 = x[0];
    for i in 1..x.len() - 1 {
        let x1 = x[i];
        x[i] = 0.5 * x[i] + 0.25 * (x0 + x[i + 1]);
        x0 = x1;
    }
}

/// I passi `1, -1, 2, -2, ...` dall'indice da 1 (`ndf=idf/2`, negativo se `idf` e' pari).
fn zigzag(k: i32) -> i32 {
    let n = k / 2;
    if k % 2 == 0 { -n } else { n }
}

/// Le larghezze B90 = 1,72^ibw provate alla profondita' 3 di `q65_decode.f90` (da `ibwa-2` a `ibwa+6+2`).
fn ibw_range(setup: Setup) -> (i32, i32) {
    let ibwa = [1, 3, 8, 8, 8][usize::from(setup.submode.min(4))];
    ((ibwa - 2).max(1), (ibwa + 6 + 2).min(15))
}

/// Spettri dei simboli a passi di 1/8 di simbolo (`q65_symspec`), per il sincronismo e il rapporto S/N.
struct SyncSpectra {
    /// Riga `j` (tempo, passi da `istep` campioni) per colonna `i` (bin di frequenza da 0 a `iz`).
    s1: Vec<f32>,
    w: usize,
    jz: usize,
    istep: usize,
    /// Riga dell'inizio nominale del segnale e ritardi provati attorno (da -1 a +1 s).
    j0: i64,
    lag1: i64,
    lag2: i64,
}

impl SyncSpectra {
    /// `eme`: `emedelay > 0` di `q65_dec0`, il ritardo si cerca fino a 5,5 s (4 s nel Q65-15) e gli spettri coprono tutto il periodo.
    fn new(audio: &[f32], setup: Setup, iz: usize, eme: bool) -> SyncSpectra {
        let nsps = setup.nsps();
        let istep = nsps / NSTEP;
        let txt = (NSYM * nsps) as f64 / FS;
        let mut jz = ((txt + if nsps >= 6912 { 2.0 } else { 1.0 }) * FS / istep as f64) as usize;
        if eme {
            // `jz=(ntrperiod*12000-nsps)/istep + 1` di `q65_dec0`: ogni finestra di simbolo completa del periodo
            jz = jz.max((setup.period_s as usize * FS as usize - nsps) / istep + 1);
        }
        let mode = 1usize << setup.submode;
        let nsmo = (0.5 * (mode * mode) as f64) as usize;
        let nsmo = if nsmo <= 1 { 0 } else { nsmo };
        let w = iz + 1;
        let mut s1 = vec![0f32; jz * w];
        let fft = FftPlanner::<f32>::new().plan_fft_forward(nsps);
        let mut buf = vec![Complex::new(0f32, 0.0); nsps];
        let rows: Vec<usize> = (0..jz).step_by(2).collect();
        // due finestre reali in una FFT complessa: la prima nella parte reale, la seconda in quella immaginaria
        for pair in rows.chunks(2) {
            let (ja, jb) = (pair[0], pair.get(1).copied());
            for (k, b) in buf.iter_mut().enumerate() {
                let a = audio.get(ja * istep + k).copied().unwrap_or(0.0);
                let c = jb.and_then(|jb| audio.get(jb * istep + k).copied()).unwrap_or(0.0);
                *b = Complex::new(a, c);
            }
            fft.process(&mut buf);
            for i in 1..=iz {
                let (x, y) = (buf[i], buf[nsps - i].conj());
                s1[ja * w + i] = 0.25 * (x + y).norm_sqr();
                if let Some(jb) = jb {
                    s1[jb * w + i] = 0.25 * (x - y).norm_sqr();
                }
            }
            for &j in pair {
                for _ in 0..nsmo {
                    smo121(&mut s1[j * w + 1..(j + 1) * w]);
                }
            }
        }
        // le righe saltate sono la media delle vicine
        for j in (1..jz).step_by(2) {
            if j + 1 < jz {
                for i in 0..w {
                    s1[j * w + i] = 0.5 * (s1[(j - 1) * w + i] + s1[(j + 1) * w + i]);
                }
            }
        }
        let dtstep = istep as f64 / FS;
        let j0 = ((if nsps >= 7200 { 1.0 } else { 0.5 }) / dtstep) as i64;
        // `lag2` di `q65_dec0`: 1 s dopo l'inizio nominale, con `emedelay > 0` 5,5 s se `nsps >= 3600` e 4 s nel Q65-15
        let late = match (eme, setup.period_s) {
            (true, 15) if nsps >= 900 => 4.0,
            (true, _) if nsps >= 3600 => 5.5,
            _ => 1.0,
        };
        SyncSpectra { s1, w, jz, istep, j0, lag1: (-1.0 / dtstep) as i64, lag2: (late / dtstep + 0.9999) as i64 }
    }

    /// Gli spettri raddrizzati della deriva `drift_hz` per trasmissione (`s1w` della fase 5 di `q65_dec0`, W3SZ): nella riga `j`
    /// il bin `i` prende il valore del bin `i + nint(drift*j/(85*NSTEP*df))`; se quello esce dagli spettri resta il suo.
    fn drifted(&self, drift_hz: f64, df: f64) -> SyncSpectra {
        let mut s1 = self.s1.clone();
        let iz = self.w as i64 - 1;
        for j in 0..self.jz {
            // `w3t` parte da 1
            let shift = (drift_hz * (j + 1) as f64 / (85.0 * NSTEP as f64 * df)).round() as i64;
            for i in 1..=iz {
                let mm = i + shift;
                if (1..=iz).contains(&mm) {
                    s1[j * self.w + i as usize] = self.s1[j * self.w + mm as usize];
                }
            }
        }
        SyncSpectra { s1, ..*self }
    }

    fn at(&self, i: usize, j: i64) -> f32 {
        if j < 0 || j as usize >= self.jz || i >= self.w { 0.0 } else { self.s1[j as usize * self.w + i] }
    }

    /// `q65_ccf_22`: per ogni bin del tono 0 da `ia` a `ib` la correlazione migliore sui 22 simboli di sincronismo (tolta la media
    /// del bin) e il suo ritardo.
    /// Con `max_drift` diverso da 0 prova anche le derive da `-max_drift` a `+max_drift` bin per trasmissione (W3SZ): il simbolo `k`
    /// si legge al bin `i + nint(idrift*(k-43)/85)`. Ritorna anche la deriva migliore di ogni bin.
    fn ccf22(&self, ia: usize, ib: usize, max_drift: i32) -> (Vec<f32>, Vec<i64>, Vec<i32>) {
        let mut ccf2 = vec![0f32; self.w];
        let mut lags = vec![0i64; self.w];
        let mut drifts = vec![0i32; self.w];
        let k22 = 22.0 / self.jz as f32;
        let iz = self.w as i64 - 1;
        for i in ia..=ib {
            let avg: f32 = (0..self.jz).map(|j| self.s1[j * self.w + i]).sum::<f32>() * k22;
            let mut best = (0f32, 0i64, 0i32);
            for lag in self.lag1..=self.lag2 {
                for idrift in -max_drift..=max_drift {
                    let t: f32 = ISYNC
                        .iter()
                        .map(|&k| {
                            let ii = i as i64 + (f64::from(idrift) * (k as f64 - 43.0) / 85.0).round() as i64;
                            if ii < 1 || ii > iz { 0.0 } else { self.at(ii as usize, (NSTEP * (k - 1)) as i64 + lag + self.j0) }
                        })
                        .sum::<f32>()
                        - avg;
                    if t > best.0 {
                        best = (t, lag, idrift);
                    }
                }
            }
            ccf2[i] = best.0;
            lags[i] = best.1;
            drifts[i] = best.2;
        }
        (ccf2, lags, drifts)
    }
}

/// I candidati di `q65_ccf_22`: i 20 picchi piu' alti (massimi locali entro un tono), finche' l'altezza sopra la mediana supera
/// `sync_snr` volte la dispersione. Ritorna (bin, ritardo, altezza) in ordine di frequenza.
fn sync_candidates(ccf2: &[f32], lags: &[i64], ia: usize, ib: usize, mode: usize, options: &Options) -> Vec<(usize, i64, f32)> {
    let mut ccf2 = ccf2.to_vec();
    // come WSJT-X, anche i due bin agli estremi si azzerano
    ccf2[..=ia].fill(0.0);
    ccf2[ib..].fill(0.0);
    let mut v = ccf2[ia..=ib].to_vec();
    let ave = pctile(&mut v, 50.0);
    let rms = pctile(&mut v, 84.0) - ave;
    if rms <= 0.0 {
        return Vec::new();
    }
    let mut order: Vec<usize> = (ia..=ib).collect();
    order.sort_by(|&a, &b| ccf2[b].total_cmp(&ccf2[a]));
    let mut out = Vec::new();
    for &i in order.iter().take(20) {
        let (i3, i4) = (i.saturating_sub(mode).max(1), (i + mode).min(ccf2.len() - 1));
        if ccf2[i3..=i4].iter().any(|&c| c > ccf2[i]) {
            continue;
        }
        let snr = (ccf2[i] - ave) / rms;
        if snr < options.sync_snr {
            break;
        }
        out.push((i, lags[i], snr));
        if out.len() >= options.max_candidates.min(20) {
            break;
        }
    }
    out.sort_by_key(|c| c.0);
    out
}

/// `q65_bzap`: le colonne che fanno da massimo in piu' di 15 simboli sono portanti, non toni: si spianano a 1.
fn bzap(s3: &mut [f32], ll: usize) {
    let mut hist = vec![0u32; ll];
    for row in s3.chunks_exact(ll) {
        let mut best = 0;
        for (i, &v) in row.iter().enumerate() {
            if v > row[best] {
                best = i;
            }
        }
        hist[best] += 1;
    }
    if hist.iter().any(|&h| h > 15) {
        for row in s3.chunks_exact_mut(ll) {
            for (v, &h) in row.iter_mut().zip(&hist) {
                if h > 15 {
                    *v = 1.0;
                }
            }
        }
    }
}

/// Equalizzazione della banda passante di `spec64`: ogni colonna si divide per il suo 45° percentile, lisciato su 50 bin.
fn equalize(s3: &mut [f32], ll: usize) {
    let mut col = vec![0f32; NTX];
    let x0: Vec<f32> = (0..ll)
        .map(|i| {
            col.iter_mut().enumerate().for_each(|(n, c)| *c = s3[n * ll + i]);
            pctile(&mut col, 45.0)
        })
        .collect();
    const NH: usize = 25;
    let mut xb = vec![0f32; ll];
    let head = x0[..NH - 1].iter().sum::<f32>() / (NH - 1) as f32;
    let tail = x0[ll - NH..].iter().sum::<f32>() / (NH - 1) as f32;
    xb[..NH - 1].fill(head);
    xb[ll - NH..].fill(tail);
    for i in NH - 1..=ll - NH - 1 {
        xb[i] = x0[i + 1 - NH..=i + NH].iter().sum::<f32>() / (2 * NH + 1) as f32;
    }
    for row in s3.chunks_exact_mut(ll) {
        row.iter_mut().zip(&xb).for_each(|(v, b)| *v /= b + 0.001);
    }
}

/// Cio' che serve per provare i candidati di un periodo: lo spettro di tutto il periodo, da cui si ritaglia la banda di ogni
/// candidato (al posto di `ana64` e `twkfreq`), e le FFT per simbolo.
struct Searcher<'a> {
    options: &'a Options,
    aps: &'a [Ap],
    sp: &'a SyncSpectra,
    spectrum: Vec<Complex<f32>>,
    /// Bin dello spettro del periodo per ogni bin di simbolo.
    ratio: usize,
    /// Campioni per simbolo dopo il ritaglio (multiplo di 16).
    nsd: usize,
    inverse: std::sync::Arc<dyn rustfft::Fft<f32>>,
    sym_fft: std::sync::Arc<dyn rustfft::Fft<f32>>,
}

impl<'a> Searcher<'a> {
    fn new(audio: &[f32], options: &'a Options, aps: &'a [Ap], sp: &'a SyncSpectra) -> Searcher<'a> {
        let setup = options.setup;
        let nsps = setup.nsps();
        let ll = M * (2 + (1usize << setup.submode));
        // periodo allungato a un numero intero di simboli; la banda ritagliata ha 64 bin di margine attorno ai 64·(2+spb) usati
        let ratio = audio.len().div_ceil(nsps).max(NSYM);
        let npad = ratio * nsps;
        let nsd = (ll + 64).div_ceil(16) * 16;
        let mut planner = FftPlanner::<f32>::new();
        let mut spectrum: Vec<Complex<f32>> = (0..npad).map(|k| Complex::new(audio.get(k).copied().unwrap_or(0.0), 0.0)).collect();
        planner.plan_fft_forward(npad).process(&mut spectrum);
        spectrum.truncate(npad / 2);
        let inverse = planner.plan_fft_inverse(ratio * nsd);
        let sym_fft = planner.plan_fft_forward(nsd);
        Searcher { options, aps, sp, spectrum, ratio, nsd, inverse, sym_fft }
    }

    fn setup(&self) -> Setup {
        self.options.setup
    }

    /// Il segnale complesso della banda del candidato a `f0`, con `nsd` campioni per simbolo; ritorna anche la frequenza che
    /// finisce a 0 Hz (quella di `f0` arrotondata alla risoluzione dello spettro del periodo).
    fn band(&self, f0: f64) -> (Vec<Complex<f32>>, f64) {
        let setup = self.setup();
        let mode = 1i64 << setup.submode;
        let nd = self.ratio * self.nsd;
        let df_big = setup.baud() / self.ratio as f64;
        let b0 = (f0 / df_big).round() as i64;
        let ll = M as i64 * (2 + mode);
        // centro dei bin di simbolo usati: da -64+spb a LL-65+spb attorno al tono 0
        let center = (ll - 129 + 2 * mode) as f64 / 2.0;
        let olo = ((center - self.nsd as f64 / 2.0) * self.ratio as f64).round() as i64;
        let mut z = vec![Complex::new(0f32, 0.0); nd];
        for o in olo..olo + nd as i64 {
            let b = b0 + o;
            if b > 0 && (b as usize) < self.spectrum.len() {
                z[o.rem_euclid(nd as i64) as usize] = self.spectrum[b as usize];
            }
        }
        self.inverse.process(&mut z);
        (z, b0 as f64 * df_big)
    }

    /// `spec64` + la normalizzazione di `q65_loops`: energie dei 63 simboli di dati (64·(2+spb) bin, il tono 0 dei dati a 64 bin
    /// dall'inizio), equalizzate, divise per il 40° percentile, con il tetto `S3LIM` e senza le portanti.
    fn energies(&self, y: &[Complex<f32>], d0: i64) -> Vec<f32> {
        let mode = 1i64 << self.setup().submode;
        let ll = M * (2 + mode as usize);
        let nsd = self.nsd;
        let mut buf = vec![Complex::new(0f32, 0.0); nsd];
        let mut s3 = vec![0f32; NTX * ll];
        let mut n = 0;
        for s in 0..NSYM {
            if ISYNC.contains(&(s + 1)) {
                continue;
            }
            let a = d0 + (s * nsd) as i64;
            for (k, b) in buf.iter_mut().enumerate() {
                let idx = a + k as i64;
                *b = if idx >= 0 && (idx as usize) < y.len() { y[idx as usize] } else { Complex::new(0.0, 0.0) };
            }
            self.sym_fft.process(&mut buf);
            for (ii, o) in s3[n * ll..(n + 1) * ll].iter_mut().enumerate() {
                *o = buf[(ii as i64 - 64 + mode).rem_euclid(nsd as i64) as usize].norm_sqr();
            }
            n += 1;
        }
        equalize(&mut s3, ll);
        let base = pctile(&mut s3.clone(), 40.0).max(f32::MIN_POSITIVE);
        s3.iter_mut().for_each(|v| *v = (*v / base).min(S3LIM));
        bzap(&mut s3, ll);
        s3
    }

    /// Prova le metriche su un insieme di energie: prima Bessel (sui segnali senza allargamento Doppler rende circa 1 dB piu' della
    /// sola evanescenza che usa WSJT-X), poi, se c'e' un modello, l'evanescenza rapida per ogni larghezza B90 ammessa da `ok_ibw`.
    fn try_metrics(&self, s3: &[f32], ap: Option<&Ap>, ok_ibw: impl Fn(i32) -> bool) -> Option<Hit> {
        let setup = self.setup();
        let spb = 1usize << setup.submode;
        let kind = ap.map_or(0, |a| a.kind);
        if let Some(msg) = qra_decode(&bessel_intrinsics(&narrow(s3, spb)), ap) {
            return Some(Hit { msg, ap: kind, b90_hz: None });
        }
        let Some(model) = self.options.fading else { return None };
        let (ibwa, ibwb) = ibw_range(setup);
        for ibw in ibwa..=ibwb {
            let b90 = 1.72f32.powi(ibw);
            if !ok_ibw(ibw) || b90 > 345.0 {
                continue;
            }
            let pix = fading_intrinsics(s3, spb, b90 / setup.baud() as f32, model, 1.0);
            if let Some(msg) = qra_decode(&pix, ap) {
                return Some(Hit { msg, ap: kind, b90_hz: Some(b90) });
            }
        }
        None
    }

    /// `q65_dec_q012` sulle energie prese dagli spettri del sincronismo (`q65_s1_to_s3`), al bin `i` del tono 0 e alla riga `jrow`
    /// del primo simbolo: e' il primo tentativo sulla frequenza di ricezione. `sp` sono gli spettri del periodo o la loro media
    /// (`s1=s1a(:,:,iseq)` di `q65_dec0` con `iavg=2`).
    fn from_sync_spectra(&self, sp: &SyncSpectra, i: usize, jrow: i64) -> Option<Hit> {
        let s3 = self.s3_from_sync(sp, i, jrow)?;
        std::iter::once(None).chain(self.aps.iter().map(Some)).find_map(|ap| self.try_metrics(&s3, ap, |_| true))
    }

    /// `q65_s1_to_s3`: le energie dei 63 simboli di dati prese dagli spettri del sincronismo, senza le portanti.
    fn s3_from_sync(&self, sp: &SyncSpectra, i: usize, jrow: i64) -> Option<Vec<f32>> {
        let mode = 1usize << self.setup().submode;
        let ll = M * (2 + mode);
        let i1 = (i + mode).checked_sub(64).filter(|&i1| i1 >= 1 && i1 + ll - 1 < sp.w)?;
        let mut s3 = vec![0f32; NTX * ll];
        let mut n = 0;
        for s in 0..NSYM {
            if ISYNC.contains(&(s + 1)) {
                continue;
            }
            let j = jrow + (NSTEP * s) as i64;
            for (ii, o) in s3[n * ll..(n + 1) * ll].iter_mut().enumerate() {
                *o = sp.at(i1 + ii, j);
            }
            n += 1;
        }
        bzap(&mut s3, ll);
        Some(s3)
    }

    /// `q65_loops`: attorno al candidato (tono 0 a `f0`, primo simbolo al campione `start`) prova mezzi bin di frequenza, sedicesimi
    /// di simbolo di tempo e larghezze B90 entro la distanza `MAXDIST`, prima senza AP e poi con ogni ipotesi.
    /// `drift_hz` e' la deriva di `q65_ccf_22` (`drift=df*idrift_best`, `q65.f90:894`), tolta in ogni tentativo come fa `twkfreq` con
    /// `a(2)=-0.5*drift` (`q65_loops.f90:53-57`): alla frequenza si aggiunge `a(2)·x`, con `x` da -1 a +1 lungo tutto il periodo
    /// (`npts` campioni di `c00`, non i soli 85 simboli) e zero a meta'.
    fn loops(&self, f0: f64, start: i64, drift_hz: f64) -> Option<(Hit, f64, i64)> {
        let setup = self.setup();
        let nsps = setup.nsps() as f64;
        let baud = setup.baud();
        let (z, fbase) = self.band(f0);
        let fsd = self.nsd as f64 * baud;
        // fase della rampa: 2π·∫a(2)·x dt = -π·drift·(t² - T·t)/T, con t in secondi dall'inizio del periodo
        let period = f64::from(setup.period_s);
        let k2 = -std::f64::consts::PI * drift_hz / period;
        let (ibwa, ibwb) = ibw_range(setup);
        let ibw0 = (ibwa + ibwb) / 2;
        let mut shifted: Vec<Option<Vec<Complex<f32>>>> = vec![None; IDFMAX as usize];
        let mut cache: Vec<Option<Vec<f32>>> = vec![None; (IDFMAX * IDTMAX) as usize];
        for ap in std::iter::once(None).chain(self.aps.iter().map(Some)) {
            for idf in 1..=IDFMAX {
                let ndf = zigzag(idf);
                let f1 = f0 + 0.5 * baud * f64::from(ndf);
                for idt in 1..=IDTMAX {
                    let ndt = zigzag(idt);
                    let base = ndf * ndf + ndt * ndt;
                    if base > MAXDIST {
                        continue;
                    }
                    let off = nsps * f64::from(ndt) / 16.0;
                    let slot = ((idf - 1) * IDTMAX + idt - 1) as usize;
                    if cache[slot].is_none() {
                        let y = shifted[(idf - 1) as usize].get_or_insert_with(|| {
                            let w = -2.0 * std::f64::consts::PI * (f1 - fbase) / fsd;
                            z.iter()
                                .enumerate()
                                .map(|(n, v)| {
                                    let t = n as f64 / fsd;
                                    let p = w * n as f64 + k2 * (t * t - period * t);
                                    v * Complex::new(p.cos() as f32, p.sin() as f32)
                                })
                                .collect()
                        });
                        let d0 = ((start as f64 + off) * self.nsd as f64 / nsps).round() as i64;
                        cache[slot] = Some(self.energies(y, d0));
                    }
                    let Some(s3) = cache[slot].as_deref() else { continue };
                    let ok = |ibw: i32| base + (ibw - ibw0) * (ibw - ibw0) <= MAXDIST;
                    if let Some(hit) = self.try_metrics(s3, ap, ok) {
                        return Some((hit, f1, start + off.round() as i64));
                    }
                }
            }
        }
        None
    }

    /// `q65_ccf_85` + `q65_dec_q3`: sincronismo con tutti gli 85 simboli di ogni messaggio della lista entro `rx_tol_hz` dalla
    /// frequenza di ricezione (bin `i0`); se il migliore stacca gli altri del 10% (o il sottomodo e' D o E), decodifica da lista: vince
    /// la parola di codice piu' verosimile sopra `PLOG_MIN`. Ritorna il messaggio, il bin del tono 0 e la riga del primo simbolo.
    /// `sp` sono gli spettri del sincronismo o quelli raddrizzati della deriva (`s1w`): con questi ultimi (`drift`) vale solo il
    /// distacco del 10%, senza l'eccezione dei sottomodi D ed E.
    fn list_decode(&self, sp: &SyncSpectra, list: &[ListWord], i0: usize, drift: bool) -> Option<(Hit, usize, i64)> {
        let setup = self.setup();
        let df = setup.baud();
        let mode = 1usize << setup.submode;
        let iz = sp.w as i64 - 1;
        let ia = (f64::from(self.options.rx_tol_hz) / df) as i64;
        let ia2 = ia.max(10 * mode as i64).max((100.0 / df).round() as i64);
        let iia = (200.0 / df) as i64;
        let nlag = (sp.lag2 - sp.lag1 + 1) as usize;
        let width = (2 * ia2 + 1) as usize;
        let mut ccf = vec![0f32; width * nlag];
        let mut best_of = vec![0f32; list.len()];
        let mut best: Option<(f32, i64, i64, usize)> = None;
        for (m, w) in list.iter().enumerate() {
            let mut tones = [0usize; NSYM];
            let mut k = 0;
            for (s, t) in tones.iter_mut().enumerate() {
                if !ISYNC.contains(&(s + 1)) {
                    *t = w.cw[k] as usize + 1;
                    k += 1;
                }
            }
            ccf.fill(0.0);
            for (l, lag) in (sp.lag1..=sp.lag2).enumerate() {
                let row = &mut ccf[l * width..(l + 1) * width];
                for (s, &t) in tones.iter().enumerate() {
                    let j = sp.j0 + (NSTEP * s) as i64 + lag;
                    if j < 0 || j as usize >= sp.jz {
                        continue;
                    }
                    let line = &sp.s1[j as usize * sp.w..(j as usize + 1) * sp.w];
                    let first = i0 as i64 + (mode * t) as i64 - ia2;
                    for (x, o) in row.iter_mut().enumerate() {
                        let ii = first + x as i64;
                        if ii >= iia && ii <= iz {
                            *o += line[ii as usize];
                        }
                    }
                }
            }
            let mut top = (0f32, 0i64, 0i64);
            for (l, lag) in (sp.lag1..=sp.lag2).enumerate() {
                for i in -ia..=ia {
                    let v = ccf[l * width + (i + ia2) as usize];
                    if v > top.0 {
                        top = (v, i, lag);
                    }
                }
            }
            best_of[m] = top.0;
            if top.0 > best.map_or(0.0, |b| b.0) {
                best = Some((top.0, top.1, top.2, m));
            }
        }
        let (ccf_best, ipk, jpk, m) = best?;
        best_of[m] = 0.0;
        let second = best_of.iter().copied().fold(0f32, f32::max);
        let better = if second > 0.0 { ccf_best / second } else { f32::MAX };
        if better < 1.10 && (mode < 8 || drift) {
            return None;
        }
        // `q65_s1_to_s3` + la verosimiglianza di ogni parola della lista per ogni larghezza B90
        let i = usize::try_from(i0 as i64 + ipk).ok()?;
        let jrow = sp.j0 + jpk;
        let s3 = self.s3_from_sync(sp, i, jrow)?;
        let spb = mode;
        let model = self.options.fading.unwrap_or(Fading::Lorentzian);
        let threshold = (LLH_THRESHOLD + (list.len() as f32 / 3.0).ln()).max(PLOG_MIN);
        let (ibwa, ibwb) = ibw_range(setup);
        for ibw in ibwa..=ibwb {
            let b90 = 1.72f32.powi(ibw);
            let pix = fading_intrinsics(&s3, spb, b90 / df as f32, model, 1.0);
            let mut win: Option<(f32, usize)> = None;
            for (k, w) in list.iter().enumerate() {
                let llh: f32 = w.cw.iter().enumerate().map(|(n, &y)| row(&pix, n)[y as usize].max(1e-36).ln()).sum();
                if llh > threshold && win.is_none_or(|(b, _)| llh > b) {
                    win = Some((llh, k));
                }
            }
            if let Some((_, k)) = win
                && list[k].msg.iter().sum::<i32>() > 0
            {
                return Some((Hit { msg: list[k].msg, ap: 3, b90_hz: Some(b90) }, i, jrow));
            }
        }
        None
    }

    /// `q65_snr`: S/N in 2500 Hz sommando gli spettri dei sincronismi allineati sugli 85 toni del messaggio decodificato. `sp` sono
    /// gli spettri del periodo o, per un messaggio dalla media, la media (in WSJT-X `s1` ne e' la copia).
    fn snr(&self, sp: &SyncSpectra, msg: &[i32; NK], f0: f64, start: i64) -> i32 {
        let setup = self.setup();
        let df = setup.baud();
        let mode = 1usize << setup.submode;
        let cw = encode_symbols(msg);
        let mut tones = [0usize; NSYM];
        let mut k = 0;
        for (s, t) in tones.iter_mut().enumerate() {
            if !ISYNC.contains(&(s + 1)) {
                *t = cw[k] as usize + 1;
                k += 1;
            }
        }
        let nominal = (setup.start_s() * FS).round() as i64;
        let jrow = sp.j0 + ((start - nominal) as f64 / sp.istep as f64).round() as i64;
        let iz = sp.w - 1;
        let nsum = (10 * mode).max((50.0 / df).round() as usize);
        let i0 = (f0 / df).round() as usize;
        let ia = i0.saturating_sub(2 * nsum).max(1);
        let ib = (i0 + 2 * nsum).min(iz);
        if ib < ia + 2 * nsum {
            return -35;
        }
        let spec: Vec<f32> = (ia..=ib)
            .map(|i| tones.iter().enumerate().map(|(s, &t)| sp.at(i + mode * t, jrow + (NSTEP * s) as i64)).sum())
            .collect();
        let avg = (spec[..nsum].iter().sum::<f32>() + spec[spec.len() - nsum..].iter().sum::<f32>()) / (2 * nsum) as f32;
        if avg <= 0.0 {
            return -35;
        }
        let area: f32 = spec[nsum..spec.len() - nsum].iter().map(|v| v / avg - 1.0).sum();
        (10.0 * f64::from(area.max(1.0)).log10() - 10.0 * (2500.0 / df).log10()).round() as i32
    }
}

impl Searcher<'_> {
    /// Campione dell'inizio del primo simbolo per il ritardo `lag` (in passi di `istep`) di `q65_ccf_22`.
    fn start_of(&self, lag: i64) -> i64 {
        (self.setup().start_s() * FS).round() as i64 + lag * self.sp.istep as i64
    }

    /// Campione dell'inizio del primo simbolo per la riga `jrow` degli spettri del sincronismo.
    fn start_of_row(&self, jrow: i64) -> i64 {
        self.start_of(jrow - self.sp.j0)
    }

    /// Il messaggio trovato con il suo S/N (`q65_snr` su `sp`).
    fn found(&self, sp: &SyncSpectra, hit: Hit, freq_hz: f64, start: i64, sync: f32, averaged: u32) -> Found {
        Found { snr_db: self.snr(sp, &hit.msg, freq_hz, start), hit, freq_hz, start, sync, averaged }
    }
}

impl SyncSpectra {
    /// Gli stessi spettri (righe, colonne, ritardi) con altri valori: la media della parita' (`s1=s1a(:,:,iseq)` di `q65_dec0`).
    fn with_s1(&self, s1: Vec<f32>) -> SyncSpectra {
        SyncSpectra { s1, ..*self }
    }
}

/// La correlazione migliore di `q65_ccf_22` entro `ntol` dalla frequenza di ricezione (`ibest`, `q65.f90:880-886`): servono valori
/// positivi (`ccfbest` parte da 0) e a parita' vince il bin piu' basso.
fn rx_peak(ccf2: &[f32], ia: usize, ib: usize, df: f64, options: &Options) -> Option<usize> {
    let rx = f64::from(options.rx_hz?);
    (ia..=ib)
        .filter(|&i| (i as f64 * df - rx).abs() <= f64::from(options.rx_tol_hz) && ccf2[i] > 0.0)
        .max_by(|&a, &b| ccf2[a].total_cmp(&ccf2[b]).then(b.cmp(&a)))
}

/// Un passaggio di `q65_ccf_22` su un insieme di spettri (del periodo o della media): correlazioni e ritardi, il picco sulla frequenza
/// di ricezione, la sua deriva in hertz e i candidati. In WSJT-X sono variabili del modulo (`drift`, `candidates`, `ncand`) che ogni
/// chiamata riscrive.
struct Sync22 {
    ccf2: Vec<f32>,
    lags: Vec<i64>,
    rx: Option<usize>,
    drift_hz: f64,
    candidates: Vec<(usize, i64, f32)>,
}

impl Sync22 {
    fn new(sp: &SyncSpectra, ia: usize, ib: usize, max_drift: i32, options: &Options) -> Sync22 {
        let setup = options.setup;
        let df = setup.baud();
        let (ccf2, lags, drifts) = sp.ccf22(ia, ib, max_drift);
        let rx = rx_peak(&ccf2, ia, ib, df, options);
        // `drift=df*idrift_best` (`q65.f90:894`): la deriva del picco migliore entro `ntol`, 0 se non ce n'e'
        let drift_hz = rx.map_or(0.0, |i| df * f64::from(drifts[i]));
        let candidates = sync_candidates(&ccf2, &lags, ia, ib, 1usize << setup.submode, options);
        Sync22 { ccf2, lags, rx, drift_hz, candidates }
    }
}

/// Un messaggio trovato va a `emit` (che dice se e' nuovo) e fra i trovati. AutoClrAvg (`q65_decode.f90:605-606,717-718`): se e'
/// nuovo, l'opzione e' accesa e cade entro `ntol` dalla frequenza di ricezione (`int(abs(f0dec-nfqso)).le.ntol`), la media della
/// parita' si svuota (`q65_clravg`), qualunque sia la stazione.
fn report(f: Found, found: &mut Vec<Found>, emit: &mut dyn FnMut(&Found) -> bool, avg: &mut Option<(&mut Averager, u64)>, options: &Options) {
    if emit(&f)
        && options.auto_clear_avg
        && let Some(rx) = options.rx_hz
        && (f.freq_hz - f64::from(rx)).abs().trunc() <= f64::from(options.rx_tol_hz)
        && let Some((averager, index)) = avg.as_mut()
    {
        averager.clear_parity(*index);
    }
    found.push(f);
}

/// Il giro dei candidati di `q65_decode.f90:630-741`: `q65_loops` (con la deriva `drift_hz`) su ognuno in ordine di frequenza,
/// saltando quelli che cadono su un segnale gia' trovato. Un tentativo identico gia' fatto (stesso bin, inizio e deriva) non si
/// ripete: darebbe lo stesso esito.
fn run_candidates(
    searcher: &Searcher,
    candidates: &[(usize, i64, f32)],
    drift_hz: f64,
    tried: &mut Vec<(usize, i64, u64)>,
    found: &mut Vec<Found>,
    report: &mut dyn FnMut(Found, &mut Vec<Found>),
) {
    let setup = searcher.setup();
    let df = setup.baud();
    let width = df * (1usize << setup.submode) as f64;
    let overlaps = |f0: f64, found: &[Found]| found.iter().any(|d| f0 - d.freq_hz > -width && f0 - d.freq_hz < 65.0 * width);
    let mut todo: Vec<(usize, i64, f32)> = Vec::new();
    for &(i, lag, snr) in candidates {
        let key = (i, searcher.start_of(lag), drift_hz.to_bits());
        if !tried.contains(&key) {
            tried.push(key);
            todo.push((i, key.1, snr));
        }
    }
    let threads = searcher.options.threads.max(1).min(todo.len().max(1));
    if threads <= 1 {
        for (i, start, snr) in todo {
            let f0 = i as f64 * df;
            if overlaps(f0, found) {
                continue;
            }
            if let Some((hit, f, start)) = searcher.loops(f0, start, drift_hz) {
                report(searcher.found(searcher.sp, hit, f, start, snr, 1), found);
            }
        }
        return;
    }
    // in parallelo si provano tutti; poi, in ordine di frequenza, si scartano quelli caduti su un segnale gia' preso
    let next = std::sync::atomic::AtomicUsize::new(0);
    let mut results: Vec<Option<(Hit, f64, i64)>> = Vec::new();
    results.resize_with(todo.len(), || None);
    let slots = std::sync::Mutex::new(results);
    let before: &[Found] = found;
    std::thread::scope(|scope| {
        for _ in 0..threads {
            scope.spawn(|| {
                loop {
                    let k = next.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                    let Some(&(i, start, _)) = todo.get(k) else { break };
                    if overlaps(i as f64 * df, before) {
                        continue;
                    }
                    let r = searcher.loops(i as f64 * df, start, drift_hz);
                    if let Ok(mut s) = slots.lock() {
                        s[k] = r;
                    }
                }
            });
        }
    });
    let results = slots.into_inner().unwrap_or_default();
    for ((i, _, snr), r) in todo.iter().zip(results) {
        if overlaps(*i as f64 * df, found) {
            continue;
        }
        if let Some((hit, f, start)) = r {
            report(searcher.found(searcher.sp, hit, f, start, *snr, 1), found);
        }
    }
}

/// La media della parita' sulla frequenza di ricezione (`iavg=1` e `iavg=2` di `q65_decode.f90:527-556`): prima la lista dei
/// messaggi del QSO (`q65_ccf_85` + `q65_dec_q3` su `s1a`), poi `q65_ccf_22` sulla media e `q65_dec_q012` sul suo picco migliore
/// entro `ntol`. Il messaggio va a `report`; ritorna il `q65_ccf_22` della media, i cui candidati e la cui deriva servono al giro dei
/// candidati che segue. `None` se la media della parita' e' vuota.
fn try_average(
    searcher: &Searcher,
    list: &[ListWord],
    (ia, ib, max_drift): (usize, usize, i32),
    avg: &mut Option<(&mut Averager, u64)>,
    found: &mut Vec<Found>,
    emit: &mut dyn FnMut(&Found) -> bool,
) -> Option<Sync22> {
    let options = searcher.options;
    let df = options.setup.baud();
    let (spa, navg) = {
        let (averager, index) = avg.as_ref()?;
        let p = (*index % 2) as usize;
        if averager.navg[p] == 0 {
            return None;
        }
        (searcher.sp.with_s1(averager.s1a[p].clone()), averager.navg[p])
    };
    let listed = options.rx_hz.filter(|_| !list.is_empty()).and_then(|f| searcher.list_decode(&spa, list, (f64::from(f) / df).round() as usize, false));
    let sync = Sync22::new(&spa, ia, ib, max_drift, options);
    let hit = match listed {
        Some((hit, i, jrow)) => Some((hit, i, searcher.start_of_row(jrow))),
        None => sync.rx.and_then(|i| searcher.from_sync_spectra(&spa, i, spa.j0 + sync.lags[i]).map(|hit| (hit, i, searcher.start_of(sync.lags[i])))),
    };
    if let Some((hit, i, start)) = hit {
        let f = searcher.found(&spa, hit, i as f64 * df, start, sync.ccf2.get(i).copied().unwrap_or(0.0), navg);
        report(f, found, emit, avg, options);
    }
    Some(sync)
}

/// Decodifica di un periodo come `q65_decode.f90`, nello stesso ordine:
/// 1. sulla frequenza di ricezione la lista dei messaggi del QSO, `q65_dec_q012` sugli spettri del sincronismo e `q65_loops`;
/// 2. se li' non esce nulla e la media della parita' ha almeno due periodi, la media (`q65_decode.f90:525`);
/// 3. se nulla e la media non e' stata provata, la fase 5 di W3SZ con `max_drift = 50`;
/// 4. `q65_loops` sui candidati (quelli della media, se e' gia' stata provata: `q65_ccf_22` li ha riscritti);
/// 5. se la media non e' stata ancora provata e ha almeno due periodi, la media e poi i suoi candidati (`q65_decode.f90:742`).
///
/// `avg` e' la media con il numero del periodo (`None` = senza media, come «Include averaging» spento in WSJT-X); ogni messaggio va a
/// `emit`, che dice se e' nuovo.
fn decode_period(audio: &[f32], options: &Options, aps: &[Ap], list: &[ListWord], mut avg: Option<(&mut Averager, u64)>, emit: &mut dyn FnMut(&Found) -> bool) {
    let setup = options.setup;
    let nsps = setup.nsps();
    let mode = 1usize << setup.submode;
    let df = setup.baud();
    let mut ia = (f64::from(options.f_low_hz).max(100.0) / df) as usize;
    let mut ib = (f64::from(options.f_high_hz).min(4900.0) / df) as usize;
    let ib_band = ib;
    // con la deriva (`max_drift`, W3SZ) `q65_ccf_22` cerca solo entro `ntol` dalla frequenza di ricezione
    if let Some(rx) = options.rx_hz.filter(|_| options.max_drift != 0) {
        let (rx, tol) = (f64::from(rx), f64::from(options.rx_tol_hz));
        ia = ((100.0 / df).round() as i64).max(((rx - tol) / df).round() as i64).max(0) as usize;
        ib = ((4900.0 / df).round() as i64).min(((rx + tol) / df).round() as i64).max(0) as usize;
        if ia >= ib {
            ia = ib.saturating_sub((tol / df) as usize);
        }
        ia = ia.max(1);
    }
    let nsum = (10 * mode).max((50.0 / df).round() as usize);
    // gli spettri coprono sempre tutta la banda di ricerca, anche con la deriva: cosi' la media non cambia forma quando si sposta la
    // frequenza di ricezione (in WSJT-X arrivano sempre a 5000 Hz, `iz=5000.0/df`)
    let iz = (ib.max(ib_band) + 2 * nsum + 66 * mode + M + 2).min(nsps / 2 - 1);
    if ia < 1 || ia >= ib || ib >= iz {
        return;
    }
    let sp = SyncSpectra::new(audio, setup, iz, options.eme_delay);
    // `q65_symspec` con dati nuovi (`lnewdat`): il periodo entra nella media della sua parita' (`q65.f90:630-635`)
    if let Some((averager, index)) = avg.as_mut() {
        averager.add(setup, *index, &sp);
    }
    let max_drift = options.max_drift.clamp(0, 50);
    let single = Sync22::new(&sp, ia, ib, max_drift, options);
    let searcher = Searcher::new(audio, options, aps, &sp);
    let mut found: Vec<Found> = Vec::new();
    let mut tried: Vec<(usize, i64, u64)> = Vec::new();
    let navg = |avg: &Option<(&mut Averager, u64)>| avg.as_ref().map_or(0, |(a, index)| a.count(*index));

    // 1. con il DX noto, prima la decodifica da lista sulla frequenza di ricezione (`q65_dec0` con `ncw>0`), poi `q65_dec_q012` e
    // `q65_loops` sul picco migliore entro `ntol`
    let listed = options.rx_hz.filter(|_| !list.is_empty()).and_then(|f| searcher.list_decode(&sp, list, (f64::from(f) / df).round() as usize, false));
    let mut at_rx = false;
    if let Some((hit, i, jrow)) = listed {
        let f = searcher.found(&sp, hit, i as f64 * df, searcher.start_of_row(jrow), single.ccf2.get(i).copied().unwrap_or(0.0), 1);
        report(f, &mut found, emit, &mut avg, options);
        at_rx = true;
    } else if let Some(i) = single.rx {
        let (f0, start) = (i as f64 * df, searcher.start_of(single.lags[i]));
        let hit = match searcher.from_sync_spectra(&sp, i, sp.j0 + single.lags[i]) {
            Some(h) => Some((h, f0, start)),
            None => {
                tried.push((i, start, single.drift_hz.to_bits()));
                searcher.loops(f0, start, single.drift_hz)
            }
        };
        if let Some((hit, f, start)) = hit {
            report(searcher.found(&sp, hit, f, start, single.ccf2[i], 1), &mut found, emit, &mut avg, options);
            at_rx = true;
        }
    }

    // 2. nulla sulla frequenza di ricezione: subito la media, se ha almeno due periodi
    let band = (ia, ib, max_drift);
    let mut avg_sync = None;
    if !at_rx && options.average_decode && navg(&avg) >= 2 {
        avg_sync = try_average(&searcher, list, band, &mut avg, &mut found, emit);
    }

    // 3. fase 5 di W3SZ (`stageno = 5`, solo con `max_drift = 50`): si riprova la lista sugli spettri raddrizzati con la deriva del picco
    // migliore entro `ntol`. Dopo la media `iavg` resta 2 e `q65_dec0` non riprova la lista (`ncw.gt.0 .and. iavg.le.1`,
    // `q65.f90:541`): in WSJT-X, provata la media, la fase 5 non da' nulla
    if !at_rx
        && avg_sync.is_none()
        && max_drift == 50
        && !list.is_empty()
        && let (Some(f), Some(_)) = (options.rx_hz, single.rx)
    {
        let s1w = sp.drifted(single.drift_hz, df);
        if let Some((hit, i, jrow)) = searcher.list_decode(&s1w, list, (f64::from(f) / df).round() as usize, true) {
            let f = searcher.found(&sp, hit, i as f64 * df, searcher.start_of_row(jrow), single.ccf2.get(i).copied().unwrap_or(0.0), 1);
            report(f, &mut found, emit, &mut avg, options);
        }
    }

    // «Single decode»: solo la frequenza di ricezione (`q65_decode.f90:628`)
    if options.single_decode {
        return;
    }
    // 4. e 5. il giro dei candidati, con la deriva del `q65_ccf_22` che li ha dati
    match avg_sync {
        Some(s) => run_candidates(&searcher, &s.candidates, s.drift_hz, &mut tried, &mut found, &mut |f, found| report(f, found, &mut *emit, &mut avg, options)),
        None => {
            run_candidates(&searcher, &single.candidates, single.drift_hz, &mut tried, &mut found, &mut |f, found| report(f, found, &mut *emit, &mut avg, options));
            if options.average_decode
                && navg(&avg) >= 2
                && let Some(s) = try_average(&searcher, list, band, &mut avg, &mut found, emit)
            {
                run_candidates(&searcher, &s.candidates, s.drift_hz, &mut tried, &mut found, &mut |f, found| report(f, found, &mut *emit, &mut avg, options));
            }
        }
    }
}

/// Media dei periodi (`s1a` e `navg` di `q65.f90`): per ogni parita' la media corrente degli spettri del sincronismo (`q65_symspec`,
/// `q65.f90:630-635`), con costante di tempo di al piu' 4 periodi (`ntc=min(navg,4)`, `s1a=s1/ntc+(1-1/ntc)·s1a`).
#[derive(Debug, Default, Clone)]
pub struct Averager {
    setup: Option<Setup>,
    /// Righe e colonne degli spettri: se cambiano la media riparte, come la nuova allocazione di `q65_dec0` (`q65.f90:396-419`).
    shape: (usize, usize),
    /// Per parita' (periodo pari, dispari): la media e i periodi entrati.
    s1a: [Vec<f32>; 2],
    navg: [u32; 2],
}

impl Averager {
    pub fn new() -> Averager {
        Averager::default()
    }

    /// Svuota la media delle due parita' («Clear Avg» di WSJT-X).
    pub fn clear(&mut self) {
        self.s1a = Default::default();
        self.navg = [0; 2];
    }

    /// `q65_clravg`: svuota la media della parita' di `period_index`.
    fn clear_parity(&mut self, period_index: u64) {
        let p = (period_index % 2) as usize;
        self.s1a[p].clear();
        self.navg[p] = 0;
    }

    /// Periodi nella media della parita' di `period_index` (`navg(iseq)`).
    pub fn count(&self, period_index: u64) -> u32 {
        self.navg[(period_index % 2) as usize]
    }

    fn add(&mut self, setup: Setup, period_index: u64, sp: &SyncSpectra) -> u32 {
        let shape = (sp.jz, sp.w);
        if self.setup != Some(setup) || self.shape != shape {
            self.clear();
            self.setup = Some(setup);
            self.shape = shape;
        }
        let p = (period_index % 2) as usize;
        let s1a = &mut self.s1a[p];
        if s1a.len() != sp.s1.len() {
            *s1a = vec![0.0; sp.s1.len()];
        }
        self.navg[p] += 1;
        let u = 1.0 / self.navg[p].min(4) as f32;
        s1a.iter_mut().zip(&sp.s1).for_each(|(a, &s)| *a = u * s + (1.0 - u) * *a);
        self.navg[p]
    }
}

/// Decodifica un periodo (audio a 12 kHz dall'inizio del periodo).
pub fn decode(audio: &[f32], options: &Options, hashes: &mut CallHashes) -> Vec<Decoded> {
    decode_inner(audio, options, hashes, None)
}

/// Come [`decode`], e in piu' mette il periodo nella media della sua parita' (`period_index` = numero d'ordine del periodo) e la
/// prova sulla frequenza di ricezione come WSJT-X con «Include averaging» acceso. La media si svuota da sola solo con
/// [`Options::auto_clear_avg`].
pub fn decode_averaged(audio: &[f32], options: &Options, hashes: &mut CallHashes, averager: &mut Averager, period_index: u64) -> Vec<Decoded> {
    decode_inner(audio, options, hashes, Some((averager, period_index)))
}

/// Un messaggio trovato, prima di spacchettarlo.
struct Found {
    hit: Hit,
    freq_hz: f64,
    /// Inizio del primo simbolo, in campioni a 12 kHz dall'inizio del periodo.
    start: i64,
    snr_db: i32,
    sync: f32,
    averaged: u32,
}

fn push_found(out: &mut Vec<Decoded>, f: &Found, setup: Setup, hashes: &mut CallHashes) -> bool {
    let Ok(text) = unpack(&symbols_to_bits(&f.hit.msg), hashes) else { return false };
    if out.iter().any(|d| d.text == text) {
        return false;
    }
    out.push(Decoded {
        text,
        freq_hz: f.freq_hz as f32,
        dt_s: (f.start as f64 / FS - setup.start_s()) as f32,
        snr_db: f.snr_db.clamp(-35, 25),
        sync: f.sync,
        ap: f.hit.ap,
        averaged: f.averaged,
        b90_hz: f.hit.b90_hz,
    });
    true
}

fn decode_inner(audio: &[f32], options: &Options, hashes: &mut CallHashes, averager: Option<(&mut Averager, u64)>) -> Vec<Decoded> {
    let setup = options.setup;
    let aps = if options.ap { ap_list(&options.my_call, &options.dx_call, hashes) } else { Vec::new() };
    // in contest la lista dei chiamanti (`q65_set_list2`), altrimenti quella del QSO con il DX (`q65_set_list`)
    let list = match (options.ap, options.contest) {
        (false, _) => Vec::new(),
        (true, true) => contest_list(&options.my_call, &options.dx_call, &options.dx_grid, &options.callers, hashes),
        (true, false) => qso_list(&options.my_call, &options.dx_call, &options.dx_grid, hashes),
    };
    let mut out: Vec<Decoded> = Vec::new();
    decode_period(audio, options, &aps, &list, averager, &mut |f| push_found(&mut out, f, setup, hashes));
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
        assert!(t.iter().filter(|&&v| v == 0).count() >= 22);
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
                // in una finestra cosi' stretta la soglia statistica dei candidati non ha senso: si prova la frequenza di ricezione
                let o = Options { setup, f_low_hz: f0 as f32 - 50.0, f_high_hz: f0 as f32 + 50.0, rx_hz: Some(f0 as f32), ap: false, ..Options::default() };
                let d = decode(&audio, &o, &mut CallHashes::default());
                let hit = d.iter().find(|d| d.text == "IU3ABC K1ABC FN42").unwrap_or_else(|| panic!("Q65-{p}{}: {d:?}", (b'A' + s) as char));
                assert!((hit.freq_hz - f0 as f32).abs() <= setup.baud() as f32 * 1.5, "{p} {s}: {hit:?}");
                assert_eq!((hit.ap, hit.averaged), (0, 1));
            }
        }
    }

    /// Un segnale troppo debole per un periodo solo si decodifica con la media dei periodi della stessa parita', sulla frequenza di
    /// ricezione come in WSJT-X; senza AutoClrAvg la media resta.
    #[test]
    fn averaging_decodes_a_signal_too_weak_for_one_period() {
        let setup = Setup::new(15, 0).expect("modo");
        let o = Options { setup, f_low_hz: 950.0, f_high_hz: 1050.0, rx_hz: Some(1000.0), ap: false, ..Options::default() };
        let msg = "IU3ABC K1ABC FN42";
        // la media corrente di WSJT-X (al piu' 4 periodi) guadagna meno di una somma senza limite
        let wave = modulate(&tones(msg, &mut CallHashes::default()).expect("messaggio"), 1000.0, setup, 0.07);
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
        let k = decoded_at.expect("la media non ha decodificato");
        assert_eq!(avg.count(0), k as u32 + 1, "senza AutoClrAvg la media resta");
    }

    /// «Enable averaging» spento (WSJT-X di fabbrica): lo stesso segnale debole non esce mai, ma la media si accumula lo stesso
    /// (`q65_symspec` la aggiorna sempre) e riaccendendo l'opzione decodifica subito.
    #[test]
    fn without_include_averaging_the_average_grows_but_is_not_decoded() {
        let setup = Setup::new(15, 0).expect("modo");
        let on = Options { setup, f_low_hz: 950.0, f_high_hz: 1050.0, rx_hz: Some(1000.0), ap: false, ..Options::default() };
        let off = Options { average_decode: false, ..on.clone() };
        let msg = "IU3ABC K1ABC FN42";
        let wave = modulate(&tones(msg, &mut CallHashes::default()).expect("messaggio"), 1000.0, setup, 0.07);
        let make = |seed: u64| {
            let mut audio = noise(15 * 12_000, 1.0, seed);
            let at = (setup.start_s() * FS) as usize;
            audio[at..at + wave.len()].iter_mut().zip(&wave).for_each(|(a, w)| *a += w);
            audio
        };
        let mut avg = Averager::new();
        for k in 0..8u64 {
            let d = decode_averaged(&make(100 + k), &off, &mut CallHashes::default(), &mut avg, 2 * k);
            assert!(!d.iter().any(|d| d.averaged > 1), "media provata con l'opzione spenta: {d:?}");
        }
        assert_eq!(avg.count(0), 8, "la media si accumula anche spenta");
        let d = decode_averaged(&make(200), &on, &mut CallHashes::default(), &mut avg, 16);
        assert!(d.iter().any(|d| d.text == msg && d.averaged >= 2), "riaccesa, la media accumulata decodifica: {d:?}");
    }

    /// «Single decode»: un segnale forte lontano dalla frequenza di ricezione non si cerca (niente giro dei candidati).
    #[test]
    fn single_decode_tries_only_the_rx_frequency() {
        let setup = Setup::new(15, 0).expect("modo");
        let strong = "CQ K9XYZ EN50";
        let audio = period(strong, 1600.0, 0.0, setup, 0.3, 1.0);
        let o = Options { setup, f_low_hz: 900.0, f_high_hz: 1700.0, rx_hz: Some(1000.0), ap: false, ..Options::default() };
        assert!(decode(&audio, &o, &mut CallHashes::default()).iter().any(|d| d.text == strong), "senza Single decode si trova");
        let single = Options { single_decode: true, ..o };
        assert!(decode(&audio, &single, &mut CallHashes::default()).is_empty());
    }

    /// Il segnale debole di `averaging_decodes_a_signal_too_weak_for_one_period` a 1000 Hz (frequenza di ricezione) e un'altra stazione
    /// a 1600 Hz (fuori dai 433 Hz del primo), forte, che si decodifica in ogni periodo: la media e i suoi periodi restano, anche con AutoClrAvg (l'altra stazione
    /// e' fuori da `ntol`).
    #[test]
    fn a_decode_of_another_station_keeps_the_average() {
        let setup = Setup::new(15, 0).expect("modo");
        let o = Options { setup, f_low_hz: 900.0, f_high_hz: 1700.0, rx_hz: Some(1000.0), ap: false, auto_clear_avg: true, ..Options::default() };
        let (weak, strong) = ("IU3ABC K1ABC FN42", "CQ K9XYZ EN50");
        let w1 = modulate(&tones(weak, &mut CallHashes::default()).expect("messaggio"), 1000.0, setup, 0.07);
        let w2 = modulate(&tones(strong, &mut CallHashes::default()).expect("messaggio"), 1600.0, setup, 0.3);
        let mut avg = Averager::new();
        let mut weak_at = None;
        for k in 0..8u64 {
            let mut audio = noise(15 * 12_000, 1.0, 100 + k);
            let at = (setup.start_s() * FS) as usize;
            audio[at..at + w1.len()].iter_mut().zip(&w1).for_each(|(a, w)| *a += w);
            audio[at..at + w2.len()].iter_mut().zip(&w2).for_each(|(a, w)| *a += w);
            let d = decode_averaged(&audio, &o, &mut CallHashes::default(), &mut avg, 2 * k);
            assert!(d.iter().any(|d| d.text == strong && d.averaged == 1), "periodo {k}: {d:?}");
            if let Some(hit) = d.iter().find(|d| d.text == weak) {
                assert!(hit.averaged >= 2, "{hit:?}");
                weak_at = Some(k);
                break;
            }
            assert_eq!(avg.count(0), k as u32 + 1, "la decodifica dell'altra stazione non svuota la media");
        }
        assert!(weak_at.is_some(), "la media non ha decodificato");
        // il segnale debole cade sulla frequenza di ricezione: con AutoClrAvg la media della parita' si svuota
        assert_eq!(avg.count(0), 0);
    }

    /// La media corrente di `q65_symspec`: con `ntc=min(navg,4)` il quinto periodo pesa 1/4.
    #[test]
    fn the_average_is_a_running_mean_with_four_periods_at_most() {
        let setup = Setup::new(15, 0).expect("modo");
        let mut avg = Averager::new();
        let base = SyncSpectra { s1: vec![0.0; 6], w: 3, jz: 2, istep: 1, j0: 0, lag1: 0, lag2: 0 };
        for (k, v) in [4.0f32, 8.0, 0.0, 4.0, 8.0].into_iter().enumerate() {
            avg.add(setup, 2 * k as u64, &base.with_s1(vec![v; 6]));
        }
        // 4, (4+8)/2=6, 6·2/3=4, 4·3/4+1=4, 4·3/4+2=5
        assert_eq!(avg.count(0), 5);
        assert!((avg.s1a[0][0] - 5.0).abs() < 1e-5, "{}", avg.s1a[0][0]);
        assert_eq!(avg.count(1), 0);
        avg.clear_parity(0);
        assert_eq!(avg.count(0), 0);
    }

    fn list_texts(my: &str, dx: &str, grid: &str) -> Vec<String> {
        let mut h = CallHashes::default();
        qso_list(my, dx, grid, &h.clone()).iter().map(|w| unpack(&symbols_to_bits(&w.msg), &mut h).expect("spacchettato")).collect()
    }

    /// `q65_set_list`: 206 messaggi; con il DX standard e «/P» o «/R» 411 (207 se il mio nominativo non e' standard).
    #[test]
    fn the_qso_list_grows_for_r_and_p_suffixes() {
        assert!(q65_std_call("K1ABC/P") && q65_std_call("K1ABC/R") && !std_call("K1ABC/P") && !q65_std_call("PJ4/K1ABC"));
        let plain = list_texts("IU3ABC", "K1ABC", "FN42");
        assert_eq!(plain.len(), 206);
        assert_eq!((plain[0].as_str(), plain[4].as_str(), plain[6].as_str()), ("IU3ABC K1ABC", "CQ K1ABC FN42", "IU3ABC K1ABC -50"));
        // 411 righe; «IU3ABC K1ABC/P» da solo (riga 1) non si impacchetta con `pack` di Aurora e si salta, come ogni riga che non
        // si impacchetta
        let p = list_texts("IU3ABC", "K1ABC/P", "FN42");
        assert_eq!(p.len(), 410, "{:?}", &p[..8]);
        assert!(!p.iter().any(|t| t == "IU3ABC K1ABC/P"));
        assert_eq!(p[0], "IU3ABC K1ABC/P RRR");
        assert_eq!(p[3], "CQ K1ABC/P FN42");
        assert_eq!(p[4], "IU3ABC K1ABC/P FN42");
        assert_eq!(p[5], "IU3ABC K1ABC/P -50");
        assert_eq!(p[204], "IU3ABC K1ABC/P R+49");
        assert_eq!(p[205], "CQ K1ABC/P");
        assert_eq!(p[206], "<IU3ABC> K1ABC/P");
        assert_eq!(p[207], "<IU3ABC> K1ABC/P RRR");
        assert_eq!(p[208], "<IU3ABC> K1ABC/P RR73");
        assert_eq!(p[209], "<IU3ABC> K1ABC/P 73");
        assert_eq!(p[210], "IU3ABC <K1ABC/P> -50");
        assert_eq!(p[211], "IU3ABC <K1ABC/P> R-50");
        assert_eq!(p[409], "IU3ABC <K1ABC/P> R+49");
        // senza locatore anche la riga 6 e' «IU3ABC K1ABC/R» da sola: due righe saltate
        let r = list_texts("IU3ABC", "K1ABC/R", "");
        assert_eq!(r.len(), 409);
        assert!(r.contains(&"CQ K1ABC/R".to_string()));
        assert_eq!(r.last().map(String::as_str), Some("IU3ABC <K1ABC/R> R+49"));
        // il mio nominativo non standard: solo il CQ senza locatore in piu'
        let ns = list_texts("PJ4/IU3ABC", "K1ABC/P", "FN42");
        assert!(ns.len() >= 206 && ns.len() <= 207, "{}", ns.len());
        assert_eq!(ns.last().map(String::as_str), Some("CQ K1ABC/P"));
        assert!(!ns.iter().any(|t| t.contains("R+49") && t.starts_with("PJ4/IU3ABC <")), "niente righe 212-411");
    }

    /// La rampa di `twkfreq` in `q65_loops`: un segnale che sale di 8 bin lungo il periodo si decodifica togliendo la deriva, non
    /// senza.
    #[test]
    fn loops_remove_the_frequency_drift() {
        let setup = Setup::new(15, 0).expect("modo");
        let tn = tones("IU3ABC K1ABC FN42", &mut CallHashes::default()).expect("messaggio");
        let (f0, drift) = (1000.0f64, 8.0 * setup.baud());
        let period = f64::from(setup.period_s);
        let mut audio = noise(15 * 12_000, 1.0, 11);
        let at = (setup.start_s() * FS) as usize;
        let mut phase = 0.0f64;
        for (s, &t) in tn.iter().enumerate() {
            for k in 0..setup.nsps() {
                let n = at + s * setup.nsps() + k;
                // la stessa deriva lineare che toglie `twkfreq`: zero a meta' periodo
                let f = f0 + f64::from(t) * setup.tone_spacing_hz() + drift * (n as f64 / FS / period - 0.5);
                phase += 2.0 * std::f64::consts::PI * f / FS;
                audio[n] += 0.3 * phase.sin() as f32;
            }
        }
        let o = Options { setup, ap: false, ..Options::default() };
        let iz = (setup.nsps() / 2 - 1).min((2000.0 / setup.baud()) as usize);
        let sp = SyncSpectra::new(&audio, setup, iz, false);
        let searcher = Searcher::new(&audio, &o, &[], &sp);
        let start = at as i64;
        assert!(searcher.loops(f0, start, 0.0).is_none(), "senza togliere la deriva non dovrebbe decodificare: aumentarla");
        let (hit, f, _) = searcher.loops(f0, start, drift).expect("decodificato togliendo la deriva");
        assert_eq!(unpack(&symbols_to_bits(&hit.msg), &mut CallHashes::default()).expect("spacchettato"), "IU3ABC K1ABC FN42");
        assert!((f - f0).abs() <= setup.baud(), "{f}");
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
            let base = Options { setup, f_low_hz: 950.0, f_high_hz: 1050.0, rx_hz: Some(1000.0), ..Options::default() };
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
        // il segnale e' la somma di molte copie con frequenza e fase casuali (allargamento di circa 60 Hz, due toni: con 30 Hz la
        // ricerca fine del sincronismo basta gia' a Bessel)
        const SPREAD: f64 = 60.0;
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
        let o = Options { setup, f_low_hz: 750.0, f_high_hz: 850.0, rx_hz: Some(800.0), ap: false, ..Options::default() };
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
