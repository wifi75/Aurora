//! PSK31 e PSK63: modulazione di fase binaria con inversioni dolci (inviluppo a coseno rialzato), codifica differenziale e varicode.
//!
//! Il bit 0 inverte la fase, il bit 1 la lascia com'è. Un carattere è il suo varicode seguito da due zeri. Il ricevitore porta il segnale
//! a banda base con la frequenza scelta, cerca l'istante dei simboli dove l'ampiezza scende a zero (le inversioni) e decide ogni bit dal
//! confronto di fase col simbolo prima.

use crate::varicode;
use std::f64::consts::PI;

/// Frequenza di campionamento dell'audio dei modi testuali.
pub const RATE: f32 = 12000.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PskMode {
    Psk31,
    Psk63,
}

impl PskMode {
    /// Campioni per simbolo a 12 kHz (31,25 e 62,5 baud).
    pub fn symbol_samples(self) -> usize {
        match self {
            PskMode::Psk31 => 384,
            PskMode::Psk63 => 192,
        }
    }
    pub fn name(self) -> &'static str {
        match self {
            PskMode::Psk31 => "PSK31",
            PskMode::Psk63 => "PSK63",
        }
    }
}

/// I bit da trasmettere: preambolo di inversioni, i caratteri (varicode + `00`) e una breve portante finale.
fn bits_of(text: &[u8]) -> Vec<bool> {
    let mut bits = vec![false; 32];
    for &c in text {
        for ch in varicode::encode(c).chars() {
            bits.push(ch == '1');
        }
        bits.push(false);
        bits.push(false);
    }
    bits.extend(std::iter::repeat_n(true, 8));
    bits
}

/// Audio (a 12 kHz) del testo, con la portante a `freq_hz`; `amp` è l'ampiezza di picco (0..1).
pub fn modulate(text: &[u8], freq_hz: f32, mode: PskMode, amp: f32) -> Vec<f32> {
    let n = mode.symbol_samples();
    let bits = bits_of(text);
    let mut out = Vec::with_capacity(bits.len() * n);
    let step = 2.0 * PI * f64::from(freq_hz) / f64::from(RATE);
    let mut phase = 0.0_f64;
    let mut prev = 1.0_f32;
    for bit in bits {
        let next = if bit { prev } else { -prev };
        for i in 0..n {
            let w = 0.5 * (1.0 + (PI * i as f64 / n as f64).cos()) as f32;
            let env = prev * w + next * (1.0 - w);
            out.push(amp * env * phase.sin() as f32);
            phase += step;
        }
        prev = next;
    }
    out
}

/// Ricevitore: si alimenta con l'audio a 12 kHz e restituisce i caratteri man mano che si decodificano.
pub struct PskDemodulator {
    freq_hz: f32,
    decim: usize,
    box_len: usize,
    ring_i: Vec<f32>,
    ring_q: Vec<f32>,
    pos: usize,
    sum_i: f32,
    sum_q: f32,
    phase: f64,
    count: usize,
    bin: usize,
    mags: [f32; 16],
    seen: usize,
    prev: (f32, f32),
    cur: String,
}

impl PskDemodulator {
    pub fn new(mode: PskMode, freq_hz: f32) -> Self {
        let decim = mode.symbol_samples() / 16;
        let box_len = decim * 2;
        Self {
            freq_hz,
            decim,
            box_len,
            ring_i: vec![0.0; box_len],
            ring_q: vec![0.0; box_len],
            pos: 0,
            sum_i: 0.0,
            sum_q: 0.0,
            phase: 0.0,
            count: 0,
            bin: 0,
            mags: [0.0; 16],
            seen: 0,
            prev: (0.0, 0.0),
            cur: String::new(),
        }
    }

    pub fn set_frequency(&mut self, freq_hz: f32) {
        self.freq_hz = freq_hz;
    }

    pub fn frequency(&self) -> f32 {
        self.freq_hz
    }

