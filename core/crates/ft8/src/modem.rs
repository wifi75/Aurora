//! Modulazione FT8: dai 174 bit ai 79 toni e all'onda GFSK; in ricezione, dal segnale allineato alle probabilita' dei bit.
//!
//! Trasmissione, portata da JTDX 2.2.159 (`lib/genft8.f90`, `lib/gen_ft8wave.f90`, `lib/gfsk_pulse.f90`):
//!
//! - [`tones_from_codeword`]: struttura S7 D29 S7 D29 S7 (sincronismo di Costas, 29 simboli di dati, Costas, 29 di dati, Costas),
//!   con i dati presi a gruppi di 3 bit e codificati in Gray (`graymap`);
//! - [`gfsk_wave`]: l'onda a 12 kHz con impulso gaussiano BT = 2.0, 1920 campioni per simbolo.
//!
//! Lunghezza dell'onda: esattamente `NN * NSPS` = 151680 campioni (12,64 s), nessun riempimento: e' quanto produce
//! `gen_ft8wave(..., nwave = 151680)` in JTDX. Il silenzio fino ai 15 s del periodo (e il ritardo iniziale di 0,5 s)
//! li gestisce chi riproduce l'audio.
//!
//! Ampiezza: come nel sorgente l'onda e' `sin(fase)`, quindi a picco 1.0 (nessuna normalizzazione ulteriore); la rampa
//! di 240 campioni (20 ms) a inizio e fine puo' solo ridurla. Il livello finale lo decide lo stadio audio a valle.

use std::f64::consts::PI;

use crate::message::{Bits77, CallHashes, MessageError};
use crate::{COSTAS, NN, NSPS, SAMPLE_RATE, TONE_SPACING_HZ, crc, ldpc, message};

/// Codifica Gray dei gruppi di 3 bit (`graymap` di `ft8_mod1`): indice = bit0*4 + bit1*2 + bit2.
pub const GRAYMAP: [u8; 8] = [0, 1, 3, 2, 5, 6, 4, 7];

/// Prodotto banda-tempo dell'impulso gaussiano di FT8.
pub const GFSK_BT: f32 = 2.0;

/// Campioni della rampa di salita e di discesa: `nint(NSPS / 8)` = 240 (20 ms).
pub const RAMP_SAMPLES: usize = NSPS / 8;

/// Campioni dell'onda completa: 79 simboli da 1920.
pub const WAVE_SAMPLES: usize = NN * NSPS;

/// Dai 174 bit ai 79 simboli (0-7): sincronismo di Costas all'inizio, a meta' e in fondo, codifica Gray dei dati.
///
/// I simboli 0-6, 36-42 e 72-78 sono la matrice di Costas; i 58 di dati occupano 7-35 e 43-71. Ogni elemento di
/// `codeword` conta solo per il suo bit meno significativo (valgono 0 e 1).
pub fn tones_from_codeword(codeword: &[u8; 174]) -> [u8; 79] {
    let mut tones = [0u8; 79];
    for start in [0usize, 36, 72] {
        tones[start..start + 7].copy_from_slice(&COSTAS);
    }
    let (groups, _) = codeword.as_chunks::<3>();
    for (j, bits) in groups.iter().enumerate() {
        // j = 0..57: i primi 29 simboli di dati partono dal 7, gli altri 29 dal 43 (dopo il secondo Costas).
        let k = if j < 29 { 7 + j } else { 14 + j };
        let index = ((bits[0] & 1) << 2) | ((bits[1] & 1) << 1) | (bits[2] & 1);
        tones[k] = GRAYMAP[index as usize];
    }
    tones
}

/// Funzione degli errori, in doppia precisione (`std` non la offre). Serie a termini positivi, stabile e precisa a circa 1e-15.
fn erf(x: f64) -> f64 {
    if x.is_nan() {
        return f64::NAN;
    }
    let a = x.abs();
    if a > 6.0 {
        return x.signum();
    }
    let a2 = a * a;
    let mut term = a;
    let mut sum = a;
    for n in 0..500 {
        term *= 2.0 * a2 / (2.0 * n as f64 + 3.0);
        sum += term;
        if term < 1e-17 * sum {
            break;
        }
    }
    let value = (2.0 / PI.sqrt() * (-a2).exp() * sum).min(1.0);
    value.copysign(x)
}

