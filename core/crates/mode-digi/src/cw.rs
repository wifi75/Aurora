//! CW (alfabeto Morse): trasmissione con un tono audio dai fronti arrotondati e ricezione con un rivelatore a soglia adattiva.
//!
//! Le regole del Morse: l'unità è un punto, una linea vale 3 unità, la pausa fra gli elementi 1, fra le lettere 3 e fra le parole 7; la
//! velocità in parole al minuto (WPM) usa la parola PARIS di 50 unità, quindi un'unità dura 1,2 / WPM secondi. La tabella dei caratteri è
//! quella del Morse internazionale (ITU-R M.1677) con i segni di procedura <AR> <BT> <KN> <SK>.
//!
//! Il ricevitore prende il meglio di fldigi 4.2.13 (`src/cw/cw.cxx`, Dave Freese W1HKJ e contributori, GPL 3, copia in
//! `terze-parti/fldigi`), con alcune scelte nostre (dall'analisi in `docs/analisi-psk-cw.md`):
//! - mixer al tono, filtro FIR passa-basso a finestra lungo 64 ms (i 512 coefficienti a 8 kHz di fldigi) e decimazione a
//!   500 Hz; banda scelta fra 50, 100, 150, 250 e 500 Hz (150 di base, come fldigi);
//! - ampiezza |z| e media mobile (fldigi `bitfilter`, 1/3 di punto; qui 1/2, che regge meglio il rumore), legata pero' alla velocità
//!   RICEVUTA e non a quella di trasmissione;
//! - AGC che sale in fretta e scende piano come quello di fldigi (`cw::decode_stream`), con picco del segnale e fondo del rumore presi come
//!   quantili del livello (le medie di fldigi sul solo rumore danno 17 dB di S/N), soglia a metà con isteresi;
//! - rapporto segnale/rumore misurato sullo spettro attorno al tono (segnale e rumore separati, con due filtri), in dB su
//!   3 kHz: sotto -20 dB non si decodifica nemmeno con lo squelch spento (sul rumore niente caratteri falsi), lo squelch ha la sua soglia;
//! - gli stati più brevi di 0,4 unità (minimo 8 ms), toni e pause, si ignorano;
//! - velocità: medie separate di punti e linee, soglia a metà fra le due, aggiornate solo dalle coppie punto-linea con rapporto
//!   fra 2 e 4 (fldigi `cw::update_tracking`, idea di Lawrence Glaister VE7IT), solo con elementi sicuri dello stesso carattere e fra la
//!   metà e il doppio della velocità di partenza (fldigi: ±10 WPM);
//! - fine del carattere dopo 2 unità di pausa, spazio fra le parole dopo 4,5 unità (fldigi dopo 4);
//! - un carattere si stampa solo se i suoi toni e le sue pause sono ben distinti (contrasto 0,6 fra fondo e picco): col segnale troppo debole
//!   si stampa meno, non spazzatura;
//! - decodifica SOM facoltativa (fldigi `cw::find_winner`: il carattere più vicino alle durate normalizzate), utile col tasto a mano;
//! - AFC: lo spettro mediato attorno al tono (±75-100 Hz) cerca il picco del segnale e porta il tono su di esso; sul solo rumore il picco
//!   non si stacca dal fondo e il tono resta fermo. fldigi in CW non ha AFC (solo il clic sulla cascata): è nostro.
//!
//! Di base pero' il testo lo decide il decodificatore probabilistico di [`crate::cw_bayes`] (traliccio con velocità, pugno e modello del
//! testo, sulla banda base dopo il filtro); la parte a soglia resta per l'AGC, il cancello sullo spettro e la decodifica SOM. Davanti a
//! tutto c'e' il noise blanker ([`crate::blanker`]): gli istanti che cancella arrivano al decodificatore come «cancellati», con il peso
//! che resta nel filtro. Il decodificatore riceve anche il rumore di ogni istante, misurato sulle bande laterali e con il minimo
//! scorrevole ([`crate::cw_noise`]).
//!
//! Per un futuro riconoscimento probabilistico dei caratteri ogni elemento lascia anche la durata in unità e il livello
//! ([`CwDemodulator::last_elements`]), non solo punto o linea.

use crate::blanker::NoiseBlanker;
use crate::cw_bayes::{CwBayes, Observation};
use crate::cw_noise::{MinTracker, SidebandNoise};
use crate::psk::RATE;
use std::collections::VecDeque;
use std::f64::consts::PI;

/// Velocità minima e massima accettate (parole al minuto).
pub const WPM_MIN: f32 = 5.0;
pub const WPM_MAX: f32 = 60.0;

/// Le bande del filtro di ricezione (Hz) e quella di base (fldigi: 150). 1000 Hz e' la banda larga per la panoramica (ascolto e cascata):
/// lascia entrare anche le stazioni vicine, il decodificatore sente la piu' forte.
pub const BANDWIDTHS: [f32; 6] = [50.0, 100.0, 150.0, 250.0, 500.0, 1000.0];
pub const BANDWIDTH_DEFAULT: f32 = 150.0;

/// Soglia di base dello squelch sul rapporto segnale/rumore (dB in 3 kHz, media nel tempo: vedi `snr_db`).
pub const SQUELCH_DEFAULT_DB: f32 = -15.0;

/// Di quanto l'AFC puo' allontanarsi dal tono scelto (Hz).
pub const AFC_LIMIT_HZ: f32 = 100.0;

/// Il Morse internazionale: lettere, cifre e i segni di punteggiatura più usati.
pub(crate) const MORSE: &[(u8, &str)] = &[
    (b'A', ".-"),
    (b'B', "-..."),
    (b'C', "-.-."),
    (b'D', "-.."),
    (b'E', "."),
    (b'F', "..-."),
    (b'G', "--."),
    (b'H', "...."),
    (b'I', ".."),
    (b'J', ".---"),
    (b'K', "-.-"),
    (b'L', ".-.."),
    (b'M', "--"),
    (b'N', "-."),
    (b'O', "---"),
    (b'P', ".--."),
    (b'Q', "--.-"),
    (b'R', ".-."),
    (b'S', "..."),
    (b'T', "-"),
    (b'U', "..-"),
    (b'V', "...-"),
    (b'W', ".--"),
    (b'X', "-..-"),
    (b'Y', "-.--"),
    (b'Z', "--.."),
    (b'0', "-----"),
    (b'1', ".----"),
    (b'2', "..---"),
    (b'3', "...--"),
    (b'4', "....-"),
    (b'5', "....."),
    (b'6', "-...."),
    (b'7', "--..."),
    (b'8', "---.."),
    (b'9', "----."),
    (b'.', ".-.-.-"),
    (b',', "--..--"),
    (b'?', "..--.."),
    (b'\'', ".----."),
    (b'!', "-.-.--"),
    (b'/', "-..-."),
    (b'(', "-.--."),
    (b')', "-.--.-"),
    (b'&', ".-..."),
    (b':', "---..."),
    (b';', "-.-.-."),
    (b'=', "-...-"),
    (b'+', ".-.-."),
    (b'-', "-....-"),
    (b'_', "..--.-"),
    (b'"', ".-..-."),
    (b'$', "...-..-"),
    (b'@', ".--.-."),
];

/// I segni di procedura (fldigi `src/cw/morse.cxx`): in ricezione si stampano così al posto di `+ = (`, in trasmissione si scrivono così
/// e partono come un segno solo, senza pausa fra le lettere.
pub(crate) const PROSIGNS: &[(&str, &str)] = &[("<AR>", ".-.-."), ("<BT>", "-...-"), ("<KN>", "-.--."), ("<SK>", "...-.-")];

pub(crate) fn pattern_of(c: u8) -> Option<&'static str> {
    let up = c.to_ascii_uppercase();
    MORSE.iter().find(|(ch, _)| *ch == up).map(|(_, p)| *p)
}

fn char_of(pattern: &str) -> Option<u8> {
    MORSE.iter().find(|(_, p)| *p == pattern).map(|(c, _)| *c)
}

/// Il testo da stampare per una sequenza di punti e linee: prima i segni di procedura, poi i caratteri; `*` se non esiste (come fldigi).
fn text_of(pattern: &str) -> Vec<u8> {
    if let Some((name, _)) = PROSIGNS.iter().find(|(_, p)| *p == pattern) {
        return name.as_bytes().to_vec();
    }
    vec![char_of(pattern).unwrap_or(b'*')]
}

