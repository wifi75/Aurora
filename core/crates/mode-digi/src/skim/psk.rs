//! Tutti i segnali PSK31 o PSK63 della banda audio (il «browser»): ognuno con il suo [`PskDemodulator`] completo, il suo storico di 60 s
//! e i nominativi sentiti.
//!
//! Il rivelatore guarda lo spettro (FFT di 4096 punti a 12 kHz, 2,9 Hz per riga, ogni 170 ms, mediato su circa un secondo): l'energia in
//! una finestra larga quanto un segnale PSK (±20 Hz in PSK31, ±40 in PSK63) deve stare 5 dB sopra il fondo della banda (la mediana delle
//! finestre). Un tono puro (portante, CW) mette quasi tutta l'energia in tre righe e si scarta. La frequenza e' il baricentro della
//! finestra: a pochi Hz dalla portante, poi l'AFC del ricevitore la aggancia.
//!
//! Un segnale visto in due spettri di fila apre un canale «in prova» (se non ce n'e' gia' uno vicino). Il ricevitore del canale ha lo
//! squelch sulla qualità della fase acceso (soglia di base), che sul rumore e sugli altri modi resta chiuso: se in `TRIAL_S` secondi non
//! si apre mai il segnale non e' un PSK del modo scelto, il canale si chiude e quella frequenza non si riprova per `REJECT_S` secondi. Un
//! canale confermato si chiude dopo `IDLE_S` secondi senza segnale e senza testo. Nell'elenco vanno solo i canali confermati.

use super::calls::Validation;
use super::{BandDecoder, ChannelInfo, DxccCheck, LocalSpot, Spotter, Transcript};
use crate::psk::{PskDemodulator, PskMode, SQUELCH_DEFAULT};
use rustfft::num_complex::Complex32;
use rustfft::{Fft, FftPlanner};
use std::collections::HashMap;
use std::sync::Arc;

const RATE: f32 = 12_000.0;
const FFT_N: usize = 4096;
const HOP: usize = 2048;
/// L'energia nella finestra del segnale sopra il fondo (dB).
const ENERGY_DB: f32 = 5.0;
/// Spettri di fila perché un segnale apra un canale.
const STABLE_FRAMES: u32 = 2;
/// Quanto resta in prova un canale nuovo (s).
const TRIAL_S: f64 = 10.0;
/// Una frequenza scartata (non e' un PSK) non si riprova per tanto (s).
const REJECT_S: f64 = 60.0;
/// Un canale senza segnale e senza testo da tanto si chiude (s).
const IDLE_S: f64 = 30.0;

struct Channel {
    id: u32,
    rx: PskDemodulator,
    tr: Transcript,
    born_s: f64,
    /// Lo squelch del ricevitore si e' aperto almeno una volta: e' un PSK.
    confirmed: bool,
    /// L'ultima volta con lo squelch aperto.
    last_dcd_s: f64,
    /// Rapporto segnale/rumore dallo spettro (dB in 3 kHz, come per il CW e l'FT8; quello del ricevitore, alla fldigi, ha un'altra scala).
    snr_db: f32,
}

impl Channel {
    fn snr(&self) -> f32 {
        self.snr_db
    }
}

pub struct PskBandDecoder {
    mode: PskMode,
    fft: Arc<dyn Fft<f32>>,
    window: Vec<f32>,
    input: Vec<f32>,
    direct: Vec<f32>,
    frames: u32,
    lo_bin: usize,
    hi_bin: usize,
    stable: HashMap<usize, u32>,
    channels: Vec<Channel>,
    rejected: Vec<(f32, f64)>,
    max_channels: usize,
    next_id: u32,
    t: f64,
    validation: Validation,
    dxcc: Option<DxccCheck>,
    spotter: Spotter,
}

