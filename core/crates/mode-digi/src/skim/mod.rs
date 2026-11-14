//! Decodifica di tutti i segnali della banda audio (CW e PSK), ognuno con il suo ricevitore completo e il suo storico.
//!
//! CW ([`CwBandDecoder`]): uno spettro (FFT di 2048 punti a 12 kHz, 5,9 Hz per riga, ogni 85 ms) cerca i toni: righe strette che stanno
//! almeno 10 dB sopra il fondo (la mediana della banda) per tre spettri di fila. Ogni tono nuovo, se non c'e' gia' un canale entro 25 Hz,
//! apre un canale: un [`CwDemodulator`] intero (filtro, AGC, AFC, decodificatore probabilistico con un fascio più piccolo) sintonizzato su
//! di lui. Un canale che non sente niente per `IDLE_S` secondi si chiude; due canali finiti sulla stessa stazione (l'AFC li ha
//! avvicinati) diventano uno. PSK: [`psk::PskBandDecoder`].
//!
//! Per ogni canale restano i caratteri degli ultimi 60 secondi con la loro probabilità (il doppio clic mostra tutto il passaggio) e i
//! nominativi sentiti, con il ruolo (CQ, DE) e la verifica per ricorrenza ([`calls`]). Gli «spot locali» sono i nominativi che passano la
//! convalida scelta: uno per nominativo ogni 10 minuti, di nuovo se cambia frequenza di oltre 1 kHz o se lo si sente chiamare CQ.

pub mod calls;
pub mod forms;
pub mod psk;
pub mod rtty;

use crate::cw::CwDemodulator;
use calls::{CallCheck, CallTracker, HeardCall, Role, Validation};
use rustfft::num_complex::Complex32;
use rustfft::{Fft, FftPlanner};
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;

const RATE: f32 = 12_000.0;
const FFT_N: usize = 2048;
const HOP: usize = 1024;
/// Quanto sopra il fondo deve stare una riga per essere un tono (dB) e per quanti spettri di fila.
const PEAK_DB: f32 = 10.0;
const STABLE_FRAMES: u32 = 3;
/// Distanza minima fra due canali (Hz).
const CHANNEL_SPACING_HZ: f32 = 25.0;
/// Un canale che non stampa niente da tanto si chiude (s).
const IDLE_S: f64 = 45.0;
/// Quanto testo si tiene per canale (s).
const HISTORY_S: f64 = 60.0;
/// Ipotesi del decodificatore probabilistico per canale (il canale scelto dall'operatore ne ha 320).
const CHANNEL_BEAM: usize = 96;
/// Ipotesi per i canali forti e la soglia (S/N in 500 Hz a tasto abbassato, dB).
const STRONG_BEAM: usize = 64;
/// Misurato (`cpu_tutta_la_banda`, `tasto_a_mano_nominativi`): a 0 dB la CPU di 30 stazioni scende da 16,0 a 13,8% di un core e i
/// nominativi a -8 e -10 dB restano 27 e 16 su 30; con 64 ipotesi per tutti a -10 dB 14, con 48 compaiono nominativi sbagliati.
const STRONG_SNR_DB: f32 = 0.0;
/// Di quanto l'AFC di un canale puo' allontanarsi dal tono su cui si e' aperto (Hz).
const CHANNEL_AFC_HZ: f32 = 20.0;
/// Un nominativo non sentito da tanto si dimentica (s).
const SPOT_REPEAT_S: f64 = 600.0;
/// Un nominativo ancora sentito si ripropone come spot dopo tanto (s); decide chi riceve gli spot.
const SPOT_REOFFER_S: f64 = 60.0;
/// Audio tenuto per i canali nuovi (s): un tono si vede dopo qualche spettro e il canale si apre in ritardo; il canale nuovo ascolta
/// prima questi secondi, cosi' l'inizio della stazione (spesso il «CQ» e il nominativo) non si perde.
const CATCH_UP_S: f32 = 2.5;
/// S/N degli spot: quanto si tengono le misure a tasto abbassato (s) e quanto sopra il fondo deve stare la riga (6 dB).
const KEYDOWN_S: f64 = 15.0;
const KEYDOWN_RATIO: f32 = 4.0;

