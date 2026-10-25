# Prepara Hamlib (rigctld) per l'installatore di Windows: copia i file in desktop\src-tauri\hamlib-bundle\ (esclusa da git).
#
#   .\scripts\prepara-hamlib.ps1                       -> cerca la cartella sorgente da sola
#   .\scripts\prepara-hamlib.ps1 -Hamlib C:\JTDX64\159\bin
#
# La cartella sorgente (cartella con rigctld*.exe, rigctl*.exe e le DLL di Hamlib) si sceglie, nell'ordine: parametro -Hamlib,
# variabile d'ambiente AURORA_HAMLIB_SRC, ultima versione di JTDX (C:\JTDX64\<versione>\bin), WSJT-X (C:\WSJT\wsjtx\bin).
# Non scarica nulla. Le DLL necessarie si ricavano leggendo le importazioni dei due eseguibili (e, a catena, delle DLL):
# si copiano solo quelle che non sono di Windows. Se manca qualcosa lo script si ferma con un messaggio chiaro.
param(
    [string]$Hamlib = $env:AURORA_HAMLIB_SRC
)

$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
$bundle = Join-Path $root 'desktop\src-tauri\hamlib-bundle'
$licenze = Join-Path $root 'desktop\src-tauri\hamlib-licenze'

function Cartella-Valida([string]$dir) {
    if (-not $dir -or -not (Test-Path -LiteralPath $dir -PathType Container)) { return $false }
    $d = Get-ChildItem -LiteralPath $dir -Filter 'rigctld*.exe' -File -ErrorAction SilentlyContinue
    return [bool]$d
}

