//! Notch e passabanda «a coltello» davanti ai ricevitori PSK, RTTY e CW, e i limiti dell'AFC.
//!
//! Il filtro lavora in frequenza: STFT a 4096 punti (2,93 Hz per riga a 12 kHz) con finestra Blackman-Harris a quattro termini e passo di un
//! quarto, sommata senza finestra di sintesi (la Blackman-Harris periodica sommata a passi di N/4 da' una costante, quindi senza maschera
//! l'audio si ricostruisce com'era). La maschera vale 1 dove si ascolta e `FLOOR` (−80 dB) dentro i notch e fuori dal passabanda, con fianchi
//! a coseno rialzato di `EDGE_BINS` righe: i lobi laterali della finestra sono sotto i −90 dB, quindi un tono dentro un notch sparisce
//! davvero e uno a qualche decina di Hz passa intatto. Ritardo: da 3/4 a una finestra intera (0,26-0,34 s), per i modi testuali indifferente.
//!
//! Senza notch ne' passabanda il filtro non c'e': l'audio va ai ricevitori senza essere toccato (vedi [`RxFilters::process`]).

use std::sync::Arc;

use rustfft::num_complex::Complex32;
use rustfft::{Fft, FftPlanner};
use serde::{Deserialize, Serialize};

use crate::RATE;

/// Punti della trasformata (a `RATE`).
pub const FFT_LEN: usize = 4096;
const HOP: usize = FFT_LEN / 4;
/// Fianco a coseno rialzato della maschera, in righe della trasformata.
const EDGE_BINS: f32 = 3.0;
/// Il fondo della maschera: −80 dB.
const FLOOR: f32 = 1e-4;
/// Blackman-Harris a quattro termini.
const BH: [f32; 4] = [0.35875, 0.48829, 0.14128, 0.01168];

/// Larghezza di un notch: di base e limiti (Hz).
pub const NOTCH_WIDTH_DEFAULT: f32 = 30.0;
pub const NOTCH_WIDTH_MIN: f32 = 10.0;
pub const NOTCH_WIDTH_MAX: f32 = 100.0;
/// Quanti notch al piu' (solo un tetto di sicurezza per le rotte: nella pratica non si arriva mai).
pub const NOTCH_MAX: usize = 256;

/// Un notch: frequenza centrale e larghezza (Hz) della zona tolta del tutto; i fianchi stanno fuori.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Notch {
    pub hz: f32,
    #[serde(default = "default_width")]
    pub width_hz: f32,
}

fn default_width() -> f32 {
    NOTCH_WIDTH_DEFAULT
}

impl Notch {
    pub fn is_valid(&self) -> bool {
        self.hz.is_finite() && (50.0..=RATE / 2.0 - 50.0).contains(&self.hz) && self.width_hz.is_finite() && (NOTCH_WIDTH_MIN..=NOTCH_WIDTH_MAX).contains(&self.width_hz)
    }
}

/// Il guadagno di un fianco: 0 a distanza `d` ≤ 0 dalla zona tolta, 1 oltre `EDGE_BINS` righe.
fn edge(d: f32, edge_hz: f32) -> f32 {
    if d <= 0.0 {
        FLOOR
    } else if d >= edge_hz {
        1.0
    } else {
        FLOOR + (1.0 - FLOOR) * 0.5 * (1.0 - (std::f32::consts::PI * d / edge_hz).cos())
    }
}

/// Il guadagno della maschera alla frequenza `f`: i notch e, se c'e', il passabanda `(basso, alto)`.
pub fn gain(f: f32, notches: &[Notch], pass: Option<(f32, f32)>) -> f32 {
    let edge_hz = EDGE_BINS * RATE / FFT_LEN as f32;
    let mut g = 1.0_f32;
    for n in notches {
        g *= edge((f - n.hz).abs() - n.width_hz / 2.0, edge_hz);
    }
    if let Some((lo, hi)) = pass {
        let out = if f < lo { lo - f } else if f > hi { f - hi } else { 0.0 };
        g *= edge(edge_hz - out, edge_hz);
    }
    g.max(FLOOR)
}

