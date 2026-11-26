//! I modulatori a flusso dei modi testuali: l'audio di una trasmissione si fa un pezzo alla volta (preambolo, un carattere, un tratto di
//! attesa, la coda), con la fase e i tempi che continuano da un pezzo all'altro. Servono alla trasmissione scritta dal vivo (il testo arriva
//! mentre si trasmette) e a sapere dove finisce l'audio di ogni carattere, per mostrare a che punto e' la trasmissione.
//!
//! Messi in fila, preambolo + caratteri + coda danno esattamente l'onda di `rtty::modulate`, `psk::modulate` e `cw::modulate` sullo stesso
//! testo (prove in fondo). In attesa (nessun carattere da mandare) l'RTTY manda LTRS come fldigi («diddle»), il PSK le inversioni di fase
//! dello zero, il CW il silenzio.

use std::f64::consts::PI;

use crate::cw::{PROSIGNS, pattern_of, unit_seconds};
use crate::olivia;
use crate::resample_8k::Interpolator;
use crate::psk::{PskMode, RATE};
use crate::qpsk::QpskTx;
use crate::rtty::{LTRS, Reg, RttyCharset, RttyParams, push_char, push_frame};
use crate::varicode;

/// Ampiezza di picco delle onde trasmesse (la stessa di `DigiService::wave_of`).
const AMP: f32 = 0.9;

/// Il modulatore a flusso di un modo.
pub enum TxModulator {
    Rtty(RttyTx),
    Psk(PskTx),
    Cw(CwTx),
    Olivia(Box<OliviaTx>),
    Throb(Box<ThrobStream>),
    Amtor(Box<crate::amtor::AmtorTx>),
}

impl TxModulator {
    pub fn rtty(params: RttyParams, charset: RttyCharset) -> Self {
        TxModulator::Rtty(RttyTx::new(params, charset))
    }

    pub fn psk(mode: PskMode, freq_hz: f32) -> Self {
        TxModulator::Psk(PskTx::new(mode, freq_hz))
    }

    /// PSK con l'inversione di banda laterale (`reverse`: la radio in LSB), che conta solo in QPSK31.
    pub fn psk_reverse(mode: PskMode, freq_hz: f32, reverse: bool) -> Self {
        let mut m = PskTx::new(mode, freq_hz);
        if mode.is_qpsk() {
            m.qpsk = Some(Box::new(QpskTx::new(freq_hz, mode.symbol_samples(), RATE, AMP, reverse)));
        }
        TxModulator::Psk(m)
    }

    pub fn cw(tone_hz: f32, wpm: f32) -> Self {
        TxModulator::Cw(CwTx::new(tone_hz, wpm))
    }

    /// Olivia o Contestia; `None` se toni, banda e centro non stanno nella banda audio.
    pub fn olivia(params: olivia::Params) -> Option<Self> {
        OliviaTx::new(params).map(|m| TxModulator::Olivia(Box::new(m)))
    }

    /// Throb o ThrobX centrato a `freq_hz`; `reverse`: la radio in LSB (toni rovesciati).
    pub fn throb(v: crate::throb::ThrobVariant, freq_hz: f32, reverse: bool) -> Self {
        TxModulator::Throb(Box::new(ThrobStream::new(v, freq_hz, reverse)))
    }

    /// Amtor FEC centrato a `freq_hz`; `reverse`: la radio in LSB (mark e space scambiati).
    pub fn amtor(freq_hz: f32, reverse: bool) -> Self {
        TxModulator::Amtor(Box::new(crate::amtor::tx_12k(freq_hz, AMP, reverse)))
    }

    /// L'inizio della trasmissione.
    pub fn preamble(&mut self) -> Vec<f32> {
        match self {
            TxModulator::Amtor(m) => m.preamble(),
            TxModulator::Throb(m) => m.preamble(),
            TxModulator::Rtty(m) => m.preamble(),
            TxModulator::Psk(m) => m.preamble(),
            TxModulator::Cw(m) => m.preamble(),
            TxModulator::Olivia(m) => m.preamble(),
        }
    }

