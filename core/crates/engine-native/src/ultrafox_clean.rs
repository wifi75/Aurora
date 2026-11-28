//! Pulizia dell'audio prima di decodificare UltraFOX/SuperFox, come fa WSJT-X 3.3 in `sfrx_sub.f90` prima di cercare il sincronismo:
//! `sfox_remove_ft8` toglie i segnali FT8 tra 500 e 2500 Hz, `sfox_remove_tone` toglie una portante fissa (il picco stretto piu' forte
//! da 50 Hz sotto la frequenza del sincronismo a 1550 Hz sopra). Senza questa pulizia un FT8 forte o un tono vicino a 750 Hz rubano il
//! picco del sincronismo e la Fox non si aggancia piu'.
//!
//! # Differenze rispetto a WSJT-X
//!
//! - I segnali FT8 da togliere sono, di norma, quelli che il decodificatore FT8 di Aurora ha gia' trovato nello stesso periodo (il Hound
//!   decodifica comunque FT8): si ricostruiscono i toni dal testo e si sottraggono con la stessa sottrazione adattiva del decodificatore.
//!   Cosi' non si decodifica FT8 due volte. Solo se l'elenco non c'e' (prove, altri chiamanti) gira un decodificatore leggero come il
//!   «mini decodificatore» di WSJT-X: una passata, 500-2500 Hz, sincronismo minimo 1,5, 100 candidati, solo propagazione di credenze.
//! - La portante si toglie sull'audio reale, senza passare dal segnale analitico: si porta il tono a banda base con un esponenziale, si
//!   filtra con la finestra a coseno quadro di 8001 campioni e si sottrae `2 Re(filtrato * esponenziale)`; l'immagine a -2f cade a
//!   migliaia di hertz dal filtro (che e' largo pochi hertz) e non conta.

use std::sync::Arc;

use ft8::decode::{self, Decoded, Options};
use ft8::message::{self, CallHashes};
use ft8::subtract::{self, Reference};
use ft8::sync::Downsampler;
use ft8::{crc, ldpc, modem};
use rustfft::{num_complex::Complex32, Fft, FftPlanner};

const RATE: f32 = 12_000.0;
/// Banda in cui WSJT-X cerca i segnali FT8 da togliere.
const FT8_LOW_HZ: f32 = 500.0;
const FT8_HIGH_HZ: f32 = 2500.0;
/// Il periodo intero (15 s).
const NMAX: usize = 15 * 12_000;
/// Lunghezza del filtro che isola la portante (campioni, `NFILT` di `sfox_remove_tone`).
const NFILT: usize = 8000;
/// Una portante e' stretta: oltre questa larghezza (Hz) il picco e' parte di un segnale a modulazione, non un tono.
const MAX_SIGMA_HZ: f32 = 2.5;
/// Campioni da cui si toglie la portante (`nframe = 50 * 3456`).
const NFRAME: usize = 50 * 3456;

/// L'esito della pulizia: l'audio ripulito (se qualcosa e' stato tolto) e che cosa e' stato tolto.
#[derive(Debug, Clone, Default)]
pub struct Cleaned {
    /// `None` se non c'era niente da togliere: si decodifica l'audio originale senza copiarlo.
    pub audio: Option<Vec<f32>>,
    /// Quanti segnali FT8 sono stati sottratti.
    pub ft8_removed: usize,
    /// La frequenza della portante tolta (Hz audio).
    pub tone_hz: Option<f32>,
}

/// Toglie i segnali FT8 e la portante fissa. `ft8`: i messaggi FT8 gia' decodificati in questo periodo; `None` per cercarli con il
/// decodificatore leggero. `fsync`: dove ci si aspetta il sincronismo (750 Hz).
pub fn clean(audio: &[f32], fsync: f32, ft8: Option<&[Decoded]>, hashes: &CallHashes) -> Cleaned {
    let mut out = Cleaned::default();
    if audio.len() < NMAX {
        return out;
    }
    let mut work: Option<Vec<f32>> = None;
    let mini;
    let known: &[Decoded] = match ft8 {
        Some(list) => list,
        None => {
            mini = mini_decode(audio, hashes);
            &mini
        }
    };
    if !known.is_empty() {
        let mut copy = audio.to_vec();
        out.ft8_removed = remove_ft8(&mut copy, known, hashes);
        if out.ft8_removed > 0 {
            work = Some(copy);
        }
    }
    let base = work.as_deref().unwrap_or(audio);
    if let Some((hz, tone_free)) = remove_tone(base, fsync) {
        out.tone_hz = Some(hz);
        work = Some(tone_free);
    }
    out.audio = work;
    out
}

