//! CAT per tutte le radio con Hamlib: Aurora lancia `rigctld` (il demone di Hamlib) come processo nascosto e ci parla con il suo protocollo di
//! testo su TCP locale. Hamlib conosce oltre 300 modelli; un blocco di Hamlib non butta giu' Aurora, e la radio si libera chiudendo il processo.
//! Scelta e motivi in `docs/analisi-decodium-cat.md` (Decodium collega invece la libreria: qui il processo separato e' piu' robusto).
//!
//! Protocollo (modo normale di `rigctld`): ogni comando e' una riga; le risposte sono i valori, una riga ciascuno, oppure `RPRT <codice>`
//! (0 = riuscito, negativo = errore di Hamlib).

use std::{
    io::{BufRead, BufReader, Write},
    net::{Ipv4Addr, SocketAddr, TcpListener, TcpStream},
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    sync::Mutex,
    time::{Duration, Instant},
};

use crate::RigError;

/// Come lanciare `rigctld`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RigctldConfig {
    pub exe: PathBuf,
    /// Numero del modello di Hamlib (1 = «dummy», senza radio, per le prove).
    pub model: u32,
    /// Porta seriale (`COM3`), o indirizzo per le radio di rete.
    pub port: Option<String>,
    pub baud: Option<u32>,
    /// Altre impostazioni `--set-conf` (`data_bits=8`, `stop_bits=1`, `serial_handshake=None`, `rts_state=OFF`, `ptt_type=RIG`...).
    pub conf: Vec<(String, String)>,
}

/// Gli argomenti di `rigctld` per questa configurazione, con la porta TCP su cui ascoltare. Solo valori ricavati da numeri e da nomi
/// controllati: nessun carattere che una shell tratterebbe in modo speciale (e comunque non si passa da una shell).
pub fn rigctld_args(config: &RigctldConfig, tcp_port: u16) -> Result<Vec<String>, RigError> {
    let mut args = vec!["-m".to_string(), config.model.to_string(), "-T".to_string(), "127.0.0.1".to_string(), "-t".to_string(), tcp_port.to_string()];
    if let Some(port) = &config.port {
        check_token(port, "porta")?;
        args.extend(["-r".to_string(), port.clone()]);
    }
    if let Some(baud) = config.baud {
        args.extend(["-s".to_string(), baud.to_string()]);
    }
    for (key, value) in &config.conf {
        check_token(key, "chiave")?;
        check_token(value, "valore")?;
        args.extend(["--set-conf".to_string(), format!("{key}={value}")]);
    }
    Ok(args)
}

fn check_token(text: &str, what: &str) -> Result<(), RigError> {
    let ok = !text.is_empty() && text.len() <= 64 && text.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '.' | ':' | '/' | '\\'));
    ok.then_some(()).ok_or_else(|| RigError(format!("{what} non valido per rigctld: {text:?}")))
}

/// Il codice di una risposta `RPRT n`: `Ok` se 0, errore di Hamlib altrimenti; `None` se la riga non e' un `RPRT`.
pub fn parse_rprt(line: &str) -> Option<Result<(), i32>> {
    let code: i32 = line.trim().strip_prefix("RPRT ")?.trim().parse().ok()?;
    Some(if code == 0 { Ok(()) } else { Err(code) })
}

/// Il nome dell'errore di Hamlib piu' comune, per i messaggi.
pub fn hamlib_error(code: i32) -> &'static str {
    match code {
        -1 => "parametro non valido",
        -2 => "configurazione non valida",
        -3 => "memoria esaurita",
        -4 => "funzione non implementata dalla radio",
        -5 => "timeout: la radio non risponde",
        -6 => "errore di ingresso/uscita (porta seriale)",
        -7 => "errore interno",
        -8 => "protocollo: risposta inattesa",
        -9 => "comando rifiutato dalla radio",
        -10 => "risposta troppo lunga",
        -11 => "funzione non disponibile",
        -12 => "VFO non valido",
        -13 => "argomento fuori campo",
        _ => "errore di Hamlib",
    }
}

/// Una connessione a un `rigctld` che ascolta (lanciato da noi o gia' esistente).
pub struct RigctldClient {
    io: Mutex<(BufReader<TcpStream>, TcpStream)>,
}

