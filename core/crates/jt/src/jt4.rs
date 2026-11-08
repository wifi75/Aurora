//! JT4: 4-FSK a 4,375 baud, 206 simboli, messaggio di 72 bit con codice convoluzionale K=32 r=1/2 (come JT9) e decodificatore di Fano.
//! Dai sorgenti di WSJT-X (GPL 3): `jt4.f90`, `gen4.f90`, `interleave4.f90`, `extract4.f90`, `getmet4.f90`, `xcor4.f90`, `jt4_decode.f90`.
//!
//! Ogni simbolo porta un bit di dati (bit alto del tono) e un bit della sequenza pseudocasuale `NPR` (bit basso): tono = 2 * dato + sincronismo.
//! I sottomodi da A a G cambiano solo la distanza fra i toni (4,375 Hz per 1, 2, 4, 9, 18, 36, 72).
//!
//! WSJT-X lavora a 11025 Hz; qui tutto e' a 12 kHz come il resto di Aurora, con i confini dei simboli frazionari (2742,857 campioni).

use rustfft::{FftPlanner, num_complex::Complex};

use crate::fano::{Metric, decode_message_with};
use crate::jt9::{encode232, entail, interleave};
use crate::pack::pack_message;

const FS: f64 = 12_000.0;
/// Simboli al secondo: 11025 / 2520.
pub const BAUD: f64 = 11_025.0 / 2_520.0;
/// Simboli trasmessi.
pub const NSYM: usize = 206;
/// Campioni per simbolo a 12 kHz (non intero).
pub const NSPS: f64 = FS / BAUD;
/// Moltiplicatore della distanza fra i toni per i sottomodi da A a G.
pub const NCH: [u32; 7] = [1, 2, 4, 9, 18, 36, 72];
/// L'inizio nominale della trasmissione: un secondo dopo l'inizio del minuto.
const NOMINAL_START_S: f64 = 1.0;

/// `npr(207)` di `jt4.f90`: la sequenza di sincronismo; il simbolo trasmesso `i` usa `NPR[i + 1]`.
pub const NPR: [u8; 207] = [
    0, 0, 0, 0, 1, 1, 0, 0, 0, 1, 1, 0, 1, 1, 0, 0, 1, 0, 1, 0, 0, 0, 0, 0, 0, 0, 1, 1, 0, 0, //
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 1, 1, 0, 1, 1, 0, 1, 0, 1, 1, 1, 1, 1, 0, 1, 0, 0, 0, //
    1, 0, 0, 1, 0, 0, 1, 1, 1, 1, 1, 0, 0, 0, 1, 0, 1, 0, 0, 0, 1, 1, 1, 1, 0, 1, 1, 0, 0, 1, //
    0, 0, 0, 1, 1, 0, 1, 0, 1, 0, 1, 0, 1, 0, 1, 1, 1, 1, 1, 0, 1, 0, 1, 0, 1, 1, 0, 1, 0, 1, //
    0, 1, 1, 1, 0, 0, 1, 0, 1, 1, 0, 1, 1, 1, 1, 0, 0, 0, 0, 1, 1, 0, 1, 1, 0, 0, 0, 1, 1, 1, //
    0, 1, 1, 1, 0, 1, 1, 1, 0, 0, 1, 0, 0, 0, 1, 1, 0, 1, 1, 0, 0, 1, 0, 0, 0, 1, 1, 1, 1, 1, //
    1, 0, 0, 1, 1, 0, 0, 0, 0, 1, 1, 0, 0, 0, 1, 0, 1, 1, 0, 1, 1, 1, 1, 0, 1, 0, 1,
];

/// Sottomodo di JT4: 0 = A ... 6 = G.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Submode(pub u8);

impl Submode {
    /// Da una lettera ('A'..='G', anche minuscola).
    pub fn from_letter(c: char) -> Option<Submode> {
        let i = (c.to_ascii_uppercase() as u8).checked_sub(b'A')?;
        (usize::from(i) < NCH.len()).then_some(Submode(i))
    }

    pub fn letter(self) -> char {
        char::from(b'A' + self.0.min(6))
    }

    fn nch(self) -> u32 {
        NCH[usize::from(self.0.min(6))]
    }

    /// Distanza fra due toni vicini, in hertz.
    pub fn tone_spacing_hz(self) -> f64 {
        BAUD * f64::from(self.nch())
    }
}

