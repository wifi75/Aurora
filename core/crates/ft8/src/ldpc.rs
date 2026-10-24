//! Codice LDPC(174,91) di FT8: codifica e decodifica (propagazione di credenze e OSD), come `encode174_91`, `bpdecode174_91`
//! e `osd174_91` di JTDX 2.2.159 (`lib/ft8v2`), con le tabelle di `ldpc_174_91_c_*`.
//!
//! # Convenzione degli LLR
//!
//! Come in WSJT-X e JTDX, un LLR **positivo vuol dire bit 1** e uno negativo bit 0: `llr = ln(P(bit=1) / P(bit=0))`.
//! Il Fortran lo fa cosi' ovunque: `where(zn .gt. 0.) cw=1` nella propagazione di credenze e `where(rx .ge. 0) hdec=1` in OSD.
//! L'ampiezza e' l'affidabilita': piu' e' grande, piu' il bit e' sicuro. Un LLR a zero non da' alcuna informazione (BP lo
//! legge come 0, OSD come 1, come nel Fortran).
//!
//! # Differenze dal Fortran
//!
//! - Le firme non portano `apmask` (maschera dei bit gia' noti): qui nessun bit e' fissato in partenza.
//! - Si restituisce un risultato solo se il CRC torna, e mai la parola di tutti zeri: in JTDX quel controllo sta in `ft8b`
//!   (`count(cw.eq.0).eq.174`), qui sta nel modulo perche' LLR tutti nulli (silenzio) altrimenti "decodificherebbero" il
//!   messaggio zero, che ha CRC valido.
//! - LLR non finiti (NaN, infinito) danno `None`.
//! - OSD usa vettori di bit a 128 bit al posto di matrici di byte: stessi passi, stessi risultati a parte l'ordine delle somme
//!   in virgola mobile e lo spareggio tra bit di pari affidabilita' (qui resta l'ordine di posizione). Se l'eliminazione di
//!   Gauss non trova un pivot entro le prime 111 colonne (`K+20`, mai visto con codeword reali) restituisce `None`.
//! - Nella seconda regola di pre-elaborazione (`npre2`) la tabella di ricerca e' un vettore ordinato per modello, con lo stesso
//!   ordine di inserimento nelle liste e la stessa memoria dell'ultimo modello cercato (`lastpat`/`inext`) di `fetchit91`.

#[path = "ldpc_tables.rs"]
mod ldpc_tables;

use crate::crc;
use ldpc_tables::{BIT_CHECKS, CHECK_BITS, CHECK_WEIGHT, GENERATOR_ROWS};

/// Bit del messaggio con CRC.
const K: usize = crate::K;
/// Bit del codice.
const N: usize = crate::N;
/// Bit di parita' (e controlli).
const M: usize = N - K;
/// Controlli per ogni bit.
const BIT_DEGREE: usize = 3;
/// Quante colonne oltre `K` si provano nell'eliminazione di Gauss di OSD (il "20 ad hoc" del Fortran).
const PIVOT_SLACK: usize = 20;

/// Un codice decodificato.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Decoded {
    /// I 174 bit corretti.
    pub codeword: [u8; 174],
    /// I 91 bit di messaggio e CRC (il CRC torna).
    pub message: [u8; 91],
    /// Iterazioni usate (0 per OSD).
    pub iterations: usize,
}

/// Da 91 bit (messaggio e CRC) ai 174 bit trasmessi: i 91 bit e poi 83 di parita'.
pub fn encode(message: &[u8; 91]) -> [u8; 174] {
    let packed = pack_message(message);
    let mut codeword = [0u8; N];
    for (dst, &src) in codeword.iter_mut().zip(message.iter()) {
        *dst = src & 1;
    }
    for (dst, row) in codeword[K..].iter_mut().zip(GENERATOR_ROWS.iter()) {
        *dst = parity(row & packed);
    }
    codeword
}

/// 91 bit in un intero: il bit `k` del messaggio sta nella posizione `k`.
fn pack_message(message: &[u8; K]) -> u128 {
    message
        .iter()
        .enumerate()
        .fold(0u128, |acc, (k, &b)| acc | (u128::from(b & 1) << k))
}

/// Parita' (0 o 1) del numero di bit a uno.
fn parity(v: u128) -> u8 {
    (v.count_ones() & 1) as u8
}

/// Dai 174 bit ai 91 di messaggio, se il CRC torna e la parola non e' quella di tutti zeri.
fn accept(codeword: &[u8; N]) -> Option<[u8; K]> {
    if codeword.iter().all(|&b| b == 0) {
        return None;
    }
    let mut message = [0u8; K];
    message.copy_from_slice(&codeword[..K]);
    if crc::check(&message) { Some(message) } else { None }
}

