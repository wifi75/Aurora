//! Un finto `rigctld` per le prove (di questo crate e di chi lo usa): risponde come quello vero, ricorda i comandi ricevuti e si puo'
//! far guastare (timeout, rifiuti, connessioni chiuse). Non serve nessuna radio ne' nessun eseguibile.

use std::{
    io::{BufRead, BufReader, Write},
    net::{SocketAddr, TcpListener, TcpStream},
    sync::{
        atomic::{AtomicBool, AtomicI32, AtomicU8, AtomicU64, Ordering},
        Arc, Mutex,
    },
    time::Duration,
};

use crate::{hamlib::RigctldClient, RigError};

/// Lo stato della finta radio, condiviso con i thread delle connessioni.
pub struct FakeState {
    freq: AtomicU64,
    mode: Mutex<String>,
    ptt: AtomicBool,
    split: AtomicBool,
    log: Mutex<Vec<String>>,
    /// Codice `RPRT` con cui rispondere a tutto (0 = nessun guasto).
    fail_code: AtomicI32,
    /// Il `T 0` fallisce con un timeout.
    fail_ptt_off: AtomicBool,
    /// Chiude ogni connessione appena arriva un comando.
    hang_up: AtomicBool,
    /// `F` riporta il modo a USB (la radio che richiama il modo memorizzato per banda).
    recall_mode: AtomicBool,
    stop: AtomicBool,
    /// I livelli che la finta radio sa dare (`STRENGTH`, `SWR`...): valore come lo scrive rigctld. Gli altri rispondono `RPRT -11`.
    levels: Mutex<Vec<(String, String)>>,
    /// Potenza di fondo scala in mW per `\power2mW` (0 = non supportato).
    full_power_mw: AtomicU64,
    /// Le funzioni acceso/spento che la finta radio sa dare (`u NB`...): nome e stato. Le altre rispondono `RPRT -11`.
    funcs: Mutex<Vec<(String, bool)>>,
    /// Le operazioni VFO ricevute (`G TUNE`...), accettate solo se la radio finta ha la funzione TUNER.
    ops: Mutex<Vec<String>>,
    /// Coppie (livello, valore) che `L` rifiuta con `RPRT -9`.
    refused: Mutex<Vec<(String, String)>>,
    /// RIT e XIT in Hz.
    offsets: Mutex<[i32; 2]>,
    vfo: Mutex<String>,
    /// Radio a due bande come l'IC-820H (`enable_dual_bands`): Main = banda 136-174 MHz, Sub = banda 430-450 MHz; una frequenza scritta nella banda
    /// sbagliata viene ripiegata di 300 MHz (437,79 -> 137,79, 145,99 -> 445,99) e `f` legge il VFO in uso.
    dual_bands: AtomicBool,
    band_freqs: Mutex<[u64; 2]>,
    current_band: AtomicU8,
    /// `get_vfo_info` passa al VFO chiesto e non torna indietro (come certe Icom con Hamlib).
    vfo_info_switches: AtomicBool,
    /// `v` risponde `RPRT -11` (la radio non dice su quale VFO e': IC-756PRO con Hamlib).
    vfo_read_unavailable: AtomicBool,
    /// Ritardo di ogni risposta in ms (una seriale lenta).
    delay_ms: AtomicU64,
    /// Codice `RPRT` con cui `X` (modo del VFO di trasmissione) risponde (0 = accettato).
    x_fail_code: AtomicI32,
    /// Modi che `M` rifiuta con `RPRT -1` (la radio non ha quel modo: `PKTUSB` sulle Icom senza modo dati).
    refused_modes: Mutex<Vec<String>>,
    /// `G CPY` (copia A=B) accettato; altrimenti `RPRT -1` come le radio senza `vfo_op CPY`.
    cpy_ok: AtomicBool,
    /// Codice `RPRT` con cui `S` (split) risponde (0 = accettato): -11 come l'IC-820H.
    s_fail_code: AtomicI32,
    /// I nomi di VFO che `V` rifiuta con `RPRT -11` (`Sub`, `Main`...).
    refused_vfos: Mutex<Vec<String>>,
}

pub struct FakeRigctld {
    pub addr: SocketAddr,
    state: Arc<FakeState>,
}

