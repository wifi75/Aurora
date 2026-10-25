//! Ricerca e allineamento dei segnali FT4: `getcandidates4`, `ft4_baseline`, `ft4_downsample`, `sync4d` e `get_ft4_bitmetrics`
//! (JTDX 2.2.159, `lib/ft4/*.f90`).
//!
//! # Percorso di un segnale
//!
//! 1. [`find_candidates`] media gli spettri del periodo (finestre di 2304 campioni ogni 48 ms), toglie il fondo con un polinomio di quarto
//!    grado, liscia e cerca i picchi; restituisce la frequenza del tono 0 (a meno di qualche Hz) e il punteggio di ogni picco.
//! 2. [`Downsampler`] porta il periodo a 666,67 Hz di campionamento (32 campioni per simbolo, 4096 campioni in tutto) con il tono 0 a 0 Hz.
//! 3. [`search`] cerca ritardo e correzione di frequenza (passi di 1 Hz, +-16 Hz) con le quattro matrici di Costas ([`sync4d`]): una
//!    ricerca grossa a passi di 3 Hz e 4 campioni, poi una fine a passi di 1 Hz e 1 campione.
//! 4. [`frame`] prende i 103 simboli dal segnale ricostruito alla frequenza corretta e [`bit_metrics`] ne ricava le tre serie di LLR.
//!
//! ## Convenzione del ritardo
//!
//! Qui il ritardo e' `start`: il campione (a 666,67 Hz) da cui comincia il primo simbolo di Costas, contato dall'inizio dell'audio. Per
//! averlo come lo scrive JTDX: `dt_s = start / 666,67 - 0,5`. Quindi con `dt_s = 0` il primo simbolo di Costas comincia a 0,5 s.
//!
//! ## Livello dell'audio
//!
//! La ricerca dei candidati lavora su rapporti e il resto del percorso normalizza il segnale (`cd / sqrt(potenza media)`): il livello
//! assoluto dell'audio non conta (JTDX tiene i campioni nella scala dei 16 bit; il decoder li porta li').
//!
//! # LLR
//!
//! Stessa convenzione di `ft8`: **LLR positivo = bit 1**. Ogni serie e' normalizzata dividendo per la radice del valore quadratico medio
//! dei 206 valori (compresi quelli dei simboli di Costas) e moltiplicata per 2,83; l'ordine dei bit e' quello del codeword (due bit per
//! simbolo dati, primo bit = il piu' significativo della coppia).
//!
//! # Differenze rispetto al Fortran
//!
//! - `getcandidates4`: stessa finestra (compresa la forma, che sale solo da 1 a 0,707 sui 128 campioni prima del blocco: cosi' e' nel
//!   sorgente), stessi limiti, `ft4_baseline` con il polinomio risolto con le equazioni normali (eliminazione di Gauss con pivot, variabile
//!   riscalata) al posto dei determinanti di `polyfit`: stesso polinomio a meno degli arrotondamenti. Se il fondo non si puo' stimare
//!   (spettro nullo, banda troppo stretta) l'elenco e' vuoto, come il `return` del Fortran.
//! - `sync4d`: sempre con la correzione di frequenza (`itwk = 1`), che e' l'unico uso del decoder; le tabelle (riferimenti di Costas per
//!   ogni correzione) sono calcolate una volta in [`SyncTables`].
//! - `get_ft4_bitmetrics`: identica; il controllo `nsync < 8` e il conteggio dei bit di sincronismo (`nsync_qual`) escono insieme in [`Metrics`].
//! - Non c'e' la compensazione AGC (`agccft4`, `llagcc`), ne' la perdita parziale di dati (`partintft4`): sono ritocchi dell'interfaccia di JTDX.

use std::f64::consts::PI;
use std::sync::Arc;

use rustfft::num_complex::Complex32;
use rustfft::{Fft, FftPlanner};

use crate::{COSTAS, GRAYMAP, NMAX, NN, NSPS, SAMPLE_RATE};

/// Un numero complesso a 32 bit: lo stesso tipo di rustfft e di `ft8::sync`.
pub type Complex = Complex32;

const ZERO: Complex = Complex32::new(0.0, 0.0);

/// Fattore di discesa: 12000 / 18 = 666,67 Hz.
pub const NDOWN: usize = 18;
/// Campioni per simbolo dopo la discesa.
pub const NSS: usize = NSPS / NDOWN;
/// Campioni del segnale dopo la discesa (`NMAX / NDOWN`).
pub const NDMAX: usize = NMAX / NDOWN;
/// Campioni al secondo dopo la discesa, come lo scrive JTDX nelle conversioni tra campioni e secondi (666,67, non 666,666...).
pub const DOWN_RATE: f32 = 666.67;
/// Fattore di scala degli LLR di JTDX (`scalefac`).
const LLR_SCALE: f32 = 2.83;

/// FFT dello spettrogramma dei candidati.
const NFFT1: usize = 2560;
/// Bin dello spettrogramma (da 1 a 1280, 4,6875 Hz ciascuno).
const NH1: usize = NFFT1 / 2;
/// Passo dello spettrogramma: un simbolo.
const NSTEP: usize = NSPS;
/// Colonne dello spettrogramma.
const NHSYM: usize = 123;
/// Bit di una serie di metriche: 2 per simbolo.
const NBM: usize = 2 * NN;
/// Campioni (a 666,67 Hz) di un blocco di Costas con un campione su due: 4 simboli da 16.
const NREF: usize = 2 * NSS;

