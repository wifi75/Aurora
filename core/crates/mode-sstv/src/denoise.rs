//! Pulizia delle immagini SSTV ricevute dal rumore, senza sfocare i dettagli.
//!
//! Metodo «non-local means»: ogni punto diventa la media dei punti vicini pesata da quanto la loro zona (una finestra di `2n+1` punti di lato) somiglia
//! alla sua. Il rumore e' stimato riga per riga, perche' in SSTV ogni riga e' un tratto di segnale a se' e arriva con il suo rumore.

use crate::RgbImage;

/// Ordine di partenza (finestra `2n+1`, raggio di ricerca `3n+1`, al massimo 10) e levigatezza di partenza.
pub const DEFAULT_ORDER: u32 = 3;
pub const DEFAULT_SMOOTH: f32 = 0.4;
const MAX_SEARCH: i32 = 10;

/// La deviazione standard del rumore di ogni riga, dalle differenze fra punti vicini (stima robusta con la mediana, non sente i bordi dell'immagine).
fn row_sigma(img: &RgbImage) -> Vec<f32> {
    let (w, h) = (img.width as usize, img.height as usize);
    let mut raw = Vec::with_capacity(h);
    for y in 0..h {
        let mut diffs: Vec<f32> = Vec::with_capacity(w.saturating_sub(1) * 3);
        for x in 0..w.saturating_sub(1) {
            for c in 0..3 {
                let a = f32::from(img.data[(y * w + x) * 3 + c]);
                let b = f32::from(img.data[(y * w + x + 1) * 3 + c]);
                diffs.push((a - b).abs());
            }
        }
        diffs.sort_by(f32::total_cmp);
        let median = diffs.get(diffs.len() / 2).copied().unwrap_or(0.0);
        // mediana delle differenze assolute di due punti con rumore sigma: 0,6745 * sqrt(2) * sigma
        raw.push((median / (0.6745 * std::f32::consts::SQRT_2)).max(0.5));
    }
    // il rumore cambia poco da una riga all'altra: mediana su tre righe contro le stime sbagliate
    (0..h)
        .map(|y| {
            let mut t = [raw[y.saturating_sub(1)], raw[y], raw[(y + 1).min(h - 1)]];
            t.sort_by(f32::total_cmp);
            t[1]
        })
        .collect()
}

