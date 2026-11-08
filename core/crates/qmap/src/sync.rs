//! Gli spettri a mezzo simbolo e la scelta dei candidati con il loro sincronismo, come QMAP (WSJT-X 3, GPL 3): `symspec.f90`,
//! `getcand2.f90`, `q65_sync.f90` della cartella `qmap/libqmap`.
//!
//! - `symspec`: ogni 0,15 s (meta' simbolo del Q65-30, un quarto del Q65-60) una FFT di 0,3 s di I/Q con bin di 96000/32768 Hz; lo
//!   spettro medio e' la somma di queste righe.
//! - `getcand2`: spettro medio normalizzato su 16 tratti (40esimo percentile), picchi sopra 1,4 cercati dal basso verso l'alto; per
//!   ogni picco il sincronismo `q65_sync` deve superare 5 perche' diventi un candidato, poi se ne azzera la larghezza piu' una guardia.
//!   Nel minuto di QMAP i candidati da 60 s si cercano da 30 s di dati in su, quelli da 30 s della prima meta' fino a 30 s e quelli
//!   della seconda da 49,5 s (`nhsym` 200 e 330), saltando la meta' in cui si e' trasmesso (`ntx30a`, `ntx30b`).
//! - `q65_sync`: per ogni ritardo (passi di una riga, fino a 33 righe = 4,95 s) la somma dell'energia dei 22 simboli di sincronismo
//!   attorno al picco; il rapporto fra il massimo e la media delle altre posizioni, diviso per la loro dispersione, e' l'S/N del
//!   sincronismo.
//!
//! Per i periodi diversi da 60 s (non previsti da QMAP) il passo e' un quarto di simbolo dai 60 s in su e mezzo simbolo sotto, la
//! finestra due passi (al massimo 32768 punti a 96 kHz).

use jt::q65::{ISYNC, Setup};
use rustfft::{FftPlanner, num_complex::Complex};

/// Risoluzione degli spettri: 96000 / 32768 Hz come `symspec` di QMAP.
pub const BIN_HZ: f64 = 96_000.0 / 32_768.0;
/// Passo degli spettri nel minuto di QMAP (`hsym=0.15d0*96000.d0`).
pub const QMAP_STEP_S: f64 = 0.15;
/// Righe in un minuto di QMAP (`ss(400,NFFT)`).
pub const QMAP_ROWS: usize = 400;
/// Fine del minuto di QMAP: l'ultima decodifica a 58,5 s (`m_hsymStop`).
pub const NHSYM_FULL: usize = 390;
const NSEG: usize = 16;
const NPCT: f64 = 0.40;
/// Prima soglia sullo spettro normalizzato (`smin`).
const SMIN: f32 = 1.4;
/// Guardia in bin attorno a un candidato (`nguard`).
const NGUARD: usize = 5;
/// S/N minimo del sincronismo (`sync_ok=snr.ge.5.0`).
const SYNC_MIN: f32 = 5.0;
/// Ritardo massimo cercato: 33 righe da 0,15 s (`LAGMAX`).
const LAG_MAX_S: f64 = 33.0 * QMAP_STEP_S;

/// Spettri di potenza a passi regolari su una fetta attorno al centro.
#[derive(Debug, Clone, Default)]
pub struct SymbolSpectra {
    /// Una riga per passo, `width` bin per riga dalla frequenza piu' bassa.
    pub rows: Vec<Vec<f32>>,
    pub step_s: f64,
    /// Larghezza di un bin in Hz.
    pub df: f64,
    /// Il bin 0 della riga e' a `first_bin * df` Hz dal centro.
    pub first_bin: i64,
    pub width: usize,
}

impl SymbolSpectra {
    /// Frequenza del bin `i` rispetto al centro.
    pub fn freq_hz(&self, i: usize) -> f64 {
        (self.first_bin + i as i64) as f64 * self.df
    }

    /// Spettro medio: somma delle righe (`savg`).
    pub fn average(&self) -> Vec<f32> {
        let mut s = vec![0f32; self.width];
        for r in &self.rows {
            s.iter_mut().zip(r).for_each(|(a, b)| *a += b);
        }
        s
    }
}