    /// Elabora altro audio; restituisce i caratteri nuovi (Latin-1).
    pub fn process(&mut self, samples: &[f32]) -> Vec<u8> {
        let mut out = Vec::new();
        let step = 2.0 * PI * f64::from(self.freq_hz) / f64::from(RATE);
        for &s in samples {
            let i = s * self.phase.cos() as f32;
            let q = -s * self.phase.sin() as f32;
            self.phase += step;
            if self.phase > 2.0 * PI {
                self.phase -= 2.0 * PI;
            }
            self.sum_i += i - self.ring_i[self.pos];
            self.sum_q += q - self.ring_q[self.pos];
            self.ring_i[self.pos] = i;
            self.ring_q[self.pos] = q;
            self.pos = (self.pos + 1) % self.box_len;
            self.count += 1;
            if self.count < self.decim {
                continue;
            }
            self.count = 0;
            let z = (self.sum_i / self.box_len as f32, self.sum_q / self.box_len as f32);
            let mag = (z.0 * z.0 + z.1 * z.1).sqrt();
            self.mags[self.bin] = if self.seen < 32 { mag } else { 0.97 * self.mags[self.bin] + 0.03 * mag };
            let b = self.bin;
            self.bin = (self.bin + 1) % 16;
            self.seen += 1;
            if self.seen < 64 {
                continue;
            }
            let min = (0..16).min_by(|&a, &c| self.mags[a].total_cmp(&self.mags[c])).unwrap_or(0);
            if b == (min + 8) % 16 {
                let dot = z.0 * self.prev.0 + z.1 * self.prev.1;
                self.prev = z;
                if mag > 1e-4 {
                    self.push_bit(dot > 0.0, &mut out);
                }
            }
        }
        out
    }

    fn push_bit(&mut self, one: bool, out: &mut Vec<u8>) {
        if !one && self.cur.is_empty() {
            return;
        }
        self.cur.push(if one { '1' } else { '0' });
        if self.cur.ends_with("00") {
            let code = &self.cur[..self.cur.len() - 2];
            if let Some(c) = varicode::decode(code) {
                out.push(c);
            }
            self.cur.clear();
        } else if self.cur.len() > 12 {
            self.cur.clear();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn andata_e_ritorno(mode: PskMode, tx_hz: f32, rx_hz: f32, text: &str) -> String {
        let audio = modulate(text.as_bytes(), tx_hz, mode, 0.5);
        let mut rx = PskDemodulator::new(mode, rx_hz);
        // un po' di silenzio prima e dopo, come in un ascolto vero
        let mut all = vec![0.0; 4000];
        all.extend(audio);
        all.extend(vec![0.0; 4000]);
        let out = rx.process(&all);
        String::from_utf8_lossy(&out).into_owned()
    }

    #[test]
    fn psk31_andata_e_ritorno() {
        let r = andata_e_ritorno(PskMode::Psk31, 1000.0, 1000.0, "CQ CQ DE IU3ABC K");
        assert!(r.contains("CQ CQ DE IU3ABC K"), "ricevuto: {r:?}");
    }

    #[test]
    fn psk63_andata_e_ritorno() {
        let r = andata_e_ritorno(PskMode::Psk63, 1500.0, 1500.0, "the quick brown fox 73");
        assert!(r.contains("the quick brown fox 73"), "ricevuto: {r:?}");
    }

    #[test]
    fn psk31_regge_qualche_hertz_di_errore() {
        let r = andata_e_ritorno(PskMode::Psk31, 1000.0, 1003.0, "CQ DE IU3ABC");
        assert!(r.contains("CQ DE IU3ABC"), "ricevuto: {r:?}");
    }

    #[test]
    fn psk31_con_rumore() {
        let audio = modulate(b"CQ CQ DE IU3ABC", 1000.0, PskMode::Psk31, 0.5);
        // rumore pseudocasuale deterministico, circa -12 dB rispetto al segnale
        let mut seed = 12345_u32;
        let noisy: Vec<f32> = audio
            .iter()
            .map(|s| {
                seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
                s + (seed as f32 / u32::MAX as f32 - 0.5) * 0.25
            })
            .collect();
        let mut rx = PskDemodulator::new(PskMode::Psk31, 1000.0);
        let mut all = vec![0.0; 4000];
        all.extend(noisy);
        all.extend(vec![0.0; 4000]);
        let r = String::from_utf8_lossy(&rx.process(&all)).into_owned();
        assert!(r.contains("CQ DE IU3ABC"), "ricevuto: {r:?}");
    }
}
