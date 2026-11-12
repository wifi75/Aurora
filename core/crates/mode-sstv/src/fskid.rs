//! FSK ID: il nominativo di chi trasmette, in FSK subito dopo l'immagine, nello stesso formato di MMSSTV (Makoto Mori JE3HHT, LGPL,
//! copia in `terze-parti/mmsstv`: `TMmsstv::OutputFSKID` in `Main.cpp`, `CSSTVMOD::WriteFSK` e `CSSTVDEM::DecodeFSK` in `sstv.cpp`),
//! cosi' lo leggono MMSSTV e i programmi che lo seguono.
//!
//! - 100 ms a 2100 Hz, 22 ms a 1900 Hz;
//! - caratteri da 6 bit, dal bit meno significativo, 22 ms per bit: 1900 Hz = 1, 2100 Hz = 0;
//! - `0x2A` d'inizio, ogni carattere del nominativo meno `0x20`, `0x01` di fine, lo XOR dei caratteri;
//! - di nuovo 100 ms a 2100 Hz.
//!
//! In ricezione si lavora sulle frequenze gia' demodulate (medie di 1 ms) invece che sui due filtri di MMSSTV: ogni bit e' la mediana
//! della sua parte centrale, cosi' i colpi del rumore non contano.

use std::collections::VecDeque;

/// Frequenza del bit 1 (e del bit di partenza).
pub const MARK_HZ: f32 = 1900.0;
/// Frequenza del bit 0 e della guardia prima e dopo.
pub const SPACE_HZ: f32 = 2100.0;
/// Durata di un bit (FSKINTVAL di MMSSTV).
pub const BIT_S: f64 = 0.022;
/// Durata della guardia (FSKGARD di MMSSTV).
pub const GUARD_S: f64 = 0.100;
/// Caratteri al massimo (MMSSTV ne accetta 16).
pub const MAX_CHARS: usize = 16;

const START: u8 = 0x2a;
const END: u8 = 0x01;
/// Campioni (a 12 kHz) in una media da 1 ms.
const BIN: usize = 12;
/// Millisecondi per bit.
const BIT_MS: usize = 22;
/// Parte del bit usata per decidere: si lasciano 4 ms per lato (fronti e incertezza sul fronte di partenza).
const EDGE_MS: usize = 4;
/// Media di 1 ms vicina a un tono: entro questa distanza.
const NEAR_HZ: f32 = 110.0;

/// Il nominativo pronto per l'FSK ID: maiuscolo, senza spazi ai lati, solo caratteri da 0x21 a 0x5F, da 1 a 16. `None` se non si puo'.
pub fn clean_call(call: &str) -> Option<String> {
    let c = call.trim().to_ascii_uppercase();
    let ok = !c.is_empty() && c.len() <= MAX_CHARS && c.bytes().all(|b| (0x21..=0x5f).contains(&b));
    ok.then_some(c)
}

/// I toni dell'FSK ID (frequenza, secondi), o `None` se il nominativo non si puo' mandare.
pub fn tones(call: &str) -> Option<Vec<(f32, f64)>> {
    let call = clean_call(call)?;
    let mut out = vec![(SPACE_HZ, GUARD_S), (MARK_HZ, BIT_S)];
    let put = |c: u8, out: &mut Vec<(f32, f64)>| {
        for bit in 0..6 {
            out.push((if (c >> bit) & 1 == 1 { MARK_HZ } else { SPACE_HZ }, BIT_S));
        }
    };
    put(START, &mut out);
    let mut sum = 0u8;
    for b in call.bytes() {
        let c = b - 0x20;
        sum ^= c;
        put(c, &mut out);
    }
    put(END, &mut out);
    put(sum, &mut out);
    out.push((SPACE_HZ, GUARD_S));
    Some(out)
}

/// Durata dell'FSK ID di un nominativo (secondi).
pub fn seconds(call: &str) -> f64 {
    tones(call).map_or(0.0, |t| t.iter().map(|x| x.1).sum())
}

