//! Il flusso I/Q di un SDR via TCI (ExpertSDR, Thetis, SunSDR), su un collegamento WebSocket suo, separato da quello del CAT: i server TCI
//! accettano piu' client. Protocollo letto da WSJT-X 3 (`TCITransceiver.cpp`: `iq_samplerate`, `iq_start`, `iq_stop`; blocchi binari
//! con l'intestazione di 64 byte e il tipo 0 = `Iq_Stream`, campioni I e Q alternati). Il centro del flusso arriva con `dds:<rx>,<hz>;`.

use std::io::ErrorKind;
use std::net::{TcpStream, ToSocketAddrs};
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use rig_control::tci::{HEADER_SIZE, StreamHeader, StreamType};
use rustfft::num_complex::Complex;
use tungstenite::{Message, WebSocket};

/// Frequenze di campionamento I/Q dei server TCI.
pub const IQ_RATES: [u32; 4] = [48_000, 96_000, 192_000, 384_000];
/// Valori float al massimo in un blocco I/Q accettato.
const MAX_IQ_FLOATS: usize = 1 << 20;

/// `iq_samplerate:<hz>;`
pub fn cmd_iq_samplerate(hz: u32) -> String {
    format!("iq_samplerate:{hz};")
}

/// `iq_start:<rx>;`
pub fn cmd_iq_start(rx: u32) -> String {
    format!("iq_start:{rx};")
}

/// `iq_stop:<rx>;`
pub fn cmd_iq_stop(rx: u32) -> String {
    format!("iq_stop:{rx};")
}

/// I comandi di testo che servono al flusso I/Q.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IqEvent {
    Ready,
    /// Centro del ricevitore (`dds:<rx>,<hz>;`).
    Dds { rx: u32, hz: u64 },
    SampleRate(u32),
    Start { rx: u32 },
    Stop { rx: u32 },
}

/// Legge i comandi di testo (piu' comandi separati da `;`, maiuscole o minuscole).
pub fn parse_iq_text(text: &str) -> Vec<IqEvent> {
    let mut out = Vec::new();
    for cmd in text.split(';') {
        let cmd = cmd.trim();
        let (name, args) = cmd.split_once(':').unwrap_or((cmd, ""));
        let args: Vec<&str> = args.split(',').map(str::trim).collect();
        let num = |i: usize| args.get(i).and_then(|a| a.parse::<u64>().ok());
        match name.to_ascii_lowercase().as_str() {
            "ready" => out.push(IqEvent::Ready),
            "dds" => {
                if let (Some(rx), Some(hz)) = (num(0), num(1)) {
                    out.push(IqEvent::Dds { rx: rx as u32, hz });
                }
            }
            "iq_samplerate" => {
                if let Some(hz) = num(0) {
                    out.push(IqEvent::SampleRate(hz as u32));
                }
            }
            "iq_start" => out.push(IqEvent::Start { rx: num(0).unwrap_or(0) as u32 }),
            "iq_stop" => out.push(IqEvent::Stop { rx: num(0).unwrap_or(0) as u32 }),
            _ => {}
        }
    }
    out
}

/// I campioni di un blocco I/Q del ricevitore `rx`; `None` se il blocco e' d'altro tipo, d'altro ricevitore o incompleto.
pub fn iq_payload(frame: &[u8], rx: u32) -> Option<(StreamHeader, Vec<Complex<f32>>)> {
    let header = StreamHeader::parse(frame)?;
    if header.kind != StreamType::Iq || header.receiver != rx {
        return None;
    }
    let count = header.length as usize;
    if count > MAX_IQ_FLOATS {
        return None;
    }
    let body = &frame[HEADER_SIZE..];
    // formati del protocollo: 0 int16, 1 int24, 2 int32, 3 float32
    let size = match header.format {
        0 => 2,
        1 => 3,
        2 | 3 => 4,
        _ => return None,
    };
    if body.len() < count * size {
        return None;
    }
    let value = |i: usize| -> f32 {
        let c = &body[i * size..(i + 1) * size];
        match header.format {
            0 => i16::from_le_bytes([c[0], c[1]]) as f32 / 32768.0,
            1 => (i32::from_le_bytes([0, c[0], c[1], c[2]]) >> 8) as f32 / 8_388_608.0,
            2 => i32::from_le_bytes([c[0], c[1], c[2], c[3]]) as f32 / 2_147_483_648.0,
            _ => f32::from_le_bytes([c[0], c[1], c[2], c[3]]),
        }
    };
    Some((header, (0..count / 2).map(|k| Complex::new(value(2 * k), value(2 * k + 1))).collect()))
}

/// Chi riceve i campioni: blocco, frequenza di campionamento, centro (se noto).
pub type IqFn = Arc<dyn Fn(&[Complex<f32>], u32, Option<u64>) + Send + Sync>;

/// Stato del collegamento I/Q.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct IqLinkStatus {
    pub connected: bool,
    pub streaming: bool,
    pub message: String,
}

struct Shared {
    address: String,
    rx: u32,
    rate: u32,
    stop: AtomicBool,
    streaming: AtomicBool,
    center: AtomicU64,
    actual_rate: AtomicU32,
    status: Mutex<IqLinkStatus>,
    sink: IqFn,
}

