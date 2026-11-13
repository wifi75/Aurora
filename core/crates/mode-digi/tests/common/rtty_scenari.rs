//! Scenari comuni delle prove del ricevitore RTTY (passo 1 di `docs/progetto-rtty-oltre.md`): lo stesso audio a 12 kHz per Aurora, fldigi e
//! MMTTY, con il rapporto segnale/rumore misurato in 3 kHz (come fldigi e MMTTY) sulla potenza media del segnale voluto. Ogni scenario e'
//! deterministico dato il seme: le tabelle si ripetono uguali.
//!
//! Misure: tasso di caratteri sbagliati (CER, distanza di Levenshtein divisa per la lunghezza del testo mandato), soglia S/N al 5 e al 10 %
//! (interpolata su una griglia di S/N), caratteri inventati sul solo rumore.

use super::{Rng, aggiungi_rumore, errore_orologio, fading_ccir, levenshtein, sigma_per_snr, testo};
use mode_digi::psk::{self, PskMode};
use mode_digi::rtty::{self, RttyParams};

pub const RATE: f64 = 12_000.0;
/// Ampiezza di picco del segnale voluto.
pub const AMP: f64 = 0.3;
/// Centro dei toni del segnale voluto (Hz).
pub const CENTRE: f32 = 1500.0;

#[derive(Clone, Copy, Debug)]
pub enum Disturbo {
    /// Solo rumore bianco gaussiano.
    Awgn,
    /// Il trasmettitore fuori di questi Hz (AFC spento).
    Fuori(f32),
    /// Lo space attenuato di questi dB (fading selettivo, filtro della radio inclinato).
    SpaceGiu(f64),
    /// Una stazione RTTY a `df` Hz dal centro, `db` dB rispetto alla voluta.
    Qrm { df: f32, db: f64 },
    /// Una portante fissa a `df` Hz dal centro, `db` dB rispetto alla voluta.
    Portante { df: f32, db: f64 },
    /// Un segnale PSK31 a `df` Hz dal centro, `db` dB rispetto alla voluta.
    Psk { df: f32, db: f64 },
    /// Fading di Watterson su due percorsi (CCIR 520 / ITU-R F.1487): Doppler (Hz) e ritardo (ms).
    Fading { spread: f64, delay_ms: f64 },
    /// Errore d'orologio della scheda audio (parti per milione).
    Ppm(f64),
    /// Stop lungo questi bit (1; 1,42; 2) invece di 1,5.
    Stop(f64),
    /// Scariche (QRN): `per_s` impulsi al secondo, ampiezza `db` dB sopra il picco del segnale.
    Impulsi { per_s: f64, db: f64 },
    /// Filtro della radio largo `bw` Hz, centrato sui toni.
    FiltroRadio(f64),
    /// Audio tagliato: segnale piu' rumore ritagliato a questa frazione del picco del segnale.
    Tagliato(f64),
    /// AGC della radio che pompa: guadagno che scende di 15 dB e risale ogni 2 secondi (segnale e rumore insieme).
    Agc,
}

#[derive(Clone, Copy, Debug)]
pub struct Scenario {
    pub nome: &'static str,
    pub shift: f32,
    pub baud: f32,
    pub disturbo: Disturbo,
}

impl Scenario {
    pub const fn new(nome: &'static str, shift: f32, baud: f32, disturbo: Disturbo) -> Self {
        Self { nome, shift, baud, disturbo }
    }

    pub fn params(&self, centre: f32) -> RttyParams {
        RttyParams { centre_hz: centre, shift_hz: self.shift, baud: self.baud, reverse: false }
    }
}

