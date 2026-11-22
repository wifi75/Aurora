//! Ricevitore WEFAX: riconoscimento di APT start e stop, fasatura, allineamento delle righe, immagine che cresce riga per riga.
//!
//! Riscritto in Rust dalla logica di ricezione di fldigi 4.2.13 (`wefax.cxx`, `wefax-pic.cxx`: W1HKJ e Remi Chateauneu, GPL 3, derivata da
//! HAMFAX di Christof Schmitt), con queste differenze:
//! - il riconoscimento del segnale non usa le potenze della cascata (fldigi le legge dal waterfall) ma solo la correlazione tra righe
//!   consecutive, che in fldigi lavora insieme a quelle;
//! - la posizione dei pixel si ricava dalla riga e dalla colonna del campione (in fldigi da un contatore progressivo con un fattore
//!   di inclinazione), cosi' l'inclinazione si corregge con il periodo di riga misurato sulla fasatura, senza buchi nell'immagine;
//! - l'AFC non usa il waterfall come fldigi ma l'istogramma della frequenza istantanea (vedi [`crate::meter::Afc`]): sposta la
//!   sottoportante del demodulatore di quanto serve, senza cambiare quella scelta nelle impostazioni;
//! - lo squelch (SQL) fa partire l'immagine quando il segnale supera la soglia e cambia (un tono fisso no) e la chiude dopo 3 secondi
//!   sotto la soglia;
//! - sul tono APT stop si tolgono dall'immagine le righe del tono (fldigi lo lascia).

use serde::{Deserialize, Serialize};

use crate::{
    GrayImage,
    demod::{Filter, FmDemod, RATE},
    meter::{AFC_MAX_HZ, Afc, METER_BLOCK, SignalMeter},
};

/// Misure di livello di fila sopra la soglia, con un segnale che cambia, per far partire l'immagine con lo squelch (circa mezzo secondo).
const SQL_OPEN_BLOCKS: u32 = 6;
/// Misure di fila sotto la soglia che chiudono l'immagine (circa 3 secondi).
const SQL_CLOSE_BLOCKS: u32 = (3 * RATE as usize / METER_BLOCK) as u32;
/// Misure di fila sotto la soglia che riarmano lo squelch dopo un'immagine (mezzo secondo).
/// Quota minima di potenza dei grigi di mezzo secondo su un tono d'inizio perche' conti come tono (un tono puro vale 1).
const TONE_SHARE_MIN: f64 = 0.3;
const SQL_REARM_BLOCKS: u32 = 6;
/// Quanto dopo la fine di un'immagine si ignora lo strascico del tono di fine (campioni: 20 s, piu' del tono di fine di 5 s e di un
/// eventuale silenzio nero).
const AFTER_END_HOLD: u32 = 20 * SR as u32;
/// Deviazione minima dei grigi demodulati in una misura perche' il segnale conti come immagine: il tono fisso che le stazioni mandano
/// prima di trasmettere (nero a 1500 Hz) e' segnale presente ma non fa partire niente.
const SQL_MIN_SPREAD: f32 = 12.0;
/// Sotto questo livello l'AFC non corregge (solo rumore).
const AFC_MIN_LEVEL: f32 = 10.0;
/// Sotto quale errore l'AFC non si muove (Hz).
const AFC_DEADBAND_HZ: f32 = 3.0;
/// Quota dell'errore corretta a ogni misura (una al secondo) e passo massimo (limite locale) sul tono d'inizio e sulla fasatura, dove l'errore
/// si legge meglio: un salto grande si raggiunge in piu' misure.
const AFC_GAIN: f32 = 0.7;
const AFC_STEP_MAX_HZ: f32 = 25.0;
/// Dentro l'immagine (il segnale e' il contenuto della cartina) l'AFC e' piu' cauto: quota, passo massimo e zona morta piu' piccoli o piu' larghi.
const AFC_IMAGE_GAIN: f32 = 0.25;
const AFC_IMAGE_STEP_MAX_HZ: f32 = 8.0;
const AFC_IMAGE_DEADBAND_HZ: f32 = 6.0;
/// Estremi ammessi per `afc_range_hz`.
pub const AFC_RANGE_MIN_HZ: f32 = 20.0;
pub const AFC_RANGE_MAX_HZ: f32 = 400.0;

const SR: i32 = RATE as i32;
const BYTES_PER_PIXEL: i64 = 3;
const APT_STOP_HZ: i32 = 450;
/// Altezza iniziale dell'immagine in righe e passo di crescita (come fldigi).
const INITIAL_ROWS: usize = 1300;
const ROWS_GROWTH: usize = 100;
/// Ogni quanti campioni si rivaluta lo stato del segnale.
const SIGNAL_VALIDITY: u32 = 1000;
/// Larghezza (in pixel) della fascia scura che si cerca per centrare l'immagine: 180 byte in fldigi, tre per pixel.
const CENTER_BAND: usize = 60;
/// Campioni di ritardo con cui si riconosce la fine della banda bianca della fasatura rispetto ai pixel dell'immagine (misurato con
/// segnali sintetici: media su 16 campioni piu' fronti del filtro d'ingresso).
const PHASING_LATENCY: i64 = 19;
/// Un'immagine che parte dopo la fasatura si annuncia all'interfaccia solo con queste righe complete: le immagini di poche righe (le righe
/// di fasatura che la ricezione riprende ogni 4 righe) restano nascoste.
const ANNOUNCE_ROWS: u32 = 8;
/// Inclinazione automatica: ogni quante righe si misura, fino a quale riga, distanza tra le righe confrontate, soglie.
const SLANT_EVERY: usize = 40;
const SLANT_LAST_ROW: usize = 3000;
const SLANT_GAP: usize = 16;
/// Distanza tra le righe della stima grossolana (su 4 righe, 5000 ppm su 1809 pixel sono 36 pixel: meno di mezzo passo delle griglie
/// di meridiani consuete) e semiampiezza (pixel) della ricerca fine intorno a essa.
const SLANT_COARSE_GAP: usize = 4;
const SLANT_FINE_RANGE_PX: i64 = 12;
/// Correzione massima accumulata (ppm).
const SLANT_MAX_PPM: f64 = 5000.0;
/// Sotto questa deriva (pixel per riga) non si corregge.
const SLANT_DEADBAND_PX: f64 = 0.03;
/// Deviazione minima dei grigi di una riga perche' conti (le righe quasi uniformi non hanno strutture da seguire).
const SLANT_MIN_STD: f32 = 12.0;
/// Correlazione minima del picco, e quanto deve superare il secondo picco (fuori da 4 pixel dal primo).
const SLANT_MIN_NCC: f32 = 0.5;
const SLANT_PEAK_RATIO: f32 = 0.9;
/// Coppie di righe valide minime e dispersione massima (pixel per riga) delle misure.
const SLANT_MIN_PAIRS: usize = 5;
const SLANT_MAX_SPREAD_PX: f64 = 0.3;
/// Errore di orologio ricordato: righe minime con una stima valida perche' l'immagine conti, righe che danno il peso pieno, peso massimo accumulato
/// (le immagini vecchie contano via via meno) e limite del valore (ppm).
const LEARN_MIN_ROWS: usize = 80;
const LEARN_FULL_ROWS: f64 = 200.0;
const LEARN_MAX_WEIGHT: f64 = 8.0;
const LEARN_MAX_PPM: f64 = 5000.0;
/// Righe della fasatura che servono per dare l'allineamento.
const PHASING_LINES: i32 = 4;
/// Alcune stazioni (DDK3 su 7880 kHz) mandano solo due barre di fasatura, senza tono d'inizio, e poi subito l'immagine: ne esce una sola
/// riga misurabile. Con almeno queste righe buone, finite le quali comincia l'immagine, ci si allinea lo stesso.
const PHASING_LINES_SHORT: i32 = 1;
/// Quota minima di nero di una riga di fasatura mentre la si aspetta (la norma e' 95%, DDK3 ne manda circa il 90%).
const PHASE_BLACK_MIN: f64 = 0.80;
/// Quota minima di nero dentro l'immagine, dove una barra scura della cartina non deve sembrare una fasatura.
const PHASE_BLACK_MIN_IMAGE: f64 = 0.94;
/// Aspettando la fasatura senza averne vista nemmeno una riga (un falso tono d'inizio), dopo tanto si torna ad aspettare il tono.
const PHASING_WAIT_S: i64 = 90;

