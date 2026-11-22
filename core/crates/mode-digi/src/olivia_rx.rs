//! Il ricevitore di Olivia e Contestia (MFSK a segnale debole di Pawel Jalocha), riscritto dalla logica della libreria usata da fldigi
//! (GPL, `terze-parti/fldigi/src/include/jalocha/pj_mfsk.h`).
//!
//! La catena, nell'ordine:
//! 1. un filtro passabasso FIR a 16 prese (il convertitore di frequenza della libreria, che lavora anche a rapporto 1);
//! 2. l'elaboratore d'ingresso: finestre lunghe 16 simboli sovrapposte a meta', spettro equalizzato (picchi limitati e energia media
//!    portata a 1, cosi' le portanti fisse e i disturbi coerenti si attenuano), ritorno nel tempo e limitazione dei picchi d'uscita;
//! 3. il demodulatore: due FFT per simbolo (a un quarto di simbolo l'una dall'altra) e l'energia di ogni bin vicino ai toni;
//! 4. per ogni mezzo simbolo e ogni spostamento di frequenza entro il margine, un decodificatore a decisione morbida (Walsh-Hadamard,
//!    rimescolamento, codice di Gray, rotazione dei bit) che legge l'ultimo blocco FEC;
//! 5. il sincronismo: integratori `LowPass3` del segnale e del rumore per ogni fase del blocco e ogni spostamento; si sceglie il migliore
//!    e, mezzo blocco dopo, se il rapporto segnale/rumore supera la soglia si mandano in uscita i caratteri del blocco migliore.

use rustfft::num_complex::Complex;

use crate::olivia::{Params, RATE_HZ, Variant};

type C64 = Complex<f64>;

/// Mezzi simboli (fette di FFT) per simbolo.
const SLICES_PER_SYMBOL: usize = 2;
/// Distanza fra due toni, in bin della FFT di un simbolo.
const CARRIER_SEPAR: usize = 2;

/// Dal codice di Gray al binario (8 bit).
fn binary_code(mut gray: u8) -> u8 {
    gray ^= gray >> 4;
    gray ^= gray >> 2;
    gray ^= gray >> 1;
    gray
}

/// Trasformata di Hadamard diretta sul posto, con lo stesso ordine delle farfalle della libreria.
fn fht(data: &mut [f64]) {
    let len = data.len();
    let mut step = 1;
    while step < len {
        let mut ptr = 0;
        while ptr < len {
            for i in ptr..ptr + step {
                let (a, b) = (data[i], data[i + step]);
                data[i] = b + a;
                data[i + step] = b - a;
            }
            ptr += 2 * step;
        }
        step *= 2;
    }
}

/// L'integratore passabasso del terzo ordine della libreria (peso = 1 / tempo di salita, retroazione 0,1).
#[derive(Clone, Copy, Debug, Default)]
struct LowPass3 {
    out1: f64,
    out2: f64,
    output: f64,
}

impl LowPass3 {
    fn process(&mut self, input: f64, weight: f64) {
        let w = 2.0 * weight;
        let d1 = (input - self.out1) * w;
        let d12 = (self.out1 - self.out2) * w;
        let d23 = (self.out2 - self.output) * w;
        self.out1 += d1;
        self.out2 += d12;
        self.output += d23;
        self.out2 += d23 * 0.1;
    }
}

/// Il convertitore di frequenza della libreria a rapporto 1: resta un FIR passabasso (sinc a 3/8 della frequenza di campionamento con
/// finestra di Blackman-Harris), 16 prese, che ritarda di circa 8 campioni.
struct InputFilter {
    taps: [f64; 16],
    history: [f64; 16],
    ptr: usize,
}

impl InputFilter {
    fn new() -> InputFilter {
        let filter_len = 256.0;
        let mut taps = [0.0; 16];
        for (k, tap) in taps.iter_mut().enumerate() {
            let idx = (15 + 16 * k) as f64;
            let phase = std::f64::consts::PI * (2.0 * idx - filter_len) / filter_len;
            let window = 0.35875 + 0.48829 * phase.cos() + 0.14128 * (2.0 * phase).cos() + 0.01168 * (3.0 * phase).cos();
            let p = phase * (3.0 / 8.0) * 16.0;
            let sinc = if p != 0.0 { p.sin() / p } else { 1.0 };
            *tap = window * sinc;
        }
        InputFilter { taps, history: [0.0; 16], ptr: 0 }
    }

    fn process(&mut self, input: f64) -> f64 {
        self.history[self.ptr] = input;
        self.ptr = (self.ptr + 1) & 15;
        let mut sum = 0.0;
        for (k, tap) in self.taps.iter().enumerate() {
            sum += self.history[(self.ptr + k) & 15] * tap;
        }
        sum
    }
}

/// FFT complessa radix-2 diretta (esponente negativo, senza normalizzazione), sul posto: riordino a bit invertiti e poi le farfalle,
/// come la `r2FFT` della libreria. `len` e' una potenza di due.
struct ForwardFft {
    len: usize,
    bit_rev: Vec<usize>,
    /// `e^{-i 2 pi k / len}` per `k` da 0 a `len / 2 - 1`.
    twiddle: Vec<C64>,
}

