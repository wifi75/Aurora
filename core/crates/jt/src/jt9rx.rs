//! JT9: ricezione. Spettri a mezzo simbolo, ricerca dei segnali (`sync9`), affinamento di frequenza e ritardo, energie dei nove toni di ogni simbolo,
//! simboli soft (`symspec2`) e decodifica di Fano (`jt9_decode`, `sync9.f90`, `softsym.f90`, `symspec2.f90` di WSJT-X, GPL 3).

use crate::fano;
use crate::jt9::{NSPS, NSYM, SYNC, TONE_SPACING_HZ, interleave};
use rustfft::FftPlanner;
use rustfft::num_complex::Complex;

const FS: f64 = 12_000.0;
const NFFT3: usize = 16_384;
/// Bin dello spettro fino a 5 kHz.
const NSMAX: usize = 6_827;
/// Mezzo simbolo in campioni.
const HSTEP: usize = NSPS / 2;
const MAX_HSYM: usize = 184;
const DF3: f64 = FS / NFFT3 as f64;
/// `ii2` di `jt9sync.f90`: le posizioni dei 16 simboli di sincronismo in mezzi simboli (da 1).
const II2: [i32; 16] = [1, 3, 9, 19, 31, 45, 65, 69, 101, 103, 109, 119, 131, 145, 165, 169];
const RED2_LIMIT: f32 = 1.6;

/// Un messaggio decodificato.
#[derive(Debug, Clone, PartialEq)]
pub struct Decoded {
    pub text: String,
    /// Frequenza del tono di sincronismo in hertz.
    pub freq_hz: f32,
    /// Ritardo rispetto all'inizio nominale (un secondo dopo l'inizio del minuto), in secondi.
    pub dt_s: f32,
    /// Rapporto segnale/rumore in 2500 Hz, in dB.
    pub snr_db: i32,
    /// Forza del sincronismo.
    pub sync: f32,
}

/// Come si cerca (`ndepth` di JTDX: 1 rapida, 2 media, 3 profonda; per JT9 conta davvero, a differenza di FT8).
#[derive(Debug, Clone)]
pub struct Options {
    pub f_low_hz: f32,
    pub f_high_hz: f32,
    /// 1, 2 o 3: soglie del sincronismo e cicli massimi del decodificatore di Fano.
    pub depth: u8,
    /// Al massimo quanti candidati si provano.
    pub max_candidates: usize,
}

impl Default for Options {
    fn default() -> Self {
        Options { f_low_hz: 200.0, f_high_hz: 4000.0, depth: 3, max_candidates: 40 }
    }
}

impl Options {
    /// `limit` (cicli di Fano per bit) e `ccflim` (soglia del sincronismo) per profondita': i valori di `jt9_decode` letti in `jtdxjt9.exe`
    /// (`0x4783b0`: 5000 / 3,0 a profondita' 1, 10000 / 2,7 a 2, 30000 / 2,5 a 3).
    fn limits(&self) -> (usize, f32) {
        match self.depth {
            0 | 1 => (5_000, 3.0),
            2 => (10_000, 2.7),
            _ => (30_000, 2.5),
        }
    }
}

/// `pctile`: il valore che sta al `pct` per cento della classifica (indice `nint(n * pct / 100)`, da 1).
fn percentile(values: &[f32], pct: usize) -> f32 {
    if values.is_empty() {
        return 0.0;
    }
    let mut v = values.to_vec();
    v.sort_by(|a, b| a.total_cmp(b));
    let j = ((values.len() as f64 * 0.01 * pct as f64).round() as usize).max(1);
    v[(j - 1).min(v.len() - 1)]
}

fn nuttall_window() -> Vec<f32> {
    let (a0, a1, a2, a3) = (0.3635819f64, -0.4891775f64, 0.1365995f64, -0.0106411f64);
    (0..NFFT3)
        .map(|i| {
            let x = std::f64::consts::TAU * i as f64 / NFFT3 as f64;
            (a0 + a1 * x.cos() + a2 * (2.0 * x).cos() + a3 * (3.0 * x).cos()) as f32
        })
        .collect()
}

