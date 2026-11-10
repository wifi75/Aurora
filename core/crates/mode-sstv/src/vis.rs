//! Riconoscimento del codice VIS (leader a 1900 Hz, bit di partenza a 1200 Hz, 8 bit da 30 ms a 1100/1300 Hz, bit di arresto), sul
//! modello di `visfskid.cpp` di QSSTV (Johan Maes ON4QZ, GPL): medie di 1 ms, mediane per tratto, scarto di sintonia dal leader.

use std::collections::VecDeque;

use crate::modes::SstvMode;

/// Campioni (a 12 kHz) in una media.
const BIN: usize = 12;
/// Millisecondi tenuti: leader + VIS + margine.
const KEEP: usize = 700;
/// Scarto di sintonia massimo accettato.
pub const MAX_OFFSET_HZ: f32 = 250.0;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct VisHit {
    pub mode: SstvMode,
    /// Indice assoluto (campioni demodulati) della fine del bit di arresto.
    pub end_sample: u64,
    /// Scarto di sintonia stimato dal leader (Hz).
    pub offset_hz: f32,
}

pub struct VisDetector {
    bins: VecDeque<f32>,
    acc: f32,
    acc_n: usize,
    /// Numero di medie complete ricevute.
    count: u64,
    /// Candidato migliore in attesa di conferma: (errore, colpo, medie trascorse).
    pending: Option<(f32, VisHit, u32)>,
    /// Centro atteso del leader (1900 + correzione manuale).
    expected_leader: f32,
}

fn median(values: &mut [f32]) -> f32 {
    values.sort_by(|a, b| a.total_cmp(b));
    values[values.len() / 2]
}

impl VisDetector {
    pub fn new(manual_offset_hz: f32) -> VisDetector {
        VisDetector { bins: VecDeque::with_capacity(KEEP), acc: 0.0, acc_n: 0, count: 0, pending: None, expected_leader: 1900.0 + manual_offset_hz }
    }

    pub fn set_manual_offset(&mut self, hz: f32) {
        self.expected_leader = 1900.0 + hz;
    }

    pub fn reset(&mut self) {
        self.bins.clear();
        self.pending = None;
    }

    /// Aggiunge le frequenze demodulate; `first_index` e' l'indice assoluto del primo valore. Ritorna un VIS riconosciuto.
    pub fn push(&mut self, freqs: &[f32], first_index: u64) -> Option<VisHit> {
        let mut found = None;
        for (k, &f) in freqs.iter().enumerate() {
            self.acc += f;
            self.acc_n += 1;
            if self.acc_n < BIN {
                continue;
            }
            let mean = self.acc / BIN as f32;
            self.acc = 0.0;
            self.acc_n = 0;
            self.count += 1;
            if self.bins.len() == KEEP {
                self.bins.pop_front();
            }
            self.bins.push_back(mean);
            let end_sample = first_index + k as u64 + 1;
            if let Some((err, hit)) = self.test() {
                let hit = VisHit { end_sample, ..hit };
                match self.pending {
                    Some((best, _, _)) if best <= err => {}
                    _ => self.pending = Some((err, hit, 0)),
                }
            }
            if let Some((err, hit, age)) = self.pending {
                // si aspetta una dozzina di millisecondi: l'allineamento con l'errore piu' piccolo vince
                if age >= 12 {
                    self.pending = None;
                    found = Some(hit);
                } else {
                    self.pending = Some((err, hit, age + 1));
                }
            }
        }
        found
    }

    fn window(&self, from: usize, to: usize) -> Vec<f32> {
        // `from`/`to` in millisecondi dall'inizio del bit di partenza; l'ultima media e' la fine del bit di arresto (300 ms)
        let n = self.bins.len();
        let base = n - 300;
        (from..to).filter_map(|i| self.bins.get(base.wrapping_add(i))).copied().collect()
    }

    /// Il VIS che finisce adesso, se c'e': (errore, colpo senza posizione).
    fn test(&self) -> Option<(f32, VisHit)> {
        if self.bins.len() < 560 {
            return None;
        }
        let n = self.bins.len();
        let base = n - 300;
        let get = |from: isize, to: isize| -> Vec<f32> { (from..to).filter_map(|i| self.bins.get((base as isize + i) as usize)).copied().collect() };
        let mut leader = get(-250, -15);
        let lead = median(&mut leader);
        let offset = lead - 1900.0;
        if (lead - self.expected_leader).abs() > MAX_OFFSET_HZ {
            return None;
        }
        let near = leader.iter().filter(|&&f| (f - lead).abs() < 80.0).count();
        if near * 10 < leader.len() * 6 {
            return None;
        }
        let mut err = 0.0;
        let tone = |from: usize, to: usize| -> f32 {
            let mut w = self.window(from, to);
            median(&mut w) - offset
        };
        for (from, to) in [(5, 25), (275, 295)] {
            let f = tone(from, to);
            if (f - 1200.0).abs() > 70.0 {
                return None;
            }
            err += (f - 1200.0).abs();
        }
        let mut code = 0u8;
        for bit in 0..8 {
            let from = 35 + 30 * bit;
            let f = tone(from, from + 20);
            let (one, zero) = ((f - 1100.0).abs(), (f - 1300.0).abs());
            if one.min(zero) > 80.0 {
                return None;
            }
            err += one.min(zero);
            if one < zero {
                code |= 1 << bit;
            }
        }
        if code.count_ones() % 2 != 0 {
            return None;
        }
        let mode = SstvMode::from_vis(code & 0x7f)?;
        Some((err, VisHit { mode, end_sample: 0, offset_hz: offset }))
    }
}
