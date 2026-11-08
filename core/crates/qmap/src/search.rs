//! La ricerca panoramica del Q65 (`qmapa.f90`, `getcand2.f90`, `q65b.f90` di WSJT-X 3).
//!
//! 1. Spettri a mezzo simbolo e candidati con il loro sincronismo ([`crate::sync`]: `symspec`, `getcand2`, `q65_sync`).
//! 2. Una sola FFT lunga quanto il periodo su tutto il flusso I/Q (`fftbig.f90`): un bin vale `1/periodo` Hz.
//! 3. Per ogni candidato si prendono i bin attorno dalla FFT lunga e si ricostruisce l'audio reale a 12 kHz con il tono 0 a 1000 Hz
//!    (`q65b`: spettro tagliato e antitrasformato; per i 30 s della seconda meta' del minuto si tiene la seconda meta'), poi si
//!    decodifica con [`jt::q65`] in una finestra stretta attorno a 1000 Hz, con il ritardo dell'eco lunare (`emedelay`).
//!
//! Nel minuto di QMAP (periodo 60 s) la ricerca si fa piu' volte mentre il minuto si riempie (`nhsym` 130, 200, 330 e 390, cioe'
//! 19,5, 30, 49,5 e 58,5 s): i candidati gia' decodificati nello stesso minuto si saltano (`found`), e chi e' in ritardo sul tempo
//! concesso a ogni passaggio smette (`tsec` di `qmapa`).
//!
//! Media dei periodi: QMAP passa ogni candidato al decodificatore Q65 di WSJT-X, che somma i periodi della stessa parita'. Qui ogni
//! segnale ha la sua media (periodo, sottomodo, meta' del minuto e frequenza entro due bin), alimentata a ogni periodo completo anche
//! quando il sincronismo di quel periodo non basta per farne un candidato.

use std::time::Instant;

use ft8::message::CallHashes;
use jt::q65::{self, Averager, Setup};
use rustfft::{FftPlanner, num_complex::Complex};
use serde::{Deserialize, Serialize};

use crate::sync::{self, CandidateSearch, NHSYM_FULL, QMAP_ROWS, submode_30};

/// Larghezza massima della fetta esplorata (QMAP: 90 kHz utili dei 96 kHz campionati).
pub const MAX_SPAN_HZ: f64 = 90_000.0;
/// Candidati al massimo per periodo (`MAX_CANDIDATES` di `getcand2`).
pub const MAX_CANDIDATES: usize = 50;
const AUDIO_FS: f64 = 12_000.0;
/// Dove finisce il tono 0 nell'audio ricostruito (`q65b`: «Frequency of ipk is now shifted to 1000 Hz»).
const SYNC_AUDIO_HZ: f64 = 1_000.0;
/// Punti dello spettro che si mostrano nell'interfaccia.
pub const SPECTRUM_POINTS: usize = 600;
/// Medie tenute al massimo e periodi senza aggiornamenti dopo i quali una media si butta.
const MAX_AVERAGES: usize = 100;
const AVERAGE_EXPIRY: u64 = 20;
/// Stesso candidato gia' decodificato nel minuto: entro 5 Hz (`abs(f0-found(j)%f).lt.0.005`).
const FOUND_TOL_HZ: f64 = 5.0;

/// Cosa cercare.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SearchOptions {
    /// Periodo in secondi (15, 30, 60, 120, 300); QMAP lavora a 60 s (e 30 s nel minuto con `also_30`).
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
    /// Con il periodo di 60 s cerca anche i Q65-30 nelle due meta' del minuto («Also Q65-30x» di QMAP).
    pub also_30: bool,
    /// Media dei periodi per ogni segnale.
    pub average: bool,
}

impl Default for SearchOptions {
    fn default() -> Self {
        SearchOptions { period_s: 60, submodes: vec![0], span_hz: MAX_SPAN_HZ, max_candidates: MAX_CANDIDATES, my_call: String::new(), dx_call: String::new(), fading: false, also_30: false, average: false }
    }
}

/// Un passaggio della ricerca su un periodo (intero o in parte).
#[derive(Debug, Clone, Copy)]
pub struct Pass {
    /// Mezzi simboli di dati nel minuto di QMAP (130, 200, 330, 390); negli altri periodi conta solo il periodo intero.
    pub nhsym: usize,
    /// Numero d'ordine del periodo (minuti nel modo a 60 s): fa la parita' della media.
    pub period_index: u64,
    /// Trasmesso nella prima / seconda meta' del minuto.
    pub skip_30a: bool,
    pub skip_30b: bool,
    /// Oltre questo istante non si provano altri candidati (solo in tempo reale).
    pub deadline: Option<Instant>,
}

