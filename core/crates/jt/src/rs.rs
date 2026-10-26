//! Codice Reed-Solomon (63,12) di JT65 su GF(64): 51 simboli di parita' (Phil Karn KA9Q, GPL; configurazione di `wrapkarn.c` di WSJT-X:
//! simboli da 6 bit, polinomio 0x43, prima radice 3, passo 1).

const MM: usize = 6;
/// Lunghezza della parola di codice.
pub const NN: usize = 63;
/// Simboli di parita'.
pub const NROOTS: usize = 51;
/// Simboli di dati.
pub const DATA: usize = NN - NROOTS;
const GFPOLY: u32 = 0x43;
const FCR: usize = 3;
const PRIM: usize = 1;
/// `A0`: il logaritmo di zero.
const A0: usize = NN;

/// Le tabelle del campo e il polinomio generatore.
pub struct Codec {
    alpha_to: [u8; NN + 1],
    index_of: [u8; NN + 1],
    genpoly: [u8; NROOTS + 1],
    iprim: usize,
}

fn modnn(mut x: usize) -> usize {
    while x >= NN {
        x -= NN;
        x = (x >> MM) + (x & NN);
    }
    x
}

impl Default for Codec {
    fn default() -> Self {
        Codec::new()
    }
}

impl Codec {
    pub fn new() -> Codec {
        let mut alpha_to = [0u8; NN + 1];
        let mut index_of = [0u8; NN + 1];
        index_of[0] = A0 as u8;
        alpha_to[A0] = 0;
        let mut sr: u32 = 1;
        for (i, a) in alpha_to.iter_mut().enumerate().take(NN) {
            index_of[sr as usize] = i as u8;
            *a = sr as u8;
            sr <<= 1;
            if sr & (1 << MM) != 0 {
                sr ^= GFPOLY;
            }
            sr &= NN as u32;
        }
        let iprim = 1;
        let mut genpoly = [0u8; NROOTS + 1];
        genpoly[0] = 1;
        let mut root = FCR * PRIM;
        for i in 0..NROOTS {
            genpoly[i + 1] = 1;
            for j in (1..=i).rev() {
                genpoly[j] = if genpoly[j] != 0 {
                    genpoly[j - 1] ^ alpha_to[modnn(index_of[genpoly[j] as usize] as usize + root)]
                } else {
                    genpoly[j - 1]
                };
            }
            genpoly[0] = alpha_to[modnn(index_of[genpoly[0] as usize] as usize + root)];
            root += PRIM;
        }
        for g in genpoly.iter_mut() {
            *g = index_of[*g as usize];
        }
        Codec { alpha_to, index_of, genpoly, iprim }
    }

    /// `encode_rs`: i 51 simboli di parita' dei 12 di dati (nell'ordine del codificatore di Karn).
    fn parity(&self, data: &[u8; DATA]) -> [u8; NROOTS] {
        let mut bb = [0u8; NROOTS];
        for &d in data {
            let feedback = self.index_of[(d ^ bb[0]) as usize] as usize;
            if feedback != A0 {
                for (j, b) in bb.iter_mut().enumerate().skip(1) {
                    *b ^= self.alpha_to[modnn(feedback + self.genpoly[NROOTS - j] as usize)];
                }
            }
            bb.copy_within(1.., 0);
            bb[NROOTS - 1] = if feedback != A0 { self.alpha_to[modnn(feedback + self.genpoly[0] as usize)] } else { 0 };
        }
        bb
    }

    /// `rs_encode` di `wrapkarn.c`: da 12 simboli di dati (`dgen`) a 63 (`sent`): parita' prima, dati in fondo, come li manda JT65.
    pub fn encode(&self, dgen: &[u8; DATA]) -> [u8; NN] {
        let mut dat1 = [0u8; DATA];
        for i in 0..DATA {
            dat1[i] = dgen[DATA - 1 - i];
        }
        let b = self.parity(&dat1);
        let mut sent = [0u8; NN];
        for i in 0..NROOTS {
            sent[NROOTS - 1 - i] = b[i];
        }
        for i in 0..DATA {
            sent[i + NROOTS] = dat1[DATA - 1 - i];
        }
        sent
    }

