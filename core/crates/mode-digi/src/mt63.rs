//! MT63 (Pawel Jalocha SP9VRC): 64 portanti in parallelo, ognuna in BPSK differenziale, con un codice di Walsh a 64 bit per carattere e
//! l'interleaving nel tempo e fra le portanti. A 8 kHz.
//!
//! Le convenzioni sono quelle di fldigi 4.2.13 (`src/mt63/mt63base.cxx` e `dsp.cxx`, Pawel Jalocha e Dave Freese W1HKJ; GPL 3, copia in
//! `terze-parti/fldigi`), studiate e riscritte qui; da fldigi vengono le tabelle (forma del simbolo, schemi dell'interleaving):
//! - un carattere a 7 bit diventa una riga della matrice di Walsh di 64 (il bit 6 ne sceglie il segno); il bit 0 di una portante e' un
//!   salto di fase di 180 gradi, l'1 nessun salto;
//! - l'interleaving ritarda il bit della portante `i` di un numero di blocchi che cresce a passi (4, 5, 6, 7 ripetuti nel corto, da 32
//!   blocchi; 1, 2, ..., 63, 0 nel lungo, da 64 blocchi);
//! - le portanti sono a 4 righe di una FFT di 512 punti, alla frequenza decimata di 8000/8, 8000/4 e 8000/2 Hz per i modi da 500, 1000
//!   e 2000 Hz (5, 10 e 20 simboli al secondo); le portanti pari e dispari sono sfalsate di mezzo simbolo (100 campioni decimati su
//!   200); la forma del simbolo e' quella di 512 punti dell'MT63 originale, sovrapposta e sommata;
//! - un filtro passabanda complesso (finestra di Blackman a 4 termini, larghezza 1,5 volte il modo) porta il segnale decimato a 8 kHz;
//! - alla fine dei caratteri si svuota l'interleaving con dei NUL e si manda un simbolo di «disturbo» (fasi a 90 gradi a caso) perche'
//!   il ricevitore perda subito l'aggancio.
//!
//! In LSB lo spettro si rovescia intorno al centro delle portanti (coniugato e riportato al centro), in trasmissione e in ricezione.

// i cicli sugli indici seguono le formule (portanti, righe della FFT, campioni): piu' leggibili cosi'
#![allow(clippy::needless_range_loop)]

use rustfft::num_complex::Complex64 as C;
use rustfft::{Fft, FftPlanner};
use std::f64::consts::PI;
use std::sync::Arc;

/// La frequenza di campionamento di MT63 (Hz).
pub const RATE: f64 = 8000.0;
/// Lunghezza del simbolo e della FFT (campioni decimati).
const SYMBOL_LEN: usize = 512;
const MASK: usize = SYMBOL_LEN - 1;
/// Distanza fra due simboli sulla stessa portante (campioni decimati).
const SYMBOL_SEPAR: usize = 200;
/// Distanza fra due portanti (righe della FFT).
const CARR_SEPAR: usize = 4;
/// Numero di portanti.
const CARRIERS: usize = 64;
/// Ampiezza di una portante prima della FFT inversa (quella di fldigi, "massima senza distorsione").
const TX_AMPL: f64 = 4.0 / CARRIERS as f64;
/// Scala dell'uscita perche' il picco resti sotto 1 (vedi la prova `il_picco_resta_sotto_uno`); il guadagno del filtro di uscita cala
/// con la decimazione, e si compensa in [`Mt63Tx::new`].
const OUT_SCALE: f64 = 0.75;

/// Le sei varianti: larghezza di banda e interleaving corto (S) o lungo (L).
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum Mt63Variant {
    Mt63_500s,
    Mt63_500l,
    Mt63_1000s,
    #[default]
    Mt63_1000l,
    Mt63_2000s,
    Mt63_2000l,
}

impl Mt63Variant {
    pub const ALL: [Mt63Variant; 6] =
        [Mt63Variant::Mt63_500s, Mt63Variant::Mt63_500l, Mt63Variant::Mt63_1000s, Mt63Variant::Mt63_1000l, Mt63Variant::Mt63_2000s, Mt63Variant::Mt63_2000l];

