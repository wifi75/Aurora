//! Ricezione di UltraFOX (e di SuperFox di WSJT-X 3.3: `sfrx_sub.f90`, `qpc_sync.f90`, `sfox_demod.f90`, `qpc_decode2.f90`,
//! `sfox_unpack.f90`). Si cerca il tono del sincronismo (il piu' forte e fisso, a circa 750 Hz), si trova l'inizio con i 24 simboli di
//! sincronismo, si misura l'energia dei 128 toni di dati in ognuno dei 127 simboli, si passa alle probabilita' e al codice polare e si
//! controlla l'integrita'. Soglia misurata (prova `soglia`): sicura a -16 dB in 2500 Hz, rara sotto -17,5 dB. Il messaggio torna nella forma di WSJT-X («K1ABC IU3ABC RR73», «K1ABC IU3ABC -12», «CQ IU3ABC IO91»).

use std::sync::Arc;

use rustfft::{num_complex::Complex32, Fft, FftPlanner};

use crate::qpc;
use crate::ultrafox::{self, NSPS, SYMBOLS, TONES};

const RATE: f32 = 12_000.0;
const BAUD: f32 = RATE / NSPS as f32;
const ISYNC: [usize; 24] = [1, 2, 4, 7, 11, 16, 22, 29, 37, 39, 42, 43, 45, 48, 52, 57, 63, 70, 78, 80, 83, 84, 86, 89];
const NDOWN: usize = 16;
const NQU1RKS: u32 = 203_514_677;
/// Sotto questo rapporto segnale/rumore una decodifica «riuscita» e' quasi sempre falsa (`qpc_decode2`).
const MIN_SNR: f32 = -16.5;

const DITHERS: usize = 100;

/// Probabilita' scosse (`qpc_decode2`): quelle grandi (> 0,5) restano, le altre si moltiplicano per 1 + r con r a caso tra -1 e 1
/// (`random` spento: r = 0), poi ogni riga si riporta a somma 1.
fn dither(py: &[f32], random: bool, rng: &mut u64) -> Vec<f32> {
    let mut out: Vec<f32> = py
        .iter()
        .map(|&p| {
            let r = if random {
                *rng = rng.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1_442_695_040_888_963_407);
                ((*rng >> 40) as f32 / (1u64 << 24) as f32) * 2.0 - 1.0
            } else {
                0.0
            };
            if p > 0.5 { p } else { p * (1.0 + r) }
        })
        .collect();
    for row in out.chunks_mut(qpc::Q) {
        let total: f32 = row.iter().sum();
        if total > 0.0 {
            row.iter_mut().for_each(|v| *v /= total);
        }
    }
    out
}

/// Una trasmissione UltraFOX/SuperFox decodificata.
#[derive(Debug, Clone, PartialEq)]
pub struct Decoded {
    /// I messaggi come li scrive WSJT-X, uno per stazione (piu' il CQ).
    pub messages: Vec<String>,
    pub fox: String,
    pub free_text: Option<String>,
    /// La firma digitale (0 se non c'e').
    pub otp: u32,
    pub snr: i32,
    /// La frequenza del tono di sincronismo (Hz audio).
    pub freq_hz: f32,
    /// Lo scarto rispetto all'inizio previsto (s).
    pub dt: f32,
}