impl ForwardFft {
    fn new(len: usize) -> ForwardFft {
        let bits = len.trailing_zeros();
        let bit_rev = (0..len).map(|i| if bits == 0 { 0 } else { i.reverse_bits() >> (usize::BITS - bits) }).collect();
        let twiddle = (0..len / 2)
            .map(|k| {
                let ph = -2.0 * std::f64::consts::PI * k as f64 / len as f64;
                C64::new(ph.cos(), ph.sin())
            })
            .collect();
        ForwardFft { len, bit_rev, twiddle }
    }

    fn process(&mut self, buf: &mut [C64]) {
        let n = self.len;
        for i in 0..n {
            let j = self.bit_rev[i];
            if j > i {
                buf.swap(i, j);
            }
        }
        let mut size = 2;
        while size <= n {
            let half = size / 2;
            let step = n / size;
            for start in (0..n).step_by(size) {
                for k in 0..half {
                    let w = self.twiddle[k * step];
                    let a = buf[start + k];
                    let b = buf[start + k + half] * w;
                    buf[start + k] = a + b;
                    buf[start + k + half] = a - b;
                }
            }
            size *= 2;
        }
    }
}

/// Separa la FFT di due sequenze reali (una nella parte reale, una nell'immaginaria) nei due mezzi spettri; nel bin 0 la parte
/// immaginaria porta il bin di Nyquist.
fn separ_two_reals(buf: &[C64], out0: &mut [C64], out1: &mut [C64]) {
    let size = buf.len();
    let half = size / 2;
    out0[0] = C64::new(buf[0].re, buf[half].re);
    out1[0] = C64::new(buf[0].im, buf[half].im);
    for i in 1..half {
        let (a, b) = (buf[i], buf[size - i]);
        out0[i] = C64::new(a.re + b.re, a.im - b.im);
        out1[i] = C64::new(a.im + b.im, b.re - a.re);
    }
}

/// L'opposto di `separ_two_reals`, gia' predisposto perche' la FFT diretta faccia da inversa (il risultato esce moltiplicato per 2).
fn join_two_reals(inp0: &[C64], inp1: &[C64], buf: &mut [C64]) {
    let size = buf.len();
    let half = size / 2;
    buf[0] = C64::new(2.0 * inp0[0].re, 2.0 * inp1[0].re);
    for i in 1..half {
        let (a, b) = (inp0[i], inp1[i]);
        buf[i] = C64::new(a.re + b.im, b.re - a.im);
        buf[size - i] = C64::new(a.re - b.im, a.im + b.re);
    }
    buf[half] = C64::new(2.0 * inp0[0].im, 2.0 * inp1[0].im);
}

/// Un filtro a media mobile (somma corrente su `len` valori), come il `BoxFilter` della libreria.
struct BoxFilter {
    tap: Vec<f64>,
    ptr: usize,
    output: f64,
}

impl BoxFilter {
    fn new(len: usize) -> BoxFilter {
        BoxFilter { tap: vec![0.0; len.max(1)], ptr: 0, output: 0.0 }
    }

    fn process(&mut self, input: f64) {
        self.output += input - self.tap[self.ptr];
        self.tap[self.ptr] = input;
        self.ptr += 1;
        if self.ptr >= self.tap.len() {
            self.ptr = 0;
        }
    }
}

/// Limita i picchi dello spettro: un bin che supera `threshold` volte la media dei vicini (su `len` bin) si riduce alla soglia.
fn limit_spectra_peaks(spectra: &mut [C64], energy: &mut [f64], len: usize, threshold: f64) {
    let len = len.max(1);
    let mut filter = BoxFilter::new(len);
    let max_freq = 3 * (energy.len() / 4);
    let mut freq = 0;
    while freq < len.min(energy.len()) {
        filter.process(energy[freq]);
        freq += 1;
    }
    let mut idx = len / 2;
    while freq < max_freq {
        filter.process(energy[freq]);
        let signal = energy[idx];
        let limit = (filter.output / len as f64) * threshold;
        if signal > limit {
            spectra[idx] *= (limit / signal).sqrt();
            energy[idx] = limit;
        }
        freq += 1;
        idx += 1;
    }
}

/// Sostituisce l'energia di ogni bin con la media mobile su `len` bin (i bordi prendono il primo e l'ultimo valore della media).
fn average_energy(energy: &mut [f64], len: usize) {
    let len = len.max(1);
    let mut filter = BoxFilter::new(len);
    let spectra_len = energy.len();
    let max_freq = 3 * (spectra_len / 4);
    let scale = 1.0 / len as f64;
    let mut freq = 0;
    while freq < len.min(spectra_len) {
        filter.process(energy[freq]);
        freq += 1;
    }
    let mut idx = 0;
    while idx < (len / 2).min(spectra_len) {
        energy[idx] = filter.output * scale;
        idx += 1;
    }
    while freq < max_freq {
        filter.process(energy[freq]);
        energy[idx] = filter.output * scale;
        freq += 1;
        idx += 1;
    }
    while idx < spectra_len {
        energy[idx] = filter.output * scale;
        idx += 1;
    }
}

