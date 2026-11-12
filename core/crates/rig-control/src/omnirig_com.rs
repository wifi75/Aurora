//! Il cliente COM di OmniRig, senza PowerShell: l'oggetto `OmniRig.OmniRigX` si crea con `CoCreateInstance` e si comanda per nome
//! (`IDispatch`, `GetIDsOfNames` + `Invoke`), come fa WSJT-X con l'oggetto generato da `dumpcpp` (`OmniRigTransceiver.cpp:130`, `do_start`).
//! Prima ogni comando passava da una sessione di PowerShell: dove l'antivirus o i criteri di Windows bloccano PowerShell («accesso negato»)
//! OmniRig non rispondeva mai, nemmeno al PTT.
//!
//! Gli oggetti COM appartengono all'appartamento (thread) che li ha creati: tutte le chiamate a OmniRig girano su un thread dedicato (STA,
//! `CoInitializeEx(COINIT_APARTMENTTHREADED)`) che riceve i lavori da una coda e risponde su un canale. Chi chiama aspetta al massimo un tempo
//! fissato: se OmniRig resta bloccato il thread si abbandona e la richiesta dopo ne apre uno nuovo (come la sessione PowerShell che si riapriva).
//!
//! Rapporti del 2026-11-11 (i5-3570, Windows 10: «nessuna risposta entro 20 s», «PTT: OmniRig non risponde»). WSJT-X non ha tempi massimi:
//! le chiamate dell'oggetto `dumpcpp` sono sincrone e, alla partenza, aspetta solo 5 volte 300 ms che `Status` diventi `ST_ONLINE`
//! (`OmniRigTransceiver.cpp:171-185`); una radio che va fuori linea la aspetta 10 s prima di dirlo (`offline_timer_`, righe 392-404). Qui:
//! - la prima creazione (che puo' avviare OmniRig.exe) ha piu' tempo (`Worker::first_timeout`) e il thread che la sta facendo non si abbandona;
//! - un tempo scaduto mentre OmniRig si avvia o con la radio non in linea e' un avviso per l'operatore, non un errore da segnalare: lo e' solo
//!   quando si ripete con la radio in linea ([`crate::omnirig::timeout_verdict`]);
//! - un lavoro arrivato in ritardo non parte piu' (`Wait::skip_if_late`), tranne lo spegnimento del PTT;
//! - con [`MAX_STUCK`] thread ancora bloccati dentro OmniRig non se ne aprono altri.
//!
//! Gli eventi di OmniRig (`IOmniRigXEvents`: `StatusChange`, `ParamsChange`...) non si usano: OmniRig li manda a ogni cliente iscritto in modo
//! sincrono, e un cliente che in quel momento aspetta la radio (le attese a scatti di [`crate::omnirig`]) bloccherebbe OmniRig stesso; in piu'
//! servirebbe un `IDispatch` scritto in Rust con gli identificativi degli eventi, che su questo PC (senza OmniRig) non si possono verificare.
//! Aurora legge la radio quando le serve (lettura della frequenza, ricontrollo ogni 10 s di `cat.rs`, come il timer «offline» di WSJT-X),
//! senza un giro continuo: il traffico verso OmniRig resta quello di prima. Tra un lavoro e l'altro il thread smaltisce i messaggi di Windows
//! (`PeekMessage`), come deve fare un appartamento STA.

use std::{
    mem::ManuallyDrop,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, AtomicU8, AtomicU32, AtomicU64, Ordering},
        mpsc,
    },
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
    omnirig::{RigApi, ST_ONLINE, TimeoutCause, explain_omnirig, last_status, note_status, timeout_verdict},
};

/// `LOCALE_USER_DEFAULT`: la lingua con cui si cercano i nomi (OmniRig non ne fa uso).
const LCID: u32 = 0x0400;

/// Thread abbandonati e ancora bloccati dentro OmniRig oltre i quali non se ne apre un altro: OmniRig e' bloccato e ogni thread nuovo si
/// bloccherebbe allo stesso modo (prima se ne apriva uno a ogni ricontrollo, ogni 10 s, e restavano tutti appesi).
const MAX_STUCK: usize = 2;

