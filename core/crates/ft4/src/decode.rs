//! La decodifica di un periodo FT4 intero (`ft4_decode` di JTDX 2.2.159, `lib/ft4_decode.f90`, e `subtractft4`): piu' passate, sottrazione dei
//! segnali gia' decodificati, elenco senza doppioni, stima dell'SNR.
//!
//! Ogni passata cerca i candidati ([`sync::find_candidates`]), per ognuno ne cerca ritardo e frequenza in tre finestre di tempo
//! ([`sync::search`]), calcola le tre serie di LLR e prova la propagazione di credenze e poi l'OSD su ciascuna; un messaggio vale solo se il
//! CRC (calcolato sul messaggio mescolato) torna e il testo si legge. I segnali decodificati si tolgono dall'audio con una stima di ampiezza e
//! fase (`subtractft4`) e la passata dopo trova quelli che erano nascosti sotto. Le passate continuano finche' l'ultima ha trovato qualcosa,
//! fino a [`Options::passes`] (3 in JTDX).
//!
//! # Convenzioni
//!
//! - `freq_hz` e' la frequenza del tono 0, a passi di 1 Hz (la correzione fine di JTDX e' intera); `dt_s` e' lo scarto rispetto a 0,5 s con
//!   passo di 1,5 ms; `snr_db` e' la stima di JTDX: dall'altezza del picco nello spettro medio, con il pavimento a -21 dB.
//! - L'elenco e' ordinato per frequenza (JTDX li emette nell'ordine di decodifica).
//!
//! # Differenze rispetto al Fortran
//!
//! - Niente analisi a-priori (AP): servono nominativi (`mycall`, `hiscall`) e lo stato del QSO, che il decoder qui non ha. Restano le tre
//!   passate senza AP (`ipass` 1-3) di ogni candidato.
//! - Niente filtri sui falsi legati ai nominativi (`chkflscall`, `chkfalse8`), ne' nascondimento di messaggi di prova o di telemetria
//!   (`lhidetest`, `lhidetelemetry`), ne' il filtro di frequenza dell'interfaccia (`lfilter`: qui `f_low_hz`/`f_high_hz` alla ricerca). Non c'e'
//!   la media dello scarto di tempo (`avexdt` sempre 0).
//! - Messaggio di tipo 0.1 (DXpedition): JTDX lo spezza in due righe; qui esce com'e' dal decodificatore dei messaggi.
//! - In piu', come in `ft8::decode`, un controllo opzionale sui bit dubbi ([`Options::max_hard_errors`]): un messaggio vero ha pochi bit diversi
//!   dalle decisioni dure del segnale, uno falso (CRC a caso) circa la meta'. In JTDX non c'e'.
//! - L'OSD e' quello di `ft8::ldpc` (stesso algoritmo di `osd4_174_91`, il codice e' identico).

use std::sync::Arc;

use ft8::ldpc;
use ft8::message::{self, CallHashes};
use rustfft::{Fft, FftPlanner};

pub use ft8::decode::Decoded;

use crate::modem;
use crate::sync::{self, Candidate, Complex, Downsampler, SyncTables, DOWN_RATE, NDMAX, NSS};
use crate::{NMAX, NN, NSPS, SAMPLE_RATE};

/// L'audio di JTDX e' a 16 bit (±32768): i campioni normalizzati a ±1 si riportano a quella scala.
const SCALE: f32 = 32768.0;
/// Sincronismo minimo dopo la ricerca fine (`smax >= 1.2` in `ft4_decode`).
const MIN_ALIGN_SYNC: f32 = 1.2;
/// Bit di sincronismo giusti, su 32, sotto i quali il candidato si scarta (`nsync_qual >= 20`).
const MIN_SYNC_BITS: usize = 20;

