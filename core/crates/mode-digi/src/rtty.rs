//! RTTY (Baudot ITA2, 5 bit): FSK a fase continua, di base 45,45 baud e 170 Hz di spostamento.
//!
//! Ogni carattere è un bit di start (space), cinque bit di dati dal meno significativo (mark = 1) e 1,5 bit di stop (mark). I codici
//! 27 (FIGS) e 31 (LTRS) cambiano la tabella. Tabelle riprese da fldigi (`src/rtty/rtty.cxx`, Dave Freese W1HKJ e altri; GPL 3 o
//! successiva), versione americana delle cifre.
//!
//! Convenzione delle frequenze come in fldigi (`rtty.cxx`, `rtty::init` e `rtty::rx_process`): la frequenza scelta è il CENTRO fra i due
//! toni, il mark è il tono ALTO (`centro + shift/2`) e lo space quello basso; `reverse` li scambia (tasto «Rev», e da solo con la radio in
//! LSB: `reverse = Reverse ^ !USB`). Il centro è anche quello che la cascata disegna (la coppia di marcatori sta a ± shift/2 dalla
//! frequenza, la bandierina in mezzo), per cui il clic sulla cascata, l'AFC e Snap lavorano tutti sul centro del segnale.
//!
//! Il ricevitore segue quello di fldigi 4.2.13 (`rtty::rx_process`, `rtty::rx`, `rtty::is_mark_space`, `rtty::baudot_dec`,
//! `fftfilt::rtty_filter` in `src/filters/fftfilt.cxx`), con due idee in piu' (nessun
//! codice) e una nostra; ogni scelta e' misurata in `tests/rtty_confronto.rs`:
//! - un front-end decimato (oscillatore a rotazione sul centro, passa-basso FIR, circa 32 campioni per bit) porta la coppia di toni in
//!   banda base; poi due mixer portano mark e space a zero e due filtri passa-basso a coseno rialzato larghi in base al baud li separano
//!   (fldigi);
//! - controllo automatico della soglia (ATC): si parte dall'«Optimal ATC» di fldigi e si passa a una variante nostra a decisione guidata,
//!   che regge un tono che svanisce (fading selettivo, filtro della radio inclinato) anche con poco segnale;
//! - ogni campione da' un valore morbido (LLR: la decisione dell'ATC divisa per la varianza del rumore misurata sui due toni) e il carattere
//!   si decodifica intero: per ogni possibile centro dello start si somma quanto sono verosimili bit precedente a mark, start a space, i
//!   cinque dati e lo stop a mark, e vince il centro migliore in una finestra di un bit (passo 7 di `docs/progetto-rtty-oltre.md`);
//! - in alternativa (`set_frame_decoder(false)`) la macchina dei bit di fldigi: l'inizio del carattere si cerca sulle ultime decisioni
//!   lunghe un bit: mark all'inizio, space alla fine e metà dei campioni a mark;
//!   il numero di mark dice dov'è il passaggio e centra il campionamento a metà di ogni bit (fldigi); dentro il carattere ogni cambio di
//!   livello corregge la fase di metà dell'errore (un orologio di bit che si corregge sui fronti);
//! - in alternativa ai due filtri, un discriminatore di frequenza per gli spostamenti stretti;
//! - stop bit controllato, UOS (si torna alle lettere dopo uno spazio), CR CR e LF LF ripetuti scartati (fldigi);
//! - lo squelch guarda il rapporto segnale/rumore (come la «metric» di fldigi), non il livello assoluto.

use crate::psk::{AFC_LIMIT_HZ, RATE};
use std::f64::consts::PI;

const LETTERS: [u8; 32] = [
    0, b'E', b'\n', b'A', b' ', b'S', b'I', b'U', b'\r', b'D', b'R', b'J', b'N', b'F', b'C', b'K', b'T', b'Z', b'L', b'W', b'H', b'Y', b'P',
    b'Q', b'O', b'B', b'G', b' ', b'M', b'X', b'V', b' ',
];
const FIGURES: [u8; 32] = [
    0, b'3', b'\n', b'-', b' ', 7, b'8', b'7', b'\r', b'$', b'4', b'\'', b',', b'!', b':', b'(', b'5', b'"', b')', b'2', b'#', b'6', b'0',
    b'1', b'9', b'?', b'&', b' ', b'.', b'/', b';', b' ',
];
/// Distanza dei filtri di controllo dell'AFC dai due toni (Hz).
const PROBE_HZ: f32 = 8.0;
/// Distanza dei due filtri del rumore (uno sotto il tono basso, uno sopra quello alto): servono a capire se c'e' un segnale RTTY stretto o
/// solo rumore (rapporto segnale/rumore per lo squelch e per l'AFC).
const NOISE_GAP_HZ: f32 = 110.0;
/// L'AFC si accende quando i due toni superano il rumore di questo rapporto (in ampiezza, circa 11 dB) e si spegne sotto l'altro (circa 6 dB).
const LOCK_ON: f32 = 3.5;
const LOCK_OFF: f32 = 2.0;
/// Quanto sposta l'AFC a ogni bit (Hz per unita' di errore), e dopo quanti bit senza segnale (circa 8 s) la correzione rientra piano verso la frequenza scelta.
const AFC_GAIN: f32 = 0.4;
const AFC_HOLD_BITS: u32 = 360;
/// Fattore di forma del filtro di ricezione (`fftfilt::rtty_filter` di fldigi): la risposta va a zero a `K * baud` Hz dal tono. fldigi
/// 4.2.13 scrive nel codice 1,4 e nelle impostazioni (`rtty_filter`, non usato da quella versione) 1,25. Nella prova `studio` di
/// `tests/rtty_confronto.rs` (errori medi): K 1,0 8,0% · 1,2 7,1% · 1,35 7,1% · 1,5 7,8% · 1,8 8,9%; si tiene 1,35.
const FILTER_K: f64 = 1.35;
/// Lunghezza del filtro in bit (la risposta all'impulso cala come 1/t^3: oltre ±1,75 bit resta meno di un millesimo).
const FILTER_SPAN_BITS: f64 = 3.5;
/// Campioni per bit dopo la decimazione: bastano per centrare il campionamento (fldigi ne ha 176 a 8 kHz, ma la decisione e' la stessa).
const SAMPLES_PER_BIT: f64 = 32.0;
/// Le medie dei livelli (ATC a decisione guidata, discriminatore) si fanno su questi bit: la prova di confronto da' il meglio fra 4 e 8.
const LEVEL_BITS: f32 = 4.0;
/// Quanto dell'errore di fase si corregge a ogni cambio di livello: meta' (orologio di bit che si corregge sui fronti).
const PLL_GAIN: f32 = 0.5;
/// La decisione di ogni bit somma i valori morbidi (LLR) di questa frazione di bit intorno al centro, invece di guardare un campione solo
/// (passo 6: e' la «maggioranza» di MMTTY fatta sui valori morbidi).
const SOFT_BITS: f32 = 0.8;
/// Scala delle somme delle LLR per la probabilita' di un bit (passo 8): le LLR dei campioni vicini sono legate dal filtro e la loro somma
/// conta la stessa prova piu' volte; il valore viene dalla prova `taratura` di `tests/rtty_tabella.rs` (probabilita' detta contro caratteri
/// giusti davvero: con 0,03 i caratteri detti al 90-100 % sono giusti al 98 %, quelli al 50-60 % al 58 %).
const LLR_SCALE: f32 = 0.03;
/// Sotto questa probabilita' un carattere e' «incerto»: l'interfaccia lo mostra in chiaro.
pub const WEAK_PROB: f32 = 0.9;
const FIGS: u8 = 27;
const LTRS: u8 = 31;
const SPACE: u8 = 4;
const CR: u8 = 8;
const LF: u8 = 2;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RttyParams {
    /// Centro fra i due toni (Hz dell'audio), come in fldigi.
    pub centre_hz: f32,
    pub shift_hz: f32,
    pub baud: f32,
    /// Mark e space scambiati: mark sul tono basso (radio in LSB, o stazione che trasmette al contrario).
    pub reverse: bool,
}

