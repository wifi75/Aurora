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
}

const fn toggle(id: &'static str, ui_id: &'static str) -> Control {
    Control { id, ui_id, kind: Kind::Toggle, tx: false }
}

const fn arming(id: &'static str, ui_id: &'static str) -> Control {
    Control { id, ui_id, kind: Kind::Toggle, tx: true }
}

const fn press(id: &'static str, ui_id: &'static str) -> Control {
    Control { id, ui_id, kind: Kind::Press, tx: false }
}

pub const CONTROLS: &[Control] = &[
    toggle("autoseq", "AutoSeqButton"),
    toggle("tx_period", "TxMinuteButton"),
    toggle("single_qso", "singleQSOButton"),
    toggle("answer_b4", "AnsB4Button"),
    toggle("bypass", "bypassButton"),
    toggle("skip_tx1", "skipTx1"),
    toggle("rrr", "rrr1CheckBox"),
    toggle("split", "pbTxLock"),
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
];

pub fn find(id: &str) -> Option<&'static Control> {
    CONTROLS.iter().find(|c| c.id == id)
}

pub(crate) const HEAD: &str = "[Console]::OutputEncoding = New-Object System.Text.UTF8Encoding $false; \
     Add-Type -AssemblyName UIAutomationClient, UIAutomationTypes; \
     $p = Get-Process jtdx -ErrorAction SilentlyContinue | Select-Object -First 1; \
     if (-not $p) { Write-Error 'JTDX non in esecuzione'; exit 3 }; \
     $root = [System.Windows.Automation.AutomationElement]::RootElement; \
     $cond = New-Object System.Windows.Automation.PropertyCondition([System.Windows.Automation.AutomationElement]::ProcessIdProperty, $p.Id); \
     $main = $root.FindAll('Children', $cond) | Where-Object { $_.Current.ClassName -eq 'MainWindow' } | Select-Object -First 1; \
     if (-not $main) { Write-Error 'finestra di JTDX non trovata'; exit 4 }; \
     $all = $main.FindAll('Descendants', [System.Windows.Automation.Condition]::TrueCondition); ";

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

/// Interpreta l'uscita di `read_script`: gli interruttori (`nome=1|0`) e, se c'e', lo stato dei messaggi
/// (`txmsg<n>=testo`, `txsel=<n>`). Le righe sconosciute si ignorano.
/// Una lettura di JTDX: gli interruttori (nome e acceso) e, se letti, i messaggi Tx.
pub type Reading = (Vec<(String, bool)>, Option<TxState>);

pub fn parse_reading(text: &str) -> Reading {
    let mut switches = Vec::new();
    let mut messages = vec![String::new(); 6];
    let mut seen = false;
    let mut selected = None;
    let mut minute_label = String::new();
    let mut autoseq_label = String::new();
    for line in text.lines() {
        // di un messaggio contano gli spazi dentro, non il fine riga
        let line = line.trim_end_matches(['\r', '\n']);
        let Some((id, value)) = line.split_once('=') else { continue };
        let id = id.trim();
        if let Some(n) = id.strip_prefix("txmsg").and_then(|n| n.parse::<usize>().ok()) {
            if (1..=6).contains(&n) {
                messages[n - 1] = value.trim().to_string();
                seen = true;
            }
        } else if id == "txminute" {
            minute_label = value.trim().to_string();
            seen = true;
        } else if id == "txautoseq" {
            autoseq_label = value.trim().to_string();
            seen = true;
        } else if id == "txsel" {
            selected = value.trim().parse::<u8>().ok().filter(|n| (1..=6).contains(n));
        } else if find(id).is_some() {
            switches.push((id.to_string(), value.trim() == "1"));
        }
    }
    (switches, seen.then_some(TxState { messages, selected, minute_label, autoseq_label }))
}

/// Script che stampa `nome=1` o `nome=0` per ogni interruttore trovato in JTDX, poi i sei messaggi Tx (`txmsg1=...`) e il
/// numero di quello selezionato (`txsel=2`), tutto nella stessa lettura (ogni lettura costa circa un secondo).
pub fn read_script() -> String {
    let pairs: Vec<String> = CONTROLS.iter().filter(|c| c.kind == Kind::Toggle).map(|c| format!("@('{}','{}')", c.id, c.ui_id)).collect();
    format!(
        "{HEAD}foreach ($pair in @({})) {{ \
         $el = $all | Where-Object {{ $_.Current.AutomationId -like ('*.' + $pair[1]) }} | Select-Object -First 1; \
         if ($el) {{ $on = $el.GetCurrentPattern([System.Windows.Automation.TogglePattern]::Pattern).Current.ToggleState -eq 'On'; \
         Write-Output ($pair[0] + '=' + [int]$on) }} }}; \
         foreach ($n in 1..6) {{ \
         $m = $all | Where-Object {{ $_.Current.AutomationId -like ('*.tx' + $n) }} | Select-Object -First 1; \
         if ($m) {{ $v = ''; try {{ $v = $m.GetCurrentPattern([System.Windows.Automation.ValuePattern]::Pattern).Current.Value }} catch {{ }}; \
         Write-Output ('txmsg' + $n + '=' + $v) }}; \
         $rb = $all | Where-Object {{ $_.Current.AutomationId -like ('*.txrb' + $n) }} | Select-Object -First 1; \
         if ($rb) {{ try {{ if ($rb.GetCurrentPattern([System.Windows.Automation.SelectionItemPattern]::Pattern).Current.IsSelected) {{ Write-Output ('txsel=' + $n) }} }} catch {{ }} }} }}; \
         $tm = $all | Where-Object {{ $_.Current.AutomationId -like '*.TxMinuteButton' }} | Select-Object -First 1; \
         if ($tm) {{ Write-Output ('txminute=' + $tm.Current.Name) }}; \
         $as = $all | Where-Object {{ $_.Current.AutomationId -like '*.AutoSeqButton' }} | Select-Object -First 1; \
         if ($as) {{ Write-Output ('txautoseq=' + $as.Current.Name) }}",
        pairs.join(", ")
    )
}

