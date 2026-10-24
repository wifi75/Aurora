//! Sessione PowerShell che resta aperta: i comandi a JTDX (premere Abilita Tx, scegliere un messaggio...) non pagano piu'
//! l'avvio di PowerShell e la ricerca di tutti i controlli della finestra a ogni volta. Prima ogni comando era un processo nuovo
//! (circa un secondo); ora la ricerca dei controlli si ricorda per dieci minuti e il comando costa pochi decimi.
//!
//! Le sessioni sono due: una per i comandi e una per le letture periodiche, cosi' una lettura lenta non fa aspettare mai un comando.
//! Se la sessione si blocca o muore si chiude e se ne apre una nuova al comando dopo, e quello in corso si ritenta una volta.

use std::{
    io::{BufRead, BufReader, Write},
    process::{Child, ChildStdin, Command, Stdio},
    sync::{mpsc, Mutex},
    time::Duration,
};

use crate::RigError;

const END: &str = "<<<AURORA-END>>>";
const ERR: &str = "AURORA-ERR:";
/// Quanto aspettare la risposta di uno script prima di considerare la sessione bloccata.
pub const TIMEOUT: Duration = Duration::from_secs(20);

/// Il programma che gira nella sessione: legge uno script (in base64) per riga, lo esegue e risponde con l'uscita e una riga di fine.
/// `Get-AllCached` ricorda per dieci minuti l'elenco dei controlli di JTDX; se qualcosa va storto la memoria si svuota.
const LOOP: &str = r#"
$ErrorActionPreference = 'Stop'
[Console]::OutputEncoding = New-Object System.Text.UTF8Encoding $false
Add-Type -AssemblyName UIAutomationClient, UIAutomationTypes
$global:AuroraCache = @{}
function Get-AllCached($main) {
  $key = ($main.GetRuntimeId() -join '.')
  $c = $global:AuroraCache
  if ($c.key -eq $key -and $c.at -gt (Get-Date).AddSeconds(-600)) { return $c.all }
  $all = $main.FindAll('Descendants', [System.Windows.Automation.Condition]::TrueCondition)
  $global:AuroraCache = @{ key = $key; all = $all; at = Get-Date }
  return $all
}
while ($true) {
  $line = [Console]::In.ReadLine()
  if ($null -eq $line) { break }
  try {
    $src = [System.Text.Encoding]::UTF8.GetString([System.Convert]::FromBase64String($line))
    $out = (& ([scriptblock]::Create($src))) | Out-String
    [Console]::Out.Write($out)
  } catch {
    $global:AuroraCache = @{}
    [Console]::Out.WriteLine('AURORA-ERR:' + $_.Exception.Message)
  }
  [Console]::Out.WriteLine('<<<AURORA-END>>>')
}
"#;

/// Adatta uno script scritto per un processo a se' alla sessione: un `exit` chiuderebbe la sessione, quindi
/// `Write-Error 'testo'; exit N` diventa `throw 'testo'`; e la ricerca di tutti i controlli passa dalla memoria.
pub fn adapt(script: &str) -> String {
    let mut out = String::with_capacity(script.len());
    let mut rest = script;
    while let Some(at) = rest.find("Write-Error '") {
        let (before, from) = rest.split_at(at);
        out.push_str(before);
        let body = &from["Write-Error ".len()..];
        // il testo tra apici, poi `; exit <numero>`
        let converted = body.strip_prefix('\'').and_then(|b| b.find('\'').map(|end| (&b[..end], &b[end + 1..]))).and_then(|(text, tail)| {
            let after = tail.strip_prefix("; exit ")?;
            let digits = after.chars().take_while(char::is_ascii_digit).count();
            (digits > 0).then(|| (text, &after[digits..]))
        });
        match converted {
            Some((text, tail)) => {
                out.push_str(&format!("throw '{text}'"));
                rest = tail;
            }
            None => {
                out.push_str("Write-Error ");
                rest = body;
            }
        }
    }
    out.push_str(rest);
    out.replace("$main.FindAll('Descendants', [System.Windows.Automation.Condition]::TrueCondition)", "Get-AllCached $main")
}

