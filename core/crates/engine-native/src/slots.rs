//! Divide il flusso a 12 kHz nei periodi FT8 (15 secondi) o FT4 (7,5 secondi): i periodi iniziano ad ore UTC esatte (FT8 :00, :15, :30, :45;
//! FT4 ogni 7,5 s).
//!
//! Il flusso non ha un'ora propria: ogni blocco porta l'istante in cui e' arrivato (la fine dei suoi campioni). Si ancora il primo
//! campione a quell'istante e si segue la scheda correggendo piano piano la deriva; un salto grande (blocco perso) riancora di colpo.

use qso_native::Mode;

use crate::RATE;

/// Durata di un periodo FT8 (usata anche per allineare i messaggi di JTDX, che e' sempre in FT8).
pub const SLOT_MS: u64 = 15_000;
/// Il periodo si consegna questi millisecondi prima della sua fine (gli ultimi non portano segnale: un messaggio FT8 finisce a 13,1 s):
/// cosi' la decodifica e la scelta della risposta sono pronte per trasmettere a inizio del periodo dopo.
pub const EARLY_MS: u64 = 800;
const EARLY_SAMPLES: usize = (EARLY_MS as usize) * RATE / 1000;
/// Oltre questo scarto tra l'ora attesa e quella vera si riancora di colpo.
const JUMP_MS: f64 = 200.0;
/// Correzione graduale dello scarto (frazione per blocco).
const SMOOTH: f64 = 0.02;

/// Un periodo completo di audio.
#[derive(Debug, Clone)]
pub struct Slot {
    /// Inizio del periodo, millisecondi dal 1970 (UTC), multiplo della durata del periodo (15 000 in FT8, 7 500 in FT4).
    pub start_unix_ms: u64,
    pub samples: Vec<f32>,
}

pub struct SlotAssembler {
    /// Durata del periodo in ms e in campioni.
    period_ms: u64,
    slot_samples: usize,
    /// Istante (ms dal 1970) del campione assoluto 0.
    anchor_ms: Option<f64>,
    buf: Vec<f32>,
    /// Indice assoluto del primo campione di `buf`.
    first_idx: u64,
    total: u64,
    next_slot_ms: Option<u64>,
}

fn time_of(anchor: f64, idx: u64) -> f64 {
    anchor + idx as f64 * 1000.0 / RATE as f64
}

impl Default for SlotAssembler {
    fn default() -> Self {
        SlotAssembler::new(Mode::Ft8)
    }
}

impl SlotAssembler {
    /// Un assemblatore per i periodi del modo dato.
    pub fn new(mode: Mode) -> SlotAssembler {
        let period_ms = mode.period_ms();
        SlotAssembler {
            period_ms,
            slot_samples: (period_ms as usize) * RATE / 1000,
            anchor_ms: None,
            buf: Vec::new(),
            first_idx: 0,
            total: 0,
            next_slot_ms: None,
        }
    }