/// Un segnale probabile nello spettro del periodo.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Candidate {
    /// Frequenza del tono 0 (Hz).
    pub freq_hz: f32,
    /// Altezza del picco rispetto al fondo (`speak` di `getcandidates4`): JTDX ne ricava l'SNR.
    pub sync: f32,
}

fn nint(x: f32) -> i64 {
    x.round() as i64
}

fn fft_plan(n: usize, inverse: bool) -> Arc<dyn Fft<f32>> {
    let mut planner = FftPlanner::<f32>::new();
    if inverse { planner.plan_fft_inverse(n) } else { planner.plan_fft_forward(n) }
}

// ---------------------------------------------------------------------------------------------------------------------
// getcandidates4

/// Risolve le equazioni normali di un polinomio di grado `nterms - 1` ai minimi quadrati (`polyfit`, modo 0). `None` se il sistema e' singolare.
fn polyfit(x: &[f64], y: &[f64], nterms: usize) -> Option<Vec<f64>> {
    let mut a = vec![vec![0.0f64; nterms + 1]; nterms];
    for (&xi, &yi) in x.iter().zip(y) {
        let mut powers = vec![1.0f64; 2 * nterms - 1];
        for n in 1..powers.len() {
            powers[n] = powers[n - 1] * xi;
        }
        for j in 0..nterms {
            for k in 0..nterms {
                a[j][k] += powers[j + k];
            }
            a[j][nterms] += yi * powers[j];
        }
    }
    for col in 0..nterms {
        let pivot = (col..nterms).max_by(|&p, &q| a[p][col].abs().total_cmp(&a[q][col].abs()))?;
        if a[pivot][col].abs().is_nan() || a[pivot][col].abs() <= 1e-300 {
            return None;
        }
        a.swap(col, pivot);
        for row in 0..nterms {
            if row != col {
                let f = a[row][col] / a[col][col];
                let pivot_row = a[col].clone();
                for (v, p) in a[row][col..=nterms].iter_mut().zip(&pivot_row[col..=nterms]) {
                    *v -= f * p;
                }
            }
        }
    }
    Some((0..nterms).map(|i| a[i][nterms] / a[i][i]).collect())
}

/// `ft4_baseline`: fondo dello spettro medio `savg` (potenza lineare) tra i bin `nfa` e `nfb`, come polinomio di quarto grado sui punti piu' bassi
/// di dieci segmenti, piu' 0,65 dB. Restituisce il fondo (lineare) per i bin da `max(43, nfa)` a `nfb` (gli altri valgono 0).
fn baseline(savg: &[f32], nfa: usize, nfb: usize) -> Option<Vec<f32>> {
    const NSEG: usize = 10;
    const NPCT: f32 = 10.0;
    const NTERMS: usize = 5;
    let ia = nfa.max(43); // nint(200 Hz / 4,6875)
    let ib = nfb.min(NH1);
    if ib <= ia {
        return None;
    }
    let mut s = vec![0.0f32; NH1 + 1];
    for i in ia..=ib {
        s[i] = 10.0 * savg[i].log10();
    }
    let nlen = (ib - ia + 1) / NSEG; // divisione intera, come nel Fortran
    if nlen == 0 {
        return None;
    }
    let i0 = (ib - ia).div_ceil(2) as f64; // = (ib - ia + 1) / 2 intero
    let mut xs: Vec<f64> = Vec::new();
    let mut ys: Vec<f64> = Vec::new();
    for n in 0..NSEG {
        let ja = ia + n * nlen;
        let jb = ja + nlen - 1;
        // pctile: il valore di rango nint(npts * 0,01 * npct) tra quelli ordinati
        let mut tmp: Vec<f32> = s[ja..=jb].to_vec();
        tmp.sort_by(f32::total_cmp);
        let rank = ((nlen as f32 * 0.01 * NPCT).round() as usize).clamp(1, nlen);
        let base = tmp[rank - 1];
        for (i, &si) in s.iter().enumerate().take(jb + 1).skip(ja) {
            if si <= base {
                let (x, y) = (i as f64 - i0, f64::from(si));
                if xs.len() < 1000 {
                    xs.push(x);
                    ys.push(y);
                } else if let (Some(lx), Some(ly)) = (xs.last_mut(), ys.last_mut()) {
                    // il Fortran con k = 1000 continua a riscrivere l'ultimo punto
                    *lx = x;
                    *ly = y;
                }
            }
        }
    }
    if xs.len() < NTERMS {
        return None;
    }
    // il polinomio nella variabile riscalata: stessa curva, equazioni normali meglio condizionate
    const SCALE: f64 = 500.0;
    let xr: Vec<f64> = xs.iter().map(|x| x / SCALE).collect();
    let a = polyfit(&xr, &ys, NTERMS)?;
    let mut sbase = vec![0.0f32; NH1 + 1];
    for (i, v) in sbase.iter_mut().enumerate().take(ib + 1).skip(ia) {
        let t = (i as f64 - i0) / SCALE;
        let db = a[0] + t * (a[1] + t * (a[2] + t * (a[3] + t * a[4]))) + 0.65;
        *v = 10f64.powf(db / 10.0) as f32;
    }
    Some(sbase)
}

