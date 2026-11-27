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
//! Il ricevitore segue la stessa struttura di quello di fldigi (riscritto): passabanda e decimazione, sincronismo di tempo e frequenza
//! dalla correlazione fra simboli successivi quattro volte per simbolo, demodulazione differenziale delle portanti, deinterleaving e
//! decodifica di Walsh con la ricerca della prima portante su +/-8 posizioni. Con lo squelch spento da' gli stessi caratteri di quello di
//! fldigi sullo stesso segnale (prova `nel_rumore_come_fldigi` dell'oracolo); lo squelch nostro guarda il rapporto segnale/rumore del
//! decodificatore, quello del singolo carattere, l'aggancio del sincronismo e i ritardi di inizio e fine. Costa poco (un minuto di audio
//! in qualche decina di millisecondi, `misura_limiti`).
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
    pub const ALL: [Mt63Variant; 6] = [
        Mt63Variant::Mt63_500s,
        Mt63Variant::Mt63_500l,
        Mt63Variant::Mt63_1000s,
        Mt63Variant::Mt63_1000l,
        Mt63Variant::Mt63_2000s,
        Mt63Variant::Mt63_2000l,
    ];

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

// ---------------------------------------------------------------------------------------------------------------------------------
// Ricevitore

/// Passa-basso del secondo ordine a due stadi (quello di Jalocha): `mid` e `out` sono lo stato.
#[inline]
fn lowpass2(inp: f64, mid: &mut f64, out: &mut f64, w: (f64, f64, f64)) {
    let sum = *mid + *out;
    let diff = *mid - *out;
    *mid += w.1 * inp - w.0 * sum;
    *out += w.2 * diff;
}

#[inline]
fn lowpass2c(inp: C, mid: &mut C, out: &mut C, w: (f64, f64, f64)) {
    lowpass2(inp.re, &mut mid.re, &mut out.re, w);
    lowpass2(inp.im, &mut mid.im, &mut out.im, w);
}

fn lp_weights(integ: f64) -> (f64, f64, f64) {
    (1.0 / integ, 2.0 / integ, 5.0 / integ)
}

/// Media «selettiva»: si scartano a ogni giro i valori oltre `thres` volte lo scarto e si ricalcola. Restituisce media e scarto.
fn sel_fit_aver(data: &[f64], thres: f64, loops: usize) -> (f64, f64) {
    let n = data.len() as f64;
    let mut lev = data.iter().sum::<f64>() / n;
    let mut err = data.iter().map(|x| x * x).sum::<f64>() / n - lev * lev;
    for _ in 0..loops {
        let t = thres * thres * err;
        let (mut sum, mut e, mut incl) = (0.0, 0.0, 0usize);
        for &x in data {
            let d = (x - lev) * (x - lev);
            if d <= t {
                sum += x;
                e += d;
                incl += 1;
            }
        }
        if incl == 0 {
            break;
        }
        let dlev = sum / incl as f64 - lev;
        err = (e / incl as f64 - dlev * dlev).abs();
        lev += dlev;
    }
    (lev, err.max(0.0).sqrt())
}

fn sel_fit_aver_c(data: &[C], thres: f64, loops: usize) -> (C, f64) {
    let n = data.len() as f64;
    let mut lev = data.iter().sum::<C>() / n;
    let mut err = data.iter().map(|x| x.norm_sqr()).sum::<f64>() / n - lev.norm_sqr();
    for _ in 0..loops {
        let t = 0.5 * thres * thres * err;
        let (mut sum, mut e, mut incl) = (C::new(0.0, 0.0), 0.0, 0usize);
        for &x in data {
            let d = (x - lev).norm_sqr();
            if d <= t {
                sum += x;
                e += d;
                incl += 1;
            }
        }
        if incl == 0 {
            break;
        }
        let dlev = sum / incl as f64 - lev;
        err = (e / incl as f64 - dlev.norm_sqr()).abs();
        lev += dlev;
    }
    (lev, err.max(0.0).sqrt())
}

/// Passabanda complesso e decimazione: dagli 8 kHz reali al segnale decimato complesso.
struct Splitter {
    si: Vec<f64>,
    sq: Vec<f64>,
    tap: Vec<f64>,
    rate: usize,
}

impl Splitter {
    fn new(v: Mt63Variant, freq_hz: f64) -> Self {
        let len = v.filter_len();
        let (low, upp) = band_edges(v, freq_hz);
        let (si, sq) = band_shape(low, upp, len);
        Splitter { si, sq, tap: vec![0.0; len], rate: v.decimate() }
    }

    fn process(&mut self, inp: &[f32], out: &mut Vec<C>) {
        self.tap.extend(inp.iter().map(|&x| f64::from(x)));
        let len = self.si.len();
        let mut i = 0;
        while i + len < self.tap.len() {
            let (mut a, mut b) = (0.0, 0.0);
            for t in 0..len {
                let x = self.tap[i + t];
                a += x * self.si[t];
                b += x * self.sq[t];
            }
            out.push(C::new(a, b));
            i += self.rate;
        }
        self.tap.drain(..i);
    }
}

/// Deinterleaving e decodifica di Walsh, con la ricerca della prima portante su +/-`margin` posizioni.
struct Decoder {
    scan_len: usize,
    scan_size: usize,
    w: (f64, f64, f64),
    pipe: Vec<f64>,
    size: usize,
    ptr: usize,
    patt: [usize; CARRIERS],
    snr_mid: Vec<f64>,
    snr_out: Vec<f64>,
    codes: Vec<u8>,
    codes_len: usize,
    codes_ptr: usize,
    output: u8,
    snr: f64,
    carr_ofs: i32,
    raw: Vec<f64>,
}

