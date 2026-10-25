//! Sequenza automatica dei QSO nativa di Aurora.
//!
//! Crate **puro**: nessun audio, nessun orologio di sistema, nessun I/O. Contiene solo stato e
//! decisioni, cosi' che ogni regola si possa provare con una sequenza di messaggi decodificati.
//!
//! - [`Sequencer`]: la macchina a stati (i sei messaggi Tx1-Tx6, AutoSeq, ripetizioni, chiusura,
//!   coda dei chiamanti, watchdog);
//! - [`message`]: lettura dei messaggi e confronto dei nominativi;
//! - [`stdmsg`]: composizione dei messaggi standard (`genStdMsgs` e `genCQMsg` di JTDX);
//! - [`report`]: rapporto segnale («-07», «R-07», «+05»);
//! - [`Mode`] e [`Slot`]: FT8 (15 s) e FT4 (7,5 s), periodi pari e dispari.
//!
//! Il comportamento di riferimento e' quello di JTDX descritto in `docs/logiche-jtdx.md`; il
//! codice non dipende da JTDX ne' da nessun altro programma: la logica e' stata riscritta dai
//! sorgenti pubblici (GPL 3).
//!
//! # Uso
//!
//! A ogni fine periodo il chiamante passa i messaggi decodificati a [`Sequencer::on_decodes`], poi
//! all'inizio del periodo di trasmissione chiama [`Sequencer::next_tx`] e, se riceve un messaggio,
//! lo trasmette. Gli esiti (QSO concluso, cambi di stato, rinunce) si leggono con
//! [`Sequencer::drain_events`].

pub mod message;
pub mod report;
pub mod sequencer;
pub mod slot;
pub mod stdmsg;
pub mod types;

pub use message::{Body, Message, base_call, is_std_call, parse, same_station};
pub use report::{format_r_report, format_report, parse_r_report, parse_report};
pub use sequencer::Sequencer;
pub use slot::{Mode, Slot};
pub use stdmsg::{StdMessages, gen_cq, gen_std_msgs};
pub use types::{
    AbandonReason, Decoded, Event, Options, Phase, QsoRecord, RepeatLimits, ReportPolicy, SeqError,
    Station, StopReason, Tx,
};
