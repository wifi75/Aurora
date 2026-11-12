//! MSK144: meteor scatter. Messaggio di 77 bit come FT8 con CRC a 13 bit, codice LDPC (128,90), quadro di 144 bit (due parole di
//! sincronismo da 8 bit) in MSK a 2000 baud: 72 ms per quadro, ripetuto per tutto il periodo. Dai sorgenti di WSJT-X (GPL 3):
//! `genmsk_128_90.f90`, `encode_128_90.f90`, `crc13.cpp`, `bpdecode128_90.f90`, `msk144decodeframe.f90`, `msk144sync.f90`,
//! `msk144_freq_search.f90`, `msk144sim.f90`.
//!
//! La ricezione e' quella in tempo reale di WSJT-X (`mskrtd.f90`): blocchi di 7168 campioni (0,6 s) ogni 3584 (0,3 s); in ogni blocco il
//! decodificatore dei ping brevi (`msk144spd.f90`: spettro del segnale al quadrato, toni a 2000 e 4000 Hz, medie di uno-tre quadri), poi i
//! messaggi brevi (MSK40, `<...>`, in `msk40`) e, se nulla e' uscito, le medie lunghe di 4, 5 e 7 quadri (`msk144sync.f90`) secondo la profondita'.

use ft8::message::{Bits77, CallHashes, pack, unpack};
use std::sync::Arc;

use rustfft::{Fft, FftPlanner, num_complex::Complex};

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
                tov[j][i] = 2.0 * platanh(-t);
            }
        }
    }
    None
}

/// `platanh` di WSJT-X: arcotangente iperbolica approssimata a tratti e limitata.
pub(crate) fn platanh(x: f32) -> f32 {
    let z = x.abs();
    if z <= 0.664 {
        x / 0.83
    } else if z <= 0.9217 {
        x.signum() * (z - 0.4064) / 0.322
    } else if z <= 0.9951 {
        x.signum() * (z - 0.8378) / 0.0524
    } else if z <= 0.9998 {
        x.signum() * (z - 0.9914) / 0.0012
    } else {
        x.signum() * 7.0
    }
}

/// Mezza sinusoide del simbolo (12 campioni).
pub(crate) fn half_sine() -> [f32; 12] {
    pp()
}

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

/// `msk144decodeframe`: un quadro allineato (864 campioni complessi a frequenza zero) -> messaggio e suo tipo (`i3`).
fn decode_frame(c: &mut [Complex<f32>], hashes: &mut CallHashes) -> Option<(String, u8)> {
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
    unpack(&bits, hashes).ok().map(|t| (t, i3))
}

/// Un messaggio ricevuto.
#[derive(Debug, Clone, PartialEq)]
pub struct Decoded {
    pub text: String,
    /// Frequenza centrale stimata, in hertz (arrotondata come la colonna «Freq» di WSJT-X).
    pub freq_hz: f32,
    /// Istante del ping dall'inizio del periodo, in secondi.
    pub t_s: f32,
    /// Forza del ping sul rumore, in dB (da -8 a +24, `mskrtd.f90:207-208`).
    pub snr_db: i32,
}

/// Le opzioni di ricezione: quelle che la finestra di WSJT-X passa a `mskrtd` (`mainwindow.cpp`, `fastSink`).
#[derive(Debug, Clone)]
pub struct Options {
    /// Frequenza di ricezione, centro del segnale (`nrxfreq`, `mskrtd.f90:82`; in WSJT-X da 1400 a 1600 Hz, 1500 all'avvio del modo).
    pub f_center_hz: f32,
    /// Tolleranza di frequenza +- (`ntol`, «F Tol»; per MSK144 50 Hz di base, `mainwindow.cpp:12090`).
    pub tolerance_hz: u32,
    /// Profondita' (`ndepth`, `mskrtd.f90:146-148`): 1 = Fast (solo ping brevi), 2 = Normal (medie di 4 quadri), 3 = Deep (anche di 5 e 7).
    /// WSJT-X parte da Deep (`NDepth` = 3, `mainwindow_settings.cpp:529`).
    pub depth: u8,
    /// Il proprio nominativo e quello del corrispondente (`mycall`, `hiscall`): i messaggi brevi del QSO portano l'hash della coppia.
    pub my_call: String,
    pub his_call: String,
    /// Messaggi brevi («Sh», `bshmsg`): si cercano quelli della coppia proprio nominativo / corrispondente.
    pub short_messages: bool,
    /// Ascolto («SWL», `bswl`): anche i messaggi brevi di altre coppie, riconosciuti con la lista dei nominativi recenti.
    pub swl: bool,
}

impl Default for Options {
    fn default() -> Self {
        Options { f_center_hz: 1500.0, tolerance_hz: 50, depth: 3, my_call: String::new(), his_call: String::new(), short_messages: false, swl: false }
    }
}

// ---------------------------------------------------------------- ricezione (`mskrtd`)

