//! Throb e ThrobX: MFSK lentissimo a coppie di toni (Lionel Sear G3PPT), a 8 kHz.
//!
//! Le convenzioni sono quelle di fldigi 4.2.13 (`src/throb/throb.cxx`, Dave Freese W1HKJ, ThrobX di Joe Veldhuis KD8ATU, da gmfsk di Tomi
//! Manninen OH2BNS; GPL 3, copia in `terze-parti/fldigi`), studiate e riscritte qui; da fldigi vengono le tabelle (toni, coppie, caratteri):
//! - un simbolo dura 8192, 4096 o 2048 campioni a 8 kHz (1, 2 e 4 simboli al secondo); a 1 e 2 simboli l'inviluppo e' «semi» (un quinto di
//!   salita a coseno, tre quinti piatto, un quinto di discesa), a 4 e' un coseno rialzato intero;
//! - Throb: 9 toni a 8 Hz (16 Hz a 4 simboli) intorno al centro, 45 simboli fatti di una coppia di toni o di un tono solo; `?`, `@`, `-` e
//!   l'a capo passano dal simbolo di cambio (shift) seguito da un altro simbolo;
//! - ThrobX: 11 toni a 7,8125 Hz (15,625 Hz a 4 simboli), 55 simboli sempre di due toni; il simbolo di attesa e quello dello spazio si
//!   scambiano a ogni spazio o attesa (cosi' due spazi di fila non sono lo stesso simbolo). ThrobX a 4 simboli e l'a capo di ThrobX sono
//!   aggiunte di fldigi fuori dallo standard;
//! - l'inversione di banda laterale (la radio in LSB) rovescia l'ordine dei toni.
//!
//! Il ricevitore e' nostro: miscelazione al centro, passa-basso FIR e decimazione per 32, correlazione con l'inviluppo di ogni tono,
//! sincronismo scelto sul punto del simbolo con le due correlazioni piu' forti (media su piu' simboli), squelch sul rapporto fra i due toni
//! e gli altri, AFC dalla rotazione di fase del tono piu' forte. Il trattino torna trattino (fldigi lo mostra come `=`, vedi [`decode`]).

use std::f64::consts::PI;

/// La frequenza di campionamento di Throb (Hz).
pub const RATE: f64 = 8000.0;
/// Decimazione del ricevitore: un campione complesso ogni 32 (250 Hz).
const DECIM: usize = 32;
/// Lunghezza del passa-basso davanti alla decimazione.
const LP_TAPS: usize = 512;
/// Taglio del passa-basso (Hz): sopra il tono piu' lontano di ThrobX4 (78 Hz).
const LP_CUT_HZ: f64 = 120.0;

/// Le sei varianti.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum ThrobVariant {
    Throb1,
    #[default]
    Throb2,
    Throb4,
    ThrobX1,
    ThrobX2,
    ThrobX4,
}

const THROB_NAR: [f64; 9] = [-32.0, -24.0, -16.0, -8.0, 0.0, 8.0, 16.0, 24.0, 32.0];
const THROB_WID: [f64; 9] = [-64.0, -48.0, -32.0, -16.0, 0.0, 16.0, 32.0, 48.0, 64.0];
const THROBX_NAR: [f64; 11] = [-39.0625, -31.25, -23.4375, -15.625, -7.8125, 0.0, 7.8125, 15.625, 23.4375, 31.25, 39.0625];
const THROBX_WID: [f64; 11] = [-78.125, -62.5, -46.875, -31.25, -15.625, 0.0, 15.625, 31.25, 46.875, 62.5, 78.125];

/// Throb: le coppie di toni (da 1) e i caratteri dei 45 simboli. 0 = attesa, 5 = cambio (shift), 44 = spazio.
const THROB_PAIRS: [(u8, u8); 45] = [
    (5, 5), (4, 5), (1, 2), (1, 3), (1, 4), (4, 6), (1, 5), (1, 6), (1, 7), (3, 7), (1, 8), (2, 3), (2, 4), (2, 8), (2, 5),
    (5, 6), (2, 6), (2, 9), (3, 4), (3, 5), (1, 9), (3, 6), (8, 9), (3, 8), (3, 3), (2, 2), (1, 1), (3, 9), (4, 7), (4, 8),
    (4, 9), (5, 7), (5, 8), (5, 9), (6, 7), (6, 8), (6, 9), (7, 8), (7, 9), (8, 8), (7, 7), (6, 6), (4, 4), (9, 9), (2, 7),
];
const THROB_CHARS: &[u8; 45] = b"\0ABCD\0FGHIJKLMNOPQRSTUVWXYZ1234567890,.'/)(E ";
const THROB_IDLE: usize = 0;
const THROB_SHIFT: usize = 5;
const THROB_SPACE: usize = 44;

