//! Sottrazione di un segnale gia' decodificato (`lib/ft8v2/subtractft8.f90`, `lib/cwfilter.f90` e il raffinamento del ritardo di
//! `lib/ft8b.f90:1990-2007`, JTDX 2.2.159).
//!
//! Il segnale ricevuto e' `dd(t) = a(t) cos(2 pi f0 t + theta(t))`, il riferimento e' `cref(t) = exp(j (2 pi f0 t + phi(t)))` con la
//! fase della modulazione GFSK del messaggio decodificato. L'ampiezza complessa e' `cfilt(t) = LPF[dd(t) conj(cref(t))]`: un filtro
//! passa-basso a coseno quadro di 4001 punti (lunghezza `NFILT1 = 4000`, 0,33 s), applicato per FFT sull'intero periodo, con la
//! correzione dei bordi (`endcorr`). Si toglie `2 Re{cref cfilt}`. Siccome `cfilt` varia nel tempo, la sottrazione segue
//! evanescenza e piccole derive di fase e di ampiezza, cosa che una sola ampiezza per messaggio non fa.
//!
//! Prima di sottrarre, [`refine_dt`] affina il ritardo con tre correlazioni (a -10, 0 e +10 campioni a 200 Hz) e la parabola
//! (`peakup`): il picco del sincronismo di `sync8d` e' a passi di 5 ms, un errore residuo di qualche millisecondo lascia un residuo
//! visibile sotto un segnale forte.
//!
//! # Differenze dal Fortran
//!
//! - Solo il filtro lungo (`NFILT1`); il `NFILT2` di `swl` non e' portato.
//! - L'onda di riferimento si genera una volta a frequenza 0 ([`Reference::new`]) e si porta a `f0` con un rotore complesso invece di
//!   rigenerarla con la tabella a 65536 punti di `gen_ft8wave`: stessa fase, errore trascurabile.
//! - La FFT lunga e' quella di `rustfft` in virgola mobile a 32 bit.

use std::f64::consts::PI;
use std::sync::{Arc, OnceLock};

use rustfft::num_complex::Complex32;
use rustfft::{Fft, FftPlanner};

use crate::modem::{self, GFSK_BT};
use crate::sync::Complex;
use crate::{NN, SAMPLE_RATE};

/// Campioni dell'onda di un messaggio (`NFRAME`).
const NFRAME: usize = crate::NN * crate::NSPS;
/// Lunghezza della FFT: i 15 secondi di audio (`NMAX`).
const NFFT: usize = 180_000;
/// Lunghezza del filtro (`NFILT1`).
const NFILT1: usize = 4000;
/// Scarto in campioni a 200 Hz usato da `peakup` (`noff`).
const NOFF: i64 = 10;

const ZERO: Complex = Complex32::new(0.0, 0.0);

/// Filtro passa-basso e correzione dei bordi, calcolati una volta sola.
struct Filter {
    /// Trasformata del filtro, gia' divisa per `NFFT` (`cw` di `cwfilter`).
    cw: Vec<Complex>,
    /// Correzione dei primi e degli ultimi `NFILT1/2 + 1` campioni (`endcorr`).
    endcorr: Vec<f32>,
    forward: Arc<dyn Fft<f32>>,
    inverse: Arc<dyn Fft<f32>>,
}