impl RigctldClient {
    /// Si collega a `addr`, riprovando fino a `wait` (il demone impiega un attimo ad aprire la porta).
    pub fn connect(addr: SocketAddr, wait: Duration) -> Result<RigctldClient, RigError> {
        let deadline = Instant::now() + wait;
        loop {
            match TcpStream::connect_timeout(&addr, Duration::from_millis(500)) {
                Ok(stream) => {
                    stream.set_read_timeout(Some(Duration::from_secs(5))).map_err(|e| RigError(e.to_string()))?;
                    stream.set_write_timeout(Some(Duration::from_secs(5))).map_err(|e| RigError(e.to_string()))?;
                    let _ = stream.set_nodelay(true);
                    let reader = BufReader::new(stream.try_clone().map_err(|e| RigError(e.to_string()))?);
                    return Ok(RigctldClient { io: Mutex::new((reader, stream)) });
                }
                Err(e) if Instant::now() >= deadline => return Err(RigError(format!("rigctld non risponde su {addr}: {e}"))),
                Err(_) => std::thread::sleep(Duration::from_millis(100)),
            }
        }
    }

    /// Manda un comando e legge `lines` righe di risposta (0 per i comandi che rispondono solo `RPRT`).
    fn ask(&self, command: &str, lines: usize) -> Result<Vec<String>, RigError> {
        let mut io = self.io.lock().map_err(|_| RigError("rigctld: sessione avvelenata".into()))?;
        let (reader, writer) = &mut *io;
        writer.write_all(format!("{command}\n").as_bytes()).map_err(|e| RigError(format!("rigctld: scrittura: {e}")))?;
        let mut first = String::new();
        reader.read_line(&mut first).map_err(|e| RigError(format!("rigctld: lettura: {e}")))?;
        if first.is_empty() {
            return Err(RigError("rigctld ha chiuso la connessione".into()));
        }
        if let Some(rprt) = parse_rprt(&first) {
            return match rprt {
                Ok(()) if lines == 0 => Ok(Vec::new()),
                Ok(()) => Err(RigError(format!("rigctld: risposta vuota a «{command}»"))),
                Err(code) => Err(RigError(format!("{}: {} ({code})", command.split_whitespace().next().unwrap_or(command), hamlib_error(code)))),
            };
        }
        let mut out = vec![first.trim().to_string()];
        while out.len() < lines {
            let mut line = String::new();
            reader.read_line(&mut line).map_err(|e| RigError(format!("rigctld: lettura: {e}")))?;
            out.push(line.trim().to_string());
        }
        if lines == 0 {
            // un comando che doveva rispondere solo RPRT ha risposto con altro: si riallinea il flusso
            return Err(RigError(format!("rigctld: risposta inattesa a «{command}»: {}", out[0])));
        }
        Ok(out)
    }

    /// Frequenza della radio (Hz).
    pub fn frequency(&self) -> Result<u64, RigError> {
        let r = self.ask("f", 1)?;
        r[0].parse::<f64>().map(|hz| hz.round() as u64).map_err(|_| RigError(format!("frequenza illeggibile: {}", r[0])))
    }

    pub fn set_frequency(&self, hz: u64) -> Result<(), RigError> {
        self.ask(&format!("F {hz}"), 0).map(|_| ())
    }

    /// Modo e larghezza di banda (Hz) come li da Hamlib («USB», «PKTUSB», «CW»...).
    pub fn mode(&self) -> Result<(String, u32), RigError> {
        let r = self.ask("m", 2)?;
        Ok((r[0].clone(), r[1].parse().unwrap_or(0)))
    }

    /// `passband` 0 = quella predefinita della radio.
    pub fn set_mode(&self, mode: &str, passband: u32) -> Result<(), RigError> {
        check_token(mode, "modo")?;
        self.ask(&format!("M {mode} {passband}"), 0).map(|_| ())
    }

    pub fn ptt(&self) -> Result<bool, RigError> {
        let r = self.ask("t", 1)?;
        Ok(r[0] != "0")
    }

    pub fn set_ptt(&self, on: bool) -> Result<(), RigError> {
        self.ask(&format!("T {}", u8::from(on)), 0).map(|_| ())
    }

    /// Split acceso o spento e VFO di trasmissione.
    pub fn split(&self) -> Result<(bool, String), RigError> {
        let r = self.ask("s", 2)?;
        Ok((r[0] != "0", r[1].clone()))
    }

