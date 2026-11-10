//! PNG in scrittura e lettura, JPEG in lettura, e adattamento di un'immagine alla risoluzione del modo (riempie e taglia al centro).

use std::io::Cursor;

use crate::{RgbImage, SstvError};

pub fn encode_png(img: &RgbImage) -> Result<Vec<u8>, SstvError> {
    let mut out = Vec::new();
    {
        let mut enc = png::Encoder::new(&mut out, img.width, img.height);
        enc.set_color(png::ColorType::Rgb);
        enc.set_depth(png::BitDepth::Eight);
        let mut writer = enc.write_header().map_err(|e| SstvError::BadImage(e.to_string()))?;
        writer.write_image_data(&img.data).map_err(|e| SstvError::BadImage(e.to_string()))?;
    }
    Ok(out)
}

fn decode_png(bytes: &[u8]) -> Result<RgbImage, SstvError> {
    let bad = |e: String| SstvError::BadImage(e);
    let mut dec = png::Decoder::new(Cursor::new(bytes));
    dec.set_transformations(png::Transformations::EXPAND | png::Transformations::STRIP_16);
    let mut reader = dec.read_info().map_err(|e| bad(e.to_string()))?;
    let size = reader.output_buffer_size().ok_or_else(|| bad("immagine troppo grande".into()))?;
    let mut buf = vec![0u8; size];
    let info = reader.next_frame(&mut buf).map_err(|e| bad(e.to_string()))?;
    let (w, h) = (info.width, info.height);
    let pixels = (w as usize) * (h as usize);
    let channels = match info.color_type {
        png::ColorType::Grayscale => 1,
        png::ColorType::GrayscaleAlpha => 2,
        png::ColorType::Rgb => 3,
        png::ColorType::Rgba => 4,
        png::ColorType::Indexed => return Err(bad("PNG a tavolozza non espanso".into())),
    };
    let mut data = Vec::with_capacity(pixels * 3);
    for p in buf.chunks(channels).take(pixels) {
        match channels {
            1 | 2 => data.extend_from_slice(&[p[0], p[0], p[0]]),
            _ => data.extend_from_slice(&p[..3]),
        }
    }
    Ok(RgbImage { width: w, height: h, data })
}

fn decode_jpeg(bytes: &[u8]) -> Result<RgbImage, SstvError> {
    let mut dec = jpeg_decoder::Decoder::new(Cursor::new(bytes));
    let pixels = dec.decode().map_err(|e| SstvError::BadImage(e.to_string()))?;
    let info = dec.info().ok_or_else(|| SstvError::BadImage("JPEG senza intestazione".into()))?;
    let (w, h) = (u32::from(info.width), u32::from(info.height));
    let data = match info.pixel_format {
        jpeg_decoder::PixelFormat::RGB24 => pixels,
        jpeg_decoder::PixelFormat::L8 => pixels.iter().flat_map(|&v| [v, v, v]).collect(),
        jpeg_decoder::PixelFormat::L16 => pixels.chunks(2).flat_map(|c| [c[0], c[0], c[0]]).collect(),
        jpeg_decoder::PixelFormat::CMYK32 => pixels
            .chunks(4)
            .flat_map(|c| {
                let k = 255 - u16::from(c[3]);
                let ch = |v: u8| ((255 - u16::from(v)) * k / 255) as u8;
                [ch(c[0]), ch(c[1]), ch(c[2])]
            })
            .collect(),
    };
    Ok(RgbImage { width: w, height: h, data })
}

/// Legge un PNG o un JPEG (dal contenuto, non dal nome).
pub fn decode_image(bytes: &[u8]) -> Result<RgbImage, SstvError> {
    let img = if bytes.starts_with(&[0x89, b'P', b'N', b'G']) {
        decode_png(bytes)?
    } else if bytes.starts_with(&[0xff, 0xd8]) {
        decode_jpeg(bytes)?
    } else {
        return Err(SstvError::BadImage("serve un PNG o un JPEG".into()));
    };
    if img.width == 0 || img.height == 0 || img.data.len() != (img.width * img.height * 3) as usize {
        return Err(SstvError::BadImage("immagine vuota o incompleta".into()));
    }
    Ok(img)
}

/// Porta l'immagine a `w`x`h`: la riempie tutta tagliando al centro quel che avanza, con interpolazione bilineare.
pub fn fit_cover(img: &RgbImage, w: u32, h: u32) -> RgbImage {
    if img.width == w && img.height == h {
        return img.clone();
    }
    let scale = (f64::from(w) / f64::from(img.width)).max(f64::from(h) / f64::from(img.height));
    let (ox, oy) = ((f64::from(img.width) * scale - f64::from(w)) / 2.0, (f64::from(img.height) * scale - f64::from(h)) / 2.0);
    let mut data = Vec::with_capacity((w * h * 3) as usize);
    let at = |x: u32, y: u32, c: usize| f64::from(img.data[((y.min(img.height - 1) * img.width + x.min(img.width - 1)) * 3) as usize + c]);
    for y in 0..h {
        let sy = ((f64::from(y) + 0.5 + oy) / scale - 0.5).max(0.0);
        let (y0, fy) = (sy.floor() as u32, sy.fract());
        for x in 0..w {
            let sx = ((f64::from(x) + 0.5 + ox) / scale - 0.5).max(0.0);
            let (x0, fx) = (sx.floor() as u32, sx.fract());
            for c in 0..3 {
                let top = at(x0, y0, c) * (1.0 - fx) + at(x0 + 1, y0, c) * fx;
                let bottom = at(x0, y0 + 1, c) * (1.0 - fx) + at(x0 + 1, y0 + 1, c) * fx;
                data.push((top * (1.0 - fy) + bottom * fy).round().clamp(0.0, 255.0) as u8);
            }
        }
    }
    RgbImage { width: w, height: h, data }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn png_round_trip() {
        let img = RgbImage { width: 3, height: 2, data: (0..18).map(|v| v * 10).collect() };
        let bytes = encode_png(&img).expect("png");
        assert_eq!(decode_image(&bytes).expect("lettura"), img);
    }

    #[test]
    fn cover_fills_the_target() {
        let img = RgbImage { width: 4, height: 2, data: vec![200; 24] };
        let out = fit_cover(&img, 320, 256);
        assert_eq!((out.width, out.height, out.data.len()), (320, 256, 320 * 256 * 3));
        assert!(out.data.iter().all(|&v| v == 200));
    }
}
