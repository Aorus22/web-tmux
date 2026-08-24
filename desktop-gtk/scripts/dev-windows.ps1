$ErrorActionPreference = "Stop"
$Repo = (Resolve-Path (Join-Path $PSScriptRoot "..\..")).Path
Push-Location (Join-Path $Repo "desktop-gtk")
try {
  . .\environment.ps1
  go run . --backend-path (Join-Path $Repo "desktop\resources\tmux-gui-server.exe")
} finally { Pop-Location }