/// Durata di un'unità (un punto) in secondi alla velocità data.
pub fn unit_seconds(wpm: f32) -> f64 {
    1.2 / f64::from(wpm.clamp(WPM_MIN, WPM_MAX))
}

/// I segni da trasmettere: per ogni carattere i suoi punti e linee (`None` = pausa fra le parole). `<AR>`, `<BT>`, `<KN>`, `<SK>` sono un
/// segno solo.
fn signs_of(text: &[u8]) -> Vec<Option<&'static str>> {
    let mut out = Vec::new();
    let mut i = 0;
    while i < text.len() {
        let c = text[i];
        if c == b'<'
            && let Some((name, p)) = PROSIGNS.iter().find(|(name, _)| text[i..].len() >= name.len() && text[i..i + name.len()].eq_ignore_ascii_case(name.as_bytes()))
        {
            out.push(Some(*p));
            i += name.len();
            continue;
        }
        if c == b' ' || c == b'\n' || c == b'\r' || c == b'\t' {
            out.push(None);
        } else if let Some(p) = pattern_of(c) {
            out.push(Some(p));
        }
        i += 1;
    }
    out
}

/// Audio (a 12 kHz) del testo in Morse; `amp` è l'ampiezza di picco (0..1). I caratteri che il Morse non ha si saltano; lo spazio e il
/// ritorno a capo sono la pausa fra le parole. I fronti salgono e scendono in 5 ms per non fare «click».
pub fn modulate(text: &[u8], tone_hz: f32, wpm: f32, amp: f32) -> Vec<f32> {
    let unit = unit_seconds(wpm) * f64::from(RATE);
    // (acceso?, unita')
    let mut seq: Vec<(bool, f64)> = Vec::new();
    for sign in signs_of(text) {
        let Some(p) = sign else {
            seq.push((false, 4.0));
            continue;
        };
        for e in p.bytes() {
            seq.push((true, if e == b'-' { 3.0 } else { 1.0 }));
            seq.push((false, 1.0));
        }
        seq.push((false, 2.0));
    }
    let lead = (0.05 * f64::from(RATE)) as usize;
    let tail = (0.25 * f64::from(RATE)) as usize;
    let mut out = vec![0.0_f32; lead];
    let step = 2.0 * PI * f64::from(tone_hz) / f64::from(RATE);
    let ramp = (0.005 * f64::from(RATE)) as usize;
    let mut phase = 0.0_f64;
    let mut t_end = lead as f64;
    for (on, units) in seq {
        t_end += units * unit;
        let start = out.len();
        let stop = t_end.round() as usize;
        let len = stop.saturating_sub(start);
        for i in 0..len {
            if !on {
                out.push(0.0);
                continue;
            }
            let edge = i.min(len - 1 - i);
            let w = if edge < ramp { 0.5 * (1.0 - (PI * edge as f64 / ramp as f64).cos()) } else { 1.0 };
            out.push((f64::from(amp) * w * phase.sin()) as f32);
            phase += step;
        }
        phase %= 2.0 * PI;
    }
    out.extend(std::iter::repeat_n(0.0, tail));
    out
}

/// Lunghezza del filtro: 64 ms (fldigi: 512 coefficienti a 8 kHz), di piu' per le bande strette (5,5 / banda: con la finestra di Blackman
/// la transizione e' larga quanto la banda).
/// La strada larga (S/N e AFC) ha un filtro suo, corto: una semplice somma lascerebbe entrare, ripiegati, i segnali forti lontani.
const WIDE_LEN: usize = 241;

fn fir_len(bw: f32) -> usize {
    (f64::from(RATE) * (5.5 / f64::from(bw)).max(0.064)).round() as usize | 1
}
/// AGC: quantili del livello per il fondo e il picco, e passo in dB per campione a 500 Hz (il picco sale di circa 25 dB/s e scende di
/// circa 3 dB/s).
const Q_NOISE: f32 = 0.2;
const Q_PEAK: f32 = 0.9;
const AGC_STEP_DB: f32 = 0.06;
/// Il fondo si muove piu' in fretta del picco (sale di circa 15 dB/s): il rumore della banda cambia, e all'avvio va trovato subito.
const NOISE_SPEED: f32 = 2.5;
/// Rapporto segnale/rumore (in 3 kHz) sotto il quale c'e' solo rumore: sul rumore la misura sta attorno a -25 dB e una volta su mille
/// arriva a -18 (la banda piu' larga, piu' righe dove cercare); un CW a -12 dB legge circa -15. Il cancello si apre a -16 e si chiude
/// sotto -20.
const MIN_SNR_DB: f32 = -16.0;
const GATE_HYST_DB: f32 = 4.0;
/// Un livello cosi' sopra il picco e' un segnale nuovo, non rumore.
const JUMP_DB: f32 = 10.0;
/// Banda minima del filtro del decodificatore probabilistico (Hz).
const BAYES_MIN_BW: f32 = 150.0;
/// Soglia di decisione fra fondo e picco, con isteresi (provate nella prova `cw_confronto`, `studio`: 0,5/0,3 meglio di 0,5/0,4 e 0,6/0,4).
const ON_AT: f32 = 0.5;
const OFF_AT: f32 = 0.3;
/// Stati più brevi di cosi' (in unità, e mai sotto 8 ms) non contano (0,4 unità sono 24 ms a 20 WPM).
const GLITCH_UNITS: f64 = 0.4;
/// Media mobile dopo il rivelatore, in unità: fldigi usa 1/3 di punto; piu' lunga rende meglio col rumore ma, se la velocità raddoppia di
/// colpo, cancella le pause fra gli elementi prima che la stima si aggiorni (prova `studio`: 1/3 → 41, 0,4 → 41 con il salto a 18, 1/2 →
/// 37 ma il salto a 32). 0,4 e' il compromesso.
const AVG_UNITS: f64 = 0.4;
/// Contrasto minimo (livello medio dei toni meno quello delle pause, fra fondo e picco) perche' un carattere si stampi: sotto e' rumore.
/// Nella prova `cw_confronto` 0,6 toglie la meta' dei caratteri inventati a -10/-15 dB senza perdere quelli veri (0,7 e 0,8 si'): la
/// media delle righe a -12 dB scende da 136 a 87.
const CHAR_CONTRAST: f32 = 0.6;
/// Ritardo della decisione rispetto allo spettro (s).
const GATE_DELAY_S: f64 = 0.4;
/// Livello medio (fra fondo e picco) perche' un elemento serva a stimare la velocità.
const SURE_LEVEL: f32 = 0.75;
const GLITCH_MIN_S: f64 = 0.008;
/// Pausa che chiude il carattere e pausa che fa lo spazio (in unità).
const CHAR_GAP: f64 = 2.0;
const WORD_GAP: f64 = 4.5;
/// Elementi al massimo in un carattere (`$` ne ha 7): oltre e' rumore.
const MAX_ELEMENTS: usize = 7;
/// Peso delle medie di punti e linee per ogni coppia valida.
const SPEED_WEIGHT: f64 = 0.25;

fn blackman(x: f64) -> f64 {
    0.42 - 0.5 * (2.0 * PI * x).cos() + 0.08 * (4.0 * PI * x).cos()
}

/// Passa-basso a finestra (come `bp_FIR` di fldigi, ma con la finestra di Blackman al posto di quella di Hamming: i lobi lontani scendono
/// sotto i -74 dB e un segnale forte a centinaia di Hz non entra), taglio `fc` in giri per campione, guadagno 1 in continua.
fn lowpass(len: usize, fc: f64) -> Vec<f32> {
    let mid = (len - 1) as f64 / 2.0;
    let mut taps: Vec<f64> = (0..len)
        .map(|i| {
            let t = i as f64 - mid;
            let sinc = if t.abs() < 1e-9 { 2.0 * fc } else { (2.0 * PI * fc * t).sin() / (PI * t) };
            sinc * blackman(i as f64 / (len - 1) as f64)
        })
        .collect();
    let sum: f64 = taps.iter().sum();
    for t in taps.iter_mut() {
        *t /= sum;
    }
    taps.into_iter().map(|t| t as f32).collect()
}