/// ThrobX: le coppie di toni (da 1) e i caratteri dei 55 simboli. 0 e 1 sono attesa e spazio (all'inizio), poi si scambiano.
const THROBX_PAIRS: [(u8, u8); 55] = [
    (6, 11), (1, 6), (2, 6), (2, 5), (2, 7), (2, 8), (5, 6), (2, 9), (2, 10), (4, 8), (4, 6), (2, 11), (3, 4), (3, 5), (3, 6),
    (6, 9), (6, 10), (3, 7), (3, 8), (3, 9), (6, 8), (6, 7), (3, 10), (3, 11), (4, 5), (4, 7), (4, 9), (4, 10), (1, 2), (1, 3),
    (1, 4), (1, 5), (1, 7), (1, 8), (1, 9), (1, 10), (2, 3), (2, 4), (4, 11), (5, 7), (5, 8), (5, 9), (5, 10), (5, 11), (7, 8),
    (7, 9), (7, 10), (7, 11), (8, 9), (8, 10), (8, 11), (9, 10), (9, 11), (10, 11), (1, 11),
];
const THROBX_CHARS: &[u8; 55] = b"\0 ABCDEFGHIJKLMNOPQRSTUVWXYZ1234567890,.'/)(#\"+-;:?!@=\n";

impl ThrobVariant {
    pub const ALL: [ThrobVariant; 6] =
        [ThrobVariant::Throb1, ThrobVariant::Throb2, ThrobVariant::Throb4, ThrobVariant::ThrobX1, ThrobVariant::ThrobX2, ThrobVariant::ThrobX4];

    /// Campioni per simbolo a 8 kHz.
    pub fn symbol_len(self) -> usize {
        match self {
            ThrobVariant::Throb1 | ThrobVariant::ThrobX1 => 8192,
            ThrobVariant::Throb2 | ThrobVariant::ThrobX2 => 4096,
            ThrobVariant::Throb4 | ThrobVariant::ThrobX4 => 2048,
        }
    }

    pub fn is_x(self) -> bool {
        matches!(self, ThrobVariant::ThrobX1 | ThrobVariant::ThrobX2 | ThrobVariant::ThrobX4)
    }

    /// Le frequenze dei toni rispetto al centro (Hz).
    pub fn tones(self) -> &'static [f64] {
        match self {
            ThrobVariant::Throb1 | ThrobVariant::Throb2 => &THROB_NAR,
            ThrobVariant::Throb4 => &THROB_WID,
            ThrobVariant::ThrobX1 | ThrobVariant::ThrobX2 => &THROBX_NAR,
            ThrobVariant::ThrobX4 => &THROBX_WID,
        }
    }

    /// La larghezza fra il primo e l'ultimo tono (Hz).
    pub fn bandwidth_hz(self) -> f64 {
        let t = self.tones();
        t[t.len() - 1] - t[0]
    }

    fn pairs(self) -> &'static [(u8, u8)] {
        if self.is_x() { &THROBX_PAIRS } else { &THROB_PAIRS }
    }

    fn chars(self) -> &'static [u8] {
        if self.is_x() { THROBX_CHARS } else { THROB_CHARS }
    }

    pub fn id(self) -> &'static str {
        match self {
            ThrobVariant::Throb1 => "throb1",
            ThrobVariant::Throb2 => "throb2",
            ThrobVariant::Throb4 => "throb4",
            ThrobVariant::ThrobX1 => "throbx1",
            ThrobVariant::ThrobX2 => "throbx2",
            ThrobVariant::ThrobX4 => "throbx4",
        }
    }

    /// Il nome per l'interfaccia e il log (sottomodo ADIF).
    pub fn name(self) -> &'static str {
        match self {
            ThrobVariant::Throb1 => "THROB1",
            ThrobVariant::Throb2 => "THROB2",
            ThrobVariant::Throb4 => "THROB4",
            ThrobVariant::ThrobX1 => "THRBX1",
            ThrobVariant::ThrobX2 => "THRBX2",
            ThrobVariant::ThrobX4 => "THRBX4",
        }
    }

    pub fn from_id(id: &str) -> Option<ThrobVariant> {
        ThrobVariant::ALL.into_iter().find(|v| v.id().eq_ignore_ascii_case(id) || v.name().eq_ignore_ascii_case(id))
    }

    /// L'inviluppo di un simbolo di `len` campioni.
    fn pulse(self, len: usize) -> Vec<f64> {
        let full = matches!(self, ThrobVariant::Throb4 | ThrobVariant::ThrobX4);
        (0..len)
            .map(|i| {
                if full {
                    return 0.5 * (1.0 - (2.0 * PI * i as f64 / len as f64).cos());
                }
                let edge = len as f64 / 5.0;
                let x = i as f64;
                if x < edge {
                    0.5 * (1.0 - (PI * x / edge).cos())
                } else if i >= len * 4 / 5 {
                    0.5 * (1.0 + (PI * (i - len * 4 / 5) as f64 / edge).cos())
                } else {
                    1.0
                }
            })
            .collect()
    }
}