/// Passo degli spettri per un periodo: 0,15 s per 30 e 60 s come QMAP; mezzo simbolo a 15 s, un quarto di simbolo a 120 e 300 s.
pub fn step_for(period_s: u32) -> f64 {
    match period_s {
        30 | 60 => QMAP_STEP_S,
        p => {
            let sym = Setup::new(p, 0).map_or(0.6, |s| s.nsps() as f64 / 12_000.0);
            if p < 30 { sym / 2.0 } else { sym / 4.0 }
        }
    }
}

/// Righe per simbolo di un periodo con il passo `step_s`.
fn rows_per_symbol(period_s: u32, step_s: f64) -> usize {
    let sym = Setup::new(period_s, 0).map_or(0.6, |s| s.nsps() as f64 / 12_000.0);
    ((sym / step_s).round() as usize).max(1)
}

/// `symspec`: al massimo `max_rows` righe dall'inizio di `iq`, finestra di due passi (al massimo una FFT), fetta di `span_hz`.
pub fn symbol_spectra(iq: &[Complex<f32>], sample_rate: u32, step_s: f64, max_rows: usize, span_hz: f64) -> SymbolSpectra {
    let fs = f64::from(sample_rate);
    let nfft = ((fs / BIN_HZ).round() as usize).max(64);
    let df = fs / nfft as f64;
    let hs = ((step_s * fs).round() as usize).max(1);
    let win = (2 * hs).min(nfft);
    let half_bins = (((span_hz / 2.0) / df).floor() as i64).clamp(1, nfft as i64 / 2 - 1);
    let width = (2 * half_bins + 1) as usize;
    let fft = FftPlanner::<f32>::new().plan_fft_forward(nfft);
    let mut buf = vec![Complex::new(0f32, 0.0); nfft];
    let mut rows = Vec::new();
    for n in 0..max_rows {
        let a = n * hs;
        if a + win > iq.len() {
            break;
        }
        buf[..win].copy_from_slice(&iq[a..a + win]);
        buf[win..].fill(Complex::new(0.0, 0.0));
        fft.process(&mut buf);
        rows.push((0..width).map(|i| buf[(i as i64 - half_bins).rem_euclid(nfft as i64) as usize].norm_sqr()).collect());
    }
    SymbolSpectra { rows, step_s, df, first_bin: -half_bins, width }
}

/// Esito di `q65_sync`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SyncResult {
    pub ok: bool,
    pub snr: f32,
    /// Ritardo trovato, in s, contato come QMAP (`xdt=lagbest*tstep-1.0`).
    pub xdt: f32,
}

/// `q65_sync`: c'e' il tono di sincronismo nel bin `i0` (piu' o meno `m` bin)? `rps` righe per simbolo, `row0` prima riga del periodo.
pub fn q65_sync(ss: &SymbolSpectra, i0: usize, m: usize, rps: usize, row0: usize) -> SyncResult {
    let none = SyncResult { ok: false, snr: 0.0, xdt: 0.0 };
    if ss.width == 0 || i0 >= ss.width {
        return none;
    }
    let lagmax = (LAG_MAX_S / ss.step_s).round() as usize;
    let (i1, i2) = (i0.saturating_sub(m), (i0 + m).min(ss.width - 1));
    let mut ccf = vec![0f32; lagmax + 1];
    for (lag, c) in ccf.iter_mut().enumerate() {
        for &s in &ISYNC {
            let k = rps * (s - 1) + lag + row0;
            for r in 0..rps {
                if let Some(row) = ss.rows.get(k + r) {
                    *c += row[i1..=i2].iter().sum::<f32>();
                }
            }
        }
    }
    let (lagbest, ccfmax) = ccf.iter().copied().enumerate().fold((0usize, f32::MIN), |b, (i, v)| if v > b.1 { (i, v) } else { b });
    let (mut xsum, mut sq, mut nsum) = (0f64, 0f64, 0usize);
    for (i, &v) in ccf.iter().enumerate() {
        if i.abs_diff(lagbest) > 2 {
            xsum += f64::from(v);
            sq += f64::from(v) * f64::from(v);
            nsum += 1;
        }
    }
    if nsum == 0 {
        return none;
    }
    let ave = xsum / nsum as f64;
    let var = sq / nsum as f64 - ave * ave;
    if var <= 0.0 || !var.is_finite() {
        return none;
    }
    let snr = ((f64::from(ccfmax) - ave) / var.sqrt()) as f32;
    SyncResult { ok: snr >= SYNC_MIN, snr, xdt: (lagbest as f64 * ss.step_s - 1.0) as f32 }
}