/// Impulso di frequenza gaussiano (`gfsk_pulse`): risposta, nel tempo `t` (in durate di simbolo), di un filtro gaussiano
/// di prodotto banda-tempo `bt` a un gradino di una durata di simbolo. Somma 1 sulle traslazioni intere.
pub fn gfsk_pulse(bt: f32, t: f32) -> f32 {
    gfsk_pulse_f64(f64::from(bt), f64::from(t)) as f32
}

fn gfsk_pulse_f64(bt: f64, t: f64) -> f64 {
    let c = PI * (2.0 / std::f64::consts::LN_2).sqrt();
    0.5 * (erf(c * bt * (t + 0.5)) - erf(c * bt * (t - 0.5)))
}

/// I 3*NSPS campioni dell'impulso, centrato sul campione 1.5*NSPS come in `gen_ft8wave`: `tt = (i - 1.5*nsps) / nsps` con `i` da 1.
fn pulse_table(bt: f64) -> Vec<f64> {
    (0..3 * NSPS)
        .map(|i0| {
            let tt = ((i0 + 1) as f64 - 1.5 * NSPS as f64) / NSPS as f64;
            gfsk_pulse_f64(bt, tt)
        })
        .collect()
}

/// Onda GFSK (BT = 2.0) di un messaggio, con frequenza di base `f0_hz`, a 12 kHz.
///
/// Restituisce [`WAVE_SAMPLES`] campioni a picco 1.0, con rampa coseno rialzato di [`RAMP_SAMPLES`] campioni a inizio
/// e fine. Il tono `k` sta a `f0_hz + 6.25 * k` Hz. Il contenuto va da circa `f0_hz` a `f0_hz + 43.75` Hz piu' i fianchi
/// del filtro gaussiano (banda occupata circa 50 Hz).
pub fn gfsk_wave(tones: &[u8; 79], f0_hz: f32) -> Vec<f32> {
    gfsk_wave_bt(tones, f0_hz, GFSK_BT)
}

/// Come [`gfsk_wave`] ma con prodotto banda-tempo `bt` a scelta (in JTDX `gen_ft8wavesub` usa 4.0 per la sottrazione).
pub fn gfsk_wave_bt(tones: &[u8; 79], f0_hz: f32, bt: f32) -> Vec<f32> {
    gfsk_iq(tones, f0_hz, bt).0
}

/// L'onda GFSK in fase (`sin`) e in quadratura (`cos`), stesse rampe: serve alla sottrazione dei segnali gia' decodificati,
/// dove ampiezza e fase del segnale ricevuto si stimano con i due riferimenti.
pub fn gfsk_iq(tones: &[u8; 79], f0_hz: f32, bt: f32) -> (Vec<f32>, Vec<f32>) {
    let nsps = NSPS;
    let pulse = pulse_table(f64::from(bt));
    let twopi = 2.0 * PI;
    let dphi_peak = twopi / nsps as f64; // hmod = 1.0: indice di modulazione 1, un tono = 1/NSPS di giro per campione

    // Frequenza smussata: (NN + 2) simboli, il primo e l'ultimo estesi con due simboli fittizi.
    let mut dphi = vec![0.0f64; (NN + 2) * nsps];
    for (j, &tone) in tones.iter().enumerate() {
        let ib = j * nsps;
        let amp = dphi_peak * f64::from(tone);
        for (d, p) in dphi[ib..ib + 3 * nsps].iter_mut().zip(&pulse) {
            *d += amp * p;
        }
    }
    // Simboli fittizi all'inizio e in fondo, con il tono del primo e dell'ultimo.
    let amp_first = dphi_peak * f64::from(tones[0]);
    for (d, p) in dphi[..2 * nsps].iter_mut().zip(&pulse[nsps..3 * nsps]) {
        *d += amp_first * p;
    }
    let amp_last = dphi_peak * f64::from(tones[NN - 1]);
    for (d, p) in dphi[NN * nsps..(NN + 2) * nsps].iter_mut().zip(&pulse[..2 * nsps]) {
        *d += amp_last * p;
    }

    // Fase accumulata, con la portante f0. Si parte dal campione `nsps` di dphi: i simboli fittizi non entrano nell'onda.
    let shift = twopi * f64::from(f0_hz) / SAMPLE_RATE as f64;
    let mut phi = 0.0f64;
    let mut wave = Vec::with_capacity(WAVE_SAMPLES);
    let mut quad = Vec::with_capacity(WAVE_SAMPLES);
    for &d in &dphi[nsps..nsps + WAVE_SAMPLES] {
        wave.push(phi.sin() as f32);
        quad.push(phi.cos() as f32);
        phi = (phi + d + shift) % twopi;
    }

    apply_ramps(&mut wave);
    apply_ramps(&mut quad);
    (wave, quad)
}

