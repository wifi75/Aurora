//! Ricampionatore dai 12 kHz dell'onda alla frequenza del dispositivo: filtro sinc con finestra di Hann
//! (passa-basso a 0,45 della frequenza piu' bassa delle due), a fasi precalcolate.
//!
//! L'uscita e' allineata all'ingresso (nessun ritardo di gruppo) e ha lunghezza `len * out / in`.

use std::f64::consts::PI;

use crate::error::TxError;

fn gcd(a: u32, b: u32) -> u32 {
    if b == 0 { a } else { gcd(b, a % b) }
}

/// Peso del filtro a distanza `t` campioni di ingresso dal centro.
fn kernel(t: f64, half: usize, cutoff: f64) -> f64 {
    let half = half as f64;
    if t.abs() >= half {
        return 0.0;
    }
    let window = 0.5 * (1.0 + (PI * t / half).cos());
    let x = 2.0 * PI * cutoff * t;
    let sinc = if x.abs() < 1e-12 { 1.0 } else { x.sin() / x };
    2.0 * cutoff * sinc * window
}

/// I pesi del filtro: una riga per fase, normalizzata a guadagno 1 in continua.
struct Polyphase {
    /// Fasi (interpolazione) e salto in ingresso per ogni campione in uscita.
    p: usize,
    q: usize,
    half: usize,
    taps: usize,
    table: Vec<f32>,
}

impl Polyphase {
    fn new(in_rate: u32, out_rate: u32) -> Polyphase {
        let g = gcd(in_rate, out_rate);
        let p = (out_rate / g) as usize;
        let q = (in_rate / g) as usize;
        let ratio = f64::from(in_rate) / f64::from(out_rate);
        let cutoff = 0.5 * 0.9 / ratio.max(1.0);
        let half = (16.0 * ratio.max(1.0)).ceil() as usize;
        let taps = 2 * half;
        let mut table = vec![0.0f32; p * taps];
        let mut row = vec![0.0f64; taps];
        for (phase, out_row) in table.chunks_mut(taps).enumerate() {
            let frac = phase as f64 / p as f64;
            for (j, w) in row.iter_mut().enumerate() {
                *w = kernel(j as f64 - half as f64 + 1.0 - frac, half, cutoff);
            }
            let sum: f64 = row.iter().sum();
            let norm = if sum.abs() > 1e-9 { 1.0 / sum } else { 1.0 };
            for (dst, w) in out_row.iter_mut().zip(&row) {
                *dst = (w * norm) as f32;
            }
        }
        Polyphase { p, q, half, taps, table }
    }
}

/// Il ricampionatore a flusso (trasmissione dei modi testuali, che arriva a pezzi): lo stesso filtro di [`resample`], con la memoria fra un
/// pezzo e l'altro, per cui i pezzi messi in fila danno la stessa uscita dell'onda intera. Ogni pezzo trattiene gli ultimi campioni che
/// servono al filtro finche' non arriva il seguente (o `flush`).
pub struct StreamResampler {
    poly: Option<Polyphase>,
    /// I campioni d'ingresso ancora utili; `base` e' l'indice (dall'inizio) del primo.
    buf: Vec<f32>,
    base: u64,
    /// Campioni d'ingresso arrivati e prossimo campione d'uscita.
    total_in: u64,
    next_out: u64,
}

impl StreamResampler {
    pub fn new(in_rate: u32, out_rate: u32) -> Result<StreamResampler, TxError> {
        if in_rate == 0 || out_rate == 0 {
            return Err(TxError::Audio("frequenza di campionamento nulla".into()));
        }
        let poly = (in_rate != out_rate).then(|| Polyphase::new(in_rate, out_rate));
        Ok(StreamResampler { poly, buf: Vec::new(), base: 0, total_in: 0, next_out: 0 })
    }

    /// Un pezzo nuovo: restituisce i campioni d'uscita che si possono gia' calcolare.
    pub fn push(&mut self, input: &[f32]) -> Vec<f32> {
        if self.poly.is_none() {
            return input.to_vec();
        }
        self.buf.extend_from_slice(input);
        self.total_in += input.len() as u64;
        self.run(self.total_in)
    }

    /// Fine dell'onda: i campioni trattenuti (dopo l'ultimo campione il filtro vede silenzio, come [`resample`]).
    pub fn flush(&mut self) -> Vec<f32> {
        let Some(half) = self.poly.as_ref().map(|p| p.half) else { return Vec::new() };
        let end = self.total_in;
        self.buf.extend(std::iter::repeat_n(0.0, half + 1));
        let mut out = self.run(end + half as u64 + 1);
        let Some(poly) = &self.poly else { return out };
        // si fermano dove si ferma `resample`: `len * p / q` campioni
        let limit = (u128::from(end) * poly.p as u128 / poly.q as u128) as u64;
        let produced_before = self.next_out - out.len() as u64;
        out.truncate(limit.saturating_sub(produced_before) as usize);
        out
    }

