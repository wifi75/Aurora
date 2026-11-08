//! I comandi che un programma collegato (GridTracker, JTAlert...) manda a chi gli manda il flusso: lo stesso elenco che
//! `MessageClient::impl::parse_message` di JTDX (`REA 0x56c360`) sa leggere. Si decodificano con [`decode_request`].

use crate::{
    message::{Decode, ANNOTATION_INFO, TX_INHIBIT, CLEAR, CONFIGURE, FREE_TEXT, HALT_TX, HIGHLIGHT_CALLSIGN, LOCATION, REPLAY, REPLY, SET_TX_FREQ, SWITCH_CONFIGURATION, TRIGGER_CQ},
    wire::Reader,
    Error, MAGIC,
};

#[derive(Debug, Clone, PartialEq)]
pub enum Request {
    /// Come un doppio clic sulla riga: ora, rapporto, dt, frequenza, modo e testo del messaggio decodificato.
    Reply { decode: Decode, modifiers: u8 },
    /// Rimanda i decode correnti.
    Replay,
    /// Ferma la trasmissione; con `auto_tx_only` spegne solo «Abilita Tx».
    HaltTx { auto_tx_only: bool },
    /// Imposta il messaggio libero; con `send` lo trasmette.
    FreeText { text: String, send: bool },
    /// Frequenza audio di trasmissione (Hz).
    SetTxFreq { hz: u32 },
    /// Come il pulsante CQ (con `direction` per esempio «DX»).
    TriggerCq { direction: String, send: bool },
    /// Clear in ingresso: svuota le liste. `window` 0 attivita' di banda, 1 frequenza Rx, 2 entrambe; `None` se il campo manca (vale 0).
    Clear { window: Option<u8> },
    /// Location (11): il locatore della stazione, per la sessione (un GPS, un programma di mappe).
    Location { location: String },
    /// Highlight Callsign (13): colori di sfondo e testo per un nominativo (RGB); entrambi `None` (colori non validi) = togli.
    /// `last_only`: solo l'ultima riga con quel nominativo.
    HighlightCallsign { call: String, background: Option<[u8; 3]>, foreground: Option<[u8; 3]>, last_only: bool },
    /// Switch Configuration (14): passa alla configurazione con quel nome.
    SwitchConfiguration { name: String },
    /// Configure (15): i campi vuoti o a `u32::MAX` non cambiano.
    Configure(Configure),
    /// AnnotationInfo (16): valore di ordinamento di un nominativo per i Hound della Fox. `None` (valore non dato o `0xffffffff`) lo toglie.
    AnnotationInfo { call: String, order: Option<u32> },
}

/// Un Configure letto: `None` dove il protocollo dice «non cambiare» (testo vuoto, `u32::MAX`).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Configure {
    pub mode: Option<String>,
    pub frequency_tolerance: Option<u32>,
    pub submode: Option<String>,
    pub fast_mode: bool,
    pub tr_period: Option<u32>,
    pub rx_df: Option<u32>,
    pub dx_call: Option<String>,
    pub dx_grid: Option<String>,
    pub generate_messages: bool,
}

/// Un datagramma ricevuto con il numero di schema e l'identificativo del destinatario (JTDX scarta i datagrammi con un altro identificativo e
/// adotta lo schema piu' alto che vede). `request` e' `None` per i tipi che non sono comandi (il Heartbeat di risposta, ...).
#[derive(Debug, Clone, PartialEq)]
pub struct Addressed {
    pub schema: u32,
    pub id: Option<String>,
    pub request: Option<Request>,
}

