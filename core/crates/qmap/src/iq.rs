//! I campioni I/Q a banda larga e il file WAV I/Q (due canali: I a sinistra, Q a destra), il formato delle registrazioni degli SDR
//! (HDSDR, SDR Console, ExpertSDR) e delle prove.

use std::io::{Read, Write};
use std::path::Path;

use rustfft::num_complex::Complex;

/// Un tratto di flusso I/Q.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct IqBlock {
    pub sample_rate: u32,
    /// Frequenza al centro del flusso (0 Hz di offset), se nota.
    pub center_hz: Option<u64>,
    pub samples: Vec<Complex<f32>>,
}

/// Una sorgente di blocchi I/Q (il TCI dal vivo o un file).
pub trait IqSource: Send {
    /// Il prossimo blocco; `None` se per ora non ce n'e' (o se il file e' finito).
    fn next_block(&mut self) -> Option<IqBlock>;
}

/// Legge un WAV I/Q: PCM a 16, 24 o 32 bit oppure float a 32 bit, due canali. Il centro si prende dal nome del file se c'e'
/// un numero seguito da «Hz» o «kHz» (come i nomi di HDSDR, `..._14074kHz.wav`), altrimenti resta sconosciuto.
pub fn read_wav(path: &Path) -> Result<IqBlock, String> {
    let mut bytes = Vec::new();
    std::fs::File::open(path).and_then(|mut f| f.read_to_end(&mut bytes)).map_err(|e| format!("{}: {e}", path.display()))?;
    let mut block = parse_wav(&bytes)?;
    block.center_hz = path.file_stem().and_then(|s| s.to_str()).and_then(center_from_name);
    Ok(block)
}

/// Il centro scritto nel nome del file («..._144100000Hz», «..._14074kHz»).
pub fn center_from_name(name: &str) -> Option<u64> {
    let lower = name.to_ascii_lowercase();
    for (unit, mult) in [("khz", 1_000u64), ("hz", 1)] {
        if let Some(pos) = lower.find(unit) {
            let digits: String = lower[..pos].chars().rev().take_while(char::is_ascii_digit).collect::<Vec<_>>().into_iter().rev().collect();
            if let Ok(v) = digits.parse::<u64>()
                && v > 0
            {
                return Some(v * mult);
            }
        }
    }
    None
}

fn u16_at(b: &[u8], i: usize) -> u16 {
    u16::from_le_bytes([b[i], b[i + 1]])
}

fn u32_at(b: &[u8], i: usize) -> u32 {
    u32::from_le_bytes([b[i], b[i + 1], b[i + 2], b[i + 3]])
}

/// Decodifica i byte di un WAV I/Q.
pub fn parse_wav(b: &[u8]) -> Result<IqBlock, String> {
    if b.len() < 12 || &b[0..4] != b"RIFF" || &b[8..12] != b"WAVE" {
        return Err("non e' un file WAV".into());
    }
    let mut pos = 12;
    let mut fmt: Option<(u16, u16, u32, u16)> = None;
    while pos + 8 <= b.len() {
        let id = &b[pos..pos + 4];
        let len = u32_at(b, pos + 4) as usize;
        let body = pos + 8;
        let end = (body + len).min(b.len());
        if id == b"fmt " && len >= 16 {
            let mut tag = u16_at(b, body);
            if tag == 0xFFFE && len >= 40 {
                tag = u16_at(b, body + 24); // WAVE_FORMAT_EXTENSIBLE: il formato vero nel sottotipo
            }
            fmt = Some((tag, u16_at(b, body + 2), u32_at(b, body + 4), u16_at(b, body + 14)));
        } else if id == b"data" {
            let (tag, channels, rate, bits) = fmt.ok_or("manca il blocco fmt prima dei dati")?;
            if channels != 2 {
                return Err(format!("servono due canali (I e Q), il file ne ha {channels}"));
            }
            let data = &b[body..end];
            let samples: Vec<Complex<f32>> = match (tag, bits) {
                (1, 16) => data.as_chunks::<4>().0.iter().map(|c| Complex::new(i16::from_le_bytes([c[0], c[1]]) as f32 / 32768.0, i16::from_le_bytes([c[2], c[3]]) as f32 / 32768.0)).collect(),
                (1, 24) => data
                    .as_chunks::<6>().0.iter()
                    .map(|c| {
                        let s = |o: usize| (i32::from_le_bytes([0, c[o], c[o + 1], c[o + 2]]) >> 8) as f32 / 8_388_608.0;
                        Complex::new(s(0), s(3))
                    })
                    .collect(),
                (1, 32) => data.as_chunks::<8>().0.iter().map(|c| Complex::new(i32::from_le_bytes([c[0], c[1], c[2], c[3]]) as f32 / 2_147_483_648.0, i32::from_le_bytes([c[4], c[5], c[6], c[7]]) as f32 / 2_147_483_648.0)).collect(),
                (3, 32) => data.as_chunks::<8>().0.iter().map(|c| Complex::new(f32::from_le_bytes([c[0], c[1], c[2], c[3]]), f32::from_le_bytes([c[4], c[5], c[6], c[7]]))).collect(),
                _ => return Err(format!("formato WAV non gestito ({tag}, {bits} bit)")),
            };
            return Ok(IqBlock { sample_rate: rate, center_hz: None, samples });
        }
        pos = body + len + (len & 1);
    }
    Err("il file WAV non ha dati".into())
}

