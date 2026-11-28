//! Riconoscimento del modo senza VIS, dal periodo delle righe: e' il modo automatico di MMSSTV (`CSYNCINT::SyncStart`, `SyncCheck` e
//! `SyncCheckSub` in `sstv.cpp`, Makoto Mori JE3HHT, LGPL, copia in `terze-parti/mmsstv`), riscritto in Rust sulle frequenze demodulate.
//!
//! Si trovano gli impulsi di sincronismo (1200 Hz), si misurano gli intervalli fra un impulso e il successivo e si cerca il modo la cui
//! durata di riga torna con gli ultimi intervalli entro ±3 ms, ammettendo come MMSSTV due o tre righe di fila con il sincronismo perso
//! (intervallo doppio o triplo). In piu' di MMSSTV si guarda anche la durata degli impulsi (5 ms Martin, 9 ms Scottie e Robot, 20 ms PD)
//! e che abbiano tutti la stessa frequenza, da cui viene anche lo scarto di sintonia.

use std::collections::VecDeque;

use crate::modes::{MODES, ModeSpec, SstvMode};

/// Campioni (a 12 kHz) in una media da 1 ms.
const BIN: usize = 12;
/// Tolleranza sulla durata di una riga (come `deff` di MMSSTV).
const TOL_MS: f64 = 3.0;
/// Impulsi ricordati.
const KEEP: usize = 32;
/// Una media sotto questa frequenza (tolta la correzione manuale) e' sincronismo: fra 1200 Hz e il nero a 1500 Hz.
const SYNC_BELOW_HZ: f32 = 1350.0;
/// Quanto possono allargarsi le medie di 1 ms dentro un impulso di sincronismo (Hz): un tono fermo, anche con un po' di rumore, resta dentro.
const PULSE_SPREAD_HZ: f32 = 160.0;
/// Quanto possono differire fra loro le frequenze degli impulsi usati (Hz) e le loro durate (ms).
const MAX_PULSE_HZ_DEV: f32 = 60.0;
const MAX_PULSE_LEN_DEV_MS: f32 = 2.0;
/// Scarto di sintonia massimo accettato.
const MAX_OFFSET_HZ: f32 = 200.0;

#[derive(Clone, Copy, Debug, PartialEq)]
struct Pulse {
    /// Indice assoluto (campioni demodulati) dell'inizio.
    start: u64,
    len_ms: f32,
    /// Frequenza media dell'impulso.
    hz: f32,
}

/// Un modo riconosciuto dal periodo delle righe.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SyncHit {
    pub mode: SstvMode,
    /// Inizio (campioni assoluti) dei sincronismi usati, dal piu' vecchio.
    pub syncs: [u64; 3],
    pub offset_hz: f32,
}

pub struct SyncDetector {
    acc: f32,
    acc_n: usize,
    /// Indice assoluto del prossimo campione.
    at: u64,
    /// Correzione manuale della sintonia.
    manual: f32,
    /// Impulso in corso: (inizio, medie, somma delle frequenze dalla seconda media, minima e massima di quelle medie).
    run: Option<(u64, u32, f32, f32, f32)>,
    pulses: VecDeque<Pulse>,
}

fn median(values: &mut [f32]) -> f32 {
    values.sort_by(|a, b| a.total_cmp(b));
    values.get(values.len() / 2).copied().unwrap_or(0.0)
}

impl SyncDetector {
    pub fn new(manual_offset_hz: f32) -> SyncDetector {
        SyncDetector { acc: 0.0, acc_n: 0, at: 0, manual: manual_offset_hz, run: None, pulses: VecDeque::with_capacity(KEEP) }
    }

    pub fn set_manual_offset(&mut self, hz: f32) {
        self.manual = hz;
    }

    /// Dimentica gli impulsi visti (all'inizio e alla fine di ogni immagine).
    pub fn reset(&mut self) {
        self.run = None;
        self.pulses.clear();
    }

