//! Decodifica "a8" (list decoding) del corrispondente: porting di `lib/ft8/ft8_a8d.f90` e di `getmsg` in `lib/ft8/ft8_a7.f90`
//! di WSJT-X 3.3.0-beta5 (licenza GNU GPL 3, file `COPYING` dei sorgenti; i due file non hanno intestazioni proprie).
//!
//! Quando DX Call e DX Grid sono noti, a fine periodo si provano sul residuo (l'audio con i segnali gia' decodificati tolti)
//! i messaggi che il corrispondente puo' mandarci (circa 206: `MYCALL DXCALL`, RRR, RR73, 73, il suo CQ con locatore, il
//! locatore, i rapporti da -30 a +30 con e senza R). Per ognuno si genera l'onda complessa a 200 Hz e la si correla con il
//! segnale attorno alla frequenza di ricezione (+-10 Hz, ritardo da -1 a +1 s); vince il messaggio con il picco piu' alto.
//! Il risultato si accetta solo se la frequenza e' entro 5 Hz, i simboli attesi spiegano bene lo spettro (`plog >= -159`,
//! non piu' di 54 simboli sbagliati, `sigobig >= 0.71`).
//!
//! # Differenze da WSJT-X
//!
//! - La discesa a 200 Hz e' quella di Aurora ([`Downsampler`], variante di JTDX), con la stessa scala di `ft8_downsample`.
//! - I messaggi che `pack` scriverebbe come testo libero non si provano (in WSJT-X `genft8` li codificherebbe come tali).
//! - L'SNR si misura sui +-2 Hz attorno al picco trovato e non attorno alla frequenza di ricezione: con il corrispondente
//!   a qualche hertz dalla frequenza di ricezione WSJT-X riporta sempre il minimo (-30 dB). La costante e' 41 dB invece di 35,
//!   tarata su audio sintetico.
//! - Il calcolo delle correlazioni si divide tra i thread con `rayon` quando [`A8Query::threads`] e' piu' di 1; a parita'
//!   di picco vince comunque il messaggio con l'indice piu' basso, come nella regione `omp critical` di WSJT-X.

use rayon::prelude::*;

use crate::a7::{encode_hypothesis, Hypothesis};
use crate::message::{self, CallHashes};
use crate::sync::{self, Complex, Downsampler};
use crate::NN;

/// Campioni dell'onda di un messaggio a 200 Hz (`NWAVE = NN*NSPS/NDOWN = 2528`).
const NWAVE: usize = NN * 32;
/// Campioni del segnale a 200 Hz e lunghezza della FFT (`NZZ`, `NFFT`).
const NZZ: usize = 3200;
const NH: i64 = (NZZ / 2) as i64;
/// Passo dei bin della FFT (Hz).
const DF: f32 = 200.0 / NZZ as f32;
/// Scala della potenza (`fac`).
const FAC: f32 = 1.0e-6;

/// Quello che serve per provare "a8".
#[derive(Debug, Clone, PartialEq)]
pub struct A8Query {
    pub mycall: String,
    pub dxcall: String,
    pub dxgrid: String,
    /// Frequenza di ricezione (`nfqso`, Hz).
    pub rx_freq_hz: f32,
    /// Thread per le correlazioni (1 = nessun parallelismo).
    pub threads: usize,
}

impl A8Query {
    /// I dati bastano? (`len(trim(hiscall12)).ge.3 .and. len(trim(hisgrid)).ge.4` di `ft8_decode.f90`)
    pub fn usable(&self) -> bool {
        self.mycall.trim().len() >= 3 && self.dxcall.trim().len() >= 3 && self.dxgrid.trim().len() >= 4
    }
}

/// Un messaggio trovato con "a8".
#[derive(Debug, Clone, PartialEq)]
pub struct A8Hit {
    pub text: String,
    pub freq_hz: f32,
    pub dt_s: f32,
    pub snr_db: f32,
    /// Logaritmo della probabilita' dei simboli attesi (`plog`).
    pub plog: f32,
    /// Simboli il cui tono piu' forte non e' quello atteso (`nhard`).
    pub hard_errors: u32,
    pub tones: [u8; NN],
}