    /// Aggiunge un blocco a 12 kHz arrivato all'istante `end_unix_ms`; restituisce i periodi che si sono completati.
    pub fn push(&mut self, samples: &[f32], end_unix_ms: u64) -> Vec<Slot> {
        if samples.is_empty() {
            return Vec::new();
        }
        let end = end_unix_ms as f64;
        let mut anchor = match self.anchor_ms {
            Some(a) => a,
            None => end - samples.len() as f64 * 1000.0 / RATE as f64,
        };
        self.buf.extend_from_slice(samples);
        self.total += samples.len() as u64;
        let error = end - time_of(anchor, self.total);
        if error.abs() > JUMP_MS {
            // blocco perso o orologio spostato: i campioni di prima non hanno piu' un'ora credibile
            anchor += error;
            self.buf.clear();
            self.buf.extend_from_slice(samples);
            self.first_idx = self.total - samples.len() as u64;
            self.next_slot_ms = None;
        } else {
            anchor += error * SMOOTH;
        }
        self.anchor_ms = Some(anchor);

        let mut ready = Vec::new();
        let buf_start_ms = time_of(anchor, self.first_idx);
        let mut next = *self.next_slot_ms.get_or_insert_with(|| ((buf_start_ms / self.period_ms as f64).ceil() as u64) * self.period_ms);
        loop {
            let slot_end_ms = (next + self.period_ms - EARLY_MS) as f64;
            if time_of(anchor, self.total) < slot_end_ms {
                break;
            }
            let from = ((next as f64 - anchor) * RATE as f64 / 1000.0).round() as i64 - self.first_idx as i64;
            if from >= 0 && from as usize + self.slot_samples - EARLY_SAMPLES <= self.buf.len() {
                let from = from as usize;
                let end = (from + self.slot_samples).min(self.buf.len());
                let mut samples = self.buf[from..end].to_vec();
                samples.resize(self.slot_samples, 0.0);
                ready.push(Slot { start_unix_ms: next, samples });
            }
            next += self.period_ms;
        }
        self.next_slot_ms = Some(next);
        // si tiene solo cio' che serve al periodo in corso (e un secondo prima, per il ritardo negativo)
        let keep_from_ms = next as f64 - 1000.0;
        let keep_idx = (((keep_from_ms - anchor) * RATE as f64 / 1000.0).floor() as i64).max(self.first_idx as i64) as u64;
        let drop = (keep_idx - self.first_idx) as usize;
        if drop > 0 && drop <= self.buf.len() {
            self.buf.drain(..drop);
            self.first_idx = keep_idx;
        }
        ready
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Simula una scheda: blocchi da `chunk` campioni ciascuno, con l'ora di arrivo giusta, a partire da `t0_ms`.
    fn feed(asm: &mut SlotAssembler, t0_ms: u64, seconds: f64, chunk: usize, pulse_at_ms: Option<u64>) -> Vec<Slot> {
        let total = (seconds * RATE as f64) as usize;
        let mut out = Vec::new();
        let mut n = 0usize;
        while n < total {
            let len = chunk.min(total - n);
            let block: Vec<f32> = (n..n + len)
                .map(|i| {
                    let t = t0_ms as f64 + i as f64 * 1000.0 / RATE as f64;
                    pulse_at_ms.map_or(0.0, |p| if (t - p as f64).abs() < 0.04 { 1.0 } else { 0.0 })
                })
                .collect();
            let end_ms = t0_ms as f64 + (n + len) as f64 * 1000.0 / RATE as f64;
            out.extend(asm.push(&block, end_ms.round() as u64));
            n += len;
        }
        out
    }

    #[test]
    fn slots_start_on_the_quarter_minute_and_are_complete() {
        let mut asm = SlotAssembler::default();
        // si parte a 1000000 ms: il primo periodo intero comincia a 1005000
        let slots = feed(&mut asm, 1_000_000, 40.0, 480, None);
        assert_eq!(slots.iter().map(|s| s.start_unix_ms).collect::<Vec<_>>(), vec![1_005_000, 1_020_000]);
        assert!(slots.iter().all(|s| s.samples.len() == 180_000));
    }

    #[test]
    fn ft4_slots_are_seven_and_a_half_seconds_aligned_to_the_utc() {
        let mut asm = SlotAssembler::new(Mode::Ft4);
        // si parte a 1 000 000 ms (multiplo di 7 500? no: 1 000 000 / 7 500 = 133,33): il primo periodo intero comincia a 1 005 000
        let slots = feed(&mut asm, 1_000_000, 30.0, 480, None);
        let starts: Vec<u64> = slots.iter().map(|s| s.start_unix_ms).collect();
        assert_eq!(starts, vec![1_005_000, 1_012_500, 1_020_000], "{starts:?}");
        assert!(starts.iter().all(|s| s % 7_500 == 0));
        assert!(slots.iter().all(|s| s.samples.len() == 90_000));
    }

    #[test]
    fn a_sample_keeps_its_place_inside_the_slot() {
        let mut asm = SlotAssembler::default();
        // un impulso a 1005000 + 2500 ms deve stare al campione 2,5 s * 12000 = 30000 del periodo
        let slots = feed(&mut asm, 1_000_000, 22.0, 600, Some(1_007_500));
        assert_eq!(slots.len(), 1);
        let peak = slots[0].samples.iter().enumerate().filter(|&(_, &v)| v > 0.5).map(|(i, _)| i).collect::<Vec<_>>();
        assert!(!peak.is_empty());
        let centre = peak.iter().sum::<usize>() as f64 / peak.len() as f64;
        assert!((centre - 30_000.0).abs() < 12.0, "centro {centre}");
    }

    #[test]
    fn a_lost_block_skips_the_slot_it_damaged_but_not_the_next() {
        let mut asm = SlotAssembler::default();
        let mut slots = feed(&mut asm, 1_000_000, 12.0, 480, None);
        // 6 secondi di buco: le ore dei blocchi dopo saltano avanti
        slots.extend(feed(&mut asm, 1_018_000, 40.0, 480, None));
        let starts: Vec<u64> = slots.iter().map(|s| s.start_unix_ms).collect();
        assert!(!starts.contains(&1_005_000), "il periodo con il buco non e' completo: {starts:?}");
        assert!(starts.contains(&1_035_000), "{starts:?}");
    }
}
