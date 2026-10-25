//! Spotting su PSK Reporter: il formato dei datagrammi (IPFIX) che JTDX manda a `report.pskreporter.info:4739`.
//!
//! Letto nel sorgente pubblico (`psk_reporter.cpp`) e nel programma `jtdx.exe` 2.2.159: i modelli esadecimali di intestazione, descrittore del
//! ricevitore (`50E2`) e del trasmettitore (`50E3`) sono gli stessi byte; il costruttore avvia un timer di 300 000 ms (5 minuti), `sendReport`
//! compone pacchetti finche' non superano 1400 byte e li manda alla porta `0x1283` (4739). Ogni spot ha: nominativo, frequenza (4 byte), rapporto
//! (1 byte), modo, locatore, fonte (1 = automatica) e ora; il ricevitore (chi ascolta) ha nominativo, locatore, programma e antenna.

use std::collections::VecDeque;

/// Dove arrivano gli spot.
pub const HOST: &str = "report.pskreporter.info";
pub const PORT: u16 = 4739;
/// Ogni quanto si manda quello che e' in coda (JTDX: `reportTimer->start(5*60*1000)`).
pub const SEND_EVERY_SECS: u64 = 300;
/// Un pacchetto accetta spot finche' non ha raggiunto questa lunghezza (`MAX_PAYLOAD_LENGTH`): l'ultimo puo' superarla di poco.
pub const MAX_PAYLOAD: usize = 1400;
/// Tetto della coda: oltre, gli spot piu' vecchi si scartano (se la rete non c'e' per ore non si riempie la memoria).
pub const MAX_QUEUE: usize = 5000;

/// Intestazione IPFIX: versione `000A`, lunghezza, ora, numero di sequenza, identificativo casuale.
const HEADER: &str = "000A";
/// Modello del ricevitore (`m_rxInfoDescriptor_h`): id `50E2`, quattro campi (nominativo, locatore, programma, antenna).
const RX_DESCRIPTOR: &str = "0003002C50E200040000\
8002FFFF0000768F8004FFFF0000768F8008FFFF0000768F8009FFFF0000768F0000";
/// Modello del trasmettitore (`m_txInfoDescriptor_h`): id `50E3`, sette campi (nominativo, frequenza, rapporto, modo, locatore, fonte, ora).
const TX_DESCRIPTOR: &str = "0002003C50E30007\
8001FFFF0000768F800500040000768F800600010000768F800AFFFF0000768F8003FFFF0000768F800B00010000768F00960004";

/// Chi ascolta: i dati del ricevitore che finiscono in ogni pacchetto.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Local {
    pub call: String,
    pub grid: String,
    pub antenna: String,
    /// Il programma (`decodingSoftware`).
    pub software: String,
}

/// Una stazione ricevuta.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Spot {
    pub call: String,
    pub grid: String,
    /// Frequenza reale (radio + tono audio), in Hz.
    pub freq_hz: u64,
    /// `FT8` o `FT4`.
    pub mode: String,
    pub snr: i32,
    /// Secondi Unix del momento della ricezione.
    pub time: u32,
}

fn unhex(s: &str) -> Vec<u8> {
    let digits: Vec<u8> = s.bytes().filter(u8::is_ascii_hexdigit).map(|b| (b as char).to_digit(16).map_or(0, |d| d as u8)).collect();
    digits.chunks(2).map(|c| (c[0] << 4) | c.get(1).copied().unwrap_or(0)).collect()
}

/// Un testo con il suo byte di lunghezza davanti (campo a lunghezza variabile di IPFIX); oltre 254 byte si taglia.
fn push_text(out: &mut Vec<u8>, text: &str) {
    let bytes = text.as_bytes();
    let n = bytes.len().min(254);
    out.push(n as u8);
    out.extend_from_slice(&bytes[..n]);
}

/// Compone i pacchetti: porta con se' l'identificativo casuale e il numero di sequenza, che cresce a ogni pacchetto.
pub struct Packetizer {
    random_id: u32,
    sequence: u32,
}