/// Gli stati di fldigi (`fax_state`) che servono alla ricezione.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Fs {
    AptStart,
    AptStop,
    Phasing,
    Image,
    Idle,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct DecoderConfig {
    /// 576 o 288.
    pub ioc: u32,
    /// L'IOC si riconosce dal tono d'inizio (300 Hz con IOC 576, 675 Hz con IOC 288): se e' quello dell'altro IOC, larghezza e righe al
    /// minuto di partenza cambiano prima della fasatura. Non agisce nel modo manuale ne' a immagine cominciata.
    pub auto_ioc: bool,
    /// Righe al minuto di partenza con IOC 576 (60, 90, 120, 240).
    pub lpm_576: u32,
    /// Righe al minuto di partenza con IOC 288.
    pub lpm_288: u32,
    pub filter: Filter,
    /// Senza APT ne' fasatura: l'immagine comincia subito e finisce solo a mano o al massimo di righe.
    pub manual: bool,
    /// Il numero di righe al minuto si ricava dalla fasatura (60, 90, 120 o 240 entro 3).
    pub auto_lpm: bool,
    /// Ogni 10 righe (dalla 30 alla 500) si ricentra l'immagine sulla fascia scura piu' larga.
    pub auto_center: bool,
    /// Ogni 40 righe (dalla 40 alla 400) si misura l'inclinazione sul contenuto delle righe gia' ricevute e si corregge il periodo di riga
    /// (e le righe gia' scritte), sopra la fasatura e la correzione a mano.
    pub auto_slant: bool,
    pub center_hz: f32,
    pub shift_hz: f32,
    /// Correzione a mano del periodo di riga in parti per milione (positivo: righe piu' lunghe), sopra quella automatica.
    pub slant_ppm: f32,
    /// Errore di orologio ricordato dalle immagini precedenti (ppm) e il suo peso: valori di partenza del decodificatore (poi li tiene lui,
    /// vedi [`Decoder::learned_ppm`]); le impostazioni nuove non li cambiano.
    pub learned_ppm: f32,
    pub learned_weight: f32,
    pub max_rows: u32,
    /// Soglia di correlazione tra righe sotto cui il segnale e' perso (0,05 in fldigi).
    pub correlation: f32,
    /// Quante righe di correlazione si mediano (15 in fldigi).
    pub correlation_rows: u32,
    /// Sotto questo numero di pixel ricevuti l'immagine non si tiene (150000 byte, 50000 pixel, in fldigi).
    pub min_pixels: u32,
    /// Correzione automatica della sintonia (sposta la sottoportante del demodulatore sul segnale vero).
    pub afc: bool,
    /// Limite globale dell'AFC: la correzione non supera questi Hz dall'ultima sottoportante scelta a mano (20..400).
    pub afc_range_hz: f32,
    /// Squelch: l'immagine parte quando il livello del segnale supera `squelch_level` e si chiude quando resta sotto per 3 secondi.
    pub squelch: bool,
    /// Soglia dello squelch (0..100, 20 dB = 100).
    pub squelch_level: f32,
}

impl Default for DecoderConfig {
    fn default() -> Self {
        DecoderConfig {
            ioc: 576,
            auto_ioc: true,
            lpm_576: 120,
            lpm_288: 60,
            filter: Filter::Narrow,
            manual: false,
            auto_lpm: true,
            auto_center: true,
            auto_slant: true,
            center_hz: 1900.0,
            shift_hz: 800.0,
            slant_ppm: 0.0,
            learned_ppm: 0.0,
            learned_weight: 0.0,
            max_rows: 4000,
            correlation: 0.05,
            correlation_rows: 15,
            min_pixels: 50_000,
            afc: true,
            afc_range_hz: AFC_MAX_HZ,
            squelch: false,
            squelch_level: 30.0,
        }
    }
}

impl DecoderConfig {
    /// Colonne dell'immagine: IOC x pi.
    pub fn width(&self) -> u32 {
        width_of(self.ioc)
    }

    fn default_lpm(&self) -> u32 {
        if self.ioc == 288 { self.lpm_288 } else { self.lpm_576 }
    }

    /// Frequenza del tono APT start (300 Hz con IOC 576, 675 Hz con IOC 288).
    pub fn apt_start_hz(&self) -> i32 {
        if self.ioc == 288 { 675 } else { 300 }
    }
}

/// Colonne dell'immagine per un IOC: IOC x pi, troncato (1809 con 576, 904 con 288).
pub fn width_of(ioc: u32) -> u32 {
    (f64::from(ioc) * std::f64::consts::PI) as u32
}

/// Come l'immagine e' cominciata.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Source {
    /// Dopo APT start e fasatura, con la riga allineata.
    Phasing,
    /// Fasatura finita senza l'allineamento (righe di fasatura perse).
    NoPhasing,
    /// Modo manuale.
    Manual,
    /// Comando di partire subito.
    Forced,
    /// Partita dallo squelch: segnale sopra la soglia (riga non allineata).
    Squelch,
}

impl Source {
    pub fn id(self) -> &'static str {
        match self {
            Source::Phasing => "phasing",
            Source::NoPhasing => "no_phasing",
            Source::Manual => "manual",
            Source::Forced => "forced",
            Source::Squelch => "squelch",
        }
    }
}

/// Un'immagine finita.
#[derive(Clone, Debug)]
pub struct Done {
    pub image: GrayImage,
    pub ioc: u32,
    pub lpm: f32,
    /// Perche' e' finita: `apt_stop`, `no_signal`, `apt_start`, `phasing`, `max_rows`, `manual`...
    pub reason: String,
    /// Chiusa dal tono APT stop (o dal comando), non per segnale perso.
    pub complete: bool,
    /// Correlazione massima tra righe (qualita').
    pub correlation: f32,
    pub source: Source,
}

#[derive(Clone, Debug)]
pub enum DecoderEvent {
    /// Il tono d'inizio era quello dell'altro IOC: da qui si riceve con `ioc` (larghezza e righe al minuto di partenza nuove).
    IocDetected { from: u32, ioc: u32, lpm: f32, width: u32 },
    /// L'immagine comincia.
    Started { ioc: u32, lpm: f32, width: u32, source: Source },
    /// Righe nuove complete da `first` (`count` righe); si legge con [`Decoder::current`].
    Rows { first: u32, count: u32 },
    Finished(Done),
    /// L'immagine in corso e' stata scartata (troppo piccola, rumore, annullata).
    Discarded { reason: String },
    /// Cambio di stato: `apt` (aspetta APT start), `phasing`, `image`.
    State(&'static str),
}

struct Img {
    data: Vec<u8>,
    /// Righe allocate.
    rows_alloc: usize,
    /// Indice dell'ultima riga con almeno un pixel, piu' uno.
    rows_written: usize,
    /// Riga del campione alla partenza dell'immagine: la riga 0 e' quella.
    row_origin: i64,
    /// Righe gia' mandate all'interfaccia.
    emitted: u32,
    resend: bool,
    /// L'evento `Started` e' partito.
    announced: bool,
    source: Source,
}

pub struct Decoder {
    cfg: DecoderConfig,
    /// L'IOC scelto nelle impostazioni: `cfg.ioc` puo' esserne diverso dopo un riconoscimento dal tono d'inizio.
    base_ioc: u32,
    demod: FmDemod,
    demod_buf: Vec<u8>,
    width: usize,
    state: Fs,
    reported: Option<&'static str>,
    events: Vec<DecoderEvent>,
    level: f32,

    // APT
    apt_high: bool,
    apt_trans: i32,
    apt_count: i32,
    curr_freq: i32,
    cr1_freq: i32,
    cr2_freq: i32,
    cr3_freq: i32,
    /// Finestra (mezzo secondo) dei grigi demodulati in attesa del tono d'inizio, e se le ultime tre contenevano il tono dell'altro IOC.
    apt_win: Vec<u8>,
    alt_hit: [bool; 3],

    // fasatura
    phase_high: bool,
    curr_phase_len: i32,
    curr_phase_high: i32,
    curr_phase_low: i32,
    phase_lines: i32,
    num_phase_lines: i32,
    /// Valore di `img_sample` all'ultimo passaggio bianco-nero di una riga di fasatura buona.
    phase_edge_sample: i64,
    phase_len_sum: i64,
    phasing_hist: [i32; 16],
    phasing_count: usize,

    // righe e immagine
    current_value: i32,
    /// Lunghezza della riga misurata sulla fasatura.
    measured_len: Option<f64>,
    lpm_img: f64,
    smpl_per_lin: f64,
    /// Correzione dell'inclinazione trovata dal contenuto dell'immagine (ppm), sopra la fasatura e la correzione a mano.
    auto_ppm: f64,
    /// Posizione di riga (frazionaria) al campione `slant_base_sample`: cambiando il periodo la posizione resta continua.
    slant_base_row: f64,
    slant_base_sample: i64,
    /// Errore di orologio ricordato (ppm) e peso accumulato: si aggiunge al periodo nominale quando la fasatura non l'ha misurato.
    learned_ppm: f64,
    learned_weight: f64,
    /// Periodo nominale (campioni) con cui e' stato scelto `measured_len`.
    measured_nominal: f64,
    /// Righe coperte dalle misure valide dell'inclinazione dell'immagine in corso.
    slant_valid_rows: usize,
    /// Riga dell'ultima misura dell'inclinazione.
    last_slant_row: usize,
    /// Posizione di riga (relativa alla prima) dell'ultima correzione.
    last_slant_pos: f64,
    img_sample: i64,
    last_col: i32,
    last_row: i64,
    pixel_val: i32,
    pix_samples_nb: i32,
    /// Contatore dei byte come `m_fax_pix_num` di fldigi (tre per pixel): serve alla soglia di immagine troppo piccola.
    fax_pix_num: i64,
    stats: [u32; 256],
    shift: i32,
    last_center_row: i64,
    image: Option<Img>,
    pending_source: Source,

    // correlazione
    corr_buf: Vec<u8>,
    corr_avg: f64,
    corr_max: f64,
    corr_calls: u32,
    cnt_upd: u64,
    stable_state: Fs,
    stop_code: &'static str,
    sig_cnt: u32,
    sig_state: Fs,

