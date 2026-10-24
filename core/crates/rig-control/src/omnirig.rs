use std::process::Command;

use crate::{Rig, RigError};

/// Frequenza via OmniRig (radio 1, la stessa che usa JTDX) e modo premendo il pulsante di JTDX: tutto con PowerShell.
pub struct WindowsRig;

/// Lo script contiene solo un numero ricavato da un `u64`: niente testo dell'utente.
pub fn script(hz: u64) -> String {
    format!(
        "$o = New-Object -ComObject OmniRig.OmniRigX; \
         if ($o.Rig1.Status -ne 4) {{ Write-Error 'radio non in linea'; exit 3 }}; \
         $o.Rig1.SetSimplexMode({hz})"
    )
}

fn powershell_output(script: &str) -> Result<String, RigError> {
    let out = Command::new("powershell.exe")
        .args(["-NoProfile", "-NonInteractive", "-ExecutionPolicy", "Bypass", "-Command", script])
        .output()
        .map_err(|e| RigError(format!("PowerShell non avviabile: {e}")))?;
    if out.status.success() {
        Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
    } else {
        let why = String::from_utf8_lossy(&out.stderr).trim().to_string();
        Err(RigError(if why.is_empty() { "nessuna risposta".into() } else { why }))
    }
}

fn powershell(script: &str) -> Result<(), RigError> {
    powershell_output(script).map(|_| ())
}

impl Rig for WindowsRig {
    fn set_frequency(&self, hz: u64) -> Result<(), RigError> {
        powershell(&script(hz))
    }

    fn set_rx_frequency(&self, hz: u32) -> Result<(), RigError> {
        powershell(&crate::modes::rx_script(hz))
    }

    fn output_attenuation(&self) -> Result<u32, RigError> {
        let text = powershell_output(&crate::modes::attenuation_read_script())?;
        text.parse().map_err(|_| RigError(format!("valore inatteso da JTDX: {text}")))
    }

    fn set_output_attenuation(&self, tenths: u32) -> Result<(), RigError> {
        powershell(&crate::modes::attenuation_set_script(tenths))
    }

    fn set_mode(&self, mode: &str) -> Result<(), RigError> {
        let button = crate::modes::button_id(mode).ok_or_else(|| RigError(format!("modo sconosciuto: {mode}")))?;
        powershell(&crate::modes::mode_script(button))
    }

    fn controls(&self) -> Result<Vec<(String, bool)>, RigError> {
        let text = powershell_output(&crate::controls::read_script())?;
        Ok(text
            .lines()
            .filter_map(|line| {
                let (id, value) = line.trim().split_once('=')?;
                crate::controls::find(id)?;
                Some((id.to_string(), value.trim() == "1"))
            })
            .collect())
    }

    fn set_control(&self, id: &str, on: Option<bool>) -> Result<(), RigError> {
        let control = crate::controls::find(id).ok_or_else(|| RigError(format!("controllo sconosciuto: {id}")))?;
        powershell(&crate::controls::set_script(control, on))
    }
}