/// Un campione della banda base in attesa della decisione: uscita del filtro di ricezione e di quello del decodificatore, tono del mixer
/// in quel momento, rumore dalle bande laterali e dal minimo scorrevole, peso dopo il noise blanker (guadagno e varianza).
#[derive(Clone, Copy)]
struct Delayed {
    z: (f32, f32),
    b: (f32, f32),
    mixed_at: f32,
    side: f64,
    floor: f64,
    gain: f32,
    var: f32,
}

/// Un elemento ricevuto: tono o pausa, durata in unità della velocità di quel momento e livello medio (0-1 fra fondo e picco).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Element {
    pub key_down: bool,
    pub units: f32,
    pub level: f32,
}

/// Il ricevitore. La velocità si adatta da sola ai punti e alle linee che sente (partendo da `wpm_hint`).
pub struct CwDemodulator {
    base_hz: f32,
    tone_hz: f32,
    bandwidth: f32,
    afc: bool,
    squelch_snr_db: Option<f32>,
    som: bool,
    /// Campionamento dopo la decimazione (500 Hz, 1000 con la banda di 500 Hz) e fattore di decimazione.
    fs: f64,
    decim: usize,
    step: f64,
    phase: f64,
    fir: Vec<f32>,
    ring: Vec<(f32, f32)>,
    pos: usize,
    count: usize,
    /// La banda base in attesa della decisione (vedi `GATE_DELAY_S`).
    /// Per ogni campione: l'uscita del filtro di ricezione e quella del filtro del decodificatore probabilistico.
    /// (e il tono del mixer in quel momento, che con l'AFC cambia)
    delay: VecDeque<Delayed>,
    /// Il filtro del decodificatore probabilistico: la banda scelta, ma mai sotto `BAYES_MIN_BW` (un filtro piu' stretto allunga i toni
    /// sulle pause; la selettivita' la danno gia' le somme coerenti sui toni e il cancello sullo spettro della banda scelta). Vuoto se e'
    /// lo stesso filtro di ricezione.
    bayes_fir: Vec<f32>,
    /// Il noise blanker davanti a tutto (vedi `blanker`), acceso di base, e per ogni campione dell'anello se e' stato cancellato (0) o no
    /// (1); da quanti campioni non ne cancella.
    blanker: Option<NoiseBlanker>,
    kept: Vec<f32>,
    since_blank: usize,
    /// Energia dei coefficienti del filtro del decodificatore (per la varianza dei campioni toccati dal blanker).
    bayes_energy: f32,
    /// Il rumore istante per istante: bande laterali (sulla strada larga) e minimo scorrevole (sulla banda base del decodificatore).
    side: Option<SidebandNoise>,
    min_noise: Option<MinTracker>,
    beam_override: Option<usize>,
    /// Il decodificatore probabilistico (vedi `cw_bayes`); spento si usa quello a soglia (sempre con la decodifica SOM).
    bayes: Option<CwBayes>,
    use_bayes: bool,
    /// Lo scarto fine del tono dal centro del filtro (Hz), dallo spettro: lo usa il decodificatore probabilistico.
    fine_hz: f32,
    /// L'ultima frequenza assoluta del picco trovata dallo spettro (per lo scarto fine serve che resti ferma).
    fine_raw: Option<f32>,
    /// I caratteri del ricevitore a soglia quando decide l'altro.
    classic_out: Vec<u8>,
    /// Le probabilità dei caratteri dell'ultima chiamata di `process` (255 = certo).
    confidences: Vec<u8>,
    /// Filtro della strada larga (S/N e AFC): ±125 Hz, 20 ms, sugli stessi campioni del mixer.
    wide_fir: Vec<f32>,
    // media di 1/3 di punto
    avg_buf: VecDeque<f32>,
    avg_sum: f32,
    // AGC: fondo e picco in dB
    agc_ready: bool,
    /// Il rapporto segnale/rumore dice che c'e' un segnale (con isteresi).
    gate: bool,
    warmup: usize,
    noise: f32,
    peak: f32,
    snr_db: f32,
    level_db: f32,
    open: bool,
    // stati
    raw_down: bool,
    /// Da quanti campioni il rivelatore dice il contrario dello stato accettato.
    pending: usize,
    pending_level: f32,
    key_down: bool,
    run: usize,
    run_level: f32,
    // tempi (campioni decimati)
    dot: f64,
    dash: f64,
    last_mark: f64,
    last_level: f32,
    hint_wpm: f32,
    pattern: String,
    durations: Vec<f64>,
    elements: Vec<Element>,
    last_elements: Vec<Element>,
    flushed: bool,
    word_sent: bool,
    spoke: bool,
    // AFC
    afc_hist: VecDeque<(f32, f32)>,
    afc_hop: usize,
    afc_power: Vec<f32>,
}

impl CwDemodulator {
    pub fn new(tone_hz: f32, wpm_hint: f32) -> Self {
        let mut d = Self {
            hint_wpm: wpm_hint,
            base_hz: tone_hz,
            tone_hz,
            bandwidth: BANDWIDTH_DEFAULT,
            afc: false,
            squelch_snr_db: None,
            som: false,
            fs: 500.0,
            decim: 24,
            step: 2.0 * PI * f64::from(tone_hz) / f64::from(RATE),
            phase: 0.0,
            fir: Vec::new(),
            ring: Vec::new(),
            pos: 0,
            count: 0,
            wide_fir: lowpass(WIDE_LEN, 125.0 / f64::from(RATE)),
            delay: VecDeque::new(),
            bayes_fir: Vec::new(),
            blanker: Some(NoiseBlanker::new()),
            kept: Vec::new(),
            since_blank: usize::MAX,
            bayes_energy: 1.0,
            side: None,
            min_noise: None,
            beam_override: None,
            bayes: None,
            use_bayes: true,
            fine_hz: 0.0,
            fine_raw: None,
            classic_out: Vec::new(),
            confidences: Vec::new(),
            avg_buf: VecDeque::new(),
            avg_sum: 0.0,
            agc_ready: false,
            gate: false,
            warmup: 0,
            noise: 0.0,
            peak: 0.0,
            snr_db: -40.0,
            level_db: -90.0,
            open: false,
            raw_down: false,
            pending: 0,
            pending_level: 0.0,
            key_down: false,
            run: 0,
            run_level: 0.0,
            dot: 0.0,
            dash: 0.0,
            last_mark: 0.0,
            last_level: 0.0,
            pattern: String::new(),
            durations: Vec::new(),
            elements: Vec::new(),
            last_elements: Vec::new(),
            flushed: true,
            word_sent: true,
            spoke: false,
            afc_hist: VecDeque::new(),
            afc_hop: 0,
            afc_power: Vec::new(),
        };
        d.set_bandwidth(BANDWIDTH_DEFAULT);
        d.set_wpm(wpm_hint);
        d
    }

    /// La velocità di partenza (o una nuova, scelta a mano): azzera le medie di punti e linee.
    pub fn set_wpm(&mut self, wpm: f32) {
        self.hint_wpm = wpm.clamp(WPM_MIN, WPM_MAX);
        self.dot = unit_seconds(wpm) * self.fs;
        self.dash = 3.0 * self.dot;
        if let Some(b) = self.bayes.as_mut() {
            b.set_wpm_hint(self.hint_wpm);
        }
    }

    /// La banda del filtro di ricezione (Hz), fra 50 e 1000.
    pub fn set_bandwidth(&mut self, hz: f32) {
        let bw = hz.clamp(BANDWIDTHS[0], BANDWIDTHS[BANDWIDTHS.len() - 1]);
        let unit_s = self.dot / self.fs;
        self.bandwidth = bw;
        // con la banda di 500 Hz serve un campionamento interno di 1000 Hz, con quella di 1000 Hz uno di 2000 Hz
        self.decim = if bw > 500.0 {
            6
        } else if bw > 250.0 {
            12
        } else {
            24
        };
        self.fs = f64::from(RATE) / self.decim as f64;
        let len = fir_len(bw);
        self.fir = lowpass(len, f64::from(bw) / 2.0 / f64::from(RATE));
        self.ring = vec![(0.0, 0.0); len];
        self.kept = vec![1.0; len];
        self.since_blank = usize::MAX;
        self.pos = 0;
        if unit_s > 0.0 {
            self.dot = unit_s * self.fs;
            self.dash = 3.0 * self.dot;
        }
        self.afc_power.clear();
        self.afc_hist.clear();
        self.delay.clear();
        self.bayes_fir = if bw < BAYES_MIN_BW { lowpass(fir_len(BAYES_MIN_BW), f64::from(BAYES_MIN_BW) / 2.0 / f64::from(RATE)) } else { Vec::new() };
        let taps = if self.bayes_fir.is_empty() { &self.fir } else { &self.bayes_fir };
        self.bayes_energy = taps.iter().map(|h| h * h).sum::<f32>().max(1e-30);
        self.side = Some(SidebandNoise::new(self.fs, self.decim, &self.wide_fir, taps, f64::from(self.hint_wpm)));
        // gradi di liberta' (reali) di una media di 40 ms del rumore dopo il filtro: due per ogni campione complesso indipendente
        let enbw = f64::from(RATE) * f64::from(self.bayes_energy) / f64::from(taps.iter().sum::<f32>()).powi(2).max(1e-30);
        self.min_noise = Some(MinTracker::new(self.fs, 2.0 * 0.04 * enbw));
        let mut b = CwBayes::new(self.fs, taps, self.decim, self.hint_wpm);
        if let Some(n) = self.beam_override {
            b.set_beam(n);
        }
        self.bayes = Some(b);
    }

