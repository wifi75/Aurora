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
    /// E' una **ritrasmissione**: nello stesso periodo la sequenza aveva gia' dato un altro messaggio (`m_restart` di JTDX, `Retransmitting` in
    /// `ALL.TXT`); chi trasmette deve fermare quello in corso e ripartire da questo (dalla coda, se e' gia' passato l'inizio del periodo).
    pub retransmit: bool,
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

/// Quante volte di fila si manda lo stesso messaggio prima di rinunciare (`None` = contatore spento, senza tetto).
///
/// Sono i quattro contatori della scheda AutoSeq di JTDX (`SeqAnswerCQCount`, `SeqAnswerInCallCount`, `SeqSentRReportCount`, `SeqSentRR7373Count`):
/// il valore e' il numero massimo di trasmissioni, la successiva non parte. Di fabbrica in JTDX **sono tutti spenti** e i valori sarebbero 2 / 3 / 5 / 4
/// (`read_settings`, `REA 0x50b240`); qui il predefinito e' la configurazione del proprietario (rapporto R e chiusura accesi a 5, gli altri spenti).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RepeatLimits {
    /// Ho risposto a un CQ (Tx1, o Tx2 con «Salta Tx1») e non risponde nessuno (`AnswerCQ`). Non vale per le stazioni volute e per i nuovi DXCC.
    pub answer_cq: Option<u32>,
    /// Ho chiamato CQ, mi ha risposto e io ho mandato il rapporto (`AnswerInCall`).
    pub answer_in_call: Option<u32>,
    /// Ho mandato R+rapporto (Tx3) e non arriva la chiusura (`SentRReport`).
    pub sent_r_report: Option<u32>,
    /// Ho mandato RR73/RRR (Tx4) o il 73 (Tx5) e non arriva niente (`SentRR7373`).
    pub closing: Option<u32>,
}

impl RepeatLimits {
    /// I valori di fabbrica di JTDX con i quattro contatori accesi (2 / 3 / 5 / 4).
    pub fn jtdx_factory_values() -> RepeatLimits {
        RepeatLimits {
            answer_cq: Some(2),
            answer_in_call: Some(3),
            sent_r_report: Some(5),
            closing: Some(4),
        }
    }

    /// Tutti spenti: si ripete fino al watchdog.
    pub fn off() -> RepeatLimits {
        RepeatLimits {
            answer_cq: None,
            answer_in_call: None,
            sent_r_report: None,
            closing: None,
        }
    }
}

impl Default for RepeatLimits {
    fn default() -> Self {
        RepeatLimits {
            answer_cq: None,
            answer_in_call: None,
            sent_r_report: Some(5),
            closing: Some(5),
        }
    }
}

/// Opzioni della sequenza (le stesse che l'utente cambia in JTDX).
#[derive(Clone, Debug, PartialEq)]
pub struct Options {
    /// AutoSeq: i messaggi avanzano da soli in base a cio' che arriva.
    pub auto_seq: bool,
    /// Livello di AutoSeq (`CallMode`): 0 nessuna scelta di chi chiamare e avanza subito, 1 prende il primo che chiama, 2 decide all'inizio del
    /// periodo di trasmissione, 3 a fine decodifica con la trasmissione che riparte se il messaggio cambia (di fabbrica 3). Con `call_prio_cq`
    /// 2 e 3 diventano i livelli 6 e 7 (ricerca tra i CQ degli altri).
    pub call_mode: u8,
    /// «Priorita' di chiamata e ricerca CQ» (`CallPriorityCQ`): livelli 6 e 7.
    pub call_prio_cq: bool,
    /// «Salta Tx1»: rispondendo a un CQ si parte da Tx2 (rapporto) invece che da Tx1 (locatore).
    pub skip_tx1: bool,
    /// Senza effetto: in JTDX «Bypassa» e' il filtro dei testi (impostazione dell'interfaccia), non un permesso di avvio tardivo. L'avvio e' sempre
    /// possibile entro la durata del messaggio (13,64 s in FT8, 6,04 s in FT4).
    pub bypass_tx1: bool,
    /// «1 QSO»: dopo un QSO (o un tentativo fallito) la trasmissione si ferma.
    pub single_qso: bool,
    /// «Risp B4»: si risponde anche a chi si e' gia' lavorato.
    pub answer_worked_before: bool,
    /// «Chiama gia' lavorati» (`CallWorkedB4`): nella ricerca tra i CQ si chiamano anche i gia' lavorati.
    pub call_worked_before: bool,
    /// «Chiama prima i nuovi nominativi» (`CallHigherNewCall`): soglia 8 nella ricerca tra i CQ.
    pub call_higher_new_call: bool,
    /// «Priorita' al rapporto»: a parita' si preferisce chi ha mandato il rapporto.
    pub report_priority: bool,
    /// «Distanza massima»: a parita' si preferisce il piu' lontano.
    pub max_distance: bool,
    /// Almeno una evidenziazione di «nuovo» (DXCC, locatore, prefisso, nominativo) e' accesa: senza, JTDX risponde a chiunque.
    pub highlights: bool,
    /// «Direzione CQ rigorosa» (`StrictDirectionalCQ`): la direzione del CQ vale anche per i desiderati e per i nuovi DXCC.
    pub strict_directional_cq: bool,
    /// Casella RRR: il Tx4 e' `RRR` invece di `RR73`.
    pub use_rrr: bool,
    /// «Marker RR73»: i messaggi di chiusura fra altre stazioni (RR73, 73) fanno entrare la stazione tra i candidati (RFIN, di fabbrica acceso).
    pub rr73_marker: bool,
    pub limits: RepeatLimits,
    /// Minuti di trasmissione senza intervento dopo i quali la trasmissione si ferma (`TxWatchdogTimer`, 0 = senza watchdog). Conta i minuti UTC
    /// solo se si e' trasmesso negli ultimi 32 s (FT8) o 16 s (FT4); si azzera con un messaggio nuovo, con un intervento o con RCALL/RREPORT.
    pub watchdog_min: u32,
    /// `SeqHaltTxReplyOther`: ferma la trasmissione se il DX risponde a un altro (di fabbrica acceso); spento, si ferma solo se il DX parla
    /// entro la banda di guardia dalla nostra frequenza di trasmissione (FT8 51 Hz, FT4 84 Hz).
    pub abandon_if_dx_answers_other: bool,
    /// Registrazione automatica del QSO (`AutoQSOLogging`): a fine QSO (FIN, 73 inviato) JTDX **ferma la trasmissione**. Spento, passa da solo al
    /// prossimo chiamante o al CQ.
    pub autolog: bool,
    /// Periodo di trasmissione di partenza: pari (`TxMinuteButton` acceso) o dispari. Dopo la
    /// scelta di un DX segue quello opposto al suo.
    pub tx_even: bool,
    /// Frequenza audio di trasmissione.
    pub tx_freq_hz: u32,
    /// Tx = Rx: si trasmette sulla frequenza del DX (`LockTxFreq`).
    pub follow_dx_freq: bool,
    pub report: ReportPolicy,
    /// Ritardo massimo, in secondi dall'inizio del periodo, oltre il quale una trasmissione non
    /// parte piu' in quel periodo. `None`: la durata del messaggio come in JTDX (13,64 s in FT8, 6,04 s in FT4, `guiUpdate` `REA 0x4c4350`):
    /// l'avvio tardivo trasmette la coda.
    pub late_start_s: Option<f64>,
    /// Direzione dei propri CQ («DX», «EU», «NA»...), `CQdirection` di JTDX.
    pub cq_modifier: Option<String>,
    /// Continente e prefisso propri (per la direzione dei CQ); vuoti = ignoti.
    pub my_continent: String,
    pub my_prefix: String,
    /// Modo Hound (DXpedition): niente Salta Tx1, niente CQ, la chiusura e' l'RR73 della spedizione.
    pub hound: bool,
}