/// Cerca i candidati tra `f_low_hz` e `f_high_hz` nell'audio del periodo (12 kHz; si usano i primi [`NMAX`] campioni, il resto si completa con zeri).
///
/// `min_sync` e' `syncmin` (1,2 in JTDX); `max` il numero massimo di candidati (JTDX 100: i primi trovati salendo di frequenza). L'elenco e' in
/// ordine di frequenza, ma quelli entro 20 Hz da `qso_freq_hz` (se c'e') vengono per primi. La frequenza e' del tono 0, a passi di circa 1 Hz.
pub fn find_candidates(dd: &[f32], f_low_hz: f32, f_high_hz: f32, min_sync: f32, max: usize, qso_freq_hz: Option<f32>) -> Vec<Candidate> {
    if max == 0 || !f_low_hz.is_finite() || !f_high_hz.is_finite() || f_high_hz <= f_low_hz || !min_sync.is_finite() {
        return Vec::new();
    }
    let df = SAMPLE_RATE as f32 / NFFT1 as f32;
    let at = |i: usize| -> f32 { if i < NMAX { dd.get(i).copied().unwrap_or(0.0) } else { 0.0 } };

    // finestra: sqrt((1 + cos(i * pi / 256)) / 2), i = 1..256
    let mut window = [0.0f32; 256];
    for (i, w) in window.iter_mut().enumerate() {
        *w = ((1.0 + ((i + 1) as f64 * PI / 256.0).cos()) / 2.0).sqrt() as f32;
    }
    let fac = 1.0f32 / 300.0;
    let plan = fft_plan(NFFT1, false);
    let mut scratch = vec![ZERO; plan.get_inplace_scratch_len()];
    let mut buf = vec![ZERO; NFFT1];
    let mut savg = vec![0.0f32; NH1 + 1];
    for j in 0..NHSYM {
        let ia = j * NSTEP;
        let ib = ia + 2303;
        if ib >= NMAX {
            break;
        }
        buf.fill(ZERO);
        for m in 0..2304 {
            buf[128 + m].re = fac * at(ia + m);
        }
        if j != 0 {
            for m in 0..128 {
                buf[m].re = fac * at(ia - 128 + m) * window[m];
            }
        }
        if j != NHSYM - 1 {
            for m in 0..128 {
                buf[2432 + m].re = fac * at(ib + 1 + m) * window[128 + m];
            }
        }
        plan.process_with_scratch(&mut buf, &mut scratch);
        for i in 1..=NH1 {
            savg[i] += buf[i].norm_sqr();
        }
    }
    for v in savg.iter_mut() {
        *v /= NHSYM as f32;
    }

    // liscia: 17 celle (a meno dei bordi: finestre piu' strette sui primi 8 bin; gli ultimi 8 restano a zero)
    let mut savsm = vec![0.0f32; NH1 + 1];
    for i in 9..=NH1 - 8 {
        savsm[i] = savg[i - 8..=i + 8].iter().sum::<f32>() / 17.0;
    }
    for i in 1..=8usize {
        let k = i - 1;
        savsm[i] = savg[i - k..=i + k].iter().sum::<f32>() / (2 * k + 1) as f32;
    }

    let nfa = nint(f_low_hz / df).max(1);
    let nfb = nint(4910.0 / df).min(nint(f_high_hz / df));
    if nfb <= nfa + 1 || nfb as usize >= NH1 {
        return Vec::new();
    }
    let (nfa, nfb) = (nfa as usize, nfb as usize);
    let nfaa = nfa.max(43);
    let Some(sbase) = baseline(&savg, nfa, nfb) else { return Vec::new() };
    if sbase[nfaa..=nfb].iter().any(|&b| b.is_nan() || b <= 0.0 || !b.is_finite()) {
        return Vec::new();
    }
    if nfa >= 43 {
        for i in nfa..=nfb {
            savsm[i] /= sbase[i];
        }
    } else {
        let b43 = sbase[43];
        for v in &mut savsm[1..=42] {
            *v /= b43;
        }
        for i in 43..=nfb {
            savsm[i] /= sbase[i];
        }
    }

    let f_offset = -1.5 * SAMPLE_RATE as f32 / NSPS as f32;
    let mut found: Vec<Candidate> = Vec::new();
    for i in nfa + 1..nfb {
        if savsm[i] >= savsm[i - 1] && savsm[i] >= savsm[i + 1] && savsm[i] >= min_sync {
            let den = savsm[i - 1] - 2.0 * savsm[i] + savsm[i + 1];
            let del = if den != 0.0 { 0.5 * (savsm[i - 1] - savsm[i + 1]) / den } else { 0.0 };
            let fpeak = (i as f32 + del) * df + f_offset;
            if fpeak > 4910.0 {
                continue;
            }
            let speak = savsm[i] - 0.25 * (savsm[i - 1] - savsm[i + 1]) * del;
            found.push(Candidate { freq_hz: fpeak, sync: speak });
            if found.len() == max {
                break;
            }
        }
    }
    match qso_freq_hz {
        Some(q) if q.is_finite() => {
            let (near, far): (Vec<Candidate>, Vec<Candidate>) = found.into_iter().partition(|c| (c.freq_hz - q).abs() <= 20.0);
            near.into_iter().chain(far).collect()
        }
        _ => found,
    }
}