/// L'elaboratore d'ingresso: equalizza lo spettro e toglie i disturbi coerenti, su finestre di `window_len` campioni sovrapposte a meta'.
struct InputProcessor {
    window_len: usize,
    mask: usize,
    slice_separ: usize,
    inp_tap: Vec<f64>,
    inp_ptr: usize,
    out_tap: Vec<f64>,
    out_ptr: usize,
    window: Vec<f64>,
    fft: ForwardFft,
    buf: Vec<C64>,
    spectra: [Vec<C64>; 2],
    energy: Vec<f64>,
    output: Vec<f64>,
}

impl InputProcessor {
    fn new(window_len: usize) -> InputProcessor {
        let scale = 2.0 / window_len as f64;
        let window = (0..window_len)
            .map(|i| scale * (1.0 - (2.0 * std::f64::consts::PI * i as f64 / window_len as f64).cos()).max(0.0).sqrt())
            .collect();
        let spectra_len = window_len / 2;
        InputProcessor {
            window_len,
            mask: window_len - 1,
            slice_separ: window_len / 2,
            inp_tap: vec![0.0; window_len],
            inp_ptr: 0,
            out_tap: vec![0.0; window_len],
            out_ptr: 0,
            window,
            fft: ForwardFft::new(window_len),
            buf: vec![C64::default(); window_len],
            spectra: [vec![C64::default(); spectra_len], vec![C64::default(); spectra_len]],
            energy: vec![0.0; spectra_len],
            output: vec![0.0; window_len],
        }
    }

    fn feed(&mut self, input: &[f64]) {
        for &x in input {
            self.inp_tap[self.inp_ptr] = x;
            self.inp_ptr = (self.inp_ptr + 1) & self.mask;
        }
    }

    fn process_spectra(spectra: &mut [C64], energy: &mut [f64], window_len: usize) {
        for (e, s) in energy.iter_mut().zip(spectra.iter()) {
            *e = s.norm_sqr();
        }
        for _ in 0..3 {
            limit_spectra_peaks(spectra, energy, window_len / 64, 4.0);
        }
        average_energy(energy, window_len / 96);
        average_energy(energy, window_len / 64);
        for (s, e) in spectra.iter_mut().zip(energy.iter()) {
            if *e > 0.0 {
                *s *= 1.0 / e.sqrt();
            }
        }
    }

    fn limit_output_peaks(&mut self, threshold: f64) {
        let rms = (self.output.iter().map(|v| v * v).sum::<f64>() / self.window_len as f64).sqrt();
        let limit = rms * threshold;
        for v in self.output.iter_mut() {
            *v = v.clamp(-limit, limit);
        }
    }

    /// Elabora `window_len` campioni d'ingresso; il risultato (altrettanti campioni) e' in `output`.
    fn process(&mut self, input: &[f64]) {
        let (first, second) = input.split_at(self.slice_separ);
        self.feed(first);
        for t in 0..self.window_len {
            self.buf[t] = C64::new(self.inp_tap[self.inp_ptr] * self.window[t], 0.0);
            self.inp_ptr = (self.inp_ptr + 1) & self.mask;
        }
        self.feed(second);
        for t in 0..self.window_len {
            self.buf[t].im = self.inp_tap[self.inp_ptr] * self.window[t];
            self.inp_ptr = (self.inp_ptr + 1) & self.mask;
        }
        self.fft.process(&mut self.buf);
        {
            let [s0, s1] = &mut self.spectra;
            separ_two_reals(&self.buf, s0, s1);
            Self::process_spectra(s0, &mut self.energy, self.window_len);
            Self::process_spectra(s1, &mut self.energy, self.window_len);
            join_two_reals(s0, s1, &mut self.buf);
        }
        self.fft.process(&mut self.buf);
        for half in 0..2 {
            for t in 0..self.window_len {
                let v = if half == 0 { self.buf[t].re } else { self.buf[t].im };
                self.out_tap[self.out_ptr] += v * self.window[t];
                self.out_ptr = (self.out_ptr + 1) & self.mask;
            }
            for i in 0..self.slice_separ {
                self.output[half * self.slice_separ + i] = self.out_tap[self.out_ptr];
                self.out_tap[self.out_ptr] = 0.0;
                self.out_ptr = (self.out_ptr + 1) & self.mask;
            }
        }
        self.limit_output_peaks(2.5);
        self.limit_output_peaks(2.5);
    }
}

/// Il demodulatore: due FFT per simbolo con finestra a coseno rialzato e l'energia dei bin attorno ai toni (margine compreso).
struct Demodulator {
    symbol_len: usize,
    mask: usize,
    symbol_separ2: usize,
    inp_tap: Vec<f64>,
    inp_ptr: usize,
    shape: Vec<f64>,
    fft: ForwardFft,
    buf: Vec<C64>,
    spectra: [Vec<C64>; 2],
    energy: [Vec<f64>; 2],
    first_bin: usize,
    decode_margin: usize,
    carriers: usize,
    bits: usize,
    /// Il simbolo (binario) portato da ogni tono.
    tone_symbol: Vec<u8>,
}