/// Approssimazione lineare a tratti di `atanh` di WSJT-X (`platanh.f90`).
fn platanh(x: f32) -> f32 {
    let (sign, z) = if x < 0.0 { (-1.0f32, -x) } else { (1.0f32, x) };
    if z <= 0.664 {
        x / 0.83
    } else if z <= 0.9217 {
        sign * (z - 0.4064) / 0.322
    } else if z <= 0.9951 {
        sign * (z - 0.8378) / 0.0524
    } else if z <= 0.9998 {
        sign * (z - 0.9914) / 0.0012
    } else {
        sign * 7.0
    }
}

/// Propagazione di credenze (log-dominio) sui rapporti di verosimiglianza (LLR, positivo = bit 1, vedi la documentazione del
/// modulo). Come `bpdecode174_91`: a ogni iterazione, da 0 a `max_iterations` compresa, controlla se i segni formano un
/// codeword; se il CRC torna lo restituisce, altrimenti prosegue, con l'uscita anticipata quando i controlli non migliorano.
pub fn bp_decode(llr: &[f32; 174], max_iterations: usize) -> Option<Decoded> {
    if llr.iter().any(|v| !v.is_finite()) {
        return None;
    }
    // messaggi dai controlli ai bit, dai bit ai controlli, e tanh dei secondi
    let mut tov = [[0f32; BIT_DEGREE]; N];
    let mut toc = [[0f32; 7]; M];
    let mut tanhtoc = [[0f32; 7]; M];
    let mut zn = [0f32; N];
    let mut nclast = 0usize;
    let mut ncnt = 0usize;

    for iter in 0..=max_iterations {
        // LLR aggiornato dei bit (all'iterazione 0 i messaggi sono a zero)
        for ((z, &l), t) in zn.iter_mut().zip(llr.iter()).zip(tov.iter()) {
            *z = l + (t[0] + t[1] + t[2]);
        }

        // c'e' un codeword? (si controlla prima di ogni iterazione)
        let mut cw = [0u8; N];
        for (c, &z) in cw.iter_mut().zip(zn.iter()) {
            *c = u8::from(z > 0.0);
        }
        let mut ncheck = 0usize;
        for (bits, &weight) in CHECK_BITS.iter().zip(CHECK_WEIGHT.iter()) {
            let sum: u8 = bits[..usize::from(weight)].iter().map(|&b| cw[usize::from(b)]).sum();
            if !sum.is_multiple_of(2) {
                ncheck += 1;
            }
        }
        if ncheck == 0 {
            // tutti i controlli tornano: se il CRC e' buono e' fatta
            match accept(&cw) {
                Some(message) => {
                    return Some(Decoded { codeword: cw, message, iterations: iter });
                }
                None if cw.iter().all(|&b| b == 0) => return None,
                None => {}
            }
        }

        // uscita anticipata: i controlli insoddisfatti non calano
        if iter > 0 {
            if ncheck < nclast {
                ncnt = 0;
            } else {
                ncnt += 1;
            }
            if ncnt >= 5 && iter >= 10 && ncheck > 15 {
                return None;
            }
        }
        nclast = ncheck;

        // dai bit ai controlli: il bit meno quello che gli aveva detto il controllo stesso
        for (j, row) in toc.iter_mut().enumerate() {
            for (i, slot) in row.iter_mut().enumerate().take(usize::from(CHECK_WEIGHT[j])) {
                let ibj = usize::from(CHECK_BITS[j][i]);
                let mut v = zn[ibj];
                for (kk, &chk) in BIT_CHECKS[ibj].iter().enumerate() {
                    if usize::from(chk) == j {
                        v -= tov[ibj][kk];
                    }
                }
                *slot = v;
            }
        }
        for (t, row) in tanhtoc.iter_mut().zip(toc.iter()) {
            for (dst, &v) in t.iter_mut().zip(row.iter()) {
                *dst = (-v / 2.0).tanh();
            }
        }

        // dai controlli ai bit
        for (j, msgs) in tov.iter_mut().enumerate() {
            for (i, msg) in msgs.iter_mut().enumerate() {
                let chk = usize::from(BIT_CHECKS[j][i]);
                let weight = usize::from(CHECK_WEIGHT[chk]);
                let mut tmn = 1.0f32;
                for t in 0..weight {
                    if usize::from(CHECK_BITS[chk][t]) != j {
                        tmn *= tanhtoc[chk][t];
                    }
                }
                *msg = 2.0 * platanh(-tmn);
            }
        }
    }
    None
}