    /// Larghezza di banda nominale (Hz).
    pub fn bandwidth_hz(self) -> f64 {
        match self {
            Mt63Variant::Mt63_500s | Mt63Variant::Mt63_500l => 500.0,
            Mt63Variant::Mt63_1000s | Mt63Variant::Mt63_1000l => 1000.0,
            Mt63Variant::Mt63_2000s | Mt63Variant::Mt63_2000l => 2000.0,
        }
    }

    /// Interleaving lungo (64 blocchi) o corto (32).
    pub fn long(self) -> bool {
        matches!(self, Mt63Variant::Mt63_500l | Mt63Variant::Mt63_1000l | Mt63Variant::Mt63_2000l)
    }

    /// Profondita' dell'interleaving (blocchi, cioe' simboli).
    pub fn interleave(self) -> usize {
        if self.long() { 64 } else { 32 }
    }

    /// Decimazione fra gli 8 kHz e la frequenza delle FFT.
    fn decimate(self) -> usize {
        match self.bandwidth_hz() as u32 {
            500 => 8,
            1000 => 4,
            _ => 2,
        }
    }

    /// Lunghezza del filtro passabanda (campioni a 8 kHz).
    fn filter_len(self) -> usize {
        if self.decimate() == 8 { 128 } else { 64 }
    }

    /// Simboli al secondo.
    pub fn baud(self) -> f64 {
        RATE / (self.decimate() * SYMBOL_SEPAR) as f64
    }

    /// Campioni a 8 kHz di un simbolo.
    pub fn symbol_samples(self) -> usize {
        self.decimate() * SYMBOL_SEPAR
    }

    /// La riga della FFT della prima portante per il centro `freq_hz` (come fldigi: puo' cadere oltre la meta', la FFT e' periodica).
    fn first_carrier(self, freq_hz: f64) -> i64 {
        let bw = self.bandwidth_hz();
        ((freq_hz - bw / 2.0) * (SYMBOL_LEN * self.decimate()) as f64 / RATE + 0.5).floor() as i64
    }

    fn pattern(self) -> [usize; CARRIERS] {
        let mut p = [0usize; CARRIERS];
        for (i, v) in p.iter_mut().enumerate() {
            *v = if self.long() { (i + 1) % CARRIERS } else { 4 + i % 4 };
        }
        p
    }

    pub fn id(self) -> &'static str {
        match self {
            Mt63Variant::Mt63_500s => "mt63_500s",
            Mt63Variant::Mt63_500l => "mt63_500l",
            Mt63Variant::Mt63_1000s => "mt63_1000s",
            Mt63Variant::Mt63_1000l => "mt63_1000l",
            Mt63Variant::Mt63_2000s => "mt63_2000s",
            Mt63Variant::Mt63_2000l => "mt63_2000l",
        }
    }

    /// Il nome come in fldigi e nel sottomodo ADIF.
    pub fn name(self) -> &'static str {
        match self {
            Mt63Variant::Mt63_500s => "MT63-500S",
            Mt63Variant::Mt63_500l => "MT63-500L",
            Mt63Variant::Mt63_1000s => "MT63-1000S",
            Mt63Variant::Mt63_1000l => "MT63-1000L",
            Mt63Variant::Mt63_2000s => "MT63-2000S",
            Mt63Variant::Mt63_2000l => "MT63-2000L",
        }
    }

    pub fn from_id(id: &str) -> Option<Mt63Variant> {
        let id = id.to_ascii_lowercase().replace('-', "_");
        Mt63Variant::ALL.into_iter().find(|v| v.id() == id || v.id().trim_start_matches("mt63_") == id)
    }
}

/// La forma del simbolo (512 punti, simmetrica).
fn symbol_shape() -> Vec<f64> {
    (0..SYMBOL_LEN).map(|i| SHAPE_HALF[if i <= SYMBOL_LEN / 2 { i } else { SYMBOL_LEN - i }]).collect()
}

fn twiddle(idx: usize) -> C {
    let a = 2.0 * PI * (idx & MASK) as f64 / SYMBOL_LEN as f64;
    C::new(a.cos(), a.sin())
}

/// La finestra di Blackman a 4 termini (fase da -pi a pi).
fn blackman3(phase: f64) -> f64 {
    0.35875 + 0.48829 * phase.cos() + 0.14128 * (2.0 * phase).cos() + 0.01168 * (3.0 * phase).cos()
}

