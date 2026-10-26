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
}
