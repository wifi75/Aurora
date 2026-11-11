//! La radio attraverso OmniRig (facoltativo, solo Windows), comandata direttamente via COM come fa `OmniRigTransceiver.cpp` di WSJT-X: niente
//! piu' PowerShell (vedi [`crate::omnirig_com`]). Qui ci sono le sequenze dei comandi, scritte su [`RigApi`] cosi' le prove le fanno girare
//! senza OmniRig; il thread COM e gli oggetti vivono in `omnirig_com.rs`.
//!
//! L'oggetto OmniRig si crea una volta e resta in vita per tutto il collegamento, come in WSJT-X (un solo `OmniRigX` creato in `do_start`):
//! OmniRig, che e' un programma a parte, si chiude quando l'ultimo cliente se ne va e si riapre al successivo mostrando la sua finestrella
//! (segnalazione #38: «si apre in permanenza»). Si lascia solo al distacco del CAT ([`release`]), per liberare la porta della radio.

use crate::{Rig, RigError};

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
/// Il PTT di OmniRig e' stato acceso da Aurora e non ancora spento.
static PTT_ON: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

// I parametri di OmniRig (`RigParamX` della sua libreria dei tipi). Verificati in wsjtx.exe 3.3.0-beta6 con Ghidra: i test bit a bit di
// `handle_params_change` (FUN_1404ac450) seguono uno per uno l'ordine di `OmniRigTransceiver.cpp:428-650`, quelli di `do_frequency`
// (FUN_1404aa820) e `do_tx_frequency` (FUN_1404aae50) le righe 722-736 e 753-796, il PTT di `do_ptt` (FUN_1404b0ed0) la riga 685.
pub const PM_UNKNOWN: i32 = 0x0000_0001;
pub const PM_FREQ: i32 = 0x0000_0002;
pub const PM_FREQA: i32 = 0x0000_0004;
pub const PM_FREQB: i32 = 0x0000_0008;
pub const PM_VFOAA: i32 = 0x0000_0080;
pub const PM_VFOAB: i32 = 0x0000_0100;
pub const PM_VFOBA: i32 = 0x0000_0200;
pub const PM_VFOBB: i32 = 0x0000_0400;
pub const PM_VFOA: i32 = 0x0000_0800;
pub const PM_VFOB: i32 = 0x0000_1000;
pub const PM_SPLITON: i32 = 0x0000_8000;
pub const PM_SPLITOFF: i32 = 0x0001_0000;
pub const PM_RX: i32 = 0x0020_0000;
pub const PM_TX: i32 = 0x0040_0000;
pub const PM_CW_U: i32 = 0x0080_0000;
pub const PM_CW_L: i32 = 0x0100_0000;
pub const PM_SSB_U: i32 = 0x0200_0000;
pub const PM_SSB_L: i32 = 0x0400_0000;
pub const PM_DIG_U: i32 = 0x0800_0000;
pub const PM_DIG_L: i32 = 0x1000_0000;
pub const PM_AM: i32 = 0x2000_0000;
pub const PM_FM: i32 = 0x4000_0000;
/// `ST_ONLINE` di `RigStatusX`: la radio risponde (`OmniRigTransceiver.cpp:175`, nel binario `== 4`).
pub const ST_ONLINE: i32 = 4;

/// Il testo dell'errore a radio non in linea: l'interfaccia lo traduce cosi' com'e' (`app/src/lib/coreText.ts`, `core.omniOffline`). Il
/// testo di OmniRig (`StatusStr`, come in `OmniRigTransceiver.cpp:184`) va nel registro.
pub const OFFLINE: &str = "radio non in linea";

/// Quello che le sequenze usano di una radio di OmniRig (`RigX`): proprieta' intere, testi e metodi. Il thread COM lo implementa con
/// `IDispatch`; le prove con una radio finta.
pub(crate) trait RigApi {
    fn get(&self, name: &str) -> Result<i32, RigError>;
    fn put(&self, name: &str, value: i32) -> Result<(), RigError>;
    /// Un metodo con argomenti interi; il risultato (0 se il metodo non ne ha).
    fn call(&self, name: &str, args: &[i32]) -> Result<i32, RigError>;
    fn text(&self, name: &str) -> Result<String, RigError>;
    fn pause(&self, ms: u64) {
        std::thread::sleep(std::time::Duration::from_millis(ms));
    }
}

