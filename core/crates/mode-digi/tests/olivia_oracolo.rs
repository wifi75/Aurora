//! Il codificatore di Olivia e Contestia confrontato, blocco per blocco, con quello della libreria di Jalocha di fldigi 4.2.13.
//! Si compila solo con `--features fldigi-oracle` (vedi `build.rs`): senza non c'e' niente da provare.
#![cfg(feature = "fldigi-oracle")]

use mode_digi::fldigi_oracle::olivia_encode_block;
use mode_digi::olivia::{Encoder, Variant};

/// Una sequenza di caratteri di prova ripetibile (senza dipendenze): tutti i valori da 0 a 255.
fn chars(seed: u64) -> Vec<u8> {
    let mut x = seed.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1_442_695_040_888_963_407);
    (0..8)
        .map(|_| {
            x = x.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1_442_695_040_888_963_407);
            (x >> 56) as u8
        })
        .collect()
}

#[test]
fn the_olivia_blocks_are_identical_to_fldigis_for_every_number_of_tones() {
    for bits in 1..=8 {
        let ours = Encoder::new(Variant::Olivia, bits).expect("bits validi");
        for seed in 0..200u64 {
            let input = chars(seed);
            let theirs = olivia_encode_block(bits, false, &input);
            assert_eq!(ours.encode_block(&input), theirs, "Olivia, {bits} bit per simbolo, caratteri {input:?}");
        }
    }
}

#[test]
fn the_contestia_blocks_are_identical_to_fldigis_for_every_number_of_tones() {
    for bits in 1..=8 {
        let ours = Encoder::new(Variant::Contestia, bits).expect("bits validi");
        for seed in 0..200u64 {
            let input = chars(seed + 1000);
            let theirs = olivia_encode_block(bits, true, &input);
            assert_eq!(ours.encode_block(&input), theirs, "Contestia, {bits} bit per simbolo, caratteri {input:?}");
        }
    }
}

#[test]
fn readable_text_blocks_match_too() {
    for (bits, text) in [(5usize, &b"CQ CQ DE"[..]), (4, b"IU3ABC K"), (3, b"hello"), (8, b"12345678")] {
        let ours = Encoder::new(Variant::Olivia, bits).expect("bits validi");
        assert_eq!(ours.encode_block(text), olivia_encode_block(bits, false, text));
        let cont = Encoder::new(Variant::Contestia, bits).expect("bits validi");
        assert_eq!(cont.encode_block(text), olivia_encode_block(bits, true, text));
    }
}

use mode_digi::fldigi_oracle::FldigiOlivia;
use mode_digi::olivia::{Params, Transmitter};

/// Trasmette con il nostro modulatore e fa decodificare dal ricevitore di fldigi: il testo deve tornare uguale.
fn through_fldigi(variant: Variant, tones: usize, bandwidth: usize, center_hz: f64, text: &[u8], noise: f32) -> String {
    let tx = Transmitter::new(Params { variant, tones, bandwidth, center_hz }).expect("parametri validi");
    let mut audio = vec![0.0f32; 8000];
    audio.extend(tx.transmit(text));
    audio.extend(std::iter::repeat_n(0.0f32, 8000 * 8));
    if noise > 0.0 {
        let mut x: u64 = 12345;
        for s in audio.iter_mut() {
            let mut g = 0.0f32;
            for _ in 0..6 {
                x = x.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1_442_695_040_888_963_407);
                g += ((x >> 33) as f32) / (u32::MAX >> 1) as f32;
            }
            *s = *s * 0.5 + noise * (g - 3.0) * 0.8165;
        }
    }
    let mut rx = FldigiOlivia::new(tones, bandwidth, center_hz, variant == Variant::Contestia);
    for chunk in audio.chunks(512) {
        rx.process(chunk);
    }
    rx.flush();
    String::from_utf8_lossy(&rx.take()).replace('\0', "")
}

#[test]
fn our_olivia_transmission_is_decoded_by_fldigi_in_the_common_variants() {
    let text = b"CQ CQ CQ DE IU3ABC IU3ABC IO91WM K";
    for (tones, bandwidth) in [(8usize, 250usize), (16, 500), (32, 1000), (4, 250), (8, 500)] {
        let got = through_fldigi(Variant::Olivia, tones, bandwidth, 1500.0, text, 0.0);
        assert!(got.contains("CQ CQ CQ DE IU3ABC IU3ABC IO91WM K"), "Olivia {tones}/{bandwidth}: ricevuto {got:?}");
    }
}

#[test]
fn our_contestia_transmission_is_decoded_by_fldigi() {
    let got = through_fldigi(Variant::Contestia, 16, 500, 1500.0, b"CQ CQ DE IU3ABC K", 0.0);
    assert!(got.contains("CQ CQ DE IU3ABC K"), "Contestia 16/500: ricevuto {got:?}");
}

#[test]
fn our_olivia_survives_some_noise_when_decoded_by_fldigi() {
    let got = through_fldigi(Variant::Olivia, 32, 1000, 1500.0, b"CQ CQ CQ DE IU3ABC IU3ABC K", 0.15);
    assert!(got.contains("IU3ABC"), "Olivia 32/1000 con rumore: ricevuto {got:?}");
}

