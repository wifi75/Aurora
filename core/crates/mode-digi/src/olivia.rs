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

    fn scrambling_code(self) -> u64 {
        match self {
            Variant::Olivia => SCRAMBLING_OLIVIA,
            Variant::Contestia => SCRAMBLING_CONTESTIA,
        }
    }

    /// Di quanto si ruota il codice di rimescolamento da un carattere al successivo.
    fn shift(self) -> usize {
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
