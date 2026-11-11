//! RTTY (Baudot ITA2, 5 bit): FSK a fase continua, di base 45,45 baud e 170 Hz di spostamento.
//!
//! Ogni carattere è un bit di start (space), cinque bit di dati dal meno significativo (mark = 1) e 1,5 bit di stop (mark). I codici
//! 27 (FIGS) e 31 (LTRS) cambiano la tabella. Tabelle riprese da fldigi (`src/rtty/rtty.cxx`, Dave Freese W1HKJ e altri; GPL 3 o
//! successiva), versione americana delle cifre.

use crate::psk::{AFC_LIMIT_HZ, RATE};
use std::f64::consts::PI;

const LETTERS: [u8; 32] = [
    0, b'E', b'\n', b'A', b' ', b'S', b'I', b'U', b'\r', b'D', b'R', b'J', b'N', b'F', b'C', b'K', b'T', b'Z', b'L', b'W', b'H', b'Y', b'P',
    b'Q', b'O', b'B', b'G', b' ', b'M', b'X', b'V', b' ',
];
const FIGURES: [u8; 32] = [
    0, b'3', b'\n', b'-', b' ', 7, b'8', b'7', b'\r', b'$', b'4', b'\'', b',', b'!', b':', b'(', b'5', b'"', b')', b'2', b'#', b'6', b'0',
    b'1', b'9', b'?', b'&', b' ', b'.', b'/', b';', b' ',
];
/// Distanza dei filtri di controllo dell'AFC dai due toni (Hz).
const PROBE_HZ: f32 = 8.0;
/// Distanza dei due filtri del rumore (uno sotto il mark, uno sopra lo space): servono a capire se c'e' un segnale RTTY stretto o solo rumore.
const NOISE_GAP_HZ: f32 = 110.0;
/// L'AFC si accende quando i due toni superano il rumore di questo rapporto (in ampiezza, circa 11 dB) e si spegne sotto l'altro (circa 6 dB).
const LOCK_ON: f32 = 3.5;
const LOCK_OFF: f32 = 2.0;
/// Quanto sposta l'AFC a ogni bit (Hz per unita' di errore), e dopo quanti bit senza segnale (circa 8 s) la correzione rientra piano verso la frequenza scelta.
const AFC_GAIN: f32 = 0.4;
const AFC_HOLD_BITS: u32 = 360;
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