// ---------------------------------------------------------------------------------------------------------------------------------
// Il nostro ricevitore confrontato con quello di fldigi sullo stesso audio.

use mode_digi::olivia_rx::Receiver;
use rustfft::FftPlanner;
use rustfft::num_complex::Complex;

/// Rumore gaussiano ripetibile (xorshift e Box-Muller).
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
    fn uniform(&mut self) -> f64 {
        ((self.next() >> 11) as f64 + 0.5) / (1u64 << 53) as f64
    }
    fn gauss(&mut self) -> f64 {
        (-2.0 * self.uniform().ln()).sqrt() * (2.0 * std::f64::consts::PI * self.uniform()).cos()
    }
}

/// Sposta in frequenza tutto l'audio di `hz` (segnale analitico su tutta la durata).
fn shift_frequency(audio: &[f32], hz: f64) -> Vec<f32> {
    let n = audio.len().next_power_of_two();
    let mut buf: Vec<Complex<f64>> = audio.iter().map(|&x| Complex::new(f64::from(x), 0.0)).collect();
    buf.resize(n, Complex::default());
    let mut planner = FftPlanner::new();
    planner.plan_fft_forward(n).process(&mut buf);
    for (i, v) in buf.iter_mut().enumerate() {
        if i != 0 && i != n / 2 {
            *v = if i < n / 2 { *v * 2.0 } else { Complex::default() };
        }
    }
    planner.plan_fft_inverse(n).process(&mut buf);
    (0..audio.len())
        .map(|i| {
            let ph = 2.0 * std::f64::consts::PI * hz * i as f64 / 8000.0;
            ((buf[i] * Complex::new(ph.cos(), ph.sin())).re / n as f64) as f32
        })
        .collect()
}

fn make_audio(p: Params, text: &[u8], delay: usize, shift_hz: f64, noise: f64, seed: u64) -> Vec<f32> {
    let tx = Transmitter::new(p).expect("parametri validi");
    let mut audio = vec![0.0f32; delay];
    audio.extend(tx.transmit(text));
    audio.extend(std::iter::repeat_n(0.0f32, 8000 * 2));
    if shift_hz != 0.0 {
        audio = shift_frequency(&audio, shift_hz);
    }
    let mut rng = Rng(seed | 1);
    for s in audio.iter_mut() {
        *s = (f64::from(*s) * 0.5 + noise * rng.gauss()) as f32;
    }
    audio
}

/// Quello che hanno visto i due ricevitori: testo, S/N e offset al momento del S/N piu' alto (letti negli stessi istanti).
struct Both {
    ours: String,
    theirs: String,
    ours_snr: f64,
    theirs_snr: f64,
    ours_freq: f64,
    theirs_freq: f64,
}

fn both(p: Params, audio: &[f32]) -> Both {
    let mut ours = Receiver::new(p, 8, 4, 3.0).expect("parametri validi");
    let mut theirs = FldigiOlivia::new(p.tones, p.bandwidth, p.center_hz, p.variant == Variant::Contestia);
    let (mut o, mut t) = (Vec::new(), Vec::new());
    let mut r = Both { ours: String::new(), theirs: String::new(), ours_snr: 0.0, theirs_snr: 0.0, ours_freq: 0.0, theirs_freq: 0.0 };
    for chunk in audio.chunks(512) {
        ours.process(chunk);
        theirs.process(chunk);
        if ours.snr() > r.ours_snr {
            r.ours_snr = ours.snr();
            r.ours_freq = ours.frequency_offset();
        }
        if theirs.snr() > r.theirs_snr {
            r.theirs_snr = theirs.snr();
            r.theirs_freq = theirs.frequency_offset();
        }
        o.extend(ours.take());
        t.extend(theirs.take());
    }
    ours.flush();
    theirs.flush();
    o.extend(ours.take());
    t.extend(theirs.take());
    r.ours = String::from_utf8_lossy(&o).into_owned();
    r.theirs = String::from_utf8_lossy(&t).replace('\0', "");
    r
}

const VARIANTS: [(Variant, usize, usize); 6] = [
    (Variant::Olivia, 4, 250),
    (Variant::Olivia, 8, 250),
    (Variant::Olivia, 8, 500),
    (Variant::Olivia, 16, 500),
    (Variant::Olivia, 32, 1000),
    (Variant::Contestia, 16, 500),
];

