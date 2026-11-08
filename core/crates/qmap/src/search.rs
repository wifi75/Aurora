//! La ricerca panoramica del Q65 (`qmapa.f90`, `getcand2.f90`, `q65b.f90` di WSJT-X 3).
//!
//! 1. Una sola FFT lunga quanto il periodo su tutto il flusso I/Q (`fftbig.f90`): un bin vale `1/periodo` Hz.
//! 2. Lo spettro medio (bin raggruppati a circa 2,93 Hz, come le FFT da 32768 punti a 96 kHz di QMAP) si normalizza con la linea di
//!    base di 16 tratti (40esimo percentile) e si cercano i picchi sopra 1,4: il tono 0 di sincronismo e' acceso in 22 simboli su 85 e
//!    sporge sopra i toni dei dati. Dopo un candidato si azzera la sua larghezza (65 toni) piu' una guardia, come `getcand2`.
//! 3. Per ogni candidato si prendono i bin attorno dalla FFT lunga e si ricostruisce l'audio reale a 12 kHz con il tono 0 a 1000 Hz
//!    (`q65b`: spettro tagliato e antitrasformato), poi si decodifica con [`jt::q65::decode`] in una finestra stretta attorno a 1000 Hz:
//!    il sincronismo vero lo verifica il decodificatore.

use ft8::message::CallHashes;
use jt::q65::{self, Setup};
use rustfft::{FftPlanner, num_complex::Complex};
use serde::{Deserialize, Serialize};

/// Larghezza massima della fetta esplorata (QMAP: 90 kHz utili dei 96 kHz campionati).
pub const MAX_SPAN_HZ: f64 = 90_000.0;
/// Candidati al massimo per periodo (`MAX_CANDIDATES` di `getcand2`).
pub const MAX_CANDIDATES: usize = 50;
const AUDIO_FS: f64 = 12_000.0;
/// Dove finisce il tono 0 nell'audio ricostruito (`q65b`: «Frequency of ipk is now shifted to 1000 Hz»).
const SYNC_AUDIO_HZ: f64 = 1_000.0;
/// Risoluzione dello spettro medio: 96000 / 32768 Hz come `symspec` di QMAP.
const AVG_BIN_HZ: f64 = 96_000.0 / 32_768.0;
const NSEG: usize = 16;
const NPCT: f64 = 0.40;
/// Prima soglia sullo spettro normalizzato (`smin` di `getcand2`).
const SMIN: f32 = 1.4;
/// Guardia in bin dello spettro medio attorno a un candidato.
const NGUARD: usize = 5;
/// Punti dello spettro che si mostrano nell'interfaccia.
pub const SPECTRUM_POINTS: usize = 600;

/// Cosa cercare.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SearchOptions {
    /// Periodo in secondi (15, 30, 60, 120, 300); QMAP lavora a 60 s (e 30 s a richiesta).
    pub period_s: u32,
    /// Sottomodi provati su ogni candidato (0 = A ... 4 = E).
    pub submodes: Vec<u8>,
    /// Larghezza della fetta attorno al centro, in Hz (al massimo [`MAX_SPAN_HZ`] e la frequenza di campionamento).
    pub span_hz: f64,
    pub max_candidates: usize,
    /// Nominativi per le ipotesi AP del decodificatore (vuoti = solo CQ).
    pub my_call: String,
    pub dx_call: String,
    /// Prova anche la metrica dell'evanescenza (EME): piu' lenta.
    pub fading: bool,
}

impl Default for SearchOptions {
    fn default() -> Self {
        SearchOptions { period_s: 60, submodes: vec![0], span_hz: MAX_SPAN_HZ, max_candidates: MAX_CANDIDATES, my_call: String::new(), dx_call: String::new(), fading: false }
    }
}

/// Un messaggio trovato nella fetta.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct QmapDecode {
    /// Frequenza del tono 0 rispetto al centro del flusso I/Q, in Hz.
    pub offset_hz: f64,
    /// Frequenza del tono 0 in radiofrequenza, se il centro e' noto.
    pub rf_hz: Option<u64>,
    pub period_s: u32,
    pub submode: u8,
    /// Nome del modo come WSJT-X («Q65-60A»).
    pub mode: String,
    pub snr_db: i32,
    pub dt_s: f32,
    pub text: String,
    pub sync: f32,
}

