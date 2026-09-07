param(
    [switch]$Release,
    [switch]$SkipBackend,
    [switch]$BuildBackend
)

$Profile = if ($Release) { "release" } else { "debug" }
$CargoArgs = if ($Release) { @("build", "--release") } else { @("build") }

# Tmux GUI Desktop Packaging Script for Windows (GPUI)
# Port of web-term's desktop-gpui/scripts/package-windows.ps1, 3 steps verbatim
# modulo binary names (webterm -> webtmux, backend -> tmux-gui-server).
$ErrorActionPreference = "Stop"

Write-Host "=======================================================" -ForegroundColor Cyan
Write-Host "Building Tmux GUI Desktop Distribution Bundle ($Profile)" -ForegroundColor Cyan
Write-Host "=======================================================" -ForegroundColor Cyan

$Root = Resolve-Path "$PSScriptRoot\..\.."
$DistDir = "$Root\dist\tmux-gui-windows-x64"

Write-Host "Target distribution directory: $DistDir"
New-Item -ItemType Directory -Force -Path $DistDir | Out-Null

$BackendExists = Test-Path "$DistDir\tmux-gui-server.exe"
if ($SkipBackend -or (-not $BuildBackend -and $BackendExists)) {
    Write-Host "[1/3] Backend binary already exists at $DistDir\tmux-gui-server.exe (skipping backend build)..." -ForegroundColor Cyan
} else {
    Write-Host "[1/3] Compiling Go backend with loopback binding..." -ForegroundColor Yellow
    Push-Location "$Root\be"
    try {
        go build -ldflags "-s -w" -o "$DistDir\tmux-gui-server.exe" ./cmd/server
    } finally {
        Pop-Location
    }
}

# Ensure shader compiler is available for GPUI build (release only needs it,
# but bootstrap unconditionally so `make desktop-gpui` never hits Pitfall 4).
$FxcTool = "$Root\desktop-gpui\tools\fxc\fxc.exe"
if ([string]::IsNullOrEmpty($env:GPUI_FXC_PATH) -or -not (Test-Path $env:GPUI_FXC_PATH -ErrorAction SilentlyContinue)) {
    if (-not (Test-Path $FxcTool)) {
        if (-not (Get-Command rustc -ErrorAction SilentlyContinue)) {
            throw "rustc not found on PATH - required to bootstrap tools/fxc for the GPUI release build. Install the Rust toolchain (https://rustup.rs/) and retry."
        }
        Write-Host "Compiling standalone FXC shader compiler helper..." -ForegroundColor Yellow
        rustc -O "$Root\desktop-gpui\tools\fxc\main.rs" -o $FxcTool
    }
    $env:GPUI_FXC_PATH = $FxcTool
}

Write-Host "[2/3] Compiling GPUI desktop client in $Profile mode..." -ForegroundColor Yellow
if ($Release) {
    cargo build --release --manifest-path "$Root\desktop-gpui\Cargo.toml" --package webtmux
} else {
    cargo build --manifest-path "$Root\desktop-gpui\Cargo.toml" --package webtmux
}

Write-Host "[3/3] Assembling distribution bundle..." -ForegroundColor Yellow
Stop-Process -Name webtmux, tmux-gui-server -Force -ErrorAction SilentlyContinue
Start-Sleep -Milliseconds 200
Copy-Item "$Root\desktop-gpui\target\$Profile\webtmux.exe" "$DistDir\webtmux.exe" -Force

# NOTE: no fonts/assets copy step - JetBrains Mono is `include_bytes!`
# compiled into the binary (main.rs); no desktop/resources staging
# (that path serves Electron - GPUI resolves adjacent-to-exe via bundle.rs).

$Readme = @"
Tmux GUI Desktop Client (Windows)
=================================
Launch with: webtmux.exe

Architecture:
- Single launchable bundle with tmux-gui-server.exe and webtmux.exe side-by-side.
- The backend process is automatically supervised and bound strictly to loopback (127.0.0.1).
- Native GPU terminal rendering via Alacritty.
- Expected siblings beside webtmux.exe: tmux-gui-server.exe, README.txt.
  If tmux-gui-server.exe is missing the app falls back to dev-path probing;
  paranoid users can pin an absolute backend via settings `backend_path`.
- Fonts: JetBrains Mono is embedded in the binary - no assets directory needed.
"@
Set-Content -Path "$DistDir\README.txt" -Value $Readme

Write-Host ""
Write-Host "=======================================================" -ForegroundColor Green
Write-Host "Tmux GUI Windows Bundle assembled successfully!" -ForegroundColor Green
Write-Host "Location: $DistDir" -ForegroundColor Green
Get-ChildItem $DistDir | Select-Object Name, Length, LastWriteTime | Format-Table -AutoSize
Write-Host "=======================================================" -ForegroundColor Green
