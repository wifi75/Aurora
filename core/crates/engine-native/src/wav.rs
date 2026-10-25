//! Salva i periodi di audio in file WAV (mono, 16 bit, 12 kHz) per provare il decoder sulle stesse registrazioni, senza JTDX acceso.

use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
};

use crate::RATE;

/// Quanti file tengono al massimo nella cartella: i piu' vecchi si cancellano (un periodo pesa 360 kB).
pub const KEEP_FILES: usize = 400;

/// Il WAV di un periodo: intestazione RIFF da 44 byte e campioni a 16 bit.
pub fn wav_bytes(samples: &[f32]) -> Vec<u8> {
    let data_len = (samples.len() * 2) as u32;
    let mut out = Vec::with_capacity(44 + samples.len() * 2);
    out.extend_from_slice(b"RIFF");
    out.extend_from_slice(&(36 + data_len).to_le_bytes());
    out.extend_from_slice(b"WAVEfmt ");
    out.extend_from_slice(&16u32.to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes()); // PCM
    out.extend_from_slice(&1u16.to_le_bytes()); // mono
    out.extend_from_slice(&(RATE as u32).to_le_bytes());
    out.extend_from_slice(&(RATE as u32 * 2).to_le_bytes());
    out.extend_from_slice(&2u16.to_le_bytes());
    out.extend_from_slice(&16u16.to_le_bytes());
    out.extend_from_slice(b"data");
    out.extend_from_slice(&data_len.to_le_bytes());
    for &s in samples {
        out.extend_from_slice(&((s.clamp(-1.0, 1.0) * 32767.0).round() as i16).to_le_bytes());
    }
    out
}

/// Il nome del file di un periodo: `AAAAMMGG_HHMMSS.wav` (UTC).
pub fn file_name(slot_unix_ms: u64) -> String {
    let secs = slot_unix_ms / 1000;
    let (days, rem) = (secs / 86_400, secs % 86_400);
    // da giorni dal 1970 a data civile (algoritmo di Howard Hinnant)
    let z = days as i64 + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    format!("{year:04}{month:02}{day:02}_{:02}{:02}{:02}.wav", rem / 3600, rem / 60 % 60, rem % 60)
}

/// Scrive il periodo nella cartella e tiene solo gli ultimi `KEEP_FILES` file.
pub fn save_slot(dir: &Path, slot_unix_ms: u64, samples: &[f32]) -> std::io::Result<PathBuf> {
    fs::create_dir_all(dir)?;
    let path = dir.join(file_name(slot_unix_ms));
    fs::File::create(&path)?.write_all(&wav_bytes(samples))?;
    let mut files: Vec<PathBuf> = fs::read_dir(dir)?.filter_map(|e| e.ok().map(|e| e.path())).filter(|p| p.extension().is_some_and(|x| x == "wav")).collect();
    if files.len() > KEEP_FILES {
        files.sort();
        for old in &files[..files.len() - KEEP_FILES] {
            let _ = fs::remove_file(old);
        }
    }
    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_header_describes_mono_16_bit_12_khz() {
        let wav = wav_bytes(&[0.0, 0.5, -0.5, 1.0]);
        assert_eq!(&wav[..4], b"RIFF");
        assert_eq!(&wav[8..16], b"WAVEfmt ");
        assert_eq!(u16::from_le_bytes([wav[22], wav[23]]), 1, "mono");
        assert_eq!(u32::from_le_bytes([wav[24], wav[25], wav[26], wav[27]]), 12_000);
        assert_eq!(u16::from_le_bytes([wav[34], wav[35]]), 16);
        assert_eq!(u32::from_le_bytes([wav[40], wav[41], wav[42], wav[43]]), 8);
        assert_eq!(wav.len(), 44 + 8);
        assert_eq!(i16::from_le_bytes([wav[50], wav[51]]), 32767, "il picco pieno");
    }

    #[test]
    fn the_file_name_is_the_utc_date_and_time_of_the_slot() {
        // 2026-10-25 01:45:00 UTC
        let ms = (20_751u64 * 86_400 + 3_600 + 45 * 60) * 1000;
        assert_eq!(file_name(ms), "20261025_014500.wav");
        assert_eq!(file_name(0), "19700101_000000.wav");
        assert_eq!(file_name(951_782_400_000), "20000229_000000.wav", "anno bisestile");
    }

    #[test]
    fn old_files_are_removed_beyond_the_limit() {
        let dir = std::env::temp_dir().join(format!("aurora-wav-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        for i in 0..(KEEP_FILES as u64 + 5) {
            save_slot(&dir, 1_700_000_000_000 + i * 15_000, &[0.0; 4]).expect("scrittura");
        }
        let count = fs::read_dir(&dir).expect("cartella").count();
        assert_eq!(count, KEEP_FILES);
        assert!(!dir.join(file_name(1_700_000_000_000)).exists(), "il piu' vecchio e' sparito");
        let _ = fs::remove_dir_all(&dir);
    }
}
