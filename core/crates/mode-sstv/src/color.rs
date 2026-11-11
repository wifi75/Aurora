//! Conversioni di colore e di frequenza, come `getLineY` e `yuvConversion` di `modebase.cpp` di QSSTV (Johan Maes ON4QZ, GPL).

use crate::modes::{BLACK_HZ, WHITE_HZ};

/// Valore 0-255 del pixel -> frequenza del tono.
pub fn value_to_hz(v: f32) -> f32 {
    BLACK_HZ + v.clamp(0.0, 255.0) * (WHITE_HZ - BLACK_HZ) / 255.0
}

/// Frequenza -> valore 0-255 (gia' corretta dello scarto di sintonia).
pub fn hz_to_value(f: f32) -> f32 {
    ((f - BLACK_HZ) * 255.0 / (WHITE_HZ - BLACK_HZ)).clamp(0.0, 255.0)
}

/// Luminanza di QSSTV: (59 G + 30 R + 11 B) / 100.
pub fn luma(r: f32, g: f32, b: f32) -> f32 {
    0.30 * r + 0.59 * g + 0.11 * b
}

/// R-Y e B-Y codificati come in QSSTV: (R - Y) / 1,40 + 127,5 e (B - Y) / 1,78 + 127,5.
pub fn chroma(r: f32, b: f32, y: f32) -> (f32, f32) {
    (((r - y) / 1.40 + 127.5).clamp(0.0, 255.0), ((b - y) / 1.78 + 127.5).clamp(0.0, 255.0))
}

/// Da Y, R-Y (`v`), B-Y (`u`) a RGB.
pub fn yuv_to_rgb(y: f32, v: f32, u: f32) -> [u8; 3] {
    let (dv, du) = (v - 127.5, u - 127.5);
    let r = y + 1.40 * dv;
    let g = y - 0.71 * dv - 0.33 * du;
    let b = y + 1.78 * du;
    [to_u8(r), to_u8(g), to_u8(b)]
}

pub fn to_u8(v: f32) -> u8 {
    v.round().clamp(0.0, 255.0) as u8
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn yuv_round_trip_is_close() {
        for &(r, g, b) in &[(255.0, 0.0, 0.0), (0.0, 255.0, 0.0), (0.0, 0.0, 255.0), (128.0, 64.0, 200.0), (20.0, 20.0, 20.0)] {
            let y = luma(r, g, b);
            let (v, u) = chroma(r, b, y);
            let out = yuv_to_rgb(y, v, u);
            assert!((f32::from(out[0]) - r).abs() < 3.0 && (f32::from(out[1]) - g).abs() < 3.0 && (f32::from(out[2]) - b).abs() < 3.0, "{r} {g} {b} -> {out:?}");
        }
    }

    #[test]
    fn frequency_scale_matches_the_standard() {
        assert_eq!(value_to_hz(0.0), 1500.0);
        assert_eq!(value_to_hz(255.0), 2300.0);
        assert!((hz_to_value(1900.0) - 127.5).abs() < 0.01);
    }
}