/// Il testo di un canale: i caratteri degli ultimi 60 s con la loro probabilità e i nominativi sentiti.
#[derive(Default)]
pub(crate) struct Transcript {
    text: VecDeque<(f64, u8, u8)>,
    pub(crate) calls: CallTracker,
    pub(crate) last_text_s: f64,
    /// L'ultimo momento visto (lo aggiorna `trim`, a ogni spettro): serve a dire da quanto non si sente un nominativo.
    now_s: f64,
}

impl Transcript {
    pub(crate) fn new(now_s: f64) -> Self {
        Transcript { last_text_s: now_s, now_s, ..Transcript::default() }
    }

    /// Altri caratteri al momento `now_s`, con la loro probabilità (0-255; vuoto = certi).
    pub(crate) fn push(&mut self, out: &[u8], conf: &[u8], now_s: f64) {
        if out.is_empty() {
            return;
        }
        self.calls.push(out, conf, now_s);
        for (i, &c) in out.iter().enumerate() {
            self.text.push_back((now_s, c, conf.get(i).copied().unwrap_or(255)));
        }
        if out.iter().any(|c| !c.is_ascii_whitespace()) {
            self.last_text_s = now_s;
        }
    }

    /// Dimentica il testo più vecchio di 60 s.
    pub(crate) fn trim(&mut self, now_s: f64) {
        self.now_s = now_s;
        while self.text.front().is_some_and(|(at, _, _)| now_s - at > HISTORY_S) {
            self.text.pop_front();
        }
    }

    /// Quello che si mostra del canale.
    pub(crate) fn info(&self, id: u32, freq_hz: f32, wpm: f32, snr_db: f32, level: Validation, check: &dyn Fn(&str) -> CallCheck) -> ChannelInfo {
        let calls = self.calls.calls(level, check);
        ChannelInfo {
            id,
            freq_hz,
            wpm,
            snr_db,
            // Latin-1, un carattere per byte: le probabilità restano allineate ai caratteri
            text: self.text.iter().map(|(_, c, _)| char::from(*c)).collect(),
            conf: self.text.iter().map(|(_, _, p)| *p).collect(),
            cq: calls.iter().any(|c| c.role == Role::Cq),
            calls,
            last_report: self.calls.last_report.clone(),
            now_s: self.now_s,
        }
    }
}

/// I candidati agli spot di tutta la banda: i nominativi verificati che passano il livello di convalida, riproposti ogni `SPOT_REOFFER_S`
/// (o subito se la frequenza cambia di oltre 1 kHz o la stazione si scopre a chiamare CQ). Le regole vere degli spot (uno per nominativo e
/// banda ogni 10 minuti...) le applica chi li riceve ([`crate::band::SpotGate`]), che conosce la banda.
#[derive(Default)]
pub(crate) struct Spotter {
    spotted: HashMap<String, (f64, f32, bool)>,
    spots: Vec<LocalSpot>,
}

impl Spotter {
    pub(crate) fn clear(&mut self) {
        self.spotted.clear();
        self.spots.clear();
    }

    /// Guarda i nominativi di un canale (frequenza, velocità, S/N per lo spot) al momento `now`.
    pub(crate) fn check(&mut self, tr: &mut Transcript, sig: (f32, f32, f32), now: f64, level: Validation, check: &dyn Fn(&str) -> CallCheck) {
        let (freq, wpm, snr_db) = sig;
        tr.calls.expire(now, SPOT_REPEAT_S);
        for call in tr.calls.calls(level, check).into_iter().filter(HeardCall::verified) {
            let cq = call.role == Role::Cq;
            let again = match self.spotted.get(&call.call) {
                None => true,
                Some(&(at, f, was_cq)) => now - at > SPOT_REOFFER_S || (f - freq).abs() > 1_000.0 || (cq && !was_cq),
            };
            if again {
                self.spotted.insert(call.call.clone(), (now, freq, cq));
                self.spots.push(LocalSpot { call: call.call.clone(), freq_hz: freq, wpm, snr_db, cq, de: call.role == Role::De, at_s: now });
            }
        }
    }

    pub(crate) fn take(&mut self) -> Vec<LocalSpot> {
        std::mem::take(&mut self.spots)
    }
}

/// Dice cosa si sa di un nominativo fuori dal segnale: paese DXCC e probabilita' a priori della forma.
pub type CallCheckFn = Box<dyn Fn(&str) -> CallCheck + Send>;