    /// Un pezzo di testo (un carattere; in CW anche un segno di procedura come `<SK>`). Un carattere che il modo non ha non da' audio.
    pub fn token(&mut self, token: &str) -> Vec<f32> {
        match self {
            TxModulator::Rtty(m) => token.chars().flat_map(|c| m.char(c)).collect(),
            TxModulator::Psk(m) => token.chars().flat_map(|c| m.char(byte_of(c))).collect(),
            TxModulator::Cw(m) => m.token(token),
            TxModulator::Olivia(m) => token.chars().flat_map(|c| m.char(byte_of(c))).collect(),
            TxModulator::Throb(m) => token.chars().flat_map(|c| m.char(byte_of(c))).collect(),
            TxModulator::Amtor(m) => token.chars().flat_map(|c| m.char(byte_of(c))).collect(),
        }
    }

    /// RTTY: in attesa si manda il «diddle» (LTRS, come MMTTY e fldigi) o, spento, il solo tono di riposo (mark); negli altri modi non fa nulla.
    #[must_use]
    pub fn with_diddle(mut self, on: bool) -> Self {
        if let TxModulator::Rtty(m) = &mut self {
            m.diddle = on;
        }
        self
    }

    /// Un tratto di attesa (niente da mandare): RTTY un LTRS (o il tono di riposo senza diddle), PSK quattro zeri, CW un decimo di secondo di silenzio.
    pub fn idle(&mut self) -> Vec<f32> {
        match self {
            TxModulator::Rtty(m) => m.idle(),
            TxModulator::Psk(m) => m.idle(),
            TxModulator::Cw(m) => m.idle(),
            TxModulator::Olivia(m) => m.idle(),
            TxModulator::Throb(m) => m.idle(),
            TxModulator::Amtor(m) => m.idle(),
        }
    }

