//! Prova di confronto del ricevitore CW: Morse generato qui (con i tempi esatti o con quelli irregolari di un tasto a mano), rumore bianco
//! gaussiano a vari rapporti segnale/rumore (misurati in 3 kHz, come nelle prove RTTY e PSK) e i casi difficili dell'analisi
//! `docs/analisi-psk-cw.md`: tasto a mano, salto di velocita', QSB, QRN (scariche), QRM (un'altra stazione CW vicina), tono spostato, livello
//! molto basso, segnale forte altrove nella banda. Misura il tasso di caratteri sbagliati (Levenshtein / lunghezza del testo mandato) e la
//! velocita' capita dal ricevitore.
//!
//! Prove lente, si lanciano a mano:
//! - `confronto_tasso_di_errore`: la tabella completa;
//! - `rumore_e_squelch`: un minuto di solo rumore, caratteri falsi con lo squelch spento e acceso;
//! - `studio`: un punteggio solo su pochi casi difficili;
//! - `contro_fldigi` (con la feature `fldigi-oracle`): la stessa tabella con il ricevitore di fldigi 4.2.13 accanto al nostro.
//!
//! `cargo test --release -p mode-digi --test cw_confronto -- --ignored --nocapture --test-threads=1`
//! (con `CW_DEBUG=1` stampa il testo mandato e quello ricevuto; con `AURORA_WAV=<cartella>` scrive i file WAV dei casi).

mod common;

use common::{Rng, aggiungi_rumore, levenshtein, salva_wav, sigma_per_snr, testo};
use mode_digi::cw::CwDemodulator;
use std::f64::consts::PI;

const RATE: f64 = 12_000.0;
/// Di quanti punti percentuali (media di una riga) il nostro ricevitore puo' sbagliare piu' di fldigi nella prova `contro_fldigi`.
#[cfg(feature = "fldigi-oracle")]
const TOLLERANZA: f64 = 2.0;
const AMP: f64 = 0.3;
const TONO: f32 = 700.0;
const WPM: f64 = 20.0;

/// Le lettere e le cifre del testo di prova (lo stesso Morse internazionale del modulatore).
fn morse(c: u8) -> Option<&'static str> {
    const T: [(u8, &str); 36] = [
        (b'A', ".-"),
        (b'B', "-..."),
        (b'C', "-.-."),
        (b'D', "-.."),
        (b'E', "."),
        (b'F', "..-."),
        (b'G', "--."),
        (b'H', "...."),
        (b'I', ".."),
        (b'J', ".---"),
        (b'K', "-.-"),
        (b'L', ".-.."),
        (b'M', "--"),
        (b'N', "-."),
        (b'O', "---"),
        (b'P', ".--."),
        (b'Q', "--.-"),
        (b'R', ".-."),
        (b'S', "..."),
        (b'T', "-"),
        (b'U', "..-"),
        (b'V', "...-"),
        (b'W', ".--"),
        (b'X', "-..-"),
        (b'Y', "-.--"),
        (b'Z', "--.."),
        (b'0', "-----"),
        (b'1', ".----"),
        (b'2', "..---"),
        (b'3', "...--"),
        (b'4', "....-"),
        (b'5', "....."),
        (b'6', "-...."),
        (b'7', "--..."),
        (b'8', "---.."),
        (b'9', "----."),
    ];
    let up = c.to_ascii_uppercase();
    T.iter().find(|(ch, _)| *ch == up).map(|(_, p)| *p)
}

/// Come si manipola: velocita' (WPM) per carattere e irregolarita' del tasto a mano (0 = keyer elettronico).
struct Mano {
    /// Deviazione relativa della durata di ogni elemento e pausa.
    jitter: f64,
    /// Rapporto linea/punto (3 nel Morse esatto, 3,3-3,8 con molti tasti a mano).
    dash: f64,
    /// Pausa fra le lettere e fra le parole in unita' (3 e 7 esatti).
    letter_gap: f64,
    word_gap: f64,
}

