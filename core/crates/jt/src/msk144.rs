//! MSK144: meteor scatter. Messaggio di 77 bit come FT8 con CRC a 13 bit, codice LDPC (128,90), quadro di 144 bit (due parole di
//! sincronismo da 8 bit) in MSK a 2000 baud: 72 ms per quadro, ripetuto per tutto il periodo. Dai sorgenti di WSJT-X (GPL 3):
//! `genmsk_128_90.f90`, `encode_128_90.f90`, `crc13.cpp`, `bpdecode128_90.f90`, `msk144decodeframe.f90`, `msk144sync.f90`,
//! `msk144_freq_search.f90`, `msk144sim.f90`.
//!
//! La ricezione cerca i «ping» (finestre di uno-tre quadri piu' forti del rumore), ne media i quadri, trova frequenza e inizio del quadro
//! con le parole di sincronismo e decodifica con la propagazione delle credenze. I messaggi brevi (MSK40, `<...>`) non ci sono ancora.

use ft8::message::{Bits77, CallHashes, pack, unpack};
use rustfft::{FftPlanner, num_complex::Complex};

use crate::msk144tables::{GEN_HEX, MN, NM, NRW};

const FS: f64 = 12_000.0;
/// Campioni per quadro (144 bit da 6 campioni).
pub const NSPM: usize = 864;
/// Velocita' di simbolo.
pub const BAUD: f64 = 2_000.0;
const N: usize = 128;
const K: usize = 90;
const M: usize = N - K;
/// Parola di sincronismo.
const S8: [i8; 8] = [0, 1, 1, 1, 0, 0, 1, 0];

/// CRC a 13 bit (polinomio 0x15D7) dei 77 bit seguiti da 19 zeri, come `boost::augmented_crc<13>` su 12 byte.
fn crc13(bits77: &[u8]) -> u16 {
    const POLY: u16 = 0x15D7;
    const MASK: u16 = 0x1FFF;
    let mut reg = 0u16;
    let mut push = |b: u8| {
        let carry = (reg >> 12) & 1;
        reg = ((reg << 1) | u16::from(b & 1)) & MASK;
        if carry == 1 {
            reg ^= POLY;
        }
    };
    for &b in bits77 {
        push(b);
    }
    for _ in 0..19 {
        push(0);
    }
    reg
}

fn generator() -> [[u8; K]; M] {
    let mut g = [[0u8; K]; M];
    for (i, row) in GEN_HEX.iter().enumerate() {
        for (j, c) in row.chars().enumerate() {
            let v = c.to_digit(16).unwrap_or(0);
            let nb = if j == 22 { 2 } else { 4 };
            for jj in 0..nb {
                if (v >> (3 - jj)) & 1 == 1 {
                    g[i][4 * j + jj] = 1;
                }
            }
        }
    }
    g
}

/// `encode_128_90`: 77 bit -> 128 bit (77 di messaggio, 13 di CRC, 38 di parita').
fn encode(bits: &Bits77) -> [u8; N] {
    let mut msg = [0u8; K];
    msg[..77].copy_from_slice(bits);
    let crc = crc13(bits);
    for i in 0..13 {
        msg[77 + i] = ((crc >> (12 - i)) & 1) as u8;
    }
    let g = generator();
    let mut cw = [0u8; N];
    cw[..K].copy_from_slice(&msg);
    for i in 0..M {
        cw[K + i] = g[i].iter().zip(&msg).map(|(a, b)| a & b).sum::<u8>() % 2;
    }
    cw
}

/// I 144 bit del quadro: sincronismo, 48 bit di codice, sincronismo, 80 bit di codice.
fn frame_bits(cw: &[u8; N]) -> [i8; 144] {
    let mut b = [0i8; 144];
    b[..8].copy_from_slice(&S8);
    for i in 0..48 {
        b[8 + i] = cw[i] as i8;
    }
    b[56..64].copy_from_slice(&S8);
    for i in 0..80 {
        b[64 + i] = cw[48 + i] as i8;
    }
    b
}