impl Default for RttyParams {
    fn default() -> Self {
        Self { centre_hz: 2210.0, shift_hz: 170.0, baud: 45.45, reverse: false }
    }
}

impl RttyParams {
    /// Il tono del mark: quello alto in USB (`centro + shift/2`), quello basso con `reverse`.
    pub fn mark_hz(&self) -> f32 {
        if self.reverse { self.centre_hz - self.shift_hz / 2.0 } else { self.centre_hz + self.shift_hz / 2.0 }
    }

    pub fn space_hz(&self) -> f32 {
        if self.reverse { self.centre_hz + self.shift_hz / 2.0 } else { self.centre_hz - self.shift_hz / 2.0 }
    }
}

fn code_of(c: u8) -> Option<(u8, bool)> {
    let up = c.to_ascii_uppercase();
    let valid = |i: &usize| *i != 0 && *i != 27 && *i != 31;
    if let Some(i) = LETTERS.iter().position(|&x| x == up).filter(valid) {
        return Some((i as u8, false));
    }
    FIGURES.iter().position(|&x| x == up).filter(valid).map(|i| (i as u8, true))
}

fn push_frame(seq: &mut Vec<(bool, f64)>, code: u8) {
    seq.push((false, 1.0));
    for b in 0..5 {
        seq.push(((code >> b) & 1 == 1, 1.0));
    }
    seq.push((true, 1.5));
}

/// Audio (a 12 kHz) del testo; `amp` è l'ampiezza di picco (0..1). I caratteri che il Baudot non ha si saltano.
///
/// UOS in trasmissione: chi riceve con «unshift on space» acceso (fldigi e MMTTY lo hanno di base) torna alle lettere dopo uno spazio,
/// chi lo ha spento resta nelle cifre. Dopo uno spazio mandato fra le cifre il registro di chi riceve non e' quindi noto, e il carattere
/// seguente si fa precedere dal suo FIGS o LTRS: «599 73» arriva giusto a tutti e due (fldigi, `rtty::tx_char` con `UOStx`, manda un LTRS
/// prima di ogni spazio; qui si spende un carattere solo quando serve).
pub fn modulate(text: &[u8], params: RttyParams, amp: f32) -> Vec<f32> {
    synth(&sequence(text), params, amp, amp)
}

/// Come [`modulate`], con uno stop lungo `stop_bits` bit (1; 1,42; 1,5; 2): per le prove del ricevitore con le stazioni che non
/// trasmettono 1,5 bit.
pub fn modulate_stop(text: &[u8], params: RttyParams, amp: f32, stop_bits: f64) -> Vec<f32> {
    let seq: Vec<(bool, f64)> = sequence(text).into_iter().map(|(m, len)| if m && (len - 1.5).abs() < 1e-9 { (m, stop_bits) } else { (m, len) }).collect();
    synth(&seq, params, amp, amp)
}

/// Come [`modulate`], con ampiezze diverse per mark e space: un tono che arriva piu' debole dell'altro (fading selettivo, filtro della
/// radio inclinato), per le prove del ricevitore.
pub fn modulate_levels(text: &[u8], params: RttyParams, amp_mark: f32, amp_space: f32) -> Vec<f32> {
    synth(&sequence(text), params, amp_mark, amp_space)
}

/// I toni del testo: (mark?, durata in bit).
fn sequence(text: &[u8]) -> Vec<(bool, f64)> {
    let mut seq: Vec<(bool, f64)> = vec![(true, 8.0)];
    push_frame(&mut seq, LTRS);
    // il registro di chi riceve: `None` = non noto (dopo uno spazio fra le cifre)
    let mut shift: Option<bool> = Some(false);
    for &c in text {
        let Some((code, fig)) = code_of(c) else { continue };
        match code {
            SPACE => {
                push_frame(&mut seq, code);
                if shift == Some(true) {
                    shift = None;
                }
            }
            // i ritorni a capo valgono in tutte e due le tabelle e non cambiano il registro
            CR | LF => push_frame(&mut seq, code),
            _ => {
                if shift != Some(fig) {
                    push_frame(&mut seq, if fig { FIGS } else { LTRS });
                    shift = Some(fig);
                }
                push_frame(&mut seq, code);
            }
        }
    }
    seq.push((true, 4.0));
    seq
}

/// FSK a fase continua; le due ampiezze separate servono alle prove (un tono che svanisce).
fn synth(seq: &[(bool, f64)], params: RttyParams, amp_mark: f32, amp_space: f32) -> Vec<f32> {
    let spb = f64::from(RATE) / f64::from(params.baud);
    let mut out = Vec::new();
    let mut phase = 0.0_f64;
    let mut t_end = 0.0_f64;
    for &(mark, bits) in seq {
        t_end += bits * spb;
        let f = f64::from(if mark { params.mark_hz() } else { params.space_hz() });
        let amp = if mark { amp_mark } else { amp_space };
        let step = 2.0 * PI * f / f64::from(RATE);
        while (out.len() as f64) < t_end {
            out.push(amp * phase.sin() as f32);
            phase += step;
        }
        phase %= 2.0 * PI;
    }
    out
}

/// Una finestra mobile di somme (I, Q) alla frequenza data: la sua ampiezza dice quanto c'è di quel tono nell'ultimo bit (filtri di controllo
/// dell'AFC). Lavora sul segnale complesso del front-end, a frequenza `rate`, con la frequenza contata dal centro.
struct Tone {
    step: f64,
    phase: f64,
    ring_i: Vec<f32>,
    ring_q: Vec<f32>,
    sum_i: f32,
    sum_q: f32,
}

impl Tone {
    fn new(freq_hz: f32, len: usize, rate: f64) -> Self {
        Self { step: 2.0 * PI * f64::from(freq_hz) / rate, phase: 0.0, ring_i: vec![0.0; len], ring_q: vec![0.0; len], sum_i: 0.0, sum_q: 0.0 }
    }

    fn push(&mut self, z: (f32, f32), pos: usize) -> f32 {
        let (c, s) = (self.phase.cos() as f32, self.phase.sin() as f32);
        // z * e^(-j fase)
        let i = z.0 * c + z.1 * s;
        let q = z.1 * c - z.0 * s;
        self.phase += self.step;
        if self.phase > 2.0 * PI {
            self.phase -= 2.0 * PI;
        } else if self.phase < -2.0 * PI {
            self.phase += 2.0 * PI;
        }
        self.sum_i += i - self.ring_i[pos];
        self.sum_q += q - self.ring_q[pos];
        self.ring_i[pos] = i;
        self.ring_q[pos] = q;
        (self.sum_i * self.sum_i + self.sum_q * self.sum_q).sqrt() / self.ring_i.len() as f32
    }
}

/// La risposta all'impulso del filtro di ricezione, alla frequenza piena: la forma di `fftfilt::rtty_filter` di fldigi, cioe' in frequenza
/// `cos^2(pi f / 2B)` per |f| < B = K * baud e zero oltre (la correzione 1/sinc di fldigi in quella banda vale meno dello 0,1% e si tralascia).
/// Nel tempo e' la trasformata di una finestra di Hann larga 2B: `B [ sinc(u)/2 + sinc(u+1)/4 + sinc(u-1)/4 ]` con u = 2 B t.
/// Guadagno 1 in continua.
fn rtty_filter(baud: f64, rate: f64) -> Vec<f32> {
    lowpass(FILTER_K * baud, baud, rate)
}

/// Passa-basso con risposta `cos^2(pi f / 2B)` fino a `b_hz` e zero oltre, lungo `FILTER_SPAN_BITS` bit, alla frequenza di campionamento
/// `rate`; guadagno 1 in continua.
fn lowpass(b_hz: f64, baud: f64, rate: f64) -> Vec<f32> {
    let b = b_hz;
    let sinc = |x: f64| if x.abs() < 1e-9 { 1.0 } else { (PI * x).sin() / (PI * x) };
    let len = ((FILTER_SPAN_BITS * rate / baud).round() as usize).max(3) | 1;
    let mid = (len / 2) as f64;
    let h: Vec<f64> = (0..len)
        .map(|n| {
            let u = 2.0 * b * (n as f64 - mid) / rate;
            0.5 * sinc(u) + 0.25 * sinc(u + 1.0) + 0.25 * sinc(u - 1.0)
        })
        .collect();
    let sum: f64 = h.iter().sum();
    h.iter().map(|v| (v / sum) as f32).collect()
}

