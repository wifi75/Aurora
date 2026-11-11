use std::process::Command;

use crate::session::SharedSession;
use crate::{Rig, RigError};

/// La sessione PowerShell che tiene aperto l'oggetto OmniRig per tutto il collegamento, come fa WSJT-X (un solo `OmniRigX` creato all'avvio e tenuto in
/// vita). Prima ogni lettura o cambio di frequenza lanciava un `powershell.exe` nuovo che creava l'oggetto e usciva: OmniRig, che e' un programma a parte,
/// si chiude quando l'ultimo cliente se ne va e si riapre al successivo mostrando la sua finestrella (segnalazione #38: «si apre in permanenza») e a ogni
/// riapertura la radio risultava un istante fuori linea (collegamento «a intermittenza»).
static SESSION: SharedSession = SharedSession::new();

/// L'inizio di ogni script: l'oggetto OmniRig si crea una volta sola nella sessione e poi si riusa.
const OBJECT: &str = "if ($global:AuroraOmni) { try { $null = $global:AuroraOmni.InterfaceVersion } catch { $global:AuroraOmni = $null } }; if (-not $global:AuroraOmni) { $global:AuroraOmni = New-Object -ComObject OmniRig.OmniRigX }; $o = $global:AuroraOmni; ";

/// Quale delle due radio di OmniRig si comanda: 1 (`Rig1`) o 2 (`Rig2`), scelta in Impostazioni › Radio (richiesta di IK1SOW, 2026-11-09).
static RIG: std::sync::atomic::AtomicU8 = std::sync::atomic::AtomicU8::new(1);

/// Sceglie la radio di OmniRig (1 o 2; ogni altro valore vale 1).
pub fn set_rig(n: u8) {
    RIG.store(if n == 2 { 2 } else { 1 }, std::sync::atomic::Ordering::Relaxed);
}

/// La radio di OmniRig scelta (1 o 2).
pub fn rig() -> u8 {
    RIG.load(std::sync::atomic::Ordering::Relaxed)
}

/// Lo split scelto in Impostazioni › Radio per OmniRig: 0 nessuno, 1 «Rig» (split vero, `SetSplitMode`), 2 «Fake It» (simulato).
static SPLIT: std::sync::atomic::AtomicU8 = std::sync::atomic::AtomicU8::new(0);
/// La frequenza audio di trasmissione (Hz, 0 = non ancora nota): da essa l'XIT dello split.
static TX_AUDIO: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
/// Split simulato: la frequenza di ricezione a cui tornare dopo la trasmissione (0 = la radio non e' stata spostata).
static EMULATE_RESTORE: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
/// Lo split vero e' stato acceso da Aurora (`SetSplitMode`): al distacco si spegne.
static SPLIT_BY_US: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// Sceglie lo split per OmniRig (dalla configurazione della radio).
pub fn set_split_mode(mode: crate::cat_config::SplitMode) {
    let code = match mode {
        crate::cat_config::SplitMode::None => 0,
        crate::cat_config::SplitMode::Rig => 1,
        crate::cat_config::SplitMode::Emulate => 2,
    };
    if SPLIT.swap(code, std::sync::atomic::Ordering::SeqCst) != code {
        // con un'altra scelta un vecchio ritorno simulato non vale piu'
        EMULATE_RESTORE.store(0, std::sync::atomic::Ordering::SeqCst);
    }
}

fn split_code() -> u8 {
    SPLIT.load(std::sync::atomic::Ordering::SeqCst)
}

/// Lo spostamento (XIT) dello split per la frequenza audio scelta, se lo split vero e' attivo e la frequenza audio e' nota.
fn rig_split_xit() -> Option<i64> {
    let audio = TX_AUDIO.load(std::sync::atomic::Ordering::SeqCst);
    (split_code() == 1 && audio != 0).then(|| crate::cat_config::split_xit_hz(audio))
}

/// La frequenza audio di trasmissione (`setXIT` di WSJT-X). Con «Rig» lo split vero si rimette subito come `do_tx_frequency` di
/// `OmniRigTransceiver.cpp`: `SetSplitMode (rx, tx)` sulla frequenza di ricezione letta dalla radio. Con «Fake It» a trasmissione in corso la
/// radio va subito sulla nuova frequenza di trasmissione (`EmulateSplitTransceiver`); prima del PTT serve solo a [`emulate_begin`].
pub fn set_tx_audio_frequency(hz: u32) -> Result<(), RigError> {
    if hz > 5000 {
        return Err(RigError(format!("frequenza audio di trasmissione non valida: {hz} Hz")));
    }
    TX_AUDIO.store(hz, std::sync::atomic::Ordering::SeqCst);
    if hz == 0 {
        return Ok(());
    }
    let n = rig();
    let xit = crate::cat_config::split_xit_hz(hz);
    match split_code() {
        1 => {
            powershell(&format!(
                "{OBJECT}$r = $o.Rig{n}; if ($r.Status -ne 4) {{ Write-Error 'radio non in linea'; exit 3 }}; \
                 $rx = [int64]$r.GetRxFrequency(); $r.SetSplitMode($rx, $rx + ({xit}))"
            ))?;
            SPLIT_BY_US.store(true, std::sync::atomic::Ordering::SeqCst);
            Ok(())
        }
        2 => {
            let restore = EMULATE_RESTORE.load(std::sync::atomic::Ordering::SeqCst);
            if restore == 0 {
                // non si sta trasmettendo con la radio spostata: si applica al prossimo PTT
                return Ok(());
            }
            let tx = crate::cat_config::split_tx_dial_hz(restore, hz);
            powershell(&move_script(tx))
        }
        _ => Ok(()),
    }
}

