//! Il cliente COM di OmniRig, senza PowerShell: l'oggetto `OmniRig.OmniRigX` si crea con `CoCreateInstance` e si comanda per nome
//! (`IDispatch`, `GetIDsOfNames` + `Invoke`), come fa WSJT-X con l'oggetto generato da `dumpcpp` (`OmniRigTransceiver.cpp:130`, `do_start`).
//! Prima ogni comando passava da una sessione di PowerShell: dove l'antivirus o i criteri di Windows bloccano PowerShell («accesso negato»)
//! OmniRig non rispondeva mai, nemmeno al PTT.
//!
//! Gli oggetti COM appartengono all'appartamento (thread) che li ha creati: tutte le chiamate a OmniRig girano su un thread dedicato (STA,
//! `CoInitializeEx(COINIT_APARTMENTTHREADED)`) che riceve i lavori da una coda e risponde su un canale. Chi chiama aspetta al massimo un tempo
//! fissato: se OmniRig resta bloccato il thread si abbandona e la richiesta dopo ne apre uno nuovo (come la sessione PowerShell che si riapriva).
//!
//! Gli eventi di OmniRig (`IOmniRigXEvents`: `StatusChange`, `ParamsChange`...) non si usano: OmniRig li manda a ogni cliente iscritto in modo
//! sincrono, e un cliente che in quel momento aspetta la radio (le attese a scatti di [`crate::omnirig`]) bloccherebbe OmniRig stesso; in piu'
//! servirebbe un `IDispatch` scritto in Rust con gli identificativi degli eventi, che su questo PC (senza OmniRig) non si possono verificare.
//! Aurora legge la radio quando le serve (lettura della frequenza, ricontrollo ogni 10 s di `cat.rs`, come il timer «offline» di WSJT-X),
//! senza un giro continuo: il traffico verso OmniRig resta quello di prima. Tra un lavoro e l'altro il thread smaltisce i messaggi di Windows
//! (`PeekMessage`), come deve fare un appartamento STA.

use std::{
    mem::ManuallyDrop,
    sync::{Mutex, mpsc},
    time::Duration,
};

use windows::{
    Win32::{
        Foundation::{CO_E_CLASSSTRING, REGDB_E_CLASSNOTREG},
        System::{
            Com::{
                CLSCTX_LOCAL_SERVER, CLSIDFromProgID, COINIT_APARTMENTTHREADED, COINIT_DISABLE_OLE1DDE, CoCreateInstance, CoInitializeEx, CoUninitialize, DISPATCH_FLAGS,
                DISPATCH_METHOD, DISPATCH_PROPERTYGET, DISPATCH_PROPERTYPUT, DISPPARAMS, EXCEPINFO, IDispatch,
            },
            Ole::DISPID_PROPERTYPUT,
            Variant::{VAR_CHANGE_FLAGS, VARIANT, VT_BSTR, VT_DISPATCH, VT_I4, VariantChangeType, VariantClear},
        },
        UI::WindowsAndMessaging::{DispatchMessageW, MSG, PM_REMOVE, PeekMessageW, TranslateMessage},
    },
    core::{GUID, HRESULT, PCWSTR, w},
};

use crate::{
    RigError,
    omnirig::{RigApi, ST_ONLINE, explain_omnirig},
};

/// `LOCALE_USER_DEFAULT`: la lingua con cui si cercano i nomi (OmniRig non ne fa uso).
const LCID: u32 = 0x0400;

type Job = Box<dyn FnOnce(&mut Com) + Send>;

/// Un thread COM con la sua coda, aperto alla prima richiesta.
pub(crate) struct Worker {
    name: &'static str,
    timeout: Duration,
    /// Il canale verso il thread vivo e il suo numero (per non abbandonare per sbaglio un thread nuovo).
    slot: Mutex<Option<(u64, mpsc::Sender<Job>)>>,
    next: std::sync::atomic::AtomicU64,
}

impl Worker {
    pub(crate) const fn new(name: &'static str, timeout: Duration) -> Worker {
        Worker { name, timeout, slot: Mutex::new(None), next: std::sync::atomic::AtomicU64::new(0) }
    }

    /// C'e' un thread aperto (quindi, di norma, un oggetto OmniRig tenuto in vita).
    pub(crate) fn is_running(&self) -> bool {
        self.slot.lock().map(|s| s.is_some()).unwrap_or(false)
    }

    /// Chiude il thread: finisce i lavori in coda, lascia gli oggetti di OmniRig (che cosi' puo' chiudersi e liberare la porta) e esce.
    pub(crate) fn stop(&self) {
        if let Ok(mut slot) = self.slot.lock() {
            *slot = None;
        }
    }

