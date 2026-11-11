//! JT65: ricezione. Sincronismo sul periodo intero, affinamento di frequenza e ritardo, energie dei 64 toni di ogni simbolo e decodifica
//! Reed-Solomon a decisione soft con l'analisi a priori (`symspec65`, `sync65`, `demod64a`, `extract.f90`, `ftrsdap.c`, `hint65.f90` e
//! `jt65_decode.f90` di WSJT-X 3.3, GPL 3). I sottomodi B e C (toni a distanza doppia e quadrupla), la lisciatura degli spettri alla
//! larghezza del segnale e la media dei periodi (`decode65a.f90`, `lorentzian.f90`, `avg65` di `jt65_decode.f90`) servono all'EME e alle VHF.

pub use crate::jt65::Submode;
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
/// Distanza tra i toni del sottomodo A in hertz (un bin dello spettro di un simbolo, `df = 1378.125 / 512` di `decode65a.f90:69`).
const TONE_DF: f64 = 11_025.0 / 4096.0;
/// Bin in piu' attorno ai 66 toni negli spettri dei simboli: la lisciatura `smo121` non tocca gli estremi e li si tiene lontani dai toni.
const MARGIN: usize = 64;
/// Al massimo quante lisciature si provano per candidato (in WSJT-X fino a `2 * nint(width / df)`, senza tetto): un segnale largo
/// 30 Hz ne chiede gia' 22; il tetto evita minuti di calcolo con una larghezza stimata sul rumore.
const MAX_SMOOTHING: i32 = 24;
/// Periodi tenuti per la media (`MAXAVE = 64` di `avg65`, `jt65_decode.f90:528`).
pub const MAX_AVERAGE: usize = 64;
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
    /// Quanti periodi sono serviti: 1 per un periodo solo, da 2 in su per un messaggio uscito dalla media (`nsum` di `avg65`).
    pub averaged: u32,
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
    /// Sottomodo A, B o C: la distanza fra i toni di dati (`mode65 = 2**nsubmode`, `jt65_decode.f90:301`).
    pub submode: Submode,
    /// «Enable VHF and submode features» (`bVHF`, `nexp_decode & 64`): un solo candidato, il piu' forte entro `rx_hz +- tol_hz` con F Tol
    /// sotto i 1000 Hz (`jt65_decode.f90:286-290`, `sync65.f90:62-70`), sincronismo in dB, larghezza del segnale (`lorentzian`) e lisciatura
    /// degli spettri in B e C (`decode65a.f90:98-139`), due passate, niente filtri dell'AFC e del livello «Aggressive». Spento di fabbrica.
    pub vhf: bool,
    /// Soglia del sincronismo («Sync», `minsync`, di fabbrica 0, `mainwindow_settings.cpp:508`): un candidato piu' debole non si decodifica
    /// e non entra nella media (`jt65_decode.f90:347`, `:416`). In VHF il sincronismo e' in dB, altrimenti in deviazioni standard.
    pub min_sync: i32,
    /// «Include averaging» (`ndepth & 16`): un candidato che da solo non si decodifica si somma ai periodi della stessa parita', della stessa
    /// frequenza entro F Tol e dello stesso ritardo entro 0,2 s e si decodifica la media (`jt65_decode.f90:413-440`, `avg65`). Serve
    /// [`decode_averaged`]. Spento di fabbrica.
    pub average: bool,
    /// «Auto Clear Avg» (`ndepth & 128`): dopo un messaggio dalla media entro F Tol dalla frequenza di ricezione la media si svuota
    /// (`jt65_decode.f90:432-433`). Spento di fabbrica.
    pub auto_clear_avg: bool,
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
            submode: Submode::A,
            vhf: false,
            min_sync: 0,
            average: false,
            auto_clear_avg: false,
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

/// `db` di WSJT-X: -99 per valori non positivi.
fn db(x: f32) -> f32 {
    if x > 1.0e-10 { 10.0 * x.log10() } else { -99.0 }
}

/// Da Aurora alle unita' della correlazione di WSJT-X: qui gli spettri si dividono per il 40 per cento dei valori di ogni frequenza
/// (0,511 volte la media del rumore), in `symspec65.f90:63-74` per `4 * pctile(28)` di `flat65.f90` (1,314 volte la media): 0,511 / 1,314.
const VHF_CCF_SCALE: f32 = 0.389;

#[derive(Debug, Clone, Copy)]
struct Candidate {
    bin: usize,
    lag: i32,
    sync: f32,
}

/// Cerca i segnali: per ogni frequenza correla l'energia del tono di sincronismo con la sequenza pseudocasuale (`sync65`). I candidati
/// escono in ordine di frequenza, come in `sync65.f90:62-97` (chi li tronca a 50 tiene i piu' bassi, `jt65_decode.f90:324`). `robust`
/// e' `nrobust` di `xcor.f90:29-40`: ogni valore diventa +1 o -1 rispetto alla mediana (contro i picchi dell'AGC a inizio periodo).
/// Con `vhf` (`sync65.f90:46-47`, `:62-66`, `:89-90`) la correlazione non si raddrizza, esce il solo picco piu' alto della finestra (se
/// supera la soglia) e il sincronismo e' in dB (`db(ccfred) - 16`).
fn find_candidates(ss: &[f32], nq: usize, f_low_hz: f32, f_high_hz: f32, thresh: f32, robust: bool, vhf: bool) -> Vec<Candidate> {
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
    let (mut ccfmax, mut ipk) = (0.0f32, 0usize);
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
        if !vhf {
            slope_normalize(&mut ccf, f64::from(lag_best - LAG1) + 1.0);
        }
        ccfred[i] = ccf[(lag_best - LAG1) as usize];
        peaks[i] = lag_best;
        if ccfred[i] > ccfmax {
            ccfmax = ccfred[i];
            ipk = i;
        }
    }
    let xmed = percentile(&ccfred[ia..=ib], 35);
    for v in ccfred[ia..=ib].iter_mut() {
        *v -= xmed;
    }
    ccfred[ia - 1] = ccfred[ia];
    ccfred[ib + 1] = ccfred[ib];
    let mut out: Vec<Candidate> = Vec::new();
    if vhf {
        // `ccfmax` e' quello prima di togliere la mediana (`sync65.f90:51-54`, `:64`)
        if ipk >= ia && ccfmax >= thresh {
            out.push(Candidate { bin: ipk, lag: peaks[ipk], sync: db(ccfred[ipk] * VHF_CCF_SCALE) - 16.0 });
        }
        return out;
    }
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