impl PskBandDecoder {
    /// Cerca i segnali del modo fra `lo_hz` e `hi_hz` (audio), con al massimo `max_channels` canali aperti.
    pub fn new(mode: PskMode, lo_hz: f32, hi_hz: f32, max_channels: usize) -> Self {
        let res = RATE / FFT_N as f32;
        PskBandDecoder {
            mode,
            fft: FftPlanner::new().plan_fft_forward(FFT_N),
            window: (0..FFT_N).map(|i| 0.5 - 0.5 * (2.0 * std::f32::consts::PI * i as f32 / FFT_N as f32).cos()).collect(),
            input: Vec::with_capacity(FFT_N * 2),
            direct: vec![0.0; FFT_N / 2],
            frames: 0,
            lo_bin: ((lo_hz.max(150.0) / res) as usize).max(40),
            hi_bin: ((hi_hz.min(5_000.0) / res) as usize).min(FFT_N / 2 - 40),
            stable: HashMap::new(),
            channels: Vec::new(),
            rejected: Vec::new(),
            max_channels: max_channels.max(1),
            next_id: 1,
            t: 0.0,
            validation: Validation::Normal,
            dxcc: None,
            spotter: Spotter::default(),
        }
    }

    pub fn mode(&self) -> PskMode {
        self.mode
    }

    /// Mezza larghezza del segnale in righe dello spettro (±20 Hz in PSK31, ±40 in PSK63).
    fn half_width(&self) -> usize {
        match self.mode {
            PskMode::Psk31 => 7,
            PskMode::Psk63 => 14,
        }
    }

    /// Distanza minima fra due canali (Hz).
    fn spacing_hz(&self) -> f32 {
        match self.mode {
            PskMode::Psk31 => 30.0,
            PskMode::Psk63 => 60.0,
        }
    }

    fn spectrum(&mut self) {
        let mut a: Vec<Complex32> = self.input[..FFT_N].iter().zip(&self.window).map(|(x, w)| Complex32::new(x * w, 0.0)).collect();
        self.fft.process(&mut a);
        // media di circa un secondo (sei spettri); i primi entrano interi
        self.frames = self.frames.saturating_add(1);
        let k = (1.0 / self.frames as f32).max(0.3);
        for (d, x) in self.direct.iter_mut().zip(&a) {
            *d += k * (x.norm_sqr() - *d);
        }
        let res = RATE / FFT_N as f32;
        let hw = self.half_width();
        let (lo, hi) = (self.lo_bin, self.hi_bin);
        // energia nella finestra del segnale, per ogni riga
        let mut energy = vec![0.0_f32; hi + 1];
        for (k, e) in energy.iter_mut().enumerate().take(hi).skip(lo) {
            *e = self.direct[k - hw..=k + hw].iter().sum();
        }
        let mut sorted = energy[lo..hi].to_vec();
        sorted.sort_by(f32::total_cmp);
        let floor = sorted[sorted.len() / 2].max(1e-20);
        let thr = floor * 10f32.powf(ENERGY_DB / 10.0);
        let mut seen = Vec::new();
        for k in lo + 1..hi - 1 {
            let e = energy[k];
            if e < thr || (k.saturating_sub(hw)..=(k + hw).min(hi - 1)).any(|j| energy[j] > e) {
                continue;
            }
            // non un tono puro: la riga piu' forte della finestra e le due accanto non fanno quasi tutta l'energia (un PSK31 ne mette
            // meno della meta' in tre righe, una portante quasi tutta)
            let top = (k - hw..=k + hw).max_by(|&x, &y| self.direct[x].total_cmp(&self.direct[y])).unwrap_or(k);
            let peak: f32 = self.direct[top - 1..=top + 1].iter().sum();
            if peak > 0.7 * e {
                continue;
            }
            // la frequenza: il baricentro della finestra, tolto il fondo
            let noise = floor / (2 * hw + 1) as f32;
            let (mut w, mut wk) = (0.0_f32, 0.0_f32);
            for j in k - hw..=k + hw {
                let p = (self.direct[j] - noise).max(0.0);
                w += p;
                wk += p * j as f32;
            }
            let hz = if w > 0.0 { wk / w * res } else { k as f32 * res };
            // S/N in 3 kHz: la potenza del segnale nella finestra contro il rumore della finestra, riportato a 3 kHz
            let snr = 10.0 * ((e - floor).max(1e-30) / floor).log10() + 10.0 * ((2 * hw + 1) as f32 * res / 3_000.0).log10();
            seen.push((k, hz, snr));
        }
        let mut next = HashMap::new();
        for (k, hz, snr) in seen {
            let n = (k.saturating_sub(2)..=k + 2).filter_map(|j| self.stable.get(&j)).copied().max().unwrap_or(0) + 1;
            next.insert(k, n);
            let mut known = false;
            let half = 0.5 * self.spacing_hz();
            for ch in self.channels.iter_mut().filter(|ch| (ch.rx.frequency() - hz).abs() < half) {
                ch.snr_db = snr;
                known = true;
            }
            if !known && n >= STABLE_FRAMES {
                self.open_channel(hz, snr);
            }
        }
        self.stable = next;
    }