/// I 144 toni (0 = frequenza bassa, 1 = alta) di `genmsk_128_90`. Errore se il testo non e' un messaggio a 77 bit.
pub fn tones(text: &str, hashes: &mut CallHashes) -> Result<[u8; 144], String> {
    let bits = pack(text, hashes).map_err(|e| format!("{e:?}"))?;
    let seq: Vec<i32> = frame_bits(&encode(&bits)).iter().map(|&v| 2 * i32::from(v) - 1).collect();
    let mut t = [0i32; 144];
    for i in 1..=72 {
        t[2 * i - 2] = (seq[2 * i - 1] * seq[2 * i - 2] + 1) / 2;
        t[2 * i - 1] = -(seq[2 * i - 1] * seq[(2 * i) % 144] - 1) / 2;
    }
    let mut out = [0u8; 144];
    for (o, v) in out.iter_mut().zip(t) {
        *o = (1 - v) as u8;
    }
    Ok(out)
}

/// L'onda a 12 kHz (fase continua) di `frames` quadri: `f_center` e' la frequenza centrale (toni a +-500 Hz).
pub fn modulate(tones: &[u8; 144], f_center: f64, frames: usize, amplitude: f32) -> Vec<f32> {
    let two_pi = 2.0 * std::f64::consts::PI;
    let d0 = two_pi * (f_center - 0.25 * BAUD) / FS;
    let d1 = two_pi * (f_center + 0.25 * BAUD) / FS;
    let mut out = Vec::with_capacity(frames * NSPM);
    let mut phi = 0.0f64;
    for _ in 0..frames {
        for &t in tones {
            let d = if t == 0 { d0 } else { d1 };
            for _ in 0..6 {
                phi = (phi + d) % two_pi;
                out.push(amplitude * phi.sin() as f32);
            }
        }
    }
    out
}

// ---------------------------------------------------------------- decodifica

/// `bpdecode128_90`: propagazione delle credenze; ritorna i 77 bit se il CRC torna, con gli errori sui bit duri.
fn bp_decode(llr: &[f32; N], max_iter: usize) -> Option<(Bits77, usize)> {
    let mut tov = [[0f32; 3]; N];
    let mut toc = [[0f32; 11]; M];
    for j in 0..M {
        for i in 0..NRW[j] as usize {
            toc[j][i] = llr[NM[j][i] as usize - 1];
        }
    }
    let (mut ncnt, mut nclast) = (0usize, 0usize);
    let mut zn = [0f32; N];
    let mut cw = [0u8; N];
    for iter in 0..=max_iter {
        for i in 0..N {
            zn[i] = llr[i] + tov[i].iter().sum::<f32>();
            cw[i] = u8::from(zn[i] > 0.0);
        }
        let ncheck = (0..M).filter(|&j| (0..NRW[j] as usize).map(|i| cw[NM[j][i] as usize - 1] as usize).sum::<usize>() % 2 != 0).count();
        if ncheck == 0 {
            let crc = crc13(&cw[..77]);
            let got = cw[77..90].iter().fold(0u16, |a, &b| (a << 1) | u16::from(b));
            if crc == got {
                let mut bits = [0u8; 77];
                bits.copy_from_slice(&cw[..77]);
                let hard = (0..N).filter(|&i| (2.0 * f32::from(cw[i]) - 1.0) * llr[i] < 0.0).count();
                return Some((bits, hard));
            }
        }
        if iter > 0 {
            if ncheck < nclast { ncnt = 0 } else { ncnt += 1 }
            if ncnt >= 3 && iter >= 5 && ncheck > 10 {
                return None;
            }
        }
        nclast = ncheck;
        for j in 0..M {
            for i in 0..NRW[j] as usize {
                let ibj = NM[j][i] as usize - 1;
                toc[j][i] = zn[ibj];
                for kk in 0..3 {
                    if MN[ibj][kk] as usize == j + 1 {
                        toc[j][i] -= tov[ibj][kk];
                    }
                }
            }
        }
        let mut tanhtoc = [[0f32; 11]; M];
        for j in 0..M {
            for i in 0..11 {
                tanhtoc[j][i] = (-toc[j][i] / 2.0).tanh();
            }
        }
        for j in 0..N {
            for i in 0..3 {
                let ichk = MN[j][i] as usize - 1;
                let mut t = 1f32;
                for k in 0..NRW[ichk] as usize {
                    if NM[ichk][k] as usize != j + 1 {
                        t *= tanhtoc[ichk][k];
                    }
                }
                // platanh di WSJT-X: arcotangente iperbolica limitata
                let x = -t;
                let y = if x.abs() <= 0.664 {
                    x / 0.83
                } else if x.abs() <= 0.9217 {
                    x.signum() * (x.abs() - 0.4064) / 0.322
                } else if x.abs() <= 0.9951 {
                    x.signum() * (x.abs() - 0.8378) / 0.0524
                } else if x.abs() <= 0.9998 {
                    x.signum() * (x.abs() - 0.9914) / 0.0012
                } else {
                    x.signum() * 7.0
                };
                tov[j][i] = 2.0 * y;
            }
        }
    }
    None
}