/// Lo stato di attesa e spazio di ThrobX (si scambiano a ogni spazio o attesa); in Throb restano fermi.
#[derive(Clone, Copy, Debug)]
struct IdleSpace {
    idle: usize,
    space: usize,
}

impl IdleSpace {
    fn new(v: ThrobVariant) -> Self {
        if v.is_x() { IdleSpace { idle: 0, space: 1 } } else { IdleSpace { idle: THROB_IDLE, space: THROB_SPACE } }
    }

    fn flip(&mut self, v: ThrobVariant) {
        if v.is_x() {
            std::mem::swap(&mut self.idle, &mut self.space);
        }
    }
}

/// I simboli di un carattere (e lo stato di attesa e spazio dopo); un carattere che il modo non ha diventa spazio, CR non si manda.
fn encode(v: ThrobVariant, c: u8, st: &mut IdleSpace) -> Vec<usize> {
    let c = c.to_ascii_uppercase();
    if c == b'\r' {
        return Vec::new();
    }
    if !v.is_x() {
        // Throb: i caratteri dopo il simbolo di cambio (come fldigi)
        let shifted = match c {
            b'?' => Some(20),
            b'@' => Some(13),
            b'-' => Some(9),
            b'\n' => Some(0),
            _ => None,
        };
        if let Some(s) = shifted {
            return vec![THROB_SHIFT, s];
        }
    }
    let chars = v.chars();
    let found = (c != 0 && c != b' ').then(|| chars.iter().rposition(|&x| x == c)).flatten();
    // in ThrobX i simboli 0 e 1 (attesa e spazio iniziali) non sono caratteri: lo spazio segue lo stato
    let found = found.filter(|&i| !v.is_x() || i > 1);
    match found {
        Some(i) => vec![i],
        None => {
            let s = st.space;
            st.flip(v);
            vec![s]
        }
    }
}

/// Il trasmettitore a 8 kHz.
pub struct ThrobTx {
    v: ThrobVariant,
    reverse: bool,
    freq_hz: f64,
    amp: f64,
    pulse: Vec<f64>,
    st: IdleSpace,
}

impl ThrobTx {
    /// `freq_hz`: il centro (il tono di mezzo); `reverse`: la radio in LSB (toni rovesciati).
    pub fn new(v: ThrobVariant, freq_hz: f64, amp: f64, reverse: bool) -> Self {
        ThrobTx { v, reverse, freq_hz, amp, pulse: v.pulse(v.symbol_len()), st: IdleSpace::new(v) }
    }

    pub fn variant(&self) -> ThrobVariant {
        self.v
    }

    fn symbol(&self, sym: usize) -> Vec<f32> {
        let (a, b) = self.v.pairs()[sym];
        let n = self.v.tones().len();
        let tone = |t: u8| {
            let i = usize::from(t - 1);
            self.v.tones()[if self.reverse { n - 1 - i } else { i }]
        };
        let w1 = 2.0 * PI * (self.freq_hz + tone(a)) / RATE;
        let w2 = 2.0 * PI * (self.freq_hz + tone(b)) / RATE;
        self.pulse.iter().enumerate().map(|(i, p)| (self.amp * p * ((w1 * i as f64).sin() + (w2 * i as f64).sin()) / 2.0) as f32).collect()
    }

    fn idle_symbol(&mut self) -> Vec<f32> {
        let out = self.symbol(self.st.idle);
        self.st.flip(self.v);
        out
    }

    /// L'inizio: quattro simboli di attesa (come fldigi).
    pub fn preamble(&mut self) -> Vec<f32> {
        (0..4).flat_map(|_| self.idle_symbol()).collect()
    }

    pub fn char(&mut self, c: u8) -> Vec<f32> {
        encode(self.v, c, &mut self.st).into_iter().flat_map(|s| self.symbol(s)).collect()
    }

    /// Niente da mandare: un simbolo di attesa.
    pub fn idle(&mut self) -> Vec<f32> {
        self.idle_symbol()
    }

