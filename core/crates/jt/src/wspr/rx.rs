//! Ricezione WSPR-2: traduzione di `lib/wsprd/wsprd.c` di WSJT-X 3.3.0-beta6 (K9AN, K1JT).
//!
//! 1. `readwavfile` (`wsprd.c:110-194`): i primi 114 s a 12 kHz, FFT di 1474560 punti, banda di 375 Hz attorno a 1500 Hz riportata in base
//!    (46080 campioni complessi a 375 Hz);
//! 2. per ogni passata (`wsprd.c:1064-1556`): spettro medio a mezzo simbolo, rumore al 30esimo percentile, candidati sui massimi locali (e con
//!    `-d` uno ogni tre cesti sopra la soglia), ricerca grossolana di frequenza, ritardo e deriva sullo spettrogramma, affinamento con
//!    `sync_and_demodulate`, demodulazione non coerente a blocchi di 1, 2, 3 simboli e bit per bit (`noncoherent_sequence_detection`),
//!    Fano, OSD se richiesto, sottrazione del segnale decodificato (`subtract_signal2`);
//! 3. tre passate (la seconda si salta se la prima non trova niente), doppioni scartati per nominativo e frequenza entro 4 Hz, risultati in
//!    ordine di frequenza.
//!
//! I calcoli sono in `f32` come nel C (FFTW in singola precisione); dove il C mescola `double` si fa lo stesso.

// cicli con l'indice come nel C di wsprd, per confrontarli riga per riga
#![allow(clippy::needless_range_loop)]

use super::fano::{BIAS, DELTA, NSYM, deinterleave, fano, metric_table};
use super::osd::osdwspr;
use super::pack::{HashTable, nhash, unpack, unpack_call, unpack_grid, unpack_pfx, unpack50};
use super::tx::tones;
use super::SYNC;
use rustfft::FftPlanner;
use rustfft::num_complex::Complex;
use std::time::Instant;

/// Campioni complessi dopo il sottocampionamento (`nfft2`, `wsprd.c:116`).
const NFFT2: usize = 46_080;
/// Lunghezza dei vettori `idat`/`qdat` (`maxpts`, `wsprd.c:766`): oltre `NFFT2` restano zeri, che lo spettrogramma legge.
const MAXPTS: usize = 65_536;
/// Campioni audio letti: 114 s (`npoints`, `wsprd.c:123`).
const NPOINTS_AUDIO: usize = 114 * 12_000;
/// Passo in frequenza dello spettro: 375 / 256 / 2 Hz (`df`, `wsprd.c:776`).
const DF: f32 = 375.0 / 256.0 / 2.0;
const DT: f32 = 1.0 / 375.0;

/// Opzioni di wsprd come le passa WSJT-X (`widgets/mainwindow.cpp:2882-2887`).
#[derive(Debug, Clone)]
pub struct Options {
    /// `-C`: cicli per bit del Fano (wsprd 10000, WSJT-X 500 alle profondita' 2 e 3).
    pub maxcycles: u32,
    /// `-o`: profondita' dell'OSD (`None` = spento, WSJT-X 4 alle profondita' 2 e 3).
    pub osd_depth: Option<i32>,
    /// `-d`: piu' candidati (WSJT-X alla profondita' 3, quella predefinita).
    pub more_candidates: bool,
    /// `-q`: niente ricerca del ritardo attorno al migliore (profondita' 1).
    pub quick: bool,
    /// `-B` spento: tre passate con la demodulazione a blocchi (`-B` le riduce a due, profondita' 1).
    pub block_demod: bool,
    /// `-s` spento: sottrazione dei segnali decodificati e piu' passate.
    pub subtraction: bool,
    /// `-w`: banda di +-150 Hz invece di +-110 Hz.
    pub wideband: bool,
    /// Scostamento della metrica del Fano (`-z`, predefinito 0,45).
    pub bias: f32,
    /// Limite di tempo: oltre si salta il resto dei candidati (non c'e' in wsprd).
    pub deadline: Option<Instant>,
}

impl Default for Options {
    /// La profondita' predefinita di WSJT-X («Deep», `NDepth` 3, `mainwindow_settings.cpp:529`): `-C 500 -o 4 -d`.
    fn default() -> Self {
        Options::depth(3)
    }
}

impl Options {
    /// Le opzioni per la profondita' di decodifica di WSJT-X (`m_ndepth & 7`, `mainwindow.cpp:2885-2887`): 1 `-qB`, 2 `-C 500 -o 4`,
    /// 3 `-C 500 -o 4 -d`.
    pub fn depth(ndepth: u8) -> Self {
        let base = Options {
            maxcycles: 10_000,
            osd_depth: None,
            more_candidates: false,
            quick: false,
            block_demod: true,
            subtraction: true,
            wideband: false,
            bias: BIAS,
            deadline: None,
        };
        match ndepth {
            1 => Options { quick: true, block_demod: false, ..base },
            2 => Options { maxcycles: 500, osd_depth: Some(4), ..base },
            _ => Options { maxcycles: 500, osd_depth: Some(4), more_candidates: true, ..base },
        }
    }
}

