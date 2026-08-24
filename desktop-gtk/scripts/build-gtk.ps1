# Build the native GTK desktop binary, matching the wa-bot build workflow.
# Run from anywhere:
#   powershell -NoProfile -ExecutionPolicy Bypass -File desktop-gtk/scripts/build-gtk.ps1
#
# Optional:
#   -Output ..\some\dir\tmux-gui-desktop.exe
#   -Console                      keep the console subsystem (logs visible)
param(
    [string]$Output = "..\compiled\tmux-gui-desktop.exe",
    [switch]$Console
)

$ErrorActionPreference = "Stop"
$Repo = (Resolve-Path (Join-Path $PSScriptRoot "..\..")).Path
Set-Location (Join-Path $Repo "desktop-gtk")

. .\environment.ps1

$outputPath = if ([IO.Path]::IsPathRooted($Output)) {
    [IO.Path]::GetFullPath($Output)
} else {
    [IO.Path]::GetFullPath((Join-Path (Get-Location) $Output))
}
New-Item -ItemType Directory -Force (Split-Path -Parent $outputPath) | Out-Null

$ldflags = "-s -w"
if (-not $Console) {
    $ldflags = "$ldflags -H windowsgui"
}

Write-Host "--- go vet ---"
go vet ./...
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

Write-Host "--- go build ---"
go build -trimpath -ldflags "$ldflags" -o $outputPath .
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

Write-Host "BUILD OK -> $outputPath"
