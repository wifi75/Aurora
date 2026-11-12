//! Decodificatore CW probabilistico: invece di decidere «tono sì / tono no» campione per campione, tiene aperte le ipotesi più probabili
//! sull'intera sequenza (punti, linee, pause, caratteri, velocità) e sceglie il testo solo quando ha visto abbastanza.
//!
//! L'idea è quella del decodificatore bayesiano di E. L. Bell (tesi NPS, 1977): un modello di Markov nascosto a durata esplicita su un
//! traliccio, con un fascio delle ipotesi migliori. Qui è scritto da zero per Aurora:
//! - **osservazione**: la banda base complessa dopo il filtro del ricevitore (500 Hz di campionamento), riportata a frequenza zero con
//!   la stima fine del tono. Un elemento acceso (punto o linea) si valuta con la somma *coerente* dei suoi campioni (il filtro adattato
//!   all'elemento, qualunque sia la sua durata), a pezzi non più lunghi di un'unità perché un piccolo errore di frequenza non la cancelli;
//!   il rapporto di verosimiglianza è quello di Rice contro Rayleigh, con l'ampiezza del segnale e la varianza del rumore stimate in linea
//!   (il rumore anche correlato dal filtro: la varianza di una somma di `n` campioni viene dall'autocorrelazione dei coefficienti). Le
//!   pause valgono zero: tutto il punteggio sta negli elementi accesi. L'ampiezza è una miscela di quattro valori attorno alla stima, così
//!   un QSB o un evanescenza non fanno perdere i segni;
//! - **durate** («pugno»): log-normali per punto, linea, pausa fra gli elementi, fra i caratteri e fra le parole, con medie e larghezze
//!   che si adattano a chi manipola (tasto a mano, bug, peso, Farnsworth) sugli elementi del percorso migliore;
//! - **velocità**: fa parte dell'ipotesi (30 valori da 5 a 60 WPM); può cambiare fra un carattere e l'altro, poco alla volta;
//! - **testo**: l'albero del Morse (un carattere esiste o no) e un modello del testo a coppie di caratteri, con un peso regolabile;
//! - **decisione ritardata**: un carattere si stampa quando il percorso migliore ne ha altri due dopo, o dopo un secondo e mezzo; le
//!   ipotesi che dicevano altro si scartano. Ogni carattere ha la sua probabilità (la somma delle ipotesi che lo condividono).
//!
//! Il percorso migliore aggiorna anche l'ampiezza del segnale (sui toni) e il rumore (sulle pause lunghe).

use crate::cw::{MORSE, PROSIGNS};
use std::collections::HashMap;
use std::f64::consts::PI;

/// Velocità del traliccio (WPM): 30 valori in progressione geometrica da 5 a 60.
const N_SPEEDS: usize = 30;
const WPM_LO: f64 = 5.0;
const WPM_HI: f64 = 60.0;
/// Ipotesi tenute a ogni passo.
const BEAM: usize = 320;
/// Ipotesi più indietro di così (in logaritmo naturale) rispetto alla migliore si scartano.
const BEAM_DROP: f32 = 30.0;
/// Passo del traliccio (s).
const STEP_S: f64 = 0.004;
/// Memoria dei campioni (s): un elemento acceso non dura di più.
const HISTORY_S: f64 = 4.0;
/// Caratteri che il percorso migliore deve avere dopo quello da stampare, e tempo massimo d'attesa (s).
const DELAY_CHARS: usize = 2;
const DELAY_S: f64 = 1.5;
/// Fattori dell'ampiezza (sulla potenza) nella miscela, e loro pesi.
const AMP_FACTORS: [f64; 4] = [0.3, 0.6, 1.0, 1.6];
const AMP_WEIGHTS: [f64; 4] = [0.1, 0.2, 0.55, 0.15];
/// Peso dell'ampiezza che spiega meglio il tono stesso.
const ML_WEIGHT: f64 = 0.03;
/// Peso di base del modello del testo.
pub const LM_WEIGHT_DEFAULT: f32 = 0.5;

/// Cambi di velocità (passi del traliccio, probabilità per verso) fra i caratteri e fra le parole.
const SPEED_MOVES_CHAR: [(i32, f32); 1] = [(1, 0.01)];
const SPEED_MOVES_WORD: [(i32, f32); 4] = [(1, 0.04), (2, 0.01), (4, 0.004), (8, 0.002)];

/// Le classi di durata: punto, linea, pausa fra gli elementi, fra i caratteri, fra le parole.
const DOT: usize = 0;
const DASH: usize = 1;
const INTRA: usize = 2;
const CHARGAP: usize = 3;
const WORDGAP: usize = 4;

/// Gli elementi (in unità) e le larghezze (logaritmo naturale) di partenza: Morse esatto, con l'incertezza del rumore.
const FIST_MEAN: [f64; 5] = [1.0, 3.0, 1.0, 3.0, 7.0];
const FIST_SIGMA: [f64; 5] = [0.28, 0.22, 0.32, 0.26, 0.3];
/// Limiti delle medie del pugno (in unità) mentre si adattano.
const FIST_LIMITS: [(f64, f64); 5] = [(1.0, 1.0), (2.5, 4.2), (0.6, 1.5), (2.2, 5.0), (4.5, 10.0)];

/// Probabilità di una pausa lunga (oltre la pausa fra le parole: fine della trasmissione, attesa) e durata massima che si considera.
const P_LONG: f64 = 0.3;

/// Il quantile 20% della potenza mediata su 10 ms, diviso per la potenza media, sul solo rumore (misurato: vedi le prove).
const NOISE_Q20: f64 = 0.45;
/// Rapporto segnale/rumore massimo per campione che si crede (7 dB, cioe' circa -3 dB in 3 kHz con la banda di 150 Hz): sopra non serve
/// altro e, con un segnale pulito, la verosimiglianza enorme renderebbe inutili le durate (un fronte fuori posto di 2 ms varrebbe piu'
/// di tutto il resto).
const MAX_SNR: f64 = 5.0;