/// Una decodifica.
#[derive(Debug, Clone, PartialEq)]
pub struct Decode {
    /// Rapporto segnale/rumore in 2500 Hz, dallo spettro medio (`candidates[j].snr`).
    pub snr_db: f32,
    /// Ritardo dall'inizio nominale (1 s dopo l'inizio del periodo): `shift * dt - 1`.
    pub dt_s: f32,
    /// Frequenza audio del centro del segnale: 1500 Hz piu' lo scostamento trovato.
    pub audio_hz: f64,
    /// Deriva in Hz sul messaggio.
    pub drift_hz: f32,
    /// Sincronismo (0-1).
    pub sync: f32,
    pub message: String,
    /// Nominativo per i doppioni e per la tabella degli hash.
    pub call: String,
    /// Cicli del Fano.
    pub cycles: u32,
    /// Scostamento del ritardo che ha decodificato (campioni a 375 Hz).
    pub jitter: i32,
    /// Blocco della demodulazione (1-3, 4 = bit per bit).
    pub blocksize: i32,
    /// Passata (da 0).
    pub pass: usize,
    /// Decodificato con l'OSD.
    pub osd: bool,
}

impl Decode {
    /// La frequenza RF del centro del segnale con la frequenza del VFO in hertz (`freq_print`, `wsprd.c:1525`).
    pub fn rf_hz(&self, dial_hz: f64) -> f64 {
        dial_hz + self.audio_hz
    }
}

/// `readwavfile` (`wsprd.c:110-194`) per WSPR-2: dall'audio a 12 kHz (valori a 16 bit diviso 32768) a `idat`/`qdat` a 375 Hz.
pub fn downsample(audio: &[f32]) -> (Vec<f32>, Vec<f32>) {
    let nfft1 = NFFT2 * 32;
    let df = 12_000.0 / nfft1 as f64;
    let i0 = (1500.0 / df + 0.5) as usize;
    let nh2 = NFFT2 / 2;
    let mut planner = FftPlanner::<f32>::new();
    let mut buf: Vec<Complex<f32>> = vec![Complex::new(0.0, 0.0); nfft1];
    for (b, &s) in buf.iter_mut().zip(audio.iter().take(NPOINTS_AUDIO)) {
        b.re = s;
    }
    planner.plan_fft_forward(nfft1).process(&mut buf);
    let mut c: Vec<Complex<f32>> = (0..NFFT2)
        .map(|i| {
            let mut j = i0 + i;
            if i > nh2 {
                j -= NFFT2;
            }
            buf[j]
        })
        .collect();
    drop(buf);
    planner.plan_fft_inverse(NFFT2).process(&mut c);
    let mut idat = vec![0f32; MAXPTS];
    let mut qdat = vec![0f32; MAXPTS];
    for (i, v) in c.iter().enumerate() {
        idat[i] = v.re / 1000.0;
        qdat[i] = v.im / 1000.0;
    }
    (idat, qdat)
}

/// I seni e coseni di un tono per i 256 campioni di un simbolo, con la ricorrenza in `f32` di `wsprd.c:253-267`.
fn tone_table(dphi: f32, n: usize, c: &mut [f32], s: &mut [f32]) {
    let cd = f64::from(dphi).cos() as f32;
    let sd = f64::from(dphi).sin() as f32;
    c[0] = 1.0;
    s[0] = 0.0;
    for j in 1..n {
        c[j] = c[j - 1] * cd - s[j - 1] * sd;
        s[j] = c[j - 1] * sd + s[j - 1] * cd;
    }
}

/// Frequenza del simbolo `i` con la deriva (`wsprd.c:235`, calcolo in `double` come nel C).
fn symbol_freq(f0: f32, drift: f32, i: usize) -> f32 {
    (f64::from(f0) + (f64::from(drift) / 2.0) * (i as f64 - 81.0) / 81.0) as f32
}

/// `twopidt` con `pi` in `float` (`wsprd.c:211`, `228`).
fn twopidt() -> f32 {
    2.0 * std::f32::consts::PI * DT
}

/// Le quattro tabelle dei toni di un simbolo di frequenza `fp`, con `n` campioni (256 o 257).
struct Tones4 {
    c: [[f32; 257]; 4],
    s: [[f32; 257]; 4],
}

impl Tones4 {
    fn new() -> Self {
        Tones4 { c: [[0.0; 257]; 4], s: [[0.0; 257]; 4] }
    }

    fn fill(&mut self, fp: f32, n: usize) {
        let df = 375.0f32 / 256.0;
        let (df15, df05) = (df * 1.5, df * 0.5);
        let tp = twopidt();
        let dphis = [tp * (fp - df15), tp * (fp - df05), tp * (fp + df05), tp * (fp + df15)];
        for (t, &dphi) in dphis.iter().enumerate() {
            tone_table(dphi, n, &mut self.c[t], &mut self.s[t]);
        }
    }