    fn open_channel(&mut self, hz: f32, snr_db: f32) {
        let spacing = self.spacing_hz();
        let now = self.t;
        self.rejected.retain(|&(_, until)| until > now);
        if self.channels.iter().any(|ch| (ch.rx.frequency() - hz).abs() < spacing)
            || self.rejected.iter().any(|&(f, _)| (f - hz).abs() < 0.5 * spacing)
            || self.channels.len() >= self.max_channels
        {
            return;
        }
        let mut rx = PskDemodulator::new(self.mode, hz);
        rx.set_afc(true);
        rx.set_squelch(Some(SQUELCH_DEFAULT));
        self.channels.push(Channel { id: self.next_id, rx, tr: Transcript::new(now), born_s: now, confirmed: false, last_dcd_s: now, snr_db });
        self.next_id += 1;
    }

    fn housekeeping(&mut self) {
        let now = self.t;
        // in prova senza mai aprire lo squelch: non e' un PSK del modo scelto
        let rejected: Vec<f32> = self.channels.iter().filter(|ch| !ch.confirmed && now - ch.born_s > TRIAL_S).map(|ch| ch.rx.frequency()).collect();
        self.rejected.extend(rejected.into_iter().map(|f| (f, now + REJECT_S)));
        self.channels.retain(|ch| if ch.confirmed { now - ch.last_dcd_s < IDLE_S || now - ch.tr.last_text_s < IDLE_S } else { now - ch.born_s <= TRIAL_S });
        let half = 0.5 * self.spacing_hz();
        let mut i = 0;
        while i < self.channels.len() {
            let fi = self.channels[i].rx.frequency();
            let dup = self.channels.iter().enumerate().any(|(j, ch)| j != i && ch.born_s < self.channels[i].born_s && (ch.rx.frequency() - fi).abs() < half);
            if dup {
                self.channels.remove(i);
            } else {
                i += 1;
            }
        }
        let dxcc_db = self.dxcc.as_deref();
        let dxcc = |c: &str| dxcc_db.and_then(|f| f(c));
        for ch in self.channels.iter_mut().filter(|ch| ch.confirmed) {
            let sig = (ch.rx.frequency(), 0.0, ch.snr());
            self.spotter.check(&mut ch.tr, sig, now, self.validation, &dxcc);
        }
    }
}

impl BandDecoder for PskBandDecoder {
    fn process(&mut self, audio: &[f32]) {
        for ch in self.channels.iter_mut() {
            // a blocchi di 80 ms: lo squelch aperto si vede anche in un blocco lungo
            for block in audio.chunks(960) {
                let out = ch.rx.process(block);
                ch.tr.push(&out, ch.rx.confidences(), self.t);
                if ch.rx.signal().1 {
                    ch.confirmed = true;
                    ch.last_dcd_s = self.t;
                }
            }
            ch.tr.trim(self.t);
        }
        self.input.extend_from_slice(audio);
        while self.input.len() >= FFT_N {
            self.spectrum();
            self.input.drain(..HOP);
            self.t += HOP as f64 / f64::from(RATE);
        }
        self.housekeeping();
    }

    fn channels(&self) -> Vec<ChannelInfo> {
        let dxcc = |c: &str| self.dxcc.as_ref().and_then(|f| f(c));
        // i canali ancora in prova non si mostrano
        let mut v: Vec<ChannelInfo> = self.channels.iter().filter(|ch| ch.confirmed).map(|ch| ch.tr.info(ch.id, ch.rx.frequency(), 0.0, ch.snr(), self.validation, &dxcc)).collect();
        v.sort_by(|a, b| a.freq_hz.total_cmp(&b.freq_hz));
        v
    }

