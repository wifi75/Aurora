//! Prova di confronto del ricevitore RTTY: segnali fatti con il modulatore a vari spostamenti e velocita', con rumore bianco gaussiano a vari
//! rapporti segnale/rumore (misurati in 3 kHz, come fldigi e MMTTY), con un errore di frequenza, con un tono attenuato (fading selettivo o
//! filtro della radio inclinato) e con una stazione vicina. Misura il tasso di caratteri sbagliati (distanza di Levenshtein fra il testo
//! mandato e quello ricevuto, divisa per la lunghezza del testo mandato).
//!
//! Tre prove, lente, che si lanciano a mano:
//! - `confronto_tasso_di_errore`: la tabella completa (spostamenti 85/170/425/850 Hz, 45,45/50/75 baud, SNR da +3 a -12 dB, errore di
//!   frequenza, space attenuato, stazione vicina);
//! - `stretti`: due filtri contro discriminatore di frequenza sugli spostamenti stretti;
//! - `studio`: un punteggio solo su pochi casi difficili, per provare in fretta una variante del ricevitore.
//!
//! `cargo test --release -p mode-digi --test rtty_confronto -- --ignored --nocapture --test-threads=1`
//! (con `RTTY_DEBUG=1` stampa anche il testo mandato e quello ricevuto). Nel conto entrano anche i caratteri inventati dal rumore nel
//! secondo senza segnale prima e dopo il testo: per questo a SNR alto resta un 1-3%.

mod common;

use common::{Rng, levenshtein, testo};
use mode_digi::rtty::{self, RttyDemodulator, RttyParams};

const RATE: f64 = 12_000.0;

/// Parametri per un segnale centrato su `centre` (mark = tono alto, come in USB).
fn params(centre: f32, shift: f32, baud: f32) -> RttyParams {
    RttyParams { centre_hz: centre, shift_hz: shift, baud, reverse: false }
}

#[derive(Clone, Copy)]
enum Prova {
    /// Solo rumore bianco.
    Awgn,
    /// Il trasmettitore e' fuori di questi Hz (AFC spento).
    Offset(f32),
    /// Lo space arriva attenuato di questi dB (fading selettivo, filtro della radio inclinato).
    SpaceGiu(f64),
    /// Una stazione vicina della stessa forza a `shift + 150` Hz dal centro.
    Vicina,
}

const AMP: f64 = 0.3;

fn una_prova(shift: f32, baud: f32, snr_db: f64, prova: Prova, seed: u64) -> (usize, usize) {
    una_prova_con(shift, baud, snr_db, prova, seed, false)
}

/// `disc`: ricevitore a discriminatore di frequenza invece dei due filtri.
fn una_prova_con(shift: f32, baud: f32, snr_db: f64, prova: Prova, seed: u64, disc: bool) -> (usize, usize) {
    let mut rng = Rng(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1);
    let centre = 1500.0_f32;
    let sent = testo(&mut rng, 120);
    let tx_centre = match prova {
        Prova::Offset(hz) => centre + hz,
        _ => centre,
    };
    let mut sig = match prova {
        Prova::SpaceGiu(db) => rtty::modulate_levels(sent.as_bytes(), params(tx_centre, shift, baud), AMP as f32, (AMP * 10f64.powf(db / 20.0)) as f32),
        _ => rtty::modulate(sent.as_bytes(), params(tx_centre, shift, baud), AMP as f32),
    };
    if let Prova::Vicina = prova {
        let other = testo(&mut rng, 200);
        let qrm = rtty::modulate(other.as_bytes(), params(centre + shift + 150.0, shift, baud), AMP as f32);
        for (s, q) in sig.iter_mut().zip(qrm.iter()) {
            *s += *q;
        }
    }
    // un secondo di solo rumore prima e dopo
    let pad = RATE as usize;
    let mut audio = vec![0.0_f32; pad];
    audio.extend_from_slice(&sig);
    audio.extend(std::iter::repeat_n(0.0, pad));
    // SNR in 3 kHz: potenza del segnale A^2/2, rumore di varianza s^2 su 6 kHz -> s^2/2 in 3 kHz
    let sigma = AMP / 10f64.powf(snr_db / 20.0);
    for s in audio.iter_mut() {
        *s += (sigma * rng.gauss()) as f32;
    }
    let mut rx = RttyDemodulator::new(params(centre, shift, baud));
    rx.set_discriminator(disc);
    let mut got = Vec::new();
    for block in audio.chunks(1024) {
        got.extend(rx.process(block));
    }
    let got: Vec<u8> = got.into_iter().filter(|&c| c != b'\r' && c != b'\n').collect();
    if std::env::var_os("RTTY_DEBUG").is_some() {
        println!("TX {sent:?}\nRX {:?}", String::from_utf8_lossy(&got));
    }
    (levenshtein(sent.as_bytes(), &got), sent.len())
}