type Job = Box<dyn FnOnce(&mut Com) + Send>;

/// Lo stato di un thread COM: in corso, abbandonato da chi aspettava (ancora bloccato in una chiamata), uscito.
const RUNNING: u8 = 0;
const ABANDONED: u8 = 1;
const EXITED: u8 = 2;

/// Il thread COM vivo: il canale dei lavori, se l'oggetto OmniRig e' pronto (creato e primo contatto fatto) e il suo stato.
struct Live {
    generation: u64,
    tx: mpsc::Sender<Job>,
    ready: Arc<AtomicBool>,
    state: Arc<AtomicU8>,
}

/// Come aspettare un lavoro.
#[derive(Clone, Copy)]
pub(crate) struct Wait {
    /// Un tempo diverso da quello del thread (per esempio la preparazione dell'oggetto del PTT, che puo' avviare OmniRig).
    pub(crate) allowance: Option<Duration>,
    /// Il lavoro non parte piu' se chi l'ha chiesto ha gia' smesso di aspettare (cambi di frequenza e letture vecchie, PTT acceso in
    /// ritardo). Lo spegnimento del PTT invece parte sempre.
    pub(crate) skip_if_late: bool,
}

/// Un thread COM con la sua coda, aperto alla prima richiesta.
pub(crate) struct Worker {
    name: &'static str,
    /// Il tempo massimo di un lavoro con l'oggetto OmniRig gia' pronto.
    timeout: Duration,
    /// Il tempo massimo finche' l'oggetto non e' pronto: la prima creazione puo' dover avviare OmniRig.exe (che mostra la sua finestra e apre
    /// la porta della radio) e su un PC lento ci vogliono molti secondi (rapporti del 2026-11-11: «nessuna risposta entro 20 s»).
    first_timeout: Duration,
    /// Il thread vivo (con il suo numero, per non abbandonare per sbaglio un thread nuovo).
    slot: Mutex<Option<Live>>,
    next: AtomicU64,
    /// Gli stati dei thread abbandonati: quelli ancora `ABANDONED` sono bloccati dentro OmniRig.
    abandoned: Mutex<Vec<Arc<AtomicU8>>>,
    /// Tempi scaduti di fila con la radio in linea (solo questi, ripetuti, sono un errore da segnalare).
    online_timeouts: AtomicU32,
}

/// Il canale verso il thread vivo con il suo numero, la sua prontezza e il suo stato.
type Sender = (u64, mpsc::Sender<Job>, Arc<AtomicBool>, Arc<AtomicU8>);

impl Worker {
    pub(crate) const fn new(name: &'static str, timeout: Duration, first_timeout: Duration) -> Worker {
        Worker {
            name,
            timeout,
            first_timeout,
            slot: Mutex::new(None),
            next: AtomicU64::new(0),
            abandoned: Mutex::new(Vec::new()),
            online_timeouts: AtomicU32::new(0),
        }
    }

    /// C'e' un thread aperto (quindi, di norma, un oggetto OmniRig tenuto in vita).
    pub(crate) fn is_running(&self) -> bool {
        self.slot.lock().map(|s| s.is_some()).unwrap_or(false)
    }

    /// Chiude il thread: finisce i lavori in coda, lascia gli oggetti di OmniRig (che cosi' puo' chiudersi e liberare la porta) e esce.
    pub(crate) fn stop(&self) {
        let live = self.slot.lock().ok().and_then(|mut slot| slot.take());
        if let Some(live) = live
            && live.state.compare_exchange(RUNNING, ABANDONED, Ordering::SeqCst, Ordering::SeqCst).is_ok()
            && let Ok(mut list) = self.abandoned.lock()
        {
            list.retain(|s| s.load(Ordering::SeqCst) == ABANDONED);
            list.push(Arc::clone(&live.state));
        }
    }