// ---------------------------------------------------------------------------------------------------------------------
// ft4_downsample

/// Calcola una volta la FFT dell'intero segnale e da questa ricava, per ogni frequenza, il segnale a 666,67 Hz.
pub struct Downsampler {
    /// Bin 0..=36864 della FFT di 73728 punti.
    spec: Vec<Complex>,
    /// Filtro di 4096 punti, gia' ruotato (`cshift`): passa da -10 Hz a +73 Hz rispetto a `f0`, con fianchi coseno rialzato di 10 Hz.
    window: Vec<f32>,
    ifft: Arc<dyn Fft<f32>>,
    /// Larghezza di un bin: 12000 / 73728.
    df: f32,
}

impl Downsampler {
    /// Costruisce lo spettro dei primi [`NMAX`] campioni di `samples` (12 kHz; il resto si scarta o si completa con zeri).
    pub fn new(samples: &[f32]) -> Self {
        let plan = fft_plan(NMAX, false);
        let mut buf = vec![ZERO; NMAX];
        for (b, &x) in buf.iter_mut().zip(samples.iter().take(NMAX)) {
            b.re = x;
        }
        let mut scratch = vec![ZERO; plan.get_inplace_scratch_len()];
        plan.process_with_scratch(&mut buf, &mut scratch);
        buf.truncate(NMAX / 2 + 1);

        // come il Fortran, in singola precisione: i bin dei fianchi si troncano a intero
        let df = SAMPLE_RATE as f32 / NMAX as f32;
        let baud = SAMPLE_RATE as f32 / NSPS as f32;
        let bw_transition = 0.5 * baud;
        let bw_flat = 4.0 * baud;
        let iwt = (bw_transition / df) as usize;
        let iwf = (bw_flat / df) as usize;
        let iws = (baud / df) as usize;
        let mut window = vec![0.0f32; NDMAX];
        for (i, w) in window.iter_mut().enumerate().take(iwt) {
            *w = (0.5 * (1.0 + (PI * (iwt - 1 - i) as f64 / iwt as f64).cos())) as f32;
        }
        for w in &mut window[iwt..iwt + iwf] {
            *w = 1.0;
        }
        for i in 0..iwt {
            window[iwt + iwf + i] = (0.5 * (1.0 + (PI * i as f64 / iwt as f64).cos())) as f32;
        }
        window.rotate_left(iws % NDMAX);
        Downsampler { spec: buf, window, ifft: fft_plan(NDMAX, true), df }
    }

    /// Porta il segnale a 666,67 Hz di campionamento con `f0_hz` a 0 Hz ([`NDMAX`] campioni complessi).
    pub fn run(&self, f0_hz: f32) -> Vec<Complex> {
        let mut c1 = vec![ZERO; NDMAX];
        if !f0_hz.is_finite() {
            return c1;
        }
        let last = (NMAX / 2) as i64;
        let i0 = nint(f0_hz / self.df);
        let get = |k: i64| -> Option<Complex> { if (0..=last).contains(&k) { Some(self.spec[k as usize]) } else { None } };
        if let Some(v) = get(i0) {
            c1[0] = v;
        }
        for i in 1..=(NDMAX / 2) as i64 {
            if let Some(v) = get(i0 + i) {
                c1[i as usize] = v;
            }
            if let Some(v) = get(i0 - i) {
                c1[NDMAX - i as usize] = v;
            }
        }
        for (c, &w) in c1.iter_mut().zip(&self.window) {
            *c *= w / NDMAX as f32;
        }
        c1[0] *= 1.9;
        c1[NDMAX - 1] *= 1.9;
        let mut scratch = vec![ZERO; self.ifft.get_inplace_scratch_len()];
        self.ifft.process_with_scratch(&mut c1, &mut scratch);
        c1
    }
}

/// Divide il segnale per la radice della sua potenza media (`sum2` del decoder): `count` e' il divisore della somma dei quadrati.
pub fn normalize(cd: &mut [Complex], count: f32) {
    let sum2 = cd.iter().map(|c| c.norm_sqr()).sum::<f32>() / count;
    if sum2 > 0.0 && sum2.is_finite() {
        let g = 1.0 / sum2.sqrt();
        for c in cd.iter_mut() {
            *c *= g;
        }
    }
}

// ---------------------------------------------------------------------------------------------------------------------
// sync4d

/// Riferimenti di Costas gia' corretti in frequenza: per ogni correzione `idf` da -16 a +16 Hz e per ognuna delle quattro matrici, i 64
/// campioni (un campione su due dopo la discesa) del tono, moltiplicati per la rotazione di `idf` Hz (`ctwk2` e `csynca..d` di JTDX).
pub struct SyncTables {
    tweaked: Vec<[[Complex; NREF]; 4]>,
}

