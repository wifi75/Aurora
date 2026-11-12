//! La varicode del PSK decodificata con le probabilità invece che bit per bit.
//!
//! Ogni simbolo arriva come rapporto di verosimiglianza (LLR, logaritmo naturale: positivo = bit 1, nessuna inversione) e un fascio di
//! ipotesi legge la sequenza dei bit come una sequenza di codici varicode validi: un carattere comincia con 1, finisce con 1, non ha mai
//! due zeri di fila e si chiude con «00»; fra i caratteri ci sono solo zeri. Un bit dubbio non rompe il carattere: vince la lettura che
//! spiega meglio tutti i bit insieme, con un modello del testo leggero (coppie di caratteri). Il testo esce con due caratteri di ritardo
//! (o dopo una pausa), e ogni carattere ha la sua probabilità: lo squelch «a probabilità» non stampa quelli incerti, e sul solo rumore non
//! esce quasi niente.

use crate::varicode;
use std::collections::HashMap;

/// Ipotesi tenute a ogni bit.
const BEAM: usize = 96;
/// Ipotesi più indietro di così (logaritmo naturale) si scartano.
const BEAM_DROP: f32 = 25.0;
/// Caratteri che devono seguire quello da stampare, e bit d'attesa massima.
const DELAY_CHARS: usize = 2;
const DELAY_BITS: u32 = 24;
/// Probabilità che dopo una pausa cominci un carattere (a ogni bit di pausa).
const P_START: f32 = 0.1;
/// Costo di un carattere abbandonato a meta' (troppi uni di fila, la coda di portante): l'ipotesi torna fra i caratteri senza scrivere.
const ABORT_COST: f32 = 12.0;
/// Peso di base del modello del testo.
pub const LM_WEIGHT_DEFAULT: f32 = 0.3;
/// Probabilità minima (0-255) perché un carattere si stampi (squelch a probabilità); sotto, sul rumore, si tace.
pub const CONF_PRINT_DEFAULT: u8 = 140;

const NONE: u16 = u16::MAX;

/// L'albero dei 256 codici varicode: figlio 0 e 1, e il carattere dei nodi che chiudono un codice.
struct Tree {
    child: Vec<[u16; 2]>,
    leaf: Vec<Option<u8>>,
}

impl Tree {
    fn new() -> Self {
        let mut t = Tree { child: vec![[NONE; 2]], leaf: vec![None] };
        for c in 0..=255u8 {
            let code = varicode::encode(c);
            let mut n = 0usize;
            for b in code.bytes() {
                let k = usize::from(b == b'1');
                if t.child[n][k] == NONE {
                    t.child[n][k] = t.child.len() as u16;
                    t.child.push([NONE; 2]);
                    t.leaf.push(None);
                }
                n = usize::from(t.child[n][k]);
            }
            t.leaf[n] = Some(c);
        }
        t
    }
}

/// Il modello del testo: coppie di caratteri contate su un testo tipico del PSK (minuscole, maiuscole, cifre), lisciate.
struct Lm {
    logp: Vec<f32>,
}

const LM_CORPUS: &str = "cq cq cq de iu3abc iu3abc pse k \
iu3abc de k1abc k1abc hello and thanks for the call. my name is joe and my qth is boston. ur rst 599 599. hw copy? btu \
k1abc de iu3abc thanks joe for the nice report. name here is tony, qth near milano, locator io91wm. rig is 100 watts into a dipole. \
the weather is sunny and warm today. how are you? 73 and good dx. iu3abc de k1abc sk \
CQ CQ DE IU3ABC IU3ABC PSE K. TU 599 73. CQ TEST IU3ABC. \
the quick brown fox jumps over the lazy dog. we are working psk31 on twenty meters with a very low power station. \
thank you very much for the qso, hope to see you again soon on the bands. best regards to you and your family. \
my computer runs aurora and the antenna is a vertical. please qsl via bureau or lotw, i will send my card. \
";

impl Lm {
    fn new() -> Self {
        let mut counts = vec![0.0_f32; 128 * 128];
        let bytes: Vec<u8> = std::iter::once(b' ').chain(LM_CORPUS.bytes()).collect();
        for w in bytes.windows(2) {
            counts[usize::from(w[0] & 0x7f) * 128 + usize::from(w[1] & 0x7f)] += 1.0;
        }
        // tutti i 128 caratteri restano possibili (mezzo conteggio ciascuno); quelli fuori dall'ASCII valgono come lo spazio di contesto
        let mut logp = vec![0.0_f32; 128 * 128];
        for a in 0..128 {
            let row = &counts[a * 128..a * 128 + 128];
            let total: f32 = row.iter().map(|c| c + 0.5).sum();
            for b in 0..128 {
                logp[a * 128 + b] = ((row[b] + 0.5) / total).ln();
            }
        }
        Lm { logp }
    }