fn base64(bytes: &[u8]) -> String {
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let n = chunk.iter().enumerate().fold(0u32, |acc, (i, b)| acc | (u32::from(*b) << (16 - 8 * i)));
        for i in 0..4 {
            if i <= chunk.len() {
                out.push(TABLE[((n >> (18 - 6 * i)) & 63) as usize] as char);
            } else {
                out.push('=');
            }
        }
    }
    out
}

fn utf16_base64(text: &str) -> String {
    let bytes: Vec<u8> = text.encode_utf16().flat_map(u16::to_le_bytes).collect();
    base64(&bytes)
}

struct Session {
    child: Child,
    stdin: ChildStdin,
    lines: mpsc::Receiver<String>,
}

impl Drop for Session {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

impl Session {
    fn spawn() -> Result<Session, RigError> {
        let mut command = Command::new("powershell.exe");
        command
            .args(["-NoProfile", "-NonInteractive", "-ExecutionPolicy", "Bypass", "-EncodedCommand", &utf16_base64(LOOP)])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null());
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            // niente finestra nera quando Aurora gira come programma a finestre
            command.creation_flags(0x0800_0000);
        }
        let mut child = command.spawn().map_err(|e| RigError(format!("PowerShell non avviabile: {e}")))?;
        let stdin = child.stdin.take().ok_or_else(|| RigError("PowerShell senza ingresso".into()))?;
        let stdout = child.stdout.take().ok_or_else(|| RigError("PowerShell senza uscita".into()))?;
        let (tx, lines) = mpsc::channel();
        std::thread::spawn(move || {
            for line in BufReader::new(stdout).lines().map_while(Result::ok) {
                if tx.send(line).is_err() {
                    break;
                }
            }
        });
        Ok(Session { child, stdin, lines })
    }

    /// Manda lo script e raccoglie l'uscita fino alla riga di fine. `Err` con il testo dell'errore dello script, oppure della
    /// sessione (morta o senza risposta): in quel caso chi chiama la chiude.
    fn run(&mut self, script: &str, timeout: Duration) -> Result<String, Failure> {
        let line = format!("{}\n", base64(adapt(script).as_bytes()));
        self.stdin.write_all(line.as_bytes()).and_then(|()| self.stdin.flush()).map_err(|e| Failure::Session(format!("sessione PowerShell chiusa: {e}")))?;
        let mut output = String::new();
        loop {
            match self.lines.recv_timeout(timeout) {
                Ok(l) if l == END => break,
                Ok(l) => {
                    output.push_str(&l);
                    output.push('\n');
                }
                Err(mpsc::RecvTimeoutError::Timeout) => return Err(Failure::Session("la sessione PowerShell non risponde".into())),
                Err(mpsc::RecvTimeoutError::Disconnected) => return Err(Failure::Session("la sessione PowerShell si e' chiusa".into())),
            }
        }
        match output.strip_prefix(ERR) {
            Some(message) => Err(Failure::Script(message.trim().to_string())),
            None => Ok(output.trim().to_string()),
        }
    }
}

enum Failure {
    /// Lo script e' fallito (JTDX non c'e', controllo non trovato...): la sessione e' a posto.
    Script(String),
    /// La sessione e' morta o bloccata.
    Session(String),
}

/// Una sessione condivisa, aperta al primo uso.
pub struct SharedSession {
    slot: Mutex<Option<Session>>,
}

impl SharedSession {
    pub const fn new() -> SharedSession {
        SharedSession { slot: Mutex::new(None) }
    }

    pub fn run(&self, script: &str) -> Result<String, RigError> {
        self.run_with_timeout(script, TIMEOUT)
    }