/// Il passabanda complesso fra `low` e `upp` (rad/campione a 8 kHz): la parte reale e quella in quadratura.
fn band_shape(low: f64, upp: f64, len: usize) -> (Vec<f64>, Vec<f64>) {
    let mut si = vec![0.0; len];
    let mut sq = vec![0.0; len];
    for i in 0..len {
        let t = i as f64 + 1.0 - len as f64 / 2.0;
        let w = blackman3(2.0 * PI * t / len as f64);
        if t == 0.0 {
            si[i] = (upp - low) * w / PI;
            sq[i] = 0.0;
        } else {
            si[i] = ((upp * t).sin() - (low * t).sin()) / t * w / PI;
            sq[i] = ((upp * t).cos() - (low * t).cos()) / t * w / PI;
        }
    }
    (si, sq)
}

/// Le pulsazioni del passabanda per il centro `freq_hz` (1,5 volte la banda, fra 100 e 4000 Hz).
fn band_edges(v: Mt63Variant, freq_hz: f64) -> (f64, f64) {
    let hbw = 1.5 * v.bandwidth_hz() / 2.0;
    let low = (freq_hz - hbw).max(100.0);
    let upp = (freq_hz + hbw).min(4000.0);
    (low * PI / 4000.0, upp * PI / 4000.0)
}

/// Trasformata di Walsh inversa (quella del codificatore).
fn walsh_inv(d: &mut [f64; CARRIERS]) {
    let mut step = CARRIERS / 2;
    while step > 0 {
        let mut ptr = 0;
        while ptr < CARRIERS {
            for p in ptr..ptr + step {
                let (a, b) = (d[p], d[p + step]);
                d[p] = a - b;
                d[p + step] = a + b;
            }
            ptr += 2 * step;
        }
        step /= 2;
    }
}

/// Trasformata di Walsh diretta (quella del decodificatore).
#[cfg_attr(not(test), allow(dead_code))]
fn walsh(d: &mut [f64; CARRIERS]) {
    let mut step = 1;
    while step < CARRIERS {
        let mut ptr = 0;
        while ptr < CARRIERS {
            for p in ptr..ptr + step {
                let (a, b) = (d[p], d[p + step]);
                d[p] = a + b;
                d[p + step] = b - a;
            }
            ptr += 2 * step;
        }
        step *= 2;
    }
}

/// Un generatore pseudocasuale piccolo (riempimento iniziale dell'interleaving e simbolo di disturbo).
struct Lcg(u64);

impl Lcg {
    fn bit(&mut self) -> bool {
        self.0 = self.0.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        (self.0 >> 33) & 1 == 1
    }
}

/// Il codificatore: Walsh e interleaving. Esce un bit per portante (1 = nessun salto di fase).
struct Encoder {
    size: usize,
    patt: [usize; CARRIERS],
    pipe: Vec<u8>,
    ptr: usize,
}

impl Encoder {
    /// `fill`: il contenuto iniziale dell'interleaving (un bit per posto, `interleave * 64`).
    fn new(v: Mt63Variant, fill: &[u8]) -> Self {
        let len = v.interleave();
        let size = len * CARRIERS;
        let mut pipe = vec![0u8; size];
        for (d, s) in pipe.iter_mut().zip(fill) {
            *d = s & 1;
        }
        let pattern = v.pattern();
        let mut patt = [0usize; CARRIERS];
        let mut p = 0;
        for i in 0..CARRIERS {
            patt[i] = p * CARRIERS;
            p += pattern[i];
            if p >= len {
                p -= len;
            }
        }
        Encoder { size, patt, pipe, ptr: 0 }
    }

    fn process(&mut self, code: u8) -> [u8; CARRIERS] {
        let code = usize::from(code & 0x7f);
        let mut w = [0.0f64; CARRIERS];
        if code < CARRIERS {
            w[code] = 1.0;
        } else {
            w[code - CARRIERS] = -1.0;
        }
        walsh_inv(&mut w);
        for i in 0..CARRIERS {
            self.pipe[self.ptr + i] = u8::from(w[i] < 0.0);
        }
        let mut out = [0u8; CARRIERS];
        for (i, o) in out.iter_mut().enumerate() {
            let mut k = self.ptr + self.patt[i];
            if k >= self.size {
                k -= self.size;
            }
            *o = self.pipe[k + i];
        }
        self.ptr += CARRIERS;
        if self.ptr >= self.size {
            self.ptr -= self.size;
        }
        out
    }
}