    /// `decode_rs`: corregge `data` (63 simboli nell'ordine di Karn) con le cancellazioni date; ritorna il numero di errori corretti,
    /// `None` se non si riesce.
    fn decode_in_place(&self, data: &mut [u8; NN], eras_pos: &[usize]) -> Option<usize> {
        let no_eras = eras_pos.len();
        let mut s = [0usize; NROOTS];
        let mut syn = [data[0]; NROOTS];
        for &d in &data[1..NN] {
            for (i, v) in syn.iter_mut().enumerate() {
                *v = if *v == 0 { d } else { d ^ self.alpha_to[modnn(self.index_of[*v as usize] as usize + (FCR + i) * PRIM)] };
            }
        }
        let mut syn_error = 0u8;
        for i in 0..NROOTS {
            syn_error |= syn[i];
            s[i] = self.index_of[syn[i] as usize] as usize;
        }
        if syn_error == 0 {
            return Some(0);
        }
        let mut lambda = [0u8; NROOTS + 1];
        lambda[0] = 1;
        if no_eras > 0 {
            lambda[1] = self.alpha_to[modnn(PRIM * (NN - 1 - eras_pos[0]))];
            for (i, &position) in eras_pos.iter().enumerate().skip(1) {
                let u = modnn(PRIM * (NN - 1 - position));
                for j in (1..=i + 1).rev() {
                    let tmp = self.index_of[lambda[j - 1] as usize] as usize;
                    if tmp != A0 {
                        lambda[j] ^= self.alpha_to[modnn(u + tmp)];
                    }
                }
            }
        }
        let mut b = [0usize; NROOTS + 1];
        for i in 0..=NROOTS {
            b[i] = self.index_of[lambda[i] as usize] as usize;
        }
        let mut r = no_eras;
        let mut el = no_eras;
        loop {
            r += 1;
            if r > NROOTS {
                break;
            }
            let mut discr_r = 0u8;
            for i in 0..r {
                if lambda[i] != 0 && s[r - i - 1] != A0 {
                    discr_r ^= self.alpha_to[modnn(self.index_of[lambda[i] as usize] as usize + s[r - i - 1])];
                }
            }
            let discr_r = self.index_of[discr_r as usize] as usize;
            if discr_r == A0 {
                b.copy_within(0..NROOTS, 1);
                b[0] = A0;
            } else {
                let mut t = [0u8; NROOTS + 1];
                t[0] = lambda[0];
                for i in 0..NROOTS {
                    t[i + 1] = if b[i] != A0 { lambda[i + 1] ^ self.alpha_to[modnn(discr_r + b[i])] } else { lambda[i + 1] };
                }
                if 2 * el < r + no_eras {
                    el = r + no_eras - el;
                    for i in 0..=NROOTS {
                        b[i] = if lambda[i] == 0 { A0 } else { modnn(self.index_of[lambda[i] as usize] as usize + NN - discr_r) };
                    }
                } else {
                    b.copy_within(0..NROOTS, 1);
                    b[0] = A0;
                }
                lambda = t;
            }
        }
        // lambda in forma logaritmica e suo grado
        let mut deg_lambda = 0usize;
        let mut lam = [0usize; NROOTS + 1];
        for i in 0..=NROOTS {
            lam[i] = self.index_of[lambda[i] as usize] as usize;
            if lam[i] != A0 {
                deg_lambda = i;
            }
        }
        // ricerca di Chien delle radici
        let mut reg = [0usize; NROOTS + 1];
        reg[1..].copy_from_slice(&lam[1..]);
        let mut root = [0usize; NROOTS];
        let mut loc = [0usize; NROOTS];
        let mut count = 0usize;
        let mut k = self.iprim - 1;
        for i in 1..=NN {
            let mut q = 1u8;
            for j in (1..=deg_lambda).rev() {
                if reg[j] != A0 {
                    reg[j] = modnn(reg[j] + j);
                    q ^= self.alpha_to[reg[j]];
                }
            }
            if q == 0 {
                root[count] = i;
                loc[count] = k;
                count += 1;
                if count == deg_lambda {
                    break;
                }
            }
            k = modnn(k + self.iprim);
        }
        if deg_lambda != count {
            return None;
        }
        // polinomio valutatore omega
        let deg_omega = deg_lambda.checked_sub(1)?;
        let mut omega = [0usize; NROOTS + 1];
        for i in 0..=deg_omega {
            let mut tmp = 0u8;
            for j in (0..=i).rev() {
                if s[i - j] != A0 && lam[j] != A0 {
                    tmp ^= self.alpha_to[modnn(s[i - j] + lam[j])];
                }
            }
            omega[i] = self.index_of[tmp as usize] as usize;
        }
        for j in (0..count).rev() {
            let mut num1 = 0u8;
            for i in (0..=deg_omega).rev() {
                if omega[i] != A0 {
                    num1 ^= self.alpha_to[modnn(omega[i] + i * root[j])];
                }
            }
            let num2 = self.alpha_to[modnn(root[j] * (FCR - 1) + NN)];
            let mut den = 0u8;
            let mut i = deg_lambda.min(NROOTS - 1) & !1;
            loop {
                if lam[i + 1] != A0 {
                    den ^= self.alpha_to[modnn(lam[i + 1] + i * root[j])];
                }
                if i < 2 {
                    break;
                }
                i -= 2;
            }
            if num1 != 0 && den != 0 {
                data[loc[j]] ^= self.alpha_to[modnn(self.index_of[num1 as usize] as usize + self.index_of[num2 as usize] as usize + NN - self.index_of[den as usize] as usize)];
            }
        }
        Some(count)
    }