/// Un pezzo di testo deciso o ancora in sospeso: un nodo terminale dell'albero del Morse, o lo spazio.
const SPACE_TOKEN: u16 = u16::MAX;

/// L'albero del Morse: dalla radice, figlio 0 = punto, 1 = linea. Ogni nodo con un carattere ha il suo testo.
struct Trie {
    child: Vec<[u16; 2]>,
    text: Vec<Option<&'static str>>,
    /// Il simbolo per il modello del testo (il segno di procedura vale come `*`).
    lm_sym: Vec<u8>,
}

const NO_CHILD: u16 = u16::MAX;

impl Trie {
    fn new() -> Self {
        let mut t = Trie { child: vec![[NO_CHILD; 2]], text: vec![None], lm_sym: vec![b' '] };
        // prima i segni di procedura: a parità di sequenza si stampano loro (come il ricevitore classico)
        for (name, p) in PROSIGNS {
            let n = t.insert(p);
            t.text[n] = Some(name);
            t.lm_sym[n] = b'*';
        }
        for (c, p) in MORSE {
            let n = t.insert(p);
            if t.text[n].is_none() {
                t.text[n] = Some(char_str(*c));
                t.lm_sym[n] = *c;
            }
        }
        t
    }

    fn insert(&mut self, pattern: &str) -> usize {
        let mut n = 0usize;
        for e in pattern.bytes() {
            let k = usize::from(e == b'-');
            if self.child[n][k] == NO_CHILD {
                self.child[n][k] = self.child.len() as u16;
                self.child.push([NO_CHILD; 2]);
                self.text.push(None);
                self.lm_sym.push(b' ');
            }
            n = usize::from(self.child[n][k]);
        }
        n
    }
}

/// Il testo di un carattere della tabella come `&'static str`.
fn char_str(c: u8) -> &'static str {
    const ALL: &str = "ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789.,?'!/()&:;=+-_\"$@";
    match ALL.bytes().position(|x| x == c) {
        Some(i) => &ALL[i..i + 1],
        None => "*",
    }
}

/// Il modello del testo: probabilità di un carattere dato quello prima (coppie), contate su un testo di QSO CW tipici e lisciate.
struct Lm {
    /// log P(b | a) per i simboli 0-127 (ASCII), righe = carattere prima.
    logp: Vec<f32>,
}

/// Il testo su cui si contano le coppie: frasi tipiche dei QSO in CW, dei contest e dell'inglese telegrafico (scritte per Aurora).
const LM_CORPUS: &str = "CQ CQ CQ DE IU3ABC IU3ABC K \
IU3ABC DE K1ABC K1ABC KN \
K1ABC DE IU3ABC GM ES TNX FER CALL UR RST 599 599 NAME IS JOE JOE QTH NR MILANO MILANO HW CPY K1ABC DE IU3ABC KN \
IU3ABC DE K1ABC R R TNX JOE UR RST 579 579 NAME BOB BOB QTH BOSTON BOSTON RIG IS 100W ANT DIPOLE WX IS SUNNY ES WARM HW \
FB OM TNX FER QSO HPE CUAGN 73 73 SK \
TEST IU3ABC TEST K1ABC 5NN 14 TU 5NN 28 TU QRZ TEST \
CQ TEST DE DL1ABC DL1ABC TEST \
G4ABC 5NN 001 TU EE \
CQ DX DE JA1ABC JA1ABC PSE K \
VK2ABC UR 559 TNX AGN QSL VIA BURO 73 GL \
QRL? QRL? QSY QRS QRQ QRM QRN QSB QTH QRP QRO PSE AGN BK \
THE QUICK BROWN FOX JUMPS OVER THE LAZY DOG \
GOOD MORNING GOOD AFTERNOON GOOD EVENING THANKS FOR THE NICE CONTACT \
MY ANTENNA IS A VERTICAL AND THE POWER IS FIVE WATTS THE WEATHER HERE IS CLOUDY AND COLD \
I AM RETIRED AND HAVE BEEN A HAM SINCE 1975 HOPE TO SEE YOU AGAIN ON THE BANDS \
WORKED ALL STATES DXCC LOTW EQSL OK QSL CARD SURE \
14025 7025 3525 10115 18080 21025 28025 \
RST 599 5NN 579 559 449 339 \
UR SIGS FB HR IN ITALY \
CQ POTA CQ SOTA DE IU3ABC/P IU3ABC/P K \
";

impl Lm {
    fn new() -> Self {
        let mut counts = vec![0.0_f64; 128 * 128];
        let bytes: Vec<u8> = std::iter::once(b' ').chain(LM_CORPUS.bytes()).collect();
        for w in bytes.windows(2) {
            let (a, b) = (usize::from(w[0] & 0x7f), usize::from(w[1] & 0x7f));
            counts[a * 128 + b] += 1.0;
        }
        // simboli possibili: quelli dell'albero piu' lo spazio
        let mut syms: Vec<u8> = MORSE.iter().map(|(c, _)| *c).collect();
        syms.push(b' ');
        syms.push(b'*');
        let mut logp = vec![(1.0 / syms.len() as f32).ln(); 128 * 128];
        for &a in &syms {
            let a = usize::from(a);
            let row = &counts[a * 128..a * 128 + 128];
            // lisciatura: mezzo conteggio a ogni simbolo
            let total: f64 = syms.iter().map(|&b| row[usize::from(b)] + 0.5).sum();
            for &b in &syms {
                let b = usize::from(b);
                logp[a * 128 + b] = (((row[b] + 0.5) / total).ln()) as f32;
            }
        }
        // niente confronto con il caso uniforme: ogni carattere costa la sua probabilita', cosi' spezzare un carattere in due non conviene
        Lm { logp }
    }