/// Onda FSK pura, senza filtro gaussiano e senza rampe: fase continua, frequenza costante `f0_hz + 6.25 * tono` per
/// 1920 campioni. Serve alle prove (e come riferimento): non e' quello che trasmette JTDX.
pub fn fsk_wave(tones: &[u8; 79], f0_hz: f32) -> Vec<f32> {
    let twopi = 2.0 * PI;
    let mut phi = 0.0f64;
    let mut wave = Vec::with_capacity(WAVE_SAMPLES);
    for &tone in tones {
        let f = f64::from(f0_hz) + f64::from(TONE_SPACING_HZ) * f64::from(tone);
        let step = twopi * f / SAMPLE_RATE as f64;
        for _ in 0..NSPS {
            wave.push(phi.sin() as f32);
            phi = (phi + step) % twopi;
        }
    }
    wave
}

/// Rampa di salita sui primi `RAMP_SAMPLES` campioni e di discesa sugli ultimi (inviluppo coseno rialzato, come in `gen_ft8wave`).
fn apply_ramps(wave: &mut [f32]) {
    let nramp = RAMP_SAMPLES;
    let len = wave.len();
    if len < 2 * nramp {
        return;
    }
    for i in 0..nramp {
        let arg = 2.0 * PI * i as f64 / (2.0 * nramp as f64);
        let up = ((1.0 - arg.cos()) / 2.0) as f32;
        let down = ((1.0 + arg.cos()) / 2.0) as f32;
        wave[i] *= up;
        wave[len - nramp + i] *= down;
    }
}

/// Verifica la frequenza di base: finita e tale che tutti gli 8 toni (50 Hz) stiano sotto la frequenza di Nyquist.
fn check_frequency(f0_hz: f32) -> Result<(), MessageError> {
    let top = f0_hz + 8.0 * TONE_SPACING_HZ;
    if f0_hz.is_finite() && f0_hz >= 0.0 && f64::from(top) < SAMPLE_RATE as f64 / 2.0 {
        Ok(())
    } else {
        Err(MessageError::Invalid(format!("frequenza di trasmissione fuori campo: {f0_hz} Hz")))
    }
}

/// Dai 77 bit di un messaggio all'onda: CRC, codice LDPC, toni, GFSK.
pub fn encode_bits77(bits: &Bits77, f0_hz: f32) -> Result<Vec<f32>, MessageError> {
    check_frequency(f0_hz)?;
    let with_crc = crc::append_crc(bits);
    let codeword = ldpc::encode(&with_crc);
    let tones = tones_from_codeword(&codeword);
    Ok(gfsk_wave(&tones, f0_hz))
}