#[derive(Debug, Clone)]
pub struct Options {
    pub f_low_hz: f32,
    pub f_high_hz: f32,
    /// Frequenza del QSO in corso: i candidati entro 20 Hz si provano per primi.
    pub qso_freq_hz: Option<f32>,
    /// Soglia di altezza dei picchi nello spettro medio (`syncmin`: 1,2).
    pub min_sync: f32,
    /// Numero massimo di candidati per passata (100).
    pub max_candidates: usize,
    /// Passate con sottrazione (JTDX ne fa tre; 1 = una sola, senza sottrazione).
    pub passes: usize,
    /// Toglie dall'audio i segnali decodificati (`dosubtract`).
    pub subtract: bool,
    /// Iterazioni della propagazione di credenze (40).
    pub bp_iterations: usize,
    /// Ordine dell'OSD (0 = nessuno; JTDX usa 3 a profondita' 3).
    pub osd_order: usize,
    /// Finestre di ricerca del ritardo piu' larghe (`swl`, ascolto): da -1,2 a +1,7 s invece che da -1,0 a +1,4 s.
    pub swl: bool,
    /// Scarta i messaggi con piu' di questi bit diversi dalle decisioni dure del segnale (`None` = nessun controllo).
    pub max_hard_errors: Option<usize>,
}

impl Default for Options {
    /// Profondita' 3 di JTDX: tre passate con sottrazione, BP e OSD.
    fn default() -> Self {
        Options::depth(3)
    }
}

impl Options {
    /// Le profondita' di JTDX (`ndepth`): 1 = una passata senza sottrazione ne' OSD; 2 = tre passate solo con BP; 3 = tre passate con BP e OSD.
    pub fn depth(ndepth: u8) -> Self {
        let mut o = Options {
            f_low_hz: 200.0,
            f_high_hz: 4000.0,
            qso_freq_hz: None,
            min_sync: 1.2,
            max_candidates: 100,
            passes: 3,
            subtract: true,
            bp_iterations: 40,
            osd_order: 3,
            swl: false,
            max_hard_errors: Some(MAX_HARD_ERRORS),
        };
        match ndepth {
            0 | 1 => {
                o.passes = 1;
                o.subtract = false;
                o.osd_order = 0;
            }
            2 => o.osd_order = 0,
            _ => {}
        }
        o
    }
}

/// Un messaggio con piu' bit diversi da quelli decisi dal segnale e' un falso: un vero ne ha pochi, uno casuale circa la meta' (87).
const MAX_HARD_ERRORS: usize = 45;

/// Decodifica i 7,5 secondi di audio di un periodo (12 kHz, campioni normalizzati a ±1; si usano i primi 73728 campioni).
pub fn decode_slot(samples: &[f32], options: &Options, hashes: &mut CallHashes) -> Vec<Decoded> {
    let mut audio: Vec<f32> = samples.iter().take(NMAX).map(|&x| x * SCALE).collect();
    audio.resize(NMAX, 0.0);
    let tables = SyncTables::new();
    let subtractor = Subtractor::new();
    let mut found: Vec<Decoded> = Vec::new();
    let mut before_pass = 0;

    for pass in 0..options.passes.max(1) {
        // dalla seconda passata in poi si prosegue solo se la precedente ha trovato qualcosa
        if pass > 0 && found.len() == before_pass {
            break;
        }
        before_pass = found.len();

        let candidates = sync::find_candidates(&audio, options.f_low_hz, options.f_high_hz, options.min_sync, options.max_candidates, options.qso_freq_hz);
        if candidates.is_empty() {
            break;
        }
        let ds = Downsampler::new(&audio);
        for cand in candidates {
            let Some(hit) = try_candidate(&ds, &tables, cand, options, hashes) else { continue };
            // come nel Fortran la sottrazione precede il controllo dei doppioni
            if options.subtract {
                subtractor.subtract(&mut audio, &hit.tones, hit.decoded.freq_hz, hit.start);
            }
            if found.iter().any(|d| d.text == hit.decoded.text) {
                continue;
            }
            found.push(hit.decoded);
        }
    }
    found.sort_by(|a, b| a.freq_hz.total_cmp(&b.freq_hz));
    found
}

