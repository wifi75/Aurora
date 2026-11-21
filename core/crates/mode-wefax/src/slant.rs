//! Correzione dell'inclinazione a mano: dalla misura a due punti al ppm, e raddrizzamento di una cartina gia' salvata.

use serde::Deserialize;

use crate::{GrayImage, WefaxError};

/// Limite della correzione (ppm), lo stesso delle impostazioni.
pub const MAX_PPM: f32 = 20_000.0;
/// Righe minime tra i due punti: troppo vicini la misura non vale.
pub const MIN_ROWS: f64 = 20.0;

/// Due punti cliccati sull'immagine (colonna `x`, riga `y`, in pixel dell'immagine) sui due capi di una linea che dovrebbe essere verticale.
#[derive(Clone, Copy, Debug, PartialEq, Deserialize)]
pub struct SlantPoints {
    pub x1: f64,
    pub y1: f64,
    pub x2: f64,
    pub y2: f64,
}

impl SlantPoints {
    /// Riga del punto piu' alto.
    pub fn top_row(&self) -> f64 {
        self.y1.min(self.y2)
    }
}

/// Correzione in ppm (positivo: righe piu' lunghe, come `slant_ppm`) che rende verticale la linea tra i due punti: se scendendo la linea
/// va a destra (`dx / dy` positivo) il periodo di riga era troppo corto e va allungato di `dx / dy / larghezza` per milione.
pub fn ppm_from_points(width: u32, p: &SlantPoints) -> Result<f32, WefaxError> {
    let bad = |m: &str| Err(WefaxError::BadSettings(m.into()));
    if width == 0 || ![p.x1, p.y1, p.x2, p.y2].iter().all(|v| v.is_finite() && *v >= 0.0) {
        return bad("punti non validi");
    }
    // il punto piu' in alto per primo
    let (a, b) = if p.y1 <= p.y2 { ((p.x1, p.y1), (p.x2, p.y2)) } else { ((p.x2, p.y2), (p.x1, p.y1)) };
    let rows = b.1 - a.1;
    if rows < MIN_ROWS {
        return bad("i due punti sono troppo vicini in verticale");
    }
    let ppm = (b.0 - a.0) / rows / f64::from(width) * 1e6;
    if ppm.abs() > f64::from(MAX_PPM) {
        return bad("inclinazione oltre 20000 ppm");
    }
    Ok(ppm as f32)
}

/// Raddrizza una cartina gia' salvata di `ppm` (stesso verso di `slant_ppm`): il pixel al tempo `tau` = riga + colonna / larghezza va
/// a sinistra di `drift * (tau - ref_row)` colonne (spostamento a capo, interpolato), con `drift = ppm * larghezza / 1e6` pixel per riga.
/// Le righe a `ref_row` restano ferme.
pub fn straighten(img: &GrayImage, ppm: f32, ref_row: f64) -> GrayImage {
    let (w, h) = (img.width as usize, img.height as usize);
    let drift = f64::from(ppm) * 1e-6 * w as f64;
    let mut data = vec![0u8; img.data.len()];
    for r in 0..h {
        let src_row = &img.data[r * w..(r + 1) * w];
        for c in 0..w {
            let tau = r as f64 + c as f64 / w as f64;
            let src = c as f64 + drift * (tau - ref_row);
            let i0 = src.floor();
            let f = (src - i0) as f32;
            let i0 = (i0 as i64).rem_euclid(w as i64) as usize;
            let (v0, v1) = (f32::from(src_row[i0]), f32::from(src_row[(i0 + 1) % w]));
            data[r * w + c] = (v0 * (1.0 - f) + v1 * f).round() as u8;
        }
    }
    GrayImage { width: img.width, height: img.height, data }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn two_points_give_the_ppm_with_the_sign_of_slant_ppm() {
        // scendendo di 100 righe la linea va a destra di 18,09 px: 0,1809 px per riga su 1809 colonne = 100 ppm
        let p = SlantPoints { x1: 500.0, y1: 100.0, x2: 518.09, y2: 200.0 };
        let ppm = ppm_from_points(1809, &p).expect("misura");
        assert!((ppm - 100.0).abs() < 0.1, "{ppm}");
        // punti in ordine inverso: stesso risultato; verso opposto: segno opposto
        let inv = SlantPoints { x1: 518.09, y1: 200.0, x2: 500.0, y2: 100.0 };
        assert!((ppm_from_points(1809, &inv).expect("misura") - ppm).abs() < 1e-3);
        let left = SlantPoints { x1: 518.09, y1: 100.0, x2: 500.0, y2: 200.0 };
        assert!((ppm_from_points(1809, &left).expect("misura") + 100.0).abs() < 0.1);
        // troppo vicini, fuori misura, non finiti
        assert!(ppm_from_points(1809, &SlantPoints { x1: 1.0, y1: 10.0, x2: 5.0, y2: 20.0 }).is_err());
        assert!(ppm_from_points(1809, &SlantPoints { x1: 0.0, y1: 0.0, x2: 1700.0, y2: 40.0 }).is_err());
        assert!(ppm_from_points(1809, &SlantPoints { x1: f64::NAN, y1: 0.0, x2: 1.0, y2: 40.0 }).is_err());
    }

    #[test]
    fn straightening_a_saved_image_makes_a_slanted_line_vertical() {
        // linea scura che scende di 0,5 px per riga (ppm = 0,5 / 400 * 1e6 = 1250)
        let (w, h) = (400usize, 300usize);
        let mut data = vec![230u8; w * h];
        for r in 0..h {
            let x = 50 + (0.5 * r as f64) as usize;
            for dx in 0..3 {
                data[r * w + x + dx] = 20;
            }
        }
        let img = GrayImage { width: w as u32, height: h as u32, data };
        let p = SlantPoints { x1: 50.0, y1: 0.0, x2: 50.0 + 0.5 * 299.0, y2: 299.0 };
        let ppm = ppm_from_points(w as u32, &p).expect("misura");
        let out = straighten(&img, ppm, p.top_row());
        let dark = |r: usize| (0..w).min_by_key(|&c| out.data[r * w + c]).unwrap_or(0) as i64;
        let (a, b) = (dark(5), dark(290));
        assert!((a - b).abs() <= 2, "la linea doveva restare verticale: colonna {a} in alto, {b} in basso");
    }
}
