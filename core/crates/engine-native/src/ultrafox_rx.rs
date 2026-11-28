//! Ricezione di UltraFOX (e di SuperFox di WSJT-X 3.3: `sfrx_sub.f90`, `qpc_sync.f90`, `sfox_demod.f90`, `qpc_decode2.f90`,
//! `sfox_unpack.f90`). Si cerca il tono del sincronismo (il piu' forte e fisso, a circa 750 Hz), si trova l'inizio con i 24 simboli di
//! sincronismo, si misura l'energia dei 128 toni di dati in ognuno dei 127 simboli, si passa alle probabilita' e al codice polare e si
//! controlla l'integrita'. Soglia misurata (prova `soglia`): sicura a -16 dB in 2500 Hz, rara sotto -17,5 dB. Il messaggio torna nella forma di WSJT-X («K1ABC IU3ABC RR73», «K1ABC IU3ABC -12», «CQ IU3ABC IO91»).

use std::sync::Arc;

use rustfft::{num_complex::Complex32, Fft, FftPlanner};

use ft8::message::CallHashes;

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

/// Spostamenti di frequenza (in mezzi hertz) e di tempo (in passi di 8 campioni) attorno al primo aggancio: i 100 di `qpc_decode2.f90`
/// (`idf` e `idt`), nello stesso ordine; il primo e' l'aggancio stesso.
const IDF: [i8; 100] = [0,0,-1,0,-1,1,0,-1,1,-2,0,-1,1,-2,2,0,-1,1,-2,2,-3,0,-1,1,-2,2,-3,3,0,-1,1,-2,2,-3,3,-4,0,-1,1,-2,2,-3,3,-4,4,0,-1,1,-2,2,-3,3,-4,4,-5,-1,1,-2,2,-3,3,-4,4,-5,1,-2,2,-3,3,-4,4,-5,-2,2,-3,3,-4,4,-5,2,-3,3,-4,4,-5,-3,3,-4,4,-5,3,-4,4,-5,-4,4,-5,4,-5,-5];
const IDT: [i8; 100] = [0,-1,0,1,-1,0,-2,1,-1,0,2,-2,1,-1,0,-3,2,-2,1,-1,0,3,-3,2,-2,1,-1,0,-4,3,-3,2,-2,1,-1,0,4,-4,3,-3,2,-2,1,-1,0,-5,4,-4,3,-3,2,-2,1,-1,0,-5,4,-4,3,-3,2,-2,1,-1,-5,4,-4,3,-3,2,-2,1,-5,4,-4,3,-3,2,-2,-5,4,-4,3,-3,2,-5,4,-4,3,-3,-5,4,-4,3,-5,4,-4,-5,4,-5];

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
    normalize_rows(&mut out);
    out
}

