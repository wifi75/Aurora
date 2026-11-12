//! Click di manipolazione: l'energia che sale insieme su una larga parte della banda per pochi millisecondi (il fronte duro di una
//! stazione CW, una scarica breve). Sulla cascata diventano righe orizzontali che coprono i segnali; per i canali CW di tutta la banda sono
//! istanti in cui l'ampiezza non dice niente.
//!
//! [`ClickDetector`] lavora sull'audio con spettri brevi (circa 5 ms, uno ogni 2,7 ms): per ogni riga di frequenza tiene il livello
//! tipico (un quantile che sale e scende piano, che i click rari non spostano); in uno spettro un tratto di 1 kHz «sale» se piu' del 30%
//! delle sue righe (e almeno due) sta almeno 12 dB sopra il livello tipico. Se in un tratto la salita dura meno di 10 ms e' un click; se
//! dura di piu' e' un'altra cosa (il tono acceso di una stazione forte, uno scroscio) e resta. Ogni tratto si guarda da solo: il fronte di
//! una stazione forte fa un click lontano dal suo tono anche se vicino al tono la salita dura quanto la linea. Gli istanti dei click si
//! leggono come campioni assoluti ([`ClickSpan`]).
//!
//! [`SpectrumAnalyzer`](crate::SpectrumAnalyzer) li usa per la cascata («anti-click»): prima della trasformata delle righe i campioni del
//! click si spengono con fronti morbidi, cosi' la riga orizzontale sparisce e i segnali restano (la finestra delle righe e' lunga 0,14 s:
//! qualche millisecondo spento non si vede).

use std::sync::Arc;

use rustfft::{num_complex::Complex, Fft, FftPlanner};

/// Durata dello spettro breve (s) e passo (meta').
const FRAME_S: f32 = 0.005;
/// Righe di frequenza considerate (Hz) e larghezza dei tratti (Hz).
const BAND_LO_HZ: f32 = 200.0;
const BAND_HI_HZ: f32 = 4_800.0;
const BLOCK_HZ: f32 = 1_000.0;
/// Una riga «sale» se sta tanti dB sopra il suo livello tipico; uno spettro e' largo se in un tratto salgono piu' di tante righe.
const RISE_DB: f32 = 12.0;
const BROAD_SHARE: f32 = 0.3;
/// Un click dura meno di cosi' (s).
const MAX_CLICK_S: f32 = 0.010;
/// Livello tipico: quantile 50% in dB, passo per spettro (dB); i primi spettri lo fissano subito.
const TYPICAL_STEP_DB: f32 = 0.05;
const WARMUP_FRAMES: usize = 40;

/// Un click: dal campione `start` (compreso) a `end` (escluso), contati dall'inizio del flusso.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ClickSpan {
    pub start: u64,
    pub end: u64,
}

/// Il rivelatore dei click (vedi il modulo).
pub struct ClickDetector {
    fft: Arc<dyn Fft<f32>>,
    n: usize,
    hop: usize,
    window: Vec<f32>,
    /// Gli ultimi `n` campioni (ad anello, `pos` e' il prossimo da scrivere) e quanti ne sono arrivati in tutto.
    buf: Vec<f32>,
    pos: usize,
    filled: usize,
    total: u64,
    since_frame: usize,
    spec: Vec<Complex<f32>>,
    scratch: Vec<Complex<f32>>,
    /// Righe considerate, il tratto di ognuna e quante righe ha ogni tratto.
    bins: std::ops::Range<usize>,
    block_of: Vec<usize>,
    block_len: Vec<usize>,
    typical_db: Vec<f32>,
    frames: usize,
    /// Per ogni tratto la salita in corso: primo campione e quanti spettri.
    run_start: Vec<u64>,
    run: Vec<usize>,
    max_run: usize,
}

impl ClickDetector {
    pub fn new(sample_rate: u32) -> Self {
        let rate = sample_rate.max(1) as f32;
        let n = ((FRAME_S * rate) as usize).next_power_of_two().max(16);
        let hop = n / 2;
        let window = (0..n).map(|i| 0.5 - 0.5 * (2.0 * std::f32::consts::PI * (i as f32 + 0.5) / n as f32).cos()).collect();
        let fft = FftPlanner::<f32>::new().plan_fft_forward(n);
        let scratch = vec![Complex::new(0.0, 0.0); fft.get_inplace_scratch_len()];
        let hz = rate / n as f32;
        let lo = ((BAND_LO_HZ / hz).ceil() as usize).max(1);
        let hi = ((BAND_HI_HZ.min(0.48 * rate) / hz).floor() as usize).max(lo + 1).min(n / 2);
        let block_of: Vec<usize> = (lo..hi).map(|k| ((k as f32 * hz - BAND_LO_HZ) / BLOCK_HZ).max(0.0) as usize).collect();
        let blocks = block_of.iter().copied().max().map_or(1, |b| b + 1);
        let mut block_len = vec![0; blocks];
        for &b in &block_of {
            block_len[b] += 1;
        }
        let max_run = ((MAX_CLICK_S * rate) / hop as f32).floor().max(1.0) as usize;
        ClickDetector {
            fft,
            n,
            hop,
            window,
            buf: vec![0.0; n],
            pos: 0,
            filled: 0,
            total: 0,
            since_frame: 0,
            spec: vec![Complex::new(0.0, 0.0); n],
            scratch,
            bins: lo..hi,
            block_of,
            block_len,
            typical_db: vec![-200.0; hi - lo],
            frames: 0,
            run_start: vec![0; blocks],
            run: vec![0; blocks],
            max_run,
        }
    }

