//! Il programma JTDX come processo di Windows: trovarlo, chiuderlo con garbo e riavviarlo come stava.
//! Serve per applicare le impostazioni: JTDX le legge solo all'avvio e riscrive il suo file all'uscita.

use crate::{omnirig::powershell_output, RigError};

/// JTDX in esecuzione: numero del processo e comando con cui e' stato avviato.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Running {
    pub id: u32,
    /// Percorso dell'eseguibile.
    pub exe: String,
    /// Argomenti dopo l'eseguibile (di solito nessuno).
    pub args: String,
}

/// Divide la riga di comando di un processo in eseguibile e argomenti: `"C:\\x y\\jtdx.exe" --a` o `C:\\x\\jtdx.exe --a`.
pub fn split_command_line(command: &str) -> (String, String) {
    let command = command.trim();
    if let Some(rest) = command.strip_prefix('"') {
        return match rest.split_once('"') {
            Some((exe, args)) => (exe.to_string(), args.trim().to_string()),
            None => (rest.to_string(), String::new()),
        };
    }
    match command.split_once(char::is_whitespace) {
        Some((exe, args)) => (exe.to_string(), args.trim().to_string()),
        None => (command.to_string(), String::new()),
    }
}

/// Testo per PowerShell tra apici singoli (l'apice si raddoppia): senza modi di uscirne.
fn quoted(text: &str) -> String {
    format!("'{}'", text.replace('\'', "''"))
}

/// Il JTDX acceso adesso, se c'e'.
pub fn find() -> Result<Option<Running>, RigError> {
    let out = powershell_output(
        "$p = Get-CimInstance Win32_Process -Filter \"Name='jtdx.exe'\" | Select-Object -First 1; \
         if ($p) { Write-Output ('{0}|{1}' -f $p.ProcessId, $p.CommandLine) }",
    )?;
    let Some((id, command)) = out.split_once('|') else { return Ok(None) };
    let id: u32 = id.trim().parse().map_err(|_| RigError("numero di processo non valido".into()))?;
    let (exe, args) = split_command_line(command);
    if exe.is_empty() {
        return Err(RigError("percorso di JTDX non trovato".into()));
    }
    Ok(Some(Running { id, exe, args }))
}

/// Chiude la finestra principale (come un utente) e aspetta che JTDX esca; se non esce entro `wait_secs` e' un errore.
pub fn stop(id: u32, wait_secs: u32) -> Result<(), RigError> {
    let ms = u64::from(wait_secs) * 1000;
    powershell_output(&format!(
        "$p = Get-Process -Id {id} -ErrorAction SilentlyContinue; if (-not $p) {{ exit 0 }}; \
         $null = $p.CloseMainWindow(); if (-not $p.WaitForExit({ms})) {{ Write-Error 'JTDX non si chiude'; exit 5 }}"
    ))
    .map(|_| ())
}

/// Avvia JTDX con gli stessi argomenti di prima, dalla sua cartella.
pub fn start(running: &Running) -> Result<(), RigError> {
    let dir = std::path::Path::new(&running.exe).parent().map(|p| p.display().to_string()).unwrap_or_default();
    let mut script = format!("Start-Process -FilePath {}", quoted(&running.exe));
    if !dir.is_empty() {
        script.push_str(&format!(" -WorkingDirectory {}", quoted(&dir)));
    }
    if !running.args.is_empty() {
        script.push_str(&format!(" -ArgumentList {}", quoted(&running.args)));
    }
    powershell_output(&script).map(|_| ())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splits_quoted_and_plain_command_lines() {
        assert_eq!(split_command_line("\"C:\\JTDX64\\159\\bin\\jtdx.exe\""), ("C:\\JTDX64\\159\\bin\\jtdx.exe".to_string(), String::new()));
        assert_eq!(split_command_line("\"C:\\Programmi\\J T\\jtdx.exe\" --rig-name=IC7300 -r x"), ("C:\\Programmi\\J T\\jtdx.exe".to_string(), "--rig-name=IC7300 -r x".to_string()));
        assert_eq!(split_command_line("C:\\j\\jtdx.exe -x"), ("C:\\j\\jtdx.exe".to_string(), "-x".to_string()));
        assert_eq!(split_command_line("  C:\\j\\jtdx.exe  "), ("C:\\j\\jtdx.exe".to_string(), String::new()));
    }

    #[test]
    fn quoting_keeps_an_apostrophe_inside_the_text() {
        assert_eq!(quoted("C:\\Bob's\\jtdx.exe"), "'C:\\Bob''s\\jtdx.exe'");
    }
}