    /// La fine: un simbolo di attesa (l'inviluppo torna a zero da solo).
    pub fn postamble(&mut self) -> Vec<f32> {
        self.symbol(self.st.idle)
    }

    /// Tutto il testo in un colpo solo (inizio, caratteri, fine).
    pub fn transmit(&mut self, text: &[u8]) -> Vec<f32> {
        let mut out = self.preamble();
        for &c in text {
            out.extend(self.char(c));
        }
        out.extend(self.postamble());
        out
    }
}

/// Lo stato della decodifica dei caratteri (cambio di Throb, attesa e spazio di ThrobX).
struct Decoder {
    v: ThrobVariant,
    shift: bool,
    st: IdleSpace,
    last: u8,
}

impl Decoder {
    fn new(v: ThrobVariant) -> Self {
        Decoder { v, shift: false, st: IdleSpace::new(v), last: 0 }
    }

    fn reset(&mut self) {
        *self = Decoder::new(self.v);
    }

    /// Il carattere della coppia di toni `t1 <= t2` (da 0), se c'e'.
    fn decode(&mut self, t1: usize, t2: usize) -> Option<u8> {
        let pair = ((t1 + 1) as u8, (t2 + 1) as u8);
        let sym = self.v.pairs().iter().position(|&p| p == pair);
        if !self.v.is_x() {
            if self.shift {
                self.shift = false;
                // dopo il cambio: (1,9) `?`, (2,8) `@`, (3,7) `-` (fldigi lo mostra `=`, ma lo trasmette per `-`), (5,5) a capo
                return match (t1, t2) {
                    (0, 8) => Some(b'?'),
                    (1, 7) => Some(b'@'),
                    (2, 6) => Some(b'-'),
                    (4, 4) => Some(b'\n'),
                    _ => None,
                };
            }
            if sym == Some(THROB_SHIFT) {
                self.shift = true;
                return None;
            }
            return sym.map(|i| THROB_CHARS[i]).filter(|&c| c != 0);
        }
        let i = sym?;
        if i == self.st.space || i == self.st.idle {
            // ThrobX: attesa o spazio, uno spazio solo dopo un carattere
            self.st.flip(self.v);
            if self.last != 0 && self.last != b' ' {
                self.last = b' ';
                return Some(b' ');
            }
            self.last = 0;
            return None;
        }
        let c = THROBX_CHARS[i];
        self.last = c;
        Some(c)
    }
}

/// Il passa-basso davanti alla decimazione (seno cardinale con finestra di Blackman, guadagno 1 in continua).
fn lowpass() -> Vec<f64> {
    let n = LP_TAPS;
    let fc = LP_CUT_HZ / RATE;
    let mid = (n - 1) as f64 / 2.0;
    let mut h: Vec<f64> = (0..n)
        .map(|i| {
            let x = i as f64 - mid;
            let sinc = if x.abs() < 1e-9 { 2.0 * fc } else { (2.0 * PI * fc * x).sin() / (PI * x) };
            let w = 0.42 - 0.5 * (2.0 * PI * i as f64 / (n - 1) as f64).cos() + 0.08 * (4.0 * PI * i as f64 / (n - 1) as f64).cos();
            sinc * w
        })
        .collect();
    let s: f64 = h.iter().sum();
    for v in &mut h {
        *v /= s;
    }
    h
}

/// Il rapporto (due toni contro gli altri, media su qualche simbolo) sopra il quale lo squelch si apre: nel solo rumore la media sta intorno a 2
/// (raramente oltre 3), un segnale anche debole la porta oltre 5 (prove qui sotto).
pub const SQUELCH_RATIO: f64 = 3.5;
/// Il rapporto minimo del singolo simbolo per stampare (la media resta alta qualche simbolo dopo la fine del segnale).
const SYMBOL_RATIO: f64 = 2.5;
/// Il tetto dei rapporti nella media.
const RATIO_CAP: f64 = 12.0;
/// Sotto questa frazione della forza dei simboli stampati il segnale e' finito (-10 dB).
const DROP: f64 = 0.3;

