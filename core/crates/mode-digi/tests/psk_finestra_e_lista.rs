//! La finestra di ricezione PSK (il ricevitore principale come lo usa il servizio: AFC con i limiti di `notch::AfcGuard`, squelch delle
//! impostazioni) contro la lista «Segnali» (`skim::psk::PskBandDecoder`) sullo stesso audio: segnale debole, frequenza scelta non centrata
//! sulla portante, un secondo segnale vicino, pause di solo rumore.
//!
//! Si contano i caratteri giusti (lunghezza del testo mandato meno la distanza di Levenshtein, fra testo mandato e ricevuto senza spazi in
//! testa e in coda) e la spazzatura (caratteri non stampabili e caratteri scritti nelle pause e nel solo rumore).
//!
//! `cargo test --release -p mode-digi --test psk_finestra_e_lista -- --ignored --nocapture --test-threads=1` stampa la tabella; le prove
//! senza `ignore` controllano i casi principali.

mod common;

use common::{Rng, aggiungi_rumore, guadagno_complesso, levenshtein};
use mode_digi::notch::AfcGuard;
use mode_digi::psk::{PskDemodulator, PskMode, SQUELCH_DEFAULT, modulate};
use mode_digi::skim::BandDecoder;
use mode_digi::skim::psk::PskBandDecoder;

const RATE: usize = 12_000;
const AMP: f32 = 0.2;
/// Blocchi della scheda audio (40 ms) come li riceve il servizio.
const BLOCK: usize = 480;

struct Caso {
    mode: PskMode,
    /// S/N in 2,5 kHz (dB).
    snr_db: f32,
    /// La portante e' a 1000 Hz + `offset`, la frequenza scelta a 1000 Hz.
    offset: f32,
    /// Il secondo segnale a 1000 Hz + `offset` + `vicino` (0 = nessuno), 3 dB piu' debole.
    vicino: f32,
}

const TESTO: &str = "CQ CQ CQ DE IU3ABC IU3ABC IU3ABC PSE K ";

/// Audio: 6 s di rumore, il testo, 8 s di pausa, il testo, 6 s di rumore. Restituisce l'audio e gli intervalli (campioni) dei due testi.
fn audio(c: &Caso, seed: u64) -> (Vec<f32>, Vec<(usize, usize)>) {
    let f = 1000.0 + c.offset;
    let wave = modulate(TESTO.as_bytes(), f, c.mode, AMP);
    let pausa = 8 * RATE;
    let testa = 6 * RATE;
    let len = testa + 2 * wave.len() + pausa + 6 * RATE;
    let mut a = vec![0.0_f32; len];
    let spans = vec![(testa, testa + wave.len()), (testa + wave.len() + pausa, testa + 2 * wave.len() + pausa)];
    for &(s, _) in &spans {
        for (x, w) in a[s..].iter_mut().zip(&wave) {
            *x += w;
        }
    }
    if c.vicino != 0.0 {
        // l'altra stazione parla durante la pausa e a cavallo dei testi
        let other = modulate(b"QRZ DE K1ABC K1ABC K1ABC K1ABC QRZ QRZ ", f + c.vicino, c.mode, AMP * 0.7);
        let s = testa + wave.len() / 2;
        for (x, w) in a[s..].iter_mut().zip(&other) {
            *x += w;
        }
    }
    // rumore: potenza del segnale AMP^2/2 contro sigma^2 * 2500/6000
    let sigma = (f64::from(AMP) * f64::from(AMP) / 2.0 / 10f64.powf(f64::from(c.snr_db) / 10.0) * 6000.0 / 2500.0).sqrt();
    aggiungi_rumore(&mut a, sigma, &mut Rng(seed));
    (a, spans)
}

/// Caratteri non stampabili (fuori da spazio-tilde, a capo escluso): quelli che la finestra mostra come riquadri.
fn sporchi(s: &[u8]) -> usize {
    s.iter().filter(|&&c| !(c == b'\n' || c == b'\r' || (0x20..0x7F).contains(&c))).count()
}

#[derive(Default, Debug, Clone, Copy)]
struct Conto {
    mandati: usize,
    giusti: usize,
    spazzatura: usize,
}

impl Conto {
    fn add(&mut self, o: Conto) {
        self.mandati += o.mandati;
        self.giusti += o.giusti;
        self.spazzatura += o.spazzatura;
    }
    fn pct(&self) -> (f64, f64) {
        let m = self.mandati.max(1) as f64;
        (100.0 * self.giusti as f64 / m, 100.0 * self.spazzatura as f64 / m)
    }
}