/// Campioni del blocco di analisi di `mskrtd` (`NZ`, 0,597 s).
pub const BLOCK: usize = 7168;
/// Passo fra un blocco e il seguente, mezzo blocco (0,3 s): `hspec.f90:91-100`, `decode_msk144.f90:13-15`.
pub const STEP: usize = BLOCK / 2;
/// Fine dell'ultimo blocco possibile dall'inizio del periodo (`k.gt.30*12000` di `hspec.f90:48`).
pub const LAST_K: usize = 30 * 12_000;
const NFFT1: usize = 8192;
/// Nominativi recenti (`MAXRECENT`, `packjt77.f90:7`).
pub(crate) const MAXRECENT: usize = 10;
/// Messaggi brevi recenti dell'ascolto (`NSHMEM`, `mskrtd.f90:14`).
const NSHMEM: usize = 50;
/// Medie lunghe di 4, 4, 5 e 7 quadri (`iavpatterns`, `mskrtd.f90:49-53`) e il centro di ciascuna in quadri (`xmc`).
const IAVPATTERNS: [[u8; 8]; 4] = [[1, 1, 1, 1, 0, 0, 0, 0], [0, 0, 1, 1, 1, 1, 0, 0], [1, 1, 1, 1, 1, 0, 0, 0], [1, 1, 1, 1, 1, 1, 1, 0]];
const XMC: [f32; 4] = [2.0, 4.5, 2.5, 3.5];
/// Le sei medie di uno, due e tre quadri dei decodificatori dei ping brevi (`navpatterns`, `msk144spd.f90:39-45` e `msk40spd.f90:39-45`).
pub(crate) const NAVPATTERNS: [[u8; 3]; 6] = [[0, 1, 0], [1, 0, 0], [0, 0, 1], [1, 1, 0], [0, 1, 1], [1, 1, 1]];

/// Il filtro passabanda di `analytic.f90:28-43`: coseno rialzato centrato a 1500 Hz, piatto fino a +-900 Hz, nullo oltre +-1100 Hz.
/// E' fisso a 1500 Hz qualunque sia la frequenza di ricezione, come in WSJT-X.
fn bandpass(nfft: usize) -> Vec<f32> {
    let df = 12_000.0 / nfft as f32;
    let (t, beta) = (1.0f32 / 2000.0, 0.1f32);
    let pi = std::f32::consts::PI;
    (0..=nfft / 2)
        .map(|i| {
            let f = (i as f32 * df - 1500.0).abs();
            if f > (1.0 - beta) / (2.0 * t) && f <= (1.0 + beta) / (2.0 * t) {
                0.5 * (1.0 + ((pi * t / beta) * (f - (1.0 - beta) / (2.0 * t))).cos())
            } else if f > (1.0 + beta) / (2.0 * t) {
                0.0
            } else {
                1.0
            }
        })
        .collect()
}

/// `msk_clip_spectrum_window` (`msk_spectrum.f90:12-36`): la finestra di ricerca di un tono in righe 1..=nfft; `None` se fuori dallo spettro.
fn clip_window(nfft: usize, lo: f64, hi: f64) -> Option<(usize, usize)> {
    let n = nfft as f64;
    if !lo.is_finite() || !hi.is_finite() || lo > hi || hi < 0.5 || lo >= n + 0.5 {
        return None;
    }
    let a = lo.clamp(1.0, n).round() as usize;
    let b = hi.clamp(1.0, n).round() as usize;
    (a <= b).then_some((a, b))
}

/// `msk_peak_offset` (`msk_spectrum.f90:38-57`): lo scarto frazionario del picco dalla curvatura dello spettro complesso (riga 1-based).
fn peak_offset(spectrum: &[Complex<f32>], peak: usize) -> f32 {
    if peak <= 1 || peak >= spectrum.len() {
        return 0.0;
    }
    let (a, b, c) = (spectrum[peak - 2], spectrum[peak - 1], spectrum[peak]);
    let curvature = b * 2.0 - a - c;
    let scale = a.norm().max(b.norm()).max(c.norm());
    if scale <= 0.0 || curvature.norm() <= f32::EPSILON * scale {
        return 0.0;
    }
    let candidate = -((a - c) / curvature).re;
    if candidate.is_finite() { candidate } else { 0.0 }
}

/// Indice (0-based) del primo massimo, come `maxloc`.
fn first_max(x: &[f32]) -> usize {
    let mut best = 0;
    for (i, &v) in x.iter().enumerate() {
        if v > x[best] {
            best = i;
        }
    }
    best
}

/// Un candidato dei decodificatori dei ping brevi: inizio (1-based nel blocco, come `nstart`), errore di frequenza e forza stimata.
pub(crate) struct Candidate {
    pub nstart: usize,
    pub ferr: f32,
    pub snr: f32,
}