/// Parametri di OSD per profondita' (`ndeep` del Fortran).
struct OsdPlan {
    /// Ordine massimo delle inversioni sui bit meno affidabili tra i K scelti.
    order: usize,
    /// Prima regola di pre-elaborazione: una inversione in piu' tra i bit piu' affidabili.
    pre1: bool,
    /// Seconda regola (ricerca per sindrome).
    pre2: bool,
    /// Bit di parita' usati dalla seconda regola.
    tau: usize,
}

impl OsdPlan {
    fn for_depth(depth: usize) -> Option<OsdPlan> {
        let (order, pre1, pre2, tau) = match depth {
            0 => return None,
            1 => (1, false, false, 0),
            2 => (1, true, false, 0),
            3 => (1, true, true, 14),
            4 => (2, true, false, 19),
            _ => (2, true, true, 19),
        };
        Some(OsdPlan { order, pre1, pre2, tau })
    }
}

/// Bit di parita' controllati dal filtro di scarto (`nt`).
const OSD_NT: usize = 40;
/// Soglia del filtro di scarto (`ntheta`).
const OSD_NTHETA: u32 = 12;

/// Vettore dei 83 bit di parita' in un intero: la parita' `p` sta nel bit `82 - p` (la prima in cima).
fn parity_bit(p: usize) -> u128 {
    1u128 << (M - 1 - p)
}

/// Il generatore con le colonne in ordine di affidabilita' e ridotto in forma sistematica sui K bit piu' sicuri.
struct Mrb {
    /// `cols[i]`: la colonna `i` (nell'ordine di affidabilita'), un bit per riga di messaggio.
    cols: [u128; N],
    /// `rows[k]`: le parita' (83 bit, vedi `parity_bit`) che il bit di messaggio `k` (nuovo ordine) tocca.
    rows: [u128; K],
    /// Posizione originale di ogni colonna.
    indices: [usize; N],
}

impl Mrb {
    /// Ordina le colonne per affidabilita' decrescente e fa l'eliminazione di Gauss.
    fn new(absrx: &[f32; N]) -> Option<Mrb> {
        let mut indices = [0usize; N];
        for (i, slot) in indices.iter_mut().enumerate() {
            *slot = i;
        }
        // decrescente per affidabilita'; a parita' resta l'ordine di posizione
        indices.sort_by(|&a, &b| absrx[b].total_cmp(&absrx[a]).then(a.cmp(&b)));

        // generatore sistematico [I | P]: la colonna c < K e' la riga c dell'identita'; K + i e' la riga i di parita'
        let mut cols = [0u128; N];
        for (col, &c) in cols.iter_mut().zip(indices.iter()) {
            *col = if c < K { 1u128 << c } else { GENERATOR_ROWS[c - K] };
        }

        for id in 0..K {
            let last = (K + PIVOT_SLACK).min(N);
            let icol = id + cols[id..last].iter().position(|&c| (c >> id) & 1 == 1)?;
            if icol != id {
                cols.swap(id, icol);
                indices.swap(id, icol);
            }
            // azzera la riga `id` in tutte le altre righe: su ogni colonna con il bit `id` acceso, tutte le righe che
            // hanno il bit nella colonna pivot cambiano
            let mask = cols[id] & !(1u128 << id);
            for col in cols.iter_mut() {
                if (*col >> id) & 1 == 1 {
                    *col ^= mask;
                }
            }
        }

        let mut rows = [0u128; K];
        for (k, row) in rows.iter_mut().enumerate() {
            for p in 0..M {
                if (cols[K + p] >> k) & 1 == 1 {
                    *row |= parity_bit(p);
                }
            }
        }
        Some(Mrb { cols, rows, indices })
    }

    /// Parita' (83 bit) del messaggio `me` (un bit per posizione K nel nuovo ordine).
    fn parity_of(&self, me: u128) -> u128 {
        let mut v = 0u128;
        for p in 0..M {
            if parity(self.cols[K + p] & me) == 1 {
                v |= parity_bit(p);
            }
        }
        v
    }
}

/// Somma delle affidabilita' dei bit a uno in `bits` (un bit per posizione, la `k` nel bit `k`).
fn weight_message(bits: u128, absrx: &[f32; N]) -> f32 {
    let mut sum = 0.0;
    for (k, &a) in absrx.iter().enumerate().take(K) {
        if (bits >> k) & 1 == 1 {
            sum += a;
        }
    }
    sum
}

/// Somma delle affidabilita' dei bit di parita' a uno in `bits` (vedi `parity_bit`).
fn weight_parity(bits: u128, absrx: &[f32; N]) -> f32 {
    let mut sum = 0.0;
    for (p, &a) in absrx[K..].iter().enumerate() {
        if bits & parity_bit(p) != 0 {
            sum += a;
        }
    }
    sum
}