    fn sender(&self) -> Result<(u64, mpsc::Sender<Job>), RigError> {
        let mut slot = self.slot.lock().unwrap_or_else(|p| p.into_inner());
        if let Some((generation, tx)) = slot.as_ref() {
            return Ok((*generation, tx.clone()));
        }
        let (tx, rx) = mpsc::channel::<Job>();
        let generation = self.next.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        std::thread::Builder::new()
            .name(format!("aurora-{}", self.name))
            .spawn(move || thread_main(rx))
            .map_err(|e| RigError(format!("OmniRig: thread non avviato: {e}")))?;
        *slot = Some((generation, tx.clone()));
        Ok((generation, tx))
    }

    fn abandon(&self, generation: u64) {
        if let Ok(mut slot) = self.slot.lock()
            && slot.as_ref().is_some_and(|(g, _)| *g == generation)
        {
            *slot = None;
        }
    }

    /// Esegue `f` sul thread COM e ne aspetta il risultato al massimo per il tempo del thread.
    pub(crate) fn run<T: Send + 'static>(&self, f: impl FnOnce(&mut Com) -> Result<T, RigError> + Send + 'static) -> Result<T, RigError> {
        let (reply_tx, reply_rx) = mpsc::sync_channel(1);
        let job: Job = Box::new(move |com| {
            let _ = reply_tx.send(f(com));
        });
        let (generation, tx) = self.sender()?;
        if let Err(mpsc::SendError(job)) = tx.send(job) {
            // il thread e' morto (un panico): se ne apre un altro e si riprova una volta
            self.abandon(generation);
            let (_, tx) = self.sender()?;
            tx.send(job).map_err(|_| RigError("OmniRig: thread non disponibile".into()))?;
        }
        match reply_rx.recv_timeout(self.timeout) {
            Ok(result) => result,
            Err(mpsc::RecvTimeoutError::Timeout) => {
                tracing::warn!("OmniRig: nessuna risposta entro {} s ({}): il thread si abbandona, il prossimo comando ne apre uno nuovo", self.timeout.as_secs(), self.name);
                self.abandon(generation);
                Err(RigError(explain_omnirig("non risponde")))
            }
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                self.abandon(generation);
                Err(RigError("OmniRig: il comando si e' interrotto".into()))
            }
        }
    }
}

fn thread_main(rx: mpsc::Receiver<Job>) {
    let init = unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED | COINIT_DISABLE_OLE1DDE) };
    let mut com = Com { init: init.ok().map_err(|e| format!("OmniRig: COM non inizializzato: {e}")), omni: None, rigs: [None, None] };
    loop {
        match rx.recv_timeout(Duration::from_millis(50)) {
            Ok(job) => job(&mut com),
            Err(mpsc::RecvTimeoutError::Timeout) => {}
            Err(mpsc::RecvTimeoutError::Disconnected) => break,
        }
        pump();
    }
    // prima gli oggetti, poi l'appartamento
    com.forget();
    if init.is_ok() {
        unsafe { CoUninitialize() };
    }
}