impl Decoder {
    fn new(v: Mt63Variant, margin: usize, integ: usize) -> Self {
        let scan_len = 2 * margin + 1;
        let scan_size = CARRIERS + 2 * margin;
        let len = v.interleave();
        let pattern = v.pattern();
        let mut patt = [0usize; CARRIERS];
        let mut p = 0;
        for i in 0..CARRIERS {
            patt[i] = p * scan_size;
            p += pattern[i];
            if p >= len {
                p -= len;
            }
        }
        let size = (len + 1) * scan_size;
        let codes_len = integ / 2 * scan_len;
        Decoder {
            scan_len,
            scan_size,
            w: lp_weights(integ as f64),
            pipe: vec![0.0; size],
            size,
            ptr: 0,
            patt,
            snr_mid: vec![0.0; scan_len],
            snr_out: vec![0.0; scan_len],
            codes: vec![0; codes_len],
            codes_len,
            codes_ptr: 0,
            output: 0,
            snr: 0.0,
            carr_ofs: 0,
            raw: vec![0.0; scan_len],
        }
    }

    fn process(&mut self, data: &[f64]) {
        self.pipe[self.ptr..self.ptr + self.scan_size].copy_from_slice(&data[..self.scan_size]);
        let mut w = [0.0f64; CARRIERS];
        for s in 0..self.scan_len {
            for (i, wi) in w.iter_mut().enumerate() {
                let mut k = self.ptr as isize - self.scan_size as isize - self.patt[i] as isize;
                if k < 0 {
                    k += self.size as isize;
                }
                let mut k = k as usize;
                // con uno scarto dispari le portanti dispari arrivano mezzo simbolo prima: si prendono dal blocco dopo
                if s & 1 == 1 && i & 1 == 1 {
                    k += self.scan_size;
                    if k >= self.size {
                        k -= self.size;
                    }
                }
                *wi = self.pipe[k + s + i];
            }
            walsh(&mut w);
            let (mut min, mut min_pos, mut max, mut max_pos) = (w[0], 0, w[0], 0);
            for (i, &x) in w.iter().enumerate().skip(1) {
                if x < min {
                    min = x;
                    min_pos = i;
                }
                if x > max {
                    max = x;
                    max_pos = i;
                }
            }
            let (code, sig) = if max.abs() > min.abs() {
                w[max_pos] = 0.0;
                (max_pos + CARRIERS, max.abs())
            } else {
                w[min_pos] = 0.0;
                (min_pos, min.abs())
            };
            let noise = (w.iter().map(|x| x * x).sum::<f64>() / CARRIERS as f64).sqrt();
            let snr = if noise > 0.0 { sig / noise } else { 0.0 };
            self.raw[s] = snr;
            lowpass2(snr, &mut self.snr_mid[s], &mut self.snr_out[s], self.w);
            self.codes[self.codes_ptr + s] = code as u8;
        }
        self.ptr += self.scan_size;
        if self.ptr >= self.size {
            self.ptr = 0;
        }
        self.codes_ptr += self.scan_len;
        if self.codes_ptr >= self.codes_len {
            self.codes_ptr = 0;
        }
        let (mut max, mut pos) = (self.snr_out[0], 0);
        for (i, &x) in self.snr_out.iter().enumerate().skip(1) {
            if x > max {
                max = x;
                pos = i;
            }
        }
        self.output = self.codes[self.codes_ptr + pos];
        self.snr = max;
        self.carr_ofs = pos as i32 - (self.scan_len as i32 - 1) / 2;
    }
}

/// Simboli di integrazione del sincronismo: 16 (normale) o 32 (segnali deboli, aggancio piu' lento).
pub const INTEG_SHORT: usize = 16;
pub const INTEG_LONG: usize = 32;
/// Righe cercate sopra e sotto le portanti (8 portanti).
const SCAN_MARGIN: usize = 8;
const SYMBOL_DIV: usize = 4;
const SYNC_STEP: usize = SYMBOL_SEPAR / SYMBOL_DIV;
/// Soglia dello squelch sul rapporto segnale/rumore del decodificatore (dB; vedi `nel_solo_rumore_tace`).
const SQUELCH_DB: f32 = 5.5;
/// Soglia che spegne lo squelch.
pub const SQUELCH_OFF: f32 = -50.0;
/// Rapporto minimo (lineare) del singolo carattere: nel solo rumore sta intorno a 3, un carattere buono sopra 6.
const RAW_MIN: f64 = 3.5;

/// Il ricevitore MT63 a 8 kHz.
pub struct Mt63Rx {
    v: Mt63Variant,
    centre_hz: f64,
    reverse: bool,
    integ: usize,
    centre_bins: usize,
    split: Splitter,
    fft: Arc<dyn Fft<f64>>,
    window: Vec<f64>,
    tw: Vec<C>,
    /// La linea di ritardo dei campioni decimati; `inp` e' il primo del pezzo appena arrivato.
    line: Vec<C>,
    keep: usize,
    inp: usize,
    n: usize,
    proc_delay: isize,
    sync_proc_ptr: isize,
    data_proc_ptr: isize,
    // sincronismo
    scan_first: usize,
    scan_len: usize,
    fit_len: usize,
    sync_pipe: [Vec<C>; SYMBOL_DIV],
    sync_ptr: usize,
    symb_ptr: usize,
    sync_ph_corr: Vec<C>,
    correl_mid: [Vec<C>; SYMBOL_DIV],
    correl_out: [Vec<C>; SYMBOL_DIV],
    power_mid: Vec<f64>,
    power_out: Vec<f64>,
    correl_norm: [Vec<C>; SYMBOL_DIV],
    correl_aver: [Vec<C>; SYMBOL_DIV],
    w: (f64, f64, f64),
    wp: (f64, f64, f64),
    symb_fit: Vec<C>,
    symb_fit_pos: isize,
    freq_pipe: Vec<f64>,
    symb_pipe: Vec<C>,
    track_ptr: usize,
    aver_freq: f64,
    aver_symb: C,
    lock_thres: f64,
    hold_thres: f64,
    locked: bool,
    sync_freq_ofs: f64,
    sync_symb_shift: f64,
    // dati
    data_scan_first: i64,
    data_scan_len: usize,
    ref_slice: Vec<C>,
    data_vect: Vec<C>,
    data_pipe: Vec<Vec<C>>,
    data_pipe_ptr: usize,
    data_pwr_mid: Vec<f64>,
    data_pwr_out: Vec<f64>,
    dw: (f64, f64, f64),
    data_phase: Vec<f64>,
    fft_a: Vec<C>,
    fft_b: Vec<C>,
    decoder: Decoder,
    out: Vec<u8>,
    snr: f64,
    squelch_db: f32,
    /// Simboli dati passati dall'ultimo aggancio.
    unlocked: usize,
    last_freq: f64,
    /// Simboli dati passati dall'aggancio (dopo uno sgancio vero).
    since_lock: usize,
}