/// Cosa cercare in `getcand2`.
#[derive(Debug, Clone)]
pub struct CandidateSearch {
    /// Periodo dei candidati principali (60 nel minuto di QMAP).
    pub period_s: u32,
    /// Sottomodi del periodo principale (0 = A); nel minuto i 30 s usano la lettera prima (`max(1,mode_q65-1)`).
    pub submodes: Vec<u8>,
    /// Minuto di QMAP: candidati da 60 s e, con `also_30`, da 30 s nelle due meta'.
    pub minute: bool,
    pub also_30: bool,
    /// Mezzi simboli disponibili (130, 200, 330 o 390) nel minuto di QMAP.
    pub nhsym: usize,
    /// Si e' trasmesso nella prima / seconda meta' del minuto (piu' di 5 s): quei 30 s non si cercano.
    pub skip_30a: bool,
    pub skip_30b: bool,
    pub max_candidates: usize,
}

/// Un candidato.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Candidate {
    /// Bin dello spettro (indice nella fetta).
    pub index: usize,
    /// Frequenza del tono 0 rispetto al centro, in Hz.
    pub freq_hz: f64,
    pub snr: f32,
    pub xdt: f32,
    pub period_s: u32,
    /// 0 prima meta' del minuto, 1 seconda (solo per i 30 s nel minuto).
    pub iseq: u8,
}

/// Sottomodo dei 30 s nel minuto per un sottomodo dei 60 s: la lettera prima, A resta A (`max(1,mode_q65-1)`).
pub fn submode_30(submode: u8) -> u8 {
    submode.saturating_sub(1)
}

/// 40esimo percentile (`pctile`).
fn pctile(v: &[f32], frac: f64) -> f32 {
    let mut s = v.to_vec();
    s.sort_by(f32::total_cmp);
    let j = ((s.len() as f64 * frac).round() as usize).clamp(1, s.len().max(1));
    s.get(j - 1).copied().unwrap_or(0.0)
}