/// Gli scenari della tabella di confronto (passo 3).
pub fn scenari() -> Vec<Scenario> {
    use Disturbo::*;
    vec![
        Scenario::new("rumore 170/45", 170.0, 45.45, Awgn),
        Scenario::new("rumore 170/75", 170.0, 75.0, Awgn),
        Scenario::new("rumore 425/50", 425.0, 50.0, Awgn),
        Scenario::new("rumore 850/45", 850.0, 45.45, Awgn),
        Scenario::new("rumore 85/45", 85.0, 45.45, Awgn),
        Scenario::new("fuori +10 Hz", 170.0, 45.45, Fuori(10.0)),
        Scenario::new("space -12 dB", 170.0, 45.45, SpaceGiu(-12.0)),
        Scenario::new("CCIR buono", 170.0, 45.45, Fading { spread: 0.1, delay_ms: 0.5 }),
        Scenario::new("CCIR medio", 170.0, 45.45, Fading { spread: 0.5, delay_ms: 1.0 }),
        Scenario::new("CCIR cattivo", 170.0, 45.45, Fading { spread: 1.0, delay_ms: 2.0 }),
        Scenario::new("flutter 10 Hz", 170.0, 45.45, Fading { spread: 10.0, delay_ms: 0.5 }),
        Scenario::new("alte latitudini", 170.0, 45.45, Fading { spread: 30.0, delay_ms: 7.0 }),
        Scenario::new("QRM a 250 Hz +6", 170.0, 45.45, Qrm { df: 250.0, db: 6.0 }),
        Scenario::new("QRM a 120 Hz -6", 170.0, 45.45, Qrm { df: 120.0, db: -6.0 }),
        Scenario::new("portante +30 Hz", 170.0, 45.45, Portante { df: 115.0, db: 0.0 }),
        Scenario::new("PSK31 a 150 Hz", 170.0, 45.45, Psk { df: 150.0, db: 0.0 }),
        Scenario::new("scheda +500 ppm", 170.0, 45.45, Ppm(500.0)),
        Scenario::new("scheda -500 ppm", 170.0, 45.45, Ppm(-500.0)),
        Scenario::new("45 baud", 170.0, 45.45, Ppm(10_000.0)),
        Scenario::new("stop 1 bit", 170.0, 45.45, Stop(1.0)),
        Scenario::new("stop 2 bit", 170.0, 45.45, Stop(2.0)),
        Scenario::new("scariche", 170.0, 45.45, Impulsi { per_s: 8.0, db: 20.0 }),
        Scenario::new("filtro 250 Hz", 170.0, 45.45, FiltroRadio(250.0)),
        Scenario::new("audio tagliato", 170.0, 45.45, Tagliato(0.7)),
        Scenario::new("AGC che pompa", 170.0, 45.45, Agc),
    ]
}

/// Uno scenario cercato per nome (per lanciare una riga sola con `RTTY_SCENARIO`).
pub fn scenario(nome: &str) -> Option<Scenario> {
    scenari().into_iter().find(|s| s.nome == nome)
}

/// L'audio di una prova: un secondo di solo rumore prima e dopo il testo. Restituisce (audio a 12 kHz, testo mandato).
pub fn genera(sc: &Scenario, snr_db: f64, seed: u64, chars: usize) -> (Vec<f32>, String) {
    let mut rng = Rng(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1);
    let sent = testo(&mut rng, chars);
    let tx_centre = if let Disturbo::Fuori(hz) = sc.disturbo { CENTRE + hz } else { CENTRE };
    let p = sc.params(tx_centre);
    let mut sig = match sc.disturbo {
        Disturbo::SpaceGiu(db) => rtty::modulate_levels(sent.as_bytes(), p, AMP as f32, (AMP * 10f64.powf(db / 20.0)) as f32),
        Disturbo::Stop(bits) => rtty::modulate_stop(sent.as_bytes(), p, AMP as f32, bits),
        _ => rtty::modulate(sent.as_bytes(), p, AMP as f32),
    };
    match sc.disturbo {
        Disturbo::Fading { spread, delay_ms } => sig = fading_ccir(&sig, spread, delay_ms, &mut rng),
        Disturbo::Ppm(ppm) => sig = errore_orologio(&sig, ppm),
        _ => {}
    }
    let pad = RATE as usize;
    let mut audio = vec![0.0_f32; pad];
    audio.extend_from_slice(&sig);
    audio.extend(std::iter::repeat_n(0.0, pad));
    // disturbi che si sommano (una stazione o una portante per tutta la durata)
    match sc.disturbo {
        Disturbo::Qrm { df, db } => {
            let other = testo(&mut rng, chars * 2);
            let q = rtty::modulate(other.as_bytes(), sc.params(CENTRE + df), (AMP * 10f64.powf(db / 20.0)) as f32);
            somma(&mut audio, &q);
        }
        Disturbo::Portante { df, db } => {
            let a = AMP * 10f64.powf(db / 20.0);
            let w = 2.0 * std::f64::consts::PI * f64::from(CENTRE + df) / RATE;
            for (n, s) in audio.iter_mut().enumerate() {
                *s += (a * (w * n as f64).sin()) as f32;
            }
        }
        Disturbo::Psk { df, db } => {
            let other = testo(&mut rng, chars * 3);
            let q = psk::modulate(other.as_bytes(), CENTRE + df, PskMode::Psk31, (AMP * 10f64.powf(db / 20.0)) as f32);
            somma(&mut audio, &q);
        }
        _ => {}
    }
    aggiungi_rumore(&mut audio, sigma_per_snr(AMP, snr_db), &mut rng);
    match sc.disturbo {
        Disturbo::Impulsi { per_s, db } => {
            let a = AMP * 10f64.powf(db / 20.0);
            let p = per_s / RATE;
            let mut n = 0;
            while n < audio.len() {
                if rng.uniform() < p {
                    // una scarica: oscillazione smorzata in 2 ms, segno e fase a caso
                    let f = 500.0 + 2500.0 * rng.uniform();
                    let ph = rng.uniform() * std::f64::consts::TAU;
                    let amp = a * (0.5 + rng.uniform());
                    for k in 0..48 {
                        if let Some(s) = audio.get_mut(n + k) {
                            let t = k as f64 / RATE;
                            *s += (amp * (-t / 0.0007).exp() * (std::f64::consts::TAU * f * t + ph).sin()) as f32;
                        }
                    }
                }
                n += 1;
            }
        }
        Disturbo::FiltroRadio(bw) => audio = passa_banda(&audio, f64::from(CENTRE), bw),
        Disturbo::Tagliato(k) => {
            let c = (AMP * k) as f32;
            for s in audio.iter_mut() {
                *s = s.clamp(-c, c);
            }
        }
        Disturbo::Agc => {
            for (n, s) in audio.iter_mut().enumerate() {
                // ogni 2 s: giu' di 15 dB in 20 ms (la scarica che arriva), poi risale in 1,5 s
                let t = (n as f64 / RATE) % 2.0;
                let g_db = if t < 0.02 { -15.0 * t / 0.02 } else { -15.0 * (1.0 - ((t - 0.02) / 1.5).min(1.0)) };
                *s *= 10f64.powf(g_db / 20.0) as f32;
            }
        }
        _ => {}
    }
    (audio, sent)
}

