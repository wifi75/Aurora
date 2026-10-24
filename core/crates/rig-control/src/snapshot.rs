//! Lo stato di JTDX letto dalla sua finestra, una volta, con l'accessibilita' di Windows.
//!
//! Serve subito dopo l'avvio di Aurora: JTDX manda il suo stato UDP solo a fine periodo (fino a 15 secondi dopo), e
//! senza stato la pagina resta vuota. Il primo stato vero lo sostituisce.

use crate::controls::HEAD;

/// Quello che si legge dalla finestra di JTDX. I campi che non si trovano restano vuoti.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Snapshot {
    pub dial_hz: Option<u64>,
    pub mode: Option<String>,
    pub dx_call: Option<String>,
    pub dx_grid: Option<String>,
    pub tx_df: Option<u32>,
    pub rx_df: Option<u32>,
    pub report: Option<String>,
    pub transmitting: bool,
    pub tx_enabled: bool,
}

const BODY: &str = r"
function Get-Item($s) { $all | Where-Object { $_.Current.AutomationId -like ('*.' + $s) } | Select-Object -First 1 };
function Get-Val($s) { $e = Get-Item $s; if ($e) { try { $e.GetCurrentPattern([System.Windows.Automation.ValuePattern]::Pattern).Current.Value } catch { '' } } };
function Get-On($s) { $e = Get-Item $s; if ($e) { try { [int]($e.GetCurrentPattern([System.Windows.Automation.TogglePattern]::Pattern).Current.ToggleState -eq 'On') } catch { 0 } } else { 0 } };
$d = Get-Item 'labDialFreq'; if ($d) { Write-Output ('dial=' + $d.Current.Name) };
Write-Output ('dxcall=' + (Get-Val 'dxCallEntry'));
Write-Output ('dxgrid=' + (Get-Val 'dxGridEntry'));
Write-Output ('txdf=' + (Get-Val 'TxFreqSpinBox'));
Write-Output ('rxdf=' + (Get-Val 'RxFreqSpinBox'));
Write-Output ('rpt=' + (Get-Val 'rptSpinBox'));
Write-Output ('ft8=' + (Get-On 'ft8Button'));
Write-Output ('ft4=' + (Get-On 'ft4Button'));
Write-Output ('jt=' + (Get-On 'jtButton'));
Write-Output ('enabletx=' + (Get-On 'enableTxButton'));
$bar = $all | Where-Object { $_.Current.ControlType -eq [System.Windows.Automation.ControlType]::StatusBar } | Select-Object -First 1;
if ($bar) { $first = $bar.FindAll('Descendants', [System.Windows.Automation.Condition]::TrueCondition) | Where-Object { $_.Current.Name } | Select-Object -First 1; if ($first) { Write-Output ('state=' + $first.Current.Name) } }
";

/// Script che stampa le righe `chiave=valore` lette da JTDX (nessun testo dell'utente nello script).
pub fn read_script() -> String {
    format!("{HEAD}{BODY}")
}

/// Le etichette con cui JTDX dice che sta solo ricevendo, nelle lingue dell'interfaccia. Qualunque altra cosa nella barra di stato
/// (per esempio «Trasmissione») fa considerare JTDX in trasmissione: nel dubbio si sbaglia dalla parte prudente.
const RECEIVING: &[&str] = &["ricezione", "receiving", "empfang", "réception", "reception", "recepción", "recepcion", "receção", "recepção"];

fn non_empty(text: &str) -> Option<String> {
    let t = text.trim();
    (!t.is_empty()).then(|| t.to_string())
}

fn first_number(text: &str) -> Option<u32> {
    let digits: String = text.chars().skip_while(|c| !c.is_ascii_digit()).take_while(|c| c.is_ascii_digit()).collect();
    digits.parse().ok()
}

/// "14,080 000" (MHz con la virgola e i gruppi di cifre separati da spazi) in Hz.
fn dial_to_hz(text: &str) -> Option<u64> {
    // si tengono solo cifre e il separatore decimale: JTDX separa i gruppi con uno spazio non divisibile, che una console
    // con un'altra codifica puo' trasformare in un carattere qualunque
    let clean: String = text.chars().filter(|c| c.is_ascii_digit() || matches!(c, ',' | '.')).map(|c| if c == ',' { '.' } else { c }).collect();
    let mhz: f64 = clean.parse().ok()?;
    (mhz > 0.0 && mhz < 1000.0).then(|| (mhz * 1_000_000.0).round() as u64)
}

