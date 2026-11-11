//! Codici LDPC (240,101) di FST4 e (240,74) di FST4W con il CRC a 24 bit: codifica, decodifica a propagazione di fiducia e OSD.
//! Porting di WSJT-X 3.3.0-beta6 (GPL 3), `lib/fst4/`: `get_crc24.f90`, `encode240_101.f90`, `encode240_74.f90`,
//! `decode240_101.f90`, `decode240_74.f90`, `osd240_101.f90`, `fastosd240_74.f90`; `indexx.f90` per l'ordinamento.
//!
//! Le parole sono vettori di bit (0/1) nell'ordine di WSJT-X; gli indici restituiti al chiamante partono da 0.

// porting indice per indice del Fortran: i cicli sugli indici restano come nell'originale
#![allow(clippy::needless_range_loop)]

use crate::fst4tables::{GEN_74, GEN_101, MN_74, MN_101, NM_74, NM_101, NRW_74, NRW_101};

/// Lunghezza della parola di codice.
pub const N: usize = 240;

/// Quale dei due codici.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Code {
    /// (240,101): 77 bit di messaggio + CRC24 (FST4).
    C101,
    /// (240,74): 50 bit di messaggio + CRC24 (FST4W).
    C74,
}

impl Code {
    /// Bit d'informazione (messaggio + CRC).
    pub fn k(self) -> usize {
        match self {
            Code::C101 => 101,
            Code::C74 => 74,
        }
    }
    /// Bit di messaggio senza CRC.
    fn payload(self) -> usize {
        self.k() - 24
    }
}

/// `get_crc24.f90`: il resto della divisione per il polinomio 0x100065b dei primi `len` bit (gli ultimi 24 a zero per calcolarlo,
/// con il CRC ricevuto per controllarlo: allora vale 0 se torna).
pub fn crc24(mc: &[u8], len: usize) -> u32 {
    const P: [u8; 25] = [1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 0, 0, 1, 0, 1, 1, 0, 1, 1];
    if len < 25 || mc.len() < len {
        return u32::MAX;
    }
    let mut r = [0u8; 25];
    r.copy_from_slice(&mc[..25]);
    for i in 0..=(len - 25) {
        r[24] = mc[i + 24];
        let r0 = r[0];
        for (x, p) in r.iter_mut().zip(P) {
            *x = (*x + r0 * p) % 2;
        }
        r.rotate_left(1);
    }
    r[..24].iter().fold(0u32, |a, &b| (a << 1) | u32::from(b))
}

/// Mette il CRC24 negli ultimi 24 bit di `msg` (lunghezza 101 o 74), come `genfst4.f90:85-95`.
pub fn append_crc24(msg: &mut [u8]) {
    let len = msg.len();
    if len < 25 {
        return;
    }
    msg[len - 24..].iter_mut().for_each(|b| *b = 0);
    let c = crc24(msg, len);
    for i in 0..24 {
        msg[len - 24 + i] = ((c >> (23 - i)) & 1) as u8;
    }
}

/// Matrice di parita' (righe M x K) letta dalle stringhe esadecimali, come il primo giro di `encode240_101.f90`/`encode240_74.f90`.
fn parity_matrix(code: Code) -> Vec<Vec<u8>> {
    let (rows, k): (&[&str], usize) = match code {
        Code::C101 => (&GEN_101, 101),
        Code::C74 => (&GEN_74, 74),
    };
    rows.iter()
        .map(|s| {
            let mut row = vec![0u8; k];
            for (j, ch) in s.chars().enumerate() {
                let v = ch.to_digit(16).unwrap_or(0);
                for jj in 0..4 {
                    let col = j * 4 + jj;
                    if col < k && (v >> (3 - jj)) & 1 == 1 {
                        row[col] = 1;
                    }
                }
            }
            row
        })
        .collect()
}

/// Tavole del grafo del codice (indici da 0).
struct Graph {
    mn: Vec<[usize; 3]>,
    nm: Vec<Vec<usize>>,
}

fn graph(code: Code) -> Graph {
    let (mn, nm): (Vec<[usize; 3]>, Vec<Vec<usize>>) = match code {
        Code::C101 => (
            MN_101.iter().map(|r| r.map(|x| usize::from(x) - 1)).collect(),
            NM_101.iter().zip(NRW_101).map(|(r, n)| r[..usize::from(n)].iter().map(|&x| usize::from(x) - 1).collect()).collect(),
        ),
        Code::C74 => (
            MN_74.iter().map(|r| r.map(|x| usize::from(x) - 1)).collect(),
            NM_74.iter().zip(NRW_74).map(|(r, n)| r[..usize::from(n)].iter().map(|&x| usize::from(x) - 1).collect()).collect(),
        ),
    };
    Graph { mn, nm }
}