/// Il conto su un testo ricevuto con il momento (campione) di ogni carattere.
fn conta(rx: &[(usize, u8)], spans: &[(usize, usize)], c: &Caso) -> Conto {
    // margine: il carattere esce un po' dopo la fine del suo audio (filtri, Viterbi)
    let tail = 3 * c.mode.symbol_samples() * 16;
    let mut conto = Conto::default();
    let mut fuori = Vec::new();
    for (i, &(s, e)) in spans.iter().enumerate() {
        let end = spans.get(i + 1).map_or(e + tail, |n| (e + tail).min(n.0));
        let got: Vec<u8> = rx.iter().filter(|(t, _)| *t >= s && *t < end).map(|(_, c)| *c).collect();
        let want = TESTO.trim().as_bytes();
        let got_t = String::from_utf8_lossy(&got).trim().as_bytes().to_vec();
        let d = levenshtein(want, &got_t);
        conto.mandati += want.len();
        conto.giusti += want.len().saturating_sub(d);
        conto.spazzatura += sporchi(&got);
        if std::env::var_os("PSK_DEBUG").is_some() {
            eprintln!("    {:?}", String::from_utf8_lossy(&got));
        }
        fuori.extend(rx.iter().filter(|(t, _)| *t >= s && *t < end).map(|(t, _)| *t));
    }
    // nelle pause e nel rumore: ogni carattere non spazio e' spazzatura
    let pausa = rx.iter().filter(|(t, ch)| !fuori.contains(t) && !ch.is_ascii_whitespace()).count();
    if std::env::var_os("PSK_DEBUG").is_some() {
        let p: Vec<u8> = rx.iter().filter(|(t, _)| !fuori.contains(t)).map(|(_, c)| *c).collect();
        eprintln!("    pause: {:?}", String::from_utf8_lossy(&p));
    }
    conto.spazzatura += pausa;
    conto
}

/// Il ricevitore principale come lo fa girare il servizio, con lo squelch dato.
fn finestra(c: &Caso, a: &[f32], squelch: Option<f32>, limit: f32, acquire: bool) -> Vec<(usize, u8)> {
    let mut rx = PskDemodulator::new(c.mode, 1000.0);
    rx.set_afc(true);
    rx.set_squelch(squelch);
    if acquire {
        rx.set_acquire(Some(limit));
    }
    let mut guard = AfcGuard::new(1000.0, limit);
    let mut out = Vec::new();
    for (i, b) in a.chunks(BLOCK).enumerate() {
        for ch in rx.process(b) {
            out.push((i * BLOCK, ch));
        }
        if let Some(hz) = rx.take_acquired() {
            guard.retune(hz);
        }
        if let Some(hz) = guard.step(rx.frequency(), Some(rx.metric()), (25.0, 12.0), rx.signal().1) {
            rx.steer(hz);
        }
    }
    out
}

/// La lista «Segnali»: il testo del canale piu' vicino alla portante (i caratteri hanno il momento dello spettro, a 170 ms).
fn lista(c: &Caso, a: &[f32]) -> Vec<(usize, u8)> {
    let mut b = PskBandDecoder::new(c.mode, 300.0, 2_500.0, 16);
    let target = 1000.0 + c.offset;
    let mut out: Vec<(usize, u8)> = Vec::new();
    let mut seen: Option<(u32, usize)> = None;
    for (i, blk) in a.chunks(BLOCK).enumerate() {
        b.process(blk);
        let chans = b.channels();
        let Some(ch) = chans.iter().filter(|ch| (ch.freq_hz - target).abs() < 25.0).min_by(|x, y| (x.freq_hz - target).abs().total_cmp(&(y.freq_hz - target).abs())) else {
            continue;
        };
        let bytes: Vec<u8> = ch.text.chars().map(|c| c as u32 as u8).collect();
        let done = match seen {
            Some((id, n)) if id == ch.id => n,
            _ => 0,
        };
        // il canale tiene il testo degli ultimi 60 s: i caratteri nuovi sono in coda
        for &x in bytes.iter().skip(done) {
            out.push((i * BLOCK, x));
        }
        seen = Some((ch.id, bytes.len()));
    }
    out
}

fn casi() -> Vec<Caso> {
    let mut v = Vec::new();
    for mode in [PskMode::Psk31, PskMode::Psk63] {
        let near = if mode == PskMode::Psk31 { 50.0 } else { 60.0 };
        for snr_db in [-10.0, -5.0, 0.0, 10.0] {
            for offset in [0.0, 8.0, 15.0] {
                for vicino in [0.0, near] {
                    v.push(Caso { mode, snr_db, offset, vicino });
                }
            }
        }
    }
    v
}

fn limite(m: PskMode) -> f32 {
    if m == PskMode::Psk31 { 15.0 } else { 30.0 }
}