/// Prossimo modello di errore di peso `order` (come `nextpat91`): restituisce la posizione del primo bit a uno, o `None`
/// quando i modelli sono finiti.
fn next_pattern(mi: &mut [u8; K], order: usize) -> Option<usize> {
    let mut ind = None;
    for i in 0..K - 1 {
        if mi[i] == 0 && mi[i + 1] == 1 {
            ind = Some(i);
        }
    }
    let ind = ind?;
    let mut ms = [0u8; K];
    ms[..ind].copy_from_slice(&mi[..ind]);
    ms[ind] = 1;
    ms[ind + 1] = 0;
    if ind + 2 < K {
        let ones: usize = ms.iter().map(|&b| usize::from(b)).sum();
        let nz = order.saturating_sub(ones);
        for slot in ms[K - nz..].iter_mut() {
            *slot = 1;
        }
    }
    *mi = ms;
    mi.iter().position(|&b| b == 1)
}

/// Il modello come intero (bit `k` = posizione `k`).
fn pattern_mask(mi: &[u8; K]) -> u128 {
    mi.iter().enumerate().fold(0u128, |acc, (k, &b)| acc | (u128::from(b) << k))
}

/// Primi `ntau` bit di parita' come chiave di ricerca (il primo e' il bit piu' significativo).
fn key(parity_vec: u128, tau: usize) -> u32 {
    (parity_vec >> (M - tau)) as u32
}

/// Decodifica a statistica d'ordine (OSD) per i casi che la propagazione non risolve. Come `osd174_91`, con `order` al posto
/// di `ndeep` (0..=5, i valori piu' grandi valgono 5): ordina i bit per affidabilita', riduce il generatore in modo che i 91
/// piu' sicuri siano il messaggio, e prova le inversioni dei meno sicuri tra quei 91; tiene la parola di distanza minima. Gli
/// LLR seguono la convenzione del modulo (positivo = bit 1).
pub fn osd_decode(llr: &[f32; 174], order: usize) -> Option<Decoded> {
    if llr.iter().any(|v| !v.is_finite()) {
        return None;
    }
    let order = order.min(5);

    let mut absrx_orig = [0f32; N];
    for (a, &l) in absrx_orig.iter_mut().zip(llr.iter()) {
        *a = l.abs();
    }
    let mrb = Mrb::new(&absrx_orig)?;
    let indices = mrb.indices;

    // decisioni dure e affidabilita' nel nuovo ordine
    let mut absrx = [0f32; N];
    let mut m0 = 0u128; // messaggio di ordine zero
    let mut hpar = 0u128; // decisioni dure sulle parita'
    for (i, &orig) in indices.iter().enumerate() {
        absrx[i] = absrx_orig[orig];
        if llr[orig] >= 0.0 {
            if i < K {
                m0 |= 1u128 << i;
            } else {
                hpar |= parity_bit(i - K);
            }
        }
    }

    // distanza da una parola messaggio `me` data la sua parita' esclusa dai confronti
    let distance = |me: u128, par: u128| weight_message(me ^ m0, &absrx) + weight_parity(par ^ hpar, &absrx);

    let mut best = m0;
    let mut dmin = distance(m0, mrb.parity_of(m0));

    if let Some(plan) = OsdPlan::for_depth(order) {
        // prima regola: inversioni di peso 1..=order sugli ultimi bit, piu' (pre1) un bit qualunque prima di loro
        for iorder in 1..=plan.order {
            let mut misub = [0u8; K];
            for slot in misub[K - iorder..].iter_mut() {
                *slot = 1;
            }
            let mut iflag = Some(K - iorder);
            while let Some(first) = iflag {
                let iend = if iorder == plan.order && !plan.pre1 { first } else { 0 };
                let submask = pattern_mask(&misub);
                let mut e2sub = 0u128;
                for n1 in (iend..=first).rev() {
                    let mi = submask | (1u128 << n1);
                    let me = m0 ^ mi;
                    let (e2, nd1kpt);
                    if n1 == first {
                        e2sub = mrb.parity_of(me) ^ hpar;
                        e2 = e2sub;
                        nd1kpt = (e2 >> (M - OSD_NT)).count_ones() + 1;
                    } else {
                        e2 = e2sub ^ mrb.rows[n1];
                        nd1kpt = (e2 >> (M - OSD_NT)).count_ones() + 2;
                    }
                    if nd1kpt <= OSD_NTHETA {
                        let dd = weight_message(mi, &absrx) + weight_parity(e2, &absrx);
                        if dd < dmin {
                            dmin = dd;
                            best = me;
                        }
                    }
                }
                iflag = next_pattern(&mut misub, iorder);
            }
        }

        if plan.pre2 {
            osd_second_rule(&plan, &mrb, m0, hpar, &distance, &mut best, &mut dmin);
        }
    }

    // la parola migliore, riportata all'ordine originale
    let par = mrb.parity_of(best);
    let mut codeword = [0u8; N];
    for (i, &orig) in indices.iter().enumerate() {
        let bit = if i < K { (best >> i) & 1 == 1 } else { par & parity_bit(i - K) != 0 };
        codeword[orig] = u8::from(bit);
    }
    let message = accept(&codeword)?;
    Some(Decoded { codeword, message, iterations: 0 })
}