/// `getcand2`: candidati in ordine di frequenza.
pub fn getcand2(ss: &SymbolSpectra, savg0: &[f32], c: &CandidateSearch) -> Vec<Candidate> {
    let n = savg0.len().min(ss.width);
    if n < 2 * NSEG || savg0.last().is_none_or(|&v| v == 0.0) || c.submodes.is_empty() {
        return Vec::new();
    }
    let mut savg = savg0[..n].to_vec();
    for seg in 0..NSEG {
        let (a, b) = (seg * n / NSEG, (seg + 1) * n / NSEG);
        let base = pctile(&savg[a..b], NPCT);
        if base > 0.0 {
            savg[a..b].iter_mut().for_each(|v| *v /= 1.015 * base);
        }
    }

    let main_setups: Vec<Setup> = c.submodes.iter().filter_map(|&s| Setup::new(c.period_s, s)).collect();
    let narrowest = c.submodes.iter().copied().min().unwrap_or(0);
    let widest = main_setups.iter().map(|s| s.bandwidth_hz()).fold(0.0f64, f64::max);
    let nbw = (widest / ss.df) as usize + 1;
    let nb0 = 2usize << narrowest;
    let i2 = n.saturating_sub(nbw + NGUARD);
    // larghezze diverse del tono 0 da provare (`m=nts_q65/2`, a 30 s nel minuto `nts_q65/4`)
    let mut ms_main: Vec<usize> = c.submodes.iter().map(|&s| (1usize << s) / 2).collect();
    ms_main.sort_unstable();
    ms_main.dedup();
    let mut ms_30: Vec<usize> = c.submodes.iter().map(|&s| (1usize << submode_30(s)) / 2).collect();
    ms_30.sort_unstable();
    ms_30.dedup();
    let rps_main = rows_per_symbol(c.period_s, ss.step_s);
    let rps_30 = rows_per_symbol(30, ss.step_s);
    let half_row = (30.0 / ss.step_s).round() as usize;
    let best = |ms: &[usize], i0: usize, rps: usize, row0: usize| -> SyncResult {
        ms.iter().map(|&m| q65_sync(ss, i0, m, rps, row0)).fold(SyncResult { ok: false, snr: f32::MIN, xdt: 0.0 }, |b, r| if r.snr > b.snr { r } else { b })
    };

    let mut out: Vec<Candidate> = Vec::new();
    let max = c.max_candidates.max(1);
    let mut i = 0usize;
    while i < i2 && out.len() < max {
        if savg[i] < SMIN {
            i += 1;
            continue;
        }
        // primo massimo in savg(i:i+nb0) (`maxloc`)
        let end = (i + nb0).min(n - 1);
        let i0 = (i..=end).fold(i, |b, k| if savg[k] > savg[b] { k } else { b });
        let freq_hz = ss.freq_hz(i0);
        let accept = |r: SyncResult, period_s: u32, iseq: u8, savg: &mut [f32], out: &mut Vec<Candidate>| {
            out.push(Candidate { index: i0, freq_hz, snr: r.snr, xdt: r.xdt, period_s, iseq });
            let ia = i.min(i0.saturating_sub(NGUARD));
            let ib = (i0 + nbw + NGUARD).min(n - 1);
            savg[ia..=ib].iter_mut().for_each(|v| *v = 0.0);
        };
        if !c.minute || c.nhsym >= 200 {
            let r = best(&ms_main, i0, rps_main, 0);
            if r.ok {
                accept(r, c.period_s, 0, &mut savg, &mut out);
            }
        }
        if c.minute && c.also_30 {
            if out.len() < max && c.nhsym <= 200 && !c.skip_30a {
                let r = best(&ms_30, i0, rps_30, 0);
                if r.ok {
                    accept(r, 30, 0, &mut savg, &mut out);
                }
            }
            if out.len() < max && c.nhsym >= 330 && !c.skip_30b {
                let r = best(&ms_30, i0, rps_30, half_row);
                if r.ok {
                    accept(r, 30, 1, &mut savg, &mut out);
                }
            }
        }
        i += 1;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::search::tests::{add_q65, noise};

    #[test]
    fn steps_follow_qmap() {
        assert_eq!(step_for(60), 0.15);
        assert_eq!(step_for(30), 0.15);
        assert_eq!(rows_per_symbol(60, 0.15), 4);
        assert_eq!(rows_per_symbol(30, 0.15), 2);
        assert_eq!(submode_30(0), 0);
        assert_eq!(submode_30(2), 1);
    }

    #[test]
    fn the_sync_tone_is_found_and_noise_is_not() {
        let fs = 96_000;
        let setup = Setup::new(60, 0).expect("60A");
        let mut iq = noise(60 * fs as usize, 1.0, 21);
        let amp = (0.1f32 * 2.0 * 2500.0 / fs as f32).sqrt();
        add_q65(&mut iq, fs, "CQ IU3ABC IO91", 10_000.0, setup, amp);
        let ss = symbol_spectra(&iq, fs, 0.15, QMAP_ROWS, 90_000.0);
        assert_eq!(ss.rows.len(), 399);
        let savg = ss.average();
        let c = CandidateSearch { period_s: 60, submodes: vec![0], minute: true, also_30: true, nhsym: NHSYM_FULL, skip_30a: false, skip_30b: false, max_candidates: 50 };
        let cands = getcand2(&ss, &savg, &c);
        let hit = cands.iter().find(|k| (k.freq_hz - 10_000.0).abs() < 6.0).unwrap_or_else(|| panic!("{cands:?}"));
        assert_eq!((hit.period_s, hit.iseq), (60, 0));
        assert!(cands.len() <= 3, "troppi candidati dal rumore: {cands:?}");
    }
}
