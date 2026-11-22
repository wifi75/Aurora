//! Olivia e Contestia (MFSK a segnale debole di Pawel Jalocha): la parte che trasforma il testo in simboli, cioe' il codificatore FEC.
//!
//! Un blocco FEC porta `bits_per_symbol` caratteri (un carattere per ogni bit del simbolo) e dura `symbols_per_block` simboli. Ogni carattere
//! diventa una parola di Walsh-Hadamard (trasformata inversa) di lunghezza `symbols_per_block`, scritta con il segno scambiato secondo un
//! codice di rimescolamento ruotato di un passo per ogni carattere; i bit risultanti si distribuiscono sui simboli ruotando di una posizione
//! a ogni simbolo, cosi' ogni simbolo contiene un bit di ciascun carattere del blocco.
//!
//! La logica e' quella descritta dalla libreria di Jalocha usata anche da fldigi (GPL, vedi `terze-parti/LEGGIMI.md`); qui e' riscritta, e le
//! prove con la feature `fldigi-oracle` la confrontano blocco per blocco con l'originale compilato.

/// Codice di rimescolamento di Olivia.
const SCRAMBLING_OLIVIA: u64 = 0xE257_E6D0_2915_74EC;
/// Codice di rimescolamento di Contestia.
const SCRAMBLING_CONTESTIA: u64 = 0xEDB8_8320;

/// Quale dei due modi: cambiano il codice di rimescolamento, i bit per carattere e il passo di rotazione.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Variant {
    Olivia,
    Contestia,
}

impl Variant {
    /// Bit per carattere: 7 in Olivia (ASCII), 6 in Contestia.
    pub fn bits_per_character(self) -> usize {
        match self {
            Variant::Olivia => 7,
            Variant::Contestia => 6,
        }
    }

    pub(crate) fn scrambling_code(self) -> u64 {
        match self {
            Variant::Olivia => SCRAMBLING_OLIVIA,
            Variant::Contestia => SCRAMBLING_CONTESTIA,
        }
    }

    /// Di quanto si ruota il codice di rimescolamento da un carattere al successivo.
    pub(crate) fn shift(self) -> usize {
        match self {
            Variant::Olivia => 13,
            Variant::Contestia => 5,
        }
    }
}

/// Trasformata di Hadamard inversa sul posto, come la `IFHT` della libreria (valori piccoli interi: bastano `i8`).
fn ifht(data: &mut [i8]) {
    let len = data.len();
    let mut step = len / 2;
    while step > 0 {
        let mut ptr = 0;
        while ptr < len {
            for i in ptr..ptr + step {
                let (a, b) = (data[i], data[i + step]);
                data[i] = a.wrapping_sub(b);
                data[i + step] = a.wrapping_add(b);
            }
            ptr += 2 * step;
        }
        step /= 2;
    }
}

/// Il codificatore FEC per un numero di bit per simbolo (toni = 2^bits).
#[derive(Clone, Debug)]
pub struct Encoder {
    variant: Variant,
    bits_per_symbol: usize,
    symbols_per_block: usize,
}

impl Encoder {
    /// `bits_per_symbol` da 1 a 8 (da 2 a 256 toni).
    pub fn new(variant: Variant, bits_per_symbol: usize) -> Option<Encoder> {
        if !(1..=8).contains(&bits_per_symbol) {
            return None;
        }
        Some(Encoder { variant, bits_per_symbol, symbols_per_block: 1 << (variant.bits_per_character() - 1) })
    }

    pub fn bits_per_symbol(&self) -> usize {
        self.bits_per_symbol
    }

    /// Simboli di un blocco: 64 in Olivia, 32 in Contestia.
    pub fn symbols_per_block(&self) -> usize {
        self.symbols_per_block
    }

    /// Il carattere come codice da 0 a `2 * symbols_per_block - 1`. In Contestia solo maiuscole, cifre e pochi segni (gli altri diventano «?»).
    fn character_code(&self, ch: u8) -> usize {
        match self.variant {
            Variant::Olivia => usize::from(ch) & (2 * self.symbols_per_block - 1),
            Variant::Contestia => {
                let ch = ch.to_ascii_uppercase();
                usize::from(match ch {
                    b' ' => 59,
                    b'\r' => 60,
                    b'\n' => 0,
                    33..=90 => ch - 32,
                    8 => 61,
                    0 => 0,
                    _ => b'?' - 32,
                })
            }
        }
    }