    /// Dove va a finire, nell'ordine del decodificatore di Karn, il simbolo `p` della parola ricevuta (`recd0` di `rs_decode`): le posizioni delle
    /// cancellazioni vanno date in quell'ordine.
    pub fn karn_position(p: usize) -> usize {
        NN - 1 - p
    }

    /// Corregge 63 simboli gia' nell'ordine di Karn (l'ultimo simbolo trasmesso per primo) con le cancellazioni date (indici in quell'ordine):
    /// e' `decode_rs_int` usato direttamente dal decodificatore a decisione soft. `None` se non si riesce.
    pub fn decode_karn(&self, data: &mut [u8; NN], erasures: &[usize]) -> Option<usize> {
        self.decode_in_place(data, erasures)
    }

    /// `rs_decode` di `wrapkarn.c`: da 63 simboli ricevuti ai 12 di dati; `erasures` sono le posizioni cancellate **nell'ordine di Karn**
    /// ([`Codec::karn_position`]); `None` se i simboli sbagliati sono troppi. Ritorna anche quanti simboli sono stati corretti.
    pub fn decode(&self, recd0: &[u8; NN], erasures: &[usize]) -> Option<([u8; DATA], usize)> {
        let mut recd = [0u8; NN];
        for i in 0..DATA {
            recd[i] = recd0[NN - 1 - i];
        }
        for i in 0..NROOTS {
            recd[DATA + i] = recd0[NROOTS - 1 - i];
        }
        let nerr = self.decode_in_place(&mut recd, erasures)?;
        let mut out = [0u8; DATA];
        for i in 0..DATA {
            out[i] = recd[DATA - 1 - i];
        }
        Some((out, nerr))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_generator_makes_a_codeword_with_zero_syndromes() {
        let rs = Codec::new();
        let dgen = [61, 36, 45, 30, 3, 55, 3, 2, 14, 5, 33, 40];
        let sent = rs.encode(&dgen);
        // il tratto dei dati e' quello dato, in coda
        assert_eq!(&sent[NROOTS..], &dgen);
        let (decoded, nerr) = rs.decode(&sent, &[]).expect("parola valida");
        assert_eq!(decoded, dgen);
        assert_eq!(nerr, 0);
    }

    #[test]
    fn up_to_twenty_five_wrong_symbols_are_corrected() {
        let rs = Codec::new();
        let dgen = [61, 36, 45, 30, 3, 55, 3, 2, 14, 5, 33, 40];
        let sent = rs.encode(&dgen);
        let mut bad = sent;
        for i in 0..25 {
            bad[i * 2 + 1] ^= (i as u8 % 62) + 1;
        }
        let (decoded, nerr) = rs.decode(&bad, &[]).expect("25 errori si correggono");
        assert_eq!(decoded, dgen);
        assert_eq!(nerr, 25);
    }

    #[test]
    fn erasures_double_the_reach() {
        let rs = Codec::new();
        let dgen = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12];
        let sent = rs.encode(&dgen);
        let mut bad = sent;
        let mut erasures: Vec<usize> = Vec::new();
        for (p, b) in bad.iter_mut().enumerate().take(40) {
            *b = 0;
            erasures.push(Codec::karn_position(p));
        }
        let (decoded, _) = rs.decode(&bad, &erasures).expect("40 cancellazioni si recuperano");
        assert_eq!(decoded, dgen);
    }

    #[test]
    fn too_many_errors_are_refused() {
        let rs = Codec::new();
        let dgen = [9; 12];
        let sent = rs.encode(&dgen);
        let mut bad = sent;
        for b in bad.iter_mut().take(40) {
            *b ^= 21;
        }
        match rs.decode(&bad, &[]) {
            None => {}
            Some((decoded, _)) => assert_ne!(decoded, dgen, "con 40 errori non si puo' ritrovare il messaggio giusto per caso"),
        }
    }
}