impl FakeRigctld {
    pub fn start(freq: u64) -> FakeRigctld {
        let state = Arc::new(FakeState {
            freq: AtomicU64::new(freq),
            mode: Mutex::new("PKTUSB".into()),
            ptt: AtomicBool::new(false),
            split: AtomicBool::new(false),
            log: Mutex::new(Vec::new()),
            fail_code: AtomicI32::new(0),
            fail_ptt_off: AtomicBool::new(false),
            hang_up: AtomicBool::new(false),
            recall_mode: AtomicBool::new(false),
            stop: AtomicBool::new(false),
            levels: Mutex::new(Vec::new()),
            full_power_mw: AtomicU64::new(0),
            funcs: Mutex::new(Vec::new()),
            ops: Mutex::new(Vec::new()),
            refused: Mutex::new(Vec::new()),
            offsets: Mutex::new([0, 0]),
            vfo: Mutex::new("VFOA".into()),
            dual_bands: AtomicBool::new(false),
            band_freqs: Mutex::new([145_000_000, 435_000_000]),
            current_band: AtomicU8::new(0),
            vfo_info_switches: AtomicBool::new(false),
            vfo_read_unavailable: AtomicBool::new(false),
            delay_ms: AtomicU64::new(0),
            x_fail_code: AtomicI32::new(0),
            refused_modes: Mutex::new(Vec::new()),
            cpy_ok: AtomicBool::new(false),
            s_fail_code: AtomicI32::new(0),
            refused_vfos: Mutex::new(Vec::new()),
        });
        let listener = TcpListener::bind("127.0.0.1:0").expect("porta locale per il finto rigctld");
        let addr = listener.local_addr().expect("indirizzo del finto rigctld");
        let shared = Arc::clone(&state);
        std::thread::spawn(move || {
            for stream in listener.incoming() {
                if shared.stop.load(Ordering::SeqCst) {
                    break;
                }
                let Ok(stream) = stream else { continue };
                let state = Arc::clone(&shared);
                std::thread::spawn(move || serve(stream, &state));
            }
        });
        FakeRigctld { addr, state }
    }

    /// Un client nuovo verso questo finto demone (ogni chiamata e' una connessione, come una riconnessione vera).
    pub fn client(&self, io_timeout: Duration) -> Result<RigctldClient, RigError> {
        RigctldClient::connect_with_timeout(self.addr, Duration::from_secs(2), io_timeout)
    }

    pub fn commands(&self) -> Vec<String> {
        self.state.log.lock().map(|l| l.clone()).unwrap_or_default()
    }

    /// `S` risponde con questo codice (per esempio -11: la radio non ha lo split da CAT).
    pub fn fail_split_with(&self, code: i32) {
        self.state.s_fail_code.store(code, Ordering::SeqCst);
    }

    /// `V` rifiuta questo nome di VFO con `RPRT -11`.
    pub fn refuse_vfo(&self, name: &str) {
        if let Ok(mut r) = self.state.refused_vfos.lock() {
            r.push(name.to_string());
        }
    }

    /// La finta radio diventa a due bande (IC-820H): Main = 144 MHz, Sub = 430 MHz, con le frequenze di partenza date.
    pub fn enable_dual_bands(&self, main_hz: u64, sub_hz: u64) {
        self.state.dual_bands.store(true, Ordering::SeqCst);
        if let Ok(mut b) = self.state.band_freqs.lock() {
            *b = [main_hz, sub_hz];
        }
        self.state.current_band.store(0, Ordering::SeqCst);
    }

    /// Le frequenze di Main e di Sub della radio a due bande.
    pub fn band_frequencies(&self) -> [u64; 2] {
        self.state.band_freqs.lock().map(|b| *b).unwrap_or([0, 0])
    }

    pub fn clear_commands(&self) {
        if let Ok(mut l) = self.state.log.lock() {
            l.clear();
        }
    }

    pub fn set_frequency(&self, hz: u64) {
        self.state.freq.store(hz, Ordering::SeqCst);
    }

    pub fn frequency(&self) -> u64 {
        self.state.freq.load(Ordering::SeqCst)
    }

    pub fn set_mode(&self, mode: &str) {
        if let Ok(mut m) = self.state.mode.lock() {
            *m = mode.to_string();
        }
    }

    /// La radio finta rifiuta questo modo (`RPRT -1`).
    pub fn refuse_mode(&self, mode: &str) {
        if let Ok(mut r) = self.state.refused_modes.lock() {
            r.push(mode.to_string());
        }
    }

    /// Cambia il modo della radio finta come se lo avesse girato l'operatore (senza passare da un comando).
    pub fn set_mode_by_hand(&self, mode: &str) {
        if let Ok(mut m) = self.state.mode.lock() {
            *m = mode.to_string();
        }
    }

    pub fn mode(&self) -> String {
        self.state.mode.lock().map(|m| m.clone()).unwrap_or_default()
    }

    pub fn ptt(&self) -> bool {
        self.state.ptt.load(Ordering::SeqCst)
    }