/// Esito di una ricerca.
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct SearchResult {
    pub decodes: Vec<QmapDecode>,
    pub candidates: usize,
    /// Spettro medio in dB su [`SearchResult::spectrum_low_hz`, `spectrum_high_hz`] (rispetto al centro).
    pub spectrum_db: Vec<f32>,
    pub spectrum_low_hz: f64,
    pub spectrum_high_hz: f64,
}

/// Nome del modo come lo scrive WSJT-X.
pub fn mode_name(period_s: u32, submode: u8) -> String {
    format!("Q65-{period_s}{}", char::from(b'A' + submode.min(4)))
}

/// Cerca e decodifica i segnali Q65 in un periodo di I/Q (campioni dall'inizio del periodo; quelli mancanti valgono zero).
pub fn search(iq: &[Complex<f32>], sample_rate: u32, center_hz: Option<u64>, options: &SearchOptions, hashes: &mut CallHashes) -> SearchResult {
    let setups: Vec<Setup> = options.submodes.iter().filter_map(|&s| Setup::new(options.period_s, s)).collect();
    if setups.is_empty() || sample_rate == 0 || iq.is_empty() {
        return SearchResult::default();
    }
    let t = f64::from(options.period_s);
    let fs = f64::from(sample_rate);
    let n = options.period_s as usize * sample_rate as usize;
    let mut planner = FftPlanner::<f32>::new();

    // 1. FFT lunga
    let mut x: Vec<Complex<f32>> = iq.iter().take(n).copied().collect();
    x.resize(n, Complex::new(0.0, 0.0));
    planner.plan_fft_forward(n).process(&mut x);
    // indice «spostato» s (da -fs/2 a +fs/2) -> bin della FFT
    let half = n / 2;
    let bin_of = |s: usize| (s + n - half) % n;
    let freq_of = |s: f64| (s - half as f64) / t;

    // 2. spettro medio a gruppi di g bin
    let g = ((AVG_BIN_HZ * t).round() as usize).max(1);
    let df = g as f64 / t;
    let nb = n / g;
    let raw: Vec<f32> = (0..nb).map(|j| (0..g).map(|m| x[bin_of(j * g + m)].norm_sqr()).sum::<f32>() / g as f32).collect();
    let span = options.span_hz.clamp(1_000.0, MAX_SPAN_HZ.min(fs));
    let j_of = |f: f64| (((f * t) + half as f64) / g as f64).floor().clamp(0.0, nb as f64) as usize;
    let (ja, jb) = (j_of(-span / 2.0), j_of(span / 2.0).max(j_of(-span / 2.0) + 1).min(nb));

    let widest = setups.iter().map(|s| s.submode).max().unwrap_or(0);
    let bw = Setup::new(options.period_s, widest).map_or(0.0, |s| s.bandwidth_hz());
    let nbw = (bw / df) as usize + 1;
    let narrowest = setups.iter().map(|s| s.submode).min().unwrap_or(0);
    let nb0 = 2usize << narrowest;

    let mut savg = raw[ja..jb].to_vec();
    let nlen = (savg.len() / NSEG).max(1);
    for seg in savg.chunks_mut(nlen) {
        let mut sorted = seg.to_vec();
        sorted.sort_by(f32::total_cmp);
        let base = sorted[((sorted.len() as f64 * NPCT) as usize).min(sorted.len() - 1)];
        if base > 0.0 {
            seg.iter_mut().for_each(|v| *v /= 1.015 * base);
        }
    }

    // picchi locali sopra la soglia (piu' alta quando i bin medi sono pochi e lo spettro e' piu' rumoroso), presi dal piu' forte: il tono
    // di sincronismo e' il punto piu' alto del suo segnale, cosi' un picco di rumore appena sotto non lo copre azzerandone la larghezza
    // (in `getcand2` lo evita `q65_sync`; qui il sincronismo vero lo verifica il decodificatore)
    let smin = SMIN.max(1.0 + 4.0 / (g as f32).sqrt());
    let last = savg.len().saturating_sub(nbw + NGUARD);
    let mut peaks: Vec<(usize, f32)> = (0..last)
        .filter(|&i| savg[i] >= smin && savg[i.saturating_sub(nb0)..(i + nb0 + 1).min(savg.len())].iter().all(|&v| v <= savg[i]))
        .map(|i| (i, savg[i]))
        .collect();
    peaks.sort_by(|a, b| b.1.total_cmp(&a.1));
    let mut taken = vec![false; savg.len()];
    let mut cands: Vec<(usize, f32)> = Vec::new();
    for (i0, pk) in peaks {
        if taken[i0] {
            continue;
        }
        cands.push((i0, pk));
        if cands.len() >= options.max_candidates.clamp(1, MAX_CANDIDATES) {
            break;
        }
        let (a, b) = (i0.saturating_sub(NGUARD), (i0 + nbw + NGUARD).min(savg.len()));
        taken[a..b].iter_mut().for_each(|v| *v = true);
    }

    // 3. ogni candidato a 12 kHz e al decodificatore
    let m = options.period_s as usize * AUDIO_FS as usize;
    let inverse = planner.plan_fft_inverse(m);
    let scale = 1.0 / (2.0 * n as f32);
    let mut decodes: Vec<QmapDecode> = Vec::new();
    for &(j, _) in &cands {
        let fc = freq_of(((ja + j) * g) as f64 + g as f64 / 2.0);
        let kc = ((fc - SYNC_AUDIO_HZ) * t).round() as i64;
        let lo_hz = (SYNC_AUDIO_HZ - 300.0).max(100.0);
        let hi_hz = (SYNC_AUDIO_HZ + bw + 300.0).min(AUDIO_FS / 2.0 - 100.0);
        let (ka, kb) = ((lo_hz * t) as usize, (hi_hz * t) as usize);
        let edge = (30.0 * t) as usize;
        let mut y = vec![Complex::new(0f32, 0.0); m];
        for k in ka..=kb.min(m / 2 - 1) {
            let taper = {
                let d = (k - ka).min(kb - k);
                if d >= edge { 1.0 } else { 0.5 * (1.0 - (std::f64::consts::PI * d as f64 / edge as f64).cos()) }
            } as f32;
            let bin = (kc + k as i64).rem_euclid(n as i64) as usize;
            let v = x[bin] * (taper * scale);
            y[k] = v;
            y[m - k] = v.conj();
        }
        inverse.process(&mut y);
        let audio: Vec<f32> = y.iter().map(|c| c.re).collect();
        let tol = (2.0 * df + 10.0) as f32;
        for &setup in &setups {
            let o = q65::Options {
                setup,
                f_low_hz: SYNC_AUDIO_HZ as f32 - tol,
                f_high_hz: SYNC_AUDIO_HZ as f32 + tol,
                max_candidates: 4,
                // il candidato e' riportato a 1000 Hz: si prova sempre li', come la frequenza di ricezione di WSJT-X
                rx_hz: Some(SYNC_AUDIO_HZ as f32),
                rx_tol_hz: tol,
                my_call: options.my_call.clone(),
                dx_call: options.dx_call.clone(),
                ap: true,
                fading: options.fading.then_some(q65::Fading::Lorentzian),
                ..q65::Options::default()
            };
            for d in q65::decode(&audio, &o, hashes) {
                if decodes.iter().any(|e| e.text == d.text && e.period_s == setup.period_s) {
                    continue;
                }
                let offset_hz = kc as f64 / t + f64::from(d.freq_hz);
                let rf_hz = center_hz.and_then(|c| u64::try_from(c as i64 + offset_hz.round() as i64).ok());
                decodes.push(QmapDecode {
                    offset_hz,
                    rf_hz,
                    period_s: setup.period_s,
                    submode: setup.submode,
                    mode: mode_name(setup.period_s, setup.submode),
                    snr_db: d.snr_db,
                    dt_s: d.dt_s,
                    text: d.text,
                    sync: d.sync,
                });
            }
        }
    }
    decodes.sort_by(|a, b| a.offset_hz.total_cmp(&b.offset_hz));

    // spettro per l'interfaccia: media a gruppi, in dB
    let width = jb - ja;
    let per = width.div_ceil(SPECTRUM_POINTS).max(1);
    let spectrum_db = raw[ja..jb].chunks(per).map(|c| 10.0 * (c.iter().sum::<f32>() / c.len() as f32).max(1e-20).log10()).collect();
    SearchResult { decodes, candidates: cands.len(), spectrum_db, spectrum_low_hz: freq_of((ja * g) as f64), spectrum_high_hz: freq_of((jb * g) as f64) }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    /// Rumore complesso gaussiano (xorshift + Box-Muller), varianza `sigma^2` per componente.
    pub(crate) fn noise(n: usize, sigma: f32, seed: u64) -> Vec<Complex<f32>> {
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
                let r = (-2.0 * u1.ln()).sqrt();
                let a = 2.0 * std::f64::consts::PI * u2;
                Complex::new(sigma * (r * a.cos()) as f32, sigma * (r * a.sin()) as f32)
            })
            .collect()
    }

    /// Aggiunge un segnale Q65 complesso (fase continua) con il tono 0 a `offset_hz` dal centro.
    pub(crate) fn add_q65(iq: &mut [Complex<f32>], fs: u32, msg: &str, offset_hz: f64, setup: Setup, amp: f32) {
        let tones = q65::tones(msg, &mut CallHashes::default()).expect("messaggio");
        let sps = setup.nsps() * fs as usize / 12_000;
        let at = (setup.start_s() * f64::from(fs)) as usize;
        let mut phase = 0.0f64;
        let two_pi = 2.0 * std::f64::consts::PI;
        for (s, &tone) in tones.iter().enumerate() {
            let dphi = two_pi * (offset_hz + f64::from(tone) * setup.tone_spacing_hz()) / f64::from(fs);
            for k in 0..sps {
                phase = (phase + dphi).rem_euclid(two_pi);
                if let Some(v) = iq.get_mut(at + s * sps + k) {
                    *v += Complex::new(amp * phase.cos() as f32, amp * phase.sin() as f32);
                }
            }
        }
    }

    #[test]
    fn finds_and_decodes_several_signals_across_the_slice() {
        let fs = 96_000;
        let setup = Setup::new(15, 0).expect("15A");
        let mut iq = noise(15 * fs as usize, 1.0, 7);
        // SNR in 2500 Hz: segnale A^2 contro rumore 2 * sigma^2 * 2500 / fs; -12 dB circa
        let amp = (0.063f32 * 2.0 * 2500.0 / fs as f32).sqrt();
        let sent = [("CQ IU3ABC IO91", -31_250.0), ("IU3XYZ K1ABC FN42", 4_321.0), ("CQ DL1AA JO62", 38_770.0)];
        for (msg, off) in sent {
            add_q65(&mut iq, fs, msg, off, setup, amp);
        }
        let options = SearchOptions { period_s: 15, ..SearchOptions::default() };
        let result = search(&iq, fs, Some(144_100_000), &options, &mut CallHashes::default());
        for (msg, off) in sent {
            let d = result.decodes.iter().find(|d| d.text == msg).unwrap_or_else(|| panic!("{msg} non trovato: {:?}", result.decodes));
            assert!((d.offset_hz - off).abs() < 4.0, "{msg}: {} invece di {off}", d.offset_hz);
            assert_eq!(d.mode, "Q65-15A");
            assert_eq!(d.rf_hz, Some((144_100_000.0 + d.offset_hz).round() as u64));
        }
        assert_eq!(result.decodes.len(), sent.len());
        assert!(!result.spectrum_db.is_empty() && result.spectrum_low_hz < -40_000.0 && result.spectrum_high_hz > 40_000.0);
    }

    #[test]
    fn sixty_seconds_with_two_submodes() {
        let fs = 96_000;
        let mut iq = noise(60 * fs as usize, 1.0, 9);
        let amp = (0.05f32 * 2.0 * 2500.0 / fs as f32).sqrt();
        let a = Setup::new(60, 0).expect("60A");
        let b = Setup::new(60, 1).expect("60B");
        add_q65(&mut iq, fs, "IU3ABC K1ABC RR73", -12_000.0, a, amp);
        add_q65(&mut iq, fs, "CQ K1ABC FN42", 21_500.0, b, amp);
        let options = SearchOptions { period_s: 60, submodes: vec![0, 1], ..SearchOptions::default() };
        let result = search(&iq, fs, None, &options, &mut CallHashes::default());
        let find = |t: &str| result.decodes.iter().find(|d| d.text == t).map(|d| (d.mode.clone(), d.offset_hz.round()));
        assert_eq!(find("IU3ABC K1ABC RR73").map(|f| f.0), Some("Q65-60A".to_string()), "{:?}", result.decodes);
        assert_eq!(find("CQ K1ABC FN42").map(|f| f.0), Some("Q65-60B".to_string()), "{:?}", result.decodes);
    }

    #[test]
    fn noise_alone_gives_nothing() {
        let fs = 48_000;
        let iq = noise(15 * fs as usize, 1.0, 3);
        let options = SearchOptions { period_s: 15, ..SearchOptions::default() };
        assert!(search(&iq, fs, None, &options, &mut CallHashes::default()).decodes.is_empty());
    }

    #[test]
    fn mode_names_follow_wsjtx() {
        assert_eq!(mode_name(60, 0), "Q65-60A");
        assert_eq!(mode_name(30, 3), "Q65-30D");
    }
}