/// Un decodificatore di tutta la banda (CW o PSK), come lo usa il servizio.
pub trait BandDecoder: Send {
    /// Altro audio a 12 kHz.
    fn process(&mut self, audio: &[f32]);
    /// I canali aperti, dal tono più basso.
    fn channels(&self) -> Vec<ChannelInfo>;
    /// Gli spot locali nuovi dall'ultima volta.
    fn take_spots(&mut self) -> Vec<LocalSpot>;
    /// Quanti canali al massimo (se sono di più si chiudono i più deboli).
    fn set_max_channels(&mut self, n: usize);
    fn channel_count(&self) -> usize;
    /// Chi dice cosa si sa di un nominativo (paese DXCC, forma).
    fn set_call_check(&mut self, f: CallCheckFn);
    /// Cancella tutto e ricomincia.
    fn clear(&mut self);
    /// Il livello di convalida dei nominativi.
    fn set_validation(&mut self, v: Validation);
    /// La radio si e' spostata di poco nella stessa banda: ogni segnale cambia tono audio di `audio_hz` (con la radio in USB e' meno lo
    /// spostamento della radio). I canali seguono il loro segnale con lo storico e i nominativi; quelli fuori da `SHIFT_LO_HZ`-`SHIFT_HI_HZ`
    /// si chiudono. Chi non sa spostarsi ricomincia da capo.
    fn shift(&mut self, audio_hz: f32) {
        let _ = audio_hz;
        self.clear();
    }
}

/// I nominativi verificati dei canali chiusi, per frequenza: un canale che si riapre entro `CALL_CACHE_HZ` e `CALL_CACHE_S` li ritrova
/// (verificati alla prima lettura esatta) invece di ricominciare da zero quando la stazione torna dopo una pausa o un QSB.
const CALL_CACHE_HZ: f32 = 50.0;
const CALL_CACHE_S: f64 = 240.0;

#[derive(Default)]
pub(crate) struct CallCache {
    items: Vec<(f32, f64, Vec<HeardCall>)>,
}

impl CallCache {
    /// Un canale chiuso a `freq_hz` al momento `now_s`.
    pub(crate) fn store(&mut self, freq_hz: f32, now_s: f64, tr: &Transcript) {
        let calls = tr.calls.verified_calls();
        if !calls.is_empty() {
            self.items.push((freq_hz, now_s, calls));
        }
    }

    /// I nominativi per un canale nuovo a `freq_hz` (si tolgono dalla memoria).
    pub(crate) fn take_near(&mut self, freq_hz: f32, now_s: f64) -> Vec<HeardCall> {
        self.items.retain(|(_, at, _)| now_s - at <= CALL_CACHE_S);
        let mut out = Vec::new();
        self.items.retain(|(f, _, calls)| {
            let near = (f - freq_hz).abs() <= CALL_CACHE_HZ;
            if near {
                out.extend(calls.iter().cloned());
            }
            !near
        });
        out
    }

    pub(crate) fn shift(&mut self, audio_hz: f32) {
        for it in self.items.iter_mut() {
            it.0 += audio_hz;
        }
    }

    pub(crate) fn clear(&mut self) {
        self.items.clear();
    }
}

/// La banda audio in cui un canale spostato resta aperto (Hz).
pub const SHIFT_LO_HZ: f32 = 150.0;
pub const SHIFT_HI_HZ: f32 = 4_000.0;

/// Un canale: un segnale CW della banda.
pub struct Channel {
    pub id: u32,
    rx: CwDemodulator,
    tr: Transcript,
    born_s: f64,
    /// Il rapporto fra la riga del tono e il fondo negli spettri a tasto abbassato degli ultimi `KEYDOWN_S` secondi (per lo spot).
    keydown: VecDeque<(f64, f32)>,
}