    fn score(&self, prev: u8, next: u8) -> f32 {
        self.logp[usize::from(prev & 0x7f) * 128 + usize::from(next & 0x7f)]
    }
}

/// Testo in sospeso di un'ipotesi: fino a 12 pezzi, e da quando aspetta il primo.
#[derive(Clone, Copy)]
struct Pending {
    len: u8,
    toks: [u16; 12],
    since: u32,
}

impl Pending {
    const EMPTY: Pending = Pending { len: 0, toks: [0; 12], since: 0 };

    fn push(&mut self, tok: u16, now: u32) -> bool {
        if usize::from(self.len) >= self.toks.len() {
            return false;
        }
        if self.len == 0 {
            self.since = now;
        }
        self.toks[usize::from(self.len)] = tok;
        self.len += 1;
        true
    }

    fn pop_front(&mut self, now: u32) {
        if self.len == 0 {
            return;
        }
        self.toks.copy_within(1..usize::from(self.len), 0);
        self.len -= 1;
        // il prossimo pezzo aspetta da adesso: non si stampa subito dopo per un ritardo che era del primo
        self.since = now;
    }
}

/// Un'ipotesi: la sequenza fino all'elemento aperto, che è cominciato in `start` (campione).
#[derive(Clone, Copy)]
struct Hyp {
    /// Punteggio totale (logaritmo naturale), elemento aperto compreso.
    score: f32,
    /// Punteggio della parte chiusa (fino a `start`).
    base: f32,
    start: u32,
    /// Inizio dell'elemento prima (per misurare l'elemento appena chiuso).
    prev_start: u32,
    node: u16,
    speed: u8,
    /// L'elemento aperto e' un tono.
    mark: bool,
    /// Nella pausa aperta il carattere e lo spazio sono gia' stati scritti.
    char_done: bool,
    word_done: bool,
    /// Ultimo simbolo scritto (per il modello del testo).
    prev: u8,
    /// Toni gia' passati (fino a 255): la pausa prima del primo non insegna niente.
    marks: u8,
    pend: Pending,
}

impl Hyp {
    fn key(&self) -> u64 {
        u64::from(self.start)
            | (u64::from(self.node) << 32)
            | (u64::from(self.speed) << 42)
            | (u64::from(self.mark) << 50)
            | (u64::from(self.char_done) << 51)
            | (u64::from(self.word_done) << 52)
            | (u64::from(self.prev & 0x7f) << 53)
    }
}

/// Hash veloce per le chiavi delle ipotesi (moltiplicazione di Fibonacci).
#[derive(Default)]
struct FastHash(u64);
impl std::hash::Hasher for FastHash {
    fn finish(&self) -> u64 {
        self.0
    }
    fn write(&mut self, bytes: &[u8]) {
        for &b in bytes {
            self.0 = (self.0 ^ u64::from(b)).wrapping_mul(0x9E37_79B9_7F4A_7C15);
        }
    }
    fn write_u64(&mut self, v: u64) {
        self.0 = (self.0 ^ v).wrapping_mul(0x9E37_79B9_7F4A_7C15).rotate_left(29);
    }
}
type FastMap<K, V> = HashMap<K, V, std::hash::BuildHasherDefault<FastHash>>;

/// log(I0(x)), la funzione di Bessel modificata (Abramowitz e Stegun 9.8.1-2).
fn ln_i0(x: f64) -> f64 {
    let ax = x.abs();
    if ax < 3.75 {
        let y = (x / 3.75) * (x / 3.75);
        (1.0 + y * (3.515_622_9 + y * (3.089_942_4 + y * (1.206_749_2 + y * (0.265_973_2 + y * (0.036_076_8 + y * 0.004_581_3)))))).ln()
    } else {
        let y = 3.75 / ax;
        let p = 0.398_942_28
            + y * (0.013_285_92 + y * (0.002_253_19 + y * (-0.001_575_65 + y * (0.009_162_81 + y * (-0.020_577_06 + y * (0.026_355_37 + y * (-0.016_476_33 + y * 0.003_923_77)))))));
        ax - 0.5 * ax.ln() + p.ln()
    }
}

/// log della probabilità che una normale standard superi `z`.
fn ln_upper_tail(z: f64) -> f64 {
    if z > 5.0 {
        return -0.5 * z * z - (z * (2.0 * PI).sqrt()).ln() + (1.0 - 1.0 / (z * z)).ln();
    }
    // erfc di Abramowitz e Stegun 7.1.26 sulla parte positiva, simmetria per quella negativa
    let x = z.abs() / std::f64::consts::SQRT_2;
    let t = 1.0 / (1.0 + 0.327_591_1 * x);
    let erfc = t * (0.254_829_592 + t * (-0.284_496_736 + t * (1.421_413_741 + t * (-1.453_152_027 + t * 1.061_405_429)))) * (-x * x).exp();
    let q = if z >= 0.0 { 0.5 * erfc } else { 1.0 - 0.5 * erfc };
    q.max(1e-300).ln()
}

