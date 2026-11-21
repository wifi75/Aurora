//! Archivio delle cartine ricevute: un PNG in grigi per immagine (nome da data e ora) e `index.json` con IOC, righe al minuto e qualita'.

use std::{
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

use serde::{Deserialize, Serialize};

use crate::{GrayImage, WefaxError, image_io::encode_png};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ImageEntry {
    pub id: String,
    pub file: String,
    pub ioc: u32,
    pub lpm: f32,
    pub width: u32,
    pub height: u32,
    /// Millisecondi dal 1970 (UTC).
    pub time_ms: u64,
    /// Correlazione massima tra righe (0..1): quanto e' buona la ricezione.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub correlation: Option<f32>,
    /// Come e' finita: `apt_stop`, `no_signal`, `max_rows`, `manual`...
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    /// Come e' cominciata: `phasing`, `no_phasing`, `manual`, `forced`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    #[serde(default = "yes")]
    pub complete: bool,
}

fn yes() -> bool {
    true
}

/// Dati in piu' da registrare con un'immagine.
#[derive(Clone, Debug, Default)]
pub struct EntryExtra {
    pub ioc: u32,
    pub lpm: f32,
    pub correlation: Option<f32>,
    pub reason: Option<String>,
    pub source: Option<String>,
    pub complete: bool,
}

pub struct ImageStore {
    dir: PathBuf,
    entries: Vec<ImageEntry>,
}

/// Cartine tenute: le piu' vecchie si tolgono (anche il file).
pub const MAX_IMAGES: usize = 200;

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

    fn save_index(&self) -> Result<(), WefaxError> {
        let json = serde_json::to_vec_pretty(&self.entries).map_err(|e| WefaxError::Io(e.to_string()))?;
        let tmp = self.dir.join("index.json.tmp");
        std::fs::write(&tmp, json).map_err(|e| WefaxError::Io(e.to_string()))?;
        std::fs::rename(&tmp, self.dir.join("index.json")).map_err(|e| WefaxError::Io(e.to_string()))
    }

    pub fn add(&mut self, img: &GrayImage, extra: EntryExtra) -> Result<ImageEntry, WefaxError> {
        self.add_at(img, extra, now_ms())
    }

    pub fn add_at(&mut self, img: &GrayImage, extra: EntryExtra, time_ms: u64) -> Result<ImageEntry, WefaxError> {
        let png = encode_png(img)?;
        std::fs::create_dir_all(&self.dir).map_err(|e| WefaxError::Io(e.to_string()))?;
        let base = format!("wefax_{}_ioc{}", stamp(time_ms), extra.ioc);
        let mut id = base.clone();
        let mut n = 2;
        while self.entries.iter().any(|e| e.id == id) {
            id = format!("{base}_{n}");
            n += 1;
        }
        let file = format!("{id}.png");
        std::fs::write(self.dir.join(&file), png).map_err(|e| WefaxError::Io(e.to_string()))?;
        let entry = ImageEntry { id, file, ioc: extra.ioc, lpm: extra.lpm, width: img.width, height: img.height, time_ms, correlation: extra.correlation, reason: extra.reason, source: extra.source, complete: extra.complete };
        self.entries.push(entry.clone());
        while self.entries.len() > MAX_IMAGES {
            let oldest = self.entries.iter().enumerate().min_by_key(|(_, e)| e.time_ms).map(|(i, _)| i);
            let Some(oldest) = oldest else { break };
            let gone = self.entries.remove(oldest);
            let _ = std::fs::remove_file(self.dir.join(gone.file));
        }
        self.save_index()?;
        Ok(entry)
    }

    /// La voce di un'immagine.
    pub fn entry(&self, id: &str) -> Option<ImageEntry> {
        self.entries.iter().find(|e| e.id == id && safe_id(id)).cloned()
    }

    /// Sostituisce il PNG di un'immagine con un'altra della stessa misura (la voce resta com'e', stessa data).
    pub fn replace(&mut self, id: &str, img: &GrayImage) -> Result<ImageEntry, WefaxError> {
        let png = encode_png(img)?;
        let entry = self.entries.iter_mut().find(|e| e.id == id && safe_id(id)).ok_or(WefaxError::NotFound)?;
        std::fs::write(self.dir.join(&entry.file), png).map_err(|e| WefaxError::Io(e.to_string()))?;
        (entry.width, entry.height) = (img.width, img.height);
        let out = entry.clone();
        self.save_index()?;
        Ok(out)
    }

    pub fn png(&self, id: &str) -> Result<Vec<u8>, WefaxError> {
        let entry = self.entries.iter().find(|e| e.id == id && safe_id(id)).ok_or(WefaxError::NotFound)?;
        std::fs::read(self.dir.join(&entry.file)).map_err(|_| WefaxError::NotFound)
    }

    pub fn delete(&mut self, id: &str) -> Result<(), WefaxError> {
        let pos = self.entries.iter().position(|e| e.id == id).ok_or(WefaxError::NotFound)?;
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
        assert_eq!(stamp(1_794_324_612_000), "20261110-153012");
        assert_eq!(stamp(0), "19700101-000000");
    }

    #[test]
    fn add_list_delete() {
        let dir = std::env::temp_dir().join(format!("aurora-wefax-store-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let mut store = ImageStore::open(&dir);
        let img = GrayImage { width: 2, height: 1, data: vec![10, 200] };
        let extra = EntryExtra { ioc: 576, lpm: 120.0, complete: true, ..EntryExtra::default() };
        let e = store.add_at(&img, extra.clone(), 1_794_324_612_000).expect("salvata");
        assert_eq!(e.file, "wefax_20261110-153012_ioc576.png");
        // lo stesso secondo: nome diverso
        let e2 = store.add_at(&img, extra, 1_794_324_612_000).expect("salvata");
        assert_eq!(e2.id, "wefax_20261110-153012_ioc576_2");
        assert_eq!(ImageStore::open(&dir).list().len(), 2);
        assert!(!store.png(&e.id).expect("png").is_empty());
        assert!(store.png("../index").is_err());
        store.delete(&e.id).expect("cancellata");
        store.delete(&e2.id).expect("cancellata");
        assert!(ImageStore::open(&dir).list().is_empty());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