    /// Frequenze demodulate a partire dal campione assoluto `first_index`; `lock`: solo quel modo. Ritorna un modo riconosciuto.
    pub fn push(&mut self, freqs: &[f32], first_index: u64, lock: Option<SstvMode>) -> Option<SyncHit> {
        if first_index != self.at {
            // un buco nel flusso (l'ascolto e' stato spento o c'e' stata una ricezione): gli intervalli di prima non valgono
            self.reset();
            self.acc = 0.0;
            self.acc_n = 0;
            self.at = first_index;
        }
        let mut found = None;
        for &f in freqs {
            self.acc += f;
            self.acc_n += 1;
            self.at += 1;
            if self.acc_n < BIN {
                continue;
            }
            let mean = self.acc / BIN as f32;
            self.acc = 0.0;
            self.acc_n = 0;
            let bin_start = self.at - BIN as u64;
            if mean - self.manual < SYNC_BELOW_HZ {
                self.run = Some(match self.run {
                    // dalla seconda media in poi: somma, minima e massima (il tono vero sta fermo, il rumore balla)
                    Some((s, n, sum, _, _)) if n == 1 => (s, n + 1, sum + mean, mean, mean),
                    Some((s, n, sum, lo, hi)) => (s, n + 1, sum + mean, lo.min(mean), hi.max(mean)),
                    // la prima media e' spesso a cavallo del fronte: non entra nella frequenza dell'impulso
                    None => (bin_start, 1, 0.0, f32::MAX, f32::MIN),
                });
                continue;
            }
            let Some((start, n, sum, lo, hi)) = self.run.take() else { continue };
            // impulsi da 3 a 40 ms: piu' corti e' rumore, piu' lunghi non e' un sincronismo (leader del VIS, un tono fisso)
            if !(3..=40).contains(&n) {
                continue;
            }
            // un sincronismo e' un tono fermo a 1200 Hz: se le medie di 1 ms dentro l'impulso si allargano oltre PULSE_SPREAD_HZ e' rumore
            // che per caso sta sotto 1350 Hz (di notte, senza segnale, bastava per riconoscere un modo e aprire una ricezione a vuoto)
            if hi - lo > PULSE_SPREAD_HZ {
                continue;
            }
            if self.pulses.len() == KEEP {
                self.pulses.pop_front();
            }
            self.pulses.push_back(Pulse { start, len_ms: n as f32, hz: sum / (n - 1) as f32 });
            if let Some(hit) = self.check(lock) {
                self.reset();
                found = Some(hit);
            }
        }
        found
    }

    /// Il modo che torna con gli ultimi impulsi, se c'e'.
    fn check(&self, lock: Option<SstvMode>) -> Option<SyncHit> {
        let mut best: Option<(u32, f64, SyncHit)> = None;
        for spec in MODES.iter().filter(|m| lock.is_none_or(|l| l == m.mode)) {
            if let Some((ones, err, hit)) = self.fit(spec)
                && best.as_ref().is_none_or(|(o, e, _)| ones > *o || (ones == *o && err < *e))
            {
                best = Some((ones, err, hit));
            }
        }
        best.map(|b| b.2)
    }

