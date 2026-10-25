//! Volume di registrazione di Windows dell'ingresso audio (quello del cursore «Registrazione» del dispositivo).
//!
//! E' il livello che conta davvero: JTDX non ha un controllo dell'ingresso, e un ingresso saturo non si corregge dopo.
//! Si agisce sul dispositivo, che e' lo stesso per JTDX e per la cascata di Aurora.

use cpal::traits::{DeviceTrait, HostTrait};

#[derive(Debug, PartialEq, Eq)]
pub enum VolumeError {
    /// Nessun ingresso corrisponde al nome cercato.
    NoDevice(String),
    /// Il controllo del volume non e' disponibile su questa piattaforma o per questo dispositivo.
    Unavailable(String),
}

impl std::fmt::Display for VolumeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            VolumeError::NoDevice(hint) => write!(f, "ingresso audio non trovato: {hint}"),
            VolumeError::Unavailable(why) => write!(f, "volume di registrazione non disponibile: {why}"),
        }
    }
}

impl std::error::Error for VolumeError {}

/// L'ingresso audio di cui si regola il volume, scelto per parte del nome come per la cascata.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InputEndpoint {
    id: String,
    name: String,
}

impl InputEndpoint {
    /// Trova l'ingresso per nome: prima quello identico, poi quello che contiene `hint` (senza distinzione di maiuscole).
    pub fn find(hint: &str) -> Result<InputEndpoint, VolumeError> {
        let host = cpal::default_host();
        let devices: Vec<cpal::Device> = host.input_devices().map_err(|e| VolumeError::Unavailable(e.to_string()))?.collect();
        let names: Vec<String> = devices.iter().map(|d| d.description().map(|d| d.name().to_string()).unwrap_or_default()).collect();
        let Some(index) = crate::match_device_name(&names, hint) else {
            return Err(VolumeError::NoDevice(hint.to_string()));
        };
        let id = devices[index].id().map_err(|e| VolumeError::Unavailable(e.to_string()))?;
        Ok(InputEndpoint { id: id.id().to_string(), name: names[index].clone() })
    }

    /// L'ingresso predefinito del sistema.
    pub fn find_default() -> Result<InputEndpoint, VolumeError> {
        let device = cpal::default_host().default_input_device().ok_or_else(|| VolumeError::NoDevice("predefinito".into()))?;
        let name = device.description().map(|d| d.name().to_string()).unwrap_or_default();
        let id = device.id().map_err(|e| VolumeError::Unavailable(e.to_string()))?;
        Ok(InputEndpoint { id: id.id().to_string(), name })
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    /// Volume attuale, da 0.0 a 1.0.
    pub fn volume(&self) -> Result<f32, VolumeError> {
        platform::get(&self.id)
    }

    /// Imposta il volume, da 0.0 a 1.0 (valori fuori scala vengono riportati nei limiti).
    pub fn set_volume(&self, value: f32) -> Result<(), VolumeError> {
        platform::set(&self.id, clamp_unit(value))
    }
}

/// Riporta un valore in [0, 1]; un valore non numerico vale 0.
pub fn clamp_unit(value: f32) -> f32 {
    if value.is_nan() { 0.0 } else { value.clamp(0.0, 1.0) }
}

#[cfg(windows)]
mod platform {
    use windows::{
        Win32::{
            Media::Audio::{Endpoints::IAudioEndpointVolume, IMMDeviceEnumerator, MMDeviceEnumerator},
            System::Com::{CLSCTX_ALL, COINIT_MULTITHREADED, CoCreateInstance, CoInitializeEx, CoUninitialize},
        },
        core::HSTRING,
    };

    use super::VolumeError;

    /// Tiene COM aperto per il thread corrente finche' vive.
    struct ComGuard(bool);

    impl ComGuard {
        fn new() -> ComGuard {
            // se il thread l'ha gia' aperto in un altro modo l'uso resta valido: non si chiude cio' che non si e' aperto noi
            ComGuard(unsafe { CoInitializeEx(None, COINIT_MULTITHREADED) }.is_ok())
        }
    }

    impl Drop for ComGuard {
        fn drop(&mut self) {
            if self.0 {
                unsafe { CoUninitialize() };
            }
        }
    }

    fn unavailable(e: windows::core::Error) -> VolumeError {
        VolumeError::Unavailable(e.message())
    }

    fn endpoint(id: &str) -> Result<IAudioEndpointVolume, VolumeError> {
        unsafe {
            let enumerator: IMMDeviceEnumerator = CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL).map_err(unavailable)?;
            let device = enumerator.GetDevice(&HSTRING::from(id)).map_err(unavailable)?;
            device.Activate::<IAudioEndpointVolume>(CLSCTX_ALL, None).map_err(unavailable)
        }
    }

    pub fn get(id: &str) -> Result<f32, VolumeError> {
        let _com = ComGuard::new();
        unsafe { endpoint(id)?.GetMasterVolumeLevelScalar() }.map_err(unavailable)
    }

    pub fn set(id: &str, value: f32) -> Result<(), VolumeError> {
        let _com = ComGuard::new();
        unsafe { endpoint(id)?.SetMasterVolumeLevelScalar(value, std::ptr::null()) }.map_err(unavailable)
    }
}

#[cfg(not(windows))]
mod platform {
    use super::VolumeError;

    pub fn get(_id: &str) -> Result<f32, VolumeError> {
        Err(VolumeError::Unavailable("solo su Windows".into()))
    }

    pub fn set(_id: &str, _value: f32) -> Result<(), VolumeError> {
        Err(VolumeError::Unavailable("solo su Windows".into()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clamps_to_the_unit_range() {
        assert_eq!(clamp_unit(-0.5), 0.0);
        assert_eq!(clamp_unit(1.7), 1.0);
        assert_eq!(clamp_unit(0.42), 0.42);
        assert_eq!(clamp_unit(f32::NAN), 0.0);
    }

    #[test]
    fn an_unknown_device_is_reported_by_name() {
        // nessuna scheda si chiama cosi'; senza schede audio l'elenco stesso puo' non essere disponibile
        match InputEndpoint::find("zzz-nessuna-scheda-zzz") {
            Err(VolumeError::NoDevice(hint)) => assert_eq!(hint, "zzz-nessuna-scheda-zzz"),
            Err(VolumeError::Unavailable(_)) => {}
            Ok(found) => panic!("non doveva trovare {}", found.name()),
        }
    }

    /// Prova con la scheda vera: `cargo test -p audio-spectrum -- --ignored --nocapture` (AURORA_TEST_AUDIO = parte del nome). Legge soltanto.
    #[test]
    #[ignore = "richiede una scheda audio vera"]
    fn reads_the_volume_of_a_real_input() {
        let hint = std::env::var("AURORA_TEST_AUDIO").unwrap_or_else(|_| "USB Audio".into());
        let input = InputEndpoint::find(&hint).expect("ingresso trovato");
        let volume = input.volume().expect("volume letto");
        println!("{}: volume di registrazione {:.0} %", input.name(), volume * 100.0);
        assert!((0.0..=1.0).contains(&volume));
    }
}
