//! MT63 di Aurora contro quello di fldigi 4.2.13 (`oracle/fldigi_mt63.cpp`, che compila `mt63base.cxx` e `dsp.cxx` di fldigi cosi' come
//! sono): il nostro trasmettitore da' la stessa onda di quello di fldigi, fldigi riceve il nostro (anche in LSB, dopo il rovesciamento
//! della radio) e noi riceviamo il suo. Si compila solo con `--features fldigi-oracle` (vedi `build.rs`).
#![cfg(feature = "fldigi-oracle")]

mod common;

use mode_digi::fldigi_oracle::FldigiMt63;
use mode_digi::mt63::{Mt63Tx, Mt63Variant};

const TESTO: &str = "CQ CQ DE IU3ABC IU3ABC IO91WM pse k 599 73 <QSL?> {test} ~";

fn con_rumore(mut audio: Vec<f32>, sigma: f64, seed: u64) -> Vec<f32> {
    let mut a = vec![0.0_f32; 4000];
    a.append(&mut audio);
    a.extend(vec![0.0_f32; 56_000]);
    let mut rng = common::Rng(seed);
    common::aggiungi_rumore(&mut a, sigma, &mut rng);
    a
}

/// Le sei varianti a due frequenze: la nostra onda e' quella di fldigi campione per campione (stessi numeri a caso).
#[test]
fn la_nostra_onda_e_quella_di_fldigi() {
    for v in Mt63Variant::ALL {
        for f in [1500.0, 1000.0 + v.bandwidth_hz() / 4.0] {
            let theirs = FldigiMt63::transmit(v, f, "IU3ABC de IU3ABC 5NN k");
            let mut tx = Mt63Tx::with_fill(v, f, 1.0, false, &theirs.fill);
            tx.set_jam_bits(theirs.jam.clone());
            let ours = tx.transmit_like_fldigi(b"IU3ABC de IU3ABC 5NN k");
            assert_eq!(ours.len(), theirs.wave.len(), "{v:?} {f}: lunghezze");
            let peak = theirs.wave.iter().fold(0.0_f32, |m, x| m.max(x.abs()));
            let max = ours.iter().zip(&theirs.wave).map(|(a, b)| (a - b).abs()).fold(0.0_f32, f32::max);
            assert!(peak > 0.1 && max < 1e-5 * peak.max(1.0), "{v:?} {f}: scarto massimo {max}, picco {peak}");
        }
    }
}

/// Rovescia lo spettro come una radio in LSB vista da un ricevitore in USB: a 8 kHz la moltiplicazione per (-1)^n porta f in 4000 - f.
fn rovescia(a: &[f32]) -> Vec<f32> {
    a.iter().enumerate().map(|(i, &x)| if i % 2 == 0 { x } else { -x }).collect()
}

/// fldigi riceve il nostro in tutte le varianti, con rumore; e in LSB dopo il rovesciamento della radio.
#[test]
fn fldigi_riceve_il_nostro() {
    for v in Mt63Variant::ALL {
        for lsb in [false, true] {
            let f = 1500.0;
            let a = con_rumore(Mt63Tx::new(v, f, 0.5, lsb).transmit(TESTO.as_bytes()), 0.05, 7);
            let (a, f_rx) = if lsb { (rovescia(&a), 4000.0 - f) } else { (a, f) };
            let mut fl = FldigiMt63::new(v, f_rx, 16);
            let got: Vec<u8> = a.chunks(512).flat_map(|b| fl.process(b)).collect();
            let got = String::from_utf8_lossy(&got).into_owned();
            assert!(got.contains(TESTO), "{v:?} lsb {lsb}: fldigi ha ricevuto {got:?}");
        }
    }
}

use mode_digi::mt63::{Mt63Rx, SQUELCH_OFF};

/// Noi riceviamo il trasmettitore di fldigi in tutte le varianti, con rumore, niente prima e niente dopo il testo; e il suo segnale
/// rovesciato come da una radio in LSB.
#[test]
fn riceviamo_fldigi() {
    for v in Mt63Variant::ALL {
        for lsb in [false, true] {
            let f = 1500.0;
            let tx = FldigiMt63::transmit(v, f, TESTO);
            let peak = tx.wave.iter().fold(0.0_f32, |m, x| m.max(x.abs()));
            let a = con_rumore(tx.wave.iter().map(|s| s * 0.5 / peak).collect(), 0.05, 9);
            let (a, f_rx) = if lsb { (rovescia(&a), 4000.0 - f) } else { (a, f) };
            let mut rx = Mt63Rx::new(v, f_rx, lsb, false);
            for b in a.chunks(512) {
                rx.process(b);
            }
            let got = String::from_utf8_lossy(&rx.take()).into_owned();
            assert_eq!(got.trim(), TESTO, "{v:?} lsb {lsb}");
        }
    }
}