/// Il decodificatore. Si alimenta con la banda base complessa (dopo il filtro del ricevitore) e restituisce il testo deciso.
pub struct CwBayes {
    fs: f64,
    /// Passo del traliccio in campioni.
    grid: u32,
    trie: Trie,
    lm: Lm,
    lm_weight: f32,
    /// Durata dell'unità (campioni) per ogni velocità del traliccio, e il suo logaritmo.
    units: [f64; N_SPEEDS],
    /// Il pugno: medie (unità) e larghezze (logaritmo) delle cinque classi.
    fist_mean: [f64; 5],
    fist_sigma: [f64; 5],
    /// Varianza della somma di `n` campioni di rumore, in unità della varianza di un campione.
    var_sum: Vec<f64>,
    var_slope: f64,
    /// Somme cumulate (complesse), delle potenze e dei campioni con il cancello aperto, ad anello.
    ring: Vec<(f64, f64, f64, u32)>,
    mask: usize,
    acc: (f64, f64, f64, u32),
    t: u32,
    /// Stima fine del tono (Hz rispetto al centro) e fase della correzione.
    offset_hz: f64,
    rot: f64,
    /// Rumore (potenza di un campione) e potenza del segnale per campione.
    noise: f64,
    signal: f64,
    noise_ready: bool,
    marks_seen: u32,
    // quantili per partire: fondo e picco della potenza mediata su 10 ms
    q_lo: f64,
    q_hi: f64,
    smooth: f64,
    warm: u32,
    beam: Vec<Hyp>,
    cand: Vec<Hyp>,
    index: FastMap<u64, usize>,
    llr_cache: FastMap<u32, f32>,
    /// L'ultimo cambio del percorso migliore gia' usato per adattare ampiezza, rumore e pugno.
    learned_at: u32,
    best_speed: u8,
    hint_wpm: f64,
    confidences: Vec<u8>,
    /// Elementi gia' passati dall'apprendimento del pugno.
    learned: u32,
    /// Ipotesi tenute (BEAM di base; meno per i canali della decodifica di tutta la banda).
    beam_size: usize,
}

impl CwBayes {
    /// `fs`: campionamento della banda base; `taps`: i coefficienti del filtro a 12 kHz e `decim` la decimazione (per la correlazione del
    /// rumore); `wpm_hint`: la velocità di partenza.
    pub fn new(fs: f64, taps: &[f32], decim: usize, wpm_hint: f32) -> Self {
        let hist = (HISTORY_S * fs) as usize;
        let size = hist.next_power_of_two();
        let mut units = [0.0; N_SPEEDS];
        for (i, u) in units.iter_mut().enumerate() {
            let wpm = WPM_LO * (WPM_HI / WPM_LO).powf(i as f64 / (N_SPEEDS - 1) as f64);
            *u = 1.2 / wpm * fs;
        }
        // autocorrelazione del rumore dopo il filtro, ai ritardi della decimazione
        let r = |m: usize| -> f64 { taps.iter().zip(taps.iter().skip(m)).map(|(a, b)| f64::from(*a) * f64::from(*b)).sum() };
        let r0 = r(0).max(1e-30);
        let rho: Vec<f64> = (0..).map(|k| k * decim).take_while(|&m| m < taps.len()).map(|m| r(m) / r0).collect();
        let var_sum: Vec<f64> = (0..=size)
            .map(|n| {
                let nf = n as f64;
                nf + 2.0 * rho.iter().enumerate().skip(1).filter(|(k, _)| *k < n).map(|(k, p)| (nf - k as f64) * p).sum::<f64>()
            })
            .collect();
        let var_slope = 1.0 + 2.0 * rho.iter().skip(1).sum::<f64>();
        let mut d = CwBayes {
            fs,
            grid: ((STEP_S * fs).round() as u32).max(1),
            trie: Trie::new(),
            lm: Lm::new(),
            lm_weight: LM_WEIGHT_DEFAULT,
            units,
            fist_mean: FIST_MEAN,
            fist_sigma: FIST_SIGMA,
            var_sum,
            var_slope,
            ring: vec![(0.0, 0.0, 0.0, 0); size],
            mask: size - 1,
            acc: (0.0, 0.0, 0.0, 0),
            t: 0,
            offset_hz: 0.0,
            rot: 0.0,
            noise: 0.0,
            signal: 0.0,
            noise_ready: false,
            marks_seen: 0,
            q_lo: 0.0,
            q_hi: 0.0,
            smooth: 0.0,
            warm: 0,
            beam: Vec::with_capacity(BEAM * 4),
            cand: Vec::with_capacity(BEAM * 8),
            index: FastMap::default(),
            llr_cache: FastMap::default(),
            learned_at: 0,
            best_speed: 0,
            hint_wpm: f64::from(wpm_hint),
            learned: 0,
            beam_size: BEAM,
            confidences: Vec::new(),
        };
        d.reset_beam();
        d
    }

    /// Quante ipotesi tenere (minimo 32): meno ipotesi, meno CPU (per i canali della decodifica di tutta la banda).
    pub fn set_beam(&mut self, n: usize) {
        self.beam_size = n.max(32);
    }

    /// Peso del modello del testo (0 = solo l'albero del Morse).
    pub fn set_lm_weight(&mut self, w: f32) {
        self.lm_weight = w.max(0.0);
    }

    /// La velocità di partenza: il traliccio ricomincia con le ipotesi attorno a lei.
    pub fn set_wpm_hint(&mut self, wpm: f32) {
        self.hint_wpm = f64::from(wpm);
        self.reset_beam();
    }

    /// Lo scarto del tono dal centro del filtro (Hz), dalla stima dello spettro.
    pub fn set_offset_hz(&mut self, hz: f64) {
        self.offset_hz = hz;
    }

    /// La velocità del percorso migliore.
    pub fn wpm(&self) -> f32 {
        (1.2 * self.fs / self.units[usize::from(self.best_speed)]) as f32
    }

    /// Le probabilità (0-255) dei pezzi di testo usciti da [`CwBayes::push`] dall'ultima volta (uno per carattere o spazio).
    pub fn take_confidences(&mut self) -> Vec<u8> {
        std::mem::take(&mut self.confidences)
    }

    /// Rapporto segnale/rumore stimato per campione (potenza del tono / potenza del rumore).
    pub fn snr(&self) -> f64 {
        if self.noise > 0.0 { self.signal / self.noise } else { 0.0 }
    }