fn filter() -> &'static Filter {
    static FILTER: OnceLock<Filter> = OnceLock::new();
    FILTER.get_or_init(|| {
        let half = (NFILT1 / 2) as i64;
        // window1(j) = cos(pi j / NFILT1)^2 per j = -2000..2000
        let window = |j: i64| -> f64 { (PI * j as f64 / NFILT1 as f64).cos().powi(2) };
        let sumw: f64 = (-half..=half).map(window).sum();
        // cw(1:NFILT1+1) = window/sumw, poi cshift di NFILT1/2+1: l'elemento (da 0) n vale window(n + 1) per n = -2001..1999
        let mut cw = vec![ZERO; NFFT];
        for j in -half..=half {
            let n = (j - 1).rem_euclid(NFFT as i64) as usize;
            cw[n] = Complex::new((window(j) / sumw) as f32, 0.0);
        }
        let mut planner = FftPlanner::<f32>::new();
        let forward = planner.plan_fft_forward(NFFT);
        let inverse = planner.plan_fft_inverse(NFFT);
        forward.process(&mut cw);
        let fac = 1.0 / NFFT as f32;
        for c in cw.iter_mut() {
            *c *= fac;
        }
        // endcorr(j) = 1 / (1 - sum(window1(j-1:2000)) / sumw), j = 1..2001
        let mut tail = vec![0.0f64; half as usize + 2];
        for m in (0..=half).rev() {
            tail[m as usize] = tail[m as usize + 1] + window(m);
        }
        let endcorr: Vec<f32> = (0..=half as usize).map(|m| (1.0 / (1.0 - tail[m] / sumw)) as f32).collect();
        Filter { cw, endcorr, forward, inverse }
    })
}

/// L'onda complessa `exp(j fase)` del messaggio a frequenza 0, con le rampe di `gen_ft8wave`.
pub struct Reference {
    base: Vec<Complex>,
}

impl Reference {
    /// Dai 79 toni (con la modulazione GFSK BT = 2 della trasmissione).
    pub fn new(tones: &[u8; NN]) -> Reference {
        let (sin, cos) = modem::gfsk_iq(tones, 0.0, GFSK_BT);
        Reference { base: cos.iter().zip(sin.iter()).map(|(&c, &s)| Complex::new(c, s)).collect() }
    }

    /// Il riferimento a 200 Hz di campionamento (un campione ogni 60), `csig0(k)` di `ft8b`: `index` da 0.
    fn at_200hz(&self, index: usize) -> Complex {
        self.base.get(index * 60).copied().unwrap_or(ZERO)
    }

    /// L'onda portata a `f0_hz` (rotore complesso in doppia precisione, ricalibrato di tanto in tanto).
    fn at_frequency(&self, f0_hz: f32) -> Vec<Complex> {
        let step = 2.0 * PI * f64::from(f0_hz) / SAMPLE_RATE as f64;
        let (sin_step, cos_step) = step.sin_cos();
        let (mut re, mut im) = (1.0f64, 0.0f64);
        let mut out = Vec::with_capacity(self.base.len());
        for (i, b) in self.base.iter().enumerate() {
            if i % 4096 == 0 {
                let phase = step * i as f64;
                (im, re) = phase.sin_cos();
            }
            out.push(Complex::new((f64::from(b.re) * re - f64::from(b.im) * im) as f32, (f64::from(b.re) * im + f64::from(b.im) * re) as f32));
            let next_re = re * cos_step - im * sin_step;
            im = re * sin_step + im * cos_step;
            re = next_re;
        }
        out
    }
}

/// Correzione del ritardo prima di sottrarre (`peakup` su tre correlazioni), in secondi da sommare al ritardo del segnale.
///
/// `cd` e' il segnale a 200 Hz centrato sulla frequenza corretta del messaggio, `i0` il campione (a 200 Hz) in cui comincia il primo
/// simbolo. Come in `ft8b`, la correzione si applica solo se la parabola cade entro un passo (`|dx| <= 1`).
pub fn refine_dt(cd: &[Complex], reference: &Reference, i0: i64) -> f32 {
    let at = |idx: i64| -> Complex { if idx >= 0 && (idx as usize) < cd.len() { cd[idx as usize] } else { ZERO } };
    let (mut sync0, mut syncp, mut syncm) = (0.0f64, 0.0f64, 0.0f64);
    for i in 0..NN {
        let mut z = [ZERO; 3];
        for j in 0..32usize {
            let r = reference.at_200hz(32 * i + j).conj();
            let base = i0 + (i * 32 + j) as i64;
            z[0] += at(base) * r;
            z[1] += at(base + NOFF) * r;
            z[2] += at(base - NOFF) * r;
        }
        sync0 += f64::from(z[0].norm_sqr());
        syncp += f64::from(z[1].norm_sqr());
        syncm += f64::from(z[2].norm_sqr());
    }
    // peakup(ym, y0, yp, dx)
    let b = syncp - syncm;
    let c = syncp + syncm - 2.0 * sync0;
    let dx = -((b / c) / 2.0);
    if !dx.is_finite() || dx.abs() > 1.0 {
        return 0.0;
    }
    (NOFF as f64 * dx * 0.005) as f32
}