/// Decodifica 15 s di audio a 12 kHz (il periodo intero, dall'inizio). `fsync` e `ftol`: dove cercare il tono di sincronismo.
pub fn decode(audio: &[f32], fsync: f32, ftol: f32) -> Option<Decoded> {
    let n9 = 9 * RATE as usize;
    if audio.len() < n9 {
        return None;
    }
    let mut planner = FftPlanner::<f32>::new();
    let f_sync = find_sync_tone(audio, n9, fsync, ftol, &mut planner)?;
    let (t, snr_sync) = find_start(audio, f_sync);
    let fft = planner.plan_fft_forward(NSPS);
    // piccoli spostamenti di frequenza (mezzo hertz) e di tempo (8 campioni) attorno al primo aggancio, se il sincronismo e' buono
    let tries: &[(f32, f32)] = if snr_sync < 4.0 { &[(0.0, 0.0)] } else { &[(0.0, 0.0), (0.0, -1.0), (-0.5, 0.0), (0.0, 1.0), (-0.5, -1.0), (0.5, 0.0), (0.0, -2.0), (-0.5, 1.0), (0.5, -1.0), (-1.0, 0.0)] };
    for (attempt, &(df, dt_steps)) in tries.iter().enumerate() {
        let f = f_sync + df;
        let t = t + dt_steps * 8.0 / NSPS as f32;
        let s3 = spectra(audio, f, t, &fft)?;
        // al primo aggancio si provano anche probabilita' leggermente scosse (`qpc_decode2`, profondita' 3: 100 prove)
        let dithers = if attempt == 0 { DITHERS } else { 1 };
        for b in [0.0f32, 0.4, 0.5, 0.6] {
            let mut s = s3.clone();
            if b > 0.0 {
                for row in s.chunks_mut(qpc::Q) {
                    smooth(row, 1.0, b);
                }
            }
            normalize_median(&mut s);
            let py = likelihoods(&s);
            let mut rng = 0x5eed_u64;
            let Some((x, word)) = (0..dithers).find_map(|k| {
                let pyd = if k == 0 { py.clone() } else { dither(&py, k > 1, &mut rng) };
                qpc::decode(&pyd).filter(|(x, _)| ultrafox::crc_ok(x))
            }) else {
                continue;
            };
            let p: f32 = word.iter().enumerate().map(|(j, &v)| s[j * qpc::Q + usize::from(v)]).sum();
            let snr = 10.0 * (p / 127.0).log10() - 10.0 * 127f32.log10() - 4.0;
            if snr < MIN_SNR {
                return None;
            }
            let mut d = unpack(&x)?;
            d.snr = snr.round() as i32;
            d.freq_hz = f;
            d.dt = t;
            return Some(d);
        }
    }
    None
}

/// Il picco piu' alto dello spettro dei primi 9 s tra `fsync - ftol` e `fsync + ftol`, rifinito col baricentro (`qpc_sync`).
fn find_sync_tone(audio: &[f32], n9: usize, fsync: f32, ftol: f32, planner: &mut FftPlanner<f32>) -> Option<f32> {
    let fft = planner.plan_fft_forward(n9);
    let mut c: Vec<Complex32> = audio[..n9].iter().map(|&v| Complex32::new(v, 0.0)).collect();
    fft.process(&mut c);
    let df = RATE / n9 as f32;
    let mut s: Vec<f32> = c[..n9 / 4].iter().map(|z| z.norm_sqr()).collect();
    for _ in 0..4 {
        smooth(&mut s, 2.0, 1.0);
    }
    let ia = (((fsync - ftol) / df).round() as usize).max(1);
    let ib = (((fsync + ftol) / df).round() as usize).min(s.len() - 1);
    if ia >= ib {
        return None;
    }
    let i0 = (ia..=ib).max_by(|&a, &b| s[a].total_cmp(&s[b]))?;
    let half = (BAUD / df).round() as usize;
    let (lo, hi) = (i0.saturating_sub(half).max(1), (i0 + half).min(s.len() - 1));
    let (s0, s1) = (lo..=hi).fold((0.0f32, 0.0f32), |(s0, s1), i| (s0 + s[i], s1 + (i as f32 - i0 as f32) * s[i]));
    if s0 <= 0.0 {
        return None;
    }
    Some((i0 as f32 + s1 / s0) * df)
}