/// `symspec`: gli spettri a mezzo simbolo, `ss[j * NSMAX + (i - 1)]`; ritorna anche il numero di mezzi simboli.
fn half_symbol_spectra(samples: &[f32]) -> (Vec<f32>, usize) {
    let nz = (samples.len() / HSTEP).min(MAX_HSYM);
    if nz < 20 {
        return (Vec::new(), 0);
    }
    let window = nuttall_window();
    let mut planner = FftPlanner::<f32>::new();
    let fft = planner.plan_fft_forward(NFFT3);
    let mut scratch = vec![Complex::new(0.0f32, 0.0); fft.get_inplace_scratch_len()];
    let mut buf = vec![Complex::new(0.0f32, 0.0); NFFT3];
    let mut ss = vec![0.0f32; nz * NSMAX];
    let fac = (1.0 / NFFT3 as f32).powi(2);
    for j in 0..nz {
        let ja = (j + 1) * HSTEP;
        for (i, b) in buf.iter_mut().enumerate() {
            // `j = ja + i - (nfft3 - 1)` da 1: la finestra finisce al campione `ja`
            let idx = ja as i64 + i as i64 - NFFT3 as i64;
            let x = if idx >= 0 && (idx as usize) < samples.len() { 0.1 * samples[idx as usize] } else { 0.0 };
            *b = Complex::new(x * window[i], 0.0);
        }
        fft.process_with_scratch(&mut buf, &mut scratch);
        for i in 1..=NSMAX {
            ss[j * NSMAX + i - 1] = fac * buf[i - 1].norm_sqr();
        }
    }
    (ss, nz)
}

/// I risultati di `sync9`: il sincronismo di ogni frequenza, il picco nello spettro medio e il ritardo migliore.
struct SyncMap {
    ccfred: Vec<f32>,
    red2: Vec<f32>,
    lag: Vec<i32>,
}

/// `sync9`: per ogni bin correla i 16 mezzi simboli di sincronismo con l'energia del bin, sui ritardi da -9 a +18 mezzi simboli.
fn sync9(ss: &[f32], nz: usize, ia: usize, ib: usize) -> SyncMap {
    let tstep = 0.5 * NSPS as f64 / FS;
    let lag1 = -((2.5 / tstep + 0.9999) as i32);
    let lag2 = (5.0 / tstep + 0.9999) as i32;
    let mut ccfred = vec![0.0f32; NSMAX + 2];
    let mut lagbest = vec![0i32; NSMAX + 2];
    let mut ss1 = vec![0.0f32; nz];
    for i in ia..=ib {
        for (j, v) in ss1.iter_mut().enumerate() {
            *v = ss[j * NSMAX + i - 1];
        }
        let xmed = percentile(&ss1, 40).max(1e-30);
        for v in ss1.iter_mut() {
            *v = (*v / xmed - 1.0).min(3.0);
        }
        let sbase = percentile(&ss1, 45);
        for v in ss1.iter_mut() {
            *v -= sbase;
        }
        let mut smax = 0.0f32;
        let mut lag_at = 0;
        for lag in lag1..=lag2 {
            let mut sum1 = 0.0f32;
            for &p in &II2 {
                let k = p + lag;
                if k >= 1 && (k as usize) <= nz {
                    sum1 += ss1[k as usize - 1];
                }
            }
            if sum1 > smax {
                smax = sum1;
                lag_at = lag;
            }
        }
        ccfred[i] = smax;
        lagbest[i] = lag_at;
    }
    let mut xmed = percentile(&ccfred[ia..=ib], 50);
    if xmed <= 0.0 {
        xmed = 1.0;
    }
    for v in ccfred[ia..=ib].iter_mut() {
        *v = 2.0 * *v / xmed;
    }
    // spettro medio, filtrato per far risaltare un segnale rispetto ai vicini
    let mut savg = vec![0.0f32; NSMAX + 2];
    for i in ia..=ib {
        let mut sum = 0.0f32;
        for j in 0..nz {
            sum += ss[j * NSMAX + i - 1];
        }
        savg[i] = sum / nz as f32;
    }
    let mut savg2 = vec![0.0f32; NSMAX + 2];
    let mut sq = Vec::with_capacity(ib - ia + 1);
    for (offset, out) in savg2[ia..=ib].iter_mut().enumerate() {
        let i = ia + offset;
        let mut sm = 0.0f32;
        for j in -5i32..=25 {
            let w = if (0..=20).contains(&j) { 1.0 / 21.0 } else { -(1.0 / 21.0) * (21.0 / 10.0) };
            let k = i as i32 + j;
            if k >= 1 && (k as usize) < NSMAX {
                sm += w * savg[k as usize];
            }
        }
        *out = sm;
        sq.push(sm * sm);
    }
    let rms = percentile(&sq, 20).sqrt().max(1e-30);
    for v in savg2[ia..=ib].iter_mut() {
        *v /= 5.0 * rms;
    }
    let mut red2 = vec![0.0f32; NSMAX + 2];
    if ib >= ia + 21 {
        for i in ia + 11..=ib - 10 {
            let reference = savg2[i - 10].max(savg2[i + 10]);
            red2[i] = (savg2[i] - reference).clamp(-99.0, 99.0);
        }
    }
    SyncMap { ccfred, red2, lag: lagbest }
}

