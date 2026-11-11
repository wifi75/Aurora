//! FST4 e FST4W: 4-GFSK (BT = 2) di 160 simboli (5 blocchi di sincronismo da 8 e 120 di dati), periodi da 15 s a 30 minuti.
//! FST4 porta un messaggio a 77 bit come FT8 (codice LDPC 240,101 con CRC24), FST4W uno di tipo WSPR a 50 bit (LDPC 240,74 con
//! CRC24). Porting di WSJT-X 3.3.0-beta6 (GPL 3): `lib/fst4/genfst4.f90`, `gen_fst4wave.f90` (con `waveform_kernels.f90`),
//! `get_fst4_bitmetrics.f90`, `fst4_baseline.f90`, `lib/fst4_decode.f90`, `lib/blanker.f90`, `lib/pctile.f90`, `lib/polyfit.f90`;
//! periodi e campioni per simbolo come `widgets/mainwindow.cpp:13487-13506` e `fst4_decode.f90:398-433`.
//!
//! La distanza fra i toni e' la velocita' di simbolo (`hmod` = 1: WSJT-X lo porta a 2 o 4 solo con le opzioni sperimentali
//! «x2/x4 Tone Spacing», `mainwindow.cpp:8090-8092`, e il ricevitore usa sempre 1, `fst4_decode.f90:313`).

// porting indice per indice del Fortran: i cicli sugli indici restano come nell'originale
#![allow(clippy::needless_range_loop)]

use ft8::message::{Bits77, CallHashes, message_type, pack, unpack};
use rustfft::{Fft, FftPlanner, num_complex::Complex};
use std::sync::Arc;

use crate::fst4ldpc::{Code, Ldpc, N as NCW, append_crc24};
use crate::fst4msg;

type C32 = Complex<f32>;

/// Simboli trasmessi.
pub const NN: usize = 160;
/// Simboli di dati.
const ND: usize = 120;
const FS: f32 = 12_000.0;
/// Candidati al massimo (`MAXCAND`, `fst4_decode.f90:17`).
pub const MAXCAND: usize = 200;

const ISYNC1: [u8; 8] = [0, 1, 3, 2, 1, 0, 2, 3];
const ISYNC2: [u8; 8] = [2, 3, 1, 0, 3, 2, 0, 1];
/// Bit con cui FST4 mescola i 77 bit prima del CRC (`rvec`, `genfst4.f90:37-39`).
const RVEC: [u8; 77] = [
    0, 1, 0, 0, 1, 0, 1, 0, 0, 1, 0, 1, 1, 1, 1, 0, 1, 0, 0, 0, 1, 0, 0, 1, 1, 0, 1, 1, 0, 1, 0, 0, 1, 0, 1, 1, 0, 0, 0, 0, 1, 0, 0, 0,
    1, 0, 1, 0, 0, 1, 1, 1, 1, 0, 0, 1, 0, 1, 0, 1, 0, 1, 0, 1, 1, 0, 1, 1, 1, 1, 1, 0, 0, 0, 1, 0, 1,
];

/// Periodi di FST4 in secondi (`mainwindow.cpp:11394`).
pub const FST4_PERIODS: [u32; 7] = [15, 30, 60, 120, 300, 900, 1800];
/// Periodi di FST4W (`mainwindow.cpp:787`).
pub const FST4W_PERIODS: [u32; 4] = [120, 300, 900, 1800];

/// Campioni per simbolo a 12 kHz, fattore di decimazione e lunghezza della FFT grande (`fst4_decode.f90:398-433`).
fn period_params(period_s: u32) -> Option<(usize, usize, usize)> {
    let (nsps, ndown, nfft1) = match period_s {
        15 => (720, 18, 180_000),
        30 => (1680, 42, 359_856),
        60 => (3888, 108, 720_000),
        120 => (8200, 205, 1_440_000),
        300 => (21_504, 512, 3_594_240),
        900 => (66_560, 1664, 10_782_720),
        1800 => (134_400, 3360, 21_591_360),
        _ => return None,
    };
    // `nfft2=nfft1/ndown; nfft1=nfft2*ndown` (righe 443-444)
    Some((nsps, ndown, nfft1 / ndown * ndown))
}

/// Campioni per simbolo a 12 kHz del periodo (`None` se il periodo non esiste).
pub fn nsps(period_s: u32) -> Option<usize> {
    period_params(period_s).map(|p| p.0)
}

/// Distanza fra i toni (= velocita' di simbolo) in hertz.
pub fn tone_spacing_hz(period_s: u32) -> f64 {
    nsps(period_s).map_or(0.0, |n| 12_000.0 / n as f64)
}

/// Inizio della trasmissione dall'inizio del periodo: 0,5 s in FST4-15, 1 s negli altri (`Modulator.cpp:163-164`).
pub fn start_s(period_s: u32) -> f64 {
    if period_s == 15 { 0.5 } else { 1.0 }
}

/// Durata del segnale in secondi.
pub fn duration_s(period_s: u32) -> f64 {
    nsps(period_s).map_or(0.0, |n| (NN * n) as f64 / 12_000.0)
}

/// `get_fst4_tones_from_bits`: dai bit d'informazione (101 o 74 con il CRC) ai 160 toni (codifica Gray 00 01 11 10).
pub fn tones_from_bits(ldpc: &Ldpc, msgbits: &[u8]) -> [u8; NN] {
    let cw = ldpc.encode(msgbits);
    let mut data = [0u8; ND];
    for (i, d) in data.iter_mut().enumerate() {
        let is = cw[2 * i + 1] + 2 * cw[2 * i];
        *d = match is {
            0 => 0,
            1 => 1,
            2 => 3,
            _ => 2,
        };
    }
    let mut t = [0u8; NN];
    t[0..8].copy_from_slice(&ISYNC1);
    t[8..38].copy_from_slice(&data[0..30]);
    t[38..46].copy_from_slice(&ISYNC2);
    t[46..76].copy_from_slice(&data[30..60]);
    t[76..84].copy_from_slice(&ISYNC1);
    t[84..114].copy_from_slice(&data[60..90]);
    t[114..122].copy_from_slice(&ISYNC2);
    t[122..152].copy_from_slice(&data[90..120]);
    t[152..160].copy_from_slice(&ISYNC1);
    t
}

/// I bit d'informazione di FST4: 77 bit mescolati con `rvec` e CRC24 (`genfst4.f90:91-95`).
fn bits101(c77: &Bits77) -> Vec<u8> {
    let mut m = vec![0u8; 101];
    for i in 0..77 {
        m[i] = (c77[i] + RVEC[i]) % 2;
    }
    append_crc24(&mut m);
    m
}

/// I bit d'informazione di FST4W: i primi 50 bit di un messaggio di tipo 0.6 e CRC24 (`genfst4.f90:85-89`).
fn bits74(c77: &Bits77) -> Vec<u8> {
    let mut m = vec![0u8; 74];
    m[..50].copy_from_slice(&c77[..50]);
    append_crc24(&mut m);
    m
}

/// Un messaggio pronto da trasmettere.
#[derive(Debug, Clone)]
pub struct Encoded {
    pub tones: [u8; NN],
    /// Il testo come lo leggera' chi riceve (`msgsent`).
    pub text: String,
}

