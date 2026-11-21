//! PNG in scala di grigi a 8 bit, in scrittura e lettura.

use std::io::Cursor;

use crate::{GrayImage, WefaxError};

pub fn encode_png(img: &GrayImage) -> Result<Vec<u8>, WefaxError> {
    if img.width == 0 || img.height == 0 || img.data.len() != (img.width as usize) * (img.height as usize) {
        return Err(WefaxError::BadImage("immagine vuota o incompleta".into()));
    }
    let mut out = Vec::new();
    {
        let mut enc = png::Encoder::new(&mut out, img.width, img.height);
        enc.set_color(png::ColorType::Grayscale);
        enc.set_depth(png::BitDepth::Eight);
        let mut writer = enc.write_header().map_err(|e| WefaxError::BadImage(e.to_string()))?;
        writer.write_image_data(&img.data).map_err(|e| WefaxError::BadImage(e.to_string()))?;
    }
    Ok(out)
}

/// Legge un PNG in scala di grigi a 8 bit (quelli scritti da Aurora).
pub fn decode_png(bytes: &[u8]) -> Result<GrayImage, WefaxError> {
    let bad = |e: String| WefaxError::BadImage(e);
    let dec = png::Decoder::new(Cursor::new(bytes));
    let mut reader = dec.read_info().map_err(|e| bad(e.to_string()))?;
    let size = reader.output_buffer_size().ok_or_else(|| bad("immagine troppo grande".into()))?;
    let mut buf = vec![0u8; size];
    let info = reader.next_frame(&mut buf).map_err(|e| bad(e.to_string()))?;
    if info.color_type != png::ColorType::Grayscale || info.bit_depth != png::BitDepth::Eight {
        return Err(bad("serve un PNG in scala di grigi a 8 bit".into()));
    }
    buf.truncate((info.width as usize) * (info.height as usize));
    Ok(GrayImage { width: info.width, height: info.height, data: buf })
}

/// Ruota l'immagine di 90 gradi: `right` in senso orario, altrimenti antiorario. Larghezza e altezza si scambiano.
pub fn rotate_quarter(img: &GrayImage, right: bool) -> GrayImage {
    let (w, h) = (img.width as usize, img.height as usize);
    let mut data = vec![0u8; img.data.len()];
    // nuova immagine: larghezza h, altezza w
    for y in 0..h {
        for x in 0..w {
            let (nx, ny) = if right { (h - 1 - y, x) } else { (y, w - 1 - x) };
            data[ny * h + nx] = img.data[y * w + x];
        }
    }
    GrayImage { width: img.height, height: img.width, data }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn png_round_trip() {
        let img = GrayImage { width: 3, height: 2, data: vec![0, 40, 80, 120, 200, 255] };
        let bytes = encode_png(&img).expect("png");
        assert_eq!(decode_png(&bytes).expect("lettura"), img);
    }
}
