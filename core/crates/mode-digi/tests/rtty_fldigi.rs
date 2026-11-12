//! Il ricevitore RTTY di fldigi come arbitro del nostro: lo stesso audio (fatto con il nostro modulatore, rumore bianco gaussiano, un
//! tono attenuato, un piccolo errore di frequenza) va a `RttyDemodulator` (12 kHz) e al ricevitore di fldigi 4.2.13 compilato dai suoi
//! sorgenti (a 8 kHz come in fldigi, dopo un ricampionamento). Si confronta il tasso di caratteri sbagliati: il nostro non deve essere
//! peggiore di fldigi oltre una piccola tolleranza.
//!
//! Solo con la feature (serve il sottomodulo `terze-parti/fldigi` e un compilatore C++):
//! `cargo test -j 6 -p mode-digi --features fldigi-oracle -- --test-threads=2 --nocapture`
#![cfg(feature = "fldigi-oracle")]

mod common;

use common::{Rng, a_8k, levenshtein, testo};
use mode_digi::fldigi_oracle::FldigiRtty;
use mode_digi::rtty::{self, RttyDemodulator, RttyParams};

const RATE: f64 = 12_000.0;
const FLDIGI_RATE: f64 = 8_000.0;
const AMP: f64 = 0.3;
/// Di quanti punti percentuali (media della riga) il nostro puo' sbagliare piu' di fldigi.
const TOLLERANZA: f64 = 2.0;

#[derive(Clone, Copy)]
enum Prova {
    Rumore,
    /// Lo space attenuato di questi dB.
    SpaceGiu(f64),
    /// Il trasmettitore fuori di questi Hz.
    Fuori(f32),
}

/// Errori (nostro, fldigi) e lunghezza del testo.
fn una_prova(shift: f32, baud: f32, snr_db: f64, prova: Prova, seed: u64) -> (usize, usize, usize) {
    let mut rng = Rng(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1);
    let centre = 1500.0_f32;
    let sent = testo(&mut rng, 120);
    let tx_centre = if let Prova::Fuori(hz) = prova { centre + hz } else { centre };
    let p = RttyParams { centre_hz: tx_centre, shift_hz: shift, baud, reverse: false };
    let sig = match prova {
        Prova::SpaceGiu(db) => rtty::modulate_levels(sent.as_bytes(), p, AMP as f32, (AMP * 10f64.powf(db / 20.0)) as f32),
        _ => rtty::modulate(sent.as_bytes(), p, AMP as f32),
    };
    let pad = RATE as usize;
    let mut audio = vec![0.0_f32; pad];
    audio.extend_from_slice(&sig);
    audio.extend(std::iter::repeat_n(0.0, pad));
    // SNR in 3 kHz: segnale A^2/2, rumore s^2/2 nei 3 kHz su 6
    let sigma = AMP / 10f64.powf(snr_db / 20.0);
    for s in audio.iter_mut() {
        *s += (sigma * rng.gauss()) as f32;
    }
    let pulisci = |v: Vec<u8>| -> Vec<u8> { v.into_iter().filter(|&c| c != b'\r' && c != b'\n').collect() };

    let mut ours = RttyDemodulator::new(RttyParams { centre_hz: centre, shift_hz: shift, baud, reverse: false });
    let mut got = Vec::new();
    for block in audio.chunks(1024) {
        got.extend(ours.process(block));
    }
    let ours_err = levenshtein(sent.as_bytes(), &pulisci(got));

    let mut theirs = FldigiRtty::new(f64::from(baud), f64::from(shift), f64::from(centre), FLDIGI_RATE);
    let low = a_8k(&audio);
    let mut got = Vec::new();
    for block in low.chunks(1024) {
        got.extend(theirs.process(block));
    }
    let theirs_err = levenshtein(sent.as_bytes(), &pulisci(got));
    (ours_err, theirs_err, sent.len())
}

#[test]
fn fldigi_decodifica_il_nostro_segnale_pulito() {
    // prima di tutto l'arbitro deve capire il nostro modulatore (mark = tono alto in USB, come in fldigi)
    let p = RttyParams { centre_hz: 1500.0, shift_hz: 170.0, baud: 45.45, reverse: false };
    let mut audio = vec![0.0; 4000];
    audio.extend(rtty::modulate(b"RYRYRY CQ CQ DE IU3ABC IU3ABC 599 73 K", p, 0.5));
    audio.extend(vec![0.0; 4000]);
    let mut fl = FldigiRtty::new(45.45, 170.0, 1500.0, FLDIGI_RATE);
    let r = String::from_utf8_lossy(&fl.process(&a_8k(&audio))).into_owned();
    assert!(r.contains("CQ CQ DE IU3ABC IU3ABC 599 73 K"), "fldigi ha ricevuto: {r:?}");
}

#[test]
fn il_nostro_ricevitore_non_e_peggiore_di_fldigi() {
    const SNRS: [f64; 4] = [0.0, -3.0, -6.0, -9.0];
    const RUNS: u64 = 2;
    let mut casi: Vec<(f32, f32, Prova, &str)> = Vec::new();
    for shift in [170.0, 425.0, 850.0] {
        for baud in [45.45, 50.0, 75.0] {
            casi.push((shift, baud, Prova::Rumore, "rumore"));
        }
    }
    casi.push((170.0, 45.45, Prova::SpaceGiu(-12.0), "space -12 dB"));
    casi.push((170.0, 45.45, Prova::Fuori(5.0), "+5 Hz"));
    casi.push((170.0, 75.0, Prova::Fuori(5.0), "+5 Hz"));

    println!("caratteri sbagliati in %, Aurora / fldigi, per SNR in 3 kHz {SNRS:?} dB");
    let mut peggiori = Vec::new();
    let (mut tot_a, mut tot_f) = (0.0, 0.0);
    for (shift, baud, prova, nome) in &casi {
        let mut riga = format!("{shift:>4} Hz {baud:>5} bd {nome:<13}|");
        let (mut media_a, mut media_f) = (0.0, 0.0);
        for (k, &snr) in SNRS.iter().enumerate() {
            let (mut ea, mut ef, mut n) = (0, 0, 0);
            for r in 0..RUNS {
                let (a, f, len) = una_prova(*shift, *baud, snr, *prova, 500 + r * 13 + k as u64);
                ea += a;
                ef += f;
                n += len;
            }
            let (pa, pf) = (100.0 * ea as f64 / n as f64, 100.0 * ef as f64 / n as f64);
            media_a += pa / SNRS.len() as f64;
            media_f += pf / SNRS.len() as f64;
            riga.push_str(&format!(" {pa:5.1}/{pf:5.1}"));
        }
        tot_a += media_a / casi.len() as f64;
        tot_f += media_f / casi.len() as f64;
        riga.push_str(&format!(" | media {media_a:5.1}/{media_f:5.1}"));
        println!("{riga}");
        if media_a > media_f + TOLLERANZA {
            peggiori.push(riga);
        }
    }
    println!("media di tutte le righe: Aurora {tot_a:.1}% · fldigi {tot_f:.1}%");
    assert!(peggiori.is_empty(), "righe dove Aurora sbaglia piu' di fldigi (oltre {TOLLERANZA} punti):\n{}", peggiori.join("\n"));
}
