# Pubblica la pagina web di Aurora (docs/index.html) sul ramo gh-pages, da cui GitHub Pages serve https://wifi75.github.io/Aurora/.
# Uso (dalla radice del repository): pwsh -File scripts\pubblica-sito.ps1
$ErrorActionPreference = 'Stop'
$repo = Split-Path -Parent $PSScriptRoot
$sorgente = Join-Path $repo 'docs\index.html'
if (-not (Test-Path $sorgente)) { throw "manca $sorgente" }
$remoto = (git -C $repo remote get-url origin).Trim()
$tmp = Join-Path ([System.IO.Path]::GetTempPath()) 'aurora-gh-pages'
if (Test-Path $tmp) { Remove-Item -Recurse -Force $tmp }
git clone --quiet --depth 1 --branch gh-pages $remoto $tmp
Copy-Item $sorgente (Join-Path $tmp 'index.html') -Force
# le immagini della pagina (screenshot di Aurora)
Get-ChildItem (Join-Path $repo 'docs') -File | Where-Object { $_.Extension -in '.png', '.jpg', '.webp' } | ForEach-Object { Copy-Item $_.FullName (Join-Path $tmp $_.Name) -Force }
New-Item -ItemType File -Force (Join-Path $tmp '.nojekyll') | Out-Null
# stesso autore dei commit del repository (indirizzo noreply di GitHub)
$nome = (git -C $repo config user.name).Trim()
$email = (git -C $repo config user.email).Trim()
git -C $tmp add -A
if (-not (git -C $tmp status --porcelain)) { Write-Output 'sito gia aggiornato'; exit 0 }
$versione = (Get-Content (Join-Path $repo 'app\package.json') | ConvertFrom-Json).version
git -C $tmp -c "user.name=$nome" -c "user.email=$email" commit --quiet -m "sito: pagina aggiornata alla $versione"
git -C $tmp push --quiet origin gh-pages
Remove-Item -Recurse -Force $tmp
Write-Output "sito pubblicato ($versione): https://wifi75.github.io/Aurora/"
