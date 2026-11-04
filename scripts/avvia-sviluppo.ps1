# Avvia Aurora da pulito: chiude le copie rimaste dall'avvio precedente (core, pannelli) e ne apre una sola.
# Una sola finestra di Windows Terminal (sempre la stessa, "aurora") con un solo pannello: il core, che serve anche l'interfaccia.

#   .\scripts\avvia-sviluppo.ps1            -> stazione simulata
#   .\scripts\avvia-sviluppo.ps1 -Motore    -> il motore di Aurora sulla scheda audio (-Audio "USB Audio" per sceglierla)
#   -Chiusa -> tiene chiusa la trasmissione (AURORA_ALLOW_TX=0); di norma lo script la apre: servono per CQ e risposte con doppio clic
#   -Sviluppo -> aggiunge il server di sviluppo dell'interfaccia (porta 5173), di norma inutile: il core serve l'interfaccia compilata
param(
    [switch]$Motore,
    [switch]$Chiusa,
    [switch]$Sviluppo,
    [string]$Audio = ''
)

$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot

# Il motore di Aurora: ascolta la scheda audio e decodifica da solo, con il file dei paesi incluso (scaricato dagli script di compilazione).
$engineEnv = ''
if ($Motore) {
    $engineEnv = "`$env:AURORA_ENGINE='native'; "
    $cty = Join-Path $root 'desktop\src-tauri\cty\cty.dat'
    if (Test-Path $cty) { $engineEnv += "`$env:AURORA_CTY_PATH='$cty'; " }
    if ($Audio) { $engineEnv += "`$env:AURORA_AUDIO_DEVICE='$Audio'; " }
}

function Test-Port([int]$Port) {
    [bool](Get-NetTCPConnection -LocalPort $Port -State Listen -ErrorAction SilentlyContinue)
}

# Avvio pulito: via le copie dell'avvio precedente (il core, la sua compilazione, i pannelli che li ospitavano).
$vecchi = Get-CimInstance Win32_Process | Where-Object {
    $_.ProcessId -ne $PID -and ($_.Name -eq 'aurora.exe' -or $_.CommandLine -match 'cargo run -p aurora|npm run dev')
}
foreach ($v in $vecchi) { Stop-Process -Id $v.ProcessId -Force -ErrorAction SilentlyContinue }
if ($vecchi) { Start-Sleep -Seconds 2 }

# Il core va per primo: l'interfaccia fa da proxy verso di lui.
$services = @()
if (Test-Path "$root\app\dist\index.html") {
    # L'interfaccia compilata si apre anche dall'indirizzo del core (porta 8787), senza il server di sviluppo.
    $engineEnv += "`$env:AURORA_UI_DIR='$root\app\dist'; "
}
if (-not $Chiusa) { $engineEnv += "`$env:AURORA_ALLOW_TX='1'; " }
# Accesso senza codice: solo dalla rete di casa, mai da internet o proxy.
$engineEnv += "`$env:AURORA_OPEN_ACCESS='1'; "
if (-not (Test-Port 8787)) {
    $services += @{
        Name = 'Aurora core'; Color = '#3fd6c8'; Port = 8787; Dir = "$root\core"
        Cmd = "$engineEnv`$env:AURORA_BIND='0.0.0.0:8787'; `$env:AURORA_DATA_DIR='$root\data'; cargo run -p aurora"
    }
}
if ($Sviluppo -and (Test-Path "$root\app\package.json") -and -not (Test-Port 5173)) {
    $services += @{
        Name = 'Aurora interfaccia'; Color = '#f2a93b'; Port = 5173; Dir = "$root\app"
        Cmd = 'npm run dev -- --host'
    }
}

if ($services.Count -eq 0) {
    Write-Host 'Tutto e'' gia'' acceso: niente da avviare.'
}
elseif (Get-Command wt.exe -ErrorAction SilentlyContinue) {
    # Un'unica invocazione di wt.exe: il ';' e' un elemento dell'array, non un separatore di PowerShell.
    # "-w aurora": tutto nella stessa finestra con nome, mai una finestra nuova a ogni avvio
    $wtArgs = @('-w', 'aurora')
    for ($i = 0; $i -lt $services.Count; $i++) {
        $s = $services[$i]
        if ($i -eq 0) { $wtArgs += 'new-tab' } else { $wtArgs += @(';', 'split-pane', '-H') }
        $wtArgs += @(
            '--title', $s.Name, '--suppressApplicationTitle', '--tabColor', $s.Color,
            '-d', $s.Dir, '--',
            'powershell', '-NoExit', '-ExecutionPolicy', 'Bypass', '-Command', $s.Cmd
        )
    }
    & wt.exe @wtArgs
}
else {
    foreach ($s in $services) {
        Start-Process powershell -WorkingDirectory $s.Dir -ArgumentList @(
            '-NoExit', '-ExecutionPolicy', 'Bypass', '-Command', $s.Cmd
        )
    }
}

# "Avviato" non vuol dire "pronto": attende che le porte siano davvero in ascolto.
$pending = $services | ForEach-Object { $_.Port }
for ($t = 0; $t -lt 240 -and $pending.Count -gt 0; $t++) {
    $pending = @($pending | Where-Object { -not (Test-Port $_) })
    if ($pending.Count -gt 0) { Start-Sleep -Milliseconds 500 }
}
if ($pending.Count -gt 0) {
    Write-Host "Ancora in attesa sulle porte: $($pending -join ', '). La prima compilazione puo' richiedere qualche minuto: guarda la tab."
}

# Indirizzi di rete reali, per provare dal telefono (escluse interfacce virtuali e link-local).
$lan = Get-NetIPAddress -AddressFamily IPv4 -ErrorAction SilentlyContinue |
    Where-Object {
        $_.IPAddress -notlike '169.254.*' -and $_.IPAddress -ne '127.0.0.1' -and
        $_.InterfaceAlias -notmatch 'vEthernet|WSL|Docker|Loopback|Hyper-V|VirtualBox|VMware'
    } | ForEach-Object { $_.IPAddress }

Write-Host ''
Write-Host 'Aurora - ambiente di sviluppo'
Write-Host '  Core:         http://127.0.0.1:8787/api/v1/health'
if ($Sviluppo) { Write-Host '  Sviluppo:     http://127.0.0.1:5173' }
foreach ($ip in $lan) { Write-Host "  Dal telefono: http://${ip}:8787  (stessa rete Wi-Fi)" }
Write-Host '  Per spegnere: chiudi la tab di Windows Terminal.'
