//! L'audio che le macro mandano in trasmissione: un file `.wav` della cartella dei testi delle macro (`<WAVE:nome>`), i toni DTMF
//! (`<DTMF:cifre>`) e la portante di accordo (`<TUNE:s>`). Tutto a 12 kHz, mono, picco al massimo 0,9.

use crate::RATE;

/// Il file WAV letto e portato a 12 kHz mono. Formati: PCM a 8, 16, 24 o 32 bit e virgola mobile a 32 bit.
pub fn read_wav(bytes: &[u8]) -> Result<Vec<f32>, String> {
    if bytes.len() < 12 || &bytes[0..4] != b"RIFF" || &bytes[8..12] != b"WAVE" {
        return Err("non e' un file WAV".into());
    }
    let mut at = 12;
    let mut fmt: Option<(u16, u16, u32, u16)> = None;
    while at + 8 <= bytes.len() {
        let id = &bytes[at..at + 4];
        let len = u32::from_le_bytes([bytes[at + 4], bytes[at + 5], bytes[at + 6], bytes[at + 7]]) as usize;
        let body = at + 8;
        let end = body.saturating_add(len).min(bytes.len());
        if id == b"fmt " && end - body >= 16 {
            let b = &bytes[body..end];
            fmt = Some((u16::from_le_bytes([b[0], b[1]]), u16::from_le_bytes([b[2], b[3]]), u32::from_le_bytes([b[4], b[5], b[6], b[7]]), u16::from_le_bytes([b[14], b[15]])));
        } else if id == b"data" {
            let (format, channels, rate, bits) = fmt.ok_or("WAV senza intestazione del formato")?;
            return decode(&bytes[body..end], format, channels, rate, bits);
        }
        at = body + len + (len & 1);
    }
    Err("WAV senza dati audio".into())
}

fn decode(data: &[u8], format: u16, channels: u16, rate: u32, bits: u16) -> Result<Vec<f32>, String> {
    if channels == 0 || rate < 1000 || rate > 384_000 {
        return Err("WAV con canali o frequenza non validi".into());
    }
    let step = usize::from(bits / 8);
    let float = format == 3;
    if !(format == 1 || format == 0xFFFE || float) || !matches!((float, bits), (false, 8 | 16 | 24 | 32) | (true, 32)) {
        return Err(format!("formato WAV non supportato ({format}, {bits} bit): usa PCM a 16 bit"));
    }
    let frame = step * usize::from(channels);
    let sample = |s: &[u8]| -> f32 {
        match (float, bits) {
            (true, _) => f32::from_le_bytes([s[0], s[1], s[2], s[3]]),
            (false, 8) => (f32::from(s[0]) - 128.0) / 128.0,
            (false, 16) => f32::from(i16::from_le_bytes([s[0], s[1]])) / 32768.0,
            (false, 24) => (i32::from_le_bytes([0, s[0], s[1], s[2]]) >> 8) as f32 / 8_388_608.0,
            _ => i32::from_le_bytes([s[0], s[1], s[2], s[3]]) as f32 / 2_147_483_648.0,
        }
    };
    // mono: la media dei canali
    let mono: Vec<f32> = data.chunks_exact(frame).map(|f| f.chunks_exact(step).map(sample).sum::<f32>() / f32::from(channels)).collect();
    let out = resample(&mono, rate as f32, RATE);
    let peak = out.iter().fold(0.0f32, |m, &v| m.max(v.abs()));
    Ok(if peak > 0.9 { out.iter().map(|v| v * 0.9 / peak).collect() } else { out })
}

/// Ricampionamento lineare (basta per la voce e i messaggi registrati).
fn resample(input: &[f32], from: f32, to: f32) -> Vec<f32> {
    if input.is_empty() || (from - to).abs() < 0.5 {
        return input.to_vec();
    }
    let n = ((input.len() as f32) * to / from) as usize;
    (0..n)
        .map(|i| {
            let pos = i as f32 * from / to;
            let k = pos as usize;
            let frac = pos - k as f32;
            let a = input.get(k).copied().unwrap_or(0.0);
            let b = input.get(k + 1).copied().unwrap_or(a);
            a + (b - a) * frac
        })
        .collect()
}