/// Il ricevitore a 8 kHz.
pub struct ThrobRx {
    v: ThrobVariant,
    reverse: bool,
    /// Il centro scelto e quello seguito dall'AFC (Hz).
    base_hz: f64,
    centre_hz: f64,
    afc: bool,
    afc_limit_hz: f64,
    phase: f64,
    lp: Vec<f64>,
    hist: Vec<(f64, f64)>,
    hist_pos: usize,
    since_decim: usize,
    /// Campioni decimati dell'ultimo simbolo (anello).
    ring: Vec<(f64, f64)>,
    ring_pos: usize,
    /// Le correlazioni con l'inviluppo di ogni tono (campioni decimati).
    templates: Vec<Vec<(f64, f64)>>,
    prev: Vec<(f64, f64)>,
    /// La forza media delle due correlazioni migliori per ogni punto del simbolo.
    timing: Vec<f64>,
    count: usize,
    /// Campioni decimati alla prossima decisione.
    countdown: usize,
    ratio: f64,
    /// La forza media dei due toni nei simboli stampati.
    sig_level: f64,
    level_db: f32,
    dec: Decoder,
    out: Vec<u8>,
}

impl ThrobRx {
    pub fn new(v: ThrobVariant, centre_hz: f64, reverse: bool, afc: bool) -> Self {
        let len = v.symbol_len() / DECIM;
        let pulse = v.pulse(v.symbol_len());
        let templates = v
            .tones()
            .iter()
            .map(|f| {
                (0..len)
                    .map(|j| {
                        let i = j * DECIM;
                        let x = -2.0 * PI * f * i as f64 / RATE;
                        (pulse[i] * x.cos(), pulse[i] * x.sin())
                    })
                    .collect()
            })
            .collect();
        let tones = v.tones();
        let afc_limit_hz = afc_limit_hz(v);
        ThrobRx {
            v,
            reverse,
            base_hz: centre_hz,
            centre_hz,
            afc,
            afc_limit_hz,
            phase: 0.0,
            lp: lowpass(),
            hist: vec![(0.0, 0.0); LP_TAPS],
            hist_pos: 0,
            since_decim: 0,
            ring: vec![(0.0, 0.0); len],
            ring_pos: 0,
            templates,
            prev: vec![(0.0, 0.0); tones.len()],
            timing: vec![0.0; len],
            count: 0,
            countdown: len,
            ratio: 0.0,
            sig_level: 0.0,
            level_db: -90.0,
            dec: Decoder::new(v),
            out: Vec::new(),
        }
    }

    /// Audio a 8 kHz in; i caratteri si prendono con [`ThrobRx::take`].
    pub fn process(&mut self, audio: &[f32]) {
        if !audio.is_empty() {
            let rms = (audio.iter().map(|s| s * s).sum::<f32>() / audio.len() as f32).sqrt();
            self.level_db = (20.0 * rms.max(1e-5).log10()).max(-90.0);
        }
        for &x in audio {
            // il centro puo' cambiare (AFC) a ogni campione decimato
            let step = 2.0 * PI * self.centre_hz / RATE;
            let (s, c) = self.phase.sin_cos();
            let x = f64::from(x);
            self.hist[self.hist_pos] = (x * c, -x * s);
            self.hist_pos = (self.hist_pos + 1) % LP_TAPS;
            self.phase = (self.phase + step) % (2.0 * PI);
            self.since_decim += 1;
            if self.since_decim >= DECIM {
                self.since_decim = 0;
                let mut acc = (0.0, 0.0);
                for (k, h) in self.lp.iter().enumerate() {
                    let z = self.hist[(self.hist_pos + k) % LP_TAPS];
                    acc.0 += h * z.0;
                    acc.1 += h * z.1;
                }
                self.decimated(acc);
            }
        }
    }

    fn decimated(&mut self, z: (f64, f64)) {
        let len = self.ring.len();
        self.ring[self.ring_pos] = z;
        self.ring_pos = (self.ring_pos + 1) % len;
        // correlazione dell'ultimo simbolo con ogni tono
        let corr: Vec<(f64, f64)> = self
            .templates
            .iter()
            .map(|t| {
                let mut a = (0.0, 0.0);
                for (j, w) in t.iter().enumerate() {
                    let s = self.ring[(self.ring_pos + j) % len];
                    a.0 += s.0 * w.0 - s.1 * w.1;
                    a.1 += s.0 * w.1 + s.1 * w.0;
                }
                a
            })
            .collect();
        let mags: Vec<f64> = corr.iter().map(|c| c.0.hypot(c.1)).collect();
        let (t1, t2) = top_two(&mags);
        let slot = self.count % len;
        self.timing[slot] = 0.7 * self.timing[slot] + 0.3 * (mags[t1] + mags[t2]);
        self.count += 1;
        self.countdown = self.countdown.saturating_sub(1);
        if self.countdown == 0 {
            self.symbol(&corr, &mags);
            // il punto del simbolo dove le due correlazioni migliori sono piu' forti, e quanto dista da qui (fra -mezzo e +mezzo simbolo)
            let best = self.timing.iter().enumerate().fold((0, f64::MIN), |b, (i, &v)| if v > b.1 { (i, v) } else { b }).0;
            let dist = ((best + len + len / 2 - slot) % len) as isize - (len / 2) as isize;
            let adj = if self.ratio >= SQUELCH_RATIO {
                // agganciato: meta' della distanza, al massimo un ottavo di simbolo per volta
                (dist / 2).clamp(-((len / 8) as isize), (len / 8) as isize)
            } else {
                // in cerca: subito al punto migliore
                dist
            };
            self.countdown = (len as isize + adj) as usize;
        }
        self.prev = corr;
    }

