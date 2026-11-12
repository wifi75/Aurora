//! AGC del ricevitore CW comandato dal fondo del rumore, a due inseguitori: rimette a posto il livello quando la radio «pompa» (una
//! stazione forte nella banda fa scendere il guadagno di tutto l'audio a ogni suo elemento) e durante gli scrosci di rumore larghi
//! (QRN), e resta spento, campione per campione, quando il rumore sta fermo.
//!
//! Come lavora, scritto per Aurora:
//! - **misura**: spettri corti dell'audio a 12 kHz (128 campioni con finestra di Hann, uno ogni 64: 5,3 ms). Della banda passante
//!   (le righe che in media stanno entro 12 dB dal terzo quartile: quello che la radio lascia passare) valgono i tre quarti più
//!   quieti in media, le righe fra un segnale e l'altro; il fondo di ogni spettro è la media dei loro due terzi più bassi. I segnali,
//!   pochi e stretti, non la comandano: né la stazione che si ascolta, né una vicina, né una forte lontana. Di tre spettri di fila
//!   vale quello di mezzo (una scarica isolata non conta). Il quarto più basso fa da controllo: sul solo rumore sta 5,2 dB sotto i
//!   due terzi; se troppi spettri dell'ultimo secondo non rispettano quel rapporto la banda è piena di stazioni, non c'è una riga
//!   libera da cui leggere il rumore, e l'AGC resta spento;
//! - **due inseguitori** sul fondo in dB: quello del bordo alto sale in 1 ms e scende in 2 s, quello del bordo basso scende in 50 ms
//!   e sale in 2 s. La loro distanza dice se il rumore oscilla: sotto 5 dB l'AGC è spento (guadagno 1 esatto: sul rumore fermo la
//!   distanza arriva a 4 dB), da 8 dB in su è tutto acceso; con poche righe le soglie salgono. Oltre 35 dB non è rumore che oscilla
//!   ma silenzio digitale fra un segnale e l'altro: spento;
//! - **guadagno**: quando è acceso riporta il fondo sul bordo basso (taglia al massimo 25 dB), interpolato in dB fra uno spettro e
//!   l'altro. Costa 16 ms di ritardo;
//! - **scroscio o radio che pompa**: un fondo che sale di almeno 4 dB in tre spettri (16 ms) è uno scroscio, e lo resta finché non
//!   torna vicino al bordo basso; il guadagno di una radio che risale dopo un segnale forte ci mette almeno un decimo di secondo.
//!
//! Il guadagno lavora prima del filtro stretto (i gradini di 20 dB in 2 ms dell'AGC di una radio si rimettono a posto prima che il
//! filtro li spalmi: dopo il filtro, nella prova `cw_confronto`, la riga «AGC che pompa» passa da 21,1 a 28,4) ma è comandato solo dal
//! rumore. Chi lo usa sa anche se il guadagno viene da uno scroscio: lì il segnale non era salito con il rumore e dopo la correzione
//! è più basso, e il decodificatore probabilistico tiene aperte le due ipotesi; con la radio che pompa segnale e rumore sono scesi
//! insieme e dopo la correzione il segnale è tornato quello di sempre.

use rustfft::num_complex::Complex32;
use rustfft::{Fft, FftPlanner};
use std::collections::VecDeque;
use std::sync::Arc;