/// Toglie dall'audio (12 kHz, nella scala a 16 bit di `decode`) il segnale di toni `reference` a `f0_hz`, che comincia a `dt_s`
/// secondi dall'inizio del periodo (ritardo assoluto, cioe' con lo 0,5 s incluso).
pub fn subtract(audio: &mut [f32], reference: &Reference, f0_hz: f32, dt_s: f64) {
    if !f0_hz.is_finite() || !dt_s.is_finite() {
        return;
    }
    let filt = filter();
    let cref = reference.at_frequency(f0_hz);
    // nstart = dt*12000+1 (da 1): in indice da 0 e' il troncamento verso zero di dt*12000 + 1, meno uno
    let start = (dt_s * SAMPLE_RATE as f64 + 1.0) as i64 - 1;
    let limit = audio.len().min(NFFT) as i64;

    let mut cfilt = vec![ZERO; NFFT];
    for (i, (c, r)) in cfilt.iter_mut().zip(cref.iter()).enumerate().take(NFRAME) {
        let id = start + i as i64;
        if id >= 0 && id < limit {
            *c = r.conj() * audio[id as usize];
        }
    }
    filt.forward.process(&mut cfilt);
    for (c, w) in cfilt.iter_mut().zip(filt.cw.iter()) {
        *c *= *w;
    }
    filt.inverse.process(&mut cfilt);
    // correzione dei bordi: i primi e gli ultimi NFILT1/2 + 1 campioni dell'onda
    for (k, &corr) in filt.endcorr.iter().enumerate() {
        cfilt[k] *= corr;
        cfilt[NFRAME - 1 - k] *= corr;
    }
    for (i, (c, r)) in cfilt.iter().zip(cref.iter()).enumerate().take(NFRAME) {
        let j = start + i as i64;
        if j >= 0 && j < limit {
            audio[j as usize] -= 2.0 * (c.re * r.re - c.im * r.im);
        }
    }
}