/// `fmtmsg`: maiuscole e spazi singoli, come lo vede `gen4` per la regola del sincronismo invertito.
fn formatted(msg: &str) -> String {
    msg.split_whitespace().collect::<Vec<_>>().join(" ").to_uppercase()
}

/// Sincronismo invertito (`#` di WSJT-X): rapporti come «-15» dopo i nominativi (un '-' dalla nona posizione seguito da 0..3).
fn inverted_sync(msg: &str) -> bool {
    let m = formatted(msg);
    let b = m.as_bytes();
    match m.find('-') {
        Some(i) if i >= 8 => b.get(i + 1).is_some_and(|c| (b'0'..=b'3').contains(c)),
        _ => false,
    }
}

/// I 206 bit di dati dopo codifica e rimescolamento.
fn data_bits(msg: &str) -> [u8; NSYM] {
    let (dgen, _) = pack_message(msg);
    interleave(&encode232(&entail(&dgen)), 1)
}

/// I 206 toni (0..=3): `2 * dato + sincronismo`.
pub fn tones(msg: &str) -> [u8; NSYM] {
    let bits = data_bits(msg);
    let inv = inverted_sync(msg);
    let mut out = [0u8; NSYM];
    for (i, t) in out.iter_mut().enumerate() {
        let s = NPR[i + 1];
        *t = 2 * bits[i] + if inv { 1 - s } else { s };
    }
    out
}

/// Primo campione del simbolo `k` partendo da `start` (confini frazionari arrotondati).
fn symbol_at(start: f64, k: usize) -> i64 {
    (start + k as f64 * NSPS).round() as i64
}

/// L'onda audio a 12 kHz (fase continua): `f0` e' la frequenza del tono 0.
pub fn modulate(tones: &[u8; NSYM], f0: f64, submode: Submode, amplitude: f32) -> Vec<f32> {
    let total = symbol_at(0.0, NSYM) as usize;
    let mut out = Vec::with_capacity(total);
    let mut phase = 0.0f64;
    let two_pi = 2.0 * std::f64::consts::PI;
    for (k, &t) in tones.iter().enumerate() {
        let n = (symbol_at(0.0, k + 1) - symbol_at(0.0, k)) as usize;
        let dphi = two_pi * (f0 + f64::from(t) * submode.tone_spacing_hz()) / FS;
        for _ in 0..n {
            phase += dphi;
            if phase > two_pi {
                phase -= two_pi;
            }
            out.push(amplitude * phase.sin() as f32);
        }
    }
    out
}

/// Un messaggio decodificato.
#[derive(Debug, Clone, PartialEq)]
pub struct Decoded {
    pub text: String,
    /// Frequenza del tono 0, in hertz.
    pub freq_hz: f32,
    /// Ritardo rispetto all'inizio nominale (un secondo dopo l'inizio del minuto), in secondi.
    pub dt_s: f32,
    /// Rapporto segnale/rumore in 2500 Hz, in dB.
    pub snr_db: i32,
    /// Forza del sincronismo (picco della correlazione sul suo rumore).
    pub sync: f32,
}

/// Come si cerca.
#[derive(Debug, Clone)]
pub struct Options {
    pub f_low_hz: f32,
    pub f_high_hz: f32,
    pub submode: Submode,
    /// Al massimo quanti candidati si provano.
    pub max_candidates: usize,
    /// Soglia del sincronismo (picco sul rumore della correlazione).
    pub min_sync: f32,
}

impl Default for Options {
    fn default() -> Self {
        Options { f_low_hz: 200.0, f_high_hz: 3000.0, submode: Submode(0), max_candidates: 20, min_sync: 3.0 }
    }
}

/// Spettri di potenza di un simbolo (finestra di un simbolo, allungata a due con zeri: un bin = mezzo tono di JT4A),
/// a passi di mezzo simbolo (`ps4` di `sync4`). `s[j][i]`.
struct Spectra {
    s: Vec<Vec<f32>>,
    df: f64,
    hstep: f64,
}

