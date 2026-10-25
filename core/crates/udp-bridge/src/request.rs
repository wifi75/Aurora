//! I comandi che un programma collegato (GridTracker, JTAlert...) manda a chi gli manda il flusso: lo stesso elenco che
//! `MessageClient::impl::parse_message` di JTDX (`REA 0x56c360`) sa leggere. Si decodificano con [`decode_request`].

use crate::{
    message::{Decode, FREE_TEXT, HALT_TX, REPLAY, REPLY, SET_TX_FREQ, TRIGGER_CQ},
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
        _ => return Ok(Addressed { schema, id, request: None }),
    };
    Ok(Addressed { schema, id, request: Some(request) })
}