/// Somma sovrapposta dei simboli (finestra lunga 512, passo 100).
struct OverlapAdd {
    buf: Vec<C>,
    window: Vec<f64>,
    dist: usize,
}

impl OverlapAdd {
    fn new(window: Vec<f64>, dist: usize) -> Self {
        OverlapAdd { buf: vec![C::new(0.0, 0.0); window.len()], window, dist }
    }

    fn process(&mut self, inp: &[C], out: &mut Vec<C>) {
        let (len, dist) = (self.window.len(), self.dist);
        for i in 0..dist {
            out.push(self.buf[i] + inp[i] * self.window[i]);
        }
        for i in dist..len - dist {
            self.buf[i - dist] = self.buf[i] + inp[i] * self.window[i];
        }
        for i in len - dist..len {
            self.buf[i - dist] = inp[i] * self.window[i];
        }
    }

    /// Un passo senza segnale nuovo (la coda dei simboli gia' entrati).
    fn silence(&mut self, out: &mut Vec<C>) {
        let (len, dist) = (self.window.len(), self.dist);
        out.extend_from_slice(&self.buf[..dist]);
        self.buf.copy_within(dist..len, 0);
        for v in &mut self.buf[len - dist..] {
            *v = C::new(0.0, 0.0);
        }
    }
}

/// Interpolazione e passabanda complesso: dal segnale decimato complesso agli 8 kHz reali.
struct Combiner {
    si: Vec<f64>,
    sq: Vec<f64>,
    tap: Vec<f64>,
    ptr: usize,
    rate: usize,
}

impl Combiner {
    fn new(v: Mt63Variant, freq_hz: f64) -> Self {
        let len = v.filter_len();
        let (low, upp) = band_edges(v, freq_hz);
        let (si, sq) = band_shape(low, upp, len);
        Combiner { si, sq, tap: vec![0.0; len], ptr: 0, rate: v.decimate() }
    }

    fn process(&mut self, inp: &[C], out: &mut Vec<f64>) {
        let len = self.tap.len();
        for z in inp {
            for r in 0..len {
                let t = (self.ptr + r) % len;
                self.tap[t] += z.re * self.si[r] + z.im * self.sq[r];
            }
            for _ in 0..self.rate {
                out.push(self.tap[self.ptr]);
                self.tap[self.ptr] = 0.0;
                self.ptr = (self.ptr + 1) % len;
            }
        }
    }

    /// La coda del filtro.
    fn flush(&mut self, out: &mut Vec<f64>) {
        let len = self.tap.len();
        for _ in 0..len {
            out.push(self.tap[self.ptr]);
            self.tap[self.ptr] = 0.0;
            self.ptr = (self.ptr + 1) % len;
        }
    }
}

/// Il trasmettitore MT63 a 8 kHz.
pub struct Mt63Tx {
    v: Mt63Variant,
    first: usize,
    centre_bins: usize,
    reverse: bool,
    amp: f64,
    encoder: Encoder,
    vect: [usize; CARRIERS],
    corr: [usize; CARRIERS],
    ifft: Arc<dyn Fft<f64>>,
    window: OverlapAdd,
    comb: Combiner,
    /// Campioni decimati usciti dalla finestra (per il rovesciamento in LSB).
    n: usize,
    rng: Lcg,
    jam_bits: Option<Vec<bool>>,
}

impl Mt63Tx {
    /// Trasmettitore centrato a `freq_hz`, ampiezza di picco circa `amp`; `reverse`: la radio in LSB (spettro rovesciato).
    pub fn new(v: Mt63Variant, freq_hz: f64, amp: f64, reverse: bool) -> Self {
        let mut rng = Lcg(0x4d54_3633_u64 ^ (freq_hz.to_bits()));
        let fill: Vec<u8> = (0..v.interleave() * CARRIERS).map(|_| u8::from(rng.bit())).collect();
        let mut t = Self::with_fill(v, freq_hz, amp * OUT_SCALE * v.decimate() as f64 / 2.0, reverse, &fill);
        t.rng = rng;
        t
    }

