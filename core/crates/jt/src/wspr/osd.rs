//! Decodificatore a statistiche ordinate (OSD) di WSPR: `lib/wsprd/osdwspr.f90` (K9AN). wsprd lo usa quando il Fano non arriva in fondo e
//! WSJT-X lo chiede alla profondita' di decodifica predefinita (`-o 4`, `widgets/mainwindow.cpp:2886-2887`); la parola trovata si accetta
//! solo se il nominativo e' gia' nella tabella degli hash (`wsprd.c:1422-1471`).

use super::fano::NSYM;

const N: usize = NSYM;
const K: usize = 50;

/// `gg` di `osdwspr.f90:23-24`: la prima riga della matrice generatrice (i due polinomi intercalati).
const GG: [u8; 64] = [
    1, 1, 0, 1, 0, 1, 0, 0, 1, 0, 0, 0, 1, 1, 0, 0, 1, 0, 1, 0, 0, 1, 0, 1, 1, 1, 0, 1, 1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 1, 0, 0, 1, 1, 1,
    1, 0, 0, 0, 1, 0, 0, 1, 0, 0, 1, 0, 1, 1, 1, 1, 1, 1,
];

/// `indexx` di `lib/indexx.f90` (Numerical Recipes): gli indici (da 0) che ordinano `arr` in modo crescente, con lo stesso trattamento dei
/// pari merito dell'originale.
pub fn indexx(arr: &[f32]) -> Vec<usize> {
    const M: usize = 7;
    let n = arr.len();
    // indici da 1 come in Fortran; `a(i)` legge `arr` con l'indice da 1
    let mut indx: Vec<usize> = (0..=n).collect();
    let a = |i: usize| arr[i - 1];
    let mut istack: Vec<usize> = Vec::with_capacity(64);
    let mut l = 1usize;
    let mut ir = n;
    if n == 0 {
        return Vec::new();
    }
    loop {
        if ir < l + M {
            for j in (l + 1)..=ir {
                let indxt = indx[j];
                let av = a(indxt);
                let mut i = j - 1;
                // `do i=j-1,1,-1`
                loop {
                    if i == 0 {
                        break;
                    }
                    if a(indx[i]) <= av {
                        break;
                    }
                    indx[i + 1] = indx[i];
                    i -= 1;
                }
                indx[i + 1] = indxt;
            }
            let (Some(r), Some(ll)) = (istack.pop(), istack.pop()) else {
                break;
            };
            ir = r;
            l = ll;
        } else {
            let k = (l + ir) / 2;
            indx.swap(k, l + 1);
            if a(indx[l + 1]) > a(indx[ir]) {
                indx.swap(l + 1, ir);
            }
            if a(indx[l]) > a(indx[ir]) {
                indx.swap(l, ir);
            }
            if a(indx[l + 1]) > a(indx[l]) {
                indx.swap(l + 1, l);
            }
            let mut i = l + 1;
            let mut j = ir;
            let indxt = indx[l];
            let av = a(indxt);
            loop {
                i += 1;
                while a(indx[i]) < av {
                    i += 1;
                }
                j -= 1;
                while a(indx[j]) > av {
                    j -= 1;
                }
                if j < i {
                    break;
                }
                indx.swap(i, j);
            }
            indx[l] = indx[j];
            indx[j] = indxt;
            if ir + 1 >= i + j - l {
                // `ir-i+1 >= j-l`
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
    indx[1..].iter().map(|&v| v - 1).collect()
}

/// `nextpat` di `osdwspr.f90:249-277`: il prossimo schema di errore dello stesso peso; `None` quando sono finiti. Ritorna la posizione
/// (da 0) del primo uno.
fn nextpat(mi: &mut [u8; K], iorder: usize) -> Option<usize> {
    let mut ind: Option<usize> = None;
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
        let s: usize = ms.iter().map(|&v| usize::from(v)).sum();
        let nz = iorder.saturating_sub(s);
        for v in ms[K - nz..].iter_mut() {
            *v = 1;
        }
    }
    *mi = ms;
    mi.iter().position(|&v| v == 1)
}

/// Una parola del codice nell'ordine delle posizioni piu' affidabili: i primi `K` bit e gli altri `N - K`.
#[derive(Clone, Copy, Default, PartialEq, Eq)]
struct Word {
    lo: u64,
    hi: u128,
}

impl std::ops::BitXor for Word {
    type Output = Word;
    fn bitxor(self, o: Word) -> Word {
        Word { lo: self.lo ^ o.lo, hi: self.hi ^ o.hi }
    }
}

impl Word {
    fn get(&self, p: usize) -> u8 {
        if p < K { ((self.lo >> p) & 1) as u8 } else { ((self.hi >> (p - K)) & 1) as u8 }
    }
    fn weight(&self) -> u32 {
        self.lo.count_ones() + self.hi.count_ones()
    }
}

fn weighted(bits_lo: u64, bits_hi: u128, absrx: &[f32; N]) -> f32 {
    let mut s = 0.0f32;
    for (p, &w) in absrx.iter().enumerate() {
        let b = if p < K { (bits_lo >> p) & 1 } else { ((bits_hi >> (p - K)) & 1) as u64 };
        if b == 1 {
            s += w;
        }
    }
    s
}

fn hi_weighted(bits_hi: u128, absrx: &[f32; N]) -> f32 {
    let mut s = 0.0f32;
    for (q, &w) in absrx[K..].iter().enumerate() {
        if (bits_hi >> q) & 1 == 1 {
            s += w;
        }
    }
    s
}

/// `boxit`/`fetchit` di `osdwspr.f90:279-338`: le coppie di colonne raccolte per schema dei primi `ntau` bit di parita'.
struct Boxes {
    indexes: Vec<(usize, usize)>,
    fp: Vec<i32>,
    np: Vec<i32>,
    lastpat: i64,
    inext: i32,
}

impl Boxes {
    fn new(ntau: usize) -> Self {
        Boxes { indexes: vec![(0, 0)], fp: vec![-1; 1 << ntau], np: vec![-1], lastpat: -1, inext: -1 }
    }

    fn pattern(e2: u128, ntau: usize) -> usize {
        // il primo bit e' il piu' significativo (`ishft(1, ntau-i)`)
        let mut ipat = 0usize;
        for i in 0..ntau {
            if (e2 >> i) & 1 == 1 {
                ipat += 1 << (ntau - 1 - i);
            }
        }
        ipat
    }

    fn put(&mut self, e2: u128, ntau: usize, i1: usize, i2: usize) {
        let npindex = self.indexes.len() as i32;
        self.indexes.push((i1, i2));
        self.np.push(-1);
        let ipat = Self::pattern(e2, ntau);
        let mut ip = self.fp[ipat];
        if ip == -1 {
            self.fp[ipat] = npindex;
        } else {
            while self.np[ip as usize] != -1 {
                ip = self.np[ip as usize];
            }
            self.np[ip as usize] = npindex;
        }
    }

    fn fetch(&mut self, e2: u128, ntau: usize) -> Option<(usize, usize)> {
        let ipat = Self::pattern(e2, ntau);
        let index = self.fp[ipat];
        let out = if self.lastpat != ipat as i64 && index > 0 {
            self.inext = self.np[index as usize];
            Some(self.indexes[index as usize])
        } else if self.lastpat == ipat as i64 && self.inext > 0 {
            let r = self.indexes[self.inext as usize];
            self.inext = self.np[self.inext as usize];
            Some(r)
        } else {
            self.inext = -1;
            None
        };
        self.lastpat = ipat as i64;
        out
    }
}

/// Il risultato dell'OSD: la parola del codice (0/1, ordine deinterfogliato), i bit diversi dalle decisioni dure e la distanza pesata.
pub struct OsdResult {
    pub cw: [u8; N],
    pub nhardmin: u32,
    pub dmin: f32,
}

/// `osdwspr(ss, apmask, ndeep, cw, nhardmin, dmin)`: `ss` sono i 162 simboli soft deinterfogliati (simbolo meno 128).
pub fn osdwspr(ss: &[f32; N], apmask: &[u8; N], ndeep: i32) -> OsdResult {
    // matrice generatrice: ogni riga e' la precedente spostata di due colonne
    let mut genm = vec![[0u8; N]; K];
    for (i, row) in genm.iter_mut().enumerate() {
        for (j, &g) in GG.iter().enumerate() {
            if 2 * i + j < N {
                row[2 * i + j] = g;
            }
        }
    }
    let rx: Vec<f32> = ss.iter().map(|&v| v / 127.0).collect();
    let hdec0: Vec<u8> = rx.iter().map(|&v| u8::from(v >= 0.0)).collect();
    let absrx0: Vec<f32> = rx.iter().map(|v| v.abs()).collect();
    let indx = indexx(&absrx0);

    // colonne in ordine di affidabilita' decrescente
    let mut genmrb = vec![[0u8; N]; K];
    let mut indices = [0usize; N];
    for i in 0..N {
        let c = indx[N - 1 - i];
        for r in 0..K {
            genmrb[r][i] = genm[r][c];
        }
        indices[i] = c;
    }
    // eliminazione di Gauss sulle prime K colonne (fino a K+20 per cercare il pivot)
    for id in 0..K {
        for icol in id..(K + 20) {
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
                        for (x, &p) in row.iter_mut().zip(pivot.iter()) {
                            *x ^= p;
                        }
                    }
                }
                break;
            }
        }
    }
    // g2(:, i) = riga i di genmrb, come parola
    let g2: Vec<Word> = genmrb
        .iter()
        .map(|row| {
            let mut w = Word::default();
            for (p, &b) in row.iter().enumerate() {
                if b == 1 {
                    if p < K {
                        w.lo |= 1 << p;
                    } else {
                        w.hi |= 1 << (p - K);
                    }
                }
            }
            w
        })
        .collect();

    let mut hdec = Word::default();
    let mut absrx = [0f32; N];
    let mut apm = [0u8; N];
    for p in 0..N {
        let c = indices[p];
        if hdec0[c] == 1 {
            if p < K {
                hdec.lo |= 1 << p;
            } else {
                hdec.hi |= 1 << (p - K);
            }
        }
        absrx[p] = absrx0[c];
        apm[p] = apmask[c];
    }
    let ap_lo: u64 = (0..K).filter(|&p| apm[p] == 1).fold(0u64, |m, p| m | (1 << p));
    let m0: u64 = hdec.lo;
    let encode = |me: u64| -> Word {
        let mut w = Word::default();
        for (i, g) in g2.iter().enumerate() {
            if (me >> i) & 1 == 1 {
                w = w ^ *g;
            }
        }
        w
    };
    let c0 = encode(m0);
    let nxor = c0 ^ hdec;
    let mut nhardmin = nxor.weight();
    let mut dmin = weighted(nxor.lo, nxor.hi, &absrx);
    let mut cw = c0;

    let params = match ndeep.min(5) {
        1 => Some((1usize, 0, 0, 66usize, 16u32, 16usize)),
        2 => Some((1, 1, 0, 66, 22, 16)),
        3 => Some((1, 1, 1, 66, 22, 16)),
        4 => Some((2, 1, 1, 66, 22, 16)),
        5 => Some((3, 1, 1, 66, 22, 16)),
        _ => None,
    };
    if let Some((nord, npre1, npre2, nt, ntheta, ntau)) = params {
        let nt_mask: u128 = (1u128 << nt) - 1;
        let to_mask = |mi: &[u8; K]| mi.iter().enumerate().fold(0u64, |m, (i, &v)| if v == 1 { m | (1 << i) } else { m });
        for iorder in 1..=nord {
            let mut misub = [0u8; K];
            for v in misub[K - iorder..].iter_mut() {
                *v = 1;
            }
            let mut iflag = Some(K - iorder);
            while let Some(fl) = iflag {
                let iend = if iorder == nord && npre1 == 0 { fl } else { 0 };
                let mut d1 = 0f32;
                let mut e2sub: u128 = 0;
                let base = to_mask(&misub);
                for n1 in (iend..=fl).rev() {
                    let mi = base | (1 << n1);
                    if mi & ap_lo != 0 {
                        continue;
                    }
                    let me = m0 ^ mi;
                    let (e2, nd1kpt) = if n1 == fl {
                        let ce = encode(me);
                        e2sub = ce.hi ^ hdec.hi;
                        d1 = weighted((me ^ hdec.lo) & ((1u64 << K) - 1), 0, &absrx);
                        (e2sub, (e2sub & nt_mask).count_ones() + 1)
                    } else {
                        let e2 = e2sub ^ g2[n1].hi;
                        (e2, (e2 & nt_mask).count_ones() + 2)
                    };
                    if nd1kpt <= ntheta {
                        let ce = encode(me);
                        let nx = ce ^ hdec;
                        let dd = if n1 == fl {
                            d1 + hi_weighted(e2sub, &absrx)
                        } else {
                            d1 + f32::from(ce.get(n1) ^ hdec.get(n1)) * absrx[n1] + hi_weighted(e2, &absrx)
                        };
                        if dd < dmin {
                            dmin = dd;
                            cw = ce;
                            nhardmin = nx.weight();
                        }
                    }
                }
                iflag = nextpat(&mut misub, iorder);
            }
        }
        if npre2 == 1 {
            let mut boxes = Boxes::new(ntau);
            let tau_mask: u128 = (1u128 << ntau) - 1;
            for i1 in (0..K).rev() {
                for i2 in (0..i1).rev() {
                    let e2 = (g2[i1].hi ^ g2[i2].hi) & tau_mask;
                    boxes.put(e2, ntau, i1, i2);
                }
            }
            let mut misub = [0u8; K];
            for v in misub[K - nord..].iter_mut() {
                *v = 1;
            }
            let mut iflag = Some(K - nord);
            while iflag.is_some() {
                let base = to_mask(&misub);
                let me = m0 ^ base;
                let ce = encode(me);
                let e2sub = ce.hi ^ hdec.hi;
                for i2 in 0..=ntau {
                    let ui: u128 = if i2 > 0 { 1 << (i2 - 1) } else { 0 };
                    let r2pat = (e2sub ^ ui) & tau_mask;
                    while let Some((in1, in2)) = boxes.fetch(r2pat, ntau) {
                        let mi = base | (1 << in1) | (1 << in2);
                        // `cycle`: si passa al prossimo i2
                        if (mi.count_ones() as usize) < nord + npre1 + npre2 || mi & ap_lo != 0 {
                            break;
                        }
                        let ce = encode(m0 ^ mi);
                        let nx = ce ^ hdec;
                        let dd = weighted(nx.lo, nx.hi, &absrx);
                        if dd < dmin {
                            dmin = dd;
                            cw = ce;
                            nhardmin = nx.weight();
                        }
                    }
                }
                iflag = nextpat(&mut misub, nord);
            }
        }
    }
    // di nuovo nell'ordine ricevuto
    let mut out = [0u8; N];
    for p in 0..N {
        out[indices[p]] = cw.get(p);
    }
    OsdResult { cw: out, nhardmin, dmin }
}