/// La ricerca dei ping di `msk144spd.f90:72-167` e `msk40spd.f90:72-167`: finestre di un quadro ogni quarto di quadro, spettro del segnale al
/// quadrato (i due toni MSK diventano righe a 2(fc-500) e 2(fc+500) Hz), metrica rispetto al quarto percentile e, se i candidati sono meno di
/// tre, rapporto del picco sulla media della finestra.
#[allow(clippy::too_many_arguments)]
pub(crate) fn candidates(cbig: &[Complex<f32>], nspm: usize, step: usize, maxsteps: usize, threshold: f32, ntol: u32, fc: f32, fft: &dyn Fft<f32>) -> Vec<Candidate> {
    const MAXCAND: usize = 5;
    let n = cbig.len();
    if n < 3 * nspm || !fc.is_finite() {
        return Vec::new();
    }
    let df = 12_000.0 / nspm as f64;
    let nstep = maxsteps.min((n - nspm) / step);
    let nfhi = 2.0 * (f64::from(fc) + 500.0);
    let nflo = 2.0 * (f64::from(fc) - 500.0);
    let tol2 = 2.0 * f64::from(ntol);
    let (Some((ihlo, ihhi)), Some((illo, ilhi))) =
        (clip_window(nspm, (nfhi - tol2) / df + 1.0, (nfhi + tol2) / df + 1.0), clip_window(nspm, (nflo - tol2) / df + 1.0, (nflo + tol2) / df + 1.0))
    else {
        return Vec::new();
    };
    let i2000 = (nflo / df).round() as f32 + 1.0;
    let i4000 = (nfhi / df).round() as f32 + 1.0;
    let mut rcw = [0f32; 12];
    for (i, r) in rcw.iter_mut().enumerate() {
        *r = (1.0 - (i as f32 * std::f32::consts::PI / 12.0).cos()) / 2.0;
    }
    let mut detmet = vec![0f32; nstep];
    let mut detmet2 = vec![0f32; nstep];
    let mut detfer = vec![-999.99f32; nstep];
    let mut ctmp = vec![Complex::new(0f32, 0.0); nspm];
    let mut tonespec = vec![0f32; nspm];
    // il picco di un tono nella finestra (righe 1-based lo..=hi): riga, scarto, ampiezza e rapporto sulla media del resto della finestra
    let tone = |spec: &[f32], ctmp: &[Complex<f32>], lo: usize, hi: usize| {
        let pk = lo + first_max(&spec[lo - 1..hi]);
        let delta = peak_offset(ctmp, pk);
        let a = spec[pk - 1];
        let avp = (spec[lo - 1..hi].iter().sum::<f32>() - a) / (hi - lo + 1) as f32;
        (pk, delta, a, a / (avp + 0.01))
    };
    for istp in 0..nstep {
        let ns = step * istp;
        if ns + nspm > n {
            break;
        }
        for (k, v) in ctmp.iter_mut().enumerate() {
            let x = cbig[ns + k];
            *v = x * x;
        }
        for k in 0..12 {
            ctmp[k] *= rcw[k];
            ctmp[nspm - 12 + k] *= rcw[11 - k];
        }
        fft.process(&mut ctmp);
        for (s, v) in tonespec.iter_mut().zip(&ctmp) {
            *s = v.norm_sqr();
        }
        let (ihpk, deltah, ah, trath) = tone(&tonespec, &ctmp, ihlo, ihhi);
        let (ilpk, deltal, al, tratl) = tone(&tonespec, &ctmp, illo, ilhi);
        let ferrh = (ihpk as f32 + deltah - i4000) * df as f32 / 2.0;
        let ferrl = (ilpk as f32 + deltal - i2000) * df as f32 / 2.0;
        detmet[istp] = ah.max(al);
        detmet2[istp] = trath.max(tratl);
        detfer[istp] = if ah >= al { ferrh } else { ferrl };
    }
    // il rumore della metrica: il valore al quarto dell'ordinamento (`indexx`, `detmet(indices(nstep/4))`)
    let mut sorted = detmet.clone();
    sorted.sort_by(f32::total_cmp);
    let Some(&xmed) = (nstep / 4).checked_sub(1).and_then(|i| sorted.get(i)) else {
        return Vec::new();
    };
    if xmed <= 0.0 || !xmed.is_finite() {
        return Vec::new();
    }
    for v in &mut detmet {
        *v /= xmed;
    }
    let mut out: Vec<Candidate> = Vec::new();
    let tol = ntol as f32;
    for _ in 0..MAXCAND {
        let il = first_max(&detmet);
        if detmet[il] < threshold {
            break;
        }
        if detfer[il].abs() <= tol {
            out.push(Candidate { nstart: 1 + il * step + 1, ferr: detfer[il], snr: 12.0 * detmet[il].log10() / 2.0 - 9.0 });
        }
        detmet[il] = 0.0;
    }
    if out.len() < 3 {
        for _ in 0..MAXCAND - out.len() {
            let il = first_max(&detmet2);
            if detmet2[il] < 12.0 {
                break;
            }
            if detfer[il].abs() <= tol {
                out.push(Candidate { nstart: 1 + il * step + 1, ferr: detfer[il], snr: 12.0 * detmet2[il].log10() / 2.0 - 9.0 });
            }
            detmet2[il] = 0.0;
        }
    }
    out
}

/// L'esito della ricerca del sincronismo: frequenza stimata, i due picchi della correlazione (1-based, come `npkloc`), riuscita
/// (`xmax >= 1.3`) e il quadro medio a frequenza zero.
pub(crate) struct Sync {
    pub fest: f32,
    pub peaks: [usize; 2],
    pub ok: bool,
    pub c: Vec<Complex<f32>>,
}

