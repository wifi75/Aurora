//! «Rimanda i QSO persi»: i QSO per un log su questo PC che in quel momento e' chiuso restano in coda e partono quando il log si riapre.
//!
//! L'UDP non ha conferme: un datagramma mandato a una porta su cui nessuno ascolta si perde. Per i destinatari su questo PC (127.0.0.1, ::1)
//! si guarda pero' nella tabella delle porte UDP del sistema se un programma ha aperto quella porta (in sola lettura: niente prove di bind che
//! potrebbero disturbare un programma che parte proprio in quell'istante).
//! - Nessuno in ascolto: il record va in coda (file `log-in-attesa.json` nella cartella dei dati, resta dopo un riavvio di Aurora).
//! - Il log si riapre: dopo tre controlli di fila con la porta aperta (10-15 s, il tempo di caricare il suo archivio) i QSO in coda partono, al
//!   piu' 20 ogni 5 s e a 50 ms l'uno dall'altro, nell'ordine in cui sono stati fatti.
//! - Ogni QSO parte una volta sola: o subito o dalla coda, mai tutte e due.
//! - Se il sistema non sa dire chi ascolta (indirizzo di un altro PC, sistema senza tabella) il QSO parte subito come prima.
//! - Dopo sette giorni in coda un QSO si scarta (con un avviso nel registro); al piu' 2000 QSO in coda.

use std::{
    collections::HashMap,
    net::{IpAddr, SocketAddr},
    path::PathBuf,
};

/// Il nome del file della coda nella cartella dei dati.
pub const FILE_NAME: &str = "log-in-attesa.json";
/// Quanti QSO al massimo in coda (i piu' vecchi escono per primi).
const MAX_WAITING: usize = 2000;
/// Dopo quanto un QSO in coda si scarta.
const MAX_AGE_SECS: u64 = 7 * 24 * 3600;
/// Quanti controlli di fila con la porta aperta prima di mandare (il log ha il tempo di finire di partire).
const SEEN_BEFORE_SEND: u32 = 3;
/// Quanti QSO al massimo per ogni controllo (l'esportatore non resta fermo a lungo).
pub const BATCH: usize = 20;

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Waiting {
    /// Il destinatario (127.0.0.1:2237...).
    pub to: SocketAddr,
    /// Il datagramma gia' pronto (record ADIF o XML N1MM).
    pub record: String,
    /// Quando e' entrato in coda (secondi Unix).
    pub at: u64,
    /// Il nominativo, per i messaggi del registro.
    #[serde(default)]
    pub call: String,
}

/// La coda dei QSO in attesa, con il file che la conserva.
#[derive(Default)]
pub struct Queue {
    items: Vec<Waiting>,
    file: Option<PathBuf>,
    seen: HashMap<SocketAddr, u32>,
}

impl Queue {
    /// Rilegge la coda dal file (se c'e'); un file illeggibile si lascia com'e' e si riparte vuoti.
    pub fn load(file: Option<PathBuf>) -> Queue {
        let items = file
            .as_ref()
            .and_then(|f| std::fs::read_to_string(f).ok())
            .and_then(|text| match serde_json::from_str::<Vec<Waiting>>(&text) {
                Ok(list) => Some(list),
                Err(e) => {
                    tracing::warn!("coda dei QSO per i log chiusi illeggibile ({e}): si riparte vuoti");
                    None
                }
            })
            .unwrap_or_default();
        if !items.is_empty() {
            tracing::info!("{} QSO in coda per i log chiusi", items.len());
        }
        Queue { items, file, seen: HashMap::new() }
    }