/// Massima correzione di frequenza (Hz) cercata.
pub const MAX_DF: i64 = 16;

impl SyncTables {
    pub fn new() -> Self {
        let twopi = 2.0 * PI;
        let mut base = [[ZERO; NREF]; 4];
        for (m, costas) in COSTAS.iter().enumerate() {
            let mut phi = 0.0f64;
            let mut k = 0;
            for &tone in costas {
                let dphi = 2.0 * twopi * f64::from(tone) / NSS as f64;
                for _ in 0..NSS / 2 {
                    base[m][k] = Complex::new(phi.cos() as f32, phi.sin() as f32);
                    phi = (phi + dphi) % twopi;
                    k += 1;
                }
            }
        }
        // rotazione: fase di idf Hz a ogni campione a 333,33 Hz (un campione su due), cominciando dal primo passo
        let half_rate = f64::from(SAMPLE_RATE as f32 / NDOWN as f32) / 2.0;
        let mut tweaked = Vec::new();
        for idf in -MAX_DF..=MAX_DF {
            let dphi = idf as f64 * twopi / half_rate;
            let mut set = [[ZERO; NREF]; 4];
            for k in 0..NREF {
                let phase = dphi * (k + 1) as f64;
                let w = Complex::new(phase.cos() as f32, phase.sin() as f32);
                for m in 0..4 {
                    set[m][k] = w * base[m][k];
                }
            }
            tweaked.push(set);
        }
        SyncTables { tweaked }
    }
}

impl Default for SyncTables {
    fn default() -> Self {
        Self::new()
    }
}

/// Somma `cd0[start + 2m] * conj(r[m])` per `m` in `0..count`.
fn dot(cd0: &[Complex], start: usize, r: &[Complex], count: usize) -> Complex {
    let mut acc = ZERO;
    for m in 0..count {
        acc += cd0[start + 2 * m] * r[m].conj();
    }
    acc
}

/// Potenza di sincronismo di `sync4d`: somma dei moduli (diviso `2 * NSS`) delle correlazioni con le quattro matrici di Costas, che cominciano a
/// `i0`, `i0 + 33 NSS`, `i0 + 66 NSS` e `i0 + 99 NSS` (campioni a 666,67 Hz). `df_hz` e' la correzione di frequenza, da -16 a +16.
///
/// Le matrici che cadono solo in parte nel segnale contano per la parte che c'e' (la prima e l'ultima, con almeno 17 punti), le due di mezzo
/// solo se intere, come in JTDX.
pub fn sync4d(cd0: &[Complex], i0: i64, tables: &SyncTables, df_hz: i64) -> f32 {
    let Some(set) = tables.tweaked.get((df_hz + MAX_DF) as usize) else { return 0.0 };
    let np = cd0.len() as i64;
    let span = 4 * NSS as i64; // 128 campioni coperti da 64 punti
    let fac = 1.0 / (2.0 * NSS as f32);
    let i1 = i0;
    let i2 = i0 + 33 * NSS as i64;
    let i3 = i0 + 66 * NSS as i64;
    let i4 = i0 + 99 * NSS as i64;

    let mut z1 = ZERO;
    if i1 >= 0 && i1 + span <= np {
        z1 = dot(cd0, i1 as usize, &set[0], NREF);
    } else if i1 < 0 {
        let npts = (i1 + span - 1) / 2;
        if npts > 16 {
            // cd0(0:i1+127:2) contro csync2(64-npts:64) (da 1): npts + 1 punti
            z1 = dot(cd0, 0, &set[0][NREF - 1 - npts as usize..], npts as usize + 1);
        }
    }
    let mut z2 = ZERO;
    if i2 >= 0 && i2 + span <= np {
        z2 = dot(cd0, i2 as usize, &set[1], NREF);
    }
    let mut z3 = ZERO;
    if i3 >= 0 && i3 + span <= np {
        z3 = dot(cd0, i3 as usize, &set[2], NREF);
    }
    let mut z4 = ZERO;
    if i4 >= 0 && i4 + span <= np {
        z4 = dot(cd0, i4 as usize, &set[3], NREF);
    } else if i4 + span - 1 > np - 1 {
        let npts = (np - i4) / 2;
        if npts > 16 && i4 >= 0 {
            z4 = dot(cd0, i4 as usize, &set[3], npts as usize);
        }
    }
    (z1.norm() + z2.norm() + z3.norm() + z4.norm()) * fac
}

/// Il migliore allineamento trovato da [`search`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Alignment {
    /// Potenza di sincronismo (`smax`).
    pub sync: f32,
    /// Campione (a 666,67 Hz) da cui comincia il primo simbolo di Costas.
    pub start: i64,
    /// Correzione di frequenza in Hz (si somma alla frequenza di discesa).
    pub df_hz: i64,
}

