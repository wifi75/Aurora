//! La decodifica di un periodo intero (`ft8_decode` e la parte di controllo di `ft8b`): piu' passate, sottrazione dei segnali gia'
//! decodificati, elenco senza doppioni, stima dell'SNR.
//!
//! Ogni passata cerca i candidati, li allinea, ne calcola le probabilita' dei bit (tre serie di metriche) e prova la propagazione di
//! credenze e poi l'OSD; un messaggio vale solo se il CRC torna e il testo si legge (`message::unpack`). I segnali decodificati si tolgono
//! dall'audio con una stima di ampiezza e fase, e la passata dopo trova quelli che erano nascosti sotto.
//!
//! Rispetto a JTDX mancano per ora i livelli di sensibilita' multipli, l'analisi a-priori (AP), la sottrazione con ampiezza variabile e
//! i filtri sui falsi legati ai nominativi: il numero di messaggi trovati si misura contro JTDX (vedi `docs/2026-10-24-aurora-motore-ft8.md`).

use std::f64::consts::PI;

use crate::{
    ldpc, message, modem,
    message::CallHashes,
    sync::{self, Candidate, Complex, Downsampler},
    SAMPLE_RATE, SLOT_SAMPLES,
};

/// Un messaggio decodificato.
#[derive(Debug, Clone, PartialEq)]
pub struct Decoded {
    /// Frequenza del tono 0 (Hz): quella che JTDX scrive nella colonna delle frequenze.
    pub freq_hz: f32,
    /// Scarto di tempo come lo scrive JTDX (secondi).
    pub dt_s: f32,
    /// Rapporto segnale/rumore in 2500 Hz (dB, intero come nell'elenco dei messaggi).
    pub snr_db: i32,
    pub text: String,
}

#[derive(Debug, Clone)]
pub struct Options {
    pub f_low_hz: f32,
    pub f_high_hz: f32,
    /// Soglia di sincronismo dei candidati (1,5 normale; 1,1-1,225 per cercare i segnali deboli).
    pub min_sync: f32,
    /// Numero massimo di candidati per passata.
    pub max_candidates: usize,
    /// Passate con sottrazione (JTDX ne fa tre).
    pub passes: usize,
    /// Iterazioni della propagazione di credenze.
    pub bp_iterations: usize,
    /// Ordine dell'OSD (0 = nessuno).
    pub osd_order: usize,
}

impl Default for Options {
    fn default() -> Self {
        Options { f_low_hz: 200.0, f_high_hz: 4000.0, min_sync: 1.5, max_candidates: 300, passes: 3, bp_iterations: 30, osd_order: 2 }
    }
}

/// L'audio di JTDX e' a 16 bit (±32768): i campioni normalizzati a ±1 si riportano a quella scala, su cui sono tarate le soglie.
const SCALE: f32 = 32768.0;
/// Un messaggio con piu' bit diversi da quelli decisi dal segnale e' un falso: un vero ne ha pochi, uno casuale circa la meta' (87).
const MAX_HARD_ERRORS: usize = 45;
/// Due decodifiche a meno di questa distanza in frequenza e tempo sono lo stesso segnale.
const SAME_FREQ_HZ: f32 = 3.0;
const SAME_DT_S: f32 = 0.1;

/// Decodifica i 15 secondi di audio di un periodo (12 kHz, campioni normalizzati a ±1).
pub fn decode_slot(samples: &[f32], options: &Options, hashes: &mut CallHashes) -> Vec<Decoded> {
    let mut audio: Vec<f32> = samples.iter().take(SLOT_SAMPLES).map(|&x| x * SCALE).collect();
    audio.resize(SLOT_SAMPLES, 0.0);
    let mut found: Vec<Decoded> = Vec::new();

    for _ in 0..options.passes.max(1) {
        let candidates = sync::find_candidates(&audio, options.f_low_hz, options.f_high_hz, options.min_sync, options.max_candidates);
        if candidates.is_empty() {
            break;
        }
        let ds = Downsampler::new(&audio);
        let mut fresh: Vec<(Decoded, [u8; 79])> = Vec::new();
        for cand in candidates {
            if found.iter().chain(fresh.iter().map(|(d, _)| d)).any(|d| (d.freq_hz - cand.freq_hz).abs() < SAME_FREQ_HZ && (d.dt_s - cand.dt_s).abs() < SAME_DT_S) {
                continue;
            }
            if let Some((decoded, tones)) = try_candidate(&ds, cand, options, hashes) {
                if found.iter().chain(fresh.iter().map(|(d, _)| d)).any(|d| d.text == decoded.text) {
                    continue;
                }
                fresh.push((decoded, tones));
            }
        }
        if fresh.is_empty() {
            break;
        }
        for (decoded, tones) in &fresh {
            subtract(&mut audio, tones, decoded.freq_hz, decoded.dt_s);
        }
        found.extend(fresh.into_iter().map(|(d, _)| d));
    }
    found.sort_by(|a, b| a.freq_hz.total_cmp(&b.freq_hz));
    found
}