/// `genfst4`: testo -> toni. In FST4W (`wspr`) il messaggio deve essere di tipo WSPR (`CALL LOC4 dBm`, `PFX/CALL dBm`,
/// `<CALL> LOC6`); in FST4 un testo di quella forma va nel tipo 0.6 come in WSJT-X (`pack77_06_candidate` e' provato prima del tipo 1,
/// `packjt77.f90:798`), gli altri nei tipi di FT8.
pub fn encode(text: &str, wspr: bool, hashes: &mut CallHashes) -> Result<Encoded, String> {
    let message: String = text.split('\0').next().unwrap_or("").trim().to_string();
    if wspr {
        let bits = fst4msg::pack(&message, hashes, true).ok_or_else(|| "messaggio FST4W non valido".to_string())?;
        let sent = fst4msg::unpack(&bits, &mut hashes.clone()).ok_or_else(|| "messaggio FST4W non valido".to_string())?;
        let ldpc = Ldpc::new(Code::C74);
        return Ok(Encoded { tones: tones_from_bits(&ldpc, &bits74(&bits)), text: sent });
    }
    let bits = match fst4msg::pack(&message, hashes, false) {
        Some(b) => b,
        None => pack(&message, hashes).map_err(|e| e.to_string())?,
    };
    let sent = unpack_any(&bits, &mut hashes.clone()).ok_or_else(|| "messaggio non valido".to_string())?;
    let ldpc = Ldpc::new(Code::C101);
    Ok(Encoded { tones: tones_from_bits(&ldpc, &bits101(&bits)), text: sent })
}

/// Spacchetta 77 bit di FST4: il tipo 0.6 qui, gli altri con FT8.
fn unpack_any(bits: &Bits77, hashes: &mut CallHashes) -> Option<String> {
    if message_type(bits) == (0, 6) {
        return fst4msg::unpack(bits, hashes);
    }
    unpack(bits, hashes).ok()
}

/// `gen_fst4wave` (`icmplx` = 0): onda GFSK a fase continua con BT = 2, `f0_hz` = frequenza del tono 0, `fs` = campioni al secondo
/// (multiplo di 12000 per avere un numero intero di campioni per simbolo, come i 48 kHz di WSJT-X). Rampa a coseno rialzato di un
/// quarto di simbolo all'inizio e alla fine. Restituisce 160 simboli di campioni.
pub fn modulate(tones: &[u8; NN], f0_hz: f64, period_s: u32, amplitude: f32, fs: f64) -> Vec<f32> {
    let Some(nsps12) = nsps(period_s) else { return Vec::new() };
    let nsps = (nsps12 as f64 * fs / 12_000.0).round() as usize;
    if nsps < 4 {
        return Vec::new();
    }
    let hmod = 1.0f64;
    let two_pi = 2.0 * std::f64::consts::PI;
    // impulso su tre simboli, `t=(i-1.5*nsps)/nsps` con i da 1 (`waveform_kernels.f90`, `prepare_pulse`)
    let pulse: Vec<f64> =
        (0..3 * nsps).map(|i| f64::from(ft8::modem::gfsk_pulse(2.0, (((i + 1) as f64 - 1.5 * nsps as f64) / nsps as f64) as f32))).collect();
    let peak = two_pi * hmod / nsps as f64;
    let carrier = two_pi * f0_hz / fs;
    let total = NN * nsps;
    let mut out = Vec::with_capacity(total);
    let mut phi = 0f64;
    for n in 0..total {
        // `source_offset = nsps`: il campione n vede l'impulso del simbolo j alla posizione n + nsps - j*nsps
        let raw = n + nsps;
        let q = raw / nsps;
        let mut inc = 0f64;
        for j in q.saturating_sub(2)..=q.min(NN - 1) {
            let p = raw - j * nsps;
            if p < 3 * nsps {
                inc += peak * pulse[p] * f64::from(tones[j]);
            }
        }
        out.push(amplitude * phi.sin() as f32);
        phi = (phi + inc + carrier) % two_pi;
    }
    // rampe (`gen_fst4wave.f90:38-39`)
    let quarter = nsps / 4;
    let den = (nsps / 2) as f64;
    for (i, v) in out.iter_mut().enumerate().take(quarter) {
        *v *= ((1.0 - (two_pi * i as f64 / den).cos()) / 2.0) as f32;
    }
    let tail = (NN - 1) * nsps + 3 * nsps / 4;
    for i in 0..=quarter {
        if let Some(v) = out.get_mut(tail + i) {
            *v *= ((1.0 + (two_pi * i as f64 / den).cos()) / 2.0) as f32;
        }
    }
    out
}

// ------------------------------------------------------------------------------------------------------------- ricezione

/// Un messaggio decodificato.
#[derive(Debug, Clone, PartialEq)]
pub struct Decoded {
    pub text: String,
    /// Frequenza del tono 0, in hertz (`fsig`).
    pub freq_hz: f32,
    /// Ritardo rispetto all'inizio nominale (0,5 s in FST4-15, 1 s negli altri), in secondi.
    pub dt_s: f32,
    /// Rapporto segnale/rumore in 2500 Hz.
    pub snr_db: i32,
    /// Altezza del picco fra i candidati (`sync`).
    pub sync: f32,
    /// Ipotesi a priori (0 = nessuna; 1 CQ, 2 mio nominativo, 3 con il DX, 4 RRR, 5 73, 6 RR73).
    pub ap: u8,
    /// Bit efficaci del decodificatore (91 in FST4, 66 o 50 in FST4W).
    pub keff: u8,
}

#[derive(Debug, Clone)]
pub struct Options {
    pub period_s: u32,
    /// FST4W.
    pub wspr: bool,
    /// Banda di ricerca di FST4 («F Low» e «F High», `nfa`/`nfb`).
    pub f_low_hz: i32,
    pub f_high_hz: i32,
    /// Frequenza di ricezione (`nfqso`).
    pub rx_hz: i32,
    /// Tolleranza (`ntol`): in FST4W la ricerca e' solo entro questa distanza dalla frequenza di ricezione.
    pub tol_hz: i32,
    /// Profondita' 1, 2 o 3 (`ndepth`): 1 = niente AP e niente spostamenti; 3 = in FST4W anche Keff 50.
    pub depth: u8,
    /// Passo del QSO (0..5, `nQSOProgress`) per le ipotesi AP.
    pub qso_progress: u8,
    /// «Single decode»: la banda di ricerca diventa frequenza di ricezione +- tolleranza (`mainwindow.cpp:5455-5457`).
    pub single_decode: bool,
    pub ap_cq_only: bool,
    pub my_call: String,
    pub dx_call: String,
    /// Percentuale di campioni da azzerare contro i disturbi impulsivi (`npct` del «NB», 0 = spento).
    pub blanker_pct: u32,
    /// Ritardo EME (`emedelay`, 0 = cerca da 0 a 2 s).
    pub eme_delay: f32,
    pub max_candidates: usize,
    pub threads: usize,
}

impl Default for Options {
    fn default() -> Self {
        Options {
            period_s: 60,
            wspr: false,
            f_low_hz: 1000,
            f_high_hz: 2000,
            rx_hz: 1500,
            tol_hz: 20,
            depth: 3,
            qso_progress: 0,
            single_decode: false,
            ap_cq_only: false,
            my_call: String::new(),
            dx_call: String::new(),
            blanker_pct: 0,
            eme_delay: 0.0,
            max_candidates: MAXCAND,
            threads: 1,
        }
    }
}

/// Stato fra un periodo e l'altro: i nominativi/locatori sentiti in FST4W per la decodifica con Keff 50 (`wcalls`, al massimo 100),
/// e le ipotesi AP gia' pronte.
#[derive(Debug, Clone, Default)]
pub struct State {
    pub known_calls: Vec<String>,
}

const MAXWCALLS: usize = 100;

/// Spettro reale della FFT grande (`fftwf_plan_dft_r2c_1d`), bin da 0 a nfft/2, calcolato con una FFT complessa lunga la meta'.
fn real_fft(x: &[f32], nfft: usize, planner: &mut FftPlanner<f32>) -> Vec<C32> {
    let h = nfft / 2;
    let mut z: Vec<C32> = (0..h).map(|j| C32::new(x.get(2 * j).copied().unwrap_or(0.0), x.get(2 * j + 1).copied().unwrap_or(0.0))).collect();
    planner.plan_fft_forward(h).process(&mut z);
    let mut out = vec![C32::new(0.0, 0.0); h + 1];
    let w = -2.0 * std::f64::consts::PI / nfft as f64;
    for (k, o) in out.iter_mut().enumerate() {
        let zk = z[k % h];
        let zc = z[(h - k % h) % h].conj();
        let e = (zk + zc) * 0.5;
        let od = (zk - zc) * C32::new(0.0, -0.5);
        let (s, c) = (w * k as f64).sin_cos();
        *o = e + C32::new(c as f32, s as f32) * od;
    }
    out
}