/// `msk144sync.f90` con `msk144_freq_search.f90` (e `msk40sync.f90` con `msk40_freq_search.f90`): per ogni frequenza da `fc - ntol` a
/// `fc + ntol` a passi `delf` si riporta il segnale a zero (`tweak1`), si sommano i quadri della maschera, si correla con la parola di
/// sincronismo (`second`: lo scarto della seconda parola, 336 campioni in MSK144, nessuna in MSK40) e si tiene la frequenza col picco piu'
/// alto; `fac0` e' 48 in MSK144 e 24 in MSK40.
#[allow(clippy::too_many_arguments)]
pub(crate) fn sync_search(cdat: &[Complex<f32>], nspm: usize, mask: &[u8], ntol: f32, delf: f32, fc: f32, cb: &[Complex<f32>; 42], second: Option<usize>, fac0: f32) -> Sync {
    let nlim = (ntol / delf).round() as i64;
    let navg = mask.iter().map(|&m| u32::from(m)).sum::<u32>();
    let mut best = Sync { fest: fc, peaks: [1, 1], ok: false, c: vec![Complex::new(0f32, 0.0); nspm] };
    let mut xccs = vec![0f32; nspm];
    let mut xmax = 0f32;
    if navg > 0 {
        let fac = 1.0 / (fac0 * (navg as f32).sqrt());
        let mut bestf = 0f32;
        let mut c = vec![Complex::new(0f32, 0.0); nspm];
        let mut d = vec![Complex::new(0f32, 0.0); nspm];
        let mut xcc = vec![0f32; nspm];
        for ifr in -nlim..=nlim {
            let ferr = ifr as f32 * delf;
            // `tweak1`: la fase avanza di un passo prima di ogni campione (il primo e' gia' ruotato di un passo)
            let dphi = (2.0 * std::f64::consts::PI * f64::from(-(fc + ferr)) / 12_000.0) as f32;
            let wstep = Complex::new(f64::from(dphi.cos()), f64::from(dphi.sin()));
            let mut w = Complex::new(1.0f64, 0.0);
            c.iter_mut().for_each(|v| *v = Complex::new(0.0, 0.0));
            for (i, x) in cdat.iter().enumerate().take(nspm * mask.len()) {
                w *= wstep;
                if mask[i / nspm] == 1 {
                    c[i % nspm] += x * Complex::new(w.re as f32, w.im as f32);
                }
            }
            for k in 0..nspm {
                d[k] = c[k] + second.map_or(Complex::new(0.0, 0.0), |s| c[(k + s) % nspm]);
            }
            for (ish, x) in xcc.iter_mut().enumerate() {
                let mut acc = Complex::new(0f32, 0.0);
                for (j, b) in cb.iter().enumerate() {
                    acc += d[(ish + j) % nspm].conj() * b;
                }
                *x = acc.norm();
            }
            if !xcc.iter().all(|v| v.is_finite()) {
                continue;
            }
            let xb = xcc.iter().copied().fold(f32::MIN, f32::max) * fac;
            if !xb.is_finite() {
                continue;
            }
            if ifr == -nlim || xb > xmax {
                xmax = xb;
                bestf = ferr;
                best.c.copy_from_slice(&c);
                xccs.copy_from_slice(&xcc);
            }
        }
        best.fest = fc + bestf;
    }
    // i due picchi piu' alti; `maxloc` da' l'indice 1-based e la zona azzerata parte da quello (`msk144sync.f90:297-303`)
    for p in &mut best.peaks {
        let ic2 = first_max(&xccs) + 1;
        *p = ic2;
        for v in &mut xccs[ic2.saturating_sub(7)..=(ic2 + 7).min(nspm - 1)] {
            *v = 0.0;
        }
    }
    best.ok = xmax >= 1.3;
    best
}

/// Il quadro che parte dal picco 1-based `ic0` (`cshift(c, ic0-1)`).
pub(crate) fn cshift(c: &[Complex<f32>], ic0: usize) -> Vec<Complex<f32>> {
    let n = c.len();
    (0..n).map(|k| c[(k + ic0 - 1) % n]).collect()
}

/// Gli inizi del quadro provati attorno a un picco (`is` = 1, 2, 3 di `msk144spd.f90:189-193`): il picco, uno prima e uno dopo.
pub(crate) fn dithers(ic: usize, nspm: usize) -> [usize; 3] {
    [ic, ic.saturating_sub(1).max(1), (ic + 1).min(nspm)]
}

/// `msk144spd.f90`: il decodificatore dei ping brevi. Restituisce messaggio, tipo, frequenza e istante (s dall'inizio del blocco).
fn spd(cbig: &[Complex<f32>], ntol: u32, fc: f32, fft: &dyn Fft<f32>, hashes: &mut CallHashes) -> Option<(String, u8, f32, f32)> {
    let n = cbig.len();
    let cb = sync_wave();
    for cand in candidates(cbig, NSPM, 216, 100, 3.0, ntol, fc, fft) {
        let mut ib = cand.nstart.saturating_sub(NSPM).max(1);
        let mut ie = ib - 1 + 3 * NSPM;
        if ie > n {
            ie = n;
            ib = ie + 1 - 3 * NSPM;
        }
        let cdat = &cbig[ib - 1..ie];
        let fo = fc + cand.ferr;
        for mask in &NAVPATTERNS {
            let s = sync_search(cdat, NSPM, mask, 8.0, 2.0, fo, &cb, Some(336), 48.0);
            if !s.ok {
                continue;
            }
            for &pk in &s.peaks {
                for ic0 in dithers(pk, NSPM) {
                    let mut ct = cshift(&s.c, ic0);
                    if let Some((text, i3)) = decode_frame(&mut ct, hashes) {
                        return Some((text, i3, s.fest, (cand.nstart + NSPM / 2) as f32 / 12_000.0));
                    }
                }
            }
        }
    }
    None
}

/// I nominativi che `unpack77` mette fra i recenti (`packjt77.f90:2155-2172` per i tipi 1 e 2, `2263-2281` per il tipo 4): i due
/// nominativi in chiaro di almeno tre caratteri, mai «CQ» ne' quelli fra `<>`; nell'ordine del messaggio.
fn recent_from(text: &str, i3: u8) -> Vec<String> {
    let t: Vec<&str> = text.split_whitespace().collect();
    let plain = |c: &str| c.len() >= 3 && !c.contains('<');
    let calls: Vec<&str> = match i3 {
        1 | 2 => {
            if t.first() == Some(&"CQ") {
                // «CQ DX IU3ABC IO91»: il modificatore (lettere fino a 4 o tre cifre) fa parte del primo campo
                let modifier = t.len() >= 3
                    && (t[1].chars().all(|ch| ch.is_ascii_alphabetic()) && t[1].len() <= 4 || t[1].len() == 3 && t[1].chars().all(|ch| ch.is_ascii_digit()));
                t.get(if modifier { 2 } else { 1 }).copied().into_iter().collect()
            } else {
                t.iter().take(2).copied().collect()
            }
        }
        4 => t.iter().take(2).copied().filter(|c| *c != "CQ" && plain(c)).take(1).collect(),
        _ => Vec::new(),
    };
    calls.into_iter().filter(|c| plain(c)).map(str::to_string).collect()
}

