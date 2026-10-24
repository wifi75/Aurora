//! Livello dell'ingresso audio. JTDX non ha un controllo dell'ingresso e un ingresso saturo rovina i decode, cosi' qui si
//! regola il volume di registrazione di Windows del dispositivo, a mano o con la regolazione automatica (AUTO): scende subito
//! se il segnale satura, sale piano se resta debole. Mai durante la trasmissione e mai su un ingresso muto.
//!
//! Le regole sono scritte da zero a partire dalle idee del confronto con Decodium (`docs/confronto-decodium.md`).

mod auto;

use std::{
    path::{Path, PathBuf},
    sync::{Arc, Mutex, MutexGuard},
    time::Instant,
};

use audio_spectrum::InputEndpoint;
use aurora_core::{AudioLevel, Event};
use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;

pub use auto::AutoGain;

#[derive(Debug, PartialEq, Eq)]
pub enum InputLevelError {
    /// La percentuale non e' tra 0 e 100.
    BadValue,
    /// Il dispositivo non risponde o non si lascia regolare.
    Device(String),
}

impl std::fmt::Display for InputLevelError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            InputLevelError::BadValue => write!(f, "il volume deve essere tra 0 e 100"),
            InputLevelError::Device(e) => write!(f, "{e}"),
        }
    }
}

impl std::error::Error for InputLevelError {}

/// Cio' che si sa fare con il volume di un ingresso: lo e' Windows, nelle prove un finto.
pub trait VolumeControl: Send + Sync {
    fn name(&self) -> String;
    fn volume(&self) -> Result<f32, String>;
    fn set_volume(&self, value: f32) -> Result<(), String>;
}

impl VolumeControl for InputEndpoint {
    fn name(&self) -> String {
        InputEndpoint::name(self).to_string()
    }

    fn volume(&self) -> Result<f32, String> {
        InputEndpoint::volume(self).map_err(|e| e.to_string())
    }

    fn set_volume(&self, value: f32) -> Result<(), String> {
        InputEndpoint::set_volume(self, value).map_err(|e| e.to_string())
    }
}

/// Lo stato che l'interfaccia mostra.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct InputStatus {
    pub device: String,
    /// Volume di registrazione di Windows, 0-100.
    pub volume_pct: u8,
    pub auto: bool,
}

#[derive(Serialize, Deserialize, Default)]
#[serde(default)]
struct Saved {
    auto: bool,
}

struct State {
    auto: bool,
    gain: AutoGain,
    transmitting: bool,
}

pub struct InputLevelService {
    control: Arc<dyn VolumeControl>,
    path: PathBuf,
    state: Mutex<State>,
}

fn lock<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
}

fn to_pct(volume: f32) -> u8 {
    (audio_spectrum::clamp_unit(volume) * 100.0).round() as u8
}

impl InputLevelService {
    /// Apre il servizio sul dispositivo dato; la scelta AUTO si ricorda in `path`.
    pub fn open(control: Arc<dyn VolumeControl>, path: &Path) -> Arc<InputLevelService> {
        let saved: Saved = std::fs::read_to_string(path).ok().and_then(|t| serde_json::from_str(&t).ok()).unwrap_or_default();
        Arc::new(InputLevelService { control, path: path.to_path_buf(), state: Mutex::new(State { auto: saved.auto, gain: AutoGain::new(), transmitting: false }) })
    }

    pub fn status(&self) -> Result<InputStatus, InputLevelError> {
        let volume = self.control.volume().map_err(InputLevelError::Device)?;
        Ok(InputStatus { device: self.control.name(), volume_pct: to_pct(volume), auto: lock(&self.state).auto })
    }

    /// Imposta il volume a mano (0-100): da quel momento la regolazione automatica e' spenta, come in Decodium.
    pub fn set_volume_pct(&self, pct: f64) -> Result<(), InputLevelError> {
        if !pct.is_finite() || !(0.0..=100.0).contains(&pct) {
            return Err(InputLevelError::BadValue);
        }
        self.control.set_volume((pct / 100.0) as f32).map_err(InputLevelError::Device)?;
        self.set_auto(false);
        Ok(())
    }

    pub fn set_auto(&self, on: bool) {
        {
            let mut state = lock(&self.state);
            if state.auto != on {
                state.gain = AutoGain::new();
            }
            state.auto = on;
        }
        if let Err(e) = std::fs::write(&self.path, serde_json::to_string(&Saved { auto: on }).unwrap_or_default()) {
            tracing::warn!("scelta AUTO del livello di ingresso non salvata: {e}");
        }
    }

    /// Una misura dell'audio: con AUTO acceso puo' cambiare il volume. Restituisce il nuovo volume (0-1) se l'ha cambiato.
    pub fn on_level(&self, level: &AudioLevel, now_ms: u64) -> Option<f32> {
        let transmitting = {
            let state = lock(&self.state);
            if !state.auto {
                return None;
            }
            state.transmitting
        };
        let volume = self.control.volume().ok()?;
        let wanted = lock(&self.state).gain.on_level(level, volume, now_ms, transmitting)?;
        match self.control.set_volume(wanted) {
            Ok(()) => {
                tracing::info!("livello di ingresso: volume di registrazione {} % -> {} %", to_pct(volume), to_pct(wanted));
                Some(wanted)
            }
            Err(e) => {
                tracing::warn!("regolazione automatica del livello non riuscita: {e}");
                None
            }
        }
    }

    pub fn set_transmitting(&self, on: bool) {
        lock(&self.state).transmitting = on;
    }

    /// Ascolta le misure e lo stato della stazione e applica la regolazione automatica.
    pub fn spawn_auto(self: &Arc<Self>, mut events: broadcast::Receiver<Event>) {
        let service = Arc::clone(self);
        tokio::spawn(async move {
            let start = Instant::now();
            loop {
                match events.recv().await {
                    Ok(Event::AudioLevel(level)) => {
                        let now_ms = start.elapsed().as_millis() as u64;
                        let service = Arc::clone(&service);
                        // la chiamata a Windows e' bloccante
                        let _ = tokio::task::spawn_blocking(move || service.on_level(&level, now_ms)).await;
                    }
                    Ok(Event::Status(status)) => service.set_transmitting(status.transmitting),
                    Ok(_) | Err(broadcast::error::RecvError::Lagged(_)) => {}
                    Err(broadcast::error::RecvError::Closed) => return,
                }
            }
        });
    }
}

#[cfg(test)]
mod tests;
