//! CRC a 14 bit del messaggio FT8 (polinomio 0x2757), come `chkcrc14a` e `crc14` di WSJT-X.

/// Messaggio a 77 bit (un bit per elemento, 0 o 1) con i 14 bit di CRC in coda: 91 bit.
pub fn append_crc(_message: &[u8; 77]) -> [u8; 91] {
    [0; 91]
}

/// Vero se i 91 bit hanno un CRC giusto.
pub fn check(_bits: &[u8; 91]) -> bool {
    false
}