/// Mezza sinusoide del simbolo (12 campioni).
fn pp() -> [f32; 12] {
    let mut p = [0f32; 12];
    for (i, v) in p.iter_mut().enumerate() {
        *v = (i as f32 * std::f32::consts::PI / 12.0).sin();
    }
    p
}

/// La forma complessa della parola di sincronismo (`cb`, 42 campioni).
fn sync_wave() -> [Complex<f32>; 42] {
    let p = pp();
    let s: Vec<f32> = S8.iter().map(|&v| f32::from(2 * v - 1)).collect();
    let mut q = [0f32; 42];
    let mut i = [0f32; 42];
    for k in 0..6 {
        q[k] = p[6 + k] * s[0];
    }
    for k in 0..12 {
        q[6 + k] = p[k] * s[2];
        q[18 + k] = p[k] * s[4];
        q[30 + k] = p[k] * s[6];
        i[k] = p[k] * s[1];
        i[12 + k] = p[k] * s[3];
        i[24 + k] = p[k] * s[5];
    }
    for k in 0..6 {
        i[36 + k] = p[k] * s[7];
    }
    let mut out = [Complex::new(0f32, 0.0); 42];
    for k in 0..42 {
        out[k] = Complex::new(i[k], q[k]);
    }
    out
}

/// `msk144decodeframe`: un quadro allineato (864 campioni complessi a frequenza zero) -> messaggio.
fn decode_frame(c: &mut [Complex<f32>], hashes: &mut CallHashes) -> Option<String> {
    let cb = sync_wave();
    let p = pp();
    let cca: Complex<f32> = (0..42).map(|k| c[k] * cb[k].conj()).sum();
    let ccb: Complex<f32> = (0..42).map(|k| c[336 + k] * cb[k].conj()).sum();
    let s = cca + ccb;
    let ph = s.im.atan2(s.re);
    let rot = Complex::new(ph.cos(), -ph.sin());
    for v in c.iter_mut() {
        *v *= rot;
    }
    let mut soft = [0f32; 144];
    soft[0] = (0..6).map(|k| c[k].im * p[6 + k]).sum::<f32>() + (0..6).map(|k| c[NSPM - 6 + k].im * p[k]).sum::<f32>();
    soft[1] = (0..12).map(|k| c[k].re * p[k]).sum();
    for i in 2..=72usize {
        let a = (i - 1) * 12 - 6;
        soft[2 * i - 2] = (0..12).map(|k| c[a + k].im * p[k]).sum();
        let b = 6 + (i - 1) * 12 - 6;
        soft[2 * i - 1] = (0..12).map(|k| c[(b + k) % NSPM].re * p[k]).sum();
    }
    let s8: Vec<i32> = S8.iter().map(|&v| 2 * i32::from(v) - 1).collect();
    let hard = |v: f32| if v >= 0.0 { 1 } else { -1 };
    let bad = |off: usize| (8 - (0..8).map(|k| hard(soft[off + k]) * s8[k]).sum::<i32>()) / 2;
    if bad(0) + bad(56) > 4 {
        return None;
    }
    let av = soft.iter().sum::<f32>() / 144.0;
    let s2 = soft.iter().map(|v| v * v).sum::<f32>() / 144.0;
    let sig = (s2 - av * av).sqrt();
    if sig <= 0.0 {
        return None;
    }
    let mut llr = [0f32; N];
    let sigma = 0.6f32;
    for i in 0..48 {
        llr[i] = 2.0 * soft[8 + i] / sig / (sigma * sigma);
    }
    for i in 0..80 {
        llr[48 + i] = 2.0 * soft[64 + i] / sig / (sigma * sigma);
    }
    let (bits, nhard) = bp_decode(&llr, 10)?;
    if nhard >= 18 {
        return None;
    }
    let n3 = bits[71..74].iter().fold(0u8, |a, &b| (a << 1) | b);
    let i3 = bits[74..77].iter().fold(0u8, |a, &b| (a << 1) | b);
    if (i3 == 0 && (n3 == 1 || n3 == 3 || n3 == 4 || n3 > 5)) || i3 == 3 || i3 > 5 {
        return None;
    }
    unpack(&bits, hashes).ok()
}

