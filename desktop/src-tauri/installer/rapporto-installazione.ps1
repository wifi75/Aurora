# Il rapporto dell'installatore di Aurora: l'installatore NSIS lo estrae nella sua cartella temporanea e lo lancia nascosto, con un
# tempo massimo (nsExec /TIMEOUT), cosi' senza rete non resta appeso. Solo testo ASCII: Windows PowerShell 5.1 legge i .ps1 senza BOM
# con la codifica ANSI.
#
#   -Azione ambiente  scrive le righe per il registro: antivirus (SecurityCenter2) e, con -Cartella, i programmi ancora aperti da quella
#                     cartella (file bloccati)
#   -Azione manda     se i rapporti non sono spenti in Aurora (rapporti.json, "auto_errors": false) manda un rapporto breve su
#                     /v1/report (area "installatore") e il registro ripulito come zip su /v1/bundle (motivo "installazione")
#   -Azione prova     scrive il registro ripulito e il rapporto che partirebbe, senza mandare nulla (per le prove)
#
# Codici di uscita: 0 fatto (rapporto mandato), 2 rapporti spenti in Aurora, 3 non mandato (rete o servizio), 4 registro mancante.
# La pulizia segue le regole di pulisci() in core/crates/aurora/src/rapporti.rs: nome utente, nome del PC e dominio (i nomi generici come
# User, Admin, Radio restano), percorsi C:\Users\<nome>, email, indirizzi IP, nominativi, locatori, segreti e codici lunghi.
param(
  [string]$Azione = 'manda',
  [string]$Registro = '',
  [string]$Versione = '?',
  [string]$Motivo = '',
  [string]$Cartella = '',
  # solo per le prove: un altro indirizzo del servizio (per esempio un server locale)
  [string]$Servizio = ''
)

$ErrorActionPreference = 'Stop'
$ProgressPreference = 'SilentlyContinue'

# lo stesso servizio e la stessa chiave d'ingresso (pubblica) di Aurora: INDIRIZZO e CHIAVE_INGRESSO in rapporti.rs
$Indirizzo = 'https://aurora-rapporti.tizianocassone.workers.dev'
if ($Servizio) { $Indirizzo = $Servizio.TrimEnd('/') }
$Chiave = 'aurora-rapporti-v1'
$Punto = [string][char]0x00B7

# ------------------------------------------------------------------------------------------------------------------------------------
# La pulizia dei dati personali
# ------------------------------------------------------------------------------------------------------------------------------------

$Comuni = @('user', 'users', 'utente', 'admin', 'administrator', 'amministratore', 'owner', 'guest', 'default', 'radio', 'ham', 'shack',
  'station', 'stazione', 'home', 'desktop', 'laptop', 'notebook', 'computer', 'workgroup', 'pc', 'windows')
$Separatori = '[\s"''(),;\[\]{}<>=]+'

function Test-Locatore([string]$p) {
  return $p -cmatch '^[A-R]{2}[0-9]{2}(?:[A-Xa-x]{2}(?:[0-9]{2})?)?$'
}

function Test-Nominativo([string]$p) {
  if ($p -cnotmatch '^[A-Z0-9]{3,10}$') { return $false }
  # la parte prima dell'ultima cifra e' il prefisso, quella dopo il suffisso (solo lettere)
  $m = [regex]::Match($p, '^([A-Z0-9]*)[0-9]([A-Z]*)$')
  if (-not $m.Success) { return $false }
  $pre = $m.Groups[1].Value
  $suf = $m.Groups[2].Value
  return ($suf.Length -ge 1 -and $suf.Length -le 4 -and $pre.Length -ge 1 -and $pre.Length -le 3 -and $pre -cmatch '[A-Z]')
}

function Test-Ipv4([string]$p) {
  $parti = $p.Split('.')
  if ($parti.Count -ne 4) { return $false }
  foreach ($x in $parti) {
    if ($x -notmatch '^[0-9]{1,3}$' -or [int]$x -gt 255) { return $false }
  }
  return $true
}

function Test-Ipv4Porta([string]$p) {
  $i = $p.LastIndexOf(':')
  if ($i -lt 0) { return $false }
  return (Test-Ipv4 $p.Substring(0, $i)) -and ($p.Substring($i + 1) -match '^[0-9]+$')
}