/// Rapporto nel formato dei messaggi (`i3.2` con il `+`): -5 -> "-05", 7 -> "+07".
pub(crate) fn report(isnr: i32) -> String {
    if isnr < 0 { format!("-{:02}", -isnr) } else { format!("+{isnr:02}") }
}

/// L'ipotesi numero `i` (da 1 a 411) di `getmsg`: vuota se non vale per questi nominativi.
pub fn getmsg(i: usize, mycall: &str, hiscall: &str, hisgrid: &str) -> String {
    let my_std = message::is_standard_call_suffix(mycall);
    let his_std = message::is_standard_call_suffix(hiscall);
    let grid4: String = hisgrid.chars().take(4).collect();
    if i >= 207 {
        // 207-411: le ipotesi con nominativo non standard per un DX standard con /R o /P
        let n = hiscall.len();
        if !(his_std && n > 3 && (hiscall.ends_with("/R") || hiscall.ends_with("/P"))) {
            return String::new();
        }
        if i == 207 {
            return format!("CQ {hiscall}");
        }
        if !my_std {
            return String::new();
        }
        if i <= 211 {
            let base = format!("<{mycall}> {hiscall}");
            return match i {
                209 => format!("{base} RRR"),
                210 => format!("{base} RR73"),
                211 => format!("{base} 73"),
                _ => base,
            };
        }
        let j = i - 205;
        let isnr = -50 + (j as i32 - 7) / 2;
        if isnr.abs() > 30 {
            return String::new();
        }
        let base = format!("{mycall} <{hiscall}>");
        return if j % 2 == 1 { format!("{base} {}", report(isnr)) } else { format!("{base} R{}", report(isnr)) };
    }

    let mut msg = format!("{mycall} {hiscall}");
    if !my_std {
        if i == 1 || i >= 6 {
            msg = format!("<{mycall}> {hiscall}");
        }
        if (2..=4).contains(&i) {
            msg = format!("{mycall} <{hiscall}>");
        }
    } else if !his_std {
        if i <= 4 || i == 6 {
            msg = format!("<{mycall}> {hiscall}");
        }
        if i >= 7 {
            msg = format!("{mycall} <{hiscall}>");
        }
    }
    match i {
        2 => msg.push_str(" RRR"),
        3 => msg.push_str(" RR73"),
        4 => msg.push_str(" 73"),
        5 => msg = if his_std { format!("CQ {hiscall} {grid4}") } else { format!("CQ {hiscall}") },
        6 if his_std => msg = format!("{msg} {grid4}"),
        7..=206 => {
            let isnr = -50 + (i as i32 - 7) / 2;
            if isnr.abs() > 30 {
                return String::new();
            }
            msg = if i % 2 == 1 { format!("{msg} {}", report(isnr)) } else { format!("{msg} R{}", report(isnr)) };
        }
        _ => {}
    }
    msg.trim().to_string()
}