    fn reset_beam(&mut self) {
        self.beam.clear();
        let lo = WPM_LO.ln();
        let hi = WPM_HI.ln();
        let hint = self.hint_wpm.clamp(WPM_LO, WPM_HI).ln();
        for s in 0..N_SPEEDS {
            let lw = lo + (hi - lo) * s as f64 / (N_SPEEDS - 1) as f64;
            // a priori sulle velocità: attorno a quella di partenza, ma largo (un fattore 2 a 1,5 sigma)
            let z = (lw - hint) / 0.45;
            let prior = (-0.5 * z * z) as f32;
            self.beam.push(Hyp {
                score: prior,
                base: prior,
                start: self.t,
                prev_start: self.t,
                node: 0,
                speed: s as u8,
                mark: false,
                char_done: true,
                word_done: true,
                prev: b' ',
                marks: 0,
                pend: Pending::EMPTY,
            });
        }
        self.best_speed = self.beam.iter().max_by(|a, b| a.score.total_cmp(&b.score)).map_or(0, |h| h.speed);
    }

    /// Un campione della banda base; `gate`: lo spettro dice che c'e' un segnale (altrimenti non si aprono toni nuovi). Il testo deciso
    /// va in `out`.
    pub fn push(&mut self, z: (f32, f32), gate: bool, out: &mut Vec<u8>) {
        if self.confidences.len() > 4096 {
            self.confidences.clear();
        }
        // riporta il tono a frequenza zero
        self.rot -= 2.0 * PI * self.offset_hz / self.fs;
        if self.rot.abs() > 2.0 * PI {
            self.rot %= 2.0 * PI;
        }
        let (s, c) = self.rot.sin_cos();
        let (x, y) = (f64::from(z.0), f64::from(z.1));
        let (zr, zi) = (x * c - y * s, x * s + y * c);
        let p = zr * zr + zi * zi;
        self.acc.0 += zr;
        self.acc.1 += zi;
        self.acc.2 += p;
        self.acc.3 = self.acc.3.wrapping_add(u32::from(gate));
        // con il cancello chiuso (nessun segnale nello spettro) tutto e' rumore: la stima piu' semplice e piu' sicura
        if !gate {
            let a = (0.5 * self.fs).recip();
            self.noise = if self.noise_ready { self.noise + a * (p - self.noise) } else { p.max(1e-30) };
            self.noise_ready = true;
        }
        self.t = self.t.wrapping_add(1);
        self.ring[self.t as usize & self.mask] = self.acc;
        if self.t.is_multiple_of(self.grid) {
            self.step(gate, out);
        }
    }

    /// Somma complessa e potenza dei campioni da `a` (compreso) a `b` (escluso), e quanti avevano il cancello aperto.
    fn sums(&self, a: u32, b: u32) -> (f64, f64, f64, u32) {
        let ra = self.ring[a as usize & self.mask];
        let rb = self.ring[b as usize & self.mask];
        (rb.0 - ra.0, rb.1 - ra.1, rb.2 - ra.2, rb.3.wrapping_sub(ra.3))
    }

    fn var_of(&self, n: usize) -> f64 {
        if n < self.var_sum.len() {
            self.var_sum[n]
        } else {
            let last = self.var_sum.len() - 1;
            self.var_sum[last] + (n - last) as f64 * self.var_slope
        }
    }

    /// Fondo e picco iniziali (quantili della potenza mediata su 10 ms), finche' il percorso migliore non da' stime migliori. Si chiama
    /// con i campioni appena arrivati, prima del ritardo della decisione: quando il traliccio vede l'inizio di un segnale, il picco lo
    /// conosce gia'.
    pub fn observe_level(&mut self, z: (f32, f32)) {
        let p = f64::from(z.0) * f64::from(z.0) + f64::from(z.1) * f64::from(z.1);
        let a = (0.01 * self.fs).recip().min(1.0);
        self.smooth += a * (p - self.smooth);
        self.warm += 1;
        // il silenzio digitale vale -120 dB: niente logaritmi infiniti
        let l = 10.0 * self.smooth.max(1e-12).log10();
        if self.warm < (0.08 * self.fs) as u32 {
            self.q_lo = l;
            self.q_hi = l;
            return;
        }
        // quantile 20% del fondo e 90% del picco, in dB, con passi da 500 Hz riportati al campionamento vero
        let eta = 0.06 * 500.0 / self.fs;
        self.q_lo += 2.5 * eta * (0.2 - if l < self.q_lo { 1.0 } else { 0.0 });
        self.q_hi += eta * (0.9 - if l < self.q_hi { 1.0 } else { 0.0 });
        // un segnale nuovo (oltre 3 dB sopra il picco): il picco lo raggiunge in qualche decina di ms
        if l > self.q_hi + 3.0 {
            self.q_hi += 0.1 * (l - self.q_hi) * 500.0 / self.fs;
        }
        self.q_hi = self.q_hi.max(self.q_lo);
        if !self.noise_ready {
            // sul solo rumore il 20% della media su 10 ms vale circa 0,45 della media (prova `noise_level_is_calibrated`)
            self.noise = 10f64.powf(self.q_lo / 10.0) / NOISE_Q20;
        }
        if self.marks_seen < 4 {
            self.signal = (10f64.powf(self.q_hi / 10.0) - self.noise).max(0.5 * self.noise);
        }
    }

