//! Misure del decoder FT8 su periodi sintetici (generatore deterministico, nessun audio reale).
//!
//! Sono prove lente e per questo `#[ignore]`: vanno lanciate in release e con `--nocapture`, per esempio
//! `cargo test -p ft8 --release -j 2 --target-dir target/agente-dec --test misure -- --ignored --nocapture`.
//! Ogni prova stampa una tabella di numeri (messaggi decodificati, falsi, tempi): sono i numeri prima/dopo riportati in
//! `docs/analisi-decodium-decoder.md`. I periodi sono sempre gli stessi (semi fissi), quindi le colonne sono confrontabili.

use std::f64::consts::PI;
use std::sync::Mutex;
use std::time::Instant;

use ft8::decode::{Decoded, Options, Subtraction, decode_slot};
use ft8::message::CallHashes;
use ft8::modem;
use ft8::{SAMPLE_RATE, SLOT_SAMPLES};

/// Generatore pseudocasuale deterministico.
struct Rng(u64);
impl Rng {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
    fn uniform(&mut self) -> f64 {
        ((self.next() >> 11) as f64 + 0.5) / (1u64 << 53) as f64
    }
    fn range(&mut self, lo: f64, hi: f64) -> f64 {
        lo + (hi - lo) * self.uniform()
    }
    fn below(&mut self, n: usize) -> usize {
        (self.next() % n as u64) as usize
    }
    fn gauss(&mut self) -> f64 {
        (-2.0 * self.uniform().ln()).sqrt() * (2.0 * PI * self.uniform()).cos()
    }
}

/// Ampiezza del rumore bianco (normalizzata a +-1).
const SIGMA: f64 = 0.05;

#[derive(Clone, Debug)]
struct Signal {
    text: String,
    freq: f32,
    dt: f32,
    snr: f32,
    /// Profondita' dell'evanescenza (0 = nessuna).
    fade: f32,
}

fn call(rng: &mut Rng) -> String {
    const PREFIXES: [&str; 20] = ["K", "W", "N", "DL", "F", "G", "I", "IU", "JA", "VK", "PA", "SP", "OK", "EA", "LA", "OH", "SM", "UA", "BG", "LU"];
    let mut s = String::from(PREFIXES[rng.below(PREFIXES.len())]);
    s.push(char::from(b'0' + rng.below(10) as u8));
    for _ in 0..2 + rng.below(2) {
        s.push(char::from(b'A' + rng.below(26) as u8));
    }
    s
}

fn grid(rng: &mut Rng) -> String {
    format!("{}{}{}{}", char::from(b'A' + rng.below(18) as u8), char::from(b'A' + rng.below(18) as u8), rng.below(10), rng.below(10))
}

fn message(rng: &mut Rng) -> String {
    let (a, b) = (call(rng), call(rng));
    let report = rng.below(30) as i32 - 24;
    match rng.below(20) {
        0..=4 => format!("CQ {a} {}", grid(rng)),
        5..=7 => format!("{a} {b} {}", grid(rng)),
        8..=12 => format!("{a} {b} {report:+03}"),
        13..=16 => format!("{a} {b} R{report:+03}"),
        17..=18 => format!("{a} {b} RR73"),
        _ => format!("{a} {b} 73"),
    }
}

/// Un periodo: rumore bianco piu' i segnali (con evanescenza lenta se `fade > 0`).
fn render(signals: &[Signal], seed: u64) -> Vec<f32> {
    let mut rng = Rng(seed ^ 0xA5A5_5A5A);
    let mut audio: Vec<f32> = (0..SLOT_SAMPLES).map(|_| (SIGMA * rng.gauss()) as f32).collect();
    let mut hashes = CallHashes::default();
    let noise_2500 = SIGMA * SIGMA * 2500.0 / 6000.0;
    for s in signals {
        let wave = modem::encode_message(&s.text, s.freq, &mut hashes).expect("messaggio di prova valido");
        let amplitude = (2.0 * noise_2500 * 10f64.powf(f64::from(s.snr) / 10.0)).sqrt();
        let start = ((f64::from(s.dt) + 0.5) * SAMPLE_RATE as f64).round() as usize;
        let (fade_hz, phase) = (rng.range(0.15, 0.6), rng.range(0.0, 2.0 * PI));
        for (i, &w) in wave.iter().enumerate() {
            if let Some(a) = audio.get_mut(start + i) {
                let t = i as f64 / SAMPLE_RATE as f64;
                let gain = 1.0 + f64::from(s.fade) * (2.0 * PI * fade_hz * t + phase).sin();
                *a += (amplitude * gain) as f32 * w;
            }
        }
    }
    audio
}