/// Legge un datagramma in arrivo da un programma collegato.
pub fn decode_request(datagram: &[u8]) -> Result<Addressed, Error> {
    let mut r = Reader::new(datagram);
    let magic = r.u32()?;
    if magic != MAGIC {
        return Err(Error::BadMagic(magic));
    }
    let schema = r.u32()?;
    let kind = r.u32()?;
    let id = r.utf8()?;
    let request = match kind {
        REPLY => {
            let time_ms = r.u32()?;
            let snr = r.i32()?;
            let delta_time = r.f64()?;
            let delta_freq = r.u32()?;
            let mode = r.utf8()?;
            let message = r.utf8()?;
            let low_confidence = r.bool()?;
            let modifiers = r.u8()?;
            Request::Reply {
                decode: Decode { id: id.clone(), is_new: true, time_ms, snr, delta_time, delta_freq, mode, message, low_confidence, off_air: false },
                modifiers,
            }
        }
        REPLAY => Request::Replay,
        HALT_TX => Request::HaltTx { auto_tx_only: r.bool()? },
        FREE_TEXT => Request::FreeText { text: r.utf8()?.unwrap_or_default(), send: r.bool()? },
        SET_TX_FREQ => Request::SetTxFreq { hz: r.u32()? },
        TRIGGER_CQ => Request::TriggerCq { direction: r.utf8()?.unwrap_or_default(), send: r.bool()? },
        CLEAR => Request::Clear { window: if r.is_empty() { None } else { Some(r.u8()?) } },
        LOCATION => Request::Location { location: r.utf8()?.unwrap_or_default() },
        HIGHLIGHT_CALLSIGN => Request::HighlightCallsign { call: r.utf8()?.unwrap_or_default(), background: r.qcolor()?, foreground: r.qcolor()?, last_only: r.bool()? },
        SWITCH_CONFIGURATION => Request::SwitchConfiguration { name: r.utf8()?.unwrap_or_default() },
        CONFIGURE => {
            let text = |v: Option<String>| v.map(|t| t.trim().to_string()).filter(|t| !t.is_empty());
            let number = |v: u32| (v != u32::MAX).then_some(v);
            Request::Configure(Configure {
                mode: text(r.utf8()?),
                frequency_tolerance: number(r.u32()?),
                submode: text(r.utf8()?),
                fast_mode: r.bool()?,
                tr_period: number(r.u32()?),
                rx_df: number(r.u32()?),
                dx_call: text(r.utf8()?),
                dx_grid: text(r.utf8()?),
                generate_messages: r.bool()?,
            })
        }
        ANNOTATION_INFO => {
            let call = r.utf8()?.unwrap_or_default();
            let provided = r.bool()?;
            let order = r.u32()?;
            Request::AnnotationInfo { call, order: (provided && order != u32::MAX).then_some(order) }
        }
        _ => return Ok(Addressed { schema, id, request: None }),
    };
    Ok(Addressed { schema, id, request: Some(request) })
}

/// Lunghezza massima di un datagramma TxInhibit (`inhibit_message_limit` di `MessageClient.cpp`, WSJT-X 3.3).
pub const INHIBIT_DATAGRAM_MAX: usize = 4096;
/// Lunghezza massima dell'identificativo di destinazione in un TxInhibit, in byte.
const INHIBIT_ID_MAX: u32 = 1024;
/// Lunghezza massima di identita' del controller e nome della stazione, in byte UTF-8.
const INHIBIT_TEXT_MAX: u32 = 128;
/// Durata minima e massima di una prenotazione, in millisecondi (0 = rilascio).
pub const TTL_MIN_MS: u32 = 100;
pub const TTL_MAX_MS: u32 = 30_000;

/// Un TxInhibit valido (tipo 18): il controller, la durata (0 = rilascia) e la stazione descrittiva (vuota se assente).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TxInhibit {
    pub schema: u32,
    pub controller: String,
    pub ttl_ms: u32,
    pub station: String,
}

/// L'esito della lettura di un datagramma come TxInhibit, come `MessageClient::impl::parse_tx_inhibit` di WSJT-X 3.3.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InhibitParse {
    /// Non e' un TxInhibit: si legge come gli altri datagrammi.
    NotInhibit,
    /// TxInhibit malformato: si ignora e si conta.
    Invalid,
    /// TxInhibit per un'altra istanza (identificativo diverso): si ignora senza contarlo.
    OtherId,
    Command(TxInhibit),
}

