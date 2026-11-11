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
/// Split DX acceso da Aurora (doppio clic su uno spot del cluster con «UP 5», «QSX ...»): finche' resta acceso lo split di FT8 non tocca il
/// VFO di trasmissione.
static DX_SPLIT: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
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

/// OmniRig non ha risposto in tempo mentre si avviava o con la radio non in linea (o una volta sola): un avviso per l'operatore, non un errore
/// da segnalare (`core.omniNotReady` nell'interfaccia; `aurora::rapporti` lo riconosce da «in linea in OmniRig?»).
pub const NOT_READY: &str = "OmniRig non risponde: la radio e' accesa e in linea in OmniRig?";

/// L'ultimo `Status` letto da OmniRig (-1 = mai letto): dice se un tempo scaduto riguarda una radio non in linea.
static LAST_STATUS: std::sync::atomic::AtomicI32 = std::sync::atomic::AtomicI32::new(-1);

/// Annota l'ultimo `Status` letto (lo fa il cliente COM a ogni lettura).
#[cfg_attr(not(windows), allow(dead_code))]
pub(crate) fn note_status(status: i32) {
    LAST_STATUS.store(status, std::sync::atomic::Ordering::SeqCst);
}

/// L'ultimo `Status` letto (-1 = mai letto).
#[cfg_attr(not(windows), allow(dead_code))]
pub(crate) fn last_status() -> i32 {
    LAST_STATUS.load(std::sync::atomic::Ordering::SeqCst)
}

/// Perche' OmniRig non ha risposto in tempo.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(not(windows), allow(dead_code))]
pub(crate) enum TimeoutCause {
    /// L'oggetto si stava ancora creando (OmniRig.exe che si avvia, la sua finestra, la porta della radio).
    Starting,
    /// L'ultimo `Status` letto non era `ST_ONLINE`: OmniRig aspetta una radio spenta o scollegata.
    Offline,
    /// Radio in linea, prima volta: puo' essere un attimo di OmniRig occupato.
    OnlineOnce,
    /// Radio in linea e di nuovo nessuna risposta: OmniRig e' bloccato (questo si segnala).
    OnlineRepeated,
}

/// Il verdetto su un tempo scaduto: la causa e il testo per chi chiama. Solo i tempi scaduti ripetuti con la radio in linea hanno il testo
/// lungo di [`explain_omnirig`] (che finisce nei rapporti); gli altri hanno [`NOT_READY`], un avviso.
#[cfg_attr(not(windows), allow(dead_code))]
pub(crate) fn timeout_verdict(starting: bool, status: i32, repeated: u32) -> (TimeoutCause, String) {
    let cause = if starting {
        TimeoutCause::Starting
    } else if status != ST_ONLINE {
        TimeoutCause::Offline
    } else if repeated < 2 {
        TimeoutCause::OnlineOnce
    } else {
        TimeoutCause::OnlineRepeated
    };
    let text = if cause == TimeoutCause::OnlineRepeated { explain_omnirig("non risponde") } else { NOT_READY.to_string() };
    (cause, text)
}

/// L'errore e' un tempo scaduto di OmniRig (riprovare subito non serve: si aspetterebbe di nuovo tutto il tempo).
pub fn is_timeout(message: &str) -> bool {
    message == NOT_READY || message == explain_omnirig("non risponde")
}

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