/// Il nome del modo per un valore `PM_*` (`map_mode` di `OmniRigTransceiver.cpp:18-55`, stesso ordine).
pub fn mode_name(param: i32) -> Option<&'static str> {
    [(PM_CW_U, "CW-R"), (PM_CW_L, "CW"), (PM_SSB_U, "USB"), (PM_SSB_L, "LSB"), (PM_DIG_U, "DIG-U"), (PM_DIG_L, "DIG-L"), (PM_AM, "AM"), (PM_FM, "FM")]
        .into_iter()
        .find(|(bit, _)| param & bit != 0)
        .map(|(_, name)| name)
}

/// La proprieta' con cui si scrive la frequenza, come `do_frequency` (`OmniRigTransceiver.cpp:722-740`): `Freq` se OmniRig la rende scrivibile,
/// altrimenti `FreqB` con la radio sul VFO B (`reversed_`) o `FreqA`. Sulla FT-2000 `WriteableParams` non ha PM_FREQ (scriverla veniva ignorato
/// senza errore): li' si scrive `FreqA`, come gia' faceva Aurora.
pub fn freq_property(writeable: i32, reversed: bool) -> Option<&'static str> {
    if writeable & PM_FREQ != 0 {
        Some("Freq")
    } else if reversed && writeable & PM_FREQB != 0 {
        Some("FreqB")
    } else if !reversed && writeable & PM_FREQA != 0 {
        Some("FreqA")
    } else {
        None
    }
}

/// La radio riceve sul VFO B (`reversed_` di WSJT-X, impostato da `handle_params_change` con PM_VFOB, PM_VFOBA o PM_VFOBB,
/// `OmniRigTransceiver.cpp:444-471`). Senza gli eventi lo si ricava leggendo `Vfo`.
pub fn is_reversed(vfo: i32) -> bool {
    vfo & (PM_VFOB | PM_VFOBA | PM_VFOBB) != 0
}

/// Il valore di `Tx` per il PTT (`do_ptt`, `OmniRigTransceiver.cpp:685`: `SetTx (on ? PM_TX : PM_RX)`).
pub fn ptt_value(on: bool) -> i32 {
    if on { PM_TX } else { PM_RX }
}

fn hz_i32(hz: u64) -> Result<i32, RigError> {
    i32::try_from(hz).map_err(|_| RigError(format!("frequenza fuori dai limiti di OmniRig: {hz} Hz")))
}

/// Errore se la radio non e' in linea; il testo di OmniRig (`StatusStr`) va nel registro, una volta per ogni cambio.
fn require_online(r: &dyn RigApi) -> Result<(), RigError> {
    if r.get("Status")? == ST_ONLINE {
        return Ok(());
    }
    let status = r.text("StatusStr").unwrap_or_default();
    static LAST: std::sync::Mutex<String> = std::sync::Mutex::new(String::new());
    if let Ok(mut last) = LAST.lock()
        && *last != status
    {
        tracing::info!("OmniRig Rig{}: radio non in linea («{status}»)", rig());
        *last = status;
    }
    Err(RigError(OFFLINE.into()))
}

/// Aspetta che la radio sia in linea, a scatti (fino a `tries` volte `ms`).
fn wait_online(r: &dyn RigApi, tries: u32, ms: u64) -> Result<(), RigError> {
    for _ in 0..tries {
        if r.get("Status")? == ST_ONLINE {
            return Ok(());
        }
        r.pause(ms);
    }
    require_online(r)
}

/// La frequenza di ricezione (`GetRxFrequency`, come `update_rx_frequency (rig_->GetRxFrequency ())` di `do_start`).
fn read_frequency(r: &dyn RigApi) -> Result<u64, RigError> {
    require_online(r)?;
    let hz = r.call("GetRxFrequency", &[])?;
    u64::try_from(hz).map_err(|_| RigError(format!("frequenza illeggibile: {hz}")))
}