impl Channel {
    /// S/N dello spot, riferito a 500 Hz: la mediana delle misure a tasto abbassato degli ultimi 15 s (il QSB e le pause non la spostano).
    /// Il rapporto riga/fondo di uno spettro e' P/(N0·risoluzione), quindi P/N0 in 500 Hz = rapporto · risoluzione / 500. Senza misure a
    /// tasto abbassato, il S/N del ricevitore (in 3 kHz) riportato a 500 Hz.
    fn spot_snr_db(&self) -> f32 {
        if self.keydown.len() < 3 {
            return self.rx.snr_db() + 10.0 * (3_000.0_f32 / 500.0).log10();
        }
        let mut v: Vec<f32> = self.keydown.iter().map(|(_, r)| *r).collect();
        v.sort_by(f32::total_cmp);
        // uno spettro (170 ms) prende spesso un punto e una pausa insieme: a tasto davvero abbassato sono quelli vicini ai piu' forti
        // (sopra il 70% del 90° percentile); di questi la mediana
        let p90 = v[(v.len() * 9 / 10).min(v.len() - 1)];
        let down: Vec<f32> = v.into_iter().filter(|r| *r >= 0.7 * p90).collect();
        let median = down[down.len() / 2];
        10.0 * (median * (RATE / FFT_N as f32) / 500.0).max(1e-6).log10()
    }
}

/// Due canali vicini (la stessa stazione presa due volte, o una lettura dall'altro lato del suo tono): un frammento o una lettura sbagliata
/// di un nominativo sentito di piu' sul canale accanto non si mostra (stessa regola di [`calls::shadows`] dentro un canale).
const SHADOW_HZ: f32 = 30.0;

pub(crate) fn shadow_across(v: &mut [ChannelInfo]) {
    let all: Vec<(f32, String, f32)> = v.iter().flat_map(|ch| ch.calls.iter().map(|c| (ch.freq_hz, c.call.clone(), c.score)).collect::<Vec<_>>()).collect();
    for ch in v.iter_mut() {
        let f = ch.freq_hz;
        ch.calls.retain(|c| !all.iter().any(|(of, other, os)| (of - f).abs() < SHADOW_HZ && calls::shadows(other, *os, &c.call, c.score)));
    }
}

/// Quello che si mostra di un canale.
#[derive(Clone, Debug, PartialEq)]
pub struct ChannelInfo {
    pub id: u32,
    pub freq_hz: f32,
    /// CW: la velocità capita; PSK: 0.
    pub wpm: f32,
    pub snr_db: f32,
    /// Il testo degli ultimi 60 secondi.
    pub text: String,
    /// La probabilità (0-255) di ogni carattere di `text`.
    pub conf: Vec<u8>,
    /// I nominativi che passano la convalida, dal più probabile.
    pub calls: Vec<HeardCall>,
    /// Il segnale chiama CQ (o TEST).
    pub cq: bool,
    pub last_report: Option<String>,
    /// Il momento del flusso (s) a cui vale il canale: con `last_s` di un nominativo dice da quanto non lo si sente.
    pub now_s: f64,
}

/// Uno spot locale: un nominativo verificato su un canale.
#[derive(Clone, Debug, PartialEq)]
pub struct LocalSpot {
    pub call: String,
    pub freq_hz: f32,
    pub wpm: f32,
    pub snr_db: f32,
    pub cq: bool,
    /// Lo si sente dopo «DE» (trasmette lui).
    pub de: bool,
    pub at_s: f64,
}

pub struct CwBandDecoder {
    fft: Arc<dyn Fft<f32>>,
    window: Vec<f32>,
    input: Vec<f32>,
    /// Gli ultimi `CATCH_UP_S` secondi di audio, per i canali nuovi.
    recent: VecDeque<f32>,
    power: Vec<f32>,
    lo_bin: usize,
    hi_bin: usize,
    /// Righe candidate (riga → spettri di fila sopra la soglia).
    stable: HashMap<usize, u32>,
    channels: Vec<Channel>,
    max_channels: usize,
    next_id: u32,
    t: f64,
    validation: Validation,
    dxcc: Option<CallCheckFn>,
    spotter: Spotter,
    cache: CallCache,
}

impl CwBandDecoder {
    /// Cerca i segnali fra `lo_hz` e `hi_hz` (audio), con al massimo `max_channels` canali aperti.
    pub fn new(lo_hz: f32, hi_hz: f32, max_channels: usize) -> Self {
        let mut planner = FftPlanner::new();
        let res = RATE / FFT_N as f32;
        CwBandDecoder {
            fft: planner.plan_fft_forward(FFT_N),
            window: (0..FFT_N).map(|i| 0.5 - 0.5 * (2.0 * std::f32::consts::PI * i as f32 / FFT_N as f32).cos()).collect(),
            input: Vec::with_capacity(FFT_N * 2),
            recent: VecDeque::with_capacity((CATCH_UP_S * RATE) as usize + 1),
            power: vec![0.0; FFT_N / 2],
            lo_bin: ((lo_hz.max(100.0) / res) as usize).max(4),
            hi_bin: ((hi_hz.min(5_500.0) / res) as usize).min(FFT_N / 2 - 5),
            stable: HashMap::new(),
            channels: Vec::new(),
            max_channels: max_channels.max(1),
            next_id: 1,
            t: 0.0,
            validation: Validation::Normal,
            dxcc: None,
            spotter: Spotter::default(),
            cache: CallCache::default(),
        }
    }

