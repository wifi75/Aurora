//! RTTY (Baudot ITA2, 5 bit): FSK a fase continua, di base 45,45 baud e 170 Hz di spostamento.
//!
//! Ogni carattere è un bit di start (space), cinque bit di dati dal meno significativo (mark = 1) e 1,5 bit di stop (mark). I codici
//! 27 (FIGS) e 31 (LTRS) cambiano la tabella. Tabelle riprese da fldigi (`src/rtty/rtty.cxx`, Dave Freese W1HKJ e altri; GPL 3 o
//! successiva), versione americana delle cifre.

use crate::psk::RATE;
use std::f64::consts::PI;

const LETTERS: [u8; 32] = [
    0, b'E', b'\n', b'A', b' ', b'S', b'I', b'U', b'\r', b'D', b'R', b'J', b'N', b'F', b'C', b'K', b'T', b'Z', b'L', b'W', b'H', b'Y', b'P',
    b'Q', b'O', b'B', b'G', b' ', b'M', b'X', b'V', b' ',
];
const FIGURES: [u8; 32] = [
    0, b'3', b'\n', b'-', b' ', 7, b'8', b'7', b'\r', b'$', b'4', b'\'', b',', b'!', b':', b'(', b'5', b'"', b')', b'2', b'#', b'6', b'0',
    b'1', b'9', b'?', b'&', b' ', b'.', b'/', b';', b' ',
];
const FIGS: u8 = 27;
const LTRS: u8 = 31;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RttyParams {
    /// Frequenza del mark (il tono più basso, come nello standard: mark 2125 Hz, space 2295 Hz).
    pub mark_hz: f32,
    pub shift_hz: f32,
    pub baud: f32,
}

impl Default for RttyParams {
    fn default() -> Self {
        Self { mark_hz: 2125.0, shift_hz: 170.0, baud: 45.45 }
    }
}

fn code_of(c: u8) -> Option<(u8, bool)> {
    let up = c.to_ascii_uppercase();
    let valid = |i: &usize| *i != 0 && *i != 27 && *i != 31;
    if let Some(i) = LETTERS.iter().position(|&x| x == up).filter(valid) {
        return Some((i as u8, false));
    }
    FIGURES.iter().position(|&x| x == up).filter(valid).map(|i| (i as u8, true))
}

fn push_frame(seq: &mut Vec<(bool, f64)>, code: u8) {
    seq.push((false, 1.0));
    for b in 0..5 {
        seq.push(((code >> b) & 1 == 1, 1.0));
    }
    seq.push((true, 1.5));
}

/// Audio (a 8 kHz) del testo; `amp` è l'ampiezza di picco (0..1). I caratteri che il Baudot non ha si saltano.
pub fn modulate(text: &[u8], params: RttyParams, amp: f32) -> Vec<f32> {
    // (tono mark?, durata in bit)
    let mut seq: Vec<(bool, f64)> = vec![(true, 8.0)];
    push_frame(&mut seq, LTRS);
    let mut figures = false;
    for &c in text {
        let Some((code, fig)) = code_of(c) else { continue };
        // gli spazi e i ritorni a capo valgono in tutte e due le tabelle: non cambiano il registro
        if code != 4 && code != 8 && code != 2 && fig != figures {
            push_frame(&mut seq, if fig { FIGS } else { LTRS });
            figures = fig;
        }
        push_frame(&mut seq, code);
    }
    seq.push((true, 4.0));

    let spb = f64::from(RATE) / f64::from(params.baud);
    let mut out = Vec::new();
    let mut phase = 0.0_f64;
    let mut t_end = 0.0_f64;
    for (mark, bits) in seq {
        t_end += bits * spb;
        let f = f64::from(params.mark_hz) + if mark { 0.0 } else { f64::from(params.shift_hz) };
        let step = 2.0 * PI * f / f64::from(RATE);
        while (out.len() as f64) < t_end {
            out.push(amp * phase.sin() as f32);
            phase += step;
        }
        phase %= 2.0 * PI;
    }
    out
}

/// Una finestra mobile di somme (I, Q) alla frequenza data: la sua ampiezza dice quanto c'è di quel tono nell'ultimo bit.
struct Tone {
    step: f64,
    phase: f64,
    ring_i: Vec<f32>,
    ring_q: Vec<f32>,
    sum_i: f32,
    sum_q: f32,
}

impl Tone {
    fn new(freq_hz: f32, len: usize) -> Self {
        Self { step: 2.0 * PI * f64::from(freq_hz) / f64::from(RATE), phase: 0.0, ring_i: vec![0.0; len], ring_q: vec![0.0; len], sum_i: 0.0, sum_q: 0.0 }
    }
    fn push(&mut self, s: f32, pos: usize) -> f32 {
        let i = s * self.phase.cos() as f32;
        let q = -s * self.phase.sin() as f32;
        self.phase += self.step;
        if self.phase > 2.0 * PI {
            self.phase -= 2.0 * PI;
        }
        self.sum_i += i - self.ring_i[pos];
        self.sum_q += q - self.ring_q[pos];
        self.ring_i[pos] = i;
        self.ring_q[pos] = q;
        (self.sum_i * self.sum_i + self.sum_q * self.sum_q).sqrt() / self.ring_i.len() as f32
    }
}