/// Un messaggio ricevuto.
#[derive(Debug, Clone, PartialEq)]
pub struct Decoded {
    pub text: String,
    /// Frequenza centrale stimata, in hertz.
    pub freq_hz: f32,
    /// Istante del ping dall'inizio del periodo, in secondi.
    pub t_s: f32,
    /// Forza del ping sul rumore, in dB.
    pub snr_db: i32,
}

#[derive(Debug, Clone)]
pub struct Options {
    /// Frequenza centrale attesa (di base 1500 Hz).
    pub f_center_hz: f32,
    /// Tolleranza di frequenza (+-).
    pub tolerance_hz: f32,
    /// Al piu' quante finestre (ping) si provano per periodo.
    pub max_pings: usize,
}

impl Default for Options {
    fn default() -> Self {
        Options { f_center_hz: 1500.0, tolerance_hz: 100.0, max_pings: 40 }
    }
}

/// Segnale analitico (frequenze negative a zero) limitato alla banda `lo..hi` Hz.
fn analytic(audio: &[f32], lo: f64, hi: f64) -> Vec<Complex<f32>> {
    let n = audio.len();
    let mut planner = FftPlanner::<f32>::new();
    let mut buf: Vec<Complex<f32>> = audio.iter().map(|&x| Complex::new(x, 0.0)).collect();
    planner.plan_fft_forward(n).process(&mut buf);
    let df = FS / n as f64;
    for (k, v) in buf.iter_mut().enumerate() {
        let f = k as f64 * df;
        if k == 0 || k > n / 2 || f < lo || f > hi {
            *v = Complex::new(0.0, 0.0);
        } else {
            *v *= 2.0 / n as f32;
        }
    }
    planner.plan_fft_inverse(n).process(&mut buf);
    buf
}