/// Le tre finestre di ricerca del ritardo di `ft4_decode` (campioni a 666,67 Hz, con scarto medio zero): da -1,0 a +1,4 s, oppure, per l'ascolto
/// largo (`swl`), da -1,2 a +1,7 s. Si provano nell'ordine: attorno allo zero, in avanti, indietro.
pub fn dt_windows(swl: bool) -> [(i64, i64); 3] {
    if swl { [(179, 823), (824, 1467), (-467, 178)] } else { [(194, 730), (731, 1266), (-344, 193)] }
}

/// Cerca ritardo e frequenza migliori in `[ibmin, ibmax]`: una ricerca grossa (frequenza a passi di 3 Hz tra -12 e +12, ritardo a passi di 4
/// campioni) e poi una fine attorno al massimo (+-4 Hz a passi di 1, +-5 campioni a passi di 1). `cd` e' il segnale gia' normalizzato.
pub fn search(cd: &[Complex], tables: &SyncTables, ibmin: i64, ibmax: i64) -> Alignment {
    let mut best = Alignment { sync: -99.0, start: -1, df_hz: 0 };
    for isync in 0..2 {
        let (idfmin, idfmax, idfstp, bmin, bmax, bstp) = if isync == 0 {
            (-12, 12, 3usize, ibmin, ibmax, 4usize)
        } else {
            (best.df_hz - 4, best.df_hz + 4, 1usize, best.start - 5, best.start + 5, 1usize)
        };
        let mut found = Alignment { sync: -99.0, start: -1, df_hz: 0 };
        for idf in (idfmin..=idfmax).step_by(idfstp) {
            for istart in (bmin..=bmax).step_by(bstp) {
                let sync = sync4d(cd, istart, tables, idf);
                if sync > found.sync {
                    found = Alignment { sync, start: istart, df_hz: idf };
                }
            }
        }
        best = found;
    }
    best
}

// ---------------------------------------------------------------------------------------------------------------------
// get_ft4_bitmetrics

/// I 103 simboli del segnale (`NN * NSS` campioni complessi), presi da `cb` a partire dal campione `start` (puo' essere negativo: il segnale
/// comincia prima dell'audio, i campioni mancanti sono zero).
pub fn frame(cb: &[Complex], start: i64) -> Vec<Complex> {
    let total = NN * NSS;
    let mut cd = vec![ZERO; total];
    let limit = cb.len().min(NDMAX) as i64;
    for (k, c) in cd.iter_mut().enumerate() {
        let src = start + k as i64;
        if (0..limit).contains(&src) {
            *c = cb[src as usize];
        }
    }
    cd
}

/// Bit dei toni di Costas nel codeword, nelle quattro posizioni (bit 0, 66, 132 e 198), come li confronta il decoder.
const SYNC_BITS: [[u8; 8]; 4] = [[0, 0, 0, 1, 1, 0, 1, 1], [0, 1, 0, 0, 1, 1, 1, 0], [1, 1, 1, 0, 0, 1, 0, 0], [1, 0, 1, 1, 0, 0, 0, 1]];
/// Posizione del primo bit di ogni matrice di Costas tra i 206 delle metriche.
const SYNC_BIT_START: [usize; 4] = [0, 66, 132, 198];

/// Le tre serie di LLR dei 174 bit e la qualita' del sincronismo.
#[derive(Debug, Clone, PartialEq)]
pub struct Metrics {
    /// LLR (positivo = bit 1) letti a 1, 2 e 4 simboli alla volta, gia' normalizzati e moltiplicati per 2,83.
    pub llr: [[f32; 174]; 3],
    /// Bit di sincronismo (su 32) che le decisioni dure della prima serie hanno giusti (`nsync_qual`).
    pub sync_bits: usize,
}

fn argmax4(v: &[f32; 4]) -> usize {
    let mut best = 0;
    for i in 1..4 {
        if v[i] > v[best] {
            best = i;
        }
    }
    best
}