function Test-Ipv6([string]$p) {
  $p = $p.TrimStart('[')
  $p = $p.Split(']')[0]
  $p = $p.Split('%')[0]
  if (([regex]::Matches($p, ':')).Count -lt 2) { return $false }
  if ($p -notmatch '^[0-9A-Fa-f:.]+$') { return $false }
  if (-not ($p.Contains('::') -or $p -match '[A-Fa-f]')) { return $false }
  foreach ($g in $p.Split(':')) {
    if ($g.Length -gt 4 -and -not (Test-Ipv4 $g)) { return $false }
  }
  return $true
}

function Test-NomeSegreto([string]$nome) {
  $basso = $nome.ToLowerInvariant()
  if ($basso.Length -eq 0) { return $false }
  foreach ($w in @('password', 'passwd', 'pwd', 'key', 'token', 'secret', 'code', 'pin', 'credential')) {
    if ($basso.EndsWith($w)) { return $true }
  }
  return $false
}

function Get-ParolaPulita([string]$parola) {
  if ($parola.Length -eq 0 -or $parola.Contains('_')) { return $parola }
  if (Test-Locatore $parola) { return '<locatore>' }
  if (Test-Nominativo $parola) { return '<nominativo>' }
  if ($parola.Length -ge 24 -and $parola -match '[0-9]' -and $parola -match '[A-Za-z]' -and -not $parola.StartsWith('0x')) { return '<codice>' }
  return $parola
}

function Get-PezzoPulito([string]$pezzo) {
  $at = $pezzo.IndexOf('@')
  if ($at -ge 0) {
    $utente = $pezzo.Substring(0, $at)
    $dominio = $pezzo.Substring($at + 1)
    if ($utente.Length -gt 0 -and $dominio.Contains('.') -and -not $dominio.StartsWith('.')) { return '<email>' }
  }
  $nudo = $pezzo.TrimEnd('.', ':')
  $coda = $pezzo.Substring($nudo.Length)
  if ((Test-Ipv4 $nudo) -or (Test-Ipv4Porta $nudo) -or (Test-Ipv6 $nudo)) { return '<ip>' + $coda }
  return [regex]::Replace($pezzo, '[A-Za-z0-9_]+', [System.Text.RegularExpressions.MatchEvaluator] { param($m) Get-ParolaPulita $m.Value })
}

function Get-TestoPulito([string]$testo, [string[]]$noti) {
  $s = $testo
  # i valori noti per primi: il nome utente potrebbe non stare in un percorso; i lunghi anche dentro altre parole, i corti solo interi
  $validi = @($noti | ForEach-Object { "$_".Trim() } | Where-Object { $_.Length -ge 3 -and ($Comuni -notcontains $_.ToLowerInvariant()) } | Sort-Object -Property Length -Descending)
  foreach ($n in $validi) {
    $e = [regex]::Escape($n)
    if ($n.Length -ge 6) {
      $s = [regex]::Replace($s, $e, '<personale>', 'IgnoreCase')
    } else {
      $s = [regex]::Replace($s, '(?<![\p{L}\p{N}])' + $e + '(?![\p{L}\p{N}])', '<personale>', 'IgnoreCase')
    }
  }
  # C:\Users\mario\... -> C:\Users\<utente>\... (anche Documents and Settings e /home/); un nome gia' tolto in parte
  # ("<personale> Rossi") si toglie fino al separatore
  $s = [regex]::Replace($s, '(?<=[\\/])(users|documents and settings|home)([\\/]+)(?!<utente>)(?:<[^\\/"''\r\n\t]*|[^\\/"''\r\n\t:;,)\]>]+)', '$1$2<utente>', 'IgnoreCase')
  # parola per parola: email, IP, nominativi, locatori, segreti dopo "nome=" e codici lunghi
  $parti = [regex]::Split($s, '(' + $Separatori + ')')
  $out = New-Object System.Text.StringBuilder
  $segreto = $false
  for ($i = 0; $i -lt $parti.Count; $i++) {
    $x = $parti[$i]
    if ($x.Length -eq 0) { continue }
    if ($x -match ('^' + $Separatori + '$')) {
      [void]$out.Append($x)
      if ($x.Contains("`n")) { $segreto = $false }
      continue
    }
    if ($segreto) {
      if ($x -eq ':') { [void]$out.Append($x) } else { [void]$out.Append('***'); $segreto = $false }
      continue
    }
    # "password=", "api_key:", "token: ...": il valore dopo e' segreto
    if (Test-NomeSegreto $x.TrimEnd(':')) {
      $annuncia = $x.EndsWith(':')
      if (-not $annuncia -and $i + 1 -lt $parti.Count) {
        $dopo = $parti[$i + 1].TrimStart(' ', '"', "'")
        if ($dopo.Length -gt 0) {
          $annuncia = $dopo.StartsWith('=') -or $dopo.StartsWith(':')
        } elseif ($i + 2 -lt $parti.Count) {
          $annuncia = $parti[$i + 2].StartsWith('=') -or $parti[$i + 2].StartsWith(':')
        }
      }
      if ($annuncia) {
        [void]$out.Append($x)
        $segreto = $true
        continue
      }
    }
    [void]$out.Append((Get-PezzoPulito $x))
  }
  return $out.ToString()
}