    /// La fine della trasmissione (torna al silenzio senza «click»).
    pub fn postamble(&mut self) -> Vec<f32> {
        match self {
            TxModulator::Rtty(m) => m.postamble(),
            TxModulator::Psk(m) => m.postamble(),
            TxModulator::Cw(m) => m.postamble(),
            TxModulator::Olivia(m) => m.postamble(),
            TxModulator::Throb(m) => m.postamble(),
            TxModulator::Amtor(m) => m.postamble(),
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
    charset: RttyCharset,
    /// In attesa si manda LTRS (diddle) invece del solo mark.
    diddle: bool,
    spb: f64,
    phase: f64,
    t_end: f64,
    emitted: usize,
    /// Il registro di chi riceve: `None` = non noto (dopo uno spazio fra le cifre).
    shift: Option<Reg>,
}

impl RttyTx {
    fn new(params: RttyParams, charset: RttyCharset) -> Self {
        Self { params, charset, diddle: true, spb: f64::from(RATE) / f64::from(params.baud), phase: 0.0, t_end: 0.0, emitted: 0, shift: Some(Reg::Letters) }
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
        self.shift = Some(Reg::Letters);
        self.synth(&seq)
    }

    fn char(&mut self, c: char) -> Vec<f32> {
        let mut seq = Vec::new();
        push_char(&mut seq, &mut self.shift, self.charset, c);
        self.synth(&seq)
    }

    fn idle(&mut self) -> Vec<f32> {
        let mut seq = Vec::new();
        if self.diddle {
            push_frame(&mut seq, LTRS);
            self.shift = Some(Reg::Letters);
        } else {
            // senza diddle: il tono di riposo (mark), per la durata di un carattere
            seq.push((true, 7.5));
        }
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
    /// QPSK31: il suo trasmettitore (quattro fasi e codice convoluzionale); `None` negli altri modi.
    qpsk: Option<Box<QpskTx>>,
}

impl PskTx {
    fn new(mode: PskMode, freq_hz: f32) -> Self {
        let qpsk = mode.is_qpsk().then(|| Box::new(QpskTx::new(freq_hz, mode.symbol_samples(), RATE, AMP, false)));
        Self { mode, step: 2.0 * PI * f64::from(freq_hz) / f64::from(RATE), phase: 0.0, prev: 0.0, sign: 1.0, qpsk }
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
        let len = self.mode.dcd_bits();
        if let Some(q) = self.qpsk.as_mut() {
            return q.preamble(len);
        }
        let mut out = Vec::new();
        for _ in 0..self.mode.dcd_bits() {
            self.symbol(Some(false), &mut out);
        }
        out
    }

    fn char(&mut self, c: u8) -> Vec<f32> {
        if let Some(q) = self.qpsk.as_mut() {
            return q.char(c);
        }
        let mut out = Vec::new();
        for ch in varicode::encode(c).chars() {
            self.symbol(Some(ch == '1'), &mut out);
        }
        self.symbol(Some(false), &mut out);
        self.symbol(Some(false), &mut out);
        out
    }

    fn idle(&mut self) -> Vec<f32> {
        if let Some(q) = self.qpsk.as_mut() {
            return q.idle();
        }
        let mut out = Vec::new();
        for _ in 0..4 {
            self.symbol(Some(false), &mut out);
        }
        out
    }

    fn postamble(&mut self) -> Vec<f32> {
        if let Some(q) = self.qpsk.as_mut() {
            return q.postamble(crate::qpsk::TAIL_ZEROS);
        }
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

/// Throb e ThrobX a flusso: ogni carattere (o attesa) e' uno o due simboli a 8 kHz che l'interpolatore 2:3 porta a 12 kHz; alla fine
/// esce anche la coda del filtro. Attesa e spazio di ThrobX restano nello stato del trasmettitore fra un pezzo e l'altro.
pub struct ThrobStream {
    tx: crate::throb::ThrobTx,
    interp: Interpolator,
}

impl ThrobStream {
    fn new(v: crate::throb::ThrobVariant, freq_hz: f32, reverse: bool) -> Self {
        Self { tx: crate::throb::ThrobTx::new(v, f64::from(freq_hz), 1.0, reverse), interp: Interpolator::new() }
    }

    fn resample_12k(&mut self, audio: &[f32]) -> Vec<f32> {
        let mut out = Vec::with_capacity(audio.len() * 3 / 2 + 2);
        self.interp.process(audio, &mut out);
        for v in &mut out {
            *v *= AMP;
        }
        out
    }

    fn preamble(&mut self) -> Vec<f32> {
        let a = self.tx.preamble();
        self.resample_12k(&a)
    }

    fn char(&mut self, c: u8) -> Vec<f32> {
        let a = self.tx.char(c);
        self.resample_12k(&a)
    }

    fn idle(&mut self) -> Vec<f32> {
        let a = self.tx.idle();
        self.resample_12k(&a)
    }

    fn postamble(&mut self) -> Vec<f32> {
        let a = self.tx.postamble();
        let mut out = self.resample_12k(&a);
        let mut rest = Vec::new();
        self.interp.flush(&mut rest);
        out.extend(rest.into_iter().map(|v| v * AMP));
        out
    }
}

/// Olivia e Contestia a flusso: i caratteri si raccolgono finche' non riempiono un blocco FEC (uno per bit del simbolo), poi il blocco
/// diventa audio a 8 kHz e l'interpolatore 2:3 lo porta a 12 kHz. Finche' il blocco non e' pieno un carattere non da' audio. In attesa
/// (nessun testo) il blocco a meta' si completa con caratteri nulli (chi riceve non li stampa) e parte, o parte un blocco tutto nullo; alla
/// fine il blocco a meta' si completa allo stesso modo, poi escono la coda del modulatore e quella del filtro. Come la libreria, la
/// trasmissione comincia con un carattere nullo.
pub struct OliviaTx {
    stream: olivia::BlockStream,
    interp: Interpolator,
    buf: Vec<u8>,
}

impl OliviaTx {
    fn new(params: olivia::Params) -> Option<Self> {
        // il ricevitore controlla che i toni stiano nella banda audio: lo stesso controllo vale per la trasmissione
        crate::olivia_rx::Receiver::new(params, 8, 4, 3.0)?;
        Some(Self { stream: olivia::BlockStream::new(params)?, interp: Interpolator::new(), buf: Vec::new() })
    }

    /// Un blocco (completato con zeri) in audio a 12 kHz.
    fn emit(&mut self) -> Vec<f32> {
        let block = self.stream.block(&self.buf);
        self.buf.clear();
        self.to_12k(&block)
    }

    fn to_12k(&mut self, audio: &[f32]) -> Vec<f32> {
        let mut out = Vec::with_capacity(audio.len() * 3 / 2 + 2);
        self.interp.process(audio, &mut out);
        for v in &mut out {
            *v *= AMP;
        }
        out
    }

    /// Un breve silenzio (il PTT si apre) e il carattere nullo d'inizio.
    fn preamble(&mut self) -> Vec<f32> {
        self.buf = vec![0];
        vec![0.0; (0.05 * f64::from(RATE)) as usize]
    }

    fn char(&mut self, c: u8) -> Vec<f32> {
        self.buf.push(c);
        if self.buf.len() >= self.stream.chars_per_block() { self.emit() } else { Vec::new() }
    }

    fn idle(&mut self) -> Vec<f32> {
        self.emit()
    }

    fn postamble(&mut self) -> Vec<f32> {
        let mut out = if self.buf.is_empty() { Vec::new() } else { self.emit() };
        let tail = self.stream.finish();
        out.extend(self.to_12k(&tail));
        let mut rest = Vec::new();
        self.interp.flush(&mut rest);
        out.extend(rest.into_iter().map(|v| v * AMP));
        out
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
        assert_eq!(whole(&mut TxModulator::rtty(p, RttyCharset::Ita2), TEXT), rtty::modulate(TEXT.as_bytes(), p, AMP));
        let t = "CQ ПРИВЕТ 599 ЧАЙ DE IU3ABC";
        assert_eq!(whole(&mut TxModulator::rtty(p, RttyCharset::Cyrillic), t), rtty::modulate_charset(t, p, AMP, RttyCharset::Cyrillic));
    }

    #[test]
    fn psk_a_pezzi_e_come_l_onda_intera() {
        for mode in [PskMode::Psk31, PskMode::Psk63] {
            assert_eq!(whole(&mut TxModulator::psk(mode, 1500.0), TEXT), psk::modulate(TEXT.as_bytes(), 1500.0, mode, AMP));
        }
    }

    /// Throb a pezzi (anche l'attesa in mezzo, che in ThrobX scambia attesa e spazio): lo stesso audio del trasmettitore intero portato a
    /// 12 kHz.
    #[test]
    fn throb_a_pezzi_e_come_l_onda_intera() {
        use crate::throb::{ThrobTx, ThrobVariant};
        for v in [ThrobVariant::Throb4, ThrobVariant::ThrobX4] {
            let mut m = TxModulator::throb(v, 1500.0, false);
            let mut got = m.preamble();
            got.extend(m.token("A"));
            got.extend(m.token(" "));
            got.extend(m.idle());
            got.extend(m.token("B"));
            got.extend(m.postamble());
            let mut t = ThrobTx::new(v, 1500.0, 1.0, false);
            let mut a8 = t.preamble();
            a8.extend(t.char(b'A'));
            a8.extend(t.char(b' '));
            a8.extend(t.idle());
            a8.extend(t.char(b'B'));
            a8.extend(t.postamble());
            let mut i = Interpolator::new();
            let mut want = Vec::new();
            i.process(&a8, &mut want);
            i.flush(&mut want);
            let want: Vec<f32> = want.into_iter().map(|x| x * AMP).collect();
            assert_eq!(got, want, "{v:?}");
        }
    }

    /// Amtor a pezzi (con l'attesa in mezzo) e' l'onda del trasmettitore con gli stessi passi.
    #[test]
    fn amtor_a_pezzi_e_come_l_onda_intera() {
        for rev in [false, true] {
            let mut m = TxModulator::amtor(1500.0, rev);
            let mut got = m.preamble();
            got.extend(m.token("A"));
            got.extend(m.idle());
            got.extend(m.token("1"));
            got.extend(m.postamble());
            let mut t = crate::amtor::tx_12k(1500.0, AMP, rev);
            let mut want = t.preamble();
            want.extend(t.char(b'A'));
            want.extend(t.idle());
            want.extend(t.char(b'1'));
            want.extend(t.postamble());
            assert_eq!(got, want);
        }
    }

    #[test]
    fn qpsk31_a_pezzi_e_come_l_onda_intera() {
        for rev in [false, true] {
            let want = psk::modulate_qpsk(TEXT.as_bytes(), 1500.0, PskMode::Qpsk31, AMP, rev);
            assert_eq!(whole(&mut TxModulator::psk_reverse(PskMode::Qpsk31, 1500.0, rev), TEXT), want);
        }
        assert_eq!(whole(&mut TxModulator::psk(PskMode::Qpsk31, 1500.0), TEXT), psk::modulate(TEXT.as_bytes(), 1500.0, PskMode::Qpsk31, AMP));
    }

    #[test]
    fn cw_a_pezzi_e_come_l_onda_intera() {
        assert_eq!(whole(&mut TxModulator::cw(700.0, 25.0), TEXT), cw::modulate(TEXT.as_bytes(), 700.0, 25.0, AMP));
    }

    /// Olivia a pezzi: lo stesso audio del trasmettitore intero (a 8 kHz) portato a 12 kHz, dopo il silenzio d'inizio.
    #[test]
    fn olivia_a_pezzi_e_come_l_onda_intera() {
        for (variant, tones, bw) in [(olivia::Variant::Olivia, 8usize, 250usize), (olivia::Variant::Olivia, 32, 1000), (olivia::Variant::Contestia, 16, 500)] {
            let p = olivia::Params { variant, tones, bandwidth: bw, center_hz: 1500.0 };
            let mut m = TxModulator::olivia(p).expect("parametri validi");
            let ours = whole(&mut m, "CQ DE IU3ABC IO91WM");
            let ref8 = olivia::Transmitter::new(p).expect("parametri validi").transmit(b"CQ DE IU3ABC IO91WM");
            let mut it = Interpolator::new();
            let mut want = vec![0.0f32; (0.05 * f64::from(RATE)) as usize];
            let mut up = Vec::new();
            it.process(&ref8, &mut up);
            it.flush(&mut up);
            want.extend(up.into_iter().map(|v| v * AMP));
            assert_eq!(ours.len(), want.len(), "{variant:?} {tones}/{bw}");
            let err = ours.iter().zip(&want).map(|(a, b)| (a - b).abs()).fold(0.0f32, f32::max);
            assert!(err < 1e-6, "{variant:?} {tones}/{bw}: differenza {err}");
        }
        // toni fuori dalla banda audio: niente modulatore
        assert!(TxModulator::olivia(olivia::Params { variant: olivia::Variant::Olivia, tones: 32, bandwidth: 2000, center_hz: 3500.0 }).is_none());
    }

    #[test]
    fn in_cw_un_segno_di_procedura_a_meta_si_aspetta() {
        let m = TxModulator::cw(700.0, 25.0);
        let t: Vec<char> = "<S".chars().collect();
        assert_eq!(m.next_token(&t, true), None);
        assert_eq!(m.next_token(&t, false), Some(1));
        let t: Vec<char> = "<sk> ".chars().collect();
        assert_eq!(m.next_token(&t, true), Some(4));
        assert_eq!(TxModulator::rtty(RttyParams::default(), RttyCharset::Ita2).next_token(&t, true), Some(1));
    }

    #[test]
    fn l_attesa_rtty_si_decodifica_come_niente() {
        // preambolo, «CQ», due tratti di attesa, «DE»: chi riceve legge «CQDE» (gli LTRS non stampano)
        let p = RttyParams::default();
        let mut m = TxModulator::rtty(p, RttyCharset::Ita2);
        let mut audio = m.preamble();
        audio.extend(m.token("C"));
        audio.extend(m.token("Q"));
        audio.extend(m.idle());
        audio.extend(m.idle());
        audio.extend(m.token("D"));
        audio.extend(m.token("E"));
        audio.extend(m.postamble());
        audio.extend(vec![0.0; 6000]);
        let mut rx = rtty::RttyDemodulator::new(p);
        assert_eq!(String::from_utf8_lossy(&rx.process(&audio)), "CQDE");
    }

    #[test]
    fn senza_diddle_l_attesa_rtty_e_solo_il_tono_di_riposo() {
        let p = RttyParams::default();
        let mut con = TxModulator::rtty(p, RttyCharset::Ita2);
        let mut senza = TxModulator::rtty(p, RttyCharset::Ita2).with_diddle(false);
        let (a, b) = (con.idle(), senza.idle());
        // stessa durata di un carattere, ma senza il diddle l'onda e' un tono solo (mark)
        assert_eq!(a.len(), b.len());
        assert_ne!(a, b);
        // il testo prima e dopo l'attesa arriva intatto
        let mut m = TxModulator::rtty(p, RttyCharset::Ita2).with_diddle(false);
        let mut audio = m.preamble();
        audio.extend(m.token("C"));
        audio.extend(m.token("Q"));
        audio.extend(m.idle());
                audio.extend(m.token("D"));
        audio.extend(m.token("E"));
        audio.extend(m.postamble());
        audio.extend(vec![0.0; 6000]);
        let mut rx = rtty::RttyDemodulator::new(p);
        // il ricevitore di prova puo' stampare un carattere vuoto nel lungo mark di riposo: conta che il testo arrivi intatto prima e dopo
        let text = String::from_utf8_lossy(&rx.process(&audio)).to_string();
        assert!(text.starts_with("CQ") && text.ends_with("DE"), "ricevuto: {text:?}");
    }
}