    /// Una parola di Hadamard per il carattere: tutti zero tranne un +1 (o -1 nella seconda meta'), poi la trasformata inversa.
    fn character_word(&self, ch: u8) -> Vec<i8> {
        let n = self.symbols_per_block;
        let code = self.character_code(ch);
        let mut word = vec![0i8; n];
        if code < n {
            word[code] = 1;
        } else {
            word[code - n] = -1;
        }
        ifht(&mut word);
        word
    }

    /// Scambia il segno delle posizioni in cui il codice di rimescolamento (ruotato di `offset` bit) vale 1.
    fn scramble(&self, word: &mut [i8], offset: usize) {
        let n = self.symbols_per_block;
        let code = self.variant.scrambling_code();
        let mut bit = offset & (n - 1);
        for item in word.iter_mut() {
            if code & (1u64 << bit) != 0 {
                *item = -*item;
            }
            bit = (bit + 1) & (n - 1);
        }
    }

    /// Codifica un blocco: `chars` ha un carattere per ogni bit del simbolo (se ne mancano, si completa con 0); restituisce
    /// `symbols_per_block` simboli, ciascuno con `bits_per_symbol` bit.
    pub fn encode_block(&self, chars: &[u8]) -> Vec<u8> {
        let mut out = vec![0u8; self.symbols_per_block];
        for freq_bit in 0..self.bits_per_symbol {
            let ch = chars.get(freq_bit).copied().unwrap_or(0);
            let mut word = self.character_word(ch);
            self.scramble(&mut word, freq_bit * self.variant.shift());
            let mut rotate = 0;
            for (time_bit, value) in word.iter().enumerate() {
                if *value < 0 {
                    let mut bit = freq_bit + rotate;
                    if bit >= self.bits_per_symbol {
                        bit -= self.bits_per_symbol;
                    }
                    out[time_bit] |= 1 << bit;
                }
                rotate += 1;
                if rotate >= self.bits_per_symbol {
                    rotate -= self.bits_per_symbol;
                }
            }
        }
        out
    }
}

/// Frequenza di campionamento interna di Olivia: 8000 Hz (le lunghezze dei simboli sono potenze di due su questa frequenza).
pub const RATE_HZ: f64 = 8000.0;

/// Il codice di Gray di un simbolo (i toni vicini differiscono di un solo bit).
fn gray(binary: u8) -> u8 {
    binary ^ (binary >> 1)
}

/// I parametri di un modo: toni, banda e dove sta il centro della banda nell'audio.
#[derive(Clone, Copy, Debug)]
pub struct Params {
    pub variant: Variant,
    /// Numero di toni: 2, 4, 8, 16, 32, 64, 128 o 256.
    pub tones: usize,
    /// Banda in Hz: 125, 250, 500, 1000 o 2000.
    pub bandwidth: usize,
    /// Il centro della banda nell'audio, in Hz.
    pub center_hz: f64,
}

impl Params {
    pub fn bits_per_symbol(&self) -> usize {
        self.tones.max(2).next_power_of_two().trailing_zeros() as usize
    }

    /// Lunghezza di un simbolo in campioni a 8000 Hz: `2^(bit + 7 - log2(banda / 125))`.
    pub(crate) fn symbol_len(&self) -> usize {
        let bw_log = (self.bandwidth.clamp(125, 2000) / 125).max(1).ilog2() as usize;
        1usize << (self.bits_per_symbol() + 7 - bw_log)
    }

    /// Il primo tono, in bin della FFT di un simbolo.
    pub(crate) fn first_carrier(&self) -> usize {
        let bandwidth = (1usize << (self.bandwidth.clamp(125, 2000) / 125).max(1).ilog2()) * 125;
        let fc_offset = bandwidth as f64 * (1.0 - 0.5 / self.tones as f64) / 2.0;
        let multiplier = (self.center_hz - fc_offset) / 500.0;
        ((self.symbol_len() / 16) as f64 * multiplier) as usize + 1
    }

    /// Durata di un simbolo in secondi (la meta' della lunghezza: i simboli si sovrappongono a meta').
    pub fn symbol_period_s(&self) -> f64 {
        (self.symbol_len() / 2) as f64 / RATE_HZ
    }
}

