//! Pezzi comuni alle prove dei ricevitori RTTY, PSK e CW: generatore pseudocasuale con rumore gaussiano, testo da QSO, distanza fra testi,
//! fading CCIR, spostamento di frequenza, errore d'orologio della scheda e file WAV per riascoltare i casi con fldigi.
#![allow(dead_code)]

pub mod rtty_scenari;

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

/// Aggiunge rumore bianco gaussiano di deviazione `sigma`.
pub fn aggiungi_rumore(audio: &mut [f32], sigma: f64, rng: &mut Rng) {
    for s in audio.iter_mut() {
        *s += (sigma * rng.gauss()) as f32;
    }
}

/// Deviazione del rumore per un SNR in 3 kHz (a 12 kHz di campionamento) rispetto a una sinusoide di ampiezza `amp`: potenza del segnale
/// amp^2/2, rumore di varianza s^2 su 6 kHz -> s^2/2 in 3 kHz (la stessa misura della prova RTTY, di fldigi e di MMTTY).
pub fn sigma_per_snr(amp: f64, snr_db: f64) -> f64 {
    amp / 10f64.powf(snr_db / 20.0)
}

/// Segnale analitico (parte reale ritardata e trasformata di Hilbert) con un FIR di 127 coefficienti a finestra di Hamming: serve per
/// spostare in frequenza o far evanescere un segnale reale a banda stretta.
pub fn analitico(x: &[f32]) -> Vec<(f64, f64)> {
    const HALF: usize = 63;
    let taps: Vec<f64> = (0..=2 * HALF)
        .map(|k| {
            let n = k as i64 - HALF as i64;
            if n % 2 == 0 {
                0.0
            } else {
                let w = 0.54 - 0.46 * (2.0 * std::f64::consts::PI * k as f64 / (2 * HALF) as f64).cos();
                2.0 / (std::f64::consts::PI * n as f64) * w
            }
        })
        .collect();
    (0..x.len())
        .map(|i| {
            let re = if i >= HALF { f64::from(x[i - HALF]) } else { 0.0 };
            let mut im = 0.0;
            for (k, &h) in taps.iter().enumerate() {
                if h != 0.0 && i >= k {
                    im += h * f64::from(x[i - k]);
                }
            }
            (re, im)
        })
        .collect()
}

/// Moltiplica il segnale (reale, a banda stretta) per un guadagno complesso che cambia nel tempo: `g(n)` = (reale, immaginario). Con
/// `g = e^(j fase)` sposta la frequenza, con un processo gaussiano complesso fa il fading. Il risultato e' in ritardo di 63 campioni.
pub fn guadagno_complesso(x: &[f32], g: impl Fn(usize) -> (f64, f64)) -> Vec<f32> {
    analitico(x)
        .into_iter()
        .enumerate()
        .map(|(n, (re, im))| {
            let (gr, gi) = g(n);
            (re * gr - im * gi) as f32
        })
        .collect()
}

/// Un percorso di fading di Rayleigh (modello di Watterson, CCIR 520): guadagno complesso gaussiano con spettro Doppler gaussiano di
/// larghezza `spread_hz` (2 sigma), potenza media 1, un valore ogni campione a 12 kHz (calcolato a 100 Hz e interpolato).
pub fn percorso_rayleigh(len: usize, spread_hz: f64, rng: &mut Rng) -> Vec<(f64, f64)> {
    const STEP: usize = 120;
    let n_slow = len / STEP + 3;
    // filtro gaussiano sui campioni lenti (100 Hz): sigma nel tempo = 1 / (2 pi sigma_f), sigma_f = spread / 2
    let sigma_f = (spread_hz / 2.0).max(0.01);
    let sigma_t = 100.0 / (2.0 * std::f64::consts::PI * sigma_f);
    let half = (4.0 * sigma_t).ceil() as usize;
    let kernel: Vec<f64> = (0..=2 * half).map(|k| (-((k as f64 - half as f64) / sigma_t).powi(2) / 2.0).exp()).collect();
    let white: Vec<(f64, f64)> = (0..n_slow + 2 * half).map(|_| (rng.gauss(), rng.gauss())).collect();
    let mut slow: Vec<(f64, f64)> = (0..n_slow)
        .map(|i| kernel.iter().enumerate().fold((0.0, 0.0), |acc, (k, &h)| (acc.0 + h * white[i + k].0, acc.1 + h * white[i + k].1)))
        .collect();
    let p = slow.iter().map(|(a, b)| a * a + b * b).sum::<f64>() / slow.len() as f64;
    let norm = p.sqrt().max(1e-12);
    for v in slow.iter_mut() {
        v.0 /= norm;
        v.1 /= norm;
    }
    (0..len)
        .map(|n| {
            let i = n / STEP;
            let f = (n % STEP) as f64 / STEP as f64;
            let (a, b) = (slow[i], slow[i + 1]);
            (a.0 + (b.0 - a.0) * f, a.1 + (b.1 - a.1) * f)
        })
        .collect()
}