#[derive(Clone, Copy, PartialEq)]
enum State {
    /// Si aspetta il passaggio da mark a space (l'inizio di un carattere).
    Idle,
    /// Dentro un carattere: prossimo campionamento fra `wait` campioni, bit numero `bit` (0 = start, 1..=5 dati, 6 = stop).
    Frame { wait: usize, bit: u8 },
}

pub struct RttyDemodulator {
    len: usize,
    pos: usize,
    mark: Tone,
    space: Tone,
    prev_d: f32,
    state: State,
    data: u8,
    figures: bool,
}

impl RttyDemodulator {
    pub fn new(params: RttyParams) -> Self {
        let len = (f64::from(RATE) / f64::from(params.baud)).round() as usize;
        Self {
            len,
            pos: 0,
            mark: Tone::new(params.mark_hz, len),
            space: Tone::new(params.mark_hz + params.shift_hz, len),
            prev_d: 0.0,
            state: State::Idle,
            data: 0,
            figures: false,
        }
    }

    /// Elabora altro audio; restituisce i caratteri nuovi.
    pub fn process(&mut self, samples: &[f32]) -> Vec<u8> {
        let mut out = Vec::new();
        for &s in samples {
            let m = self.mark.push(s, self.pos);
            let sp = self.space.push(s, self.pos);
            self.pos = (self.pos + 1) % self.len;
            let d = m - sp;
            let level = m + sp;
            match self.state {
                State::Idle => {
                    // il filtro vede il bit a metà finestra: l'inizio del carattere è mezzo bit prima, il primo campionamento un bit dopo
                    if self.prev_d > 0.0 && d <= 0.0 && level > 1e-3 {
                        self.state = State::Frame { wait: self.len / 2, bit: 0 };
                        self.data = 0;
                    }
                }
                State::Frame { wait, bit } => {
                    if wait > 1 {
                        self.state = State::Frame { wait: wait - 1, bit };
                    } else {
                        let mark = d > 0.0;
                        match bit {
                            0 => {
                                self.state = if mark { State::Idle } else { State::Frame { wait: self.len, bit: 1 } };
                            }
                            1..=5 => {
                                if mark {
                                    self.data |= 1 << (bit - 1);
                                }
                                self.state = State::Frame { wait: self.len, bit: bit + 1 };
                            }
                            _ => {
                                if mark {
                                    self.emit(&mut out);
                                }
                                self.state = State::Idle;
                            }
                        }
                    }
                }
            }
            self.prev_d = d;
        }
        out
    }

    fn emit(&mut self, out: &mut Vec<u8>) {
        match self.data {
            FIGS => self.figures = true,
            LTRS => self.figures = false,
            0 => {}
            c => out.push(if self.figures { FIGURES[c as usize] } else { LETTERS[c as usize] }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn andata_e_ritorno(params: RttyParams, text: &str) -> String {
        let audio = modulate(text.as_bytes(), params, 0.5);
        let mut rx = RttyDemodulator::new(params);
        let mut all = vec![0.0; 4000];
        all.extend(audio);
        all.extend(vec![0.0; 4000]);
        String::from_utf8_lossy(&rx.process(&all)).into_owned()
    }

    #[test]
    fn lettere() {
        let r = andata_e_ritorno(RttyParams::default(), "RYRYRY CQ DE IU3ABC K");
        assert!(r.contains("RYRYRY CQ DE IU"), "ricevuto: {r:?}");
    }

    #[test]
    fn cifre_e_ritorni_a_capo() {
        let r = andata_e_ritorno(RttyParams::default(), "CQ TEST 599 73\r\nDE IU3ABC");
        assert!(r.contains("CQ TEST 599 73\r\nDE IU3ABC"), "ricevuto: {r:?}");
    }

    #[test]
    fn altri_toni() {
        let p = RttyParams { mark_hz: 1275.0, shift_hz: 170.0, baud: 45.45 };
        let r = andata_e_ritorno(p, "THE QUICK BROWN FOX 1234567890");
        assert!(r.contains("THE QUICK BROWN FOX 1234567890"), "ricevuto: {r:?}");
    }

    #[test]
    fn baud_50() {
        let p = RttyParams { mark_hz: 2125.0, shift_hz: 450.0, baud: 50.0 };
        let r = andata_e_ritorno(p, "CQ DE IU3ABC");
        assert!(r.contains("CQ DE IU3ABC"), "ricevuto: {r:?}");
    }
}