    pub fn bandwidth(&self) -> f32 {
        self.bandwidth
    }

    /// Il tono su cui si ascolta (con l'AFC puo' essersi spostato da quello scelto).
    pub fn frequency(&self) -> f32 {
        self.tone_hz
    }

    /// Il tono scelto dall'utente: l'AFC riparte da qui.
    pub fn set_frequency(&mut self, hz: f32) {
        self.base_hz = hz;
        self.tune(hz);
    }

    fn tune(&mut self, hz: f32) {
        self.tone_hz = hz;
        self.step = 2.0 * PI * f64::from(hz) / f64::from(RATE);
    }

    /// AFC: il tono segue il segnale entro `AFC_LIMIT_HZ` da quello scelto; spento si torna al tono scelto.
    pub fn set_afc(&mut self, on: bool) {
        self.afc = on;
        if !on {
            self.tune(self.base_hz);
        }
        self.afc_power.clear();
    }

    /// Soglia dello squelch sul rapporto segnale/rumore (dB); `None` lo spegne.
    pub fn set_squelch_snr_db(&mut self, db: Option<f32>) {
        self.squelch_snr_db = db;
    }

    /// Decodifica SOM di fldigi al posto della tabella (per il tasto a mano irregolare): lavora sul ricevitore a soglia.
    pub fn set_som(&mut self, on: bool) {
        self.som = on;
    }

    /// Il decodificatore probabilistico (di base) o quello a soglia. Con la decodifica SOM si usa sempre quello a soglia.
    pub fn set_probabilistic(&mut self, on: bool) {
        self.use_bayes = on;
    }

    /// Ipotesi del decodificatore probabilistico (meno = meno CPU, per i canali della decodifica di tutta la banda). Vale fino al prossimo
    /// cambio di banda o di velocità di partenza.
    pub fn set_beam(&mut self, n: usize) {
        self.beam_override = Some(n);
        if let Some(b) = self.bayes.as_mut() {
            b.set_beam(n);
        }
    }

    /// Peso del modello del testo nel decodificatore probabilistico (0 = nessuno).
    pub fn set_text_model_weight(&mut self, w: f32) {
        if let Some(b) = self.bayes.as_mut() {
            b.set_lm_weight(w);
        }
    }

    /// Il noise blanker contro le scariche (acceso di base).
    pub fn set_noise_blanker(&mut self, on: bool) {
        if on != self.blanker.is_some() {
            self.blanker = on.then(NoiseBlanker::new);
        }
    }

    fn bayes_active(&self) -> bool {
        self.use_bayes && !self.som && self.bayes.is_some()
    }

    /// Le probabilità (0-255) dei caratteri restituiti dall'ultima chiamata di [`CwDemodulator::process`], uno per carattere o spazio; vuoto
    /// con il ricevitore a soglia.
    pub fn confidences(&self) -> &[u8] {
        &self.confidences
    }

    /// Livello (dB) del tono e squelch aperto.
    pub fn signal(&self) -> (f32, bool) {
        (self.level_db, self.squelch_snr_db.is_none() || self.open)
    }

    /// Rapporto segnale/rumore (dB in 3 kHz, media nel tempo del tono contro il rumore attorno): e' quello su cui lavora lo squelch.
    pub fn snr_db(&self) -> f32 {
        self.snr_db
    }

    /// La velocità che il ricevitore ha capito, in parole al minuto.
    pub fn wpm(&self) -> f32 {
        match &self.bayes {
            Some(b) if self.bayes_active() => b.wpm(),
            _ => (1.2 / (self.unit() / self.fs)) as f32,
        }
    }

    /// Gli elementi (toni e pause) dell'ultimo carattere stampato, con durata in unità e livello.
    pub fn last_elements(&self) -> &[Element] {
        &self.last_elements
    }

    /// Durata dell'unità in campioni decimati: la media fra il punto e un terzo della linea.
    fn unit(&self) -> f64 {
        let (lo, hi) = self.unit_limits();
        ((self.dot + self.dash / 3.0) / 2.0).clamp(lo, hi)
    }

    /// L'unità piu' corta e piu' lunga ammesse: fra la meta' e il doppio della velocità di partenza, dentro 5-60 WPM.
    fn unit_limits(&self) -> (f64, f64) {
        let fast = (2.0 * self.hint_wpm).min(WPM_MAX);
        let slow = (0.5 * self.hint_wpm).max(WPM_MIN);
        (unit_seconds(fast) * self.fs, unit_seconds(slow) * self.fs)
    }

    fn clamp_speed(&mut self) {
        let (lo, hi) = self.unit_limits();
        self.dot = self.dot.clamp(lo, hi);
        self.dash = self.dash.clamp(2.0 * lo, 4.0 * hi);
        // le due medie non si incrociano: la linea resta almeno il doppio del punto
        if self.dash < 2.0 * self.dot {
            let u = self.unit();
            self.dot = u;
            self.dash = 3.0 * u;
        }
    }

