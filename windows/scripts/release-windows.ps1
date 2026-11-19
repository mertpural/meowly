# Meowly Windows sürümü hazırlar (Mac: scripts/release.sh). YAYINLAMAZ: dosyaları site klasörüne koyar, yayın ayrı adımdır.
# Kullanım:
#   powershell -ExecutionPolicy Bypass -File windows\scripts\release-windows.ps1 -Version 0.9.0 -Notes "İlk Windows sürümü"
#   (-Site <klasör>: sitenin klasörü; verilmezse windows\release-out\site kullanılır, oradan Mac'teki site klasörüne kopyalanır.)
# Üretilenler (site köküne göre):
#   download/Meowly-Setup-<sürüm>.exe   güncelleyicinin indirdiği, sürüme özel dosya
#   download/Meowly-Setup.exe           sitedeki "Windows için indir" düğmesinin sabit bağlantısı
#   latest-windows.json                 {version, url, sha256, notes, size}; kurulu uygulamalar günde bir kez okur
#   win.js                              sitenin indirme bölümüne Windows düğmesini ve adımlarını ekler
param(
  [Parameter(Mandatory = $true)][string]$Version,
  [string]$Notes = "",
  [string]$Site = ""
)
$ErrorActionPreference = "Stop"
$Root = Resolve-Path (Join-Path $PSScriptRoot "..")
$Url = "https://meowly.vercel.app"
if (-not $Site) { $Site = Join-Path $Root "release-out\site" }
if ($Version -notmatch '^\d+\.\d+\.\d+$') { throw "Sürüm 1.2.3 biçiminde olmalı" }
$utf8 = New-Object System.Text.UTF8Encoding $false

# 1) Sürüm numarası (tauri.conf.json, Cargo.toml, package.json)
$conf = Join-Path $Root "src-tauri\tauri.conf.json"
$t = [IO.File]::ReadAllText($conf, $utf8) -replace '"version":\s*"[^"]*"', "`"version`": `"$Version`""
[IO.File]::WriteAllText($conf, $t, $utf8)
$cargo = Join-Path $Root "src-tauri\Cargo.toml"
$t = [IO.File]::ReadAllText($cargo, $utf8); $t = ([regex]'(?m)^version = "[^"]*"').Replace($t, "version = `"$Version`"", 1)
[IO.File]::WriteAllText($cargo, $t, $utf8)
$pkg = Join-Path $Root "package.json"
$t = [IO.File]::ReadAllText($pkg, $utf8) -replace '"version":\s*"[^"]*"', "`"version`": `"$Version`""
[IO.File]::WriteAllText($pkg, $t, $utf8)

# 2) Derle ve kurulum dosyasını üret (çalışan Meowly exe'yi kilitler; önce kapatılır)
Get-Process meowly -ErrorAction SilentlyContinue | Stop-Process -Force
$env:Path += ";$env:USERPROFILE\.cargo\bin"
Push-Location $Root
try { npx tauri build; if ($LASTEXITCODE -ne 0) { throw "Derleme başarısız" } } finally { Pop-Location }
$setup = Join-Path $Root "src-tauri\target\release\bundle\nsis\Meowly_${Version}_x64-setup.exe"
if (-not (Test-Path $setup)) { throw "Kurulum dosyası bulunamadı: $setup" }

# 3) Site klasörüne koy, SHA-256 ve boyut
$dl = Join-Path $Site "download"
New-Item -ItemType Directory -Force $dl | Out-Null
$verFile = Join-Path $dl "Meowly-Setup-$Version.exe"
Copy-Item $setup $verFile -Force
Copy-Item $setup (Join-Path $dl "Meowly-Setup.exe") -Force
$sha = (Get-FileHash $verFile -Algorithm SHA256).Hash.ToLower()
$size = (Get-Item $verFile).Length

# 4) Güncelleme bilgisi
$feed = [ordered]@{ version = $Version; url = "$Url/download/Meowly-Setup-$Version.exe"; sha256 = $sha; notes = $Notes; size = $size }
[IO.File]::WriteAllText((Join-Path $Site "latest-windows.json"), ($feed | ConvertTo-Json), $utf8)

# 5) Sitenin Windows eklentisi; index.html varsa ve eklenti bağlı değilse tek satır eklenir.
Copy-Item (Join-Path $Root "site\win.js") (Join-Path $Site "win.js") -Force
$index = Join-Path $Site "index.html"
if (Test-Path $index) {
  $h = [IO.File]::ReadAllText($index, $utf8)
  if ($h -notmatch 'src="/win\.js') {
    $h = $h -replace '</body>', "  <script src=`"/win.js?v=$Version`" defer></script>`n</body>"
    [IO.File]::WriteAllText($index, $h, $utf8)
    Write-Host "index.html: win.js satırı eklendi"
  }
} else {
  Write-Host "Not: $Site içinde index.html yok. Sitenin index.html'ine </body> öncesine şunu ekle:"
  Write-Host "  <script src=`"/win.js?v=$Version`" defer></script>"
}

Write-Host ""
Write-Host "Hazır: Meowly $Version · sha256 $sha · $size bayt"
Write-Host "Klasör: $Site"
Write-Host "Yayın (ayrı adım, site klasöründen): npx --yes vercel@latest deploy --prod --yes"