const KEYER: Mano = Mano { jitter: 0.0, dash: 3.0, letter_gap: 3.0, word_gap: 7.0 };
const TASTO: Mano = Mano { jitter: 0.15, dash: 3.4, letter_gap: 3.6, word_gap: 6.0 };

/// Sequenza (acceso?, secondi) del testo; `wpm(i)` e' la velocita' dell'i-esimo carattere.
fn chiavi(text: &str, wpm: impl Fn(usize) -> f64, m: &Mano, rng: &mut Rng) -> Vec<(bool, f64)> {
    let mut seq = Vec::new();
    let j = |x: f64, rng: &mut Rng| (x * (1.0 + m.jitter * rng.gauss())).max(0.3 * x);
    for (i, c) in text.bytes().enumerate() {
        let unit = 1.2 / wpm(i);
        if c == b' ' {
            // la pausa dopo l'ultima lettera vale gia' letter_gap unita'
            seq.push((false, j((m.word_gap - m.letter_gap) * unit, rng)));
            continue;
        }
        let Some(p) = morse(c) else { continue };
        for (k, e) in p.bytes().enumerate() {
            if k > 0 {
                seq.push((false, j(unit, rng)));
            }
            seq.push((true, j(if e == b'-' { m.dash } else { 1.0 } * unit, rng)));
        }
        seq.push((false, j(m.letter_gap * unit, rng)));
    }
    seq
}

/// L'audio della sequenza: tono con fronti a coseno di 5 ms; `amp(t)` e' l'ampiezza nel tempo (QSB).
fn suona(seq: &[(bool, f64)], tone: f64, amp: impl Fn(f64) -> f64) -> Vec<f32> {
    let ramp = 0.005 * RATE;
    let mut out = Vec::new();
    let mut t_end = 0.0;
    for &(on, secs) in seq {
        let start = out.len();
        t_end += secs * RATE;
        let len = (t_end.round() as usize).saturating_sub(start);
        for i in 0..len {
            let n = start + i;
            if !on {
                out.push(0.0);
                continue;
            }
            let edge = i.min(len - 1 - i) as f64;
            let w = if edge < ramp { 0.5 * (1.0 - (PI * edge / ramp).cos()) } else { 1.0 };
            let t = n as f64 / RATE;
            out.push((amp(t) * w * (2.0 * PI * tone * t).sin()) as f32);
        }
    }
    out
}

#[derive(Clone, Copy, Debug)]
enum Prova {
    /// Keyer elettronico a 20 WPM, solo rumore.
    Awgn,
    /// Tasto a mano: durate irregolari, linee lunghe, pause corte fra le parole.
    Mano,
    /// La prima meta' a 14 WPM, la seconda a 28 (il ricevitore parte da 20).
    Salto,
    /// QSB: l'ampiezza oscilla di 15 dB ogni 8 secondi.
    Qsb,
    /// QRN: scariche brevi e forti, in media 4 al secondo.
    Qrn,
    /// QRM: un'altra stazione CW della stessa forza a +150 Hz, a 25 WPM.
    Qrm,
    /// Il tono e' 25 Hz sopra la frequenza di ascolto.
    Spostato,
    /// Segnale e rumore 40 dB piu' bassi (ingresso a basso volume).
    Livello,
    /// Un segnale 20 dB piu' forte a 1500 Hz (fuori dalla banda del CW).
    ForteAltrove,
}