#[cfg(test)]
mod tests {
    use super::super::fano::encode;
    use super::*;

    #[test]
    fn indexx_sorts_like_a_sort() {
        let v = [3.0f32, 1.0, 2.0, 1.0, 5.0, 0.5, 9.0, 2.0, 7.0, 4.0, 4.0, 0.1];
        let idx = indexx(&v);
        for w in idx.windows(2) {
            assert!(v[w[0]] <= v[w[1]]);
        }
        let mut seen = idx.clone();
        seen.sort_unstable();
        assert_eq!(seen, (0..v.len()).collect::<Vec<_>>());
    }

    #[test]
    fn osd_corrects_a_few_weak_errors() {
        let data = [0xA5, 0x3C, 0x0F, 0xF0, 0x5A, 0xC3, 0x80, 0, 0, 0, 0];
        let bits = encode(&data);
        // ampiezze diverse fra loro (con tutti i valori uguali l'ordine dei pari merito lascia la matrice senza pivot, anche in WSJT-X)
        let mut seed = 99u32;
        let mut ss: [f32; N] = std::array::from_fn(|i| {
            seed = seed.wrapping_mul(1_103_515_245).wrapping_add(12_345);
            let a = 30.0 + ((seed >> 16) % 90) as f32;
            if bits[i] == 1 { a } else { -a }
        });
        // tre simboli sbagliati ma poco affidabili
        for &p in &[5usize, 77, 140] {
            ss[p] = -ss[p].signum() * 5.0;
        }
        let r = osdwspr(&ss, &[0u8; N], 4);
        assert_eq!(r.cw, bits);
        assert_eq!(r.nhardmin, 3);
    }
}