    /// Il thread vivo, o uno nuovo. Con troppi thread ancora bloccati dentro OmniRig non se ne apre un altro.
    fn sender(&self) -> Result<Sender, RigError> {
        let mut slot = self.slot.lock().unwrap_or_else(|p| p.into_inner());
        if let Some(live) = slot.as_ref() {
            return Ok((live.generation, live.tx.clone(), Arc::clone(&live.ready), Arc::clone(&live.state)));
        }
        if self.stuck() >= MAX_STUCK {
            tracing::info!("OmniRig ({}): {MAX_STUCK} comandi ancora bloccati dentro OmniRig, non apro un altro thread", self.name);
            return Err(RigError(explain_omnirig("non risponde")));
        }
        let (tx, rx) = mpsc::channel::<Job>();
        let generation = self.next.fetch_add(1, Ordering::SeqCst);
        let (ready, state) = (Arc::new(AtomicBool::new(false)), Arc::new(AtomicU8::new(RUNNING)));
        let (thread_ready, thread_state) = (Arc::clone(&ready), Arc::clone(&state));
        std::thread::Builder::new()
            .name(format!("aurora-{}", self.name))
            .spawn(move || {
                // uscito, anche per un panico (anche se era stato abbandonato): non conta piu' tra i bloccati
                let _exit = ExitMark(thread_state);
                thread_main(rx, thread_ready);
            })
            .map_err(|e| RigError(format!("OmniRig: thread non avviato: {e}")))?;
        *slot = Some(Live { generation, tx: tx.clone(), ready: Arc::clone(&ready), state: Arc::clone(&state) });
        Ok((generation, tx, ready, state))
    }

    /// Lascia il thread (bloccato in una chiamata): il prossimo comando ne apre uno nuovo.
    fn abandon(&self, generation: u64, state: &Arc<AtomicU8>) {
        if let Ok(mut slot) = self.slot.lock()
            && slot.as_ref().is_some_and(|l| l.generation == generation)
        {
            *slot = None;
        }
        if state.compare_exchange(RUNNING, ABANDONED, Ordering::SeqCst, Ordering::SeqCst).is_ok()
            && let Ok(mut list) = self.abandoned.lock()
        {
            list.retain(|s| s.load(Ordering::SeqCst) == ABANDONED);
            list.push(Arc::clone(state));
        }
    }

    /// I thread abbandonati ancora bloccati dentro OmniRig (quelli usciti nel frattempo non contano).
    fn stuck(&self) -> usize {
        let Ok(mut list) = self.abandoned.lock() else { return 0 };
        list.retain(|s| s.load(Ordering::SeqCst) == ABANDONED);
        list.len()
    }