/// L'inizio del primo simbolo rispetto ai 0,5 s previsti (s) e quanto spicca il sincronismo: energia del tono di sincronismo nei 24
/// simboli giusti, spostando l'inizio di +-1,5 s (`qpc_sync`).
fn find_start(audio: &[f32], f: f32) -> (f32, f32) {
    let w = -std::f32::consts::TAU * f / RATE;
    let n = audio.len() / NDOWN;
    let mut sum = vec![Complex32::new(0.0, 0.0); n + 1];
    for k in 0..n {
        let mut z = Complex32::new(0.0, 0.0);
        for (i, &v) in audio.iter().enumerate().skip(k * NDOWN).take(NDOWN) {
            z += Complex32::from_polar(v, w * i as f32);
        }
        sum[k + 1] = sum[k] + z;
    }
    let nspsd = NSPS / NDOWN;
    let dt = NDOWN as f32 / RATE;
    let lagmax = (1.5 / dt) as i64;
    let i0 = (0.5 * RATE / NDOWN as f32).round() as i64;
    let power = |lag: i64| -> f32 {
        ISYNC
            .iter()
            .filter_map(|&j| {
                let a = i0 + (j as i64 - 1) * nspsd as i64 + lag;
                let b = a + nspsd as i64;
                (a >= 0 && (b as usize) <= n).then(|| (sum[b as usize] - sum[a as usize]).norm_sqr())
            })
            .sum()
    };
    let p: Vec<(i64, f32)> = (-lagmax..=lagmax).map(|lag| (lag, power(lag))).collect();
    let (lag_pk, pmax) = p.iter().copied().max_by(|a, b| a.1.total_cmp(&b.1)).unwrap_or((0, 0.0));
    let tsym = NSPS as f32 / RATE;
    let others: Vec<f32> = p.iter().filter(|(l, _)| ((l - lag_pk) as f32 * dt).abs() >= tsym).map(|(_, v)| *v).collect();
    let snr = if others.is_empty() {
        0.0
    } else {
        let ave = others.iter().sum::<f32>() / others.len() as f32;
        let var = others.iter().map(|v| v * v).sum::<f32>() / others.len() as f32 - ave * ave;
        if var > 0.0 { (pmax - ave) / var.sqrt() } else { 0.0 }
    };
    (lag_pk as f32 * dt, snr)
}

/// L'energia dei 128 toni di dati (dal tono 1) in ognuno dei 127 simboli di dati: `s[j * 128 + v]` per il valore `v` del simbolo `j`.
fn spectra(audio: &[f32], f: f32, t: f32, fft: &Arc<dyn Fft<f32>>) -> Option<Vec<f32>> {
    let j0 = (RATE * (t + 0.5)).round() as i64;
    let w = -std::f32::consts::TAU * f / RATE;
    let mut out = Vec::with_capacity(qpc::NP * qpc::Q);
    let mut buf = vec![Complex32::new(0.0, 0.0); NSPS];
    for n in 1..=TONES {
        if ISYNC.contains(&n) {
            continue;
        }
        let start = j0 + (n as i64 - 1) * NSPS as i64;
        if start < 0 || start as usize + NSPS > audio.len() {
            out.extend(std::iter::repeat_n(1.0, qpc::Q));
            continue;
        }
        let start = start as usize;
        for (k, z) in buf.iter_mut().enumerate() {
            *z = Complex32::from_polar(audio[start + k], w * (start + k) as f32);
        }
        fft.process(&mut buf);
        out.extend(buf[1..=qpc::Q].iter().map(|z| z.norm_sqr()));
    }
    (out.len() == qpc::NP * qpc::Q).then_some(out)
}

/// Media mobile 1-2-1 pesata (`smo121a`): `a` al centro, `b` ai lati.
fn smooth(x: &mut [f32], a: f32, b: f32) {
    let fac = 1.0 / (a + 2.0 * b);
    let mut prev = x[0];
    for i in 1..x.len().saturating_sub(1) {
        let cur = x[i];
        x[i] = fac * (a * cur + b * (prev + x[i + 1]));
        prev = cur;
    }
}

fn normalize_median(s: &mut [f32]) {
    let mut sorted = s.to_vec();
    sorted.sort_by(f32::total_cmp);
    let base = sorted[sorted.len() / 2];
    if base > 0.0 {
        s.iter_mut().for_each(|v| *v /= base);
    }
}

/// Dall'energia alle probabilita' (`qpc_likelihoods2`, Es/No 3,16).
fn likelihoods(s: &[f32]) -> Vec<f32> {
    let norm = 3.16 / 4.16;
    let mut py = Vec::with_capacity(s.len());
    for row in s.chunks(qpc::Q) {
        let max = row.iter().fold(0f32, |m, &v| m.max(norm * v));
        let e: Vec<f32> = row.iter().map(|&v| (norm * v - max).exp()).collect();
        let total: f32 = e.iter().sum();
        py.extend(e.iter().map(|v| v / total));
    }
    py
}