/// Scrive un WAV I/Q in float a 32 bit (per le registrazioni e le prove).
pub fn write_wav(path: &Path, block: &IqBlock) -> Result<(), String> {
    let data_len = block.samples.len() * 8;
    let mut out = Vec::with_capacity(44 + data_len);
    out.extend_from_slice(b"RIFF");
    out.extend_from_slice(&((36 + data_len) as u32).to_le_bytes());
    out.extend_from_slice(b"WAVEfmt ");
    out.extend_from_slice(&16u32.to_le_bytes());
    out.extend_from_slice(&3u16.to_le_bytes());
    out.extend_from_slice(&2u16.to_le_bytes());
    out.extend_from_slice(&block.sample_rate.to_le_bytes());
    out.extend_from_slice(&(block.sample_rate * 8).to_le_bytes());
    out.extend_from_slice(&8u16.to_le_bytes());
    out.extend_from_slice(&32u16.to_le_bytes());
    out.extend_from_slice(b"data");
    out.extend_from_slice(&(data_len as u32).to_le_bytes());
    for c in &block.samples {
        out.extend_from_slice(&c.re.to_le_bytes());
        out.extend_from_slice(&c.im.to_le_bytes());
    }
    std::fs::File::create(path).and_then(|mut f| f.write_all(&out)).map_err(|e| format!("{}: {e}", path.display()))
}

/// Un file WAV I/Q come sorgente: consegna tutto il file in un blocco solo, una volta.
pub struct WavIqSource {
    block: Option<IqBlock>,
}

impl WavIqSource {
    pub fn open(path: &Path) -> Result<WavIqSource, String> {
        Ok(WavIqSource { block: Some(read_wav(path)?) })
    }
}

impl IqSource for WavIqSource {
    fn next_block(&mut self) -> Option<IqBlock> {
        self.block.take()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_float_wav_round_trips() {
        let dir = std::env::temp_dir().join(format!("aurora-qmap-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("cartella");
        let path = dir.join("prova_144100kHz.wav");
        let block = IqBlock { sample_rate: 96_000, center_hz: None, samples: vec![Complex::new(0.5, -0.25), Complex::new(-1.0, 0.125)] };
        write_wav(&path, &block).expect("scrittura");
        let back = read_wav(&path).expect("lettura");
        assert_eq!(back.samples, block.samples);
        assert_eq!(back.sample_rate, 96_000);
        assert_eq!(back.center_hz, Some(144_100_000));
        let mut src = WavIqSource::open(&path).expect("sorgente");
        assert!(src.next_block().is_some() && src.next_block().is_none());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn pcm16_and_bad_files() {
        let mut b = Vec::new();
        b.extend_from_slice(b"RIFF\0\0\0\0WAVEfmt ");
        b.extend_from_slice(&16u32.to_le_bytes());
        b.extend_from_slice(&[1, 0, 2, 0]);
        b.extend_from_slice(&48_000u32.to_le_bytes());
        b.extend_from_slice(&(48_000u32 * 4).to_le_bytes());
        b.extend_from_slice(&[4, 0, 16, 0]);
        b.extend_from_slice(b"data");
        b.extend_from_slice(&4u32.to_le_bytes());
        b.extend_from_slice(&16384i16.to_le_bytes());
        b.extend_from_slice(&(-16384i16).to_le_bytes());
        let block = parse_wav(&b).expect("pcm16");
        assert_eq!(block.samples, vec![Complex::new(0.5, -0.5)]);
        assert!(parse_wav(b"nulla").is_err());
        assert_eq!(center_from_name("rec_14074kHz"), Some(14_074_000));
        assert_eq!(center_from_name("senza"), None);
    }
}
