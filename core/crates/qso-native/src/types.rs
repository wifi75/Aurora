//! Tipi in ingresso e in uscita della sequenza.

use crate::slot::Mode;
use std::fmt;

/// La propria stazione.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Station {
    pub call: String,
    /// Locatore di 4 o 6 caratteri; nei messaggi se ne usano i primi 4.
    pub grid: String,
}

impl Station {
    pub fn new(call: &str, grid: &str) -> Self {
        Station {
            call: call.to_string(),
            grid: grid.to_string(),
        }
    }
}

/// Un messaggio da trasmettere nel periodo indicato.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Tx {
    /// Quale dei sei messaggi: 1..=6.
    pub n: u8,
    pub text: String,
    /// Frequenza audio di trasmissione.
    pub freq_hz: u32,
    /// Il periodo e' pari: il chiamante trasmette solo nei periodi di questa parita'.
    pub even_slot: bool,
}

/// Un messaggio decodificato.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Decoded {
    pub text: String,
    pub snr: i32,
    /// Frequenza audio del segnale ricevuto.
    pub freq_hz: u32,
}

impl Decoded {
    pub fn new(text: &str, snr: i32, freq_hz: u32) -> Self {
        Decoded {
            text: text.to_string(),
            snr,
            freq_hz,
        }
    }
}

/// Come si sceglie il rapporto da mandare al DX.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReportPolicy {
    /// L'SNR dell'ultimo messaggio ricevuto dal DX (come JTDX).
    Measured,
    /// Sempre lo stesso valore.
    Fixed(i32),
}

/// Quante volte di fila si manda lo stesso messaggio prima di rinunciare (`None` = senza tetto).
///
/// Sono i quattro contatori della scheda AutoSeq di JTDX; il tetto e' il numero massimo di
/// trasmissioni, la sesta non parte.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RepeatLimits {
    /// Ho risposto a un CQ (Tx1, o Tx2 con «Salta Tx1») e non risponde nessuno (`AnswerCQ`).
    pub answer_cq: Option<u32>,
    /// Ho chiamato CQ, mi ha risposto e io ho mandato il rapporto (`AnswerInCall`).
    pub answer_in_call: Option<u32>,
    /// Ho mandato R+rapporto (Tx3) e non arriva la chiusura (`SentRReport`).
    pub sent_r_report: Option<u32>,
    /// Ho mandato RR73/RRR (Tx4) e non arriva il 73 (`SentRR7373`).
    pub closing: Option<u32>,
}

impl Default for RepeatLimits {
    fn default() -> Self {
        RepeatLimits {
            answer_cq: Some(5),
            answer_in_call: Some(5),
            sent_r_report: Some(5),
            closing: Some(3),
        }
    }
}

/// Opzioni della sequenza (le stesse che l'utente cambia in JTDX).
#[derive(Clone, Debug, PartialEq)]
pub struct Options {
    /// AutoSeq: i messaggi avanzano da soli in base a cio' che arriva.
    pub auto_seq: bool,
    /// «Salta Tx1»: rispondendo a un CQ si parte da Tx2 (rapporto) invece che da Tx1 (locatore).
    pub skip_tx1: bool,
    /// «Bypassa Tx1»: la prima trasmissione dopo la scelta di un DX puo' partire anche tardi
    /// (fino ai tre quarti del periodo) invece che entro `late_start_s`.
    pub bypass_tx1: bool,
    /// «1 QSO»: dopo un QSO (o un tentativo fallito) la trasmissione si ferma.
    pub single_qso: bool,
    /// «Risp B4»: si risponde anche a chi si e' gia' lavorato.
    pub answer_worked_before: bool,
    /// Casella RRR: il Tx4 e' `RRR` invece di `RR73`.
    pub use_rrr: bool,
    pub limits: RepeatLimits,
    /// Minuti senza progressi dopo i quali la trasmissione si ferma (0 = senza watchdog).
    pub watchdog_min: u32,
    /// Si rinuncia subito se il DX, non ancora in contatto con noi, risponde a un altro
    /// (`SeqHaltTxReplyOther`).
    pub abandon_if_dx_answers_other: bool,
    /// Periodo di trasmissione di partenza: pari (`TxMinuteButton` acceso) o dispari. Dopo la
    /// scelta di un DX segue quello opposto al suo.
    pub tx_even: bool,
    /// Frequenza audio di trasmissione.
    pub tx_freq_hz: u32,
    /// Tx = Rx: si trasmette sulla frequenza del DX (`LockTxFreq`).
    pub follow_dx_freq: bool,
    pub report: ReportPolicy,
    /// Ritardo massimo, in secondi dall'inizio del periodo, oltre il quale una trasmissione non
    /// parte piu' in quel periodo. `None`: 2 s in FT8, 1 s in FT4 (valore da verificare, vedi la
    /// documentazione del crate).
    pub late_start_s: Option<f64>,
    /// Per quanto un chiamante in coda resta valido senza essere risentito.
    pub queue_ttl_s: f64,
    /// Chiamanti in coda al massimo (Decodium: 20).
    pub queue_max: usize,
}