/// Lo stato di `mskrtd` fra un blocco e l'altro (le variabili `save` di `mskrtd.f90:55-56`): rumore medio, filtro dei doppioni,
/// nominativi recenti con i loro hash e messaggi brevi dell'ascolto.
pub struct Rtd {
    fft: Arc<dyn Fft<f32>>,
    ifft: Arc<dyn Fft<f32>>,
    fft864: Arc<dyn Fft<f32>>,
    fft240: Arc<dyn Fft<f32>>,
    h: Vec<f32>,
    first: bool,
    nutc00: u64,
    tsec0: f32,
    pnoise: f32,
    msglast: String,
    msglastswl: String,
    nsnrlast: i32,
    nsnrlastswl: i32,
    nhasharray: [[i32; MAXRECENT]; MAXRECENT],
    recent_calls: Vec<String>,
    recent_shmsgs: Vec<String>,
    mycall13: String,
    dxcall13: String,
}

impl Default for Rtd {
    fn default() -> Self {
        Rtd::new()
    }
}

impl std::fmt::Debug for Rtd {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Rtd").field("pnoise", &self.pnoise).field("recent_calls", &self.recent_calls).finish()
    }
}

/// I primi 12 caratteri di un nominativo (`character*12` di WSJT-X).
fn call12(c: &str) -> String {
    c.trim().chars().take(12).collect()
}

impl Rtd {
    pub fn new() -> Self {
        let mut planner = FftPlanner::<f32>::new();
        Rtd {
            fft: planner.plan_fft_forward(NFFT1),
            ifft: planner.plan_fft_inverse(NFFT1),
            fft864: planner.plan_fft_forward(NSPM),
            fft240: planner.plan_fft_forward(crate::msk40::NSPM40),
            h: bandpass(NFFT1),
            first: true,
            nutc00: 0,
            tsec0: 0.0,
            pnoise: -1.0,
            msglast: String::new(),
            msglastswl: String::new(),
            nsnrlast: -99,
            nsnrlastswl: -99,
            // `nhasharray` non ha valore iniziale in WSJT-X: qui -1, che nessun hash a 12 bit eguaglia
            nhasharray: [[-1; MAXRECENT]; MAXRECENT],
            recent_calls: vec![String::new(); MAXRECENT],
            recent_shmsgs: vec![String::new(); NSHMEM],
            mycall13: String::new(),
            dxcall13: String::new(),
        }
    }

    /// I nominativi recenti, il piu' nuovo per primo (`recent_calls`).
    pub fn recent_calls(&self) -> &[String] {
        &self.recent_calls
    }

    /// `add_call_to_recent_calls` (`packjt77.f90:1561-1584`): in testa se non e' gia' fra i primi nove.
    fn add_recent(&mut self, call: &str) {
        if self.recent_calls[..MAXRECENT - 1].iter().any(|c| c == call) {
            return;
        }
        self.recent_calls.pop();
        self.recent_calls.insert(0, call.to_string());
    }

    /// `update_msk40_hasharray.f90`: gli hash a 12 bit di ogni coppia ordinata di nominativi recenti.
    fn update_hasharray(&mut self) {
        self.nhasharray = [[-1; MAXRECENT]; MAXRECENT];
        for i in 0..MAXRECENT {
            for j in i + 1..MAXRECENT {
                let (a, b) = (&self.recent_calls[i], &self.recent_calls[j]);
                if !a.is_empty() && !b.is_empty() {
                    self.nhasharray[i][j] = i32::from(crate::msk40::hash40(a, b));
                    self.nhasharray[j][i] = i32::from(crate::msk40::hash40(b, a));
                }
            }
        }
    }

    /// `analytic.f90`: il segnale analitico del blocco gia' normalizzato (FFT di 8192 punti, filtro a 1500 Hz, frequenze negative a zero).
    fn analytic(&self, d: &[f32]) -> Vec<Complex<f32>> {
        let fac = 2.0 / NFFT1 as f32;
        let mut c: Vec<Complex<f32>> = (0..NFFT1).map(|i| Complex::new(d.get(i).map_or(0.0, |x| fac * x), 0.0)).collect();
        self.fft.process(&mut c);
        let nh = NFFT1 / 2;
        for (v, h) in c[..=nh].iter_mut().zip(&self.h) {
            *v *= *h;
        }
        c[0] *= 0.5;
        c[nh + 1..].iter_mut().for_each(|v| *v = Complex::new(0.0, 0.0));
        self.ifft.process(&mut c);
        c
    }