    pub fn run_with_timeout(&self, script: &str, timeout: Duration) -> Result<String, RigError> {
        let mut slot = self.slot.lock().unwrap_or_else(|p| p.into_inner());
        // un tentativo in piu' dopo una sessione morta o bloccata, e dopo un errore di script (la memoria dei controlli puo' essere vecchia)
        let mut last = RigError("nessuna risposta".into());
        for attempt in 0..2 {
            if slot.is_none() {
                *slot = Some(Session::spawn()?);
            }
            let Some(session) = slot.as_mut() else { continue };
            match session.run(script, timeout) {
                Ok(text) => return Ok(text),
                Err(Failure::Script(message)) => {
                    last = RigError(message);
                    if attempt == 1 {
                        break;
                    }
                }
                Err(Failure::Session(message)) => {
                    tracing::warn!("{message}: riapro la sessione PowerShell");
                    last = RigError(message);
                    *slot = None;
                }
            }
        }
        Err(last)
    }
}

impl Default for SharedSession {
    fn default() -> Self {
        SharedSession::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exit_after_an_error_becomes_a_throw_so_the_session_survives() {
        let script = "if (-not $p) { Write-Error 'JTDX non in esecuzione'; exit 3 }; if (-not $m) { Write-Error 'finestra non trovata'; exit 14 }; $x";
        assert_eq!(adapt(script), "if (-not $p) { throw 'JTDX non in esecuzione' }; if (-not $m) { throw 'finestra non trovata' }; $x");
    }

    #[test]
    fn an_error_without_exit_is_left_alone() {
        assert_eq!(adapt("Write-Error 'solo avviso'; $y"), "Write-Error 'solo avviso'; $y");
        assert_eq!(adapt("Write-Error $msg"), "Write-Error $msg");
    }

    #[test]
    fn the_search_of_all_controls_goes_through_the_memory() {
        let script = "$all = $main.FindAll('Descendants', [System.Windows.Automation.Condition]::TrueCondition); $el = $all | Select -First 1";
        assert_eq!(adapt(script), "$all = Get-AllCached $main; $el = $all | Select -First 1");
        assert_eq!(adapt("$o.FindAll('Children', $cond)"), "$o.FindAll('Children', $cond)", "la ricerca dei figli non cambia");
    }

    #[test]
    fn base64_matches_the_standard_alphabet() {
        assert_eq!(base64(b""), "");
        assert_eq!(base64(b"f"), "Zg==");
        assert_eq!(base64(b"fo"), "Zm8=");
        assert_eq!(base64(b"foo"), "Zm9v");
        assert_eq!(base64(b"foobar"), "Zm9vYmFy");
        assert_eq!(utf16_base64("A"), "QQA=");
    }

    // Le prove che avviano PowerShell davvero (Windows).
    #[cfg(windows)]
    mod live {
        use super::*;

        #[test]
        fn runs_scripts_in_one_session_that_remembers_variables() {
            let s = SharedSession::new();
            assert_eq!(s.run("$global:n = 41; 'ciao'").unwrap(), "ciao");
            assert_eq!(s.run("$global:n + 1").unwrap(), "42");
        }

        #[test]
        fn a_failing_script_reports_its_message_and_the_session_keeps_working() {
            let s = SharedSession::new();
            let e = s.run("Write-Error 'controllo non trovato'; exit 5").unwrap_err();
            assert!(e.0.contains("controllo non trovato"), "{e}");
            assert_eq!(s.run("1 + 1").unwrap(), "2");
        }

        #[test]
        fn accented_text_survives_the_round_trip() {
            let s = SharedSession::new();
            assert_eq!(s.run("'perche\u{301} \u{e8} cos\u{ec}'").unwrap(), "perche\u{301} \u{e8} cos\u{ec}");
        }

        #[test]
        fn a_stuck_script_times_out_and_the_next_command_gets_a_fresh_session() {
            let s = SharedSession::new();
            s.run("$global:n = 1").unwrap();
            let e = s.run_with_timeout("Start-Sleep -Seconds 6", Duration::from_millis(700)).unwrap_err();
            assert!(e.0.contains("non risponde"), "{e}");
            // la sessione nuova non ricorda la variabile di prima
            assert_eq!(s.run("if ($null -eq $global:n) { 'nuova' } else { 'vecchia' }").unwrap(), "nuova");
        }
    }
}
