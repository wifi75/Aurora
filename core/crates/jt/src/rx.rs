//! JT65: ricezione. Sincronismo sul periodo intero, affinamento di frequenza e ritardo, energie dei 64 toni di ogni simbolo e decodifica
//! Reed-Solomon a decisione soft con l'analisi a priori (`symspec65`, `sync65`, `demod64a`, `extract.f90`, `ftrsdap.c`, `hint65.f90` e
//! `jt65_decode.f90` di WSJT-X 3.3, GPL 3).

use crate::jt65::{NSYM, SYNC_PATTERN, data_symbols, gray, interleave};
use crate::pack::{Kind, pack_message, unpack_message};
use crate::rs::{Codec, DATA, NN};
use rustfft::FftPlanner;
use rustfft::num_complex::Complex;

const FS: f64 = 12_000.0;
const NFFT: usize = 8192;
/// Bin dello spettro grande che arrivano a 5 kHz.
const NSZ: usize = 3413;
/// Simbolo in campioni a 12 kHz: 4096 campioni a 11025 Hz.
const SYMBOL: f64 = 4096.0 * FS / 11_025.0;
/// Un quarto di simbolo: il passo con cui si cerca il ritardo.
const QSTEP: f64 = SYMBOL / 4.0;
const WINDOW: usize = 4458;
const DF_BIN: f64 = FS / NFFT as f64;
/// Distanza tra i toni in hertz.
const TONE_DF: f64 = 11_025.0 / 4096.0;
const MAX_QSYM: usize = 600;
/// Energia media minima dei toni previsti dalla parola decodificata, in unita' di rumore mediano (un segnale a -27 dB ne ha circa 4,6).
const MIN_TONE_ENERGY: f32 = 3.2;
const LAG1: i32 = -32;
const LAG2: i32 = 82;

/// Un messaggio decodificato.
#[derive(Debug, Clone, PartialEq)]
pub struct Decoded {
    pub text: String,
    /// Frequenza del tono di sincronismo (il piu' basso) in hertz.
    pub freq_hz: f32,
    /// Ritardo rispetto all'inizio nominale (un secondo dopo l'inizio del minuto), in secondi.
    pub dt_s: f32,
    /// Rapporto segnale/rumore in 2500 Hz, in dB.
    pub snr_db: i32,
    /// Forza del sincronismo (deviazioni standard).
    pub sync: f32,
    /// Simboli di cui il decodificatore ha dovuto cambiare il valore (distanza dura).
    pub hard_errors: u32,
    /// Distanza totale (dura piu' soffice) dal messaggio ricevuto e `pp2 / pp1`: gli stessi due numeri con cui `extract` accetta o scarta.
    pub total_distance: i32,
    pub rtt: f32,
    /// Tipo a priori con cui e' uscito (`ntype` di `extract.f90:162-174`: 1 CQ, 2 MyCall, 3 MyCall DxCall, 4 ... RRR, 5 ... 73,
    /// 6 ... DxGrid, 7 CQ DxCall DxGrid); 0 senza analisi a priori.
    pub ap_type: u8,
    /// Uscito dalla ricerca profonda sulla lista dei messaggi (`hint65`, `nft = 2`).
    pub hinted: bool,
}

/// Livello «Aggressive decoding level» di fabbrica di WSJT-X (`Configuration.cpp:3721`, `settings_->value ("Aggressive", 4)`).
pub const DEFAULT_AGGRESSIVE: u8 = 4;
/// F Tol di fabbrica di WSJT-X in JT65 (`mainwindow.cpp:11867`, `Ftol_JT65` = 50 Hz).
pub const DEFAULT_TOL_HZ: f32 = 50.0;

/// Le soglie della seconda accettazione per livello «Aggressive» (`h0`, `d0`, `r0` di `jt65_decode.f90:215-220`): distanza dura, distanza
/// totale e `pp2/pp1` massime di un messaggio uscito senza analisi a priori.
const H0: [i32; 12] = [41, 42, 43, 43, 44, 45, 46, 47, 48, 48, 49, 49];
const D0: [i32; 12] = [71, 72, 73, 74, 76, 77, 78, 80, 81, 82, 83, 83];
const R0: [f32; 12] = [0.70, 0.72, 0.74, 0.76, 0.78, 0.80, 0.82, 0.84, 0.86, 0.88, 0.90, 0.90];

/// Quante passate a priori per punto del QSO (`nappasses`, `extract.f90:65`) e di che tipo (`naptypes`, `:66-71`).
const NAPPASSES: [usize; 6] = [3, 4, 2, 3, 3, 4];
const NAPTYPES: [[usize; 4]; 6] = [[1, 2, 6, 0], [2, 3, 6, 7], [2, 3, 0, 0], [3, 4, 5, 0], [3, 4, 5, 0], [2, 3, 4, 5]];

/// I nominativi per l'analisi a priori (`mycall`, `hiscall`, `hisgrid`, `nQSOProgress` di `extract.f90`).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ApCalls {
    pub my_call: String,
    pub dx_call: String,
    /// Locatore del DX: contano i primi 4 caratteri.
    pub dx_grid: String,
    /// Punto del QSO (0 CQ, 1 locatore, 2 rapporto, 3 R+rapporto, 4 RRR, 5 73).
    pub qso_progress: u8,
}

/// Come si cerca.
#[derive(Debug, Clone)]
pub struct Options {
    pub f_low_hz: f32,
    pub f_high_hz: f32,
    /// Profondita' (`ndepth & 3`, «Deep» di fabbrica: `NDepth` = 3): 1 due passate e 100 prove di cancellazioni, 2 due passate e 1000,
    /// 3 quattro passate e 1000 (`jt65_decode.f90:256-268`).
    pub depth: u8,
    /// «Aggressive decoding level» 0..10 (`naggressive`): sceglie le soglie `h0`/`d0`/`r0`; 10 allarga anche quelle di `extract`.
    pub aggressive: u8,
    /// Al massimo quanti candidati alla prima passata (`ncand = min(ncand, 50 / ipass)`, `jt65_decode.f90:324`).
    pub max_candidates: usize,
    /// Frequenza di ricezione (`nfqso`).
    pub rx_hz: Option<f32>,
    /// Tolleranza attorno a `rx_hz` (`ntol`, F Tol).
    pub tol_hz: f32,
    /// «Single decode» (`nexp_decode & 32`): si cerca solo entro `rx_hz +- tol_hz` con soglia 1,0 e ci si ferma al primo messaggio
    /// (`jt65_decode.f90:302-306`, `:486`). Spento di fabbrica (`SingleDecode` = false).
    pub single_decode: bool,
    /// Analisi a priori (`ljt65apon`, «Enable AP»): `None` spenta.
    pub ap: Option<ApCalls>,
    /// Ricerca profonda sulla lista dei messaggi (`ndepth & 32`, «Include correlation», `hint65`): spenta di fabbrica.
    pub deep_search: bool,
    /// I nominativi noti (nominativo, locatore a 4) per la ricerca profonda (`set_calls`, al massimo 10000).
    pub hint_calls: Vec<(String, String)>,
}

impl Default for Options {
    fn default() -> Self {
        Options {
            f_low_hz: 200.0,
            f_high_hz: 4000.0,
            depth: 3,
            aggressive: DEFAULT_AGGRESSIVE,
            max_candidates: 50,
            rx_hz: None,
            tol_hz: DEFAULT_TOL_HZ,
            single_decode: false,
            ap: None,
            deep_search: false,
            hint_calls: Vec::new(),
        }
    }
}

fn percentile(values: &[f32], pct: usize) -> f32 {
    if values.is_empty() {
        return 0.0;
    }
    let mut v = values.to_vec();
    v.sort_by(|a, b| a.total_cmp(b));
    let k = ((v.len() * pct) / 100).min(v.len() - 1);
    v[k]
}

