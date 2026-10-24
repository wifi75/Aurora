//! Cambio modo: JTDX 159 non accetta la configurazione via UDP, quindi si preme il suo pulsante
//! con l'accessibilita' di Windows (UI Automation), come farebbe un utente.

/// Identificativo del pulsante di JTDX per un modo; `None` per qualunque altro testo.
pub fn button_id(mode: &str) -> Option<&'static str> {
    match mode.trim().to_uppercase().as_str() {
        "FT8" => Some("ft8Button"),
        "FT4" => Some("ft4Button"),
        _ => None,
    }
}

/// Script PowerShell che imposta la frequenza di ricezione (casella Rx di JTDX); `hz` e' un numero.
pub fn rx_script(hz: u32) -> String {
    format!(
        "Add-Type -AssemblyName UIAutomationClient, UIAutomationTypes; \
         $p = Get-Process jtdx -ErrorAction SilentlyContinue | Select-Object -First 1; \
         if (-not $p) {{ Write-Error 'JTDX non in esecuzione'; exit 3 }}; \
         $root = [System.Windows.Automation.AutomationElement]::RootElement; \
         $cond = New-Object System.Windows.Automation.PropertyCondition([System.Windows.Automation.AutomationElement]::ProcessIdProperty, $p.Id); \
         $main = $root.FindAll('Children', $cond) | Where-Object {{ $_.Current.ClassName -eq 'MainWindow' }} | Select-Object -First 1; \
         if (-not $main) {{ Write-Error 'finestra di JTDX non trovata'; exit 4 }}; \
         $all = $main.FindAll('Descendants', [System.Windows.Automation.Condition]::TrueCondition); \
         $el = $all | Where-Object {{ $_.Current.AutomationId -like '*.RxFreqSpinBox' }} | Select-Object -First 1; \
         if (-not $el) {{ Write-Error 'casella Rx non trovata'; exit 5 }}; \
         $el.GetCurrentPattern([System.Windows.Automation.RangeValuePattern]::Pattern).SetValue({hz})"
    )
}

/// Script PowerShell che accende il pulsante (se non lo e' gia'). `button` viene solo da `button_id`.
pub fn mode_script(button: &str) -> String {
    format!(
        "Add-Type -AssemblyName UIAutomationClient, UIAutomationTypes; \
         $p = Get-Process jtdx -ErrorAction SilentlyContinue | Select-Object -First 1; \
         if (-not $p) {{ Write-Error 'JTDX non in esecuzione'; exit 3 }}; \
         $root = [System.Windows.Automation.AutomationElement]::RootElement; \
         $cond = New-Object System.Windows.Automation.PropertyCondition([System.Windows.Automation.AutomationElement]::ProcessIdProperty, $p.Id); \
         $main = $root.FindAll('Children', $cond) | Where-Object {{ $_.Current.ClassName -eq 'MainWindow' }} | Select-Object -First 1; \
         if (-not $main) {{ Write-Error 'finestra di JTDX non trovata'; exit 4 }}; \
         $all = $main.FindAll('Descendants', [System.Windows.Automation.Condition]::TrueCondition); \
         $el = $all | Where-Object {{ $_.Current.AutomationId -like '*.{button}' }} | Select-Object -First 1; \
         if (-not $el) {{ Write-Error 'pulsante non trovato'; exit 5 }}; \
         $t = $el.GetCurrentPattern([System.Windows.Automation.TogglePattern]::Pattern); \
         if ($t.Current.ToggleState -ne 'On') {{ $t.Toggle() }}"
    )
}