impl Mt63Rx {
    /// Ricevitore centrato a `centre_hz`; `reverse`: la radio in LSB; `long_integ`: integrazione di 32 simboli invece di 16.
    pub fn new(v: Mt63Variant, centre_hz: f64, reverse: bool, long_integ: bool) -> Self {
        let integ = if long_integ { INTEG_LONG } else { INTEG_SHORT };
        let first = v.first_carrier(centre_hz).rem_euclid(SYMBOL_LEN as i64) as usize;
        let proc_delay = integ * SYMBOL_SEPAR;
        let keep = proc_delay + SYMBOL_LEN + SYMBOL_SEPAR + SYMBOL_LEN;
        let scan_first = (first + SYMBOL_LEN - SCAN_MARGIN * CARR_SEPAR) & MASK;
        let scan_len = (CARRIERS + 2 * SCAN_MARGIN) * CARR_SEPAR;
        let tw: Vec<C> = (0..SYMBOL_LEN).map(twiddle).collect();
        let mut sync_ph_corr = Vec::with_capacity(scan_len);
        let mut c = (scan_first * SYMBOL_SEPAR) & MASK;
        for _ in 0..scan_len {
            sync_ph_corr.push(tw[c] * tw[c]);
            c = (c + SYMBOL_SEPAR) & MASK;
        }
        let fit_len = 2 * SCAN_MARGIN * CARR_SEPAR;
        let zc = |n: usize| vec![C::new(0.0, 0.0); n];
        let hold = 1.5 * (1.0 / (integ * CARRIERS) as f64).sqrt();
        let data_scan_len = CARRIERS + 2 * SCAN_MARGIN;
        Mt63Rx {
            v,
            centre_hz,
            reverse,
            integ,
            centre_bins: first + (CARRIERS - 1) * CARR_SEPAR / 2,
            split: Splitter::new(v, centre_hz),
            fft: FftPlanner::new().plan_fft_forward(SYMBOL_LEN),
            window: symbol_shape(),
            tw,
            line: zc(keep),
            keep,
            inp: keep,
            n: 0,
            proc_delay: proc_delay as isize,
            sync_proc_ptr: 0,
            data_proc_ptr: -(proc_delay as isize),
            scan_first,
            scan_len,
            fit_len,
            sync_pipe: std::array::from_fn(|_| zc(scan_len)),
            sync_ptr: 0,
            symb_ptr: 0,
            sync_ph_corr,
            correl_mid: std::array::from_fn(|_| zc(scan_len)),
            correl_out: std::array::from_fn(|_| zc(scan_len)),
            power_mid: vec![0.0; scan_len],
            power_out: vec![0.0; scan_len],
            correl_norm: std::array::from_fn(|_| zc(scan_len)),
            correl_aver: std::array::from_fn(|_| zc(fit_len)),
            w: lp_weights(integ as f64),
            wp: lp_weights((integ * SYMBOL_DIV) as f64),
            symb_fit: zc(fit_len),
            symb_fit_pos: (SCAN_MARGIN * CARR_SEPAR) as isize,
            freq_pipe: vec![0.0; integ],
            symb_pipe: zc(integ),
            track_ptr: 0,
            aver_freq: 0.0,
            aver_symb: C::new(0.0, 0.0),
            lock_thres: 1.5 * hold,
            hold_thres: hold,
            locked: false,
            sync_freq_ofs: 0.0,
            sync_symb_shift: 0.0,
            data_scan_first: first as i64 - (SCAN_MARGIN * CARR_SEPAR) as i64,
            data_scan_len,
            ref_slice: zc(data_scan_len),
            data_vect: zc(data_scan_len),
            data_pipe: vec![zc(data_scan_len); integ / 2],
            data_pipe_ptr: 0,
            data_pwr_mid: vec![0.0; data_scan_len],
            data_pwr_out: vec![0.0; data_scan_len],
            dw: lp_weights(integ as f64),
            data_phase: vec![0.0; data_scan_len],
            fft_a: zc(SYMBOL_LEN),
            fft_b: zc(SYMBOL_LEN),
            decoder: Decoder::new(v, SCAN_MARGIN, integ),
            out: Vec::new(),
            snr: 0.0,
            squelch_db: SQUELCH_DB,
            unlocked: usize::MAX,
            last_freq: centre_hz,
            since_lock: 0,
        }
    }

    pub fn variant(&self) -> Mt63Variant {
        self.v
    }

    /// Audio a 8 kHz.
    pub fn process(&mut self, audio: &[f32]) {
        let mut z = Vec::with_capacity(audio.len() / self.v.decimate() + 2);
        self.split.process(audio, &mut z);
        if self.reverse {
            for s in &mut z {
                let rot = self.tw[(2 * self.centre_bins * (self.n & MASK)) & MASK];
                *s = s.conj() * rot;
                self.n += 1;
            }
        }
        self.feed(&z);
    }

    fn feed(&mut self, z: &[C]) {
        if self.line.len() > self.keep {
            let cut = self.line.len() - self.keep;
            self.line.drain(..cut);
        }
        self.inp = self.line.len();
        self.line.extend_from_slice(z);
        let inp_len = z.len() as isize;
        let win = SYMBOL_LEN as isize;
        while self.sync_proc_ptr + win < inp_len {
            let at = (self.inp as isize + self.sync_proc_ptr) as usize;
            self.sync_process(at);
            if self.sync_ptr == self.symb_ptr {
                let s1 = self.sync_proc_ptr - self.proc_delay + (self.sync_symb_shift as isize - (self.symb_ptr * SYNC_STEP) as isize);
                let s2 = s1 + (SYMBOL_SEPAR / 2) as isize;
                let time_dist = s1 - self.data_proc_ptr;
                let (a, b) = ((self.inp as isize + s1) as usize, (self.inp as isize + s2) as usize);
                self.data_process(a, b, self.sync_freq_ofs, time_dist);
                self.data_proc_ptr = s1;
            }
            self.sync_proc_ptr += SYNC_STEP as isize;
        }
        self.sync_proc_ptr -= inp_len;
        self.data_proc_ptr -= inp_len;
    }