/// Identita' del controller (`identity`: non vuota, niente spazi) o nome della stazione (nulla = vuota): al piu' 128 byte UTF-8 stampabili.
fn read_inhibit_text(r: &mut Reader, identity: bool) -> Option<String> {
    let len = r.u32().ok()?;
    if len == u32::MAX && !identity {
        return Some(String::new());
    }
    if len > INHIBIT_TEXT_MAX {
        return None;
    }
    let bytes = r.bytes(len as usize).ok()?;
    let text = std::str::from_utf8(bytes).ok()?.to_string();
    if identity && text.is_empty() {
        return None;
    }
    if text.chars().any(|c| c.is_control() || (identity && c.is_whitespace())) {
        return None;
    }
    Some(text)
}

/// Legge un datagramma come TxInhibit per l'istanza `own_id`. Limiti di WSJT-X 3.3: datagramma fino a 4096 byte, identificativo fino a 1024,
/// schema almeno 2, controller non vuoto senza spazi (fino a 128 byte), durata 0 oppure 100-30000 ms, stazione fino a 128 byte.
pub fn parse_tx_inhibit(datagram: &[u8], own_id: &str) -> InhibitParse {
    let word = |at: usize| datagram.get(at..at + 4).and_then(|b| <[u8; 4]>::try_from(b).ok()).map(u32::from_be_bytes);
    if word(8) != Some(TX_INHIBIT) {
        return InhibitParse::NotInhibit;
    }
    if datagram.len() < 16 || datagram.len() > INHIBIT_DATAGRAM_MAX || word(12).is_none_or(|n| n > INHIBIT_ID_MAX) || word(0) != Some(MAGIC) {
        return InhibitParse::Invalid;
    }
    let mut r = Reader::new(datagram);
    let header = (|| -> Result<(u32, Option<String>), Error> {
        r.u32()?;
        let schema = r.u32()?;
        r.u32()?;
        Ok((schema, r.utf8()?))
    })();
    let Ok((schema, Some(id))) = header else { return InhibitParse::Invalid };
    if schema < 2 {
        return InhibitParse::Invalid;
    }
    if id != own_id {
        return InhibitParse::OtherId;
    }
    let Some(controller) = read_inhibit_text(&mut r, true) else { return InhibitParse::Invalid };
    let Ok(ttl_ms) = r.u32() else { return InhibitParse::Invalid };
    if ttl_ms != 0 && !(TTL_MIN_MS..=TTL_MAX_MS).contains(&ttl_ms) {
        return InhibitParse::Invalid;
    }
    let Some(station) = read_inhibit_text(&mut r, false) else { return InhibitParse::Invalid };
    InhibitParse::Command(TxInhibit { schema, controller, ttl_ms, station })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::message::encode_tx_inhibit;

    #[test]
    fn a_hold_and_a_release_are_read() {
        let hold = encode_tx_inhibit("WSJT-X", 3, "AMP-1", 600, Some("K1ABC amp"));
        assert_eq!(parse_tx_inhibit(&hold, "WSJT-X"), InhibitParse::Command(TxInhibit { schema: 3, controller: "AMP-1".into(), ttl_ms: 600, station: "K1ABC amp".into() }));
        let release = encode_tx_inhibit("WSJT-X", 2, "AMP-1", 0, None);
        assert_eq!(parse_tx_inhibit(&release, "WSJT-X"), InhibitParse::Command(TxInhibit { schema: 2, controller: "AMP-1".into(), ttl_ms: 0, station: String::new() }));
    }

    #[test]
    fn other_types_and_other_instances_are_not_counted() {
        assert_eq!(parse_tx_inhibit(&crate::message::encode_replay("WSJT-X", 2), "WSJT-X"), InhibitParse::NotInhibit);
        assert_eq!(parse_tx_inhibit(&[1, 2, 3], "WSJT-X"), InhibitParse::NotInhibit);
        assert_eq!(parse_tx_inhibit(&encode_tx_inhibit("ALTRO", 2, "AMP-1", 600, None), "WSJT-X"), InhibitParse::OtherId);
    }

    #[test]
    fn out_of_range_and_malformed_commands_are_invalid() {
        for ttl in [1, 99, 30_001, u32::MAX] {
            assert_eq!(parse_tx_inhibit(&encode_tx_inhibit("JTDX", 2, "AMP", ttl, None), "JTDX"), InhibitParse::Invalid, "durata {ttl}");
        }
        for ttl in [100, 30_000] {
            assert!(matches!(parse_tx_inhibit(&encode_tx_inhibit("JTDX", 2, "AMP", ttl, None), "JTDX"), InhibitParse::Command(_)), "durata {ttl}");
        }
        // controller vuoto, con spazi, troppo lungo; stazione troppo lunga o con caratteri di controllo
        assert_eq!(parse_tx_inhibit(&encode_tx_inhibit("JTDX", 2, "", 600, None), "JTDX"), InhibitParse::Invalid);
        assert_eq!(parse_tx_inhibit(&encode_tx_inhibit("JTDX", 2, "AMP 1", 600, None), "JTDX"), InhibitParse::Invalid);
        assert_eq!(parse_tx_inhibit(&encode_tx_inhibit("JTDX", 2, &"A".repeat(129), 600, None), "JTDX"), InhibitParse::Invalid);
        assert!(matches!(parse_tx_inhibit(&encode_tx_inhibit("JTDX", 2, &"A".repeat(128), 600, None), "JTDX"), InhibitParse::Command(_)));
        assert_eq!(parse_tx_inhibit(&encode_tx_inhibit("JTDX", 2, "AMP", 600, Some(&"S".repeat(129))), "JTDX"), InhibitParse::Invalid);
        assert_eq!(parse_tx_inhibit(&encode_tx_inhibit("JTDX", 2, "AMP", 600, Some("a\u{7}b")), "JTDX"), InhibitParse::Invalid);
        // schema 1, numero magico sbagliato, datagramma troncato o troppo lungo
        assert_eq!(parse_tx_inhibit(&encode_tx_inhibit("JTDX", 1, "AMP", 600, None), "JTDX"), InhibitParse::Invalid);
        let mut bad = encode_tx_inhibit("JTDX", 2, "AMP", 600, None);
        bad[0] = 0;
        assert_eq!(parse_tx_inhibit(&bad, "JTDX"), InhibitParse::Invalid);
        let full = encode_tx_inhibit("JTDX", 2, "AMP", 600, None);
        assert_eq!(parse_tx_inhibit(&full[..full.len() - 2], "JTDX"), InhibitParse::Invalid);
        let mut long = full.clone();
        long.resize(INHIBIT_DATAGRAM_MAX + 1, 0);
        assert_eq!(parse_tx_inhibit(&long, "JTDX"), InhibitParse::Invalid);
        // campi in piu' in coda si ignorano
        let mut trailing = full;
        trailing.extend_from_slice(&[0, 0, 0, 0]);
        assert!(matches!(parse_tx_inhibit(&trailing, "JTDX"), InhibitParse::Command(_)));
    }

    #[test]
    fn the_inhibit_status_reads_back() {
        let status = crate::message::InhibitStatus { id: "JTDX".into(), supported: true, inhibited: true, source_station: "AMP".into(), hold_rx: 3, release_rx: 1, expiries: 2, invalid: 4 };
        let datagram = crate::encode_inhibit_status(3, &status);
        let packet = crate::decode(&datagram).expect("datagramma");
        assert_eq!((packet.schema, packet.message), (3, crate::Message::InhibitStatus(status)));
    }
}