/// `blanker.f90`: azzera i campioni sopra la soglia che lascia fuori `npct` per cento (e il successivo), in unita' di campione a
/// 16 bit.
fn blanker(x: &mut [f32], npct: u32) {
    if npct == 0 {
        return;
    }
    let nz = x.len();
    let mut hist = vec![0usize; 32_769];
    for v in x.iter() {
        let n = (v.clamp(-32767.0, 32767.0) as i32).unsigned_abs() as usize;
        hist[n.min(32_768)] += 1;
    }
    let target = (nz as f32 * 0.01 * npct as f32).round() as usize;
    let mut n = 0;
    let mut nthresh = 0i32;
    for i in (0..=32_768).rev() {
        n += hist[i];
        nthresh = i as i32;
        if n >= target {
            break;
        }
    }
    let mut ndrop = 0;
    for v in x.iter_mut() {
        let mut i0 = v.clamp(-32767.0, 32767.0) as i32;
        if ndrop > 0 {
            i0 = 0;
            ndrop -= 1;
        }
        if i0.abs() > nthresh {
            i0 = 0;
            ndrop = 1;
        }
        *v = i0 as f32;
    }
}

/// `pctile.f90`: il valore al `npct` per cento.
fn pctile(x: &[f32], npct: usize) -> f32 {
    if x.is_empty() {
        return 1.0;
    }
    let mut t = x.to_vec();
    t.sort_by(f32::total_cmp);
    let n = t.len();
    let j = ((n as f32 * 0.01 * npct as f32).round() as usize).clamp(1, n);
    t[j - 1]
}

/// `polyfit.f90` (peso 1, tre termini): parabola dei minimi quadrati con la regola di Cramer.
fn polyfit3(x: &[f64], y: &[f64]) -> [f64; 3] {
    let mut sumx = [0f64; 5];
    let mut sumy = [0f64; 3];
    for (&xi, &yi) in x.iter().zip(y) {
        let mut t = 1.0;
        for s in sumx.iter_mut() {
            *s += t;
            t *= xi;
        }
        let mut t = yi;
        for s in sumy.iter_mut() {
            *s += t;
            t *= xi;
        }
    }
    let det = |a: &[[f64; 3]; 3]| {
        a[0][0] * (a[1][1] * a[2][2] - a[1][2] * a[2][1]) - a[0][1] * (a[1][0] * a[2][2] - a[1][2] * a[2][0])
            + a[0][2] * (a[1][0] * a[2][1] - a[1][1] * a[2][0])
    };
    let base: [[f64; 3]; 3] = std::array::from_fn(|j| std::array::from_fn(|k| sumx[j + k]));
    let delta = det(&base);
    if delta == 0.0 {
        return [0.0; 3];
    }
    std::array::from_fn(|l| {
        let mut a = base;
        for (j, row) in a.iter_mut().enumerate() {
            row[l] = sumy[j];
        }
        det(&a) / delta
    })
}

/// `fst4_baseline_owned`: linea di base del rumore (in potenza) fra `ia` e `ib` (indici di `s`), da una parabola sui punti piu'
/// bassi di 8 segmenti; fuori dall'intervallo vale 1.
fn baseline(s: &[f32], ia: usize, ib: usize, npct: usize) -> Vec<f32> {
    let mut sbase = vec![0f32; s.len()];
    let mut sw = vec![0f32; s.len()];
    for i in ia..=ib {
        sw[i] = (10.0 * f64::from(s[i]).log10()) as f32;
    }
    let nseg = 8;
    let nlen = (ib + 1 - ia) / nseg;
    let i0 = ((ib + 1 - ia) / 2) as f64;
    let mut x = Vec::with_capacity(1000);
    let mut y = Vec::with_capacity(1000);
    for n in 0..nseg {
        let ja = ia + n * nlen;
        let jb = ja + nlen;
        if nlen == 0 || jb > sw.len() {
            continue;
        }
        let base = pctile(&sw[ja..jb], npct);
        for (i, &v) in sw.iter().enumerate().take(jb).skip(ja) {
            if v <= base {
                // `if (k.lt.1000) k=k+1`: oltre i 1000 si riscrive l'ultimo
                if x.len() == 1000 {
                    x.pop();
                    y.pop();
                }
                x.push(i as f64 - i0);
                y.push(f64::from(v));
            }
        }
    }
    let a = polyfit3(&x, &y);
    for (i, b) in sbase.iter_mut().enumerate() {
        let v = if i >= ia && i <= ib {
            let t = i as f64 - i0;
            (a[0] + t * (a[1] + t * a[2]) + 0.2) as f32
        } else {
            0.0
        };
        *b = 10f32.powf(v / 10.0);
    }
    sbase
}

/// Un candidato (`candidates(:,1:5)`): frequenza di partenza, picco, frequenza e inizio dopo il sincronismo, rumore.
#[derive(Debug, Clone, Copy)]
struct Cand {
    f0: f32,
    sync: f32,
    fc: f32,
    is: i64,
    base: f32,
}

/// `get_candidates_fst4`: picchi della correlazione dello spettro a mezzo bin con i quattro toni, tolti uno alla volta (CLEAN).
#[allow(clippy::too_many_arguments)]
fn candidates(big: &[C32], nfft1: usize, nsps: usize, fa: f32, fb: f32, nfa: i32, nfb: i32, minsync: f32, maxcand: usize) -> Vec<Cand> {
    const XDB: [f32; 7] = [0.25, 0.50, 0.75, 1.0, 0.75, 0.50, 0.25];
    let hmod = 1i64;
    let df1 = FS / nfft1 as f32;
    let baud = FS / nsps as f32;
    let df2 = baud / 2.0;
    let nd = (df2 / df1) as i64;
    let ndh = nd / 2;
    let mut ia = (fa.max(100.0) / df2).round() as i64;
    let mut ib = (fb.min(4800.0) / df2).round() as i64;
    let mut ina = (100f32.max(nfa as f32) / df2).round() as i64;
    let mut inb = (4800f32.min(nfb as f32) / df2).round() as i64;
    ia = ia.max(ina);
    ib = ib.min(inb);
    if ib < ia || inb - ina < 8 {
        return Vec::new();
    }
    let nnw = (48000.0 * nsps as f32 * 2.0 / FS).round() as i64;
    // indici da 1 come in Fortran: posizione 0 inutilizzata
    let mut s = vec![0f32; nnw as usize + 1];
    for i in ina..=inb {
        if i < 1 || i > nnw {
            continue;
        }
        let j0 = (i as f32 * df2 / df1).round() as i64;
        for j in j0 - ndh..=j0 + ndh {
            if let Some(c) = usize::try_from(j).ok().and_then(|j| big.get(j)) {
                s[i as usize] += c.norm_sqr();
            }
        }
    }
    ina = ina.max(1 + 3 * hmod);
    inb = inb.min(nnw - 3 * hmod);
    if inb - ina < 8 {
        return Vec::new();
    }
    let mut s2 = vec![0f32; nnw as usize + 1];
    for i in ina..=inb {
        let u = i as usize;
        s2[u] = s[u - 3] + s[u - 1] + s[u + 1] + s[u + 3];
    }
    let sbase = baseline(&s2, (ina + 3 * hmod) as usize, (inb - 3 * hmod) as usize, 30);
    if (ina..=inb).any(|i| sbase[i as usize] <= 0.0) {
        return Vec::new();
    }
    for i in ina..=inb {
        s2[i as usize] /= sbase[i as usize];
    }
    ia = ia.max(3);
    ib = ib.min(nnw - 2);
    let mut out = Vec::new();
    while out.len() < maxcand {
        let mut iploc = ia;
        let mut pval = f32::MIN;
        for i in ia..=ib {
            if s2[i as usize] > pval {
                pval = s2[i as usize];
                iploc = i;
            }
        }
        if pval < minsync {
            break;
        }
        for (k, x) in XDB.iter().enumerate() {
            let kk = iploc + 2 * hmod * (k as i64 - 3);
            if kk >= ia && kk <= ib {
                let v = &mut s2[kk as usize];
                *v = (*v - 0.9 * pval * x).max(0.0);
            }
        }
        out.push(Cand { f0: df2 * iploc as f32, sync: pval, fc: 0.0, is: 0, base: sbase[iploc as usize] });
    }
    out
}

