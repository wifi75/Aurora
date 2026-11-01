//! Decodifica "a7": porting di `lib/ft8/ft8_a7.f90` (`ft8_a7_save`, `ft8_a7d`) e della sua chiamata in `lib/ft8_decode.f90`
//! (righe 164-186 e 343-376) di WSJT-X 3.3.0-beta5 (licenza GNU GPL 3, file `COPYING` dei sorgenti; i file non hanno
//! intestazioni proprie).
//!
//! Ogni messaggio decodificato si annota per la sua parita' come "nominativo_1 nominativo_2" (o "CQ nominativo"), con il
//! locatore se c'era, la frequenza e il ritardo. Nel periodo successivo della stessa parita' (30 s dopo), a fine
//! decodifica, per ogni voce si scende a 200 Hz in quel punto, si affinano ritardo e frequenza, si calcolano le metriche dei
//! bit e si confrontano con le parole di codice dei circa 206 messaggi che quella coppia puo' mandarsi dopo (RRR, RR73, 73,
//! CQ, locatore, rapporti). Vince la parola piu' vicina; si accetta se la distanza e' sotto 100 e la seconda e' almeno 1,3
//! volte piu' lontana. Una voce si scarta se nel periodo attuale lo stesso mittente e' gia' stato decodificato entro 3 Hz.
//!
//! # Differenze da WSJT-X
//!
//! - L'SNR si stima dai toni attesi contro i toni opposti (come `ft8b`) invece che dalla linea di base `sbase`, che il
//!   decodificatore di Aurora non calcola.
//! - Il ritardo riportato e' `ibest * 0.005 - 0.5` come negli altri decode di Aurora (WSJT-X usa `ibest - 1`).
//! - Le voci "CQ xx NOMINATIVO" (CQ con parola di 1-2 lettere) si annotano come in WSJT-X ma non si provano: la
//!   lettura di `ft8_decode` le spezzerebbe in nominativi sbagliati.
//! - I messaggi che `pack` scriverebbe come testo libero non si provano.
//! - Rifare la decodifica dello stesso periodo (pulsante «Decodifica») non sposta la tabella: si ricomincia la parte
//!   "attuale" ([`A7Table::begin_period`] con la stessa chiave).

use crate::message::{self, CallHashes};
use crate::sync::{self, Complex, Downsampler, SyncKind};
use crate::{crc, ldpc, modem, NN};

/// Voci per parita' (`MAXDEC`).
const MAXDEC: usize = 200;
/// Campioni utili a 200 Hz per i simboli (`NP2`).
const NP2: i64 = 2812;
/// Mappa Gray: indice (3 bit) -> tono.
const GRAY_MAP: [usize; 8] = [0, 1, 3, 2, 5, 6, 4, 7];

/// Un'ipotesi di messaggio con i suoi bit e toni.
#[derive(Debug, Clone)]
pub(crate) struct Hypothesis {
    pub text: String,
    pub codeword: [u8; 174],
    pub tones: [u8; NN],
}

/// Impacchetta un'ipotesi; `None` se non si impacchetta o diventerebbe testo libero.
pub(crate) fn encode_hypothesis(text: &str, hashes: &mut CallHashes) -> Option<Hypothesis> {
    let bits = message::pack(text, hashes).ok()?;
    if message::message_type(&bits) == (0, 0) {
        return None;
    }
    let codeword = ldpc::encode(&crc::append_crc(&bits));
    let tones = modem::tones_from_codeword(&codeword);
    Some(Hypothesis { text: text.to_string(), codeword, tones })
}

/// Una voce della tabella.
#[derive(Debug, Clone, PartialEq)]
pub struct A7Entry {
    pub dt_s: f32,
    pub freq_hz: f32,
    pub msg: String,
    /// "Non usare": lo stesso mittente e' gia' stato decodificato in questo periodo (`f0 = -98`).
    pub skip: bool,
}

#[derive(Debug, Clone, Default)]
struct Side {
    previous: Vec<A7Entry>,
    current: Vec<A7Entry>,
    key: Option<u64>,
}