    fn symbol(&mut self, corr: &[(f64, f64)], mags: &[f64]) {
        let n = mags.len();
        let (max_i, second) = top_two(mags);
        let (mut a, mut b) = (max_i, second);
        // Throb: un tono solo se il piu' forte supera il doppio del secondo (come fldigi)
        if !self.v.is_x() && mags[max_i] > 2.0 * mags[second] {
            b = a;
        }
        if a > b {
            std::mem::swap(&mut a, &mut b);
        }
        let signal = (mags[a] + mags[b]) / 2.0;
        let noise = mags.iter().enumerate().filter(|(i, _)| *i != a && *i != b).map(|(_, m)| m).sum::<f64>() / (n as f64 - 2.0);
        let r = signal / (noise + 1e-12);
        // la media non conta i rapporti enormi di un segnale forte (scenderebbe troppo piano alla fine)
        self.ratio = 0.75 * self.ratio + 0.25 * r.min(RATIO_CAP);
        // il segnale e' finito: i due toni migliori sono crollati rispetto a quelli dei simboli stampati (la media chiude subito)
        if signal < DROP * self.sig_level {
            self.ratio = self.ratio.min(r);
            self.sig_level = 0.0;
        }
        // il simbolo da solo e la media di qualche simbolo
        if self.ratio < SQUELCH_RATIO || r < SYMBOL_RATIO {
            self.dec.reset();
            return;
        }
        self.sig_level = if self.sig_level > 0.0 { 0.8 * self.sig_level + 0.2 * signal } else { signal };
        let (d1, d2) = if self.reverse { (n - 1 - b, n - 1 - a) } else { (a, b) };
        if let Some(c) = self.dec.decode(d1, d2) {
            self.out.push(c);
        }
        if self.afc {
            // la rotazione di fase del tono piu' forte fra un campione decimato e il successivo
            let (p, q) = (self.prev[max_i], corr[max_i]);
            let re = p.0 * q.0 + p.1 * q.1;
            let im = p.0 * q.1 - p.1 * q.0;
            let f = im.atan2(re) / (2.0 * PI * DECIM as f64 / RATE) - self.v.tones()[max_i];
            let next = self.centre_hz + 0.3 * f;
            self.centre_hz = next.clamp(self.base_hz - self.afc_limit_hz, self.base_hz + self.afc_limit_hz);
        }
    }

    /// I caratteri ricevuti dall'ultima volta.
    pub fn take(&mut self) -> Vec<u8> {
        std::mem::take(&mut self.out)
    }

    /// Il centro seguito dall'AFC (Hz).
    pub fn frequency(&self) -> f64 {
        self.centre_hz
    }

    /// Il rapporto fra i due toni e gli altri, in dB.
    pub fn snr_db(&self) -> f32 {
        if self.ratio > 0.0 { (20.0 * self.ratio.log10()) as f32 } else { 0.0 }
    }

    /// Livello dell'audio (dB) e squelch aperto.
    pub fn signal(&self) -> (f32, bool) {
        (self.level_db, self.ratio >= SQUELCH_RATIO)
    }

    /// La soglia dello squelch in dB.
    pub fn threshold_db() -> f32 {
        (20.0 * SQUELCH_RATIO.log10()) as f32
    }
}

impl ThrobRx {
    /// Porta l'ascolto su `hz` (centro scelto e centro dell'AFC).
    pub fn steer(&mut self, hz: f64) {
        self.base_hz = hz;
        self.centre_hz = hz;
    }
}

/// Di quanto l'AFC di una variante puo' allontanarsi dal centro: mezzo passo fra i toni piu' 4 Hz (oltre, la decisione sbaglia tono).
pub fn afc_limit_hz(v: ThrobVariant) -> f64 {
    let t = v.tones();
    (t[1] - t[0]) / 2.0 + 4.0
}

/// Il ricevitore ad audio a 12 kHz (decimatore 3:2 davanti a [`ThrobRx`]), per il servizio dei modi testuali.
pub struct ThrobLink {
    rx: ThrobRx,
    decimator: crate::resample_8k::Decimator,
    buf: Vec<f32>,
}

