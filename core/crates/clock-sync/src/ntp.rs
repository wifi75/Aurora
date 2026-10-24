//! Protocollo SNTP (RFC 4330): una richiesta UDP da 48 byte e il calcolo dello scarto dell'orologio.

use std::{
    net::{ToSocketAddrs, UdpSocket},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

/// Secondi tra l'epoca NTP (1900) e quella Unix (1970).
const NTP_TO_UNIX: f64 = 2_208_988_800.0;
const PACKET: usize = 48;

/// Esito di una misura: quanto l'orologio locale e' indietro (+) o avanti (-) rispetto al server.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Sample {
    pub offset_ms: f64,
    pub delay_ms: f64,
    pub stratum: u8,
}

fn now_unix() -> f64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map_or(0.0, |d| d.as_secs_f64())
}

fn to_bytes(unix_secs: f64) -> [u8; 8] {
    let ntp = unix_secs + NTP_TO_UNIX;
    let secs = ntp.trunc();
    let frac = ((ntp - secs) * 4_294_967_296.0) as u32;
    let mut out = [0u8; 8];
    out[..4].copy_from_slice(&(secs as u32).to_be_bytes());
    out[4..].copy_from_slice(&frac.to_be_bytes());
    out
}

fn from_bytes(b: &[u8]) -> f64 {
    let secs = u32::from_be_bytes([b[0], b[1], b[2], b[3]]);
    let frac = u32::from_be_bytes([b[4], b[5], b[6], b[7]]);
    f64::from(secs) + f64::from(frac) / 4_294_967_296.0 - NTP_TO_UNIX
}

/// La richiesta (modo client, versione 4) con l'ora di invio nel campo di trasmissione.
pub fn build_request(t0_unix: f64) -> [u8; PACKET] {
    let mut p = [0u8; PACKET];
    p[0] = 0x23; // LI 0, VN 4, modo 3 (client)
    p[40..48].copy_from_slice(&to_bytes(t0_unix));
    p
}

/// Legge la risposta. `request` e' quella inviata (la risposta deve rimandare indietro la sua ora di invio),
/// `t0` e `t3` sono le ore locali di invio e di arrivo, in secondi Unix.
pub fn parse_response(buf: &[u8], request: &[u8; PACKET], t0: f64, t3: f64) -> Result<Sample, String> {
    if buf.len() < PACKET {
        return Err("risposta troppo corta".into());
    }
    if buf[0] & 7 != 4 && buf[0] & 7 != 5 {
        return Err("non e' una risposta di un server".into());
    }
    if buf[0] >> 6 == 3 {
        return Err("il server non e' sincronizzato".into());
    }
    if buf[1] == 0 {
        return Err("il server rifiuta la richiesta (kiss-of-death)".into());
    }
    if buf[24..32] != request[40..48] {
        return Err("risposta non pertinente alla richiesta".into());
    }
    let (t1, t2) = (from_bytes(&buf[32..40]), from_bytes(&buf[40..48]));
    if t1 == -NTP_TO_UNIX || t2 == -NTP_TO_UNIX {
        return Err("il server non ha messo l'ora".into());
    }
    Ok(Sample {
        offset_ms: ((t1 - t0) + (t2 - t3)) / 2.0 * 1000.0,
        delay_ms: ((t3 - t0) - (t2 - t1)) * 1000.0,
        stratum: buf[1],
    })
}

/// Interroga un server (`nome` o `nome:porta`, porta 123 se manca).
pub fn query(server: &str, timeout: Duration) -> Result<Sample, String> {
    let target = if server.contains(':') { server.to_string() } else { format!("{server}:123") };
    let addr = target
        .to_socket_addrs()
        .map_err(|e| format!("{server}: {e}"))?
        .find(|a| a.is_ipv4())
        .ok_or_else(|| format!("{server}: nessun indirizzo IPv4"))?;
    let socket = UdpSocket::bind("0.0.0.0:0").map_err(|e| e.to_string())?;
    socket.set_read_timeout(Some(timeout)).map_err(|e| e.to_string())?;
    socket.set_write_timeout(Some(timeout)).map_err(|e| e.to_string())?;
    let t0 = now_unix();
    let request = build_request(t0);
    socket.send_to(&request, addr).map_err(|e| format!("{server}: {e}"))?;
    let mut buf = [0u8; 128];
    let (n, from) = socket.recv_from(&mut buf).map_err(|e| format!("{server}: nessuna risposta ({e})"))?;
    let t3 = now_unix();
    if from.ip() != addr.ip() {
        return Err(format!("{server}: risposta da un altro indirizzo"));
    }
    parse_response(&buf[..n], &request, t0, t3).map_err(|e| format!("{server}: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Una risposta di un server il cui orologio e' avanti di `ahead` secondi, con `latency` di rete per verso.
    fn reply(request: &[u8; PACKET], t0: f64, ahead: f64, latency: f64) -> ([u8; PACKET], f64) {
        let server_receive = t0 + latency + ahead;
        let server_transmit = server_receive + 0.001;
        let mut p = [0u8; PACKET];
        p[0] = 0x24; // LI 0, VN 4, modo 4 (server)
        p[1] = 2;
        p[24..32].copy_from_slice(&request[40..48]);
        p[32..40].copy_from_slice(&to_bytes(server_receive));
        p[40..48].copy_from_slice(&to_bytes(server_transmit));
        (p, t0 + latency * 2.0 + 0.001)
    }

    #[test]
    fn a_local_clock_that_is_behind_gives_a_positive_offset() {
        let t0 = 1_800_000_000.25;
        let request = build_request(t0);
        let (packet, t3) = reply(&request, t0, 0.350, 0.020);
        let s = parse_response(&packet, &request, t0, t3).unwrap();
        assert!((s.offset_ms - 350.0).abs() < 0.5, "{s:?}");
        assert!((s.delay_ms - 40.0).abs() < 0.5, "{s:?}");
        assert_eq!(s.stratum, 2);
    }

    #[test]
    fn a_local_clock_that_is_ahead_gives_a_negative_offset() {
        let t0 = 1_800_000_000.0;
        let request = build_request(t0);
        let (packet, t3) = reply(&request, t0, -0.120, 0.010);
        assert!((parse_response(&packet, &request, t0, t3).unwrap().offset_ms + 120.0).abs() < 0.5);
    }

    #[test]
    fn the_request_is_a_client_packet_of_48_bytes_with_the_send_time() {
        let r = build_request(1_800_000_000.5);
        assert_eq!(r[0], 0x23);
        assert!((from_bytes(&r[40..48]) - 1_800_000_000.5).abs() < 1e-6);
    }

    #[test]
    fn refuses_bad_answers() {
        let t0 = 1_800_000_000.0;
        let request = build_request(t0);
        let (good, t3) = reply(&request, t0, 0.0, 0.01);
        assert!(parse_response(&good[..40], &request, t0, t3).is_err(), "corta");
        let mut other = good;
        other[24] ^= 0xff;
        assert!(parse_response(&other, &request, t0, t3).is_err(), "non pertinente");
        let mut unsynced = good;
        unsynced[0] |= 0xc0;
        assert!(parse_response(&unsynced, &request, t0, t3).is_err(), "non sincronizzato");
        let mut kiss = good;
        kiss[1] = 0;
        assert!(parse_response(&kiss, &request, t0, t3).is_err(), "kiss-of-death");
        let mut client = good;
        client[0] = 0x23;
        assert!(parse_response(&client, &request, t0, t3).is_err(), "modo sbagliato");
    }
}
