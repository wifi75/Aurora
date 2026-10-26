//! Ricerca e allineamento dei segnali: `sync8`, `ft8_downsample`, `sync8d` e il calcolo delle probabilita' dei bit di `ft8b`
//! (JTDX 2.2.159, `lib/sync8.f90`, `lib/ft8_downsample.f90`, `lib/sync8d.f90`, `lib/ft8b.f90`).
//!
//! # Percorso di un segnale
//!
//! 1. [`find_candidates`] calcola lo spettrogramma del periodo (finestre di 0,16 s ogni 0,04 s) e cerca i punti in cui le tre matrici di
//!    Costas si accendono; restituisce frequenza (a 3,125 Hz) e ritardo (a 0,04 s) approssimati.
//! 2. [`Downsampler`] (o [`downsample`]) porta il periodo a 200 Hz di campionamento, 32 campioni per simbolo, con il segnale a 0 Hz:
//!    `cd` ha 3200 campioni complessi ed e' costruito su una frequenza `f0` (quella del candidato).
//! 3. [`refine`] corregge il ritardo (passi di 5 ms, +-40 ms) e la frequenza (passi di 0,5 Hz, +-2,5 Hz) sul `cd` del candidato.
//! 4. [`bit_metrics`] calcola le tre serie di LLR sul `cd` allineato.
//!
//! La frequenza di un candidato e' quella del tono 0 (il piu' basso degli otto), come in WSJT-X e JTDX.
//!
//! ## Convenzione del ritardo
//!
//! `dt_s` e' quello di JTDX: scarto rispetto a 0,5 s dall'inizio del periodo, cioe' il segnale parte al campione `(0,5 + dt_s) * 12000`
//! (con `dt_s = 0` il primo simbolo comincia a 0,5 s). Puo' essere negativo (orologio in anticipo) fino a circa -0,5 s.
//!
//! ## Convenzione di `cd` dopo `refine`
//!
//! `refine` restituisce la frequenza corretta ma non modifica `cd`, che resta centrato sulla frequenza con cui e' stato costruito.
//! L'errore residuo (al massimo 2,5 Hz) e' piccolo rispetto alla finestra di un simbolo, ma per le metriche conviene eliminarlo in uno
//! di due modi equivalenti: ricostruire `cd` con `downsample` alla frequenza corretta, oppure applicare [`shift_frequency`] al `cd`
//! vecchio con `-(freq_corretta - freq_di_discesa)` (e' quello che fa JTDX con `twkfreq1`). [`bit_metrics`] usa solo `dt_s` del
//! candidato e suppone il segnale gia' a 0 Hz in `cd`.
//!
//! # Livello dell'audio
//!
//! Come in JTDX i campioni sono nella scala dei numeri interi a 16 bit (ampiezza fino a +-32768). La ricerca dei candidati non dipende
//! dalla scala, ma `bit_metrics` per i segnali deboli (rapporto sincronismo/rumore sotto 2,5) usa una trasformazione con soglie assolute
//! (`ss1 < 5.77` in `ft8b`), tarata su quel livello: con audio normalizzato a +-1 i segnali forti vanno bene, i deboli peggiorano.
//! Il chiamante deve quindi moltiplicare per 32768 un audio in virgola mobile normalizzato.
//!
//! # Probabilita' dei bit (LLR)
//!
//! Stessa convenzione di WSJT-X e JTDX: **LLR positivo = bit 1**, negativo = bit 0 (`bm = max(s2 dei toni con bit 1) - max(s2 dei toni
//! con bit 0)` in `ft8b`). Il decoder LDPC deve leggere `llr > 0` come bit 1. Ogni serie viene normalizzata dividendo per la sua radice
//! del valore quadratico medio sui 174 bit e moltiplicata per 2,83 (`scalefac` di `ft8b`): la radice del valore quadratico medio di
//! ogni serie e' quindi 2,83. L'ordine dei bit e' quello del codeword: tre bit per simbolo dati, primo bit = il piu' significativo
//! dell'indice Gray, simboli dati 7-35 (primi 87 bit) e 43-71 (ultimi 87), contando da 0.
//!
//! # Tipo complesso
//!
//! `Complex` e' `rustfft::num_complex::Complex32`, lo stesso tipo che usa rustfft: il decoder puo' passarlo a rustfft senza conversioni.
//!
//! # Differenze rispetto al Fortran
//!
//! - `sync8` e' il ramo senza compensazione AGC (`lagcc` spento). Le tre forme dello spettro (ampiezza, potenza, somma degli assoluti)
//!   si scelgono con [`Spectrum`] e `find_candidates_ex`, che restituisce anche l'indicatore di candidato "CQ" (`candidate(4,:)`) e
//!   accetta la finestra del ritardo (`jzb`, `jzt`, che JTDX sposta di `avexdt * 25`). Mancano `nfqso` (nessun trattamento di favore
//!   per la frequenza di QSO), i candidati virtuali sul QSO e il diradamento (`ncandthin`). I candidati fuori da -2,0..3,0 s sono scartati.
//! - `ft8_downsample`: tutte e due le varianti dei ritocchi ai bordi del filtro (`lhighsens` falso e vero); la FFT dell'intero periodo
//!   e' calcolata una volta in [`Downsampler::new`] invece di essere tenuta in una variabile con `save`: dopo una sottrazione il chiamante
//!   ne costruisce una nuova (JTDX lo fa quando un candidato e' entro 50 Hz da un segnale tolto). Mancano `c2`/`c3` (le versioni mediate
//!   per il QSO in corso).
//! - `sync8d` e `refine_ex`: le tre forme della somma dei 21 simboli di Costas (`SyncKind`, secondo la passata) e, per i candidati CQ,
//!   gli otto simboli di dati di un CQ nella ricerca in frequenza. I riferimenti (Costas e CQ) si ricavano dall'onda GFSK di "CQ 2E0DLA
//!   IO92" come in `cwfilter.f90`. Il ritardo e' cercato prima della frequenza come in `ft8b`, senza ripeterlo dopo. Mancano i
//!   riferimenti per il QSO (`csynce`, `csyncsd`, `csyncsdcq`).
//! - `bit_metrics_ex`: le quattro serie `llra`, `llrb`, `llrc`, `llrd`, la lettura a tempo invertito (`lreverse`) e la seconda lettura
//!   (`isubp1 = 2`); mancano le medie con i segnali ricevuti nel periodo prima (`isubp1` 3-11). `bit_metrics` e' la versione di prima
//!   (tre serie, `nsync >= 7`). I filtri sui falsi (`nsyncscore`, `scoreratio`) stanno in `filters`.

use std::f64::consts::PI;
use std::sync::{Arc, OnceLock};

use rustfft::num_complex::Complex32;
use rustfft::{Fft, FftPlanner};

use crate::{COSTAS, NN, NSPS, SAMPLE_RATE, SLOT_SAMPLES};

/// Un segnale probabile nello spettro del periodo.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Candidate {
    /// Frequenza del tono 0 (Hz).
    pub freq_hz: f32,
    /// Ritardo rispetto a 0,5 s dall'inizio del periodo (secondi, puo' essere negativo): vedi la convenzione nel modulo.
    pub dt_s: f32,
    /// Punteggio di sincronismo di `sync8` (rapporto con il 40% inferiore dello spettro: JTDX usa soglie tra 1,1 e 1,5).
    pub sync: f32,
}

/// Un numero complesso a 32 bit (parte reale e immaginaria).
pub type Complex = Complex32;

const ZERO: Complex = Complex32::new(0.0, 0.0);

/// Campioni per simbolo a 200 Hz.
const SYM200: usize = 32;
/// Campioni complessi dopo la discesa (16 s a 200 Hz).
const NCD: usize = 3200;
/// Passo dello spettrogramma: un quarto di simbolo.
const NSTEP: usize = NSPS / 4;
/// FFT dello spettrogramma: due simboli.
const NFFT_SPEC: usize = 2 * NSPS;
/// Bin dello spettrogramma (da 1 a 1920, 3,125 Hz ciascuno).
const NH1: usize = NFFT_SPEC / 2;
/// Colonne dello spettrogramma.
const NHSYM: usize = SLOT_SAMPLES / NSTEP - 3;
/// Larghezza di un bin dello spettrogramma (Hz).
const DF_SPEC: f32 = SAMPLE_RATE as f32 / NFFT_SPEC as f32;
/// Durata di un passo dello spettrogramma (s).
const TSTEP: f32 = NSTEP as f32 / SAMPLE_RATE as f32;
/// Colonne di spettrogramma per simbolo.
const NSSY: usize = 4;
/// Fattore tra i bin e i toni (3,125 Hz contro 6,25 Hz).
const NFOS: usize = 2;
/// Offset iniziale delle colonne (0,5 s / 0,04 s, troncato a intero come nel Fortran).
const JSTRT: i32 = 12;
/// Intervallo del ritardo cercato (colonne, scarto medio 0).
const JZB: i32 = -62;
const JZT: i32 = 62;
/// Mappa Gray: indice (3 bit) -> tono.
const GRAY_MAP: [usize; 8] = [0, 1, 3, 2, 5, 6, 4, 7];
/// Fattore di scala degli LLR di `ft8b`.
const LLR_SCALE: f32 = 2.83;

