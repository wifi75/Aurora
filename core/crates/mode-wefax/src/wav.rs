//! Lettura di un file WAV (PCM a 8, 16, 24 o 32 bit, oppure float a 32 o 64 bit; mono o stereo; da 6000 a 96000 Hz) senza dipendenze: serve
//! alla decodifica di una registrazione. I campioni si convertono a pezzi (`read`), cosi' un file grande non raddoppia la memoria.

use crate::WefaxError;

/// Frequenze di campionamento ammesse.
pub const MIN_RATE: u32 = 6000;
pub const MAX_RATE: u32 = 96_000;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Kind {
    U8,
    I16,
    I24,
    I32,
    F32,
    F64,
}

impl Kind {
    fn bytes(self) -> usize {
        match self {
            Kind::U8 => 1,
            Kind::I16 => 2,
            Kind::I24 => 3,
            Kind::I32 | Kind::F32 => 4,
            Kind::F64 => 8,
        }
    }
}

/// L'intestazione di un WAV letto: dove stanno i campioni e come sono fatti.
#[derive(Clone, Debug)]
pub struct Wav {
    pub rate: u32,
    pub channels: u16,
    kind: Kind,
    data_start: usize,
    frames: usize,
}

fn bad<T>(m: &str) -> Result<T, WefaxError> {
    Err(WefaxError::BadAudio(m.into()))
}

fn u16_at(b: &[u8], i: usize) -> Option<u16> {
    Some(u16::from_le_bytes(b.get(i..i + 2)?.try_into().ok()?))
}

fn u32_at(b: &[u8], i: usize) -> Option<u32> {
    Some(u32::from_le_bytes(b.get(i..i + 4)?.try_into().ok()?))
}

/// Legge l'intestazione. Errori chiari per un file che non e' un WAV o ha un formato che non si legge.
pub fn parse(bytes: &[u8]) -> Result<Wav, WefaxError> {
    if bytes.len() < 12 || &bytes[0..4] != b"RIFF" || &bytes[8..12] != b"WAVE" {
        return bad("non e' un file WAV");
    }
    let mut fmt: Option<(u16, u16, u32, u16, Option<u16>)> = None;
    let mut data: Option<(usize, usize)> = None;
    let mut pos = 12usize;
    while pos + 8 <= bytes.len() {
        let id = &bytes[pos..pos + 4];
        let size = u32_at(bytes, pos + 4).unwrap_or(0) as usize;
        let body = pos + 8;
        let remaining = bytes.len() - body;
        if id == b"fmt " {
            if size < 16 || size > remaining {
                return bad("intestazione del WAV danneggiata");
            }
            let tag = u16_at(bytes, body).unwrap_or(0);
            let channels = u16_at(bytes, body + 2).unwrap_or(0);
            let rate = u32_at(bytes, body + 4).unwrap_or(0);
            let bits = u16_at(bytes, body + 14).unwrap_or(0);
            // formato esteso: il tipo vero sta nei primi due byte del GUID
            let sub = if tag == 0xFFFE && size >= 26 { u16_at(bytes, body + 24) } else { None };
            fmt = Some((tag, channels, rate, bits, sub));
        } else if id == b"data" {
            // dimensione 0 o troppo grande (WAV scritto in streaming): si prende fino alla fine del file
            let len = if size == 0 || size > remaining { remaining } else { size };
            data = Some((body, len));
        }
        if fmt.is_some() && data.is_some() {
            break;
        }
        let advance = size.checked_add(size & 1).and_then(|s| s.checked_add(8));
        match advance.and_then(|a| pos.checked_add(a)) {
            Some(next) if next > pos => pos = next,
            _ => break,
        }
    }
    let Some((tag, channels, rate, bits, sub)) = fmt else { return bad("manca l'intestazione del formato (fmt)") };
    let Some((data_start, data_len)) = data else { return bad("mancano i campioni (data)") };
    let tag = if tag == 0xFFFE { sub.unwrap_or(0) } else { tag };
    let kind = match (tag, bits) {
        (1, 8) => Kind::U8,
        (1, 16) => Kind::I16,
        (1, 24) => Kind::I24,
        (1, 32) => Kind::I32,
        (3, 32) => Kind::F32,
        (3, 64) => Kind::F64,
        (1 | 3, _) => return bad("serve un WAV PCM a 16 bit (o float a 32 bit)"),
        _ => return bad("serve un WAV non compresso (PCM o float)"),
    };
    if channels == 0 || channels > 2 {
        return bad("serve un WAV mono o stereo");
    }
    if !(MIN_RATE..=MAX_RATE).contains(&rate) {
        return bad("la frequenza di campionamento deve essere tra 6000 e 96000 Hz");
    }
    let frames = data_len / (kind.bytes() * usize::from(channels));
    if frames == 0 {
        return bad("il file non contiene campioni");
    }
    Ok(Wav { rate, channels, kind, data_start, frames })
}