fn spectra(audio: &[f32], max_hz: f64) -> Spectra {
    let n = NSPS.round() as usize;
    let nfft = 2 * n;
    let df = FS / nfft as f64;
    let hstep = NSPS / 2.0;
    let nbins = ((max_hz / df).ceil() as usize + 2).min(nfft / 2);
    let fft = FftPlanner::<f32>::new().plan_fft_forward(nfft);
    let mut s = Vec::new();
    let mut buf = vec![Complex::new(0.0f32, 0.0); nfft];
    let mut j = 0usize;
    loop {
        let k = (j as f64 * hstep).round() as usize;
        if k + n > audio.len() {
            break;
        }
        for (i, b) in buf.iter_mut().enumerate() {
            *b = Complex::new(if i < n { audio[k + i] } else { 0.0 }, 0.0);
        }
        fft.process(&mut buf);
        s.push(buf[..nbins].iter().map(|c| c.norm_sqr()).collect());
        j += 1;
    }
    Spectra { s, df, hstep }
}

/// `xcor4`: per il tono 0 al bin `ipk` la differenza fra l'energia dei toni con sincronismo 1 e quelli con 0, correlata con `NPR`.
/// Ritorna (picco con segno, ritardo in mezzi simboli, rumore della correlazione).
fn xcor(sp: &Spectra, ipk: usize, nch: u32, lags: std::ops::RangeInclusive<i64>) -> Option<(f32, f64, f32)> {
    let n = 2 * nch as usize;
    let kz = (nch as i64 / 2).max(1);
    let nbins = sp.s.first()?.len();
    if ipk < kz as usize || ipk + 3 * n + kz as usize >= nbins {
        return None;
    }
    let a: Vec<f32> = sp
        .s
        .iter()
        .map(|row| {
            let mut ss = [0f32; 4];
            let mut wsum = 0f32;
            for k in -kz + 1..kz {
                let w = (kz - k.abs()) as f32 / nch as f32;
                wsum += w;
                for (t, v) in ss.iter_mut().enumerate() {
                    *v += w * row[(ipk as i64 + (t * n) as i64 + k) as usize];
                }
            }
            (ss[1].max(ss[3]) - ss[0].max(ss[2])) / wsum.sqrt()
        })
        .collect();
    let ccf: Vec<(i64, f32)> = lags
        .map(|lag| {
            let mut x = 0f32;
            for i in 0..NSYM {
                let j = 2 * i as i64 + lag;
                if j >= 0 && (j as usize) < a.len() {
                    x += a[j as usize] * (2.0 * f32::from(NPR[i + 1]) - 1.0);
                }
            }
            (lag, x)
        })
        .collect();
    let (pk, val) = ccf.iter().copied().max_by(|a, b| a.1.abs().total_cmp(&b.1.abs()))?;
    let idx = ccf.iter().position(|c| c.0 == pk)?;
    // `peakup`: vertice della parabola sui tre punti
    let mut xlag = pk as f64;
    if idx > 0 && idx + 1 < ccf.len() {
        let (y1, y2, y3) = (ccf[idx - 1].1.abs(), val.abs(), ccf[idx + 1].1.abs());
        let den = y1 - 2.0 * y2 + y3;
        if den.abs() > f32::EPSILON {
            xlag += f64::from((0.5 * (y1 - y3) / den).clamp(-0.5, 0.5));
        }
    }
    let rest: Vec<f32> = ccf.iter().filter(|c| (c.0 - pk).abs() > 2).map(|c| c.1).collect();
    if rest.is_empty() {
        return None;
    }
    let mean = rest.iter().sum::<f32>() / rest.len() as f32;
    let rms = (rest.iter().map(|v| (v - mean).powi(2)).sum::<f32>() / rest.len() as f32).sqrt();
    (rms > 0.0).then_some((val, xlag, rms))
}

/// Energia del tono `freq` nel simbolo che parte da `start`, sommata su `chips` pezzi (per i sottomodi larghi, come `decode4`).
fn tone_power(audio: &[f32], start: i64, len: usize, freq: f64, chips: usize) -> f64 {
    let w = 2.0 * std::f64::consts::PI * freq / FS;
    let per = (len / chips).max(1);
    let mut total = 0.0;
    for c in 0..chips {
        let (mut re, mut im) = (0.0f64, 0.0f64);
        for i in 0..per {
            let k = start + (c * per + i) as i64;
            if k < 0 || k as usize >= audio.len() {
                continue;
            }
            let x = f64::from(audio[k as usize]);
            let ph = w * (i as f64);
            re += x * ph.cos();
            im -= x * ph.sin();
        }
        total += re * re + im * im;
    }
    total
}