/// Audio (a 12 kHz) del testo; `amp` è l'ampiezza di picco (0..1). I caratteri che il Baudot non ha si saltano.
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
    /// Cambia la frequenza senza perdere la fase.
    fn retune(&mut self, freq_hz: f32) {
        self.step = 2.0 * PI * f64::from(freq_hz) / f64::from(RATE);
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
    params: RttyParams,
    /// Correzione dell'AFC sulla frequenza scelta (Hz).
    offset_hz: f32,
    afc: bool,
    squelch_db: Option<f32>,
    level_db: f32,
    open: bool,
    /// Quattro filtri appena sopra e sotto i due toni: dicono da che parte la portante e' scivolata.
    probes: [Tone; 4],
    probe_count: usize,
    /// Due filtri lontani dai toni per misurare il rumore, e le medie lente che servono all'AFC: ampiezza dei due toni, del rumore, errore.
    noise: [Tone; 2],
    mark_avg: f32,
    space_avg: f32,
    noise_avg: f32,
    err_avg: f32,
    /// C'e' un segnale RTTY (l'AFC lavora solo allora) e da quanti bit non c'e' piu'.
    locked: bool,
    lost_bits: u32,
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
        let s = params.mark_hz + params.shift_hz;
        Self {
            params,
            offset_hz: 0.0,
            afc: false,
            squelch_db: None,
            level_db: -90.0,
            open: true,
            probes: [
                Tone::new(params.mark_hz + PROBE_HZ, len),
                Tone::new(params.mark_hz - PROBE_HZ, len),
                Tone::new(s + PROBE_HZ, len),
                Tone::new(s - PROBE_HZ, len),
            ],
            probe_count: 0,
            noise: [Tone::new(params.mark_hz - NOISE_GAP_HZ, len), Tone::new(s + NOISE_GAP_HZ, len)],
            mark_avg: 0.0,
            space_avg: 0.0,
            noise_avg: 0.0,
            err_avg: 0.0,
            locked: false,
            lost_bits: 0,
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

    /// Il mark su cui si sta ascoltando adesso (con l'AFC: la frequenza scelta piu' la correzione).
    pub fn frequency(&self) -> f32 {
        self.params.mark_hz + self.offset_hz
    }

    pub fn set_afc(&mut self, on: bool) {
        self.afc = on;
        if !on && self.offset_hz != 0.0 {
            self.offset_hz = 0.0;
            self.retune_all();
        }
    }

    /// `None` spegne lo squelch.
    pub fn set_squelch_db(&mut self, db: Option<f32>) {
        self.squelch_db = db;
    }

    /// Livello del segnale in dB e se lo squelch lo lascia passare.
    pub fn signal(&self) -> (f32, bool) {
        (self.level_db, self.open)
    }

    fn retune_all(&mut self) {
        let mark = self.params.mark_hz + self.offset_hz;
        let space = mark + self.params.shift_hz;
        self.mark.retune(mark);
        self.space.retune(space);
        for (i, t) in self.probes.iter_mut().enumerate() {
            let f = if i < 2 { mark } else { space };
            t.retune(f + if i % 2 == 0 { PROBE_HZ } else { -PROBE_HZ });
        }
        self.noise[0].retune(mark - NOISE_GAP_HZ);
        self.noise[1].retune(space + NOISE_GAP_HZ);
    }

    /// L'AFC, una volta per bit. Si muove solo se c'e' un segnale RTTY: i due toni devono superare di molto il rumore misurato fuori dalla coppia
    /// (un segnale largo, la voce o il solo rumore alzano anche quello e non passano). Senza segnale resta fermo e, dopo circa 8 secondi,
    /// rientra piano verso la frequenza scelta, cosi' la stazione dopo si cerca da li'.
    fn afc_step(&mut self, m: f32, sp: f32, p: &[f32; 4], nz: &[f32; 2]) {
        const SMOOTH: f32 = 0.15;
        self.mark_avg += SMOOTH * (m - self.mark_avg);
        self.space_avg += SMOOTH * (sp - self.space_avg);
        self.noise_avg += SMOOTH * (nz[0] + nz[1] - self.noise_avg);
        let signal = self.mark_avg + self.space_avg;
        let snr = signal / (self.noise_avg + 1e-4);
        if self.locked {
            if snr < LOCK_OFF || signal < 1e-3 {
                self.locked = false;
            }
        } else if snr > LOCK_ON && signal > 2e-3 {
            self.locked = true;
        }
        if self.locked && self.open {
            self.lost_bits = 0;
            // da che parte e' scivolata la portante, pesando ogni tono per quanto e' presente (in pausa trasmette solo il mark)
            let em = (p[0] - p[1]) / (p[0] + p[1]).max(1e-6);
            let es = (p[2] - p[3]) / (p[2] + p[3]).max(1e-6);
            let (wm, ws) = (self.mark_avg / signal.max(1e-6), self.space_avg / signal.max(1e-6));
            self.err_avg += 0.3 * (wm * em + ws * es - self.err_avg);
            if self.err_avg.abs() > 0.03 {
                self.offset_hz = (self.offset_hz + AFC_GAIN * self.err_avg).clamp(-AFC_LIMIT_HZ, AFC_LIMIT_HZ);
                self.retune_all();
            }
            return;
        }
        self.err_avg = 0.0;
        self.lost_bits = self.lost_bits.saturating_add(1);
        if self.lost_bits > AFC_HOLD_BITS && self.offset_hz != 0.0 {
            self.offset_hz -= self.offset_hz.signum() * self.offset_hz.abs().min(0.05);
            self.retune_all();
        }
    }

    /// Elabora altro audio; restituisce i caratteri nuovi.
    pub fn process(&mut self, samples: &[f32]) -> Vec<u8> {
        let mut out = Vec::new();
        for &s in samples {
            let m = self.mark.push(s, self.pos);
            let sp = self.space.push(s, self.pos);
            let p: [f32; 4] = std::array::from_fn(|i| self.probes[i].push(s, self.pos));
            let nz: [f32; 2] = std::array::from_fn(|i| self.noise[i].push(s, self.pos));
            self.pos = (self.pos + 1) % self.len;
            let d = m - sp;
            let level = m + sp;
            self.level_db = (20.0 * level.max(1e-6).log10()).max(-90.0);
            self.open = self.squelch_db.is_none_or(|t| self.level_db >= t);
            self.probe_count += 1;
            if self.afc && self.probe_count >= self.len {
                self.probe_count = 0;
                self.afc_step(m, sp, &p, &nz);
            }
            match self.state {
                State::Idle => {
                    // il filtro vede il bit a metà finestra: l'inizio del carattere è mezzo bit prima, il primo campionamento un bit dopo
                    if self.prev_d > 0.0 && d <= 0.0 && level > 1e-3 && self.open {
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
    fn l_afc_insegue_i_toni() {
        let tx = RttyParams { mark_hz: 2137.0, shift_hz: 170.0, baud: 45.45 };
        let audio = modulate(b"RYRYRY CQ CQ DE IU3ABC IU3ABC PSE K RYRYRY", tx, 0.5);
        let mut rx = RttyDemodulator::new(RttyParams::default());
        rx.set_afc(true);
        let mut all = vec![0.0; 4000];
        all.extend(audio);
        all.extend(vec![0.0; 4000]);
        let r = String::from_utf8_lossy(&rx.process(&all)).into_owned();
        assert!((rx.frequency() - 2137.0).abs() < 6.0, "frequenza raggiunta: {}", rx.frequency());
        assert!(r.contains("IU3ABC"), "ricevuto: {r:?}");
    }

    fn rumore(n: usize, ampiezza: f32, mut seed: u32) -> Vec<f32> {
        (0..n)
            .map(|_| {
                seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
                (seed as f32 / u32::MAX as f32 - 0.5) * ampiezza
            })
            .collect()
    }

    #[test]
    fn l_afc_non_si_muove_sul_solo_rumore() {
        let mut rx = RttyDemodulator::new(RttyParams::default());
        rx.set_afc(true);
        // dieci secondi di rumore, anche forte: la frequenza resta quella scelta
        for (i, amp) in [0.02_f32, 0.2, 0.8].into_iter().enumerate() {
            rx.process(&rumore(120_000, amp, 11 + i as u32));
            assert!((rx.frequency() - 2125.0).abs() < 0.5, "con rumore {amp} la frequenza e' andata a {}", rx.frequency());
        }
    }

    #[test]
    fn l_afc_non_segue_un_tono_fisso_dentro_il_rumore_forte() {
        // un solo tono a 2140 Hz sepolto in un rumore molto piu' forte non e' un segnale RTTY
        let mut rx = RttyDemodulator::new(RttyParams::default());
        rx.set_afc(true);
        let tono: Vec<f32> = rumore(120_000, 0.8, 5).iter().enumerate().map(|(i, r)| r + 0.05 * (2.0 * std::f32::consts::PI * 2140.0 * i as f32 / 12_000.0).sin()).collect();
        rx.process(&tono);
        assert!((rx.frequency() - 2125.0).abs() < 1.0, "frequenza: {}", rx.frequency());
    }

    #[test]
    fn l_afc_con_rumore_insegue_il_segnale_e_poi_rientra() {
        let tx = RttyParams { mark_hz: 2140.0, shift_hz: 170.0, baud: 45.45 };
        let mut audio = modulate(b"RYRYRYRYRYRY CQ CQ DE IU3ABC IU3ABC PSE K RYRYRYRYRY", tx, 0.4);
        for (a, r) in audio.iter_mut().zip(rumore(200_000, 0.1, 3)) {
            *a += r;
        }
        let mut rx = RttyDemodulator::new(RttyParams::default());
        rx.set_afc(true);
        rx.process(&audio);
        assert!((rx.frequency() - 2140.0).abs() < 6.0, "frequenza raggiunta: {}", rx.frequency());
        // finito il segnale: ferma per circa 8 s, poi rientra piano verso la frequenza scelta
        rx.process(&rumore(60_000, 0.1, 9));
        let ferma = rx.frequency();
        assert!((ferma - 2140.0).abs() < 8.0, "dopo 5 s senza segnale la frequenza si e' mossa: {ferma}");
        rx.process(&rumore(1_200_000, 0.1, 10));
        assert!((rx.frequency() - 2125.0).abs() < 1.0, "dopo molto tempo senza segnale non e' rientrata: {}", rx.frequency());
    }

    #[test]
    fn lo_squelch_tiene_chiusa_la_ricezione_senza_segnale() {
        let mut seed = 7_u32;
        let noise: Vec<f32> = (0..24_000)
            .map(|_| {
                seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
                (seed as f32 / u32::MAX as f32 - 0.5) * 0.2
            })
            .collect();
        let mut rx = RttyDemodulator::new(RttyParams::default());
        rx.set_squelch_db(Some(-35.0));
        assert!(rx.process(&noise).is_empty());
        assert!(!rx.signal().1);
        let audio = modulate(b"CQ DE IU3ABC", RttyParams::default(), 0.5);
        let r = String::from_utf8_lossy(&rx.process(&audio)).into_owned();
        assert!(r.contains("IU3ABC"), "ricevuto: {r:?}");
    }

    #[test]
    fn baud_50() {
        let p = RttyParams { mark_hz: 2125.0, shift_hz: 450.0, baud: 50.0 };
        let r = andata_e_ritorno(p, "CQ DE IU3ABC");
        assert!(r.contains("CQ DE IU3ABC"), "ricevuto: {r:?}");
    }
}
