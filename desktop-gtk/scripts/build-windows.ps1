param(
  [string]$MsysRoot = "C:\msys64",
  [string]$Configuration = "release"
)

$ErrorActionPreference = "Stop"
$Repo = (Resolve-Path (Join-Path $PSScriptRoot "..\..")).Path
$Stage = Join-Path $Repo "dist\gtk\windows\Tmux GUI"
$Archive = Join-Path $Repo "dist\gtk\tmux-gui-gtk-windows-x64.zip"
$Mingw = Join-Path $MsysRoot "mingw64"
$WindowsOut = [IO.Path]::GetFullPath((Join-Path $Repo "dist\gtk\windows"))
if (-not ([IO.Path]::GetFullPath($Stage).StartsWith($WindowsOut + [IO.Path]::DirectorySeparatorChar))) { throw "Refusing to use an output path outside dist\\gtk\\windows" }

foreach ($required in @(
  (Join-Path $Mingw "bin\gcc.exe"),
  (Join-Path $Mingw "lib\pkgconfig\gtk4.pc"),
  (Join-Path $Mingw "lib\pkgconfig\libadwaita-1.pc"),
  (Join-Path $Mingw "lib\pkgconfig\vterm.pc"),
  (Join-Path $MsysRoot "usr\bin\bash.exe")
)) {
  if (-not (Test-Path -LiteralPath $required)) { throw "Missing dependency: $required" }
}

if (Test-Path -LiteralPath $Stage) { Remove-Item -Recurse -Force -LiteralPath $Stage }
New-Item -ItemType Directory -Force $Stage | Out-Null

$env:PATH = (Join-Path $Mingw "bin") + ";" + $env:PATH
$env:CC = Join-Path $Mingw "bin\gcc.exe"
$env:CGO_ENABLED = "1"
$env:PKG_CONFIG_PATH = (Join-Path $Mingw "lib\pkgconfig") + ";" + (Join-Path $Mingw "share\pkgconfig")

Push-Location (Join-Path $Repo "be")
try { go build -trimpath -ldflags "-s -w" -o (Join-Path $Stage "tmux-gui-server.exe") ./cmd/server } finally { Pop-Location }
Push-Location (Join-Path $Repo "desktop-gtk")
try { go build -trimpath -ldflags "-H=windowsgui -s -w" -o (Join-Path $Stage "tmux-gui-gtk.exe") . } finally { Pop-Location }

& (Join-Path $MsysRoot "usr\bin\bash.exe") (Join-Path $Repo "desktop-gtk\scripts\collect-mingw-dlls.sh") (Join-Path $Stage "tmux-gui-gtk.exe") $Stage
if ($LASTEXITCODE -ne 0) { throw "Failed to collect GTK runtime DLLs" }

New-Item -ItemType Directory -Force (Join-Path $Stage "share\glib-2.0"), (Join-Path $Stage "share\icons"), (Join-Path $Stage "lib") | Out-Null
Copy-Item -Recurse -Force (Join-Path $Mingw "share\glib-2.0\schemas") (Join-Path $Stage "share\glib-2.0\schemas")
Copy-Item -Recurse -Force (Join-Path $Mingw "share\icons\Adwaita") (Join-Path $Stage "share\icons\Adwaita")
Copy-Item -Recurse -Force (Join-Path $Mingw "share\icons\hicolor") (Join-Path $Stage "share\icons\hicolor")
if (Test-Path (Join-Path $Mingw "lib\gdk-pixbuf-2.0")) { Copy-Item -Recurse -Force (Join-Path $Mingw "lib\gdk-pixbuf-2.0") (Join-Path $Stage "lib\gdk-pixbuf-2.0") }
Copy-Item -Force (Join-Path $Repo "desktop-gtk\README.md") (Join-Path $Stage "README.txt")

if (Test-Path -LiteralPath $Archive) { Remove-Item -Force -LiteralPath $Archive }
Compress-Archive -Path $Stage -DestinationPath $Archive -CompressionLevel Optimal
Write-Host "Portable GTK build: $Archive"