    /// Le correlazioni dei quattro toni sul simbolo che comincia a `start` (`wsprd.c:276-288`).
    fn correlate(&self, id: &[f32], qd: &[f32], np: i64, start: i64) -> [(f32, f32); 4] {
        let mut out = [(0f32, 0f32); 4];
        for j in 0..256usize {
            let k = start + j as i64;
            if k > 0 && k < np {
                let (x, y) = (id[k as usize], qd[k as usize]);
                for t in 0..4 {
                    let (c, s) = (self.c[t][j], self.s[t][j]);
                    out[t].0 = out[t].0 + x * c + y * s;
                    out[t].1 = out[t].1 - x * s + y * c;
                }
            }
        }
        out
    }
}

/// `sync_and_demodulate` di `wsprd.c:197-342`, modi 0 (ricerca del ritardo) e 1 (ricerca della frequenza). Ritorna il sincronismo migliore e
/// aggiorna frequenza e ritardo.
#[allow(clippy::too_many_arguments)]
fn sync_and_demodulate(
    id: &[f32],
    qd: &[f32],
    np: i64,
    f1: &mut f32,
    ifr: (i32, i32, f32),
    shift1: &mut i32,
    lags: (i32, i32, i32),
    drift1: f32,
    mode: u8,
) -> f32 {
    let (mut ifmin, mut ifmax, mut fstep) = ifr;
    let (mut lagmin, mut lagmax, lagstep) = lags;
    if mode == 0 {
        ifmin = 0;
        ifmax = 0;
        fstep = 0.0;
    }
    if mode == 1 {
        lagmin = *shift1;
        lagmax = *shift1;
    }
    let mut syncmax = -1e30f32;
    let mut best_shift = 0i32;
    let mut fbest = 0f32;
    let mut tones = Tones4::new();
    for ifreq in ifmin..=ifmax {
        let f0 = *f1 + ifreq as f32 * fstep;
        let mut lag = lagmin;
        while lag <= lagmax {
            let mut ss = 0f32;
            let mut totp = 0f32;
            let mut fplast = f32::NAN;
            for i in 0..NSYM {
                let fp = symbol_freq(f0, drift1, i);
                if i == 0 || fp != fplast {
                    tones.fill(fp, 256);
                    fplast = fp;
                }
                let r = tones.correlate(id, qd, np, i64::from(lag) + (i * 256) as i64);
                let p: [f32; 4] = std::array::from_fn(|t| (r[t].0 * r[t].0 + r[t].1 * r[t].1).sqrt());
                totp = totp + p[0] + p[1] + p[2] + p[3];
                let cmet = (p[1] + p[3]) - (p[0] + p[2]);
                ss = if SYNC[i] == 1 { ss + cmet } else { ss - cmet };
            }
            ss /= totp;
            if ss > syncmax {
                syncmax = ss;
                best_shift = lag;
                fbest = f0;
            }
            lag += lagstep;
        }
    }
    *shift1 = best_shift;
    *f1 = fbest;
    syncmax
}

/// `noncoherent_sequence_detection` di `wsprd.c:344-485`: simboli soft (0-255) con la stima coerente su blocchi di `nblock` simboli;
/// `bitbybit` normalizza ogni bit sul massimo.
#[allow(clippy::too_many_arguments)]
fn noncoherent_sequence_detection(
    id: &[f32],
    qd: &[f32],
    np: i64,
    f1: f32,
    lag: i32,
    drift1: f32,
    symfac: f32,
    nblock: usize,
    bitbybit: bool,
) -> [u8; NSYM] {
    let nseq = 1usize << nblock;
    let mut is = [[0f32; NSYM]; 4];
    let mut qs = [[0f32; NSYM]; 4];
    let mut cf = [[0f32; NSYM]; 4];
    let mut sf = [[0f32; NSYM]; 4];
    let mut tones = Tones4::new();
    let mut fplast = f32::NAN;
    for i in 0..NSYM {
        let fp = symbol_freq(f1, drift1, i);
        if i == 0 || fp != fplast {
            tones.fill(fp, 257);
            fplast = fp;
        }
        for t in 0..4 {
            cf[t][i] = tones.c[t][256];
            sf[t][i] = tones.s[t][256];
        }
        let r = tones.correlate(id, qd, np, i64::from(lag) + (i * 256) as i64);
        for t in 0..4 {
            is[t][i] = r[t].0;
            qs[t][i] = r[t].1;
        }
    }
    let mut fsymb = [0f32; NSYM];
    let mut p = [0f32; 512];
    let mut i = 0usize;
    while i < NSYM {
        for (j, pj) in p.iter_mut().enumerate().take(nseq) {
            let (mut xi, mut xq) = (0f32, 0f32);
            let (mut cm, mut sm) = (1f32, 0f32);
            for ib in 0..nblock {
                let b = (j >> (nblock - 1 - ib)) & 1;
                let k = i + ib;
                if k >= NSYM {
                    continue;
                }
                let itone = SYNC[k] as usize + 2 * b;
                xi = xi + is[itone][k] * cm + qs[itone][k] * sm;
                xq = xq + qs[itone][k] * cm - is[itone][k] * sm;
                let cmp = cf[itone][k] * cm - sf[itone][k] * sm;
                let smp = sf[itone][k] * cm + cf[itone][k] * sm;
                cm = cmp;
                sm = smp;
            }
            *pj = (xi * xi + xq * xq).sqrt();
        }
        for ib in 0..nblock {
            if i + ib >= NSYM {
                break;
            }
            let imask = 1usize << (nblock - 1 - ib);
            let (mut xm1, mut xm0) = (0f32, 0f32);
            for (j, &pj) in p.iter().enumerate().take(nseq) {
                if j & imask != 0 {
                    if pj > xm1 {
                        xm1 = pj;
                    }
                } else if pj > xm0 {
                    xm0 = pj;
                }
            }
            let mut v = xm1 - xm0;
            if bitbybit {
                v /= if xm1 > xm0 { xm1 } else { xm0 };
            }
            fsymb[i + ib] = v;
        }
        i += nblock;
    }
    normalize(&fsymb, symfac)
}