/// Scrive la frequenza con la proprieta' giusta ([`freq_property`]) e aspetta che la radio la confermi (al massimo 2,4 s).
fn write_frequency(r: &dyn RigApi, hz: i32) -> Result<(), RigError> {
    let writeable = r.get("WriteableParams")?;
    let readable = r.get("ReadableParams")?;
    let reversed = readable & (PM_VFOA | PM_VFOB | PM_VFOAA | PM_VFOAB | PM_VFOBA | PM_VFOBB) != 0 && r.get("Vfo").is_ok_and(is_reversed);
    let prop = freq_property(writeable, reversed).ok_or_else(|| RigError("OmniRig: non so come impostare la frequenza di questa radio (OmniRig non rende scrivibile Freq, FreqA ne' FreqB)".into()))?;
    r.put(prop, hz)?;
    for _ in 0..24 {
        if r.get(prop).is_ok_and(|f| f == hz) {
            break;
        }
        r.pause(100);
    }
    Ok(())
}

/// Il cambio di frequenza (cambio banda). Il modo della radio (per esempio USB dati) si ricorda prima e si rimette dopo se la radio, cambiando
/// banda, ha richiamato quello che aveva memorizzato per quella banda: il modo dati di prima serve, o i segnali escono distorti. Si riscrive solo
/// se e' cambiato e se OmniRig lo permette per quella radio (`mapped & writable_params_` di `do_mode`, `OmniRigTransceiver.cpp:815`).
///
/// Senza split non si usa `SetSimplexMode` (che WSJT-X chiama in `do_tx_frequency`, `OmniRigTransceiver.cpp:783`): sulla FT-2000 mette la
/// radio sul VFO B e la lascia (o la rimette) in split, anche se subito dopo si scrive lo split spento. Provato dal vivo, in questo ordine: si
/// scrive `Split` spento (PM_SPLITOFF) solo se non e' gia' spento (di norma lo e': il percorso veloce non aspetta nulla), si scrive la frequenza
/// e si ricontrolla lo split una volta dopo un attimo. Scrivere il VFO subito dopo lo split lo fa riaccendere: non si fa.
///
/// Con lo split vero («Rig») e la frequenza audio nota, come `do_frequency` + `do_tx_frequency`: la frequenza e poi `SetSplitMode (rx, rx + XIT)`
/// (`OmniRigTransceiver.cpp:776`), senza spegnere lo split.
fn tune(r: &dyn RigApi, hz: i32, xit: Option<i64>) -> Result<(), RigError> {
    wait_online(r, 8, 400)?;
    let m0 = r.get("Mode").unwrap_or(0);
    match xit {
        Some(xit) => {
            let tx = i32::try_from((i64::from(hz) + xit).max(0)).map_err(|_| RigError("frequenza di trasmissione fuori dai limiti".into()))?;
            write_frequency(r, hz)?;
            r.call("SetSplitMode", &[hz, tx])?;
            r.pause(200);
        }
        None => {
            if r.get("Split")? != PM_SPLITOFF {
                r.put("Split", PM_SPLITOFF)?;
                for _ in 0..16 {
                    r.pause(200);
                    if r.get("Split").is_ok_and(|s| s == PM_SPLITOFF) {
                        break;
                    }
                }
            }
            write_frequency(r, hz)?;
            r.pause(300);
            if r.get("Split")? != PM_SPLITOFF {
                r.put("Split", PM_SPLITOFF)?;
            }
            r.pause(200);
        }
    }
    if m0 != 0 && r.get("Mode").is_ok_and(|m| m != m0) && r.get("WriteableParams").is_ok_and(|w| w & m0 != 0) {
        r.put("Mode", m0)?;
        for _ in 0..10 {
            r.pause(100);
            if r.get("Mode").is_ok_and(|m| m == m0) {
                break;
            }
        }
        tracing::info!("OmniRig: modo della radio rimesso a {} dopo il cambio di frequenza", mode_name(m0).unwrap_or("?"));
    }
    Ok(())
}