/// Pulisce l'immagine. `order`: 1..=6 (ordine del filtro); `smooth`: 0,05..=2,0 (piu' alto = piu' liscio, meno dettaglio).
pub fn denoise(img: &RgbImage, order: u32, smooth: f32) -> RgbImage {
    let (w, h) = (img.width as i32, img.height as i32);
    let n = order.clamp(1, 6) as i32;
    let smooth = smooth.clamp(0.05, 2.0);
    let search = (3 * n + 1).min(MAX_SEARCH);
    let sigma = row_sigma(img);
    let pixels = (w * h) as usize;

    // un punto per posizione, tre canali
    let px = |x: i32, y: i32, c: usize| f32::from(img.data[(((y.clamp(0, h - 1)) * w + x.clamp(0, w - 1)) * 3) as usize + c]);

    let mut acc = vec![0.0f32; pixels * 3];
    let mut wsum = vec![0.0f32; pixels];
    let mut wmax = vec![0.0f32; pixels];
    let iw = (w + 1) as usize;
    let mut integral = vec![0.0f32; iw * (h + 1) as usize];
    let mut d = vec![0.0f32; pixels];

    for dy in -search..=search {
        for dx in -search..=search {
            if dy < 0 || (dy == 0 && dx <= 0) {
                continue; // ogni coppia si guarda una volta e si conta per tutti e due i punti
            }
            // differenza al quadrato, media dei tre canali, fra ogni punto e quello spostato
            for y in 0..h {
                for x in 0..w {
                    let mut s = 0.0;
                    for c in 0..3 {
                        let e = px(x, y, c) - px(x + dx, y + dy, c);
                        s += e * e;
                    }
                    d[(y * w + x) as usize] = s / 3.0;
                }
            }
            // somma sulle finestre con l'immagine integrale
            for y in 0..h {
                let mut row = 0.0;
                for x in 0..w {
                    row += d[(y * w + x) as usize];
                    integral[(y as usize + 1) * iw + x as usize + 1] = integral[y as usize * iw + x as usize + 1] + row;
                }
            }
            let area = ((2 * n + 1) * (2 * n + 1)) as f32;
            for y in 0..h {
                let (y0, y1) = ((y - n).clamp(0, h), (y + n + 1).clamp(0, h));
                for x in 0..w {
                    let (x0, x1) = ((x - n).clamp(0, w), (x + n + 1).clamp(0, w));
                    let cells = ((y1 - y0) * (x1 - x0)) as f32;
                    let sum = integral[y1 as usize * iw + x1 as usize] - integral[y0 as usize * iw + x1 as usize] - integral[y1 as usize * iw + x0 as usize] + integral[y0 as usize * iw + x0 as usize];
                    let dist = sum / cells.max(1.0);
                    // le due righe possono avere rumore diverso: si usa la media delle varianze; si toglie la parte dovuta al solo rumore (2 sigma^2)
                    let (ya, yb) = (y as usize, (y + dy).clamp(0, h - 1) as usize);
                    let var = 0.5 * (sigma[ya] * sigma[ya] + sigma[yb] * sigma[yb]);
                    let excess = (dist - 2.0 * var).max(0.0);
                    let h2 = (smooth * smooth * 4.0 * var).max(1e-3);
                    let wgt = (-excess / h2).exp() * (cells / area).min(1.0);
                    if wgt < 1e-4 {
                        continue;
                    }
                    let (a, b) = ((y * w + x) as usize, ((y + dy).clamp(0, h - 1) * w + (x + dx).clamp(0, w - 1)) as usize);
                    for c in 0..3 {
                        acc[a * 3 + c] += wgt * px(x + dx, y + dy, c);
                        acc[b * 3 + c] += wgt * px(x, y, c);
                    }
                    wsum[a] += wgt;
                    wsum[b] += wgt;
                    wmax[a] = wmax[a].max(wgt);
                    wmax[b] = wmax[b].max(wgt);
                }
            }
        }
    }

    let mut data = Vec::with_capacity(pixels * 3);
    for y in 0..h {
        for x in 0..w {
            let i = (y * w + x) as usize;
            // il punto stesso pesa quanto il suo miglior vicino
            let own = wmax[i].max(1e-3);
            let total = wsum[i] + own;
            for c in 0..3 {
                data.push(((acc[i * 3 + c] + own * px(x, y, c)) / total).round().clamp(0.0, 255.0) as u8);
            }
        }
    }
    RgbImage { width: img.width, height: img.height, data }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Numeri a caso ripetibili (non serve niente di crittografico).
    struct Lcg(u64);
    impl Lcg {
        fn next(&mut self) -> f32 {
            self.0 = self.0.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
            ((self.0 >> 33) as f32) / (1u64 << 31) as f32
        }
        /// Rumore circa gaussiano (somma di dodici numeri uniformi).
        fn gauss(&mut self) -> f32 {
            (0..12).map(|_| self.next()).sum::<f32>() - 6.0
        }
    }

    fn scene(w: u32, h: u32) -> RgbImage {
        let mut data = Vec::new();
        for y in 0..h {
            for x in 0..w {
                let v = if (x / 40 + y / 32) % 2 == 0 { 60.0 } else { 190.0 } + (x as f32 / w as f32) * 30.0;
                data.extend_from_slice(&[v as u8, (v * 0.8) as u8, (255.0 - v) as u8]);
            }
        }
        RgbImage { width: w, height: h, data }
    }

    fn noisy(clean: &RgbImage, sigma: f32) -> RgbImage {
        let mut rng = Lcg(7);
        let w = clean.width as usize;
        let data = clean
            .data
            .iter()
            .enumerate()
            .map(|(i, &v)| {
                let row = i / 3 / w;
                let s = sigma * (0.7 + 0.6 * ((row % 5) as f32 / 4.0)); // il rumore cambia da riga a riga
                (f32::from(v) + s * rng.gauss()).round().clamp(0.0, 255.0) as u8
            })
            .collect();
        RgbImage { width: clean.width, height: clean.height, data }
    }

    fn mse(a: &RgbImage, b: &RgbImage) -> f32 {
        a.data.iter().zip(&b.data).map(|(&x, &y)| (f32::from(x) - f32::from(y)).powi(2)).sum::<f32>() / a.data.len() as f32
    }

    #[test]
    fn toglie_il_rumore_e_tiene_i_bordi() {
        let clean = scene(160, 128);
        let bad = noisy(&clean, 18.0);
        let out = denoise(&bad, DEFAULT_ORDER, DEFAULT_SMOOTH);
        let (before, after) = (mse(&clean, &bad), mse(&clean, &out));
        assert!(after < before * 0.5, "errore da {before} a {after}");
        // un bordo netto resta netto: i due lati di una riga di quadrati restano lontani
        let at = |i: &RgbImage, x: usize, y: usize| i32::from(i.data[(y * 160 + x) * 3]);
        assert!((at(&out, 38, 10) - at(&out, 42, 10)).abs() > 80);
    }

    #[test]
    fn immagine_pulita_resta_quasi_uguale() {
        let clean = scene(80, 64);
        let out = denoise(&clean, DEFAULT_ORDER, DEFAULT_SMOOTH);
        assert!(mse(&clean, &out) < 4.0);
    }

    #[test]
    fn piu_levigatezza_piu_pulizia() {
        let clean = scene(120, 96);
        let bad = noisy(&clean, 18.0);
        let low = mse(&clean, &denoise(&bad, 2, 0.1));
        let high = mse(&clean, &denoise(&bad, 2, 0.6));
        assert!(high < low, "{high} contro {low}");
    }
}
