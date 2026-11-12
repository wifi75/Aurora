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
//!   di fila.
//!
//! Per le decisioni «soft» future (varicode con più ipotesi, più ricevitori in parallelo) ogni simbolo lascia anche la sua affidabilità
//! ([`PskDemodulator::soft_bits`]), non solo il bit deciso.

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
    /// Lunghezza del preambolo, della coda e della finestra del DCD in simboli (fldigi `dcdbits`: 32 in PSK31, 64 in PSK63).
    pub fn dcd_bits(self) -> usize {
        match self {
            PskMode::Psk31 => 32,
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
        }
    }

    pub fn mode(&self) -> PskMode {
        self.mode
    }

    /// La frequenza scelta dall'utente: l'AFC riparte da qui.
    pub fn set_frequency(&mut self, freq_hz: f32) {
        self.base_hz = freq_hz;
        self.tune(freq_hz);
    }

    fn tune(&mut self, freq_hz: f32) {
        self.freq_hz = freq_hz;
        self.step = 2.0 * PI * f64::from(freq_hz) / f64::from(RATE);
    }

    /// La frequenza su cui si sta ascoltando adesso (con l'AFC puo' essere vicina a quella scelta, non uguale).
    pub fn frequency(&self) -> f32 {
        self.freq_hz
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
        let len1 = self.ring1.len();
        for &s in samples {
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
                }
            }
            _ => {
                self.dcd = self.squelch.is_none() || self.open;
                self.dcd_off = self.dcd_off.saturating_sub(1);
            }
        }
        self.zeros = if one { 0 } else { self.zeros.saturating_add(1) };

        if self.dcd {
            self.push_bit(one, out);
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
}
