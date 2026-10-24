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

const HEAD: &str = "Add-Type -AssemblyName UIAutomationClient, UIAutomationTypes; \
     $p = Get-Process jtdx -ErrorAction SilentlyContinue | Select-Object -First 1; \
     if (-not $p) { Write-Error 'JTDX non in esecuzione'; exit 3 }; \
     $root = [System.Windows.Automation.AutomationElement]::RootElement; \
     $cond = New-Object System.Windows.Automation.PropertyCondition([System.Windows.Automation.AutomationElement]::ProcessIdProperty, $p.Id); \
     $main = $root.FindAll('Children', $cond) | Where-Object { $_.Current.ClassName -eq 'MainWindow' } | Select-Object -First 1; \
     if (-not $main) { Write-Error 'finestra di JTDX non trovata'; exit 4 }; \
     $all = $main.FindAll('Descendants', [System.Windows.Automation.Condition]::TrueCondition); ";

/// Script che stampa `nome=1` o `nome=0` per ogni interruttore trovato in JTDX.
pub fn read_script() -> String {
    let pairs: Vec<String> = CONTROLS.iter().filter(|c| c.kind == Kind::Toggle).map(|c| format!("@('{}','{}')", c.id, c.ui_id)).collect();
    format!(
        "{HEAD}foreach ($pair in @({})) {{ \
         $el = $all | Where-Object {{ $_.Current.AutomationId -like ('*.' + $pair[1]) }} | Select-Object -First 1; \
         if ($el) {{ $on = $el.GetCurrentPattern([System.Windows.Automation.TogglePattern]::Pattern).Current.ToggleState -eq 'On'; \
         Write-Output ($pair[0] + '=' + [int]$on) }} }}",
        pairs.join(", ")
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