/// `fst4_downsample`: la fetta di spettro attorno a `f0` portata in banda base a 12000/ndown campioni al secondo.
fn downsample(big: &[C32], nfft1: usize, nfft2: usize, f0: f32, sigbw: f32, ifft: &Arc<dyn Fft<f32>>) -> Vec<C32> {
    let df = FS / nfft1 as f32;
    let i0 = (f0 / df).round() as i64;
    let ih = ((f0 + 1.3 * sigbw / 2.0) / df).round() as i64;
    let nbw = ih - i0 + 1;
    let mut c1 = vec![C32::new(0.0, 0.0); nfft2];
    let at = |i: i64| usize::try_from(i).ok().and_then(|i| big.get(i)).copied().unwrap_or_default();
    c1[0] = at(i0);
    let h = (nfft1 / 2) as i64;
    for i in 1..=nbw {
        if i0 + i <= h && (i as usize) < nfft2 {
            c1[i as usize] = at(i0 + i);
        }
        if i0 - i >= 0 && (i as usize) < nfft2 {
            c1[nfft2 - i as usize] = at(i0 - i);
        }
    }
    let g = 1.0 / nfft2 as f32;
    c1.iter_mut().for_each(|c| *c *= g);
    ifft.process(&mut c1);
    c1
}

/// Le due forme d'onda di sincronismo (8 simboli) spostate di `f0` hertz (`sync1`, `sync2`, `sync_tweak` di `sync_fst4`,
/// `fst4_decode.f90:949-983`): fasi accumulate in precisione semplice con `mod` come in Fortran, per restare sugli stessi pareggi.
fn sync_waves(nss: usize, f0: f32, fs2: f32) -> (Vec<C32>, Vec<C32>) {
    let twopi = 8.0f32 * 1f32.atan();
    let mut w1 = Vec::with_capacity(8 * nss);
    let mut w2 = Vec::with_capacity(8 * nss);
    let (mut p1, mut p2) = (0f32, 0f32);
    for i in 0..8 {
        let d1 = twopi * (f32::from(ISYNC1[i]) - 1.5) / nss as f32;
        let d2 = twopi * (f32::from(ISYNC2[i]) - 1.5) / nss as f32;
        for _ in 0..nss {
            w1.push(C32::new(p1.cos(), p1.sin()));
            w2.push(C32::new(p2.cos(), p2.sin()));
            p1 = (p1 + d1) % twopi;
            p2 = (p2 + d2) % twopi;
        }
    }
    let dphi = twopi * f0 * (1.0 / fs2);
    let mut phi = 0f32;
    for (a, b) in w1.iter_mut().zip(w2.iter_mut()) {
        let t = C32::new(phi.cos(), phi.sin());
        *a *= t;
        *b *= t;
        phi = (phi + dphi) % twopi;
    }
    (w1, w2)
}

/// `sync_fst4` con `ncoh` = 8: somma dei moduli delle correlazioni coerenti sui cinque blocchi di sincronismo.
fn sync_at(cd: &[C32], i0: usize, nss: usize, w1: &[C32], w2: &[C32]) -> f32 {
    let nz = (8 * nss) as f32;
    let np = cd.len();
    let corr = |start: usize, w: &[C32]| -> f32 {
        if start + w.len() > np {
            return 0.0;
        }
        cd[start..start + w.len()].iter().zip(w).map(|(a, b)| a * b.conj()).sum::<C32>().norm() / nz
    };
    corr(i0, w1) + corr(i0 + 38 * nss, w2) + corr(i0 + 76 * nss, w1) + corr(i0 + 114 * nss, w2) + corr(i0 + 152 * nss, w1)
}

/// `fst4_sync_search`: frequenza fine (passi di 0,1 e poi 0,02 della velocita' di simbolo) e inizio del segnale.
fn sync_search(c2: &[C32], nss: usize, fs2: f32, eme_delay: f32) -> Option<(f32, i64)> {
    let nfft2 = c2.len() as i64;
    let last_start = nfft2 - (160 * nss) as i64;
    let nspsec = fs2 as i64;
    let baud = fs2 / nss as f32;
    let (is0, ishw) = if eme_delay < 0.1 {
        ((1.5 * nspsec as f32) as i64, (1.5 * nspsec as f32) as i64)
    } else {
        (((eme_delay + 1.0) * nspsec as f32).round() as i64, (1.5 * nspsec as f32) as i64)
    };
    let lo = 1.max(is0 - ishw);
    let hi = (is0 + ishw).min(last_start);
    if hi < lo {
        return None;
    }
    let mut sbest = -1e30f32;
    let mut fcbest = 0f32;
    let mut isbest = -1i64;
    for ifr in -12..=12 {
        let fc = 0.1 * baud * ifr as f32;
        let (w1, w2) = sync_waves(nss, fc, fs2);
        let mut is = lo;
        while is <= hi {
            let s = sync_at(c2, is as usize, nss, &w1, &w2);
            if s > sbest {
                sbest = s;
                fcbest = fc;
                isbest = is;
            }
            is += 4;
        }
    }
    let fc1 = fcbest;
    let is0 = isbest;
    let lo = 1.max(is0 - 4);
    let hi = (is0 + 4).min(last_start);
    sbest = 0.0;
    for ifr in -7..=7 {
        let fc = fc1 + 0.02 * baud * ifr as f32;
        let (w1, w2) = sync_waves(nss, fc, fs2);
        for is in lo..=hi {
            let s = sync_at(c2, is as usize, nss, &w1, &w2);
            if s > sbest {
                sbest = s;
                fcbest = fc;
                isbest = is;
            }
        }
    }
    Some((fcbest, isbest))
}

/// `normalizebmet`.
fn normalize(b: &mut [f32]) {
    let n = b.len() as f32;
    let av = b.iter().sum::<f32>() / n;
    let av2 = b.iter().map(|x| x * x).sum::<f32>() / n;
    let var = av2 - av * av;
    let sig = if var > 0.0 { var.sqrt() } else { av2.sqrt() };
    if sig > 0.0 {
        b.iter_mut().for_each(|x| *x /= sig);
    }
}

/// Metriche dei 320 bit per le quattro lunghezze di correlazione.
type BitMetrics = [[f32; 2 * NN]; 4];
/// Energie dei quattro toni di ogni simbolo.
type ToneEnergy = [[f32; 4]; NN];

