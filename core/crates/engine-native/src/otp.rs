//! Il codice di verifica della Fox (`foxOTPcode` di WSJT-X 3.x, `otpgenerator.cpp`): TOTP a 6 cifre con HMAC-SHA1, passo di 30 s,
//! seme in base32 di 16 caratteri. Il Fox classico lo manda come testo libero `CALL.123456` ogni `OTPinterval` periodi, SuperFox (UltraFOX)
//! nel campo d20 del pacchetto; il Hound lo verifica con il servizio in linea (`features::otp_action`).

use sha1::{Digest, Sha1};

/// Lunghezza del seme che WSJT-X accetta (`Configuration::OTPEnabled`: `OTPSeed_.size() == 16`).
pub const SEED_LEN: usize = 16;
/// Il codice che vuol dire «nessuna verifica» (senza seme).
pub const NO_CODE: &str = "000000";

/// Il seme e' valido: 16 caratteri dell'alfabeto base32 (A-Z, 2-7).
pub fn seed_ok(seed: &str) -> bool {
    seed.len() == SEED_LEN && seed.bytes().all(|b| b.is_ascii_uppercase() || (b'2'..=b'7').contains(&b))
}

/// Da base32 a byte (`OTPGenerator::fromBase32`: i caratteri fuori alfabeto si saltano).
pub fn from_base32(input: &str) -> Vec<u8> {
    let mut out = Vec::with_capacity(input.len() * 5 / 8 + 1);
    let (mut buffer, mut bits) = (0u32, 0u32);
    for ch in input.bytes() {
        let value = match ch {
            b'A'..=b'Z' => ch - b'A',
            b'2'..=b'7' => 26 + ch - b'2',
            _ => continue,
        };
        buffer = (buffer << 5) | u32::from(value);
        bits += 5;
        if bits >= 8 {
            out.push((buffer >> (bits - 8)) as u8);
            bits -= 8;
        }
    }
    out
}

/// HMAC-SHA1 (RFC 2104).
fn hmac_sha1(key: &[u8], data: &[u8]) -> [u8; 20] {
    const BLOCK: usize = 64;
    let mut k = [0u8; BLOCK];
    if key.len() > BLOCK {
        k[..20].copy_from_slice(&Sha1::digest(key));
    } else {
        k[..key.len()].copy_from_slice(key);
    }
    let mut inner = Sha1::new();
    inner.update(k.map(|b| b ^ 0x36));
    inner.update(data);
    let mut outer = Sha1::new();
    outer.update(k.map(|b| b ^ 0x5c));
    outer.update(inner.finalize());
    outer.finalize().into()
}

/// HOTP (RFC 4226, `OTPGenerator::generateHOTP`) con `digits` cifre.
pub fn hotp(secret: &[u8], counter: u64, digits: u32) -> String {
    let mac = hmac_sha1(secret, &counter.to_be_bytes());
    let off = usize::from(mac[19] & 0x0f);
    let truncated = (u32::from(mac[off] & 0x7f) << 24) | (u32::from(mac[off + 1]) << 16) | (u32::from(mac[off + 2]) << 8) | u32::from(mac[off + 3]);
    format!("{:0width$}", truncated % 10u32.pow(digits), width = digits as usize)
}

/// Il codice a 6 cifre del momento `unix_ms` (`foxOTPcode`): `000000` se il seme non e' valido.
pub fn fox_code(seed: &str, unix_ms: u64) -> String {
    if !seed_ok(seed) {
        return NO_CODE.to_string();
    }
    hotp(&from_base32(seed), unix_ms / 30_000, 6)
}

/// Il messaggio OTP del Fox classico: nominativo troncato a 6 caratteri e tagliato alla prima barra, poi `.` e il codice
/// (`N5J.123456`, `W1AW/7` -> `W1AW.123456`, `4U1IARU` -> `4U1IAR.123456`).
pub fn classic_message(my_call: &str, code: &str) -> String {
    let six: String = my_call.chars().take(6).collect();
    let call = six.split('/').next().unwrap_or_default();
    format!("{call}.{code}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rfc6238_sha1_vectors() {
        // RFC 6238, appendice B: seme ASCII «12345678901234567890», 8 cifre
        let secret = b"12345678901234567890";
        assert_eq!(hotp(secret, 59 / 30, 8), "94287082");
        assert_eq!(hotp(secret, 1_111_111_109 / 30, 8), "07081804");
        assert_eq!(hotp(secret, 20_000_000_000 / 30, 8), "65353130");
        // RFC 4226, appendice D: 6 cifre
        assert_eq!(hotp(secret, 0, 6), "755224");
        assert_eq!(hotp(secret, 9, 6), "520489");
    }

    #[test]
    fn base32_and_seed() {
        // «12345678901234567890» in base32
        assert_eq!(from_base32("GEZDGNBVGY3TQOJQGEZDGNBVGY3TQOJQ"), b"12345678901234567890");
        assert!(seed_ok("JBSWY3DPEHPK3PXP"));
        assert!(!seed_ok("JBSWY3DPEHPK3PX"));
        assert!(!seed_ok("jbswy3dpehpk3pxp"));
        assert_eq!(fox_code("corto", 0), NO_CODE);
        let c = fox_code("JBSWY3DPEHPK3PXP", 1_700_000_000_000);
        assert_eq!(c.len(), 6);
        assert_eq!(c, fox_code("JBSWY3DPEHPK3PXP", 1_700_000_000_000 + 1));
    }

    #[test]
    fn classic_otp_message() {
        assert_eq!(classic_message("N5J", "123456"), "N5J.123456");
        assert_eq!(classic_message("W1AW/7", "123456"), "W1AW.123456");
        assert_eq!(classic_message("4U1IARU", "123456"), "4U1IAR.123456");
    }
}
