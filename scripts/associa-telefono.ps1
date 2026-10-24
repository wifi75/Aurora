# Mostra sul PC il codice (e il QR) per associare un telefono ad Aurora.
# Uso: .\scripts\associa-telefono.ps1 [-Porta 5173] [-Core 8787]
#   -Porta: porta dell'interfaccia raggiungibile dal telefono (5173 in sviluppo, 8787 con l'interfaccia compilata)
param(
    [int]$Porta = 5173,
    [int]$Core = 8787
)

$ErrorActionPreference = 'Stop'

# Indirizzo del PC sulla rete locale (escluse interfacce virtuali e link-local)
$ip = Get-NetIPAddress -AddressFamily IPv4 -ErrorAction SilentlyContinue |
    Where-Object {
        $_.IPAddress -notlike '169.254.*' -and $_.IPAddress -ne '127.0.0.1' -and
        $_.InterfaceAlias -notmatch 'vEthernet|WSL|Docker|Loopback|Hyper-V|VirtualBox|VMware'
    } | Select-Object -First 1 -ExpandProperty IPAddress
if (-not $ip) { $ip = '127.0.0.1' }
$base = "http://${ip}:${Porta}"

try {
    $body = @{ base_url = $base } | ConvertTo-Json
    $r = Invoke-RestMethod -Method Post -Uri "http://127.0.0.1:$Core/api/v1/pair/start" `
        -ContentType 'application/json' -Body $body
}
catch {
    Write-Host "Aurora non risponde su 127.0.0.1:$Core. Avvia prima scripts\avvia-sviluppo.ps1." -ForegroundColor Red
    exit 1
}

$svg = Join-Path $env:TEMP 'aurora-associazione.svg'
Set-Content -Path $svg -Value $r.qr_svg -Encoding UTF8

Write-Host ''
Write-Host "Codice di associazione:  $($r.code)   (vale $($r.expires_in) secondi)"
Write-Host "Link da aprire sul telefono:  $base/#/pair/$($r.code)"
Write-Host 'Il QR si apre nel browser: inquadralo con la fotocamera del telefono (stessa rete Wi-Fi).'
Start-Process $svg
