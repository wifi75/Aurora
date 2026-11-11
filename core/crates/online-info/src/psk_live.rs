//! PSK Reporter in tempo reale: chi riceve il proprio nominativo, dal flusso MQTT pubblico `mqtt.pskreporter.info` (senza account), lo stesso che usa
//! GridTracker. La richiesta `retrieve.pskreporter.info` si puo' fare al massimo ogni 5 minuti, e dopo aver chiamato i primi rapporti arrivavano con
//! quel ritardo (segnalazione #39); dal flusso arrivano in pochi secondi.
//!
//! Il topic e' `pskr/filter/v2/{banda}/{modo}/{chi trasmette}/{chi riceve}/{locatore tx}/{locatore rx}/{dxcc tx}/{dxcc rx}`: ci si abbona a
//! `pskr/filter/v2/+/+/NOMINATIVO/#`, cioe' alle ricezioni dei propri segnali. Dati pubblici (stessi della mappa di pskreporter.info), porta 1883 senza TLS
//! (la 1884 con TLS richiederebbe una libreria in piu': nel topic c'e' solo il nominativo, che il sito conosce gia' dalla richiesta HTTP).
//!
//! Un piccolo client MQTT 3.1.1 (CONNECT, SUBSCRIBE, PUBLISH a QoS 0, PINGREQ) scritto qui: non serve altro.

use std::collections::VecDeque;
use std::io::{self, Read, Write};
use std::net::{TcpStream, ToSocketAddrs};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use crate::psk::Report;

pub const HOST: &str = "mqtt.pskreporter.info";
pub const PORT: u16 = 1883;
/// Quanti rapporti arrivati dal flusso si tengono.
pub const MAX_LIVE: usize = 500;
const KEEPALIVE_S: u16 = 60;

/// I rapporti arrivati dal flusso, i piu' recenti per primi.
pub type Live = Arc<Mutex<VecDeque<Report>>>;

/// Il flusso in corso per un nominativo: si ferma da solo quando `stop` diventa vero.
pub struct Feed {
    pub callsign: String,
    pub reports: Live,
    stop: Arc<AtomicBool>,
}

impl Feed {
    /// Avvia il flusso in un thread suo (ricollega da solo se cade). Il nominativo deve essere gia' ripulito (`psk::clean_callsign`).
    pub fn start(callsign: &str) -> Feed {
        let feed = Feed { callsign: callsign.to_string(), reports: Arc::new(Mutex::new(VecDeque::new())), stop: Arc::new(AtomicBool::new(false)) };
        let (call, reports, stop) = (feed.callsign.clone(), Arc::clone(&feed.reports), Arc::clone(&feed.stop));
        let _ = std::thread::Builder::new().name("aurora-psk-live".into()).spawn(move || run(&call, &reports, &stop));
        feed
    }

    pub fn stop(&self) {
        self.stop.store(true, Ordering::SeqCst);
    }
}

impl Drop for Feed {
    fn drop(&mut self) {
        self.stop();
    }
}

fn run(call: &str, reports: &Live, stop: &AtomicBool) {
    let mut wait = Duration::from_secs(5);
    while !stop.load(Ordering::SeqCst) {
        let started = Instant::now();
        match session(call, reports, stop) {
            Ok(()) => return,
            Err(e) => tracing::info!("PSK Reporter in tempo reale: {e}; riprovo tra {} s", wait.as_secs()),
        }
        // una connessione durata a lungo non e' un guasto: si riparte dall'attesa breve
        wait = if started.elapsed() > Duration::from_secs(120) { Duration::from_secs(5) } else { (wait * 2).min(Duration::from_secs(120)) };
        let until = Instant::now() + wait;
        while Instant::now() < until && !stop.load(Ordering::SeqCst) {
            std::thread::sleep(Duration::from_millis(500));
        }
    }
}

