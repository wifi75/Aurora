//! PSK31 e PSK63: modulazione di fase binaria con inversioni dolci (inviluppo a coseno rialzato), codifica differenziale e varicode.
//!
//! Il bit 0 inverte la fase, il bit 1 la lascia com'è. Un carattere è il suo varicode seguito da due zeri.
//!
//! Il ricevitore segue quello di fldigi 4.2.13 (`src/psk/psk.cxx`, Dave Freese W1HKJ e contributori, derivato da gmfsk di Tomi Manninen
//! OH2BNS; GPL 3, copia in `terze-parti/fldigi`), con alcune scelte nostre (dall'analisi in
//! `docs/analisi-psk-cw.md`):
//! - mixer, primo filtro `fir1` a coseno rialzato (Hann) di 8 ms con decimazione a 16 campioni per simbolo, filtro adattato `fir2` di
//!   4 simboli con i coefficienti `pskcore_filter` (`src/psk/pskcoeff.cxx`);
//! - sincronismo di simbolo ad anticipo/ritardo sul `syncbuf` (fldigi `psk::rx_process`), con correzione frazionaria;
//! - metrica di qualità della fase 0-100 indipendente dal livello (fldigi `psk::rx_symbol`) per lo squelch, con la tenuta di 20 simboli;
//!   DCD sul preambolo di inversioni e sulla coda di portante (fldigi), che azzera il registro dei bit;
//! - AFC solo con il DCD acceso e la metrica sopra 0,05, guadagno 1/32 (PSK31) o 1/64 (PSK63) come fldigi, ma sull'errore di fase avvolto
//!   in ±90° (fldigi tiene la fase in [0, 2π) e sui bit 1 corregge solo da una parte);
//! - S/N e IMD con filtri di Goertzel (fldigi `psk::calcSN_IMD` e `psk::signalquality`); l'IMD vale solo durante almeno 21 inversioni
//!   di fila;
//! - noise blanker senza ritardo davanti al mixer (P4 di `docs/progetto-psk-cw-oltre.md`): le scariche diventano buchi di pochi ms invece
//!   di simboli sbagliati; senza scariche l'ingresso passa com'e'. Con il QRN della prova `psk_confronto` gli errori in PSK31 scendono dal
//!   27,7% al 12,6% (fldigi 30,5%).
//!
//! Per le decisioni «soft» future (varicode con più ipotesi, più ricevitori in parallelo) ogni simbolo lascia anche la sua affidabilità
//! ([`PskDemodulator::soft_bits`]), non solo il bit deciso.

use crate::psk_viterbi::PskViterbi;
use crate::qpsk::{ConvViterbi, QpskTx, symbol_metrics};
use crate::varicode;
use std::collections::VecDeque;
use std::f64::consts::PI;

/// Di quanto l'AFC puo' allontanarsi dalla frequenza scelta (Hz).
pub const AFC_LIMIT_HZ: f32 = 30.0;

/// Frequenza di campionamento dell'audio dei modi testuali.
pub const RATE: f32 = 12000.0;

/// Soglia di base dello squelch sulla metrica di qualità (0-100). Sul solo rumore la metrica supera 15 una volta su milioni di simboli;
/// con il segnale la soglia toglie poco (prova `psk_confronto` con `PSK_SQUELCH`: a -10 dB in 3 kHz l'1% di errori in PSK31).
pub const SQUELCH_DEFAULT: f32 = 15.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PskMode {
    Psk31,
    Psk63,
    /// QPSK31: quattro fasi con codice convoluzionale (vedi `qpsk`).
    Qpsk31,
}

impl PskMode {
    /// Campioni per simbolo a 12 kHz (31,25 e 62,5 baud).
    pub fn symbol_samples(self) -> usize {
        match self {
            PskMode::Psk31 | PskMode::Qpsk31 => 384,
            PskMode::Psk63 => 192,
        }
    }
    /// Il modo a quattro fasi con il codice convoluzionale.
    pub fn is_qpsk(self) -> bool {
        self == PskMode::Qpsk31
    }
    pub fn name(self) -> &'static str {
        match self {
            PskMode::Psk31 => "PSK31",
            PskMode::Psk63 => "PSK63",
            PskMode::Qpsk31 => "QPSK31",
        }
    }
    /// Lunghezza del preambolo, della coda e della finestra del DCD in simboli (fldigi `dcdbits`: 32 in PSK31, 64 in PSK63).
    pub fn dcd_bits(self) -> usize {
        match self {
            PskMode::Psk31 | PskMode::Qpsk31 => 32,
            PskMode::Psk63 => 64,
        }
    }
}

/// Un simbolo da trasmettere: inversione (bit 0), nessuna inversione (bit 1) o silenzio (per salire e scendere dolcemente).
#[derive(Clone, Copy, PartialEq)]
enum TxSym {
    Bit(bool),
    Silence,
}

/// I simboli da trasmettere: preambolo di inversioni lungo `dcd_bits` (fldigi), i caratteri (varicode + `00`), coda di portante pura lunga
/// `dcd_bits` (fldigi `tx_flush`: fa spegnere il DCD di chi riceve) e discesa al silenzio.
fn symbols_of(text: &[u8], mode: PskMode) -> Vec<TxSym> {
    let n = mode.dcd_bits();
    let mut syms = vec![TxSym::Bit(false); n];
    for &c in text {
        for ch in varicode::encode(c).chars() {
            syms.push(TxSym::Bit(ch == '1'));
        }
        syms.push(TxSym::Bit(false));
        syms.push(TxSym::Bit(false));
    }
    syms.extend(std::iter::repeat_n(TxSym::Bit(true), n));
    syms.push(TxSym::Silence);
    syms
}

