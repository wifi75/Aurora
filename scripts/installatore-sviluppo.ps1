# Installatore della versione di sviluppo (ramo privato `sviluppo`) compilato sul PC, senza consumare i minuti di GitHub Actions.
# Compilato con AURORA_SVILUPPO: non cerca aggiornamenti su GitHub, cosi' non torna mai da sola alle release pubbliche.
# Niente firme (ne' Windows ne' aggiornamenti). Uso, dalla cartella del ramo sviluppo:
#   .\scripts\installatore-sviluppo.ps1            -> compila e lascia l'installatore in dist-sviluppo\
#   .\scripts\installatore-sviluppo.ps1 -Installa  -> compila e lo avvia subito
param([switch]$Installa)
$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
Set-Location $root

$ramo = (git branch --show-current).Trim()
if ($ramo -ne 'sviluppo') { throw "Sei sul ramo '$ramo': l'installatore di sviluppo si fa dal ramo 'sviluppo'." }

Write-Host '1/4 File dei paesi (cty.dat)...'
$ctyDir = Join-Path $root 'desktop\src-tauri\cty'
New-Item -ItemType Directory -Force $ctyDir | Out-Null
try { Invoke-WebRequest -Uri 'https://www.country-files.com/cty/cty.dat' -OutFile (Join-Path $ctyDir 'cty.dat') -UseBasicParsing -TimeoutSec 90 }
catch { Write-Warning 'country-files.com non risponde: uso la copia di riserva'; Copy-Item (Join-Path $root 'desktop\cty-riserva\cty.dat') (Join-Path $ctyDir 'cty.dat') -Force }

Write-Host '2/4 Hamlib (rigctld, rotctld)...'
& (Join-Path $root 'scripts\prepara-hamlib.ps1')

Write-Host '3/4 Interfaccia...'
Push-Location (Join-Path $root 'app')
try { npm ci; npm run build; if ($LASTEXITCODE -ne 0) { throw 'compilazione dell''interfaccia fallita' } } finally { Pop-Location }

Write-Host '4/4 Programma e installatore...'
Push-Location (Join-Path $root 'desktop')
try {
    npm ci
    '{"bundle":{"createUpdaterArtifacts":false}}' | Set-Content -Encoding utf8NoBOM senza-aggiornamenti.json
    $env:AURORA_SVILUPPO = '1'
    $env:AURORA_REQUIRE_HAMLIB = '1'
    npx tauri build --config senza-aggiornamenti.json
    if ($LASTEXITCODE -ne 0) { throw 'compilazione del programma fallita' }
} finally {
    Remove-Item Env:AURORA_SVILUPPO -ErrorAction SilentlyContinue
    Remove-Item senza-aggiornamenti.json -ErrorAction SilentlyContinue
    Pop-Location
}

$exe = Get-ChildItem (Join-Path $root 'desktop\src-tauri\target\release\bundle\nsis') -Filter '*-setup.exe' | Sort-Object LastWriteTime -Descending | Select-Object -First 1
if (-not $exe) { throw 'installatore non trovato' }
$dist = Join-Path $root 'dist-sviluppo'
New-Item -ItemType Directory -Force $dist | Out-Null
$commit = (git rev-parse --short HEAD).Trim()
$dest = Join-Path $dist ("Aurora-sviluppo-{0}-{1}.exe" -f (Get-Date -Format 'yyyyMMdd-HHmm'), $commit)
Copy-Item $exe.FullName $dest
Write-Host "Installatore pronto: $dest"
if ($Installa) { Start-Process $dest }