impl Shared {
    fn set_status(&self, connected: bool, message: String) {
        if let Ok(mut s) = self.status.lock() {
            s.connected = connected;
            s.streaming = self.streaming.load(Ordering::SeqCst);
            s.message = message;
        }
    }
}

/// Il flusso I/Q dal vivo: un thread che si collega, chiede il flusso e si ricollega da solo.
pub struct TciIqStream {
    shared: Arc<Shared>,
    worker: Mutex<Option<JoinHandle<()>>>,
}

impl TciIqStream {
    pub fn start(address: &str, rx: u32, rate: u32, sink: IqFn) -> TciIqStream {
        let address = if address.trim().is_empty() { rig_control::tci::DEFAULT_TCI_ADDRESS.to_string() } else { address.trim().to_string() };
        let rate = if IQ_RATES.contains(&rate) { rate } else { 96_000 };
        let shared = Arc::new(Shared {
            address,
            rx,
            rate,
            stop: AtomicBool::new(false),
            streaming: AtomicBool::new(false),
            center: AtomicU64::new(0),
            actual_rate: AtomicU32::new(rate),
            status: Mutex::new(IqLinkStatus::default()),
            sink,
        });
        let s = Arc::clone(&shared);
        let worker = std::thread::Builder::new().name("tci-iq".into()).spawn(move || run(&s)).ok();
        TciIqStream { shared, worker: Mutex::new(worker) }
    }

    pub fn status(&self) -> IqLinkStatus {
        self.shared.status.lock().map(|s| s.clone()).unwrap_or_default()
    }

    pub fn center_hz(&self) -> Option<u64> {
        Some(self.shared.center.load(Ordering::SeqCst)).filter(|&c| c > 0)
    }

    pub fn shutdown(&self) {
        self.shared.stop.store(true, Ordering::SeqCst);
        if let Some(h) = self.worker.lock().ok().and_then(|mut w| w.take()) {
            let _ = h.join();
        }
    }
}

impl Drop for TciIqStream {
    fn drop(&mut self) {
        self.shutdown();
    }
}

fn open(address: &str) -> Result<WebSocket<TcpStream>, String> {
    let addr = address.to_socket_addrs().map_err(|e| format!("{address}: {e}"))?.next().ok_or_else(|| format!("{address}: indirizzo non valido"))?;
    let stream = TcpStream::connect_timeout(&addr, Duration::from_secs(3)).map_err(|e| format!("{address}: {e}"))?;
    stream.set_read_timeout(Some(Duration::from_secs(3))).map_err(|e| e.to_string())?;
    let _ = stream.set_nodelay(true);
    let (ws, _) = tungstenite::client(format!("ws://{address}/"), stream).map_err(|e| format!("{address}: {e}"))?;
    ws.get_ref().set_read_timeout(Some(Duration::from_millis(50))).map_err(|e| e.to_string())?;
    Ok(ws)
}

fn run(s: &Shared) {
    let mut attempt = 0u32;
    while !s.stop.load(Ordering::SeqCst) {
        match open(&s.address) {
            Ok(mut ws) => {
                attempt = 0;
                s.set_status(true, String::new());
                let reason = serve(s, &mut ws);
                if s.streaming.load(Ordering::SeqCst) {
                    let _ = ws.send(Message::text(cmd_iq_stop(s.rx)));
                }
                let _ = ws.close(None);
                let _ = ws.flush();
                s.streaming.store(false, Ordering::SeqCst);
                if s.stop.load(Ordering::SeqCst) {
                    break;
                }
                s.set_status(false, format!("Flusso I/Q perso ({reason}): nuovo tentativo"));
            }
            Err(e) => {
                attempt += 1;
                s.set_status(false, format!("Server TCI non raggiungibile ({e})"));
            }
        }
        let until = Instant::now() + Duration::from_millis((500 * u64::from(attempt.clamp(1, 10))).min(5000));
        while Instant::now() < until && !s.stop.load(Ordering::SeqCst) {
            std::thread::sleep(Duration::from_millis(20));
        }
    }
    s.set_status(false, String::new());
}

