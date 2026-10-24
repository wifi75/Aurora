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

const OUT_ATTENUATION_HEAD: &str = "Add-Type -AssemblyName UIAutomationClient, UIAutomationTypes; \
         $p = Get-Process jtdx -ErrorAction SilentlyContinue | Select-Object -First 1; \
         if (-not $p) { Write-Error 'JTDX non in esecuzione'; exit 3 }; \
         $root = [System.Windows.Automation.AutomationElement]::RootElement; \
         $cond = New-Object System.Windows.Automation.PropertyCondition([System.Windows.Automation.AutomationElement]::ProcessIdProperty, $p.Id); \
         $main = $root.FindAll('Children', $cond) | Where-Object { $_.Current.ClassName -eq 'MainWindow' } | Select-Object -First 1; \
         if (-not $main) { Write-Error 'finestra di JTDX non trovata'; exit 4 }; \
         $all = $main.FindAll('Descendants', [System.Windows.Automation.Condition]::TrueCondition); \
         $el = $all | Where-Object { $_.Current.AutomationId -like '*.outAttenuation' } | Select-Object -First 1; \
         if (-not $el) { Write-Error 'cursore non trovato'; exit 5 }; \
         $r = $el.GetCurrentPattern([System.Windows.Automation.RangeValuePattern]::Pattern); ";

/// Script che stampa l'attenuazione di uscita di JTDX in decimi di dB (cursore "Pwr").
pub fn attenuation_read_script() -> String {
    format!("{OUT_ATTENUATION_HEAD}[int]$r.Current.Value")
}

/// Script che imposta l'attenuazione di uscita (decimi di dB); `tenths` e' un numero.
pub fn attenuation_set_script(tenths: u32) -> String {
    format!("{OUT_ATTENUATION_HEAD}$r.SetValue({tenths})")
}

/// Script PowerShell che sceglie il modo premendo il pulsante come un clic (`Invoke`): `Toggle()` accenderebbe solo
/// la casella senza far cambiare modo a JTDX. Si preme se il pulsante e' spento o se anche l'altro risulta acceso
/// (stato incoerente). `button` viene solo da `button_id`.
pub fn mode_script(button: &str) -> String {
    let other = if button == "ft4Button" { "ft8Button" } else { "ft4Button" };
    format!(
        "Add-Type -AssemblyName UIAutomationClient, UIAutomationTypes;          $p = Get-Process jtdx -ErrorAction SilentlyContinue | Select-Object -First 1;          if (-not $p) {{ Write-Error 'JTDX non in esecuzione'; exit 3 }};          $root = [System.Windows.Automation.AutomationElement]::RootElement;          $cond = New-Object System.Windows.Automation.PropertyCondition([System.Windows.Automation.AutomationElement]::ProcessIdProperty, $p.Id);          $main = $root.FindAll('Children', $cond) | Where-Object {{ $_.Current.ClassName -eq 'MainWindow' }} | Select-Object -First 1;          if (-not $main) {{ Write-Error 'finestra di JTDX non trovata'; exit 4 }};          $all = $main.FindAll('Descendants', [System.Windows.Automation.Condition]::TrueCondition);          $el = $all | Where-Object {{ $_.Current.AutomationId -like '*.{button}' }} | Select-Object -First 1;          $ot = $all | Where-Object {{ $_.Current.AutomationId -like '*.{other}' }} | Select-Object -First 1;          if (-not $el) {{ Write-Error 'pulsante non trovato'; exit 5 }};          $on = $el.GetCurrentPattern([System.Windows.Automation.TogglePattern]::Pattern).Current.ToggleState -eq 'On';          $otherOn = $ot -and ($ot.GetCurrentPattern([System.Windows.Automation.TogglePattern]::Pattern).Current.ToggleState -eq 'On');          if ((-not $on) -or $otherOn) {{ $el.GetCurrentPattern([System.Windows.Automation.InvokePattern]::Pattern).Invoke() }}"
    )
}
