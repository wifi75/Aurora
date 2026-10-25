//! Protocollo UDP di JTDX/WSJT-X: pacchetti big-endian in formato Qt `QDataStream`.

pub mod emit;
pub mod message;
pub mod request;
pub mod wire;

pub use emit::{
    encode_clear, encode_close, encode_decode, encode_heartbeat, encode_logged_adif, encode_qso_logged, encode_status, QsoLoggedData, ADIF_HEADER, HEARTBEAT_SECS, INITIAL_SCHEMA,
    MAX_SCHEMA,
};
pub use request::{decode_request, Addressed, Request};
pub use message::{decode, encode_free_text, encode_halt_tx, encode_replay, encode_reply, encode_set_tx_freq, encode_trigger_cq, Decode, Message, Packet, QsoExtra, Status};

/// Numero magico all'inizio di ogni datagramma.
pub const MAGIC: u32 = 0xadbccbda;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// Il datagramma finisce prima del previsto.
    Truncated,
    /// Il numero magico non e' quello del protocollo.
    BadMagic(u32),
    /// Una stringa non e' UTF-8 valido.
    BadUtf8,
    /// La lunghezza dichiarata di una stringa supera i byte disponibili.
    TooLong(u32),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Truncated => write!(f, "datagramma troncato"),
            Error::BadMagic(m) => write!(f, "numero magico errato: {m:#010x}"),
            Error::BadUtf8 => write!(f, "stringa non UTF-8"),
            Error::TooLong(n) => write!(f, "lunghezza stringa implausibile: {n}"),
        }
    }
}

impl std::error::Error for Error {}