/// Gli spettri dei 63 simboli di dati attorno al segnale (`s1` di `decode65a.f90:66-86`, un bin = 2,69 Hz = la distanza dei toni di A):
/// `rows[j][MARGIN + b]` e' il bin `b` sopra il tono di sincronismo, da `-MARGIN` a `65 * mode65 + MARGIN`, gia' corretti di frequenza,
/// ritardo e deriva. Il tono di dati `i` sta al bin `mode65 * (i + 2)` (`decode65a.f90:126-133`: in C WSJT-X ricampiona una seconda volta a
/// `f0 + 355,3 Hz` solo perche' a 1378 Hz di campionamento i toni alti non ci starebbero; qui lo spettro e' quello a 12 kHz e non serve).
#[derive(Debug, Clone)]
struct BinSpectra {
    mode65: usize,
    rows: Vec<Vec<f32>>,
}

impl BinSpectra {
    fn bins(mode65: usize) -> usize {
        65 * mode65 + 2 * MARGIN + 1
    }

    /// Le energie dei 64 toni di dati (`s3` di `decode65b.f90:15-21`).
    fn s3(&self) -> Vec<[f32; 64]> {
        self.rows.iter().map(|row| std::array::from_fn(|i| row[MARGIN + self.mode65 * (i + 2)])).collect()
    }

    /// Una lisciatura `smo121` di ogni simbolo.
    fn smooth(&mut self) {
        for row in self.rows.iter_mut() {
            smo121(row);
        }
    }
}

/// `smo121.f90`: media pesata 1/4, 1/2, 1/4 con il vicino, gli estremi restano.
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

/// Gli spettri dei simboli di dati per il sottomodo `mode65` (1, 2, 4), con la stessa finestra di un simbolo di `decode65a.f90:71-86`.
fn bin_spectra(samples: &[f32], f: f64, dt_samples: f64, drift: f64, mode65: usize) -> BinSpectra {
    let mut planner = FftPlanner::<f32>::new();
    let fft = planner.plan_fft_forward(WINDOW);
    let mut scratch = vec![Complex::new(0.0f32, 0.0); fft.get_inplace_scratch_len()];
    let nb = BinSpectra::bins(mode65);
    let mut rows = Vec::with_capacity(NN);
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
        // i bin sotto lo zero stanno in fondo alla trasformata
        let row: Vec<f32> = (0..nb).map(|b| buf[(b + WINDOW - MARGIN) % WINDOW].norm_sqr()).collect();
        rows.push(row);
    }
    BinSpectra { mode65, rows }
}

/// Lo spettro medio del periodo diviso per il fondo (`savg` di `symspec65.f90:59-81` con il riferimento di `flat65.f90`: per ogni frequenza
/// il 28 per cento dei valori, poi il 28 per cento su 33 frequenze vicine, per 4).
fn average_spectrum(ss: &[f32], nq: usize) -> Vec<f32> {
    if nq == 0 {
        return vec![0.0; NSZ];
    }
    let stmp: Vec<f32> = (0..NSZ).map(|i| percentile(&(0..nq).map(|j| ss[j * NSZ + i]).collect::<Vec<f32>>(), 28)).collect();
    let nsmo = 33usize;
    let (ia, ib) = (nsmo / 2, NSZ - nsmo / 2 - 2);
    let mut reference = vec![0.0f32; NSZ];
    for i in ia..=ib {
        reference[i] = 4.0 * percentile(&stmp[i - nsmo / 2..i - nsmo / 2 + nsmo], 28);
    }
    let (first, last) = (reference[ia], reference[ib]);
    reference[..ia].fill(first);
    reference[ib + 1..].fill(last);
    (0..NSZ)
        .map(|i| {
            if reference[i] <= 0.0 {
                return 0.0;
            }
            (0..nq).map(|j| ss[j * NSZ + i]).sum::<f32>() / nq as f32 / reference[i]
        })
        .collect()
}

/// `fchisq0.f90`: lo scarto quadratico da una lorentziana troncata a tre semilarghezze.
fn fchisq0(y: &[f32], a: &[f32; 5]) -> f32 {
    let mut chisq = 0.0f32;
    for (i, &v) in y.iter().enumerate() {
        let z = ((i + 1) as f32 - a[2]) / (0.5 * a[3]);
        let mut yfit = a[0];
        if z.abs() < 3.0 {
            yfit = a[0] + a[1] * (1.0 / (1.0 + z * z) - 0.1);
        }
        chisq += (v - yfit) * (v - yfit);
    }
    chisq
}