    /// Come [`Mt63Tx::new`] con il riempimento iniziale dell'interleaving dato e l'uscita grezza moltiplicata per `gain` (prove).
    pub fn with_fill(v: Mt63Variant, freq_hz: f64, gain: f64, reverse: bool, fill: &[u8]) -> Self {
        let first = v.first_carrier(freq_hz).rem_euclid(SYMBOL_LEN as i64) as usize;
        let mut vect = [0usize; CARRIERS];
        let (mut step, mut p) = (0usize, 0usize);
        for x in vect.iter_mut() {
            *x = p;
            step += 1;
            p = (p + step) & MASK;
        }
        let mut corr = [0usize; CARRIERS];
        let incr = (SYMBOL_SEPAR * CARR_SEPAR) & MASK;
        let mut p = (SYMBOL_SEPAR * first) & MASK;
        for c in corr.iter_mut() {
            *c = p;
            p = (p + incr) & MASK;
        }
        Mt63Tx {
            v,
            first,
            centre_bins: first + (CARRIERS - 1) * CARR_SEPAR / 2,
            reverse,
            amp: gain,
            encoder: Encoder::new(v, fill),
            vect,
            corr,
            ifft: FftPlanner::new().plan_fft_inverse(SYMBOL_LEN),
            window: OverlapAdd::new(symbol_shape(), SYMBOL_SEPAR / 2),
            comb: Combiner::new(v, freq_hz),
            n: 0,
            rng: Lcg(1),
            jam_bits: None,
        }
    }

    /// I bit del simbolo di disturbo (uno per portante: vero = 90 gradi a sinistra), al posto di quelli a caso (prove).
    pub fn set_jam_bits(&mut self, bits: Vec<bool>) {
        self.jam_bits = Some(bits);
    }

    pub fn variant(&self) -> Mt63Variant {
        self.v
    }

    /// Le fasi correnti diventano un simbolo: due FFT inverse (portanti pari e dispari), finestra sovrapposta, passabanda a 8 kHz.
    fn emit(&mut self) -> Vec<f32> {
        let mut bufs = [vec![C::new(0.0, 0.0); SYMBOL_LEN], vec![C::new(0.0, 0.0); SYMBOL_LEN]];
        for i in 0..CARRIERS {
            let bin = (self.first + i * CARR_SEPAR) & MASK;
            bufs[i & 1][bin] = twiddle(self.vect[i]) * TX_AMPL;
        }
        let mut z = Vec::with_capacity(SYMBOL_SEPAR);
        for b in &mut bufs {
            self.ifft.process(b);
            self.window.process(b, &mut z);
        }
        self.finish(z)
    }

    fn finish(&mut self, mut z: Vec<C>) -> Vec<f32> {
        if self.reverse {
            // specchio intorno al centro delle portanti: coniugato e riportato al centro
            for s in &mut z {
                let rot = twiddle((2 * self.centre_bins * (self.n & MASK)) & MASK);
                *s = s.conj() * rot;
                self.n += 1;
            }
        } else {
            self.n += z.len();
        }
        let mut out = Vec::with_capacity(z.len() * self.v.decimate());
        self.comb.process(&z, &mut out);
        out.into_iter().map(|x| (x * self.amp) as f32).collect()
    }

    /// Un carattere (7 bit) diventa un simbolo.
    fn send_code(&mut self, c: u8) -> Vec<f32> {
        let bits = self.encoder.process(c);
        for i in 0..CARRIERS {
            let flip = if bits[i] != 0 { 0 } else { SYMBOL_LEN / 2 };
            self.vect[i] = (self.vect[i] + self.corr[i] + flip) & MASK;
        }
        self.emit()
    }

    /// Il simbolo di disturbo: ogni portante gira di 90 gradi a caso.
    fn jam(&mut self) -> Vec<f32> {
        for i in 0..CARRIERS {
            let left = match &self.jam_bits {
                Some(b) => b.get(i).copied().unwrap_or(false),
                None => self.rng.bit(),
            };
            let turn = if left { SYMBOL_LEN / 4 } else { 3 * SYMBOL_LEN / 4 };
            self.vect[i] = (self.vect[i] + self.corr[i] + turn) & MASK;
        }
        self.emit()
    }

    /// I NUL che riempiono l'interleaving senza uscire (come fldigi, che li calcola e non li manda).
    fn silent_run_in(&mut self) {
        for _ in 0..self.v.interleave() {
            let _ = self.send_code(0);
        }
    }