    fn do_correl_sum(&mut self, s: usize, s2: usize, k: usize) {
        let step = 2 * CARR_SEPAR;
        let d = CARRIERS * CARR_SEPAR;
        let c1 = &self.correl_norm[s];
        let c2 = &self.correl_norm[s2];
        let o2 = k + CARR_SEPAR;
        let mut sx = C::new(0.0, 0.0);
        let mut i = 0;
        while i < d {
            sx += c1[k + i] + c2[o2 + i];
            i += step;
        }
        let n = CARRIERS as f64;
        let mut aver = Vec::with_capacity(self.fit_len / step + 1);
        aver.push(sx / n);
        let mut i = 0;
        while i < self.fit_len - step {
            sx -= c1[k + i] + c2[o2 + i];
            // come fldigi: della portante che entra si somma la parte reale e si sottrae quella immaginaria. Sembra una svista, ma
            // sommandola il sincronismo non e' meglio su tutti i segnali (con certi rumori si aggancia piu' tardi) e cosi' il ricevitore
            // da' gli stessi caratteri di quello di fldigi (prova `nel_rumore_come_fldigi` dell'oracolo)
            sx.re += c1[k + i + d].re + c2[o2 + i + d].re;
            sx.im -= c1[k + i + d].im + c2[o2 + i + d].im;
            i += step;
            aver.push(sx / n);
        }
        for (j, a) in aver.into_iter().enumerate() {
            self.correl_aver[s][k + j * step] = a;
        }
    }

    fn sync_process(&mut self, at: usize) {
        self.sync_ptr = (self.sync_ptr + 1) & (SYMBOL_DIV - 1);
        for i in 0..SYMBOL_LEN {
            self.fft_a[i] = self.line[at + i] * self.window[i];
        }
        self.fft.process(&mut self.fft_a);
        let sp = self.sync_ptr;
        for i in 0..self.scan_len {
            let z = self.fft_a[(self.scan_first + i) & MASK];
            let p = z.norm_sqr();
            let a = p.sqrt();
            let d = if p > 0.0 { C::new((z.re * z.re - z.im * z.im) / a, 2.0 * z.re * z.im / a) } else { C::new(0.0, 0.0) };
            lowpass2(p, &mut self.power_mid[i], &mut self.power_out[i], self.wp);
            let prev = self.sync_pipe[sp][i] * self.sync_ph_corr[i];
            let correl = C::new(d.im * prev.im + d.re * prev.re, d.im * prev.re - d.re * prev.im);
            lowpass2c(correl, &mut self.correl_mid[sp][i], &mut self.correl_out[sp][i], self.w);
            self.sync_pipe[sp][i] = d;
        }
        if self.sync_ptr != (self.symb_ptr ^ 2) {
            return;
        }
        for s in 0..SYMBOL_DIV {
            for i in 0..self.scan_len {
                self.correl_norm[s][i] = if self.power_out[i] > 0.0 { self.correl_out[s][i] / self.power_out[i] } else { C::new(0.0, 0.0) };
            }
        }
        for s in 0..SYMBOL_DIV {
            let s2 = (s + SYMBOL_DIV / 2) & (SYMBOL_DIV - 1);
            for k in 0..2 * CARR_SEPAR {
                self.do_correl_sum(s, s2, k);
            }
        }
        for i in 0..self.fit_len {
            let a = &self.correl_aver;
            self.symb_fit[i] = C::new(a[0][i].norm() - a[2][i].norm(), a[1][i].norm() - a[3][i].norm());
        }
        let fit = |i: isize| -> C { if i >= 0 && (i as usize) < self.fit_len { self.symb_fit[i as usize] } else { C::new(0.0, 0.0) } };
        let (mut pmax, mut j) = (fit(2).norm_sqr(), 2isize);
        for i in 3..self.fit_len as isize - 2 {
            let p = fit(i).norm_sqr();
            if p > pmax {
                pmax = p;
                j = i;
            }
        }
        let cs = CARR_SEPAR as isize;
        let k = (j - self.symb_fit_pos) / cs;
        if k > 1 {
            j -= (k - 1) * cs;
        } else if k < -1 {
            j -= (k + 1) * cs;
        }
        self.symb_fit_pos = j;
        let (mut symb_time, mut freq_ofs);
        if pmax > 0.0 {
            let i_ = fit(j).re + 0.5 * (fit(j - 1).re + fit(j + 1).re);
            let q_ = fit(j).im + 0.5 * (fit(j - 1).im + fit(j + 1).im);
            symb_time = C::new(i_, q_);
            let mut symb_shift = q_.atan2(i_) / (2.0 * PI) * SYMBOL_DIV as f64;
            if symb_shift < 0.0 {
                symb_shift += SYMBOL_DIV as f64;
            }
            let sp = |c: C| i_ * c.re + q_ * c.im;
            let pi_ = sp(fit(j)) + 0.7 * sp(fit(j - 1)) + 0.7 * sp(fit(j + 1));
            let pq_ = 0.7 * sp(fit(j + 1)) - 0.7 * sp(fit(j - 1)) + 0.5 * sp(fit(j + 2)) - 0.5 * sp(fit(j - 2));
            freq_ofs = j as f64 + pq_.atan2(pi_) / (2.0 * PI / 8.0);
            let i = (freq_ofs + 0.5).floor() as isize;
            let s = (symb_shift.floor() as usize).min(SYMBOL_DIV - 1);
            let s2 = (s + 1) & (SYMBOL_DIV - 1);
            let w0 = s as f64 + 1.0 - symb_shift;
            let w1 = symb_shift - s as f64;
            let a = (0.5 * SYMBOL_LEN as f64) / SYMBOL_SEPAR as f64;
            let aver = |s: usize| -> C { if i >= 0 && (i as usize) < self.fit_len { self.correl_aver[s][i as usize] } else { C::new(0.0, 0.0) } };
            let iq = aver(s) * w0 + aver(s2) * w1;
            let f0 = i as f64 + iq.im.atan2(iq.re) / (2.0 * PI) * a - freq_ofs;
            let (fl, fu) = (f0 - a, f0 + a);
            freq_ofs += if fl.abs() < f0.abs() {
                if fu.abs() < fl.abs() { fu } else { fl }
            } else if fu.abs() < f0.abs() {
                fu
            } else {
                f0
            };
        } else {
            symb_time = C::new(0.0, 0.0);
            freq_ofs = 0.0;
        }
        let cs = CARR_SEPAR as f64;
        if self.locked {
            if symb_time.re * self.aver_symb.re + symb_time.im * self.aver_symb.im < 0.0 {
                symb_time = -symb_time;
                freq_ofs -= cs;
            }
            let a = 2.0 * cs;
            let k = ((freq_ofs - self.aver_freq) / a + 0.5).floor();
            freq_ofs -= k * a;
            let a = (0.5 * SYMBOL_LEN as f64) / SYMBOL_SEPAR as f64;
            let f0 = freq_ofs - self.aver_freq;
            freq_ofs += wrap_step(f0, a);
        } else {
            let prev = self.symb_pipe[self.track_ptr];
            if symb_time.re * prev.re + symb_time.im * prev.im < 0.0 {
                symb_time = -symb_time;
                freq_ofs -= cs;
            }
            let a = 2.0 * cs;
            let k = (freq_ofs / a + 0.5).floor();
            freq_ofs -= k * a;
            let f0 = freq_ofs - self.freq_pipe[self.track_ptr];
            freq_ofs += wrap_step(f0, a);
        }
        self.track_ptr = (self.track_ptr + 1) % self.integ;
        self.symb_pipe[self.track_ptr] = symb_time;
        self.freq_pipe[self.track_ptr] = freq_ofs;
        let (aver_symb, _) = sel_fit_aver_c(&self.symb_pipe, 3.0, 4);
        let (aver_freq, freq_dev) = sel_fit_aver(&self.freq_pipe, 2.5, 4);
        self.aver_symb = aver_symb;
        self.aver_freq = aver_freq;
        let conf = aver_symb.norm();
        self.sync_freq_ofs = aver_freq;
        if conf > 0.0 {
            let ph = aver_symb.im.atan2(aver_symb.re) / (2.0 * PI);
            let mut shift = ph * SYMBOL_SEPAR as f64;
            if shift < 0.0 {
                shift += SYMBOL_SEPAR as f64;
            }
            let mut sp = (ph * SYMBOL_DIV as f64).floor() as isize;
            if sp < 0 {
                sp += SYMBOL_DIV as isize;
            }
            self.symb_ptr = (sp as usize) & (SYMBOL_DIV - 1);
            self.sync_symb_shift = shift;
        }
        if self.locked {
            if conf < self.hold_thres || freq_dev > 0.250 {
                self.locked = false;
            }
        } else if conf > self.lock_thres && freq_dev < 0.125 {
            self.locked = true;
        }
    }

