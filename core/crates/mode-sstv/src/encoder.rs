//! Dall'immagine all'audio: toni di calibrazione, leader e VIS come `sstvTx::sendPreamble`/`sendVIS` di QSSTV, poi le righe del modo
//! con un oscillatore a fase continua (`synthesizer` di QSSTV).

use std::f64::consts::TAU;

use crate::{
    RgbImage, SstvError,
    color::{chroma, luma, value_to_hz},
    modes::{Chan, Family, LEADER_HZ, ModeSpec, SYNC_HZ, SegKind, SstvMode, WHITE_HZ},
};

/// Ampiezza di picco dell'onda (la catena di trasmissione regola poi il livello).
const AMPLITUDE: f64 = 0.95;

struct Synth {
    rate: f64,
    phase: f64,
    /// Tempo (secondi) alla fine dell'ultimo segmento: i campioni seguono il tempo esatto, senza errori che si accumulano.
    clock: f64,
    /// Spostamento di tutti i toni (Hz).
    shift: f64,
    out: Vec<f32>,
}

impl Synth {
    fn new(rate: u32, seconds: f64, shift: f64) -> Synth {
        Synth { rate: f64::from(rate), phase: 0.0, clock: 0.0, shift, out: Vec::with_capacity((seconds * f64::from(rate)) as usize + 16) }
    }

    fn push(&mut self, hz: f64) {
        self.out.push((AMPLITUDE * self.phase.sin()) as f32);
        self.phase = (self.phase + TAU * (hz + self.shift) / self.rate) % TAU;
    }

    fn tone(&mut self, hz: f32, dur: f64) {
        self.clock += dur;
        let end = (self.clock * self.rate).round() as usize;
        while self.out.len() < end {
            self.push(f64::from(hz));
        }
    }

    /// Una scansione: `values` sono i pixel (0-255), distribuiti in parti uguali su `dur`.
    fn scan(&mut self, values: &[f32], dur: f64) {
        let start = self.clock;
        self.clock += dur;
        let end = (self.clock * self.rate).round() as usize;
        let n = values.len().max(1);
        while self.out.len() < end {
            let t = self.out.len() as f64 / self.rate - start;
            let i = ((t / dur * n as f64).floor().max(0.0) as usize).min(n - 1);
            let hz = values.get(i).map_or(1500.0, |&v| value_to_hz(v));
            self.push(f64::from(hz));
        }
    }
}

/// Toni di calibrazione e leader prima del VIS (sendPreamble di QSSTV).
fn header(s: &mut Synth, vis: u8) {
    for hz in [1900.0, 1500.0, 1900.0, 1500.0, 2300.0, 1500.0, 2300.0, 1500.0] {
        s.tone(hz, 0.1);
    }
    s.tone(LEADER_HZ, 0.3);
    s.tone(SYNC_HZ, 0.01);
    s.tone(LEADER_HZ, 0.3);
    // VIS: bit di partenza, 7 bit dal meno significativo, parita' pari, bit di arresto; 1 = 1100 Hz, 0 = 1300 Hz
    let parity = vis.count_ones() % 2 == 1;
    let code = u16::from(vis & 0x7f) | if parity { 0x80 } else { 0 };
    s.tone(SYNC_HZ, 0.03);
    for bit in 0..8 {
        s.tone(if code >> bit & 1 == 1 { 1100.0 } else { 1300.0 }, 0.03);
    }
    s.tone(SYNC_HZ, 0.03);
}

/// I canali di un gruppo pronti da trasmettere.
struct GroupData {
    r: Vec<f32>,
    g: Vec<f32>,
    b: Vec<f32>,
    y0: Vec<f32>,
    y1: Vec<f32>,
    v: Vec<f32>,
    u: Vec<f32>,
}

fn pixel(img: &RgbImage, x: u32, y: u32) -> [f32; 3] {
    let i = ((y.min(img.height - 1) * img.width + x) * 3) as usize;
    [f32::from(img.data[i]), f32::from(img.data[i + 1]), f32::from(img.data[i + 2])]
}

