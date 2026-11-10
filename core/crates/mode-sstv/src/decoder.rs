//! Decodificatore a flusso: VIS, poi l'immagine gruppo per gruppo con inseguimento del sincronismo e correzione dell'inclinazione.
//!
//! Lo schema segue `sstvrx.cpp`, `syncprocessor.cpp` e `modes/*.cpp` di QSSTV (Johan Maes ON4QZ, GPL): posizione di ogni sincronismo
//! misurata riga per riga, retta dei sincronismi (orologio vero di chi trasmette) usata per collocare i pixel, e alla fine l'immagine
//! ridisegnata con la retta definitiva, cosi' l'inclinazione sparisce anche dalle prime righe.

use std::collections::VecDeque;

use crate::{
    RgbImage,
    color::{hz_to_value, to_u8, yuv_to_rgb},
    demod::{FmDemod, RATE, product_to_hz},
    modes::{Chan, Family, ModeSpec, SegKind, SstvMode},
    vis::VisDetector,
};

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct DecoderConfig {
    /// Solo questo modo (gli altri VIS si ignorano).
    pub lock: Option<SstvMode>,
    /// Correzione manuale della sintonia (Hz): si somma a quella stimata dal VIS quando si parte senza VIS.
    pub offset_hz: f32,
    /// Correzione manuale dell'orologio di chi trasmette, in parti per milione.
    pub slant_ppm: f32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct DecodedImage {
    pub mode: SstvMode,
    pub image: RgbImage,
    /// Parte dei gruppi con un sincronismo trovato (0-1).
    pub sync_quality: f32,
    pub offset_hz: f32,
    /// Scarto dell'orologio misurato (ppm).
    pub clock_ppm: f32,
    /// Tutte le righe ricevute.
    pub complete: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub enum DecoderEvent {
    Started { mode: SstvMode, offset_hz: f32, forced: bool },
    /// Righe dell'immagine in corso aggiornate (vedi [`Decoder::current`]).
    Rows { first: u32, count: u32 },
    Finished(DecodedImage),
    /// Ricezione interrotta senza abbastanza righe da salvare.
    Lost,
}

/// Valori di un gruppo (0-255 per canale).
#[derive(Clone, Debug, Default)]
struct GroupVals {
    r: Vec<f32>,
    g: Vec<f32>,
    b: Vec<f32>,
    y0: Vec<f32>,
    y1: Vec<f32>,
    v: Vec<f32>,
    u: Vec<f32>,
    /// Robot 36: la crominanza di questa riga e' B-Y.
    alt_is_u: bool,
}

struct Rx {
    spec: &'static ModeSpec,
    /// Prodotti del discriminatore, uno per campione.
    buf: Vec<[f32; 2]>,
    offset: f32,
    /// Campioni per gruppo attesi (con la correzione manuale).
    b0: f64,
    syncs: Vec<(f64, f64)>,
    fit: (f64, f64),
    next_group: u32,
    groups: Vec<Option<GroupVals>>,
    image: RgbImage,
    missed: u32,
    phase_known: bool,
}

/// Quanti gruppi di fila senza sincronismo chiudono la ricezione.
const MAX_MISSED: u32 = 20;
/// Parte minima dell'immagine per salvarla quando la ricezione si interrompe.
const MIN_SAVE: f32 = 0.3;

pub struct Decoder {
    cfg: DecoderConfig,
    demod: FmDemod,
    vis: VisDetector,
    history: VecDeque<[f32; 2]>,
    total: u64,
    rx: Option<Rx>,
    scratch: Vec<f32>,
    products: Vec<[f32; 2]>,
}

/// Frequenza media fra le posizioni `t0` e `t1` (campioni frazionari: il campione n copre [n-0,5; n+0,5)): somma pesata dei
/// prodotti del discriminatore, poi l'angolo.
fn mean_between(buf: &[[f32; 2]], t0: f64, t1: f64) -> Option<f32> {
    if buf.is_empty() || t1 <= t0 {
        return None;
    }
    let first = (t0 + 0.5).floor().max(0.0) as usize;
    let last = ((t1 + 0.5).ceil() as usize).min(buf.len());
    let (mut re, mut im, mut weight) = (0.0f64, 0.0f64, 0.0f64);
    for (n, p) in buf.iter().enumerate().take(last).skip(first) {
        let lo = (n as f64 - 0.5).max(t0);
        let hi = (n as f64 + 0.5).min(t1);
        if hi > lo {
            re += f64::from(p[0]) * (hi - lo);
            im += f64::from(p[1]) * (hi - lo);
            weight += hi - lo;
        }
    }
    (weight > 0.0).then(|| product_to_hz(re as f32, im as f32))
}

impl Decoder {
    pub fn new(cfg: DecoderConfig) -> Decoder {
        Decoder { cfg, demod: FmDemod::new(), vis: VisDetector::new(cfg.offset_hz), history: VecDeque::with_capacity(RATE as usize * 2), total: 0, rx: None, scratch: Vec::new(), products: Vec::new() }
    }

    pub fn set_config(&mut self, cfg: DecoderConfig) {
        self.cfg = cfg;
        self.vis.set_manual_offset(cfg.offset_hz);
    }

    pub fn config(&self) -> DecoderConfig {
        self.cfg
    }

    pub fn level_db(&self) -> f32 {
        self.demod.level_db()
    }

    /// Modo e righe complete dell'immagine in corso.
    pub fn current(&self) -> Option<(SstvMode, &RgbImage, u32)> {
        self.rx.as_ref().map(|rx| (rx.spec.mode, &rx.image, (rx.next_group * rx.spec.rows_per_group()).min(rx.spec.height)))
    }

    pub fn sync_quality(&self) -> f32 {
        self.rx.as_ref().map_or(0.0, |rx| if rx.next_group == 0 { 0.0 } else { rx.syncs.len() as f32 / rx.next_group as f32 })
    }

    pub fn offset_hz(&self) -> Option<f32> {
        self.rx.as_ref().map(|rx| rx.offset)
    }

    pub fn is_receiving(&self) -> bool {
        self.rx.is_some()
    }

    /// Butta l'immagine in corso.
    pub fn cancel(&mut self) {
        self.rx = None;
        self.vis.reset();
    }

    /// Parte subito nel modo dato, senza VIS (la fase della riga si cerca dal primo sincronismo).
    pub fn force_start(&mut self, mode: SstvMode) -> DecoderEvent {
        let offset = self.cfg.offset_hz;
        self.rx = Some(self.new_rx(mode, offset, true));
        DecoderEvent::Started { mode, offset_hz: offset, forced: true }
    }

    fn new_rx(&self, mode: SstvMode, offset: f32, forced: bool) -> Rx {
        let spec = mode.spec();
        let b0 = spec.group_seconds() * f64::from(RATE) * (1.0 + f64::from(self.cfg.slant_ppm) * 1e-6);
        let a0 = spec.sync_offset() * f64::from(RATE);
        Rx {
            spec,
            buf: Vec::with_capacity((spec.image_seconds() * f64::from(RATE)) as usize + RATE as usize),
            offset,
            b0,
            syncs: Vec::new(),
            fit: (a0, b0),
            next_group: 0,
            groups: vec![None; spec.groups() as usize],
            image: RgbImage { width: spec.width, height: spec.height, data: vec![0; (spec.width * spec.height * 3) as usize] },
            missed: 0,
            phase_known: !forced,
        }
    }

    /// Audio a 12 kHz in ingresso; ritorna quel che e' successo.
    pub fn push(&mut self, samples: &[f32]) -> Vec<DecoderEvent> {
        let mut events = Vec::new();
        let mut freqs = std::mem::take(&mut self.scratch);
        let mut prods = std::mem::take(&mut self.products);
        freqs.clear();
        prods.clear();
        self.demod.process_full(samples, &mut freqs, Some(&mut prods));
        let first = self.total;
        self.total += freqs.len() as u64;
        for &p in &prods {
            if self.history.len() == RATE as usize {
                self.history.pop_front();
            }
            self.history.push_back(p);
        }
        let hit = self.vis.push(&freqs, first);
        if let Some(rx) = &mut self.rx {
            rx.buf.extend_from_slice(&prods);
        }
        self.scratch = freqs;
        self.products = prods;

        if let Some(hit) = hit
            && self.cfg.lock.is_none_or(|m| m == hit.mode)
        {
            // un VIS nuovo durante una ricezione vale solo se l'immagine di prima ha perso il sincronismo
            let busy = self.rx.as_ref().is_some_and(|rx| rx.missed < 4 && rx.next_group > 0);
            if !busy {
                if let Some(ev) = self.finish(false) {
                    events.push(ev);
                }
                let spec = hit.mode.spec();
                let start = hit.end_sample + (spec.lead_sync() * f64::from(RATE)).round() as u64;
                let mut rx = self.new_rx(hit.mode, hit.offset_hz, false);
                // i campioni dopo il VIS gia' demodulati vengono dalla memoria
                let hist_start = self.total - self.history.len() as u64;
                if start >= hist_start {
                    rx.buf.extend(self.history.iter().skip((start - hist_start) as usize));
                }
                tracing::info!("SSTV: VIS di {} (scarto {:+.0} Hz)", spec.name, hit.offset_hz);
                events.push(DecoderEvent::Started { mode: hit.mode, offset_hz: hit.offset_hz, forced: false });
                self.rx = Some(rx);
            }
        }
        self.advance(&mut events);
        events
    }

    fn advance(&mut self, events: &mut Vec<DecoderEvent>) {
        let Some(rx) = &mut self.rx else { return };
        let mut first_row: Option<u32> = None;
        let mut last_row = 0;
        let groups = rx.spec.groups();
        let rate = f64::from(RATE);
        let sync_len = rx.spec.sync * rate;
        let margin = 0.02 * rate;
        while rx.next_group < groups {
            let g = rx.next_group;
            let (a, b) = rx.fit;
            let predicted = a + b * f64::from(g);
            let needed = if rx.phase_known { predicted + b + margin + sync_len } else { b * f64::from(g + 2) + margin };
            if (rx.buf.len() as f64) < needed {
                break;
            }
            measure_sync(rx, g);
            let vals = decode_group(rx, g);
            rx.groups[g as usize] = Some(vals);
            let (r0, n) = compose(rx, g);
            first_row = Some(first_row.map_or(r0, |f: u32| f.min(r0)));
            last_row = last_row.max(r0 + n);
            rx.next_group += 1;
            if rx.missed >= MAX_MISSED {
                break;
            }
        }
        if let Some(first) = first_row {
            events.push(DecoderEvent::Rows { first, count: last_row - first });
        }
        let lost = rx.missed >= MAX_MISSED;
        if rx.next_group >= groups || lost {
            if let Some(ev) = self.finish(true) {
                events.push(ev);
            }
        }
    }

    /// Chiude la ricezione in corso: `Finished` se c'e' abbastanza immagine, altrimenti `Lost`.
    pub fn finish(&mut self, _natural: bool) -> Option<DecoderEvent> {
        let mut rx = self.rx.take()?;
        self.vis.reset();
        let groups = rx.spec.groups();
        let done = rx.next_group.min(groups);
        let part = done as f32 / groups as f32;
        if part < MIN_SAVE || rx.syncs.len() < 3 {
            tracing::info!("SSTV: ricezione di {} interrotta ({:.0} %)", rx.spec.name, part * 100.0);
            return Some(DecoderEvent::Lost);
        }
        // ridisegno con la retta definitiva dei sincronismi
        for g in 0..done {
            let vals = decode_group(&rx, g);
            rx.groups[g as usize] = Some(vals);
        }
        for g in 0..done {
            compose(&mut rx, g);
        }
        // le righe mai ricevute restano nere: si tagliano
        let rows = (done * rx.spec.rows_per_group()).min(rx.spec.height);
        let mut image = rx.image;
        image.data.truncate((image.width * rows * 3) as usize);
        image.height = rows;
        let sync_quality = rx.syncs.len() as f32 / done.max(1) as f32;
        let clock_ppm = ((rx.fit.1 / (rx.spec.group_seconds() * f64::from(RATE)) - 1.0) * 1e6) as f32;
        tracing::info!("SSTV: immagine {} ricevuta, {} righe, sincronismo {:.0} %, orologio {:+.0} ppm", rx.spec.name, rows, sync_quality * 100.0, clock_ppm);
        Some(DecoderEvent::Finished(DecodedImage { mode: rx.spec.mode, image, sync_quality, offset_hz: rx.offset, clock_ppm, complete: done >= groups }))
    }
}

/// Cerca il sincronismo del gruppo `g` vicino alla posizione prevista e aggiorna la retta.
fn measure_sync(rx: &mut Rx, g: u32) {
    let rate = f64::from(RATE);
    let len = (rx.spec.sync * rate).round().max(1.0) as usize;
    let (a, b) = rx.fit;
    let gf = f64::from(g);
    let (lo, hi) = if rx.phase_known {
        let p = a + b * gf;
        let w = if rx.syncs.len() < 4 { 0.015 * rate } else { 0.006 * rate };
        ((p - w).max(0.0) as usize, (p + w) as usize)
    } else {
        let s = (b * gf).max(0.0) as usize;
        (s, s + b as usize)
    };
    let end = (hi + len + 1).min(rx.buf.len());
    if end <= lo + len {
        rx.missed += 1;
        return;
    }
    // indicatore morbido: 1 a 1200 Hz, 0 dai 1450 Hz in su (il nero a 1500 Hz non conta)
    let off = rx.offset;
    let mut prefix = Vec::with_capacity(end - lo + 1);
    prefix.push(0.0f32);
    let mut acc = 0.0;
    for p in &rx.buf[lo..end] {
        let f = product_to_hz(p[0], p[1]);
        acc += ((1450.0 - (f - off)) / 200.0).clamp(0.0, 1.0);
        prefix.push(acc);
    }
    let (mut best, mut best_at) = (-1.0f32, 0usize);
    for s in 0..=(end - lo - len) {
        let v = prefix[s + len] - prefix[s];
        if v > best {
            best = v;
            best_at = s;
        }
    }
    let score = best / len as f32;
    if score < 0.6 {
        rx.missed += 1;
        return;
    }
    rx.missed = 0;
    let pos = (lo + best_at) as f64;
    if !rx.phase_known {
        rx.phase_known = true;
        rx.fit = (pos - rx.b0 * gf, rx.b0);
    }
    rx.syncs.push((gf, pos));
    refit(rx);
}

/// Retta dei sincronismi ai minimi quadrati, con gli scarti grandi tolti; la pendenza si stima solo con abbastanza punti.
fn refit(rx: &mut Rx) {
    let b0 = rx.b0;
    let fit_once = |pts: &[(f64, f64)]| -> (f64, f64) {
        let n = pts.len() as f64;
        let span = pts.last().map_or(0.0, |l| l.0) - pts.first().map_or(0.0, |f| f.0);
        if pts.len() >= 6 && span >= 8.0 {
            let (sx, sy) = pts.iter().fold((0.0, 0.0), |(x, y), p| (x + p.0, y + p.1));
            let (mx, my) = (sx / n, sy / n);
            let (mut num, mut den) = (0.0, 0.0);
            for p in pts {
                num += (p.0 - mx) * (p.1 - my);
                den += (p.0 - mx) * (p.0 - mx);
            }
            let b = if den > 0.0 { (num / den).clamp(b0 * 0.99, b0 * 1.01) } else { b0 };
            (my - b * mx, b)
        } else {
            let mut d: Vec<f64> = pts.iter().map(|p| p.1 - b0 * p.0).collect();
            d.sort_by(|x, y| x.total_cmp(y));
            (d.get(d.len() / 2).copied().unwrap_or(rx.fit.0), b0)
        }
    };
    let first = fit_once(&rx.syncs);
    let tol = 0.002 * f64::from(RATE);
    let kept: Vec<(f64, f64)> = rx.syncs.iter().copied().filter(|p| (p.1 - (first.0 + first.1 * p.0)).abs() <= tol).collect();
    rx.fit = if kept.len() >= 3 { fit_once(&kept) } else { first };
}

/// I valori dei canali del gruppo `g` letti con la retta attuale.
fn decode_group(rx: &Rx, g: u32) -> GroupVals {
    let spec = rx.spec;
    let rate = f64::from(RATE);
    let (a, b) = rx.fit;
    let scale = b / (spec.group_seconds() * rate);
    let mut t = a + b * f64::from(g) - spec.sync_offset() * rate * scale;
    let w = spec.width as usize;
    let off = rx.offset;
    let mut vals = GroupVals::default();
    for seg in spec.segments() {
        let dur = seg.dur * rate * scale;
        match seg.kind {
            SegKind::RobotSep => {
                // 1500 Hz = R-Y, 2300 Hz = B-Y (modeRobot1 di QSSTV legge la media della pausa)
                let f = mean_between(&rx.buf, t + dur * 0.2, t + dur * 0.8).map_or(1900.0, |f| f - off);
                vals.alt_is_u = f > 1900.0;
            }
            SegKind::Scan(chan) => {
                let px = dur / w as f64;
                let mut line = Vec::with_capacity(w);
                for i in 0..w {
                    let s = t + px * i as f64;
                    let f = mean_between(&rx.buf, s, s + px).unwrap_or(1500.0 + off);
                    line.push(hz_to_value(f - off));
                }
                match chan {
                    Chan::R => vals.r = line,
                    Chan::G => vals.g = line,
                    Chan::B => vals.b = line,
                    Chan::Y0 => vals.y0 = line,
                    Chan::Y1 => vals.y1 = line,
                    Chan::V => vals.v = line,
                    Chan::U => vals.u = line,
                    Chan::Alt => vals.v = line,
                }
            }
            SegKind::Tone(_) => {}
        }
        t += dur;
    }
    if spec.family == Family::Robot36 && vals.alt_is_u {
        vals.u = std::mem::take(&mut vals.v);
    }
    vals
}

/// Scrive nell'immagine le righe che il gruppo `g` completa; ritorna (prima riga, quante).
fn compose(rx: &mut Rx, g: u32) -> (u32, u32) {
    let spec = rx.spec;
    let w = spec.width as usize;
    let gray = vec![127.5f32; w];
    let put = |image: &mut RgbImage, row: u32, px: &dyn Fn(usize) -> [u8; 3]| {
        let base = (row * image.width * 3) as usize;
        for x in 0..w {
            let [r, g, b] = px(x);
            if let Some(dst) = image.data.get_mut(base + x * 3..base + x * 3 + 3) {
                dst.copy_from_slice(&[r, g, b]);
            }
        }
    };
    let Some(vals) = rx.groups.get(g as usize).cloned().flatten() else { return (g, 0) };
    let at = |v: &Vec<f32>, x: usize| v.get(x).copied().unwrap_or(127.5);
    match spec.family {
        Family::Martin | Family::Scottie | Family::Wraase => {
            put(&mut rx.image, g, &|x| [to_u8(at(&vals.r, x)), to_u8(at(&vals.g, x)), to_u8(at(&vals.b, x))]);
            (g, 1)
        }
        Family::Robot72 => {
            put(&mut rx.image, g, &|x| yuv_to_rgb(at(&vals.y0, x), at(&vals.v, x), at(&vals.u, x)));
            (g, 1)
        }
        Family::Pd => {
            put(&mut rx.image, 2 * g, &|x| yuv_to_rgb(at(&vals.y0, x), at(&vals.v, x), at(&vals.u, x)));
            put(&mut rx.image, 2 * g + 1, &|x| yuv_to_rgb(at(&vals.y1, x), at(&vals.v, x), at(&vals.u, x)));
            (2 * g, 2)
        }
        Family::Robot36 => {
            // la coppia di righe (pari R-Y, dispari B-Y) condivide la crominanza
            let pair = g & !1;
            let other = rx.groups.get((g ^ 1) as usize).cloned().flatten();
            let pick = |want_u: bool| -> Vec<f32> {
                for cand in [Some(&vals), other.as_ref()].into_iter().flatten() {
                    let v = if want_u { &cand.u } else { &cand.v };
                    if !v.is_empty() {
                        return v.clone();
                    }
                }
                gray.clone()
            };
            let (v, u) = (pick(false), pick(true));
            let mut rows = 0;
            for row in [pair, pair + 1] {
                let src = if row == g { Some(&vals) } else { other.as_ref() };
                if let Some(src) = src {
                    put(&mut rx.image, row, &|x| yuv_to_rgb(at(&src.y0, x), at(&v, x), at(&u, x)));
                    rows += 1;
                }
            }
            (if other.is_some() { pair } else { g }, rows.max(1))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn weighted_mean_handles_fractions() {
        // prodotti di ampiezza 1 con angoli che danno 1900 + 100 e 1900 - 100 Hz
        let a = std::f32::consts::TAU * 100.0 / RATE as f32;
        let buf = [[a.cos(), a.sin()], [a.cos(), -a.sin()]];
        assert!((mean_between(&buf, 0.5, 1.5).unwrap_or(0.0) - 1800.0).abs() < 0.5);
        assert!((mean_between(&buf, 0.0, 1.0).unwrap_or(0.0) - 1900.0).abs() < 0.5);
    }
}
