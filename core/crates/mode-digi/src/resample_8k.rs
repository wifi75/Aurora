//! I due ricampionatori fra i 12 kHz del programma e gli 8 kHz di Olivia e Contestia: il decimatore 3:2 (12 -> 8 kHz, ricezione) e
//! l'interpolatore 2:3 (8 -> 12 kHz, trasmissione).
//!
//! Tutti e due passano dalla frequenza comune di 24 kHz: si inseriscono `up - 1` zeri fra un campione e l'altro, si filtra con un FIR
//! passa-basso simmetrico (fase lineare: ritardo uguale per ogni frequenza) e si tiene un campione ogni `down`. Il filtro e' un seno
//! cardinale con finestra di Blackman, taglio a 3700 Hz (sotto i 4 kHz di Nyquist degli 8 kHz, sopra il bordo piu' alto dei segnali di
//! Olivia). Il calcolo e' polifase (si moltiplicano solo i campioni veri, non gli zeri) e a flusso: lo stato resta fra un blocco e l'altro,
//! cosi' l'uscita a pezzi e' identica a quella in un colpo solo.

/// La frequenza intermedia comune.
const MID_RATE: f64 = 24_000.0;
/// Il taglio del filtro (Hz).
const CUTOFF_HZ: f64 = 3_700.0;
/// Le prese del filtro a 24 kHz (dispari: il centro cade su una presa).
const TAPS: usize = 241;

/// Il filtro passa-basso a 24 kHz, con guadagno `gain` in continua.
fn design(gain: f64) -> Vec<f32> {
    let mid = (TAPS - 1) as f64 / 2.0;
    let fc = CUTOFF_HZ / MID_RATE;
    let mut h: Vec<f64> = (0..TAPS)
        .map(|i| {
            let t = i as f64 - mid;
            let sinc = if t == 0.0 { 2.0 * fc } else { (2.0 * std::f64::consts::PI * fc * t).sin() / (std::f64::consts::PI * t) };
            let x = 2.0 * std::f64::consts::PI * i as f64 / (TAPS - 1) as f64;
            let w = 0.42 - 0.5 * x.cos() + 0.08 * (2.0 * x).cos();
            sinc * w
        })
        .collect();
    let sum: f64 = h.iter().sum();
    for v in &mut h {
        *v *= gain / sum;
    }
    h.into_iter().map(|v| v as f32).collect()
}

/// Ricampionatore razionale `up/down` a flusso.
struct Rational {
    up: usize,
    down: usize,
    taps: Vec<f32>,
    /// Gli ultimi campioni d'ingresso (abbastanza per coprire il filtro); `base` e' l'indice assoluto del primo.
    hist: Vec<f32>,
    base: usize,
    /// Campioni d'ingresso arrivati in tutto.
    total_in: usize,
    /// L'indice (alla frequenza intermedia) della prossima uscita.
    next: usize,
}

impl Rational {
    fn new(up: usize, down: usize) -> Rational {
        // guadagno `up`: gli zeri inseriti tolgono energia
        Rational { up, down, taps: design(up as f64), hist: Vec::new(), base: 0, total_in: 0, next: 0 }
    }

    /// Il campione d'ingresso di indice assoluto `i` (prima dell'inizio vale zero).
    fn at(&self, i: isize) -> f32 {
        if i < 0 {
            return 0.0;
        }
        let i = i as usize;
        if i < self.base { 0.0 } else { self.hist.get(i - self.base).copied().unwrap_or(0.0) }
    }

