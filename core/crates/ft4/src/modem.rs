//! Modulazione FT4: dal messaggio ai 103 toni e all'onda GFSK. Portato da JTDX 2.2.159 (`lib/ft4/genft4.f90`, `lib/ft4/gen_ft4wave.f90`,
//! `lib/gfsk_pulse.f90`).
//!
//! - [`codeword_from_bits77`]: i 77 bit si sommano (XOR) al vettore [`RVEC`], si aggiunge il CRC a 14 bit **sul messaggio gia' mescolato** e
//!   si codifica con l'LDPC(174,91) di `ft8`;
//! - [`tones_from_codeword`]: struttura C4 D29 C4 D29 C4 D29 C4 (quattro matrici di Costas 4x4 e 87 simboli di dati), con i dati a coppie di bit
//!   in codifica Gray (00 -> 0, 01 -> 1, 11 -> 2, 10 -> 3);
//! - [`gfsk_wave`]: l'onda a 12 kHz con impulso gaussiano BT = 1,0 e 576 campioni per simbolo.
//!
//! Lunghezza dell'onda: 105 simboli, `NN2 * NSPS` = 60480 campioni (5,04 s): il primo simbolo e l'ultimo sono di rampa (coseno rialzato) e
//! non portano informazione. Il primo simbolo di Costas comincia quindi a [`NSPS`] campioni dall'inizio dell'onda: se il segnale deve avere
//! `dt_s` (scarto rispetto a 0,5 s dal via del periodo) l'onda parte al campione `(0,5 + dt_s) * 12000 - NSPS`. Il silenzio fino ai 7,5 s
//! del periodo e il ritardo di trasmissione li gestisce chi riproduce l'audio.
//!
//! Ampiezza: come nel sorgente l'onda e' `sin(fase)`, quindi a picco 1,0; le rampe possono solo ridurla.

use std::f64::consts::PI;

use ft8::message::{self, Bits77, CallHashes, MessageError};
use ft8::modem::gfsk_pulse;
use ft8::{crc, ldpc};

use crate::sync::Complex;
use crate::{COSTAS, GRAYMAP, N, ND, NN, NN2, NSPS, RVEC, SAMPLE_RATE, TONE_SPACING_HZ};

/// Prodotto banda-tempo dell'impulso gaussiano di FT4 (`gfsk_pulse(1.0, tt)`).
pub const GFSK_BT: f32 = 1.0;

/// Campioni dell'onda completa: 105 simboli da 576.
pub const WAVE_SAMPLES: usize = NN2 * NSPS;

/// I 77 bit del messaggio mescolati con [`RVEC`] (`msgbits = mod(msgbits + rvec, 2)`); lo scrambling e' una somma, quindi si fa e si toglie
/// con la stessa funzione.
pub fn scramble(bits: &Bits77) -> Bits77 {
    let mut out = [0u8; 77];
    for ((o, &b), &r) in out.iter_mut().zip(bits.iter()).zip(RVEC.iter()) {
        *o = (b ^ r) & 1;
    }
    out
}

/// Dai 77 bit del messaggio ai 174 bit trasmessi: scrambling, CRC sul messaggio mescolato, codice LDPC.
pub fn codeword_from_bits77(bits: &Bits77) -> [u8; N] {
    let with_crc = crc::append_crc(&scramble(bits));
    ldpc::encode(&with_crc)
}

/// Dai 174 bit ai 103 simboli (0-3): Costas a 0-3, 33-36, 66-69 e 99-102; i dati a coppie di bit con codifica Gray nei simboli 4-32, 37-65 e 70-98.
///
/// Ogni elemento di `codeword` conta solo per il suo bit meno significativo.
pub fn tones_from_codeword(codeword: &[u8; N]) -> [u8; NN] {
    let mut data = [0u8; ND];
    let (pairs, _) = codeword.as_chunks::<2>();
    for (d, pair) in data.iter_mut().zip(pairs) {
        *d = GRAYMAP[usize::from(((pair[0] & 1) << 1) | (pair[1] & 1))];
    }
    let mut tones = [0u8; NN];
    tones[0..4].copy_from_slice(&COSTAS[0]);
    tones[4..33].copy_from_slice(&data[0..29]);
    tones[33..37].copy_from_slice(&COSTAS[1]);
    tones[37..66].copy_from_slice(&data[29..58]);
    tones[66..70].copy_from_slice(&COSTAS[2]);
    tones[70..99].copy_from_slice(&data[58..87]);
    tones[99..103].copy_from_slice(&COSTAS[3]);
    tones
}

