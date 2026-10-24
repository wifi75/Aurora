# Prepara il pacchetto di installazione di Aurora per Windows e la sua firma, pronti da mettere in una release di GitHub.
#
#   .\scripts\rilascio-windows.ps1                 -> controlla le versioni, compila l'interfaccia e il programma, firma, scrive dist-release\
#   .\scripts\rilascio-windows.ps1 -SoloControllo  -> controlla soltanto che le versioni coincidano
#
# La chiave PRIVATA di firma sta fuori dal repository (di norma %USERPROFILE%\.aurora-signing\aurora.key): se si perde,
# le copie gia' installate non potranno piu' aggiornarsi. Lo script NON pubblica nulla e non fa push: a fine lavoro stampa il
# comando per creare la release, da lanciare a mano.
param(
    [string]$Chiave = (Join-Path $env:USERPROFILE '.aurora-signing\aurora.key'),
    [switch]$SoloControllo
)

$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot

function Versione-Toml([string]$path, [string]$pattern) {
    $m = [regex]::Match((Get-Content $path -Raw), $pattern, 'Multiline')
    if (-not $m.Success) { throw "Versione non trovata in $path" }
    return $m.Groups[1].Value
}

$vCore = Versione-Toml "$root\core\Cargo.toml" '^version\s*=\s*"([^"]+)"'
$vDesktop = Versione-Toml "$root\desktop\src-tauri\Cargo.toml" '^version\s*=\s*"([^"]+)"'
$vConf = (Get-Content "$root\desktop\src-tauri\tauri.conf.json" -Raw | ConvertFrom-Json).version
$vNpm = (Get-Content "$root\desktop\package.json" -Raw | ConvertFrom-Json).version
$vApp = (Get-Content "$root\app\package.json" -Raw | ConvertFrom-Json).version

Write-Host "Versioni: core $vCore, programma Windows $vDesktop, tauri.conf $vConf, desktop/package.json $vNpm, app $vApp"
if (@($vCore, $vDesktop, $vConf, $vNpm, $vApp | Select-Object -Unique).Count -ne 1) {
    throw 'Le versioni non coincidono: allinea core/Cargo.toml, desktop/src-tauri/Cargo.toml, tauri.conf.json e i due package.json.'
}
if ($SoloControllo) { Write-Host 'Versioni allineate.'; return }

if (-not (Test-Path $Chiave)) { throw "Chiave di firma non trovata: $Chiave" }

Write-Host '1/3 Interfaccia...'
Push-Location "$root\app"
try { npm run build; if ($LASTEXITCODE -ne 0) { throw 'Compilazione dell''interfaccia fallita' } } finally { Pop-Location }

Write-Host '2/3 Programma e installatore (la prima volta richiede qualche minuto)...'
Push-Location "$root\desktop"
try {
    $env:TAURI_SIGNING_PRIVATE_KEY_PATH = $Chiave
    if (-not (Test-Path env:TAURI_SIGNING_PRIVATE_KEY_PASSWORD)) { $env:TAURI_SIGNING_PRIVATE_KEY_PASSWORD = '' }
    npx tauri build
    if ($LASTEXITCODE -ne 0) { throw 'Compilazione del programma fallita' }
} finally { Pop-Location }

Write-Host '3/3 File della release...'
$bundle = "$root\desktop\src-tauri\target\release\bundle\nsis"
$setup = Get-ChildItem $bundle -Filter "*_${vCore}_*-setup.exe" | Select-Object -First 1
if (-not $setup) { throw "Installatore della versione $vCore non trovato in $bundle" }
$sig = "$($setup.FullName).sig"
if (-not (Test-Path $sig)) { throw "Firma non trovata: $sig" }
$dest = "$root\dist-release"
New-Item -ItemType Directory -Force $dest | Out-Null
Copy-Item $setup.FullName, $sig $dest -Force
$sums = Get-ChildItem $dest -Filter '*-setup.exe' | ForEach-Object { '{0}  {1}' -f (Get-FileHash $_.FullName -Algorithm SHA256).Hash.ToLower(), $_.Name }
Set-Content "$dest\SHA256SUMS.txt" $sums -Encoding ascii

Write-Host ''
Write-Host "Pronto in $dest :"
Get-ChildItem $dest | ForEach-Object { Write-Host ('  {0}  ({1:N1} MB)' -f $_.Name, ($_.Length / 1MB)) }
Write-Host ''
Write-Host 'Per pubblicare (dopo il tag e il push, a mano):'
Write-Host "  gh release upload v$vCore `"$dest\$($setup.Name)`" `"$dest\$($setup.Name).sig`" `"$dest\SHA256SUMS.txt`""
Write-Host '  (o, se la release non esiste ancora: gh release create v... con gli stessi tre file)'
