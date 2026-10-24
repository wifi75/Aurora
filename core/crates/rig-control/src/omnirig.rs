use std::process::Command;

use crate::{Rig, RigError};

/// OmniRig (radio 1, la stessa che usa JTDX) comandato tramite PowerShell e COM.
pub struct OmniRig;

/// Lo script contiene solo un numero ricavato da un `u64`: niente testo dell'utente.
pub fn script(hz: u64) -> String {
    format!(
        "$o = New-Object -ComObject OmniRig.OmniRigX; \
         if ($o.Rig1.Status -ne 4) {{ Write-Error 'radio non in linea'; exit 3 }}; \
         $o.Rig1.SetSimplexMode({hz})"
    )
}

impl Rig for OmniRig {
    fn set_frequency(&self, hz: u64) -> Result<(), RigError> {
        let out = Command::new("powershell.exe")
            .args(["-NoProfile", "-NonInteractive", "-ExecutionPolicy", "Bypass", "-Command", &script(hz)])
            .output()
            .map_err(|e| RigError(format!("PowerShell non avviabile: {e}")))?;
        if out.status.success() {
            Ok(())
        } else {
            let why = String::from_utf8_lossy(&out.stderr).trim().to_string();
            Err(RigError(if why.is_empty() { "OmniRig non ha risposto".into() } else { why }))
        }
    }
}
