//! Prove dell'AFC del ricevitore RTTY: aggancio (da quanti Hz di errore e in quanto tempo) e stabilita' (rumore, stazione accanto,
//! deriva). Si lancia a mano:
//!
//! `cargo test --release -p mode-digi --test rtty_afc -- --ignored --nocapture --test-threads=1`

mod common;

use common::rtty_scenari::{AMP, CENTRE, RATE, errori, solo_rumore};
use mode_digi::rtty::{self, RttyDemodulator, RttyParams};

const TESTO: &str = "RYRYRY CQ TEST DE IU3ABC IU3ABC K IU3ABC 599 150 TU CQ TEST DE IU3ABC IU3ABC K TEST QTH IO91WM NAME TEST HW CPY BK RYRY";

fn params(centre: f32) -> RttyParams {
    RttyParams { centre_hz: centre, shift_hz: 170.0, baud: 45.45, reverse: false }
}

/// Segnale a `centre + df` (df che puo' cambiare nel tempo: `deriva` Hz al secondo) con rumore a `snr` dB in 3 kHz, 1 s di rumore prima
/// e 2 s dopo; piu' eventuali altre stazioni (scostamento, dB rispetto al segnale).
fn audio(df: f32, deriva: f32, snr: f64, altre: &[(f32, f64)], seed: u64) -> Vec<f32> {
    let mut sig = vec![0.0_f32; RATE as usize];
    if deriva == 0.0 {
        sig.extend(rtty::modulate(TESTO.as_bytes(), params(CENTRE + df), AMP as f32));
    } else {
        // a pezzi di una parola, ognuno un po' piu' in la'
        let mut t = 0.0_f32;
        for w in TESTO.split_inclusive(' ') {
            let pezzo = rtty::modulate(w.as_bytes(), params(CENTRE + df + deriva * t), AMP as f32);
            t += pezzo.len() as f32 / RATE as f32;
            sig.extend(pezzo);
        }
    }
    sig.extend(vec![0.0; 2 * RATE as usize]);
    for &(d, db) in altre {
        let q = rtty::modulate(TESTO.repeat(2).as_bytes(), params(CENTRE + d), (AMP * 10f64.powf(db / 20.0)) as f32);
        for (a, b) in sig.iter_mut().zip(q) {
            *a += b;
        }
    }
    let g = 10f64.powf(-snr / 20.0) as f32;
    let rumore = solo_rumore(sig.len() as f64 / RATE + 1.0, seed);
    for (a, r) in sig.iter_mut().zip(rumore) {
        *a += g * r;
    }
    sig
}

/// Decodifica con l'AFC acceso, ascoltando su CENTRE: (secondi per arrivare entro 3 Hz dal segnale, errore finale a fine segnale,
/// massimo scarto dalla frequenza giusta dopo l'aggancio, CER %).
fn prova(a: &[f32], vero: impl Fn(f32) -> f32, fine_s: f32) -> (Option<f32>, f32, f32, f64) {
    let mut rx = RttyDemodulator::new(params(CENTRE));
    rx.set_afc(true);
    let mut got = Vec::new();
    let (mut aggancio, mut fine, mut scarto) = (None, 0.0_f32, 0.0_f32);
    for (k, b) in a.chunks(240).enumerate() {
        got.extend(rx.process(b));
        let t = (k + 1) as f32 * 240.0 / RATE as f32;
        if t > fine_s {
            continue;
        }
        let e = rx.frequency() - vero(t);
        if aggancio.is_none() && e.abs() < 3.0 && t > 1.0 {
            aggancio = Some(t - 1.0);
        }
        if aggancio.is_some() {
            scarto = scarto.max(e.abs());
        }
        fine = e;
    }
    (aggancio, fine, scarto, 100.0 * errori(TESTO, got) as f64 / TESTO.len() as f64)
}

fn riga(nome: &str, r: (Option<f32>, f32, f32, f64)) {
    let ag = r.0.map_or("  --".to_string(), |t| format!("{t:4.1}"));
    println!("{nome:<34} aggancio {ag} s  errore finale {:6.1} Hz  scarto dopo {:5.1} Hz  CER {:5.1} %", r.1, r.2, r.3);
}

fn lunghezza(df: f32) -> f32 {
    rtty::modulate(TESTO.as_bytes(), params(CENTRE + df), 0.1).len() as f32 / RATE as f32 + 1.0
}

#[test]
#[ignore]
fn afc() {
    for snr in [10.0, 0.0, -5.0] {
        for df in [10.0_f32, 20.0, 30.0, 40.0, 50.0, 70.0, 100.0] {
            let mut tot = (0.0, 0, 0.0, 0.0, 0.0);
            for seed in 0..3 {
                let a = audio(df, 0.0, snr, &[], 11 + seed);
                let r = prova(&a, |_| CENTRE + df, lunghezza(df));
                tot.0 += r.0.unwrap_or(99.0);
                tot.1 += usize::from(r.0.is_some());
                tot.2 += r.1.abs();
                tot.3 += r.2;
                tot.4 += r.3;
            }
            println!(
                "fuori {df:5.0} Hz, S/N {snr:4.0} dB: agganciati {}/3, aggancio medio {:4.1} s, |errore finale| {:5.1} Hz, scarto {:5.1} Hz, CER {:5.1} %",
                tot.1,
                tot.0 / 3.0,
                tot.2 / 3.0,
                tot.3 / 3.0,
                tot.4 / 3.0
            );
        }
    }
    for (snr, d) in [(10.0, 1.0_f32), (10.0, 3.0), (0.0, 1.0), (0.0, 3.0)] {
        let a = audio(0.0, d, snr, &[], 5);
        riga(&format!("deriva {d} Hz/s, S/N {snr}"), prova(&a, |t| CENTRE + d * (t - 1.0).max(0.0), lunghezza(0.0)));
    }
    for (snr, altra) in [(10.0, (200.0, 6.0)), (10.0, (120.0, 0.0)), (0.0, (200.0, 6.0)), (0.0, (90.0, -6.0)), (10.0, (-60.0, 0.0))] {
        let a = audio(10.0, 0.0, snr, &[altra], 7);
        riga(&format!("stazione a {:+} Hz {:+} dB, S/N {snr}", altra.0, altra.1), prova(&a, |_| CENTRE + 10.0, lunghezza(10.0)));
    }
    // senza il nostro segnale: solo una stazione accanto (non deve tirare l'AFC) e solo rumore
    for altra in [(60.0_f32, 6.0), (100.0, 10.0), (200.0, 10.0)] {
        let mut a = audio(0.0, 0.0, 0.0, &[altra], 9);
        let q = rtty::modulate(TESTO.as_bytes(), params(CENTRE), AMP as f32);
        for (x, y) in a.iter_mut().skip(RATE as usize).zip(q) {
            *x -= y;
        }
        let mut rx = RttyDemodulator::new(params(CENTRE));
        rx.set_afc(true);
        let mut max = 0.0_f32;
        for b in a.chunks(240) {
            rx.process(b);
            max = max.max((rx.frequency() - CENTRE).abs());
        }
        println!("solo una stazione a {:+} Hz {:+} dB: AFC mosso al massimo di {max:.1} Hz", altra.0, altra.1);
    }
}
