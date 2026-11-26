//! QPSK31: PSK a quattro fasi a 31,25 baud con codice convoluzionale (vincolo 5, velocita' 1/2) davanti alla varicode del PSK31.
//!
//! Le convenzioni sono quelle di fldigi 4.2.13 (`src/psk/psk.cxx`, Dave Freese W1HKJ e contributori; GPL 3, copia in
//! `terze-parti/fldigi`), studiate e riscritte qui (niente codice copiato):
//! - codice convoluzionale con i polinomi 0x17 e 0x19: il registro prende il bit nuovo nel bit meno significativo, l'uscita ha la parita'
//!   del primo polinomio nel bit 0 e del secondo nel bit 1 (`encoder::init` di `src/filters/viterbi.cxx`);
//! - il simbolo `s` (0-3) fa girare la fase di 180° − 90°·s in banda laterale superiore e di 180° + 90°·s con l'inversione (LSB); la fase
//!   e' quella del complesso I + jQ trasmesso come I·cos + Q·sin. Lo zero ripetuto (registro vuoto) da' le inversioni di 180°: il
//!   preambolo e la coda sono inversioni come nel PSK31;
//! - un carattere e' il suo varicode seguito da due zeri; la coda manda zeri per svuotare il codificatore ([`TAIL_ZEROS`]).
//!
//! La ricezione usa il ricevitore del PSK31 (`psk.rs`) fino al simbolo; qui ci sono il codificatore, la decisione «soft» dei quattro
//! simboli e il decodificatore di Viterbi (16 stati, nostro).

use std::f64::consts::PI;

/// Polinomi del codice convoluzionale (fldigi `POLY1`, `POLY2`), vincolo 5.
const POLY1: u32 = 0x17;
const POLY2: u32 = 0x19;
const K: u32 = 5;
const STATES: usize = 1 << (K - 1);
/// Profondita' della risalita del Viterbi (simboli): piu' di cinque volte il vincolo.
const TRACEBACK: usize = 40;

/// L'uscita (due bit) del codificatore per il registro `reg` (bit nuovo nel bit 0).
fn output(reg: u32) -> u8 {
    let p = |x: u32| (x.count_ones() & 1) as u8;
    p(reg & POLY1) | (p(reg & POLY2) << 1)
}

/// Il codificatore convoluzionale.
#[derive(Clone, Default)]
pub struct ConvEncoder {
    reg: u32,
}

impl ConvEncoder {
    pub fn encode(&mut self, bit: bool) -> u8 {
        self.reg = ((self.reg << 1) | u32::from(bit)) & ((1 << K) - 1);
        output(self.reg)
    }
}

/// Il giro di fase (radianti, nel senso del complesso I + jQ) del simbolo `s`; `reverse`: inversione di banda laterale.
pub fn phase_step(s: u8, reverse: bool) -> f64 {
    let d = PI - PI / 2.0 * f64::from(s & 3);
    if reverse { -d } else { d }
}

/// Le metriche dei quattro simboli dal prodotto `w` = simbolo · coniugato del simbolo prima (nel senso I + jQ): la correlazione con il
/// giro atteso di ciascuno.
pub fn symbol_metrics(w: (f32, f32), reverse: bool) -> [f32; 4] {
    let mut m = [0.0_f32; 4];
    for (s, v) in m.iter_mut().enumerate() {
        let (sin, cos) = phase_step(s as u8, reverse).sin_cos();
        *v = w.0 * cos as f32 + w.1 * sin as f32;
    }
    m
}

/// Decodificatore di Viterbi a decisioni «soft» per il codice (0x17, 0x19): un bit esce `TRACEBACK` simboli dopo.
pub struct ConvViterbi {
    metric: [f32; STATES],
    /// Per ogni simbolo, il bit uscito dal registro per arrivare a ciascuno stato (bit 4 del registro a 5 bit).
    hist: Vec<u16>,
    pos: usize,
    filled: usize,
}

impl Default for ConvViterbi {
    fn default() -> Self {
        Self::new()
    }
}