struct Hit {
    decoded: Decoded,
    tones: [u8; NN],
    /// Campione (a 666,67 Hz) del primo simbolo di Costas.
    start: i64,
}

/// Cerca ritardo e frequenza di un candidato nelle tre finestre di tempo e prova a decodificarlo.
fn try_candidate(ds: &Downsampler, tables: &SyncTables, cand: Candidate, options: &Options, hashes: &mut CallHashes) -> Option<Hit> {
    let f0 = cand.freq_hz;
    let mut cd2 = ds.run(f0);
    sync::normalize(&mut cd2, NDMAX as f32);

    let mut smax1 = 0.0f32;
    for (iseg, &(ibmin, ibmax)) in sync::dt_windows(options.swl).iter().enumerate() {
        let al = sync::search(&cd2, tables, ibmin, ibmax);
        if iseg == 0 {
            smax1 = al.sync;
        }
        if al.sync < MIN_ALIGN_SYNC {
            continue;
        }
        if iseg > 0 && al.sync < smax1 {
            continue;
        }
        let f1 = f0 + al.df_hz as f32;
        if f1 <= 10.0 || f1 >= 4990.0 {
            continue;
        }
        let mut cb = ds.run(f1);
        sync::normalize(&mut cb, (NSS * NN) as f32);
        let cd = sync::frame(&cb, al.start);
        let Some(metrics) = sync::bit_metrics(&cd) else { continue };
        if metrics.sync_bits < MIN_SYNC_BITS {
            continue;
        }
        if let Some((text, codeword)) = decode_llrs(&metrics, options, hashes) {
            let snr = sync_to_snr(cand.sync);
            let dt = al.start as f32 / DOWN_RATE - 0.5;
            let tones = modem::tones_from_codeword(&codeword);
            return Some(Hit { decoded: Decoded { freq_hz: f1, dt_s: dt, snr_db: snr, text }, tones, start: al.start });
        }
    }
    None
}

/// Le tre serie di LLR, nell'ordine: per ognuna la propagazione di credenze e, se non basta, l'OSD. Il primo messaggio valido vince.
fn decode_llrs(metrics: &sync::Metrics, options: &Options, hashes: &mut CallHashes) -> Option<(String, [u8; 174])> {
    for llr in &metrics.llr {
        let mut code = ldpc::bp_decode(llr, options.bp_iterations);
        if code.is_none() && options.osd_order > 0 {
            code = ldpc::osd_decode(llr, options.osd_order);
        }
        let Some(code) = code else { continue };
        if code.message[..77].iter().all(|&b| b == 0) {
            continue;
        }
        if let Some(limit) = options.max_hard_errors {
            let hard_errors = code.codeword.iter().zip(llr.iter()).filter(|&(&bit, &l)| (l > 0.0) != (bit == 1)).count();
            if hard_errors > limit {
                continue;
            }
        }
        let mut scrambled = [0u8; 77];
        scrambled.copy_from_slice(&code.message[..77]);
        let bits = modem::scramble(&scrambled);
        let Ok(text) = message::unpack(&bits, hashes) else { continue };
        let free_text = message::message_type(&bits) == (0, 0);
        let text = if free_text { text } else { strip_braces(&text) };
        if text.trim().is_empty() {
            continue;
        }
        return Some((text, code.codeword));
    }
    None
}

/// Toglie le parentesi angolari al primo dei primi tre nominativi che le ha (`<IU3ABC>` diventa `IU3ABC`); `<...>` (hash sconosciuto) resta.
fn strip_braces(text: &str) -> String {
    if !text.contains('<') {
        return text.to_string();
    }
    let mut parts: Vec<String> = text.split(' ').map(str::to_string).collect();
    for p in parts.iter_mut().take(3) {
        if p.starts_with('<') && !p.starts_with("<.") {
            let inner = p.trim_start_matches('<').trim_end_matches('>');
            *p = inner.to_string();
            break;
        }
    }
    parts.join(" ")
}

