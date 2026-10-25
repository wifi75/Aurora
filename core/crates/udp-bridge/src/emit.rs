//! Costruzione dei datagrammi che JTDX manda al suo primo server UDP (`MessageClient`, `REA 0x56abc0` e vicini):
//! Heartbeat, Status, Decode, Clear, QSO Logged, Logged ADIF e Close. Stesso ordine dei campi e stessi tipi di JTDX 2.2.159;
//! i datagrammi si rileggono con [`crate::decode`].

use crate::message::{command_header, Decode, Status, CLEAR, CLOSE, DECODE, HEARTBEAT, LOGGED_ADIF, QSO_LOGGED, STATUS};

/// Schema massimo che JTDX dichiara nel Heartbeat (`schema_number`, con Qt 5.4 o piu' recente).
pub const MAX_SCHEMA: u32 = 3;
/// Schema di partenza di `MessageClient`: 2 finche' il server non risponde con uno piu' alto.
pub const INITIAL_SCHEMA: u32 = 2;
/// Intervallo del Heartbeat (`NetworkMessage::pulse`, 15 s; `REA 0x56e800`: `QTimer::start(15000)`).
pub const HEARTBEAT_SECS: u64 = 15;
/// Intestazione che JTDX antepone al record ADIF di `LoggedADIF` (`REA 0x56a6d0`).
pub const ADIF_HEADER: &str = "\n<adif_ver:5>3.1.0\n<programid:4>JTDX\n<EOH>\n";

/// Heartbeat: schema massimo accettato e versione; JTDX non manda la revisione (WSJT-X si').
pub fn encode_heartbeat(id: &str, schema: u32, max_schema: u32, version: &str) -> Vec<u8> {
    let mut w = command_header(HEARTBEAT, schema, id);
    w.u32(max_schema);
    w.utf8(Some(version));
    w.into_bytes()
}

/// Status. Dopo `fast_mode` JTDX manda un solo byte (`tx_first`: tocca a noi il primo periodo): e' il campo che il decoder chiama
/// `special_op_mode`. I campi seguenti (`freq_tolerance`, `tr_period`, `config_name`, `tx_message`) sono di WSJT-X: si scrivono solo
/// se presenti, in ordine, e ciascuno solo se ci sono tutti i precedenti.
pub fn encode_status(schema: u32, s: &Status) -> Vec<u8> {
    let mut w = command_header(STATUS, schema, s.id.as_deref().unwrap_or_default());
    w.u64(s.dial_freq);
    w.utf8(s.mode.as_deref());
    w.utf8(s.dx_call.as_deref());
    w.utf8(s.report.as_deref());
    w.utf8(s.tx_mode.as_deref());
    w.bool(s.tx_enabled);
    w.bool(s.transmitting);
    w.bool(s.decoding);
    w.u32(s.rx_df);
    w.u32(s.tx_df);
    w.utf8(s.de_call.as_deref());
    w.utf8(s.de_grid.as_deref());
    w.utf8(s.dx_grid.as_deref());
    w.bool(s.tx_watchdog);
    w.utf8(s.submode.as_deref());
    w.bool(s.fast_mode);
    if let Some(special) = s.special_op_mode {
        w.u8(special);
        if let Some(tolerance) = s.freq_tolerance {
            w.u32(tolerance);
            if let Some(period) = s.tr_period {
                w.u32(period);
                if s.config_name.is_some() || s.tx_message.is_some() {
                    w.utf8(s.config_name.as_deref());
                    if s.tx_message.is_some() {
                        w.utf8(s.tx_message.as_deref());
                    }
                }
            }
        }
    }
    w.into_bytes()
}

/// Decode: un messaggio ricevuto. `delta_time` va come `double` (il `float` di Qt si scrive a 64 bit).
pub fn encode_decode(schema: u32, d: &Decode) -> Vec<u8> {
    let mut w = command_header(DECODE, schema, d.id.as_deref().unwrap_or_default());
    w.bool(d.is_new);
    w.u32(d.time_ms);
    w.i32(d.snr);
    w.f64(d.delta_time);
    w.u32(d.delta_freq);
    w.utf8(d.mode.as_deref());
    w.utf8(d.message.as_deref());
    w.bool(d.low_confidence);
    w.bool(d.off_air);
    w.into_bytes()
}

/// Clear: svuota le attivita' di banda (il pulsante «Erase»).
pub fn encode_clear(id: &str, schema: u32) -> Vec<u8> {
    command_header(CLEAR, schema, id).into_bytes()
}

/// Close: alla chiusura del programma.
pub fn encode_close(id: &str, schema: u32) -> Vec<u8> {
    command_header(CLOSE, schema, id).into_bytes()
}

/// Un QSO registrato, nell'ordine dei campi di `MessageClient::qso_logged` (`REA 0x569b60`). Gli istanti sono millisecondi Unix UTC.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct QsoLoggedData {
    pub time_off_ms: Option<i64>,
    pub dx_call: String,
    pub dx_grid: String,
    /// Frequenza di dial in Hz (JTDX: la frequenza nominale, senza il tono di trasmissione).
    pub dial_freq: u64,
    pub mode: String,
    pub report_sent: String,
    pub report_received: String,
    pub tx_power: String,
    pub comments: String,
    pub name: String,
    pub time_on_ms: Option<i64>,
    pub operator_call: String,
    pub my_call: String,
    pub my_grid: String,
}

/// QSO Logged (tipo 5): ora di fine, nominativo, locatore, frequenza, modo, rapporti, potenza, commento, nome, ora di inizio,
/// operatore, nominativo e locatore propri.
pub fn encode_qso_logged(id: &str, schema: u32, q: &QsoLoggedData) -> Vec<u8> {
    let mut w = command_header(QSO_LOGGED, schema, id);
    w.qdatetime_utc(q.time_off_ms);
    w.utf8(Some(&q.dx_call));
    w.utf8(Some(&q.dx_grid));
    w.u64(q.dial_freq);
    w.utf8(Some(&q.mode));
    w.utf8(Some(&q.report_sent));
    w.utf8(Some(&q.report_received));
    w.utf8(Some(&q.tx_power));
    w.utf8(Some(&q.comments));
    w.utf8(Some(&q.name));
    w.qdatetime_utc(q.time_on_ms);
    w.utf8(Some(&q.operator_call));
    w.utf8(Some(&q.my_call));
    w.utf8(Some(&q.my_grid));
    w.into_bytes()
}

/// Logged ADIF (tipo 12): l'intestazione ADIF di JTDX seguita dal record, in un'unica stringa.
pub fn encode_logged_adif(id: &str, schema: u32, adif_record: &str) -> Vec<u8> {
    let mut w = command_header(LOGGED_ADIF, schema, id);
    w.utf8(Some(&format!("{ADIF_HEADER}{adif_record}")));
    w.into_bytes()
}
