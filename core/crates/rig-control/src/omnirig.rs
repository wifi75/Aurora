use std::process::Command;

use crate::{Rig, RigError};

/// Frequenza via OmniRig (radio 1, la stessa che usa JTDX) e modo premendo il pulsante di JTDX: tutto con PowerShell.
pub struct WindowsRig;

/// Lo script contiene solo un numero ricavato da un `u64`: niente testo dell'utente.
///
/// Non si usa `SetSimplexMode`: sulla FT-2000 mette la radio sul VFO B e la lascia (o la rimette) in split, anche se
/// subito dopo si scrive lo split spento. Provato dal vivo, in questo ordine: si scrive `Split` spento
/// (`PM_SPLITOFF` = 0x10000), si aspetta che la radio lo accetti (torna da sola sul VFO A), si scrive la frequenza e si
/// ricontrolla lo split un paio di volte.
///
/// Si scrive `FreqA`, non `Freq`: sulla FT-2000 OmniRig non rende scrivibile `Freq` (`WriteableParams` senza PM_FREQ) e la
/// scrittura viene ignorata senza errore: il comando risulta riuscito ma la radio resta dov'era. Scrivere il VFO subito dopo lo split lo fa riaccendere: non si fa.
pub fn script(hz: u64) -> String {
    format!(
        "$o = New-Object -ComObject OmniRig.OmniRigX; \
         for ($i = 0; $i -lt 8 -and $o.Rig1.Status -ne 4; $i++) {{ Start-Sleep -Milliseconds 400 }}; \
         if ($o.Rig1.Status -ne 4) {{ Write-Error 'radio non in linea'; exit 3 }}; \
         $r = $o.Rig1; $r.Split = 65536; \
         for ($i = 0; $i -lt 8; $i++) {{ Start-Sleep -Milliseconds 400; if ($r.Split -eq 65536) {{ break }} }}; \
         $r.FreqA = {hz}; \
         for ($i = 0; $i -lt 8; $i++) {{ Start-Sleep -Milliseconds 300; if ($r.FreqA -eq {hz}) {{ break }} }}; \
         for ($i = 0; $i -lt 4; $i++) {{ Start-Sleep -Milliseconds 400; if ($r.Split -ne 65536) {{ $r.Split = 65536 }} }}"
    )
}

pub(crate) fn powershell_output(script: &str) -> Result<String, RigError> {
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
        // OmniRig a volte e' occupato un istante (JTDX che legge la radio): un secondo tentativo basta
        powershell(&script(hz)).or_else(|first| {
            tracing::warn!("cambio frequenza, primo tentativo fallito: {first}");
            std::thread::sleep(std::time::Duration::from_millis(600));
            powershell(&script(hz))
        })
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