fn serve(s: &Shared, ws: &mut WebSocket<TcpStream>) -> String {
    loop {
        if s.stop.load(Ordering::SeqCst) {
            return "chiuso".into();
        }
        match ws.read() {
            Ok(Message::Text(text)) => {
                for ev in parse_iq_text(text.as_str()) {
                    match ev {
                        IqEvent::Ready => {
                            for cmd in [cmd_iq_samplerate(s.rate), cmd_iq_start(s.rx)] {
                                if let Err(e) = ws.send(Message::text(cmd)) {
                                    return e.to_string();
                                }
                            }
                        }
                        IqEvent::Dds { rx, hz } if rx == s.rx => s.center.store(hz, Ordering::SeqCst),
                        IqEvent::SampleRate(hz) if hz > 0 => s.actual_rate.store(hz, Ordering::SeqCst),
                        IqEvent::Start { rx } if rx == s.rx => {
                            s.streaming.store(true, Ordering::SeqCst);
                            s.set_status(true, String::new());
                        }
                        IqEvent::Stop { rx } if rx == s.rx => {
                            s.streaming.store(false, Ordering::SeqCst);
                            s.set_status(true, String::new());
                        }
                        _ => {}
                    }
                }
            }
            Ok(Message::Binary(data)) => {
                if let Some((header, samples)) = iq_payload(&data, s.rx) {
                    if !s.streaming.swap(true, Ordering::SeqCst) {
                        s.set_status(true, String::new());
                    }
                    let rate = if header.sample_rate > 0 { header.sample_rate } else { s.actual_rate.load(Ordering::SeqCst) };
                    let center = Some(s.center.load(Ordering::SeqCst)).filter(|&c| c > 0);
                    (s.sink)(&samples, rate, center);
                }
            }
            Ok(Message::Close(_)) => return "chiuso dal server".into(),
            Ok(_) => {}
            Err(tungstenite::Error::Io(e)) if matches!(e.kind(), ErrorKind::WouldBlock | ErrorKind::TimedOut) => {
                if let Err(tungstenite::Error::Io(e)) = ws.flush()
                    && !matches!(e.kind(), ErrorKind::WouldBlock | ErrorKind::TimedOut)
                {
                    return e.to_string();
                }
            }
            Err(e) => return e.to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_commands() {
        assert_eq!(cmd_iq_samplerate(96_000), "iq_samplerate:96000;");
        assert_eq!(cmd_iq_start(0), "iq_start:0;");
        assert_eq!(cmd_iq_stop(1), "iq_stop:1;");
        assert_eq!(
            parse_iq_text("ready;DDS:0,144100000;iq_samplerate:192000;iq_start:0;vfo:0,0,1;"),
            vec![IqEvent::Ready, IqEvent::Dds { rx: 0, hz: 144_100_000 }, IqEvent::SampleRate(192_000), IqEvent::Start { rx: 0 }]
        );
    }

    fn frame(rx: u32, kind: StreamType, floats: &[f32]) -> Vec<u8> {
        let h = StreamHeader { receiver: rx, sample_rate: 96_000, format: 3, codec: 0, crc: 0, length: floats.len() as u32, kind, channels: 2 };
        let mut out = h.to_bytes().to_vec();
        for f in floats {
            out.extend_from_slice(&f.to_le_bytes());
        }
        out
    }

    #[test]
    fn iq_frames_are_read() {
        let (h, iq) = iq_payload(&frame(0, StreamType::Iq, &[0.5, -0.5, 0.25, 1.0]), 0).expect("blocco");
        assert_eq!(h.sample_rate, 96_000);
        assert_eq!(iq, vec![Complex::new(0.5, -0.5), Complex::new(0.25, 1.0)]);
        assert!(iq_payload(&frame(1, StreamType::Iq, &[0.0, 0.0]), 0).is_none());
        assert!(iq_payload(&frame(0, StreamType::RxAudio, &[0.0, 0.0]), 0).is_none());
        let mut short = frame(0, StreamType::Iq, &[0.0, 0.0]);
        short.truncate(HEADER_SIZE + 4);
        assert!(iq_payload(&short, 0).is_none());
    }

    /// Server TCI finto: risponde `ready;` e `dds:`, poi manda un blocco I/Q dopo `iq_start`.
    #[test]
    fn streams_from_a_fake_server() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("porta");
        let address = listener.local_addr().expect("indirizzo").to_string();
        let server = std::thread::spawn(move || {
            let (stream, _) = listener.accept().expect("client");
            let mut ws = tungstenite::accept(stream).expect("ws");
            ws.send(Message::text("protocol:ExpertSDR3,1.9;dds:0,50313000;ready;")).expect("invio");
            let mut got = Vec::new();
            while got.len() < 2 {
                if let Ok(Message::Text(t)) = ws.read() {
                    got.push(t.to_string());
                }
            }
            ws.send(Message::text("iq_start:0;")).expect("invio");
            ws.send(Message::binary(frame(0, StreamType::Iq, &[0.1, 0.2, 0.3, 0.4]))).expect("invio");
            // resta aperto finche' il client chiude
            while ws.read().is_ok() {}
            got
        });
        let seen: Arc<Mutex<Vec<(usize, u32, Option<u64>)>>> = Arc::default();
        let s2 = Arc::clone(&seen);
        let stream = TciIqStream::start(&address, 0, 96_000, Arc::new(move |iq, rate, center| s2.lock().expect("lock").push((iq.len(), rate, center))));
        let until = Instant::now() + Duration::from_secs(5);
        while seen.lock().expect("lock").is_empty() && Instant::now() < until {
            std::thread::sleep(Duration::from_millis(10));
        }
        assert_eq!(seen.lock().expect("lock").first(), Some(&(2, 96_000, Some(50_313_000))));
        assert!(stream.status().streaming);
        stream.shutdown();
        let got = server.join().expect("server");
        assert_eq!(got, vec!["iq_samplerate:96000;".to_string(), "iq_start:0;".to_string()]);
    }
}