fn rotation_table(f: f64) -> Vec<Complex<f32>> {
    (0..NSPS)
        .map(|n| {
            let ph = -std::f64::consts::TAU * f * n as f64 / FS;
            Complex::new(ph.cos() as f32, ph.sin() as f32)
        })
        .collect()
}

fn tone_energy(samples: &[f32], start: i64, table: &[Complex<f32>]) -> f32 {
    let (mut re, mut im) = (0.0f32, 0.0f32);
    for (n, t) in table.iter().enumerate() {
        let idx = start + n as i64;
        if idx >= 0 && (idx as usize) < samples.len() {
            let x = samples[idx as usize];
            re += x * t.re;
            im += x * t.im;
        }
    }
    re * re + im * im
}

fn symbol_start(dt_samples: f64, k: usize) -> i64 {
    (dt_samples + k as f64 * NSPS as f64).round() as i64
}

/// Energia del tono 0 nei 16 simboli di sincronismo rispetto a quella nei simboli di dati (`fchisq` senza il rapporto di potenza).
fn sync_metric(samples: &[f32], dt_samples: f64, table: &[Complex<f32>]) -> f32 {
    let (mut s1, mut s0) = (0.0f32, 0.0f32);
    for (k, &p) in SYNC.iter().enumerate() {
        let e = tone_energy(samples, symbol_start(dt_samples, k), table);
        if p == 1 {
            s1 += e;
        } else {
            s0 += e;
        }
    }
    (s1 / 16.0) / (s0 / 69.0 + 1e-30)
}

/// Affina frequenza e ritardo: prima il ritardo, poi la frequenza, poi il ritardo piu' fine.
fn refine(samples: &[f32], f0: f64, dt0: f64) -> (f64, f64) {
    let mut f = f0;
    let mut dt = dt0;
    let mut table = rotation_table(f);
    let mut best = sync_metric(samples, dt, &table);
    let sym = NSPS as f64;
    for (step, with_freq) in [(sym / 16.0, true), (sym / 64.0, false)] {
        let center = dt;
        for d in -8i32..=8 {
            let t = center + f64::from(d) * step;
            let m = sync_metric(samples, t, &table);
            if m > best {
                best = m;
                dt = t;
            }
        }
        if with_freq {
            let center_f = f;
            for d in -4i32..=4 {
                let ff = center_f + f64::from(d) * 0.35;
                let tb = rotation_table(ff);
                let m = sync_metric(samples, dt, &tb);
                if m > best {
                    best = m;
                    f = ff;
                    table = tb;
                }
            }
        }
    }
    (f, dt)
}