/// Audio (a 12 kHz) del testo, con la portante a `freq_hz`; `amp` è l'ampiezza di picco (0..1). Il primo simbolo sale dal silenzio con
/// mezzo coseno (niente «click»), e l'ultimo torna al silenzio allo stesso modo.
pub fn modulate(text: &[u8], freq_hz: f32, mode: PskMode, amp: f32) -> Vec<f32> {
    if mode.is_qpsk() {
        return modulate_qpsk(text, freq_hz, mode, amp, false);
    }
    let n = mode.symbol_samples();
    let syms = symbols_of(text, mode);
    let mut out = Vec::with_capacity(syms.len() * n);
    let step = 2.0 * PI * f64::from(freq_hz) / f64::from(RATE);
    let mut phase = 0.0_f64;
    let mut prev = 0.0_f32;
    let mut sign = 1.0_f32;
    for sym in syms {
        let next = match sym {
            TxSym::Bit(bit) => {
                if !bit && prev != 0.0 {
                    sign = -sign;
                }
                sign
            }
            TxSym::Silence => 0.0,
        };
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

/// QPSK31: l'audio del testo; `reverse`: inversione di banda laterale (la radio in LSB).
pub fn modulate_qpsk(text: &[u8], freq_hz: f32, mode: PskMode, amp: f32, reverse: bool) -> Vec<f32> {
    let mut tx = QpskTx::new(freq_hz, mode.symbol_samples(), RATE, amp, reverse);
    let mut out = tx.preamble(mode.dcd_bits());
    for &c in text {
        out.extend(tx.char(c));
    }
    out.extend(tx.postamble(mode.dcd_bits()));
    out
}

/// Il primo filtro di fldigi (`raisedcosfilt` di `pskcoeff.cxx`): coseno rialzato lungo `len + 1` campioni.
fn raised_cosine(len: usize) -> Vec<f32> {
    (0..=len).map(|i| ((1.0 - (2.0 * PI * i as f64 / len as f64).cos()) / (2.0 * len as f64)) as f32).collect()
}

/// Il filtro adattato di fldigi (`pskcore_filter` di `src/psk/pskcoeff.cxx`, «PSKcore»): 65 coefficienti, 4 simboli a 16 campioni per
/// simbolo.
#[allow(clippy::excessive_precision)]
const PSKCORE: [f32; 65] = [
    4.3453566e-5,
    -0.00049122414,
    -0.00078771292,
    -0.0013507826,
    -0.0021287814,
    -0.003133466,
    -0.004366817,
    -0.0058112187,
    -0.0074249976,
    -0.0091398882,
    -0.010860157,
    -0.012464086,
    -0.013807772,
    -0.014731191,
    -0.015067057,
    -0.014650894,
    -0.013333425,
    -0.01099166,
    -0.0075431246,
    -0.0029527849,
    0.0027546292,
    0.0094932775,
    0.017113308,
    0.025403511,
    0.034099681,
    0.042895839,
    0.051458575,
    0.059444853,
    0.066521003,
    0.072381617,
    0.076767694,
    0.079481619,
    0.080420311,
    0.079481619,
    0.076767694,
    0.072381617,
    0.066521003,
    0.059444853,
    0.051458575,
    0.042895839,
    0.034099681,
    0.025403511,
    0.017113308,
    0.0094932775,
    0.0027546292,
    -0.0029527849,
    -0.0075431246,
    -0.01099166,
    -0.013333425,
    -0.014650894,
    -0.015067057,
    -0.014731191,
    -0.013807772,
    -0.012464086,
    -0.010860157,
    -0.0091398882,
    -0.0074249976,
    -0.0058112187,
    -0.004366817,
    -0.003133466,
    -0.0021287814,
    -0.0013507826,
    -0.00078771292,
    -0.00049122414,
    4.3453566e-5,
];

/// Lunghezza del primo filtro: 8 ms (96 campioni a 12 kHz), come i 64 campioni a 8 kHz di fldigi per PSK31 e PSK63.
const FIR1_LEN: usize = 96;
/// Campioni per simbolo dopo la decimazione.
const STEPS: usize = 16;
/// Media mobile di fldigi per la metrica (`SQLDECAY`).
const SQL_DECAY: f32 = 50.0;
/// Tenuta dello squelch dopo l'ultima volta sopra la soglia (20 simboli).
const SQUELCH_HOLD: u32 = 20;
/// Lunghezza dei blocchi di Goertzel per S/N e IMD (fldigi: 160 campioni decimati, 10 simboli).
const GOERTZEL_N: usize = 160;
/// Inversioni di fila perche' l'IMD valga.
const IMD_ZEROS: u32 = 21;
/// Quante affidabilita' di simbolo si tengono.
const SOFT_KEEP: usize = 256;

fn decay_avg(avg: f32, input: f32, weight: f32) -> f32 {
    avg + (input - avg) / weight
}

/// Aggancio alla portante. La differenziale del PSK insegue la frequenza solo entro circa un quarto della velocita' di simbolo (8 Hz in
/// PSK31, 16 in PSK63): piu' in la' la fase gira di oltre 90 gradi a simbolo, i bit escono a caso e l'AFC va dalla parte sbagliata o
/// si ferma al limite. Il segnale PSK al quadrato perde la modulazione (le fasi 0 e 180 gradi diventano la stessa) e lascia una riga
/// netta al doppio della portante: l'audio intorno alla frequenza scelta si porta in banda base (500 campioni al secondo, ±120 Hz), si
/// eleva al quadrato e il suo spettro medio (FFT di 512 punti, un secondo, a ogni mezzo secondo) dice dove sono le portanti, con
/// precisione sotto l'hertz e senza che una stazione vicina sposti la stima (le righe di due stazioni a 40 Hz stanno a 80 Hz).
const ACQ_RATE: usize = 500;
const ACQ_N: usize = 512;
const ACQ_HOP: usize = 256;
/// La riga sopra la mediana dello spettro al quadrato perche' valga come portante (12 dB).
const ACQ_LINE: f32 = 12.0;
/// Spettri da aspettare dopo un aggancio prima di un altro (un secondo e mezzo).
const ACQ_HOLD: u32 = 3;
/// Spettri di fila in cui una portante nuova deve vedersi prima di agganciarla (un secondo e mezzo: il rumore non fa righe ferme).
const ACQ_CONFIRM: u32 = 3;
/// Con la stazione agganciata l'ascolto non se ne allontana piu' di tanto (Hz): la riga al quadrato e' piu' precisa dell'AFC della fase
/// quando una stazione vicina o il rumore lo tirano.
const ACQ_TRIM_HZ: f32 = 1.5;
/// Coefficienti del filtro di banda base (finestra di Blackman, taglio 120 Hz a 12 kHz).
const ACQ_TAPS: usize = 193;

struct Acquire {
    fft: std::sync::Arc<dyn rustfft::Fft<f32>>,
    window: Vec<f32>,
    taps: Vec<f32>,
    ring: Vec<(f32, f32)>,
    pos: usize,
    count: usize,
    phase: f64,
    centre: f32,
    /// Banda base al quadrato, 500 campioni al secondo.
    sq: Vec<(f32, f32)>,
    avg: Vec<f32>,
    frames: u32,
    hold: u32,
    /// La portante della stazione scelta (Hz audio): resta fino a una frequenza scelta nuova.
    station: Option<f32>,
    /// La portante candidata e in quanti spettri di fila si e' vista.
    cand: Option<(f32, u32)>,
}

impl Acquire {
    fn new(centre: f32) -> Self {
        let fc = 120.0 / f64::from(RATE);
        let m = (ACQ_TAPS - 1) as f64;
        let mut taps: Vec<f32> = (0..ACQ_TAPS)
            .map(|i| {
                let x = i as f64 - m / 2.0;
                let sinc = if x == 0.0 { 2.0 * fc } else { (2.0 * PI * fc * x).sin() / (PI * x) };
                let w = 0.42 - 0.5 * (2.0 * PI * i as f64 / m).cos() + 0.08 * (4.0 * PI * i as f64 / m).cos();
                (sinc * w) as f32
            })
            .collect();
        let sum: f32 = taps.iter().sum();
        taps.iter_mut().for_each(|t| *t /= sum);
        Acquire {
            fft: rustfft::FftPlanner::new().plan_fft_forward(ACQ_N),
            window: (0..ACQ_N).map(|i| (0.5 - 0.5 * (2.0 * PI * i as f64 / ACQ_N as f64).cos()) as f32).collect(),
            taps,
            ring: vec![(0.0, 0.0); ACQ_TAPS],
            pos: 0,
            count: 0,
            phase: 0.0,
            centre,
            sq: Vec::with_capacity(2 * ACQ_N),
            avg: vec![0.0; ACQ_N],
            frames: 0,
            hold: 0,
            station: None,
            cand: None,
        }
    }

    /// Audio nuovo: in banda base intorno a `centre`, decimato e al quadrato.
    fn feed(&mut self, samples: &[f32]) {
        let step = 2.0 * PI * f64::from(self.centre) / f64::from(RATE);
        let decim = RATE as usize / ACQ_RATE;
        let n = self.ring.len();
        for &s in samples {
            let (sin, cos) = self.phase.sin_cos();
            self.ring[self.pos] = (s * cos as f32, -s * sin as f32);
            self.pos = (self.pos + 1) % n;
            self.phase = (self.phase + step) % (2.0 * PI);
            self.count += 1;
            if self.count < decim {
                continue;
            }
            self.count = 0;
            let (mut i, mut q) = (0.0_f32, 0.0_f32);
            for (k, &c) in self.taps.iter().enumerate() {
                let (a, b) = self.ring[(self.pos + k) % n];
                i += c * a;
                q += c * b;
            }
            self.sq.push((i * i - q * q, 2.0 * i * q));
        }
    }

    /// Uno spettro del segnale al quadrato: le portanti (Hz audio) entro `centre ± limit`. Le inversioni del PSK fanno anche due righe
    /// a ±mezzo baud dalla portante (l'inviluppo al quadrato batte alla velocita' di simbolo), 6 dB sotto quella vera: una riga con
    /// una piu' forte a mezzo baud non e' una portante.
    fn lines(&mut self, limit: f32, baud: f32) -> Vec<f32> {
        let mut a: Vec<rustfft::num_complex::Complex32> =
            self.sq[..ACQ_N].iter().zip(&self.window).map(|(&(i, q), w)| rustfft::num_complex::Complex32::new(i * w, q * w)).collect();
        self.fft.process(&mut a);
        self.frames = self.frames.saturating_add(1);
        let k = (1.0 / self.frames as f32).max(0.3);
        for (d, x) in self.avg.iter_mut().zip(&a) {
            *d += k * (x.norm_sqr() - *d);
        }
        let res = ACQ_RATE as f32 / ACQ_N as f32;
        let mut sorted = self.avg.clone();
        sorted.sort_by(f32::total_cmp);
        let floor = sorted[sorted.len() / 2].max(1e-30);
        let thr = floor * 10f32.powf(ACQ_LINE / 10.0);
        // la riga j dello spettro (con segno) sta al doppio dello scostamento della portante
        let reach = (2.0 * (limit + baud / 2.0) / res).ceil() as isize + 3;
        let n = ACQ_N as isize;
        let at = |j: isize| self.avg[j.rem_euclid(n) as usize];
        let mut out = Vec::new();
        for j in -reach..=reach {
            let v = at(j);
            if v < thr || v < at(j - 1) || v < at(j + 1) || v < at(j - 2) || v < at(j + 2) {
                continue;
            }
            // interpolazione parabolica sul massimo
            let (l, r) = (at(j - 1), at(j + 1));
            let den = l - 2.0 * v + r;
            let d = if den.abs() > 1e-30 { (0.5 * (l - r) / den).clamp(-0.5, 0.5) } else { 0.0 };
            out.push(((j as f32 + d) * res / 2.0, v));
        }
        let tol = 2.0 * res;
        out.iter()
            .filter(|&&(f, v)| {
                // non a mezzo baud da una riga piu' forte (inversioni) ne' a meno di un quarto di baud (code della finestra)
                f.abs() <= limit + baud / 2.0 && !out.iter().any(|&(g, w)| w > v && (((g - f).abs() - baud / 2.0).abs() < tol || (g - f).abs() < baud / 4.0))
            })
            .map(|&(f, _)| self.centre + f)
            .collect()
    }
}

/// Ricevitore: si alimenta con l'audio a 12 kHz e restituisce i caratteri man mano che si decodificano.
pub struct PskDemodulator {
    mode: PskMode,
    /// La frequenza scelta dall'utente e quella che l'AFC ha raggiunto.
    base_hz: f32,
    freq_hz: f32,
    /// AFC: segue la deriva della portante entro `AFC_LIMIT_HZ` dalla frequenza scelta.
    afc: bool,
    /// Squelch: soglia della metrica di qualità (0-100); `None` = spento.
    squelch: Option<f32>,
    baud: f32,
    decim: usize,
    phase: f64,
    step: f64,
    fir1: Vec<f32>,
    ring1: Vec<(f32, f32)>,
    pos1: usize,
    count: usize,
    fir2: Vec<f32>,
    ring2: Vec<(f32, f32)>,
    pos2: usize,
    syncbuf: [f32; STEPS],
    bitclk: f32,
    prev: (f32, f32),
    /// Media della potenza dei simboli, per l'affidabilità.
    power: f32,
    quality: (f32, f32),
    metric: f32,
    afcmetric: f32,
    dcdshreg: u32,
    dcd: bool,
    dcd_off: u32,
    hold: u32,
    open: bool,
    shreg: u32,
    level_db: f32,
    soft: VecDeque<f32>,
    // S/N e IMD
    g_n: usize,
    g_acc: [(f32, f32); 7],
    e_hist: VecDeque<[f32; 4]>,
    e_len: usize,
    snr_db: Option<f32>,
    imd_db: Option<f32>,
    zeros: u32,
    idle: [f32; 4],
    idle_blocks: u32,
    /// La varicode letta con le probabilità (vedi `psk_viterbi`); `None` = bit per bit come fldigi.
    viterbi: Option<Box<PskViterbi>>,
    /// Momenti del prodotto con il simbolo prima (secondo e quarto), per la scala dei rapporti di verosimiglianza.
    m2: f32,
    m4: f32,
    moments_n: u32,
    /// Potenza media degli ultimi simboli.
    p_local: f32,
    confidences: Vec<u8>,
    /// P4: noise blanker davanti al mixer (le scariche diventano buchi invece di simboli sbagliati).
    blanker: Option<crate::blanker::NoiseBlanker>,
    /// Aggancio alla portante entro tanti Hz dalla frequenza scelta (con l'AFC acceso); `None` = spento.
    acquire_limit: Option<f32>,
    acquire: Option<Box<Acquire>>,
    /// L'ultimo aggancio non ancora letto da [`PskDemodulator::take_acquired`].
    acquired: Option<f32>,
    /// QPSK31: inversione di banda laterale e decodificatore convoluzionale.
    reverse: bool,
    conv: Option<Box<ConvViterbi>>,
}

impl PskDemodulator {
    pub fn new(mode: PskMode, freq_hz: f32) -> Self {
        // filtri normalizzati a guadagno 1 in continua: il livello del segnale resta quello dell'ingresso
        let norm = |v: Vec<f32>| {
            let sum: f32 = v.iter().sum();
            v.into_iter().map(|c| c / sum).collect::<Vec<f32>>()
        };
        Self {
            mode,
            base_hz: freq_hz,
            freq_hz,
            afc: false,
            squelch: None,
            baud: RATE / mode.symbol_samples() as f32,
            decim: mode.symbol_samples() / STEPS,
            phase: 0.0,
            step: 2.0 * PI * f64::from(freq_hz) / f64::from(RATE),
            fir1: norm(raised_cosine(FIR1_LEN)),
            ring1: vec![(0.0, 0.0); FIR1_LEN + 1],
            pos1: 0,
            count: 0,
            fir2: norm(PSKCORE.to_vec()),
            ring2: vec![(0.0, 0.0); PSKCORE.len()],
            pos2: 0,
            syncbuf: [0.0; STEPS],
            bitclk: 0.0,
            prev: (0.0, 0.0),
            power: 0.0,
            quality: (0.0, 0.0),
            metric: 0.0,
            afcmetric: 0.0,
            dcdshreg: 0,
            dcd: false,
            dcd_off: 0,
            hold: 0,
            open: false,
            shreg: 0,
            level_db: -90.0,
            soft: VecDeque::with_capacity(SOFT_KEEP),
            g_n: 0,
            g_acc: [(0.0, 0.0); 7],
            e_hist: VecDeque::new(),
            e_len: mode.dcd_bits() / 2,
            snr_db: None,
            imd_db: None,
            zeros: 0,
            idle: [0.0; 4],
            idle_blocks: 0,
            viterbi: Some(Box::new(PskViterbi::new())),
            m2: 0.0,
            m4: 0.0,
            moments_n: 0,
            p_local: 0.0,
            confidences: Vec::new(),
            blanker: Some(crate::blanker::NoiseBlanker::without_delay()),
            acquire_limit: None,
            acquire: None,
            acquired: None,
            reverse: false,
            conv: mode.is_qpsk().then(|| Box::new(ConvViterbi::new())),
        }
    }

    /// QPSK31: inversione di banda laterale (la radio in LSB); negli altri modi non cambia niente.
    pub fn set_reverse(&mut self, on: bool) {
        self.reverse = on;
    }

    /// Aggancio alla portante del segnale PSK piu' forte entro `limit` Hz dalla frequenza scelta, quando l'ascolto ne e' lontano piu'
    /// di quanto l'AFC sappia recuperare (vale solo con l'AFC acceso); `None` lo spegne.
    pub fn set_acquire(&mut self, limit: Option<f32>) {
        self.acquire_limit = limit.filter(|l| l.is_finite() && *l > 0.0);
        if self.acquire_limit.is_none() {
            self.acquire = None;
        }
    }

    /// La frequenza dell'ultimo aggancio, una volta sola (chi tiene i limiti dell'AFC, `notch::AfcGuard::retune`, deve saperlo).
    pub fn take_acquired(&mut self) -> Option<f32> {
        self.acquired.take()
    }

    /// Lo spettro dell'aggancio sull'audio nuovo. La stazione scelta e' la portante piu' vicina alla frequenza scelta (entro il limite);
    /// poi si segue solo lei (pochi Hz per spettro: la deriva), mai la vicina. Se l'ascolto se ne allontana oltre un quinto della velocita'
    /// di simbolo (bandierina non centrata, AFC trascinato dalla vicina o dal rumore) torna sulla stazione; nelle pause resta dov'e'.
    fn run_acquire(&mut self, samples: &[f32]) {
        let Some(limit) = self.acquire_limit.filter(|_| self.afc) else {
            return;
        };
        let base = self.base_hz;
        let acq = self.acquire.get_or_insert_with(|| Box::new(Acquire::new(base)));
        acq.feed(samples);
        let follow = (0.15 * self.baud).max(3.0);
        while acq.sq.len() >= ACQ_N {
            let lines = acq.lines(limit, self.baud);
            acq.sq.drain(..ACQ_HOP);
            acq.hold = acq.hold.saturating_sub(1);
            let near = |f: f32| lines.iter().copied().min_by(|a, b| (a - f).abs().total_cmp(&(b - f).abs()));
            // la stazione agganciata si segue anche oltre il limite (deriva), di pochi Hz per spettro
            // in silenzio (l'altro sta trasmettendo, anche per minuti) la stazione resta quella: l'ascolto non va sulla vicina
            if let Some(f) = acq.station.and_then(|st| near(st).filter(|f| (f - st).abs() < follow)) {
                acq.station = Some(f);
            }
            // la stazione scelta e' la portante piu' vicina alla frequenza scelta entro il limite, se resta ferma per qualche spettro;
            // finche' quella agganciata si sente non si cambia (niente salti alla vicina)
            acq.cand = match (near(base).filter(|f| (f - base).abs() <= limit + 1.0), acq.cand) {
                (Some(f), Some((c, n))) if (f - c).abs() < follow => Some((f, n + 1)),
                (Some(f), _) => Some((f, 1)),
                (None, _) => None,
            };
            if let Some((f, _)) = acq.cand.filter(|&(_, n)| n >= ACQ_CONFIRM && acq.station.is_none()) {
                acq.station = Some(f);
            }
            let Some(st) = acq.station else {
                continue;
            };
            // lontano: salto sulla stazione; vicino ma fuori di piu' di 1,5 Hz (AFC tirato dalla vicina o dal rumore): correzione
            let d = (st - self.freq_hz).abs();
            if (acq.hold == 0 && d > 0.2 * self.baud) || (d > ACQ_TRIM_HZ && d <= 0.2 * self.baud) {
                if d > 0.2 * self.baud {
                    acq.hold = ACQ_HOLD;
                }
                self.freq_hz = st;
                self.step = 2.0 * PI * f64::from(st) / f64::from(RATE);
                self.acquired = Some(st);
            }
        }
    }

    /// La varicode letta con le probabilità (di base) o bit per bit come fldigi.
    pub fn set_soft_decoding(&mut self, on: bool) {
        if on != self.viterbi.is_some() {
            self.viterbi = on.then(|| Box::new(PskViterbi::new()));
        }
    }

    /// Probabilità minima (0-255) di un carattere per stamparlo con la lettura probabilistica (0 = tutto).
    pub fn set_min_confidence(&mut self, c: u8) {
        if let Some(v) = self.viterbi.as_mut() {
            v.set_min_confidence(c);
        }
    }

    /// Le probabilità (0-255) dei caratteri restituiti dall'ultima chiamata di [`PskDemodulator::process`] (vuoto bit per bit).
    pub fn confidences(&self) -> &[u8] {
        &self.confidences
    }

    pub fn mode(&self) -> PskMode {
        self.mode
    }

    /// La frequenza scelta dall'utente: l'AFC riparte da qui.
    pub fn set_frequency(&mut self, freq_hz: f32) {
        self.base_hz = freq_hz;
        self.tune(freq_hz);
        // l'aggancio riparte dalla frequenza nuova
        self.acquire = None;
        self.acquired = None;
    }

    fn tune(&mut self, freq_hz: f32) {
        self.freq_hz = freq_hz;
        self.step = 2.0 * PI * f64::from(freq_hz) / f64::from(RATE);
    }

    /// La frequenza su cui si sta ascoltando adesso (con l'AFC puo' essere vicina a quella scelta, non uguale).
    pub fn frequency(&self) -> f32 {
        self.freq_hz
    }

    /// Porta l'ascolto su `freq_hz` senza cambiare la frequenza scelta (i limiti dell'AFC, `notch::AfcGuard`).
    pub fn steer(&mut self, freq_hz: f32) {
        self.tune(freq_hz);
    }

    pub fn set_afc(&mut self, on: bool) {
        self.afc = on;
        if !on {
            self.tune(self.base_hz);
        }
    }

    /// Soglia dello squelch sulla metrica di qualità della fase (0-100, come fldigi); `None` lo spegne.
    pub fn set_squelch(&mut self, threshold: Option<f32>) {
        self.squelch = threshold.map(|t| t.clamp(0.0, 100.0));
    }

    /// Livello del segnale in dB (sull'uscita del filtro adattato) e se lo squelch lo lascia passare.
    pub fn signal(&self) -> (f32, bool) {
        (self.level_db, self.squelch.is_none() || self.open)
    }

    /// La metrica di qualità della fase, 0 (solo rumore) - 100 (fase perfetta): è quella su cui lavora lo squelch.
    pub fn metric(&self) -> f32 {
        self.metric
    }

    /// Il DCD: c'e' un segnale PSK (preambolo visto, o squelch spento, o metrica sopra la soglia) e si stampa.
    pub fn dcd(&self) -> bool {
        self.dcd
    }

    /// Rapporto segnale/rumore in dB come lo misura fldigi (potenza dei toni a ±baud/2 o della portante contro il rumore a ±2·baud);
    /// `None` finche' non c'e' un segnale chiaro.
    pub fn snr_db(&self) -> Option<f32> {
        self.snr_db
    }

    /// IMD in dB (prodotti di terzo ordine a ±3·baud/2 contro i toni a ±baud/2), misurata solo durante almeno 21 inversioni di fila
    /// (preambolo o pausa); resta l'ultima misura valida finche' il DCD non si spegne.
    pub fn imd_db(&self) -> Option<f32> {
        self.imd_db
    }

    /// Le affidabilità degli ultimi simboli, dal piu' vecchio: positive per il bit 1 (nessuna inversione), negative per lo 0; il valore
    /// assoluto cresce con la qualità del simbolo (prodotto scalare col simbolo prima, diviso per la potenza media).
    pub fn soft_bits(&self) -> impl Iterator<Item = f32> + '_ {
        self.soft.iter().copied()
    }

    /// Elabora altro audio; restituisce i caratteri nuovi (Latin-1).
    pub fn process(&mut self, samples: &[f32]) -> Vec<u8> {
        let mut out = Vec::new();
        self.confidences.clear();
        self.run_acquire(samples);
        let len1 = self.ring1.len();
        for &s in samples {
            let s = match self.blanker.as_mut() {
                Some(nb) => nb.process(s),
                None => s,
            };
            let (sin, cos) = self.phase.sin_cos();
            self.ring1[self.pos1] = (s * cos as f32, -s * sin as f32);
            self.pos1 = (self.pos1 + 1) % len1;
            self.phase += self.step;
            if self.phase > 2.0 * PI {
                self.phase -= 2.0 * PI;
            }
            self.count += 1;
            if self.count < self.decim {
                continue;
            }
            self.count = 0;
            // fir1 si calcola solo quando serve (decimazione)
            let (mut zi, mut zq) = (0.0_f32, 0.0_f32);
            for (k, &c) in self.fir1.iter().enumerate() {
                let (i, q) = self.ring1[(self.pos1 + k) % len1];
                zi += c * i;
                zq += c * q;
            }
            self.decimated((zi, zq), &mut out);
        }
        if let Some(v) = self.viterbi.as_mut() {
            self.confidences = v.take_confidences();
        }
        out
    }

    /// Un campione a 16 per simbolo: S/N e IMD, filtro adattato, sincronismo e, a ogni simbolo, la decisione.
    fn decimated(&mut self, z1: (f32, f32), out: &mut Vec<u8>) {
        self.goertzel(z1);
        let len2 = self.ring2.len();
        self.ring2[self.pos2] = z1;
        self.pos2 = (self.pos2 + 1) % len2;
        let (mut zi, mut zq) = (0.0_f32, 0.0_f32);
        for (k, &c) in self.fir2.iter().enumerate() {
            let (i, q) = self.ring2[(self.pos2 + k) % len2];
            zi += c * i;
            zq += c * q;
        }
        let mag = (zi * zi + zq * zq).sqrt();
        // fldigi: l'ampiezza disegna un simbolo nel syncbuf; la meta' sinistra contro quella destra spinge l'orologio
        let idx = (self.bitclk as usize).min(STEPS - 1);
        self.syncbuf[idx] = 0.8 * self.syncbuf[idx] + 0.2 * mag;
        let half = STEPS / 2;
        let (mut sum, mut ampsum) = (0.0_f32, 0.0_f32);
        for i in 0..half {
            sum += self.syncbuf[i] - self.syncbuf[i + half];
            ampsum += self.syncbuf[i] + self.syncbuf[i + half];
        }
        let sum = if ampsum == 0.0 { 0.0 } else { sum / ampsum };
        self.bitclk -= sum / 5.0;
        self.bitclk += 1.0;
        if self.bitclk < 0.0 {
            self.bitclk += STEPS as f32;
        }
        if self.bitclk >= STEPS as f32 {
            self.bitclk -= STEPS as f32;
            self.symbol((zi, zq), mag, out);
        }
    }

    fn symbol(&mut self, z: (f32, f32), mag: f32, out: &mut Vec<u8>) {
        if self.mode.is_qpsk() {
            return self.symbol_qpsk(z, mag, out);
        }
        let prev = self.prev;
        self.prev = z;
        let dot = z.0 * prev.0 + z.1 * prev.1;
        let cross = z.1 * prev.0 - z.0 * prev.1;
        let dphi = cross.atan2(dot);
        // il silenzio (dot = 0) vale «nessuna inversione»: non deve sembrare un preambolo
        let one = dot >= 0.0;
        self.level_db = (20.0 * mag.max(1e-6).log10()).max(-90.0);
        self.power = decay_avg(self.power, mag * mag, 32.0);
        // affidabilità: prodotto scalare col simbolo prima, normalizzato alla potenza media
        let soft = (dot / self.power.max(1e-12)).clamp(-20.0, 20.0);
        if self.soft.len() == SOFT_KEEP {
            self.soft.pop_front();
        }
        self.soft.push_back(soft);
        // rapporto di verosimiglianza del bit: il prodotto con il simbolo prima vale ±mu piu' rumore; mu e la varianza dai momenti
        // secondo e quarto (mu^4 = (3 m2^2 - m4) / 2), che non dipendono dal volume e non si gonfiano col rumore
        // il prodotto si divide per la potenza degli ultimi quattro simboli (provate 1, 2, 4, 8, 24 e 64 nella prova `psk_confronto`, `studio`: 4 e' la migliore): con il fading l'ampiezza cambia e i momenti la scambierebbero per rumore
        let p = (z.0 * z.0 + z.1 * z.1 + prev.0 * prev.0 + prev.1 * prev.1) / 2.0;
        self.p_local += (p - self.p_local) / if self.p_local > 0.0 { 4.0 } else { 1.0 };
        let dot = dot / self.p_local.max(1e-30);
        let d2 = dot * dot;
        self.moments_n = (self.moments_n + 1).min(64);
        let a = 1.0 / self.moments_n as f32;
        self.m2 += a * (d2 - self.m2);
        self.m4 += a * (d2 * d2 - self.m4);
        let mu2 = ((3.0 * self.m2 * self.m2 - self.m4) / 2.0).max(0.0).sqrt();
        let var = (self.m2 - mu2).max(0.05 * self.m2).max(1e-30);
        let llr = (2.0 * mu2.sqrt() * dot / var).clamp(-30.0, 30.0);

        // metrica di fldigi: media di e^(j 2 dphi), cosi' 0 e 180 gradi coincidono (fldigi confronta per sbaglio anche la parte immaginaria
        // con quella reale per scegliere fra salita e discesa: qui le due medie sono uguali e il problema non c'e')
        let (s2, c2) = (2.0 * dphi).sin_cos();
        self.quality = (decay_avg(self.quality.0, c2, SQL_DECAY), decay_avg(self.quality.1, s2, SQL_DECAY));
        let q = self.quality.0 * self.quality.0 + self.quality.1 * self.quality.1;
        self.metric = (100.0 * q).min(100.0);
        self.afcmetric = decay_avg(self.afcmetric, q, 50.0);
        // tenuta: lo squelch resta aperto 20 simboli dopo l'ultima volta sopra la soglia
        if self.squelch.is_some_and(|t| self.metric > t) {
            self.hold = SQUELCH_HOLD;
        } else {
            self.hold = self.hold.saturating_sub(1);
        }
        self.open = self.hold > 0;

        // DCD di fldigi: 16 inversioni di fila (preambolo) lo accendono; 16 simboli senza inversioni (la coda), visti 6 volte, lo spengono
        self.dcdshreg = (self.dcdshreg << 2) | if one { 0 } else { 2 };
        match self.dcdshreg {
            0xAAAA_AAAA => {
                self.dcd = true;
                self.dcd_off = 0;
                self.quality = (1.0, 0.0);
                self.hold = SQUELCH_HOLD;
                self.open = true;
                // un segnale che comincia: i bit rimasti nel registro non sono suoi
                self.shreg = 0;
                if let Some(v) = self.viterbi.as_mut() {
                    v.reset();
                }
                // la scala dei rapporti di verosimiglianza riparte dal segnale nuovo (i momenti del rumore di prima non gli servono)
                self.moments_n = 0;
            }
            0 => {
                self.dcd_off += 1;
                if self.dcd_off > 5 {
                    self.dcd_off = 0;
                    self.dcd = false;
                    self.quality = (0.0, 0.0);
                    self.hold = 0;
                    self.open = false;
                    self.snr_db = None;
                    self.imd_db = None;
                    self.shreg = 0;
                    if let Some(v) = self.viterbi.as_mut() {
                        v.flush(out);
                    }
                }
            }
            _ => {
                self.dcd = self.squelch.is_none() || self.open;
                self.dcd_off = self.dcd_off.saturating_sub(1);
            }
        }
        self.zeros = if one { 0 } else { self.zeros.saturating_add(1) };

        if self.dcd {
            match self.viterbi.as_mut() {
                Some(v) => v.push(llr, out),
                None => self.push_bit(one, out),
            }
            if self.afc && self.afcmetric >= 0.05 {
                // errore di fase rimasto dopo aver tolto il bit (0 = inversione di 180 gradi), avvolto in ±90 gradi
                let err = if one { dphi } else { (dphi + 2.0 * PI as f32).rem_euclid(2.0 * PI as f32) - PI as f32 };
                let err_hz = err * self.baud / (2.0 * PI as f32);
                if err_hz.abs() < self.baud {
                    let f = (self.freq_hz + err_hz / self.mode.dcd_bits() as f32).clamp(self.base_hz - AFC_LIMIT_HZ, self.base_hz + AFC_LIMIT_HZ);
                    self.tune(f);
                }
            }
        }
    }

    /// QPSK31: niente DCD (preambolo e coda sono uguali, come in fldigi): decide lo squelch sulla metrica di e^(j 4 dphi); il
    /// decodificatore di Viterbi riceve le metriche dei quattro simboli e i suoi bit vanno nel registro della varicode.
    fn symbol_qpsk(&mut self, z: (f32, f32), mag: f32, out: &mut Vec<u8>) {
        let prev = self.prev;
        self.prev = z;
        let dot = z.0 * prev.0 + z.1 * prev.1;
        let cross = z.1 * prev.0 - z.0 * prev.1;
        // il nostro mixer moltiplica per e^(-j w t): la fase vista e' la coniugata di quella trasmessa (I·cos + Q·sin)
        let dphi = cross.atan2(dot);
        self.level_db = (20.0 * mag.max(1e-6).log10()).max(-90.0);
        self.power = decay_avg(self.power, mag * mag, 32.0);
        let p = (z.0 * z.0 + z.1 * z.1 + prev.0 * prev.0 + prev.1 * prev.1) / 2.0;
        self.p_local += (p - self.p_local) / if self.p_local > 0.0 { 4.0 } else { 1.0 };
        let w = (dot / self.p_local.max(1e-30), -cross / self.p_local.max(1e-30));
        let (s4, c4) = (4.0 * dphi).sin_cos();
        self.quality = (decay_avg(self.quality.0, c4, SQL_DECAY), decay_avg(self.quality.1, s4, SQL_DECAY));
        let q = self.quality.0 * self.quality.0 + self.quality.1 * self.quality.1;
        self.metric = (100.0 * q).min(100.0);
        self.afcmetric = decay_avg(self.afcmetric, q, 50.0);
        if self.squelch.is_some_and(|t| self.metric > t) {
            self.hold = SQUELCH_HOLD;
        } else {
            self.hold = self.hold.saturating_sub(1);
        }
        self.open = self.hold > 0;
        let was = self.dcd;
        self.dcd = self.squelch.is_none() || self.open;
        if self.dcd && !was {
            self.shreg = 0;
            if let Some(c) = self.conv.as_mut() {
                c.reset();
            }
        }
        if !self.dcd {
            if was {
                let bits = self.conv.as_mut().map(|c| c.flush()).unwrap_or_default();
                for b in bits {
                    self.push_bit(b, out);
                }
                self.shreg = 0;
            }
            return;
        }
        let reverse = self.reverse;
        if let Some(b) = self.conv.as_mut().and_then(|c| c.push(symbol_metrics(w, reverse))) {
            self.push_bit(b, out);
        }
        if self.afc && self.afcmetric >= 0.05 {
            // errore di fase rimasto dopo aver tolto il multiplo di 90 gradi piu' vicino (±45 gradi)
            let quarter = PI as f32 / 2.0;
            let err = dphi - (dphi / quarter).round() * quarter;
            let err_hz = err * self.baud / (2.0 * PI as f32);
            let f = (self.freq_hz + err_hz / self.mode.dcd_bits() as f32).clamp(self.base_hz - AFC_LIMIT_HZ, self.base_hz + AFC_LIMIT_HZ);
            self.tune(f);
        }
    }

    /// Un bit nel registro (come `psk::rx_bit` di fldigi, senza limiti di lunghezza): due zeri chiudono il carattere.
    fn push_bit(&mut self, one: bool, out: &mut Vec<u8>) {
        self.shreg = (self.shreg << 1) | u32::from(one);
        if self.shreg & 3 == 0 {
            if let Some(c) = varicode::decode_value(self.shreg >> 2) {
                out.push(c);
            }
            self.shreg = 0;
        }
    }

    /// DFT su blocchi di 10 simboli a 0, ±baud/2 (fondamentale), ±3·baud/2 (IMD) e ±2·baud (rumore) dei campioni decimati: 0, 1/32, 3/32
    /// e 4/32 di giro per campione (fldigi: 0; 15,625; 46,875 e 62,5 Hz a 500 Hz in PSK31). Potenze mediate su `dcdbits/2` blocchi.
    fn goertzel(&mut self, z: (f32, f32)) {
        const BINS: [f64; 7] = [0.0, 1.0 / 32.0, -1.0 / 32.0, 3.0 / 32.0, -3.0 / 32.0, 4.0 / 32.0, -4.0 / 32.0];
        let n = self.g_n as f64;
        for (acc, f) in self.g_acc.iter_mut().zip(BINS) {
            let (s, c) = (-2.0 * PI * f * n).sin_cos();
            let (c, s) = (c as f32, s as f32);
            acc.0 += z.0 * c - z.1 * s;
            acc.1 += z.0 * s + z.1 * c;
        }
        self.g_n += 1;
        if self.g_n < GOERTZEL_N {
            return;
        }
        let p = |a: (f32, f32)| a.0 * a.0 + a.1 * a.1;
        let g = self.g_acc;
        // portante, fondamentale, rumore, terza armonica (e0..e3 di fldigi)
        let e = [p(g[0]), p(g[1]) + p(g[2]), p(g[5]) + p(g[6]), p(g[3]) + p(g[4])];
        self.g_n = 0;
        self.g_acc = [(0.0, 0.0); 7];
        if self.e_hist.len() == self.e_len {
            self.e_hist.pop_front();
        }
        self.e_hist.push_back(e);
        let k = self.e_hist.len() as f32;
        let mut avg = [0.0_f32; 4];
        for h in &self.e_hist {
            for (a, v) in avg.iter_mut().zip(h) {
                *a += v / k;
            }
        }
        let sig = avg[0].max(avg[1]);
        self.snr_db = (avg[2] > 0.0 && sig > 2.0 * avg[2] && self.dcd).then(|| 10.0 * (sig / avg[2]).log10());
        // IMD: solo blocchi interi dentro una serie di inversioni, e valida dopo almeno 21 inversioni di fila
        if self.zeros as usize > GOERTZEL_N / STEPS {
            if self.idle_blocks == 0 {
                self.idle = [0.0; 4];
            }
            self.idle_blocks += 1;
            for (a, v) in self.idle.iter_mut().zip(e) {
                *a += v;
            }
            if self.zeros >= IMD_ZEROS && self.idle[1] > 4.0 * self.idle[2] && self.idle[1] > 0.0 {
                // prodotti sotto il rumore: si mostra il limite che da' il rumore (come fldigi, che non scende sotto 1/S/N)
                let products = self.idle[3].max(self.idle[2]);
                self.imd_db = Some(10.0 * (products / self.idle[1]).log10());
            }
        } else {
            self.idle_blocks = 0;
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
    fn l_afc_insegue_la_portante() {
        // l'AFC segue la deriva entro circa 7 Hz (un quarto della velocita'): oltre ci pensa lo Snap, che porta vicino alla stazione
        for offset in [5.0_f32, -5.0] {
            let audio = modulate(b"CQ CQ CQ DE IU3ABC IU3ABC PSE K", 1000.0 + offset, PskMode::Psk31, 0.5);
            let mut rx = PskDemodulator::new(PskMode::Psk31, 1000.0);
            rx.set_afc(true);
            let mut all = vec![0.0; 4000];
            all.extend(audio);
            all.extend(vec![0.0; 4000]);
            let r = String::from_utf8_lossy(&rx.process(&all)).into_owned();
            assert!((rx.frequency() - (1000.0 + offset)).abs() < 1.5, "offset {offset}: frequenza raggiunta {}", rx.frequency());
            assert!(r.contains("IU3ABC"), "offset {offset}: ricevuto {r:?}");
        }
        // spegnendo l'AFC si torna alla frequenza scelta
        let mut rx = PskDemodulator::new(PskMode::Psk31, 1000.0);
        rx.set_afc(true);
        rx.set_afc(false);
        assert_eq!(rx.frequency(), 1000.0);
    }

    #[test]
    fn lo_squelch_ferma_il_rumore_e_lascia_passare_il_segnale() {
        let mut seed = 99_u32;
        let noise: Vec<f32> = (0..48_000)
            .map(|_| {
                seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
                (seed as f32 / u32::MAX as f32 - 0.5) * 0.2
            })
            .collect();
        let mut rx = PskDemodulator::new(PskMode::Psk31, 1000.0);
        rx.set_squelch(Some(SQUELCH_DEFAULT));
        let from_noise = rx.process(&noise);
        assert!(from_noise.is_empty(), "il rumore non deve stampare: {from_noise:?}");
        assert!(!rx.signal().1, "con solo rumore lo squelch e' chiuso");
        let audio = modulate(b"CQ DE IU3ABC", 1000.0, PskMode::Psk31, 0.5);
        // fino all'inizio della coda di portante lo squelch e' aperto; la coda spegne il DCD e lo richiude
        let tail = (PskMode::Psk31.dcd_bits() + 1) * PskMode::Psk31.symbol_samples();
        let (body, end) = audio.split_at(audio.len() - tail);
        let mut r = String::from_utf8_lossy(&rx.process(body)).into_owned();
        assert!(rx.signal().1, "con il segnale lo squelch e' aperto");
        assert!(rx.dcd());
        r.push_str(&String::from_utf8_lossy(&rx.process(end)));
        assert!(r.contains("IU3ABC"), "ricevuto: {r:?}");
        assert!(!rx.dcd(), "la coda di portante spegne il DCD");
        assert!(!rx.signal().1);
    }

    #[test]
    fn l_afc_non_si_muove_sul_solo_rumore() {
        // squelch spento (il caso peggiore, fldigi stampa e insegue sempre): l'AFC resta fermo perche' la metrica e' bassa
        let mut seed = 7_u32;
        let noise: Vec<f32> = (0..12_000 * 20)
            .map(|_| {
                seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
                (seed as f32 / u32::MAX as f32 - 0.5) * 0.5
            })
            .collect();
        for mode in [PskMode::Psk31, PskMode::Psk63] {
            let mut rx = PskDemodulator::new(mode, 1000.0);
            rx.set_afc(true);
            let mut far = 0.0_f32;
            for block in noise.chunks(1000) {
                rx.process(block);
                far = far.max((rx.frequency() - 1000.0).abs());
            }
            assert!(far < 1.0, "{}: l'AFC si e' spostato di {far} Hz sul rumore", mode.name());
        }
    }

    #[test]
    fn s_n_e_imd_si_misurano_sul_segnale() {
        // segnale pulito con un po' di rumore: S/N alto; nel preambolo (inversioni continue) l'IMD e' bassa perche' il modulatore e' lineare
        let mut seed = 5_u32;
        let mut audio = vec![0.0_f32; 2000];
        audio.extend(modulate(b"cq cq de iu3abc iu3abc pse k", 1000.0, PskMode::Psk31, 0.5));
        for s in audio.iter_mut() {
            seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
            *s += (seed as f32 / u32::MAX as f32 - 0.5) * 0.02;
        }
        let mut rx = PskDemodulator::new(PskMode::Psk31, 1000.0);
        let tail = (PskMode::Psk31.dcd_bits() + 1) * PskMode::Psk31.symbol_samples();
        let (body, _) = audio.split_at(audio.len() - tail);
        // solo il preambolo: 32 inversioni
        let pre = 2000 + 33 * PskMode::Psk31.symbol_samples();
        rx.process(&body[..pre]);
        let imd = rx.imd_db();
        assert!(imd.is_some_and(|d| d < -20.0), "IMD nel preambolo: {imd:?}");
        rx.process(&body[pre..]);
        let snr = rx.snr_db();
        assert!(snr.is_some_and(|d| d > 15.0), "S/N: {snr:?}");
    }

    #[test]
    fn l_imd_di_un_trasmettitore_che_distorce_si_vede() {
        // compressione del picco (come un amplificatore saturato): i prodotti a ±3·baud/2 crescono e l'IMD sale
        let clean = modulate(b"", 1000.0, PskMode::Psk31, 0.5);
        let mut rx = PskDemodulator::new(PskMode::Psk31, 1000.0);
        let pre = 33 * PskMode::Psk31.symbol_samples();
        rx.process(&clean[..pre]);
        let good = rx.imd_db().unwrap_or(0.0);
        let dirty: Vec<f32> = clean.iter().map(|s| (s * 4.0).tanh() / 4.0).collect();
        let mut rx = PskDemodulator::new(PskMode::Psk31, 1000.0);
        rx.process(&dirty[..pre]);
        let bad = rx.imd_db().unwrap_or(-99.0);
        assert!(bad > good + 6.0 && bad > -25.0, "IMD pulito {good:.1} dB, distorto {bad:.1} dB");
    }

    #[test]
    fn passano_tutti_i_256_caratteri() {
        // anche i codici da 12 bit (è é ì ò ù e gli altri da 232 a 255), che prima si perdevano
        let all: Vec<u8> = (1..=255u8).collect();
        let audio = modulate(&all, 1500.0, PskMode::Psk63, 0.5);
        let mut rx = PskDemodulator::new(PskMode::Psk63, 1500.0);
        let mut input = vec![0.0; 4000];
        input.extend(audio);
        input.extend(vec![0.0; 4000]);
        let got = rx.process(&input);
        let start = got.iter().position(|&c| c == 1).unwrap_or(0);
        assert_eq!(&got[start..], &all[..], "ricevuto: {got:?}");
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

    /// QPSK31 ad anello chiuso: `ppm` scosta l'orologio di chi trasmette (ricampionamento lineare), `noise` l'ampiezza del rumore.
    fn qpsk_giro(text: &str, tx_hz: f32, rx_hz: f32, ppm: f64, noise: f32, rev: bool) -> String {
        let audio = modulate_qpsk(text.as_bytes(), tx_hz, PskMode::Qpsk31, 0.5, rev);
        let ratio = 1.0 + ppm * 1e-6;
        let len = (audio.len() as f64 / ratio) as usize;
        let audio: Vec<f32> = (0..len)
            .map(|i| {
                let t = i as f64 * ratio;
                let k = t as usize;
                let f = (t - k as f64) as f32;
                let a = audio.get(k).copied().unwrap_or(0.0);
                let b = audio.get(k + 1).copied().unwrap_or(0.0);
                a * (1.0 - f) + b * f
            })
            .collect();
        let mut seed = 4242_u32;
        let mut all = vec![0.0; 4000];
        all.extend(audio);
        all.extend(vec![0.0; 4000]);
        for x in all.iter_mut() {
            seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
            *x += (seed as f32 / u32::MAX as f32 - 0.5) * noise;
        }
        let mut rx = PskDemodulator::new(PskMode::Qpsk31, rx_hz);
        rx.set_squelch(Some(SQUELCH_DEFAULT));
        rx.set_reverse(rev);
        String::from_utf8_lossy(&rx.process(&all)).into_owned()
    }

    #[test]
    fn qpsk31_andata_e_ritorno() {
        let r = qpsk_giro("CQ CQ DE IU3ABC IU3ABC PSE K", 1000.0, 1000.0, 0.0, 0.0, false);
        assert!(r.contains("CQ CQ DE IU3ABC IU3ABC PSE K"), "ricevuto: {r:?}");
    }

    #[test]
    fn qpsk31_con_l_inversione_di_banda() {
        let r = qpsk_giro("the quick brown fox 73", 1500.0, 1500.0, 0.0, 0.0, true);
        assert!(r.contains("the quick brown fox 73"), "ricevuto: {r:?}");
        // senza l'inversione dalla parte di chi riceve il testo non esce
        let audio = modulate_qpsk(b"the quick brown fox 73", 1500.0, PskMode::Qpsk31, 0.5, true);
        let mut rx = PskDemodulator::new(PskMode::Qpsk31, 1500.0);
        rx.set_squelch(Some(SQUELCH_DEFAULT));
        let wrong = String::from_utf8_lossy(&rx.process(&audio)).into_owned();
        assert!(!wrong.contains("quick brown"), "ricevuto senza inversione: {wrong:?}");
    }

    #[test]
    fn qpsk31_con_rumore_scostamento_e_orologio() {
        let r = qpsk_giro("CQ CQ DE IU3ABC IU3ABC PSE K", 1000.0, 1002.0, 300.0, 0.6, false);
        assert!(r.contains("DE IU3ABC IU3ABC"), "ricevuto: {r:?}");
        let r = qpsk_giro("CQ CQ DE IU3ABC IU3ABC PSE K", 1000.0, 998.0, -300.0, 0.6, false);
        assert!(r.contains("DE IU3ABC IU3ABC"), "ricevuto: {r:?}");
    }

    #[test]
    fn qpsk31_l_afc_insegue_la_portante() {
        // con quattro fasi l'AFC della fase recupera meno di un ottavo della velocita' (3,9 Hz): oltre ci pensa l'aggancio
        let audio = modulate_qpsk(b"CQ CQ CQ DE IU3ABC IU3ABC PSE K", 1003.0, PskMode::Qpsk31, 0.5, false);
        let mut rx = PskDemodulator::new(PskMode::Qpsk31, 1000.0);
        rx.set_squelch(Some(SQUELCH_DEFAULT));
        rx.set_afc(true);
        let mut all = vec![0.0; 4000];
        all.extend(audio);
        let mut r = String::from_utf8_lossy(&rx.process(&all)).into_owned();
        let f = rx.frequency();
        r.push_str(&String::from_utf8_lossy(&rx.process(&[0.0; 12000])));
        assert!((f - 1003.0).abs() < 1.0, "frequenza raggiunta {f}");
        assert!(r.contains("IU3ABC PSE K"), "ricevuto {r:?}");
    }

    #[test]
    fn qpsk31_il_rumore_da_solo_non_stampa() {
        let mut seed = 99_u32;
        let noise: Vec<f32> = (0..12_000 * 10)
            .map(|_| {
                seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
                (seed as f32 / u32::MAX as f32 - 0.5) * 0.2
            })
            .collect();
        let mut rx = PskDemodulator::new(PskMode::Qpsk31, 1000.0);
        rx.set_squelch(Some(SQUELCH_DEFAULT));
        let out = rx.process(&noise);
        assert!(out.len() < 3, "dal rumore: {out:?}");
    }
}