/// Il front-end decimato (passo 5 di `docs/progetto-rtty-oltre.md`): un oscillatore a rotazione porta il centro della coppia di toni a zero,
/// un passa-basso FIR (finestra di Blackman) tiene i toni e i due canali del rumore e la decimazione riduce la frequenza a `RATE / decim`
/// (circa 32 campioni per bit). Tutto il resto del ricevitore lavora su questo segnale complesso: i filtri dei toni costano molto meno che
/// alla frequenza piena.
struct FrontEnd {
    /// Il fasore dell'oscillatore (cos, sin) e la rotazione di un campione.
    rot: (f64, f64),
    step: (f64, f64),
    renorm: u32,
    h: Vec<f32>,
    re: Vec<f32>,
    im: Vec<f32>,
    pos: usize,
    decim: usize,
    count: usize,
}

impl FrontEnd {
    /// `half_band_hz`: meta' della banda da tenere (dal centro al bordo del canale del rumore piu' lontano).
    fn new(centre_hz: f32, half_band_hz: f64, decim: usize) -> Self {
        let rate = f64::from(RATE);
        let out_rate = rate / decim as f64;
        // banda passante fino a `half_band_hz`, banda oscura da dove l'aliasing ricadrebbe dentro (`out_rate - half_band_hz`)
        let stop = (out_rate - half_band_hz).max(half_band_hz + 50.0).min(rate / 2.0);
        let fc = 0.5 * (half_band_hz + stop) / rate;
        let width = (stop - half_band_hz).max(50.0);
        let len = ((5.5 * rate / width).ceil() as usize).clamp(15, 1501) | 1;
        let mid = (len / 2) as f64;
        let h: Vec<f64> = (0..len)
            .map(|k| {
                let n = k as f64 - mid;
                let sinc = if n == 0.0 { 2.0 * fc } else { (2.0 * PI * fc * n).sin() / (PI * n) };
                let w = 0.42 - 0.5 * (2.0 * PI * k as f64 / (len - 1) as f64).cos() + 0.08 * (4.0 * PI * k as f64 / (len - 1) as f64).cos();
                sinc * w
            })
            .collect();
        let sum: f64 = h.iter().sum();
        let mut fe = Self {
            rot: (1.0, 0.0),
            step: (1.0, 0.0),
            renorm: 0,
            h: h.iter().map(|v| (v / sum) as f32).collect(),
            re: vec![0.0; 2 * len],
            im: vec![0.0; 2 * len],
            pos: 0,
            decim,
            count: 0,
        };
        fe.retune(centre_hz);
        fe
    }

    /// Cambia la frequenza dell'oscillatore senza perdere la fase.
    fn retune(&mut self, centre_hz: f32) {
        let w = 2.0 * PI * f64::from(centre_hz) / f64::from(RATE);
        self.step = (w.cos(), w.sin());
    }

    /// Un campione nuovo; restituisce un campione decimato (I, Q) ogni `decim`.
    fn push(&mut self, s: f32) -> Option<(f32, f32)> {
        let len = self.h.len();
        // s * e^(-j fase)
        let (re, im) = ((f64::from(s) * self.rot.0) as f32, (-f64::from(s) * self.rot.1) as f32);
        let (c, d) = self.rot;
        self.rot = (c * self.step.0 - d * self.step.1, c * self.step.1 + d * self.step.0);
        self.renorm += 1;
        if self.renorm >= 1024 {
            self.renorm = 0;
            let m = (self.rot.0 * self.rot.0 + self.rot.1 * self.rot.1).sqrt();
            self.rot = (self.rot.0 / m, self.rot.1 / m);
        }
        self.re[self.pos] = re;
        self.re[self.pos + len] = re;
        self.im[self.pos] = im;
        self.im[self.pos + len] = im;
        self.pos = (self.pos + 1) % len;
        self.count += 1;
        if self.count < self.decim {
            return None;
        }
        self.count = 0;
        let (r, m) = (&self.re[self.pos..self.pos + len], &self.im[self.pos..self.pos + len]);
        let (mut i, mut q) = (0.0_f32, 0.0_f32);
        for k in 0..len {
            i += self.h[k] * r[k];
            q += self.h[k] * m[k];
        }
        Some((i, q))
    }
}

/// Un tono portato in banda base (mixer, sul segnale complesso del front-end) e filtrato con `rtty_filter`.
struct Channel {
    step: f64,
    phase: f64,
    /// Gli ultimi `len` campioni in banda base, scritti due volte (a `pos` e a `pos + len`) per leggerli sempre di seguito.
    re: Vec<f32>,
    im: Vec<f32>,
    pos: usize,
}

impl Channel {
    /// `freq_hz`: la frequenza del tono contata dal centro; `rate`: la frequenza dei campioni del front-end.
    fn new(freq_hz: f32, len: usize, rate: f64) -> Self {
        Self { step: 2.0 * PI * f64::from(freq_hz) / rate, phase: 0.0, re: vec![0.0; 2 * len], im: vec![0.0; 2 * len], pos: 0 }
    }

    fn push(&mut self, z: (f32, f32)) {
        let len = self.re.len() / 2;
        let (c, s) = (self.phase.cos() as f32, self.phase.sin() as f32);
        let (re, im) = (z.0 * c + z.1 * s, z.1 * c - z.0 * s);
        self.phase += self.step;
        if self.phase > 2.0 * PI {
            self.phase -= 2.0 * PI;
        } else if self.phase < -2.0 * PI {
            self.phase += 2.0 * PI;
        }
        self.re[self.pos] = re;
        self.re[self.pos + len] = re;
        self.im[self.pos] = im;
        self.im[self.pos + len] = im;
        self.pos = (self.pos + 1) % len;
    }

    /// L'uscita del filtro adesso (I, Q); `h` deve essere lungo quanto la memoria del canale.
    fn output(&self, h: &[f32]) -> (f32, f32) {
        let len = h.len();
        let (re, im) = (&self.re[self.pos..self.pos + len], &self.im[self.pos..self.pos + len]);
        let (mut i, mut q) = (0.0_f32, 0.0_f32);
        for k in 0..len {
            i += h[k] * re[k];
            q += h[k] * im[k];
        }
        (i, q)
    }

    /// L'ampiezza dell'uscita del filtro adesso (un tono di ampiezza A da' A/2).
    fn magnitude(&self, h: &[f32]) -> f32 {
        let (i, q) = self.output(h);
        (i * i + q * q).sqrt()
    }
}

/// Il discriminatore di frequenza : un solo mixer al centro, un
/// passa-basso largo quanto la coppia di toni, la frequenza istantanea dalla differenza di fase fra due campioni e una media su mezzo bit.
/// Serve con gli spostamenti stretti (sotto i 50 Hz, come i 23 Hz dell'elenco), dove i due filtri separati si sovrappongono.
struct Discriminator {
    channel: Channel,
    h: Vec<f32>,
    prev: (f32, f32),
    /// Media mobile della frequenza (Hz rispetto al centro) su mezzo bit.
    ring: Vec<f32>,
    pos: usize,
    sum: f32,
    /// Frequenza media del mark e dello space misurate (decisione guidata): la soglia sta in mezzo, e segue anche un piccolo errore di sintonia.
    f_mark: f32,
    f_space: f32,
}

impl Discriminator {
    fn new(params: &RttyParams, spb: f32, rate: f64) -> Self {
        let baud = f64::from(params.baud);
        let h = lowpass(f64::from(params.shift_hz) / 2.0 + FILTER_K * baud, baud, rate);
        let half = ((spb / 2.0).round() as usize).max(1);
        let (fm, fs) = (params.mark_hz() - params.centre_hz, params.space_hz() - params.centre_hz);
        // il segnale del front-end e' gia' centrato: il canale non sposta la frequenza
        Self { channel: Channel::new(0.0, h.len(), rate), h, prev: (0.0, 0.0), ring: vec![0.0; half], pos: 0, sum: 0.0, f_mark: fm, f_space: fs }
    }