/// Lunghezza dello spettro e passo (campioni a 12 kHz).
const FRAME: usize = 128;
const HOP: usize = 64;
/// Ritardo del guadagno rispetto all'audio (campioni): lo spettro di mezzo di tre più mezzo passo per interpolare. Multiplo della
/// decimazione del ricevitore, così con l'AGC spento la banda base è la stessa di prima, solo più tardi.
pub(crate) const DELAY: usize = 192;
/// Costanti di tempo degli inseguitori (s): il bordo alto sale e scende, il bordo basso scende e sale.
const HI_ATTACK_S: f32 = 0.001;
const HI_RELEASE_S: f32 = 2.0;
const LO_FALL_S: f32 = 0.05;
const LO_RISE_S: f32 = 2.0;
/// Distanza fra i due bordi (dB) sotto la quale l'AGC è spento, da cui è tutto acceso, e oltre la quale non è rumore.
const SPAN_OFF_DB: f32 = 5.0;
const SPAN_FULL_DB: f32 = 8.0;
const SPAN_MAX_DB: f32 = 35.0;
/// Taglio massimo (dB).
const MAX_CUT_DB: f32 = 25.0;
/// Uno scroscio: il fondo sale di tanto (dB) in tre spettri; e' finito quando torna a meno di tanto (dB) dal bordo basso.
const BURST_RISE_DB: f32 = 4.0;
const BURST_END_DB: f32 = 2.0;
/// Righe della banda passante: entro tanti dB dal terzo quartile dei livelli medi; ne servono almeno tante.
const PASS_DB: f32 = 12.0;
const MIN_BINS: usize = 8;
/// Righe usate per il fondo con il rumore su tutta la banda dell'audio (le soglie valgono per questo numero).
const REF_BINS: f32 = 32.0;
/// Sul rumore la media dei due terzi piu' bassi delle righe sta 5,2 dB sopra quella del quarto piu' basso (potenze esponenziali:
/// 0,451 contro 0,137 della media); di piu' (oltre il margine) vuol dire segnali dentro i due terzi.
const QUARTER_DB: f32 = 5.2;
const QUARTER_SLACK_DB: f32 = 1.5;
/// Banda piena: se piu' di questa quota degli spettri dell'ultimo secondo non sembrava solo rumore l'AGC resta spento.
const CROWD_S: f32 = 1.0;
const CROWD_MAX: f32 = 0.25;
/// Il fondo vale solo se non sta troppo sotto la riga piu' forte dello spettro (70 dB).
const SPLATTER: f32 = 1e-7;
/// Media dei livelli delle righe (s) e spettri prima di cominciare (0,3 s).
const SLOW_S: f32 = 2.0;
const WARM_FRAMES: u32 = 56;

pub(crate) struct NoiseAgc {
    fft: Arc<dyn Fft<f32>>,
    window: Vec<f32>,
    buf: Vec<Complex32>,
    recent: VecDeque<f32>,
    since_hop: usize,
    /// Potenza media di ogni riga e righe della banda passante.
    slow: Vec<f32>,
    pass: Vec<bool>,
    frames: u32,
    sorted: Vec<f32>,
    /// Il fondo (dB) degli ultimi tre spettri.
    last3: [f32; 3],
    hi: f32,
    lo: f32,
    /// Il guadagno (dB) ai centri dei due spettri attorno al campione che esce, e i campioni passati dal primo.
    g_prev: f32,
    g_next: f32,
    pos: usize,
    /// Il fondo (di mezzo) dei due spettri prima, e se il fondo e' salito di colpo (uno scroscio) e non e' ancora tornato giu': vale per
    /// i due spettri attorno al campione che esce.
    /// Righe usate per il fondo dell'ultimo spettro, e quota degli ultimi spettri che non sembravano solo rumore.
    used: usize,
    crowd: f32,
    mids: [f32; 2],
    burst: bool,
    b_prev: bool,
    b_next: bool,
}

impl Default for NoiseAgc {
    fn default() -> Self {
        Self::new()
    }
}

impl NoiseAgc {
    pub(crate) fn new() -> Self {
        NoiseAgc {
            fft: FftPlanner::new().plan_fft_forward(FRAME),
            window: (0..FRAME).map(|i| 0.5 - 0.5 * (2.0 * std::f32::consts::PI * (i as f32 + 0.5) / FRAME as f32).cos()).collect(),
            buf: vec![Complex32::new(0.0, 0.0); FRAME],
            recent: VecDeque::with_capacity(FRAME + 1),
            since_hop: 0,
            slow: vec![0.0; FRAME / 2],
            pass: vec![false; FRAME / 2],
            frames: 0,
            sorted: Vec::with_capacity(FRAME / 2),
            last3: [0.0; 3],
            hi: 0.0,
            lo: 0.0,
            g_prev: 0.0,
            g_next: 0.0,
            pos: 0,
            used: 0,
            crowd: 0.0,
            mids: [0.0; 2],
            burst: false,
            b_prev: false,
            b_next: false,
        }
    }