/// Il decodificatore FT8 leggero di `sfox_remove_ft8`: sincronismo 500-2500 Hz, minimo 1,5, fino a 100 candidati, una sola passata,
/// solo propagazione di credenze (`maxosd = -1`), niente analisi a priori, nessun filtro sui falsi.
pub fn mini_decode(audio: &[f32], hashes: &CallHashes) -> Vec<Decoded> {
    let mut o = Options::depth(1);
    o.f_low_hz = FT8_LOW_HZ;
    o.f_high_hz = FT8_HIGH_HZ;
    o.min_sync = 1.5;
    o.max_candidates = 100;
    o.passes = 1;
    o.osd_order = 0;
    o.low_threshold = false;
    o.subpass = false;
    o.filters = false;
    o.ap = None;
    o.ap_cq = false;
    o.a7 = false;
    o.a8 = None;
    o.threads = 1;
    // gli hash servono solo a rileggere il testo: quelli scoperti qui non devono restare nella memoria vera
    let mut local = hashes.clone();
    decode::decode_slot(audio, &o, &mut local)
}

/// I toni (79 simboli) del messaggio, se il testo si impacchetta e passa il filtro dei tipi di `sfox_remove_ft8` (`i3` fino a 5,
/// nessun `n3` oltre 6 con `i3` = 0, niente `i3` = 0 con `n3` = 2).
fn tones_of(text: &str, hashes: &mut CallHashes) -> Option<[u8; 79]> {
    let bits = message::pack(text, hashes).ok()?;
    let (i3, n3) = message::message_type(&bits);
    if i3 > 5 || (i3 == 0 && n3 > 6) || (i3 == 0 && n3 == 2) {
        return None;
    }
    let codeword = ldpc::encode(&crc::append_crc(&bits));
    Some(modem::tones_from_codeword(&codeword))
}

/// Sottrae da `audio` (12 kHz, a +-1) i segnali FT8 dati; torna quanti. Sono presi solo quelli tra 500 e 2500 Hz, certi (non a priori) e
/// il cui testo si ricostruisce (un `<...>` non risolto no). Il ritardo si affina come nel decodificatore (`refine_dt`).
pub fn remove_ft8(audio: &mut [f32], signals: &[Decoded], hashes: &CallHashes) -> usize {
    let mut local = hashes.clone();
    let mut ds: Option<Downsampler> = None;
    let mut stale: Vec<f32> = Vec::new();
    let mut n = 0;
    for s in signals {
        if s.hint || !(FT8_LOW_HZ..=FT8_HIGH_HZ).contains(&s.freq_hz) || s.text.contains("<...>") {
            continue;
        }
        let Some(tones) = tones_of(&s.text, &mut local) else { continue };
        // la FFT lunga si rifa se si e' appena tolto un segnale vicino (50 Hz), come fa il decodificatore
        if ds.is_none() || stale.iter().any(|&f| (f - s.freq_hz).abs() < 50.0) {
            ds = Some(Downsampler::new(audio));
            stale.clear();
        }
        let Some(ds_ref) = ds.as_ref() else { continue };
        let reference = Reference::new(&tones);
        let cd = ds_ref.run(s.freq_hz, false);
        let i0 = ((f64::from(s.dt_s) + 0.5) * 200.0).round() as i64;
        let correction = subtract::refine_dt(&cd, &reference, i0);
        subtract::subtract(audio, &reference, s.freq_hz, f64::from(s.dt_s) + 0.5 + f64::from(correction));
        stale.push(s.freq_hz);
        n += 1;
    }
    n
}