/// Il periodo `k` dell'insieme principale: da 5 a 15 segnali, SNR da -24 a +6 dB, un terzo con evanescenza.
fn main_period(k: u64) -> (Vec<Signal>, Vec<f32>) {
    let mut rng = Rng(1000 + k);
    let n = 5 + rng.below(11);
    let mut signals: Vec<Signal> = Vec::new();
    while signals.len() < n {
        let freq = rng.range(300.0, 2800.0) as f32;
        if signals.iter().any(|s| (s.freq - freq).abs() < 9.0) {
            continue;
        }
        let snr = rng.range(-24.0, 6.0) as f32;
        let fade = if rng.below(3) == 0 { 0.25 } else { 0.0 };
        signals.push(Signal { text: message(&mut rng), freq, dt: rng.range(-0.3, 1.0) as f32, snr, fade });
    }
    let audio = render(&signals, 1000 + k);
    (signals, audio)
}

/// Un periodo con un segnale forte e uno debole vicino (`distance` Hz sopra), stesso ritardo (a meno di 0,2 s).
fn pair_period(k: u64, distance: f32, strong_snr: f32, weak_snr: f32) -> (Vec<Signal>, Vec<f32>) {
    let mut rng = Rng(5000 + k);
    let f = rng.range(600.0, 2000.0) as f32;
    let strong = Signal { text: message(&mut rng), freq: f, dt: rng.range(0.0, 0.5) as f32, snr: strong_snr, fade: 0.2 };
    let weak = Signal { text: message(&mut rng), freq: f + distance, dt: rng.range(0.0, 0.5) as f32, snr: weak_snr, fade: 0.0 };
    let signals = vec![strong, weak];
    let audio = render(&signals, 5000 + k);
    (signals, audio)
}

struct Score {
    sent: usize,
    found: usize,
    false_: usize,
    millis: f64,
    periods: usize,
}

fn is_true(sent: &[Signal], d: &Decoded) -> bool {
    sent.iter().any(|s| s.text == d.text && (s.freq - d.freq_hz).abs() < 3.0)
}

/// Decodifica un insieme di periodi (due thread) e conta messaggi veri e falsi.
fn run<F: Fn(u64) -> (Vec<Signal>, Vec<f32>) + Sync>(count: u64, make: F, options: &Options) -> Score {
    let total = Mutex::new(Score { sent: 0, found: 0, false_: 0, millis: 0.0, periods: 0 });
    let next = Mutex::new(0u64);
    std::thread::scope(|scope| {
        for _ in 0..2 {
            scope.spawn(|| {
                loop {
                    let k = {
                        let mut n = next.lock().unwrap();
                        if *n >= count {
                            return;
                        }
                        *n += 1;
                        *n - 1
                    };
                    let (signals, audio) = make(k);
                    let t0 = Instant::now();
                    let out = decode_slot(&audio, options, &mut CallHashes::default());
                    let ms = t0.elapsed().as_secs_f64() * 1000.0;
                    let mut seen: Vec<&str> = Vec::new();
                    let (mut found, mut fals) = (0, 0);
                    for d in &out {
                        if is_true(&signals, d) {
                            if !seen.contains(&d.text.as_str()) {
                                seen.push(&d.text);
                                found += 1;
                            }
                        } else {
                            fals += 1;
                            eprintln!("  falso: {:?} (periodo {k})", d);
                        }
                    }
                    let mut t = total.lock().unwrap();
                    t.sent += signals.len();
                    t.found += found;
                    t.false_ += fals;
                    t.millis += ms;
                    t.periods += 1;
                }
            });
        }
    });
    total.into_inner().unwrap()
}

