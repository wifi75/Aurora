//! I modulatori a flusso dei modi testuali: l'audio di una trasmissione si fa un pezzo alla volta (preambolo, un carattere, un tratto di
//! attesa, la coda), con la fase e i tempi che continuano da un pezzo all'altro. Servono alla trasmissione scritta dal vivo (il testo arriva
//! mentre si trasmette) e a sapere dove finisce l'audio di ogni carattere, per mostrare a che punto e' la trasmissione.
//!
//! Messi in fila, preambolo + caratteri + coda danno esattamente l'onda di `rtty::modulate`, `psk::modulate` e `cw::modulate` sullo stesso
//! testo (prove in fondo). In attesa (nessun carattere da mandare) l'RTTY manda LTRS come fldigi («diddle»), il PSK le inversioni di fase
//! dello zero, il CW il silenzio.

use std::f64::consts::PI;

use crate::cw::{PROSIGNS, pattern_of, unit_seconds};
use crate::psk::{PskMode, RATE};
use crate::rtty::{CR, FIGS, LF, LTRS, RttyParams, SPACE, code_of, push_frame};
use crate::varicode;

/// Ampiezza di picco delle onde trasmesse (la stessa di `DigiService::wave_of`).
const AMP: f32 = 0.9;

/// Il modulatore a flusso di un modo.
pub enum TxModulator {
    Rtty(RttyTx),
    Psk(PskTx),
    Cw(CwTx),
}

impl TxModulator {
    pub fn rtty(params: RttyParams) -> Self {
        TxModulator::Rtty(RttyTx::new(params))
    }

    pub fn psk(mode: PskMode, freq_hz: f32) -> Self {
        TxModulator::Psk(PskTx::new(mode, freq_hz))
    }

    pub fn cw(tone_hz: f32, wpm: f32) -> Self {
        TxModulator::Cw(CwTx::new(tone_hz, wpm))
    }

    /// L'inizio della trasmissione.
    pub fn preamble(&mut self) -> Vec<f32> {
        match self {
            TxModulator::Rtty(m) => m.preamble(),
            TxModulator::Psk(m) => m.preamble(),
            TxModulator::Cw(m) => m.preamble(),
        }
    }

    /// Un pezzo di testo (un carattere; in CW anche un segno di procedura come `<SK>`). Un carattere che il modo non ha non da' audio.
    pub fn token(&mut self, token: &str) -> Vec<f32> {
        match self {
            TxModulator::Rtty(m) => token.chars().flat_map(|c| m.char(byte_of(c))).collect(),
            TxModulator::Psk(m) => token.chars().flat_map(|c| m.char(byte_of(c))).collect(),
            TxModulator::Cw(m) => m.token(token),
        }
    }

    /// Un tratto di attesa (niente da mandare): RTTY un LTRS, PSK quattro zeri, CW un decimo di secondo di silenzio.
    pub fn idle(&mut self) -> Vec<f32> {
        match self {
            TxModulator::Rtty(m) => m.idle(),
            TxModulator::Psk(m) => m.idle(),
            TxModulator::Cw(m) => m.idle(),
        }
    }

    /// La fine della trasmissione (torna al silenzio senza «click»).
    pub fn postamble(&mut self) -> Vec<f32> {
        match self {
            TxModulator::Rtty(m) => m.postamble(),
            TxModulator::Psk(m) => m.postamble(),
            TxModulator::Cw(m) => m.postamble(),
        }
    }

    /// Quanti caratteri del testo `text` (che puo' ancora crescere se `more` e' vero) formano il prossimo pezzo; `None` se bisogna
    /// aspettare il resto (CW: `<S` potrebbe diventare `<SK>`).
    pub fn next_token(&self, text: &[char], more: bool) -> Option<usize> {
        let first = *text.first()?;
        if !matches!(self, TxModulator::Cw(_)) || first != '<' {
            return Some(1);
        }
        let upper: String = text.iter().take(4).collect::<String>().to_ascii_uppercase();
        if PROSIGNS.iter().any(|(name, _)| upper.starts_with(name)) {
            return Some(4);
        }
        if more && text.len() < 4 && PROSIGNS.iter().any(|(name, _)| name.starts_with(upper.as_str())) {
            return None;
        }
        Some(1)
    }
}

/// Il byte trasmesso per un carattere (come `DigiService::wave_of`: oltre il Latin-1 un punto interrogativo).
fn byte_of(c: char) -> u8 {
    if (c as u32) < 256 { c as u32 as u8 } else { b'?' }
}

/// RTTY: FSK a fase continua, gli stessi conti di `rtty::modulate` (anche i FIGS e LTRS rimandati dopo uno spazio fra le cifre).
pub struct RttyTx {
    params: RttyParams,
    spb: f64,
    phase: f64,
    t_end: f64,
    emitted: usize,
    /// Il registro di chi riceve: `None` = non noto (dopo uno spazio fra le cifre).
    shift: Option<bool>,
}