/// Lo Split DX: la ricezione come un cambio di frequenza senza split (split spento, frequenza, modo rimesso: vedi [`tune`]), poi
/// `SetSplitMode (rx, tx)` come `do_tx_frequency` di WSJT-X (`OmniRigTransceiver.cpp:776`), con la trasmissione scelta dallo spot.
fn dx_split_sequence(r: &dyn RigApi, rx: i32, tx: i32) -> Result<(), RigError> {
    tune(r, rx, None)?;
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

/// Il PTT acceso: in attesa, acceso in tempo, chi lo chiedeva ha smesso di aspettare.
const PTT_PENDING: u8 = 0;
const PTT_KEYED: u8 = 1;
const PTT_GAVE_UP: u8 = 2;

/// Il testo di un PTT arrivato a OmniRig dopo che il tempo era scaduto (e subito spento).
const PTT_LATE: &str = "OmniRig: PTT arrivato in ritardo, spento";

/// Accende il PTT solo se chi lo chiede sta ancora aspettando: un `Tx = PM_TX` rimasto bloccato dentro OmniRig e arrivato dopo il tempo
/// massimo metterebbe la radio in trasmissione senza audio e senza nessuno che la spenga. Se il tempo scade mentre il comando e' in corso, la
/// radio torna subito in ricezione.
fn ptt_on_unless_late(r: &dyn RigApi, state: &std::sync::atomic::AtomicU8) -> Result<(), RigError> {
    use std::sync::atomic::Ordering;
    if state.load(Ordering::SeqCst) == PTT_GAVE_UP {
        return Err(RigError(PTT_LATE.into()));
    }
    ptt(r, true)?;
    if state.compare_exchange(PTT_PENDING, PTT_KEYED, Ordering::SeqCst, Ordering::SeqCst).is_err() {
        tracing::info!("OmniRig: PTT acceso dopo il tempo massimo, la radio torna subito in ricezione");
        r.put("Tx", ptt_value(false))?;
        return Err(RigError(PTT_LATE.into()));
    }
    Ok(())
}

#[cfg(windows)]
mod workers {
    use std::time::Duration;

    use crate::omnirig_com::Worker;

    /// Il thread dei comandi (frequenza, split, letture). Il tempo massimo e' quello della vecchia sessione: un cambio di banda con le sue
    /// attese a scatti sta ben sotto. La prima creazione dell'oggetto, che puo' avviare OmniRig.exe, ha 30 s.
    pub(super) static MAIN: Worker = Worker::new("omnirig", Duration::from_secs(20), Duration::from_secs(30));
    /// Il thread del PTT, a parte: un PTT non aspetta mai un cambio di banda in corso. Anche alla prima creazione il PTT non aspetta piu' di
    /// 5 s (la trasmissione ha i suoi tempi): l'oggetto del PTT si prepara al collegamento ([`super::warm_up_ptt_in_background`]).
    pub(super) static PTT: Worker = Worker::new("omnirig-ptt", Duration::from_secs(5), Duration::from_secs(5));
}

/// Il tempo concesso alla preparazione dell'oggetto del PTT (puo' dover aspettare l'avvio di OmniRig).
#[cfg_attr(not(windows), allow(dead_code))]
const WARM_UP: std::time::Duration = std::time::Duration::from_secs(30);

/// Esegue una sequenza sulla radio scelta, nel thread COM indicato. Se chi chiede smette di aspettare prima che la sequenza parta, non parte
/// piu' (un cambio di frequenza vecchio non arriva alla radio minuti dopo).
#[cfg(windows)]
fn on_rig<T: Send + 'static>(worker: &'static crate::omnirig_com::Worker, f: impl FnOnce(&dyn RigApi) -> Result<T, RigError> + Send + 'static) -> Result<T, RigError> {
    on_rig_with(worker, None, true, f)
}

/// Come [`on_rig`], ma la sequenza parte anche in ritardo: spegnimento del PTT, ritorno alla ricezione, split spento al distacco.
#[cfg(windows)]
fn on_rig_always<T: Send + 'static>(worker: &'static crate::omnirig_com::Worker, f: impl FnOnce(&dyn RigApi) -> Result<T, RigError> + Send + 'static) -> Result<T, RigError> {
    on_rig_with(worker, None, false, f)
}

