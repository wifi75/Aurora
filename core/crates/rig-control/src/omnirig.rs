use std::process::Command;

use crate::{Rig, RigError};

/// La radio via OmniRig (radio 1): solo frequenza. OmniRig e' un driver CAT facoltativo che l'utente sceglie in Impostazioni › Radio;
/// Aurora non comanda nessun altro programma.
pub struct WindowsRig;

/// Lo script contiene solo un numero ricavato da un `u64`: niente testo dell'utente.
///
/// Il modo della radio (per esempio USB dati) si ricorda prima del cambio e si rimette dopo se la radio, cambiando banda, ha
/// richiamato quello che aveva memorizzato per quella banda: il modo dati di prima serve, o i segnali escono distorti.
/// Si riscrive solo se e' cambiato e se OmniRig lo permette per quella radio (`WriteableParams`).
///
/// Non si usa `SetSimplexMode`: sulla FT-2000 mette la radio sul VFO B e la lascia (o la rimette) in split, anche se
/// subito dopo si scrive lo split spento. Provato dal vivo, in questo ordine: si scrive `Split` spento
/// (`PM_SPLITOFF` = 0x10000) solo se non e' gia' spento (di norma lo e': il percorso veloce non aspetta nulla), si scrive
/// la frequenza e si ricontrolla lo split una volta dopo un attimo. Le attese sono a scatti brevi con uscita appena la
/// radio ha risposto: il cambio banda dura poco piu' dell'avvio di PowerShell.
///
/// Si scrive `FreqA`, non `Freq`: sulla FT-2000 OmniRig non rende scrivibile `Freq` (`WriteableParams` senza PM_FREQ) e la
/// scrittura viene ignorata senza errore: il comando risulta riuscito ma la radio resta dov'era. Scrivere il VFO subito dopo lo split lo fa riaccendere: non si fa.
pub fn script(hz: u64) -> String {
    format!(
        "$o = New-Object -ComObject OmniRig.OmniRigX; \
         for ($i = 0; $i -lt 8 -and $o.Rig1.Status -ne 4; $i++) {{ Start-Sleep -Milliseconds 400 }}; \
         if ($o.Rig1.Status -ne 4) {{ Write-Error 'radio non in linea'; exit 3 }}; \
         $r = $o.Rig1; \
         $m0 = $r.Mode; \
         if ($r.Split -ne 65536) {{ $r.Split = 65536; for ($i = 0; $i -lt 16; $i++) {{ Start-Sleep -Milliseconds 200; if ($r.Split -eq 65536) {{ break }} }} }}; \
         $r.FreqA = {hz}; \
         for ($i = 0; $i -lt 24; $i++) {{ if ($r.FreqA -eq {hz}) {{ break }}; Start-Sleep -Milliseconds 100 }}; \
         Start-Sleep -Milliseconds 300; if ($r.Split -ne 65536) {{ $r.Split = 65536 }}; \
         Start-Sleep -Milliseconds 200; \
         if ($m0 -ne 0 -and $r.Mode -ne $m0 -and (($r.WriteableParams -band $m0) -ne 0)) {{ \
         $r.Mode = $m0; for ($i = 0; $i -lt 10; $i++) {{ Start-Sleep -Milliseconds 100; if ($r.Mode -eq $m0) {{ break }} }} }}"
    )
}

/// Un errore di PowerShell con OmniRig detto in chiaro: «classe non registrata» (in qualunque lingua di Windows) vuol dire che OmniRig non c'e'.
pub fn explain_omnirig(why: &str) -> String {
    if why.contains("REGDB_E_CLASSNOTREG") || why.contains("80040154") || (why.contains("CLSID") && why.contains("New-Object")) {
        "OmniRig non e' installato su questo PC: in Impostazioni > Radio scegli Hamlib (gia' incluso in Aurora) oppure installa OmniRig".to_string()
    } else {
        why.to_string()
    }
}

pub(crate) fn powershell_output(script: &str) -> Result<String, RigError> {
    let mut command = Command::new("powershell.exe");
    command.args(["-NoProfile", "-NonInteractive", "-ExecutionPolicy", "Bypass", "-Command", script]);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        // niente finestra nera quando Aurora gira come programma a finestre (cambio banda, letture)
        command.creation_flags(0x0800_0000);
    }
    let out = command.output().map_err(|e| RigError(format!("PowerShell non avviabile: {e}")))?;
    if out.status.success() {
        Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
    } else {
        let why = String::from_utf8_lossy(&out.stderr).trim().to_string();
        Err(RigError(if why.is_empty() { "nessuna risposta".into() } else { explain_omnirig(&why) }))
    }
}

fn powershell(script: &str) -> Result<(), RigError> {
    powershell_output(script).map(|_| ())
}

/// Con OmniRig Aurora comanda solo la frequenza della radio: modo, audio e trasmissione sono del motore di Aurora.
const NOT_HERE: &str = "con OmniRig Aurora comanda solo la frequenza della radio";

impl Rig for WindowsRig {
    fn frequency(&self) -> Result<u64, RigError> {
        let out = powershell_output(
            "$o = New-Object -ComObject OmniRig.OmniRigX; $r = $o.Rig1; if ($r.Status -ne 4) { Write-Error 'radio non in linea'; exit 3 }; [int64]$r.GetRxFrequency()",
        )?;
        out.trim().parse::<u64>().map_err(|_| RigError(format!("frequenza illeggibile: {out}")))
    }

    fn set_frequency(&self, hz: u64) -> Result<(), RigError> {
        // OmniRig a volte e' occupato un istante (un altro programma che legge la radio): un secondo tentativo basta
        powershell(&script(hz)).or_else(|first| {
            tracing::warn!("cambio frequenza, primo tentativo fallito: {first}");
            std::thread::sleep(std::time::Duration::from_millis(600));
            // terzo tentativo dopo una pausa piu' lunga: OmniRig a volte si riprende dopo un paio di secondi (radio che si risveglia)
            powershell(&script(hz)).or_else(|second| {
                tracing::warn!("cambio frequenza, secondo tentativo fallito: {second}");
                std::thread::sleep(std::time::Duration::from_millis(2000));
                powershell(&script(hz)).inspect_err(|e| aurora_core::notice::notice("omnirig", e.0.clone()))
            })
        })
    }

    fn set_rx_frequency(&self, _hz: u32) -> Result<(), RigError> {
        Err(RigError(NOT_HERE.into()))
    }

    fn output_attenuation(&self) -> Result<u32, RigError> {
        Err(RigError(NOT_HERE.into()))
    }

    fn set_output_attenuation(&self, _tenths: u32) -> Result<(), RigError> {
        Err(RigError(NOT_HERE.into()))
    }

    fn set_mode(&self, _mode: &str) -> Result<(), RigError> {
        Err(RigError(NOT_HERE.into()))
    }

    fn set_extra(&self, _item: crate::extras::ExtraItem) -> Result<(), RigError> {
        Err(RigError("con OmniRig le impostazioni della radio (preamplificatore, AGC, NB, VFO...) non si cambiano da qui: scegli il CAT Hamlib in Impostazioni → Radio".into()))
    }
}