/// Simboli soft dei dati (gia' rimessi in ordine) e rapporto S/N stimato, per tono 0 a `f0` e primo simbolo a `start`.
fn soft_symbols(audio: &[f32], f0: f64, start: f64, submode: Submode, flip: bool) -> ([f32; NSYM], i32) {
    let sp = submode.tone_spacing_hz();
    let chips = (submode.nch() as usize).min(70);
    let mut soft = [0f32; NSYM];
    let (mut sig, mut noise) = (0.0f64, 0.0f64);
    for (i, out) in soft.iter_mut().enumerate() {
        let s = f64::from(if flip { 1 - NPR[i + 1] } else { NPR[i + 1] });
        let a = symbol_at(start, i);
        let len = (symbol_at(start, i + 1) - a) as usize;
        let p0 = tone_power(audio, a, len, f0 + s * sp, chips);
        let p1 = tone_power(audio, a, len, f0 + (2.0 + s) * sp, chips);
        let q0 = tone_power(audio, a, len, f0 + (1.0 - s) * sp, chips);
        let q1 = tone_power(audio, a, len, f0 + (3.0 - s) * sp, chips);
        *out = (p1 - p0) as f32;
        sig += p0.max(p1);
        noise += 0.5 * (q0 + q1);
    }
    // i toni senza segnale (sincronismo sbagliato) danno il rumore in una banda di 4,375 Hz per pezzo
    let bw = BAUD * chips as f64;
    let snr = if noise > 0.0 { 10.0 * ((sig - noise).max(noise * 1e-4) / noise * bw / 2500.0).log10() } else { -30.0 };
    (soft, (snr.round() as i32).clamp(-30, 20))
}

/// `extract4`: normalizza, scala, rimette in ordine e decodifica con Fano.
fn extract(soft: &[f32; NSYM]) -> Option<String> {
    let mean = soft.iter().sum::<f32>() / NSYM as f32;
    let rms = (soft.iter().map(|v| (v - mean).powi(2)).sum::<f32>() / NSYM as f32).sqrt();
    if rms <= f32::MIN_POSITIVE {
        return None;
    }
    let mut sym = [0u8; NSYM];
    for (o, v) in sym.iter_mut().zip(soft) {
        // si passa per u8 per poter riusare `interleave`, poi si torna al segno
        *o = ((30.0 * (v - mean) / rms).round().clamp(-127.0, 127.0) as i8) as u8;
    }
    let ordered = interleave(&sym, -1);
    let mut symbols = [0i8; 207];
    for (o, v) in symbols.iter_mut().zip(ordered) {
        *o = v as i8;
    }
    decode_message_with(&symbols, 10_000, Metric::Jt4)
}