/// `platanh` di `bpdecode144.f90`: arcotangente iperbolica a tratti.
fn platanh(x: f32) -> f32 {
    let (sign, z) = if x < 0.0 { (-1.0, -x) } else { (1.0, x) };
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

/// `indexx.f90` (quicksort di Numerical Recipes, crescente): copiato alla lettera perche' l'ordine dei pari decide le colonne
/// dell'OSD con i bit a priori, che hanno tutti la stessa ampiezza. Restituisce indici da 0.
fn indexx(arr: &[f32]) -> Vec<usize> {
    const M: usize = 7;
    let n = arr.len();
    // indici da 1 come in Fortran, posizione 0 inutilizzata
    let mut indx: Vec<usize> = (0..=n).collect();
    let mut istack: Vec<usize> = Vec::with_capacity(64);
    let a_of = |indx: &Vec<usize>, i: usize| arr[indx[i] - 1];
    let mut l = 1usize;
    let mut ir = n;
    loop {
        if ir < l + M {
            for j in (l + 1)..=ir {
                let indxt = indx[j];
                let a = arr[indxt - 1];
                let mut i = j - 1;
                let mut placed = false;
                while i >= 1 {
                    if a_of(&indx, i) <= a {
                        indx[i + 1] = indxt;
                        placed = true;
                        break;
                    }
                    indx[i + 1] = indx[i];
                    i -= 1;
                }
                if !placed {
                    indx[1] = indxt;
                }
            }
            let (Some(r), Some(ll)) = (istack.pop(), istack.pop()) else { break };
            ir = r;
            l = ll;
        } else {
            let k = (l + ir) / 2;
            indx.swap(k, l + 1);
            if a_of(&indx, l + 1) > a_of(&indx, ir) {
                indx.swap(l + 1, ir);
            }
            if a_of(&indx, l) > a_of(&indx, ir) {
                indx.swap(l, ir);
            }
            if a_of(&indx, l + 1) > a_of(&indx, l) {
                indx.swap(l + 1, l);
            }
            let mut i = l + 1;
            let mut j = ir;
            let indxt = indx[l];
            let a = arr[indxt - 1];
            loop {
                i += 1;
                while a_of(&indx, i) < a {
                    i += 1;
                }
                j -= 1;
                while a_of(&indx, j) > a {
                    j -= 1;
                }
                if j < i {
                    break;
                }
                indx.swap(i, j);
            }
            indx[l] = indx[j];
            indx[j] = indxt;
            if ir + 1 - i >= j - l {
                istack.push(i);
                istack.push(ir);
                ir = j - 1;
            } else {
                istack.push(l);
                istack.push(j - 1);
                l = i;
            }
        }
    }
    indx[1..].iter().map(|&x| x - 1).collect()
}

/// Esito di una decodifica.
#[derive(Debug, Clone)]
pub struct Decoded {
    /// I bit d'informazione (101 o 74, con il CRC).
    pub message: Vec<u8>,
    /// La parola di codice trovata.
    pub cw: [u8; N],
    /// 1 = propagazione di fiducia, 2.. = OSD (passo + 1), 0 = niente.
    pub ntype: i32,
    /// Errori rispetto alle decisioni dure dei bit ricevuti (negativo = niente).
    pub nharderror: i32,
    pub dmin: f32,
}

/// Il motore di codifica e decodifica di un codice, con le matrici dell'OSD gia' pronte per ogni `Keff`
/// (`fst4_osd_workspace.f90`).
pub struct Ldpc {
    code: Code,
    parity: Vec<Vec<u8>>,
    graph: Graph,
    generators: Vec<Option<Vec<[u8; N]>>>,
    boxes: Boxes,
}

impl Ldpc {
    pub fn new(code: Code) -> Ldpc {
        Ldpc { code, parity: parity_matrix(code), graph: graph(code), generators: vec![None; 102], boxes: Boxes::default() }
    }

    pub fn code(&self) -> Code {
        self.code
    }

    /// `encode240_101`/`encode240_74`: i K bit d'informazione seguiti dai bit di parita'.
    pub fn encode(&self, message: &[u8]) -> [u8; N] {
        let k = self.code.k();
        let mut cw = [0u8; N];
        for (c, m) in cw.iter_mut().zip(message.iter().take(k)) {
            *c = *m & 1;
        }
        for (i, row) in self.parity.iter().enumerate() {
            let s: u32 = row.iter().zip(message.iter()).map(|(g, m)| u32::from(g & m & 1)).sum();
            cw[k + i] = (s % 2) as u8;
        }
        cw
    }

    /// `decode240_101_owned`/`decode240_74_owned`: propagazione di fiducia (30 iterazioni, una con `Keff` = 50 nel codice da 74)
    /// e poi l'OSD sulle somme salvate dei primi `maxosd` giri (0 = sulle llr del canale).
    pub fn decode(&mut self, llr: &[f32; N], keff: usize, maxosd: i32, norder: i32, apmask: &[u8; N]) -> Decoded {
        let k = self.code.k();
        let m = N - k;
        let maxosd = maxosd.min(3);
        let maxiterations = if self.code == Code::C74 && keff == 50 { 1 } else { 30 };
        let nosd: usize = if maxosd == 0 { 1 } else if maxosd > 0 { maxosd as usize } else { 0 };
        let mut zsave = vec![[0f32; N]; 3];
        if maxosd == 0 {
            zsave[0] = *llr;
        }
        let failed = |cw| Decoded { message: vec![0; k], cw, ntype: 0, nharderror: -1, dmin: 0.0 };
        let hard_quality = |cw: &[u8; N]| {
            let mut nh = 0i32;
            let mut d = 0f32;
            for i in 0..N {
                let h = u8::from(llr[i] >= 0.0);
                if h != cw[i] {
                    nh += 1;
                    d += llr[i].abs();
                }
            }
            (nh, d)
        };
        let mut cw = [0u8; N];
        if maxosd != 0 || self.code == Code::C101 {
            let g = &self.graph;
            let mut toc: Vec<Vec<f32>> = g.nm.iter().map(|r| r.iter().map(|&b| llr[b]).collect()).collect();
            let mut tov = vec![[0f32; 3]; N];
            let mut tanhtoc: Vec<Vec<f32>> = g.nm.iter().map(|r| vec![0f32; r.len()]).collect();
            let mut zn = [0f32; N];
            let mut zsum = [0f32; N];
            let mut ncnt = 0;
            let mut nclast = 0usize;
            for iter in 0..=maxiterations {
                for i in 0..N {
                    zn[i] = if apmask[i] != 1 { llr[i] + tov[i].iter().sum::<f32>() } else { llr[i] };
                }
                for i in 0..N {
                    zsum[i] += zn[i];
                }
                if iter > 0 && maxosd > 0 && iter <= maxosd as usize {
                    zsave[iter - 1] = zsum;
                }
                for i in 0..N {
                    cw[i] = u8::from(zn[i] > 0.0);
                }
                let ncheck = g.nm.iter().filter(|r| r.iter().map(|&b| u32::from(cw[b])).sum::<u32>() % 2 != 0).count();
                if ncheck == 0 && crc24(&cw[..k], k) == 0 {
                    let (nh, d) = hard_quality(&cw);
                    return Decoded { message: cw[..k].to_vec(), cw, ntype: 1, nharderror: nh, dmin: d };
                }
                if iter > 0 {
                    if ncheck < nclast {
                        ncnt = 0;
                    } else {
                        ncnt += 1;
                    }
                    if ncnt >= 5 && iter >= 10 && ncheck > 15 {
                        break;
                    }
                }
                nclast = ncheck;
                for (j, r) in g.nm.iter().enumerate() {
                    for (i, &ibj) in r.iter().enumerate() {
                        let mut t = zn[ibj];
                        for kk in 0..3 {
                            if g.mn[ibj][kk] == j {
                                t -= tov[ibj][kk];
                            }
                        }
                        toc[j][i] = t;
                    }
                }
                for j in 0..m {
                    for (t, &x) in tanhtoc[j].iter_mut().zip(&toc[j]) {
                        *t = (-x / 2.0).tanh();
                    }
                }
                for j in 0..N {
                    for i in 0..3 {
                        let ichk = g.mn[j][i];
                        let mut tmn = 1f32;
                        for (&b, &t) in g.nm[ichk].iter().zip(&tanhtoc[ichk]) {
                            if b != j {
                                tmn *= t;
                            }
                        }
                        tov[j][i] = 2.0 * platanh(-tmn);
                    }
                }
            }
        }
        for (i, zs) in zsave.iter().take(nosd).enumerate() {
            let r = match self.code {
                Code::C101 => self.osd101(zs, keff, apmask, norder),
                Code::C74 => self.fastosd74(zs, keff, apmask, norder),
            };
            if let Some((c, nh)) = r {
                // l'OSD riscrive `cw` anche quando non trova nulla (`decode240_101.f90:155`)
                cw = c;
                if nh > 0 {
                    let (nh2, d) = hard_quality(&c);
                    return Decoded { message: c[..k].to_vec(), cw: c, ntype: 2 + i as i32, nharderror: nh2, dmin: d };
                }
            }
        }
        failed(cw)
    }

    /// Matrice generatrice per `Keff` bit (gli ultimi `K - Keff` bit del CRC concatenati al codice), come il primo giro di
    /// `osd240_101.f90`/`fastosd240_74.f90`.
    fn generator(&mut self, keff: usize) -> Vec<[u8; N]> {
        if let Some(Some(g)) = self.generators.get(keff) {
            return g.clone();
        }
        let k = self.code.k();
        let payload = self.code.payload();
        let mut g = Vec::with_capacity(keff);
        for i in 0..keff {
            let mut msg = vec![0u8; k];
            msg[i] = 1;
            if i < payload {
                append_crc24(&mut msg);
                for b in msg.iter_mut().take(keff).skip(payload) {
                    *b = 0;
                }
            }
            g.push(self.encode(&msg));
        }
        if let Some(slot) = self.generators.get_mut(keff) {
            *slot = Some(g.clone());
        }
        g
    }

    /// `osd240_101_owned`: OSD con le regole di pre-elaborazione 1 e 2. Restituisce la parola e gli errori duri (negativi se il
    /// CRC non torna).
    fn osd101(&mut self, llr: &[f32; N], k: usize, apmask: &[u8; N], ndeep: i32) -> Option<([u8; N], i32)> {
        if k == 0 || k > self.code.k() {
            return None;
        }
        let generator = self.generator(k);
        let mut hdec = [0u8; N];
        let mut absrx = [0f32; N];
        for i in 0..N {
            hdec[i] = u8::from(llr[i] >= 0.0);
            absrx[i] = llr[i].abs();
        }
        let indx = indexx(&absrx);
        // genmrb: K righe, colonne riordinate per affidabilita' decrescente
        let mut indices = [0usize; N];
        let mut genmrb: Vec<[u8; N]> = vec![[0u8; N]; k];
        for i in 0..N {
            let c = indx[N - 1 - i];
            indices[i] = c;
            for (row, g) in genmrb.iter_mut().zip(&generator) {
                row[i] = g[c];
            }
        }
        for id in 0..k {
            for icol in id..(k + 20).min(N) {
                if genmrb[id][icol] == 1 {
                    if icol != id {
                        for row in genmrb.iter_mut() {
                            row.swap(id, icol);
                        }
                        indices.swap(id, icol);
                    }
                    let pivot = genmrb[id];
                    for (ii, row) in genmrb.iter_mut().enumerate() {
                        if ii != id && row[id] == 1 {
                            for (x, p) in row.iter_mut().zip(pivot.iter()) {
                                *x ^= p;
                            }
                        }
                    }
                    break;
                }
            }
        }
        let hdec_r: [u8; N] = std::array::from_fn(|i| hdec[indices[i]]);
        let absrx_r: [f32; N] = std::array::from_fn(|i| absrx[indices[i]]);
        let apmaskr: [u8; N] = std::array::from_fn(|i| apmask[indices[i]]);
        let hdec = hdec_r;
        let absrx = absrx_r;
        let m0: Vec<u8> = hdec[..k].to_vec();
        let encode = |me: &[u8]| -> [u8; N] {
            let mut c = [0u8; N];
            for (i, &b) in me.iter().enumerate() {
                if b == 1 {
                    for (x, g) in c.iter_mut().zip(genmrb[i].iter()) {
                        *x ^= g;
                    }
                }
            }
            c
        };
        let c0 = encode(&m0);
        let mut dmin = 0f32;
        let mut nhardmin = 0i32;
        for i in 0..N {
            if c0[i] != hdec[i] {
                nhardmin += 1;
                dmin += absrx[i];
            }
        }
        let mut cw = c0;
        let (nord, npre1, npre2, nt, ntheta, ntau) = match ndeep.min(6) {
            d if d <= 0 => (0, 0, 0, 0, 0, 0),
            1 => (1, 0, 0, 40, 12, 0),
            2 => (1, 1, 0, 40, 12, 0),
            3 => (1, 1, 1, 40, 12, 14),
            4 => (2, 1, 1, 40, 12, 17),
            5 => (3, 1, 1, 40, 12, 15),
            _ => (4, 1, 1, 95, 12, 15),
        };
        let mut misub = vec![0u8; k];
        let mut e2sub = vec![0u8; N - k];
        let mut e2 = vec![0u8; N - k];
        let mut d1 = 0f32;
        for iorder in 1..=nord {
            misub.iter_mut().enumerate().for_each(|(i, x)| *x = u8::from(i >= k - iorder));
            let mut iflag: i64 = (k - iorder + 1) as i64;
            while iflag >= 0 {
                let iend: i64 = if iorder == nord && npre1 == 0 { iflag } else { 1 };
                let mut n1 = iflag;
                while n1 >= iend {
                    let n1i = (n1 - 1) as usize;
                    let mut mi = misub.clone();
                    mi[n1i] = 1;
                    if (0..k).any(|i| apmaskr[i] & mi[i] == 1) {
                        n1 -= 1;
                        continue;
                    }
                    let me: Vec<u8> = m0.iter().zip(&mi).map(|(a, b)| a ^ b).collect();
                    let nd1kpt;
                    if n1 == iflag {
                        let ce = encode(&me);
                        for i in 0..N - k {
                            e2sub[i] = ce[k + i] ^ hdec[k + i];
                            e2[i] = e2sub[i];
                        }
                        nd1kpt = e2sub[..nt].iter().map(|&x| u32::from(x)).sum::<u32>() + 1;
                        d1 = (0..k).filter(|&i| me[i] != hdec[i]).map(|i| absrx[i]).sum();
                    } else {
                        for i in 0..N - k {
                            e2[i] = e2sub[i] ^ genmrb[n1i][k + i];
                        }
                        nd1kpt = e2[..nt].iter().map(|&x| u32::from(x)).sum::<u32>() + 2;
                    }
                    if nd1kpt <= ntheta {
                        let ce = encode(&me);
                        let dd = if n1 == iflag {
                            d1 + (0..N - k).filter(|&i| e2sub[i] == 1).map(|i| absrx[k + i]).sum::<f32>()
                        } else {
                            d1 + if ce[n1i] != hdec[n1i] { absrx[n1i] } else { 0.0 }
                                + (0..N - k).filter(|&i| e2[i] == 1).map(|i| absrx[k + i]).sum::<f32>()
                        };
                        if dd < dmin {
                            dmin = dd;
                            cw = ce;
                            nhardmin = (0..N).filter(|&i| ce[i] != hdec[i]).count() as i32;
                        }
                    }
                    n1 -= 1;
                }
                iflag = next_pattern(&mut misub, iorder);
            }
        }
        if npre2 == 1 {
            self.boxes.reset(ntau);
            let mut ntotal = 0usize;
            for i1 in (1..=k).rev() {
                for i2 in (1..i1).rev() {
                    ntotal += 1;
                    let pat = (0..ntau).fold(0usize, |a, i| (a << 1) | usize::from(genmrb[i1 - 1][k + i] ^ genmrb[i2 - 1][k + i]));
                    self.boxes.boxit(pat, ntotal, i1, i2);
                }
            }
            self.boxes.last_pattern = None;
            misub.iter_mut().enumerate().for_each(|(i, x)| *x = u8::from(i >= k - nord));
            let mut iflag: i64 = (k - nord + 1) as i64;
            while iflag >= 0 {
                let me: Vec<u8> = m0.iter().zip(&misub).map(|(a, b)| a ^ b).collect();
                let ce = encode(&me);
                for i in 0..N - k {
                    e2sub[i] = ce[k + i] ^ hdec[k + i];
                }
                'i2: for i2 in 0..=ntau {
                    let mut r2pat = e2sub.clone();
                    if i2 > 0 {
                        r2pat[i2 - 1] ^= 1;
                    }
                    let pat = (0..ntau).fold(0usize, |a, i| (a << 1) | usize::from(r2pat[i]));
                    while let Some((in1, in2)) = self.boxes.fetchit(pat) {
                        let mut mi = misub.clone();
                        mi[in1 - 1] = 1;
                        mi[in2 - 1] = 1;
                        let w: usize = mi.iter().map(|&x| usize::from(x)).sum();
                        if w < nord + npre1 + npre2 || (0..k).any(|i| apmaskr[i] & mi[i] == 1) {
                            continue 'i2;
                        }
                        let me: Vec<u8> = m0.iter().zip(&mi).map(|(a, b)| a ^ b).collect();
                        let ce = encode(&me);
                        let mut dd = 0f32;
                        let mut nh = 0i32;
                        for i in 0..N {
                            if ce[i] != hdec[i] {
                                dd += absrx[i];
                                nh += 1;
                            }
                        }
                        if dd < dmin {
                            dmin = dd;
                            cw = ce;
                            nhardmin = nh;
                        }
                    }
                }
                iflag = next_pattern(&mut misub, nord);
            }
        }
        let _ = dmin;
        let mut out = [0u8; N];
        for i in 0..N {
            out[indices[i]] = cw[i];
        }
        if crc24(&out[..self.code.k()], self.code.k()) != 0 {
            nhardmin = -nhardmin;
        }
        Some((out, nhardmin))
    }

    /// `fastosd240_74_owned`: OSD veloce con la sindrome parziale e due basi (la seconda scambia i 20 bit meno affidabili della
    /// base con i 20 migliori della parita').
    fn fastosd74(&mut self, llr: &[f32; N], k: usize, _apmask: &[u8; N], ndeep: i32) -> Option<([u8; N], i32)> {
        if k == 0 || k + 20 > N || k > self.code.k() {
            return None;
        }
        let generator = self.generator(k);
        const NSWAP: usize = 20;
        const NP: usize = 32;
        let mut result = None;
        for ibasis in 1..=2 {
            let mut hdec = [0u8; N];
            let mut absrx = [0f32; N];
            for i in 0..N {
                hdec[i] = u8::from(llr[i] >= 0.0);
                absrx[i] = llr[i].abs();
            }
            let indx = indexx(&absrx);
            let mut indices = [0usize; N];
            let mut genmrb: Vec<[u8; N]> = vec![[0u8; N]; k];
            for i in 0..N {
                let c = indx[N - 1 - i];
                indices[i] = c;
                for (row, g) in genmrb.iter_mut().zip(&generator) {
                    row[i] = g[c];
                }
            }
            if ibasis == 2 {
                for i in (k - NSWAP)..k {
                    for row in genmrb.iter_mut() {
                        row.swap(i, i + NSWAP);
                    }
                    indices.swap(i, i + NSWAP);
                }
            }
            // eliminazione di Gauss con scambio di righe e colonne saltate
            let mut icol = 0usize;
            let mut indices2 = [0usize; N];
            let mut nskipped = 0usize;
            for id in 0..k {
                loop {
                    if icol >= N {
                        return result;
                    }
                    if genmrb[id][icol] != 1 {
                        let mut found = false;
                        for j in id + 1..k {
                            if genmrb[j][icol] == 1 {
                                genmrb.swap(id, j);
                                found = true;
                            }
                        }
                        if !found {
                            if k + nskipped >= N {
                                return result;
                            }
                            indices2[k + nskipped] = icol;
                            nskipped += 1;
                            icol += 1;
                            continue;
                        }
                    }
                    break;
                }
                indices2[id] = icol;
                let pivot = genmrb[id];
                for (j, row) in genmrb.iter_mut().enumerate() {
                    if j != id && row[icol] == 1 {
                        for (x, p) in row.iter_mut().zip(pivot.iter()) {
                            *x ^= p;
                        }
                    }
                }
                icol += 1;
            }
            for (i, v) in indices2.iter_mut().enumerate().skip(k + nskipped) {
                *v = i;
            }
            for row in genmrb.iter_mut() {
                let t = *row;
                for j in 0..N {
                    row[j] = t[indices2[j]];
                }
            }
            let t = indices;
            for j in 0..N {
                indices[j] = t[indices2[j]];
            }
            let hdec: [u8; N] = std::array::from_fn(|i| hdec[indices[i]]);
            let absrx: [f32; N] = std::array::from_fn(|i| absrx[indices[i]]);
            let m0: Vec<u8> = hdec[..k].to_vec();
            let encode = |me: &[u8]| -> [u8; N] {
                let mut c = [0u8; N];
                for (i, &b) in me.iter().enumerate() {
                    if b == 1 {
                        for (x, g) in c.iter_mut().zip(genmrb[i].iter()) {
                            *x ^= g;
                        }
                    }
                }
                c
            };
            let c0 = encode(&m0);
            let mut nhardmin = 0i32;
            let mut dmin = 0f32;
            for i in 0..N {
                if c0[i] != hdec[i] {
                    nhardmin += 1;
                    dmin += absrx[i];
                }
            }
            let mut cw = c0;
            if ndeep != 0 {
                let (nord, xlambda, nsyndmax) = match ndeep.min(4) {
                    1 => (1usize, 0f32, NP as u32),
                    2 => (2, 0.0, NP as u32),
                    3 => (3, 4.0, 11),
                    4 => (4, 3.4, 12),
                    _ => (0, 0.0, 0),
                };
                let s1: f32 = absrx[..k].iter().sum();
                let s2: f32 = absrx[k..].iter().sum();
                let rho = s1 / (s1 + xlambda * s2);
                let mut rhodmin = rho * dmin;
                let mut mi = vec![0u8; k];
                for iorder in 1..=nord {
                    mi.iter_mut().enumerate().for_each(|(i, x)| *x = u8::from(i >= k - iorder));
                    let mut iflag: i64 = (k - iorder + 1) as i64;
                    while iflag >= 0 {
                        let d1: f32 = (0..k).filter(|&i| mi[i] == 1).map(|i| absrx[i]).sum();
                        if d1 > rhodmin {
                            break;
                        }
                        let me: Vec<u8> = m0.iter().zip(&mi).map(|(a, b)| a ^ b).collect();
                        // sindrome parziale: bit da K (da 1) a K+NP-1 delle colonne scelte
                        let mut sp = [0u8; NP];
                        for (i, &b) in me.iter().enumerate() {
                            if b == 1 {
                                for (s, g) in sp.iter_mut().zip(&genmrb[i][k - 1..k - 1 + NP]) {
                                    *s ^= g;
                                }
                            }
                        }
                        let nwhsp: u32 = sp.iter().zip(&hdec[k - 1..k - 1 + NP]).map(|(a, b)| u32::from(a ^ b)).sum();
                        if nwhsp <= nsyndmax {
                            let ce = encode(&me);
                            let mut dd = 0f32;
                            let mut nh = 0i32;
                            for i in 0..N {
                                if ce[i] != hdec[i] {
                                    dd += absrx[i];
                                    nh += 1;
                                }
                            }
                            if dd < dmin {
                                dmin = dd;
                                rhodmin = rho * dmin;
                                cw = ce;
                                nhardmin = nh;
                            }
                        }
                        iflag = next_pattern(&mut mi, iorder);
                    }
                }
            }
            let mut out = [0u8; N];
            for i in 0..N {
                out[indices[i]] = cw[i];
            }
            if crc24(&out[..74], 74) == 0 {
                return Some((out, nhardmin));
            }
            result = Some((out, -nhardmin));
        }
        result
    }
}