fn somma(a: &mut [f32], b: &[f32]) {
    for (x, y) in a.iter_mut().zip(b.iter()) {
        *x += *y;
    }
}

/// Filtro passa-banda della radio: FIR con finestra di Blackman, banda `bw` centrata su `centre`, transizione di circa 40 Hz.
pub fn passa_banda(x: &[f32], centre: f64, bw: f64) -> Vec<f32> {
    let len = 1201;
    let half = (len / 2) as f64;
    let fc = bw / 2.0 / RATE;
    let w0 = 2.0 * std::f64::consts::PI * centre / RATE;
    let h: Vec<f64> = (0..len)
        .map(|k| {
            let n = k as f64 - half;
            let lp = if n == 0.0 { 2.0 * fc } else { (2.0 * std::f64::consts::PI * fc * n).sin() / (std::f64::consts::PI * n) };
            let w = 0.42 - 0.5 * (2.0 * std::f64::consts::PI * k as f64 / (len - 1) as f64).cos() + 0.08 * (4.0 * std::f64::consts::PI * k as f64 / (len - 1) as f64).cos();
            2.0 * lp * w * (w0 * n).cos()
        })
        .collect();
    (0..x.len())
        .map(|i| {
            let mut acc = 0.0;
            for (k, &hk) in h.iter().enumerate() {
                // ritardo di gruppo compensato: uscita allineata all'ingresso
                let j = i as i64 + half as i64 - k as i64;
                if j >= 0 && (j as usize) < x.len() {
                    acc += hk * f64::from(x[j as usize]);
                }
            }
            acc as f32
        })
        .collect()
}

/// Solo rumore bianco (per contare i caratteri inventati): `secondi` di rumore di deviazione pari ad AMP.
pub fn solo_rumore(secondi: f64, seed: u64) -> Vec<f32> {
    let mut rng = Rng(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1);
    let mut audio = vec![0.0_f32; (secondi * RATE) as usize];
    aggiungi_rumore(&mut audio, AMP, &mut rng);
    audio
}

/// Toglie i ritorni a capo (non contano nel confronto).
pub fn pulisci(v: Vec<u8>) -> Vec<u8> {
    v.into_iter().filter(|&c| c != b'\r' && c != b'\n').collect()
}

/// Errori (Levenshtein) del testo ricevuto rispetto a quello mandato.
pub fn errori(sent: &str, got: Vec<u8>) -> usize {
    levenshtein(sent.as_bytes(), &pulisci(got))
}

/// La soglia S/N (dB) a cui il CER scende sotto `livello` (0..1), interpolando in scala logaritmica fra i punti della griglia
/// (`punti`: (S/N, CER) con S/N crescente). `None` se non scende mai sotto; il primo S/N se e' gia' sotto all'inizio.
pub fn soglia(punti: &[(f64, f64)], livello: f64) -> Option<f64> {
    // si cerca dall'alto l'ultimo tratto in cui si passa sopra la soglia: un CER che risale a S/N alto (caratteri inventati) non conta
    let sotto = punti.iter().rposition(|&(_, c)| c > livello).map_or(0, |i| i + 1);
    if sotto >= punti.len() {
        return None;
    }
    if sotto == 0 {
        return Some(punti[0].0);
    }
    let (s0, c0) = punti[sotto - 1];
    let (s1, c1) = punti[sotto];
    let l = |c: f64| c.max(1e-4).ln();
    let t = if (l(c0) - l(c1)).abs() < 1e-12 { 1.0 } else { (l(c0) - l(livello)) / (l(c0) - l(c1)) };
    Some(s0 + t.clamp(0.0, 1.0) * (s1 - s0))
}
