//! Protocollo UDP di JTDX/WSJT-X: pacchetti big-endian in formato Qt `QDataStream`.

pub mod message;
pub mod wire;

pub use message::{decode, Decode, Message, Packet, Status};

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