function Get-Noti {
  return @($env:USERNAME, $env:COMPUTERNAME, $env:USERDOMAIN) | Where-Object { $_ }
}

# ------------------------------------------------------------------------------------------------------------------------------------
# Il PC, come lo descrive Aurora (sistema() e pc() in rapporti.rs)
# ------------------------------------------------------------------------------------------------------------------------------------

function Get-Sistema {
  try {
    $k = Get-ItemProperty -LiteralPath 'HKLM:\SOFTWARE\Microsoft\Windows NT\CurrentVersion'
    $prodotto = "$($k.ProductName)"
    $build = "$($k.CurrentBuild)"
    $n = 0
    # dalla build 22000 e' Windows 11, anche se ProductName dice ancora 10
    if ([int]::TryParse($build, [ref]$n) -and $n -ge 22000) { $prodotto = $prodotto.Replace('Windows 10', 'Windows 11') }
    $ubr = if ($null -ne $k.UBR) { ".$($k.UBR)" } else { '' }
    return "$prodotto $($k.DisplayVersion) (build $build$ubr)"
  } catch {
    return 'Windows'
  }
}

function Get-Pc {
  $cpu = '?'
  try {
    $cpu = ((Get-ItemProperty -LiteralPath 'HKLM:\HARDWARE\DESCRIPTION\System\CentralProcessor\0').ProcessorNameString -split '\s+' | Where-Object { $_ }) -join ' '
  } catch { }
  if ($cpu.Length -gt 80) { $cpu = $cpu.Substring(0, 80) }
  $ram = 0
  try {
    $ram = [int][math]::Round((Get-CimInstance -ClassName Win32_ComputerSystem -OperationTimeoutSec 3).TotalPhysicalMemory / 1GB)
  } catch { }
  return "$cpu $Punto $([Environment]::ProcessorCount) core $Punto $ram GB"
}

# ------------------------------------------------------------------------------------------------------------------------------------
# Le azioni
# ------------------------------------------------------------------------------------------------------------------------------------