/// La tabella "a7" delle due parita'.
#[derive(Debug, Clone, Default)]
pub struct A7Table {
    even: Side,
    odd: Side,
}

fn is_grid4(g: &str) -> bool {
    let b = g.as_bytes();
    b.len() == 4 && (b'A'..=b'R').contains(&b[0]) && (b'A'..=b'R').contains(&b[1]) && b[2].is_ascii_digit() && b[3].is_ascii_digit()
}

impl A7Table {
    fn side_mut(&mut self, even: bool) -> &mut Side {
        if even { &mut self.even } else { &mut self.odd }
    }

    /// Inizio di un periodo: le voci del periodo precedente della stessa parita' diventano quelle da provare. `key` identifica
    /// il periodo (per esempio i quarti di minuto dall'inizio dei tempi): con la stessa chiave di prima la tabella non si
    /// sposta e si svuota soltanto la parte attuale; `None` sposta sempre.
    pub fn begin_period(&mut self, even: bool, key: Option<u64>) {
        let side = self.side_mut(even);
        if key.is_none() || key != side.key {
            side.previous = std::mem::take(&mut side.current);
        } else {
            side.current.clear();
        }
        side.key = key;
    }

    /// `ft8_a7_save`: annota un messaggio decodificato in questo periodo.
    pub fn save(&mut self, even: bool, dt_s: f32, freq_hz: f32, msg: &str) {
        if msg.contains('/') || msg.contains('<') {
            return;
        }
        let w: Vec<&str> = msg.split_whitespace().collect();
        if w.is_empty() || w[0].starts_with("CQ_") {
            return;
        }
        let side = self.side_mut(even);
        if side.current.len() >= MAXDEC {
            return;
        }
        let mut saved = if w.len() >= 2 { format!("{} {}", w[0], w[1]) } else { w[0].to_string() };
        if w[0] == "CQ" && w.len() >= 3 && w[1].len() <= 2 {
            saved = format!("CQ {} {}", w[1], w[2]);
        }
        if let Some(last) = w.last()
            && is_grid4(last)
        {
            saved = format!("{saved} {last}");
        }
        // lo stesso mittente gia' decodificato qui: la voce del periodo precedente non serve piu'
        if let Some(sender) = saved.split_whitespace().nth(1) {
            let needle = format!(" {sender}");
            for e in side.previous.iter_mut().filter(|e| !e.skip) {
                if (freq_hz - e.freq_hz).abs() <= 3.0 && e.msg.find(&needle).is_some_and(|i| i >= 2) {
                    e.skip = true;
                }
            }
        }
        side.current.push(A7Entry { dt_s, freq_hz, msg: saved, skip: false });
    }

    /// Le voci da provare in questo periodo.
    pub fn candidates(&self, even: bool) -> Vec<A7Entry> {
        let side = if even { &self.even } else { &self.odd };
        side.previous.iter().filter(|e| !e.skip && !e.msg.contains('<')).cloned().collect()
    }
}

/// Un messaggio trovato con "a7".
#[derive(Debug, Clone, PartialEq)]
pub struct A7Hit {
    pub text: String,
    pub freq_hz: f32,
    pub dt_s: f32,
    pub snr_db: f32,
    pub hard_errors: u32,
    pub dmin: f32,
    pub tones: [u8; NN],
}