/// `symspec2`: le energie dei nove toni in ogni simbolo e, da quelle, i 207 simboli soft (nell'ordine rimescolato) con `schk` e il rapporto S/N.
fn soft_symbols(samples: &[f32], f: f64, dt_samples: f64) -> ([i8; 207], f32, i32) {
    let mut planner = FftPlanner::<f32>::new();
    let fft = planner.plan_fft_forward(NSPS);
    let mut scratch = vec![Complex::new(0.0f32, 0.0); fft.get_inplace_scratch_len()];
    let table = rotation_table(f);
    let mut buf = vec![Complex::new(0.0f32, 0.0); NSPS];
    let mut ss2 = vec![[0.0f32; 9]; NSYM];
    for (k, row) in ss2.iter_mut().enumerate() {
        let start = symbol_start(dt_samples, k);
        for (n, b) in buf.iter_mut().enumerate() {
            let idx = start + n as i64;
            let x = if idx >= 0 && (idx as usize) < samples.len() { samples[idx as usize] } else { 0.0 };
            *b = table[n] * x;
        }
        fft.process_with_scratch(&mut buf, &mut scratch);
        for (i, r) in row.iter_mut().enumerate() {
            *r = buf[i].norm_sqr();
        }
    }
    // `chkss2`: quanto il tono 0 spicca nei simboli di sincronismo
    let ave2 = ss2.iter().flat_map(|r| r.iter()).sum::<f32>() / (9.0 * NSYM as f32);
    let schk = SYNC.iter().enumerate().filter(|(_, p)| **p == 1).map(|(j, _)| ss2[j][0] / ave2 - 1.0).sum::<f32>() / 16.0;
    // solo i simboli di dati, toni 1..8
    let mut ss3: Vec<[f32; 8]> = Vec::with_capacity(69);
    for (j, &p) in SYNC.iter().enumerate() {
        if p == 0 {
            let mut r = [0.0f32; 8];
            r.copy_from_slice(&ss2[j][1..9]);
            ss3.push(r);
        }
    }
    let (mut ss, mut sig) = (0.0f32, 0.0f32);
    for row in &ss3 {
        let smax = row.iter().copied().fold(0.0f32, f32::max);
        ss += row.iter().sum::<f32>();
        sig += smax;
        ss -= smax;
    }
    let ave = (ss / (69.0 * 7.0)).max(1e-30);
    // il rumore e' misurato in un tono di 1,74 Hz: in 2500 Hz sono 31,6 dB in piu'
    let ratio = (sig / 69.0 / ave - 1.0).max(1e-2);
    let snr = (10.0 * ratio.log10() - 31.6).round() as i32;
    let mut soft = [0i8; 207];
    let mut k = 0usize;
    for row in &ss3 {
        let r: Vec<f32> = row.iter().map(|v| v / ave).collect();
        for m in (0..3).rev() {
            let (r1, r0) = match m {
                2 => (r[4].max(r[5]).max(r[6]).max(r[7]), r[0].max(r[1]).max(r[2]).max(r[3])),
                1 => (r[2].max(r[3]).max(r[4]).max(r[5]), r[0].max(r[1]).max(r[6]).max(r[7])),
                _ => (r[1].max(r[2]).max(r[4]).max(r[7]), r[0].max(r[3]).max(r[5]).max(r[6])),
            };
            let i4 = (10.0 * (r1 - r0)).round().clamp(-127.0, 127.0) as i32;
            soft[k] = i4 as i8;
            k += 1;
        }
    }
    (soft, schk, snr)
}

