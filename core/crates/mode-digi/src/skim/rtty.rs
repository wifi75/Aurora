//! RTTY di tutta la banda («Radar» RTTY): i segnali li trova e li decodifica [`crate::rtty_multi::RttyMulti`] (una coppia di toni alla
//! distanza dello spostamento che si alternano, un ricevitore completo per segnale); qui ogni segnale diventa un canale con il suo testo
//! degli ultimi 60 secondi e i nominativi sentiti, come i canali CW e PSK.

use super::calls::Validation;
use super::{BandDecoder, CallCheckFn, ChannelInfo, LocalSpot, Spotter, Transcript};
use crate::rtty_multi::RttyMulti;

const RATE: f64 = 12_000.0;
/// Un canale si riconosce nel segnale nuovo se il centro sta entro questa frazione dello spostamento.
const SAME_FRACTION: f32 = 0.5;
/// Un canale senza testo da tanto si dimentica (s); `RttyMulti` chiude il suo ricevitore dopo 30 s.
const IDLE_S: f64 = 45.0;

struct Lane {
    id: u32,
    centre_hz: f32,
    tr: Transcript,
}

pub struct RttyBandDecoder {
    multi: RttyMulti,
    shift_hz: f32,
    lanes: Vec<Lane>,
    max_channels: usize,
    next_id: u32,
    t: f64,
    validation: Validation,
    dxcc: Option<CallCheckFn>,
    spotter: Spotter,
}

impl RttyBandDecoder {
    pub fn new(shift_hz: f32, baud: f32, max_channels: usize) -> Self {
        RttyBandDecoder {
            multi: RttyMulti::new(shift_hz, baud),
            shift_hz,
            lanes: Vec::new(),
            max_channels: max_channels.max(1),
            next_id: 1,
            t: 0.0,
            validation: Validation::Normal,
            dxcc: None,
            spotter: Spotter::default(),
        }
    }
}

impl BandDecoder for RttyBandDecoder {
    fn process(&mut self, audio: &[f32]) {
        for block in audio.chunks(960) {
            self.t += block.len() as f64 / RATE;
            for lt in self.multi.process(block) {
                let tol = SAME_FRACTION * self.shift_hz;
                let at = self.lanes.iter().position(|l| (l.centre_hz - lt.centre_hz).abs() < tol);
                let idx = match at {
                    Some(i) => i,
                    None => {
                        if self.lanes.len() >= self.max_channels {
                            continue;
                        }
                        self.lanes.push(Lane { id: self.next_id, centre_hz: lt.centre_hz, tr: Transcript::new(self.t) });
                        self.next_id += 1;
                        self.lanes.len() - 1
                    }
                };
                let lane = &mut self.lanes[idx];
                lane.centre_hz = lt.centre_hz;
                let mut text = Vec::with_capacity(lt.chars.len());
                let mut conf = Vec::with_capacity(lt.chars.len());
                for c in &lt.chars {
                    // Latin-1, un carattere per byte (le probabilita' restano allineate)
                    let b = u32::from(c.ch);
                    text.push(if b < 256 { b as u8 } else { b'?' });
                    conf.push((c.prob.clamp(0.0, 1.0) * 255.0).round() as u8);
                }
                lane.tr.push(&text, &conf, self.t);
            }
        }
        let now = self.t;
        for l in self.lanes.iter_mut() {
            l.tr.trim(now);
        }
        self.lanes.retain(|l| now - l.tr.last_text_s < IDLE_S);
        let dxcc_db = self.dxcc.as_deref();
        let dxcc = |c: &str| dxcc_db.map(|f| f(c)).unwrap_or_default();
        for l in self.lanes.iter_mut() {
            self.spotter.check(&mut l.tr, (l.centre_hz, 0.0, 0.0), now, self.validation, &dxcc);
        }
    }

    fn channels(&self) -> Vec<ChannelInfo> {
        let dxcc = |c: &str| self.dxcc.as_ref().map(|f| f(c)).unwrap_or_default();
        let mut v: Vec<ChannelInfo> = self.lanes.iter().map(|l| l.tr.info(l.id, l.centre_hz, 0.0, 0.0, self.validation, &dxcc)).collect();
        v.sort_by(|a, b| a.freq_hz.total_cmp(&b.freq_hz));
        v
    }

    fn take_spots(&mut self) -> Vec<LocalSpot> {
        self.spotter.take()
    }

    fn set_max_channels(&mut self, n: usize) {
        self.max_channels = n.max(1);
        self.lanes.truncate(self.max_channels);
    }

    fn channel_count(&self) -> usize {
        self.lanes.len()
    }

    fn set_call_check(&mut self, f: CallCheckFn) {
        self.dxcc = Some(f);
    }

    fn clear(&mut self) {
        self.lanes.clear();
        self.spotter.clear();
    }

    fn set_validation(&mut self, v: Validation) {
        self.validation = v;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rtty::{RttyParams, modulate};

    /// Le estremita' della banda: una stazione forte a 450 Hz e una a 3700 Hz di centro si trovano e si leggono come quelle in mezzo.
    #[test]
    fn stations_at_the_edges_of_the_band_are_read() {
        let mut band = vec![0.0_f32; 12_000 * 40];
        for (centre, call) in [(450.0_f32, "IU3ABC"), (3_700.0, "K1XYZ")] {
            let text = format!("RYRY CQ TEST {call} {call} {call}  CQ TEST {call} {call} {call}  CQ TEST {call} {call} ");
            let wave = modulate(text.as_bytes(), RttyParams { centre_hz: centre, shift_hz: 170.0, baud: 45.45, reverse: false }, 0.3);
            for (s, w) in band.iter_mut().skip(6_000).zip(wave) {
                *s += w;
            }
        }
        for (s, n) in band.iter_mut().zip(crate::skim::tests::noise(12_000 * 40, 0.02, 7)) {
            *s += n;
        }
        let mut d = RttyBandDecoder::new(170.0, 45.45, 10);
        for block in band.chunks(4_800) {
            d.process(block);
        }
        let chans = d.channels();
        for call in ["IU3ABC", "K1XYZ"] {
            assert!(chans.iter().any(|ch| ch.calls.iter().any(|c| c.call == call && c.verified())), "{call}: {chans:#?}");
        }
    }

    /// Due stazioni RTTY nella banda: due canali, ognuno con il suo nominativo verificato.
    #[test]
    fn two_rtty_stations_two_channels_with_their_calls() {
        let mut band = vec![0.0_f32; 12_000 * 40];
        for (centre, call) in [(900.0_f32, "IU3ABC"), (1_700.0, "K1XYZ")] {
            let text = format!("RYRY CQ CQ DE {call} {call} {call} K  CQ CQ DE {call} {call} {call} K  CQ DE {call} {call} K ");
            let wave = modulate(text.as_bytes(), RttyParams { centre_hz: centre, shift_hz: 170.0, baud: 45.45, reverse: false }, 0.2);
            for (s, w) in band.iter_mut().skip(6_000).zip(wave) {
                *s += w;
            }
        }
        for (s, n) in band.iter_mut().zip(crate::skim::tests::noise(12_000 * 40, 0.02, 5)) {
            *s += n;
        }
        let mut d = RttyBandDecoder::new(170.0, 45.45, 10);
        for block in band.chunks(4_800) {
            d.process(block);
        }
        let chans = d.channels();
        for call in ["IU3ABC", "K1XYZ"] {
            assert!(chans.iter().any(|ch| ch.calls.iter().any(|c| c.call == call && c.verified())), "{call}: {chans:#?}");
        }
    }
}