impl Demodulator {
    fn new(symbol_len: usize, first_carrier: usize, decode_margin: usize, bits: usize) -> Demodulator {
        let carriers = 1usize << bits;
        let decode_width = carriers * CARRIER_SEPAR - 1 + 2 * decode_margin;
        let shape = (0..symbol_len)
            .map(|t| (1.0 - (2.0 * std::f64::consts::PI * t as f64 / symbol_len as f64).cos()) / symbol_len as f64)
            .collect();
        Demodulator {
            symbol_len,
            mask: symbol_len - 1,
            symbol_separ2: symbol_len / 4,
            inp_tap: vec![0.0; symbol_len],
            inp_ptr: 0,
            shape,
            fft: ForwardFft::new(symbol_len),
            buf: vec![C64::default(); symbol_len],
            spectra: [vec![C64::default(); symbol_len / 2], vec![C64::default(); symbol_len / 2]],
            energy: [vec![0.0; decode_width], vec![0.0; decode_width]],
            first_bin: first_carrier - decode_margin,
            decode_margin,
            carriers,
            bits,
            tone_symbol: (0..carriers).map(|i| binary_code(i as u8)).collect(),
        }
    }

    fn feed(&mut self, input: &[f64]) {
        for &x in input {
            self.inp_tap[self.inp_ptr] = x;
            self.inp_ptr = (self.inp_ptr + 1) & self.mask;
        }
    }

    /// Un periodo di simbolo (`symbol_len / 2` campioni).
    fn process(&mut self, input: &[f64]) {
        let (first, second) = input.split_at(self.symbol_separ2.min(input.len()));
        self.feed(first);
        for t in 0..self.symbol_len {
            self.buf[t] = C64::new(self.inp_tap[self.inp_ptr] * self.shape[t], 0.0);
            self.inp_ptr = (self.inp_ptr + 1) & self.mask;
        }
        self.feed(second);
        for t in 0..self.symbol_len {
            self.buf[t].im = self.inp_tap[self.inp_ptr] * self.shape[t];
            self.inp_ptr = (self.inp_ptr + 1) & self.mask;
        }
        self.fft.process(&mut self.buf);
        let [s0, s1] = &mut self.spectra;
        separ_two_reals(&self.buf, s0, s1);
        for slice in 0..SLICES_PER_SYMBOL {
            let spectra = &self.spectra[slice];
            for (idx, e) in self.energy[slice].iter_mut().enumerate() {
                *e = spectra[self.first_bin + idx].norm_sqr();
            }
        }
    }

    /// I bit morbidi di un mezzo simbolo con lo spostamento `offset` (in bin): positivo vuol dire bit 0, negativo bit 1.
    fn soft_decode(&self, slice: usize, offset: isize, symbol: &mut [f64]) {
        symbol.fill(0.0);
        let base = (self.decode_margin as isize + offset).max(0) as usize;
        let energy = &self.energy[slice];
        let mut total = 0.0;
        for idx in 0..self.carriers {
            let e = energy.get(base + CARRIER_SEPAR * idx).copied().unwrap_or(0.0);
            let e2 = e * e;
            total += e2;
            let s = self.tone_symbol[idx];
            for (bit, value) in symbol.iter_mut().enumerate().take(self.bits) {
                if s >> bit & 1 == 1 {
                    *value -= e2;
                } else {
                    *value += e2;
                }
            }
        }
        if total > 0.0 {
            for value in symbol.iter_mut().take(self.bits) {
                *value /= total;
            }
        }
    }
}

/// Il decodificatore a decisione morbida di un blocco FEC: tiene gli ultimi `symbols_per_block` simboli e ne ricava un carattere per bit.
struct SoftDecoder {
    bits: usize,
    symbols_per_block: usize,
    contestia: bool,
    input: Vec<f64>,
    ptr: usize,
    fht_buf: Vec<f64>,
    /// Per ogni carattere del blocco, il segno da applicare a ogni bit (codice di rimescolamento gia' ruotato).
    flip: Vec<Vec<bool>>,
    output: [u8; 8],
    signal: f64,
    noise_energy: f64,
}

impl SoftDecoder {
    fn new(variant: Variant, bits: usize) -> SoftDecoder {
        let symbols_per_block = 1usize << (variant.bits_per_character() - 1);
        let wrap = symbols_per_block - 1;
        let code = variant.scrambling_code();
        let flip = (0..bits)
            .map(|fb| {
                let start = (fb * variant.shift()) & wrap;
                (0..symbols_per_block).map(|tb| code & (1u64 << ((start + tb) & wrap)) != 0).collect()
            })
            .collect();
        SoftDecoder {
            bits,
            symbols_per_block,
            contestia: variant == Variant::Contestia,
            input: vec![0.0; symbols_per_block * bits],
            ptr: 0,
            fht_buf: vec![0.0; symbols_per_block],
            flip,
            output: [0; 8],
            signal: 0.0,
            noise_energy: 0.0,
        }
    }

    fn push(&mut self, symbol: &[f64]) {
        for &v in symbol.iter().take(self.bits) {
            self.input[self.ptr] = v;
            self.ptr += 1;
        }
        if self.ptr >= self.input.len() {
            self.ptr -= self.input.len();
        }
    }

