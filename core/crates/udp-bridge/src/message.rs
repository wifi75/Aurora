use crate::{wire::{Reader, Writer}, Error, MAGIC};

pub const HEARTBEAT: u32 = 0;
pub const STATUS: u32 = 1;
pub const DECODE: u32 = 2;
pub const CLEAR: u32 = 3;
pub const REPLY: u32 = 4;
pub const QSO_LOGGED: u32 = 5;
pub const CLOSE: u32 = 6;
pub const REPLAY: u32 = 7;
pub const HALT_TX: u32 = 8;
pub const FREE_TEXT: u32 = 9;
pub const WSPR_DECODE: u32 = 10;
pub const LOCATION: u32 = 11;
pub const LOGGED_ADIF: u32 = 12;
/// Solo JTDX: frequenza audio di trasmissione.
pub const SET_TX_FREQ: u32 = 50;
/// Solo JTDX: come il pulsante "CQ". Provato dal vivo: con il bool a `true` JTDX abilita Tx e trasmette il CQ.
pub const TRIGGER_CQ: u32 = 51;

/// Un datagramma decodificato: numero di schema e contenuto.
#[derive(Debug, Clone, PartialEq)]
pub struct Packet {
    pub schema: u32,
    pub message: Message,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Decode {
    pub id: Option<String>,
    pub is_new: bool,
    /// Millisecondi dalla mezzanotte UTC.
    pub time_ms: u32,
    pub snr: i32,
    pub delta_time: f64,
    pub delta_freq: u32,
    pub mode: Option<String>,
    pub message: Option<String>,
    pub low_confidence: bool,
    pub off_air: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Status {
    pub id: Option<String>,
    pub dial_freq: u64,
    pub mode: Option<String>,
    pub dx_call: Option<String>,
    pub report: Option<String>,
    pub tx_mode: Option<String>,
    pub tx_enabled: bool,
    pub transmitting: bool,
    pub decoding: bool,
    pub rx_df: u32,
    pub tx_df: u32,
    pub de_call: Option<String>,
    pub de_grid: Option<String>,
    pub dx_grid: Option<String>,
    pub tx_watchdog: bool,
    pub submode: Option<String>,
    pub fast_mode: bool,
    /// I campi seguenti mancano nei pacchetti di schemi o versioni piu' vecchi.
    pub special_op_mode: Option<u8>,
    pub freq_tolerance: Option<u32>,
    pub tr_period: Option<u32>,
    pub config_name: Option<String>,
    pub tx_message: Option<String>,
}

/// La coda di un `QsoLogged`, dopo i rapporti. Gli istanti sono millisecondi Unix UTC.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct QsoExtra {
    /// Ora di fine del QSO: e' il primo campo del pacchetto (prima del nominativo).
    pub time_off_ms: Option<i64>,
    pub tx_power: Option<String>,
    pub comments: Option<String>,
    pub name: Option<String>,
    pub time_on_ms: Option<i64>,
    pub operator_call: Option<String>,
    pub my_call: Option<String>,
    pub my_grid: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Message {
    Heartbeat {
        id: Option<String>,
        max_schema: u32,
        version: Option<String>,
        revision: Option<String>,
    },
    Status(Status),
    Decode(Decode),
    Clear {
        id: Option<String>,
    },
    QsoLogged {
        id: Option<String>,
        dx_call: Option<String>,
        dx_grid: Option<String>,
        tx_freq: u64,
        mode: Option<String>,
        report_sent: Option<String>,
        report_received: Option<String>,
        /// I campi che seguono i rapporti (potenza, commento, nome, ora di inizio, nominativi e locatore propri): mancano nei pacchetti troncati.
        extra: Option<QsoExtra>,
    },
    /// Il record ADIF del QSO registrato (tipo 12): in JTDX preceduto da una breve intestazione ADIF.
    LoggedAdif {
        id: Option<String>,
        adif: Option<String>,
    },
    Close {
        id: Option<String>,
    },
    /// Tipo non (ancora) modellato: i byte originali non si perdono.
    Unknown {
        kind: u32,
        id: Option<String>,
        raw: Vec<u8>,
    },
}

pub fn decode(datagram: &[u8]) -> Result<Packet, Error> {
    let mut r = Reader::new(datagram);
    let magic = r.u32()?;
    if magic != MAGIC {
        return Err(Error::BadMagic(magic));
    }
    let schema = r.u32()?;
    let kind = r.u32()?;
    let message = match kind {
        HEARTBEAT => {
            let id = r.utf8()?;
            let max_schema = r.u32()?;
            let version = r.utf8()?;
            // JTDX invia solo lo schema massimo e la versione: la revisione (WSJT-X) puo' mancare.
            let revision = if r.is_empty() { None } else { r.utf8()? };
            Message::Heartbeat { id, max_schema, version, revision }
        }
        STATUS => Message::Status(read_status(&mut r)?),
        DECODE => Message::Decode(read_decode(&mut r)?),
        CLEAR => Message::Clear { id: r.utf8()? },
        QSO_LOGGED => read_qso_logged(&mut r)?,
        LOGGED_ADIF => Message::LoggedAdif { id: r.utf8()?, adif: r.utf8()? },
        CLOSE => Message::Close { id: r.utf8()? },
        _ => Message::Unknown {
            kind,
            id: r.utf8().ok().flatten(),
            raw: datagram.to_vec(),
        },
    };
    Ok(Packet { schema, message })
}

fn read_decode(r: &mut Reader) -> Result<Decode, Error> {
    Ok(Decode {
        id: r.utf8()?,
        is_new: r.bool()?,
        time_ms: r.u32()?,
        snr: r.i32()?,
        delta_time: r.f64()?,
        delta_freq: r.u32()?,
        mode: r.utf8()?,
        message: r.utf8()?,
        low_confidence: r.bool()?,
        off_air: r.bool()?,
    })
}

fn read_status(r: &mut Reader) -> Result<Status, Error> {
    let mut s = Status {
        id: r.utf8()?,
        dial_freq: r.u64()?,
        mode: r.utf8()?,
        dx_call: r.utf8()?,
        report: r.utf8()?,
        tx_mode: r.utf8()?,
        tx_enabled: r.bool()?,
        transmitting: r.bool()?,
        decoding: r.bool()?,
        rx_df: r.u32()?,
        tx_df: r.u32()?,
        de_call: r.utf8()?,
        de_grid: r.utf8()?,
        dx_grid: r.utf8()?,
        tx_watchdog: r.bool()?,
        submode: r.utf8()?,
        fast_mode: r.bool()?,
        special_op_mode: None,
        freq_tolerance: None,
        tr_period: None,
        config_name: None,
        tx_message: None,
    };
    if r.is_empty() {
        return Ok(s);
    }
    s.special_op_mode = Some(r.u8()?);
    if r.is_empty() {
        return Ok(s);
    }
    s.freq_tolerance = Some(r.u32()?);
    if r.is_empty() {
        return Ok(s);
    }
    s.tr_period = Some(r.u32()?);
    if r.is_empty() {
        return Ok(s);
    }
    s.config_name = r.utf8()?;
    if r.is_empty() {
        return Ok(s);
    }
    s.tx_message = r.utf8()?;
    Ok(s)
}

fn read_qso_logged(r: &mut Reader) -> Result<Message, Error> {
    let id = r.utf8()?;
    let time_off_ms = read_qdatetime(r)?;
    let dx_call = r.utf8()?;
    let dx_grid = r.utf8()?;
    let tx_freq = r.u64()?;
    let mode = r.utf8()?;
    let report_sent = r.utf8()?;
    let report_received = r.utf8()?;
    let extra = if r.is_empty() {
        None
    } else {
        let tx_power = r.utf8()?;
        let comments = r.utf8()?;
        let name = r.utf8()?;
        let time_on_ms = read_qdatetime(r)?;
        let operator_call = r.utf8()?;
        let my_call = r.utf8()?;
        let my_grid = r.utf8()?;
        Some(QsoExtra { time_off_ms, tx_power, comments, name, time_on_ms, operator_call, my_call, my_grid })
    };
    Ok(Message::QsoLogged { id, dx_call, dx_grid, tx_freq, mode, report_sent, report_received, extra })
}

/// Giorno giuliano del 1970-01-01 (la data zero di `QDate`).
const UNIX_EPOCH_JULIAN_DAY: i64 = 2_440_588;

/// `QDateTime`: giorno giuliano (i64), millisecondi (u32), tipo di fuso (u8),
/// piu' lo scostamento (i32) se il fuso e' "OffsetFromUTC" (2). Restituisce i millisecondi Unix
/// (senza tener conto dello scostamento: JTDX manda sempre UTC); `None` se la data e' nulla.
fn read_qdatetime(r: &mut Reader) -> Result<Option<i64>, Error> {
    let julian_day = r.i64()?;
    let ms = r.u32()?;
    if r.u8()? == 2 {
        r.i32()?;
    }
    // `QDateTime` nullo: giorno 0 (e ora non valida)
    if julian_day == 0 || ms == u32::MAX {
        return Ok(None);
    }
    Ok(julian_day.checked_sub(UNIX_EPOCH_JULIAN_DAY).and_then(|d| d.checked_mul(86_400_000)).and_then(|d| d.checked_add(i64::from(ms))))
}

pub(crate) fn command_header(kind: u32, schema: u32, id: &str) -> Writer {
    let mut w = Writer::new();
    w.u32(MAGIC);
    w.u32(schema);
    w.u32(kind);
    w.utf8(Some(id));
    w
}

/// Ferma la trasmissione; con `auto_tx_only` disattiva solo la trasmissione automatica.
pub fn encode_halt_tx(id: &str, schema: u32, auto_tx_only: bool) -> Vec<u8> {
    let mut w = command_header(HALT_TX, schema, id);
    w.bool(auto_tx_only);
    w.into_bytes()
}

/// Imposta il testo libero; con `send` lo trasmette subito.
pub fn encode_free_text(id: &str, schema: u32, text: &str, send: bool) -> Vec<u8> {
    let mut w = command_header(FREE_TEXT, schema, id);
    w.utf8(Some(text));
    w.bool(send);
    w.into_bytes()
}

/// Risponde a una stazione: rimanda a JTDX il decode originale, come un doppio clic.
pub fn encode_reply(id: &str, schema: u32, decode: &Decode, modifiers: u8) -> Vec<u8> {
    let mut w = command_header(REPLY, schema, id);
    w.u32(decode.time_ms);
    w.i32(decode.snr);
    w.f64(decode.delta_time);
    w.u32(decode.delta_freq);
    w.utf8(decode.mode.as_deref());
    w.utf8(decode.message.as_deref());
    w.bool(decode.low_confidence);
    w.u8(modifiers);
    w.into_bytes()
}

/// Imposta la frequenza audio di trasmissione (Hz); non fa trasmettere.
pub fn encode_set_tx_freq(id: &str, schema: u32, hz: i32) -> Vec<u8> {
    let mut w = command_header(SET_TX_FREQ, schema, id);
    w.i32(hz);
    w.into_bytes()
}

/// Come il pulsante CQ di JTDX (con `modifier` per esempio "DX"); fa trasmettere.
pub fn encode_trigger_cq(id: &str, schema: u32, modifier: &str, flag: bool) -> Vec<u8> {
    let mut w = command_header(TRIGGER_CQ, schema, id);
    w.utf8(Some(modifier));
    w.bool(flag);
    w.into_bytes()
}

/// Chiede a JTDX di rimandare i decode correnti (utile a chi si collega a sessione iniziata).
pub fn encode_replay(id: &str, schema: u32) -> Vec<u8> {
    command_header(REPLAY, schema, id).into_bytes()
}