    /// Il PTT acceso o spento sulla radio da fuori (rimasto acceso da una trasmissione interrotta).
    pub fn set_ptt(&self, on: bool) {
        self.state.ptt.store(on, Ordering::SeqCst);
    }

    pub fn split(&self) -> bool {
        self.state.split.load(Ordering::SeqCst)
    }

    /// Split acceso o spento sulla radio da fuori (a mano, da un altro programma sulla condivisione).
    pub fn set_split(&self, on: bool) {
        self.state.split.store(on, Ordering::SeqCst);
    }

    /// Risponde `RPRT code` a ogni comando (-5 = timeout della radio, -9 = comando rifiutato); 0 toglie il guasto.
    pub fn fail_with(&self, code: i32) {
        self.state.fail_code.store(code, Ordering::SeqCst);
    }

    /// `X` (modo del VFO di trasmissione) risponde `RPRT code`; 0 lo accetta.
    pub fn fail_split_mode_with(&self, code: i32) {
        self.state.x_fail_code.store(code, Ordering::SeqCst);
    }

    /// `G CPY` (copia A=B) accettato (`true`) o rifiutato con `RPRT -1` (`false`, predefinito).
    pub fn accept_vfo_copy(&self, on: bool) {
        self.state.cpy_ok.store(on, Ordering::SeqCst);
    }

    pub fn fail_ptt_off(&self, on: bool) {
        self.state.fail_ptt_off.store(on, Ordering::SeqCst);
    }

    pub fn hang_up(&self, on: bool) {
        self.state.hang_up.store(on, Ordering::SeqCst);
    }

    /// La finta radio da' (o cambia) questo livello, ad esempio `set_level("STRENGTH", "-12")`.
    pub fn set_level(&self, name: &str, value: &str) {
        if let Ok(mut levels) = self.state.levels.lock() {
            match levels.iter_mut().find(|(n, _)| n == name) {
                Some(entry) => entry.1 = value.to_string(),
                None => levels.push((name.to_string(), value.to_string())),
            }
        }
    }

    /// La finta radio da' (o cambia) questa funzione acceso/spento, ad esempio `set_func("NB", true)`.
    /// Le operazioni VFO ricevute finora.
    pub fn ops(&self) -> Vec<String> {
        self.state.ops.lock().map(|o| o.clone()).unwrap_or_default()
    }

    pub fn set_func(&self, name: &str, on: bool) {
        if let Ok(mut funcs) = self.state.funcs.lock() {
            match funcs.iter_mut().find(|(n, _)| n == name) {
                Some(entry) => entry.1 = on,
                None => funcs.push((name.to_string(), on)),
            }
        }
    }

    pub fn func(&self, name: &str) -> Option<bool> {
        self.state.funcs.lock().ok().and_then(|f| f.iter().find(|(n, _)| n == name).map(|(_, v)| *v))
    }

    /// `L name value` verra' rifiutato con `RPRT -9`.
    pub fn refuse_level_value(&self, name: &str, value: &str) {
        if let Ok(mut r) = self.state.refused.lock() {
            r.push((name.to_string(), value.to_string()));
        }
    }

    pub fn level(&self, name: &str) -> Option<String> {
        self.state.levels.lock().ok().and_then(|l| l.iter().find(|(n, _)| n == name).map(|(_, v)| v.clone()))
    }

    pub fn set_offset(&self, xit: bool, hz: i32) {
        if let Ok(mut o) = self.state.offsets.lock() {
            o[usize::from(xit)] = hz;
        }
    }

    pub fn offset(&self, xit: bool) -> i32 {
        self.state.offsets.lock().map_or(0, |o| o[usize::from(xit)])
    }

    /// La radio finta lascia il VFO letto da `get_vfo_info` come VFO in uso.
    /// La radio finta non sa dire su quale VFO e' (`v` -> `RPRT -11`).
    pub fn set_vfo_read_unavailable(&self, on: bool) {
        self.state.vfo_read_unavailable.store(on, Ordering::SeqCst);
    }

    pub fn set_vfo_info_switches(&self, on: bool) {
        self.state.vfo_info_switches.store(on, Ordering::SeqCst);
    }

    /// Il testo che la radio finta risponde a `v` (anche una riga che non e' un VFO, come una risposta rimasta in coda).
    pub fn set_vfo_text(&self, text: &str) {
        if let Ok(mut v) = self.state.vfo.lock() {
            *v = text.to_string();
        }
    }

    pub fn vfo(&self) -> String {
        self.state.vfo.lock().map(|v| v.clone()).unwrap_or_default()
    }