    /// `mskrtd`: un blocco di [`BLOCK`] campioni a 12 kHz che comincia `tsec` secondi dopo l'inizio del periodo `nutc0`; al piu' un messaggio,
    /// gia' filtrato dai doppioni.
    pub fn block(&mut self, id2: &[f32], nutc0: u64, tsec: f32, o: &Options, hashes: &mut CallHashes) -> Option<Decoded> {
        let mycall = call12(&o.my_call);
        let hiscall = call12(&o.his_call);
        if self.first {
            self.tsec0 = tsec;
            self.nutc00 = nutc0;
            self.pnoise = -1.0;
            self.recent_calls.iter_mut().for_each(String::clear);
            self.recent_shmsgs.iter_mut().for_each(String::clear);
            self.msglast.clear();
            self.msglastswl.clear();
            self.nsnrlast = -99;
            self.nsnrlastswl = -99;
            self.mycall13 = mycall.clone();
            self.dxcall13 = hiscall.clone();
            self.first = false;
        }
        // nominativi cambiati: si riparte da capo al blocco seguente (`mskrtd.f90:85`)
        if self.mycall13 != mycall || self.dxcall13 != hiscall {
            self.first = true;
        }
        // il filtro dei doppioni vale per periodo (`mskrtd.f90:88-94`)
        if self.nutc00 != nutc0 || tsec < self.tsec0 {
            self.msglast.clear();
            self.msglastswl.clear();
            self.nsnrlast = -99;
            self.nsnrlastswl = -99;
            self.nutc00 = nutc0;
        }
        let out = self.analyse(id2, tsec, o, &mycall, &hiscall, hashes);
        self.tsec0 = tsec;
        out
    }

    fn analyse(&mut self, id2: &[f32], tsec: f32, o: &Options, mycall: &str, hiscall: &str, hashes: &mut CallHashes) -> Option<Decoded> {
        if id2.len() < BLOCK {
            return None;
        }
        let d = &id2[..BLOCK];
        let rms = (d.iter().map(|x| x * x).sum::<f32>() / BLOCK as f32).sqrt();
        // `rms.lt.1.0` sui campioni a 16 bit
        if rms < 1.0 / 32_768.0 {
            return None;
        }
        let d: Vec<f32> = d.iter().map(|x| x / rms).collect();
        let cdat = self.analytic(&d);
        // potenza di ogni quadro: la piu' alta e' segnale piu' rumore, la media aggiorna il rumore se non esce nulla
        let pow: Vec<f32> = (0..8).map(|i| cdat[i * NSPM..(i + 1) * NSPM].iter().map(|c| c.norm_sqr()).sum::<f32>() * rms * rms).collect();
        let pmax = pow.iter().copied().fold(-99.0f32, f32::max);
        let pavg = pow.iter().sum::<f32>() / 8.0;
        let np = 8 * NSPM;
        let ntol = o.tolerance_hz;
        let fc = o.f_center_hz;
        // (testo, esito: 1 = messaggio intero o breve del QSO, 2-3 = breve d'ascolto; frequenza; istante; breve)
        let mut found: Option<(String, u8, f32, f32, bool)> = None;
        if let Some((text, i3, fest, tret)) = spd(&cdat[..np], ntol, fc, self.fft864.as_ref(), hashes) {
            self.stage_calls(&text, i3);
            found = Some((text, 1, fest, tsec + tret, false));
        } else if o.short_messages || o.swl {
            let ctx = crate::msk40::ShortContext { mycall, hiscall, swl: o.swl, nhasharray: &self.nhasharray, recent_calls: &self.recent_calls };
            if let Some((text, nsuccess, fest, tret)) = crate::msk40::spd40(&cdat[..np], ntol, fc, &ctx, self.fft240.as_ref()) {
                found = Some((text, nsuccess, fest, tsec + tret, true));
            }
        }
        if found.is_none() {
            // le medie lunghe: Fast nessuna, Normal 4 quadri, Deep anche 5 e 7 (`mskrtd.f90:146-171`)
            let npat = match o.depth {
                1 => 0,
                2 => 2,
                _ => IAVPATTERNS.len(),
            };
            let cb = sync_wave();
            'patterns: for (iavg, mask) in IAVPATTERNS.iter().enumerate().take(npat) {
                let navg = mask.iter().map(|&m| u32::from(m)).sum::<u32>();
                let deltaf = 10.0 / navg as f32;
                let s = sync_search(&cdat[..np], NSPM, mask, ntol as f32, deltaf, fc, &cb, Some(336), 48.0);
                if !s.ok {
                    continue;
                }
                for &pk in &s.peaks {
                    for ic0 in dithers(pk, NSPM) {
                        let mut ct = cshift(&s.c, ic0);
                        if let Some((text, i3)) = decode_frame(&mut ct, hashes) {
                            self.stage_calls(&text, i3);
                            found = Some((text, 1, s.fest, tsec + XMC[iavg] * NSPM as f32 / 12_000.0, false));
                            break 'patterns;
                        }
                    }
                }
            }
        }
        let Some((text, nsuccess, fest, tdec, short)) = found else {
            // nessun messaggio: il rumore sale piano e scende subito (`mskrtd.f90:176-183`)
            if self.pnoise < 0.0 || pavg < self.pnoise {
                self.pnoise = pavg;
            } else if pavg > self.pnoise {
                self.pnoise = 0.9 * self.pnoise + 0.1 * pavg;
            }
            return None;
        };
        // forza: il quadro piu' forte sul rumore medio, da -8 a +24 dB (`mskrtd.f90:188-193,207-208`); con il rapporto non positivo
        // (log10 non definito in WSJT-X) si da' il minimo
        let snr0 = if self.pnoise > 0.0 { 10.0 * (pmax / self.pnoise - 1.0).log10() } else { 0.0 };
        let nsnr = if snr0.is_finite() { (snr0.round() as i32).clamp(-8, 24) } else { -8 };
        let decoded = Decoded { text: text.clone(), freq_hz: fest.round(), t_s: tdec, snr_db: nsnr };
        // filtro dei doppioni (`mskrtd.f90:211-237`): si ripete solo un messaggio diverso dall'ultimo, o piu' forte, o all'inizio di un periodo
        if nsuccess == 1 && (text != self.msglast || nsnr > self.nsnrlast || tsec < self.tsec0) {
            self.msglast = text;
            self.nsnrlast = nsnr;
            if !short {
                self.update_hasharray();
            }
            return Some(decoded);
        }
        if o.swl && nsuccess >= 2 {
            // i brevi dell'ascolto si mostrano dalla seconda volta che si vedono, se non troppo deboli
            let seenb4 = self.recent_shmsgs.contains(&text);
            if !seenb4 {
                self.recent_shmsgs.pop();
                self.recent_shmsgs.insert(0, text.clone());
            }
            if seenb4 && (text != self.msglastswl || nsnr > self.nsnrlastswl || tsec < self.tsec0) && nsnr > -6 {
                self.msglastswl = text;
                self.nsnrlastswl = nsnr;
                return Some(decoded);
            }
        }
        None
    }

    fn stage_calls(&mut self, text: &str, i3: u8) {
        for c in recent_from(text, i3) {
            self.add_recent(&c);
        }
    }
}