    /// L'inizio: l'interleaving si riempie in silenzio, poi 16 simboli di attesa (il tempo che il ricevitore si agganci).
    pub fn preamble(&mut self) -> Vec<f32> {
        self.silent_run_in();
        let mut out = Vec::new();
        for _ in 0..16 {
            out.extend(self.send_code(0));
        }
        out
    }

    /// Un carattere: oltre i 7 bit diventa un punto, come in fldigi senza gli 8 bit.
    pub fn char(&mut self, c: u8) -> Vec<f32> {
        let c = if c > 127 { b'.' } else { c };
        self.send_code(c)
    }

    /// Attesa: un NUL (chi riceve non lo stampa).
    pub fn idle(&mut self) -> Vec<f32> {
        self.send_code(0)
    }

    /// La fine: NUL fino a svuotare l'interleaving, il simbolo di disturbo, poi la coda della finestra e del filtro.
    pub fn postamble(&mut self) -> Vec<f32> {
        let mut out = Vec::new();
        for _ in 1..self.v.interleave() {
            out.extend(self.send_code(0));
        }
        out.extend(self.jam());
        let mut z = Vec::new();
        for _ in 0..SYMBOL_LEN / (SYMBOL_SEPAR / 2) + 1 {
            self.window.silence(&mut z);
        }
        out.extend(self.finish(z));
        let mut tail = Vec::new();
        self.comb.flush(&mut tail);
        out.extend(tail.into_iter().map(|x| (x * self.amp) as f32));
        out
    }

    /// Tutto il testo in un colpo solo.
    pub fn transmit(&mut self, text: &[u8]) -> Vec<f32> {
        let mut out = self.preamble();
        for &c in text {
            out.extend(self.char(c));
        }
        out.extend(self.postamble());
        out
    }

    /// La sequenza esatta di fldigi (prove): interleaving riempito in silenzio, il testo, `interleave - 1` NUL e il disturbo, senza code.
    pub fn transmit_like_fldigi(&mut self, text: &[u8]) -> Vec<f32> {
        self.silent_run_in();
        let mut out = Vec::new();
        for &c in text {
            out.extend(self.char(c));
        }
        for _ in 1..self.v.interleave() {
            out.extend(self.send_code(0));
        }
        out.extend(self.jam());
        out
    }
}