    /// Un campione dell'audio; restituisce il guadagno (ampiezza, 1 = spento) da dare al campione di [`DELAY`] campioni prima, e se in
    /// quel momento il rumore era salito di colpo (uno scroscio: il segnale non e' salito con lui) invece che piano (il guadagno della
    /// radio che risale: il segnale e' salito insieme al rumore).
    pub(crate) fn push(&mut self, x: f32) -> (f32, bool) {
        self.recent.push_back(x);
        if self.recent.len() > FRAME {
            self.recent.pop_front();
        }
        self.since_hop += 1;
        if self.since_hop >= HOP && self.recent.len() == FRAME {
            self.since_hop = 0;
            let g = self.frame();
            self.g_prev = self.g_next;
            self.g_next = g;
            self.b_prev = self.b_next;
            self.b_next = self.burst;
            self.pos = 0;
        }
        let db = self.g_prev + (self.g_next - self.g_prev) * (self.pos.min(HOP) as f32 / HOP as f32);
        self.pos += 1;
        if db == 0.0 { (1.0, false) } else { (10f32.powf(db / 20.0), self.b_prev || self.b_next) }
    }

    /// La distanza fra i due inseguitori (dB): quanto oscilla il fondo del rumore.
    #[cfg(test)]
    pub(crate) fn span_db(&self) -> f32 {
        self.hi - self.lo
    }