/// Dal testo all'onda pronta da trasmettere.
pub fn encode_message(text: &str, f0_hz: f32, hashes: &mut CallHashes) -> Result<Vec<f32>, MessageError> {
    check_frequency(f0_hz)?;
    let bits = message::pack(text, hashes)?;
    encode_bits77(&bits, f0_hz)
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use rustfft::FftPlanner;
    use rustfft::num_complex::Complex;

    use super::*;

    /// Toni di prova pseudo-casuali ma ripetibili (generatore lineare), con Costas al suo posto.
    fn test_tones() -> [u8; 79] {
        let mut state = 12345u32;
        let mut tones = [0u8; 79];
        for t in tones.iter_mut() {
            state = state.wrapping_mul(1664525).wrapping_add(1013904223);
            *t = ((state >> 16) % 8) as u8;
        }
        for start in [0usize, 36, 72] {
            tones[start..start + 7].copy_from_slice(&COSTAS);
        }
        tones
    }

    fn fft(input: &[f32], len: usize) -> Vec<Complex<f64>> {
        let mut buf: Vec<Complex<f64>> = input.iter().map(|&x| Complex::new(f64::from(x), 0.0)).collect();
        buf.resize(len, Complex::new(0.0, 0.0));
        let mut planner = FftPlanner::<f64>::new();
        let plan = planner.plan_fft_forward(len);
        plan.process(&mut buf);
        buf
    }

    /// Frequenza istantanea in Hz per ogni campione, dal segnale analitico (trasformata di Hilbert via FFT).
    fn instantaneous_frequency(wave: &[f32]) -> Vec<f64> {
        let n = wave.len();
        let mut spectrum = fft(wave, n);
        // Segnale analitico: si azzerano le frequenze negative e si raddoppiano le positive.
        let half = n / 2;
        for (k, v) in spectrum.iter_mut().enumerate() {
            if k == 0 || (n.is_multiple_of(2) && k == half) {
                continue;
            }
            if k < half + n % 2 {
                *v *= 2.0;
            } else {
                *v = Complex::new(0.0, 0.0);
            }
        }
        let mut planner = FftPlanner::<f64>::new();
        let inverse: Arc<dyn rustfft::Fft<f64>> = planner.plan_fft_inverse(n);
        inverse.process(&mut spectrum);
        let analytic: Vec<Complex<f64>> = spectrum.iter().map(|v| *v / n as f64).collect();
        let mut freq = vec![0.0; n];
        for i in 1..n - 1 {
            let d = (analytic[i + 1] * analytic[i - 1].conj()).arg();
            freq[i] = d / 2.0 * SAMPLE_RATE as f64 / (2.0 * PI);
        }
        freq
    }

    #[test]
    fn costas_nelle_posizioni_giuste() {
        let tones = tones_from_codeword(&[0u8; 174]);
        for start in [0usize, 36, 72] {
            assert_eq!(&tones[start..start + 7], &COSTAS, "Costas a {start}");
        }
        let tones = tones_from_codeword(&[1u8; 174]);
        for start in [0usize, 36, 72] {
            assert_eq!(&tones[start..start + 7], &COSTAS, "Costas a {start} con bit a 1");
        }
    }

    #[test]
    fn gray_di_una_parola_nota() {
        // Gruppi di 3 bit: 000 001 010 011 100 101 110 111 poi ripetuti; ci si aspetta graymap = 0 1 3 2 5 6 4 7.
        let mut codeword = [0u8; 174];
        for g in 0..58 {
            let v = g % 8;
            codeword[3 * g] = ((v >> 2) & 1) as u8;
            codeword[3 * g + 1] = ((v >> 1) & 1) as u8;
            codeword[3 * g + 2] = (v & 1) as u8;
        }
        let tones = tones_from_codeword(&codeword);
        let attesi_gray = [0u8, 1, 3, 2, 5, 6, 4, 7];
        for g in 0..58 {
            let k = if g < 29 { 7 + g } else { 14 + g };
            assert_eq!(tones[k], attesi_gray[g % 8], "gruppo {g}, simbolo {k}");
        }
        // I simboli di dati sono esattamente quelli non occupati dal Costas.
        let mut occupati = [false; 79];
        for g in 0..58 {
            occupati[if g < 29 { 7 + g } else { 14 + g }] = true;
        }
        for start in [0usize, 36, 72] {
            for o in &mut occupati[start..start + 7] {
                assert!(!*o);
                *o = true;
            }
        }
        assert!(occupati.iter().all(|&o| o));
    }

    #[test]
    fn gray_primo_e_ultimo_gruppo() {
        let mut codeword = [0u8; 174];
        // primo gruppo 110 -> graymap[6] = 4 al simbolo 7; ultimo gruppo 101 -> graymap[5] = 6 al simbolo 71
        codeword[0] = 1;
        codeword[1] = 1;
        codeword[171] = 1;
        codeword[173] = 1;
        let tones = tones_from_codeword(&codeword);
        assert_eq!(tones[7], 4);
        assert_eq!(tones[71], 6);
        // il gruppo 30 (indice 29) cade dopo il secondo Costas: simbolo 43
        let mut cw = [0u8; 174];
        cw[87] = 1;
        cw[88] = 1;
        cw[89] = 1; // 111 -> 7
        assert_eq!(tones_from_codeword(&cw)[43], 7);
    }

    #[test]
    fn erf_valori_noti() {
        let casi = [
            (0.0, 0.0),
            (0.5, 0.520_499_877_813_046_5),
            (1.0, 0.842_700_792_949_714_9),
            (2.0, 0.995_322_265_018_952_7),
            (3.0, 0.999_977_909_503_001_4),
            (-1.0, -0.842_700_792_949_714_9),
        ];
        for (x, atteso) in casi {
            assert!((erf(x) - atteso).abs() < 1e-12, "erf({x}) = {}", erf(x));
        }
        assert_eq!(erf(10.0), 1.0);
        assert_eq!(erf(-10.0), -1.0);
    }

    #[test]
    fn impulso_gaussiano_somma_uno() {
        for t in [0.0f32, 0.1, 0.25, 0.5, 0.9] {
            let somma: f32 = (-3..=3).map(|k| gfsk_pulse(GFSK_BT, t + k as f32)).sum();
            assert!((somma - 1.0).abs() < 1e-5, "t={t}, somma={somma}");
        }
        assert!((gfsk_pulse(GFSK_BT, 0.3) - gfsk_pulse(GFSK_BT, -0.3)).abs() < 1e-6);
    }

    #[test]
    fn lunghezza_onda() {
        let wave = gfsk_wave(&test_tones(), 1500.0);
        assert_eq!(wave.len(), 151_680);
        assert_eq!(wave.len(), WAVE_SAMPLES);
        assert_eq!(fsk_wave(&test_tones(), 1500.0).len(), 151_680);
    }

    #[test]
    fn frequenza_al_centro_di_ogni_simbolo() {
        let tones = test_tones();
        let f0 = 1234.5f32;
        let wave = gfsk_wave(&tones, f0);
        let freq = instantaneous_frequency(&wave);
        for (s, &t) in tones.iter().enumerate() {
            let atteso = f64::from(f0) + 6.25 * f64::from(t);
            let misurata = freq[s * NSPS + NSPS / 2];
            assert!((misurata - atteso).abs() < 0.05, "simbolo {s}: {misurata} Hz invece di {atteso} Hz");
        }
    }

    #[test]
    fn picco_fft_per_simbolo() {
        // f0 multiplo di 6.25 Hz: ogni tono cade esattamente su un canale della FFT di un simbolo (1920 campioni).
        let tones = test_tones();
        let f0 = 1500.0f32; // canale 240
        let wave = gfsk_wave(&tones, f0);
        for (s, &t) in tones.iter().enumerate() {
            let spettro = fft(&wave[s * NSPS..(s + 1) * NSPS], NSPS);
            let (picco, _) = spettro[..NSPS / 2]
                .iter()
                .enumerate()
                .map(|(k, v)| (k, v.norm_sqr()))
                .max_by(|a, b| a.1.total_cmp(&b.1))
                .unwrap();
            assert_eq!(picco, 240 + t as usize, "simbolo {s}");
        }
    }

    #[test]
    fn fsk_pura_ha_il_tono_esatto() {
        let tones = test_tones();
        let wave = fsk_wave(&tones, 1500.0);
        for (s, &t) in tones.iter().enumerate() {
            let spettro = fft(&wave[s * NSPS..(s + 1) * NSPS], NSPS);
            let (picco, _) = spettro[..NSPS / 2]
                .iter()
                .enumerate()
                .map(|(k, v)| (k, v.norm_sqr()))
                .max_by(|a, b| a.1.total_cmp(&b.1))
                .unwrap();
            assert_eq!(picco, 240 + t as usize, "simbolo {s}");
        }
    }

    #[test]
    fn spettro_occupa_circa_cinquanta_hz() {
        let tones = test_tones();
        let f0 = 1500.0f32;
        let wave = gfsk_wave(&tones, f0);
        let len = 262_144usize; // risoluzione 12000/262144 = 0.046 Hz
        let spettro = fft(&wave, len);
        let hz = |k: usize| k as f64 * SAMPLE_RATE as f64 / len as f64;
        let mut totale = 0.0;
        let mut dentro = 0.0;
        for (k, v) in spettro[..len / 2].iter().enumerate() {
            let p = v.norm_sqr();
            totale += p;
            let f = hz(k);
            // dai toni 0-7 (43.75 Hz) piu' un canale di margine per parte: circa 56 Hz
            if f >= f64::from(f0) - 6.25 && f <= f64::from(f0) + 43.75 + 6.25 {
                dentro += p;
            }
        }
        assert!(dentro / totale > 0.99, "in banda {:.5}", dentro / totale);
        // Il filtro gaussiano toglie i fianchi: lontano oltre 25 Hz dai bordi resta pochissimo.
        let mut lontano = 0.0;
        for (k, v) in spettro[..len / 2].iter().enumerate() {
            let f = hz(k);
            if f < f64::from(f0) - 25.0 || f > f64::from(f0) + 43.75 + 25.0 {
                lontano += v.norm_sqr();
            }
        }
        assert!(lontano / totale < 1e-4, "fuori banda {:.3e}", lontano / totale);
    }

    #[test]
    fn nessun_campione_fuori_da_meno_uno_a_uno() {
        for f0 in [200.0f32, 1500.0, 2900.0] {
            let wave = gfsk_wave(&test_tones(), f0);
            assert!(wave.iter().all(|x| x.is_finite() && (-1.0..=1.0).contains(x)));
            let picco = wave.iter().fold(0.0f32, |m, x| m.max(x.abs()));
            assert!(picco > 0.99, "picco {picco}");
        }
    }

    #[test]
    fn rampe_senza_scatti() {
        let tones = test_tones();
        let f0 = 1500.0f32;
        let wave = gfsk_wave(&tones, f0);
        assert_eq!(wave[0], 0.0);
        assert!(wave[WAVE_SAMPLES - 1].abs() < 1e-3);
        // Un campione di una sinusoide alla frequenza massima non puo' distare dal precedente piu' di 2*sin(pi*f/fs).
        let fmax = f64::from(f0) + 6.25 * 7.0;
        let limite = (2.0 * (PI * fmax / SAMPLE_RATE as f64).sin() * 1.02 + 1e-3) as f32;
        for (i, w) in wave.windows(2).enumerate() {
            assert!((w[1] - w[0]).abs() <= limite, "scatto al campione {i}: {}", (w[1] - w[0]).abs());
        }
        // L'inviluppo cresce senza buchi nei primi 240 campioni e cala negli ultimi: confronto con la FSK pura, a fase uguale.
        let nramp = RAMP_SAMPLES;
        let rms = |s: &[f32]| (s.iter().map(|x| f64::from(*x).powi(2)).sum::<f64>() / s.len() as f64).sqrt();
        let primo = rms(&wave[..nramp / 4]);
        let ultimo_quarto = rms(&wave[nramp - nramp / 4..nramp]);
        assert!(primo < 0.1 && ultimo_quarto > 0.6, "{primo} {ultimo_quarto}");
        let fine_a = rms(&wave[WAVE_SAMPLES - nramp..WAVE_SAMPLES - nramp + nramp / 4]);
        let fine_b = rms(&wave[WAVE_SAMPLES - nramp / 4..]);
        assert!(fine_a > 0.6 && fine_b < 0.1, "{fine_a} {fine_b}");
        // Oltre la rampa il segnale ha ampiezza piena (nessuna modulazione di ampiezza).
        let meta = rms(&wave[WAVE_SAMPLES / 2..WAVE_SAMPLES / 2 + NSPS]);
        assert!((meta - std::f64::consts::FRAC_1_SQRT_2).abs() < 0.01, "{meta}");
    }

    #[test]
    fn rampa_uguale_al_sorgente() {
        let mut unit = vec![1.0f32; WAVE_SAMPLES];
        apply_ramps(&mut unit);
        assert_eq!(unit[0], 0.0);
        assert!((unit[RAMP_SAMPLES] - 1.0).abs() < 1e-9);
        assert!((unit[WAVE_SAMPLES - RAMP_SAMPLES - 1] - 1.0).abs() < 1e-9);
        assert_eq!(unit[WAVE_SAMPLES - RAMP_SAMPLES], 1.0);
        // meta' rampa: (1 - cos(pi/2)) / 2 = 0.5
        assert!((unit[RAMP_SAMPLES / 2] - 0.5).abs() < 1e-6);
        assert!(unit[WAVE_SAMPLES - 1] < 1e-4);
    }

    #[test]
    fn frequenza_fuori_campo_e_un_errore() {
        let bits = [0u8; 77];
        assert!(encode_bits77(&bits, f32::NAN).is_err());
        assert!(encode_bits77(&bits, -1.0).is_err());
        assert!(encode_bits77(&bits, 5990.0).is_err());
        let mut hashes = CallHashes::default();
        assert!(encode_message("CQ TEST", f32::INFINITY, &mut hashes).is_err());
    }

    #[test]
    fn dai_bit_all_onda() {
        // Funziona anche con crc e ldpc ancora segnaposto: conta la composizione e la lunghezza.
        let wave = encode_bits77(&[0u8; 77], 1000.0).expect("onda");
        assert_eq!(wave.len(), WAVE_SAMPLES);
        assert!(wave.iter().all(|x| (-1.0..=1.0).contains(x)));
    }

    #[test]
    fn dal_testo_all_onda_quando_il_messaggio_e_gestito() {
        let mut hashes = CallHashes::default();
        match encode_message("CQ TEST AA00", 1500.0, &mut hashes) {
            Ok(wave) => {
                assert_eq!(wave.len(), WAVE_SAMPLES);
                assert!(wave.iter().all(|x| (-1.0..=1.0).contains(x)));
            }
            // finche' `message::pack` e' un segnaposto
            Err(MessageError::Unsupported) => {}
            Err(e) => panic!("errore inatteso: {e:?}"),
        }
    }
}