/// La meta' della forma del simbolo dell'MT63 originale (punti da 0 a 256; la seconda meta' e' lo specchio), dalla tabella `symbol.dat`
/// di fldigi.
const SHAPE_HALF: [f64; 257] = [
    0.0, 0.00000665, 0.00002657, 0.00005975, 0.00010613, 0.00016562, 0.00023810, 0.00032341, 0.00042134, 0.00053162,
    0.00065389, 0.00078773, 0.00093261, 0.00108789, 0.00125283, 0.00142653, 0.00160798, 0.00179599, 0.00198926, 0.00218628,
    0.00238542, 0.00258487, 0.00278264, 0.00297662, 0.00316452, 0.00334394, 0.00351232, 0.00366701, 0.00380526, 0.00392424,
    0.00402109, 0.00409288, 0.00413671, 0.00414969, 0.00412898, 0.00407182, 0.00397555, 0.00383764, 0.00365574, 0.00342767,
    0.00315145, 0.00282534, 0.00244787, 0.00201781, 0.00153424, 0.00099653, 0.00040435, -0.00024231, -0.00094314, -0.00169753,
    -0.00250453, -0.00336293, -0.00427118, -0.00522749, -0.00622977, -0.00727569, -0.00836272, -0.00948809, -0.01064886, -0.01184193,
    -0.01306405, -0.01431189, -0.01558198, -0.01687083, -0.01817486, -0.01949051, -0.02081416, -0.02214223, -0.02347113, -0.02479733,
    -0.02611728, -0.02742752, -0.02872457, -0.03000504, -0.03126551, -0.03250262, -0.03371298, -0.03489320, -0.03603988, -0.03714954,
    -0.03821868, -0.03924367, -0.04022079, -0.04114620, -0.04201589, -0.04282570, -0.04357126, -0.04424801, -0.04485118, -0.04537575,
    -0.04581648, -0.04616787, -0.04642421, -0.04657955, -0.04662769, -0.04656225, -0.04637665, -0.04606414, -0.04561786, -0.04503082,
    -0.04429599, -0.04340631, -0.04235475, -0.04113436, -0.03973834, -0.03816006, -0.03639316, -0.03443155, -0.03226956, -0.02990192,
    -0.02732385, -0.02453112, -0.02152012, -0.01828789, -0.01483216, -0.01115146, -0.00724508, -0.00311317, 0.00124328, 0.00582236,
    0.01062127, 0.01563627, 0.02086273, 0.02629504, 0.03192674, 0.03775043, 0.04375787, 0.04993995, 0.05628681, 0.06278780,
    0.06943159, 0.07620621, 0.08309914, 0.09009732, 0.09718730, 0.10435526, 0.11158715, 0.11886870, 0.12618560, 0.13352351,
    0.14086819, 0.14820561, 0.15552198, 0.16280389, 0.17003841, 0.17721311, 0.18431620, 0.19133661, 0.19826401, 0.20508896,
    0.21180289, 0.21839823, 0.22486845, 0.23120806, 0.23741270, 0.24347919, 0.24940549, 0.25519079, 0.26083547, 0.26634116,
    0.27171067, 0.27694807, 0.28205857, 0.28704860, 0.29192571, 0.29669855, 0.30137684, 0.30597130, 0.31049362, 0.31495636,
    0.31937292, 0.32375741, 0.32812465, 0.33249001, 0.33686936, 0.34127898, 0.34573545, 0.35025554, 0.35485613, 0.35955412,
    0.36436627, 0.36930915, 0.37439902, 0.37965170, 0.38508250, 0.39070609, 0.39653642, 0.40258662, 0.40886890, 0.41539446,
    0.42217341, 0.42921470, 0.43652603, 0.44411383, 0.45198311, 0.46013753, 0.46857925, 0.47730896, 0.48632585, 0.49562756,
    0.50521021, 0.51506840, 0.52519520, 0.53558220, 0.54621950, 0.55709582, 0.56819849, 0.57951351, 0.59102568, 0.60271860,
    0.61457478, 0.62657574, 0.63870210, 0.65093366, 0.66324951, 0.67562817, 0.68804763, 0.70048553, 0.71291922, 0.72532590,
    0.73768272, 0.74996688, 0.76215572, 0.77422687, 0.78615828, 0.79792836, 0.80951602, 0.82090079, 0.83206287, 0.84298315,
    0.85364335, 0.86402598, 0.87411443, 0.88389296, 0.89334677, 0.90246195, 0.91122553, 0.91962547, 0.92765062, 0.93529073,
    0.94253642, 0.94937916, 0.95581122, 0.96182562, 0.96741616, 0.97257728, 0.97730410, 0.98159233, 0.98543825, 0.98883864,
    0.99179079, 0.99429241, 0.99634163, 0.99793696, 0.99907728, 0.99976178, 0.99999000,
];

#[cfg(test)]
mod tests {
    use super::*;

    /// Walsh diretta dopo l'inversa: torna la riga del carattere moltiplicata per 64.
    #[test]
    fn walsh_avanti_e_indietro() {
        for code in [0usize, 1, 37, 63] {
            let mut w = [0.0f64; CARRIERS];
            w[code] = 1.0;
            walsh_inv(&mut w);
            walsh(&mut w);
            for (i, x) in w.iter().enumerate() {
                let want = if i == code { CARRIERS as f64 } else { 0.0 };
                assert!((x - want).abs() < 1e-9, "codice {code}, posto {i}: {x}");
            }
        }
    }

    /// Il picco dell'uscita resta sotto 1 (ampiezza 1) in tutte le varianti, con testi diversi e in LSB.
    #[test]
    fn il_picco_resta_sotto_uno() {
        let mut worst = 0.0f32;
        for v in Mt63Variant::ALL {
            for (k, t) in ["", "CQ CQ DE IU3ABC", "~~~~~~~~~~~~~~~~~~~~", "\0\0\0\0\0\0\0\0", "The quick brown fox 0123456789"].iter().enumerate() {
                for rev in [false, true] {
                    let a = Mt63Tx::new(v, 1000.0 + 250.0 * k as f64, 1.0, rev).transmit(t.as_bytes());
                    let peak = a.iter().fold(0.0f32, |m, x| m.max(x.abs()));
                    worst = worst.max(peak);
                    assert!(peak < 1.0 && peak > 0.5, "{v:?} {t:?} rev {rev}: picco {peak}");
                }
            }
        }
        eprintln!("picco massimo {worst}");
    }
}
