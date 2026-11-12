//! Decodifica di tutti i segnali CW della banda audio, ognuno con il suo ricevitore completo e il suo storico.
//!
//! Uno spettro (FFT di 2048 punti a 12 kHz, 5,9 Hz per riga, ogni 85 ms) cerca i toni CW: righe strette che stanno almeno 10 dB sopra il
//! fondo (la mediana della banda) per tre spettri di fila. Ogni tono nuovo, se non c'e' gia' un canale entro 25 Hz, apre un canale: un
//! [`CwDemodulator`] intero (filtro, AGC, AFC, decodificatore probabilistico con un fascio più piccolo) sintonizzato su di lui. Un canale
//! che non sente niente per `IDLE_S` secondi si chiude; due canali finiti sulla stessa stazione (l'AFC li ha avvicinati) diventano uno.
//!
//! Per ogni canale restano i caratteri degli ultimi 60 secondi (il clic sulla cascata può mostrare tutto il passaggio) e i nominativi
//! sentiti, con il ruolo (CQ, DE) e la verifica per ricorrenza ([`calls`]). Gli «spot locali» sono i nominativi che passano la convalida
//! scelta: uno per nominativo ogni 10 minuti, di nuovo se cambia frequenza di oltre 1 kHz o se lo si sente chiamare CQ.

pub mod calls;

use crate::cw::CwDemodulator;
use calls::{CallTracker, HeardCall, Role, Validation};
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
/// Un nominativo si ripete come spot dopo tanto (s).
const SPOT_REPEAT_S: f64 = 600.0;

/// Un canale: un segnale CW della banda.
pub struct Channel {
    pub id: u32,
    rx: CwDemodulator,
    text: VecDeque<(f64, u8)>,
    calls: CallTracker,
    born_s: f64,
    last_text_s: f64,
}

/// Quello che si mostra di un canale.
#[derive(Clone, Debug, PartialEq)]
pub struct ChannelInfo {
    pub id: u32,
    pub freq_hz: f32,
    pub wpm: f32,
    pub snr_db: f32,
    /// Il testo degli ultimi 60 secondi.
    pub text: String,
    /// I nominativi che passano la convalida, dal più probabile.
    pub calls: Vec<HeardCall>,
    /// Il segnale chiama CQ (o TEST).
    pub cq: bool,
    pub last_report: Option<String>,
}

/// Uno spot locale: un nominativo verificato su un canale.
#[derive(Clone, Debug, PartialEq)]
pub struct LocalSpot {
    pub call: String,
    pub freq_hz: f32,
    pub wpm: f32,
    pub snr_db: f32,
    pub cq: bool,
    pub at_s: f64,
}

pub struct CwBandDecoder {
    fft: Arc<dyn Fft<f32>>,
    window: Vec<f32>,
    input: Vec<f32>,
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
    dxcc: Option<Box<dyn Fn(&str) -> Option<bool> + Send>>,
    spotted: HashMap<String, (f64, f32, bool)>,
    spots: Vec<LocalSpot>,
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
            spotted: HashMap::new(),
            spots: Vec::new(),
        }
    }

    /// Il livello di convalida dei nominativi (di base Normale).
    pub fn set_validation(&mut self, v: Validation) {
        self.validation = v;
    }

    /// Chi dice se un nominativo porta a un paese DXCC (`Some(false)` = no; `None` = non si sa).
    pub fn set_dxcc(&mut self, f: Box<dyn Fn(&str) -> Option<bool> + Send>) {
        self.dxcc = Some(f);
    }

    /// Cancella tutto e ricomincia.
    pub fn clear(&mut self) {
        self.channels.clear();
        self.stable.clear();
        self.spotted.clear();
        self.spots.clear();
    }

    /// Altro audio a 12 kHz.
    pub fn process(&mut self, audio: &[f32]) {
        for ch in self.channels.iter_mut() {
            let mut out = Vec::new();
            for block in audio.chunks(960) {
                out.extend(ch.rx.process(block));
                let conf = ch.rx.confidences().to_vec();
                if !out.is_empty() {
                    ch.calls.push(&out, &conf, self.t);
                    for &c in &out {
                        ch.text.push_back((self.t, c));
                    }
                    if out.iter().any(|c| *c != b' ') {
                        ch.last_text_s = self.t;
                    }
                    out.clear();
                }
            }
            while ch.text.front().is_some_and(|(at, _)| self.t - at > HISTORY_S) {
                ch.text.pop_front();
            }
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
        rx.set_beam(CHANNEL_BEAM);
        self.channels.push(Channel { id: self.next_id, rx, text: VecDeque::new(), calls: CallTracker::new(), born_s: self.t, last_text_s: self.t });
        self.next_id += 1;
    }

    /// Chiude i canali muti e quelli doppi, e raccoglie gli spot.
    fn housekeeping(&mut self) {
        let now = self.t;
        self.channels.retain(|ch| now - ch.last_text_s < IDLE_S);
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
        let dxcc_db = self.dxcc.as_deref();
        let dxcc = |c: &str| dxcc_db.and_then(|f| f(c));
        for ch in self.channels.iter_mut() {
            ch.calls.expire(now, SPOT_REPEAT_S);
            for call in ch.calls.calls(Validation::Strict.max(self.validation), &dxcc) {
                let freq = ch.rx.frequency();
                let cq = call.role == Role::Cq;
                let again = match self.spotted.get(&call.call) {
                    None => true,
                    Some(&(at, f, was_cq)) => now - at > SPOT_REPEAT_S || (f - freq).abs() > 1_000.0 || (cq && !was_cq),
                };
                if again {
                    self.spotted.insert(call.call.clone(), (now, freq, cq));
                    self.spots.push(LocalSpot { call: call.call.clone(), freq_hz: freq, wpm: ch.rx.wpm(), snr_db: ch.rx.snr_db(), cq, at_s: now });
                }
            }
        }
    }

    /// I canali aperti, dal tono più basso.
    pub fn channels(&self) -> Vec<ChannelInfo> {
        let dxcc = |c: &str| self.dxcc.as_ref().and_then(|f| f(c));
        let mut v: Vec<ChannelInfo> = self
            .channels
            .iter()
            .map(|ch| {
                let calls = ch.calls.calls(self.validation, &dxcc);
                ChannelInfo {
                    id: ch.id,
                    freq_hz: ch.rx.frequency(),
                    wpm: ch.rx.wpm(),
                    snr_db: ch.rx.snr_db(),
                    text: String::from_utf8_lossy(&ch.text.iter().map(|(_, c)| *c).collect::<Vec<u8>>()).into_owned(),
                    cq: calls.iter().any(|c| c.role == Role::Cq),
                    calls,
                    last_report: ch.calls.last_report.clone(),
                }
            })
            .collect();
        v.sort_by(|a, b| a.freq_hz.total_cmp(&b.freq_hz));
        v
    }

    /// Gli spot locali nuovi dall'ultima volta.
    pub fn take_spots(&mut self) -> Vec<LocalSpot> {
        std::mem::take(&mut self.spots)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cw::modulate;

    fn noise(n: usize, sigma: f32, seed: u64) -> Vec<f32> {
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
        }
        let spots = sk.take_spots();
        // il primo CQ si perde mentre il canale si apre: lo spot arriva con il primo nominativo verificato e torna quando la stazione
        // si scopre a chiamare CQ
        for (_, _, call) in stations {
            let mine: Vec<&LocalSpot> = spots.iter().filter(|s| s.call == call).collect();
            assert!((1..=2).contains(&mine.len()) && mine.iter().any(|s| s.cq), "{spots:?}");
        }
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