/// Un nominativo credibile: lettere, cifre e «/», almeno una lettera e una cifra, da 3 caratteri.
fn plausible(call: &str) -> bool {
    call.len() >= 3 && call.chars().all(|c| c.is_ascii_alphanumeric() || c == '/') && call.chars().any(|c| c.is_ascii_digit()) && call.chars().any(|c| c.is_ascii_alphabetic())
}

fn median(values: &mut [f32]) -> f32 {
    values.sort_by(|a, b| a.total_cmp(b));
    values.get(values.len() / 2).copied().unwrap_or(0.0)
}

enum State {
    /// Si cerca la guardia a 2100 Hz: le ultime medie, vere se vicine a 2100 Hz.
    Idle,
    /// Guardia trovata: si aspetta il passaggio a 1900 Hz (medie dall'aggancio).
    Armed { bins: Vec<f32> },
    /// Dal fronte del bit di partenza: le medie arrivate, i bit gia' decisi.
    Bits { bins: Vec<f32>, bits: u32, chars: Vec<u8>, cur: u8, nbits: u8 },
}

/// Il lettore dell'FSK ID, a flusso: riceve le frequenze demodulate (12 kHz) e ritorna il nominativo quando lo trova intero e con la
/// somma di controllo giusta.
pub struct FskIdDetector {
    acc: f32,
    acc_n: usize,
    /// Scarto di sintonia (Hz) da togliere: quello stimato sull'immagine.
    offset: f32,
    guard: VecDeque<bool>,
    state: State,
}

impl Default for FskIdDetector {
    fn default() -> Self {
        Self::new()
    }
}

impl FskIdDetector {
    pub fn new() -> FskIdDetector {
        FskIdDetector { acc: 0.0, acc_n: 0, offset: 0.0, guard: VecDeque::with_capacity(50), state: State::Idle }
    }

    pub fn set_offset(&mut self, hz: f32) {
        if hz.is_finite() {
            self.offset = hz;
        }
    }

    pub fn reset(&mut self) {
        self.guard.clear();
        self.state = State::Idle;
    }

    /// Frequenze demodulate in ingresso; ritorna un nominativo letto.
    pub fn push(&mut self, freqs: &[f32]) -> Option<String> {
        let mut found = None;
        for &f in freqs {
            self.acc += f;
            self.acc_n += 1;
            if self.acc_n < BIN {
                continue;
            }
            let mean = self.acc / BIN as f32 - self.offset;
            self.acc = 0.0;
            self.acc_n = 0;
            if let Some(call) = self.step(mean) {
                found = Some(call);
            }
        }
        found
    }

    fn step(&mut self, f: f32) -> Option<String> {
        match &mut self.state {
            State::Idle => {
                if self.guard.len() == 50 {
                    self.guard.pop_front();
                }
                self.guard.push_back((f - SPACE_HZ).abs() < NEAR_HZ);
                // almeno 40 ms dei 100 della guardia (il rumore puo' sporcarne qualcuno)
                if self.guard.len() == 50 && self.guard.iter().filter(|&&g| g).count() >= 40 {
                    self.guard.clear();
                    self.state = State::Armed { bins: Vec::with_capacity(160) };
                }
                None
            }
            State::Armed { bins } => {
                bins.push(f);
                let n = bins.len();
                // il fronte: una media sotto i 2000 Hz seguita da altre quattro con la mediana sotto i 2000 Hz
                if n >= 5 {
                    let e = n - 5;
                    let mut look = bins[e..].to_vec();
                    if bins[e] < 2000.0 && median(&mut look) < 2000.0 {
                        let rest = bins[e..].to_vec();
                        self.state = State::Bits { bins: rest, bits: 0, chars: Vec::new(), cur: 0, nbits: 0 };
                        return self.decide();
                    }
                }
                // la guardia dura 100 ms: senza fronte entro 200 ms si ricomincia
                if n > 200 {
                    self.state = State::Idle;
                }
                None
            }
            State::Bits { bins, .. } => {
                bins.push(f);
                self.decide()
            }
        }
    }