    /// Il rapporto di verosimiglianza (logaritmo) di un tono fra `a` e `b` contro il solo rumore: somme coerenti a pezzi di al massimo
    /// `piece` campioni, miscela sulle ampiezze.
    fn mark_llr(&self, a: u32, b: u32, piece: u32) -> f32 {
        let n = b.wrapping_sub(a);
        if n == 0 || self.noise <= 0.0 {
            return 0.0;
        }
        let k = n.div_ceil(piece.max(1)).min(16);
        // il rumore non si considera mai sotto 1/MAX_SNR del livello forte che si vede (il quantile alto, che sale subito)
        let noise = self.noise.max(MAX_SNR.recip() * 10f64.powf(self.q_hi / 10.0));
        let mut pieces = [(0.0_f64, 0.0_f64); 16];
        let mut coherent = 0.0;
        for (j, p) in pieces.iter_mut().enumerate().take(k as usize) {
            let j = j as u32;
            let pa = a.wrapping_add(j * n / k);
            let pb = a.wrapping_add((j + 1) * n / k);
            let len = pb.wrapping_sub(pa) as usize;
            let (sr, si, _, _) = self.sums(pa, pb);
            let mag = (sr * sr + si * si).sqrt();
            *p = (len as f64, mag);
            coherent += mag;
        }
        // l'ampiezza: quattro valori attorno alla stima e quella che spiega meglio questo tono (un segnale che sale o un QSB)
        let ml = (coherent / f64::from(n)).powi(2);
        let mut acc = [0.0_f64; AMP_FACTORS.len() + 1];
        for &(len, mag) in pieces.iter().take(k as usize) {
            if len == 0.0 {
                continue;
            }
            let var = noise * self.var_of(len as usize);
            for (i, a2) in AMP_FACTORS.iter().map(|f| self.signal * f).chain(std::iter::once(ml)).enumerate() {
                let mu = len * a2.sqrt();
                acc[i] += ln_i0(2.0 * mu * mag / var) - mu * mu / var;
            }
        }
        let best = acc.iter().copied().fold(f64::MIN, f64::max);
        let mix: f64 = acc.iter().zip(AMP_WEIGHTS.iter().map(|w| w * (1.0 - ML_WEIGHT)).chain(std::iter::once(ML_WEIGHT))).map(|(l, w)| w * (l - best).exp()).sum();
        (best + mix.ln()) as f32
    }

    /// log della densità (per passo del traliccio) di una durata `d` (campioni) nella classe `c` alla velocità `s`.
    fn ln_pdf(&self, c: usize, d: u32, s: usize) -> f32 {
        let mu = self.fist_mean[c] * self.units[s];
        let sg = self.fist_sigma[c];
        let df = f64::from(d.max(1));
        let z = (df / mu).ln() / sg;
        let lognormal = -0.5 * z * z - (sg * df * (2.0 * PI).sqrt()).ln() + f64::from(self.grid).ln();
        if c == WORDGAP {
            // pause lunghe (fine della trasmissione, attesa): densità uniforme sul logaritmo fra 5 e 300 unità
            let u = self.units[s];
            let long = if df > 5.0 * u { -(df * (60.0_f64).ln()).ln() + f64::from(self.grid).ln() } else { f64::NEG_INFINITY };
            let a = (1.0 - P_LONG).ln() + lognormal;
            let b = P_LONG.ln() + long;
            let m = a.max(b);
            return (m + ((a - m).exp() + (b - m).exp()).ln()) as f32;
        }
        lognormal as f32
    }

    /// log della probabilità che una durata della classe `c` superi `d`.
    fn ln_surv(&self, c: usize, d: u32, s: usize) -> f64 {
        let mu = self.fist_mean[c] * self.units[s];
        let z = (f64::from(d.max(1)) / mu).ln() / self.fist_sigma[c];
        let ln = ln_upper_tail(z);
        if c == WORDGAP {
            let u = self.units[s];
            let ratio = (f64::from(d) / (5.0 * u)).max(1.0);
            let long = (1.0 - ratio.ln() / (60.0_f64).ln()).max(1e-6);
            return ((1.0 - P_LONG) * ln.exp() + P_LONG * long).ln();
        }
        ln
    }

    /// Le soglie (campioni) oltre le quali una pausa aperta e' gia' fine del carattere e fine della parola.
    fn gap_thresholds(&self, s: usize) -> (u32, u32) {
        let u = self.units[s];
        let t1 = (self.fist_mean[INTRA] * self.fist_mean[CHARGAP]).sqrt() * u;
        let t2 = (self.fist_mean[CHARGAP] * self.fist_mean[WORDGAP]).sqrt() * u;
        (t1.round() as u32, t2.round() as u32)
    }

    fn add(&mut self, h: Hyp) {
        if !h.score.is_finite() {
            return;
        }
        match self.index.get(&h.key()) {
            Some(&i) => {
                if h.score > self.cand[i].score {
                    self.cand[i] = h;
                }
            }
            None => {
                self.index.insert(h.key(), self.cand.len());
                self.cand.push(h);
            }
        }
    }

