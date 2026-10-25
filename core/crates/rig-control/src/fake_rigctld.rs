//! Un finto `rigctld` per le prove (di questo crate e di chi lo usa): risponde come quello vero, ricorda i comandi ricevuti e si puo'
//! far guastare (timeout, rifiuti, connessioni chiuse). Non serve nessuna radio ne' nessun eseguibile.

use std::{
    io::{BufRead, BufReader, Write},
    net::{SocketAddr, TcpListener, TcpStream},
    sync::{
        atomic::{AtomicBool, AtomicI32, AtomicU64, Ordering},
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

    pub fn mode(&self) -> String {
        self.state.mode.lock().map(|m| m.clone()).unwrap_or_default()
    }

    pub fn ptt(&self) -> bool {
        self.state.ptt.load(Ordering::SeqCst)
    }

    pub fn split(&self) -> bool {
        self.state.split.load(Ordering::SeqCst)
    }

    /// Risponde `RPRT code` a ogni comando (-5 = timeout della radio, -9 = comando rifiutato); 0 toglie il guasto.
    pub fn fail_with(&self, code: i32) {
        self.state.fail_code.store(code, Ordering::SeqCst);
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

    /// Il fondo scala della potenza (mW) per `\power2mW`.
    pub fn set_full_power_mw(&self, mw: u64) {
        self.state.full_power_mw.store(mw, Ordering::SeqCst);
    }

    /// Quante volte e' arrivato questo comando (testo esatto).
    pub fn count(&self, command: &str) -> usize {
        self.commands().iter().filter(|c| *c == command).count()
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
        let fail = state.fail_code.load(Ordering::SeqCst);
        let mut parts = cmd.split_whitespace();
        let head = parts.next().unwrap_or("");
        let arg = parts.next().unwrap_or("");
        let reply = if fail != 0 {
            format!("RPRT {fail}\n")
        } else {
            match head {
                "f" => format!("{}\n", state.freq.load(Ordering::SeqCst)),
                "m" => format!("{}\n2400\n", state.mode.lock().map(|m| m.clone()).unwrap_or_default()),
                "t" => format!("{}\n", u8::from(state.ptt.load(Ordering::SeqCst))),
                "s" => format!("{}\nVFOB\n", u8::from(state.split.load(Ordering::SeqCst))),
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
                    if let Ok(mut m) = state.mode.lock() {
                        *m = arg.to_string();
                    }
                    "RPRT 0\n".into()
                }
                "T" => {
                    if arg == "0" && state.fail_ptt_off.load(Ordering::SeqCst) {
                        "RPRT -5\n".into()
                    } else {
                        state.ptt.store(arg == "1", Ordering::SeqCst);
                        "RPRT 0\n".into()
                    }
                }
                "S" => {
                    state.split.store(arg == "1", Ordering::SeqCst);
                    "RPRT 0\n".into()
                }
                "I" => "RPRT 0\n".into(),
                "l" => {
                    let value = state.levels.lock().ok().and_then(|l| l.iter().find(|(n, _)| n == arg).map(|(_, v)| v.clone()));
                    value.map_or_else(|| "RPRT -11\n".to_string(), |v| format!("{v}\n"))
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