    pub fn set_split(&self, on: bool, tx_vfo: &str) -> Result<(), RigError> {
        check_token(tx_vfo, "VFO")?;
        self.ask(&format!("S {} {tx_vfo}", u8::from(on)), 0).map(|_| ())
    }

    /// Chiede al demone di chiudere la sessione (lo lascia in vita, per altri client).
    pub fn quit(&self) {
        if let Ok(mut io) = self.io.lock() {
            let _ = io.1.write_all(b"q\n");
        }
    }
}

/// Un `rigctld` lanciato da Aurora: si ferma quando l'oggetto cade (cosi' la porta seriale si libera).
pub struct Rigctld {
    child: Child,
    pub client: RigctldClient,
    pub addr: SocketAddr,
}

impl Rigctld {
    pub fn start(config: &RigctldConfig) -> Result<Rigctld, RigError> {
        if !Path::new(&config.exe).is_file() {
            return Err(RigError(format!("rigctld non trovato: {}", config.exe.display())));
        }
        // una porta libera: si apre e si chiude subito, il demone la riprende (la gara e' improbabile e si vedrebbe come errore di connessione)
        let probe = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).map_err(|e| RigError(format!("nessuna porta libera: {e}")))?;
        let tcp_port = probe.local_addr().map_err(|e| RigError(e.to_string()))?.port();
        drop(probe);
        let args = rigctld_args(config, tcp_port)?;
        let mut command = Command::new(&config.exe);
        command.args(&args).stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null());
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            // niente finestra nera quando Aurora gira come programma a finestre
            command.creation_flags(0x0800_0000);
        }
        let mut child = command.spawn().map_err(|e| RigError(format!("rigctld non avviato: {e}")))?;
        let addr = SocketAddr::from((Ipv4Addr::LOCALHOST, tcp_port));
        match RigctldClient::connect(addr, Duration::from_secs(6)) {
            Ok(client) => Ok(Rigctld { child, client, addr }),
            Err(e) => {
                // il demone e' morto subito (modello o porta sbagliati) o non apre la porta
                let why = child.try_wait().ok().flatten().map(|s| format!(" (uscito con {s}: controlla modello, porta seriale e velocita')")).unwrap_or_default();
                let _ = child.kill();
                let _ = child.wait();
                Err(RigError(format!("{e}{why}")))
            }
        }
    }
}

impl Drop for Rigctld {
    fn drop(&mut self) {
        self.client.quit();
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        io::{BufRead, Write},
        net::TcpListener,
        sync::mpsc,
    };

    fn config() -> RigctldConfig {
        RigctldConfig { exe: "rigctld.exe".into(), model: 1049, port: Some("COM3".into()), baud: Some(38400), conf: vec![("data_bits".into(), "8".into()), ("ptt_type".into(), "RIG".into())] }
    }

    #[test]
    fn the_arguments_follow_the_configuration() {
        let args = rigctld_args(&config(), 4532).expect("argomenti");
        assert_eq!(args, ["-m", "1049", "-T", "127.0.0.1", "-t", "4532", "-r", "COM3", "-s", "38400", "--set-conf", "data_bits=8", "--set-conf", "ptt_type=RIG"]);
    }

    #[test]
    fn dangerous_values_are_refused() {
        for bad in ["COM3 & calc", "COM3;x", "a b", "", "COM3\n"] {
            let mut c = config();
            c.port = Some(bad.to_string());
            assert!(rigctld_args(&c, 1).is_err(), "{bad:?}");
        }
        let mut c = config();
        c.conf.push(("rts_state".into(), "ON; del".into()));
        assert!(rigctld_args(&c, 1).is_err());
    }

    #[test]
    fn rprt_lines_are_read() {
        assert_eq!(parse_rprt("RPRT 0\n"), Some(Ok(())));
        assert_eq!(parse_rprt("RPRT -5"), Some(Err(-5)));
        assert_eq!(parse_rprt("14074000"), None);
        assert_eq!(parse_rprt("RPRT x"), None);
        assert!(hamlib_error(-5).contains("timeout"));
    }