    fn take_spots(&mut self) -> Vec<LocalSpot> {
        self.spotter.take()
    }

    fn set_max_channels(&mut self, n: usize) {
        self.max_channels = n.max(1);
        if self.channels.len() > self.max_channels {
            self.channels.sort_by(|a, b| b.rx.metric().total_cmp(&a.rx.metric()));
            self.channels.truncate(self.max_channels);
        }
    }

    fn channel_count(&self) -> usize {
        self.channels.len()
    }

    fn set_dxcc(&mut self, f: DxccCheck) {
        self.dxcc = Some(f);
    }

    fn clear(&mut self) {
        self.channels.clear();
        self.stable.clear();
        self.rejected.clear();
        self.spotter.clear();
        self.frames = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::super::tests::noise;
    use super::*;
    use crate::psk::modulate;
    use crate::skim::calls::Role;

    fn band(stations: &[(f32, &str)], mode: PskMode, sigma: f32, seconds: usize) -> Vec<f32> {
        let mut band = vec![0.0_f32; 12_000 * seconds];
        for (i, (hz, call)) in stations.iter().enumerate() {
            let text = format!("cq cq cq de {call} {call} {call} pse k   cq cq de {call} {call} k");
            let wave = modulate(text.as_bytes(), *hz, mode, 0.15);
            // le stazioni non partono tutte insieme
            for (s, w) in band.iter_mut().skip(3_000 + 1_500 * i).zip(wave) {
                *s += w;
            }
        }
        for (s, n) in band.iter_mut().zip(noise(12_000 * seconds, sigma, 11)) {
            *s += n;
        }
        band
    }

    #[test]
    fn three_psk31_stations_three_channels_with_their_calls() {
        let stations = [(700.0_f32, "IU3ABC"), (1210.0, "K1ABC"), (1830.0, "DL1ABC")];
        let audio = band(&stations, PskMode::Psk31, 0.05, 40);
        let mut b = PskBandDecoder::new(PskMode::Psk31, 300.0, 2_500.0, 16);
        for block in audio.chunks(4_800) {
            b.process(block);
        }
        let chans = b.channels();
        assert_eq!(chans.len(), 3, "{chans:#?}");
        for ((hz, call), ch) in stations.iter().zip(&chans) {
            assert!((ch.freq_hz - hz).abs() < 3.0, "{ch:?}");
            assert!(ch.calls.iter().any(|c| c.call == *call && c.role == Role::Cq && c.verified()), "{ch:?}");
            assert!(ch.cq);
        }
        let spots = b.take_spots();
        for (_, call) in stations {
            assert!(spots.iter().any(|s| s.call == call && s.cq), "{spots:?}");
        }
    }

    #[test]
    fn psk63_and_a_cw_carrier() {
        let mut audio = band(&[(1000.0, "IU3ABC")], PskMode::Psk63, 0.05, 25);
        // una portante pura a 1600 Hz: non e' un PSK
        for (n, s) in audio.iter_mut().enumerate() {
            *s += 0.15 * (2.0 * std::f32::consts::PI * 1600.0 * n as f32 / RATE).sin();
        }
        let mut b = PskBandDecoder::new(PskMode::Psk63, 300.0, 2_500.0, 16);
        for block in audio.chunks(4_800) {
            b.process(block);
        }
        let chans = b.channels();
        assert_eq!(chans.len(), 1, "{chans:#?}");
        assert!(chans[0].text.contains("de IU3ABC"), "{chans:?}");
    }

    #[test]
    fn noise_opens_no_channel() {
        let mut b = PskBandDecoder::new(PskMode::Psk31, 300.0, 2_500.0, 16);
        for block in noise(12_000 * 20, 0.2, 5).chunks(4_800) {
            b.process(block);
        }
        assert!(b.channels().is_empty(), "{:?}", b.channels());
    }
}