/// Le tre serie di LLR dei 174 bit del segnale allineato (`cd` di `NN * NSS` campioni, tono 0 a 0 Hz); `None` se meno di 8 dei 16 simboli di
/// Costas hanno il tono piu' forte al posto giusto o se le metriche sono nulle.
pub fn bit_metrics(cd: &[Complex]) -> Option<Metrics> {
    if cd.len() < NN * NSS {
        return None;
    }
    let fft32 = fft_plan(NSS, false);
    let mut scratch = vec![ZERO; fft32.get_inplace_scratch_len()];
    let mut cs = vec![[ZERO; 4]; NN];
    let mut s4 = vec![[0.0f32; 4]; NN];
    for k in 0..NN {
        let mut c = [ZERO; NSS];
        c.copy_from_slice(&cd[k * NSS..(k + 1) * NSS]);
        c[0] *= 1.9;
        c[NSS - 1] *= 1.9;
        fft32.process_with_scratch(&mut c, &mut scratch);
        for t in 0..4 {
            cs[k][t] = c[t];
            s4[k][t] = c[t].norm();
        }
    }

    // controllo di sincronismo: simboli di Costas con il tono piu' forte al posto giusto
    let mut nsync = 0;
    for (m, costas) in COSTAS.iter().enumerate() {
        for (k, &tone) in costas.iter().enumerate() {
            if argmax4(&s4[k + 33 * m]) == usize::from(tone) {
                nsync += 1;
            }
        }
    }
    if nsync < 8 {
        return None;
    }

    let mut bm = [[0.0f32; NBM]; 3];
    let mut s2 = [0.0f32; 256];
    for (nseq, &nsym) in [1usize, 2, 4].iter().enumerate() {
        let nt = 1usize << (2 * nsym);
        let ibmax = match nsym {
            1 => 1,
            2 => 3,
            _ => 7,
        };
        for ks in (0..=NN - nsym).step_by(nsym) {
            for (i, v) in s2.iter_mut().enumerate().take(nt) {
                let g = |shift: usize| usize::from(GRAYMAP[(i >> shift) & 3]);
                *v = match nsym {
                    1 => cs[ks][g(0)].norm(),
                    2 => (cs[ks][g(2)] + cs[ks + 1][g(0)]).norm(),
                    _ => (cs[ks][g(6)] + cs[ks + 1][g(4)] + cs[ks + 2][g(2)] + cs[ks + 3][g(0)]).norm(),
                };
            }
            let ipt = 2 * ks;
            for ib in 0..=ibmax {
                if ipt + ib >= NBM {
                    continue;
                }
                let bit = ibmax - ib;
                let mut one = f32::MIN;
                let mut zero = f32::MIN;
                for (i, &v) in s2.iter().enumerate().take(nt) {
                    if (i >> bit) & 1 == 1 {
                        one = one.max(v);
                    } else {
                        zero = zero.max(v);
                    }
                }
                bm[nseq][ipt + ib] = one - zero;
            }
        }
    }
    // gli ultimi bit non vengono dalle letture lunghe: si copiano da quelle piu' corte
    let tail = [bm[0][204], bm[0][205]];
    bm[1][204..].copy_from_slice(&tail);
    bm[2][204..].copy_from_slice(&tail);
    let mid: [f32; 4] = [bm[1][200], bm[1][201], bm[1][202], bm[1][203]];
    bm[2][200..204].copy_from_slice(&mid);
    // normalizebmet: radice del valore quadratico medio
    for set in bm.iter_mut() {
        let sig = (set.iter().map(|b| b * b).sum::<f32>() / NBM as f32).sqrt();
        if !sig.is_finite() || sig <= 0.0 {
            return None;
        }
        for b in set.iter_mut() {
            *b /= sig;
        }
    }

    let mut sync_bits = 0;
    for (expected, &start) in SYNC_BITS.iter().zip(&SYNC_BIT_START) {
        for (i, &e) in expected.iter().enumerate() {
            if u8::from(bm[0][start + i] >= 0.0) == e {
                sync_bits += 1;
            }
        }
    }

    let mut llr = [[0.0f32; 174]; 3];
    for (out, set) in llr.iter_mut().zip(bm.iter()) {
        out[..58].copy_from_slice(&set[8..66]);
        out[58..116].copy_from_slice(&set[74..132]);
        out[116..].copy_from_slice(&set[140..198]);
        for v in out.iter_mut() {
            *v *= LLR_SCALE;
        }
    }
    Some(Metrics { llr, sync_bits })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::modem;
    use crate::testutil::slot;
    use ft8::message::{self, CallHashes};

    #[test]
    fn i_bit_di_costas_sono_quelli_dei_toni() {
        // tono -> coppia di bit con la codifica Gray inversa (0 -> 00, 1 -> 01, 3 -> 10, 2 -> 11)
        let bits_of = |t: u8| -> [u8; 2] {
            let v = GRAYMAP.iter().position(|&g| g == t).expect("tono valido") as u8;
            [v >> 1, v & 1]
        };
        for (m, costas) in COSTAS.iter().enumerate() {
            let derived: Vec<u8> = costas.iter().flat_map(|&t| bits_of(t)).collect();
            assert_eq!(derived, SYNC_BITS[m], "matrice {m}");
        }
        assert_eq!(SYNC_BIT_START, [0, 66, 132, 198]);
    }

    #[test]
    fn candidato_alla_frequenza_giusta() {
        let audio = slot(&[("CQ K1ABC FN42", 1500.0, 0.2, 0.0)], 1, 0.05);
        let cands = find_candidates(&audio, 200.0, 4000.0, 1.2, 100, None);
        assert!(!cands.is_empty());
        let best = cands.iter().max_by(|a, b| a.sync.total_cmp(&b.sync)).expect("elenco non vuoto");
        assert!((best.freq_hz - 1500.0).abs() < 5.0, "{best:?}");
        assert!(best.sync > 2.0, "{best:?}");
    }

    #[test]
    fn solo_rumore_o_silenzio_non_rompono_la_ricerca() {
        assert!(find_candidates(&[], 200.0, 4000.0, 1.2, 100, None).is_empty());
        assert!(find_candidates(&vec![0.0; 90_000], 200.0, 4000.0, 1.2, 100, None).is_empty());
        assert!(find_candidates(&vec![0.1; 1000], 3000.0, 100.0, 1.2, 100, None).is_empty());
        let noise = slot(&[], 5, 0.05);
        let cands = find_candidates(&noise, 200.0, 4000.0, 1.2, 100, None);
        assert!(cands.len() < 40, "troppi picchi nel rumore: {}", cands.len());
    }

    #[test]
    fn candidati_vicini_al_qso_vengono_per_primi() {
        let audio = slot(&[("CQ K1ABC FN42", 800.0, 0.2, 5.0), ("IU3ABC K1ABC -15", 2200.0, 0.2, 5.0)], 2, 0.05);
        let cands = find_candidates(&audio, 200.0, 4000.0, 1.2, 100, Some(2200.0));
        assert!((cands[0].freq_hz - 2200.0).abs() < 40.0, "{:?}", &cands[..cands.len().min(3)]);
    }

    #[test]
    fn discesa_porta_il_tono_zero_a_zero_hz() {
        // un tono puro a 1000 Hz: dopo la discesa a f0 = 1000 resta un numero complesso fermo (frequenza 0)
        let audio: Vec<f32> = (0..NMAX).map(|i| (2.0 * PI * 1000.0 * i as f64 / 12000.0).sin() as f32).collect();
        let ds = Downsampler::new(&audio);
        let cd = ds.run(1000.0);
        assert_eq!(cd.len(), NDMAX);
        let mid = &cd[1000..3000];
        let power: f32 = mid.iter().map(|c| c.norm_sqr()).sum::<f32>() / mid.len() as f32;
        assert!(power > 1e-4, "potenza {power}");
        // fase stabile: il prodotto di campioni vicini ha argomento ~0
        let drift: f32 = mid.windows(2).map(|w| (w[1] * w[0].conj()).arg().abs()).sum::<f32>() / (mid.len() - 1) as f32;
        assert!(drift < 0.02, "deriva {drift}");
        // a 30 Hz di distanza (fuori dal filtro di -10..+73 Hz se piu' basso): nessuna energia
        let off = ds.run(1100.0);
        let p_off: f32 = off[1000..3000].iter().map(|c| c.norm_sqr()).sum::<f32>() / 2000.0;
        assert!(p_off < power * 1e-3, "{p_off} contro {power}");
    }

    #[test]
    fn sync4d_trova_ritardo_e_frequenza() {
        // segnale pulito: il massimo di sync4d cade sul ritardo vero e sulla correzione vera
        let freq = 1500.0f32;
        let dt = 0.2f32;
        let audio = slot(&[("CQ K1ABC FN42", freq + 7.0, dt, 20.0)], 3, 0.001);
        let ds = Downsampler::new(&audio);
        let mut cd = ds.run(freq);
        normalize(&mut cd, NDMAX as f32);
        let tables = SyncTables::new();
        let (mut best_sync, mut best_start, mut best_df) = (-1.0f32, 0i64, 0i64);
        for (lo, hi) in dt_windows(false) {
            let a = search(&cd, &tables, lo, hi);
            if a.sync > best_sync {
                (best_sync, best_start, best_df) = (a.sync, a.start, a.df_hz);
            }
        }
        let expected_start = (0.5 + f64::from(dt)) * f64::from(DOWN_RATE);
        assert!((best_start as f64 - expected_start).abs() <= 1.5, "start {best_start} contro {expected_start}");
        assert!((best_df - 7).abs() <= 1, "df {best_df}");
        assert!(best_sync > 1.5, "sync {best_sync}");
    }

    #[test]
    fn le_metriche_di_un_segnale_pulito_danno_il_codeword() {
        let text = "IU3ABC K1ABC -15";
        let audio = slot(&[(text, 1000.0, 0.1, 15.0)], 4, 0.01);
        let ds = Downsampler::new(&audio);
        let mut cd0 = ds.run(1000.0);
        normalize(&mut cd0, NDMAX as f32);
        let a = search(&cd0, &SyncTables::new(), 194, 730);
        let mut cb = ds.run(1000.0 + a.df_hz as f32);
        normalize(&mut cb, (NSS * NN) as f32);
        let metrics = bit_metrics(&frame(&cb, a.start)).expect("sincronismo buono");
        assert!(metrics.sync_bits >= 30, "bit di sincronismo {}", metrics.sync_bits);
        let bits = message::pack(text, &mut CallHashes::default()).expect("messaggio valido");
        let cw = modem::codeword_from_bits77(&bits);
        for (s, set) in metrics.llr.iter().enumerate() {
            let errors = set.iter().zip(cw.iter()).filter(|&(&l, &b)| (l > 0.0) != (b == 1)).count();
            assert!(errors <= 5, "serie {s}: {errors} bit sbagliati");
            let rms = (set.iter().map(|l| l * l).sum::<f32>() / 174.0).sqrt();
            assert!(rms > 2.0 && rms < 3.5, "scala {rms}");
        }
    }

    #[test]
    fn rumore_puro_non_ha_un_sincronismo_forte() {
        let noise = slot(&[], 9, 0.05);
        let ds = Downsampler::new(&noise);
        let mut cd = ds.run(1234.0);
        normalize(&mut cd, NDMAX as f32);
        let a = search(&cd, &SyncTables::new(), 194, 730);
        // il massimo su 1200 posizioni nel rumore supera 1,2 (per questo il decoder chiede anche i bit di sincronismo), ma resta lontano da un segnale
        assert!(a.sync < 3.0, "sync nel rumore {}", a.sync);
    }
}