if (-not $Hamlib) {
    $candidate = @()
    if (Test-Path 'C:\JTDX64') {
        $candidate += Get-ChildItem 'C:\JTDX64' -Directory | Sort-Object Name -Descending | ForEach-Object { Join-Path $_.FullName 'bin' }
    }
    $candidate += 'C:\WSJT\wsjtx\bin'
    $Hamlib = $candidate | Where-Object { Cartella-Valida $_ } | Select-Object -First 1
}
if (-not (Cartella-Valida $Hamlib)) {
    throw @"
Hamlib non trovato: serve una cartella con rigctld.exe (o rigctld-jtdx.exe), rigctl.exe e le DLL di Hamlib.
Indicala con  .\scripts\prepara-hamlib.ps1 -Hamlib <cartella>  oppure con la variabile AURORA_HAMLIB_SRC
(di base si cerca in C:\JTDX64\<versione>\bin e C:\WSJT\wsjtx\bin: installa JTDX o WSJT-X, oppure usa il pacchetto Windows ufficiale di Hamlib, https://github.com/Hamlib/Hamlib/releases).
"@
}
$Hamlib = (Resolve-Path -LiteralPath $Hamlib).Path
Write-Host "Hamlib: sorgente $Hamlib"

function Scegli-Exe([string]$prefisso) {
    $files = Get-ChildItem -LiteralPath $Hamlib -Filter "$prefisso*.exe" -File |
        Where-Object { $_.Name -notlike 'rigctlcom*' -and ($prefisso -eq 'rigctld' -or $_.Name -notlike 'rigctld*') }
    $f = $files | Sort-Object { $_.Name.Length } | Select-Object -First 1
    if (-not $f) { throw "Manca $prefisso*.exe in $Hamlib" }
    return $f
}
$rigctld = Scegli-Exe 'rigctld'
$rigctl = Scegli-Exe 'rigctl'

# --- importazioni di un file PE (nomi delle DLL) ---
function Importazioni([string]$path) {
    $b = [System.IO.File]::ReadAllBytes($path)
    $pe = [BitConverter]::ToInt32($b, 0x3c)
    $nSez = [BitConverter]::ToUInt16($b, $pe + 6)
    $optSize = [BitConverter]::ToUInt16($b, $pe + 20)
    $opt = $pe + 24
    $magic = [BitConverter]::ToUInt16($b, $opt)
    $dd = $opt + $(if ($magic -eq 0x20b) { 112 } else { 96 })
    $impRva = [BitConverter]::ToUInt32($b, $dd + 8)
    if ($impRva -eq 0) { return @() }
    $sez = @()
    for ($i = 0; $i -lt $nSez; $i++) {
        $o = $opt + $optSize + $i * 40
        $sez += [pscustomobject]@{ Va = [BitConverter]::ToUInt32($b, $o + 12); Vs = [BitConverter]::ToUInt32($b, $o + 8); Rs = [BitConverter]::ToUInt32($b, $o + 16); Rp = [BitConverter]::ToUInt32($b, $o + 20) }
    }
    function Off([uint32]$rva) {
        foreach ($s in $sez) { if ($rva -ge $s.Va -and $rva -lt ($s.Va + [Math]::Max($s.Vs, $s.Rs))) { return [int]($rva - $s.Va + $s.Rp) } }
        return -1
    }
    $out = @()
    $p = Off $impRva
    while ($p -ge 0) {
        $nome = [BitConverter]::ToUInt32($b, $p + 12)
        if ($nome -eq 0) { break }
        $s = Off $nome
        $e = $s
        while ($b[$e] -ne 0) { $e++ }
        $out += [System.Text.Encoding]::ASCII.GetString($b, $s, $e - $s)
        $p += 20
    }
    return $out
}

$sys32 = Join-Path $env:windir 'System32'
function E-Di-Windows([string]$dll) {
    $l = $dll.ToLowerInvariant()
    if ($l -like 'api-ms-*' -or $l -like 'ext-ms-*') { return $true }
    return (Test-Path -LiteralPath (Join-Path $sys32 $dll))
}

# chiusura delle dipendenze: partendo dai due eseguibili, tutte le DLL non di Windows che servono
$servono = [ordered]@{}
$coda = New-Object System.Collections.Queue
$coda.Enqueue($rigctld.FullName); $coda.Enqueue($rigctl.FullName)
while ($coda.Count -gt 0) {
    $file = $coda.Dequeue()
    foreach ($dll in (Importazioni $file)) {
        if ($servono.Contains($dll.ToLowerInvariant())) { continue }
        $qui = Join-Path $Hamlib $dll
        if (Test-Path -LiteralPath $qui) {
            $servono[$dll.ToLowerInvariant()] = $qui
            $coda.Enqueue($qui)
        } elseif (-not (E-Di-Windows $dll)) {
            throw "Manca la DLL '$dll' (richiesta da $([System.IO.Path]::GetFileName($file))) in ${Hamlib}: la cartella sorgente non e' completa."
        }
    }
}
if (-not ($servono.Keys | Where-Object { $_ -like 'libhamlib*' })) { throw "Nessuna libhamlib*.dll tra le dipendenze di $($rigctld.Name): cartella sorgente non valida." }

# --- copia nel pacchetto (si svuota prima, tranne .gitkeep: niente file vecchi) ---
New-Item -ItemType Directory -Force $bundle | Out-Null
Get-ChildItem -LiteralPath $bundle -Force | Where-Object { $_.Name -ne '.gitkeep' } | Remove-Item -Recurse -Force
Copy-Item $rigctld.FullName (Join-Path $bundle 'rigctld.exe')
Copy-Item $rigctl.FullName (Join-Path $bundle 'rigctl.exe')
foreach ($p in $servono.Values) { Copy-Item $p (Join-Path $bundle ([System.IO.Path]::GetFileName($p))) }
if (-not (Test-Path $licenze)) { throw "Cartella delle licenze mancante: $licenze" }
Copy-Item (Join-Path $licenze '*') $bundle -Force

# --- controlli: parte, dice la versione, conosce la FT-2000 e le FT-DX ---
$versione = (& (Join-Path $bundle 'rigctld.exe') -V 2>&1 | Out-String).Trim()
if ($LASTEXITCODE -ne 0 -or $versione -notmatch 'Hamlib') { throw "rigctld.exe del pacchetto non parte (versione letta: '$versione'): controlla le DLL." }
$modelli = (& (Join-Path $bundle 'rigctl.exe') -l 2>&1 | Out-String)
foreach ($m in 'FT-2000', 'FTDX-101D', 'FTDX-5000', 'Dummy') {
    if ($modelli -notmatch [regex]::Escape($m)) { throw "Nell'elenco dei modelli di questo Hamlib manca '$m': versione troppo vecchia o incompleta." }
}
$nMod = @($modelli -split "`n" | Where-Object { $_ -match '^\s*\d+\s' }).Count

$righe = @(
    'Hamlib incluso in Aurora (copiato, non scaricato)',
    "Versione:  $versione",
    "Modelli:   $nMod (FT-2000, FTDX-101D, FTDX-5000 inclusi)",
    "Origine:   $Hamlib ($($rigctld.Name), $($rigctl.Name) rinominati rigctld.exe e rigctl.exe)",
    "Sorgenti:  https://github.com/Hamlib/Hamlib (stessa versione/commit indicati sopra)",
    'Licenze:   LGPL-2.1+ (libreria), GPL-2.0+ (programmi); testi nei file LGPL-2.1.txt e GPL-3.0.txt',
    '',
    'SHA-256:'
)
$righe += Get-ChildItem $bundle -File | Where-Object { $_.Extension -in '.exe', '.dll' } | Sort-Object Name | ForEach-Object { '  {0}  {1}' -f (Get-FileHash $_.FullName -Algorithm SHA256).Hash.ToLower(), $_.Name }
Set-Content (Join-Path $bundle 'VERSIONE.txt') $righe -Encoding utf8

Write-Host "Hamlib pronto in $bundle :"
Get-ChildItem $bundle -File | Where-Object { $_.Name -ne '.gitkeep' } | ForEach-Object { Write-Host ('  {0}  ({1:N0} KB)' -f $_.Name, ($_.Length / 1KB)) }
Write-Host $versione