/// Fading CCIR su due percorsi di uguale potenza (Watterson): il secondo in ritardo di `delay_ms`, ognuno con il suo Doppler.
pub fn fading_ccir(x: &[f32], spread_hz: f64, delay_ms: f64, rng: &mut Rng) -> Vec<f32> {
    let d = (delay_ms * 12.0).round() as usize;
    let a = analitico(x);
    let p1 = percorso_rayleigh(x.len(), spread_hz, rng);
    let p2 = percorso_rayleigh(x.len(), spread_hz, rng);
    let k = std::f64::consts::FRAC_1_SQRT_2;
    (0..x.len())
        .map(|n| {
            let (re1, im1) = a[n];
            let (re2, im2) = if n >= d { a[n - d] } else { (0.0, 0.0) };
            let y1 = re1 * p1[n].0 - im1 * p1[n].1;
            let y2 = re2 * p2[n].0 - im2 * p2[n].1;
            (k * (y1 + y2)) as f32
        })
        .collect()
}

/// Ricampiona con un errore d'orologio della scheda audio (`ppm` parti per milione, positivo = la scheda corre piu' lenta del vero e il
/// segnale si allunga): interpolazione lineare, sufficiente per segnali sotto i 3 kHz a 12 kHz.
pub fn errore_orologio(x: &[f32], ppm: f64) -> Vec<f32> {
    let step = 1.0 / (1.0 + ppm * 1e-6);
    let n_out = ((x.len() - 1) as f64 / step) as usize;
    (0..n_out)
        .map(|n| {
            let t = n as f64 * step;
            let i = t.floor() as usize;
            let f = (t - i as f64) as f32;
            let b = x.get(i + 1).copied().unwrap_or(0.0);
            x[i] * (1.0 - f) + b * f
        })
        .collect()
}

/// Scrive un WAV mono a 16 bit (per riascoltare un caso con fldigi). Solo se la variabile `AURORA_WAV` indica una cartella.
pub fn salva_wav(nome: &str, rate: u32, x: &[f32]) {
    let Some(dir) = std::env::var_os("AURORA_WAV") else { return };
    let path = std::path::Path::new(&dir).join(format!("{nome}.wav"));
    let data_len = (x.len() * 2) as u32;
    let mut b = Vec::with_capacity(44 + x.len() * 2);
    b.extend_from_slice(b"RIFF");
    b.extend_from_slice(&(36 + data_len).to_le_bytes());
    b.extend_from_slice(b"WAVEfmt ");
    b.extend_from_slice(&16u32.to_le_bytes());
    b.extend_from_slice(&1u16.to_le_bytes());
    b.extend_from_slice(&1u16.to_le_bytes());
    b.extend_from_slice(&rate.to_le_bytes());
    b.extend_from_slice(&(rate * 2).to_le_bytes());
    b.extend_from_slice(&2u16.to_le_bytes());
    b.extend_from_slice(&16u16.to_le_bytes());
    b.extend_from_slice(b"data");
    b.extend_from_slice(&data_len.to_le_bytes());
    for &s in x {
        b.extend_from_slice(&((s.clamp(-1.0, 1.0) * 32767.0) as i16).to_le_bytes());
    }
    if let Err(e) = std::fs::write(&path, b) {
        eprintln!("WAV non scritto ({}): {e}", path.display());
    }
}

/// Da 12 a 8 kHz: interpolazione con un seno cardinale a finestra (Blackman, ±32 campioni) e taglio a 3,6 kHz.
pub fn a_8k(x: &[f32]) -> Vec<f32> {
    const HALF: i64 = 32;
    let fc = 3600.0 / 12_000.0;
    let n_out = (x.len() as f64 * 8_000.0 / 12_000.0) as usize;
    (0..n_out)
        .map(|n| {
            let t = n as f64 * 12_000.0 / 8_000.0;
            let k0 = t.floor() as i64;
            let mut acc = 0.0;
            for k in (k0 - HALF + 1)..=(k0 + HALF) {
                if k < 0 || k as usize >= x.len() {
                    continue;
                }
                let d = t - k as f64;
                let sinc = if d.abs() < 1e-12 { 2.0 * fc } else { (2.0 * std::f64::consts::PI * fc * d).sin() / (std::f64::consts::PI * d) };
                let w = 0.42 + 0.5 * (std::f64::consts::PI * d / HALF as f64).cos() + 0.08 * (2.0 * std::f64::consts::PI * d / HALF as f64).cos();
                acc += f64::from(x[k as usize]) * sinc * w;
            }
            acc as f32
        })
        .collect()
}