    /// La frequenza istantanea mediata (Hz rispetto al centro); `rate` e' la frequenza dei campioni decimati.
    fn frequency(&mut self, rate: f32) -> f32 {
        let (i, q) = self.channel.output(&self.h);
        let (pi, pq) = self.prev;
        self.prev = (i, q);
        // z * conj(z_prima): il suo angolo e' il giro di fase fra i due campioni
        let f = (q * pi - i * pq).atan2(i * pi + q * pq) * rate / (2.0 * std::f32::consts::PI);
        self.sum += f - self.ring[self.pos];
        self.ring[self.pos] = f;
        self.pos = (self.pos + 1) % self.ring.len();
        self.sum / self.ring.len() as f32
    }
}

/// Un carattere riconosciuto da [`Frames`]: il codice a 5 bit e le somme delle LLR dei sette bit (start, cinque dati, stop).
#[derive(Clone, Copy, Debug)]
struct Frame {
    code: u8,
    sums: [f32; 7],
}

impl Frame {
    /// La probabilita' che il carattere sia giusto: il prodotto delle probabilita' dei sette bit, start a space e stop a mark compresi
    /// (le somme delle LLR scalate con `LLR_SCALE`).
    fn prob(&self) -> f32 {
        let sig = |x: f32| 1.0 / (1.0 + (-LLR_SCALE * x).exp());
        let mut p = sig(-self.sums[0]) * sig(self.sums[6]);
        for x in &self.sums[1..6] {
            p *= sig(x.abs());
        }
        p
    }
}

/// Un carattere ricevuto con la probabilita' che sia giusto (0..1). Sotto [`WEAK_PROB`] e' incerto e l'interfaccia lo mostra in chiaro.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RttyChar {
    pub ch: u8,
    pub prob: f32,
}

/// Decodifica del carattere intero (passo 7 di `docs/progetto-rtty-oltre.md`): invece di cercare lo start su decisioni dure e campionare
/// ogni bit una volta, si tengono le LLR degli ultimi 9 bit e per ogni possibile centro dello start si calcola quanto il carattere intero
/// e' verosimile: bit prima dello start a mark (pausa o stop del carattere precedente), start a space, cinque bit di dati comunque, stop a
/// mark. Il centro con il punteggio piu' alto in una finestra lunga un bit e' quello buono; i dati sono i segni delle somme a quei centri.
/// Il carattere arriva con circa 7 bit di ritardo.
struct Frames {
    /// Campioni per bit.
    t: f64,
    /// Campioni sommati intorno al centro di ogni bit.
    w: usize,
    /// Somme cumulative delle LLR (anello): `prefix[i % len]` = somma dei campioni prima dell'i-esimo.
    prefix: Vec<f64>,
    n: u64,
    /// Il primo centro di start ammesso (dopo lo stop del carattere precedente).
    next_min: f64,
    /// La ricerca in corso: inizio, centro migliore, punteggio migliore.
    search: Option<(f64, f64, f32)>,
}

impl Frames {
    fn new(spb: f32, w: usize) -> Self {
        let t = f64::from(spb);
        // la storia copre il bit prima dello start, il carattere e la finestra di ricerca lunga un bit
        let len = (9.5 * t).ceil() as usize + w + 8;
        Self { t, w: w.max(1), prefix: vec![0.0; len], n: 0, next_min: 0.0, search: None }
    }

    /// La somma delle LLR intorno al centro `c` (in campioni dall'inizio).
    fn sum(&self, c: f64) -> f32 {
        let len = self.prefix.len() as u64;
        let a = (c - self.w as f64 / 2.0).round().max(0.0) as u64;
        let b = a + self.w as u64;
        if b > self.n || self.n - a >= len {
            return 0.0;
        }
        (self.prefix[(b % len) as usize] - self.prefix[(a % len) as usize]) as f32
    }

    /// Le somme dei sette bit di un carattere con lo start centrato in `c`.
    fn sums(&self, c: f64) -> [f32; 7] {
        let mut v = [0.0; 7];
        for (k, x) in v.iter_mut().enumerate() {
            *x = self.sum(c + k as f64 * self.t);
        }
        v
    }

    /// Il punteggio del carattere con lo start centrato in `c`.
    fn score(&self, c: f64) -> f32 {
        let v = self.sums(c);
        self.sum(c - self.t) - v[0] + v[1..6].iter().map(|x| x.abs()).sum::<f32>() + v[6]
    }

    /// Un valore morbido nuovo; restituisce il carattere finito, se c'e'.
    fn push(&mut self, llr: f32) -> Option<Frame> {
        let len = self.prefix.len() as u64;
        let last = self.prefix[(self.n % len) as usize];
        self.n += 1;
        self.prefix[(self.n % len) as usize] = last + f64::from(llr);
        // l'ultimo centro di start per cui lo stop e' gia' tutto arrivato
        let c = self.n as f64 - 6.0 * self.t - self.w as f64 / 2.0 - 1.0;
        if c - self.t - (self.w as f64) < 0.0 || c < self.next_min {
            return None;
        }
        match self.search {
            None => {
                if self.sum(c) < 0.0 && self.sum(c - self.t) > 0.0 {
                    self.search = Some((c, c, self.score(c)));
                }
                None
            }
            Some((begin, best, best_score)) => {
                if c - begin <= self.t {
                    let sc = self.score(c);
                    if sc > best_score {
                        self.search = Some((begin, c, sc));
                    }
                    return None;
                }
                self.search = None;
                let sums = self.sums(best);
                if sums[0] < 0.0 && sums[6] > 0.0 && self.sum(best - self.t) > 0.0 {
                    // il prossimo start non prima di 6,5 bit (stop di un bit con mezzo bit di tolleranza)
                    self.next_min = best + 6.5 * self.t;
                    let mut code = 0;
                    for k in 0..5 {
                        if sums[k + 1] > 0.0 {
                            code |= 1 << k;
                        }
                    }
                    Some(Frame { code, sums })
                } else {
                    self.next_min = best + 0.5 * self.t;
                    None
                }
            }
        }
    }
}

/// La media mobile esponenziale di fldigi (`decayavg` in `src/include/misc.h`).
fn decayavg(average: f32, input: f32, weight: f32) -> f32 {
    if weight <= 1.0 { input } else { average + (input - average) / weight }
}

#[derive(Clone, Copy, PartialEq)]
enum State {
    /// Si aspetta il passaggio da mark a space (l'inizio di un carattere).
    Idle,
    /// Bit di start: fra `wait` campioni (decimati) si controlla che sia space a meta' bit.
    Start { wait: f32 },
    /// Bit di dati numero `bit` (0..5), poi lo stop (`bit` = 5).
    Data { wait: f32, bit: u8 },
}