/// Metriche dei bit con correlazioni su 1, 2, 4 e 8 simboli, energie dei toni per il rapporto e qualita' del sincronismo
/// (`get_fst4_bitmetrics_owned`). `None` se il sincronismo e' cattivo.
fn bitmetrics(cd: &[C32], nss: usize) -> Option<(BitMetrics, ToneEnergy)> {
    const GRAY: [usize; 4] = [0, 1, 3, 2];
    let twopi = 8.0f32 * 1f32.atan();
    let ci: Vec<Vec<C32>> = (0..4)
        .map(|t| {
            let dp = (t as f32 - 1.5) * (twopi / nss as f32);
            let mut phi = 0f32;
            (0..nss)
                .map(|_| {
                    let c = C32::new(phi.cos(), phi.sin());
                    phi = (phi + dp) % twopi;
                    c
                })
                .collect()
        })
        .collect();
    let mut cs = vec![[C32::new(0.0, 0.0); 4]; NN];
    let mut s4 = [[0f32; 4]; NN];
    for k in 0..NN {
        let seg = cd.get(k * nss..(k + 1) * nss)?;
        for t in 0..4 {
            cs[k][t] = seg.iter().zip(&ci[t]).map(|(a, b)| a * b.conj()).sum();
            s4[k][t] = cs[k][t].norm_sqr();
        }
    }
    let argmax = |v: &[f32; 4]| {
        let mut b = 0;
        for i in 1..4 {
            if v[i] > v[b] {
                b = i;
            }
        }
        b
    };
    let mut nsync = 0;
    for k in 0..8 {
        for (off, sw) in [(0, &ISYNC1), (38, &ISYNC2), (76, &ISYNC1), (114, &ISYNC2), (152, &ISYNC1)] {
            if usize::from(sw[k]) == argmax(&s4[k + off]) {
                nsync += 1;
            }
        }
    }
    if nsync < 16 {
        return None;
    }
    let mut bm = [[0f32; 2 * NN]; 4];
    // massimo con il bit `bit` acceso e spento fra gli indici 0..n
    let split = |s: &[f32], bit: usize| {
        let (mut on, mut off) = (f32::MIN, f32::MIN);
        for (i, &v) in s.iter().enumerate() {
            if (i >> bit) & 1 == 1 {
                on = on.max(v);
            } else {
                off = off.max(v);
            }
        }
        on - off
    };
    let mut s8 = vec![0f32; 65_536];
    for k in (0..NN).step_by(8) {
        let mut c1 = [[C32::new(0.0, 0.0); 8]; 4];
        for m in 0..8 {
            let mut s = [0f32; 4];
            for n in 0..4 {
                c1[n][m] = cs[k + m][GRAY[n]];
                s[n] = c1[n][m].norm();
            }
            let ipt = 2 * k + 2 * m;
            for ib in 0..2 {
                bm[0][ipt + ib] = split(&s, 1 - ib);
            }
        }
        let mut c2 = [[C32::new(0.0, 0.0); 4]; 16];
        for m in 0..4 {
            let mut s = [0f32; 16];
            for i in 0..4 {
                for j in 0..4 {
                    let is = i * 4 + j;
                    c2[is][m] = c1[i][2 * m] - c1[j][2 * m + 1];
                    s[is] = c2[is][m].norm();
                }
            }
            let ipt = 2 * k + 4 * m;
            for ib in 0..4 {
                bm[1][ipt + ib] = split(&s, 3 - ib);
            }
        }
        let mut c4 = [[C32::new(0.0, 0.0); 2]; 256];
        for m in 0..2 {
            let mut s = [0f32; 256];
            for i in 0..16 {
                for j in 0..16 {
                    let is = i * 16 + j;
                    c4[is][m] = c2[i][2 * m] + c2[j][2 * m + 1];
                    s[is] = c4[is][m].norm();
                }
            }
            let ipt = 2 * k + 8 * m;
            for ib in 0..8 {
                bm[2][ipt + ib] = split(&s, 7 - ib);
            }
        }
        for i in 0..256 {
            for j in 0..256 {
                s8[i * 256 + j] = (c4[i][0] + c4[j][1]).norm();
            }
        }
        let mut on = [f32::MIN; 16];
        let mut off = [f32::MIN; 16];
        for (idx, &v) in s8.iter().enumerate() {
            for b in 0..16 {
                if (idx >> b) & 1 == 1 {
                    if v > on[b] {
                        on[b] = v;
                    }
                } else if v > off[b] {
                    off[b] = v;
                }
            }
        }
        let ipt = 2 * k;
        for ib in 0..16 {
            bm[3][ipt + ib] = on[15 - ib] - off[15 - ib];
        }
    }
    const P1: [u8; 16] = [0, 0, 0, 1, 1, 0, 1, 1, 0, 1, 0, 0, 1, 1, 1, 0];
    const P2: [u8; 16] = [1, 1, 1, 0, 0, 1, 0, 0, 1, 0, 1, 1, 0, 0, 0, 1];
    let mut q = 0;
    for (start, p) in [(0, &P1), (76, &P2), (152, &P1), (228, &P2), (304, &P1)] {
        for i in 0..16 {
            if u8::from(bm[0][start + i] >= 0.0) == p[i] {
                q += 1;
            }
        }
    }
    if q < 46 {
        return None;
    }
    for b in bm.iter_mut() {
        normalize(b);
        b.iter_mut().for_each(|x| *x *= 2.83);
    }
    Some((bm, s4))
}

/// Parametri comuni di un periodo per la decodifica dei candidati.
struct Ctx<'a> {
    big: &'a [C32],
    nfft1: usize,
    nfft2: usize,
    nss: usize,
    fs2: f32,
    baud: f32,
    ifft: Arc<dyn Fft<f32>>,
    opt: &'a Options,
    apbits: Option<[f32; 77]>,
    ap_dx: bool,
}

/// Un messaggio uscito da un candidato, prima di spacchettarlo per davvero.
struct Hit {
    bits: Bits77,
    wspr: bool,
    ap: u8,
    keff: u8,
    xsig: f32,
    sync: f32,
    xdt: f32,
    fsig: f32,
    base: f32,
}

/// Le ipotesi AP per passo del QSO (`naptypes`, `fst4_decode.f90:349-354`) e quante (`nappasses`).
const NAPTYPES: [[u8; 4]; 6] = [[1, 2, 0, 0], [2, 3, 0, 0], [2, 3, 0, 0], [3, 6, 0, 0], [3, 6, 0, 0], [3, 1, 2, 0]];
const NAPPASSES: [usize; 6] = [2, 2, 2, 2, 2, 3];

/// I 77 bit (mescolati con `rvec`, +-1) di `MYCALL DXCALL RR73` per le ipotesi AP, e se il DX e' noto (`fst4_decode.f90:365-392`).
fn ap_bits(my: &str, dx: &str, hashes: &CallHashes) -> (Option<[f32; 77]>, bool) {
    let my = my.trim().to_ascii_uppercase();
    let dx = dx.trim().to_ascii_uppercase();
    if my.len() < 3 {
        return (None, false);
    }
    let (his, has_dx) = if dx.len() < 3 { (my.clone(), false) } else { (dx, true) };
    let msg = format!("{my} {his} RR73");
    let mut h = hashes.clone();
    let Ok(bits) = pack(&msg, &mut h) else { return (None, false) };
    if message_type(&bits).0 != 1 || unpack(&bits, &mut h).ok().as_deref() != Some(msg.as_str()) {
        return (None, false);
    }
    let mut out = [0f32; 77];
    for i in 0..77 {
        out[i] = if (bits[i] + RVEC[i]) % 2 == 1 { 1.0 } else { -1.0 };
    }
    (Some(out), has_dx)
}

/// `true` se un messaggio di FST4W spacchettato contiene un nominativo/locatore gia' sentito (accettazione della decodifica con
/// Keff 50, che non ha CRC: `fst4_decode.f90:811-819`).
fn known(msg: &str, calls: &[String]) -> bool {
    calls.iter().any(|c| !c.trim().is_empty() && msg.contains(c.trim()))
}

