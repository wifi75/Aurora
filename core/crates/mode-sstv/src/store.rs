//! Archivio delle immagini ricevute e trasmesse: un PNG per immagine e `index.json` con modo, ora, direzione e qualita'.

use std::{
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

use serde::{Deserialize, Serialize};

use crate::{RgbImage, SstvError, SstvMode, image_io::encode_png};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ImageEntry {
    pub id: String,
    pub file: String,
    /// `rx` o `tx`.
    pub direction: String,
    pub mode: SstvMode,
    pub width: u32,
    pub height: u32,
    /// Millisecondi dal 1970 (UTC).
    pub time_ms: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sync_quality: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub offset_hz: Option<f32>,
    #[serde(default = "yes")]
    pub complete: bool,
    /// Nominativo del corrispondente, se noto.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub call: Option<String>,
}

fn yes() -> bool {
    true
}

/// Dati in piu' da registrare con un'immagine.
#[derive(Clone, Debug, Default)]
pub struct EntryExtra {
    pub sync_quality: Option<f32>,
    pub offset_hz: Option<f32>,
    pub complete: bool,
    pub call: Option<String>,
}

pub struct ImageStore {
    dir: PathBuf,
    entries: Vec<ImageEntry>,
}

/// Al massimo tante immagini nell'indice: le piu' vecchie si tolgono (anche il file).
const MAX_ENTRIES: usize = 500;

pub fn now_ms() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |d| d.as_millis() as u64)
}

/// `AAAAMMGG-hhmmss` in UTC (algoritmo dei giorni civili di H. Hinnant).
pub fn stamp(ms: u64) -> String {
    let secs = ms / 1000;
    let days = (secs / 86_400) as i64;
    let rem = secs % 86_400;
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + i64::from(m <= 2);
    format!("{y:04}{m:02}{d:02}-{:02}{:02}{:02}", rem / 3600, rem / 60 % 60, rem % 60)
}

fn safe_id(id: &str) -> bool {
    !id.is_empty() && id.len() < 80 && id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
}

impl ImageStore {
    pub fn open(dir: &Path) -> ImageStore {
        let entries = std::fs::read(dir.join("index.json")).ok().and_then(|b| serde_json::from_slice::<Vec<ImageEntry>>(&b).ok()).unwrap_or_default();
        // le voci il cui file e' sparito (cancellato a mano) non si mostrano
        let entries = entries.into_iter().filter(|e| dir.join(&e.file).is_file()).collect();
        ImageStore { dir: dir.to_path_buf(), entries }
    }

    pub fn dir(&self) -> &Path {
        &self.dir
    }

    /// Le immagini, dalla piu' recente.
    pub fn list(&self) -> Vec<ImageEntry> {
        let mut out = self.entries.clone();
        out.sort_by(|a, b| b.time_ms.cmp(&a.time_ms));
        out
    }

    fn save_index(&self) -> Result<(), SstvError> {
        let json = serde_json::to_vec_pretty(&self.entries).map_err(|e| SstvError::Io(e.to_string()))?;
        let tmp = self.dir.join("index.json.tmp");
        std::fs::write(&tmp, json).map_err(|e| SstvError::Io(e.to_string()))?;
        std::fs::rename(&tmp, self.dir.join("index.json")).map_err(|e| SstvError::Io(e.to_string()))
    }

    pub fn add(&mut self, direction: &str, mode: SstvMode, img: &RgbImage, extra: EntryExtra) -> Result<ImageEntry, SstvError> {
        std::fs::create_dir_all(&self.dir).map_err(|e| SstvError::Io(e.to_string()))?;
        let time_ms = now_ms();
        let base = format!("{}_{}_{}", stamp(time_ms), direction, mode.id().replace('-', ""));
        let mut id = base.clone();
        let mut n = 2;
        while self.entries.iter().any(|e| e.id == id) {
            id = format!("{base}_{n}");
            n += 1;
        }
        let file = format!("{id}.png");
        std::fs::write(self.dir.join(&file), encode_png(img)?).map_err(|e| SstvError::Io(e.to_string()))?;
        let entry = ImageEntry {
            id,
            file,
            direction: direction.to_string(),
            mode,
            width: img.width,
            height: img.height,
            time_ms,
            sync_quality: extra.sync_quality,
            offset_hz: extra.offset_hz,
            complete: extra.complete,
            call: extra.call.filter(|c| !c.trim().is_empty()).map(|c| c.trim().to_uppercase()),
        };
        self.entries.push(entry.clone());
        while self.entries.len() > MAX_ENTRIES {
            let oldest = self.entries.iter().enumerate().min_by_key(|(_, e)| e.time_ms).map(|(i, _)| i).unwrap_or(0);
            let gone = self.entries.remove(oldest);
            let _ = std::fs::remove_file(self.dir.join(gone.file));
        }
        self.save_index()?;
        Ok(entry)
    }

    pub fn png(&self, id: &str) -> Result<Vec<u8>, SstvError> {
        let entry = self.entries.iter().find(|e| e.id == id && safe_id(id)).ok_or(SstvError::NotFound)?;
        std::fs::read(self.dir.join(&entry.file)).map_err(|_| SstvError::NotFound)
    }

    pub fn delete(&mut self, id: &str) -> Result<(), SstvError> {
        let pos = self.entries.iter().position(|e| e.id == id).ok_or(SstvError::NotFound)?;
        let gone = self.entries.remove(pos);
        let _ = std::fs::remove_file(self.dir.join(gone.file));
        self.save_index()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stamp_is_utc_calendar_time() {
        // 2026-11-10 15:30:12 UTC
        assert_eq!(stamp(1_794_324_612_000), "20261110-153012");
        assert_eq!(stamp(0), "19700101-000000");
    }

    #[test]
    fn add_list_delete() {
        let dir = std::env::temp_dir().join(format!("aurora-sstv-store-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let mut store = ImageStore::open(&dir);
        let img = RgbImage { width: 2, height: 1, data: vec![1, 2, 3, 4, 5, 6] };
        let e = store.add("rx", SstvMode::Martin1, &img, EntryExtra { call: Some("iu3abc".into()), complete: true, ..EntryExtra::default() }).expect("salvata");
        assert_eq!(e.call.as_deref(), Some("IU3ABC"));
        let again = ImageStore::open(&dir);
        assert_eq!(again.list().len(), 1);
        assert!(!again.png(&e.id).expect("png").is_empty());
        store.delete(&e.id).expect("cancellata");
        assert!(ImageStore::open(&dir).list().is_empty());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