    // squelch e AFC
    meter: SignalMeter,
    afc: Afc,
    freq_buf: Vec<f32>,
    /// Correzione dell'AFC sopra la sottoportante delle impostazioni (Hz).
    afc_hz: f32,
    sql_open: u32,
    sql_closed: u32,
    /// Lo squelch puo' far partire un'immagine (dopo un'immagine serve prima che il segnale sparisca).
    sql_armed: bool,
    /// Campioni dalla fine dell'ultima immagine (saturato). Per `AFTER_END_HOLD` campioni il tono di fine e il suo strascico (che la
    /// correlazione scambia per immagine e l'AFC per un tono nero o bianco) non fanno partire la fasatura ne' spostare la sottoportante:
    /// la trasmissione che comincia subito dopo, magari con l'altro IOC, deve poter essere riconosciuta dal suo tono d'inizio.
    since_end: u32,
    /// Somme dei grigi demodulati della misura in corso (per la deviazione).
    spread_sum: f64,
    spread_sq: f64,
    spread_n: u32,
}

fn near(a: i32, b: i32, margin: i32) -> bool {
    (a - b).abs() < margin
}

/// Quota della potenza di una finestra di grigi (senza la media) che sta in una riga a `hz` (entro 6 Hz): vicina a 1 per un tono puro, quasi
/// 0 per rumore o immagine. 0 se la finestra e' quasi piatta (deviazione sotto 6 livelli).
fn tone_share(win: &[u8], hz: f64) -> f64 {
    let n = win.len() as f64;
    let mean = win.iter().map(|&v| f64::from(v)).sum::<f64>() / n;
    let energy: f64 = win.iter().map(|&v| (f64::from(v) - mean).powi(2)).sum();
    if energy / n < 36.0 {
        return 0.0;
    }
    let mut best = 0.0f64;
    for k in -2..=2 {
        let w = std::f64::consts::TAU * (hz + 3.0 * f64::from(k)) / f64::from(RATE);
        let coeff = 2.0 * w.cos();
        let (mut s1, mut s2) = (0.0f64, 0.0f64);
        for &v in win {
            let s0 = (f64::from(v) - mean) + coeff * s1 - s2;
            s2 = s1;
            s1 = s0;
        }
        let power = s1 * s1 + s2 * s2 - coeff * s1 * s2;
        best = best.max(2.0 * power / (n * energy));
    }
    best
}

fn lpm_to_samples(lpm: u32) -> i64 {
    i64::from(SR) * 60 / i64::from(lpm.max(1))
}

impl Decoder {
    pub fn new(cfg: DecoderConfig) -> Decoder {
        let width = cfg.width() as usize;
        let mut d = Decoder {
            base_ioc: cfg.ioc,
            demod: FmDemod::new(cfg.filter, cfg.center_hz, cfg.shift_hz),
            demod_buf: Vec::new(),
            width,
            state: Fs::AptStart,
            reported: None,
            events: Vec::new(),
            level: 0.0,
            apt_high: false,
            apt_trans: 0,
            apt_count: 0,
            curr_freq: 0,
            cr1_freq: 0,
            cr2_freq: 0,
            cr3_freq: 0,
            apt_win: Vec::new(),
            alt_hit: [false; 3],
            phase_high: false,
            curr_phase_len: 0,
            curr_phase_high: 0,
            curr_phase_low: 0,
            phase_lines: 0,
            num_phase_lines: 0,
            phase_edge_sample: 0,
            phase_len_sum: 0,
            phasing_hist: [0; 16],
            phasing_count: 0,
            current_value: 0,
            measured_len: None,
            lpm_img: 0.0,
            smpl_per_lin: 1.0,
            auto_ppm: 0.0,
            learned_ppm: f64::from(cfg.learned_ppm).clamp(-LEARN_MAX_PPM, LEARN_MAX_PPM),
            learned_weight: f64::from(cfg.learned_weight).clamp(0.0, LEARN_MAX_WEIGHT),
            measured_nominal: 0.0,
            slant_valid_rows: 0,
            slant_base_row: 0.0,
            slant_base_sample: 0,
            last_slant_row: 0,
            last_slant_pos: 0.0,
            img_sample: 0,
            last_col: 0,
            last_row: 0,
            pixel_val: 0,
            pix_samples_nb: 0,
            fax_pix_num: 0,
            stats: [0; 256],
            shift: 0,
            last_center_row: 0,
            image: None,
            pending_source: Source::Phasing,
            corr_buf: Vec::new(),
            corr_avg: 0.0,
            corr_max: 0.0,
            corr_calls: 0,
            cnt_upd: 0,
            stable_state: Fs::Idle,
            stop_code: "",
            sig_cnt: 0,
            sig_state: Fs::Idle,
            meter: SignalMeter::new(cfg.center_hz, cfg.shift_hz),
            afc: Afc::default(),
            freq_buf: Vec::new(),
            afc_hz: 0.0,
            sql_open: 0,
            sql_closed: 0,
            sql_armed: true,
            since_end: u32::MAX,
            spread_sum: 0.0,
            spread_sq: 0.0,
            spread_n: 0,
            cfg,
        };
        d.lpm_set();
        d
    }

    pub fn config(&self) -> &DecoderConfig {
        &self.cfg
    }

    /// Cambia le impostazioni: se cambia l'IOC o il modo di avvio l'immagine in corso si butta.
    pub fn set_config(&mut self, mut cfg: DecoderConfig) {
        // l'IOC riconosciuto dal tono d'inizio resta (e l'immagine in corso con lui) finche' nelle impostazioni non cambia l'IOC
        let ioc_chosen = cfg.ioc != self.base_ioc;
        self.base_ioc = cfg.ioc;
        if cfg.auto_ioc && !ioc_chosen {
            cfg.ioc = self.cfg.ioc;
        }
        let restart = cfg.ioc != self.cfg.ioc || cfg.manual != self.cfg.manual;
        let tuning_changed = cfg.center_hz != self.cfg.center_hz || cfg.shift_hz != self.cfg.shift_hz;
        let demod_changed = cfg.filter != self.cfg.filter || tuning_changed;
        // correzione a mano cambiata con un'immagine in corso: vale subito (per le righe nuove col periodo, per quelle scritte raddrizzandole)
        let slant_delta = f64::from(cfg.slant_ppm) - f64::from(self.cfg.slant_ppm);
        // sottoportante cambiata a mano o AFC spento: la correzione ricomincia da zero
        if tuning_changed || !cfg.afc {
            self.afc_hz = 0.0;
            self.afc.reset();
        }
        self.cfg = cfg;
        self.width = self.cfg.width() as usize;
        if demod_changed {
            self.demod = FmDemod::new(self.cfg.filter, self.rx_center(), self.cfg.shift_hz);
        } else {
            self.demod.set_center(self.rx_center());
        }
        self.meter.set_band(self.rx_center(), self.cfg.shift_hz);
        if restart {
            self.cancel();
            self.measured_len = None;
            self.corr_buf.clear();
        }
        // il periodo di riga puo' cambiare a immagine in corso (correzione a mano): la posizione di riga resta continua
        self.rebase();
        self.lpm_set();
        if !restart && slant_delta != 0.0 && self.image.is_some() {
            let applied = slant_delta * 1e-6 * self.width as f64;
            self.straighten_written(applied, self.slant_base_row, 0.0);
        }
    }

    /// Errore di orologio ricordato dalle immagini precedenti (ppm): si aggiunge al periodo nominale delle immagini senza fasatura (manuale,
    /// «parti subito», squelch) ed e' la media pesata delle stime valide dell'inclinazione automatica, limitata a +-5000 ppm.
    pub fn learned_ppm(&self) -> f32 {
        self.learned_ppm as f32
    }

    /// Peso accumulato del valore ricordato (0: niente di ricordato).
    pub fn learned_weight(&self) -> f32 {
        self.learned_weight as f32
    }

    /// Dimentica l'errore di orologio ricordato (si cambia scheda audio o frequenza di campionamento).
    pub fn reset_learned(&mut self) {
        self.learned_ppm = 0.0;
        self.learned_weight = 0.0;
        self.rebase();
        self.lpm_set();
    }

    /// Fissa la posizione di riga del campione in corso come nuova base: da qui il periodo di riga puo' cambiare senza salti.
    fn rebase(&mut self) {
        if self.image.is_some() {
            self.slant_base_row = self.row_pos();
            self.slant_base_sample = self.img_sample;
        }
    }

    /// Correzione che non viene dalla stima dell'immagine in corso: quella a mano, e l'errore ricordato se la fasatura non ha misurato il
    /// periodo (altrimenti l'errore di orologio e' gia' dentro la misura).
    fn fixed_ppm(&self) -> f64 {
        f64::from(self.cfg.slant_ppm) + if self.measured_len.is_none() { self.learned_ppm } else { 0.0 }
    }

    /// A fine immagine: se l'inclinazione automatica ha avuto stime valide su abbastanza righe, l'errore totale visto (quello gia' applicato
    /// piu' il residuo corretto sull'immagine) entra nella media pesata dell'errore ricordato.
    fn learn_from_image(&mut self) {
        if !self.cfg.auto_slant || self.slant_valid_rows < LEARN_MIN_ROWS {
            return;
        }
        let base = match self.measured_len {
            None => self.learned_ppm,
            Some(len) if self.measured_nominal > 0.0 => (len / self.measured_nominal - 1.0) * 1e6,
            Some(_) => return,
        };
        let sample = (base + self.auto_ppm).clamp(-LEARN_MAX_PPM, LEARN_MAX_PPM);
        let w = (self.slant_valid_rows as f64 / LEARN_FULL_ROWS).min(1.0);
        self.learned_ppm = ((self.learned_ppm * self.learned_weight + sample * w) / (self.learned_weight + w)).clamp(-LEARN_MAX_PPM, LEARN_MAX_PPM);
        self.learned_weight = (self.learned_weight + w).min(LEARN_MAX_WEIGHT);
    }

