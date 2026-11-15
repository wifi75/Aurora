//! La registrazione dell'audio ricevuto per le macro (`<STARTRECORD:nome>`, `<STOPRECORD>`) e gli ultimi 60 secondi tenuti da parte per
//! rileggerli nel ricevitore (`<REPLAY:n>`). Il file e' un WAV mono a 16 bit nella cartella `recordings` dei dati.

use std::collections::VecDeque;
use std::path::{Path, PathBuf};

/// Al massimo 30 minuti per registrazione: oltre si chiude da sola.
const MAX_SECONDS: usize = 30 * 60;
/// Quanti secondi restano da parte per `<REPLAY>`.
const RING_SECONDS: usize = 60;

#[derive(Default)]
pub struct Recorder {
    file: Option<(PathBuf, u32, Vec<i16>)>,
    ring: VecDeque<f32>,
    ring_rate: u32,
}

impl Recorder {
    /// Un blocco dell'audio d'ingresso.
    pub fn push(&mut self, rate: u32, samples: &[f32]) {
        if rate != self.ring_rate {
            self.ring.clear();
            self.ring_rate = rate;
        }
        self.ring.extend(samples.iter().copied());
        let keep = rate as usize * RING_SECONDS;
        if self.ring.len() > keep {
            let extra = self.ring.len() - keep;
            self.ring.drain(..extra);
        }
        let mut full = None;
        if let Some((path, r, data)) = self.file.as_mut() {
            if *r == 0 {
                *r = rate;
            }
            data.extend(samples.iter().map(|&x| (x.clamp(-1.0, 1.0) * 32767.0) as i16));
            if data.len() >= *r as usize * MAX_SECONDS {
                full = Some(path.clone());
            }
        }
        if full.is_some() && let Err(e) = self.stop() {
            tracing::warn!("registrazione non salvata: {e}");
        }
    }

    pub fn is_recording(&self) -> bool {
        self.file.is_some()
    }

    /// Comincia una registrazione (quella in corso si salva prima).
    pub fn start(&mut self, path: PathBuf) -> Result<(), String> {
        self.stop()?;
        self.file = Some((path, 0, Vec::new()));
        Ok(())
    }

    /// Chiude e salva: il percorso del file, `None` se non registrava.
    pub fn stop(&mut self) -> Result<Option<PathBuf>, String> {
        let Some((path, rate, data)) = self.file.take() else { return Ok(None) };
        write_wav(&path, rate.max(8000), &data)?;
        Ok(Some(path))
    }

    /// Gli ultimi `seconds` secondi ricevuti, con la loro frequenza di campionamento.
    pub fn last(&self, seconds: f32) -> (u32, Vec<f32>) {
        let n = ((self.ring_rate as f32 * seconds.clamp(1.0, RING_SECONDS as f32)) as usize).min(self.ring.len());
        (self.ring_rate, self.ring.iter().skip(self.ring.len() - n).copied().collect())
    }
}

/// Un WAV mono a 16 bit.
pub fn write_wav(path: &Path, rate: u32, data: &[i16]) -> Result<(), String> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    let bytes = data.len() as u32 * 2;
    let mut v = Vec::with_capacity(44 + bytes as usize);
    v.extend_from_slice(b"RIFF");
    v.extend_from_slice(&(36 + bytes).to_le_bytes());
    v.extend_from_slice(b"WAVEfmt ");
    v.extend_from_slice(&16u32.to_le_bytes());
    v.extend_from_slice(&1u16.to_le_bytes());
    v.extend_from_slice(&1u16.to_le_bytes());
    v.extend_from_slice(&rate.to_le_bytes());
    v.extend_from_slice(&(rate * 2).to_le_bytes());
    v.extend_from_slice(&2u16.to_le_bytes());
    v.extend_from_slice(&16u16.to_le_bytes());
    v.extend_from_slice(b"data");
    v.extend_from_slice(&bytes.to_le_bytes());
    for s in data {
        v.extend_from_slice(&s.to_le_bytes());
    }
    std::fs::write(path, v).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn records_and_saves_a_readable_wav() {
        let dir = std::env::temp_dir().join(format!("aurora-rec-{}", std::process::id()));
        let path = dir.join("prova.wav");
        let mut r = Recorder::default();
        r.start(path.clone()).unwrap();
        assert!(r.is_recording());
        r.push(12_000, &[0.5; 1200]);
        assert_eq!(r.stop().unwrap(), Some(path.clone()));
        let back = crate::macro_audio::read_wav(&std::fs::read(&path).unwrap()).unwrap();
        assert_eq!(back.len(), 1200);
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn keeps_only_the_last_minute_for_replay() {
        let mut r = Recorder::default();
        for _ in 0..70 {
            r.push(1000, &[0.1; 1000]);
        }
        let (rate, last) = r.last(100.0);
        assert_eq!(rate, 1000);
        assert_eq!(last.len(), 60_000);
        assert_eq!(r.last(2.0).1.len(), 2000);
        assert_eq!(r.stop().unwrap(), None);
    }
}