/// Il modulatore MFSK: ogni simbolo e' un tono con una finestra a coseno rialzato, i simboli si sovrappongono a meta' e la fase cambia di
/// un quarto di giro, in verso casuale, a ogni simbolo.
struct Modulator {
    symbol_len: usize,
    symbol_separ: usize,
    first_carrier: i64,
    cosine: Vec<f64>,
    out_tap: Vec<f64>,
    tap_ptr: usize,
    wrap: i64,
    symbol_phase: i64,
    rng: u64,
}

impl Modulator {
    fn new(params: &Params) -> Modulator {
        let symbol_len = params.symbol_len();
        let cosine = (0..symbol_len).map(|i| (2.0 * std::f64::consts::PI * i as f64 / symbol_len as f64).cos()).collect();
        Modulator {
            symbol_len,
            symbol_separ: symbol_len / 2,
            first_carrier: params.first_carrier() as i64,
            cosine,
            out_tap: vec![0.0; symbol_len],
            tap_ptr: 0,
            wrap: symbol_len as i64 - 1,
            symbol_phase: 0,
            rng: 0x9E37_79B9_7F4A_7C15,
        }
    }

    /// Un bit casuale (xorshift): la fase di partenza del simbolo dopo e' sfasata di un quarto di giro in un verso o nell'altro.
    fn coin(&mut self) -> bool {
        self.rng ^= self.rng << 13;
        self.rng ^= self.rng >> 7;
        self.rng ^= self.rng << 17;
        self.rng & 1 == 1
    }

    fn add_symbol(&mut self, freq: i64, mut phase: i64) {
        for t in 0..self.symbol_len {
            let shape = 1.0 - self.cosine[t];
            self.out_tap[self.tap_ptr] += self.cosine[(phase & self.wrap) as usize] * shape;
            phase = (phase + freq) & self.wrap;
            self.tap_ptr = (self.tap_ptr + 1) & self.wrap as usize;
        }
    }

    fn send(&mut self, symbol: u8) {
        let freq = self.first_carrier + 2 * i64::from(gray(symbol));
        let sep = self.symbol_separ as i64;
        let len = self.symbol_len as i64;
        self.symbol_phase = (self.symbol_phase + freq * (sep / 2 - len / 2)) & self.wrap;
        self.add_symbol(freq, self.symbol_phase);
        self.symbol_phase = (self.symbol_phase + freq * (sep / 2 + len / 2)) & self.wrap;
        let differ = len / 4;
        self.symbol_phase += if self.coin() { -differ } else { differ };
        self.symbol_phase &= self.wrap;
    }

    /// I prossimi `symbol_separ` campioni (la meta' di un simbolo): si leggono e si azzerano nella coda di uscita.
    fn output(&mut self) -> Vec<f64> {
        let mut out = Vec::with_capacity(self.symbol_separ);
        for _ in 0..self.symbol_separ {
            out.push(self.out_tap[self.tap_ptr]);
            self.out_tap[self.tap_ptr] = 0.0;
            self.tap_ptr = (self.tap_ptr + 1) & self.wrap as usize;
        }
        out
    }
}

/// Il trasmettitore: testo in, audio a 8000 Hz in uscita (ampiezza massima 1).
pub struct Transmitter {
    params: Params,
    encoder: Encoder,
}

impl Transmitter {
    pub fn new(params: Params) -> Option<Transmitter> {
        let encoder = Encoder::new(params.variant, params.bits_per_symbol())?;
        Some(Transmitter { params, encoder })
    }

    pub fn params(&self) -> &Params {
        &self.params
    }

    /// Caratteri che il trasmettitore manda per secondo.
    pub fn characters_per_second(&self) -> f64 {
        self.encoder.bits_per_symbol() as f64 / (self.encoder.symbols_per_block() as f64 * self.params.symbol_period_s())
    }