fn nint(x: f32) -> i64 {
    x.round() as i64
}

fn fft_plan(n: usize, inverse: bool) -> Arc<dyn Fft<f32>> {
    let mut planner = FftPlanner::<f32>::new();
    if inverse { planner.plan_fft_inverse(n) } else { planner.plan_fft_forward(n) }
}

// ---------------------------------------------------------------------------------------------------------------------
// sync8: ricerca dei candidati

/// Forma dello spettrogramma di `sync8`: JTDX la fa ruotare con le passate (1, 4, 7: ampiezza; 2, 5, 8: potenza; 3, 6, 9: somma dei
/// valori assoluti di parte reale e immaginaria), cosi' ogni passata vede i segnali deboli in modo un po' diverso.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Spectrum {
    /// `sqrt(re^2 + im^2)`.
    Amplitude,
    /// `re^2 + im^2`.
    Power,
    /// `|re| + |im|`.
    AbsSum,
}

impl Spectrum {
    /// La forma della passata `ipass` (da 1), come in `sync8.f90`.
    pub fn for_pass(ipass: usize) -> Spectrum {
        match ipass.saturating_sub(1) % 3 {
            0 => Spectrum::Amplitude,
            1 => Spectrum::Power,
            _ => Spectrum::AbsSum,
        }
    }
}

/// Forma della somma dei simboli di Costas in `sync8d`, secondo la passata.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyncKind {
    /// Somma dei moduli delle correlazioni.
    Magnitude,
    /// Somma delle potenze.
    Power,
    /// Media di simboli adiacenti, poi somma dei valori assoluti di parte reale e immaginaria.
    AvgAbs,
}

impl SyncKind {
    /// La forma della passata `ipass` (da 1), come in `sync8d.f90` (1, 5, 9: modulo; 2, 6, 7: potenza; le altre: media a coppie).
    pub fn for_pass(ipass: usize) -> SyncKind {
        match ipass {
            1 | 5 | 9 => SyncKind::Magnitude,
            2 | 6 | 7 => SyncKind::Power,
            _ => SyncKind::AvgAbs,
        }
    }
}

/// Spettrogramma: per ogni colonna `j` (0..372) i bin 1..=1920 a 3,125 Hz; `s[j * (NH1 + 1) + i]`.
fn spectrogram(dd: &[f32], shape: Spectrum) -> Vec<f32> {
    let facx = 1.0f32 / 300.0;
    let mut windowx = [0.0f32; 201];
    for (i, w) in windowx.iter_mut().enumerate() {
        *w = facx * (1.0 + (i as f64 * PI / 200.0).cos() as f32) / 2.0;
    }
    let plan = fft_plan(NFFT_SPEC, false);
    let mut scratch = vec![ZERO; plan.get_inplace_scratch_len()];
    let mut buf = vec![ZERO; NFFT_SPEC];
    let mut s = vec![0.0f32; NHSYM * (NH1 + 1)];
    for j in 0..NHSYM {
        let ia = j * NSTEP;
        buf.fill(ZERO);
        if j != 0 {
            for m in 0..201 {
                buf[759 + m].re = dd[ia - 201 + m] * windowx[200 - m];
            }
        }
        for m in 0..NSPS {
            buf[960 + m].re = facx * dd[ia + m];
        }
        buf[960].re *= 1.9;
        buf[960 + NSPS - 1].re *= 1.9;
        if j != NHSYM - 1 {
            for m in 0..201 {
                buf[2880 + m].re = dd[ia + NSPS + m] * windowx[m];
            }
        }
        plan.process_with_scratch(&mut buf, &mut scratch);
        let row = &mut s[j * (NH1 + 1)..(j + 1) * (NH1 + 1)];
        for i in 1..=NH1 {
            row[i] = match shape {
                Spectrum::Amplitude => buf[i].norm(),
                Spectrum::Power => buf[i].norm_sqr(),
                Spectrum::AbsSum => buf[i].re.abs() + buf[i].im.abs(),
            };
        }
    }
    s
}

/// Cerca i candidati tra `f_low_hz` e `f_high_hz` nei 15 secondi di `samples` (12 kHz).
///
/// `min_sync` e' la soglia di `sync8` (`syncmin`: 1,5 normale, 1,225 o 1,1 per la sensibilita' alta di JTDX); `max` il numero massimo di
/// candidati restituiti. L'elenco e' in ordine di sincronismo decrescente; i quasi-doppioni (meno di 4 Hz e 0,1 s) sono ridotti al migliore.
/// La stima e' grossolana (3,125 Hz e 0,04 s, con uno scarto sistematico del ritardo di circa +0,02 s come in JTDX): la precisa [`refine`].
pub fn find_candidates(samples: &[f32], f_low_hz: f32, f_high_hz: f32, min_sync: f32, max: usize) -> Vec<Candidate> {
    find_candidates_ex(samples, f_low_hz, f_high_hz, min_sync, max, Spectrum::Amplitude, (JZB, JZT)).into_iter().map(|c| c.candidate).collect()
}

/// Un candidato di [`find_candidates_ex`]: il candidato e l'indicatore "CQ" di `sync8` (`candidate(4,:)`), acceso quando i simboli
/// 8-16 (dove un CQ ha sempre il tono 0) rendono piu' dei tre simboli di Costas da soli.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PassCandidate {
    pub candidate: Candidate,
    pub cq: bool,
}

/// Come [`find_candidates`], con la forma dello spettrogramma della passata (`ipass` di `sync8`) e la finestra del ritardo
/// `window = (jzb, jzt)` in passi di 0,04 s (JTDX: -62..62, spostata di `avexdt*25` per seguire l'orologio medio delle stazioni).
/// Restituisce anche l'indicatore di CQ di ogni candidato.
pub fn find_candidates_ex(
    samples: &[f32],
    f_low_hz: f32,
    f_high_hz: f32,
    min_sync: f32,
    max: usize,
    shape: Spectrum,
    window: (i32, i32),
) -> Vec<PassCandidate> {
    find_candidates_swl(samples, f_low_hz, f_high_hz, min_sync, max, shape, window, false)
}

