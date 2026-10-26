use std::process::Command;

use crate::{session::SharedSession, Rig, RigError};

/// Sessioni PowerShell sempre aperte: una per i comandi a JTDX (veloci) e una per le letture periodiche, cosi' una lettura
/// lenta non fa aspettare mai un comando.
static COMMANDS: SharedSession = SharedSession::new();
static READS: SharedSession = SharedSession::new();

/// Frequenza via OmniRig (radio 1, la stessa che usa JTDX) e modo premendo il pulsante di JTDX: tutto con PowerShell.
pub struct WindowsRig;

/// Lo script contiene solo un numero ricavato da un `u64`: niente testo dell'utente.
///
/// Il modo della radio (per esempio USB dati) si ricorda prima del cambio e si rimette dopo se la radio, cambiando banda, ha
/// richiamato quello che aveva memorizzato per quella banda: JTDX ha bisogno del modo di prima, o i segnali escono distorti.
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
        Err(RigError(if why.is_empty() { "nessuna risposta".into() } else { why }))
    }
}

fn powershell(script: &str) -> Result<(), RigError> {
    powershell_output(script).map(|_| ())
}

/// Un comando a JTDX nella sessione dei comandi.
fn command(script: &str) -> Result<(), RigError> {
    COMMANDS.run(script).map(|_| ())
}

/// Una lettura di JTDX nella sessione delle letture.
fn read(script: &str) -> Result<String, RigError> {
    READS.run(script)
}

impl WindowsRig {
    /// Apre le due sessioni e ricorda i controlli di JTDX subito (cosi' il primo comando non paga l'avvio) e poi ogni cinque minuti
    /// (la memoria scade dopo dieci). Senza JTDX acceso non fa nulla di male: riprova al giro dopo.
    pub fn keep_warm() {
        std::thread::spawn(|| {
            loop {
                let warm = format!("{}'ok'", crate::controls::HEAD);
                let _ = COMMANDS.run(&warm);
                let _ = READS.run(&warm);
                std::thread::sleep(std::time::Duration::from_secs(300));
            }
        });
    }
}

impl Rig for WindowsRig {
    fn frequency(&self) -> Result<u64, RigError> {
        let out = powershell_output(
            "$o = New-Object -ComObject OmniRig.OmniRigX; $r = $o.Rig1; if ($r.Status -ne 4) { Write-Error 'radio non in linea'; exit 3 }; [int64]$r.GetRxFrequency()",
        )?;
        out.trim().parse::<u64>().map_err(|_| RigError(format!("frequenza illeggibile: {out}")))
    }

    fn set_frequency(&self, hz: u64) -> Result<(), RigError> {
        // OmniRig a volte e' occupato un istante (JTDX che legge la radio): un secondo tentativo basta
        powershell(&script(hz)).or_else(|first| {
            tracing::warn!("cambio frequenza, primo tentativo fallito: {first}");
            std::thread::sleep(std::time::Duration::from_millis(600));
            powershell(&script(hz))
        })
    }

    fn set_rx_frequency(&self, hz: u32) -> Result<(), RigError> {
        command(&crate::modes::rx_script(hz))
    }

    fn output_attenuation(&self) -> Result<u32, RigError> {
        let text = read(&crate::modes::attenuation_read_script())?;
        text.parse().map_err(|_| RigError(format!("valore inatteso da JTDX: {text}")))
    }

    fn set_output_attenuation(&self, tenths: u32) -> Result<(), RigError> {
        command(&crate::modes::attenuation_set_script(tenths))
    }

    fn set_mode(&self, mode: &str) -> Result<(), RigError> {
        let button = crate::modes::button_id(mode).ok_or_else(|| RigError(format!("modo sconosciuto: {mode}")))?;
        command(&crate::modes::mode_script(button))
    }

    fn controls(&self) -> Result<Vec<(String, bool)>, RigError> {
        Ok(self.controls_and_tx()?.0)
    }

    fn controls_and_tx(&self) -> Result<crate::controls::Reading, RigError> {
        let text = read(&crate::controls::read_script())?;
        Ok(crate::controls::parse_reading(&text))
    }

    fn engine_snapshot(&self) -> Result<crate::snapshot::Snapshot, RigError> {
        Ok(crate::snapshot::parse(&read(&crate::snapshot::read_script())?))
    }

    fn select_tx_message(&self, n: u8) -> Result<(), RigError> {
        if !(1..=6).contains(&n) {
            return Err(RigError(format!("messaggio Tx non valido: {n}")));
        }
        command(&crate::controls::select_tx_script(n))
    }

    fn set_control(&self, id: &str, on: Option<bool>) -> Result<(), RigError> {
        let control = crate::controls::find(id).ok_or_else(|| RigError(format!("controllo sconosciuto: {id}")))?;
        if control.own {
            return Err(RigError(format!("«{id}» esiste solo nel motore proprio di Aurora, non in JTDX")));
        }
        command(&crate::controls::set_script(control, on))
    }
}