    fn process(&mut self, input: &[f32], out: &mut Vec<f32>) {
        self.hist.extend_from_slice(input);
        self.total_in += input.len();
        // l'uscita `n` (a 24 kHz) usa gli ingressi fino a `n / up`: si calcola solo quando quello e' arrivato
        while self.next / self.up < self.total_in {
            let n = self.next;
            // le prese che cadono su un campione vero: j con (n - j) multiplo di `up`
            let mut acc = 0.0f32;
            let mut j = n % self.up;
            while j < self.taps.len() {
                let idx = (n as isize - j as isize) / self.up as isize;
                if (n as isize - j as isize) < 0 {
                    break;
                }
                acc += self.taps[j] * self.at(idx);
                j += self.up;
            }
            out.push(acc);
            self.next += self.down;
        }
        // si tiene solo quanto serve al filtro
        let keep = self.taps.len() / self.up + 2;
        if self.hist.len() > keep {
            let drop = self.hist.len() - keep;
            self.hist.drain(..drop);
            self.base += drop;
        }
    }

    /// Il ritardo del filtro in campioni d'ingresso.
    fn delay_in(&self) -> f64 {
        (self.taps.len() - 1) as f64 / 2.0 / self.up as f64
    }
}

/// Il decimatore 3:2: 12 kHz in, 8 kHz out.
pub struct Decimator(Rational);

impl Default for Decimator {
    fn default() -> Self {
        Decimator::new()
    }
}

impl Decimator {
    pub fn new() -> Decimator {
        Decimator(Rational::new(2, 3))
    }

    /// Aggiunge in `out` i campioni a 8 kHz che `input` (a 12 kHz) completa.
    pub fn process(&mut self, input: &[f32], out: &mut Vec<f32>) {
        self.0.process(input, out);
    }

    /// Il ritardo in campioni d'uscita (8 kHz).
    pub fn delay_out(&self) -> f64 {
        self.0.delay_in() * 2.0 / 3.0
    }
}

/// L'interpolatore 2:3: 8 kHz in, 12 kHz out.
pub struct Interpolator(Rational);

impl Default for Interpolator {
    fn default() -> Self {
        Interpolator::new()
    }
}

impl Interpolator {
    pub fn new() -> Interpolator {
        Interpolator(Rational::new(3, 2))
    }

    /// Aggiunge in `out` i campioni a 12 kHz che `input` (a 8 kHz) completa.
    pub fn process(&mut self, input: &[f32], out: &mut Vec<f32>) {
        self.0.process(input, out);
    }

    /// La coda del filtro: gli ultimi campioni ancora dentro escono spinti da zeri.
    pub fn flush(&mut self, out: &mut Vec<f32>) {
        let n = self.0.delay_in().ceil() as usize + 1;
        self.0.process(&vec![0.0; n], out);
    }