#[test]
#[ignore]
fn tabella() {
    println!("modo   S/N  fuori vicino | prima (sq. spento)   | dopo (sq. {SQUELCH_DEFAULT} + aggancio) | lista");
    let mut tot = [Conto::default(); 3];
    for c in casi() {
        let mut r = [Conto::default(); 3];
        for seed in 1..=2 {
            let (a, spans) = audio(&c, 7 + seed);
            r[0].add(conta(&finestra(&c, &a, None, limite(c.mode), false), &spans, &c));
            r[1].add(conta(&finestra(&c, &a, Some(SQUELCH_DEFAULT), limite(c.mode), true), &spans, &c));
            r[2].add(conta(&lista(&c, &a), &spans, &c));
        }
        let f = |x: &Conto| {
            let (g, s) = x.pct();
            format!("{g:5.1}% ok {s:5.1}% sp")
        };
        println!("{:5} {:4} {:5} {:6} | {} | {} | {}", c.mode.name(), c.snr_db, c.offset, c.vicino, f(&r[0]), f(&r[1]), f(&r[2]));
        for (t, x) in tot.iter_mut().zip(r) {
            t.add(x);
        }
    }
    let f = |x: &Conto| {
        let (g, s) = x.pct();
        format!("{g:5.1}% ok {s:5.1}% sp")
    };
    println!("TOTALE                    | {} | {} | {}", f(&tot[0]), f(&tot[1]), f(&tot[2]));
}

/// La frequenza ascoltata dopo 4 s di segnale con la portante fuori di `off` Hz (S/N in 2,5 kHz), con le impostazioni del servizio.
fn converge(mode: PskMode, off: f32, snr_db: f32, vicino: f32, acquire: bool) -> f32 {
    let c = Caso { mode, snr_db, offset: off, vicino };
    let (a, spans) = audio(&c, 3);
    let limit = limite(mode);
    let mut rx = PskDemodulator::new(mode, 1000.0);
    rx.set_afc(true);
    rx.set_squelch(Some(SQUELCH_DEFAULT));
    if acquire {
        rx.set_acquire(Some(limit));
    }
    let mut guard = AfcGuard::new(1000.0, limit);
    let stop = spans[0].0 + 4 * RATE;
    for b in a[..stop].chunks(BLOCK) {
        rx.process(b);
        if let Some(hz) = rx.take_acquired() {
            guard.retune(hz);
        }
        if let Some(hz) = guard.step(rx.frequency(), Some(rx.metric()), (25.0, 12.0), rx.signal().1) {
            rx.steer(hz);
        }
    }
    rx.frequency()
}

#[test]
#[ignore]
fn tabella_afc() {
    for mode in [PskMode::Psk31, PskMode::Psk63] {
        for off in [-25.0, -15.0, -10.0, -5.0, 5.0, 10.0, 15.0, 25.0] {
            let before = converge(mode, off, 0.0, 0.0, false);
            let after = converge(mode, off, 0.0, 0.0, true);
            let nv = if mode == PskMode::Psk31 { 50.0 } else { 60.0 };
            let near0 = converge(mode, off, 0.0, nv, false);
            let near = converge(mode, off, 0.0, nv, true);
            println!("{} portante {:+5.0} Hz: prima {:+6.1}  dopo {:+6.1}  con vicino prima {:+6.1} dopo {:+6.1}", mode.name(), off, before - 1000.0, after - 1000.0, near0 - 1000.0, near - 1000.0);
        }
    }
}

#[test]
fn l_afc_arriva_sulla_portante_entro_il_limite() {
    for (mode, offs) in [(PskMode::Psk31, [-15.0, -10.0, -5.0, 5.0, 10.0, 15.0]), (PskMode::Psk63, [-25.0, -15.0, -10.0, 10.0, 15.0, 25.0])] {
        for off in offs {
            let f = converge(mode, off, -5.0, 0.0, true);
            assert!((f - 1000.0 - off).abs() < 2.0, "{} portante {off:+}: ascolto a {:+.1}", mode.name(), f - 1000.0);
        }
    }
}

#[test]
fn finestra_pulita_con_la_bandierina_fuori_e_nel_rumore() {
    // PSK31 con la portante 12 Hz fuori: prima 0-5% di caratteri giusti
    let c = Caso { mode: PskMode::Psk31, snr_db: 0.0, offset: 12.0, vicino: 50.0 };
    let (a, spans) = audio(&c, 21);
    let r = conta(&finestra(&c, &a, Some(SQUELCH_DEFAULT), 15.0, true), &spans, &c);
    let (ok, sp) = r.pct();
    assert!(ok >= 90.0 && sp <= 2.0, "{r:?}");
    // 20 s di solo rumore: niente caratteri
    let mut n = vec![0.0_f32; 20 * RATE];
    aggiungi_rumore(&mut n, 0.1, &mut Rng(5));
    let got = finestra(&c, &n, Some(SQUELCH_DEFAULT), 15.0, true);
    assert!(got.iter().all(|(_, ch)| ch.is_ascii_whitespace()), "{:?}", got.iter().map(|(_, c)| *c as char).collect::<String>());
}