    /// Il livello di convalida dei nominativi (di base Normale).
    pub fn set_validation(&mut self, v: Validation) {
        self.validation = v;
    }

    /// Chi dice cosa si sa di un nominativo (paese DXCC, forma).
    pub fn set_call_check(&mut self, f: CallCheckFn) {
        self.dxcc = Some(f);
    }

    /// Cancella tutto e ricomincia.
    pub fn clear(&mut self) {
        self.channels.clear();
        self.stable.clear();
        self.recent.clear();
        self.spotter.clear();
        self.cache.clear();
    }

    /// La radio spostata di poco: i canali seguono il loro segnale (vedi [`BandDecoder::shift`]).
    pub fn shift(&mut self, audio_hz: f32) {
        self.channels.retain(|ch| (SHIFT_LO_HZ..=SHIFT_HI_HZ).contains(&(ch.rx.frequency() + audio_hz)));
        for ch in self.channels.iter_mut() {
            let f = ch.rx.frequency() + audio_hz;
            ch.rx.set_frequency(f);
            ch.keydown.clear();
        }
        // lo spettro mediato si sposta con i segnali; le righe candidate e l'audio recente (della sintonia vecchia) si dimenticano
        let bins = (audio_hz / (RATE / FFT_N as f32)).round() as isize;
        let old = std::mem::take(&mut self.power);
        self.power = (0..old.len() as isize).map(|k| usize::try_from(k - bins).ok().and_then(|j| old.get(j).copied()).unwrap_or(0.0)).collect();
        self.stable.clear();
        self.recent.clear();
        self.cache.shift(audio_hz);
    }

    /// Quanti canali al massimo: se sono di più si chiudono i più deboli.
    pub fn set_max_channels(&mut self, n: usize) {
        self.max_channels = n.max(1);
        if self.channels.len() > self.max_channels {
            self.channels.sort_by(|a, b| b.rx.snr_db().total_cmp(&a.rx.snr_db()));
            self.channels.truncate(self.max_channels);
        }
    }

    pub fn channel_count(&self) -> usize {
        self.channels.len()
    }

    /// Altro audio a 12 kHz.
    pub fn process(&mut self, audio: &[f32]) {
        for ch in self.channels.iter_mut() {
            for block in audio.chunks(960) {
                let out = ch.rx.process(block);
                ch.tr.push(&out, ch.rx.confidences(), self.t);
            }
            ch.tr.trim(self.t);
        }
        let keep = (CATCH_UP_S * RATE) as usize;
        self.recent.extend(audio.iter().copied());
        while self.recent.len() > keep {
            self.recent.pop_front();
        }
        self.input.extend_from_slice(audio);
        while self.input.len() >= FFT_N {
            self.spectrum();
            self.input.drain(..HOP);
            self.t += HOP as f64 / f64::from(RATE);
        }
        self.housekeeping();
    }

    fn spectrum(&mut self) {
        let mut buf: Vec<Complex32> = self.input[..FFT_N].iter().zip(&self.window).map(|(x, w)| Complex32::new(x * w, 0.0)).collect();
        self.fft.process(&mut buf);
        for (p, c) in self.power.iter_mut().zip(&buf) {
            let v = c.norm_sqr();
            *p += 0.4 * (v - *p);
        }
        let band = &self.power[self.lo_bin..self.hi_bin];
        let mut sorted = band.to_vec();
        sorted.sort_by(f32::total_cmp);
        let floor = sorted[sorted.len() / 2].max(1e-20);
        let thr = floor * 10f32.powf(PEAK_DB / 10.0);
        let mut seen = Vec::new();
        for k in self.lo_bin + 4..self.hi_bin - 4 {
            let p = self.power[k];
            if p < thr || (k - 3..=k + 3).any(|j| self.power[j] > p) {
                continue;
            }
            // una riga stretta: 4 righe più in là (23 Hz) è già 6 dB sotto (un CW; RTTY e PSK sono larghi o doppi)
            if self.power[k - 4] > p / 4.0 || self.power[k + 4] > p / 4.0 {
                continue;
            }
            seen.push(k);
        }
        let mut next = HashMap::new();
        for k in seen {
            // la riga accanto conta come la stessa (un tono fra due righe)
            let n = (k.saturating_sub(1)..=k + 1).filter_map(|j| self.stable.get(&j)).copied().max().unwrap_or(0) + 1;
            next.insert(k, n);
            if n >= STABLE_FRAMES {
                self.open_channel(k);
            }
        }
        self.stable = next;
        self.measure_keydown(&buf, floor);
    }