    pub fn len(&self) -> usize {
        self.items.len()
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// Vero se il QSO per `to` va in coda: ci sono gia' QSO in attesa per lui (l'ordine si rispetta) o nessuno ascolta sulla sua porta.
    pub fn should_hold(&self, to: SocketAddr) -> bool {
        self.items.iter().any(|w| w.to == to) || listening(to) == Some(false)
    }

    pub fn push(&mut self, item: Waiting) {
        tracing::info!("QSO con {} in coda: nessun programma in ascolto su {}", item.call, item.to);
        self.items.push(item);
        if self.items.len() > MAX_WAITING {
            let extra = self.items.len() - MAX_WAITING;
            let dropped: Vec<Waiting> = self.items.drain(..extra).collect();
            for w in dropped {
                tracing::warn!("QSO con {} per {} scartato: coda piena", w.call, w.to);
            }
        }
        self.save();
    }

    /// I QSO da mandare adesso, gia' tolti dalla coda: quelli dei destinatari non piu' in attesa (opzione spenta o destinatario tolto: partono
    /// come prima) e, per quelli che vogliono il rinvio, quando il log e' aperto da tre controlli di fila. Scarta i QSO troppo vecchi.
    pub fn due(&mut self, wanted: &[SocketAddr], now: u64) -> Vec<Waiting> {
        let before = self.items.len();
        self.items.retain(|w| {
            let keep = now.saturating_sub(w.at) <= MAX_AGE_SECS;
            if !keep {
                tracing::warn!("QSO con {} per {} scartato: in coda da piu' di sette giorni", w.call, w.to);
            }
            keep
        });
        let mut ready: Vec<SocketAddr> = Vec::new();
        let mut targets: Vec<SocketAddr> = self.items.iter().map(|w| w.to).collect();
        targets.sort();
        targets.dedup();
        for to in targets {
            if !wanted.contains(&to) {
                ready.push(to);
                continue;
            }
            match listening(to) {
                Some(false) => {
                    self.seen.remove(&to);
                }
                // aperto, o non si sa piu' (si torna al comportamento di prima: si manda)
                _ => {
                    let n = self.seen.entry(to).or_insert(0);
                    *n += 1;
                    if *n >= SEEN_BEFORE_SEND {
                        ready.push(to);
                    }
                }
            }
        }
        let mut out = Vec::new();
        let mut rest = Vec::with_capacity(self.items.len());
        for w in self.items.drain(..) {
            if out.len() < BATCH && ready.contains(&w.to) {
                out.push(w);
            } else {
                rest.push(w);
            }
        }
        self.items = rest;
        if !self.items.iter().any(|w| ready.contains(&w.to)) {
            for to in &ready {
                self.seen.remove(to);
            }
        }
        if self.items.len() != before {
            self.save();
        }
        out
    }

    /// Scrive la coda (file temporaneo e poi rinomina); vuota, il file si toglie.
    fn save(&self) {
        let Some(file) = &self.file else { return };
        if self.items.is_empty() {
            if file.exists()
                && let Err(e) = std::fs::remove_file(file)
            {
                tracing::warn!("{}: {e}", file.display());
            }
            return;
        }
        let text = match serde_json::to_string_pretty(&self.items) {
            Ok(t) => t,
            Err(e) => {
                tracing::warn!("coda dei QSO per i log chiusi non salvata: {e}");
                return;
            }
        };
        let tmp = file.with_extension("json.tmp");
        if let Err(e) = std::fs::write(&tmp, text).and_then(|()| std::fs::rename(&tmp, file)) {
            tracing::warn!("coda dei QSO per i log chiusi non salvata in {}: {e}", file.display());
        }
    }
}

/// Un programma ascolta sulla porta UDP di `to`? Solo per gli indirizzi di questo PC (127.0.0.1, ::1); `None` quando non si puo' sapere.
pub fn listening(to: SocketAddr) -> Option<bool> {
    if !to.ip().is_loopback() {
        return None;
    }
    let sockets = udp_sockets()?;
    Some(sockets.iter().any(|(ip, port)| *port == to.port() && covers(*ip, to.ip())))
}

/// Un socket aperto su `bound` riceve i datagrammi per `target`? Su tutti gli indirizzi (0.0.0.0, ::) o sullo stesso indirizzo.
fn covers(bound: IpAddr, target: IpAddr) -> bool {
    bound.is_unspecified() || bound == target || (bound.is_loopback() && target.is_loopback() && bound.is_ipv4() == target.is_ipv4())
}

/// Le porte UDP aperte su questo PC dagli altri programmi (Aurora esclusa dove il sistema lo dice).
#[cfg(windows)]
fn udp_sockets() -> Option<Vec<(IpAddr, u16)>> {
    use std::net::{Ipv4Addr, Ipv6Addr};
    use windows::Win32::NetworkManagement::IpHelper::{GetExtendedUdpTable, MIB_UDP6ROW_OWNER_PID, MIB_UDPROW_OWNER_PID, UDP_TABLE_OWNER_PID};

    /// La tabella grezza di una famiglia di indirizzi (2 = IPv4, 23 = IPv6).
    fn table(family: u32) -> Option<Vec<u8>> {
        let mut size: u32 = 0;
        // SAFETY: con il puntatore nullo la funzione scrive solo la dimensione richiesta in `size`.
        unsafe { GetExtendedUdpTable(None, &mut size, false, family, UDP_TABLE_OWNER_PID, 0) };
        for _ in 0..3 {
            let mut buf = vec![0u8; size as usize + 1024];
            let mut len = buf.len() as u32;
            // SAFETY: `buf` e' lungo `len` byte e resta vivo per tutta la chiamata.
            let rc = unsafe { GetExtendedUdpTable(Some(buf.as_mut_ptr().cast()), &mut len, false, family, UDP_TABLE_OWNER_PID, 0) };
            if rc == 0 {
                return Some(buf);
            }
            size = len;
        }
        None
    }

    fn rows<T: Copy>(buf: &[u8]) -> Vec<T> {
        let Some(count) = buf.get(..4).and_then(|b| b.try_into().ok()).map(u32::from_ne_bytes) else { return Vec::new() };
        // le righe partono dopo il contatore, allineate come la struttura
        let start = std::mem::align_of::<T>().max(4);
        let size = std::mem::size_of::<T>();
        (0..count as usize)
            .filter_map(|i| {
                let off = start + i * size;
                let bytes = buf.get(off..off + size)?;
                // SAFETY: `bytes` ha esattamente la dimensione di `T`, una struttura C di soli interi; la lettura non e' allineata.
                Some(unsafe { std::ptr::read_unaligned(bytes.as_ptr().cast::<T>()) })
            })
            .collect()
    }

    let me = std::process::id();
    let port = |p: u32| u16::from_be((p & 0xffff) as u16);
    let mut out = Vec::new();
    let v4 = table(2)?;
    for r in rows::<MIB_UDPROW_OWNER_PID>(&v4) {
        if r.dwOwningPid != me {
            out.push((IpAddr::V4(Ipv4Addr::from(r.dwLocalAddr.to_ne_bytes())), port(r.dwLocalPort)));
        }
    }
    if let Some(v6) = table(23) {
        for r in rows::<MIB_UDP6ROW_OWNER_PID>(&v6) {
            if r.dwOwningPid != me {
                out.push((IpAddr::V6(Ipv6Addr::from(r.ucLocalAddr)), port(r.dwLocalPort)));
            }
        }
    }
    Some(out)
}

/// Le porte UDP aperte su questo PC (`/proc/net/udp` e `udp6`; Aurora compresa: Linux non dice qui di chi sono).
#[cfg(target_os = "linux")]
fn udp_sockets() -> Option<Vec<(IpAddr, u16)>> {
    let mut out = Vec::new();
    let v4 = std::fs::read_to_string("/proc/net/udp").ok()?;
    out.extend(parse_proc(&v4));
    if let Ok(v6) = std::fs::read_to_string("/proc/net/udp6") {
        out.extend(parse_proc(&v6));
    }
    Some(out)
}

/// Su questi sistemi non si sa chi ascolta: i QSO partono subito come prima.
#[cfg(not(any(windows, target_os = "linux")))]
fn udp_sockets() -> Option<Vec<(IpAddr, u16)>> {
    None
}

/// Le righe di `/proc/net/udp` o `udp6`: indirizzo locale in esadecimale (parole a 32 bit nell'ordine della macchina) e porta.
#[cfg_attr(not(target_os = "linux"), allow(dead_code))]
fn parse_proc(text: &str) -> Vec<(IpAddr, u16)> {
    use std::net::{Ipv4Addr, Ipv6Addr};
    text.lines()
        .skip(1)
        .filter_map(|line| {
            let local = line.split_whitespace().nth(1)?;
            let (addr, port) = local.split_once(':')?;
            let port = u16::from_str_radix(port, 16).ok()?;
            let word = |s: &str| u32::from_str_radix(s, 16).ok().map(u32::to_le_bytes);
            let ip = match addr.len() {
                8 => IpAddr::V4(Ipv4Addr::from(word(addr)?)),
                32 => {
                    let mut b = [0u8; 16];
                    for i in 0..4 {
                        b[i * 4..i * 4 + 4].copy_from_slice(&word(addr.get(i * 8..i * 8 + 8)?)?);
                    }
                    IpAddr::V6(Ipv6Addr::from(b))
                }
                _ => return None,
            };
            Some((ip, port))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn proc_lines_give_address_and_port() {
        let text = "  sl  local_address rem_address   st\n   1: 0100007F:08BD 00000000:0000 07\n   2: 00000000:2F1C 00000000:0000 07\n";
        let got = parse_proc(text);
        assert_eq!(got, vec![("127.0.0.1".parse().unwrap(), 2237), ("0.0.0.0".parse().unwrap(), 12060)]);
        let v6 = "  sl  local_address\n   1: 00000000000000000000000001000000:08BD 0\n";
        assert_eq!(parse_proc(v6), vec![("::1".parse().unwrap(), 2237)]);
    }

    #[test]
    fn a_socket_on_all_addresses_covers_the_loopback() {
        let lo: IpAddr = "127.0.0.1".parse().unwrap();
        assert!(covers("0.0.0.0".parse().unwrap(), lo));
        assert!(covers(lo, lo));
        assert!(!covers("192.168.1.5".parse().unwrap(), lo));
    }

    #[test]
    fn a_remote_address_is_never_held() {
        assert_eq!(listening("192.168.1.20:2237".parse().unwrap()), None);
    }

    #[test]
    fn a_closed_local_port_is_seen_as_closed() {
        // una porta presa e poi lasciata: subito dopo nessuno ascolta
        let port = {
            let s = std::net::UdpSocket::bind("127.0.0.1:0").unwrap();
            s.local_addr().unwrap().port()
        };
        let to: SocketAddr = format!("127.0.0.1:{port}").parse().unwrap();
        if cfg!(any(windows, target_os = "linux")) {
            assert_eq!(listening(to), Some(false));
        }
    }

    #[test]
    fn the_queue_waits_for_three_checks_then_sends_in_order() {
        let dir = std::env::temp_dir().join(format!("aurora-attesa-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join(FILE_NAME);
        let _ = std::fs::remove_file(&file);
        // un "log" che ascolta: un socket di un altro processo non c'e' nelle prove, quindi su Windows Aurora stessa e' esclusa; si prova la
        // logica con un destinatario remoto (stato sconosciuto = si manda) e con l'opzione spenta
        let remote: SocketAddr = "192.168.1.20:2237".parse().unwrap();
        let mut q = Queue::load(Some(file.clone()));
        for (i, call) in ["K1ABC", "K2ABC"].iter().enumerate() {
            q.push(Waiting { to: remote, record: format!("r{i}"), at: 100, call: (*call).into() });
        }
        assert!(file.exists());
        assert_eq!(Queue::load(Some(file.clone())).len(), 2, "la coda resta dopo un riavvio");
        assert!(q.due(&[remote], 200).is_empty());
        assert!(q.due(&[remote], 205).is_empty());
        let sent = q.due(&[remote], 210);
        assert_eq!(sent.iter().map(|w| w.record.as_str()).collect::<Vec<_>>(), ["r0", "r1"]);
        assert!(q.is_empty());
        assert!(!file.exists(), "vuota, il file si toglie");
        // opzione spenta: si manda subito
        q.push(Waiting { to: remote, record: "r2".into(), at: 300, call: "K3ABC".into() });
        assert_eq!(q.due(&[], 301).len(), 1);
        // troppo vecchio: scartato
        q.push(Waiting { to: remote, record: "r3".into(), at: 0, call: "K4ABC".into() });
        assert!(q.due(&[remote], MAX_AGE_SECS + 10).is_empty());
        assert!(q.is_empty());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