/// Stazione scelta con la bandierina fuori di `off` Hz, deriva lenta di `drift` Hz in tutto, tre testi con pause di 2 s; stazione vicina
/// a `near` Hz, `near_db` dB piu' forte, sempre in trasmissione. S/N 0 dB in 2,5 kHz. Restituisce la frequenza ascoltata ogni blocco
/// (meno la portante del momento), i salti (passi oltre 2 Hz fra due blocchi) e l'errore alla fine di ogni testo.
fn tenuta(mode: PskMode, off: f32, drift: f64, near: f32, near_db: f32) -> (Vec<f32>, usize, Vec<f32>) {
    let f0 = 1000.0 + off;
    let wave = modulate(TESTO.as_bytes(), f0, mode, AMP);
    let pausa = 2 * RATE;
    let testa = RATE;
    let mut a = vec![0.0_f32; testa + 3 * (wave.len() + pausa)];
    let mut ends = Vec::new();
    for i in 0..3 {
        let s = testa + i * (wave.len() + pausa);
        for (x, w) in a[s..].iter_mut().zip(&wave) {
            *x += w;
        }
        ends.push(s + wave.len() - RATE / 2);
    }
    // deriva: frequenza che cresce piano (fase integrata)
    let total = a.len() as f64;
    let rate = RATE as f64;
    let a = guadagno_complesso(&a, |n| {
        let t = n as f64;
        let ph = 2.0 * std::f64::consts::PI * drift * t * t / (2.0 * total * rate);
        (ph.cos(), ph.sin())
    });
    let mut a = a;
    if near != 0.0 {
        let txt = b"QRZ DE K1ABC K1ABC K1ABC QRZ QRZ QRZ DE K1ABC K1ABC K1ABC QRZ QRZ QRZ DE K1ABC K1ABC ";
        let other = modulate(txt.repeat(4).as_slice(), f0 + near, mode, AMP * 10f32.powf(near_db / 20.0));
        for (x, w) in a.iter_mut().zip(&other) {
            *x += w;
        }
    }
    let sigma = (f64::from(AMP) * f64::from(AMP) / 2.0 * 6000.0 / 2500.0).sqrt();
    aggiungi_rumore(&mut a, sigma, &mut Rng(17));
    let limit = limite(mode);
    let mut rx = PskDemodulator::new(mode, 1000.0);
    rx.set_afc(true);
    rx.set_squelch(Some(SQUELCH_DEFAULT));
    rx.set_acquire(Some(limit));
    let mut guard = AfcGuard::new(1000.0, limit);
    let mut track = Vec::new();
    let mut errs = Vec::new();
    let carrier = |n: usize| f0 + (drift * n as f64 / total) as f32;
    for (i, b) in a.chunks(BLOCK).enumerate() {
        rx.process(b);
        if let Some(hz) = rx.take_acquired() {
            guard.retune(hz);
        }
        if let Some(hz) = guard.step(rx.frequency(), Some(rx.metric()), (25.0, 12.0), rx.signal().1) {
            rx.steer(hz);
        }
        let n = (i + 1) * BLOCK;
        track.push(rx.frequency() - carrier(n));
        if ends.iter().any(|&e| e >= n - BLOCK && e < n) {
            errs.push(rx.frequency() - carrier(n));
        }
    }
    let jumps = track.windows(2).filter(|w| (w[1] - w[0]).abs() > 2.0).count();
    (track, jumps, errs)
}

#[test]
#[ignore]
fn tabella_tenuta() {
    for mode in [PskMode::Psk31, PskMode::Psk63] {
        for (off, near) in [(0.0, 0.0), (8.0, 40.0), (-12.0, 40.0), (12.0, 60.0), (-5.0, -60.0), (15.0, -40.0)] {
            let (track, jumps, errs) = tenuta(mode, off, 4.0, near, 3.0);
            let every: Vec<String> = track.iter().step_by(25).map(|e| format!("{e:+.1}")).collect();
            println!("{} bandierina {:+3.0} vicino {:+3.0} (+3 dB): salti {jumps}, errore a fine testi {errs:.1?}
   ogni secondo: {}", mode.name(), -off, near, every.join(" "));
        }
    }
}

#[test]
fn l_afc_resta_sulla_stazione_scelta_con_una_vicina_piu_forte() {
    for mode in [PskMode::Psk31, PskMode::Psk63] {
        for (off, near) in [(8.0, 40.0), (-12.0, 60.0), (10.0, -60.0)] {
            let (_, jumps, errs) = tenuta(mode, off, 4.0, near, 3.0);
            assert!(jumps <= 1, "{} {off} {near}: salti {jumps}", mode.name());
            assert!(errs.iter().all(|e| e.abs() < 2.0), "{} {off} {near}: {errs:?}", mode.name());
        }
    }
}
