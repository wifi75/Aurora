//! CRC a 14 bit del messaggio FT8 (polinomio 0x2757), come `chkcrc14a` e `crc14` di JTDX/WSJT-X.
//!
//! Il CRC si calcola sui 77 bit del messaggio seguiti da 19 zeri (5 di riempimento fino a 82 bit e 14 per il campo del CRC
//! stesso: in tutto 96 bit, i 12 byte che JTDX passa a `crc14`), con registro iniziale a zero e senza inversioni. I 14 bit
//! risultanti, dal piu' significativo, si accodano al messaggio: i 91 bit che entrano nel codice LDPC.

/// Polinomio troncato (senza il termine di grado 14).
const POLY: u16 = 0x2757;
/// Il registro e' largo 14 bit.
const MASK: u16 = 0x3FFF;
/// Bit del messaggio.
const MESSAGE_BITS: usize = 77;
/// Zeri che seguono il messaggio nel calcolo: 5 di riempimento e 14 del campo CRC.
const TRAILING_ZEROS: usize = 19;

/// Un passo della divisione tra polinomi: entra un bit, esce quello in cima al registro.
fn shift_in(register: u16, bit: u8) -> u16 {
    let carry = (register >> 13) & 1;
    let next = ((register << 1) | u16::from(bit & 1)) & MASK;
    if carry == 1 { next ^ POLY } else { next }
}

/// CRC a 14 bit di 77 bit (uno per elemento, 0 o 1).
fn crc14(message: &[u8; MESSAGE_BITS]) -> u16 {
    let mut register = 0;
    for &bit in message {
        register = shift_in(register, bit);
    }
    for _ in 0..TRAILING_ZEROS {
        register = shift_in(register, 0);
    }
    register
}

/// Messaggio a 77 bit (un bit per elemento, 0 o 1) con i 14 bit di CRC in coda: 91 bit.
pub fn append_crc(message: &[u8; 77]) -> [u8; 91] {
    let crc = crc14(message);
    let mut out = [0u8; 91];
    for (dst, &src) in out.iter_mut().zip(message.iter()) {
        *dst = src & 1;
    }
    for (i, dst) in out[MESSAGE_BITS..].iter_mut().enumerate() {
        *dst = u8::from((crc >> (13 - i)) & 1 == 1);
    }
    out
}

/// Vero se i 91 bit hanno un CRC giusto.
pub fn check(bits: &[u8; 91]) -> bool {
    let mut message = [0u8; MESSAGE_BITS];
    message.copy_from_slice(&bits[..MESSAGE_BITS]);
    let received = bits[MESSAGE_BITS..]
        .iter()
        .fold(0u16, |acc, &b| (acc << 1) | u16::from(b & 1));
    crc14(&message) == received
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Generatore pseudocasuale deterministico (splitmix64).
    struct Rng(u64);
    impl Rng {
        fn next(&mut self) -> u64 {
            self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
            let mut z = self.0;
            z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
            z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
            z ^ (z >> 31)
        }
        fn message(&mut self) -> [u8; 77] {
            let mut m = [0u8; 77];
            for chunk in m.chunks_mut(32) {
                let v = self.next();
                for (i, b) in chunk.iter_mut().enumerate() {
                    *b = ((v >> i) & 1) as u8;
                }
            }
            m
        }
    }

    /// Riferimento indipendente, a byte e senza zeri aggiunti: e' `ftx_compute_crc` di ft8_lib (divisione modulo 2 con il
    /// metodo "non aumentato").
    fn reference_crc(bytes: &[u8], num_bits: usize) -> u16 {
        let top = 1u16 << 13;
        let mut remainder: u16 = 0;
        let mut idx_byte = 0;
        for idx_bit in 0..num_bits {
            if idx_bit % 8 == 0 {
                remainder ^= u16::from(bytes[idx_byte]) << 6;
                idx_byte += 1;
            }
            remainder = if remainder & top != 0 {
                (remainder << 1) ^ POLY
            } else {
                remainder << 1
            };
        }
        remainder & 0x3FFF
    }

    /// 77 bit in 10 byte, con i 3 bit in coda a zero (82 bit con il riempimento, come `ftx_add_crc`).
    fn to_bytes(message: &[u8; 77]) -> [u8; 12] {
        let mut bytes = [0u8; 12];
        for (i, &b) in message.iter().enumerate() {
            bytes[i / 8] |= b << (7 - i % 8);
        }
        bytes
    }

    #[test]
    fn vettore_noto_di_wsjtx() {
        // Vettore del programma di prova di weakmon (ft8.py), ricavato da WSJT-X: messaggio a 82 bit con i bit 3, 7, 44, 45,
        // 46, 51, 61 e 71 a uno (base zero), CRC atteso 11001111110101.
        let mut message = [0u8; 77];
        for i in [3, 7, 44, 45, 46, 51, 61, 71] {
            message[i] = 1;
        }
        let expected = [1, 1, 0, 0, 1, 1, 1, 1, 1, 1, 0, 1, 0, 1];
        let bits = append_crc(&message);
        assert_eq!(&bits[77..], &expected[..]);
        assert!(check(&bits));
        assert_eq!(reference_crc(&to_bytes(&message), 82), 0b11_0011_1111_0101);
    }

    #[test]
    fn coincide_con_il_riferimento_su_82_bit() {
        let mut rng = Rng(1);
        for _ in 0..500 {
            let m = rng.message();
            let expected = reference_crc(&to_bytes(&m), 82);
            let bits = append_crc(&m);
            let got = bits[77..].iter().fold(0u16, |a, &b| (a << 1) | u16::from(b));
            assert_eq!(got, expected);
        }
    }

    #[test]
    fn messaggio_a_zero_ha_crc_zero() {
        let bits = append_crc(&[0; 77]);
        assert!(bits.iter().all(|&b| b == 0));
        assert!(check(&bits));
    }

    #[test]
    fn il_crc_e_lineare() {
        let mut rng = Rng(2);
        for _ in 0..100 {
            let a = rng.message();
            let b = rng.message();
            let mut x = [0u8; 77];
            for i in 0..77 {
                x[i] = a[i] ^ b[i];
            }
            let (ca, cb, cx) = (append_crc(&a), append_crc(&b), append_crc(&x));
            for i in 77..91 {
                assert_eq!(cx[i], ca[i] ^ cb[i]);
            }
        }
    }

    #[test]
    fn append_poi_check_torna() {
        let mut rng = Rng(3);
        for _ in 0..500 {
            let m = rng.message();
            let bits = append_crc(&m);
            assert_eq!(&bits[..77], &m[..]);
            assert!(check(&bits));
        }
    }

    #[test]
    fn un_bit_cambiato_rompe_il_crc() {
        let mut rng = Rng(4);
        for _ in 0..20 {
            let bits = append_crc(&rng.message());
            for i in 0..91 {
                let mut bad = bits;
                bad[i] ^= 1;
                assert!(!check(&bad), "bit {i}");
            }
        }
    }

    #[test]
    fn due_bit_cambiati_rompono_il_crc() {
        // Il polinomio rileva tutti gli errori doppi su una parola cosi' corta.
        let mut rng = Rng(5);
        let bits = append_crc(&rng.message());
        for i in 0..91 {
            for j in (i + 1)..91 {
                let mut bad = bits;
                bad[i] ^= 1;
                bad[j] ^= 1;
                assert!(!check(&bad), "bit {i} e {j}");
            }
        }
    }
}
