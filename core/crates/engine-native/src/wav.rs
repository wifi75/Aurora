//! Salva i periodi di audio in file WAV (mono, 16 bit, 12 kHz).
//!
//! Due usi, due formati:
//! - **registratore delle misure** (`AURORA_NATIVE_RECORD`, cartella `recordings`, ultimi [`KEEP_FILES`] file, nome `AAAAMMGG_HHMMSS.wav`): per provare il
//!   decoder sulle stesse registrazioni senza JTDX acceso;
//! - **salvataggio dei file audio** della scheda Audio (come JTDX: «Nessuno / Decodificati / Tutti», cartella a scelta): nome `aammgg_hhmmss.wav`
//!   (`MainWindow::dataSink`, `yyMMdd_hhmmss`, ora UTC dell'inizio del periodo), un file per periodo, mai cancellati da Aurora, con in coda un blocco
//!   `LIST`/`INFO` (`ISRC` «nominativo, locatore», `ISFT` il programma, `ICRD` la data del salvataggio, `ICMT` «Mode=FT8, Freq=14.074000, DXCall=..., DXGrid=...»)
//!   come scrive `BWFFile::impl::update_header` di JTDX. «Decodificati» tiene il file solo se il periodo ha dato almeno un messaggio (`killFile`).

use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
    sync::RwLock,
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

/// Data e ora UTC (anno, mese, giorno, ore, minuti, secondi) da millisecondi dal 1970.
fn civil(unix_ms: u64) -> (i64, i64, i64, u64, u64, u64) {
    let secs = unix_ms / 1000;
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
    (year, month, day, rem / 3600, rem / 60 % 60, rem % 60)
}

/// Il nome del file di un periodo: `AAAAMMGG_HHMMSS.wav` (UTC).
pub fn file_name(slot_unix_ms: u64) -> String {
    let (year, month, day, h, m, s) = civil(slot_unix_ms);
    format!("{year:04}{month:02}{day:02}_{h:02}{m:02}{s:02}.wav")
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

/// Quando salvare i file audio (`SaveWav` di JTDX: 0, 1, 2).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum SaveMode {
    #[default]
    None,
    /// Solo i periodi con almeno un messaggio decodificato.
    Decoded,
    All,
}

impl SaveMode {
    pub fn parse(text: &str) -> SaveMode {
        match text.trim() {
            "1" => SaveMode::Decoded,
            "2" => SaveMode::All,
            _ => SaveMode::None,
        }
    }

    /// Il periodo con `decoded` messaggi va tenuto? (`MainWindow::killFile`: Tutti oppure Decodificati con almeno un messaggio.)
    pub fn keeps(self, decoded: usize) -> bool {
        match self {
            SaveMode::None => false,
            SaveMode::Decoded => decoded > 0,
            SaveMode::All => true,
        }
    }
}

/// Cartella e modo del salvataggio: li cambia la scheda Audio, li legge il motore a ogni periodo.
#[derive(Debug, Default)]
pub struct SaveSettings {
    state: RwLock<(SaveMode, PathBuf)>,
}

impl SaveSettings {
    pub fn new(mode: SaveMode, dir: PathBuf) -> SaveSettings {
        SaveSettings { state: RwLock::new((mode, dir)) }
    }

    pub fn set(&self, mode: SaveMode, dir: PathBuf) {
        if let Ok(mut state) = self.state.write() {
            *state = (mode, dir);
        }
    }

    pub fn get(&self) -> (SaveMode, PathBuf) {
        self.state.read().map(|s| s.clone()).unwrap_or_default()
    }
}

/// Cio' che JTDX scrive nel blocco `INFO` del file.
#[derive(Clone, Debug, Default)]
pub struct SaveInfo {
    pub call: String,
    pub grid: String,
    /// `FT8` o `FT4`.
    pub mode: String,
    pub dial_hz: u64,
    pub dx_call: String,
    pub dx_grid: String,
    /// Il programma (`ISFT`), per esempio `Aurora v0.12.0`.
    pub program: String,
}