/// Dai 50 simboli (come escono dal codice polare) al testo (`sfox_unpack.f90`).
pub fn unpack(symbols: &[u8; SYMBOLS]) -> Option<Decoded> {
    let mut x = *symbols;
    x.reverse();
    let mut bits = [0u8; 329];
    for (k, &s) in x.iter().take(47).enumerate() {
        for b in 0..7 {
            bits[7 * k + b] = (s >> (6 - b)) & 1;
        }
    }
    let get = |from: usize, width: usize| -> u128 { (0..width).fold(0, |v, i| (v << 1) | u128::from(bits[from - 1 + i])) };
    let i3 = get(327, 3);
    let otp = get(307, 20) as u32;
    let mut messages = Vec::new();
    let mut free_text = None;
    let text = |a: usize, b: usize| -> Option<String> {
        let t = format!("{:>13}{:>13}", ft8::message::unpack_text71(get(a, 71)), ft8::message::unpack_text71(get(b, 71)));
        let t = t.trim().trim_end_matches('.').trim().to_string();
        (!t.is_empty()).then_some(t)
    };
    let fox;
    if i3 == 3 {
        const C: &[u8] = b" 0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ/";
        let mut n58 = get(1, 58);
        let mut call = [b' '; 11];
        for c in call.iter_mut().rev() {
            *c = C[(n58 % 38) as usize];
            n58 /= 38;
        }
        fox = String::from_utf8_lossy(&call).trim().to_string();
        let g = get(59, 15) as u32;
        let grid = format!("{}{}{}{}", char::from(b'A' + (g / 1800) as u8), char::from(b'A' + (g / 100 % 18) as u8), g / 10 % 10, g % 10);
        messages.push(format!("CQ {fox} {grid}"));
        if !(0..7).all(|i| get(74 + 32 * i, 32) == u128::from(NQU1RKS)) {
            free_text = text(74, 145);
        }
        return Some(Decoded { messages, fox, free_text, otp, snr: 0, freq_hz: 0.0, dt: 0.0 });
    }
    if i3 != 0 && i3 != 2 {
        return None;
    }
    fox = ft8::message::unpack28_standard(get(1, 28) as u32)?;
    if i3 == 2 {
        free_text = text(161, 232);
    }
    let reports: Vec<String> = (0..4)
        .map(|i| {
            let n = get(if i3 == 2 { 141 } else { 281 } + 5 * i, 5) as i32;
            if n == 31 { "RR73".to_string() } else { format!("{:+03}", n - 18) }
        })
        .collect();
    let slots = if i3 == 2 { 4 } else { 9 };
    let mut cqs = 0;
    for i in 1..=slots {
        let n28 = get(28 * i + 1, 28) as u32;
        if n28 == 0 || n28 == NQU1RKS {
            continue;
        }
        let Some(call) = ft8::message::unpack28_standard(n28) else { continue };
        if call.starts_with("CQ") {
            cqs += 1;
            if cqs == 1 {
                messages.push(format!("{call} {fox}"));
            }
            continue;
        }
        let tail = if i3 == 2 { reports[i - 1].clone() } else if i <= 5 { "RR73".to_string() } else { reports[i - 6].clone() };
        messages.push(format!("{call} {fox} {tail}"));
    }
    if get(306, 1) == 1 && cqs == 0 {
        messages.push(format!("CQ {fox}"));
    }
    Some(Decoded { messages, fox, free_text, otp, snr: 0, freq_hz: 0.0, dt: 0.0 })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ultrafox::{Content, Extras};

    fn hounds() -> Content {
        Content::Hounds { fox: "IU3ABC".into(), rr73: vec!["K1ABC".into(), "DL1ABC".into()], reports: vec![("JA1XYZ".into(), -12), ("VK2AB".into(), 3)] }
    }

    /// Rumore gaussiano ripetibile (Box-Muller su un generatore lineare).
    fn noise(n: usize, sigma: f32, seed: u64) -> Vec<f32> {
        let mut s = seed;
        let mut next = || {
            s = s.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1_442_695_040_888_963_407);
            ((s >> 11) as f64 / (1u64 << 53) as f64).max(1e-12)
        };
        (0..n).map(|_| (sigma as f64 * (-2.0 * next().ln()).sqrt() * (std::f64::consts::TAU * next()).cos()) as f32).collect()
    }

    /// Il periodo di 15 s con il segnale che parte a `start_s` (il primo simbolo vero un simbolo dopo, per la rampa).
    fn period(content: &Content, extras: &Extras, start_s: f32, amp: f32, sigma: f32) -> Vec<f32> {
        let w = ultrafox::transmission(content, extras).expect("segnale");
        let mut audio = noise(15 * 12_000, sigma, 7);
        let at = (start_s * RATE) as usize;
        for (k, v) in w.iter().enumerate() {
            if let Some(a) = audio.get_mut(at + k) {
                *a += amp * v;
            }
        }
        audio
    }

    #[test]
    fn unpack_gives_back_the_wsjtx_messages() {
        let x = ultrafox::pack(&hounds(), &Extras { more_cqs: true, free_text: None, otp: 0 }).expect("impacchettato");
        let d = unpack(&x).expect("spacchettato");
        assert_eq!(d.messages, vec!["K1ABC IU3ABC RR73", "DL1ABC IU3ABC RR73", "JA1XYZ IU3ABC -12", "VK2AB IU3ABC +03", "CQ IU3ABC"]);
        let cq = ultrafox::pack(&Content::Cq { call: "IU3ABC".into(), grid: "IO91".into() }, &Extras { free_text: Some("QSL VIA LOTW".into()), ..Extras::default() }).expect("CQ");
        let d = unpack(&cq).expect("CQ");
        assert_eq!(d.messages, vec!["CQ IU3ABC IO91"]);
        assert_eq!(d.free_text.as_deref(), Some("QSL VIA LOTW"));
        let txt = ultrafox::pack(&Content::Hounds { fox: "IU3ABC".into(), rr73: vec!["K1ABC".into()], reports: vec![("JA1XYZ".into(), -5)] }, &Extras { free_text: Some("TNX".into()), ..Extras::default() }).expect("testo");
        let d = unpack(&txt).expect("testo");
        assert_eq!(d.messages, vec!["K1ABC IU3ABC RR73", "JA1XYZ IU3ABC -05"]);
        assert_eq!(d.free_text.as_deref(), Some("TNX"));
    }

    #[test]
    fn a_clean_signal_decodes_with_time_offset() {
        let start = 0.5 - NSPS as f32 / RATE + 0.3;
        let audio = period(&hounds(), &Extras::default(), start, 0.5, 0.05);
        let d = decode(&audio, 750.0, 100.0).expect("decodificato");
        assert_eq!(d.fox, "IU3ABC");
        assert_eq!(d.messages.len(), 4);
        assert!((d.dt - 0.3).abs() < 0.02, "dt {}", d.dt);
        assert!((d.freq_hz - 750.0).abs() < 1.0, "f {}", d.freq_hz);
    }

    #[test]
    fn a_weak_signal_in_noise_still_decodes() {
        // ampiezza 0,15 con rumore di varianza 1 su 6 kHz: circa -16 dB in 2500 Hz
        let start = 0.5 - NSPS as f32 / RATE;
        let audio = period(&hounds(), &Extras::default(), start, 0.15, 1.0);
        let d = decode(&audio, 750.0, 100.0).expect("decodificato nel rumore");
        assert_eq!(d.messages[0], "K1ABC IU3ABC RR73");
    }

    #[test]
    #[ignore = "misura della soglia: cargo test --release -- --ignored soglia --nocapture"]
    fn soglia() {
        let start = 0.5 - NSPS as f32 / RATE;
        for amp in [0.2f32, 0.15, 0.12, 0.1, 0.09, 0.08, 0.07] {
            let ok = (0..10).filter(|&seed| {
                let w = ultrafox::transmission(&hounds(), &Extras::default()).expect("segnale");
                let mut audio = noise(15 * 12_000, 1.0, seed);
                for (k, v) in w.iter().enumerate() {
                    audio[(start * RATE) as usize + k] += amp * v;
                }
                decode(&audio, 750.0, 100.0).is_some()
            }).count();
            let snr = 10.0 * ((amp * amp / 2.0) / (2500.0 / 6000.0)).log10();
            println!("{snr:.1} dB: {ok}/10");
        }
    }

    #[test]
    fn noise_alone_gives_nothing() {
        assert!(decode(&noise(15 * 12_000, 1.0, 3), 750.0, 100.0).is_none());
    }
}