/// Lo split vero sulla frequenza di ricezione letta dalla radio (`do_tx_frequency`, `OmniRigTransceiver.cpp:776`).
fn split_on(r: &dyn RigApi, xit: i64) -> Result<(), RigError> {
    require_online(r)?;
    let rx = r.call("GetRxFrequency", &[])?;
    let tx = i32::try_from((i64::from(rx) + xit).max(0)).map_err(|_| RigError("frequenza di trasmissione fuori dai limiti".into()))?;
    r.call("SetSplitMode", &[rx, tx]).map(|_| ())
}

/// Il salto breve dello split simulato: solo la frequenza, senza toccare split e modo.
fn jump(r: &dyn RigApi, hz: i32) -> Result<(), RigError> {
    require_online(r)?;
    write_frequency(r, hz)
}

/// Al distacco, lo split vero acceso da Aurora si spegne (`PM_SPLITOFF`, non `SetSimplexMode`: vedi [`tune`]).
fn split_off(r: &dyn RigApi) -> Result<(), RigError> {
    if r.get("Status")? == ST_ONLINE && r.get("Split")? != PM_SPLITOFF {
        r.put("Split", PM_SPLITOFF)?;
    }
    Ok(())
}

/// Il PTT (`Tx`); acceso solo con la radio in linea.
fn ptt(r: &dyn RigApi, on: bool) -> Result<(), RigError> {
    if on {
        require_online(r)?;
    }
    r.put("Tx", ptt_value(on))
}

#[cfg(windows)]
mod workers {
    use std::time::Duration;

    use crate::omnirig_com::Worker;

    /// Il thread dei comandi (frequenza, split, letture). Il tempo massimo e' quello della vecchia sessione: un cambio di banda con le sue
    /// attese a scatti sta ben sotto.
    pub(super) static MAIN: Worker = Worker::new("omnirig", Duration::from_secs(20));
    /// Il thread del PTT, a parte: un PTT non aspetta mai un cambio di banda in corso.
    pub(super) static PTT: Worker = Worker::new("omnirig-ptt", Duration::from_secs(5));
}

/// Esegue una sequenza sulla radio scelta, nel thread COM indicato.
#[cfg(windows)]
fn on_rig<T: Send + 'static>(worker: &'static crate::omnirig_com::Worker, f: impl FnOnce(&dyn RigApi) -> Result<T, RigError> + Send + 'static) -> Result<T, RigError> {
    let n = rig();
    worker.run(move |com| {
        let r = com.rig(n)?;
        f(&r)
    })
}

#[cfg(not(windows))]
mod workers {
    /// Fuori da Windows OmniRig non esiste: non c'e' nessun thread.
    pub(super) struct Worker;
    impl Worker {
        pub(super) fn is_running(&self) -> bool {
            false
        }
        pub(super) fn stop(&self) {}
    }
    pub(super) static MAIN: Worker = Worker;
    pub(super) static PTT: Worker = Worker;
}

#[cfg(not(windows))]
fn on_rig<T>(_worker: &'static workers::Worker, _f: impl FnOnce(&dyn RigApi) -> Result<T, RigError>) -> Result<T, RigError> {
    Err(RigError("OmniRig esiste solo su Windows".into()))
}

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
    let xit = crate::cat_config::split_xit_hz(hz);
    match split_code() {
        1 => {
            on_rig(&workers::MAIN, move |r| split_on(r, xit))?;
            SPLIT_BY_US.store(true, std::sync::atomic::Ordering::SeqCst);
            Ok(())
        }
        2 => {
            let restore = EMULATE_RESTORE.load(std::sync::atomic::Ordering::SeqCst);
            if restore == 0 {
                // non si sta trasmettendo con la radio spostata: si applica al prossimo PTT
                return Ok(());
            }
            let tx = hz_i32(crate::cat_config::split_tx_dial_hz(restore, hz))?;
            on_rig(&workers::MAIN, move |r| jump(r, tx))
        }
        _ => Ok(()),
    }
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
    let tx = hz_i32(tx)?;
    on_rig(&workers::MAIN, move |r| jump(r, tx))?;
    EMULATE_RESTORE.store(dial, std::sync::atomic::Ordering::SeqCst);
    Ok(())
}