/// Dai 77 bit del messaggio ai 103 toni.
pub fn tones_from_bits77(bits: &Bits77) -> [u8; NN] {
    tones_from_codeword(&codeword_from_bits77(bits))
}

/// I 3*NSPS campioni dell'impulso, centrato sul campione 1,5*NSPS come in `gen_ft4wave`: `tt = (i - 1.5*nsps) / nsps` con `i` da 1.
fn pulse_table() -> Vec<f64> {
    (0..3 * NSPS)
        .map(|i0| {
            let tt = ((i0 + 1) as f64 - 1.5 * NSPS as f64) / NSPS as f64;
            f64::from(gfsk_pulse(GFSK_BT, tt as f32))
        })
        .collect()
}

/// L'onda GFSK in fase (`sin`) e in quadratura (`cos`), con le stesse rampe: la sottrazione dei segnali decodificati stima ampiezza e fase del
/// segnale ricevuto con il riferimento complesso (`icmplx = 1` di `gen_ft4wave`).
pub fn gfsk_iq(tones: &[u8; NN], f0_hz: f32) -> (Vec<f32>, Vec<f32>) {
    let pulse = pulse_table();
    let twopi = 2.0 * PI;
    let dphi_peak = twopi / NSPS as f64; // hmod = 1: un tono = 1/NSPS di giro per campione

    // frequenza smussata: il simbolo j occupa [(j + 1) * NSPS, (j + 2) * NSPS), centrato sul campione (j + 1,5) * NSPS dell'impulso
    let mut dphi = vec![0.0f64; WAVE_SAMPLES];
    for (j, &tone) in tones.iter().enumerate() {
        let amp = dphi_peak * f64::from(tone);
        for (d, p) in dphi[j * NSPS..(j + 3) * NSPS].iter_mut().zip(&pulse) {
            *d += amp * p;
        }
    }
    let shift = twopi * f64::from(f0_hz) / SAMPLE_RATE as f64;
    let mut phi = 0.0f64;
    let mut wave = Vec::with_capacity(WAVE_SAMPLES);
    let mut quad = Vec::with_capacity(WAVE_SAMPLES);
    for &d in &dphi {
        wave.push(phi.sin() as f32);
        quad.push(phi.cos() as f32);
        phi = (phi + d + shift) % twopi;
    }
    apply_ramps(&mut wave);
    apply_ramps(&mut quad);
    (wave, quad)
}

/// Onda GFSK (BT = 1,0) di un messaggio, con frequenza di base `f0_hz` (tono 0), a 12 kHz: [`WAVE_SAMPLES`] campioni a picco 1,0.
///
/// Il tono `k` sta a `f0_hz + 20,833 * k` Hz; la banda occupata e' di circa 100 Hz.
pub fn gfsk_wave(tones: &[u8; NN], f0_hz: f32) -> Vec<f32> {
    gfsk_iq(tones, f0_hz).0
}

/// Come [`gfsk_iq`] ma con il riferimento complesso `cos + j sin` (il `cwave` di `gen_ft4wave`).
pub fn gfsk_complex(tones: &[u8; NN], f0_hz: f32) -> Vec<Complex> {
    let (s, c) = gfsk_iq(tones, f0_hz);
    c.into_iter().zip(s).map(|(re, im)| Complex::new(re, im)).collect()
}