    /// Decide i bit completi; alla fine del nominativo lo ritorna.
    fn decide(&mut self) -> Option<String> {
        loop {
            let State::Bits { bins, bits, chars, cur, nbits } = &mut self.state else { return None };
            let k = *bits as usize;
            let (from, to) = (k * BIT_MS + EDGE_MS, (k + 1) * BIT_MS - EDGE_MS);
            if bins.len() < to {
                return None;
            }
            let mut w = bins[from..to].to_vec();
            let m = median(&mut w);
            let one = (m - MARK_HZ).abs() < NEAR_HZ;
            let zero = (m - SPACE_HZ).abs() < NEAR_HZ;
            if !(one || zero) || (k == 0 && !one) {
                self.state = State::Idle;
                return None;
            }
            *bits += 1;
            if k == 0 {
                continue;
            }
            *cur = (*cur >> 1) | if one { 0x20 } else { 0 };
            *nbits += 1;
            if *nbits < 6 {
                continue;
            }
            let c = *cur;
            *cur = 0;
            *nbits = 0;
            let first = chars.is_empty();
            let closed = chars.last() == Some(&END);
            if first && c != START {
                self.state = State::Idle;
                return None;
            }
            if closed {
                // somma di controllo: lo XOR dei caratteri fra l'inizio e la fine
                let data = &chars[1..chars.len() - 1];
                let sum = data.iter().fold(0u8, |s, &x| s ^ x) & 0x3f;
                let call: String = data.iter().map(|&x| char::from(x + 0x20)).collect::<String>().trim().to_string();
                self.state = State::Idle;
                return (c == sum && plausible(&call)).then_some(call);
            }
            if c == END && chars.len() < 2 {
                self.state = State::Idle;
                return None;
            }
            chars.push(c);
            if chars.len() > MAX_CHARS + 2 {
                self.state = State::Idle;
                return None;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::demod::{FmDemod, RATE};

    fn synth(tones: &[(f32, f64)]) -> Vec<f32> {
        let mut out = Vec::new();
        let mut phase = 0.0f64;
        let mut clock = 0.0;
        for &(hz, dur) in tones {
            clock += dur;
            let end = (clock * f64::from(RATE)).round() as usize;
            while out.len() < end {
                out.push(phase.sin() as f32);
                phase = (phase + std::f64::consts::TAU * f64::from(hz) / f64::from(RATE)) % std::f64::consts::TAU;
            }
        }
        out
    }

    #[test]
    fn the_layout_follows_mmsstv() {
        let t = tones("iu3abc").expect("toni");
        // guardia, bit di partenza, 0x2A, 6 caratteri, fine, somma, guardia
        assert_eq!(t.len(), 2 + 6 * (1 + 6 + 1 + 1) + 1);
        assert_eq!(t[0], (SPACE_HZ, GUARD_S));
        assert_eq!(t[1], (MARK_HZ, BIT_S));
        // 0x2A = 101010: dal meno significativo 0,1,0,1,0,1
        let first: Vec<f32> = t[2..8].iter().map(|x| x.0).collect();
        assert_eq!(first, [SPACE_HZ, MARK_HZ, SPACE_HZ, MARK_HZ, SPACE_HZ, MARK_HZ]);
        assert!(tones("").is_none());
        assert!(tones("abcdefghijklmnopq").is_none());
        assert!((seconds("IU3ABC") - (0.2 + 0.022 * 55.0)).abs() < 1e-9);
    }

    #[test]
    fn reads_back_a_clean_id() {
        let mut audio = vec![0.0f32; 3000];
        audio.extend(synth(&tones("IU3ABC").expect("toni")));
        audio.extend(vec![0.0f32; 3000]);
        let mut freqs = Vec::new();
        FmDemod::new().process(&audio, &mut freqs);
        let mut det = FskIdDetector::new();
        let got: Vec<String> = freqs.chunks(500).filter_map(|c| det.push(c)).collect();
        assert_eq!(got, ["IU3ABC"]);
    }
}