fn session(call: &str, reports: &Live, stop: &AtomicBool) -> Result<(), String> {
    let addr = (HOST, PORT).to_socket_addrs().map_err(|e| format!("indirizzo non risolto: {e}"))?.next().ok_or("nessun indirizzo")?;
    let mut stream = TcpStream::connect_timeout(&addr, Duration::from_secs(10)).map_err(|e| format!("collegamento non riuscito: {e}"))?;
    let _ = stream.set_nodelay(true);
    stream.set_write_timeout(Some(Duration::from_secs(10))).map_err(|e| e.to_string())?;
    stream.set_read_timeout(Some(Duration::from_secs(10))).map_err(|e| e.to_string())?;
    let id = format!("aurora-{:x}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_nanos() & 0xffff_ffff));
    stream.write_all(&connect_packet(&id)).map_err(|e| e.to_string())?;
    let (kind, body) = read_packet(&mut stream).map_err(|e| format!("niente risposta al collegamento: {e}"))?;
    if kind >> 4 != 2 || body.len() < 2 || body[1] != 0 {
        return Err("il server ha rifiutato il collegamento".into());
    }
    stream.write_all(&subscribe_packet(1, &format!("pskr/filter/v2/+/+/{call}/#"))).map_err(|e| e.to_string())?;
    tracing::info!("PSK Reporter in tempo reale: abbonato ai rapporti su {call}");
    let mut last_ping = Instant::now();
    while !stop.load(Ordering::SeqCst) {
        // attesa breve del primo byte: serve a controllare `stop` e a mandare il PINGREQ; il resto del pacchetto ha piu' tempo
        stream.set_read_timeout(Some(Duration::from_secs(5))).map_err(|e| e.to_string())?;
        let mut first = [0u8; 1];
        match stream.read(&mut first) {
            Ok(0) => return Err("il server ha chiuso il collegamento".into()),
            Ok(_) => {
                stream.set_read_timeout(Some(Duration::from_secs(15))).map_err(|e| e.to_string())?;
                let body = read_body(&mut stream).map_err(|e| format!("pacchetto interrotto: {e}"))?;
                if first[0] >> 4 == 3
                    && let Some(report) = parse_publish(first[0], &body)
                {
                    push(reports, report);
                }
            }
            Err(e) if matches!(e.kind(), io::ErrorKind::WouldBlock | io::ErrorKind::TimedOut) => {}
            Err(e) => return Err(format!("lettura: {e}")),
        }
        if last_ping.elapsed() >= Duration::from_secs(u64::from(KEEPALIVE_S) / 2) {
            stream.write_all(&[0xC0, 0x00]).map_err(|e| e.to_string())?;
            last_ping = Instant::now();
        }
    }
    let _ = stream.write_all(&[0xE0, 0x00]);
    Ok(())
}

/// Mette un rapporto in testa, senza doppioni (stesso ricevente, modo e minuto), e tiene al massimo [`MAX_LIVE`] rapporti.
pub fn push(reports: &Live, report: Report) {
    let mut list = reports.lock().unwrap_or_else(|p| p.into_inner());
    if list.iter().any(|r| same(r, &report)) {
        return;
    }
    list.push_front(report);
    list.truncate(MAX_LIVE);
}

/// Lo stesso rapporto (anche se arriva da due fonti): stesso ricevente, modo e minuto.
pub fn same(a: &Report, b: &Report) -> bool {
    a.receiver == b.receiver && a.mode.eq_ignore_ascii_case(&b.mode) && a.time_s / 60 == b.time_s / 60 && a.freq_hz / 5_000 == b.freq_hz / 5_000
}

fn put_str(out: &mut Vec<u8>, s: &str) {
    out.extend_from_slice(&(s.len() as u16).to_be_bytes());
    out.extend_from_slice(s.as_bytes());
}

fn put_len(out: &mut Vec<u8>, mut n: usize) {
    loop {
        let mut byte = (n % 128) as u8;
        n /= 128;
        if n > 0 {
            byte |= 0x80;
        }
        out.push(byte);
        if n == 0 {
            break;
        }
    }
}

pub fn connect_packet(client_id: &str) -> Vec<u8> {
    let mut body = Vec::new();
    put_str(&mut body, "MQTT");
    body.push(4); // MQTT 3.1.1
    body.push(0x02); // sessione pulita, senza nome ne' parola d'ordine
    body.extend_from_slice(&KEEPALIVE_S.to_be_bytes());
    put_str(&mut body, client_id);
    let mut out = vec![0x10];
    put_len(&mut out, body.len());
    out.extend(body);
    out
}

pub fn subscribe_packet(id: u16, filter: &str) -> Vec<u8> {
    let mut body = Vec::new();
    body.extend_from_slice(&id.to_be_bytes());
    put_str(&mut body, filter);
    body.push(0); // QoS 0
    let mut out = vec![0x82];
    put_len(&mut out, body.len());
    out.extend(body);
    out
}

/// Legge il resto di un pacchetto dopo il primo byte: la lunghezza (da 1 a 4 byte) e il contenuto.
fn read_body(r: &mut impl Read) -> io::Result<Vec<u8>> {
    let (mut len, mut mult) = (0usize, 1usize);
    for _ in 0..4 {
        let mut b = [0u8; 1];
        r.read_exact(&mut b)?;
        len += usize::from(b[0] & 0x7f) * mult;
        if b[0] & 0x80 == 0 {
            if len > 1 << 20 {
                return Err(io::Error::new(io::ErrorKind::InvalidData, "pacchetto troppo grande"));
            }
            let mut body = vec![0u8; len];
            r.read_exact(&mut body)?;
            return Ok(body);
        }
        mult *= 128;
    }
    Err(io::Error::new(io::ErrorKind::InvalidData, "lunghezza non valida"))
}

fn read_packet(r: &mut impl Read) -> io::Result<(u8, Vec<u8>)> {
    let mut first = [0u8; 1];
    r.read_exact(&mut first)?;
    Ok((first[0], read_body(r)?))
}

/// Un PUBLISH a QoS 0 (o piu': l'identificativo si salta) con il JSON di PSK Reporter: `sc` chi trasmette, `rc`/`rl` chi riceve e dove, `f` frequenza,
/// `md` modo, `rp` SNR, `t` quando (secondi Unix).
pub fn parse_publish(first: u8, body: &[u8]) -> Option<Report> {
    let topic_len = usize::from(u16::from_be_bytes([*body.first()?, *body.get(1)?]));
    let qos = (first >> 1) & 3;
    let payload_at = 2 + topic_len + if qos > 0 { 2 } else { 0 };
    let value: serde_json::Value = serde_json::from_slice(body.get(payload_at..)?).ok()?;
    let receiver = value.get("rc")?.as_str().filter(|c| !c.is_empty())?.to_uppercase();
    Some(Report {
        receiver,
        locator: value.get("rl").and_then(|v| v.as_str()).unwrap_or_default().to_uppercase(),
        snr: value.get("rp").and_then(serde_json::Value::as_i64).and_then(|v| i32::try_from(v).ok()),
        freq_hz: value.get("f").and_then(serde_json::Value::as_u64)?,
        mode: value.get("md").and_then(|v| v.as_str()).unwrap_or_default().to_string(),
        time_s: value.get("t").and_then(serde_json::Value::as_u64)?,
        country: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn publish(topic: &str, json: &str) -> (u8, Vec<u8>) {
        let mut body = Vec::new();
        put_str(&mut body, topic);
        body.extend_from_slice(json.as_bytes());
        (0x30, body)
    }

    #[test]
    fn a_published_spot_becomes_a_report() {
        let (first, body) = publish("pskr/filter/v2/20m/FT8/IU3ABC/K1ABC/JN45/FN42/248/291", r#"{"sq":1,"f":14075500,"md":"FT8","rp":-12,"t":1792864254,"sc":"IU3ABC","sl":"JN45","rc":"k1abc","rl":"fn42","sa":248,"ra":291,"b":"20m"}"#);
        let r = parse_publish(first, &body).expect("rapporto");
        assert_eq!((r.receiver.as_str(), r.locator.as_str(), r.snr, r.freq_hz, r.mode.as_str(), r.time_s), ("K1ABC", "FN42", Some(-12), 14_075_500, "FT8", 1_792_864_254));
    }

    #[test]
    fn spots_without_receiver_frequency_or_time_are_dropped() {
        assert!(parse_publish(0x30, &publish("t", r#"{"f":1,"t":1}"#).1).is_none());
        assert!(parse_publish(0x30, &publish("t", r#"{"rc":"K1ABC","t":1}"#).1).is_none());
        assert!(parse_publish(0x30, &publish("t", r#"{"rc":"K1ABC","f":1}"#).1).is_none());
        assert!(parse_publish(0x30, b"\x00").is_none());
    }

    #[test]
    fn packets_have_the_mqtt_311_shape() {
        let c = connect_packet("x");
        assert_eq!(&c[..2], &[0x10, 13]);
        assert_eq!(&c[2..8], &[0, 4, b'M', b'Q', b'T', b'T']);
        assert_eq!(&c[8..10], &[4, 0x02]);
        let s = subscribe_packet(1, "a/#");
        assert_eq!(s, vec![0x82, 8, 0, 1, 0, 3, b'a', b'/', b'#', 0]);
    }

    #[test]
    fn lengths_use_the_variable_encoding() {
        let mut out = Vec::new();
        put_len(&mut out, 321);
        assert_eq!(out, vec![0xC1, 0x02]);
        assert_eq!(read_body(&mut io::Cursor::new(vec![0x03, 1, 2, 3])).expect("corpo"), vec![1, 2, 3]);
        assert!(read_body(&mut io::Cursor::new(vec![0xff, 0xff, 0xff, 0xff])).is_err());
    }

    #[test]
    fn the_same_spot_from_two_sources_is_kept_once() {
        let live: Live = Arc::new(Mutex::new(VecDeque::new()));
        let r = |t| Report { receiver: "K1ABC".into(), locator: "FN42".into(), snr: Some(-5), freq_hz: 14_075_500, mode: "FT8".into(), time_s: t, country: None };
        push(&live, r(1_000));
        push(&live, r(1_010));
        push(&live, r(1_200));
        assert_eq!(live.lock().expect("elenco").len(), 2);
    }

    /// Prova col server vero (`cargo test -p online-info -- --ignored`): ascolta 3 rapporti qualsiasi del flusso pubblico, senza nessun nominativo.
    #[test]
    #[ignore = "usa la rete"]
    fn the_real_broker_answers_and_sends_spots() {
        let addr = (HOST, PORT).to_socket_addrs().expect("indirizzo").next().expect("indirizzo");
        let mut s = TcpStream::connect_timeout(&addr, Duration::from_secs(10)).expect("collegamento");
        s.set_read_timeout(Some(Duration::from_secs(20))).expect("timeout");
        s.write_all(&connect_packet("aurora-prova")).expect("connect");
        let (kind, body) = read_packet(&mut s).expect("connack");
        assert_eq!((kind >> 4, body.get(1).copied()), (2, Some(0)), "CONNACK");
        s.write_all(&subscribe_packet(1, "pskr/filter/v2/20m/FT8/#")).expect("subscribe");
        let mut spots = 0;
        let deadline = Instant::now() + Duration::from_secs(60);
        while spots < 3 && Instant::now() < deadline {
            let (kind, body) = read_packet(&mut s).expect("pacchetto");
            if kind >> 4 == 3 {
                let report = parse_publish(kind, &body).expect("rapporto leggibile");
                println!("{} {} Hz {} {:?} dB alle {}", report.receiver, report.freq_hz, report.mode, report.snr, report.time_s);
                spots += 1;
            }
        }
        assert_eq!(spots, 3);
    }
}