    fn decode_character(&mut self, freq_bit: usize) {
        let len = self.input.len();
        let mut ptr = self.ptr;
        let mut rotate = freq_bit;
        for tb in 0..self.symbols_per_block {
            let bit = self.input[ptr + rotate];
            self.fht_buf[tb] = if self.flip[freq_bit][tb] { -bit } else { bit };
            rotate += 1;
            if rotate >= self.bits {
                rotate -= self.bits;
            }
            ptr += self.bits;
            if ptr >= len {
                ptr -= len;
            }
        }
        fht(&mut self.fht_buf);
        let mut peak = 0.0f64;
        let mut peak_pos = 0;
        let mut sqr_sum = 0.0;
        for (tb, &s) in self.fht_buf.iter().enumerate() {
            sqr_sum += s * s;
            if s.abs() > peak.abs() {
                peak = s;
                peak_pos = tb;
            }
        }
        let mut ch = peak_pos as u8;
        if peak < 0.0 {
            ch += self.symbols_per_block as u8;
        }
        sqr_sum -= peak * peak;
        if self.contestia && ch > 0 {
            ch = match ch {
                59 => b' ',
                60 => b'\r',
                61 => 8,
                _ => ch + 32,
            };
        }
        self.output[freq_bit] = ch;
        self.noise_energy += sqr_sum / (self.symbols_per_block - 1) as f64;
        self.signal += peak.abs();
    }

    fn process(&mut self) {
        self.signal = 0.0;
        self.noise_energy = 0.0;
        for fb in 0..self.bits {
            self.decode_character(fb);
        }
        self.signal /= self.bits as f64;
        self.noise_energy /= self.bits as f64;
    }

    /// I caratteri del blocco impacchettati in un `u64`, il primo nel byte basso.
    fn packed(&self) -> u64 {
        self.output.iter().take(self.bits).rev().fold(0u64, |acc, &c| (acc << 8) | u64::from(c))
    }
}

/// Il ricevitore di Olivia (o Contestia): audio a 8000 Hz in ingresso, caratteri in uscita.
pub struct Receiver {
    filter: InputFilter,
    input_buf: Vec<f64>,
    processor: InputProcessor,
    demodulator: Demodulator,
    symbol_len: usize,
    symbol_separ: usize,
    bits: usize,
    symbols_per_block: usize,
    sync_integ_len: usize,
    sync_threshold: f64,
    freq_offsets: usize,
    block_phases: usize,
    decoders: Vec<SoftDecoder>,
    block_phase: usize,
    sync_noise: Vec<LowPass3>,
    sync_signal: Vec<LowPass3>,
    sync_weight: f64,
    best_signal: f64,
    best_block_phase: usize,
    best_freq_offset: usize,
    snr: f64,
    /// I blocchi decodificati: per ogni fase del blocco, `sync_integ_len` righe di `freq_offsets` blocchi.
    pipe: Vec<u64>,
    pipe_ptr: Vec<usize>,
    output: Vec<u8>,
    symbol: [f64; 8],
}

impl Receiver {
    /// `sync_margin`: di quanti bin si cerca il segnale attorno alla frequenza nominale (minimo 2, di base 8 in fldigi);
    /// `sync_integ_len`: su quanti blocchi FEC si integra il sincronismo (minimo 2, di base 4); `sync_threshold`: la soglia del
    /// rapporto segnale/rumore per mandare in uscita i caratteri (minimo 3). `None` se i parametri non stanno nella banda audio.
    pub fn new(params: Params, sync_margin: usize, sync_integ_len: usize, sync_threshold: f64) -> Option<Receiver> {
        let bits = params.bits_per_symbol();
        if !(1..=8).contains(&bits) {
            return None;
        }
        let symbol_len = params.symbol_len();
        let first_carrier = params.first_carrier();
        let margin = sync_margin.max(2).min(first_carrier);
        let carriers = 1usize << bits;
        let decode_width = carriers * CARRIER_SEPAR - 1 + 2 * margin;
        if first_carrier - margin + decode_width > symbol_len / 2 {
            return None;
        }
        let sync_integ_len = sync_integ_len.max(2);
        let sync_threshold = if sync_threshold.is_finite() { sync_threshold.max(3.0) } else { 3.0 };
        let symbols_per_block = 1usize << (params.variant.bits_per_character() - 1);
        let freq_offsets = 2 * margin + 1;
        let block_phases = SLICES_PER_SYMBOL * symbols_per_block;
        let decoders = (0..SLICES_PER_SYMBOL * freq_offsets).map(|_| SoftDecoder::new(params.variant, bits)).collect();
        Some(Receiver {
            filter: InputFilter::new(),
            input_buf: Vec::new(),
            processor: InputProcessor::new(16 * symbol_len),
            demodulator: Demodulator::new(symbol_len, first_carrier, margin, bits),
            symbol_len,
            symbol_separ: symbol_len / 2,
            bits,
            symbols_per_block,
            sync_integ_len,
            sync_threshold,
            freq_offsets,
            block_phases,
            decoders,
            block_phase: 0,
            sync_noise: vec![LowPass3::default(); block_phases * freq_offsets],
            sync_signal: vec![LowPass3::default(); block_phases * freq_offsets],
            sync_weight: 1.0 / sync_integ_len as f64,
            best_signal: 0.0,
            best_block_phase: 0,
            best_freq_offset: 0,
            snr: 0.0,
            pipe: vec![0; block_phases * sync_integ_len * freq_offsets],
            pipe_ptr: vec![0; block_phases],
            output: Vec::new(),
            symbol: [0.0; 8],
        })
    }