/// `nextpat101`/`nextpat74`: il prossimo modello d'errore dello stesso peso; restituisce la posizione (da 1) del primo 1, o -1 alla
/// fine.
fn next_pattern(mi: &mut [u8], iorder: usize) -> i64 {
    let k = mi.len();
    let mut ind: Option<usize> = None;
    for i in 0..k.saturating_sub(1) {
        if mi[i] == 0 && mi[i + 1] == 1 {
            ind = Some(i);
        }
    }
    let Some(ind) = ind else { return -1 };
    let mut ms = vec![0u8; k];
    ms[..ind].copy_from_slice(&mi[..ind]);
    ms[ind] = 1;
    ms[ind + 1] = 0;
    if ind + 2 < k {
        let s: usize = ms.iter().map(|&x| usize::from(x)).sum();
        let nz = iorder.saturating_sub(s);
        for x in ms[k - nz..].iter_mut() {
            *x = 1;
        }
    }
    mi.copy_from_slice(&ms);
    mi.iter().position(|&x| x == 1).map_or(-1, |p| (p + 1) as i64)
}

/// Le «scatole» della seconda regola di pre-elaborazione (`boxit101`/`fetchit101`): coppie di colonne raggruppate per i primi
/// `ntau` bit di parita'.
#[derive(Default)]
struct Boxes {
    first: Vec<Option<usize>>,
    next: Vec<Option<usize>>,
    pairs: Vec<(usize, usize)>,
    last_pattern: Option<usize>,
    next_index: Option<usize>,
}