/// Un filtro in frequenza a flusso (STFT, overlap-add). L'uscita esce a blocchi di `HOP` campioni: puo' essere piu' corta o piu' lunga
/// dell'ingresso di ogni chiamata, il totale e' lo stesso meno il ritardo.
pub struct SpectralFilter {
    fwd: Arc<dyn Fft<f32>>,
    inv: Arc<dyn Fft<f32>>,
    window: Vec<f32>,
    mask: Vec<f32>,
    frame: Vec<f32>,
    pending: Vec<f32>,
    acc: Vec<f32>,
    buf: Vec<Complex32>,
    scratch: Vec<Complex32>,
    scale: f32,
}

impl SpectralFilter {
    pub fn new() -> SpectralFilter {
        let mut planner = FftPlanner::new();
        let fwd = planner.plan_fft_forward(FFT_LEN);
        let inv = planner.plan_fft_inverse(FFT_LEN);
        let len = fwd.get_inplace_scratch_len().max(inv.get_inplace_scratch_len());
        let w = |i: usize| {
            let x = 2.0 * std::f32::consts::PI * i as f32 / FFT_LEN as f32;
            BH[0] - BH[1] * x.cos() + BH[2] * (2.0 * x).cos() - BH[3] * (3.0 * x).cos()
        };
        SpectralFilter {
            fwd,
            inv,
            window: (0..FFT_LEN).map(w).collect(),
            mask: vec![1.0; FFT_LEN],
            frame: vec![0.0; FFT_LEN],
            pending: Vec::with_capacity(HOP),
            acc: vec![0.0; FFT_LEN],
            buf: vec![Complex32::new(0.0, 0.0); FFT_LEN],
            scratch: vec![Complex32::new(0.0, 0.0); len],
            // la Blackman-Harris periodica sommata a passi di N/4 vale 4·a0; la trasformata inversa di rustfft non divide per N
            scale: 1.0 / (FFT_LEN as f32 * 4.0 * BH[0]),
        }
    }

    /// La maschera dai notch e dal passabanda (simmetrica: l'audio e' reale). Non tocca lo stato: cambiarla non fa click.
    pub fn set_mask(&mut self, notches: &[Notch], pass: Option<(f32, f32)>) {
        let bin = RATE / FFT_LEN as f32;
        for k in 0..=FFT_LEN / 2 {
            let g = gain(k as f32 * bin, notches, pass);
            self.mask[k] = g;
            self.mask[(FFT_LEN - k) % FFT_LEN] = g;
        }
    }

    pub fn process(&mut self, input: &[f32], out: &mut Vec<f32>) {
        for &x in input {
            self.pending.push(x);
            if self.pending.len() == HOP {
                self.frame.copy_within(HOP.., 0);
                self.frame[FFT_LEN - HOP..].copy_from_slice(&self.pending);
                self.pending.clear();
                self.step(out);
            }
        }
    }

    fn step(&mut self, out: &mut Vec<f32>) {
        for ((b, &x), &w) in self.buf.iter_mut().zip(&self.frame).zip(&self.window) {
            *b = Complex32::new(x * w, 0.0);
        }
        self.fwd.process_with_scratch(&mut self.buf, &mut self.scratch);
        for (b, &m) in self.buf.iter_mut().zip(&self.mask) {
            *b *= m;
        }
        self.inv.process_with_scratch(&mut self.buf, &mut self.scratch);
        for (a, b) in self.acc.iter_mut().zip(&self.buf) {
            *a += b.re * self.scale;
        }
        out.extend_from_slice(&self.acc[..HOP]);
        self.acc.copy_within(HOP.., 0);
        self.acc[FFT_LEN - HOP..].fill(0.0);
    }
}

impl Default for SpectralFilter {
    fn default() -> Self {
        SpectralFilter::new()
    }
}

/// Il passabanda di base di ogni modo (Hz): PSK tre volte il baud (almeno 60 Hz), RTTY lo spostamento piu' due volte il baud, CW 80 Hz.
pub fn default_passband(mode: crate::DigiMode, shift_hz: f32, rtty_baud: f32) -> f32 {
    match mode {
        crate::DigiMode::Psk31 => 3.0 * 31.25,
        crate::DigiMode::Psk63 => 3.0 * 62.5,
        crate::DigiMode::Rtty => shift_hz + 2.0 * rtty_baud,
        crate::DigiMode::Cw => 80.0,
    }
}