/// Le due frequenze di un tasto DTMF (0-9, *, #, A-D).
pub fn dtmf_pair(c: char) -> Option<(f32, f32)> {
    let (row, col) = match c.to_ascii_uppercase() {
        '1' => (0, 0),
        '2' => (0, 1),
        '3' => (0, 2),
        'A' => (0, 3),
        '4' => (1, 0),
        '5' => (1, 1),
        '6' => (1, 2),
        'B' => (1, 3),
        '7' => (2, 0),
        '8' => (2, 1),
        '9' => (2, 2),
        'C' => (2, 3),
        '*' => (3, 0),
        '0' => (3, 1),
        '#' => (3, 2),
        'D' => (3, 3),
        _ => return None,
    };
    Some(([697.0, 770.0, 852.0, 941.0][row], [1209.0, 1336.0, 1477.0, 1633.0][col]))
}

/// Le cifre DTMF: 100 ms di tono e 100 ms di pausa ciascuna; una virgola fa una pausa di mezzo secondo. Gli altri caratteri si saltano.
pub fn dtmf(digits: &str) -> Vec<f32> {
    let tone = (RATE * 0.1) as usize;
    let mut out = Vec::new();
    for c in digits.chars() {
        if c == ',' {
            out.extend(std::iter::repeat_n(0.0, (RATE * 0.5) as usize));
            continue;
        }
        let Some((lo, hi)) = dtmf_pair(c) else { continue };
        out.extend((0..tone).map(|i| {
            let t = i as f32 / RATE;
            0.45 * ((std::f32::consts::TAU * lo * t).sin() + (std::f32::consts::TAU * hi * t).sin())
        }));
        out.extend(std::iter::repeat_n(0.0, tone));
    }
    out
}

/// Una portante sinusoidale (accordo) di `seconds` secondi su `hz`, con salita e discesa di 5 ms.
pub fn tone(hz: f32, seconds: f32) -> Vec<f32> {
    let n = (RATE * seconds.clamp(0.1, 60.0)) as usize;
    let ramp = (RATE * 0.005) as usize;
    (0..n)
        .map(|i| {
            let edge = (i.min(n - 1 - i) as f32 / ramp as f32).min(1.0);
            0.9 * edge * (std::f32::consts::TAU * hz * i as f32 / RATE).sin()
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn wav16(rate: u32, channels: u16, samples: &[i16]) -> Vec<u8> {
        let data: Vec<u8> = samples.iter().flat_map(|s| s.to_le_bytes()).collect();
        let mut v = Vec::new();
        v.extend_from_slice(b"RIFF");
        v.extend_from_slice(&(36 + data.len() as u32).to_le_bytes());
        v.extend_from_slice(b"WAVEfmt ");
        v.extend_from_slice(&16u32.to_le_bytes());
        v.extend_from_slice(&1u16.to_le_bytes());
        v.extend_from_slice(&channels.to_le_bytes());
        v.extend_from_slice(&rate.to_le_bytes());
        v.extend_from_slice(&(rate * 2 * u32::from(channels)).to_le_bytes());
        v.extend_from_slice(&(2 * channels).to_le_bytes());
        v.extend_from_slice(&16u16.to_le_bytes());
        v.extend_from_slice(b"data");
        v.extend_from_slice(&(data.len() as u32).to_le_bytes());
        v.extend_from_slice(&data);
        v
    }

    #[test]
    fn reads_a_16_bit_wav_and_resamples_to_12k() {
        let w = read_wav(&wav16(24_000, 1, &[1000; 2400])).unwrap();
        assert_eq!(w.len(), 1200);
        assert!((w[10] - 1000.0 / 32768.0).abs() < 1e-4);
    }

    #[test]
    fn stereo_becomes_mono_and_loud_files_are_scaled() {
        let w = read_wav(&wav16(12_000, 2, &[32767, 32767, 32767, 32767])).unwrap();
        assert_eq!(w.len(), 2);
        assert!((w[0] - 0.9).abs() < 1e-3);
    }

    #[test]
    fn rejects_what_is_not_a_wav() {
        assert!(read_wav(b"hello world, not a wav").is_err());
    }

    #[test]
    fn dtmf_has_tone_and_pause_per_digit() {
        assert_eq!(dtmf_pair('5'), Some((770.0, 1336.0)));
        assert_eq!(dtmf("12").len(), 2 * 2 * 1200);
        assert_eq!(dtmf("x"), Vec::<f32>::new());
        assert!(dtmf("1").iter().all(|v| v.abs() <= 0.9));
    }

    #[test]
    fn tone_has_the_asked_length() {
        assert_eq!(tone(1000.0, 2.0).len(), 24_000);
        assert!(tone(1000.0, 1.0)[0].abs() < 1e-6);
    }
}