    /// Un passo del traliccio.
    fn step(&mut self, gate: bool, out: &mut Vec<u8>) {
        let t = self.t;
        let piece = (self.units[usize::from(self.best_speed)].round() as u32).max(2);
        self.cand.clear();
        self.index.clear();
        self.llr_cache.clear();
        let beam = std::mem::take(&mut self.beam);
        for h in &beam {
            let s = usize::from(h.speed);
            let d = t.wrapping_sub(h.start);
            if h.mark {
                let llr = match self.llr_cache.get(&h.start) {
                    Some(&v) => v,
                    None => {
                        let v = self.mark_llr(h.start, t, piece);
                        self.llr_cache.insert(h.start, v);
                        v
                    }
                };
                let node = usize::from(h.node);
                let kids = self.trie.child[node];
                let has = [kids[0] != NO_CHILD, kids[1] != NO_CHILD];
                // continua: il tono e' ancora acceso
                if f64::from(d) < 2.5 * self.fist_mean[DASH] * self.units[s] {
                    let surv = match has {
                        [true, true] => (0.5 * self.ln_surv(DOT, d, s).exp() + 0.5 * self.ln_surv(DASH, d, s).exp()).ln(),
                        [true, false] => self.ln_surv(DOT, d, s),
                        [false, true] => self.ln_surv(DASH, d, s),
                        [false, false] => f64::NEG_INFINITY,
                    };
                    let mut c = *h;
                    c.score = h.base + llr + surv as f32;
                    self.add(c);
                }
                // si chiude come punto o come linea
                let n_kids = has.iter().filter(|x| **x).count() as f32;
                for (k, cls) in [(0usize, DOT), (1, DASH)] {
                    if !has[k] {
                        continue;
                    }
                    let base = h.base + llr + self.ln_pdf(cls, d, s) - n_kids.ln();
                    self.add(Hyp {
                        score: base,
                        base,
                        start: t,
                        prev_start: h.start,
                        node: kids[k],
                        marks: h.marks.saturating_add(1),
                        mark: false,
                        char_done: false,
                        word_done: false,
                        ..*h
                    });
                }
            } else {
                let mut h = *h;
                let (t1, t2) = self.gap_thresholds(s);
                // la pausa e' gia' lunga abbastanza: si scrive il carattere (e poi lo spazio) senza aspettare il prossimo tono
                if !h.char_done && d >= t1 {
                    let node = usize::from(h.node);
                    if self.trie.text[node].is_none() {
                        continue;
                    }
                    let sym = self.trie.lm_sym[node];
                    h.base += self.lm_weight * self.lm.score(h.prev, sym);
                    if !h.pend.push(h.node, t) {
                        continue;
                    }
                    h.prev = sym;
                    h.node = 0;
                    h.char_done = true;
                }
                if h.char_done && !h.word_done && d >= t2 {
                    h.base += self.lm_weight * self.lm.score(h.prev, b' ');
                    if !h.pend.push(SPACE_TOKEN, t) {
                        continue;
                    }
                    h.prev = b' ';
                    h.word_done = true;
                }
                let cls = if d < t1 {
                    INTRA
                } else if d < t2 {
                    CHARGAP
                } else {
                    WORDGAP
                };
                // dopo un tono la pausa e' fra gli elementi (60%), fra i caratteri (30%) o fra le parole (10%)
                let prior = [0.0, 0.0, 0.6_f64, 0.3, 0.1];
                let surv = match cls {
                    INTRA => (prior[INTRA] * self.ln_surv(INTRA, d, s).exp() + prior[CHARGAP] * self.ln_surv(CHARGAP, d, s).exp() + prior[WORDGAP] * self.ln_surv(WORDGAP, d, s).exp()).ln(),
                    CHARGAP => (prior[CHARGAP] * self.ln_surv(CHARGAP, d, s).exp() + prior[WORDGAP] * self.ln_surv(WORDGAP, d, s).exp()).ln(),
                    _ => prior[WORDGAP].ln() + self.ln_surv(WORDGAP, d, s),
                };
                let mut c = h;
                c.score = h.base + surv as f32;
                self.add(c);
                // comincia un tono (solo con un segnale nello spettro)
                if gate && d > 0 {
                    let base = h.base + self.ln_pdf(cls, d, s) + (prior[cls] as f32).ln();
                    let next = Hyp { score: base, base, start: t, prev_start: h.start, mark: true, char_done: false, word_done: false, ..h };
                    if cls == INTRA {
                        self.add(next);
                    } else {
                        // fra un carattere e l'altro la velocità puo' cambiare di un passo, poco; fra le parole un po' di piu'
                        // (fra le parole anche salti grandi: chi passa la mano a un'altra stazione, o il QRQ dopo il CQ)
                        let moves: &[(i32, f32)] = if cls == WORDGAP { &SPEED_MOVES_WORD } else { &SPEED_MOVES_CHAR };
                        let p_stay = 1.0 - 2.0 * moves.iter().map(|m| m.1).sum::<f32>();
                        let stay = base + p_stay.ln();
                        self.add(Hyp { score: stay, base: stay, ..next });
                        for &(step, p) in moves {
                            for ds in [-step, step] {
                                let ns = i32::from(h.speed) + ds;
                                if (0..N_SPEEDS as i32).contains(&ns) {
                                    let b2 = base + p.ln();
                                    self.add(Hyp { score: b2, base: b2, speed: ns as u8, ..next });
                                }
                            }
                        }
                    }
                }
            }
        }
        // si tengono le migliori
        let best = self.cand.iter().map(|h| h.score).fold(f32::MIN, f32::max);
        // la migliore di ogni velocità resta comunque (se non e' troppo indietro): un avvio sbagliato non blocca la velocità per sempre
        let mut per_speed = [usize::MAX; 2 * N_SPEEDS];
        for (i, h) in self.cand.iter().enumerate() {
            let k = &mut per_speed[2 * usize::from(h.speed) + usize::from(h.mark)];
            if *k == usize::MAX || h.score > self.cand[*k].score {
                *k = i;
            }
        }
        let extras: Vec<Hyp> = per_speed.iter().filter(|&&i| i != usize::MAX).map(|&i| self.cand[i]).filter(|h| h.score > best - 200.0).collect();
        self.cand.retain(|h| h.score > best - BEAM_DROP);
        if self.cand.len() > self.beam_size {
            self.cand.select_nth_unstable_by(self.beam_size - 1, |a, b| b.score.total_cmp(&a.score));
            self.cand.truncate(self.beam_size);
        }
        let floor = self.cand.iter().map(|h| h.score).fold(f32::MAX, f32::min);
        self.cand.extend(extras.into_iter().filter(|h| h.score < floor));
        // i punteggi restano vicini allo zero
        for h in self.cand.iter_mut() {
            h.score -= best;
            h.base -= best;
        }
        self.beam = std::mem::take(&mut self.cand);
        if self.beam.is_empty() {
            self.reset_beam();
            return;
        }
        self.learn();
        self.commit(out);
    }