    pub fn is_receiving(&self) -> bool {
        self.image.is_some()
    }

    /// `apt` (in attesa del tono di partenza), `phasing`, `image`.
    pub fn rx_state(&self) -> &'static str {
        match self.state {
            Fs::Phasing => "phasing",
            Fs::Image => "image",
            _ => "apt",
        }
    }

    /// Media delle ultime righe della correlazione tra righe consecutive (0..1).
    pub fn correlation(&self) -> f32 {
        self.corr_avg as f32
    }

    pub fn level_db(&self) -> f32 {
        20.0 * self.level.max(1e-6).log10()
    }

    pub fn lpm(&self) -> f32 {
        self.lpm_img as f32
    }

    /// Livello del segnale per lo squelch (0..100: 0 solo rumore, 100 almeno 20 dB sopra il rumore).
    pub fn signal_level(&self) -> f32 {
        self.meter.level()
    }

    /// Correzione dell'AFC in Hz sopra la sottoportante delle impostazioni.
    pub fn afc_hz(&self) -> f32 {
        self.afc_hz
    }

    /// Sottoportante su cui si riceve: quella delle impostazioni piu' la correzione dell'AFC.
    fn rx_center(&self) -> f32 {
        self.cfg.center_hz + self.afc_hz
    }

    /// L'immagine in arrivo: larghezza, righe complete e i loro pixel.
    pub fn current(&self) -> Option<(u32, u32, &[u8])> {
        let img = self.image.as_ref()?;
        let rows = self.complete_rows(img);
        Some((self.width as u32, rows, &img.data[..rows as usize * self.width]))
    }

    /// Butta l'immagine in arrivo e torna ad aspettare APT start. Gli eventi (lo scarto) escono al prossimo `push` o `finish`.
    pub fn cancel(&mut self) {
        if self.image.as_ref().is_some_and(|i| i.announced) {
            self.events.push(DecoderEvent::Discarded { reason: "cancelled".into() });
        }
        self.end_rx();
        self.apt_count = 0;
        self.apt_trans = 0;
        self.apt_win.clear();
        self.alt_hit = [false; 3];
    }

    /// Eventi in sospeso (per esempio lo scarto di `cancel`).
    pub fn take_events(&mut self) -> Vec<DecoderEvent> {
        std::mem::take(&mut self.events)
    }

    /// Comincia subito un'immagine dal punto in cui si e' (APT e fasatura saltati): la riga non e' allineata.
    pub fn force_start(&mut self) {
        if self.image.is_some() {
            self.cancel();
        }
        self.pending_source = Source::Forced;
        self.skip_apt_rx();
        self.skip_phasing_to_image();
        self.reset_counters();
    }

    /// Chiude e salva l'immagine in corso com'e' (senza i controlli di qualita'), se ha almeno qualche riga.
    pub fn finish(&mut self) -> Vec<DecoderEvent> {
        self.flush_rows();
        let rows = self.image.as_ref().map_or(0, |i| i.rows_written);
        if rows >= 2 {
            self.announce();
            self.learn_from_image();
            let done = self.build_done("manual", true, rows);
            self.events.push(DecoderEvent::Finished(done));
        } else if self.image.as_ref().is_some_and(|i| i.announced) {
            self.events.push(DecoderEvent::Discarded { reason: "too_small".into() });
        }
        self.end_rx();
        std::mem::take(&mut self.events)
    }

    /// Audio a 12 kHz (valori non finiti ignorati).
    pub fn push(&mut self, audio: &[f32]) -> Vec<DecoderEvent> {
        if audio.is_empty() {
            return self.take_events();
        }
        let mut buf = std::mem::take(&mut self.demod_buf);
        let mut freq = std::mem::take(&mut self.freq_buf);
        buf.clear();
        freq.clear();
        self.demod.process_freq(audio, &mut buf, self.cfg.afc.then_some(&mut freq));
        let power = audio.iter().filter(|v| v.is_finite()).map(|&v| v * v).sum::<f32>() / audio.len() as f32;
        self.level = 0.8 * self.level + 0.2 * power.sqrt();
        for (i, &v) in buf.iter().enumerate() {
            let g = f64::from(v);
            self.spread_sum += g;
            self.spread_sq += g * g;
            self.spread_n += 1;
            if let Some(level) = self.meter.push(audio[i]) {
                let n = f64::from(self.spread_n.max(1));
                let mean = self.spread_sum / n;
                let spread = (self.spread_sq / n - mean * mean).max(0.0).sqrt() as f32;
                (self.spread_sum, self.spread_sq, self.spread_n) = (0.0, 0.0, 0);
                self.on_level(level, spread);
            }
            if let Some(&f) = freq.get(i)
                && let Some(err) = self.afc.push(f, self.cfg.shift_hz, self.state != Fs::Image && !self.after_end())
            {
                self.on_afc(err);
            }
            self.step(i32::from(v));
        }
        self.demod_buf = buf;
        self.freq_buf = freq;
        self.flush_rows();
        let st = self.rx_state();
        if self.reported != Some(st) {
            self.reported = Some(st);
            self.events.push(DecoderEvent::State(st));
        }
        std::mem::take(&mut self.events)
    }

    // ---------------------------------------------------------------- squelch e AFC

    /// Una misura nuova del livello (ogni 1024 campioni) con la deviazione dei grigi demodulati: con lo squelch acceso fa partire o chiude
    /// l'immagine. Un tono fisso (deviazione bassa) tiene aperto lo squelch ma non fa partire un'immagine.
    fn on_level(&mut self, level: f32, spread: f32) {
        if !self.cfg.squelch || self.cfg.manual {
            return;
        }
        if level >= self.cfg.squelch_level {
            self.sql_open = if spread >= SQL_MIN_SPREAD { self.sql_open + 1 } else { 0 };
            self.sql_closed = 0;
        } else {
            self.sql_closed += 1;
            self.sql_open = 0;
            if self.sql_closed >= SQL_REARM_BLOCKS {
                self.sql_armed = true;
            }
        }
        // anche in attesa della fasatura senza righe di fasatura viste (una portante fissa puo' portare li')
        let waiting = self.state == Fs::AptStart || (self.state == Fs::Phasing && self.phase_lines == 0);
        if self.image.is_none() && waiting && self.sql_armed && self.sql_open >= SQL_OPEN_BLOCKS {
            // come «parti subito», ma l'immagine si annuncia solo con qualche riga: se e' un tono APT start la ricezione la chiude
            // da sola e passa alla fasatura
            self.sql_armed = false;
            self.pending_source = Source::Squelch;
            self.skip_apt_rx();
            self.skip_phasing_to_image();
            self.reset_counters();
        } else if self.image.is_some() && self.sql_closed >= SQL_CLOSE_BLOCKS {
            self.flush_rows();
            self.save_automatic("squelch");
        }
    }

    /// Errore di sintonia misurato dall'AFC: la sottoportante del demodulatore lo segue (solo con un segnale vero). Due limiti: ogni passo non
    /// supera `AFC_STEP_MAX_HZ` (locale) e la correzione totale non esce da `afc_range_hz` dall'ultima sottoportante scelta (globale).
    /// Sul tono d'inizio e sulla fasatura si corregge con piu' decisione che dentro l'immagine (modo manuale compreso: li' non c'e' altro).
    fn on_afc(&mut self, err_hz: f32) {
        let (gain, step_max, dead) = if self.state == Fs::Image { (AFC_IMAGE_GAIN, AFC_IMAGE_STEP_MAX_HZ, AFC_IMAGE_DEADBAND_HZ) } else { (AFC_GAIN, AFC_STEP_MAX_HZ, AFC_DEADBAND_HZ) };
        if !self.cfg.afc || self.meter.level() < AFC_MIN_LEVEL || err_hz.abs() < dead {
            return;
        }
        let range = self.cfg.afc_range_hz.clamp(AFC_RANGE_MIN_HZ, AFC_RANGE_MAX_HZ);
        let step = (gain * err_hz).clamp(-step_max, step_max);
        self.afc_hz = (self.afc_hz + step).clamp(-range, range);
        self.demod.set_center(self.rx_center());
        self.meter.set_band(self.rx_center(), self.cfg.shift_hz);
    }

    // ---------------------------------------------------------------- un campione

    fn after_end(&self) -> bool {
        self.since_end < AFTER_END_HOLD
    }