pub struct RttyDemodulator {
    params: RttyParams,
    /// Correzione dell'AFC sulla frequenza scelta (Hz).
    offset_hz: f32,
    afc: bool,
    uos: bool,
    /// Soglia dello squelch sul rapporto segnale/rumore (dB); `None` = spento.
    squelch_snr_db: Option<f32>,
    level_db: f32,
    snr_db: f32,
    open: bool,
    /// Il front-end decimato, il filtro di ricezione e i quattro canali: mark, space e i due del rumore (sotto il tono basso e sopra quello
    /// alto).
    front: FrontEnd,
    h: Vec<f32>,
    decim: usize,
    mark: Channel,
    space: Channel,
    noise: [Channel; 2],
    /// Campioni (decimati) per bit, e quelli per bit arrotondati (lunghezza della memoria delle decisioni).
    spb: f32,
    /// ATC di fldigi: inviluppo e fondo del rumore di ogni tono.
    mark_env: f32,
    space_env: f32,
    mark_noise: f32,
    space_noise: f32,
    /// Le ultime decisioni lunghe un bit (`true` = mark), la prossima da scrivere e quante sono mark.
    bits: Vec<bool>,
    /// I valori morbidi (LLR, positivi = mark) degli stessi campioni.
    soft: Vec<f32>,
    /// La decodifica del carattere intero (passo 7); `None` = la macchina dei bit di fldigi.
    frames: Option<Frames>,
    /// Quanti campioni intorno al centro del bit entrano nella decisione.
    soft_len: usize,
    /// Varianza del rumore sulle ampiezze dei due toni (decisione guidata, mark acceso/spento, space acceso/spento): scala le LLR.
    var: [f32; 4],
    bit_pos: usize,
    marks: usize,
    state: State,
    data: u8,
    /// La probabilita' del carattere in decodifica e quella dell'ultimo cambio di registro (FIGS o LTRS), che vale per le lettere e le
    /// cifre dopo.
    data_prob: f32,
    shift_prob: f32,
    figures: bool,
    last_char: u8,
    /// Livelli medi (decisione guidata) di mark acceso, mark spento, space acceso, space spento.
    levels: [f32; 4],
    /// Da quanti campioni ogni media non si aggiorna.
    level_age: [u32; 4],
    /// Il discriminatore di frequenza, se scelto al posto dei due filtri.
    disc: Option<Discriminator>,
    /// Quattro filtri appena sopra e sotto i due toni (solo con l'AFC): dicono da che parte la portante e' scivolata.
    probes: [Tone; 4],
    probe_len: usize,
    probe_pos: usize,
    /// Somme lungo un bit delle ampiezze di mark, space e rumore; poi le medie lente che servono a squelch e AFC.
    sum_m: f32,
    sum_s: f32,
    sum_n: f32,
    sum_count: u32,
    last_probes: [f32; 4],
    mark_avg: f32,
    space_avg: f32,
    noise_avg: f32,
    err_avg: f32,
    /// C'e' un segnale RTTY (l'AFC lavora solo allora) e da quanti bit non c'e' piu'.
    locked: bool,
    lost_bits: u32,
}

impl RttyDemodulator {
    pub fn new(params: RttyParams) -> Self {
        let baud = f64::from(params.baud);
        let decim = ((f64::from(RATE) / (baud * SAMPLES_PER_BIT)).floor() as usize).max(1);
        let rate = f64::from(RATE) / decim as f64;
        let spb = (rate / baud) as f32;
        let h = rtty_filter(baud, rate);
        let len = h.len();
        let probe_len = (rate / baud).round() as usize;
        // i toni contati dal centro
        let (m, s) = (params.mark_hz() - params.centre_hz, params.space_hz() - params.centre_hz);
        let (lo, hi) = (m.min(s), m.max(s));
        let half_band = f64::from(hi + NOISE_GAP_HZ) + FILTER_K * baud;
        let nbits = (spb.round() as usize).max(4);
        Self {
            params,
            offset_hz: 0.0,
            afc: false,
            uos: true,
            squelch_snr_db: None,
            level_db: -90.0,
            snr_db: 0.0,
            open: true,
            front: FrontEnd::new(params.centre_hz, half_band, decim),
            h,
            decim,
            mark: Channel::new(m, len, rate),
            space: Channel::new(s, len, rate),
            noise: [Channel::new(lo - NOISE_GAP_HZ, len, rate), Channel::new(hi + NOISE_GAP_HZ, len, rate)],
            spb,
            mark_env: 0.0,
            space_env: 0.0,
            mark_noise: 0.0,
            space_noise: 0.0,
            bits: vec![true; nbits],
            soft: vec![0.0; nbits],
            soft_len: ((spb * SOFT_BITS).round() as usize).clamp(1, nbits),
            frames: Some(Frames::new(spb, ((spb * SOFT_BITS).round() as usize).clamp(1, nbits))),
            var: [0.0; 4],
            bit_pos: 0,
            marks: nbits,
            state: State::Idle,
            data: 0,
            data_prob: 1.0,
            shift_prob: 1.0,
            figures: false,
            last_char: 0,
            levels: [0.0; 4],
            level_age: [0; 4],
            disc: None,
            probes: [
                Tone::new(m + PROBE_HZ, probe_len, rate),
                Tone::new(m - PROBE_HZ, probe_len, rate),
                Tone::new(s + PROBE_HZ, probe_len, rate),
                Tone::new(s - PROBE_HZ, probe_len, rate),
            ],
            probe_len,
            probe_pos: 0,
            sum_m: 0.0,
            sum_s: 0.0,
            sum_n: 0.0,
            sum_count: 0,
            last_probes: [0.0; 4],
            mark_avg: 0.0,
            space_avg: 0.0,
            noise_avg: 0.0,
            err_avg: 0.0,
            locked: false,
            lost_bits: 0,
        }
    }

    /// Il centro su cui si sta ascoltando adesso (con l'AFC: la frequenza scelta piu' la correzione).
    pub fn frequency(&self) -> f32 {
        self.params.centre_hz + self.offset_hz
    }

    pub fn set_afc(&mut self, on: bool) {
        self.afc = on;
        if !on && self.offset_hz != 0.0 {
            self.offset_hz = 0.0;
            self.retune_all();
        }
    }

    /// UOS («unshift on space»): dopo uno spazio si torna alle lettere, come fa fldigi di base. Acceso alla creazione.
    pub fn set_uos(&mut self, on: bool) {
        self.uos = on;
    }

    /// Ricevitore a discriminatore di frequenza (`true`) invece dei due filtri sui toni (di base).
    pub fn set_discriminator(&mut self, on: bool) {
        if on == self.disc.is_some() {
            return;
        }
        let rate = f64::from(RATE) / self.decim as f64;
        self.disc = on.then(|| Discriminator::new(&self.params, self.spb, rate));
    }

    /// Decodifica del carattere intero (`true`, di base) o macchina dei bit di fldigi (`false`).
    pub fn set_frame_decoder(&mut self, on: bool) {
        let w = self.soft_len;
        self.frames = on.then(|| Frames::new(self.spb, w));
    }

    /// Soglia dello squelch sul rapporto segnale/rumore in dB (`snr_db`); `None` lo spegne.
    pub fn set_squelch_snr_db(&mut self, db: Option<f32>) {
        self.squelch_snr_db = db;
    }

    /// Livello del segnale in dB e se lo squelch lo lascia passare.
    pub fn signal(&self) -> (f32, bool) {
        (self.level_db, self.open)
    }

    /// Rapporto segnale/rumore in dB: l'ampiezza dei due toni contro quella di due canali uguali fuori dalla coppia (0 dB = solo rumore).
    pub fn snr_db(&self) -> f32 {
        self.snr_db
    }

    /// L'AFC sposta solo l'oscillatore del front-end: i toni, i filtri di controllo e i canali del rumore sono contati dal centro.
    fn retune_all(&mut self) {
        self.front.retune(self.params.centre_hz + self.offset_hz);
    }

    /// Una volta per bit: rapporto segnale/rumore (squelch) e AFC. L'AFC si muove solo se c'e' un segnale RTTY: i due toni devono superare di
    /// molto il rumore misurato fuori dalla coppia (un segnale largo, la voce o il solo rumore alzano anche quello e non passano). Senza segnale
    /// resta fermo e, dopo circa 8 secondi, rientra piano verso la frequenza scelta, cosi' la stazione dopo si cerca da li'.
    fn bit_step(&mut self) {
        const SMOOTH: f32 = 0.15;
        let n = self.sum_count.max(1) as f32;
        let (m, sp, nz) = (self.sum_m / n, self.sum_s / n, self.sum_n / n);
        self.sum_m = 0.0;
        self.sum_s = 0.0;
        self.sum_n = 0.0;
        self.sum_count = 0;
        self.mark_avg += SMOOTH * (m - self.mark_avg);
        self.space_avg += SMOOTH * (sp - self.space_avg);
        self.noise_avg += SMOOTH * (nz - self.noise_avg);
        let signal = self.mark_avg + self.space_avg;
        let snr = signal / (self.noise_avg + 1e-6);
        self.snr_db = (20.0 * snr.max(1e-3).log10()).clamp(-20.0, 60.0);
        self.open = self.squelch_snr_db.is_none_or(|t| self.snr_db >= t);
        if !self.afc {
            return;
        }
        if self.locked {
            if snr < LOCK_OFF || signal < 1e-3 {
                self.locked = false;
            }
        } else if snr > LOCK_ON && signal > 2e-3 {
            self.locked = true;
        }
        if self.locked && self.open {
            self.lost_bits = 0;
            // da che parte e' scivolata la portante, pesando ogni tono per quanto e' presente (in pausa trasmette solo il mark)
            let p = self.last_probes;
            let em = (p[0] - p[1]) / (p[0] + p[1]).max(1e-6);
            let es = (p[2] - p[3]) / (p[2] + p[3]).max(1e-6);
            let (wm, ws) = (self.mark_avg / signal.max(1e-6), self.space_avg / signal.max(1e-6));
            self.err_avg += 0.3 * (wm * em + ws * es - self.err_avg);
            if self.err_avg.abs() > 0.03 {
                self.offset_hz = (self.offset_hz + AFC_GAIN * self.err_avg).clamp(-AFC_LIMIT_HZ, AFC_LIMIT_HZ);
                self.retune_all();
            }
            return;
        }
        self.err_avg = 0.0;
        self.lost_bits = self.lost_bits.saturating_add(1);
        if self.lost_bits > AFC_HOLD_BITS && self.offset_hz != 0.0 {
            self.offset_hz -= self.offset_hz.signum() * self.offset_hz.abs().min(0.05);
            self.retune_all();
        }
    }

