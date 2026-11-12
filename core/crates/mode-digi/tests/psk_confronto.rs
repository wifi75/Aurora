//! Prova di confronto del ricevitore PSK31/PSK63: segnali fatti con il modulatore di Aurora, rumore bianco gaussiano a vari rapporti
//! segnale/rumore (misurati in 3 kHz come nella prova RTTY, in fldigi e in MMTTY) e i casi difficili dell'analisi `docs/analisi-psk-cw.md`:
//! errore di frequenza, mezzo minuto di solo rumore prima del segnale, deriva, fading CCIR, stazione vicina piu' forte, errore d'orologio
//! della scheda audio, livello molto basso. Misura il tasso di caratteri sbagliati (distanza di Levenshtein fra il testo mandato e quello
//! ricevuto, divisa per la lunghezza del testo mandato); nel conto entrano anche i caratteri inventati dal rumore prima e dopo il testo.
//!
//! Il ricevitore lavora con le impostazioni di base del servizio (AFC acceso, squelch spento). Prove lente, si lanciano a mano:
//! - `confronto_tasso_di_errore`: la tabella completa;
//! - `rumore_e_squelch`: un minuto di solo rumore, caratteri falsi con lo squelch spento e acceso e quanto si allontana l'AFC;
//! - `studio`: un punteggio solo su pochi casi difficili, per provare in fretta una variante.
//!
//! `cargo test --release -p mode-digi --test psk_confronto -- --ignored --nocapture --test-threads=1`
//! (con `PSK_DEBUG=1` stampa il testo mandato e quello ricevuto; con `AURORA_WAV=<cartella>` scrive i file WAV dei casi, da riascoltare
//! con fldigi o ).

mod common;

use common::{Rng, aggiungi_rumore, errore_orologio, fading_ccir, guadagno_complesso, levenshtein, salva_wav, sigma_per_snr, testo};
use mode_digi::psk::{self, PskDemodulator, PskMode};

const RATE: f64 = 12_000.0;
const AMP: f64 = 0.3;
const FREQ: f32 = 1000.0;

#[derive(Clone, Copy, Debug)]
enum Prova {
    /// Solo rumore bianco.
    Awgn,
    /// Il trasmettitore e' fuori di questi Hz (l'AFC deve agganciarlo).
    Fuori(f32),
    /// Trenta secondi di solo rumore prima del segnale (l'AFC non deve andarsene).
    Attesa,
    /// La portante scivola di questi Hz dall'inizio alla fine del testo.
    Deriva(f64),
    /// Fading CCIR «moderate»: due percorsi, 1 ms, 1 Hz di Doppler.
    Fading,
    /// Una stazione 6 dB piu' forte a 2,25 volte la velocita' di simbolo (70 Hz in PSK31).
    Vicina,
    /// La scheda audio sbaglia l'orologio di tante parti per milione.
    Orologio(f64),
    /// Segnale e rumore 40 dB piu' bassi (ingresso a basso volume).
    Livello,
}

/// Il ricevitore con le impostazioni di base del servizio.
fn ricevitore(mode: PskMode) -> PskDemodulator {
    let mut rx = PskDemodulator::new(mode, FREQ);
    rx.set_afc(true);
    rx
}

fn baud(mode: PskMode) -> f64 {
    RATE / mode.symbol_samples() as f64
}

/// Una prova: (errori, caratteri mandati).
fn una_prova(mode: PskMode, prova: Prova, snr_db: f64, seed: u64) -> (usize, usize) {
    let mut rng = Rng(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1);
    let sent = testo(&mut rng, 100).to_lowercase();
    let tx_freq = match prova {
        Prova::Fuori(hz) => FREQ + hz,
        _ => FREQ,
    };
    let mut sig = psk::modulate(sent.as_bytes(), tx_freq, mode, AMP as f32);
    match prova {
        Prova::Deriva(hz) => {
            let t_tot = sig.len() as f64 / RATE;
            sig = guadagno_complesso(&sig, |n| {
                let t = n as f64 / RATE;
                let ph = 2.0 * std::f64::consts::PI * hz * t * t / (2.0 * t_tot);
                (ph.cos(), ph.sin())
            });
        }
        Prova::Fading => sig = fading_ccir(&sig, 1.0, 1.0, &mut rng),
        Prova::Vicina => {
            let other = testo(&mut rng, 160).to_lowercase();
            let off = (2.25 * baud(mode)) as f32;
            let qrm = psk::modulate(other.as_bytes(), FREQ + off, mode, (2.0 * AMP) as f32);
            for (s, q) in sig.iter_mut().zip(qrm.iter()) {
                *s += *q;
            }
        }
        Prova::Orologio(ppm) => sig = errore_orologio(&sig, ppm),
        _ => {}
    }
    let pad_before = if matches!(prova, Prova::Attesa) { 30 * RATE as usize } else { RATE as usize };
    let mut audio = vec![0.0_f32; pad_before];
    audio.extend_from_slice(&sig);
    audio.extend(std::iter::repeat_n(0.0, RATE as usize));
    aggiungi_rumore(&mut audio, sigma_per_snr(AMP, snr_db), &mut rng);
    if matches!(prova, Prova::Livello) {
        for s in audio.iter_mut() {
            *s *= 0.01;
        }
    }
    salva_wav(&format!("psk_{}_{prova:?}_{snr_db}dB_{seed}", mode.name()), RATE as u32, &audio);
    let mut rx = ricevitore(mode);
    let mut got = Vec::new();
    for block in audio.chunks(1024) {
        got.extend(rx.process(block));
    }
    if std::env::var_os("PSK_DEBUG").is_some() {
        println!("TX {sent:?}\nRX {:?}", String::from_utf8_lossy(&got));
    }
    (levenshtein(sent.as_bytes(), &got), sent.len())
}