/// Il testo mandato e l'audio (12 kHz) di una prova.
fn audio_di_prova(prova: Prova, snr_db: f64, seed: u64) -> (String, Vec<f32>) {
    let mut rng = Rng(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1);
    let sent = testo(&mut rng, 60).trim_end().to_string();
    let n_chars = sent.len();
    let mano = if matches!(prova, Prova::Mano) { &TASTO } else { &KEYER };
    let seq = chiavi(&sent, |i| if matches!(prova, Prova::Salto) { if i < n_chars / 2 { 14.0 } else { 28.0 } } else { WPM }, mano, &mut rng);
    let tone = f64::from(TONO) + if matches!(prova, Prova::Spostato) { 25.0 } else { 0.0 };
    let mut sig = match prova {
        Prova::Qsb => suona(&seq, tone, |t| AMP * 10f64.powf(-15.0 / 20.0 * 0.5 * (1.0 - (2.0 * PI * t / 8.0).cos()))),
        _ => suona(&seq, tone, |_| AMP),
    };
    match prova {
        Prova::Qrm => {
            let other = testo(&mut rng, 80);
            let qrm = suona(&chiavi(&other, |_| 25.0, &KEYER, &mut rng), tone + 150.0, |_| AMP);
            for (s, q) in sig.iter_mut().zip(qrm.iter()) {
                *s += *q;
            }
        }
        Prova::ForteAltrove => {
            let other = testo(&mut rng, 80);
            let strong = suona(&chiavi(&other, |_| 22.0, &KEYER, &mut rng), 1500.0, |_| 10.0 * AMP);
            for (s, q) in sig.iter_mut().zip(strong.iter()) {
                *s += *q;
            }
        }
        _ => {}
    }
    let pad = RATE as usize;
    let mut audio = vec![0.0_f32; pad];
    audio.extend_from_slice(&sig);
    audio.extend(std::iter::repeat_n(0.0, 2 * pad));
    let sigma = sigma_per_snr(AMP, snr_db);
    aggiungi_rumore(&mut audio, sigma, &mut rng);
    if let Prova::Qrn = prova {
        // scariche di 1-3 ms, 15 volte la deviazione del rumore
        let mut n = 0usize;
        while n < audio.len() {
            n += (-(rng.uniform().ln()) * RATE / 4.0) as usize + 1;
            let len = 12 + (rng.next_u64() % 24) as usize;
            for k in n..(n + len).min(audio.len()) {
                audio[k] += (15.0 * sigma.max(0.02) * rng.gauss()) as f32;
            }
        }
    }
    if matches!(prova, Prova::Livello) {
        for s in audio.iter_mut() {
            *s *= 0.01;
        }
    }
    salva_wav(&format!("cw_{prova:?}_{snr_db}dB_{seed}"), RATE as u32, &audio);
    (sent, audio)
}

