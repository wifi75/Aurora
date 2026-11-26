//! Throb e ThrobX di Aurora contro quelli di fldigi 4.2.13 (`oracle/fldigi_throb.cpp`): il nostro trasmettitore da' la stessa onda di
//! quello di fldigi, fldigi riceve il nostro e noi riceviamo il suo, in banda laterale superiore e con l'inversione.
//! Si compila solo con `--features fldigi-oracle` (vedi `build.rs`).
#![cfg(feature = "fldigi-oracle")]

mod common;

use mode_digi::fldigi_oracle::FldigiThrob;
use mode_digi::throb::{ThrobRx, ThrobTx, ThrobVariant};

const TESTO: &str = "CQ DE IU3ABC IO91WM 599 73? @ (K)";
const TESTO_X: &str = "CQ DE IU3ABC IO91WM #\"+;:?!@= 73";

fn testo(v: ThrobVariant) -> &'static str {
    if v.is_x() { TESTO_X } else { TESTO }
}

fn con_rumore(mut audio: Vec<f32>, seed: u64) -> Vec<f32> {
    let mut a = vec![0.0_f32; 4000];
    a.append(&mut audio);
    a.extend(vec![0.0_f32; 16000]);
    let mut rng = common::Rng(seed);
    common::aggiungi_rumore(&mut a, 0.02, &mut rng);
    a
}

/// Le sei varianti, dritte e invertite: la nostra onda e' quella di fldigi campione per campione (stesse tabelle, stessi inviluppi).
#[test]
fn la_nostra_onda_e_quella_di_fldigi() {
    for v in ThrobVariant::ALL {
        for rev in [false, true] {
            let t = if v.is_x() { "IU3ABC  #-=\n73" } else { "IU3ABC 5NN ?@-\nK" };
            let ours = ThrobTx::new(v, 1000.0, 1.0, rev).transmit(t.as_bytes());
            let theirs = FldigiThrob::transmit(v, 1000.0, rev, t);
            assert_eq!(ours.len(), theirs.len(), "{v:?} rev {rev}: lunghezze");
            let max = ours.iter().zip(&theirs).map(|(a, b)| (a - b).abs()).fold(0.0_f32, f32::max);
            assert!(max < 1e-4, "{v:?} rev {rev}: scarto massimo {max}");
        }
    }
}

/// fldigi riceve il nostro Throb4, ThrobX2 e Throb2, dritto e invertito (il trattino di Throb fldigi lo mostra come `=`).
#[test]
fn fldigi_riceve_il_nostro() {
    for v in [ThrobVariant::Throb4, ThrobVariant::ThrobX2, ThrobVariant::Throb2] {
        for rev in [false, true] {
            let a = con_rumore(ThrobTx::new(v, 1000.0, 0.5, rev).transmit(testo(v).as_bytes()), 7);
            let mut fl = FldigiThrob::new(v, 1000.0, rev);
            let got: Vec<u8> = a.chunks(512).flat_map(|b| fl.process(b)).collect();
            let got = String::from_utf8_lossy(&got).into_owned();
            assert!(got.contains(testo(v)), "{v:?} rev {rev}: fldigi ha ricevuto {got:?}");
        }
    }
    let a = con_rumore(ThrobTx::new(ThrobVariant::Throb4, 1000.0, 0.5, false).transmit(b"A-B"), 8);
    let mut fl = FldigiThrob::new(ThrobVariant::Throb4, 1000.0, false);
    let got: Vec<u8> = a.chunks(512).flat_map(|b| fl.process(b)).collect();
    assert!(String::from_utf8_lossy(&got).contains("A=B"), "trattino: {:?}", String::from_utf8_lossy(&got));
}

/// Noi riceviamo il trasmettitore di fldigi (dritto e invertito; il trattino di fldigi torna trattino).
#[test]
fn riceviamo_fldigi() {
    for v in [ThrobVariant::Throb4, ThrobVariant::ThrobX2, ThrobVariant::ThrobX4, ThrobVariant::Throb2] {
        for rev in [false, true] {
            let a = con_rumore(FldigiThrob::transmit(v, 1000.0, rev, testo(v)).into_iter().map(|s| s * 0.5).collect(), 9);
            let mut rx = ThrobRx::new(v, 1000.0, rev, false);
            for b in a.chunks(512) {
                rx.process(b);
            }
            let got = String::from_utf8_lossy(&rx.take()).into_owned();
            assert_eq!(got.trim(), testo(v), "{v:?} rev {rev}");
        }
    }
    let a = con_rumore(FldigiThrob::transmit(ThrobVariant::Throb4, 1000.0, false, "A-B").into_iter().map(|s| s * 0.5).collect(), 10);
    let mut rx = ThrobRx::new(ThrobVariant::Throb4, 1000.0, false, false);
    rx.process(&a);
    assert_eq!(String::from_utf8_lossy(&rx.take()).trim(), "A-B");
}