    fn data_process(&mut self, even: usize, odd: usize, freq_ofs: f64, time_dist: isize) {
        let p = -2.0 * PI * freq_ofs / SYMBOL_LEN as f64;
        let freq = C::new(p.cos(), p.sin());
        let mut phas = C::new(1.0, 0.0);
        for i in 0..SYMBOL_LEN {
            self.fft_a[i] = self.line[even + i] * phas * self.window[i];
            self.fft_b[i] = self.line[odd + i] * phas * self.window[i];
            phas *= freq;
        }
        self.fft.process(&mut self.fft_a);
        self.fft.process(&mut self.fft_b);
        let td = time_dist as i64;
        let incr = (td * CARR_SEPAR as i64).rem_euclid(SYMBOL_LEN as i64) as usize;
        let mut p = (td * self.data_scan_first).rem_euclid(SYMBOL_LEN as i64) as usize;
        let mut c = self.data_scan_first.rem_euclid(SYMBOL_LEN as i64) as usize;
        for i in 0..self.data_scan_len {
            let z = if i & 1 == 0 { self.fft_a[c] } else { self.fft_b[c] };
            let r = self.ref_slice[i] * self.tw[p];
            self.data_vect[i] = z * r.conj();
            lowpass2(z.norm_sqr(), &mut self.data_pwr_mid[i], &mut self.data_pwr_out[i], self.dw);
            self.ref_slice[i] = z;
            c = (c + CARR_SEPAR) & MASK;
            p = (p + incr) & MASK;
        }
        let p = -(time_dist as f64) * 2.0 * PI * freq_ofs / SYMBOL_LEN as f64;
        let freq = C::new(p.cos(), p.sin());
        let row = &mut self.data_pipe[self.data_pipe_ptr];
        for i in 0..self.data_scan_len {
            let f = self.data_vect[i] * freq;
            self.data_vect[i] = row[i];
            row[i] = f;
        }
        self.data_pipe_ptr = (self.data_pipe_ptr + 1) % self.data_pipe.len();
        for i in 0..self.data_scan_len {
            self.data_phase[i] = if self.data_pwr_out[i] > 0.0 { (self.data_vect[i].re / self.data_pwr_out[i]).clamp(-1.0, 1.0) } else { 0.0 };
        }
        self.decoder.process(&self.data_phase);
        self.snr = self.decoder.snr;
        let c = self.decoder.output;
        let pos = (self.decoder.carr_ofs + SCAN_MARGIN as i32) as usize;
        // anche il rapporto del simbolo appena decodificato (non solo la media): a fine trasmissione la media scende piano e lascerebbe
        // passare qualche carattere di rumore
        let raw_ok = self.decoder.raw[pos] >= RAW_MIN;
        // i dati passano 16 (o 32) simboli dopo il sincronismo e lo sgancio arriva in ritardo di altrettanto: dopo
        // senza aggancio per un'integrazione intera il testo e' finito e quello che segue e' rumore (o la coda dell'interleaving)
        if self.locked {
            self.unlocked = 0;
        } else {
            self.unlocked = self.unlocked.saturating_add(1);
        }
        let hold_ok = self.unlocked <= self.integ;
        // appena agganciato escono i caratteri ricevuti a meta' (il riempimento dell'interleaving di chi trasmette, solo in parte in
        // onda): il testo vero arriva dopo un interleaving, il ritardo dei dati (un'integrazione) e quello del decodificatore (mezza),
        // meno i simboli che il sincronismo ha impiegato ad agganciarsi (si contano 8). Con un segnale debolissimo che parte subito col
        // testo (fldigi) si possono perdere i primi caratteri; il nostro trasmettitore manda prima 16 simboli di attesa.
        if self.unlocked > self.integ {
            self.since_lock = 0;
        } else {
            self.since_lock = self.since_lock.saturating_add(1);
        }
        let start_ok = self.since_lock > self.v.interleave() + self.integ * 3 / 2 - 8;
        let open = self.squelch_db <= SQUELCH_OFF || (start_ok && hold_ok && raw_ok && self.snr_db() >= self.squelch_db);
        if open && ((b' '..127).contains(&c) || c == b'\n') {
            self.out.push(c);
            self.last_freq = self.measured_frequency();
        }
    }