    /// Elabora un blocco di audio a 8000 Hz (di qualunque lunghezza).
    pub fn process(&mut self, samples: &[f32]) {
        self.input_buf.reserve(samples.len());
        for &s in samples {
            let x = if s.is_finite() { f64::from(s) } else { 0.0 };
            let y = self.filter.process(x);
            self.input_buf.push(y);
        }
        self.process_input_buffer();
    }

    /// Finisce di elaborare quello che e' in coda e spinge fuori gli ultimi blocchi con del silenzio (come fa la libreria quando si
    /// passa in trasmissione).
    pub fn flush(&mut self) {
        let window = self.processor.window_len;
        self.process_input_buffer();
        self.input_buf.resize(window, 0.0);
        self.process_input_buffer();
        let flush_len = self.symbol_separ * self.symbols_per_block * self.sync_integ_len * 2;
        let mut done = 0;
        while done < flush_len {
            self.input_buf.clear();
            self.input_buf.resize(window, 0.0);
            self.process_input_buffer();
            done += window;
        }
        self.input_buf.clear();
    }

    /// I caratteri decodificati finora (senza gli zeri); la coda si svuota.
    pub fn take(&mut self) -> Vec<u8> {
        let mut out = std::mem::take(&mut self.output);
        out.retain(|&c| c != 0);
        out
    }

    /// Il rapporto segnale/rumore del sincronismo, calcolato come nella libreria (lo stesso numero confrontato con la soglia).
    pub fn snr(&self) -> f64 {
        self.snr
    }

    /// Lo spostamento di frequenza del segnale trovato dal sincronismo, in Hz.
    pub fn frequency_offset(&self) -> f64 {
        (self.best_freq_offset as f64 - (self.freq_offsets / 2) as f64) * (RATE_HZ / self.symbol_len as f64)
    }

    fn process_input_buffer(&mut self) {
        let window = self.processor.window_len;
        let mut start = 0;
        while self.input_buf.len() - start >= window {
            self.processor.process(&self.input_buf[start..start + window]);
            start += window;
            let output = std::mem::take(&mut self.processor.output);
            for chunk in output.chunks_exact(self.symbol_separ) {
                self.process_symbol(chunk);
            }
            self.processor.output = output;
        }
        self.input_buf.drain(..start);
    }