/// L'SNR di JTDX: dall'altezza del picco (`snr = speak - 1`): `10 log10(snr) - 14,8` dB, con il pavimento a -21.
fn sync_to_snr(sync: f32) -> i32 {
    let snr = sync - 1.0;
    let xsnr = if snr > 0.0 { 10.0 * snr.log10() - 14.8 } else { -21.0 };
    xsnr.max(-21.0).round() as i32
}

// ---------------------------------------------------------------------------------------------------------------------
// subtractft4

/// Campioni del filtro passa-basso della sottrazione.
const NFILT: usize = 1400;

/// Toglie dall'audio un segnale decodificato (`subtractft4`): lo rigenera, ne stima ampiezza e fase complesse filtrando `audio * conj(riferimento)`
/// con una finestra coseno quadro di 1401 punti, e sottrae `2 Re(ampiezza * riferimento)`.
struct Subtractor {
    forward: Arc<dyn Fft<f32>>,
    inverse: Arc<dyn Fft<f32>>,
    /// FFT del filtro, gia' divisa per la lunghezza.
    kernel: Vec<Complex>,
}

impl Subtractor {
    fn new() -> Self {
        let mut planner = FftPlanner::<f32>::new();
        let forward = planner.plan_fft_forward(NMAX);
        let inverse = planner.plan_fft_inverse(NMAX);
        // finestra cos^2(pi j / NFILT), j = -700..700, normalizzata a somma 1; il Fortran la ruota con cshift(NFILT/2 + 1): il centro finisce
        // sull'ultimo campione, cioe' il filtro guarda un campione avanti (si conserva)
        let half = (NFILT / 2) as i64;
        let weights: Vec<f64> = (-half..=half).map(|j| (std::f64::consts::PI * j as f64 / NFILT as f64).cos().powi(2)).collect();
        let sum: f64 = weights.iter().sum();
        let mut kernel = vec![Complex::new(0.0, 0.0); NMAX];
        for (idx, w) in weights.iter().enumerate() {
            let j = idx as i64 - half; // da -700 a 700
            let pos = (j - 1).rem_euclid(NMAX as i64) as usize;
            kernel[pos].re = (w / sum) as f32;
        }
        let mut scratch = vec![Complex::new(0.0, 0.0); forward.get_inplace_scratch_len()];
        forward.process_with_scratch(&mut kernel, &mut scratch);
        for k in kernel.iter_mut() {
            *k /= NMAX as f32;
        }
        Subtractor { forward, inverse, kernel }
    }