/// Smaltisce i messaggi di Windows del thread (un appartamento STA deve farlo).
fn pump() {
    let mut msg = MSG::default();
    unsafe {
        while PeekMessageW(&mut msg, None, 0, 0, PM_REMOVE).as_bool() {
            let _ = TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
    }
}

/// Lo stato del thread COM: l'oggetto `OmniRigX` e le due radio, creati una volta e tenuti in vita.
pub(crate) struct Com {
    init: Result<(), String>,
    omni: Option<Disp>,
    rigs: [Option<Disp>; 2],
}

impl Com {
    fn forget(&mut self) {
        self.rigs = [None, None];
        self.omni = None;
    }

    /// L'oggetto `OmniRigX`: se c'e' si controlla che risponda ancora (`InterfaceVersion`, come il preambolo dello script di prima), se OmniRig
    /// e' stato chiuso si ricrea. Ritorna anche se e' appena stato creato.
    fn omni(&mut self) -> Result<(Disp, bool), RigError> {
        self.init.clone().map_err(RigError)?;
        if self.omni.as_ref().is_some_and(|o| o.get("InterfaceVersion").is_err()) {
            tracing::info!("OmniRig: l'oggetto non risponde piu' (OmniRig chiuso?): lo ricreo");
            self.forget();
        }
        let fresh = self.omni.is_none();
        if fresh {
            let omni = create()?;
            let sw = omni.get("SoftwareVersion").unwrap_or(0);
            let iface = omni.get("InterfaceVersion").unwrap_or(0);
            // `OmniRigTransceiver.cpp:149-152`
            tracing::info!("OmniRig: versione {}.{}, interfaccia {}.{}", (sw >> 16) & 0xffff, sw & 0xffff, (iface >> 8) & 0xff, iface & 0xff);
            self.omni = Some(omni);
        }
        let omni = self.omni.clone().ok_or_else(|| RigError("OmniRig: oggetto non disponibile".into()))?;
        Ok((omni, fresh))
    }

    /// La radio `n` (1 o 2) di OmniRig (`Rig1` / `Rig2`, `OmniRigTransceiver.cpp:155-159`). Appena creato l'oggetto si lascia a OmniRig il
    /// tempo della prima lettura: fino a 5 volte 300 ms finche' `Status` e' `ST_ONLINE` (`OmniRigTransceiver.cpp:171-179`).
    pub(crate) fn rig(&mut self, n: u8) -> Result<Disp, RigError> {
        let index = usize::from(n.clamp(1, 2) - 1);
        let (omni, fresh) = self.omni()?;
        let fresh = fresh || self.rigs[index].is_none();
        if self.rigs[index].is_none() {
            let rig = omni.get_disp(if index == 0 { "Rig1" } else { "Rig2" })?;
            self.rigs[index] = Some(rig);
        }
        let rig = self.rigs[index].clone().ok_or_else(|| RigError("OmniRig: radio non disponibile".into()))?;
        if fresh {
            for _ in 0..5 {
                rig.pause(300);
                if rig.get("Status").is_ok_and(|s| s == ST_ONLINE) {
                    break;
                }
            }
            tracing::info!(
                "OmniRig Rig{n}: tipo {}, stato «{}», parametri leggibili 0x{:08x}, scrivibili 0x{:08x}",
                rig.text("RigType").unwrap_or_default(),
                rig.text("StatusStr").unwrap_or_default(),
                rig.get("ReadableParams").unwrap_or(0),
                rig.get("WriteableParams").unwrap_or(0)
            );
        }
        Ok(rig)
    }
}

/// Crea `OmniRig.OmniRigX` (server locale: il programma OmniRig.exe).
fn create() -> Result<Disp, RigError> {
    let clsid = unsafe { CLSIDFromProgID(w!("OmniRig.OmniRigX")) }.map_err(|e| creation_error(e.code(), &e.message()))?;
    let omni: IDispatch = unsafe { CoCreateInstance(&clsid, None, CLSCTX_LOCAL_SERVER) }.map_err(|e| creation_error(e.code(), &e.message()))?;
    Ok(Disp(omni))
}

fn creation_error(code: HRESULT, message: &str) -> RigError {
    let hex = format!("{:08X}", code.0 as u32);
    if code == CO_E_CLASSSTRING || code == REGDB_E_CLASSNOTREG {
        return RigError(explain_omnirig("80040154"));
    }
    if code.0 as u32 == 0x8007_0005 {
        // E_ACCESSDENIED: di solito uno dei due programmi gira come amministratore e l'altro no
        return RigError(format!(
            "Windows non lascia avviare OmniRig ad Aurora (0x{hex}): Aurora e OmniRig devono girare con gli stessi privilegi (tutti e due normali o tutti e due come amministratore)"
        ));
    }
    RigError(format!("OmniRig non avviabile (0x{hex}): {}", message.trim()))
}

/// Un oggetto di OmniRig comandato per nome.
#[derive(Clone)]
pub(crate) struct Disp(IDispatch);

/// Un `VARIANT` che si libera da solo.
struct Var(VARIANT);

impl Drop for Var {
    fn drop(&mut self) {
        let _ = unsafe { VariantClear(&mut self.0) };
    }
}

impl Var {
    fn empty() -> Var {
        Var(VARIANT::default())
    }

    fn converted(&self, to: windows::Win32::System::Variant::VARENUM, what: &str) -> Result<Var, RigError> {
        let mut out = Var::empty();
        unsafe { VariantChangeType(&mut out.0, &self.0, VAR_CHANGE_FLAGS(0), to) }.map_err(|e| RigError(format!("OmniRig: {what} illeggibile: {}", e.message())))?;
        Ok(out)
    }

    fn to_i32(&self, what: &str) -> Result<i32, RigError> {
        let v = self.converted(VT_I4, what)?;
        Ok(unsafe { v.0.Anonymous.Anonymous.Anonymous.lVal })
    }

    fn to_text(&self, what: &str) -> Result<String, RigError> {
        let v = self.converted(VT_BSTR, what)?;
        Ok(unsafe { v.0.Anonymous.Anonymous.Anonymous.bstrVal.to_string() })
    }

    fn to_disp(&self, what: &str) -> Result<Disp, RigError> {
        unsafe {
            if self.0.Anonymous.Anonymous.vt == VT_DISPATCH
                && let Some(d) = self.0.Anonymous.Anonymous.Anonymous.pdispVal.as_ref()
            {
                return Ok(Disp(d.clone()));
            }
        }
        Err(RigError(format!("OmniRig: {what} non e' un oggetto")))
    }
}

impl Disp {
    fn dispid(&self, name: &str) -> Result<i32, RigError> {
        let wide: Vec<u16> = name.encode_utf16().chain(std::iter::once(0)).collect();
        let names = [PCWSTR(wide.as_ptr())];
        let mut id = 0i32;
        unsafe { self.0.GetIDsOfNames(&GUID::zeroed(), names.as_ptr(), 1, LCID, &mut id) }.map_err(|e| RigError(format!("OmniRig: «{name}» sconosciuto: {}", e.message())))?;
        Ok(id)
    }

    /// `args` nell'ordine di chiamata (si girano qui: `DISPPARAMS` li vuole dall'ultimo al primo).
    fn invoke(&self, name: &str, flags: DISPATCH_FLAGS, args: &[i32]) -> Result<Var, RigError> {
        let id = self.dispid(name)?;
        // solo interi: questi `VARIANT` non hanno nulla da liberare
        let mut raw: Vec<VARIANT> = args.iter().rev().map(|a| variant_i32(*a)).collect();
        let put = flags == DISPATCH_PROPERTYPUT;
        let mut named = DISPID_PROPERTYPUT;
        let params = DISPPARAMS {
            rgvarg: if raw.is_empty() { std::ptr::null_mut() } else { raw.as_mut_ptr() },
            rgdispidNamedArgs: if put { &mut named } else { std::ptr::null_mut() },
            cArgs: raw.len() as u32,
            cNamedArgs: u32::from(put),
        };
        let mut result = Var::empty();
        let mut excep = EXCEPINFO::default();
        let out: Option<*mut VARIANT> = if put { None } else { Some(&mut result.0 as *mut VARIANT) };
        let outcome = unsafe { self.0.Invoke(id, &GUID::zeroed(), LCID, flags, &params, out, Some(&mut excep as *mut EXCEPINFO), None) };
        let EXCEPINFO { bstrSource, bstrDescription, bstrHelpFile, .. } = excep;
        let (_source, description, _help) = (ManuallyDrop::into_inner(bstrSource), ManuallyDrop::into_inner(bstrDescription), ManuallyDrop::into_inner(bstrHelpFile));
        outcome.map_err(|e| {
            let detail = description.to_string();
            RigError(format!("OmniRig: {name}: {}", if detail.trim().is_empty() { e.message() } else { detail.trim().to_string() }))
        })?;
        Ok(result)
    }

    fn get_disp(&self, name: &str) -> Result<Disp, RigError> {
        self.invoke(name, DISPATCH_PROPERTYGET, &[])?.to_disp(name)
    }
}

impl RigApi for Disp {
    fn get(&self, name: &str) -> Result<i32, RigError> {
        self.invoke(name, DISPATCH_PROPERTYGET, &[])?.to_i32(name)
    }

    fn put(&self, name: &str, value: i32) -> Result<(), RigError> {
        self.invoke(name, DISPATCH_PROPERTYPUT, &[value]).map(|_| ())
    }

    fn call(&self, name: &str, args: &[i32]) -> Result<i32, RigError> {
        let result = self.invoke(name, DISPATCH_METHOD, args)?;
        // un metodo senza valore (SetSplitMode) torna VT_EMPTY: vale 0
        if unsafe { result.0.Anonymous.Anonymous.vt } == windows::Win32::System::Variant::VT_EMPTY { Ok(0) } else { result.to_i32(name) }
    }

    fn text(&self, name: &str) -> Result<String, RigError> {
        self.invoke(name, DISPATCH_PROPERTYGET, &[])?.to_text(name)
    }

    /// Le attese tra un comando e l'altro smaltiscono i messaggi del thread (appartamento STA).
    fn pause(&self, ms: u64) {
        let until = std::time::Instant::now() + Duration::from_millis(ms);
        loop {
            pump();
            let left = until.saturating_duration_since(std::time::Instant::now());
            if left.is_zero() {
                break;
            }
            std::thread::sleep(left.min(Duration::from_millis(20)));
        }
    }
}

/// Un `VARIANT` intero (`VT_I4`): i parametri di OmniRig (frequenze in Hz, valori `PM_*`) sono tutti `Integer`.
fn variant_i32(value: i32) -> VARIANT {
    let mut v = VARIANT::default();
    unsafe {
        (*v.Anonymous.Anonymous).vt = VT_I4;
        (*v.Anonymous.Anonymous).Anonymous.lVal = value;
    }
    v
}