/// Sottrazione di una sola ampiezza e fase per tutto il messaggio (quella che Aurora faceva prima): serve a confronto nelle misure.
pub fn subtract_constant(audio: &mut [f32], tones: &[u8; NN], f0_hz: f32, dt_s: f64) {
    let (sin_ref, cos_ref) = modem::gfsk_iq(tones, f0_hz, GFSK_BT);
    let start = (dt_s * SAMPLE_RATE as f64).round() as i64;
    let (mut xs, mut xc, mut ss, mut cc) = (0.0f64, 0.0f64, 0.0f64, 0.0f64);
    for (i, (&s, &c)) in sin_ref.iter().zip(cos_ref.iter()).enumerate() {
        let at = start + i as i64;
        if at < 0 || at as usize >= audio.len() {
            continue;
        }
        let x = f64::from(audio[at as usize]);
        xs += x * f64::from(s);
        xc += x * f64::from(c);
        ss += f64::from(s) * f64::from(s);
        cc += f64::from(c) * f64::from(c);
    }
    if ss <= 0.0 || cc <= 0.0 {
        return;
    }
    let (a, b) = (xs / ss, xc / cc);
    for (i, (&s, &c)) in sin_ref.iter().zip(cos_ref.iter()).enumerate() {
        let at = start + i as i64;
        if at < 0 || at as usize >= audio.len() {
            continue;
        }
        audio[at as usize] -= (a * f64::from(s) + b * f64::from(c)) as f32;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::message::CallHashes;

    /// Energia del segnale nell'intervallo [da, a) in un'onda.
    fn energy(x: &[f32]) -> f64 {
        x.iter().map(|&v| f64::from(v) * f64::from(v)).sum()
    }

    fn tones_of(text: &str) -> [u8; 79] {
        let bits = crate::message::pack(text, &mut CallHashes::default()).expect("messaggio valido");
        let with_crc = crate::crc::append_crc(&bits);
        modem::tones_from_codeword(&crate::ldpc::encode(&with_crc))
    }

    /// Un'onda con ampiezza che varia lentamente (evanescenza del 30%) e un ritardo non multiplo di un campione a 200 Hz.
    fn faded_signal(tones: &[u8; 79], f0: f32, start: usize) -> Vec<f32> {
        let wave = modem::gfsk_wave(tones, f0);
        let mut audio = vec![0.0f32; crate::SLOT_SAMPLES];
        for (i, &w) in wave.iter().enumerate() {
            let t = i as f64 / SAMPLE_RATE as f64;
            let gain = 8000.0 * (1.0 + 0.3 * (2.0 * PI * 0.4 * t + 1.0).sin());
            audio[start + i] += (gain * f64::from(w)) as f32;
        }
        audio
    }

    #[test]
    fn la_sottrazione_con_filtro_toglie_piu_energia_di_una_ampiezza_sola() {
        let tones = tones_of("CQ K1ABC FN42");
        let (f0, start) = (1500.0f32, 6000usize);
        let original = faded_signal(&tones, f0, start);
        let before = energy(&original);

        let mut adaptive = original.clone();
        subtract(&mut adaptive, &Reference::new(&tones), f0, start as f64 / SAMPLE_RATE as f64);
        let mut constant = original.clone();
        subtract_constant(&mut constant, &tones, f0, start as f64 / SAMPLE_RATE as f64);

        let (ra, rc) = (10.0 * (energy(&adaptive) / before).log10(), 10.0 * (energy(&constant) / before).log10());
        println!("residuo: filtro {ra:.1} dB, ampiezza costante {rc:.1} dB");
        assert!(ra < -25.0, "residuo con il filtro {ra} dB");
        assert!(ra < rc - 8.0, "il filtro deve battere l'ampiezza costante di molto: {ra} contro {rc}");
    }

    #[test]
    fn il_residuo_resta_basso_senza_evanescenza() {
        let tones = tones_of("K1ABC W9XYZ -12");
        let (f0, start) = (2100.0f32, 6000usize);
        let wave = modem::gfsk_wave(&tones, f0);
        let mut audio = vec![0.0f32; crate::SLOT_SAMPLES];
        for (i, &w) in wave.iter().enumerate() {
            audio[start + i] = 5000.0 * w;
        }
        let before = energy(&audio);
        subtract(&mut audio, &Reference::new(&tones), f0, start as f64 / SAMPLE_RATE as f64);
        let residual = 10.0 * (energy(&audio) / before).log10();
        assert!(residual < -40.0, "residuo {residual} dB");
    }

    #[test]
    fn il_rotore_coincide_con_la_generazione_diretta() {
        let tones = tones_of("CQ K1ABC FN42");
        let direct = modem::gfsk_iq(&tones, 1234.5, GFSK_BT);
        let rotated = Reference::new(&tones).at_frequency(1234.5);
        let worst = rotated
            .iter()
            .zip(direct.1.iter().zip(direct.0.iter()))
            .map(|(r, (&c, &s))| (r.re - c).abs().max((r.im - s).abs()))
            .fold(0.0f32, f32::max);
        assert!(worst < 2e-3, "scarto massimo {worst}");
    }

    #[test]
    fn un_buffer_corto_o_un_ritardo_fuori_scala_non_vanno_in_panico() {
        let tones = tones_of("CQ K1ABC FN42");
        let reference = Reference::new(&tones);
        let mut short = vec![1.0f32; 1000];
        subtract(&mut short, &reference, 1500.0, 0.3);
        let mut audio = vec![1.0f32; crate::SLOT_SAMPLES];
        subtract(&mut audio, &reference, 1500.0, -5.0);
        subtract(&mut audio, &reference, 1500.0, 20.0);
        subtract(&mut audio, &reference, f32::NAN, 0.3);
        assert!(refine_dt(&[], &reference, 0).abs() <= 0.05);
    }
}