    fn step(&mut self, x: i32) {
        self.since_end = self.since_end.saturating_add(1);
        self.sig_refresh();
        self.current_value = x;
        self.correlation_update(x as u8);
        if self.cfg.manual {
            self.state = Fs::Image;
            if self.decode_image(x) {
                // massimo di righe: l'immagine e' salvata, se ne comincia un'altra
                self.skip_apt_rx();
                self.skip_phasing_to_image();
                self.reset_counters();
            }
        } else {
            self.decode_apt(x);
            if self.state == Fs::Phasing && self.phase_lines == 0 && self.img_sample > PHASING_WAIT_S * i64::from(SR) {
                // un falso tono d'inizio: la fasatura non e' mai arrivata, si torna ad aspettare il tono
                self.state = Fs::AptStart;
                self.apt_count = 0;
                self.apt_trans = 0;
                self.img_sample = 0;
            }
            // anche aspettando il tono d'inizio: certe stazioni non lo mandano e cominciano con la sola fasatura
            if self.state == Fs::Phasing || self.state == Fs::Image || self.state == Fs::AptStart {
                self.decode_phasing(x);
            }
            if self.state == Fs::Image && self.lpm_img > 0.0 {
                self.decode_image(x);
            }
        }
        self.img_sample += 1;
    }

    /// Lunghezza della riga (misurata sulla fasatura se c'e', altrimenti quella delle righe al minuto scelte) con la correzione a mano.
    fn lpm_set(&mut self) {
        let nominal = lpm_to_samples(self.cfg.default_lpm()) as f64;
        self.set_line_len(self.measured_len.unwrap_or(nominal));
    }

    fn set_line_len(&mut self, base: f64) {
        self.smpl_per_lin = base * (1.0 + (self.fixed_ppm() + self.auto_ppm) * 1e-6);
        self.lpm_img = 60.0 * f64::from(SR) / self.smpl_per_lin;
    }

    // ---------------------------------------------------------------- APT

    /// Si contano le transizioni nero-bianco: dopo mezzo secondo la frequenza dice se e' APT start (300 o 675 Hz) o stop (450 Hz).
    fn decode_apt(&mut self, x: i32) {
        if x > 215 && !self.apt_high {
            self.apt_high = true;
            self.apt_trans += 1;
        } else if x < 40 && self.apt_high {
            self.apt_high = false;
        }
        self.apt_count += 1;
        // il tono d'inizio dell'altro IOC (con il filtro stretto i 675 Hz non superano le soglie fisse del conteggio): si tiene la finestra
        // e si misura quanta potenza cade su quel tono
        if self.cfg.auto_ioc && self.state == Fs::AptStart && self.image.is_none() {
            self.apt_win.push(x.clamp(0, 255) as u8);
        } else if !self.apt_win.is_empty() {
            self.apt_win.clear();
            self.alt_hit = [false; 3];
        }
        if self.apt_count < SR / 2 {
            return;
        }
        if !self.apt_win.is_empty() {
            let other = f64::from(if self.cfg.apt_start_hz() == 300 { 675 } else { 300 });
            let share = tone_share(&self.apt_win, other);
            self.alt_hit = [share >= TONE_SHARE_MIN, self.alt_hit[0], self.alt_hit[1]];
            self.apt_win.clear();
        }
        self.cr3_freq = self.cr2_freq;
        self.cr2_freq = self.cr1_freq;
        self.cr1_freq = self.curr_freq;
        self.curr_freq = SR * self.apt_trans / self.apt_count;
        self.apt_count = 0;
        self.apt_trans = 0;
        let start = self.cfg.apt_start_hz();
        let (c0, c1, c2, c3) = (self.curr_freq, self.cr1_freq, self.cr2_freq, self.cr3_freq);

        if self.state == Fs::AptStart {
            if near(c0, start, 8) && near(c1, start, 8) {
                self.skip_apt_rx();
                return;
            }
            // il tono d'inizio dell'altro IOC: si cambia IOC prima che cominci la fasatura (mai a immagine cominciata)
            if self.cfg.auto_ioc && self.image.is_none() && self.alt_hit.iter().all(|&h| h) {
                self.switch_ioc();
                self.skip_apt_rx();
                return;
            }
            if near(c0, APT_STOP_HZ, 2) && near(c1, APT_STOP_HZ, 2) {
                // un falso stop mentre si aspetta la partenza
                return;
            }
        }
        // APT start nel mezzo di un'immagine: l'APT stop precedente e' sfuggito
        if self.state == Fs::Image {
            let msg = if near(c0, start, 4) && near(c1, start, 4) && near(c2, start, 4) {
                Some("apt")
            } else if near(c0, start, 5) && near(c1, start, 5) && near(c2, start, 5) && near(c3, start, 5) {
                Some("apt2")
            } else {
                None
            };
            if let Some(m) = msg {
                self.save_automatic(m);
                self.skip_apt_rx();
                return;
            }
        }
        let msg_stop = if near(c0, APT_STOP_HZ, 6) && near(c1, APT_STOP_HZ, 6) {
            Some("ok")
        } else if near(c0, APT_STOP_HZ, 7) && near(c1, APT_STOP_HZ, 7) && near(c2, APT_STOP_HZ, 7) {
            Some("ok2")
        } else {
            None
        };
        if let Some(m) = msg_stop {
            self.save_automatic(m);
            return;
        }
        match (self.sig_state, self.state) {
            (Fs::Image, Fs::AptStart) if !self.after_end() => self.skip_apt_rx(),
            (Fs::AptStop, Fs::Image) => {
                let code = self.stop_code;
                self.save_automatic(code);
            }
            _ => {}
        }
    }

    /// Passa all'altro IOC (576 / 288) in attesa della fasatura: larghezza e righe al minuto di partenza seguono. Resta anche dopo
    /// l'immagine (la trasmissione dopo si riconosce da sola dal suo tono d'inizio); le impostazioni salvate non cambiano.
    fn switch_ioc(&mut self) {
        let from = self.cfg.ioc;
        self.cfg.ioc = if from == 288 { 576 } else { 288 };
        self.width = self.cfg.width() as usize;
        self.measured_len = None;
        self.corr_buf.clear();
        self.alt_hit = [false; 3];
        self.lpm_set();
        let (ioc, width) = (self.cfg.ioc, self.cfg.width());
        let lpm = 60.0 * f64::from(SR) / self.smpl_per_lin;
        self.events.push(DecoderEvent::IocDetected { from, ioc, lpm: lpm as f32, width });
    }

    /// Esce dall'attesa del tono di partenza e aspetta la fasatura.
    fn skip_apt_rx(&mut self) {
        self.measured_len = None;
        self.lpm_set();
        self.state = Fs::Phasing;
        self.reset_phasing_counters();
        self.img_sample = 0;
        self.corr_max = 0.0;
        self.stats = [0; 256];
    }

    fn reset_phasing_counters(&mut self) {
        self.phase_len_sum = 0;
        self.phase_high = self.current_value >= 128;
        self.curr_phase_len = 0;
        self.curr_phase_high = 0;
        self.curr_phase_low = 0;
        self.phase_lines = 0;
        self.num_phase_lines = 0;
    }

    // ---------------------------------------------------------------- fasatura

    /// Le righe di fasatura sono 2,5% bianco, 95% nero, 2,5% bianco: dal tempo tra due passaggi bianco-nero si ricavano il periodo della
    /// riga e il suo inizio. Dopo 4 righe buone l'immagine comincia con la riga allineata.
    fn decode_phasing(&mut self, x: i32) {
        // il nero e' largo: l'ingresso si media su 16 campioni contro i disturbi
        let mut x = x;
        self.phasing_hist[self.phasing_count % 16] = x;
        self.phasing_count += 1;
        if self.phasing_count >= 16 {
            x = self.phasing_hist.iter().sum::<i32>() / 16;
        }
        self.curr_phase_len += 1;
        if x > 188 {
            self.curr_phase_high += 1;
        } else if x < 68 {
            self.curr_phase_low += 1;
        }
        if x > 200 && !self.phase_high {
            self.phase_high = true;
        } else if x < 25 && self.phase_high {
            self.phase_high = false;
            let len = f64::from(self.curr_phase_len);
            let black_min = if self.state == Fs::Image { PHASE_BLACK_MIN_IMAGE } else { PHASE_BLACK_MIN };
            // aspettando il tono d'inizio (nessun APT visto) conta solo una riga che dura come il periodo nominale: il rumore non la imita
            let nominal = lpm_to_samples(self.cfg.default_lpm()) as f64;
            let on_time = self.state != Fs::AptStart || (len - nominal).abs() < 0.03 * nominal;
            if on_time && f64::from(self.curr_phase_high) >= 0.04 * len && f64::from(self.curr_phase_low) >= black_min * len && self.curr_phase_len >= (0.4 * f64::from(SR)) as i32 {
                self.phase_len_sum += i64::from(self.curr_phase_len);
                self.phase_lines += 1;
                self.num_phase_lines = 0;
                self.phase_edge_sample = self.img_sample;
                if self.phase_lines >= PHASING_LINES {
                    self.align_to_phasing(0);
                }
            } else if self.state == Fs::AptStart && self.phase_lines > 0 && self.img_sample - self.phase_edge_sample > (3.0 * nominal) as i64 {
                // una riga buona isolata, di molto tempo fa, senza l'immagine che la segue: era rumore, si ricomincia
                self.reset_phasing_counters();
            } else if (self.state == Fs::Phasing || self.state == Fs::AptStart) && self.phase_lines > 0 && {
                self.num_phase_lines += 1;
                self.num_phase_lines >= 3
            } {
                if self.phase_lines >= PHASING_LINES_SHORT {
                    // fasatura corta (poche righe, poi subito l'immagine): ci si allinea sull'ultima riga buona, ormai di qualche riga fa
                    let elapsed = self.img_sample - self.phase_edge_sample;
                    self.align_to_phasing(elapsed);
                } else {
                    // fasatura finita ma l'allineamento non e' stato dato
                    self.pending_source = Source::NoPhasing;
                    self.skip_phasing_to_image();
                }
            }
            self.curr_phase_len = 0;
            self.curr_phase_high = 0;
            self.curr_phase_low = 0;
        }
    }