fn group_data(spec: &ModeSpec, img: &RgbImage, group: u32) -> GroupData {
    let w = spec.width;
    let mut d = GroupData { r: Vec::new(), g: Vec::new(), b: Vec::new(), y0: Vec::new(), y1: Vec::new(), v: Vec::new(), u: Vec::new() };
    match spec.family {
        Family::Martin | Family::Scottie | Family::Wraase => {
            for x in 0..w {
                let [r, g, b] = pixel(img, x, group);
                d.r.push(r);
                d.g.push(g);
                d.b.push(b);
            }
        }
        Family::Robot72 => {
            for x in 0..w {
                let [r, g, b] = pixel(img, x, group);
                let y = luma(r, g, b);
                let (v, u) = chroma(r, b, y);
                d.y0.push(y);
                d.v.push(v);
                d.u.push(u);
            }
        }
        Family::Robot36 | Family::Pd => {
            // crominanza media delle due righe della coppia (getLineY con evenodd di QSSTV)
            let (row0, row1) = match spec.family {
                Family::Pd => (group * 2, group * 2 + 1),
                _ => (group & !1, group | 1),
            };
            for x in 0..w {
                let [r0, g0, b0] = pixel(img, x, row0);
                let [r1, g1, b1] = pixel(img, x, row1);
                let (ya, yb) = (luma(r0, g0, b0), luma(r1, g1, b1));
                let (v, u) = chroma((r0 + r1) / 2.0, (b0 + b1) / 2.0, (ya + yb) / 2.0);
                if spec.family == Family::Pd {
                    d.y0.push(ya);
                    d.y1.push(yb);
                } else {
                    d.y0.push(if group.is_multiple_of(2) { ya } else { yb });
                }
                d.v.push(v);
                d.u.push(u);
            }
        }
    }
    d
}

/// Codifica un'immagine (gia' alla risoluzione del modo) in audio a `rate` Hz, VIS compreso.
pub fn encode(mode: SstvMode, img: &RgbImage, rate: u32) -> Result<Vec<f32>, SstvError> {
    encode_shifted(mode, img, rate, 0.0)
}

/// Come [`encode`] con tutti i toni spostati di `shift_hz` (correzione della sintonia in trasmissione).
pub fn encode_shifted(mode: SstvMode, img: &RgbImage, rate: u32, shift_hz: f32) -> Result<Vec<f32>, SstvError> {
    let spec = mode.spec();
    if img.width != spec.width || img.height != spec.height || img.data.len() != (img.width * img.height * 3) as usize {
        return Err(SstvError::BadImage(format!("servono {}x{} pixel RGB", spec.width, spec.height)));
    }
    if rate < 8000 {
        return Err(SstvError::BadRate(rate));
    }
    let mut s = Synth::new(rate, spec.total_seconds(), f64::from(shift_hz.clamp(-300.0, 300.0)));
    header(&mut s, spec.vis);
    if spec.lead_sync() > 0.0 {
        s.tone(SYNC_HZ, spec.lead_sync());
    }
    let segments = spec.segments();
    for group in 0..spec.groups() {
        let d = group_data(spec, img, group);
        for seg in &segments {
            match seg.kind {
                SegKind::Tone(hz) => s.tone(hz, seg.dur),
                SegKind::RobotSep => s.tone(if group % 2 == 0 { 1500.0 } else { WHITE_HZ }, seg.dur),
                SegKind::Scan(chan) => {
                    let values = match chan {
                        Chan::R => &d.r,
                        Chan::G => &d.g,
                        Chan::B => &d.b,
                        Chan::Y0 => &d.y0,
                        Chan::Y1 => &d.y1,
                        Chan::V => &d.v,
                        Chan::U => &d.u,
                        Chan::Alt => {
                            if group % 2 == 0 {
                                &d.v
                            } else {
                                &d.u
                            }
                        }
                    };
                    s.scan(values, seg.dur);
                }
            }
        }
    }
    // breve coda a 1500 Hz: chi riceve chiude bene l'ultima riga
    s.tone(1500.0, 0.05);
    Ok(s.out)
}