    /// La decisione del bit dalle due ampiezze filtrate (ATC, controllo automatico della soglia).
    ///
    /// Si parte dall'«Optimal ATC» di fldigi (`rtty::rx_process`, `v3`): per ogni tono inviluppo con salita veloce (un quarto di bit) e
    /// discesa lenta (16 bit), fondo del rumore con discesa veloce e salita lenta (48 bit), ampiezze ritagliate fra i due. Appena ci sono
    /// le misure si passa alla nostra variante a decisione guidata: per ogni tono la media di quando e' acceso e di quando e' spento (su
    /// 4 bit), la soglia a meta' fra le due e ogni tono pesato per quanto le due medie sono lontane. Il fondo di fldigi segue i minimi del
    /// rumore e mette la soglia di un tono debole dentro il rumore: con lo space 12 dB sotto il mark e 3 dB di S/N fldigi sbaglia il 45%
    /// dei caratteri, questa variante il 3% (`tests/rtty_confronto.rs`, prova `studio`); con i due toni uguali le due fanno lo stesso.
    fn decide(&mut self, m: f32, s: f32) -> f32 {
        let sl = self.spb;
        self.mark_env = decayavg(self.mark_env, m, if m > self.mark_env { sl / 4.0 } else { sl * 16.0 });
        self.mark_noise = decayavg(self.mark_noise, m, if m < self.mark_noise { sl / 4.0 } else { sl * 48.0 });
        self.space_env = decayavg(self.space_env, s, if s > self.space_env { sl / 4.0 } else { sl * 16.0 });
        self.space_noise = decayavg(self.space_noise, s, if s < self.space_noise { sl / 4.0 } else { sl * 48.0 });
        let [m_on, m_off, s_on, s_off] = self.levels;
        // un tono mai visto acceso (o con le medie scambiate dal rumore) pesa zero: decide l'altro, e intanto la sua media si rifa'
        let (dm, ds) = ((m_on - m_off).max(0.0), (s_on - s_off).max(0.0));
        let v = if dm + ds > 0.05 * (self.mark_env + self.space_env) {
            (m - 0.5 * (m_on + m_off)) * dm - (s - 0.5 * (s_on + s_off)) * ds
        } else {
            let floor = self.mark_noise.min(self.space_noise);
            let mc = m.min(self.mark_env).max(floor) - floor;
            let sc = s.min(self.space_env).max(floor) - floor;
            let (me, se) = (self.mark_env - floor, self.space_env - floor);
            mc * me - sc * se - 0.25 * (me * me - se * se)
        };
        let w = sl * LEVEL_BITS;
        // il rumore in banda: lo scarto delle due ampiezze dal livello che la decisione dice (passo 6), su una media piu' lunga dei livelli
        let (a, b) = if v > 0.0 { (0, 3) } else { (1, 2) };
        let wv = 4.0 * w;
        self.var[a] = decayavg(self.var[a], (m - self.levels[a]).powi(2), wv);
        self.var[b] = decayavg(self.var[b], (s - self.levels[b]).powi(2), wv);
        if v > 0.0 {
            self.levels[0] = decayavg(self.levels[0], m, w);
            self.levels[3] = decayavg(self.levels[3], s, w);
        } else {
            self.levels[1] = decayavg(self.levels[1], m, w);
            self.levels[2] = decayavg(self.levels[2], s, w);
        }
        // una media non aggiornata da piu' di 16 bit (un livello che non si decide mai, per esempio rimasto da un rumore forte di prima)
        // non supera l'inviluppo di fldigi: non blocca la decisione
        let (upd_a, upd_b) = if v > 0.0 { (0, 3) } else { (1, 2) };
        for (i, l) in self.levels.iter_mut().enumerate() {
            self.level_age[i] = if i == upd_a || i == upd_b { 0 } else { self.level_age[i].saturating_add(1) };
            if self.level_age[i] as f32 > 16.0 * sl {
                *l = l.min(if i < 2 { self.mark_env } else { self.space_env });
            }
        }
        v
    }

    /// La decisione del discriminatore: la frequenza mediata contro la soglia a meta' fra le frequenze medie di mark e space.
    fn decide_disc(&mut self) -> f32 {
        let rate = RATE / self.decim as f32;
        let w = self.spb * LEVEL_BITS;
        let Some(d) = &mut self.disc else { return 0.0 };
        let f = d.frequency(rate);
        let span = d.f_mark - d.f_space;
        let v = (f - 0.5 * (d.f_mark + d.f_space)) * span.signum();
        // le medie restano dalla parte giusta del centro (un rumore lungo non le scambia)
        let half = 0.5 * self.params.shift_hz;
        let mark_up = self.params.mark_hz() > self.params.space_hz();
        if v > 0.0 {
            d.f_mark = decayavg(d.f_mark, f, w);
        } else {
            d.f_space = decayavg(d.f_space, f, w);
        }
        let (lo, hi) = (-2.0 * half, 2.0 * half);
        if mark_up {
            d.f_mark = d.f_mark.clamp(0.1 * half, hi);
            d.f_space = d.f_space.clamp(lo, -0.1 * half);
        } else {
            d.f_mark = d.f_mark.clamp(lo, -0.1 * half);
            d.f_space = d.f_space.clamp(0.1 * half, hi);
        }
        v
    }

    /// La decisione di `age` campioni fa (0 = l'ultima).
    fn bit_at(&self, age: usize) -> bool {
        let n = self.bits.len();
        self.bits[(self.bit_pos + n - 1 - age.min(n - 1)) % n]
    }

    /// La decisione sul bit che e' appena finito: la somma dei valori morbidi intorno al suo centro.
    fn mid_bit(&self) -> bool {
        let n = self.bits.len();
        let first = n / 2 + self.soft_len / 2;
        let sum: f32 = (0..self.soft_len).map(|k| self.soft[(self.bit_pos + 2 * n - 1 - (first - k).min(n - 1)) % n]).sum();
        sum > 0.0
    }

    /// La LLR di un campione: la decisione dell'ATC divisa per la varianza del rumore misurata sui due toni (0 finche' non c'e' misura).
    fn llr(&self, v: f32) -> f32 {
        let var = 0.25 * self.var.iter().sum::<f32>();
        if var > 0.0 { v / var } else { v }
    }