/// L'onda complessa del messaggio a 200 Hz (32 campioni per simbolo, BT = 2, frequenza 0), con le rampe: `gen_ft8wave` con
/// `icmplx = 1`.
fn complex_wave(tones: &[u8; NN]) -> Vec<Complex> {
    const NSPS: usize = 32;
    let bt = 2.0f64;
    let twopi = 2.0 * std::f64::consts::PI;
    let pulse: Vec<f64> = (0..3 * NSPS)
        .map(|i0| crate::modem::gfsk_pulse_f64(bt, ((i0 + 1) as f64 - 1.5 * NSPS as f64) / NSPS as f64))
        .collect();
    let dphi_peak = twopi / NSPS as f64;
    let mut dphi = vec![0.0f64; (NN + 2) * NSPS];
    for (j, &t) in tones.iter().enumerate() {
        let ib = j * NSPS;
        for (d, p) in dphi[ib..ib + 3 * NSPS].iter_mut().zip(&pulse) {
            *d += dphi_peak * p * f64::from(t);
        }
    }
    for (d, p) in dphi[..2 * NSPS].iter_mut().zip(&pulse[NSPS..]) {
        *d += dphi_peak * f64::from(tones[0]) * p;
    }
    for (d, p) in dphi[NN * NSPS..].iter_mut().zip(&pulse[..2 * NSPS]) {
        *d += dphi_peak * f64::from(tones[NN - 1]) * p;
    }
    let mut phi = 0.0f64;
    let mut wave = Vec::with_capacity(NWAVE);
    for &d in &dphi[NSPS..NSPS + NWAVE] {
        wave.push(Complex::new(phi.cos() as f32, phi.sin() as f32));
        phi = (phi + d) % twopi;
    }
    let nramp = NSPS / 8;
    for i in 0..nramp {
        let up = ((1.0 - (twopi * i as f64 / (2.0 * nramp as f64)).cos()) / 2.0) as f32;
        wave[i] *= up;
        let down = ((1.0 + (twopi * i as f64 / (2.0 * nramp as f64)).cos()) / 2.0) as f32;
        wave[NN * NSPS - nramp + i] *= down;
    }
    wave
}

/// Il picco di correlazione di un messaggio: valore, frequenza, ritardo e spettro (indice `j + NH`).
struct Peak {
    spk: f32,
    fpk: f32,
    tpk: f32,
    spectrum: Vec<f32>,
}

/// Correlazione di un'ipotesi con il segnale su tutti i ritardi (`lag` da -200 a 200 a passi di 4, poi +-8 a passi di 1).
fn correlate(cd: &[Complex], cwave_conj: &[Complex], f1: f32) -> Peak {
    let fft = sync::fft_plan(NZZ, false);
    let mut scratch = vec![Complex::new(0.0, 0.0); fft.get_inplace_scratch_len()];
    let mut cd0 = vec![Complex::new(0.0, 0.0); NZZ];
    let n = 2 * NH as usize + 1;
    let mut raw = vec![0.0f32; n];
    let mut s = vec![0.0f32; n];
    let mut best = Peak { spk: 0.0, fpk: 0.0, tpk: 0.0, spectrum: vec![0.0; n] };
    let mut lagpk: i64 = 0;
    for iter in 1..=2 {
        let (lag1, lag2, step) = if iter == 1 { (-200i64, 200i64, 4usize) } else { (lagpk - 8, lagpk + 8, 1) };
        for lag in (lag1..=lag2).step_by(step) {
            let ioffset = lag + 100;
            let i1 = (-ioffset).max(0);
            let i2 = (NWAVE as i64 - 1).min(NWAVE as i64 - 1 - ioffset);
            cd0.iter_mut().for_each(|c| *c = Complex::new(0.0, 0.0));
            if i1 <= i2 {
                for i in i1..=i2 {
                    cd0[i as usize] = cd[(i + ioffset) as usize] * cwave_conj[i as usize];
                }
            }
            fft.process_with_scratch(&mut cd0, &mut scratch);
            for (i, c) in cd0.iter().enumerate() {
                let j = if i as i64 > NH { i as i64 - NZZ as i64 } else { i as i64 };
                raw[(j + NH) as usize] = FAC * c.norm_sqr();
            }
            s[0] = raw[0];
            s[n - 1] = raw[n - 1];
            for j in 1..n - 1 {
                s[j] = 0.5 * raw[j] + 0.25 * (raw[j - 1] + raw[j + 1]);
            }
            // il primo massimo (confronto stretto come nel Fortran)
            let mut jmax = 0usize;
            for j in 1..n {
                if s[j] > s[jmax] {
                    jmax = j;
                }
            }
            if s[jmax] > best.spk {
                best.spk = s[jmax];
                best.fpk = (jmax as i64 - NH) as f32 * DF + f1;
                best.tpk = lag as f32 * 0.005;
                lagpk = lag;
                best.spectrum.copy_from_slice(&s);
            }
        }
    }
    best
}

fn db(x: f32) -> f32 {
    if x > 1.259e-10 { 10.0 * x.log10() } else { -99.0 }
}