/// Script che seleziona il messaggio Tx `n` (1-6) in JTDX, come un clic sul pallino accanto al messaggio: da quello il motore
/// trasmette alla prossima occasione e poi prosegue da solo. Non preme il pulsante «Tx n» (riavvierebbe una trasmissione in corso).
/// `n` e' un numero da 1 a 6: niente testo dell'utente nello script.
pub fn select_tx_script(n: u8) -> String {
    format!(
        "{HEAD}$el = $all | Where-Object {{ $_.Current.AutomationId -like '*.txrb{n}' }} | Select-Object -First 1; \
         if (-not $el) {{ Write-Error 'messaggio non trovato'; exit 5 }}; \
         $el.GetCurrentPattern([System.Windows.Automation.SelectionItemPattern]::Pattern).Select()"
    )
}

/// Script che porta un interruttore allo stato voluto (premendolo solo se serve) o preme un pulsante semplice.
/// `control` viene solo dalla tabella: nessun testo dell'utente nello script.
pub fn set_script(control: &Control, on: Option<bool>) -> String {
    let find = format!(
        "{HEAD}$el = $all | Where-Object {{ $_.Current.AutomationId -like '*.{}' }} | Select-Object -First 1; \
         if (-not $el) {{ Write-Error 'controllo non trovato'; exit 5 }}; ",
        control.ui_id
    );
    let invoke = "$el.GetCurrentPattern([System.Windows.Automation.InvokePattern]::Pattern).Invoke()";
    match (control.kind, on) {
        (Kind::Toggle, Some(want)) => {
            let want = if want { "$true" } else { "$false" };
            format!(
                "{find}$now = $el.GetCurrentPattern([System.Windows.Automation.TogglePattern]::Pattern).Current.ToggleState -eq 'On'; \
                 if ($now -ne {want}) {{ {invoke} }}"
            )
        }
        _ => format!("{find}{invoke}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_switches_messages_and_the_selected_one() {
        let text = "autoseq=1\nskip_tx1=1\nmonitor=0\ntxmsg1=M7SZY IU3ABC IO91\ntxmsg2=M7SZY IU3ABC -07\ntxmsg3=M7SZY IU3ABC R-07\n\
                    txmsg4=M7SZY IU3ABC RR73\ntxmsg5=M7SZY IU3ABC 73\ntxmsg6=CQ DX IU3ABC IO91\ntxsel=2\ntxminute=TX 15/45\ntxautoseq=AutoSeq 3\n";
        let (switches, tx) = parse_reading(text);
        assert_eq!(switches, vec![("autoseq".into(), true), ("skip_tx1".into(), true), ("monitor".into(), false)]);
        let tx = tx.expect("stato dei messaggi");
        assert_eq!(tx.selected, Some(2));
        assert_eq!(tx.messages[1], "M7SZY IU3ABC -07");
        assert_eq!(tx.messages[5], "CQ DX IU3ABC IO91");
        assert_eq!(tx.minute_label, "TX 15/45");
        assert_eq!(tx.autoseq_label, "AutoSeq 3");
    }

    #[test]
    fn without_message_lines_there_is_no_tx_state_and_unknown_lines_are_ignored() {
        let (switches, tx) = parse_reading("autoseq=1\nstrano=1\nsenza uguale\ntxsel=9\n");
        assert_eq!(switches, vec![("autoseq".into(), true)]);
        assert!(tx.is_none());
    }

    #[test]
    fn a_message_that_contains_an_equals_sign_keeps_its_text_and_a_bad_selection_is_dropped() {
        let (_, tx) = parse_reading("txmsg6=CQ A=B\r\ntxsel=0\r\n");
        let tx = tx.expect("stato dei messaggi");
        assert_eq!(tx.messages[5], "CQ A=B");
        assert_eq!(tx.selected, None);
    }
}