    /// `start` e' il campione (a 666,67 Hz) del primo simbolo di Costas: il riferimento parte un simbolo di rampa prima, come in
    /// `nstart = dt * 12000 + 1 - NSPS` con `dt = start / 666,67` (anche la stessa aritmetica in singola precisione).
    fn subtract(&self, audio: &mut [f32], tones: &[u8; NN], f0_hz: f32, start: i64) {
        let dt = start as f32 / DOWN_RATE;
        let nstart = (dt * SAMPLE_RATE as f32 + 1.0 - NSPS as f32) as i64 - 1; // da 0
        let cref = modem::gfsk_complex(tones, f0_hz);
        let zero = Complex::new(0.0, 0.0);
        let mut buf = vec![zero; NMAX];
        for (i, (b, r)) in buf.iter_mut().zip(&cref).enumerate() {
            let id = nstart + i as i64;
            if id >= 0 && (id as usize) < audio.len() {
                *b = r.conj() * audio[id as usize];
            }
        }
        let mut scratch = vec![zero; self.forward.get_inplace_scratch_len().max(self.inverse.get_inplace_scratch_len())];
        self.forward.process_with_scratch(&mut buf, &mut scratch);
        for (b, k) in buf.iter_mut().zip(&self.kernel) {
            *b *= k;
        }
        self.inverse.process_with_scratch(&mut buf, &mut scratch);
        for (i, (b, r)) in buf.iter().zip(&cref).enumerate() {
            let id = nstart + i as i64;
            if id >= 0 && (id as usize) < audio.len() {
                audio[id as usize] -= 2.0 * (*b * *r).re;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::SLOT_SAMPLES;
    use crate::testutil::slot;

    fn decode(audio: &[f32]) -> Vec<Decoded> {
        decode_slot(audio, &Options::default(), &mut CallHashes::default())
    }

    fn texts(out: &[Decoded]) -> Vec<String> {
        out.iter().map(|d| d.text.clone()).collect()
    }

    #[test]
    fn un_messaggio_pulito_con_frequenza_tempo_e_snr() {
        let audio = slot(&[("CQ K1ABC FN42", 1500.0, 0.3, -5.0)], 1, 0.05);
        let out = decode(&audio);
        assert_eq!(out.len(), 1, "{out:?}");
        let d = &out[0];
        assert_eq!(d.text, "CQ K1ABC FN42");
        assert!((d.freq_hz - 1500.0).abs() <= 2.0, "{}", d.freq_hz);
        assert!((d.dt_s - 0.3).abs() < 0.02, "{}", d.dt_s);
        assert!((-21..=5).contains(&d.snr_db), "snr {}", d.snr_db);
    }

    #[test]
    fn i_tre_messaggi_di_prova_a_snr_medio() {
        for (seed, snr) in [(21u64, -8.0f32), (22, -11.0)] {
            for text in ["CQ K1ABC FN42", "IU3ABC K1ABC -15", "K1ABC IU3ABC RR73"] {
                let audio = slot(&[(text, 1234.0, 0.1, snr)], seed, 0.05);
                let out = decode(&audio);
                assert!(texts(&out).iter().any(|t| t == text), "{text} a {snr} dB (seme {seed}): {out:?}");
                assert_eq!(out.len(), 1, "nessun falso: {out:?}");
            }
        }
    }

    #[test]
    fn piu_segnali_nello_stesso_periodo() {
        let audio = slot(&[("CQ K1ABC FN42", 800.0, 0.1, -6.0), ("IU3ABC K1ABC -15", 1500.0, 0.3, -9.0), ("K1ABC IU3ABC RR73", 2200.0, -0.2, -4.0)], 7, 0.05);
        let out = decode(&audio);
        let t = texts(&out);
        for expected in ["CQ K1ABC FN42", "IU3ABC K1ABC -15", "K1ABC IU3ABC RR73"] {
            assert!(t.iter().any(|x| x == expected), "manca {expected}: {out:?}");
        }
        assert_eq!(out.len(), 3, "nessun falso: {out:?}");
        for d in &out {
            let (f, dt) = match d.text.as_str() {
                "CQ K1ABC FN42" => (800.0, 0.1),
                "IU3ABC K1ABC -15" => (1500.0, 0.3),
                _ => (2200.0, -0.2),
            };
            assert!((d.freq_hz - f).abs() <= 2.0 && (d.dt_s - dt).abs() < 0.02, "{d:?}");
        }
    }

    #[test]
    fn un_debole_sotto_un_forte_compare_dopo_la_sottrazione() {
        // 25 Hz di distanza: il debole e' dentro la banda del forte (4 toni = 62 Hz) finche' questo non viene tolto
        let audio = slot(&[("CQ DX IU3ABC IO91", 1500.0, 0.2, 8.0), ("K1ABC W9XYZ -12", 1525.0, 0.35, -9.0)], 11, 0.05);
        let out = decode(&audio);
        let t = texts(&out);
        assert!(t.iter().any(|x| x == "CQ DX IU3ABC IO91"), "{out:?}");
        assert!(t.iter().any(|x| x == "K1ABC W9XYZ -12"), "{out:?}");
        // senza sottrazione il debole si perde (la prova controlla che la passata serva davvero)
        let single = decode_slot(&audio, &Options::depth(1), &mut CallHashes::default());
        assert!(!texts(&single).iter().any(|x| x == "K1ABC W9XYZ -12"), "senza sottrazione: {single:?}");
    }

    #[test]
    fn solo_rumore_non_decodifica_nulla() {
        for seed in [3u64, 4, 5, 6, 8, 9, 10, 12] {
            let out = decode(&slot(&[], seed, 0.05));
            assert!(out.is_empty(), "seme {seed}: {out:?}");
        }
    }

    #[test]
    fn rumore_senza_il_controllo_sui_bit_dubbi_non_da_falsi() {
        // il controllo sui bit dubbi e' una cintura in piu': i filtri di JTDX da soli devono bastare sul rumore bianco
        let options = Options { max_hard_errors: None, ..Options::default() };
        for seed in [13u64, 14, 15, 16] {
            let out = decode_slot(&slot(&[], seed, 0.05), &options, &mut CallHashes::default());
            assert!(out.is_empty(), "seme {seed}: {out:?}");
        }
    }

    #[test]
    fn buffer_corti_o_vuoti_non_vanno_in_panico() {
        assert!(decode(&[]).is_empty());
        assert!(decode(&vec![0.0; 1000]).is_empty());
        assert!(decode(&vec![0.0; SLOT_SAMPLES]).is_empty());
        assert!(decode(&vec![f32::NAN; SLOT_SAMPLES]).is_empty());
        assert!(decode(&vec![1.0; SLOT_SAMPLES]).is_empty());
    }

    #[test]
    fn ritardi_ai_bordi_delle_finestre() {
        // da -0,2 s a +0,6 s: copre le tre finestre di ricerca del ritardo
        for (i, dt) in [-0.25f32, 0.0, 0.55, 0.8].into_iter().enumerate() {
            let audio = slot(&[("CQ K1ABC FN42", 1000.0, dt, -3.0)], 30 + i as u64, 0.05);
            let out = decode(&audio);
            assert!(texts(&out).iter().any(|t| t == "CQ K1ABC FN42"), "dt {dt}: {out:?}");
            if let Some(d) = out.first() {
                assert!((d.dt_s - dt).abs() < 0.02, "dt {dt}: {d:?}");
            }
        }
    }

    #[test]
    fn le_parentesi_angolari_dei_nominativi_si_tolgono() {
        assert_eq!(strip_braces("CQ DX K1ABC"), "CQ DX K1ABC");
        assert_eq!(strip_braces("<IU3ABC> K1ABC -15"), "IU3ABC K1ABC -15");
        assert_eq!(strip_braces("K1ABC <IU3ABC> RR73"), "K1ABC IU3ABC RR73");
        assert_eq!(strip_braces("<...> K1ABC -15"), "<...> K1ABC -15");
    }

    #[test]
    fn snr_di_jtdx() {
        assert_eq!(sync_to_snr(0.5), -21);
        assert_eq!(sync_to_snr(1.0), -21);
        assert_eq!(sync_to_snr(1.1), -21);
        assert!(sync_to_snr(100.0) > sync_to_snr(10.0));
    }

    #[test]
    fn la_sottrazione_toglie_il_segnale() {
        // 40 dB sopra il rumore: l'energia che resta dopo la sottrazione e' quasi tutta rumore
        let audio = slot(&[("CQ K1ABC FN42", 1500.0, 0.2, 40.0)], 40, 0.001);
        let mut scaled: Vec<f32> = audio.iter().map(|&x| x * SCALE).collect();
        scaled.resize(NMAX, 0.0);
        let before: f64 = scaled.iter().map(|&x| f64::from(x) * f64::from(x)).sum();
        let tones = modem::tones_from_bits77(&message::pack("CQ K1ABC FN42", &mut CallHashes::default()).expect("messaggio valido"));
        let start = ((0.5f32 + 0.2) * DOWN_RATE).round() as i64;
        Subtractor::new().subtract(&mut scaled, &tones, 1500.0, start);
        let after: f64 = scaled.iter().map(|&x| f64::from(x) * f64::from(x)).sum();
        assert!(after < before * 0.02, "energia residua {} su {}", after, before);
    }

    #[test]
    #[ignore = "misura dei tempi: cargo test -p ft4 --release -- --ignored --nocapture"]
    fn tempi_di_decodifica() {
        let signals: Vec<(String, f32, f32, f32)> = (0..12)
            .map(|i| {
                let call = format!("K{}ABC", (b'A' + (i % 26) as u8) as char);
                (format!("CQ {call} FN42"), 400.0 + 280.0 * i as f32, 0.1 + 0.03 * i as f32, -6.0 - (i % 4) as f32 * 2.0)
            })
            .collect();
        let refs: Vec<(&str, f32, f32, f32)> = signals.iter().map(|(t, f, d, s)| (t.as_str(), *f, *d, *s)).collect();
        let busy = slot(&refs, 77, 0.05);
        let noise = slot(&[], 78, 0.05);
        for (name, audio) in [("rumore", &noise), ("12 segnali", &busy)] {
            for options in [Options::depth(1), Options::depth(2), Options::depth(3)] {
                let t0 = std::time::Instant::now();
                let out = decode_slot(audio, &options, &mut CallHashes::default());
                println!("{name:>10} passate {} osd {}: {} messaggi in {:?}", options.passes, options.osd_order, out.len(), t0.elapsed());
            }
        }
    }

    #[test]
    fn sensibilita_a_meno_14_db_e_snr_riportato() {
        let mut ok = 0;
        for seed in 100..108u64 {
            let audio = slot(&[("CQ K1ABC FN42", 1500.0, 0.2, -14.0)], seed, 0.05);
            let out = decode(&audio);
            if let Some(d) = out.iter().find(|d| d.text == "CQ K1ABC FN42") {
                ok += 1;
                assert!((d.snr_db - (-14)).abs() <= 3, "snr {}", d.snr_db);
            }
        }
        assert!(ok >= 7, "decodificati {ok} su 8");
    }

    #[test]
    #[ignore = "misura: cargo test -p ft4 --release -- --ignored --nocapture sensibilita_curva"]
    fn sensibilita_curva() {
        let senza = Options { max_hard_errors: None, ..Options::default() };
        for snr in [-14.0f32, -16.0, -17.0, -18.0, -19.0] {
            let (mut ok, mut ok_senza) = (0, 0);
            for seed in 100..120u64 {
                let audio = slot(&[("CQ K1ABC FN42", 1500.0, 0.2, snr)], seed, 0.05);
                if decode(&audio).iter().any(|d| d.text == "CQ K1ABC FN42") {
                    ok += 1;
                }
                if decode_slot(&audio, &senza, &mut CallHashes::default()).iter().any(|d| d.text == "CQ K1ABC FN42") {
                    ok_senza += 1;
                }
            }
            println!("SNR {snr}: {ok}/20 con il controllo sui bit dubbi, {ok_senza}/20 senza");
        }
    }

    #[test]
    #[ignore = "misura: cargo test -p ft4 --release -- --ignored --nocapture falsi_su_molti"]
    fn falsi_su_molti_periodi_di_rumore() {
        let senza = Options { max_hard_errors: None, ..Options::default() };
        let (mut con_controllo, mut senza_controllo) = (0, 0);
        for seed in 1000..1200u64 {
            let audio = slot(&[], seed, 0.05);
            con_controllo += decode(&audio).len();
            senza_controllo += decode_slot(&audio, &senza, &mut CallHashes::default()).len();
        }
        println!("falsi su 200 periodi di solo rumore: {con_controllo} con il controllo, {senza_controllo} senza");
    }
}