    /// Uno spettro nuovo: restituisce il guadagno (dB, mai sopra zero) per lo spettro di prima (quello di mezzo degli ultimi tre).
    fn frame(&mut self) -> f32 {
        for (b, (x, w)) in self.buf.iter_mut().zip(self.recent.iter().zip(&self.window)) {
            *b = Complex32::new(x * w, 0.0);
        }
        self.fft.process(&mut self.buf);
        self.frames = self.frames.saturating_add(1);
        let dt = HOP as f32 / crate::psk::RATE;
        let a = (1.0 / self.frames as f32).max(dt / SLOW_S);
        for (k, s) in self.slow.iter_mut().enumerate().skip(1) {
            *s += a * (self.buf[k].norm_sqr() - *s);
        }
        // la banda passante si rivede ogni 32 spettri (e subito, il primo)
        if self.frames % 32 == 1 {
            self.sorted.clear();
            self.sorted.extend_from_slice(&self.slow[1..]);
            self.sorted.sort_by(f32::total_cmp);
            let q3 = self.sorted[self.sorted.len() * 3 / 4];
            let least = q3 * 10f32.powf(-PASS_DB / 10.0);
            // della banda passante valgono le righe piu' quiete in media (i tre quarti piu' bassi): quelle fra un segnale e l'altro. In
            // una banda piena di stazioni le righe piu' basse di ogni spettro seguirebbero la loro manipolazione
            self.sorted.clear();
            self.sorted.extend(self.slow.iter().skip(1).filter(|s| **s >= least && **s > 0.0));
            self.sorted.sort_by(f32::total_cmp);
            let most = self.sorted.get(self.sorted.len() * 3 / 4).copied().unwrap_or(0.0);
            for (p, s) in self.pass.iter_mut().zip(&self.slow).skip(1) {
                *p = *s >= least && *s > 0.0 && *s <= most;
            }
        }
        self.sorted.clear();
        self.sorted.extend(self.pass.iter().zip(&self.buf).skip(1).filter(|(p, _)| **p).map(|(_, b)| b.norm_sqr()));
        let enough = self.sorted.len() >= MIN_BINS;
        // un fondo oltre 70 dB sotto la riga piu' forte non e' rumore ma quello che la finestra lascia passare di quel segnale: non vale
        let peak = self.buf[1..FRAME / 2].iter().map(|b| b.norm_sqr()).fold(0.0_f32, f32::max);
        let mut floor_db = self.last3[2];
        if enough {
            self.sorted.sort_by(f32::total_cmp);
            // i due terzi piu' bassi: una stazione che comincia su una riga quieta non conta
            let used = self.sorted.len() * 2 / 3;
            let mean = self.sorted[..used].iter().sum::<f32>() / used as f32;
            // il quarto piu' basso dice se quei due terzi sono davvero rumore: sul rumore le due medie stanno in un rapporto fisso
            // (QUARTER_DB), con le stazioni di una banda piena dentro i due terzi no, e allora vale il quarto
            let quarter = (self.sorted.len() / 4).max(1);
            let low = self.sorted[..quarter].iter().sum::<f32>() / quarter as f32;
            let limit = low * 10f32.powf((QUARTER_DB + QUARTER_SLACK_DB) / 10.0);
            // quanti spettri, nell'ultimo secondo, non sembravano solo rumore: una banda piena non ha righe libere, e li' l'AGC tace
            self.crowd += (dt / CROWD_S) * (if mean > limit { 1.0 } else { 0.0 } - self.crowd);
            let mean = mean.min(limit);
            self.used = used;
            if mean >= SPLATTER * peak {
                floor_db = 10.0 * mean.max(1e-20).log10();
            }
        }
        self.last3 = [self.last3[1], self.last3[2], floor_db];
        let [a3, b3, c3] = self.last3;
        let mid = a3.max(b3).min(a3.min(b3).max(c3));
        // all'inizio i due bordi partono insieme dal fondo di adesso
        if self.frames < WARM_FRAMES {
            (self.hi, self.lo) = (mid, mid);
            self.mids = [mid; 2];
            return 0.0;
        }
        let k = |tau: f32| 1.0 - (-dt / tau).exp();
        self.hi += (mid - self.hi) * if mid > self.hi { k(HI_ATTACK_S) } else { k(HI_RELEASE_S) };
        self.lo += (mid - self.lo) * if mid < self.lo { k(LO_FALL_S) } else { k(LO_RISE_S) };
        // scroscio: il fondo sale di almeno BURST_RISE_DB in tre spettri (16 ms); finisce quando torna vicino al bordo basso. Il guadagno
        // di una radio che risale dopo un segnale forte ci mette almeno un decimo di secondo
        let rise = mid - self.mids[0].min(self.mids[1]);
        self.mids = [self.mids[1], mid];
        if rise >= BURST_RISE_DB {
            self.burst = true;
        } else if mid < self.lo + BURST_END_DB {
            self.burst = false;
        }
        let span = self.hi - self.lo;
        // con poche righe il fondo balla di piu': le soglie salgono con la radice del loro numero
        let off = SPAN_OFF_DB * (REF_BINS / self.used.max(1) as f32).sqrt().max(1.0);
        if !enough || span <= off || span > SPAN_MAX_DB || self.crowd > CROWD_MAX {
            return 0.0;
        }
        let w = ((span - off) / (SPAN_FULL_DB - SPAN_OFF_DB)).min(1.0);
        -(w * (mid - self.lo)).clamp(0.0, MAX_CUT_DB)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Rumore gaussiano (somma di dodici uniformi).
    fn noise(n: usize, amp: f32, seed: u64) -> Vec<f32> {
        let mut x = seed | 1;
        let mut uni = move || {
            x ^= x >> 12;
            x ^= x << 25;
            x ^= x >> 27;
            (x.wrapping_mul(0x2545_F491_4F6C_DD1D) >> 40) as f32 / (1u64 << 24) as f32
        };
        (0..n).map(|_| ((0..12).map(|_| uni()).sum::<f32>() - 6.0) * amp).collect()
    }

    /// Un tono manipolato a tratti di `keyed_ms` (0 = sempre acceso), con fronti a coseno di 5 ms come quelli di una radio.
    fn tone(n: usize, hz: f32, amp: f32, keyed_ms: usize) -> Vec<f32> {
        let period = 12 * keyed_ms.max(1);
        (0..n)
            .map(|i| {
                let s = amp * (2.0 * std::f32::consts::PI * hz * i as f32 / 12_000.0).sin();
                if keyed_ms == 0 {
                    return s;
                }
                if !(i / period).is_multiple_of(2) {
                    return 0.0;
                }
                let edge = (i % period).min(period - 1 - i % period) as f32 / 60.0;
                s * if edge < 1.0 { 0.5 - 0.5 * (std::f32::consts::PI * edge).cos() } else { 1.0 }
            })
            .collect()
    }

    #[test]
    fn steady_noise_with_signals_leaves_the_gain_at_one() {
        let mut audio = noise(120_000, 0.1, 3);
        // la stazione che si ascolta, una vicina e una forte lontana, tutte manipolate
        for (hz, amp, ms) in [(700.0, 0.3, 60), (850.0, 0.3, 48), (1500.0, 3.0, 55)] {
            for (s, t) in audio.iter_mut().zip(tone(120_000, hz, amp, ms)) {
                *s += t;
            }
        }
        let mut agc = NoiseAgc::new();
        for (i, &x) in audio.iter().enumerate() {
            let (g, _) = agc.push(x);
            assert!(g == 1.0, "guadagno {g} al campione {i}, distanza {}", agc.span_db());
        }
        assert!(agc.span_db() < SPAN_OFF_DB);
        assert!(agc.crowd < CROWD_MAX / 2.0, "quota di spettri non di solo rumore {}", agc.crowd);
    }

    /// Una banda piena: il rumore solo nella banda passante di una radio in SSB (300-2700 Hz, somma di tanti toni a caso) e dodici
    /// stazioni CW dentro, una ogni 190 Hz, 15 dB sopra il rumore della loro riga. La loro manipolazione non e' rumore che oscilla.
    #[test]
    fn a_crowded_band_is_not_pumping() {
        let n = 120_000;
        let mut x = 77u64;
        let mut uni = move || {
            x ^= x >> 12;
            x ^= x << 25;
            x ^= x >> 27;
            (x.wrapping_mul(0x2545_F491_4F6C_DD1D) >> 40) as f32 / (1u64 << 24) as f32
        };
        let mut audio = vec![0.0_f32; n];
        for _ in 0..800 {
            let (hz, ph) = (300.0 + 2400.0 * uni(), 2.0 * std::f32::consts::PI * uni());
            for (i, s) in audio.iter_mut().enumerate() {
                *s += 0.004 * (2.0 * std::f32::consts::PI * hz * i as f32 / 12_000.0 + ph).sin();
            }
        }
        for k in 0..12 {
            for (s, t) in audio.iter_mut().zip(tone(n, 400.0 + 190.0 * k as f32, 0.12, 40 + 7 * k)) {
                *s += t;
            }
        }
        let mut agc = NoiseAgc::new();
        let cut = audio.iter().filter(|&&x| agc.push(x).0 < 0.9).count();
        assert!(cut < n / 200, "campioni abbassati di piu' di 1 dB: {cut}, distanza {}", agc.span_db());
    }

    #[test]
    fn a_noise_burst_is_brought_back_to_the_floor() {
        let mut audio = noise(120_000, 0.05, 5);
        // uno scroscio di 200 ms, 12 dB sopra il fondo
        for s in audio[60_000..62_400].iter_mut() {
            *s *= 4.0;
        }
        let mut agc = NoiseAgc::new();
        let (gains, bursts): (Vec<f32>, Vec<bool>) = audio.iter().map(|&x| agc.push(x)).unzip();
        // il guadagno del campione n esce DELAY campioni dopo
        let at = |n: usize| 20.0 * gains[n + DELAY].log10();
        assert!(at(50_000) == 0.0);
        let inside = (60_300..62_100).map(at).sum::<f32>() / 1_800.0;
        assert!((-14.5..-9.5).contains(&inside), "dentro lo scroscio {inside} dB");
        assert!(at(64_000) > -2.5, "dopo lo scroscio {} dB", at(64_000));
        assert!(bursts[61_000 + DELAY] && !bursts[50_000 + DELAY] && !bursts[70_000 + DELAY]);
    }

    #[test]
    fn digital_silence_between_tones_is_not_pumping() {
        let audio = tone(60_000, 700.0, 0.5, 60);
        let mut agc = NoiseAgc::new();
        for &x in &audio {
            assert!(agc.push(x).0 == 1.0);
        }
    }
}