/// Gli spettri di potenza a passo di un quarto di simbolo (`symspec65`): `ss[j * NSZ + (i - 1)]`, `i` da 1.
fn symbol_spectra(samples: &[f32]) -> (Vec<f32>, usize) {
    if samples.len() <= NFFT {
        return (Vec::new(), 0);
    }
    let nq = (((samples.len() - NFFT) as f64 / QSTEP) as usize).min(MAX_QSYM);
    let mut planner = FftPlanner::<f32>::new();
    let fft = planner.plan_fft_forward(NFFT);
    let mut ss = vec![0.0f32; nq * NSZ];
    let mut buf = vec![Complex::new(0.0f32, 0.0); NFFT];
    let mut scratch = vec![Complex::new(0.0f32, 0.0); fft.get_inplace_scratch_len()];
    for j in 0..nq {
        let i0 = (j as f64 * QSTEP) as usize;
        for (i, b) in buf.iter_mut().enumerate() {
            *b = if i < WINDOW { Complex::new(1.0e-3 * samples[i0 + i], 0.0) } else { Complex::new(0.0, 0.0) };
        }
        fft.process_with_scratch(&mut buf, &mut scratch);
        for i in 1..=NSZ {
            ss[j * NSZ + i - 1] = buf[i].norm_sqr();
        }
    }
    (ss, nq)
}