    /// Il percorso migliore insegna ampiezza, rumore, velocità e pugno.
    fn learn(&mut self) {
        let Some(best) = self.beam.iter().max_by(|a, b| a.score.total_cmp(&b.score)).copied() else { return };
        self.best_speed = best.speed;
        // il percorso migliore ha cominciato un elemento nuovo: quello prima e' finito in `best.start`
        let t = best.start;
        // solo cambi nuovi (il percorso migliore puo' tornare a un'ipotesi piu' vecchia)
        if (best.start.wrapping_sub(self.learned_at) as i32) <= 0 || best.start == best.prev_start {
            return;
        }
        self.learned_at = t;
        let s = usize::from(best.speed);
        let u = self.units[s];
        let a = best.prev_start;
        let n = t.wrapping_sub(a);
        if n == 0 || n as usize > self.mask {
            return;
        }
        if best.mark && best.marks == 0 {
            return;
        }
        if best.mark {
            // e' appena finita una pausa: rumore dalla sua parte centrale (il filtro sbava i toni per qualche ms) e pugno
            let edge = (0.012 * self.fs) as u32;
            if n > 2 * edge + (0.5 * u) as u32 {
                let (_, _, p, open) = self.sums(a.wrapping_add(edge), t.wrapping_sub(edge));
                if open == n - 2 * edge {
                    let mean = p / f64::from(open);
                    let w = (f64::from(n) / (2.0 * self.fs)).min(0.5);
                    self.noise += w * (mean - self.noise);
                    self.noise_ready = true;
                }
            }
            let (t1, t2) = self.gap_thresholds(s);
            let cls = if n < t1 {
                INTRA
            } else if n < t2 {
                CHARGAP
            } else {
                WORDGAP
            };
            if cls != WORDGAP || f64::from(n) < 12.0 * u {
                self.adapt_fist(cls, f64::from(n) / u);
            }
        } else {
            // e' appena finito un tono: ampiezza e pugno
            let (sr, si, _, _) = self.sums(a, t);
            let est = ((sr * sr + si * si) - self.noise * self.var_of(n as usize)) / f64::from(n * n);
            let w = (f64::from(n) / (0.6 * self.fs)).min(0.4);
            if est.is_finite() {
                self.signal += w * (est.max(0.05 * self.noise) - self.signal);
                self.marks_seen = self.marks_seen.saturating_add(1);
            }
            let units = f64::from(n) / u;
            let cls = if units < (self.fist_mean[DOT] * self.fist_mean[DASH]).sqrt() { DOT } else { DASH };
            self.adapt_fist(cls, units);
        }
    }

    /// Medie e larghezze del pugno: medie esponenziali lente dei logaritmi, dentro i loro limiti.
    fn adapt_fist(&mut self, cls: usize, units: f64) {
        if cls == DOT || units <= 0.0 {
            return;
        }
        // si comincia a imparare il pugno dopo qualche decina di elementi, piano, e solo da elementi che stanno nella loro classe
        self.learned = self.learned.saturating_add(1);
        let r = (units / self.fist_mean[cls]).ln();
        if self.learned < 20 || r.abs() > 0.4 {
            return;
        }
        let (lo, hi) = FIST_LIMITS[cls];
        self.fist_mean[cls] = (self.fist_mean[cls] * (0.03 * r).exp()).clamp(lo, hi);
        let v = self.fist_sigma[cls] * self.fist_sigma[cls];
        self.fist_sigma[cls] = (v + 0.02 * (r * r - v)).sqrt().clamp(FIST_SIGMA[cls] * 0.8, FIST_SIGMA[cls] * 1.3);
    }

    /// Stampa il primo pezzo in sospeso del percorso migliore quando ne ha altri dopo, o dopo `DELAY_S`; scarta chi diceva altro.
    fn commit(&mut self, out: &mut Vec<u8>) {
        loop {
            let Some(best) = self.beam.iter().max_by(|a, b| a.score.total_cmp(&b.score)).copied() else { return };
            let waited = f64::from(self.t.wrapping_sub(best.pend.since)) / self.fs;
            // a fine parola (la pausa aperta e' gia' lunga) si stampa tutto: le prove per quei caratteri sono gia' arrivate
            if best.pend.len == 0 || (usize::from(best.pend.len) <= DELAY_CHARS && waited < DELAY_S && !(best.word_done && !best.mark)) {
                return;
            }
            let tok = best.pend.toks[0];
            let (mut agree, mut all) = (0.0_f64, 0.0_f64);
            for h in &self.beam {
                let w = f64::from(h.score - best.score).exp();
                all += w;
                if h.pend.len > 0 && h.pend.toks[0] == tok {
                    agree += w;
                }
            }
            let t = self.t;
            self.beam.retain(|h| h.pend.len > 0 && h.pend.toks[0] == tok);
            for h in self.beam.iter_mut() {
                h.pend.pop_front(t);
            }
            if tok == SPACE_TOKEN {
                out.push(b' ');
            } else if let Some(text) = self.trie.text[usize::from(tok)] {
                out.extend_from_slice(text.as_bytes());
            }
            self.confidences.push((255.0 * agree / all.max(1e-30)).round().clamp(0.0, 255.0) as u8);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_morse_tree_has_every_character() {
        let t = Trie::new();
        for (c, p) in MORSE {
            let mut n = 0usize;
            for e in p.bytes() {
                n = usize::from(t.child[n][usize::from(e == b'-')]);
            }
            let text = t.text[n].unwrap_or("");
            assert!(text.as_bytes() == [*c] || text.starts_with('<'), "{p}: {text}");
        }
    }

    #[test]
    fn bessel_and_tail_are_sane() {
        assert!((ln_i0(0.0)).abs() < 1e-6);
        assert!((ln_i0(1.0) - 1.266_065_9_f64.ln()).abs() < 1e-5);
        assert!((ln_i0(10.0) - 2_815.716_6_f64.ln()).abs() < 1e-4);
        assert!((ln_upper_tail(0.0) - 0.5_f64.ln()).abs() < 1e-6);
        assert!((ln_upper_tail(1.0) - 0.158_655_25_f64.ln()).abs() < 1e-4);
        assert!(ln_upper_tail(8.0) < -30.0);
    }

    #[test]
    fn the_text_model_prefers_qso_words() {
        let lm = Lm::new();
        assert!(lm.score(b'C', b'Q') > lm.score(b'C', b'X'));
        assert!(lm.score(b'Q', b'X') < 0.0);
    }
}
