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

fn powershell(script: &str) -> Result<(), RigError> {
    let out = Command::new("powershell.exe")
        .args(["-NoProfile", "-NonInteractive", "-ExecutionPolicy", "Bypass", "-Command", script])
        .output()
        .map_err(|e| RigError(format!("PowerShell non avviabile: {e}")))?;
    if out.status.success() {
        Ok(())
    } else {
        let why = String::from_utf8_lossy(&out.stderr).trim().to_string();
        Err(RigError(if why.is_empty() { "nessuna risposta".into() } else { why }))
    }
}

impl Rig for WindowsRig {
    fn set_frequency(&self, hz: u64) -> Result<(), RigError> {
        powershell(&script(hz))
    }

    fn set_rx_frequency(&self, hz: u32) -> Result<(), RigError> {
        powershell(&crate::modes::rx_script(hz))
    }

    fn set_mode(&self, mode: &str) -> Result<(), RigError> {
        let button = crate::modes::button_id(mode).ok_or_else(|| RigError(format!("modo sconosciuto: {mode}")))?;
        powershell(&crate::modes::mode_script(button))
    }
}