#[test]
fn on_clean_audio_our_receiver_gives_the_same_text_snr_and_offset_as_fldigi() {
    let text = b"CQ CQ CQ DE IU3ABC IU3ABC IO91WM K";
    for (i, (variant, tones, bandwidth)) in VARIANTS.into_iter().enumerate() {
        let p = Params { variant, tones, bandwidth, center_hz: 1500.0 };
        let bin_hz = 8000.0 / (1usize << (p.bits_per_symbol() + 7 - (bandwidth / 125).ilog2() as usize)) as f64;
        // offset di un numero intero di bin, ottenuto spostando il centro del trasmettitore (il primo tono si sposta di un bin esatto
        // per ogni `bin_hz`): il silenzio resta zero esatto e la stima e' esatta per tutti e due; ritardo diverso per ogni variante
        let shift = if i % 2 == 0 { 3.0 * bin_hz } else { -2.0 * bin_hz };
        let tx = Params { center_hz: 1500.0 + shift, ..p };
        let audio = make_audio(tx, text, 1234 * i, 0.0, 0.0, 1);
        let r = both(p, &audio);
        println!(
            "{variant:?} {tones}/{bandwidth}: S/N nostro {:.2} fldigi {:.2}, offset nostro {:.2} fldigi {:.2} Hz",
            r.ours_snr, r.theirs_snr, r.ours_freq, r.theirs_freq
        );
        // Sul tratto trasmesso i due ricevitori danno gli stessi caratteri. Prima e dopo (i blocchi che si chiudono su simboli di
        // silenzio, mentre gli integratori stanno ancora sopra la soglia) i bin sono quasi vuoti e l'equalizzazione dello spettro ne
        // ingrandisce gli arrotondamenti (doppia precisione noi, singola fldigi): per esempio "\u{7f}j\\CQ ... K\u{1}>H" contro
        // "QF\\CQ ... K}I4". Quei caratteri spuri sono casuali in tutti e due e non si confrontano.
        let upto_end = |s: &str| match (s.find("CQ CQ CQ"), s.rfind("IO91WM K")) {
            (Some(a), Some(b)) if b > a => s[a..b + 8].to_string(),
            _ => s.to_string(),
        };
        assert_eq!(upto_end(&r.ours), upto_end(&r.theirs), "{variant:?} {tones}/{bandwidth}: testo diverso ({:?} / {:?})", r.ours, r.theirs);
        assert!(r.ours.contains("CQ CQ CQ DE IU3ABC IU3ABC IO91WM K"), "{variant:?} {tones}/{bandwidth}: {:?}", r.ours);
        assert!((r.ours_freq - r.theirs_freq).abs() < 1e-6, "{variant:?} {tones}/{bandwidth}: offset diverso");
        assert!((r.ours_freq - shift).abs() < 1e-6, "{variant:?} {tones}/{bandwidth}: offset {} invece di {shift}", r.ours_freq);
        let rel = (r.ours_snr - r.theirs_snr).abs() / r.theirs_snr.max(1e-9);
        assert!(rel < 0.02, "{variant:?} {tones}/{bandwidth}: S/N {} contro {}", r.ours_snr, r.theirs_snr);
    }
}

/// Quanti caratteri del testo atteso si ritrovano, in ordine, nel testo ricevuto (sottosequenza comune piu' lunga): i caratteri
/// spuri prima e dopo non contano, quelli sbagliati o mancanti si'.
fn lcs(a: &[u8], b: &[u8]) -> usize {
    let mut prev = vec![0usize; b.len() + 1];
    for &x in a {
        let mut cur = vec![0usize; b.len() + 1];
        for (j, &y) in b.iter().enumerate() {
            cur[j + 1] = if x == y { prev[j] + 1 } else { prev[j + 1].max(cur[j]) };
        }
        prev = cur;
    }
    prev[b.len()]
}

/// Con rumore crescente il nostro ricevitore deve ritrovare almeno i caratteri che ritrova fldigi, meno il 3% del totale (le due catene
/// lavorano in doppia e in singola precisione: vicino alla soglia un blocco puo' passare per l'una e non per l'altra).
#[test]
fn with_growing_noise_we_decode_at_least_as_much_as_fldigi() {
    let text = b"CQ CQ DE IU3ABC IU3ABC IO91WM PSE K";
    for (variant, tones, bandwidth) in VARIANTS {
        let p = Params { variant, tones, bandwidth, center_hz: 1500.0 };
        let (mut ours_total, mut theirs_total, mut possible) = (0usize, 0usize, 0usize);
        let mut report = String::new();
        for (k, noise) in [1.0f64, 1.5, 2.0, 2.5, 3.0, 3.5].into_iter().enumerate() {
            let (mut o, mut t) = (0, 0);
            for seed in 0..3u64 {
                let audio = make_audio(p, text, 777 * seed as usize, 11.0, noise, 1000 + 10 * k as u64 + seed);
                let r = both(p, &audio);
                o += lcs(text, r.ours.as_bytes());
                t += lcs(text, r.theirs.as_bytes());
                possible += text.len();
            }
            report.push_str(&format!(" rumore {noise}: {o}/{t}"));
            ours_total += o;
            theirs_total += t;
        }
        println!("{variant:?} {tones}/{bandwidth} (nostri/fldigi):{report}; totale {ours_total}/{theirs_total} su {possible}");
        assert!(
            ours_total as f64 >= theirs_total as f64 - 0.03 * possible as f64,
            "{variant:?} {tones}/{bandwidth}: noi {ours_total}, fldigi {theirs_total} su {possible}"
        );
    }
}