/// Ogni riga (i 128 valori di un simbolo) si riporta a somma 1; una riga a somma zero resta com'e'.
fn normalize_rows(p: &mut [f32]) {
    for row in p.chunks_mut(qpc::Q) {
        let total: f32 = row.iter().sum();
        if total > 0.0 {
            row.iter_mut().for_each(|v| *v /= total);
        }
    }
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
/// Prima toglie i segnali FT8 e la portante fissa (`ultrafox_clean`, come `sfrx_sub.f90`).
pub fn decode(audio: &[f32], fsync: f32, ftol: f32) -> Option<Decoded> {
    decode_with(audio, fsync, ftol, &CallHashes::default())
}

/// Come [`decode`] con la memoria degli hash dei nominativi: i nominativi non standard («<IU3ABC/P>») tornano per nome se gia' noti.
/// I segnali FT8 da togliere si cercano con il decodificatore leggero.
pub fn decode_with(audio: &[f32], fsync: f32, ftol: f32, hashes: &CallHashes) -> Option<Decoded> {
    decode_with_known(audio, fsync, ftol, hashes, None)
}

/// Come [`decode_with`], ma con i messaggi FT8 che il decodificatore FT8 ha gia' trovato in questo periodo (`Some`): si sottraggono
/// quelli, senza decodificare FT8 una seconda volta. Con `None` si usa il decodificatore leggero.
pub fn decode_with_known(audio: &[f32], fsync: f32, ftol: f32, hashes: &CallHashes, ft8: Option<&[ft8::decode::Decoded]>) -> Option<Decoded> {
    match crate::ultrafox_clean::clean(audio, fsync, ft8, hashes).audio {
        Some(cleaned) => decode_raw(&cleaned, fsync, ftol, hashes),
        None => decode_raw(audio, fsync, ftol, hashes),
    }
}

/// La decodifica vera e propria, sull'audio cosi' com'e' (senza togliere FT8 ne' portanti).
pub fn decode_raw(audio: &[f32], fsync: f32, ftol: f32, hashes: &CallHashes) -> Option<Decoded> {
    let n9 = 9 * RATE as usize;
    if audio.len() < n9 {
        return None;
    }
    let mut planner = FftPlanner::<f32>::new();
    let f_sync = find_sync_tone(audio, n9, fsync, ftol, &mut planner)?;
    let (t, snr_sync) = find_start(audio, f_sync);
    let fft = planner.plan_fft_forward(NSPS);
    // fino a 100 spostamenti di frequenza (mezzo hertz) e di tempo (8 campioni) attorno al primo aggancio, se il sincronismo e' buono
    let max_tries = if snr_sync < 4.0 { 1 } else { IDF.len() };
    for attempt in 0..max_tries {
        let f = f_sync + f32::from(IDF[attempt]) * 0.5;
        let t = t + f32::from(IDT[attempt]) * 8.0 / NSPS as f32;
        let s3 = spectra(audio, f, t, &fft)?;
        // al primo aggancio si provano anche probabilita' scosse (`qpc_decode2`, profondita' 3: 100 prove)
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
                // come `qpc_decode2`: la prima prova usa le energie normalizzate (`pyd=py0`), la seconda le probabilita', poi quelle scosse
                let pyd = match k {
                    0 => {
                        let mut p = s.clone();
                        normalize_rows(&mut p);
                        p
                    }
                    1 => py.clone(),
                    _ => dither(&py, true, &mut rng),
                };
                qpc::decode(&pyd).filter(|(x, _)| ultrafox::crc_ok(x))
            }) else {
                continue;
            };
            let p: f32 = word.iter().enumerate().map(|(j, &v)| s[j * qpc::Q + usize::from(v)]).sum();
            let snr = 10.0 * (p / 127.0).log10() - 10.0 * 127f32.log10() - 4.0;
            if snr < MIN_SNR {
                return None;
            }
            let mut d = unpack_with(&x, hashes)?;
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
/// Come `sfox_demod.f90`: gli spettri di tutti i 151 simboli (sincronismo compreso) si normalizzano con la mediana e si cercano le portanti
/// fisse («birdie»), che si azzerano.
fn spectra(audio: &[f32], f: f32, t: f32, fft: &Arc<dyn Fft<f32>>) -> Option<Vec<f32>> {
    let j0 = (RATE * (t + 0.5)).round() as i64;
    let w = -std::f32::consts::TAU * f / RATE;
    let q = qpc::Q;
    // colonna 0: il simbolo bucato, sempre a zero; poi i 151 simboli (anche quelli di sincronismo)
    let mut s2 = vec![0.0f32; (TONES + 1) * q];
    let mut buf = vec![Complex32::new(0.0, 0.0); NSPS];
    for n in 1..=TONES {
        let start = j0 + (n as i64 - 1) * NSPS as i64;
        if start < 0 || start as usize + NSPS > audio.len() {
            continue;
        }
        let start = start as usize;
        for (k, z) in buf.iter_mut().enumerate() {
            *z = Complex32::from_polar(audio[start + k], w * (start + k) as f32);
        }
        fft.process(&mut buf);
        for (o, z) in s2[n * q..(n + 1) * q].iter_mut().zip(&buf[1..=q]) {
            *o = z.norm_sqr();
        }
    }
    // mediana dei primi 151 simboli (`pctile(s2,NQ*151,50,base2)`: la colonna 0 e i simboli da 1 a 150)
    let base2 = median(&s2[..TONES * q]);
    if base2 > 0.0 {
        s2.iter_mut().for_each(|v| *v /= base2);
    }
    remove_birdies(&mut s2);
    let mut out = Vec::with_capacity(qpc::NP * q);
    for n in 1..=TONES {
        if !ISYNC.contains(&n) {
            out.extend_from_slice(&s2[n * q..(n + 1) * q]);
        }
    }
    (out.len() == qpc::NP * q).then_some(out)
}

/// La mediana come `pctile(x, n, 50)`.
fn median(x: &[f32]) -> f32 {
    let mut sorted = x.to_vec();
    sorted.sort_by(f32::total_cmp);
    sorted.get((sorted.len() / 2).saturating_sub(1)).copied().unwrap_or(0.0)
}

/// Soppressione delle portanti fisse (`sfox_demod.f90:35-66`): per ogni simbolo si prende il tono piu' forte (da 1 a 127: l'indice del
/// Fortran, che conta da 0, e' uno meno), si fa l'istogramma e si porta a 1,0 (il livello del rumore) la colonna di un tono che e' il
/// picco in piu' di 12 simboli, e le 8 colonne a partire da quattro adiacenti che lo sono in piu' di 20.
fn remove_birdies(s2: &mut [f32]) {
    let q = qpc::Q;
    let mut hist1 = [0u32; 128];
    for col in s2.chunks(q) {
        // `maxloc(s2(1:NQ-1,j))`: la prima posizione del massimo
        let mut pos = 1;
        for i in 2..q {
            if col[i] > col[pos] {
                pos = i;
            }
        }
        hist1[pos - 1] += 1;
    }
    hist1[0] = 0; // il tono di sincronismo non e' una portante
    let hist2: Vec<u32> = (0..=123).map(|i| hist1[i..i + 4].iter().sum()).collect();
    let m1 = hist1.iter().copied().max().unwrap_or(0);
    let (mut i2, m2) = hist2.iter().enumerate().fold((0usize, 0u32), |b, (i, &v)| if v > b.1 { (i, v) } else { b });
    if m1 > 12 {
        for (i, &h) in hist1.iter().enumerate() {
            if h > 12 {
                s2.chunks_mut(q).for_each(|col| col[i] = 1.0);
            }
        }
    }
    if m2 > 20 {
        if i2 >= 1 {
            i2 -= 1;
        }
        i2 = i2.min(120);
        s2.chunks_mut(q).for_each(|col| col[i2..i2 + 8].fill(1.0));
    }
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

impl Decoded {
    /// La riga di verifica che WSJT-X scrive dopo ogni pacchetto (`sfox_unpack.f90` con la firma sempre richiesta da `sfrx_sub.f90`):
    /// `$VERIFY$ CALL 123456`, anche con codice `000000`; il Hound la nasconde o la fa verificare (`features::parse_superfox_verify`).
    pub fn verify_line(&self) -> String {
        format!("$VERIFY$ {} {:06}", self.fox, self.otp)
    }
}

/// Dai 50 simboli (come escono dal codice polare) al testo (`sfox_unpack.f90`).
pub fn unpack(symbols: &[u8; SYMBOLS]) -> Option<Decoded> {
    unpack_with(symbols, &CallHashes::default())
}

/// Come [`unpack`] con la memoria degli hash (`unpack28_for_state`).
pub fn unpack_with(symbols: &[u8; SYMBOLS], hashes: &CallHashes) -> Option<Decoded> {
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
    // `sfox_unpack.f90`: i3 = 1 non e' mai trasmesso e non si legge; gli i3 da 4 a 7 si trattano come i3 = 0 (nove stazioni)
    if i3 == 1 {
        return None;
    }
    let i3 = if i3 == 2 { 2 } else { 0 };
    fox = ft8::message::unpack28_with(get(1, 28) as u32, hashes)?;
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
        let Some(call) = ft8::message::unpack28_with(n28, hashes) else { continue };
        if call == "CQ" {
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


    /// Nominativi non standard come in WSJT-X: fra «<>» vanno nell'hash a 22 bit e tornano per nome solo se l'hash e' noto; senza «<>» sono
    /// rifiutati (il controllo `valid_sfox_call` rilegge «<K1ABC/P>» e lo trova diverso).
    #[test]
    fn non_standard_calls_use_the_22_bit_hash_when_in_angle_brackets() {
        let c = Content::Hounds { fox: "IU3ABC".into(), rr73: vec!["<K1ABC/P>".into()], reports: vec![("<DL1ABC/QRP>".into(), -5)] };
        let mut known = CallHashes::default();
        let x = ultrafox::pack_with(&c, &Extras::default(), &mut known).expect("impacchettato");
        assert!(ultrafox::crc_ok(&x));
        assert_eq!(unpack(&x).expect("senza hash").messages, vec!["<...> IU3ABC RR73", "<...> IU3ABC -05"]);
        assert_eq!(unpack_with(&x, &known).expect("con hash").messages, vec!["<K1ABC/P> IU3ABC RR73", "<DL1ABC/QRP> IU3ABC -05"]);
        let plain = Content::Hounds { fox: "IU3ABC".into(), rr73: vec!["K1ABC/P".into()], reports: vec![] };
        assert_eq!(ultrafox::pack(&plain, &Extras::default()), Err(ultrafox::PackError::BadCall));
        let broken = Content::Hounds { fox: "IU3ABC".into(), rr73: vec!["<K1ABC".into()], reports: vec![] };
        assert_eq!(ultrafox::pack(&broken, &Extras::default()), Err(ultrafox::PackError::BadCall));
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


    /// Misura del tempo peggiore: sincronismo buono ma messaggio non decodificabile, cosi' si fanno tutte le 100 prove.
    #[test]
    #[ignore = "misura del tempo: cargo test --release -p engine-native -- --ignored tempo --nocapture"]
    fn tempo() {
        let start = 0.5 - NSPS as f32 / RATE;
        for amp in [0.2f32, 0.1, 0.07, 0.06, 0.05] {
            let audio = period(&hounds(), &Extras::default(), start, amp, 1.0);
            let t0 = std::time::Instant::now();
            let r = decode(&audio, 750.0, 100.0);
            let f = find_sync_tone(&audio, 9 * RATE as usize, 750.0, 100.0, &mut FftPlanner::<f32>::new()).unwrap_or(0.0);
            println!("amp {amp}: {:?} in {} ms, snr sync {:.1}", r.map(|d| d.snr), t0.elapsed().as_millis(), find_start(&audio, f).1);
        }
    }

    /// Le 100 prove di frequenza e di tempo sono quelle di `qpc_decode2.f90`: la prima e' l'aggancio, nessuna coppia si ripete.
    #[test]
    fn the_hundred_frequency_and_time_tries_are_the_wsjtx_ones() {
        assert_eq!((IDF.len(), IDT.len()), (100, 100));
        assert_eq!((IDF[0], IDT[0]), (0, 0));
        assert_eq!((IDF[99], IDT[99]), (-5, -5));
        let mut pairs: Vec<(i8, i8)> = IDF.iter().copied().zip(IDT.iter().copied()).collect();
        pairs.sort_unstable();
        pairs.dedup();
        assert_eq!(pairs.len(), 100);
        assert!(IDF.iter().all(|f| (-5..=4).contains(f)) && IDT.iter().all(|t| (-5..=4).contains(t)));
    }

    /// Una portante fissa dentro i 1500 Hz del segnale sarebbe il picco di quasi ogni simbolo: la soppressione dei «birdie» di
    /// `sfox_demod.f90` la toglie e il messaggio si decodifica.
    #[test]
    fn a_steady_carrier_in_the_band_does_not_stop_the_decoding() {
        let start = 0.5 - NSPS as f32 / RATE;
        let mut audio = period(&hounds(), &Extras::default(), start, 0.3, 0.05);
        // 1,0 di ampiezza contro 0,3 del segnale, su un tono dei dati (circa 750 + 40 baud)
        let hz = 750.0 + 40.4 * BAUD;
        for (k, a) in audio.iter_mut().enumerate() {
            *a += (std::f32::consts::TAU * hz * k as f32 / RATE).sin();
        }
        let d = decode(&audio, 750.0, 100.0).expect("decodificato con la portante");
        assert_eq!(d.messages[0], "K1ABC IU3ABC RR73");
    }

    /// Un segnale FT8 (messaggio vero, codice LDPC e GFSK) che comincia a `dt` secondi, sommato al periodo.
    fn add_ft8(audio: &mut [f32], text: &str, hz: f32, amp: f32, dt: f32) {
        let w = ft8::modem::encode_message(text, hz, &mut CallHashes::default()).expect("onda FT8");
        let at = (dt * RATE) as usize;
        for (k, v) in w.iter().enumerate() {
            if let Some(a) = audio.get_mut(at + k) {
                *a += amp * v;
            }
        }
    }

    /// Il periodo della Fox a ampiezza 0,3 con poco rumore, senza altro.
    fn fox_period() -> Vec<f32> {
        period(&hounds(), &Extras::default(), 0.5 - NSPS as f32 / RATE, 0.3, 0.05)
    }

    /// `sfox_remove_ft8`: un FT8 forte dentro i 100 Hz del sincronismo ruba il picco e la Fox non si aggancia; tolto l'FT8 si decodifica.
    /// La prova richiede che senza pulizia non decodifichi, altrimenti non proverebbe niente.
    #[test]
    fn a_strong_ft8_signal_near_the_sync_tone_is_removed() {
        let mut audio = fox_period();
        add_ft8(&mut audio, "CQ IU3ABC IO91", 790.0, 1.0, 0.5);
        let hashes = CallHashes::default();
        assert!(decode_raw(&audio, 750.0, 100.0, &hashes).is_none(), "senza pulizia dovrebbe fallire");
        let d = decode(&audio, 750.0, 100.0).expect("decodificato dopo aver tolto l'FT8");
        assert_eq!(d.messages[0], "K1ABC IU3ABC RR73");
        // lo stesso con l'elenco gia' decodificato dal decodificatore FT8 completo (il percorso del Hound)
        let mut o = ft8::decode::Options::depth(1);
        o.threads = 1;
        let found = ft8::decode::decode_slot(&audio, &o, &mut CallHashes::default());
        assert!(found.iter().any(|f| f.text == "CQ IU3ABC IO91"), "{found:?}");
        let d = decode_with_known(&audio, 750.0, 100.0, &hashes, Some(&found)).expect("decodificato con l'elenco noto");
        assert_eq!(d.messages[0], "K1ABC IU3ABC RR73");
    }

    /// `sfox_remove_tone`: una portante forte vicino al sincronismo (non esattamente a 750 Hz) lo copre; tolta la portante si decodifica.
    #[test]
    fn a_steady_carrier_near_the_sync_tone_is_removed() {
        let mut audio = fox_period();
        for (k, a) in audio.iter_mut().enumerate() {
            *a += (std::f32::consts::TAU * 781.3 * k as f32 / RATE).sin();
        }
        assert!(decode_raw(&audio, 750.0, 100.0, &CallHashes::default()).is_none(), "senza pulizia dovrebbe fallire");
        let (hz, _) = crate::ultrafox_clean::remove_tone(&audio, 750.0).expect("portante trovata");
        assert!((hz - 781.3).abs() < 0.2, "{hz}");
        let d = decode(&audio, 750.0, 100.0).expect("decodificato dopo aver tolto la portante");
        assert_eq!(d.messages[0], "K1ABC IU3ABC RR73");
    }

    /// Una Fox sola non deve essere toccata: il mini decodificatore non trova FT8 e il sincronismo (acceso e spento) non e' una portante.
    #[test]
    fn a_fox_alone_is_left_untouched() {
        let audio = fox_period();
        assert!(crate::ultrafox_clean::mini_decode(&audio, &CallHashes::default()).is_empty());
        assert!(crate::ultrafox_clean::remove_tone(&audio, 750.0).is_none());
        let c = crate::ultrafox_clean::clean(&audio, 750.0, None, &CallHashes::default());
        assert!(c.audio.is_none() && c.ft8_removed == 0 && c.tone_hz.is_none());
        assert!(decode(&audio, 750.0, 100.0).is_some());
        // anche nel rumore forte, dove le false portanti sarebbero piu' facili
        let noisy = period(&hounds(), &Extras::default(), 0.5 - NSPS as f32 / RATE, 0.3, 1.0);
        assert!(crate::ultrafox_clean::remove_tone(&noisy, 750.0).is_none());
    }

    /// Il rumore solo non produce falsi FT8 da togliere ne' portanti.
    #[test]
    fn noise_alone_has_nothing_to_remove() {
        let audio = noise(15 * 12_000, 1.0, 11);
        let c = crate::ultrafox_clean::clean(&audio, 750.0, None, &CallHashes::default());
        assert!(c.audio.is_none(), "{c:?}");
    }

    /// Messaggi che non si ricostruiscono (nominativo in hash non risolto, testo a priori) non si sottraggono.
    #[test]
    fn unresolvable_ft8_texts_are_not_subtracted() {
        let mut audio = vec![0.0f32; 15 * 12_000];
        let d = |text: &str, hint: bool| ft8::decode::Decoded { freq_hz: 1000.0, dt_s: 0.0, snr_db: -10, text: text.into(), hint };
        let list = [d("<...> IU3ABC RR73", false), d("CQ IU3ABC IO91", true), d("CQ IU3ABC IO91", false)];
        // solo l'ultimo e' valido: il segnale non c'e' nell'audio, ma la sottrazione si conta lo stesso
        assert_eq!(crate::ultrafox_clean::remove_ft8(&mut audio, &list, &CallHashes::default()), 1);
        let far = ft8::decode::Decoded { freq_hz: 3000.0, ..d("CQ IU3ABC IO91", false) };
        assert_eq!(crate::ultrafox_clean::remove_ft8(&mut audio, &[far], &CallHashes::default()), 0);
    }

    /// Misura dei tempi della pulizia: cargo test --release -j 2 -p engine-native pulizia -- --ignored --nocapture --test-threads=1
    #[test]
    #[ignore = "misura del tempo"]
    fn pulizia() {
        use std::time::Instant;
        let hashes = CallHashes::default();
        let texts = ["CQ IU3ABC IO91", "K1ABC DL1ABC JO62", "JA1XYZ VK2AB -12", "DL1ABC K1ABC RR73", "CQ DX VK2AB QF56", "IU3ABC JA1XYZ R-05", "W1AW K1ABC FN31", "OH2BH DL1ABC 73"];
        let mut busy = noise(15 * 12_000, 0.05, 5);
        for (i, t) in texts.iter().enumerate() {
            add_ft8(&mut busy, t, 600.0 + 230.0 * i as f32, 0.2, 0.3 + 0.1 * i as f32);
        }
        let fox = {
            let mut a = fox_period();
            add_ft8(&mut a, "CQ IU3ABC IO91", 790.0, 1.0, 0.5);
            a
        };
        let runs = 3;
        let time = |f: &dyn Fn()| {
            let t = Instant::now();
            (0..runs).for_each(|_| f());
            t.elapsed().as_secs_f64() * 1000.0 / runs as f64
        };
        let mut o = ft8::decode::Options::depth(1);
        o.threads = 1;
        let full = time(&|| {
            ft8::decode::decode_slot(&busy, &o, &mut CallHashes::default());
        });
        let found = ft8::decode::decode_slot(&busy, &o, &mut CallHashes::default());
        println!("FT8 completo (profondita' 1, un thread): {full:.0} ms, {} messaggi", found.len());
        let mini = time(&|| {
            crate::ultrafox_clean::mini_decode(&busy, &hashes);
        });
        let mini_found = crate::ultrafox_clean::mini_decode(&busy, &hashes);
        println!("mini decodificatore FT8: {mini:.0} ms, {} messaggi", mini_found.len());
        let sub = time(&|| {
            let mut c = busy.clone();
            crate::ultrafox_clean::remove_ft8(&mut c, &found, &hashes);
        });
        println!("sottrazione di {} FT8 gia' noti: {sub:.0} ms", found.len());
        let tone = time(&|| {
            crate::ultrafox_clean::remove_tone(&busy, 750.0);
        });
        println!("ricerca/rimozione portante: {tone:.1} ms");
        let before = time(&|| {
            decode_raw(&busy, 750.0, 200.0, &hashes);
        });
        let known = time(&|| {
            decode_with_known(&busy, 750.0, 200.0, &hashes, Some(&found));
        });
        let own = time(&|| {
            decode_with_known(&busy, 750.0, 200.0, &hashes, None);
        });
        println!("periodo affollato, senza Fox: prima {before:.0} ms, ora con elenco noto {known:.0} ms, con mini decodificatore {own:.0} ms");
        let fb = time(&|| {
            decode_raw(&fox, 750.0, 100.0, &hashes);
        });
        let fa = time(&|| {
            decode(&fox, 750.0, 100.0);
        });
        println!("Fox con FT8 vicino: prima (fallisce) {fb:.0} ms, ora (decodifica) {fa:.0} ms");
    }

    #[test]
    fn birdie_columns_are_set_to_the_noise_level() {
        let q = qpc::Q;
        let mut s2 = vec![0.5f32; 152 * q];
        // il tono 40 e' il picco in 30 simboli
        for n in 1..=30 {
            s2[n * q + 40] = 9.0;
        }
        remove_birdies(&mut s2);
        assert!(s2.chunks(q).all(|c| c[40] == 1.0));
        assert!(s2.chunks(q).all(|c| c[10] != 1.0));
    }

    /// Come `sfox_unpack.f90`: i3 da 4 a 7 si leggono come i3 = 0, i3 = 1 non si legge; «CQ_DX» non e' il CQ ma un nominativo come gli altri.
    #[test]
    fn i3_from_4_to_7_is_read_as_zero_and_cq_needs_the_exact_word() {
        let x = ultrafox::pack(&hounds(), &Extras { more_cqs: true, free_text: None, otp: 0 }).expect("impacchettato");
        // si riscrivono i bit del messaggio (329) e si rifa' il controllo d'integrita' sui simboli
        let rebuild = |edit: &dyn Fn(&mut [u8; 329])| {
            let mut y = x;
            y.reverse();
            let mut bits = [0u8; 329];
            for (k, &s) in y.iter().take(47).enumerate() {
                (0..7).for_each(|b| bits[7 * k + b] = (s >> (6 - b)) & 1);
            }
            edit(&mut bits);
            for (k, s) in y.iter_mut().take(47).enumerate() {
                *s = (0..7).fold(0, |v, b| (v << 1) | bits[7 * k + b]);
            }
            let crc = ultrafox::hashlittle(&y[..47], 571) & ((1 << 21) - 1);
            y[47] = (crc >> 14) as u8;
            y[48] = ((crc >> 7) & 127) as u8;
            y[49] = (crc & 127) as u8;
            y.reverse();
            y
        };
        let put = |bits: &mut [u8; 329], from: usize, width: usize, v: u32| (0..width).for_each(|i| bits[from - 1 + i] = ((v >> (width - 1 - i)) & 1) as u8);
        let with_i3 = |i3: u32| rebuild(&|bits| put(bits, 327, 3, i3));
        let base = unpack(&with_i3(0)).expect("i3 0").messages;
        for i3 in 4..=7 {
            assert_eq!(unpack(&with_i3(i3)).expect("i3 da 4 a 7").messages, base, "i3 {i3}");
        }
        assert!(unpack(&with_i3(1)).is_none());
        // primo nominativo «CQ_DX» (3 + 1000 + 132): un nominativo qualunque, con RR73
        let cq_dx = rebuild(&|bits| put(bits, 29, 28, 3 + 1000 + 132));
        let m = unpack(&cq_dx).expect("con CQ_DX").messages;
        assert_eq!(m[0], "CQ_DX IU3ABC RR73", "{m:?}");
        // il vero «CQ» (n28 = 2) resta un CQ, senza RR73
        let cq = rebuild(&|bits| put(bits, 29, 28, 2));
        assert_eq!(unpack(&cq).expect("con CQ").messages[0], "CQ IU3ABC");
    }
    #[test]
    fn noise_alone_gives_nothing() {
        assert!(decode(&noise(15 * 12_000, 1.0, 3), 750.0, 100.0).is_none());
    }
}