/// Prova un candidato (spostamenti di un campione, metriche su 1, 2, 4 e 8 simboli, ipotesi AP): il ciclo interno di
/// `decode_kernel` (`fst4_decode.f90:647-916`).
fn try_candidate(ctx: &Ctx, cand: &Cand, ldpc: &mut (Ldpc, Ldpc), hashes: &CallHashes, calls: &[String]) -> Option<Hit> {
    let opt = ctx.opt;
    let nss = ctx.nss;
    let nspsec = ctx.fs2.round() as i64;
    let xdt = if opt.period_s == 15 {
        (cand.is as f32 - nspsec as f32 / 2.0) / ctx.fs2
    } else {
        (cand.is - nspsec) as f32 / ctx.fs2
    };
    let c2 = downsample(ctx.big, ctx.nfft1, ctx.nfft2, cand.fc, 4.0 * ctx.baud, &ctx.ifft);
    let jitters: &[i64] = if opt.depth <= 1 { &[0] } else { &[0, 1, -1] };
    let nblock = 4usize;
    let do_k50 = opt.depth >= 3;
    for &off in jitters {
        let is0 = cand.is + off;
        let iend = is0 + (160 * nss) as i64 - 1;
        if is0 < 0 || iend > ctx.nfft2 as i64 - 1 {
            continue;
        }
        let frame = &c2[is0 as usize..=iend as usize];
        let Some((bm, s4)) = bitmetrics(frame, nss) else { continue };
        let mut llrs = [[0f32; NCW]; 4];
        for (il, l) in llrs.iter_mut().enumerate() {
            l[0..60].copy_from_slice(&bm[il][16..76]);
            l[60..120].copy_from_slice(&bm[il][92..152]);
            l[120..180].copy_from_slice(&bm[il][168..228]);
            l[180..240].copy_from_slice(&bm[il][244..304]);
        }
        let apmag = llrs[3].iter().fold(0f32, |a, x| a.max(x.abs())) * 1.1;
        let qp = usize::from(opt.qso_progress.min(5));
        let mut ntmax = nblock + NAPPASSES[qp];
        if opt.ap_cq_only {
            ntmax = nblock + 1;
        }
        if opt.depth <= 1 || opt.wspr {
            ntmax = nblock;
        }
        for itry in 1..=ntmax {
            let mut apmask = [0u8; NCW];
            let mut llr;
            let mut iaptype = 0u8;
            if itry <= nblock {
                llr = llrs[itry - 1];
            } else {
                llr = llrs[nblock - 1];
                iaptype = if opt.ap_cq_only { 1 } else { NAPTYPES[qp][itry - nblock - 1] };
                let has_my = ctx.apbits.is_some();
                if iaptype >= 2 && !has_my {
                    continue;
                }
                if iaptype >= 3 && !ctx.ap_dx {
                    continue;
                }
                const CQ: [u8; 29] = [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0];
                const RRR: [u8; 19] = [0, 1, 1, 1, 1, 1, 1, 0, 1, 0, 0, 1, 0, 0, 1, 0, 0, 0, 1];
                const R73: [u8; 19] = [0, 1, 1, 1, 1, 1, 1, 0, 1, 0, 0, 1, 0, 1, 0, 0, 0, 0, 1];
                let pm = |b: u8, r: u8| if (b + r) % 2 == 1 { 1.0 } else { -1.0 };
                let ap = ctx.apbits.unwrap_or([0.0; 77]);
                match iaptype {
                    1 => {
                        for i in 0..29 {
                            apmask[i] = 1;
                            llr[i] = apmag * pm(CQ[i], RVEC[i]);
                        }
                    }
                    2 | 3 => {
                        let n = if iaptype == 2 { 29 } else { 58 };
                        for i in 0..n {
                            apmask[i] = 1;
                            llr[i] = apmag * ap[i];
                        }
                    }
                    4..=6 => {
                        for i in 0..77 {
                            apmask[i] = 1;
                        }
                        for i in 0..58 {
                            llr[i] = apmag * ap[i];
                        }
                        for i in 0..19 {
                            llr[58 + i] = match iaptype {
                                4 => apmag * pm(RRR[i], RVEC[58 + i]),
                                5 => apmag * pm(R73[i], RVEC[58 + i]),
                                _ => apmag * ap[58 + i],
                            };
                        }
                    }
                    _ => continue,
                }
            }
            let mut found: Option<(Bits77, u8, String)> = None;
            if !opt.wspr {
                let d = ldpc.0.decode(&llr, 91, 2, 3, &apmask);
                if d.cw.iter().all(|&b| b == 0) {
                    continue;
                }
                if d.nharderror < 0 {
                    continue;
                }
                let mut c77 = [0u8; 77];
                for i in 0..77 {
                    c77[i] = (d.message[i] + RVEC[i]) % 2;
                }
                if let Some(t) = unpack_any(&c77, &mut hashes.clone()) {
                    found = Some((c77, 91, t));
                }
            } else {
                let d = ldpc.1.decode(&llr, 66, 2, 3, &apmask);
                let mut ok = None;
                if d.nharderror >= 0 {
                    if d.cw.iter().all(|&b| b == 0) {
                        continue;
                    }
                    let c77 = wspr_c77(&d.message);
                    if let Some(t) = fst4msg::unpack(&c77, &mut hashes.clone()) {
                        ok = Some((c77, 66, t));
                    }
                }
                if ok.is_none() && do_k50 {
                    let d = ldpc.1.decode(&llr, 50, 1, 4, &apmask);
                    if d.cw.iter().all(|&b| b == 0) {
                        continue;
                    }
                    if d.nharderror >= 0 {
                        let c77 = wspr_c77(&d.message);
                        if let Some(t) = fst4msg::unpack(&c77, &mut hashes.clone())
                            && known(&t, calls)
                        {
                            ok = Some((c77, 50, t));
                        }
                    }
                }
                found = ok;
            }
            let Some((bits, keff, text)) = found else { continue };
            // energia del segnale sui toni del messaggio (`fst4_decode.f90:837-856`)
            let msgbits = if opt.wspr { bits74(&bits) } else { bits101(&bits) };
            let ld = if opt.wspr { &ldpc.1 } else { &ldpc.0 };
            let itone = tones_from_bits(ld, &msgbits);
            let xsig: f32 = (0..NN).map(|i| s4[i][usize::from(itone[i])]).sum();
            let hit = Hit {
                bits,
                wspr: opt.wspr,
                ap: iaptype,
                keff,
                xsig,
                sync: cand.sync,
                xdt,
                fsig: cand.fc - 1.5 * ctx.baud,
                base: cand.base,
            };
            let _ = text;
            return Some(hit);
        }
    }
    None
}

/// I 77 bit di un messaggio di FST4W: i 50 decodificati e la coda del tipo 0.6 (`c77(51:77)`, `fst4_decode.f90:765-766`).
fn wspr_c77(m74: &[u8]) -> Bits77 {
    let mut c77 = [0u8; 77];
    c77[..50].copy_from_slice(&m74[..50]);
    for (i, ch) in "000000000000000000000110000".bytes().enumerate() {
        c77[50 + i] = ch - b'0';
    }
    c77
}

/// Rapporto in 2500 Hz (`fst4_decode.f90:857-882`).
fn snr_db(xsig: f32, base: f32, period_s: u32, nsps: usize) -> (i32, f32) {
    let calfac = match period_s {
        15 => 800.0,
        30 => 600.0,
        60 => 430.0,
        120 => 390.0,
        300 => 340.0,
        900 | 1800 => 320.0,
        _ => 430.0,
    };
    let arg = calfac * xsig / base - 1.0;
    let x = if arg > 0.0 { 10.0 * arg.log10() + 10.0 * (1.46f32 / 2500.0).log10() + 10.0 * (8200.0 / nsps as f32).log10() } else { -99.9 };
    (x.round() as i32, x)
}

/// Esegue `f` sugli elementi con al massimo `threads` thread, mantenendo l'ordine.
fn par_map<T: Sync, R: Send>(items: &[T], threads: usize, f: impl Fn(&T) -> R + Sync) -> Vec<R> {
    par_map_with(items, threads, || (), |_, x| f(x))
}

/// Come [`par_map`], con uno stato di lavoro per thread creato da `init`. Gli elementi si distribuiscono a righe alterne per
/// bilanciare il carico (i primi candidati sono i piu' forti).
fn par_map_with<T: Sync, R: Send, S>(items: &[T], threads: usize, init: impl Fn() -> S + Sync, f: impl Fn(&mut S, &T) -> R + Sync) -> Vec<R> {
    let threads = threads.max(1).min(items.len().max(1));
    if threads <= 1 {
        let mut s = init();
        return items.iter().map(|x| f(&mut s, x)).collect();
    }
    let mut parts: Vec<Vec<(usize, R)>> = std::thread::scope(|sc| {
        let handles: Vec<_> = (0..threads)
            .map(|t| {
                let (init, f) = (&init, &f);
                sc.spawn(move || {
                    let mut s = init();
                    items.iter().enumerate().skip(t).step_by(threads).map(|(i, x)| (i, f(&mut s, x))).collect::<Vec<_>>()
                })
            })
            .collect();
        handles.into_iter().map(|h| h.join().unwrap_or_default()).collect()
    });
    let mut all: Vec<(usize, R)> = parts.iter_mut().flat_map(std::mem::take).collect();
    all.sort_by_key(|(i, _)| *i);
    all.into_iter().map(|(_, r)| r).collect()
}