/// Scrive `FreqA` e aspetta che la radio la confermi (al massimo 2,4 s), senza toccare split e modo: e' il salto breve dello split simulato.
fn move_script(hz: u64) -> String {
    let n = rig();
    format!(
        "{OBJECT}$r = $o.Rig{n}; if ($r.Status -ne 4) {{ Write-Error 'radio non in linea'; exit 3 }}; \
         $r.FreqA = {hz}; for ($i = 0; $i -lt 24; $i++) {{ if ($r.FreqA -eq {hz}) {{ break }}; Start-Sleep -Milliseconds 100 }}"
    )
}

/// Split simulato («Fake It»), prima del PTT: la radio va sulla frequenza di trasmissione (ricezione + XIT) e si ricorda dove tornare, come
/// `EmulateSplitTransceiver` di WSJT-X. Senza split simulato o senza frequenza audio non fa nulla.
pub fn emulate_begin() -> Result<(), RigError> {
    let audio = TX_AUDIO.load(std::sync::atomic::Ordering::SeqCst);
    if split_code() != 2 || audio == 0 || EMULATE_RESTORE.load(std::sync::atomic::Ordering::SeqCst) != 0 {
        return Ok(());
    }
    let dial = WindowsRig.frequency()?;
    let tx = crate::cat_config::split_tx_dial_hz(dial, audio);
    if tx == dial {
        return Ok(());
    }
    powershell(&move_script(tx))?;
    EMULATE_RESTORE.store(dial, std::sync::atomic::Ordering::SeqCst);
    Ok(())
}

/// Split simulato, dopo il PTT: la radio torna sulla frequenza di ricezione.
pub fn emulate_end() -> Result<(), RigError> {
    let back = EMULATE_RESTORE.swap(0, std::sync::atomic::Ordering::SeqCst);
    if back == 0 {
        return Ok(());
    }
    powershell(&move_script(back))
}

/// Al distacco: con lo split vero acceso da Aurora lo si spegne (`PM_SPLITOFF`), con quello simulato la radio torna sulla ricezione. Non si usa
/// `SetSimplexMode` (vedi [`script`]).
fn split_cleanup() {
    if let Err(e) = emulate_end() {
        tracing::warn!("OmniRig: ritorno alla frequenza di ricezione non riuscito: {e}");
    }
    if SPLIT_BY_US.swap(false, std::sync::atomic::Ordering::SeqCst) {
        let n = rig();
        if let Err(e) = powershell(&format!("{OBJECT}$r = $o.Rig{n}; if ($r.Status -eq 4 -and $r.Split -ne 65536) {{ $r.Split = 65536 }}")) {
            tracing::warn!("OmniRig: split non spento al distacco: {e}");
        }
    }
}