impl Default for Options {
    fn default() -> Self {
        Options {
            auto_seq: true,
            call_mode: 3,
            call_prio_cq: false,
            skip_tx1: false,
            bypass_tx1: false,
            single_qso: false,
            answer_worked_before: false,
            call_worked_before: false,
            call_higher_new_call: false,
            report_priority: false,
            max_distance: false,
            highlights: false,
            strict_directional_cq: false,
            use_rrr: false,
            rr73_marker: true,
            limits: RepeatLimits::default(),
            watchdog_min: 6,
            abandon_if_dx_answers_other: true,
            autolog: true,
            tx_even: true,
            tx_freq_hz: 1500,
            follow_dx_freq: false,
            report: ReportPolicy::Measured,
            late_start_s: None,
            cq_modifier: None,
            my_continent: String::new(),
            my_prefix: String::new(),
            hound: false,
        }
    }
}

impl Options {
    /// Ritardo di avvio massimo per il modo dato: la durata del messaggio (`txDuration` di `guiUpdate`).
    pub fn late_limit_s(&self, mode: Mode) -> f64 {
        self.late_start_s.unwrap_or(match mode {
            Mode::Ft8 => 13.64,
            Mode::Ft4 => 6.04,
        })
    }

    /// Banda di guardia tra la nostra trasmissione e un altro segnale (`m_nguardfreq`): FT8 51 Hz, FT4 84 Hz.
    pub fn guard_hz(&self, mode: Mode) -> i32 {
        match mode {
            Mode::Ft8 => 51,
            Mode::Ft4 => 84,
        }
    }
}

/// Cio' che il registro e la tabella dei paesi dicono di una stazione (le evidenziazioni di JTDX): serve alla scelta di chi chiamare.
#[derive(Clone, Debug, Default)]
pub struct CallerInfo {
    /// Continente («EU», «NA»...); vuoto = ignoto. JTDX **non sceglie mai** una stazione senza continente: qui l'ignoto vale «??».
    pub continent: String,
    /// Prefisso principale del paese.
    pub prefix: String,
    /// Priorita' 0-31 come `displayDecodedText`: 30-31 nuova zona CQ, 28-29 su banda, 26-27 zona ITU, 24-25 su banda, 22-23 nuovo DXCC, 20-21 su banda,
    /// 15-16 locatore, 13-14 su banda, 11-12 prefisso, 9-10 su banda, 7-8 nominativo, 5-6 su banda; i desiderati valgono 2-4 e 17-19; 0 = niente di nuovo.
    pub priority: u8,
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
    /// Il DX ha risposto a un altro vicino alla nostra frequenza (`readFromStdout`: «not owner of the frequency or reply to other»).
    ReplyOther,
    /// Fine del QSO con la registrazione automatica (`autoStopTx`: FIN, 73 inviato, RR73 senza risposta).
    QsoFinished,
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