    /// Il ritardo in campioni d'uscita (12 kHz).
    pub fn delay_out(&self) -> f64 {
        self.0.delay_in() * 1.5
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f64::consts::PI;

    fn tone(hz: f64, rate: f64, n: usize) -> Vec<f32> {
        (0..n).map(|i| (2.0 * PI * hz * i as f64 / rate).sin() as f32).collect()
    }

    /// Ampiezza e fase del tono `hz` in `x` (campionato a `rate`), dal campione `from` in poi (correlazione).
    fn measure(x: &[f32], hz: f64, rate: f64, from: usize) -> (f64, f64) {
        let (mut re, mut im) = (0.0, 0.0);
        let n = x.len() - from;
        for (i, &v) in x.iter().enumerate().skip(from) {
            let a = 2.0 * PI * hz * i as f64 / rate;
            re += f64::from(v) * a.cos();
            im += f64::from(v) * a.sin();
        }
        (2.0 * (re * re + im * im).sqrt() / n as f64, im.atan2(re))
    }

    #[test]
    fn le_lunghezze_tornano() {
        let mut d = Decimator::new();
        let mut out = Vec::new();
        d.process(&vec![0.0; 12_000], &mut out);
        assert_eq!(out.len(), 8_000);
        let mut i = Interpolator::new();
        let mut out = Vec::new();
        i.process(&vec![0.0; 8_000], &mut out);
        assert_eq!(out.len(), 12_000);
    }

    #[test]
    fn a_pezzi_e_come_in_un_colpo_solo() {
        let x = tone(1234.0, 12_000.0, 9_001);
        let mut one = Vec::new();
        Decimator::new().process(&x, &mut one);
        let mut d = Decimator::new();
        let mut parts = Vec::new();
        let mut rest = &x[..];
        for size in [1001usize, 7, 1, 2, 3, 500].iter().cycle() {
            if rest.is_empty() {
                break;
            }
            let n = (*size).min(rest.len());
            d.process(&rest[..n], &mut parts);
            rest = &rest[n..];
        }
        assert_eq!(one, parts);
        let y = tone(777.0, 8_000.0, 5_003);
        let mut one = Vec::new();
        Interpolator::new().process(&y, &mut one);
        let mut it = Interpolator::new();
        let mut parts = Vec::new();
        for c in y.chunks(13) {
            it.process(c, &mut parts);
        }
        assert_eq!(one, parts);
    }

    /// In banda (200-3300 Hz) il guadagno e' piatto entro 0,1 dB e il ritardo e' lo stesso per ogni frequenza (fase lineare); oltre i 4 kHz
    /// il decimatore taglia.
    #[test]
    fn risposta_in_banda_piatta_e_ritardo_costante() {
        let d0 = Decimator::new().delay_out();
        for hz in [200.0, 500.0, 1000.0, 1500.0, 2000.0, 2500.0, 3000.0, 3300.0] {
            let mut out = Vec::new();
            Decimator::new().process(&tone(hz, 12_000.0, 24_000), &mut out);
            let (amp, ph) = measure(&out, hz, 8_000.0, 1_000);
            assert!((20.0 * amp.log10()).abs() < 0.1, "decimatore {hz} Hz: {amp}");
            // la fase misurata e' quella del tono ritardato di `d0` campioni
            let expected = 2.0 * PI * hz * d0 / 8_000.0 + PI / 2.0;
            let err = (ph - expected).rem_euclid(2.0 * PI);
            let err = err.min(2.0 * PI - err);
            assert!(err < 0.02, "decimatore {hz} Hz: errore di fase {err}");

            let it = Interpolator::new();
            let d1 = it.delay_out();
            let mut out = Vec::new();
            let mut it = it;
            it.process(&tone(hz, 8_000.0, 16_000), &mut out);
            let (amp, ph) = measure(&out, hz, 12_000.0, 1_500);
            assert!((20.0 * amp.log10()).abs() < 0.1, "interpolatore {hz} Hz: {amp}");
            let expected = 2.0 * PI * hz * d1 / 12_000.0 + PI / 2.0;
            let err = (ph - expected).rem_euclid(2.0 * PI);
            let err = err.min(2.0 * PI - err);
            assert!(err < 0.02, "interpolatore {hz} Hz: errore di fase {err}");
        }
        // 5 kHz a 12 kHz cadrebbe a 3 kHz dopo la decimazione: il filtro lo toglie
        let mut out = Vec::new();
        Decimator::new().process(&tone(5_000.0, 12_000.0, 24_000), &mut out);
        let (amp, _) = measure(&out, 3_000.0, 8_000.0, 1_000);
        assert!(20.0 * amp.log10() < -50.0, "alias: {amp}");
    }

    /// Andata e ritorno 8 -> 12 -> 8 kHz: lo stesso segnale, ritardato.
    #[test]
    fn andata_e_ritorno() {
        let x = tone(1500.0, 8_000.0, 8_000);
        let mut up = Vec::new();
        let mut it = Interpolator::new();
        it.process(&x, &mut up);
        it.flush(&mut up);
        let mut back = Vec::new();
        Decimator::new().process(&up, &mut back);
        let delay = (Interpolator::new().delay_out() / 1.5 + Decimator::new().delay_out()).round() as usize;
        let err = (1000..7000).map(|i| (back[i + delay] - x[i]).abs()).fold(0.0f32, f32::max);
        assert!(err < 0.01, "errore {err}");
    }
}