impl Default for Options {
    fn default() -> Self {
        Options {
            auto_seq: true,
            skip_tx1: false,
            bypass_tx1: false,
            single_qso: false,
            answer_worked_before: false,
            use_rrr: false,
            limits: RepeatLimits::default(),
            watchdog_min: 6,
            abandon_if_dx_answers_other: true,
            tx_even: true,
            tx_freq_hz: 1500,
            follow_dx_freq: false,
            report: ReportPolicy::Measured,
            late_start_s: None,
            queue_ttl_s: 120.0,
            queue_max: 20,
        }
    }
}

impl Options {
    /// Ritardo di avvio massimo per il modo dato.
    pub fn late_limit_s(&self, mode: Mode) -> f64 {
        self.late_start_s.unwrap_or(match mode {
            Mode::Ft8 => 2.0,
            Mode::Ft4 => 1.0,
        })
    }
}

/// A che punto e' la sequenza (le fasi di `m_QSOProgress` di JTDX).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    /// Niente da trasmettere.
    Idle,
    /// Si chiama CQ (Tx6).
    Cq,
    /// Tx1 (locatore).
    Reply,
    /// Tx2 (rapporto).
    Report,
    /// Tx3 (R+rapporto).
    RogerReport,
    /// Tx4 (RR73 o RRR).
    Closing,
    /// Tx5 (73).
    SignOff,
}

impl Phase {
    pub fn from_ntx(n: u8) -> Phase {
        match n {
            1 => Phase::Reply,
            2 => Phase::Report,
            3 => Phase::RogerReport,
            4 => Phase::Closing,
            5 => Phase::SignOff,
            6 => Phase::Cq,
            _ => Phase::Idle,
        }
    }
}

/// Un QSO concluso, pronto per il log (`aurora_core::QsoRecord` si ricava da qui).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct QsoRecord {
    pub call: String,
    /// Locatore del DX (4 caratteri), vuoto se non e' stato ricevuto.
    pub grid: String,
    /// Rapporto che abbiamo mandato, come scritto nel messaggio («-07»).
    pub rst_sent: String,
    /// Rapporto ricevuto dal DX («-10»).
    pub rst_rcvd: String,
    pub mode: Mode,
    /// Inizio del primo periodo del QSO (millisecondi UTC dall'epoca UNIX).
    pub time_on_ms: u64,
    /// Inizio del periodo in cui il QSO e' stato dato per concluso.
    pub time_off_ms: u64,
    /// Frequenza audio di trasmissione.
    pub tx_freq_hz: u32,
    /// Frequenza audio del DX.
    pub rx_freq_hz: u32,
}

/// Perche' un tentativo di QSO e' finito senza esito.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AbandonReason {
    /// Il tetto di ripetizioni di quel messaggio e' stato raggiunto senza risposta.
    NoReply,
    /// Il DX ha risposto a un altro (`SeqHaltTxReplyOther`).
    DxAnsweredOther,
    /// Scelto un altro DX a mano.
    Replaced,
    /// Cancellato a mano.
    Cleared,
}

/// Perche' la trasmissione si e' fermata.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StopReason {
    /// `halt()`: lo ha voluto l'operatore.
    Halt,
    /// «1 QSO»: concluso (o fallito) il QSO.
    SingleQso,
    /// Il watchdog e' scaduto.
    Watchdog,
    /// Non c'e' piu' nulla da fare e non si stava chiamando CQ.
    NothingToDo,
}

/// Cio' che la sequenza comunica a chi la usa.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Event {
    /// Un QSO e' concluso: va registrato (una sola volta per QSO).
    QsoComplete(QsoRecord),
    /// Cambia la fase o il DX in collegamento.
    StateChanged {
        phase: Phase,
        dx: Option<String>,
        ntx: u8,
    },
    /// Un chiamante in piu' e' stato messo in coda.
    CallerQueued { call: String },
    /// Un chiamante e' stato ignorato (gia' lavorato e «Risp B4» spento).
    CallerIgnored { call: String },
    /// Il tentativo con questo DX e' finito senza QSO.
    Abandoned { call: String, reason: AbandonReason },
    /// La trasmissione automatica si e' fermata.
    Stopped(StopReason),
}

/// Errori nell'uso della sequenza.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SeqError {
    /// Il proprio nominativo non e' valido.
    InvalidCall(String),
    /// Il proprio locatore non e' valido.
    InvalidGrid(String),
    /// Il messaggio scelto non ha un mittente a cui rispondere.
    NotReplyable(String),
    /// Si e' provato a rispondere a se stessi.
    OwnCall,
    /// Modificatore di CQ non ammesso.
    InvalidModifier(String),
    /// Numero di messaggio fuori da 1..=6.
    InvalidTxNumber(u8),
    /// Il messaggio richiede un DX in collegamento.
    NoDx,
}

impl fmt::Display for SeqError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SeqError::InvalidCall(c) => write!(f, "nominativo non valido: {c}"),
            SeqError::InvalidGrid(g) => write!(f, "locatore non valido: {g}"),
            SeqError::NotReplyable(t) => write!(f, "nessun mittente a cui rispondere in «{t}»"),
            SeqError::OwnCall => write!(f, "il messaggio e' del proprio nominativo"),
            SeqError::InvalidModifier(m) => write!(f, "modificatore di CQ non ammesso: {m}"),
            SeqError::InvalidTxNumber(n) => write!(f, "numero di messaggio fuori da 1..6: {n}"),
            SeqError::NoDx => write!(f, "nessun DX in collegamento"),
        }
    }
}

impl std::error::Error for SeqError {}