    /// I campioni d'uscita il cui filtro sta tutto prima di `avail` (campioni d'ingresso disponibili).
    fn run(&mut self, avail: u64) -> Vec<f32> {
        let Some(poly) = &self.poly else { return Vec::new() };
        let (p, q, half, taps) = (poly.p as u64, poly.q as u64, poly.half as u64, poly.taps);
        let mut out = Vec::new();
        loop {
            let num = self.next_out * q;
            let centre = num / p;
            if centre + half >= avail {
                break;
            }
            let weights = &poly.table[((num % p) as usize) * taps..][..taps];
            let first = centre as i64 - half as i64 + 1;
            let mut acc = 0.0f32;
            for (j, w) in weights.iter().enumerate() {
                let idx = first + j as i64;
                if idx >= self.base as i64 && ((idx - self.base as i64) as usize) < self.buf.len() {
                    acc += self.buf[(idx - self.base as i64) as usize] * w;
                }
            }
            out.push(acc);
            self.next_out += 1;
        }
        // si tengono solo i campioni che servono ancora al filtro
        let keep_from = ((self.next_out * q / p) as i64 - half as i64 + 1).max(self.base as i64) as u64;
        let drop = ((keep_from - self.base) as usize).min(self.buf.len());
        self.buf.drain(..drop);
        self.base += drop as u64;
        out
    }
}

/// Porta `input` da `in_rate` a `out_rate` (Hz).
pub fn resample(input: &[f32], in_rate: u32, out_rate: u32) -> Result<Vec<f32>, TxError> {
    if in_rate == 0 || out_rate == 0 {
        return Err(TxError::Audio("frequenza di campionamento nulla".into()));
    }
    if in_rate == out_rate {
        return Ok(input.to_vec());
    }
    let Polyphase { p, q, half, taps, table } = Polyphase::new(in_rate, out_rate);

    let out_len = (input.len() as u128 * p as u128 / q as u128) as usize;
    let mut out = Vec::with_capacity(out_len);
    for m in 0..out_len {
        let num = m * q;
        let centre = num / p;
        let weights = &table[(num % p) * taps..][..taps];
        let first = centre as isize - half as isize + 1;
        let mut acc = 0.0f32;
        if first >= 0 && first as usize + taps <= input.len() {
            for (x, w) in input[first as usize..][..taps].iter().zip(weights) {
                acc += x * w;
            }
        } else {
            for (j, w) in weights.iter().enumerate() {
                let idx = first + j as isize;
                if idx >= 0 && (idx as usize) < input.len() {
                    acc += input[idx as usize] * w;
                }
            }
        }
        out.push(acc);
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tone(rate: u32, freq: f64, seconds: f64) -> Vec<f32> {
        (0..(f64::from(rate) * seconds) as usize).map(|n| (2.0 * PI * freq * n as f64 / f64::from(rate)).sin() as f32).collect()
    }

    fn rms(x: &[f32]) -> f64 {
        (x.iter().map(|&v| f64::from(v) * f64::from(v)).sum::<f64>() / x.len() as f64).sqrt()
    }

    fn crossings(x: &[f32]) -> usize {
        x.windows(2).filter(|w| w[0] < 0.0 && w[1] >= 0.0).count()
    }

    #[test]
    fn same_rate_passes_through() {
        assert_eq!(resample(&[0.5, -0.5, 0.25], 12_000, 12_000), Ok(vec![0.5, -0.5, 0.25]));
        assert!(resample(&[0.0], 0, 48_000).is_err());
    }

    #[test]
    fn upsampling_keeps_length_level_and_frequency() {
        for out_rate in [48_000u32, 44_100, 96_000, 192_000] {
            let input = tone(12_000, 1500.0, 2.0);
            let out = resample(&input, 12_000, out_rate).expect("ricampionamento");
            assert_eq!(out.len(), 2 * out_rate as usize, "{out_rate}");
            let body = &out[2000..out.len() - 2000];
            assert!((rms(body) - std::f64::consts::FRAC_1_SQRT_2).abs() < 0.01, "{out_rate}: rms {}", rms(body));
            // 1500 Hz per 2 s = 3000 cicli: la frequenza non cambia
            assert!(crossings(&out).abs_diff(3000) <= 2, "{out_rate}: {} cicli", crossings(&out));
        }
    }

    #[test]
    fn a_tone_above_the_new_nyquist_is_removed_when_downsampling() {
        let out = resample(&tone(48_000, 9_000.0, 1.0), 48_000, 12_000).expect("ricampionamento");
        assert_eq!(out.len(), 12_000);
        assert!(rms(&out[500..out.len() - 500]) < 0.02, "rms {}", rms(&out));
    }

    #[test]
    fn the_stream_gives_the_same_samples_as_the_whole_wave() {
        for out_rate in [48_000u32, 44_100, 12_000, 8_000] {
            let input = tone(12_000, 1500.0, 1.0);
            let whole = resample(&input, 12_000, out_rate).expect("ricampionamento");
            let mut rs = StreamResampler::new(12_000, out_rate).expect("ricampionatore");
            let mut out = Vec::new();
            for piece in input.chunks(997) {
                out.extend(rs.push(piece));
            }
            out.extend(rs.flush());
            assert_eq!(out.len(), whole.len(), "{out_rate}");
            assert!(out.iter().zip(&whole).all(|(a, b)| (a - b).abs() < 1e-5), "{out_rate}");
        }
    }

    #[test]
    fn dc_gain_is_unity_for_any_phase() {
        let out = resample(&vec![1.0f32; 3000], 12_000, 44_100).expect("ricampionamento");
        for v in &out[200..out.len() - 200] {
            assert!((v - 1.0).abs() < 1e-3, "{v}");
        }
    }
}