    /// Passa l'audio (a 12 kHz); restituisce i caratteri decodificati.
    pub fn process(&mut self, audio: &[f32]) -> Vec<u8> {
        let mut out = Vec::new();
        self.confidences.clear();
        let bayes = self.bayes_active();
        let len = self.ring.len();
        for &s in audio {
            let (s, blanked) = match self.blanker.as_mut() {
                Some(nb) => nb.process_marked(s),
                None => (s, false),
            };
            self.kept[self.pos] = if blanked { 0.0 } else { 1.0 };
            self.since_blank = if blanked { 0 } else { self.since_blank.saturating_add(1) };
            let (sin, cos) = self.phase.sin_cos();
            let z = (s * cos as f32, -s * sin as f32);
            self.phase += self.step;
            if self.phase > 2.0 * PI {
                self.phase -= 2.0 * PI;
            }
            self.ring[self.pos] = z;
            self.pos = (self.pos + 1) % len;
            self.count += 1;
            if self.count < self.decim {
                continue;
            }
            self.count = 0;
            let (mut zi, mut zq) = (0.0_f32, 0.0_f32);
            for (k, &c) in self.fir.iter().enumerate() {
                let (i, q) = self.ring[(self.pos + k) % len];
                zi += c * i;
                zq += c * q;
            }
            let (mut wi, mut wq) = (0.0_f32, 0.0_f32);
            let first = self.pos + len - self.wide_fir.len();
            for (k, &c) in self.wide_fir.iter().enumerate() {
                let (i, q) = self.ring[(first + k) % len];
                wi += c * i;
                wq += c * q;
            }
            self.spectrum_step((wi, wq));
            // la decisione lavora in ritardo di GATE_DELAY_S: cosi' il rapporto segnale/rumore dello spettro (che ha bisogno di un quarto di
            // secondo per accorgersi di un segnale nuovo) vale gia' per il primo elemento, e il primo carattere non si perde
            let (mut bi, mut bq) = (zi, zq);
            if bayes && !self.bayes_fir.is_empty() {
                (bi, bq) = (0.0, 0.0);
                let first = self.pos + len - self.bayes_fir.len();
                for (k, &c) in self.bayes_fir.iter().enumerate() {
                    let (i, q) = self.ring[(first + k) % len];
                    bi += c * i;
                    bq += c * q;
                }
            }
            let (mut gain, mut var) = (1.0_f32, 1.0_f32);
            if bayes {
                if let Some(b) = self.bayes.as_mut() {
                    b.observe_level((bi, bq));
                }
                // dopo il blanker il filtro del decodificatore ha in parte campioni azzerati: quanto segnale e quanto rumore restano
                let taps = if self.bayes_fir.is_empty() { &self.fir } else { &self.bayes_fir };
                if self.since_blank < taps.len() {
                    let first = self.pos + len - taps.len();
                    (gain, var) = (0.0, 0.0);
                    for (k, &c) in taps.iter().enumerate() {
                        let m = self.kept[(first + k) % len];
                        gain += c * m;
                        var += c * c * m;
                    }
                    var /= self.bayes_energy;
                }
            }
            let side = if bayes { self.side.as_mut().map_or(0.0, |sd| sd.push((wi, wq))) } else { 0.0 };
            let floor = if bayes { self.min_noise.as_mut().map_or(0.0, |m| m.push(f64::from(bi * bi + bq * bq))) } else { 0.0 };
            self.delay.push_back(Delayed { z: (zi, zq), b: (bi, bq), mixed_at: self.tone_hz, side: 0.0, floor, gain, var });
            // le bande laterali arrivano in ritardo sulla banda base del decodificatore: la misura va al campione che descrive
            let lag = self.side.as_ref().map_or(0, SidebandNoise::lag);
            if let Some(d) = self.delay.iter_mut().rev().nth(lag) {
                d.side = side;
            }
            if self.delay.len() as f64 > GATE_DELAY_S * self.fs {
                let Some(Delayed { z: (zi, zq), b: z, mixed_at, side, floor, gain, var }) = self.delay.pop_front() else { continue };
                let env = (zi * zi + zq * zq).sqrt() * 2.0;
                if bayes {
                    let mut classic = std::mem::take(&mut self.classic_out);
                    self.sample(env, &mut classic);
                    classic.clear();
                    self.classic_out = classic;
                    let gate = self.open && self.gate;
                    if let Some(b) = self.bayes.as_mut() {
                        // il segnale e' a tono + scarto fine; questo campione e' stato portato in banda base con il tono di allora
                        b.set_offset_hz(f64::from(self.tone_hz + self.fine_hz - mixed_at));
                        let obs = Observation { noise: noise_of(side, floor), gain: f64::from(gain), var: f64::from(var) };
                        b.push_observed(z, obs, gate, &mut out);
                    }
                } else {
                    self.sample(env, &mut out);
                }
            }
        }
        if bayes && let Some(b) = self.bayes.as_mut() {
            self.confidences = b.take_confidences();
            // la larghezza dei filtri delle bande laterali segue la velocità misurata (non quella di partenza del traliccio, che all'avvio
            // salta)
            if let (Some(wpm), Some(sd)) = (b.measured_wpm(), self.side.as_mut()) {
                sd.set_wpm(f64::from(wpm));
            }
        }
        out
    }

    /// Un campione decimato dell'ampiezza: media, AGC, decisione, durate.
    fn sample(&mut self, env: f32, out: &mut Vec<u8>) {
        // media mobile lunga 1/3 di punto alla velocità ricevuta (fldigi la lega a quella di trasmissione)
        let n = ((self.unit() * AVG_UNITS).round() as usize).max(1);
        self.avg_buf.push_back(env);
        self.avg_sum += env;
        while self.avg_buf.len() > n {
            self.avg_sum -= self.avg_buf.pop_front().unwrap_or(0.0);
        }
        let v = self.avg_sum / self.avg_buf.len() as f32;
        self.level_db = 20.0 * v.max(1e-5).log10();

        // AGC: fondo (tasto alzato) e picco (tasto giu') come quantili del livello in dB, il 20% e il 90%. Salgono in fretta e scendono
        // piano come le medie di fldigi (salita circa 0,25 s, discesa qualche secondo), ma non si lasciano ingannare dal rumore: le medie di
        // fldigi (`noise_floor` sotto `sig_avg`, `agc_peak` sopra) sul solo rumore danno circa 17 dB di S/N e lo squelch non lo distingue.
        let l = 20.0 * v.max(1e-7).log10();
        let eta = AGC_STEP_DB * (500.0 / self.fs as f32);
        if !self.agc_ready {
            // si parte quando il filtro e' pieno (64 ms): prima l'uscita e' solo il suo transitorio
            self.warmup += 1;
            if self.warmup * self.decim < self.fir.len() {
                return;
            }
            (self.noise, self.peak, self.agc_ready) = (l, l, true);
        }
        self.noise += NOISE_SPEED * eta * (Q_NOISE - if l < self.noise { 1.0 } else { 0.0 });
        self.peak += eta * (Q_PEAK - if l < self.peak { 1.0 } else { 0.0 });
        // un salto di livello che il rumore da solo non fa (oltre 10 dB sopra il picco): il picco lo raggiunge in una ventina di ms
        if l > self.peak + JUMP_DB {
            self.peak += 0.1 * (l - self.peak) * (500.0 / self.fs as f32);
        }
        self.peak = self.peak.max(self.noise);
        // il fondo non scende sotto -100 dBFS: con il silenzio digitale anche i resti del filtro sembrerebbero un segnale
        let floor = 10f32.powf(self.noise.max(-100.0) / 20.0);
        let top = 10f32.powf(self.peak / 20.0);
        // squelch e soglia minima sul rapporto segnale/rumore dello spettro (spectrum_step), non sul livello
        self.open = self.squelch_snr_db.is_none_or(|t| self.snr_db >= t);
        let span = top - floor;
        let norm = if span > 0.0 { ((v - floor) / span).clamp(0.0, 1.5) } else { 0.0 };
        // sotto MIN_SNR_DB c'e' solo rumore: anche con lo squelch spento non si inventano punti e linee
        self.gate = if self.gate { self.snr_db >= MIN_SNR_DB - GATE_HYST_DB } else { self.snr_db >= MIN_SNR_DB };
        let raw = self.open && self.gate && if self.raw_down { norm > OFF_AT } else { norm > ON_AT };
        self.raw_down = raw;

        // filtro degli stati brevi: un cambio vale solo se dura almeno GLITCH; la sua durata parte da quando e' cominciato
        let glitch = (GLITCH_UNITS * self.unit()).max(GLITCH_MIN_S * self.fs).round() as usize;
        self.run += 1;
        self.run_level += norm;
        if raw != self.key_down {
            self.pending += 1;
            self.pending_level += norm;
            if self.pending >= glitch.max(1) {
                let len = self.run - self.pending;
                let level = (self.run_level - self.pending_level) / len.max(1) as f32;
                self.state_ended(self.key_down, len as f64, level);
                self.key_down = raw;
                self.run = self.pending;
                self.run_level = self.pending_level;
                self.pending = 0;
                self.pending_level = 0.0;
            }
        } else {
            self.pending = 0;
            self.pending_level = 0.0;
        }
        if !self.key_down {
            let gap = (self.run - self.pending) as f64;
            let u = self.unit();
            if !self.flushed && gap > CHAR_GAP * u {
                self.emit(out);
                self.flushed = true;
            }
            if !self.word_sent && gap > WORD_GAP * u {
                if self.spoke {
                    out.push(b' ');
                    self.spoke = false;
                }
                self.word_sent = true;
            }
        }
    }

    /// Uno stato (tono o pausa) e' finito.
    fn state_ended(&mut self, was_down: bool, len: f64, level: f32) {
        let unit = self.unit();
        self.elements.push(Element { key_down: was_down, units: (len / unit) as f32, level });
        if self.elements.len() > 2 * MAX_ELEMENTS + 2 {
            self.elements.remove(0);
        }
        if !was_down {
            return;
        }
        // coppie punto-linea con rapporto fra 2 e 4 (fldigi): aggiornano le medie separate di punto e linea
        // solo elementi sicuri (livello medio vicino al picco) e dello stesso carattere: il rumore fa coppie a caso e porterebbe via la velocita'
        let last = self.last_mark;
        let sure = level.min(self.last_level) >= SURE_LEVEL && !self.pattern.is_empty();
        if last > 0.0 && sure {
            let pair = if len > 2.0 * last && len < 4.0 * last {
                Some((last, len))
            } else if last > 2.0 * len && last < 4.0 * len {
                Some((len, last))
            } else {
                None
            };
            if let Some((d, l)) = pair {
                self.dot += SPEED_WEIGHT * (d - self.dot);
                self.dash += SPEED_WEIGHT * (l - self.dash);
                self.clamp_speed();
            }
        }
        self.last_mark = len;
        self.last_level = level;
        // soglia a meta' fra punto e linea
        let threshold = (self.dot + self.dash) / 2.0;
        self.pattern.push(if len > threshold { '-' } else { '.' });
        self.durations.push(len);
        if self.pattern.len() > MAX_ELEMENTS {
            // nessun carattere ha piu' di 7 elementi: era rumore
            self.pattern.clear();
            self.durations.clear();
        }
        self.flushed = false;
        self.word_sent = false;
    }