    /// S/N per gli spot: in ogni spettro (non mediato) la potenza delle tre righe attorno al tono di ogni canale contro il fondo; conta solo
    /// a tasto abbassato (6 dB sopra il fondo), e si tiene `KEYDOWN_S` secondi.
    fn measure_keydown(&mut self, buf: &[Complex32], floor: f32) {
        let res = RATE / FFT_N as f32;
        let now = self.t;
        // il fondo e' la mediana dello spettro mediato: per lo spettro di un solo istante va bene lo stesso (le scale sono uguali)
        for ch in self.channels.iter_mut() {
            let k = (ch.rx.frequency() / res).round() as usize;
            if k < 2 || k + 2 >= buf.len() {
                continue;
            }
            let p: f32 = buf[k - 1..=k + 1].iter().map(|c| c.norm_sqr()).sum();
            let ratio = (p - 3.0 * floor) / floor;
            if ratio > KEYDOWN_RATIO {
                ch.keydown.push_back((now, ratio));
            }
            while ch.keydown.front().is_some_and(|(at, _)| now - at > KEYDOWN_S) {
                ch.keydown.pop_front();
            }
        }
    }

    fn open_channel(&mut self, bin: usize) {
        let res = RATE / FFT_N as f32;
        // interpolazione parabolica del picco
        let (a, b, c) = (self.power[bin - 1], self.power[bin], self.power[bin + 1]);
        let den = a - 2.0 * b + c;
        let off = if den.abs() > 1e-30 { 0.5 * (a - c) / den } else { 0.0 };
        let hz = (bin as f32 + off.clamp(-0.5, 0.5)) * res;
        if self.channels.iter().any(|ch| (ch.rx.frequency() - hz).abs() < CHANNEL_SPACING_HZ) || self.channels.len() >= self.max_channels {
            return;
        }
        let mut rx = CwDemodulator::new(hz, 20.0);
        rx.set_afc(true);
        // la stazione accanto puo' stare a 60-100 Hz: nelle pause l'AFC non deve saltarle addosso
        rx.set_afc_limit(CHANNEL_AFC_HZ);
        rx.set_beam(CHANNEL_BEAM);
        // il canale ascolta prima l'audio recente (fino alla fine del blocco in corso): l'inizio della stazione non si perde
        let mut tr = Transcript::new(self.t);
        tr.calls.remember(self.cache.take_near(hz, self.t));
        let recent: Vec<f32> = self.recent.iter().copied().collect();
        for block in recent.chunks(960) {
            let out = rx.process(block);
            tr.push(&out, rx.confidences(), self.t);
        }
        self.channels.push(Channel { id: self.next_id, rx, tr, born_s: self.t, keydown: VecDeque::new() });
        self.next_id += 1;
    }

    /// Chiude i canali muti e quelli doppi, e raccoglie gli spot.
    fn housekeeping(&mut self) {
        let now = self.t;
        let cache = &mut self.cache;
        self.channels.retain(|ch| {
            let open = now - ch.tr.last_text_s < IDLE_S;
            if !open {
                cache.store(ch.rx.frequency(), now, &ch.tr);
            }
            open
        });
        // due canali sulla stessa stazione: resta il più vecchio
        let mut i = 0;
        while i < self.channels.len() {
            let fi = self.channels[i].rx.frequency();
            let dup = self.channels.iter().enumerate().any(|(j, ch)| j != i && ch.born_s < self.channels[i].born_s && (ch.rx.frequency() - fi).abs() < 0.5 * CHANNEL_SPACING_HZ);
            if dup {
                self.channels.remove(i);
            } else {
                i += 1;
            }
        }
        // le stazioni forti (S/N a tasto abbassato misurato sullo spettro) bastano meno ipotesi: meno CPU dove la lettura e' facile
        for ch in self.channels.iter_mut() {
            let beam = if ch.keydown.len() >= 3 && ch.spot_snr_db() >= STRONG_SNR_DB { STRONG_BEAM } else { CHANNEL_BEAM };
            ch.rx.set_beam(beam);
        }
        let dxcc_db = self.dxcc.as_deref();
        let dxcc = |c: &str| dxcc_db.map(|f| f(c)).unwrap_or_default();
        for ch in self.channels.iter_mut() {
            let sig = (ch.rx.frequency(), ch.rx.wpm(), ch.spot_snr_db());
            self.spotter.check(&mut ch.tr, sig, now, self.validation, &dxcc);
        }
    }