/// `lorentzian.f90`: fondo, ampiezza, centro (da 1), larghezza a meta' altezza (in bin) e scarto della lorentziana che meglio segue `y`.
/// Ne serve la larghezza: quanto il segnale e' allargato (dall'EME, dallo scatter) e quindi quanto lisciare gli spettri.
fn lorentzian(y: &[f32]) -> [f32; 5] {
    let mut a = [0.0f32; 5];
    let npts = y.len();
    if npts < 40 {
        return a;
    }
    let (mut ipk, mut ymax) = (0usize, f32::MIN);
    for (i, &v) in y.iter().enumerate() {
        if v > ymax {
            ymax = v;
            ipk = i;
        }
    }
    let base = (y[..20].iter().sum::<f32>() + y[npts - 20..].iter().sum::<f32>()) / 40.0;
    let stest = ymax - 0.5 * (ymax - base);
    let mut ssum = y[ipk];
    for i in 1..=50 {
        if ipk + i >= npts || y[ipk + i] < stest {
            break;
        }
        ssum += y[ipk + i];
    }
    for i in 1..=50 {
        if i > ipk || y[ipk - i] < stest {
            break;
        }
        ssum += y[ipk - i];
    }
    let ww = if y[ipk] != 0.0 { ssum / y[ipk] } else { 0.0 };
    let t = ww * ww - 5.67;
    a[0] = base;
    a[1] = ymax - base;
    a[2] = (ipk + 1) as f32;
    a[3] = if t > 0.0 { t.sqrt() } else { 2.0 };
    // la ricerca a passi di `lorentzian.f90:52-104`, con un tetto ai passi (in Fortran i cicli `go to 10` e `go to 20` non ne hanno)
    let mut deltaa = [0.1f32, 0.1, 1.0, 1.0];
    let mut chisqr0 = 1.0e6f32;
    for _iter in 0..5 {
        for j in 0..4 {
            let mut chisq1 = fchisq0(y, &a);
            let mut fnn = 0.0f32;
            let mut delta = deltaa[j];
            let mut chisq2;
            let mut guard = 0;
            loop {
                a[j] += delta;
                chisq2 = fchisq0(y, &a);
                guard += 1;
                if chisq2 != chisq1 || guard > 200 {
                    break;
                }
            }
            if chisq2 > chisq1 {
                delta = -delta;
                a[j] += delta;
                std::mem::swap(&mut chisq1, &mut chisq2);
            }
            let mut chisq3;
            let mut guard = 0;
            loop {
                fnn += 1.0;
                a[j] += delta;
                chisq3 = fchisq0(y, &a);
                guard += 1;
                if chisq3 < chisq2 && guard < 1000 {
                    chisq1 = chisq2;
                    chisq2 = chisq3;
                } else {
                    break;
                }
            }
            if chisq3 == chisq2 {
                delta *= 0.5;
            } else {
                delta *= 1.0 / (1.0 + (chisq1 - chisq2) / (chisq3 - chisq2)) + 0.5;
            }
            a[j] -= delta;
            deltaa[j] = deltaa[j] * fnn / 3.0;
        }
        let chisqr = fchisq0(y, &a);
        a[4] = chisqr;
        if chisqr / chisqr0 > 0.99 {
            break;
        }
        chisqr0 = chisqr;
    }
    a
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
/// supera il secondo (`hint65.f90:70-123`, `nflip = 1`). `nadd` e' quanti spettri sono stati sommati o lisciati: da 4 e da 8 il margine
/// richiesto sul secondo scende (`hint65.f90:118-120`).
fn hint_search<'a>(s3: &[[f32; 64]], mrs: &[u8; NN], mrs2: &[u8; NN], hints: &'a [Hint], nadd: i32) -> Option<(&'a Hint, f32)> {
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
    let bias = if nadd >= 8 {
        (1.04 * u2).max(0.60)
    } else if nadd >= 4 {
        (1.08 * u2).max(0.45)
    } else {
        (1.12 * u2).max(0.35)
    };
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
    /// `qual` di `hint65` per i messaggi della lista (0 per gli altri).
    qual: f32,
}

/// Come si prova un insieme di spettri: `nadd` per `hint65`, `smoothed` se lisciato o sommato su piu' periodi.
#[derive(Debug, Clone, Copy)]
struct Trial {
    trials: u32,
    nadd: i32,
    smoothed: bool,
}