    fn emit(&mut self, out: &mut Vec<u8>) {
        if self.pattern.is_empty() {
            return;
        }
        let threshold = (self.dot + self.dash) / 2.0;
        let pattern = if self.som { som_winner(&self.durations, threshold).unwrap_or(&self.pattern) } else { &self.pattern };
        // contrasto fra i toni e le pause del carattere: un carattere nato dal rumore ha toni appena sopra la soglia e pause appena sotto
        let mean = |down: bool| {
            let v: Vec<f32> = self.elements.iter().filter(|e| e.key_down == down).map(|e| e.level).collect();
            if v.is_empty() { 0.0 } else { v.iter().sum::<f32>() / v.len() as f32 }
        };
        let contrast = mean(true) - mean(false);
        if contrast >= CHAR_CONTRAST {
            out.extend(text_of(pattern));
        }
        self.pattern.clear();
        self.durations.clear();
        self.last_elements = std::mem::take(&mut self.elements);
        self.spoke = true;
    }

    /// Spettro (DFT con finestra di Hann su 256 ms, ogni 128 ms) della strada larga attorno al tono, mediato (sale in fretta, scende piano).
    /// Da qui vengono il rapporto segnale/rumore e l'AFC.
    ///
    /// S/N: la potenza nelle righe a ±12 Hz attorno al segnale piu' forte dentro la banda del filtro, contro la mediana delle righe (il rumore, anche con un'altra stazione vicina), riportata
    /// a 3 kHz come nelle prove e in WSJT-X; e' una media nel tempo, quindi un CW con il tasto giu' meta' del tempo legge circa 3 dB meno del
    /// suo S/N a tasto giu'. L'idea e' quella di due filtri di larghezza diversa (segnale e rumore misurati a parte).
    ///
    /// AFC: se il picco si stacca dal fondo, il tono si sposta verso di lui; il picco piu' vicino al tono pesa di piu' (un'altra stazione
    /// lontana non lo porta via). Sul solo rumore il picco non si stacca e il tono resta fermo.
    fn spectrum_step(&mut self, z: (f32, f32)) {
        let n = (0.256 * self.fs).round() as usize;
        self.afc_hist.push_back(z);
        while self.afc_hist.len() > n {
            self.afc_hist.pop_front();
        }
        self.afc_hop += 1;
        if self.afc_hop < n / 2 || self.afc_hist.len() < n {
            return;
        }
        self.afc_hop = 0;
        let res = self.fs / n as f64;
        let range = f64::from((self.bandwidth / 2.0 + 50.0).clamp(75.0, AFC_LIMIT_HZ));
        let kmax = (range / res).ceil() as i64;
        let bins = (2 * kmax + 1) as usize;
        if self.afc_power.len() != bins {
            self.afc_power = vec![0.0; bins];
        }
        for (b, kk) in (-kmax..=kmax).enumerate() {
            let (mut re, mut im) = (0.0_f64, 0.0_f64);
            for (i, &(x, y)) in self.afc_hist.iter().enumerate() {
                let w = 0.5 - 0.5 * (2.0 * PI * i as f64 / n as f64).cos();
                let (s, c) = (-2.0 * PI * kk as f64 * i as f64 / n as f64).sin_cos();
                re += w * (f64::from(x) * c - f64::from(y) * s);
                im += w * (f64::from(x) * s + f64::from(y) * c);
            }
            let p = (re * re + im * im) as f32;
            let a = if p > self.afc_power[b] { 0.5 } else { 0.15 };
            self.afc_power[b] += a * (p - self.afc_power[b]);
        }
        let mut sorted = self.afc_power.clone();
        sorted.sort_by(f32::total_cmp);
        let median = sorted[sorted.len() / 2];
        if median <= 0.0 {
            return;
        }
        let near = ((12.0 / res).round() as i64).max(1);
        // il segnale piu' forte dentro la banda del filtro (righe a ±12 Hz attorno al suo centro)
        let window = |c: i64| -> f32 {
            let lo = (c - near).max(0) as usize;
            let hi = ((c + near) as usize).min(bins - 1);
            self.afc_power[lo..=hi].iter().map(|p| p - median).sum()
        };
        let pass = ((f64::from(self.bandwidth) / 2.0 - 12.0).max(0.0) / res).round() as i64;
        let tone = ((kmax - pass).max(0)..=(kmax + pass).min(2 * kmax)).map(window).fold(f32::MIN, f32::max);
        self.snr_db = (10.0 * (tone.max(1e-6 * median) / median).log10() - 10.0 * (3000.0 / res).log10() as f32).max(-40.0);
        // senza AFC il picco si cerca solo dentro la banda del filtro (per lo scarto fine del decodificatore probabilistico)
        let reach = if self.afc { kmax } else { pass.min(kmax) };
        let best = ((kmax - reach).max(0) as usize..=((kmax + reach) as usize).min(bins - 1))
            .max_by(|&a, &b| {
                let w = |i: usize| {
                    let d = (i as f64 - kmax as f64) * res / range;
                    f64::from(self.afc_power[i]) / (1.0 + d * d)
                };
                w(a).total_cmp(&w(b))
            })
            .unwrap_or(kmax as usize);
        // il picco deve passare la stessa soglia del decodificatore (MIN_SNR_DB, righe a ±12 Hz contro la mediana) e stare 9 dB sopra il fondo
        let (plo, phi) = (best.saturating_sub(near as usize), (best + near as usize).min(bins - 1));
        let peak: f32 = self.afc_power[plo..=phi].iter().map(|p| p - median).sum();
        let peak_snr = 10.0 * (peak.max(1e-6 * median) / median).log10() - 10.0 * (3000.0 / res).log10() as f32;
        if self.afc_power[best] <= 8.0 * median || peak_snr < MIN_SNR_DB {
            return;
        }
        // interpolazione parabolica del picco
        let mut off = (best as f64 - kmax as f64) * res;
        if best > 0 && best + 1 < bins {
            let (a, b, c) = (f64::from(self.afc_power[best - 1]), f64::from(self.afc_power[best]), f64::from(self.afc_power[best + 1]));
            let den = a - 2.0 * b + c;
            if den.abs() > 1e-20 {
                off += 0.5 * (a - c) / den * res;
            }
        }
        // lo scarto fine vale solo se due spettri di fila trovano il picco nello stesso posto (all'inizio di un segnale lo spettro e'
        // largo e il picco salta)
        let abs_hz = self.tone_hz + off as f32;
        let steady = self.fine_raw.is_some_and(|prev| (prev - abs_hz).abs() < 2.0 * res as f32);
        self.fine_raw = Some(abs_hz);
        if steady {
            self.fine_hz = off as f32;
        }
        // anche l'AFC si muove solo su un picco fermo: all'inizio di un segnale il primo spettro lo vede spostato di decine di Hz
        if !self.afc || !steady {
            return;
        }
        if off.abs() < 0.5 {
            return;
        }
        let moved = (0.5 * off) as f32;
        let f = (self.tone_hz + moved).clamp(self.base_hz - AFC_LIMIT_HZ, self.base_hz + AFC_LIMIT_HZ);
        // il tono si sposta: lo scarto fine resta riferito a lui
        self.fine_hz -= f - self.tone_hz;
        let shift = ((f - self.tone_hz) as f64 / res).round() as i64;
        self.tune(f);
        // lo spettro mediato si sposta con il tono
        if shift != 0 {
            let old = std::mem::take(&mut self.afc_power);
            self.afc_power = (0..bins as i64).map(|i| old.get((i + shift) as usize).copied().unwrap_or(median)).collect();
        }
    }
}

/// Il rumore di un istante per il decodificatore: le bande laterali, ma mai sotto il minimo scorrevole (che sul solo rumore vale la
/// media); senza bande laterali il minimo, senza nessuno dei due 0 (non si sa).
fn noise_of(side: f64, floor: f64) -> f64 {
    if side > 0.0 { side.max(NOISE_FLOOR_SHARE * floor) } else { floor }
}