/// Interpreta l'uscita di `read_script`. Le righe sconosciute si ignorano.
pub fn parse(text: &str) -> Snapshot {
    let mut s = Snapshot::default();
    let (mut ft8, mut ft4, mut jt) = (false, false, false);
    let mut state = String::new();
    for line in text.lines() {
        let Some((key, value)) = line.trim_end_matches(['\r', '\n']).split_once('=') else { continue };
        match key.trim() {
            "dial" => s.dial_hz = dial_to_hz(value),
            "dxcall" => s.dx_call = non_empty(value),
            "dxgrid" => s.dx_grid = non_empty(value),
            "txdf" => s.tx_df = first_number(value),
            "rxdf" => s.rx_df = first_number(value),
            // "Rapporto -2": l'ultimo pezzo e' il numero con il segno
            "rpt" => s.report = value.split_whitespace().last().filter(|t| t.parse::<i32>().is_ok()).map(str::to_string),
            "ft8" => ft8 = value.trim() == "1",
            "ft4" => ft4 = value.trim() == "1",
            "jt" => jt = value.trim() == "1",
            "enabletx" => s.tx_enabled = value.trim() == "1",
            "state" => state = value.trim().to_lowercase(),
            _ => {}
        }
    }
    s.mode = if ft8 {
        Some("FT8".into())
    } else if ft4 {
        Some("FT4".into())
    } else if jt {
        Some("JT".into())
    } else {
        None
    };
    s.transmitting = !state.is_empty() && !RECEIVING.iter().any(|w| state.starts_with(w));
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    const TEXT: &str = "dial=14,080 000\ndxcall=M7SZY\ndxgrid=JO02\ntxdf=Tx  1084  Hz\nrxdf=Rx  1034  Hz\nrpt=Rapporto -2\nft8=0\nft4=1\njt=0\nenabletx=1\nstate=Ricezione\n";

    #[test]
    fn reads_the_fields_of_the_window() {
        let s = parse(TEXT);
        assert_eq!(s.dial_hz, Some(14_080_000));
        assert_eq!(s.mode.as_deref(), Some("FT4"));
        assert_eq!(s.dx_call.as_deref(), Some("M7SZY"));
        assert_eq!(s.dx_grid.as_deref(), Some("JO02"));
        assert_eq!((s.tx_df, s.rx_df), (Some(1084), Some(1034)));
        assert_eq!(s.report.as_deref(), Some("-2"));
        assert!(s.tx_enabled);
        assert!(!s.transmitting);
    }

    #[test]
    fn dial_handles_non_breaking_spaces_and_the_usual_bands() {
        assert_eq!(dial_to_hz("21,074\u{a0}000"), Some(21_074_000));
        assert_eq!(dial_to_hz("7,074 000"), Some(7_074_000));
        assert_eq!(dial_to_hz("28.074000"), Some(28_074_000));
        // lo spazio non divisibile letto male da una console OEM diventa il carattere di sostituzione
        assert_eq!(dial_to_hz("14,080\u{fffd}000"), Some(14_080_000));
        assert_eq!(dial_to_hz(""), None);
        assert_eq!(dial_to_hz("0,000 000"), None);
        assert_eq!(dial_to_hz("abc"), None);
    }

    #[test]
    fn anything_but_a_receiving_label_counts_as_transmitting_to_stay_on_the_safe_side() {
        for label in ["Ricezione", "Receiving", "Empfang", "Réception", "Recepción", "Receção"] {
            assert!(!parse(&format!("dial=14,074 000\nstate={label}\n")).transmitting, "{label}");
        }
        assert!(parse("dial=14,074 000\nstate=Trasmissione\n").transmitting);
        assert!(parse("dial=14,074 000\nstate=Tx: FT8\n").transmitting);
        // senza barra di stato non si sa: non si inventa una trasmissione
        assert!(!parse("dial=14,074 000\n").transmitting);
    }

    #[test]
    fn empty_fields_stay_empty_and_the_mode_may_be_unknown() {
        let s = parse("dial=14,074 000\ndxcall=\ndxgrid= \ntxdf=\nrpt=\nft8=0\nft4=0\njt=0\nenabletx=0\n");
        assert_eq!((s.dx_call, s.dx_grid, s.tx_df, s.report, s.mode), (None, None, None, None, None));
        assert_eq!(parse("righe senza uguale\n"), Snapshot::default());
    }
}