/// `sfox_remove_tone`: se il picco piu' forte tra `fsync - 50` e `fsync + 1550` Hz e' una portante (larghezza quadratica media sotto
/// 2,5 Hz) la toglie. Torna la frequenza e l'audio senza la portante; `None` se non c'e' (o se `fsync` e' oltre 1400 Hz).
pub fn remove_tone(audio: &[f32], fsync: f32) -> Option<(f32, Vec<f32>)> {
    let n = NMAX;
    if audio.len() < n || fsync > 1400.0 || !fsync.is_finite() {
        return None;
    }
    let mut planner = FftPlanner::<f32>::new();
    let fwd = planner.plan_fft_forward(n);
    let inv = planner.plan_fft_inverse(n);
    let mut spec: Vec<Complex32> = audio[..n].iter().map(|&v| Complex32::new(v, 0.0)).collect();
    fwd.process(&mut spec);
    let df = RATE / n as f32;
    let nq = n / 4;
    let s: Vec<f32> = spec[..nq].iter().map(|z| z.norm_sqr()).collect();
    let ia = (((fsync - 50.0) / df).round() as usize).max(1);
    let ib = (((fsync + 1550.0) / df).round() as usize).min(nq - 1);
    if ia >= ib {
        return None;
    }
    let mut i0 = ia;
    for i in ia..=ib {
        if s[i] > s[i0] {
            i0 = i;
        }
    }
    let nbaud = ((RATE / 1024.0) / df).round() as usize;
    let range = |c: usize| (c.saturating_sub(nbaud).max(1), (c + nbaud).min(nq - 1));
    let (lo, hi) = range(i0);
    let (s0, s1) = (lo..=hi).fold((0.0f64, 0.0f64), |(a, b), i| (a + f64::from(s[i]), b + (i as f64 - i0 as f64) * f64::from(s[i])));
    if !(s0 > 0.0) {
        return None;
    }
    // baricentro (arrotondato come in WSJT-X), poi larghezza quadratica media attorno ad esso
    let c = (i0 as f64 + s1 / s0).round() as usize;
    let c = c.clamp(1, nq - 1);
    let (lo, hi) = range(c);
    let s2: f64 = (lo..=hi).map(|i| f64::from(s[i]) * (i as f64 - c as f64).powi(2)).sum();
    let sigma = (s2 / s0).sqrt() as f32 * df;
    if sigma > MAX_SIGMA_HZ {
        return None;
    }
    let f2 = c as f32 * df;

    // banda base: audio * exp(-j w n), poi il filtro a coseno quadro (convoluzione circolare per FFT)
    let w = std::f64::consts::TAU * f64::from(f2) / f64::from(RATE);
    let phasor = |k: usize| {
        let (sn, cs) = (w * k as f64).sin_cos();
        Complex32::new(cs as f32, sn as f32)
    };
    let mut base: Vec<Complex32> = audio[..n].iter().enumerate().map(|(k, &v)| phasor(k).conj() * v).collect();
    fwd.process(&mut base);
    let h = lowpass_response(&fwd, n);
    for (b, g) in base.iter_mut().zip(&h) {
        *b *= *g;
    }
    inv.process(&mut base);
    let mut out = audio.to_vec();
    let nframe = NFRAME.min(n);
    for k in 0..nframe {
        // `inv` non normalizza: 1/n; il fattore 2 recupera la meta' persa nel mettere a banda base un segnale reale
        let z = base[k] * phasor(k) * (2.0 / n as f32);
        out[k] -= z.re;
    }
    Some((f2, out))
}

/// La risposta in frequenza del filtro a coseno quadro di `NFILT + 1` campioni, a somma 1, centrato sul campione 0.
fn lowpass_response(fwd: &Arc<dyn Fft<f32>>, n: usize) -> Vec<Complex32> {
    let half = NFILT / 2;
    let weight = |j: i64| (std::f64::consts::PI * j as f64 / NFILT as f64).cos().powi(2);
    let sum: f64 = (-(half as i64)..=half as i64).map(weight).sum();
    let mut h = vec![Complex32::new(0.0, 0.0); n];
    for j in -(half as i64)..=half as i64 {
        h[j.rem_euclid(n as i64) as usize] = Complex32::new((weight(j) / sum) as f32, 0.0);
    }
    fwd.process(&mut h);
    h
}