/// Quanto del minimo scorrevole fa da pavimento alle bande laterali.
const NOISE_FLOOR_SHARE: f64 = 0.0;

/// Decodifica SOM di fldigi (`cw::find_winner` e `cw::normalize`, tabella `som_table` di `src/cw/cw.cxx`): le durate dei toni si
/// normalizzano (la piu' lunga vale 1 se e' una linea, 0,33 se e' un punto) e vince il carattere con la distanza minima, dove il punto
/// vale 0,33 e la linea 1.
fn som_winner(durations: &[f64], threshold: f64) -> Option<&'static str> {
    const SIZE: usize = 7;
    let max = durations.iter().copied().fold(0.0_f64, f64::max);
    if max <= 0.0 || durations.len() > SIZE {
        return None;
    }
    let ratio = if max > threshold { 1.0 } else { 0.33 } / max;
    let mut v = [0.0_f64; SIZE];
    for (x, d) in v.iter_mut().zip(durations) {
        *x = d * ratio;
    }
    let candidates = MORSE.iter().map(|(_, p)| *p).chain(PROSIGNS.iter().map(|(_, p)| *p));
    candidates
        .filter(|p| p.len() <= SIZE)
        .map(|p| {
            let mut w = [0.0_f64; SIZE];
            for (x, e) in w.iter_mut().zip(p.bytes()) {
                *x = if e == b'-' { 1.0 } else { 0.33 };
            }
            let d: f64 = v.iter().zip(w).map(|(a, b)| (a - b) * (a - b)).sum();
            (p, d)
        })
        .min_by(|a, b| a.1.total_cmp(&b.1))
        .map(|(p, _)| p)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn decode(wave: &[f32], tone: f32, hint: f32) -> String {
        let mut d = CwDemodulator::new(tone, hint);
        let mut audio = wave.to_vec();
        // mezzo secondo di silenzio alla fine fa uscire l'ultimo carattere
        audio.extend(std::iter::repeat_n(0.0, RATE as usize));
        let mut text = Vec::new();
        for chunk in audio.chunks(480) {
            text.extend(d.process(chunk));
        }
        String::from_utf8_lossy(&text).trim().to_string()
    }

    #[test]
    fn morse_table_round_trips_and_has_no_duplicates() {
        for (c, p) in MORSE {
            assert_eq!(char_of(p), Some(*c), "{p}");
        }
        assert_eq!(pattern_of(b'k'), Some("-.-"));
        assert_eq!(pattern_of(b'#'), None);
    }

    /// Sul solo rumore le bande laterali e il minimo scorrevole danno la potenza di rumore del filtro del decodificatore (entro il 15%),
    /// con tutte le bande.
    #[test]
    fn noise_estimates_match_the_decoder_filter() {
        let mut all = Vec::new();
        for bw in [100.0, 150.0, 250.0, 500.0, 1000.0] {
            let mut d = CwDemodulator::new(700.0, 20.0);
            d.set_bandwidth(bw);
            let mut x = 12345u64;
            let audio: Vec<f32> = (0..12_000 * 8)
                .map(|_| {
                    // somma di 12 uniformi: quasi gaussiana
                    (0..12)
                        .map(|_| {
                            x ^= x << 13;
                            x ^= x >> 7;
                            x ^= x << 17;
                            (x >> 40) as f32 / (1u64 << 24) as f32 - 0.5
                        })
                        .sum::<f32>()
                        * 0.1
                })
                .collect();
            let (mut side, mut floor, mut power, mut n) = (0.0, 0.0, 0.0, 0.0);
            for (k, chunk) in audio.chunks(480).enumerate() {
                d.process(chunk);
                if k > 100 {
                    for e in &d.delay {
                        side += e.side;
                        floor += e.floor;
                        power += f64::from(e.b.0 * e.b.0 + e.b.1 * e.b.1);
                        n += 1.0;
                    }
                }
            }
            let (side, floor) = (side / power, floor / power);
            println!("banda {bw}: bande laterali {side:.3} minimo {floor:.3}");
            let _ = n;
            all.push((bw, side, floor));
        }
        for (bw, side, floor) in all {
            assert!((side - 1.0).abs() < 0.15, "banda {bw}: bande laterali {side:.2}");
            assert!((floor - 1.0).abs() < 0.15, "banda {bw}: minimo scorrevole {floor:.2}");
        }
    }

    /// La velocità capita (quella che «TX alla velocità ricevuta» usa per trasmettere) sta entro il 5% di quella vera, anche lontana da
    /// quella di partenza (prima andava a gradini del 9%, le velocità del traliccio).
    #[test]
    fn the_received_speed_is_measured_within_five_percent() {
        for wpm in [16.0_f32, 25.0, 32.0] {
            let mut d = CwDemodulator::new(700.0, 20.0);
            let wave = modulate(b"CQ CQ DE IU3ABC IU3ABC K TEST 599 73", 700.0, wpm, 0.8);
            for chunk in wave.chunks(480) {
                d.process(chunk);
            }
            assert!((d.wpm() / wpm - 1.0).abs() < 0.05, "a {wpm} WPM ne capisce {}", d.wpm());
        }
    }

    #[test]
    fn paris_lasts_fifty_units() {
        // «PARIS » a 20 WPM dura 3 secondi (50 unita' da 60 ms), con il silenzio d'avvio e di coda
        let wave = modulate(b"PARIS ", 700.0, 20.0, 0.9);
        let secs = wave.len() as f64 / f64::from(RATE) - 0.05 - 0.25;
        assert!((secs - 3.0).abs() < 0.02, "{secs}");
    }

    #[test]
    fn it_hears_what_it_sends_at_several_speeds() {
        let text = "CQ CQ DE IU3ABC K";
        for wpm in [12.0, 20.0, 30.0] {
            let wave = modulate(text.as_bytes(), 700.0, wpm, 0.8);
            assert_eq!(decode(&wave, 700.0, 20.0), text, "a {wpm} WPM");
        }
    }

    #[test]
    fn a_long_silence_before_does_not_eat_the_first_character() {
        for lead in [6000usize, 30_000] {
            for afc in [false, true] {
                let mut wave = vec![0.0; lead];
                wave.extend(modulate(b"CQ IU3ABC", 1000.0, 20.0, 0.9));
                wave.extend(vec![0.0; 12_000]);
                let mut d = CwDemodulator::new(1000.0, 20.0);
                d.set_afc(afc);
                assert_eq!(listen(&mut d, &wave), "CQ IU3ABC", "{lead} campioni di silenzio prima, AFC {afc}");
            }
        }
    }

    #[test]
    fn it_adapts_to_a_speed_far_from_the_hint() {
        // lontano dal valore di partenza i primi segni si sbagliano: un avvio «VVV» fa capire la velocita'
        let text = "TNX FER QSO 73 DE IU3ABC";
        let wave = modulate(format!("VVV VVV {text}").as_bytes(), 600.0, 30.0, 0.7);
        let heard = decode(&wave, 600.0, 18.0);
        assert!(heard.ends_with(text), "{heard}");
    }

    #[test]
    fn digits_and_punctuation_pass() {
        let text = "RST 599 QTH? 1+2=3";
        let wave = modulate(text.as_bytes(), 800.0, 22.0, 0.9);
        assert_eq!(decode(&wave, 800.0, 22.0), "RST 599 QTH? 1<AR>2<BT>3", "+ e = sono anche <AR> e <BT>: si stampano come segni di procedura");
    }

    #[test]
    fn silence_and_noise_print_nothing() {
        let mut d = CwDemodulator::new(700.0, 20.0);
        let mut seed = 12345_u32;
        let noise: Vec<f32> = (0..RATE as usize * 3)
            .map(|_| {
                seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
                ((seed >> 16) as f32 / 32768.0 - 1.0) * 0.05
            })
            .collect();
        assert!(d.process(&noise).is_empty());
        assert!(d.process(&vec![0.0; RATE as usize]).is_empty());
    }

    #[test]
    fn a_different_tone_is_not_decoded() {
        // un segnale forte 800 Hz piu' in alto, con un fondo di rumore 40 dB sotto (i «click» dei suoi fronti restano sotto il rumore)
        let clean = modulate(b"CQ CQ DE IU3ABC", 1500.0, 20.0, 0.9);
        let wave: Vec<f32> = clean.iter().zip(white(clean.len(), 0.01, 77)).map(|(s, n)| s + n).collect();
        assert_eq!(decode(&wave, 700.0, 20.0), "");
    }

    #[test]
    fn the_squelch_looks_at_the_signal_to_noise_ratio() {
        // lo stesso segnale debolissimo ma pulito passa (lo squelch non guarda il livello), il solo rumore no
        let wave = modulate(b"CQ DE IU3ABC", 700.0, 20.0, 0.003);
        let mut d = CwDemodulator::new(700.0, 20.0);
        d.set_squelch_snr_db(Some(SQUELCH_DEFAULT_DB));
        let mut text = Vec::new();
        for chunk in wave.chunks(480) {
            text.extend(d.process(chunk));
        }
        text.extend(d.process(&vec![0.0; RATE as usize]));
        assert_eq!(String::from_utf8_lossy(&text).trim(), "CQ DE IU3ABC");
        let mut seed = 3_u32;
        let noise: Vec<f32> = (0..RATE as usize * 20)
            .map(|_| {
                seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
                ((seed >> 16) as f32 / 32768.0 - 1.0) * 0.3
            })
            .collect();
        let mut d = CwDemodulator::new(700.0, 20.0);
        d.set_squelch_snr_db(Some(SQUELCH_DEFAULT_DB));
        assert!(d.process(&noise).is_empty());
        assert!(!d.signal().1, "S/N sul rumore: {}", d.snr_db());
    }

    /// Rumore bianco uniforme da xorshift64* (il generatore congruenziale semplice ha righe nello spettro, che lo S/N e l'AFC vedono come
    /// segnali).
    fn white(n: usize, amp: f32, seed: u64) -> Vec<f32> {
        let mut x = seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1;
        (0..n)
            .map(|_| {
                x ^= x >> 12;
                x ^= x << 25;
                x ^= x >> 27;
                ((x.wrapping_mul(0x2545_F491_4F6C_DD1D) >> 40) as f32 / (1u64 << 23) as f32 - 1.0) * amp
            })
            .collect()
    }

    /// Morse del testo con un tono che cambia nel tempo (`hz(t)`), fronti di 5 ms, e rumore uniforme di ampiezza `noise`.
    fn keyed(text: &str, wpm: f32, hz: impl Fn(f64) -> f64, noise: f32, seed: u32) -> Vec<f32> {
        let unit = unit_seconds(wpm);
        let mut seq = vec![(false, 0.5)];
        for sign in signs_of(text.as_bytes()) {
            match sign {
                None => seq.push((false, 4.0 * unit)),
                Some(p) => {
                    for e in p.bytes() {
                        seq.push((true, if e == b'-' { 3.0 } else { 1.0 } * unit));
                        seq.push((false, unit));
                    }
                    seq.push((false, 2.0 * unit));
                }
            }
        }
        seq.push((false, 1.5));
        let mut out = Vec::new();
        let (mut phase, mut seed) = (0.0_f64, seed);
        let mut t_end = 0.0;
        for (on, secs) in seq {
            let start = out.len();
            t_end += secs * f64::from(RATE);
            let len = (t_end.round() as usize).saturating_sub(start);
            for i in 0..len {
                let t = (start + i) as f64 / f64::from(RATE);
                phase += 2.0 * PI * hz(t) / f64::from(RATE);
                let edge = i.min(len - 1 - i) as f64;
                let w = if !on { 0.0 } else if edge < 60.0 { 0.5 * (1.0 - (PI * edge / 60.0).cos()) } else { 1.0 };
                seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
                out.push((0.3 * w * phase.sin()) as f32 + ((seed >> 16) as f32 / 32768.0 - 1.0) * noise);
            }
        }
        out
    }

    fn listen(d: &mut CwDemodulator, audio: &[f32]) -> String {
        let mut text = Vec::new();
        for chunk in audio.chunks(480) {
            text.extend(d.process(chunk));
        }
        String::from_utf8_lossy(&text).trim().to_string()
    }

    /// Issue #46: l'AFC del CW non faceva niente. Con il filtro stretto (50 Hz) un tono 60 Hz sopra (o 55 sotto) non si sente; con l'AFC
    /// si aggancia, si decodifica e la frequenza arriva sul tono.
    #[test]
    fn the_afc_locks_on_a_tone_tens_of_hertz_away() {
        let text = "CQ CQ CQ DE IU3ABC IU3ABC IU3ABC K";
        for offset in [60.0, -55.0] {
            let audio = keyed(text, 20.0, |_| 700.0 + offset, 0.05, 11);
            let mut off = CwDemodulator::new(700.0, 20.0);
            off.set_bandwidth(50.0);
            assert!(!listen(&mut off, &audio).contains("IU3ABC"), "senza AFC il tono a {offset} Hz non deve passare dal filtro di 50 Hz");
            let mut on = CwDemodulator::new(700.0, 20.0);
            on.set_bandwidth(50.0);
            on.set_afc(true);
            let heard = listen(&mut on, &audio);
            assert!(heard.ends_with("IU3ABC IU3ABC K"), "offset {offset}: {heard}");
            assert!((on.frequency() - (700.0 + offset as f32)).abs() < 2.0, "offset {offset}: tono {}", on.frequency());
        }
    }

    #[test]
    fn the_afc_follows_a_slow_drift() {
        // 20 Hz di deriva in mezzo minuto, con rumore
        let text = "CQ CQ DE IU3ABC IU3ABC TEST TEST 599 599 TNX FER QSO 73 73 DE IU3ABC K";
        let audio = keyed(text, 18.0, |t| 690.0 + 20.0 * (t / 30.0).min(1.0), 0.1, 5);
        let mut d = CwDemodulator::new(690.0, 18.0);
        d.set_afc(true);
        let mut far = 0.0_f32;
        let mut text_out = Vec::new();
        for chunk in audio.chunks(480) {
            text_out.extend(d.process(chunk));
        }
        let secs = audio.len() as f64 / f64::from(RATE);
        let expected = 690.0 + 20.0 * (secs / 30.0).min(1.0);
        far = far.max((d.frequency() - expected as f32).abs());
        let heard = String::from_utf8_lossy(&text_out).into_owned();
        assert!(far < 3.0, "tono {} invece di {expected}", d.frequency());
        assert!(heard.contains("TNX FER QSO 73 73 DE IU3ABC K"), "{heard}");
    }

    #[test]
    fn the_afc_does_not_move_on_noise() {
        let noise = white(RATE as usize * 30, 0.3, 21);
        for bw in BANDWIDTHS {
            let mut d = CwDemodulator::new(700.0, 20.0);
            d.set_bandwidth(bw);
            d.set_afc(true);
            let mut far = 0.0_f32;
            for chunk in noise.chunks(480) {
                assert!(d.process(chunk).is_empty(), "banda {bw}: caratteri dal rumore");
                far = far.max((d.frequency() - 700.0).abs());
            }
            assert!(far < 0.01, "banda {bw}: l'AFC si e' mosso di {far} Hz sul rumore");
        }
    }

    #[test]
    fn prosigns_go_out_as_one_sign_and_come_back() {
        let wave = modulate(b"TNX 73 <SK> <KN> <AR>", 700.0, 22.0, 0.8);
        assert_eq!(decode(&wave, 700.0, 22.0), "TNX 73 <SK> <KN> <AR>");
    }

    #[test]
    fn the_som_decoder_reads_a_clean_signal_too() {
        let wave = modulate(b"CQ DE IU3ABC 599", 700.0, 20.0, 0.8);
        let mut d = CwDemodulator::new(700.0, 20.0);
        d.set_som(true);
        let mut audio = wave.clone();
        audio.extend(vec![0.0; RATE as usize]);
        assert_eq!(listen(&mut d, &audio), "CQ DE IU3ABC 599");
    }

    #[test]
    fn every_bandwidth_decodes() {
        let wave = modulate(b"CQ DE IU3ABC K", 700.0, 25.0, 0.5);
        for bw in BANDWIDTHS {
            let mut d = CwDemodulator::new(700.0, 25.0);
            d.set_bandwidth(bw);
            let mut audio = wave.clone();
            audio.extend(vec![0.0; RATE as usize]);
            assert_eq!(listen(&mut d, &audio), "CQ DE IU3ABC K", "banda {bw}");
        }
    }
}
