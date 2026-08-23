$ErrorActionPreference = "Stop"
$Repo = (Resolve-Path (Join-Path $PSScriptRoot "..\..")).Path
$Mingw = "C:\msys64\mingw64"
$env:PATH = (Join-Path $Mingw "bin") + ";" + $env:PATH
$env:CC = Join-Path $Mingw "bin\gcc.exe"
$env:CGO_ENABLED = "1"
$env:PKG_CONFIG_PATH = (Join-Path $Mingw "lib\pkgconfig")
Push-Location (Join-Path $Repo "desktop-gtk")
try { go run . --backend-path (Join-Path $Repo "desktop\resources\tmux-gui-server.exe") } finally { Pop-Location }