/// Le ipotesi di `ft8_a7d` per la coppia `call_1 call_2` (con il locatore `grid4`, vuoto se ignoto).
fn hypotheses(call_1: &str, call_2: &str, grid4: &str) -> Vec<String> {
    let cq = call_1 == "CQ";
    let std_1 = cq || message::is_standard_call(call_1);
    let std_2 = message::is_standard_call(call_2);
    let mut out = Vec::with_capacity(206);
    for i in 1..=206usize {
        let mut msg = format!("{call_1} {call_2}");
        if cq && i != 5 {
            msg = format!("QU1RK {call_2}");
        }
        let c1 = if cq { "QU1RK" } else { call_1 };
        if !std_1 {
            if i == 1 || i >= 6 {
                msg = format!("<{c1}> {call_2}");
            }
            if (2..=4).contains(&i) {
                msg = format!("{c1} <{call_2}>");
            }
        } else if !std_2 {
            if i <= 4 || i == 6 {
                msg = format!("<{c1}> {call_2}");
            }
            if i >= 7 {
                msg = format!("{c1} <{call_2}>");
            }
        }
        match i {
            2 => msg.push_str(" RRR"),
            3 => msg.push_str(" RR73"),
            4 => msg.push_str(" 73"),
            5 => {
                msg = if std_2 {
                    let mut m = format!("CQ {call_2}");
                    if call_1.len() >= 3 && call_1.as_bytes()[2] == b'_' {
                        m = format!("{call_1} {call_2}");
                    }
                    if grid4 != "RR73" && !grid4.is_empty() {
                        m = format!("{m} {grid4}");
                    }
                    m
                } else {
                    format!("CQ {call_2}")
                };
            }
            6 if std_2 => {
                if !grid4.is_empty() {
                    msg = format!("{msg} {grid4}");
                }
            }
            7..=206 => {
                let isnr = -50 + (i as i32 - 7) / 2;
                let r = crate::a8::report(isnr);
                msg = if i % 2 == 1 { format!("{msg} {r}") } else { format!("{msg} R{r}") };
            }
            _ => {}
        }
        out.push(msg);
    }
    out
}

/// `normalizebmet`: divide per la deviazione standard.
fn normalize(b: &mut [f32; 174]) {
    let n = b.len() as f32;
    let av = b.iter().sum::<f32>() / n;
    let av2 = b.iter().map(|x| x * x).sum::<f32>() / n;
    let var = av2 - av * av;
    let sig = if var > 0.0 { var.sqrt() } else { av2.sqrt() };
    if sig > 0.0 {
        b.iter_mut().for_each(|x| *x /= sig);
    }
}

/// Le quattro serie di metriche dei bit di `ft8_a7d` (simboli singoli, a coppie, a terne, e la normalizzata `bmetd`).
fn bit_metrics(cs: &[[Complex; 8]; NN]) -> [[f32; 174]; 4] {
    let mut bmet = [[0.0f32; 174]; 4];
    for nsym in 1..=3usize {
        let nt = 1usize << (3 * nsym);
        let ibmax = 3 * nsym - 1;
        let mut s2 = vec![0.0f32; nt];
        for ihalf in 0..2usize {
            for k in (1..=29usize).step_by(nsym) {
                let ks = k + 7 + 36 * ihalf - 1; // 0-based
                for (i, v) in s2.iter_mut().enumerate() {
                    let i1 = i / 64;
                    let i2 = (i & 63) / 8;
                    let i3 = i & 7;
                    *v = match nsym {
                        1 => cs[ks][GRAY_MAP[i3]].norm(),
                        2 => (cs[ks][GRAY_MAP[i2]] + cs[ks + 1][GRAY_MAP[i3]]).norm(),
                        _ => (cs[ks][GRAY_MAP[i1]] + cs[ks + 1][GRAY_MAP[i2]] + cs[ks + 2][GRAY_MAP[i3]]).norm(),
                    };
                }
                let i32_ = (k - 1) * 3 + ihalf * 87; // 0-based
                for ib in 0..=ibmax {
                    let idx = i32_ + ib;
                    if idx >= 174 {
                        continue;
                    }
                    let bit = ibmax - ib;
                    let (mut on, mut off) = (f32::MIN, f32::MIN);
                    for (i, &v) in s2.iter().enumerate() {
                        if i >> bit & 1 == 1 {
                            on = on.max(v);
                        } else {
                            off = off.max(v);
                        }
                    }
                    let bm = on - off;
                    match nsym {
                        1 => {
                            bmet[0][idx] = bm;
                            let den = on.max(off);
                            bmet[3][idx] = if den > 0.0 { bm / den } else { 0.0 };
                        }
                        2 => bmet[1][idx] = bm,
                        _ => bmet[2][idx] = bm,
                    }
                }
            }
        }
    }
    for b in bmet.iter_mut() {
        normalize(b);
        b.iter_mut().for_each(|x| *x *= 2.83);
    }
    bmet
}