    /// Il fondo scala della potenza (mW) per `\power2mW`.
    pub fn set_full_power_mw(&self, mw: u64) {
        self.state.full_power_mw.store(mw, Ordering::SeqCst);
    }

    /// Quante volte e' arrivato questo comando (testo esatto).
    pub fn count(&self, command: &str) -> usize {
        self.commands().iter().filter(|c| *c == command).count()
    }

    /// Ogni risposta arriva dopo `ms` millisecondi, come da una radio su una seriale lenta.
    pub fn set_delay_ms(&self, ms: u64) {
        self.state.delay_ms.store(ms, Ordering::SeqCst);
    }

    pub fn recall_mode_on_qsy(&self, on: bool) {
        self.state.recall_mode.store(on, Ordering::SeqCst);
    }
}

impl Drop for FakeRigctld {
    fn drop(&mut self) {
        self.state.stop.store(true, Ordering::SeqCst);
        // sveglia il thread che aspetta una connessione
        let _ = TcpStream::connect(self.addr);
    }
}

fn serve(stream: TcpStream, state: &FakeState) {
    let Ok(clone) = stream.try_clone() else { return };
    let mut reader = BufReader::new(clone);
    let mut writer = stream;
    let mut line = String::new();
    loop {
        line.clear();
        if reader.read_line(&mut line).unwrap_or(0) == 0 {
            return;
        }
        let cmd = line.trim().to_string();
        if cmd == "q" {
            return;
        }
        if let Ok(mut log) = state.log.lock() {
            log.push(cmd.clone());
        }
        if state.hang_up.load(Ordering::SeqCst) {
            return;
        }
        let delay = state.delay_ms.load(Ordering::SeqCst);
        if delay > 0 {
            std::thread::sleep(Duration::from_millis(delay));
        }
        let fail = state.fail_code.load(Ordering::SeqCst);
        let mut parts = cmd.split_whitespace();
        let head = parts.next().unwrap_or("");
        let arg = parts.next().unwrap_or("");
        let arg2 = parts.next().unwrap_or("");
        let reply = if fail != 0 {
            format!("RPRT {fail}\n")
        } else {
            match head {
                "f" if state.dual_bands.load(Ordering::SeqCst) => {
                    let band = usize::from(state.current_band.load(Ordering::SeqCst));
                    format!("{}\n", state.band_freqs.lock().map(|b| b[band]).unwrap_or(0))
                }
                "f" => format!("{}\n", state.freq.load(Ordering::SeqCst)),
                "m" => format!("{}\n2400\n", state.mode.lock().map(|m| m.clone()).unwrap_or_default()),
                "t" => format!("{}\n", u8::from(state.ptt.load(Ordering::SeqCst))),
                "s" => format!("{}\nVFOB\n", u8::from(state.split.load(Ordering::SeqCst))),
                "F" if state.dual_bands.load(Ordering::SeqCst) => match arg.parse::<u64>() {
                    Ok(hz) => {
                        let band = state.current_band.load(Ordering::SeqCst);
                        // la banda sbagliata non esiste: la radio ripiega la frequenza di 300 MHz
                        let folded = if band == 0 && hz >= 300_000_000 {
                            hz - 300_000_000
                        } else if band == 1 && hz < 300_000_000 {
                            hz + 300_000_000
                        } else {
                            hz
                        };
                        if let Ok(mut b) = state.band_freqs.lock() {
                            b[usize::from(band)] = folded;
                        }
                        "RPRT 0\n".into()
                    }
                    Err(_) => "RPRT -1\n".into(),
                },
                "F" => match arg.parse::<u64>() {
                    Ok(hz) => {
                        state.freq.store(hz, Ordering::SeqCst);
                        if state.recall_mode.load(Ordering::SeqCst)
                            && let Ok(mut m) = state.mode.lock()
                        {
                            *m = "USB".into();
                        }
                        "RPRT 0\n".into()
                    }
                    Err(_) => "RPRT -1\n".into(),
                },
                "M" => {
                    if state.refused_modes.lock().is_ok_and(|r| r.iter().any(|x| x == arg)) {
                        "RPRT -1\n".into()
                    } else {
                        if let Ok(mut m) = state.mode.lock() {
                            *m = arg.to_string();
                        }
                        "RPRT 0\n".into()
                    }
                }
                "T" => {
                    if arg == "0" && state.fail_ptt_off.load(Ordering::SeqCst) {
                        "RPRT -5\n".into()
                    } else {
                        state.ptt.store(arg == "1", Ordering::SeqCst);
                        "RPRT 0\n".into()
                    }
                }
                "S" if state.s_fail_code.load(Ordering::SeqCst) != 0 => format!("RPRT {}
", state.s_fail_code.load(Ordering::SeqCst)),
                "V" if state.refused_vfos.lock().is_ok_and(|r| r.iter().any(|x| x == arg)) => "RPRT -11
".into(),
                "S" => {
                    state.split.store(arg == "1", Ordering::SeqCst);
                    "RPRT 0\n".into()
                }
                "X" => format!("RPRT {}\n", state.x_fail_code.load(Ordering::SeqCst)),
                "I" => "RPRT 0\n".into(),
                "l" => {
                    let value = state.levels.lock().ok().and_then(|l| l.iter().find(|(n, _)| n == arg).map(|(_, v)| v.clone()));
                    value.map_or_else(|| "RPRT -11\n".to_string(), |v| format!("{v}\n"))
                }
                "L" => {
                    let refused = state.refused.lock().is_ok_and(|r| r.iter().any(|(n, v)| n == arg && v == arg2));
                    let mut levels = state.levels.lock().ok();
                    match levels.as_mut().and_then(|l| l.iter_mut().find(|(n, _)| n == arg)) {
                        _ if refused => "RPRT -9
".to_string(),
                        Some(entry) => {
                            entry.1 = arg2.to_string();
                            "RPRT 0
".to_string()
                        }
                        None => "RPRT -11
".to_string(),
                    }
                }
                "G" => {
                    let tuner = state.funcs.lock().is_ok_and(|f| f.iter().any(|(n, _)| n == "TUNER"));
                    if arg == "TUNE" && tuner {
                        if let Ok(mut o) = state.ops.lock() {
                            o.push(arg.to_string());
                        }
                        "RPRT 0
".into()
                    } else if arg == "CPY" {
                        if state.cpy_ok.load(Ordering::SeqCst) { "RPRT 0
".into() } else { "RPRT -1
".into() }
                    } else {
                        "RPRT -11
".into()
                    }
                }
                "u" => {
                    let value = state.funcs.lock().ok().and_then(|f| f.iter().find(|(n, _)| n == arg).map(|(_, v)| *v));
                    value.map_or_else(|| "RPRT -11
".to_string(), |v| format!("{}
", u8::from(v)))
                }
                "U" => match state.funcs.lock().ok().and_then(|mut f| f.iter_mut().find(|(n, _)| n == arg).map(|e| e.1 = arg2 == "1")) {
                    Some(()) => "RPRT 0
".into(),
                    None => "RPRT -11
".into(),
                },
                "j" | "z" => format!("{}
", state.offsets.lock().map_or(0, |o| o[usize::from(head == "z")])),
                "J" | "Z" => match arg.parse::<i32>() {
                    Ok(hz) => {
                        if let Ok(mut o) = state.offsets.lock() {
                            o[usize::from(head == "Z")] = hz;
                        }
                        "RPRT 0
".into()
                    }
                    Err(_) => "RPRT -1
".into(),
                },
                "v" if state.vfo_read_unavailable.load(Ordering::SeqCst) => "RPRT -11\n".into(),
                "v" => format!("{}
", state.vfo.lock().map(|v| v.clone()).unwrap_or_default()),
                "V" => {
                    if let Ok(mut v) = state.vfo.lock() {
                        *v = arg.to_string();
                    }
                    match arg {
                        "Main" => state.current_band.store(0, Ordering::SeqCst),
                        "Sub" => state.current_band.store(1, Ordering::SeqCst),
                        _ => {}
                    }
                    "RPRT 0
".into()
                }
                "+\\get_vfo_info" => {
                    if state.vfo_info_switches.load(Ordering::SeqCst)
                        && let Ok(mut v) = state.vfo.lock()
                    {
                        *v = arg.to_string();
                    }
                    format!(
                        "get_vfo_info: {arg}\nVFO: {arg}\nFreq: {}\nMode: {}\nWidth: 2400\nRPRT 0\n",
                        state.freq.load(Ordering::SeqCst),
                        state.mode.lock().map(|m| m.clone()).unwrap_or_default()
                    )
                }
                "\\power2mW" => match (state.full_power_mw.load(Ordering::SeqCst), arg.parse::<f64>()) {
                    (0, _) | (_, Err(_)) => "RPRT -11\n".into(),
                    (mw, Ok(fraction)) => format!("{}\n", (mw as f64 * fraction).round() as u64),
                },
                _ => "RPRT -11\n".into(),
            }
        };
        if writer.write_all(reply.as_bytes()).is_err() {
            return;
        }
    }
}