fn errori(chi: &str, sent: &str, got: &[u8]) -> usize {
    let got = String::from_utf8_lossy(got).trim().to_string();
    if std::env::var_os("CW_DEBUG").is_some() {
        println!("TX        {sent:?}
RX {chi:<6} {got:?}");
    }
    levenshtein(sent.as_bytes(), got.as_bytes())
}

/// Una prova con il ricevitore di Aurora: (errori, caratteri mandati, velocita' capita alla fine).
fn una_prova(prova: Prova, snr_db: f64, seed: u64) -> (usize, usize, f32) {
    let (sent, audio) = audio_di_prova(prova, snr_db, seed);
    let mut rx = ricevitore();
    let mut got = Vec::new();
    for block in audio.chunks(480) {
        got.extend(rx.process(block));
    }
    (errori("Aurora", &sent, &got), sent.len(), rx.wpm())
}

/// La stessa prova con il ricevitore di fldigi (150 Hz di banda, 20 WPM impostati con l'inseguimento acceso, squelch spento):
/// (errori Aurora, errori fldigi, caratteri, WPM Aurora, WPM fldigi).
#[cfg(feature = "fldigi-oracle")]
fn prova_contro_fldigi(prova: Prova, snr_db: f64, seed: u64) -> (usize, usize, usize, f32, f64) {
    let (sent, audio) = audio_di_prova(prova, snr_db, seed);
    let mut rx = ricevitore();
    let mut got = Vec::new();
    for block in audio.chunks(480) {
        got.extend(rx.process(block));
    }
    let ours = errori("Aurora", &sent, &got);
    let mut fl = mode_digi::fldigi_oracle::FldigiCw::new(f64::from(TONO), WPM, 150.0, 8000.0, None);
    let mut got = Vec::new();
    for block in common::a_8k(&audio).chunks(512) {
        got.extend(fl.process(block));
    }
    (ours, errori("fldigi", &sent, &got), sent.len(), rx.wpm(), fl.wpm())
}

/// Il ricevitore con le impostazioni di base del servizio (20 WPM di partenza, squelch spento).
fn ricevitore() -> CwDemodulator {
    CwDemodulator::new(TONO, WPM as f32)
}

const SNRS: [f64; 7] = [5.0, 0.0, -4.0, -8.0, -10.0, -12.0, -15.0];

const CASI: [(Prova, &str); 9] = [
    (Prova::Awgn, "rumore"),
    (Prova::Mano, "tasto a mano"),
    (Prova::Salto, "14 -> 28 WPM"),
    (Prova::Qsb, "QSB 15 dB"),
    (Prova::Qrn, "QRN"),
    (Prova::Qrm, "QRM +150 Hz"),
    (Prova::Spostato, "tono +25 Hz"),
    (Prova::Livello, "livello -40 dB"),
    (Prova::ForteAltrove, "forte a 1500 Hz"),
];

#[test]
#[ignore]
fn confronto_tasso_di_errore() {
    const RUNS: u64 = 2;
    let mut totali = vec![(0usize, 0usize); SNRS.len()];
    for (prova, nome) in CASI {
        let mut riga = format!("{nome:<16}|");
        let mut wpm = Vec::new();
        for (k, &snr) in SNRS.iter().enumerate() {
            let (mut e, mut n) = (0, 0);
            for r in 0..RUNS {
                let (ee, nn, w) = una_prova(prova, snr, 900 + r * 31 + k as u64 * 7);
                e += ee;
                n += nn;
                if r == 0 {
                    wpm.push(w);
                }
            }
            totali[k].0 += e;
            totali[k].1 += n;
            riga.push_str(&format!(" {:>6.1}", 100.0 * e as f64 / n as f64));
        }
        riga.push_str(&format!("  | WPM capiti {}", wpm.iter().map(|w| format!("{w:.0}")).collect::<Vec<_>>().join(" ")));
        println!("{riga}");
    }
    let mut media = format!("{:<16}|", "media");
    for (e, n) in totali {
        media.push_str(&format!(" {:>6.1}", 100.0 * e as f64 / n as f64));
    }
    println!("{}", "-".repeat(media.len()));
    println!("{media}");
    println!("(colonne: SNR in 3 kHz {SNRS:?} dB; valori: caratteri sbagliati in %)");
}

/// La stessa tabella con fldigi 4.2.13 come arbitro: ogni casella e' «Aurora/fldigi» (caratteri sbagliati in %). Solo con la feature
/// `fldigi-oracle`: `cargo test --release -p mode-digi --features fldigi-oracle --test cw_confronto contro_fldigi -- --ignored --nocapture`.
#[cfg(feature = "fldigi-oracle")]
#[test]
#[ignore]
fn contro_fldigi() {
    const RUNS: u64 = 2;
    let mut peggiori = Vec::new();
    let (mut tot_a, mut tot_f) = (0.0, 0.0);
    for (prova, nome) in CASI {
        let mut riga = format!("{nome:<16}|");
        let (mut media_a, mut media_f) = (0.0, 0.0);
        let mut wpm = String::new();
        for (k, &snr) in SNRS.iter().enumerate() {
            let (mut ea, mut ef, mut n) = (0, 0, 0);
            for r in 0..RUNS {
                let (a, f, len, wa, wf) = prova_contro_fldigi(prova, snr, 900 + r * 31 + k as u64 * 7);
                ea += a;
                ef += f;
                n += len;
                if r == 0 {
                    wpm.push_str(&format!(" {wa:.0}/{wf:.0}"));
                }
            }
            let (pa, pf) = (100.0 * ea as f64 / n as f64, 100.0 * ef as f64 / n as f64);
            media_a += pa / SNRS.len() as f64;
            media_f += pf / SNRS.len() as f64;
            riga.push_str(&format!(" {pa:5.1}/{pf:5.1}"));
        }
        tot_a += media_a / CASI.len() as f64;
        tot_f += media_f / CASI.len() as f64;
        println!("{riga} | media {media_a:5.1}/{media_f:5.1} | WPM{wpm}");
        if media_a > media_f + TOLLERANZA {
            peggiori.push(riga);
        }
    }
    println!("media di tutte le righe: Aurora {tot_a:.1}% · fldigi {tot_f:.1}%");
    println!("(colonne: SNR in 3 kHz {SNRS:?} dB; caselle: Aurora/fldigi, caratteri sbagliati in %)");
    assert!(peggiori.is_empty(), "righe dove Aurora sbaglia piu' di fldigi (oltre {TOLLERANZA} punti di media):
{}", peggiori.join("
"));
}

/// Un minuto di solo rumore (forte e debole): quanti caratteri si stampano con lo squelch spento e acceso.
#[test]
#[ignore]
fn rumore_e_squelch() {
    for sigma in [0.3, 0.003] {
        for squelch in [false, true] {
            let mut rng = Rng(4243);
            let mut audio = vec![0.0_f32; 60 * RATE as usize];
            aggiungi_rumore(&mut audio, sigma, &mut rng);
            let mut rx = ricevitore();
            if squelch {
                rx.set_squelch_snr_db(Some(mode_digi::cw::SQUELCH_DEFAULT_DB));
            }
            let mut n = 0;
            for block in audio.chunks(480) {
                n += rx.process(block).len();
            }
            println!("CW rumore sigma {sigma:<5} squelch {:<6}: {n:>4} caratteri falsi", if squelch { "acceso" } else { "spento" });
        }
    }
}

/// Un punteggio solo (errori medi su pochi casi difficili), per provare in fretta le varianti del ricevitore.
#[test]
#[ignore]
fn studio() {
    let casi: [(Prova, f64); 9] = [
        (Prova::Awgn, -8.0),
        (Prova::Awgn, -10.0),
        (Prova::Awgn, -12.0),
        (Prova::Mano, -8.0),
        (Prova::Salto, -8.0),
        (Prova::Qsb, 0.0),
        (Prova::Qrn, -8.0),
        (Prova::Qrm, -8.0),
        (Prova::Spostato, -8.0),
    ];
    let mut riga = String::new();
    let mut tot = 0.0;
    for (prova, snr) in casi {
        let (mut e, mut n) = (0, 0);
        for r in 0..4 {
            let (ee, nn, _) = una_prova(prova, snr, 77 + r);
            e += ee;
            n += nn;
        }
        let cer = 100.0 * e as f64 / n as f64;
        tot += cer;
        riga.push_str(&format!(" {cer:5.1}"));
    }
    println!("STUDIO media {:5.1} |{riga}", tot / casi.len() as f64);
}

/// Tutta la banda: dieci stazioni CW fra 400 e 2300 Hz a -5 dB ognuna, un minuto. Stampa canali, nominativi verificati e quanto piu'
/// in fretta del tempo vero lavora la decodifica di tutta la banda.
#[test]
#[ignore]
fn tutta_la_banda() {
    use mode_digi::skim::CwBandDecoder;
    let mut rng = Rng(77);
    let n = 60 * RATE as usize;
    let mut band = vec![0.0_f32; n];
    let calls = ["IU3ABC", "K1ABC", "DL1ABC", "G4ABC", "JA1ABC", "VK2ABC", "F5ABC", "EA3ABC", "OH2ABC", "W1AW"];
    for (i, call) in calls.iter().enumerate() {
        let hz = 400.0 + 200.0 * i as f64 + 13.0 * (i % 3) as f64;
        let wpm = 16.0 + 2.0 * i as f64;
        let text = format!("CQ CQ DE {call} {call} K   CQ CQ DE {call} {call} K   CQ DE {call} {call} K");
        let seq = chiavi(&text, |_| wpm, &KEYER, &mut rng);
        let wave = suona(&seq, hz, |_| AMP);
        for (s, w) in band.iter_mut().skip(6_000 + 3_000 * i).zip(wave) {
            *s += w;
        }
    }
    aggiungi_rumore(&mut band, sigma_per_snr(AMP, -5.0), &mut rng);
    let mut sk = CwBandDecoder::new(300.0, 2_600.0, 40);
    let t0 = std::time::Instant::now();
    for block in band.chunks(4_800) {
        sk.process(block);
    }
    let secs = t0.elapsed().as_secs_f64();
    let chans = sk.channels();
    let mut found = 0;
    for ch in &chans {
        let verified: Vec<String> = ch.calls.iter().filter(|c| c.verified()).map(|c| c.call.clone()).collect();
        found += calls.iter().filter(|c| verified.iter().any(|v| v == *c)).count();
        let tail: String = ch.text.chars().rev().take(60).collect::<Vec<char>>().into_iter().rev().collect();
        println!("{:7.1} Hz {:4.0} WPM S/N {:5.1} | {} | {tail}", ch.freq_hz, ch.wpm, ch.snr_db, verified.join(" "));
    }
    println!("canali {} · nominativi verificati {found}/10 · tempo vero / tempo di calcolo = {:.1}", chans.len(), 60.0 / secs);
}

/// CPU di tutta la banda: 10 e 30 stazioni CW fra 300 e 3700 Hz a -5 dB, un minuto ciascuna prova, con il limite di canali del servizio
/// (40). Stampa canali aperti, nominativi verificati e la CPU in percento di un core (tempo di calcolo diviso per il tempo dell'audio).
#[test]
#[ignore]
fn cpu_tutta_la_banda() {
    use mode_digi::skim::CwBandDecoder;
    for stations in [10_usize, 30] {
        let mut rng = Rng(91 + stations as u64);
        let n = 60 * RATE as usize;
        let mut band = vec![0.0_f32; n];
        let step = 3_400.0 / stations as f64;
        let calls: Vec<String> = (0..stations).map(|i| format!("IU{}A{}", i % 10, ["BC", "BD", "BE", "BF"][i / 10])).collect();
        for (i, call) in calls.iter().enumerate() {
            let hz = 320.0 + step * i as f64 + 7.0 * (i % 3) as f64;
            let wpm = 16.0 + (i % 8) as f64 * 2.0;
            let text = format!("CQ CQ DE {call} {call} K   CQ CQ DE {call} {call} K   CQ DE {call} {call} K");
            let seq = chiavi(&text, |_| wpm, &KEYER, &mut rng);
            let wave = suona(&seq, hz, |_| AMP);
            for (s, w) in band.iter_mut().skip(6_000 + 1_000 * i).zip(wave) {
                *s += w;
            }
        }
        aggiungi_rumore(&mut band, sigma_per_snr(AMP, -5.0), &mut rng);
        let mut sk = CwBandDecoder::new(150.0, 4_000.0, 40);
        let t0 = std::time::Instant::now();
        for block in band.chunks(1_200) {
            sk.process(block);
        }
        let secs = t0.elapsed().as_secs_f64();
        let chans = sk.channels();
        let found = calls.iter().filter(|c| chans.iter().any(|ch| ch.calls.iter().any(|h| h.verified() && &h.call == *c))).count();
        if std::env::var_os("CW_DEBUG").is_some() {
            for ch in &chans {
                println!("{:7.1} Hz {:4.0} WPM {:5.1} dB {:?}", ch.freq_hz, ch.wpm, ch.snr_db, ch.calls.iter().map(|c| (c.call.clone(), c.verified())).collect::<Vec<_>>());
            }
        }
        println!("{stations} stazioni: canali {} · nominativi verificati {found}/{stations} · CPU {:.1}% di un core", chans.len(), 100.0 * secs / 60.0);
    }
}
