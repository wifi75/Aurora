//! MSK144 in tempo reale (come `mskrtd` di WSJT-X): mentre il periodo scorre si decodificano gli ultimi 2,5 s di audio ogni 0,75 s, cosi'
//! un ping compare subito invece che a fine periodo. I messaggi gia' mostrati si ricordano per periodo: la decodifica di fine periodo
//! li passa ancora alla sequenza dei QSO ma non li ripubblica.

use std::collections::HashSet;

use ft8::message::CallHashes;

/// Audio tenuto: 2,5 s a 12 kHz.
const KEEP: usize = 30_000;
/// Ogni quanti campioni nuovi si decodifica: 0,75 s.
const EVERY: usize = 9_000;
/// Durata del periodo MSK144.
const PERIOD_MS: u64 = 15_000;

#[derive(Default)]
pub struct Realtime {
    buf: Vec<f32>,
    fresh: usize,
    /// Il periodo dei messaggi ricordati e i loro testi.
    slot: u64,
    seen: HashSet<String>,
}

impl Realtime {
    /// Aggiunge l'audio (12 kHz) arrivato a `now_ms`; quando e' il momento decodifica e ritorna i messaggi nuovi con l'inizio del loro periodo.
    pub fn push(&mut self, samples: &[f32], now_ms: u64, my_call: &str, his_call: &str, hashes: &mut CallHashes) -> Option<(u64, Vec<ft8::decode::Decoded>)> {
        self.buf.extend_from_slice(samples);
        if self.buf.len() > KEEP {
            let cut = self.buf.len() - KEEP;
            self.buf.drain(..cut);
        }
        self.fresh += samples.len();
        if self.fresh < EVERY || self.buf.len() < KEEP / 2 {
            return None;
        }
        self.fresh = 0;
        let slot = now_ms / PERIOD_MS * PERIOD_MS;
        if slot != self.slot {
            self.slot = slot;
            self.seen.clear();
        }
        let options = jt::msk144::Options { max_pings: 8, my_call: my_call.to_string(), his_call: his_call.to_string(), ..jt::msk144::Options::default() };
        let found: Vec<ft8::decode::Decoded> = jt::msk144::decode(&self.buf, &options, hashes)
            .into_iter()
            .filter(|d| self.seen.insert(d.text.clone()))
            .map(|d| {
                // l'istante del ping nel periodo: fine del blocco meno la sua distanza dalla fine
                let t = (now_ms.saturating_sub(slot)) as f32 / 1000.0 - (self.buf.len() as f32 / 12_000.0 - d.t_s);
                ft8::decode::Decoded { freq_hz: d.freq_hz, dt_s: t.max(0.0), snr_db: d.snr_db, text: d.text, hint: false }
            })
            .collect();
        (!found.is_empty()).then_some((slot, found))
    }

    /// I testi gia' mostrati per il periodo che comincia a `slot_ms` (la decodifica di fine periodo non li ripubblica).
    pub fn shown(&self, slot_ms: u64) -> Vec<String> {
        if slot_ms == self.slot { self.seen.iter().cloned().collect() } else { Vec::new() }
    }

    /// Cambio di modo o di banda: si riparte da zero.
    pub fn reset(&mut self) {
        *self = Realtime::default();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_ping_shows_up_before_the_end_of_the_period_and_only_once() {
        let mut rt = Realtime::default();
        let tones = jt::msk144::tones("CQ IU3ABC IO91", &mut CallHashes::default()).expect("messaggio");
        let ping = jt::msk144::modulate(&tones, 1500.0, 4, 1.0);
        let mut audio = vec![0.0f32; 36_000];
        for (k, v) in ping.iter().enumerate() {
            audio[20_000 + k] += v;
        }
        let mut shown = Vec::new();
        let start = 1_794_000_000_000u64;
        for (i, chunk) in audio.chunks(1_200).enumerate() {
            let now = start + (i as u64 + 1) * 100;
            if let Some((slot, list)) = rt.push(chunk, now, "", "", &mut CallHashes::default()) {
                assert_eq!(slot, start);
                shown.extend(list.into_iter().map(|d| d.text));
            }
        }
        assert_eq!(shown, vec!["CQ IU3ABC IO91".to_string()]);
        assert_eq!(rt.shown(start), vec!["CQ IU3ABC IO91".to_string()]);
    }
}