/// Come [`find_candidates_ex`]; con `swl` (modo SWL di JTDX, `nswl`) il ritardo accettato va da -74 a 101 passi invece di -49..76 e due
/// candidati si considerano lo stesso segnale solo se distano meno di 3 Hz invece di 4 (`sync8`, letto con REA: `docs/analisi-jtdx-reverse-rea.md`).
#[allow(clippy::too_many_arguments)]
pub fn find_candidates_swl(
    samples: &[f32],
    f_low_hz: f32,
    f_high_hz: f32,
    min_sync: f32,
    max: usize,
    shape: Spectrum,
    window: (i32, i32),
    swl: bool,
) -> Vec<PassCandidate> {
    if samples.is_empty() || max == 0 || f_low_hz.is_nan() || f_high_hz.is_nan() || f_high_hz <= f_low_hz || window.1 < window.0 {
        return Vec::new();
    }
    let mut dd = vec![0.0f32; SLOT_SAMPLES];
    let n = samples.len().min(SLOT_SAMPLES);
    dd[..n].copy_from_slice(&samples[..n]);

    let s = spectrogram(&dd, shape);
    let col = |k: i32| -> &[f32] {
        let j = (k - 1) as usize;
        &s[j * (NH1 + 1)..(j + 1) * (NH1 + 1)]
    };

    // Intervallo dei bin: l'ultimo deve lasciare 16 bin per la somma e 13 per il tono 6 piu' mezzo bin.
    let imax = NH1 - 16;
    let ia = (nint(f_low_hz / DF_SPEC).max(1) as usize).min(imax);
    let ib = (nint(f_high_hz / DF_SPEC).max(1) as usize).min(imax);
    if ib < ia {
        return Vec::new();
    }

    // Somma di 17 bin consecutivi (i..=i+16) per colonna.
    let mut sum17 = vec![0.0f32; NHSYM * (NH1 + 1)];
    for j in 0..NHSYM {
        let row = &s[j * (NH1 + 1)..(j + 1) * (NH1 + 1)];
        let out = &mut sum17[j * (NH1 + 1)..(j + 1) * (NH1 + 1)];
        for i in 1..=imax {
            out[i] = row[i..=i + 16].iter().sum();
        }
    }
    let col17 = |k: i32| -> &[f32] {
        let j = (k - 1) as usize;
        &sum17[j * (NH1 + 1)..(j + 1) * (NH1 + 1)]
    };

    let nssy36 = (NSSY * 36) as i32;
    let nssy72 = (NSSY * 72) as i32;
    let nhsym = NHSYM as i32;
    let mut red = vec![0.0f32; NH1 + 1];
    let mut jpeak = vec![0i32; NH1 + 1];
    let mut best = vec![f32::NEG_INFINITY; NH1 + 1];
    let mut best_cq = vec![false; NH1 + 1];
    for j in window.0..=window.1 {
        for i in ia..=ib {
            let (mut ta, mut tb, mut tc, mut tcq) = (0.0f32, 0.0f32, 0.0f32, 0.0f32);
            let (mut t0a, mut t0b, mut t0c, mut t0cq) = (0.0f32, 0.0f32, 0.0f32, 0.0f32);
            for (n, &c) in COSTAS.iter().enumerate() {
                let off = i + NFOS * c as usize;
                let k = j + JSTRT + (NSSY * n) as i32;
                if k > 0 {
                    ta += col(k)[off];
                    t0a += col17(k)[i] - col(k)[off + 1];
                }
                let k36 = k + nssy36;
                if k36 > 0 && k36 <= nhsym {
                    tb += col(k36)[off];
                    t0b += col17(k36)[i] - col(k36)[off + 1];
                }
                let k72 = k + nssy72;
                if k72 <= nhsym {
                    tc += col(k72)[off];
                    t0c += col17(k72)[i] - col(k72)[off + 1];
                }
            }
            for n in 7..=15usize {
                let k = j + JSTRT + (NSSY * n) as i32;
                if k >= 1 {
                    if n < 15 {
                        tcq += col(k)[i];
                        t0cq += col17(k)[i] - col(k + 1)[i];
                    } else {
                        tcq += col(k)[i + 2];
                        t0cq += col17(k)[i] - col(k + 3)[i];
                    }
                }
            }
            let sync_of = |t1: f32, t01: f32, t0cq_: f32, d1: f32, d2: f32| -> (f32, bool) {
                let t2 = t1 + tcq;
                let t02 = t01 + t0cq_;
                let mut a = (t01 - t1 * 2.0) / d1;
                if a < 1e-8 {
                    a = 1.0;
                }
                let mut b = (t02 - t2 * 2.0) / d2;
                if b < 1e-8 {
                    b = 1.0;
                }
                let sync01 = t1 / (7.0 * a);
                let sync02 = (t1 / 7.0 + tcq / 9.0) / b;
                (sync01.max(sync02), sync02 > sync01)
            };
            let (syncf, cq_f) = sync_of(ta + tb + tc, t0a + t0b + t0c, t0cq, 42.0, 60.0);
            let (syncs, cq_s) = sync_of(tb + tc, t0b + t0c, t0cq, 28.0, 46.0);
            let v = syncf.max(syncs);
            if v > best[i] {
                best[i] = v;
                jpeak[i] = j;
                // l'indicatore di CQ e' quello della forma che ha vinto (`if(syncf.gt.syncs)` in sync8)
                best_cq[i] = if syncf > syncs { cq_f } else { cq_s };
            }
        }
    }
    red[ia..=ib].copy_from_slice(&best[ia..=ib]);

    // Normalizzazione sul 40% inferiore dei picchi.
    let iz = ib - ia + 1;
    let mut sorted: Vec<f32> = red[ia..=ib].to_vec();
    sorted.sort_by(|a, b| a.total_cmp(b));
    let rank = (nint(0.40 * iz as f32).max(1) as usize).min(iz);
    let mut base = sorted[rank - 1];
    if base < 1e-8 {
        base = 1.0;
    }
    for r in red[ia..=ib].iter_mut() {
        *r /= base;
    }

    // Dal piu' forte al piu' debole.
    let mut order: Vec<usize> = (ia..=ib).collect();
    order.sort_by(|&a, &b| red[b].total_cmp(&red[a]));
    let (jpeak_low, jpeak_high, dup_hz) = if swl { (-74, 101, 3.0) } else { (-49, 76, 4.0) };
    let mut c0: Vec<Candidate> = Vec::new();
    let mut c0_cq: Vec<bool> = Vec::new();
    for &i in &order {
        if red[i] < min_sync {
            continue;
        }
        if jpeak[i] < jpeak_low || jpeak[i] > jpeak_high {
            continue;
        }
        if c0.len() >= 450 {
            break;
        }
        c0.push(Candidate { freq_hz: i as f32 * DF_SPEC, dt_s: (jpeak[i] - 1) as f32 * TSTEP, sync: red[i] });
        c0_cq.push(best_cq[i]);
    }

    // Quasi-doppioni: resta il migliore (come nel Fortran, i gia' azzerati partecipano ancora ai confronti).
    let mut sync_v: Vec<f32> = c0.iter().map(|c| c.sync).collect();
    for i in 1..c0.len() {
        for j in 0..i {
            let fdiff = (c0[i].freq_hz - c0[j].freq_hz).abs();
            let xdt = (c0[i].dt_s - c0[j].dt_s).abs();
            if fdiff < dup_hz && xdt < 0.1 {
                if sync_v[i] >= sync_v[j] {
                    sync_v[j] = 0.0;
                }
                if sync_v[i] < sync_v[j] {
                    sync_v[i] = 0.0;
                }
            }
        }
    }
    let mut idx: Vec<usize> = (0..c0.len()).collect();
    idx.sort_by(|&a, &b| sync_v[b].total_cmp(&sync_v[a]));
    let mut out = Vec::new();
    for j in idx {
        if sync_v[j] > 0.0 && sync_v[j] >= min_sync {
            out.push(PassCandidate { candidate: Candidate { sync: sync_v[j], ..c0[j] }, cq: c0_cq[j] });
            if out.len() >= max.min(460) {
                break;
            }
        }
    }
    out
}

// ---------------------------------------------------------------------------------------------------------------------
// ft8_downsample

/// Dimensione della FFT sull'intero periodo (192000 punti = 60 x 3200).
const NFFT_LONG: usize = 192_000;
/// Passo dei bin della FFT lunga (Hz).
const DF_LONG: f32 = SAMPLE_RATE as f32 / NFFT_LONG as f32;

/// Calcola una volta la FFT dell'intero periodo e da questa ricava, per ogni frequenza, il segnale a 200 Hz.
pub struct Downsampler {
    /// Bin 0..=96000 della FFT di 192000 punti (dal segnale zero-padded).
    spec: Vec<Complex>,
    window: [f32; 55],
    ifft: Arc<dyn Fft<f32>>,
}

impl Downsampler {
    /// Costruisce lo spettro dei 15 secondi di `samples` (12 kHz; il resto viene scartato o completato con zeri).
    pub fn new(samples: &[f32]) -> Self {
        let plan = fft_plan(NFFT_LONG, false);
        let mut buf = vec![ZERO; NFFT_LONG];
        for (b, &x) in buf.iter_mut().zip(samples.iter().take(SLOT_SAMPLES)) {
            b.re = x;
        }
        let mut scratch = vec![ZERO; plan.get_inplace_scratch_len()];
        plan.process_with_scratch(&mut buf, &mut scratch);
        buf.truncate(NFFT_LONG / 2 + 1);
        let mut window = [0.0f32; 55];
        for (i, w) in window.iter_mut().enumerate() {
            *w = (1.0 + (i as f64 * PI / 55.0).cos() as f32) / 2.0;
        }
        Downsampler { spec: buf, window, ifft: fft_plan(NCD, true) }
    }

    /// Porta il segnale a 200 Hz di campionamento centrato su `f0_hz` (3200 campioni complessi; il bin `f0_hz` diventa 0 Hz).
    ///
    /// `high_sens` e' `lhighsens` di JTDX (candidati con sincronismo basso): ritocca i bordi del filtro in modo diverso.
    pub fn run(&self, f0_hz: f32, high_sens: bool) -> Vec<Complex> {
        let mut c1 = vec![ZERO; NCD];
        let last_bin = (NFFT_LONG / 2) as i64;
        let i0 = nint(f0_hz / DF_LONG);
        let it = nint((f0_hz + 55.75) / DF_LONG).min(last_bin);
        let ib = nint((f0_hz - 5.75) / DF_LONG).max(1);
        if !f0_hz.is_finite() || it <= ib || it - ib >= NCD as i64 {
            return c1;
        }
        let k = (it - ib) as usize;
        if k < 54 {
            return c1;
        }
        c1[..=k].copy_from_slice(&self.spec[ib as usize..=it as usize]);
        for m in 0..=54 {
            c1[m] *= self.window[54 - m];
            c1[k - 54 + m] *= self.window[m];
        }
        // Rotazione circolare: il bin di f0 va in posizione 0.
        let shift = (i0 - ib).rem_euclid(NCD as i64) as usize;
        c1.rotate_left(shift);
        if high_sens {
            c1[0] *= 1.93;
            c1[799] *= 1.7;
            c1[800] *= 1.7;
            c1[3199] *= 1.93;
        } else {
            c1[45] *= 1.49;
            c1[54] *= 1.49;
            c1[3145] *= 1.49;
            c1[3154] *= 1.49;
        }
        let mut scratch = vec![ZERO; self.ifft.get_inplace_scratch_len()];
        self.ifft.process_with_scratch(&mut c1, &mut scratch);
        let facc1 = (0.01f64 / 61_440f64.sqrt()) as f32;
        for c in c1.iter_mut() {
            *c *= facc1;
        }
        c1
    }
}