impl Packetizer {
    pub fn new(random_id: u32) -> Packetizer {
        Packetizer { random_id, sequence: 0 }
    }

    /// Svuota la coda in uno o piu' pacchetti (`sendReport`): niente pacchetti con il solo ricevitore.
    pub fn build(&mut self, local: &Local, queue: &mut VecDeque<Spot>, now_secs: u32) -> Vec<Vec<u8>> {
        let rx_descriptor = unhex(RX_DESCRIPTOR);
        let tx_descriptor = unhex(TX_DESCRIPTOR);
        let mut packets = Vec::new();
        while !queue.is_empty() {
            // ricevitore
            let mut rx = vec![0x50, 0xE2, 0, 0];
            push_text(&mut rx, &local.call);
            push_text(&mut rx, &local.grid);
            push_text(&mut rx, &local.software);
            push_text(&mut rx, &local.antenna);
            rx.extend_from_slice(&[0, 0]);
            let rx_len = rx.len() as u16;
            rx[2..4].copy_from_slice(&rx_len.to_be_bytes());
            // trasmettitori: se ne aggiungono finche' il pacchetto non ha raggiunto il limite
            let mut tx = vec![0x50, 0xE3, 0, 0];
            while let Some(spot) = queue.front() {
                if 16 + rx_descriptor.len() + tx_descriptor.len() + rx.len() + tx.len() >= MAX_PAYLOAD && tx.len() > 4 {
                    break;
                }
                push_text(&mut tx, &spot.call);
                tx.extend_from_slice(&(spot.freq_hz.min(u64::from(u32::MAX)) as u32).to_be_bytes());
                tx.push(spot.snr.clamp(-128, 127) as i8 as u8);
                push_text(&mut tx, &spot.mode);
                push_text(&mut tx, &spot.grid);
                tx.push(1); // `REPORTER_SOURCE_AUTOMATIC`
                tx.extend_from_slice(&spot.time.to_be_bytes());
                queue.pop_front();
            }
            tx.extend_from_slice(&[0, 0]);
            let tx_len = tx.len() as u16;
            tx[2..4].copy_from_slice(&tx_len.to_be_bytes());
            self.sequence = self.sequence.wrapping_add(1);
            let mut packet = unhex(HEADER);
            packet.extend_from_slice(&[0, 0]);
            packet.extend_from_slice(&now_secs.to_be_bytes());
            packet.extend_from_slice(&self.sequence.to_be_bytes());
            packet.extend_from_slice(&self.random_id.to_be_bytes());
            packet.extend_from_slice(&rx_descriptor);
            packet.extend_from_slice(&tx_descriptor);
            packet.extend_from_slice(&rx);
            packet.extend_from_slice(&tx);
            let total = packet.len() as u16;
            packet[2..4].copy_from_slice(&total.to_be_bytes());
            packets.push(packet);
        }
        packets
    }
}

fn is_call(token: &str) -> bool {
    token.len() >= 3
        && token.chars().all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '/')
        && token.chars().any(|c| c.is_ascii_digit())
        && token.chars().any(|c| c.is_ascii_uppercase())
        && !is_grid(token)
}

/// Un locatore di quattro caratteri valido per lo spot: `[A-R]{2}[0-9]{2}` e non `RR73` (`gridOK` e `gridRR73` di JTDX).
pub fn is_grid(token: &str) -> bool {
    let b = token.as_bytes();
    b.len() == 4 && (b'A'..=b'R').contains(&b[0]) && (b'A'..=b'R').contains(&b[1]) && b[2].is_ascii_digit() && b[3].is_ascii_digit() && token != "RR73"
}