impl ConvViterbi {
    pub fn new() -> Self {
        let mut metric = [-1e9_f32; STATES];
        metric[0] = 0.0;
        Self { metric, hist: vec![0; TRACEBACK], pos: 0, filled: 0 }
    }

    pub fn reset(&mut self) {
        *self = Self::new();
    }

    /// Un simbolo (le metriche dei quattro valori, piu' alto = piu' probabile); restituisce il bit deciso di `TRACEBACK` simboli prima.
    pub fn push(&mut self, m: [f32; 4]) -> Option<bool> {
        let mut next = [f32::MIN; STATES];
        let mut dec = 0u16;
        for (n, slot) in next.iter_mut().enumerate() {
            // stato n = ultimi 4 bit (nuovo nel bit 0); il registro a 5 bit e' (vecchio << 4) | n
            let mut best = f32::MIN;
            let mut from_one = false;
            for old in 0..2u32 {
                let reg = (old << 4) | n as u32;
                let prev = (reg >> 1) as usize;
                let v = self.metric[prev] + m[usize::from(output(reg))];
                if v > best {
                    best = v;
                    from_one = old == 1;
                }
            }
            *slot = best;
            if from_one {
                dec |= 1 << n;
            }
        }
        // normalizza per non perdere precisione
        let top = next.iter().copied().fold(f32::MIN, f32::max);
        for v in next.iter_mut() {
            *v -= top;
        }
        self.metric = next;
        self.hist[self.pos] = dec;
        self.pos = (self.pos + 1) % TRACEBACK;
        self.filled += 1;
        if self.filled < TRACEBACK {
            return None;
        }
        Some(self.trace(TRACEBACK).first().copied().unwrap_or(false))
    }

    /// Risale dallo stato migliore per `depth` simboli; restituisce i bit dal piu' vecchio al piu' nuovo.
    fn trace(&self, depth: usize) -> Vec<bool> {
        let mut state = self.metric.iter().enumerate().fold((0usize, f32::MIN), |b, (i, &v)| if v > b.1 { (i, v) } else { b }).0;
        let mut bits = Vec::with_capacity(depth);
        for k in 0..depth {
            let idx = (self.pos + TRACEBACK - 1 - k) % TRACEBACK;
            bits.push(state & 1 == 1);
            let old = (self.hist[idx] >> state) & 1;
            state = ((usize::from(old) << 4) | state) >> 1;
        }
        bits.reverse();
        bits
    }

    /// I bit ancora in sospeso (alla fine del segnale), poi riparte da capo.
    pub fn flush(&mut self) -> Vec<bool> {
        let bits = self.trace(self.filled.min(TRACEBACK - 1));
        self.reset();
        bits
    }
}

/// Zeri della coda: fldigi ne manda 32, ma il suo decodificatore di Viterbi decide un bit 60 simboli dopo (a gruppi di 8): con 32 l'ultimo
/// carattere resta nel decodificatore di chi riceve con fldigi (prova `qpsk_oracolo`). 72 zeri lo fanno uscire.
pub const TAIL_ZEROS: usize = 72;

/// Il trasmettitore QPSK31 a pezzi (preambolo, caratteri, attesa, coda) a 12 kHz: transizioni a coseno rialzato fra un simbolo complesso
/// e il successivo, come fldigi (`psk::tx_symbol`).
pub struct QpskTx {
    enc: ConvEncoder,
    reverse: bool,
    n: usize,
    step: f64,
    phase: f64,
    prev: (f64, f64),
    amp: f32,
}

impl QpskTx {
    pub fn new(freq_hz: f32, symbol_samples: usize, rate: f32, amp: f32, reverse: bool) -> Self {
        Self {
            enc: ConvEncoder::default(),
            reverse,
            n: symbol_samples,
            step: 2.0 * PI * f64::from(freq_hz) / f64::from(rate),
            phase: 0.0,
            prev: (0.0, 0.0),
            amp,
        }
    }