    /// Esegue `f` sul thread COM e ne aspetta il risultato per il tempo scelto da `wait` (o quello del thread: normale con l'oggetto pronto,
    /// piu' lungo mentre si crea). Un tempo scaduto mentre OmniRig si avvia o con la radio non in linea e' un avviso per l'operatore; solo
    /// i tempi scaduti ripetuti con la radio in linea (OmniRig bloccato) restano un errore ([`timeout_verdict`]).
    pub(crate) fn run_with<T: Send + 'static>(&self, wait: Wait, f: impl FnOnce(&mut Com) -> Result<T, RigError> + Send + 'static) -> Result<T, RigError> {
        let (reply_tx, reply_rx) = mpsc::sync_channel(1);
        let late = Arc::new(AtomicBool::new(false));
        let job_late = Arc::clone(&late);
        let job: Job = Box::new(move |com| {
            if wait.skip_if_late && job_late.load(Ordering::SeqCst) {
                return;
            }
            com.late = wait.skip_if_late.then(|| Arc::clone(&job_late));
            let _ = reply_tx.send(f(com));
            com.late = None;
        });
        let (mut generation, tx, mut ready, mut state) = self.sender()?;
        if let Err(mpsc::SendError(job)) = tx.send(job) {
            // il thread e' morto (un panico): se ne apre un altro e si riprova una volta
            self.abandon(generation, &state);
            let (g, tx, r, s) = self.sender()?;
            (generation, ready, state) = (g, r, s);
            tx.send(job).map_err(|_| RigError("OmniRig: thread non disponibile".into()))?;
        }
        let limit = wait.allowance.unwrap_or(if ready.load(Ordering::SeqCst) { self.timeout } else { self.first_timeout });
        match reply_rx.recv_timeout(limit) {
            Ok(result) => {
                self.online_timeouts.store(0, Ordering::SeqCst);
                result
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {
                late.store(true, Ordering::SeqCst);
                let starting = !ready.load(Ordering::SeqCst);
                // un thread che sta ancora creando l'oggetto (OmniRig che si avvia) si tiene: i lavori in coda aspettano lui e quelli scaduti
                // non partono. Uno bloccato in una chiamata a un oggetto gia' pronto si abbandona: il prossimo comando ne apre uno nuovo
                if !starting {
                    self.abandon(generation, &state);
                }
                let status = last_status();
                let hung_before = self.online_timeouts.load(Ordering::SeqCst) > 0;
                let repeated = if status != ST_ONLINE || (starting && !hung_before) { 0 } else { self.online_timeouts.fetch_add(1, Ordering::SeqCst) + 1 };
                let (cause, message) = timeout_verdict(starting && repeated == 0, status, repeated);
                let secs = limit.as_secs();
                match cause {
                    TimeoutCause::Starting => tracing::info!("OmniRig: nessuna risposta entro {secs} s ({}) mentre si prepara l'oggetto (OmniRig che si avvia?)", self.name),
                    TimeoutCause::Offline => tracing::info!("OmniRig: nessuna risposta entro {secs} s ({}) con la radio non in linea (stato {status})", self.name),
                    TimeoutCause::OnlineOnce => tracing::info!("OmniRig: nessuna risposta entro {secs} s ({}) con la radio in linea: il thread si abbandona", self.name),
                    TimeoutCause::OnlineRepeated => {
                        tracing::warn!("OmniRig: nessuna risposta entro {secs} s ({}), {repeated} volte di fila con la radio in linea: il thread si abbandona", self.name)
                    }
                }
                Err(RigError(message))
            }
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                // nessuna risposta: il thread e' caduto (un panico nel lavoro)
                self.abandon(generation, &state);
                Err(RigError("OmniRig: il comando si e' interrotto".into()))
            }
        }
    }
}

/// Segna il thread come uscito quando finisce, in ogni modo.
struct ExitMark(Arc<AtomicU8>);

impl Drop for ExitMark {
    fn drop(&mut self) {
        self.0.store(EXITED, Ordering::SeqCst);
    }
}

fn thread_main(rx: mpsc::Receiver<Job>, ready: Arc<AtomicBool>) {
    let init = unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED | COINIT_DISABLE_OLE1DDE) };
    let mut com = Com { init: init.ok().map_err(|e| format!("OmniRig: COM non inizializzato: {e}")), omni: None, rigs: [None, None], ready, late: None };
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
    /// L'oggetto e la radio ci sono e il primo contatto e' fatto: da qui i lavori hanno il tempo normale.
    ready: Arc<AtomicBool>,
    /// Per il lavoro in corso: chi l'ha chiesto ha gia' smesso di aspettare (vedi [`Com::is_late`]).
    late: Option<Arc<AtomicBool>>,
}

impl Com {
    /// Il lavoro in corso e' scaduto per chi l'aveva chiesto (e non deve partire in ritardo): si guarda dopo la creazione dell'oggetto, che
    /// con OmniRig che si avvia puo' durare decine di secondi.
    pub(crate) fn is_late(&self) -> bool {
        self.late.as_ref().is_some_and(|l| l.load(Ordering::SeqCst))
    }

    fn forget(&mut self) {
        self.ready.store(false, Ordering::SeqCst);
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
        self.ready.store(true, Ordering::SeqCst);
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
        let value = self.invoke(name, DISPATCH_PROPERTYGET, &[])?.to_i32(name)?;
        if name == "Status" {
            // l'ultimo stato letto: dice se un tempo scaduto e' di una radio non in linea (avviso) o di OmniRig bloccato (errore)
            note_status(value);
        }
        Ok(value)
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