    /// Una decisione nuova nella macchina dei caratteri (`rtty::rx` di fldigi).
    fn rx_bit(&mut self, v: f32, out: &mut Vec<RttyChar>) {
        let n = self.bits.len();
        let bit = v > 0.0;
        if self.bits[self.bit_pos] {
            self.marks -= 1;
        }
        self.soft[self.bit_pos] = v;
        self.bits[self.bit_pos] = bit;
        if bit {
            self.marks += 1;
        }
        self.bit_pos = (self.bit_pos + 1) % n;
        // l'orologio di bit (un PLL): a ogni cambio di livello dentro il carattere la fase si corregge di meta' dell'errore. Il
        // confine fra due bit cade quando `wait` arriva a zero (si campiona la decisione di mezzo bit prima): un cambio con `wait` piccolo
        // e' arrivato in anticipo, con `wait` vicino a un bit in ritardo. fldigi centra solo sullo start; le due cose insieme reggono meglio
        // un errore di velocita' o una deriva lungo il carattere.
        if bit != self.bit_at(1)
            && let State::Data { wait, bit: b } = self.state
        {
            let e = if wait < self.spb / 2.0 { wait } else { wait - self.spb };
            self.state = State::Data { wait: wait - PLL_GAIN * e, bit: b };
        }
        match self.state {
            State::Idle => {
                // `rtty::is_mark_space`: mark all'inizio della memoria, space alla fine e il passaggio a meta' (fldigi vuole lo scarto sotto
                // 6 campioni su 176; qui la stessa proporzione, almeno uno). Il numero di mark e' il tempo che manca a meta' dello start.
                let tol = ((n as f32 * 6.0 / 176.0).round() as usize).max(1);
                if self.bit_at(n - 1) && !self.bit_at(0) && (n / 2).abs_diff(self.marks) <= tol {
                    self.state = State::Start { wait: self.marks as f32 };
                }
            }
            State::Start { wait } => {
                if wait > 1.0 {
                    self.state = State::Start { wait: wait - 1.0 };
                } else if self.mid_bit() {
                    self.state = State::Idle;
                } else {
                    self.data = 0;
                    self.state = State::Data { wait: wait - 1.0 + self.spb, bit: 0 };
                }
            }
            State::Data { wait, bit } => {
                if wait > 1.0 {
                    self.state = State::Data { wait: wait - 1.0, bit };
                } else if bit < 5 {
                    if self.mid_bit() {
                        self.data |= 1 << bit;
                    }
                    self.state = State::Data { wait: wait - 1.0 + self.spb, bit: bit + 1 };
                } else {
                    // lo stop deve essere mark; con lo squelch acceso si stampa solo se il rapporto segnale/rumore lo apre (come fldigi)
                    if self.mid_bit() && self.open {
                        self.data_prob = 1.0;
                        self.emit(out);
                    }
                    self.state = State::Idle;
                }
            }
        }
    }

    /// Elabora altro audio; restituisce i caratteri nuovi.
    pub fn process(&mut self, samples: &[f32]) -> Vec<u8> {
        self.process_chars(samples).into_iter().map(|c| c.ch).collect()
    }

    /// Come [`Self::process`], con la probabilita' che ogni carattere sia giusto (sempre 1 con la macchina dei bit di fldigi).
    pub fn process_chars(&mut self, samples: &[f32]) -> Vec<RttyChar> {
        let mut out = Vec::new();
        for &s in samples {
            let Some(z) = self.front.push(s) else { continue };
            self.mark.push(z);
            self.space.push(z);
            self.noise[0].push(z);
            self.noise[1].push(z);
            if let Some(d) = &mut self.disc {
                d.channel.push(z);
            }
            if self.afc {
                let pos = self.probe_pos;
                for (i, t) in self.probes.iter_mut().enumerate() {
                    self.last_probes[i] = t.push(z, pos);
                }
            }
            self.probe_pos += 1;
            if self.probe_pos >= self.probe_len {
                self.probe_pos = 0;
                self.bit_step();
            }
            let m = self.mark.magnitude(&self.h);
            let sp = self.space.magnitude(&self.h);
            let nz = 0.5 * (self.noise[0].magnitude(&self.h) + self.noise[1].magnitude(&self.h));
            self.sum_m += m;
            self.sum_s += sp;
            self.sum_n += 2.0 * nz;
            self.sum_count += 1;
            self.level_db = (20.0 * (m + sp).max(1e-6).log10()).max(-90.0);
            let v = if self.disc.is_some() { self.decide_disc() } else { self.decide(m, sp) };
            let l = self.llr(v);
            match &mut self.frames {
                Some(f) => {
                    if let Some(fr) = f.push(l)
                        && self.open
                    {
                        self.data = fr.code;
                        self.data_prob = fr.prob();
                        self.emit(&mut out);
                    }
                }
                None => self.rx_bit(l, &mut out),
            }
        }
        out
    }