    /// Un simbolo verso il punto `next` (complesso; (0, 0) = silenzio).
    fn emit(&mut self, next: (f64, f64), out: &mut Vec<f32>) {
        for i in 0..self.n {
            let w = 0.5 * (1.0 + (PI * i as f64 / self.n as f64).cos());
            let ival = self.prev.0 * w + next.0 * (1.0 - w);
            let qval = self.prev.1 * w + next.1 * (1.0 - w);
            out.push(self.amp * (ival * self.phase.cos() + qval * self.phase.sin()) as f32);
            self.phase += self.step;
            if self.phase > 2.0 * PI {
                self.phase -= 2.0 * PI;
            }
        }
        self.prev = next;
    }

    /// Un simbolo `s` (dal silenzio si parte dalla fase zero).
    fn symbol(&mut self, s: u8, out: &mut Vec<f32>) {
        let next = if self.prev == (0.0, 0.0) {
            (1.0, 0.0)
        } else {
            let (sin, cos) = phase_step(s, self.reverse).sin_cos();
            (self.prev.0 * cos - self.prev.1 * sin, self.prev.0 * sin + self.prev.1 * cos)
        };
        self.emit(next, out);
    }

    fn bit(&mut self, b: bool, out: &mut Vec<f32>) {
        let s = self.enc.encode(b);
        self.symbol(s, out);
    }

    /// Preambolo: `len` inversioni (il simbolo 0 senza passare dal codificatore, come fldigi).
    pub fn preamble(&mut self, len: usize) -> Vec<f32> {
        let mut out = Vec::new();
        for _ in 0..len {
            self.symbol(0, &mut out);
        }
        out
    }

    pub fn char(&mut self, c: u8) -> Vec<f32> {
        let mut out = Vec::new();
        for ch in crate::varicode::encode(c).chars() {
            self.bit(ch == '1', &mut out);
        }
        self.bit(false, &mut out);
        self.bit(false, &mut out);
        out
    }

    /// Attesa: quattro zeri (inversioni).
    pub fn idle(&mut self) -> Vec<f32> {
        let mut out = Vec::new();
        for _ in 0..4 {
            self.bit(false, &mut out);
        }
        out
    }

    /// Coda: `len` zeri che svuotano il codificatore, poi la discesa al silenzio.
    pub fn postamble(&mut self, len: usize) -> Vec<f32> {
        let mut out = Vec::new();
        for _ in 0..len {
            self.bit(false, &mut out);
        }
        self.emit((0.0, 0.0), &mut out);
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn il_viterbi_decodifica_il_codificatore_senza_rumore() {
        let mut seed = 1u32;
        let bits: Vec<bool> = (0..400)
            .map(|_| {
                seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
                seed >> 31 == 1
            })
            .collect();
        let mut enc = ConvEncoder::default();
        let mut dec = ConvViterbi::new();
        let mut out = Vec::new();
        for &b in &bits {
            let s = enc.encode(b);
            let mut m = [-1.0_f32; 4];
            m[usize::from(s)] = 1.0;
            out.extend(dec.push(m));
        }
        out.extend(dec.flush());
        assert_eq!(out, bits);
    }

    #[test]
    fn il_viterbi_corregge_simboli_sbagliati() {
        let bits: Vec<bool> = (0..300).map(|i| (i * 7 + i / 3) % 5 < 2).collect();
        let mut enc = ConvEncoder::default();
        let mut dec = ConvViterbi::new();
        let mut out = Vec::new();
        for (i, &b) in bits.iter().enumerate() {
            let mut s = enc.encode(b);
            if i % 17 == 5 {
                s ^= 1;
            }
            let mut m = [-1.0_f32; 4];
            m[usize::from(s)] = 1.0;
            out.extend(dec.push(m));
        }
        out.extend(dec.flush());
        assert_eq!(out, bits);
    }

    #[test]
    fn le_metriche_riconoscono_il_giro_di_fase() {
        for rev in [false, true] {
            for s in 0..4u8 {
                let (sin, cos) = phase_step(s, rev).sin_cos();
                let m = symbol_metrics((cos as f32, sin as f32), rev);
                let best = (0..4).max_by(|&a, &b| m[a].total_cmp(&m[b])).unwrap_or(9);
                assert_eq!(best, usize::from(s));
            }
        }
    }
}