/// Allinea un candidato, ne calcola le probabilita' dei bit e prova a decodificarlo.
fn try_candidate(ds: &Downsampler, cand: Candidate, options: &Options, hashes: &mut CallHashes) -> Option<(Decoded, [u8; 79])> {
    let coarse = ds.run(cand.freq_hz, false);
    let refined = sync::refine(&coarse, cand);
    // le metriche vogliono il segnale con il tono 0 a 0 Hz: si rifa la discesa alla frequenza corretta
    let cd = if (refined.freq_hz - cand.freq_hz).abs() > 0.01 { ds.run(refined.freq_hz, false) } else { coarse };
    let metrics = sync::bit_metrics(&cd, refined)?;

    let mut result: Option<(ldpc::Decoded, [f32; 174])> = None;
    for llr in &metrics {
        if let Some(d) = ldpc::bp_decode(llr, options.bp_iterations) {
            result = Some((d, *llr));
            break;
        }
    }
    if result.is_none() && options.osd_order > 0 {
        for llr in &metrics {
            if let Some(d) = ldpc::osd_decode(llr, options.osd_order) {
                result = Some((d, *llr));
                break;
            }
        }
    }
    let (code, llr) = result?;

    let hard_errors = code.codeword.iter().zip(llr.iter()).filter(|&(&bit, &l)| (l > 0.0) != (bit == 1)).count();
    if hard_errors > MAX_HARD_ERRORS {
        return None;
    }
    let mut bits77 = [0u8; 77];
    bits77.copy_from_slice(&code.message[..77]);
    let text = message::unpack(&bits77, hashes).ok()?;
    let tones = modem::tones_from_codeword(&code.codeword);
    let snr = estimate_snr(&cd, &tones, refined.dt_s);
    Some((Decoded { freq_hz: refined.freq_hz, dt_s: refined.dt_s, snr_db: snr, text }, tones))
}

/// SNR in 2500 Hz dai 79 simboli del segnale a 200 Hz: l'energia nel tono giusto contro la media degli altri sette (solo rumore).
fn estimate_snr(cd: &[Complex], tones: &[u8; 79], dt_s: f32) -> i32 {
    let start = ((f64::from(dt_s) + 0.5) * 200.0).round() as i64;
    let (mut signal, mut noise) = (0.0f64, 0.0f64);
    for (k, &tone) in tones.iter().enumerate() {
        let from = start + (k as i64) * 32;
        if from < 0 || (from + 32) as usize > cd.len() {
            continue;
        }
        let window = &cd[from as usize..from as usize + 32];
        let mut power = [0.0f64; 8];
        for (m, p) in power.iter_mut().enumerate() {
            let (mut re, mut im) = (0.0f64, 0.0f64);
            for (n, x) in window.iter().enumerate() {
                let phase = -2.0 * PI * (m * n) as f64 / 32.0;
                let (s, c) = phase.sin_cos();
                re += f64::from(x.re) * c - f64::from(x.im) * s;
                im += f64::from(x.re) * s + f64::from(x.im) * c;
            }
            *p = re * re + im * im;
        }
        let right = power[usize::from(tone)];
        signal += right;
        noise += (power.iter().sum::<f64>() - right) / 7.0;
    }
    if noise <= 0.0 {
        return 0;
    }
    // il bin del tono giusto contiene segnale e rumore: si toglie il rumore; 2500 Hz / 6,25 Hz = 400 (26 dB)
    let ratio = (signal / noise - 1.0).max(1e-3);
    (10.0 * ratio.log10() - 26.02).round().clamp(-30.0, 30.0) as i32
}