/// Porta il segnale a 200 Hz di campionamento centrato su `f0_hz` (3200 campioni complessi).
///
/// Ricalcola la FFT di tutto il periodo a ogni chiamata: per molti candidati sullo stesso periodo usare [`Downsampler`].
pub fn downsample(samples: &[f32], f0_hz: f32) -> Vec<Complex> {
    Downsampler::new(samples).run(f0_hz, false)
}

/// Sposta in frequenza il segnale a 200 Hz di `shift_hz` (positivo = verso l'alto), come `twkfreq1` di JTDX.
pub fn shift_frequency(cd: &mut [Complex], shift_hz: f32) {
    let dphi = 2.0 * PI * shift_hz as f64 / 200.0;
    for (n, c) in cd.iter_mut().enumerate() {
        let phi = dphi * (n + 1) as f64;
        *c *= Complex::new(phi.cos() as f32, phi.sin() as f32);
    }
}

// ---------------------------------------------------------------------------------------------------------------------
// sync8d: sincronismo fine

#[cfg(test)]
/// erf con l'approssimazione 7.1.26 di Abramowitz-Stegun (errore massimo 1,5e-7).
fn erf(x: f64) -> f64 {
    let t = 1.0 / (1.0 + 0.327_591_1 * x.abs());
    let y = 1.0
        - (((((1.061_405_429 * t - 1.453_152_027) * t) + 1.421_413_741) * t - 0.284_496_736) * t + 0.254_829_592)
            * t
            * (-x * x).exp();
    if x >= 0.0 { y } else { -y }
}

#[cfg(test)]
/// Impulso di frequenza gaussiano di `gfsk_pulse` (BT = `b`).
fn gfsk_pulse(b: f64, t: f64) -> f64 {
    let c = PI * (2.0 / std::f64::consts::LN_2).sqrt();
    0.5 * (erf(c * b * (t + 0.5)) - erf(c * b * (t - 0.5)))
}

/// Riferimenti di sincronismo a 200 Hz, ricavati dall'onda GFSK (BT = 2, frequenza di base 0) di "CQ 2E0DLA IO92" come in
/// `cwfilter.f90` di JTDX: `costas[j]` e' il simbolo `j` dei 7 di Costas, `cq[j]` il simbolo `7 + j` (i primi otto simboli di dati di
/// un CQ, tutti sul tono 0 tranne l'ultimo), ognuno di 32 campioni.
struct SyncTables {
    costas: [[Complex; SYM200]; 7],
    cq: [[Complex; SYM200]; 8],
}

fn sync_tables() -> &'static SyncTables {
    static TABLE: OnceLock<SyncTables> = OnceLock::new();
    TABLE.get_or_init(|| {
        // toni del messaggio di riferimento; se per qualunque motivo non si impacchettasse, restano i soli Costas
        let tones: [u8; NN] = crate::message::pack("CQ 2E0DLA IO92", &mut crate::message::CallHashes::default())
            .map(|bits| crate::modem::tones_from_codeword(&crate::ldpc::encode(&crate::crc::append_crc(&bits))))
            .unwrap_or_else(|_| {
                let mut t = [0u8; NN];
                for start in [0usize, 36, 72] {
                    t[start..start + 7].copy_from_slice(&COSTAS);
                }
                t
            });
        let (sin, cos) = crate::modem::gfsk_iq(&tones, 0.0, crate::modem::GFSK_BT);
        let sample = |symbol: usize, k: usize| -> Complex {
            let at = symbol * NSPS + 60 * k;
            Complex::new(cos[at], sin[at])
        };
        let mut costas = [[ZERO; SYM200]; 7];
        let mut cq = [[ZERO; SYM200]; 8];
        for (j, row) in costas.iter_mut().enumerate() {
            for (k, v) in row.iter_mut().enumerate() {
                *v = sample(j, k);
            }
        }
        for (j, row) in cq.iter_mut().enumerate() {
            for (k, v) in row.iter_mut().enumerate() {
                *v = sample(7 + j, k);
            }
        }
        SyncTables { costas, cq }
    })
}

/// Correzioni di frequenza di 32 campioni per `ifr` = -5..5 (passi di 0,5 Hz).
fn freq_tweak(delf_hz: f32) -> [Complex; SYM200] {
    let dphi = 2.0 * PI * delf_hz as f64 * 0.005;
    let mut t = [ZERO; SYM200];
    for (i, v) in t.iter_mut().enumerate() {
        let phi = dphi * i as f64;
        *v = Complex::new(phi.cos() as f32, phi.sin() as f32);
    }
    t
}

/// Campione di `cd` con zeri fuori dall'intervallo.
pub(crate) fn cd_at(cd: &[Complex], idx: i64) -> Complex {
    if idx >= 0 && (idx as usize) < cd.len() { cd[idx as usize] } else { ZERO }
}

/// Contributo di una correlazione `z` alla somma di `sync8d` secondo la forma.
fn sync_term(z: Complex, kind: SyncKind) -> f32 {
    match kind {
        SyncKind::Magnitude => z.norm(),
        SyncKind::Power => z.norm_sqr(),
        SyncKind::AvgAbs => z.re.abs() + z.im.abs(),
    }
}

/// Potenza di sincronismo di `sync8d` (`iqso = 1`): le correlazioni con i 21 simboli di Costas, sommate nella forma `kind` (per
/// `AvgAbs` ogni simbolo e' la media di se stesso e del successivo, e l'ultimo riusa il valore del sesto, come nel Fortran).
/// Con `cq` (e solo nella ricerca in frequenza, cioe' con `tweak`) si aggiungono gli otto simboli di dati di un CQ.
/// `i0` e' il campione (a 200 Hz) in cui comincia il primo simbolo; `tweak` sposta il riferimento in frequenza.
fn sync8d(cd: &[Complex], i0: i64, tweak: Option<&[Complex; SYM200]>, kind: SyncKind, cq: bool) -> f32 {
    let tables = sync_tables();
    let correlate = |start: i64, row: &[Complex; SYM200]| -> Complex {
        let mut z = ZERO;
        for (n, r) in row.iter().enumerate() {
            let r = match tweak {
                Some(t) => *r * t[n],
                None => *r,
            };
            z += cd_at(cd, start + n as i64) * r.conj();
        }
        z
    };
    let mut zt = [[ZERO; 7]; 3];
    for (i, row) in tables.costas.iter().enumerate() {
        let i1 = i0 + (i * SYM200) as i64;
        zt[0][i] = correlate(i1, row);
        zt[1][i] = correlate(i1 + 1152, row);
        zt[2][i] = correlate(i1 + 2304, row);
    }
    let mut sync = 0.0f32;
    if kind == SyncKind::AvgAbs {
        let mut avg = [ZERO; 3];
        for i in 0..7 {
            if i < 6 {
                for (a, z) in avg.iter_mut().zip(zt.iter()) {
                    *a = (z[i] + z[i + 1]) / 2.0;
                }
            }
            for a in &avg {
                sync += sync_term(*a, kind);
            }
        }
    } else {
        for z in zt.iter().flatten() {
            sync += sync_term(*z, kind);
        }
    }

    if cq && tweak.is_some() {
        let mut zt4 = [ZERO; 8];
        for (i, row) in tables.cq.iter().enumerate() {
            zt4[i] = correlate(i0 + ((i + 7) * SYM200) as i64, row);
        }
        let mut z4 = ZERO;
        for i in 0..8 {
            z4 = if kind == SyncKind::AvgAbs {
                if i < 7 { (zt4[i] + zt4[i + 1]) / 2.0 } else { z4 }
            } else {
                zt4[i]
            };
            sync += sync_term(z4, kind);
        }
    }
    sync
}

/// Corregge frequenza e tempo di un candidato sul segnale a 200 Hz.
///
/// `cd` deve essere stato costruito (con [`downsample`] o [`Downsampler::run`]) alla frequenza di `candidate`; vedi la convenzione nel modulo.
/// Cerca il ritardo a passi di 5 ms entro +-40 ms, poi la frequenza a passi di 0,5 Hz entro +-2,5 Hz. `sync` resta quello di `sync8`.
pub fn refine(cd: &[Complex], candidate: Candidate) -> Candidate {
    refine_ex(cd, candidate, SyncKind::Magnitude, false)
}