/// Toglie la tendenza lineare e divide per la deviazione standard, escludendo l'intorno del picco (`slope_normalize`).
fn slope_normalize(y: &mut [f32], xpk: f64) {
    let (mut sumw, mut sumx, mut sumy, mut sumx2, mut sumxy) = (0.0f64, 0.0f64, 0.0f64, 0.0f64, 0.0f64);
    for (i, &v) in y.iter().enumerate() {
        let x = (i + 1) as f64;
        if (x - xpk).abs() > 4.0 {
            sumw += 1.0;
            sumx += x;
            sumy += f64::from(v);
            sumx2 += x * x;
            sumxy += x * f64::from(v);
        }
    }
    let delta = sumw * sumx2 - sumx * sumx;
    if sumw < 2.0 || delta.abs() < 1e-12 {
        return;
    }
    let a = (sumx2 * sumy - sumx * sumxy) / delta;
    let b = (sumw * sumxy - sumx * sumy) / delta;
    let mut sq = 0.0f64;
    for (i, v) in y.iter_mut().enumerate() {
        let x = (i + 1) as f64;
        *v = (f64::from(*v) - (a + b * x)) as f32;
        if (x - xpk).abs() > 4.0 {
            sq += f64::from(*v) * f64::from(*v);
        }
    }
    let rms = (sq / (sumw - 4.0).max(1.0)).sqrt();
    if rms > 1e-30 {
        for v in y.iter_mut() {
            *v = (f64::from(*v) / rms) as f32;
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct Candidate {
    bin: usize,
    lag: i32,
    sync: f32,
}

/// Cerca i segnali: per ogni frequenza correla l'energia del tono di sincronismo con la sequenza pseudocasuale (`sync65`). I candidati
/// escono in ordine di frequenza, come in `sync65.f90:62-97` (chi li tronca a 50 tiene i piu' bassi, `jt65_decode.f90:324`). `robust`
/// e' `nrobust` di `xcor.f90:29-40`: ogni valore diventa +1 o -1 rispetto alla mediana (contro i picchi dell'AGC a inizio periodo).
fn find_candidates(ss: &[f32], nq: usize, f_low_hz: f32, f_high_hz: f32, thresh: f32, robust: bool) -> Vec<Candidate> {
    if nq == 0 {
        return Vec::new();
    }
    let ia = ((f64::from(f_low_hz) / DF_BIN).round() as usize).max(2);
    let ib = ((f64::from(f_high_hz) / DF_BIN).round() as usize).min(NSZ - 1);
    if ia >= ib {
        return Vec::new();
    }
    // la potenza di fondo di ogni frequenza (il 40 per cento dei valori: il segnale occupa meno tempo)
    let mut reference = vec![1.0f32; NSZ + 1];
    for i in ia.saturating_sub(1)..=(ib + 1).min(NSZ) {
        let col: Vec<f32> = (0..nq).map(|j| ss[j * NSZ + i - 1]).collect();
        reference[i] = percentile(&col, 40).max(1e-30);
    }
    let pr: Vec<f32> = SYNC_PATTERN.iter().map(|&p| 2.0 * f32::from(p) - 1.0).collect();
    let nlag = (LAG2 - LAG1 + 1) as usize;
    let mut ccfred = vec![0.0f32; NSZ + 2];
    let mut peaks = vec![LAG1; NSZ + 2];
    let mut a = vec![0.0f32; nq];
    for i in ia..=ib {
        for (j, v) in a.iter_mut().enumerate() {
            *v = ss[j * NSZ + i - 1] / reference[i];
        }
        if robust {
            let xmed = percentile(&a, 50);
            for v in a.iter_mut() {
                *v = if *v >= xmed { 1.0 } else { -1.0 };
            }
        }
        let mut ccf = vec![0.0f32; nlag];
        let (mut best, mut lag_best) = (f32::MIN, LAG1);
        for lag in LAG1..=LAG2 {
            let mut x = 0.0f32;
            for (k, &p) in pr.iter().enumerate() {
                let j = 4 * k as i32 + lag;
                if j >= 0 && (j as usize) < nq {
                    x += a[j as usize] * p;
                }
            }
            let c = 2.0 * x;
            ccf[(lag - LAG1) as usize] = c;
            if c > best {
                best = c;
                lag_best = lag;
            }
        }
        slope_normalize(&mut ccf, f64::from(lag_best - LAG1) + 1.0);
        ccfred[i] = ccf[(lag_best - LAG1) as usize];
        peaks[i] = lag_best;
    }
    let xmed = percentile(&ccfred[ia..=ib], 35);
    for v in ccfred[ia..=ib].iter_mut() {
        *v -= xmed;
    }
    ccfred[ia - 1] = ccfred[ia];
    ccfred[ib + 1] = ccfred[ib];
    let mut out: Vec<Candidate> = Vec::new();
    for i in ia..=ib {
        if ccfred[i] >= thresh && ccfred[i] > ccfred[i - 1] && ccfred[i] > ccfred[i + 1] {
            out.push(Candidate { bin: i, lag: peaks[i], sync: ccfred[i] });
        }
    }
    out
}

/// Energia del tono a frequenza `freq` nella finestra di un simbolo (rotazione di fase per ricorrenza: non serve una tabella per frequenza).
fn tone_energy_at(samples: &[f32], start: i64, freq: f64) -> f32 {
    let (mut re, mut im) = (0.0f64, 0.0f64);
    let step = std::f64::consts::TAU * freq / FS;
    let (wr, wi) = (step.cos(), step.sin());
    let (mut cr, mut ci) = (1.0f64, 0.0f64);
    for n in 0..WINDOW {
        let idx = start + n as i64;
        if idx >= 0 && (idx as usize) < samples.len() {
            let x = f64::from(samples[idx as usize]);
            re += x * cr;
            im -= x * ci;
        }
        let nr = cr * wr - ci * wi;
        ci = cr * wi + ci * wr;
        cr = nr;
    }
    (re * re + im * im) as f32
}

/// La frequenza del tono di sincronismo nel simbolo `k` con una deriva lineare di `drift` hertz su tutto il messaggio.
fn symbol_freq(f: f64, drift: f64, k: usize) -> f64 {
    f + drift * (k as f64 / (NSYM - 1) as f64 - 0.5)
}

fn symbol_start(dt_samples: f64, k: usize) -> i64 {
    (dt_samples + k as f64 * SYMBOL).round() as i64
}

/// Correlazione con la sequenza di sincronismo di tutta la trasmissione (energie del tono 0 meno quelle dei simboli di dati).
fn sync_metric(samples: &[f32], dt_samples: f64, f: f64, drift: f64) -> f32 {
    let mut m = 0.0f32;
    for (k, &p) in SYNC_PATTERN.iter().enumerate() {
        let e = tone_energy_at(samples, symbol_start(dt_samples, k), symbol_freq(f, drift, k));
        m += if p == 1 { e } else { -e };
    }
    m
}

/// Affina frequenza, ritardo e deriva attorno al candidato (`afc65b`): prima il ritardo, poi la frequenza, poi la deriva lineare (da -3 a +3 Hz
/// su tutto il messaggio), poi ancora il ritardo piu' fine e una frequenza piu' fine. Ritorna la frequenza a meta' messaggio, il ritardo e la deriva.
fn refine(samples: &[f32], c: &Candidate) -> (f64, f64, f64) {
    let mut f = c.bin as f64 * DF_BIN;
    let mut dt = f64::from(c.lag) * QSTEP;
    let mut drift = 0.0f64;
    let mut best = sync_metric(samples, dt, f, drift);
    for step in [QSTEP / 4.0, QSTEP / 16.0] {
        let center = dt;
        for d in -4i32..=4 {
            let t = center + f64::from(d) * step;
            let m = sync_metric(samples, t, f, drift);
            if m > best {
                best = m;
                dt = t;
            }
        }
        if step > QSTEP / 8.0 {
            // la frequenza si cerca dopo il primo passo del ritardo, poi la deriva. Prima a passi di 2 Hz fino a +-8 Hz, poi di 1 Hz attorno
            // al migliore, come `afc65b.f90:16-39` (`a(1)` da -8 a +8): un candidato su un lobo laterale di un segnale forte finisce cosi'
            // sul segnale vero invece di far uscire un messaggio inventato
            for (span, df) in [(4i32, 2.0f64), (2, 1.0)] {
                let center_f = f;
                for d in -span..=span {
                    let ff = center_f + f64::from(d) * df;
                    let m = sync_metric(samples, dt, ff, drift);
                    if m > best {
                        best = m;
                        f = ff;
                    }
                }
            }
            let center_f = f;
            for d in -4i32..=4 {
                let ff = center_f + f64::from(d) * 0.35;
                let m = sync_metric(samples, dt, ff, drift);
                if m > best {
                    best = m;
                    f = ff;
                }
            }
            for d in -6i32..=6 {
                let dr = f64::from(d) * 0.5;
                let m = sync_metric(samples, dt, f, dr);
                if m > best {
                    best = m;
                    drift = dr;
                }
            }
        }
    }
    // ultimo ritocco: frequenza e deriva piu' fini
    let (f0, d0) = (f, drift);
    for df in -2i32..=2 {
        for dd in -2i32..=2 {
            let (ff, dr) = (f0 + f64::from(df) * 0.15, d0 + f64::from(dd) * 0.25);
            let m = sync_metric(samples, dt, ff, dr);
            if m > best {
                best = m;
                f = ff;
                drift = dr;
            }
        }
    }
    (f, dt, drift)
}

/// Le energie dei 64 toni di dati per i 63 simboli di dati: `s3[j][i]`, `j` nell'ordine di trasmissione.
fn tone_matrix(samples: &[f32], f: f64, dt_samples: f64, drift: f64) -> Vec<[f32; 64]> {
    let mut planner = FftPlanner::<f32>::new();
    let fft = planner.plan_fft_forward(WINDOW);
    let mut scratch = vec![Complex::new(0.0f32, 0.0); fft.get_inplace_scratch_len()];
    let mut s3 = Vec::with_capacity(NN);
    let mut buf = vec![Complex::new(0.0f32, 0.0); WINDOW];
    for (k, &p) in SYNC_PATTERN.iter().enumerate() {
        if p == 1 {
            continue;
        }
        let start = symbol_start(dt_samples, k);
        // si porta il tono di sincronismo di questo simbolo a frequenza zero (con la deriva ogni simbolo ha la sua)
        let step = -std::f64::consts::TAU * symbol_freq(f, drift, k) / FS;
        let (wr, wi) = (step.cos(), step.sin());
        let (mut cr, mut ci) = (1.0f64, 0.0f64);
        for (n, b) in buf.iter_mut().enumerate() {
            let idx = start + n as i64;
            let x = if idx >= 0 && (idx as usize) < samples.len() { f64::from(samples[idx as usize]) } else { 0.0 };
            *b = Complex::new((x * cr) as f32, (x * ci) as f32);
            let nr = cr * wr - ci * wi;
            ci = cr * wi + ci * wr;
            cr = nr;
        }
        fft.process_with_scratch(&mut buf, &mut scratch);
        let mut row = [0.0f32; 64];
        for (i, r) in row.iter_mut().enumerate() {
            // il tono di dati 0 sta due intervalli sopra il sincronismo
            *r = buf[i + 2].norm_sqr();
        }
        s3.push(row);
    }
    s3
}

struct Demod {
    mrsym: [u8; NN],
    mrprob: [i32; NN],
    mr2sym: [u8; NN],
    mr2prob: [i32; NN],
}

/// `demod64a`: per ogni simbolo i due toni piu' forti e la loro quota dell'energia (0..255).
fn demod(s3: &[[f32; 64]]) -> Demod {
    let mut d = Demod { mrsym: [0; NN], mrprob: [0; NN], mr2sym: [0; NN], mr2prob: [0; NN] };
    for (j, row) in s3.iter().enumerate().take(NN) {
        let mut s1 = f32::MIN;
        let mut i1 = 0usize;
        let mut psum = 0.0f32;
        for (i, &v) in row.iter().enumerate() {
            psum += v;
            if v > s1 {
                s1 = v;
                i1 = i;
            }
        }
        if psum == 0.0 {
            psum = 1e-6;
        }
        let mut s2 = f32::MIN;
        let mut i2 = 0usize;
        for (i, &v) in row.iter().enumerate() {
            if i != i1 && v > s2 {
                s2 = v;
                i2 = i;
            }
        }
        d.mrsym[j] = i1 as u8;
        d.mr2sym[j] = i2 as u8;
        d.mrprob[j] = (255.999 * f64::from(s1 / psum)) as i32;
        d.mr2prob[j] = (255.999 * f64::from(s2 / psum)) as i32;
    }
    d
}

/// Dalla parola di Karn (63 simboli, ultimo trasmesso per primo) ai toni di canale trasmessi.
fn channel_tones(word: &[u8; NN]) -> [u8; NN] {
    let mut a = [0u8; NN];
    for i in 0..NN {
        a[i] = word[NN - 1 - i];
    }
    let mut a = interleave(&a, 1);
    for v in a.iter_mut() {
        *v = gray(*v, 1);
    }
    a
}

/// Il risultato di un tentativo di decodifica a decisione soft.
struct SoftResult {
    word: [u8; NN],
    nhard: i32,
    ntotal: i32,
    /// `pp2/pp1`: quanto il secondo candidato e' vicino al primo (piu' basso, piu' sicuro).
    rtt: f32,
}

const PERR: [[i32; 8]; 8] = [
    [4, 9, 11, 13, 14, 14, 15, 15],
    [2, 20, 20, 30, 40, 50, 50, 50],
    [7, 24, 27, 40, 50, 50, 50, 50],
    [13, 25, 35, 46, 52, 70, 50, 50],
    [17, 30, 42, 54, 55, 64, 71, 70],
    [25, 39, 48, 57, 64, 66, 77, 77],
    [32, 45, 54, 63, 66, 75, 78, 83],
    [51, 58, 57, 66, 72, 77, 82, 86],
];

/// `ftrsdap`: prima la decodifica a decisione dura, poi `trials` prove con cancellazioni scelte a caso fra i simboli meno sicuri; fra le
/// parole candidate vince quella la cui energia, nei toni che la parola prevede, e' maggiore (`pp`). `ap` sono i 12 simboli di dati noti a
/// priori (-1 ignoto): si impongono nella parola ricevuta e non si cancellano mai (`ftrsdap.c:62-68`, `:131-133`).
fn soft_decode(codec: &Codec, d: &Demod, s3_norm: &[[f32; 64]], trials: u32, ap: &[i16; DATA]) -> Option<SoftResult> {
    let mut rxdat = [0u8; NN];
    let mut rxprob = [0i32; NN];
    let mut rxdat2 = [0u8; NN];
    let mut rxprob2 = [0i32; NN];
    // dagli indici in ordine di trasmissione a quelli di Karn
    let (mut de_dat, mut de_prob, mut de_dat2, mut de_prob2) = ([0u8; NN], [0i32; NN], [0u8; NN], [0i32; NN]);
    for i in 0..NN {
        de_dat[i] = gray(d.mrsym[i], -1);
        de_dat2[i] = gray(d.mr2sym[i], -1);
    }
    let de_dat = interleave(&de_dat, -1);
    let de_dat2 = interleave(&de_dat2, -1);
    // le probabilita' si disinterlacciano come i simboli
    let mut tmp = [0u8; NN];
    for (t, p) in tmp.iter_mut().zip(d.mrprob.iter()) {
        *t = (*p).clamp(0, 255) as u8;
    }
    let tmp = interleave(&tmp, -1);
    let mut tmp2 = [0u8; NN];
    for (t, p) in tmp2.iter_mut().zip(d.mr2prob.iter()) {
        *t = (*p).clamp(0, 255) as u8;
    }
    let tmp2 = interleave(&tmp2, -1);
    for i in 0..NN {
        de_prob[i] = i32::from(tmp[i]);
        de_prob2[i] = i32::from(tmp2[i]);
    }
    for i in 0..NN {
        rxdat[i] = de_dat[NN - 1 - i];
        rxprob[i] = de_prob[NN - 1 - i];
        rxdat2[i] = de_dat2[NN - 1 - i];
        rxprob2[i] = de_prob2[NN - 1 - i];
    }
    // i simboli noti a priori: i 12 di dati stanno in testa alla parola di Karn, in ordine inverso
    for (i, &a) in ap.iter().enumerate() {
        if (0..64).contains(&a) {
            rxdat[DATA - 1 - i] = a as u8;
            rxprob2[DATA - 1 - i] = -1;
        }
    }

    // simboli dal meno sicuro: ordine per probabilita' decrescente, poi si legge dal fondo (ordinamento a bolle stabile come in C)
    let mut indexes: Vec<usize> = (0..NN).collect();
    let mut probs: Vec<i32> = rxprob.to_vec();
    for pass in 1..NN {
        for k in 0..NN - pass {
            if probs[k] < probs[k + 1] {
                probs.swap(k, k + 1);
                indexes.swap(k, k + 1);
            }
        }
    }

    // decisione dura
    let mut work = rxdat;
    if codec.decode_karn(&mut work, &[]).is_some() {
        // `param(5)` e `param(4)` restano a zero (`ftrsdap.c:94-104`): la decisione dura passa sempre le soglie
        let nhard = work.iter().zip(rxdat.iter()).filter(|(a, b)| a != b).count() as i32;
        return Some(SoftResult { word: work, nhard, ntotal: 0, rtt: 0.0 });
    }

    let mut nsum = 0i32;
    let mut thresh0 = [0i32; NN];
    for i in 0..NN {
        nsum += rxprob[i];
        let j = indexes[62 - i];
        if rxprob2[j] >= 0 {
            let ratio = rxprob2[j] as f32 / (rxprob[j] as f32 + 0.01);
            let ii = ((7.999 * ratio) as usize).min(7);
            let jj = (62 - i) / 8;
            thresh0[i] = (1.3 * PERR[ii][jj] as f32) as i32;
        }
    }
    if nsum <= 0 {
        return None;
    }

    let mut nseed: u32 = 1;
    let (mut pp1, mut pp2) = (0.0f32, 0.0f32);
    let mut best: Option<([u8; NN], i32, i32)> = None;
    for _ in 1..=trials {
        let mut work = rxdat;
        let mut era = [0usize; 51];
        let mut numera = 0usize;
        for i in 0..NN {
            let j = indexes[62 - i];
            let thresh = thresh0[i];
            nseed = nseed.wrapping_mul(1_103_515_245).wrapping_add(12_345);
            let ir = ((nseed / 65_536) % 32_768) as i64;
            let ir = (100 * ir) / 32_768;
            if ir < i64::from(thresh) && numera < 51 {
                era[numera] = j;
                numera += 1;
            }
        }
        if codec.decode_karn(&mut work, &era[..numera]).is_some() {
            let mut nhard = 0i32;
            let mut nsoft = 0i32;
            for i in 0..NN {
                if work[i] != rxdat[i] {
                    nhard += 1;
                    if work[i] != rxdat2[i] {
                        nsoft += rxprob[i];
                    }
                }
            }
            nsoft = 63 * nsoft / nsum;
            let ntotal = nsoft + nhard;
            // pp: energia media dei toni previsti dalla parola
            let tones = channel_tones(&work);
            let mut psum = 0.0f32;
            for (j, &t) in tones.iter().enumerate() {
                psum += s3_norm[j][t as usize];
            }
            let pp = psum / 63.0;
            if pp > pp1 {
                pp2 = pp1;
                pp1 = pp;
                best = Some((work, nhard, ntotal));
            } else if pp > pp2 && pp != pp1 {
                pp2 = pp;
            }
            if let Some((_, h, t)) = best
                && h <= 41
                && t <= 71
            {
                break;
            }
        }
    }
    let (word, nhard, ntotal) = best?;
    // `param(4) = 1000 * pp2 / pp1` e' un intero: `rtt = 0.001 * param(4)` (`ftrsdap.c:180`, `extract.f90:157`)
    let rtt = if pp1 > 0.0 { (1000.0 * pp2 / pp1).trunc() / 1000.0 } else { 1.0 };
    Some(SoftResult { word, nhard, ntotal, rtt })
}

/// `chkhist`: il valore di simbolo piu' frequente e quante volte compare (`extract.f90:117`).
fn most_frequent_symbol(mrsym: &[u8; NN]) -> (usize, usize) {
    let mut hist = [0usize; 64];
    for &s in mrsym {
        hist[usize::from(s) & 63] += 1;
    }
    let (mut nmax, mut ipk) = (0usize, 0usize);
    for (i, &h) in hist.iter().enumerate() {
        if h > nmax {
            nmax = h;
            ipk = i;
        }
    }
    (nmax, ipk)
}

/// I simboli di dati noti per ognuno dei sette tipi a priori (`apsymbols`, `extract.f90:79-118`); -1 dove ignoto.
#[derive(Debug, Clone)]
struct ApTable {
    symbols: [[i16; DATA]; 8],
    progress: usize,
    enabled: bool,
}

/// Nominativo come lo vede `extract`: `mycall_12(1:6)`, i primi sei caratteri.
fn six(call: &str) -> String {
    call.trim().to_uppercase().chars().take(6).collect()
}

fn ap_symbols_of(msg: &str) -> [i16; DATA] {
    let (dat, kind) = pack_message(msg);
    // solo i messaggi standard (`itype = 1`), altrimenti nessun simbolo noto
    if kind != Kind::Standard {
        return [-1; DATA];
    }
    dat.map(i16::from)
}

impl ApTable {
    fn new(ap: Option<&ApCalls>) -> ApTable {
        let mut symbols = [[-1i16; DATA]; 8];
        let Some(ap) = ap else { return ApTable { symbols, progress: 0, enabled: false } };
        let mycall = six(&ap.my_call);
        let hiscall = six(&ap.dx_call);
        let grid: String = ap.dx_grid.trim().to_uppercase().chars().take(4).collect();
        symbols[1][..4].copy_from_slice(&[62, 32, 32, 49]);
        if !mycall.is_empty() {
            symbols[2][..4].copy_from_slice(&ap_symbols_of(&format!("{mycall} {mycall} RRR"))[..4]);
            if !hiscall.is_empty() {
                let rrr = ap_symbols_of(&format!("{mycall} {hiscall} RRR"));
                symbols[3][..9].copy_from_slice(&rrr[..9]);
                symbols[4] = rrr;
                symbols[5] = ap_symbols_of(&format!("{mycall} {hiscall} 73"));
                if !grid.is_empty() {
                    symbols[6] = ap_symbols_of(&format!("{mycall} {hiscall} {grid}"));
                    symbols[7] = ap_symbols_of(&format!("CQ {hiscall} {grid}"));
                }
            }
        }
        // `npass = 1 + nappasses(nQSOProgress)` solo con il proprio nominativo (`extract.f90:142-145`)
        ApTable { symbols, progress: usize::from(ap.qso_progress).min(5), enabled: !mycall.is_empty() }
    }
}

/// Un messaggio della lista della ricerca profonda e i suoi 63 simboli di canale (`sym2` di `hint65.f90`).
struct Hint {
    text: String,
    symbols: [u8; NN],
}

/// La lista dei messaggi di `hint65.f90:37-66`: il testo di prova, poi con il DX il suo locatore, il suo CQ e i 63 rapporti, poi per ogni
/// nominativo noto la chiamata e il CQ.
fn hint_messages(codec: &Codec, ap: &ApCalls, calls: &[(String, String)]) -> Vec<Hint> {
    const RPT: [&str; 63] = [
        "-01", "-02", "-03", "-04", "-05", "-06", "-07", "-08", "-09", "-10", "-11", "-12", "-13", "-14", "-15", "-16", "-17", "-18", "-19",
        "-20", "-21", "-22", "-23", "-24", "-25", "-26", "-27", "-28", "-29", "-30", "R-01", "R-02", "R-03", "R-04", "R-05", "R-06", "R-07",
        "R-08", "R-09", "R-10", "R-11", "R-12", "R-13", "R-14", "R-15", "R-16", "R-17", "R-18", "R-19", "R-20", "R-21", "R-22", "R-23",
        "R-24", "R-25", "R-26", "R-27", "R-28", "R-29", "R-30", "RO", "RRR", "73",
    ];
    let mycall = six(&ap.my_call);
    let hiscall = six(&ap.dx_call);
    let hisgrid: String = ap.dx_grid.trim().to_uppercase().chars().take(4).collect();
    let mut texts: Vec<String> = vec!["0123456789ABC".to_string()];
    if !hiscall.is_empty() || !hisgrid.is_empty() {
        texts.push(format!("{mycall} {hiscall} {hisgrid}"));
        texts.push(format!("CQ {hiscall} {hisgrid}"));
        texts.extend(RPT.iter().map(|r| format!("{mycall} {hiscall} {r}")));
    }
    for (call, grid) in calls.iter().take(10_000) {
        let call = six(call);
        let grid: String = grid.trim().to_uppercase().chars().take(4).collect();
        texts.push(format!("{mycall} {call} {grid}"));
        texts.push(format!("CQ {call} {grid}"));
    }
    texts
        .into_iter()
        .map(|t| {
            let (symbols, _) = data_symbols(codec, &t);
            let (dat, _) = pack_message(&t);
            Hint { text: unpack_message(&dat), symbols }
        })
        .collect()
}

/// `hint65`: un banco di filtri adattati sulle energie dei simboli, uno per messaggio della lista. Ritorna il migliore e `qual`, quanto
/// supera il secondo (`hint65.f90:70-113`, con `nadd = 0` e `nflip = 1` di JT65A in HF).
fn hint_search<'a>(s3: &[[f32; 64]], mrs: &[u8; NN], mrs2: &[u8; NN], hints: &'a [Hint]) -> Option<(&'a Hint, f32)> {
    let ref0: f32 = (0..NN).map(|j| s3[j][usize::from(mrs[j])]).sum();
    let (mut u1, mut u2) = (-99.0f32, -99.0f32);
    let mut ipk: Option<usize> = None;
    let mut msg00 = "";
    for (k, h) in hints.iter().enumerate() {
        let mut psum = 0.0f32;
        let mut reference = ref0;
        for j in 0..NN {
            let i = usize::from(h.symbols[j]);
            psum += s3[j][i];
            if h.symbols[j] == mrs[j] {
                reference = reference - s3[j][i] + s3[j][usize::from(mrs2[j])];
            }
        }
        if reference <= 0.0 {
            continue;
        }
        let p = psum / reference;
        if p > u1 {
            if h.text != msg00 {
                u2 = u1;
            }
            u1 = p;
            ipk = Some(k);
            msg00 = &h.text;
        }
        if h.text != msg00 && p > u2 {
            u2 = p;
        }
    }
    let bias = (1.12 * u2).max(0.35);
    let qual = 100.0 * (u1 - bias);
    ipk.map(|k| (&hints[k], qual))
}

/// Il risultato di `extract` per un candidato.
struct Extracted {
    text: String,
    /// I 63 simboli di dati trasmessi (ordine di canale, 0..63).
    channel: [u8; NN],
    nhard: i32,
    ntotal: i32,
    rtt: f32,
    ap_type: u8,
    hinted: bool,
    /// Energia media dei toni previsti dalla parola, in unita' di rumore mediano (`pp1`).
    pp: f32,
}

/// `extract.f90`: normalizza le energie, toglie i toni fissi (birdie), prova la decisione soft senza e poi con i tipi a priori del punto del
/// QSO; senza esito, e con la ricerca profonda accesa, prova la lista dei messaggi.
fn extract(codec: &Codec, s3: &[[f32; 64]], options: &Options, ap: &ApTable, hints: &[Hint], trials: u32) -> Option<Extracted> {
    let mut all: Vec<f32> = Vec::with_capacity(64 * NN);
    for row in s3 {
        all.extend_from_slice(row);
    }
    let base = percentile(&all, 50);
    if base <= 0.0 {
        return None;
    }
    // `s3a`: le energie normalizzate per `getpp`; `s3w` quelle da cui si tolgono i toni fissi
    let s3a: Vec<[f32; 64]> = s3.iter().map(|row| row.map(|v| v / base)).collect();
    let mut s3w = s3a.clone();
    let mut nfail = 0;
    let d = loop {
        let d = demod(&s3w);
        // `chkhist`: un tono che vince in 20 simboli o piu' e' un disturbo fisso; si porta al valore mediano e si ricomincia (`extract.f90:117-128`)
        let (nhist, ipk) = most_frequent_symbol(&d.mrsym);
        if nhist < 20 {
            break d;
        }
        nfail += 1;
        if nfail > 30 {
            return None;
        }
        let mut flat: Vec<f32> = Vec::with_capacity(64 * NN);
        for row in &s3w {
            flat.extend_from_slice(row);
        }
        let b = percentile(&flat, 50);
        for row in s3w.iter_mut() {
            row[ipk] = b;
        }
    };
    let (nd0, r0) = if options.aggressive == 10 { (83, 0.90) } else { (81, 0.87) };
    let npass = if ap.enabled { 1 + NAPPASSES[ap.progress] } else { 1 };
    for ipass in 1..=npass {
        let (ntype, known) = if ipass == 1 {
            (0usize, [-1i16; DATA])
        } else {
            let t = NAPTYPES[ap.progress][ipass - 2];
            if t == 0 || ap.symbols[t].iter().all(|&s| s < 0) {
                continue;
            }
            (t, ap.symbols[t])
        };
        let Some(res) = soft_decode(codec, &d, &s3a, trials, &known) else { continue };
        // la prima accettazione (`extract.f90:158-167`)
        if res.ntotal > nd0 || res.rtt > r0 {
            continue;
        }
        let mut dat = [0u8; DATA];
        for (i, v) in dat.iter_mut().enumerate() {
            *v = res.word[DATA - 1 - i];
        }
        let text = unpack_message(&dat);
        // `decode65b.f90:25-27`: messaggi «birdie» e altra spazzatura
        if text.is_empty() || text.starts_with("000AAA ") || text.starts_with("0L6MWK ") {
            return None;
        }
        // un messaggio di testo libero e' il tipo che il decodificatore a cancellazioni casuali inventa piu' spesso (nessun nominativo da
        // controllare): si accetta solo se e' uscito dalla decisione dura (al massimo 25 simboli sbagliati). JTDX lo mostra con un
        // marcatore (`.` o `,`) e un punto interrogativo se non supera il controllo dei nominativi; qui non si mostra.
        if dat[9] & 8 != 0 && res.nhard > 25 {
            return None;
        }
        let channel = channel_tones(&res.word);
        let pp = channel.iter().enumerate().map(|(j, &t)| s3a[j][usize::from(t)]).sum::<f32>() / NN as f32;
        return Some(Extracted { text, channel, nhard: res.nhard, ntotal: res.ntotal, rtt: res.rtt, ap_type: ntype as u8, hinted: false, pp });
    }
    if options.deep_search && !hints.is_empty() {
        let qmin = 2.0 - 0.1 * f32::from(options.aggressive.min(10));
        let (hint, qual) = hint_search(&s3w, &d.mrsym, &d.mr2sym, hints)?;
        if qual >= qmin {
            let pp = hint.symbols.iter().enumerate().map(|(j, &t)| s3a[j][usize::from(t)]).sum::<f32>() / NN as f32;
            return Some(Extracted { text: hint.text.clone(), channel: hint.symbols, nhard: 0, ntotal: 0, rtt: 0.0, ap_type: 0, hinted: true, pp });
        }
    }
    None
}

/// La seconda accettazione di `jt65_decode.f90:441-448`, solo per i messaggi usciti senza analisi a priori e senza lista (`nft = 1`):
/// distanza dura, distanza totale e `pp2/pp1` entro le soglie del livello «Aggressive».
fn passes_aggressive_level(nhard: i32, ntotal: i32, rtt: f32, aggressive: u8) -> bool {
    let n = usize::from(aggressive).min(11);
    nhard <= 50 && nhard <= H0[n] && ntotal <= D0[n] && rtt <= R0[n]
}

/// Stima del rapporto segnale/rumore in 2500 Hz dalle energie dei toni trasmessi e dal rumore degli altri.
fn estimate_snr(s3: &[[f32; 64]], tones: &[u8; NN]) -> i32 {
    let mut all: Vec<f32> = Vec::with_capacity(64 * NN);
    for row in s3 {
        all.extend_from_slice(row);
    }
    // il rumore e' il valore mediano (il 50 per cento): esponenziale, media = mediana / ln 2
    let noise = (percentile(&all, 50) / std::f32::consts::LN_2).max(1e-30);
    let sig: f32 = tones.iter().enumerate().map(|(j, &t)| s3[j][t as usize]).sum::<f32>() / NN as f32;
    let ratio = ((sig - noise) / noise).max(1e-3);
    // il rumore e' misurato in un tono di 2,69 Hz: in 2500 Hz sono 29,7 dB in piu'
    (10.0 * ratio.log10() - 29.7).round() as i32
}

/// Il tono (0 = sincronismo, da 2 a 65 = dati) di ognuno dei 126 simboli, dalla parola di canale.
fn all_tones(channel: &[u8; NN]) -> [u8; NSYM] {
    let mut tones = [0u8; NSYM];
    let mut k = 0usize;
    for (t, &p) in tones.iter_mut().zip(SYNC_PATTERN.iter()) {
        if p == 0 {
            *t = channel[k] + 2;
            k += 1;
        }
    }
    tones
}

/// L'ampiezza e la fase (come coseno e seno) del tono `freq` nella finestra di un simbolo; `count` sono i campioni dentro l'audio.
fn demodulate(audio: &[f32], start: i64, freq: f64) -> (f64, f64, usize) {
    let (mut re, mut im) = (0.0f64, 0.0f64);
    let mut count = 0usize;
    let step = std::f64::consts::TAU * freq / FS;
    let (wr, wi) = (step.cos(), step.sin());
    let (mut cr, mut ci) = (1.0f64, 0.0f64);
    for n in 0..WINDOW {
        let idx = start + n as i64;
        if idx >= 0 && (idx as usize) < audio.len() {
            let x = f64::from(audio[idx as usize]);
            re += x * cr;
            im -= x * ci;
            count += 1;
        }
        let nr = cr * wr - ci * wi;
        ci = cr * wi + ci * wr;
        cr = nr;
    }
    (re, im, count)
}

/// Quanta energia il segnale ricostruito (frequenza `f`, ritardo `dt`) trova nell'audio: serve a rifinire f e dt sul messaggio gia' noto.
fn tone_fit(audio: &[f32], tones: &[u8; NSYM], f: f64, dt_samples: f64, drift: f64) -> f64 {
    let mut total = 0.0f64;
    for (k, &tone) in tones.iter().enumerate() {
        let (re, im, count) = demodulate(audio, symbol_start(dt_samples, k), symbol_freq(f, drift, k) + f64::from(tone) * TONE_DF);
        if count >= WINDOW / 2 {
            total += re * re + im * im;
        }
    }
    total
}

/// `subtract65`: toglie dall'audio un segnale decodificato. Prima si rifinisce frequenza e ritardo sul messaggio noto (un errore di un decimo di
/// hertz lascerebbe un residuo di -18 dB), poi per ogni simbolo si stima ampiezza e fase del suo tono e si sottrae l'onda ricostruita; senza questo
/// un segnale forte lascia ai vicini tanta energia da far «decodificare» messaggi inesistenti.
fn subtract(audio: &mut [f32], channel: &[u8; NN], f0: f64, dt0: f64, drift: f64) {
    let tones = all_tones(channel);
    let (mut f, mut dt) = (f0, dt0);
    let mut best = tone_fit(audio, &tones, f, dt, drift);
    for d in -6i32..=6 {
        let ff = f0 + f64::from(d) * 0.04;
        let fit = tone_fit(audio, &tones, ff, dt, drift);
        if fit > best {
            best = fit;
            f = ff;
        }
    }
    let dt_center = dt0;
    for d in -6i32..=6 {
        let t = dt_center + f64::from(d) * 8.0;
        let fit = tone_fit(audio, &tones, f, t, drift);
        if fit > best {
            best = fit;
            dt = t;
        }
    }
    for (k, &tone) in tones.iter().enumerate() {
        let freq = symbol_freq(f, drift, k) + f64::from(tone) * TONE_DF;
        let start = symbol_start(dt, k);
        let (re, im, count) = demodulate(audio, start, freq);
        if count < WINDOW / 2 {
            continue;
        }
        // x ~ a cos(wt + phi): le somme valgono (a N / 2) (cos phi, sin phi)
        let scale = 2.0 / count as f64;
        let (a_cos, a_sin) = (re * scale, im * scale);
        let step = std::f64::consts::TAU * freq / FS;
        let (wr, wi) = (step.cos(), step.sin());
        let (mut cr, mut ci) = (1.0f64, 0.0f64);
        for n in 0..WINDOW {
            let idx = start + n as i64;
            if idx >= 0 && (idx as usize) < audio.len() {
                // a cos(wt + phi) = a_cos cos(wt) - a_sin sin(wt)
                audio[idx as usize] -= (a_cos * cr - a_sin * ci) as f32;
            }
            let nr = cr * wr - ci * wi;
            ci = cr * wi + ci * wr;
            cr = nr;
        }
    }
}

/// Decodifica un periodo di un minuto (audio a 12 kHz dall'inizio del minuto), con le passate di `jt65_decode.f90:256-291`: due (quattro a
/// profondita' 3), soglia di sincronismo 2,5 alla prima e 2,0 dopo, l'ultima delle quattro senza sottrazione e con il sincronismo robusto;
/// dalla seconda in poi ci si ferma alla prima passata che non trova niente di nuovo (`:489`). Ogni messaggio trovato si toglie dall'audio
/// prima di provare il candidato successivo (`subtract65`).
pub fn decode(samples: &[f32], options: &Options) -> Vec<Decoded> {
    let codec = Codec::new();
    let mut audio = samples.to_vec();
    let mut out: Vec<Decoded> = Vec::new();
    let (npass, trials) = match options.depth {
        0 | 1 => (2usize, 100u32),
        2 => (2, 1000),
        _ => (4, 1000),
    };
    let ap = ApTable::new(options.ap.as_ref());
    let hints = match &options.ap {
        Some(calls) if options.deep_search && !six(&calls.my_call).is_empty() => hint_messages(&codec, calls, &options.hint_calls),
        _ => Vec::new(),
    };
    let mut found_before = 0usize;
    for ipass in 1..=npass {
        let (mut thresh, subtract, robust) = match ipass {
            1 => (2.5f32, true, false),
            2 | 3 => (2.0, true, false),
            _ => (2.0, false, true),
        };
        let (mut fa, mut fb) = (options.f_low_hz, options.f_high_hz);
        // «Single decode»: solo attorno alla frequenza di ricezione, con soglia 1,0 (`jt65_decode.f90:302-306`)
        if options.single_decode
            && let Some(rx) = options.rx_hz
        {
            fa = (rx - options.tol_hz).max(200.0);
            fb = (rx + options.tol_hz).min(4000.0);
            thresh = 1.0;
        }
        let (ss, nq) = symbol_spectra(&audio);
        if nq == 0 {
            break;
        }
        let mut candidates = find_candidates(&ss, nq, fa, fb, thresh, robust);
        drop(ss);
        candidates.truncate(options.max_candidates / ipass);
        let pass = Pass { ipass, subtract, trials };
        if decode_candidates(&codec, &mut audio, &candidates, options, &ap, &hints, &pass, &mut out) {
            break;
        }
        if ipass > 1 && out.len() == found_before {
            break;
        }
        found_before = out.len();
    }
    out
}

/// I parametri di una passata.
struct Pass {
    ipass: usize,
    subtract: bool,
    trials: u32,
}

/// Prova i candidati di una passata e toglie dall'audio ogni messaggio trovato. Ritorna `true` quando «Single decode» ha il suo messaggio.
#[allow(clippy::too_many_arguments)]
fn decode_candidates(
    codec: &Codec,
    audio: &mut [f32],
    candidates: &[Candidate],
    options: &Options,
    ap: &ApTable,
    hints: &[Hint],
    pass: &Pass,
    out: &mut Vec<Decoded>,
) -> bool {
    // `decoded0`, `freq0`: l'ultimo messaggio della passata, per non ripeterlo da un candidato vicino (`jt65_decode.f90:451-452`)
    let mut last: Option<(String, f64)> = None;
    for c in candidates {
        let bin_hz = c.bin as f64 * DF_BIN;
        let (f, dt, drift) = refine(audio, c);
        // il filtro sulle correzioni dell'AFC (`jt65_decode.f90:367-374`): `a(1)` scarto di frequenza, `a(2)` meta' della deriva, `dtx` ritardo
        // dall'inizio dell'audio
        let (a1, a2, dtx) = (f - bin_hz, drift / 2.0, dt / FS);
        if a1.abs() > 10.0 / pass.ipass as f64 {
            continue;
        }
        let ibad = usize::from(a1.abs() > 5.0) + usize::from(a2.abs() > 2.0) + usize::from((dtx - 1.0).abs() > 2.5);
        if ibad >= 2 {
            continue;
        }
        let s3 = tone_matrix(audio, f, dt, drift);
        if s3.len() != NN {
            continue;
        }
        let Some(x) = extract(codec, &s3, options, ap, hints, pass.trials) else { continue };
        // la seconda accettazione, per livello «Aggressive», solo senza analisi a priori e senza lista (`nft < 2`)
        if x.ap_type == 0 && !x.hinted && !passes_aggressive_level(x.nhard, x.ntotal, x.rtt, options.aggressive) {
            continue;
        }
        // l'energia media dei toni che la parola prevede, rispetto al rumore: un messaggio vero ne ha molta, uno inventato dal decodificatore
        // a cancellazioni casuali ne ha poca (di sicuro vale solo la meta' dei simboli) e si scarta (controllo di Aurora, non di WSJT-X)
        if !x.hinted && x.pp < MIN_TONE_ENERGY {
            continue;
        }
        if let Some((t, f0)) = &last
            && *t == x.text
            && (bin_hz - f0).abs() < 3.0
        {
            continue;
        }
        // la sottrazione viene prima del controllo dei doppioni: un residuo gia' decodificato si toglie di nuovo (`jt65_decode.f90:455-467`)
        if pass.subtract {
            subtract(audio, &x.channel, f, dt, drift);
        }
        last = Some((x.text.clone(), bin_hz));
        if out.iter().any(|o| o.text == x.text) {
            continue;
        }
        out.push(Decoded {
            snr_db: estimate_snr(&s3, &x.channel),
            text: x.text,
            freq_hz: f as f32,
            dt_s: (dt / FS - 1.0) as f32,
            sync: c.sync,
            hard_errors: x.nhard.max(0) as u32,
            total_distance: x.ntotal,
            rtt: x.rtt,
            ap_type: x.ap_type,
            hinted: x.hinted,
        });
        if options.single_decode {
            return true;
        }
    }
    false
}

/// Ritorna le costanti di tempo per chi deve sapere quanto dura un messaggio.
pub fn transmission_samples() -> usize {
    (NSYM as f64 * SYMBOL).ceil() as usize
}

/// La distanza tra i toni di JT65 sottomodo A, per chi deve disegnare la banda occupata.
pub fn tone_spacing_hz() -> f64 {
    TONE_DF
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::jt65::{modulate, tones};

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

    /// Un minuto di audio con rumore bianco (sigma 1) e un messaggio al rapporto dato (dB in 2500 Hz).
    fn minute(messages: &[(&str, f64, f64, f64)], seed: u64) -> Vec<f32> {
        let codec = Codec::new();
        let mut rng = Lcg(seed);
        let n = 60 * 12_000;
        let mut audio: Vec<f32> = (0..n).map(|_| rng.gauss() as f32).collect();
        for &(msg, f0, dt_s, snr_db) in messages {
            let snr = 10f64.powf(snr_db / 10.0);
            let amp = (2.0 * snr * 2500.0 / 6000.0).sqrt() as f32;
            let wave = modulate(&tones(&codec, msg), f0, amp, 12_000.0);
            let start = ((1.0 + dt_s) * 12_000.0) as usize;
            for (i, v) in wave.iter().enumerate() {
                if start + i < n {
                    audio[start + i] += v;
                }
            }
        }
        audio
    }

    #[test]
    fn a_clean_signal_is_found_with_its_frequency_and_delay() {
        let audio = minute(&[("CQ IU3ABC JN55", 1500.0, 0.3, -10.0)], 11);
        let out = decode(&audio, &Options::default());
        assert_eq!(out.len(), 1, "{out:?}");
        assert_eq!(out[0].text, "CQ IU3ABC JN55");
        assert!((out[0].freq_hz - 1500.0).abs() < 1.5, "{:?}", out[0]);
        assert!((out[0].dt_s - 0.3).abs() < 0.08, "{:?}", out[0]);
    }

    #[test]
    fn a_weak_signal_at_minus_twenty_one_db_still_decodes() {
        let audio = minute(&[("K1ABC IU3ABC -15", 1200.0, 0.0, -21.0)], 5);
        let out = decode(&audio, &Options::default());
        assert!(out.iter().any(|d| d.text == "K1ABC IU3ABC -15"), "{out:?}");
    }

    #[test]
    fn two_signals_are_both_decoded_and_noise_gives_nothing() {
        let audio = minute(&[("CQ IU3ABC JN55", 900.0, 0.2, -12.0), ("IU3ABC K1ABC FN42", 2100.0, -0.1, -14.0)], 9);
        let out = decode(&audio, &Options::default());
        assert!(out.iter().any(|d| d.text == "CQ IU3ABC JN55"), "{out:?}");
        assert!(out.iter().any(|d| d.text == "IU3ABC K1ABC FN42"), "{out:?}");
        for seed in [1, 2] {
            let noise = minute(&[], seed);
            assert!(decode(&noise, &Options::default()).is_empty(), "il rumore non deve dare messaggi");
        }
    }
    #[test]
    fn a_strong_signal_is_subtracted_so_it_does_not_hide_or_invent_neighbours() {
        let audio = minute(&[("CQ IU3ABC JN55", 1500.0, 0.1, 3.0), ("K1ABC IU3ABC -15", 1640.0, 0.2, -17.0)], 3);
        let out = decode(&audio, &Options::default());
        let texts: Vec<&str> = out.iter().map(|d| d.text.as_str()).collect();
        assert!(texts.contains(&"CQ IU3ABC JN55"), "{out:?}");
        assert!(texts.contains(&"K1ABC IU3ABC -15"), "{out:?}");
        assert_eq!(out.len(), 2, "nessun messaggio inventato vicino al segnale forte: {out:?}");
    }
    /// Un segnale che deriva di due hertz sul messaggio (uno spostamento da propagazione) si decodifica e la frequenza riportata e' quella di mezzo.
    #[test]
    fn a_drifting_signal_is_followed() {
        let codec = Codec::new();
        let mut rng = Lcg(5);
        let n = 60 * 12_000;
        let mut audio: Vec<f32> = (0..n).map(|_| rng.gauss() as f32).collect();
        let amp = (2.0 * 10f64.powf(-16.0 / 10.0) * 2500.0 / 6000.0).sqrt() as f32;
        let wave = crate::jt65::modulate_drifting(&tones(&codec, "K1ABC IU3ABC -15"), 1400.0, amp, 12_000.0, 3.0);
        let start = 12_000 + 1_200;
        for (i, v) in wave.iter().enumerate() {
            audio[start + i] += v;
        }
        let out = decode(&audio, &Options::default());
        assert!(out.iter().any(|d| d.text == "K1ABC IU3ABC -15"), "{out:?}");
        let d = out.iter().find(|d| d.text == "K1ABC IU3ABC -15").expect("trovato");
        // a meta' messaggio il tono di sincronismo sta a 1400 Hz (deriva simmetrica attorno a f0)
        assert!((d.freq_hz - 1400.0).abs() < 1.0, "{d:?}");
    }

    /// Le soglie `h0`/`d0`/`r0` di `jt65_decode.f90:215-220` al livello 4 di fabbrica: 44, 76, 0,78.
    #[test]
    fn the_aggressive_level_four_thresholds_are_those_of_jt65_decode() {
        assert_eq!(DEFAULT_AGGRESSIVE, 4);
        assert!(passes_aggressive_level(44, 76, 0.78, 4));
        assert!(!passes_aggressive_level(45, 76, 0.78, 4));
        assert!(!passes_aggressive_level(44, 77, 0.78, 4));
        assert!(!passes_aggressive_level(44, 76, 0.79, 4));
        // la soglia grezza di `extract` (81 e 0,87) non basta piu' da sola
        assert!(!passes_aggressive_level(30, 81, 0.87, 4));
        assert!(passes_aggressive_level(41, 71, 0.70, 0));
        assert!(!passes_aggressive_level(42, 71, 0.70, 0));
        assert!(passes_aggressive_level(49, 83, 0.90, 10));
        // oltre 50 simboli duri sbagliati si scarta a ogni livello
        assert!(!passes_aggressive_level(51, 70, 0.5, 11));
        // la decisione dura (`ntotal` e `rtt` a zero, al massimo 25 simboli) passa sempre
        assert!(passes_aggressive_level(25, 0, 0.0, 0));
    }

    /// I simboli noti dei sette tipi a priori (`extract.f90:79-118`).
    #[test]
    fn the_ap_symbols_are_those_of_extract() {
        let ap = ApCalls { my_call: "IU3ABC".into(), dx_call: "K1ABC".into(), dx_grid: "IO91".into(), qso_progress: 1 };
        let t = ApTable::new(Some(&ap));
        assert!(t.enabled);
        assert_eq!(t.progress, 1);
        let pack = |m: &str| pack_message(m).0.map(i16::from);
        assert_eq!(t.symbols[1][..4], [62, 32, 32, 49]);
        assert_eq!(t.symbols[1][..4], pack("CQ K1ABC IO91")[..4]);
        assert_eq!(t.symbols[2][..4], pack("IU3ABC IU3ABC RRR")[..4]);
        assert!(t.symbols[2][4..].iter().all(|&s| s < 0));
        assert_eq!(t.symbols[3][..9], pack("IU3ABC K1ABC RRR")[..9]);
        assert!(t.symbols[3][9..].iter().all(|&s| s < 0));
        assert_eq!(t.symbols[4], pack("IU3ABC K1ABC RRR"));
        assert_eq!(t.symbols[5], pack("IU3ABC K1ABC 73"));
        assert_eq!(t.symbols[6], pack("IU3ABC K1ABC IO91"));
        assert_eq!(t.symbols[7], pack("CQ K1ABC IO91"));
        // senza il proprio nominativo nessuna passata a priori (`extract.f90:142`)
        assert!(!ApTable::new(Some(&ApCalls { my_call: String::new(), ..ap })).enabled);
        assert!(!ApTable::new(None).enabled);
    }

    /// «Single decode»: solo la finestra di ricezione, e ci si ferma al primo messaggio (`jt65_decode.f90:302-306`, `:486`).
    #[test]
    fn single_decode_searches_only_around_the_rx_frequency() {
        let audio = minute(&[("CQ IU3ABC JN55", 900.0, 0.2, -12.0), ("IU3ABC K1ABC FN42", 2100.0, -0.1, -14.0)], 9);
        let options = Options { rx_hz: Some(2100.0), single_decode: true, ..Options::default() };
        let out = decode(&audio, &options);
        assert_eq!(out.len(), 1, "{out:?}");
        assert_eq!(out[0].text, "IU3ABC K1ABC FN42");
        // senza «Single decode» la frequenza di ricezione non restringe la ricerca (in HF `nfqso` e `ntol` contano solo per la media)
        let out = decode(&audio, &Options { rx_hz: Some(2100.0), ..Options::default() });
        assert_eq!(out.len(), 2, "{out:?}");
    }

    fn qso_calls() -> ApCalls {
        ApCalls { my_call: "IU3ABC".into(), dx_call: "K1ABC".into(), dx_grid: "IO91".into(), qso_progress: 1 }
    }

    /// Un messaggio a -27 dB che la decodifica normale non trova esce con l'analisi a priori (IU3ABC chiama K1ABC che risponde col locatore).
    #[test]
    fn a_weak_message_of_the_qso_decodes_with_a_priori_symbols() {
        let audio = minute(&[("IU3ABC K1ABC IO91", 1300.0, 0.1, -27.0), ("IU3ABC K1ABC -15", 2200.0, 0.0, -27.0)], 5);
        let plain = decode(&audio, &Options::default());
        assert!(plain.iter().all(|d| d.text != "IU3ABC K1ABC IO91"), "{plain:?}");
        let with_ap = decode(&audio, &Options { ap: Some(qso_calls()), ..Options::default() });
        let d = with_ap.iter().find(|d| d.text == "IU3ABC K1ABC IO91").expect("decodificato con l'analisi a priori");
        assert!(d.ap_type > 0 && !d.hinted, "{d:?}");
        assert!((d.freq_hz - 1300.0).abs() < 1.5, "{d:?}");
    }

    /// Con l'analisi a priori e la ricerca profonda accese il rumore resta rumore.
    #[test]
    fn noise_gives_nothing_with_a_priori_and_deep_search() {
        let options = Options { ap: Some(qso_calls()), deep_search: true, ..Options::default() };
        for seed in [1, 2] {
            let out = decode(&minute(&[], seed), &options);
            assert!(out.is_empty(), "il rumore non deve dare messaggi: {out:?}");
        }
    }
}