/// Sullo stesso segnale di fldigi con molto rumore il nostro ricevitore, con lo squelch spento come quello di fldigi, da' gli stessi
/// caratteri, errori compresi (noi non stampiamo i caratteri di controllo; i primi caratteri di rumore possono differire); con lo squelch
/// acceso, a un livello di rumore piu' basso, arriva il testo senza rumore intorno (al massimo senza i primi due caratteri).
#[test]
fn nel_rumore_come_fldigi() {
    let testo = "THE QUICK BROWN FOX JUMPS OVER THE LAZY DOG 0123456789 de IU3ABC IO91WM pse k";
    let casi = [
        (Mt63Variant::Mt63_500s, 0.5, false),
        (Mt63Variant::Mt63_1000l, 0.35, false),
        (Mt63Variant::Mt63_2000l, 0.2, false),
        (Mt63Variant::Mt63_1000s, 0.4, true),
    ];
    for (v, sigma, long) in casi {
        let tx = FldigiMt63::transmit(v, 1500.0, testo);
        let peak = tx.wave.iter().fold(0.0_f32, |m, x| m.max(x.abs()));
        for seed in 0..3 {
            let a = con_rumore(tx.wave.iter().map(|s| s * 0.5 / peak).collect(), sigma, 200 + seed);
            let mut rx = Mt63Rx::new(v, 1500.0, false, long);
            rx.set_squelch_db(SQUELCH_OFF);
            let mut fl = FldigiMt63::new(v, 1500.0, if long { 32 } else { 16 });
            let mut fl_out = Vec::new();
            for b in a.chunks(512) {
                rx.process(b);
                fl_out.extend(fl.process(b));
            }
            fl_out.retain(|&c| c >= b' ' || c == b'\n');
            let ours = rx.take();
            // i primissimi caratteri (rumore prima dell'aggancio, con i filtri appena partiti) possono differire
            let same = ours.iter().rev().zip(fl_out.iter().rev()).take_while(|(a, b)| a == b).count();
            assert!(
                same + 12 >= ours.len().max(fl_out.len()),
                "{v:?} sigma {sigma} seme {seed}:
noi    {:?}
fldigi {:?}",
                String::from_utf8_lossy(&ours),
                String::from_utf8_lossy(&fl_out)
            );
        }
        let a = con_rumore(tx.wave.iter().map(|s| s * 0.5 / peak).collect(), sigma * 0.6, 300);
        let mut rx = Mt63Rx::new(v, 1500.0, false, long);
        rx.process(&a);
        // fldigi parte subito col testo: con un segnale debole si possono perdere i primi due caratteri (vedi `start_ok` nel ricevitore),
        // ma niente rumore prima o dopo
        let got = String::from_utf8_lossy(&rx.take()).trim().to_string();
        assert!(testo.ends_with(&got) && got.len() + 2 >= testo.len(), "{v:?} sigma {} con lo squelch: {got:?}", sigma * 0.6);
    }
}

/// Il nostro ricevitore e quello di fldigi sullo stesso segnale rumoroso (di fldigi): errori sui caratteri a vari livelli di rumore.
/// Solo misura: `cargo test --features fldigi-oracle --test mt63_oracolo -- --ignored --nocapture confronto`.
#[test]
#[ignore]
fn confronto_nel_rumore() {
    let testo = "THE QUICK BROWN FOX JUMPS OVER THE LAZY DOG 0123456789 de IU3ABC IO91WM pse k ".repeat(2);
    for v in [Mt63Variant::Mt63_500s, Mt63Variant::Mt63_1000l, Mt63Variant::Mt63_2000l] {
        let tx = FldigiMt63::transmit(v, 1500.0, &testo);
        let peak = tx.wave.iter().fold(0.0_f32, |m, x| m.max(x.abs()));
        for sigma in [0.2, 0.35, 0.5, 0.7, 0.9] {
            let (mut e_ours, mut e_fl) = (0, 0);
            for seed in 0..3 {
                let a = con_rumore(tx.wave.iter().map(|s| s * 0.5 / peak).collect(), sigma, 100 + seed);
                let mut rx = Mt63Rx::new(v, 1500.0, false, false);
                rx.set_squelch_db(SQUELCH_OFF);
                let mut fl = FldigiMt63::new(v, 1500.0, 16);
                let mut fl_out = Vec::new();
                for b in a.chunks(512) {
                    rx.process(b);
                    fl_out.extend(fl.process(b));
                }
                let ours = rx.take();
                e_ours += errori(&ours, testo.as_bytes());
                e_fl += errori(&fl_out, testo.as_bytes());
            }
            eprintln!("{v:?} sigma {sigma}: errori noi {e_ours}, fldigi {e_fl} (su {})", 3 * testo.len());
        }
        for sq in [3.0f32, 4.0, 5.0, 5.5, 6.0] {
            let mut e = 0;
            for (k, sigma) in [0.2, 0.35, 0.5].into_iter().enumerate() {
                let a = con_rumore(tx.wave.iter().map(|s| s * 0.5 / peak).collect(), sigma, 300 + k as u64);
                let mut rx = Mt63Rx::new(v, 1500.0, false, false);
                rx.set_squelch_db(sq);
                rx.process(&a);
                e += errori(&rx.take(), testo.as_bytes());
            }
            let mut a = vec![0.0f32; 8000 * 600];
            let mut rng = common::Rng(sq as u64);
            common::aggiungi_rumore(&mut a, 0.3, &mut rng);
            let mut rx = Mt63Rx::new(v, 1500.0, false, false);
            rx.set_squelch_db(sq);
            rx.process(&a);
            eprintln!("{v:?} squelch {sq}: errori con rumore 0.2/0.35/0.5 {e}, caratteri in 600 s di solo rumore {}", rx.take().len());
        }
    }
}

/// Errori rispetto al testo: la distanza di modifica fra il testo e il pezzo ricevuto che gli somiglia di piu' (fldigi stampa anche
/// rumore prima e dopo, senza squelch): si tagliano le parti fuori dal primo e dall'ultimo carattere giusto.
fn errori(got: &[u8], want: &[u8]) -> usize {
    let mut best = common::levenshtein(got, want);
    for s in 0..got.len().min(40) {
        for e in 0..got.len().min(40) {
            if s + e < got.len() {
                best = best.min(common::levenshtein(&got[s..got.len() - e], want));
            }
        }
    }
    best
}