/// Split simulato, dopo il PTT: la radio torna sulla frequenza di ricezione.
pub fn emulate_end() -> Result<(), RigError> {
    let back = EMULATE_RESTORE.swap(0, std::sync::atomic::Ordering::SeqCst);
    if back == 0 {
        return Ok(());
    }
    let back = hz_i32(back)?;
    on_rig(&workers::MAIN, move |r| jump(r, back))
}

/// Toglie quello che Aurora ha cambiato nello split: con lo split vero acceso da Aurora lo si spegne (`PM_SPLITOFF`), con quello simulato la
/// radio torna sulla ricezione. L'oggetto OmniRig resta aperto.
pub fn reset_split() {
    if let Err(e) = emulate_end() {
        tracing::warn!("OmniRig: ritorno alla frequenza di ricezione non riuscito: {e}");
    }
    if SPLIT_BY_US.swap(false, std::sync::atomic::Ordering::SeqCst)
        && let Err(e) = on_rig(&workers::MAIN, split_off)
    {
        tracing::warn!("OmniRig: split non spento al distacco: {e}");
    }
}

/// La radio via OmniRig (Rig1 o Rig2, vedi [`set_rig`]): solo frequenza. OmniRig e' un driver CAT facoltativo che l'utente sceglie in Impostazioni › Radio;
/// Aurora non comanda nessun altro programma.
pub struct WindowsRig;

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
            let found = registered();
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

/// La chiave `HKCR\OmniRig.OmniRigX` c'e' (letta con le API del registro: prima si lanciava `reg.exe`, un processo in piu' che i criteri
/// possono bloccare).
#[cfg(windows)]
fn registered() -> bool {
    use windows::Win32::System::Registry::{HKEY, HKEY_CLASSES_ROOT, KEY_READ, RegCloseKey, RegOpenKeyExW};
    let mut key = HKEY::default();
    let opened = unsafe { RegOpenKeyExW(HKEY_CLASSES_ROOT, windows::core::w!("OmniRig.OmniRigX"), Some(0), KEY_READ, &mut key) };
    if opened.is_ok() {
        let _ = unsafe { RegCloseKey(key) };
        true
    } else {
        false
    }
}

/// Un errore di OmniRig detto in chiaro: «classe non registrata» vuol dire che OmniRig non c'e', «non risponde» che OmniRig e' bloccato.
pub fn explain_omnirig(why: &str) -> String {
    if why.contains("REGDB_E_CLASSNOTREG") || why.contains("80040154") || why.contains("CLSID") {
        format!("OmniRig non e' installato su questo PC: in Impostazioni > Radio scegli Hamlib (gia' incluso in Aurora) oppure installa OmniRig da {OMNIRIG_URL}")
    } else if why == "non risponde" {
        "OmniRig non risponde: apri OmniRig e controlla che Rig1 abbia la porta e il modello giusti e la radio accesa (stato «On-line»), oppure in Impostazioni > Radio scegli Hamlib (gia' incluso in Aurora)".to_string()
    } else {
        why.to_string()
    }
}

/// Il PTT con OmniRig (`Rig<n>.Tx` = PM_TX / PM_RX), sul thread del PTT.
pub fn set_ptt(on: bool) -> Result<(), RigError> {
    on_rig(&workers::PTT, move |r| ptt(r, on))?;
    PTT_ON.store(on, std::sync::atomic::Ordering::SeqCst);
    Ok(())
}

/// Apre in anticipo l'oggetto OmniRig del PTT, cosi' il primo PTT non paga la creazione (e l'attesa della prima lettura di OmniRig).
pub fn warm_up_ptt() -> Result<(), RigError> {
    on_rig(&workers::PTT, |r| r.get("Status").map(|_| ()))
}

/// Lascia OmniRig (al distacco del CAT o al cambio di sistema di controllo): si toglie lo split messo da Aurora, si spegne il PTT se era rimasto
/// acceso e si chiudono i thread COM con i loro oggetti, cosi' OmniRig puo' chiudersi e liberare la porta della radio per un altro programma o
/// per Hamlib. Se OmniRig non era mai stato aperto non fa nulla (non lo apre per chiuderlo).
pub fn release() {
    reset_split();
    if PTT_ON.load(std::sync::atomic::Ordering::SeqCst)
        && workers::PTT.is_running()
        && let Err(e) = set_ptt(false)
    {
        tracing::warn!("OmniRig: PTT non spento al distacco: {e}");
    }
    workers::MAIN.stop();
    workers::PTT.stop();
}