    fn score(&self, prev: u8, next: u8) -> f32 {
        self.logp[usize::from(prev & 0x7f) * 128 + usize::from(next & 0x7f)]
    }
}

#[derive(Clone, Copy)]
struct Hyp {
    score: f32,
    /// Il nodo dell'albero (0 = fra i caratteri).
    node: u16,
    /// L'ultimo bit era 0 (dentro un carattere: puo' essere la fine).
    zero: bool,
    prev: u8,
    len: u8,
    text: [u8; 12],
    /// Da quale bit aspetta il primo carattere in sospeso.
    since: u32,
}

impl Hyp {
    fn key(&self) -> u32 {
        u32::from(self.node) | (u32::from(self.zero) << 16) | (u32::from(self.prev & 0x7f) << 17)
    }

    fn push(&mut self, c: u8, now: u32) -> bool {
        if usize::from(self.len) >= self.text.len() {
            return false;
        }
        if self.len == 0 {
            self.since = now;
        }
        self.text[usize::from(self.len)] = c;
        self.len += 1;
        true
    }
}

/// log P(bit) dal rapporto di verosimiglianza: −log(1 + e^(∓L)).
fn ln_bit(llr: f32, one: bool) -> f32 {
    let x = if one { -llr } else { llr };
    if x > 30.0 { -x } else { -(x.exp()).ln_1p() }
}

pub struct PskViterbi {
    tree: Tree,
    lm: Lm,
    lm_weight: f32,
    min_conf: u8,
    beam: Vec<Hyp>,
    cand: Vec<Hyp>,
    index: HashMap<u32, usize>,
    bits: u32,
    confidences: Vec<u8>,
}

impl Default for PskViterbi {
    fn default() -> Self {
        Self::new()
    }
}

impl PskViterbi {
    pub fn new() -> Self {
        let mut d = PskViterbi {
            tree: Tree::new(),
            lm: Lm::new(),
            lm_weight: LM_WEIGHT_DEFAULT,
            min_conf: CONF_PRINT_DEFAULT,
            beam: Vec::with_capacity(BEAM * 2),
            cand: Vec::with_capacity(BEAM * 4),
            index: HashMap::new(),
            bits: 0,
            confidences: Vec::new(),
        };
        d.reset();
        d
    }

    /// Ricomincia fra i caratteri (inizio di un segnale).
    pub fn reset(&mut self) {
        self.beam.clear();
        self.beam.push(Hyp { score: 0.0, node: 0, zero: true, prev: b' ', len: 0, text: [0; 12], since: 0 });
    }

    pub fn set_lm_weight(&mut self, w: f32) {
        self.lm_weight = w.max(0.0);
    }

    /// Probabilità minima (0-255) di un carattere per stamparlo; 0 = si stampa tutto.
    pub fn set_min_confidence(&mut self, c: u8) {
        self.min_conf = c;
    }

    /// Le probabilità (0-255) dei caratteri usciti dall'ultima volta (anche di quelli non stampati perche' incerti, che valgono 0).
    pub fn take_confidences(&mut self) -> Vec<u8> {
        std::mem::take(&mut self.confidences)
    }

    fn add(&mut self, h: Hyp) {
        match self.index.get(&h.key()) {
            Some(&i) => {
                if h.score > self.cand[i].score {
                    self.cand[i] = h;
                }
            }
            None => {
                self.index.insert(h.key(), self.cand.len());
                self.cand.push(h);
            }
        }
    }

    /// Un bit con il suo rapporto di verosimiglianza; i caratteri decisi vanno in `out`.
    pub fn push(&mut self, llr: f32, out: &mut Vec<u8>) {
        self.bits = self.bits.wrapping_add(1);
        let now = self.bits;
        let (l1, l0) = (ln_bit(llr, true), ln_bit(llr, false));
        self.cand.clear();
        self.index.clear();
        let beam = std::mem::take(&mut self.beam);
        for h in &beam {
            if h.node == 0 {
                // fra i caratteri: altro zero, o comincia un carattere (sempre con 1)
                self.add(Hyp { score: h.score + l0 + (1.0 - P_START).ln(), ..*h });
                let n = self.tree.child[0][1];
                if n != NONE {
                    self.add(Hyp { score: h.score + l1 + P_START.ln(), node: n, zero: false, ..*h });
                }
                continue;
            }
            let node = usize::from(h.node);
            if h.zero {
                // dopo uno zero: un altro zero chiude il carattere, un uno lo continua con «01»
                if let Some(c) = self.tree.leaf[node] {
                    let mut e = Hyp { score: h.score + l0 + self.lm_weight * self.lm.score(h.prev, c), node: 0, zero: true, prev: c, ..*h };
                    if e.push(c, now) {
                        self.add(e);
                    }
                }
                let z = self.tree.child[node][0];
                if z != NONE {
                    let o = self.tree.child[usize::from(z)][1];
                    if o != NONE {
                        self.add(Hyp { score: h.score + l1, node: o, zero: false, ..*h });
                    }
                }
            } else {
                // dopo un uno: un uno continua, uno zero puo' essere fine o meta' di «01»
                let o = self.tree.child[node][1];
                if o != NONE {
                    self.add(Hyp { score: h.score + l1, node: o, ..*h });
                } else {
                    // nessun codice continua cosi': il carattere si perde e si torna fra i caratteri
                    self.add(Hyp { score: h.score + l1 - ABORT_COST, node: 0, zero: true, ..*h });
                }
                if self.tree.leaf[node].is_some() || self.tree.child[node][0] != NONE {
                    self.add(Hyp { score: h.score + l0, zero: true, ..*h });
                }
            }
        }
        let best = self.cand.iter().map(|h| h.score).fold(f32::MIN, f32::max);
        self.cand.retain(|h| h.score > best - BEAM_DROP);
        if self.cand.len() > BEAM {
            self.cand.select_nth_unstable_by(BEAM - 1, |a, b| b.score.total_cmp(&a.score));
            self.cand.truncate(BEAM);
        }
        for h in self.cand.iter_mut() {
            h.score -= best;
        }
        self.beam = std::mem::take(&mut self.cand);
        if self.beam.is_empty() {
            self.reset();
        }
        self.commit(out);
    }

