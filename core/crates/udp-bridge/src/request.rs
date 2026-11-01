//! I comandi che un programma collegato (GridTracker, JTAlert...) manda a chi gli manda il flusso: lo stesso elenco che
//! `MessageClient::impl::parse_message` di JTDX (`REA 0x56c360`) sa leggere. Si decodificano con [`decode_request`].

use crate::{
    message::{Decode, CLEAR, CONFIGURE, FREE_TEXT, HALT_TX, HIGHLIGHT_CALLSIGN, LOCATION, REPLAY, REPLY, SET_TX_FREQ, SWITCH_CONFIGURATION, TRIGGER_CQ},
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
        _ => return Ok(Addressed { schema, id, request: None }),
    };
    Ok(Addressed { schema, id, request: Some(request) })
}