/// Decodifica un periodo intero (audio a 12 kHz dall'inizio del periodo) come `decode_msk144_file`: `mskrtd` su blocchi di 7168 campioni
/// ogni 3584, con il filtro dei doppioni di un periodo.
pub fn decode(audio: &[f32], options: &Options, hashes: &mut CallHashes) -> Vec<Decoded> {
    let mut rtd = Rtd::new();
    let mut out = Vec::new();
    let mut pos = 0usize;
    while pos + BLOCK <= audio.len() && pos + BLOCK <= LAST_K {
        if let Some(d) = rtd.block(&audio[pos..pos + BLOCK], 0, pos as f32 / 12_000.0, options, hashes) {
            out.push(d);
        }
        pos += STEP;
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

    fn add(audio: &mut [f32], wave: &[f32], at: usize) {
        for (k, v) in wave.iter().enumerate() {
            audio[at + k] += v;
        }
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
    fn the_bandpass_is_the_one_of_analytic_f90() {
        let h = bandpass(NFFT1);
        let at = |f: f32| h[(f / (12_000.0 / NFFT1 as f32)).round() as usize];
        // piatto da 600 a 2400 Hz, meta' a 500 e 2500 Hz, nullo da 400 Hz in giu' e da 2600 Hz in su
        assert_eq!((at(1500.0), at(600.0), at(2400.0)), (1.0, 1.0, 1.0));
        assert_eq!((at(300.0), at(2700.0)), (0.0, 0.0));
        assert!((at(500.0) - 0.5).abs() < 0.05 && (at(2500.0) - 0.5).abs() < 0.05);
    }

    #[test]
    fn a_short_ping_in_noise_decodes() {
        let mut audio = noise(4 * 12_000, 1.0, 3);
        let t = tones("IU3ABC K1ABC R-05", &mut CallHashes::default()).expect("messaggio");
        // ping di 3 quadri (216 ms) a 2,2 s, frequenza 1520 Hz
        add(&mut audio, &modulate(&t, 1520.0, 3, 0.8), 26_400 + 123);
        let d = decode(&audio, &Options::default(), &mut CallHashes::default());
        let hit = d.iter().find(|d| d.text == "IU3ABC K1ABC R-05").expect("decodificato");
        assert!((hit.freq_hz - 1520.0).abs() <= 3.0, "{hit:?}");
        assert!((hit.t_s - 2.3).abs() < 0.3, "{hit:?}");
        assert!((-8..=24).contains(&hit.snr_db), "{hit:?}");
    }

    /// La frequenza di ricezione e la tolleranza contano: un ping a 1590 Hz si legge con Rx 1600 Hz, non con Rx 1500 Hz e F Tol 50.
    #[test]
    fn the_rx_frequency_and_tolerance_steer_the_search() {
        let mut audio = noise(3 * 12_000, 1.0, 5);
        let t = tones("CQ IU3ABC IO91", &mut CallHashes::default()).expect("messaggio");
        add(&mut audio, &modulate(&t, 1590.0, 2, 0.8), 15_000);
        let at_1600 = Options { f_center_hz: 1600.0, ..Options::default() };
        let d = decode(&audio, &at_1600, &mut CallHashes::default());
        assert!(d.iter().any(|d| d.text == "CQ IU3ABC IO91" && (d.freq_hz - 1590.0).abs() <= 3.0), "{d:?}");
        assert!(decode(&audio, &Options::default(), &mut CallHashes::default()).is_empty());
    }

    /// Un ping debole di un solo quadro (72 ms) lo trova il decodificatore dei ping brevi di WSJT-X.
    #[test]
    fn a_weak_single_frame_ping_decodes() {
        let t = tones("K1ABC IU3ABC -07", &mut CallHashes::default()).expect("messaggio");
        let mut audio = noise(3 * 12_000, 1.0, 9);
        add(&mut audio, &modulate(&t, 1537.0, 1, 1.4), 16_321);
        let o = Options { f_center_hz: 1550.0, ..Options::default() };
        let d = decode(&audio, &o, &mut CallHashes::default());
        assert!(d.iter().any(|d| d.text == "K1ABC IU3ABC -07"), "{d:?}");
    }

    #[test]
    fn a_short_msk40_ping_decodes_with_the_qso_calls() {
        // il corrispondente K1ABC manda a noi (IU3ABC) «<IU3ABC K1ABC> R+03»: l'hash e' della coppia proprio nominativo, corrispondente
        let t = crate::msk40::tones40("<IU3ABC K1ABC> R+03").expect("breve");
        let mut audio = noise(3 * 12_000, 1.0, 21);
        add(&mut audio, &crate::msk40::modulate40(&t, 1500.0, 10, 1.0), 15_000);
        let o = Options { my_call: "IU3ABC".into(), his_call: "K1ABC".into(), short_messages: true, ..Options::default() };
        let d = decode(&audio, &o, &mut CallHashes::default());
        assert!(d.iter().any(|d| d.text == "<IU3ABC K1ABC> R+03"), "{d:?}");
        // senza «Sh» (ne' SWL) i messaggi brevi non si cercano; con la coppia rovesciata l'hash non torna
        let off = Options { short_messages: false, ..o.clone() };
        assert!(decode(&audio, &off, &mut CallHashes::default()).iter().all(|d| !d.text.starts_with('<')));
        let swapped = Options { my_call: "K1ABC".into(), his_call: "IU3ABC".into(), ..o };
        assert!(decode(&audio, &swapped, &mut CallHashes::default()).iter().all(|d| !d.text.starts_with('<')));
    }

    /// In ascolto (SWL) un messaggio breve di altri si riconosce con la lista dei nominativi recenti e si mostra dalla seconda volta.
    #[test]
    fn swl_short_messages_resolve_with_the_recent_calls() {
        let mut hashes = CallHashes::default();
        let long = tones("K1ABC IU3ABC R-05", &mut hashes).expect("messaggio");
        let short = crate::msk40::tones40("<K1ABC IU3ABC> RRR").expect("breve");
        let mut audio = noise(6 * 12_000, 1.0, 33);
        add(&mut audio, &modulate(&long, 1500.0, 4, 1.0), 8_000);
        add(&mut audio, &crate::msk40::modulate40(&short, 1500.0, 12, 1.0), 30_000);
        add(&mut audio, &crate::msk40::modulate40(&short, 1500.0, 12, 1.0), 55_000);
        let o = Options { my_call: "IU3XYZ".into(), swl: true, ..Options::default() };
        let d = decode(&audio, &o, &mut CallHashes::default());
        let texts: Vec<&str> = d.iter().map(|d| d.text.as_str()).collect();
        assert_eq!(texts.iter().filter(|t| **t == "K1ABC IU3ABC R-05").count(), 1, "{d:?}");
        assert_eq!(texts.iter().filter(|t| **t == "<K1ABC IU3ABC> RRR").count(), 1, "{d:?}");
        // senza SWL il breve di altri non esce
        let plain = decode(&audio, &Options { swl: false, ..o }, &mut CallHashes::default());
        assert!(plain.iter().all(|d| !d.text.starts_with('<')), "{plain:?}");
    }

    #[test]
    fn the_recent_calls_follow_unpack77() {
        assert_eq!(recent_from("IU3ABC K1ABC/R R-05", 1), vec!["IU3ABC", "K1ABC/R"]);
        assert_eq!(recent_from("CQ IU3ABC IO91", 1), vec!["IU3ABC"]);
        assert_eq!(recent_from("CQ DX IU3ABC IO91", 1), vec!["IU3ABC"]);
        assert_eq!(recent_from("<...> K1ABC RRR", 4), vec!["K1ABC"]);
        assert!(recent_from("TNX BOB 73", 0).is_empty());
        let mut r = Rtd::new();
        for c in ["A1AA", "B1BB", "A1AA"] {
            r.add_recent(c);
        }
        assert_eq!(&r.recent_calls()[..2], &["B1BB".to_string(), "A1AA".to_string()]);
    }

    /// Filtro dei doppioni di `mskrtd`: lo stesso messaggio nello stesso periodo si ripete solo se piu' forte; nel periodo dopo si mostra di nuovo.
    #[test]
    fn the_duplicate_filter_works_per_period() {
        let t = tones("CQ IU3ABC IO91", &mut CallHashes::default()).expect("messaggio");
        let quiet = noise(BLOCK, 1.0, 41);
        let mut ping = noise(BLOCK, 1.0, 42);
        add(&mut ping, &modulate(&t, 1500.0, 2, 1.0), 3000);
        let mut weaker = noise(BLOCK, 1.0, 43);
        add(&mut weaker, &modulate(&t, 1500.0, 2, 0.6), 3000);
        let o = Options::default();
        let mut h = CallHashes::default();
        let mut r = Rtd::new();
        assert!(r.block(&quiet, 0, 0.0, &o, &mut h).is_none());
        let first = r.block(&ping, 0, 0.3, &o, &mut h).expect("primo");
        assert!(r.block(&weaker, 0, 0.6, &o, &mut h).is_none(), "piu' debole nello stesso periodo");
        assert!(r.block(&ping, 0, 0.9, &o, &mut h).is_none(), "uguale nello stesso periodo");
        let again = r.block(&ping, 15_000, 0.3, &o, &mut h).expect("periodo seguente");
        assert_eq!((first.text.as_str(), again.text.as_str()), ("CQ IU3ABC IO91", "CQ IU3ABC IO91"));
        assert!(first.snr_db > -8, "{first:?}");
    }

    #[test]
    fn pure_noise_decodes_nothing() {
        let audio = noise(8 * 12_000, 1.0, 11);
        let o = Options { short_messages: true, swl: true, my_call: "IU3ABC".into(), his_call: "K1ABC".into(), ..Options::default() };
        assert!(decode(&audio, &o, &mut CallHashes::default()).is_empty());
    }
}