/// Il nome del file di JTDX per un periodo: `aammgg_hhmmss` (UTC, senza estensione).
pub fn jtdx_stem(slot_unix_ms: u64) -> String {
    let (year, month, day, h, m, s) = civil(slot_unix_ms);
    format!("{:02}{month:02}{day:02}_{h:02}{m:02}{s:02}", year % 100)
}

/// `aaaa-MM-ggTHH:mm:ss.zzzZ`, come `ICRD`.
fn iso_utc_ms(unix_ms: u64) -> String {
    let (year, month, day, h, m, s) = civil(unix_ms);
    format!("{year:04}-{month:02}-{day:02}T{h:02}:{m:02}:{s:02}.{:03}Z", unix_ms % 1000)
}

/// Un sotto-blocco `INFO`: sigla, lunghezza (testo piu' il NUL finale) e, se serve, un byte per arrivare a lunghezza pari.
fn info_chunk(out: &mut Vec<u8>, id: &[u8; 4], text: &str) {
    out.extend_from_slice(id);
    let len = text.len() as u32 + 1;
    out.extend_from_slice(&len.to_le_bytes());
    out.extend_from_slice(text.as_bytes());
    out.push(0);
    if len % 2 == 1 {
        out.push(0);
    }
}

/// Il WAV di un periodo come lo scrive JTDX: intestazione `RIFF`/`fmt ` PCM mono 12 kHz a 16 bit, `data`, poi il blocco `LIST`/`INFO`.
/// `saved_unix_ms` e' l'istante del salvataggio (`ICRD`).
pub fn jtdx_wav_bytes(samples: &[f32], info: &SaveInfo, saved_unix_ms: u64) -> Vec<u8> {
    let mut out = wav_bytes(samples);
    let mut list = Vec::new();
    list.extend_from_slice(b"INFO");
    let comment = format!("Mode={}, Freq={:.6}, DXCall={}, DXGrid={}", info.mode, info.dial_hz as f64 / 1e6, info.dx_call, info.dx_grid);
    // l'ordine e' quello del QMap di JTDX (per sigla)
    info_chunk(&mut list, b"ICMT", &comment);
    info_chunk(&mut list, b"ICRD", &iso_utc_ms(saved_unix_ms));
    info_chunk(&mut list, b"ISFT", &info.program);
    info_chunk(&mut list, b"ISRC", &format!("{}, {}", info.call, info.grid));
    out.extend_from_slice(b"LIST");
    out.extend_from_slice(&(list.len() as u32).to_le_bytes());
    out.extend_from_slice(&list);
    let riff_len = (out.len() - 8) as u32;
    out[4..8].copy_from_slice(&riff_len.to_le_bytes());
    out
}