    fn commit(&mut self, out: &mut Vec<u8>) {
        loop {
            let Some(best) = self.beam.iter().max_by(|a, b| a.score.total_cmp(&b.score)).copied() else { return };
            let waited = self.bits.wrapping_sub(best.since);
            if best.len == 0 || (usize::from(best.len) <= DELAY_CHARS && waited < DELAY_BITS) {
                return;
            }
            let c = best.text[0];
            let (mut agree, mut all) = (0.0_f64, 0.0_f64);
            for h in &self.beam {
                let w = f64::from(h.score - best.score).exp();
                all += w;
                if h.len > 0 && h.text[0] == c {
                    agree += w;
                }
            }
            let now = self.bits;
            self.beam.retain(|h| h.len > 0 && h.text[0] == c);
            for h in self.beam.iter_mut() {
                h.text.copy_within(1..usize::from(h.len), 0);
                h.len -= 1;
                h.since = now;
            }
            let conf = (255.0 * agree / all.max(1e-30)).round().clamp(0.0, 255.0) as u8;
            if conf >= self.min_conf {
                out.push(c);
                self.confidences.push(conf);
            }
        }
    }

    /// Fine del segnale: stampa quello che resta del percorso migliore.
    pub fn flush(&mut self, out: &mut Vec<u8>) {
        if let Some(best) = self.beam.iter().max_by(|a, b| a.score.total_cmp(&b.score)).copied() {
            for &c in &best.text[..usize::from(best.len)] {
                out.push(c);
                self.confidences.push(255);
            }
        }
        self.reset();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bits_of(text: &[u8]) -> Vec<bool> {
        let mut v = vec![false; 8];
        for &c in text {
            v.extend(varicode::encode(c).bytes().map(|b| b == b'1'));
            v.extend([false, false]);
        }
        v.extend([false; 50]);
        v
    }

    fn run(bits: &[bool], flip: &[usize], strength: f32) -> String {
        let mut d = PskViterbi::new();
        d.set_min_confidence(0);
        let mut out = Vec::new();
        for (i, &b) in bits.iter().enumerate() {
            let mut l = if b { strength } else { -strength };
            if flip.contains(&i) {
                // un bit sbagliato ma poco sicuro
                l = -0.1 * l;
            }
            d.push(l, &mut out);
        }
        String::from_utf8_lossy(&out).into_owned()
    }

    #[test]
    fn clean_bits_read_back() {
        let text = b"cq cq de iu3abc pse k 599 73!";
        assert_eq!(run(&bits_of(text), &[], 6.0), String::from_utf8_lossy(text));
    }

    #[test]
    fn an_unsure_wrong_bit_is_corrected() {
        let text = b"the quick brown fox";
        let bits = bits_of(text);
        // un bit dentro «quick» e uno dentro «brown»
        let got = run(&bits, &[40, 75], 4.0);
        assert_eq!(got, "the quick brown fox");
    }

    #[test]
    fn noise_prints_little() {
        let mut d = PskViterbi::new();
        let mut out = Vec::new();
        let mut x = 7u64;
        for _ in 0..3000 {
            x ^= x << 13;
            x ^= x >> 7;
            x ^= x << 17;
            // sul rumore i rapporti di verosimiglianza sono piccoli (la scala viene dai momenti, che col rumore danno quasi zero)
            let g = ((x >> 11) as f32 / (1u64 << 53) as f32 - 0.5) * 0.6;
            d.push(g, &mut out);
        }
        assert!(out.len() < 15, "dal rumore: {:?}", String::from_utf8_lossy(&out));
    }
}