    /// `rtty::baudot_dec` di fldigi, con UOS e senza CR CR o LF LF ripetuti. La probabilita' di una lettera o di una cifra tiene conto
    /// anche del cambio di registro da cui dipende.
    fn emit(&mut self, out: &mut Vec<RttyChar>) {
        let mut prob = self.data_prob;
        let c = match self.data {
            FIGS => {
                self.figures = true;
                self.shift_prob = prob;
                return;
            }
            LTRS => {
                self.figures = false;
                self.shift_prob = prob;
                return;
            }
            0 => return,
            SPACE => {
                if self.uos {
                    self.figures = false;
                    self.shift_prob = 1.0;
                }
                b' '
            }
            CR => b'\r',
            LF => b'\n',
            c => {
                prob *= self.shift_prob;
                if self.figures {
                    FIGURES[c as usize]
                } else {
                    LETTERS[c as usize]
                }
            }
        };
        if (c == b'\r' || c == b'\n') && c == self.last_char {
            return;
        }
        self.last_char = c;
        out.push(RttyChar { ch: c, prob });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(centre_hz: f32, shift_hz: f32, baud: f32) -> RttyParams {
        RttyParams { centre_hz, shift_hz, baud, reverse: false }
    }

    fn andata_e_ritorno(params: RttyParams, text: &str) -> String {
        let audio = modulate(text.as_bytes(), params, 0.5);
        let mut rx = RttyDemodulator::new(params);
        let mut all = vec![0.0; 4000];
        all.extend(audio);
        all.extend(vec![0.0; 4000]);
        String::from_utf8_lossy(&rx.process(&all)).into_owned()
    }

    #[test]
    fn lettere() {
        let r = andata_e_ritorno(RttyParams::default(), "RYRYRY CQ DE IU3ABC K");
        assert!(r.contains("RYRYRY CQ DE IU"), "ricevuto: {r:?}");
    }

    #[test]
    fn cifre_e_ritorni_a_capo() {
        let r = andata_e_ritorno(RttyParams::default(), "CQ TEST 599 73\r\nDE IU3ABC");
        assert!(r.contains("CQ TEST 599 73\r\nDE IU3ABC"), "ricevuto: {r:?}");
    }

    #[test]
    fn altri_toni() {
        let r = andata_e_ritorno(p(1360.0, 170.0, 45.45), "THE QUICK BROWN FOX 1234567890");
        assert!(r.contains("THE QUICK BROWN FOX 1234567890"), "ricevuto: {r:?}");
    }

    #[test]
    fn il_mark_e_il_tono_alto_in_usb() {
        let q = p(1500.0, 170.0, 45.45);
        assert_eq!((q.mark_hz(), q.space_hz()), (1585.0, 1415.0));
        let r = RttyParams { reverse: true, ..q };
        assert_eq!((r.mark_hz(), r.space_hz()), (1415.0, 1585.0));
        // in pausa si trasmette il mark: l'audio iniziale sta sul tono alto
        let audio = modulate(b"", q, 0.5);
        let mut alto = Tone::new(1585.0, 264, f64::from(RATE));
        let mut basso = Tone::new(1415.0, 264, f64::from(RATE));
        let (mut a, mut b) = (0.0, 0.0);
        for (i, &s) in audio[..2000].iter().enumerate() {
            a = alto.push((s, 0.0), i % 264);
            b = basso.push((s, 0.0), i % 264);
        }
        assert!(a > 10.0 * b, "mark {a} space {b}");
    }

    #[test]
    fn con_mark_e_space_scambiati_si_decodifica_solo_con_rev() {
        let tx = RttyParams { reverse: true, ..p(1500.0, 170.0, 45.45) };
        let audio = modulate(b"RYRYRY CQ DE IU3ABC IU3ABC K", tx, 0.5);
        let mut giusto = RttyDemodulator::new(tx);
        let r = String::from_utf8_lossy(&giusto.process(&audio)).into_owned();
        assert!(r.contains("CQ DE IU3ABC"), "ricevuto con Rev: {r:?}");
        let mut sbagliato = RttyDemodulator::new(p(1500.0, 170.0, 45.45));
        let r = String::from_utf8_lossy(&sbagliato.process(&audio)).into_owned();
        assert!(!r.contains("IU3ABC"), "ricevuto senza Rev: {r:?}");
    }

    #[test]
    fn uos_dopo_lo_spazio_si_torna_alle_lettere() {
        // un trasmettitore senza UOS che non rimanda LTRS dopo lo spazio: FIGS 5 9 9 spazio, poi le lettere T U senza LTRS
        let mut seq: Vec<(bool, f64)> = vec![(true, 8.0)];
        for code in [LTRS, FIGS, 16, 24, 24, SPACE, 16, 7] {
            push_frame(&mut seq, code);
        }
        seq.push((true, 4.0));
        let params = p(1500.0, 170.0, 45.45);
        let audio = synth(&seq, params, 0.5, 0.5);
        let mut rx = RttyDemodulator::new(params);
        assert_eq!(String::from_utf8_lossy(&rx.process(&audio)), "599 TU");
        let mut rx = RttyDemodulator::new(params);
        rx.set_uos(false);
        assert_eq!(String::from_utf8_lossy(&rx.process(&audio)), "599 57");
    }

    #[test]
    fn in_trasmissione_dopo_lo_spazio_fra_le_cifre_si_rimanda_il_registro() {
        // anche un ricevitore con UOS (fldigi, MMTTY) riceve giuste le cifre dopo lo spazio
        let r = andata_e_ritorno(RttyParams::default(), "RST 599 73 TU");
        assert!(r.contains("RST 599 73 TU"), "ricevuto: {r:?}");
    }

    #[test]
    fn l_afc_insegue_i_toni() {
        let tx = p(2222.0, 170.0, 45.45);
        let audio = modulate(b"RYRYRY CQ CQ DE IU3ABC IU3ABC PSE K RYRYRY", tx, 0.5);
        let mut rx = RttyDemodulator::new(RttyParams::default());
        rx.set_afc(true);
        let mut all = vec![0.0; 4000];
        all.extend(audio);
        all.extend(vec![0.0; 4000]);
        let r = String::from_utf8_lossy(&rx.process(&all)).into_owned();
        assert!((rx.frequency() - 2222.0).abs() < 6.0, "frequenza raggiunta: {}", rx.frequency());
        assert!(r.contains("IU3ABC"), "ricevuto: {r:?}");
    }

    fn rumore(n: usize, ampiezza: f32, mut seed: u32) -> Vec<f32> {
        (0..n)
            .map(|_| {
                seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
                (seed as f32 / u32::MAX as f32 - 0.5) * ampiezza
            })
            .collect()
    }

    #[test]
    fn l_afc_non_si_muove_sul_solo_rumore() {
        let mut rx = RttyDemodulator::new(RttyParams::default());
        rx.set_afc(true);
        // dieci secondi di rumore, anche forte: la frequenza resta quella scelta
        for (i, amp) in [0.02_f32, 0.2, 0.8].into_iter().enumerate() {
            rx.process(&rumore(120_000, amp, 11 + i as u32));
            assert!((rx.frequency() - 2210.0).abs() < 0.5, "con rumore {amp} la frequenza e' andata a {}", rx.frequency());
        }
    }

    #[test]
    fn l_afc_non_segue_un_tono_fisso_dentro_il_rumore_forte() {
        // un solo tono vicino al mark sepolto in un rumore molto piu' forte non e' un segnale RTTY
        let mut rx = RttyDemodulator::new(RttyParams::default());
        rx.set_afc(true);
        let tono: Vec<f32> = rumore(120_000, 0.8, 5).iter().enumerate().map(|(i, r)| r + 0.05 * (2.0 * std::f32::consts::PI * 2310.0 * i as f32 / 12_000.0).sin()).collect();
        rx.process(&tono);
        assert!((rx.frequency() - 2210.0).abs() < 1.0, "frequenza: {}", rx.frequency());
    }

    #[test]
    fn l_afc_con_rumore_insegue_il_segnale_e_poi_rientra() {
        let tx = p(2225.0, 170.0, 45.45);
        let mut audio = modulate(b"RYRYRYRYRYRY CQ CQ DE IU3ABC IU3ABC PSE K RYRYRYRYRY", tx, 0.4);
        for (a, r) in audio.iter_mut().zip(rumore(200_000, 0.1, 3)) {
            *a += r;
        }
        let mut rx = RttyDemodulator::new(RttyParams::default());
        rx.set_afc(true);
        rx.process(&audio);
        assert!((rx.frequency() - 2225.0).abs() < 6.0, "frequenza raggiunta: {}", rx.frequency());
        // finito il segnale: ferma per circa 8 s, poi rientra piano verso la frequenza scelta
        rx.process(&rumore(60_000, 0.1, 9));
        let ferma = rx.frequency();
        assert!((ferma - 2225.0).abs() < 8.0, "dopo 5 s senza segnale la frequenza si e' mossa: {ferma}");
        rx.process(&rumore(1_200_000, 0.1, 10));
        assert!((rx.frequency() - 2210.0).abs() < 1.0, "dopo molto tempo senza segnale non e' rientrata: {}", rx.frequency());
    }

    #[test]
    fn lo_squelch_guarda_il_rapporto_segnale_rumore() {
        let mut rx = RttyDemodulator::new(RttyParams::default());
        rx.set_squelch_snr_db(Some(6.0));
        // rumore forte o debole: nessun carattere, lo squelch resta chiuso
        assert!(rx.process(&rumore(36_000, 0.8, 7)).is_empty());
        assert!(!rx.signal().1, "S/N sul solo rumore: {}", rx.snr_db());
        assert!(rx.process(&rumore(36_000, 0.002, 8)).is_empty());
        assert!(!rx.signal().1);
        // un segnale debole (ma pulito) apre: conta il rapporto, non il livello
        let audio = modulate(b"RYRY CQ DE IU3ABC IU3ABC K", RttyParams::default(), 0.01);
        let r = String::from_utf8_lossy(&rx.process(&audio)).into_owned();
        assert!(r.contains("IU3ABC"), "ricevuto: {r:?}");
        assert!(rx.snr_db() > 20.0, "S/N: {}", rx.snr_db());
    }

    #[test]
    fn un_tono_che_svanisce_non_ferma_la_decodifica() {
        // lo space 20 dB sotto il mark (fading selettivo), e poi il contrario: con l'ATC si decodifica lo stesso
        let params = RttyParams::default();
        let seq = sequence(b"RYRYRY CQ DE IU3ABC IU3ABC K");
        for (am, asp) in [(0.5, 0.05), (0.05, 0.5)] {
            let mut rx = RttyDemodulator::new(params);
            let r = String::from_utf8_lossy(&rx.process(&synth(&seq, params, am, asp))).into_owned();
            assert!(r.contains("CQ DE IU3ABC"), "mark {am} space {asp}, ricevuto: {r:?}");
        }
    }

    #[test]
    fn i_caratteri_hanno_la_loro_probabilita() {
        // segnale pulito: tutti certi; solo rumore forte: quasi tutti incerti
        let audio = modulate(b"RYRYRY CQ DE IU3ABC 599 K", RttyParams::default(), 0.5);
        let mut rx = RttyDemodulator::new(RttyParams::default());
        let chars = rx.process_chars(&audio);
        assert!(!chars.is_empty() && chars.iter().all(|c| c.prob >= WEAK_PROB), "{chars:?}");
        let mut rx = RttyDemodulator::new(RttyParams::default());
        let chars = rx.process_chars(&rumore(240_000, 0.8, 21));
        let weak = chars.iter().filter(|c| c.prob < WEAK_PROB).count();
        assert!(weak * 10 >= chars.len() * 8, "incerti {weak} su {}", chars.len());
    }

    #[test]
    fn baud_50() {
        let r = andata_e_ritorno(p(2350.0, 450.0, 50.0), "CQ DE IU3ABC");
        assert!(r.contains("CQ DE IU3ABC"), "ricevuto: {r:?}");
    }

    #[test]
    fn baud_75_e_shift_850() {
        let r = andata_e_ritorno(p(1700.0, 850.0, 75.0), "CQ DE IU3ABC 599");
        assert!(r.contains("CQ DE IU3ABC 599"), "ricevuto: {r:?}");
    }
}