    /// Campioni arrivati finora.
    pub fn samples_seen(&self) -> u64 {
        self.total
    }

    /// Aggiunge campioni; restituisce i click finiti in questo blocco.
    pub fn push(&mut self, input: &[f32]) -> Vec<ClickSpan> {
        let mut out = Vec::new();
        for &s in input {
            self.buf[self.pos] = if s.is_finite() { s } else { 0.0 };
            self.pos = (self.pos + 1) % self.n;
            self.filled = (self.filled + 1).min(self.n);
            self.total += 1;
            self.since_frame += 1;
            if self.since_frame >= self.hop && self.filled == self.n {
                self.since_frame = 0;
                let rising = self.frame();
                self.track(&rising, &mut out);
            }
        }
        out
    }

    /// Uno spettro breve sugli ultimi `n` campioni: quali tratti salgono.
    fn frame(&mut self) -> Vec<bool> {
        for (i, &w) in self.window.iter().enumerate() {
            self.spec[i] = Complex::new(self.buf[(self.pos + i) % self.n] * w, 0.0);
        }
        self.fft.process_with_scratch(&mut self.spec, &mut self.scratch);
        self.frames += 1;
        let warm = self.frames <= WARMUP_FRAMES;
        let mut rising = vec![0usize; self.block_len.len()];
        for (j, k) in self.bins.clone().enumerate() {
            let db = 10.0 * self.spec[k].norm_sqr().max(1e-20).log10();
            let typ = &mut self.typical_db[j];
            if warm {
                // all'avvio il livello tipico e' la media dei primi spettri
                *typ = if self.frames == 1 { db } else { *typ + (db - *typ) / self.frames as f32 };
                continue;
            }
            if db > *typ + RISE_DB {
                rising[self.block_of[j]] += 1;
            }
            *typ += TYPICAL_STEP_DB * if db > *typ { 1.0 } else { -1.0 };
        }
        rising.iter().zip(&self.block_len).map(|(&r, &len)| !warm && len > 0 && r >= 2 && r as f32 > BROAD_SHARE * len as f32).collect()
    }

    fn track(&mut self, rising: &[bool], out: &mut Vec<ClickSpan>) {
        for (b, &up) in rising.iter().enumerate() {
            if up {
                if self.run[b] == 0 {
                    // lo spettro copre gli ultimi `n` campioni
                    self.run_start[b] = self.total.saturating_sub(self.n as u64);
                }
                self.run[b] += 1;
                continue;
            }
            if self.run[b] > 0 && self.run[b] <= self.max_run {
                // il click sta fra l'inizio del primo spettro e la fine dell'ultimo; un click visto da piu' tratti si unisce
                let span = ClickSpan { start: self.run_start[b], end: self.total.saturating_sub(self.hop as u64).max(self.run_start[b] + 1) };
                match out.last_mut() {
                    Some(last) if span.start <= last.end && span.end >= last.start => {
                        last.start = last.start.min(span.start);
                        last.end = last.end.max(span.end);
                    }
                    _ => out.push(span),
                }
            }
            self.run[b] = 0;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn noise(n: usize, amp: f32, seed: u64) -> Vec<f32> {
        let mut x = seed | 1;
        (0..n)
            .map(|_| {
                x ^= x >> 12;
                x ^= x << 25;
                x ^= x >> 27;
                ((x.wrapping_mul(0x2545_F491_4F6C_DD1D) >> 40) as f32 / (1u64 << 23) as f32 - 1.0) * amp
            })
            .collect()
    }

    #[test]
    fn a_short_broadband_burst_is_a_click() {
        let mut audio = noise(24_000, 0.01, 5);
        // un impulso di 1 ms a 1 s
        for (i, s) in audio[12_000..12_012].iter_mut().enumerate() {
            *s += if i % 2 == 0 { 0.8 } else { -0.8 };
        }
        let mut d = ClickDetector::new(12_000);
        let clicks = d.push(&audio);
        assert_eq!(clicks.len(), 1, "{clicks:?}");
        let c = clicks[0];
        assert!(c.start <= 12_000 && c.end >= 12_012 && c.end - c.start < 300, "{c:?}");
    }

    #[test]
    fn tones_and_long_bursts_are_not_clicks() {
        let mut audio = noise(36_000, 0.01, 9);
        // un tono che parte con il fronte morbido e resta acceso
        for (i, s) in audio.iter_mut().enumerate().skip(6_000) {
            let ramp = ((i - 6_000) as f32 / 60.0).min(1.0);
            *s += 0.5 * ramp * (2.0 * std::f32::consts::PI * 700.0 * i as f32 / 12_000.0).sin();
        }
        // uno scroscio di rumore largo di 100 ms
        let burst = noise(1_200, 0.3, 77);
        for (s, b) in audio[24_000..25_200].iter_mut().zip(burst) {
            *s += b;
        }
        let mut d = ClickDetector::new(12_000);
        assert!(d.push(&audio).is_empty());
    }

    #[test]
    fn it_works_at_48_khz() {
        let mut audio = noise(96_000, 0.01, 3);
        for (i, s) in audio[48_000..48_096].iter_mut().enumerate() {
            *s += if i % 2 == 0 { 0.8 } else { -0.8 };
        }
        let mut d = ClickDetector::new(48_000);
        let clicks = d.push(&audio);
        assert_eq!(clicks.len(), 1, "{clicks:?}");
    }
}