impl Pass {
    /// Il periodo intero, senza limiti di tempo (file).
    pub fn full(period_index: u64) -> Pass {
        Pass { nhsym: NHSYM_FULL, period_index, skip_30a: false, skip_30b: false, deadline: None }
    }
}

/// Un messaggio trovato nella fetta.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct QmapDecode {
    /// Frequenza del tono 0 rispetto al centro del flusso I/Q, in Hz.
    pub offset_hz: f64,
    /// Frequenza del tono 0 in radiofrequenza, se il centro e' noto.
    pub rf_hz: Option<u64>,
    /// Frequenza «sked» (`fsked` di QMAP): `rf_hz` meno meta' del Doppler della propria eco, dove trasmette chi corregge il suo.
    #[serde(default)]
    pub sked_hz: Option<u64>,
    pub period_s: u32,
    pub submode: u8,
    /// Nome del modo come WSJT-X («Q65-60A»).
    pub mode: String,
    /// Meta' del minuto per i Q65-30 nel minuto di QMAP (0 prima, 1 seconda).
    #[serde(default)]
    pub iseq: u8,
    pub snr_db: i32,
    pub dt_s: f32,
    pub text: String,
    pub sync: f32,
    /// Periodi sommati (1 = un periodo solo).
    #[serde(default = "one")]
    pub averaged: u32,
}