/// Scrive il file del periodo nella cartella di salvataggio (la crea se manca) e ne restituisce il percorso. Un file dello stesso periodo si sostituisce.
pub fn save_jtdx_style(dir: &Path, slot_unix_ms: u64, samples: &[f32], info: &SaveInfo, saved_unix_ms: u64) -> std::io::Result<PathBuf> {
    fs::create_dir_all(dir)?;
    let path = dir.join(format!("{}.wav", jtdx_stem(slot_unix_ms)));
    fs::write(&path, jtdx_wav_bytes(samples, info, saved_unix_ms))?;
    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_jtdx_file_name_is_yymmdd_hhmmss_in_utc() {
        // 2026-10-25 01:45:00 UTC
        let ms = (20_751u64 * 86_400 + 3_600 + 45 * 60) * 1000;
        assert_eq!(jtdx_stem(ms), "261025_014500");
        assert_eq!(jtdx_stem(ms + 7_500), "261025_014507", "FT4: i 7,5 s non si arrotondano in su");
        assert_eq!(jtdx_stem(951_782_400_000), "000229_000000");
    }

    #[test]
    fn the_jtdx_wav_has_the_header_the_data_and_a_list_info_block() {
        let info = SaveInfo { call: "IU3ABC".into(), grid: "IO91wm".into(), mode: "FT8".into(), dial_hz: 14_074_000, dx_call: "K1ABC".into(), dx_grid: "FN42".into(), program: "Aurora v0.1.0".into() };
        let samples = vec![0.5f32; 6];
        let bytes = jtdx_wav_bytes(&samples, &info, 1_792_000_000_123);
        assert_eq!(&bytes[..4], b"RIFF");
        assert_eq!(u32::from_le_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]) as usize, bytes.len() - 8, "la lunghezza del RIFF comprende il blocco INFO");
        assert_eq!(u16::from_le_bytes([bytes[22], bytes[23]]), 1, "mono");
        assert_eq!(u32::from_le_bytes([bytes[24], bytes[25], bytes[26], bytes[27]]), 12_000);
        assert_eq!(u32::from_le_bytes([bytes[40], bytes[41], bytes[42], bytes[43]]), 12, "dati: 6 campioni da 16 bit");
        assert_eq!(&bytes[44 + 12..44 + 16], b"LIST");
        let list_len = u32::from_le_bytes([bytes[60], bytes[61], bytes[62], bytes[63]]) as usize;
        assert_eq!(bytes.len(), 64 + list_len);
        let body = String::from_utf8_lossy(&bytes[64..]).replace('\0', "|");
        assert!(body.starts_with("INFO"), "{body}");
        assert!(body.contains("ICMT") && body.contains("Mode=FT8, Freq=14.074000, DXCall=K1ABC, DXGrid=FN42"), "{body}");
        assert!(body.contains("ISRC") && body.contains("IU3ABC, IO91wm"), "{body}");
        assert!(body.contains("ISFT") && body.contains("Aurora v0.1.0"), "{body}");
        assert!(body.contains("2026-10-14T17:46:40.123Z"), "{body}");
        // ordine delle sigle come nel QMap di JTDX
        let order: Vec<usize> = ["ICMT", "ICRD", "ISFT", "ISRC"].iter().filter_map(|k| body.find(k)).collect();
        assert!(order.len() == 4 && order.windows(2).all(|w| w[0] < w[1]), "{body}");
    }

    #[test]
    fn save_modes_keep_the_files_like_jtdx_kill_file() {
        assert!(!SaveMode::None.keeps(5) && !SaveMode::None.keeps(0));
        assert!(SaveMode::Decoded.keeps(1) && !SaveMode::Decoded.keeps(0));
        assert!(SaveMode::All.keeps(0) && SaveMode::All.keeps(3));
        assert_eq!((SaveMode::parse("0"), SaveMode::parse("1"), SaveMode::parse("2"), SaveMode::parse("x")), (SaveMode::None, SaveMode::Decoded, SaveMode::All, SaveMode::None));
    }

    #[test]
    fn a_period_is_saved_in_the_chosen_folder_without_the_measurement_limit() {
        let dir = std::env::temp_dir().join(format!("aurora-save-{}", std::process::id())).join("sotto");
        let _ = fs::remove_dir_all(dir.parent().expect("padre"));
        let info = SaveInfo { call: "IU3ABC".into(), grid: "IO91wm".into(), mode: "FT8".into(), ..SaveInfo::default() };
        for i in 0..(KEEP_FILES as u64 + 3) {
            save_jtdx_style(&dir, 1_700_000_000_000 + i * 15_000, &[0.0; 4], &info, 1_700_000_000_000).expect("scrittura");
        }
        assert_eq!(fs::read_dir(&dir).expect("cartella").count(), KEEP_FILES + 3, "i file di JTDX non si cancellano");
        let first = dir.join(format!("{}.wav", jtdx_stem(1_700_000_000_000)));
        assert!(first.is_file());
        let _ = fs::remove_dir_all(dir.parent().expect("padre"));
    }

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