/// Normalizzazione dei simboli soft (`wsprd.c:473-483`): deviazione standard a `symfac`, poi da -128..127 a 0..255.
fn normalize(fsymb: &[f32; NSYM], symfac: f32) -> [u8; NSYM] {
    let (mut fsum, mut f2sum) = (0f32, 0f32);
    for &v in fsymb {
        fsum += v / 162.0;
        f2sum += v * v / 162.0;
    }
    let fac = (f2sum - fsum * fsum).sqrt();
    std::array::from_fn(|i| {
        let v = (symfac * fsymb[i] / fac).clamp(-128.0, 127.0);
        // `symbols[i] = fsymb[i] + 128` (conversione a `unsigned char` che tronca)
        let x = v + 128.0;
        if x.is_nan() { 128 } else { x as u8 }
    })
}

/// `subtract_signal2` di `wsprd.c:546-658`: toglie la componente coerente del segnale decodificato (ampiezza complessa stimata con un passa
/// basso di 360 campioni).
fn subtract_signal2(id: &mut [f32], qd: &mut [f32], np: i64, f0: f32, shift0: i32, drift0: f32, channel: &[u8; NSYM]) {
    const NSPERSYM: usize = 256;
    const NFILT: usize = 360;
    const NC2: usize = 45_000;
    let nsym = NSYM;
    let nsig = nsym * NSPERSYM;
    let pi = (4.0f64 * 1.0f64.atan()) as f32;
    let twopidt = (2.0 * f64::from(pi) * f64::from(DT)) as f32;
    let df = 375.0f32 / 256.0;
    let mut refi = vec![0f32; NC2];
    let mut refq = vec![0f32; NC2];
    let mut ci = vec![0f32; NC2];
    let mut cq = vec![0f32; NC2];
    let mut cfi = vec![0f32; NC2];
    let mut cfq = vec![0f32; NC2];
    let mut phi = 0f32;
    for i in 0..nsym {
        let cs = f32::from(channel[i]);
        let dphi = (f64::from(twopidt)
            * (f64::from(f0)
                + (f64::from(drift0) / 2.0) * (f64::from(i as f32) - nsym as f64 / 2.0) / (nsym as f64 / 2.0)
                + f64::from((cs - 1.5) * df))) as f32;
        for j in 0..NSPERSYM {
            let ii = NSPERSYM * i + j;
            refi[ii] = f64::from(phi).cos() as f32;
            refq[ii] = f64::from(phi).sin() as f32;
            phi += dphi;
        }
    }
    let mut w = [0f32; NFILT];
    let mut partialsum = [0f32; NFILT];
    let mut norm = 0f32;
    for (i, wi) in w.iter_mut().enumerate() {
        *wi = f64::from(pi * i as f32 / (NFILT - 1) as f32).sin() as f32;
        norm += *wi;
    }
    for wi in w.iter_mut() {
        *wi /= norm;
    }
    for i in 1..NFILT {
        partialsum[i] = partialsum[i - 1] + w[i];
    }
    for i in 0..nsig {
        let k = i64::from(shift0) + i as i64;
        if k > 0 && k < np {
            let k = k as usize;
            ci[i + NFILT] = id[k] * refi[i] + qd[k] * refq[i];
            cq[i + NFILT] = qd[k] * refi[i] - id[k] * refq[i];
        }
    }
    for i in NFILT / 2..NC2 - NFILT / 2 {
        let (mut a, mut b) = (0f32, 0f32);
        let base = i - NFILT / 2;
        for j in 0..NFILT {
            a += w[j] * ci[base + j];
            b += w[j] * cq[base + j];
        }
        cfi[i] = a;
        cfq[i] = b;
    }
    for i in 0..nsig {
        let norm = if i < NFILT / 2 {
            partialsum[NFILT / 2 + i]
        } else if i > nsig - 1 - NFILT / 2 {
            partialsum[NFILT / 2 + nsig - 1 - i]
        } else {
            1.0
        };
        let k = i64::from(shift0) + i as i64;
        let j = i + NFILT;
        if k > 0 && k < np {
            let k = k as usize;
            id[k] -= (cfi[j] * refi[i] - cfq[j] * refq[i]) / norm;
            qd[k] -= (cfi[j] * refq[i] + cfq[j] * refi[i]) / norm;
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct Cand {
    freq: f32,
    snr: f32,
    shift: i32,
    drift: f32,
    sync: f32,
}

/// Lo spettrogramma della passata: radice della potenza `ps[j][i]` (512 cesti per `nffts` mezzi simboli) e spettro medio
/// (`wsprd.c:1078-1100`).
fn spectrogram(id: &[f32], qd: &[f32], nffts: usize) -> (Vec<f32>, [f32; 512]) {
    let mut planner = FftPlanner::<f32>::new();
    let fft = planner.plan_fft_forward(512);
    let w: Vec<f32> = (0..512).map(|i| (0.006147931f64 * i as f64).sin() as f32).collect();
    let mut ps = vec![0f32; 512 * nffts];
    let mut psavg = [0f32; 512];
    let mut buf = vec![Complex::new(0f32, 0f32); 512];
    for i in 0..nffts {
        for j in 0..512 {
            let k = i * 128 + j;
            buf[j] = Complex::new(id[k] * w[j], qd[k] * w[j]);
        }
        fft.process(&mut buf);
        for j in 0..512 {
            let k = (j + 256) % 512;
            ps[j * nffts + i] = buf[k].re * buf[k].re + buf[k].im * buf[k].im;
        }
    }
    for i in 0..nffts {
        for j in 0..512 {
            psavg[j] += ps[j * nffts + i];
        }
    }
    for v in ps.iter_mut() {
        *v = v.sqrt();
    }
    (ps, psavg)
}

/// Il decodificatore: tiene la tabella degli hash fra un periodo e l'altro (`hashtable.txt` di wsprd).
#[derive(Debug, Clone, Default)]
pub struct Decoder {
    pub hashes: HashTable,
}

impl Decoder {
    pub fn new() -> Self {
        Self::default()
    }

    /// Decodifica un periodo: `audio` a 12 kHz che comincia all'inizio del periodo pari (almeno 114 s, i valori a 16 bit diviso 32768).
    pub fn decode(&mut self, audio: &[f32], opts: &Options) -> Vec<Decode> {
        decode_with(audio, opts, &mut self.hashes)
    }
}

/// Decodifica con una tabella degli hash data (vedi [`Decoder`]).
pub fn decode_with(audio: &[f32], opts: &Options, hashes: &mut HashTable) -> Vec<Decode> {
    let (mut idat, mut qdat) = downsample(audio);
    let np = NFFT2 as i64;
    let mettab = metric_table(opts.bias);
    let symfac = 50f32;
    let minsync1 = 0.10f32;
    let iifac = 8i32;
    let minrms = 52.0f32 * (symfac / 64.0);
    let (fmin, fmax) = if opts.wideband { (-150.0f32, 150.0f32) } else { (-110.0, 110.0) };
    let mut npasses = 3usize;
    if !opts.block_demod {
        npasses = 2;
    }
    if !opts.subtraction {
        npasses = 1;
    }
    let nffts = 4 * (NFFT2 / 512) - 1;
    let mut decodes: Vec<Decode> = Vec::new();
    let mut ndecodes_pass = 0usize;
    let out_of_time = || opts.deadline.is_some_and(|d| Instant::now() >= d);

    let mut ipass = 0usize;
    while ipass < npasses {
        if ipass == 1 && ndecodes_pass == 0 && npasses > 2 {
            ipass = 2;
        }
        let (nblocksize, maxdrift, minsync2) = if ipass < 2 { (1usize, 4i32, 0.12f32) } else { (4, 0, 0.10) };
        ndecodes_pass = 0;
        if out_of_time() {
            break;
        }

        let (ps, psavg) = spectrogram(&idat, &qdat, nffts);
        // lisciatura su 7 cesti e +-150 Hz (`wsprd.c:1101-1110`)
        let mut smspec = [0f32; 411];
        for (i, v) in smspec.iter_mut().enumerate() {
            for j in 0..7 {
                *v += psavg[256 - 205 + i + j - 3];
            }
        }
        let mut tmpsort = smspec;
        tmpsort.sort_by(f32::total_cmp);
        let noise_level = tmpsort[122];
        let min_snr = 10f64.powf(-8.0 / 10.0) as f32;
        let snr_scaling_factor = 26.3f32;
        for v in smspec.iter_mut() {
            *v = *v / noise_level - 1.0;
            if *v < min_snr {
                *v = 0.1 * min_snr;
            }
        }
        let snr_of = |v: f32| (10.0 * f64::from(v).log10()) as f32 - snr_scaling_factor;
        let mut cands: Vec<Cand> = Vec::with_capacity(200);
        for j in 1..410 {
            if smspec[j] > smspec[j - 1] && smspec[j] > smspec[j + 1] && cands.len() < 200 {
                cands.push(Cand { freq: (j as f32 - 205.0) * DF, snr: snr_of(smspec[j]), shift: 0, drift: 0.0, sync: 0.0 });
            }
        }
        if opts.more_candidates {
            for j in (0..411).step_by(3) {
                if smspec[j] > min_snr && cands.len() < 200 {
                    cands.push(Cand { freq: (j as f32 - 205.0) * DF, snr: snr_of(smspec[j]), shift: 0, drift: 0.0, sync: 0.0 });
                }
            }
        }
        cands.retain(|c| c.freq >= fmin && c.freq <= fmax);
        // ordinamento a bolle per rapporto decrescente: stabile
        cands.sort_by(|a, b| b.snr.partial_cmp(&a.snr).unwrap_or(std::cmp::Ordering::Equal));

        // stima grossolana di ritardo, frequenza e deriva (`wsprd.c:1221-1261`)
        for c in cands.iter_mut() {
            let mut smax = -1e30f32;
            let if0 = (c.freq / DF + 256.0) as i32;
            for ifr in (if0 - 2)..=(if0 + 2) {
                for k0 in -10i32..22 {
                    for idrift in -maxdrift..=maxdrift {
                        let mut ss = 0f32;
                        let mut pow = 0f32;
                        for k in 0..NSYM {
                            let ifd = (f64::from(ifr)
                                + (f64::from(k as f32) - 81.0) / 81.0 * f64::from(idrift as f32) / (2.0 * f64::from(DF)))
                                as i32;
                            let kindex = k0 + 2 * k as i32;
                            if kindex >= 0 && (kindex as usize) < nffts {
                                let at = |d: i32| ps[(ifd + d) as usize * nffts + kindex as usize];
                                let (p0, p1, p2, p3) = (at(-3), at(-1), at(1), at(3));
                                ss += (2.0 * f32::from(SYNC[k]) - 1.0) * ((p1 + p3) - (p0 + p2));
                                pow = pow + p0 + p1 + p2 + p3;
                            }
                        }
                        let sync1 = ss / pow;
                        if sync1 > smax {
                            smax = sync1;
                            c.shift = 128 * (k0 + 1);
                            c.drift = idrift as f32;
                            c.freq = (ifr - 256) as f32 * DF;
                            c.sync = sync1;
                        }
                    }
                }
            }
        }

        // affinamento (`wsprd.c:1280-1345`)
        for c in cands.iter_mut() {
            if out_of_time() {
                break;
            }
            let mut f1 = c.freq;
            let mut drift1 = c.drift;
            let mut shift1 = c.shift;
            let lagmin = shift1 - 128;
            let lagmax = shift1 + 128;
            let _ = sync_and_demodulate(&idat, &qdat, np, &mut f1, (0, 0, 0.0), &mut shift1, (lagmin, lagmax, 64), drift1, 0);
            let mut sync1 = sync_and_demodulate(&idat, &qdat, np, &mut f1, (-2, 2, 0.25), &mut shift1, (lagmin, lagmax, 64), drift1, 1);
            if ipass < 2 {
                let driftp = drift1 + 0.5;
                let syncp = sync_and_demodulate(&idat, &qdat, np, &mut f1, (0, 0, 0.0), &mut shift1, (lagmin, lagmax, 64), driftp, 1);
                let driftm = drift1 - 0.5;
                let syncm = sync_and_demodulate(&idat, &qdat, np, &mut f1, (0, 0, 0.0), &mut shift1, (lagmin, lagmax, 64), driftm, 1);
                if syncp > sync1 {
                    drift1 = driftp;
                    sync1 = syncp;
                } else if syncm > sync1 {
                    drift1 = driftm;
                    sync1 = syncm;
                }
            }
            if sync1 > minsync1 {
                let (lagmin, lagmax) = (shift1 - 32, shift1 + 32);
                let _ = sync_and_demodulate(&idat, &qdat, np, &mut f1, (0, 0, 0.0), &mut shift1, (lagmin, lagmax, 16), drift1, 0);
                sync1 = sync_and_demodulate(&idat, &qdat, np, &mut f1, (-2, 2, 0.05), &mut shift1, (lagmin, lagmax, 16), drift1, 1);
                c.freq = f1;
                c.shift = shift1;
                c.drift = drift1;
                c.sync = sync1;
            }
        }

        // doppioni (`wsprd.c:1347-1364`)
        let mut nwat: Vec<Cand> = Vec::new();
        for c in &cands {
            if let Some(k) = nwat.iter().position(|w| (c.freq - w.freq).abs() < 0.05 && (c.shift - w.shift).abs() < 16) {
                if c.sync > nwat[k].sync {
                    nwat[k] = *c;
                }
            } else if c.sync > minsync2 {
                nwat.push(*c);
            }
        }

        for c in &nwat {
            if out_of_time() {
                break;
            }
            let (f1, shift1, drift1) = (c.freq, c.shift, c.drift);
            let mut decoded: Option<([u8; 11], u32)> = None;
            let mut osd_decode = false;
            let (mut blocksize, mut bitmetric, mut jitter) = (1i32, false, 0i32);
            let mut ib = 1usize;
            'blocks: while ib <= nblocksize && decoded.is_none() {
                if ib < 4 {
                    blocksize = ib as i32;
                    bitmetric = false;
                } else {
                    blocksize = 1;
                    bitmetric = true;
                }
                let mut idt = 0i32;
                while decoded.is_none() && idt <= 128 / iifac {
                    let mut ii = (idt + 1) / 2;
                    if idt % 2 == 1 {
                        ii = -ii;
                    }
                    ii *= iifac;
                    jitter = ii;
                    let mut symbols = noncoherent_sequence_detection(
                        &idat,
                        &qdat,
                        np,
                        f1,
                        shift1 + ii,
                        drift1,
                        symfac,
                        blocksize as usize,
                        bitmetric,
                    );
                    let sq: f32 = symbols.iter().map(|&s| (f32::from(s) - 128.0).powi(2)).sum();
                    let rms = (sq / 162.0).sqrt();
                    if rms > minrms {
                        deinterleave(&mut symbols);
                        let r = fano(&symbols, &mettab, DELTA, opts.maxcycles);
                        if r.ok {
                            decoded = Some((r.data, r.cycles));
                        } else if let Some(depth) = opts.osd_depth {
                            let fs: [f32; NSYM] = std::array::from_fn(|i| f32::from(symbols[i]) - 128.0);
                            let o = osdwspr(&fs, &[0u8; NSYM], depth);
                            let hard: [u8; NSYM] = std::array::from_fn(|i| 255 * o.cw[i]);
                            let r = fano(&hard, &mettab, DELTA, opts.maxcycles);
                            // `wsprd.c:1442-1470`: si accetta solo un nominativo gia' nella tabella degli hash
                            let (n1, n2) = unpack50(&r.data);
                            let Some(mut callsign) = unpack_call(n1) else {
                                break;
                            };
                            let Some(grid) = unpack_grid(n2) else {
                                break;
                            };
                            let ntype = (n2 & 127) as i32 - 64;
                            if (0..=62).contains(&ntype) {
                                let nu = ntype % 10;
                                let mut itype = 1;
                                if !(nu == 0 || nu == 3 || nu == 7) {
                                    let mut nadd = nu;
                                    if nu > 3 {
                                        nadd = nu - 3;
                                    }
                                    if nu > 7 {
                                        nadd = nu - 7;
                                    }
                                    let n3 = (n2 / 128) as i32 + 32_768 * (nadd - 1);
                                    let Some(c2) = u32::try_from(n3).ok().and_then(|n3| unpack_pfx(n3, &callsign)) else {
                                        break;
                                    };
                                    callsign = c2;
                                    itype = 2;
                                }
                                let ih = nhash(&callsign);
                                if hashes.call(ih).unwrap_or("") == callsign
                                    && ((itype == 1 && hashes.grid(ih).unwrap_or("") == grid) || itype == 2)
                                {
                                    decoded = Some((r.data, r.cycles));
                                    osd_decode = true;
                                }
                            }
                        }
                    }
                    idt += 1;
                    if opts.quick {
                        break;
                    }
                }
                ib += 1;
                if decoded.is_some() {
                    break 'blocks;
                }
            }
            let Some((data, cycles)) = decoded else {
                continue;
            };
            ndecodes_pass += 1;
            let Some(u) = unpack(&data, hashes) else {
                continue;
            };
            if opts.subtraction && !u.noprint {
                match tones(&u.message) {
                    Some(channel) => subtract_signal2(&mut idat, &mut qdat, np, f1, shift1, drift1, &channel),
                    // `else break` di `wsprd.c:1501-1503`: si chiude la passata
                    None => break,
                }
            }
            let dupe = decodes.iter().any(|d| d.call == u.call && (f64::from(f1) - (d.audio_hz - 1500.0)).abs() < 4.0);
            if !dupe && !u.noprint && decodes.len() < 50 {
                decodes.push(Decode {
                    snr_db: c.snr,
                    dt_s: shift1 as f32 * DT - 1.0,
                    audio_hz: 1500.0 + f64::from(f1),
                    drift_hz: drift1,
                    sync: c.sync,
                    message: u.message,
                    call: u.call,
                    cycles,
                    jitter,
                    blocksize: blocksize + 3 * i32::from(bitmetric),
                    pass: ipass,
                    osd: osd_decode,
                });
            }
        }
        ipass += 1;
    }
    // in ordine di frequenza (ordinamento a bolle, stabile)
    decodes.sort_by(|a, b| a.audio_hz.partial_cmp(&b.audio_hz).unwrap_or(std::cmp::Ordering::Equal));
    decodes
}

#[cfg(test)]
mod tests {
    use super::super::tx::modulate;
    use super::*;

    struct Lcg(u64);
    impl Lcg {
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

    /// Due minuti di audio con rumore bianco (deviazione 1000 su 32768) e un segnale al rapporto `snr_db` in 2500 Hz.
    fn period(msgs: &[(&str, f64, f64, f64, f64)], seed: u64) -> Vec<f32> {
        let mut rng = Lcg(seed);
        let n = 120 * 12_000;
        let sigma = 1000.0;
        let mut audio: Vec<f64> = (0..n).map(|_| sigma * rng.gauss()).collect();
        for &(msg, snr_db, f, dt, drift) in msgs {
            let amp = (sigma * (2.0 * 10f64.powf(snr_db / 10.0) * 2500.0 / 6000.0).sqrt()) as f32;
            let t = tones(msg).unwrap_or([0; NSYM]);
            let wave = modulate(&t, f, amp, 12_000.0, drift);
            let start = ((1.0 + dt) * 12_000.0).round() as usize;
            for (i, v) in wave.iter().enumerate() {
                if start + i < n {
                    audio[start + i] += f64::from(*v);
                }
            }
        }
        audio.iter().map(|v| (v.round().clamp(-32768.0, 32767.0) / 32768.0) as f32).collect()
    }

    fn quick() -> Options {
        // nei test (senza ottimizzazioni) la profondita' 2 basta e costa meno
        Options::depth(2)
    }

    #[test]
    fn a_strong_signal_decodes_with_its_parameters() {
        let audio = period(&[("IU3ABC JN55 37", -15.0, 1520.0, 0.0, 0.0)], 1);
        let d = Decoder::new().decode(&audio, &quick());
        assert_eq!(d.len(), 1, "{d:?}");
        assert_eq!(d[0].message, "IU3ABC JN55 37");
        assert!((d[0].audio_hz - 1520.0).abs() < 0.3, "{:?}", d[0]);
        assert!(d[0].dt_s.abs() < 0.2, "{:?}", d[0]);
        assert!((d[0].snr_db + 15.0).abs() < 2.5, "{:?}", d[0]);
    }

    #[test]
    fn a_weak_drifting_signal_and_a_second_station_decode() {
        let audio = period(&[("K1ABC FN42 30", -26.0, 1450.0, 0.5, 2.0), ("G4ABC IO91 23", -22.0, 1560.0, -0.3, 0.0)], 7);
        let d = Decoder::new().decode(&audio, &quick());
        let msgs: Vec<&str> = d.iter().map(|x| x.message.as_str()).collect();
        assert_eq!(msgs, vec!["K1ABC FN42 30", "G4ABC IO91 23"], "{d:?}");
        assert!((d[0].drift_hz - 2.0).abs() <= 1.0, "{:?}", d[0]);
        assert!((d[0].dt_s - 0.5).abs() < 0.2, "{:?}", d[0]);
    }

    #[test]
    fn a_signal_near_the_threshold_decodes() {
        // -28 dB in 2500 Hz: al limite tipico di WSPR (wsprd lo decodifica allo stesso modo, vedi `examples/oracolo_wspr.rs`)
        let audio = period(&[("DL1ABC JO62 27", -28.0, 1450.0, 0.1, 0.0)], 113);
        let d = Decoder::new().decode(&audio, &Options::default());
        assert_eq!(d.iter().map(|x| x.message.as_str()).collect::<Vec<_>>(), vec!["DL1ABC JO62 27"], "{d:?}");
        assert!((d[0].snr_db + 28.0).abs() <= 1.5, "{:?}", d[0]);
    }

    #[test]
    fn the_hash_table_fills_type3_calls_across_periods() {
        let mut dec = Decoder::new();
        let first = dec.decode(&period(&[("PJ4/K1ABC 37", -18.0, 1470.0, 0.0, 0.0)], 21), &quick());
        assert_eq!(first.first().map(|d| d.message.as_str()), Some("PJ4/K1ABC 37"));
        let second = dec.decode(&period(&[("<PJ4/K1ABC> FK52UD 37", -18.0, 1470.0, 0.0, 0.0)], 22), &quick());
        assert_eq!(second.first().map(|d| d.message.as_str()), Some("<PJ4/K1ABC> FK52UD 37"));
    }

    #[test]
    fn noise_alone_gives_nothing() {
        let audio = period(&[], 3);
        assert!(Decoder::new().decode(&audio, &quick()).is_empty());
    }
}