impl RttyTx {
    fn new(params: RttyParams) -> Self {
        Self { params, spb: f64::from(RATE) / f64::from(params.baud), phase: 0.0, t_end: 0.0, emitted: 0, shift: Some(false) }
    }

    fn synth(&mut self, seq: &[(bool, f64)]) -> Vec<f32> {
        let mut out = Vec::new();
        for &(mark, bits) in seq {
            self.t_end += bits * self.spb;
            let f = f64::from(if mark { self.params.mark_hz() } else { self.params.space_hz() });
            let step = 2.0 * PI * f / f64::from(RATE);
            while (self.emitted as f64) < self.t_end {
                out.push(AMP * self.phase.sin() as f32);
                self.phase += step;
                self.emitted += 1;
            }
            self.phase %= 2.0 * PI;
        }
        out
    }

    fn preamble(&mut self) -> Vec<f32> {
        let mut seq = vec![(true, 8.0)];
        push_frame(&mut seq, LTRS);
        self.shift = Some(false);
        self.synth(&seq)
    }

    fn char(&mut self, c: u8) -> Vec<f32> {
        let Some((code, fig)) = code_of(c) else { return Vec::new() };
        let mut seq = Vec::new();
        match code {
            SPACE => {
                push_frame(&mut seq, code);
                if self.shift == Some(true) {
                    self.shift = None;
                }
            }
            CR | LF => push_frame(&mut seq, code),
            _ => {
                if self.shift != Some(fig) {
                    push_frame(&mut seq, if fig { FIGS } else { LTRS });
                    self.shift = Some(fig);
                }
                push_frame(&mut seq, code);
            }
        }
        self.synth(&seq)
    }

    fn idle(&mut self) -> Vec<f32> {
        let mut seq = Vec::new();
        push_frame(&mut seq, LTRS);
        self.shift = Some(false);
        self.synth(&seq)
    }

    fn postamble(&mut self) -> Vec<f32> {
        self.synth(&[(true, 4.0)])
    }
}

/// PSK31 e PSK63: gli stessi conti di `psk::modulate` (inversione di fase per lo zero, transizioni a coseno rialzato).
pub struct PskTx {
    mode: PskMode,
    step: f64,
    phase: f64,
    prev: f32,
    sign: f32,
}

impl PskTx {
    fn new(mode: PskMode, freq_hz: f32) -> Self {
        Self { mode, step: 2.0 * PI * f64::from(freq_hz) / f64::from(RATE), phase: 0.0, prev: 0.0, sign: 1.0 }
    }

    /// Un simbolo: `Some(bit)`, oppure `None` = silenzio (la discesa finale).
    fn symbol(&mut self, sym: Option<bool>, out: &mut Vec<f32>) {
        let n = self.mode.symbol_samples();
        let next = match sym {
            Some(bit) => {
                if !bit && self.prev != 0.0 {
                    self.sign = -self.sign;
                }
                self.sign
            }
            None => 0.0,
        };
        for i in 0..n {
            let w = 0.5 * (1.0 + (PI * i as f64 / n as f64).cos()) as f32;
            let env = self.prev * w + next * (1.0 - w);
            out.push(AMP * env * self.phase.sin() as f32);
            self.phase += self.step;
        }
        self.prev = next;
    }

    fn preamble(&mut self) -> Vec<f32> {
        let mut out = Vec::new();
        for _ in 0..self.mode.dcd_bits() {
            self.symbol(Some(false), &mut out);
        }
        out
    }

    fn char(&mut self, c: u8) -> Vec<f32> {
        let mut out = Vec::new();
        for ch in varicode::encode(c).chars() {
            self.symbol(Some(ch == '1'), &mut out);
        }
        self.symbol(Some(false), &mut out);
        self.symbol(Some(false), &mut out);
        out
    }

    fn idle(&mut self) -> Vec<f32> {
        let mut out = Vec::new();
        for _ in 0..4 {
            self.symbol(Some(false), &mut out);
        }
        out
    }

    fn postamble(&mut self) -> Vec<f32> {
        let mut out = Vec::new();
        for _ in 0..self.mode.dcd_bits() {
            self.symbol(Some(true), &mut out);
        }
        self.symbol(None, &mut out);
        out
    }
}

/// CW: gli stessi conti di `cw::modulate` (fronti di 5 ms, pause di 1, 3 e 7 unita').
pub struct CwTx {
    unit: f64,
    step: f64,
    ramp: usize,
    phase: f64,
    t_end: f64,
    emitted: usize,
}

impl CwTx {
    fn new(tone_hz: f32, wpm: f32) -> Self {
        Self {
            unit: unit_seconds(wpm) * f64::from(RATE),
            step: 2.0 * PI * f64::from(tone_hz) / f64::from(RATE),
            ramp: (0.005 * f64::from(RATE)) as usize,
            phase: 0.0,
            t_end: 0.0,
            emitted: 0,
        }
    }