#[test]
#[ignore]
fn confronto_tasso_di_errore() {
    const SNRS: [f64; 6] = [3.0, 0.0, -3.0, -6.0, -9.0, -12.0];
    const RUNS: u64 = 3;
    let mut righe = Vec::new();
    let casi: Vec<(f32, f32, Prova, &str)> = {
        let mut v = Vec::new();
        for shift in [85.0, 170.0, 425.0, 850.0] {
            for baud in [45.45, 50.0, 75.0] {
                v.push((shift, baud, Prova::Awgn, "rumore"));
            }
        }
        for (shift, baud) in [(170.0, 45.45), (85.0, 45.45), (170.0, 75.0)] {
            v.push((shift, baud, Prova::Offset(5.0), "+5 Hz"));
            v.push((shift, baud, Prova::Offset(10.0), "+10 Hz"));
            v.push((shift, baud, Prova::SpaceGiu(-12.0), "space -12 dB"));
            v.push((shift, baud, Prova::Vicina, "vicina"));
        }
        v
    };
    let mut totali = vec![(0usize, 0usize); SNRS.len()];
    for (shift, baud, prova, nome) in casi {
        let mut riga = format!("{shift:>4} Hz {baud:>5} bd {nome:<13}|");
        for (k, &snr) in SNRS.iter().enumerate() {
            let (mut e, mut n) = (0, 0);
            for r in 0..RUNS {
                let (ee, nn) = una_prova(shift, baud, snr, prova, 1000 + r * 31 + k as u64 * 7 + shift as u64 + baud as u64);
                e += ee;
                n += nn;
            }
            totali[k].0 += e;
            totali[k].1 += n;
            riga.push_str(&format!(" {:>6.1}", 100.0 * e as f64 / n as f64));
        }
        println!("{riga}");
        righe.push(riga);
    }
    let mut media = format!("{:<27}|", "media");
    for (e, n) in totali {
        media.push_str(&format!(" {:>6.1}", 100.0 * e as f64 / n as f64));
    }
    println!("{}", "-".repeat(media.len()));
    println!("{media}");
    println!("(colonne: SNR in 3 kHz {SNRS:?} dB; valori: caratteri sbagliati in %)");
}

/// Un punteggio solo (errori medi su pochi casi difficili), per provare in fretta le varianti del ricevitore.
#[test]
#[ignore]
fn studio() {
    let casi: [(f32, f32, Prova, f64); 9] = [
        (170.0, 45.45, Prova::Awgn, -6.0),
        (170.0, 45.45, Prova::Awgn, -9.0),
        (170.0, 75.0, Prova::Awgn, -6.0),
        (850.0, 45.45, Prova::Awgn, -9.0),
        (170.0, 45.45, Prova::SpaceGiu(-12.0), 0.0),
        (170.0, 45.45, Prova::SpaceGiu(-12.0), -3.0),
        (85.0, 45.45, Prova::Offset(10.0), -6.0),
        (170.0, 45.45, Prova::Vicina, -6.0),
        (170.0, 45.45, Prova::Awgn, 10.0),
    ];
    let mut riga = String::new();
    let mut tot = 0.0;
    for (shift, baud, prova, snr) in casi {
        let (mut e, mut n) = (0, 0);
        for r in 0..6 {
            let (ee, nn) = una_prova(shift, baud, snr, prova, 77 + r);
            e += ee;
            n += nn;
        }
        let cer = 100.0 * e as f64 / n as f64;
        tot += cer;
        riga.push_str(&format!(" {cer:5.1}"));
    }
    println!("STUDIO media {:5.1} |{riga}", tot / casi.len() as f64);
}

/// Spostamenti stretti: due filtri (F) contro discriminatore di frequenza (D), 45,45 baud, solo rumore.
#[test]
#[ignore]
fn stretti() {
    const SNRS: [f64; 5] = [10.0, 3.0, 0.0, -3.0, -6.0];
    for shift in [23.0, 50.0, 85.0, 170.0] {
        for disc in [false, true] {
            let mut riga = format!("{shift:>4} Hz {} |", if disc { "D" } else { "F" });
            for snr in SNRS {
                let (mut e, mut n) = (0, 0);
                for r in 0..4 {
                    let (ee, nn) = una_prova_con(shift, 45.45, snr, Prova::Awgn, 300 + r, disc);
                    e += ee;
                    n += nn;
                }
                riga.push_str(&format!(" {:5.1}", 100.0 * e as f64 / n as f64));
            }
            println!("{riga}");
        }
    }
    println!("(colonne: SNR in 3 kHz {SNRS:?} dB; valori: caratteri sbagliati in %)");
}