/// Decodifica un periodo di FST4 o FST4W (`decode_kernel`). `audio` e' a 12 kHz dall'inizio del periodo (si completa con zeri);
/// la scala e' libera (le misure sono rapporti), salvo il «NB» che ragiona in campioni a 16 bit con fondo scala 1.0.
pub fn decode(audio: &[f32], options: &Options, hashes: &mut CallHashes, state: &mut State) -> Vec<Decoded> {
    let opt = options;
    let Some((nsps, ndown, nfft1)) = period_params(opt.period_s) else { return Vec::new() };
    if opt.wspr && !FST4W_PERIODS.contains(&opt.period_s) {
        return Vec::new();
    }
    let nfft2 = nfft1 / ndown;
    let nss = nsps / ndown;
    let fs2 = FS / ndown as f32;
    let baud = FS / nsps as f32;
    let mut x: Vec<f32> = audio.iter().take(nfft1).map(|v| v * 32768.0).collect();
    x.resize(nfft1, 0.0);
    blanker(&mut x, opt.blanker_pct);
    let mut planner = FftPlanner::<f32>::new();
    let big = real_fft(&x, nfft1, &mut planner);
    drop(x);
    let ifft = planner.plan_fft_inverse(nfft2);

    // finestre di ricerca (`fst4_decode.f90:528-547`, e `mainwindow.cpp:5453-5462` per il «Single decode»)
    let (nfqso, ntol) = (opt.rx_hz, opt.tol_hz);
    let (mut nfa, mut nfb) = if opt.single_decode { (nfqso - ntol, nfqso + ntol) } else { (opt.f_low_hz, opt.f_high_hz) };
    let (fa, fb);
    if opt.wspr {
        nfa = 100.max(nfqso - ntol - 100);
        nfb = 4800.min(nfqso + ntol + 100);
        fa = 100f32.max((nfqso as f32 + 1.5 * baud - ntol as f32).round());
        fb = 4800f32.min((nfqso as f32 + 1.5 * baud + ntol as f32).round());
    } else {
        fa = 100f32.max((nfa as f32 + 1.5 * baud).round());
        fb = 4800f32.min((nfb as f32 + 1.5 * baud).round());
        nfa = 100.max(nfa - 100);
        nfb = 4800.min(nfb + 100);
    }
    let minsync = if opt.period_s == 15 { 1.15 } else { 1.20 };
    let maxcand = opt.max_candidates.clamp(1, MAXCAND);
    let mut cands = candidates(&big, nfft1, nsps, fa, fb, nfa, nfb, minsync, maxcand);

    // sincronismo fine di ogni candidato
    let synced = par_map(&cands, opt.threads, |c| {
        let c2 = downsample(&big, nfft1, nfft2, c.f0, 4.0 * baud, &ifft);
        sync_search(&c2, nss, fs2, opt.eme_delay)
    });
    for (c, s) in cands.iter_mut().zip(synced) {
        match s {
            Some((fcbest, isbest)) if isbest >= 0 => {
                c.fc = c.f0 + fcbest;
                c.is = isbest;
            }
            _ => c.fc = -1.0,
        }
    }
    // doppioni: stessa frequenza entro 0,1 baud e stesso inizio entro 2 campioni (righe 601-615)
    for i in 0..cands.len() {
        let (fc, is) = (cands[i].fc, cands[i].is);
        for j in i + 1..cands.len() {
            if cands[j].fc > 0.0 && (cands[j].fc - fc).abs() < 0.10 * baud && (cands[j].is - is).abs() <= 2 {
                cands[j].fc = -1.0;
            }
        }
    }
    cands.retain(|c| c.fc > 0.0);
    // in FST4 senza «Single decode» prima quelli entro 20 Hz dalla frequenza di ricezione (righe 627-641)
    if !opt.wspr && !opt.single_decode {
        let target = nfqso as f32 + 1.5 * baud;
        let (mut near, far): (Vec<Cand>, Vec<Cand>) = cands.into_iter().partition(|c| (c.fc - target).abs() <= 20.0);
        near.extend(far);
        cands = near;
    }

    let (apbits, ap_dx) = if opt.wspr { (None, false) } else { ap_bits(&opt.my_call, &opt.dx_call, hashes) };
    let ctx = Ctx { big: &big, nfft1, nfft2, nss, fs2, baud, ifft: ifft.clone(), opt, apbits, ap_dx };
    let snapshot = hashes.clone();
    let calls = state.known_calls.clone();
    // FST4W in ordine (i nominativi nuovi servono ai candidati successivi), FST4 in parallelo
    let threads = if opt.wspr { 1 } else { opt.threads };
    let hits = par_map_with(
        &cands,
        threads,
        || (Ldpc::new(Code::C101), Ldpc::new(Code::C74)),
        |l, c| try_candidate(&ctx, c, l, &snapshot, &calls),
    );
    let mut out: Vec<Decoded> = Vec::new();
    let mut seen: Vec<(String, Option<u32>)> = Vec::new();
    for hit in hits.into_iter().flatten() {
        let text = if hit.wspr { fst4msg::unpack(&hit.bits, hashes) } else { unpack_any(&hit.bits, hashes) };
        let Some(text) = text else { continue };
        let unresolved = (hit.wspr && text.contains("<...>")).then(|| hit.bits[..22].iter().fold(0u32, |a, &b| (a << 1) | u32::from(b)));
        if seen.iter().any(|(t, h)| *t == text && *h == unresolved) || seen.len() >= MAXCAND {
            continue;
        }
        seen.push((text.clone(), unresolved));
        if hit.wspr && hit.keff == 66 && opt.depth >= 3 {
            // nominativo e locatore dei messaggi di tipo 1 per la decodifica con Keff 50 (righe 768-791)
            let mut w = text.split(' ');
            let wpart = match (w.next(), w.next()) {
                (Some(a), Some(b)) => format!("{a} {b} "),
                _ => String::new(),
            };
            let wpart = wpart.trim_end().to_string();
            if !wpart.contains('/') && !wpart.contains('<') && !state.known_calls.iter().any(|c| c.contains(&wpart)) {
                if state.known_calls.len() >= MAXWCALLS {
                    state.known_calls.remove(0);
                }
                state.known_calls.push(wpart);
            }
        }
        let (snr, _) = snr_db(hit.xsig, hit.base, opt.period_s, nsps);
        out.push(Decoded { text, freq_hz: hit.fsig, dt_s: hit.xdt, snr_db: snr, sync: hit.sync, ap: hit.ap, keff: hit.keff });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Rng(u64);
    impl Rng {
        fn next(&mut self) -> f64 {
            self.0 = self.0.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
            ((self.0 >> 11) as f64) / ((1u64 << 53) as f64)
        }
        fn gauss(&mut self) -> f64 {
            let u1 = self.next().max(1e-12);
            let u2 = self.next();
            (-2.0 * u1.ln()).sqrt() * (2.0 * std::f64::consts::PI * u2).cos()
        }
    }

    /// Un periodo a 12 kHz con il segnale (`snr_db` in 2500 Hz) a `f0` (tono 0) e `dt` secondi dopo l'inizio nominale.
    fn period(text: &str, wspr: bool, period_s: u32, f0: f64, dt: f64, snr_db: Option<f64>, seed: u64) -> Vec<f32> {
        let mut h = CallHashes::new();
        let e = encode(text, wspr, &mut h).unwrap_or_else(|e| panic!("{text}: {e}"));
        let n = period_s as usize * 12_000;
        let mut rng = Rng(seed);
        let sigma = 0.02;
        let mut audio: Vec<f32> = match snr_db {
            Some(_) => (0..n).map(|_| (sigma * rng.gauss()) as f32).collect(),
            None => vec![0.0; n],
        };
        let amp = match snr_db {
            Some(s) => (sigma * (2.0 * 10f64.powf(s / 10.0) * 2500.0 / 6000.0).sqrt()) as f32,
            None => 0.1,
        };
        let wave = modulate(&e.tones, f0, period_s, amp, 12_000.0);
        let start = ((start_s(period_s) + dt) * 12_000.0).round() as usize;
        for (i, v) in wave.iter().enumerate() {
            if let Some(a) = audio.get_mut(start + i) {
                *a += v;
            }
        }
        audio
    }

    fn opts(period_s: u32, wspr: bool, f0: f64) -> Options {
        Options {
            period_s,
            wspr,
            f_low_hz: (f0 - 200.0) as i32,
            f_high_hz: (f0 + 200.0) as i32,
            rx_hz: f0 as i32,
            tol_hz: if wspr { 100 } else { 20 },
            threads: 2,
            ..Options::default()
        }
    }

    #[test]
    fn i_toni_e_i_bit_tornano_indietro_in_ogni_periodo() {
        // codifica -> toni -> bit duri (Gray inversa) -> LDPC senza rumore, per FST4 e FST4W in tutti i periodi
        for &(text, wspr) in &[("CQ IU3ABC IO91", false), ("IU3ABC K1ABC -15", false), ("IU3ABC IO91 37", true)] {
            let mut h = CallHashes::new();
            let e = encode(text, wspr, &mut h).unwrap_or_else(|e| panic!("{e}"));
            assert_eq!(e.text, text);
            let code = if wspr { Code::C74 } else { Code::C101 };
            let mut l = Ldpc::new(code);
            let mut llr = [0f32; NCW];
            let data: Vec<u8> = e.tones.iter().enumerate().filter(|(i, _)| !matches!(i, 0..=7 | 38..=45 | 76..=83 | 114..=121 | 152..=159)).map(|(_, &t)| t).collect();
            for (i, &t) in data.iter().enumerate() {
                let (b0, b1) = match t {
                    0 => (0, 0),
                    1 => (0, 1),
                    3 => (1, 0),
                    _ => (1, 1),
                };
                llr[2 * i] = if b0 == 1 { 5.0 } else { -5.0 };
                llr[2 * i + 1] = if b1 == 1 { 5.0 } else { -5.0 };
            }
            let d = l.decode(&llr, if wspr { 66 } else { 91 }, 2, 3, &[0; NCW]);
            assert!(d.nharderror >= 0);
            let c77 = if wspr {
                wspr_c77(&d.message)
            } else {
                let mut c = [0u8; 77];
                for i in 0..77 {
                    c[i] = (d.message[i] + RVEC[i]) % 2;
                }
                c
            };
            let back = if wspr { fst4msg::unpack(&c77, &mut h) } else { unpack_any(&c77, &mut h) };
            assert_eq!(back.as_deref(), Some(text));
            for &p in FST4_PERIODS.iter() {
                let w = modulate(&e.tones, 1500.0, p, 1.0, 48_000.0);
                assert_eq!(w.len(), NN * nsps(p).unwrap_or(0) * 4);
                assert!(w.iter().all(|v| v.abs() <= 1.0));
            }
        }
    }

    #[test]
    fn fst4w_rifiuta_i_messaggi_non_wspr() {
        let mut h = CallHashes::new();
        assert!(encode("CQ IU3ABC IO91", true, &mut h).is_err());
        assert!(encode("IU3ABC IO91 37", true, &mut h).is_ok());
    }

    #[test]
    fn decodifica_senza_rumore_fst4_15_30_60_120() {
        for (k, &p) in [15u32, 30, 60, 120].iter().enumerate() {
            let f0 = 1400.0 + 37.0 * k as f64;
            let audio = period("CQ IU3ABC IO91", false, p, f0, 0.3, Some(-5.0), 10 + k as u64);
            let mut h = CallHashes::new();
            let out = decode(&audio, &opts(p, false, f0), &mut h, &mut State::default());
            let d = out.iter().find(|d| d.text == "CQ IU3ABC IO91").unwrap_or_else(|| panic!("FST4-{p}: {out:?}"));
            assert!((f64::from(d.freq_hz) - f0).abs() < 1.0, "FST4-{p} f {}", d.freq_hz);
            assert!((d.dt_s - 0.3).abs() < 0.1, "FST4-{p} dt {}", d.dt_s);
            assert!((d.snr_db + 5).abs() <= 3, "FST4-{p} snr {}", d.snr_db);
        }
    }

    #[test]
    fn fst4w_120_andata_e_ritorno() {
        for (k, &p) in [120u32].iter().enumerate() {
            let audio = period("IU3ABC IO91 37", true, p, 1500.0, -0.2, Some(-15.0), 40 + k as u64);
            let mut h = CallHashes::new();
            let mut st = State::default();
            let out = decode(&audio, &opts(p, true, 1500.0), &mut h, &mut st);
            let d = out.iter().find(|d| d.text == "IU3ABC IO91 37").unwrap_or_else(|| panic!("FST4W-{p}: {out:?}"));
            assert!((d.freq_hz - 1500.0).abs() < 1.0);
            assert!((d.dt_s + 0.2).abs() < 0.2, "dt {}", d.dt_s);
            assert_eq!(st.known_calls, vec!["IU3ABC IO91".to_string()]);
        }
    }

    /// I periodi lunghi (da 5 a 30 minuti di audio, FFT fino a 21,6 milioni di punti): lenti fuori da `--release`, si lanciano a mano con
    /// `cargo test --release -p jt fst4 -- --ignored`.
    #[test]
    #[ignore]
    fn andata_e_ritorno_periodi_lunghi() {
        for (k, &(p, wspr)) in [(300u32, false), (900, false), (1800, false), (300, true), (900, true), (1800, true)].iter().enumerate() {
            let text = if wspr { "PJ4/K1ABC 27" } else { "IU3ABC K1ABC R-25" };
            let audio = period(text, wspr, p, 1500.0, 0.2, Some(-20.0), 60 + k as u64);
            let mut h = CallHashes::new();
            let out = decode(&audio, &opts(p, wspr, 1500.0), &mut h, &mut State::default());
            let d = out.iter().find(|d| d.text == text).unwrap_or_else(|| panic!("{p} {wspr}: {out:?}"));
            assert!((d.freq_hz - 1500.0).abs() < 0.5, "{p}: f {}", d.freq_hz);
            assert!((d.dt_s - 0.2).abs() < 0.3, "{p}: dt {}", d.dt_s);
            assert!((d.snr_db + 20).abs() <= 2, "{p}: snr {}", d.snr_db);
        }
    }

    #[test]
    fn segnale_debole_vicino_alla_soglia_fst4_60() {
        // WSJT-X da' FST4-60 al 50% attorno a -24 dB (Quick Start Guide): a -21 dB deve uscire
        let audio = period("IU3ABC K1ABC FN42", false, 60, 1500.0, 0.0, Some(-21.0), 77);
        let mut h = CallHashes::new();
        let out = decode(&audio, &opts(60, false, 1500.0), &mut h, &mut State::default());
        assert!(out.iter().any(|d| d.text == "IU3ABC K1ABC FN42"), "{out:?}");
    }

    #[test]
    fn solo_rumore_niente_decodifiche() {
        let mut rng = Rng(5);
        let audio: Vec<f32> = (0..15 * 12_000).map(|_| (0.02 * rng.gauss()) as f32).collect();
        let mut h = CallHashes::new();
        let mut o = opts(15, false, 1500.0);
        o.f_low_hz = 200;
        o.f_high_hz = 2200;
        assert!(decode(&audio, &o, &mut h, &mut State::default()).is_empty());
    }
}