/// Nominativo e locatore di chi trasmette, come `DecodedText::deCallAndGrid` (poi `gridOK`): per un CQ il nominativo dopo il CQ (e dopo l'eventuale
/// direzione) con il suo locatore; per gli altri messaggi il secondo nominativo con il locatore che segue (anche dopo un `R`). Senza i due, nessuno spot.
/// I messaggi con nominativi tra `<...>` o non standard non si segnalano.
pub fn spot_call_and_grid(text: &str) -> Option<(String, String)> {
    if text.contains('<') {
        return None;
    }
    let t: Vec<String> = text.split_whitespace().map(str::to_uppercase).collect();
    let (call, grid) = if matches!(t.first().map(String::as_str), Some("CQ" | "DE" | "QRZ")) {
        // direzione: `DX`, una o due lettere, tre cifre, oppure una parola di 3 o 4 lettere senza cifre (`POTA`, `SOTA`...)
        let second = t.get(1)?;
        let modifier = second == "DX"
            || ((1..=2).contains(&second.len()) && second.chars().all(|c| c.is_ascii_uppercase()))
            || (second.len() == 3 && second.chars().all(|c| c.is_ascii_digit()))
            || ((3..=4).contains(&second.len()) && !second.chars().any(|c| c.is_ascii_digit()) && !is_call(second));
        let at = if modifier { 2 } else { 1 };
        (t.get(at)?.clone(), t.get(at + 1).cloned().unwrap_or_default())
    } else {
        let call = t.get(1)?.clone();
        let mut grid = t.get(2).cloned().unwrap_or_default();
        if grid == "R" {
            grid = t.get(3).cloned().unwrap_or_default();
        }
        (call, grid)
    };
    (is_call(&call) && is_grid(&grid)).then_some((call, grid))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn spot(call: &str, freq: u64, snr: i32) -> Spot {
        Spot { call: call.into(), grid: "FN42".into(), freq_hz: freq, mode: "FT8".into(), snr, time: 0x6700_0000 }
    }

    fn local() -> Local {
        Local { call: "IU3ABC".into(), grid: "JN45".into(), antenna: "Dipolo".into(), software: "Aurora v0.11.12".into() }
    }

    fn hex(bytes: &[u8]) -> String {
        bytes.iter().map(|b| format!("{b:02X}")).collect()
    }

    #[test]
    fn the_templates_are_the_bytes_of_jtdx() {
        // lunghezze scritte nei modelli stessi: 0x2C = 44 byte e 0x3C = 60 byte di descrittore
        assert_eq!(unhex(RX_DESCRIPTOR).len(), 0x2C, "descrittore del ricevitore");
        assert_eq!(unhex(TX_DESCRIPTOR).len(), 0x3C, "descrittore del trasmettitore");
        assert_eq!(&hex(&unhex(RX_DESCRIPTOR))[..20], "0003002C50E200040000");
        assert_eq!(&hex(&unhex(TX_DESCRIPTOR))[..16], "0002003C50E30007");
    }

    #[test]
    fn one_spot_makes_the_exact_packet_of_jtdx() {
        let mut p = Packetizer::new(0xDEAD_BEEF);
        let mut queue: VecDeque<Spot> = VecDeque::from([spot("K1ABC", 14_075_084, -16)]);
        let local = local();
        let packets = p.build(&local, &mut queue, 0x6700_0001);
        assert!(queue.is_empty());
        assert_eq!(packets.len(), 1);
        let packet = &packets[0];
        // intestazione: versione, lunghezza, ora, sequenza, identificativo
        let h = hex(packet);
        assert!(h.starts_with(&format!("000A{:04X}6700000100000001DEADBEEF", packet.len())), "{h}");
        // descrittori subito dopo l'intestazione
        assert_eq!(&h[32..32 + 88], hex(&unhex(RX_DESCRIPTOR)));
        // dati del ricevitore: 50E2, lunghezza, nominativo, locatore, programma, antenna, 0000
        let rx_at = 32 + 88 + 120;
        let rx = &h[rx_at..];
        assert!(rx.starts_with("50E2"), "{rx}");
        let rx_len = usize::from_str_radix(&rx[4..8], 16).unwrap();
        assert_eq!(&rx[8..8 + 14], hex(b"\x06IU3ABC").as_str());
        assert!(rx[8..2 * rx_len].ends_with("0000"));
        // dati del trasmettitore: 50E3, lunghezza, nominativo, frequenza, rapporto, modo, locatore, fonte, ora, 0000
        let tx = &rx[2 * rx_len..];
        assert_eq!(&tx[..4], "50E3");
        let tx_len = usize::from_str_radix(&tx[4..8], 16).unwrap();
        let expected_body = format!("05{}{:08X}{:02X}03{}04{}01{:08X}0000", hex(b"K1ABC"), 14_075_084u32, (-16i8) as u8, hex(b"FT8"), hex(b"FN42"), 0x6700_0000u32);
        assert_eq!(&tx[8..], expected_body, "il record del trasmettitore");
        assert_eq!(tx_len * 2, 8 + expected_body.len(), "la lunghezza copre l'insieme intero");
        assert_eq!(packet.len(), h.len() / 2);
    }

    #[test]
    fn sequence_grows_and_big_queues_are_split_around_1400_bytes() {
        let mut p = Packetizer::new(1);
        let mut queue: VecDeque<Spot> = (0..200).map(|i| spot(&format!("K{}ABC", i % 10), 14_074_000 + i, -10)).collect();
        let packets = p.build(&local(), &mut queue, 100);
        assert!(queue.is_empty());
        assert!(packets.len() >= 2, "200 spot non stanno in un pacchetto: {}", packets.len());
        for (i, packet) in packets.iter().enumerate() {
            assert!(packet.len() < MAX_PAYLOAD + 40, "un pacchetto supera il limite al piu' di un record: {}", packet.len());
            assert_eq!(u32::from_be_bytes([packet[8], packet[9], packet[10], packet[11]]), i as u32 + 1, "numero di sequenza");
            assert_eq!(u16::from_be_bytes([packet[2], packet[3]]) as usize, packet.len());
        }
        // un secondo invio continua la sequenza
        let mut again: VecDeque<Spot> = VecDeque::from([spot("K1ABC", 14_074_000, 0)]);
        let next = p.build(&local(), &mut again, 200);
        assert_eq!(u32::from_be_bytes([next[0][8], next[0][9], next[0][10], next[0][11]]), packets.len() as u32 + 1);
        // coda vuota: niente pacchetti (mai un pacchetto con il solo ricevitore)
        assert!(p.build(&local(), &mut VecDeque::new(), 300).is_empty());
    }

    #[test]
    fn only_messages_with_a_call_and_a_valid_grid_are_spotted() {
        let s = |t: &str| spot_call_and_grid(t);
        assert_eq!(s("CQ K1ABC FN42"), Some(("K1ABC".into(), "FN42".into())));
        assert_eq!(s("CQ DX K1ABC FN42"), Some(("K1ABC".into(), "FN42".into())));
        assert_eq!(s("CQ POTA K1ABC FN42"), Some(("K1ABC".into(), "FN42".into())));
        assert_eq!(s("CQ 123 K1ABC FN42"), Some(("K1ABC".into(), "FN42".into())));
        assert_eq!(s("IU3ABC K1ABC FN42"), Some(("K1ABC".into(), "FN42".into())));
        assert_eq!(s("IU3ABC K1ABC R FN42"), Some(("K1ABC".into(), "FN42".into())), "il locatore dopo la R");
        assert_eq!(s("IU3ABC K1ABC RR73"), None, "RR73 non e' un locatore");
        assert_eq!(s("IU3ABC K1ABC -10"), None);
        assert_eq!(s("IU3ABC K1ABC 73"), None);
        assert_eq!(s("CQ K1ABC"), None, "CQ senza locatore");
        assert_eq!(s("<...> K1ABC FN42"), None, "nominativi tra <...>");
        assert_eq!(s("CQ TEST K1ABC FN42"), Some(("K1ABC".into(), "FN42".into())));
        assert_eq!(s("cq k1abc fn42"), Some(("K1ABC".into(), "FN42".into())), "maiuscole come nel decodificatore");
        assert!(!is_grid("SA00") && is_grid("AA00") && is_grid("RR99") && !is_grid("RR73") && !is_grid("FN4"));
    }
}