    /// Passa all'immagine allineata sulla fasatura: periodo di riga misurato e inizio della riga. `elapsed` sono i campioni passati dall'ultimo
    /// passaggio bianco-nero buono (0 con la fasatura lunga, di piu' con quella corta, dove si e' aspettato di vedere la fine).
    fn align_to_phasing(&mut self, elapsed: i64) {
        // con poche righe il periodo misurato e' troppo incerto (un solo intervallo, qualche campione di troppo = migliaia di ppm di
        // inclinazione): si tiene il nominale e al resto pensa l'inclinazione automatica
        let measured = if self.phase_lines >= PHASING_LINES {
            self.phase_len_sum as f64 / f64::from(self.phase_lines)
        } else {
            lpm_to_samples(self.cfg.default_lpm()) as f64
        };
        self.skip_phasing_to_image_save();
        self.apply_measured(measured);
        // la riga comincia 2,5% prima della fine della banda bianca della fasatura; il passaggio bianco-nero si vede con il
        // ritardo della media su 16 campioni e dei fronti filtrati, che i pixel dell'immagine non hanno: lo si recupera
        self.img_sample = (1.025 * self.smpl_per_lin) as i64 + PHASING_LATENCY + elapsed;
        let frac = (self.img_sample as f64 % self.smpl_per_lin) / self.smpl_per_lin;
        self.last_col = (frac * self.width as f64) as i32;
        self.last_row = (self.img_sample as f64 / self.smpl_per_lin) as i64;
        self.fax_pix_num = i64::from(self.last_col) * BYTES_PER_PIXEL;
        self.pix_samples_nb = 0;
        self.pixel_val = 0;
    }

    /// Periodo di riga misurato sulla fasatura: sceglie il numero di righe al minuto piu' vicino (entro 3) e, se il periodo e' entro
    /// l'1,5% di quello nominale, lo usa al posto del nominale: e' la correzione automatica dell'inclinazione.
    fn apply_measured(&mut self, len: f64) {
        let lpm_meas = 60.0 * f64::from(SR) / len;
        let mut nominal = lpm_to_samples(self.cfg.default_lpm()) as f64;
        if self.cfg.auto_lpm
            && let Some(std) = [240u32, 120, 90, 60].into_iter().find(|&l| (lpm_meas - f64::from(l)).abs() < 3.0)
        {
            nominal = lpm_to_samples(std) as f64;
        }
        self.measured_len = Some(len).filter(|l| (l - nominal).abs() / nominal < 0.015);
        self.measured_nominal = nominal;
        self.set_line_len(self.measured_len.unwrap_or(nominal));
    }

    fn skip_phasing_to_image_save(&mut self) {
        if self.state == Fs::Image {
            // la fasatura di un'altra immagine nel mezzo di questa: si chiude e si ricomincia
            self.save_automatic("phasing");
            self.skip_apt_rx();
        }
        self.reset_phasing_counters();
        self.pending_source = Source::Phasing;
        self.skip_phasing_to_image();
    }

    fn skip_phasing_to_image(&mut self) {
        self.state = Fs::Image;
        self.auto_ppm = 0.0;
        self.lpm_set();
    }

    // ---------------------------------------------------------------- immagine

    fn ensure_image(&mut self) {
        if self.image.is_some() {
            return;
        }
        let row_origin = self.row_pos() as i64;
        let source = if self.cfg.manual { Source::Manual } else { self.pending_source };
        self.pending_source = source;
        self.image = Some(Img { data: vec![255; INITIAL_ROWS * self.width], rows_alloc: INITIAL_ROWS, rows_written: 0, row_origin, emitted: 0, resend: false, announced: false, source });
        self.shift = 0;
        self.last_center_row = 0;
        self.stats = [0; 256];
        // le immagini volute (manuale, «parti subito») si vedono subito; quelle dopo la fasatura quando hanno qualche riga
        if matches!(source, Source::Manual | Source::Forced) {
            self.announce();
        }
    }

    fn announce(&mut self) {
        let (ioc, lpm, width) = (self.cfg.ioc, self.lpm_img as f32, self.width as u32);
        if let Some(img) = self.image.as_mut()
            && !img.announced
        {
            img.announced = true;
            self.events.push(DecoderEvent::Started { ioc, lpm, width, source: img.source });
        }
    }

    /// Un campione dell'immagine: i campioni della stessa colonna si mediano in un pixel. `true` se si e' raggiunto il massimo di righe.
    fn decode_image(&mut self, x: i32) -> bool {
        self.ensure_image();
        let row_dbl = self.row_pos();
        let row = row_dbl as i64;
        let col = (self.width as f64 * (row_dbl - row as f64)) as i32;
        if col == self.last_col {
            self.pixel_val += x;
            self.pix_samples_nb += 1;
        } else {
            if self.pix_samples_nb > 0 {
                let v = (self.pixel_val / self.pix_samples_nb).clamp(0, 255);
                self.put_pixel(self.last_row, self.last_col, v as u8);
                self.stats[v as usize] += 1;
                self.fax_pix_num += BYTES_PER_PIXEL;
            }
            self.last_col = col;
            self.last_row = row;
            self.pixel_val = x;
            self.pix_samples_nb = 1;
        }
        if row >= i64::from(self.cfg.max_rows) {
            self.save_automatic("max");
            return true;
        }
        false
    }

    fn put_pixel(&mut self, row_abs: i64, col: i32, v: u8) {
        let width = self.width;
        let shift = self.shift;
        let rn;
        {
            let Some(img) = self.image.as_mut() else { return };
            let row = row_abs - img.row_origin;
            if row < 0 || col < 0 || col as usize >= width {
                return;
            }
            let row = row as usize;
            if row >= img.rows_alloc {
                let new_rows = row + ROWS_GROWTH;
                img.data.resize(new_rows * width, 255);
                img.rows_alloc = new_rows;
            }
            let c = (i64::from(col) + i64::from(shift)).rem_euclid(width as i64) as usize;
            img.data[row * width + c] = v;
            if row + 1 > img.rows_written {
                img.rows_written = row + 1;
            }
            rn = row_abs - img.row_origin + 1;
        }
        // ricentra l'immagine ogni 10 righe, dalla 30 alla 500
        if self.cfg.auto_center
            && rn != self.last_center_row
            && rn > 30
            && rn < 500
            && rn % 10 == 0
        {
            self.last_center_row = rn;
            if let Some(new_center) = self.estimate_center(rn as usize)
                && new_center.abs() > 3
            {
                self.shift_image(-new_center);
            }
        }
        // la riga in scrittura e' `rn - 1`: le righe complete sono quelle prima
        if self.cfg.auto_slant
            && rn as usize > SLANT_EVERY
            && rn as usize % SLANT_EVERY == 1
            && rn as usize <= SLANT_LAST_ROW + 1
            && rn as usize - 1 != self.last_slant_row
        {
            self.update_slant(rn as usize - 1);
        }
    }

    /// Posizione di riga (frazionaria) del campione in corso, continua anche se il periodo di riga cambia a immagine iniziata.
    fn row_pos(&self) -> f64 {
        self.slant_base_row + (self.img_sample - self.slant_base_sample) as f64 / self.smpl_per_lin
    }

    /// Misura l'inclinazione sulle righe tra l'ultima misura e `end` (escluse) e, se e' netta, corregge il periodo di riga per le righe
    /// che seguono e raddrizza quelle gia' scritte. Le righe prima dell'ultima misura sono gia' dritte: si spostano tutte insieme.
    fn update_slant(&mut self, end: usize) {
        let from = self.last_slant_row;
        self.last_slant_row = end;
        let Some(drift) = self.estimate_slant(from, end) else { return };
        self.slant_valid_rows += end - from;
        if drift.abs() < SLANT_DEADBAND_PX {
            return;
        }
        let w = self.width as f64;
        let old = self.auto_ppm;
        let new = (old + drift / w * 1e6).clamp(-SLANT_MAX_PPM, SLANT_MAX_PPM);
        if (new - old).abs() < 1e-9 {
            return;
        }
        let applied = (new - old) * 1e-6 * w;
        // il periodo cambia senza salti di posizione
        let now = self.row_pos();
        self.slant_base_row = now;
        self.slant_base_sample = self.img_sample;
        let base = self.smpl_per_lin / (1.0 + (self.fixed_ppm() + old) * 1e-6);
        self.auto_ppm = new;
        self.set_line_len(base);
        let t_prev = self.last_slant_pos;
        self.straighten_written(applied, now, t_prev);
        self.last_slant_pos = now - self.image.as_ref().map_or(0.0, |i| i.row_origin as f64);
    }