    /// Tutta la trasmissione di un testo: parte con un carattere nullo (come fa la libreria), poi il testo, poi il blocco si completa con
    /// caratteri nulli e finisce. L'audio e' a 8000 Hz; ogni mezzo simbolo e' normalizzato al suo massimo, come fa la libreria.
    pub fn transmit(&self, text: &[u8]) -> Vec<f32> {
        let mut stream = BlockStream::from_parts(self.params, self.encoder.clone());
        let mut chars = vec![0u8];
        chars.extend_from_slice(text);
        let mut audio: Vec<f32> = Vec::new();
        for block in chars.chunks(self.encoder.bits_per_symbol()) {
            audio.extend(stream.block(block));
        }
        audio.extend(stream.finish());
        audio
    }
}

/// Il trasmettitore a blocchi, per la trasmissione a flusso: un blocco FEC alla volta, con la fase e la coda del modulatore che continuano
/// da un blocco all'altro. Messi in fila, i blocchi e la coda danno esattamente l'audio di [`Transmitter::transmit`].
pub struct BlockStream {
    encoder: Encoder,
    modulator: Modulator,
}

impl BlockStream {
    pub fn new(params: Params) -> Option<BlockStream> {
        let encoder = Encoder::new(params.variant, params.bits_per_symbol())?;
        Some(BlockStream::from_parts(params, encoder))
    }

    fn from_parts(params: Params, encoder: Encoder) -> BlockStream {
        BlockStream { modulator: Modulator::new(&params), encoder }
    }

    /// Caratteri per blocco (uno per ogni bit del simbolo).
    pub fn chars_per_block(&self) -> usize {
        self.encoder.bits_per_symbol()
    }

    /// L'audio (8000 Hz) di un blocco: `chars` ha al massimo [`BlockStream::chars_per_block`] caratteri, i mancanti valgono zero.
    pub fn block(&mut self, chars: &[u8]) -> Vec<f32> {
        let symbols = self.encoder.encode_block(chars);
        let mut audio = Vec::new();
        for symbol in symbols.iter().take(self.encoder.symbols_per_block()) {
            self.modulator.send(*symbol);
            push_normalized(&mut audio, &self.modulator.output());
        }
        audio
    }

    /// La coda dell'ultimo simbolo (si sovrappone a meta'): la fine della trasmissione.
    pub fn finish(&mut self) -> Vec<f32> {
        let mut audio = Vec::new();
        push_normalized(&mut audio, &self.modulator.output());
        audio
    }
}

/// Mezzo simbolo normalizzato al suo massimo, come fa la libreria.
fn push_normalized(audio: &mut Vec<f32>, chunk: &[f64]) {
    let peak = chunk.iter().fold(0.0f64, |m, v| m.max(v.abs()));
    let scale = if peak > 0.0 { 1.0 / peak } else { 0.0 };
    audio.extend(chunk.iter().map(|v| (v * scale) as f32));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_block_has_the_expected_size_and_every_symbol_fits_in_the_bits() {
        for bits in 1..=8 {
            let o = Encoder::new(Variant::Olivia, bits).expect("bits validi");
            assert_eq!(o.symbols_per_block(), 64);
            let block = o.encode_block(b"HELLO WO");
            assert_eq!(block.len(), 64);
            assert!(block.iter().all(|s| usize::from(*s) < (1 << bits)), "bits {bits}");
            let c = Encoder::new(Variant::Contestia, bits).expect("bits validi");
            assert_eq!(c.symbols_per_block(), 32);
            assert!(c.encode_block(b"HELLO W").iter().all(|s| usize::from(*s) < (1 << bits)));
        }
        assert!(Encoder::new(Variant::Olivia, 0).is_none());
        assert!(Encoder::new(Variant::Olivia, 9).is_none());
    }

    #[test]
    fn the_same_block_always_gives_the_same_symbols_and_different_text_differs() {
        let e = Encoder::new(Variant::Olivia, 5).expect("5 bit");
        assert_eq!(e.encode_block(b"CQ CQ"), e.encode_block(b"CQ CQ"));
        assert_ne!(e.encode_block(b"CQ CQ"), e.encode_block(b"CQ DX"));
    }

    #[test]
    fn contestia_turns_lowercase_into_uppercase_and_unknown_characters_into_a_question_mark() {
        let e = Encoder::new(Variant::Contestia, 5).expect("5 bit");
        assert_eq!(e.encode_block(b"cq"), e.encode_block(b"CQ"));
        assert_eq!(e.encode_block(&[0x7f]), e.encode_block(b"?"));
    }
}