/// Con OmniRig Aurora comanda solo la frequenza della radio: modo, audio e trasmissione sono del motore di Aurora.
const NOT_HERE: &str = "con OmniRig Aurora comanda solo la frequenza della radio";

impl Rig for WindowsRig {
    fn frequency(&self) -> Result<u64, RigError> {
        on_rig(&workers::MAIN, read_frequency)
    }

    fn set_frequency(&self, hz: u64) -> Result<(), RigError> {
        let hz = hz_i32(hz)?;
        let attempt = || {
            let xit = rig_split_xit();
            on_rig(&workers::MAIN, move |r| tune(r, hz, xit))
        };
        // OmniRig a volte e' occupato un istante (un altro programma che legge la radio): un secondo tentativo basta
        attempt().or_else(|first| {
            tracing::warn!("cambio frequenza, primo tentativo fallito: {first}");
            std::thread::sleep(std::time::Duration::from_millis(600));
            // terzo tentativo dopo una pausa piu' lunga: OmniRig a volte si riprende dopo un paio di secondi (radio che si risveglia)
            attempt().or_else(|second| {
                tracing::warn!("cambio frequenza, secondo tentativo fallito: {second}");
                std::thread::sleep(std::time::Duration::from_millis(2000));
                attempt().inspect_err(|e| aurora_core::notice::notice("omnirig", e.0.clone()))
            })
        })?;
        // con lo split vero la sequenza l'ha acceso (`SetSplitMode`): al distacco va spento
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

#[cfg(test)]
mod tests {
    use std::{cell::RefCell, collections::HashMap};

    use super::*;

    /// Una radio di OmniRig finta: le proprieta' scritte si rileggono subito, i comandi si annotano in ordine.
    struct FakeRig {
        values: RefCell<HashMap<String, i32>>,
        log: RefCell<Vec<String>>,
        /// Dopo una scrittura di `Freq*` la radio richiama questo modo (cambio di banda con il modo memorizzato per la banda).
        mode_after_band: Option<i32>,
    }

    impl FakeRig {
        fn new(pairs: &[(&str, i32)]) -> FakeRig {
            FakeRig { values: RefCell::new(pairs.iter().map(|(k, v)| (k.to_string(), *v)).collect()), log: RefCell::new(Vec::new()), mode_after_band: None }
        }

        fn log(&self) -> Vec<String> {
            self.log.borrow().clone()
        }

        fn index(&self, entry: &str) -> usize {
            self.log().iter().position(|l| l == entry).unwrap_or_else(|| panic!("manca «{entry}» in {:?}", self.log()))
        }
    }

    impl RigApi for FakeRig {
        fn get(&self, name: &str) -> Result<i32, RigError> {
            self.values.borrow().get(name).copied().ok_or_else(|| RigError(format!("{name} sconosciuto")))
        }

        fn put(&self, name: &str, value: i32) -> Result<(), RigError> {
            self.log.borrow_mut().push(format!("{name}={value}"));
            self.values.borrow_mut().insert(name.to_string(), value);
            if name.starts_with("Freq")
                && let Some(m) = self.mode_after_band
            {
                self.values.borrow_mut().insert("Mode".into(), m);
            }
            Ok(())
        }

        fn call(&self, name: &str, args: &[i32]) -> Result<i32, RigError> {
            self.log.borrow_mut().push(format!("{name}{args:?}"));
            match name {
                "GetRxFrequency" => self.get("FreqA"),
                _ => Ok(0),
            }
        }

        fn text(&self, name: &str) -> Result<String, RigError> {
            Ok(if name == "StatusStr" { "Rig is not responding".into() } else { String::new() })
        }

        fn pause(&self, _ms: u64) {}
    }

    /// La FT-2000 di prima: `Freq` non scrivibile, FreqA si'; in linea, split spento, USB dati.
    fn ft2000() -> FakeRig {
        FakeRig::new(&[
            ("Status", ST_ONLINE),
            ("WriteableParams", PM_FREQA | PM_FREQB | PM_SPLITOFF | PM_SPLITON | PM_DIG_U | PM_SSB_U),
            ("ReadableParams", PM_FREQA | PM_FREQB | PM_VFOAA | PM_VFOAB),
            ("Vfo", PM_VFOAA),
            ("Split", PM_SPLITOFF),
            ("Mode", PM_DIG_U),
            ("FreqA", 14_074_000),
        ])
    }

    #[test]
    fn the_frequency_property_follows_wsjtx_do_frequency() {
        assert_eq!(freq_property(PM_FREQ | PM_FREQA | PM_FREQB, false), Some("Freq"));
        assert_eq!(freq_property(PM_FREQ | PM_FREQA | PM_FREQB, true), Some("Freq"));
        assert_eq!(freq_property(PM_FREQA | PM_FREQB, false), Some("FreqA"), "FT-2000: senza PM_FREQ si scrive FreqA");
        assert_eq!(freq_property(PM_FREQA | PM_FREQB, true), Some("FreqB"));
        assert_eq!(freq_property(PM_FREQA, true), None, "sul VFO B senza FreqB scrivibile non si sa come fare");
        assert_eq!(freq_property(0, false), None);
    }

    #[test]
    fn the_vfo_b_means_reversed() {
        assert!(!is_reversed(PM_VFOA) && !is_reversed(PM_VFOAA) && !is_reversed(PM_VFOAB) && !is_reversed(PM_UNKNOWN));
        assert!(is_reversed(PM_VFOB) && is_reversed(PM_VFOBA) && is_reversed(PM_VFOBB));
    }

    #[test]
    fn modes_map_like_wsjtx() {
        assert_eq!(mode_name(PM_DIG_U), Some("DIG-U"));
        assert_eq!(mode_name(PM_SSB_U), Some("USB"));
        assert_eq!(mode_name(PM_CW_U | PM_SSB_U), Some("CW-R"), "stesso ordine di map_mode");
        assert_eq!(mode_name(PM_FM), Some("FM"));
        assert_eq!(mode_name(0), None);
    }

    #[test]
    fn the_ptt_writes_tx_and_rx_like_wsjtx() {
        assert_eq!(ptt_value(true), 0x0040_0000);
        assert_eq!(ptt_value(false), 0x0020_0000);
        let r = ft2000();
        ptt(&r, true).expect("ptt");
        ptt(&r, false).expect("ptt");
        assert_eq!(r.log(), ["Tx=4194304", "Tx=2097152"]);
    }

    #[test]
    fn the_ptt_does_not_key_an_offline_radio_but_always_releases() {
        let r = ft2000();
        r.values.borrow_mut().insert("Status".into(), 3);
        assert_eq!(ptt(&r, true).unwrap_err().0, OFFLINE);
        ptt(&r, false).expect("lo spegnimento si manda sempre");
        assert_eq!(r.log(), ["Tx=2097152"]);
    }

    #[test]
    fn a_band_change_turns_split_off_first_writes_freqa_and_never_uses_simplex_mode() {
        let r = ft2000();
        r.values.borrow_mut().insert("Split".into(), PM_SPLITON);
        tune(&r, 7_074_000, None).expect("cambio");
        let log = r.log();
        assert!(r.index("Split=65536") < r.index("FreqA=7074000"), "prima lo split spento, poi la frequenza: {log:?}");
        assert!(!log.iter().any(|l| l.starts_with("SetSimplexMode")), "SetSimplexMode rimette la radio in split: {log:?}");
        assert!(!log.iter().any(|l| l.starts_with("Freq=")), "Freq non e' scrivibile sulla FT-2000: {log:?}");
        assert!(!log.iter().any(|l| l.starts_with("Vfo")), "scrivere il VFO dopo lo split lo fa riaccendere: {log:?}");
        assert_eq!(r.get("FreqA"), Ok(7_074_000));
    }

    #[test]
    fn with_split_already_off_the_band_change_only_writes_the_frequency() {
        let r = ft2000();
        tune(&r, 7_074_000, None).expect("cambio");
        assert_eq!(r.log(), ["FreqA=7074000"]);
    }

    #[test]
    fn a_rig_with_writable_freq_gets_freq_like_wsjtx() {
        let r = ft2000();
        r.values.borrow_mut().insert("WriteableParams".into(), PM_FREQ | PM_FREQA | PM_FREQB);
        tune(&r, 21_074_000, None).expect("cambio");
        assert_eq!(r.log(), ["Freq=21074000"]);
    }

    #[test]
    fn a_radio_on_vfo_b_gets_freqb() {
        let r = ft2000();
        r.values.borrow_mut().insert("Vfo".into(), PM_VFOBB);
        tune(&r, 21_074_000, None).expect("cambio");
        assert_eq!(r.log(), ["FreqB=21074000"]);
    }

    #[test]
    fn the_radio_mode_is_restored_only_when_it_changed_and_is_writable() {
        let mut r = ft2000();
        r.mode_after_band = Some(PM_SSB_U);
        tune(&r, 7_074_000, None).expect("cambio");
        assert!(r.index("FreqA=7074000") < r.index(&format!("Mode={PM_DIG_U}")), "{:?}", r.log());

        let mut r = ft2000();
        r.values.borrow_mut().insert("WriteableParams".into(), PM_FREQA | PM_SSB_U);
        r.mode_after_band = Some(PM_SSB_U);
        tune(&r, 7_074_000, None).expect("cambio");
        assert!(!r.log().iter().any(|l| l.starts_with("Mode=")), "modo non scrivibile: {:?}", r.log());

        let r = ft2000();
        r.values.borrow_mut().insert("Mode".into(), 0);
        tune(&r, 7_074_000, None).expect("cambio");
        assert!(!r.log().iter().any(|l| l.starts_with("Mode=")), "un modo sconosciuto non si riscrive: {:?}", r.log());
    }

    #[test]
    fn the_rig_split_sets_split_mode_on_rx_plus_xit_without_turning_split_off() {
        let r = ft2000();
        tune(&r, 14_074_000, Some(1500)).expect("cambio");
        assert_eq!(r.log(), ["FreqA=14074000", "SetSplitMode[14074000, 14075500]"]);
        let r = ft2000();
        split_on(&r, -500).expect("split");
        assert_eq!(r.log(), ["GetRxFrequency[]", "SetSplitMode[14074000, 14073500]"]);
    }

    #[test]
    fn an_offline_radio_is_reported_with_the_text_the_interface_translates() {
        let r = ft2000();
        r.values.borrow_mut().insert("Status".into(), 2);
        assert_eq!(read_frequency(&r).unwrap_err().0, "radio non in linea");
        assert_eq!(tune(&r, 7_074_000, None).unwrap_err().0, "radio non in linea");
        assert!(r.log().is_empty(), "a radio non in linea non si scrive nulla: {:?}", r.log());
    }

    #[test]
    fn the_split_is_turned_off_on_release_only_when_on() {
        let r = ft2000();
        split_off(&r).expect("niente da fare");
        assert!(r.log().is_empty());
        r.values.borrow_mut().insert("Split".into(), PM_SPLITON);
        split_off(&r).expect("spento");
        assert_eq!(r.log(), ["Split=65536"]);
    }

    #[test]
    fn frequencies_beyond_omnirig_integers_are_refused() {
        assert!(hz_i32(2_400_000_000).is_err());
        assert_eq!(hz_i32(144_174_000), Ok(144_174_000));
    }

    #[test]
    fn errors_are_explained() {
        assert!(explain_omnirig("80040154").starts_with("OmniRig non e' installato"));
        assert!(explain_omnirig("non risponde").starts_with("OmniRig non risponde"));
        assert_eq!(explain_omnirig("altro"), "altro");
    }

    /// Con OmniRig installato e la radio accesa: `cargo test -p rig-control --lib omnirig -- --ignored --nocapture`.
    #[cfg(windows)]
    #[test]
    #[ignore = "richiede OmniRig installato con una radio in linea"]
    fn live_reads_the_frequency_through_com() {
        let hz = WindowsRig.frequency().expect("frequenza");
        println!("frequenza letta da OmniRig: {hz} Hz");
        release();
    }
}