/// Dalla voce annotata (`call_1 call_2 [grid]`) ai due nominativi e al locatore come li legge `ft8_decode`.
pub(crate) fn split_entry(msg: &str) -> Option<(String, String, String)> {
    let w: Vec<&str> = msg.split_whitespace().collect();
    if w.len() < 2 || w[1].len() <= 2 {
        return None;
    }
    let mut grid4: String = w.get(2).map(|g| g.chars().take(4).collect()).unwrap_or_default();
    if grid4 == "RR73" || grid4.contains('+') || grid4.contains('-') {
        grid4.clear();
    }
    Some((w[0].to_string(), w[1].to_string(), grid4))
}

/// `ft8_a7d`: prova la voce `entry` sul residuo `ds`.
pub fn a7_decode(ds: &Downsampler, entry: &A7Entry, hashes: &CallHashes) -> Option<A7Hit> {
    let (call_1, call_2, grid4) = split_entry(&entry.msg)?;
    let std_2 = message::is_standard_call(&call_2);
    let mut f1 = entry.freq_hz;
    let cd0 = ds.run(f1, false);

    // ritardo: +-10 campioni attorno a quello del periodo precedente
    let i0 = ((entry.dt_s + 0.5) * 200.0).round() as i64;
    let mut ibest = i0;
    let mut smax = 0.0f32;
    for idt in i0 - 10..=i0 + 10 {
        let s = sync::sync8d(&cd0, idt, None, SyncKind::Power, false);
        if s > smax {
            smax = s;
            ibest = idt;
        }
    }
    // frequenza: +-2,5 Hz a passi di 0,5
    let mut delfbest = 0.0f32;
    smax = 0.0;
    for ifr in -5..=5 {
        let delf = ifr as f32 * 0.5;
        let s = sync::sync8d(&cd0, ibest, Some(&sync::freq_tweak(delf)), SyncKind::Power, false);
        if s > smax {
            smax = s;
            delfbest = delf;
        }
    }
    f1 += delfbest;
    let cd0 = ds.run(f1, false);
    let mut best = (f32::MIN, ibest);
    for idt in -4..=4 {
        let s = sync::sync8d(&cd0, ibest + idt, None, SyncKind::Power, false);
        if s > best.0 {
            best = (s, ibest + idt);
        }
    }
    let ibest = best.1;
    let xdt = ibest as f32 * 0.005 - 0.5;

    // spettri dei simboli
    let fft32 = sync::fft_plan(32, false);
    let mut cs = [[Complex::new(0.0, 0.0); 8]; NN];
    let mut s8 = [[0.0f32; 8]; NN];
    for k in 0..NN {
        let i1 = ibest + 32 * k as i64;
        let mut csymb = vec![Complex::new(0.0, 0.0); 32];
        if i1 >= 0 && i1 + 31 < NP2 && ((i1 + 31) as usize) < cd0.len() {
            csymb.copy_from_slice(&cd0[i1 as usize..i1 as usize + 32]);
        }
        fft32.process(&mut csymb);
        for j in 0..8 {
            cs[k][j] = csymb[j] / 1.0e3;
            s8[k][j] = cs[k][j].norm() * 1.0e3;
        }
    }
    let llr = bit_metrics(&cs);

    let mut scratch = hashes.clone();
    let mut dmm: Vec<f32> = Vec::with_capacity(206);
    let mut best_hyp: Option<(Hypothesis, u32)> = None;
    let mut dmin = f32::MAX;
    for text in hypotheses(&call_1, &call_2, &grid4) {
        let Some(h) = encode_hypothesis(&text, &mut scratch) else {
            dmm.push(1.0e30);
            continue;
        };
        let mut dists = [0.0f32; 4];
        for (d, l) in dists.iter_mut().zip(&llr) {
            *d = l.iter().zip(&h.codeword).filter(|(v, c)| u8::from(**v >= 0.0) != **c & 1).map(|(v, _)| v.abs()).sum();
        }
        let (mut which, mut dm) = (0usize, dists[0]);
        for (j, &d) in dists.iter().enumerate().skip(1) {
            if d < dm {
                dm = d;
                which = j;
            }
        }
        dmm.push(dm);
        if dm < dmin {
            dmin = dm;
            let hard = llr[which].iter().zip(&h.codeword).filter(|(v, c)| (2.0 * f32::from(**c & 1) - 1.0) * **v < 0.0).count() as u32;
            best_hyp = Some((h, hard));
        }
    }
    let (hyp, hard) = best_hyp?;
    let mut sorted = dmm.clone();
    sorted.sort_by(f32::total_cmp);
    let dmin2 = sorted.get(1).copied().unwrap_or(1.0e30);
    if dmin > 100.0 || dmin2 / (dmin + 1e-6) < 1.3 {
        return None;
    }
    if hyp.text.starts_with("CQ ") && std_2 && grid4.is_empty() {
        return None;
    }
    if hyp.text.starts_with("QU1RK ") {
        return None;
    }
    // SNR: toni attesi contro i toni opposti (come `ft8b`)
    let (mut sig, mut noi) = (0.0f32, 0.0f32);
    for (row, &tone) in s8.iter().zip(&hyp.tones) {
        let t = usize::from(tone);
        sig += row[t] * row[t];
        noi += row[(t + 4) % 8] * row[(t + 4) % 8];
    }
    let arg = if noi > 0.0 { sig / noi - 1.0 } else { 0.0 };
    let snr = if arg > 0.1 { (10.0 * arg.log10() - 27.0).max(-25.0) } else { -25.0 };
    Some(A7Hit { text: hyp.text, freq_hz: f1, dt_s: xdt, snr_db: snr, hard_errors: hard, dmin, tones: hyp.tones })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn la_tabella_annota_le_coppie_e_scarta_i_mittenti_gia_sentiti() {
        let mut t = A7Table::default();
        t.begin_period(true, Some(10));
        t.save(true, 0.1, 1000.0, "K1ABC W9XYZ EN37");
        t.save(true, 0.2, 1500.0, "CQ IK2ABC JN45");
        t.save(true, 0.2, 1700.0, "K1ABC W9XYZ -10");
        t.save(true, 0.2, 1800.0, "CQ PJ4/K1ABC");
        // stessa parita', periodo dopo
        t.begin_period(true, Some(12));
        let c = t.candidates(true);
        let msgs: Vec<&str> = c.iter().map(|e| e.msg.as_str()).collect();
        assert_eq!(msgs, ["K1ABC W9XYZ EN37", "CQ IK2ABC JN45", "K1ABC W9XYZ"]);
        // W9XYZ decodificato di nuovo a 1001 Hz: la voce a 1000 Hz non serve piu', quella a 1700 resta
        t.save(true, 0.1, 1001.0, "K1ABC W9XYZ R-12");
        let msgs: Vec<String> = t.candidates(true).into_iter().map(|e| e.msg).collect();
        assert_eq!(msgs, ["CQ IK2ABC JN45", "K1ABC W9XYZ"]);
        // la parita' dispari e' separata
        assert!(t.candidates(false).is_empty());
        // ridecodifica dello stesso periodo: la tabella non si sposta
        t.begin_period(true, Some(12));
        assert_eq!(t.candidates(true).len(), 2);
    }

    #[test]
    fn le_ipotesi_seguono_ft8_a7d() {
        let h = hypotheses("K1ABC", "W9XYZ", "EN37");
        assert_eq!(h.len(), 206);
        assert_eq!(h[0], "K1ABC W9XYZ");
        assert_eq!(h[2], "K1ABC W9XYZ RR73");
        assert_eq!(h[4], "CQ W9XYZ EN37");
        assert_eq!(h[5], "K1ABC W9XYZ EN37");
        assert_eq!(h[6], "K1ABC W9XYZ -50");
        let cq = hypotheses("CQ", "IK2ABC", "JN45");
        assert_eq!(cq[0], "QU1RK IK2ABC");
        assert_eq!(cq[4], "CQ IK2ABC JN45");
        assert_eq!(split_entry("K1ABC W9XYZ RR73"), Some(("K1ABC".into(), "W9XYZ".into(), String::new())));
    }
}