/// Toglie dall'audio il segnale decodificato: lo rigenera, ne stima ampiezza e fase con i due riferimenti (seno e coseno) e lo sottrae.
fn subtract(audio: &mut [f32], tones: &[u8; 79], freq_hz: f32, dt_s: f32) {
    let (sin_ref, cos_ref) = modem::gfsk_iq(tones, freq_hz, modem::GFSK_BT);
    let start = ((f64::from(dt_s) + 0.5) * SAMPLE_RATE as f64).round() as i64;
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

    /// Generatore pseudocasuale deterministico (nessuna dipendenza): uniforme in (0,1) e gaussiana (Box-Muller).
    struct Rng(u64);
    impl Rng {
        fn uniform(&mut self) -> f64 {
            self.0 = self.0.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
            ((self.0 >> 11) as f64 + 0.5) / (1u64 << 53) as f64
        }
        fn gauss(&mut self) -> f64 {
            (-2.0 * self.uniform().ln()).sqrt() * (2.0 * PI * self.uniform()).cos()
        }
    }

    /// Un periodo con rumore bianco e i messaggi dati (testo, frequenza, dt, SNR in 2500 Hz).
    fn slot(signals: &[(&str, f32, f32, f32)], seed: u64) -> Vec<f32> {
        let mut rng = Rng(seed);
        let sigma = 0.05f64; // ampiezza del rumore (normalizzata)
        let mut audio: Vec<f32> = (0..SLOT_SAMPLES).map(|_| (sigma * rng.gauss()) as f32).collect();
        let mut hashes = CallHashes::default();
        for &(text, freq, dt, snr) in signals {
            let wave = modem::encode_message(text, freq, &mut hashes).expect("messaggio valido");
            // potenza del rumore in 2500 Hz = sigma^2 * 2500 / 6000; l'onda ha picco a, potenza a^2 / 2
            let noise_2500 = sigma * sigma * 2500.0 / 6000.0;
            let amplitude = (2.0 * noise_2500 * 10f64.powf(f64::from(snr) / 10.0)).sqrt() as f32;
            let start = ((f64::from(dt) + 0.5) * SAMPLE_RATE as f64).round() as usize;
            for (i, &w) in wave.iter().enumerate() {
                if let Some(a) = audio.get_mut(start + i) {
                    *a += amplitude * w;
                }
            }
        }
        audio
    }

    fn decode(audio: &[f32]) -> Vec<Decoded> {
        decode_slot(audio, &Options::default(), &mut CallHashes::default())
    }

    #[test]
    fn a_clean_message_is_found_with_frequency_time_and_snr() {
        let audio = slot(&[("CQ K1ABC FN42", 1500.0, 0.3, -8.0)], 1);
        let out = decode(&audio);
        assert_eq!(out.len(), 1, "{out:?}");
        let d = &out[0];
        assert_eq!(d.text, "CQ K1ABC FN42");
        assert!((d.freq_hz - 1500.0).abs() < 1.0, "{}", d.freq_hz);
        assert!((d.dt_s - 0.3).abs() < 0.05, "{}", d.dt_s);
        assert!((d.snr_db - (-8)).abs() <= 3, "snr {}", d.snr_db);
    }

    #[test]
    fn several_signals_are_all_found() {
        let audio = slot(&[("CQ K1ABC FN42", 800.0, 0.1, -6.0), ("IU3ABC K1ABC -15", 1500.0, 0.3, -9.0), ("K1ABC IU3ABC RR73", 2200.0, -0.2, -4.0)], 7);
        let texts: Vec<String> = decode(&audio).into_iter().map(|d| d.text).collect();
        for expected in ["CQ K1ABC FN42", "IU3ABC K1ABC -15", "K1ABC IU3ABC RR73"] {
            assert!(texts.iter().any(|t| t == expected), "manca {expected}: {texts:?}");
        }
        assert_eq!(texts.len(), 3, "nessun falso: {texts:?}");
    }

    #[test]
    fn a_weak_signal_under_a_strong_one_appears_after_subtraction() {
        // i due toni sono a 12 Hz di distanza: il debole e' coperto dai fianchi del forte finche' questo non viene tolto
        let audio = slot(&[("CQ DX IU3ABC IO91", 1500.0, 0.2, 5.0), ("K1ABC W9XYZ -12", 1512.0, 0.4, -9.0)], 11);
        let texts: Vec<String> = decode(&audio).into_iter().map(|d| d.text).collect();
        assert!(texts.iter().any(|t| t == "CQ DX IU3ABC IO91"), "{texts:?}");
        assert!(texts.iter().any(|t| t == "K1ABC W9XYZ -12"), "{texts:?}");
    }

    #[test]
    fn noise_alone_decodes_nothing() {
        for seed in [3, 4, 5] {
            let out = decode(&slot(&[], seed));
            assert!(out.is_empty(), "seme {seed}: {out:?}");
        }
    }

    #[test]
    fn a_short_or_empty_buffer_does_not_panic() {
        assert!(decode(&[]).is_empty());
        assert!(decode(&vec![0.0; 1000]).is_empty());
        assert!(decode(&vec![0.0; SLOT_SAMPLES]).is_empty());
    }
}