    /// I caratteri decodificati dall'ultima volta.
    pub fn take(&mut self) -> Vec<u8> {
        std::mem::take(&mut self.out)
    }

    /// Agganciato al segnale (sincronismo di tempo e frequenza).
    pub fn locked(&self) -> bool {
        self.locked
    }

    /// La frequenza del centro del segnale ricevuto (Hz), misurata all'ultimo carattere decodificato (prima: quella impostata).
    pub fn frequency(&self) -> f64 {
        self.last_freq
    }

    fn measured_frequency(&self) -> f64 {
        let ofs = (self.sync_freq_ofs + CARR_SEPAR as f64 * f64::from(self.decoder.carr_ofs)) * (RATE / self.v.decimate() as f64) / SYMBOL_LEN as f64;
        if self.reverse { self.centre_hz - ofs } else { self.centre_hz + ofs }
    }

    /// Il rapporto segnale/rumore del decodificatore (dB, 10 log del rapporto di fldigi).
    pub fn snr_db(&self) -> f32 {
        (10.0 * self.snr.max(1e-3).log10()) as f32
    }

    /// Livello per lo squelch e se c'e' segnale.
    pub fn signal(&self) -> (f32, bool) {
        (self.snr_db(), self.snr_db() >= SQUELCH_DB)
    }

    pub fn threshold_db() -> f32 {
        SQUELCH_DB
    }

    /// Cambia la soglia dello squelch (dB). Da [`SQUELCH_OFF`] in giu' lo squelch e' spento del tutto (anche aggancio e inizio), come
    /// fldigi senza squelch: esce ogni carattere decodificato, rumore compreso (per le misure).
    pub fn set_squelch_db(&mut self, db: f32) {
        self.squelch_db = db;
    }

    /// Integrazione lunga (32 simboli) o corta (16).
    pub fn long_integ(&self) -> bool {
        self.integ == INTEG_LONG
    }
}

/// La correzione del giro dell'autocorrelatore: aggiunge -a, 0 o +a, quella che porta `f0` piu' vicino a zero.
fn wrap_step(f0: f64, a: f64) -> f64 {
    let (fl, fu) = (f0 - a, f0 + a);
    if fl.abs() < f0.abs() {
        if fu.abs() < fl.abs() { a } else { -a }
    } else if fu.abs() < f0.abs() {
        a
    } else {
        0.0
    }
}

/// MT63 dietro il decimatore 12 -> 8 kHz del programma.
pub struct Mt63Link {
    rx: Mt63Rx,
    decimator: crate::resample_8k::Decimator,
    buf: Vec<f32>,
}