/// Onda FSK pura, senza filtro gaussiano e senza rampe: fase continua, frequenza costante `f0_hz + 20,833 * tono` per 576 campioni.
/// Serve alle prove (come riferimento): non e' quello che trasmette JTDX. Dura 103 simboli, senza i due di rampa.
pub fn fsk_wave(tones: &[u8; NN], f0_hz: f32) -> Vec<f32> {
    let twopi = 2.0 * PI;
    let mut phi = 0.0f64;
    let mut wave = Vec::with_capacity(NN * NSPS);
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

/// Rampa di salita sul primo simbolo e di discesa sull'ultimo (coseno rialzato su `NSPS` campioni, come in `gen_ft4wave`).
fn apply_ramps(wave: &mut [f32]) {
    let len = wave.len();
    if len < 2 * NSPS {
        return;
    }
    for i in 0..NSPS {
        let arg = 2.0 * PI * i as f64 / (2.0 * NSPS as f64);
        wave[i] *= ((1.0 - arg.cos()) / 2.0) as f32;
        wave[len - NSPS + i] *= ((1.0 + arg.cos()) / 2.0) as f32;
    }
}

/// Verifica la frequenza di base: finita e tale che i 4 toni (62,5 Hz) con i fianchi del filtro stiano sotto la frequenza di Nyquist.
fn check_frequency(f0_hz: f32) -> Result<(), MessageError> {
    let top = f0_hz + 5.0 * TONE_SPACING_HZ;
    if f0_hz.is_finite() && f0_hz >= 0.0 && f64::from(top) < SAMPLE_RATE as f64 / 2.0 {
        Ok(())
    } else {
        Err(MessageError::Invalid(format!("frequenza di trasmissione fuori campo: {f0_hz} Hz")))
    }
}

/// Dai 77 bit di un messaggio all'onda: scrambling, CRC, codice LDPC, toni, GFSK.
pub fn encode_bits77(bits: &Bits77, f0_hz: f32) -> Result<Vec<f32>, MessageError> {
    check_frequency(f0_hz)?;
    Ok(gfsk_wave(&tones_from_bits77(bits), f0_hz))
}

/// Dal testo all'onda pronta da trasmettere.
pub fn encode_message(text: &str, f0_hz: f32, hashes: &mut CallHashes) -> Result<Vec<f32>, MessageError> {
    check_frequency(f0_hz)?;
    let bits = message::pack(text, hashes)?;
    encode_bits77(&bits, f0_hz)
}

#[cfg(test)]
mod tests {
    use rustfft::FftPlanner;
    use rustfft::num_complex::Complex as C64;

    use super::*;

    /// Il vettore di scrambling di FT4 in byte (`kFT4_XOR_sequence` di ft8_lib, kgoba/ft8_lib `ft8/constants.c`: una fonte indipendente da
    /// JTDX): gli ultimi 3 bit del byte finale non contano.
    const XOR_SEQUENCE: [u8; 10] = [0x4A, 0x5E, 0x89, 0xB4, 0xB0, 0x8A, 0x79, 0x55, 0xBE, 0x28];

    #[test]
    fn rvec_uguale_alla_sequenza_xor_di_ft8_lib() {
        for (i, &r) in RVEC.iter().enumerate() {
            let atteso = (XOR_SEQUENCE[i / 8] >> (7 - i % 8)) & 1;
            assert_eq!(r, atteso, "bit {i}");
        }
    }

    #[test]
    fn scrambling_si_toglie_con_se_stesso() {
        let mut bits = [0u8; 77];
        for (i, b) in bits.iter_mut().enumerate() {
            *b = u8::from((i * 7 + 3) % 5 == 0);
        }
        assert_eq!(scramble(&scramble(&bits)), bits);
        assert_ne!(scramble(&bits), bits);
        assert_eq!(scramble(&[0u8; 77]), RVEC);
    }

    #[test]
    fn costas_nelle_posizioni_giuste() {
        for cw in [[0u8; 174], [1u8; 174]] {
            let tones = tones_from_codeword(&cw);
            for (m, start) in [0usize, 33, 66, 99].into_iter().enumerate() {
                assert_eq!(&tones[start..start + 4], &COSTAS[m], "Costas {m} a {start}");
            }
        }
        // le matrici sono quelle di WSJT-X / ft8_lib (kFT4_Costas_pattern)
        assert_eq!(COSTAS, [[0, 1, 3, 2], [1, 0, 2, 3], [2, 3, 1, 0], [3, 2, 0, 1]]);
    }

    #[test]
    fn gray_a_coppie_di_bit() {
        // coppie 00 01 10 11 ripetute: toni 0 1 3 2
        let mut cw = [0u8; 174];
        for g in 0..87 {
            let v = g % 4;
            cw[2 * g] = ((v >> 1) & 1) as u8;
            cw[2 * g + 1] = (v & 1) as u8;
        }
        let tones = tones_from_codeword(&cw);
        let gray = [0u8, 1, 3, 2];
        let mut occupati = [false; NN];
        for g in 0..87 {
            let k = if g < 29 {
                4 + g
            } else if g < 58 {
                8 + g
            } else {
                12 + g
            };
            assert_eq!(tones[k], gray[g % 4], "coppia {g}, simbolo {k}");
            occupati[k] = true;
        }
        for start in [0usize, 33, 66, 99] {
            for o in &mut occupati[start..start + 4] {
                assert!(!*o);
                *o = true;
            }
        }
        assert!(occupati.iter().all(|&o| o), "dati e Costas coprono tutti i 103 simboli");
    }

    #[test]
    fn il_codice_ha_crc_sul_messaggio_mescolato() {
        let mut hashes = CallHashes::default();
        let bits = message::pack("CQ K1ABC FN42", &mut hashes).expect("messaggio valido");
        let cw = codeword_from_bits77(&bits);
        let mut m91 = [0u8; 91];
        m91.copy_from_slice(&cw[..91]);
        assert!(crc::check(&m91), "il CRC torna sui 91 bit trasmessi");
        assert_eq!(&m91[..77], &scramble(&bits)[..], "i bit del codice sono il messaggio mescolato");
        // il messaggio in chiaro con il CRC del mescolato non e' valido
        let mut chiaro = m91;
        chiaro[..77].copy_from_slice(&bits);
        assert!(!crc::check(&chiaro));
    }

    fn dft_bins(x: &[f32], bins: usize) -> Vec<f64> {
        let n = x.len();
        let mut buf: Vec<C64<f64>> = x.iter().map(|&v| C64::new(f64::from(v), 0.0)).collect();
        FftPlanner::<f64>::new().plan_fft_forward(n).process(&mut buf);
        buf.iter().take(bins).map(|c| c.norm()).collect()
    }

    #[test]
    fn onda_lunghezza_e_frequenza_per_simbolo() {
        let mut hashes = CallHashes::default();
        let wave = encode_message("CQ K1ABC FN42", 1500.0, &mut hashes).expect("messaggio valido");
        assert_eq!(wave.len(), 105 * 576);
        assert!(wave.iter().all(|x| x.is_finite() && x.abs() <= 1.0 + 1e-6));
        let peak = wave.iter().fold(0.0f32, |a, &x| a.max(x.abs()));
        assert!(peak > 0.99, "picco {peak}");
        // rampe: il primo e l'ultimo simbolo partono e finiscono da zero
        assert!(wave[0].abs() < 1e-3 && wave[wave.len() - 1].abs() < 0.01);

        // 1500 Hz = 72 * 20,8333: la FFT di 576 campioni ha i bin esattamente sui toni, 72 + tono
        let mut hashes = CallHashes::default();
        let bits = message::pack("CQ K1ABC FN42", &mut hashes).expect("messaggio valido");
        let tones = tones_from_bits77(&bits);
        for (j, &tone) in tones.iter().enumerate() {
            let from = (j + 1) * NSPS; // il simbolo di rampa in salita viene prima
            let spec = dft_bins(&wave[from..from + NSPS], 80);
            let best = (72..76).max_by(|&a, &b| spec[a].total_cmp(&spec[b])).expect("intervallo non vuoto");
            assert_eq!(best, 72 + usize::from(tone), "simbolo {j}");
        }
    }

    #[test]
    fn tono_costante_sta_alla_frequenza_nominale() {
        // dieci simboli uguali: la frequenza al centro si assesta sul tono (l'impulso ha memoria di tre simboli)
        let mut tones = [0u8; NN];
        for t in tones.iter_mut().skip(20).take(10) {
            *t = 3;
        }
        let f0 = 1000.0f32;
        let wave = gfsk_wave(&tones, f0);
        let from = 26 * NSPS; // simbolo 25 (dopo la rampa), circondato da simboli uguali
        let n = 8 * NSPS; // zero-padding per una risoluzione di 2,6 Hz
        let mut buf = vec![0.0f32; n];
        buf[..NSPS].copy_from_slice(&wave[from..from + NSPS]);
        let spec = dft_bins(&buf, n / 2);
        let k = (0..n / 2).max_by(|&a, &b| spec[a].total_cmp(&spec[b])).expect("spettro non vuoto");
        let f = k as f64 * SAMPLE_RATE as f64 / n as f64;
        let atteso = f64::from(f0) + 3.0 * f64::from(TONE_SPACING_HZ);
        assert!((f - atteso).abs() < 3.0, "{f} contro {atteso}");
    }

    #[test]
    fn fsk_pura_ha_la_frequenza_dei_toni() {
        let tones = tones_from_bits77(&[0u8; 77]);
        let w = fsk_wave(&tones, 1500.0);
        assert_eq!(w.len(), 103 * NSPS);
        let spec = dft_bins(&w[..NSPS], 80);
        let best = (72..76).max_by(|&a, &b| spec[a].total_cmp(&spec[b])).expect("intervallo non vuoto");
        assert_eq!(best, 72 + usize::from(tones[0]));
    }

    #[test]
    fn frequenza_fuori_campo_e_un_errore() {
        let mut h = CallHashes::default();
        assert!(encode_message("CQ K1ABC FN42", -1.0, &mut h).is_err());
        assert!(encode_message("CQ K1ABC FN42", 5990.0, &mut h).is_err());
        assert!(encode_message("CQ K1ABC FN42", f32::NAN, &mut h).is_err());
        assert!(encode_message("questo non e' un messaggio valido di FT4 perche' troppo lungo", 1500.0, &mut h).is_err());
    }
}