    fn process_symbol(&mut self, input: &[f64]) {
        self.demodulator.process(input);
        let fo = self.freq_offsets;
        let half = (fo / 2) as isize;
        let mut decoder_idx = 0;
        for slice in 0..SLICES_PER_SYMBOL {
            let bp = self.block_phase;
            let row = bp * fo;
            let pipe_row = (bp * self.sync_integ_len + self.pipe_ptr[bp]) * fo;
            let mut best_offset = 0;
            let mut best_signal = 0.0;
            for offset in 0..fo {
                self.demodulator.soft_decode(slice, offset as isize - half, &mut self.symbol);
                let decoder = &mut self.decoders[decoder_idx];
                decoder.push(&self.symbol);
                decoder.process();
                self.pipe[pipe_row + offset] = decoder.packed();
                self.sync_noise[row + offset].process(decoder.noise_energy, self.sync_weight);
                let integ = &mut self.sync_signal[row + offset];
                integ.process(decoder.signal, self.sync_weight);
                if integ.output > best_signal {
                    best_signal = integ.output;
                    best_offset = offset;
                }
                decoder_idx += 1;
            }
            self.pipe_ptr[bp] = (self.pipe_ptr[bp] + 1) % self.sync_integ_len;

            if bp == self.best_block_phase {
                self.best_signal = best_signal;
                self.best_freq_offset = best_offset;
            } else if best_signal > self.best_signal {
                self.best_signal = best_signal;
                self.best_block_phase = bp;
                self.best_freq_offset = best_offset;
            }

            let dist = (bp + self.block_phases - self.best_block_phase) % self.block_phases;
            if dist == self.block_phases / 2 {
                // Il rumore si legge dove lo legge la libreria: nell'integratore subito dopo l'ultimo usato (la fase seguente, primo
                // spostamento). Non e' quello del segnale migliore, ma cosi' la soglia scatta come in fldigi; all'ultima fase si torna
                // all'inizio invece di uscire dalla tabella.
                let noise_idx = (row + fo) % self.sync_noise.len();
                let best_noise = self.sync_noise[noise_idx].output.max(0.0).sqrt();
                self.snr = if best_noise == 0.0 { 0.0 } else { self.best_signal / best_noise };
                if self.snr >= self.sync_threshold {
                    let best_row =
                        (self.best_block_phase * self.sync_integ_len + self.pipe_ptr[self.best_block_phase]) * fo;
                    let mut block = self.pipe[best_row + self.best_freq_offset];
                    for _ in 0..self.bits {
                        self.output.push((block & 0xFF) as u8);
                        block >>= 8;
                    }
                }
                if !self.snr.is_finite() || self.snr > 100.0 {
                    self.snr = 0.0;
                }
            }
            self.block_phase += 1;
        }
        if self.block_phase >= self.block_phases {
            self.block_phase -= self.block_phases;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::olivia::Transmitter;
    use rustfft::FftPlanner;

    /// Generatore ripetibile (xorshift) per ritardi e rumore.
    struct Rng(u64);

    impl Rng {
        fn next(&mut self) -> u64 {
            self.0 ^= self.0 << 13;
            self.0 ^= self.0 >> 7;
            self.0 ^= self.0 << 17;
            self.0
        }
        fn uniform(&mut self) -> f64 {
            ((self.next() >> 11) as f64 + 0.5) / (1u64 << 53) as f64
        }
        fn gauss(&mut self) -> f64 {
            (-2.0 * self.uniform().ln()).sqrt() * (2.0 * std::f64::consts::PI * self.uniform()).cos()
        }
    }

    /// Sposta in frequenza di `hz` tutto l'audio (segnale analitico con una FFT sull'intera durata, poi parte reale).
    fn shift_frequency(audio: &[f32], hz: f64) -> Vec<f32> {
        let n = audio.len().next_power_of_two();
        let mut buf: Vec<C64> = audio.iter().map(|&x| C64::new(f64::from(x), 0.0)).collect();
        buf.resize(n, C64::default());
        let mut planner = FftPlanner::new();
        planner.plan_fft_forward(n).process(&mut buf);
        for (i, v) in buf.iter_mut().enumerate() {
            if i == 0 || i == n / 2 {
                continue;
            }
            *v = if i < n / 2 { *v * 2.0 } else { C64::default() };
        }
        planner.plan_fft_inverse(n).process(&mut buf);
        (0..audio.len())
            .map(|i| {
                let ph = 2.0 * std::f64::consts::PI * hz * i as f64 / RATE_HZ;
                ((buf[i] * C64::new(ph.cos(), ph.sin())).re / n as f64) as f32
            })
            .collect()
    }

    fn params(variant: Variant, tones: usize, bandwidth: usize) -> Params {
        Params { variant, tones, bandwidth, center_hz: 1500.0 }
    }

    /// Trasmette, aggiunge silenzio iniziale, spostamento e rumore, e decodifica con il nostro ricevitore a blocchi irregolari.
    fn round_trip(p: Params, text: &[u8], delay: usize, shift_hz: f64, noise: f64, seed: u64) -> (String, f64, f64) {
        let tx = Transmitter::new(p).expect("parametri validi");
        let mut audio = vec![0.0f32; delay];
        audio.extend(tx.transmit(text));
        audio.extend(std::iter::repeat_n(0.0f32, 8000 * 2));
        if shift_hz != 0.0 {
            audio = shift_frequency(&audio, shift_hz);
        }
        let mut rng = Rng(seed | 1);
        for s in audio.iter_mut() {
            *s = (f64::from(*s) * 0.5 + noise * rng.gauss()) as f32;
        }
        let mut rx = Receiver::new(p, 8, 4, 3.0).expect("parametri validi");
        let mut pos = 0;
        let mut text_out = Vec::new();
        let mut snr: f64 = 0.0;
        let mut freq = 0.0;
        while pos < audio.len() {
            let n = (1 + (rng.next() % 3000) as usize).min(audio.len() - pos);
            rx.process(&audio[pos..pos + n]);
            pos += n;
            if rx.snr() > snr {
                snr = rx.snr();
                freq = rx.frequency_offset();
            }
            text_out.extend(rx.take());
        }
        rx.flush();
        text_out.extend(rx.take());
        (String::from_utf8_lossy(&text_out).into_owned(), snr, freq)
    }

    const TEXT: &[u8] = b"CQ CQ CQ DE IU3ABC IU3ABC IO91WM K";

    #[test]
    fn our_fft_matches_the_direct_dft() {
        for len in [4usize, 16, 256] {
            let mut rng = Rng(len as u64);
            let input: Vec<C64> = (0..len).map(|_| C64::new(rng.gauss(), rng.gauss())).collect();
            let mut fast = input.clone();
            ForwardFft::new(len).process(&mut fast);
            for (k, got) in fast.iter().enumerate() {
                let want: C64 = input
                    .iter()
                    .enumerate()
                    .map(|(t, x)| {
                        let ph = -2.0 * std::f64::consts::PI * (k * t % len) as f64 / len as f64;
                        x * C64::new(ph.cos(), ph.sin())
                    })
                    .sum();
                assert!((got - want).norm() < 1e-9 * len as f64, "len {len}, bin {k}");
            }
        }
    }

    #[test]
    fn clean_signal_decodes_in_every_common_variant() {
        for (variant, tones, bw) in [
            (Variant::Olivia, 4usize, 250usize),
            (Variant::Olivia, 8, 250),
            (Variant::Olivia, 8, 500),
            (Variant::Olivia, 16, 500),
            (Variant::Olivia, 32, 1000),
            (Variant::Contestia, 16, 500),
        ] {
            let (got, snr, freq) = round_trip(params(variant, tones, bw), TEXT, 0, 0.0, 0.0, 7);
            assert!(got.contains("CQ CQ CQ DE IU3ABC IU3ABC IO91WM K"), "{variant:?} {tones}/{bw}: {got:?}");
            assert!(snr >= 3.0, "{variant:?} {tones}/{bw}: S/N {snr}");
            assert!(freq.abs() < 1e-9, "{variant:?} {tones}/{bw}: offset {freq}");
        }
    }

    #[test]
    fn offset_delay_and_noise_still_decode() {
        let mut rng = Rng(0x1234_5678);
        for (i, (variant, tones, bw)) in [
            (Variant::Olivia, 4usize, 250usize),
            (Variant::Olivia, 8, 250),
            (Variant::Olivia, 8, 500),
            (Variant::Olivia, 16, 500),
            (Variant::Olivia, 32, 1000),
            (Variant::Contestia, 16, 500),
        ]
        .into_iter()
        .enumerate()
        {
            let p = params(variant, tones, bw);
            let bin_hz = RATE_HZ / p.symbol_len() as f64;
            // fra mezzo passo dei toni e tre bin, alternando il segno: sempre entro il margine di 8 bin
            let sign = if i % 2 == 0 { 1.0 } else { -1.0 };
            let shift = sign * bin_hz * (0.4 + 2.6 * rng.uniform());
            let delay = (rng.next() % 16000) as usize;
            let (got, _, freq) = round_trip(p, TEXT, delay, shift, 0.25, 100 + i as u64);
            assert!(got.contains("IU3ABC IU3ABC IO91WM"), "{variant:?} {tones}/{bw}, {shift:.1} Hz, ritardo {delay}: {got:?}");
            assert!((freq - shift).abs() <= bin_hz, "{variant:?} {tones}/{bw}: stimato {freq:.1} Hz invece di {shift:.1}");
        }
    }

    #[test]
    fn empty_zero_and_noise_input_never_panics() {
        for (variant, tones, bw) in [(Variant::Olivia, 2usize, 125usize), (Variant::Olivia, 32, 1000), (Variant::Contestia, 256, 2000), (Variant::Olivia, 8, 250)] {
            let mut rx = Receiver::new(params(variant, tones, bw), 8, 4, 3.0).expect("parametri validi");
            rx.process(&[]);
            rx.flush();
            assert!(rx.take().is_empty());
            rx.process(&vec![0.0; 50_000]);
            rx.flush();
            assert!(rx.take().is_empty(), "silenzio: niente caratteri");
            let mut rng = Rng(99);
            let noise: Vec<f32> = (0..80_000).map(|_| rng.gauss() as f32).collect();
            let mut pos = 0;
            let mut len = 0;
            while pos < noise.len() {
                let n = len.min(noise.len() - pos);
                rx.process(&noise[pos..pos + n]);
                pos += n;
                len = len * 3 + 1;
                if len > 20_000 {
                    len = 0;
                }
            }
            rx.process(&[f32::NAN, f32::INFINITY, -f32::INFINITY, 1e30]);
            rx.flush();
            let _ = rx.take();
            assert!(rx.snr().is_finite() && rx.frequency_offset().is_finite());
        }
        // banda fuori dall'audio
        assert!(Receiver::new(Params { variant: Variant::Olivia, tones: 32, bandwidth: 1000, center_hz: 3900.0 }, 8, 4, 3.0).is_none());
        assert!(Receiver::new(Params { variant: Variant::Olivia, tones: 512, bandwidth: 1000, center_hz: 1500.0 }, 8, 4, 3.0).is_none());
    }

    #[test]
    fn sixty_seconds_of_audio_run_much_faster_than_real_time() {
        let p = params(Variant::Olivia, 32, 1000);
        let tx = Transmitter::new(p).expect("parametri validi");
        let mut audio = tx.transmit(b"CQ CQ DE IU3ABC IU3ABC IO91WM PSE K ");
        let mut rng = Rng(5);
        audio.resize(8000 * 60, 0.0);
        for s in audio.iter_mut() {
            *s = (f64::from(*s) * 0.5 + 0.2 * rng.gauss()) as f32;
        }
        for (variant, tones, bw) in [(Variant::Olivia, 32usize, 1000usize), (Variant::Olivia, 8, 250), (Variant::Olivia, 256, 2000)] {
            let mut rx = Receiver::new(params(variant, tones, bw), 8, 4, 3.0).expect("parametri validi");
            let start = std::time::Instant::now();
            for chunk in audio.chunks(1024) {
                rx.process(chunk);
            }
            let elapsed = start.elapsed().as_secs_f64();
            println!("Olivia {tones}/{bw}: 60 s di audio in {elapsed:.3} s ({:.0} volte il tempo reale)", 60.0 / elapsed);
            assert!(elapsed < 60.0, "{tones}/{bw}: piu' lento del tempo reale ({elapsed:.1} s)");
        }
    }
}
