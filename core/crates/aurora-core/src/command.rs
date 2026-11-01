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
    Options { slots: usize, more_cqs: bool, cq: String, free_text: Option<String>, auto_queue: bool },
    /// Tutto da capo.
    Reset,
}