#[cfg(windows)]
fn on_rig_with<T: Send + 'static>(
    worker: &'static crate::omnirig_com::Worker,
    allowance: Option<std::time::Duration>,
    skip_if_late: bool,
    f: impl FnOnce(&dyn RigApi) -> Result<T, RigError> + Send + 'static,
) -> Result<T, RigError> {
    let n = rig();
    worker.run_with(crate::omnirig_com::Wait { allowance, skip_if_late }, move |com| {
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

#[cfg(not(windows))]
fn on_rig_always<T>(worker: &'static workers::Worker, f: impl FnOnce(&dyn RigApi) -> Result<T, RigError>) -> Result<T, RigError> {
    on_rig(worker, f)
}

#[cfg(not(windows))]
fn on_rig_with<T>(worker: &'static workers::Worker, _allowance: Option<std::time::Duration>, _skip_if_late: bool, f: impl FnOnce(&dyn RigApi) -> Result<T, RigError>) -> Result<T, RigError> {
    on_rig(worker, f)
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
    if DX_SPLIT.load(std::sync::atomic::Ordering::SeqCst) {
        return None;
    }
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
    // con lo Split DX il VFO di trasmissione e' quello dello spot
    if DX_SPLIT.load(std::sync::atomic::Ordering::SeqCst) {
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
    if split_code() != 2 || audio == 0 || DX_SPLIT.load(std::sync::atomic::Ordering::SeqCst) || EMULATE_RESTORE.load(std::sync::atomic::Ordering::SeqCst) != 0 {
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
    on_rig_always(&workers::MAIN, move |r| jump(r, back))
}

/// Toglie quello che Aurora ha cambiato nello split: con lo split vero acceso da Aurora lo si spegne (`PM_SPLITOFF`), con quello simulato la
/// radio torna sulla ricezione. L'oggetto OmniRig resta aperto.
pub fn reset_split() {
    if let Err(e) = emulate_end() {
        tracing::warn!("OmniRig: ritorno alla frequenza di ricezione non riuscito: {e}");
    }
    DX_SPLIT.store(false, std::sync::atomic::Ordering::SeqCst);
    if SPLIT_BY_US.swap(false, std::sync::atomic::Ordering::SeqCst)
        && let Err(e) = on_rig_always(&workers::MAIN, split_off)
    {
        tracing::warn!("OmniRig: split non spento al distacco: {e}");
    }
}

/// Split DX (doppio clic su uno spot del cluster): ricezione su `rx_hz`, trasmissione su `tx_hz` con `SetSplitMode`.
pub fn set_dx_split(rx_hz: u64, tx_hz: u64) -> Result<(), RigError> {
    let (rx, tx) = (hz_i32(rx_hz)?, hz_i32(tx_hz)?);
    DX_SPLIT.store(true, std::sync::atomic::Ordering::SeqCst);
    match on_rig(&workers::MAIN, move |r| dx_split_sequence(r, rx, tx)) {
        Ok(()) => {
            SPLIT_BY_US.store(true, std::sync::atomic::Ordering::SeqCst);
            Ok(())
        }
        Err(e) => {
            DX_SPLIT.store(false, std::sync::atomic::Ordering::SeqCst);
            Err(e)
        }
    }
}

/// Spegne lo Split DX, se acceso (`PM_SPLITOFF`, non `SetSimplexMode`: vedi [`tune`]).
pub fn clear_dx_split() -> Result<(), RigError> {
    if !DX_SPLIT.load(std::sync::atomic::Ordering::SeqCst) {
        return Ok(());
    }
    on_rig(&workers::MAIN, split_off)?;
    DX_SPLIT.store(false, std::sync::atomic::Ordering::SeqCst);
    Ok(())
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

/// Il PTT con OmniRig (`Rig<n>.Tx` = PM_TX / PM_RX), sul thread del PTT. Lo spegnimento parte sempre, anche in ritardo; l'accensione solo se
/// chi la chiede sta ancora aspettando ([`ptt_on_unless_late`]): un PTT scaduto non lascia mai la radio in trasmissione.
pub fn set_ptt(on: bool) -> Result<(), RigError> {
    use std::sync::{Arc, atomic::{AtomicU8, Ordering}};
    if !on {
        on_rig_always(&workers::PTT, |r| ptt(r, false))?;
        PTT_ON.store(false, Ordering::SeqCst);
        return Ok(());
    }
    let state = Arc::new(AtomicU8::new(PTT_PENDING));
    let job_state = Arc::clone(&state);
    let result = on_rig(&workers::PTT, move |r| ptt_on_unless_late(r, &job_state));
    if result.is_err() && state.swap(PTT_GAVE_UP, Ordering::SeqCst) == PTT_KEYED {
        // acceso proprio mentre scadeva il tempo: per Aurora e' acceso, cosi' lo spegnimento che segue (PttGuard, distacco) lo spegne
        PTT_ON.store(true, Ordering::SeqCst);
    }
    result?;
    PTT_ON.store(true, Ordering::SeqCst);
    Ok(())
}

/// Apre in anticipo l'oggetto OmniRig del PTT, cosi' il primo PTT non paga la creazione (e l'attesa della prima lettura di OmniRig). Ha il
/// tempo lungo della prima creazione: OmniRig potrebbe essere ancora in avvio.
pub fn warm_up_ptt() -> Result<(), RigError> {
    on_rig_with(&workers::PTT, Some(WARM_UP), true, |r| r.get("Status").map(|_| ()))
}

/// [`warm_up_ptt`] in un thread a parte (al collegamento di OmniRig): chi collega non aspetta. Gli errori vanno solo nel registro.
pub fn warm_up_ptt_in_background() {
    let spawned = std::thread::Builder::new().name("aurora-omnirig-ptt-warm-up".into()).spawn(|| {
        if let Err(e) = warm_up_ptt() {
            tracing::info!("OmniRig: oggetto del PTT non preparato in anticipo: {e}");
        }
    });
    if let Err(e) = spawned {
        tracing::info!("OmniRig: preparazione del PTT non avviata: {e}");
    }
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
        // OmniRig a volte e' occupato un istante (un altro programma che legge la radio): un secondo tentativo basta. Dopo un tempo scaduto
        // (OmniRig che si avvia o bloccato) non si riprova: si aspetterebbe di nuovo tutto il tempo, due volte
        let give_up = |e: &RigError| {
            if is_timeout(&e.0) {
                aurora_core::notice::notice("omnirig", e.0.clone());
                true
            } else {
                false
            }
        };
        attempt().or_else(|first| {
            if give_up(&first) {
                return Err(first);
            }
            tracing::info!("cambio frequenza, primo tentativo fallito: {first}");
            std::thread::sleep(std::time::Duration::from_millis(600));
            // terzo tentativo dopo una pausa piu' lunga: OmniRig a volte si riprende dopo un paio di secondi (radio che si risveglia)
            attempt().or_else(|second| {
                if give_up(&second) {
                    return Err(second);
                }
                tracing::info!("cambio frequenza, secondo tentativo fallito: {second}");
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
    fn the_dx_split_turns_split_off_tunes_the_rx_and_sets_split_mode_last() {
        let r = ft2000();
        r.values.borrow_mut().insert("Split".into(), PM_SPLITON);
        dx_split_sequence(&r, 14_025_000, 14_030_000).expect("split DX");
        let log = r.log();
        assert!(r.index("Split=65536") < r.index("FreqA=14025000"), "prima lo split di prima spento, poi la ricezione: {log:?}");
        assert_eq!(log.last().map(String::as_str), Some("SetSplitMode[14025000, 14030000]"), "lo split per ultimo: {log:?}");
        assert!(!log.iter().any(|l| l.starts_with("SetSimplexMode")), "{log:?}");
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
    fn a_timeout_is_a_notice_unless_it_repeats_with_the_radio_online() {
        assert_eq!(timeout_verdict(true, ST_ONLINE, 0), (TimeoutCause::Starting, NOT_READY.to_string()), "OmniRig che si avvia");
        assert_eq!(timeout_verdict(true, -1, 0).0, TimeoutCause::Starting);
        assert_eq!(timeout_verdict(false, 2, 0), (TimeoutCause::Offline, NOT_READY.to_string()), "radio non in linea");
        assert_eq!(timeout_verdict(false, -1, 0).0, TimeoutCause::Offline, "stato mai letto: non e' una radio in linea");
        assert_eq!(timeout_verdict(false, ST_ONLINE, 1), (TimeoutCause::OnlineOnce, NOT_READY.to_string()));
        let (cause, text) = timeout_verdict(false, ST_ONLINE, 2);
        assert_eq!(cause, TimeoutCause::OnlineRepeated);
        assert!(text.starts_with("OmniRig non risponde: apri OmniRig"), "{text}");
        assert!(is_timeout(&text) && is_timeout(NOT_READY) && !is_timeout(OFFLINE));
    }

    #[test]
    fn the_last_status_is_remembered() {
        note_status(3);
        assert_eq!(last_status(), 3);
        note_status(ST_ONLINE);
        assert_eq!(last_status(), ST_ONLINE);
    }

    #[test]
    fn a_ptt_that_arrives_late_never_leaves_the_radio_keyed() {
        use std::sync::atomic::{AtomicU8, Ordering};
        // in tempo: acceso
        let r = ft2000();
        let state = AtomicU8::new(PTT_PENDING);
        ptt_on_unless_late(&r, &state).expect("ptt");
        assert_eq!(state.load(Ordering::SeqCst), PTT_KEYED);
        assert_eq!(r.log(), ["Tx=4194304"]);
        // chi lo chiedeva ha gia' rinunciato prima che partisse: la radio non si tocca
        let r = ft2000();
        let state = AtomicU8::new(PTT_GAVE_UP);
        assert_eq!(ptt_on_unless_late(&r, &state).unwrap_err().0, PTT_LATE);
        assert!(r.log().is_empty(), "{:?}", r.log());
    }

    /// Una radio in cui il tempo scade mentre il PTT e' dentro OmniRig: chi aspettava rinuncia durante `put("Tx")`.
    struct SlowPtt<'a> {
        rig: FakeRig,
        state: &'a std::sync::atomic::AtomicU8,
    }

    impl RigApi for SlowPtt<'_> {
        fn get(&self, name: &str) -> Result<i32, RigError> {
            self.rig.get(name)
        }
        fn put(&self, name: &str, value: i32) -> Result<(), RigError> {
            if value == PM_TX {
                self.state.store(PTT_GAVE_UP, std::sync::atomic::Ordering::SeqCst);
            }
            self.rig.put(name, value)
        }
        fn call(&self, name: &str, args: &[i32]) -> Result<i32, RigError> {
            self.rig.call(name, args)
        }
        fn text(&self, name: &str) -> Result<String, RigError> {
            self.rig.text(name)
        }
    }

    #[test]
    fn a_ptt_keyed_while_the_wait_expires_is_released_at_once() {
        let state = std::sync::atomic::AtomicU8::new(PTT_PENDING);
        let r = SlowPtt { rig: ft2000(), state: &state };
        assert_eq!(ptt_on_unless_late(&r, &state).unwrap_err().0, PTT_LATE);
        assert_eq!(r.rig.log(), ["Tx=4194304", "Tx=2097152"], "acceso in ritardo e subito spento");
    }

    #[test]
    fn an_offline_radio_is_never_keyed_and_nothing_waits_on_it() {
        let state = std::sync::atomic::AtomicU8::new(PTT_PENDING);
        let r = ft2000();
        r.values.borrow_mut().insert("Status".into(), 1);
        assert_eq!(ptt_on_unless_late(&r, &state).unwrap_err().0, OFFLINE);
        assert!(r.log().is_empty());
        assert_eq!(state.load(std::sync::atomic::Ordering::SeqCst), PTT_PENDING);
        // le sequenze controllano Status per primo: con la radio non in linea non si scrive ne' si legge altro
        assert_eq!(split_on(&r, 500).unwrap_err().0, OFFLINE);
        assert_eq!(jump(&r, 7_074_000).unwrap_err().0, OFFLINE);
        assert!(r.log().is_empty(), "{:?}", r.log());
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