    /// Un finto rigctld: risponde come quello vero e ricorda i comandi ricevuti.
    fn fake_server() -> (SocketAddr, mpsc::Receiver<String>) {
        let listener = TcpListener::bind("127.0.0.1:0").expect("porta");
        let addr = listener.local_addr().expect("indirizzo");
        let (tx, rx) = mpsc::channel();
        std::thread::spawn(move || {
            let Ok((stream, _)) = listener.accept() else { return };
            let mut reader = std::io::BufReader::new(stream.try_clone().expect("clone"));
            let mut writer = stream;
            let mut line = String::new();
            while reader.read_line(&mut line).unwrap_or(0) > 0 {
                let cmd = line.trim().to_string();
                line.clear();
                let _ = tx.send(cmd.clone());
                let reply = match cmd.split_whitespace().next().unwrap_or("") {
                    "f" => "7074000\n".to_string(),
                    "m" => "PKTUSB\n3000\n".to_string(),
                    "t" => "0\n".to_string(),
                    "s" => "1\nVFOB\n".to_string(),
                    "F" if cmd.ends_with("999") => "RPRT -5\n".to_string(),
                    "F" | "M" | "T" | "S" => "RPRT 0\n".to_string(),
                    "q" => return,
                    _ => "RPRT -11\n".to_string(),
                };
                let _ = writer.write_all(reply.as_bytes());
            }
        });
        (addr, rx)
    }

    #[test]
    fn the_client_speaks_the_rigctld_protocol() {
        let (addr, seen) = fake_server();
        let client = RigctldClient::connect(addr, Duration::from_secs(2)).expect("connessione");
        assert_eq!(client.frequency().expect("f"), 7_074_000);
        client.set_frequency(14_074_000).expect("F");
        assert_eq!(client.mode().expect("m"), ("PKTUSB".to_string(), 3000));
        client.set_mode("PKTUSB", 0).expect("M");
        assert!(!client.ptt().expect("t"));
        client.set_ptt(true).expect("T");
        assert_eq!(client.split().expect("s"), (true, "VFOB".to_string()));
        client.set_split(false, "VFOA").expect("S");
        let commands: Vec<String> = seen.try_iter().collect();
        assert_eq!(commands, ["f", "F 14074000", "m", "M PKTUSB 0", "t", "T 1", "s", "S 0 VFOA"]);
    }

    #[test]
    fn hamlib_errors_are_reported_in_words() {
        let (addr, _seen) = fake_server();
        let client = RigctldClient::connect(addr, Duration::from_secs(2)).expect("connessione");
        let err = client.set_frequency(999).expect_err("doveva fallire");
        assert!(err.0.contains("timeout"), "{err}");
        let err = client.set_mode("PKT USB", 0).expect_err("modo non valido");
        assert!(err.0.contains("modo"), "{err}");
    }

    #[test]
    fn a_missing_daemon_is_an_error_not_a_hang() {
        let mut c = config();
        c.exe = "C:/non/esiste/rigctld.exe".into();
        assert!(Rigctld::start(&c).is_err());
        let closed = TcpListener::bind("127.0.0.1:0").expect("porta").local_addr().expect("indirizzo");
        assert!(RigctldClient::connect(closed, Duration::from_millis(300)).is_err());
    }

    /// Con un rigctld vero e il modello «dummy» (nessuna radio): frequenza, modo e PTT tornano come scritti. Si salta se non c'e' rigctld.
    #[test]
    fn a_real_dummy_rig_round_trips() {
        let exe = ["C:/JTDX64/159/bin/rigctld-jtdx.exe", "C:/WSJT/wsjtx/bin/rigctld-wsjtx.exe"].into_iter().map(PathBuf::from).find(|p| p.is_file());
        let Some(exe) = exe else { return };
        let rig = Rigctld::start(&RigctldConfig { exe, model: 1, port: None, baud: None, conf: Vec::new() }).expect("rigctld dummy");
        rig.client.set_frequency(14_074_000).expect("F");
        assert_eq!(rig.client.frequency().expect("f"), 14_074_000);
        rig.client.set_mode("USB", 0).expect("M");
        assert_eq!(rig.client.mode().expect("m").0, "USB");
        rig.client.set_ptt(true).expect("T 1");
        assert!(rig.client.ptt().expect("t"));
        rig.client.set_ptt(false).expect("T 0");
        assert!(!rig.client.ptt().expect("t"));
    }
}