const SNRS: [f64; 7] = [5.0, 0.0, -5.0, -8.0, -10.0, -12.0, -15.0];

#[test]
#[ignore]
fn confronto_tasso_di_errore() {
    const RUNS: u64 = 2;
    let casi: [(Prova, &str); 9] = [
        (Prova::Awgn, "rumore"),
        (Prova::Fuori(4.0), "fuori +4 Hz"),
        (Prova::Fuori(-6.0), "fuori -6 Hz"),
        (Prova::Attesa, "30 s di rumore"),
        (Prova::Deriva(4.0), "deriva 4 Hz"),
        (Prova::Fading, "fading CCIR"),
        (Prova::Vicina, "vicina +6 dB"),
        (Prova::Orologio(200.0), "orologio 200ppm"),
        (Prova::Livello, "livello -40 dB"),
    ];
    for mode in [PskMode::Psk31, PskMode::Psk63] {
        let mut totali = vec![(0usize, 0usize); SNRS.len()];
        for (prova, nome) in casi {
            let mut riga = format!("{} {nome:<16}|", mode.name());
            for (k, &snr) in SNRS.iter().enumerate() {
                let (mut e, mut n) = (0, 0);
                for r in 0..RUNS {
                    let (ee, nn) = una_prova(mode, prova, snr, 500 + r * 31 + k as u64 * 7);
                    e += ee;
                    n += nn;
                }
                totali[k].0 += e;
                totali[k].1 += n;
                riga.push_str(&format!(" {:>6.1}", 100.0 * e as f64 / n as f64));
            }
            println!("{riga}");
        }
        let mut media = format!("{} {:<16}|", mode.name(), "media");
        for (e, n) in totali {
            media.push_str(&format!(" {:>6.1}", 100.0 * e as f64 / n as f64));
        }
        println!("{}", "-".repeat(media.len()));
        println!("{media}");
    }
    println!("(colonne: SNR in 3 kHz {SNRS:?} dB; valori: caratteri sbagliati in %)");
}

/// Un minuto di solo rumore (forte e debole): quanti caratteri si stampano con lo squelch spento e acceso, e di quanto si sposta l'AFC.
#[test]
#[ignore]
fn rumore_e_squelch() {
    for mode in [PskMode::Psk31, PskMode::Psk63] {
        for sigma in [0.3, 0.003] {
            for squelch in [false, true] {
                let mut rng = Rng(4242);
                let mut audio = vec![0.0_f32; 60 * RATE as usize];
                aggiungi_rumore(&mut audio, sigma, &mut rng);
                let mut rx = ricevitore(mode);
                if squelch {
                    rx.set_squelch_db(Some(-45.0));
                }
                let mut n = 0;
                let mut lontano = 0.0_f32;
                for block in audio.chunks(1024) {
                    n += rx.process(block).len();
                    lontano = lontano.max((rx.frequency() - FREQ).abs());
                }
                println!("{} rumore sigma {sigma:<5} squelch {:<6}: {n:>4} caratteri falsi, AFC fino a {lontano:.1} Hz", mode.name(), if squelch { "acceso" } else { "spento" });
            }
        }
    }
}

/// Un punteggio solo (errori medi su pochi casi difficili), per provare in fretta le varianti del ricevitore.
#[test]
#[ignore]
fn studio() {
    let casi: [(PskMode, Prova, f64); 8] = [
        (PskMode::Psk31, Prova::Awgn, -8.0),
        (PskMode::Psk31, Prova::Awgn, -10.0),
        (PskMode::Psk31, Prova::Awgn, -12.0),
        (PskMode::Psk31, Prova::Fading, -5.0),
        (PskMode::Psk31, Prova::Vicina, -5.0),
        (PskMode::Psk31, Prova::Attesa, -8.0),
        (PskMode::Psk31, Prova::Orologio(200.0), -8.0),
        (PskMode::Psk63, Prova::Awgn, -8.0),
    ];
    let mut riga = String::new();
    let mut tot = 0.0;
    for (mode, prova, snr) in casi {
        let (mut e, mut n) = (0, 0);
        for r in 0..4 {
            let (ee, nn) = una_prova(mode, prova, snr, 77 + r);
            e += ee;
            n += nn;
        }
        let cer = 100.0 * e as f64 / n as f64;
        tot += cer;
        riga.push_str(&format!(" {cer:5.1}"));
    }
    println!("STUDIO media {:5.1} |{riga}", tot / casi.len() as f64);
}
