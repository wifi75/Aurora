# Disegna le due immagini dell'installatore (intestazione 150x57 e barra laterale 164x314, BMP a 24 bit) con il logo di Aurora.
# Si lancia a mano quando cambia il logo o la versione; i file prodotti stanno nel repository (desktop/src-tauri/installer).
#   .\desktop\scripts\genera-immagini-installer.ps1 -Versione 0.10.0
param([string]$Versione = '0.10.0')

$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.Drawing
$root = Split-Path -Parent (Split-Path -Parent $PSScriptRoot)
$logoPath = Join-Path $root 'app\public\icon-512.png'
$out = Join-Path $root 'desktop\src-tauri\installer'
New-Item -ItemType Directory -Force $out | Out-Null
$logo = [System.Drawing.Image]::FromFile($logoPath)

function New-Canvas([int]$w, [int]$h) {
    $bmp = New-Object System.Drawing.Bitmap($w, $h, [System.Drawing.Imaging.PixelFormat]::Format24bppRgb)
    $g = [System.Drawing.Graphics]::FromImage($bmp)
    $g.SmoothingMode = 'AntiAlias'
    $g.InterpolationMode = 'HighQualityBicubic'
    $g.TextRenderingHint = 'AntiAliasGridFit'
    return @($bmp, $g)
}

function Fill-Night($g, [int]$w, [int]$h) {
    $rect = New-Object System.Drawing.Rectangle(0, 0, $w, $h)
    $brush = New-Object System.Drawing.Drawing2D.LinearGradientBrush($rect, [System.Drawing.Color]::FromArgb(8, 14, 28), [System.Drawing.Color]::FromArgb(22, 40, 70), 90)
    $g.FillRectangle($brush, $rect)
    $brush.Dispose()
}

function Draw-Aurora($g, [int]$w, [int]$h) {
    # bande luminose in basso, come un'aurora: verde acqua, viola, rosa
    $bands = @(
        @{ c = [System.Drawing.Color]::FromArgb(70, 63, 214, 200); y = 0.62; a = 0.10 },
        @{ c = [System.Drawing.Color]::FromArgb(60, 142, 123, 255); y = 0.72; a = 0.12 },
        @{ c = [System.Drawing.Color]::FromArgb(50, 255, 93, 138); y = 0.82; a = 0.09 }
    )
    foreach ($b in $bands) {
        $pts = @()
        for ($x = 0; $x -le $w; $x += 4) {
            $y = $h * $b.y + [Math]::Sin($x / $w * 5.2 + $b.y * 9) * $h * $b.a
            $pts += New-Object System.Drawing.PointF($x, $y)
        }
        $pts += New-Object System.Drawing.PointF($w, $h)
        $pts += New-Object System.Drawing.PointF(0, $h)
        $brush = New-Object System.Drawing.SolidBrush($b.c)
        $g.FillPolygon($brush, [System.Drawing.PointF[]]$pts)
        $brush.Dispose()
    }
}

# --- barra laterale 164x314 ---
$canvas = New-Canvas 164 314
$bmp = $canvas[0]; $g = $canvas[1]
Fill-Night $g 164 314
Draw-Aurora $g 164 314
$g.DrawImage($logo, 26, 28, 112, 112)
$title = New-Object System.Drawing.Font('Segoe UI', 17, [System.Drawing.FontStyle]::Bold)
$small = New-Object System.Drawing.Font('Segoe UI', 8.5)
$tiny = New-Object System.Drawing.Font('Segoe UI', 7.5)
$white = New-Object System.Drawing.SolidBrush([System.Drawing.Color]::FromArgb(238, 244, 255))
$muted = New-Object System.Drawing.SolidBrush([System.Drawing.Color]::FromArgb(170, 188, 214))
$center = New-Object System.Drawing.StringFormat; $center.Alignment = 'Center'
$g.DrawString('AURORA', $title, $white, (New-Object System.Drawing.RectangleF(0, 150, 164, 30)), $center)
$g.DrawString("Stazione digitale`nFT8 / FT4", $small, $muted, (New-Object System.Drawing.RectangleF(0, 184, 164, 40)), $center)
$g.DrawString("Versione $Versione", $tiny, $muted, (New-Object System.Drawing.RectangleF(0, 262, 164, 16)), $center)
$g.DrawString('Tiziano Cassone', $tiny, $white, (New-Object System.Drawing.RectangleF(0, 280, 164, 16)), $center)
$bmp.Save((Join-Path $out 'sidebar.bmp'), [System.Drawing.Imaging.ImageFormat]::Bmp)
$g.Dispose(); $bmp.Dispose()

# --- intestazione 150x57 ---
$canvas = New-Canvas 150 57
$bmp = $canvas[0]; $g = $canvas[1]
Fill-Night $g 150 57
Draw-Aurora $g 150 57
$g.DrawImage($logo, 8, 7, 43, 43)
$g.DrawString('AURORA', (New-Object System.Drawing.Font('Segoe UI', 13, [System.Drawing.FontStyle]::Bold)), $white, 54, 9)
$g.DrawString("v$Versione", $tiny, $muted, 56, 33)
$bmp.Save((Join-Path $out 'header.bmp'), [System.Drawing.Imaging.ImageFormat]::Bmp)
$g.Dispose(); $bmp.Dispose()
$logo.Dispose()
Write-Host "Immagini scritte in $out"