/// Decodifica un periodo (audio a 12 kHz dall'inizio del periodo).
pub fn decode(audio: &[f32], options: &Options, hashes: &mut CallHashes) -> Vec<Decoded> {
    let fc = f64::from(options.f_center_hz);
    if audio.len() < 3 * NSPM {
        return Vec::new();
    }
    let z = analytic(audio, fc - 1100.0, fc + 1100.0);
    // potenza di ogni mezzo quadro: i ping sono le finestre piu' forti del rumore (mediana)
    let half = NSPM / 2;
    let nblk = z.len() / half;
    let pw: Vec<f32> = (0..nblk).map(|b| z[b * half..(b + 1) * half].iter().map(|c| c.norm_sqr()).sum::<f32>() / half as f32).collect();
    let mut sorted = pw.clone();
    sorted.sort_by(f32::total_cmp);
    let noise = sorted[sorted.len() / 2].max(f32::MIN_POSITIVE);
    let mut windows: Vec<(usize, usize, f32)> = Vec::new(); // (inizio in campioni, quadri, rapporto)
    for nfr in [1usize, 2, 3] {
        let span = 2 * nfr; // mezzi quadri
        for b in 0..nblk.saturating_sub(span) {
            let p = pw[b..b + span].iter().sum::<f32>() / span as f32 / noise;
            windows.push((b * half, nfr, p));
        }
    }
    windows.sort_by(|a, b| b.2.total_cmp(&a.2));
    let cb = sync_wave();
    let ntol = (f64::from(options.tolerance_hz) / 5.0).round() as i64;
    let mut out: Vec<Decoded> = Vec::new();
    let mut tried = 0usize;
    for (start, nfr, ratio) in windows {
        if tried >= options.max_pings {
            break;
        }
        // finestre gia' coperte da un messaggio trovato
        if out.iter().any(|d| ((f64::from(d.t_s) * FS) as i64 - start as i64).abs() < (NSPM * 3) as i64) {
            continue;
        }
        tried += 1;
        let seg = &z[start..start + nfr * NSPM];
        let (mut best, mut best_f, mut best_c, mut best_x) = (0f32, 0f64, vec![Complex::new(0f32, 0.0); NSPM], vec![0f32; NSPM]);
        for ifr in -ntol..=ntol {
            let f = fc + ifr as f64 * 5.0;
            let w = -2.0 * std::f64::consts::PI * f / FS;
            let mut c = vec![Complex::new(0f32, 0.0); NSPM];
            for (k, v) in seg.iter().enumerate() {
                let ph = w * (start + k) as f64;
                c[k % NSPM] += v * Complex::new(ph.cos() as f32, ph.sin() as f32);
            }
            let mut xcc = vec![0f32; NSPM];
            for (ish, x) in xcc.iter_mut().enumerate() {
                let mut acc = Complex::new(0f32, 0.0);
                for k in 0..42 {
                    acc += (c[(ish + k) % NSPM] + c[(ish + 336 + k) % NSPM]) * cb[k].conj();
                }
                *x = acc.norm();
            }
            let m = xcc.iter().copied().fold(0f32, f32::max);
            if m > best {
                best = m;
                best_f = f;
                best_c = c;
                best_x = xcc;
            }
        }
        // i due picchi del sincronismo piu' alti
        let mut peaks = Vec::new();
        let mut x = best_x;
        for _ in 0..2 {
            let (ic, _) = x.iter().enumerate().fold((0, f32::MIN), |a, (i, &v)| if v > a.1 { (i, v) } else { a });
            peaks.push(ic);
            for k in ic.saturating_sub(7)..(ic + 8).min(NSPM) {
                x[k] = 0.0;
            }
        }
        for ic in peaks {
            let mut frame: Vec<Complex<f32>> = (0..NSPM).map(|k| best_c[(ic + k) % NSPM]).collect();
            if let Some(text) = decode_frame(&mut frame, hashes)
                && !out.iter().any(|d| d.text == text)
            {
                let snr = 10.0 * (f64::from(ratio) - 1.0).max(0.01).log10();
                out.push(Decoded { text, freq_hz: best_f as f32, t_s: (start as f64 / FS) as f32, snr_db: snr.round() as i32 });
                break;
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

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

    #[test]
    fn crc_and_code_check_each_other() {
        let bits = pack("CQ IU3ABC IO91", &mut CallHashes::default()).expect("messaggio");
        let cw = encode(&bits);
        let mut llr = [0f32; N];
        for i in 0..N {
            llr[i] = if cw[i] == 1 { 5.0 } else { -5.0 };
        }
        assert_eq!(bp_decode(&llr, 10).map(|r| r.0), Some(bits));
    }

    #[test]
    fn tones_are_binary_and_the_frame_lasts_72_ms() {
        let t = tones("CQ IU3ABC IO91", &mut CallHashes::default()).expect("messaggio");
        assert!(t.iter().all(|&v| v <= 1));
        assert_eq!(modulate(&t, 1500.0, 1, 1.0).len(), NSPM);
    }

    #[test]
    fn a_short_ping_in_noise_decodes() {
        let mut audio = noise(15 * 12_000, 1.0, 3);
        let t = tones("IU3ABC K1ABC R-05", &mut CallHashes::default()).expect("messaggio");
        // ping di 3 quadri (216 ms) a 6,2 s, frequenza 1520 Hz
        let wave = modulate(&t, 1520.0, 3, 0.7);
        let at = 74_400 + 123;
        for (k, v) in wave.iter().enumerate() {
            audio[at + k] += v;
        }
        let d = decode(&audio, &Options::default(), &mut CallHashes::default());
        let hit = d.iter().find(|d| d.text == "IU3ABC K1ABC R-05").expect("decodificato");
        assert!((hit.freq_hz - 1520.0).abs() <= 10.0, "{hit:?}");
        assert!((hit.t_s - 6.2).abs() < 0.3, "{hit:?}");
    }

    #[test]
    fn pure_noise_decodes_nothing() {
        let audio = noise(15 * 12_000, 1.0, 11);
        assert!(decode(&audio, &Options::default(), &mut CallHashes::default()).is_empty());
    }
}