function Get-Ambiente {
  $righe = New-Object System.Collections.Generic.List[string]
  try {
    $prodotti = @(Get-CimInstance -Namespace 'root/SecurityCenter2' -ClassName AntiVirusProduct -OperationTimeoutSec 4)
    if ($prodotti.Count -eq 0) {
      $righe.Add('antivirus: nessuno registrato in Sicurezza di Windows')
    } else {
      # productState: il secondo byte dice se e' acceso (10, 11), il terzo se le firme sono aggiornate (00)
      $nomi = foreach ($p in $prodotti) {
        $h = '{0:x6}' -f [int]$p.productState
        $acceso = if (@('10', '11') -contains $h.Substring(2, 2)) { 'acceso' } else { 'spento' }
        $firme = if ($h.Substring(4, 2) -eq '00') { 'aggiornato' } else { 'firme vecchie' }
        "$($p.displayName) ($acceso, $firme)"
      }
      $righe.Add('antivirus: ' + ($nomi -join '; '))
    }
  } catch {
    $righe.Add('antivirus: non letto (' + $_.Exception.GetType().Name + ')')
  }
  if ($Cartella) {
    $dir = $Cartella.TrimEnd('\') + '\'
    try {
      $aperti = @(Get-Process -ErrorAction SilentlyContinue | Where-Object { $_.Path -and $_.Path.StartsWith($dir, [StringComparison]::OrdinalIgnoreCase) } | ForEach-Object { $_.ProcessName } | Sort-Object -Unique)
      if ($aperti.Count -gt 0) {
        $righe.Add('AVVISO: programmi ancora aperti dalla cartella di installazione (file bloccati): ' + ($aperti -join ', '))
      } else {
        $righe.Add('nessun programma aperto dalla cartella di installazione')
      }
    } catch { }
  }
  $testo = $righe -join "`r`n"
  if ($testo.Length -gt 900) { $testo = $testo.Substring(0, 900) }
  return $testo
}

function Read-Registro {
  if (-not $Registro -or -not (Test-Path -LiteralPath $Registro)) { return $null }
  # UTF-16 con BOM (scritto dall'installatore): Get-Content lo riconosce da solo
  $t = Get-Content -LiteralPath $Registro -Raw
  if ($t.Length -gt 200000) { $t = $t.Substring($t.Length - 200000) }
  return $t
}

# Il messaggio del rapporto: prima il motivo (il servizio raggruppa sui primi 300 caratteri), poi gli avvisi e gli errori del registro,
# alla fine modo e versioni
function Get-Messaggio([string]$pulito, [string]$motivo) {
  # senza le righe dell'elenco della finestra (tra "--- dettagli" e "--- fine dei dettagli"): non sono passi dell'installatore
  $righe = ([regex]::Replace($pulito, '(?s)--- dettagli dell''installazione.*?(--- fine dei dettagli[^\r\n]*|$)', '')) -split "`r?`n"
  $problemi = @($righe | Where-Object { $_ -match '(AVVISO|ERRORE):' } | ForEach-Object { ($_ -replace '^\[[0-9:]+\]\s*', '').Trim() } | Where-Object { $_ -ne "ERRORE: $motivo" } | Select-Object -First 6)
  $modo = ($righe | Where-Object { $_ -like 'modo:*' } | Select-Object -First 1)
  $prima = ($righe | Where-Object { $_ -like 'versione installata prima:*' } | Select-Object -First 1)
  $m = "installazione non riuscita: $motivo"
  if ($problemi.Count -gt 0) { $m += ' | ' + ($problemi -join ' | ') }
  $coda = @($modo, $prima) | Where-Object { $_ }
  if ($coda.Count -gt 0) { $m += ' (' + ($coda -join '; ') + ')' }
  if ($m.Length -gt 1000) { $m = $m.Substring(0, 1000) }
  return $m
}

function Get-Installazione([string]$dati) {
  $f = Join-Path $dati 'rapporti-installazione.txt'
  try {
    if (Test-Path -LiteralPath $f) {
      $id = (Get-Content -LiteralPath $f -Raw).Trim()
      if ($id.Length -eq 36) { return $id }
    }
  } catch { }
  return [guid]::NewGuid().ToString()
}

function Test-Spenti([string]$dati) {
  # come Aurora: se rapporti.json non c'e' o non si legge valgono le scelte di fabbrica (accesi)
  $f = Join-Path $dati 'rapporti.json'
  try {
    if (Test-Path -LiteralPath $f) {
      $j = Get-Content -LiteralPath $f -Raw | ConvertFrom-Json
      return ($j.auto_errors -eq $false)
    }
  } catch { }
  return $false
}

function New-Zip([string]$log, [string]$info) {
  Add-Type -AssemblyName System.IO.Compression
  $ms = New-Object System.IO.MemoryStream
  $zip = New-Object System.IO.Compression.ZipArchive($ms, [System.IO.Compression.ZipArchiveMode]::Create, $true)
  $utf8 = New-Object System.Text.UTF8Encoding($false)
  foreach ($voce in @(@('info.json', $info), @('installazione.log', $log))) {
    $w = New-Object System.IO.StreamWriter($zip.CreateEntry($voce[0]).Open(), $utf8)
    $w.Write($voce[1])
    $w.Dispose()
  }
  $zip.Dispose()
  return $ms.ToArray()
}

function Send-Rapporto([string]$percorso, [byte[]]$corpo, [string]$tipo, [hashtable]$intestazioni, [int]$secondi) {
  $h = @{ 'X-Aurora-Key' = $Chiave }
  foreach ($k in $intestazioni.Keys) { $h[$k] = $intestazioni[$k] }
  $r = Invoke-WebRequest -Uri ($Indirizzo + $percorso) -Method Post -Body $corpo -ContentType $tipo -Headers $h -UserAgent 'Aurora-installatore' -TimeoutSec $secondi -UseBasicParsing
  return [int]$r.StatusCode
}

function Invoke-Manda([bool]$davvero) {
  if (-not $Registro -or -not (Test-Path -LiteralPath $Registro)) { [Console]::Out.Write('registro mancante'); return 4 }
  # prima l'ambiente in fondo al registro (antivirus, programmi che tengono i file): resta sul PC anche con i rapporti spenti.
  # Una PowerShell sola per tutto: su alcuni PC (antivirus che la controllano) parte in decine di secondi
  if ($davvero) {
    try { [IO.File]::AppendAllText($Registro, (Get-Ambiente) + "`r`n", [Text.Encoding]::Unicode) } catch { }
  }
  $testo = Read-Registro
  $dati = Split-Path -Parent $Registro
  if ($davvero -and (Test-Spenti $dati)) { [Console]::Out.Write('rapporti spenti in Aurora'); return 2 }
  $noti = Get-Noti
  $pulito = Get-TestoPulito $testo $noti
  $motivo = Get-TestoPulito $Motivo $noti
  $messaggio = Get-Messaggio $pulito $motivo
  $installazione = Get-Installazione $dati
  $giorno = [DateTime]::UtcNow.ToString('yyyy-MM-dd')
  $sistema = Get-Sistema
  $pc = Get-Pc
  $rapporto = [ordered]@{
    schema = 1; id = [guid]::NewGuid().ToString(); installazione = $installazione; giorno = $giorno; versione = $Versione
    sistema = $sistema; pc = $pc; area = 'installatore'; livello = 'error'; origine = 'installatore'; messaggio = $messaggio; conteggio = 1
  }
  $json = $rapporto | ConvertTo-Json -Compress
  $info = [ordered]@{
    schema = 1; installazione = $installazione; giorno = $giorno; versione = $Versione; sistema = $sistema; pc = $pc; motivo = $motivo
    nota = 'Registro dell''installatore senza dati personali: nome utente, nome del PC, percorsi con il nome utente, IP, email, nominativi e codici sono tolti.'
  } | ConvertTo-Json
  if (-not $davvero) {
    [Console]::Out.Write($json + "`r`n----`r`n" + $pulito)
    return 0
  }
  [Net.ServicePointManager]::SecurityProtocol = [Net.ServicePointManager]::SecurityProtocol -bor [Net.SecurityProtocolType]::Tls12
  try {
    $stato = Send-Rapporto '/v1/report' ([Text.Encoding]::UTF8.GetBytes($json)) 'application/json; charset=utf-8' @{} 6
    if ($stato -lt 200 -or $stato -gt 299) { [Console]::Out.Write("il servizio ha risposto $stato"); return 3 }
  } catch {
    [Console]::Out.Write('rapporto non mandato: ' + $_.Exception.GetType().Name)
    return 3
  }
  # il registro ripulito, se ci sta: il codice breve e' quello con cui lo si ritrova nel servizio
  $codice = ([guid]::NewGuid().ToString('N').Substring(0, 8)).ToUpperInvariant()
  $riassunto = $messaggio
  if ($riassunto.Length -gt 1500) { $riassunto = $riassunto.Substring(0, 1500) }
  $esito = "rapporto mandato"
  try {
    $zip = New-Zip $pulito $info
    $stato = Send-Rapporto '/v1/bundle' $zip 'application/zip' @{
      'X-Aurora-Installazione' = $installazione; 'X-Aurora-Codice' = $codice; 'X-Aurora-Motivo' = 'installazione'
      'X-Aurora-Versione' = $Versione; 'X-Aurora-Riassunto' = [Uri]::EscapeDataString($riassunto)
    } 10
    if ($stato -ge 200 -and $stato -le 299) { $esito += ", pacchetto $codice" }
  } catch {
    $esito += ', pacchetto non mandato'
  }
  [Console]::Out.Write($esito)
  return 0
}

try {
  switch ($Azione) {
    'ambiente' { [Console]::Out.Write((Get-Ambiente)); exit 0 }
    'prova' { exit (Invoke-Manda $false) }
    default { exit (Invoke-Manda $true) }
  }
} catch {
  [Console]::Out.Write('errore: ' + $_.Exception.GetType().Name)
  exit 3
}
