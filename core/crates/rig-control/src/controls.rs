//! I pulsanti di JTDX che Aurora puo' comandare (AutoSeq, 1 QSO, Abilita Tx...), premuti con l'accessibilita'
//! di Windows come farebbe un utente. L'elenco e' chiuso: l'accordo (portante) e il log non ci sono, e l'arresto
//! della trasmissione ha la sua via (`HaltTx`).

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// Casella accendibile: si imposta lo stato voluto.
    Toggle,
    /// Pulsante semplice: si preme.
    Press,
}

#[derive(Debug, Clone, Copy)]
pub struct Control {
    /// Nome nell'API.
    pub id: &'static str,
    /// Fine dell'identificativo del controllo in JTDX (cerca `*.<ui_id>`).
    pub ui_id: &'static str,
    pub kind: Kind,
    /// Accenderlo puo' far trasmettere: serve la trasmissione aperta.
    pub tx: bool,
    /// Il nome nell'API e il controllo di JTDX hanno il senso contrario: acceso nell'API = spento in JTDX (`split` e `pbTxLock`).
    pub inverted: bool,
    /// Esiste solo nel motore proprio di Aurora: JTDX non ha il pulsante (non si legge ne' si preme con l'accessibilita').
    pub own: bool,
}

const fn toggle(id: &'static str, ui_id: &'static str) -> Control {
    Control { id, ui_id, kind: Kind::Toggle, tx: false, inverted: false, own: false }
}

/// Interruttore del solo motore proprio (non e' un controllo di JTDX).
const fn own_toggle(id: &'static str) -> Control {
    Control { id, ui_id: "", kind: Kind::Toggle, tx: false, inverted: false, own: true }
}

/// Pulsante semplice del solo motore proprio.
const fn own_press(id: &'static str) -> Control {
    Control { id, ui_id: "", kind: Kind::Press, tx: false, inverted: false, own: true }
}

const fn arming(id: &'static str, ui_id: &'static str) -> Control {
    Control { id, ui_id, kind: Kind::Toggle, tx: true, inverted: false, own: false }
}

const fn press(id: &'static str, ui_id: &'static str) -> Control {
    Control { id, ui_id, kind: Kind::Press, tx: false, inverted: false, own: false }
}

/// Interruttore dal senso contrario a quello di JTDX. `pbTxLock` e' «Lockd Tx=Rx» quando e' acceso e «Tx/Rx Split» quando e' spento
/// (`on_pbTxLock_clicked`, `REA 0x4a4b30`: `m_lockTxFreq = checked`): lo «Split» dell'API e' **acceso quando il blocco e' spento**.
const fn inverted(id: &'static str, ui_id: &'static str) -> Control {
    Control { id, ui_id, kind: Kind::Toggle, tx: false, inverted: true, own: false }
}

pub const CONTROLS: &[Control] = &[
    toggle("autoseq", "AutoSeqButton"),
    toggle("tx_period", "TxMinuteButton"),
    toggle("single_qso", "singleQSOButton"),
    // «Rispondi a tutti»: ogni stazione che mi chiama entra in coda da sola (senza, solo quelle scelte con il tasto destro)
    own_toggle("answer_all"),
    // «JT65+JT9»: in uno dei due modi JT si decodifica nello stesso minuto anche l'altro (la trasmissione resta nel modo scelto)
    own_toggle("jt_both"),
    // «a7/a8»: decodifiche a liste di WSJT-X in FT8 (seguiti dei QSO del periodo precedente, DX alla frequenza di ricezione)
    own_toggle("ap_list"),
    // «Multithread»: la decodifica FT8 su tutti i processori (stessi decode del singolo thread)
    own_toggle("mtd"),
    // «Wait and Call» di WSJT-X: armato, quando il DX si libera (CQ, RR73, RRR, 73) lo si chiama da soli
    own_toggle("wait_call"),
    // «Wait and Pounce» di WSJT-X: armato con la trasmissione spenta, risponde da solo al CQ scelto con il criterio delle impostazioni
    own_toggle("pounce"),
    own_toggle("hold_tx"),
    own_toggle("fox"),
    // Q65: modello dell'evanescenza gaussiano invece del lorentziano di WSJT-X (`fadingModel` di `q65_intrinsics_fastfading`)
    own_toggle("q65_gauss"),
    // Q65: ritardo dell'eco lunare (`emedelay` di WSJT-X, «Decode after EME delay»): il sincronismo si cerca fino a 5,5 s
    own_toggle("q65_eme"),
    // Q65: deriva di frequenza al massimo (`max_drift = 50` di WSJT-X, W3SZ), con l'ultimo tentativo di lista raddrizzato
    own_toggle("q65_drift"),
    // Q65: contest NA VHF / WW Digi / ARRL Digi (`ncontest = 1`): lista dei messaggi dei chiamanti (`q65_set_list2`)
    own_toggle("q65_contest"),
    toggle("answer_b4", "AnsB4Button"),
    toggle("bypass", "bypassButton"),
    toggle("skip_tx1", "skipTx1"),
    toggle("rrr", "rrr1CheckBox"),
    inverted("split", "pbTxLock"),
    toggle("show_wanted", "cbShowWanted"),
    toggle("hound", "HoundButton"),
    toggle("jt", "jtButton"),
    toggle("monitor", "monitorButton"),
    toggle("swl", "swlButton"),
    toggle("sync", "syncButton"),
    toggle("agc", "AGCcButton"),
    toggle("filter", "filterButton"),
    arming("auto_tx", "AutoTxButton"),
    arming("enable_tx", "enableTxButton"),
    press("decode", "DecodeButton"),
    press("erase", "EraseButton"),
    press("stop_rx", "stopButton"),
    press("clear_dx", "ClearDxButton"),
    press("gen_msgs", "genStdMsgsPushButton"),
    press("rx_to_tx", "pbR2T"),
    press("tx_to_rx", "pbT2R"),
    // Q65: «Clear Avg» di WSJT-X, svuota la media dei periodi
    own_press("q65_clear_avg"),
];

pub fn find(id: &str) -> Option<&'static Control> {
    CONTROLS.iter().find(|c| c.id == id)
}


/// I sei messaggi di trasmissione di JTDX (Tx1-Tx6) e quale e' selezionato (1-6): e' lui a decidere quale mandare, anche
/// con «Salta Tx1» o dopo una risposta, quindi Aurora lo legge invece di prevederlo.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct TxState {
    pub messages: Vec<String>,
    pub selected: Option<u8>,
    /// Il testo della casella del periodo come lo scrive JTDX ("TX 00/30", "TX 15/45", "TX 00"...): cambia con lo stato e col modo,
    /// quindi si legge invece di ricostruirlo.
    pub minute_label: String,
    /// Il testo del pulsante AutoSeq, che porta il livello ("AutoSeq 3").
    pub autoseq_label: String,
}

/// Gli interruttori (nome e acceso) e, se letti, i messaggi Tx.
pub type Reading = (Vec<(String, bool)>, Option<TxState>);