    /// Raddrizza le righe gia' scritte di una correzione di `applied` pixel per riga decisa all'istante di riga `now`: un pixel al tempo `tau`
    /// (riga + colonna / larghezza, dalla prima riga dell'immagine) deve andare a destra di (adesso - max(tau, `lower`)) * `applied`, cosi' le
    /// righe gia' scritte combaciano con quelle nuove (il periodo sbagliato inclina e anche stira la riga). `lower` e' il tempo da cui la
    /// correzione vale: la stima automatica passa l'istante dell'ultima correzione (le righe prima sono gia' dritte e si spostano insieme),
    /// la correzione a mano passa 0 (e' un errore di orologio: vale per tutte le righe gia' scritte, anche dopo molte correzioni di fila).
    fn straighten_written(&mut self, applied: f64, now: f64, lower: f64) {
        let wi = self.width;
        let shift = self.shift;
        let Some(img) = self.image.as_mut() else { return };
        let (t_now, t_prev) = (now - img.row_origin as f64, lower);
        let rows = ((t_now.floor().max(0.0)) as usize + 1).min(img.rows_alloc);
        let sh_at = |r: usize, c: usize| -> f64 {
            let col = (c as i64 - i64::from(shift)).rem_euclid(wi as i64) as f64;
            (t_now - (r as f64 + col / wi as f64).max(t_prev)) * applied
        };
        for r in 0..rows {
            let old: Vec<u8> = img.data[r * wi..(r + 1) * wi].to_vec();
            let row = &mut img.data[r * wi..(r + 1) * wi];
            for (c, px) in row.iter_mut().enumerate() {
                // sorgente: dove il pixel era prima dello spostamento (due passi per tener conto della variazione lungo la riga)
                let mut src = c as f64 - sh_at(r, c);
                src = c as f64 - sh_at(r, src.rem_euclid(wi as f64) as usize);
                let i0 = src.floor();
                let f = (src - i0) as f32;
                let i0 = (i0 as i64).rem_euclid(wi as i64) as usize;
                let (v0, v1) = (f32::from(old[i0]), f32::from(old[(i0 + 1) % wi]));
                *px = (v0 * (1.0 - f) + v1 * f).round() as u8;
            }
        }
        img.resend = true;
    }

    /// Deriva orizzontale in pixel per riga (positivo: le strutture vanno a destra scendendo) dalle righe `from..to`: mediana degli
    /// spostamenti che rendono massima la correlazione tra righe distanti `SLANT_GAP`. `None` se le righe non hanno strutture nette.
    /// Due passi: prima righe vicine (`SLANT_COARSE_GAP`) su tutto l'intervallo di spostamenti, che con poche righe di distanza resta
    /// piu' stretto del passo di una griglia regolare (meridiani) e non la scambia con un'altra maglia; poi righe distanti `SLANT_GAP`,
    /// cercando solo vicino alla stima grossolana, per la precisione.
    fn estimate_slant(&self, from: usize, to: usize) -> Option<f64> {
        let w = self.width as f64;
        let max_c = (SLANT_COARSE_GAP as f64 * SLANT_MAX_PPM * 1e-6 * w).ceil() as i64 + 3;
        let coarse = self.estimate_slant_gap(from, to, SLANT_COARSE_GAP, -max_c, max_c)?;
        let c = (coarse * SLANT_GAP as f64).round() as i64;
        self.estimate_slant_gap(from, to, SLANT_GAP, c - SLANT_FINE_RANGE_PX, c + SLANT_FINE_RANGE_PX)
    }

    /// Mediana della deriva (pixel per riga) tra righe distanti `gap`, cercando gli spostamenti `lo..=hi` (pixel su `gap` righe).
    fn estimate_slant_gap(&self, from: usize, to: usize, gap: usize, lo: i64, hi: i64) -> Option<f64> {
        let img = self.image.as_ref()?;
        let w = self.width;
        if to <= from + gap + SLANT_MIN_PAIRS || to * w > img.data.len() || hi < lo {
            return None;
        }
        // righe senza media e normalizzate; `None` se quasi uniformi
        let rows: Vec<Option<Vec<f32>>> = (from..to)
            .map(|r| {
                let src = &img.data[r * w..(r + 1) * w];
                let mean = src.iter().map(|&v| f32::from(v)).sum::<f32>() / w as f32;
                let var = src.iter().map(|&v| (f32::from(v) - mean).powi(2)).sum::<f32>() / w as f32;
                let std = var.sqrt();
                (std >= SLANT_MIN_STD).then(|| src.iter().map(|&v| (f32::from(v) - mean) / (std * (w as f32).sqrt())).collect())
            })
            .collect();
        let mut shifts: Vec<f64> = Vec::new();
        for i in 0..rows.len().saturating_sub(gap) {
            let (Some(a), Some(b)) = (&rows[i], &rows[i + gap]) else { continue };
            let mut ncc = Vec::with_capacity((hi - lo + 1) as usize);
            for s in lo..=hi {
                // a[c] contro b[(c + s) % w]
                let k = s.rem_euclid(w as i64) as usize;
                let head = w - k;
                let sum: f32 = a[..head].iter().zip(&b[k..]).map(|(x, y)| x * y).sum::<f32>() + a[head..].iter().zip(&b[..k]).map(|(x, y)| x * y).sum::<f32>();
                ncc.push(sum);
            }
            let Some((bi, &best)) = ncc.iter().enumerate().max_by(|x, y| x.1.total_cmp(y.1)) else { continue };
            if best < SLANT_MIN_NCC {
                continue;
            }
            let second = ncc.iter().enumerate().filter(|(j, _)| j.abs_diff(bi) > 4).map(|(_, &v)| v).fold(f32::MIN, f32::max);
            if second > SLANT_PEAK_RATIO * best {
                continue;
            }
            // interpolazione parabolica del picco
            let frac = if bi > 0 && bi + 1 < ncc.len() {
                let (l, c, r) = (f64::from(ncc[bi - 1]), f64::from(ncc[bi]), f64::from(ncc[bi + 1]));
                let den = l - 2.0 * c + r;
                if den.abs() > 1e-12 { (0.5 * (l - r) / den).clamp(-0.5, 0.5) } else { 0.0 }
            } else {
                0.0
            };
            // un picco sul bordo dell'intervallo non e' un massimo vero
            if bi == 0 || bi + 1 == ncc.len() {
                continue;
            }
            shifts.push((bi as f64 + lo as f64 + frac) / gap as f64);
        }
        if shifts.len() < SLANT_MIN_PAIRS {
            return None;
        }
        shifts.sort_by(f64::total_cmp);
        let median = shifts[shifts.len() / 2];
        let mut dev: Vec<f64> = shifts.iter().map(|v| (v - median).abs()).collect();
        dev.sort_by(f64::total_cmp);
        (dev[dev.len() / 2] <= SLANT_MAX_SPREAD_PX).then_some(median)
    }

    /// Dove sta il centro della fascia scura piu' larga (60 colonne) sulle ultime 10 righe complete: stima del margine, come fldigi.
    fn estimate_center(&self, row_number: usize) -> Option<i32> {
        let img = self.image.as_ref()?;
        let numrows = 10usize;
        let end = row_number.checked_sub(1)?;
        if end < numrows {
            return None;
        }
        let w = self.width;
        let mut avgs = vec![0i32; w];
        for (col, a) in avgs.iter_mut().enumerate() {
            let mut sum = 0i32;
            for row in end - numrows..end {
                sum += i32::from(img.data[row * w + col]);
            }
            *a = sum / numrows as i32;
        }
        let mut min_idx = 0usize;
        let mut min_val = 255i32;
        for col in 0..w {
            let mut s = 0i32;
            for n in 0..CENTER_BAND {
                s += avgs[(col + n) % w];
            }
            s /= CENTER_BAND as i32;
            if s < min_val {
                min_val = s;
                min_idx = col;
            }
        }
        if min_val > 40 {
            return None;
        }
        let mut idx = min_idx as i32;
        if idx > w as i32 / 2 {
            idx -= w as i32;
        }
        Some(idx + CENTER_BAND as i32 / 2)
    }

    /// Porta la colonna `x` all'inizio di ogni riga dell'immagine in corso e di quelle che arriveranno (a mano, «Imposta inizio riga»).
    pub fn set_line_start(&mut self, x: u32) {
        let w = self.width.max(1) as u32;
        let x = (x % w) as i32;
        if x != 0 {
            self.shift_image(-x);
        }
    }

    /// Sposta tutta l'immagine a destra di `delta` colonne (a capo), e anche i pixel che arriveranno.
    fn shift_image(&mut self, delta: i32) {
        let w = self.width;
        self.shift += delta;
        let d = delta.rem_euclid(w as i32) as usize;
        if let Some(img) = self.image.as_mut() {
            for r in 0..img.rows_alloc {
                img.data[r * w..(r + 1) * w].rotate_right(d);
            }
            img.resend = true;
        }
    }

    fn complete_rows(&self, img: &Img) -> u32 {
        let cur = self.row_pos() as i64 - img.row_origin;
        (cur.max(0) as usize).min(img.rows_written) as u32
    }

    /// Manda all'interfaccia le righe complete nuove (tutte, se l'immagine e' stata spostata).
    fn flush_rows(&mut self) {
        let Some(img) = self.image.as_ref() else { return };
        let complete = self.complete_rows(img);
        let (resend, emitted, announced) = (img.resend, img.emitted, img.announced);
        if !announced {
            if complete < ANNOUNCE_ROWS {
                return;
            }
            self.announce();
        }
        if resend && complete > 0 {
            self.events.push(DecoderEvent::Rows { first: 0, count: complete });
        } else if complete > emitted {
            self.events.push(DecoderEvent::Rows { first: emitted, count: complete - emitted });
        } else {
            return;
        }
        if let Some(img) = self.image.as_mut() {
            img.emitted = complete;
            img.resend = false;
        }
    }

