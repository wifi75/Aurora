//! Pezzi comuni alle prove del ricevitore RTTY: generatore pseudocasuale con rumore gaussiano, testo da QSO, distanza fra testi.
#![allow(dead_code)]

/// Generatore pseudocasuale (xorshift64*) e rumore gaussiano (Box-Muller): niente dipendenze in piu'.
pub struct Rng(pub u64);

impl Rng {
    pub fn next_u64(&mut self) -> u64 {
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        self.0.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }
    pub fn uniform(&mut self) -> f64 {
        ((self.next_u64() >> 11) as f64 + 0.5) / (1u64 << 53) as f64
    }
    pub fn gauss(&mut self) -> f64 {
        let (u1, u2) = (self.uniform(), self.uniform());
        (-2.0 * u1.ln()).sqrt() * (2.0 * std::f64::consts::PI * u2).cos()
    }
}

/// Un testo da QSO con lettere, cifre e spazi (cambi di registro frequenti).
pub fn testo(rng: &mut Rng, chars: usize) -> String {
    const PAROLE: [&str; 24] = [
        "CQ", "DE", "IU3ABC", "TEST", "599", "73", "TNX", "FER", "QSO", "UR", "RST", "NAME", "QTH", "IO91WM", "K", "PSE", "BK", "14080", "RIG", "ANT",
        "DIPOLE", "100W", "HW", "CPY",
    ];
    let mut s = String::new();
    while s.len() < chars {
        let w = PAROLE[(rng.next_u64() % PAROLE.len() as u64) as usize];
        s.push_str(w);
        s.push(' ');
    }
    s
}

pub fn levenshtein(a: &[u8], b: &[u8]) -> usize {
    let mut prev: Vec<usize> = (0..=b.len()).collect();
    let mut cur = vec![0; b.len() + 1];
    for (i, &ca) in a.iter().enumerate() {
        cur[0] = i + 1;
        for (j, &cb) in b.iter().enumerate() {
            cur[j + 1] = (prev[j] + usize::from(ca != cb)).min(prev[j + 1] + 1).min(cur[j] + 1);
        }
        std::mem::swap(&mut prev, &mut cur);
    }
    prev[b.len()]
}