/// La radio via OmniRig (Rig1 o Rig2, vedi [`set_rig`]): solo frequenza. OmniRig e' un driver CAT facoltativo che l'utente sceglie in Impostazioni › Radio;
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
///
/// Con lo split vero («Rig») e la frequenza audio nota, come `do_frequency` + `do_tx_frequency` di WSJT-X: si scrive `FreqA` e poi
/// `SetSplitMode (rx, rx + XIT)`, senza spegnere lo split.
pub fn script(hz: u64) -> String {
    let n = rig();
    if let Some(xit) = rig_split_xit() {
        let tx = (hz as i64 + xit).max(0);
        return format!(
            "{OBJECT}\
             for ($i = 0; $i -lt 8 -and $o.Rig{n}.Status -ne 4; $i++) {{ Start-Sleep -Milliseconds 400 }}; \
             if ($o.Rig{n}.Status -ne 4) {{ Write-Error 'radio non in linea'; exit 3 }}; \
             $r = $o.Rig{n}; \
             $m0 = $r.Mode; \
             $r.FreqA = {hz}; \
             for ($i = 0; $i -lt 24; $i++) {{ if ($r.FreqA -eq {hz}) {{ break }}; Start-Sleep -Milliseconds 100 }}; \
             $r.SetSplitMode({hz}, {tx}); \
             Start-Sleep -Milliseconds 200; \
             if ($m0 -ne 0 -and $r.Mode -ne $m0 -and (($r.WriteableParams -band $m0) -ne 0)) {{ \
             $r.Mode = $m0; for ($i = 0; $i -lt 10; $i++) {{ Start-Sleep -Milliseconds 100; if ($r.Mode -eq $m0) {{ break }} }} }}"
        );
    }
    format!(
        "{OBJECT}\
         for ($i = 0; $i -lt 8 -and $o.Rig{n}.Status -ne 4; $i++) {{ Start-Sleep -Milliseconds 400 }}; \
         if ($o.Rig{n}.Status -ne 4) {{ Write-Error 'radio non in linea'; exit 3 }}; \
         $r = $o.Rig{n}; \
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

/// Dove si scarica OmniRig (sito del suo autore).
pub const OMNIRIG_URL: &str = "https://www./omnirig/";

/// OmniRig e' installato su questo PC? Si guarda nel registro di Windows se c'e' il suo oggetto (`OmniRig.OmniRigX`), al massimo una volta
/// al minuto: lo stato del CAT si chiede ogni pochi secondi. Fuori da Windows OmniRig non esiste.
pub fn installed() -> bool {
    #[cfg(windows)]
    {
        use std::sync::Mutex;
        use std::time::{Duration, Instant};
        static LAST: Mutex<Option<(Instant, bool)>> = Mutex::new(None);
        if let Ok(mut last) = LAST.lock() {
            if let Some((at, found)) = *last
                && at.elapsed() < Duration::from_secs(60)
            {
                return found;
            }
            let found = {
                use std::os::windows::process::CommandExt;
                Command::new("reg.exe")
                    .args(["query", r"HKCR\OmniRig.OmniRigX"])
                    .creation_flags(0x0800_0000)
                    .output()
                    .is_ok_and(|o| o.status.success())
            };
            *last = Some((Instant::now(), found));
            return found;
        }
        false
    }
    #[cfg(not(windows))]
    {
        false
    }
}

/// Un errore di PowerShell con OmniRig detto in chiaro: «classe non registrata» (in qualunque lingua di Windows) vuol dire che OmniRig non c'e'.
pub fn explain_omnirig(why: &str) -> String {
    if why.contains("REGDB_E_CLASSNOTREG") || why.contains("80040154") || why.contains("CLSID") {
        format!("OmniRig non e' installato su questo PC: in Impostazioni > Radio scegli Hamlib (gia' incluso in Aurora) oppure installa OmniRig da {OMNIRIG_URL}")
    } else {
        why.to_string()
    }
}

pub(crate) fn powershell_output(script: &str) -> Result<String, RigError> {
    // l'oggetto NON si butta dopo un errore («radio non in linea» e' normale a radio spenta): sarebbe l'unico cliente e OmniRig si chiuderebbe e
    // riaprirebbe con la sua finestrella a ogni lettura. Se OmniRig e' stato chiuso, il preambolo (`OBJECT`) se ne accorge e lo ricrea.
    // Gli script aspettano gia' da soli la radio e chi chiama ha i suoi tentativi: un errore di script non si ripete qui.
    SESSION.run_once(script).map(|out| out.trim().to_string()).map_err(|e| {
        let why = e.0.trim();
        RigError(if why.is_empty() { "nessuna risposta".into() } else { explain_omnirig(why) })
    })
}

/// Lascia OmniRig: chiude la sessione che teneva l'oggetto aperto (al distacco del CAT o al cambio di sistema di controllo), cosi' OmniRig puo' chiudersi
/// e liberare la porta della radio per un altro programma o per Hamlib.
pub fn release() {
    split_cleanup();
    SESSION.close();
}

fn powershell(script: &str) -> Result<(), RigError> {
    powershell_output(script).map(|_| ())
}

/// Con OmniRig Aurora comanda solo la frequenza della radio: modo, audio e trasmissione sono del motore di Aurora.
const NOT_HERE: &str = "con OmniRig Aurora comanda solo la frequenza della radio";

impl Rig for WindowsRig {
    fn frequency(&self) -> Result<u64, RigError> {
        let out = powershell_output(&format!(
            "{OBJECT}$r = $o.Rig{}; if ($r.Status -ne 4) {{ Write-Error 'radio non in linea'; exit 3 }}; [int64]$r.GetRxFrequency()",
            rig()
        ))?;
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
        })?;
        // con lo split vero lo script l'ha acceso (`SetSplitMode`): al distacco va spento
        if rig_split_xit().is_some() {
            SPLIT_BY_US.store(true, std::sync::atomic::Ordering::SeqCst);
        }
        Ok(())
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