impl ThrobLink {
    pub fn new(v: ThrobVariant, centre_hz: f32, reverse: bool, afc: bool) -> Self {
        ThrobLink { rx: ThrobRx::new(v, f64::from(centre_hz), reverse, afc), decimator: crate::resample_8k::Decimator::new(), buf: Vec::new() }
    }

    /// Audio a 12 kHz in, i caratteri nuovi in uscita.
    pub fn process(&mut self, audio: &[f32]) -> String {
        self.buf.clear();
        self.decimator.process(audio, &mut self.buf);
        self.rx.process(&self.buf);
        self.rx.take().into_iter().map(char::from).collect()
    }

    pub fn frequency(&self) -> f32 {
        self.rx.frequency() as f32
    }

    pub fn snr_db(&self) -> f32 {
        self.rx.snr_db()
    }

    pub fn signal(&self) -> (f32, bool) {
        self.rx.signal()
    }

    pub fn steer(&mut self, hz: f32) {
        self.rx.steer(f64::from(hz));
    }
}

/// Gli indici del valore piu' alto e del secondo.
fn top_two(m: &[f64]) -> (usize, usize) {
    let mut a = 0;
    for i in 1..m.len() {
        if m[i] > m[a] {
            a = i;
        }
    }
    let mut b = if a == 0 { 1 } else { 0 };
    for i in 0..m.len() {
        if i != a && m[i] > m[b] {
            b = i;
        }
    }
    (a, b)
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Rng(u64);
    impl Rng {
        fn uniform(&mut self) -> f64 {
            self.0 ^= self.0 << 13;
            self.0 ^= self.0 >> 7;
            self.0 ^= self.0 << 17;
            ((self.0 >> 11) as f64 + 0.5) / (1u64 << 53) as f64
        }
        fn gauss(&mut self) -> f32 {
            ((-2.0 * self.uniform().ln()).sqrt() * (2.0 * PI * self.uniform()).cos()) as f32
        }
    }

    fn noisy(x: &mut [f32], sigma: f32, seed: u64) {
        let mut r = Rng(seed | 1);
        for v in x {
            *v += sigma * r.gauss();
        }
    }

    fn rx(v: ThrobVariant, audio: &[f32], centre: f64, reverse: bool, afc: bool) -> String {
        let mut r = ThrobRx::new(v, centre, reverse, afc);
        for b in audio.chunks(700) {
            r.process(b);
        }
        String::from_utf8_lossy(&r.take()).into_owned()
    }

    fn signal(v: ThrobVariant, text: &str, freq: f64, reverse: bool) -> Vec<f32> {
        let mut a = vec![0.0_f32; 3000];
        a.extend(ThrobTx::new(v, freq, 0.5, reverse).transmit(text.as_bytes()));
        a.extend(vec![0.0_f32; 2 * v.symbol_len()]);
        a
    }

    /// Tutti i caratteri di Throb (anche quelli dopo il cambio) e di ThrobX, ad anello chiuso con un po' di rumore.
    #[test]
    fn throb4_e_throbx2_ad_anello_chiuso() {
        let t = "CQ DE IU3ABC 73? @-/ (0123456789) .,'";
        let mut a = signal(ThrobVariant::Throb4, t, 1000.0, false);
        noisy(&mut a, 0.02, 1);
        assert_eq!(rx(ThrobVariant::Throb4, &a, 1000.0, false, false).trim(), t);
        let tx = "CQ DE IU3ABC  #\"+-;:?!@= 73";
        let mut a = signal(ThrobVariant::ThrobX2, tx, 1200.0, false);
        noisy(&mut a, 0.02, 2);
        // due spazi di fila: ThrobX ne stampa uno (attesa e spazio si scambiano)
        assert_eq!(rx(ThrobVariant::ThrobX2, &a, 1200.0, false, false).trim(), "CQ DE IU3ABC #\"+-;:?!@= 73");
    }

    /// Le varianti lente e quella larga di ThrobX (testo breve: un simbolo dura fino a un secondo).
    #[test]
    fn le_altre_varianti() {
        for v in [ThrobVariant::Throb1, ThrobVariant::Throb2, ThrobVariant::ThrobX1, ThrobVariant::ThrobX4] {
            let mut a = signal(v, "IU3ABC 5NN", 1500.0, false);
            noisy(&mut a, 0.02, 3);
            assert_eq!(rx(v, &a, 1500.0, false, false).trim(), "IU3ABC 5NN", "{v:?}");
        }
    }

    /// In LSB i toni si rovesciano: si legge solo con l'inversione anche in ricezione.
    #[test]
    fn inversione_di_banda_laterale() {
        let a = signal(ThrobVariant::Throb4, "IU3ABC IO91WM", 1000.0, true);
        assert_eq!(rx(ThrobVariant::Throb4, &a, 1000.0, true, false).trim(), "IU3ABC IO91WM");
        assert_ne!(rx(ThrobVariant::Throb4, &a, 1000.0, false, false).trim(), "IU3ABC IO91WM");
    }

    /// Rumore forte (S/N circa -5 dB in 3 kHz): legge ancora quasi tutto.
    #[test]
    fn nel_rumore() {
        let t = "CQ CQ DE IU3ABC IU3ABC K";
        let mut a = signal(ThrobVariant::Throb4, t, 1000.0, false);
        // ampiezza dei due toni 0,25 ciascuno: potenza 0,0625; rumore sigma 0,4 (0,16 in 4 kHz)
        noisy(&mut a, 0.4, 4);
        let r = rx(ThrobVariant::Throb4, &a, 1000.0, false, false);
        let ok = r.trim().chars().zip(t.chars()).filter(|(x, y)| x == y).count();
        assert!(ok + 2 >= t.len(), "ricevuto {r:?}");
    }

    /// Nel solo rumore (debole e forte) per due minuti non esce nessun carattere.
    #[test]
    fn nel_solo_rumore_tace() {
        for (sigma, v) in [(0.01_f32, ThrobVariant::Throb4), (0.3, ThrobVariant::ThrobX2), (0.1, ThrobVariant::Throb2)] {
            let mut a = vec![0.0_f32; 8000 * 120];
            noisy(&mut a, sigma, 5);
            let r = rx(v, &a, 1000.0, false, true);
            assert!(r.is_empty(), "{v:?} nel rumore: {r:?}");
        }
    }

    /// Scostamento di frequenza: 2 Hz senza AFC, 6 Hz con l'AFC (Throb4, toni a 16 Hz: oltre mezzo passo la decisione sbaglia tono, come in fldigi); orologio sbagliato di 500 ppm.
    #[test]
    fn scostamento_di_frequenza_e_di_orologio() {
        let t = "CQ DE IU3ABC IU3ABC K";
        let a = signal(ThrobVariant::Throb4, t, 1002.0, false);
        assert_eq!(rx(ThrobVariant::Throb4, &a, 1000.0, false, false).trim(), t);
        let a = signal(ThrobVariant::Throb4, &format!("RYRYRY {t}"), 1006.0, false);
        let r = rx(ThrobVariant::Throb4, &a, 1000.0, false, true);
        assert!(r.trim().ends_with(t), "con AFC: {r:?}");
        let mut r = ThrobRx::new(ThrobVariant::Throb4, 1000.0, false, true);
        r.process(&a);
        assert!((r.frequency() - 1006.0).abs() < 1.5, "AFC a {:.1} Hz", r.frequency());
        // orologio: la stessa onda letta 500 ppm piu' veloce
        let a = signal(ThrobVariant::ThrobX2, t, 1000.0, false);
        let k = 1.0005;
        let b: Vec<f32> = (0..((a.len() as f64 / k) as usize - 1))
            .map(|i| {
                let x = i as f64 * k;
                let j = x as usize;
                let f = (x - j as f64) as f32;
                a[j] * (1.0 - f) + a[j + 1] * f
            })
            .collect();
        assert_eq!(rx(ThrobVariant::ThrobX2, &b, 1000.0 / k, false, false).trim(), t);
    }

    /// I nomi e gli identificatori delle varianti, i caratteri che il modo non ha (spazio) e il CR (niente).
    #[test]
    fn varianti_e_caratteri() {
        for v in ThrobVariant::ALL {
            assert_eq!(ThrobVariant::from_id(v.id()), Some(v));
            assert_eq!(ThrobVariant::from_id(v.name()), Some(v));
        }
        let mut st = IdleSpace::new(ThrobVariant::Throb2);
        assert_eq!(encode(ThrobVariant::Throb2, b'\r', &mut st), Vec::<usize>::new());
        assert_eq!(encode(ThrobVariant::Throb2, b'#', &mut st), vec![THROB_SPACE]);
        assert_eq!(encode(ThrobVariant::Throb2, b'e', &mut st), vec![43]);
        let mut st = IdleSpace::new(ThrobVariant::ThrobX2);
        assert_eq!(encode(ThrobVariant::ThrobX2, b' ', &mut st), vec![1]);
        assert_eq!(encode(ThrobVariant::ThrobX2, b' ', &mut st), vec![0]);
    }
}