/// I filtri della ricezione: i notch per tutta la banda e per il ricevitore, il passabanda solo per il ricevitore.
#[derive(Default)]
pub struct RxFilters {
    notches: Vec<Notch>,
    pass: Option<(f32, f32)>,
    /// Il filtro dei soli notch (va a tutta la banda e, senza passabanda, anche al ricevitore).
    band: Option<SpectralFilter>,
    /// Il filtro dei notch piu' il passabanda (solo al ricevitore).
    rx: Option<SpectralFilter>,
    band_out: Vec<f32>,
    rx_out: Vec<f32>,
}

impl RxFilters {
    pub fn new() -> RxFilters {
        RxFilters::default()
    }

    pub fn notches(&self) -> &[Notch] {
        &self.notches
    }

    pub fn passband(&self) -> Option<(f32, f32)> {
        self.pass
    }

    /// Notch e passabanda nuovi. I filtri gia' accesi tengono lo stato (nessun buco nell'audio); quelli che non servono piu' si spengono.
    pub fn configure(&mut self, notches: &[Notch], pass: Option<(f32, f32)>) {
        if self.notches == notches && self.pass == pass {
            return;
        }
        self.notches = notches.to_vec();
        self.pass = pass;
        if notches.is_empty() {
            self.band = None;
        } else {
            self.band.get_or_insert_with(SpectralFilter::new).set_mask(notches, None);
        }
        if pass.is_none() {
            self.rx = None;
        } else {
            self.rx.get_or_insert_with(SpectralFilter::new).set_mask(notches, pass);
        }
    }

    /// L'audio per tutta la banda e per il ricevitore. Senza filtri e' lo stesso blocco d'ingresso, non toccato.
    pub fn process<'a>(&'a mut self, audio: &'a [f32]) -> (&'a [f32], &'a [f32]) {
        if let Some(f) = self.band.as_mut() {
            self.band_out.clear();
            f.process(audio, &mut self.band_out);
        }
        if let Some(f) = self.rx.as_mut() {
            self.rx_out.clear();
            f.process(audio, &mut self.rx_out);
        }
        let band: &[f32] = if self.band.is_some() { &self.band_out } else { audio };
        let rx: &[f32] = if self.rx.is_some() { &self.rx_out } else { band };
        (band, rx)
    }
}

/// L'AFC vincolato: la frequenza su cui si ascolta resta entro `limit` Hz da quella scelta, dentro il passabanda e fuori dai notch, e si ferma
/// quando il segnale sparisce (con isteresi). Si applica dopo ogni blocco del ricevitore: se la sua AFC e' uscita, la riporta indietro.
#[derive(Clone, Debug)]
pub struct AfcGuard {
    base: f32,
    limit: f32,
    notches: Vec<Notch>,
    pass: Option<(f32, f32)>,
    present: bool,
    last_good: f32,
}

impl AfcGuard {
    pub fn new(base: f32, limit: f32) -> AfcGuard {
        AfcGuard { base, limit, notches: Vec::new(), pass: None, present: false, last_good: base }
    }

    pub fn set_filters(&mut self, notches: &[Notch], pass: Option<(f32, f32)>) {
        self.notches = notches.to_vec();
        self.pass = pass;
    }

    pub fn present(&self) -> bool {
        self.present
    }

    /// La frequenza `f` e' in un notch (con i suoi fianchi).
    fn in_notch(&self, f: f32) -> bool {
        let edge_hz = EDGE_BINS * RATE / FFT_LEN as f32;
        self.notches.iter().any(|n| (f - n.hz).abs() < n.width_hz / 2.0 + edge_hz)
    }

    fn allowed(&self, f: f32) -> f32 {
        let (mut lo, mut hi) = (self.base - self.limit, self.base + self.limit);
        if let Some((plo, phi)) = self.pass {
            lo = lo.max(plo);
            hi = hi.min(phi);
        }
        if lo > hi { self.base } else { f.clamp(lo, hi) }
    }