    fn elements(&mut self, seq: &[(bool, f64)]) -> Vec<f32> {
        let mut out = Vec::new();
        for &(on, units) in seq {
            self.t_end += units * self.unit;
            let stop = self.t_end.round() as usize;
            let len = stop.saturating_sub(self.emitted);
            for i in 0..len {
                if !on {
                    out.push(0.0);
                    continue;
                }
                let edge = i.min(len - 1 - i);
                let w = if edge < self.ramp { 0.5 * (1.0 - (PI * edge as f64 / self.ramp as f64).cos()) } else { 1.0 };
                out.push((f64::from(AMP) * w * self.phase.sin()) as f32);
                self.phase += self.step;
            }
            self.emitted += len;
            self.phase %= 2.0 * PI;
        }
        out
    }

    fn preamble(&mut self) -> Vec<f32> {
        let lead = (0.05 * f64::from(RATE)) as usize;
        self.emitted = lead;
        self.t_end = lead as f64;
        vec![0.0; lead]
    }

    fn token(&mut self, token: &str) -> Vec<f32> {
        let bytes = token.as_bytes();
        let pattern = PROSIGNS.iter().find(|(name, _)| bytes.eq_ignore_ascii_case(name.as_bytes())).map(|(_, p)| *p);
        let c = token.chars().next().map_or(b' ', byte_of);
        let mut seq = Vec::new();
        match pattern.or_else(|| pattern_of(c)) {
            Some(p) => {
                for e in p.bytes() {
                    seq.push((true, if e == b'-' { 3.0 } else { 1.0 }));
                    seq.push((false, 1.0));
                }
                seq.push((false, 2.0));
            }
            None if matches!(c, b' ' | b'\n' | b'\r' | b'\t') => seq.push((false, 4.0)),
            None => {}
        }
        self.elements(&seq)
    }

    fn idle(&mut self) -> Vec<f32> {
        let units = (0.1 * f64::from(RATE) / self.unit).ceil().max(1.0);
        self.elements(&[(false, units)])
    }

    fn postamble(&mut self) -> Vec<f32> {
        let tail = (0.25 * f64::from(RATE)) as usize;
        self.emitted += tail;
        self.t_end += tail as f64;
        vec![0.0; tail]
    }
}

/// L'onda intera di un testo con i modulatori a flusso (per le prove e per chi non trasmette a flusso).
pub fn whole(m: &mut TxModulator, text: &str) -> Vec<f32> {
    let chars: Vec<char> = text.chars().collect();
    let mut out = m.preamble();
    let mut i = 0;
    while i < chars.len() {
        let n = m.next_token(&chars[i..], false).unwrap_or(1);
        let tok: String = chars[i..i + n].iter().collect();
        out.extend(m.token(&tok));
        i += n;
    }
    out.extend(m.postamble());
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{cw, psk, rtty};

    const TEXT: &str = "CQ TEST 599 73 DE IU3ABC <SK> 14080\r\nK";

    #[test]
    fn rtty_a_pezzi_e_come_l_onda_intera() {
        let p = RttyParams::default();
        assert_eq!(whole(&mut TxModulator::rtty(p), TEXT), rtty::modulate(TEXT.as_bytes(), p, AMP));
    }

    #[test]
    fn psk_a_pezzi_e_come_l_onda_intera() {
        for mode in [PskMode::Psk31, PskMode::Psk63] {
            assert_eq!(whole(&mut TxModulator::psk(mode, 1500.0), TEXT), psk::modulate(TEXT.as_bytes(), 1500.0, mode, AMP));
        }
    }

    #[test]
    fn cw_a_pezzi_e_come_l_onda_intera() {
        assert_eq!(whole(&mut TxModulator::cw(700.0, 25.0), TEXT), cw::modulate(TEXT.as_bytes(), 700.0, 25.0, AMP));
    }

    #[test]
    fn in_cw_un_segno_di_procedura_a_meta_si_aspetta() {
        let m = TxModulator::cw(700.0, 25.0);
        let t: Vec<char> = "<S".chars().collect();
        assert_eq!(m.next_token(&t, true), None);
        assert_eq!(m.next_token(&t, false), Some(1));
        let t: Vec<char> = "<sk> ".chars().collect();
        assert_eq!(m.next_token(&t, true), Some(4));
        assert_eq!(TxModulator::rtty(RttyParams::default()).next_token(&t, true), Some(1));
    }

    #[test]
    fn l_attesa_rtty_si_decodifica_come_niente() {
        // preambolo, «CQ», due tratti di attesa, «DE»: chi riceve legge «CQDE» (gli LTRS non stampano)
        let p = RttyParams::default();
        let mut m = TxModulator::rtty(p);
        let mut audio = m.preamble();
        audio.extend(m.token("C"));
        audio.extend(m.token("Q"));
        audio.extend(m.idle());
        audio.extend(m.idle());
        audio.extend(m.token("D"));
        audio.extend(m.token("E"));
        audio.extend(m.postamble());
        let mut rx = rtty::RttyDemodulator::new(p);
        assert_eq!(String::from_utf8_lossy(&rx.process(&audio)), "CQDE");
    }
}