fn report(label: &str, s: &Score) {
    println!(
        "{label:<46} messaggi {:>4}/{:<4} ({:>5.1}%)  falsi {:>3}  {:>7.0} ms/periodo",
        s.found,
        s.sent,
        100.0 * s.found as f64 / s.sent.max(1) as f64,
        s.false_,
        s.millis / s.periods.max(1) as f64
    );
}

/// Le varianti da confrontare: si aggiungono qui quando il decoder ne ha di nuove.
fn variants() -> Vec<(&'static str, Options)> {
    // il decoder com'era prima di questo lavoro, ricostruito con le opzioni: 3 passate, ampiezza costante, a fine passata, senza filtri
    let old = Options {
        passes: 3,
        osd_order: 2,
        subtraction: Subtraction::Constant,
        immediate_subtraction: false,
        filters: false,
        ap_cq: false,
        ..Options::depth(1)
    };
    vec![
        ("A. come prima (3 passate, ampiezza costante)", old.clone()),
        ("B. A + sottrazione adattiva, a fine passata", Options { subtraction: Subtraction::Adaptive, ..old.clone() }),
        ("C. B + sottrazione subito per ogni segnale", Options { subtraction: Subtraction::Adaptive, immediate_subtraction: true, ..old.clone() }),
        ("D. C + filtri sui falsi", Options { subtraction: Subtraction::Adaptive, immediate_subtraction: true, filters: true, ..old }),
        ("E. profondita' 1 (3 passate, OSD 2)", Options::depth(1)),
        ("F. profondita' 2 (6 passate, OSD 3, AP CQ)", Options::depth(2)),
        ("G. profondita' 3 (9 passate, soglia bassa)", Options::depth(3)),
        ("H. profondita' 2 senza filtri", Options { filters: false, ..Options::depth(2) }),
        ("I. profondita' 3 senza filtri", Options { filters: false, ..Options::depth(3) }),
    ]
}

#[test]
#[ignore]
fn insieme_principale() {
    println!("\n== 30 periodi, 5-15 segnali, SNR -24..+6 dB, un terzo con evanescenza ==");
    for (name, o) in variants() {
        report(name, &run(30, main_period, &o));
    }
}

#[test]
#[ignore]
fn forte_e_debole_vicini() {
    println!("\n== segnale forte (+8 dB, evanescenza 20%) e uno debole (-14 dB) vicino: 24 periodi per distanza ==");
    for distance in [12.0f32, 18.0, 25.0] {
        for (name, o) in variants() {
            let label = format!("{name} a {distance} Hz");
            report(&label, &run(24, |k| pair_period(k, distance, 8.0, -14.0), &o));
        }
    }
}

#[test]
#[ignore]
fn solo_rumore_nessun_falso() {
    println!("\n== 500 periodi di solo rumore ==");
    for (name, o) in variants() {
        let s = run(500, |k| (Vec::new(), render(&[], 90_000 + k)), &o);
        println!("{name:<46} falsi {:>3} su {} periodi  {:>7.0} ms/periodo", s.false_, s.periods, s.millis / s.periods as f64);
    }
}

#[test]
#[ignore]
fn un_segnale_debole_a_snr_decrescente() {
    println!("\n== un solo segnale, 40 periodi per SNR ==");
    for snr in [-20.0f32, -22.0, -23.0, -24.0] {
        for (name, o) in variants() {
            let make = |k: u64| {
                let mut rng = Rng(7000 + k);
                let s = Signal { text: message(&mut rng), freq: rng.range(400.0, 2600.0) as f32, dt: rng.range(-0.2, 0.8) as f32, snr, fade: 0.0 };
                let audio = render(std::slice::from_ref(&s), 7000 + k);
                (vec![s], audio)
            };
            report(&format!("{name} a {snr} dB"), &run(40, make, &o));
        }
    }
}