/// Decodifica un periodo di un minuto (audio a 12 kHz dall'inizio del minuto).
pub fn decode(audio: &[f32], options: &Options) -> Vec<Decoded> {
    let sub = options.submode;
    let nch = sub.nch();
    let max_hz = f64::from(options.f_high_hz) + 4.0 * sub.tone_spacing_hz() + 20.0;
    let sp = spectra(audio, max_hz);
    if sp.s.len() < 2 * NSYM {
        return Vec::new();
    }
    // ritardi da -1 a +3 secondi rispetto all'inizio nominale, in mezzi simboli
    let lag_of = |dt: f64| ((NOMINAL_START_S + dt) * FS / sp.hstep).round() as i64;
    let lags = lag_of(-1.0)..=lag_of(3.0);
    let ia = (f64::from(options.f_low_hz) / sp.df).floor().max(1.0) as usize;
    let ib = (f64::from(options.f_high_hz) / sp.df).ceil() as usize;
    let mut found: Vec<(usize, f32, f64, f32)> = Vec::new();
    for i in ia..=ib {
        if let Some((val, xlag, rms)) = xcor(&sp, i, nch, lags.clone()) {
            found.push((i, val, xlag, val.abs() / rms));
        }
    }
    // un candidato per picco: il migliore entro un tono
    let width = 2 * nch as usize;
    let mut candidates: Vec<(usize, f32, f64, f32)> = found
        .iter()
        .copied()
        .filter(|c| c.3 >= options.min_sync && found.iter().all(|o| o.0.abs_diff(c.0) > width || o.3 <= c.3))
        .collect();
    candidates.sort_by(|a, b| b.3.total_cmp(&a.3));
    candidates.truncate(options.max_candidates);
    let mut out: Vec<Decoded> = Vec::new();
    for (i, val, xlag, sync) in candidates {
        let flip = val < 0.0;
        let f_bin = i as f64 * sp.df;
        let start_bin = xlag * sp.hstep;
        let mut done = None;
        'search: for dt_off in [0.0, -0.25, 0.25, -0.5, 0.5] {
            for f_off in [0.0, -0.5, 0.5] {
                let f0 = f_bin + f_off * sp.df;
                let start = start_bin + dt_off * sp.hstep;
                let (soft, snr) = soft_symbols(audio, f0, start, sub, flip);
                if let Some(text) = extract(&soft) {
                    done = Some((text, f0, start, snr));
                    break 'search;
                }
            }
        }
        if let Some((text, f0, start, snr)) = done
            && !out.iter().any(|d| d.text == text)
        {
            out.push(Decoded { text, freq_hz: f0 as f32, dt_s: (start / FS - NOMINAL_START_S) as f32, snr_db: snr, sync });
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Rumore gaussiano ripetibile (xorshift e Box-Muller).
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

    fn period(msg: &str, f0: f64, dt: f64, sub: Submode, amp: f32, sigma: f32) -> Vec<f32> {
        let mut audio = noise(60 * 12_000, sigma, 7);
        let wave = modulate(&tones(msg), f0, sub, amp);
        let at = ((NOMINAL_START_S + dt) * FS) as usize;
        for (k, v) in wave.iter().enumerate() {
            audio[at + k] += v;
        }
        audio
    }

    #[test]
    fn tones_carry_the_sync_in_the_low_bit() {
        let t = tones("CQ IU3ABC IO91");
        assert!(t.iter().all(|&v| v < 4));
        for (i, &v) in t.iter().enumerate() {
            assert_eq!(v & 1, NPR[i + 1]);
        }
    }

    #[test]
    fn a_report_after_the_calls_inverts_the_sync() {
        assert!(inverted_sync("IU3ABC IU3ABD -15"));
        assert!(!inverted_sync("CQ IU3ABC IO91"));
        let t = tones("IU3ABC IU3ABD -15");
        for (i, &v) in t.iter().enumerate() {
            assert_eq!(v & 1, 1 - NPR[i + 1]);
        }
    }

    #[test]
    fn submodes_have_the_wsjt_spacing() {
        assert!((Submode(0).tone_spacing_hz() - 4.375).abs() < 1e-9);
        assert_eq!(Submode::from_letter('g'), Some(Submode(6)));
        assert_eq!(Submode::from_letter('H'), None);
        assert!((Submode(2).tone_spacing_hz() - 17.5).abs() < 1e-9);
    }

    #[test]
    fn the_wave_lasts_206_symbols() {
        let w = modulate(&tones("CQ IU3ABC IO91"), 1000.0, Submode(0), 1.0);
        assert_eq!(w.len(), (206.0 * NSPS).round() as usize);
    }

    #[test]
    fn a_noisy_jt4a_signal_decodes() {
        let audio = period("CQ IU3ABC IO91", 1200.0, 0.4, Submode(0), 0.1, 1.0);
        let d = decode(&audio, &Options::default());
        let hit = d.iter().find(|d| d.text == "CQ IU3ABC IO91").expect("decodificato");
        assert!((hit.freq_hz - 1200.0).abs() < 3.0, "{hit:?}");
        assert!((hit.dt_s - 0.4).abs() < 0.15, "{hit:?}");
    }

    #[test]
    fn an_inverted_sync_report_decodes() {
        let audio = period("IU3ABC IU3ABD -15", 800.0, 0.0, Submode(0), 0.2, 1.0);
        let d = decode(&audio, &Options::default());
        assert!(d.iter().any(|d| d.text == "IU3ABC IU3ABD -15"), "{d:?}");
    }

    #[test]
    fn a_wide_submode_decodes() {
        let sub = Submode(2);
        let audio = period("CQ IU3ABC IO91", 1000.0, 0.2, sub, 0.2, 1.0);
        let d = decode(&audio, &Options { submode: sub, ..Options::default() });
        assert!(d.iter().any(|d| d.text == "CQ IU3ABC IO91"), "{d:?}");
    }

    #[test]
    fn pure_noise_decodes_nothing() {
        let audio = noise(60 * 12_000, 1.0, 3);
        assert!(decode(&audio, &Options::default()).is_empty());
    }
}