/// Decodifica un periodo di un minuto (audio a 12 kHz dall'inizio del minuto).
pub fn decode(samples: &[f32], options: &Options) -> Vec<Decoded> {
    let (ss, nz) = half_symbol_spectra(samples);
    if nz == 0 {
        return Vec::new();
    }
    let ia = ((f64::from(options.f_low_hz) / DF3).round() as usize).max(12);
    let ib = ((f64::from(options.f_high_hz) / DF3).round() as usize).min(NSMAX - 26);
    if ia + 22 >= ib {
        return Vec::new();
    }
    let map = sync9(&ss, nz, ia, ib);
    drop(ss);
    let (limit, ccflim) = options.limits();
    let df8 = 1500.0 / (NSPS as f64 / 8.0);
    let mut candidates: Vec<usize> = Vec::new();
    for i in ia..=ib {
        if map.ccfred[i] > ccflim && map.red2[i] > RED2_LIMIT {
            // un solo candidato per picco: il migliore entro +-12 bin
            let lo = i.saturating_sub(12).max(ia);
            let hi = (i + 12).min(ib);
            if (lo..=hi).all(|k| map.ccfred[k] <= map.ccfred[i]) {
                candidates.push(i);
            }
        }
    }
    candidates.sort_by(|&a, &b| map.ccfred[b].total_cmp(&map.ccfred[a]));
    candidates.truncate(options.max_candidates);
    let mut out: Vec<Decoded> = Vec::new();
    let mut tried: Vec<f64> = Vec::new();
    for i in candidates {
        let f_bin = (i - 1) as f64 * DF3;
        if tried.iter().any(|&t| (t - f_bin).abs() < 10.0 * df8) && out.iter().any(|o| (f64::from(o.freq_hz) - f_bin).abs() < 10.0 * df8) {
            continue;
        }
        tried.push(f_bin);
        // il simbolo 1 comincia a `lag * mezzo simbolo - 8192` campioni (la finestra dello spettro e' centrata sul simbolo)
        let dt0 = f64::from(map.lag[i]) * HSTEP as f64 - 8192.0;
        let (f, dt) = refine(samples, f_bin, dt0);
        let (soft_scrambled, schk, snr) = soft_symbols(samples, f, dt);
        if schk < 1.0 {
            continue;
        }
        // interleaving da togliere: i primi 206 simboli soft, in ordine di trasmissione
        let mut scrambled = [0u8; 206];
        for (k, s) in soft_scrambled.iter().take(206).enumerate() {
            scrambled[k] = *s as u8;
        }
        let ordered = interleave(&scrambled, -1);
        let mut soft = [0i8; 207];
        for (k, v) in ordered.iter().enumerate() {
            soft[k] = *v as i8;
        }
        let Some(text) = fano::decode_message(&soft, limit) else { continue };
        if out.iter().any(|o| o.text == text) {
            continue;
        }
        out.push(Decoded { text, freq_hz: f as f32, dt_s: (dt / FS - 1.0) as f32, snr_db: snr, sync: map.ccfred[i] });
    }
    out
}

/// La distanza tra i toni di JT9, per chi deve disegnare la banda occupata.
pub fn tone_spacing_hz() -> f64 {
    TONE_SPACING_HZ
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::jt9::{modulate, tones};

    struct Lcg(u64);
    impl Lcg {
        fn next(&mut self) -> f64 {
            self.0 = self.0.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
            ((self.0 >> 11) as f64) / ((1u64 << 53) as f64)
        }
        fn gauss(&mut self) -> f64 {
            let u1 = self.next().max(1e-12);
            let u2 = self.next();
            (-2.0 * u1.ln()).sqrt() * (std::f64::consts::TAU * u2).cos()
        }
    }

    /// Un minuto di rumore bianco (sigma 1) con i messaggi al rapporto dato (dB in 2500 Hz).
    fn minute(messages: &[(&str, f64, f64, f64)], seed: u64) -> Vec<f32> {
        let mut rng = Lcg(seed);
        let n = 60 * 12_000;
        let mut audio: Vec<f32> = (0..n).map(|_| rng.gauss() as f32).collect();
        for &(msg, f0, dt_s, snr_db) in messages {
            let snr = 10f64.powf(snr_db / 10.0);
            let amp = (2.0 * snr * 2500.0 / 6000.0).sqrt() as f32;
            let wave = modulate(&tones(msg), f0, amp);
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
            assert!(decode(&minute(&[], seed), &Options::default()).is_empty(), "il rumore non deve dare messaggi");
        }
    }
}