    /// I canali aperti, dal tono più basso.
    pub fn channels(&self) -> Vec<ChannelInfo> {
        let dxcc = |c: &str| self.dxcc.as_ref().map(|f| f(c)).unwrap_or_default();
        let mut v: Vec<ChannelInfo> = self.channels.iter().map(|ch| ch.tr.info(ch.id, ch.rx.frequency(), ch.rx.wpm(), ch.rx.snr_db(), self.validation, &dxcc)).collect();
        shadow_across(&mut v);
        v.sort_by(|a, b| a.freq_hz.total_cmp(&b.freq_hz));
        v
    }

    /// Gli spot locali nuovi dall'ultima volta.
    pub fn take_spots(&mut self) -> Vec<LocalSpot> {
        self.spotter.take()
    }
}

impl BandDecoder for CwBandDecoder {
    fn process(&mut self, audio: &[f32]) {
        CwBandDecoder::process(self, audio);
    }
    fn channels(&self) -> Vec<ChannelInfo> {
        CwBandDecoder::channels(self)
    }
    fn take_spots(&mut self) -> Vec<LocalSpot> {
        CwBandDecoder::take_spots(self)
    }
    fn set_max_channels(&mut self, n: usize) {
        CwBandDecoder::set_max_channels(self, n);
    }
    fn channel_count(&self) -> usize {
        CwBandDecoder::channel_count(self)
    }
    fn set_call_check(&mut self, f: CallCheckFn) {
        CwBandDecoder::set_call_check(self, f);
    }
    fn clear(&mut self) {
        CwBandDecoder::clear(self);
    }
    fn set_validation(&mut self, v: Validation) {
        CwBandDecoder::set_validation(self, v);
    }
    fn shift(&mut self, audio_hz: f32) {
        CwBandDecoder::shift(self, audio_hz);
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::cw::modulate;

    pub(crate) fn noise(n: usize, sigma: f32, seed: u64) -> Vec<f32> {
        let mut x = seed | 1;
        let mut next = move || {
            x ^= x >> 12;
            x ^= x << 25;
            x ^= x >> 27;
            ((x.wrapping_mul(0x2545_F491_4F6C_DD1D) >> 11) as f64 + 0.5) / (1u64 << 53) as f64
        };
        (0..n).map(|_| ((-2.0 * next().ln()).sqrt() * (2.0 * std::f64::consts::PI * next()).cos()) as f32 * sigma).collect()
    }

    #[test]
    fn three_stations_three_channels_with_their_calls() {
        let stations = [(600.0_f32, 18.0_f32, "IU3ABC"), (1150.0, 26.0, "K1ABC"), (1900.0, 22.0, "DL1ABC")];
        let mut band = vec![0.0_f32; 12_000 * 40];
        for (hz, wpm, call) in stations {
            let text = format!("CQ CQ DE {call} {call} K   CQ CQ DE {call} {call} K");
            let wave = modulate(text.as_bytes(), hz, wpm, 0.2);
            for (s, w) in band.iter_mut().skip(6_000).zip(wave) {
                *s += w;
            }
        }
        let hiss = noise(band.len(), 0.05, 9);
        for (s, n) in band.iter_mut().zip(hiss) {
            *s += n;
        }
        let mut sk = CwBandDecoder::new(300.0, 2_500.0, 16);
        for block in band.chunks(4_800) {
            sk.process(block);
        }
        let chans = sk.channels();
        assert_eq!(chans.len(), 3, "{chans:#?}");
        for ((hz, _, call), ch) in stations.iter().zip(&chans) {
            assert!((ch.freq_hz - hz).abs() < 5.0, "{ch:?}");
            assert!(ch.calls.iter().any(|c| c.call == *call && c.role == Role::Cq && c.verified()), "{ch:?}");
            assert!(ch.cq);
            assert_eq!(ch.conf.len(), ch.text.chars().count());
        }
        let spots = sk.take_spots();
        // il primo CQ si perde mentre il canale si apre: lo spot arriva con il primo nominativo verificato e torna quando la stazione
        // si scopre a chiamare CQ
        for (_, _, call) in stations {
            let mine: Vec<&LocalSpot> = spots.iter().filter(|s| s.call == call).collect();
            assert!(!mine.is_empty() && mine.iter().any(|s| s.cq), "{spots:?}");
            // S/N in 500 Hz a tasto abbassato: tono di ampiezza 0,2 nel rumore di deviazione 0,05 a 12 kHz fa 19,8 dB
            let last = mine.last().map_or(0.0, |s| s.snr_db);
            assert!((last - 19.8).abs() < 3.0, "{call}: {last:.1} dB");
        }
        // meno canali: restano i più forti
        sk.set_max_channels(2);
        assert_eq!(sk.channel_count(), 2);
    }

    /// La radio si sposta di poco: i canali seguono il tono nuovo con i loro nominativi, quelli che escono dalla banda audio si chiudono,
    /// e l'ascolto continua sul tono nuovo senza aprire canali doppi.
    #[test]
    fn a_small_dial_move_shifts_the_channels() {
        let make = |hz: f32, secs: usize| {
            let mut band = vec![0.0_f32; 12_000 * secs];
            let wave = modulate(b"CQ CQ DE IU3ABC IU3ABC K   CQ CQ DE IU3ABC IU3ABC K", hz, 20.0, 0.2);
            for (s, w) in band.iter_mut().skip(6_000).zip(wave) {
                *s += w;
            }
            for (s, n) in band.iter_mut().zip(noise(12_000 * secs, 0.05, 11)) {
                *s += n;
            }
            band
        };
        let mut sk = CwBandDecoder::new(150.0, 4_000.0, 16);
        for block in make(800.0, 30).chunks(4_800) {
            sk.process(block);
        }
        let before = sk.channels();
        assert!(before.len() == 1 && before[0].calls.iter().any(|c| c.call == "IU3ABC" && c.verified()), "{before:#?}");
        // radio 300 Hz piu' in basso (USB): il tono sale di 300 Hz
        sk.shift(300.0);
        let after = sk.channels();
        assert_eq!(after.len(), 1);
        assert!((after[0].freq_hz - 1_100.0).abs() < 5.0 && after[0].id == before[0].id, "{after:#?}");
        assert!(after[0].calls.iter().any(|c| c.call == "IU3ABC"), "{after:#?}");
        for block in make(1_100.0, 30).chunks(4_800) {
            sk.process(block);
        }
        let later = sk.channels();
        assert!(later.len() == 1 && later[0].id == before[0].id, "{later:#?}");
        // fuori dalla banda audio: il canale si chiude
        sk.shift(-1_000.0);
        assert!(sk.channels().is_empty());
    }

    /// La memoria dei canali chiusi: un canale nuovo vicino ritrova i nominativi verificati, uno lontano o troppo tardi no.
    #[test]
    fn closed_channels_leave_their_calls_to_a_reopened_one() {
        let mut tr = Transcript::new(0.0);
        tr.calls.push(b"CQ TEST EC7YY EC7YY TEST ", &[], 1.0);
        let mut cache = CallCache::default();
        cache.store(1_000.0, 50.0, &tr);
        assert!(cache.take_near(1_100.0, 60.0).is_empty());
        cache.shift(100.0);
        assert!(cache.take_near(1_130.0, 60.0).iter().any(|c| c.call == "EC7YY"));
        assert!(cache.take_near(1_100.0, 60.0).is_empty());
        cache.store(1_000.0, 50.0, &tr);
        assert!(cache.take_near(1_000.0, 50.0 + CALL_CACHE_S + 1.0).is_empty());
    }

    #[test]
    fn noise_opens_no_channel() {
        let mut sk = CwBandDecoder::new(300.0, 2_500.0, 16);
        for block in noise(12_000 * 20, 0.2, 3).chunks(4_800) {
            sk.process(block);
        }
        assert!(sk.channels().is_empty());
    }
}