    /// Gli ultimi impulsi letti con le righe del modo: (intervalli da una riga sola, errore medio in ms, colpo). Partendo dall'impulso
    /// appena finito si va indietro di riga in riga; gli impulsi in mezzo (rumore su un nero, un disturbo) si saltano.
    fn fit(&self, spec: &ModeSpec) -> Option<(u32, f64, SyncHit)> {
        let line_ms = spec.group_seconds() * 1000.0;
        // MMSSTV vuole tre intervalli di fila, quattro per i modi veloci; qui quattro sempre: sul solo rumore tre bastavano per caso
        // e si apriva una ricezione senza nessun segnale
        let need = 4;
        let sync_ms = (spec.sync * 1000.0) as f32;
        // la durata contata in medie da 1 ms: quella vera piu' al massimo due medie (il fronte e un bordo che sfora)
        let fits_width = |p: &Pulse| p.len_ms >= sync_ms - 1.0 && p.len_ms <= sync_ms + 2.5;
        let ms = |a: &Pulse, b: &Pulse| (b.start - a.start) as f64 / f64::from(crate::RATE) * 1000.0;
        let all: Vec<Pulse> = self.pulses.iter().copied().collect();
        let mut used = vec![*all.last()?];
        if !fits_width(&used[0]) {
            return None;
        }
        let (mut ones, mut err) = (0u32, 0.0f64);
        let mut i = all.len() - 1;
        while used.len() <= need {
            let cur = used[used.len() - 1];
            // il piu' vicino all'indietro che cade su un numero intero di righe (1, 2 o 3)
            let mut pick = None;
            for j in (0..i).rev() {
                let w = ms(&all[j], &cur);
                if w > line_ms * 3.0 + TOL_MS {
                    break;
                }
                let k = (w / line_ms).round();
                if (1.0..=3.0).contains(&k) && (w / k - line_ms).abs() <= TOL_MS && fits_width(&all[j]) {
                    pick = Some((j, k, (w / k - line_ms).abs()));
                    break;
                }
            }
            let (j, k, e) = pick?;
            if k == 1.0 {
                ones += 1;
            }
            err += e;
            used.push(all[j]);
            i = j;
        }
        used.reverse();
        // almeno due righe consecutive con il sincronismo
        if ones < 2 {
            return None;
        }
        let mut freqs: Vec<f32> = used.iter().map(|p| p.hz).collect();
        let hz = median(&mut freqs);
        if used.iter().any(|p| (p.hz - hz).abs() > MAX_PULSE_HZ_DEV) {
            return None;
        }
        // i sincronismi di una trasmissione hanno tutti la stessa durata (a meno di una media da 1 ms): il rumore no
        let (short, long) = used.iter().fold((f32::MAX, f32::MIN), |(lo, hi), p| (lo.min(p.len_ms), hi.max(p.len_ms)));
        if long - short > MAX_PULSE_LEN_DEV_MS {
            return None;
        }
        let offset = hz - 1200.0;
        if (offset - self.manual).abs() > MAX_OFFSET_HZ {
            return None;
        }
        let n = used.len();
        let syncs = [used[n - 3].start, used[n - 2].start, used[n - 1].start];
        Some((ones, err / need as f64, SyncHit { mode: spec.mode, syncs, offset_hz: offset }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Frequenze demodulate di solo rumore: una passeggiata correlata attorno a 1500 Hz, da 250 a 2750 Hz, come il discriminatore sul fruscio.
    fn noise(seconds: usize, seed: u64) -> Vec<f32> {
        let mut state = seed;
        let mut f = 1500.0f32;
        (0..seconds * 12_000)
            .map(|_| {
                state = state.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
                let u = ((state >> 33) as f32 / (1u64 << 31) as f32) * 2500.0 + 250.0;
                f = 0.995 * f + 0.005 * u + (u - 1500.0) * 0.02;
                f.clamp(250.0, 2750.0)
            })
            .collect()
    }

    #[test]
    fn noise_alone_never_starts_an_image() {
        for seed in 1..=6u64 {
            let mut det = SyncDetector::new(0.0);
            let hits: Vec<_> = noise(180, seed).chunks(12_000).enumerate().filter_map(|(i, c)| det.push(c, (i * 12_000) as u64, None)).collect();
            assert!(hits.is_empty(), "seme {seed}: rumore preso per {:?}", hits.first().map(|h| h.mode));
        }
    }

    /// Impulsi veri: sincronismi da 9 ms a 1200 Hz ogni riga di Martin 1 su un nero a 1500 Hz.
    #[test]
    fn real_sync_pulses_are_still_found() {
        let spec = crate::modes::SstvMode::Martin1.spec();
        let line = (spec.group_seconds() * 12_000.0).round() as usize;
        let mut samples = Vec::new();
        for _ in 0..6 {
            let mut one = vec![1500.0f32; line];
            for v in one.iter_mut().take((spec.sync * 12_000.0) as usize) {
                *v = 1200.0;
            }
            samples.extend(one);
        }
        let mut det = SyncDetector::new(0.0);
        let hit = det.push(&samples, 0, None);
        assert_eq!(hit.map(|h| h.mode), Some(crate::modes::SstvMode::Martin1));
    }
}