/// `extract.f90`: normalizza le energie, toglie i toni fissi (birdie), prova la decisione soft senza e poi con i tipi a priori del punto del
/// QSO; senza esito, e con la ricerca profonda accesa, prova la lista dei messaggi.
fn extract(codec: &Codec, s3: &[[f32; 64]], options: &Options, ap: &ApTable, hints: &[Hint], trial: Trial) -> Option<Extracted> {
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
    // il controllo dell'energia dei toni (`pp`, di Aurora) misura i toni in unita' di rumore mediano: per il rumore esponenziale di un periodo
    // (deviazione = media = mediana / ln 2) la soglia 3,2 vuol dire toni 1,22 deviazioni sopra la media. Uno spettro lisciato o sommato non
    // e' piu' esponenziale (la deviazione scende molto piu' della media, e la lisciatura spalma il tono sui vicini): lo stesso controllo si fa
    // allora in deviazioni, `(1 + z) / ln 2`, che sul rumore di un periodo da' lo stesso valore
    let pp_of = |tone_mean: f32| -> f32 {
        if !trial.smoothed {
            return tone_mean;
        }
        let n = all.len() as f32;
        let mean = all.iter().sum::<f32>() / n;
        let std = (all.iter().map(|v| (v - mean) * (v - mean)).sum::<f32>() / n).sqrt().max(1e-30);
        (1.0 + (tone_mean * base - mean) / std) / std::f32::consts::LN_2
    };
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
        let Some(res) = soft_decode(codec, &d, &s3a, trial.trials, &known) else { continue };
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
        let pp = pp_of(channel.iter().enumerate().map(|(j, &t)| s3a[j][usize::from(t)]).sum::<f32>() / NN as f32);
        return Some(Extracted { text, channel, nhard: res.nhard, ntotal: res.ntotal, rtt: res.rtt, ap_type: ntype as u8, hinted: false, pp, qual: 0.0 });
    }
    if options.deep_search && !hints.is_empty() {
        let qmin = 2.0 - 0.1 * f32::from(options.aggressive.min(10));
        let (hint, qual) = hint_search(&s3w, &d.mrsym, &d.mr2sym, hints, trial.nadd)?;
        if qual >= qmin {
            let pp = pp_of(hint.symbols.iter().enumerate().map(|(j, &t)| s3a[j][usize::from(t)]).sum::<f32>() / NN as f32);
            return Some(Extracted { text: hint.text.clone(), channel: hint.symbols, nhard: 0, ntotal: 0, rtt: 0.0, ap_type: 0, hinted: true, pp, qual });
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
fn tone_fit(audio: &[f32], tones: &[u8; NSYM], f: f64, dt_samples: f64, drift: f64, spacing: f64) -> f64 {
    let mut total = 0.0f64;
    for (k, &tone) in tones.iter().enumerate() {
        let (re, im, count) = demodulate(audio, symbol_start(dt_samples, k), symbol_freq(f, drift, k) + f64::from(tone) * spacing);
        if count >= WINDOW / 2 {
            total += re * re + im * im;
        }
    }
    total
}

/// `subtract65`: toglie dall'audio un segnale decodificato. Prima si rifinisce frequenza e ritardo sul messaggio noto (un errore di un decimo di
/// hertz lascerebbe un residuo di -18 dB), poi per ogni simbolo si stima ampiezza e fase del suo tono e si sottrae l'onda ricostruita; senza questo
/// un segnale forte lascia ai vicini tanta energia da far «decodificare» messaggi inesistenti.
fn subtract(audio: &mut [f32], channel: &[u8; NN], f0: f64, dt0: f64, drift: f64, spacing: f64) {
    let tones = all_tones(channel);
    let (mut f, mut dt) = (f0, dt0);
    let mut best = tone_fit(audio, &tones, f, dt, drift, spacing);
    for d in -6i32..=6 {
        let ff = f0 + f64::from(d) * 0.04;
        let fit = tone_fit(audio, &tones, ff, dt, drift, spacing);
        if fit > best {
            best = fit;
            f = ff;
        }
    }
    let dt_center = dt0;
    for d in -6i32..=6 {
        let t = dt_center + f64::from(d) * 8.0;
        let fit = tone_fit(audio, &tones, f, t, drift, spacing);
        if fit > best {
            best = fit;
            dt = t;
        }
    }
    for (k, &tone) in tones.iter().enumerate() {
        let freq = symbol_freq(f, drift, k) + f64::from(tone) * spacing;
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
/// prima di provare il candidato successivo (`subtract65`). In VHF ([`Options::vhf`]) due passate con 1000 prove (`ntrials`, «Random
/// erasure patterns» 6 di fabbrica, `Configuration.cpp:3718`; `TwoPass` acceso, `:3901`). Senza la media dei periodi: per quella
/// [`decode_averaged`].
pub fn decode(samples: &[f32], options: &Options) -> Vec<Decoded> {
    decode_inner(samples, options, None)
}

/// Come [`decode`], con la media dei periodi quando [`Options::average`] e' acceso (`avg65`). `period` e' il numero del periodo (per esempio i
/// minuti dall'epoca Unix): si sommano solo periodi della stessa parita' (`mod(iutc(i),2)`, `jt65_decode.f90:584`) e due chiamate con lo stesso
/// numero sono lo stesso periodo (una ridecodifica non lo aggiunge due volte). La media si svuota da sola cambiando sottomodo o VHF
/// (`jt65_decode.f90:258-259`).
pub fn decode_averaged(samples: &[f32], options: &Options, averager: &mut Averager, period: u64) -> Vec<Decoded> {
    averager.check_profile(options.submode, options.vhf);
    decode_inner(samples, options, Some((averager, period)))
}

fn decode_inner(samples: &[f32], options: &Options, mut avg: Option<(&mut Averager, u64)>) -> Vec<Decoded> {
    let codec = Codec::new();
    let mut audio = samples.to_vec();
    let mut out: Vec<Decoded> = Vec::new();
    let (npass, trials) = if options.vhf {
        (2usize, 1000u32)
    } else {
        match options.depth {
            0 | 1 => (2, 100),
            2 => (2, 1000),
            _ => (4, 1000),
        }
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
        // «Single decode», e in VHF con F Tol sotto i 1000 Hz: solo attorno alla frequenza di ricezione, con soglia 1,0 (`jt65_decode.f90:286-290`)
        if (options.single_decode || (options.vhf && options.tol_hz < 1000.0))
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
        // in VHF la larghezza del segnale piu' forte della finestra, da 100 Hz sotto a 100 Hz sopra (`jt65_decode.f90:292-302`)
        let width_hz = if options.vhf {
            let ia = ((f64::from(fa) - 100.0) / DF_BIN).round().max(1.0) as usize;
            let ib = (((f64::from(fb) + 100.0) / DF_BIN).round() as usize).min(NSZ);
            if ib + 1 < ia + 50 {
                break;
            }
            let savg = average_spectrum(&ss, nq);
            lorentzian(&savg[ia - 1..ib])[3] * DF_BIN as f32
        } else {
            0.0
        };
        let mut candidates = find_candidates(&ss, nq, fa, fb, thresh, robust, options.vhf);
        drop(ss);
        candidates.truncate(options.max_candidates / ipass);
        let pass = Pass { ipass, subtract, trials, width_hz };
        if decode_candidates(&codec, &mut audio, &candidates, options, (&ap, &hints), &pass, &mut out, &mut avg) {
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
    /// Larghezza del segnale in hertz (`width`, solo in VHF; 0 altrimenti).
    width_hz: f32,
}

/// Quante lisciature provare (`minsmo`, `maxsmo` di `decode65a.f90:98-104`): solo in B e C, da `nint(width / df) - 1` a `2 * nint(width / df)`.
/// Con larghezza nulla (fuori dalla VHF) WSJT-X prova due volte gli stessi spettri non lisciati (-1 e 0): qui una volta sola.
fn smoothing_range(mode65: usize, width_hz: f32) -> (i32, i32) {
    let w = (width_hz / TONE_DF as f32).round() as i32;
    if mode65 < 2 || 2 * w <= 0 { (0, 0) } else { (w - 1, 2 * w) }
}

/// Il rapporto segnale/rumore in VHF dal sincronismo in dB e dalla larghezza (`jt65_decode.f90:395-398`), fra -30 e -1 dB (`:402-403`).
fn vhf_snr(sync_db: f32, width_hz: f32) -> i32 {
    let s2db = 1.1 * db(10f32.powf((sync_db + 16.0) / 10.0)) + 1.4 * (db(width_hz) - 4.3) - 52.0;
    (s2db.round() as i32).clamp(-30, -1)
}

/// La lisciatura degli spettri (`decode65a.f90:105-157`, `avg65` in `jt65_decode.f90:616-646`): per ogni `ismo` da `minsmo` a `maxsmo` si liscia
/// una volta (due da quattro lisciature in poi) e si prova `extract`; il primo messaggio della decodifica normale chiude, altrimenti vince quello
/// della lista con `qual` piu' alto e ci si ferma quando `qual` scende. `nsum` e' quanti periodi sono sommati (`nadd = nsum * ismo`). Gli
/// spettri restano lisciati (sono quelli che `avg65` mette da parte, come il modulo `s1` di WSJT-X). In WSJT-X `qual` di un tentativo fallito
/// e' `pp2` dell'ultima prova di `ftrsdap`, qui non c'e' e un tentativo fallito vale 0: si provano tutte le lisciature.
#[allow(clippy::too_many_arguments)]
fn extract_smoothed(
    codec: &Codec,
    spectra: &mut BinSpectra,
    (minsmo, maxsmo): (i32, i32),
    nsum: i32,
    options: &Options,
    (ap, hints): (&ApTable, &[Hint]),
    trials: u32,
) -> Option<Extracted> {
    let maxsmo = maxsmo.min(minsmo.max(0) + MAX_SMOOTHING);
    let mut nn = 0;
    let mut best: Option<Extracted> = None;
    let mut qual0 = f32::MIN;
    for ismo in minsmo..=maxsmo {
        if ismo > 0 {
            spectra.smooth();
            nn += 1;
            if nn >= 4 {
                spectra.smooth();
                nn += 1;
            }
        }
        let trial = Trial { trials, nadd: nsum * ismo, smoothed: nn > 0 || nsum > 1 };
        let qual = match extract(codec, &spectra.s3(), options, ap, hints, trial) {
            Some(x) if !x.hinted => return Some(x),
            Some(x) => {
                let q = x.qual;
                if best.as_ref().is_none_or(|b| q > b.qual) {
                    best = Some(x);
                }
                q
            }
            None => 0.0,
        };
        if qual < qual0 {
            break;
        }
        qual0 = qual;
    }
    best
}

/// Prova i candidati di una passata e toglie dall'audio ogni messaggio trovato. Ritorna `true` quando «Single decode» ha il suo messaggio.
#[allow(clippy::too_many_arguments)]
fn decode_candidates(
    codec: &Codec,
    audio: &mut [f32],
    candidates: &[Candidate],
    options: &Options,
    lists: (&ApTable, &[Hint]),
    pass: &Pass,
    out: &mut Vec<Decoded>,
    avg: &mut Option<(&mut Averager, u64)>,
) -> bool {
    // `decoded0`, `freq0`: l'ultimo messaggio della passata, per non ripeterlo da un candidato vicino (`jt65_decode.f90:451-452`)
    let mut last: Option<(String, f64)> = None;
    // `prtavg`: al massimo un messaggio dalla media per passata (`jt65_decode.f90:416`, `:434`)
    let mut printed_avg = false;
    let mode65 = options.submode.mode65() as usize;
    let spacing = TONE_DF * mode65 as f64;
    for c in candidates {
        // sotto la soglia «Sync» il candidato non si decodifica (`nflip = 0`, `jt65_decode.f90:347`, `decode65a.f90:48`) ne' entra nella media
        if c.sync < options.min_sync as f32 {
            continue;
        }
        let bin_hz = c.bin as f64 * DF_BIN;
        let (f, dt, drift) = refine(audio, c);
        // il filtro sulle correzioni dell'AFC (`jt65_decode.f90:358-365`, non in VHF): `a(1)` scarto di frequenza, `a(2)` meta' della deriva,
        // `dtx` ritardo dall'inizio dell'audio
        let (a1, a2, dtx) = (f - bin_hz, drift / 2.0, dt / FS);
        if !options.vhf {
            if a1.abs() > 10.0 / pass.ipass as f64 {
                continue;
            }
            let ibad = usize::from(a1.abs() > 5.0) + usize::from(a2.abs() > 2.0) + usize::from((dtx - 1.0).abs() > 2.5);
            if ibad >= 2 {
                continue;
            }
        }
        let mut spectra = bin_spectra(audio, f, dt, drift, mode65);
        if spectra.rows.len() != NN {
            continue;
        }
        let s3 = spectra.s3();
        let snr = |channel: &[u8; NN]| if options.vhf { vhf_snr(c.sync, pass.width_hz) } else { estimate_snr(&s3, channel) };
        let single = extract_smoothed(codec, &mut spectra, smoothing_range(mode65, pass.width_hz), 1, options, lists, pass.trials);
        // da solo non esce (o esce solo dalla lista): con «Include averaging» si prova la media dei periodi (`jt65_decode.f90:413-440`)
        if options.average
            && !printed_avg
            && single.as_ref().is_none_or(|x| x.hinted)
            && let Some((averager, period)) = avg.as_mut()
        {
            let nfreq = f.round() as i32;
            if averager.wants(*period, nfreq, options.tol_hz)
                && let Some((x, nsum)) = averager.add_and_decode(codec, *period, (nfreq, dtx as f32), &spectra, options, lists, pass)
                && (x.hinted || x.pp >= MIN_TONE_ENERGY)
            {
                printed_avg = true;
                // «Auto Clear Avg»: un messaggio dalla media vicino alla frequenza di ricezione svuota la media (`jt65_decode.f90:432-433`)
                if options.auto_clear_avg && options.rx_hz.is_some_and(|rx| (f as f32 - rx).abs() <= options.tol_hz) {
                    averager.clear();
                }
                if !out.iter().any(|o| o.text == x.text) {
                    out.push(Decoded {
                        snr_db: snr(&x.channel),
                        text: x.text,
                        freq_hz: f as f32,
                        dt_s: (dtx - 1.0) as f32,
                        sync: c.sync,
                        hard_errors: x.nhard.max(0) as u32,
                        total_distance: x.ntotal,
                        rtt: x.rtt,
                        ap_type: x.ap_type,
                        hinted: x.hinted,
                        averaged: nsum,
                    });
                }
            }
        }
        let Some(x) = single else { continue };
        // la seconda accettazione, per livello «Aggressive», solo senza analisi a priori e senza lista (`nft < 2`) e non in VHF (`:441-448`)
        if !options.vhf && x.ap_type == 0 && !x.hinted && !passes_aggressive_level(x.nhard, x.ntotal, x.rtt, options.aggressive) {
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
            subtract(audio, &x.channel, f, dt, drift, spacing);
        }
        last = Some((x.text.clone(), bin_hz));
        if out.iter().any(|o| o.text == x.text) {
            continue;
        }
        out.push(Decoded {
            snr_db: snr(&x.channel),
            text: x.text,
            freq_hz: f as f32,
            dt_s: (dtx - 1.0) as f32,
            sync: c.sync,
            hard_errors: x.nhard.max(0) as u32,
            total_distance: x.ntotal,
            rtt: x.rtt,
            ap_type: x.ap_type,
            hinted: x.hinted,
            averaged: 1,
        });
        if options.single_decode {
            return true;
        }
    }
    false
}

/// Un periodo messo da parte per la media (`iutc`, `nfsave`, `dtsave`, `s1save` di `avg65`).
#[derive(Debug, Clone)]
struct SavedPeriod {
    period: u64,
    freq: i32,
    dt_s: f32,
    spectra: BinSpectra,
}

/// La media dei periodi di JT65 (`avg65` di `jt65_decode.f90:497-680`): fino a 64 periodi, gli spettri dei candidati che da soli non si
/// decodificano. Si tiene fra un periodo e l'altro; [`Averager::clear`] e' «Clear Avg».
#[derive(Debug, Clone, Default)]
pub struct Averager {
    saved: Vec<SavedPeriod>,
    /// Dove va il prossimo periodo quando i 64 posti sono pieni (il piu' vecchio).
    next: usize,
    /// `nutc0`, `nfreq0`: periodo e frequenza dell'ultimo candidato mandato alla media (`jt65_decode.f90:417-424`).
    last: Option<(u64, i32)>,
    /// Sottomodo e VHF con cui sono stati presi gli spettri.
    profile: Option<(Submode, bool)>,
}

impl Averager {
    pub fn new() -> Averager {
        Averager::default()
    }

    /// «Clear Avg» (`clear_jt65_averages` di `jt65_mod.f90:58-68`).
    pub fn clear(&mut self) {
        self.saved.clear();
        self.next = 0;
        self.last = None;
    }

    /// Quanti periodi sono messi da parte.
    pub fn len(&self) -> usize {
        self.saved.len()
    }

    pub fn is_empty(&self) -> bool {
        self.saved.is_empty()
    }

    fn check_profile(&mut self, submode: Submode, vhf: bool) {
        if self.profile != Some((submode, vhf)) {
            self.clear();
            self.profile = Some((submode, vhf));
        }
    }

    /// Un altro periodo o una frequenza piu' lontana di F Tol dall'ultima mandata alla media (`jt65_decode.f90:417-419`).
    fn wants(&self, period: u64, freq: i32, tol_hz: f32) -> bool {
        self.last.is_none_or(|(p, f)| p != period || (freq - f).abs() as f32 > tol_hz)
    }

    /// Mette da parte gli spettri del candidato (se lo stesso periodo non ne ha gia' uno entro F Tol, `avg65:556-563`), somma quelli della
    /// stessa parita', dello stesso ritardo entro 0,2 s e della stessa frequenza entro F Tol (`:580-601`) e, con almeno due periodi, prova a
    /// decodificare la somma lisciandola come `decode65a` (`:607-650`, da `nint(width / df)` a due volte tanto). Ritorna il messaggio e quanti
    /// periodi sono stati sommati.
    #[allow(clippy::too_many_arguments)]
    fn add_and_decode(
        &mut self,
        codec: &Codec,
        period: u64,
        (freq, dt_s): (i32, f32),
        spectra: &BinSpectra,
        options: &Options,
        lists: (&ApTable, &[Hint]),
        pass: &Pass,
    ) -> Option<(Extracted, u32)> {
        self.last = Some((period, freq));
        let tol = options.tol_hz;
        if !self.saved.iter().any(|e| e.period == period && (freq - e.freq).abs() as f32 <= tol) {
            let entry = SavedPeriod { period, freq, dt_s, spectra: spectra.clone() };
            if self.saved.len() < MAX_AVERAGE {
                self.saved.push(entry);
            } else {
                self.saved[self.next] = entry;
                self.next = (self.next + 1) % MAX_AVERAGE;
            }
        }
        let mut sum: Option<BinSpectra> = None;
        let mut nsum = 0u32;
        for e in &self.saved {
            if e.period % 2 != period % 2 || (dt_s - e.dt_s).abs() > 0.2 || (freq - e.freq).abs() as f32 > tol || e.spectra.mode65 != spectra.mode65 {
                continue;
            }
            match sum.as_mut() {
                None => sum = Some(e.spectra.clone()),
                Some(s) => {
                    for (row, add) in s.rows.iter_mut().zip(e.spectra.rows.iter()) {
                        for (v, a) in row.iter_mut().zip(add.iter()) {
                            *v += a;
                        }
                    }
                }
            }
            nsum += 1;
        }
        if nsum < 2 {
            return None;
        }
        let mut sum = sum?;
        let w = (pass.width_hz / TONE_DF as f32).round() as i32;
        let range = if spectra.mode65 >= 2 { (w.max(0), 2 * w.max(0)) } else { (0, 0) };
        extract_smoothed(codec, &mut sum, range, nsum as i32, options, lists, pass.trials).map(|x| (x, nsum))
    }
}

/// Ritorna le costanti di tempo per chi deve sapere quanto dura un messaggio.
pub fn transmission_samples() -> usize {
    (NSYM as f64 * SYMBOL).ceil() as usize
}

/// La distanza tra i toni di JT65 sottomodo A, per chi deve disegnare la banda occupata (per B e C [`Submode::tone_spacing_hz`]).
pub fn tone_spacing_hz() -> f64 {
    TONE_DF
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::jt65::{modulate_submode, tones};

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
        minute_sub(messages, Submode::A, seed)
    }

    /// Come [`minute`] nel sottomodo dato.
    fn minute_sub(messages: &[(&str, f64, f64, f64)], sub: Submode, seed: u64) -> Vec<f32> {
        let codec = Codec::new();
        let mut rng = Lcg(seed);
        let n = 60 * 12_000;
        let mut audio: Vec<f32> = (0..n).map(|_| rng.gauss() as f32).collect();
        for &(msg, f0, dt_s, snr_db) in messages {
            let snr = 10f64.powf(snr_db / 10.0);
            let amp = (2.0 * snr * 2500.0 / 6000.0).sqrt() as f32;
            let wave = modulate_submode(&tones(&codec, msg), f0, sub, amp, 12_000.0);
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

    /// JT65B e JT65C: toni a distanza doppia e quadrupla, trasmessi e ricevuti; anche in VHF (un solo candidato attorno alla frequenza di
    /// ricezione, lisciatura alla larghezza del segnale).
    #[test]
    fn submodes_b_and_c_round_trip() {
        for (sub, seed) in [(Submode::B, 21), (Submode::C, 22)] {
            let audio = minute_sub(&[("K1ABC IU3ABC FN42", 1200.0, 0.2, -19.0)], sub, seed);
            let out = decode(&audio, &Options { submode: sub, ..Options::default() });
            let d = out.iter().find(|d| d.text == "K1ABC IU3ABC FN42").unwrap_or_else(|| panic!("{sub:?}: {out:?}"));
            assert!((d.freq_hz - 1200.0).abs() < 1.5 && (d.dt_s - 0.2).abs() < 0.08, "{sub:?}: {d:?}");
            let vhf = Options { submode: sub, vhf: true, rx_hz: Some(1210.0), ..Options::default() };
            let out = decode(&audio, &vhf);
            assert!(out.iter().any(|d| d.text == "K1ABC IU3ABC FN42" && d.averaged == 1), "{sub:?} VHF: {out:?}");
            assert!(out.iter().all(|d| (-30..=-1).contains(&d.snr_db)), "{out:?}");
        }
    }

    /// Un segnale di un sottomodo non si decodifica con un altro: i toni di dati cadono altrove.
    #[test]
    fn the_wrong_submode_does_not_decode() {
        for (sent, tried) in [(Submode::B, Submode::A), (Submode::C, Submode::B), (Submode::A, Submode::C)] {
            let audio = minute_sub(&[("K1ABC IU3ABC FN42", 1200.0, 0.2, -15.0)], sent, 23);
            let out = decode(&audio, &Options { submode: tried, ..Options::default() });
            assert!(out.iter().all(|d| d.text != "K1ABC IU3ABC FN42"), "{sent:?} letto come {tried:?}: {out:?}");
        }
    }

    /// Lo stesso segnale in tanti periodi pari, ognuno col suo rumore.
    fn periods(msg: &str, f0: f64, sub: Submode, snr_db: f64, n: u64) -> Vec<(u64, Vec<f32>)> {
        (0..n).map(|k| (2 * k, minute_sub(&[(msg, f0, 0.1 + 0.01 * k as f64, snr_db)], sub, 100 + k))).collect()
    }

    /// JT65B in VHF: un messaggio troppo debole per un periodo esce dalla media di piu' periodi pari (`avg65`); con «Include averaging» spento
    /// no, e «Clear Avg» ricomincia da capo.
    #[test]
    fn averaging_decodes_a_signal_too_weak_for_one_period() {
        let msg = "K1ABC IU3ABC FN42";
        let sub = Submode::B;
        let o = Options { submode: sub, vhf: true, rx_hz: Some(1000.0), average: true, ..Options::default() };
        let all = periods(msg, 1000.0, sub, -27.0, 6);
        let alone = decode(&all[0].1, &o);
        assert!(alone.iter().all(|d| d.text != msg), "un periodo solo non basta: {alone:?}");
        let mut avg = Averager::new();
        let mut found = None;
        for (period, audio) in &all {
            if let Some(d) = decode_averaged(audio, &o, &mut avg, *period).into_iter().find(|d| d.text == msg) {
                found = Some(d);
                break;
            }
        }
        let d = found.expect("decodificato dalla media");
        assert!(d.averaged >= 2, "{d:?}");
        assert!((d.freq_hz - 1000.0).abs() < 2.0, "{d:?}");
        // senza «Include averaging» la media non si usa
        let mut off = Averager::new();
        let plain = Options { average: false, ..o.clone() };
        for (period, audio) in &all {
            assert!(decode_averaged(audio, &plain, &mut off, *period).iter().all(|d| d.text != msg));
        }
        assert!(off.is_empty());
        avg.clear();
        assert!(avg.is_empty());
        // un periodo dispari non si somma ai pari
        let odd = decode_averaged(&all[1].1, &o, &mut avg, 1);
        assert!(odd.is_empty() && avg.len() == 1, "{odd:?}");
        let even = decode_averaged(&all[2].1, &o, &mut avg, 4);
        assert!(even.iter().all(|d| d.text != msg), "{even:?}");
    }

    /// Due stazioni deboli a frequenze diverse: ognuna ha la sua media, nessuna somma gli spettri dell'altra.
    #[test]
    fn averages_keep_separate_stations() {
        let o = Options { average: true, ..Options::default() };
        let mut avg = Averager::new();
        let mut got: Vec<Decoded> = Vec::new();
        for k in 0..6u64 {
            let audio = minute(&[("K1ABC IU3ABC FN42", 800.0, 0.1, -27.5), ("G0XYZ IU3ABC IO91", 1700.0, 0.4, -27.5)], 300 + k);
            for d in decode_averaged(&audio, &o, &mut avg, 2 * k) {
                if !got.iter().any(|g| g.text == d.text) {
                    got.push(d);
                }
            }
        }
        for (msg, f) in [("K1ABC IU3ABC FN42", 800.0), ("G0XYZ IU3ABC IO91", 1700.0)] {
            let d = got.iter().find(|d| d.text == msg).unwrap_or_else(|| panic!("{msg}: {got:?}"));
            assert!((d.freq_hz - f).abs() < 2.0, "{d:?}");
        }
        assert!(got.iter().all(|d| d.text == "K1ABC IU3ABC FN42" || d.text == "G0XYZ IU3ABC IO91"), "{got:?}");
        assert!(got.iter().any(|d| d.averaged >= 2), "{got:?}");
        // nessuna media somma piu' periodi di quelli della sua stazione: ogni stazione ne ha al massimo uno per periodo
        assert!(got.iter().all(|d| d.averaged <= 6), "{got:?}");
        for f in [800, 1700] {
            let mine: Vec<u64> = avg.saved.iter().filter(|e| (e.freq - f).abs() <= 50).map(|e| e.period).collect();
            let mut unique = mine.clone();
            unique.dedup();
            assert_eq!(mine, unique, "un solo spettro per periodo e stazione a {f} Hz");
        }
    }

    /// Con la media accesa il rumore resta rumore, anche sommando tanti periodi.
    #[test]
    fn averaging_noise_gives_nothing() {
        let o = Options { submode: Submode::C, vhf: true, rx_hz: Some(1000.0), average: true, ..Options::default() };
        let mut avg = Averager::new();
        for k in 0..6u64 {
            let out = decode_averaged(&minute_sub(&[], Submode::C, 400 + k), &o, &mut avg, 2 * k);
            assert!(out.is_empty(), "periodo {k}: {out:?}");
        }
    }

    /// `lorentzian`: centro e larghezza a meta' altezza di una lorentziana su un fondo, col modello di `fchisq0` (troncata a tre semilarghezze).
    #[test]
    fn the_lorentzian_fit_finds_the_width() {
        let y: Vec<f32> = (1..=140)
            .map(|i| {
                let z = (i as f32 - 70.0) / 4.0;
                if z.abs() < 3.0 { 1.0 + 5.0 * (1.0 / (1.0 + z * z) - 0.1) } else { 1.0 }
            })
            .collect();
        let a = lorentzian(&y);
        assert!((a[2] - 70.0).abs() < 1.0 && (a[3] - 8.0).abs() < 1.5, "{a:?}");
    }
}