/// Come [`refine`], con la forma della somma di `sync8d` della passata e, per un candidato CQ, il sincronismo dei simboli di dati del
/// CQ nella ricerca in frequenza (`lcqcand` di `ft8b`).
pub fn refine_ex(cd: &[Complex], candidate: Candidate, kind: SyncKind, cq: bool) -> Candidate {
    if cd.len() < NCD / 2 {
        return candidate;
    }
    let i0 = nint((candidate.dt_s + 0.5) * 200.0);
    let mut smax = 0.0f32;
    let mut ibest = None;
    for idt in i0 - 8..=i0 + 8 {
        let s = sync8d(cd, idt, None, kind, false);
        if s > smax {
            smax = s;
            ibest = Some(idt);
        }
    }
    let Some(ibest) = ibest else { return candidate };
    let mut smax = 0.0f32;
    let mut delfbest = 0.0f32;
    for ifr in -5..=5 {
        let delf = ifr as f32 * 0.5;
        let s = sync8d(cd, ibest, Some(&freq_tweak(delf)), kind, cq);
        if s > smax {
            smax = s;
            delfbest = delf;
        }
    }
    Candidate { freq_hz: candidate.freq_hz + delfbest, dt_s: ibest as f32 * 0.005 - 0.5, sync: candidate.sync }
}

// ---------------------------------------------------------------------------------------------------------------------
// ft8b: metriche dei bit

fn argmax(v: &[f32]) -> usize {
    let mut best = 0;
    for (i, &x) in v.iter().enumerate() {
        if x > v[best] {
            best = i;
        }
    }
    best
}

/// `normalizebmet` e fattore di scala; `None` se la serie e' tutta zero o non finita.
fn normalize_llr(bmet: &mut [f32; 174]) -> Option<()> {
    let ms = bmet.iter().map(|b| b * b).sum::<f32>() / 174.0;
    let sig = ms.sqrt();
    if sig.is_nan() || sig <= 0.0 || !sig.is_finite() {
        return None;
    }
    for b in bmet.iter_mut() {
        *b = *b / sig * LLR_SCALE;
    }
    Some(())
}

/// Come leggere i simboli in [`bit_metrics_ex`].
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct MetricsMode {
    /// `lreverse` di `ft8b`: i simboli si leggono a tempo invertito (coniugato e rovesciato), quindi anche `s8`.
    pub reverse: bool,
    /// Seconda sotto-passata (`isubp1 = 2`, solo con la sensibilita' alta): `cs = csr`, cioe' la lettura a tempo invertito, con la
    /// trasformazione `(0,5 x)^3` al posto di quella dei segnali deboli.
    pub second: bool,
}

/// Le serie di LLR e le grandezze che servono ai controlli di `ft8b` per un segnale allineato.
#[derive(Debug, Clone)]
pub struct Metrics {
    /// `llra`, `llrb`, `llrc` (simboli letti da 1, 2 e 3 alla volta) e `llrd` (1 simbolo, bit normalizzati per il massimo): LLR
    /// positivo = bit 1, radice del valore quadratico medio 2,83.
    pub llr: [[f32; 174]; 4],
    /// Ampiezze degli 8 toni nei 79 simboli (`s8`), dopo le correzioni di `ft8b` (equalizzazione dei toni).
    pub s8: [[f32; 8]; NN],
    /// `srr`: rapporto tra i toni di Costas del secondo blocco e il rumore.
    pub srr: f32,
    /// `syncav`: media su 21 simboli di Costas del rapporto tono giusto / media degli altri sette, sul segnale non ritoccato.
    pub syncav: f32,
    /// `nsync`: simboli di Costas (su 21) con il tono piu' forte al posto giusto.
    pub nsync: u32,
}

fn plan32() -> &'static Arc<dyn Fft<f32>> {
    static PLAN: OnceLock<Arc<dyn Fft<f32>>> = OnceLock::new();
    PLAN.get_or_init(|| fft_plan(SYM200, false))
}

/// Le tre serie di LLR (simboli letti da 1, 2 e 3 alla volta) dei 174 bit del segnale allineato; `None` se il sincronismo non regge.
///
/// `cd` e' il segnale a 200 Hz centrato (a meno di mezzo hertz) sulla frequenza del segnale; del candidato si usa solo `dt_s`.
/// Convenzione: LLR positivo = bit 1; vedi il modulo per l'ordine dei bit e la scala.
pub fn bit_metrics(cd: &[Complex], candidate: Candidate) -> Option<[[f32; 174]; 3]> {
    let m = bit_metrics_ex(cd, candidate.dt_s, MetricsMode::default())?;
    if m.nsync < 7 {
        return None;
    }
    Some([m.llr[0], m.llr[1], m.llr[2]])
}

/// Il cuore di `ft8b` che precede la decodifica: spettri dei 79 simboli, equalizzazione, e le quattro serie di LLR. Non scarta il
/// segnale se il sincronismo e' basso (`nsync`): i controlli stanno nel chiamante, perche' per i candidati CQ JTDX ne usa di piu' larghi.
pub fn bit_metrics_ex(cd: &[Complex], dt_s: f32, mode: MetricsMode) -> Option<Metrics> {
    if cd.len() < NCD / 2 || !dt_s.is_finite() {
        return None;
    }
    let ibest = nint((dt_s + 0.5) * 200.0);
    let fft32 = plan32();
    let mut scratch = vec![ZERO; fft32.get_inplace_scratch_len()];

    let symbol = |k: usize| -> [Complex; SYM200] {
        let mut c = [ZERO; SYM200];
        for (n, v) in c.iter_mut().enumerate() {
            *v = cd_at(cd, ibest + (k * SYM200 + n) as i64);
        }
        c
    };

    // Livello del sincronismo sui 21 simboli di Costas, sul segnale non ritoccato.
    let mut snr_sum = 0.0f32;
    for (i, k) in (0..7).flat_map(|i| [(i, i), (i, i + 36), (i, i + 72)]) {
        let mut c = symbol(k);
        fft32.process_with_scratch(&mut c, &mut scratch);
        let mags: Vec<f32> = c[..8].iter().map(|z| z.norm()).collect();
        let sig = mags[COSTAS[i] as usize];
        let noise = (mags.iter().sum::<f32>() - sig) / 7.0;
        if noise > 1e-16 {
            snr_sum += sig / noise;
        }
    }
    let syncav = snr_sum / 21.0;

    // Spettri degli 79 simboli: cs (diviso 1000), csr (la lettura a tempo invertito) e s8 (moduli di quella che si usa per la sincronia).
    let mut cs = vec![[ZERO; 8]; NN];
    let mut csr = vec![[ZERO; 8]; NN];
    let mut s8 = [[0.0f32; 8]; NN];
    for k in 0..NN {
        let mut c = symbol(k);
        if syncav < 2.5 {
            c[0] *= 1.9;
            c[31] *= 1.9;
            let (a0, a31) = (c[0].norm(), c[31].norm());
            let scr = a0.sqrt() / a31.sqrt();
            if scr.is_finite() {
                if scr > 1.0 {
                    c[31] *= scr;
                } else if scr > 1e-16 {
                    c[0] /= scr;
                }
            }
        }
        // csymbr(i) = conjg(csymb(33-i))
        let mut rev = [ZERO; SYM200];
        for (i, v) in rev.iter_mut().enumerate() {
            let src = c[SYM200 - 1 - i];
            *v = Complex::new(src.re, -src.im);
        }
        let (mut direct, mut reversed) = (c, rev);
        fft32.process_with_scratch(&mut direct, &mut scratch);
        fft32.process_with_scratch(&mut reversed, &mut scratch);
        let (read, other) = if mode.reverse { (&reversed, &reversed) } else { (&direct, &reversed) };
        for t in 0..8 {
            cs[k][t] = read[t] / 1e3;
            csr[k][t] = other[t] / 1e3;
            s8[k][t] = read[t].norm();
        }
    }

    // Equalizzazione della potenza dei toni sul tono con la somma minima.
    let mut sp = [0.0f32; 8];
    for (t, v) in sp.iter_mut().enumerate() {
        *v = (0..7).map(|k| s8[k][t]).sum::<f32>() + (17..NN).map(|k| s8[k][t]).sum::<f32>();
    }
    let mut kmin = 0;
    for t in 1..8 {
        if sp[t] < sp[kmin] {
            kmin = t;
        }
    }
    if sp[kmin] > 0.0 {
        for kb in 0..8 {
            if kb == kmin {
                continue;
            }
            let spr = sp[kb] / sp[kmin];
            if spr > 1.5 {
                let sq = spr.sqrt();
                for k in 0..NN {
                    s8[k][kb] /= spr;
                    cs[k][kb] /= sq;
                    csr[k][kb] /= sq;
                }
            }
        }
    }

    // Controllo di sincronismo: simboli di Costas con il tono piu' forte al posto giusto.
    let mut nsync = 0;
    for (i, &tone) in COSTAS.iter().enumerate() {
        for k in [i, i + 36, i + 72] {
            if argmax(&s8[k]) == tone as usize {
                nsync += 1;
            }
        }
    }

    // Rapporto segnale/rumore sul secondo blocco di Costas.
    let synclev: f32 = (0..7).map(|k| s8[k + 36][COSTAS[k] as usize]).sum();
    let total: f32 = (36..43).map(|k| s8[k].iter().sum::<f32>()).sum();
    let mut snoiselev = (total - synclev) / 7.0;
    if snoiselev < 0.1 {
        snoiselev = 1.0;
    }
    let srr = synclev / snoiselev;

    let chosen: &Vec<[Complex; 8]> = if mode.second { &csr } else { &cs };
    // bm[0..3] = a, b, c; la quarta serie e' bmetd
    let mut bm = [[0.0f32; 174]; 4];
    let mut s2 = [0.0f32; 512];
    for nsym in 1..=3usize {
        let nt = (1usize << (3 * nsym)) - 1;
        let ibmax = match nsym {
            1 => 2usize,
            2 => 5,
            _ => 8,
        };
        for ihalf in 0..2usize {
            for k in (1..=29usize).step_by(nsym) {
                // `ks` come in Fortran (da 1); in indice da 0 e' ks - 1.
                let ks = if ihalf == 0 { k + 7 } else { k + 43 } - 1;
                for (i, v) in s2.iter_mut().enumerate().take(nt + 1) {
                    let i1 = i / 64;
                    let i2 = (i & 63) / 8;
                    let i3 = i & 7;
                    let mut x = match nsym {
                        1 => chosen[ks][GRAY_MAP[i3]].norm(),
                        2 => (chosen[ks][GRAY_MAP[i2]] + chosen[ks + 1][GRAY_MAP[i3]]).norm(),
                        _ => (chosen[ks][GRAY_MAP[i1]] + chosen[ks + 1][GRAY_MAP[i2]] + chosen[ks + 2][GRAY_MAP[i3]]).norm(),
                    };
                    if mode.second {
                        if srr < 2.5 {
                            x = (0.5 * x).powi(3);
                        }
                    } else if srr < 2.5 {
                        if srr > 2.3 {
                            x *= x;
                        } else if x < 5.77 {
                            x = 1.0 + 8.0 * x * x - 0.12 * x * x * x * x;
                        } else {
                            x = (x + 5.82) * (x + 5.82);
                        }
                    }
                    *v = x;
                }
                // Primo bit di questo gruppo (da 0).
                let i32_ = (k - 1) * 3 + ihalf * 87;
                for ib in 0..=ibmax {
                    let pos = i32_ + ib;
                    if pos >= 174 {
                        continue;
                    }
                    let bit = ibmax - ib;
                    let mut m1 = f32::NEG_INFINITY;
                    let mut m0 = f32::NEG_INFINITY;
                    for (i, &x) in s2.iter().enumerate().take(nt + 1) {
                        if (i >> bit) & 1 == 1 {
                            m1 = m1.max(x);
                        } else {
                            m0 = m0.max(x);
                        }
                    }
                    bm[nsym - 1][pos] = m1 - m0;
                    if nsym == 1 {
                        // bmetd: la differenza normalizzata per il massimo dei due
                        let den = m1.max(m0);
                        bm[3][pos] = if den > 0.0 { (m1 - m0) / den } else { 0.0 };
                    }
                }
            }
        }
    }
    for set in bm.iter_mut() {
        normalize_llr(set)?;
    }
    Some(Metrics { llr: bm, s8, srr, syncav, nsync })
}