impl Mt63Link {
    pub fn new(v: Mt63Variant, centre_hz: f32, reverse: bool, long_integ: bool) -> Self {
        Mt63Link { rx: Mt63Rx::new(v, f64::from(centre_hz), reverse, long_integ), decimator: crate::resample_8k::Decimator::new(), buf: Vec::new() }
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
}

/// La meta' della forma del simbolo dell'MT63 originale (punti da 0 a 256; la seconda meta' e' lo specchio), dalla tabella `symbol.dat`
/// di fldigi.
const SHAPE_HALF: [f64; 257] = [
    0.0,
    0.00000665,
    0.00002657,
    0.00005975,
    0.00010613,
    0.00016562,
    0.00023810,
    0.00032341,
    0.00042134,
    0.00053162,
    0.00065389,
    0.00078773,
    0.00093261,
    0.00108789,
    0.00125283,
    0.00142653,
    0.00160798,
    0.00179599,
    0.00198926,
    0.00218628,
    0.00238542,
    0.00258487,
    0.00278264,
    0.00297662,
    0.00316452,
    0.00334394,
    0.00351232,
    0.00366701,
    0.00380526,
    0.00392424,
    0.00402109,
    0.00409288,
    0.00413671,
    0.00414969,
    0.00412898,
    0.00407182,
    0.00397555,
    0.00383764,
    0.00365574,
    0.00342767,
    0.00315145,
    0.00282534,
    0.00244787,
    0.00201781,
    0.00153424,
    0.00099653,
    0.00040435,
    -0.00024231,
    -0.00094314,
    -0.00169753,
    -0.00250453,
    -0.00336293,
    -0.00427118,
    -0.00522749,
    -0.00622977,
    -0.00727569,
    -0.00836272,
    -0.00948809,
    -0.01064886,
    -0.01184193,
    -0.01306405,
    -0.01431189,
    -0.01558198,
    -0.01687083,
    -0.01817486,
    -0.01949051,
    -0.02081416,
    -0.02214223,
    -0.02347113,
    -0.02479733,
    -0.02611728,
    -0.02742752,
    -0.02872457,
    -0.03000504,
    -0.03126551,
    -0.03250262,
    -0.03371298,
    -0.03489320,
    -0.03603988,
    -0.03714954,
    -0.03821868,
    -0.03924367,
    -0.04022079,
    -0.04114620,
    -0.04201589,
    -0.04282570,
    -0.04357126,
    -0.04424801,
    -0.04485118,
    -0.04537575,
    -0.04581648,
    -0.04616787,
    -0.04642421,
    -0.04657955,
    -0.04662769,
    -0.04656225,
    -0.04637665,
    -0.04606414,
    -0.04561786,
    -0.04503082,
    -0.04429599,
    -0.04340631,
    -0.04235475,
    -0.04113436,
    -0.03973834,
    -0.03816006,
    -0.03639316,
    -0.03443155,
    -0.03226956,
    -0.02990192,
    -0.02732385,
    -0.02453112,
    -0.02152012,
    -0.01828789,
    -0.01483216,
    -0.01115146,
    -0.00724508,
    -0.00311317,
    0.00124328,
    0.00582236,
    0.01062127,
    0.01563627,
    0.02086273,
    0.02629504,
    0.03192674,
    0.03775043,
    0.04375787,
    0.04993995,
    0.05628681,
    0.06278780,
    0.06943159,
    0.07620621,
    0.08309914,
    0.09009732,
    0.09718730,
    0.10435526,
    0.11158715,
    0.11886870,
    0.12618560,
    0.13352351,
    0.14086819,
    0.14820561,
    0.15552198,
    0.16280389,
    0.17003841,
    0.17721311,
    0.18431620,
    0.19133661,
    0.19826401,
    0.20508896,
    0.21180289,
    0.21839823,
    0.22486845,
    0.23120806,
    0.23741270,
    0.24347919,
    0.24940549,
    0.25519079,
    0.26083547,
    0.26634116,
    0.27171067,
    0.27694807,
    0.28205857,
    0.28704860,
    0.29192571,
    0.29669855,
    0.30137684,
    0.30597130,
    0.31049362,
    0.31495636,
    0.31937292,
    0.32375741,
    0.32812465,
    0.33249001,
    0.33686936,
    0.34127898,
    0.34573545,
    0.35025554,
    0.35485613,
    0.35955412,
    0.36436627,
    0.36930915,
    0.37439902,
    0.37965170,
    0.38508250,
    0.39070609,
    0.39653642,
    0.40258662,
    0.40886890,
    0.41539446,
    0.42217341,
    0.42921470,
    0.43652603,
    0.44411383,
    0.45198311,
    0.46013753,
    0.46857925,
    0.47730896,
    0.48632585,
    0.49562756,
    0.50521021,
    0.51506840,
    0.52519520,
    0.53558220,
    0.54621950,
    0.55709582,
    0.56819849,
    0.57951351,
    0.59102568,
    0.60271860,
    0.61457478,
    0.62657574,
    0.63870210,
    0.65093366,
    0.66324951,
    0.67562817,
    0.68804763,
    0.70048553,
    0.71291922,
    0.72532590,
    0.73768272,
    0.74996688,
    0.76215572,
    0.77422687,
    0.78615828,
    0.79792836,
    0.80951602,
    0.82090079,
    0.83206287,
    0.84298315,
    0.85364335,
    0.86402598,
    0.87411443,
    0.88389296,
    0.89334677,
    0.90246195,
    0.91122553,
    0.91962547,
    0.92765062,
    0.93529073,
    0.94253642,
    0.94937916,
    0.95581122,
    0.96182562,
    0.96741616,
    0.97257728,
    0.97730410,
    0.98159233,
    0.98543825,
    0.98883864,
    0.99179079,
    0.99429241,
    0.99634163,
    0.99793696,
    0.99907728,
    0.99976178,
    0.99999000,
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

    fn rumore(a: &mut [f32], sigma: f64, seed: u64) {
        let mut x = seed.wrapping_mul(2862933555777941757).wrapping_add(3037000493);
        let mut u = || {
            x = x.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
            ((x >> 11) as f64 + 0.5) / (1u64 << 53) as f64
        };
        for v in a.iter_mut() {
            let g = (-2.0 * u().ln()).sqrt() * (2.0 * PI * u()).cos();
            *v += (g * sigma) as f32;
        }
    }

    fn ricevi(v: Mt63Variant, audio: &[f32], centre: f64, reverse: bool) -> String {
        let mut rx = Mt63Rx::new(v, centre, reverse, false);
        for b in audio.chunks(700) {
            rx.process(b);
        }
        String::from_utf8_lossy(&rx.take()).into_owned()
    }

    fn segnale(v: Mt63Variant, text: &str, freq: f64, reverse: bool) -> Vec<f32> {
        let mut a = vec![0.0f32; 3000];
        a.extend(Mt63Tx::new(v, freq, 0.5, reverse).transmit(text.as_bytes()));
        a.extend(vec![0.0f32; 8 * v.symbol_samples() * 40]);
        a
    }

    const TESTO: &str = "CQ CQ DE IU3ABC IU3ABC IO91WM k";

    /// Anello chiuso senza rumore, tutte le varianti, USB e LSB.
    #[test]
    fn anello_chiuso() {
        for v in Mt63Variant::ALL {
            for rev in [false, true] {
                let got = ricevi(v, &segnale(v, TESTO, 1500.0, rev), 1500.0, rev);
                assert_eq!(got.trim(), TESTO, "{v:?} rev {rev}");
            }
        }
    }

    /// Integrazione lunga (32 simboli): stesso testo, anche in LSB.
    #[test]
    fn integrazione_lunga() {
        for v in [Mt63Variant::Mt63_500l, Mt63Variant::Mt63_2000s] {
            for rev in [false, true] {
                let mut a = segnale(v, TESTO, 1400.0, rev);
                a.extend(vec![0.0; 16 * v.symbol_samples() * 2]);
                rumore(&mut a, 0.1, 9);
                let mut rx = Mt63Rx::new(v, 1400.0, rev, true);
                assert!(rx.long_integ());
                rx.process(&a);
                assert_eq!(String::from_utf8_lossy(&rx.take()).trim(), TESTO, "{v:?} rev {rev}");
            }
        }
    }

    /// Con rumore: tutte le varianti a un livello in cui il testo arriva intero.
    #[test]
    fn nel_rumore() {
        for v in Mt63Variant::ALL {
            // stesso rapporto segnale/rumore per hertz: il rumore cala con la radice della banda
            let mut a = segnale(v, TESTO, 1500.0, false);
            rumore(&mut a, 0.2 * (500.0 / v.bandwidth_hz()).sqrt(), 3);
            assert_eq!(ricevi(v, &a, 1500.0, false).trim(), TESTO, "{v:?}");
        }
    }

    /// Nel solo rumore (un minuto) lo squelch non fa uscire niente.
    #[test]
    fn nel_solo_rumore_tace() {
        for v in [Mt63Variant::Mt63_500s, Mt63Variant::Mt63_1000l, Mt63Variant::Mt63_2000s] {
            let mut a = vec![0.0f32; 8000 * 60];
            rumore(&mut a, 0.3, 11);
            let got = ricevi(v, &a, 1500.0, false);
            assert!(got.is_empty(), "{v:?}: {got:?}");
        }
    }

    /// Ricampiona come una scheda audio con l'orologio sbagliato di `ppm` (interpolazione lineare).
    fn orologio(a: &[f32], ppm: f64) -> Vec<f32> {
        let r = 1.0 + ppm * 1e-6;
        let n = (a.len() as f64 / r) as usize - 1;
        (0..n)
            .map(|i| {
                let t = i as f64 * r;
                let k = t as usize;
                let f = (t - k as f64) as f32;
                a[k] * (1.0 - f) + a[(k + 1).min(a.len() - 1)] * f
            })
            .collect()
    }

    /// Scostamento di frequenza (fino a tre portanti e mezza, fra le righe) e di orologio (+/-300 ppm): il testo arriva intero e la
    /// frequenza misurata e' quella vera.
    #[test]
    fn scostamento_di_frequenza_e_di_orologio() {
        for v in Mt63Variant::ALL {
            let step = v.bandwidth_hz() / 64.0;
            for k in [-3.4, -1.3, 0.45, 2.7, 3.5] {
                let df = k * step;
                let mut a = segnale(v, TESTO, 1500.0 + df, false);
                rumore(&mut a, 0.05, 5);
                let mut rx = Mt63Rx::new(v, 1500.0, false, false);
                let mut got = Vec::new();
                for b in a.chunks(700) {
                    rx.process(b);
                    let new = rx.take();
                    if !new.is_empty() {
                        // la misura del sincronismo ha la grana di una riga della FFT (un quarto di portante), piu' o meno mezza
                        assert!((rx.frequency() - 1500.0 - df).abs() < 0.4 * step, "{v:?} {df} Hz: misurata {}", rx.frequency());
                    }
                    got.extend(new);
                }
                assert_eq!(String::from_utf8_lossy(&got).trim(), TESTO, "{v:?} scostamento {df} Hz");
            }
            for ppm in [-300.0, 300.0] {
                let mut a = orologio(&segnale(v, TESTO, 1500.0, false), ppm);
                rumore(&mut a, 0.05, 6);
                assert_eq!(ricevi(v, &a, 1500.0, false).trim(), TESTO, "{v:?} orologio {ppm} ppm");
            }
        }
    }

    /// Solo misura (`--ignored --nocapture misura_limiti`, in release): fin dove reggono orologio e frequenza, e quanto costa il ricevitore.
    #[test]
    #[ignore]
    fn misura_limiti() {
        for v in [Mt63Variant::Mt63_500s, Mt63Variant::Mt63_1000l, Mt63Variant::Mt63_2000l] {
            let step = v.bandwidth_hz() / 64.0;
            let base = segnale(v, TESTO, 1500.0, false);
            let ok_ppm: Vec<f64> = [500.0, 1000.0, 2000.0, 4000.0, 8000.0]
                .into_iter()
                .filter(|&ppm| [ppm, -ppm].iter().all(|&p| ricevi(v, &orologio(&base, p), 1500.0, false).trim() == TESTO))
                .collect();
            let ok_k: Vec<f64> = [4.5, 5.5, 6.5, 7.5, 8.5, 9.5]
                .into_iter()
                .filter(|&k| [k, -k].iter().all(|&kk| ricevi(v, &segnale(v, TESTO, 1500.0 + kk * step, false), 1500.0, false).trim() == TESTO))
                .collect();
            let mut a = vec![0.0f32; 8000 * 60];
            rumore(&mut a, 0.1, 1);
            let t = std::time::Instant::now();
            let _ = ricevi(v, &a, 1500.0, false);
            eprintln!("{v:?}: orologio ok a +/- {ok_ppm:?} ppm, scostamento ok a +/- {ok_k:?} portanti, 60 s di audio in {:?}", t.elapsed());
        }
    }

    /// Il ricevitore dietro il decimatore a 12 kHz e il trasmettitore portato a 12 kHz dall'interpolatore.
    #[test]
    fn a_dodici_khz() {
        let v = Mt63Variant::Mt63_1000s;
        let a8 = segnale(v, TESTO, 1200.0, true);
        let mut up = crate::resample_8k::Interpolator::new();
        let mut a12 = Vec::new();
        up.process(&a8, &mut a12);
        let mut link = Mt63Link::new(v, 1200.0, true, false);
        let got: String = a12.chunks(1200).map(|b| link.process(b)).collect();
        assert_eq!(got.trim(), TESTO);
        assert!((link.frequency() - 1200.0).abs() < 4.0, "{}", link.frequency());
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