fn one() -> u32 {
    1
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

/// La media di un segnale.
#[derive(Debug, Clone)]
struct AvgSlot {
    period_s: u32,
    submode: u8,
    iseq: u8,
    center_hz: Option<u64>,
    /// Frequenza del tono 0 dal centro con cui si ricostruisce l'audio (fissa, cosi' i periodi si sommano allineati).
    freq_hz: f64,
    averager: Averager,
    /// Ultimo periodo sommato (indice di parita').
    last: u64,
}

/// Quello che resta fra un passaggio e l'altro: i candidati gia' decodificati nel minuto e le medie.
#[derive(Debug, Clone, Default)]
pub struct SearchState {
    found: Vec<(f64, u32, u8)>,
    averages: Vec<AvgSlot>,
}

impl SearchState {
    /// Nuovo minuto (o periodo): si dimenticano i candidati decodificati.
    pub fn new_period(&mut self) {
        self.found.clear();
    }

    /// «Cancella media».
    pub fn clear_average(&mut self) {
        self.averages.clear();
    }

    /// Segnali con una media in corso.
    pub fn averaging(&self) -> usize {
        self.averages.len()
    }
}

/// Nome del modo come lo scrive WSJT-X.
pub fn mode_name(period_s: u32, submode: u8) -> String {
    format!("Q65-{period_s}{}", char::from(b'A' + submode.min(4)))
}

/// La FFT lunga del periodo e quello che serve per ricostruire l'audio di un candidato.
struct LongFft {
    x: Vec<Complex<f32>>,
    /// Durata del periodo in s.
    t: f64,
    inverse: std::sync::Arc<dyn rustfft::Fft<f32>>,
}

impl LongFft {
    /// `q65b`: audio reale a 12 kHz di tutto il periodo con il tono 0 (a `fc` Hz dal centro) a 1000 Hz; `bw` larghezza del segnale.
    fn audio(&self, fc: f64, bw: f64) -> (i64, Vec<f32>) {
        let (t, n) = (self.t, self.x.len());
        let m = (t * AUDIO_FS) as usize;
        let kc = ((fc - SYNC_AUDIO_HZ) * t).round() as i64;
        let lo_hz = (SYNC_AUDIO_HZ - 300.0).max(100.0);
        let hi_hz = (SYNC_AUDIO_HZ + bw + 300.0).min(AUDIO_FS / 2.0 - 100.0);
        let (ka, kb) = ((lo_hz * t) as usize, (hi_hz * t) as usize);
        let edge = ((30.0 * t) as usize).max(1);
        let scale = 1.0 / (2.0 * n as f32);
        let mut y = vec![Complex::new(0f32, 0.0); m];
        for k in ka..=kb.min(m / 2 - 1) {
            let d = (k - ka).min(kb - k);
            let taper = if d >= edge { 1.0 } else { 0.5 * (1.0 - (std::f64::consts::PI * d as f64 / edge as f64).cos()) } as f32;
            let bin = (kc + k as i64).rem_euclid(n as i64) as usize;
            let v = self.x[bin] * (taper * scale);
            y[k] = v;
            y[m - k] = v.conj();
        }
        self.inverse.process(&mut y);
        (kc, y.iter().map(|c| c.re).collect())
    }
}

/// Cerca e decodifica i segnali Q65 in un periodo di I/Q (campioni dall'inizio del periodo; quelli mancanti valgono zero).
pub fn search(iq: &[Complex<f32>], sample_rate: u32, center_hz: Option<u64>, options: &SearchOptions, hashes: &mut CallHashes) -> SearchResult {
    search_pass(iq, sample_rate, center_hz, options, &Pass::full(0), &mut SearchState::default(), hashes)
}

/// Un passaggio della ricerca (vedi [`Pass`]), con lo stato del minuto e le medie.
pub fn search_pass(iq: &[Complex<f32>], sample_rate: u32, center_hz: Option<u64>, options: &SearchOptions, pass: &Pass, state: &mut SearchState, hashes: &mut CallHashes) -> SearchResult {
    let mut submodes: Vec<u8> = options.submodes.iter().copied().filter(|&s| s <= 4).collect();
    submodes.sort_unstable();
    submodes.dedup();
    if submodes.is_empty() || Setup::new(options.period_s, 0).is_none() || sample_rate == 0 || iq.is_empty() {
        return SearchResult::default();
    }
    let minute = options.period_s == 60;
    let t = f64::from(options.period_s);
    let fs = f64::from(sample_rate);
    let n = options.period_s as usize * sample_rate as usize;
    let span = options.span_hz.clamp(1_000.0, MAX_SPAN_HZ.min(fs));
    let data = &iq[..iq.len().min(n)];

    // 1. spettri a mezzo simbolo e candidati
    let step = sync::step_for(options.period_s);
    let rows = if minute { pass.nhsym.min(QMAP_ROWS) } else { (t / step).round() as usize };
    let ss = sync::symbol_spectra(data, sample_rate, step, rows, span);
    let savg = ss.average();
    let cands = sync::getcand2(
        &ss,
        &savg,
        &CandidateSearch {
            period_s: options.period_s,
            submodes: submodes.clone(),
            minute,
            also_30: minute && options.also_30,
            nhsym: if minute { pass.nhsym } else { NHSYM_FULL },
            skip_30a: pass.skip_30a,
            skip_30b: pass.skip_30b,
            max_candidates: options.max_candidates.clamp(1, MAX_CANDIDATES),
        },
    );

    // 2. FFT lunga
    let mut planner = FftPlanner::<f32>::new();
    let mut x: Vec<Complex<f32>> = data.to_vec();
    x.resize(n, Complex::new(0.0, 0.0));
    planner.plan_fft_forward(n).process(&mut x);
    let long = LongFft { x, t, inverse: planner.plan_fft_inverse((t * AUDIO_FS) as usize) };

    // 3. decodifica dei candidati
    let tol = (2.0 * ss.df + 10.0) as f32;
    let decode_options = |setup: Setup| q65::Options {
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
        eme_delay: true,
        ..q65::Options::default()
    };
    // un tipo di candidato e' completo (si somma nella media) quando il suo periodo e' finito
    let complete = |period_s: u32, iseq: u8| !minute || if period_s == 30 && iseq == 0 { pass.nhsym >= 200 } else { pass.nhsym >= NHSYM_FULL };
    let parity_index = |period_s: u32, iseq: u8| if minute && period_s == 30 { pass.period_index * 2 + u64::from(iseq) } else { pass.period_index };
    let setups_for = |period_s: u32| -> Vec<Setup> {
        let mut subs: Vec<u8> = if minute && period_s == 30 { submodes.iter().map(|&s| submode_30(s)).collect() } else { submodes.clone() };
        subs.dedup();
        subs.iter().filter_map(|&s| Setup::new(period_s, s)).collect()
    };
    // audio a 12 kHz di un candidato (tutto il minuto; per i 30 s la meta' giusta)
    let audio_for = |fc: f64, period_s: u32, iseq: u8, bw: f64| -> (i64, Vec<f32>) {
        let (kc, audio) = long.audio(fc, bw);
        if minute && period_s == 30 {
            let half = (30.0 * AUDIO_FS) as usize;
            let a = usize::from(iseq) * half;
            (kc, audio[a.min(audio.len())..(a + half).min(audio.len())].to_vec())
        } else {
            (kc, audio)
        }
    };
    let mut decodes: Vec<QmapDecode> = Vec::new();
    let push = |decodes: &mut Vec<QmapDecode>, d: q65::Decoded, kc: i64, setup: Setup, iseq: u8| -> bool {
        if decodes.iter().any(|e| e.text == d.text && e.period_s == setup.period_s && e.iseq == iseq) {
            return false;
        }
        let offset_hz = kc as f64 / t + f64::from(d.freq_hz);
        let rf_hz = center_hz.and_then(|c| u64::try_from(c as i64 + offset_hz.round() as i64).ok());
        decodes.push(QmapDecode {
            offset_hz,
            rf_hz,
            sked_hz: None,
            period_s: setup.period_s,
            submode: setup.submode,
            mode: mode_name(setup.period_s, setup.submode),
            iseq,
            snr_db: d.snr_db,
            dt_s: d.dt_s,
            text: d.text,
            sync: d.sync,
            averaged: d.averaged.max(1),
        });
        true
    };

    let near = 2.0 * ss.df;
    for cand in &cands {
        if pass.deadline.is_some_and(|d| Instant::now() > d) {
            tracing::debug!("QMAP: tempo del passaggio finito, candidati saltati");
            break;
        }
        if state.found.iter().any(|&(f, p, s)| (f - cand.freq_hz).abs() < FOUND_TOL_HZ && p == cand.period_s && s == cand.iseq) {
            continue;
        }
        let averaging = options.average && complete(cand.period_s, cand.iseq);
        let index = parity_index(cand.period_s, cand.iseq);
        let mut any = false;
        for setup in setups_for(cand.period_s) {
            let bw = setup.bandwidth_hz();
            let o = decode_options(setup);
            let found = if averaging {
                let pos = state.averages.iter().position(|s| s.period_s == setup.period_s && s.submode == setup.submode && s.iseq == cand.iseq && s.center_hz == center_hz && (s.freq_hz - cand.freq_hz).abs() <= near);
                let pos = pos.unwrap_or_else(|| {
                    if state.averages.len() >= MAX_AVERAGES {
                        // si butta la media aggiornata meno di recente
                        if let Some(old) = state.averages.iter().enumerate().min_by_key(|(_, s)| s.last).map(|(i, _)| i) {
                            state.averages.remove(old);
                        }
                    }
                    state.averages.push(AvgSlot { period_s: setup.period_s, submode: setup.submode, iseq: cand.iseq, center_hz, freq_hz: cand.freq_hz, averager: Averager::new(), last: index });
                    state.averages.len() - 1
                });
                let slot = &mut state.averages[pos];
                let (kc, audio) = audio_for(slot.freq_hz, setup.period_s, cand.iseq, bw);
                slot.last = index;
                q65::decode_averaged(&audio, &o, hashes, &mut slot.averager, index).into_iter().map(|d| (kc, d)).collect::<Vec<_>>()
            } else {
                let (kc, audio) = audio_for(cand.freq_hz, setup.period_s, cand.iseq, bw);
                q65::decode(&audio, &o, hashes).into_iter().map(|d| (kc, d)).collect()
            };
            for (kc, d) in found {
                any |= push(&mut decodes, d, kc, setup, cand.iseq);
            }
        }
        if any {
            state.found.push((cand.freq_hz, cand.period_s, cand.iseq));
        }
    }

    // le medie dei segnali che in questo periodo non sono diventati candidati (stessa parita', periodo completo)
    if options.average {
        state.averages.retain(|s| parity_index(s.period_s, s.iseq).saturating_sub(s.last) <= AVERAGE_EXPIRY);
        for k in 0..state.averages.len() {
            if pass.deadline.is_some_and(|d| Instant::now() > d) {
                break;
            }
            let (period_s, iseq, submode, freq_hz, last, slot_center) = {
                let s = &state.averages[k];
                (s.period_s, s.iseq, s.submode, s.freq_hz, s.last, s.center_hz)
            };
            let index = parity_index(period_s, iseq);
            let in_minute_kind = !minute || period_s == 60 || (options.also_30 && period_s == 30);
            if !in_minute_kind || !complete(period_s, iseq) || index.checked_sub(last).is_none_or(|d| d == 0 || d % 2 != 0) || slot_center != center_hz {
                continue;
            }
            if (period_s == 30 && iseq == 0 && pass.skip_30a) || (period_s == 30 && iseq == 1 && pass.skip_30b) {
                continue;
            }
            if state.found.iter().any(|&(f, p, s)| (f - freq_hz).abs() < FOUND_TOL_HZ && p == period_s && s == iseq) {
                continue;
            }
            let Some(setup) = Setup::new(period_s, submode) else { continue };
            let (kc, audio) = audio_for(freq_hz, period_s, iseq, setup.bandwidth_hz());
            let o = decode_options(setup);
            let slot = &mut state.averages[k];
            slot.last = index;
            let found = q65::decode_averaged(&audio, &o, hashes, &mut slot.averager, index);
            let mut any = false;
            for d in found {
                any |= push(&mut decodes, d, kc, setup, iseq);
            }
            if any {
                state.found.push((freq_hz, period_s, iseq));
            }
        }
    }
    decodes.sort_by(|a, b| a.offset_hz.total_cmp(&b.offset_hz));

    // spettro per l'interfaccia: media a gruppi, in dB
    let width = savg.len();
    let (spectrum_db, low, high) = if width == 0 || ss.rows.is_empty() {
        (Vec::new(), -span / 2.0, span / 2.0)
    } else {
        let per = width.div_ceil(SPECTRUM_POINTS).max(1);
        let rows = ss.rows.len() as f32;
        (savg.chunks(per).map(|c| 10.0 * (c.iter().sum::<f32>() / (c.len() as f32 * rows)).max(1e-20).log10()).collect(), ss.freq_hz(0), ss.freq_hz(width))
    };
    SearchResult { decodes, candidates: cands.len(), spectrum_db, spectrum_low_hz: low, spectrum_high_hz: high }
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

    /// Aggiunge un segnale Q65 complesso (fase continua) con il tono 0 a `offset_hz` dal centro, `delay_s` dopo l'inizio nominale.
    pub(crate) fn add_q65_at(iq: &mut [Complex<f32>], fs: u32, msg: &str, offset_hz: f64, setup: Setup, amp: f32, delay_s: f64) {
        let tones = q65::tones(msg, &mut CallHashes::default()).expect("messaggio");
        let sps = setup.nsps() * fs as usize / 12_000;
        let at = ((setup.start_s() + delay_s) * f64::from(fs)) as usize;
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

    pub(crate) fn add_q65(iq: &mut [Complex<f32>], fs: u32, msg: &str, offset_hz: f64, setup: Setup, amp: f32) {
        add_q65_at(iq, fs, msg, offset_hz, setup, amp, 0.0);
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
    fn thirty_seconds_in_both_halves_of_the_minute() {
        let fs = 96_000;
        let mut iq = noise(60 * fs as usize, 1.0, 13);
        let amp = (0.1f32 * 2.0 * 2500.0 / fs as f32).sqrt();
        let a30 = Setup::new(30, 0).expect("30A");
        add_q65(&mut iq, fs, "CQ K1ABC FN42", -5_000.0, a30, amp);
        // seconda meta': 30 s dopo, con un ritardo da eco lunare
        add_q65_at(&mut iq, fs, "IU3ABC K1ABC -15", 15_000.0, a30, amp, 32.0);
        let options = SearchOptions { period_s: 60, submodes: vec![1], also_30: true, ..SearchOptions::default() };
        let mut state = SearchState::default();
        let mut hashes = CallHashes::default();
        let first = search_pass(&iq, fs, None, &options, &Pass { nhsym: 200, ..Pass::full(0) }, &mut state, &mut hashes);
        assert!(first.decodes.iter().any(|d| d.text == "CQ K1ABC FN42" && d.mode == "Q65-30A" && d.iseq == 0), "{:?}", first.decodes);
        let last = search_pass(&iq, fs, None, &options, &Pass::full(0), &mut state, &mut hashes);
        assert!(last.decodes.iter().any(|d| d.text == "IU3ABC K1ABC -15" && d.iseq == 1), "{:?}", last.decodes);
        // il primo non si ridecodifica nello stesso minuto
        assert!(!last.decodes.iter().any(|d| d.iseq == 0), "{:?}", last.decodes);
    }

    #[test]
    fn the_average_decodes_what_one_period_cannot() {
        let fs = 48_000;
        let setup = Setup::new(60, 0).expect("60A");
        // abbastanza forte per il sincronismo di un periodo, troppo debole per decodificarlo da solo
        let amp = (0.0025f32 * 2.0 * 2500.0 / fs as f32).sqrt();
        let options = SearchOptions { period_s: 60, average: true, span_hz: 20_000.0, ..SearchOptions::default() };
        let mut state = SearchState::default();
        let mut hashes = CallHashes::default();
        let mut got = None;
        for k in 0..6u64 {
            let mut iq = noise(60 * fs as usize, 1.0, 100 + k);
            add_q65(&mut iq, fs, "CQ IU3ABC IO91", 3_000.0, setup, amp);
            state.new_period();
            let r = search_pass(&iq, fs, None, &options, &Pass::full(2 * k), &mut state, &mut hashes);
            if let Some(d) = r.decodes.into_iter().find(|d| d.text == "CQ IU3ABC IO91") {
                got = Some(d);
                break;
            }
        }
        let d = got.expect("la media non ha decodificato");
        assert!(d.averaged >= 2, "{d:?}");
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