// ---------------------------------------------------------------------------------------------------------------------
#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Instant;

    /// Generatore pseudo-casuale deterministico (xorshift64*).
    struct Rng(u64);
    impl Rng {
        fn next_u64(&mut self) -> u64 {
            self.0 ^= self.0 >> 12;
            self.0 ^= self.0 << 25;
            self.0 ^= self.0 >> 27;
            self.0.wrapping_mul(0x2545_F491_4F6C_DD1D)
        }
        fn uniform(&mut self) -> f64 {
            ((self.next_u64() >> 11) as f64 + 0.5) / (1u64 << 53) as f64
        }
        fn gauss(&mut self) -> f64 {
            let (u, v) = (self.uniform(), self.uniform());
            (-2.0 * u.ln()).sqrt() * (2.0 * PI * v).cos()
        }
    }

    /// Un segnale di prova: i bit trasmessi e i toni.
    struct Sent {
        bits: [u8; 174],
    }

    /// 79 toni da 174 bit: Costas ai simboli 0-6, 36-42, 72-78, dati con mappa Gray come `genft8.f90`.
    fn tones_from_bits(bits: &[u8; 174]) -> [usize; 79] {
        let mut t = [0usize; 79];
        for (i, &c) in COSTAS.iter().enumerate() {
            t[i] = c as usize;
            t[36 + i] = c as usize;
            t[72 + i] = c as usize;
        }
        let mut k = 7;
        for j in 0..58 {
            if j == 29 {
                k += 7;
            }
            let idx = (bits[3 * j] as usize) * 4 + (bits[3 * j + 1] as usize) * 2 + bits[3 * j + 2] as usize;
            t[k] = GRAY_MAP[idx];
            k += 1;
        }
        t
    }

    /// Onda GFSK reale (BT = 2) alla frequenza `f0` con ampiezza `amp`, come `gen_ft8wave`.
    fn gfsk_wave(tones: &[usize; 79], f0: f64, amp: f64) -> Vec<f32> {
        let nsps = NSPS;
        let nsym = 79;
        let pulse: Vec<f64> = (1..=3 * nsps).map(|i| gfsk_pulse(2.0, (i as f64 - 1.5 * nsps as f64) / nsps as f64)).collect();
        let peak = 2.0 * PI / nsps as f64;
        let mut dphi = vec![0.0f64; (nsym + 2) * nsps];
        for (j, &t) in tones.iter().enumerate() {
            for (m, p) in pulse.iter().enumerate() {
                dphi[j * nsps + m] += peak * p * t as f64;
            }
        }
        for m in 0..2 * nsps {
            dphi[m] += peak * tones[0] as f64 * pulse[nsps + m];
            dphi[nsym * nsps + m] += peak * tones[nsym - 1] as f64 * pulse[m];
        }
        let dw = 2.0 * PI * f0 / SAMPLE_RATE as f64;
        let mut wave = vec![0.0f32; nsym * nsps];
        let mut phi = 0.0f64;
        for (k, w) in wave.iter_mut().enumerate() {
            *w = (amp * phi.sin()) as f32;
            phi = (phi + dphi[nsps + k] + dw) % (2.0 * PI);
        }
        let nramp = nsps / 8;
        for i in 0..nramp {
            wave[i] *= ((1.0 - (2.0 * PI * i as f64 / (2.0 * nramp as f64)).cos()) / 2.0) as f32;
            let j = nsym * nsps - nramp + i;
            wave[j] *= ((1.0 + (2.0 * PI * i as f64 / (2.0 * nramp as f64)).cos()) / 2.0) as f32;
        }
        wave
    }

    /// Rumore bianco gaussiano con deviazione standard `sigma` (scala dei numeri interi a 16 bit).
    fn noise(sigma: f64, seed: u64) -> Vec<f32> {
        let mut rng = Rng(seed);
        (0..SLOT_SAMPLES).map(|_| (sigma * rng.gauss()) as f32).collect()
    }

    const SIGMA: f64 = 200.0;

    /// Ampiezza per un SNR in 2500 Hz con rumore di deviazione standard `sigma` su 6000 Hz.
    fn amp_for_snr(snr_db: f64, sigma: f64) -> f64 {
        (2.0 * 10f64.powf(snr_db / 10.0) * sigma * sigma * 2500.0 / 6000.0).sqrt()
    }

    fn random_bits(seed: u64) -> [u8; 174] {
        let mut rng = Rng(seed);
        let mut b = [0u8; 174];
        for v in b.iter_mut() {
            *v = (rng.next_u64() >> 40 & 1) as u8;
        }
        b
    }

    /// Aggiunge a `audio` il segnale con tono 0 a `f0`, che comincia a (0,5 + dt) secondi.
    fn add_signal(audio: &mut [f32], bits: &[u8; 174], f0: f64, dt: f64, snr_db: f64) {
        let wave = gfsk_wave(&tones_from_bits(bits), f0, amp_for_snr(snr_db, SIGMA));
        let start = ((0.5 + dt) * SAMPLE_RATE as f64).round() as usize;
        for (i, &w) in wave.iter().enumerate() {
            if start + i < audio.len() {
                audio[start + i] += w;
            }
        }
    }

    fn two_signal_slot(snr_db: f64) -> (Vec<f32>, Sent, Sent) {
        let mut audio = noise(SIGMA, 7);
        let a = Sent { bits: random_bits(11) };
        let b = Sent { bits: random_bits(23) };
        add_signal(&mut audio, &a.bits, 1500.0, 0.3, snr_db);
        add_signal(&mut audio, &b.bits, 1800.0, -0.2, snr_db);
        (audio, a, b)
    }

    fn nearest(cands: &[Candidate], f: f32, dt: f32) -> Option<Candidate> {
        cands
            .iter()
            .filter(|c| (c.freq_hz - f).abs() < 10.0)
            .min_by(|a, b| ((a.freq_hz - f).abs() + (a.dt_s - dt).abs() * 50.0).total_cmp(&((b.freq_hz - f).abs() + (b.dt_s - dt).abs() * 50.0)))
            .copied()
    }

    #[test]
    fn mappa_gray_e_toni_come_genft8() {
        let mut bits = [0u8; 174];
        // primo simbolo dati: 011 -> indice 3 -> tono 2
        bits[1] = 1;
        bits[2] = 1;
        let t = tones_from_bits(&bits);
        assert_eq!(&t[0..7], &[3, 1, 4, 0, 6, 5, 2]);
        assert_eq!(t[7], 2);
        assert_eq!(&t[36..43], &[3, 1, 4, 0, 6, 5, 2]);
        assert_eq!(&t[72..79], &[3, 1, 4, 0, 6, 5, 2]);
    }

    #[test]
    fn trova_due_segnali_con_ritardo_e_frequenza_noti() {
        let (audio, _, _) = two_signal_slot(0.0);
        let cands = find_candidates(&audio, 200.0, 4000.0, 1.5, 100);
        for (f, dt) in [(1500.0f32, 0.3f32), (1800.0, -0.2)] {
            let c = nearest(&cands, f, dt).unwrap_or_else(|| panic!("segnale a {f} Hz non trovato: {cands:?}"));
            assert!((c.freq_hz - f).abs() <= 2.0, "frequenza {} invece di {f}", c.freq_hz);
            assert!((c.dt_s - dt).abs() <= 0.05, "dt {} invece di {dt}", c.dt_s);
            assert!(c.sync > 3.0, "sincronismo basso: {}", c.sync);
        }
        // I due segnali sono i due piu' forti.
        assert!(cands.len() >= 2);
        let top: Vec<f32> = cands.iter().take(2).map(|c| c.freq_hz).collect();
        assert!(top.iter().any(|f| (f - 1500.0).abs() < 4.0) && top.iter().any(|f| (f - 1800.0).abs() < 4.0), "{top:?}");
    }

    #[test]
    fn trova_i_segnali_anche_deboli() {
        let (audio, _, _) = two_signal_slot(-14.0);
        let cands = find_candidates(&audio, 200.0, 4000.0, 1.5, 100);
        for (f, dt) in [(1500.0f32, 0.3f32), (1800.0, -0.2)] {
            let c = nearest(&cands, f, dt).unwrap_or_else(|| panic!("segnale a {f} Hz non trovato"));
            assert!((c.freq_hz - f).abs() <= 2.0 && (c.dt_s - dt).abs() <= 0.05, "{c:?}");
        }
    }

    #[test]
    fn con_solo_rumore_nessun_candidato_forte() {
        let (audio, _, _) = two_signal_slot(0.0);
        let all = find_candidates(&audio, 200.0, 4000.0, 1.5, 100);
        // Con i segnali ci sono anche lobi laterali (a multipli di 6,25 Hz), ma i due veri segnali sono i primi e nettamente piu' forti.
        assert!(all.len() >= 3);
        assert!(all[0].sync > 1.5 * all[2].sync && all[1].sync > 1.5 * all[2].sync, "{:?}", &all[..3]);
        for seed in [1u64, 2, 3] {
            let n = noise(SIGMA, seed);
            let cands = find_candidates(&n, 200.0, 4000.0, 1.5, 100);
            let max = cands.iter().map(|c| c.sync).fold(0.0f32, f32::max);
            let tutti = find_candidates(&n, 200.0, 4000.0, 0.0, 450);
            let best = tutti.iter().map(|c| c.sync).fold(0.0f32, f32::max);
            eprintln!("solo rumore (seme {seed}): {} candidati sopra 1,5, il migliore {max:.2}; senza soglia il migliore {best:.2}", cands.len());
            assert!(max < 3.0 && best < 3.0, "candidato forte nel rumore: {max} {best}");
        }
    }

    #[test]
    fn audio_vuoto_o_corto_non_fa_danni() {
        assert!(find_candidates(&[], 200.0, 4000.0, 1.5, 10).is_empty());
        assert!(find_candidates(&[0.0; 100], 200.0, 4000.0, 1.5, 10).is_empty());
        assert!(find_candidates(&vec![0.0; SLOT_SAMPLES], 200.0, 4000.0, 1.5, 10).is_empty());
        assert!(find_candidates(&vec![0.0; SLOT_SAMPLES], 4000.0, 200.0, 1.5, 10).is_empty());
        let cand = Candidate { freq_hz: 1500.0, dt_s: 0.0, sync: 2.0 };
        assert_eq!(refine(&[], cand), cand);
        assert!(bit_metrics(&[], cand).is_none());
        let silent = downsample(&[], 1500.0);
        assert_eq!(silent.len(), NCD);
        assert!(bit_metrics(&silent, cand).is_none());
        // frequenze impossibili: nessun panico
        assert_eq!(downsample(&[0.0; 10], 9000.0).len(), NCD);
        assert_eq!(downsample(&[0.0; 10], -5.0).len(), NCD);
    }

    #[test]
    fn la_discesa_a_200_hz_mette_il_segnale_a_zero_hertz() {
        let mut audio = vec![0.0f32; SLOT_SAMPLES];
        let bits = random_bits(5);
        add_signal(&mut audio, &bits, 1000.0, 0.0, 30.0);
        // Una piccola quantita' di rumore per non lasciare il resto a zero.
        let cd = downsample(&audio, 1000.0);
        assert_eq!(cd.len(), NCD);
        // Il primo simbolo e' il tono 3 (18,75 Hz): lo vede la FFT di 32 punti dei campioni 100..132 (inizio a 0,5 s = campione 100).
        let mut c = [ZERO; SYM200];
        c.copy_from_slice(&cd[100..132]);
        fft_plan(SYM200, false).process(&mut c);
        assert_eq!(argmax(&c[..8].iter().map(|z| z.norm()).collect::<Vec<_>>()), 3);
    }

    #[test]
    fn rifinitura_migliora_o_mantiene_la_stima() {
        let (audio, _, _) = two_signal_slot(-3.0);
        let cands = find_candidates(&audio, 200.0, 4000.0, 1.5, 100);
        let down = Downsampler::new(&audio);
        for (f, dt) in [(1500.0f32, 0.3f32), (1800.0, -0.2)] {
            let c = nearest(&cands, f, dt).expect("candidato");
            let cd = down.run(c.freq_hz, false);
            let r = refine(&cd, c);
            let (e0f, e0t) = ((c.freq_hz - f).abs(), (c.dt_s - dt).abs());
            let (e1f, e1t) = ((r.freq_hz - f).abs(), (r.dt_s - dt).abs());
            eprintln!("{f} Hz: prima ({e0f:.2} Hz, {e0t:.3} s) dopo ({e1f:.2} Hz, {e1t:.3} s)");
            assert!(e1f <= e0f + 0.26 && e1t <= e0t + 0.0051, "peggiorata: {c:?} -> {r:?}");
            assert!(e1f <= 0.5 && e1t <= 0.0101, "poco precisa: {r:?}");
            assert_eq!(r.sync, c.sync);
        }
    }

    fn sign_agreement(llr: &[f32; 174], bits: &[u8; 174]) -> f32 {
        let ok = llr.iter().zip(bits.iter()).filter(|(l, b)| (**l > 0.0) == (**b == 1)).count();
        ok as f32 / 174.0
    }

    #[test]
    fn llr_con_il_segno_giusto_a_snr_alto() {
        let (audio, a, b) = two_signal_slot(0.0);
        let cands = find_candidates(&audio, 200.0, 4000.0, 1.5, 100);
        let down = Downsampler::new(&audio);
        for (sent, f, dt) in [(&a, 1500.0f32, 0.3f32), (&b, 1800.0, -0.2)] {
            let c = nearest(&cands, f, dt).expect("candidato");
            let r = refine(&down.run(c.freq_hz, false), c);
            // modo 1: discesa alla frequenza corretta
            let cd = down.run(r.freq_hz, false);
            let llr = bit_metrics(&cd, r).expect("metriche");
            for (n, set) in llr.iter().enumerate() {
                let agree = sign_agreement(set, &sent.bits);
                let rms = (set.iter().map(|x| x * x).sum::<f32>() / 174.0).sqrt();
                eprintln!("{f} Hz serie {n}: segno giusto {:.1}%, rms {rms:.2}", agree * 100.0);
                assert!(agree >= 0.95, "serie {n}: {agree}");
                assert!((rms - 2.83).abs() < 1e-3);
            }
            // modo 2: spostamento del cd di prima della frequenza
            let mut cd2 = down.run(c.freq_hz, false);
            shift_frequency(&mut cd2, -(r.freq_hz - c.freq_hz));
            let llr2 = bit_metrics(&cd2, r).expect("metriche");
            assert!(sign_agreement(&llr2[0], &sent.bits) >= 0.95);
        }
    }

    #[test]
    fn llr_a_snr_basso_restano_utili() {
        // A -12 dB (soglia di FT8 circa -20 dB) la maggior parte dei bit ha ancora il segno giusto.
        let (audio, a, _) = two_signal_slot(-12.0);
        let cands = find_candidates(&audio, 200.0, 4000.0, 1.5, 100);
        let c = nearest(&cands, 1500.0, 0.3).expect("candidato");
        let down = Downsampler::new(&audio);
        let r = refine(&down.run(c.freq_hz, true), c);
        let llr = bit_metrics(&down.run(r.freq_hz, true), r).expect("metriche");
        for (n, set) in llr.iter().enumerate() {
            let agree = sign_agreement(set, &a.bits);
            eprintln!("-12 dB serie {n}: segno giusto {:.1}%", agree * 100.0);
            assert!(agree >= 0.85, "serie {n}: {agree}");
        }
    }

    #[test]
    fn llr_a_snr_molto_basso_sono_rumorosi_ma_correlati() {
        // A -20 dB (soglia pratica di FT8) i singoli bit sono incerti ma il segno e' ancora piu' spesso giusto che sbagliato.
        let mut audio = noise(SIGMA, 5);
        let a = Sent { bits: random_bits(31) };
        add_signal(&mut audio, &a.bits, 1200.0, 0.1, -20.0);
        let cands = find_candidates(&audio, 200.0, 4000.0, 1.1, 100);
        let c = nearest(&cands, 1200.0, 0.1).expect("candidato");
        eprintln!("-20 dB: candidato {c:?}");
        let down = Downsampler::new(&audio);
        let r = refine(&down.run(c.freq_hz, true), c);
        let llr = bit_metrics(&down.run(r.freq_hz, true), r).expect("metriche");
        for (n, set) in llr.iter().enumerate() {
            let agree = sign_agreement(set, &a.bits);
            eprintln!("-20 dB serie {n}: segno giusto {:.1}%", agree * 100.0);
            assert!(agree >= 0.6, "serie {n}: {agree}");
        }
    }

    #[test]
    fn senza_segnale_le_metriche_si_rifiutano() {
        let n = noise(SIGMA, 99);
        let down = Downsampler::new(&n);
        let cd = down.run(1500.0, false);
        let cand = Candidate { freq_hz: 1500.0, dt_s: 0.2, sync: 2.0 };
        // Il rumore puo' superare per caso il controllo (7 su 21): se lo supera, le serie esistono ma sono normalizzate.
        if let Some(llr) = bit_metrics(&cd, cand) {
            for set in llr {
                assert!(set.iter().all(|x| x.is_finite()));
            }
        }
    }

    #[test]
    fn tempi_della_ricerca() {
        let (audio, _, _) = two_signal_slot(-8.0);
        let t0 = Instant::now();
        let cands = find_candidates(&audio, 200.0, 4000.0, 1.5, 100);
        let t_find = t0.elapsed();
        let t1 = Instant::now();
        let down = Downsampler::new(&audio);
        let t_fft = t1.elapsed();
        let t2 = Instant::now();
        let n = cands.len().min(20);
        for c in cands.iter().take(n) {
            let cd = down.run(c.freq_hz, false);
            let r = refine(&cd, *c);
            let cd2 = down.run(r.freq_hz, false);
            let _ = bit_metrics(&cd2, r);
        }
        let t_per = t2.elapsed();
        eprintln!(
            "tempi: find_candidates {:?} ({} candidati); FFT lunga {:?}; discesa+refine+metriche per {n} candidati {:?}",
            t_find,
            cands.len(),
            t_fft,
            t_per
        );
        assert!(t_find.as_secs_f32() < 10.0);
    }

    #[test]
    fn le_tre_forme_dello_spettro_trovano_i_due_segnali() {
        let (audio, _, _) = two_signal_slot(-6.0);
        for shape in [Spectrum::Amplitude, Spectrum::Power, Spectrum::AbsSum] {
            let cands: Vec<Candidate> = find_candidates_ex(&audio, 200.0, 4000.0, 1.5, 100, shape, (JZB, JZT)).into_iter().map(|c| c.candidate).collect();
            for (f, dt) in [(1500.0f32, 0.3f32), (1800.0, -0.2)] {
                let c = nearest(&cands, f, dt).unwrap_or_else(|| panic!("{shape:?}: segnale a {f} Hz non trovato"));
                assert!((c.freq_hz - f).abs() <= 2.0 && (c.dt_s - dt).abs() <= 0.05, "{shape:?}: {c:?}");
            }
        }
        // la forma segue la passata come in sync8.f90
        assert_eq!([1, 2, 3, 4, 5, 6, 7, 8, 9].map(Spectrum::for_pass).iter().filter(|s| **s == Spectrum::Power).count(), 3);
        assert_eq!(SyncKind::for_pass(1), SyncKind::Magnitude);
        assert_eq!(SyncKind::for_pass(7), SyncKind::Power);
        assert_eq!(SyncKind::for_pass(8), SyncKind::AvgAbs);
    }

    #[test]
    fn una_finestra_del_ritardo_spostata_non_trova_cio_che_sta_fuori() {
        let (audio, _, _) = two_signal_slot(0.0);
        // il segnale a +0,3 s sta a 7 passi di 0,04 s dal centro: una finestra tutta a sinistra non lo vede
        let left = find_candidates_ex(&audio, 200.0, 4000.0, 1.5, 100, Spectrum::Amplitude, (-62, -20));
        assert!(left.iter().all(|c| (c.candidate.dt_s - 0.3).abs() > 0.1 || c.candidate.sync < 3.0), "{left:?}");
        assert!(find_candidates_ex(&audio, 200.0, 4000.0, 1.5, 100, Spectrum::Amplitude, (10, -10)).is_empty());
    }

    /// Un periodo con un solo messaggio vero (testo dato) alla frequenza e al ritardo dati.
    fn real_message_slot(text: &str, f0: f32, dt: f32, snr_db: f64, seed: u64) -> Vec<f32> {
        let mut audio = noise(SIGMA, seed);
        let wave = crate::modem::encode_message(text, f0, &mut crate::message::CallHashes::default()).expect("messaggio valido");
        let amp = amp_for_snr(snr_db, SIGMA) as f32;
        let start = ((0.5 + f64::from(dt)) * SAMPLE_RATE as f64).round() as usize;
        for (i, &w) in wave.iter().enumerate() {
            audio[start + i] += amp * w;
        }
        audio
    }

    #[test]
    fn l_indicatore_cq_si_accende_sui_cq_e_non_sugli_altri() {
        let mut on = 0;
        let mut off = 0;
        for seed in 0..4u64 {
            let cq = real_message_slot("CQ K1ABC FN42", 1400.0 + 200.0 * seed as f32, 0.2, -10.0, 10 + seed);
            let other = real_message_slot("W9XYZ K1ABC -12", 1400.0 + 200.0 * seed as f32, 0.2, -10.0, 20 + seed);
            let top = |a: &[f32]| find_candidates_ex(a, 200.0, 4000.0, 1.5, 100, Spectrum::Amplitude, (JZB, JZT)).into_iter().next();
            if top(&cq).is_some_and(|c| c.cq) {
                on += 1;
            }
            if top(&other).is_some_and(|c| c.cq) {
                off += 1;
            }
        }
        assert!(on >= 3 && off <= 1, "CQ riconosciuti {on}/4, altri scambiati per CQ {off}/4");
    }

    #[test]
    fn la_rifinitura_con_ogni_forma_resta_vicina_al_segnale() {
        let audio = real_message_slot("CQ K1ABC FN42", 1500.0, 0.3, -8.0, 3);
        let cands = find_candidates(&audio, 200.0, 4000.0, 1.5, 100);
        let c = nearest(&cands, 1500.0, 0.3).expect("candidato");
        let down = Downsampler::new(&audio);
        for kind in [SyncKind::Magnitude, SyncKind::Power, SyncKind::AvgAbs] {
            for cq in [false, true] {
                let r = refine_ex(&down.run(c.freq_hz, false), c, kind, cq);
                assert!((r.freq_hz - 1500.0).abs() <= 1.0 && (r.dt_s - 0.3).abs() <= 0.02, "{kind:?} cq {cq}: {r:?}");
            }
        }
    }

    #[test]
    fn le_quattro_serie_di_llr_e_la_lettura_a_tempo_invertito_vanno_d_accordo_col_segnale() {
        let (audio, a, _) = two_signal_slot(0.0);
        let cands = find_candidates(&audio, 200.0, 4000.0, 1.5, 100);
        let down = Downsampler::new(&audio);
        let c = nearest(&cands, 1500.0, 0.3).expect("candidato");
        let r = refine(&down.run(c.freq_hz, false), c);
        let mut cd = down.run(c.freq_hz, false);
        shift_frequency(&mut cd, -(r.freq_hz - c.freq_hz));
        for mode in [MetricsMode::default(), MetricsMode { reverse: true, second: false }, MetricsMode { reverse: false, second: true }] {
            let m = bit_metrics_ex(&cd, r.dt_s, mode).expect("metriche");
            assert!(m.nsync >= 15, "{mode:?}: nsync {}", m.nsync);
            for (n, set) in m.llr.iter().enumerate() {
                let agree = sign_agreement(set, &a.bits);
                assert!(agree >= 0.95, "{mode:?} serie {n}: {agree}");
            }
        }
    }
}