    // ---------------------------------------------------------------- fine dell'immagine

    fn reset_counters(&mut self) {
        self.slant_valid_rows = 0;
        self.slant_base_row = 0.0;
        self.slant_base_sample = 0;
        self.last_slant_row = 0;
        self.last_slant_pos = 0.0;
        self.last_col = 0;
        self.last_row = 0;
        self.fax_pix_num = 0;
        self.img_sample = 0;
        self.pix_samples_nb = 0;
        self.pixel_val = 0;
    }

    fn end_rx(&mut self) {
        self.image = None;
        self.auto_ppm = 0.0;
        self.lpm_set();
        // lo squelch riparte solo dopo che il segnale e' sparito (il tono di stop o la portante dopo l'immagine non ne aprono un'altra)
        self.sql_armed = false;
        self.sql_open = 0;
        self.state = if self.cfg.manual { Fs::Image } else { Fs::AptStart };
        self.reset_counters();
    }

    fn build_done(&mut self, extra: &str, complete: bool, rows_written: usize) -> Done {
        let w = self.width;
        let mut rows = rows_written;
        // il tono di stop (circa 1,25 s) occupa le ultime righe
        if matches!(extra, "ok" | "ok2") {
            let trim = ((1.25 * self.lpm_img / 60.0) as usize).max(1);
            rows = rows.saturating_sub(trim).max(1);
        }
        let data = self.image.as_ref().map(|i| i.data[..rows * w].to_vec()).unwrap_or_default();
        Done { image: GrayImage { width: w as u32, height: rows as u32, data }, ioc: self.cfg.ioc, lpm: self.lpm_img as f32, reason: reason_of(extra).into(), complete, correlation: self.corr_max as f32, source: self.pending_source }
    }

    /// Chiude l'immagine in corso (APT stop, segnale perso, APT start di un'altra, massimo di righe) tenendola solo se ha l'aspetto di una
    /// cartina: i controlli sono quelli di fldigi contro le immagini bianche, scure o con correlazione bassa.
    fn save_automatic(&mut self, extra: &str) {
        let hist = self.stats;
        let sum: i64 = hist.iter().map(|&h| i64::from(h)).sum();
        let sum_vals: i64 = hist.iter().enumerate().map(|(i, &h)| i as i64 * i64::from(h)).sum();
        let (avg, stddev) = if sum == 0 {
            (0.0, 0.0)
        } else {
            let avg = (sum_vals / sum) as f64;
            let var: f64 = hist.iter().enumerate().map(|(i, &h)| (i as f64 - avg).powi(2) * f64::from(h)).sum::<f64>() / sum as f64;
            (avg, var.sqrt())
        };
        let current_row = (self.img_sample as f64 / self.smpl_per_lin) as i64;
        let cmax = self.corr_max;
        let verdict: Option<&'static str> = if self.fax_pix_num < i64::from(self.cfg.min_pixels) * BYTES_PER_PIXEL {
            Some("too_small")
        } else if extra.starts_with("garbage") {
            Some("garbage")
        } else if cmax < 0.20 {
            Some("low_correlation")
        } else if (avg > 220.0 && cmax < 0.30 && stddev < 30.0) || (avg > 240.0 && cmax < 0.35 && stddev < 30.0) || (avg > 230.0 && cmax < 0.25 && stddev < 35.0) || (avg > 220.0 && stddev < 20.0) {
            Some("blank")
        } else if (avg < 100.0 && cmax < 0.30) || (avg < 80.0 && cmax < 0.35) {
            Some("dark")
        } else if (avg > 235.0 && cmax < 0.65 && stddev < 40.0 && current_row < 100) || (avg < 130.0 && cmax < 0.70 && stddev < 100.0 && current_row < 100) {
            Some("small_blank")
        } else {
            None
        };
        if self.image.is_some() {
            match verdict {
                None => {
                    let rows = self.image.as_ref().map_or(0, |i| i.rows_written);
                    let complete = matches!(extra, "ok" | "ok2" | "stop");
                    self.announce();
                    self.learn_from_image();
                    let done = self.build_done(extra, complete, rows);
                    self.events.push(DecoderEvent::Finished(done));
                }
                Some(why) => {
                    if self.image.as_ref().is_some_and(|i| i.announced) {
                        self.events.push(DecoderEvent::Discarded { reason: why.into() });
                    }
                }
            }
        }
        self.end_rx();
        // chiusa dal tono di fine: da qui lo strascico del tono non conta (con gli altri motivi, per esempio la fasatura mancata, no)
        if matches!(extra, "ok" | "ok2" | "stop") {
            self.since_end = 0;
        }
    }

    // ---------------------------------------------------------------- correlazione tra righe

    fn sig_refresh(&mut self) {
        if self.sig_cnt % SIGNAL_VALIDITY == 0 {
            self.sig_state = Fs::Idle;
            self.stop_code = "";
            // le righe consecutive di un fax sono molto correlate; senza segnale la correlazione cade
            self.sig_state = match self.correlation_state() {
                Fs::AptStop => Fs::AptStop,
                Fs::Image => Fs::Image,
                _ => Fs::Idle,
            };
        }
        self.sig_cnt = self.sig_cnt.wrapping_add(1);
    }

    fn corr_at(&self, i: i64) -> i64 {
        i64::from(self.corr_buf[(i as usize) % self.corr_buf.len()])
    }

    /// Correlazione (valore assoluto) tra la riga che parte da `img_sample` e quella `line_offset` campioni dopo.
    fn correlation_from_index(&self, line_length: usize, line_offset: usize) -> f64 {
        if self.corr_buf.is_empty() || line_length == 0 {
            return 0.0;
        }
        let start = self.img_sample;
        let end = start + line_length as i64;
        let off = line_offset as i64;
        let (mut avg_pred, mut avg_curr) = (0i64, 0i64);
        for i in start..end {
            avg_pred += self.corr_at(i);
            avg_curr += self.corr_at(i + off);
        }
        avg_pred /= line_length as i64;
        avg_curr /= line_length as i64;
        let (mut num, mut dp, mut dc) = (0i64, 0i64, 0i64);
        for i in start..end {
            let p = self.corr_at(i) - avg_pred;
            let c = self.corr_at(i + off) - avg_curr;
            num += p * c;
            dp += p * p;
            dc += c * c;
        }
        let den = ((dp as f64) * (dc as f64)).sqrt();
        if den == 0.0 { 0.0 } else { (num as f64 / den).abs() }
    }

    fn correlation_calc(&mut self) {
        self.corr_calls += 1;
        let l = lpm_to_samples(self.cfg.default_lpm()) as usize;
        let current = self.correlation_from_index(l, l).min(1.0);
        let crows = self.cfg.correlation_rows.max(2);
        if self.corr_calls < crows {
            self.corr_avg = current;
            self.corr_max = 0.0;
        } else {
            self.corr_avg = (self.corr_avg * f64::from(crows) + current) / f64::from(crows + 1);
            self.corr_max = self.corr_max.max(self.corr_avg);
        }
    }

    fn correlation_update(&mut self, sample: u8) {
        let l = lpm_to_samples(self.cfg.default_lpm()) as usize;
        let size = 2 * l;
        if self.corr_buf.len() != size {
            self.corr_buf = vec![0; size];
            self.corr_avg = 0.0;
        }
        if self.cnt_upd % l as u64 == 0 {
            self.correlation_calc();
        }
        self.cnt_upd += 1;
        let n = self.corr_buf.len();
        self.corr_buf[(self.img_sample as usize) % n] = sample;
    }

    /// Stato stabile dedotto dalla correlazione media: immagine, fine (segnale perso o immagine senza senso) o incerto (`Idle`).
    fn correlation_state(&mut self) -> Fs {
        self.stop_code = match self.stable_state {
            Fs::AptStart => "stable_start",
            Fs::AptStop => "stable_stop",
            Fs::Phasing => "stable_phasing",
            Fs::Image => "stable_image",
            Fs::Idle => "stable_idle",
        };
        if self.corr_calls >= self.cfg.correlation_rows {
            let crr_row = (self.img_sample as f64 / self.smpl_per_lin) as i64;
            let low = f64::from(self.cfg.correlation);
            if self.corr_avg < low {
                self.stop_code = "nocorr";
                self.stable_state = Fs::AptStop;
            } else if self.corr_avg > 1.5 * low {
                self.stable_state = Fs::Image;
            } else if self.corr_max < 0.10 && crr_row > 200 {
                // correlazione sempre bassa per molte righe: si e' cominciato un'immagine sbagliata (sintonia, portante fissa)
                self.stop_code = "garbage.200";
                self.stable_state = Fs::AptStop;
            } else if self.corr_max < 0.20 && crr_row > 500 {
                self.stop_code = "garbage.500";
                self.stable_state = Fs::AptStop;
            } else {
                self.stable_state = Fs::Idle;
            }
        }
        self.stable_state
    }
}

fn reason_of(extra: &str) -> &'static str {
    match extra {
        "ok" | "ok2" => "apt_stop",
        "apt" | "apt2" => "apt_start",
        "phasing" => "phasing",
        "max" => "max_rows",
        "manual" => "manual",
        "nocorr" => "no_signal",
        "squelch" => "squelch",
        s if s.starts_with("garbage") => "garbage",
        _ => "stop",
    }
}
