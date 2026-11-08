/// Comandi dall'esterno verso il motore.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Command {
    HaltTx,
    Reply { decode_id: u64 },
    /// Tasto destro su una stazione che chiama: in coda se c'e' un QSO in corso, altrimenti come `Reply`; di nuovo sulla stessa la toglie.
    QueueReply { decode_id: u64 },
    /// Messaggio libero; con `send` JTDX lo trasmette subito (serve la trasmissione abilitata).
    SendFreeText { text: String, send: bool },
    /// Come il pulsante CQ di JTDX (`modifier` per esempio "DX"); fa trasmettere.
    CallCq { modifier: String },
    /// Frequenza audio di trasmissione, in Hz; non fa trasmettere.
    SetTxFrequency { hz: u32 },
    /// I campi «DX Call» e «Locatore» scritti a mano: imposta la stazione e rigenera i messaggi; non fa trasmettere.
    SetDx { call: String, grid: String },
    /// Il rapporto da inviare scelto a mano, in dB.
    SetReport { db: i32 },
    /// «Tune»: portante sulla frequenza Tx (acceso) o ferma (spento).
    Tune { on: bool },
    /// «Log QSO» a mano.
    LogQso,
    /// Un comando della pagina Fox.
    Fox(FoxCommand),
    /// La sequenza di contest scelta nella pagina Contest (`None` = QSO normali).
    Contest(Option<ContestSetup>),
}

/// La sequenza di contest da usare nel motore: attivita' (come la «Special operating activity» di WSJT-X), parola del CQ, scambio fisso e
/// seriale. La sceglie la pagina Contest dalle regole del contest attivo.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ContestSetup {
    /// `standard`, `na-vhf`, `arrl-digi`, `ww-digi`, `eu-vhf`, `field-day`, `roundup`.
    pub activity: String,
    /// Parola del CQ (`TEST`, `WW`, `FD`...); vuota = quella dell'attivita'.
    pub cq_word: String,
    /// Scambio fisso: Field Day `1D DX`; Roundup lo stato o `#` per il seriale.
    pub exchange: String,
    /// Il prossimo numero progressivo da inviare.
    pub next_serial: u32,
    /// Locatore a 6 caratteri per EU VHF (vuoto = quello della stazione).
    pub grid6: String,
    /// Chiusura con RRR invece di RR73.
    pub rrr: bool,
}

/// I comandi della pagina Fox.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FoxCommand {
    /// Mette in coda un chiamante (in testa con `top`).
    Select { call: String, top: bool },
    /// Toglie un Hound dalla coda.
    Unqueue { call: String },
    /// Chiude un QSO in corso.
    Drop { call: String },
    /// Le scelte dell'operatore.
    Options {
        slots: usize,
        more_cqs: bool,
        cq: String,
        free_text: Option<String>,
        auto_queue: bool,
        /// UltraFOX (compatibile con SuperFox): un solo segnale per periodo al posto dei posti FT8 affiancati.
        ultra: bool,
    },
    /// Tutto da capo.
    Reset,
    /// La verifica OTP (`foxOTPcode`): seme TOTP di 16 caratteri base32 (`None` = spenta) e ogni quanti periodi il Fox classico la manda.
    /// Con `keep` il seme gia' in uso resta (la pagina non lo rilegge: cambia solo l'intervallo).
    Otp { seed: Option<String>, interval: u32, keep: bool },
    /// Valore di ordinamento di un Hound dato da un programma esterno (UDP `AnnotationInfo`, tipo 16); `None` lo toglie.
    Annotate { call: String, order: Option<u32> },
}