impl Boxes {
    fn reset(&mut self, ntau: usize) {
        self.first.clear();
        self.first.resize(1 << ntau, None);
        self.next.clear();
        self.pairs.clear();
        self.next.push(None);
        self.pairs.push((0, 0));
        self.last_pattern = None;
        self.next_index = None;
    }

    fn boxit(&mut self, ipat: usize, npindex: usize, i1: usize, i2: usize) {
        if self.pairs.len() <= npindex {
            self.pairs.resize(npindex + 1, (0, 0));
            self.next.resize(npindex + 1, None);
        }
        self.pairs[npindex] = (i1, i2);
        match self.first.get(ipat).copied().flatten() {
            None => {
                if let Some(f) = self.first.get_mut(ipat) {
                    *f = Some(npindex);
                }
            }
            Some(mut ip) => {
                while let Some(nx) = self.next[ip] {
                    ip = nx;
                }
                self.next[ip] = Some(npindex);
            }
        }
    }

    /// Restituisce la prossima coppia (indici da 1) per il modello `ipat`, come `fetchit101` (che ricorda l'ultimo modello).
    fn fetchit(&mut self, ipat: usize) -> Option<(usize, usize)> {
        let index = self.first.get(ipat).copied().flatten();
        let out = if self.last_pattern != Some(ipat) && index.is_some() {
            let i = index?;
            self.next_index = self.next[i];
            Some(self.pairs[i])
        } else if self.last_pattern == Some(ipat) && self.next_index.is_some() {
            let i = self.next_index?;
            self.next_index = self.next[i];
            Some(self.pairs[i])
        } else {
            self.next_index = None;
            None
        };
        self.last_pattern = Some(ipat);
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Rng(u64);
    impl Rng {
        fn next(&mut self) -> f64 {
            self.0 = self.0.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
            ((self.0 >> 11) as f64) / ((1u64 << 53) as f64)
        }
        fn gauss(&mut self) -> f64 {
            let u1 = self.next().max(1e-12);
            let u2 = self.next();
            (-2.0 * u1.ln()).sqrt() * (2.0 * std::f64::consts::PI * u2).cos()
        }
    }

    fn random_message(code: Code, rng: &mut Rng) -> Vec<u8> {
        let k = code.k();
        let mut m: Vec<u8> = (0..k).map(|_| u8::from(rng.next() < 0.5)).collect();
        append_crc24(&mut m);
        m
    }

    fn checks_ok(code: Code, cw: &[u8; N]) -> bool {
        graph(code).nm.iter().all(|r| r.iter().map(|&b| u32::from(cw[b])).sum::<u32>() % 2 == 0)
    }

    #[test]
    fn la_codifica_soddisfa_le_parita_e_il_crc() {
        let mut rng = Rng(7);
        for code in [Code::C101, Code::C74] {
            let l = Ldpc::new(code);
            for _ in 0..20 {
                let m = random_message(code, &mut rng);
                assert_eq!(crc24(&m, m.len()), 0);
                let cw = l.encode(&m);
                assert!(checks_ok(code, &cw), "{code:?}");
            }
        }
    }

    #[test]
    fn indexx_ordina_come_un_ordinamento_crescente() {
        let mut rng = Rng(3);
        let v: Vec<f32> = (0..240).map(|_| rng.next() as f32).collect();
        let idx = indexx(&v);
        for w in idx.windows(2) {
            assert!(v[w[0]] <= v[w[1]]);
        }
        let mut s = idx.clone();
        s.sort_unstable();
        assert_eq!(s, (0..240).collect::<Vec<_>>());
    }

    fn noisy(cw: &[u8; N], sigma: f64, rng: &mut Rng) -> [f32; N] {
        std::array::from_fn(|i| {
            let x = if cw[i] == 1 { 1.0 } else { -1.0 } + sigma * rng.gauss();
            (2.0 * x / (sigma * sigma)) as f32
        })
    }

    #[test]
    fn bp_e_osd_decodificano_con_rumore() {
        let mut rng = Rng(11);
        for (code, keff) in [(Code::C101, 91), (Code::C74, 66)] {
            let mut l = Ldpc::new(code);
            let mut ok = 0;
            for _ in 0..10 {
                let m = random_message(code, &mut rng);
                let cw = l.encode(&m);
                let llr = noisy(&cw, 0.9, &mut rng);
                let d = l.decode(&llr, keff, 2, 3, &[0; N]);
                if d.nharderror >= 0 && d.message == m {
                    ok += 1;
                }
            }
            assert!(ok >= 9, "{code:?}: {ok}/10");
        }
    }

    #[test]
    fn osd_recupera_dove_bp_non_basta() {
        let mut rng = Rng(19);
        for (code, keff) in [(Code::C101, 91), (Code::C74, 66)] {
            let mut l = Ldpc::new(code);
            let (mut bp, mut all) = (0, 0);
            for _ in 0..30 {
                let m = random_message(code, &mut rng);
                let cw = l.encode(&m);
                let llr = noisy(&cw, 1.15, &mut rng);
                if l.decode(&llr, keff, -1, 0, &[0; N]).message == m {
                    bp += 1;
                }
                let d = l.decode(&llr, keff, 2, 3, &[0; N]);
                if d.nharderror >= 0 && d.message == m {
                    all += 1;
                }
            }
            assert!(all >= bp, "{code:?}: bp {bp} osd {all}");
            assert!(all > 0, "{code:?}");
        }
    }

    #[test]
    fn senza_segnale_niente_messaggi_falsi() {
        let mut rng = Rng(23);
        let mut l = Ldpc::new(Code::C101);
        for _ in 0..20 {
            let llr: [f32; N] = std::array::from_fn(|_| (2.0 * rng.gauss()) as f32);
            let d = l.decode(&llr, 91, 2, 3, &[0; N]);
            assert!(d.nharderror < 0 || crc24(&d.message, 101) == 0);
        }
    }
}