#[test]
#[ignore]
fn analisi_a_priori_su_messaggi_per_noi() {
    use ft8::ap::ApOptions;
    println!("\n== un messaggio per IU3ABC (profondita' 2), 40 periodi per SNR: senza AP, con AP giusta, con AP sbagliata ==");
    for snr in [-21.0f32, -22.0, -23.0, -24.0] {
        let make = |k: u64| {
            let mut rng = Rng(9000 + k);
            let text = format!("IU3ABC {} {:+03}", call(&mut rng), rng.below(20) as i32 - 24);
            let s = Signal { text, freq: rng.range(400.0, 2600.0) as f32, dt: rng.range(-0.2, 0.8) as f32, snr, fade: 0.0 };
            let audio = render(std::slice::from_ref(&s), 9000 + k);
            (vec![s], audio)
        };
        let base = Options::depth(2);
        let giusta = Options { ap: Some(ApOptions::new("IU3ABC")), ..Options::depth(2) };
        let sbagliata = Options { ap: Some(ApOptions::new("W1AW")), ..Options::depth(2) };
        report(&format!("senza AP a {snr} dB"), &run(40, make, &base));
        report(&format!("AP IU3ABC a {snr} dB"), &run(40, make, &giusta));
        report(&format!("AP W1AW (sbagliata) a {snr} dB"), &run(40, make, &sbagliata));
    }
}

/// Legge un WAV PCM 16 bit mono a 12 kHz (quelli di `recordings` di Aurora) in campioni da -1 a 1.
fn read_wav(path: &std::path::Path) -> Option<Vec<f32>> {
    let bytes = std::fs::read(path).ok()?;
    let data = bytes.windows(4).position(|w| w == b"data")? + 8;
    Some(bytes.get(data..)?.as_chunks::<2>().0.iter().map(|c| f32::from(i16::from_le_bytes([c[0], c[1]])) / 32768.0).collect())
}

/// Banda piena vera: i WAV di una cartella fuori dal repository (`AURORA_WAV_DIR`, periodi FT8 da 15 s registrati da Aurora; contengono
/// nominativi reali e non vanno nel repository). Per ogni momento di avvio della decodifica (Presto 13,824 s, Normale 14,112 s, Tardi 14,400 s:
/// l'audio si taglia li', come fa il motore) e per 1 thread e tutti i processori, profondita' 3 come il motore: messaggi e millisecondi per periodo.
/// `cargo test -p ft8 --release --test misure -- --ignored --nocapture banda_vera`
#[test]
#[ignore]
fn banda_vera_da_wav() {
    let Ok(dir) = std::env::var("AURORA_WAV_DIR") else {
        println!("AURORA_WAV_DIR non impostata: niente da misurare");
        return;
    };
    let mut files: Vec<_> = std::fs::read_dir(&dir).map(|r| r.filter_map(|e| e.ok().map(|e| e.path())).collect()).unwrap_or_default();
    files.retain(|p| p.extension().is_some_and(|e| e.eq_ignore_ascii_case("wav")));
    files.sort();
    let periods: Vec<Vec<f32>> = files.iter().filter_map(|p| read_wav(p)).collect();
    let cpus = std::thread::available_parallelism().map_or(1, std::num::NonZero::get);
    println!("\n== {} periodi veri da {dir}, {cpus} processori ==", periods.len());
    for (label, start_ms) in [("Presto 13,824 s", 13_824usize), ("Normale 14,112 s", 14_112), ("Tardi 14,400 s", 14_400)] {
        for threads in [1, cpus] {
            let options = Options { threads, ..Options::depth(3) };
            let (mut total, mut ms, mut worst) = (0usize, 0.0f64, 0.0f64);
            let mut counts = Vec::new();
            for audio in &periods {
                let cut = &audio[..audio.len().min(start_ms * SAMPLE_RATE / 1000)];
                // il decoder vuole il periodo intero: la parte non ancora arrivata e' silenzio, come nel motore
                let mut slot = cut.to_vec();
                slot.resize(SLOT_SAMPLES.max(cut.len()), 0.0);
                let t0 = Instant::now();
                let out = decode_slot(&slot, &options, &mut CallHashes::default());
                let t = t0.elapsed().as_secs_f64() * 1000.0;
                worst = worst.max(t);
                ms += t;
                total += out.len();
                counts.push(out.len());
            }
            println!(
                "{label:<18} thread {threads:>2}: messaggi {total:>4} ({counts:?})  media {:>6.0} ms/periodo  peggiore {worst:>6.0} ms",
                ms / periods.len().max(1) as f64
            );
        }
    }
}