/// Seconda regola di pre-elaborazione di OSD (`npre2`): cerca coppie di bit da invertire la cui parita' (primi `tau` bit)
/// compensa quella degli errori di una inversione di ordine `order`.
fn osd_second_rule(
    plan: &OsdPlan,
    mrb: &Mrb,
    m0: u128,
    hpar: u128,
    distance: &dyn Fn(u128, u128) -> f32,
    best: &mut u128,
    dmin: &mut f32,
) {
    let tau = plan.tau;
    // tabella: (modello delle prime `tau` parita', coppia i1 > i2 che lo produce), ordinata per modello; l'ordinamento e'
    // stabile, quindi dentro ogni modello resta l'ordine di inserimento
    let mut table: Vec<(u32, (usize, usize))> = Vec::with_capacity(K * (K - 1) / 2);
    for i1 in (0..K).rev() {
        for i2 in (0..i1).rev() {
            table.push((key(mrb.rows[i1] ^ mrb.rows[i2], tau), (i1, i2)));
        }
    }
    table.sort_by_key(|entry| entry.0);
    let need = plan.order + usize::from(plan.pre1) + usize::from(plan.pre2);

    // stato di `fetchit91`: ultimo modello cercato e prossimo elemento della sua lista
    let mut lastpat: Option<u32> = None;
    let mut inext: Option<usize> = None;

    let mut misub = [0u8; K];
    for slot in misub[K - plan.order..].iter_mut() {
        *slot = 1;
    }
    let mut iflag = Some(K - plan.order);
    while iflag.is_some() {
        let submask = pattern_mask(&misub);
        let me = m0 ^ submask;
        let e2sub = mrb.parity_of(me) ^ hpar;
        'tau: for i2 in 0..=tau {
            let ui = if i2 > 0 { parity_bit(i2 - 1) } else { 0 };
            let ipat = key(e2sub ^ ui, tau);
            let lo = table.partition_point(|entry| entry.0 < ipat);
            let hi = table.partition_point(|entry| entry.0 <= ipat);
            let bucket = &table[lo..hi];
            loop {
                // fetchit91
                let found = if lastpat != Some(ipat) && !bucket.is_empty() {
                    inext = if bucket.len() > 1 { Some(1) } else { None };
                    bucket.first().map(|entry| entry.1)
                } else if let Some(pos) = inext.filter(|_| lastpat == Some(ipat)) {
                    inext = if pos + 1 < bucket.len() { Some(pos + 1) } else { None };
                    bucket.get(pos).map(|entry| entry.1)
                } else {
                    inext = None;
                    None
                };
                lastpat = Some(ipat);
                let Some((in1, in2)) = found else { break };

                let mi = submask | (1u128 << in1) | (1u128 << in2);
                if (mi.count_ones() as usize) < need {
                    continue 'tau;
                }
                let me2 = m0 ^ mi;
                let dd = distance(me2, mrb.parity_of(me2));
                if dd < *dmin {
                    *dmin = dd;
                    *best = me2;
                }
            }
        }
        iflag = next_pattern(&mut misub, plan.order);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Instant;

    /// Generatore pseudocasuale deterministico (splitmix64) con variabili gaussiane (Box-Muller).
    struct Rng {
        state: u64,
        spare: Option<f64>,
    }
    impl Rng {
        fn new(seed: u64) -> Rng {
            Rng { state: seed, spare: None }
        }
        fn next_u64(&mut self) -> u64 {
            self.state = self.state.wrapping_add(0x9E37_79B9_7F4A_7C15);
            let mut z = self.state;
            z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
            z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
            z ^ (z >> 31)
        }
        /// Uniforme in (0, 1).
        fn uniform(&mut self) -> f64 {
            ((self.next_u64() >> 11) as f64 + 0.5) / (1u64 << 53) as f64
        }
        fn gauss(&mut self) -> f64 {
            if let Some(v) = self.spare.take() {
                return v;
            }
            let r = (-2.0 * self.uniform().ln()).sqrt();
            let theta = 2.0 * std::f64::consts::PI * self.uniform();
            self.spare = Some(r * theta.sin());
            r * theta.cos()
        }
        fn message77(&mut self) -> [u8; 77] {
            let mut m = [0u8; 77];
            for b in m.iter_mut() {
                *b = ((self.next_u64() >> 40) & 1) as u8;
            }
            m
        }
    }

    fn random_codeword(rng: &mut Rng) -> ([u8; 91], [u8; 174]) {
        let message = crc::append_crc(&rng.message77());
        (message, encode(&message))
    }

    /// LLR di una parola trasmessa in BPSK su canale AWGN con rumore `sigma` (bit 1 = +1): `2 y / sigma^2`, positivo = bit 1.
    fn noisy_llr(codeword: &[u8; 174], sigma: f64, rng: &mut Rng) -> [f32; 174] {
        let mut llr = [0f32; 174];
        for (l, &b) in llr.iter_mut().zip(codeword.iter()) {
            let x = if b == 1 { 1.0 } else { -1.0 };
            let y = x + sigma * rng.gauss();
            *l = (2.0 * y / (sigma * sigma)) as f32;
        }
        llr
    }

    /// Sigma del rumore per un dato Eb/N0 (dB) con codice di rate 91/174.
    fn sigma_for_ebn0(db: f64) -> f64 {
        let ebn0 = 10f64.powf(db / 10.0);
        let esn0 = ebn0 * 91.0 / 174.0;
        (1.0 / (2.0 * esn0)).sqrt()
    }

    #[test]
    fn le_tabelle_sono_coerenti() {
        // ogni bit sta in tre controlli diversi, ogni controllo ha 6 o 7 bit diversi, e le due viste coincidono
        let mut total = 0;
        for (c, bits) in CHECK_BITS.iter().enumerate() {
            let w = usize::from(CHECK_WEIGHT[c]);
            assert!(w == 6 || w == 7);
            total += w;
            for (i, &b) in bits[..w].iter().enumerate() {
                assert!(BIT_CHECKS[usize::from(b)].contains(&(c as u8)));
                assert!(!bits[..i].contains(&b), "bit ripetuto nel controllo {c}");
            }
            assert!(bits[w..].iter().all(|&b| b == 0));
        }
        assert_eq!(total, 3 * N);
        for (j, checks) in BIT_CHECKS.iter().enumerate() {
            assert!(checks[0] != checks[1] && checks[0] != checks[2] && checks[1] != checks[2]);
            for &c in checks {
                let c = usize::from(c);
                let w = usize::from(CHECK_WEIGHT[c]);
                assert!(CHECK_BITS[c][..w].contains(&(j as u8)));
            }
        }
        // 83 righe di generatore da 91 bit
        for row in GENERATOR_ROWS {
            assert!(row >> K == 0);
        }
    }

    /// Le 83 equazioni di parita' (dalle tabelle `Nm`, indipendenti dal generatore) valgono per ogni codeword.
    fn satisfies_all_checks(cw: &[u8; N]) -> bool {
        CHECK_BITS.iter().zip(CHECK_WEIGHT.iter()).all(|(bits, &w)| {
            bits[..usize::from(w)].iter().map(|&b| cw[usize::from(b)]).sum::<u8>() % 2 == 0
        })
    }

    #[test]
    fn la_codifica_soddisfa_tutte_le_parita() {
        let mut rng = Rng::new(10);
        for _ in 0..300 {
            let (message, cw) = random_codeword(&mut rng);
            assert_eq!(&cw[..K], &message[..], "i primi 91 bit sono il messaggio");
            assert!(satisfies_all_checks(&cw));
        }
        // anche senza CRC il codice e' lineare e le equazioni valgono
        for _ in 0..100 {
            let mut m = [0u8; 91];
            for b in m.iter_mut() {
                *b = ((rng.next_u64() >> 33) & 1) as u8;
            }
            assert!(satisfies_all_checks(&encode(&m)));
        }
    }

    #[test]
    fn la_codifica_e_lineare_e_lo_zero_resta_zero() {
        assert!(encode(&[0; 91]).iter().all(|&b| b == 0));
        let mut rng = Rng::new(11);
        let (a, ca) = random_codeword(&mut rng);
        let (b, cb) = random_codeword(&mut rng);
        let mut x = [0u8; 91];
        for ((x, &a), &b) in x.iter_mut().zip(a.iter()).zip(b.iter()) {
            *x = a ^ b;
        }
        let cx = encode(&x);
        for i in 0..N {
            assert_eq!(cx[i], ca[i] ^ cb[i]);
        }
    }

    #[test]
    fn una_parola_sbagliata_non_soddisfa_le_parita() {
        let mut rng = Rng::new(12);
        let (_, cw) = random_codeword(&mut rng);
        for i in 0..N {
            let mut bad = cw;
            bad[i] ^= 1;
            assert!(!satisfies_all_checks(&bad), "bit {i}");
        }
    }

    #[test]
    fn ogni_bit_di_messaggio_tocca_la_parita() {
        for k in 0..K {
            let mut m = [0u8; 91];
            m[k] = 1;
            let w: usize = encode(&m).iter().map(|&b| usize::from(b)).sum();
            assert!(w >= 6, "peso {w} per il bit {k}");
        }
    }

    #[test]
    fn llr_ideali_bp_restituisce_il_messaggio() {
        let mut rng = Rng::new(20);
        for _ in 0..50 {
            let (message, cw) = random_codeword(&mut rng);
            let mut llr = [0f32; N];
            for (l, &b) in llr.iter_mut().zip(cw.iter()) {
                *l = if b == 1 { 8.0 } else { -8.0 };
            }
            let d = bp_decode(&llr, 30).expect("deve decodificare");
            assert_eq!(d.message, message);
            assert_eq!(d.codeword, cw);
            assert_eq!(d.iterations, 0);
            assert!(crc::check(&d.message));
        }
    }

    #[test]
    fn llr_ideali_osd_restituisce_il_messaggio_a_tutti_gli_ordini() {
        let mut rng = Rng::new(21);
        for order in 0..=6 {
            let (message, cw) = random_codeword(&mut rng);
            let mut llr = [0f32; N];
            for (l, &b) in llr.iter_mut().zip(cw.iter()) {
                *l = if b == 1 { 4.0 } else { -4.0 };
            }
            let d = osd_decode(&llr, order).expect("deve decodificare");
            assert_eq!(d.message, message, "ordine {order}");
            assert_eq!(d.codeword, cw);
            assert_eq!(d.iterations, 0);
        }
    }

    #[test]
    fn la_convenzione_positivo_uguale_uno() {
        let mut rng = Rng::new(22);
        let (message, cw) = random_codeword(&mut rng);
        // con il segno opposto non si deve decodificare lo stesso messaggio
        let mut llr = [0f32; N];
        for (l, &b) in llr.iter_mut().zip(cw.iter()) {
            *l = if b == 1 { -6.0 } else { 6.0 };
        }
        assert!(bp_decode(&llr, 30).is_none_or(|d| d.message != message));
        assert!(osd_decode(&llr, 2).is_none_or(|d| d.message != message));
    }

    #[test]
    fn a_snr_alto_decodificano_sempre() {
        let mut rng = Rng::new(30);
        let sigma = sigma_for_ebn0(6.0);
        for _ in 0..100 {
            let (message, cw) = random_codeword(&mut rng);
            let llr = noisy_llr(&cw, sigma, &mut rng);
            let bp = bp_decode(&llr, 30).expect("BP");
            assert_eq!(bp.message, message);
            let osd = osd_decode(&llr, 2).expect("OSD");
            assert_eq!(osd.message, message);
            assert_eq!(osd.codeword, cw);
        }
    }

    #[test]
    fn a_snr_medio_osd_recupera_piu_di_bp() {
        let mut rng = Rng::new(40);
        let sigma = sigma_for_ebn0(2.0);
        let (mut bp_ok, mut osd_ok, mut wrong) = (0, 0, 0);
        let trials = 200;
        for _ in 0..trials {
            let (message, cw) = random_codeword(&mut rng);
            let llr = noisy_llr(&cw, sigma, &mut rng);
            if let Some(d) = bp_decode(&llr, 30) {
                if d.message == message { bp_ok += 1 } else { wrong += 1 }
            }
            if let Some(d) = osd_decode(&llr, 2) {
                if d.message == message { osd_ok += 1 } else { wrong += 1 }
            }
        }
        eprintln!("Eb/N0 2 dB: BP {bp_ok}/{trials}, OSD(2) {osd_ok}/{trials}, errati {wrong}");
        assert!(osd_ok > bp_ok, "OSD {osd_ok} contro BP {bp_ok}");
        assert!(osd_ok >= trials / 2);
        // un CRC valido su un messaggio sbagliato e' possibile ma rarissimo (2^-14 per tentativo)
        assert!(wrong <= 1, "messaggi errati accettati: {wrong}");
    }

    #[test]
    fn piu_profondita_osd_non_peggiora() {
        let mut rng = Rng::new(41);
        let sigma = sigma_for_ebn0(1.0);
        let trials = 60;
        let mut ok = [0usize; 6];
        for _ in 0..trials {
            let (message, cw) = random_codeword(&mut rng);
            let llr = noisy_llr(&cw, sigma, &mut rng);
            for (order, count) in ok.iter_mut().enumerate() {
                if osd_decode(&llr, order).is_some_and(|d| d.message == message) {
                    *count += 1;
                }
            }
        }
        eprintln!("Eb/N0 1 dB, OSD per ordine 0..5 su {trials}: {ok:?}");
        assert!(ok[2] >= ok[0] && ok[5] >= ok[2]);
        assert!(ok[5] > ok[0]);
    }

    #[test]
    fn senza_segnale_non_decodifica_nulla() {
        let mut rng = Rng::new(50);
        for trial in 0..300 {
            let mut llr = [0f32; N];
            for l in llr.iter_mut() {
                *l = (3.0 * rng.gauss()) as f32;
            }
            assert!(bp_decode(&llr, 30).is_none(), "BP, prova {trial}");
            assert!(osd_decode(&llr, 2).is_none(), "OSD(2), prova {trial}");
        }
        for trial in 0..40 {
            let mut llr = [0f32; N];
            for l in llr.iter_mut() {
                *l = (3.0 * rng.gauss()) as f32;
            }
            assert!(osd_decode(&llr, 5).is_none(), "OSD(5), prova {trial}");
        }
    }

    #[test]
    fn llr_nulli_o_non_finiti_non_decodificano() {
        let zeros = [0f32; N];
        assert!(bp_decode(&zeros, 30).is_none());
        for order in 0..=5 {
            assert!(osd_decode(&zeros, order).is_none(), "ordine {order}");
        }
        let mut bad = [1f32; N];
        bad[7] = f32::NAN;
        assert!(bp_decode(&bad, 30).is_none());
        assert!(osd_decode(&bad, 2).is_none());
        bad[7] = f32::INFINITY;
        assert!(bp_decode(&bad, 30).is_none());
        assert!(osd_decode(&bad, 2).is_none());
    }

    #[test]
    fn bp_corregge_bit_invertiti_con_poca_fiducia() {
        let mut rng = Rng::new(60);
        for _ in 0..30 {
            let (message, cw) = random_codeword(&mut rng);
            let mut llr = [0f32; N];
            for (l, &b) in llr.iter_mut().zip(cw.iter()) {
                *l = if b == 1 { 5.0 } else { -5.0 };
            }
            // 8 bit con il segno sbagliato ma poca fiducia
            for _ in 0..8 {
                let i = (rng.next_u64() % N as u64) as usize;
                llr[i] = if cw[i] == 1 { -0.5 } else { 0.5 };
            }
            let d = bp_decode(&llr, 30).expect("BP");
            assert_eq!(d.message, message);
            let d = osd_decode(&llr, 2).expect("OSD");
            assert_eq!(d.message, message);
        }
    }

    #[test]
    fn platanh_come_in_wsjtx() {
        assert_eq!(platanh(0.0), 0.0);
        assert!((platanh(0.5) - 0.5 / 0.83).abs() < 1e-6);
        assert!((platanh(-0.5) + 0.5 / 0.83).abs() < 1e-6);
        assert!((platanh(0.8) - (0.8 - 0.4064) / 0.322).abs() < 1e-5);
        assert!((platanh(0.99) - (0.99 - 0.8378) / 0.0524).abs() < 1e-4);
        assert_eq!(platanh(1.0), 7.0);
        assert_eq!(platanh(-1.0), -7.0);
    }

    #[test]
    fn i_modelli_di_errore_sono_tutti_e_senza_ripetizioni() {
        for order in 1..=2usize {
            let mut mi = [0u8; K];
            for slot in mi[K - order..].iter_mut() {
                *slot = 1;
            }
            let mut seen = std::collections::HashSet::new();
            seen.insert(pattern_mask(&mi));
            let mut flag = Some(K - order);
            while flag.is_some() {
                flag = next_pattern(&mut mi, order);
                if flag.is_some() {
                    assert!(seen.insert(pattern_mask(&mi)));
                    assert_eq!(mi.iter().map(|&b| usize::from(b)).sum::<usize>(), order);
                    assert_eq!(flag, mi.iter().position(|&b| b == 1));
                }
            }
            let expected = if order == 1 { K } else { K * (K - 1) / 2 };
            assert_eq!(seen.len(), expected);
        }
    }

    /// Non e' una prova: stampa i tempi per codeword (`cargo test -p ft8 tempi -- --ignored --nocapture`).
    #[test]
    #[ignore]
    fn tempi_di_decodifica() {
        let mut rng = Rng::new(70);
        let words: Vec<[f32; N]> = (0..50)
            .map(|_| {
                let (_, cw) = random_codeword(&mut rng);
                noisy_llr(&cw, sigma_for_ebn0(2.0), &mut rng)
            })
            .collect();
        let t = Instant::now();
        for w in &words {
            let _ = bp_decode(w, 30);
        }
        eprintln!("BP (30 iterazioni): {:?} per codeword", t.elapsed() / words.len() as u32);
        for order in 0..=5 {
            let t = Instant::now();
            for w in &words {
                let _ = osd_decode(w, order);
            }
            eprintln!("OSD ordine {order}: {:?} per codeword", t.elapsed() / words.len() as u32);
        }
    }
}