    /// Dopo un blocco: `f` e' la frequenza a cui l'AFC del ricevitore e' arrivato, `level` la misura del segnale (qualita' o S/N) con le soglie
    /// d'isteresi `(accendi, spegni)` e `open` lo squelch. Risponde la frequenza su cui riportare il ricevitore se va corretta.
    pub fn step(&mut self, f: f32, level: Option<f32>, (on, off): (f32, f32), open: bool) -> Option<f32> {
        let level = level.unwrap_or(f32::NEG_INFINITY);
        self.present = open && if self.present { level >= off } else { level >= on };
        let target = if !self.present {
            // segnale sparito: l'AFC si ferma dov'era, non insegue il rumore ne' la stazione vicina
            self.last_good
        } else {
            let g = self.allowed(f);
            if self.in_notch(g) { self.last_good } else { g }
        };
        let target = self.allowed(target);
        if !self.in_notch(target) {
            self.last_good = target;
        }
        ((target - f).abs() > 1e-3).then_some(target)
    }
}

/// Il filtro su tutto un segnale, a blocchi di 40 ms come arriva dalla scheda.
#[cfg(test)]
fn run(filter: &mut SpectralFilter, input: &[f32]) -> Vec<f32> {
    let mut out = Vec::new();
    for chunk in input.chunks(480) {
        filter.process(chunk, &mut out);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tone(hz: f32, secs: f32) -> Vec<f32> {
        (0..(RATE * secs) as usize).map(|i| (2.0 * std::f32::consts::PI * hz * i as f32 / RATE).sin() * 0.5).collect()
    }

    fn rms(x: &[f32]) -> f32 {
        (x.iter().map(|v| v * v).sum::<f32>() / x.len() as f32).sqrt()
    }

    /// Attenuazione (dB) di un tono a `hz` con i notch dati, misurata a regime (dopo le prime due finestre).
    fn attenuation(hz: f32, notches: &[Notch], pass: Option<(f32, f32)>) -> f32 {
        let x = tone(hz, 4.0);
        let mut f = SpectralFilter::new();
        f.set_mask(notches, pass);
        let y = run(&mut f, &x);
        let skip = 2 * FFT_LEN;
        20.0 * (rms(&x[skip..]) / rms(&y[skip..]).max(1e-12)).log10()
    }

    #[test]
    fn un_tono_dentro_il_notch_sparisce_e_uno_a_due_larghezze_passa() {
        let notch = [Notch { hz: 1000.0, width_hz: NOTCH_WIDTH_DEFAULT }];
        for hz in [1000.0, 1007.0, 993.5] {
            let a = attenuation(hz, &notch, None);
            println!("tono a {hz} Hz nel notch: {a:.1} dB");
            assert!(a >= 50.0, "tono a {hz} Hz attenuato solo di {a:.1} dB");
        }
        for hz in [1000.0 + 2.0 * NOTCH_WIDTH_DEFAULT, 1000.0 - 2.0 * NOTCH_WIDTH_DEFAULT] {
            let a = attenuation(hz, &notch, None);
            println!("tono a {hz} Hz fuori dal notch: {a:.3} dB");
            assert!(a.abs() < 1.0, "tono a {hz} Hz cambiato di {a:.3} dB");
        }
    }

    #[test]
    fn notch_stretti_e_larghi_e_piu_notch_insieme() {
        let notches = [Notch { hz: 700.0, width_hz: 10.0 }, Notch { hz: 1500.0, width_hz: 100.0 }, Notch { hz: 2200.0, width_hz: 30.0 }];
        for hz in [1500.0, 1540.0, 2200.0] {
            let a = attenuation(hz, &notches, None);
            println!("tono a {hz} Hz: {a:.1} dB");
            assert!(a >= 50.0, "{hz} Hz: {a:.1} dB");
        }
        let a = attenuation(1000.0, &notches, None);
        assert!(a.abs() < 0.1, "1000 Hz: {a:.3} dB");
    }

    #[test]
    fn il_passabanda_lascia_il_segnale_e_toglie_il_vicino() {
        let pass = Some((960.0, 1040.0));
        let dentro = attenuation(1000.0, &[], pass);
        let fuori = attenuation(1100.0, &[], pass);
        println!("passabanda: dentro {dentro:.3} dB, a 100 Hz {fuori:.1} dB");
        assert!(dentro.abs() < 0.1);
        assert!(fuori >= 50.0);
    }

    #[test]
    fn senza_filtri_l_audio_resta_identico_bit_per_bit() {
        let x: Vec<f32> = (0..20_000).map(|i| (i as f32 * 0.37).sin() * 0.3 + (i % 7) as f32 * 1e-3).collect();
        let mut f = RxFilters::new();
        for chunk in x.chunks(333) {
            let (band, rx) = f.process(chunk);
            assert_eq!(band.as_ptr(), chunk.as_ptr());
            assert_eq!(rx.as_ptr(), chunk.as_ptr());
            assert!(band.iter().zip(chunk).all(|(a, b)| a.to_bits() == b.to_bits()));
        }
        // anche dopo averli accesi e spenti
        f.configure(&[Notch { hz: 1000.0, width_hz: 30.0 }], Some((900.0, 1100.0)));
        let _ = f.process(&x[..1000]);
        f.configure(&[], None);
        let (band, rx) = f.process(&x);
        assert_eq!(band.as_ptr(), x.as_ptr());
        assert_eq!(rx.as_ptr(), x.as_ptr());
    }

    #[test]
    fn senza_maschera_lo_stft_ricostruisce_l_audio() {
        let x = tone(1234.5, 2.0);
        let mut f = SpectralFilter::new();
        let y = run(&mut f, &x);
        // il ritardo e' di FFT_LEN − HOP campioni
        let d = FFT_LEN - HOP;
        let err = y[FFT_LEN..].iter().zip(&x[FFT_LEN - d..]).map(|(a, b)| (a - b).abs()).fold(0.0_f32, f32::max);
        assert!(err < 1e-4, "errore {err}");
    }

    #[test]
    fn l_afc_non_esce_dai_limiti_e_non_entra_nei_notch() {
        let mut g = AfcGuard::new(1000.0, 15.0);
        g.set_filters(&[Notch { hz: 1020.0, width_hz: 10.0 }], None);
        let th = (25.0, 12.0);
        // segnale presente, l'AFC del ricevitore vuole andare a 1040: si ferma prima del notch (ultimo punto buono)
        let mut f = 1000.0;
        for step in 0..60 {
            let wanted = 1000.0 + step as f32;
            f = g.step(wanted, Some(50.0), th, true).unwrap_or(wanted);
            assert!((985.0..=1015.0).contains(&f), "uscito: {f}");
            assert!(!g.in_notch(f), "nel notch: {f}");
        }
        // verso il basso: limite a 985
        for step in 0..60 {
            let wanted = 1000.0 - step as f32;
            f = g.step(wanted, Some(50.0), th, true).unwrap_or(wanted);
            assert!(f >= 985.0, "sotto il limite: {f}");
        }
        assert!((f - 985.0).abs() < 1e-3);
        // il segnale sparisce: ferma dov'era anche se il ricevitore va altrove; con isteresi (sopra la soglia di spegnimento resta acceso)
        assert!(g.step(990.0, Some(20.0), th, true).is_some_and(|x| (x - 990.0).abs() < 1e-3) || g.present());
        let held = g.step(995.0, Some(5.0), th, true).unwrap_or(995.0);
        let again = g.step(1003.0, Some(18.0), th, true).unwrap_or(1003.0);
        assert!((again - held).abs() < 1e-3, "senza segnale si e' mosso: {held} → {again}");
        assert!(!g.present(), "riacceso sotto la soglia d'accensione");
        // passabanda piu' stretto del limite
        g.set_filters(&[], Some((995.0, 1005.0)));
        let f = g.step(1010.0, Some(50.0), th, true).unwrap_or(1010.0);
        assert!((f - 1005.0).abs() < 1e-3, "fuori dal passabanda: {f}");
    }

    #[test]
    fn l_afc_vero_del_psk_resta_nei_limiti() {
        // PSK31 a 1030 Hz, frequenza scelta 1000, limite 15: il ricevitore corregge ma la guardia non lo lascia oltre 1015
        let wave = crate::psk::modulate(b"CQ CQ DE IU3ABC IU3ABC PSE K", 1030.0, crate::PskMode::Psk31, 0.5);
        let mut rx = crate::PskDemodulator::new(crate::PskMode::Psk31, 1000.0);
        rx.set_afc(true);
        let mut g = AfcGuard::new(1000.0, 15.0);
        for chunk in wave.chunks(240) {
            let _ = rx.process(chunk);
            if let Some(f) = g.step(rx.frequency(), Some(rx.metric()), (25.0, 12.0), true) {
                rx.steer(f);
            }
            assert!((985.0..=1015.0).contains(&rx.frequency()), "uscito: {}", rx.frequency());
        }
    }
}