/// Prova "a8" alla frequenza di ricezione sul residuo `ds`. `hashes` serve solo a impacchettare le ipotesi (non si modifica).
pub fn a8_decode(ds: &Downsampler, query: &A8Query, hashes: &CallHashes) -> Option<A8Hit> {
    if !query.usable() {
        return None;
    }
    let mycall = query.mycall.trim().to_uppercase();
    let dxcall = query.dxcall.trim().to_uppercase();
    let dxgrid = query.dxgrid.trim().to_uppercase();
    let f1 = query.rx_freq_hz;
    let mut cd = ds.run(f1, false);
    if cd.len() < NZZ {
        return None;
    }

    // quante ipotesi: 206, 207 o 411 per un DX standard con /R o /P
    let mut nrun = 206;
    if message::is_standard_call_suffix(&dxcall) && dxcall.len() > 3 && (dxcall.ends_with("/R") || dxcall.ends_with("/P")) {
        nrun = if message::is_standard_call_suffix(&mycall) { 411 } else { 207 };
    }
    let mut scratch_hashes = hashes.clone();
    let hyps: Vec<(Hypothesis, Vec<Complex>)> = (1..=nrun)
        .filter_map(|i| {
            let msg = getmsg(i, &mycall, &dxcall, &dxgrid);
            if msg.is_empty() {
                return None;
            }
            let h = encode_hypothesis(&msg, &mut scratch_hashes)?;
            let conj: Vec<Complex> = complex_wave(&h.tones).iter().map(|c| c.conj()).collect();
            Some((h, conj))
        })
        .collect();
    if hyps.is_empty() {
        return None;
    }

    let peaks: Vec<Peak> = if query.threads > 1 {
        let pool = rayon::ThreadPoolBuilder::new().num_threads(query.threads).build().ok();
        match pool {
            Some(p) => p.install(|| hyps.par_iter().map(|(_, w)| correlate(&cd, w, f1)).collect()),
            None => hyps.iter().map(|(_, w)| correlate(&cd, w, f1)).collect(),
        }
    } else {
        hyps.iter().map(|(_, w)| correlate(&cd, w, f1)).collect()
    };
    // il migliore; a parita' vince l'indice piu' basso
    let mut ibest: Option<usize> = None;
    let mut sbest = 0.0f32;
    for (k, p) in peaks.iter().enumerate() {
        if p.spk > sbest {
            sbest = p.spk;
            ibest = Some(k);
        }
    }
    let k = ibest?;
    let peak = &peaks[k];
    let hyp = &hyps[k].0;
    let fbest = peak.fpk;
    let shift = f1 - fbest;
    if shift.abs() > 5.0 {
        return None;
    }
    sync::shift_frequency(&mut cd, shift);
    let xdt = peak.tpk;

    // SNR dallo spettro migliore
    let s1 = &peak.spectrum;
    let at = |j: i64| s1[(j + NH) as usize];
    let ave = ((-200..=-100).map(at).sum::<f32>() + (100..=200).map(at).sum::<f32>()) / 202.0;
    if ave <= 0.0 {
        return None;
    }
    let norm = |j: i64| at(j) / ave - 1.0;
    // la finestra di +-2 Hz attorno al picco (WSJT-X la tiene attorno alla frequenza di ricezione)
    let jpk = ((fbest - f1) / DF).round() as i64;
    let s1pk = (jpk - 32..=jpk + 32).map(norm).fold(f32::MIN, f32::max);
    let (mut sig, mut nsig) = (0.0f32, 0u32);
    for j in jpk - 32..=jpk + 32 {
        let v = norm(j);
        if v >= 0.5 * s1pk {
            sig += v;
            nsig += 1;
        }
    }
    let sig = if nsig > 0 { sig / nsig as f32 } else { 0.0 };
    // WSJT-X sottrae 35 dB; con la discesa a 200 Hz di Aurora la stessa misura esce 6 dB piu' alta (tarato su audio sintetico
    // da -23 a -25 dB), quindi qui si sottraggono 41 dB
    let xsnr = (db(sig) - 41.0).max(-30.0);

    // probabilita' dei simboli attesi
    let fft32 = sync::fft_plan(32, false);
    let mut plog = 0.0f32;
    let (mut nhard, mut nsum) = (0u32, 0usize);
    let (mut sum_sync, mut sum_sig, mut sum_big) = (0.0f32, 0.0f32, 0.0f32);
    let start = ((xdt + 0.5) / 0.005).round() as i64;
    for k in 0..NN {
        let i0 = 32 * k as i64 + start;
        let mut csymb: Vec<Complex> = (0..32).map(|i| sync::cd_at(&cd[..NZZ], i0 + i)).collect();
        fft32.process(&mut csymb);
        let mut s8 = [0.0f32; 8];
        for (v, c) in s8.iter_mut().zip(&csymb) {
            *v = c.norm_sqr();
        }
        let total: f32 = s8.iter().sum();
        let want = usize::from(hyp.tones[k]);
        if total > 0.0 {
            plog += (s8[want] / total).ln();
            nsum += 1;
        }
        let mut ipk = 0usize;
        for i in 1..8 {
            if s8[i] > s8[ipk] {
                ipk = i;
            }
        }
        if ipk != want {
            nhard += 1;
        }
        if k < 7 || (36..43).contains(&k) || k >= 72 {
            sum_sync += s8[want];
        } else {
            sum_sig += s8[want];
        }
        sum_big += s8[ipk];
    }
    if nsum < NN {
        plog += (NN - nsum) as f32 * 0.125f32.ln();
    }
    let sigobig = if sum_big > 0.0 { (sum_sync + sum_sig) / sum_big } else { 0.0 };
    if nhard > 54 || plog < -159.0 || sigobig < 0.71 {
        return None;
    }
    Some(A8Hit { text: hyp.text.clone(), freq_hz: fbest, dt_s: xdt, snr_db: xsnr, plog, hard_errors: nhard, tones: hyp.tones })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn le_ipotesi_di_getmsg_sono_quelle_di_wsjtx() {
        assert_eq!(getmsg(1, "K1ABC", "W9XYZ", "EN37"), "K1ABC W9XYZ");
        assert_eq!(getmsg(2, "K1ABC", "W9XYZ", "EN37"), "K1ABC W9XYZ RRR");
        assert_eq!(getmsg(3, "K1ABC", "W9XYZ", "EN37"), "K1ABC W9XYZ RR73");
        assert_eq!(getmsg(4, "K1ABC", "W9XYZ", "EN37"), "K1ABC W9XYZ 73");
        assert_eq!(getmsg(5, "K1ABC", "W9XYZ", "EN37"), "CQ W9XYZ EN37");
        assert_eq!(getmsg(6, "K1ABC", "W9XYZ", "EN37"), "K1ABC W9XYZ EN37");
        // 7..46 sono rapporti sotto -30: vuoti
        assert_eq!(getmsg(7, "K1ABC", "W9XYZ", "EN37"), "");
        assert_eq!(getmsg(47, "K1ABC", "W9XYZ", "EN37"), "K1ABC W9XYZ -30");
        assert_eq!(getmsg(48, "K1ABC", "W9XYZ", "EN37"), "K1ABC W9XYZ R-30");
        assert_eq!(getmsg(107, "K1ABC", "W9XYZ", "EN37"), "K1ABC W9XYZ +00");
        assert_eq!(getmsg(168, "K1ABC", "W9XYZ", "EN37"), "K1ABC W9XYZ R+30");
        assert_eq!(getmsg(169, "K1ABC", "W9XYZ", "EN37"), "");
        assert_eq!(getmsg(207, "K1ABC", "W9XYZ", "EN37"), "");
        assert_eq!(getmsg(207, "K1ABC", "W9XYZ/P", "EN37"), "CQ W9XYZ/P");
        assert_eq!(getmsg(210, "K1ABC", "W9XYZ/P", "EN37"), "<K1ABC> W9XYZ/P RR73");
    }
}