impl Wav {
    /// Quanti campioni per canale (frame) ci sono.
    pub fn frames(&self) -> usize {
        self.frames
    }

    /// Converte `count` frame da `from` in campioni mono (a -1..1; in stereo i due canali si mediano) e li aggiunge a `out`.
    pub fn read(&self, bytes: &[u8], from: usize, count: usize, out: &mut Vec<f32>) {
        let width = self.kind.bytes();
        let ch = usize::from(self.channels);
        let end = from.saturating_add(count).min(self.frames);
        for f in from..end {
            let mut sum = 0.0f32;
            for c in 0..ch {
                let i = self.data_start + (f * ch + c) * width;
                let Some(s) = bytes.get(i..i + width) else { break };
                sum += match self.kind {
                    Kind::U8 => (f32::from(s[0]) - 128.0) / 128.0,
                    Kind::I16 => f32::from(i16::from_le_bytes([s[0], s[1]])) / 32768.0,
                    Kind::I24 => (i32::from_le_bytes([0, s[0], s[1], s[2]]) >> 8) as f32 / 8_388_608.0,
                    Kind::I32 => i32::from_le_bytes([s[0], s[1], s[2], s[3]]) as f32 / 2_147_483_648.0,
                    Kind::F32 => f32::from_le_bytes([s[0], s[1], s[2], s[3]]),
                    Kind::F64 => f64::from_le_bytes([s[0], s[1], s[2], s[3], s[4], s[5], s[6], s[7]]) as f32,
                };
            }
            let v = sum / ch as f32;
            out.push(if v.is_finite() { v } else { 0.0 });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Un WAV di prova: intestazione `fmt ` (con il formato esteso se `extensible`) e i byte di `data`.
    fn wav(tag: u16, channels: u16, rate: u32, bits: u16, extensible: bool, data: &[u8]) -> Vec<u8> {
        let mut fmt = Vec::new();
        fmt.extend_from_slice(&(if extensible { 0xFFFEu16 } else { tag }).to_le_bytes());
        fmt.extend_from_slice(&channels.to_le_bytes());
        fmt.extend_from_slice(&rate.to_le_bytes());
        let block = channels * bits / 8;
        fmt.extend_from_slice(&(rate * u32::from(block)).to_le_bytes());
        fmt.extend_from_slice(&block.to_le_bytes());
        fmt.extend_from_slice(&bits.to_le_bytes());
        if extensible {
            fmt.extend_from_slice(&22u16.to_le_bytes());
            fmt.extend_from_slice(&bits.to_le_bytes());
            fmt.extend_from_slice(&3u32.to_le_bytes());
            fmt.extend_from_slice(&tag.to_le_bytes());
            fmt.extend_from_slice(&[0, 0, 0, 0, 0x10, 0, 0x80, 0, 0, 0xAA, 0, 0x38, 0x9B, 0x71]);
        }
        let mut out = b"RIFF".to_vec();
        out.extend_from_slice(&0u32.to_le_bytes());
        out.extend_from_slice(b"WAVE");
        out.extend_from_slice(b"fmt ");
        out.extend_from_slice(&(fmt.len() as u32).to_le_bytes());
        out.extend_from_slice(&fmt);
        // un blocco in mezzo di misura dispari (con il byte di riempimento): si deve saltare
        out.extend_from_slice(b"LIST");
        out.extend_from_slice(&3u32.to_le_bytes());
        out.extend_from_slice(&[1, 2, 3, 0]);
        out.extend_from_slice(b"data");
        out.extend_from_slice(&(data.len() as u32).to_le_bytes());
        out.extend_from_slice(data);
        out
    }

    #[test]
    fn it_reads_pcm16_mono_and_mixes_stereo() {
        let mono: Vec<u8> = [16384i16, -16384, 0].iter().flat_map(|v| v.to_le_bytes()).collect();
        let bytes = wav(1, 1, 12_000, 16, false, &mono);
        let w = parse(&bytes).expect("mono");
        assert_eq!((w.rate, w.channels, w.frames()), (12_000, 1, 3));
        let mut out = Vec::new();
        w.read(&bytes, 0, 10, &mut out);
        assert_eq!(out, vec![0.5, -0.5, 0.0]);
        let stereo: Vec<u8> = [16384i16, 0, -16384, -16384].iter().flat_map(|v| v.to_le_bytes()).collect();
        let bytes = wav(1, 2, 44_100, 16, false, &stereo);
        let w = parse(&bytes).expect("stereo");
        assert_eq!((w.rate, w.frames()), (44_100, 2));
        let mut out = Vec::new();
        w.read(&bytes, 0, 2, &mut out);
        assert_eq!(out, vec![0.25, -0.5]);
    }

    #[test]
    fn it_reads_float_and_the_extensible_header() {
        let data: Vec<u8> = [0.25f32, -1.0].iter().flat_map(|v| v.to_le_bytes()).collect();
        for ext in [false, true] {
            let bytes = wav(3, 1, 48_000, 32, ext, &data);
            let w = parse(&bytes).expect("float");
            let mut out = Vec::new();
            w.read(&bytes, 1, 5, &mut out);
            assert_eq!(out, vec![-1.0], "extensible {ext}");
        }
        let d64: Vec<u8> = [0.5f64].iter().flat_map(|v| v.to_le_bytes()).collect();
        let bytes = wav(3, 1, 8000, 64, false, &d64);
        let mut out = Vec::new();
        parse(&bytes).expect("float64").read(&bytes, 0, 1, &mut out);
        assert_eq!(out, vec![0.5]);
    }

    #[test]
    fn it_refuses_what_it_cannot_read_with_a_clear_error() {
        let data = [0u8; 8];
        let msg = |b: &[u8]| parse(b).expect_err("doveva rifiutare").to_string();
        assert!(msg(b"questo non e' un file wav, e' solo testo").contains("non e' un file WAV"));
        assert!(msg(b"").contains("non e' un file WAV"));
        assert!(msg(&wav(1, 1, 5_000, 16, false, &data)).contains("6000"));
        assert!(msg(&wav(1, 1, 192_000, 16, false, &data)).contains("96000"));
        assert!(msg(&wav(1, 6, 12_000, 16, false, &data)).contains("mono o stereo"));
        assert!(msg(&wav(2, 1, 12_000, 4, false, &data)).contains("non compresso"));
        assert!(msg(&wav(1, 1, 12_000, 16, false, &[])).contains("campioni"));
        // intestazione tagliata
        let full = wav(1, 1, 12_000, 16, false, &data);
        assert!(parse(&full[..30]).is_err());
    }

    #[test]
    fn a_data_size_larger_than_the_file_reads_up_to_the_end() {
        let data: Vec<u8> = [100i16, 200, 300].iter().flat_map(|v| v.to_le_bytes()).collect();
        let mut bytes = wav(1, 1, 12_000, 16, false, &data);
        let at = bytes.len() - data.len() - 4;
        bytes[at..at + 4].copy_from_slice(&u32::MAX.to_le_bytes());
        assert_eq!(parse(&bytes).expect("streaming").frames(), 3);
        // un campione tagliato a meta' non conta
        bytes.pop();
        assert_eq!(parse(&bytes).expect("tagliato").frames(), 2);
    }
}
